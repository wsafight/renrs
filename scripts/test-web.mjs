import { spawn, spawnSync } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Builds the WASM engine and Web shell, generates browser fixtures, and runs Playwright.
const root = fileURLToPath(new URL('../', import.meta.url));
const arguments_ = new Set(process.argv.slice(2));
const allowed = new Set(['--web', '--media']);
const unknown = [...arguments_].filter((argument) => !allowed.has(argument));
const web = arguments_.has('--web');
const media = arguments_.has('--media');
if (unknown.length || !(web || media)) {
  throw new Error('Usage: node scripts/test-web.mjs [--web] [--media]');
}

function run(label, command, args, cwd = root, env = {}) {
  console.log(`\n==> ${label}`);
  const result = spawnSync(command, args, {
    cwd,
    env: { ...process.env, ...env },
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${label} failed with exit code ${result.status ?? 1}`);
}

function requireMediaFfmpeg() {
  const ffmpeg = process.env.RENRS_FFMPEG || 'ffmpeg';
  const result = spawnSync(ffmpeg, ['-hide_banner', '-formats'], { encoding: 'utf8' });
  if (
    result.error ||
    result.status !== 0 ||
    !`${result.stdout}${result.stderr}`.includes('lavfi')
  ) {
    throw new Error(
      'Media verification requires FFmpeg with the lavfi input. Set RENRS_FFMPEG to a full build.',
    );
  }
  return ffmpeg;
}

async function availablePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close((error) => (error ? reject(error) : resolve(port)));
    });
  });
}

async function waitForServer(url, child) {
  for (let attempt = 0; attempt < 100; attempt += 1) {
    if (child.exitCode !== null) throw new Error('Web preview exited before becoming ready');
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

const temporary = await mkdtemp(path.join(tmpdir(), 'renrs-web-'));
try {
  run('Web shell', process.execPath, ['scripts/build-web.mjs']);
  const webRoot = path.join(temporary, 'web');
  run('Web demo', 'cargo', [
    'run',
    '--locked',
    '--quiet',
    '--bin',
    'renrs-web-build',
    '--',
    'demo',
    webRoot,
  ]);
  if (web) {
    const product = path.join(temporary, 'product');
    run('Product fixture', 'cargo', [
      'run',
      '--locked',
      '--quiet',
      '--example',
      'generate_product_fixture',
      '--',
      product,
    ]);
    const composition = path.join(temporary, 'composition');
    run('Composition fixture', 'cargo', [
      'run',
      '--locked',
      '--quiet',
      '--example',
      'generate_composition_fixture',
      '--',
      composition,
    ]);
    run('Web product fixture', 'cargo', [
      'run',
      '--locked',
      '--quiet',
      '--bin',
      'renrs-web-build',
      '--',
      product,
      path.join(webRoot, 'product'),
    ]);
    run('Web composition fixture', 'cargo', [
      'run',
      '--locked',
      '--quiet',
      '--bin',
      'renrs-web-build',
      '--',
      composition,
      path.join(webRoot, 'composition'),
    ]);
  }
  if (media) {
    const ffmpeg = requireMediaFfmpeg();
    const reading = path.join(temporary, 'reading');
    const frameMedia = path.join(temporary, 'media');
    const streamMedia = path.join(temporary, 'stream');
    run('Video tool', 'cargo', ['build', '--locked', '--bin', 'renrs-video']);
    run(
      'Reading fixture',
      'cargo',
      ['run', '--locked', '--quiet', '--example', 'generate_reading_fixture', '--', reading],
      root,
      { RENRS_FFMPEG: ffmpeg },
    );
    run('Frame video fixture', process.execPath, ['scripts/validate-media.mjs', frameMedia], root, {
      RENRS_FFMPEG: ffmpeg,
    });
    run(
      'Streaming video fixture',
      process.execPath,
      ['scripts/validate-media.mjs', streamMedia, '--stream'],
      root,
      { RENRS_FFMPEG: ffmpeg },
    );
    for (const [label, source, destination] of [
      ['reading', reading, 'reading'],
      ['frame video', frameMedia, 'media'],
      ['streaming video', streamMedia, 'stream'],
    ]) {
      run(`Web ${label} fixture`, 'cargo', [
        'run',
        '--locked',
        '--quiet',
        '--bin',
        'renrs-web-build',
        '--',
        source,
        path.join(webRoot, destination),
      ]);
    }
  }

  const port = await availablePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const server = spawn(process.execPath, ['scripts/serve-web.mjs', webRoot, String(port)], {
    cwd: root,
    stdio: 'inherit',
  });
  try {
    await waitForServer(baseUrl, server);
    if (web) {
      run(
        'Web browser tests',
        'npx',
        [
          'playwright',
          'test',
          '--grep-invert',
          'parallel animation|video advances|streaming video',
        ],
        path.join(root, 'web'),
        { RENRS_WEB_URL: baseUrl },
      );
    }
    if (media) {
      run(
        'Web media browser tests',
        'npx',
        ['playwright', 'test', '--grep', 'parallel animation|video advances|streaming video'],
        path.join(root, 'web'),
        {
          RENRS_MEDIA_TEST: '1',
          RENRS_STREAM_TEST: '1',
          RENRS_WEB_URL: baseUrl,
        },
      );
    }
  } finally {
    server.kill('SIGTERM');
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}

import {spawn, spawnSync} from 'node:child_process';
import {existsSync} from 'node:fs';
import {mkdtemp, rm} from 'node:fs/promises';
import {createServer} from 'node:net';
import {tmpdir} from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const full = process.argv.slice(2).includes('--full');
if (process.argv.slice(2).some(argument => argument !== '--full')) {
  throw new Error('Usage: node scripts/verify-local.mjs [--full]');
}

function run(label, command, args, cwd = root, env = {}) {
  console.log(`\n==> ${label}`);
  const result = spawnSync(command, args, {
    cwd,
    env: {...process.env, ...env},
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${label} failed with exit code ${result.status ?? 1}`);
}

function requirePath(target, setup) {
  if (!existsSync(target)) throw new Error(`Missing ${target}. ${setup}`);
}

function requireMediaFfmpeg() {
  const ffmpeg = process.env.RENRS_FFMPEG || 'ffmpeg';
  const result = spawnSync(ffmpeg, ['-hide_banner', '-formats'], {encoding: 'utf8'});
  if (result.error || result.status !== 0 || !`${result.stdout}${result.stderr}`.includes('lavfi')) {
    throw new Error('Full verification requires FFmpeg with the lavfi input. Set RENRS_FFMPEG to a full build.');
  }
  return ffmpeg;
}

async function availablePort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const {port} = server.address();
      server.close(error => error ? reject(error) : resolve(port));
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
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out waiting for ${url}`);
}

const temporary = await mkdtemp(path.join(tmpdir(), 'renrs-verify-'));
try {
  run('Rust formatting', 'cargo', ['fmt', '--all', '--', '--check']);
  run('Rust Clippy', 'cargo', ['clippy', '--offline', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings']);
  run('Rust tests', 'cargo', ['test', '--offline', '--workspace', '--all-targets']);
  run('Demo validation', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-check', '--', 'demo']);
  run('Demo formatting', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-fmt', '--', '--check', 'demo']);

  const product = path.join(temporary, 'product');
  const story = path.join(temporary, 'story');
  run('Product fixture', 'cargo', ['run', '--offline', '--quiet', '--example', 'generate_product_fixture', '--', product]);
  run('Product acceptance', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-accept', '--', product]);
  run('Project scaffold', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-init', '--', story]);
  run('Scaffold routes', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-debug', '--', 'test', story, path.join(story, 'routes.json')]);
  run('Scaffold exploration', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-debug', '--', 'explore', story]);
  run('Web unit tests', process.execPath, ['--test', 'web/unit/audio.test.js', 'web/unit/reading.test.js']);
  run('VS Code extension syntax', process.execPath, ['--check', 'editors/vscode-renrs/extension.js']);
  run('Launcher syntax', process.execPath, ['--check', 'scripts/launcher.mjs']);
  run('Launcher jobs syntax', process.execPath, ['--check', 'scripts/launcher/jobs.mjs']);
  run('Launcher acceptance syntax', process.execPath, ['--check', 'scripts/test-launcher.mjs']);

  if (full) {
    requirePath(path.join(root, 'web/node_modules'), 'Run npm ci --prefix web.');
    requirePath(path.join(root, 'editors/vscode-renrs/node_modules'), 'Run npm ci --prefix editors/vscode-renrs.');
    const ffmpeg = requireMediaFfmpeg();
    run('Debug binaries', 'cargo', ['build', '--offline', '--bins']);
    run('Release binaries', 'cargo', ['build', '--offline', '--release', '--bins']);
    run('Web shell', process.execPath, ['scripts/build-web.mjs']);

    const webRoot = path.join(temporary, 'web');
    const reading = path.join(temporary, 'reading');
    const media = path.join(temporary, 'media');
    run('Web demo', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-web-build', '--', 'demo', webRoot]);
    run('Reading fixture', 'cargo', ['run', '--offline', '--quiet', '--example', 'generate_reading_fixture', '--', reading]);
    run('Video tool', 'cargo', ['build', '--offline', '--bin', 'renrs-video']);
    run('Media fixture', process.execPath, ['scripts/validate-media.mjs', media], root, {RENRS_FFMPEG: ffmpeg});
    run('Web product fixture', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-web-build', '--', product, path.join(webRoot, 'product')]);
    run('Web reading fixture', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-web-build', '--', reading, path.join(webRoot, 'reading')]);
    run('Web media fixture', 'cargo', ['run', '--offline', '--quiet', '--bin', 'renrs-web-build', '--', media, path.join(webRoot, 'media')]);

    const port = await availablePort();
    const baseUrl = `http://127.0.0.1:${port}`;
    const server = spawn(process.execPath, ['scripts/serve-web.mjs', webRoot, String(port)], {cwd: root, stdio: 'inherit'});
    try {
      await waitForServer(baseUrl, server);
      run('Web browser tests', 'npx', ['playwright', 'test'], path.join(root, 'web'), {
        RENRS_MEDIA_TEST: '1',
        RENRS_WEB_URL: baseUrl,
      });
    } finally {
      server.kill('SIGTERM');
    }

    run('VS Code extension host', 'npm', ['run', 'test:host'], path.join(root, 'editors/vscode-renrs'));
    run('Package VS Code extension', 'npm', ['run', 'package'], path.join(root, 'editors/vscode-renrs'));
    const visual = path.join(temporary, 'visual');
    const distribution = path.join(temporary, 'distribution');
    const captures = path.join(temporary, 'captures');
    const executable = path.join(root, 'target/release', process.platform === 'win32' ? 'renrs.exe' : 'renrs');
    const builder = path.join(root, 'target/release', process.platform === 'win32' ? 'renrs-build.exe' : 'renrs-build');
    run('Native visual fixture', 'cargo', ['run', '--offline', '--quiet', '--example', 'generate_visual_fixture', '--', visual]);
    run('Native distribution', builder, [visual, distribution]);
    run('Native demo smoke', executable, ['demo', '--smoke-test', path.join(captures, 'demo'), '--window-size', '800x600']);
    run('Packaged player smoke', path.join(distribution, process.platform === 'win32' ? 'renrs.exe' : 'renrs'), ['--smoke-test', path.join(captures, 'packaged'), '--window-size', '800x600'], temporary);
    const sdk = path.join(temporary, 'sdk');
    run('Package SDK', process.execPath, ['scripts/package-sdk.mjs', sdk, path.join(root, 'target/release')]);
    run('Packaged Launcher and SDK', process.execPath, ['scripts/test-launcher.mjs', sdk]);
  }
  console.log(`\nLocal ${full ? 'full' : 'core'} verification passed.`);
} finally {
  await rm(temporary, {recursive: true, force: true});
}

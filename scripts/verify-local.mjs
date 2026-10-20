import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const arguments_ = new Set(process.argv.slice(2));
const allowed = new Set(['--web', '--media', '--editor', '--release', '--full']);
const unknown = [...arguments_].filter((argument) => !allowed.has(argument));
if (unknown.length) {
  throw new Error(
    'Usage: node scripts/verify-local.mjs [--web] [--media] [--editor] [--release] [--full]',
  );
}
const full = arguments_.has('--full');
const web = full || arguments_.has('--web');
const media = full || arguments_.has('--media');
const editor = full || arguments_.has('--editor');
const release = full || arguments_.has('--release');

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

function requirePath(target, setup) {
  if (!existsSync(target)) throw new Error(`Missing ${target}. ${setup}`);
}

const temporary = await mkdtemp(path.join(tmpdir(), 'renrs-verify-'));
try {
  requirePath(path.join(root, 'node_modules'), 'Run npm ci.');
  requirePath(path.join(root, 'web/node_modules'), 'Run npm ci --prefix web.');
  requirePath(path.join(root, 'launcher/node_modules'), 'Run npm ci --prefix launcher.');
  requirePath(
    path.join(root, 'editors/vscode-renrs/node_modules'),
    'Run npm ci --prefix editors/vscode-renrs.',
  );
  run('Biome formatting and linting', 'npm', ['exec', 'biome', '--', 'check', '.']);
  run('Dependency advisories', process.execPath, ['scripts/audit-local.mjs']);
  run('Release contract versions', process.execPath, ['scripts/check-release.mjs']);
  run('Web TypeScript, unit tests and build', 'npm', ['run', 'check', '--prefix', 'web']);
  run('Launcher TypeScript and build', 'npm', ['run', 'check', '--prefix', 'launcher']);
  run('VS Code extension TypeScript and build', 'npm', [
    'run',
    'build',
    '--prefix',
    'editors/vscode-renrs',
  ]);
  run('Rust formatting', 'cargo', ['fmt', '--all', '--', '--check']);
  run('Rust Clippy', 'cargo', [
    'clippy',
    '--offline',
    '--workspace',
    '--all-targets',
    '--all-features',
    '--',
    '-D',
    'warnings',
  ]);
  run('Rust tests', 'cargo', ['test', '--offline', '--workspace', '--all-targets']);
  run('Demo validation', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-check',
    '--',
    'demo',
  ]);
  run('Demo formatting', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-fmt',
    '--',
    '--check',
    'demo',
  ]);

  const product = path.join(temporary, 'product');
  const reference = path.join(temporary, 'reference');
  const story = path.join(temporary, 'story');
  run('Product fixture', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--example',
    'generate_product_fixture',
    '--',
    product,
  ]);
  run('Product acceptance', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-accept',
    '--',
    product,
  ]);
  run('First-party reference fixture', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-bench',
    '--',
    'generate-reference',
    reference,
  ]);
  run('First-party reference acceptance', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-accept',
    '--',
    reference,
  ]);
  run('Project scaffold', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-init',
    '--',
    story,
  ]);
  run('Scaffold routes', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-debug',
    '--',
    'test',
    story,
    path.join(story, 'routes.json'),
  ]);
  run('Scaffold exploration', 'cargo', [
    'run',
    '--offline',
    '--quiet',
    '--bin',
    'renrs-debug',
    '--',
    'explore',
    story,
  ]);
  run('Launcher syntax', process.execPath, ['--check', 'scripts/launcher.mjs']);
  run('Launcher jobs syntax', process.execPath, ['--check', 'scripts/launcher/jobs.mjs']);
  run('Launcher acceptance syntax', process.execPath, ['--check', 'scripts/test-launcher.mjs']);
  run('Mobile acceptance syntax', process.execPath, ['--check', 'scripts/test-mobile.mjs']);

  if (release && !(web || media)) {
    run('Web shell', process.execPath, ['scripts/build-web.mjs']);
  }

  if (web || media) {
    run('Web browser verification', process.execPath, [
      'scripts/test-web.mjs',
      ...(web ? ['--web'] : []),
      ...(media ? ['--media'] : []),
    ]);
  }

  if (editor) {
    run(
      'VS Code extension host',
      'npm',
      ['run', 'test:host'],
      path.join(root, 'editors/vscode-renrs'),
    );
    run(
      'Package VS Code extension',
      'npm',
      ['run', 'package'],
      path.join(root, 'editors/vscode-renrs'),
    );
  }

  if (release) {
    run('Release binaries', 'cargo', ['build', '--offline', '--release', '--bins']);
    run('Release player', 'cargo', [
      'build',
      '--offline',
      '--release',
      '-p',
      'renrs-player',
      '--bin',
      'renrs',
    ]);
    const visual = path.join(temporary, 'visual');
    const distribution = path.join(temporary, 'distribution');
    const captures = path.join(temporary, 'captures');
    const executable = path.join(
      root,
      'target/release',
      process.platform === 'win32' ? 'renrs.exe' : 'renrs',
    );
    const builder = path.join(
      root,
      'target/release',
      process.platform === 'win32' ? 'renrs-build.exe' : 'renrs-build',
    );
    run('Native visual fixture', 'cargo', [
      'run',
      '--offline',
      '--quiet',
      '--example',
      'generate_visual_fixture',
      '--',
      visual,
    ]);
    run('Native distribution', builder, [visual, distribution]);
    const mobileWeb = path.join(temporary, 'mobile-web');
    const mobileProject = path.join(temporary, 'mobile-project');
    run('Mobile Web fixture', 'cargo', [
      'run',
      '--offline',
      '--quiet',
      '--bin',
      'renrs-web-build',
      '--',
      product,
      mobileWeb,
    ]);
    run('Capacitor project', process.execPath, ['scripts/mobile.mjs', mobileWeb, mobileProject]);
    run('Capacitor project acceptance', process.execPath, [
      'scripts/test-mobile.mjs',
      mobileProject,
    ]);
    run('Native demo smoke', executable, [
      'demo',
      '--smoke-test',
      path.join(captures, 'demo'),
      '--window-size',
      '800x600',
    ]);
    run(
      'Packaged player smoke',
      path.join(distribution, process.platform === 'win32' ? 'renrs.exe' : 'renrs'),
      ['--smoke-test', path.join(captures, 'packaged'), '--window-size', '800x600'],
      temporary,
    );
    const sdk = path.join(temporary, 'sdk');
    run('Package SDK', process.execPath, [
      'scripts/package-sdk.mjs',
      sdk,
      path.join(root, 'target/release'),
    ]);
    run('Packaged Launcher and SDK', process.execPath, ['scripts/test-launcher.mjs', sdk]);
  }
  const profiles = [web && 'web', media && 'media', editor && 'editor', release && 'release']
    .filter(Boolean)
    .join(', ');
  console.log(`\nLocal core${profiles ? ` + ${profiles}` : ''} verification passed.`);
} finally {
  await rm(temporary, { recursive: true, force: true });
}

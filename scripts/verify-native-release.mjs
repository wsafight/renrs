import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const [requestedOutput] = process.argv.slice(2);
if (!requestedOutput) {
  throw new Error('Usage: node scripts/verify-native-release.mjs <new-evidence-directory>');
}
const output = path.resolve(root, requestedOutput);
if (existsSync(output)) throw new Error(`Evidence directory already exists: ${output}`);

function run(label, command, args, cwd = root) {
  console.log(`\n==> ${label}`);
  const result = spawnSync(command, args, {
    cwd,
    env: process.env,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${label} failed with exit code ${result.status ?? 1}`);
}

function runSmoke(label, executable, args, cwd = root) {
  const wrapper = process.env.RENRS_SMOKE_WRAPPER;
  if (wrapper) run(label, wrapper, ['-a', executable, ...args], cwd);
  else run(label, executable, args, cwd);
}

async function sha256(file) {
  return createHash('sha256')
    .update(await readFile(file))
    .digest('hex');
}

async function validateReport(directory) {
  const report = JSON.parse(await readFile(path.join(directory, 'report.json'), 'utf8'));
  if (report.passed !== true || report.quick_load_restored !== true) {
    throw new Error(`Smoke report failed: ${directory}`);
  }
  if (!Array.isArray(report.captures) || report.captures.length < 10) {
    throw new Error(`Smoke report has insufficient captures: ${directory}`);
  }
  for (const capture of report.captures) {
    const image = path.join(directory, capture.file);
    if ((await stat(image)).size === 0) throw new Error(`Smoke capture is empty: ${image}`);
  }
  return report;
}

const release = JSON.parse(await readFile(path.join(root, 'release.json'), 'utf8'));
const target = path.resolve(root, process.env.CARGO_TARGET_DIR || 'target');
const executableName = process.platform === 'win32' ? 'renrs.exe' : 'renrs';
const builderName = process.platform === 'win32' ? 'renrs-build.exe' : 'renrs-build';
const executable = path.join(target, 'release', executableName);
const builder = path.join(target, 'release', builderName);
const temporary = await mkdtemp(path.join(tmpdir(), 'renrs-native-release-'));

await mkdir(output, { recursive: false });
try {
  run('Release tools', 'cargo', ['build', '--locked', '--release', '--bins']);
  run('Release player', 'cargo', [
    'build',
    '--locked',
    '--release',
    '-p',
    'renrs-player',
    '--bin',
    'renrs',
  ]);

  const visual = path.join(temporary, 'visual');
  const distribution = path.join(temporary, 'distribution');
  run('Native visual fixture', 'cargo', [
    'run',
    '--locked',
    '--quiet',
    '--example',
    'generate_visual_fixture',
    '--',
    visual,
  ]);
  run('Native distribution', builder, [visual, distribution]);

  const directCaptures = path.join(output, 'direct');
  const packagedCaptures = path.join(output, 'packaged');
  runSmoke('Native demo smoke', executable, [
    'demo',
    '--smoke-test',
    directCaptures,
    '--window-size',
    '800x600',
  ]);
  const packagedPlayer = path.join(distribution, executableName);
  runSmoke(
    'Packaged player smoke',
    packagedPlayer,
    ['--smoke-test', packagedCaptures, '--window-size', '800x600'],
    temporary,
  );

  const direct = await validateReport(directCaptures);
  const packaged = await validateReport(packagedCaptures);
  const expectedPlatform = process.platform === 'darwin' ? 'macos' : process.platform;
  if (direct.platform !== expectedPlatform || packaged.platform !== expectedPlatform) {
    throw new Error(
      `Smoke platform mismatch: expected ${expectedPlatform}, got ${direct.platform}/${packaged.platform}`,
    );
  }
  await writeFile(
    path.join(output, 'summary.json'),
    `${JSON.stringify(
      {
        schema_version: 1,
        engine_version: release.engine_version,
        platform: process.platform,
        arch: process.arch,
        direct_player_sha256: await sha256(executable),
        packaged_player_sha256: await sha256(packagedPlayer),
        direct: {
          project_id: direct.project_id,
          captures: direct.captures.length,
          elapsed_ms: direct.elapsed_ms,
        },
        packaged: {
          project_id: packaged.project_id,
          captures: packaged.captures.length,
          elapsed_ms: packaged.elapsed_ms,
        },
      },
      null,
      2,
    )}\n`,
  );
  console.log(`\nNative release verification passed: ${output}`);
} catch (error) {
  await rm(output, { recursive: true, force: true });
  throw error;
} finally {
  await rm(temporary, { recursive: true, force: true });
}

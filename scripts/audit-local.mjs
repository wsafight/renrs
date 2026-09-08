import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));

function run(command, args, cwd = root) {
  const result = spawnSync(command, args, {
    cwd,
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    process.stderr.write(result.stdout);
    process.stderr.write(result.stderr);
    throw new Error(`${command} ${args.join(' ')} failed`);
  }
}

for (const directory of ['.', 'web', 'launcher', 'site', 'editors/vscode-renrs']) {
  const cwd = path.join(root, directory);
  if (existsSync(path.join(cwd, 'package-lock.json'))) {
    run('npm', ['audit', '--omit=dev', '--audit-level=high'], cwd);
  }
}
// No patched release exists for these transitive or renderer advisories. Keep the
// exception list exact so every new RustSec advisory still fails the local gate.
const acceptedRustAdvisories = [
  'RUSTSEC-2025-0035', // macroquad soundness; replacement requires a renderer migration
  'RUSTSEC-2026-0192', // ttf-parser via macroquad and ab_glyph
  'RUSTSEC-2026-0206', // rustybuzz; harfrust migration is tracked separately
  'RUSTSEC-2026-0249', // smartstring via Rhai
];
run('cargo', [
  'audit',
  '--deny',
  'warnings',
  ...acceptedRustAdvisories.flatMap((advisory) => ['--ignore', advisory]),
]);
console.log('Local dependency advisory checks passed.');

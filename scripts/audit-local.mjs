import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const offline = process.argv.includes('--offline') || process.env.RENRS_AUDIT_OFFLINE === '1';

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
    run(
      'npm',
      ['audit', '--omit=dev', '--audit-level=high', ...(offline ? ['--offline'] : [])],
      cwd,
    );
  }
}
// No patched release exists for these renderer advisories. Keep each exception exact,
// owned and time-bounded so every new RustSec advisory still fails the local gate.
const acceptedRustAdvisories = [
  {
    id: 'RUSTSEC-2025-0035',
    owner: 'native renderer',
    reviewBy: '2027-02-09',
    url: 'https://rustsec.org/advisories/RUSTSEC-2025-0035',
    exit: 'Upgrade Macroquad past the advisory or complete the renderer migration.',
  },
  {
    id: 'RUSTSEC-2026-0192',
    owner: 'native text stack',
    reviewBy: '2027-02-09',
    url: 'https://rustsec.org/advisories/RUSTSEC-2026-0192',
    exit: 'Remove ttf-parser from Macroquad/fontdue and ab_glyph dependency chains.',
  },
];
const today = new Date().toISOString().slice(0, 10);
for (const advisory of acceptedRustAdvisories) {
  if (advisory.reviewBy < today) {
    console.warn(
      `RustSec exception review overdue: ${advisory.id} (${advisory.owner}); ${advisory.exit} ${advisory.url}`,
    );
  }
}
run('cargo', [
  'audit',
  ...(offline ? ['--no-fetch'] : []),
  '--deny',
  'warnings',
  ...acceptedRustAdvisories.flatMap(({ id }) => ['--ignore', id]),
]);
console.log(
  `Local dependency advisory checks passed (${offline ? 'offline cache' : 'network'} mode).`,
);

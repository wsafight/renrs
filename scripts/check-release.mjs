import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const release = JSON.parse(await readFile(path.join(root, 'release.json'), 'utf8'));
for (const file of ['site/package.json', 'editors/vscode-renrs/package.json']) {
  const package_ = JSON.parse(await readFile(path.join(root, file), 'utf8'));
  if (package_.version !== release.engine_version)
    throw new Error(`${file} version ${package_.version} does not match ${release.engine_version}`);
}
const cargo = spawnSync('cargo', ['metadata', '--offline', '--no-deps', '--format-version', '1'], {
  cwd: root,
  encoding: 'utf8',
});
if (cargo.error) throw cargo.error;
if (cargo.status !== 0) throw new Error(cargo.stderr.trim() || 'cargo metadata failed');
const metadata = JSON.parse(cargo.stdout);
const members = new Set(metadata.workspace_members);
for (const package_ of metadata.packages.filter((package_) => members.has(package_.id))) {
  if (package_.version !== release.engine_version)
    throw new Error(
      `${path.relative(root, package_.manifest_path)} version ${package_.version} does not match ${release.engine_version}`,
    );
}
console.log(`Release contract ${release.engine_version} matches workspace packages.`);

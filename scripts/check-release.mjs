import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const release = JSON.parse(await readFile(path.join(root, 'release.json'), 'utf8'));

async function requireDocumentation(file, expected) {
  const contents = await readFile(path.join(root, file), 'utf8');
  for (const fragment of expected) {
    if (!contents.includes(fragment)) {
      throw new Error(`${file} does not document release contract fragment: ${fragment}`);
    }
  }
}

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

const snapshot = release.formats.snapshot;
const saveContainer = release.formats.save_container;
await Promise.all([
  requireDocumentation('docs/RELEASE.md', [
    `| Runtime snapshot | \`${snapshot}\` |`,
    `| 桌面/Web 存档容器 | \`${saveContainer}\` |`,
  ]),
  requireDocumentation('docs/PRODUCT.md', [
    `当前运行时快照格式为 v${snapshot}`,
    `当前存档容器为 v${saveContainer}`,
  ]),
  requireDocumentation('docs/ROADMAP.md', [
    `存档写入当前快照格式 v${snapshot}`,
    `存档容器写入 v${saveContainer}`,
  ]),
  requireDocumentation('docs/UPGRADES.md', [
    `version ${snapshot} inside version ${saveContainer} save containers`,
  ]),
  requireDocumentation('docs/PERFORMANCE.md', [`current\n  container v${saveContainer}`]),
  requireDocumentation('site/i18n/docs/en/RELEASE.md', [
    `| Runtime snapshot | \`${snapshot}\` |`,
    `| Desktop/Web save container | \`${saveContainer}\` |`,
  ]),
  requireDocumentation('site/i18n/docs/en/PRODUCT.md', [
    `current runtime snapshot is v${snapshot}`,
    `current save container is v${saveContainer}`,
  ]),
  requireDocumentation('site/i18n/docs/zh/UPGRADES.md', [
    `运行时快照使用版本 ${snapshot}，放在版本 ${saveContainer} 存档容器里`,
  ]),
]);

console.log(
  `Release contract ${release.engine_version} matches workspace packages and documentation.`,
);

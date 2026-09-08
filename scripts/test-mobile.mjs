import { existsSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const [directory, ...extra] = process.argv.slice(2);
if (!directory || extra.length)
  throw new Error('Usage: node scripts/test-mobile.mjs <generated-mobile-directory>');
const project = path.resolve(directory);
const release = JSON.parse(await readFile(path.join(root, 'release.json'), 'utf8'));
const capacitor = JSON.parse(await readFile(path.join(project, 'capacitor.config.json'), 'utf8'));
const package_ = JSON.parse(await readFile(path.join(project, 'package.json'), 'utf8'));
const web = JSON.parse(await readFile(path.join(project, 'www/project.json'), 'utf8'));

if (capacitor.appId !== web.program.project_id || capacitor.appName !== web.program.title)
  throw new Error('Capacitor identity does not match the compiled RenRS project');
if (capacitor.webDir !== 'www' || capacitor.server?.androidScheme !== 'https')
  throw new Error('Capacitor WebView configuration is not release-safe');
if (capacitor.android?.allowMixedContent !== false)
  throw new Error('Capacitor must reject mixed content');
if (package_.version !== release.engine_version)
  throw new Error('Mobile package version does not match release.json');
for (const dependency of [
  '@capacitor/core',
  '@capacitor/android',
  '@capacitor/ios',
  '@capacitor/app',
  '@capacitor/filesystem',
  '@capacitor/share',
]) {
  if (!/^\d+\.\d+\.\d+$/.test(package_.dependencies?.[dependency] ?? ''))
    throw new Error(`${dependency} must use an exact version`);
}
for (const file of ['www/index.html', 'www/engine/renrs_web_bg.wasm', 'README.md']) {
  if (!existsSync(path.join(project, file))) throw new Error(`Mobile project is missing ${file}`);
}
console.log(`Mobile wrapper acceptance passed: ${project}`);

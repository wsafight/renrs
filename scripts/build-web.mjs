import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = fileURLToPath(new URL('../', import.meta.url));
function run(command, args, cwd = root) {
  const result = spawnSync(command, args, {cwd, stdio: 'inherit', shell: process.platform === 'win32'});
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run('cargo', ['build', '--locked', '-p', 'renrs-web', '--target', 'wasm32-unknown-unknown', '--release']);
const local = path.join(root, 'target/web-tools/bin', process.platform === 'win32' ? 'wasm-bindgen.exe' : 'wasm-bindgen');
run(existsSync(local) ? local : 'wasm-bindgen', ['target/wasm32-unknown-unknown/release/renrs_web.wasm', '--target', 'web', '--out-dir', 'web/public/engine']);
run('npm', ['run', 'build'], path.join(root, 'web'));

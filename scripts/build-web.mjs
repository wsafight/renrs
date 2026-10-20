import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
const root = fileURLToPath(new URL('../', import.meta.url));
function run(command, args, cwd = root) {
  const result = spawnSync(command, args, {
    cwd,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run('cargo', [
  'build',
  '--locked',
  '-p',
  'renrs-web',
  '--target',
  'wasm32-unknown-unknown',
  '--release',
]);
const local = path.join(
  root,
  'target/web-tools/bin',
  process.platform === 'win32' ? 'wasm-bindgen.exe' : 'wasm-bindgen',
);
const bindgen = existsSync(local) ? local : 'wasm-bindgen';
const expected = readFileSync(path.join(root, 'crates/web/Cargo.toml'), 'utf8').match(
  /^wasm-bindgen = "=([^"]+)"/m,
)?.[1];
const installed = spawnSync(bindgen, ['--version'], {
  encoding: 'utf8',
  shell: process.platform === 'win32',
}).stdout?.trim();
if (expected && installed !== `wasm-bindgen ${expected}`) {
  console.error(
    `wasm-bindgen CLI ${installed || 'not found'} does not match crate ${expected}. Install it with:\n` +
      `cargo install wasm-bindgen-cli --version ${expected} --locked --root target/web-tools`,
  );
  process.exit(1);
}
run(bindgen, [
  'target/wasm32-unknown-unknown/release/renrs_web.wasm',
  '--target',
  'web',
  '--out-dir',
  'web/public/engine',
]);
run('npm', ['run', 'build'], path.join(root, 'web'));

import {mkdir, cp, readFile, writeFile, readdir, stat} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {existsSync} from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const [output, binaries = path.join(root, 'target/release')] = process.argv.slice(2);
if (!output || existsSync(output)) throw new Error('Usage: node scripts/package-sdk.mjs <new-directory> [binary-directory]');
const tools = (await readdir(binaries)).filter(name => /^renrs(?:-[a-z-]+)?(?:\.exe)?$/.test(name));
for (const name of ['renrs','renrs-init','renrs-check','renrs-inspect','renrs-build','renrs-pack','renrs-web-build','renrs-graph']) if (!tools.includes(name) && !tools.includes(`${name}.exe`)) throw new Error(`Missing ${name}`);
if (!existsSync(path.join(root, 'web/dist/engine/renrs_web_bg.wasm'))) throw new Error('Build Web shell first');
const staging = path.resolve(`${output}.${process.pid}.tmp`);
await mkdir(path.join(staging, 'bin'), {recursive:true});
for (const name of tools) await cp(path.join(binaries,name),path.join(staging,'bin',name));
for (const [from,to] of [['launcher','launcher'],['scripts/launcher','scripts/launcher'],['scripts/launcher.mjs','scripts/launcher.mjs'],['web/dist','web'],['docs','docs'],['editors/vscode-renrs','editors/vscode-renrs']]) await cp(path.join(root,from),path.join(staging,to),{recursive:true,filter:source=>!source.includes('node_modules')&&!source.includes('.vscode-test')});
await mkdir(path.join(staging,'vendor'));
await cp(path.join(root,'web/node_modules/lucide/dist/umd/lucide.js'),path.join(staging,'vendor/lucide.js'));
const files = {};
async function index(directory,prefix='') {for(const entry of await readdir(directory,{withFileTypes:true})){if(entry.isDirectory())await index(path.join(directory,entry.name),`${prefix}${entry.name}/`);else if(entry.isFile())files[prefix+entry.name]=createHash('sha256').update(await readFile(path.join(directory,entry.name))).digest('hex');}}
await index(staging);
await writeFile(path.join(staging,'sdk.json'),JSON.stringify({version:1,engine_version:'0.1.0',platform:process.platform,arch:process.arch,node:'>=22',files},null,2));
await writeFile(path.join(staging,'README.txt'),'RenRS SDK\nRequires Node.js 22 or newer.\nStart: node scripts/launcher.mjs\nThe bundled player targets the platform recorded in sdk.json.\n');
const {rename} = await import('node:fs/promises'); await rename(staging,path.resolve(output));
console.log(`SDK: ${path.resolve(output)}`);

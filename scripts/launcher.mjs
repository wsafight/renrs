import {createServer} from 'node:http';
import {readFile, writeFile, mkdir, rename, realpath, readdir, stat} from 'node:fs/promises';
import {existsSync} from 'node:fs';
import {spawn} from 'node:child_process';
import {createHash, randomUUID} from 'node:crypto';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {Jobs} from './launcher/jobs.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const args = process.argv.slice(2), option = (name, fallback) => args.includes(name) ? args[args.indexOf(name) + 1] : fallback;
const port = Number(option('--port', '4185'));
const statePath = path.resolve(option('--state', process.env.RENRS_LAUNCHER_STATE || path.join(root, '.renrs/launcher.json')));
const defaultSdk = existsSync(path.join(root, 'bin')) ? root : path.join(root, 'target/debug');
let state;
try { state = JSON.parse(await readFile(statePath, 'utf8')); } catch (error) { if (error.code !== 'ENOENT') throw error; state = {sdk: defaultSdk, projects: []}; }
const jobs = new Jobs();
const digest = text => createHash('sha256').update(text).digest('hex');
async function persist() { await mkdir(path.dirname(statePath), {recursive: true}); const temp = `${statePath}.tmp`; await writeFile(temp, JSON.stringify(state, null, 2)); await rename(temp, statePath); }
function binary(name, sdk = state.sdk) { return path.join(sdk, existsSync(path.join(sdk, 'bin')) ? 'bin' : '', `${name}${process.platform === 'win32' ? '.exe' : ''}`); }
async function sdkStatus(sdk = state.sdk) {
  const tools = {};
  for (const name of ['renrs', 'renrs-init', 'renrs-check', 'renrs-inspect', 'renrs-build', 'renrs-pack', 'renrs-web-build', 'renrs-graph']) tools[name] = existsSync(binary(name, sdk));
  return {path: sdk, tools, ready: Object.values(tools).every(Boolean)};
}
function machine(executable, commandArguments, timeout = 60000) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, commandArguments, {cwd:root, shell:false, stdio:['ignore','pipe','pipe']});
    let stdout = '', stderr = '', settled = false, timer;
    const finish = callback => value => {if (!settled) {settled = true; clearTimeout(timer); callback(value);}};
    const append = (current, chunk) => {const next = current + chunk.toString(); if (Buffer.byteLength(next) > 4 * 1024 * 1024) {child.kill('SIGTERM'); throw new Error('Machine report exceeds 4 MiB');} return next;};
    child.stdout.on('data', chunk => {try {stdout = append(stdout, chunk);} catch (error) {finish(reject)(error);}});
    child.stderr.on('data', chunk => {try {stderr = append(stderr, chunk);} catch (error) {finish(reject)(error);}});
    child.once('error', finish(reject));
    child.once('close', finish(() => {try {const report = JSON.parse(stdout); if (report.protocol_version !== 1) throw new Error('Unsupported machine protocol'); resolve(report);} catch (error) {reject(new Error(stderr.trim() || error.message));}}));
    timer = setTimeout(() => {child.kill('SIGTERM'); finish(reject)(new Error('Machine inspection timed out'));}, timeout);
  });
}
function project(id) { const project = state.projects.find(item => item.id === id); if (!project) throw new Error('Project not registered'); return project; }
async function resource(project, name) {
  const base = await realpath(project.path), target = await realpath(path.join(base, name));
  if (!target.startsWith(`${base}${path.sep}`)) throw new Error('Resource outside project');
  if ((await stat(target)).size > 4 * 1024 * 1024) throw new Error('Resource exceeds 4 MiB');
  return target;
}
async function scripts(directory, prefix = '', depth = 0) {
  if (depth > 16) return [];
  const files = [];
  for (const entry of await readdir(directory, {withFileTypes: true})) {
    if (entry.name.startsWith('.') || ['dist', 'node_modules', 'target'].includes(entry.name)) continue;
    if (entry.isDirectory()) files.push(...await scripts(path.join(directory, entry.name), `${prefix}${entry.name}/`, depth + 1));
    else if (entry.isFile() && entry.name.endsWith('.rns')) files.push(prefix + entry.name);
    if (files.length > 1024) throw new Error('Project exceeds 1024 scripts');
  }
  return files.sort();
}
async function body(request) { let body = ''; for await (const chunk of request) { body += chunk; if (Buffer.byteLength(body) > 1024 * 1024) throw new Error('Request exceeds 1 MiB'); } return JSON.parse(body || '{}'); }
async function register(directory) {
  const location = await realpath(directory);
  if (!(await stat(location)).isDirectory() || !(await scripts(location)).length) throw new Error('Directory has no .rns scripts');
  let item = state.projects.find(item => item.path === location);
  if (!item) { item = {id: randomUUID(), path: location, name: path.basename(location)}; state.projects.push(item); await persist(); }
  return item;
}
async function api(request, url) {
  if (request.method === 'GET') {
    if (url.pathname === '/api/state') return {...state, sdk: await sdkStatus(), jobs: jobs.list()};
    if (url.pathname === '/api/scripts') return scripts(project(url.searchParams.get('id')).path);
    if (url.pathname === '/api/script') { const file = await resource(project(url.searchParams.get('id')), url.searchParams.get('file')); if (!file.endsWith('.rns')) throw new Error('Expected .rns'); const text = await readFile(file, 'utf8'); return {text, revision: digest(text)}; }
    if (url.pathname === '/api/inspection') { const item = project(url.searchParams.get('id')); return machine(binary('renrs-inspect'), [item.path]); }
    throw new Error('Unknown endpoint');
  }
  if (request.method !== 'POST') throw new Error('Method not allowed');
  const data = await body(request);
  switch (url.pathname) {
    case '/api/register': return register(data.path);
    case '/api/remove': state.projects = state.projects.filter(item => item.id !== data.id); await persist(); return {};
    case '/api/sdk': { const sdk = await realpath(data.path); const status = await sdkStatus(sdk); if (!status.ready) throw new Error('SDK is missing required executables'); state.sdk = sdk; await persist(); return status; }
    case '/api/cancel': jobs.cancel(data.id); return {};
    case '/api/create': {
      const destination = path.resolve(data.path);
      if (existsSync(destination)) throw new Error('Destination already exists');
      const job = jobs.start('Create project', binary('renrs-init'), [destination, '--title', data.title, '--id', data.projectId, '--template', data.template || 'story'], root, 600000, () => register(destination));
      return jobs.public(job);
    }
    case '/api/script': {
      const file = await resource(project(data.id), data.file); if (!file.endsWith('.rns') || typeof data.text !== 'string') throw new Error('Expected script');
      if (digest(await readFile(file, 'utf8')) !== data.revision) throw new Error('File changed on disk. Reload before saving.');
      const temporary = `${file}.${randomUUID()}.tmp`; await writeFile(temporary, data.text); await rename(temporary, file); return {revision: digest(data.text)};
    }
    case '/api/task': {
      const item = project(data.id), output = data.output && path.resolve(data.output);
      const commands = {check: ['renrs-check', [item.path]], run: ['renrs', [item.path]], graph: ['renrs-graph', [item.path]], pack: ['renrs-pack', [item.path, output]], build: ['renrs-build', [item.path, output]], web: ['renrs-web-build', [item.path, output, '--shell', existsSync(path.join(state.sdk, 'web')) ? path.join(state.sdk, 'web') : path.join(root, 'web/dist')]]};
      const command = commands[data.action]; if (!command || command[1].some(value => !value)) throw new Error('Invalid task or missing output directory');
      if (['pack','build','web'].includes(data.action) && existsSync(output)) throw new Error('Output already exists');
      return jobs.public(jobs.start(`${item.name}: ${data.action}`, binary(command[0]), command[1], root, data.action === 'run' ? 0 : 600000));
    }
    default: throw new Error('Unknown endpoint');
  }
}
const types = {'.html':'text/html; charset=utf-8','.css':'text/css','.js':'text/javascript','.mjs':'text/javascript','.png':'image/png'};
let mutations = Promise.resolve();
const server = createServer(async (request, response) => {
  try {
    const origin = `http://127.0.0.1:${server.address().port}`;
    if (request.headers.host !== new URL(origin).host || request.headers.origin && request.headers.origin !== origin) throw new Error('Local origin required');
    const url = new URL(request.url, origin);
    response.setHeader('Cache-Control', 'no-store'); response.setHeader('X-Content-Type-Options', 'nosniff');
    if (url.pathname.startsWith('/api/')) {
      if (request.method === 'POST' && request.headers['content-type'] !== 'application/json') throw new Error('JSON required');
      const result = request.method === 'POST' ? await (mutations = mutations.catch(() => {}).then(() => api(request, url))) : await api(request, url);
      response.setHeader('Content-Type', 'application/json'); response.end(JSON.stringify(result)); return;
    }
    let file;
    if (url.pathname === '/thumbnail') file = await resource(project(url.searchParams.get('id')), 'images/studio.png');
    else if (url.pathname === '/lucide.js') file = path.join(root, existsSync(path.join(root, 'vendor/lucide.js')) ? 'vendor/lucide.js' : 'web/node_modules/lucide/dist/umd/lucide.js');
    else { const names = {'/':'index.html','/app.js':'app.js','/style.css':'style.css'}; if (!names[url.pathname]) throw new Error('Not found'); file = path.join(root, 'launcher', names[url.pathname]); }
    response.setHeader('Content-Type', types[path.extname(file)] || 'application/octet-stream'); response.end(await readFile(file));
  } catch (error) { response.writeHead(400, {'Content-Type':'application/json'}); response.end(JSON.stringify({error:error.message})); }
});
server.listen(port, '127.0.0.1', () => console.log(`RenRS Launcher: http://127.0.0.1:${server.address().port}`));
for (const signal of ['SIGINT','SIGTERM']) process.on(signal, () => { jobs.stop(); server.close(() => process.exit()); });

import {spawn} from 'node:child_process';
import {existsSync} from 'node:fs';
import {mkdtemp, readFile, realpath, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import path from 'node:path';

const [sdkArgument] = process.argv.slice(2);
if (!sdkArgument) throw new Error('Usage: node scripts/test-launcher.mjs <sdk-directory>');
const sdk = path.resolve(sdkArgument);
const launcher = path.join(sdk, 'scripts/launcher.mjs');
if (!existsSync(launcher)) throw new Error(`Packaged launcher is missing: ${launcher}`);

const temporary = await mkdtemp(path.join(tmpdir(), 'renrs-launcher-'));
const child = spawn(process.execPath, [launcher, '--port', '0', '--state', path.join(temporary, 'state.json')], {
  cwd: sdk,
  stdio: ['ignore', 'pipe', 'inherit'],
});

async function origin() {
  return new Promise((resolve, reject) => {
    let output = '';
    const timer = setTimeout(() => reject(new Error('Timed out waiting for Launcher')), 10_000);
    child.once('error', reject);
    child.stdout.on('data', chunk => {
      output += chunk;
      const match = output.match(/RenRS Launcher: (http:\/\/127\.0\.0\.1:\d+)/);
      if (match) {
        clearTimeout(timer);
        resolve(match[1]);
      }
    });
    child.once('exit', code => reject(new Error(`Launcher exited early with code ${code}`)));
  });
}

async function request(base, pathname, data) {
  const response = await fetch(`${base}${pathname}`, data === undefined ? {} : {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify(data),
  });
  const result = await response.json();
  if (!response.ok) throw new Error(result.error || `${pathname} returned ${response.status}`);
  return result;
}

async function job(base, started) {
  for (let attempt = 0; attempt < 600; attempt += 1) {
    const state = await request(base, '/api/state');
    const current = state.jobs.find(item => item.id === started.id);
    if (current?.code === 0) return current;
    if (current?.code != null) throw new Error(`${current.name} failed:\n${current.log}`);
    await new Promise(resolve => setTimeout(resolve, 100));
  }
  throw new Error(`${started.name} did not finish`);
}

async function task(base, id, action, output) {
  const started = await request(base, '/api/task', {id, action, output});
  return job(base, started);
}

try {
  const base = await origin();
  const initial = await request(base, '/api/state');
  if (!initial.sdk.ready) throw new Error('Packaged SDK does not expose all Launcher tools');

  const projectPath = path.join(temporary, 'story');
  await job(base, await request(base, '/api/create', {
    path: projectPath,
    title: 'Launcher Acceptance',
    projectId: 'org.renrs.launcher_acceptance',
    template: 'story',
  }));
  const state = await request(base, '/api/state');
  const canonicalProject = await realpath(projectPath);
  const project = state.projects.find(item => item.path === canonicalProject);
  if (!project) throw new Error('Created project was not registered');

  const inspection = await request(base, `/api/inspection?id=${encodeURIComponent(project.id)}`);
  if (inspection.protocol_version !== 1 || !inspection.ok) throw new Error('Launcher inspection did not return a valid machine report');
  if (inspection.data.routes.passed !== 2 || inspection.data.routes.coverage.uncovered_labels.length) throw new Error('Launcher inspection route coverage is incomplete');

  const files = await request(base, `/api/scripts?id=${encodeURIComponent(project.id)}`);
  const scriptFile = files.find(file => file.endsWith('.rns'));
  if (!scriptFile) throw new Error('Created project scripts were not indexed');
  const query = `?id=${encodeURIComponent(project.id)}&file=${encodeURIComponent(scriptFile)}`;
  const script = await request(base, `/api/script${query}`);
  const updated = await request(base, '/api/script', {
    id: project.id,
    file: scriptFile,
    revision: script.revision,
    text: `${script.text}\n# Launcher acceptance\n`,
  });
  if (updated.revision === script.revision) throw new Error('Script revision did not change');
  const stale = await fetch(`${base}/api/script`, {
    method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({id: project.id, file: scriptFile, revision: script.revision, text: script.text}),
  });
  if (stale.ok) throw new Error('Launcher accepted a stale script overwrite');

  await task(base, project.id, 'check');
  await task(base, project.id, 'graph');
  const archive = path.join(temporary, 'story.renrs');
  const distribution = path.join(temporary, 'distribution');
  const web = path.join(temporary, 'web');
  await task(base, project.id, 'pack', archive);
  await task(base, project.id, 'build', distribution);
  await task(base, project.id, 'web', web);
  for (const artifact of [archive, path.join(distribution, 'game.renrs'), path.join(web, 'index.html')]) {
    if (!existsSync(artifact)) throw new Error(`Launcher task did not create ${artifact}`);
  }

  await request(base, '/api/remove', {id: project.id});
  if (!existsSync(path.join(projectPath, scriptFile))) throw new Error('Removing a project deleted its files');
  if ((await request(base, '/api/state')).projects.some(item => item.id === project.id)) {
    throw new Error('Removed project is still registered');
  }
  const persisted = JSON.parse(await readFile(path.join(temporary, 'state.json'), 'utf8'));
  if (persisted.projects.length !== 0) throw new Error('Removed project remained in persisted state');
  console.log('Packaged Launcher and SDK acceptance passed.');
} finally {
  if (child.exitCode === null) {
    const exited = new Promise(resolve => child.once('exit', resolve));
    child.kill('SIGTERM');
    await exited;
  }
  await rm(temporary, {recursive: true, force: true});
}

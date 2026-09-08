import { spawn, execFileSync } from 'node:child_process';
import { readFile, writeFile, mkdir, access } from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { createHash } from 'node:crypto';

const root = path.resolve(process.argv[2] || 'target/engine-comparison');
const sdk = path.resolve(process.argv[3] || 'target/renpy-sdk/renpy-8.5.3-sdk');
const repeats = Number(process.argv[4] || 3);
if (!Number.isInteger(repeats) || repeats < 1 || repeats > 10)
  throw new Error('repeats must be 1..10');
const output = path.join(root, `results-${Date.now()}`);
await mkdir(output);
const binary = path.resolve('target/release/renrs');
const binarySha256 = createHash('sha256')
  .update(await readFile(binary))
  .digest('hex');
const exists = async (file) =>
  access(file).then(
    () => true,
    () => false,
  );
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const sample = (pid) => {
  const fields = execFileSync('ps', ['-p', String(pid), '-o', 'time=,rss='], { encoding: 'utf8' })
    .trim()
    .split(/\s+/);
  const parts = fields[0].split(':').map(Number);
  return {
    at: performance.now(),
    cpu: parts.reduce((sum, value) => sum * 60 + value, 0),
    rss: Number(fields[1]) * 1024,
  };
};
const median = (values) => values.sort((a, b) => a - b)[Math.floor(values.length / 2)];
const results = [];
for (const scenario of ['idle', 'motion'])
  for (let repeat = 0; repeat < repeats; repeat++) {
    // Alternate order between repetitions to reduce cache/thermal order bias.
    for (const engine of repeat % 2 ? ['renpy', 'renrs'] : ['renrs', 'renpy']) {
      const report = path.join(output, `${scenario}-${engine}-${repeat}.json`);
      const command = engine === 'renrs' ? binary : path.join(sdk, 'renpy.sh');
      const args =
        engine === 'renrs'
          ? [path.join(root, `renrs-${scenario}`), '--benchmark', report]
          : [path.join(root, `renpy-${scenario}`)];
      const launched = performance.now();
      let log = '',
        finished = false;
      const child = spawn(command, args, {
        env: {
          ...process.env,
          RENRS_COMPARISON_OUTPUT: report,
          RENRS_COMPARISON_SCENARIO: scenario,
        },
      });
      child.stdout.on('data', (data) => (log += data));
      child.stderr.on('data', (data) => (log += data));
      const completion = new Promise((resolve, reject) => {
        child.on('error', reject);
        child.on('exit', (code, signal) => {
          finished = true;
          resolve({ code, signal });
        });
      });
      const timeout = setTimeout(() => child.kill('SIGTERM'), 45000);
      try {
        while (!finished && !(await exists(report.replace('.json', '.ready.json'))))
          await delay(50);
        if (finished) throw new Error(log || 'engine exited before readiness');
        const readyAfterMs = performance.now() - launched;
        const first = sample(child.pid),
          samples = [first];
        while (!finished && !(await exists(report))) {
          await delay(250);
          if (!finished && !(await exists(report))) {
            const current = sample(child.pid);
            // The report can appear while ps is running; exclude capture-time samples.
            if (!(await exists(report))) samples.push(current);
          }
        }
        const status = await completion;
        if (status.code !== 0) throw new Error(log || JSON.stringify(status));
        if (samples.length < 2) throw new Error('insufficient process samples');
        const last = samples.at(-1),
          data = JSON.parse(await readFile(report, 'utf8'));
        const pixels = JSON.parse(
          execFileSync(
            path.resolve('target/release/examples/inspect_frames'),
            [report.replace('.json', '.start.png'), report.replace('.json', '.end.png')],
            { encoding: 'utf8' },
          ),
        );
        if (pixels.colors < 32 || (scenario === 'motion' && pixels.changed_pixels < 500))
          throw new Error(`blank or stationary scene: ${JSON.stringify(pixels)}`);
        const result = {
          engine_id: engine,
          scenario,
          repeat,
          report,
          ready_after_ms: readyAfterMs,
          sample_seconds: (last.at - first.at) / 1000,
          cpu_percent: ((last.cpu - first.cpu) / ((last.at - first.at) / 1000)) * 100,
          peak_rss_bytes: Math.max(...samples.map((item) => item.rss)),
          pixels,
          ...data,
        };
        results.push(result);
        await writeFile(
          report.replace('.json', '.process.json'),
          JSON.stringify({ result, samples, log }, null, 2),
        );
        console.log(
          `${scenario} ${engine} #${repeat + 1}: CPU ${result.cpu_percent.toFixed(2)}%, RSS ${(result.peak_rss_bytes / 1048576).toFixed(1)} MiB, draws ${result.scene_redraws}`,
        );
      } finally {
        clearTimeout(timeout);
        if (!finished) {
          child.kill('SIGTERM');
          await completion;
        }
      }
    }
  }
const summary = [];
for (const scenario of ['idle', 'motion'])
  for (const engine of ['renrs', 'renpy']) {
    const selected = results.filter((row) => row.scenario === scenario && row.engine_id === engine);
    summary.push({
      scenario,
      engine: selected[0].engine,
      cpu_percent: median(selected.map((row) => row.cpu_percent)),
      rss_mib: median(selected.map((row) => row.peak_rss_bytes / 1048576)),
      scene_redraws: median(selected.map((row) => row.scene_redraws)),
    });
  }
await writeFile(
  path.join(output, 'summary.json'),
  JSON.stringify(
    {
      platform: `${os.platform()}-${os.arch()}`,
      cpu: os.cpus()[0].model,
      repeats,
      binary_sha256: binarySha256,
      renpy_sdk: sdk,
      manifest: JSON.parse(await readFile(path.join(root, 'manifest.json'), 'utf8')),
      summary,
      results,
    },
    null,
    2,
  ),
);
console.log(`Results: ${output}`);

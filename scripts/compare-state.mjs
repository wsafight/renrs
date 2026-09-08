import { execFileSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';

const root = path.resolve(process.argv[2] || 'target/engine-comparison');
const output = path.join(root, `state-results-${Date.now()}`);
await mkdir(output);
const binaries = {
  before: path.resolve('target/renrs-bench-before-p1'),
  after: path.resolve('target/release/renrs-bench'),
};
const binarySha256 = {};
for (const [version, binary] of Object.entries(binaries))
  binarySha256[version] = createHash('sha256')
    .update(await readFile(binary))
    .digest('hex');
const results = [];
for (let repeat = 0; repeat < 3; repeat++) {
  for (const version of repeat % 2 ? ['after', 'before'] : ['before', 'after']) {
    const binary = binaries[version];
    const report = JSON.parse(
      execFileSync(binary, [path.join(root, 'large-state'), '3'], {
        encoding: 'utf8',
        timeout: 120000,
      }),
    );
    results.push({ version, repeat, ...report });
    await writeFile(
      path.join(output, `${version}-${repeat}.json`),
      JSON.stringify(report, null, 2),
    );
    console.log(
      `${version} #${repeat + 1}: play ${report.play_ms.toFixed(2)} ms, snapshot ${report.snapshot_serialize_ms.toFixed(2)} ms, restore ${report.restore_ms.toFixed(2)} ms, save ${report.save_ms.toFixed(2)} ms`,
    );
  }
}
const median = (values) => values.sort((a, b) => a - b)[1];
for (const key of ['instructions', 'interactions', 'snapshot_bytes']) {
  if (new Set(results.map((row) => row[key])).size !== 1)
    throw new Error(`workload differs: ${key}`);
}
const summary = ['before', 'after'].map((version) => {
  const rows = results.filter((row) => row.version === version);
  return {
    version,
    ...Object.fromEntries(
      [
        'play_ms',
        'snapshot_serialize_ms',
        'restore_ms',
        'save_ms',
        'listing_ms',
        'snapshot_bytes',
      ].map((key) => [key, median(rows.map((row) => row[key]))]),
    ),
  };
});
await writeFile(
  path.join(output, 'summary.json'),
  JSON.stringify({ binary_sha256: binarySha256, summary, results }, null, 2),
);
console.log(output);

import {mkdir, copyFile, readFile, writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import path from 'node:path';

const destination = path.resolve(process.argv[2] || 'target/engine-comparison');
await mkdir(destination);
const assets = {'studio.png': 'demo/images/studio.png', 'mira.png': 'demo/images/mira.png', 'font.ttf': 'assets/fonts/NotoSansSC-Medium.ttf'};
const hashes = {};
for (const [name, source] of Object.entries(assets)) hashes[name] = createHash('sha256').update(await readFile(source)).digest('hex');
for (const mode of ['idle', 'motion']) {
  const native = path.join(destination, `renrs-${mode}`), renpy = path.join(destination, `renpy-${mode}`), game = path.join(renpy, 'game');
  await mkdir(native); await mkdir(game, {recursive: true});
  for (const [name, source] of Object.entries(assets)) {
    await copyFile(source, path.join(native, name)); await copyFile(source, path.join(game, name));
  }
  await copyFile('scripts/benchmarks/renpy.rpy', path.join(game, 'script.rpy'));
  await writeFile(path.join(native, 'theme.json'), JSON.stringify({font_path: 'font.ttf', dialogue_font_size: 30, dialogue_line_height: 40}));
  await writeFile(path.join(native, 'script.rns'), `config title "Engine Comparison"
config id "org.renrs.comparison.${mode}"
label start:
    scene "studio.png"
    show "mira.png" as mira at left
    ${mode === 'motion' ? 'transform mira x 400 over 30' : '"The receiver found another signal. Same time, same impossible frequency."'}
    return
`);
}
// A nested inventory stays constant while a scalar changes at every interaction.
const large = path.join(destination, 'large-state'); await mkdir(large);
const leaf = `list(${Array.from({length: 64}, (_, i) => JSON.stringify(`item-${i}-` + 'x'.repeat(120))).join(', ')})`;
await writeFile(path.join(large, 'script.rns'), `config id "org.renrs.comparison.state"
default bag = list()
default score = 0
label start:
${Array(16).fill(`    set bag = push(bag, ${leaf})`).join('\n')}
${Array.from({length: 400}, (_, i) => `    set score = ${i}\n    "Record ${i}."`).join('\n')}
    return
`);
await writeFile(path.join(destination, 'manifest.json'), JSON.stringify({assets: hashes, canvas: [1280,720], sprite_size: [440,650], motion: {x: [84,484], seconds: 30}, audio: 'none', warmup_seconds: 2, measurement_seconds: 8}, null, 2));
console.log(destination);

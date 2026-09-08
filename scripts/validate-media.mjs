import { spawnSync } from 'node:child_process';
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
const [destination = 'target/media-fixture', ...options] = process.argv.slice(2);
if (options.some((option) => option !== '--stream') || options.length > 1)
  throw new Error('Usage: node scripts/validate-media.mjs [directory] [--stream]');
const streaming = options.includes('--stream');
const root = path.resolve(destination);
const ffmpeg =
  process.env.RENRS_FFMPEG ||
  path.resolve('target/media-tools/node_modules/@ffmpeg-installer/darwin-arm64/ffmpeg');
function run(command, args, env = process.env) {
  const result = spawnSync(command, args, { stdio: 'inherit', env });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed`);
}
await mkdir(root, { recursive: true });
await cp('demo/images', path.join(root, 'images'), { recursive: true });
run(ffmpeg, [
  '-nostdin',
  '-v',
  'error',
  '-f',
  'lavfi',
  '-i',
  'testsrc2=size=640x360:rate=24',
  '-f',
  'lavfi',
  '-i',
  'sine=frequency=440:sample_rate=48000',
  '-t',
  '2',
  '-c:v',
  'libx264',
  '-pix_fmt',
  'yuv420p',
  '-n',
  path.join(root, 'input.mp4'),
]);
run(
  './target/debug/renrs-video',
  [path.join(root, 'input.mp4'), root, 'clips/intro', ...(streaming ? ['--stream'] : [])],
  {
    ...process.env,
    RENRS_FFMPEG: ffmpeg,
  },
);
const clipDirectory = path.join(root, 'clips/intro');
const soundtrack = path.join(clipDirectory, 'audio.wav');
await Promise.all([
  cp(soundtrack, path.join(clipDirectory, 'audio-en.wav')),
  cp(soundtrack, path.join(clipDirectory, 'audio-zh.wav')),
]);
const manifestPath = path.join(clipDirectory, 'clip.json');
const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
delete manifest.audio;
manifest.audio_tracks = [
  {
    path: 'clips/intro/audio-en.wav',
    language: 'en',
    label: 'English',
    default: true,
    volume: 0.5,
  },
  {
    path: 'clips/intro/audio-zh.wav',
    language: 'zh-Hans',
    label: '简体中文',
    volume: 0.8,
  },
];
manifest.subtitles = [
  {
    language: 'en',
    label: 'English',
    default: true,
    cues: [{ start: 0, end: 1.6, text: 'Signal received.' }],
  },
  {
    language: 'zh-Hans',
    label: '简体中文',
    cues: [{ start: 0, end: 1.6, text: '信号已收到。' }],
  },
];
await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
await writeFile(
  path.join(root, 'script.rns'),
  `config title "RenRS Media Test"
config id "org.renrs.media-test"
default name = "Reader"
label start:
    scene "images/studio.png"
    show "images/mira.png" as mira at center
    "Media checkpoint."
    timeline:
        @id "media.move" transform mira x 100 over 0.4 ease in_out
        @id "media.alpha" transform mira alpha 0.5 over 0.4
    scene "images/rooftop.png"
    @id "media.dissolve" transition dissolve 0.5
    @id "media.video" video "clips/intro/clip.json" over 2
    "Video finished."
    menu:
        "Finish":
            jump ending
        "Replay":
            jump start
label ending:
    "Thank you, [name]."
    return
`,
);
await writeFile(path.join(root, 'resources.json'), JSON.stringify({ exclude: ['input.mp4'] }));
await writeFile(
  path.join(root, 'progress.json'),
  JSON.stringify({
    achievements: [{ id: 'media', title: 'Media complete', label: 'ending' }],
    gallery: [{ id: 'roof', title: 'Rooftop', label: 'ending', image: 'images/rooftop.png' }],
    endings: [{ id: 'ending', title: 'Completed', label: 'ending' }],
    rollback_barriers: ['ending'],
  }),
);
console.log(root);

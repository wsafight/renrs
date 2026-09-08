import assert from 'node:assert/strict';
import { test } from 'vitest';
import type { AudioApp } from '../src/audio';
import { setupAudio } from '../src/audio';
import type { AudioEvent } from '../src/protocol';
import type { AudioHandle, PlayerSettings } from '../src/types';

class FakeAudio implements AudioHandle {
  currentTime = 0;
  dataset = {} as DOMStringMap;
  ended = false;
  loop = false;
  onended: AudioHandle['onended'] = null;
  onerror: OnErrorEventHandler = null;
  onloadeddata: AudioHandle['onloadeddata'] = null;
  paused = true;
  preload = '';
  released = false;
  src: string;
  volume = 1;

  constructor(src = '') {
    this.src = src;
  }
  async play() {
    this.paused = false;
  }
  pause() {
    this.paused = true;
  }
  removeAttribute(_name: string) {
    this.src = '';
  }
  load() {
    this.released = true;
  }
}
const settings = (): PlayerSettings => ({
  fontSize: 20,
  music_volume: 0.6,
  sound_volume: 0.8,
  voice_volume: 1,
  auto: false,
  skip: false,
  text_speed: 40,
  auto_delay: 2.5,
  wait_voice: true,
  self_voicing: false,
  contrast: false,
  reduced: false,
  language: '',
});
function fixture() {
  const events: AudioEvent[] = [];
  const app: AudioApp = {
    voices: new Map(),
    asset: (path: string) => path,
    settings: settings(),
    state: { stage: {} },
    notify: () => {},
    engine: {
      audio_events: () => JSON.stringify(events.splice(0)),
      music_ended: () => {},
      state: () => '',
    },
  };
  return { app, events, audio: setupAudio(app, FakeAudio) };
}
test('sound bursts stay bounded and ended/error channels release their resources', async () => {
  const { app, events, audio } = fixture();
  events.push(...Array.from({ length: 100 }, () => ({ PlaySound: { path: 'sound.wav' } })));
  await audio.events();
  assert.equal(app.voices.size, 8);
  const ended = [...app.voices.values()][0] as FakeAudio;
  ended.onended?.(new Event('ended'));
  assert.equal(app.voices.size, 7);
  assert.equal(ended.released, true);
  const failed = [...app.voices.values()][0] as FakeAudio;
  if (typeof failed.onerror === 'function') failed.onerror(new Event('error'));
  assert.equal(app.voices.size, 6);
  assert.equal(failed.released, true);
  audio.reset();
  assert.equal(app.voices.size, 0);
});
test('completed voice is released without restarting until explicitly replayed', async () => {
  const { app, events, audio } = fixture();
  app.state.stage.voice = 'voice.wav';
  events.push({ PlayVoice: { path: 'voice.wav' } });
  await audio.events();
  const first = app.voices.get('voice');
  first?.onended?.(new Event('ended'));
  await audio.events();
  assert.equal(app.voices.size, 0);
  await audio.replay('voice.wav');
  assert.notEqual(app.voices.get('voice'), first);
  audio.reset();
  await audio.events();
  assert.equal(app.voices.size, 1);
});
test('relative media volume multiplies channel preferences', async () => {
  const { app, events, audio } = fixture();
  app.settings = settings();
  app.state.stage.music = { path: 'music.ogg', repeat: true, volume: 0.5 };
  events.push({ PlaySound: { path: 'sound.wav', volume: 0.25 } });
  await audio.events();
  assert.equal(app.voices.get('music')?.volume, 0.3);
  assert.equal(app.voices.get('sound-1')?.volume, 0.2);
  const first = app.voices.get('music');
  app.state.stage.music.volume = 0.25;
  await audio.events();
  assert.notEqual(app.voices.get('music'), first);
  assert.equal(app.voices.get('music')?.volume, 0.15);
});

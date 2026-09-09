import assert from 'node:assert/strict';
import { test } from 'vitest';
import type { AudioApp } from '../src/audio';
import { setupAudio } from '../src/audio';
import { type AudioEvent, parseRuntimeState } from '../src/protocol';
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
function fixture(omitStage = false) {
  const events: AudioEvent[] = [];
  const app: AudioApp = {
    voices: new Map(),
    asset: (path: string) => path,
    settings: settings(),
    state: parseRuntimeState(
      JSON.stringify({
        stage: { sprites: [], camera: {} },
        waiting: null,
        debug: { paused: false },
        profile_revision: 0,
        history_count: 0,
        can_rollback: false,
      }),
    ),
    notify: () => {},
    engine: {
      audio_events: () => JSON.stringify(events.splice(0)),
      music_ended: () => {},
      sound_ended: () => {
        app.state.stage.sound = null;
      },
      state: () =>
        JSON.stringify({
          stage: omitStage ? null : app.state.stage,
          waiting: null,
          debug: { paused: false },
          profile_revision: 0,
          history_count: 0,
          can_rollback: false,
        }),
    },
  };
  return { app, events, audio: setupAudio(app, FakeAudio) };
}
test('audio completion reuses the previous stage when the engine omits it', async () => {
  const { app, events, audio } = fixture(true);
  app.state.stage.sound = { path: 'sound.wav', repeat: false, volume: 1 };
  events.push({ PlaySound: { path: 'sound.wav', repeat: false, volume: 1 } });
  await audio.events();

  const ended = app.voices.get('sound');
  assert.doesNotThrow(() => ended?.onended?.(new Event('ended')));
  assert.equal(app.state.stage.sound, null);
});
test('playing a sound replaces the channel and ended/error states release resources', async () => {
  const { app, events, audio } = fixture();
  app.state.stage.sound = { path: 'sound.wav', repeat: false, volume: 1 };
  events.push(
    ...Array.from({ length: 100 }, () => ({
      PlaySound: { path: 'sound.wav', repeat: false, volume: 1 },
    })),
  );
  await audio.events();
  assert.equal(app.voices.size, 1);
  const ended = app.voices.get('sound') as FakeAudio;
  ended.onended?.(new Event('ended'));
  assert.equal(app.voices.size, 0);
  assert.equal(ended.released, true);
  app.state.stage.sound = { path: 'broken.wav', repeat: false, volume: 1 };
  events.push({ PlaySound: { path: 'broken.wav', repeat: false, volume: 1 } });
  await audio.events();
  const failed = app.voices.get('sound') as FakeAudio;
  if (typeof failed.onerror === 'function') failed.onerror(new Event('error'));
  assert.equal(app.voices.size, 0);
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
  app.state.stage.sound = { path: 'sound.wav', repeat: false, volume: 0.25 };
  events.push({ PlaySound: { path: 'sound.wav', volume: 0.25, repeat: false } });
  await audio.events();
  assert.equal(app.voices.get('music')?.volume, 0.3);
  assert.equal(app.voices.get('sound')?.volume, 0.2);
  const first = app.voices.get('music');
  app.state.stage.music.volume = 0.25;
  await audio.events();
  assert.notEqual(app.voices.get('music'), first);
  assert.equal(app.voices.get('music')?.volume, 0.15);
});

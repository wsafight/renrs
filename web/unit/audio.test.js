import {test} from 'node:test';
import assert from 'node:assert/strict';
import {setupAudio} from '../src/audio.js';

class FakeAudio {
  constructor(src) { this.src = src; this.dataset = {}; }
  async play() { this.paused = false; }
  pause() { this.paused = true; }
  removeAttribute() { this.src = ''; }
  load() { this.released = true; }
}
function fixture() {
  let events = [];
  const app = {voices:new Map(), asset:path=>path, settings:{volume:1}, state:{stage:{}}, notify:()=>{},
    engine:{audio_events:()=>JSON.stringify(events.splice(0))}};
  return {app, events, audio:setupAudio(app, FakeAudio)};
}
test('sound bursts stay bounded and ended/error channels release their resources', async () => {
  const {app, events, audio} = fixture();
  events.push(...Array.from({length:100}, ()=>({PlaySound:{path:'sound.wav'}})));
  await audio.events(); assert.equal(app.voices.size, 8);
  const ended = [...app.voices.values()][0]; ended.onended();
  assert.equal(app.voices.size, 7); assert.equal(ended.released, true);
  const failed = [...app.voices.values()][0]; failed.onerror();
  assert.equal(app.voices.size, 6); assert.equal(failed.released, true);
  audio.reset(); assert.equal(app.voices.size, 0);
});
test('completed voice is released without restarting until explicitly replayed', async () => {
  const {app, events, audio} = fixture();
  app.state.stage.voice = 'voice.wav'; events.push({PlayVoice:{path:'voice.wav'}});
  await audio.events(); const first = app.voices.get('voice'); first.onended();
  await audio.events(); assert.equal(app.voices.size, 0);
  await audio.replay('voice.wav'); assert.notEqual(app.voices.get('voice'), first);
  audio.reset(); await audio.events(); assert.equal(app.voices.size, 1);
});

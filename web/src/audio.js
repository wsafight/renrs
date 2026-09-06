import {audioVolume} from './presentation.js';

export function setupAudio(app, AudioClass = Audio) {
  const channels = app.voices;
  let sequence = 0, voicePath = null;
  function stop(channel) {
    const audio = channels.get(channel);
    if (!audio) return;
    channels.delete(channel);
    audio.onended = audio.onerror = null;
    audio.pause(); audio.removeAttribute('src'); audio.load();
    app.refreshLayerClock?.();
  }
  function reset() {
    for (const channel of [...channels.keys()]) stop(channel);
    voicePath = null;
  }
  async function play(path, channel, repeat = false) {
    stop(channel);
    if (channel.startsWith('sound-')) {
      const sounds = [...channels.keys()].filter(key => key.startsWith('sound-'));
      if (sounds.length >= 8) stop(sounds[0]);
    }
    if (channel === 'voice') voicePath = path;
    const audio = new AudioClass(app.asset(path));
    audio.loop = repeat; audio.volume = audioVolume(app.settings, channel); channels.set(channel, audio);
    audio.onended = () => {
      if (channels.get(channel) !== audio) return;
      stop(channel);
      if (channel === 'music') {
        app.engine.music_ended(); app.state = JSON.parse(app.engine.state());
        events().catch(app.notify);
      }
    };
    audio.onerror = () => { if (channels.get(channel) === audio) stop(channel); app.notify(`Could not play ${path}`); };
    try { await audio.play(); app.refreshLayerClock?.(); } catch (error) {
      if (channels.get(channel) !== audio) return;
      if (error.name === 'NotAllowedError') audio.dataset.pendingGesture = 'true';
      else { stop(channel); throw error; }
    }
  }
  async function events() {
    for (const event of JSON.parse(app.engine.audio_events())) {
      if (event.PlaySound) await play(event.PlaySound.path, `sound-${++sequence}`);
      if (event.PlayVoice) await play(event.PlayVoice.path, 'voice');
      if (event === 'StopVoice') { stop('voice'); voicePath = null; }
      if (event.StopMusic) stop('music');
    }
    const music = app.state.stage.music;
    if (music && channels.get('music')?.src !== app.asset(music.path)) await play(music.path, 'music', music.repeat);
    if (!music) stop('music');
    const voice = app.state.stage.voice;
    if (voice && voice !== voicePath) await play(voice, 'voice');
    if (!voice) { stop('voice'); voicePath = null; }
  }
  function resume() {
    for (const audio of channels.values()) {
      if (audio.dataset.pendingGesture) audio.play().then(() => delete audio.dataset.pendingGesture).catch(() => {});
    }
  }
  return {events, reset, resume, replay: path => play(path, 'voice')};
}

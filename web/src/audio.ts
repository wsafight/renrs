import { audioVolume } from './presentation';
import { parseAudioEvents, parseRuntimeState } from './protocol';
import type { AudioConstructor, AudioHandle, PlayerSettings, RuntimeState } from './types';

export interface AudioApp {
  voices: Map<string, AudioHandle>;
  settings: PlayerSettings;
  state: RuntimeState;
  engine: {
    audio_events(): string;
    music_ended(): void;
    sound_ended(): void;
    state(): string;
  };
  asset(path: string): string;
  notify(message: unknown): void;
  refreshLayerClock?: () => void;
}

export function setupAudio(app: AudioApp, AudioClass: AudioConstructor = Audio) {
  const channels = app.voices;
  let voicePath: string | null = null;
  function release(audio: AudioHandle): void {
    audio.pause();
    audio.removeAttribute('src');
    audio.load();
  }
  function stop(channel: string, fadeOut = 0): void {
    const audio = channels.get(channel);
    if (!audio) return;
    channels.delete(channel);
    audio.onended = audio.onerror = null;
    if (fadeOut > 0 && !audio.paused) {
      const initial = audio.volume,
        started = performance.now();
      const fade = () => {
        const progress = Math.min(1, (performance.now() - started) / (fadeOut * 1000));
        audio.volume = initial * (1 - progress);
        if (progress < 1) setTimeout(fade, 25);
        else release(audio);
      };
      fade();
    } else release(audio);
    app.refreshLayerClock?.();
  }
  function reset(): void {
    for (const channel of [...channels.keys()]) stop(channel);
    voicePath = null;
  }
  async function play(path: string, channel: string, repeat = false, volume = 1): Promise<void> {
    stop(channel);
    if (channel === 'voice') voicePath = path;
    const audio = new AudioClass(app.asset(path));
    audio.loop = repeat;
    audio.dataset.relativeVolume = String(volume);
    audio.volume = audioVolume(app.settings, channel, volume);
    channels.set(channel, audio);
    audio.onended = () => {
      if (channels.get(channel) !== audio) return;
      stop(channel);
      if (channel === 'music') {
        app.engine.music_ended();
        app.state = parseRuntimeState(app.engine.state(), app.state);
        events().catch(app.notify);
      } else if (channel === 'sound') {
        app.engine.sound_ended();
        app.state = parseRuntimeState(app.engine.state(), app.state);
        events().catch(app.notify);
      }
    };
    audio.onerror = () => {
      if (channels.get(channel) === audio) {
        stop(channel);
        if (channel === 'sound') {
          app.engine.sound_ended();
          app.state = parseRuntimeState(app.engine.state(), app.state);
          events().catch(app.notify);
        }
      }
      app.notify(`Could not play ${path}`);
    };
    try {
      await audio.play();
      app.refreshLayerClock?.();
    } catch (error) {
      if (channels.get(channel) !== audio) return;
      if (error instanceof DOMException && error.name === 'NotAllowedError')
        audio.dataset.pendingGesture = 'true';
      else {
        stop(channel);
        throw error;
      }
    }
  }
  async function events(): Promise<void> {
    for (const event of parseAudioEvents(app.engine.audio_events())) {
      if ('PlaySound' in event)
        await play(event.PlaySound.path, 'sound', event.PlaySound.repeat, event.PlaySound.volume);
      if (typeof event === 'object' && 'PlayVoice' in event)
        await play(event.PlayVoice.path, 'voice');
      if ('StopVoice' in event) {
        stop('voice', event.StopVoice.fade_out);
        voicePath = null;
      }
      if ('StopMusic' in event) stop('music', event.StopMusic.fade_out);
      if ('StopSound' in event) stop('sound', event.StopSound.fade_out);
    }
    const music = app.state.stage.music;
    const currentMusic = channels.get('music'),
      musicVolume = music?.volume ?? 1;
    if (
      music &&
      (currentMusic?.src !== app.asset(music.path) ||
        currentMusic.loop !== music.repeat ||
        Number(currentMusic.dataset.relativeVolume) !== musicVolume)
    ) {
      await play(music.path, 'music', music.repeat, musicVolume);
    }
    if (!music) stop('music');
    const sound = app.state.stage.sound;
    const currentSound = channels.get('sound'),
      soundVolume = sound?.volume ?? 1;
    if (
      sound &&
      (currentSound?.src !== app.asset(sound.path) ||
        currentSound.loop !== sound.repeat ||
        Number(currentSound.dataset.relativeVolume) !== soundVolume)
    ) {
      await play(sound.path, 'sound', sound.repeat, soundVolume);
    }
    if (!sound) stop('sound');
    const voice = app.state.stage.voice;
    if (voice && voice !== voicePath) await play(voice, 'voice');
    if (!voice) {
      stop('voice');
      voicePath = null;
    }
  }
  function resume(): void {
    for (const audio of channels.values()) {
      if (audio.dataset.pendingGesture)
        audio
          .play()
          .then(() => delete audio.dataset.pendingGesture)
          .catch(() => {});
    }
  }
  return { events, reset, resume, replay: (path: string) => play(path, 'voice') };
}

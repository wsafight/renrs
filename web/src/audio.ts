import { audioVolume } from './presentation';
import { parseAudioEvents, parseRuntimeState } from './protocol';
import type {
  AudioConstructor,
  AudioHandle,
  PlayerSettings,
  RuntimeState,
  StageState,
} from './types';

export interface AudioApp {
  voices: Map<string, AudioHandle>;
  settings: PlayerSettings;
  state: { stage: Pick<StageState, 'music' | 'voice'> };
  engine: {
    audio_events(): string;
    music_ended(): void;
    state(): string;
  };
  asset(path: string): string;
  notify(message: unknown): void;
  refreshLayerClock?: () => void;
}

export function setupAudio(app: AudioApp, AudioClass: AudioConstructor = Audio) {
  const channels = app.voices;
  let sequence = 0,
    voicePath: string | null = null;
  function stop(channel: string): void {
    const audio = channels.get(channel);
    if (!audio) return;
    channels.delete(channel);
    audio.onended = audio.onerror = null;
    audio.pause();
    audio.removeAttribute('src');
    audio.load();
    app.refreshLayerClock?.();
  }
  function reset(): void {
    for (const channel of [...channels.keys()]) stop(channel);
    voicePath = null;
  }
  async function play(path: string, channel: string, repeat = false, volume = 1): Promise<void> {
    stop(channel);
    if (channel.startsWith('sound-')) {
      const sounds = [...channels.keys()].filter((key) => key.startsWith('sound-'));
      if (sounds.length >= 8) stop(sounds[0]);
    }
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
        app.state = parseRuntimeState(app.engine.state()) as RuntimeState;
        events().catch(app.notify);
      }
    };
    audio.onerror = () => {
      if (channels.get(channel) === audio) stop(channel);
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
      if (typeof event === 'object' && 'PlaySound' in event)
        await play(event.PlaySound.path, `sound-${++sequence}`, false, event.PlaySound.volume ?? 1);
      if (typeof event === 'object' && 'PlayVoice' in event)
        await play(event.PlayVoice.path, 'voice');
      if (event === 'StopVoice') {
        stop('voice');
        voicePath = null;
      }
      if (typeof event === 'object' && 'StopMusic' in event) stop('music');
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

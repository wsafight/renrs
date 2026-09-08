import type { PlayerApp } from './types';

export async function soundtrack(
  app: PlayerApp,
  path: string,
  elapsed: number,
  relativeVolume = 1,
): Promise<HTMLAudioElement> {
  const audio = new Audio(app.asset(path));
  app.clipAudio = audio;
  app.clipAudioGain = relativeVolume;
  audio.volume = app.settings.sound_volume * relativeVolume;
  audio.preload = 'auto';
  await new Promise<void>((resolve, reject) => {
    audio.onloadeddata = () => resolve();
    audio.onerror = () => reject(new Error(`Soundtrack unavailable: ${path}`));
    audio.load();
  });
  if (elapsed > 0) audio.currentTime = elapsed / 1000;
  audio.onerror = () => app.notify(`Soundtrack unavailable: ${path}`);
  return audio;
}

export function releaseSoundtrack(app: PlayerApp): void {
  const audio = app.clipAudio;
  if (audio) {
    audio.pause();
    audio.onended = audio.onloadeddata = audio.onerror = null;
    audio.removeAttribute('src');
    audio.load();
  }
  app.clipAudio = null;
  app.clipAudioGain = 1;
}

export async function resumeMedia(app: PlayerApp): Promise<void> {
  const audio = app.clipAudio,
    video = app.$('video');
  try {
    if (audio) await audio.play();
    if (app.streamingVideo) await video.play();
  } catch (error) {
    if (error instanceof DOMException && error.name === 'NotAllowedError')
      app.mediaNeedsGesture = true;
    else app.notify(error);
  }
}

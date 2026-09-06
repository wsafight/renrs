export async function soundtrack(app, path, elapsed) {
  const audio = new Audio(app.asset(path));
  app.clipAudio = audio; audio.volume = app.settings.sound_volume;
  audio.preload = 'auto';
  await new Promise((resolve, reject) => {
    audio.onloadeddata = resolve;
    audio.onerror = () => reject(new Error(`Soundtrack unavailable: ${path}`));
    audio.load();
  });
  if (elapsed > 0) audio.currentTime = elapsed / 1000;
  audio.onerror = () => app.notify(`Soundtrack unavailable: ${path}`);
  return audio;
}

export function releaseSoundtrack(app) {
  const audio = app.clipAudio;
  if (audio) { audio.pause(); audio.onended = audio.onloadeddata = audio.onerror = null; audio.removeAttribute('src'); audio.load(); }
  app.clipAudio = null;
}

export async function resumeMedia(app) {
  const audio = app.clipAudio, video = app.$('video');
  try {
    if (audio) await audio.play();
    if (app.streamingVideo) await video.play();
  } catch (error) {
    if (error.name === 'NotAllowedError') app.mediaNeedsGesture = true;
    else app.notify(error.message);
  }
}

const clips = new Map();
import {soundtrack, releaseSoundtrack, resumeMedia} from './video-audio.js';
export async function showVideo(app, effect, elapsed = 0) {
  app.mediaGeneration = (app.mediaGeneration || 0) + 1;
  const generation = app.mediaGeneration;
  const video = app.$('video');
  releaseSoundtrack(app); app.mediaNeedsGesture = false;
  app.streamingVideo = false;
  video.onended = video.onerror = null;
  video.pause(); video.hidden = true; video.removeAttribute('src'); video.load();
  if (!effect) return;
  if (!clips.has(effect.path)) {
    const response = await fetch(app.asset(effect.path));
    if (!response.ok) throw new Error(`Video unavailable: ${effect.path}`);
    const clip = await response.json();
    if (!(clip.fps >= 1 && clip.fps <= 60) || !((clip.version === 1 && clip.frames?.length >= 1 && clip.frames.length <= 7200) || (clip.version === 2 && clip.stream?.path && clip.stream.seconds > 0))) throw new Error('Invalid video manifest');
    clips.set(effect.path, clip);
  }
  const clip = clips.get(effect.path);
  const audio = clip.audio ? await soundtrack(app, clip.audio, elapsed) : null;
  if (clip.stream) {
    app.streamingVideo = true;
    video.controls = false; video.muted = true; video.preload = 'auto';
    const ready = new Promise((resolve, reject) => {
      video.onloadeddata = resolve;
      video.onerror = () => reject(new Error(`Video unavailable: ${clip.stream.path}`));
    });
    video.src = app.asset(clip.stream.path); video.load(); await ready;
    if (generation !== app.mediaGeneration) return;
    if (elapsed > 0) {
      const sought = new Promise(resolve => video.addEventListener('seeked', resolve, {once:true}));
      video.currentTime = Math.min(video.duration, elapsed / 1000); await sought;
    }
    video.hidden = false;
    video.onended = app.guard(() => app.act('next'));
    video.onerror = () => app.notify(`Could not play ${clip.stream.path}`);
    if (!app.$('modal').open) await resumeMedia(app);
    if (audio) {
      const sync = () => {
        if (generation !== app.mediaGeneration) return;
        if (!app.$('modal').open && Math.abs(video.currentTime - audio.currentTime) > .12) video.currentTime = Math.min(video.duration, audio.currentTime);
        requestAnimationFrame(sync);
      };
      requestAnimationFrame(sync);
    }
    return;
  }
  const first = new Image();
  first.src = app.asset(clip.frames[Math.min(clip.frames.length - 1, Math.floor(elapsed * clip.fps / 1000))]);
  await first.decode();
  if (generation !== app.mediaGeneration) return;
  if (audio && !app.$('modal').open) await resumeMedia(app);
  let played=elapsed,previous=performance.now();
  let current = -1;
  function frame() {
    if (generation !== app.mediaGeneration) return;
    const now=performance.now();
    if (audio) played = audio.currentTime * 1000;
    else if(!app.$('modal').open)played+=now-previous;
    previous=now;
    const index = Math.min(clip.frames.length - 1, Math.floor(played * clip.fps / 1000));
    if (index !== current) { app.$('background').src = app.asset(clip.frames[index]); app.$('background').hidden = false; current = index; }
    if (played < effect.seconds * 1000) requestAnimationFrame(frame);
  }
  frame();
}

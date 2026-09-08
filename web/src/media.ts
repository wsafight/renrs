import { parseVideoManifest, type VideoManifest } from './protocol';
import type { PlayerApp, VideoEffect } from './types';
import { releaseSoundtrack, resumeMedia, soundtrack } from './video-audio';

const clips = new Map<string, VideoManifest>();

export async function showVideo(app: PlayerApp, effect?: VideoEffect, elapsed = 0): Promise<void> {
  app.mediaGeneration = (app.mediaGeneration || 0) + 1;
  const generation = app.mediaGeneration;
  const video = app.$('video');
  releaseSoundtrack(app);
  app.mediaNeedsGesture = false;
  app.streamingVideo = false;
  video.onended = video.onerror = null;
  video.pause();
  video.hidden = true;
  video.removeAttribute('src');
  video.load();
  if (!effect) return;
  const videoEffect = effect;
  if (!clips.has(effect.path)) {
    const response = await fetch(app.asset(effect.path));
    if (!response.ok) throw new Error(`Video unavailable: ${effect.path}`);
    const clip = parseVideoManifest(await response.json());
    clips.set(effect.path, clip);
  }
  const clip = clips.get(effect.path);
  if (!clip) throw new Error(`Video manifest missing: ${effect.path}`);
  const audio = clip.audio ? await soundtrack(app, clip.audio, elapsed) : null;
  const stream = clip.stream;
  if (stream) {
    app.streamingVideo = true;
    video.controls = false;
    video.muted = true;
    video.preload = 'auto';
    const ready = new Promise<void>((resolve, reject) => {
      video.onloadeddata = () => resolve();
      video.onerror = () => reject(new Error(`Video unavailable: ${stream.path}`));
    });
    video.src = app.asset(stream.path);
    video.load();
    await ready;
    if (generation !== app.mediaGeneration) return;
    if (elapsed > 0) {
      const sought = new Promise<void>((resolve) =>
        video.addEventListener('seeked', () => resolve(), { once: true }),
      );
      video.currentTime = Math.min(video.duration, elapsed / 1000);
      await sought;
    }
    video.hidden = false;
    video.onended = app.guard(() => app.act('next'));
    video.onerror = () => app.notify(`Could not play ${stream.path}`);
    if (!app.$('modal').open) await resumeMedia(app);
    if (audio) {
      const sync = () => {
        if (generation !== app.mediaGeneration) return;
        if (!app.$('modal').open && Math.abs(video.currentTime - audio.currentTime) > 0.12)
          video.currentTime = Math.min(video.duration, audio.currentTime);
        requestAnimationFrame(sync);
      };
      requestAnimationFrame(sync);
    }
    return;
  }
  const frames = clip.frames;
  const fps = clip.fps;
  const first = new Image();
  first.src = app.asset(frames[Math.min(frames.length - 1, Math.floor((elapsed * fps) / 1000))]);
  await first.decode();
  if (generation !== app.mediaGeneration) return;
  if (audio && !app.$('modal').open) await resumeMedia(app);
  let played = elapsed,
    previous = performance.now();
  let current = -1;
  function frame(): void {
    if (generation !== app.mediaGeneration) return;
    const now = performance.now();
    if (audio) played = audio.currentTime * 1000;
    else if (!app.$('modal').open) played += now - previous;
    previous = now;
    const index = Math.min(frames.length - 1, Math.floor((played * fps) / 1000));
    if (index !== current) {
      app.$('background').src = app.asset(frames[index]);
      app.$('background').hidden = false;
      current = index;
    }
    if (played < videoEffect.seconds * 1000) requestAnimationFrame(frame);
  }
  frame();
}

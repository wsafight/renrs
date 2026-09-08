import {
  parseVideoManifest,
  type SubtitleTrack,
  type VideoAudioTrack,
  type VideoManifest,
} from './protocol';
import type { PlayerApp, VideoEffect } from './types';
import { releaseSoundtrack, resumeMedia, soundtrack } from './video-audio';

const clips = new Map<string, VideoManifest>();

function localizedTrack<T extends { language?: string | null; default: boolean }>(
  tracks: T[],
  language: string,
): T | undefined {
  if (language) {
    const exact = tracks.find((track) => track.language?.toLowerCase() === language.toLowerCase());
    if (exact) return exact;
    const base = language.split('-')[0].toLowerCase();
    const related = tracks.find((track) => {
      const candidate = track.language?.toLowerCase();
      return candidate === base || candidate?.startsWith(`${base}-`);
    });
    if (related) return related;
  }
  return (
    tracks.find((track) => track.default) ?? tracks.find((track) => !track.language) ?? tracks[0]
  );
}

export function selectVideoAudio(
  clip: VideoManifest,
  language: string,
): VideoAudioTrack | undefined {
  if (clip.audio) return { path: clip.audio, default: true, volume: 1 };
  return localizedTrack(clip.audio_tracks, language);
}

export function selectVideoSubtitle(
  clip: VideoManifest,
  language: string,
): SubtitleTrack | undefined {
  return localizedTrack(clip.subtitles, language);
}

function showCaption(app: PlayerApp, track: SubtitleTrack | undefined, seconds: number): void {
  const caption = app.$('video-caption');
  const cue = track?.cues.find((cue) => seconds >= cue.start && seconds < cue.end);
  caption.textContent = cue?.text ?? '';
  caption.hidden = !cue;
}

export async function showVideo(app: PlayerApp, effect?: VideoEffect, elapsed = 0): Promise<void> {
  app.mediaGeneration = (app.mediaGeneration || 0) + 1;
  const generation = app.mediaGeneration;
  const video = app.$('video');
  const caption = app.$('video-caption');
  releaseSoundtrack(app);
  app.mediaNeedsGesture = false;
  app.streamingVideo = false;
  video.onended = video.onerror = null;
  video.pause();
  video.hidden = true;
  caption.hidden = true;
  caption.textContent = '';
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
  const audioTrack = selectVideoAudio(clip, app.settings.language);
  const subtitleTrack = selectVideoSubtitle(clip, app.settings.language);
  const audio = audioTrack
    ? await soundtrack(app, audioTrack.path, elapsed, audioTrack.volume)
    : null;
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
    const sync = () => {
      if (generation !== app.mediaGeneration) return;
      const clock = audio?.currentTime ?? video.currentTime;
      showCaption(app, subtitleTrack, clock);
      if (audio && !app.$('modal').open && Math.abs(video.currentTime - clock) > 0.12)
        video.currentTime = Math.min(video.duration, clock);
      requestAnimationFrame(sync);
    };
    requestAnimationFrame(sync);
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
    showCaption(app, subtitleTrack, played / 1000);
    if (played < videoEffect.seconds * 1000) requestAnimationFrame(frame);
  }
  frame();
}

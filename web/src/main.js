import './style.css';
import { createIcons, RotateCcw, Undo2, Save, History, Images, Settings, Bug, ChevronRight, Play, StepForward, Download, X } from 'lucide';
import { panels } from './panels.js';
import { setupAudio } from './audio.js';
import { writeSave } from './storage.js';
import { showVideo } from './media.js';
import { setupPresentation, audioVolume } from './presentation.js';
import { renderStage, resizeStage } from './stage.js';
import { customScreen, storyScreens } from './screens.js';
import {resumeMedia} from './video-audio.js';
import {setupPlatform} from './platform.js';
import {ReadStore} from './read-store.js';
import {setupReading} from './reading.js';

const $ = id => document.getElementById(id);
export const element = (tag, text, attributes = {}) => { const node = document.createElement(tag); if (text != null) node.textContent = text; Object.assign(node, attributes); return node; };
const profileKey = project => `renrs:${project}:profile`;
const settingsKey = project => `renrs:${project}:settings`;
let noticeTimer;
export const app = {
  $, element, state: null, engine: null, data: null, timer: null, deadline: 0, voices: new Map(), route: [],
  settings: {fontSize: 20, music_volume: .6, sound_volume: .8, voice_volume: 1, auto: false, skip: false, text_speed: 40, auto_delay: 2.5, wait_voice: true, contrast: false, reduced: false, language: ''},
  notify(message) { $('notice').textContent = String(message); $('notice').hidden = false; clearTimeout(noticeTimer); noticeTimer = setTimeout(() => $('notice').hidden = true, 6000); },
  guard(action) { return async (...args) => { try { return await action(...args); } catch (error) { app.notify(error.message || error); } }; },
  asset(path) { return new URL(`game/${path.split('/').map(encodeURIComponent).join('/')}`, location.href).href; },
  applySettings() {
    document.documentElement.style.setProperty('--text-size', `${app.settings.fontSize}px`);
    document.body.classList.toggle('contrast', app.settings.contrast); document.body.classList.toggle('reduced', app.settings.reduced);
    for (const [channel, audio] of app.voices) audio.volume = audioVolume(app.settings, channel, Number(audio.dataset.relativeVolume ?? 1));
    if (app.clipAudio) app.clipAudio.volume = app.settings.sound_volume;
    if (!app.settings.self_voicing && 'speechSynthesis' in window) speechSynthesis.cancel();
    localStorage.setItem(settingsKey(app.data.program.project_id), JSON.stringify(app.settings));
  },
  async setLanguage(language) {
    app.state = JSON.parse(app.engine.language(language)); app.settings.language = language;
    app.applySettings(); app.localize(document); await app.render();
  },
  async openPanel(kind) {
    const customKind = kind === 'saves' ? 'save' : kind;
    if (app.data.screens?.layouts?.[customKind]) {
      const body = panels.open(app, app.tr(customKind === 'save' ? 'Save' : customKind === 'load' ? 'Load' : customKind[0].toUpperCase() + customKind.slice(1)));
      await customScreen(app, customKind, body);
    } else await panels[{save: 'saves', load: 'saves'}[kind] || kind]?.(app);
  },
  async act(command, index = 0) {
    if (command === 'next' && app.state?.waiting === 'Dialogue' && app.reading.finish()) return;
    clearTimeout(app.timer);
    app.state = JSON.parse(app.engine.action(command, index));
    app.reading.reset(); app.pausedRemaining = null;
    app.trackRead();
    if (command === 'choose') app.route.push(index);
    await render(); await audioEvents();
    app.syncProfile();
    schedule();
  },
  trackRead() {
    const id = app.state.stage.dialogue?.statement_id;
    app.wasRead = !!id && app.read.has(id);
    if (id && app.state.waiting === 'Dialogue') {
      app.read.add(id);
    }
  },
  remaining() {
    const video = app.state.waiting?.Effect?.effect?.Video;
    if (video && (app.clipAudio || app.streamingVideo)) return Math.max(0, (video.seconds - (app.clipAudio || $('video')).currentTime) * 1000);
    return app.pausedRemaining ?? Math.max(0, app.deadline - performance.now());
  },
  async save(slot, note = '') {
    app.read.flush();
    const save = {version: 1, id: `${app.data.program.project_id}:${slot}`, project: app.data.program.project_id, slot, time: Date.now(), note, chapter: app.state.debug.label,
      text: app.state.stage.dialogue?.text || '', thumbnail: app.state.stage.background || '', snapshot: app.engine.snapshot(), remaining: app.remaining(),
      presentation: {dialogue_page: 0, visible_characters: app.reading.clock.visible, sprite_elapsed_ms: Math.round(app.layerTime || 0)}};
    await writeSave(save); return save;
  },
  async restore(save) {
    const next = JSON.parse(app.engine.restore(save.snapshot));
    app.layerTime = save.presentation?.sprite_elapsed_ms || 0;
    app.state = next; app.route = []; clearTimeout(app.timer); app.pausedRemaining=save.remaining;
    app.reading.reset(save.presentation?.visible_characters ?? 0);
    app.trackRead();
    const effect=next.waiting?.Effect?.effect; const duration=effect && Object.values(effect)[0]?.seconds;
    await render(duration == null ? 0 : Math.max(0,duration * 1000 - save.remaining));
    audio.reset(); app.profileRevision = null; app.syncProfile();
    await audioEvents(); schedule(save.remaining);
  },
  newGame() {
    app.layerTime = 0;
    audio.reset(); app.profileRevision = null;
    app.engine?.free(); app.engine = new app.Engine(app.data.program_json || JSON.stringify(app.data.program), localStorage.getItem(profileKey(app.data.program.project_id)) || '');
    for (const catalog of app.data.catalogs) app.engine.catalog(JSON.stringify(catalog));
    app.engine.language(app.settings.language); app.route = []; return app.act('start');
  },
  syncProfile() {
    if (app.profileRevision === app.state.profile_revision) return;
    const profile = app.engine.profile(); app.profile = JSON.parse(profile);
    app.profileRevision = app.state.profile_revision;
    localStorage.setItem(profileKey(app.data.program.project_id), profile);
  }
};
async function render(elapsed = 0) {
  const {stage, waiting, debug} = app.state;
  await renderStage(app, stage, elapsed);
  $('speaker').textContent = stage.nvl ? '' : stage.dialogue?.speaker_name || '';
  $('speaker').style.color = stage.dialogue?.speaker_color || '';
  $('game').classList.toggle('nvl', !!stage.nvl);
  const dialogue = app.state.nvl || stage.dialogue || {text: waiting === 'Finished' ? app.tr('The End') : ''};
  app.reading.prepare(dialogue); app.reading.bind($('text'), dialogue);
  $('chapter').textContent = `${debug.label || ''}${waiting === 'Finished' ? ' / The End' : ''}`;
  $('rollback').disabled = !app.state.can_rollback;
  $('next').disabled = waiting === 'Finished' || !!waiting?.Choice || debug.paused;
  $('choices').replaceChildren();
  for (const [index, text] of (waiting?.Choice?.options || []).entries()) {
    const button = element('button', text); button.onclick = app.guard(() => app.act('choose', index)); $('choices').append(button);
  }
  app.updateDebug?.(); await storyScreens(app);
  app.speak(stage.dialogue?.text);
  if ($('modal').open) for(const animation of $('stage').getAnimations({subtree:true}))animation.pause();
  await showVideo(app, waiting?.Effect?.effect?.Video, elapsed);
  app.reading.resume();
}
app.render = render;
function schedule(remaining) {
  clearTimeout(app.timer); app.deadline = 0;
  if (app.streamingVideo || app.inTitle || document.hidden) return;
  const wait = app.state.waiting;
  const effect = wait?.Effect?.effect;
  const seconds = wait?.Pause?.seconds ?? (effect && Object.values(effect)[0]?.seconds);
  let delay = seconds == null ? null : Math.max(0, remaining ?? seconds * 1000);
  if (app.clipAudio) { app.clipAudio.onended = app.guard(() => app.act('next')); return; }
  if (wait === 'Dialogue') {
    if (app.settings.skip && app.wasRead) {
      if (app.reading.finish()) return;
      delay = 40;
    } else if (app.settings.auto && app.reading.clock.complete) delay = remaining > 0 ? remaining : app.settings.auto_delay * 1000;
  }
  if (delay != null && !app.state.debug.paused && !$('modal').open) {
    app.deadline = performance.now() + delay;
    app.timer = setTimeout(app.guard(async () => {
      const voice = app.voices.get('voice');
      if (app.settings.auto && app.settings.wait_voice && voice && !voice.paused && !voice.ended) { schedule(250); return; }
      await app.act('next');
    }), delay);
  }
}
const audio = setupAudio(app);
app.reschedule = schedule;
const audioEvents = audio.events;
app.replay = audio.replay;
document.addEventListener('pointerdown', audio.resume);
document.addEventListener('keydown', audio.resume);
const resumeVideo = () => { if (app.mediaNeedsGesture && !$('modal').open) { app.mediaNeedsGesture = false; resumeMedia(app); } };
document.addEventListener('pointerdown', resumeVideo); document.addEventListener('keydown', resumeVideo);
app.closeModal = () => { $('modal').close(); app.reading.resume(); schedule(app.pausedRemaining); app.pausedRemaining=null; resumeMedia(app); for(const animation of $('stage').getAnimations({subtree:true}))animation.play(); };
$('close').onclick = app.closeModal;
$('modal').addEventListener('cancel', event => { event.preventDefault(); app.closeModal(); });
for (const name of ['saves','history','collection','settings']) $(name).onclick = app.guard(() => app.openPanel(name));
$('next').onclick = app.guard(() => app.act('next'));
$('rollback').onclick = app.guard(() => app.act('rollback'));
$('restart').onclick = app.guard(async () => { if (confirm('Start a new game?')) { await app.save('resume'); await app.newGame(); } });
document.addEventListener('keydown', event => { if (event.code === 'Space' && !$('modal').open && !['INPUT','BUTTON','SELECT'].includes(event.target.tagName)) { event.preventDefault(); $('next').click(); } });
document.addEventListener('visibilitychange', () => {
  if (!app.engine) return;
  if (document.hidden) {
    app.reading.pause(); app.pausedRemaining = app.remaining(); clearTimeout(app.timer);
    app.clipAudio?.pause(); if (app.streamingVideo) $('video').pause();
    for (const animation of $('stage').getAnimations({subtree: true})) animation.pause();
    app.save('resume').catch(app.notify);
  } else if (!$('modal').open) {
    app.reading.resume(); schedule(app.pausedRemaining); app.pausedRemaining = null; resumeMedia(app);
    for (const animation of $('stage').getAnimations({subtree: true})) animation.play();
  }
});
window.addEventListener('pagehide', () => app.read?.flush());
setInterval(() => { if (app.engine && !app.state.debug.paused) app.save('auto').catch(app.notify); }, 30000);

try {
  createIcons({icons:{RotateCcw,Undo2,Save,History,Images,Settings,Bug,ChevronRight,Play,StepForward,Download,X}});
  const response = await fetch('./project.json'); if (!response.ok) throw new Error('Project data could not be loaded');
  app.data = await response.json(); document.title = app.data.program.title; $('title').textContent = document.title;
  app.read = new ReadStore(localStorage, `renrs:${app.data.program.project_id}:read`, app.notify);
  setupReading(app);
  const module = await import(/* @vite-ignore */ new URL('engine/renrs_web.js', location.href).href); await module.default(); app.Engine = module.Engine;
  const theme = app.data.theme || {};
  for (const key of ['music_volume', 'sound_volume', 'voice_volume']) app.settings[key] = theme[key] ?? app.settings[key];
  app.settings.contrast = theme.high_contrast || false; app.settings.reduced = theme.reduced_motion || false;
  const savedSettings = JSON.parse(localStorage.getItem(settingsKey(app.data.program.project_id)) || '{}');
  if (typeof savedSettings.volume === 'number') for (const key of ['music_volume', 'sound_volume', 'voice_volume']) savedSettings[key] ??= savedSettings.volume;
  Object.assign(app.settings, savedSettings); setupPresentation(app); resizeStage(app); app.applySettings();
  $('debug-toggle').onclick = app.guard(async () => {
    const {setupDebugger} = await import('./debug.js');
    setupDebugger(app); $('debug-toggle').click();
  });
  app.inTitle = !!app.data.screens?.layouts?.main_menu;
  await app.newGame(); app.localize(document);
  await setupPlatform(app);
} catch (error) { app.notify(error.message || error); $('text').textContent = 'Unable to start this project.'; }

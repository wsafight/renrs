import './style.css';
import {
  Bug,
  ChevronRight,
  createIcons,
  Download,
  History,
  Images,
  Play,
  RotateCcw,
  Save,
  Settings,
  StepForward,
  Undo2,
  X,
} from 'lucide';
import { setupAudio } from './audio';
import { showVideo } from './media';
import { panels } from './panels';
import { setupPlatform } from './platform';
import { audioVolume, setupPresentation, storyAnnouncement } from './presentation';
import { parseProfile, parseProjectData, parseRuntimeState, parseSettings } from './protocol';
import { ReadStore } from './read-store';
import { setupReading } from './reading';
import { customScreen, storyScreens } from './screens';
import { renderStage, resizeStage } from './stage';
import { writeSave } from './storage';
import type {
  AppElement,
  EffectMap,
  ElementFactory,
  EngineConstructor,
  PlayerApp,
  ProjectData,
  ReadingController,
  RuntimeState,
  SaveRecord,
} from './types';
import { errorMessage, waitingObject } from './types';
import { resumeMedia } from './video-audio';

type Engine = PlayerApp['engine'];
type WasmModule = {
  default(): Promise<unknown>;
  Engine: EngineConstructor;
};

function required<T>(value: T | undefined, name: string): T {
  if (value === undefined) throw new Error(`${name} is not initialized`);
  return value;
}

const $ = (id: string): AppElement => {
  const node = document.getElementById(id);
  if (!node) throw new Error(`Missing required element #${id}`);
  return node as AppElement;
};

export const element: ElementFactory = (tag, text, attributes = {}) => {
  const node = document.createElement(tag);
  if (text != null) node.textContent = text;
  Object.assign(node, attributes);
  return node;
};

const profileKey = (project: string): string => `renrs:${project}:profile`;
const settingsKey = (project: string): string => `renrs:${project}:settings`;
let noticeTimer: ReturnType<typeof setTimeout> | undefined;
let stateValue: RuntimeState | undefined;
let engineValue: Engine | undefined;
let engineConstructorValue: EngineConstructor | undefined;
let projectValue: ProjectData | undefined;
let readValue: ReadStore | undefined;
let readingValue: ReadingController | undefined;

function clearAppTimer(): void {
  if (app.timer !== null) clearTimeout(app.timer);
  app.timer = null;
}

function effectSeconds(effect?: EffectMap): number | undefined {
  return Object.values(effect ?? {}).find((value) => value !== undefined)?.seconds;
}

function asWasmModule(value: unknown): WasmModule {
  if (typeof value !== 'object' || value === null) throw new Error('Invalid WebAssembly module');
  const module = value as Record<string, unknown>;
  if (typeof module.default !== 'function' || typeof module.Engine !== 'function') {
    throw new Error('WebAssembly module is missing its initializer or Engine');
  }
  return value as WasmModule;
}

export const app: PlayerApp = {
  $,
  element,
  get state() {
    return required(stateValue, 'Runtime state');
  },
  set state(value) {
    stateValue = value;
  },
  get engine() {
    return required(engineValue, 'Engine');
  },
  set engine(value) {
    engineValue = value;
  },
  get Engine() {
    return required(engineConstructorValue, 'Engine constructor');
  },
  set Engine(value) {
    engineConstructorValue = value;
  },
  get data() {
    return required(projectValue, 'Project data');
  },
  set data(value) {
    projectValue = value;
  },
  get read() {
    return required(readValue, 'Read history');
  },
  set read(value) {
    readValue = value;
  },
  get reading() {
    return required(readingValue, 'Reading controller');
  },
  set reading(value) {
    readingValue = value;
  },
  timer: null,
  deadline: 0,
  voices: new Map(),
  route: [],
  settings: {
    fontSize: 20,
    music_volume: 0.6,
    sound_volume: 0.8,
    voice_volume: 1,
    auto: false,
    skip: false,
    text_speed: 40,
    auto_delay: 2.5,
    wait_voice: true,
    self_voicing: false,
    contrast: false,
    reduced: false,
    language: '',
  },
  profile: { achievements: [], gallery: [], endings: [] },
  profileRevision: null,
  clipAudio: null,
  clipAudioGain: 1,
  streamingVideo: false,
  mediaNeedsGesture: false,
  mediaGeneration: 0,
  pausedRemaining: null,
  layerTime: 0,
  layerNodes: [],
  wasRead: false,
  inTitle: false,
  notify(message: unknown): void {
    $('notice').textContent = String(message);
    $('notice').hidden = false;
    if (noticeTimer !== undefined) clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => {
      $('notice').hidden = true;
    }, 6000);
  },
  guard<Args extends unknown[], Result>(action: (...args: Args) => Result | Promise<Result>) {
    return async (...args: Args): Promise<Result | undefined> => {
      try {
        return await action(...args);
      } catch (error) {
        app.notify(errorMessage(error));
        return undefined;
      }
    };
  },
  asset(path: string): string {
    return new URL(`game/${path.split('/').map(encodeURIComponent).join('/')}`, location.href).href;
  },
  applySettings(): void {
    document.documentElement.style.setProperty('--text-size', `${app.settings.fontSize}px`);
    document.body.classList.toggle('contrast', app.settings.contrast);
    document.body.classList.toggle('reduced', app.settings.reduced);
    for (const [channel, audio] of app.voices) {
      audio.volume = audioVolume(app.settings, channel, Number(audio.dataset.relativeVolume ?? 1));
    }
    if (app.clipAudio) app.clipAudio.volume = app.settings.sound_volume * app.clipAudioGain;
    if (!app.settings.self_voicing && 'speechSynthesis' in window) speechSynthesis.cancel();
    localStorage.setItem(settingsKey(app.data.program.project_id), JSON.stringify(app.settings));
  },
  async setLanguage(language: string): Promise<void> {
    app.state = parseRuntimeState(app.engine.language(language));
    app.settings.language = language;
    app.applySettings();
    app.localize(document);
    await app.render();
  },
  async openPanel(kind: string): Promise<void> {
    const customKind = kind === 'saves' ? 'save' : kind;
    if (app.data.screens?.layouts?.[customKind]) {
      const title =
        customKind === 'save'
          ? 'Save'
          : customKind === 'load'
            ? 'Load'
            : customKind[0].toUpperCase() + customKind.slice(1);
      const body = panels.open(app, app.tr(title));
      await customScreen(app, customKind, body);
      return;
    }
    const panelKind = kind === 'save' || kind === 'load' ? 'saves' : kind;
    if (panelKind === 'saves') await panels.saves(app);
    else if (panelKind === 'history') panels.history(app);
    else if (panelKind === 'collection') panels.collection(app);
    else if (panelKind === 'settings') panels.settings(app);
  },
  async act(command: string, index = 0): Promise<void> {
    if (command === 'next' && app.state.waiting === 'Dialogue' && app.reading.finish()) return;
    clearAppTimer();
    app.state = parseRuntimeState(app.engine.action(command, index));
    app.reading.reset();
    app.pausedRemaining = null;
    app.trackRead();
    if (command === 'choose') app.route.push(index);
    await render();
    await audioEvents();
    app.syncProfile();
    schedule();
  },
  trackRead(): void {
    const id = app.state.stage.dialogue?.statement_id;
    app.wasRead = Boolean(id && app.read.has(id));
    if (id && app.state.waiting === 'Dialogue') app.read.add(id);
  },
  remaining(): number {
    const video = waitingObject(app.state.waiting).Effect?.effect?.Video;
    if (video && (app.clipAudio || app.streamingVideo)) {
      return Math.max(0, (video.seconds - (app.clipAudio || $('video')).currentTime) * 1000);
    }
    return app.pausedRemaining ?? Math.max(0, app.deadline - performance.now());
  },
  async save(slot: string, note = '') {
    app.read.flush();
    return writeSave({
      version: 1,
      id: `${app.data.program.project_id}:${slot}`,
      project: app.data.program.project_id,
      slot,
      time: Date.now(),
      note,
      chapter: app.state.debug.label ?? '',
      text: app.state.stage.dialogue?.text || '',
      thumbnail: app.state.stage.background || '',
      snapshot: app.engine.snapshot(),
      remaining: app.remaining(),
      presentation: {
        dialogue_page: 0,
        visible_characters: app.reading.clock.visible,
        sprite_elapsed_ms: Math.round(app.layerTime || 0),
      },
    });
  },
  async restore(save: SaveRecord): Promise<void> {
    const next = parseRuntimeState(app.engine.restore(save.snapshot));
    app.layerTime = save.presentation?.sprite_elapsed_ms || 0;
    app.state = next;
    app.route = [];
    clearAppTimer();
    app.pausedRemaining = save.remaining;
    app.reading.reset(save.presentation?.visible_characters ?? 0);
    app.trackRead();
    const duration = effectSeconds(waitingObject(next.waiting).Effect?.effect);
    await render(duration == null ? 0 : Math.max(0, duration * 1000 - save.remaining));
    audio.reset();
    app.profileRevision = null;
    app.syncProfile();
    await audioEvents();
    schedule(save.remaining);
  },
  async newGame(): Promise<void> {
    app.layerTime = 0;
    audio.reset();
    app.profileRevision = null;
    engineValue?.free();
    app.engine = new app.Engine(
      app.data.program_json || JSON.stringify(app.data.program),
      localStorage.getItem(profileKey(app.data.program.project_id)) || '',
    );
    for (const catalog of app.data.catalogs) app.engine.catalog(JSON.stringify(catalog));
    app.engine.language(app.settings.language);
    app.route = [];
    await app.act('start');
  },
  syncProfile(): void {
    if (app.profileRevision === app.state.profile_revision) return;
    const profile = app.engine.profile();
    app.profile = parseProfile(profile);
    app.profileRevision = app.state.profile_revision;
    localStorage.setItem(profileKey(app.data.program.project_id), profile);
  },
  render: async () => {},
  reschedule: () => {},
  replay: async () => {},
  closeModal: () => {},
  tr: (text: string) => text,
  localize: () => {},
  speak: () => {},
};

async function render(elapsed = 0): Promise<void> {
  const { stage, waiting, debug } = app.state;
  const wait = waitingObject(waiting);
  await renderStage(app, stage, elapsed);
  $('speaker').textContent = stage.nvl ? '' : stage.dialogue?.speaker_name || '';
  $('speaker').style.color = stage.dialogue?.speaker_color || '';
  $('game').classList.toggle('nvl', Boolean(stage.nvl));
  const dialogue = app.state.nvl ||
    stage.dialogue || { text: waiting === 'Finished' ? app.tr('The End') : '' };
  app.reading.prepare(dialogue);
  app.reading.bind($('text'), dialogue);
  $('chapter').textContent = `${debug.label || ''}${waiting === 'Finished' ? ' / The End' : ''}`;
  $('rollback').disabled = !app.state.can_rollback;
  $('next').disabled = waiting === 'Finished' || Boolean(wait.Choice) || debug.paused;
  $('choices').replaceChildren();
  for (const [index, text] of (wait.Choice?.options || []).entries()) {
    const choice = element('button', text);
    choice.onclick = app.guard(() => app.act('choose', index));
    $('choices').append(choice);
  }
  app.updateDebug?.();
  await storyScreens(app);
  const announcement = storyAnnouncement(stage.dialogue);
  const status = $('story-status');
  if (status.textContent !== announcement) status.textContent = announcement;
  app.speak(stage.dialogue?.text);
  if ($('modal').open) {
    for (const animation of $('stage').getAnimations({ subtree: true })) animation.pause();
  }
  await showVideo(app, wait.Effect?.effect?.Video, elapsed);
  app.reading.resume();
}

app.render = render;

function schedule(remaining?: number): void {
  clearAppTimer();
  app.deadline = 0;
  if (app.streamingVideo || app.inTitle || document.hidden) return;
  const waiting = app.state.waiting;
  const wait = waitingObject(waiting);
  const seconds = wait.Pause?.seconds ?? effectSeconds(wait.Effect?.effect);
  let delay = seconds == null ? null : Math.max(0, remaining ?? seconds * 1000);
  if (app.clipAudio) {
    app.clipAudio.onended = app.guard(() => app.act('next'));
    return;
  }
  if (waiting === 'Dialogue') {
    if (app.settings.skip && app.wasRead) {
      if (app.reading.finish()) return;
      delay = 40;
    } else if (
      (app.settings.auto || app.state.stage.dialogue?.no_wait) &&
      app.reading.clock.complete &&
      !app.reading.blocked
    ) {
      delay = (remaining ?? 0) > 0 ? (remaining ?? 0) : app.settings.auto_delay * 1000;
    }
  }
  if (delay != null && !app.state.debug.paused && !$('modal').open) {
    app.deadline = performance.now() + delay;
    app.timer = setTimeout(
      app.guard(async () => {
        const voice = app.voices.get('voice');
        if (
          app.settings.auto &&
          app.settings.wait_voice &&
          voice &&
          !voice.paused &&
          !voice.ended
        ) {
          schedule(250);
          return;
        }
        await app.act('next');
      }),
      delay,
    );
  }
}

const audio = setupAudio(app);
app.reschedule = schedule;
const audioEvents = audio.events;
app.replay = audio.replay;
document.addEventListener('pointerdown', audio.resume);
document.addEventListener('keydown', audio.resume);

const resumeVideo = (): void => {
  if (app.mediaNeedsGesture && !$('modal').open) {
    app.mediaNeedsGesture = false;
    void resumeMedia(app);
  }
};

document.addEventListener('pointerdown', resumeVideo);
document.addEventListener('keydown', resumeVideo);
app.closeModal = () => {
  $('modal').close();
  app.reading.resume();
  schedule(app.pausedRemaining ?? undefined);
  app.pausedRemaining = null;
  void resumeMedia(app);
  for (const animation of $('stage').getAnimations({ subtree: true })) animation.play();
};
$('close').onclick = app.closeModal;
$('modal').addEventListener('cancel', (event) => {
  event.preventDefault();
  app.closeModal();
});
for (const name of ['saves', 'history', 'collection', 'settings']) {
  $(name).onclick = app.guard(() => app.openPanel(name));
}
$('next').onclick = app.guard(() => app.act('next'));
$('rollback').onclick = app.guard(() => app.act('rollback'));
$('restart').onclick = app.guard(async () => {
  if (confirm('Start a new game?')) {
    await app.save('resume');
    await app.newGame();
  }
});
document.addEventListener('keydown', (event) => {
  const target = event.target;
  if (
    event.code === 'Space' &&
    !$('modal').open &&
    target instanceof HTMLElement &&
    !['INPUT', 'BUTTON', 'SELECT'].includes(target.tagName)
  ) {
    event.preventDefault();
    $('next').click();
  }
});
document.addEventListener('visibilitychange', () => {
  if (!engineValue || !stateValue || !readingValue) return;
  if (document.hidden) {
    app.reading.pause();
    app.pausedRemaining = app.remaining();
    clearAppTimer();
    app.clipAudio?.pause();
    if (app.streamingVideo) $('video').pause();
    for (const animation of $('stage').getAnimations({ subtree: true })) animation.pause();
    app.save('resume').catch(app.notify);
  } else if (!$('modal').open) {
    app.reading.resume();
    schedule(app.pausedRemaining ?? undefined);
    app.pausedRemaining = null;
    void resumeMedia(app);
    for (const animation of $('stage').getAnimations({ subtree: true })) animation.play();
  }
});
window.addEventListener('pagehide', () => readValue?.flush());
setInterval(() => {
  if (engineValue && stateValue && !app.state.debug.paused) app.save('auto').catch(app.notify);
}, 30000);

try {
  createIcons({
    icons: {
      RotateCcw,
      Undo2,
      Save,
      History,
      Images,
      Settings,
      Bug,
      ChevronRight,
      Play,
      StepForward,
      Download,
      X,
    },
  });
  const response = await fetch('./project.json');
  if (!response.ok) throw new Error('Project data could not be loaded');
  app.data = parseProjectData(await response.json());
  document.title = app.data.program.title;
  $('title').textContent = document.title;
  app.read = new ReadStore(localStorage, `renrs:${app.data.program.project_id}:read`, app.notify);
  setupReading(app);
  const loaded: unknown = await import(
    /* @vite-ignore */ new URL('engine/renrs_web.js', location.href).href
  );
  const module = asWasmModule(loaded);
  await module.default();
  app.Engine = module.Engine;
  const theme = app.data.theme;
  for (const key of ['music_volume', 'sound_volume', 'voice_volume'] as const) {
    const value = theme[key];
    if (typeof value === 'number') Object.assign(app.settings, { [key]: value });
  }
  app.settings.contrast = theme.high_contrast || false;
  app.settings.reduced = theme.reduced_motion || false;
  const savedSettings = parseSettings(
    localStorage.getItem(settingsKey(app.data.program.project_id)) || '{}',
  );
  if (typeof savedSettings.volume === 'number') {
    savedSettings.music_volume ??= savedSettings.volume;
    savedSettings.sound_volume ??= savedSettings.volume;
    savedSettings.voice_volume ??= savedSettings.volume;
  }
  Object.assign(app.settings, savedSettings);
  resizeStage(app);
  app.applySettings();
  $('debug-toggle').onclick = app.guard(async () => {
    const { setupDebugger } = await import('./debug');
    setupDebugger(app);
    $('debug-toggle').click();
  });
  app.inTitle = Boolean(app.data.screens?.layouts?.main_menu);
  await app.newGame();
  setupPresentation(app);
  app.localize(document);
  await setupPlatform(app);
} catch (error) {
  app.notify(errorMessage(error));
  $('text').textContent = 'Unable to start this project.';
}

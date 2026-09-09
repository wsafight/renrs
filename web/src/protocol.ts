import type {
  Dialogue,
  PlayerSettings,
  ProfileData,
  ProjectData,
  RuntimeState,
  SaveRecord,
  SpriteState,
  StageState,
} from './types';

export type AudioEvent =
  | { PlayMusic: { path: string; repeat: boolean; fade_in: number; volume: number } }
  | { QueueMusic: { path: string; repeat: boolean; fade_in: number; volume: number } }
  | { PlaySound: { path: string; volume: number; repeat: boolean } }
  | { QueueSound: { path: string; volume: number; repeat: boolean } }
  | { PlayVoice: { path: string } }
  | { StopMusic: { fade_out: number } }
  | { StopSound: { fade_out: number } }
  | { StopVoice: { fade_out: number } };

export interface VideoManifest {
  version: 1 | 2;
  fps: number;
  frames: string[];
  audio?: string | null;
  audio_tracks: VideoAudioTrack[];
  subtitles: SubtitleTrack[];
  stream?: { path: string; seconds: number; width: number; height: number } | null;
}

export interface VideoAudioTrack {
  path: string;
  language?: string | null;
  label?: string | null;
  default: boolean;
  volume: number;
}

export interface SubtitleTrack {
  language: string;
  label?: string | null;
  default: boolean;
  cues: Array<{ start: number; end: number; text: string }>;
}

export interface DebugInspection {
  debug: {
    label?: string | null;
    paused: boolean;
    next_instruction: number;
    variables: Record<string, unknown>;
    call_stack: unknown[];
    location?: { source?: string; line?: number } | null;
  };
  coverage: number[];
}

export type ImportedSave = Omit<SaveRecord, 'checksum' | 'id' | 'slot'>;

type JsonRecord = Record<string, unknown>;

function record(value: unknown, context: string): JsonRecord {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error(`Invalid ${context}: expected an object`);
  }
  return value as JsonRecord;
}

function string(value: unknown, context: string): string {
  if (typeof value !== 'string') throw new Error(`Invalid ${context}: expected a string`);
  return value;
}

function nullableString(value: unknown, context: string): string {
  return value == null ? '' : string(value, context);
}

function number(value: unknown, context: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    throw new Error(`Invalid ${context}: expected a finite number`);
  }
  return value;
}

function array(value: unknown, context: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`Invalid ${context}: expected an array`);
  return value;
}

function optionalBoolean(value: unknown, context: string): boolean {
  if (value === undefined) return false;
  if (typeof value !== 'boolean') throw new Error(`Invalid ${context}: expected a boolean`);
  return value;
}

const byteLength = (value: string): number => new TextEncoder().encode(value).byteLength;

function visiblePath(value: string): boolean {
  const lower = value.toLowerCase();
  const parts = value.split('/');
  return (
    value.length > 0 &&
    !value.includes('\\') &&
    !value.includes(':') &&
    !value.startsWith('/') &&
    parts.every(
      (part) =>
        part.length > 0 &&
        part !== '.' &&
        part !== '..' &&
        !part.startsWith('.') &&
        !['dist', 'target', 'node_modules'].includes(part.toLowerCase()),
    ) &&
    !lower.endsWith('.renrs') &&
    !value.endsWith('~') &&
    !/\.(tmp|swp|swo)$/.test(value)
  );
}

function hasExtension(path: string, extensions: string[]): boolean {
  const extension = path.split('.').pop()?.toLowerCase();
  return visiblePath(path) && extension !== undefined && extensions.includes(extension);
}

function validLanguage(language: string): boolean {
  return language.length > 0 && language.length <= 35 && /^[a-z0-9-]+$/i.test(language);
}

function validLabel(label: string | null): boolean {
  return label === null || byteLength(label) <= 80;
}

function hasInvalidControl(text: string): boolean {
  return [...text].some((character) => character !== '\n' && /\p{Cc}/u.test(character));
}

export function parseJson(text: string, context: string): unknown {
  try {
    return JSON.parse(text) as unknown;
  } catch (error) {
    throw new Error(`Invalid ${context} JSON`, { cause: error });
  }
}

export function parseProjectData(value: unknown): ProjectData {
  const project = record(value, 'project data');
  const program = record(project.program, 'project program');
  string(program.title, 'program title');
  string(program.project_id, 'program project_id');
  string(program.fingerprint, 'program fingerprint');
  record(program.labels, 'program labels');
  array(program.instructions, 'program instructions');
  record(program.progress, 'program progress');
  array(project.catalogs, 'translation catalogs');
  record(project.theme, 'project theme');
  if (project.program_json !== undefined) string(project.program_json, 'serialized program');
  if (project.screens !== undefined) {
    const screens = record(project.screens, 'project screens');
    record(screens.styles, 'screen styles');
    record(screens.layouts, 'screen layouts');
  }
  return project as unknown as ProjectData;
}

function validateStage(value: unknown): StageState {
  const stage = record(value, 'runtime stage');
  array(stage.sprites, 'stage sprites');
  record(stage.camera, 'stage camera');
  if (stage.dialogue != null) validateDialogue(stage.dialogue);
  return stage as unknown as StageState;
}

function validateDialogue(value: unknown): Dialogue {
  const dialogue = record(value, 'dialogue');
  string(dialogue.text, 'dialogue text');
  if (dialogue.runs !== undefined) array(dialogue.runs, 'dialogue runs');
  return dialogue as unknown as Dialogue;
}

export function parseRuntimeState(text: string): RuntimeState {
  const value = record(parseJson(text, 'runtime state'), 'runtime state');
  validateStage(value.stage);
  record(value.debug, 'runtime debug state');
  number(value.profile_revision, 'profile revision');
  number(value.history_count, 'history count');
  if (typeof value.can_rollback !== 'boolean') {
    throw new Error('Invalid runtime state: expected can_rollback boolean');
  }
  if (value.nvl != null) validateDialogue(value.nvl);
  const waiting = value.waiting;
  if (waiting !== null && typeof waiting !== 'string') record(waiting, 'runtime wait state');
  return value as unknown as RuntimeState;
}

export function parseHistory(text: string): Dialogue[] {
  return array(parseJson(text, 'history'), 'history').map(validateDialogue);
}

export function parseProfile(text: string): ProfileData {
  const profile = record(parseJson(text, 'profile'), 'profile');
  for (const category of ['achievements', 'gallery', 'endings']) {
    const values = array(profile[category], `profile ${category}`);
    if (values.some((value) => typeof value !== 'string')) {
      throw new Error(`Invalid profile ${category}: expected string IDs`);
    }
  }
  return profile as unknown as ProfileData;
}

export function parseSettings(text: string): Partial<PlayerSettings> {
  const settings = record(parseJson(text, 'player settings'), 'player settings');
  const numbers = [
    'fontSize',
    'music_volume',
    'sound_volume',
    'voice_volume',
    'text_speed',
    'auto_delay',
    'volume',
  ];
  const booleans = ['auto', 'skip', 'wait_voice', 'self_voicing', 'contrast', 'reduced'];
  for (const key of numbers)
    if (settings[key] !== undefined) number(settings[key], `setting ${key}`);
  for (const key of booleans)
    if (settings[key] !== undefined && typeof settings[key] !== 'boolean') {
      throw new Error(`Invalid setting ${key}: expected a boolean`);
    }
  if (settings.language !== undefined) string(settings.language, 'setting language');
  return settings as Partial<PlayerSettings>;
}

export function parseImportedSave(text: string): ImportedSave {
  const save = record(parseJson(text, 'imported save'), 'imported save');
  const version = number(save.version, 'save version');
  if (version !== 1) throw new Error('Invalid save version');
  const imported: ImportedSave = {
    version,
    project: string(save.project, 'save project'),
    time: number(save.time, 'save time'),
    snapshot: string(save.snapshot, 'save snapshot'),
    remaining: number(save.remaining, 'save remaining'),
    note: nullableString(save.note, 'save note'),
    chapter: nullableString(save.chapter, 'save chapter'),
    text: nullableString(save.text, 'save text'),
    thumbnail: nullableString(save.thumbnail, 'save thumbnail'),
  };
  if (save.play_time_seconds !== undefined) {
    imported.play_time_seconds = number(save.play_time_seconds, 'save play time');
  }
  if (save.presentation !== undefined) {
    const value = record(save.presentation, 'save presentation');
    imported.presentation = {
      dialogue_page: number(value.dialogue_page, 'save dialogue page'),
      visible_characters: number(value.visible_characters, 'save visible characters'),
      sprite_elapsed_ms: number(value.sprite_elapsed_ms, 'save sprite elapsed time'),
    };
  }
  return imported;
}

export function parseAudioEvents(text: string): AudioEvent[] {
  const values = array(parseJson(text, 'audio events'), 'audio events');
  for (const value of values) {
    record(value, 'audio event');
  }
  return values as AudioEvent[];
}

export function parseVideoManifest(value: unknown): VideoManifest {
  const clip = record(value, 'video manifest');
  const version = number(clip.version, 'video version');
  const fps = number(clip.fps, 'video fps');
  const frames = clip.frames === undefined ? [] : array(clip.frames, 'video frames');
  const validFrames =
    frames.length >= 1 &&
    frames.length <= 7200 &&
    frames.every((frame) => typeof frame === 'string' && visiblePath(frame));
  let stream: Exclude<VideoManifest['stream'], null>;
  if (clip.stream != null) {
    const value = record(clip.stream, 'video stream');
    stream = {
      path: string(value.path, 'video stream path'),
      seconds: number(value.seconds, 'video stream duration'),
      width: number(value.width, 'video stream width'),
      height: number(value.height, 'video stream height'),
    };
  }
  const audioTracks = (
    clip.audio_tracks === undefined ? [] : array(clip.audio_tracks, 'video audio tracks')
  ).map((item): VideoAudioTrack => {
    const track = record(item, 'video audio track');
    const volume = track.volume === undefined ? 1 : number(track.volume, 'video audio volume');
    const path = string(track.path, 'video audio path');
    const language = track.language == null ? null : string(track.language, 'video audio language');
    const label = track.label == null ? null : string(track.label, 'video audio label');
    if (
      !hasExtension(path, ['wav']) ||
      (language !== null && !validLanguage(language)) ||
      !validLabel(label) ||
      volume < 0 ||
      volume > 1
    ) {
      throw new Error('Invalid video audio track');
    }
    return {
      path,
      language,
      label,
      default: optionalBoolean(track.default, 'video audio default'),
      volume,
    };
  });
  const subtitles = (
    clip.subtitles === undefined ? [] : array(clip.subtitles, 'video subtitles')
  ).map((item): SubtitleTrack => {
    const track = record(item, 'video subtitle track');
    const language = string(track.language, 'video subtitle language');
    const label = track.label == null ? null : string(track.label, 'video subtitle label');
    const cues = array(track.cues, 'video subtitle cues').map((item) => {
      const cue = record(item, 'video subtitle cue');
      return {
        start: number(cue.start, 'video subtitle start'),
        end: number(cue.end, 'video subtitle end'),
        text: string(cue.text, 'video subtitle text'),
      };
    });
    return {
      language,
      label,
      default: optionalBoolean(track.default, 'video subtitle default'),
      cues,
    };
  });
  const duration = version === 2 ? stream?.seconds : frames.length / fps;
  const validStream =
    stream !== undefined &&
    hasExtension(stream.path, ['mp4', 'webm']) &&
    stream.seconds > 0 &&
    stream.seconds <= 86400 &&
    Number.isInteger(stream.width) &&
    stream.width > 0 &&
    stream.width <= 1920 &&
    Number.isInteger(stream.height) &&
    stream.height > 0 &&
    stream.height <= 1080;
  const validSubtitles = subtitles.every((track) => {
    if (
      !validLanguage(track.language) ||
      !validLabel(track.label ?? null) ||
      track.cues.length > 10_000 ||
      duration === undefined
    ) {
      return false;
    }
    let previousEnd = 0;
    for (const cue of track.cues) {
      if (
        cue.start < previousEnd ||
        cue.end <= cue.start ||
        cue.end > duration + 0.001 ||
        cue.text.length === 0 ||
        byteLength(cue.text) > 2048 ||
        hasInvalidControl(cue.text)
      ) {
        return false;
      }
      previousEnd = cue.end;
    }
    return true;
  });
  if (
    (version !== 1 && version !== 2) ||
    fps < 1 ||
    fps > 60 ||
    (version === 1 && (!validFrames || stream !== undefined)) ||
    (version === 2 && (frames.length > 0 || !validStream)) ||
    (clip.audio != null && audioTracks.length > 0) ||
    audioTracks.length > 16 ||
    audioTracks.filter((track) => track.default).length > 1 ||
    subtitles.length > 16 ||
    subtitles.filter((track) => track.default).length > 1 ||
    !validSubtitles
  ) {
    throw new Error('Invalid video manifest');
  }
  const audio = clip.audio == null ? null : string(clip.audio, 'video soundtrack');
  if (audio !== null && !hasExtension(audio, ['wav'])) {
    throw new Error('Invalid video soundtrack');
  }
  return {
    version,
    fps,
    frames: frames as string[],
    audio,
    audio_tracks: audioTracks,
    subtitles,
    stream,
  };
}

export function parseDebugInspection(text: string): DebugInspection {
  const inspection = record(parseJson(text, 'debug inspection'), 'debug inspection');
  const debug = record(inspection.debug, 'debug inspection state');
  const coverage = array(inspection.coverage, 'debug coverage').map((value) =>
    number(value, 'debug coverage index'),
  );
  return { ...inspection, debug, coverage } as unknown as DebugInspection;
}

export function parseSpriteFrames(text: string): SpriteState[][] {
  return array(parseJson(text, 'animation frames'), 'animation frames') as SpriteState[][];
}

export function parseCameraFrames(text: string): StageState['camera'][] {
  return array(parseJson(text, 'camera frames'), 'camera frames') as StageState['camera'][];
}

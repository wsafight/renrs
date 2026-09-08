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
  | 'StopVoice'
  | { PlaySound: { path: string; volume?: number } }
  | { PlayVoice: { path: string } }
  | { StopMusic: { fade_out: number } };

export interface VideoManifest {
  version: 1 | 2;
  fps: number;
  frames: string[];
  audio?: string | null;
  stream?: { path: string; seconds: number } | null;
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
    if (value === 'StopVoice') continue;
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
    frames.every((frame) => typeof frame === 'string');
  let stream: VideoManifest['stream'];
  if (clip.stream != null) {
    const value = record(clip.stream, 'video stream');
    stream = {
      path: string(value.path, 'video stream path'),
      seconds: number(value.seconds, 'video stream duration'),
    };
  }
  if (
    (version !== 1 && version !== 2) ||
    fps < 1 ||
    fps > 60 ||
    (version === 1 && !validFrames) ||
    (version === 2 && (!stream || stream.seconds <= 0))
  ) {
    throw new Error('Invalid video manifest');
  }
  return {
    version,
    fps,
    frames: frames as string[],
    audio: clip.audio == null ? null : string(clip.audio, 'video soundtrack'),
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

import type { Engine } from '../public/engine/renrs_web';
import type { ReadStore } from './read-store';
import type { RevealClock } from './reveal';

export type JsonPrimitive = boolean | number | string | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };
export type EngineConstructor = new (program: string, profile: string) => Engine;

export type AppElement = HTMLElement & {
  checked: boolean;
  controls: boolean;
  currentTime: number;
  disabled: boolean;
  duration: number;
  files: FileList | null;
  hidden: boolean;
  max: string;
  min: string;
  muted: boolean;
  onended: ((event: Event) => unknown) | null;
  onerror: OnErrorEventHandler;
  onloadeddata: ((event: Event) => unknown) | null;
  open: boolean;
  paused: boolean;
  preload: string;
  src: string;
  step: string;
  value: string;
  close(): void;
  decode(): Promise<void>;
  load(): void;
  pause(): void;
  play(): Promise<void>;
  showModal(): void;
};

export type ElementFactory = <K extends keyof HTMLElementTagNameMap>(
  tag: K,
  text?: string | null,
  attributes?: Partial<HTMLElementTagNameMap[K]>,
) => HTMLElementTagNameMap[K];

export interface TextStyle {
  bold?: boolean;
  color?: string | null;
  underline?: boolean;
  ruby?: string | null;
}

export interface TextRun {
  text: string;
  style: TextStyle;
}

export interface Dialogue {
  statement_id?: string | null;
  speaker_id?: string | null;
  speaker_name?: string | null;
  speaker_color?: string;
  translation_id?: string | null;
  text: string;
  voice_path?: string | null;
  runs?: TextRun[];
}

export interface Rect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface TransformState {
  x: number;
  y: number;
  scale: number;
  rotation: number;
  alpha: number;
  anchor_x: number;
  anchor_y: number;
  crop: Rect | null;
}

export interface ImageFrame {
  path: string;
  seconds: number;
}

export interface ImageLayer {
  path: string;
  x: number;
  y: number;
  frames: ImageFrame[];
  speaking: boolean;
}

export interface LayeredImage {
  width: number;
  height: number;
  layers: ImageLayer[];
}

export interface SpriteState {
  composition?: LayeredImage | null;
  path: string;
  alias: string;
  position: 'Left' | 'Center' | 'Right';
  layer: number;
  display_layer: string;
  display_order: number;
  transform: TransformState;
}

export interface MusicState {
  path: string;
  repeat: boolean;
  fade_in?: number;
  volume: number;
}

export interface StageState {
  camera: TransformState;
  nvl?: boolean;
  nvl_start?: number;
  background?: string | null;
  sprites: SpriteState[];
  music?: MusicState | null;
  music_queue?: MusicState[];
  voice?: string | null;
  dialogue?: Dialogue | null;
}

export interface VideoEffect {
  path: string;
  seconds: number;
}

export interface TimedEffect {
  seconds: number;
}

export interface TransformEffect extends TimedEffect {
  alias: string;
  from: TransformState;
  to: TransformState;
  easing: 'Linear' | 'EaseIn' | 'EaseOut' | 'EaseInOut';
}

export interface TweenEffect extends TimedEffect {
  alias: string;
  from: SpriteState['position'];
  to: SpriteState['position'];
}

export interface DissolveEffect extends TimedEffect {
  from: StageState;
}

export interface ParallelEffect extends TimedEffect {
  from: StageState;
}

export interface EffectMap {
  Dissolve?: DissolveEffect;
  Fade?: TimedEffect;
  Parallel?: ParallelEffect;
  Transform?: TransformEffect;
  Tween?: TweenEffect;
  Video?: VideoEffect;
}

export interface WaitingObject {
  Choice?: { options: string[] };
  Effect?: { effect: EffectMap };
  Pause?: { seconds: number };
}

export type WaitingState = 'Dialogue' | 'Finished' | WaitingObject | null;

export interface RuntimeState {
  stage: StageState;
  waiting: WaitingState;
  debug: { label?: string | null; paused: boolean };
  profile_revision: number;
  history_count: number;
  can_rollback: boolean;
  nvl?: Dialogue | null;
}

export interface InstructionPayload {
  target?: number;
  options?: Array<{ target: number }>;
}

export interface ProgramInstruction {
  id: string;
  span: { source: string; line: number };
  kind: Record<string, InstructionPayload>;
}

export type CollectionCategory = 'achievements' | 'gallery' | 'endings';

export interface Unlock {
  id: string;
  title: string;
  label: string;
  image?: string | null;
}

export interface ProgramData {
  title: string;
  project_id: string;
  fingerprint: string;
  labels: Record<string, number>;
  instructions: ProgramInstruction[];
  progress: Record<CollectionCategory, Unlock[]>;
}

export interface ThemeData
  extends Partial<
    Record<
      | 'accent_color'
      | 'background_color'
      | 'danger_color'
      | 'focus_color'
      | 'muted_text_color'
      | 'panel_color'
      | 'surface_color'
      | 'text_color',
      string
    >
  > {
  font_path?: string | null;
  font_fallbacks?: string[];
  ui_font_size?: number;
  high_contrast?: boolean;
  reduced_motion?: boolean;
  music_volume?: number;
  sound_volume?: number;
  voice_volume?: number;
}

export interface ScreenStyle {
  font_size?: number | null;
  text_color?: string | null;
  background_color?: string | null;
}

export type ScreenWidgetType =
  | 'button'
  | 'choices'
  | 'data_list'
  | 'dialogue'
  | 'drag'
  | 'drop'
  | 'extension'
  | 'image'
  | 'input'
  | 'list'
  | 'set'
  | 'slider'
  | 'text'
  | 'toggle';

export interface ScreenWidget {
  type: ScreenWidgetType;
  action?: string;
  expression?: string;
  gap?: number;
  input?: string;
  item_height?: number;
  label?: string | null;
  max_length?: number;
  name?: string;
  path?: string;
  selected?: string;
  setting?: string;
  source?: string;
  text?: string;
  variable?: string;
}

export interface ViewportFrame {
  id: string;
  bounds: Rect;
  content_height: number;
}

export interface ScreenItem {
  key: number;
  bounds: Rect;
  style?: string | null;
  widget: ScreenWidget;
  viewports?: ViewportFrame[];
}

export interface ScreensData {
  styles: Record<string, ScreenStyle>;
  layouts: Record<string, ScreenItem[]>;
}

export interface ProjectData {
  program: ProgramData;
  program_json?: string;
  catalogs: Array<{ language: string }>;
  theme: ThemeData;
  screens?: ScreensData;
  ui_zh?: Record<string, string>;
}

export interface ProfileData extends Record<CollectionCategory, string[]> {}

export interface SavePresentation {
  dialogue_page: number;
  visible_characters: number;
  sprite_elapsed_ms: number;
}

export interface SaveSummary {
  id: string;
  version: number;
  project: string;
  slot: string;
  time: number;
  note: string;
  chapter: string;
  text: string;
  thumbnail: string;
  remaining: number;
  play_time_seconds?: number;
  presentation?: Partial<SavePresentation>;
  snapshot?: string;
  checksum?: string;
}

export interface SaveRecord extends SaveSummary {
  snapshot: string;
  checksum: string;
}

export interface ReadingController {
  clock: RevealClock;
  prefix: number;
  pause(): void;
  resume(): void;
  reset(position?: number): void;
  prepare(dialogue: Dialogue): void;
  bind(node: HTMLElement, dialogue: Dialogue | null | undefined, prefix?: number): void;
  finish(): boolean;
}

export interface AudioHandle {
  currentTime: number;
  dataset: DOMStringMap;
  ended: boolean;
  loop: boolean;
  onended: ((event: Event) => unknown) | null;
  onerror: OnErrorEventHandler;
  onloadeddata: ((event: Event) => unknown) | null;
  paused: boolean;
  preload: string;
  src: string;
  volume: number;
  load(): void;
  pause(): void;
  play(): Promise<void>;
  removeAttribute(name: string): void;
}

export type AudioConstructor = new (src?: string) => AudioHandle;

export interface LayerNode {
  image: HTMLImageElement;
  layer: ImageLayer;
  path: string;
}

export interface PlayerApp {
  $(id: string): AppElement;
  element: ElementFactory;
  state: RuntimeState;
  engine: Engine;
  Engine: EngineConstructor;
  data: ProjectData;
  timer: ReturnType<typeof setTimeout> | null;
  deadline: number;
  voices: Map<string, AudioHandle>;
  route: number[];
  settings: PlayerSettings;
  profile: ProfileData;
  profileRevision: number | null;
  read: ReadStore;
  reading: ReadingController;
  clipAudio: HTMLAudioElement | null;
  clipAudioGain: number;
  streamingVideo: boolean;
  mediaNeedsGesture: boolean;
  mediaGeneration: number;
  pausedRemaining: number | null;
  layerTime: number;
  layerNodes: LayerNode[];
  refreshLayerClock?: () => void;
  viewportScroll?: Map<string, number>;
  dragExpression?: string | null;
  saveGroup?: string;
  wasRead: boolean;
  inTitle: boolean;
  graph?: unknown;
  notify(message: unknown): void;
  guard<Args extends unknown[], Result>(
    action: (...args: Args) => Result | Promise<Result>,
  ): (...args: Args) => Promise<Result | undefined>;
  asset(path: string): string;
  applySettings(): void;
  setLanguage(language: string): Promise<void>;
  openPanel(kind: string): Promise<void>;
  act(command: string, index?: number): Promise<void>;
  trackRead(): void;
  remaining(): number;
  save(slot: string, note?: string): Promise<SaveSummary>;
  restore(save: SaveRecord): Promise<void>;
  newGame(): Promise<void>;
  syncProfile(): void;
  render(elapsed?: number): Promise<void>;
  reschedule(remaining?: number): void;
  replay(path: string): Promise<void>;
  closeModal(): void;
  tr(text: string): string;
  localize(root: ParentNode): void;
  speak(text?: string | null): void;
  updateDebug?: () => void;
}

export interface PlayerSettings {
  fontSize: number;
  music_volume: number;
  sound_volume: number;
  voice_volume: number;
  auto: boolean;
  skip: boolean;
  text_speed: number;
  auto_delay: number;
  wait_voice: boolean;
  self_voicing: boolean;
  contrast: boolean;
  reduced: boolean;
  language: string;
  volume?: number;
}

export type SettingKey = keyof Pick<
  PlayerSettings,
  | 'auto'
  | 'auto_delay'
  | 'contrast'
  | 'fontSize'
  | 'music_volume'
  | 'reduced'
  | 'self_voicing'
  | 'skip'
  | 'sound_volume'
  | 'text_speed'
  | 'voice_volume'
  | 'wait_voice'
>;

export function waitingObject(waiting: WaitingState): WaitingObject {
  return waiting !== null && typeof waiting === 'object' ? waiting : {};
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

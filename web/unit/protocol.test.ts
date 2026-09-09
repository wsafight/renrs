import { describe, expect, it } from 'vitest';
import { selectVideoAudio, selectVideoSubtitle } from '../src/media';
import { parseImportedSave, parseRuntimeState, parseVideoManifest } from '../src/protocol';

describe('web protocol boundaries', () => {
  it('accepts the empty wait state emitted while the debugger is paused', () => {
    const state = parseRuntimeState(
      JSON.stringify({
        stage: { sprites: [], camera: {} },
        waiting: null,
        debug: { paused: true },
        profile_revision: 0,
        history_count: 1,
        can_rollback: true,
      }),
    );

    expect(state.waiting).toBeNull();
    expect(state.debug.paused).toBe(true);
  });

  it('reuses the previous stage when the engine omits an unchanged snapshot', () => {
    const previous = parseRuntimeState(
      JSON.stringify({
        stage: { sprites: [], camera: {}, background: 'studio.png' },
        waiting: 'Dialogue',
        debug: { paused: false },
        profile_revision: 0,
        history_count: 2,
        can_rollback: true,
      }),
    );
    const state = parseRuntimeState(
      JSON.stringify({
        stage: null,
        waiting: 'Dialogue',
        debug: { paused: false },
        profile_revision: 0,
        history_count: 2,
        can_rollback: true,
      }),
      previous,
    );
    expect(state.stage.background).toBe('studio.png');
    expect(() =>
      parseRuntimeState(
        JSON.stringify({
          stage: null,
          waiting: 'Dialogue',
          debug: { paused: false },
          profile_revision: 0,
          history_count: 2,
          can_rollback: true,
        }),
      ),
    ).toThrow(/missing stage/);
  });

  it('normalizes the portable save metadata before assigning a local slot', () => {
    const save = parseImportedSave(
      JSON.stringify({
        version: 1,
        project: 'org.renrs.test',
        time: 1234,
        snapshot: '{}',
        remaining: 250,
        note: 'Portable',
        chapter: null,
        text: null,
        thumbnail: null,
        play_time_seconds: 12,
        presentation: {
          dialogue_page: 2,
          visible_characters: 45,
          sprite_elapsed_ms: 80,
        },
      }),
    );

    expect(save).toMatchObject({
      version: 1,
      project: 'org.renrs.test',
      note: 'Portable',
      chapter: '',
      text: '',
      thumbnail: '',
      play_time_seconds: 12,
    });
    expect(save).not.toHaveProperty('id');
    expect(save).not.toHaveProperty('slot');
  });

  it('validates and selects localized video tracks', () => {
    const clip = parseVideoManifest({
      version: 2,
      fps: 24,
      stream: { path: 'video.mp4', seconds: 2, width: 1280, height: 720 },
      audio_tracks: [
        { path: 'en.wav', language: 'en', default: true, volume: 0.5 },
        { path: 'zh.wav', language: 'zh-Hans' },
      ],
      subtitles: [
        { language: 'en', default: true, cues: [{ start: 0, end: 1, text: 'Hello' }] },
        { language: 'zh', cues: [{ start: 0, end: 1, text: 'Ni hao' }] },
      ],
    });

    expect(selectVideoAudio(clip, 'zh-CN')).toMatchObject({ path: 'zh.wav', volume: 1 });
    expect(selectVideoAudio(clip, 'fr')).toMatchObject({ path: 'en.wav', volume: 0.5 });
    expect(selectVideoSubtitle(clip, 'zh-CN')?.cues[0].text).toBe('Ni hao');
    expect(() =>
      parseVideoManifest({
        version: 1,
        fps: 24,
        frames: ['frame.png'],
        audio: 'old.wav',
        audio_tracks: [{ path: 'new.wav' }],
      }),
    ).toThrow();
  });

  it.each([
    {
      name: 'unsafe soundtrack path',
      manifest: { version: 1, fps: 24, frames: ['frame.png'], audio: '../audio.wav' },
    },
    {
      name: 'missing stream dimensions',
      manifest: { version: 2, fps: 24, stream: { path: 'video.mp4', seconds: 2 } },
    },
    {
      name: 'too many audio tracks',
      manifest: {
        version: 1,
        fps: 24,
        frames: ['frame.png'],
        audio_tracks: Array.from({ length: 17 }, (_, index) => ({ path: `${index}.wav` })),
      },
    },
    {
      name: 'invalid subtitle language',
      manifest: {
        version: 1,
        fps: 1,
        frames: ['frame.png'],
        subtitles: [{ language: 'zh_CN', cues: [] }],
      },
    },
    {
      name: 'overlapping subtitle cues',
      manifest: {
        version: 1,
        fps: 1,
        frames: ['one.png', 'two.png'],
        subtitles: [
          {
            language: 'en',
            cues: [
              { start: 0, end: 1, text: 'One' },
              { start: 0.5, end: 1.5, text: 'Two' },
            ],
          },
        ],
      },
    },
  ])('rejects $name at the video protocol boundary', ({ manifest }) => {
    expect(() => parseVideoManifest(manifest)).toThrow();
  });
});

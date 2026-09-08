import { describe, expect, it } from 'vitest';
import { parseImportedSave, parseRuntimeState } from '../src/protocol';

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
});

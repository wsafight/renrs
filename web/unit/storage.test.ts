import 'fake-indexeddb/auto';
import { describe, expect, it } from 'vitest';
import { listSaves, validateSave, writeSave } from '../src/storage';
import type { SaveRecord } from '../src/types';

function save(
  project: string,
  slot: string,
  snapshot: string,
  time: number,
): Omit<SaveRecord, 'checksum'> {
  return {
    version: 1,
    id: `${project}:${slot}`,
    project,
    slot,
    time,
    note: '',
    chapter: 'start',
    text: snapshot,
    thumbnail: '',
    remaining: 0,
    snapshot,
  };
}

async function snapshotAt(project: string, slot: string): Promise<string> {
  const summary = (await listSaves(project)).find((item) => item.slot === slot);
  return (await validateSave(summary, project)).snapshot;
}

describe('save storage', () => {
  it('rotates three quick saves without losing the newest snapshots', async () => {
    const project = 'org.renrs.test.quick';
    for (const [index, snapshot] of ['one', 'two', 'three', 'four'].entries()) {
      await writeSave(save(project, 'quick', snapshot, index + 1), { prefix: 'quick', count: 3 });
    }

    expect((await listSaves(project)).map((item) => item.slot)).toEqual([
      'quick-1',
      'quick-2',
      'quick-3',
    ]);
    expect(await snapshotAt(project, 'quick-1')).toBe('four');
    expect(await snapshotAt(project, 'quick-2')).toBe('three');
    expect(await snapshotAt(project, 'quick-3')).toBe('two');
    await expect(snapshotAt(project, 'quick-4')).rejects.toThrow('Save no longer exists');
  });

  it('moves a legacy automatic save into the five-save rotation', async () => {
    const project = 'org.renrs.test.auto';
    await writeSave(save(project, 'auto', 'legacy', 1));
    await writeSave(save(project, 'auto', 'current', 2), { prefix: 'auto', count: 5 });

    expect((await listSaves(project)).map((item) => item.slot)).toEqual(['auto-1', 'auto-2']);
    expect(await snapshotAt(project, 'auto-1')).toBe('current');
    expect(await snapshotAt(project, 'auto-2')).toBe('legacy');
  });

  it('serializes concurrent writes so the last invocation remains in the slot', async () => {
    const project = 'org.renrs.test.order';
    await Promise.all(
      ['first', 'second', 'last'].map((snapshot, index) =>
        writeSave(save(project, 'slot-1', snapshot, index + 1)),
      ),
    );

    expect(await snapshotAt(project, 'slot-1')).toBe('last');
  });
});

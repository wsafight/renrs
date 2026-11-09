import { nativeDownload } from './platform';
import type { JsonValue, SaveRecord, SaveSummary } from './types';

const database = new Promise<IDBDatabase>((resolve, reject) => {
  const request = indexedDB.open('renrs-saves', 2);
  request.onupgradeneeded = () => {
    const db = request.result;
    const transaction = request.transaction;
    if (!transaction) throw new Error('Save database upgrade transaction unavailable');
    const saves = db.objectStoreNames.contains('saves')
      ? transaction.objectStore('saves')
      : db.createObjectStore('saves', { keyPath: 'id' });
    const summaries = db.createObjectStore('summaries', { keyPath: 'id' });
    summaries.createIndex('project', 'project');
    const cursor = saves.openCursor();
    cursor.onsuccess = () => {
      if (!cursor.result) return;
      summaries.put(summary(cursor.result.value));
      cursor.result.continue();
    };
  };
  request.onsuccess = () => resolve(request.result);
  request.onerror = () => reject(request.error);
});
function summary({
  snapshot: _snapshot,
  checksum: _checksum,
  ...metadata
}: SaveRecord): SaveSummary {
  return metadata;
}
async function transaction<Result>(
  mode: IDBTransactionMode,
  action: (saves: IDBObjectStore, summaries: IDBObjectStore) => IDBRequest<Result>,
): Promise<Result> {
  const db = await database;
  return new Promise<Result>((resolve, reject) => {
    const tx = db.transaction(['saves', 'summaries'], mode);
    const request = action(tx.objectStore('saves'), tx.objectStore('summaries'));
    tx.oncomplete = () => resolve(request.result);
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error || new Error('Save transaction aborted'));
  });
}
export const listSaves = (project: string): Promise<SaveSummary[]> =>
  transaction<SaveSummary[]>('readonly', (_, summaries) =>
    summaries.index('project').getAll(project),
  ).then((items) => items.sort((a, b) => b.time - a.time));
export type SaveRotation = { prefix: 'quick' | 'auto'; count: number };

let writeQueue: Promise<unknown> = Promise.resolve();
function enqueueWrite<Result>(action: () => Promise<Result>): Promise<Result> {
  const result = writeQueue.then(action);
  writeQueue = result.catch(() => undefined);
  return result;
}

export const deleteSave = (id: string): Promise<undefined> =>
  enqueueWrite(() =>
    transaction<undefined>('readwrite', (store, summaries) => {
      summaries.delete(id);
      return store.delete(id) as IDBRequest<undefined>;
    }),
  );
const checksum = async (value: string): Promise<string> =>
  Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value))))
    .map((byte) => byte.toString(16).padStart(2, '0'))
    .join('');

function rotatedSlot(slot: string, prefix: string): number | null {
  if (slot === prefix) return 1;
  const match = new RegExp(`^${prefix}-(\\d+)$`).exec(slot);
  return match ? Number(match[1]) : null;
}

export function writeSave(
  save: Omit<SaveRecord, 'checksum'>,
  rotation?: SaveRotation,
): Promise<SaveRecord> {
  const input = rotation
    ? {
        ...save,
        id: `${save.project}:${rotation.prefix}-1`,
        slot: `${rotation.prefix}-1`,
      }
    : save;
  return enqueueWrite(async () => {
    const data = { ...input, checksum: await checksum(input.snapshot) };
    if (!rotation) {
      await transaction('readwrite', (store, summaries) => {
        summaries.put(summary(data));
        return store.put(data);
      });
      return data;
    }
    await transaction<SaveRecord[]>('readwrite', (store, summaries) => {
      const request = store.getAll() as IDBRequest<SaveRecord[]>;
      request.onsuccess = () => {
        const existing = request.result
          .map((record) => ({ record, index: rotatedSlot(record.slot, rotation.prefix) }))
          .filter(
            (item): item is { record: SaveRecord; index: number } =>
              item.record.project === data.project && item.index !== null,
          )
          .sort((a, b) => b.index - a.index || a.record.time - b.record.time);
        for (const { record, index } of existing) {
          summaries.delete(record.id);
          store.delete(record.id);
          if (index >= rotation.count) continue;
          const slot = `${rotation.prefix}-${index + 1}`;
          const moved = { ...record, id: `${record.project}:${slot}`, slot };
          summaries.put(summary(moved));
          store.put(moved);
        }
        summaries.put(summary(data));
        store.put(data);
      };
      return request;
    });
    return data;
  });
}
export async function validateSave(
  save: SaveSummary | SaveRecord | undefined,
  project: string,
): Promise<SaveRecord> {
  let candidate = save;
  if (candidate && !candidate.snapshot && candidate.id) {
    const id = candidate.id;
    candidate = await transaction<SaveRecord | undefined>('readonly', (store) => store.get(id));
  }
  if (!candidate) throw new Error('Save no longer exists');
  if (
    candidate.version !== 1 ||
    candidate.project !== project ||
    typeof candidate.snapshot !== 'string' ||
    !Number.isFinite(candidate.remaining) ||
    candidate.remaining < 0 ||
    candidate.checksum !== (await checksum(candidate.snapshot))
  )
    throw new Error('Invalid save, checksum mismatch, or another project');
  return candidate as SaveRecord;
}
export async function download(
  name: string,
  data: string | JsonValue | Record<string, unknown>,
  type = 'application/json',
): Promise<void> {
  if (await nativeDownload(name, data)) return;
  const url = URL.createObjectURL(
    new Blob([typeof data === 'string' ? data : JSON.stringify(data, null, 2)], { type }),
  );
  const link = document.createElement('a');
  link.href = url;
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

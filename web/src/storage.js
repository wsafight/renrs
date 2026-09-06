const database = new Promise((resolve, reject) => {
  const request = indexedDB.open('renrs-saves', 2);
  request.onupgradeneeded = () => {
    const db = request.result;
    const saves = db.objectStoreNames.contains('saves') ? request.transaction.objectStore('saves') : db.createObjectStore('saves', {keyPath: 'id'});
    const summaries = db.createObjectStore('summaries', {keyPath: 'id'});
    summaries.createIndex('project', 'project');
    const cursor = saves.openCursor();
    cursor.onsuccess = () => {
      if (!cursor.result) return;
      summaries.put(summary(cursor.result.value)); cursor.result.continue();
    };
  };
  request.onsuccess = () => resolve(request.result);
  request.onerror = () => reject(request.error);
});
function summary({snapshot, checksum, ...metadata}) { return metadata; }
async function transaction(mode, action) {
  const db = await database;
  return new Promise((resolve, reject) => {
    const tx = db.transaction(['saves', 'summaries'], mode);
    const request = action(tx.objectStore('saves'), tx.objectStore('summaries'));
    tx.oncomplete = () => resolve(request.result);
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error || new Error('Save transaction aborted'));
  });
}
export const listSaves = project => transaction('readonly', (_, summaries) => summaries.index('project').getAll(project)).then(items => items.sort((a,b) => b.time - a.time));
export const deleteSave = id => transaction('readwrite', (store, summaries) => { summaries.delete(id); return store.delete(id); });
const checksum = async value => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', new TextEncoder().encode(value)))).map(byte => byte.toString(16).padStart(2, '0')).join('');
export async function writeSave(save) {
  const data = { ...save, checksum: await checksum(save.snapshot) };
  await transaction('readwrite', (store, summaries) => { summaries.put(summary(data)); return store.put(data); });
  return data;
}
export async function validateSave(save, project) {
  if (!save.snapshot && save.id) save = await transaction('readonly', store => store.get(save.id));
  if (!save) throw new Error('Save no longer exists');
  if (save.version !== 1 || save.project !== project || typeof save.snapshot !== 'string' || !Number.isFinite(save.remaining) || save.remaining < 0 || save.checksum !== await checksum(save.snapshot)) throw new Error('Invalid save, checksum mismatch, or another project');
  return save;
}
export async function download(name, data, type = 'application/json') {
  if (await nativeDownload(name, data)) return;
  const url = URL.createObjectURL(new Blob([typeof data === 'string' ? data : JSON.stringify(data, null, 2)], {type}));
  const link = document.createElement('a'); link.href = url; link.download = name; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
import {nativeDownload} from './platform.js';

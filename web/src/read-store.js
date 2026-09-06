export class ReadStore {
  constructor(storage, key, notify, delay = 2000) {
    this.storage = storage; this.key = key; this.notify = notify; this.delay = delay;
    this.dirty = false; this.timer = null;
    try {
      const ids = JSON.parse(storage.getItem(key) || '[]');
      if (!Array.isArray(ids) || ids.some(id => typeof id !== 'string')) throw new Error('Invalid read history');
      this.ids = new Set(ids);
    } catch (error) { this.ids = new Set(); notify(error.message); }
  }
  has(id) { return this.ids.has(id); }
  add(id) {
    if (this.ids.has(id)) return;
    this.ids.add(id); this.dirty = true;
    this.queue();
  }
  queue() {
    if (this.timer == null) this.timer = setTimeout(() => this.flush(), this.delay);
  }
  flush() {
    clearTimeout(this.timer); this.timer = null;
    if (!this.dirty) return true;
    try {
      this.storage.setItem(this.key, JSON.stringify([...this.ids]));
      this.dirty = false; return true;
    } catch (error) { this.notify(error.message); this.queue(); return false; }
  }
}

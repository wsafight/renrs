export class ReadStore {
  readonly storage: Pick<Storage, 'getItem' | 'setItem'>;
  readonly key: string;
  readonly notify: (message: unknown) => void;
  readonly delay: number;
  dirty = false;
  timer: ReturnType<typeof setTimeout> | null = null;
  ids: Set<string>;

  constructor(
    storage: Pick<Storage, 'getItem' | 'setItem'>,
    key: string,
    notify: (message: unknown) => void,
    delay = 2000,
  ) {
    this.storage = storage;
    this.key = key;
    this.notify = notify;
    this.delay = delay;
    this.dirty = false;
    this.timer = null;
    try {
      const ids = JSON.parse(storage.getItem(key) || '[]');
      if (!Array.isArray(ids) || ids.some((id) => typeof id !== 'string'))
        throw new Error('Invalid read history');
      this.ids = new Set(ids);
    } catch (error) {
      this.ids = new Set();
      notify(error instanceof Error ? error.message : error);
    }
  }
  has(id: string): boolean {
    return this.ids.has(id);
  }
  add(id: string): void {
    if (this.ids.has(id)) return;
    this.ids.add(id);
    this.dirty = true;
    this.queue();
  }
  queue(): void {
    if (this.timer == null) this.timer = setTimeout(() => this.flush(), this.delay);
  }
  flush(): boolean {
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    if (!this.dirty) return true;
    try {
      this.storage.setItem(this.key, JSON.stringify([...this.ids]));
      this.dirty = false;
      return true;
    } catch (error) {
      this.notify(error instanceof Error ? error.message : error);
      this.queue();
      return false;
    }
  }
}

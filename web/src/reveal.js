// Counts Unicode scalar values, matching the native dialogue renderer.
export class RevealClock {
  constructor() { this.total = 0; this.position = 0; this.last = null; }
  reset(total, position = 0) {
    this.total = total; this.position = Math.min(total, Math.max(0, position)); this.last = null;
  }
  get visible() { return Math.floor(this.position); }
  get complete() { return this.position >= this.total; }
  tick(now, speed) {
    if (this.last != null) this.position = Math.min(this.total, this.position + Math.max(0, now - this.last) * speed / 1000);
    this.last = now;
  }
  pause() { this.last = null; }
  finish() { this.position = this.total; this.pause(); }
}

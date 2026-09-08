// Counts Unicode scalar values, matching the native dialogue renderer.
export class RevealClock {
  total = 0;
  position = 0;
  last: number | null = null;

  reset(total: number, position = 0): void {
    this.total = total;
    this.position = Math.min(total, Math.max(0, position));
    this.last = null;
  }
  get visible(): number {
    return Math.floor(this.position);
  }
  get complete(): boolean {
    return this.position >= this.total;
  }
  tick(now: number, speed: number): void {
    if (this.last != null)
      this.position = Math.min(
        this.total,
        this.position + (Math.max(0, now - this.last) * speed) / 1000,
      );
    this.last = now;
  }
  pause(): void {
    this.last = null;
  }
  finish(): void {
    this.position = this.total;
    this.pause();
  }
}

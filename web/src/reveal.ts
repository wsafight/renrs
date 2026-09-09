export interface RevealCue {
  position: number;
  delayMs: number | null;
  fast: boolean;
}

// Counts Unicode scalar values, matching the native dialogue renderer.
export class RevealClock {
  total = 0;
  position = 0;
  last: number | null = null;
  cues: RevealCue[] = [];
  cueIndex = 0;
  blockedRemaining: number | null | undefined;

  reset(total: number, position = 0, cues: RevealCue[] = []): void {
    this.total = total;
    this.position = Math.min(total, Math.max(0, position));
    this.cues = cues;
    this.cueIndex = cues.findIndex((cue) => cue.position >= this.position);
    if (this.cueIndex < 0) this.cueIndex = cues.length;
    this.blockedRemaining = undefined;
    this.last = null;
  }
  get visible(): number {
    return Math.floor(this.position);
  }
  get complete(): boolean {
    return this.position >= this.total && !this.blocked;
  }
  get blocked(): boolean {
    return this.blockedRemaining !== undefined;
  }
  get needsTick(): boolean {
    return !this.complete && (!this.blocked || this.blockedRemaining !== null);
  }
  tick(now: number, speed: number): void {
    const elapsed = this.last == null ? 0 : Math.max(0, now - this.last);
    this.last = now;
    const remaining = this.blockedRemaining;
    if (remaining !== undefined) {
      if (remaining === null) return;
      this.blockedRemaining = remaining - elapsed;
      if (this.blockedRemaining > 0) return;
      this.release();
      this.last = now;
    } else {
      this.position = Math.min(this.total, this.position + (elapsed * speed) / 1000);
    }
    this.processCues();
    if (this.blocked && this.last === null) this.last = now;
  }
  pause(): void {
    this.last = null;
  }
  finish(): void {
    this.position = this.total;
    this.cueIndex = this.cues.length;
    this.blockedRemaining = undefined;
    this.pause();
  }

  skip(): boolean {
    if (this.blocked) {
      this.release();
      this.pause();
      this.processCues();
      return true;
    }
    if (this.complete) return false;
    while (this.cueIndex < this.cues.length && this.cues[this.cueIndex].fast)
      this.cueIndex += 1;
    const cue = this.cues.slice(this.cueIndex).find((candidate) => !candidate.fast);
    if (cue) {
      this.cueIndex = this.cues.indexOf(cue, this.cueIndex);
      this.position = cue.position;
      this.processCues();
    } else {
      this.cueIndex = this.cues.length;
      this.finish();
    }
    return true;
  }

  private release(): void {
    this.blockedRemaining = undefined;
    this.cueIndex += 1;
  }

  private processCues(): void {
    while (this.cueIndex < this.cues.length) {
      const cue = this.cues[this.cueIndex];
      if (cue.position > this.position) return;
      if (cue.fast) {
        this.cueIndex += 1;
        const next = this.cues.slice(this.cueIndex).find((candidate) => !candidate.fast);
        this.position = next?.position ?? this.total;
        continue;
      }
      this.position = cue.position;
      this.blockedRemaining = cue.delayMs;
      this.pause();
      return;
    }
  }
}

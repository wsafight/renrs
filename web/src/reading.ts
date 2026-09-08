import { richText } from './presentation';
import { RevealClock } from './reveal';
import type { Dialogue, PlayerApp } from './types';

export function setupReading(app: PlayerApp): void {
  const clock = new RevealClock();
  let frame: number | null = null;
  let bindings: Array<(visible: number) => void> = [];
  let text = '';
  let pendingPosition: number | null = 0;
  const paused = () =>
    document.hidden || app.$('modal').open || app.inTitle || app.state?.debug.paused;
  const paint = () => {
    for (const update of bindings) update(clock.visible);
  };
  const stop = () => {
    if (frame !== null) cancelAnimationFrame(frame);
    frame = null;
    clock.pause();
  };
  const animate = (now: number) => {
    frame = null;
    if (paused()) {
      clock.pause();
      return;
    }
    const previous = clock.visible;
    clock.tick(now, app.settings.text_speed);
    if (previous !== clock.visible) paint();
    if (clock.complete) app.reschedule();
    else frame = requestAnimationFrame(animate);
  };
  const resume = () => {
    if (frame == null && !paused() && !clock.complete) frame = requestAnimationFrame(animate);
  };
  app.reading = {
    clock,
    prefix: 0,
    pause: stop,
    resume,
    reset(position = 0) {
      stop();
      pendingPosition = position;
    },
    prepare(dialogue: Dialogue) {
      stop();
      bindings = [];
      const current = app.state.stage.dialogue?.text || '';
      if (pendingPosition != null || text !== current) {
        clock.reset(Array.from(current).length, pendingPosition ?? clock.visible);
        pendingPosition = null;
        text = current;
      }
      if (app.state.waiting !== 'Dialogue' || (app.settings.skip && app.wasRead)) clock.finish();
      this.prefix = Math.max(0, Array.from(dialogue.text || '').length - clock.total);
    },
    bind(node: HTMLElement, dialogue: Dialogue | null | undefined, prefix = app.reading.prefix) {
      const update = richText(node, dialogue, clock.visible + prefix);
      bindings.push((visible: number) => update(visible + prefix));
    },
    finish() {
      if (clock.complete) return false;
      stop();
      clock.finish();
      paint();
      app.reschedule();
      return true;
    },
  };
}

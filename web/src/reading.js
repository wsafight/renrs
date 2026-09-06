import {RevealClock} from './reveal.js';
import {richText} from './presentation.js';

export function setupReading(app) {
  const clock = new RevealClock();
  let frame = null, bindings = [], text = '', pendingPosition = 0;
  const paused = () => document.hidden || app.$('modal').open || app.inTitle || app.state?.debug.paused;
  const paint = () => { for (const update of bindings) update(clock.visible); };
  const stop = () => { cancelAnimationFrame(frame); frame = null; clock.pause(); };
  const animate = now => {
    frame = null;
    if (paused()) { clock.pause(); return; }
    const previous = clock.visible;
    clock.tick(now, app.settings.text_speed);
    if (previous !== clock.visible) paint();
    if (clock.complete) app.reschedule(); else frame = requestAnimationFrame(animate);
  };
  const resume = () => {
    if (frame == null && !paused() && !clock.complete) frame = requestAnimationFrame(animate);
  };
  app.reading = {
    clock, pause: stop, resume,
    reset(position = 0) { stop(); pendingPosition = position; },
    prepare(dialogue) {
      stop(); bindings = [];
      const current = app.state.stage.dialogue?.text || '';
      if (pendingPosition != null || text !== current) {
        clock.reset(Array.from(current).length, pendingPosition ?? clock.visible);
        pendingPosition = null; text = current;
      }
      if (app.state.waiting !== 'Dialogue' || (app.settings.skip && app.wasRead)) clock.finish();
      this.prefix = Math.max(0, Array.from(dialogue.text || '').length - clock.total);
    },
    bind(node, dialogue, prefix = this.prefix) {
      const update = richText(node, dialogue, clock.visible + prefix);
      bindings.push(visible => update(visible + prefix));
    },
    finish() {
      if (clock.complete) return false;
      stop(); clock.finish(); paint(); app.reschedule(); return true;
    },
  };
}

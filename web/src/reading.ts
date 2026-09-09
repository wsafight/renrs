import { richText } from './presentation';
import { RevealClock } from './reveal';
import type { RevealCue } from './reveal';
import type { Dialogue, PlayerApp, TextRun } from './types';

function revealCues(runs: TextRun[] | undefined) {
  let position = 0;
  const cues: RevealCue[] = [];
  for (const run of runs || []) {
    const cue = run.cue;
    if (cue === 'Fast') cues.push({ position, delayMs: null, fast: true });
    else if (cue && cue !== 'NoWait') {
      const value = 'Wait' in cue ? cue.Wait : cue.Page;
      cues.push({
        position,
        delayMs: value.hundredths == null ? null : value.hundredths * 10,
        fast: false,
      });
    }
    position += Array.from(run.text).length;
  }
  return cues;
}

export function setupReading(app: PlayerApp): void {
  const clock = new RevealClock();
  let frame: number | null = null;
  let bindings: Array<(visible: number) => void> = [];
  let dialogueKey = '';
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
    else if (clock.needsTick) frame = requestAnimationFrame(animate);
  };
  const resume = () => {
    if (frame == null && !paused() && clock.needsTick) frame = requestAnimationFrame(animate);
  };
  app.reading = {
    clock,
    prefix: 0,
    get blocked() {
      return clock.blocked;
    },
    pause: stop,
    resume,
    reset(position = 0) {
      stop();
      pendingPosition = position;
    },
    prepare(dialogue: Dialogue) {
      stop();
      bindings = [];
      const currentDialogue = app.state.stage.dialogue,
        current = currentDialogue?.text || '',
        currentKey = `${currentDialogue?.statement_id || ''}\u0000${current}\u0000${JSON.stringify(currentDialogue?.runs?.map((run) => run.cue))}`;
      if (pendingPosition != null || dialogueKey !== currentKey) {
        clock.reset(
          Array.from(current).length,
          pendingPosition ?? clock.visible,
          revealCues(currentDialogue?.runs),
        );
        pendingPosition = null;
        dialogueKey = currentKey;
      }
      if (app.state.waiting !== 'Dialogue' || (app.settings.skip && app.wasRead)) clock.finish();
      this.prefix = Math.max(0, Array.from(dialogue.text || '').length - clock.total);
    },
    bind(node: HTMLElement, dialogue: Dialogue | null | undefined, prefix = app.reading.prefix) {
      const update = richText(node, dialogue, clock.visible + prefix);
      bindings.push((visible: number) => update(visible + prefix));
    },
    finish() {
      stop();
      if (!clock.skip()) return false;
      paint();
      if (clock.complete) app.reschedule();
      else resume();
      return true;
    },
  };
}

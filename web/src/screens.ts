import { richText, storyAnnouncement } from './presentation';
import { parseHistory, parseJson, parseRuntimeState } from './protocol';
import { dataControl, viewportParent } from './screen-composition';
import { listSaves, validateSave } from './storage';
import type { PlayerApp, ScreenWidget, SettingKey } from './types';
import { waitingObject } from './types';

type Preference = readonly [SettingKey, number?, number?, number?];
const preferences: Record<string, Preference> = {
  text_speed: ['text_speed', 10, 100, 5],
  auto_delay: ['auto_delay', 0.5, 10, 0.5],
  music_volume: ['music_volume', 0, 1, 0.05],
  sound_volume: ['sound_volume', 0, 1, 0.05],
  voice_volume: ['voice_volume', 0, 1, 0.05],
  high_contrast: ['contrast'],
  reduced_motion: ['reduced'],
  wait_voice: ['wait_voice'],
  self_voicing: ['self_voicing'],
};
interface ListEntry {
  text: string;
  run: () => unknown | Promise<unknown>;
  disabled?: boolean;
}

function translated(app: PlayerApp, text: string): string {
  try {
    return app.engine.screen_text(app.tr(text));
  } catch {
    return app.tr(text);
  }
}
function button(
  app: PlayerApp,
  text: string,
  action: () => unknown | Promise<unknown>,
): HTMLButtonElement {
  const node = app.element('button', text);
  node.onclick = app.guard(action);
  return node;
}
async function action(app: PlayerApp, command: string): Promise<unknown> {
  if (command === 'new_game') {
    if (!app.inTitle && !confirm(app.tr('Start a new game?'))) return;
    app.inTitle = false;
    app.closeModal();
    await app.newGame();
    return;
  }
  if (command === 'continue' || command === 'quick_load') {
    const saves = await listSaves(app.data.program.project_id);
    const save =
      command === 'quick_load' ? saves.find((item) => item.slot === 'quick-1') : saves[0];
    if (save && (app.inTitle || confirm(app.tr('Replace current progress?')))) {
      app.inTitle = false;
      app.closeModal();
      await app.restore(await validateSave(save, app.data.program.project_id));
    }
    return;
  }
  if (command === 'close') return app.closeModal();
  if (command === 'quit') {
    await app.save('resume');
    app.inTitle = true;
    app.closeModal();
    return app.render();
  }
  if (command === 'quick_save') return app.save('quick-1');
  if (command === 'rollback') {
    app.closeModal();
    return app.act('rollback');
  }
  if (command === 'auto' || command === 'skip') {
    app.settings[command] = !app.settings[command];
    app.applySettings();
    app.closeModal();
    return app.reschedule();
  }
  const panel =
    ({ manual_saves: 'load', quick_saves: 'load', auto_saves: 'load' } as Record<string, string>)[
      command
    ] || command;
  if (command.endsWith('_saves')) app.saveGroup = command.split('_')[0];
  return app.openPanel(panel);
}

async function list(
  app: PlayerApp,
  widget: ScreenWidget,
  kind: string,
  root: HTMLElement,
): Promise<void> {
  root.classList.add('custom-list');
  root.tabIndex = 0;
  let items: ListEntry[] = [];
  if (widget.source === 'history') {
    const pages = Math.max(1, Math.ceil(app.state.history_count / 50));
    const show = (page: number) => {
      root.replaceChildren();
      for (const entry of parseHistory(app.engine.history(page * 50, 50))) {
        const row = app.element('article');
        row.append(app.element('strong', entry.speaker_name || ''));
        const text = app.element('p');
        richText(text, entry);
        row.append(text);
        root.append(row);
      }
      const bar = app.element('nav', null, { className: 'pagination' });
      const previous = button(app, '<', () => show(page - 1));
      previous.ariaLabel = app.tr('Previous');
      previous.disabled = page === 0;
      const next = button(app, '>', () => show(page + 1));
      next.ariaLabel = app.tr('Next');
      next.disabled = page + 1 === pages;
      bar.append(previous, app.element('span', `${page + 1} / ${pages}`), next);
      root.append(bar);
    };
    show(pages - 1);
    return;
  }
  if (widget.source === 'languages') {
    items = [
      { text: app.tr('Source text'), run: () => app.setLanguage('') },
      ...app.data.catalogs.map((catalog) => ({
        text: catalog.language,
        run: () => app.setLanguage(catalog.language),
      })),
    ];
  } else {
    const saves = await listSaves(app.data.program.project_id);
    const source = widget.source ?? 'saves';
    const group =
      source === 'saves'
        ? kind === 'save'
          ? 'manual'
          : app.saveGroup || 'manual'
        : source.split('_')[0];
    if (group === 'manual') {
      items = Array.from({ length: 60 }, (_, index) => {
        const slot = `slot-${index + 1}`,
          save = saves.find((item) => item.slot === slot);
        return {
          text: `${index + 1}. ${save?.note || save?.text || app.tr('Empty')}`,
          disabled: kind !== 'save' && !save,
          run: async () => {
            if (kind === 'save') {
              if (!save || confirm(app.tr('Replace this save?'))) {
                await app.save(slot);
                await app.openPanel(kind);
              }
            } else if (confirm(app.tr('Replace current progress?'))) {
              await app.restore(await validateSave(save, app.data.program.project_id));
              app.closeModal();
            }
          },
        };
      });
    } else {
      items = saves
        .filter((save) => save.slot.startsWith(group === 'quick' ? 'quick-' : 'auto'))
        .map((save) => ({
          text: `${save.slot}: ${save.note || save.text}`,
          run: async () => {
            if (confirm(app.tr('Replace current progress?'))) {
              await app.restore(await validateSave(save, app.data.program.project_id));
              app.closeModal();
            }
          },
        }));
    }
  }
  for (const item of items) {
    const node = button(app, item.text, item.run);
    node.disabled = !!item.disabled;
    root.append(node);
  }
  if (!items.length) root.append(app.element('p', app.tr('Empty')));
}

export async function customScreen(
  app: PlayerApp,
  kind: string,
  host: HTMLElement,
): Promise<boolean> {
  const screens = app.data.screens;
  const layout = screens?.layouts?.[kind];
  if (!layout) return false;
  const root = app.element('div', null, { className: 'custom-screen' });
  root.dataset.screen = kind;
  root.setAttribute('role', 'group');
  root.setAttribute('aria-label', translated(app, kind.replaceAll('_', ' ')));
  const viewports = new Map<string, { node: HTMLDivElement; content: HTMLDivElement }>();
  if (layout.some((item) => item.viewports?.length)) root.classList.add('has-viewports');
  for (const item of layout) {
    const widget = item.widget,
      style = screens.styles[item.style ?? ''] || {};
    const node = app.element('div', null, { className: `custom-widget custom-${widget.type}` });
    if (widget.label) node.setAttribute('aria-label', translated(app, widget.label));
    const { host: parent, position } = viewportParent(app, kind, root, item, viewports);
    Object.assign(node.style, {
      ...position,
      color: style.text_color || '',
      backgroundColor: style.background_color || '',
      fontSize: `${style.font_size || app.data.theme.ui_font_size || 22}px`,
    });
    const text = translated(app, widget.text || '');
    if (['data_list', 'drag', 'drop', 'extension'].includes(widget.type))
      await dataControl(app, kind, widget, node, text);
    else if (widget.type === 'text') node.textContent = text;
    else if (widget.type === 'image' && widget.path)
      node.append(
        app.element('img', null, {
          src: app.asset(widget.path),
          alt: widget.label ? translated(app, widget.label) : '',
        }),
      );
    else if (widget.type === 'button') {
      const control = button(app, text, () => action(app, widget.action ?? ''));
      if (widget.action === 'rollback') control.disabled = !app.state.can_rollback;
      if (widget.action === 'continue' || widget.action === 'quick_load') {
        const saves = await listSaves(app.data.program.project_id);
        control.disabled =
          widget.action === 'quick_load'
            ? !saves.some((save) => save.slot === 'quick-1')
            : !saves.length;
      }
      node.append(control);
    } else if (widget.type === 'set') {
      const control = button(app, text, async () => {
        if (!widget.variable || !widget.expression)
          throw new Error('Set widget requires variable and expression');
        app.state = parseRuntimeState(
          app.engine.apply_expression(widget.variable, widget.expression),
        );
        app.syncProfile();
        if (app.$('modal').open) await app.openPanel(kind);
        else await app.render();
      });
      control.disabled = !(
        app.state.waiting === 'Dialogue' || waitingObject(app.state.waiting).Choice
      );
      node.append(control);
    } else if (widget.type === 'dialogue') {
      const dialogue = app.state.stage.dialogue;
      node.setAttribute('role', 'group');
      node.setAttribute('aria-label', storyAnnouncement(dialogue));
      const speaker = app.element('strong', dialogue?.speaker_name || '');
      speaker.style.color = dialogue?.speaker_color || '';
      const body = app.element('p');
      app.reading.bind(body, dialogue, 0);
      node.append(
        speaker,
        body,
        button(app, app.tr('Continue'), () => app.act('next')),
      );
    } else if (widget.type === 'choices') {
      node.setAttribute('role', 'group');
      node.setAttribute('aria-label', app.tr('Choices'));
      for (const [index, option] of (
        waitingObject(app.state.waiting).Choice?.options || []
      ).entries())
        node.append(button(app, option, () => app.act('choose', index)));
    } else if (widget.type === 'list') await list(app, widget, kind, node);
    else if (widget.type === 'input') {
      const label = app.element('label', text);
      if (!widget.variable) throw new Error('Input widget requires variable');
      const value = parseJson(
        app.engine.screen_value(widget.variable),
        `screen value ${widget.variable}`,
      );
      const input = app.element('input', null, {
        type: 'text',
        maxLength: widget.max_length,
        value: typeof value === 'string' ? value : '',
      });
      input.oninput = app.guard(() => {
        app.state = parseRuntimeState(
          app.engine.set_variable(widget.variable ?? '', JSON.stringify(input.value)),
        );
        app.syncProfile();
      });
      label.append(input);
      node.append(label);
    } else if (widget.type === 'slider' || widget.type === 'toggle') {
      const preference = widget.setting ? preferences[widget.setting] : undefined;
      if (!preference) throw new Error(`Unknown screen preference: ${widget.setting ?? ''}`);
      const [key, min, max, step] = preference;
      const label = app.element('label', text),
        input = app.element('input', null, {
          type: widget.type === 'toggle' ? 'checkbox' : 'range',
        });
      if (widget.type === 'toggle') input.checked = Boolean(app.settings[key]);
      else Object.assign(input, { min, max, step, value: app.settings[key] });
      input.oninput = () => {
        Object.assign(app.settings, {
          [key]: widget.type === 'toggle' ? input.checked : Number(input.value),
        });
        app.applySettings();
      };
      label.append(input);
      node.append(label);
    }
    parent.append(node);
  }
  host.append(root);
  return true;
}

export async function storyScreens(app: PlayerApp): Promise<void> {
  app
    .$('game')
    .querySelectorAll(':scope > .custom-screen')
    .forEach((node) => {
      node.remove();
    });
  app.$('dialogue').hidden = app.$('choices').hidden = !!app.inTitle;
  if (app.inTitle) {
    if (!(await customScreen(app, 'main_menu', app.$('game')))) {
      app.inTitle = false;
      app.$('dialogue').hidden = false;
    }
    return;
  }
  await customScreen(app, 'hud', app.$('game'));
  const kind =
    app.state.waiting === 'Dialogue' && !app.state.stage.nvl
      ? 'dialogue'
      : waitingObject(app.state.waiting).Choice
        ? 'choices'
        : null;
  if (kind && (await customScreen(app, kind, app.$('game'))))
    app.$(kind === 'dialogue' ? 'dialogue' : 'choices').hidden = true;
}

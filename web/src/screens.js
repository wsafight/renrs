import {listSaves, validateSave} from './storage.js';
import {richText} from './presentation.js';
import {viewportParent, dataControl} from './screen-composition.js';

const preferences = {
  text_speed: ['text_speed', 10, 100, 5], auto_delay: ['auto_delay', .5, 10, .5],
  music_volume: ['music_volume', 0, 1, .05], sound_volume: ['sound_volume', 0, 1, .05], voice_volume: ['voice_volume', 0, 1, .05],
  high_contrast: ['contrast'], reduced_motion: ['reduced'], wait_voice: ['wait_voice'], self_voicing: ['self_voicing'],
};
function translated(app, text) {
  try { return app.engine.screen_text(app.tr(text)); } catch { return app.tr(text); }
}
function button(app, text, action) {
  const node = app.element('button', text);
  node.onclick = app.guard(action);
  return node;
}
async function action(app, command) {
  if (command === 'new_game') {
    if (!app.inTitle && !confirm(app.tr('Start a new game?'))) return;
    app.inTitle = false; app.closeModal(); await app.newGame(); return;
  }
  if (command === 'continue' || command === 'quick_load') {
    const saves = await listSaves(app.data.program.project_id);
    const save = command === 'quick_load' ? saves.find(item => item.slot === 'quick-1') : saves[0];
    if (save && (app.inTitle || confirm(app.tr('Replace current progress?')))) {
      app.inTitle = false; app.closeModal(); await app.restore(await validateSave(save, app.data.program.project_id));
    }
    return;
  }
  if (command === 'close') return app.closeModal();
  if (command === 'quit') { await app.save('resume'); app.inTitle = true; app.closeModal(); return app.render(); }
  if (command === 'quick_save') return app.save('quick-1');
  if (command === 'rollback') { app.closeModal(); return app.act('rollback'); }
  if (command === 'auto' || command === 'skip') { app.settings[command] = !app.settings[command]; app.applySettings(); app.closeModal(); return app.reschedule(); }
  const panel = {manual_saves: 'load', quick_saves: 'load', auto_saves: 'load'}[command] || command;
  if (command.endsWith('_saves')) app.saveGroup = command.split('_')[0];
  return app.openPanel(panel);
}

async function list(app, widget, kind, root) {
  root.classList.add('custom-list'); root.tabIndex = 0;
  let items = [];
  if (widget.source === 'history') {
    const pages = Math.max(1, Math.ceil(app.state.history_count / 50));
    const show = page => {
      root.replaceChildren();
      for (const entry of JSON.parse(app.engine.history(page * 50, 50))) {
        const row = app.element('article'); row.append(app.element('strong', entry.speaker_name || ''));
        const text = app.element('p'); richText(text, entry); row.append(text); root.append(row);
      }
      const bar = app.element('nav', null, {className: 'pagination'});
      const previous = button(app, '<', () => show(page - 1)); previous.ariaLabel = app.tr('Previous'); previous.disabled = page === 0;
      const next = button(app, '>', () => show(page + 1)); next.ariaLabel = app.tr('Next'); next.disabled = page + 1 === pages;
      bar.append(previous, app.element('span', `${page + 1} / ${pages}`), next); root.append(bar);
    };
    show(pages - 1);
    return;
  }
  if (widget.source === 'languages') {
    items = [{text: app.tr('Source text'), run: () => app.setLanguage('')}, ...app.data.catalogs.map(catalog => ({text: catalog.language, run: () => app.setLanguage(catalog.language)}))];
  } else {
    const saves = await listSaves(app.data.program.project_id);
    const group = widget.source === 'saves' ? kind === 'save' ? 'manual' : app.saveGroup || 'manual' : widget.source.split('_')[0];
    if (group === 'manual') {
      items = Array.from({length: 60}, (_, index) => {
        const slot = `slot-${index + 1}`, save = saves.find(item => item.slot === slot);
        return {text: `${index + 1}. ${save?.note || save?.text || app.tr('Empty')}`, disabled: kind !== 'save' && !save, run: async () => {
          if (kind === 'save') { if (!save || confirm(app.tr('Replace this save?'))) { await app.save(slot); await app.openPanel(kind); } }
          else if (confirm(app.tr('Replace current progress?'))) { await app.restore(await validateSave(save, app.data.program.project_id)); app.closeModal(); }
        }};
      });
    } else {
      items = saves.filter(save => save.slot.startsWith(group === 'quick' ? 'quick-' : 'auto')).map(save => ({text: `${save.slot}: ${save.note || save.text}`, run: async () => {
        if (confirm(app.tr('Replace current progress?'))) { await app.restore(await validateSave(save, app.data.program.project_id)); app.closeModal(); }
      }}));
    }
  }
  for (const item of items) { const node = button(app, item.text, item.run); node.disabled = !!item.disabled; root.append(node); }
  if (!items.length) root.append(app.element('p', app.tr('Empty')));
}

export async function customScreen(app, kind, host) {
  const layout = app.data.screens?.layouts?.[kind];
  if (!layout) return false;
  const root = app.element('div', null, {className: 'custom-screen'}); root.dataset.screen = kind;
  const viewports = new Map();
  if (layout.some(item => item.viewports?.length)) root.classList.add('has-viewports');
  for (const item of layout) {
    const widget = item.widget, style = app.data.screens.styles[item.style] || {}, bounds = item.bounds;
    const node = app.element('div', null, {className: `custom-widget custom-${widget.type}`});
    const {host: parent, position} = viewportParent(app, kind, root, item, viewports);
    Object.assign(node.style, {...position, color: style.text_color || '', backgroundColor: style.background_color || '', fontSize: `${style.font_size || app.data.theme.ui_font_size || 22}px`});
    const text = translated(app, widget.text || '');
    if (['data_list', 'drag', 'drop', 'extension'].includes(widget.type)) await dataControl(app, kind, widget, node, text);
    else if (widget.type === 'text') node.textContent = text;
    else if (widget.type === 'image') node.append(app.element('img', null, {src: app.asset(widget.path), alt: ''}));
    else if (widget.type === 'button') {
      const control = button(app, text, () => action(app, widget.action));
      if (widget.action === 'rollback') control.disabled = !app.state.can_rollback;
      if (widget.action === 'continue' || widget.action === 'quick_load') {
        const saves = await listSaves(app.data.program.project_id);
        control.disabled = widget.action === 'quick_load' ? !saves.some(save => save.slot === 'quick-1') : !saves.length;
      }
      node.append(control);
    }
    else if (widget.type === 'set') {
      const control = button(app, text, async () => {
        app.state = JSON.parse(app.engine.apply_expression(widget.variable, widget.expression)); app.syncProfile();
        if (app.$('modal').open) await app.openPanel(kind); else await app.render();
      });
      control.disabled = !(app.state.waiting === 'Dialogue' || app.state.waiting?.Choice);
      node.append(control);
    }
    else if (widget.type === 'dialogue') {
      const dialogue = app.state.stage.dialogue;
      const speaker = app.element('strong', dialogue?.speaker_name || ''); speaker.style.color = dialogue?.speaker_color || '';
      const body = app.element('p'); app.reading.bind(body, dialogue, 0);
      node.append(speaker, body, button(app, app.tr('Continue'), () => app.act('next')));
    } else if (widget.type === 'choices') {
      for (const [index, option] of (app.state.waiting?.Choice?.options || []).entries()) node.append(button(app, option, () => app.act('choose', index)));
    } else if (widget.type === 'list') await list(app, widget, kind, node);
    else if (widget.type === 'input') {
      const label = app.element('label', text);
      const input = app.element('input', null, {type: 'text', maxLength: widget.max_length, value: JSON.parse(app.engine.screen_value(widget.variable)) || ''});
      input.oninput = app.guard(() => { app.state = JSON.parse(app.engine.set_variable(widget.variable, JSON.stringify(input.value))); app.syncProfile(); });
      label.append(input); node.append(label);
    } else if (widget.type === 'slider' || widget.type === 'toggle') {
      const [key, min, max, step] = preferences[widget.setting];
      const label = app.element('label', text), input = app.element('input', null, {type: widget.type === 'toggle' ? 'checkbox' : 'range'});
      if (widget.type === 'toggle') input.checked = app.settings[key];
      else Object.assign(input, {min, max, step, value: app.settings[key]});
      input.oninput = () => { app.settings[key] = widget.type === 'toggle' ? input.checked : Number(input.value); app.applySettings(); };
      label.append(input); node.append(label);
    }
    parent.append(node);
  }
  host.append(root);
  return true;
}

export async function storyScreens(app) {
  app.$('game').querySelectorAll(':scope > .custom-screen').forEach(node => node.remove());
  app.$('dialogue').hidden = app.$('choices').hidden = !!app.inTitle;
  if (app.inTitle) {
    if (!await customScreen(app, 'main_menu', app.$('game'))) {
      app.inTitle = false; app.$('dialogue').hidden = false;
    }
    return;
  }
  await customScreen(app, 'hud', app.$('game'));
  const kind = app.state.waiting === 'Dialogue' && !app.state.stage.nvl ? 'dialogue' : app.state.waiting?.Choice ? 'choices' : null;
  if (kind && await customScreen(app, kind, app.$('game'))) app.$(kind === 'dialogue' ? 'dialogue' : 'choices').hidden = true;
}

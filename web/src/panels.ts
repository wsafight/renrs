import { richText } from './presentation';
import { parseHistory, parseImportedSave } from './protocol';
import { deleteSave, download, listSaves, validateSave, writeSave } from './storage';
import type { CollectionCategory, PlayerApp, SettingKey } from './types';

function open(app: PlayerApp, title: string): HTMLElement {
  app.reading.pause();
  app.clipAudio?.pause();
  if (app.streamingVideo) app.$('video').pause();
  if (!app.$('modal').open) app.pausedRemaining = app.remaining();
  for (const animation of app.$('stage').getAnimations({ subtree: true })) animation.pause();
  if (app.timer !== null) clearTimeout(app.timer);
  app.$('modal-title').textContent = app.tr(title);
  app.$('modal-body').replaceChildren();
  if (!app.$('modal').open) app.$('modal').showModal();
  return app.$('modal-body');
}
function button(
  app: PlayerApp,
  text: string,
  callback: () => unknown | Promise<unknown>,
): HTMLButtonElement {
  const node = app.element('button', app.tr(text));
  node.onclick = app.guard(callback);
  return node;
}

type SettingRow = readonly [SettingKey, string, 'checkbox' | 'range', number?, number?, number?];

export const panels = {
  open,
  async saves(app: PlayerApp, page = 0): Promise<void> {
    const saved = await listSaves(app.data.program.project_id);
    const body = open(app, 'Saves');
    const bar = app.element('div', null, { className: 'commands' });
    const picker = app.element('input', null, { type: 'file', accept: '.json', hidden: true });
    picker.onchange = app.guard(async () => {
      if (!picker.files?.length) return;
      const text = await picker.files[0].text();
      const save = parseImportedSave(app.engine.import_save(text));
      const empty = Array.from({ length: 60 }, (_, i) => `slot-${i + 1}`).find(
        (slot) => !saved.some((item) => item.slot === slot),
      );
      if (!empty) throw new Error(app.tr('No empty manual slot'));
      await writeSave({ ...save, id: `${save.project}:${empty}`, slot: empty });
      await panels.saves(app, page);
    });
    bar.append(
      button(app, 'Import', () => picker.click()),
      picker,
    );
    for (const slot of ['resume', 'auto']) {
      const save = saved.find((item) => item.slot === slot);
      if (save)
        bar.append(
          button(app, slot === 'resume' ? 'Resume' : 'Autosave', async () => {
            if (confirm(app.tr('Replace current progress?'))) {
              await app.restore(await validateSave(save, app.data.program.project_id));
              app.closeModal();
            }
          }),
        );
    }
    body.append(bar);
    for (let index = page * 6 + 1; index <= page * 6 + 6; index++) {
      const slot = `slot-${index}`,
        save = saved.find((item) => item.slot === slot);
      const row = app.element('div', null, { className: 'slot' });
      const image = app.element('img', null, { alt: '' });
      if (save?.thumbnail) image.src = app.asset(save.thumbnail);
      else image.style.visibility = 'hidden';
      const info = app.element('div');
      info.append(app.element('h3', `${app.tr('Slot')} ${index}`));
      if (save)
        info.append(
          app.element('small', `${save.chapter || ''} / ${new Date(save.time).toLocaleString()}`),
          app.element('p', save.note || save.text),
        );
      const note = app.element('input', null, {
        placeholder: app.tr('Note'),
        value: save?.note || '',
        maxLength: 256,
        ariaLabel: `${app.tr('Slot')} ${index} ${app.tr('note')}`,
      });
      const commands = app.element('div', null, { className: 'commands' });
      commands.append(
        button(app, 'Save', async () => {
          if (!save || confirm(app.tr('Replace this save?'))) {
            await app.save(slot, note.value);
            await panels.saves(app, page);
          }
        }),
      );
      if (save) {
        commands.append(
          button(app, 'Load', async () => {
            if (confirm(app.tr('Replace current progress?'))) {
              await app.restore(await validateSave(save, app.data.program.project_id));
              app.closeModal();
            }
          }),
          button(app, 'Export', async () => {
            const full = await validateSave(save, app.data.program.project_id);
            await download(
              `${slot}.json`,
              app.engine.export_save(full.snapshot, JSON.stringify(full)),
            );
          }),
          button(app, 'Delete', async () => {
            if (confirm(app.tr('Delete this save?'))) {
              await deleteSave(save.id);
              await panels.saves(app, page);
            }
          }),
        );
      }
      info.append(note, commands);
      row.append(image, info);
      body.append(row);
    }
    const pagination = app.element('div', null, { className: 'pagination' });
    const previous = button(app, '<', () => panels.saves(app, page - 1));
    previous.disabled = page === 0;
    const next = button(app, '>', () => panels.saves(app, page + 1));
    next.disabled = page === 9;
    pagination.append(previous, app.element('span', `${page + 1} / 10`), next);
    body.append(pagination);
  },
  history(app: PlayerApp, page = Math.max(0, Math.ceil(app.state.history_count / 50) - 1)): void {
    const body = open(app, 'History');
    const pages = Math.max(1, Math.ceil(app.state.history_count / 50));
    for (const item of parseHistory(app.engine.history(page * 50, 50))) {
      const row = app.element('div', null, { className: 'history-row' });
      const text = app.element('p');
      richText(text, item);
      row.append(app.element('strong', item.speaker_name || ''), text);
      if (item.voice_path) {
        const voicePath = item.voice_path;
        row.append(button(app, 'Play voice', () => app.replay(voicePath)));
      }
      body.append(row);
    }
    const bar = app.element('div', null, { className: 'pagination' });
    const previous = button(app, '<', () => panels.history(app, page - 1));
    previous.disabled = page === 0;
    const next = button(app, '>', () => panels.history(app, page + 1));
    next.disabled = page + 1 >= pages;
    bar.append(previous, app.element('span', `${page + 1} / ${pages}`), next);
    body.append(bar);
  },
  collection(app: PlayerApp): void {
    const body = open(app, 'Collection');
    for (const category of ['achievements', 'gallery', 'endings'] as CollectionCategory[]) {
      body.append(app.element('h3', app.tr(`${category[0].toUpperCase()}${category.slice(1)}`)));
      const grid = app.element('div', null, { className: 'collection-grid' });
      for (const item of app.data.program.progress[category]) {
        const unlocked = app.profile[category].includes(item.id);
        const figure = app.element('figure');
        if (unlocked && item.image) {
          const image = app.element('img', null, {
            src: app.asset(item.image),
            alt: item.title,
            loading: 'lazy',
          });
          figure.append(image);
          image.onclick = () => {
            const preview = open(app, item.title);
            const full = app.element('img', null, { src: image.src, alt: item.title });
            full.style.width = '100%';
            preview.append(
              full,
              button(app, 'Back', () => panels.collection(app)),
            );
          };
        }
        figure.append(app.element('figcaption', unlocked ? item.title : app.tr('Locked')));
        grid.append(figure);
      }
      body.append(grid);
    }
  },
  settings(app: PlayerApp): void {
    const body = open(app, 'Settings');
    for (const [key, text, type, min, max, step] of [
      ['fontSize', 'Text size', 'range', 14, 52, 1],
      ['music_volume', 'Music volume', 'range', 0, 1, 0.05],
      ['sound_volume', 'Sound volume', 'range', 0, 1, 0.05],
      ['voice_volume', 'Voice volume', 'range', 0, 1, 0.05],
      ['text_speed', 'Text speed', 'range', 10, 100, 5],
      ['auto_delay', 'Auto-play delay', 'range', 0.5, 10, 0.5],
      ['auto', 'Auto advance', 'checkbox'],
      ['wait_voice', 'Wait for voice', 'checkbox'],
      ['contrast', 'High contrast', 'checkbox'],
      ['reduced', 'Reduced motion', 'checkbox'],
      ['self_voicing', 'Self voicing', 'checkbox'],
    ] as SettingRow[]) {
      const row = app.element('label', null, { className: 'setting' });
      row.append(app.element('span', app.tr(text)));
      const input = app.element('input', null, { type });
      if (type === 'checkbox') input.checked = Boolean(app.settings[key]);
      else {
        input.min = String(min);
        input.max = String(max);
        input.step = String(step);
        input.value = String(app.settings[key]);
      }
      input.oninput = app.guard(() => {
        Object.assign(app.settings, {
          [key]: type === 'checkbox' ? input.checked : Number(input.value),
        });
        app.applySettings();
      });
      row.append(input);
      body.append(row);
    }
    const row = app.element('label', null, { className: 'setting' });
    row.append(app.element('span', app.tr('Language')));
    const select = app.element('select', null, { ariaLabel: app.tr('Language') });
    select.append(app.element('option', app.tr('Source text'), { value: '' }));
    for (const catalog of app.data.catalogs)
      select.append(app.element('option', catalog.language, { value: catalog.language }));
    select.value = app.settings.language;
    select.onchange = app.guard(async () => {
      await app.setLanguage(select.value);
      panels.settings(app);
    });
    row.append(select);
    body.append(row);
  },
};

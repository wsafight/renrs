import type { Dialogue, PlayerApp, PlayerSettings, SettingKey } from './types';

export function richText(
  node: HTMLElement,
  dialogue?: Dialogue | null,
  visible = Infinity,
): (count: number) => void {
  node.dir = 'auto';
  node.replaceChildren();
  const updates: Array<(count: number) => void> = [];
  let offset = 0;
  for (const run of dialogue?.runs?.length
    ? dialogue.runs
    : [{ text: dialogue?.text || '', style: {} }]) {
    const span = document.createElement(run.style.bold ? 'strong' : 'span');
    const characters = Array.from(run.text),
      start = offset;
    offset += characters.length;
    let annotation: HTMLElement | undefined;
    if (Number.isFinite(visible)) {
      const shown = document.createTextNode(''),
        hidden = document.createElement('span');
      hidden.style.visibility = 'hidden';
      hidden.setAttribute('aria-hidden', 'true');
      span.append(shown, hidden);
      let previous = -1;
      updates.push((count: number) => {
        const length = Math.min(characters.length, Math.max(0, count - start));
        if (length === previous) return;
        previous = length;
        shown.data = characters.slice(0, length).join('');
        hidden.textContent = characters.slice(length).join('');
        if (annotation) annotation.style.visibility = length === characters.length ? '' : 'hidden';
      });
    } else span.textContent = run.text;
    if (run.style.color) span.style.color = run.style.color;
    if (run.style.underline) span.style.textDecoration = 'underline';
    if (run.style.ruby) {
      const ruby = document.createElement('ruby');
      annotation = document.createElement('rt');
      annotation.textContent = run.style.ruby;
      ruby.append(span, annotation);
      node.append(ruby);
    } else node.append(span);
  }
  const update = (count: number) => {
    node.dataset.visibleCharacters = String(Math.min(offset, count));
    node.dataset.revealing = String(count < offset);
    for (const change of updates) change(count);
  };
  update(visible);
  return update;
}

export function setupPresentation(app: PlayerApp): void {
  const theme = app.data.theme || {};
  for (const [property, field] of Object.entries({
    '--accent': 'accent_color',
    '--focus': 'focus_color',
    '--text': 'text_color',
    '--muted': 'muted_text_color',
    '--panel': 'panel_color',
    '--surface': 'surface_color',
    '--background': 'background_color',
    '--danger': 'danger_color',
  } satisfies Record<string, keyof typeof theme>)) {
    const value = theme[field];
    if (typeof value === 'string') document.documentElement.style.setProperty(property, value);
  }
  const fonts = [theme.font_path, ...(theme.font_fallbacks || [])].filter((path): path is string =>
    Boolean(path),
  );
  Promise.all(
    fonts.map(async (path, index) => {
      const name = `ProjectFont${index}`;
      const face = await new FontFace(name, `url(${JSON.stringify(app.asset(path))})`).load();
      document.fonts.add(face);
      return name;
    }),
  )
    .then((names) => {
      document.body.style.fontFamily = [...names, 'system-ui', 'sans-serif'].join(', ');
    })
    .catch(app.notify);
  app.tr = (text: string) => {
    const translated = app.engine?.translate_ui(text) ?? text;
    return translated !== text
      ? translated
      : app.settings.language.startsWith('zh')
        ? app.data.ui_zh?.[text] || text
        : text;
  };
  app.localize = (root: ParentNode) => {
    document.documentElement.lang = app.settings.language || 'en';
    for (const node of root.querySelectorAll<HTMLElement>('[data-ui]')) {
      if (!node.dataset.ui) continue;
      const translated = app.tr(node.dataset.ui);
      if (node.dataset.uiAttribute) {
        for (const attribute of node.dataset.uiAttribute.split(' '))
          node.setAttribute(attribute, translated);
      } else node.textContent = translated;
    }
  };
  for (const node of document.querySelectorAll<HTMLElement>('[aria-label]')) {
    node.dataset.ui = node.getAttribute('aria-label') ?? '';
    node.dataset.uiAttribute = node.hasAttribute('title') ? 'title aria-label' : 'aria-label';
  }
  app.localize(document);
  let lastSpoken = '';
  app.speak = (text?: string | null) => {
    if (!app.settings.self_voicing || !('speechSynthesis' in window)) return;
    if (!text || text === lastSpoken) return;
    lastSpoken = text;
    speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text);
    utterance.lang = app.settings.language || document.documentElement.lang;
    speechSynthesis.speak(utterance);
  };
  document.addEventListener('focusin', (event) => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const label = target instanceof HTMLInputElement ? target.labels?.[0]?.textContent : null;
    app.speak(target.getAttribute('aria-label') || label || target.textContent);
  });
}

export function audioVolume(settings: PlayerSettings, channel: string, relative = 1): number {
  const key: SettingKey =
    channel === 'music' ? 'music_volume' : channel === 'voice' ? 'voice_volume' : 'sound_volume';
  return (settings[key] ?? settings.volume ?? 0.8) * relative;
}

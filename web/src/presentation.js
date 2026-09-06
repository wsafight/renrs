export function richText(node, dialogue, visible = Infinity) {
  node.dir = 'auto';
  node.replaceChildren();
  const updates = []; let offset = 0;
  for (const run of dialogue?.runs?.length ? dialogue.runs : [{text: dialogue?.text || '', style: {}}]) {
    const span = document.createElement(run.style.bold ? 'strong' : 'span');
    const characters = Array.from(run.text), start = offset; offset += characters.length;
    let annotation;
    if (Number.isFinite(visible)) {
      const shown = document.createTextNode(''), hidden = document.createElement('span');
      hidden.style.visibility = 'hidden'; hidden.setAttribute('aria-hidden', 'true');
      span.append(shown, hidden);
      let previous = -1;
      updates.push(count => {
        const length = Math.min(characters.length, Math.max(0, count - start));
        if (length === previous) return; previous = length;
        shown.data = characters.slice(0, length).join(''); hidden.textContent = characters.slice(length).join('');
        if (annotation) annotation.style.visibility = length === characters.length ? '' : 'hidden';
      });
    } else span.textContent = run.text;
    if (run.style.color) span.style.color = run.style.color;
    if (run.style.underline) span.style.textDecoration = 'underline';
    if (run.style.ruby) {
      const ruby = document.createElement('ruby'); annotation = document.createElement('rt');
      annotation.textContent = run.style.ruby; ruby.append(span, annotation); node.append(ruby);
    } else node.append(span);
  }
  const update = count => {
    node.dataset.visibleCharacters = String(Math.min(offset, count));
    node.dataset.revealing = String(count < offset);
    for (const change of updates) change(count);
  };
  update(visible); return update;
}

export function setupPresentation(app) {
  const theme = app.data.theme || {};
  for (const [property, field] of Object.entries({
    '--accent': 'accent_color', '--focus': 'focus_color', '--text': 'text_color',
    '--muted': 'muted_text_color', '--panel': 'panel_color', '--surface': 'surface_color',
    '--background': 'background_color', '--danger': 'danger_color',
  })) if (theme[field]) document.documentElement.style.setProperty(property, theme[field]);
  const fonts = [theme.font_path, ...(theme.font_fallbacks || [])].filter(Boolean);
  Promise.all(fonts.map(async (path, index) => {
    const name = `ProjectFont${index}`;
    const face = await new FontFace(name, `url(${JSON.stringify(app.asset(path))})`).load();
    document.fonts.add(face); return name;
  })).then(names => { document.body.style.fontFamily = [...names, 'system-ui', 'sans-serif'].join(', '); }).catch(app.notify);
  app.tr = text => {
    const translated = app.engine?.translate_ui(text) ?? text;
    return translated !== text ? translated : app.settings.language.startsWith('zh') ? app.data.ui_zh?.[text] || text : text;
  };
  app.localize = root => {
    document.documentElement.lang = app.settings.language || 'en';
    for (const node of root.querySelectorAll('[data-ui]')) {
      const translated = app.tr(node.dataset.ui);
      if (node.dataset.uiAttribute) {
        for (const attribute of node.dataset.uiAttribute.split(' ')) node.setAttribute(attribute, translated);
      } else node.textContent = translated;
    }
  };
  for (const node of document.querySelectorAll('[aria-label]')) {
    node.dataset.ui = node.getAttribute('aria-label');
    node.dataset.uiAttribute = node.hasAttribute('title') ? 'title aria-label' : 'aria-label';
  }
  app.localize(document);
  let lastSpoken = '';
  app.speak = text => {
    if (!app.settings.self_voicing || !('speechSynthesis' in window)) return;
    if (!text || text === lastSpoken) return;
    lastSpoken = text; speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance(text); utterance.lang = app.settings.language || document.documentElement.lang;
    speechSynthesis.speak(utterance);
  };
  document.addEventListener('focusin', event => app.speak(event.target.getAttribute('aria-label') || event.target.labels?.[0]?.textContent || event.target.textContent));
}

export function audioVolume(settings, channel) {
  const key = channel === 'music' ? 'music_volume' : channel === 'voice' ? 'voice_volume' : 'sound_volume';
  return settings[key] ?? settings.volume ?? 0.8;
}

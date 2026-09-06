export function viewportParent(app, kind, root, item, viewports) {
  let host = root, origin = {x: 0, y: 0, width: 1280, height: 720};
  for (const frame of item.viewports || []) {
    let saved = viewports.get(frame.id);
    if (!saved) {
      const node = app.element('div', null, {className: 'custom-viewport', tabIndex: 0});
      node.setAttribute('aria-label', frame.id);
      const content = app.element('div', null, {className: 'custom-viewport-content'});
      Object.assign(node.style, coordinates(frame.bounds, origin));
      content.style.height = `${frame.content_height / frame.bounds.height * 100}%`;
      node.append(content); host.append(node);
      app.viewportScroll ||= new Map();
      const key = `${kind}:${frame.id}`;
      requestAnimationFrame(() => { node.scrollTop = app.viewportScroll.get(key) || 0; });
      node.onscroll = () => app.viewportScroll.set(key, node.scrollTop);
      saved = {node, content}; viewports.set(frame.id, saved);
    }
    host = saved.content;
    origin = {...frame.bounds, height: frame.content_height};
  }
  return {host, position: coordinates(item.bounds, origin)};
}

function coordinates(bounds, origin) {
  return {left: `${(bounds.x - origin.x) / origin.width * 100}%`, top: `${(bounds.y - origin.y) / origin.height * 100}%`,
    width: `${bounds.width / origin.width * 100}%`, height: `${bounds.height / origin.height * 100}%`};
}

export async function dataControl(app, kind, widget, node, text) {
  const update = async (variable, expression) => {
    app.state = JSON.parse(app.engine.apply_expression(variable, expression)); app.syncProfile();
    if (app.$('modal').open) await app.openPanel(kind); else await app.render();
  };
  const enabled = app.state.waiting === 'Dialogue' || !!app.state.waiting?.Choice;
  if (widget.type === 'extension') {
    const control = app.element('button', text); control.disabled = !enabled;
    control.onclick = app.guard(async () => {
      app.state = JSON.parse(app.engine.apply_extension(widget.variable, widget.name, widget.input)); app.syncProfile();
      if (app.$('modal').open) await app.openPanel(kind); else await app.render();
    });
    node.append(control); return;
  }
  if (widget.type === 'data_list') {
    const values = JSON.parse(app.engine.screen_value(widget.variable));
    if (!Array.isArray(values)) throw new Error(`Expected list: ${widget.variable}`);
    node.classList.add('custom-list');
    const pageSize = 50, pages = Math.max(1, Math.ceil(values.length / pageSize));
    const show = page => {
      node.replaceChildren();
      values.slice(page * pageSize, (page + 1) * pageSize).forEach((value, offset) => {
        const label = widget.label && value && typeof value === 'object' ? value[widget.label] : value;
        const control = app.element('button', typeof label === 'string' ? label : JSON.stringify(label));
        control.disabled = !enabled;
        control.onclick = app.guard(() => update(widget.selected, `get(${widget.variable}, ${page * pageSize + offset})`));
        node.append(control);
      });
      if (pages > 1) {
        const bar = app.element('nav', null, {className: 'pagination'});
        for (const [label, delta] of [['<', -1], ['>', 1]]) {
          const control = app.element('button', label); control.ariaLabel = app.tr(delta < 0 ? 'Previous' : 'Next');
          control.disabled = page + delta < 0 || page + delta >= pages;
          control.onclick = () => show(page + delta); bar.append(control);
        }
        node.append(bar);
      }
    };
    show(0); return;
  }
  const control = app.element('button', text); control.disabled = !enabled;
  if (widget.type === 'drag') {
    control.draggable = true;
    const select = () => { app.dragExpression = widget.expression; control.setAttribute('aria-pressed', 'true'); };
    control.onclick = select;
    control.ondragstart = event => { select(); event.dataTransfer.setData('text/renrs', 'internal'); };
    control.onpointerdown = event => {
      select();
      if (event.pointerType === 'touch') control.setPointerCapture(event.pointerId);
    };
    control.onpointerup = app.guard(async event => {
      if (event.pointerType !== 'touch') return;
      const target = document.elementFromPoint(event.clientX, event.clientY)?.closest('[data-drop-variable]');
      if (target) { const expression = app.dragExpression; app.dragExpression = null; await update(target.dataset.dropVariable, expression); }
    });
  } else {
    control.dataset.dropVariable = widget.variable;
    const drop = app.guard(async event => {
      event.preventDefault();
      const expression = app.dragExpression;
      if (!expression) return;
      app.dragExpression = null; await update(widget.variable, expression);
    });
    control.onclick = drop; control.ondrop = drop;
    control.ondragover = event => { if (app.dragExpression) event.preventDefault(); };
  }
  node.append(control);
}

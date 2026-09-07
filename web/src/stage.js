const dimensions = new Map();
async function size(url) {
  if (!dimensions.has(url)) {
    const image = new Image(); image.src = url;
    dimensions.set(url, image.decode().then(() => ({width: image.naturalWidth, height: image.naturalHeight})).catch(error => { dimensions.delete(url); throw error; }));
  }
  return dimensions.get(url);
}
const xPosition = (position, width) => position === 'Left' ? 84 : position === 'Right' ? 1280 - width - 84 : (1280 - width) / 2;
const ease = (name, x) => name === 'EaseIn' ? x * x : name === 'EaseOut' ? 1 - (1 - x) ** 2 : name === 'EaseInOut' ? x < .5 ? 2 * x * x : 1 - (-2 * x + 2) ** 2 / 2 : x;
function interpolate(from, to, progress) {
  const value = {...to};
  for (const key of ['x', 'y', 'scale', 'rotation', 'alpha', 'anchor_x', 'anchor_y']) value[key] = from[key] + (to[key] - from[key]) * progress;
  if (from.crop && to.crop) value.crop = Object.fromEntries(Object.keys(from.crop).map(key => [key, from.crop[key] + (to.crop[key] - from.crop[key]) * progress]));
  else value.crop = progress < 1 ? from.crop : to.crop;
  return value;
}
const cameraStyle = camera => ({transform: `translate(${camera?.x || 0}px, ${camera?.y || 0}px) rotate(${-camera?.rotation || 0}deg) scale(${camera?.scale || 1})`, transformOrigin: '640px 360px', opacity: camera?.alpha ?? 1});
const spriteOrder = (a, b) => (a.display_order - b.display_order) || (a.display_layer < b.display_layer ? -1 : a.display_layer > b.display_layer ? 1 : 0) || (a.layer - b.layer);
function animateCamera(node, camera, effect, elapsed, frames) {
  Object.assign(node.style, cameraStyle(camera));
  let keys;
  if (frames?.length) keys = frames.map((frame, index) => ({...cameraStyle(frame), offset: index / 60}));
  else if (effect?.Transform?.alias === 'camera') {
    const animation = effect.Transform;
    keys = Array.from({length: 61}, (_, index) => ({...cameraStyle(interpolate(animation.from, animation.to, ease(animation.easing, index / 60))), offset: index / 60}));
  }
  if (keys) node.animate(keys, {duration: (effect.Parallel || effect.Transform).seconds * 1000, fill: 'forwards'}).currentTime = elapsed;
}
function geometry(sprite, transform, source, position) {
  const crop = transform.crop || {...source, x: 0, y: 0};
  const height = 650 * transform.scale, width = crop.width / crop.height * height;
  const x = (position ?? xPosition(sprite.position, width)) + transform.x - transform.anchor_x * width;
  return {
    outer: {left: `${x}px`, top: `${720 + transform.y - transform.anchor_y * height}px`, width: `${width}px`, height: `${height}px`, opacity: transform.alpha,
      transform: `rotate(${transform.rotation}deg)`, transformOrigin: `${transform.anchor_x * 100}% ${transform.anchor_y * 100}%`},
    inner: {width: `${source.width / crop.width * width}px`, height: `${source.height / crop.height * height}px`, left: `${-crop.x / crop.width * width}px`, top: `${-crop.y / crop.height * height}px`}
  };
}
async function spriteNode(app, sprite, effect, elapsed, parallel) {
  const node = app.element('div', null, {className: 'sprite'});
  const image = sprite.composition ? await layerNode(app, sprite) : app.element('img', null, {src: app.asset(sprite.path), alt: sprite.alias});
  node.append(image); node.dataset.alias = sprite.alias;
  const source = sprite.composition || await size(image.src);
  const final = geometry(sprite, sprite.transform, source);
  Object.assign(node.style, final.outer); Object.assign(image.style, final.inner);
  if (!app.settings.reduced && parallel) {
    const frames = parallel.frames.map((sprites, index) => ({...geometry(sprite, sprites.find(item => item.alias === sprite.alias).transform, source).outer, offset: index / 60}));
    const imageFrames = parallel.frames.map((sprites, index) => ({...geometry(sprite, sprites.find(item => item.alias === sprite.alias).transform, source).inner, offset: index / 60}));
    for (const [target, keys] of [[node, frames], [image, imageFrames]]) target.animate(keys, {duration: parallel.seconds * 1000, fill: 'forwards'}).currentTime = elapsed;
  } else if (!app.settings.reduced && effect?.alias === sprite.alias) {
    const frames = [], imageFrames = [];
    for (let index = 0; index <= 60; index++) {
      const t = index / 60, progress = ease(effect.easing, t);
      const transform = typeof effect.from === 'object' ? interpolate(effect.from, effect.to, progress) : sprite.transform;
      const width = (transform.crop || source).width / (transform.crop || source).height * 650 * transform.scale;
      const position = typeof effect.from === 'string' ? xPosition(effect.from, width) + (xPosition(effect.to, width) - xPosition(effect.from, width)) * t : undefined;
      const frame = geometry(sprite, transform, source, position);
      frames.push({...frame.outer, offset: t}); imageFrames.push({...frame.inner, offset: t});
    }
    for (const [target, keys] of [[node, frames], [image, imageFrames]]) target.animate(keys, {duration: effect.seconds * 1000, fill: 'forwards'}).currentTime = elapsed;
  }
  return node;
}
export async function renderStage(app, stage, elapsed = 0) {
  app.layerNodes = [];
  const effect = app.state.waiting?.Effect?.effect;
  const root = app.$('stage');
  for (const animation of root.getAnimations({subtree: true})) animation.cancel();
  root.querySelectorAll('.old-stage').forEach(node => node.remove());
  const background = app.$('background');
  background.style.objectFit = effect?.Video ? 'contain' : 'cover';
  if (stage.background) { background.src = app.asset(stage.background); background.hidden = false; } else background.hidden = true;
  const sprites = app.$('sprites');
  sprites.hidden = !!effect?.Video;
  const parallel = effect?.Parallel && {frames: JSON.parse(app.engine.animation_frames()), seconds: effect.Parallel.seconds};
  sprites.replaceChildren(...await Promise.all([...stage.sprites].sort(spriteOrder).map(sprite => spriteNode(app, sprite, effect?.Transform || effect?.Tween, elapsed, parallel))));
  startLayerClock(app);
  const cameraFrames = !app.settings.reduced && effect?.Parallel ? JSON.parse(app.engine.camera_frames()) : null;
  for (const node of [background, sprites]) animateCamera(node, stage.camera, app.settings.reduced ? null : effect, elapsed, cameraFrames);
  if (app.settings.reduced) return;
  if (effect?.Fade) root.animate([{opacity: 0}, {opacity: 1}], {duration: effect.Fade.seconds * 1000}).currentTime = elapsed;
  if (effect?.Dissolve) {
    const old = app.element('div', null, {className: 'old-stage'});
    Object.assign(old.style, cameraStyle(effect.Dissolve.from.camera));
    if (effect.Dissolve.from.background) old.append(app.element('img', null, {className: 'stage-background', src: app.asset(effect.Dissolve.from.background), alt: ''}));
    old.append(...await Promise.all([...effect.Dissolve.from.sprites].sort(spriteOrder).map(sprite => spriteNode(app, sprite, null, 0))));
    root.append(old);
    const animation = old.animate([{opacity: 1}, {opacity: 0}], {duration: effect.Dissolve.seconds * 1000, fill: 'forwards'});
    animation.currentTime = elapsed; animation.onfinish = () => old.remove();
  }
}

async function layerNode(app, sprite) {
  const root = app.element('div', null, {className: 'sprite-layers'});
  for (const layer of sprite.composition.layers) {
    const image = app.element('img', null, {src: app.asset(layer.path), alt: sprite.alias});
    const updateSize = () => Object.assign(image.style, {
      left: `${layer.x / sprite.composition.width * 100}%`, top: `${layer.y / sprite.composition.height * 100}%`,
      width: `${image.naturalWidth / sprite.composition.width * 100}%`, height: `${image.naturalHeight / sprite.composition.height * 100}%`
    });
    await image.decode(); updateSize(); image.onload = updateSize;
    root.append(image); app.layerNodes.push({image, layer, path: layer.path});
  }
  return root;
}

function startLayerClock(app) {
  if (app.refreshLayerClock) { app.refreshLayerClock(); return; }
  app.layerTime ||= 0;
  let previous = performance.now(), timer, running = false;
  function tick() {
    clearTimeout(timer);
    const now = performance.now();
    if (running) app.layerTime += Math.min(250, now - previous);
    previous = now;
    const voice = app.voices.get('voice');
    const active = !document.hidden && !app.$('modal').open && !app.inTitle && !app.settings.reduced;
    let next = 250; running = false;
    for (const item of app.layerNodes || []) {
      const {layer, image} = item; let path = layer.path;
      if (!image.isConnected) continue;
      if (active && layer.frames.length && (!layer.speaking || voice && !voice.paused && !voice.ended)) {
        running = true;
        const duration = layer.frames.reduce((sum, frame) => sum + frame.seconds, 0);
        let elapsed = app.layerTime / 1000 % duration;
        for (const frame of layer.frames) { if (elapsed < frame.seconds) {path = frame.path; next = Math.min(next, (frame.seconds - elapsed) * 1000); break;} elapsed -= frame.seconds; }
      }
      if (path !== item.path) {item.path = path; image.src = app.asset(path);}
    }
    if (running) timer = setTimeout(tick, Math.max(8, next));
  }
  app.refreshLayerClock = tick;
  document.addEventListener('visibilitychange', tick);
  new MutationObserver(tick).observe(app.$('modal'), {attributes: true, attributeFilter: ['open']});
  tick();
}

export function resizeStage(app) {
  const game = app.$('game'), root = app.$('stage');
  const resize = () => {
    const scale = Math.min(game.clientWidth / 1280, game.clientHeight / 720);
    Object.assign(root.style, {width: '1280px', height: '720px', left: `${(game.clientWidth - 1280 * scale) / 2}px`, top: `${matchMedia('(max-width: 760px)').matches ? 0 : (game.clientHeight - 720 * scale) / 2}px`, transform: `scale(${scale})`, transformOrigin: 'top left'});
    game.style.setProperty('--stage-height', `${720 * scale}px`);
  };
  new ResizeObserver(resize).observe(game); resize();
}

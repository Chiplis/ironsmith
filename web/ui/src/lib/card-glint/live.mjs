import {cardGlintAsset,loadCardGlint} from './session.mjs';
import {CARD_GLINT_FINISHES,CARD_PRINT_EFFECTS,normalizeCardPresentation,presentCardGlintFace} from './printing-choices.mjs';

// card-glint's WebGL renderer for finishes and print effects. session.mjs
// remains the static Canvas2D path.
let live;
export function loadCardGlintLive() {
  if (!live) live = Promise.all([
    loadCardGlint(),
    import(/* @vite-ignore */ cardGlintAsset('browser.js').href),
    import(/* @vite-ignore */ cardGlintAsset('magic.js').href),
  ]).then(([,browser,magic]) => ({browser,magic}))
    .catch(error => {live = undefined; throw error;});
  return live;
}

// Local unit quad → clip space for a canvas the size of one inspected face.
const QUAD = [2,0,0,0,0,-2,0,0,0,0,1,0,-1,1,0,1];
export const CARD_GLINT_TILT = Object.freeze({x:.3,y:.38});

// One renderer per canvas. It keeps drawing the last prepared revision until
// the next is ready, so edits never flash an empty card.
export async function createCardGlintLiveView(canvas,{onReady,onError}={}) {
  const {browser,magic} = await loadCardGlintLive();
  canvas.width = 732; canvas.height = 1020;
  let pointer = null, settings = normalizeCardPresentation(), revision = 0, faceKey = '';
  let frame = 0, last = 0, elapsed = 0, visible = true, disposed = false, controller;
  const reduced = matchMedia('(prefers-reduced-motion: reduce)');
  const finish = () => CARD_GLINT_FINISHES.find(option => option.id === settings.finish);
  const paint = () => {
    if (disposed || !revision) return;
    const at = pointer ?? {x:.5,y:.5};
    // Match the host's CSS tilt so the light follows the visible card.
    const rx = (at.y-.5)*CARD_GLINT_TILT.x, ry = (at.x-.5)*CARD_GLINT_TILT.y;
    renderer.updateScene([{id:1,card:1,finish:settings.finish,transform:QUAD,
      normal:[Math.sin(ry),-Math.sin(rx)*Math.cos(ry),Math.cos(rx)*Math.cos(ry)],
      ...(settings.printEffects ? CARD_PRINT_EFFECTS : {})}]);
    renderer.render({viewProjection:browser.IDENTITY,pointer:[at.x,at.y],time:elapsed,
      hover:pointer ? {instance:1,uv:[Math.min(1,Math.max(0,at.x)),Math.min(1,Math.max(0,at.y))]} : undefined});
  };
  // Only animated finishes need a clock; others repaint on pointer input.
  const wake = () => {
    const animated = Boolean(finish()?.tone) && visible && !document.hidden && !reduced.matches && !disposed;
    if (!animated) {cancelAnimationFrame(frame); frame = 0; last = 0; return;}
    if (!frame) frame = requestAnimationFrame(tick);
  };
  function tick(now) {
    frame = 0;
    if (!last || now-last >= 32) {elapsed += last ? Math.min(.1,(now-last)/1000) : 0; last = now; paint();}
    wake();
  }
  const observer = new IntersectionObserver(entries => {visible = entries.at(-1).isIntersecting; wake();});
  observer.observe(canvas);
  document.addEventListener('visibilitychange',wake); reduced.addEventListener('change',wake);
  const unlisten = () => {observer.disconnect(); document.removeEventListener('visibilitychange',wake); reduced.removeEventListener('change',wake);};
  let renderer;
  try {
    renderer = await browser.createRenderer({canvas,wasmUrl:cardGlintAsset('renderer_bg.wasm'),etching:'rust',invalidate:paint,onError});
  } catch (error) {unlisten(); throw error;}
  return {
    // Finish and print effects are instance options and never re-prepare the face.
    update(face,presentation) {
      settings = normalizeCardPresentation(presentation);
      const tone = finish()?.tone ?? null, key = JSON.stringify([face,tone]);
      if (key !== faceKey) {
        faceKey = key; revision++;
        controller?.abort(); controller = new AbortController();
        const current = revision, presented = presentCardGlintFace(magic,face);
        renderer.upsertCards([tone ? magic.opticalDocument(1,revision,presented,tone)
          : {id:1,revision,recipe:magic.magic.describe(presented,'inspect')}]);
        renderer.ready(1,current,controller.signal).then(() => {
          if (!disposed && current === revision) {paint(); onReady?.(face);}
        },error => {if (!disposed && current === revision && error?.name !== 'AbortError') onError?.(error);});
      }
      paint(); wake();
    },
    pointer(value) {pointer = value; paint();},
    dispose() {
      disposed = true; controller?.abort(); cancelAnimationFrame(frame); unlisten();
      renderer.dispose(); canvas.width = canvas.height = 0;
    },
  };
}

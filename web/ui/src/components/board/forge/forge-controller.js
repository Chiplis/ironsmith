import { FORGE_MEASURE_SELECTOR, createForgeMeasurer } from './measure-forge.js';
import { settleForgeZones } from '../../../lib/forge-board-layout.js';

export function mountForgeBoard(root, host, scene, initialSignal) {
  const measure = createForgeMeasurer(root);
  const media = window.matchMedia('(prefers-reduced-motion: reduce)');
  let reduced = media.matches || !scene;
  let zones = new Map(), displayed = new Map(), layout, signal = initialSignal;
  let dirty = true, reset = true, locked = false, frame = 0, lastMeasure = -Infinity, lastDraw = -Infinity;
  let disposed = false, contextLost = false, flightActive = false, lastSignal = initialSignal;
  const observed = new Set();
  const resize = new ResizeObserver(() => { dirty = true; wake(); });
  const mutations = new MutationObserver(records => {
    if (records.some(record => !host.contains(record.target))) { dirty = true; wake(); }
  });
  mutations.observe(root, { childList: true, subtree: true, attributes: true, attributeFilter: ['class', 'style', 'hidden', 'data-zone-id', 'data-arena-owner', 'data-zone-owner', 'data-zone-anchor-player', 'data-player-drop-target'] });
  resize.observe(root);
  function syncObserved() {
    const current = new Set(root.querySelectorAll(FORGE_MEASURE_SELECTOR));
    for (const el of observed) if (!current.has(el)) { resize.unobserve(el); observed.delete(el); }
    for (const el of current) if (!observed.has(el)) { resize.observe(el); observed.add(el); }
  }
  function wake() { if (!disposed && !contextLost && !frame && !document.hidden) frame = requestAnimationFrame(tick); }
  function tick(now) {
    frame = 0;
    if (disposed || document.hidden) return;
    const settling = [...zones.values()].some(zone => zone.removing || zone.rect !== zone.requested);
    if ((dirty || settling) && now - lastMeasure >= 100) {
      const nextLayout = measure();
      const resized = !layout || nextLayout.width !== layout.width || nextLayout.height !== layout.height;
      layout = nextLayout;
      flightActive = Boolean(root.querySelector('.zone-move-effects-layer, .game-fx-layer'));
      zones = settleForgeZones(zones, layout.zones, now, { reset: reset || resized, locked: locked || flightActive });
      if (reset || resized) displayed = new Map(zones);
      reset = false; dirty = false; lastMeasure = now;
      syncObserved();
    }
    let moving = false;
    let drew = false;
    if (layout && now - lastDraw >= 1000 / 30) {
      const next = new Map();
      for (const [key, zone] of zones) {
        const from = displayed.get(key)?.rect || zone.rect;
        const rect = {};
        const alpha = reduced ? 1 : 1 - Math.exp(-Math.min(now - lastDraw, 100) / 100);
        for (const edge of ['left', 'top', 'right', 'bottom']) {
          const delta = zone.rect[edge] - from[edge];
          rect[edge] = Math.abs(delta) < 0.5 ? zone.rect[edge] : from[edge] + delta * alpha;
          if (Math.abs(delta) >= 0.5) moving = true;
        }
        next.set(key, { ...zone, rect });
      }
      displayed = next;
      // Ornaments retreat immediately when any card, hand, or control needs
      // their space. They never participate in DOM layout or pointer capture.
      scene?.layout(layout, displayed);
      scene?.signal(signal, lastSignal.turn !== signal.turn && !reset, now);
      lastSignal = signal;
      try { scene?.draw(now, reduced); }
      catch {
        scene?.dispose(); scene = null; reduced = true;
        host.dataset.renderer = 'fallback';
      }
      lastDraw = now;
      drew = true;
      host.dataset.zoneCount = String(zones.size);
      host.dataset.motion = reduced ? 'reduced' : 'animated';

    }
    const pending = [...zones.values()].some(zone => zone.removing || zone.rect !== zone.requested);
    if (!reduced || dirty || (pending && !locked && !flightActive) || moving || !drew) wake();
  }
  function onVisibility() {
    if (document.hidden) { cancelAnimationFrame(frame); frame = 0; }
    else { reset = true; dirty = true; wake(); }
  }
  function onMotion() { reduced = media.matches || !scene; dirty = true; wake(); }
  function onContextLost() { contextLost = true; cancelAnimationFrame(frame); frame = 0; }
  function onContextRestored() { contextLost = false; reset = true; dirty = true; wake(); }
  function onScroll() { dirty = true; wake(); }
  function onClick(event) {
    if (locked || event.target.closest('button, a, input, select, .game-card, .battlefield-row-card, [role="button"]')) return;
    const r = root.getBoundingClientRect();
    const x = event.clientX - r.left, y = event.clientY - r.top;
    if (scene?.interact?.(x, y, performance.now())) wake();
  }
  document.addEventListener('visibilitychange', onVisibility);
  media.addEventListener('change', onMotion);
  root.addEventListener('scroll', onScroll, true);
  root.addEventListener('click', onClick);
  window.addEventListener('resize', onScroll);
  host.addEventListener('forgeassetsready', onScroll);
  host.addEventListener('webglcontextlost', onContextLost, true);
  host.addEventListener('webglcontextrestored', onContextRestored, true);
  wake();
  return {
    update(nextSignal, interactionLocked) { signal = nextSignal; locked = interactionLocked; dirty = true; wake(); },
    dispose() {
      disposed = true; cancelAnimationFrame(frame); resize.disconnect(); mutations.disconnect();
      document.removeEventListener('visibilitychange', onVisibility); media.removeEventListener('change', onMotion);
      root.removeEventListener('scroll', onScroll, true); root.removeEventListener('click', onClick);
      host.removeEventListener('forgeassetsready', onScroll);
      window.removeEventListener('resize', onScroll); host.removeEventListener('webglcontextlost', onContextLost, true);
      host.removeEventListener('webglcontextrestored', onContextRestored, true); scene?.dispose();
    },
  };
}

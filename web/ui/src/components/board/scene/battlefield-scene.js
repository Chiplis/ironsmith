import { OrthographicCamera, Scene } from 'three';
import { CSS3DObject, CSS3DRenderer } from 'three/addons/renderers/CSS3DRenderer.js';
import { advanceBattlefieldPose, battlefieldSceneCellAtPoint, fitBattlefieldScene,
  geometryForScale, battlefieldPlacementOutline, BATTLEFIELD_GROW_DELAY, BATTLEFIELD_GROW_MARGIN } from '../../../lib/battlefield-scene-layout.js';

/** Three owns transforms; React owns the contents of each fixed-size object.
 * CSS3D keeps the existing accessible card faces and nested controls intact.
 * Only the field viewport is observed, never card or text dimensions. */
export function createBattlefieldScene(root, registry) {
  // Legacy theme selectors contain !important grid/padding rules. The scene
  // owns its viewport; reserve insets in the solver, not in both CSS and math.
  const overrides = { display: 'block', padding: '0px', position: 'relative', 'overflow-x': 'auto', 'overflow-y': 'auto', 'min-height': '0px',
    // Scrollbars must never subtract space from the viewport being solved.
    // Crowded boards retain wheel/touch/focus scrolling without native gutters.
    'scrollbar-width': 'none', 'scrollbar-gutter': 'auto', 'clip-path': 'none' };
  const oldStyles = Object.keys(overrides).map(key => [key, root.style.getPropertyValue(key), root.style.getPropertyPriority(key)]);
  for (const [key, value] of Object.entries(overrides)) root.style.setProperty(key, value, 'important');
  const scene = new Scene();
  const camera = new OrthographicCamera(0, 1, 0, -1, .1, 2000);
  camera.position.z = 1000;
  const renderer = new CSS3DRenderer();
  renderer.domElement.className = 'battlefield-scene';
  for (const [key, value] of Object.entries({ position: 'absolute', top: '0px', left: '0px', margin: '0px', overflow: 'hidden', contain: 'size layout paint' })) renderer.domElement.style.setProperty(key, value, 'important');
  root.append(renderer.domElement);
  const objects = new Map();
  const media = matchMedia('(prefers-reduced-motion: reduce)');
  let config, geometry, viewport, growTimer, frame = 0, lastTime = 0, disposed = false;
  let growCandidate = null, targetScale = null, leftInset = 0, reservedStackWidth = 0, stackResizeFrame = 0;
  const sceneOptions = () => ({ ...config.options, leftInset });
  function sizeRenderer(width, height) {
    const size = renderer.getSize();
    if (size.width === width && size.height === height) return;
    renderer.setSize(width, height);
    camera.right = width; camera.bottom = -height; camera.updateProjectionMatrix();
  }
  const onFitted = () => window.dispatchEvent(new Event('ironsmith:battlefield-layout-fitted'));
  function wake() {
    if (!disposed && !frame && !document.hidden) frame = requestAnimationFrame(tick);
  }
  function syncObjects() {
    for (const [key, entry] of objects) if (!registry.has(key)) {
      scene.remove(entry.object); objects.delete(key);
    }
    for (const [key, element] of registry) if (!objects.has(key)) {
      const object = new CSS3DObject(element);
      object.element.style.pointerEvents = 'none';
      scene.add(object);
      objects.set(key, { object, current: null });
    }
  }
  function solve(allowGrowth = false) {
    if (!config || !viewport?.width || !viewport?.height) return;
    const wanted = fitBattlefieldScene(config.layout, viewport, sceneOptions());
    let scale = wanted.scale;
    if (targetScale != null && scale > targetScale && !allowGrowth) {
      const candidate = `${config.layout.signature}:${viewport.width}:${viewport.height}:${scale}`;
      if (scale >= targetScale * BATTLEFIELD_GROW_MARGIN && candidate !== growCandidate) {
        clearTimeout(growTimer);
        growCandidate = candidate;
        growTimer = setTimeout(() => { growCandidate = null; solve(true); }, BATTLEFIELD_GROW_DELAY);
      } else if (scale < targetScale * BATTLEFIELD_GROW_MARGIN) {
        clearTimeout(growTimer); growCandidate = null;
      }
      scale = targetScale;
    } else {
      clearTimeout(growTimer); growCandidate = null;
    }
    targetScale = scale;
    geometry = geometryForScale(config.layout, viewport, { ...sceneOptions(), gap: wanted.gap }, scale);
    const size = renderer.getSize();
    // Retain outgoing scroll space until the transition ends. Shrinking the
    // content box now would clamp scrollTop in the middle of the animation.
    sizeRenderer(Math.max(size.width || 0, geometry.width), Math.max(size.height || 0, geometry.height));
    // Compatibility metadata is not inherited by card faces: their dimensions
    // are fixed on the scene objects, so these values cannot cause card reflow.
    root.style.setProperty('--bf-card-width', `${geometry.cardWidth}px`);
    root.style.setProperty('--bf-card-height', `${geometry.cardHeight}px`);
    root.style.setProperty('--bf-cols', String(config.layout.maxCols));
    root.style.setProperty('--bf-rows', String(config.layout.rowCount));
    root.dataset.sceneScale = String(scale);
    syncObjects();
    draw(0, true);
    wake();
  }
  function draw(elapsed, initializeOnly = false) {
    if (!geometry) return false;
    let moving = false;
    for (const [key, entry] of objects) {
      const target = geometry.poses.get(key);
      entry.object.visible = Boolean(target);
      if (!target) continue;
      // Newly mounted objects occupy their reserved destination immediately;
      // the existing card arrival effect remains local to that object.
      if (!initializeOnly || !entry.current) entry.current = advanceBattlefieldPose(entry.current, target, elapsed, media.matches);
      const p = entry.current;
      entry.object.element.style.setProperty('--scene-layer', String(Math.max(1, 20 - (p.depth || 0))));
      moving ||= p.x !== target.x || p.y !== target.y || p.scale !== target.scale;
      entry.object.position.set(p.x + config.options.baseWidth * p.scale / 2,
        -(p.y + config.options.baseHeight * p.scale / 2), 0);
      entry.object.scale.setScalar(p.scale);
    }
    renderer.render(scene, camera);
    root.dataset.sceneMoving = String(moving);
    // Placement previews use the same geometry as the scene, outside CSS grid.
    for (const node of root.querySelectorAll('[data-battlefield-drop-slot]')) {
      let pose = geometry.cells.get(`${node.dataset.row}:${node.dataset.column}`);
      const preview = config.placementPreview;
      if (preview?.automatic) {
        pose = geometry.poses.get(config.layout.visualKeyById.get('placement-preview'));
      } else if (pose && preview?.compact && preview.card) {
        // A move preview includes the compact land packing at its destination.
        const id = String(preview.card.id);
        const positions = new Map(config.layout.gridPositionById);
        positions.set(id, { row: Number(node.dataset.row), column: Number(node.dataset.column) });
        const moved = geometryForScale({ ...config.layout, gridPositionById: positions }, viewport,
          { ...sceneOptions(), gap: geometry.gap }, geometry.scale);
        pose = moved.poses.get(config.layout.visualKeyById.get(id)) || pose;
      }
      const p = pose && battlefieldPlacementOutline(pose, preview?.card, preview?.compact);
      if (p) Object.assign(node.style, { position: 'absolute', left: `${p.x}px`, top: `${p.y}px`,
        width: `${p.width}px`, minWidth: '0', height: `${p.height}px`, minHeight: '0', borderRadius: p.borderRadius });
    }
    return moving;
  }
  function tick(now) {
    frame = 0;
    const moving = draw(lastTime ? now - lastTime : 16);
    if (!moving && geometry) {
      sizeRenderer(geometry.width, geometry.height);
      renderer.render(scene, camera);
    }
    lastTime = now;
    onFitted();
    if (moving) wake();
    else lastTime = 0;
  }
  function resize() {
    const next = { width: root.clientWidth, height: root.clientHeight };
    const shell = root.closest('[data-my-zone] .my-zone-board-shell');
    const dock = shell?.querySelector(':scope > .my-zone-stack-rail');
    const dockRect = dock?.getBoundingClientRect();
    const pileWidth = Number(dock?.dataset.stackLaneWidth) || 0;
    const stackActive = shell?.classList.contains('my-zone-board-shell--with-stack') || pileWidth > 0;
    // Keep one reservation for the entire nonempty Stack. Ordering swaps the
    // renderer, not the battlefield's available space. Entry counts cannot
    // resize this reservation; release it only after the final exit.
    if (stackActive) {
      const initialWidth = Math.max(240, Math.min(360, window.innerWidth * .24));
      reservedStackWidth ||= Math.max(initialWidth, pileWidth);
      shell.style.setProperty('--stack-reserved-width', `${reservedStackWidth}px`);
    } else {
      reservedStackWidth = 0;
      shell?.style.removeProperty('--stack-reserved-width');
    }
    const laneWidth = reservedStackWidth;
    const inset = laneWidth > 0
      ? Math.max(0, (dockRect?.left || 0) + laneWidth + 12 - root.getBoundingClientRect().left)
      : 0;
    if (viewport?.width === next.width && viewport?.height === next.height && leftInset === inset) return;
    leftInset = inset;
    // Horizontal scrolling on a crowded field must not expose cards beneath
    // the reserved lane. The clip belongs to the viewport, not its content.
    root.style.setProperty('clip-path', inset > 0 ? `inset(0 0 0 ${inset}px)` : 'none', 'important');
    viewport = next;
    // Viewport growth is intentional; hysteresis is for changing card counts.
    solve(true);
  }
  const observer = new ResizeObserver(resize);
  observer.observe(root);
  const stackShell = root.closest('[data-my-zone] .my-zone-board-shell');
  const stackDock = stackShell?.querySelector(':scope > .my-zone-stack-rail');
  if (stackDock) observer.observe(stackDock);
  // Renderer teardown/setup happens in the same React commit. Measure after
  // both complete so the transient missing pile does not release the lane.
  function scheduleStackResize() {
    cancelAnimationFrame(stackResizeFrame);
    stackResizeFrame = requestAnimationFrame(() => { stackResizeFrame = 0; resize(); });
  }
  const stackObserver = new MutationObserver(scheduleStackResize);
  if (stackShell) stackObserver.observe(stackShell, { attributes: true, attributeFilter: ['class'], childList: true });
  window.addEventListener('ironsmith:stack-lane-changed', scheduleStackResize);
  window.addEventListener('resize', resize);
  function visibility() { if (document.hidden) { cancelAnimationFrame(frame); frame = 0; lastTime = 0; } else wake(); }
  function motion() { wake(); }
  document.addEventListener('visibilitychange', visibility);
  media.addEventListener('change', motion);
  return {
    configure(next) { config = next; resize(); solve(); },
    sync() { syncObjects(); draw(0, true); wake(); },
    cellAt(x, y, allowOccupied) {
      // An occupied card follows its displayed transform throughout animation.
      for (const entry of objects.values()) {
        const p = entry.current;
        if (p && x >= p.x && x <= p.x + (p.faceWidth || config.options.baseWidth) * p.scale
          && y >= p.y && y <= p.y + (p.faceHeight || config.options.baseHeight) * p.scale) {
          return allowOccupied ? { row: p.row, column: p.column } : null;
        }
      }
      const cell = battlefieldSceneCellAtPoint(geometry, x, y);
      if (!allowOccupied && cell && [...config.layout.gridPositionById.values()]
        .some(position => position.row === cell.row && position.column === cell.column)) return null;
      return cell;
    },
    bounds() {
      return [...objects.values()].filter(entry => entry.current && entry.object.visible).map(({ current: p }) => ({
        left: p.x - (config.options.paddingX || 0) * p.scale, top: p.y - (config.options.paddingY || 0) * p.scale,
        right: p.x + ((p.faceWidth || config.options.baseWidth) + (config.options.paddingX || 0)) * p.scale,
        bottom: p.y + ((p.faceHeight || config.options.baseHeight) + (config.options.paddingY || 0)) * p.scale,
      }));
    },
    positions() {
      const result = new Map(), rect = root.getBoundingClientRect();
      for (const card of config?.layout.orderedCards || []) {
        const entry = objects.get(config.layout.visualKeyById.get(String(card.id))), p = entry?.current;
        if (!p) continue;
        const image = entry.object.element.querySelector('img');
        const position = { left: p.x, top: p.y,
          viewportLeft: rect.left + p.x - root.scrollLeft, viewportTop: rect.top + p.y - root.scrollTop,
          width: (p.faceWidth || config.options.baseWidth) * p.scale,
          height: (p.faceHeight || config.options.baseHeight) * p.scale,
          sourceImageUrl: image?.currentSrc || image?.src || null };
        for (const id of card.member_stable_ids?.length ? card.member_stable_ids : [card.stable_id ?? card.id]) result.set(String(id), position);
      }
      return result;
    },
    dispose() {
      disposed = true; cancelAnimationFrame(stackResizeFrame); stackShell?.style.removeProperty('--stack-reserved-width'); cancelAnimationFrame(frame); clearTimeout(growTimer); observer.disconnect(); stackObserver.disconnect();
      window.removeEventListener('ironsmith:stack-lane-changed', scheduleStackResize);
      window.removeEventListener('resize', resize);
      document.removeEventListener('visibilitychange', visibility); media.removeEventListener('change', motion);
      for (const entry of objects.values()) scene.remove(entry.object);
      objects.clear(); renderer.domElement.remove();
      for (const [key, value, priority] of oldStyles) value ? root.style.setProperty(key, value, priority) : root.style.removeProperty(key);
    },
  };
}

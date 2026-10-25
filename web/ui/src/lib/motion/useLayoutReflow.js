import { useLayoutEffect, useRef } from 'react';
import { uiSpring } from './anime';
import { compositorEase } from './compositor';

// FLIP measures only when the list changes. Native transform animations keep
// real entries in normal flow; cancellation cannot strand absolute children.
export default function useLayoutReflow(rootRef, signature, options = {}) {
  const { children = '.game-card', disabled = false, duration = 360, bounce = 0.14 } = options;
  const recordsRef = useRef(new Map());
  const runningRef = useRef([]);
  const paramsRef = useRef(options);
  useLayoutEffect(() => { paramsRef.current = options; });

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    for (const cleanup of runningRef.current) cleanup();
    runningRef.current = [];
    const nodes = [...root.querySelectorAll(children)];
    const previous = recordsRef.current;
    const next = new Map();
    const { delay, enterFrom } = paramsRef.current;
    const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const ease = compositorEase(uiSpring({ duration, bounce }).ease);
    const start = (node, keyframes, index) => {
      const origin = node.style.transformOrigin;
      node.style.transformOrigin = '0 0';
      const motion = node.animate(keyframes, {
        duration, delay: typeof delay === 'function' ? delay(node, index, nodes.length) : delay || 0,
        easing: ease, fill: 'both', composite: 'add',
      });
      const cleanup = () => {
        motion.cancel();
        node.style.transformOrigin = origin;
      };
      motion.onfinish = cleanup;
      runningRef.current.push(cleanup);
    };
    // Read every final box before any transform-origin/animation writes. A
    // write followed by the next child's read would flush layout per child.
    const measured = nodes.map(node => ({ node, rect: node.getBoundingClientRect() }));
    for (const [index, {node, rect}] of measured.entries()) {
      next.set(node, { rect });
      if (disabled || reduced || !previous.size || !rect.width || !rect.height) continue;
      const old = previous.get(node)?.rect;
      if (old) {
        const dx = old.left - rect.left, dy = old.top - rect.top;
        const sx = old.width / rect.width, sy = old.height / rect.height;
        if (Math.abs(dx) + Math.abs(dy) + Math.abs(sx - 1) + Math.abs(sy - 1) < 0.01) continue;
        start(node, [{ transform: `translate(${dx}px, ${dy}px) scale(${sx}, ${sy})` }, { transform: 'translate(0px, 0px) scale(1, 1)' }], index);
      } else if (enterFrom) {
        // Opacity is replace-composited; transforms add to the entry's CSS.
        start(node, [
          { transform: `translate(${enterFrom.x || 0}px, ${enterFrom.y || 0}px) scale(${enterFrom.scale ?? 1})`, opacity: enterFrom.opacity ?? 0, composite: 'replace' },
          { transform: 'translate(0px, 0px) scale(1)', opacity: 1, composite: 'replace' },
        ], index);
      }
    }
    // Stack entries own their exit animation and remain mounted until it
    // finishes. Cloning removed entries here would replay that exit and leave
    // stale visual cards after their occurrence has already resolved.
    recordsRef.current = next;
    return () => {
      for (const cleanup of runningRef.current) cleanup();
      runningRef.current = [];
    };
  }, [rootRef, signature, children, disabled, duration, bounce]);
}

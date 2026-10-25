import { useLayoutEffect, useRef } from "react";

const selectors = [".topbar-phase-status", ".table-shared-player-header", "[data-human-action-dock]"];
const duration = 240;

/** FLIP the HUD's actual layout changes, including intrinsic sizes and anchors.
 * Layout is committed immediately; only the visual presentation interpolates.
 * This keeps collision calculations tied to the final layout.
 */
export default function useDecisionControlMotion(rootRef) {
  const positions = useRef(new Map());

  useLayoutEffect(() => {
    const root = rootRef.current;
    if (!root) return;
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    let frame = 0;
    const update = () => {
      frame = 0;
      // Sample interrupted poses together, then cancel together. Starting an
      // animation for one HUD node must not invalidate the next node's read.
      const samples = selectors.map(selector => {
        const node = root.querySelector(selector);
        const previous = positions.current.get(selector);
        const progress = previous?.animation?.effect?.getComputedTiming().progress ?? 1;
        const visualStyle = previous?.animation && node ? getComputedStyle(node) : null;
        return { selector, node, previous, progress,
          translation: visualStyle?.translate.split(" ").map(parseFloat) || [0, 0],
          scale: visualStyle?.scale.split(" ").map(parseFloat) || [1, 1],
          animationTime: previous?.animation?.currentTime };
      });
      for (const { previous } of samples) previous?.animation?.cancel();
      for (const sample of samples) {
        if (!sample.node) continue;
        sample.rect = sample.node.getBoundingClientRect();
        sample.visible = sample.rect.width > 0 && sample.rect.height > 0 && getComputedStyle(sample.node).visibility !== "hidden";
      }
      for (const { selector, node, previous, progress, translation, scale, animationTime, rect, visible } of samples) {
        if (!node) { positions.current.delete(selector); continue; }
        if (previous?.node === node && visible === previous.visible && !preference.matches
          && Math.abs(previous.x - rect.x) < .1 && Math.abs(previous.y - rect.y) < .1
          && Math.abs(previous.width - rect.width) < .1 && Math.abs(previous.height - rect.height) < .1) {
          if (animationTime != null && progress < 1) {
            previous.animation.play();
            previous.animation.currentTime = animationTime;
          }
          continue;
        }
        const entry = { node, x: rect.x, y: rect.y, width: rect.width, height: rect.height, dx: 0, dy: 0, sx: 1, sy: 1, visible };
        if (previous?.visible && visible && !preference.matches) {
          entry.dx = previous.x + (translation[0] || 0) - rect.x;
          entry.dy = previous.y + (translation[1] || 0) - rect.y;
          entry.sx = previous.width * (scale[0] || 1) / rect.width;
          entry.sy = previous.height * (scale[1] || scale[0] || 1) / rect.height;
          const moved = Math.abs(entry.dx) > .5 || Math.abs(entry.dy) > .5;
          const resized = Math.abs(entry.sx - 1) > .005 || Math.abs(entry.sy - 1) > .005;
          if (moved || resized) {
            entry.animation = node.animate([
              { translate: `${entry.dx}px ${entry.dy}px`, scale: `${entry.sx} ${entry.sy}`, transformOrigin: "0 0" },
              { translate: "0px 0px", scale: "1 1", transformOrigin: "0 0" },
            ], { duration, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
          }
        }
        positions.current.set(selector, entry);
      }
    };
    // Commit and observer notifications share one measurement pass before
    // paint instead of repeatedly flushing layout during the same update.
    const schedule = () => { if (!frame) frame = requestAnimationFrame(update); };
    schedule();
    const resize = new ResizeObserver(schedule);
    const mutations = new MutationObserver(schedule);
    resize.observe(root);
    for (const node of selectors.map(selector => root.querySelector(selector)).filter(Boolean)) {
      resize.observe(node);
      mutations.observe(node, { attributes: true, attributeFilter: ["style", "class"], childList: true, subtree: true, characterData: true });
    }
    preference.addEventListener("change", schedule);
    return () => { cancelAnimationFrame(frame); resize.disconnect(); mutations.disconnect(); preference.removeEventListener("change", schedule); };
  });

  useLayoutEffect(() => () => {
    for (const entry of positions.current.values()) entry.animation?.cancel();
    positions.current.clear();
  }, []);
}

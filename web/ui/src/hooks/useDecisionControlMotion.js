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
    const update = () => {
      for (const selector of selectors) {
        const node = root.querySelector(selector);
        const previous = positions.current.get(selector);
        if (!node) {
          previous?.animation?.cancel();
          positions.current.delete(selector);
          continue;
        }
        // Cancel our own transform before reading the final layout, otherwise
        // ResizeObserver/React updates would mistake animation for a new layout.
        const progress = previous?.animation?.effect?.getComputedTiming().progress ?? 1;
        // Timing progress is linear; read the actual eased visual position
        // so an interrupted animation cannot jump backwards.
        const visualStyle = previous?.animation ? getComputedStyle(node) : null;
        const translation = visualStyle?.translate.split(" ").map(parseFloat) || [0, 0];
        const scale = visualStyle?.scale.split(" ").map(parseFloat) || [1, 1];
        const animationTime = previous?.animation?.currentTime;
        previous?.animation?.cancel();
        const rect = node.getBoundingClientRect();
        const visible = rect.width > 0 && rect.height > 0 && getComputedStyle(node).visibility !== "hidden";
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
    update();
    const resize = new ResizeObserver(update);
    const mutations = new MutationObserver(update);
    resize.observe(root);
    for (const { node } of positions.current.values()) {
      resize.observe(node);
      mutations.observe(node, { attributes: true, attributeFilter: ["style", "class"], childList: true, subtree: true, characterData: true });
    }
    preference.addEventListener("change", update);
    return () => { resize.disconnect(); mutations.disconnect(); preference.removeEventListener("change", update); };
  });

  useLayoutEffect(() => () => {
    for (const entry of positions.current.values()) entry.animation?.cancel();
    positions.current.clear();
  }, []);
}

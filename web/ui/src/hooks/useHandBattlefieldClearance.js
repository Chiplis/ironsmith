import { useLayoutEffect } from "react";

const GAP = 12;
const FOOTPRINTS = ".game-card, .battlefield-row-card, .battlefield-counter-rail, .battlefield-group-badge, .battlefield-token-side-badge";

// Keep resting fans outside occupied battlefield space: ours below, opponents above. Hover, keyboard
// inspection, and explicitly opening the hand retain their floating behavior.
export default function useHandBattlefieldClearance(handRef, layoutKey, suspended, placement = "below") {
  useLayoutEffect(() => {
    const above = placement === "above";
    const shell = above ? handRef.current : handRef.current?.closest(".hand-reveal-shell");
    if (!shell || suspended) return undefined;
    const doc = shell.ownerDocument;
    const view = doc.defaultView;
    const scope = above ? shell.closest("[data-opponent-zone]") || doc : doc;
    let offset = 0;
    let frame = 0;
    let trackUntil = 0;
    const previousTranslate = shell.style.translate;

    const rectOf = element => {
      if (!element.checkVisibility({ checkVisibilityCSS: true, checkOpacity: true })) return null;
      const rect = element.getBoundingClientRect();
      return rect.width && rect.height ? rect : null;
    };
    const measure = () => {
      frame = 0;
      const obstacles = [];
      for (const field of scope.querySelectorAll(".battlefield-row")) {
        const rect = rectOf(field);
        if (!rect) continue;
        if (field.battlefieldScene) {
          for (const bounds of field.battlefieldScene.bounds()) {
            const clipped = {
              left: rect.left + Math.max(0, bounds.left - field.scrollLeft),
              right: rect.left + Math.min(field.clientWidth, bounds.right - field.scrollLeft),
              top: rect.top + Math.max(0, bounds.top - field.scrollTop),
              bottom: rect.top + Math.min(field.clientHeight, bounds.bottom - field.scrollTop),
            };
            if (clipped.left < clipped.right && clipped.top < clipped.bottom) obstacles.push(clipped);
          }
        } else {
          for (const element of field.querySelectorAll(FOOTPRINTS)) {
            const bounds = rectOf(element);
            if (bounds) obstacles.push(bounds);
          }
        }
      }
      for (const element of scope.querySelectorAll(".zone-pile-slot, .battlefield-overlay-zone")) {
        if (shell.contains(element)) continue;
        const bounds = rectOf(element);
        if (bounds) obstacles.push(bounds);
      }
      let next = 0;
      for (const card of shell.querySelectorAll(above ? ".opponent-hand-card" : ".game-card.hand-card")) {
        const rect = rectOf(card);
        if (!rect) continue;
        // Remove our own translation before solving, avoiding feedback that
        // would otherwise alternate between overlapping and clear frames.
        const top = rect.top - offset;
        const bottom = rect.bottom - offset;
        for (const obstacle of obstacles) {
          if (rect.left < obstacle.right + GAP && rect.right > obstacle.left - GAP) {
            if (above && top < obstacle.bottom + GAP) {
              next = Math.min(next, obstacle.top - GAP - bottom);
            } else if (!above && bottom > obstacle.top - GAP) {
              next = Math.max(next, obstacle.bottom + GAP - top);
            }
          }
        }
      }
      next = above ? Math.min(0, Math.floor(next)) : Math.max(0, Math.ceil(next));
      if (next !== offset) {
        offset = next;
        shell.style.translate = offset ? `0 ${offset}px` : previousTranslate;
      }
      if (view.performance.now() < trackUntil) frame = view.requestAnimationFrame(measure);
    };
    const schedule = () => {
      trackUntil = view.performance.now() + 500;
      if (!frame) frame = view.requestAnimationFrame(measure);
    };
    const mutations = new MutationObserver(records => {
      // Our translation writes must not keep the measurement loop alive.
      if (records.some(record => record.target !== shell || record.attributeName !== "style")) schedule();
    });
    mutations.observe(doc.body, { subtree: true, childList: true, attributes: true, attributeFilter: ["class", "style", "data-open"] });
    const resize = new ResizeObserver(schedule);
    resize.observe(doc.documentElement);
    resize.observe(shell);
    view.addEventListener("resize", schedule);
    doc.addEventListener("scroll", schedule, true);
    view.addEventListener("ironsmith:battlefield-layout-fitted", schedule);
    schedule();
    return () => {
      view.cancelAnimationFrame(frame);
      mutations.disconnect();
      resize.disconnect();
      view.removeEventListener("resize", schedule);
      doc.removeEventListener("scroll", schedule, true);
      view.removeEventListener("ironsmith:battlefield-layout-fitted", schedule);
      shell.style.translate = previousTranslate;
    };
  }, [handRef, layoutKey, suspended, placement]);
}

import { useLayoutEffect, useRef } from "react";

const PHASE_TRACK = ".topbar-phase-shell .phase-track:not(.phase-track--combat)";

// The panel never cuts a phase in half: when its left edge lands inside a
// phase cell, a cover of the same surface (its ::before) reaches back to that
// cell's left edge. The cover never changes the panel's own layout, so
// measuring and writing it cannot feed back into another resize.
function coverWholePhases(lane, panel) {
  const divider = lane.parentElement;
  if (!divider) return;
  const left = panel.getBoundingClientRect().left;
  const split = [...divider.querySelectorAll(`${PHASE_TRACK} .phase-track-cell`)]
    .map(cell => cell.getBoundingClientRect())
    .find(cell => cell.width > 0 && cell.left < left - 0.5 && cell.right > left + 0.5);
  const cover = split ? `${Math.ceil(left - split.left)}px` : "";
  if (panel.style.getPropertyValue("--phase-cover") === cover) return;
  if (cover) {
    panel.style.setProperty("--phase-cover", cover);
    panel.dataset.phaseCover = "true";
  } else {
    panel.style.removeProperty("--phase-cover");
    delete panel.dataset.phaseCover;
  }
}

// Payment lies on the middle strip's band (see usePhaseStripBand), pinned to
// its right end.
export default function DesktopManaPaymentStrip({ children }) {
  const laneRef = useRef(null);
  const panelRef = useRef(null);
  useLayoutEffect(() => {
    const lane = laneRef.current;
    const panel = panelRef.current;
    if (!lane || !panel) return undefined;
    const update = () => coverWholePhases(lane, panel);
    const resize = new ResizeObserver(update);
    const observeAll = () => {
      resize.disconnect();
      // The panel's own intrinsic width can change while its content's box
      // does not (late fonts), so it is watched too.
      resize.observe(lane);
      resize.observe(panel);
      for (const child of panel.children) resize.observe(child);
      // The shell can widen and slide the track along without the track
      // itself changing size.
      for (const phases of lane.parentElement?.querySelectorAll(`.topbar-phase-shell, ${PHASE_TRACK}`) || []) {
        resize.observe(phases);
      }
    };
    // The panel's content swaps (loading notice, then the plan) under it.
    const mutations = new MutationObserver(() => { observeAll(); update(); });
    mutations.observe(panel, { childList: true });
    observeAll();
    update();
    // Phase cells move without resizing when their labels' font arrives.
    document.fonts?.addEventListener?.("loadingdone", update);
    return () => {
      resize.disconnect();
      mutations.disconnect();
      document.fonts?.removeEventListener?.("loadingdone", update);
    };
  }, []);
  return (
    <div ref={laneRef} className="desktop-mana-payment" data-mana-payment="true">
      {/* One panel per payment, so it rolls in once even as its content
          changes from the loading notice to the plan. */}
      <div ref={panelRef} className="desktop-mana-payment-panel">{children}</div>
    </div>
  );
}

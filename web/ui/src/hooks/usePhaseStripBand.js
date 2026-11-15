import { useLayoutEffect } from "react";

// The middle strip is a solid band across the whole divider at the height of
// the phase track's main row. The track's vertical position comes from several
// layout rules (duel padding, wide-table sizing, multiplayer sequences), so it
// is measured and published on the divider as --phase-strip-top/height. The
// combat row only reserves space below the sequence, so it is not included.
const PHASE_TRACKS = ".topbar-phase-shell .phase-track:not(.phase-track--combat)";

function phaseTrackBand(divider) {
  const origin = divider.getBoundingClientRect();
  let top = Infinity, bottom = -Infinity;
  for (const track of divider.querySelectorAll(PHASE_TRACKS)) {
    const rect = track.getBoundingClientRect();
    if (!rect.width || !rect.height) continue;
    top = Math.min(top, rect.top);
    bottom = Math.max(bottom, rect.bottom);
  }
  return Number.isFinite(top) ? { top: top - origin.top, height: bottom - top } : null;
}

export default function usePhaseStripBand(dividerRef, enabled) {
  useLayoutEffect(() => {
    const divider = dividerRef.current;
    if (!enabled || !divider) return undefined;
    const measure = () => {
      const band = phaseTrackBand(divider);
      if (!band) {
        divider.removeAttribute("data-phase-strip");
        return;
      }
      divider.style.setProperty("--phase-strip-top", `${band.top}px`);
      divider.style.setProperty("--phase-strip-height", `${band.height}px`);
      divider.setAttribute("data-phase-strip", "true");
    };
    measure();
    const observer = typeof ResizeObserver === "function" ? new ResizeObserver(measure) : null;
    if (observer) {
      observer.observe(divider);
      for (const track of divider.querySelectorAll(PHASE_TRACKS)) observer.observe(track);
    }
    window.addEventListener("resize", measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener("resize", measure);
    };
  }, [dividerRef, enabled]);
}

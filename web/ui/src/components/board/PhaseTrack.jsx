import { useRef, useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { PHASE_TRACK, normalizePhaseStep } from "@/lib/constants";
import { cn } from "@/lib/utils";

export default function PhaseTrack({ compact = false, showBrand = false }) {
  const { state } = useGame();
  const { t } = useI18n();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;
  const trackRef = useRef(null);
  const [indicator, setIndicator] = useState(null);
  const firstRender = useRef(true);

  // Recompute when the active phase or the track width changes. The middle
  // inspector deliberately contracts this strip, so the indicator geometry
  // cannot be tied to phase changes alone.
  useEffect(() => {
    if (!active || !trackRef.current) {
      const clearRafId = requestAnimationFrame(() => setIndicator(null));
      return () => cancelAnimationFrame(clearRafId);
    }

    const track = trackRef.current;
    let rafId = null;
    const measureIndicator = () => {
      rafId = null;
      const idx = PHASE_TRACK.indexOf(active);
      if (idx < 0) { setIndicator(null); return; }

      const cells = track.querySelectorAll(".phase-track-cell");
      const cell = cells[idx];
      const lastPhaseCell = cells[cells.length - 1];
      if (!cell) { setIndicator(null); return; }

      const trackRect = track.getBoundingClientRect();
      const cellRect = cell.getBoundingClientRect();
      const lastPhaseRect = lastPhaseCell?.getBoundingClientRect?.() || cellRect;
      const isFirst = firstRender.current;
      firstRender.current = false;
      const leftInset = cellRect.left - trackRect.left;
      const rightInset = lastPhaseRect.right - cellRect.right;

      setIndicator({
        left: idx === 0 ? 0 : leftInset,
        width: cellRect.width + (idx === 0 ? leftInset : 0) + (idx === PHASE_TRACK.length - 1 ? rightInset : 0),
        animate: !isFirst,
      });
    };
    const scheduleIndicatorMeasure = () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(measureIndicator);
    };

    scheduleIndicatorMeasure();
    const observer = new ResizeObserver(scheduleIndicatorMeasure);
    observer.observe(track);
    window.addEventListener("resize", scheduleIndicatorMeasure);

    return () => {
      if (rafId != null) cancelAnimationFrame(rafId);
      observer.disconnect();
      window.removeEventListener("resize", scheduleIndicatorMeasure);
    };
  }, [active]);

  return (
    <section
      ref={trackRef}
      className="phase-track grid gap-px min-h-[24px] relative overflow-hidden"
      data-compact={compact ? "true" : "false"}
      data-has-brand={showBrand ? "true" : "false"}
    >
      {/* Sliding glow indicator */}
      {indicator && (
        <div
          className="phase-track-indicator absolute top-0 bottom-0 z-0 pointer-events-none"
          style={{
            left: indicator.left,
            width: indicator.width,
            transition: indicator.animate
              ? "left 350ms cubic-bezier(0.4, 0, 0.2, 1), width 350ms cubic-bezier(0.4, 0, 0.2, 1)"
              : "none",
          }}
        />
      )}

      {PHASE_TRACK.map((name) => (
        <div
          key={name}
          aria-current={name === active ? "step" : undefined}
          data-phase-name={name}
          data-phase-active={name === active ? "true" : "false"}
          className={cn(
            "phase-track-cell relative z-[1] grid items-center justify-items-center text-[13px] uppercase tracking-wide font-semibold transition-colors duration-300",
            name === active
              ? "text-[#f3f9ff] font-bold"
              : "text-[#d7c8a8]"
          )}
        >
          {compact
            ? t(`game.trackCompact.${name}`, null, name)
            : t(`game.track.${name}`, null, name)}
        </div>
      ))}
      {showBrand ? (
        <h1 className="toolbar-brand phase-track-brand relative z-[2] m-0 whitespace-nowrap font-bold">
          Ironsmith
        </h1>
      ) : null}
    </section>
  );
}

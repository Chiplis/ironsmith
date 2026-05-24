import { useRef, useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import { PHASE_TRACK, normalizePhaseStep } from "@/lib/constants";
import { cn } from "@/lib/utils";

const COMPACT_PHASE_LABELS = {
  Untap: "Untap",
  Upkeep: "Upkeep",
  Draw: "Draw",
  Main: "Main",
  Combat: "Combat",
  Main2: "M2",
  End: "End",
  Cleanup: "Clean",
};

export default function PhaseTrack({ compact = false }) {
  const { state } = useGame();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;
  const trackRef = useRef(null);
  const [indicator, setIndicator] = useState(null);
  const prevActiveRef = useRef(null);
  const firstRender = useRef(true);

  // Compute indicator position when active phase changes
  useEffect(() => {
    if (!active || !trackRef.current) {
      setIndicator(null);
      return;
    }

    const track = trackRef.current;
    const idx = PHASE_TRACK.indexOf(active);
    if (idx < 0) { setIndicator(null); return; }

    const cell = track.querySelectorAll(".phase-track-cell")[idx];
    if (!cell) { setIndicator(null); return; }

    const trackRect = track.getBoundingClientRect();
    const cellRect = cell.getBoundingClientRect();

    const isFirst = firstRender.current;
    firstRender.current = false;

    const leftInset = cellRect.left - trackRect.left;
    const rightInset = trackRect.right - cellRect.right;

    setIndicator({
      left: idx === 0 ? 0 : leftInset,
      width: cellRect.width + (idx === 0 ? leftInset : 0) + (idx === PHASE_TRACK.length - 1 ? rightInset : 0),
      animate: !isFirst && prevActiveRef.current !== active,
    });

    prevActiveRef.current = active;
  }, [active]);

  return (
    <section
      ref={trackRef}
      className="phase-track grid grid-cols-8 gap-px min-h-[24px] relative overflow-hidden"
      data-compact={compact ? "true" : "false"}
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
          {compact ? (COMPACT_PHASE_LABELS[name] || name) : name}
        </div>
      ))}
    </section>
  );
}

import { useRef, useEffect, useState } from "react";
import { useGame } from "@/context/GameContext";
import { PHASE_TRACK, normalizePhaseStep } from "@/lib/constants";
import { cn } from "@/lib/utils";

export default function PhaseTrack() {
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

    const cell = track.children[idx + 1];
    if (!cell) { setIndicator(null); return; }

    const trackRect = track.getBoundingClientRect();
    const cellRect = cell.getBoundingClientRect();

    const isFirst = firstRender.current;
    firstRender.current = false;

    setIndicator({
      left: cellRect.left - trackRect.left,
      width: cellRect.width,
      animate: !isFirst && prevActiveRef.current !== active,
    });

    prevActiveRef.current = active;
  }, [active]);

  return (
    <section ref={trackRef} className="phase-track grid grid-cols-8 gap-px min-h-[24px] relative overflow-hidden">
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
          data-phase-active={name === active ? "true" : "false"}
          className={cn(
            "phase-track-cell relative z-[1] grid items-center justify-items-center text-[13px] uppercase tracking-wide font-semibold transition-colors duration-300",
            name === active
              ? "text-[#f3f9ff] font-bold"
              : "text-[#d7c8a8]"
          )}
        >
          {name}
        </div>
      ))}
    </section>
  );
}

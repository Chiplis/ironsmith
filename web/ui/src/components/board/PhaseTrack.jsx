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

    const cell = track.children[idx];
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
    <section ref={trackRef} className="phase-track bg-[#0e141d] grid grid-cols-8 gap-px min-h-[24px] rounded relative overflow-hidden">
      {/* Sliding glow indicator */}
      {indicator && (
        <div
          className="absolute top-0 bottom-0 z-0 rounded-sm pointer-events-none"
          style={{
            left: indicator.left,
            width: indicator.width,
            background: "linear-gradient(180deg, #4475a8, #2b4e73)",
            boxShadow: "0 0 12px 2px rgba(68,117,168,0.4), inset 0 1px 0 rgba(255,255,255,0.12)",
            transition: indicator.animate
              ? "left 350ms cubic-bezier(0.4, 0, 0.2, 1), width 350ms cubic-bezier(0.4, 0, 0.2, 1)"
              : "none",
          }}
        />
      )}

      {PHASE_TRACK.map((name) => (
        <div
          key={name}
          className={cn(
            "relative z-[1] grid items-center justify-items-center text-[13px] uppercase tracking-wide font-semibold transition-colors duration-300",
            name === active
              ? "text-[#f3f9ff] font-bold"
              : "text-[#96abc7]"
          )}
        >
          {name}
        </div>
      ))}
    </section>
  );
}

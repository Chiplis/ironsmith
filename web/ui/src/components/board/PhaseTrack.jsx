import { useGame } from "@/context/GameContext";
import { PHASE_TRACK, normalizePhaseStep } from "@/lib/constants";
import { cn } from "@/lib/utils";

export default function PhaseTrack() {
  const { state } = useGame();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;

  return (
    <section className="border border-[#42566f] bg-[#0e141d] grid grid-cols-8 gap-px min-h-[34px]">
      {PHASE_TRACK.map((name) => (
        <div
          key={name}
          className={cn(
            "grid items-center justify-items-center text-[12px] text-[#96abc7] border-r border-[#1d2a3b] uppercase tracking-wide",
            name === active && "text-[#f3f9ff] bg-gradient-to-b from-[#4475a8] to-[#2b4e73] font-bold"
          )}
        >
          {name}
        </div>
      ))}
    </section>
  );
}

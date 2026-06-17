import { useMemo } from "react";
import { Sparkles, Eye, BookOpen, Zap, Swords, Hourglass, MoonStar, Brush } from "lucide-react";
import { useGame } from "@/context/GameContext";
import { normalizePhaseStep } from "@/lib/constants";
import { useMobileBattle } from "@/context/MobileBattleContext";
import { cn } from "@/lib/utils";

const PHASE_CELLS = [
  { key: "Untap", label: "Untap", Icon: Sparkles },
  { key: "Upkeep", label: "Upkeep", Icon: Eye },
  { key: "Draw", label: "Draw", Icon: BookOpen },
  { key: "Main", label: "Pre-Main", Icon: Zap },
  { key: "Combat", label: "Combat", Icon: Swords },
  { key: "Main2", label: "Post-Main", Icon: Zap },
  { key: "End", label: "End", Icon: MoonStar },
  { key: "Cleanup", label: "Cleanup", Icon: Brush },
];

const COMPACT_PHASE_KEYS = new Set(["Untap", "Cleanup"]);

export default function MobilePhaseStrip({ className }) {
  const { state } = useGame();
  const { phaseStops, togglePhaseStop } = useMobileBattle();
  const activeKey = useMemo(
    () => normalizePhaseStep(state?.phase, state?.step),
    [state?.phase, state?.step]
  );

  return (
    <div
      className={cn("mobile-mtga-phase-strip", className)}
      role="group"
      aria-label="Turn phases"
    >
      {PHASE_CELLS.map((cell) => {
        const { key, label } = cell;
        const PhaseIcon = cell.Icon;
        const isActive = activeKey === key;
        const isStopped = phaseStops?.has?.(key);
        const isCompact = COMPACT_PHASE_KEYS.has(key);
        return (
          <button
            key={key}
            type="button"
            className={cn(
              "mobile-mtga-phase-cell",
              isActive && "mobile-mtga-phase-cell--active",
              isStopped && "mobile-mtga-phase-cell--stopped",
              isCompact && "mobile-mtga-phase-cell--compact"
            )}
            data-phase-key={key}
            aria-label={`${label}${isActive ? " (current)" : ""}${isStopped ? " (stop set)" : ""}`}
            aria-pressed={isStopped}
            onClick={() => togglePhaseStop?.(key)}
          >
            <PhaseIcon className="size-3" aria-hidden="true" />
            <span className="mobile-mtga-phase-cell-label">{label}</span>
            {isStopped ? (
              <span className="mobile-mtga-phase-cell-stop-dot" aria-hidden="true" />
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

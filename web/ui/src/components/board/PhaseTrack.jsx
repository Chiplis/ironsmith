import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { normalizePhaseStep } from "@/lib/constants";

export default function PhaseTrack({ compact = false }) {
  const { state } = useGame();
  const { t } = useI18n();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;
  const [transition, setTransition] = useState({ active, previous: null, revision: 0 });

  if (transition.active !== active) {
    setTransition({ active, previous: transition.active, revision: transition.revision + 1 });
  }

  const label = (phase) => phase
    ? t(`game.track.${phase}`, null, phase)
    : "—";

  return (
    <section className="phase-track phase-track--single" data-compact={compact ? "true" : "false"}>
      <div key={transition.revision} className="phase-track-window">
        {transition.previous && active ? (
          <div className="phase-track-label phase-track-label--outgoing" aria-hidden="true">
            {label(transition.previous)}
          </div>
        ) : null}
        <div
          className={`phase-track-label${transition.previous && active ? " phase-track-label--incoming" : ""}`}
          data-phase-name={active}
          aria-current={active ? "step" : undefined}
          aria-live="polite"
          aria-atomic="true"
          onAnimationEnd={() => setTransition((current) => (
            current.revision === transition.revision ? { ...current, previous: null } : current
          ))}
        >
          {label(active)}
        </div>
      </div>
    </section>
  );
}

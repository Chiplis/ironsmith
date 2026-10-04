import { useGame } from "@/context/GameContext";
import { useI18n } from "@/i18n/I18nContext";
import { normalizePhaseStep, PHASE_TRACK } from "@/lib/constants";

export default function PhaseTrack({ compact = false }) {
  const { state } = useGame();
  const { t } = useI18n();
  const active = state ? normalizePhaseStep(state.phase, state.step) : null;
  const activeIndex = PHASE_TRACK.indexOf(active);

  return (
    <nav
      className="phase-track phase-track--sequence"
      data-compact={compact ? "true" : "false"}
      aria-label={t("game.phaseTrack", null, "Turn phases")}
    >
      <ol className="phase-track-steps">
        {PHASE_TRACK.map((phase, index) => {
          const isActive = phase === active;
          const isComplete = activeIndex >= 0 && index < activeIndex;
          return (
            <li
              key={phase}
              className="phase-track-cell"
              data-phase-name={phase}
              data-phase-active={isActive ? "true" : "false"}
              data-phase-complete={isComplete ? "true" : "false"}
              aria-current={isActive ? "step" : undefined}
            >
              <span className="phase-track-step-marker" aria-hidden="true">
                {isComplete ? "✓" : index + 1}
              </span>
              <span className="phase-track-step-label">
                {t(`game.track.${phase}`, null, phase)}
              </span>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

import useUiText from "@/i18n/useUiText";
import {
  ChevronsLeft,
  ChevronsRight,
  Loader2,
  PauseCircle,
  Play,
  StepBack,
  StepForward,
} from "lucide-react";
import { useGame } from "@/context/GameContext";

function clampPosition(value, actionCount) {
  return Math.max(0, Math.min(Number(value) || 0, Number(actionCount) || 0));
}

export default function AuditReplayControls() {
  const ui = useUiText();
  const {
    auditReplay,
    beginAuditReplaySession,
    setAuditReplayPosition,
    exitAuditReplaySession,
  } = useGame();

  const available = Boolean(auditReplay?.available || auditReplay?.active);
  if (!available) return null;

  const active = Boolean(auditReplay?.active);
  const busy = Boolean(auditReplay?.busy);
  const actionCount = Number(auditReplay?.actionCount || 0);
  const position = clampPosition(auditReplay?.currentActionIndex, actionCount);
  const actionLabel = auditReplay?.currentActionLabel || "Match start";
  const sourceLabel = auditReplay?.sourceLabel || "Verified match";

  const startReplay = async () => {
    try {
      await beginAuditReplaySession();
    } catch {
      // GameContext publishes the actionable error.
    }
  };

  const moveReplay = async (nextPosition) => {
    try {
      await setAuditReplayPosition(clampPosition(nextPosition, actionCount));
    } catch {
      // GameContext publishes the actionable error.
    }
  };

  const exitReplay = async () => {
    try {
      await exitAuditReplaySession();
    } catch {
      // GameContext publishes the actionable error.
    }
  };

  return (
    <section className="audit-replay-rail" aria-label={ui("Match replay controls")}>
      <div className="audit-replay-rail-meta">
        <span>{ui("Replay")}</span>
        <strong title={ui(active ? actionLabel : sourceLabel)}>
          {active ? `${position}/${actionCount}` : ui("{0} actions", { 0: actionCount })}
        </strong>
      </div>

      {active ? (
        <div className="audit-replay-rail-label" title={ui(actionLabel)}>
          {ui(actionLabel)}
        </div>
      ) : null}

      <div className="audit-replay-rail-controls">
        {!active ? (
          <button
            type="button"
            className="stone-pill audit-replay-rail-button audit-replay-rail-button--wide"
            disabled={busy}
            onClick={() => void startReplay()}
            aria-label={ui("Start replay")}
            title={ui("Start replay")}
          >
            {busy ? (
              <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
            ) : (
              <Play className="size-3.5" aria-hidden="true" />
            )}{ui("Start")}</button>
        ) : (
          <>
            <button
              type="button"
              className="stone-icon-button audit-replay-rail-button"
              disabled={busy || position <= 0}
              onClick={() => void moveReplay(0)}
              aria-label={ui("Jump to replay start")}
              title={ui("Jump to start")}
            >
              <ChevronsLeft className="size-3.5" aria-hidden="true" />
            </button>
            <button
              type="button"
              className="stone-icon-button audit-replay-rail-button"
              disabled={busy || position <= 0}
              onClick={() => void moveReplay(position - 1)}
              aria-label={ui("Previous replay action")}
              title={ui("Previous action")}
            >
              <StepBack className="size-3.5" aria-hidden="true" />
            </button>
            <button
              type="button"
              className="stone-icon-button audit-replay-rail-button"
              disabled={busy || position >= actionCount}
              onClick={() => void moveReplay(position + 1)}
              aria-label={ui("Next replay action")}
              title={ui("Next action")}
            >
              {busy ? (
                <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
              ) : (
                <StepForward className="size-3.5" aria-hidden="true" />
              )}
            </button>
            <button
              type="button"
              className="stone-icon-button audit-replay-rail-button"
              disabled={busy || position >= actionCount}
              onClick={() => void moveReplay(actionCount)}
              aria-label={ui("Jump to replay end")}
              title={ui("Jump to end")}
            >
              <ChevronsRight className="size-3.5" aria-hidden="true" />
            </button>
            <button
              type="button"
              className="stone-pill audit-replay-rail-button audit-replay-rail-button--wide"
              disabled={busy}
              onClick={() => void exitReplay()}
              aria-label={ui("Exit replay")}
              title={ui("Exit replay")}
            >
              <PauseCircle className="size-3.5" aria-hidden="true" />{ui("Exit")}</button>
          </>
        )}
      </div>

      {auditReplay?.error ? (
        <div className="audit-replay-rail-error">{ui(auditReplay.error)}</div>
      ) : null}
    </section>
  );
}

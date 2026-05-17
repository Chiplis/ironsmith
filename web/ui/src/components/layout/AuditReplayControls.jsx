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
    <section className="audit-replay-rail" aria-label="Match replay controls">
      <div className="audit-replay-rail-meta">
        <span>Replay</span>
        <strong title={active ? actionLabel : sourceLabel}>
          {active ? `${position}/${actionCount}` : `${actionCount} actions`}
        </strong>
      </div>

      {active ? (
        <div className="audit-replay-rail-label" title={actionLabel}>
          {actionLabel}
        </div>
      ) : null}

      <div className="audit-replay-rail-controls">
        {!active ? (
          <button
            type="button"
            className="stone-pill audit-replay-rail-button audit-replay-rail-button--wide"
            disabled={busy}
            onClick={() => void startReplay()}
            aria-label="Start replay"
            title="Start replay"
          >
            {busy ? (
              <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
            ) : (
              <Play className="size-3.5" aria-hidden="true" />
            )}
            Start
          </button>
        ) : (
          <>
            <button
              type="button"
              className="stone-icon-button audit-replay-rail-button"
              disabled={busy || position <= 0}
              onClick={() => void moveReplay(0)}
              aria-label="Jump to replay start"
              title="Jump to start"
            >
              <ChevronsLeft className="size-3.5" aria-hidden="true" />
            </button>
            <button
              type="button"
              className="stone-icon-button audit-replay-rail-button"
              disabled={busy || position <= 0}
              onClick={() => void moveReplay(position - 1)}
              aria-label="Previous replay action"
              title="Previous action"
            >
              <StepBack className="size-3.5" aria-hidden="true" />
            </button>
            <button
              type="button"
              className="stone-icon-button audit-replay-rail-button"
              disabled={busy || position >= actionCount}
              onClick={() => void moveReplay(position + 1)}
              aria-label="Next replay action"
              title="Next action"
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
              aria-label="Jump to replay end"
              title="Jump to end"
            >
              <ChevronsRight className="size-3.5" aria-hidden="true" />
            </button>
            <button
              type="button"
              className="stone-pill audit-replay-rail-button audit-replay-rail-button--wide"
              disabled={busy}
              onClick={() => void exitReplay()}
              aria-label="Exit replay"
              title="Exit replay"
            >
              <PauseCircle className="size-3.5" aria-hidden="true" />
              Exit
            </button>
          </>
        )}
      </div>

      {auditReplay?.error ? (
        <div className="audit-replay-rail-error">{auditReplay.error}</div>
      ) : null}
    </section>
  );
}

import { cloneElement, isValidElement, useRef, useState } from "react";
import {
  AlertTriangle,
  CheckCircle2,
  ChevronsLeft,
  ChevronsRight,
  FileJson,
  Loader2,
  PauseCircle,
  Play,
  StepBack,
  StepForward,
  ShieldCheck,
  Upload,
} from "lucide-react";
import { useGame } from "@/context/GameContext";
import { Button } from "@/components/ui/button";
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { verifyLiveAuditTranscript } from "@/lib/multiplayer-audit";
import { useI18n } from "@/i18n/I18nContext";

const defaultTriggerClassName = "stone-pill table-zone-action-button inline-flex items-center justify-center rounded-none px-2.5 py-0.5 text-[13px] font-medium uppercase transition-all select-none hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45";

const emptyVerification = {
  phase: "idle",
  sourceLabel: "",
  summary: "",
  report: null,
  transcript: null,
  error: "",
};

function compactHash(value) {
  const text = String(value || "");
  if (text.length <= 18) return text || "None";
  return `${text.slice(0, 10)}...${text.slice(-8)}`;
}

function playerCountForTranscript(transcript) {
  return Array.isArray(transcript?.match?.players) ? transcript.match.players.length : 0;
}

function outcomeLabel(outcome = {}) {
  if (outcome.status === "winner") {
    return outcome.winnerName || `Player ${Number(outcome.winner) + 1} wins`;
  }
  if (outcome.status === "draw") return "Draw";
  if (outcome.status === "disputed") return "Disputed";
  return "Stalled";
}

function verificationFacts(report, transcript) {
  if (!report) return [];
  return [
    ["Match", compactHash(transcript?.matchId || transcript?.match?.auditMatchId || "")],
    ["Protocol", String(transcript?.protocolVersion || transcript?.match?.protocolVersion || "Unknown")],
    ["Players", String(playerCountForTranscript(transcript) || "Unknown")],
    ["Actions", String(Number(report.verifiedActions || 0))],
    ["Engine Replay", report.engineReplay?.verified
      ? `${Number(report.engineReplay.replayedActions || 0)} actions`
      : "Not run"],
    ["Outcome", outcomeLabel(report.outcome)],
    ["Final State", compactHash(report.finalStateHash)],
    ["Public Checkpoint", compactHash(report.finalPublicCheckpointHash)],
    ["Disputes", String(Array.isArray(report.disputes) ? report.disputes.length : 0)],
  ];
}

function replayCommandLabel(action, index) {
  if (!action) return "Match start";
  const command = action.command || action.audit?.command || {};
  const actor = Number(action.actorIndex ?? action.audit?.actor);
  const actorLabel = Number.isInteger(actor) ? `P${actor + 1}` : "Player";
  if (action.label) return `${index}. ${actorLabel}: ${action.label}`;
  if (command.type === "priority_action") {
    const kind = String(command.action_ref?.kind || "");
    if (kind === "cast_spell") return `${index}. ${actorLabel}: Cast spell`;
    if (kind === "play_land") return `${index}. ${actorLabel}: Play land`;
    if (kind === "pass_priority") return `${index}. ${actorLabel}: Pass priority`;
    if (kind === "keep_opening_hand") return `${index}. ${actorLabel}: Keep hand`;
    if (kind === "continue_pregame" || kind === "begin_game") return `${index}. ${actorLabel}: Pregame`;
    return `${index}. ${actorLabel}: ${kind || "Priority action"}`;
  }
  if (command.type === "select_targets") return `${index}. ${actorLabel}: Select targets`;
  if (command.type === "select_options") return `${index}. ${actorLabel}: Select option`;
  if (command.type === "declare_attackers") return `${index}. ${actorLabel}: Declare attackers`;
  if (command.type === "declare_blockers") return `${index}. ${actorLabel}: Declare blockers`;
  return `${index}. ${actorLabel}: ${command.type || "Action"}`;
}

export default function VerifyMatchSheet({
  trigger,
  triggerClassName = defaultTriggerClassName,
}) {
  const {
    game,
    state,
    multiplayer,
    setStatus,
    exportAuditTranscript,
    replayAuditTranscript,
    auditReplay,
    prepareAuditReplaySession,
    beginAuditReplaySession,
    setAuditReplayPosition,
    exitAuditReplaySession,
  } = useGame();
  const { t } = useI18n();
  const [verifyOpen, setVerifyOpen] = useState(false);
  const [verification, setVerification] = useState(emptyVerification);
  const verifyInputRef = useRef(null);

  const canVerifyCurrentMatch = Boolean(
    typeof exportAuditTranscript === "function"
    && (
      multiplayer?.matchStarted
      || multiplayer?.matchDisputed
      || multiplayer?.mode === "disputed"
      || state?.game_over
    )
  );
  const canVerifyMatch = Boolean(game && typeof replayAuditTranscript === "function");

  const verifyExportedShuffleProof = async (proof) => {
    if (!game || typeof game.ziffleVerifyShuffle !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const verified = await game.ziffleVerifyShuffle({
      deckCount: Number(proof.deckCount),
      context: String(proof.context || ""),
      keyContext: String(proof.keyContext || proof.context || ""),
      keys: proof.keys || [],
      steps: proof.steps || [],
    });
    if (String(verified.deckHash || "") !== String(proof.deckHash || "")) {
      throw new Error(`Ziffle shuffle proof mismatch for player ${Number(proof.owner) + 1}`);
    }
  };

  const verifyExportedZiffleOpening = async ({ proof, ceremony }) => {
    if (!game || typeof game.ziffleRevealCard !== "function") {
      throw new Error("Ziffle mental-poker backend is not available");
    }
    const reveal = await game.ziffleRevealCard({
      deckCount: Number(ceremony.deckCount),
      context: String(ceremony.context || ""),
      keyContext: String(ceremony.keyContext || ceremony.context || ""),
      keys: ceremony.keys || [],
      steps: ceremony.steps || [],
      cardPosition: Number(proof.position),
      tokens: proof.tokens || [],
    });
    return { originalSlot: Number(reveal.originalSlot) };
  };

  const describeVerificationReport = (report) => {
    const actions = Number(report?.verifiedActions || 0);
    const suffix = `${actions} action${actions === 1 ? "" : "s"}`;
    const replaySuffix = " with engine replay";
    const prefix = "Match verified";
    const outcome = report?.outcome || {};
    if (outcome.status === "winner") {
      const winner = outcome.winnerName || `Player ${Number(outcome.winner) + 1}`;
      return `${prefix}${replaySuffix}: ${winner} wins (${suffix})`;
    }
    if (outcome.status === "draw") {
      return `${prefix}${replaySuffix}: draw (${suffix})`;
    }
    if (outcome.status === "disputed") {
      const accused = (outcome.accusedPlayers || [])
        .map((player) => `Player ${Number(player) + 1}`)
        .join(", ");
      return `${prefix}${replaySuffix}: disputed${accused ? `; evidence implicates ${accused}` : ""} (${suffix})`;
    }
    return `${prefix}${replaySuffix}: stalled or incomplete (${suffix})`;
  };

  const verifyTranscript = async (transcript, sourceLabel) => {
    setVerifyOpen(true);
    setVerification({
      phase: "verifying",
      sourceLabel,
      summary: "Verifying",
      report: null,
      transcript,
      error: "",
    });
    try {
      if (typeof replayAuditTranscript !== "function") {
        throw new Error("Match verification requires engine replay, but the replay engine is unavailable");
      }
      const report = await verifyLiveAuditTranscript(
        transcript,
        globalThis.crypto,
        {
          requireEngineReplay: true,
          replayTranscript: replayAuditTranscript,
          verifyShuffleProof: verifyExportedShuffleProof,
          verifyZiffleOpening: verifyExportedZiffleOpening,
        }
      );
      const summary = describeVerificationReport(report);
      if (
        report?.engineReplay?.verified
        && typeof prepareAuditReplaySession === "function"
      ) {
        prepareAuditReplaySession({ transcript, sourceLabel });
      }
      setVerification({
        phase: "valid",
        sourceLabel,
        summary,
        report,
        transcript,
        error: "",
      });
      setStatus(summary);
    } catch (err) {
      const message = String(err?.message || err);
      setVerification({
        phase: "error",
        sourceLabel,
        summary: "Verification failed",
        report: null,
        transcript,
        error: message,
      });
      setStatus(`Verify match failed: ${message}`, true);
    }
  };

  const handleVerifyCurrentMatch = async () => {
    if (typeof exportAuditTranscript !== "function") {
      setStatus("No match transcript is available to verify", true);
      return;
    }
    try {
      const transcript = await exportAuditTranscript();
      if (!transcript) {
        setVerification({
          ...emptyVerification,
          phase: "error",
          summary: "Verification failed",
          error: "No match transcript is available to verify",
        });
        setVerifyOpen(true);
        setStatus("No match transcript is available to verify", true);
        return;
      }
      await verifyTranscript(transcript, "Current match");
    } catch (err) {
      const message = String(err?.message || err);
      setVerification({
        ...emptyVerification,
        phase: "error",
        sourceLabel: "Current match",
        summary: "Verification failed",
        error: message,
      });
      setVerifyOpen(true);
      setStatus(`Verify match failed: ${message}`, true);
    }
  };

  const handleVerifyMatchFile = async (event) => {
    const file = event.target.files?.[0] || null;
    event.target.value = "";
    if (!file) return;
    setVerifyOpen(true);
    setVerification({
      phase: "verifying",
      sourceLabel: file.name,
      summary: "Reading JSON",
      report: null,
      transcript: null,
      error: "",
    });
    try {
      const transcript = JSON.parse(await file.text());
      await verifyTranscript(transcript, file.name);
    } catch (err) {
      const message = String(err?.message || err);
      setVerification({
        phase: "error",
        sourceLabel: file.name,
        summary: "Verification failed",
        report: null,
        transcript: null,
        error: message,
      });
      setStatus(`Verify match failed: ${message}`, true);
    }
  };

  const openSheet = () => setVerifyOpen(true);
  const openFilePicker = () => verifyInputRef.current?.click();
  const facts = verificationFacts(verification.report, verification.transcript);
  const verificationBusy = verification.phase === "verifying";
  const verificationValid = verification.phase === "valid";
  const verificationError = verification.phase === "error";
  const canStartReplay = Boolean(
    verificationValid
    && verification.transcript
    && verification.report?.engineReplay?.verified
    && typeof beginAuditReplaySession === "function"
  );
  const replayActionCount = Number(auditReplay?.actionCount || 0);
  const replayPosition = Number(auditReplay?.currentActionIndex || 0);
  const replayBusy = Boolean(auditReplay?.busy);
  const replayActive = Boolean(auditReplay?.active);
  const replayAction = replayPosition > 0
    ? verification.transcript?.actions?.[replayPosition - 1]
    : null;
  const replayLabel = replayCommandLabel(replayAction, replayPosition);

  const startReplay = async () => {
    try {
      await beginAuditReplaySession({
        transcript: verification.transcript,
        sourceLabel: verification.sourceLabel || "Verified match",
      });
    } catch {
      // GameContext publishes the actionable error.
    }
  };

  const moveReplay = async (position) => {
    try {
      await setAuditReplayPosition(position);
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

  const triggerNode = trigger && isValidElement(trigger)
    ? cloneElement(trigger, {
      disabled: trigger.props.disabled || !canVerifyMatch,
      onClick: (event) => {
        trigger.props.onClick?.(event);
        if (!event.defaultPrevented) openSheet();
      },
    })
    : (
      <button
        type="button"
        className={triggerClassName}
        disabled={!canVerifyMatch}
        onClick={openSheet}
      >
        <ShieldCheck className="size-3.5" aria-hidden="true" />
        {t("action.verifyMatch")}
      </button>
    );

  return (
    <>
      <input
        ref={verifyInputRef}
        type="file"
        accept="application/json,.json"
        className="hidden"
        onChange={handleVerifyMatchFile}
      />
      {triggerNode}
      <Sheet open={verifyOpen} onOpenChange={setVerifyOpen}>
        <SheetContent
          side="center"
          className="verify-match-sheet fantasy-sheet overflow-hidden p-0"
          style={{ width: "min(96vw, 720px)", maxWidth: "720px" }}
        >
          <SheetHeader className="fantasy-sheet-header verify-match-header pr-12">
            <div className="verify-match-eyebrow">Audit</div>
            <SheetTitle className="verify-match-title">
              {t("action.verifyMatch")}
            </SheetTitle>
          </SheetHeader>

          <div className="verify-match-body">
            <div className="verify-match-toolbar">
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill verify-match-command"
                disabled={!canVerifyMatch || verificationBusy}
                onClick={openFilePicker}
              >
                <Upload className="size-3.5" aria-hidden="true" />
                Open JSON
              </Button>
              <Button
                type="button"
                variant="secondary"
                size="sm"
                className="stone-pill verify-match-command"
                disabled={!canVerifyMatch || !canVerifyCurrentMatch || verificationBusy}
                onClick={() => void handleVerifyCurrentMatch()}
              >
                <ShieldCheck className="size-3.5" aria-hidden="true" />
                Current Match
              </Button>
            </div>

            <section
              className={[
                "verify-match-result",
                verificationValid ? "verify-match-result--valid" : "",
                verificationError ? "verify-match-result--error" : "",
              ].filter(Boolean).join(" ")}
              aria-live="polite"
            >
              <div className="verify-match-result-icon" aria-hidden="true">
                {verificationBusy ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : verificationValid ? (
                  <CheckCircle2 className="size-4" />
                ) : verificationError ? (
                  <AlertTriangle className="size-4" />
                ) : (
                  <FileJson className="size-4" />
                )}
              </div>
              <div className="verify-match-result-copy">
                <div className="verify-match-result-title">
                  {verification.summary || "No audit loaded"}
                </div>
                {verification.sourceLabel ? (
                  <div className="verify-match-source">{verification.sourceLabel}</div>
                ) : null}
                {verification.error ? (
                  <div className="verify-match-error">{verification.error}</div>
                ) : null}
              </div>
            </section>

            {facts.length > 0 ? (
              <dl className="verify-match-facts">
                {facts.map(([label, value]) => (
                  <div className="verify-match-fact" key={label}>
                    <dt>{label}</dt>
                    <dd title={value}>{value}</dd>
                  </div>
                ))}
              </dl>
            ) : null}

            {verificationValid ? (
              <section className="verify-match-replay">
                <div className="verify-match-replay-head">
                  <div>
                    <div className="verify-match-replay-eyebrow">Replay</div>
                    <div className="verify-match-replay-title">
                      {replayActive
                        ? `Action ${replayPosition} of ${replayActionCount}`
                        : "Load into table"}
                    </div>
                  </div>
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    className="stone-pill verify-match-command"
                    disabled={!canStartReplay || replayBusy}
                    onClick={() => void startReplay()}
                  >
                    {replayBusy && !replayActive ? (
                      <Loader2 className="size-3.5 animate-spin" aria-hidden="true" />
                    ) : (
                      <Play className="size-3.5" aria-hidden="true" />
                    )}
                    Start
                  </Button>
                </div>

                {replayActive ? (
                  <>
                    <div className="verify-match-replay-current" title={replayLabel}>
                      {replayLabel}
                    </div>
                    <div className="verify-match-replay-controls">
                      <Button
                        type="button"
                        variant="secondary"
                        size="icon"
                        className="stone-icon-button verify-match-replay-button"
                        disabled={replayBusy || replayPosition <= 0}
                        onClick={() => void moveReplay(0)}
                        title="Jump to start"
                      >
                        <ChevronsLeft className="size-4" aria-hidden="true" />
                      </Button>
                      <Button
                        type="button"
                        variant="secondary"
                        size="icon"
                        className="stone-icon-button verify-match-replay-button"
                        disabled={replayBusy || replayPosition <= 0}
                        onClick={() => void moveReplay(replayPosition - 1)}
                        title="Previous action"
                      >
                        <StepBack className="size-4" aria-hidden="true" />
                      </Button>
                      <Button
                        type="button"
                        variant="secondary"
                        size="icon"
                        className="stone-icon-button verify-match-replay-button"
                        disabled={replayBusy || replayPosition >= replayActionCount}
                        onClick={() => void moveReplay(replayPosition + 1)}
                        title="Next action"
                      >
                        {replayBusy ? (
                          <Loader2 className="size-4 animate-spin" aria-hidden="true" />
                        ) : (
                          <StepForward className="size-4" aria-hidden="true" />
                        )}
                      </Button>
                      <Button
                        type="button"
                        variant="secondary"
                        size="icon"
                        className="stone-icon-button verify-match-replay-button"
                        disabled={replayBusy || replayPosition >= replayActionCount}
                        onClick={() => void moveReplay(replayActionCount)}
                        title="Jump to end"
                      >
                        <ChevronsRight className="size-4" aria-hidden="true" />
                      </Button>
                      <Button
                        type="button"
                        variant="secondary"
                        size="sm"
                        className="stone-pill verify-match-command"
                        disabled={replayBusy}
                        onClick={() => setVerifyOpen(false)}
                      >
                        View Table
                      </Button>
                      <Button
                        type="button"
                        variant="secondary"
                        size="sm"
                        className="stone-pill verify-match-command"
                        disabled={replayBusy}
                        onClick={() => void exitReplay()}
                      >
                        <PauseCircle className="size-3.5" aria-hidden="true" />
                        Exit
                      </Button>
                    </div>
                  </>
                ) : null}

                {auditReplay?.error ? (
                  <div className="verify-match-replay-error">{auditReplay.error}</div>
                ) : null}
              </section>
            ) : null}
          </div>
        </SheetContent>
      </Sheet>
    </>
  );
}

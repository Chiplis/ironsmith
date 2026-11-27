import { useEffect, useState } from "react";
import { Repeat2, Square, X } from "lucide-react";
import { useGame } from "@/context/GameContext";
import useUiText from "@/i18n/useUiText";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Sheet, SheetContent, SheetDescription, SheetHeader, SheetTitle } from "@/components/ui/sheet";
import { playerDisplayName, samePlayerId } from "@/lib/player-display";
import { getPlayerAccent } from "@/lib/player-colors";

const DEFAULT_REPETITIONS = 10;

function parseRepetitions(raw, max = Infinity) {
  const value = Math.floor(Number(raw));
  if (!Number.isFinite(value) || value < 0) return null;
  return Math.min(value, max);
}

/**
 * Record-loop toggle and loop status for the table's quick controls
 * (CR 732.2 shortcuts). Recording keeps the player's own choices as steps;
 * stopping opens the proposal.
 */
export function LoopShortcutButton() {
  const { state, loopShortcut } = useGame();
  const ui = useUiText();
  if (!loopShortcut || !state || state.game_over) return null;
  const shortcut = loopShortcut.shortcut;
  const perspective = state.perspective;

  if (shortcut) {
    const isProposer = samePlayerId(shortcut.proposer, perspective);
    const waitingFor = shortcut.awaiting_player != null
      ? playerDisplayName(state.players, state.players?.find((player) => samePlayerId(player.id, shortcut.awaiting_player)))
      : null;
    const label = shortcut.stepping
      ? ui("Loop: manual responses on iteration {0}", { 0: shortcut.interrupt_at })
      : shortcut.running ? ui("Loop running ×{0}", { 0: shortcut.agreed })
      : ui("Loop: waiting for {0}", { 0: waitingFor || ui("players") });
    return (
      <span className="loop-shortcut-status decision-quick-chip" role="status">
        <span>{label}</span>
        {isProposer ? (
          <button type="button" className="decision-phase-pass" onClick={() => loopShortcut.cancel(ui("Loop ended"))}>
            {ui("Stop loop")}
          </button>
        ) : null}
      </span>
    );
  }

  if (loopShortcut.recording) {
    return (
      <span className="loop-shortcut-control loop-shortcut-control--recording" role="group" aria-label={ui("Stop recording ({0})", { 0: loopShortcut.recordedStepCount })}>
        <button
          type="button"
          className="loop-shortcut-action loop-shortcut-recording"
          aria-label={ui("Stop recording ({0})", { 0: loopShortcut.recordedStepCount })}
          title={ui("Stop recording and propose repeating these actions")}
          onClick={loopShortcut.stopRecording}
        >
          <Square size={14} fill="currentColor" aria-hidden="true" />
        </button>
        <button type="button" className="loop-shortcut-action" aria-label={ui("Discard recording")} title={ui("Discard recording")} onClick={loopShortcut.discardRecording}>
          <X size={16} aria-hidden="true" />
        </button>
      </span>
    );
  }

  // A loop the engine detected is a recording that is already made: the same
  // button glows and opens its proposal. Declining returns it to recording.
  const detected = loopShortcut.detectedLoop;
  return (
    <span className={`loop-shortcut-control${detected ? " loop-shortcut-control--available" : ""}`}>
      <button
        type="button"
        className="loop-shortcut-action loop-shortcut-record"
        data-loop-available={detected ? "true" : undefined}
        aria-label={detected ? ui("Repeat loop ({0})", { 0: detected.summary }) : ui("Record loop")}
        title={detected
          ? ui("You repeated a loop. Each repetition: {0}. Propose repeating it automatically.", { 0: detected.summary })
          : ui("Record a loop: your next choices are saved so you can repeat them")}
        onClick={detected ? loopShortcut.proposeDetectedLoop : loopShortcut.startRecording}
      >
        <Repeat2 size={18} aria-hidden="true" />
      </button>
    </span>
  );
}

/** "+1 colorless mana Alice, -1 life Bob", each name in its player's colour. */
function LoopPerLapSummary({ entries, players, perspective, accentOverrides }) {
  return entries.map((entry, index) => {
    const amount = Number(entry.amount) || 0;
    const player = entry.player == null ? null : players.find((candidate) => samePlayerId(candidate.id, entry.player));
    const accent = player ? getPlayerAccent(players, player.id, perspective, accentOverrides) : null;
    return (
      <span key={`${entry.player ?? "none"}:${entry.resource}`} className="loop-shortcut-delta">
        {index > 0 ? ", " : ""}
        {`${amount > 0 ? "+" : ""}${amount} ${entry.resource}`}
        {player ? (
          <>
            {" "}
            <span className="loop-shortcut-delta-player" style={{ color: accent?.hex, fontWeight: 700 }}>
              {playerDisplayName(players, player)}
            </span>
          </>
        ) : null}
      </span>
    );
  });
}

function ProposalDialog() {
  const { state, loopShortcut, playerAccentOverrides } = useGame();
  const ui = useUiText();
  const [count, setCount] = useState(String(DEFAULT_REPETITIONS));
  const [submitting, setSubmitting] = useState(false);
  const draft = loopShortcut?.proposalDraft;
  // A draw loop stops at the last card of the library, never past it.
  const most = Math.min(10_000, Number.isFinite(Number(draft?.maxRepetitions)) && draft?.maxRepetitions != null ? Number(draft.maxRepetitions) : 10_000);
  useEffect(() => { if (draft) setCount(String(Math.min(DEFAULT_REPETITIONS, most))); }, [draft, most]);
  if (!draft) return null;
  const decision = state?.decision;
  const hasPriority = decision?.kind === "priority" && samePlayerId(decision.player, state?.perspective);
  const repetitions = parseRepetitions(count, most);
  // Cards the proposer draws each repetition, for the up-front warning.
  const drawnPerLap = (draft.perLap || []).reduce((total, entry) => (
    entry.resource === "cards in library" && samePlayerId(entry.player, state?.perspective) && entry.amount < 0
      ? total - Number(entry.amount) : total
  ), 0);
  const propose = async () => {
    if (!repetitions || submitting) return;
    setSubmitting(true);
    try { await loopShortcut.propose(repetitions); } finally { setSubmitting(false); }
  };
  return (
    <Sheet open onOpenChange={(open) => { if (!open) loopShortcut.discardRecording(); }}>
      <SheetContent side="center" className="fantasy-sheet gap-0 p-0" style={{ maxWidth: "420px" }}>
        <SheetHeader className="fantasy-sheet-header">
          <SheetTitle>{ui("Repeat this loop")}</SheetTitle>
          <SheetDescription>
            {ui("{0} recorded step(s). Opponents can accept or choose an iteration to interrupt. Accepted iterations run automatically.", { 0: draft.steps.length })}
            {draft.perLap?.length ? (
              <span className="loop-shortcut-per-lap">
                {` ${ui("Each repetition:")} `}
                <LoopPerLapSummary
                  entries={draft.perLap}
                  players={state?.players || []}
                  perspective={state?.perspective}
                  accentOverrides={playerAccentOverrides}
                />
                .
              </span>
            ) : draft.summary ? ` ${ui("Each repetition: {0}.", { 0: draft.summary })}` : ""}
          </SheetDescription>
        </SheetHeader>
        <div className="grid gap-3 px-4 py-4">
          <label className="grid gap-1 text-sm">
            <span>{ui("Repetitions")}</span>
            <Input type="number" min={1} max={most} value={count} onChange={(event) => setCount(event.target.value)} autoFocus />
          </label>
          {draft.drawsCards ? (
            <p className="loop-shortcut-draw-warning text-sm" role="alert">
              {drawnPerLap > 0
                ? ui("This loop draws cards. You will draw {0} card(s), and you cannot act on any of them until all {1} repetition(s) have finished.", { 0: drawnPerLap * (repetitions || 0), 1: repetitions || 0 })
                : ui("This loop draws cards. All {0} repetition(s) finish before anyone can act on the cards drawn.", { 0: repetitions || 0 })}
              {draft.maxRepetitions != null ? ` ${ui("At most {0} repetition(s) before a library runs out.", { 0: most })}` : ""}
            </p>
          ) : null}
          {!hasPriority ? <p className="text-sm opacity-80">{ui("You can propose the loop when you have priority.")}</p> : null}
          <div className="flex justify-end gap-2">
            <Button className="decision-neon-button" variant="ghost" onClick={loopShortcut.discardRecording}>{ui("Discard")}</Button>
            <Button className="decision-neon-button" disabled={!repetitions || !hasPriority || submitting} onClick={propose}>
              {ui("Propose ×{0}", { 0: repetitions || 0 })}
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

function ResponseDialog() {
  const { state, loopShortcut } = useGame();
  const ui = useUiText();
  const shortcut = loopShortcut?.shortcut;
  const awaitingMe = Boolean(shortcut && !shortcut.running && samePlayerId(shortcut.awaiting_player, state?.perspective));
  const [count, setCount] = useState("");
  const [submitting, setSubmitting] = useState(false);
  useEffect(() => { if (awaitingMe) setCount(String(shortcut.agreed)); }, [awaitingMe, shortcut?.agreed]);
  if (!awaitingMe) return null;
  const proposer = state.players?.find((player) => samePlayerId(player.id, shortcut.proposer));
  const shortened = parseRepetitions(count, shortcut.agreed);
  const answer = async (iterations, interruptAt = null) => {
    if (submitting || iterations == null) return;
    setSubmitting(true);
    try { await loopShortcut.respond(iterations, interruptAt); } finally { setSubmitting(false); }
  };
  const steps = Array.isArray(shortcut.steps) ? shortcut.steps.length : 0;
  return (
    <Sheet open>
      <SheetContent side="center" className="fantasy-sheet gap-0 p-0" style={{ maxWidth: "440px" }} showCloseButton={false}>
        <SheetHeader className="fantasy-sheet-header">
          <SheetTitle>{ui("Loop proposed")}</SheetTitle>
          <SheetDescription>
            {ui("{0} proposes repeating a {1}-step loop {2} time(s). You pass priority while it repeats.", {
              0: playerDisplayName(state.players, proposer),
              1: steps,
              2: shortcut.agreed,
            })}
          </SheetDescription>
        </SheetHeader>
        <div className="grid gap-3 px-4 py-4">
          <label className="grid gap-1 text-sm">
            <span>{ui("Interrupt on iteration")}</span>
            <Input type="number" min={1} max={shortcut.agreed} value={count} onChange={(event) => setCount(event.target.value)} />
          </label>
          <p className="text-sm opacity-80">{ui("Earlier iterations finish automatically. On this iteration, pass priority one step at a time until you want to respond.")}</p>
          <div className="flex flex-wrap justify-end gap-2">
            <Button className="decision-neon-button" variant="ghost" disabled={submitting} onClick={() => answer(0)}>{ui("Decline")}</Button>
            <Button
              className="decision-neon-button"
              disabled={submitting || shortened == null || shortened < 1}
              onClick={() => answer(shortcut.agreed, shortened)}
            >
              {ui("Interrupt on iteration {0}", { 0: shortened ?? 0 })}
            </Button>
            <Button className="decision-neon-button" disabled={submitting} onClick={() => answer(shortcut.agreed)}>
              {ui("Accept ×{0}", { 0: shortcut.agreed })}
            </Button>
          </div>
        </div>
      </SheetContent>
    </Sheet>
  );
}

/** Proposal and response prompts; mount once per table. */
export function LoopShortcutDialogs() {
  return (
    <>
      <ProposalDialog />
      <ResponseDialog />
    </>
  );
}

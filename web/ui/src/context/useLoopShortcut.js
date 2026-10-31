import { useCallback, useEffect, useRef, useState, useMemo } from "react";
import { describeLoopStep, recordLoopStep, recordResolveStackStep, resolveLoopStep } from "@/lib/loop-shortcut";

// A replayed step that leaves the snapshot unchanged this long was rejected.
const STEP_ACCEPT_TIMEOUT_MS = 10_000;

function samePlayer(left, right) {
  return left != null && right != null && Number(left) === Number(right);
}

function passCommand(decision) {
  const pass = (decision?.actions || []).find((action) => (action?.action_ref?.kind || action?.kind) === "pass_priority");
  if (!pass) return null;
  return { type: "priority_action", action_index: Number(pass.index), action_ref: pass.action_ref || { kind: "pass_priority" } };
}

// The shortcut's identity, so a new proposal restarts the replay cursor.
function shortcutKey(shortcut) {
  return shortcut ? `${shortcut.proposer}:${shortcut.proposed}:${JSON.stringify(shortcut.steps)}` : "";
}

/**
 * CR 732.2 loop shortcuts on the client.
 *
 * Recording keeps every command the local player submits for their own
 * decisions as a step (by stable object identity). Stopping opens the
 * proposal; the engine replicates the proposal and each opponent's response.
 * Once everyone accepted, the proposer's client replays the steps as ordinary
 * commands while every other player's client passes priority, until the
 * agreed count is reached or a step no longer resolves.
 *
 * A command counts as accepted once the snapshot it was submitted against is
 * replaced; multiplayer submission reports failures without throwing.
 */
export default function useLoopShortcut({ state, stateRef, dispatch, multiplayerMatchStarted, setStatus }) {
  const [recording, setRecording] = useState(null);
  const recordingRef = useRef(null);
  const pendingRecordRef = useRef(null);
  const [proposalDraft, setProposalDraft] = useState(null);
  const replayRef = useRef({ key: "", completed: 0, inFlight: false, awaiting: null });
  const [replaySubmission, setReplaySubmission] = useState(null);
  const autoRef = useRef({ inFlight: false });
  const [tick, setTick] = useState(0);
  const bump = useCallback(() => setTick((value) => value + 1), []);

  const perspective = state?.perspective;
  const snapshotId = state?.snapshot_id;
  const shortcut = state?.loop_shortcut || null;

  // Effects can run before the previous dispatch has released its input gate.
  // Keep automatic continuations queued while their snapshot is still current.
  const dispatchAutomatic = useCallback((command, message) => dispatch(command, message, {
    waitForLoopReady: true,
    expectedLoopSnapshotId: snapshotId,
  }), [dispatch, snapshotId]);

  const startRecording = useCallback(() => {
    recordingRef.current = { steps: [] };
    pendingRecordRef.current = null;
    setRecording({ steps: [] });
    setProposalDraft(null);
    setStatus?.("Recording loop: play one full cycle, then stop recording");
  }, [setStatus]);

  const discardRecording = useCallback(() => {
    recordingRef.current = null;
    pendingRecordRef.current = null;
    setRecording(null);
    setProposalDraft(null);
  }, []);

  const appendStep = useCallback((step) => {
    if (!recordingRef.current) return;
    recordingRef.current = { steps: [...recordingRef.current.steps, step] };
    setRecording(recordingRef.current);
  }, []);

  // A recorded command is kept once the engine moved past its snapshot.
  useEffect(() => {
    const pending = pendingRecordRef.current;
    if (pending && snapshotId !== pending.snapshotId) {
      pendingRecordRef.current = null;
      appendStep(pending.step);
    }
  }, [appendStep, snapshotId]);

  const stopRecording = useCallback(() => {
    const pending = pendingRecordRef.current;
    if (pending && stateRef.current?.snapshot_id !== pending.snapshotId) appendStep(pending.step);
    pendingRecordRef.current = null;
    const steps = recordingRef.current?.steps || [];
    recordingRef.current = null;
    setRecording(null);
    if (steps.length === 0) {
      setStatus?.("Loop recording stopped: no actions were recorded");
      return;
    }
    setProposalDraft({ steps });
  }, [appendStep, setStatus, stateRef]);

  /** Wraps a user dispatch: a command the engine accepts becomes a step. */
  const recordingDispatch = useCallback(async (command, ...rest) => {
    const before = stateRef.current;
    const ownsDecision = before?.decision && samePlayer(before.decision.player, before.perspective);
    const step = recordingRef.current && ownsDecision ? recordLoopStep(command, before) : null;
    if (step) pendingRecordRef.current = { step, snapshotId: before?.snapshot_id };
    const result = await dispatch(command, ...rest);
    const pending = pendingRecordRef.current;
    if (pending?.step === step && stateRef.current?.snapshot_id !== pending.snapshotId) {
      pendingRecordRef.current = null;
      appendStep(step);
    }
    return result;
  }, [appendStep, dispatch, stateRef]);

  /** "Resolve all" was pressed: while recording, that is a step too. */
  const recordResolveStack = useCallback(() => {
    const current = stateRef.current;
    if (!recordingRef.current || !samePlayer(current?.decision?.player, current?.perspective)) return;
    const step = recordResolveStackStep(current);
    if (step) appendStep(step);
  }, [appendStep, stateRef]);

  const propose = useCallback(async (iterations) => {
    const steps = proposalDraft?.steps;
    const count = Math.floor(Number(iterations));
    if (!steps?.length || !Number.isFinite(count) || count < 1) return false;
    await dispatch({ type: "propose_loop_shortcut", steps, iterations: count }, `Proposed repeating the loop ${count} time(s)`);
    if (stateRef.current?.loop_shortcut) {
      setProposalDraft(null);
      return true;
    }
    return false;
  }, [dispatch, proposalDraft, stateRef]);

  const respond = useCallback((iterations, interruptAt = null) => {
    const count = Math.max(0, Math.floor(Number(iterations) || 0));
    return dispatch({
      type: "respond_loop_shortcut",
      player: Number(stateRef.current?.perspective),
      iterations: count,
      ...(interruptAt == null ? {} : { interrupt_at: interruptAt }),
    }, count > 0 ? "Accepted the loop" : "Declined the loop");
  }, [dispatch, stateRef]);

  const cancel = useCallback((message) => {
    const current = stateRef.current?.loop_shortcut;
    if (!current) return Promise.resolve();
    return dispatchAutomatic({ type: "cancel_loop_shortcut", player: Number(current.proposer) }, message || "Loop ended");
  }, [dispatchAutomatic, stateRef]);

  // Local games: the other seats are not people, so they accept as proposed.
  useEffect(() => {
    if (stateRef.current !== state) return;
    if (multiplayerMatchStarted || !shortcut || shortcut.running) return;
    if (shortcut.awaiting_player == null || samePlayer(shortcut.awaiting_player, perspective)) return;
    if (autoRef.current.inFlight) return;
    autoRef.current.inFlight = true;
    Promise.resolve(dispatchAutomatic({ type: "respond_loop_shortcut", player: Number(shortcut.awaiting_player), iterations: shortcut.agreed }))
      .finally(() => { autoRef.current.inFlight = false; bump(); });
  }, [bump, dispatchAutomatic, multiplayerMatchStarted, perspective, shortcut, state, stateRef, tick]);

  // Replay on the proposer's client; passing on everyone else's.
  useEffect(() => {
    if (stateRef.current !== state) return undefined;
    if (!shortcut?.running) {
      replayRef.current = { key: "", completed: 0, inFlight: false, awaiting: null };
      return undefined;
    }
    const decision = state?.decision;
    if (!decision) return undefined;
    const decidesLocally = samePlayer(decision.player, perspective) || !multiplayerMatchStarted;

    // Every other player agreed to pass while the loop repeats.
    if (!samePlayer(decision.player, shortcut.proposer)) {
      if (shortcut.stepping) return undefined;
      if (!decidesLocally || decision.kind !== "priority" || autoRef.current.inFlight) return undefined;
      const command = passCommand(decision);
      if (!command) return undefined;
      autoRef.current.inFlight = true;
      Promise.resolve(dispatchAutomatic(command)).finally(() => { autoRef.current.inFlight = false; bump(); });
      return undefined;
    }
    if (!decidesLocally) return undefined;

    const key = shortcutKey(shortcut);
    if (replayRef.current.key !== key) replayRef.current = { key, completed: 0, inFlight: false, awaiting: null };
    const replay = replayRef.current;
    if (replay.inFlight) return undefined;
    const steps = Array.isArray(shortcut.steps) ? shortcut.steps : [];

    if (replay.awaiting) {
      if (snapshotId !== replay.awaiting.snapshotId) {
        const draining = replay.awaiting.step.type === "resolve_stack"
          && decision.kind === "priority" && Number(state.stack_size) > 0;
        replay.awaiting = null;
        if (!draining) replay.completed += 1;
      } else if (Date.now() - replay.awaiting.since > STEP_ACCEPT_TIMEOUT_MS) {
        const step = replay.awaiting.step;
        replay.awaiting = null;
        replay.inFlight = true;
        setStatus?.(`Loop stopped: ${describeLoopStep(step)} was not accepted`, true);
        Promise.resolve(cancel("Loop stopped")).finally(() => { replay.inFlight = false; bump(); });
        return undefined;
      } else {
        const timer = setTimeout(bump, 500);
        return () => clearTimeout(timer);
      }
    }

    if (shortcut.interrupt_at != null && !shortcut.stepping
      && replay.completed >= steps.length * (Number(shortcut.interrupt_at) - 1)) {
      replay.inFlight = true;
      Promise.resolve(dispatchAutomatic({ type: "step_loop_shortcut", player: Number(shortcut.proposer) },
        `Stepping through loop iteration ${shortcut.interrupt_at}`))
        .finally(() => { replay.inFlight = false; bump(); });
      return undefined;
    }

    if (replay.completed >= steps.length * Number(shortcut.agreed || 0)) {
      replay.inFlight = true;
      const message = shortcut.stepping ? `Loop paused after iteration ${shortcut.interrupt_at}` : `Loop finished: repeated ${shortcut.agreed} time(s)`;
      setStatus?.(message);
      Promise.resolve(cancel(message)).finally(() => { replay.inFlight = false; bump(); });
      return undefined;
    }
    const step = steps[replay.completed % steps.length];
    if (step?.type === "resolve_stack" && Number(state.stack_size) === 0) {
      replay.completed += 1;
      const timer = setTimeout(bump, 0);
      return () => clearTimeout(timer);
    }
    const resolved = resolveLoopStep(step, state);
    replay.inFlight = true;
    if (resolved.error) {
      const repetitions = Math.floor(replay.completed / steps.length);
      setStatus?.(`Loop stopped after ${repetitions} repetition(s): ${resolved.error}`, true);
      Promise.resolve(cancel("Loop stopped")).finally(() => { replay.inFlight = false; bump(); });
      return undefined;
    }
    replay.awaiting = { snapshotId, since: Date.now(), step };
    setReplaySubmission({ key, completed: replay.completed, snapshotId });
    Promise.resolve(dispatchAutomatic(resolved.command)).finally(() => { replay.inFlight = false; bump(); });
    return undefined;
  }, [bump, cancel, dispatchAutomatic, multiplayerMatchStarted, perspective, setStatus, shortcut, snapshotId, state, stateRef, tick]);

  // Advance the rendered cursor past an accepted action before effects run,
  // so a target prompt never briefly asks for a choice replay already owns.
  const completed = replaySubmission?.key === shortcutKey(shortcut)
    ? replaySubmission.completed + (snapshotId !== replaySubmission.snapshotId ? 1 : 0)
    : 0;
  const nextStep = shortcut?.steps?.[completed % shortcut.steps.length];
  const replayingTargets = Boolean(shortcut?.running
    && samePlayer(shortcut.proposer, perspective)
    && samePlayer(state?.decision?.player, perspective)
    && nextStep?.type === "select_targets"
    && !resolveLoopStep(nextStep, state).error);

  return useMemo(() => ({
    replayingTargets,
    recording: Boolean(recording),
    recordedStepCount: recording?.steps?.length || 0,
    startRecording,
    stopRecording,
    discardRecording,
    proposalDraft,
    propose,
    respond,
    cancel,
    shortcut,
    recordingDispatch,
    recordResolveStack,
  }), [replayingTargets, recording, startRecording, stopRecording, discardRecording,
    proposalDraft, propose, respond, cancel, shortcut, recordingDispatch, recordResolveStack]);
}

import { createContext, useContext, useState, useCallback, useRef, useMemo } from "react";
import { useWasmGame } from "@/hooks/useWasmGame";

const GameContext = createContext(null);

function decodeAttackTargetChoice(choice) {
  if (choice && typeof choice === "object") {
    if ("Player" in choice) return { kind: "player", player: Number(choice.Player) };
    if ("Planeswalker" in choice)
      return { kind: "planeswalker", object: Number(choice.Planeswalker) };
    if (choice.kind === "player") return { kind: "player", player: Number(choice.player) };
    if (choice.kind === "planeswalker")
      return { kind: "planeswalker", object: Number(choice.object) };
  }
  return { kind: "player", player: Number(choice) };
}

function defaultOpponentAttackerDeclarations(decision) {
  const declarations = [];
  for (const option of decision.attacker_options || []) {
    if (!option.must_attack) continue;
    const firstTarget = (option.valid_targets || [])[0];
    if (!firstTarget) continue;
    declarations.push({
      creature: Number(option.creature),
      target: decodeAttackTargetChoice(firstTarget),
    });
  }
  return declarations;
}

function tryBuildAutoResolveCommand(decision) {
  if (!decision) return null;

  if (
    decision.kind === "select_options" &&
    decision.min === 1 &&
    decision.max === 1 &&
    !(decision.reason || "").toLowerCase().includes("order")
  ) {
    const legal = (decision.options || []).filter((o) => o.legal);
    if (legal.length === 1) {
      return {
        cmd: { type: "select_options", option_indices: [legal[0].index] },
        label: `Auto: ${legal[0].description}`,
      };
    }
  }

  if (decision.kind === "select_objects") {
    const legal = (decision.candidates || []).filter((c) => c.legal);
    if (legal.length > 0 && legal.length === decision.min && decision.min === decision.max) {
      return {
        cmd: { type: "select_objects", object_ids: legal.map((c) => c.id) },
        label: `Auto: ${legal.map((c) => c.name).join(", ")}`,
      };
    }
  }

  if (decision.kind === "number" && decision.min === decision.max) {
    return {
      cmd: { type: "number_choice", value: decision.min },
      label: `Auto: ${decision.min}`,
    };
  }

  if (decision.kind === "targets") {
    const reqs = decision.requirements || [];
    if (
      reqs.length > 0 &&
      reqs.every((req) => {
        const maxT =
          req.max_targets === null || req.max_targets === undefined
            ? req.legal_targets.length
            : Number(req.max_targets);
        return (
          req.legal_targets.length > 0 &&
          req.legal_targets.length === req.min_targets &&
          req.min_targets === maxT
        );
      })
    ) {
      const targets = reqs.flatMap((req) =>
        req.legal_targets.map((t) =>
          t.kind === "player"
            ? { kind: "player", player: Number(t.player) }
            : { kind: "object", object: Number(t.object) }
        )
      );
      return { cmd: { type: "select_targets", targets }, label: "Auto: targets selected" };
    }
  }

  return null;
}

export function GameProvider({ children }) {
  const {
    game,
    loading,
    error: wasmError,
    progress: wasmProgress,
    phase: wasmPhase,
    registryCount: wasmRegistryCount,
    registryTotal: wasmRegistryTotal,
  } = useWasmGame();
  const [state, setState] = useState(null);
  const [status, setStatusRaw] = useState({ msg: "Loading WASM...", isError: false });
  const [autoPassEnabled, setAutoPassEnabled] = useState(true);
  const [holdRule, setHoldRule] = useState("never");
  const logRef = useRef([]);
  const [logEntries, setLogEntries] = useState([]);

  const pushLog = useCallback((message, isError = false) => {
    const time = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
    logRef.current = [{ time, message, isError }, ...logRef.current].slice(0, 120);
    setLogEntries([...logRef.current]);
  }, []);

  const setStatus = useCallback(
    (msg, isError = false) => {
      setStatusRaw({ msg, isError });
      pushLog(msg, isError);
    },
    [pushLog]
  );

  const opponentHoldReason = useCallback(
    (decision, currentState) => {
      if (!autoPassEnabled) return "auto-pass disabled";
      if (!decision || decision.kind !== "priority") return null;
      if (decision.player === currentState.perspective) return null;

      if (holdRule === "never") return null;
      if (holdRule === "always") return "always hold";
      if (holdRule === "stack" && currentState.stack_size > 0) return "stack non-empty";
      if (holdRule === "main" && (currentState.phase === "FirstMain" || currentState.phase === "NextMain"))
        return "main phase";
      if (holdRule === "combat" && currentState.phase === "Combat") return "combat phase";
      if (holdRule === "ending" && currentState.phase === "Ending") return "ending phase";
      if (holdRule === "if_actions") {
        const hasNonPass = (decision.actions || []).some((a) => a.kind !== "pass_priority");
        if (hasNonPass) return "opponent has playable actions";
      }
      return null;
    },
    [autoPassEnabled, holdRule]
  );

  const settleOpponentPriority = useCallback(
    async (currentGame, currentState) => {
      if (!currentState || !autoPassEnabled) {
        return { state: currentState, autoPasses: 0, autoDeclares: 0, phaseAdvances: 0, holdReason: null };
      }

      let st = currentState;
      let autoPasses = 0;
      let autoDeclares = 0;
      let phaseAdvances = 0;
      let holdReason = null;

      for (let i = 0; i < 24; i++) {
        while (st && st.decision && st.decision.player !== st.perspective) {
          if (st.decision.kind === "priority") {
            holdReason = opponentHoldReason(st.decision, st);
            if (holdReason) break;
            const passAction = (st.decision.actions || []).find((a) => a.kind === "pass_priority");
            if (!passAction) { holdReason = "no pass action available"; break; }
            if (autoPasses >= 80) { holdReason = "auto-pass safety limit reached"; break; }
            st = await currentGame.dispatch({ type: "priority_action", action_index: passAction.index });
            autoPasses += 1;
            continue;
          }
          if (autoDeclares >= 40) { holdReason = "auto-declare safety limit reached"; break; }
          if (st.decision.kind === "attackers") {
            const declarations = defaultOpponentAttackerDeclarations(st.decision);
            st = await currentGame.dispatch({ type: "declare_attackers", declarations });
            autoDeclares += 1;
            continue;
          }
          if (st.decision.kind === "blockers") {
            st = await currentGame.dispatch({ type: "declare_blockers", declarations: [] });
            autoDeclares += 1;
            continue;
          }
          holdReason = "opponent has non-priority decision";
          break;
        }
        if (holdReason) break;
        if (!st || st.game_over || st.decision) break;
        if (phaseAdvances >= 24) { holdReason = "phase auto-advance safety limit reached"; break; }
        const before = `${st.turn_number}|${st.phase}|${st.step}|${st.priority_player}|${st.stack_size}`;
        await currentGame.advancePhase();
        phaseAdvances += 1;
        st = await currentGame.uiState();
        const after = `${st.turn_number}|${st.phase}|${st.step}|${st.priority_player}|${st.stack_size}`;
        if (before === after) { holdReason = "advance phase made no progress"; break; }
      }

      return { state: st, autoPasses, autoDeclares, phaseAdvances, holdReason };
    },
    [autoPassEnabled, opponentHoldReason]
  );

  const autoResolveTrivialDecisions = useCallback(
    async (currentGame, currentState, settle) => {
      let resolved = 0;
      let st = currentState;
      while (resolved < 50 && st && st.decision) {
        const auto = tryBuildAutoResolveCommand(st.decision);
        if (!auto) break;
        try {
          st = await currentGame.dispatch(auto.cmd);
          resolved++;
          const settleResult = await settle(currentGame, st);
          st = settleResult.state;
        } catch (err) {
          console.warn("Auto-resolve failed:", err);
          break;
        }
      }
      return { state: st, resolved };
    },
    []
  );

  const refresh = useCallback(
    async (message) => {
      if (!game) return;
      try {
        let st = await game.uiState();
        const autoResult = await settleOpponentPriority(game, st);
        st = autoResult.state;
        const autoResolved = await autoResolveTrivialDecisions(game, st, settleOpponentPriority);
        st = autoResolved.state;
        setState(st);

        const parts = [];
        if (message) parts.push(message);
        if (autoResult.autoPasses > 0) parts.push(`auto-passed x${autoResult.autoPasses}`);
        if (autoResult.autoDeclares > 0) parts.push(`auto-declared x${autoResult.autoDeclares}`);
        if (autoResult.phaseAdvances > 0) parts.push(`auto-advanced x${autoResult.phaseAdvances}`);
        if (autoResult.holdReason && st?.decision?.player !== st?.perspective)
          parts.push(`holding (${autoResult.holdReason})`);
        if (autoResolved.resolved > 0) parts.push(`${autoResolved.resolved} auto-resolved`);
        if (parts.length) setStatus(parts.join(" \u2022 "));
      } catch (err) {
        setStatus(`Refresh failed: ${err}`, true);
      }
    },
    [game, settleOpponentPriority, autoResolveTrivialDecisions, setStatus]
  );

  const dispatch = useCallback(
    async (command, successMessage) => {
      if (!game) return;
      try {
        let st = await game.dispatch(command);
        const autoResult = await settleOpponentPriority(game, st);
        st = autoResult.state;
        const autoResolved = await autoResolveTrivialDecisions(game, st, settleOpponentPriority);
        st = autoResolved.state;
        setState(st);

        const parts = [];
        if (successMessage) parts.push(successMessage);
        if (autoResult.autoPasses > 0) parts.push(`auto-passed x${autoResult.autoPasses}`);
        if (autoResult.phaseAdvances > 0) parts.push(`auto-advanced x${autoResult.phaseAdvances}`);
        if (autoResolved.resolved > 0) parts.push(`${autoResolved.resolved} auto-resolved`);
        if (parts.length) setStatus(parts.join(" \u2022 "));
      } catch (err) {
        try {
          const st = await game.uiState();
          setState(st);
        } catch (_) {
          // keep original error
        }
        setStatus(`Action failed: ${err}`, true);
        console.error(err);
      }
    },
    [game, settleOpponentPriority, autoResolveTrivialDecisions, setStatus]
  );

  const value = useMemo(
    () => ({
      game,
      state,
      setState,
      loading,
      wasmError,
      wasmProgress,
      wasmPhase,
      wasmRegistryCount,
      wasmRegistryTotal,
      status,
      setStatus,
      dispatch,
      refresh,
      autoPassEnabled,
      setAutoPassEnabled,
      holdRule,
      setHoldRule,
      logEntries,
      pushLog,
    }),
    [
      game,
      state,
      loading,
      wasmError,
      wasmProgress,
      wasmPhase,
      wasmRegistryCount,
      wasmRegistryTotal,
      status,
      setStatus,
      dispatch, refresh, autoPassEnabled, holdRule, logEntries, pushLog,
    ]
  );

  return <GameContext.Provider value={value}>{children}</GameContext.Provider>;
}

export function useGame() {
  const ctx = useContext(GameContext);
  if (!ctx) throw new Error("useGame must be used within GameProvider");
  return ctx;
}

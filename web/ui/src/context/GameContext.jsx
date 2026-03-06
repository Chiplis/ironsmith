import { createContext, useContext, useState, useCallback, useRef, useMemo, useEffect } from "react";
import { useWasmGame } from "@/hooks/useWasmGame";
import { usePeerLobby } from "@/hooks/usePeerLobby";
import { cardsMeetingThresholdFromStats, loadSemanticStats } from "@/lib/semanticCache";

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

function isPaymentLikeOptionDescription(text) {
  const description = String(text || "").trim().toLowerCase();
  if (!description) return false;
  if (/^pay\b/.test(description)) return true;
  if (/^use\b.*\bfrom mana pool\b/.test(description)) return true;
  if (/^tap\b.*:\s*add\b/.test(description)) return true;
  return false;
}

function isPaymentSelectOptionsDecision(decision) {
  if (!decision || decision.kind !== "select_options") return false;
  if (isPaymentLikeOptionDescription(decision.description || "")) return true;
  return (decision.options || []).some((opt) => isPaymentLikeOptionDescription(opt?.description || ""));
}

function isCastOrPlayConfirmDecision(decision) {
  if (!decision || decision.kind !== "select_options") return false;
  const legal = (decision.options || []).filter((opt) => opt?.legal !== false);
  if (legal.length !== 1) return false;
  const optionText = String(legal[0]?.description || "");
  return /^\s*(cast|play)\b/i.test(optionText);
}

function tryBuildAutoResolveCommand(decision) {
  if (!decision) return null;

  if (
    decision.kind === "select_options" &&
    decision.min === 1 &&
    decision.max === 1 &&
    !(decision.reason || "").toLowerCase().includes("order")
  ) {
    if (isPaymentSelectOptionsDecision(decision) || isCastOrPlayConfirmDecision(decision)) {
      return null;
    }
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

function buildPriorityActionDescriptor(action) {
  return {
    kind: String(action?.kind || ""),
    label: String(action?.label || ""),
    object_id: action?.object_id == null ? null : Number(action.object_id),
    from_zone: action?.from_zone == null ? null : String(action.from_zone),
    to_zone: action?.to_zone == null ? null : String(action.to_zone),
  };
}

function samePriorityActionDescriptor(left, right) {
  return (
    left.kind === right.kind &&
    left.label === right.label &&
    left.object_id === right.object_id &&
    left.from_zone === right.from_zone &&
    left.to_zone === right.to_zone
  );
}

function buildOptionDescriptor(option) {
  return {
    description: String(option?.description || ""),
  };
}

function sameOptionDescriptor(left, right) {
  return left.description === right.description;
}

function buildOrdinalDescriptor(items, targetIndex, buildDescriptor, sameDescriptor) {
  const normalizedTargetIndex = Number(targetIndex);
  let target = null;
  for (const item of items || []) {
    if (Number(item?.index) === normalizedTargetIndex) {
      target = item;
      break;
    }
  }
  if (!target) return null;

  const descriptor = buildDescriptor(target);
  let ordinal = 0;
  for (const item of items || []) {
    if (Number(item?.index) === normalizedTargetIndex) break;
    if (sameDescriptor(buildDescriptor(item), descriptor)) {
      ordinal += 1;
    }
  }

  return { ...descriptor, ordinal };
}

function resolveOrdinalDescriptor(items, descriptor, buildDescriptor, sameDescriptor) {
  const expectedOrdinal = Math.max(0, Number(descriptor?.ordinal) || 0);
  let ordinal = 0;
  for (const item of items || []) {
    if (!sameDescriptor(buildDescriptor(item), descriptor)) continue;
    if (ordinal === expectedOrdinal) return item;
    ordinal += 1;
  }
  return null;
}

function serializeMultiplayerCommand(command, currentState) {
  if (!command || typeof command !== "object") return command;

  if (command.type === "priority_action") {
    const decision = currentState?.decision;
    if (decision?.kind !== "priority") return command;
    const action_ref = buildOrdinalDescriptor(
      decision.actions,
      command.action_index,
      buildPriorityActionDescriptor,
      samePriorityActionDescriptor
    );
    if (!action_ref) return command;
    return {
      type: "priority_action",
      action_ref,
    };
  }

  if (command.type === "select_options") {
    const decision = currentState?.decision;
    if (decision?.kind !== "select_options") return command;
    const option_refs = (command.option_indices || [])
      .map((optionIndex) =>
        buildOrdinalDescriptor(
          decision.options,
          optionIndex,
          buildOptionDescriptor,
          sameOptionDescriptor
        )
      )
      .filter(Boolean);
    if (option_refs.length !== (command.option_indices || []).length) {
      return command;
    }
    return {
      type: "select_options",
      option_refs,
    };
  }

  return command;
}

function resolveSyncedCommand(command, currentState) {
  if (!command || typeof command !== "object") return command;

  if (command.type === "priority_action" && command.action_ref) {
    const decision = currentState?.decision;
    if (decision?.kind !== "priority") {
      throw new Error("Expected a priority decision while syncing an action");
    }
    const match = resolveOrdinalDescriptor(
      decision.actions,
      command.action_ref,
      buildPriorityActionDescriptor,
      samePriorityActionDescriptor
    );
    if (!match) {
      throw new Error(`Could not resolve synced priority action: ${command.action_ref.label}`);
    }
    return {
      type: "priority_action",
      action_index: Number(match.index),
    };
  }

  if (command.type === "select_options" && Array.isArray(command.option_refs)) {
    const decision = currentState?.decision;
    if (decision?.kind !== "select_options") {
      throw new Error("Expected an options decision while syncing a choice");
    }
    const option_indices = command.option_refs.map((optionRef) => {
      const match = resolveOrdinalDescriptor(
        decision.options,
        optionRef,
        buildOptionDescriptor,
        sameOptionDescriptor
      );
      if (!match) {
        throw new Error(`Could not resolve synced option: ${optionRef.description}`);
      }
      return Number(match.index);
    });
    return {
      type: "select_options",
      option_indices,
    };
  }

  return command;
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
  const [inspectorDebug, setInspectorDebug] = useState(false);
  const [semanticThreshold, setSemanticThresholdRaw] = useState(96);
  const [cardsMeetingThreshold, setCardsMeetingThreshold] = useState(0);
  const [semanticStats, setSemanticStats] = useState(null);
  const logRef = useRef([]);
  const [logEntries, setLogEntries] = useState([]);
  const gameRef = useRef(game);
  const stateRef = useRef(state);
  const multiplayerActiveRef = useRef(false);

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

  useEffect(() => {
    gameRef.current = game;
  }, [game]);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const setSemanticThreshold = useCallback(
    async (value) => {
      setSemanticThresholdRaw(value);
      if (game && typeof game.setSemanticThreshold === "function") {
        try {
          await game.setSemanticThreshold(value);
        } catch (err) {
          console.warn("setSemanticThreshold failed:", err);
        }
      }

      const localCount = cardsMeetingThresholdFromStats(value, semanticStats);
      if (localCount !== null) {
        setCardsMeetingThreshold(localCount);
        return;
      }

      if (game && typeof game.cardsMeetingThreshold === "function") {
        try {
          const count = await game.cardsMeetingThreshold();
          setCardsMeetingThreshold(count);
        } catch (err) {
          console.warn("cardsMeetingThreshold failed:", err);
        }
      }
    },
    [game, semanticStats]
  );

  useEffect(() => {
    let cancelled = false;
    loadSemanticStats()
      .then((stats) => {
        if (cancelled) return;
        setSemanticStats(stats);
      })
      .catch((err) => {
        console.warn("semantic cache unavailable:", err);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    const localCount = cardsMeetingThresholdFromStats(semanticThreshold, semanticStats);
    if (localCount !== null) {
      queueMicrotask(() => {
        setCardsMeetingThreshold(localCount);
      });
      return;
    }

    if (!game || typeof game.cardsMeetingThreshold !== "function") return;
    game.cardsMeetingThreshold()
      .then((count) => setCardsMeetingThreshold(count))
      .catch(() => {});
  }, [game, wasmRegistryCount, semanticThreshold, semanticStats]);

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
      if (!currentState) {
        return { state: currentState, autoPasses: 0, autoDeclares: 0, phaseAdvances: 0, holdReason: null };
      }

      if (multiplayerActiveRef.current || !autoPassEnabled) {
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

  const settleNoop = useCallback(async (_currentGame, currentState) => ({
    state: currentState,
    autoPasses: 0,
    autoDeclares: 0,
    phaseAdvances: 0,
    holdReason: null,
  }), []);

  const finalizeState = useCallback(
    async (
      currentGame,
      currentState,
      {
        message = "",
        allowOpponentAutomation = true,
        allowTrivialAutomation = true,
      } = {}
    ) => {
      let st = currentState;
      const autoResult = allowOpponentAutomation
        ? await settleOpponentPriority(currentGame, st)
        : await settleNoop(currentGame, st);
      st = autoResult.state;

      const autoResolved = allowTrivialAutomation
        ? await autoResolveTrivialDecisions(
          currentGame,
          st,
          allowOpponentAutomation ? settleOpponentPriority : settleNoop
        )
        : { state: st, resolved: 0 };
      st = autoResolved.state;
      setState(st);
      stateRef.current = st;

      const parts = [];
      if (message) parts.push(message);
      if (allowOpponentAutomation && autoResult.autoPasses > 0) {
        parts.push(`auto-passed x${autoResult.autoPasses}`);
      }
      if (allowOpponentAutomation && autoResult.autoDeclares > 0) {
        parts.push(`auto-declared x${autoResult.autoDeclares}`);
      }
      if (allowOpponentAutomation && autoResult.phaseAdvances > 0) {
        parts.push(`auto-advanced x${autoResult.phaseAdvances}`);
      }
      if (
        allowOpponentAutomation &&
        autoResult.holdReason &&
        st?.decision?.player !== st?.perspective
      ) {
        parts.push(`holding (${autoResult.holdReason})`);
      }
      if (allowTrivialAutomation && autoResolved.resolved > 0) {
        parts.push(`${autoResolved.resolved} auto-resolved`);
      }
      if (parts.length > 0) {
        setStatus(parts.join(" \u2022 "));
      }

      return st;
    },
    [
      autoResolveTrivialDecisions,
      settleNoop,
      settleOpponentPriority,
      setStatus,
    ]
  );

  const applySyncedCommand = useCallback(
    async (command, successMessage = "") => {
      const currentGame = gameRef.current;
      if (!currentGame) {
        throw new Error("WASM game is not ready");
      }

      const resolvedCommand = resolveSyncedCommand(command, stateRef.current);
      const st = await currentGame.dispatch(resolvedCommand);
      return finalizeState(currentGame, st, {
        message: successMessage,
        allowOpponentAutomation: false,
        allowTrivialAutomation: false,
      });
    },
    [finalizeState]
  );

  const {
    multiplayer,
    canStartHostedMatch,
    createLobby,
    joinLobby,
    leaveLobby,
    startHostedMatch,
    updateLobbyDeck,
    submitMultiplayerCommand,
  } = usePeerLobby({
    game,
    setState,
    setStatus,
    applySyncedCommand,
  });

  useEffect(() => {
    multiplayerActiveRef.current = multiplayer.matchStarted;
  }, [multiplayer.matchStarted]);

  useEffect(() => {
    if (!game || typeof game.setAutoCleanupDiscard !== "function") return;
    game
      .setAutoCleanupDiscard(autoPassEnabled && !multiplayer.matchStarted)
      .catch((err) => console.warn("setAutoCleanupDiscard failed:", err));
  }, [autoPassEnabled, game, multiplayer.matchStarted]);

  const refresh = useCallback(
    async (message) => {
      if (!game) return;
      try {
        let st = await game.uiState();
        if (multiplayer.matchStarted) {
          setState(st);
          stateRef.current = st;
          if (message) setStatus(message);
          return;
        }
        await finalizeState(game, st, {
          message,
          allowOpponentAutomation: true,
          allowTrivialAutomation: true,
        });
      } catch (err) {
        setStatus(`Refresh failed: ${err}`, true);
      }
    },
    [finalizeState, game, multiplayer.matchStarted, setStatus]
  );

  const dispatch = useCallback(
    async (command, successMessage) => {
      if (!game) return;
      if (multiplayer.matchStarted) {
        const currentState = stateRef.current;
        if (!currentState?.decision) {
          setStatus("No pending decision to submit", true);
          return;
        }
        if (currentState.decision.player !== currentState.perspective) {
          setStatus("Waiting for the active player");
          return;
        }
        try {
          const syncedCommand = serializeMultiplayerCommand(command, currentState);
          await submitMultiplayerCommand(syncedCommand, successMessage);
        } catch (err) {
          setStatus(`Sync failed: ${err}`, true);
          console.error(err);
        }
        return;
      }

      try {
        let st = await game.dispatch(command);
        await finalizeState(game, st, {
          message: successMessage,
          allowOpponentAutomation: true,
          allowTrivialAutomation: true,
        });
      } catch (err) {
        try {
          // Roll back to the replay checkpoint so the game returns to a
          // consistent state (e.g. before a multi-step decision chain).
          let st = await game.cancelDecision();
          await finalizeState(game, st, {
            allowOpponentAutomation: true,
            allowTrivialAutomation: true,
          });
        } catch {
          // keep original error
        }
        setStatus(`Action failed: ${err}`, true);
        console.error(err);
      }
    },
    [finalizeState, game, multiplayer.matchStarted, setStatus, submitMultiplayerCommand]
  );

  const cancelDecision = useCallback(
    async () => {
      if (!game) return;
      if (multiplayer.matchStarted) {
        setStatus("Undo is disabled during multiplayer matches", true);
        return;
      }
      try {
        let st = await game.cancelDecision();
        await finalizeState(game, st, {
          message: "Decision cancelled",
          allowOpponentAutomation: true,
          allowTrivialAutomation: true,
        });
      } catch (err) {
        setStatus(`Cancel failed: ${err}`, true);
        console.error(err);
      }
    },
    [finalizeState, game, multiplayer.matchStarted, setStatus]
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
      cancelDecision,
      refresh,
      autoPassEnabled,
      setAutoPassEnabled,
      holdRule,
      setHoldRule,
      inspectorDebug,
      setInspectorDebug,
      semanticThreshold,
      setSemanticThreshold,
      cardsMeetingThreshold,
      logEntries,
      pushLog,
      multiplayer,
      canStartHostedMatch,
      createLobby,
      joinLobby,
      leaveLobby,
      startHostedMatch,
      updateLobbyDeck,
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
      dispatch, cancelDecision, refresh, autoPassEnabled, holdRule, inspectorDebug,
      semanticThreshold, setSemanticThreshold, cardsMeetingThreshold,
      logEntries, pushLog,
      multiplayer, canStartHostedMatch, createLobby, joinLobby, leaveLobby, startHostedMatch, updateLobbyDeck,
    ]
  );

  return <GameContext.Provider value={value}>{children}</GameContext.Provider>;
}

// eslint-disable-next-line react-refresh/only-export-components
export function useGame() {
  const ctx = useContext(GameContext);
  if (!ctx) throw new Error("useGame must be used within GameProvider");
  return ctx;
}

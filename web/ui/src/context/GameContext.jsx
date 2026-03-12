import { createContext, useContext, useState, useCallback, useRef, useMemo, useEffect } from "react";
import { useWasmGame } from "@/hooks/useWasmGame";
import { usePeerLobby } from "@/hooks/usePeerLobby";
import { isCombatPhase, isEndingPhase, isMainPhase } from "@/lib/constants";
import { cardsMeetingThresholdFromStats, loadSemanticStats } from "@/lib/semanticCache";
import {
  buildTriggerOrderingKey,
  defaultTriggerOrderingOrder,
  isTriggerOrderingDecision,
  normalizeTriggerOrderingOrder,
} from "@/lib/trigger-ordering";

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

function priorityHoldReason({
  autoPassEnabled,
  holdRule,
  decision,
  currentState,
  perspectiveMode = "any",
  requireNonEmptyStack = false,
}) {
  if (!autoPassEnabled) return "auto-pass disabled";
  if (!decision || decision.kind !== "priority") return "not a priority decision";

  const perspective = currentState?.perspective;
  if (perspectiveMode === "local" && decision.player !== perspective) return "not local priority";
  if (perspectiveMode === "opponent" && decision.player === perspective) return "not opponent priority";

  const stackSize = Number(currentState?.stack_size || 0);
  if (requireNonEmptyStack && stackSize <= 0) return "stack empty";

  if (holdRule === "never") return null;
  if (holdRule === "always") return "always hold";
  if (holdRule === "stack" && stackSize > 0) return "stack non-empty";
  if (holdRule === "main" && isMainPhase(currentState?.phase)) return "main phase";
  if (holdRule === "combat" && isCombatPhase(currentState?.phase)) return "combat phase";
  if (holdRule === "ending" && isEndingPhase(currentState?.phase)) return "ending phase";
  if (holdRule === "if_actions") {
    const hasNonPass = (decision.actions || []).some((action) => action.kind !== "pass_priority");
    if (hasNonPass) {
      return perspectiveMode === "opponent"
        ? "opponent has playable actions"
        : "playable actions available";
    }
  }

  return null;
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
    object_id: option?.object_id == null ? null : Number(option.object_id),
  };
}

function sameOptionDescriptor(left, right) {
  return left.description === right.description && left.object_id === right.object_id;
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

function currentOrderForDecision(triggerOrderingState, decision, key = buildTriggerOrderingKey(decision)) {
  if (!isTriggerOrderingDecision(decision)) return [];
  if (triggerOrderingState?.key === key) {
    return normalizeTriggerOrderingOrder(triggerOrderingState.order, decision);
  }
  return defaultTriggerOrderingOrder(decision);
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
  const [confirmEnabled, setConfirmEnabled] = useState(false);
  const [inspectorDebug, setInspectorDebug] = useState(false);
  const [triggerOrderingState, setTriggerOrderingState] = useState({ key: "", order: [] });
  const [semanticThreshold, setSemanticThresholdRaw] = useState(96);
  const [cardsMeetingThreshold, setCardsMeetingThreshold] = useState(0);
  const [semanticStats, setSemanticStats] = useState(null);
  const logRef = useRef([]);
  const [logEntries, setLogEntries] = useState([]);
  const gameRef = useRef(game);
  const semanticThresholdRef = useRef(semanticThreshold);
  const stateRef = useRef(state);
  const multiplayerActiveRef = useRef(false);
  const stickyViewedCardsRef = useRef(null);

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
    semanticThresholdRef.current = semanticThreshold;
  }, [semanticThreshold]);

  useEffect(() => {
    if (!game || typeof game.setSemanticThreshold !== "function") return;
    game.setSemanticThreshold(semanticThresholdRef.current).catch((err) => {
      console.warn("initial setSemanticThreshold failed:", err);
    });
  }, [game]);

  useEffect(() => {
    stateRef.current = state;
  }, [state]);

  const moveTriggerOrderingItem = useCallback((position, direction) => {
    const decision = stateRef.current?.decision || null;
    if (!isTriggerOrderingDecision(decision)) return;
    const key = buildTriggerOrderingKey(decision);

    setTriggerOrderingState((current) => {
      const currentOrder = current.key === key
        ? normalizeTriggerOrderingOrder(current.order, decision)
        : defaultTriggerOrderingOrder(decision);
      const nextPosition = Number(position) + Number(direction);
      if (
        !Number.isInteger(position)
        || !Number.isInteger(direction)
        || nextPosition < 0
        || nextPosition >= currentOrder.length
      ) {
        return current;
      }

      const nextOrder = [...currentOrder];
      [nextOrder[position], nextOrder[nextPosition]] = [nextOrder[nextPosition], nextOrder[position]];
      return {
        key,
        order: nextOrder,
      };
    });
  }, []);

  const activeTriggerOrderingState = useMemo(() => {
    const decision = state?.decision || null;
    if (!isTriggerOrderingDecision(decision)) return null;

    const key = buildTriggerOrderingKey(decision);
    return {
      key,
      order: currentOrderForDecision(triggerOrderingState, decision, key),
    };
  }, [state?.decision, triggerOrderingState]);

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
      if (!decision || decision.kind !== "priority") return "not a priority decision";
      if (decision.player === currentState?.perspective) return "not opponent priority";
      return null;
    },
    [autoPassEnabled]
  );

  const localTurnHoldReason = useCallback(
    (decision, currentState) => (
      priorityHoldReason({
        autoPassEnabled,
        holdRule,
        decision,
        currentState,
        perspectiveMode: "local",
        requireNonEmptyStack: true,
      })
    ),
    [autoPassEnabled, holdRule]
  );

  const localOffTurnHoldReason = useCallback(
    (decision, currentState) => (
      priorityHoldReason({
        autoPassEnabled,
        holdRule,
        decision,
        currentState,
        perspectiveMode: "local",
      })
    ),
    [autoPassEnabled, holdRule]
  );

  const settleLocalStackPriority = useCallback(
    async (currentGame, currentState) => {
      if (!currentState) {
        return { state: currentState, autoPasses: 0, holdReason: null };
      }

      if (multiplayerActiveRef.current || !autoPassEnabled) {
        return { state: currentState, autoPasses: 0, holdReason: null };
      }

      let st = currentState;
      let autoPasses = 0;
      let holdReason = null;

      for (let i = 0; i < 4; i++) {
        if (!st?.decision || st.decision.kind !== "priority" || st.decision.player !== st.perspective) {
          break;
        }
        if (st.active_player !== st.perspective) {
          break;
        }

        holdReason = localTurnHoldReason(st.decision, st);
        if (holdReason) break;

        const passAction = (st.decision.actions || []).find((action) => action.kind === "pass_priority");
        if (!passAction) {
          holdReason = "no pass action available";
          break;
        }
        if (passAction.label && passAction.label !== "Pass priority") {
          holdReason = "custom pass action";
          break;
        }

        st = await currentGame.dispatch({ type: "priority_action", action_index: passAction.index });
        autoPasses += 1;

        if (Number(st?.stack_size || 0) <= 0) break;
      }

      return { state: st, autoPasses, holdReason };
    },
    [autoPassEnabled, localTurnHoldReason]
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
        while (
          st
          && st.decision
          && (
            st.decision.player !== st.perspective
            || st.active_player !== st.perspective
          )
        ) {
          if (st.decision.kind === "priority") {
            const isLocalOffTurnPriority = st.decision.player === st.perspective;
            const passAction = (st.decision.actions || []).find((a) => a.kind === "pass_priority");
            if (!passAction) { holdReason = "no pass action available"; break; }
            const isCustomPassAction = !!passAction.label && passAction.label !== "Pass priority";
            if (!isLocalOffTurnPriority && isCustomPassAction) {
              if (autoPasses >= 80) { holdReason = "auto-pass safety limit reached"; break; }
              st = await currentGame.dispatch({ type: "priority_action", action_index: passAction.index });
              autoPasses += 1;
              continue;
            }
            holdReason = isLocalOffTurnPriority
              ? localOffTurnHoldReason(st.decision, st)
              : opponentHoldReason(st.decision, st);
            if (holdReason) break;
            if (passAction.label && passAction.label !== "Pass priority") {
              holdReason = "custom pass action";
              break;
            }
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
    [autoPassEnabled, localOffTurnHoldReason, opponentHoldReason]
  );

  const settlePriorityAutomation = useCallback(
    async (currentGame, currentState) => {
      const localAutoResult = await settleLocalStackPriority(currentGame, currentState);
      const opponentAutoResult = await settleOpponentPriority(currentGame, localAutoResult.state);
      return {
        ...opponentAutoResult,
        localAutoPasses: localAutoResult.autoPasses,
        localHoldReason: localAutoResult.holdReason,
      };
    },
    [settleLocalStackPriority, settleOpponentPriority]
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
    localAutoPasses: 0,
    autoPasses: 0,
    autoDeclares: 0,
    phaseAdvances: 0,
    localHoldReason: null,
    holdReason: null,
  }), []);

  const applyStickyViewedCards = useCallback((nextState, { clear = false } = {}) => {
    if (!nextState) {
      if (clear) stickyViewedCardsRef.current = null;
      return nextState;
    }

    if (clear) {
      stickyViewedCardsRef.current = null;
    }

    if (nextState.viewed_cards) {
      stickyViewedCardsRef.current = nextState.viewed_cards;
      return nextState;
    }

    if (!stickyViewedCardsRef.current) return nextState;
    return { ...nextState, viewed_cards: stickyViewedCardsRef.current };
  }, []);

  const finalizeState = useCallback(
    async (
      currentGame,
      currentState,
      {
        message = "",
        allowOpponentAutomation = true,
        allowTrivialAutomation = true,
        clearViewedCards = false,
      } = {}
    ) => {
      let st = currentState;
      const autoResult = allowOpponentAutomation
        ? await settlePriorityAutomation(currentGame, st)
        : await settleNoop(currentGame, st);
      st = autoResult.state;

      const autoResolved = allowTrivialAutomation
        ? await autoResolveTrivialDecisions(
          currentGame,
          st,
          allowOpponentAutomation ? settlePriorityAutomation : settleNoop
        )
        : { state: st, resolved: 0 };
      st = autoResolved.state;
      st = applyStickyViewedCards(st, { clear: clearViewedCards });
      setState(st);
      stateRef.current = st;

      const parts = [];
      if (message) parts.push(message);
      if (allowOpponentAutomation && autoResult.localAutoPasses > 0) {
        parts.push(`passed priority x${autoResult.localAutoPasses}`);
      }
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
      applyStickyViewedCards,
      autoResolveTrivialDecisions,
      settleNoop,
      settlePriorityAutomation,
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
        clearViewedCards: true,
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
          const visibleState = applyStickyViewedCards(st);
          setState(visibleState);
          stateRef.current = visibleState;
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
    [applyStickyViewedCards, finalizeState, game, multiplayer.matchStarted, setStatus]
  );

  const dispatch = useCallback(
    async (command, successMessage) => {
      if (!game) return;
      const currentDecision = stateRef.current?.decision || null;
      const stopAfterTriggerOrderingSubmit = (
        command?.type === "select_options"
        && isTriggerOrderingDecision(currentDecision)
      );
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
          allowOpponentAutomation: !stopAfterTriggerOrderingSubmit,
          allowTrivialAutomation: !stopAfterTriggerOrderingSubmit,
          clearViewedCards: true,
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
          clearViewedCards: true,
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
      confirmEnabled,
      setConfirmEnabled,
      inspectorDebug,
      setInspectorDebug,
      triggerOrderingState: activeTriggerOrderingState,
      moveTriggerOrderingItem,
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
      dispatch, cancelDecision, refresh, autoPassEnabled, holdRule, confirmEnabled, inspectorDebug,
      activeTriggerOrderingState, moveTriggerOrderingItem,
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

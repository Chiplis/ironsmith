import { createContext, useContext, useState, useCallback, useRef, useMemo, useEffect } from "react";
import { useWasmGame } from "@/hooks/useWasmGame";
import { usePeerLobby } from "@/hooks/usePeerLobby";
import { emitSyncFailureNotice } from "@/lib/ui-notices";
import { cardsMeetingThresholdFromStats, loadSemanticStats } from "@/lib/semanticCache";
import { priorityHoldReason } from "@/lib/priority-automation";
import {
  buildTriggerOrderingKey,
  defaultTriggerOrderingOrder,
  isTriggerOrderingDecision,
  normalizeTriggerOrderingOrder,
} from "@/lib/trigger-ordering";

const GameContext = createContext(null);
const TARGET_SUBMIT_CANCEL_DEBOUNCE_MS = 250;

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

function normalizeMultiplayerTarget(target) {
  if (!target || typeof target !== "object") return target;
  if (target.kind === "player") {
    return {
      kind: "player",
      player: Number(target.player),
    };
  }
  if (target.kind === "object") {
    return {
      kind: "object",
      object: Number(target.object),
    };
  }
  return target;
}

function normalizeAttackTargetInput(target, declaration = null) {
  if (target && typeof target === "object") {
    if (target.kind === "player") {
      return {
        kind: "player",
        player: Number(target.player),
      };
    }
    if (target.kind === "planeswalker") {
      return {
        kind: "planeswalker",
        object: Number(target.object),
      };
    }
  }

  if (declaration && typeof declaration === "object") {
    if (declaration.target_player != null) {
      return {
        kind: "player",
        player: Number(declaration.target_player),
      };
    }
    if (declaration.target_battlefield != null) {
      return {
        kind: "planeswalker",
        object: Number(declaration.target_battlefield),
      };
    }
  }

  return null;
}

function normalizeAttackerDeclaration(declaration) {
  if (!declaration || typeof declaration !== "object") return declaration;

  const target = normalizeAttackTargetInput(declaration.target, declaration);
  return {
    creature: Number(declaration.creature ?? declaration.attacker),
    target,
  };
}

function normalizeBlockerDeclaration(declaration) {
  if (!declaration || typeof declaration !== "object") return declaration;

  return {
    blocker: Number(declaration.blocker),
    blocking: Number(declaration.blocking ?? declaration.attacker),
  };
}

function serializeMultiplayerCommand(command, _currentState) {
  if (!command || typeof command !== "object") return command;

  if (command.type === "priority_action") {
    const actions = Array.isArray(_currentState?.decision?.actions)
      ? _currentState.decision.actions
      : [];
    const normalizedIndex = Number(command.action_index);
    const action = actions.find((candidate) => Number(candidate?.index) === normalizedIndex);
    if (!action?.action_ref) {
      throw new Error(`Missing priority action ref for action index ${normalizedIndex}`);
    }
    return {
      type: "priority_action",
      action_ref: action.action_ref,
    };
  }

  if (command.type === "select_options") {
    return {
      type: "select_options",
      option_indices: (command.option_indices || []).map((optionIndex) => Number(optionIndex)),
    };
  }

  if (command.type === "select_objects") {
    return {
      type: "select_objects",
      object_ids: (command.object_ids || []).map((objectId) => Number(objectId)),
    };
  }

  if (command.type === "select_targets") {
    return {
      type: "select_targets",
      targets: (command.targets || []).map(normalizeMultiplayerTarget),
    };
  }

  if (command.type === "number_choice") {
    return {
      type: "number_choice",
      value: Number(command.value),
    };
  }

  if (command.type === "declare_attackers") {
    return {
      type: "declare_attackers",
      declarations: (command.declarations || []).map(normalizeAttackerDeclaration),
    };
  }

  if (command.type === "declare_blockers") {
    return {
      type: "declare_blockers",
      declarations: (command.declarations || []).map(normalizeBlockerDeclaration),
    };
  }

  if (command.type === "cancel_decision") {
    return { type: "cancel_decision" };
  }

  return command;
}

function resolveSyncedCommand(command) {
  if (!command || typeof command !== "object") return command;

  if (command.type === "priority_action" && command.action_ref) {
    return {
      type: "priority_action",
      action_ref: command.action_ref,
    };
  }

  if (command.type === "priority_action" && command.action_index != null) {
    return {
      type: "priority_action",
      action_index: Number(command.action_index),
    };
  }

  if (command.type === "select_options" && Array.isArray(command.option_indices)) {
    return {
      type: "select_options",
      option_indices: command.option_indices.map((optionIndex) => Number(optionIndex)),
    };
  }

  if (command.type === "select_objects" && Array.isArray(command.object_ids)) {
    return {
      type: "select_objects",
      object_ids: command.object_ids.map((objectId) => Number(objectId)),
    };
  }

  if (command.type === "select_targets" && Array.isArray(command.targets)) {
    return {
      type: "select_targets",
      targets: command.targets.map(normalizeMultiplayerTarget),
    };
  }

  if (command.type === "number_choice") {
    return {
      type: "number_choice",
      value: Number(command.value),
    };
  }

  if (command.type === "declare_attackers" && Array.isArray(command.declarations)) {
    return {
      type: "declare_attackers",
      declarations: command.declarations.map(normalizeAttackerDeclaration),
    };
  }

  if (command.type === "declare_blockers" && Array.isArray(command.declarations)) {
    return {
      type: "declare_blockers",
      declarations: command.declarations.map(normalizeBlockerDeclaration),
    };
  }

  if (command.type === "cancel_decision") {
    return { type: "cancel_decision" };
  }

  return command;
}

function summarizeDecision(decision) {
  if (!decision || typeof decision !== "object") return null;

  const summary = {
    kind: String(decision.kind || ""),
    player: decision.player == null ? null : Number(decision.player),
    source_name: decision.source_name ? String(decision.source_name) : null,
    reason: decision.reason ? String(decision.reason) : null,
  };

  if (Array.isArray(decision.requirements)) {
    summary.requirements = decision.requirements.length;
  }
  if (Array.isArray(decision.options)) {
    summary.options = decision.options.length;
  }
  if (Array.isArray(decision.candidates)) {
    summary.candidates = decision.candidates.length;
  }
  if (Array.isArray(decision.actions)) {
    summary.actions = decision.actions.length;
  }

  return summary;
}

function summarizeCommand(command) {
  if (!command || typeof command !== "object") return null;

  const summary = {
    type: String(command.type || ""),
  };

  if (Array.isArray(command.targets)) {
    summary.targets = command.targets.length;
  }
  if (Array.isArray(command.option_indices)) {
    summary.option_indices = [...command.option_indices];
  }
  if (Array.isArray(command.object_ids)) {
    summary.object_ids = [...command.object_ids];
  }
  if (Array.isArray(command.declarations)) {
    summary.declarations = command.declarations.length;
  }
  if (command.action_index != null) {
    summary.action_index = Number(command.action_index);
  }
  if (command.value != null) {
    summary.value = Number(command.value);
  }

  return summary;
}

function isDecisionCommandCompatible(decision, command) {
  if (!decision || !command) return false;
  if (command.type === "cancel_decision") {
    return true;
  }

  switch (decision.kind) {
    case "priority":
      return command.type === "priority_action";
    case "targets":
      return command.type === "select_targets";
    case "select_options":
    case "modes":
    case "hybrid_choice":
      return command.type === "select_options";
    case "select_objects":
      return command.type === "select_objects";
    case "number":
      return command.type === "number_choice";
    case "attackers":
      return command.type === "declare_attackers";
    case "blockers":
      return command.type === "declare_blockers";
    default:
      return false;
  }
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
  const recentTargetSubmitRef = useRef({
    inFlight: false,
    expiresAt: -Infinity,
  });

  const armTargetSubmitDebounce = useCallback(() => {
    const now = performance.now();
    recentTargetSubmitRef.current = {
      inFlight: true,
      expiresAt: now + TARGET_SUBMIT_CANCEL_DEBOUNCE_MS,
    };
  }, []);

  const settleTargetSubmitDebounce = useCallback(() => {
    const now = performance.now();
    recentTargetSubmitRef.current = {
      inFlight: false,
      expiresAt: now + TARGET_SUBMIT_CANCEL_DEBOUNCE_MS,
    };
  }, []);

  const clearTargetSubmitDebounce = useCallback(() => {
    recentTargetSubmitRef.current = {
      inFlight: false,
      expiresAt: -Infinity,
    };
  }, []);

  const shouldSuppressImmediateCancel = useCallback(() => {
    const { inFlight, expiresAt } = recentTargetSubmitRef.current;
    return inFlight || expiresAt > performance.now();
  }, []);

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
    (decision, currentState) => (
      priorityHoldReason({
        autoPassEnabled,
        holdRule,
        decision,
        currentState,
        perspectiveMode: "opponent",
      })
    ),
    [autoPassEnabled, holdRule]
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
        manualResolveOnLocalStack: true,
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
        manualResolveOnLocalStack: true,
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
      console.debug("[ironsmith] finalize:state", {
        message,
        allow_opponent_automation: allowOpponentAutomation,
        allow_trivial_automation: allowTrivialAutomation,
        auto_result: {
          local_auto_passes: autoResult.localAutoPasses,
          auto_passes: autoResult.autoPasses,
          auto_declares: autoResult.autoDeclares,
          phase_advances: autoResult.phaseAdvances,
          local_hold_reason: autoResult.localHoldReason,
          hold_reason: autoResult.holdReason,
        },
        auto_resolved: autoResolved.resolved,
        final_decision: summarizeDecision(st?.decision || null),
        final_stack_size: st?.stack_size ?? null,
        final_stack_preview: Array.isArray(st?.stack_preview) ? st.stack_preview.slice(0, 4) : null,
        final_resolving: st?.resolving_stack_object
          ? {
              id: st.resolving_stack_object.id,
              name: st.resolving_stack_object.name,
            }
          : null,
      });
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
    async (command, successMessage = "", syncContext = null) => {
      const currentGame = gameRef.current;
      if (!currentGame) {
        throw new Error("WASM game is not ready");
      }

      const decisionBefore = summarizeDecision(stateRef.current?.decision || null);
      const resolvedCommand = resolveSyncedCommand(command, stateRef.current);
      const commandSummary = summarizeCommand(resolvedCommand);
      console.debug("[ironsmith] synced dispatch:start", {
        command: commandSummary,
        decision: decisionBefore,
        sync_context: syncContext,
        compatible: isDecisionCommandCompatible(stateRef.current?.decision || null, resolvedCommand),
      });

      try {
        const st = resolvedCommand?.type === "cancel_decision"
          ? await currentGame.cancelDecision()
          : await currentGame.dispatch(resolvedCommand);
        console.debug("[ironsmith] synced dispatch:success", {
          command: commandSummary,
          decision_before: decisionBefore,
          decision_after: summarizeDecision(st?.decision || null),
          sync_context: syncContext,
        });
        return finalizeState(currentGame, st, {
          message: successMessage,
          allowOpponentAutomation: false,
          allowTrivialAutomation: false,
          clearViewedCards: true,
        });
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        emitSyncFailureNotice("Sync failed", errorMessage);
        let decisionAfterError = null;
        try {
          const liveState = await currentGame.uiState();
          decisionAfterError = summarizeDecision(liveState?.decision || null);
        } catch {
          // Best effort only; keep the original failure as the main error.
        }
        console.error("[ironsmith] synced dispatch:failed", {
          error: errorMessage,
          command: commandSummary,
          decision_before: decisionBefore,
          decision_after_error: decisionAfterError,
          sync_context: syncContext,
          compatible_before: isDecisionCommandCompatible(decisionBefore, commandSummary),
          compatible_after_error: isDecisionCommandCompatible(
            decisionAfterError,
            commandSummary
          ),
        });

        try {
          const rollbackState = await currentGame.cancelDecision();
          await finalizeState(currentGame, rollbackState, {
            allowOpponentAutomation: false,
            allowTrivialAutomation: false,
            clearViewedCards: true,
          });
        } catch {
          // Keep the original sync failure.
        }

        throw err;
      }
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
      const isTargetSubmit = command?.type === "select_targets";
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
          if (isTargetSubmit) armTargetSubmitDebounce();
          const syncedCommand = serializeMultiplayerCommand(command, currentState);
          await submitMultiplayerCommand(syncedCommand, successMessage);
          if (isTargetSubmit) settleTargetSubmitDebounce();
        } catch (err) {
          if (isTargetSubmit) clearTargetSubmitDebounce();
          emitSyncFailureNotice(
            "Sync failed",
            err instanceof Error ? err.message : String(err)
          );
          setStatus(`Sync failed: ${err}`, true);
          console.error(err);
        }
        return;
      }

      const decisionBefore = summarizeDecision(stateRef.current?.decision || null);
      const commandSummary = summarizeCommand(command);

      try {
        console.debug("[ironsmith] dispatch:start", {
          command: commandSummary,
          decision: decisionBefore,
          compatible: isDecisionCommandCompatible(stateRef.current?.decision || null, command),
        });

        if (isTargetSubmit) armTargetSubmitDebounce();
        let st = await game.dispatch(command);
        if (isTargetSubmit) settleTargetSubmitDebounce();
        console.debug("[ironsmith] dispatch:success", {
          command: commandSummary,
          decision_before: decisionBefore,
          decision_after: summarizeDecision(st?.decision || null),
          stack_size_after: st?.stack_size ?? null,
          stack_preview_after: Array.isArray(st?.stack_preview) ? st.stack_preview.slice(0, 4) : null,
          resolving_after: st?.resolving_stack_object
            ? {
              id: st.resolving_stack_object.id,
              name: st.resolving_stack_object.name,
            }
            : null,
        });
        await finalizeState(game, st, {
          message: successMessage,
          allowOpponentAutomation: !stopAfterTriggerOrderingSubmit,
          allowTrivialAutomation: !stopAfterTriggerOrderingSubmit,
          clearViewedCards: true,
        });
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        let decisionAfterError = null;
        if (isTargetSubmit) clearTargetSubmitDebounce();
        try {
          const liveState = await game.uiState();
          decisionAfterError = summarizeDecision(liveState?.decision || null);
        } catch {
          // Best effort only; keep the original dispatch failure.
        }
        console.error("[ironsmith] dispatch:failed", {
          error: errorMessage,
          command: commandSummary,
          decision_before: decisionBefore,
          decision_after_error: decisionAfterError,
          compatible_before: isDecisionCommandCompatible(decisionBefore, commandSummary),
          compatible_after_error: isDecisionCommandCompatible(
            decisionAfterError,
            commandSummary
          ),
        });

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
    [
      armTargetSubmitDebounce,
      clearTargetSubmitDebounce,
      finalizeState,
      game,
      multiplayer.matchStarted,
      setStatus,
      settleTargetSubmitDebounce,
      submitMultiplayerCommand,
    ]
  );

  const cancelDecision = useCallback(
    async () => {
      if (!game) return;
      if (shouldSuppressImmediateCancel()) {
        return;
      }
      if (multiplayer.matchStarted) {
        const currentState = stateRef.current;
        if (!currentState?.decision) {
          setStatus("No pending decision to cancel", true);
          return;
        }
        if (currentState.decision.player !== currentState.perspective) {
          setStatus("Waiting for the active player");
          return;
        }
        try {
          await submitMultiplayerCommand({ type: "cancel_decision" }, "Decision cancelled");
        } catch (err) {
          emitSyncFailureNotice(
            "Sync failed",
            err instanceof Error ? err.message : String(err)
          );
          setStatus(`Cancel failed: ${err}`, true);
          console.error(err);
        }
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
    [
      finalizeState,
      game,
      multiplayer.matchStarted,
      setStatus,
      shouldSuppressImmediateCancel,
      submitMultiplayerCommand,
    ]
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

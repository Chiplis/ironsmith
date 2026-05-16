import { createContext, useContext, useState, useCallback, useRef, useMemo, useEffect } from "react";
import { useWasmGame } from "@/hooks/useWasmGame";
import { usePeerLobby } from "@/hooks/usePeerLobby";
import { emitSyncFailureNotice } from "@/lib/ui-notices";
import { cardsMeetingThresholdFromStats, loadSemanticStats } from "@/lib/semanticCache";
import {
  buildMultiplayerSmartAutoPass,
  priorityHoldReason,
} from "@/lib/priority-automation";
import { createWasmInteractionGate } from "@/lib/wasmInteractionGate";
import {
  describeDecisionCommandMismatch,
  findPriorityActionForCommand,
  isDecisionCommandCompatible,
  priorityCommandForAction,
} from "@/lib/sync-commands";
import {
  buildTriggerOrderingKey,
  defaultTriggerOrderingOrder,
  isTriggerOrderingDecision,
  normalizeTriggerOrderingOrder,
} from "@/lib/trigger-ordering";
import { DEFAULT_UI_FONT, uiFontStack } from "@/lib/ui-fonts";
import { hexToRgbString } from "@/lib/player-colors";
import { samePlayerId } from "@/lib/player-display";

const GameContext = createContext(null);
const TARGET_SUBMIT_CANCEL_DEBOUNCE_MS = 250;
const UI_FONT_STORAGE_KEY = "ironsmith.uiFont";
const PLAYER_ACCENTS_STORAGE_KEY = "ironsmith.playerAccentOverrides";
const DEFAULT_PHASE_ACCENT = "#876221";
const PHASE_ACCENT_STORAGE_KEY = "ironsmith.phaseAccent";

function normalizeHexColor(color) {
  const raw = String(color || "").trim();
  const rgb = hexToRgbString(raw);
  if (!rgb) return null;
  return raw.startsWith("#") ? raw.toLowerCase() : `#${raw.toLowerCase()}`;
}

function readStoredPlayerAccentOverrides() {
  if (typeof window === "undefined") return {};
  try {
    const parsed = JSON.parse(window.localStorage.getItem(PLAYER_ACCENTS_STORAGE_KEY) || "{}");
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) return {};
    return Object.fromEntries(
      Object.entries(parsed)
        .filter(([key, value]) => Number.isFinite(Number(key)) && hexToRgbString(value))
        .map(([key, value]) => [String(Number(key)), normalizeHexColor(value)])
    );
  } catch {
    return {};
  }
}

function readStoredPhaseAccent() {
  if (typeof window === "undefined") return DEFAULT_PHASE_ACCENT;
  return normalizeHexColor(window.localStorage.getItem(PHASE_ACCENT_STORAGE_KEY)) || DEFAULT_PHASE_ACCENT;
}

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

  if (!isDecisionCommandCompatible(_currentState?.decision || null, command)) {
    throw new Error(
      describeDecisionCommandMismatch(_currentState?.decision || null, command),
    );
  }

  if (command.type === "priority_action") {
    const action = findPriorityActionForCommand(_currentState?.decision || null, command);
    if (!action?.action_ref) {
      throw new Error("Priority action is no longer available");
    }
    const syncedCommand = {
      type: "priority_action",
      action_ref: action.action_ref,
    };
    if (action.object_id != null) {
      syncedCommand.object_id = Number(action.object_id);
    }
    return syncedCommand;
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

  if (command.type === "text_choice") {
    return {
      type: "text_choice",
      value: String(command.value ?? ""),
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

  if (command.type === "forfeit_player") {
    return {
      type: "forfeit_player",
      player: Number(command.player),
      reason: String(command.reason || "forfeit"),
      timeout_ms: command.timeout_ms == null ? undefined : Number(command.timeout_ms),
      deadline_started_at_ms: command.deadline_started_at_ms == null
        ? undefined
        : Number(command.deadline_started_at_ms),
      deadline_at_ms: command.deadline_at_ms == null ? undefined : Number(command.deadline_at_ms),
      claimed_at_ms: command.claimed_at_ms == null ? undefined : Number(command.claimed_at_ms),
      basis_sequence: command.basis_sequence == null ? undefined : Number(command.basis_sequence),
      match_clock_hash: command.match_clock_hash == null
        ? undefined
        : String(command.match_clock_hash),
      remaining_ms: command.remaining_ms == null ? undefined : Number(command.remaining_ms),
      disconnected_peer_id: command.disconnected_peer_id == null
        ? undefined
        : String(command.disconnected_peer_id),
      disconnect_timeout_ms: command.disconnect_timeout_ms == null
        ? undefined
        : Number(command.disconnect_timeout_ms),
      disconnected_at_ms: command.disconnected_at_ms == null
        ? undefined
        : Number(command.disconnected_at_ms),
      auto_forfeit_at_ms: command.auto_forfeit_at_ms == null
        ? undefined
        : Number(command.auto_forfeit_at_ms),
      disconnect_certificate: command.disconnect_certificate,
    };
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

  if (command.type === "forfeit_player") {
    return {
      type: "forfeit_player",
      player: Number(command.player),
      reason: String(command.reason || "forfeit"),
      timeout_ms: command.timeout_ms == null ? undefined : Number(command.timeout_ms),
      deadline_started_at_ms: command.deadline_started_at_ms == null
        ? undefined
        : Number(command.deadline_started_at_ms),
      deadline_at_ms: command.deadline_at_ms == null ? undefined : Number(command.deadline_at_ms),
      claimed_at_ms: command.claimed_at_ms == null ? undefined : Number(command.claimed_at_ms),
      basis_sequence: command.basis_sequence == null ? undefined : Number(command.basis_sequence),
      match_clock_hash: command.match_clock_hash == null
        ? undefined
        : String(command.match_clock_hash),
      remaining_ms: command.remaining_ms == null ? undefined : Number(command.remaining_ms),
      disconnected_peer_id: command.disconnected_peer_id == null
        ? undefined
        : String(command.disconnected_peer_id),
      disconnect_timeout_ms: command.disconnect_timeout_ms == null
        ? undefined
        : Number(command.disconnect_timeout_ms),
      disconnected_at_ms: command.disconnected_at_ms == null
        ? undefined
        : Number(command.disconnected_at_ms),
      auto_forfeit_at_ms: command.auto_forfeit_at_ms == null
        ? undefined
        : Number(command.auto_forfeit_at_ms),
      disconnect_certificate: command.disconnect_certificate,
    };
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

function readDispatchPerf(state) {
  return state && typeof state === "object" && state.__perf ? state.__perf : null;
}

function recordPerfEvent(label, payload) {
  if (typeof window === "undefined") return;
  const bucket = Array.isArray(window.__ironsmithPerfEvents)
    ? window.__ironsmithPerfEvents
    : [];
  bucket.push({
    label,
    payload,
    recorded_at_ms: performance.now(),
  });
  window.__ironsmithPerfEvents = bucket.slice(-100);
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
  if (command.player != null) {
    summary.player = Number(command.player);
  }
  if (command.reason != null) {
    summary.reason = String(command.reason);
  }

  return summary;
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
  const [uiFont, setUiFont] = useState(() => {
    if (typeof window === "undefined") return DEFAULT_UI_FONT;
    return window.localStorage.getItem(UI_FONT_STORAGE_KEY) || DEFAULT_UI_FONT;
  });
  const [playerAccentOverrides, setPlayerAccentOverrides] = useState(readStoredPlayerAccentOverrides);
  const [phaseAccent, setPhaseAccentRaw] = useState(readStoredPhaseAccent);
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
  const multiplayerAutoPassAttemptRef = useRef("");
  const stickyViewedCardsRef = useRef(null);
  const stickyGameOverRef = useRef(null);
  const queuedSyncedCancelRef = useRef(false);
  const recentTargetSubmitRef = useRef({
    inFlight: false,
    expiresAt: -Infinity,
  });
  const wasmInteractionGateRef = useRef(createWasmInteractionGate());
  // External UI-only auto-pass gate. The mobile phase-strip writes a closure here that
  // returns a hold-reason string when the user has set a stop on the current phase.
  // Returning a non-empty string suppresses *local* auto-pass without touching the engine
  // or multiplayer sync — opponent priority and explicit user actions are unaffected.
  const externalAutoPassGateRef = useRef(null);
  const setExternalAutoPassGate = useCallback((gate) => {
    externalAutoPassGateRef.current = typeof gate === "function" ? gate : null;
  }, []);

  useEffect(() => {
    const nextFont = String(uiFont || DEFAULT_UI_FONT).trim() || DEFAULT_UI_FONT;
    const stack = uiFontStack(nextFont);
    document.documentElement.style.setProperty("--ironsmith-ui-font", stack);
    window.localStorage.setItem(UI_FONT_STORAGE_KEY, nextFont);
  }, [uiFont]);

  useEffect(() => {
    window.localStorage.setItem(PLAYER_ACCENTS_STORAGE_KEY, JSON.stringify(playerAccentOverrides));
  }, [playerAccentOverrides]);

  useEffect(() => {
    const normalizedColor = normalizeHexColor(phaseAccent) || DEFAULT_PHASE_ACCENT;
    const rgb = hexToRgbString(normalizedColor) || "135, 98, 33";
    document.documentElement.style.setProperty("--ironsmith-phase-accent", normalizedColor);
    document.documentElement.style.setProperty("--ironsmith-phase-accent-rgb", rgb);
    window.localStorage.setItem(PHASE_ACCENT_STORAGE_KEY, normalizedColor);
  }, [phaseAccent]);

  const setPhaseAccent = useCallback((color) => {
    const normalizedColor = normalizeHexColor(color);
    if (!normalizedColor) return;
    setPhaseAccentRaw(normalizedColor);
  }, []);

  const setPlayerAccentOverride = useCallback((playerId, color) => {
    const numericPlayerId = Number(playerId);
    const normalizedColor = normalizeHexColor(color);
    if (!Number.isFinite(numericPlayerId) || !normalizedColor) return;
    setPlayerAccentOverrides((current) => ({
      ...current,
      [String(numericPlayerId)]: normalizedColor,
    }));
  }, []);

  const runWasmInteraction = useCallback(
    (task) => wasmInteractionGateRef.current.run(task),
    []
  );

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
    if (state?.viewed_cards) {
      stickyViewedCardsRef.current = state.viewed_cards;
    }
  }, [state]);

  const setPeerState = useCallback((nextState) => {
    if (nextState?.viewed_cards) {
      stickyViewedCardsRef.current = nextState.viewed_cards;
    }
    setState(nextState);
  }, []);

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
        return { state: currentState, autoPasses: 0, holdReason: null, trace: [] };
      }

      if (multiplayerActiveRef.current || !autoPassEnabled) {
        return { state: currentState, autoPasses: 0, holdReason: null, trace: [] };
      }

      let st = currentState;
      let autoPasses = 0;
      let holdReason = null;
      const trace = [];

      for (let i = 0; i < 4; i++) {
        if (!st?.decision || st.decision.kind !== "priority" || !samePlayerId(st.decision.player, st.perspective)) {
          break;
        }
        if (st.active_player !== st.perspective) {
          break;
        }

        holdReason = localTurnHoldReason(st.decision, st);
        if (holdReason) break;

        const externalReason = externalAutoPassGateRef.current
          ? externalAutoPassGateRef.current(st)
          : null;
        if (externalReason) {
          holdReason = String(externalReason);
          break;
        }

        const passAction = (st.decision.actions || []).find((action) => action.kind === "pass_priority");
        if (!passAction) {
          holdReason = "no pass action available";
          break;
        }
        if (passAction.label && passAction.label !== "Pass priority") {
          holdReason = "custom pass action";
          break;
        }

        const stepStartedAt = performance.now();
        const decisionBefore = summarizeDecision(st?.decision || null);
        st = await currentGame.dispatch(priorityCommandForAction(passAction));
        autoPasses += 1;
        const elapsedMs = performance.now() - stepStartedAt;
        const workerPerf = readDispatchPerf(st);
        trace.push({
          kind: "local_auto_pass",
          iteration: i + 1,
          elapsed_ms: elapsedMs,
          decision_before: decisionBefore,
          decision_after: summarizeDecision(st?.decision || null),
          stack_size_after: st?.stack_size ?? null,
          worker_round_trip_ms: workerPerf?.worker_round_trip_ms ?? null,
          worker: workerPerf,
        });

        if (Number(st?.stack_size || 0) <= 0) break;
      }

      return { state: st, autoPasses, holdReason, trace };
    },
    [autoPassEnabled, localTurnHoldReason]
  );

  const settleOpponentPriority = useCallback(
    async (currentGame, currentState) => {
      if (!currentState) {
        return {
          state: currentState,
          autoPasses: 0,
          autoDeclares: 0,
          phaseAdvances: 0,
          holdReason: null,
          trace: [],
        };
      }

      if (multiplayerActiveRef.current || !autoPassEnabled) {
        return {
          state: currentState,
          autoPasses: 0,
          autoDeclares: 0,
          phaseAdvances: 0,
          holdReason: null,
          trace: [],
        };
      }

      let st = currentState;
      let autoPasses = 0;
      let autoDeclares = 0;
      let phaseAdvances = 0;
      let holdReason = null;
      const trace = [];

      for (let i = 0; i < 24; i++) {
        while (
          st
          && st.decision
          && (
            !samePlayerId(st.decision.player, st.perspective)
            || st.active_player !== st.perspective
          )
        ) {
          if (st.decision.kind === "priority") {
            const isLocalOffTurnPriority = samePlayerId(st.decision.player, st.perspective);
            const passAction = (st.decision.actions || []).find((a) => a.kind === "pass_priority");
            if (!passAction) { holdReason = "no pass action available"; break; }
            const isCustomPassAction = !!passAction.label && passAction.label !== "Pass priority";
            if (!isLocalOffTurnPriority && isCustomPassAction) {
              if (autoPasses >= 80) { holdReason = "auto-pass safety limit reached"; break; }
              const stepStartedAt = performance.now();
              const decisionBefore = summarizeDecision(st?.decision || null);
              st = await currentGame.dispatch(priorityCommandForAction(passAction));
              autoPasses += 1;
              const elapsedMs = performance.now() - stepStartedAt;
              const workerPerf = readDispatchPerf(st);
              trace.push({
                kind: "opponent_auto_pass_custom",
                iteration: autoPasses,
                elapsed_ms: elapsedMs,
                decision_before: decisionBefore,
                decision_after: summarizeDecision(st?.decision || null),
                stack_size_after: st?.stack_size ?? null,
                worker: workerPerf,
              });
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
            const stepStartedAt = performance.now();
            const decisionBefore = summarizeDecision(st?.decision || null);
            st = await currentGame.dispatch(priorityCommandForAction(passAction));
            autoPasses += 1;
            const elapsedMs = performance.now() - stepStartedAt;
            const workerPerf = readDispatchPerf(st);
            trace.push({
              kind: isLocalOffTurnPriority ? "local_off_turn_auto_pass" : "opponent_auto_pass",
              iteration: autoPasses,
              elapsed_ms: elapsedMs,
              decision_before: decisionBefore,
              decision_after: summarizeDecision(st?.decision || null),
              stack_size_after: st?.stack_size ?? null,
              worker: workerPerf,
            });
            continue;
          }
          if (autoDeclares >= 40) { holdReason = "auto-declare safety limit reached"; break; }
          if (st.decision.kind === "attackers") {
            const declarations = defaultOpponentAttackerDeclarations(st.decision);
            const stepStartedAt = performance.now();
            const decisionBefore = summarizeDecision(st?.decision || null);
            st = await currentGame.dispatch({ type: "declare_attackers", declarations });
            autoDeclares += 1;
            const elapsedMs = performance.now() - stepStartedAt;
            const workerPerf = readDispatchPerf(st);
            trace.push({
              kind: "auto_declare_attackers",
              iteration: autoDeclares,
              elapsed_ms: elapsedMs,
              decision_before: decisionBefore,
              decision_after: summarizeDecision(st?.decision || null),
              worker: workerPerf,
            });
            continue;
          }
          if (st.decision.kind === "blockers") {
            const stepStartedAt = performance.now();
            const decisionBefore = summarizeDecision(st?.decision || null);
            st = await currentGame.dispatch({ type: "declare_blockers", declarations: [] });
            autoDeclares += 1;
            const elapsedMs = performance.now() - stepStartedAt;
            const workerPerf = readDispatchPerf(st);
            trace.push({
              kind: "auto_declare_blockers",
              iteration: autoDeclares,
              elapsed_ms: elapsedMs,
              decision_before: decisionBefore,
              decision_after: summarizeDecision(st?.decision || null),
              worker: workerPerf,
            });
            continue;
          }
          holdReason = "opponent has non-priority decision";
          break;
        }
        if (holdReason) break;
        if (!st || st.game_over || st.decision) break;
        if (phaseAdvances >= 24) { holdReason = "phase auto-advance safety limit reached"; break; }
        const before = `${st.turn_number}|${st.phase}|${st.step}|${st.priority_player}|${st.stack_size}`;
        const advanceStartedAt = performance.now();
        await currentGame.advancePhase();
        const advancePhaseMs = performance.now() - advanceStartedAt;
        phaseAdvances += 1;
        const uiStateStartedAt = performance.now();
        st = await currentGame.uiState();
        const uiStateMs = performance.now() - uiStateStartedAt;
        const after = `${st.turn_number}|${st.phase}|${st.step}|${st.priority_player}|${st.stack_size}`;
        trace.push({
          kind: "phase_advance",
          iteration: phaseAdvances,
          advance_phase_ms: advancePhaseMs,
          ui_state_ms: uiStateMs,
          state_before: before,
          state_after: after,
          worker: readDispatchPerf(st),
        });
        if (before === after) { holdReason = "advance phase made no progress"; break; }
      }

      return { state: st, autoPasses, autoDeclares, phaseAdvances, holdReason, trace };
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
        trace: [
          ...(localAutoResult.trace || []),
          ...(opponentAutoResult.trace || []),
        ],
      };
    },
    [settleLocalStackPriority, settleOpponentPriority]
  );

  const autoResolveTrivialDecisions = useCallback(
    async (currentGame, currentState, settle) => {
      let resolved = 0;
      let st = currentState;
      const trace = [];
      while (resolved < 50 && st && st.decision) {
        const auto = tryBuildAutoResolveCommand(st.decision);
        if (!auto) break;
        try {
          const dispatchStartedAt = performance.now();
          const decisionBefore = summarizeDecision(st?.decision || null);
          st = await currentGame.dispatch(auto.cmd);
          resolved++;
          const dispatchMs = performance.now() - dispatchStartedAt;
          const dispatchWorker = readDispatchPerf(st);
          const settleStartedAt = performance.now();
          const settleResult = await settle(currentGame, st);
          const settleMs = performance.now() - settleStartedAt;
          st = settleResult.state;
          trace.push({
            kind: "trivial_auto_resolve",
            iteration: resolved,
            label: auto.label,
            dispatch_ms: dispatchMs,
            settle_ms: settleMs,
            decision_before: decisionBefore,
            decision_after: summarizeDecision(st?.decision || null),
            dispatch_worker: dispatchWorker,
            settle_trace: settleResult.trace || [],
          });
        } catch (err) {
          console.warn("Auto-resolve failed:", err);
          break;
        }
      }
      return { state: st, resolved, trace };
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
    trace: [],
  }), []);

  const applyStickyViewedCards = useCallback((nextState, { clear = false } = {}) => {
    if (!nextState) {
      if (clear) {
        stickyViewedCardsRef.current = null;
        stickyGameOverRef.current = null;
      }
      return nextState;
    }

    if (clear) {
      stickyViewedCardsRef.current = null;
      stickyGameOverRef.current = null;
    }

    if (nextState.game_over) {
      stickyGameOverRef.current = nextState.game_over;
    } else if (nextState.decision) {
      stickyGameOverRef.current = null;
    }

    let visibleState = nextState;
    if (nextState.viewed_cards) {
      stickyViewedCardsRef.current = nextState.viewed_cards;
    } else if (stickyViewedCardsRef.current) {
      visibleState = { ...visibleState, viewed_cards: stickyViewedCardsRef.current };
    }

    if (!visibleState.game_over && !visibleState.decision && stickyGameOverRef.current) {
      visibleState = { ...visibleState, game_over: stickyGameOverRef.current };
    }

    return visibleState;
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
        publishState = true,
      } = {}
    ) => {
      const finalizeStartedAt = performance.now();
      let st = currentState;
      const settleStartedAt = performance.now();
      const autoResult = allowOpponentAutomation
        ? await settlePriorityAutomation(currentGame, st)
        : await settleNoop(currentGame, st);
      const settlePriorityMs = performance.now() - settleStartedAt;
      st = autoResult.state;

      const trivialResolveStartedAt = performance.now();
      const autoResolved = allowTrivialAutomation
        ? await autoResolveTrivialDecisions(
            currentGame,
            st,
            allowOpponentAutomation ? settlePriorityAutomation : settleNoop
          )
        : { state: st, resolved: 0, trace: [] };
      const trivialAutoResolveMs = performance.now() - trivialResolveStartedAt;
      st = autoResolved.state;
      const stickyStartedAt = performance.now();
      st = applyStickyViewedCards(st, { clear: clearViewedCards });
      const applyStickyViewedCardsMs = performance.now() - stickyStartedAt;
      const totalFinalizeMs = performance.now() - finalizeStartedAt;
      const finalizePerfPayload = {
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
          trace: autoResult.trace || [],
        },
        auto_resolved: autoResolved.resolved,
        auto_resolve_trace: autoResolved.trace || [],
        final_decision: summarizeDecision(st?.decision || null),
        final_stack_size: st?.stack_size ?? null,
        final_stack_preview: Array.isArray(st?.stack_preview) ? st.stack_preview.slice(0, 4) : null,
        final_resolving: st?.resolving_stack_object
          ? {
              id: st.resolving_stack_object.id,
              name: st.resolving_stack_object.name,
            }
          : null,
        perf: {
          settle_priority_ms: settlePriorityMs,
          trivial_auto_resolve_ms: trivialAutoResolveMs,
          apply_sticky_viewed_cards_ms: applyStickyViewedCardsMs,
          total_finalize_ms: totalFinalizeMs,
        },
      };
      console.info("[ironsmith] finalize:state", finalizePerfPayload);
      recordPerfEvent("finalize:state", finalizePerfPayload);
      if (publishState) {
        setState(st);
        stateRef.current = st;
      }

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
        !samePlayerId(st?.decision?.player, st?.perspective)
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

      let liveStateBefore = null;
      try {
        liveStateBefore = await currentGame.uiState();
      } catch {
        liveStateBefore = stateRef.current;
      }

      const currentStateBefore = liveStateBefore || stateRef.current;
      const currentDecisionBefore = currentStateBefore?.decision || null;
      const decisionBefore = summarizeDecision(currentDecisionBefore);
      const resolvedCommand = resolveSyncedCommand(command, currentStateBefore);
      const commandSummary = summarizeCommand(resolvedCommand);
      const compatibleBefore = isDecisionCommandCompatible(
        currentDecisionBefore,
        resolvedCommand,
      );
      const dispatchStartedAt = performance.now();
      console.debug("[ironsmith] synced dispatch:start", {
        command: commandSummary,
        decision: decisionBefore,
        sync_context: syncContext,
        compatible: compatibleBefore,
      });

      try {
        if (!compatibleBefore) {
          const err = new Error(
            describeDecisionCommandMismatch(currentDecisionBefore, resolvedCommand),
          );
          err.syncedNeedsResync = true;
          throw err;
        }

        let st;
        if (resolvedCommand?.type === "cancel_decision") {
          st = await currentGame.cancelDecision();
        } else if (resolvedCommand?.type === "forfeit_player") {
          if (typeof currentGame.forfeitPlayer !== "function") {
            throw new Error("WASM game does not support forfeits");
          }
          st = await currentGame.forfeitPlayer(Number(resolvedCommand.player));
        } else {
          st = await currentGame.dispatch(resolvedCommand);
        }
        const workerRoundTripMs = performance.now() - dispatchStartedAt;
        const workerPerf = readDispatchPerf(st);
        const syncedDispatchSuccessPayload = {
          command: commandSummary,
          decision_before: decisionBefore,
          decision_after: summarizeDecision(st?.decision || null),
          sync_context: syncContext,
          perf: {
            worker_round_trip_ms: workerRoundTripMs,
            worker_to_main_transfer_ms: workerPerf
              ? Math.max(0, workerRoundTripMs - Number(workerPerf.totalWorkerMs || 0))
              : null,
            worker: workerPerf,
          },
        };
        console.info("[ironsmith] synced dispatch:success", syncedDispatchSuccessPayload);
        recordPerfEvent("synced dispatch:success", syncedDispatchSuccessPayload);
        const finalizeStartedAt = performance.now();
        const finalized = await finalizeState(currentGame, st, {
          message: successMessage,
          allowOpponentAutomation: false,
          allowTrivialAutomation: false,
          clearViewedCards: true,
          publishState: syncContext?.publishState !== false,
        });
        const finalizeMs = performance.now() - finalizeStartedAt;
        const syncedDispatchTimingPayload = {
          command: commandSummary,
          sync_context: syncContext,
          worker_round_trip_ms: workerRoundTripMs,
          finalize_ms: finalizeMs,
          total_to_finalize_ms: performance.now() - dispatchStartedAt,
        };
        console.info("[ironsmith] synced dispatch:timing", syncedDispatchTimingPayload);
        recordPerfEvent("synced dispatch:timing", syncedDispatchTimingPayload);
        if (typeof window !== "undefined" && typeof window.requestAnimationFrame === "function") {
          const paintRequestedAt = performance.now();
          window.requestAnimationFrame(() => {
            const syncedDispatchPaintPayload = {
              command: commandSummary,
              sync_context: syncContext,
              post_finalize_to_next_paint_ms: performance.now() - paintRequestedAt,
              total_to_next_paint_ms: performance.now() - dispatchStartedAt,
            };
            console.info("[ironsmith] synced dispatch:paint", syncedDispatchPaintPayload);
            recordPerfEvent("synced dispatch:paint", syncedDispatchPaintPayload);
          });
        }
        return finalized;
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
          compatible_before: compatibleBefore,
          compatible_after_error: isDecisionCommandCompatible(
            decisionAfterError,
            commandSummary
          ),
        });

        let rollbackApplied = false;
        if (!err?.syncedNeedsResync && compatibleBefore) {
          try {
            const rollbackState = await currentGame.cancelDecision();
            await finalizeState(currentGame, rollbackState, {
              allowOpponentAutomation: false,
              allowTrivialAutomation: false,
              clearViewedCards: true,
            });
            rollbackApplied = true;
          } catch {
            // Keep the original sync failure.
          }
        }

        const errorToThrow = err && typeof err === "object"
          ? err
          : new Error(String(err));
        errorToThrow.syncedRollbackApplied = rollbackApplied;
        throw errorToThrow;
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
    startHostedMatch: rawStartHostedMatch,
    updateLobbyDeck,
    startRematchSideboarding,
    updateRematchDecks,
    readyForRematch,
    submitMultiplayerCommand,
    submitMultiplayerAddCardCheat,
    exportAuditTranscript,
  } = usePeerLobby({
    game,
    state,
    setState: setPeerState,
    setStatus,
    applySyncedCommand,
  });

  const startHostedMatch = useCallback(
    () => runWasmInteraction(() => rawStartHostedMatch()),
    [rawStartHostedMatch, runWasmInteraction]
  );

  useEffect(() => {
    multiplayerActiveRef.current = multiplayer.matchStarted;
  }, [multiplayer.matchStarted]);

  useEffect(() => {
    if (!multiplayer.matchStarted) {
      queuedSyncedCancelRef.current = false;
      return;
    }
    if (!queuedSyncedCancelRef.current || multiplayer.submittingAction) {
      return;
    }

    const currentState = stateRef.current;
    const currentDecision = currentState?.decision || null;
    if (
      !currentDecision
      || !samePlayerId(currentDecision.player, currentState?.perspective)
      || !currentState?.cancelable
      || currentDecision.kind === "priority"
    ) {
      queuedSyncedCancelRef.current = false;
      return;
    }

    queuedSyncedCancelRef.current = false;
    submitMultiplayerCommand({ type: "cancel_decision" }, "Decision cancelled").catch((err) => {
      emitSyncFailureNotice(
        "Sync failed",
        err instanceof Error ? err.message : String(err)
      );
      setStatus(`Cancel failed: ${err}`, true);
      console.error(err);
    });
  }, [multiplayer.matchStarted, multiplayer.submittingAction, setStatus, submitMultiplayerCommand]);

  useEffect(() => {
    if (!game || typeof game.setAutoCleanupDiscard !== "function") return;
    game
      .setAutoCleanupDiscard(autoPassEnabled && !multiplayer.matchStarted)
      .catch((err) => console.warn("setAutoCleanupDiscard failed:", err));
  }, [autoPassEnabled, game, multiplayer.matchStarted]);

  useEffect(() => {
    if (!multiplayer.matchStarted) {
      multiplayerAutoPassAttemptRef.current = "";
      return;
    }
    if (multiplayer.submittingAction) return;

    const currentState = state;
    const result = buildMultiplayerSmartAutoPass({
      autoPassEnabled,
      holdRule,
      decision: currentState?.decision || null,
      currentState,
    });

    if (!result.command) {
      multiplayerAutoPassAttemptRef.current = "";
      return;
    }

    const decision = currentState?.decision || null;
    const passKey = [
      currentState?.snapshot_id ?? "",
      currentState?.turn_number ?? "",
      currentState?.phase ?? "",
      currentState?.step ?? "",
      currentState?.priority_player ?? "",
      currentState?.stack_size ?? "",
      decision?.player ?? "",
      result.command.action_index,
    ].join("|");

    if (multiplayerAutoPassAttemptRef.current === passKey) return;
    multiplayerAutoPassAttemptRef.current = passKey;

    let syncedCommand;
    try {
      syncedCommand = serializeMultiplayerCommand(result.command, currentState);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      queueMicrotask(() => setStatus(`Auto-pass failed: ${message}`, true));
      console.error(err);
      return;
    }

    submitMultiplayerCommand(syncedCommand, "Auto-passed priority").catch((err) => {
      const message = err instanceof Error ? err.message : String(err);
      emitSyncFailureNotice("Auto-pass failed", message);
      setStatus(`Auto-pass failed: ${message}`, true);
      console.error(err);
    });
  }, [
    autoPassEnabled,
    holdRule,
    multiplayer.matchStarted,
    multiplayer.submittingAction,
    setStatus,
    state,
    submitMultiplayerCommand,
  ]);

  const refresh = useCallback(
    async (message) => {
      if (!game) return;
      try {
        if (multiplayer.matchStarted && multiplayer.role === "client") {
          const visibleState = applyStickyViewedCards(stateRef.current);
          setState(visibleState);
          stateRef.current = visibleState;
          if (message) setStatus(message);
          return;
        }
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
    [
      applyStickyViewedCards,
      finalizeState,
      game,
      multiplayer.matchStarted,
      multiplayer.role,
      setStatus,
    ]
  );

  const dispatch = useCallback(
    async (command, successMessage) => {
      if (!game) return;
      return runWasmInteraction(async () => {
        const isTargetSubmit = command?.type === "select_targets";
        const currentDecision = stateRef.current?.decision || null;
        const stopAfterTriggerOrderingSubmit = (
          command?.type === "select_options"
          && isTriggerOrderingDecision(currentDecision)
        );
        if (multiplayer.matchStarted) {
          let currentState = stateRef.current;
          if (multiplayer.role !== "client") {
            try {
              const liveState = await game.uiState();
              currentState = applyStickyViewedCards(liveState, { clear: true });
              setState(currentState);
              stateRef.current = currentState;
            } catch {
              // Fall back to the last rendered state; compatibility checks below still guard submission.
            }
          }
          if (!currentState?.decision) {
            setStatus("No pending decision to submit", true);
            return;
          }
          if (!samePlayerId(currentState.decision.player, currentState.perspective)) {
            setStatus("Waiting for the active player");
            return;
          }
          if (!isDecisionCommandCompatible(currentState.decision, command)) {
            setStatus(describeDecisionCommandMismatch(currentState.decision, command), true);
            try {
              if (multiplayer.role !== "client") {
                const liveState = await game.uiState();
                const visibleState = applyStickyViewedCards(liveState, { clear: true });
                setState(visibleState);
                stateRef.current = visibleState;
              }
            } catch {
              // Keep the stale-command status; the next normal sync/resync will refresh state.
            }
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

          const dispatchStartedAt = performance.now();
          if (isTargetSubmit) armTargetSubmitDebounce();
          let st = await game.dispatch(command);
          const workerRoundTripMs = performance.now() - dispatchStartedAt;
          if (isTargetSubmit) settleTargetSubmitDebounce();
          const workerPerf = readDispatchPerf(st);
          const dispatchSuccessPayload = {
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
            perf: {
              worker_round_trip_ms: workerRoundTripMs,
              worker_to_main_transfer_ms: workerPerf
                ? Math.max(0, workerRoundTripMs - Number(workerPerf.totalWorkerMs || 0))
                : null,
              worker: workerPerf,
            },
          };
          console.info("[ironsmith] dispatch:success", dispatchSuccessPayload);
          recordPerfEvent("dispatch:success", dispatchSuccessPayload);
          const finalizeStartedAt = performance.now();
          await finalizeState(game, st, {
            message: successMessage,
            allowOpponentAutomation: !stopAfterTriggerOrderingSubmit,
            allowTrivialAutomation: !stopAfterTriggerOrderingSubmit,
            clearViewedCards: true,
          });
          const finalizeMs = performance.now() - finalizeStartedAt;
          const dispatchTimingPayload = {
            command: commandSummary,
            worker_round_trip_ms: workerRoundTripMs,
            finalize_ms: finalizeMs,
            total_to_finalize_ms: performance.now() - dispatchStartedAt,
          };
          console.info("[ironsmith] dispatch:timing", dispatchTimingPayload);
          recordPerfEvent("dispatch:timing", dispatchTimingPayload);
          if (typeof window !== "undefined" && typeof window.requestAnimationFrame === "function") {
            const paintRequestedAt = performance.now();
            window.requestAnimationFrame(() => {
              const dispatchPaintPayload = {
                command: commandSummary,
                post_finalize_to_next_paint_ms: performance.now() - paintRequestedAt,
                total_to_next_paint_ms: performance.now() - dispatchStartedAt,
              };
              console.info("[ironsmith] dispatch:paint", dispatchPaintPayload);
              recordPerfEvent("dispatch:paint", dispatchPaintPayload);
            });
          }
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
      });
    },
    [
      armTargetSubmitDebounce,
      applyStickyViewedCards,
      clearTargetSubmitDebounce,
      finalizeState,
      game,
      multiplayer.matchStarted,
      multiplayer.role,
      runWasmInteraction,
      setStatus,
      settleTargetSubmitDebounce,
      submitMultiplayerCommand,
    ]
  );

  const cancelDecision = useCallback(
    async () => {
      if (!game) return;
      return runWasmInteraction(async () => {
        if (multiplayer.matchStarted) {
          const currentState = stateRef.current;
          if (!currentState?.decision) {
            setStatus("No pending decision to cancel", true);
            return;
          }
          if (!samePlayerId(currentState.decision.player, currentState.perspective)) {
            setStatus("Waiting for the active player");
            return;
          }
          if (multiplayer.submittingAction || shouldSuppressImmediateCancel()) {
            queuedSyncedCancelRef.current = true;
            setStatus("Cancel queued while the current action syncs");
            return;
          }
          try {
            queuedSyncedCancelRef.current = false;
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
        if (shouldSuppressImmediateCancel()) {
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
      });
    },
    [
      finalizeState,
      game,
      multiplayer.matchStarted,
      multiplayer.submittingAction,
      runWasmInteraction,
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
      runWasmInteraction,
      dispatch,
      cancelDecision,
      refresh,
      autoPassEnabled,
      setAutoPassEnabled,
      holdRule,
      setHoldRule,
      uiFont,
      setUiFont,
      playerAccentOverrides,
      setPlayerAccentOverride,
      phaseAccent,
      setPhaseAccent,
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
      startRematchSideboarding,
      updateRematchDecks,
      readyForRematch,
      exportAuditTranscript,
      submitMultiplayerCommand,
      submitMultiplayerAddCardCheat,
      setExternalAutoPassGate,
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
      runWasmInteraction,
      dispatch, cancelDecision, refresh, autoPassEnabled, holdRule, uiFont,
      playerAccentOverrides, setPlayerAccentOverride, phaseAccent, setPhaseAccent, inspectorDebug,
      activeTriggerOrderingState, moveTriggerOrderingItem,
      semanticThreshold, setSemanticThreshold, cardsMeetingThreshold,
      logEntries, pushLog,
      multiplayer, canStartHostedMatch, createLobby, joinLobby, leaveLobby, startHostedMatch, updateLobbyDeck,
      startRematchSideboarding, updateRematchDecks, readyForRematch,
      exportAuditTranscript,
      submitMultiplayerCommand,
      submitMultiplayerAddCardCheat,
      setExternalAutoPassGate,
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

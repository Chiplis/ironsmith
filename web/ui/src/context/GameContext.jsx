import { readAutoResolveMode, AUTO_RESOLVE_STORAGE_KEY } from "@/lib/priority-clock";
import { tryBuildAutoResolveCommand } from "@/lib/auto-resolve-decision";
import useLoopShortcut from "./useLoopShortcut";
import { stackArrivalActivity } from "@/lib/stack-arrival-activity";
import { isOwnPassCancellation } from "@/lib/sync-commands.js";
import { useAutomaticHandReveal } from "@/hooks/useAutomaticHandReveal";
import { buildOpponentDecisionCommand } from "@/lib/opponent-decision.js";
import { selectGoldfishPlayer } from "@/lib/goldfish-player.js";
import { createPriorityStops, priorityStopsForPlayer } from "@/lib/priority-stops";
import { useHandManaCosts } from "@/hooks/useHandManaCosts";
import { usePaymentOptions } from "@/hooks/usePaymentOptions";
import { ManaPaymentEditorProvider } from "@/context/ManaPaymentEditorContext";
import { improvePayment } from "@/lib/payment-analysis.js";
import {
  beginActionTrace,
  completeActionTrace,
  markActionStage,
  recordEnginePerf,
  startMainThreadMonitor,
} from "@/lib/action-diagnostics";
import { setJournalPolicy } from "@/lib/engine-journal";
import { isVerifiedMultiplayerSecurityMode } from "@/lib/multiplayer-security";
import { captureEngineRestorePoint, restoreEngineRestorePoint } from "@/lib/engine-restore-point";
import { subscribePriorityAnalysisSnapshots } from "@/lib/priority-analysis-scheduler.js";
import { castingMethodChoiceForAction, finishExplicitCastingMethod } from "@/lib/casting-method-choice";
import { startTransition, useContext, useState, useCallback, useRef, useMemo, useEffect, useSyncExternalStore } from "react";
import { useGameSnapshot } from "@/hooks/useGameSnapshot";
import { useWasmGame } from "@/hooks/useWasmGame";
import { usePeerLobby } from "@/hooks/usePeerLobby";
import { assertCurrentAuditReplayProtocol } from "@/lib/multiplayer-audit";
import {
  applyAuditReplayActionWithGame,
  replayAuditTranscriptWithGame,
  startAuditTranscriptReplayWithGame,
} from "@/lib/audit-replay";
import { emitSyncFailureNotice } from "@/lib/ui-notices";
import { cardsMeetingThresholdFromStats, loadSemanticStats } from "@/lib/semanticCache";
import {
  buildMultiplayerSmartAutoPass,
  findPassPriorityAction,
  priorityHoldReason,
} from "@/lib/priority-automation";
import {
  DEFAULT_WASM_INTERACTION_DEBOUNCE_MS,
  createWasmInteractionGate,
} from "@/lib/wasmInteractionGate";
import {
  describeDecisionCommandMismatch,
  serializePriorityCommand,
  isDecisionCommandCompatible,
  priorityCommandForAction,
  normalizeSelectObjectHiddenRef,
  selectObjectSyncMetadataForCommand,
} from "@/lib/sync-commands";
import {
  buildEffectOrderingKey,
  defaultEffectOrderingOrder,
  isEffectOrderingDecision,
  normalizeEffectOrderingOrder,
} from "@/lib/effect-ordering";
import { DEFAULT_UI_FONT, uiFontStack } from "@/lib/ui-fonts";
import { readFixedStartingBoard, storeFixedStartingBoard } from "@/lib/starting-board";
import { hexToRgbString } from "@/lib/player-colors";
import { samePlayerId } from "@/lib/player-display";
import { createPregameFollower, emptyPregameAction, hotseatFollowTarget, isAutomatedSeat, readFollowActivePlayer, storeFollowActivePlayer } from "@/lib/hotseat-follow";
import { forcedObjectSelectionCommand, localForcedObjectSelectionCommand } from "@/lib/forced-object-selection";

import { GameContext } from "./GameContext.shared";
const TARGET_SUBMIT_CANCEL_DEBOUNCE_MS = 250;
const AUDIT_REPLAY_GATE_WAIT_ATTEMPTS = 8;
const UI_FONT_STORAGE_KEY = "ironsmith.uiFont";
const PLAYER_ACCENTS_STORAGE_KEY = "ironsmith.playerAccentOverrides";

const emptyAuditReplayState = {
  available: false,
  active: false,
  sourceLabel: "",
  currentActionIndex: 0,
  currentActionLabel: "",
  actionCount: 0,
  busy: false,
  error: "",
};

function cloneJson(value) {
  if (value == null) return value;
  return JSON.parse(JSON.stringify(value));
}

function delay(ms) {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

function isInspectorOnlyViewedCards(viewedCards) {
  return Boolean(viewedCards?.inspector_only || viewedCards?.inspectorOnly);
}

function finiteNumberOrNull(value) {
  const numeric = Number(value);
  return Number.isFinite(numeric) ? numeric : null;
}

function playerStateKey(player) {
  return String(player?.id ?? player?.index ?? player?.name ?? "");
}

function stabilizePeerUiState(nextState, previousState) {
  if (
    !nextState
    || !previousState
    || !Array.isArray(nextState.players)
    || !Array.isArray(previousState.players)
  ) {
    return nextState;
  }

  const previousPlayersByKey = new Map(
    previousState.players.map((player) => [playerStateKey(player), player])
  );
  let changed = false;
  const players = nextState.players.map((player) => {
    if (finiteNumberOrNull(player?.hand_size) != null) return player;

    const previousPlayer = previousPlayersByKey.get(playerStateKey(player));
    const previousHandSize = finiteNumberOrNull(previousPlayer?.hand_size);
    if (previousHandSize == null) return player;

    changed = true;
    return {
      ...player,
      hand_size: previousHandSize,
    };
  });

  return changed ? { ...nextState, players } : nextState;
}

async function waitForAuditReplayGate(gate) {
  for (
    let attempt = 0;
    attempt < AUDIT_REPLAY_GATE_WAIT_ATTEMPTS && gate?.isBlocked?.();
    attempt += 1
  ) {
    await delay(DEFAULT_WASM_INTERACTION_DEBOUNCE_MS);
  }
}

function auditReplayActionLabel(action, index) {
  if (!action || Number(index) <= 0) return "Match start";
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
    if (kind === "continue_pregame" || kind === "begin_game") {
      return `${index}. ${actorLabel}: Pregame`;
    }
    return `${index}. ${actorLabel}: ${kind || "Priority action"}`;
  }
  if (command.type === "select_targets") return `${index}. ${actorLabel}: Select targets`;
  if (command.type === "select_options") return `${index}. ${actorLabel}: Select option`;
  if (command.type === "declare_attackers") return `${index}. ${actorLabel}: Declare attackers`;
  if (command.type === "declare_blockers") return `${index}. ${actorLabel}: Declare blockers`;
  return `${index}. ${actorLabel}: ${command.type || "Action"}`;
}

function auditReplayStateForPrepared(prepared, overrides = {}) {
  if (!prepared) return { ...emptyAuditReplayState, ...overrides };
  return {
    available: true,
    active: false,
    sourceLabel: prepared.sourceLabel,
    currentActionIndex: 0,
    currentActionLabel: "Match start",
    actionCount: prepared.actionCount,
    busy: false,
    error: "",
    ...overrides,
  };
}

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

function decodeAttackTargetChoice(choice) {
  if (choice && typeof choice === "object") {
    if ("Player" in choice) return { kind: "player", player: Number(choice.Player) };
    if ("Planeswalker" in choice)
      return { kind: "planeswalker", object: Number(choice.Planeswalker) };
    if ("Battle" in choice) return { kind: "battle", object: Number(choice.Battle) };
    if (choice.kind === "player") return { kind: "player", player: Number(choice.player) };
    if (choice.kind === "battle") return { kind: "battle", object: Number(choice.object) };
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
    if (firstTarget == null) continue;
    declarations.push({
      creature: Number(option.creature),
      target: decodeAttackTargetChoice(firstTarget),
    });
  }
  return declarations;
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
    return serializePriorityCommand(command, _currentState?.decision,
      objectStableIdMapFromState(_currentState));
  }

  if (command.type === "select_options") {
    return {
      type: "select_options",
      option_indices: (command.option_indices || []).map((optionIndex) => Number(optionIndex)),
    };
  }

  if (command.type === "mana_payment") {
    return {
      type: "mana_payment",
      response: {
        ...(command.response || {}),
        plan_id: command.response?.plan_id == null ? undefined : String(command.response.plan_id),
        request_hash: command.response?.request_hash == null ? undefined : String(command.response.request_hash),
        required_source_ids: (command.response?.required_source_ids || []).map(String),
        excluded_source_ids: (command.response?.excluded_source_ids || []).map(String),
        preserved_source_ids: (command.response?.preserved_source_ids || []).map(String),
      },
    };
  }

  if (command.type === "select_objects") {
    const objectIds = (command.object_ids || []).map((objectId) => Number(objectId));
    const { stableIds, hiddenRefs } = selectObjectSyncMetadataForCommand(
      { ...command, object_ids: objectIds },
      _currentState
    );
    const syncedCommand = {
      type: "select_objects",
      object_ids: objectIds,
    };
    if (stableIds.some((stableId) => stableId != null)) {
      syncedCommand.object_stable_ids = stableIds;
    }
    if (hiddenRefs.some((hiddenRef) => hiddenRef != null)) {
      syncedCommand.object_hidden_refs = hiddenRefs;
    }
    return syncedCommand;
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
    const syncedCommand = {
      type: "priority_action",
      action_ref: command.action_ref,
    };
    if (command.object_id != null || command.objectId != null) {
      syncedCommand.object_id = Number(command.object_id ?? command.objectId);
    }
    if (command.object_stable_id != null || command.objectStableId != null) {
      const stableId = Number(command.object_stable_id ?? command.objectStableId);
      if (Number.isSafeInteger(stableId) && stableId > 0) {
        syncedCommand.object_stable_id = stableId;
      }
    }
    const rawHiddenRef = command.object_hidden_ref ?? command.objectHiddenRef;
    // Opaque commands must reach the paired-identity checker unchanged. The
    // legacy normalizer may complete partial fields or discard a supplied pair.
    const hiddenRef = ["open_exiled_card_for_play", "cast_exiled_card_face_down"].includes(command.action_ref?.kind)
      ? rawHiddenRef : normalizeSelectObjectHiddenRef(rawHiddenRef);
    if (hiddenRef) {
      syncedCommand.object_hidden_ref = hiddenRef;
    }
    return syncedCommand;
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

  if (command.type === "mana_payment" && command.response) {
    return {
      type: "mana_payment",
      response: {
        ...command.response,
        plan_id: command.response.plan_id == null ? undefined : String(command.response.plan_id),
        request_hash: command.response.request_hash == null ? undefined : String(command.response.request_hash),
        required_source_ids: (command.response.required_source_ids || []).map(String),
        excluded_source_ids: (command.response.excluded_source_ids || []).map(String),
        preserved_source_ids: (command.response.preserved_source_ids || []).map(String),
      },
    };
  }

  if (command.type === "select_objects" && Array.isArray(command.object_ids)) {
    const syncedCommand = {
      type: "select_objects",
      object_ids: command.object_ids.map((objectId) => Number(objectId)),
    };
    const stableIds = Array.isArray(command.object_stable_ids)
      ? command.object_stable_ids
      : Array.isArray(command.objectStableIds)
        ? command.objectStableIds
        : [];
    if (stableIds.length > 0) {
      syncedCommand.object_stable_ids = stableIds.map((stableId) => {
        const normalized = Number(stableId);
        return Number.isSafeInteger(normalized) && normalized > 0 ? normalized : null;
      });
    }
    const hiddenRefs = Array.isArray(command.object_hidden_refs)
      ? command.object_hidden_refs
      : Array.isArray(command.objectHiddenRefs)
        ? command.objectHiddenRefs
        : [];
    if (hiddenRefs.length > 0) {
      syncedCommand.object_hidden_refs = hiddenRefs.map(normalizeSelectObjectHiddenRef);
    }
    return syncedCommand;
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
    description: decision.description ? String(decision.description) : null,
    placeholder: decision.placeholder ? String(decision.placeholder) : null,
    require_known_value: Boolean(decision.require_known_value),
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

function rememberCardStableId(map, card) {
  if (!card || typeof card !== "object") return;
  const id = Number(card.id);
  const stableId = Number(card.stable_id ?? card.stableId);
  if (Number.isSafeInteger(id) && id > 0 && Number.isSafeInteger(stableId) && stableId > 0) {
    map.set(id, stableId);
  }
  const memberIds = Array.isArray(card.member_ids) ? card.member_ids : [];
  const memberStableIds = Array.isArray(card.member_stable_ids) ? card.member_stable_ids : [];
  for (let index = 0; index < memberIds.length; index += 1) {
    const memberId = Number(memberIds[index]);
    const memberStableId = Number(memberStableIds[index]);
    if (
      Number.isSafeInteger(memberId)
      && memberId > 0
      && Number.isSafeInteger(memberStableId)
      && memberStableId > 0
    ) {
      map.set(memberId, memberStableId);
    }
  }
}

function objectStableIdMapFromState(state) {
  const map = new Map();
  const zoneKeys = [
    "battlefield",
    "hand_cards",
    "graveyard_cards",
    "exile_cards",
    "command_cards",
    "ante_cards",
  ];
  for (const player of state?.players || []) {
    for (const zoneKey of zoneKeys) {
      for (const card of player?.[zoneKey] || []) {
        rememberCardStableId(map, card);
      }
    }
  }
  for (const stackEntry of state?.stack_preview || []) {
    rememberCardStableId(map, stackEntry);
  }
  return map;
}

function readDispatchPerf(state) {
  return state && typeof state === "object" && state.__perf ? state.__perf : null;
}

// Results that only feed finalizeState (local commands, auto-passes, default
// declarations) skip the engine's priority presentation menu: castable
// highlights and their payment searches, wasted on every priority that is
// passed automatically. finalizeState fetches the full view once, for the
// state it actually publishes.
const automationSnapshots = new WeakSet();

function needsPresentation(state) {
  return automationSnapshots.has(state)
    && state.decision?.kind === "priority"
    && state.decision.analysis_complete === false;
}

async function dispatchForAutomation(game, command) {
  if (typeof game.dispatchForAutomation !== "function") return game.dispatch(command);
  const state = await game.dispatchForAutomation(command);
  if (state && typeof state === "object") automationSnapshots.add(state);
  return state;
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

function describeCommandLabel(command) {
  if (!command || typeof command !== "object") return "action";
  const type = String(command.type || "action");
  const ref = command.action_ref?.kind ? String(command.action_ref.kind) : "";
  return ref && ref !== type ? `${type} · ${ref}` : type;
}

// Close the trace on the frame after the state landed, so the total is
// click-to-pixels rather than click-to-promise.
function completeActionTraceOnPaint(traceId, meta = null) {
  if (typeof window !== "undefined" && typeof window.requestAnimationFrame === "function") {
    const paintRequestedAt = performance.now();
    window.requestAnimationFrame(() => {
      completeActionTrace(traceId, {
        outcome: "ok",
        meta: { ...(meta || {}), to_next_paint_ms: performance.now() - paintRequestedAt },
      });
    });
    return;
  }
  completeActionTrace(traceId, { outcome: "ok", meta });
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

function currentOrderForDecision(effectOrderingState, decision, key = buildEffectOrderingKey(decision)) {
  if (!isEffectOrderingDecision(decision)) return [];
  if (effectOrderingState?.key === key) {
    return normalizeEffectOrderingOrder(effectOrderingState.order, decision);
  }
  return defaultEffectOrderingOrder(decision);
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
  const { state, setState, stateRef, subscribeState, isSnapshotRendered } = useGameSnapshot();
  const [status, setStatusRaw] = useState({ msg: "Loading WASM...", isError: false });
  const [autoPassEnabled, setAutoPassEnabled] = useState(true);
  const [autoResolveMode, setAutoResolveModeRaw] = useState(readAutoResolveMode);
  const autoResolveModeRef = useRef(autoResolveMode);
  const autoResolveEnabled = autoResolveMode !== "off";
  const autoResolveEnabledRef = useRef(autoResolveEnabled);
  const [phasePassing, setPhasePassing] = useState(false);
  const [holdRule, setHoldRule] = useState("never");
  const [priorityStops] = useState(() => createPriorityStops({
    phaseStopsEnabled: autoResolveMode !== "all",
    resolveMode: autoResolveMode,
    storage: typeof window === "undefined" ? null : window.localStorage,
  }));
  const setAutoResolveMode = useCallback(value => {
    const mode = typeof value === "function" ? value(autoResolveModeRef.current) : value;
    const enabled = mode !== "off";
    autoResolveModeRef.current = mode;
    autoResolveEnabledRef.current = enabled;
    priorityStops.setResolveMode(mode);
    priorityStops.resume(stateRef.current);
    priorityStops.setPhaseStopsEnabled(mode !== "all");
    priorityStops.observe(stateRef.current);
    setAutoResolveModeRaw(mode);
    try { window.localStorage.setItem(AUTO_RESOLVE_STORAGE_KEY, mode); } catch { /* Storage may be unavailable. */ }
  }, [priorityStops, stateRef]);
  const setAutoResolveEnabled = useCallback(value => {
    const enabled = typeof value === "function" ? value(autoResolveEnabledRef.current) : value;
    setAutoResolveMode(enabled ? "safe" : "off");
  }, [setAutoResolveMode]);
  const pendingStackArrivals = useSyncExternalStore(stackArrivalActivity.subscribe, stackArrivalActivity.getSnapshot);
  const priorityStopsState = useSyncExternalStore(priorityStops.subscribe, priorityStops.getSnapshot);
  const cyclePriorityStop = useCallback((key, player) => priorityStops.cycleStop(key, stateRef.current, player), [priorityStops, stateRef]);
  const holdPostActionPriority = useCallback(() => priorityStops.hold(stateRef.current), [priorityStops, stateRef]);
  const pausePostActionPriority = useCallback(id => priorityStops.pauseWindow(id), [priorityStops]);
  const resumePostActionPriority = useCallback(id => priorityStops.resumeWindow(id), [priorityStops]);
  useEffect(() => subscribeState(priorityStops.observe), [subscribeState, priorityStops]);
  useEffect(() => { priorityStops.reset(); }, [game, priorityStops]);
  const [surrenderRequested, setSurrenderRequested] = useState(false);
  const [fixedStartingBoard, setFixedStartingBoard] = useState(readFixedStartingBoard);
  const [uiFont, setUiFont] = useState(() => {
    if (typeof window === "undefined") return DEFAULT_UI_FONT;
    return window.localStorage.getItem(UI_FONT_STORAGE_KEY) || DEFAULT_UI_FONT;
  });
  const [playerAccentOverrides, setPlayerAccentOverrides] = useState(readStoredPlayerAccentOverrides);
  const [inspectorDebug, setInspectorDebug] = useState(false);
  const [effectOrderingState, setEffectOrderingState] = useState({ key: "", order: [] });
  const [semanticThreshold, setSemanticThresholdRaw] = useState(96);
  const [cardsMeetingThreshold, setCardsMeetingThreshold] = useState(0);
  const [semanticStats, setSemanticStats] = useState(null);
  const [auditReplayState, setAuditReplayState] = useState(emptyAuditReplayState);
  const logRef = useRef([]);
  const [logEntries, setLogEntries] = useState([]);
  const gameRef = useRef(game);
  const semanticThresholdRef = useRef(semanticThreshold);
  const auditReplaySessionRef = useRef(null);
  const auditReplayPreparedRef = useRef(null);
  const multiplayerActiveRef = useRef(false);
  const multiplayerAutomationAttemptRef = useRef("");
  const multiplayerSubmitInFlightRef = useRef(false);
  const [multiplayerSubmissionRevision, setMultiplayerSubmissionRevision] = useState(0);
  const finishMultiplayerSubmission = useCallback(() => {
    if (!multiplayerSubmitInFlightRef.current) return;
    multiplayerSubmitInFlightRef.current = false;
    // The next decision can render while this guard is still held. Releasing
    // a ref alone cannot wake the automation effect that skipped that render.
    setMultiplayerSubmissionRevision(revision => revision + 1);
  }, []);
  const stickyViewedCardsRef = useRef(null);
  const stickyGameOverRef = useRef(null);
  const queuedSyncedCancelRef = useRef(false);
  // User input cancels ranking slices and suggestions waiting to be submitted.
  // Keep manual ownership across payment component remounts.
  const backgroundDispatchGenerationRef = useRef(0);
  const manuallyControlledPaymentRef = useRef(null);
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
  const [followActivePlayer, setFollowActivePlayerRaw] = useState(readFollowActivePlayer);
  const followActivePlayerRef = useRef(followActivePlayer);
  const pregameFollowerRef = useRef(createPregameFollower());
  const setFollowActivePlayer = useCallback((value) => {
    const enabled = Boolean(value);
    followActivePlayerRef.current = enabled;
    setFollowActivePlayerRaw(enabled);
    storeFollowActivePlayer(enabled);
  }, []);
  // Goldfished seats in local games: their decisions are answered
  // automatically and the hotseat view never hands the table to them.
  const [automatedSeats, setAutomatedSeats] = useState(() => new Set());
  const automatedSeatsRef = useRef(automatedSeats);
  const toggleAutomatedSeat = useCallback((player) => {
    const seat = Number(player);
    if (!Number.isInteger(seat)) return;
    setAutomatedSeats(current => {
      const next = new Set(current);
      if (next.has(seat)) next.delete(seat);
      else next.add(seat);
      automatedSeatsRef.current = next;
      return next;
    });
  }, []);
  // Bumped whenever hotseat automation hands the table to another seat; the
  // handoff banner keys its animation on `seq`.
  const [perspectiveHandoff, setPerspectiveHandoff] = useState(null);
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
    storeFixedStartingBoard(fixedStartingBoard);
  }, [fixedStartingBoard]);

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
      // Status shares the board context; publishing it urgently would still
      // force the entire table to render synchronously after an engine action.
      startTransition(() => {
        setStatusRaw({ msg, isError });
        pushLog(msg, isError);
      });
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
    if (state?.viewed_cards && !isInspectorOnlyViewedCards(state.viewed_cards)) {
      stickyViewedCardsRef.current = state.viewed_cards;
    }
  }, [state]);

  const setPeerState = useCallback((nextState, { replacePresentation = false } = {}) => {
    const visibleState = stabilizePeerUiState(nextState, stateRef.current);
    if (stateRef.current?.loop_shortcut?.stepping && !visibleState?.loop_shortcut) {
      priorityStops.holdCurrentPriority(visibleState);
    }
    if (replacePresentation) {
      stickyViewedCardsRef.current = null;
      stickyGameOverRef.current = visibleState?.game_over || null;
    }
    if (visibleState?.viewed_cards && !isInspectorOnlyViewedCards(visibleState.viewed_cards)) {
      stickyViewedCardsRef.current = visibleState.viewed_cards;
    }
    setState(visibleState);
    stateRef.current = visibleState;
  }, [priorityStops, setState, stateRef]);

  const moveEffectOrderingItem = useCallback((position, direction) => {
    const decision = stateRef.current?.decision || null;
    if (!isEffectOrderingDecision(decision)) return;
    const key = buildEffectOrderingKey(decision);

    setEffectOrderingState((current) => {
      const currentOrder = current.key === key
        ? normalizeEffectOrderingOrder(current.order, decision)
        : defaultEffectOrderingOrder(decision);
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
  }, [stateRef]);

  const activeEffectOrderingState = useMemo(() => {
    const decision = state?.decision || null;
    if (!isEffectOrderingDecision(decision)) return null;

    const key = buildEffectOrderingKey(decision);
    return {
      key,
      order: currentOrderForDecision(effectOrderingState, decision, key),
    };
  }, [state?.decision, effectOrderingState]);

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
      priorityStops.observe(currentState),
      isAutomatedSeat(automatedSeatsRef.current, decision?.player) ? null : priorityHoldReason({
        autoPassEnabled,
        holdRule,
        decision,
        currentState,
        perspectiveMode: "opponent",
      })
    ),
    [autoPassEnabled, holdRule, priorityStops]
  );

  const localTurnHoldReason = useCallback(
    (decision, currentState, canAct) => (
      priorityStops.stopReason(currentState, canAct) || priorityHoldReason({
        autoPassEnabled,
        holdRule,
        decision,
        currentState,
        perspectiveMode: "local",
        requireNonEmptyStack: true,
        manualResolveOnLocalStack: true,
      })
    ),
    [autoPassEnabled, holdRule, priorityStops]
  );

  const localOffTurnHoldReason = useCallback(
    (decision, currentState, canAct) => (
      priorityStops.stopReason(currentState, canAct) || priorityHoldReason({
        autoPassEnabled,
        holdRule,
        decision,
        currentState,
        perspectiveMode: "local",
        manualResolveOnLocalStack: true,
      })
    ),
    [autoPassEnabled, holdRule, priorityStops]
  );

  // Automation snapshots skip the presentation menu, so an "I can act" stop
  // asks the engine directly. Null keeps the stop until the analysis finishes.
  const resolveStopCanAct = useCallback(async (currentGame, currentState) => {
    const known = currentState?.decision?.can_act ?? null;
    if (known != null || !priorityStops.awaitsCanAct(currentState, known)
        || typeof currentGame?.getPriorityCanAct !== "function") return known;
    try {
      return await currentGame.getPriorityCanAct();
    } catch {
      return null;
    }
  }, [priorityStops]);

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

        holdReason = localTurnHoldReason(st.decision, st, await resolveStopCanAct(currentGame, st));
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
        st = await dispatchForAutomation(currentGame, priorityCommandForAction(passAction));
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
    [autoPassEnabled, localTurnHoldReason, resolveStopCanAct]
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

      if (multiplayerActiveRef.current) {
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
      const playerCount = Math.max(2, Array.isArray(st?.players) ? st.players.length : 2);
      const maxAutoPasses = Math.max(80, playerCount * playerCount * 16);
      const maxPhaseAdvances = Math.max(24, playerCount * 12);
      const maxSettleIterations = maxPhaseAdvances + playerCount * 4;

      let perspectiveSwitches = 0;
      for (let i = 0; i < maxSettleIterations; i++) {
        while (st && st.decision) {
          const pregameTarget = pregameFollowerRef.current(st);
          const emptyPregame = emptyPregameAction(st);
          if (emptyPregame) {
            if (autoPasses >= maxAutoPasses) { holdReason = "auto-pass safety limit reached"; break; }
            const decisionBefore = summarizeDecision(st.decision);
            st = await dispatchForAutomation(currentGame, priorityCommandForAction(emptyPregame));
            autoPasses += 1;
            trace.push({ kind: "empty_pregame", decision_before: decisionBefore,
              decision_after: summarizeDecision(st?.decision || null) });
            continue;
          }
          const followTarget = typeof currentGame.setPerspective === "function"
            ? pregameTarget ?? (autoPassEnabled && followActivePlayerRef.current ? hotseatFollowTarget(st, automatedSeatsRef.current) : null)
            : null;
          if (followTarget !== null && perspectiveSwitches < playerCount * 4) {
            const decisionBefore = summarizeDecision(st.decision);
            await currentGame.setPerspective(followTarget);
            st = await currentGame.uiState();
            perspectiveSwitches += 1;
            const handoffTurn = st?.turn_number ?? null;
            const handoffReason = st?.decision?.kind === "priority" ? "turn" : st?.decision?.kind || "decision";
            setPerspectiveHandoff(current => ({
              player: followTarget,
              turn: handoffTurn,
              reason: handoffReason,
              seq: (current?.seq || 0) + 1,
            }));
            trace.push({ kind: "hotseat_follow", player: followTarget, decision_before: decisionBefore,
              decision_after: summarizeDecision(st?.decision || null) });
            continue;
          }
          if (!autoPassEnabled) break;
          if (samePlayerId(st.decision.player, st.perspective)
            && samePlayerId(st.active_player, st.perspective)) break;
          if (st.decision.kind === "priority") {
            const isLocalOffTurnPriority = samePlayerId(st.decision.player, st.perspective);
            const passAction = (st.decision.actions || []).find((a) => a.kind === "pass_priority");
            if (!passAction) {
              if (isLocalOffTurnPriority) { holdReason = "no pass action available"; break; }
              if (autoPasses >= maxAutoPasses) { holdReason = "auto-pass safety limit reached"; break; }
              const command = await buildOpponentDecisionCommand(st, currentGame);
              if (!command) { holdReason = "no automatic priority response available"; break; }
              const decisionBefore = summarizeDecision(st.decision);
              st = await dispatchForAutomation(currentGame, command);
              autoPasses += 1;
              trace.push({ kind: "opponent_required_priority_choice", decision_before: decisionBefore,
                decision_after: summarizeDecision(st?.decision || null) });
              continue;
            }
            const isCustomPassAction = !!passAction.label && passAction.label !== "Pass priority";
            if (!isLocalOffTurnPriority && isCustomPassAction) {
              if (autoPasses >= maxAutoPasses) { holdReason = "auto-pass safety limit reached"; break; }
              const stepStartedAt = performance.now();
              const decisionBefore = summarizeDecision(st?.decision || null);
              st = await dispatchForAutomation(currentGame, priorityCommandForAction(passAction));
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
              ? localOffTurnHoldReason(st.decision, st, await resolveStopCanAct(currentGame, st))
              : opponentHoldReason(st.decision, st);
            if (holdReason) break;
            if (passAction.label && passAction.label !== "Pass priority") {
              holdReason = "custom pass action";
              break;
            }
            if (autoPasses >= maxAutoPasses) { holdReason = "auto-pass safety limit reached"; break; }
            const stepStartedAt = performance.now();
            const decisionBefore = summarizeDecision(st?.decision || null);
            st = await dispatchForAutomation(currentGame, priorityCommandForAction(passAction));
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
          if (samePlayerId(st.decision.player, st.perspective)) {
            holdReason = "local decision requires input";
            break;
          }
          if (autoDeclares >= 40) { holdReason = "auto-declare safety limit reached"; break; }
          if (st.decision.kind === "attackers") {
            const command = currentGame.getDefaultSelectionCommand
              ? await currentGame.getDefaultSelectionCommand()
              : { type: "declare_attackers", declarations: defaultOpponentAttackerDeclarations(st.decision) };
            if (!command) { holdReason = "no legal attacker response available"; break; }
            const stepStartedAt = performance.now();
            const decisionBefore = summarizeDecision(st?.decision || null);
            st = await dispatchForAutomation(currentGame, command);
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
            const command = currentGame.getDefaultSelectionCommand
              ? await currentGame.getDefaultSelectionCommand()
              : { type: "declare_blockers", declarations: [] };
            if (!command) { holdReason = "no legal blocker response available"; break; }
            st = await dispatchForAutomation(currentGame, command);
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
          const command = await buildOpponentDecisionCommand(st, currentGame);
          if (!command) { holdReason = "no automatic response available"; break; }
          const decisionBefore = summarizeDecision(st.decision);
          st = command.type === "cancel_decision"
            ? await currentGame.cancelDecision()
            : await dispatchForAutomation(currentGame, command);
          autoDeclares += 1;
          trace.push({ kind: "opponent_effect_decision", decision_before: decisionBefore,
            decision_after: summarizeDecision(st?.decision || null) });
          continue;
        }
        if (holdReason || !autoPassEnabled) break;
        if (!st || st.game_over || st.decision) break;
        if (phaseAdvances >= maxPhaseAdvances) { holdReason = "phase auto-advance safety limit reached"; break; }
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
    [autoPassEnabled, localOffTurnHoldReason, opponentHoldReason, resolveStopCanAct]
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
      while (holdRule !== "always" && autoPassEnabled && resolved < 50 && st && st.decision) {
        // Peer answers must go through submitMultiplayerCommand after the
        // snapshot is published, so the actor can provide verified openings.
        const forcedObjects = !multiplayerActiveRef.current
          ? forcedObjectSelectionCommand(st.decision)
          : null;
        const auto = forcedObjects
          ? { cmd: forcedObjects, label: "Auto: only required card selected" }
          : tryBuildAutoResolveCommand(st.decision);
        if (!auto) break;
        try {
          const dispatchStartedAt = performance.now();
          const decisionBefore = summarizeDecision(st?.decision || null);
          st = await dispatchForAutomation(currentGame, auto.cmd);
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
    [holdRule, autoPassEnabled]
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
    if (nextState.viewed_cards && !isInspectorOnlyViewedCards(nextState.viewed_cards)) {
      stickyViewedCardsRef.current = nextState.viewed_cards;
    } else if (!nextState.viewed_cards && stickyViewedCardsRef.current) {
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
      const loopOwned = Boolean(st?.loop_shortcut || stateRef.current?.loop_shortcut);
      if (loopOwned) {
        allowOpponentAutomation = false;
        allowTrivialAutomation = false;
      }
      if (publishState && stateRef.current?.loop_shortcut?.stepping && !st?.loop_shortcut) {
        priorityStops.holdCurrentPriority(st);
      }
      priorityStops.observe(st);
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
      const presentationStartedAt = performance.now();
      if (needsPresentation(st) && typeof currentGame.uiState === "function") {
        st = await currentGame.uiState();
      }
      const presentationMs = performance.now() - presentationStartedAt;
      const stickyStartedAt = performance.now();
      // Background verification and provisional transactions must not mutate
      // presentation caches before their controller publishes the result.
      if (publishState) st = applyStickyViewedCards(st, { clear: clearViewedCards });
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
          presentation_ms: presentationMs,
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
      setState,
      stateRef,
      applyStickyViewedCards,
      autoResolveTrivialDecisions,
      settleNoop,
      settlePriorityAutomation,
      priorityStops,
      setStatus,
    ]
  );

  const applySyncedCommand = useCallback(
    async (command, successMessage = "", syncOptions = null) => {
      const { preState, runtimeGame, provisional = false, ...syncContext } = syncOptions || {};
      const currentGame = runtimeGame || gameRef.current;
      if (!currentGame) {
        throw new Error("WASM game is not ready");
      }

      let liveStateBefore = null;
      try {
        liveStateBefore = currentGame.isCurrentSnapshot?.(preState)
          ? preState : await currentGame.uiState();
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
        recordEnginePerf(workerPerf);
        markActionStage(null, "engine", { ...syncedDispatchSuccessPayload.perf, sync_context: syncContext });
        const finalizeStartedAt = performance.now();
        const finalized = await finalizeState(currentGame, st, {
          message: successMessage,
          allowOpponentAutomation: false,
          allowTrivialAutomation: false,
          clearViewedCards: true,
          publishState: syncContext?.publishState !== false,
        });
        currentGame.adoptSnapshotVersion?.(finalized, st);
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
        if (!provisional) emitSyncFailureNotice("Sync failed", errorMessage);
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
              publishState: syncContext?.publishState !== false,
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
    [stateRef, finalizeState]
  );

  const {
    matchClockStore,
    multiplayer,
    canStartHostedMatch,
    createLobby,
    joinLobby,
    leaveLobby,
    redeemTournamentInvite,
    startHostedMatch: rawStartHostedMatch,
    updateLobbyDeck,
    startRematchSideboarding,
    updateRematchDeck,
    readyForRematch,
    startRematch,
    submitMultiplayerCommand,
    sendLobbyChat,
    submitMultiplayerAddCardCheat,
    exportAuditTranscript,
    routePeerIdForPlayer,
  } = usePeerLobby({
    game,
    state,
    setState: setPeerState,
    subscribeState,
    setStatus,
    applySyncedCommand,
  });

  const startHostedMatch = useCallback(
    () => runWasmInteraction(() => rawStartHostedMatch()),
    [rawStartHostedMatch, runWasmInteraction]
  );

  useEffect(() => {
    multiplayerActiveRef.current = multiplayer.matchStarted;
    // A local session's journal may carry card identities: they are the
    // player's own, and without them the export cannot rebuild the board. A
    // Trusted match starts every peer with every deck and the shuffle seed, so
    // its journal would disclose other people's hidden cards: it keeps method
    // names and argument shapes only and the bundle reports itself as not
    // replayable. A Verified match's engine only ever holds openings this seat
    // was entitled to see, so its journal discloses nothing the exporter could
    // not already see.
    const redact = multiplayer.matchStarted
      && !isVerifiedMultiplayerSecurityMode(multiplayer.securityMode);
    setJournalPolicy(redact ? "redacted" : "full");
  }, [multiplayer.matchStarted, multiplayer.securityMode]);

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
  }, [stateRef, multiplayer.matchStarted, multiplayer.submittingAction, setStatus, submitMultiplayerCommand]);

  useEffect(() => {
    if (!game || typeof game.setAutoCleanupDiscard !== "function") return;
    game
      .setAutoCleanupDiscard(autoPassEnabled && holdRule !== "always" && !multiplayer.matchStarted)
      .catch((err) => console.warn("setAutoCleanupDiscard failed:", err));
  }, [autoPassEnabled, holdRule, game, multiplayer.matchStarted]);

  useEffect(() => {
    if (!multiplayer.matchStarted) {
      multiplayerAutomationAttemptRef.current = "";
      return;
    }
    if (state?.loop_shortcut || phasePassing || multiplayer.submittingAction || multiplayerSubmitInFlightRef.current) return;
    if (stateRef.current !== state) return;

    const currentState = state;
    const emptyBlockers = autoPassEnabled && holdRule !== "always"
      && samePlayerId(currentState?.decision?.player, currentState?.perspective)
      && ["blockers", "attackers"].includes(currentState?.decision?.kind)
      ? tryBuildAutoResolveCommand(currentState.decision)?.cmd : null;
    const forcedObjects = holdRule === "always" ? null : (emptyBlockers || localForcedObjectSelectionCommand(currentState));
    if (!forcedObjects && (priorityStops.stopReason(currentState)
      || Number(currentState?.stack_size || 0) > 0)) return;
    const result = forcedObjects ? { command: forcedObjects } : buildMultiplayerSmartAutoPass({
      autoPassEnabled,
      holdRule,
      decision: currentState?.decision || null,
      currentState,
    });

    if (!result.command) {
      multiplayerAutomationAttemptRef.current = "";
      return;
    }

    const decision = currentState?.decision || null;
    const attemptKey = [
      currentState?.snapshot_id ?? "",
      currentState?.turn_number ?? "",
      currentState?.phase ?? "",
      currentState?.step ?? "",
      currentState?.priority_player ?? "",
      currentState?.stack_size ?? "",
      decision?.player ?? "",
      result.command.type,
      result.command.action_index,
      result.command.object_ids?.join(",") ?? "",
    ].join("|");

    if (multiplayerAutomationAttemptRef.current === attemptKey) return;
    multiplayerAutomationAttemptRef.current = attemptKey;

    let syncedCommand;
    try {
      syncedCommand = serializeMultiplayerCommand(result.command, currentState);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      queueMicrotask(() => setStatus(`Automatic action failed: ${message}`, true));
      console.error(err);
      return;
    }

    multiplayerSubmitInFlightRef.current = true;
    submitMultiplayerCommand(syncedCommand, forcedObjects
      ? "Auto: only required card selected"
      : "Auto-passed priority")
      .catch((err) => {
        const message = err instanceof Error ? err.message : String(err);
        emitSyncFailureNotice("Automatic action failed", message);
        setStatus(`Automatic action failed: ${message}`, true);
        console.error(err);
      })
      .finally(() => {
        finishMultiplayerSubmission();
      });
  }, [
    autoPassEnabled,
    autoResolveEnabled,
    phasePassing,
    priorityStops,
    priorityStopsState,
    finishMultiplayerSubmission,
    holdRule,
    multiplayer.matchStarted,
    multiplayer.submittingAction,
    multiplayerSubmissionRevision,
    setStatus,
    state,
    stateRef,
    submitMultiplayerCommand,
  ]);

  const refresh = useCallback(
    async (message, { throwOnError = false } = {}) => {
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
        if (throwOnError) throw err;
      }
    },
    [
      setState,
      stateRef,
      applyStickyViewedCards,
      finalizeState,
      game,
      multiplayer.matchStarted,
      multiplayer.role,
      setStatus,
    ]
  );

  useEffect(() => {
    if (!game?.subscribePriorityAnalysis) return;
    return subscribePriorityAnalysisSnapshots({
      game, getState: () => stateRef.current, setState, subscribeState,
    });
  }, [setState, stateRef, subscribeState, game]);

  useHandManaCosts({ game, state, stateRef, setState });

  usePaymentOptions({ game, state, stateRef, setState,
    enabled: samePlayerId(state?.decision?.player, state?.perspective) });

  const automatedAnalysisRevisionRef = useRef(null);
  useEffect(() => {
    const revision = state?.__priority_revision;
    if (multiplayer.matchStarted || state?.decision?.analysis_complete !== true
        || game?.latestPriorityAnalysis?.()?.revision !== revision
        || automatedAnalysisRevisionRef.current === revision) return;
    let timer;
    const resume = () => {
      if (stateRef.current?.__priority_revision !== revision) return;
      if (wasmInteractionGateRef.current.isInFlight()) {
        timer = setTimeout(resume, 25);
        return;
      }
      automatedAnalysisRevisionRef.current = revision;
      void wasmInteractionGateRef.current.runAutomatic(async () => {
        try {
          // The analysis was merged into this revision already; another
          // uiState round trip just rebuilds the snapshot we have in hand.
          await finalizeState(game, stateRef.current);
        } catch (err) {
          setStatus(`Refresh failed: ${err}`, true);
        }
      });
    };
    timer = setTimeout(resume, 0);
    return () => clearTimeout(timer);
  }, [stateRef, game, state?.__priority_revision, state?.decision?.analysis_complete, multiplayer.matchStarted, finalizeState, setStatus]);

  const dispatch = useCallback(
    async (command, successMessage, {
      castingAction = null,
      waitForPaymentReady = false,
      waitForLoopReady = false,
      expectedLoopSnapshotId = null,
      paymentTransactionId = null,
      backgroundGeneration = null,
    } = {}) => {
      if (!game) return;
      if (backgroundGeneration == null && stateRef.current?.decision?.kind === "mana_payment") {
        backgroundDispatchGenerationRef.current += 1;
        manuallyControlledPaymentRef.current = stateRef.current?.mana_payment?.request_hash;
        void game.cancelPaymentAnalysis().catch(() => {});
      }
      const payment = stateRef.current?.mana_payment;
      const backgroundIsCurrent = () => (
        backgroundGeneration == null
        || backgroundDispatchGenerationRef.current === backgroundGeneration
      );
      // Loop continuations must survive the previous action's cooldown and wait
      // for rendering, but become obsolete when another snapshot replaces theirs.
      const loopSnapshotId = expectedLoopSnapshotId ?? stateRef.current?.snapshot_id;
      const runInteraction = waitForLoopReady
        ? (task) => wasmInteractionGateRef.current.runWhenReady(task,
          () => stateRef.current?.snapshot_id === loopSnapshotId && Boolean(stateRef.current?.loop_shortcut),
          isSnapshotRendered)
        : waitForPaymentReady
        ? (task) => wasmInteractionGateRef.current.runWhenReady(task, () => {
          if (!backgroundIsCurrent()) return false;
          const current = stateRef.current;
          return current?.decision?.kind === "mana_payment"
            && samePlayerId(current.decision.player, current.perspective)
            && (paymentTransactionId != null
              ? String(current.mana_payment?.transaction_id) === paymentTransactionId
              : current.mana_payment?.request_hash === payment?.request_hash)
            && (paymentTransactionId != null
              || current.mana_payment?.plan_id === payment?.plan_id)
            && (command.response?.action !== "confirm" || current.mana_payment?.can_confirm !== false);
        })
        : runWasmInteraction;
      return runInteraction(async () => {
        if (!backgroundIsCurrent()) return;
        // Serialized draft edits are tied to the announced transaction rather
        // than a render. Exact confirmations validate their reviewed plan below.
        const exactPaymentConfirmation = command?.type === "mana_payment" && command.response?.action === "confirm";
        if (!isSnapshotRendered() && !exactPaymentConfirmation && paymentTransactionId == null) return;
        if (exactPaymentConfirmation) {
          const liveState = await game.uiState();
          if (String(liveState.mana_payment?.plan_id) !== String(command.response.plan_id)
            || String(liveState.mana_payment?.request_hash) !== String(command.response.request_hash)) {
            await finalizeState(game, liveState);
            setStatus("The payment plan changed. Review it before paying.");
            return;
          }
        }
        const isTargetSubmit = command?.type === "select_targets";
        const currentDecision = stateRef.current?.decision || null;
        const stopAfterEffectOrderingSubmit = (
          command?.type === "select_options"
          && isEffectOrderingDecision(currentDecision)
        );
        if (multiplayer.matchStarted) {
          let currentState = stateRef.current;
          if (multiplayer.role !== "client") {
            try {
              const liveState = await game.uiState();
              if (!backgroundIsCurrent()) return;
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
          if (!samePlayerId(currentState.decision.player, currentState.perspective)
              && !isOwnPassCancellation(command, currentState.perspective)) {
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
          if (!backgroundIsCurrent()) return;
          let multiplayerTraceId = null;
          let priorityCheckpoint = null;
          try {
            if (isTargetSubmit) armTargetSubmitDebounce();
            const syncedCommand = serializeMultiplayerCommand(command, currentState);
            priorityCheckpoint = priorityStops.beforeCommand(command, currentState);
            multiplayerSubmitInFlightRef.current = true;
            multiplayerTraceId = beginActionTrace({
              label: describeCommandLabel(command),
              command: syncedCommand,
              mode: "multiplayer",
            });
            try {
              await submitMultiplayerCommand(syncedCommand, successMessage);
              markActionStage(multiplayerTraceId, "submit returned");
              completeActionTraceOnPaint(multiplayerTraceId);
              // Keep the explicit method choice within this interaction; a
              // render-driven dispatch would be dropped by the input cooldown.
              const nextState = stateRef.current;
              const methodCommand = castingMethodChoiceForAction(nextState?.decision, castingAction);
              if (methodCommand && samePlayerId(nextState.decision.player, nextState.perspective)) {
                await submitMultiplayerCommand(
                  serializeMultiplayerCommand(methodCommand, nextState),
                  successMessage,
                );
              }
            } finally {
              finishMultiplayerSubmission();
            }
            if (isTargetSubmit) settleTargetSubmitDebounce();
          } catch (err) {
            finishMultiplayerSubmission();
            if (isTargetSubmit) clearTargetSubmitDebounce();
            completeActionTrace(multiplayerTraceId, {
              outcome: "failed",
              meta: { error: err instanceof Error ? err.message : String(err) },
            });
            priorityStops.rollbackAction(priorityCheckpoint, stateRef.current);
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
        const localTraceId = beginActionTrace({
          label: describeCommandLabel(command),
          command,
          mode: "local",
        });

        let priorityCheckpoint = null;
        try {
          console.debug("[ironsmith] dispatch:start", {
            command: commandSummary,
            decision: decisionBefore,
            compatible: isDecisionCommandCompatible(stateRef.current?.decision || null, command),
          });

          const dispatchStartedAt = performance.now();
          if (isTargetSubmit) armTargetSubmitDebounce();
          const continuingYield = command.type === "priority_action"
            && command.action_ref?.kind === "pass_priority"
            && (stateRef.current?.pass_intents || []).length > 0;
          const isYield = command.type === "priority_yield" || continuingYield;
          priorityCheckpoint = priorityStops.beforeCommand(command, stateRef.current);
          let st = await dispatchForAutomation(game, command);
          st = await finishExplicitCastingMethod(st, castingAction, (nextCommand) => dispatchForAutomation(game, nextCommand));
          // Goldfish opponents already governed by the automatic hold policy
          // grant their own intention too. Required choices always return to UI.
          if (isYield && !st?.loop_shortcut) {
            const actor = Number(stateRef.current?.perspective);
            for (let pass = 0; pass < 4096; pass++) {
              if (!(st?.pass_intents || []).includes(actor)
                  || st?.decision?.kind !== "priority"
                  || samePlayerId(st.decision.player, actor)
                  || opponentHoldReason(st.decision, st)) break;
              st = await dispatchForAutomation(game, holdRule === "never" ? command
                : priorityCommandForAction(findPassPriorityAction(st.decision)));
            }
          }
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
          recordEnginePerf(workerPerf);
          markActionStage(localTraceId, "engine", dispatchSuccessPayload.perf);
          const finalizeStartedAt = performance.now();
          await finalizeState(game, st, {
            message: successMessage,
            allowOpponentAutomation: !stopAfterEffectOrderingSubmit && !isYield && command.type !== "cancel_priority_yield",
            allowTrivialAutomation: !stopAfterEffectOrderingSubmit && !isYield && command.type !== "cancel_priority_yield",
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
              completeActionTrace(localTraceId, { outcome: "ok", meta: dispatchPaintPayload });
            });
          }
        } catch (err) {
          priorityStops.rollbackAction(priorityCheckpoint, stateRef.current);
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
      setState,
      stateRef,
      armTargetSubmitDebounce,
      isSnapshotRendered,
      applyStickyViewedCards,
      clearTargetSubmitDebounce,
      finalizeState,
      priorityStops,
      finishMultiplayerSubmission,
      opponentHoldReason,
      holdRule,
      game,
      multiplayer.matchStarted,
      multiplayer.role,
      runWasmInteraction,
      setStatus,
      settleTargetSubmitDebounce,
      submitMultiplayerCommand,
    ]
  );

  const loopShortcut = useLoopShortcut({
    state,
    stateRef,
    dispatch,
    multiplayerMatchStarted: Boolean(multiplayer.matchStarted),
    setStatus,
  });
  const userDispatch = loopShortcut.recordingDispatch;

  useEffect(() => {
    if (state?.decision?.kind !== "mana_payment") manuallyControlledPaymentRef.current = null;
  }, [state?.decision?.kind]);

  // The visual drain and this deadline share the same origin. Only submit a
  // normal local pass, and revalidate authoritative state after any input/sync.
  useEffect(() => {
    const window = priorityStopsState.window;
    // Replay owns its exact sequence of passes; the timer must not race it.
    if (!window || state?.loop_shortcut) return;
    // Read live activity too: child layout effects register arrivals before
    // this effect, potentially after this render read the store.
    if (stackArrivalActivity.getSnapshot() > 0) {
      priorityStops.deferWindow(window.id);
      return;
    }
    if (window.pausedAt != null) return;
    // Start only after the snapshot is drawn and every stack arrival settles.
    if (window.startedAt === null) {
      if (state === stateRef.current && isSnapshotRendered() && priorityStops.windowFor(state)) priorityStops.armWindow(window.id);
      return;
    }
    if (!autoPassEnabled || holdRule === "always") {
      priorityStops.hold(stateRef.current);
      return;
    }
    let timer;
    const passWhenReady = () => {
      const current = stateRef.current;
      if (current?.loop_shortcut) return;
      priorityStops.observe(current);
      if (priorityStops.getSnapshot().window?.id !== window.id) return;
      const remaining = window.startedAt + window.duration - performance.now();
      if (remaining > 0) {
        timer = setTimeout(passWhenReady, Math.ceil(remaining));
        return;
      }
      if (wasmInteractionGateRef.current.isBlocked() || !isSnapshotRendered()
        || multiplayer.submittingAction || multiplayerSubmitInFlightRef.current) {
        timer = setTimeout(passWhenReady, 25);
        return;
      }
      const pass = findPassPriorityAction(current?.decision);
      if (!pass || (pass.label && pass.label !== "Pass priority")) {
        priorityStops.hold(current);
        return;
      }
      if (priorityStops.expire(window.id, current)) {
        void userDispatch(priorityCommandForAction(pass), "Passed priority");
      } else if (priorityStops.getSnapshot().window?.id === window.id) {
        timer = setTimeout(passWhenReady, 25);
      }
    };
    timer = setTimeout(passWhenReady, Math.max(0, Math.ceil(window.startedAt + window.duration - performance.now())));
    return () => clearTimeout(timer);
  }, [priorityStops, priorityStopsState.window, autoResolveEnabled, autoPassEnabled, holdRule, userDispatch,
    isSnapshotRendered, multiplayer.submittingAction, stateRef, state, pendingStackArrivals]);

  // Each run sends one authenticated intention. The engine drains authorized
  // priority without intermediate renders and reports retained consent.
  const [resolveAllTick, setResolveAllTick] = useState(0);
  const resolveAllRef = useRef(null);
  const stopResolveAll = useCallback(() => {
    resolveAllRef.current = null;
    setPhasePassing(false);
  }, []);
  useEffect(() => { stopResolveAll(); }, [game, stopResolveAll]);
  // A stop ends the current phase run. It must not switch off auto-pass or
  // the saved auto-resolve preference: the stop itself keeps holding until the
  // player acts, and with default stops a paused run is the normal outcome.
  const pausePassing = useCallback(() => {
    const run = resolveAllRef.current;
    if (!run) return;
    run.cancelRequested = true;
    setPhasePassing(false);
    setResolveAllTick(tick => tick + 1);
  }, []);
  const changeGoldfishPlayer = useCallback(async (playerId) => runWasmInteraction(async () => {
    if (!game || multiplayerActiveRef.current) return;
    stopResolveAll();
    try {
      for (const player of stateRef.current?.pass_intents || []) {
        await dispatchForAutomation(game, { type: "cancel_priority_yield", player });
      }
      const result = await selectGoldfishPlayer(game, playerId,
        command => dispatchForAutomation(game, command));
      const current = await game.uiState();
      priorityStops.holdCurrentPriority(current);
      await finalizeState(game, current, {
        message: result.reached ? `Viewing as player ${playerId}` : "Player switch paused for a required decision",
        allowOpponentAutomation: false,
        allowTrivialAutomation: false,
      });
    } catch (error) {
      const current = await game.uiState();
      priorityStops.holdCurrentPriority(current);
      await finalizeState(game, current, { allowOpponentAutomation: false, allowTrivialAutomation: false });
      setStatus(`Change player failed: ${error}`, true);
    }
  }), [game, runWasmInteraction, stopResolveAll, priorityStops, finalizeState, setStatus, stateRef]);
  // continueThroughOwn (double-click Pay): keep resolving through items this
  // seat adds while the stack resolves; only an opponent's addition pauses.
  const startResolveAll = useCallback(({ continueThroughOwn = false } = {}) => {
    const current = stateRef.current;
    const stackSize = Number(current?.stack_size || 0);
    if (stackSize <= 0) return;
    setAutoPassEnabled(true);
    const pass = findPassPriorityAction(current?.decision);
    if (pass && samePlayerId(current?.decision?.player, current?.perspective)) {
      priorityStops.beforeCommand(priorityCommandForAction(pass), current);
    }
    resolveAllRef.current = {
      turn: current?.turn_number ?? null,
      stackIds: new Set((current.stack_objects || []).map(entry => String(entry.id))),
      continueThroughOwn,
      stackSize,
      passes: 0,
      inFlight: false,
      dispatchedFrom: null,
    };
    setPhasePassing(false);
    setResolveAllTick((tick) => tick + 1);
  }, [stateRef, priorityStops]);

  const beginPhasePassing = useCallback((ignoreStops = false) => {
    if (resolveAllRef.current?.phases) {
      pausePassing();
      return;
    }
    const current = stateRef.current;
    if (!current || current.game_over
      || (ignoreStops && !samePlayerId(current.active_player, current.perspective))
      || current.mana_payment || current.decision?.kind === "mana_payment") return;
    // Explicitly resume automation, but leave phase/step stops intact.
    priorityStops.resume(current);
    // Release the starting stop; Go To honors later stops and End turn bypasses them.
    const pass = findPassPriorityAction(current.decision);
    if (pass && samePlayerId(current.decision?.player, current.perspective)) {
      priorityStops.beforeCommand(priorityCommandForAction(pass), current);
    }
    setAutoPassEnabled(true);
    setHoldRule("never");
    resolveAllRef.current = {
      phases: true,
      ignoreStops,
      turn: current.turn_number ?? null,
      stackSize: Number(current.stack_size || 0),
      passes: 0,
      inFlight: false,
      dispatchedFrom: null,
    };
    setPhasePassing(true);
    setResolveAllTick(tick => tick + 1);
  }, [stateRef, pausePassing, priorityStops]);
  const togglePhasePassing = useCallback(() => beginPhasePassing(true), [beginPhasePassing]);
  const startPhaseAdvance = useCallback(() => beginPhasePassing(false), [beginPhasePassing]);

  useEffect(() => {
    const run = resolveAllRef.current;
    if (state?.loop_shortcut) {
      if (run) stopResolveAll();
      return;
    }
    if (!run || run.inFlight || stateRef.current !== state) return;
    if (!state || state.game_over || state.turn_number !== run.turn) {
      stopResolveAll();
      return;
    }
    if (run.submitted && (run.stopKeys !== JSON.stringify(priorityStopsState.stops)
        || holdRule === "always" || !autoPassEnabled)) run.cancelRequested = true;
    const additions = (state.stack_objects || []).filter(entry => !run.stackIds.has(String(entry.id)));
    if (run.continueThroughOwn && !run.phases && additions.length > 0
      && additions.every(entry => samePlayerId(entry.controller, state.perspective))) {
      // Our own new items join the run; the engine stopped for them, so resume.
      for (const entry of additions) run.stackIds.add(String(entry.id));
      if (run.submitted && !(state.pass_intents || []).includes(Number(state.perspective))
        && state.decision?.kind === "priority" && samePlayerId(state.decision.player, state.perspective)) {
        run.submitted = false;
        run.dispatchedFrom = null;
      }
    }
    if (run.submitted && !(state.pass_intents || []).includes(Number(state.perspective))) {
      // The engine stops at every "I can act" step; only this client knows
      // whether its seat can act there, so it resumes the advance when not.
      const canActStop = run.phases && !run.ignoreStops && !run.cancelRequested
        && state.decision?.kind === "priority" && samePlayerId(state.decision.player, state.perspective)
        && Number(state.stack_size || 0) === 0 && !priorityStops.stopReason(state, false);
      const canAct = state.decision?.can_act ?? null;
      if (canActStop && canAct == null) return;
      if (canActStop && canAct === false) {
        run.submitted = false;
        run.dispatchedFrom = null;
      } else {
        priorityStops.holdCurrentPriority(state);
        stopResolveAll();
        return;
      }
    }
    if (run.cancelRequested) {
      if (!run.submitted) { stopResolveAll(); return; }
      const timer = setTimeout(() => {
        if (wasmInteractionGateRef.current.isBlocked() || !isSnapshotRendered()
            || multiplayer.submittingAction || multiplayerSubmitInFlightRef.current) {
          setResolveAllTick(tick => tick + 1);
          return;
        }
        run.inFlight = true;
        Promise.resolve(dispatch({ type: "cancel_priority_yield", player: Number(state.perspective) }, "Passing paused"))
          .finally(() => { run.inFlight = false; setResolveAllTick(tick => tick + 1); });
      }, 25);
      return () => clearTimeout(timer);
    }
    if (run.dispatchedFrom === state) return;
    if (state.decision?.kind !== "priority" || (!run.phases && (!state.stack_size
        || (state.stack_objects || []).some(entry => !run.stackIds.has(String(entry.id)))))) {
      stopResolveAll();
      return;
    }
    if (!samePlayerId(state.decision?.player, state.perspective)) return;
    const pass = findPassPriorityAction(state.decision);
    if (!pass || (pass.label && pass.label !== "Pass priority") || holdRule === "always") {
      stopResolveAll();
      return;
    }
    const timer = setTimeout(() => {
      if (stateRef.current !== state || wasmInteractionGateRef.current.isBlocked()
          || !isSnapshotRendered() || multiplayer.submittingAction || multiplayerSubmitInFlightRef.current) {
        setResolveAllTick(tick => tick + 1);
        return;
      }
      const continuing = run.submitted;
      run.inFlight = true;
      run.submitted = true;
      run.dispatchedFrom = state;
      run.stopKeys = JSON.stringify(priorityStops.getSnapshot().stops);
      Promise.resolve(dispatch(continuing ? priorityCommandForAction(pass) : {
        type: "priority_yield",
        mode: run.phases ? "phases" : "stack",
        stops: run.ignoreStops ? [] : Object.keys(priorityStopsForPlayer(priorityStops.getSnapshot().stops, state.active_player)),
      }, run.phases ? "Advance to next pause" : "Resolve all"))
        .catch(stopResolveAll)
        .finally(() => {
          run.inFlight = false;
          setResolveAllTick(tick => tick + 1);
        });
    }, 25);
    return () => clearTimeout(timer);
  }, [dispatch, isSnapshotRendered, multiplayer.submittingAction, resolveAllTick,
    state, stateRef, stopResolveAll, priorityStops, priorityStopsState, holdRule, autoPassEnabled]);

  const automaticRevealBlocked = useCallback(() =>
    wasmInteractionGateRef.current.isBlocked() || !isSnapshotRendered()
      || multiplayer.submittingAction || multiplayerSubmitInFlightRef.current,
  [isSnapshotRendered, multiplayer.submittingAction]);
  useAutomaticHandReveal({ state, stateRef, dispatch, isBlocked: automaticRevealBlocked });

  // Ranking is read-only and sliced. Only a finished, still-current suggestion
  // becomes an ordinary synchronized command; manual input wins every race.
  const cancelBackgroundDispatch = useCallback(() => {
    backgroundDispatchGenerationRef.current += 1;
    const current = stateRef.current;
    manuallyControlledPaymentRef.current = current?.mana_payment?.request_hash;
    if (current?.mana_payment) {
      const next = { ...current, mana_payment: { ...current.mana_payment, planning_complete: true } };
      stateRef.current = next;
      setState(next);
    }
    void game?.cancelPaymentAnalysis().catch(() => {});
  }, [game, setState, stateRef]);

  const dispatchInBackground = useCallback(async () => {
    if (!game) return;
    const generation = backgroundDispatchGenerationRef.current;
    const initial = stateRef.current?.mana_payment;
    if (!initial || manuallyControlledPaymentRef.current === initial.request_hash) return;
    const isCurrent = () => backgroundDispatchGenerationRef.current === generation
      && stateRef.current?.decision?.kind === "mana_payment"
      && stateRef.current?.mana_payment?.request_hash === initial?.request_hash
      && stateRef.current?.mana_payment?.plan_id === initial?.plan_id;
    try {
      const command = await improvePayment({ game, token: String(generation), isCurrent });
      if (!isCurrent()) return;
      if (command) {
        await dispatch(command, undefined, { waitForPaymentReady: true, backgroundGeneration: generation });
      }
      // Also finish the indicator if a suggestion was superseded or the
      // interaction gate declined it without changing this payment.
      if (isCurrent()) {
        const next = { ...stateRef.current, mana_payment: { ...stateRef.current.mana_payment, planning_complete: true } };
        stateRef.current = next;
        setState(next);
      }
    } catch (error) {
      console.warn("Background payment analysis failed:", error);
      if (isCurrent()) {
        const next = { ...stateRef.current, mana_payment: { ...stateRef.current.mana_payment,
          planning_complete: true, planning_error: String(error?.message || error) } };
        stateRef.current = next;
        setState(next);
      }
    }
  }, [dispatch, game, setState, stateRef]);

  useEffect(() => {
    if (surrenderRequested && (!multiplayer.matchStarted || state?.game_over)) {
      const timer = setTimeout(() => setSurrenderRequested(false), 0);
      return () => clearTimeout(timer);
    }
  }, [surrenderRequested, multiplayer.matchStarted, state?.game_over]);

  const cancelDecision = useCallback(
    async ({ waitForPaymentReady = false } = {}) => {
      if (!game) return;
      const paymentTransaction = stateRef.current?.mana_payment?.transaction_id;
      const isCurrentPayment = () => stateRef.current?.decision?.kind === "mana_payment"
        && stateRef.current?.mana_payment?.transaction_id === paymentTransaction
        && samePlayerId(stateRef.current.decision.player, stateRef.current.perspective);
      const runCancel = waitForPaymentReady
        ? async task => {
          // A payment cancellation is an explicit click, including while a
          // replan is running. Retain it through the previous action's cooldown.
          while (isCurrentPayment() && shouldSuppressImmediateCancel()) {
            await new Promise(resolve => setTimeout(resolve, 25));
          }
          return wasmInteractionGateRef.current.runWhenReady(task, isCurrentPayment);
        }
        : runWasmInteraction;
      return runCancel(async () => {
        priorityStops.cancelAction();
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
      stateRef,
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

  const replayAuditTranscript = useCallback(
    async (args = {}) => runWasmInteraction(async () => {
      const currentGame = gameRef.current;
      if (!currentGame) {
        throw new Error("WASM game is not ready");
      }
      return replayAuditTranscriptWithGame({
        game: currentGame,
        transcript: args.transcript,
        perspectiveIndex: stateRef.current?.perspective ?? 0,
        cryptoImpl: globalThis.crypto,
      });
    }),
    [stateRef, runWasmInteraction]
  );

  const runAuditReplayWasmInteraction = useCallback(
    async (task, failurePrefix = "Replay failed") => {
      await waitForAuditReplayGate(wasmInteractionGateRef.current);
      const result = await runWasmInteraction(task);
      if (result !== undefined) return result;
      const message = "Game engine is busy";
      setAuditReplayState((current) => ({
        ...current,
        busy: false,
        error: message,
      }));
      setStatus(`${failurePrefix}: ${message}`, true);
      throw new Error(message);
    },
    [runWasmInteraction, setStatus]
  );

  const replayTranscriptToPosition = useCallback(async (currentGame, session, position) => {
    const transcript = session?.transcript;
    const actions = Array.isArray(transcript?.actions) ? transcript.actions : [];
    const targetPosition = Math.max(0, Math.min(Number(position) || 0, actions.length));
    const startReport = await startAuditTranscriptReplayWithGame({
      game: currentGame,
      transcript,
      perspectiveIndex: session.restorePerspective,
      cryptoImpl: globalThis.crypto,
    });
    let replayState = startReport.state;
    for (let index = 0; index < targetPosition; index += 1) {
      const action = actions[index];
      const actionReport = await applyAuditReplayActionWithGame({
        game: currentGame,
        action,
        actionIndex: index,
        cryptoImpl: globalThis.crypto,
      });
      const expectedHash = String(action?.audit?.publicCheckpointHash || "");
      if (
        expectedHash
        && String(actionReport.publicCheckpointHash || "") !== expectedHash
      ) {
        throw new Error(`Replay public checkpoint hash mismatch at action ${index + 1}`);
      }
      replayState = actionReport.state || replayState;
    }
    if (!replayState && typeof currentGame.uiState === "function") {
      replayState = await currentGame.uiState();
    }
    return {
      position: targetPosition,
      state: replayState,
    };
  }, []);

  const prepareAuditReplaySession = useCallback(
    ({ transcript, sourceLabel = "Verified match" } = {}) => {
      if (!transcript || typeof transcript !== "object") {
        throw new Error("Missing audit transcript");
      }
      assertCurrentAuditReplayProtocol(transcript);
      const prepared = {
        transcript: cloneJson(transcript),
        sourceLabel,
        actionCount: Array.isArray(transcript.actions) ? transcript.actions.length : 0,
      };
      auditReplayPreparedRef.current = prepared;
      setAuditReplayState((current) => {
        if (current.active) {
          return {
            ...current,
            available: true,
            sourceLabel: prepared.sourceLabel,
            actionCount: prepared.actionCount,
            error: "",
          };
        }
        return auditReplayStateForPrepared(prepared);
      });
      return prepared;
    },
    []
  );

  const beginAuditReplaySession = useCallback(
    async ({ transcript, sourceLabel = "Verified match" } = {}) => {
      const prepared = transcript
        ? prepareAuditReplaySession({ transcript, sourceLabel })
        : auditReplayPreparedRef.current;
      if (!prepared?.transcript || typeof prepared.transcript !== "object") {
        throw new Error("Missing audit transcript");
      }
      assertCurrentAuditReplayProtocol(prepared.transcript);
      setAuditReplayState((current) => ({
        ...current,
        available: true,
        busy: true,
        error: "",
      }));
      return runAuditReplayWasmInteraction(async () => {
        const currentGame = gameRef.current;
        if (!currentGame) {
          throw new Error("WASM game is not ready");
        }
        if (!currentGame.supportsRuntimeSavepoints) {
          throw new Error("Game engine cannot start replay mode");
        }
        const existingSession = auditReplaySessionRef.current;
        // Preserve the complete live runtime while replay uses this engine.
        const restorePoint = existingSession?.restorePoint
          || await captureEngineRestorePoint(currentGame);
        const restorePerspective = Number(
          existingSession?.restorePerspective ?? stateRef.current?.perspective ?? 0
        );
        const session = {
          transcript: cloneJson(prepared.transcript),
          sourceLabel: prepared.sourceLabel || sourceLabel,
          restorePoint,
          restorePerspective,
          actionCount: prepared.actionCount,
        };
        try {
          const replay = await replayTranscriptToPosition(currentGame, session, 0);
          auditReplaySessionRef.current = session;
          if (replay.state) {
            stateRef.current = replay.state;
            setState(replay.state);
          }
          const nextReplayState = {
            available: true,
            active: true,
            sourceLabel: session.sourceLabel,
            currentActionIndex: 0,
            currentActionLabel: "Match start",
            actionCount: session.actionCount,
            busy: false,
            error: "",
          };
          setAuditReplayState(nextReplayState);
          setStatus(`Replay loaded: action 0 of ${session.actionCount}`);
          return nextReplayState;
        } catch (err) {
          auditReplaySessionRef.current = existingSession || null;
          if (!existingSession) {
            try {
              await restoreEngineRestorePoint(currentGame, restorePoint);
              const restored = typeof currentGame.uiState === "function"
                ? await currentGame.uiState()
                : stateRef.current;
              if (restored) {
                stateRef.current = restored;
                setState(restored);
              }
            } catch {
              // Preserve the replay error as the actionable failure.
            }
          }
          const message = err instanceof Error ? err.message : String(err);
          setAuditReplayState((current) => ({
            ...current,
            busy: false,
            error: message,
          }));
          setStatus(`Replay failed: ${message}`, true);
          throw err;
        }
      });
    },
    [setState, stateRef, prepareAuditReplaySession, replayTranscriptToPosition, runAuditReplayWasmInteraction, setStatus]
  );

  const setAuditReplayPosition = useCallback(
    async (position) => {
      const session = auditReplaySessionRef.current;
      if (!session) {
        throw new Error("No audit replay is loaded");
      }
      const targetPosition = Math.max(
        0,
        Math.min(Number(position) || 0, session.actionCount)
      );
      setAuditReplayState((current) => ({
        ...current,
        busy: true,
        error: "",
      }));
      return runAuditReplayWasmInteraction(async () => {
        const currentGame = gameRef.current;
        if (!currentGame) {
          throw new Error("WASM game is not ready");
        }
        try {
          const replay = await replayTranscriptToPosition(currentGame, session, targetPosition);
          if (replay.state) {
            stateRef.current = replay.state;
            setState(replay.state);
          }
          const action = replay.position > 0
            ? session.transcript?.actions?.[replay.position - 1]
            : null;
          const nextReplayState = {
            available: true,
            active: true,
            sourceLabel: session.sourceLabel,
            currentActionIndex: replay.position,
            currentActionLabel: auditReplayActionLabel(action, replay.position),
            actionCount: session.actionCount,
            busy: false,
            error: "",
          };
          setAuditReplayState(nextReplayState);
          setStatus(`Replay action ${replay.position} of ${session.actionCount}`);
          return nextReplayState;
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          setAuditReplayState((current) => ({
            ...current,
            busy: false,
            error: message,
          }));
          setStatus(`Replay failed: ${message}`, true);
          throw err;
        }
      });
    },
    [setState, stateRef, replayTranscriptToPosition, runAuditReplayWasmInteraction, setStatus]
  );

  const exitAuditReplaySession = useCallback(
    async () => {
      const session = auditReplaySessionRef.current;
      if (!session) {
        const prepared = auditReplayPreparedRef.current;
        const nextReplayState = prepared
          ? auditReplayStateForPrepared(prepared)
          : emptyAuditReplayState;
        setAuditReplayState(nextReplayState);
        return nextReplayState;
      }
      setAuditReplayState((current) => ({
        ...current,
        busy: true,
        error: "",
      }));
      return runAuditReplayWasmInteraction(async () => {
        const currentGame = gameRef.current;
        try {
          if (currentGame && session.restorePoint) {
            await restoreEngineRestorePoint(
              currentGame,
              session.restorePoint,
            );
            const restored = typeof currentGame.uiState === "function"
              ? await currentGame.uiState()
              : stateRef.current;
            if (restored) {
              stateRef.current = restored;
              setState(restored);
            }
          }
          auditReplaySessionRef.current = null;
          const prepared = auditReplayPreparedRef.current;
          const nextReplayState = prepared
            ? auditReplayStateForPrepared(prepared)
            : emptyAuditReplayState;
          setAuditReplayState(nextReplayState);
          setStatus("Replay closed");
          return nextReplayState;
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          setAuditReplayState((current) => ({
            ...current,
            busy: false,
            error: message,
          }));
          setStatus(`Replay restore failed: ${message}`, true);
          throw err;
        }
      }, "Replay restore failed");
    },
    [setState, stateRef, runAuditReplayWasmInteraction, setStatus]
  );

  useEffect(() => startMainThreadMonitor(), []);

  useEffect(() => {
    if (typeof window === "undefined" || import.meta.env?.VITE_E2E_TEST !== "true") {
      return undefined;
    }

    const snapshot = () => {
      const stableIdByObjectId = objectStableIdMapFromState(state);
      return cloneJson({
      loading,
      wasmError: wasmError ? String(wasmError?.message || wasmError) : "",
      wasmPhase,
      wasmProgress,
      canStartHostedMatch,
      perfEvents: Array.isArray(window.__ironsmithPerfEvents)
        ? window.__ironsmithPerfEvents.slice(-100)
        : [],
      status: status
        ? {
            msg: String(status.msg || ""),
            isError: Boolean(status.isError),
          }
        : null,
      multiplayer: {
        mode: multiplayer.mode,
        role: multiplayer.role,
        localPeerId: multiplayer.localPeerId,
        hostPeerId: multiplayer.hostPeerId,
        lobbyId: multiplayer.lobbyId,
        localPlayerIndex: multiplayer.localPlayerIndex,
        desiredPlayers: multiplayer.desiredPlayers,
        matchStarted: multiplayer.matchStarted,
        lastAppliedSequence: multiplayer.lastAppliedSequence,
        submittingAction: multiplayer.submittingAction,
        securityMode: multiplayer.securityMode,
        verifyingAction: multiplayer.verifyingAction,
        provisionalSequence: multiplayer.provisionalSequence,
        pendingVerification: multiplayer.pendingVerification,
        connectionWarnings: (multiplayer.connectionWarnings || []).map((warning) => ({
          peerId: warning.peerId,
          name: warning.name,
          local: Boolean(warning.local),
          remainingMs: warning.remainingMs,
          kind: warning.kind,
        })),
        players: (multiplayer.players || []).map((player) => ({
          index: player.index,
          name: player.name,
          peerId: player.peerId,
          currentPeerId: player.currentPeerId,
          routePeerId: typeof routePeerIdForPlayer === "function"
            ? routePeerIdForPlayer(player)
            : (player.currentPeerId || player.peerId || ""),
          connected: player.connected,
          ready: player.ready,
          disconnectedAtMs: player.disconnectedAtMs,
          autoForfeitAtMs: player.autoForfeitAtMs,
          disconnectRemainingMs: player.disconnectRemainingMs,
        })),
      },
      state: state
        ? {
            snapshot_id: state.snapshot_id,
            game_over: state.game_over || null,
            perspective: state.perspective,
            turn_number: state.turn_number,
            active_player: state.active_player,
            phase: state.phase,
            step: state.step,
            priority_revision: state.__priority_revision,
            priority_analysis_complete: state.decision?.analysis_complete,
            decision: summarizeDecision(state.decision || null),
            decisionDetail: state.decision,
            combat: state.combat,
            decisionActions: state.decision?.kind === "priority"
              ? (state.decision.actions || []).map((action, actionIndex) => ({
                  index: Number.isFinite(Number(action.index))
                    ? Number(action.index)
                    : actionIndex,
                  label: action.label ? String(action.label) : "",
                  object_id: action.object_id == null ? null : Number(action.object_id),
                  action_ref: action.action_ref || null,
                }))
              : undefined,
            decisionCandidates: state.decision?.kind === "select_objects"
              ? (state.decision.candidates || []).map((candidate) => ({
                  id: candidate.id,
                  stable_id: candidate.stable_id ?? candidate.stableId ?? stableIdByObjectId.get(Number(candidate.id)) ?? null,
                  selection_identity: candidate.selection_identity ?? candidate.selectionIdentity ?? null,
                  reveal_policy: candidate.reveal_policy ?? candidate.revealPolicy ?? null,
                  hidden_ref: candidate.hidden_ref ?? candidate.hiddenRef ?? null,
                  name: candidate.name,
                  legal: candidate.legal,
                }))
              : undefined,
            decisionOptions: state.decision?.kind === "select_options"
              ? (state.decision.options || []).map((option, optionIndex) => ({
                  index: Number.isFinite(Number(option.index))
                    ? Number(option.index)
                    : optionIndex,
                  description: option.description,
                  legal: option.legal,
                }))
              : undefined,
            stack_preview: (state.stack_preview || []).map((entry) => ({
              id: entry.id,
              stable_id: entry.stable_id ?? entry.stableId ?? null,
              name: entry.name,
            })),
            players: (state.players || []).map((player) => ({
              id: player.id,
              name: player.name,
              life: player.life,
              mana_pool: player.mana_pool,
              hand_cards: (player.hand_cards || []).map(card => ({ id: card.id, name: card.name })),
              hand_size: Number.isFinite(Number(player.hand_size))
                ? Number(player.hand_size)
                : (player.hand_cards || []).length,
              library_size: Number.isFinite(Number(player.library_size))
                ? Number(player.library_size)
                : null,
              graveyard_size: Number.isFinite(Number(player.graveyard_size))
                ? Number(player.graveyard_size)
                : (player.graveyard_cards || []).length,
              exile_cards: (player.exile_cards || []).map((card) => ({
                id: card.id,
                stable_id: card.stable_id ?? card.stableId ?? null,
                name: card.name,
                count: card.count,
              })),
              battlefield: (player.battlefield || []).map((card) => ({
                id: card.id,
                stable_id: card.stable_id ?? card.stableId ?? null,
                name: card.name,
                tapped: card.tapped,
                count: card.count,
              })),
            })),
          }
        : null,
      });
    };

    const e2eApi = {
      snapshot,
      priorityAnalysis: () => game?.latestPriorityAnalysis?.() || null,
      runtimeState: () => gameRef.current?.uiState?.() || null,
      publicCheckpoint: () => gameRef.current?.exportPublicAuditCheckpoint?.() || null,
      auditTranscript: () => exportAuditTranscript?.({ includeLiveCheckpoint: false }) || null,
      dispatch: (command, label) => dispatch(command, label),
      submitMultiplayerCommand: (command, label) => submitMultiplayerCommand(command, label),
      cancelDecision: () => cancelDecision(),
    };
    window.__ironsmithE2E = e2eApi;
    return () => {
      if (window.__ironsmithE2E === e2eApi) {
        delete window.__ironsmithE2E;
      }
    };
  }, [
    canStartHostedMatch,
    cancelDecision,
    dispatch,
    loading,
    multiplayer,
    routePeerIdForPlayer,
    state,
    status,
    submitMultiplayerCommand,
    wasmError,
    wasmPhase,
    wasmProgress,
  ]);

  const { recordResolveStack } = loopShortcut;
  const userStartResolveAll = useCallback((options) => {
    recordResolveStack();
    startResolveAll(options);
  }, [recordResolveStack, startResolveAll]);

  // Double-clicking Pay asks to resolve the paid spell or ability once it is
  // on the stack. A cancelled payment returns without a new stack entry.
  const resolveAfterPaymentRef = useRef(null);
  const [resolveAfterPaymentTick, setResolveAfterPaymentTick] = useState(0);
  const requestResolveAfterPayment = useCallback(sourceId => {
    const current = stateRef.current;
    resolveAfterPaymentRef.current = {
      sourceId: sourceId == null ? null : String(sourceId),
      turn: current?.turn_number ?? null,
      stackIds: new Set((current?.stack_objects || []).map(entry => String(entry.id))),
    };
    setResolveAfterPaymentTick(tick => tick + 1);
  }, [stateRef]);
  useEffect(() => {
    const request = resolveAfterPaymentRef.current;
    if (!request || !state) return;
    if (state.game_over || state.turn_number !== request.turn) { resolveAfterPaymentRef.current = null; return; }
    if (state.mana_payment || state.decision?.kind !== "priority"
      || !samePlayerId(state.decision.player, state.perspective)) return;
    resolveAfterPaymentRef.current = null;
    const top = (state.stack_objects || [])[0];
    const paidSource = entry => request.sourceId != null
      && [entry?.id, entry?.inspect_object_id].some(id => id != null && String(id) === request.sourceId);
    const landed = (state.stack_objects || []).some(paidSource)
      || (top && !request.stackIds.has(String(top.id)) && samePlayerId(top.controller, state.perspective));
    // Cast triggers are already on the stack here, so they resolve with it.
    if (landed) userStartResolveAll({ continueThroughOwn: true });
  }, [state, resolveAfterPaymentTick, userStartResolveAll]);

  const value = useMemo(
    () => ({
      matchClockStore,
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
      // User-submitted commands; a loop being recorded keeps them as steps.
      dispatch: userDispatch,
      dispatchInBackground,
      loopShortcut,
      cancelDecision,
      refresh,
      changeGoldfishPlayer,
      autoPassEnabled,
      setAutoPassEnabled,
      autoResolveEnabled,
      setAutoResolveEnabled,
      autoResolveMode,
      setAutoResolveMode,
      holdRule,
      setHoldRule,
      priorityStops: priorityStopsState.stops,
      postActionPriorityWindow: priorityStops.windowFor(state) ? priorityStopsState.window : null,
      cyclePriorityStop,
      holdPostActionPriority, pausePostActionPriority, resumePostActionPriority,
      fixedStartingBoard,
      setFixedStartingBoard,
      uiFont,
      setUiFont,
      playerAccentOverrides,
      setPlayerAccentOverride,
      inspectorDebug,
      setInspectorDebug,
      effectOrderingState: activeEffectOrderingState,
      moveEffectOrderingItem,
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
      redeemTournamentInvite,
      startHostedMatch,
      updateLobbyDeck,
      startRematchSideboarding,
      updateRematchDeck,
      readyForRematch,
      startRematch,
      exportAuditTranscript,
      replayAuditTranscript,
      auditReplay: auditReplayState,
      prepareAuditReplaySession,
      beginAuditReplaySession,
      setAuditReplayPosition,
      exitAuditReplaySession,
      surrenderRequested,
      setSurrenderRequested,
      submitMultiplayerCommand,
      sendLobbyChat,
      submitMultiplayerAddCardCheat,
      cancelBackgroundDispatch,
      setExternalAutoPassGate,
      followActivePlayer,
      setFollowActivePlayer,
      automatedSeats,
      toggleAutomatedSeat,
      perspectiveHandoff,
      startResolveAll: userStartResolveAll,
      requestResolveAfterPayment,
      phasePassing,
      togglePhasePassing,
      startPhaseAdvance,
    }),
    [
      setState,
      matchClockStore,
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
      userDispatch, loopShortcut, dispatchInBackground, cancelBackgroundDispatch, cancelDecision, refresh, changeGoldfishPlayer, autoPassEnabled, autoResolveEnabled, autoResolveMode, setAutoResolveMode, setAutoResolveEnabled, holdRule, uiFont,
      priorityStopsState, cyclePriorityStop, holdPostActionPriority, pausePostActionPriority, resumePostActionPriority,
      playerAccentOverrides, setPlayerAccentOverride, inspectorDebug, fixedStartingBoard,
      activeEffectOrderingState, moveEffectOrderingItem,
      semanticThreshold, setSemanticThreshold, cardsMeetingThreshold,
      logEntries, pushLog,
      multiplayer, canStartHostedMatch, createLobby, joinLobby, leaveLobby, redeemTournamentInvite, startHostedMatch, updateLobbyDeck,
      startRematchSideboarding, updateRematchDeck, readyForRematch, startRematch,
      exportAuditTranscript,
      replayAuditTranscript,
      auditReplayState,
      prepareAuditReplaySession,
      beginAuditReplaySession,
      setAuditReplayPosition,
      exitAuditReplaySession,
      surrenderRequested,
      setSurrenderRequested,
      submitMultiplayerCommand,
      sendLobbyChat,
      submitMultiplayerAddCardCheat,
      setExternalAutoPassGate,
      followActivePlayer,
      automatedSeats,
      toggleAutomatedSeat,
      setFollowActivePlayer,
      perspectiveHandoff,
      userStartResolveAll,
      requestResolveAfterPayment,
      phasePassing,
      togglePhasePassing,
      startPhaseAdvance,
    ]
  );

  return <GameContext.Provider value={value}>
    <ManaPaymentEditorProvider state={state} dispatch={userDispatch} cancelBackgroundDispatch={cancelBackgroundDispatch}>
      {children}
    </ManaPaymentEditorProvider>
  </GameContext.Provider>;
}

// eslint-disable-next-line react-refresh/only-export-components
export function useGame() {
  const ctx = useContext(GameContext);
  if (!ctx) throw new Error("useGame must be used within GameProvider");
  return ctx;
}

// Clock consumers subscribe directly so ticking time never republishes board state.
// eslint-disable-next-line react-refresh/only-export-components
export function useMatchClock() {
  const { matchClockStore } = useGame();
  return useSyncExternalStore(matchClockStore.subscribe, matchClockStore.getSnapshot, matchClockStore.getSnapshot);
}

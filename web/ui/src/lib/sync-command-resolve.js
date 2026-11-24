// Normalizes a UI/engine command into the exact shape every peer signs and
// replays. Shared by the web app (GameContext) and the headless session host.
import { normalizeSelectObjectHiddenRef } from "./sync-commands.js";

export function normalizeMultiplayerTarget(target) {
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

export function normalizeAttackTargetInput(target, declaration = null) {
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

export function normalizeAttackerDeclaration(declaration) {
  if (!declaration || typeof declaration !== "object") return declaration;

  const target = normalizeAttackTargetInput(declaration.target, declaration);
  return {
    creature: Number(declaration.creature ?? declaration.attacker),
    target,
  };
}

export function normalizeBlockerDeclaration(declaration) {
  if (!declaration || typeof declaration !== "object") return declaration;

  return {
    blocker: Number(declaration.blocker),
    blocking: Number(declaration.blocking ?? declaration.attacker),
  };
}


export function resolveSyncedCommand(command) {
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


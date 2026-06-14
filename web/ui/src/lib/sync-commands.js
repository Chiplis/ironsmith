function stableStringify(value) {
  if (value === null || typeof value !== "object") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map(stableStringify).join(",")}]`;
  }
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(",")}}`;
}

export function sameActionRef(left, right) {
  if (!left || !right) return false;
  return stableStringify(left) === stableStringify(right);
}

const DISCONNECT_TIMEOUT_POLICY_REASONS = new Set([
  "disconnect_timeout_policy",
  "peer_claimed_disconnect_timeout",
]);

export function findPriorityActionForCommand(decision, command) {
  if (!decision || decision.kind !== "priority" || command?.type !== "priority_action") {
    return null;
  }

  const actions = Array.isArray(decision.actions) ? decision.actions : [];
  if (command.action_ref) {
    return actions.find((action) => sameActionRef(action?.action_ref, command.action_ref)) || null;
  }

  if (command.action_index !== null && command.action_index !== undefined) {
    const actionIndex = Number(command.action_index);
    return actions.find((action) => Number(action?.index) === actionIndex) || null;
  }

  return null;
}

export function priorityCommandForAction(action) {
  const command = {
    type: "priority_action",
    action_index: Number(action?.index),
  };
  if (action?.action_ref) {
    command.action_ref = action.action_ref;
  }
  return command;
}

export function isDecisionCommandCompatible(decision, command) {
  if (!command) return false;
  if (command.type === "cancel_decision") return true;
  if (command.type === "forfeit_player") {
    if (DISCONNECT_TIMEOUT_POLICY_REASONS.has(String(command.reason || ""))) {
      return command.player !== null && command.player !== undefined;
    }
    return decision?.player !== null
      && decision?.player !== undefined
      && Number(decision.player) === Number(command.player);
  }
  if (!decision) return false;

  switch (decision.kind) {
    case "priority":
      return command.type === "priority_action" && Boolean(findPriorityActionForCommand(decision, command));
    case "targets":
      return command.type === "select_targets";
    case "select_options":
    case "modes":
    case "hybrid_choice":
    case "colors":
      return command.type === "select_options";
    case "select_objects":
      return command.type === "select_objects";
    case "number":
      return command.type === "number_choice";
    case "text_input":
      return command.type === "text_choice";
    case "attackers":
      return command.type === "declare_attackers";
    case "blockers":
      return command.type === "declare_blockers";
    default:
      return false;
  }
}

export function describeDecisionCommandMismatch(decision, command) {
  const commandType = command?.type ? String(command.type) : "unknown";
  const decisionKind = decision?.kind ? String(decision.kind) : "none";
  return `Synced command ${commandType} does not match pending ${decisionKind} decision`;
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

export function normalizeSelectObjectHiddenRef(hiddenRef) {
  if (!hiddenRef || typeof hiddenRef !== "object") return null;
  const normalized = {};
  const owner = Number(hiddenRef.owner);
  if (Number.isSafeInteger(owner) && owner >= 0) normalized.owner = owner;
  const zone = String(hiddenRef.zone || "").trim();
  if (zone) normalized.zone = zone;
  const publicSlot = Number(hiddenRef.public_slot ?? hiddenRef.publicSlot);
  const publicCommitment = String(
    hiddenRef.public_commitment ?? hiddenRef.publicCommitment ?? ""
  ).trim();
  const hasPublicIdentity =
    Number.isSafeInteger(publicSlot) && publicSlot >= 0
    || Boolean(publicCommitment);
  const slot = Number(hiddenRef.slot);
  const commitment = String(hiddenRef.commitment || "").trim();
  const hasPrivateIdentity =
    Number.isSafeInteger(slot) && slot >= 0
    || Boolean(commitment);
  const usePublicLibraryIdentity = zone === "library" && hasPublicIdentity;
  if (usePublicLibraryIdentity) {
    if (Number.isSafeInteger(publicSlot) && publicSlot >= 0) normalized.public_slot = publicSlot;
    if (publicCommitment) normalized.public_commitment = publicCommitment;
  } else {
    if (Number.isSafeInteger(slot) && slot >= 0) {
      normalized.slot = slot;
    } else if (publicCommitment.startsWith("ziffle:") && Number.isSafeInteger(publicSlot) && publicSlot >= 0) {
      normalized.slot = publicSlot;
    }
    if (commitment) {
      normalized.commitment = commitment;
    } else if (publicCommitment.startsWith("ziffle:")) {
      normalized.commitment = publicCommitment;
    }
    if (!hasPrivateIdentity && !publicCommitment.startsWith("ziffle:")) {
      if (Number.isSafeInteger(publicSlot) && publicSlot >= 0) normalized.public_slot = publicSlot;
      if (publicCommitment) normalized.public_commitment = publicCommitment;
    }
  }
  return Object.keys(normalized).length > 0 ? normalized : null;
}

export function selectObjectCandidateForId(decision, objectId) {
  if (!decision || String(decision.kind || "") !== "select_objects") return null;
  const selected = String(objectId);
  return (decision.candidates || []).find((candidate) =>
    String(candidate?.id) === selected
  ) || null;
}

export function selectObjectCandidateIdentity(decision, candidate) {
  return String(
    candidate?.selection_identity
    ?? candidate?.selectionIdentity
    ?? decision?.selection_identity
    ?? decision?.selectionIdentity
    ?? "object_id"
  );
}

export function selectObjectCandidateRevealPolicy(decision, candidate) {
  return String(
    candidate?.reveal_policy
    ?? candidate?.revealPolicy
    ?? decision?.reveal_policy
    ?? decision?.revealPolicy
    ?? "none"
  );
}

export function selectObjectSyncMetadataForCommand(command, stateOrDecision) {
  const decision = stateOrDecision?.kind === "select_objects"
    ? stateOrDecision
    : stateOrDecision?.decision;
  const objectIds = Array.isArray(command?.object_ids) ? command.object_ids : [];
  if (!decision || String(decision.kind || "") !== "select_objects" || objectIds.length === 0) {
    return { stableIds: [], hiddenRefs: [] };
  }
  const stableIds = [];
  const hiddenRefs = [];
  for (const objectId of objectIds) {
    const candidate = selectObjectCandidateForId(decision, objectId);
    const identity = selectObjectCandidateIdentity(decision, candidate);
    const stableId = Number(candidate?.stable_id ?? candidate?.stableId);
    stableIds.push(
      identity === "stable_id" && Number.isSafeInteger(stableId) && stableId > 0
        ? stableId
        : null
    );
    hiddenRefs.push(
      identity === "hidden_reference"
        ? normalizeSelectObjectHiddenRef(candidate?.hidden_ref ?? candidate?.hiddenRef)
        : null
    );
  }
  return { stableIds, hiddenRefs };
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
    const hiddenRef = normalizeSelectObjectHiddenRef(
      command.object_hidden_ref ?? command.objectHiddenRef
    );
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

  if (command.type === "text_choice") {
    return {
      type: "text_choice",
      value: String(command.value ?? ""),
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
      disconnect_certificate: command.disconnect_certificate ?? command.disconnectCertificate,
      protocol_timeout_certificate: command.protocol_timeout_certificate
        ?? command.protocolTimeoutCertificate,
      timeout_certificate: command.timeout_certificate ?? command.timeoutCertificate,
    };
  }

  return command;
}

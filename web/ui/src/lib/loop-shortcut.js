// CR 732.2 loop shortcuts. While a player records a loop, every command they
// submit for their own decisions becomes a step. Object references in a step
// are stored by stable identity (CR 400.7 gives an object a new ObjectId when
// it changes zones, but its stable id survives) plus the card name, never by
// raw ObjectId. Replaying a step re-resolves those references against the
// decision the engine is showing now; a step that no longer resolves ends the
// loop.

const ZONE_KEYS = [
  "battlefield",
  "hand_cards",
  "graveyard_cards",
  "exile_cards",
  "command_cards",
  "ante_cards",
];

// Commands that steer automation or the shortcut itself are never steps.
const NON_STEP_COMMANDS = new Set([
  "priority_yield",
  "cancel_priority_yield",
  "propose_loop_shortcut",
  "respond_loop_shortcut",
  "step_loop_shortcut",
  "cancel_loop_shortcut",
  "cancel_decision",
  "forfeit_player",
]);

function positiveId(value) {
  const id = Number(value);
  return Number.isSafeInteger(id) && id > 0 ? id : null;
}

function rememberObject(index, card) {
  if (!card || typeof card !== "object") return;
  const id = positiveId(card.id ?? card.object_id);
  if (id != null && !index.has(id)) {
    index.set(id, {
      stable_id: positiveId(card.stable_id ?? card.stableId),
      name: String(card.name || ""),
      token: card.token === true,
    });
  }
  const memberIds = Array.isArray(card.member_ids) ? card.member_ids : [];
  const memberStableIds = Array.isArray(card.member_stable_ids) ? card.member_stable_ids : [];
  memberIds.forEach((memberId, position) => {
    const member = positiveId(memberId);
    if (member != null && !index.has(member)) {
      index.set(member, { stable_id: positiveId(memberStableIds[position]), name: String(card.name || ""), token: card.token === true });
    }
  });
}

/** Map of current ObjectId -> { stable_id, name, token } for every visible object. */
export function loopObjectIndex(state) {
  const index = new Map();
  for (const player of state?.players || []) {
    for (const zoneKey of ZONE_KEYS) {
      for (const card of player?.[zoneKey] || []) rememberObject(index, card);
    }
  }
  for (const entry of state?.stack_preview || []) rememberObject(index, entry);
  for (const entry of state?.stack_objects || []) rememberObject(index, entry);
  const candidates = state?.decision?.candidates;
  for (const candidate of Array.isArray(candidates) ? candidates : []) rememberObject(index, candidate);
  return index;
}

function objectRef(index, objectId, fallbackName = "") {
  const id = positiveId(objectId);
  if (id == null) return null;
  const known = index.get(id);
  return {
    stable_id: known?.stable_id ?? null,
    name: known?.name || String(fallbackName || ""),
    ...(known?.token ? { token: true } : {}),
  };
}

/**
 * Pick the current object a recorded reference means among `candidates`
 * (objects with { id, name }): the same stable object if it is still a
 * candidate. A vanished token can match a fresh token with the same name;
 * a card with a known stable identity must never be replaced by another card.
 * Returns null when that identity is unavailable or the token match is ambiguous.
 */
export function resolveLoopObject(ref, candidates, index) {
  if (!ref) return null;
  const list = (candidates || []).filter((candidate) => positiveId(candidate?.id) != null);
  if (ref.stable_id != null) {
    const same = list.find((candidate) => index.get(Number(candidate.id))?.stable_id === ref.stable_id
      || positiveId(candidate.stable_id) === ref.stable_id);
    if (same) return Number(same.id);
    if (!ref.token || [...index.values()].some((object) => object.stable_id === ref.stable_id)) return null;
  }
  const named = list.filter((candidate) => {
    if (ref.token && index.get(Number(candidate.id))?.token !== true && candidate.token !== true) return false;
    const name = candidate.name || index.get(Number(candidate.id))?.name || "";
    return name && name === ref.name;
  });
  return named.length === 1 ? Number(named[0].id) : null;
}

function actionObjectId(action) {
  return positiveId(action?.object_id);
}

/** Turn a command the recording player submitted into a replayable step. */
export function recordLoopStep(command, state) {
  if (!command || typeof command !== "object" || NON_STEP_COMMANDS.has(command.type)) return null;
  const decision = state?.decision;
  if (!decision) return null;
  const index = loopObjectIndex(state);
  const step = { kind: String(decision.kind || ""), type: String(command.type || "") };
  switch (command.type) {
    case "priority_action": {
      const actions = Array.isArray(decision.actions) ? decision.actions : [];
      const action = actions.find((candidate) => command.action_ref
        && JSON.stringify(candidate?.action_ref) === JSON.stringify(command.action_ref))
        || actions.find((candidate) => command.action_index != null
          && Number(candidate?.index) === Number(command.action_index));
      if (!action) return null;
      step.action = {
        kind: String(action.action_ref?.kind || action.kind || ""),
        ability_index: action.ability_index ?? null,
        label: String(action.label || ""),
        object: objectRef(index, actionObjectId(action)),
      };
      return step;
    }
    case "select_targets":
      step.targets = (command.targets || []).map((target) => (target?.kind === "player"
        ? { kind: "player", player: Number(target.player) }
        : { kind: "object", object: objectRef(index, target?.object) }));
      return step;
    case "select_objects":
      step.objects = (command.object_ids || []).map((id) => objectRef(index, id));
      return step;
    case "select_options":
      step.option_indices = (command.option_indices || []).map(Number);
      step.option_descriptions = step.option_indices.map((optionIndex) => String(
        (decision.options || []).find((option) => Number(option?.index) === optionIndex)?.description || ""));
      return step;
    case "select_counters":
      step.allocations = (command.allocations || []).map((entry) => ({ index: Number(entry.index), count: Number(entry.count) }));
      return step;
    case "number_choice":
      step.value = Number(command.value);
      return step;
    case "text_choice":
      step.value = String(command.value ?? "");
      return step;
    case "declare_attackers":
      step.declarations = (command.declarations || []).map((declaration) => ({
        creature: objectRef(index, declaration?.creature),
        target: declaration?.target?.kind === "player"
          ? { kind: "player", player: Number(declaration.target.player) }
          : { kind: String(declaration?.target?.kind || ""), object: objectRef(index, declaration?.target?.object) },
      }));
      return step;
    case "declare_blockers":
      step.declarations = (command.declarations || []).map((declaration) => ({
        blocker: objectRef(index, declaration?.blocker),
        blocking: objectRef(index, declaration?.blocking),
      }));
      return step;
    case "mana_payment":
      // Payment is re-planned on every repetition: plan ids are per request.
      if (command.response?.action !== "confirm") return null;
      step.confirm = true;
      return step;
    default:
      return null;
  }
}

/**
 * "Resolve all" while recording: pass until the stack is empty. Replayed as the
 * ordinary priority passes until resolution reaches the recorded checkpoint.
 */
export function recordResolveStackStep(state) {
  const decision = state?.decision;
  if (decision?.kind !== "priority" || Number(state?.stack_size || 0) <= 0) return null;
  if (!(decision.actions || []).some((action) => (action?.action_ref?.kind || action?.kind) === "pass_priority")) return null;
  return { kind: "priority", type: "resolve_stack" };
}

/** Human-readable name of a step, for "loop stopped" messages. */
export function describeLoopStep(step) {
  if (!step) return "a recorded step";
  if (step.type === "priority_action") return step.action?.label || step.action?.kind || "an action";
  if (step.type === "select_targets") return "choosing targets";
  if (step.type === "select_objects") return "choosing cards";
  if (step.type === "mana_payment") return "paying mana";
  if (step.type === "resolve_stack") return "resolving the stack";
  return step.type.replace(/_/g, " ");
}

function failure(step, reason) {
  return { error: `${describeLoopStep(step)}: ${reason}` };
}

/**
 * Re-resolve a recorded step against the current state. Returns
 * `{ command }` to submit, or `{ error }` when the step is no longer available.
 */
export function resolveLoopStep(step, state) {
  const decision = state?.decision;
  if (!decision) return failure(step, "no decision is pending");
  if (String(decision.kind) !== step.kind) {
    return failure(step, `expected a ${step.kind.replace(/_/g, " ")} decision`);
  }
  const index = loopObjectIndex(state);
  switch (step.type) {
    case "priority_action": {
      const actions = (decision.actions || []).filter((action) => String(action?.action_ref?.kind || action?.kind) === step.action.kind
        && (step.action.ability_index == null || action.ability_index === step.action.ability_index));
      let action = null;
      if (step.action.object) {
        const candidates = actions.map((candidate) => ({ id: actionObjectId(candidate), name: index.get(actionObjectId(candidate))?.name }));
        const objectId = resolveLoopObject(step.action.object, candidates, index);
        action = objectId == null ? null : actions.find((candidate) => actionObjectId(candidate) === objectId);
      } else {
        action = actions.find((candidate) => candidate.label === step.action.label) || (actions.length === 1 ? actions[0] : null);
      }
      if (!action) return failure(step, "is no longer available");
      const command = { type: "priority_action", action_index: Number(action.index) };
      if (action.action_ref) command.action_ref = action.action_ref;
      return { command };
    }
    case "select_targets": {
      const legal = (decision.requirements || []).flatMap((requirement) => requirement?.legal_targets || []);
      const objectCandidates = legal.filter((target) => target?.kind === "object")
        .map((target) => ({ id: target.object, name: target.name }));
      const targets = [];
      for (const target of step.targets || []) {
        if (target.kind === "player") {
          if (!legal.some((choice) => choice?.kind === "player" && Number(choice.player) === target.player)) {
            return failure(step, "that player can no longer be targeted");
          }
          targets.push({ kind: "player", player: target.player });
          continue;
        }
        const objectId = resolveLoopObject(target.object, objectCandidates, index);
        if (objectId == null) return failure(step, `${target.object?.name || "the target"} can no longer be targeted`);
        targets.push({ kind: "object", object: objectId });
      }
      return { command: { type: "select_targets", targets } };
    }
    case "select_objects": {
      const candidates = (decision.candidates || []).filter((candidate) => candidate?.legal !== false);
      const objectIds = [];
      for (const ref of step.objects || []) {
        const remaining = candidates.filter((candidate) => !objectIds.includes(Number(candidate.id)));
        const objectId = resolveLoopObject(ref, remaining, index);
        if (objectId == null) return failure(step, `${ref?.name || "a card"} can no longer be chosen`);
        objectIds.push(objectId);
      }
      return { command: { type: "select_objects", object_ids: objectIds } };
    }
    case "select_options": {
      const options = decision.options || [];
      for (let position = 0; position < step.option_indices.length; position += 1) {
        const option = options.find((candidate) => Number(candidate?.index) === step.option_indices[position]);
        if (!option || option.legal === false || String(option.description || "") !== step.option_descriptions[position]) {
          return failure(step, "the same choice is no longer offered");
        }
      }
      return { command: { type: "select_options", option_indices: step.option_indices } };
    }
    case "select_counters":
      return { command: { type: "select_counters", allocations: step.allocations } };
    case "number_choice":
      if (Number.isFinite(Number(decision.min)) && step.value < Number(decision.min)) return failure(step, "that number is no longer allowed");
      if (Number.isFinite(Number(decision.max)) && step.value > Number(decision.max)) return failure(step, "that number is no longer allowed");
      return { command: { type: "number_choice", value: step.value } };
    case "text_choice":
      return { command: { type: "text_choice", value: step.value } };
    case "declare_attackers": {
      const options = decision.attacker_options || [];
      const declarations = [];
      for (const declaration of step.declarations || []) {
        const creature = resolveLoopObject(declaration.creature,
          options.map((option) => ({ id: option.creature, name: option.creature_name })), index);
        const option = options.find((candidate) => Number(candidate.creature) === creature);
        if (!option) return failure(step, `${declaration.creature?.name || "a creature"} can no longer attack`);
        let target = null;
        if (declaration.target.kind === "player") {
          target = (option.valid_targets || []).some((candidate) => candidate?.kind === "player"
            && Number(candidate.player) === declaration.target.player) ? { kind: "player", player: declaration.target.player } : null;
        } else {
          const objects = (option.valid_targets || []).filter((candidate) => candidate?.kind === declaration.target.kind)
            .map((candidate) => ({ id: candidate.object, name: candidate.name }));
          const objectId = resolveLoopObject(declaration.target.object, objects, index);
          target = objectId == null ? null : { kind: declaration.target.kind, object: objectId };
        }
        if (!target) return failure(step, "that attack is no longer possible");
        declarations.push({ creature, target });
      }
      return { command: { type: "declare_attackers", declarations, bands: [] } };
    }
    case "declare_blockers": {
      const options = decision.blocker_options || [];
      const declarations = [];
      for (const declaration of step.declarations || []) {
        const blocking = resolveLoopObject(declaration.blocking,
          options.map((option) => ({ id: option.attacker, name: option.attacker_name })), index);
        const option = options.find((candidate) => Number(candidate.attacker) === blocking);
        const blocker = option ? resolveLoopObject(declaration.blocker, option.valid_blockers || [], index) : null;
        if (blocker == null) return failure(step, "that block is no longer possible");
        declarations.push({ blocker, blocking });
      }
      return { command: { type: "declare_blockers", declarations } };
    }
    case "resolve_stack":
      if (Number(state?.stack_size || 0) <= 0) return failure(step, "the stack is empty");
      if (!(decision.actions || []).some((action) => (action?.action_ref?.kind || action?.kind) === "pass_priority")) {
        return failure(step, "priority cannot be passed");
      }
      return { command: { type: "priority_action", action_ref: { kind: "pass_priority" } } };
    case "mana_payment": {
      const payment = state?.mana_payment;
      const planId = payment?.plan_id ?? decision.plan_id;
      const requestHash = payment?.request_hash ?? decision.request_hash;
      if (planId == null || requestHash == null || payment?.can_confirm === false) {
        return failure(step, "the cost can no longer be paid");
      }
      return { command: { type: "mana_payment", response: { action: "confirm", plan_id: String(planId), request_hash: String(requestHash) } } };
    }
    default:
      return failure(step, "cannot be replayed");
  }
}

/** The proposer's position in the replay after `completed` steps. */
export function loopReplayPosition(steps, completed) {
  const length = Array.isArray(steps) ? steps.length : 0;
  if (length === 0) return { iteration: 0, stepIndex: 0 };
  return { iteration: Math.floor(completed / length), stepIndex: completed % length };
}

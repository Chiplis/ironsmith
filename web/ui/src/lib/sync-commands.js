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

function sameActionRef(left, right) {
  if (!left || !right) return false;
  return stableStringify(left) === stableStringify(right);
}

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
    if (String(command.reason || "") === "peer_claimed_disconnect_timeout") {
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

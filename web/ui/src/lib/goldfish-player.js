import { samePlayerId } from "./player-display.js";
import { findPassPriorityAction } from "./priority-automation.js";
import { priorityCommandForAction } from "./sync-commands.js";

// Explicit seat selection overrides automatic view-following for this run.
// Required choices still belong to the player making them.
export async function selectGoldfishPlayer(game, playerId, dispatch = (command) => game.dispatch(command)) {
  let state = await game.uiState();
  if (!state?.players?.some(player => samePlayerId(player.id, playerId))) {
    throw new Error("Invalid player");
  }
  const seekPriority = Number(state.stack_size ?? state.stack_objects?.length ?? 0) > 0;
  await game.setPerspective(playerId);
  for (let passes = 0; passes < 800; passes += 1) {
    const decision = state.decision;
    const reached = samePlayerId(decision?.player, playerId)
      && (seekPriority ? decision?.kind === "priority" : samePlayerId(state.active_player, playerId));
    if (state.game_over || reached) return { reached };
    if (!decision) {
      const before = JSON.stringify([state.turn_number, state.active_player, state.phase, state.step, state.priority_player]);
      await game.advancePhase();
      state = await game.uiState();
      if (!state.decision && before === JSON.stringify([state.turn_number, state.active_player, state.phase, state.step, state.priority_player])) break;
      continue;
    }
    let command;
    if (decision?.kind === "priority") {
      const pass = findPassPriorityAction(decision);
      if (!pass || (pass.label && pass.label !== "Pass priority")) {
        if (decision.player != null) await game.setPerspective(Number(decision.player));
        return { reached: false };
      }
      command = priorityCommandForAction(pass);
    } else if (decision?.kind === "attackers"
      && !(decision.attacker_options || []).some(option => option.must_attack)) {
      command = { type: "declare_attackers", declarations: [] };
    } else {
      // Targeting, payments, combat requirements and resolution choices must
      // be answered before the requested player can receive control.
      if (decision?.player != null) await game.setPerspective(Number(decision.player));
      return { reached: false };
    }
    state = await dispatch(command);
  }
  throw new Error("Could not reach the selected player within the automatic passing limit");
}

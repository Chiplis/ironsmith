import { isDecisionCommandCompatible, resolveSyncedCommand } from '../sync-commands.js';

// WASM checkpoints omit live decision continuations (for example a surveil choice).
// Replaying the accepted commands rebuilds those continuations as well as the board.
export async function replayTrustedMatch(game, config, actions, perspective) {
  await game.startMatch(config);
  await game.setPerspective(perspective);
  if (typeof game.setAutoCleanupDiscard === 'function') await game.setAutoCleanupDiscard(false);
  let state = await game.uiState();
  for (let index = 0; index < actions.length; index++) {
    const entry = actions[index];
    if (Number(entry.seq) !== index + 1) throw new Error('Saved match action transcript is incomplete');
    const command = resolveSyncedCommand(entry.command, state);
    // Incomplete priority snapshots may contain only pass-priority. A stable
    // action reference can still be resolved and checked by the engine against
    // the current game; an index alone cannot identify a missing action safely.
    const engineCanResolvePriorityRef = state?.decision?.kind === 'priority'
      && state.decision.analysis_complete === false
      && command?.type === 'priority_action'
      && command.action_ref != null;
    if (command?.type !== 'forfeit_player' && !engineCanResolvePriorityRef
        && !isDecisionCommandCompatible(state?.decision, command)) {
      throw new Error(`Could not restore action ${entry.seq}: ${command?.type} does not match ${state?.decision?.kind || 'no decision'}`);
    }
    if (command.type === 'cancel_decision') await game.cancelDecision();
    else if (command.type === 'forfeit_player') await game.forfeitPlayer(Number(command.player));
    else {
      try {
        await game.dispatch(command);
      } catch (error) {
        throw new Error(`Could not restore action ${entry.seq}: ${error?.message || error}`, { cause: error });
      }
    }
    state = await game.uiState();
  }
  return state;
}

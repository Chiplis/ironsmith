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
    if (command?.type !== 'forfeit_player' && !isDecisionCommandCompatible(state?.decision, command)) {
      throw new Error(`Could not restore action ${entry.seq}: ${command?.type} does not match ${state?.decision?.kind || 'no decision'}`);
    }
    if (command.type === 'cancel_decision') await game.cancelDecision();
    else if (command.type === 'forfeit_player') await game.forfeitPlayer(Number(command.player));
    else await game.dispatch(command);
    state = await game.uiState();
  }
  return state;
}

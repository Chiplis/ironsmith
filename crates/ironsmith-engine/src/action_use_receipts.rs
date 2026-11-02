//! Named action-use receipts scoped to an exact permanent or one of its abilities.
use crate::{GameState, ObjectId};
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActionUseKey {
    pub source: ObjectId,
    /// None shares one allowance across all actions/abilities on the object.
    pub ability: Option<crate::continuous::AbilityOrigin>,
    /// The acquired typed ability definition distinguishes changing text boxes.
    pub definition: Option<ironsmith_core::LinkedExileDefinition>,
    pub group: String,
}
pub fn used_this_turn(game: &GameState, key: &ActionUseKey) -> u32 {
    game.effect_store
        .named_action_uses
        .get(key)
        .filter(|(turn, _)| *turn == game.turn.turn_number)
        .map_or(0, |(_, used)| *used)
}
pub fn remaining_this_turn(game: &GameState, key: &ActionUseKey, maximum: u32) -> u32 {
    maximum.saturating_sub(used_this_turn(game, key))
}
/// Call only when the action's chosen method is paid/committed. Ordinary native
/// action checkpoints clone this map, so cancellation restores the allowance.
pub fn consume_this_turn(
    game: &mut GameState,
    key: &ActionUseKey,
    maximum: u32,
) -> Result<(), String> {
    let used = used_this_turn(game, key);
    if used >= maximum {
        return Err("This action's turn allowance has already been used".into());
    }
    game.effect_store
        .named_action_uses
        .insert(key.clone(), (game.turn.turn_number, used + 1));
    game.mark_object_state_changed_keeping_characteristics(key.source);
    game.mark_continuous_state_dirty();
    Ok(())
}

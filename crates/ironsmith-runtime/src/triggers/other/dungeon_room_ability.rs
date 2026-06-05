//! Dungeon room ability triggers.

use crate::events::DungeonRoomEnteredEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, player_filter_matches_with_context};

#[derive(Debug, Clone, PartialEq)]
pub struct DungeonRoomAbilityTrigger {
    pub owner: PlayerFilter,
}

impl DungeonRoomAbilityTrigger {
    pub fn new(owner: PlayerFilter) -> Self {
        Self { owner }
    }
}

impl TriggerMatcher for DungeonRoomAbilityTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(room_event) = event.downcast::<DungeonRoomEnteredEvent>() else {
            return false;
        };
        player_filter_matches_with_context(
            &self.owner,
            room_event.owner,
            ctx.controller,
            ctx.game,
            None,
        )
    }

    fn display(&self) -> String {
        "Whenever you enter a dungeon room".to_string()
    }
}

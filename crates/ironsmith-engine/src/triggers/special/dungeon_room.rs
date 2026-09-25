//! Dungeon room ability trigger (CR 309.4c).

use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

/// "When you move your venture marker into this room."
///
/// CR 309.4c: every room ability shares this unprinted trigger condition.
/// Dungeon cards are never on the battlefield, so the event scan never sees
/// them; the venture action queues the room ability of the room the marker
/// moved into directly. The room's name (CR 309.4b) and the rooms its arrows
/// lead to (CR 309.5a) ride on the trigger, so a compiled dungeon's room
/// graph is fully described by its room abilities.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DungeonRoomTrigger {
    pub room: String,
    pub leads_to: Vec<String>,
}

impl DungeonRoomTrigger {
    pub fn new(room: impl Into<String>, leads_to: Vec<String>) -> Self {
        Self {
            room: room.into(),
            leads_to,
        }
    }
}

impl TriggerMatcher for DungeonRoomTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        false
    }

    fn display(&self) -> String {
        "When you move your venture marker into this room".to_string()
    }

    fn dungeon_room(&self) -> Option<&DungeonRoomTrigger> {
        Some(self)
    }
}

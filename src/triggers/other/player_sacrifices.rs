//! "Whenever [player] sacrifices [filter]" trigger.

use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSacrificesTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
}

impl PlayerSacrificesTrigger {
    pub fn new(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self { player, filter }
    }
}

impl TriggerMatcher for PlayerSacrificesTrigger {
    fn matches(&self, _event: &TriggerEvent, _ctx: &TriggerContext) -> bool {
        // We don't have a Sacrifice event currently.
        // This would need to be added for full implementation.
        false
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you sacrifice",
            PlayerFilter::Opponent => "an opponent sacrifices",
            PlayerFilter::Any => "a player sacrifices",
            _ => "someone sacrifices",
        };
        format!("Whenever {} {}", player_text, self.filter.description())
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PlayerSacrificesTrigger::new(PlayerFilter::Any, ObjectFilter::creature());
        assert!(trigger.display().contains("sacrifices"));
    }
}

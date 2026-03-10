//! "Whenever [player] plays [land filter]" trigger.

use crate::events::EventKind;
use crate::events::other::LandPlayedEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerPlaysLandTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
}

impl PlayerPlaysLandTrigger {
    pub fn new(player: PlayerFilter, filter: ObjectFilter) -> Self {
        Self { player, filter }
    }
}

impl TriggerMatcher for PlayerPlaysLandTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::LandPlayed {
            return false;
        }
        let Some(e) = event.downcast::<LandPlayedEvent>() else {
            return false;
        };

        let player_matches = match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Active => e.player == ctx.game.turn.active_player,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        };
        if !player_matches {
            return false;
        }

        ctx.game
            .object(e.land)
            .is_some_and(|obj| self.filter.matches(obj, &ctx.filter_ctx, ctx.game))
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you play",
            PlayerFilter::Opponent => "an opponent plays",
            PlayerFilter::Any => "a player plays",
            PlayerFilter::Active => "the active player plays",
            _ => "someone plays",
        };
        let mut object_text = self.filter.description();
        if !object_text.starts_with("a ") && !object_text.starts_with("an ") {
            let article = match object_text.chars().next() {
                Some(ch) if matches!(ch.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u') => "an",
                _ => "a",
            };
            object_text = format!("{article} {object_text}");
        }
        format!("Whenever {player_text} {object_text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = PlayerPlaysLandTrigger::new(PlayerFilter::Opponent, ObjectFilter::land());
        assert_eq!(trigger.display(), "Whenever an opponent plays a land");
    }
}

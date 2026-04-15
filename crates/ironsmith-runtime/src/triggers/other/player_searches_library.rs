//! "Whenever [player] searches their library" trigger.

use crate::events::EventKind;
use crate::events::other::SearchLibraryEvent;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerSearchesLibraryTrigger {
    pub player: PlayerFilter,
}

impl PlayerSearchesLibraryTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for PlayerSearchesLibraryTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SearchLibrary {
            return false;
        }
        let Some(e) = event.downcast::<SearchLibraryEvent>() else {
            return false;
        };

        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Active => e.player == ctx.game.turn.active_player,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you search your library",
            PlayerFilter::Opponent => "an opponent searches their library",
            PlayerFilter::Any => "a player searches their library",
            PlayerFilter::Active => "the active player searches their library",
            _ => "someone searches their library",
        };
        format!("Whenever {player_text}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::other::SearchLibraryEvent;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::triggers::TriggerEvent;

    #[test]
    fn opponent_search_trigger_matches_opponent_search() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let trigger = PlayerSearchesLibraryTrigger::new(PlayerFilter::Opponent);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);
        let event = TriggerEvent::new_with_provenance(
            SearchLibraryEvent::new(bob, Some(bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn opponent_search_trigger_does_not_match_controller_search() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let trigger = PlayerSearchesLibraryTrigger::new(PlayerFilter::Opponent);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);
        let event = TriggerEvent::new_with_provenance(
            SearchLibraryEvent::new(alice, Some(alice)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn opponent_search_trigger_display_uses_their_library_wording() {
        let trigger = PlayerSearchesLibraryTrigger::new(PlayerFilter::Opponent);

        assert_eq!(
            trigger.display(),
            "Whenever an opponent searches their library"
        );
    }
}

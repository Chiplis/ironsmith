//! "Whenever [player] shuffles their library" trigger.

use crate::events::EventKind;
use crate::events::cause::CauseType;
use crate::events::other::ShuffleLibraryEvent;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerShufflesLibraryTrigger {
    pub player: PlayerFilter,
    pub caused_by_effect: bool,
    pub source_controller_shuffles: bool,
}

impl PlayerShufflesLibraryTrigger {
    pub fn new(
        player: PlayerFilter,
        caused_by_effect: bool,
        source_controller_shuffles: bool,
    ) -> Self {
        Self {
            player,
            caused_by_effect,
            source_controller_shuffles,
        }
    }
}

impl TriggerMatcher for PlayerShufflesLibraryTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::ShuffleLibrary {
            return false;
        }
        let Some(e) = event.downcast::<ShuffleLibraryEvent>() else {
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

        if self.caused_by_effect && e.cause.cause_type != CauseType::Effect {
            return false;
        }

        if self.source_controller_shuffles && e.cause.source_controller != Some(e.player) {
            return false;
        }

        true
    }

    fn display(&self) -> String {
        if self.caused_by_effect {
            if self.source_controller_shuffles {
                return "Whenever a spell or ability causes its controller to shuffle their library"
                    .to_string();
            }

            let player_text = match &self.player {
                PlayerFilter::You => "you",
                PlayerFilter::Opponent => "an opponent",
                PlayerFilter::Active => "the active player",
                PlayerFilter::Specific(_) => "that player",
                _ => "a player",
            };
            return format!(
                "Whenever a spell or ability causes {player_text} to shuffle their library"
            );
        }

        match &self.player {
            PlayerFilter::You => "Whenever you shuffle your library".to_string(),
            PlayerFilter::Opponent => "Whenever an opponent shuffles their library".to_string(),
            PlayerFilter::Any => "Whenever a player shuffles their library".to_string(),
            PlayerFilter::Active => "Whenever the active player shuffles their library".to_string(),
            PlayerFilter::Specific(_) => "Whenever that player shuffles their library".to_string(),
            _ => "Whenever a player shuffles their library".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::cause::EventCause;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use crate::triggers::TriggerEvent;

    #[test]
    fn opponent_shuffle_trigger_matches_opponent_shuffle() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let trigger = PlayerShufflesLibraryTrigger::new(PlayerFilter::Opponent, false, false);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);
        let event = TriggerEvent::new_with_provenance(
            ShuffleLibraryEvent::new(bob, EventCause::from_effect(ObjectId::from_raw(2), bob)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn effect_caused_shuffle_trigger_requires_effect_cause() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let trigger = PlayerShufflesLibraryTrigger::new(PlayerFilter::Any, true, false);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);
        let effect_event = TriggerEvent::new_with_provenance(
            ShuffleLibraryEvent::new(bob, EventCause::from_effect(ObjectId::from_raw(2), alice)),
            crate::provenance::ProvNodeId::default(),
        );
        let rule_event = TriggerEvent::new_with_provenance(
            ShuffleLibraryEvent::new(bob, EventCause::from_game_rule()),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&effect_event, &ctx));
        assert!(!trigger.matches(&rule_event, &ctx));
    }

    #[test]
    fn controller_shuffle_trigger_requires_cause_controller_to_match_shuffler() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let trigger = PlayerShufflesLibraryTrigger::new(PlayerFilter::Any, true, true);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(1), alice, &game);
        let matching_event = TriggerEvent::new_with_provenance(
            ShuffleLibraryEvent::new(bob, EventCause::from_effect(ObjectId::from_raw(2), bob)),
            crate::provenance::ProvNodeId::default(),
        );
        let non_matching_event = TriggerEvent::new_with_provenance(
            ShuffleLibraryEvent::new(bob, EventCause::from_effect(ObjectId::from_raw(2), alice)),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(trigger.matches(&matching_event, &ctx));
        assert!(!trigger.matches(&non_matching_event, &ctx));
    }
}

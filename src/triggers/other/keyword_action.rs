//! "Whenever [player] [keyword action]" trigger.

use crate::events::EventKind;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct KeywordActionTrigger {
    pub action: KeywordActionKind,
    pub player: PlayerFilter,
    pub source_must_match: bool,
}

impl KeywordActionTrigger {
    pub fn new(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: false,
        }
    }

    pub fn from_source(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self {
            action,
            player,
            source_must_match: true,
        }
    }
}

impl TriggerMatcher for KeywordActionTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::KeywordAction {
            return false;
        }
        let Some(e) = event.downcast::<KeywordActionEvent>() else {
            return false;
        };
        if e.action != self.action {
            return false;
        }

        if self.source_must_match {
            // Zone changes create a new ObjectId (rule 400.7), so match on the
            // source's stable identity when possible.
            let ctx_stable_source = ctx
                .game
                .object(ctx.source_id)
                .map(|obj| obj.stable_id.object_id())
                .unwrap_or(ctx.source_id);
            if e.source != ctx.source_id && e.source != ctx_stable_source {
                return false;
            }
        }

        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }
    }

    fn display(&self) -> String {
        if self.source_must_match && self.action == KeywordActionKind::Cycle {
            return match &self.player {
                PlayerFilter::You => "Whenever you cycle this card".to_string(),
                PlayerFilter::Opponent => "Whenever an opponent cycles this card".to_string(),
                PlayerFilter::Any => "Whenever a player cycles this card".to_string(),
                _ => "Whenever a player cycles this card".to_string(),
            };
        }
        if self.action == KeywordActionKind::Vote && self.player == PlayerFilter::Any {
            return "Whenever players finish voting".to_string();
        }
        if self.action == KeywordActionKind::NameSticker {
            return match &self.player {
                PlayerFilter::You => "Whenever you put a name sticker on a creature".to_string(),
                PlayerFilter::Opponent => {
                    "Whenever an opponent puts a name sticker on a creature".to_string()
                }
                _ => "Whenever a player puts a name sticker on a creature".to_string(),
            };
        }

        match &self.player {
            PlayerFilter::You => format!("Whenever you {}", self.action.infinitive()),
            PlayerFilter::Opponent => {
                format!("Whenever an opponent {}", self.action.third_person())
            }
            PlayerFilter::Any => format!("Whenever a player {}", self.action.third_person()),
            _ => format!("Whenever a player {}", self.action.third_person()),
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn keyword_action_trigger_matches_you() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);

        let trigger = KeywordActionTrigger::new(KeywordActionKind::Earthbend, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let you_event = TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Earthbend,
            alice,
            source_id,
            2,
        ));
        assert!(trigger.matches(&you_event, &ctx));

        let opp_event = TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Earthbend,
            bob,
            source_id,
            2,
        ));
        assert!(!trigger.matches(&opp_event, &ctx));
    }

    #[test]
    fn keyword_action_trigger_matches_source_stable_id() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let hand_id = game.create_object_from_card(
            &crate::card::CardBuilder::new(crate::ids::CardId::from_raw(1), "Cycler")
                .card_types(vec![crate::types::CardType::Creature])
                .build(),
            alice,
            crate::zone::Zone::Hand,
        );
        let source_id = game
            .move_object(hand_id, crate::zone::Zone::Graveyard)
            .expect("move to graveyard should create new id");

        // Simulate an event emitted using the old/stable ID.
        let stable = game
            .object(source_id)
            .map(|obj| obj.stable_id.object_id())
            .unwrap_or(source_id);
        assert_ne!(stable, source_id, "expected stable id to differ after zone change");
        let event = TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Cycle,
            alice,
            stable,
            1,
        ));

        let trigger = KeywordActionTrigger::from_source(KeywordActionKind::Cycle, PlayerFilter::You);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        assert!(trigger.matches(&event, &ctx));
    }

    #[test]
    fn keyword_action_trigger_mismatched_action() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let trigger = KeywordActionTrigger::new(KeywordActionKind::Investigate, PlayerFilter::Any);
        let ctx = TriggerContext::for_source(source_id, alice, &game);
        let event = TriggerEvent::new(KeywordActionEvent::new(
            KeywordActionKind::Scry,
            alice,
            source_id,
            1,
        ));
        assert!(!trigger.matches(&event, &ctx));
    }

    #[test]
    fn keyword_action_vote_display_uses_finished_voting_phrase() {
        let trigger = KeywordActionTrigger::new(KeywordActionKind::Vote, PlayerFilter::Any);
        assert_eq!(trigger.display(), "Whenever players finish voting");
    }

    #[test]
    fn keyword_action_name_sticker_display_phrase() {
        let trigger = KeywordActionTrigger::new(KeywordActionKind::NameSticker, PlayerFilter::You);
        assert_eq!(
            trigger.display(),
            "Whenever you put a name sticker on a creature"
        );
    }
}

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
}

impl KeywordActionTrigger {
    pub fn new(action: KeywordActionKind, player: PlayerFilter) -> Self {
        Self { action, player }
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

        match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        }
    }

    fn display(&self) -> String {
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

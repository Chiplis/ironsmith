//! "Whenever [player] win a clash" trigger.

use crate::events::EventKind;
use crate::events::other::{KeywordActionEvent, KeywordActionKind};
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct WinsClashTrigger {
    pub player: PlayerFilter,
}

impl WinsClashTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for WinsClashTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::KeywordAction {
            return false;
        }
        let Some(e) = event.downcast::<KeywordActionEvent>() else {
            return false;
        };
        if e.action != KeywordActionKind::Clash {
            return false;
        }
        let Some(winners) = e.player_tags.get(&TagKey::from("winner")) else {
            return false;
        };
        if !winners.contains(&e.player) {
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
        match &self.player {
            PlayerFilter::You => "Whenever you win a clash".to_string(),
            PlayerFilter::Opponent => "Whenever an opponent wins a clash".to_string(),
            PlayerFilter::Any => "Whenever a player wins a clash".to_string(),
            _ => "Whenever a player wins a clash".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::GameState;
    use crate::ids::{ObjectId, PlayerId};
    use std::collections::HashMap;

    fn clash_event(player: PlayerId, winner: Option<PlayerId>) -> TriggerEvent {
        let mut tags = HashMap::new();
        if let Some(winner) = winner {
            tags.insert(TagKey::from("winner"), vec![winner]);
        }
        TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Clash, player, ObjectId::from_raw(99), 1)
                .with_player_tags(tags),
            crate::provenance::ProvNodeId::default(),
        )
    }

    #[test]
    fn matches_when_you_win_a_clash() {
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1);

        let trigger = WinsClashTrigger::new(PlayerFilter::You);
        let ctx = TriggerContext::for_source(source, alice, &game);

        assert!(trigger.matches(&clash_event(alice, Some(alice)), &ctx));
        assert!(!trigger.matches(&clash_event(alice, None), &ctx));
    }
}

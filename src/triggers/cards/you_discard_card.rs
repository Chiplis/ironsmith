//! "Whenever you discard a card" trigger.

use crate::events::EventKind;
use crate::events::other::CardDiscardedEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct YouDiscardCardTrigger {
    pub player: PlayerFilter,
    pub filter: Option<ObjectFilter>,
}

impl YouDiscardCardTrigger {
    pub fn new(player: PlayerFilter, filter: Option<ObjectFilter>) -> Self {
        Self { player, filter }
    }
}

impl TriggerMatcher for YouDiscardCardTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CardDiscarded {
            return false;
        }
        let Some(e) = event.downcast::<CardDiscardedEvent>() else {
            return false;
        };
        let player_matches = match &self.player {
            PlayerFilter::You => e.player == ctx.controller,
            PlayerFilter::Opponent => e.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => e.player == *id,
            _ => true,
        };
        if !player_matches {
            return false;
        }
        if let Some(filter) = &self.filter {
            let Some(card) = ctx.game.object(e.card) else {
                return false;
            };
            return filter.matches(card, &ctx.filter_ctx, ctx.game);
        }
        true
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you".to_string(),
            PlayerFilter::Opponent => "an opponent".to_string(),
            PlayerFilter::Any => "a player".to_string(),
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => "that player".to_string(),
            _ => "a player".to_string(),
        };
        let verb = if matches!(self.player, PlayerFilter::You) {
            "discard"
        } else {
            "discards"
        };
        if let Some(filter) = &self.filter {
            let mut filter_text = filter.description();
            if filter.zone.is_none() && filter_text.ends_with("permanent") {
                let prefix = filter_text.trim_end_matches("permanent").trim_end();
                filter_text = if prefix.is_empty() {
                    "card".to_string()
                } else {
                    format!("{prefix} card")
                };
            } else if !filter_text.ends_with("card") && !filter_text.ends_with("cards") {
                filter_text = format!("{filter_text} card");
            }
            format!("Whenever {player_text} {verb} a {filter_text}")
        } else {
            format!("Whenever {player_text} {verb} a card")
        }
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn test_matches() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source_id = ObjectId::from_raw(1);
        let creature_card = CardBuilder::new(CardId::from_raw(1), "Test Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let card_id = game.create_object_from_card(&creature_card, alice, Zone::Graveyard);

        let trigger = YouDiscardCardTrigger::new(PlayerFilter::You, None);
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let event = TriggerEvent::new(CardDiscardedEvent::new(alice, card_id));
        assert!(trigger.matches(&event, &ctx));

        let opponent_event = TriggerEvent::new(CardDiscardedEvent::new(bob, card_id));
        assert!(!trigger.matches(&opponent_event, &ctx));
    }

    #[test]
    fn test_matches_filtered_card() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source_id = ObjectId::from_raw(1);
        let creature_card = CardBuilder::new(CardId::from_raw(2), "Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let land_card = CardBuilder::new(CardId::from_raw(3), "Land")
            .card_types(vec![CardType::Land])
            .build();
        let creature = game.create_object_from_card(&creature_card, alice, Zone::Graveyard);
        let land = game.create_object_from_card(&land_card, alice, Zone::Graveyard);

        let mut creature_filter = ObjectFilter::default();
        creature_filter.card_types.push(CardType::Creature);
        let trigger = YouDiscardCardTrigger::new(PlayerFilter::You, Some(creature_filter));
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        assert!(trigger.matches(&TriggerEvent::new(CardDiscardedEvent::new(alice, creature)), &ctx));
        assert!(!trigger.matches(&TriggerEvent::new(CardDiscardedEvent::new(alice, land)), &ctx));
    }
}

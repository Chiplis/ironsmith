//! "Whenever [player] reveals [matching] card" trigger.

use crate::events::EventKind;
use crate::events::other::CardRevealedEvent;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerRevealsCardTrigger {
    pub player: PlayerFilter,
    pub filter: ObjectFilter,
    pub from_source: bool,
}

impl PlayerRevealsCardTrigger {
    pub fn new(player: PlayerFilter, filter: ObjectFilter, from_source: bool) -> Self {
        Self {
            player,
            filter,
            from_source,
        }
    }
}

impl TriggerMatcher for PlayerRevealsCardTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CardRevealed {
            return false;
        }
        let Some(revealed) = event.downcast::<CardRevealedEvent>() else {
            return false;
        };

        let player_matches = match &self.player {
            PlayerFilter::You => revealed.player == ctx.controller,
            PlayerFilter::Opponent => revealed.player != ctx.controller,
            PlayerFilter::Any => true,
            PlayerFilter::Specific(id) => revealed.player == *id,
            PlayerFilter::IteratedPlayer => revealed.player == ctx.controller,
            _ => true,
        };
        if !player_matches {
            return false;
        }

        if self.from_source && revealed.source != Some(ctx.source_id) {
            return false;
        }

        let mut filter = self.filter.clone();
        filter.zone = None;

        if let Some(snapshot) = revealed.snapshot.as_ref() {
            return filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }

        ctx.game
            .object(revealed.card)
            .is_some_and(|obj| filter.matches(obj, &ctx.filter_ctx, ctx.game))
    }

    fn display(&self) -> String {
        let player_text = match &self.player {
            PlayerFilter::You => "you".to_string(),
            PlayerFilter::Opponent => "an opponent".to_string(),
            PlayerFilter::Any => "a player".to_string(),
            PlayerFilter::Specific(_) | PlayerFilter::IteratedPlayer => "that player".to_string(),
            _ => "a player".to_string(),
        };
        let suffix = if self.from_source { " this way" } else { "" };
        format!(
            "Whenever {player_text} reveal{} {}{}",
            if matches!(self.player, PlayerFilter::You) {
                ""
            } else {
                "s"
            },
            self.filter.description(),
            suffix
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn reveal_trigger_matches_snapshot_filter_and_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(50);
        let card = CardBuilder::new(CardId::from_raw(1), "Forest")
            .card_types(vec![CardType::Land])
            .build();
        let card_id = game.create_object_from_card(&card, alice, Zone::Hand);
        let snapshot = ObjectSnapshot::from_object(game.object(card_id).expect("card"), &game);
        let event = TriggerEvent::new_with_provenance(
            CardRevealedEvent::new(alice, card_id, Zone::Hand, Some(source), Some(snapshot)),
            crate::provenance::ProvNodeId::default(),
        );
        let trigger = PlayerRevealsCardTrigger::new(PlayerFilter::You, ObjectFilter::land(), true);
        let ctx = TriggerContext::for_source(source, alice, &game);

        assert!(trigger.matches(&event, &ctx));
    }
}

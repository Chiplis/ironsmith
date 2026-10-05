//! "Whenever you discard a card" trigger.

use crate::events::EventKind;
use crate::events::other::CardDiscardedEvent;
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::snapshot::ObjectSnapshot;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct YouDiscardCardTrigger {
    pub player: PlayerFilter,
    pub filter: Option<ObjectFilter>,
    pub cause_controller: Option<PlayerFilter>,
    pub effect_like_only: bool,
    pub one_or_more: bool,
}

impl YouDiscardCardTrigger {
    pub fn new(player: PlayerFilter, filter: Option<ObjectFilter>) -> Self {
        Self {
            player,
            filter,
            cause_controller: None,
            effect_like_only: false,
            one_or_more: false,
        }
    }

    pub fn one_or_more(mut self) -> Self {
        self.one_or_more = true;
        self
    }

    pub fn caused_by_controller(mut self, player: PlayerFilter) -> Self {
        self.cause_controller = Some(player);
        self
    }

    pub fn effect_like_only(mut self) -> Self {
        self.effect_like_only = true;
        self
    }

    fn snapshot_matches_filter(
        snapshot: &ObjectSnapshot,
        filter: &ObjectFilter,
        ctx: &TriggerContext,
    ) -> bool {
        if filter.source && snapshot.object_id == ctx.source_id {
            return true;
        }
        filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game)
    }

    fn event_card_matches_filter(&self, e: &CardDiscardedEvent, ctx: &TriggerContext) -> bool {
        let Some(filter) = &self.filter else {
            return true;
        };
        if filter.source && e.card == ctx.source_id {
            return true;
        }
        if let Some(snapshot) = &e.snapshot {
            return Self::snapshot_matches_filter(snapshot, filter, ctx);
        }
        let Some(card) = ctx.game.object(e.card) else {
            return false;
        };
        filter.matches(card, &ctx.filter_ctx, ctx.game)
    }

    fn batch_matching_count(&self, e: &CardDiscardedEvent, ctx: &TriggerContext) -> usize {
        let Some(filter) = &self.filter else {
            return e.batch_cards.len().max(1);
        };
        if !e.batch_snapshots.is_empty() {
            return e
                .batch_snapshots
                .iter()
                .filter(|snapshot| Self::snapshot_matches_filter(snapshot, filter, ctx))
                .count();
        }
        usize::from(self.event_card_matches_filter(e, ctx))
    }

    /// Last-known information for every card of this discard batch that
    /// matched a "one or more" trigger ("exile them"), in batch order.
    pub(crate) fn matching_batch_snapshots(
        &self,
        e: &CardDiscardedEvent,
        ctx: &TriggerContext,
    ) -> Vec<ObjectSnapshot> {
        if !self.one_or_more {
            return Vec::new();
        }
        e.batch_snapshots
            .iter()
            .filter(|snapshot| {
                self.filter
                    .as_ref()
                    .is_none_or(|filter| Self::snapshot_matches_filter(snapshot, filter, ctx))
            })
            .cloned()
            .collect()
    }

    fn is_first_matching_card_in_batch(
        &self,
        e: &CardDiscardedEvent,
        ctx: &TriggerContext,
    ) -> bool {
        let Some(batch_index) = e.batch_index else {
            return true;
        };
        let Some(filter) = &self.filter else {
            return batch_index == 0;
        };
        if e.batch_snapshots.is_empty() {
            return true;
        }
        e.batch_snapshots
            .iter()
            .take(batch_index)
            .all(|snapshot| !Self::snapshot_matches_filter(snapshot, filter, ctx))
    }
}

impl TriggerMatcher for YouDiscardCardTrigger {
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::CardDiscarded])
    }
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::CardDiscarded {
            return false;
        }
        let Some(e) = event.downcast::<CardDiscardedEvent>() else {
            return false;
        };
        let player_matches = crate::filter::player_filter_matches_game(
            &self.player,
            e.player,
            ctx.game,
            &ctx.filter_ctx,
        );
        if !player_matches {
            return false;
        }
        if self.effect_like_only
            && !e
                .cause
                .as_ref()
                .is_some_and(|cause| cause.cause_type.is_effect_like() && cause.source.is_some())
        {
            return false;
        }
        if let Some(controller_filter) = &self.cause_controller {
            let Some(controller) = e.cause.as_ref().and_then(|cause| cause.source_controller)
            else {
                return false;
            };
            let controller_matches = crate::filter::player_filter_matches_game(
                controller_filter,
                controller,
                ctx.game,
                &ctx.filter_ctx,
            );
            if !controller_matches {
                return false;
            }
        }
        if !self.event_card_matches_filter(e, ctx) {
            return false;
        }
        if self.one_or_more {
            return self.batch_matching_count(e, ctx) > 0
                && self.is_first_matching_card_in_batch(e, ctx);
        }
        true
    }

    fn event_value_amount(&self, event: &TriggerEvent, ctx: &TriggerContext) -> Option<i32> {
        if !self.one_or_more || event.kind() != EventKind::CardDiscarded {
            return None;
        }
        let e = event.downcast::<CardDiscardedEvent>()?;
        Some(self.batch_matching_count(e, ctx) as i32)
    }

    fn display(&self) -> String {
        if self.effect_like_only
            && self.player == PlayerFilter::You
            && self.filter.as_ref().is_none_or(|filter| filter.source)
            && let Some(controller) = &self.cause_controller
        {
            let actor = match controller {
                PlayerFilter::You => "you control".to_string(),
                PlayerFilter::Opponent => "an opponent controls".to_string(),
                PlayerFilter::Any => "a player controls".to_string(),
                player => format!("{} controls", player.description()),
            };
            let card = if self.filter.as_ref().is_some_and(|filter| filter.source) {
                "this card"
            } else if self.one_or_more {
                "one or more cards"
            } else {
                "a card"
            };
            return format!("Whenever a spell or ability {actor} causes you to discard {card}");
        }
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
            if self.one_or_more {
                format!("Whenever {player_text} {verb} one or more {filter_text}s")
            } else {
                format!("Whenever {player_text} {verb} a {filter_text}")
            }
        } else if self.one_or_more {
            format!("Whenever {player_text} {verb} one or more cards")
        } else {
            format!("Whenever {player_text} {verb} a card")
        }
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

        let event = TriggerEvent::new_with_provenance(
            CardDiscardedEvent::new(alice, card_id),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&event, &ctx));

        let opponent_event = TriggerEvent::new_with_provenance(
            CardDiscardedEvent::new(bob, card_id),
            crate::provenance::ProvNodeId::default(),
        );
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

        assert!(trigger.matches(
            &TriggerEvent::new_with_provenance(
                CardDiscardedEvent::new(alice, creature),
                crate::provenance::ProvNodeId::default()
            ),
            &ctx
        ));
        assert!(!trigger.matches(
            &TriggerEvent::new_with_provenance(
                CardDiscardedEvent::new(alice, land),
                crate::provenance::ProvNodeId::default()
            ),
            &ctx
        ));
    }

    #[test]
    fn test_matches_opponent_controlled_effect_discard() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::from_raw(2), "Sand Golem")
            .card_types(vec![CardType::Artifact, CardType::Creature])
            .build();
        let card_id = game.create_object_from_card(&card, alice, crate::zone::Zone::Graveyard);
        let source_id = card_id;

        let trigger = YouDiscardCardTrigger::new(PlayerFilter::You, Some(ObjectFilter::source()))
            .caused_by_controller(PlayerFilter::Opponent)
            .effect_like_only();
        let ctx = TriggerContext::for_source(source_id, alice, &game);

        let matching = TriggerEvent::new_with_provenance(
            CardDiscardedEvent::with_cause(
                alice,
                card_id,
                crate::events::cause::EventCause::from_effect(source_id, bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(trigger.matches(&matching, &ctx));

        let own_controller = TriggerEvent::new_with_provenance(
            CardDiscardedEvent::with_cause(
                alice,
                card_id,
                crate::events::cause::EventCause::from_effect(source_id, alice),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&own_controller, &ctx));

        let cost_discard = TriggerEvent::new_with_provenance(
            CardDiscardedEvent::with_cause(
                alice,
                card_id,
                crate::events::cause::EventCause::from_cost(source_id, bob),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        assert!(!trigger.matches(&cost_discard, &ctx));
    }
}

#[cfg(test)]
mod causal_group_tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};
    #[test]
    fn grouped_causal_discard_keeps_effect_kind_actor_team_and_batch_index() {
        let mut game = crate::game_state::GameState::new(
            vec!["A".into(), "B".into(), "C".into(), "D".into()],
            20,
        );
        let [a, b, c, d] = [0, 1, 2, 3].map(PlayerId::from_index);
        game.set_teams(vec![vec![a, b], vec![c, d]]).unwrap();
        let trigger = YouDiscardCardTrigger::new(PlayerFilter::You, None)
            .caused_by_controller(PlayerFilter::Opponent)
            .effect_like_only()
            .one_or_more();
        let ctx = TriggerContext::for_source(ObjectId::from_raw(10), a, &game);
        for (actor, cost, index, expected) in [
            (c, false, 0, true),
            (b, false, 0, false),
            (c, true, 0, false),
            (c, false, 1, false),
        ] {
            let source = ObjectId::from_raw(20);
            let cards = vec![ObjectId::from_raw(30), ObjectId::from_raw(31)];
            let cause = if cost {
                crate::events::cause::EventCause::from_cost(source, actor)
            } else {
                crate::events::cause::EventCause::from_effect(source, actor)
            };
            let event = TriggerEvent::new_with_provenance(
                CardDiscardedEvent::with_cause(a, cards[index], cause).with_batch(
                    cards,
                    vec![],
                    index,
                ),
                Default::default(),
            );
            assert_eq!(trigger.matches(&event, &ctx), expected);
            if expected {
                assert_eq!(trigger.event_value_amount(&event, &ctx), Some(2));
            }
        }
    }
}

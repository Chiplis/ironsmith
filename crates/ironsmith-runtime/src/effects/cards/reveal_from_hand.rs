//! Reveal cards from hand.

use crate::decision::FallbackStrategy;
use crate::decisions::context::ViewCardsContext;
use crate::decisions::{ChooseObjectsSpec, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::helpers::normalize_object_selection;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::types::CardType;

/// Effect that reveals cards from the controller's hand.
///
/// Revealing is primarily a visibility action in this engine model.
#[derive(Debug, Clone, PartialEq)]
pub struct RevealFromHandEffect {
    pub count: u32,
    pub card_type: Option<CardType>,
}

impl RevealFromHandEffect {
    pub fn new(count: u32, card_type: Option<CardType>) -> Self {
        Self { count, card_type }
    }

    fn valid_cards(
        &self,
        game: &GameState,
        player: crate::ids::PlayerId,
        source: crate::ids::ObjectId,
    ) -> Vec<ObjectId> {
        game.player(player)
            .map(|p| {
                p.hand
                    .iter()
                    .copied()
                    .filter(|card_id| {
                        if *card_id == source {
                            return false;
                        }
                        self.card_type.map_or(true, |card_type| {
                            game.object(*card_id)
                                .is_some_and(|obj| obj.has_card_type(card_type))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn cost_display(&self) -> String {
        let type_str = self
            .card_type
            .map_or("card".to_string(), |ct| ct.card_phrase().to_string());
        if self.count == 1 {
            format!("Reveal a {} from your hand", type_str)
        } else {
            format!("Reveal {} {}s from your hand", self.count, type_str)
        }
    }
}

impl EffectExecutor for RevealFromHandEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let valid_cards = self.valid_cards(game, ctx.controller, ctx.source);
        let required = (self.count as usize).min(valid_cards.len());
        if required == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let explicit_cards: Vec<_> = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::executor::ResolvedTarget::Object(id) => Some(*id),
                crate::executor::ResolvedTarget::Player(_) => None,
            })
            .collect();

        let cards_to_reveal = if !explicit_cards.is_empty() {
            normalize_object_selection(explicit_cards, &valid_cards, required)
        } else {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!(
                    "Choose {} card{} to reveal",
                    required,
                    if required == 1 { "" } else { "s" }
                ),
                valid_cards.clone(),
                required,
                Some(required),
            );
            let chosen: Vec<_> = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
                FallbackStrategy::Maximum,
            );
            normalize_object_selection(chosen, &valid_cards, required)
        };

        for viewer_idx in 0..game.players.len() {
            let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
            let view_ctx = ViewCardsContext::new(
                viewer,
                ctx.controller,
                Some(ctx.source),
                crate::zone::Zone::Hand,
                "Reveal cards from hand",
            )
            .with_public(true);
            ctx.decision_maker
                .view_cards(game, viewer, &cards_to_reveal, &view_ctx);
        }

        let revealed_snapshots: Vec<_> = cards_to_reveal
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect();
        if !revealed_snapshots.is_empty() {
            let entry = ctx
                .tagged_objects
                .entry(TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
                .or_default();
            for snapshot in revealed_snapshots {
                if !entry
                    .iter()
                    .any(|existing| existing.object_id == snapshot.object_id)
                {
                    entry.push(snapshot);
                }
            }
        }

        let reveal_events = cards_to_reveal
            .iter()
            .filter_map(|&id| {
                let snapshot = game
                    .object(id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))?;
                Some(crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CardRevealedEvent::new(
                        ctx.controller,
                        id,
                        crate::zone::Zone::Hand,
                        Some(ctx.source),
                        Some(snapshot),
                    ),
                    ctx.provenance,
                ))
            })
            .collect::<Vec<_>>();

        Ok(EffectOutcome::count(cards_to_reveal.len() as i32).with_events(reveal_events))
    }

    fn cost_description(&self) -> Option<String> {
        Some(self.cost_display())
    }
}

impl CostExecutableEffect for RevealFromHandEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        if self.valid_cards(game, controller, source).len() < self.count as usize {
            return Err(CostValidationError::NotEnoughCards);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::costs::{Cost, CostContext, CostPaymentResult};
    use crate::decision::DecisionMaker;
    use crate::executor::{ExecutionContext, ResolvedTarget};
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }
    }

    fn create_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn simple_card(name: &str, id: u32) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn display_text() {
        assert_eq!(
            RevealFromHandEffect::new(1, None).cost_display(),
            "Reveal a card from your hand"
        );
        assert_eq!(
            RevealFromHandEffect::new(1, Some(CardType::Land)).cost_display(),
            "Reveal a land card from your hand"
        );
    }

    #[test]
    fn pay_with_preselected_cards() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);

        let card1 = simple_card("Card 1", 1);
        let id1 = game.create_object_from_card(&card1, alice, Zone::Hand);

        let cost = Cost::effect(RevealFromHandEffect::new(1, None));
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = CostContext::new(source, alice, &mut dm).with_pre_chosen_cards(vec![id1]);

        assert_eq!(cost.pay(&mut game, &mut ctx), Ok(CostPaymentResult::Paid));
    }

    #[test]
    fn reveal_from_hand_emits_public_view_cards_event() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(999);
        let id1 = game.create_object_from_card(&simple_card("Card 1", 1), alice, Zone::Hand);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(id1)]);

        RevealFromHandEffect::new(1, None)
            .execute(&mut game, &mut ctx)
            .expect("reveal from hand");

        assert_eq!(dm.calls.len(), 2);
        assert!(dm.calls.iter().all(|(_, subject, zone, public, cards)| {
            *subject == alice && *zone == Zone::Hand && *public && cards == &vec![id1]
        }));
    }

    #[test]
    fn reveal_from_hand_records_public_reveal_tag_for_stack_lifetime() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(1000);
        let id1 = game.create_object_from_card(&simple_card("Card 1", 3), alice, Zone::Hand);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(id1)]);

        RevealFromHandEffect::new(1, None)
            .execute(&mut game, &mut ctx)
            .expect("reveal from hand");

        let revealed = ctx
            .tagged_objects
            .get(&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
            .expect("reveal should record the public reveal tag");
        assert_eq!(revealed.len(), 1);
        assert_eq!(revealed[0].object_id, id1);
        assert_eq!(revealed[0].zone, Zone::Hand);
    }
}

//! Reveal cards from hand.

use crate::decision::FallbackStrategy;
use crate::decisions::context::ViewCardsContext;
use crate::decisions::{ChooseObjectsSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{normalize_object_selection, resolve_value};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::zone::Zone;

pub type RevealSourceFromHandEffect = ironsmith_core::RevealSourceFromHandEffect;
pub type RevealFromHandEffect = ironsmith_core::RevealFromHandEffect;

fn valid_reveal_from_hand_cards(
    effect: &RevealFromHandEffect,
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
                    let Some(obj) = game.object(*card_id) else {
                        return false;
                    };
                    if effect
                        .card_type
                        .is_some_and(|card_type| !obj.has_card_type(card_type))
                    {
                        return false;
                    }
                    if let Some(required_colors) = effect.color_filter {
                        return game.current_colors(*card_id).is_some_and(|colors| {
                            !colors.intersection(required_colors).is_empty()
                        });
                    }
                    true
                })
                .collect()
        })
        .unwrap_or_default()
}

fn required_reveal_count(
    effect: &RevealFromHandEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<usize, ExecutionError> {
    Ok(resolve_value(game, &effect.count, ctx)?.max(0) as usize)
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
        let valid_cards = valid_reveal_from_hand_cards(self, game, ctx.controller, ctx.source);
        let required = required_reveal_count(self, game, ctx)?;
        if valid_cards.len() < required {
            return Err(ExecutionError::Impossible(format!(
                "cannot reveal {required} card(s): only {} matching card(s) are available",
                valid_cards.len()
            )));
        }
        if required == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let explicit_cards: Vec<_> = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                crate::effects::ResolvedTarget::Player(_) => None,
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
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
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

    fn references_cost_x(&self) -> bool {
        self.count == Value::X
    }

    fn max_cost_x(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Option<u32> {
        if !self.references_cost_x() {
            return None;
        }
        u32::try_from(valid_reveal_from_hand_cards(self, game, controller, source).len()).ok()
    }
}

impl CostExecutableEffect for RevealFromHandEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        let Value::Fixed(count) = self.count else {
            return Ok(());
        };
        if valid_reveal_from_hand_cards(self, game, controller, source).len()
            < count.max(0) as usize
        {
            return Err(CostValidationError::NotEnoughCards);
        }
        Ok(())
    }
}

impl EffectExecutor for RevealSourceFromHandEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(source) = game.object(ctx.source) else {
            return Ok(EffectOutcome::count(0));
        };
        if source.owner != ctx.controller || source.zone != Zone::Hand {
            return Ok(EffectOutcome::count(0));
        }

        let source_id = ctx.source;
        for viewer_idx in 0..game.players.len() {
            let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
            let view_ctx = ViewCardsContext::new(
                viewer,
                ctx.controller,
                Some(ctx.source),
                Zone::Hand,
                "Reveal source card from hand",
            )
            .with_public(true);
            ctx.decision_maker
                .view_cards(game, viewer, &[source_id], &view_ctx);
        }

        let Some(snapshot) = game
            .object(source_id)
            .map(|obj| ObjectSnapshot::from_object(obj, game))
        else {
            return Ok(EffectOutcome::count(0));
        };
        let entry = ctx
            .tagged_objects
            .entry(TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
            .or_default();
        if !entry
            .iter()
            .any(|existing| existing.object_id == snapshot.object_id)
        {
            entry.push(snapshot.clone());
        }

        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::CardRevealedEvent::new(
                ctx.controller,
                source_id,
                Zone::Hand,
                Some(ctx.source),
                Some(snapshot),
            ),
            ctx.provenance,
        );
        Ok(EffectOutcome::count(1).with_events(vec![event]))
    }

    fn cost_description(&self) -> Option<String> {
        Some("Reveal this card from your hand".to_string())
    }
}

impl CostExecutableEffect for RevealSourceFromHandEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        let Some(object) = game.object(source) else {
            return Err(CostValidationError::Other(
                "source card is not available to reveal".to_string(),
            ));
        };
        if object.owner != controller || object.zone != Zone::Hand {
            return Err(CostValidationError::Other(
                "source card is not in your hand".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::color::ColorSet;
    use crate::costs::{Cost, CostContext, CostPaymentResult};
    use crate::decision::DecisionMaker;
    use crate::effects::{ExecutionContext, ResolvedTarget};
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
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

    fn colored_card(name: &str, id: u32, colors: ColorSet) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(id), name)
            .card_types(vec![CardType::Creature])
            .color_indicator(colors)
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
    fn martyr_of_spores_reveal_x_green_cost_reveals_only_green_cards() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source_card = colored_card("Martyr of Spores", 99, ColorSet::GREEN);
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let green_one = game.create_object_from_card(
            &colored_card("Green Card One", 1, ColorSet::GREEN),
            alice,
            Zone::Hand,
        );
        let blue_card = game.create_object_from_card(
            &colored_card("Blue Card", 2, ColorSet::BLUE),
            alice,
            Zone::Hand,
        );
        let green_two = game.create_object_from_card(
            &colored_card("Green Card Two", 3, ColorSet::GREEN),
            alice,
            Zone::Hand,
        );

        let cost = Cost::effect(RevealFromHandEffect::with_color_filter(
            Value::X,
            None,
            Some(ColorSet::GREEN),
        ));
        let mut dm = CaptureViewDm::default();
        let mut ctx = CostContext::new(source, alice, &mut dm)
            .with_x(2)
            .with_pre_chosen_cards(vec![green_one, blue_card, green_two]);

        assert_eq!(cost.pay(&mut game, &mut ctx), Ok(CostPaymentResult::Paid));
        let revealed = ctx
            .tagged_objects
            .get(&TagKey::from(crate::effects::PUBLIC_REVEALED_TAG))
            .expect("Martyr of Spores reveal cost should tag revealed cards");
        let revealed_ids: Vec<_> = revealed.iter().map(|snapshot| snapshot.object_id).collect();
        assert_eq!(revealed_ids, vec![green_one, green_two]);
        assert!(
            !revealed_ids.contains(&blue_card),
            "Martyr of Spores should not allow non-green cards to pay the reveal-X-green cost"
        );
    }

    #[test]
    fn martyr_of_spores_reveal_x_green_cost_requires_enough_green_cards() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source_card = colored_card("Martyr of Spores", 100, ColorSet::GREEN);
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let green_card = game.create_object_from_card(
            &colored_card("Only Green Card", 4, ColorSet::GREEN),
            alice,
            Zone::Hand,
        );
        game.create_object_from_card(
            &colored_card("Non-green Card", 5, ColorSet::BLUE),
            alice,
            Zone::Hand,
        );

        let cost = Cost::effect(RevealFromHandEffect::with_color_filter(
            Value::X,
            None,
            Some(ColorSet::GREEN),
        ));
        let mut dm = CaptureViewDm::default();
        let mut ctx = CostContext::new(source, alice, &mut dm)
            .with_x(2)
            .with_pre_chosen_cards(vec![green_card]);

        let err = cost
            .pay(&mut game, &mut ctx)
            .expect_err("Martyr of Spores should require X green cards to pay X=2");
        assert!(
            err.to_string().contains("cannot reveal 2 card"),
            "expected insufficient matching green cards error, got {err:?}"
        );
    }

    #[test]
    fn reveal_source_from_hand_cost_reveals_the_source_card() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(99), "Forecast Card").build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Hand);

        let cost = Cost::effect(RevealSourceFromHandEffect::new());
        let mut dm = CaptureViewDm::default();
        let mut ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(cost.pay(&mut game, &mut ctx), Ok(CostPaymentResult::Paid));
        assert!(dm.calls.iter().all(|(_, _, zone, public, cards)| {
            *zone == Zone::Hand && *public && cards.as_slice() == [source]
        }));
        assert_eq!(dm.calls.len(), 2);
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

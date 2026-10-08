//! Reveal cards from hand.

use crate::decision::FallbackStrategy;
use crate::decisions::context::{SelectionRevealPolicy, ViewCardsContext};
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

/// The hand cards `effect` may reveal, as a filter (used for hidden-hand
/// claims in peer matches).
fn reveal_filter(
    effect: &RevealFromHandEffect,
    player: crate::ids::PlayerId,
) -> crate::filter::ObjectFilter {
    let mut filter = crate::filter::ObjectFilter::default()
        .in_zone(Zone::Hand)
        .nontoken()
        .owned_by(crate::target::PlayerFilter::Specific(player));
    if let Some(card_type) = effect.card_type {
        filter = filter.with_type(card_type);
    }
    if let Some(colors) = effect.color_filter {
        filter = filter.with_colors(colors);
    }
    filter
}

/// The hand cards `player` may reveal, and whether that depends on hidden
/// hand identities (symmetric across peers; see
/// `game_state::hidden_hand_choices`). When it does, this peer's hidden-card
/// placeholders stay revealable: only the owner knows whether they match.
fn reveal_from_hand_candidates(
    effect: &RevealFromHandEffect,
    game: &GameState,
    player: crate::ids::PlayerId,
    source: crate::ids::ObjectId,
) -> (Vec<ObjectId>, bool) {
    let hand: Vec<ObjectId> = game
        .player(player)
        .map(|p| p.hand.iter().copied().filter(|id| *id != source).collect())
        .unwrap_or_default();
    let filter = reveal_filter(effect, player);
    let hidden_hand_choice =
        game.hand_choice_depends_on_hidden_identity(&filter, hand.iter().copied());
    let placeholders = if hidden_hand_choice {
        let filter_ctx = crate::filter::FilterContext::new(player).with_source(source);
        game.hidden_hand_placeholder_candidates(&filter, &filter_ctx, hand.iter().copied())
    } else {
        Vec::new()
    };
    let candidates = hand
        .into_iter()
        .filter(|card_id| {
            let Some(obj) = game.object(*card_id) else {
                return false;
            };
            if obj.zone != Zone::Hand
                || obj.owner != player
                || obj.kind != crate::object::ObjectKind::Card
            {
                return false;
            }
            if placeholders.contains(card_id) {
                return true;
            }
            if effect
                .card_type
                .is_some_and(|card_type| !obj.has_card_type(card_type))
            {
                return false;
            }
            if let Some(required_colors) = effect.color_filter {
                return game
                    .current_colors(*card_id)
                    .is_some_and(|colors| !colors.intersection(required_colors).is_empty());
            }
            true
        })
        .collect();
    (candidates, hidden_hand_choice)
}

fn valid_reveal_from_hand_cards(
    effect: &RevealFromHandEffect,
    game: &GameState,
    player: crate::ids::PlayerId,
    source: crate::ids::ObjectId,
) -> Vec<ObjectId> {
    reveal_from_hand_candidates(effect, game, player, source).0
}

/// Shared by cost admission, native special actions, and resolving reveal
/// payments. Hidden candidates retain the existing public-opening contract.
pub(crate) fn legal_reveal_from_hand_cards(
    game: &GameState,
    player: crate::ids::PlayerId,
    source: ObjectId,
    card_type: Option<crate::types::CardType>,
    color_filter: Option<crate::color::ColorSet>,
) -> Vec<ObjectId> {
    valid_reveal_from_hand_cards(
        &RevealFromHandEffect::with_color_filter(1, card_type, color_filter),
        game,
        player,
        source,
    )
}

pub(crate) fn is_exact_reveal_selection(
    selected: &[ObjectId],
    candidates: &[ObjectId],
    required: usize,
) -> bool {
    selected.len() == required
        && selected
            .iter()
            .enumerate()
            .all(|(index, id)| candidates.contains(id) && !selected[..index].contains(id))
}

fn required_reveal_count(
    effect: &RevealFromHandEffect,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<usize, ExecutionError> {
    Ok(resolve_value(game, &effect.count, ctx)?.max(0) as usize)
}

impl EffectExecutor for RevealFromHandEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Revealed)
    }
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let (valid_cards, hidden_hand_choice) =
            reveal_from_hand_candidates(self, game, ctx.controller, ctx.source);
        let required = required_reveal_count(self, game, ctx)?;
        // With hidden hand cards involved the local candidate count differs
        // between the owner and peers holding placeholders, so a shortfall
        // decides nothing here: every peer asks and replays the owner's answer.
        if valid_cards.len() < required && !hidden_hand_choice {
            return Err(ExecutionError::Impossible(format!(
                "cannot reveal {required} card(s): only {} matching card(s) are available",
                valid_cards.len()
            )));
        }
        if required == 0 {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        // Revealed hidden cards are opened on every peer before the answer is
        // replayed, so the choice is always asked (never auto-picked).
        let reveal_publicly = valid_cards
            .iter()
            .any(|id| game.hidden_identity_is_private(*id));

        let explicit_cards: Vec<_> = ctx
            .targets
            .iter()
            .filter_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => Some(*id),
                crate::effects::ResolvedTarget::Player(_) => None,
            })
            .collect();

        let cards_to_reveal = if !explicit_cards.is_empty() {
            if ctx.targets_are_cost_choices {
                if !is_exact_reveal_selection(&explicit_cards, &valid_cards, required) {
                    return Err(ExecutionError::Impossible(
                        "reveal payment must select exactly the required legal hand cards".into(),
                    ));
                }
                explicit_cards
            } else {
                normalize_object_selection(explicit_cards, &valid_cards, required)
            }
        } else {
            let mut spec = ChooseObjectsSpec::new(
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
            if reveal_publicly {
                spec = spec
                    .require_explicit_choice()
                    .with_selection_reveal_policy(SelectionRevealPolicy::Public);
            }
            if hidden_hand_choice && ctx.optional_action {
                // The owner may hold fewer matching cards than a peer's
                // placeholder count suggests.
                spec = spec.allow_partial_completion();
            }
            let chosen: Vec<_> = make_decision_with_fallback(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
                FallbackStrategy::Maximum,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            if ctx.targets_are_cost_choices {
                if !is_exact_reveal_selection(&chosen, &valid_cards, required) {
                    return Err(ExecutionError::Impossible(
                        "reveal payment must select exactly the required legal hand cards".into(),
                    ));
                }
                chosen
            } else if hidden_hand_choice {
                // No fill-up: it would pick different cards on peers holding
                // placeholders.
                let mut normalized = Vec::new();
                for id in chosen {
                    if normalized.len() < required
                        && valid_cards.contains(&id)
                        && !normalized.contains(&id)
                    {
                        normalized.push(id);
                    }
                }
                normalized
            } else {
                normalize_object_selection(chosen, &valid_cards, required)
            }
        };
        if let Some(admission) =
            super::reveal::prospective_reveal_admission(ctx, cards_to_reveal.len())
        {
            return Ok(admission);
        }

        if ctx.targets_are_cost_choices
            && cards_to_reveal
                .iter()
                .any(|id| game.is_hidden_card_placeholder(*id))
        {
            // Public selection opens exactly these identities before replay.
            // A deferred hand claim proves neither their color/type nor that
            // the mandatory reveal completed in the current payment.
            return Err(ExecutionError::IncompleteEvidence(
                "reveal payment is awaiting its selected card's public identity opening".into(),
            ));
        }
        if hidden_hand_choice {
            let filter = reveal_filter(self, ctx.controller);
            let filter_ctx =
                crate::filter::FilterContext::new(ctx.controller).with_source(ctx.source);
            let description = self.cost_display();
            game.record_hidden_identity_obligations(
                &cards_to_reveal,
                &filter,
                &filter_ctx,
                &description,
            );
        }
        if reveal_publicly {
            // Every peer opened the chosen cards before this replay.
            game.mark_hidden_cards_publicly_revealed(&cards_to_reveal);
        }
        if cards_to_reveal.len() < required {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::impossible(),
            ));
        }

        let revealed = cards_to_reveal
            .iter()
            .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
            .collect();
        let actor = ctx.controller;
        super::reveal_objects_with_outputs(
            game,
            ctx,
            revealed,
            Some(actor),
            "Reveal cards from hand",
            None,
        )
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
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Revealed)
    }
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Some(source) = game.object(ctx.source) else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        };
        if source.owner != ctx.controller || source.zone != Zone::Hand {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        let source_id = ctx.source;
        let Some(snapshot) = ObjectSnapshot::from_object_id(game, source_id) else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        };
        if let Some(admission) = super::reveal::prospective_reveal_admission(ctx, 1) {
            return Ok(admission);
        }
        if self.duration
            == ironsmith_core::RevealSourceFromHandDuration::UntilUpkeepEndsOrLeavesHand
        {
            game.reveal_hand_card_until_upkeep_ends(source_id);
        }
        let actor = ctx.controller;
        super::reveal_objects_with_outputs(
            game,
            ctx,
            vec![snapshot],
            Some(actor),
            "Reveal source card from hand",
            None,
        )
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
        assert!(!game.is_hand_card_revealed_until_upkeep_ends(source));
    }

    #[test]
    fn forecast_reveal_persists_until_the_source_leaves_hand() {
        let mut game = create_test_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(101), "Forecast Card").build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Hand);

        let cost = Cost::effect(RevealSourceFromHandEffect::until_upkeep_ends_or_leaves_hand());
        let mut dm = CaptureViewDm::default();
        let mut ctx = CostContext::new(source, alice, &mut dm);

        assert_eq!(cost.pay(&mut game, &mut ctx), Ok(CostPaymentResult::Paid));
        assert!(game.is_hand_card_revealed_until_upkeep_ends(source));

        game.move_object_by_effect(source, Zone::Graveyard)
            .expect("Forecast source should move");
        assert!(!game.is_hand_card_revealed_until_upkeep_ends(source));
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

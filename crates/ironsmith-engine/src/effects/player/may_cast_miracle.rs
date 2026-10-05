//! May cast for miracle cost effect implementation.
//!
//! This effect is used by Miracle triggers to present the player with the choice
//! to cast the spell for its miracle cost.
//!
//! This effect uses the triggering event (CardsDrawnEvent) to find the card
//! that was drawn. This is more robust than storing card_id/owner because
//! it automatically handles zone changes.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::other::CardsDrawnEvent;
use crate::game_state::GameState;
use crate::zone::Zone;

use super::runtime_helpers::with_spell_cast_event;

/// Effect that allows casting a spell for its miracle cost.
///
/// When this effect resolves, it presents the player with a choice to cast
/// the spell for its miracle cost. If they choose yes and can pay the cost,
/// the spell is cast.
///
/// This effect gets the card and owner from the triggering CardsDrawnEvent.
/// The miracle card must be the first card in the event (is_miracle_eligible).
pub use ironsmith_core::MayCastForMiracleCostEffect;

impl EffectExecutor for MayCastForMiracleCostEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::alternative_cast::CastingMethod;

        // Get card_id and owner from the triggering CardsDrawnEvent
        let Some(ref triggering_event) = ctx.triggering_event else {
            return Err(ExecutionError::Impossible(
                "MayCastForMiracleCostEffect requires a triggering event".to_string(),
            ));
        };

        let Some(drawn) = triggering_event.downcast::<CardsDrawnEvent>() else {
            return Err(ExecutionError::Impossible(
                "MayCastForMiracleCostEffect requires a CardsDrawnEvent".to_string(),
            ));
        };

        if let Some(decision) = drawn.miracle.clone() {
            let drawn = drawn.clone();
            return execute_captured_miracle(game, ctx, &drawn, &decision);
        }

        // Get the first card drawn (miracle only works on the first card)
        let Some(card_id) = drawn.first_card() else {
            return Ok(EffectOutcome::impossible());
        };
        let owner = drawn.player;

        // Verify the card is still in hand
        let obj = game.object(card_id).ok_or(ExecutionError::InvalidTarget)?;

        if obj.zone != Zone::Hand {
            // Card is no longer in hand (may have been discarded or played)
            return Ok(EffectOutcome::target_invalid());
        }

        // Get the miracle cost
        let miracle_cost = obj
            .alternative_casts
            .iter()
            .find_map(|alt| alt.miracle_cost().cloned());

        let Some(miracle_cost) = miracle_cost else {
            // Card doesn't have miracle (shouldn't happen)
            return Ok(EffectOutcome::impossible());
        };

        // Find the miracle alternative cast index
        let miracle_index = obj
            .alternative_casts
            .iter()
            .position(|alt| alt.is_miracle());

        let Some(miracle_index) = miracle_index else {
            return Ok(EffectOutcome::impossible());
        };

        let card_name = obj.name.to_string();

        // Ask the player if they want to cast for miracle cost
        let bool_ctx = crate::decisions::context::BooleanContext::new(
            owner,
            Some(card_id),
            format!(
                "Cast {} for its miracle cost ({})?",
                card_name,
                miracle_cost.to_oracle()
            ),
        )
        .with_source_name(&card_name);

        let wants_to_cast = ctx.decision_maker.decide_boolean(game, &bool_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        if !wants_to_cast {
            // Player chose not to cast - card stays in hand
            return Ok(EffectOutcome::resolved());
        }

        let casting_method = CastingMethod::Alternative(miracle_index);
        game.authorize_miracle_cast(card_id);
        let result = crate::game_loop::cast_spell_from_resolving_effect(
            game,
            card_id,
            Zone::Hand,
            owner,
            &casting_method,
            false,
            None,
            ctx.provenance,
            &mut ctx.decision_maker,
        );
        game.revoke_miracle_cast(card_id);
        let result = result.map_err(super::runtime_helpers::effect_driven_cast_error)?;
        if let Some(new_id) = result {
            Ok(with_spell_cast_event(
                EffectOutcome::with_objects(vec![new_id]),
                game,
                new_id,
                owner,
                Zone::Hand,
                ctx.provenance,
            )?)
        } else if ctx.decision_maker.awaiting_choice() {
            Ok(EffectOutcome::count(0))
        } else {
            Ok(EffectOutcome::impossible())
        }
    }
}


fn execute_captured_miracle(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    drawn: &CardsDrawnEvent,
    decision: &crate::events::other::MiracleDrawDecision,
) -> Result<EffectOutcome, ExecutionError> {
    use crate::events::other::DrawnMiraclePrice;
    let proofs = decision.revealed_instances();
    if proofs.is_empty() { return Ok(EffectOutcome::impossible()); }
    let [proof] = proofs else {
        return Err(ExecutionError::IncompleteEvidence("a Miracle casting trigger has no single linked reveal instance".into()));
    };
    if !drawn.is_miracle_eligible(proof.card) || drawn.player != proof.player
        || ctx.source != proof.card
        || proof.drawn_snapshot.object_id != proof.card || proof.drawn_snapshot.stable_id != proof.stable_id
        || proof.drawn_snapshot.zone != Zone::Hand || proof.drawn_snapshot.owner != proof.player
    {
        return Err(ExecutionError::IncompleteEvidence("Miracle reveal does not identify this draw, source and casting player".into()));
    }
    let Some(object) = game.object(proof.card).filter(|object| object.zone == Zone::Hand
        && object.stable_id == proof.stable_id && object.owner == proof.player)
    else { return Ok(EffectOutcome::target_invalid()); };
    let price = match &proof.instance.price {
        DrawnMiraclePrice::Fixed(cost) => cost.to_oracle(),
        DrawnMiraclePrice::ReducedManaCost { generic_reduction, .. } =>
            format!("its mana cost reduced by {{{generic_reduction}}}"),
    };
    // A copy retains the exact draw/reveal but its controller makes the
    // resolution choice and pays to cast (CR 109.5, 707.10).
    let caster = ctx.controller;
    let question = crate::decisions::context::BooleanContext::new(caster, Some(proof.card),
        format!("Cast {} for its miracle cost ({price})?", object.name)).with_source_name(object.name.to_string());
    let accepts = ctx.decision_maker.decide_boolean(game, &question);
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    if !accepts { return Ok(EffectOutcome::resolved()); }
    let result = crate::game_loop::cast_spell_from_revealed_miracle(game, proof, caster, ctx.provenance, &mut ctx.decision_maker)
        .map_err(super::runtime_helpers::effect_driven_cast_error)?;
    if let Some(new_id) = result {
        with_spell_cast_event(EffectOutcome::with_objects(vec![new_id]), game,
            new_id, caster, Zone::Hand, ctx.provenance)
    } else if ctx.decision_maker.awaiting_choice() { Ok(EffectOutcome::count(0)) }
    else { Ok(EffectOutcome::impossible()) }
}

#[cfg(test)]
mod captured_price_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::Effect;
    use crate::events::other::{DrawnMiracleInstance, DrawnMiraclePrice, MiracleDrawDecision, MiracleInstanceIdentity, RevealedMiracle};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::triggers::TriggerEvent;
    struct Answers { x: u32 }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool { true }
        fn decide_number(&mut self, _: &GameState, context: &crate::decisions::context::NumberContext) -> u32 {
            assert!(self.x >= context.min && self.x <= context.max, "the recipe must retain affordable X headroom");
            self.x
        }
    }
    #[test]
    fn native_cast_uses_the_captured_recipe_after_granter_loss_and_announces_x() {
        for (base, x, generic_due) in [(ManaSymbol::Generic(6), 0, 2), (ManaSymbol::Generic(2), 0, 0), (ManaSymbol::X, 4, 0)] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            let mana_cost = ManaCost::from_symbols(vec![base, ManaSymbol::Black, ManaSymbol::Black]);
            let definition = CardBuilder::new(CardId::new(), "Captured miracle enchantment")
                .card_types(vec![crate::types::CardType::Enchantment]).mana_cost(mana_cost.clone()).build();
            let card = game.create_object_from_card(&definition, player, Zone::Hand);
            let stable_id = game.object(card).unwrap().stable_id;
            // The grantor is already absent. No intrinsic alternative price
            // or live grant may be consulted to reconstruct this permission.
            let grantor = ObjectId::new();
            let mut draw = CardsDrawnEvent::single(player, card, true);
            draw.miracle = Some(MiracleDrawDecision::Revealed(RevealedMiracle {
                drawn_snapshot: crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(card).unwrap(), &game),
                card, stable_id, player,
                instance: DrawnMiracleInstance {
                    identity: MiracleInstanceIdentity::Granted(crate::grant_registry::GrantPermissionIdentity::Stored(37)),
                    granting_source: grantor,
                    price: DrawnMiraclePrice::ReducedManaCost { mana_cost: Some(mana_cost), other_face_mana_cost: None, generic_reduction: 4 },
                },
            }));
            game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Black, 2);
            game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Colorless, generic_due);
            let mut answers = Answers { x };
            let mut ctx = ExecutionContext::new(card, player, &mut answers)
                .with_triggering_event(TriggerEvent::new_with_provenance(draw, Default::default()));
            let outcome = crate::effects::execute_effect(&mut game, &Effect::may_cast_for_miracle_cost(), &mut ctx).unwrap();
            assert_eq!(game.stack.len(), 1);
            let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
            assert_eq!(spell.stable_id, stable_id);
            assert!(spell.cast_alternative_method.as_ref().is_some_and(|method| method.is_miracle()));
            if x > 0 { assert_eq!(spell.x_value, Some(x)); }
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
            assert!(!game.miracle_cast_is_authorized(card));
            assert!(outcome.events.iter().any(|event| event.kind() == crate::events::EventKind::SpellCast));
        }
    }

    struct HybridAnswers { generic: bool, x: u32 }
    impl crate::decision::DecisionMaker for HybridAnswers {
        fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool { true }
        fn decide_number(&mut self, _: &GameState, context: &crate::decisions::context::NumberContext) -> u32 {
            assert!(self.x >= context.min && self.x <= context.max); self.x
        }
        fn decide_options(&mut self, game: &GameState, context: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
            let prefix = if self.generic { "{2}" } else { "{W}" };
            if let Some(option) = context.options.iter().find(|option| option.description.starts_with(prefix)) {
                vec![option.index]
            } else { crate::decision::SelectFirstDecisionMaker.decide_options(game, context) }
        }
    }
    #[test]
    fn miracle_recipe_reduces_announced_generic_hybrid_and_x_but_never_additional_costs() {
        for generic in [false, true] { for x in [0, 2] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let player = PlayerId::from_index(0);
            let mut pips = vec![vec![ManaSymbol::Generic(2), ManaSymbol::White], vec![ManaSymbol::Black]];
            if x > 0 { pips.insert(0, vec![ManaSymbol::X]); }
            let mana_cost = ManaCost::from_pips(pips);
            let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Hybrid miracle enchantment")
                .card_types(vec![crate::types::CardType::Enchantment]).mana_cost(mana_cost.clone())
                .additional_cost(crate::cost::TotalCost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(3)])))
                .build();
            let card = game.create_object_from_definition(&definition, player, Zone::Hand);
            let stable_id = game.object(card).unwrap().stable_id;
            let mut draw = CardsDrawnEvent::single(player, card, true);
            draw.miracle = Some(MiracleDrawDecision::Revealed(RevealedMiracle {
                drawn_snapshot: crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(game.object(card).unwrap(), &game),
                card, stable_id, player,
                instance: DrawnMiracleInstance {
                    identity: MiracleInstanceIdentity::Granted(crate::grant_registry::GrantPermissionIdentity::Stored(38)),
                    granting_source: ObjectId::new(),
                    price: DrawnMiraclePrice::ReducedManaCost { mana_cost: Some(mana_cost), other_face_mana_cost: None, generic_reduction: 4 },
                },
            }));
            game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Black, 1);
            game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Colorless, 3);
            if !generic { game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::White, 1); }
            let mut answers = HybridAnswers { generic, x };
            let mut ctx = ExecutionContext::new(card, player, &mut answers)
                .with_triggering_event(TriggerEvent::new_with_provenance(draw, Default::default()));
            crate::effects::execute_effect(&mut game, &Effect::may_cast_for_miracle_cost(), &mut ctx).unwrap();
            assert_eq!(game.stack.len(), 1);
            let spell = game.object(game.stack.last().unwrap().object_id).unwrap();
            assert_eq!(spell.stable_id, stable_id);
            let method = spell.cast_alternative_method.as_ref().unwrap();
            assert_eq!(method.miracle_cost().unwrap().mana_value(), if generic { 1 } else { 2 });
            assert_eq!(spell.mana_spent_to_cast.total(), if generic { 4 } else { 5 }, "the mandatory three mana remains payable");
            assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
        } }
    }

}

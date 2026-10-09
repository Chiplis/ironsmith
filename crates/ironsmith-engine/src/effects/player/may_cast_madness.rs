//! Madness's linked triggered ability (CR 702.35a).
//!
//! "When this card is exiled this way, its owner may cast it by paying [cost]
//! rather than paying its mana cost. If that player doesn't, they put this
//! card into their graveyard."
//!
//! The trigger is created when the madness replacement exiles a discarded
//! card. The spell is cast while it resolves through the normal cast
//! pipeline, so it's a real cast (cast triggers, X, the "Madness" paid label)
//! and it then waits on the stack like any other spell.

use crate::alternative_cast::CastingMethod;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::zone::Zone;

use super::runtime_helpers::complete_native_cast_with_outputs;

/// Resolves a madness trigger whose source is the exiled card.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MayCastForMadnessCostEffect;

impl MayCastForMadnessCostEffect {
    pub fn new() -> Self {
        Self
    }
}

fn put_madness_card_into_graveyard(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card_id: crate::ids::ObjectId,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    // Use the full zone-changing instruction owner: it consumes deferred
    // replacement programs and rolls the entire movement back on suspension
    // or failure before this linked madness marker is cleared.
    let outcome = crate::effects::MoveToZoneEffect::to_graveyard(
        crate::target::ChooseSpec::SpecificObject(card_id),
    )
    .execute_child_with_outputs(game, ctx)?;
    if !ctx.decision_maker.awaiting_choice() {
        game.clear_madness_exiled(card_id);
    }
    Ok(outcome)
}

/// Madness granted to a card by another object ("Each Vampire creature card
/// you own that isn't on the battlefield has madness. The madness cost is
/// equal to its mana cost.", Falkenrath Gorger) while it is in `zone`: the
/// granted method's casting route and cost (CR 702.35a).
pub(crate) fn granted_madness_route(
    game: &GameState,
    card_id: crate::ids::ObjectId,
    zone: Zone,
) -> Option<(CastingMethod, crate::cost::TotalCost)> {
    let card = game.object(card_id)?;
    let base = card.alternative_casts.len();
    game.effect_store
        .grant_registry
        .granted_alternative_casts_for_card(game, card_id, zone, card.owner)
        .iter()
        .enumerate()
        .find_map(|(offset, grant)| {
            let cost = grant.method.madness_cost()?.clone();
            Some((
                CastingMethod::PlayFrom {
                    source: grant.source_id,
                    zone,
                    use_alternative: Some(base + offset),
                },
                cost,
            ))
        })
}

impl EffectExecutor for MayCastForMadnessCostEffect {
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
        let card_id = ctx.source;
        // The card must still be the object the madness replacement exiled.
        let Some(card) = game.object(card_id) else {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::target_invalid(),
            ));
        };
        if card.zone != Zone::Exile || !game.is_madness_exiled(card_id) {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::target_invalid(),
            ));
        }
        let owner = card.owner;
        let printed = card
            .alternative_casts
            .iter()
            .enumerate()
            .find_map(|(index, method)| match method {
                crate::alternative_cast::AlternativeCastingMethod::Madness { total_cost } => {
                    Some((CastingMethod::Alternative(index), total_cost.clone()))
                }
                _ => None,
            });
        // A printed madness ability, else one granted to the card while it
        // is in exile.
        let Some((casting_method, madness_cost)) =
            printed.or_else(|| granted_madness_route(game, card_id, Zone::Exile))
        else {
            return put_madness_card_into_graveyard(game, ctx, card_id);
        };

        let wants_to_cast = crate::decisions::make_decision(
            game,
            ctx.decision_maker,
            owner,
            Some(card_id),
            crate::decisions::specs::MadnessSpec::new(card_id, madness_cost),
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        if !wants_to_cast {
            return put_madness_card_into_graveyard(game, ctx, card_id);
        }

        game.authorize_madness_cast(card_id);
        let result = crate::game_loop::cast_spell_from_resolving_effect_with_outputs(
            game,
            card_id,
            Zone::Exile,
            owner,
            &casting_method,
            false,
            None,
            ctx.provenance,
            &mut ctx.decision_maker,
        );
        game.revoke_madness_cast(card_id);
        let result = result.map_err(super::runtime_helpers::effect_driven_cast_error)?;
        if let Some(cast) = result {
            return complete_native_cast_with_outputs(
                EffectOutcome::with_objects(vec![cast.new_id]),
                game,
                cast,
                owner,
                Zone::Exile,
                ctx.provenance,
            );
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        // The cast didn't happen (for example, no legal targets or the cost
        // couldn't be paid), so the card goes to its owner's graveyard.
        if game
            .object(card_id)
            .is_some_and(|object| object.zone == Zone::Exile)
        {
            return put_madness_card_into_graveyard(game, ctx, card_id);
        }
        Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ))
    }
}

#[cfg(test)]
mod replacement_receipt_contract_tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId, StableId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::{ObjectFilter, PlayerFilter};

    struct Answers { stable: StableId, pause: bool, pending: bool, calls: usize }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            let arrived = game.find_object_by_stable_id(self.stable).unwrap();
            assert_eq!(game.object(arrived).unwrap().zone, Zone::Graveyard);
            assert_eq!(game.player(PlayerId::from_index(1)).unwrap().life, 23);
            self.pending = self.pause;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, owner: PlayerId, zone: Zone, name: &str) -> ObjectId {
        let card = crate::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        game.create_object_from_card(&card, owner, zone)
    }
    fn check(mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let source = card(&mut game, bob, Zone::Battlefield, "Madness replacement probe");
        let exiled = card(&mut game, alice, Zone::Exile, "Madness fallback probe");
        game.set_madness_exiled(exiled);
        let stable = game.object(exiled).unwrap().stable_id;
        let mut effects = vec![Effect::gain_life(3)];
        if mode == 2 { effects.push(Effect::lose_life(Value::X)); }
        else { effects.push(Effect::new(crate::effects::MayEffect::new_for_player(
            vec![Effect::gain_life(4)], PlayerFilter::You))); }
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            source, bob, crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(exiled), Some(Zone::Exile), Some(Zone::Graveyard)),
            ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events();
        let before_ids = game.next_object_id_counter();
        let mut dm = Answers { stable, pause: mode == 1, pending: false, calls: 0 };
        let mut ctx = ExecutionContext::new(exiled, alice, &mut dm);
        let result = MayCastForMadnessCostEffect::new().execute(&mut game, &mut ctx);
        if mode == 0 {
            result.unwrap();
            assert_eq!(game.player(bob).unwrap().life, 27);
            assert_eq!(game.object(game.find_object_by_stable_id(stable).unwrap()).unwrap().zone, Zone::Graveyard);
            assert!(!game.is_madness_exiled(exiled));
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        } else {
            if mode == 2 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
            else { result.unwrap(); assert!(ctx.decision_maker.awaiting_choice()); }
            assert_eq!(game.object(exiled).unwrap().zone, Zone::Exile);
            assert!(game.is_madness_exiled(exiled));
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode == 1 {
            assert_eq!(dm.calls, 1);
            let mut resumed = Answers { stable, pause: false, pending: false, calls: 0 };
            let mut fresh = ExecutionContext::new(exiled, alice, &mut resumed);
            MayCastForMadnessCostEffect::new().execute(&mut game, &mut fresh).unwrap();
            assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(!game.is_madness_exiled(exiled));
            assert_eq!(game.players[0].graveyard.len(), 1);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert!(!fresh.decision_maker.awaiting_choice());
            drop(fresh); assert_eq!(resumed.calls, 1);
        }
    }
    #[test] fn madness_fallback_executes_added_replacement_program() { check(0); }
    #[test] fn madness_fallback_added_pending_rolls_back_and_fresh_resume_commits_once() { check(1); }
    #[test] fn madness_fallback_added_error_restores_card_marker_life_and_shield() { check(2); }
}

//! Unearth effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification};
use crate::effect::{Effect, EffectOutcome, Until};
use crate::effects::zones::MoveToZoneEffect;
use crate::effects::{
    ApplyContinuousEffect, ApplyReplacementEffect, EffectExecutor, ScheduleDelayedTriggerEffect,
};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::events::zones::matchers::WouldLeaveBattlefieldMatcher;
use crate::game_state::GameState;
use crate::replacement::{ReplacementAction, ReplacementEffect};
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::Trigger;
use crate::zone::Zone;
pub use ironsmith_core::UnearthEffect;

/// Effect that executes the rules text for Unearth.
///
/// "Return this card from your graveyard to the battlefield. It gains haste.
/// Exile it at the beginning of the next end step or if it would leave the
/// battlefield."
impl EffectExecutor for UnearthEffect {
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            execute_unearth_with_outputs,
        )
    }
}

fn execute_unearth_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    let source_id = ctx.source;
    let Some(source_obj) = game.object(source_id) else {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::target_invalid(),
        ));
    };
    if source_obj.zone != Zone::Graveyard {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::target_invalid(),
        ));
    }

    let move_to_battlefield = Effect::new(
        MoveToZoneEffect::new(ChooseSpec::Source, Zone::Battlefield, false).under_owner_control(),
    );
    let move_outcome =
        crate::effects::execute_effect_with_outputs(game, &move_to_battlefield, ctx)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let new_id = if let Some(id) = move_outcome
        .outcome
        .instruction_result()
        .objects()
        .and_then(|objects| objects.first())
        .copied()
    {
        id
    } else {
        let EffectOutcome {
            status,
            value,
            events,
            execution_facts,
            instruction_result,
        } = move_outcome.outcome.clone();
        let status = if matches!(value, crate::effect::OutcomeValue::Objects(_)) {
            crate::effect::OutcomeStatus::TargetInvalid
        } else {
            status
        };
        let mut outcome = EffectOutcome::with_details(
            status,
            if status == crate::effect::OutcomeStatus::TargetInvalid {
                crate::effect::OutcomeValue::None
            } else {
                value
            },
            events,
            execution_facts,
        );
        outcome.instruction_result = instruction_result;
        return Ok(crate::effects::CompletedEffectOutputs::from_children(
            [move_outcome],
            |_| outcome,
        ));
    };
    // Later clauses refer to the original arrival, never a successor
    // found by stable identity or an object created by a replacement.
    if !game
        .object(new_id)
        .is_some_and(|object| object.zone == Zone::Battlefield)
        || game.is_phased_out(new_id)
    {
        return Ok(move_outcome);
    }
    let primary = move_outcome.outcome.summary_projection();
    let mut children = vec![move_outcome];

    // CR 702.84a gives the returned permanent haste without a duration.
    // It remains if the delayed exile trigger is countered.
    let haste_effect = ApplyContinuousEffect::new(
        EffectTarget::Specific(new_id),
        Modification::AddAbility(StaticAbility::haste()),
        Until::Forever,
    )
    .with_source_type(EffectSourceType::Resolution {
        locked_targets: vec![new_id],
    });
    children.push(haste_effect.execute_child_with_outputs(game, ctx)?);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }

    // "If it would leave the battlefield, exile it instead."
    let replacement = ReplacementEffect::with_matcher(
        new_id,
        ctx.controller,
        WouldLeaveBattlefieldMatcher::new(ObjectFilter::specific(new_id)),
        ReplacementAction::ChangeDestination(Zone::Exile),
    );
    children
        .push(ApplyReplacementEffect::one_shot(replacement).execute_child_with_outputs(game, ctx)?);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }

    // "Exile it at the beginning of the next end step."
    let schedule = ScheduleDelayedTriggerEffect::new(
        Trigger::beginning_of_end_step(PlayerFilter::Any),
        vec![Effect::exile(ChooseSpec::SpecificObject(new_id))],
        true,
        vec![new_id],
        PlayerFilter::Specific(ctx.controller),
    );
    children.push(schedule.execute_child_with_outputs(game, ctx)?);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    Ok(crate::effects::CompletedEffectOutputs::from_children(
        children,
        |children| EffectOutcome::aggregate_with_primary_result(primary, children),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_in_graveyard(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Graveyard);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_unearth_effect_returns_from_graveyard_and_sets_cleanup() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = create_creature_in_graveyard(&mut game, "Unearth Tester", alice);
        let mut ctx = ExecutionContext::new_default(source_id, alice);

        let result = UnearthEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("unearth should resolve");

        let crate::effect::OutcomeValue::Objects(ids) = result.value else {
            panic!("expected returned battlefield object");
        };
        let returned_id = ids[0];

        assert!(
            game.battlefield.contains(&returned_id),
            "unearthed card should be on battlefield"
        );
        assert_eq!(
            game.effect_store.delayed_triggers.len(),
            1,
            "unearthed card should have next end-step exile trigger"
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .one_shot_effects_snapshot()
                .len(),
            1,
            "unearthed card should register one-shot leave-battlefield replacement"
        );
    }

    #[test]
    fn test_unearth_leave_battlefield_replacement_exiles_instead() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = create_creature_in_graveyard(&mut game, "Unearth Tester", alice);
        let mut ctx = ExecutionContext::new_default(source_id, alice);

        let result = UnearthEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("unearth should resolve");
        let returned_id = match result.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            other => panic!("expected returned battlefield object, got {other:?}"),
        };

        let stable_id = game
            .object(returned_id)
            .expect("returned object should exist")
            .stable_id;

        let mut move_ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        let move_to_hand = Effect::new(MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(returned_id),
            Zone::Hand,
            false,
        ));
        let _ = execute_effect(&mut game, &move_to_hand, &mut move_ctx)
            .expect("move should resolve through replacement processing");

        let in_hand = game.players[0]
            .hand
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|obj| obj.stable_id == stable_id);
        let in_exile = game
            .exile
            .iter()
            .filter_map(|id| game.object(*id))
            .any(|obj| obj.stable_id == stable_id);

        assert!(
            !in_hand,
            "unearthed card should not go to hand when leaving battlefield"
        );
        assert!(
            in_exile,
            "unearthed card should be exiled when it would leave battlefield"
        );
    }
}

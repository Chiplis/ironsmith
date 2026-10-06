//! Remove up to any counters effect implementation.

use super::remove_counters::SelectedCounterRemovalPlan;
use crate::decision::FallbackStrategy;
use crate::decisions::{CounterRemovalSpec, DecisionSpec as _, make_decision_with_fallback};
use crate::effect::EffectOutcome;
use crate::effects::helpers::{
    resolve_single_object_for_effect, resolve_single_target_from_spec, resolve_value_wide,
};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, RemoveAnyCountersAmongEffect};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::{GameState, Target};
use crate::object::CounterType;
use crate::target::ChooseSpec;
pub use ironsmith_core::RemoveUpToAnyCountersEffect;

/// Effect that removes up to a number of counters of ANY type from a target.
///
/// Used by cards like Hex Parasite. The player chooses which counters to remove.
///
/// # Fields
///
/// * `max_count` - Maximum total counters the player can choose to remove
/// * `target` - Which permanent or player to target
///
/// # Example
///
/// ```ignore
/// // Remove up to X counters from target permanent
/// let effect = RemoveUpToAnyCountersEffect::new(Value::X, ChooseSpec::permanent());
/// ```
impl EffectExecutor for RemoveUpToAnyCountersEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(super::remove_counters::selected_counter_removal_proposal(
            super::remove_counters::CounterRemovalSelector::UpTo(self.clone()),
        ))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        game.clear_pending_decision_controllers();
        let result = crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| execute_up_to_any_counter_removal(self, game, ctx),
        );
        // The shared transaction restores the action; keep this adapter's
        // existing neutral suspension policy even if the child failed.
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "target to remove counters from"
    }
}

fn execute_up_to_any_counter_removal(
    effect: &RemoveUpToAnyCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let plan = select_up_to_any_counter_removal(effect, game, ctx)?;
    super::remove_counters::complete_selected_counter_removal_plan(game, ctx, plan)
}

pub(super) fn select_up_to_any_counter_removal(
    effect: &RemoveUpToAnyCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<SelectedCounterRemovalPlan, ExecutionError> {
    let max_count = resolve_value_wide(game, &effect.max_count, ctx)?.max(0) as u64;
    if let ChooseSpec::All(filter) = effect.target.unhinted() {
        if max_count > u64::from(u32::MAX) {
            return super::remove_any_counters_among::select_wide_counter_removal_among(
                game,
                ctx,
                filter.clone(),
                None,
                max_count,
                effect.up_to,
            );
        }
        let max_count = u32::try_from(max_count).expect("bounded branch");
        let min_count = if effect.up_to { 0 } else { max_count };
        let distributed =
            RemoveAnyCountersAmongEffect::dynamic(min_count, max_count, filter.clone(), false);
        return Ok(SelectedCounterRemovalPlan::Recorded(Box::new(
            super::remove_any_counters_among::select_distributed_counter_removal(
                &distributed,
                game,
                ctx,
            )?,
        )));
    }
    let target = match effect.target.base() {
        ChooseSpec::Player(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::ObjectOrPlayer(_, _)
        | ChooseSpec::PlayerOrPlaneswalker(_)
        | ChooseSpec::AttackedPlayerOrPlaneswalker
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::EachPlayer(_) => resolve_single_target_from_spec(game, &effect.target, ctx)?,
        _ => ResolvedTarget::Object(resolve_single_object_for_effect(game, ctx, &effect.target)?),
    };

    // Get available counters on the target
    let available_counters: Vec<(CounterType, u32)> = match target {
        ResolvedTarget::Object(target_id) => game.object(target_id).map(|object| {
            object
                .counters
                .iter()
                .filter(|(_, count)| **count > 0)
                .map(|(counter_type, count)| (*counter_type, *count))
                .collect()
        }),
        ResolvedTarget::Player(target_player) => game.player(target_player).map(|player| {
            player
                .counter_types_with_counters()
                .into_iter()
                .map(|counter_type| (counter_type, player.counter_count(counter_type)))
                .collect()
        }),
    }
    .unwrap_or_default();

    // Count total counters available
    let total_counters: u64 = available_counters
        .iter()
        .try_fold(0u64, |total, (_, count)| {
            total.checked_add(u64::from(*count)).ok_or_else(|| {
                ExecutionError::InternalError("available counter total overflow".into())
            })
        })?;

    // The actual maximum we can remove is the lesser of max_count and total available
    let actual_max = max_count.min(total_counters);

    // If there's nothing to remove, return 0
    if actual_max == 0 {
        return Ok(SelectedCounterRemovalPlan::Finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }

    // Ask the player which counters to remove using the spec-based system
    let min_count = if effect.up_to { 0 } else { actual_max };
    let decision_target = match target {
        ResolvedTarget::Object(id) => Target::Object(id),
        ResolvedTarget::Player(id) => Target::Player(id),
    };
    let spec = CounterRemovalSpec::for_target_wide(
        ctx.source,
        decision_target,
        actual_max,
        available_counters.clone(),
    )
    .with_min_total_wide(min_count);
    let mandatory_fallback = spec.default_response(FallbackStrategy::Maximum);
    let mut selections = make_decision_with_fallback(
        game,
        &mut ctx.decision_maker,
        ctx.controller,
        Some(ctx.source),
        spec,
        FallbackStrategy::Maximum,
    );
    if ctx.decision_maker.awaiting_choice() {
        return Ok(SelectedCounterRemovalPlan::Finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }
    if selections.iter().try_fold(0u64, |total, (_, count)| {
        total
            .checked_add(u64::from(*count))
            .ok_or_else(|| ExecutionError::InternalError("selected counter total overflow".into()))
    })? < min_count
    {
        selections = mandatory_fallback;
    }

    let mut selected_total = 0u64;
    let mut events = Vec::new();
    for (counter_type, requested) in selections {
        if selected_total >= actual_max {
            break;
        }
        let amount = requested.min(u32::try_from(actual_max - selected_total).unwrap_or(u32::MAX));
        if amount == 0 {
            continue;
        }
        let event = match target {
            ResolvedTarget::Object(target_id) => {
                crate::events::Event::remove_counters(target_id, counter_type, amount)
                    .with_provenance(ctx.provenance)
            }
            ResolvedTarget::Player(player) => crate::events::Event::new_with_provenance(
                crate::events::RemovePlayerCountersEvent::new(
                    player,
                    counter_type,
                    amount,
                    Some(ctx.source),
                    Some(ctx.controller),
                ),
                ctx.provenance,
            ),
        };
        selected_total += u64::from(amount);
        events.push(event);
    }
    Ok(SelectedCounterRemovalPlan::Groups {
        events,
        requested: selected_total,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_with_multiple_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(CounterType::PlusOnePlusOne, 3);
        obj.counters.insert(CounterType::MinusOneMinusOne, 2);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_remove_up_to_any_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_multiple_counters(&mut game, "Test Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Remove up to 4 counters of any type
        let effect = RemoveUpToAnyCountersEffect::new(4, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(4));
        let obj = game.object(creature_id).unwrap();
        // Default removes from first types in order
        let total_remaining: u32 = obj.counters.values().sum();
        assert_eq!(total_remaining, 1); // Started with 5, removed 4
    }

    #[test]
    fn test_remove_up_to_any_limited_by_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_multiple_counters(&mut game, "Test Creature", alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Request up to 10, but only 5 available (3 + 2)
        let effect = RemoveUpToAnyCountersEffect::new(10, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5)); // Limited by available
        let obj = game.object(creature_id).unwrap();
        let total_remaining: u32 = obj.counters.values().sum();
        assert_eq!(total_remaining, 0);
    }

    #[test]
    fn test_remove_up_to_any_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, "Empty Creature");
        let obj = Object::from_card(id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(id)]);

        let effect = RemoveUpToAnyCountersEffect::new(5, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_remove_up_to_any_counters_from_target_opponent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let bob_state = game.player_mut(bob).expect("Bob should exist");
        bob_state.poison_counters = 3;
        bob_state.energy_counters = 4;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let target = ChooseSpec::target(ChooseSpec::ObjectOrPlayer(
            crate::target::ObjectFilter::default()
                .with_type(CardType::Artifact)
                .with_type(CardType::Creature)
                .with_type(CardType::Planeswalker),
            crate::target::PlayerFilter::Opponent,
        ));
        let effect = RemoveUpToAnyCountersEffect::new(5, target);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(5));
        let bob_state = game.player(bob).expect("Bob should remain in the game");
        assert_eq!(bob_state.poison_counters, 0);
        assert_eq!(bob_state.energy_counters, 2);
    }

    #[test]
    fn test_remove_up_to_any_counters_clone_box() {
        let effect = RemoveUpToAnyCountersEffect::new(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveUpToAnyCountersEffect"));
    }
}

#[cfg(test)]
mod removal_quantity_event_contract_tests {
    use super::*;
    use crate::effect::{Effect,EffectId,Value};
    use crate::effects::{execute_effect,PutCountersEffect,RemoveUpToCountersEffect};
    use crate::ids::{CardId,PlayerId,ObjectId};
    fn object(game:&mut GameState,alice:PlayerId)->ObjectId {
        let card=crate::card::CardBuilder::new(CardId::new(),"Counter removal quantity owner").card_types(vec![crate::types::CardType::Artifact]).build();game.create_object_from_card(&card,alice,crate::zone::Zone::Battlefield)
    }
    fn put(game:&mut GameState,ctx:&mut ExecutionContext,id:ObjectId,kind:CounterType,count:u32,receipt:u32) {
        let effect=Effect::with_id(receipt,Effect::new(PutCountersEffect::new(kind,count,ChooseSpec::SpecificObject(id))));let out=execute_effect(game,&effect,ctx).unwrap();assert_eq!(out.as_count(),Some(i64::from(count)));assert_eq!(game.counter_count(id,kind),count);
    }
    fn bounded_prior(any:bool) {
        for amount in [i32::MAX as u32,i32::MAX as u32+1,u32::MAX] {
            let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let target=object(&mut game,alice);let following=object(&mut game,alice);let mut ctx=ExecutionContext::new_default(source,alice);
            put(&mut game,&mut ctx,source,CounterType::Charge,amount,31);put(&mut game,&mut ctx,target,CounterType::Charge,3,32);
            let value=Value::EffectValue(EffectId(31));let removal=if any {Effect::new(RemoveUpToAnyCountersEffect::new(value,ChooseSpec::SpecificObject(target)))} else {Effect::new(RemoveUpToCountersEffect::new(CounterType::Charge,value,ChooseSpec::SpecificObject(target)))};
            let out=execute_effect(&mut game,&Effect::with_id(57,removal),&mut ctx).expect("wide real maximum must be capped by available counters before the decision");assert_eq!(out.as_count(),Some(3));assert_eq!(game.counter_count(target,CounterType::Charge),0);assert_eq!(game.counter_count(source,CounterType::Charge),amount);assert!(!out.events.is_empty());
            let follow=Effect::new(PutCountersEffect::new(CounterType::Charge,Value::EffectValue(EffectId(57)),ChooseSpec::SpecificObject(following)));assert_eq!(execute_effect(&mut game,&follow,&mut ctx).unwrap().as_count(),Some(3));assert_eq!(game.counter_count(following,CounterType::Charge),3);
        }
    }
    #[test] fn typed_up_to_removal_caps_real_unsigned_prior_by_available_kind() {bounded_prior(false);}
    #[test] fn any_up_to_removal_caps_real_unsigned_prior_by_available_kinds() {bounded_prior(true);}
    #[test] fn small_any_removal_budget_works_with_mixed_total_above_u32() {
        let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let mut ctx=ExecutionContext::new_default(source,alice);
        for (kind,id) in [(CounterType::Charge,31),(CounterType::PlusOnePlusOne,32)] {put(&mut game,&mut ctx,source,kind,u32::MAX,id);}
        let out=RemoveUpToAnyCountersEffect::new(5,ChooseSpec::SpecificObject(source)).execute(&mut game,&mut ctx).expect("small realizable budget must not reject mixed available total");assert_eq!(out.as_count(),Some(5));let remaining=u64::from(game.counter_count(source,CounterType::Charge))+u64::from(game.counter_count(source,CounterType::PlusOnePlusOne));assert_eq!(remaining,2*u64::from(u32::MAX)-5);assert!(!out.events.is_empty());
    }
    #[test] fn any_removal_wide_budget_preserves_two_real_prior_receipts_and_followup() {
        let alice=PlayerId::from_index(0);let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let source=object(&mut game,alice);let following=object(&mut game,alice);let mut ctx=ExecutionContext::new_default(source,alice);
        for (kind,id) in [(CounterType::Charge,31),(CounterType::PlusOnePlusOne,32)] {put(&mut game,&mut ctx,source,kind,u32::MAX,id);}
        let maximum=Value::Add(Box::new(Value::EffectValue(EffectId(31))),Box::new(Value::EffectValue(EffectId(32))));let removal=Effect::with_id(57,Effect::new(RemoveUpToAnyCountersEffect::exact(maximum,ChooseSpec::SpecificObject(source))));let out=execute_effect(&mut game,&removal,&mut ctx).expect("logical mixed removal budget may exceed individual kind storage");assert_eq!(out.as_count(),Some(2*i64::from(u32::MAX)));assert_eq!(game.counter_count(source,CounterType::Charge),0);assert_eq!(game.counter_count(source,CounterType::PlusOnePlusOne),0);assert_eq!(out.events.len(),2);
        let follow=Effect::new(PutCountersEffect::new(CounterType::Charge,Value::HalfRoundedDown(Box::new(Value::EffectValue(EffectId(57)))),ChooseSpec::SpecificObject(following)));assert_eq!(execute_effect(&mut game,&follow,&mut ctx).unwrap().as_count(),Some(i64::from(u32::MAX)));assert_eq!(game.counter_count(following,CounterType::Charge),u32::MAX);
    }
}

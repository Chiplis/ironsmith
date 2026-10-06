//! Remove up to counters effect implementation.

use super::remove_counters::SelectedCounterRemovalPlan;
use crate::decision::FallbackStrategy;
use crate::decisions::{NumberSpec, make_decision_with_fallback};
use crate::effect::{EffectOutcome, Value};
use crate::effects::helpers::{resolve_single_object_for_effect, resolve_value_wide};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, RemoveAnyCountersAmongEffect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;

/// Effect that removes up to a number of counters from a target permanent.
///
/// The player chooses how many counters to remove (0 to max).
///
/// # Fields
///
/// * `counter_type` - The type of counter to remove
/// * `max_count` - Maximum counters the player can choose to remove
/// * `target` - Which permanent to target
///
/// # Example
///
/// ```ignore
/// // Remove up to two +1/+1 counters from target creature
/// let effect = RemoveUpToCountersEffect::new(
///     CounterType::PlusOnePlusOne,
///     2,
///     ChooseSpec::creature(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveUpToCountersEffect {
    /// The type of counter to remove.
    pub counter_type: CounterType,
    /// Maximum counters to remove.
    pub max_count: Value,
    /// Which permanent to target.
    pub target: ChooseSpec,
}

impl RemoveUpToCountersEffect {
    /// Create a new remove up to counters effect.
    pub fn new(counter_type: CounterType, max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            counter_type,
            max_count: max_count.into(),
            target,
        }
    }

    /// Create an effect that removes up to N +1/+1 counters.
    pub fn plus_one_counters(max_count: impl Into<Value>, target: ChooseSpec) -> Self {
        Self::new(CounterType::PlusOnePlusOne, max_count, target)
    }
}

impl EffectExecutor for RemoveUpToCountersEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(super::remove_counters::selected_counter_removal_proposal(
            super::remove_counters::CounterRemovalSelector::UpToKind(self.clone()),
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
            |game, ctx| execute_up_to_counter_removal(self, game, ctx),
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

fn execute_up_to_counter_removal(
    effect: &RemoveUpToCountersEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let plan = select_up_to_counter_removal(effect, game, ctx)?;
    super::remove_counters::complete_selected_counter_removal_plan(game, ctx, plan)
}

pub(super) fn select_up_to_counter_removal(
    effect: &RemoveUpToCountersEffect,
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
                Some(effect.counter_type),
                max_count,
                true,
            );
        }
        let distributed = RemoveAnyCountersAmongEffect::dynamic(
            0,
            u32::try_from(max_count).expect("bounded branch"),
            filter.clone(),
            false,
        )
        .with_counter_type(Some(effect.counter_type));
        return Ok(SelectedCounterRemovalPlan::Recorded(Box::new(
            super::remove_any_counters_among::select_distributed_counter_removal(
                &distributed,
                game,
                ctx,
            )?,
        )));
    }
    let target_id = resolve_single_object_for_effect(game, ctx, &effect.target)?;

    // Get the current count of counters on the target
    let available = game
        .object(target_id)
        .map(|obj| obj.counters.get(&effect.counter_type).copied().unwrap_or(0))
        .unwrap_or(0);

    // The actual maximum we can remove is the lesser of max_count and available
    let actual_max =
        u32::try_from(max_count.min(u64::from(available))).expect("bounded by per-kind storage");

    // If there's nothing to remove, return 0
    if actual_max == 0 {
        return Ok(SelectedCounterRemovalPlan::Finished(
            CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        ));
    }

    // Ask the player how many counters to remove (0 to actual_max)
    let description = format!(
        "Choose how many {} counters to remove (0-{})",
        effect.counter_type.description(),
        actual_max
    );
    let spec = NumberSpec::up_to(ctx.source, actual_max, description);
    let chosen_count = make_decision_with_fallback(
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
    let chosen_count = chosen_count.min(actual_max);

    let event = crate::events::Event::remove_counters(target_id, effect.counter_type, chosen_count)
        .with_provenance(ctx.provenance);
    Ok(SelectedCounterRemovalPlan::Single(event))
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

    fn create_creature_with_counters(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        counter_type: CounterType,
        count: u32,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.counters.insert(counter_type, count);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_remove_up_to_counters_default_max() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            5,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // No decision maker, so defaults to removing maximum
        let effect = RemoveUpToCountersEffect::plus_one_counters(3, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        let obj = game.object(creature_id).unwrap();
        assert_eq!(obj.counters.get(&CounterType::PlusOnePlusOne), Some(&2)); // 5 - 3
    }

    #[test]
    fn test_remove_up_to_limited_by_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Hangarback Walker",
            alice,
            CounterType::PlusOnePlusOne,
            2,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        // Request up to 5, but only 2 available
        let effect = RemoveUpToCountersEffect::plus_one_counters(5, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2)); // Limited by available
        // When all counters are removed, the entry is removed from the HashMap
        assert_eq!(
            game.counter_count(creature_id, CounterType::PlusOnePlusOne),
            0
        );
    }

    #[test]
    fn test_remove_up_to_no_counters() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature_id = create_creature_with_counters(
            &mut game,
            "Grizzly Bears",
            alice,
            CounterType::PlusOnePlusOne,
            0,
        );
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let effect = RemoveUpToCountersEffect::plus_one_counters(3, ChooseSpec::creature());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
    }

    #[test]
    fn test_remove_up_to_counters_distributes_across_all_matching_permanents() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = create_creature_with_counters(
            &mut game,
            "First Stunned Creature",
            alice,
            CounterType::Stun,
            1,
        );
        let second = create_creature_with_counters(
            &mut game,
            "Second Stunned Creature",
            bob,
            CounterType::Stun,
            1,
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = RemoveUpToCountersEffect::new(
            CounterType::Stun,
            2,
            ChooseSpec::All(crate::filter::ObjectFilter::permanent().in_zone(Zone::Battlefield)),
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(game.counter_count(first, CounterType::Stun), 0);
        assert_eq!(game.counter_count(second, CounterType::Stun), 0);
    }

    #[test]
    fn test_remove_up_to_counters_clone_box() {
        let effect = RemoveUpToCountersEffect::plus_one_counters(1, ChooseSpec::creature());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("RemoveUpToCountersEffect"));
    }
}

#[cfg(test)]
mod removal_caller_replacement_owner_tests {
    use super::*;
    fn check_owner(any_from_source: bool, instead: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Removal caller source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let source = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let counter_type = crate::object::CounterType::Charge;
        game.object_mut(source).unwrap().counters.insert(counter_type, 3);
        let action = if instead {
            crate::replacement::ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(2)])
        } else { crate::replacement::ReplacementAction::Prevent };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(source, alice,
                crate::events::counters::matchers::WouldRemoveCountersMatcher::any(), action));
        let effect = if any_from_source {
            crate::effect::Effect::new(crate::effects::RemoveAnyCountersFromSourceEffect::all(Some(counter_type)))
        } else {
            crate::effect::Effect::new(RemoveUpToCountersEffect::new(counter_type, 2, ChooseSpec::Source))
        };
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        assert_eq!(game.counter_count(source, counter_type), 3, "every removal owner must process its replacement proposal");
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(game.player(alice).unwrap().life, if instead { 22 } else { 20 });
        assert_eq!(outcome.events_of_type::<crate::events::MarkersChangedEvent>().count(), 0);
        assert_eq!(outcome.events_of_type::<crate::events::LifeGainEvent>().count(), usize::from(instead));
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        let next = crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
        let actual = if any_from_source { 3 } else { 2 };
        assert_eq!(next.count_or_zero(), actual);
        assert_eq!(game.counter_count(source, counter_type), 3 - actual as u32);
        assert_eq!(next.events_of_type::<crate::events::MarkersChangedEvent>().count(), 1);
        assert_eq!(game.player(alice).unwrap().life, if instead { 22 } else { 20 });
    }
    #[test]
    fn up_to_counter_removal_honors_prevention() { check_owner(false, false); }
    #[test]
    fn up_to_counter_removal_honors_instead_program() { check_owner(false, true); }
    #[test]
    fn source_any_counter_removal_honors_prevention() { check_owner(true, false); }
    #[test]
    fn source_any_counter_removal_honors_instead_program() { check_owner(true, true); }
}

//! Earthbend effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification, PtSublayer};
use crate::effect::{Effect, EffectOutcome, Until, Value};
#[cfg(test)]
use crate::effects::ResolvedTarget;
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::effects::{
    ApplyContinuousEffect, CompletedEffectOutputs, EffectExecutor, ScheduleDelayedTriggerEffect,
    TargetReusePolicy,
};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect_with_outputs};
use crate::events::KeywordActionKind;
use crate::game_state::GameState;
use crate::object::CounterType;
use crate::target::ChooseSpec;
use crate::triggers::Trigger;
use crate::types::CardType;

/// Earthbend effect: make target land a 0/0 creature with haste, put counters,
/// and return it tapped if it dies or is exiled.
pub type EarthbendEffect = ironsmith_core::EarthbendEffect;

impl EffectExecutor for EarthbendEffect {
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
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let target = resolve_single_object_for_effect(game, ctx, &self.target)?;
                if self.awaken {
                    return execute_land_animation_with_outputs(
                        game,
                        ctx,
                        target,
                        self.counters,
                        true,
                        None,
                    );
                }
                let subject = keyword_object_snapshot(game, target)?
                    .into_iter()
                    .collect::<Vec<_>>();
                let proposal = crate::events::KeywordActionEvent::new(
                    KeywordActionKind::Earthbend,
                    ctx.controller,
                    ctx.source,
                    self.counters,
                )
                .with_snapshot(
                    keyword_object_snapshot(game, ctx.source)?
                        .or_else(|| ctx.source_snapshot.clone()),
                )
                .with_object_tags(std::collections::HashMap::from([
                    ("it".into(), subject.clone()),
                    ("__it__".into(), subject),
                ]));
                let event = crate::events::Event::new_with_provenance(proposal, ctx.provenance);
                crate::effects::composition::execute_keyword_action_with_outputs(
                    game,
                    ctx,
                    event,
                    crate::effects::composition::KeywordActionOutput::Body,
                    crate::effects::composition::KeywordActionAmount::BodyMagnitude,
                    |game, ctx, action| {
                        execute_land_animation_with_outputs(
                            game,
                            ctx,
                            target,
                            action.amount,
                            false,
                            Some(action),
                        )
                    },
                )
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_reuse_policy(&self) -> TargetReusePolicy {
        TargetReusePolicy::AlwaysDeclareNew
    }

    fn target_description(&self) -> &'static str {
        "target land you control"
    }
}

/// Only capture the exact incarnation. A departed object is supplied by the
/// completed child receipt or the last observation, never by stable identity.
fn keyword_object_snapshot(
    game: &GameState,
    object: crate::ids::ObjectId,
) -> Result<Option<crate::snapshot::ObjectSnapshot>, ExecutionError> {
    if game.object(object).is_none() {
        return Ok(None);
    }
    let observed = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let effects = observed
        .try_all_continuous_effects_arc()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    Ok(observed.object(object).map(|object| {
        crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(
            object, &observed, &effects,
        )
    }))
}

fn execute_land_animation_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    target: crate::ids::ObjectId,
    amount: u32,
    awaken: bool,
    action: Option<&crate::events::KeywordActionEvent>,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let mut outcomes = Vec::new();
    // Awaken places counters while the object is still a land. Counter
    // replacements must see that world, rather than an animated one.
    if awaken {
        outcomes.push(place_animation_counters(game, ctx, target, amount)?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
    }
    // A replacement program may have removed this incarnation.
    // Never animate a successor found by stable identity.
    if game
        .object(target)
        .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
        && !game.is_phased_out(target)
    {
        for effect in land_animation_program(target, awaken) {
            outcomes.push(execute_effect_with_outputs(game, &effect, ctx)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
        }
    }
    if awaken {
        return Ok(collect_animation_outputs(outcomes));
    }
    let last_subject = keyword_object_snapshot(game, target)?.or_else(|| {
        action.and_then(|action| {
            action
                .object_tags
                .get("it")
                .and_then(|objects| objects.first())
                .cloned()
        })
    });
    // Earthbend's counters follow animation. Its watcher is created
    // afterwards and cannot retroactively observe a departure caused
    // by a counter replacement program (CR 701.66, 603.7).
    outcomes.push(place_animation_counters(game, ctx, target, amount)?);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let placed_subject = outcomes.last().and_then(|outcome| {
        // Auxiliary programs are not original placement results, but a
        // departure they caused supplies the exact land's latest LKI. Read
        // the full observation stream only for this identity, never adopt a
        // replacement program's output object as the earthbent land.
        outcome
            .outcome
            .events
            .iter()
            .rev()
            .filter_map(|event| event.downcast::<crate::events::zones::ZoneChangeEvent>())
            .flat_map(|event| event.snapshots().iter())
            .find(|object| object.object_id == target)
            .cloned()
            .or_else(|| {
                outcome
                    .outcome
                    .instruction_result()
                    .affected_object_memory()
                    .and_then(|objects| objects.iter().find(|object| object.object_id == target))
                    .cloned()
            })
    });
    let schedule = ScheduleDelayedTriggerEffect::new(
        Trigger::this_dies_or_is_exiled(),
        vec![Effect::return_from_graveyard_or_exile_to_battlefield(true)],
        true,
        vec![target],
        crate::target::PlayerFilter::Specific(ctx.controller),
    )
    .with_ability_source(target);
    outcomes.push(schedule.execute_child_with_outputs(game, ctx)?);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    let mut completion = action
        .ok_or_else(|| {
            ExecutionError::InternalError("earthbend body lost its action proposal".into())
        })?
        .clone();
    // The land is the subject; the instruction source can be another
    // object entirely. Freeze the completed incarnation before additions.
    let subject = keyword_object_snapshot(game, target)?
        .or(placed_subject)
        .or(last_subject);
    let subjects = subject.into_iter().collect::<Vec<_>>();
    completion.object_tags.insert("it".into(), subjects.clone());
    completion.object_tags.insert("__it__".into(), subjects);
    completion.snapshot = keyword_object_snapshot(game, completion.source)?.or(completion.snapshot);
    let notification = crate::effects::composition::complete_keyword_action(game, ctx, completion)?;
    let outcome = EffectOutcome::aggregate_with_primary_result(
        EffectOutcome::resolved(),
        outcomes
            .iter()
            .map(|child| child.outcome.clone())
            .chain([notification.clone()]),
    );
    let mut outputs = collect_animation_outputs(outcomes);
    outputs.retain_batch_children([CompletedEffectOutputs::aggregate_only(notification)]);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::count(0),
        ));
    }
    Ok(outputs.project_aggregate(outcome))
}

fn collect_animation_outputs(children: Vec<CompletedEffectOutputs>) -> CompletedEffectOutputs {
    let outcome = EffectOutcome::aggregate_with_primary_result(
        EffectOutcome::resolved(),
        children.iter().map(|child| child.outcome.clone()),
    );
    let mut outputs = CompletedEffectOutputs::aggregate_only(outcome);
    for child in children {
        outputs.retain_owned_child(child);
    }
    outputs
}

/// Pure composition plan; all registration is owned by ApplyContinuousEffect.
fn land_animation_program(target: crate::ids::ObjectId, elemental: bool) -> Vec<Effect> {
    let mut modifications = vec![Modification::AddCardTypes(vec![CardType::Creature])];
    if elemental {
        modifications.push(Modification::AddSubtypes(vec![
            crate::types::Subtype::Elemental,
        ]));
    }
    modifications.extend([
        Modification::SetPowerToughness {
            power: Value::Fixed(0),
            toughness: Value::Fixed(0),
            sublayer: PtSublayer::Setting,
        },
        Modification::AddAbility(crate::static_abilities::StaticAbility::haste()),
    ]);
    modifications
        .into_iter()
        .map(|modification| {
            Effect::new(
                ApplyContinuousEffect::new(
                    EffectTarget::Specific(target),
                    modification,
                    Until::Forever,
                )
                .with_source_type(EffectSourceType::Resolution {
                    locked_targets: vec![target],
                }),
            )
        })
        .collect()
}

fn place_animation_counters(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    target: crate::ids::ObjectId,
    amount: u32,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let event = crate::events::Event::put_counters(
        target,
        CounterType::PlusOnePlusOne,
        amount,
        ctx.cause.clone(),
    )
    .with_provenance(ctx.provenance);
    crate::effects::counters::execute_counter_placement_with_outputs(game, ctx, event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn targeted_earthbend_puts_counters_on_selected_land() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(881_001), "Awaken Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let land = game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Battlefield,
        );

        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
        let effect = EarthbendEffect::new(target, 4);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(land)]);

        effect
            .execute(&mut game, &mut ctx)
            .expect("targeted earthbend should resolve");

        assert_eq!(
            game.counter_count(land, CounterType::PlusOnePlusOne),
            4,
            "earthbend should put counters on the chosen land"
        );
        assert_eq!(game.calculated_power(land), Some(4));
        assert_eq!(game.calculated_toughness(land), Some(4));
    }

    #[test]
    fn earthbend_returns_a_sacrificed_land_to_the_battlefield() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(881_002), "Awaken Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let land = game.create_object_from_definition(
            &crate::cards::definitions::basic_plains(),
            alice,
            Zone::Battlefield,
        );

        let effect = EarthbendEffect::new(
            ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control())),
            1,
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(land)]);
        effect
            .execute(&mut game, &mut ctx)
            .expect("earthbend should resolve");
        let land_stable_id = game.object(land).expect("land should exist").stable_id;

        game.move_object_by_effect(land, Zone::Graveyard)
            .expect("the earthbent land should be sacrificed");
        let mut triggered = Vec::new();
        for event in game.take_pending_trigger_events() {
            triggered.extend(crate::triggers::check_delayed_triggers(&mut game, &event));
        }

        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].source, land);
        let mut trigger_queue = crate::triggers::TriggerQueue::new();
        for entry in triggered {
            trigger_queue.add(entry);
        }
        crate::game_loop::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("earthbend trigger should reach the stack");
        crate::game_loop::resolve_stack_entry(&mut game).expect("earthbend return should resolve");

        let returned_land = game
            .find_object_by_stable_id(land_stable_id)
            .expect("earthbent land should still exist");
        assert_eq!(
            game.object(returned_land).map(|object| object.zone),
            Some(Zone::Battlefield)
        );
        assert!(game.is_tapped(returned_land));
    }
}

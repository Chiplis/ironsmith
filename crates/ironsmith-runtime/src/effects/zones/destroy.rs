//! Destroy effect implementation.

use crate::effect::{
    ChoiceCount, EffectOutcome, ExecutionFact, OutcomeObjectMemory, OutcomeStatus,
};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    ObjectApplyResultPolicy, apply_single_target_object_from_spec, apply_to_selected_objects,
};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{EventOutcome, process_destroy};
use crate::events::zones::ZoneChangeEvent;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

/// Effect that destroys permanents.
///
/// Destruction moves permanents from the battlefield to the graveyard,
/// subject to replacement effects (regeneration, indestructible, etc.).
///
/// Supports both targeted and non-targeted (all) selection modes.
///
/// # Examples
///
/// ```ignore
/// // Destroy target creature (targeted - can fizzle)
/// let effect = DestroyEffect::target(ChooseSpec::creature());
///
/// // Destroy all creatures (non-targeted - cannot fizzle)
/// let effect = DestroyEffect::all(ObjectFilter::creature());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DestroyEffect {
    /// What to destroy - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
}

impl DestroyEffect {
    /// Create a destroy effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self { spec }
    }

    /// Create a targeted destroy effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
        }
    }

    /// Create a targeted destroy effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
        }
    }

    /// Create a non-targeted destroy effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
        }
    }

    /// Create a destroy effect targeting any creature.
    pub fn creature() -> Self {
        Self::target(ChooseSpec::creature())
    }

    /// Create a destroy effect targeting any permanent.
    pub fn permanent() -> Self {
        Self::target(ChooseSpec::permanent())
    }

    /// Helper to destroy a single object (shared logic).
    ///
    /// Uses `process_destroy` to handle all destruction logic through
    /// the trait-based event/replacement system with decision maker support.
    fn destroy_object(
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Result<Option<OutcomeStatus>, ExecutionError> {
        let pre_snapshot = game
            .object(object_id)
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
        let result = process_destroy(game, object_id, Some(ctx.source), &mut *ctx.decision_maker);
        if let Some(snapshot) = pre_snapshot
            && !game
                .object(object_id)
                .is_some_and(|obj| obj.zone == Zone::Battlefield)
        {
            ctx.refresh_target_snapshot(snapshot.clone());
            if snapshot.object_id == ctx.source {
                ctx.refresh_source_snapshot(snapshot);
            }
        }

        match result {
            EventOutcome::Proceed(_) => Ok(None), // Successfully destroyed
            EventOutcome::Prevented => Ok(Some(crate::effect::OutcomeStatus::Protected)),
            EventOutcome::Replaced => Ok(Some(crate::effect::OutcomeStatus::Replaced)),
            EventOutcome::NotApplicable => Ok(Some(crate::effect::OutcomeStatus::TargetInvalid)),
        }
    }
}

impl EffectExecutor for DestroyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        if self.spec.is_target() && self.spec.is_single() {
            return apply_single_target_object_from_spec(
                game,
                ctx,
                &self.spec,
                |game, ctx, object_id| Self::destroy_object(game, ctx, object_id),
            );
        }

        // For all/multi-target effects, count only successful destructions.
        let pending_start = game.effect_store.pending_trigger_events.len();
        let mut destroyed_objects = Vec::new();
        let mut destroyed_memory = Vec::new();
        let mut graveyard_zone_changes = Vec::new();
        let apply_result = match apply_to_selected_objects(
            game,
            ctx,
            &self.spec,
            ObjectApplyResultPolicy::CountApplied,
            |game, ctx, object_id| {
                let pre_snapshot = game.object(object_id).map(|obj| {
                    ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
                });
                let result =
                    process_destroy(game, object_id, Some(ctx.source), &mut *ctx.decision_maker);
                if let Some(snapshot) = pre_snapshot.as_ref()
                    && !game
                        .object(object_id)
                        .is_some_and(|obj| obj.zone == Zone::Battlefield)
                {
                    ctx.refresh_target_snapshot(snapshot.clone());
                    if snapshot.object_id == ctx.source {
                        ctx.refresh_source_snapshot(snapshot.clone());
                    }
                }
                if matches!(result, EventOutcome::Proceed(crate::zone::Zone::Graveyard)) {
                    if let Some(snapshot) = pre_snapshot.as_ref() {
                        destroyed_memory.push(OutcomeObjectMemory::from_snapshot(snapshot));
                    }
                    let result_objects = game.take_zone_change_results(object_id);
                    if let Some(snapshot) = pre_snapshot {
                        graveyard_zone_changes.push((object_id, result_objects.clone(), snapshot));
                    }
                    destroyed_objects.extend(result_objects);
                    return Ok(true);
                }
                Ok(false)
            },
        ) {
            Ok(result) => result,
            Err(_) => return Ok(EffectOutcome::target_invalid()),
        };

        if graveyard_zone_changes.len() > 1 {
            let event_objects = graveyard_zone_changes
                .iter()
                .map(|(id, _, _)| *id)
                .collect::<Vec<_>>();
            let result_objects = graveyard_zone_changes
                .iter()
                .flat_map(|(_, result_ids, _)| result_ids.iter().copied())
                .collect::<Vec<_>>();
            let snapshots = graveyard_zone_changes
                .iter()
                .map(|(_, _, snapshot)| snapshot.clone())
                .collect::<Vec<_>>();

            let removed =
                game.remove_pending_trigger_events_matching_from(pending_start, |event| {
                    let Some(zone_change) = event.downcast::<ZoneChangeEvent>() else {
                        return false;
                    };
                    zone_change.from == Zone::Battlefield
                        && zone_change.to == Zone::Graveyard
                        && zone_change.objects.len() == 1
                        && event_objects.contains(&zone_change.objects[0])
                });

            if !removed.is_empty() {
                let mut event = ZoneChangeEvent::batch_with_snapshots(
                    event_objects,
                    Zone::Battlefield,
                    Zone::Graveyard,
                    ctx.cause.clone(),
                    snapshots,
                );
                event.result_objects = result_objects;
                game.queue_trigger_event(
                    ctx.provenance,
                    TriggerEvent::new_with_provenance(event, ctx.provenance),
                );
            }
        }

        let mut outcome = apply_result.outcome;
        if !destroyed_objects.is_empty() {
            outcome =
                outcome.with_execution_fact(ExecutionFact::AffectedObjects(destroyed_objects));
            outcome = outcome.with_affected_object_memory(destroyed_memory);
        }

        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to destroy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::ColorSet;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, ResolvedTarget};
    use crate::filter::ObjectRef;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::types::Subtype;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, owner: PlayerId, name: &str, id_raw: u32) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(id_raw), name)
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn create_elephant_token() -> crate::cards::CardDefinition {
        crate::cards::CardDefinition::new(
            CardBuilder::new(CardId::new(), "Elephant")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Elephant])
                .color_indicator(ColorSet::GREEN)
                .power_toughness(PowerToughness::fixed(3, 3))
                .token()
                .build(),
        )
    }

    #[test]
    fn destroy_multi_target_records_graveyard_results_for_tagged_followups() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let first = create_creature(&mut game, bob, "First Target", 50_001);
        let second = create_creature(&mut game, bob, "Second Target", 50_002);

        let spec = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let effect = DestroyEffect::with_spec(spec.clone());
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![
                ResolvedTarget::Object(first),
                ResolvedTarget::Object(second),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec,
                range: 0..2,
            }]);

        let outcome = effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(outcome.as_count(), Some(2));
        assert_eq!(outcome.output_objects().len(), 2);
        assert!(
            outcome.output_objects().iter().all(|id| {
                game.object(*id).is_some_and(|obj| {
                    obj.zone == Zone::Graveyard && game.controller_of(obj) == bob
                })
            }),
            "destroy effect should surface the graveyard objects for tagged follow-ups, got {:?}",
            outcome.output_objects()
        );
    }

    #[test]
    fn destroy_multi_target_records_actual_object_memory_not_only_count() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let first = create_creature(&mut game, alice, "Alice Target", 50_011);
        let second = create_creature(&mut game, bob, "Bob Target", 50_012);
        let first_stable_id = game.object(first).expect("first target").stable_id;
        let second_stable_id = game.object(second).expect("second target").stable_id;

        let spec = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let effect = DestroyEffect::with_spec(spec.clone());
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![
                ResolvedTarget::Object(first),
                ResolvedTarget::Object(second),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec,
                range: 0..2,
            }]);

        let outcome = effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(outcome.as_count(), Some(2));
        let memory = outcome
            .affected_object_memory()
            .expect("destroyed object memory should be recorded");
        assert_eq!(memory.len(), 2);
        assert_eq!(memory[0].stable_id, first_stable_id);
        assert_eq!(memory[0].controller, alice);
        assert_eq!(memory[0].zone, Zone::Battlefield);
        assert_eq!(memory[0].power, Some(2));
        assert_eq!(memory[0].toughness, Some(2));
        assert!(memory[0].card_types.contains(&CardType::Creature));
        assert_eq!(memory[1].stable_id, second_stable_id);
        assert_eq!(memory[1].controller, bob);
        assert_eq!(memory[1].zone, Zone::Battlefield);
        assert_eq!(memory[1].power, Some(2));
        assert_eq!(memory[1].toughness, Some(2));
        assert!(memory[1].card_types.contains(&CardType::Creature));
        assert_eq!(outcome.output_objects().len(), 2);
        assert!(outcome.output_objects().iter().all(|id| {
            game.object(*id)
                .is_some_and(|obj| obj.zone == Zone::Graveyard)
        }));
    }

    #[test]
    fn destroy_multi_target_tagged_followup_uses_each_destroyed_objects_controller() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let alice_target = create_creature(&mut game, alice, "Alice Target", 50_101);
        let bob_target = create_creature(&mut game, bob, "Bob Target", 50_102);
        let spec = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let destroy = Effect::new(DestroyEffect::with_spec(spec.clone())).tag("destroyed");
        let create_elephants = Effect::for_each_tagged(
            "destroyed",
            vec![Effect::create_tokens_player(
                create_elephant_token(),
                1,
                PlayerFilter::ControllerOf(ObjectRef::tagged("__it__")),
            )],
        );
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![
                ResolvedTarget::Object(alice_target),
                ResolvedTarget::Object(bob_target),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec,
                range: 0..2,
            }]);

        crate::effects::execute_effect(&mut game, &destroy, &mut ctx).expect("destroy resolves");
        crate::effects::execute_effect(&mut game, &create_elephants, &mut ctx)
            .expect("follow-up resolves");

        let alice_elephants = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Elephant" && game.controller_of(obj) == alice)
            })
            .count();
        let bob_elephants = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Elephant" && game.controller_of(obj) == bob)
            })
            .count();

        assert_eq!(alice_elephants, 1);
        assert_eq!(bob_elephants, 1);
    }
}

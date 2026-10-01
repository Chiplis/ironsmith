//! Destroy effect implementation.

use crate::effect::{
    ChoiceCount, EffectOutcome, ExecutionFact, OutcomeObjectMemory, OutcomeStatus,
};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{apply_single_target_object_from_spec, resolve_objects_for_effect};
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
        can_be_regenerated: bool,
        receipts: &mut Vec<crate::events::processing::DestroyExecutionReceipt>,
    ) -> Result<Option<OutcomeStatus>, ExecutionError> {
        let pre_snapshot = game
            .object(object_id)
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
        let result = process_destroy_with_regeneration(
            game,
            object_id,
            Some(ctx.source),
            ctx,
            can_be_regenerated,
        )?;
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

        let Some(receipt) = result else { return Ok(None); };
        let original = receipt.result.clone();
        receipts.push(receipt);
        match original {
            EventOutcome::Proceed(_) => Ok(None), // Successfully destroyed
            EventOutcome::Prevented => Ok(Some(crate::effect::OutcomeStatus::Protected)),
            EventOutcome::Replaced => Ok(Some(crate::effect::OutcomeStatus::Replaced)),
            EventOutcome::NotApplicable => Ok(Some(crate::effect::OutcomeStatus::TargetInvalid)),
        }
    }
}

/// Destroy one permanent, honoring "can't be regenerated" (CR 701.19c).
///
/// The permanent's regeneration shields can't replace this destruction, but
/// they aren't used up by it: when the permanent survives (indestructible, a
/// shield counter, "can't be destroyed"), its shields are still there.
pub(crate) fn process_destroy_with_regeneration(
    game: &mut GameState, object_id: crate::ids::ObjectId, source: Option<crate::ids::ObjectId>,
    ctx: &mut ExecutionContext, can_be_regenerated: bool,
) -> Result<Option<crate::events::processing::DestroyExecutionReceipt>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() { return Ok(None); }
    game.clear_pending_decision_controllers();
    if can_be_regenerated {
        return crate::events::processing::process_destroy_scoped(game, object_id, source, ctx, None);
    }
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let suspended = game.effect_store.replacement_effects.suspend_regeneration_shields_from_source(object_id);
    let shield_count = game.regeneration_shield_count(object_id);
    game.clear_regeneration_shields(object_id);
    let result = crate::events::processing::process_destroy_scoped(game, object_id, source, ctx, None);
    if result.is_err() || ctx.decision_maker.awaiting_choice() {
        game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice()); context_checkpoint.restore(ctx); return result;
    }
    if game.object(object_id).is_some_and(|object| object.zone == Zone::Battlefield) {
        game.effect_store.replacement_effects.restore_suspended_effects(suspended);
        game.add_regeneration_shield(object_id, shield_count);
    }
    result
}

/// "Destroy target permanent" (optionally "It can't be regenerated").
///
/// A follow-up that asks what was "destroyed this way" (Noxious Gearhulk,
/// Dire-Strain Rampage) reads the destroyed permanent's last-known
/// information, so it is kept when the destruction happened (CR 608.2c).
pub(crate) fn execute_single_target_destroy(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    can_be_regenerated: bool,
) -> Result<EffectOutcome, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let mut receipts = Vec::new();
    let result = (|| -> Result<EffectOutcome, ExecutionError> {
    let mut destroyed_memory = None;
    let outcome = apply_single_target_object_from_spec(game, ctx, spec, |game, ctx, object_id| {
        let pre_memory = OutcomeObjectMemory::from_object_id(game, object_id);
        let status = DestroyEffect::destroy_object(game, ctx, object_id, can_be_regenerated, &mut receipts)?;
        if status.is_none() {
            destroyed_memory = receipts.last().and_then(|receipt| receipt.snapshot.as_ref()).map(OutcomeObjectMemory::from_snapshot).or(pre_memory);
        }
        Ok(status)
    })?;
    let original = match destroyed_memory {
        Some(memory) => outcome.with_affected_object_memory(vec![memory]),
        None => outcome,
    };
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    crate::events::processing::finish_destroy_receipts(game, ctx, original, receipts)
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() { game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice()); context_checkpoint.restore(ctx); }
    if result.is_ok() && ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    result
}

/// Destroy every selected permanent as one simultaneous event (CR 701.8a,
/// 603.2c, 603.10a): the destruction is staged together, the owners order
/// the cards going to their graveyards, every departure looks back at the
/// same trigger sources, and the deaths form one batch.
pub(crate) fn execute_simultaneous_destroy(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    spec: &ChooseSpec,
    can_be_regenerated: bool,
) -> Result<EffectOutcome, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    game.clear_pending_decision_controllers();
    let checkpoint = game.clone();
    let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
    let result = (|| -> Result<EffectOutcome, ExecutionError> {
    let mut receipts = Vec::new();
    let selected_objects = match resolve_objects_for_effect(game, ctx, spec) {
        Ok(objects) => objects,
        Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
        Err(error) => return Err(error),
    };
    if ctx.decision_maker.awaiting_choice() {
        return Ok(EffectOutcome::count(0));
    }

    // Stage the entire simultaneous destruction transaction on a clone.
    // Owner order choices see `decision_view`, the immutable pre-event
    // state, and the clone is committed only after every choice succeeds.
    let decision_view = game.clone();
    let mut staged_game = decision_view.clone();
    let pinned_lookback =
        crate::effects::helpers::begin_simultaneous_zone_change_lookback(&mut staged_game);
    let opened_batch = staged_game.open_simultaneous_action();
    let batch = staged_game
        .simultaneous_action_batch()
        .unwrap_or(ctx.provenance);
    let pending_start = staged_game.effect_store.pending_trigger_events.len();
    let mut destroyed_objects = Vec::new();
    let mut destroyed_memory = Vec::new();
    let mut graveyard_zone_changes = Vec::new();
    let mut departed_snapshots = Vec::new();
    let mut applied_count = 0usize;
    for object_id in selected_objects {
        let pre_snapshot = decision_view.object(object_id).map(|object| {
            ObjectSnapshot::from_object_with_calculated_characteristics(object, &decision_view)
        });
        let result = process_destroy_with_regeneration(
            &mut staged_game,
            object_id,
            Some(ctx.source),
            ctx,
            can_be_regenerated,
        )?;
        if ctx.decision_maker.awaiting_choice() {
            game.retain_pending_decision_controllers_from(&mut staged_game);
            return Ok(EffectOutcome::count(0));
        }
        if let Some(snapshot) = pre_snapshot.as_ref()
            && !staged_game
                .object(object_id)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        {
            departed_snapshots.push(snapshot.clone());
        }
        let Some(receipt) = result else { return Ok(EffectOutcome::count(0)); };
        let result = receipt.result.clone();
        let object_id = receipt.permanent;
        let pre_snapshot = receipt.snapshot.clone().or(pre_snapshot);
        receipts.push(receipt);
        if matches!(result, EventOutcome::Proceed(Zone::Graveyard)) {
            applied_count += 1;
            if let Some(snapshot) = pre_snapshot.as_ref() {
                destroyed_memory.push(OutcomeObjectMemory::from_snapshot(snapshot));
            }
            let result_objects = staged_game.take_zone_change_results(object_id);
            if let Some(snapshot) = pre_snapshot {
                graveyard_zone_changes.push((object_id, result_objects.clone(), snapshot));
            }
            destroyed_objects.extend(result_objects);
        }
    }

    if !super::order_simultaneous_graveyard_batch(
        &decision_view,
        &mut staged_game,
        &mut *ctx.decision_maker,
        Some(ctx.source),
        &destroyed_objects,
    ) {
        if ctx.decision_maker.awaiting_choice() {
            // This prompt is asked against the immutable pre-event view.
            let mut pending_view = decision_view.clone();
            pending_view.capture_pending_decision_controllers();
            game.retain_pending_decision_controllers_from(&mut pending_view);
        }
        return Ok(EffectOutcome::count(0));
    }
    staged_game.close_simultaneous_action(opened_batch);
    crate::effects::helpers::end_simultaneous_zone_change_lookback(&mut staged_game, pinned_lookback);

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
            staged_game.remove_pending_trigger_events_matching_from(pending_start, |event| {
                let Some(zone_change) = event.downcast::<ZoneChangeEvent>() else {
                    return false;
                };
                zone_change.from == Zone::Battlefield
                    && zone_change.to == Zone::Graveyard
                    && zone_change.objects.len() == 1
                    && event_objects.contains(&zone_change.objects[0])
            });

        if !removed.is_empty() {
            let mut lookback_source_snapshots = Vec::new();
            for snapshot in removed
                .iter()
                .flat_map(|event| event.lookback_source_snapshots())
            {
                if !lookback_source_snapshots
                    .iter()
                    .any(|existing: &ObjectSnapshot| existing.stable_id == snapshot.stable_id)
                {
                    lookback_source_snapshots.push(snapshot.clone());
                }
            }
            let mut event = ZoneChangeEvent::batch_with_snapshots(
                event_objects,
                Zone::Battlefield,
                Zone::Graveyard,
                ctx.cause.clone(),
                snapshots,
            );
            event.result_objects = result_objects;
            // The destruction's other zone changes (a replacement's "exile
            // it instead") belong to the same simultaneous event.
            staged_game.queue_trigger_event(
                ctx.provenance,
                TriggerEvent::new_with_provenance(event, ctx.provenance)
                    .with_simultaneous_batch(batch)
                    .with_lookback_source_snapshots(lookback_source_snapshots),
            );
        }
    }

    *game = staged_game;
    for snapshot in departed_snapshots {
        ctx.refresh_target_snapshot(snapshot.clone());
        if snapshot.object_id == ctx.source {
            ctx.refresh_source_snapshot(snapshot);
        }
    }

    let mut outcome = EffectOutcome::count(applied_count as i32);
    if !destroyed_objects.is_empty() {
        outcome = outcome.with_execution_fact(ExecutionFact::AffectedObjects(destroyed_objects));
        outcome = outcome.with_affected_object_memory(destroyed_memory);
    }

    crate::events::processing::finish_destroy_receipts(game, ctx, outcome, receipts)
    })();
    if result.is_err() || ctx.decision_maker.awaiting_choice() { game.restore_execution_checkpoint(checkpoint, result.is_ok() && ctx.decision_maker.awaiting_choice()); context_checkpoint.restore(ctx); }
    if result.is_ok() && ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
    result
}

impl EffectExecutor for DestroyEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Handle targeted effects with special single-target behavior
        if self.spec.is_target() && self.spec.is_single() {
            return execute_single_target_destroy(game, ctx, &self.spec, true);
        }
        execute_simultaneous_destroy(game, ctx, &self.spec, true)
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
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::ColorSet;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, ResolvedTarget};
    use crate::filter::ObjectRef;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::CounterType;
    use crate::static_abilities::StaticAbility;
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

    fn rayami_definition(id: u32) -> crate::cards::CardDefinition {
        crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(id),
            "Rayami, First of the Fallen",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 4))
        .with_ability(Ability::static_ability(
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                ObjectFilter::creature().nontoken(),
                None,
                vec![(CounterType::Blood, 1)],
                Vec::new(),
            ),
        ))
        .build()
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

    fn create_zombie_token() -> crate::cards::CardDefinition {
        crate::cards::CardDefinition::new(
            CardBuilder::new(CardId::new(), "Zombie")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Zombie])
                .color_indicator(ColorSet::BLACK)
                .power_toughness(PowerToughness::fixed(2, 2))
                .token()
                .build(),
        )
    }

    #[test]
    fn destroy_replacement_exiles_with_source_link_and_runs_followup_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let replacement_source = crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(50_200),
            "Kalitas Replacement",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 4))
        .with_ability(Ability::static_ability(
            StaticAbility::exile_would_die_instead_with_damage_source_and_follow_up(
                ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::Opponent),
                None,
                vec![Effect::create_tokens(create_zombie_token(), 1)],
            ),
        ))
        .build();
        let source =
            game.create_object_from_definition(&replacement_source, alice, Zone::Battlefield);
        let victim = create_creature(&mut game, bob, "Opponent Target", 50_201);
        let victim_stable_id = game.object(victim).expect("victim").stable_id;

        game.update_replacement_effects();
        let mut dm = SelectFirstDecisionMaker;
        let outcome = process_destroy(&mut game, victim, Some(source), &mut dm).expect("destruction succeeds").expect("destruction is not pending");

        assert!(
            matches!(outcome, EventOutcome::Replaced),
            "expected replacement outcome, got {outcome:?}"
        );
        let exiled_victim = game
            .find_object_by_stable_id(victim_stable_id)
            .expect("exiled victim should still be findable");
        assert_eq!(
            game.object(exiled_victim).expect("exiled victim").zone,
            Zone::Exile
        );
        assert!(
            game.get_exiled_with_source_links(source)
                .contains(&exiled_victim),
            "replacement should link the exiled card to its source"
        );

        let zombie_count = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .is_some_and(|obj| obj.name == "Zombie" && game.controller_of(obj) == alice)
            })
            .count();
        assert_eq!(zombie_count, 1);
    }

    #[test]
    fn rayami_replacement_exiles_nontoken_creature_with_blood_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let rayami = rayami_definition(50_210);
        let source = game.create_object_from_definition(&rayami, alice, Zone::Battlefield);
        let victim = create_creature(&mut game, bob, "Rayami Victim", 50_211);
        let victim_stable_id = game
            .object(victim)
            .expect("victim before destroy")
            .stable_id;

        game.update_replacement_effects();
        let mut dm = SelectFirstDecisionMaker;
        let outcome = process_destroy(&mut game, victim, Some(source), &mut dm).expect("destruction succeeds").expect("destruction is not pending");

        assert!(
            matches!(outcome, EventOutcome::Replaced),
            "expected replacement outcome, got {outcome:?}"
        );
        let exiled_victim = game
            .find_object_by_stable_id(victim_stable_id)
            .expect("exiled victim should still be findable");
        assert_eq!(
            game.object(exiled_victim).expect("exiled victim").zone,
            Zone::Exile
        );
        assert_eq!(
            game.counter_count(exiled_victim, CounterType::Blood),
            1,
            "exiled creature should have one blood counter"
        );
    }

    #[test]
    fn rayami_replacement_does_not_exile_noncreature_permanent() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let rayami = rayami_definition(50_220);
        let source = game.create_object_from_definition(&rayami, alice, Zone::Battlefield);
        let noncreature = CardBuilder::new(CardId::from_raw(50_221), "Rayami Noncreature")
            .card_types(vec![CardType::Artifact])
            .build();
        let noncreature_victim = game.create_object_from_card(&noncreature, bob, Zone::Battlefield);

        game.update_replacement_effects();
        let mut dm = SelectFirstDecisionMaker;
        let outcome = process_destroy(&mut game, noncreature_victim, Some(source), &mut dm).expect("destruction succeeds").expect("destruction is not pending");

        assert!(
            !matches!(outcome, EventOutcome::Replaced),
            "noncreature death should not be replaced by Rayami"
        );
        assert!(
            game.object(noncreature_victim)
                .is_none_or(|obj| obj.zone != Zone::Exile),
            "noncreature permanent should not be exiled by Rayami replacement"
        );
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

    #[derive(Default)]
    struct U010OrderDecisionMaker {
        calls: Vec<PlayerId>,
        pre_event_objects: Vec<ObjectId>,
        every_prompt_saw_pre_event_state: bool,
        defer: bool,
        awaiting: bool,
        malformed_order: bool,
    }

    impl crate::decision::DecisionMaker for U010OrderDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.awaiting
        }

        fn decide_order(
            &mut self,
            game: &GameState,
            ctx: &crate::decisions::context::OrderContext,
        ) -> Vec<ObjectId> {
            self.calls.push(ctx.player);
            self.every_prompt_saw_pre_event_state |= !self.pre_event_objects.is_empty();
            self.every_prompt_saw_pre_event_state &=
                self.pre_event_objects.iter().all(|object_id| {
                    game.object(*object_id)
                        .is_some_and(|object| object.zone == Zone::Battlefield)
                });
            if self.defer {
                self.awaiting = true;
                return Vec::new();
            }
            if self.malformed_order {
                let last = ctx.items.last().map(|(object_id, _)| *object_id);
                return last
                    .into_iter()
                    .chain([ObjectId::from_raw(9_999_999)])
                    .chain(last)
                    .collect();
            }
            ctx.items
                .iter()
                .rev()
                .map(|(object_id, _)| *object_id)
                .collect()
        }
    }

    fn graveyard_names(game: &GameState, player: PlayerId) -> Vec<String> {
        game.player(player)
            .expect("player")
            .graveyard
            .iter()
            .filter_map(|object_id| game.object(*object_id))
            .map(|object| object.name.to_string())
            .collect()
    }

    #[test]
    fn u010_owner_orders_only_cards_legally_reaching_the_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature(&mut game, alice, "First", 60_001);
        let second = create_creature(&mut game, alice, "Second", 60_002);
        let indestructible_definition =
            crate::cards::CardDefinitionBuilder::new(CardId::from_raw(60_003), "Indestructible")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(2, 2))
                .with_ability(Ability::static_ability(StaticAbility::indestructible()))
                .build();
        let indestructible = game.create_object_from_definition(
            &indestructible_definition,
            alice,
            Zone::Battlefield,
        );

        let mut decisions = U010OrderDecisionMaker {
            pre_event_objects: vec![first, second, indestructible],
            every_prompt_saw_pre_event_state: true,
            malformed_order: true,
            ..U010OrderDecisionMaker::default()
        };
        let source = game.new_object_id();
        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
            DestroyEffect::all(ObjectFilter::creature())
                .execute(&mut game, &mut ctx)
                .expect("mass destruction should resolve")
        };

        assert_eq!(outcome.as_count(), Some(2));
        assert_eq!(decisions.calls, vec![alice]);
        assert!(decisions.every_prompt_saw_pre_event_state);
        assert_eq!(graveyard_names(&game, alice), vec!["Second", "First"]);
        assert!(game.object(indestructible).is_some_and(|object| {
            object.zone == Zone::Battlefield && object.name == "Indestructible"
        }));
    }

    #[test]
    fn u010_multiplayer_owner_choices_are_apnap_and_commit_simultaneously() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        game.turn_store.turn_order = vec![alice, bob, cara];
        game.turn.active_player = cara;

        let objects = vec![
            create_creature(&mut game, alice, "Alice One", 60_011),
            create_creature(&mut game, alice, "Alice Two", 60_012),
            create_creature(&mut game, bob, "Bob One", 60_013),
            create_creature(&mut game, bob, "Bob Two", 60_014),
            create_creature(&mut game, cara, "Cara One", 60_015),
            create_creature(&mut game, cara, "Cara Two", 60_016),
        ];
        let mut decisions = U010OrderDecisionMaker {
            pre_event_objects: objects.clone(),
            every_prompt_saw_pre_event_state: true,
            ..U010OrderDecisionMaker::default()
        };
        let source = game.new_object_id();
        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
            DestroyEffect::all(ObjectFilter::creature())
                .execute(&mut game, &mut ctx)
                .expect("multiplayer mass destruction should resolve")
        };

        assert_eq!(outcome.as_count(), Some(6));
        assert_eq!(decisions.calls, vec![cara, alice, bob]);
        assert!(decisions.every_prompt_saw_pre_event_state);
        assert!(
            objects
                .iter()
                .all(|object_id| game.object(*object_id).is_none())
        );
        assert_eq!(
            graveyard_names(&game, alice),
            vec!["Alice Two", "Alice One"]
        );
        assert_eq!(graveyard_names(&game, bob), vec!["Bob Two", "Bob One"]);
        assert_eq!(graveyard_names(&game, cara), vec!["Cara Two", "Cara One"]);
    }

    #[test]
    fn u010_deferred_owner_order_cancels_the_whole_batch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature(&mut game, alice, "First", 60_021);
        let second = create_creature(&mut game, alice, "Second", 60_022);
        let pending_before = game.effect_store.pending_trigger_events.len();
        let mut decisions = U010OrderDecisionMaker {
            pre_event_objects: vec![first, second],
            every_prompt_saw_pre_event_state: true,
            defer: true,
            ..U010OrderDecisionMaker::default()
        };
        let source = game.new_object_id();
        let outcome = {
            let mut ctx = ExecutionContext::new(source, alice, &mut decisions);
            DestroyEffect::all(ObjectFilter::creature())
                .execute(&mut game, &mut ctx)
                .expect("deferred order should yield cleanly")
        };

        assert_eq!(outcome.as_count(), Some(0));
        assert_eq!(decisions.calls, vec![alice]);
        assert!(decisions.every_prompt_saw_pre_event_state);
        assert!(game.player(alice).expect("alice").graveyard.is_empty());
        assert!(
            game.object(first)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        );
        assert!(
            game.object(second)
                .is_some_and(|object| object.zone == Zone::Battlefield)
        );
        assert_eq!(
            game.effect_store.pending_trigger_events.len(),
            pending_before
        );
    }

    #[test]
    fn u010_batch_event_keeps_simultaneous_provenance_and_lki() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature(&mut game, alice, "First LKI", 60_031);
        let second = create_creature(&mut game, alice, "Second LKI", 60_032);
        let source = game.new_object_id();
        let mut decisions = U010OrderDecisionMaker::default();
        let provenance = game.provenance_graph_mut().alloc_root(
            crate::provenance::ProvenanceNodeKind::EffectExecution {
                source,
                controller: alice,
            },
        );
        {
            let mut ctx =
                ExecutionContext::new(source, alice, &mut decisions).with_provenance(provenance);
            DestroyEffect::all(ObjectFilter::creature())
                .execute(&mut game, &mut ctx)
                .expect("mass destruction should resolve");
        }

        let events = game.take_pending_trigger_events();
        let batch_events = events
            .iter()
            .filter_map(|event| {
                event
                    .downcast::<ZoneChangeEvent>()
                    .filter(|change| {
                        change.from == Zone::Battlefield && change.to == Zone::Graveyard
                    })
                    .map(|change| (event, change))
            })
            .collect::<Vec<_>>();
        assert_eq!(batch_events.len(), 1);
        let (raw, change) = batch_events[0];
        assert_eq!(change.objects, vec![first, second]);
        assert_eq!(change.result_objects.len(), 2);
        assert_eq!(change.snapshots().len(), 2);
        assert_eq!(
            change
                .snapshots()
                .iter()
                .map(|snapshot| snapshot.name.as_str())
                .collect::<Vec<_>>(),
            vec!["First LKI", "Second LKI"]
        );
        assert_eq!(change.cause.source, Some(source));
        assert!(game.provenance_graph().node(raw.provenance()).is_some());
        // The destruction is its own simultaneous action, not the whole
        // resolution's (a later instruction is a separate event).
        assert!(raw.simultaneous_batch().is_some());
    }
}


#[cfg(test)]
mod deferred_destroy_owner_contract_tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    struct Answers { pending: bool, pause: bool, questions: usize, originals: Vec<ObjectId>, instead: bool }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.questions += 1;
            if !self.instead { assert!(self.originals.iter().all(|id| game.object(*id).is_none()), "the original destruction batch completes before additions"); }
            self.pending = self.pause; !self.pause
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn perform(root: bool, game: &mut GameState, ctx: &mut ExecutionContext, originals: &[ObjectId]) -> Result<EffectOutcome, ExecutionError> {
        if root {
            // The pre-fix root API is scalar and the proposed root is fallible;
            // common gameplay assertions below verify neither can publish a
            // partially committed replacement or lose its observations.
            let _result = crate::events::processing::process_destroy(game, originals[0], Some(ctx.source), ctx.decision_maker);
            Ok(EffectOutcome::resolved())
        } else {
            let filter = ObjectFilter { any_of: originals.iter().copied().map(ObjectFilter::specific).collect(), ..ObjectFilter::default() };
            DestroyEffect::all(filter).execute(game, ctx)
        }
    }
    fn check(kind: u8, mode: u8) {
        let root = kind == 2; let instead = kind == 1;
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(CardId::new(), "Destroy original").card_types(vec![crate::types::CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2,2)).build();
        let originals = (0..if root { 1 } else { 2 }).map(|_| game.create_object_from_card(&card, alice, Zone::Battlefield)).collect::<Vec<_>>();
        let tracked = game.object(originals[0]).unwrap().stable_id;
        let parent_card = crate::card::CardBuilder::new(CardId::new(), "Destroy parent").card_types(vec![crate::types::CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1,1)).build();
        let parent = game.create_object_from_card(&parent_card, alice, Zone::Battlefield);
        let replacement_card = crate::card::CardBuilder::new(CardId::new(), "Destroy replacement").card_types(vec![crate::types::CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(4,4)).build();
        let replacement_source = game.create_object_from_card(&replacement_card, bob, Zone::Battlefield);
        let effects = if mode == 3 { vec![Effect::new(crate::effects::PutCountersEffect::new(crate::object::CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it")))] }
            else if mode == 1 { vec![Effect::gain_life(3), Effect::lose_life(Value::X)] }
            else { let mut effects = vec![Effect::gain_life(if instead { Value::SourcePower } else { Value::Fixed(3) }), Effect::may(vec![Effect::gain_life(4)])]; if mode == 2 { effects.push(Effect::gain_life(5)); } effects };
        let action = if instead { ReplacementAction::Instead(effects) } else { ReplacementAction::Additionally(effects) };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(replacement_source, bob,
            crate::events::permanents::matchers::WouldBeDestroyedMatcher::new(ObjectFilter::specific(originals[0])), action));
        game.take_pending_trigger_events(); let before_ids = game.next_object_id_counter();
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let mut dm = Answers { pending: false, pause: mode == 2, questions: 0, originals: originals.clone(), instead };
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm); ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let result = perform(root, &mut game, &mut ctx, &originals);
        if mode == 1 && !root { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))), "destruction cannot suppress replacement errors"); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); }
        else if mode != 1 {
            let outcome = result.unwrap();
            if !root { assert_eq!(outcome.count_or_zero(), if instead {1} else {2}); }
            if mode == 3 {
                let object = game.find_object_by_stable_id(tracked).unwrap();
                assert_eq!(game.object(object).unwrap().counters.get(&crate::object::CounterType::PlusOnePlusOne), Some(&1));
                assert!(!game.object(parent).unwrap().counters.contains_key(&crate::object::CounterType::PlusOnePlusOne));
                if !root {
                    // The counter payload's exact object/zone remains an observation.
                    assert!(outcome.execution_facts.iter().any(|fact| matches!(fact,
                        ExecutionFact::AffectedObjectMemory(memories) if memories.iter().any(|m|
                            m.object_id == object && m.zone == if instead {Zone::Battlefield} else {Zone::Graveyard}))));
                    // "Destroyed this way" reads only original destruction LKI.
                    let destroyed = outcome.affected_object_memory().unwrap();
                    assert_eq!(destroyed.len(), if instead { 1 } else { 2 });
                    assert!(destroyed.iter().all(|memory| memory.zone == Zone::Battlefield
                        && originals.contains(&memory.object_id)));
                    assert_eq!(destroyed.iter().any(|memory| memory.object_id == originals[0]), !instead);
                }
            } else {
                assert_eq!(game.player(bob).unwrap().life, if instead {28} else {27}, "replacement uses its own source/controller");
                let events = if root { game.take_pending_trigger_events() } else { outcome.events };
                assert_eq!(events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).map(|e| (e.player,e.amount)).collect::<Vec<_>>(), vec![(bob,if instead {4} else {3}),(bob,4)]);
            }
        }
        assert_eq!(ctx.source,parent); assert_eq!(ctx.controller,alice); assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id,parent); drop(ctx);
        if mode == 1 || mode == 2 {
            assert!(originals.iter().all(|id| game.object(*id).is_some_and(|o| o.zone == Zone::Battlefield)));
            assert!(game.player(alice).unwrap().graveyard.is_empty()); assert_eq!(game.player(bob).unwrap().life,20); assert_eq!(game.next_object_id_counter(),before_ids);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        } else { assert!(game.effect_store.replacement_effects.get_effect(shield).is_none()); }
        if mode == 2 {
            assert_eq!(dm.questions,1); let mut dm = Answers { pending:false,pause:false,questions:0,originals:originals.clone(),instead };
            let mut ctx = ExecutionContext::new(parent,alice,&mut dm); let outcome = perform(root,&mut game,&mut ctx,&originals).unwrap(); assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx);
            assert_eq!(dm.questions,1); if !root { assert_eq!(outcome.count_or_zero(),if instead {1} else {2}); }
            assert_eq!(game.player(bob).unwrap().life,if instead {33} else {32}); assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            let events = if root {game.take_pending_trigger_events()} else {outcome.events};
            assert_eq!(events.iter().filter_map(|e| e.downcast::<crate::events::LifeGainEvent>()).map(|e| e.amount).collect::<Vec<_>>(),vec![if instead {4} else {3},4,5]);
        }
    }
    #[test] fn additional_destroy_batch_success() { check(0,0); }
    #[test] fn additional_destroy_batch_error() { check(0,1); }
    #[test] fn additional_destroy_batch_pending_replay() { check(0,2); }
    #[test] fn additional_destroy_batch_object_binding() { check(0,3); }
    #[test] fn instead_destroy_batch_success() { check(1,0); }
    #[test] fn instead_destroy_batch_error() { check(1,1); }
    #[test] fn instead_destroy_batch_pending_replay() { check(1,2); }
    #[test] fn instead_destroy_batch_object_binding() { check(1,3); }
    #[test] fn additional_destroy_root_success() { check(2,0); }
    #[test] fn additional_destroy_root_error() { check(2,1); }
    #[test] fn additional_destroy_root_pending_replay() { check(2,2); }
    #[test] fn additional_destroy_root_object_binding() { check(2,3); }
}

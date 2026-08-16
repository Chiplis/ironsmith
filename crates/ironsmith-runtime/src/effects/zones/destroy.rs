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
                Self::destroy_object,
            );
        }

        let selected_objects = match resolve_objects_for_effect(game, ctx, &self.spec) {
            Ok(objects) => objects,
            Err(_) => return Ok(EffectOutcome::target_invalid()),
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        // Stage the entire simultaneous destruction transaction on a clone.
        // Owner order choices see `decision_view`, the immutable pre-event
        // state, and the clone is committed only after every choice succeeds.
        let decision_view = game.clone();
        let mut staged_game = decision_view.clone();
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
            let result = process_destroy(
                &mut staged_game,
                object_id,
                Some(ctx.source),
                &mut *ctx.decision_maker,
            );
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if let Some(snapshot) = pre_snapshot.as_ref()
                && !staged_game
                    .object(object_id)
                    .is_some_and(|object| object.zone == Zone::Battlefield)
            {
                departed_snapshots.push(snapshot.clone());
            }
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
            return Ok(EffectOutcome::count(0));
        }

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
                staged_game.queue_trigger_event(
                    ctx.provenance,
                    TriggerEvent::new_with_provenance(event, ctx.provenance)
                        .with_simultaneous_batch(ctx.provenance)
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
        let outcome = process_destroy(&mut game, victim, Some(source), &mut dm);

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

        let rayami = crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(50_210),
            "Rayami, First of the Fallen",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 4))
        .parse_text(
            "If a nontoken creature would die, exile that card with a blood counter on it instead.",
        )
        .expect("rayami replacement clause should parse");
        let source = game.create_object_from_definition(&rayami, alice, Zone::Battlefield);
        let victim = create_creature(&mut game, bob, "Rayami Victim", 50_211);
        let victim_stable_id = game
            .object(victim)
            .expect("victim before destroy")
            .stable_id;

        game.update_replacement_effects();
        let mut dm = SelectFirstDecisionMaker;
        let outcome = process_destroy(&mut game, victim, Some(source), &mut dm);

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

        let rayami = crate::cards::CardDefinitionBuilder::new(
            CardId::from_raw(50_220),
            "Rayami, First of the Fallen",
        )
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(5, 4))
        .parse_text(
            "If a nontoken creature would die, exile that card with a blood counter on it instead.",
        )
        .expect("rayami replacement clause should parse");
        let source = game.create_object_from_definition(&rayami, alice, Zone::Battlefield);
        let noncreature = CardBuilder::new(CardId::from_raw(50_221), "Rayami Noncreature")
            .card_types(vec![CardType::Artifact])
            .build();
        let noncreature_victim = game.create_object_from_card(&noncreature, bob, Zone::Battlefield);

        game.update_replacement_effects();
        let mut dm = SelectFirstDecisionMaker;
        let outcome = process_destroy(&mut game, noncreature_victim, Some(source), &mut dm);

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
        assert_eq!(raw.simultaneous_batch(), Some(provenance));
    }
}

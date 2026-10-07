//! Full frozen Avatar Destiny body. Authored source evidence; execution deferred.
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::SelectObjectsContext;
use ironsmith::effects::{AttachToEffect, DestroyEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{check_and_apply_sbas, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::object::AttachmentTarget;
use ironsmith::target::ChooseSpec;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, ObjectId, PlayerId, Subtype, Zone};
use ironsmith::ids::StableId;
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
fn definitions() -> [CardDefinition; 2] {
    let row: serde_json::Value = serde_json::from_str(include_str!("../../../fixtures/avatar_destiny_body.json.fixture")).unwrap();
    let name = row["name"].as_str().unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(),
        row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    let (direct, loss) = parse_loss::capture(|| compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (artifact, loss) = parse_loss::capture(|| compile_to_artifact(name, &text, false));
    let (artifact, _) = artifact.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, artifact);
    let definitions = [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap()];
    for definition in &definitions { assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition)); }
    definitions
}
fn simple(name: &str, creature: bool) -> CardDefinition {
    compile_to_runtime_definition(name, if creature { "Type: Creature — Elf\nPower/Toughness: 2/3" }
        else { "Type: Land" }, false).unwrap()
}
fn card(game: &mut GameState, name: &str, owner: PlayerId, zone: Zone, creature: bool) -> ObjectId {
    game.create_object_from_definition(&simple(name, creature), owner, zone)
}
struct MillChoice { pick: bool, expected: StableId, unrelated: Vec<StableId>, saw_choice: bool }
impl DecisionMaker for MillChoice {
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        let expected = context.candidates.iter().find(|candidate| candidate.legal
            && game.object(candidate.id).is_some_and(|object| object.stable_id == self.expected));
        if let Some(expected) = expected {
            self.saw_choice = true;
            for candidate in context.candidates.iter().filter(|candidate| candidate.legal) {
                assert!(!self.unrelated.contains(&game.object(candidate.id).unwrap().stable_id),
                    "only this mill's actual creature results may be selected");
            }
            if self.pick { vec![expected.id] } else { vec![] }
        } else { SelectFirstDecisionMaker.decide_objects(game, context) }
    }
}

#[test]
fn avatar_death_uses_host_lki_and_exact_mill_results_after_the_auras_owner_return() {
    for definition in definitions() { for pick in [false, true] { for move_aura_again in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = card(&mut game, "Original host", A, Zone::Battlefield, true);
        let old_a = card(&mut game, "Old graveyard creature A", A, Zone::Graveyard, true);
        let old_b = card(&mut game, "Old graveyard creature B", A, Zone::Graveyard, true);
        for _ in 0..5 { card(&mut game, "Opponent graveyard", B, Zone::Graveyard, true); }
        // The Aura is owned by Bob but controlled by Alice: its modifier,
        // mill and reanimation use Alice; its return-to-hand uses its owner.
        let aura = game.create_object_from_definition(&definition, B, Zone::Battlefield);
        game.set_current_controller(aura, A).unwrap();
        AttachToEffect::new(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.object(aura).unwrap().attached_to, Some(AttachmentTarget::Object(host)));
        assert_eq!(game.current_power(host), Some(4));
        assert_eq!(game.current_toughness(host), Some(5));
        assert!(game.current_has_subtype(host, Subtype::Avatar));
        assert!(game.current_has_subtype(host, Subtype::Elf));
        let aura_stable = game.object(aura).unwrap().stable_id;
        let host_stable = game.object(host).unwrap().stable_id;
        let old_stables = [old_a, old_b, host].into_iter().map(|id| game.object(id).unwrap().stable_id).collect::<Vec<_>>();
        for _ in 0..2 { card(&mut game, "Unmilled bottom", A, Zone::Library, false); }
        let choice = card(&mut game, "Milled choice", A, Zone::Library, true);
        let choice_stable = game.object(choice).unwrap().stable_id;
        card(&mut game, "Milled land one", A, Zone::Library, false);
        card(&mut game, "Milled land two", A, Zone::Library, false);
        card(&mut game, "Other milled creature", A, Zone::Library, true);
        game.take_pending_trigger_events();
        let outcome = DestroyEffect::with_spec(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        let mut queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), 1, "the full death trigger must be present");
        assert!(game.stack.last().unwrap().targets.is_empty(), "milled cards are chosen on resolution, not targeted on announcement");
        let aura_grave = game.find_object_by_stable_id(aura_stable).unwrap();
        assert_eq!(game.object(aura_grave).unwrap().zone, Zone::Graveyard);
        if move_aura_again {
            let exile = game.move_object_by_effect(aura_grave, Zone::Exile).unwrap();
            game.move_object_by_effect(exile, Zone::Graveyard).unwrap();
        }
        // Changing the dying host's current incarnation cannot change the
        // four-power death snapshot used to determine this mill's quantity.
        let dead_host = game.find_object_by_stable_id(host_stable).unwrap();
        let returned_host = game.move_object_by_effect(dead_host, Zone::Battlefield).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(returned_host), Some(2));
        let mut dm = MillChoice { pick, expected: choice_stable, unrelated: old_stables, saw_choice: false };
        while !game.stack_is_empty() { resolve_stack_entry_with(&mut game, &mut dm).unwrap(); }
        assert!(dm.saw_choice);
        assert_eq!(game.player(A).unwrap().library.len(), 2, "use the dying creature's four-power LKI");
        let aura_now = game.find_object_by_stable_id(aura_stable).unwrap();
        assert_eq!(game.object(aura_now).unwrap().zone, if move_aura_again { Zone::Graveyard } else { Zone::Hand });
        if !move_aura_again { assert!(game.player(B).unwrap().hand.contains(&aura_now)); }
        let creature_now = game.find_object_by_stable_id(choice_stable).unwrap();
        assert_eq!(game.object(creature_now).unwrap().zone, if pick { Zone::Battlefield } else { Zone::Graveyard });
        if pick { assert_eq!(game.current_controller(creature_now), Some(A)); }
        assert!(game.player(A).unwrap().graveyard.contains(&old_a));
        assert!(game.player(A).unwrap().graveyard.contains(&old_b));
    } } }
}

#[test]
fn avatar_enchant_scope_and_live_modifier_disappear_when_unattached() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let own = card(&mut game, "Own creature", A, Zone::Battlefield, true);
        let other = card(&mut game, "Opponent creature", B, Zone::Battlefield, true);
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        AttachToEffect::new(ChooseSpec::SpecificObject(other))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        assert!(game.object(aura).unwrap().attached_to.is_none());
        AttachToEffect::new(ChooseSpec::SpecificObject(own))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        let grave = card(&mut game, "Counted creature", A, Zone::Graveyard, true);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(3));
        assert!(game.current_has_subtype(own, Subtype::Avatar));
        game.move_object_by_effect(grave, Zone::Exile).unwrap();
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(2));
        game.move_object_by_effect(aura, Zone::Graveyard).unwrap();
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_has_subtype(own, Subtype::Avatar));
        assert!(game.current_has_subtype(own, Subtype::Elf));
    }
}

#[test]
fn avatar_returns_itself_when_the_mill_contains_no_creature_cards() {
    for definition in definitions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = card(&mut game, "Host", A, Zone::Battlefield, true);
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let stable = game.object(aura).unwrap().stable_id;
        AttachToEffect::new(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        for _ in 0..3 { card(&mut game, "Milled land", A, Zone::Library, false); }
        game.take_pending_trigger_events();
        let outcome = DestroyEffect::with_spec(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        let mut queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), 1);
        while !game.stack_is_empty() { resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap(); }
        assert_eq!(game.player(A).unwrap().library.len(), 1);
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert!(game.player(A).unwrap().hand.contains(&returned));
        assert!(!game.battlefield.iter().any(|id| game.current_has_card_type(*id, ironsmith::CardType::Creature)));
    }
}

#[test]
fn missing_aura_departure_receipt_rolls_back_the_full_trigger_including_its_prior_mill() {
    use ironsmith::events::ZoneChangeEvent;
    use ironsmith::triggers::TriggerEvent;
    for definition in definitions() { for missing_kind in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = card(&mut game, "Host", A, Zone::Battlefield, true);
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let aura_stable = game.object(aura).unwrap().stable_id;
        AttachToEffect::new(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        for _ in 0..3 { card(&mut game, "Mill rollback witness", A, Zone::Library, false); }
        game.take_pending_trigger_events();
        let outcome = DestroyEffect::with_spec(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        let mut queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), 1);
        let aura_grave = game.find_object_by_stable_id(aura_stable).unwrap();
        assert_eq!(game.object(aura_grave).unwrap().zone, Zone::Graveyard);
        let unrelated = card(&mut game, "Wrong mapped identity", B, Zone::Battlefield, true);
        let mut removed = 0;
        let history = &mut game.turn_store.turn_history;
        for records in [&mut history.event_records, &mut history.staged_event_records] {
            let old = records.iter().cloned().collect::<Vec<_>>();
            records.clear();
            for mut record in old {
                if let Some(change) = record.event.downcast::<ZoneChangeEvent>()
                    && change.from == Zone::Battlefield && change.objects.contains(&aura)
                {
                    removed += 1;
                    if missing_kind == 0 { continue; }
                    let mut missing = change.clone();
                    missing.result_objects = if missing_kind == 1 { Vec::new() } else { vec![unrelated] };
                    record.event = TriggerEvent::new_with_provenance(missing, record.event.provenance());
                }
                records.push(record);
            }
        }
        assert!(removed > 0, "remove real producer evidence, not a fabricated event");
        game.take_pending_trigger_events();
        let library = game.player(A).unwrap().library.clone();
        let graveyard = game.player(A).unwrap().graveyard.clone();
        let result = resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker);
        assert!(matches!(result, Err(ironsmith::game_loop::GameLoopError::ExecutionFailed(
            ironsmith::effects::ExecutionError::IncompleteEvidence(_)))), "{result:?}");
        assert_eq!(game.player(A).unwrap().library, library, "earlier mill must roll back");
        assert_eq!(game.player(A).unwrap().graveyard, graveyard);
        assert_eq!(game.stack.len(), 1, "the unresolved trigger remains available to retry");
        assert_eq!(game.find_object_by_stable_id(aura_stable), Some(aura_grave));
        assert!(game.take_pending_trigger_events().is_empty());
    } }
}


#[derive(Debug, Clone)]
struct ChangeMilledSuccessor { face_down: bool, move_again: bool }
impl EffectExecutor for ChangeMilledSuccessor {
    fn execute(&self, game: &mut GameState, ctx: &mut EffectContext) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        let ids = ironsmith::effects::helpers::resolve_objects_from_spec(game, &ChooseSpec::tagged("it"), ctx)?;
        assert_eq!(ids.len(), 1);
        let exact = ids[0];
        assert_eq!(game.object(exact).unwrap().zone, Zone::Exile);
        if self.face_down {
            assert!(game.set_face_down(exact));
            // CR 406.3a: public-zone membership is independent of the card's
            // absent characteristics, including when a player may look at it.
            game.grant_face_down_exile_view(exact, A);
            let chars = game.current_characteristics(exact).unwrap();
            assert!(chars.card_types.is_empty());
            assert!(chars.name.is_empty());
            assert!(chars.mana_cost.is_none());
        }
        if self.move_again {
            let hand = game.move_object_by_game_rule(exact, Zone::Hand).unwrap();
            let returned = game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
            assert_ne!(returned, exact);
        }
        Ok(ironsmith::effect::EffectOutcome::count(1))
    }
}

#[test]
fn avatar_mill_finds_original_public_results_but_not_hidden_qualities_or_later_incarnations() {
    use ironsmith::effect::Effect;
    use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
    use ironsmith::target::ObjectFilter;
    for definition in definitions() { for mode in 0..4 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let host = card(&mut game, "Host", A, Zone::Battlefield, true);
        let aura = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let aura_stable = game.object(aura).unwrap().stable_id;
        AttachToEffect::new(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        card(&mut game, "Unqualified land", A, Zone::Library, false);
        let original = card(&mut game, "Only milled creature", A, Zone::Library, true);
        let stable = game.object(original).unwrap().stable_id;
        game.take_pending_trigger_events();
        let outcome = DestroyEffect::with_spec(ChooseSpec::SpecificObject(host))
            .execute(&mut game, &mut EffectContext::new(aura, A, &mut SelectFirstDecisionMaker)).unwrap();
        for event in outcome.events { game.queue_trigger_event(Default::default(), event); }
        let mut queue = TriggerQueue::new();
        check_and_apply_sbas(&mut game, &mut queue).unwrap();
        put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), 1);
        let replacement = card(&mut game, "Mill replacement", B, Zone::Battlefield, false);
        if mode >= 2 {
            game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
                replacement, B, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(original), Some(Zone::Library), None),
                ReplacementAction::Additionally(vec![Effect::new(ChangeMilledSuccessor {
                    face_down: mode == 2, move_again: mode == 3,
                })]),
            ));
        }
        game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement, B, ironsmith::events::zones::matchers::WouldChangeZoneMatcher::new(
                ObjectFilter::specific(original), Some(Zone::Library), Some(Zone::Graveyard)),
            ReplacementAction::ChangeDestination(if mode == 1 { Zone::Hand } else { Zone::Exile }),
        ));
        let mut dm = MillChoice { pick: true, expected: stable, unrelated: vec![], saw_choice: false };
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.stack.is_empty());
        assert_eq!(dm.saw_choice, mode == 0, "mode {mode}: only the face-up original public successor is a creature card");
        let current = game.find_object_by_stable_id(stable).unwrap();
        assert_eq!(game.object(current).unwrap().zone,
            if mode == 0 { Zone::Battlefield } else if mode == 1 { Zone::Hand } else { Zone::Exile });
        assert_eq!(game.object(game.find_object_by_stable_id(aura_stable).unwrap()).unwrap().zone, Zone::Hand);
        assert!(game.player(A).unwrap().library.is_empty());
    } }
}

#[test]
fn an_unqualified_milled_card_reference_keeps_face_down_public_identity_and_explicit_zone_constraints() {
    use ironsmith::effect::Effect;
    use ironsmith::effects::{execute_effect, ExileInsteadOfGraveyardEffect};
    use ironsmith::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use ironsmith::tag::TagKey;
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let source = card(&mut game, "Source", A, Zone::Battlefield, false);
    card(&mut game, "Milled creature", A, Zone::Library, true);
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    execute_effect(&mut game, &Effect::new(ExileInsteadOfGraveyardEffect::you()), &mut ctx).unwrap();
    execute_effect(&mut game, &Effect::mill(1).tag("milled"), &mut ctx).unwrap();
    let exact = ctx.get_tagged_all("milled").unwrap()[0].object_id;
    assert!(game.set_face_down(exact));
    let mut filter = ObjectFilter {
        match_captured_public_destination: true,
        tagged_constraints: vec![TaggedObjectConstraint { tag: TagKey::from("milled"), relation: TaggedOpbjectRelation::IsTaggedObject }],
        ..Default::default()
    };
    let resolve = |game: &GameState, filter: &ObjectFilter, ctx: &EffectContext|
        ironsmith::effects::helpers::resolve_objects_from_spec(game, &ChooseSpec::All(filter.clone()), ctx).unwrap();
    assert_eq!(resolve(&game, &filter, &ctx), vec![exact], "a face-down exiled card is still in its original public destination");
    filter.card_types = vec![ironsmith::CardType::Creature];
    assert!(resolve(&game, &filter, &ctx).is_empty(), "printed creature characteristics cannot qualify it");
    filter.card_types.clear();
    filter.zone = Some(Zone::Graveyard);
    assert!(resolve(&game, &filter, &ctx).is_empty(), "an explicit graveyard clause remains restrictive");
    filter.zone = None;
    let hand = game.move_object_by_game_rule(exact, Zone::Hand).unwrap();
    game.move_object_by_game_rule(hand, Zone::Exile).unwrap();
    assert!(resolve(&game, &filter, &ctx).is_empty(), "a later incarnation cannot inherit the original mill identity");
}

#[test]
fn missing_public_mill_collection_is_incomplete_even_when_no_candidate_exists_or_count_is_negated() {
    use ironsmith::effect::{Effect, Value, ValueComparisonOperator};
    use ironsmith::effects::{execute_effect, SequenceEffect};
    use ironsmith::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    for known_empty in [false, true] { for query in 0..3 {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(ObjectId::new(), A, &mut dm);
        if known_empty { ctx.set_tagged_objects(ironsmith::tag::TagKey::from("milled"), vec![]); }
        let filter = ObjectFilter {
            match_captured_public_destination: true,
            tagged_constraints: vec![TaggedObjectConstraint { tag: "milled".into(), relation: TaggedOpbjectRelation::IsTaggedObject }],
            ..Default::default()
        };
        let followup = match query {
            0 => Effect::move_to_zone(ChooseSpec::All(filter), Zone::Battlefield, false),
            1 => Effect::gain_life(Value::Count(filter)),
            _ => Effect::conditional_only(ironsmith::ConditionExpr::Not(Box::new(
                ironsmith::ConditionExpr::ValueComparison {
                    left: Value::Count(filter), operator: ValueComparisonOperator::GreaterThan,
                    right: Value::Fixed(0),
                })), vec![Effect::gain_life(2)]),
        };
        let result = execute_effect(&mut game, &Effect::new(SequenceEffect::new(vec![Effect::gain_life(3), followup])), &mut ctx);
        if known_empty {
            assert!(result.is_ok(), "explicit empty collection is complete: {result:?}");
            assert_eq!(game.player(A).unwrap().life, if query == 2 { 25 } else { 23 });
        } else {
            assert!(matches!(result, Err(ironsmith::effects::ExecutionError::IncompleteEvidence(_))), "query {query}: {result:?}");
            assert_eq!(game.player(A).unwrap().life, 20, "the earlier instruction rolls back");
        }
    } }
}

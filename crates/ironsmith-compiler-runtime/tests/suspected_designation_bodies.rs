//! Frozen full-body source scenarios. UNRUN while campaign builds/tests are deferred.
use ironsmith::cards::CardDefinition;
use ironsmith::continuous::{EffectTarget, Modification};
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, SelectObjectsContext, SelectOptionsContext, TargetsContext};
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{ApplyContinuousEffect, EffectContext, EffectExecutor};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, drain_pending_trigger_events, put_triggers_on_stack_with_dm,
    resolve_stack_entry_with};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::object::{AttachmentTarget, CounterType};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
const C: PlayerId = PlayerId(2);
fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/suspected_designation_bodies.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let mut lines = vec![format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()), format!("Type: {}", row["type_line"].as_str().unwrap())];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) { lines.push(format!("Power/Toughness: {power}/{toughness}")); }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    let text = lines.join("\n");
    let (direct_result, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, text.clone(), false));
    let direct = direct_result.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());
    let (artifact_result, artifact_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_artifact(name, text, false));
    let (artifact, _) = artifact_result.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!artifact_loss.is_lossy(), "artifact {name}: {}", artifact_loss.reasons_text());
    artifact.validate().unwrap();
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [direct, ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap()]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.step = None; game.turn.active_player = A; game.turn.priority_player = Some(A);
    for symbol in [ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black, ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless] { game.player_mut(A).unwrap().mana_pool.add(symbol, 20); }
    game
}
fn creature(game: &mut GameState, owner: PlayerId, name: &str, zone: Zone) -> ObjectId {
    let definition = compile_to_runtime_definition(name, "Type: Creature — Soldier\nPower/Toughness: 3/4", false).unwrap();
    game.create_object_from_definition(&definition, owner, zone)
}
#[derive(Default)]
struct Choices {
    target: Option<ObjectId>, target_sequence: Vec<ObjectId>, target_index: usize, pick: Option<ObjectId>, modes: Option<Vec<usize>>, accept: bool,
    suspend_boolean: bool, pending: bool, boolean_players: Vec<PlayerId>, candidates: Vec<ObjectId>,
}
impl DecisionMaker for Choices {
    fn awaiting_choice(&self) -> bool { self.pending }
    fn decide_boolean(&mut self, _: &GameState, context: &BooleanContext) -> bool {
        self.boolean_players.push(context.player);
        self.pending = self.suspend_boolean;
        self.accept
    }
    fn decide_objects(&mut self, game: &GameState, context: &SelectObjectsContext) -> Vec<ObjectId> {
        self.candidates = context.candidates.iter().filter(|candidate| candidate.legal).map(|candidate| candidate.id).collect();
        if let Some(id) = self.pick { assert!(self.candidates.contains(&id)); vec![id] }
        else { SelectFirstDecisionMaker.decide_objects(game, context) }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if !self.target_sequence.is_empty() {
            return context.requirements.iter().map(|requirement| {
                let id = self.target_sequence[self.target_index]; self.target_index += 1;
                assert!(requirement.legal_targets.contains(&Target::Object(id)));
                Target::Object(id)
            }).collect();
        }
        if let Some(id) = self.target {
            assert!(context.requirements.iter().all(|requirement| requirement.legal_targets.contains(&Target::Object(id))));
            vec![Target::Object(id)]
        } else { SelectFirstDecisionMaker.decide_targets(game, context) }
    }
    fn decide_options(&mut self, game: &GameState, context: &SelectOptionsContext) -> Vec<usize> {
        if context.description.starts_with("Choose mode") && let Some(modes) = &self.modes { return modes.clone(); }
        SelectFirstDecisionMaker.decide_options(game, context)
    }
}
fn announce(game: &mut GameState, action: LegalAction, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..64 {
        if state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_mana_ability.is_none() { break; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("{progress:?}"); };
        state = state.clone();
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    assert!(state.pending_cast.is_none() && state.pending_activation.is_none() && state.pending_mana_ability.is_none());
}
fn cast(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) {
    let spell = game.create_object_from_definition(definition, A, Zone::Hand);
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::CastSpell { spell_id, .. } if *spell_id == spell)).unwrap();
    announce(game, action, dm);
}
fn drain(game: &mut GameState, dm: &mut Choices) {
    let mut queue = TriggerQueue::new(); drain_pending_trigger_events(game, &mut queue);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    while !game.stack.is_empty() { resolve_stack_entry_with(game, dm).unwrap(); drain_pending_trigger_events(game, &mut queue); put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap(); }
}
fn remove_abilities(game: &mut GameState, source: ObjectId, target: ObjectId) {
    ApplyContinuousEffect::new(EffectTarget::Specific(target), Modification::RemoveAllAbilities, Until::Forever)
        .execute(game, &mut EffectContext::new_default(source, A)).unwrap();
    game.refresh_continuous_state().unwrap();
}

#[test]
fn bounded_complete_bodies_keep_metadata_and_artifact_identity() {
    assert_eq!(rows().len(), 6);
    for name in ["Agency Coroner", "Airtight Alibi", "Deadly Complication", "Eliminate the Impossible"] {
        for definition in definitions(name) { assert_eq!(definition.card.name, name); }
    }
}

#[test]
fn coroner_native_paid_sacrifice_draws_one_or_two_from_retained_designation() {
    for definition in definitions("Agency Coroner") { for suspected in [false, true] { for return_paid_card in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let paid = creature(&mut game, A, "Paid creature", Zone::Battlefield);
        let paid_stable = game.object(paid).unwrap().stable_id;
        if suspected { game.set_suspected(paid); remove_abilities(&mut game, source, paid); }
        for n in 0..5 { creature(&mut game, A, &format!("Draw {n}"), Zone::Library); }
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        announce(&mut game, action, &mut Choices { pick: Some(paid), ..Default::default() });
        assert!(game.object(paid).is_none());
        if return_paid_card {
            let departed = game.find_object_by_stable_id(paid_stable).unwrap();
            let returned = game.move_object_by_game_rule(departed, Zone::Battlefield).unwrap();
            assert_ne!(returned, paid); assert!(!game.is_suspected(returned));
        }
        // Neither the source's existence nor a later incarnation owns the paid evidence.
        game.move_object_by_game_rule(source, Zone::Exile).unwrap();
        resolve_stack_entry_with(&mut game, &mut Choices::default()).unwrap();
        assert_eq!(game.player(A).unwrap().hand.len(), if suspected { 2 } else { 1 });
    } } }
}

#[test]
fn alibi_native_trigger_keeps_untap_hexproof_pump_and_exact_live_prohibition() {
    for definition in definitions("Airtight Alibi") {
        let mut game = game();
        let first = creature(&mut game, B, "First host", Zone::Battlefield);
        let second = creature(&mut game, C, "Second host", Zone::Battlefield);
        game.set_suspected(first); game.tap(first);
        let mut dm = Choices { target: Some(first), ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        let aura = game.battlefield.iter().copied().find(|id| game.object(*id).unwrap().name == "Airtight Alibi").unwrap();
        drain(&mut game, &mut dm);
        assert!(!game.is_tapped(first)); assert!(!game.is_suspected(first));
        assert!(game.current_has_static_ability_id(first, StaticAbilityId::Hexproof));
        assert_eq!(game.current_power(first), Some(5)); assert_eq!(game.current_toughness(first), Some(6));
        assert!(!game.set_suspected(first)); assert!(game.set_suspected(second)); game.clear_suspected(second);
        remove_abilities(&mut game, aura, first);
        assert!(!game.set_suspected(first), "the Aura owns the prohibition");
        assert!(game.attach_object_to_target(aura, AttachmentTarget::Object(second)));
        game.refresh_continuous_state().unwrap();
        assert!(game.set_suspected(first)); assert!(!game.set_suspected(second));
        game.phase_out(aura); assert!(game.set_suspected(second));
        game.clear_suspected(second); game.phase_in(aura); assert!(!game.set_suspected(second));
        game.move_object_by_game_rule(aura, Zone::Graveyard).unwrap(); assert!(game.set_suspected(second));
    }
}

#[test]
fn deadly_optional_clear_keeps_counter_and_uses_spell_controller_for_choice() {
    for definition in definitions("Deadly Complication") { for accept in [false, true] {
        let mut game = game(); let target = creature(&mut game, A, "Suspect target", Zone::Battlefield); game.set_suspected(target);
        let mut dm = Choices { target: Some(target), modes: Some(vec![1]), accept, ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 1);
        assert_eq!(game.is_suspected(target), !accept);
        assert_eq!(dm.boolean_players, vec![A]);
    } }
}

#[test]
fn deadly_pending_choice_rolls_back_counter_then_resumes_once() {
    for definition in definitions("Deadly Complication") {
        let mut game = game(); let target = creature(&mut game, A, "Pending target", Zone::Battlefield); game.set_suspected(target);
        let mut dm = Choices { target: Some(target), modes: Some(vec![1]), accept: true, ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        dm.suspend_boolean = true;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(dm.pending); assert_eq!(game.stack.len(), 1);
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 0); assert!(game.is_suspected(target));
        let mut recovered = game.clone();
        dm.pending = false; dm.suspend_boolean = false;
        resolve_stack_entry_with(&mut recovered, &mut dm).unwrap();
        assert!(recovered.stack.is_empty()); assert_eq!(recovered.counter_count(target, CounterType::PlusOnePlusOne), 1); assert!(!recovered.is_suspected(target));
        assert!(game.is_suspected(target), "recovery clone must preserve its checkpoint");
    }
}

#[test]
fn deadly_lost_target_does_not_clear_another_suspect() {
    for definition in definitions("Deadly Complication") {
        let mut game = game(); let target = creature(&mut game, A, "Announced suspect", Zone::Battlefield); game.set_suspected(target);
        let other = creature(&mut game, A, "Other suspect", Zone::Battlefield); game.set_suspected(other);
        let mut dm = Choices { target: Some(target), modes: Some(vec![1]), accept: true, ..Default::default() };
        cast(&mut game, &definition, &mut dm); game.clear_suspected(target);
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert_eq!(game.counter_count(target, CounterType::PlusOnePlusOne), 0); assert!(game.is_suspected(other)); assert!(dm.boolean_players.is_empty());
    }
}

#[test]
fn eliminate_keeps_clue_pump_and_the_exact_opposing_set() {
    for definition in definitions("Eliminate the Impossible") { for any_suspected in [false, true] {
        let mut game = game();
        let own = creature(&mut game, A, "Own suspect", Zone::Battlefield); game.set_suspected(own);
        let bob = creature(&mut game, B, "Bob creature", Zone::Battlefield);
        let carol = creature(&mut game, C, "Carol creature", Zone::Battlefield);
        let ordinary = creature(&mut game, B, "Ordinary creature", Zone::Battlefield);
        if any_suspected { game.set_suspected(bob); game.set_suspected(carol); }
        let mut dm = Choices::default(); cast(&mut game, &definition, &mut dm); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.is_suspected(own)); assert!(!game.is_suspected(bob)); assert!(!game.is_suspected(carol));
        assert_eq!(game.current_power(own), Some(3));
        for target in [bob, carol, ordinary] { assert_eq!(game.current_power(target), Some(1)); assert_eq!(game.current_toughness(target), Some(4)); }
        assert_eq!(game.battlefield.iter().filter(|id| game.object(**id).is_some_and(|object| object.name == "Clue Token" && object.owner == A)).count(), 1);
        let late = creature(&mut game, B, "Later creature", Zone::Battlefield); game.set_suspected(late);
        assert!(game.is_suspected(late)); assert_eq!(game.current_power(late), Some(3));
    } }
}

#[test]
fn coroner_unknown_payment_designation_is_incomplete_and_resolution_rolls_back() {
    for definition in definitions("Agency Coroner") {
        let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let paid = creature(&mut game, A, "Unknown paid evidence", Zone::Battlefield); game.set_suspected(paid);
        for n in 0..5 { creature(&mut game, A, &format!("Unknown draw {n}"), Zone::Library); }
        let action = compute_legal_actions(&game, A).unwrap().into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).unwrap();
        announce(&mut game, action, &mut Choices { pick: Some(paid), ..Default::default() });
        let mut changed = false;
        for snapshots in game.stack.last_mut().unwrap().tagged_objects.values_mut() {
            for snapshot in snapshots { if snapshot.object_id == paid { snapshot.suspected = None; changed = true; } }
        }
        assert!(changed, "native activation must retain the sacrificed object's snapshot");
        let before = game.player(A).unwrap().library.clone();
        assert!(resolve_stack_entry_with(&mut game, &mut Choices::default()).is_err());
        assert_eq!(game.stack.len(), 1); assert_eq!(game.player(A).unwrap().library, before); assert!(game.player(A).unwrap().hand.is_empty());
        assert!(game.object(paid).is_none(), "failure must not undo an already paid activation cost");
    }
}

#[test]
fn unsupported_full_body_lifetimes_and_completed_losses_are_not_silently_dropped() {
    let row = rows().into_iter().find(|row| row["name"] == "Hot Pursuit").unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    assert!(compile_to_artifact("Hot Pursuit", text, false).is_err());
}

#[test]
fn deadly_destroy_only_and_both_modes_keep_distinct_target_bindings() {
    for definition in definitions("Deadly Complication") { for both in [false, true] { for accept in [false, true] {
        let mut game = game();
        let destroy = creature(&mut game, B, "Destroy target", Zone::Battlefield);
        let suspect = creature(&mut game, A, "Counter target", Zone::Battlefield); game.set_suspected(suspect);
        let unrelated = creature(&mut game, C, "Unrelated suspect", Zone::Battlefield); game.set_suspected(unrelated);
        let mut dm = Choices {
            target_sequence: if both { vec![destroy, suspect] } else { vec![destroy] },
            modes: Some(if both { vec![0, 1] } else { vec![0] }), accept, ..Default::default()
        };
        cast(&mut game, &definition, &mut dm); resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        assert!(game.object(destroy).is_none());
        assert!(game.player(B).unwrap().graveyard.iter().any(|id| game.object(*id).unwrap().name == "Destroy target"));
        assert_eq!(game.counter_count(suspect, CounterType::PlusOnePlusOne), u32::from(both));
        assert_eq!(game.is_suspected(suspect), !(both && accept));
        assert!(game.is_suspected(unrelated)); assert_eq!(game.counter_count(unrelated, CounterType::PlusOnePlusOne), 0);
        assert_eq!(dm.boolean_players, if both { vec![A] } else { vec![] });
    } } }
}

#[test]
fn detached_alibi_does_not_protect_an_unrelated_enchanted_creature() {
    for definition in definitions("Airtight Alibi") {
        let mut game = game();
        let first = creature(&mut game, A, "Former Alibi host", Zone::Battlefield);
        let unrelated = creature(&mut game, B, "Other Aura host", Zone::Battlefield);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let other_definition = compile_to_runtime_definition("Other Aura", "Type: Enchantment — Aura\nEnchant creature", false).unwrap();
        let other_aura = game.create_object_from_definition(&other_definition, B, Zone::Battlefield);
        assert!(game.attach_object_to_target(source, AttachmentTarget::Object(first)));
        assert!(game.attach_object_to_target(other_aura, AttachmentTarget::Object(unrelated)));
        assert!(!game.set_suspected(first)); assert!(game.set_suspected(unrelated)); game.clear_suspected(unrelated);
        assert!(game.detach_object_from_current_target(source));
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.object(source).unwrap().attached_to, None);
        assert!(game.set_suspected(first)); assert!(game.set_suspected(unrelated));
        assert_eq!(game.current_power(first), Some(3)); assert_eq!(game.current_power(unrelated), Some(3));
    }
}

#[derive(Debug, Clone)]
struct BlinkAndSuspectReplacementAddition { original: ObjectId }
impl EffectExecutor for BlinkAndSuspectReplacementAddition {
    fn execute(&self, game: &mut GameState, _: &mut EffectContext) -> Result<ironsmith::effect::EffectOutcome, ironsmith::effects::ExecutionError> {
        let exile = game.move_object_by_game_rule(self.original, Zone::Exile).unwrap();
        let returned = game.move_object_by_game_rule(exile, Zone::Battlefield).unwrap();
        assert!(game.set_suspected(returned));
        Ok(ironsmith::effect::EffectOutcome::count(1))
    }
}

#[test]
fn deadly_counter_replacement_blink_cannot_redirect_optional_clear_to_new_incarnation() {
    for definition in definitions("Deadly Complication") { for pause in [false, true] {
        let mut game = game();
        let original = creature(&mut game, A, "Replaced counter recipient", Zone::Battlefield); game.set_suspected(original);
        let stable = game.object(original).unwrap().stable_id;
        let replacement_source = creature(&mut game, B, "Counter replacement source", Zone::Battlefield);
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ironsmith::replacement::ReplacementEffect::with_matcher(
            replacement_source, B, ironsmith::events::counters::matchers::WouldPutCountersMatcher::any(),
            ironsmith::replacement::ReplacementAction::Additionally(vec![Effect::new(BlinkAndSuspectReplacementAddition { original })]),
        ));
        let mut dm = Choices { target: Some(original), modes: Some(vec![1]), accept: true, ..Default::default() };
        cast(&mut game, &definition, &mut dm);
        dm.suspend_boolean = pause;
        resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        if pause {
            assert!(dm.pending); assert_eq!(game.stack.len(), 1); assert!(game.object(original).is_some());
            assert_eq!(game.counter_count(original, CounterType::PlusOnePlusOne), 0); assert!(game.is_suspected(original));
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some());
            dm.pending = false; dm.suspend_boolean = false;
            resolve_stack_entry_with(&mut game, &mut dm).unwrap();
        }
        let returned = game.find_object_by_stable_id(stable).unwrap();
        assert_ne!(original, returned); assert!(game.object(original).is_none());
        assert_eq!(game.object(returned).unwrap().zone, Zone::Battlefield);
        assert!(game.is_suspected(returned), "the old announced target is the only object the clear instruction names");
        assert_eq!(game.counter_count(returned, CounterType::PlusOnePlusOne), 0);
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none()); assert!(game.stack.is_empty());
    } }
}

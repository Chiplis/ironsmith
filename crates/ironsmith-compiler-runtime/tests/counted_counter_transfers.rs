//! Complete frozen counter-transfer bodies. These scenarios are authored but
//! unrun while the user's implementation-first validation gate is closed.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::decisions::context::{BooleanContext, NumberContext, TargetsContext};
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::game_state::{Phase, Step};
use ironsmith::replacement::{ReplacementAction, ReplacementEffect};
use ironsmith::target::{ChooseSpec, ObjectFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{CounterType, GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const KIND: CounterType = CounterType::PlusOnePlusOne;
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{}", loss.reasons_text());
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    restored.validate().unwrap(); assert_eq!(artifact, restored);
    let materialized = ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&restored).unwrap();
    for definition in [&direct, &materialized] {
        assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(definition));
    }
    [direct, materialized]
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/counted_counter_transfers.json.fixture")).unwrap();
    assert_eq!(rows.len(), 4);
    let row = rows.iter().find(|row| row["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    definitions_text(name, &text)
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    game.turn.phase = Phase::FirstMain; game.turn.active_player = A; game.turn.priority_player = Some(A); game
}
fn witness(game: &mut GameState, owner: PlayerId, zone: Zone, types: &str, counters: u32) -> ObjectId {
    let definition = compile_to_runtime_definition("Transfer witness", &format!("Type: {types}\nPower/Toughness: 2/2"), false).unwrap();
    let id = game.create_object_from_definition(&definition, owner, zone);
    if counters > 0 { game.object_mut(id).unwrap().counters.insert(KIND, counters); }
    id
}
#[derive(Default)]
struct Choices { amounts: std::collections::VecDeque<u32>, targets: std::collections::VecDeque<Target>, bounds: Vec<(u32, u32)> }
impl DecisionMaker for Choices {
    fn decide_boolean(&mut self, _: &GameState, _: &BooleanContext) -> bool { true }
    fn decide_number(&mut self, _: &GameState, context: &NumberContext) -> u32 {
        self.bounds.push((context.min, context.max));
        let amount = self.amounts.pop_front().unwrap_or(context.max);
        assert!(amount >= context.min && amount <= context.max); amount
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if self.targets.is_empty() { return SelectFirstDecisionMaker.decide_targets(game, context); }
        context.requirements.iter().map(|requirement| {
            let target = self.targets.pop_front().unwrap(); assert!(requirement.legal_targets.contains(&target)); target
        }).collect()
    }
}
fn trigger(game: &mut GameState, event: &TriggerEvent, dm: &mut Choices) -> usize {
    let entries = check_triggers(game, event); let count = entries.len(); let mut queue = TriggerQueue::new();
    for entry in entries { queue.add(entry); }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap(); count
}
fn enter(game: &mut GameState, definition: &CardDefinition, dm: &mut Choices) -> ObjectId {
    let old = game.create_object_from_definition(definition, A, Zone::Hand); let stable = game.object(old).unwrap().stable_id;
    let result = execute_effect(game, &Effect::move_to_zone(ChooseSpec::SpecificObject(old), Zone::Battlefield, false), &mut EffectContext::new(old, A, dm)).unwrap();
    for event in result.events { trigger(game, &event, dm); }
    game.find_object_by_stable_id(stable).unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    while !game.stack_is_empty() { resolve_stack_entry_with(game, dm).unwrap(); }
}
fn activate(game: &mut GameState, source: ObjectId, dm: &mut Choices) {
    let ability_index = game.current_abilities(source).unwrap().iter().position(|ability| matches!(ability.kind, AbilityKind::Activated(_))).unwrap();
    let action = compute_legal_actions(game, A).unwrap().into_iter().find(|action| matches!(action,
        LegalAction::ActivateAbility { source: id, ability_index: index } if *id == source && *index == ability_index)).unwrap();
    let mut queue = TriggerQueue::new(); let mut state = PriorityLoopState::new(2);
    let mut progress = apply_priority_response_with_dm(game, &mut queue, &mut state, &PriorityResponse::PriorityAction(action), dm).unwrap();
    for _ in 0..32 {
        if !state.has_pending_action() { return; }
        let GameProgress::NeedsDecisionCtx(context) = progress else { panic!("pending activation must expose its decision"); };
        *game = game.clone(); state = state.clone();
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &context, dm).unwrap();
    }
    panic!("activation failed to settle");
}

#[test]
fn marauder_entry_keeps_flying_lifelink_all_owned_permanents_and_partial_choices() {
    for definition in definitions("Aetherborn Marauder") {
        let mut game = game(); let first = witness(&mut game, A, Zone::Battlefield, "Artifact", 2);
        let second = witness(&mut game, A, Zone::Battlefield, "Creature", 3);
        let foreign = witness(&mut game, B, Zone::Battlefield, "Creature", 7);
        game.object_mut(first).unwrap().counters.insert(CounterType::Charge, 4);
        let mut dm = Choices { amounts: [0, 2].into(), ..Default::default() };
        let source = enter(&mut game, &definition, &mut dm);
        for ability in [ironsmith::static_abilities::StaticAbilityId::Flying, ironsmith::static_abilities::StaticAbilityId::Lifelink] {
            assert!(game.current_has_static_ability_id(source, ability));
        }
        let native = game.clone();
        for mut branch in [game, native] {
            let mut choices = Choices { amounts: [0, 2].into(), ..Default::default() }; resolve(&mut branch, &mut choices);
            assert_eq!(choices.bounds, vec![(0, 2), (0, 3)]);
            assert_eq!(branch.counter_count(source, KIND), 2); assert_eq!(branch.counter_count(first, KIND), 2);
            assert_eq!(branch.counter_count(second, KIND), 1); assert_eq!(branch.counter_count(foreign, KIND), 7);
            assert_eq!(branch.counter_count(first, CounterType::Charge), 4);
        }
    }
}

#[test]
fn spike_entry_keeps_its_initial_counter_and_collects_all_creatures_on_both_sides() {
    for definition in definitions("Spike Cannibal") {
        let mut game = game(); let first = witness(&mut game, A, Zone::Battlefield, "Creature", 2);
        let second = witness(&mut game, B, Zone::Battlefield, "Creature", 3);
        let artifact = witness(&mut game, A, Zone::Battlefield, "Artifact", 7);
        let mut dm = Choices::default(); let source = enter(&mut game, &definition, &mut dm);
        assert_eq!(game.counter_count(source, KIND), 1);
        resolve(&mut game, &mut dm);
        assert_eq!(game.counter_count(source, KIND), 6); assert_eq!(game.counter_count(first, KIND), 0);
        assert_eq!(game.counter_count(second, KIND), 0); assert_eq!(game.counter_count(artifact, KIND), 7);
        assert!(dm.bounds.is_empty(), "all counters is not an optional quantity");
    }
}

#[test]
fn bandar_keeps_entry_counters_controller_upkeep_and_optional_partial_transfer() {
    for definition in definitions("Scrounging Bandar") {
        let mut game = game(); let mut dm = Choices::default(); let source = enter(&mut game, &definition, &mut dm);
        assert_eq!(game.counter_count(source, KIND), 2); assert!(game.stack_is_empty());
        let target = witness(&mut game, B, Zone::Battlefield, "Creature", 0);
        let foreign = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(B), Default::default());
        assert_eq!(trigger(&mut game, &foreign, &mut dm), 0);
        game.turn.phase = Phase::Beginning; game.turn.step = Some(Step::Upkeep);
        dm.targets.push_back(Target::Object(target)); dm.amounts.push_back(1);
        let upkeep = TriggerEvent::new_with_provenance(ironsmith::events::BeginningOfUpkeepEvent::new(A), Default::default());
        assert_eq!(trigger(&mut game, &upkeep, &mut dm), 1);
        let native = game.clone();
        for mut branch in [game, native] {
            let mut choices = Choices { amounts: [1].into(), ..Default::default() }; resolve(&mut branch, &mut choices);
            assert_eq!(branch.counter_count(source, KIND), 1); assert_eq!(branch.counter_count(target, KIND), 1);
        }
    }
}

#[test]
fn black_panther_native_activation_counts_removed_counters_and_preserves_both_targets() {
    for definition in definitions("Black Panther, Wakandan King") {
        for scenario in ["ordinary", "doubled", "same object", "land left", "creature left", "prevented removal"] {
            let mut game = game(); let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert!(game.current_has_static_ability_id(source, ironsmith::static_abilities::StaticAbilityId::FirstStrike));
            let land = witness(&mut game, A, Zone::Battlefield, "Land Creature", 3);
            let target = if scenario == "same object" { land } else { witness(&mut game, B, Zone::Battlefield, "Creature", 0) };
            for _ in 0..3 { witness(&mut game, A, Zone::Library, "Creature", 0); }
            game.player_mut(A).unwrap().mana_pool.add(ironsmith::mana::ManaSymbol::Colorless, 3);
            let mut dm = Choices { targets: [Target::Object(land), Target::Object(target)].into(), ..Default::default() };
            activate(&mut game, source, &mut dm);
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            if scenario == "doubled" { game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
                ironsmith::events::counters::matchers::WouldPutCountersMatcher::any(),
                ReplacementAction::Modify(ironsmith::replacement::EventModification::Multiply(2)))); }
            if scenario == "prevented removal" { game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, A,
                ironsmith::events::counters::matchers::WouldRemoveCountersMatcher::new(ObjectFilter::specific(land), Some(KIND)), ReplacementAction::Prevent)); }
            if matches!(scenario, "land left" | "creature left") {
                game.move_object_by_effect(if scenario == "land left" { land } else { target }, Zone::Graveyard).unwrap();
            }
            let native = game.clone();
            for mut branch in [game, native] {
                resolve(&mut branch, &mut Choices::default());
                let moved = matches!(scenario, "ordinary" | "doubled");
                assert_eq!(branch.player(A).unwrap().life, if moved { 23 } else { 20 }, "{scenario}");
                assert_eq!(branch.player(A).unwrap().hand.len(), usize::from(moved), "{scenario}");
                if scenario == "same object" { assert_eq!(branch.counter_count(land, KIND), 3); }
                if moved { assert_eq!(branch.counter_count(land, KIND), 0); assert_eq!(branch.counter_count(target, KIND), if scenario == "doubled" { 6 } else { 3 }); }
            }
        }
    }
}

#[test]
fn black_panther_survey_triggers_for_self_and_other_owned_creatures_only() {
    for definition in definitions("Black Panther, Wakandan King") {
        let mut game = game(); let land = witness(&mut game, A, Zone::Battlefield, "Land", 0);
        let mut dm = Choices { targets: [Target::Object(land)].into(), ..Default::default() };
        let source = enter(&mut game, &definition, &mut dm); resolve(&mut game, &mut dm);
        assert_eq!(game.counter_count(land, KIND), 1);
        for (owner, expected) in [(A, 2), (B, 2)] {
            let old = witness(&mut game, owner, Zone::Hand, "Creature", 0);
            dm.targets.push_back(Target::Object(land));
            let out = execute_effect(&mut game, &Effect::move_to_zone(ChooseSpec::SpecificObject(old), Zone::Battlefield, false), &mut EffectContext::new(source, owner, &mut dm)).unwrap();
            for event in out.events { trigger(&mut game, &event, &mut dm); } resolve(&mut game, &mut dm);
            assert_eq!(game.counter_count(land, KIND), expected);
        }
    }
}

#[test]
fn an_unlisted_name_uses_the_same_transfer_and_result_grammar() {
    for definition in definitions_text("Unlisted Counter Carrier", "Type: Artifact\n{0}: Move all charge counters from target artifact onto target creature. If one or more charge counters are moved this way, you gain that much life and draw a card.") {
        assert_eq!(definition.abilities.iter().filter(|ability| matches!(ability.kind, AbilityKind::Activated(_))).count(), 1);
    }
}

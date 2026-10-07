//! Recovered source-only scenarios. Builds and execution remain deferred.
use ironsmith::effect::{Effect, Value};
use ironsmith::effects::BolsterEffect;

#[test]
fn fixed_and_dynamic_bolster_payloads_round_trip_without_narrowing_unsigned_values() {
    for effect in [
        BolsterEffect::new(2),
        BolsterEffect::new(u32::MAX),
        BolsterEffect::with_value(Value::CardsInHand(ironsmith::target::PlayerFilter::You)),
    ] {
        let model = ironsmith_runtime_catalog::artifact_materializer::encode_runtime_effect(
            Effect::new(effect.clone())).unwrap();
        let model = serde_json::from_slice(&serde_json::to_vec(&model).unwrap()).unwrap();
        let restored = ironsmith_runtime_catalog::artifact_materializer::materialize_effect(model).unwrap();
        assert_eq!(restored.downcast_ref::<BolsterEffect>(), Some(&effect));
    }
    let old: BolsterEffect = serde_json::from_str(r#"{"amount":2}"#).unwrap();
    assert_eq!(old, BolsterEffect::new(2));
}

mod full_cards {
//! Recovered full-card dynamic keyword scenarios; execution is deferred.
#![allow(dead_code)]
use ironsmith::alternative_cast::CastingMethod;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::{
    DecisionMaker, LegalAction, SelectFirstDecisionMaker, compute_legal_actions,
};
use ironsmith::decisions::context::{
    BooleanContext, NumberContext, SelectObjectsContext, SelectOptionsContext, TargetsContext,
};
use ironsmith::effect::{Effect, EffectOutcome, Until};
use ironsmith::effects::{EffectContext, execute_effect};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_decision_context_with_dm,
    apply_priority_response_with_dm, put_triggers_on_stack_with_dm, resolve_stack_entry_with,
};
use ironsmith::game_state::Phase;
use ironsmith::mana::ManaSymbol;
use ironsmith::target::{ChooseSpec, PlayerFilter};
use ironsmith::triggers::{TriggerEvent, TriggerQueue, check_triggers};
use ironsmith::{GameProgress, GameState, ObjectId, PlayerId, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

fn fixtures() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/dynamic_keyword_amounts.json.fixture"
    ))
    .unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = fixtures().into_iter().find(|r| r["name"] == name).unwrap();
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(p), Some(t)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {p}/{t}"));
    }
    if let Some(loyalty) = row["loyalty"].as_str() {
        lines.push(format!("Loyalty: {loyalty}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().into());
    definitions_text(name, &lines.join("\n"))
}
fn definitions_text(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, _) = result.unwrap_or_else(|e| panic!("{name}: {e}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let (result, direct_loss) = parse_loss::capture(|| compile_to_runtime_definition(name, text, false));
    let direct = result.unwrap_or_else(|e| panic!("{name} direct: {e}"));
    assert!(!direct_loss.is_lossy(), "{name} direct: {}", direct_loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    [
        direct,
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap(),
    ]
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    for color in [
        ManaSymbol::White,
        ManaSymbol::Blue,
        ManaSymbol::Black,
        ManaSymbol::Red,
        ManaSymbol::Green,
        ManaSymbol::Colorless,
    ] {
        game.player_mut(A).unwrap().mana_pool.add(color, 20);
    }
    game
}
fn vanilla(name: &str, cost: &str, subtype: &str, p: i32, t: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {cost}\nType: Creature — {subtype}\nPower/Toughness: {p}/{t}"),
        false,
    )
    .unwrap()
}
#[derive(Default)]
struct Choices {
    targets: Vec<Target>,
    objects: Vec<ObjectId>,
    objects_explicit: bool,
    x: u32,
    decline: bool,
    land_choice: Option<&'static str>,
    land_prompts: usize,
    modes: Vec<usize>,
}
impl DecisionMaker for Choices {
    fn decide_options(&mut self, game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
        if ctx.description.starts_with("Choose mode") && !self.modes.is_empty() {
            return self.modes.clone();
        }
        if ctx.description == "Choose a basic land type" {
            self.land_prompts += 1;
            return vec![
                ctx.options
                    .iter()
                    .find(|option| option.description == self.land_choice.unwrap_or("Island"))
                    .unwrap()
                    .index,
            ];
        }
        if ctx.description.starts_with("Choose optional costs") {
            if self.decline {
                return vec![];
            }
            return ctx
                .options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .collect();
        }
        SelectFirstDecisionMaker.decide_options(game, ctx)
    }
    fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
        !self.decline
    }
    fn decide_number(&mut self, game: &GameState, ctx: &NumberContext) -> u32 {
        if ctx.is_x_value {
            assert!(self.x <= ctx.max);
            self.x
        } else {
            SelectFirstDecisionMaker.decide_number(game, ctx)
        }
    }
    fn decide_targets(&mut self, game: &GameState, context: &TargetsContext) -> Vec<Target> {
        if !self.targets.is_empty() {
            assert_eq!(context.requirements.len(), self.targets.len());
            for (requirement, target) in context.requirements.iter().zip(&self.targets) {
                assert!(requirement.legal_targets.contains(target));
            }
            self.targets.clone()
        } else {
            SelectFirstDecisionMaker.decide_targets(game, context)
        }
    }
    fn decide_objects(
        &mut self,
        game: &GameState,
        context: &SelectObjectsContext,
    ) -> Vec<ObjectId> {
        if self.objects_explicit || !self.objects.is_empty() {
            for id in &self.objects {
                assert!(
                    context
                        .candidates
                        .iter()
                        .any(|candidate| candidate.id == *id && candidate.legal)
                );
            }
            self.objects.clone()
        } else {
            SelectFirstDecisionMaker.decide_objects(game, context)
        }
    }
}
fn apply(game: &mut GameState, source: ObjectId, effect: Effect) -> EffectOutcome {
    let mut dm = SelectFirstDecisionMaker;
    let controller = game.current_controller(source).unwrap_or(A);
    execute_effect(
        game,
        &effect,
        &mut EffectContext::new(source, controller, &mut dm),
    )
    .unwrap()
}
fn resolve(game: &mut GameState, dm: &mut Choices) {
    resolve_stack_entry_with(game, dm).unwrap();
}
fn resolve_all(game: &mut GameState, dm: &mut Choices) {
    for _ in 0..30 {
        if game.stack_is_empty() {
            return;
        }
        resolve(game, dm);
    }
    panic!("unexpected continuing trigger chain");
}
fn resource(name: &str, types: &str) -> CardDefinition {
    let pt = if types.contains("Creature") {
        "\nPower/Toughness: 1/1"
    } else {
        ""
    };
    compile_to_runtime_definition(name, format!("Mana cost: {{1}}\nType: {types}{pt}"), false)
        .unwrap()
}
fn queue_outcome(game: &mut GameState, outcome: EffectOutcome, dm: &mut Choices) {
    let mut queue = TriggerQueue::new();
    for event in outcome.events {
        for entry in check_triggers(game, &event) {
            queue.add(entry);
        }
    }
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn pt(game: &GameState, id: ObjectId) -> (i32, i32) {
    (
        game.current_power(id).unwrap(),
        game.current_toughness(id).unwrap(),
    )
}
fn pump(game: &mut GameState, source: ObjectId, id: ObjectId, p: i32, t: i32) {
    apply(
        game,
        source,
        Effect::pump(p, t, ChooseSpec::SpecificObject(id), Until::EndOfTurn),
    );
}
fn counter(game: &mut GameState, source: ObjectId, id: ObjectId, count: i32) {
    apply(
        game,
        source,
        Effect::put_counters(
            ironsmith::object::CounterType::PlusOnePlusOne,
            count,
            ChooseSpec::SpecificObject(id),
        ),
    );
}
fn cast(
    game: &mut GameState,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    cast_for(game, A, definition, method, dm)
}
fn cast_for(
    game: &mut GameState,
    player: PlayerId,
    definition: &CardDefinition,
    method: CastingMethod,
    dm: &mut Choices,
) -> ObjectId {
    let id = game.create_object_from_definition(definition, player, Zone::Hand);
    let action = LegalAction::CastSpell {
        spell_id: id,
        from_zone: Zone::Hand,
        casting_method: method,
    };
    assert!(
        compute_legal_actions(game, player)
            .unwrap()
            .contains(&action)
    );
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_cast.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}");
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_cast.is_none());
    let spell = game
        .stack
        .iter()
        .find(|entry| !entry.is_ability)
        .unwrap()
        .object_id;
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
    spell
}

fn activate(game: &mut GameState, source: ObjectId, ability_index: usize, dm: &mut Choices) {
    game.turn.priority_player = Some(A);
    let action = LegalAction::ActivateAbility {
        source,
        ability_index,
    };
    assert!(compute_legal_actions(game, A).unwrap().contains(&action));
    let mut queue = TriggerQueue::new();
    let mut state = PriorityLoopState::new(3);
    let mut progress = apply_priority_response_with_dm(
        game,
        &mut queue,
        &mut state,
        &PriorityResponse::PriorityAction(action),
        dm,
    )
    .unwrap();
    for _ in 0..60 {
        if state.pending_activation.is_none() {
            break;
        }
        let GameProgress::NeedsDecisionCtx(ctx) = progress else {
            panic!("{progress:?}")
        };
        progress = apply_decision_context_with_dm(game, &mut queue, &mut state, &ctx, dm).unwrap();
    }
    assert!(state.pending_activation.is_none());
}
fn activated_at(definition: &CardDefinition, position: usize) -> usize {
    definition
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(i, a)| {
            matches!(a.kind, ironsmith::ability::AbilityKind::Activated(_)).then_some(i)
        })
        .nth(position)
        .unwrap()
}

fn has(game: &GameState, id: ObjectId, kind: ironsmith::static_abilities::StaticAbilityId) -> bool {
    game.current_has_static_ability_id(id, kind)
}
fn phase_event(game: &mut GameState, event: TriggerEvent, dm: &mut Choices) {
    game.queue_trigger_event(Default::default(), event);
    put_triggers_on_stack_with_dm(game, &mut TriggerQueue::new(), dm).unwrap();
}
fn declare(game: &mut GameState, id: ObjectId, dm: &mut Choices) {
    game.remove_summoning_sickness(id);
    game.turn.active_player = A;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    let mut combat = ironsmith::combat_state::CombatState::default();
    let mut queue = TriggerQueue::new();
    let declaration = ironsmith::decision::AttackerDeclaration {
        creature: id, target: ironsmith::combat_state::AttackTarget::Player(B),
    };
    ironsmith::game_loop::apply_attacker_declarations(game, &mut combat, &mut queue, &[declaration]).unwrap();
    game.combat = Some(combat);
    put_triggers_on_stack_with_dm(game, &mut queue, dm).unwrap();
}
fn warrior_tokens(game: &GameState) -> Vec<ObjectId> {
    game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|object| {
        object.kind == ironsmith::object::ObjectKind::Token && game.controller_of(object) == A
            && object.subtypes.contains(&ironsmith::types::Subtype::Warrior)
    })).collect()
}
fn token(game: &mut GameState, source: ObjectId, name: &str, types: &str, count: i32, player: PlayerId) {
    let definition = resource(name, types);
    apply(game, source, Effect::new(ironsmith::effects::CreateTokenEffect::new(
        definition, count, PlayerFilter::Specific(player))));
}
#[test]
fn five_frozen_complete_bodies_have_strict_direct_and_artifact_programs() {
    for name in ["Dragonscale General", "Sandsteppe War Riders", "Sunbringer's Touch", "Avenger of the Fallen", "Infantry Shield"] {
        let _ = definitions(name);
    }
}
#[test]
fn general_bolster_reads_resolving_tapped_count_and_current_least_toughness() {
    for definition in definitions("Dragonscale General") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let small = game.create_object_from_definition(&vanilla("Small", "{1}", "Human", 1, 1), A, Zone::Battlefield);
        let large = game.create_object_from_definition(&vanilla("Large", "{1}", "Human", 4, 4), A, Zone::Battlefield);
        let phased = game.create_object_from_definition(&vanilla("Phased", "{1}", "Human", 1, 1), A, Zone::Battlefield);
        game.tap(phased);
        game.phase_out(phased);
        game.tap(small); game.tap(large);
        let mut dm = Choices::default();
        phase_event(&mut game, TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfEndStepEvent::new(A), Default::default()), &mut dm);
        game.tap(source);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.counter_count(small, ironsmith::object::CounterType::PlusOnePlusOne), 3);
        assert_eq!(game.counter_count(large, ironsmith::object::CounterType::PlusOnePlusOne), 0);
        assert_eq!(game.counter_count(phased, ironsmith::object::CounterType::PlusOnePlusOne), 0);
    }
}
#[test]
fn sandsteppe_counts_only_differently_named_own_artifact_tokens() {
    for definition in definitions("Sandsteppe War Riders") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let small = game.create_object_from_definition(&vanilla("Small", "{1}", "Human", 1, 1), A, Zone::Battlefield);
        assert!(has(&game, source, ironsmith::static_abilities::StaticAbilityId::Trample));
        token(&mut game, source, "Treasure", "Artifact — Treasure", 2, A);
        token(&mut game, source, "Food", "Artifact — Food", 1, A);
        token(&mut game, source, "Enemy Clue", "Artifact — Clue", 1, B);
        token(&mut game, source, "Warrior", "Creature — Warrior", 1, A);
        let mut dm = Choices { objects: vec![small], ..Default::default() };
        phase_event(&mut game, TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfCombatEvent::new(A), Default::default()), &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.counter_count(small, ironsmith::object::CounterType::PlusOnePlusOne), 2);
    }
}
#[test]
fn sandsteppe_uses_current_names_after_trigger_time_name_changes() {
    for definition in definitions("Sandsteppe War Riders") { for split in [false, true] {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let small = game.create_object_from_definition(&vanilla("Small", "{1}", "Human", 1, 1), A, Zone::Battlefield);
        token(&mut game, source, "Treasure", "Artifact — Treasure", 2, A);
        token(&mut game, source, "Food", "Artifact — Food", 1, A);
        let owned = game.battlefield.iter().copied().filter(|id| game.object(*id).is_some_and(|object| {
            object.kind == ironsmith::object::ObjectKind::Token && game.controller_of(object) == A
        })).collect::<Vec<_>>();
        let food = *owned.iter().find(|id| game.object(**id).unwrap().name.as_str() == "Food").unwrap();
        let treasure = *owned.iter().find(|id| game.object(**id).unwrap().name.as_str() == "Treasure").unwrap();
        let mut dm = Choices::default();
        phase_event(&mut game, TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfCombatEvent::new(A), Default::default()), &mut dm);
        let (changed, name, expected) = if split { (treasure, "Clue", 3) } else { (food, "Treasure", 1) };
        apply(&mut game, source, Effect::new(ironsmith::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(changed), ironsmith::continuous::Modification::SetName(name.into()), Until::EndOfTurn)));
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.counter_count(small, ironsmith::object::CounterType::PlusOnePlusOne), expected);
    }}
}

#[test]
fn sandsteppe_uses_layer_one_copy_names_when_the_trigger_resolves() {
    for definition in definitions("Sandsteppe War Riders") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let small = game.create_object_from_definition(&vanilla("Small", "{1}", "Human", 1, 1), A, Zone::Battlefield);
        token(&mut game, source, "Treasure", "Artifact — Treasure", 1, A);
        token(&mut game, source, "Food", "Artifact — Food", 1, A);
        let named = |game: &GameState, name: &str| *game.battlefield.iter()
            .find(|id| game.object(**id).unwrap().name.as_str() == name).unwrap();
        let food = named(&game, "Food");
        let treasure = named(&game, "Treasure");
        let mut dm = Choices::default();
        phase_event(&mut game, TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfCombatEvent::new(A), Default::default()), &mut dm);
        apply(&mut game, source, Effect::new(ironsmith::effects::ApplyContinuousEffect::new_runtime(
            ironsmith::continuous::EffectTarget::Specific(food),
            ironsmith::effects::continuous::RuntimeModification::CopyOf {
                source: ChooseSpec::SpecificObject(treasure),
                preserve_source_abilities: false,
                name_override: None,
                name_override_surface: None,
                add_supertypes: Vec::new(),
                copy_exception_surface: None,
            }, Until::EndOfTurn)));
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.counter_count(small, ironsmith::object::CounterType::PlusOnePlusOne), 1);
    }
}
#[test]
fn sunbringer_uses_current_hand_then_grants_the_complete_counter_creature_set() {
    for definition in definitions("Sunbringer's Touch") { for hand_size in [0, 3] {
        let mut game = game();
        let small = game.create_object_from_definition(&vanilla("Small", "{1}", "Human", 1, 1), A, Zone::Battlefield);
        let prior = game.create_object_from_definition(&vanilla("Prior", "{1}", "Human", 4, 4), A, Zone::Battlefield);
        let absent = game.create_object_from_definition(&vanilla("Absent", "{1}", "Human", 4, 4), A, Zone::Battlefield);
        game.add_counters(prior, ironsmith::object::CounterType::PlusOnePlusOne, 1).unwrap();
        let mut dm = Choices::default();
        cast(&mut game, &definition, CastingMethod::Normal, &mut dm);
        for _ in 0..hand_size { game.create_object_from_definition(&vanilla("Hand", "{1}", "Human", 1, 1), A, Zone::Hand); }
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.counter_count(small, ironsmith::object::CounterType::PlusOnePlusOne), hand_size);
        assert_eq!(has(&game, small, ironsmith::static_abilities::StaticAbilityId::Trample), hand_size > 0);
        assert!(has(&game, prior, ironsmith::static_abilities::StaticAbilityId::Trample));
        assert!(!has(&game, absent, ironsmith::static_abilities::StaticAbilityId::Trample));
        let late = game.create_object_from_definition(&vanilla("Late", "{1}", "Human", 1, 1), A, Zone::Battlefield);
        game.add_counters(late, ironsmith::object::CounterType::PlusOnePlusOne, 1).unwrap();
        assert!(!has(&game, late, ironsmith::static_abilities::StaticAbilityId::Trample));
        ironsmith::turn::execute_cleanup_step(&mut game);
        assert!(!has(&game, prior, ironsmith::static_abilities::StaticAbilityId::Trample));
        assert!(!has(&game, small, ironsmith::static_abilities::StaticAbilityId::Trample));
    }}
}
#[test]
fn avenger_reads_graveyard_quantity_at_resolution_and_retains_exact_token_cleanup() {
    for definition in definitions("Avenger of the Fallen") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        assert!(has(&game, source, ironsmith::static_abilities::StaticAbilityId::Deathtouch));
        for _ in 0..2 { game.create_object_from_definition(&vanilla("Dead", "{1}", "Human", 1, 1), A, Zone::Graveyard); }
        game.create_object_from_definition(&resource("Graveyard artifact", "Artifact"), A, Zone::Graveyard);
        game.create_object_from_definition(&vanilla("Enemy dead", "{1}", "Human", 1, 1), B, Zone::Graveyard);
        token(&mut game, source, "Existing Warrior", "Creature — Warrior", 1, A);
        let existing = warrior_tokens(&game);
        let mut dm = Choices::default();
        declare(&mut game, source, &mut dm);
        game.create_object_from_definition(&vanilla("Later dead", "{1}", "Human", 1, 1), A, Zone::Graveyard);
        resolve_all(&mut game, &mut dm);
        let tokens = warrior_tokens(&game).into_iter().filter(|id| !existing.contains(id)).collect::<Vec<_>>();
        assert_eq!(tokens.len(), 3);
        for id in &tokens {
            assert_eq!(game.object(*id).unwrap().name.as_ref(), "Warrior Token");
            assert_eq!(pt(&game, *id), (1, 1));
            assert_eq!(game.current_colors(*id), Some(ironsmith::color::ColorSet::RED));
            assert!(game.is_tapped(*id));
            assert!(game.combat.as_ref().unwrap().attackers.iter().any(|attacker| attacker.creature == *id));
        }
        phase_event(&mut game, TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfEndStepEvent::new(B), Default::default()), &mut dm);
        resolve_all(&mut game, &mut dm);
        for id in tokens { assert!(!game.battlefield.contains(&id)); }
        assert!(game.battlefield.contains(&source));
        assert!(existing.iter().all(|id| game.battlefield.contains(id)));
    }
}
#[test]
fn shield_pays_equip_then_uses_recipient_power_after_its_own_departure() {
    for definition in definitions("Infantry Shield") {
        let mut game = game();
        let equipment = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let recipient = game.create_object_from_definition(&vanilla("Bearer", "{1}", "Human", 3, 4), A, Zone::Battlefield);
        let mut dm = Choices { targets: vec![Target::Object(recipient)], ..Default::default() };
        let before = game.player(A).unwrap().mana_pool.total();
        activate(&mut game, equipment, activated_at(&definition, 0), &mut dm);
        resolve_all(&mut game, &mut dm);
        assert_eq!(game.player(A).unwrap().mana_pool.total(), before - 2);
        assert!(has(&game, recipient, ironsmith::static_abilities::StaticAbilityId::Menace));
        dm.targets.clear();
        declare(&mut game, recipient, &mut dm);
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.stack[0].object_id, recipient);
        assert_eq!(game.stack[0].controller, A);
        game.move_object_by_effect(equipment, Zone::Graveyard).unwrap();
        pump(&mut game, recipient, recipient, 2, 0);
        resolve_all(&mut game, &mut dm);
        let tokens = warrior_tokens(&game);
        assert_eq!(tokens.len(), 5);
        assert!(tokens.iter().all(|id| game.object(*id).unwrap().name.as_ref() == "Warrior Token"));
        assert!(!has(&game, recipient, ironsmith::static_abilities::StaticAbilityId::Menace));
        phase_event(&mut game, TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfEndStepEvent::new(B), Default::default()), &mut dm);
        resolve_all(&mut game, &mut dm);
        assert!(tokens.iter().all(|id| !game.battlefield.contains(id)));
        assert!(game.battlefield.contains(&recipient));
    }
}

#[test]
fn avenger_zero_graveyard_creatures_creates_no_tokens_or_cleanup() {
    for definition in definitions("Avenger of the Fallen") {
        let mut game = game();
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.create_object_from_definition(&resource("Only an artifact", "Artifact"), A, Zone::Graveyard);
        let mut dm = Choices::default();
        declare(&mut game, source, &mut dm);
        resolve_all(&mut game, &mut dm);
        assert!(warrior_tokens(&game).is_empty());
        phase_event(&mut game, TriggerEvent::new_with_provenance(
            ironsmith::events::BeginningOfEndStepEvent::new(B), Default::default()), &mut dm);
        assert!(game.stack_is_empty());
        assert!(game.battlefield.contains(&source));
    }
}
}

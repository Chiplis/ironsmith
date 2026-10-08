//! Full frozen bodies, direct and JSON-restored artifacts. Authored, UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::card::{PowerToughness, PtValue};
use ironsmith::cards::CardDefinition;
use ironsmith::combat_state::{AttackTarget, CombatState};
use ironsmith::decision::{AttackerDeclaration, DecisionMaker, SelectFirstDecisionMaker};
use ironsmith::decisions::context::TargetsContext;
use ironsmith::effect::{Condition, Value, ValueComparisonOperator};
use ironsmith::game_loop::{apply_attacker_declarations, drain_pending_trigger_events,
    put_triggers_on_stack_with_dm, resolve_stack_entry_with};
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::target::{ObjectFilter, PlayerFilter};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{CardType, GameState, ObjectId, Phase, PlayerId, Subtype, Target, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);
const D: PlayerId = PlayerId::from_index(3);
const TEAM_CARDS: &[&str] = &["Aurora Champion", "Bull-Rush Bruiser", "Sickle Dancer"];
const LESSON_CARDS: &[&str] = &["Dragonfly Swarm", "Walltop Sentries"];

fn rows() -> Vec<serde_json::Value> {
    serde_json::from_str(include_str!("../../../fixtures/intervening_predicate_cohort.json.fixture")).unwrap()
}
fn definitions(name: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    definitions_with_oracle(name, row["oracle_text"].as_str().unwrap())
}
fn definitions_with_oracle(name: &str, oracle: &str) -> [CardDefinition; 2] {
    let row = rows().into_iter().find(|row| row["name"] == name).unwrap();
    let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
        row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
        row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(),
        oracle);
    let (direct, direct_loss) = ironsmith_compiler::parse_loss::capture(||
        compile_to_runtime_definition(name, &text, false));
    let direct = direct.unwrap_or_else(|error| panic!("direct {name}: {error}"));
    assert!(!direct_loss.is_lossy(), "direct {name}: {}", direct_loss.reasons_text());
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, &text, false));
    // The second return is itself materialized from the artifact, not an
    // independent direct compiler-runtime conversion.
    let (artifact, _) = result.unwrap_or_else(|error| panic!("artifact {name}: {error}"));
    assert!(!loss.is_lossy(), "artifact {name}: {}", loss.reasons_text());
    artifact.validate().unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn object(game: &mut GameState, player: PlayerId, zone: Zone, types: &str) -> ObjectId {
    let text = format!("Type: {types}{}", if types.contains("Creature") { "\nPower/Toughness: 2/2" } else { "" });
    let definition = compile_to_runtime_definition("Predicate resource", text, false).unwrap();
    let id = game.create_object_from_definition(&definition, player, zone);
    game.remove_summoning_sickness(id);
    id
}
fn game() -> GameState {
    let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
    game.set_teams(vec![vec![A, B], vec![C, D]]).unwrap();
    game.turn.active_player = A;
    game.turn.priority_player = Some(A);
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
    game.mark_combat_phase_started();
    for player in [A, B, C, D] {
        for _ in 0..3 { object(&mut game, player, Zone::Library, "Artifact"); }
    }
    game
}
fn source(game: &mut GameState, definition: &CardDefinition) -> ObjectId {
    let id = game.create_object_from_definition(definition, A, Zone::Battlefield);
    game.remove_summoning_sickness(id);
    id
}
fn attack(game: &mut GameState, source: ObjectId) -> TriggerQueue {
    let mut queue = TriggerQueue::new();
    apply_attacker_declarations(game, &mut CombatState::default(), &mut queue,
        &[AttackerDeclaration { creature: source, target: AttackTarget::Player(C) }]).unwrap();
    queue
}
fn die(game: &mut GameState, source: ObjectId) -> TriggerQueue {
    game.take_pending_trigger_events();
    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
    let mut queue = TriggerQueue::new();
    drain_pending_trigger_events(game, &mut queue);
    queue
}
struct Targets {
    target: ObjectId,
    forbidden: ObjectId,
    calls: usize,
}
impl DecisionMaker for Targets {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert_eq!(context.player, A);
        assert_eq!(context.requirements.len(), 1);
        assert!(context.requirements[0].legal_targets.contains(&Target::Object(self.target)));
        assert!(!context.requirements[0].legal_targets.contains(&Target::Object(self.forbidden)));
        self.calls += 1;
        vec![Target::Object(self.target)]
    }
}
fn settle(game: &mut GameState, queue: &mut TriggerQueue, dm: &mut dyn DecisionMaker) {
    put_triggers_on_stack_with_dm(game, queue, dm).unwrap();
    while !game.stack_is_empty() { resolve_stack_entry_with(game, dm).unwrap(); }
}
fn assert_team_effect(game: &GameState, name: &str, host: ObjectId, target: ObjectId, happened: bool) {
    match name {
        "Aurora Champion" => assert_eq!(game.is_tapped(target), happened),
        "Bull-Rush Bruiser" => assert_eq!(game.current_has_static_ability_id(host, StaticAbilityId::FirstStrike), happened),
        "Sickle Dancer" => {
            assert_eq!(game.calculated_power(host), Some(if happened { 4 } else { 3 }));
            assert_eq!(game.calculated_toughness(host), Some(if happened { 3 } else { 2 }));
        }
        _ => unreachable!(),
    }
}
fn assert_death_reward(game: &GameState, name: &str, recipient: PlayerId, happened: bool) {
    for player in [A, B, C, D] {
        assert_eq!(game.player(player).unwrap().hand.len(),
            usize::from(name == "Dragonfly Swarm" && player == recipient && happened));
        assert_eq!(game.player(player).unwrap().life,
            if name == "Walltop Sentries" && player == recipient && happened { 22 } else { 20 });
    }
}

#[test]
fn full_bodies_round_trip_with_independent_rendering_and_typed_intervening_conditions() {
    assert_eq!(rows().len(), 7);
    for row in rows().iter().filter(|row| row["proposed_complete"] == true) {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name) {
            assert!(!ironsmith::cards::generated_definition_has_unimplemented_content(&definition));
            assert_eq!(definition.card.name, name);
            assert_eq!(definition.card.card_types, vec![CardType::Creature]);
            let (mana_cost, subtypes, printed_pt) = match name {
                "Aurora Champion" => ("{2}{W}", vec![Subtype::Elf, Subtype::Warrior], PowerToughness::fixed(3, 2)),
                "Bull-Rush Bruiser" => ("{3}{R}", vec![Subtype::Minotaur, Subtype::Warrior], PowerToughness::fixed(4, 3)),
                "Sickle Dancer" => ("{2}{B}", vec![Subtype::Human, Subtype::Warrior], PowerToughness::fixed(3, 2)),
                "Dragonfly Swarm" => ("{1}{U}{R}", vec![Subtype::Dragon, Subtype::Insect], PowerToughness::new(PtValue::Star, PtValue::Fixed(3))),
                "Walltop Sentries" => ("{2}{G}", vec![Subtype::Human, Subtype::Soldier, Subtype::Ally], PowerToughness::fixed(2, 3)),
                _ => unreachable!(),
            };
            assert_eq!(definition.card.mana_cost.as_ref().unwrap().to_oracle(), mana_cost, "{name}");
            assert_eq!(definition.card.subtypes, subtypes, "{name}");
            assert_eq!(definition.card.power_toughness, Some(printed_pt), "{name}");
            let gated = definition.abilities.iter().filter_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) if triggered.intervening_if.is_some() => Some(triggered),
                _ => None,
            }).collect::<Vec<_>>();
            assert_eq!(gated.len(), 1, "{name}");
            if TEAM_CARDS.contains(&name) {
                let filter = ObjectFilter::default().with_subtype(Subtype::Warrior)
                    .in_zone(Zone::Battlefield).controlled_by(PlayerFilter::your_team());
                let mut source_filter = filter.clone();
                source_filter.source = true;
                assert_eq!(gated[0].intervening_if, Some(Condition::ValueComparison {
                    left: Value::Count(filter), operator: ValueComparisonOperator::GreaterThan, right: Value::Count(source_filter),
                }));
            } else {
                let Some(Condition::PlayerControls { player, filter }) = &gated[0].intervening_if else { panic!("{name}") };
                assert_eq!(*player, PlayerFilter::You);
                assert_eq!(filter.owner, Some(PlayerFilter::You));
                assert_eq!(filter.zone, Some(Zone::Graveyard));
                assert_eq!(filter.subtypes, vec![Subtype::Lesson]);
            }
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            let expected_trigger = match name {
                "Aurora Champion" => "Whenever this creature attacks, if your team controls another Warrior, tap target creature.",
                "Bull-Rush Bruiser" => "Whenever this creature attacks, if your team controls another Warrior, this creature gains first strike until end of turn.",
                "Sickle Dancer" => "Whenever this creature attacks, if your team controls another Warrior, this creature gets +1/+1 until end of turn.",
                "Dragonfly Swarm" => "When this creature dies, if there is a Lesson card in your graveyard, draw a card.",
                "Walltop Sentries" => "When this creature dies, if there is a Lesson card in your graveyard, you gain 2 life.",
                _ => unreachable!(),
            };
            assert!(rendered.contains(expected_trigger), "{name}: {rendered}");
            if name == "Dragonfly Swarm" {
                assert!(rendered.contains("Flying"), "{rendered}");
                assert!(rendered.to_lowercase().contains("ward {1}"), "{rendered}");
                assert!(rendered.contains("power is equal to the number of noncreature, nonland cards in your graveyard"), "{rendered}");
            } else if name == "Walltop Sentries" {
                assert!(rendered.contains("Reach") && rendered.to_lowercase().contains("deathtouch"), "{rendered}");
            }
        }
    }
}

#[test]
fn team_condition_checks_other_permanents_on_either_teammates_battlefield_at_trigger_time() {
    for name in TEAM_CARDS {
        for definition in definitions(name) {
            for (owner, zone, types, qualifies) in [
                (A, Zone::Battlefield, "Creature — Warrior", true),
                (B, Zone::Battlefield, "Creature — Warrior", true),
                (B, Zone::Battlefield, "Kindred Enchantment — Warrior", true),
                (C, Zone::Battlefield, "Creature — Warrior", false),
                (D, Zone::Battlefield, "Creature — Warrior", false),
                (B, Zone::Graveyard, "Creature — Warrior", false),
                (B, Zone::Battlefield, "Creature — Soldier", false),
            ] {
                let mut game = game();
                let host = source(&mut game, &definition);
                object(&mut game, owner, zone, types);
                let target = object(&mut game, C, Zone::Battlefield, "Creature — Soldier");
                let other = object(&mut game, D, Zone::Battlefield, "Creature — Soldier");
                let land = object(&mut game, C, Zone::Battlefield, "Land");
                let mut dm = Targets { target, forbidden: land, calls: 0 };
                let mut queue = attack(&mut game, host);
                assert_eq!(queue.entries.len(), usize::from(qualifies), "{name}: {owner:?}/{zone:?}/{types}");
                // A condition becoming true later must never create a missing trigger.
                if !qualifies { object(&mut game, B, Zone::Battlefield, "Creature — Warrior"); }
                settle(&mut game, &mut queue, &mut dm);
                assert_team_effect(&game, name, host, target, qualifies);
                assert!(!game.is_tapped(other));
                assert_eq!(game.calculated_power(other), Some(2));
                assert!(!game.current_has_static_ability_id(other, StaticAbilityId::FirstStrike));
                assert_eq!(dm.calls, usize::from(qualifies && *name == "Aurora Champion"));
                if *name != "Aurora Champion" {
                    ironsmith::turn::execute_cleanup_step(&mut game);
                    game.refresh_continuous_state().unwrap();
                    assert_team_effect(&game, name, host, target, false);
                }
            }
            let mut game = game();
            let host = source(&mut game, &definition);
            assert!(attack(&mut game, host).entries.is_empty(), "{name}: source cannot be another Warrior");
        }
    }
}

#[test]
fn team_gate_rechecks_live_qualifier_but_keeps_the_ability_controller_and_exact_source() {
    for name in TEAM_CARDS {
        for definition in definitions(name) {
            for change in ["leave", "phase", "enemy_control", "source_control", "replace_qualifier"] {
                let mut game = game();
                let host = source(&mut game, &definition);
                let qualifier = object(&mut game, B, Zone::Battlefield, "Creature — Warrior");
                let target = object(&mut game, C, Zone::Battlefield, "Creature — Soldier");
                let land = object(&mut game, C, Zone::Battlefield, "Land");
                let mut dm = Targets { target, forbidden: land, calls: 0 };
                let mut queue = attack(&mut game, host);
                assert_eq!(queue.entries.len(), 1);
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
                match change {
                    "phase" => { game.phase_out(qualifier); }
                    "enemy_control" => { game.set_current_controller(qualifier, C).unwrap(); }
                    "source_control" => { game.set_current_controller(host, C).unwrap(); }
                    _ => {
                        game.move_object_by_effect(qualifier, Zone::Exile).unwrap();
                        if change == "replace_qualifier" { object(&mut game, A, Zone::Battlefield, "Creature — Warrior"); }
                    }
                }
                let saved = game.clone();
                for mut resumed in [game, saved] {
                    resolve_stack_entry_with(&mut resumed, &mut dm).unwrap();
                    assert_team_effect(&resumed, name, host, target, matches!(change, "source_control" | "replace_qualifier"));
                }
            }
        }
    }
}

#[test]
fn team_count_includes_tokens_but_a_free_for_all_has_no_implicit_teammate() {
    for definition in definitions("Sickle Dancer") {
        let mut game = game();
        let host = source(&mut game, &definition);
        let warrior = object(&mut game, B, Zone::Battlefield, "Creature — Warrior");
        game.object_mut(warrior).unwrap().kind = ironsmith::object::ObjectKind::Token;
        let mut queue = attack(&mut game, host);
        assert_eq!(queue.entries.len(), 1);
        settle(&mut game, &mut queue, &mut SelectFirstDecisionMaker);
        assert_eq!(game.calculated_power(host), Some(4));

        let mut free_for_all = GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 20);
        free_for_all.turn.active_player = A;
        free_for_all.turn.priority_player = Some(A);
        free_for_all.turn.phase = Phase::Combat;
        free_for_all.turn.step = Some(ironsmith::game_state::Step::DeclareAttackers);
        free_for_all.mark_combat_phase_started();
        let host = source(&mut free_for_all, &definition);
        object(&mut free_for_all, B, Zone::Battlefield, "Creature — Warrior");
        assert!(attack(&mut free_for_all, host).entries.is_empty());
    }
}

#[test]
fn doubled_aurora_triggers_exclude_the_exact_source_even_with_a_targeted_sibling_on_stack() {
    // A real static trigger doubler produces two independent occurrences.
    // While the top resolves, the lower occurrence's target is imported into
    // the event filter context. It must never change what "another" excludes.
    let doubler = compile_to_runtime_definition("Attack trigger doubler",
        "Type: Enchantment\nIf a creature attacking causes a triggered ability of a permanent you control to trigger, that ability triggers an additional time.",
        false).unwrap();
    for definition in definitions("Aurora Champion") {
        for change in [
            "remove_qualifier", "qualifier_to_opponent", "source_to_opponent_remove_qualifier",
            "source_to_opponent_keep_qualifier", "noncreature_qualifier", "blink_source", "source_leaves",
        ] {
            let mut game = game();
            let host = source(&mut game, &definition);
            let stable = game.object(host).unwrap().stable_id;
            game.create_object_from_definition(&doubler, A, Zone::Battlefield);
            let qualifier = object(&mut game, B, Zone::Battlefield,
                if change == "noncreature_qualifier" { "Kindred Enchantment — Warrior" } else { "Creature — Warrior" });
            let target = object(&mut game, C, Zone::Battlefield, "Creature — Soldier");
            let land = object(&mut game, C, Zone::Battlefield, "Land");
            let mut dm = Targets { target, forbidden: land, calls: 0 };
            let mut queue = attack(&mut game, host);
            assert_eq!(queue.entries.len(), 2, "the actual doubler must create both triggers");
            put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm).unwrap();
            assert_eq!(game.stack.len(), 2);
            assert_eq!(dm.calls, 2, "each occurrence announces its own target");
            assert!(game.stack.iter().all(|entry|
                entry.object_id == host && entry.controller == A
                    && entry.targets == vec![Target::Object(target)]));

            match change {
                "qualifier_to_opponent" => { game.set_current_controller(qualifier, C).unwrap(); }
                "source_to_opponent_keep_qualifier" => { game.set_current_controller(host, C).unwrap(); }
                "noncreature_qualifier" => {}
                _ => {
                    game.move_object_by_effect(qualifier, Zone::Exile).unwrap();
                    match change {
                        "source_to_opponent_remove_qualifier" => { game.set_current_controller(host, C).unwrap(); }
                        "source_leaves" => { game.move_object_by_effect(host, Zone::Exile).unwrap(); }
                        "blink_source" => {
                            let exiled = game.move_object_by_effect(host, Zone::Exile).unwrap();
                            let returned = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
                            assert_ne!(returned, host);
                            assert_eq!(game.object(returned).unwrap().stable_id, stable);
                        }
                        _ => {}
                    }
                }
            }
            let qualifies = matches!(change,
                "source_to_opponent_keep_qualifier" | "noncreature_qualifier" | "blink_source");
            let saved = game.clone();
            for mut resumed in [game, saved] {
                resolve_stack_entry_with(&mut resumed, &mut dm).unwrap();
                assert_eq!(resumed.stack.len(), 1, "the targeted sibling remains during this check");
                assert_eq!(resumed.is_tapped(target), qualifies, "top trigger: {change}");
                resolve_stack_entry_with(&mut resumed, &mut dm).unwrap();
                assert!(resumed.stack_is_empty());
                assert_eq!(resumed.is_tapped(target), qualifies, "lower trigger: {change}");
                assert!(!resumed.is_tapped(land));
            }
        }
    }
}

#[test]
fn death_lesson_gate_checks_owned_graveyard_at_death_and_only_rewards_last_controller() {
    for name in LESSON_CARDS {
        for definition in definitions(name) {
            for (owner, zone, types, qualifies) in [
                (A, Zone::Graveyard, "Sorcery — Lesson", true),
                (B, Zone::Graveyard, "Sorcery — Lesson", false),
                (C, Zone::Graveyard, "Sorcery — Lesson", false),
                (D, Zone::Graveyard, "Sorcery — Lesson", false),
                (A, Zone::Exile, "Sorcery — Lesson", false),
                (A, Zone::Graveyard, "Sorcery", false),
            ] {
                let mut game = game();
                let host = source(&mut game, &definition);
                object(&mut game, owner, zone, types);
                let mut queue = die(&mut game, host);
                assert_eq!(queue.entries.len(), usize::from(qualifies), "{name}: {owner:?}/{zone:?}/{types}");
                if !qualifies { object(&mut game, A, Zone::Graveyard, "Sorcery — Lesson"); }
                settle(&mut game, &mut queue, &mut SelectFirstDecisionMaker);
                assert_death_reward(&game, name, A, qualifies);
            }
            for lesson_owner in [A, B] {
                let mut game = game();
                let host = source(&mut game, &definition);
                game.set_current_controller(host, B).unwrap();
                object(&mut game, lesson_owner, Zone::Graveyard, "Sorcery — Lesson");
                let mut queue = die(&mut game, host);
                assert_eq!(queue.entries.len(), usize::from(lesson_owner == B));
                settle(&mut game, &mut queue, &mut SelectFirstDecisionMaker);
                assert_death_reward(&game, name, B, lesson_owner == B);
            }
        }
    }
}

#[test]
fn death_lesson_gate_rechecks_at_resolution_and_does_not_latch_the_original_card() {
    for name in LESSON_CARDS {
        for definition in definitions(name) {
            for replacement in [false, true] {
                let mut game = game();
                let host = source(&mut game, &definition);
                let lesson = object(&mut game, A, Zone::Graveyard, "Sorcery — Lesson");
                let mut queue = die(&mut game, host);
                assert_eq!(queue.entries.len(), 1);
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
                game.move_object_by_effect(lesson, Zone::Exile).unwrap();
                if replacement { object(&mut game, A, Zone::Graveyard, "Instant — Lesson"); }
                let saved = game.clone();
                for mut resumed in [game, saved] {
                    resolve_stack_entry_with(&mut resumed, &mut SelectFirstDecisionMaker).unwrap();
                    assert_death_reward(&resumed, name, A, replacement);
                }
            }
        }
    }
}

#[test]
fn companion_lines_keep_dragonfly_characteristic_power_and_sentries_keywords() {
    for definition in definitions("Dragonfly Swarm") {
        let mut game = game();
        let host = source(&mut game, &definition);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Flying));
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Ward));
        assert_eq!(game.calculated_power(host), Some(0));
        object(&mut game, A, Zone::Graveyard, "Sorcery — Lesson");
        object(&mut game, A, Zone::Graveyard, "Artifact");
        object(&mut game, A, Zone::Graveyard, "Artifact Creature");
        object(&mut game, A, Zone::Graveyard, "Land");
        object(&mut game, A, Zone::Exile, "Instant");
        object(&mut game, B, Zone::Graveyard, "Instant");
        assert_eq!(game.calculated_power(host), Some(2));
        assert_eq!(game.calculated_toughness(host), Some(3));
        game.set_current_controller(host, B).unwrap();
        assert_eq!(game.calculated_power(host), Some(1));
        game.set_current_controller(host, A).unwrap();
        assert_eq!(game.calculated_power(host), Some(2));
    }
    for definition in definitions("Walltop Sentries") {
        let mut game = game();
        let host = source(&mut game, &definition);
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Reach));
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Deathtouch));
    }
}

struct WardAnswers {
    target: ObjectId,
    caster: PlayerId,
    pay: bool,
    prompts: usize,
}
impl DecisionMaker for WardAnswers {
    fn decide_targets(&mut self, _: &GameState, context: &TargetsContext) -> Vec<Target> {
        assert_eq!(context.player, self.caster);
        assert!(context.requirements[0].legal_targets.contains(&Target::Object(self.target)));
        vec![Target::Object(self.target)]
    }
    fn decide_boolean(&mut self, _: &GameState, context: &ironsmith::decisions::context::BooleanContext) -> bool {
        assert_eq!(context.player, self.caster, "the targeting spell's controller pays ward");
        self.prompts += 1;
        self.pay
    }
    fn decide_mana_payment(&mut self, _: &GameState,
        context: &ironsmith::decisions::context::ManaPaymentContext) -> ironsmith::mana_payment::ManaPaymentResponse {
        assert_eq!(context.player, self.caster);
        ironsmith::mana_payment::ManaPaymentResponse::Confirm {
            plan_id: context.plan.id, request_hash: context.plan.request_hash,
        }
    }
}

#[test]
fn dragonfly_full_body_retains_real_ward_payment_and_teammate_exclusion() {
    use ironsmith::alternative_cast::CastingMethod;
    use ironsmith::decision::LegalAction;
    use ironsmith::game_loop::{PriorityLoopState, PriorityResponse,
        apply_decision_context_with_dm, apply_priority_response_with_dm};
    use ironsmith::mana::ManaSymbol;
    for definition in definitions("Dragonfly Swarm") {
        for caster in [A, B, C, D] {
            for pay in [false, true] {
                let mut game = game();
                game.turn.active_player = caster;
                game.turn.priority_player = Some(caster);
                game.turn.phase = Phase::FirstMain;
                game.turn.step = None;
                let host = source(&mut game, &definition);
                let spell = compile_to_runtime_definition("Ward targeting resource",
                    "Mana cost: {U}\nType: Instant\nTap target creature.", false).unwrap();
                let spell = game.create_object_from_definition(&spell, caster, Zone::Hand);
                game.player_mut(caster).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
                game.player_mut(caster).unwrap().mana_pool.add(ManaSymbol::Colorless, 1);
                let mut answers = WardAnswers { target: host, caster, pay, prompts: 0 };
                let mut state = PriorityLoopState::new(4);
                let mut queue = TriggerQueue::new();
                let mut progress = apply_priority_response_with_dm(&mut game, &mut queue, &mut state,
                    &PriorityResponse::PriorityAction(LegalAction::CastSpell {
                        spell_id: spell, from_zone: Zone::Hand, casting_method: CastingMethod::Normal,
                    }), &mut answers).unwrap();
                for _ in 0..32 {
                    if state.pending_cast.is_none() && state.pending_method_selection.is_none() { break; }
                    let ironsmith::GameProgress::NeedsDecisionCtx(context) = progress else { break; };
                    progress = apply_decision_context_with_dm(&mut game, &mut queue, &mut state,
                        &context, &mut answers).unwrap();
                }
                assert!(state.pending_cast.is_none() && state.pending_method_selection.is_none());
                put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut answers).unwrap();
                let opponent = matches!(caster, C | D);
                assert_eq!(game.stack.len(), if opponent { 2 } else { 1 });
                if opponent {
                    resolve_stack_entry_with(&mut game, &mut answers).unwrap();
                    assert_eq!(game.stack.len(), usize::from(pay));
                    assert!(!game.is_tapped(host));
                }
                if !game.stack_is_empty() { resolve_stack_entry_with(&mut game, &mut answers).unwrap(); }
                assert_eq!(game.is_tapped(host), !opponent || pay);
                assert_eq!(answers.prompts, usize::from(opponent));
                assert_eq!(game.player(caster).unwrap().mana_pool.total(), if opponent && pay { 0 } else { 1 });
            }
        }
    }
}

#[test]
fn unsupported_predicate_neighbors_remain_errors_in_full_triggered_lines() {
    for text in [
        "Whenever this creature attacks, if your team controls another Warrior with an unknown qualification, draw a card.",
        "Whenever this creature attacks, if your team controls another Warrior and a missing predicate, draw a card.",
        "Whenever this creature attacks, if your team controls another Warrior or a missing predicate, draw a card.",
        "When this creature dies, if there's a Lesson card in your opponent's graveyard, draw a card.",
        "When this creature dies, if there's a Lesson card in your graveyard with an unknown qualification, draw a card.",
    ] {
        assert!(compile_to_artifact("Unsupported neighbor", format!("Type: Creature\nPower/Toughness: 1/1\n{text}"), false).is_err(), "{text}");
    }
}

// Regression controls use the complete retained bodies and their actual metadata.
// The only edited surface is the existential head in the death-trigger condition.
#[test]
fn complete_lesson_bodies_route_contractions_and_recheck_exact_owned_graveyard() {
    for name in LESSON_CARDS {
        let row = rows().into_iter().find(|row| row["name"] == *name).unwrap();
        for head in ["there's", "there’s", "there is"] {
            let oracle = row["oracle_text"].as_str().unwrap().replace("there's", head);
            for definition in definitions_with_oracle(name, &oracle) {
                let gates = definition.abilities.iter().filter_map(|ability| match &ability.kind {
                    AbilityKind::Triggered(triggered) => triggered.intervening_if.as_ref(),
                    _ => None,
                }).collect::<Vec<_>>();
                let expected = Condition::PlayerControls {
                    player: PlayerFilter::You,
                    filter: ObjectFilter::default().with_subtype(Subtype::Lesson)
                        .in_zone(Zone::Graveyard).owned_by(PlayerFilter::You),
                };
                assert_eq!(gates, vec![&expected], "{name}: {head}");
                for (at_death, at_resolution) in [(false, true), (true, false), (true, true)] {
                    let mut game = game();
                    let host = source(&mut game, &definition);
                    // The controller's graveyard matters even when the source is stolen.
                    game.set_current_controller(host, B).unwrap();
                    object(&mut game, A, Zone::Graveyard, "Sorcery — Lesson");
                    object(&mut game, B, Zone::Hand, "Sorcery — Lesson");
                    object(&mut game, B, Zone::Exile, "Instant — Lesson");
                    object(&mut game, B, Zone::Graveyard, "Sorcery");
                    let lesson = at_death.then(|| object(&mut game, B, Zone::Graveyard, "Sorcery — Lesson"));
                    let before = [A, B, C, D].into_iter().map(|player| {
                        let state = game.player(player).unwrap();
                        (player, state.hand.clone(), state.library.clone(), state.life)
                    }).collect::<Vec<_>>();
                    let draw_source = game.player(B).unwrap().library.last().copied().unwrap();
                    let expected_draw = game.object(draw_source).unwrap().stable_id;
                    if *name == "Dragonfly Swarm" {
                        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Flying));
                        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Ward));
                        assert_eq!(game.calculated_power(host), Some(1 + i32::from(at_death)));
                        assert_eq!(game.calculated_toughness(host), Some(3));
                    } else {
                        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Reach));
                        assert!(game.current_has_static_ability_id(host, StaticAbilityId::Deathtouch));
                    }
                    let mut queue = die(&mut game, host);
                    assert_eq!(queue.entries.len(), usize::from(at_death), "{name}: {head}");
                    put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut SelectFirstDecisionMaker).unwrap();
                    if let Some(lesson) = lesson {
                        game.move_object_by_effect(lesson, Zone::Exile).unwrap();
                    }
                    if at_resolution { object(&mut game, B, Zone::Graveyard, "Instant — Lesson"); }
                    while !game.stack_is_empty() {
                        resolve_stack_entry_with(&mut game, &mut SelectFirstDecisionMaker).unwrap();
                    }
                    let happened = at_death && at_resolution;
                    for (player, hand, mut library, life) in before {
                        let state = game.player(player).unwrap();
                        let drew = *name == "Dragonfly Swarm" && player == B && happened;
                        assert_eq!(state.hand.len(), hand.len() + usize::from(drew));
                        assert!(hand.iter().all(|id| state.hand.contains(id)), "hand decoy changed");
                        let added = state.hand.iter().copied()
                            .filter(|id| !hand.contains(id)).collect::<Vec<_>>();
                        assert_eq!(added.len(), usize::from(drew));
                        if drew {
                            assert_eq!(game.object(added[0]).unwrap().stable_id, expected_draw);
                            library.pop();
                        }
                        assert_eq!(state.library, library);
                        assert_eq!(state.life, life + if *name == "Walltop Sentries" && player == B && happened { 2 } else { 0 });
                    }
                }
            }
            for suffix in [" with an unknown qualification", " {2}", " and a missing predicate"] {
                let invalid = oracle.replace("in your graveyard,", &format!("in your graveyard{suffix},"));
                let text = format!("Mana cost: {}\nType: {}\nPower/Toughness: {}/{}\n{}",
                    row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(),
                    row["power"].as_str().unwrap(), row["toughness"].as_str().unwrap(), invalid);
                assert!(compile_to_runtime_definition(name, &text, false).is_err(), "{name}: {head}: {suffix}");
                assert!(compile_to_artifact(name, &text, false).is_err(), "{name}: {head}: {suffix}");
            }
        }
    }
}

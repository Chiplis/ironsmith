//! Authored source-only scenarios. Compilation and execution are deferred.
use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::card::PowerToughness;
use ironsmith::effects::{CreateTokenEffect, EffectContext, EffectExecutor};
use ironsmith::object::CounterType;
use ironsmith::static_abilities::StaticAbilityId;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::compile_to_artifact;
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId(0);
const B: PlayerId = PlayerId(1);
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = ironsmith_compiler::parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap();
    assert!(!loss.is_lossy(), "{}", loss.reasons_text()); artifact.validate().unwrap();
    let encoded = artifact.to_json().unwrap();
    assert!(std::str::from_utf8(&encoded).unwrap().contains("SourceCaseSolved"));
    let restored = CompiledCardArtifact::from_json(&encoded).unwrap();
    assert_eq!(artifact, restored);
    [direct, materialize_artifact(&restored).unwrap()]
}
fn creature() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Case guard creature")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2)).build()
}
fn create(game: &mut GameState, source: ObjectId, controller: PlayerId) -> usize {
    CreateTokenEffect::you(creature(), 1)
        .execute(game, &mut EffectContext::new_default(source, controller)).unwrap()
        .result_objects().unwrap().len()
}
#[test]
fn solved_replacement_uses_designation_current_controller_and_current_incarnation() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../../fixtures/token_template_follow_ups.json.fixture")).unwrap();
    let row = rows.iter().find(|row| row["name"] == "Case of the Pilfered Proof").unwrap();
    let text = format!("Mana cost: {}\nType: {}\n{}", row["mana_cost"].as_str().unwrap(), row["type_line"].as_str().unwrap(), row["oracle_text"].as_str().unwrap());
    for definition in definitions("Case of the Pilfered Proof", &text) {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.add_counters(host, CounterType::Level, 99);
        assert!(!game.is_case_solved(host));
        assert_eq!(create(&mut game, host, A), 1, "level counters never solve a Case");
        assert!(game.solve_case(host));
        assert_eq!(create(&mut game, host, A), 2);
        game.set_current_controller(host, B).unwrap();
        assert_eq!(create(&mut game, host, A), 1);
        assert_eq!(create(&mut game, host, B), 2);
        game.phase_out(host);
        assert!(game.is_case_solved(host));
        assert_eq!(create(&mut game, host, B), 1);
        game.phase_in(host);
        assert_eq!(create(&mut game, host, B), 2);
        let exiled = game.move_object_by_effect(host, Zone::Exile).unwrap();
        let returned = game.move_object_by_effect(exiled, Zone::Battlefield).unwrap();
        assert_ne!(returned, host);
        assert!(!game.is_case_solved(returned));
        assert_eq!(create(&mut game, returned, A), 1);
    }
}
#[test]
fn solving_invalidates_other_permanents_characteristics_and_gates_every_static_member() {
    for definition in definitions("Case anthem gate probe", "Type: Enchantment — Case\nSolved — Creatures you control get +1/+1 and have vigilance.") {
        let lines = ironsmith_text::compiled_text_lines(&definition).join("\n");
        assert_eq!(lines.matches("Solved —").count(), 1, "{lines}");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = game.create_object_from_definition(&creature(), A, Zone::Battlefield);
        let other = game.create_object_from_definition(&creature(), B, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(2));
        assert!(!game.current_has_static_ability_id(own, StaticAbilityId::Vigilance));
        game.solve_case(host); game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(3));
        assert!(game.current_has_static_ability_id(own, StaticAbilityId::Vigilance));
        assert_eq!(game.current_power(other), Some(2));
        game.phase_out(host); game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(2));
        assert!(!game.current_has_static_ability_id(own, StaticAbilityId::Vigilance));
        game.phase_in(host); game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(3));
    }
}
#[test]
fn solved_visibility_uses_live_static_gate_and_keeps_its_authored_surface() {
    for definition in definitions("Case private top gate probe", "Type: Enchantment — Case\nSolved — You may look at the top card of your library any time.") {
        let lines = ironsmith_text::compiled_text_lines(&definition).join("\n");
        assert!(lines.contains("Solved —"), "{lines}");
        assert!(!lines.contains("As long as this Case is solved"), "{lines}");
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert!(!game.current_has_static_ability_id(host, StaticAbilityId::LookAtTopCardOfLibrary));
        game.solve_case(host); game.refresh_continuous_state().unwrap();
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::LookAtTopCardOfLibrary));
        game.phase_out(host); game.refresh_continuous_state().unwrap();
        assert!(!game.current_has_static_ability_id(host, StaticAbilityId::LookAtTopCardOfLibrary));
        game.phase_in(host); game.refresh_continuous_state().unwrap();
        assert!(game.current_has_static_ability_id(host, StaticAbilityId::LookAtTopCardOfLibrary));
    }
}

#[test]
fn solved_scope_does_not_replace_an_inner_turn_condition() {
    for definition in definitions("Case nested guard probe", "Type: Enchantment — Case\nSolved — As long as it's your turn, creatures you control get +1/+1 and have vigilance.") {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = A;
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let own = game.create_object_from_definition(&creature(), A, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(2));
        game.solve_case(host); game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(3));
        assert!(game.current_has_static_ability_id(own, StaticAbilityId::Vigilance));
        game.next_turn(); assert_eq!(game.turn.active_player, B);
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_power(own), Some(2));
        assert!(!game.current_has_static_ability_id(own, StaticAbilityId::Vigilance));
    }
}

fn nested_native_case(leaf: ironsmith::static_abilities::CompiledStaticAbility) -> CardDefinition {
    let model = leaf
        .with_labeled_condition(ironsmith::ConditionExpr::YourTurn, "During your turn")
        .with_labeled_condition(ironsmith::ConditionExpr::SourceCaseSolved, "Solved");
    CardDefinitionBuilder::new(CardId::new(), "Nested Case native probe")
        .card_types(vec![CardType::Enchantment])
        .with_ability(ironsmith::ability::Ability::static_ability(
            ironsmith::static_abilities::StaticAbility::from_model(model))).build()
}
#[test]
fn nested_solved_ability_loss_requires_both_predicates() {
    use ironsmith::static_abilities::{CompiledStaticAbility, StaticAbility};
    let definition = nested_native_case(CompiledStaticAbility::remove_ability(
        ironsmith::target::ObjectFilter::creature().you_control(), CompiledStaticAbility::flying()));
    let flier = CardDefinitionBuilder::new(CardId::new(), "Nested guard flier")
        .card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(ironsmith::ability::Ability::static_ability(StaticAbility::flying())).build();
    for solved in [false, true] { for own_turn in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = if own_turn { A } else { B };
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let affected = game.create_object_from_definition(&flier, A, Zone::Battlefield);
        if solved { game.solve_case(host); }
        game.refresh_continuous_state().unwrap();
        assert_eq!(game.current_has_static_ability_id(affected, StaticAbilityId::Flying), !(solved && own_turn));
    } }
}
#[test]
fn nested_solved_damage_replacement_requires_both_predicates() {
    use ironsmith::static_abilities::CompiledStaticAbility;
    use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    let definition = nested_native_case(CompiledStaticAbility::modify_damage_amount_replacement(
        ObjectFilter::default().you_control(), Some(PlayerFilter::Opponent), None, 1,
        "Your sources deal one additional damage to opponents"));
    for solved in [false, true] { for own_turn in [false, true] {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        game.turn.active_player = if own_turn { A } else { B };
        let host = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        if solved { game.solve_case(host); }
        ironsmith::effects::DealDamageEffect::new(2, ChooseSpec::Player(PlayerFilter::Specific(B)))
            .execute(&mut game, &mut EffectContext::new_default(host, A)).unwrap();
        assert_eq!(game.player(B).unwrap().life, 20 - if solved && own_turn { 3 } else { 2 });
    } }
}

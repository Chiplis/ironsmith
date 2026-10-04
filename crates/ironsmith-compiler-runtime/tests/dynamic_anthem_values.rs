//! Metadata -> typed static values -> artifact -> live characteristic coverage.
//! Authored but unrun under the implementation-first campaign workflow.
use ironsmith::ability::AbilityKind;
use ironsmith::cards::CardDefinition;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext as ExecutionContext, execute_effect};
use ironsmith::object::AttachmentTarget;
use ironsmith::{GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler::parse_loss;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use serde_json::Value;

fn fixtures() -> Vec<Value> {
    serde_json::from_str(include_str!(
        "../../../fixtures/dynamic_anthem_values.json.fixture"
    ))
    .unwrap()
}
fn fixture(name: &str) -> Value {
    fixtures()
        .into_iter()
        .find(|row| row["name"] == name)
        .unwrap()
}
fn source(row: &Value) -> String {
    let mut lines = vec![
        format!("Mana cost: {}", row["mana_cost"].as_str().unwrap()),
        format!("Type: {}", row["type_line"].as_str().unwrap()),
    ];
    if let (Some(power), Some(toughness)) = (row["power"].as_str(), row["toughness"].as_str()) {
        lines.push(format!("Power/Toughness: {power}/{toughness}"));
    }
    lines.push(row["oracle_text"].as_str().unwrap().to_owned());
    lines.join("\n")
}
fn definitions(name: &str, text: &str) -> [CardDefinition; 2] {
    let (result, loss) = parse_loss::capture(|| compile_to_artifact(name, text, false));
    let (artifact, direct) = result.unwrap_or_else(|error| panic!("{name}: {error}"));
    assert!(!loss.is_lossy(), "{name}: {}", loss.reasons_text());
    let decoded = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(artifact, decoded);
    let restored =
        ironsmith_runtime_catalog::artifact_materializer::materialize_artifact(&decoded).unwrap();
    [direct, restored]
}
fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20)
}
fn vanilla(name: &str, power: i32, toughness: i32) -> CardDefinition {
    compile_to_runtime_definition(
        name,
        format!("Mana cost: {{2}}\nType: Creature — Human\nPower/Toughness: {power}/{toughness}"),
        false,
    )
    .unwrap()
}
fn apply(game: &mut GameState, source: ObjectId, controller: PlayerId, effect: Effect) {
    let mut dm = SelectFirstDecisionMaker;
    let mut context = ExecutionContext::new(source, controller, &mut dm);
    execute_effect(game, &effect, &mut context).unwrap();
}
fn library(game: &mut GameState, owner: PlayerId, count: usize) {
    let card = vanilla("Library resource", 1, 1);
    for _ in 0..count {
        game.create_object_from_definition(&card, owner, Zone::Library);
    }
}
fn pt(game: &GameState, object: ObjectId) -> (i32, i32) {
    (
        game.current_power(object).unwrap(),
        game.current_toughness(object).unwrap(),
    )
}

#[test]
fn nine_exact_inputs_preserve_metadata_dynamic_nodes_signs_and_artifacts() {
    let rows = fixtures();
    assert_eq!(rows.len(), 44);
    let selected = rows
        .iter()
        .filter(|row| row["repair_group"] == "game_state_anthem_value")
        .collect::<Vec<_>>();
    assert_eq!(selected.len(), 9);
    for row in selected {
        let name = row["name"].as_str().unwrap();
        for definition in definitions(name, &source(row)) {
            assert_eq!(
                definition.card.mana_cost.as_ref().unwrap().to_oracle(),
                row["mana_cost"].as_str().unwrap()
            );
            assert!(!definition.card.card_types.is_empty());
            if let Some(power) = row["power"].as_str() {
                let value = definition.card.power_toughness.unwrap();
                assert_eq!(value.power.to_string(), power);
                assert_eq!(
                    value.toughness.to_string(),
                    row["toughness"].as_str().unwrap()
                );
            }
            let dynamic = definition.abilities.iter().any(|ability| {
                let AbilityKind::Static(ability) = &ability.kind else {
                    return false;
                };
                ability.compiled_model().is_some_and(|model| {
                    matches!(&model.payload,
                    ironsmith_core::StaticAbilityPayload::Anthem(anthem)
                    if matches!(anthem.power, ironsmith_core::AnthemValue::Dynamic(_)))
                })
            });
            assert!(dynamic, "{name}: preserve the executable dynamic basis");
            let rendered = ironsmith_text::compiled_text_lines(&definition).join("\n");
            if row["oracle_text"].as_str().unwrap().contains("-X/") {
                assert!(rendered.contains("-X/"), "{name}: {rendered}");
                assert!(
                    !rendered.contains("-1 times"),
                    "sign must not be duplicated: {rendered}"
                );
                let mut rendered_row = row.clone();
                rendered_row["oracle_text"] = Value::String(rendered.clone());
                let reparsed =
                    compile_to_runtime_definition(name, source(&rendered_row), false).unwrap();
                assert!(
                    ironsmith_text::compiled_text_lines(&reparsed)
                        .join("\n")
                        .contains("-X/"),
                    "signed where-X rendering must round-trip"
                );
            }
        }
    }
}

#[test]
fn negative_life_basis_uses_current_source_controller_and_live_life_actions() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for name in ["Death's Shadow", "The Last Ride"] {
        for definition in definitions(name, &source(&fixture(name))) {
            let mut game = game();
            game.lose_life(alice, 15);
            game.lose_life(bob, 13);
            let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            assert_eq!(pt(&game, host), (8, 8));
            apply(&mut game, host, alice, Effect::gain_life(2));
            assert_eq!(pt(&game, host), (6, 6));
            apply(&mut game, host, alice, Effect::lose_life(4));
            assert_eq!(pt(&game, host), (10, 10));
            game.set_current_controller(host, bob).unwrap();
            assert_eq!(
                pt(&game, host),
                (6, 6),
                "new controller's seven life, not owner's three"
            );
            apply(&mut game, host, alice, Effect::gain_life(3));
            assert_eq!(
                pt(&game, host),
                (6, 6),
                "former controller no longer determines X"
            );
            apply(&mut game, host, bob, Effect::gain_life(2));
            assert_eq!(pt(&game, host), (4, 4));
        }
    }
}

#[test]
fn hand_size_basis_tracks_real_draw_discard_and_aura_controller_not_recipient() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    for name in ["Kagemaro's Clutch", "Meishin, the Mind Cage"] {
        for definition in definitions(name, &source(&fixture(name))) {
            let mut game = game();
            let host = game.create_object_from_definition(
                &vanilla("Host", 10, 10),
                bob,
                Zone::Battlefield,
            );
            let other = game.create_object_from_definition(
                &vanilla("Other", 10, 10),
                charlie,
                Zone::Battlefield,
            );
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            if name == "Kagemaro's Clutch" {
                game.attach_object_to_target(source, AttachmentTarget::Object(host));
            }
            library(&mut game, alice, 5);
            library(&mut game, bob, 5);
            assert_eq!(pt(&game, host), (10, 10));
            apply(&mut game, source, alice, Effect::draw(2));
            let toughness = if name == "Kagemaro's Clutch" { 8 } else { 10 };
            assert_eq!(pt(&game, host), (8, toughness));
            assert_eq!(
                pt(&game, other),
                if name == "Kagemaro's Clutch" {
                    (10, 10)
                } else {
                    (8, 10)
                }
            );
            apply(&mut game, source, alice, Effect::discard(1));
            assert_eq!(game.player(alice).unwrap().hand.len(), 1);
            assert_eq!(game.current_power(host), Some(9));
            apply(&mut game, source, bob, Effect::draw(3));
            assert_eq!(
                game.current_power(host),
                Some(9),
                "recipient hand is irrelevant"
            );
            game.set_current_controller(source, bob).unwrap();
            assert_eq!(game.current_power(host), Some(7));
            game.set_current_controller(host, charlie).unwrap();
            assert_eq!(
                game.current_power(host),
                Some(7),
                "changing only the recipient controller does not rebind 'your hand'"
            );
            game.move_object_by_game_rule(source, Zone::Graveyard)
                .unwrap();
            assert_eq!(pt(&game, host), (10, 10));
        }
    }
}

#[test]
fn greatest_graveyard_power_obeys_owner_scope_and_recounts_after_zone_changes() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for (name, all_graveyards, base_power) in [
        ("Carrion Grub", false, 0),
        ("Coram, the Undertaker", true, 0),
    ] {
        for definition in definitions(name, &source(&fixture(name))) {
            let mut game = game();
            let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            assert_eq!(game.current_power(host), Some(base_power));
            let own = game.create_object_from_definition(
                &vanilla("Own grave candidate", 4, 4),
                alice,
                Zone::Hand,
            );
            let enemy = game.create_object_from_definition(
                &vanilla("Opponent grave candidate", 7, 7),
                bob,
                Zone::Hand,
            );
            let own = game.move_object_by_effect(own, Zone::Graveyard).unwrap();
            assert_eq!(game.current_power(host), Some(base_power + 4));
            let enemy = game.move_object_by_effect(enemy, Zone::Graveyard).unwrap();
            assert_eq!(
                game.current_power(host),
                Some(base_power + if all_graveyards { 7 } else { 4 })
            );
            game.move_object_by_effect(own, Zone::Exile).unwrap();
            assert_eq!(
                game.current_power(host),
                Some(base_power + if all_graveyards { 7 } else { 0 })
            );
            game.set_current_controller(host, bob).unwrap();
            assert_eq!(game.current_power(host), Some(base_power + 7));
            game.move_object_by_effect(enemy, Zone::Library).unwrap();
            assert_eq!(game.current_power(host), Some(base_power));
        }
    }
}

#[test]
fn greven_counts_life_lost_not_net_life_change_and_resets_at_next_turn() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions(
        "Greven, Predator Captain",
        &source(&fixture("Greven, Predator Captain")),
    ) {
        let mut game = game();
        let host = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert_eq!(pt(&game, host), (5, 5));
        apply(&mut game, host, alice, Effect::lose_life(3));
        assert_eq!(pt(&game, host), (8, 5));
        apply(&mut game, host, alice, Effect::gain_life(2));
        assert_eq!(pt(&game, host), (8, 5));
        game.set_current_controller(host, bob).unwrap();
        assert_eq!(pt(&game, host), (5, 5));
        apply(&mut game, host, bob, Effect::lose_life(4));
        assert_eq!(pt(&game, host), (9, 5));
        game.next_turn();
        assert_eq!(pt(&game, host), (5, 5));
    }
}

#[test]
fn cards_drawn_history_is_not_current_hand_size_and_follows_source_control() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions(
        "Knowledge Is Power",
        &source(&fixture("Knowledge Is Power")),
    ) {
        let mut game = game();
        let host = game.create_object_from_definition(
            &vanilla("Own recipient", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let opposing = game.create_object_from_definition(
            &vanilla("Opponent recipient", 2, 2),
            bob,
            Zone::Battlefield,
        );
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        library(&mut game, alice, 5);
        library(&mut game, bob, 5);
        assert_eq!(pt(&game, host), (2, 2));
        apply(&mut game, source, alice, Effect::draw(2));
        assert_eq!(pt(&game, host), (4, 4));
        apply(&mut game, source, alice, Effect::discard(1));
        assert_eq!(pt(&game, host), (4, 4));
        apply(&mut game, source, bob, Effect::draw(3));
        assert_eq!(pt(&game, host), (4, 4));
        assert_eq!(pt(&game, opposing), (2, 2));
        game.set_current_controller(source, bob).unwrap();
        assert_eq!(pt(&game, host), (2, 2));
        assert_eq!(pt(&game, opposing), (5, 5));
        game.next_turn();
        assert_eq!(pt(&game, opposing), (2, 2));
    }
}

#[test]
fn entered_creature_history_counts_events_not_current_population_and_resets() {
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for definition in definitions("Kinbinding", &source(&fixture("Kinbinding"))) {
        let mut game = game();
        let host = game.create_object_from_definition(
            &vanilla("Own recipient", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let opposing = game.create_object_from_definition(
            &vanilla("Opponent recipient", 2, 2),
            bob,
            Zone::Battlefield,
        );
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert_eq!(pt(&game, host), (2, 2));
        let own = game.create_object_from_definition(
            &vanilla("Entering creature", 1, 1),
            alice,
            Zone::Hand,
        );
        let own = game.move_object_by_effect(own, Zone::Battlefield).unwrap();
        assert_eq!(pt(&game, host), (3, 3));
        let own = game.move_object_by_effect(own, Zone::Graveyard).unwrap();
        assert_eq!(pt(&game, host), (3, 3));
        game.move_object_by_effect(own, Zone::Battlefield).unwrap();
        assert_eq!(pt(&game, host), (4, 4));
        let enemy = game.create_object_from_definition(
            &vanilla("Other entering creature", 1, 1),
            bob,
            Zone::Hand,
        );
        game.move_object_by_effect(enemy, Zone::Battlefield)
            .unwrap();
        assert_eq!(pt(&game, host), (4, 4));
        game.set_current_controller(source, bob).unwrap();
        assert_eq!(pt(&game, host), (2, 2));
        assert_eq!(pt(&game, opposing), (3, 3));
        game.next_turn();
        assert_eq!(pt(&game, opposing), (2, 2));
    }
}

#[test]
fn publishing_a_real_draw_invalidates_characteristics_warmed_during_its_reveal() {
    use ironsmith::ability::Ability;
    use ironsmith::decision::DecisionMaker;
    use ironsmith::decisions::context::ViewCardsContext;
    use ironsmith::static_abilities::StaticAbility;
    struct ObserveReveal {
        recipient: ObjectId,
        observed: Vec<i32>,
    }
    impl DecisionMaker for ObserveReveal {
        fn view_cards(
            &mut self,
            game: &GameState,
            _viewer: PlayerId,
            _cards: &[ObjectId],
            _context: &ViewCardsContext,
        ) {
            self.observed
                .push(game.current_power(self.recipient).unwrap());
        }
    }
    let alice = PlayerId::from_index(0);
    for mut definition in definitions(
        "Knowledge Is Power",
        &source(&fixture("Knowledge Is Power")),
    ) {
        definition.abilities.push(Ability::static_ability(
            StaticAbility::reveal_first_card_you_draw_each_turn(false, false),
        ));
        let mut game = game();
        let recipient = game.create_object_from_definition(
            &vanilla("History recipient", 2, 2),
            alice,
            Zone::Battlefield,
        );
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        library(&mut game, alice, 2);
        assert_eq!(game.current_power(recipient), Some(2));
        let mut decisions = ObserveReveal {
            recipient,
            observed: Vec::new(),
        };
        let mut context = ExecutionContext::new(source, alice, &mut decisions);
        execute_effect(&mut game, &Effect::draw(1), &mut context).unwrap();
        drop(context);
        assert!(
            !decisions.observed.is_empty(),
            "the public reveal callback actually ran"
        );
        assert!(
            decisions.observed.iter().all(|power| *power == 2),
            "the draw notification is published after this reveal boundary"
        );
        assert_eq!(
            game.current_power(recipient),
            Some(3),
            "publishing history must invalidate the warmed characteristic"
        );
    }
}

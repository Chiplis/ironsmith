use super::*;
use ironsmith_registry_test::cards::definitions::{basic_island, basic_mountain, ornithopter};
use ironsmith_registry_test::compile_to_runtime_definition;

#[test]
fn highway_robbery_resolution_options_have_visible_labels() {
    let _ids = crate::test_id_counter_guard();
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let (artifact, _) = ironsmith_registry_test::compile_to_artifact(
        "Highway Robbery",
        "Mana cost: {1}{R}\nType: Sorcery\nYou may discard a card or sacrifice a land. If you do, draw two cards.\nPlot {1}{R}",
        false,
    ).expect("Highway Robbery should compile");
    use ironsmith_registry_test::CardRegistryArtifactExt;
    let mut registry = ironsmith::cards::CardRegistry::new();
    registry
        .register_compiled_artifact(&artifact)
        .expect("baked artifact should load");
    let definition = registry.get("Highway Robbery").unwrap().clone();
    assert_eq!(
        definition.canonical_text.lines().next(),
        Some("You may discard a card or sacrifice a land. If you do, draw two cards."),
        "UI labels must preserve the original inline card text"
    );
    let spell = game.create_object_from_definition(&definition, alice, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, alice));
    let land = game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);
    let hand = game.create_object_from_definition(&ornithopter(), alice, Zone::Hand);
    seed_library(&mut game, alice);
    let before_resolution = game.clone();

    // Accept "You may ...", then suspend at the choice between the two actions.
    let answers = [crate::ReplayDecisionAnswer::Boolean(true)];
    let mut dm = WasmReplayDecisionMaker::new(&answers);
    ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm)
        .expect("spell should suspend for a resolution choice");
    let (context, viewed, _, pending_game) = dm.finish();
    let context = context.expect("expected the discard/sacrifice choice");
    let visible_game = pending_game.as_deref().unwrap_or(&game);
    let view =
        crate::DecisionView::from_context(visible_game, &context, alice, viewed.as_ref(), None);
    let view = serde_json::to_value(view).expect("decision should serialize");
    let options = view["options"].as_array().expect("expected option buttons");
    assert_eq!(options.len(), 2);
    assert!(options.iter().all(|option| option["legal"] == true));
    assert_eq!(options[0]["description"], "Discard a card");
    assert_eq!(options[1]["description"], "Sacrifice a land");
    assert!(
        options
            .iter()
            .all(|option| option["related_object_ids"].is_null()),
        "action choices must not acquire an Empty pile label"
    );
    assert!(
        options.iter().all(|option| !option["description"]
            .as_str()
            .unwrap_or("")
            .trim()
            .is_empty()),
        "Highway Robbery must label its discard and sacrifice options: {view}"
    );
    // Labels must stay attached to the original executable alternatives.
    for (mode, chosen, discarded, expected_hand_size) in
        [(0, hand, "Ornithopter", 2), (1, land, "Mountain", 3)]
    {
        let mut branch = before_resolution.clone();
        let answers = [
            crate::ReplayDecisionAnswer::Boolean(true),
            crate::ReplayDecisionAnswer::Options(vec![mode]),
            crate::ReplayDecisionAnswer::Objects(vec![chosen]),
        ];
        let mut dm = WasmReplayDecisionMaker::new(&answers);
        ironsmith::game_loop::resolve_stack_entry_with(&mut branch, &mut dm)
            .expect("selected alternative should resolve");
        let (pending, _, _, _) = dm.finish();
        assert!(
            pending.is_none(),
            "unexpected prompt after choosing {discarded}: {pending:?}"
        );
        let player = branch.player(alice).unwrap();
        assert_eq!(
            player.hand.len(),
            expected_hand_size,
            "draw two cards after choosing {discarded}"
        );
        assert!(
            player
                .graveyard
                .iter()
                .any(|id| branch.object(*id).unwrap().name == discarded)
        );
        assert_eq!(branch.battlefield.len(), if mode == 0 { 1 } else { 0 });
    }
}

fn seed_library(game: &mut GameState, player: PlayerId) {
    for _ in 0..2 {
        game.create_object_from_definition(&basic_island(), player, Zone::Library);
    }
}

#[test]
fn disjunction_choice_labels_survive_compilation_for_other_actions() {
    let _ids = crate::test_id_counter_guard();
    for (text, expected) in [
        (
            "You may sacrifice a creature or discard a card. If you do, draw a card.",
            vec!["sacrifice a creature", "discard a card"],
        ),
        (
            "Choose one —\n• Draw a card.\n• Gain 2 life.\n• Create a Treasure token.",
            vec!["draw a card", "gain 2 life", "create a treasure token"],
        ),
        (
            "Deal 2 damage to target opponent or you gain 2 life.",
            vec!["2 damage to target opponent", "gain 2 life"],
        ),
        (
            "Target creature loses first strike or swampwalk until end of turn.",
            vec!["first strike", "swampwalk"],
        ),
    ] {
        let definition = compile_to_runtime_definition(
            "Alternative action fixture",
            &format!("Mana cost: {{1}}{{B}}\nType: Sorcery\n{text}"),
            false,
        )
        .unwrap_or_else(|error| panic!("{text}: {error:?}"));
        let mut labels = Vec::new();
        fn collect(effect: &ironsmith::effect::Effect, labels: &mut Vec<Vec<String>>) {
            if let Some(choice) = effect.downcast_ref::<ironsmith::effects::ChooseModeEffect>() {
                labels.push(
                    choice
                        .modes
                        .iter()
                        .enumerate()
                        .map(|(index, _)| choice.decision_label(index).to_ascii_lowercase())
                        .collect(),
                );
            }
            effect.visit_child_effects(&mut |child| collect(child, labels));
        }
        for effect in definition
            .spell_effect
            .as_ref()
            .expect("spell program")
            .all_effects()
        {
            collect(effect, &mut labels);
        }
        assert!(
            !labels.is_empty()
                && labels
                    .iter()
                    .flatten()
                    .all(|label| !label.trim().is_empty()),
            "all alternatives need visible labels for {text}: {labels:?}"
        );
        for expected in expected {
            assert!(
                labels
                    .iter()
                    .flatten()
                    .any(|label| label.contains(expected)),
                "missing action label {expected} for {text}: {labels:?}"
            );
        }
    }
}

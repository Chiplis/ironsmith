//! Complete source inventory; every native/artifact sibling is compiled independently.
#[path = "custom_final_next_wave_support.rs"]
mod source_support;
#[test]
fn all_twenty_six_exact_bless_sources_keep_fifty_complete_faces_and_linked_definitions() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/revolution-bless-complete.json"
    ))
    .unwrap();
    let mut cards = 0;
    let mut faces = 0;
    for row in fixture["mechanics"].as_array().unwrap() {
        assert_eq!(row["keyword"], "Bless");
        for card in row["representative_cards"].as_array().unwrap() {
            cards += 1;
            let source_faces = card["source_faces"].as_array().unwrap();
            let executable_faces = card["executable_faces"].as_array().unwrap();
            assert_eq!(source_faces.len(), executable_faces.len());
            let pairs = source_support::linked_definitions(card)
                .unwrap_or_else(|error| panic!("{}: {error}", card["name"]));
            for (index, face) in executable_faces.iter().enumerate() {
                faces += 1;
                assert_eq!(face["name"], source_faces[index]["name"]);
                let name = face["name"].as_str().unwrap();
                let text = face["executable_text"].as_str().unwrap();
                for (route, definition) in pairs[index].iter().enumerate() {
                    assert_eq!(definition.card.name, name);
                    if source_faces.len() == 2 {
                        assert_eq!(
                            definition.card.other_face,
                            Some(pairs[1 - index][route].card.id)
                        );
                        assert_eq!(
                            definition.card.other_face_name.as_deref(),
                            executable_faces[1 - index]["name"].as_str()
                        );
                    }
                    if matches!(name, "Canopy of the Winter Nexus" | "Kynaios, Blade-Bound") {
                        assert!(
                            !text.contains("If this permanent's back face"),
                            "a source merely referring to Blessings cannot gain the action"
                        );
                    }
                }
            }
        }
    }
    assert_eq!(cards, 26);
    assert_eq!(faces, 50);
}

fn source_pairs(name: &str) -> Vec<(ironsmith::CardDefinition, ironsmith::CardDefinition)> {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/custom_mechanics/revolution-bless-complete.json"
    ))
    .unwrap();
    let card = fixture["mechanics"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|row| row["representative_cards"].as_array().unwrap())
        .find(|card| card["name"] == name)
        .unwrap();
    let faces = source_support::linked_definitions(card).unwrap();
    assert_eq!(faces.len(), 2);
    vec![
        (faces[0][0].clone(), faces[1][0].clone()),
        (faces[0][1].clone(), faces[1][1].clone()),
    ]
}
#[test]
fn exact_furies_any_number_player_targets_create_one_complete_blessing_per_selected_player_including_zero()
 {
    use ironsmith::decision::DecisionMaker;
    use ironsmith::{GameState, PlayerId, Target, Zone};
    const A: PlayerId = PlayerId(0);
    const B: PlayerId = PlayerId(1);
    struct SelectPlayers(usize);
    impl DecisionMaker for SelectPlayers {
        fn decide_targets(
            &mut self,
            _: &GameState,
            _: &ironsmith::decisions::context::TargetsContext,
        ) -> Vec<Target> {
            [Target::Player(A), Target::Player(B)]
                .into_iter()
                .take(self.0)
                .collect()
        }
    }
    for (front, back) in source_pairs("Furies of the Natural Order") {
        for selected in [0, 2] {
            let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);
            let source = game.create_object_from_definition(&front, A, Zone::Hand);
            let outcome = ironsmith::effects::execute_effect(
                &mut game,
                &ironsmith::Effect::move_to_zone(
                    ironsmith::target::ChooseSpec::SpecificObject(source),
                    Zone::Battlefield,
                    false,
                ),
                &mut ironsmith::effects::EffectContext::new_default(source, A),
            )
            .unwrap();
            let mut queue = ironsmith::triggers::TriggerQueue::new();
            for event in outcome.events {
                for entry in ironsmith::triggers::check_triggers(&game, &event) {
                    queue.add(entry);
                }
            }
            let mut dm = SelectPlayers(selected);
            ironsmith::game_loop::put_triggers_on_stack_with_dm(&mut game, &mut queue, &mut dm)
                .unwrap();
            assert_eq!(game.stack.len(), 1);
            ironsmith::game_loop::resolve_stack_entry_with(&mut game, &mut dm).unwrap();
            let tokens = game
                .battlefield
                .iter()
                .filter_map(|id| game.object(*id))
                .filter(|object| {
                    object.kind == ironsmith::object::ObjectKind::Token
                        && object.name.as_ref() == back.card.name
                })
                .collect::<Vec<_>>();
            assert_eq!(tokens.len(), selected);
            for player in [A, B] {
                assert_eq!(
                    tokens
                        .iter()
                        .filter(|token| token.attached_to
                            == Some(ironsmith::object::AttachmentTarget::Player(player)))
                        .count(),
                    usize::from(selected == 2)
                );
            }
            for token in tokens {
                assert_eq!(&*token.abilities, &back.abilities);
                assert_eq!(token.mana_cost.as_deref(), back.card.mana_cost.as_ref());
            }
        }
    }
}

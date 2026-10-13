//! CR 605.1a, effective 2026-09-25: library movement in costs or effects
//! disqualifies activated mana abilities, including nested/optional effects.
use ironsmith::ability::{AbilityKind, ActivatedAbilityRuntimeExt};
use ironsmith::decision::{LegalAction, SelectFirstDecisionMaker, compute_legal_actions};
use ironsmith::game_loop::{
    PriorityLoopState, PriorityResponse, apply_priority_response_with_dm, resolve_stack_entry,
};
use ironsmith::mana::{ManaCost, ManaSymbol};
use ironsmith::mana_payment::{
    ManaPaymentRequest, mana_payment_activation_inventory_checked, manual_mana_abilities_checked,
};
use ironsmith::triggers::TriggerQueue;
use ironsmith::{GameState, PlayerId, Zone};
use ironsmith_compiled_artifact::CompiledCardArtifact;
use ironsmith_compiler_runtime::{compile_to_artifact, compile_to_runtime_definition};
use ironsmith_runtime_catalog::artifact_materializer::materialize_artifact;
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

fn definitions(name: &str, text: &str) -> [ironsmith::cards::CardDefinition; 2] {
    let direct = compile_to_runtime_definition(name, text, false).unwrap();
    let (artifact, _) = compile_to_artifact(name, text, false).unwrap();
    let restored = CompiledCardArtifact::from_json(&artifact.to_json().unwrap()).unwrap();
    [direct, materialize_artifact(&restored).unwrap()]
}

#[test]
fn library_mana_abilities_are_classified_from_costs_and_effects_without_reminder_text() {
    run_with_large_stack(
        library_mana_abilities_are_classified_from_costs_and_effects_without_reminder_text_fixture,
    );
}

fn library_mana_abilities_are_classified_from_costs_and_effects_without_reminder_text_fixture() {
    // No "activate only as an instant" reminder: classification must follow
    // executable structure, including artifacts compiled before the CR change.
    let cards = [
        (
            "Selvala, Explorer Returned",
            "Type: Creature\nPower/Toughness: 1/1\n{T}: Each player reveals the top card of their library. For each nonland card revealed this way, add {G} and you gain 1 life. Then each player draws a card.",
        ),
        (
            "Chromatic Sphere",
            "Type: Artifact\n{1}, {T}, Sacrifice this artifact: Add one mana of any color. Draw a card.",
        ),
        (
            "Millikin",
            "Type: Artifact Creature\nPower/Toughness: 1/1\n{T}, Mill a card: Add {C}.",
        ),
        (
            "Deranged Assistant",
            "Type: Creature\nPower/Toughness: 1/1\n{T}, Mill a card: Add {C}.",
        ),
        (
            "Darkwater Egg",
            "Type: Artifact\n{2}, {T}, Sacrifice this artifact: Add {U}{B}. Draw a card.",
        ),
        (
            "Mossfire Egg",
            "Type: Artifact\n{2}, {T}, Sacrifice this artifact: Add {R}{G}. Draw a card.",
        ),
        (
            "Shadowblood Egg",
            "Type: Artifact\n{2}, {T}, Sacrifice this artifact: Add {B}{R}. Draw a card.",
        ),
        (
            "Skycloud Egg",
            "Type: Artifact\n{2}, {T}, Sacrifice this artifact: Add {W}{U}. Draw a card.",
        ),
        (
            "Sungrass Egg",
            "Type: Artifact\n{2}, {T}, Sacrifice this artifact: Add {G}{W}. Draw a card.",
        ),
    ];
    for (name, text) in cards {
        for definition in definitions(name, text) {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            assert_eq!(
                definition
                    .abilities
                    .iter()
                    .filter(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
                    .count(),
                1,
                "{name}"
            );
            for ability in &definition.abilities {
                if let AbilityKind::Activated(activated) = &ability.kind {
                    assert!(activated.could_add_mana(&game, source, A), "{name}");
                    assert!(activated.moves_library_cards(), "{name}: {activated:?}");
                    assert!(!activated.is_mana_ability(), "{name}");
                    assert!(
                        !activated.is_runtime_mana_ability(&game, source, A),
                        "{name}"
                    );
                }
            }
        }
    }
}

#[test]
fn library_mana_abilities_use_the_stack_and_cannot_fund_an_in_progress_wurm_cast() {
    run_with_large_stack(
        library_mana_abilities_use_the_stack_and_cannot_fund_an_in_progress_wurm_cast_fixture,
    );
}

fn library_mana_abilities_use_the_stack_and_cannot_fund_an_in_progress_wurm_cast_fixture() {
    for (name, text, mills) in [
        (
            "Selvala, Explorer Returned",
            "Type: Creature\nPower/Toughness: 1/1\n{T}: Each player reveals the top card of their library. For each nonland card revealed this way, add {G} and you gain 1 life. Then each player draws a card.",
            false,
        ),
        (
            "Millikin",
            "Type: Artifact Creature\nPower/Toughness: 1/1\n{T}, Mill a card: Add {C}.",
            true,
        ),
    ] {
        let mut variants = definitions(name, text).to_vec();
        if mills {
            variants.push(
                ironsmith::cards::builders::CardDefinitionBuilder::new(
                    ironsmith::ids::CardId::new(),
                    name,
                )
                .card_types(vec![ironsmith::types::CardType::Artifact])
                .with_ability(ironsmith::ability::Ability::mana(
                    ironsmith::cost::TotalCost::from_cost(ironsmith::costs::Cost::mill(1)),
                    vec![ManaSymbol::Colorless],
                ))
                .build(),
            );
        }
        for definition in variants {
            if mills {
                let rendered =
                    ironsmith_text::compiled_text::compiled_text_lines(&definition).join("\n");
                assert!(
                    rendered.contains("Add {C}"),
                    "fixed output must remain visible: {rendered}"
                );
            }
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
            let card = compile_to_runtime_definition(
                "Nonland",
                "Mana cost: {G}\nType: Sorcery\nYou gain 1 life.",
                false,
            )
            .unwrap();
            game.create_object_from_definition(&card, A, Zone::Library);
            game.create_object_from_definition(&card, B, Zone::Library);
            let wurm = compile_to_runtime_definition("Panglacial Wurm", "Mana cost: {5}{G}{G}\nType: Creature — Wurm\nPower/Toughness: 9/5\nTrample\nWhile you're searching your library, you may cast this card from your library.", false).unwrap();
            let wurm_id = game.create_object_from_definition(&wurm, A, Zone::Stack);
            game.turn.turn_number += 1;
            game.turn.active_player = A;
            game.turn.priority_player = Some(A);
            let payment = ManaPaymentRequest::new(
                A,
                wurm_id,
                ironsmith::costs::PaymentReason::CastSpell,
                ManaCost::from_symbols(vec![
                    ManaSymbol::Generic(5),
                    ManaSymbol::Green,
                    ManaSymbol::Green,
                ]),
            );
            assert!(
                !manual_mana_abilities_checked(&game, &payment)
                    .unwrap()
                    .iter()
                    .any(|(id, _)| *id == source)
            );
            assert!(
                !mana_payment_activation_inventory_checked(&game, &payment)
                    .unwrap()
                    .iter()
                    .any(|option| option.source == source)
            );
            let actions = compute_legal_actions(&game, A).unwrap();
            assert!(!actions.iter().any(|action| matches!(action, LegalAction::ActivateManaAbility { source: id, .. } if *id == source)));
            let activation = actions.into_iter().find(|action| matches!(action, LegalAction::ActivateAbility { source: id, .. } if *id == source)).expect("ordinary activation at priority");
            apply_priority_response_with_dm(
                &mut game,
                &mut TriggerQueue::new(),
                &mut PriorityLoopState::new(2),
                &PriorityResponse::PriorityAction(activation),
                &mut SelectFirstDecisionMaker,
            )
            .unwrap();
            assert_eq!(game.stack.len(), 1, "{name} must wait on the stack");
            assert_eq!(game.player(A).unwrap().mana_pool.total(), 0);
            assert_eq!(
                game.player(A).unwrap().library.len(),
                if mills { 0 } else { 1 }
            );
            resolve_stack_entry(&mut game).unwrap();
            assert_eq!(
                game.player(A).unwrap().mana_pool.total(),
                if mills { 1 } else { 2 },
                "{name} still adds mana on resolution"
            );
            if !mills {
                assert_eq!(game.player(A).unwrap().hand.len(), 1);
                assert_eq!(game.player(B).unwrap().hand.len(), 1);
            }
        }
    }
}

#[test]
fn chromatic_star_remains_a_mana_ability_with_a_separate_draw_trigger() {
    run_with_large_stack(
        chromatic_star_remains_a_mana_ability_with_a_separate_draw_trigger_fixture,
    );
}

fn chromatic_star_remains_a_mana_ability_with_a_separate_draw_trigger_fixture() {
    for definition in definitions(
        "Chromatic Star",
        "Type: Artifact\n{1}, {T}, Sacrifice this artifact: Add one mana of any color.\nWhen this artifact is put into a graveyard from the battlefield, draw a card.",
    ) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let activated = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .unwrap();
        assert!(activated.is_mana_ability());
        assert!(activated.is_runtime_mana_ability(&game, source, A));
        assert!(
            definition
                .abilities
                .iter()
                .any(|ability| matches!(&ability.kind, AbilityKind::Triggered(_)))
        );
    }
}

// Native compiler/session fixtures contain large by-value debug frames.
fn run_with_large_stack(test: fn()) {
    std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(test)
        .expect("spawn library-mana fixture")
        .join()
        .expect("library-mana fixture should pass");
}

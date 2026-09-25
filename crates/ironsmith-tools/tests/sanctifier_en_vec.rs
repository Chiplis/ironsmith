use ironsmith::cards::{CardDefinition, builders::CardDefinitionBuilder};
use ironsmith::color::ColorSet;
use ironsmith::ids::CardId;
use ironsmith::{CardType, GameState, PlayerId, Zone};

fn payload() -> ironsmith_tools::CardPayload {
    ironsmith_tools::load_card_payloads_by_name(
        ironsmith_tools::default_cards_path().to_str().unwrap(),
        "Sanctifier en-Vec",
    )
    .unwrap()
    .remove(0)
}
fn definition() -> CardDefinition {
    ironsmith_tools::compile_definition_from_payload(&payload()).unwrap()
}
fn probe(colors: ColorSet) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Zone-change probe")
        .card_types(vec![CardType::Creature])
        .color_indicator(colors)
        .power_toughness(ironsmith::card::PowerToughness::fixed(2, 2))
        .build()
}
#[test]
fn strict_snapshot_and_full_quality_gate() {
    let snapshot = ironsmith_tools::compile_authoritative_snapshot_from_payload(&payload());
    assert_eq!(
        snapshot.parse_status,
        ironsmith_tools::ParseStatus::StrictCompiled,
        "{:?}",
        snapshot.parse_error
    );
    assert!(!snapshot.parse_lossy && !snapshot.has_unimplemented && snapshot.parse_error.is_none());
    assert!(
        snapshot.similarity_score >= 0.99,
        "{}: {:?}",
        snapshot.similarity_score,
        snapshot.compiled_text
    );
}
#[test]
fn replaces_only_black_or_red_objects_going_to_either_graveyard_from_any_zone() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    for origin in [
        Zone::Battlefield,
        Zone::Stack,
        Zone::Hand,
        Zone::Library,
        Zone::Exile,
        Zone::Command,
        Zone::Ante,
        Zone::OutsideGame,
    ] {
        for owner in [alice, PlayerId::from_index(1)] {
            for (color, expected) in [
                (ColorSet::BLACK, Zone::Exile),
                (ColorSet::RED, Zone::Exile),
                (ColorSet::RED.union(ColorSet::GREEN), Zone::Exile),
                (ColorSet::WHITE, Zone::Graveyard),
                (ColorSet::BLUE, Zone::Graveyard),
                (ColorSet::COLORLESS, Zone::Graveyard),
            ] {
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source =
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let victim = game.create_object_from_definition(&probe(color), owner, origin);
                let moved = move_to_zone(&mut game, victim, owner, Zone::Graveyard);
                assert_eq!(
                    game.object(moved).unwrap().zone,
                    expected,
                    "origin={origin:?}, owner={owner:?}, color={color:?}"
                );
                move_to_zone(&mut game, source, alice, Zone::Exile);
                let victim = game.create_object_from_definition(&probe(color), owner, origin);
                let moved = move_to_zone(&mut game, victim, owner, Zone::Graveyard);
                assert_eq!(
                    game.object(moved).unwrap().zone,
                    Zone::Graveyard,
                    "replacement ends when source leaves"
                );
            }
        }
    }
}

fn move_to_zone(
    game: &mut GameState,
    victim: ironsmith::ObjectId,
    controller: PlayerId,
    destination: Zone,
) -> ironsmith::ObjectId {
    use ironsmith::effects::EffectExecutor;
    let identity = game.object(victim).unwrap().stable_id;
    let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
    ironsmith::effects::MoveToZoneEffect::new(
        ironsmith::target::ChooseSpec::Source,
        destination,
        false,
    )
    .execute(
        game,
        &mut ironsmith::effects::EffectContext::new(victim, controller, &mut dm),
    )
    .unwrap();
    game.find_object_by_stable_id(identity).unwrap()
}

#[test]
fn source_only_graveyard_replacement_still_functions_from_hand() {
    let alice = PlayerId::from_index(0);
    for compiled in [false, true] {
        let filter = ironsmith::filter::ObjectFilter::source();
        let ability = if compiled {
            ironsmith::static_abilities::StaticAbility::from_model(
                ironsmith::static_abilities::CompiledStaticAbility::exile_to_exile_instead_of_graveyard(filter, ironsmith::PlayerFilter::Any))
        } else {
            ironsmith::static_abilities::StaticAbility::exile_to_exile_instead_of_graveyard(
                filter,
                ironsmith::PlayerFilter::Any,
            )
        };
        let definition = CardDefinitionBuilder::new(CardId::new(), "Self replacement probe")
            .card_types(vec![CardType::Creature])
            .with_ability(ironsmith::ability::Ability::static_ability(ability))
            .build();
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let source = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let moved = move_to_zone(&mut game, source, alice, Zone::Graveyard);
        assert_eq!(
            game.object(moved).unwrap().zone,
            Zone::Exile,
            "compiled={compiled}"
        );
    }
}

#[test]
fn global_graveyard_replacement_functions_only_while_source_is_on_battlefield() {
    let alice = PlayerId::from_index(0);
    for compiled in [false, true] {
        let filter = ironsmith::filter::ObjectFilter::default();
        let ability = if compiled {
            ironsmith::static_abilities::StaticAbility::from_model(
                ironsmith::static_abilities::CompiledStaticAbility::exile_to_exile_instead_of_graveyard(filter, ironsmith::PlayerFilter::Any))
        } else {
            ironsmith::static_abilities::StaticAbility::exile_to_exile_instead_of_graveyard(
                filter,
                ironsmith::PlayerFilter::Any,
            )
        };
        let definition = CardDefinitionBuilder::new(CardId::new(), "Global replacement probe")
            .card_types(vec![CardType::Enchantment])
            .with_ability(ironsmith::ability::Ability::static_ability(ability))
            .build();
        for origin in [Zone::Hand, Zone::Battlefield, Zone::Exile] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, origin);
            let victim =
                game.create_object_from_definition(&probe(ColorSet::BLACK), alice, Zone::Hand);
            let moved = move_to_zone(&mut game, victim, alice, Zone::Graveyard);
            let expected = if origin == Zone::Battlefield {
                Zone::Exile
            } else {
                Zone::Graveyard
            };
            assert_eq!(
                game.object(moved).unwrap().zone,
                expected,
                "compiled={compiled}, source zone={origin:?}"
            );
            if origin == Zone::Battlefield {
                move_to_zone(&mut game, source, alice, Zone::Exile);
                let victim =
                    game.create_object_from_definition(&probe(ColorSet::BLACK), alice, Zone::Hand);
                let moved = move_to_zone(&mut game, victim, alice, Zone::Graveyard);
                assert_eq!(game.object(moved).unwrap().zone, Zone::Graveyard);
            }
        }
    }
}

#[test]
fn enter_trigger_exiles_matching_cards_from_all_graveyards_even_after_source_leaves() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    for remove_source in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let mut victims = Vec::new();
        for owner in [alice, PlayerId::from_index(1)] {
            for (color, expected) in [
                (ColorSet::BLACK, Zone::Exile),
                (ColorSet::RED, Zone::Exile),
                (ColorSet::BLACK.union(ColorSet::WHITE), Zone::Exile),
                (ColorSet::WHITE, Zone::Graveyard),
                (ColorSet::BLUE, Zone::Graveyard),
                (ColorSet::COLORLESS, Zone::Graveyard),
            ] {
                let id = game.create_object_from_definition(&probe(color), owner, Zone::Graveyard);
                victims.push((game.object(id).unwrap().stable_id, expected));
            }
        }
        let hand = game.create_object_from_definition(&definition, alice, Zone::Hand);
        let source = move_to_zone(&mut game, hand, alice, Zone::Battlefield);
        let mut queue = ironsmith::triggers::TriggerQueue::new();
        ironsmith::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
        assert_eq!(game.stack.len(), 1);
        if remove_source {
            move_to_zone(&mut game, source, alice, Zone::Exile);
        }
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        for (identity, expected) in victims {
            let id = game.find_object_by_stable_id(identity).unwrap();
            assert_eq!(game.object(id).unwrap().zone, expected);
        }
    }
}

#[test]
fn protection_prevents_damage_from_black_and_red_sources_only() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    for (color, expected) in [
        (ColorSet::BLACK, 0),
        (ColorSet::RED, 0),
        (ColorSet::BLACK.union(ColorSet::WHITE), 0),
        (ColorSet::WHITE, 1),
        (ColorSet::COLORLESS, 1),
    ] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let target = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source = game.create_object_from_definition(&probe(color), alice, Zone::Battlefield);
        let mut dm = ironsmith::decision::SelectFirstDecisionMaker;
        let mut ctx = ironsmith::effects::EffectContext::new(source, alice, &mut dm);
        ironsmith::effects::execute_effect(
            &mut game,
            &ironsmith::Effect::deal_damage(
                1,
                ironsmith::target::ChooseSpec::SpecificObject(target),
            ),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(game.damage_on(target), expected, "{color:?}");
    }
}

#[test]
fn protection_uses_damage_source_last_known_colors_and_respects_unpreventable_damage() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    for unpreventable in [false, true] {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let target = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let source =
            game.create_object_from_definition(&probe(ColorSet::RED), alice, Zone::Battlefield);
        let snapshot =
            ironsmith::snapshot::ObjectSnapshot::from_object(game.object(source).unwrap(), &game);
        if unpreventable {
            let rule = CardDefinitionBuilder::new(CardId::new(), "Damage rule probe")
                .card_types(vec![CardType::Enchantment])
                .with_ability(ironsmith::Ability::static_ability(
                    ironsmith::static_abilities::StaticAbility::damage_cant_be_prevented(),
                ))
                .build();
            game.create_object_from_definition(&rule, alice, Zone::Battlefield);
        }
        game.push_to_stack(
            ironsmith::game_state::StackEntry::ability(
                source,
                alice,
                vec![ironsmith::Effect::deal_damage(
                    1,
                    ironsmith::target::ChooseSpec::SpecificObject(target),
                )],
            )
            .with_source_snapshot(snapshot),
        );
        move_to_zone(&mut game, source, alice, Zone::Exile);
        ironsmith::game_loop::resolve_stack_entry(&mut game).unwrap();
        assert_eq!(game.damage_on(target), if unpreventable { 1 } else { 0 });
    }
}

#[test]
fn discard_events_exile_matching_cards_for_either_player() {
    let definition = definition();
    let alice = PlayerId::from_index(0);
    for owner in [alice, PlayerId::from_index(1)] {
        for (color, expected) in [
            (ColorSet::BLACK, Zone::Exile),
            (ColorSet::RED, Zone::Exile),
            (ColorSet::BLUE, Zone::Graveyard),
        ] {
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let card = game.create_object_from_definition(&probe(color), owner, Zone::Hand);
            let stable = game.object(card).unwrap().stable_id;
            game.push_to_stack(ironsmith::game_state::StackEntry::ability(
                source,
                owner,
                vec![ironsmith::Effect::discard(1)],
            ));
            ironsmith::game_loop::resolve_stack_entry_with(
                &mut game,
                &mut ironsmith::decision::SelectFirstDecisionMaker,
            )
            .unwrap();
            let moved = game.find_object_by_stable_id(stable).unwrap();
            assert_eq!(
                game.object(moved).unwrap().zone,
                expected,
                "{owner:?} {color:?}"
            );
        }
    }
}

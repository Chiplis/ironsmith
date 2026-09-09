use super::*;
const TEXT: &str = "This creature can't be blocked if you've cast a historic spell this turn.";

fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Relic Runner")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 1))
        .parse_text(TEXT)
        .unwrap()
}

#[test]
fn historic_cast_block_condition_updates_after_cast_and_resets_each_turn() {
    let definition = definition();
    assert!(
        definition.spell_effect.is_none(),
        "the condition must be a static ability: {definition:#?}"
    );
    for kind in 0..4 {
        for own_cast in [false, true] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Blocker")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(3, 3))
                .build();
            let blocker = game.create_object_from_card(&creature, bob, Zone::Battlefield);
            assert!(crate::rules::combat::can_block(
                game.object(source).unwrap(),
                game.object(blocker).unwrap(),
                &game
            ));
            let mut spell = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Cast Probe")
                .card_types(vec![match kind {
                    0 => CardType::Artifact,
                    2 => CardType::Enchantment,
                    _ => CardType::Creature,
                }]);
            if kind == 1 {
                spell = spell.supertypes(vec![Supertype::Legendary]);
            }
            if kind == 2 {
                spell = spell.subtypes(vec![Subtype::Saga]);
            }
            let caster = if own_cast { alice } else { bob };
            let spell = game.create_object_from_card(&spell.build(), caster, Zone::Stack);
            let event = crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::spells::SpellCastEvent::new_with_snapshot(
                    spell,
                    caster,
                    Zone::Hand,
                    crate::snapshot::ObjectSnapshot::from_object(
                        game.object(spell).unwrap(),
                        &game,
                    ),
                ),
                crate::provenance::ProvNodeId::default(),
            );
            game.queue_trigger_event(crate::provenance::ProvNodeId::default(), event);
            crate::game_loop::drain_pending_trigger_events(
                &mut game,
                &mut crate::triggers::TriggerQueue::new(),
            );
            let expected = own_cast && kind != 3;
            assert_eq!(
                game.current_has_static_ability_id(
                    source,
                    crate::static_abilities::StaticAbilityId::Unblockable
                ),
                expected,
                "kind={kind}, own_cast={own_cast}"
            );
            assert_eq!(
                crate::rules::combat::can_block(
                    game.object(source).unwrap(),
                    game.object(blocker).unwrap(),
                    &game
                ),
                !expected
            );
            game.move_object_by_effect(spell, Zone::Graveyard).unwrap();
            assert_eq!(
                game.current_has_static_ability_id(
                    source,
                    crate::static_abilities::StaticAbilityId::Unblockable
                ),
                expected
            );
            game.next_turn();
            assert!(!game.current_has_static_ability_id(
                source,
                crate::static_abilities::StaticAbilityId::Unblockable
            ));
            assert!(crate::rules::combat::can_block(
                game.object(source).unwrap(),
                game.object(blocker).unwrap(),
                &game
            ));
        }
    }
}

#[test]
fn historic_cast_block_condition_renders_as_a_static_restriction() {
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition()).join("\n"),
        "As long as you've cast a historic spell this turn, this creature can't be blocked."
    );
}

#[test]
fn conditional_unblockable_accepts_other_conditions_and_qualified_subjects() {
    for text in [
        "This creature can't be blocked if you control an artifact.",
        "Creatures you control can't be blocked if you control an artifact.",
        "This creature cannot be blocked as long as you control an artifact.",
    ] {
        let definition =
            crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Conditional Block Probe")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 2))
                .parse_text(text)
                .unwrap();
        assert!(definition.spell_effect.is_none(), "{text}");
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        assert!(!game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Unblockable
        ));
        let artifact = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Artifact Probe")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact = game.create_object_from_card(&artifact, alice, Zone::Battlefield);
        assert!(game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Unblockable
        ));
        game.move_object_by_effect(artifact, Zone::Graveyard)
            .unwrap();
        assert!(!game.current_has_static_ability_id(
            source,
            crate::static_abilities::StaticAbilityId::Unblockable
        ));
    }
}

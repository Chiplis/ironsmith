use super::*;
const TEXT: &str = "When this creature enters, target creature you control gets +X/+0 until end of turn and up to one target creature an opponent controls gets -0/-X until end of turn, where X is the number of Elves you control plus the number of Elf cards in your graveyard.";
fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Gloom Ripper")
        .card_types(vec![CardType::Creature])
        .subtypes(vec![crate::types::Subtype::Elf])
        .power_toughness(crate::card::PowerToughness::fixed(3, 3))
        .parse_text(TEXT)
        .unwrap()
}
#[test]
fn shared_pump_targets_keep_both_sides_and_one_shared_count() {
    for include_opponent in [false, true] {
        let definition = definition();
        let crate::ability::AbilityKind::Triggered(triggered) = &definition.abilities[0].kind
        else {
            panic!("trigger");
        };
        assert_eq!(triggered.choices.len(), 2, "{triggered:#?}");
        let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 20))
            .build();
        let own = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let opposing = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let elf = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Elf")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![crate::types::Subtype::Elf])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build();
        for _ in 0..2 {
            game.create_object_from_card(&elf, alice, Zone::Battlefield);
        }
        for _ in 0..3 {
            game.create_object_from_card(&elf, alice, Zone::Graveyard);
        }
        for _ in 0..4 {
            game.create_object_from_card(&elf, alice, Zone::Exile);
            game.create_object_from_card(&elf, bob, Zone::Battlefield);
            game.create_object_from_card(&elf, bob, Zone::Graveyard);
            game.create_object_from_card(&card, alice, Zone::Graveyard);
        }
        let mut targets = vec![crate::effects::ResolvedTarget::Object(own)];
        if include_opponent {
            targets.push(crate::effects::ResolvedTarget::Object(opposing));
        }
        let mut ctx = crate::effects::EffectContext::new_default(source, alice)
            .with_targets(targets)
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: triggered.choices[0].clone(),
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: triggered.choices[1].clone(),
                    range: 1..if include_opponent { 2 } else { 1 },
                },
            ]);
        ctx.snapshot_targets(&game);
        for effect in &triggered.effects {
            crate::effects::execute_effect(&mut game, effect, &mut ctx).unwrap_or_else(|error| {
                panic!("include_opponent={include_opponent}: {error:?} {triggered:#?}")
            });
        }
        assert_eq!(game.calculated_power(own), Some(8));
        assert_eq!(game.calculated_toughness(own), Some(20));
        assert_eq!(game.calculated_power(opposing), Some(2));
        assert_eq!(
            game.calculated_toughness(opposing),
            Some(if include_opponent { 14 } else { 20 })
        );
        game.create_object_from_card(&elf, alice, Zone::Battlefield);
        assert_eq!(game.calculated_power(own), Some(8), "X must stay fixed after resolution");
        assert_eq!(game.calculated_toughness(opposing), Some(if include_opponent {14} else {20}));
        game.effect_store.continuous_effects.cleanup_end_of_turn();
        game.refresh_continuous_state();
        assert_eq!(game.calculated_power(own), Some(2));
        assert_eq!(game.calculated_toughness(opposing), Some(20));
    }
}
#[test]
fn shared_pump_targets_render_both_target_clauses() {
    let rendered = crate::compiled_text::compiled_text_lines(&definition()).join("\n");
    assert!(
        rendered.contains("up to one target creature an opponent controls gets -0/-X"),
        "{rendered}"
    );
}

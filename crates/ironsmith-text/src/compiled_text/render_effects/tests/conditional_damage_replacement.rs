use super::*;
const TEXT: &str = "Whenever a creature you control attacks, it gets +2/+0 until end of turn and this enchantment deals 1 damage to you.\nHellbent — As long as you have no cards in hand, if a source you control would deal damage to a permanent or player, it deals double that damage to that permanent or player instead.";
#[test]
fn conditional_damage_replacement_applies_only_with_empty_hand_and_own_source() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Anthem of Rakdos")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    for empty_hand in [false, true] {
        for own_source in [false, true] {
            for creature_target in [false, true] {
                let mut game =
                    crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let alice = game.players[0].id;
                let bob = game.players[1].id;
                game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Damage Probe")
                    .card_types(vec![CardType::Creature])
                    .power_toughness(crate::card::PowerToughness::fixed(2, 20))
                    .build();
                if !empty_hand {
                    game.create_object_from_card(&card, alice, Zone::Hand);
                }
                let controller = if own_source { alice } else { bob };
                let source = game.create_object_from_card(&card, controller, Zone::Battlefield);
                let victim = game.create_object_from_card(&card, bob, Zone::Battlefield);
                let target = if creature_target {
                    ChooseSpec::SpecificObject(victim)
                } else {
                    ChooseSpec::SpecificPlayer(bob)
                };
                let effect = Effect::deal_damage(Value::Fixed(3), target);
                let mut ctx = crate::effects::EffectContext::new_default(source, controller);
                crate::effects::execute_effect(&mut game, &effect, &mut ctx).unwrap();
                let expected = if empty_hand && own_source { 6 } else { 3 };
                if creature_target {
                    assert_eq!(game.damage_on(victim), expected);
                } else {
                    assert_eq!(game.player(bob).unwrap().life, 20 - expected as i32);
                }
            }
        }
    }
}

#[test]
fn conditional_damage_replacement_renders_a_conditional_rule() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Anthem of Rakdos")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(rendered.contains("As long as you have no cards in hand, if a source you control would deal damage to a permanent or player, it deals double that damage to that permanent or player instead."), "{rendered}");
    assert!(!rendered.contains("creature has"), "{rendered}");
}

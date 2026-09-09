use super::*;
const TEXT: &str = "As long as your life total is greater than or equal to your starting life total, creatures you control get +1/+1.\nWhenever one or more creatures you control attack, you gain life equal to the number of attacking creatures.";
#[test]
fn starting_life_anthem_checks_starting_total_and_one_attack_group() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Path of Bravery")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    for starting in [20, 40] {
        for delta in [-1, 0, 1] {
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], starting);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let creature = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Attacker")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 3))
                .build();
            let own = (0..3)
                .map(|_| game.create_object_from_card(&creature, alice, Zone::Battlefield))
                .collect::<Vec<_>>();
            let opposing = game.create_object_from_card(&creature, bob, Zone::Battlefield);
            game.player_mut(alice).unwrap().life = starting + delta;
            game.refresh_continuous_state();
            for &id in &own {
                assert_eq!(game.current_power(id), Some(if delta >= 0 { 3 } else { 2 }));
            }
            assert_eq!(game.current_power(opposing), Some(2));
            let mut combat = crate::combat_state::CombatState::default();
            combat.attackers = own
                .iter()
                .map(|&creature| crate::combat_state::AttackerInfo {
                    creature,
                    target: crate::combat_state::AttackTarget::Player(bob),
                })
                .collect();
            game.combat = Some(combat);
            let mut queue = crate::triggers::TriggerQueue::new();
            let mut total = 0;
            for &id in &own {
                let event = crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CreatureAttackedEvent::with_total_attackers(
                        id,
                        crate::events::AttackEventTarget::Player(bob),
                        3,
                    ),
                    crate::provenance::ProvNodeId::default(),
                );
                let triggers = crate::triggers::check_triggers(&game, &event);
                total += triggers.len();
                for trigger in triggers {
                    queue.add(trigger);
                }
            }
            assert_eq!(total, 1);
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
            assert_eq!(game.player(alice).unwrap().life, starting + delta + 3);
            for &id in &own {
                assert_eq!(game.current_power(id), Some(3));
            }
        }
    }
}

#[test]
fn starting_life_anthem_renders_the_direct_life_comparison() {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Path of Bravery")
            .card_types(vec![CardType::Enchantment])
            .parse_text(TEXT)
            .unwrap();
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.starts_with(TEXT.split('\n').next().unwrap()),
        "{rendered}"
    );
}

use super::*;

#[test]
fn filtered_blocking_life_cost_respects_color_controller_count_and_affordability() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Filtered blocking fixture")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Cumulative upkeep {R}\nBlue creatures can't block creatures you control.\nNonblue creatures can't block creatures you control unless their controller pays 1 life for each blocking creature they control.").unwrap();
    for own_attacker in [false, true] {
        for blue in [false, true] {
            for count in [1usize, 2] {
                for life in [1, 3] {
                    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                    let alice = game.players[0].id; let bob = game.players[1].id; let carol = game.players[2].id;
                    game.player_mut(bob).unwrap().life = life;
                    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
                    let creature = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Combat fixture")
                        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
                    let attacker = game.create_object_from_definition(&creature, if own_attacker { alice } else { carol }, Zone::Battlefield);
                    let blocker_card = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Blocker fixture")
                        .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(2, 2))
                        .color_indicator(if blue { crate::color::ColorSet::BLUE } else { crate::color::ColorSet::GREEN }).build();
                    let declarations: Vec<_> = (0..count).map(|_| crate::decision::BlockerDeclaration {
                        blocker: game.create_object_from_definition(&blocker_card, bob, Zone::Battlefield), blocking: attacker,
                    }).collect();
                    game.refresh_continuous_state();
                    let mut combat = crate::combat_state::CombatState::default();
                    combat.attackers.push(crate::combat_state::AttackerInfo { creature: attacker, target: crate::combat_state::AttackTarget::Player(bob) });
                    let mut queue = crate::triggers::TriggerQueue::new();
                    let result = crate::game_loop::apply_blocker_declarations(&mut game, &mut combat, &mut queue, &declarations, bob);
                    let cost = if own_attacker && !blue { count as i32 } else { 0 };
                    let succeeds = !(own_attacker && blue) && life >= cost;
                    assert_eq!(result.is_ok(), succeeds, "own={own_attacker} blue={blue} count={count} life={life}: {result:?}");
                    assert_eq!(game.player(bob).unwrap().life, life - if succeeds { cost } else { 0 });
                    assert_eq!(combat.blockers.get(&attacker).map_or(0, Vec::len), if succeeds { count } else { 0 });
                }
            }
        }
    }
}

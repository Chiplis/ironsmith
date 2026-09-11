use super::*;

#[test]
fn intrinsic_counter_gate_uses_its_source_and_preserves_comparisons() {
    for subject in ["it", "this creature"] {
        for quantity in ["five or more", "exactly five", "two or fewer"] {
            let oracle = format!(
                "This creature can't attack or block unless {subject} has {quantity} +1/+1 counters on it."
            );
            let definition =
                crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Counter Gate Probe")
                    .card_types(vec![CardType::Creature])
                    .parse_text(&oracle)
                    .unwrap();
            assert!(definition.spell_effect.is_none(), "{oracle}");
            let mut game =
                crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let alice = game.players[0].id;
            let bob = game.players[1].id;
            let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
            let other = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Other")
                .card_types(vec![CardType::Creature])
                .build();
            let friend = game.create_object_from_card(&other, alice, Zone::Battlefield);
            let enemy = game.create_object_from_card(&other, bob, Zone::Battlefield);
            game.add_counters(friend, CounterType::PlusOnePlusOne, 10);
            game.add_counters(enemy, CounterType::PlusOnePlusOne, 10);
            for count in [0, 1, 2, 3, 4, 5, 6, 2, 5] {
                game.remove_counters(source, CounterType::PlusOnePlusOne, 100, None, None);
                game.add_counters(source, CounterType::PlusOnePlusOne, count);
                game.refresh_continuous_state();
                let allowed = match quantity {
                    "five or more" => count >= 5,
                    "exactly five" => count == 5,
                    _ => count <= 2,
                };
                assert_eq!(game.can_attack(source), allowed, "{oracle}, count={count}");
                assert_eq!(game.can_block_attacker(source, enemy), allowed);
                assert!(game.can_attack(friend));
                assert!(game.can_block_attacker(friend, enemy));
            }
        }
    }
}

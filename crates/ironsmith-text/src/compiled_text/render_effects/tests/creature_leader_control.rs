use super::*;
const TEXT: &str = "At the beginning of your upkeep, if a player controls more creatures than each other player, the player who controls the most creatures gains control of this creature.";
fn definition() -> crate::cards::CardDefinition {
    crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Wild Mammoth")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(3, 4))
        .parse_text(TEXT)
        .unwrap()
}
#[test]
fn creature_leader_control_uses_the_current_unique_leader_at_resolution() {
    for (counts, change, expected) in [
        ([1, 3, 2], "same", Some(1)),
        ([1, 2, 3], "same", Some(2)),
        ([3, 2, 1], "same", Some(0)),
        ([2, 2, 1], "same", Some(0)),
        ([1, 3, 2], "swap", Some(2)),
        ([1, 3, 2], "tie", Some(0)),
        ([1, 3, 2], "gone", None),
    ] {
        let mut game = crate::game_state::GameState::new(
            vec!["Alice".into(), "Bob".into(), "Carol".into()],
            20,
        );
        let players = [game.players[0].id, game.players[1].id, game.players[2].id];
        let source =
            game.create_object_from_definition(&definition(), players[0], Zone::Battlefield);
        let card = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(1, 1))
            .build();
        let mut extras = [vec![], vec![], vec![]];
        for i in 0..3 {
            for _ in 0..counts[i] - usize::from(i == 0) {
                extras[i].push(game.create_object_from_card(&card, players[i], Zone::Battlefield));
            }
        }
        let upkeep = |player| {
            crate::triggers::TriggerEvent::new_with_provenance(
                crate::events::BeginningOfUpkeepEvent::new(player),
                crate::provenance::ProvNodeId::default(),
            )
        };
        assert!(crate::triggers::check_triggers(&game, &upkeep(players[1])).is_empty());
        let triggers = crate::triggers::check_triggers(&game, &upkeep(players[0]));
        let max = *counts.iter().max().unwrap();
        let unique = counts.iter().filter(|n| **n == max).count() == 1;
        assert_eq!(triggers.len(), usize::from(unique));
        if unique {
            let mut queue = crate::triggers::TriggerQueue::new();
            for trigger in triggers {
                queue.add(trigger);
            }
            crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
            match change {
                "swap" => {
                    for id in &extras[1] {
                        game.move_object_by_effect(*id, Zone::Graveyard).unwrap();
                    }
                }
                "tie" => {
                    game.create_object_from_card(&card, players[2], Zone::Battlefield);
                }
                "gone" => {
                    game.move_object_by_effect(source, Zone::Graveyard).unwrap();
                }
                _ => {}
            }
            crate::game_loop::resolve_stack_entry(&mut game).unwrap();
        }
        assert_eq!(
            game.controller_of_id(source),
            expected.map(|i| players[i]),
            "counts={counts:?} change={change}"
        );
        if expected.is_some() {
            for _ in 0..6 {
                game.create_object_from_card(&card, players[0], Zone::Battlefield);
            }
            game.next_turn();
            assert_eq!(
                game.controller_of_id(source),
                expected.map(|i| players[i]),
                "control change must remain fixed after resolution"
            );
        }
    }
}
#[test]
fn creature_leader_control_renders_the_recipient() {
    let rendered = crate::compiled_text::compiled_text_lines(&definition()).join("\n");
    assert!(
        rendered.contains("the player who controls the most creatures gains control"),
        "{rendered}"
    );
}

#[test]
fn creature_leader_control_relative_subject_composes_with_lands_and_life() {
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Leader Test")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of your upkeep, the player who controls the most lands gains 3 life.",
        )
        .unwrap();
    let mut game = crate::game_state::GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = game.players[0].id;
    let bob = game.players[1].id;
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let land = crate::card::CardBuilder::new(crate::ids::CardId::new(), "Land")
        .card_types(vec![CardType::Land])
        .build();
    game.create_object_from_card(&land, bob, Zone::Battlefield);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::BeginningOfUpkeepEvent::new(alice),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = crate::triggers::TriggerQueue::new();
    for trigger in crate::triggers::check_triggers(&game, &event) {
        queue.add(trigger);
    }
    crate::game_loop::put_triggers_on_stack(&mut game, &mut queue).unwrap();
    crate::game_loop::resolve_stack_entry(&mut game).unwrap();
    assert_eq!(game.players[0].life, 20);
    assert_eq!(game.players[1].life, 23);
    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert!(
        rendered.contains("the player who controls the most lands gains 3 life"),
        "{rendered}"
    );
}

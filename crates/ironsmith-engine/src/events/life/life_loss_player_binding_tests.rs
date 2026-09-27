use super::*;
use crate::ability::Ability;
use crate::card::{CardBuilder, PowerToughness};
use crate::cards::builders::CardDefinitionBuilder;
use crate::effect::{Effect, EventValueSpec, Value};
use crate::game_loop::{put_triggers_on_stack, queue_triggers_from_event, resolve_stack_entry};
use crate::ids::CardId;
use crate::target::PlayerFilter;
use crate::triggers::{Trigger, TriggerEvent, TriggerQueue};
use crate::types::CardType;
use crate::zone::Zone;

fn assert_life_loss_trigger_mills_affected_player(player_index: u8, from_damage: bool) {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let affected = PlayerId::from_index(player_index);
    // This is the compiled life-loss ability of The Master of Lake-town.
    let definition = CardDefinitionBuilder::new(CardId::new(), "Life-loss mill trigger")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 2))
        .with_ability(Ability::triggered(
            Trigger::player_loses_life(PlayerFilter::Any),
            vec![Effect::mill_player(
                Value::EventValue(EventValueSpec::Amount),
                PlayerFilter::IteratedPlayer,
            )],
        ))
        .build();
    game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let library_card = CardBuilder::new(CardId::new(), "Library card")
        .card_types(vec![CardType::Land])
        .build();
    for player in [alice, bob] {
        for _ in 0..5 {
            game.create_object_from_card(&library_card, player, Zone::Library);
        }
    }

    let event = TriggerEvent::new_with_provenance(
        LifeLossEvent::new(affected, 3, from_damage),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = TriggerQueue::new();
    queue_triggers_from_event(&mut game, &mut queue, event, false);
    assert_eq!(queue.entries.len(), 1, "life loss should trigger once");
    put_triggers_on_stack(&mut game, &mut queue).expect("life-loss trigger should stack");
    assert_eq!(game.stack.len(), 1);
    resolve_stack_entry(&mut game).expect("life-loss trigger should resolve");

    for player in [alice, bob] {
        let state = game.player(player).expect("player should exist");
        let milled = if player == affected { 3 } else { 0 };
        assert_eq!(state.graveyard.len(), milled, "mill the affected player");
        assert_eq!(state.library.len(), 5 - milled, "mill the event's amount");
    }
}

#[test]
fn life_loss_trigger_binds_controller_and_opponent_from_effects() {
    for player_index in 0..2 {
        assert_life_loss_trigger_mills_affected_player(player_index, false);
    }
}

#[test]
fn life_loss_trigger_binds_controller_and_opponent_from_damage() {
    for player_index in 0..2 {
        assert_life_loss_trigger_mills_affected_player(player_index, true);
    }
}

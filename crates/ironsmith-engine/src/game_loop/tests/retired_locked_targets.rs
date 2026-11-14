//! CR 400.7 / 611.2c: a resolved effect locked to objects that have all left
//! their zones can never apply again, so it is retired between actions.
use super::*;
use crate::continuous::{ContinuousEffect, EffectSourceType, Modification};

fn creature(game: &mut GameState, name: &str) -> ObjectId {
    game.create_object_from_card(
        &CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        PlayerId::from_index(0),
        Zone::Battlefield,
    )
}

fn register_rest_of_game_pump(game: &mut GameState, locked: Vec<ObjectId>) {
    let source = locked[0];
    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::from_resolution(
            source,
            PlayerId::from_index(0),
            locked,
            Modification::ModifyPowerToughness { power: 1, toughness: 1 },
        )
        .until(Until::Forever),
    );
    game.mark_continuous_state_dirty();
}

fn locked_sets(game: &GameState) -> Vec<Vec<ObjectId>> {
    game.effect_store
        .continuous_effects
        .effects()
        .iter()
        .filter_map(|effect| match &effect.source_type {
            EffectSourceType::Resolution { locked_targets } => Some(locked_targets.clone()),
            _ => None,
        })
        .collect()
}

fn leave(game: &mut GameState, id: ObjectId) {
    game.move_object(id, Zone::Graveyard, crate::events::cause::EventCause::effect())
        .expect("move to graveyard");
    check_and_apply_sbas(game, &mut TriggerQueue::new()).unwrap();
}

#[test]
fn rest_of_game_effect_retires_once_every_locked_object_left() {
    let mut game = setup_game();
    let first = creature(&mut game, "First");
    let second = creature(&mut game, "Second");
    register_rest_of_game_pump(&mut game, vec![first, second]);
    assert_eq!(game.calculated_power(first), Some(3));

    leave(&mut game, first);
    assert_eq!(locked_sets(&game), vec![vec![first, second]], "a live member keeps the effect intact");
    assert_eq!(game.calculated_power(second), Some(3));

    leave(&mut game, second);
    assert!(locked_sets(&game).is_empty(), "no locked object exists any more");
}

#[test]
fn returning_card_is_a_new_object_and_does_not_keep_the_effect() {
    let mut game = setup_game();
    let pumped = creature(&mut game, "Pumped");
    register_rest_of_game_pump(&mut game, vec![pumped]);

    let in_graveyard = game
        .move_object(pumped, Zone::Graveyard, crate::events::cause::EventCause::effect())
        .expect("move to graveyard");
    let returned = game
        .move_object(in_graveyard, Zone::Battlefield, crate::events::cause::EventCause::effect())
        .expect("return to battlefield");
    check_and_apply_sbas(&mut game, &mut TriggerQueue::new()).unwrap();

    assert!(locked_sets(&game).is_empty());
    assert_eq!(game.calculated_power(returned), Some(2));
}

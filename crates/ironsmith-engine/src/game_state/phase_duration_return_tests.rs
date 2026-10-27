use super::*;
use crate::ability::Ability;
use crate::cards::builders::CardDefinitionBuilder;
use crate::events::PermanentPhasedInEvent;
use crate::static_abilities::Anthem;

fn setup() -> (GameState, [ObjectId; 2], [ObjectId; 2]) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let source = CardDefinitionBuilder::new(CardId::new(), "Phase duration source")
        .card_types(vec![crate::types::CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::new(Anthem::new(
            crate::target::ObjectFilter::creature(),
            1,
            0,
        ))))
        .build();
    let sources = [
        game.create_object_from_definition(&source, PlayerId(0), Zone::Battlefield),
        game.create_object_from_definition(&source, PlayerId(1), Zone::Battlefield),
    ];
    let target = CardDefinitionBuilder::new(CardId::new(), "Held permanent")
        .card_types(vec![crate::types::CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(1, 2))
        .build();
    let targets = [
        game.create_object_from_definition(&target, PlayerId(0), Zone::Battlefield),
        game.create_object_from_definition(&target, PlayerId(1), Zone::Battlefield),
    ];
    game.phase_out_simultaneously(&targets);
    for index in 0..2 {
        game.hold_phased_out_until_source_leaves(targets[index], sources[index]);
    }
    game.take_pending_trigger_events();
    (game, sources, targets)
}

#[test]
fn simultaneous_and_sequential_source_departures_have_distinct_return_instructions_and_worlds() {
    for simultaneous in [false, true] {
        for reverse in [false, true] {
            let (mut game, sources, targets) = setup();
            let opened = simultaneous && game.open_simultaneous_action();
            let original_batch = game.simultaneous_action_batch();
            let order = if reverse { [1, 0] } else { [0, 1] };
            for index in order {
                game.move_object_by_game_rule(sources[index], Zone::Graveyard)
                    .unwrap();
                assert_eq!(game.is_phased_out(targets[index]), simultaneous);
            }
            game.close_simultaneous_action(opened);
            for target in targets {
                assert!(!game.is_phased_out(target));
            }
            let events = game.take_pending_trigger_events();
            let incoming = events
                .iter()
                .filter_map(|event| {
                    event
                        .downcast::<PermanentPhasedInEvent>()
                        .map(|phased| (event.simultaneous_batch(), phased))
                })
                .collect::<Vec<_>>();
            assert_eq!(incoming.len(), 2);
            assert_eq!(incoming[0].0 == incoming[1].0, simultaneous);
            if simultaneous {
                assert_ne!(incoming[0].0, original_batch);
            }
            let powers = incoming
                .iter()
                .map(|(_, phased)| phased.snapshot.as_ref().unwrap().power)
                .collect::<Vec<_>>();
            assert_eq!(
                powers,
                if simultaneous {
                    vec![Some(1), Some(1)]
                } else {
                    vec![Some(2), Some(1)]
                },
                "returned snapshots see the completed source departures"
            );
            game.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn independent_phase_in_cancels_a_queued_tapped_return_even_after_another_phase_out() {
    for phase_again in [false, true] {
        let (mut game, sources, targets) = setup();
        game.tap_as_held_phase_in(targets[0]);
        let opened = game.open_simultaneous_action();
        game.move_object_by_game_rule(sources[0], Zone::Graveyard)
            .unwrap();
        assert!(game.is_phased_out(targets[0]));
        game.phase_in(targets[0]);
        assert!(!game.is_phased_out(targets[0]));
        assert!(!game.is_tapped(targets[0]));
        if phase_again {
            game.phase_out(targets[0]);
        }
        game.close_simultaneous_action(opened);
        assert_eq!(game.is_phased_out(targets[0]), phase_again);
        assert!(!game.is_tapped(targets[0]));
    }
}

#[test]
fn pending_return_groups_are_isolated_between_native_game_clones() {
    let (mut game, sources, targets) = setup();
    let opened = game.open_simultaneous_action();
    game.move_object_by_game_rule(sources[0], Zone::Graveyard)
        .unwrap();
    let mut clone = game.clone();
    clone.close_simultaneous_action(opened);
    assert!(!clone.is_phased_out(targets[0]));
    assert!(clone.is_phased_out(targets[1]));
    assert!(game.is_phased_out(targets[0]));
    game.move_object_by_game_rule(sources[1], Zone::Graveyard)
        .unwrap();
    game.close_simultaneous_action(opened);
    assert!(!game.is_phased_out(targets[0]) && !game.is_phased_out(targets[1]));
    assert!(clone.is_phased_out(targets[1]));
}

#[test]
fn resumed_original_frames_finish_their_phase_return_group_after_the_physical_original() {
    let (mut game, sources, targets) = setup();
    let opened = game.open_simultaneous_action();
    let frame = game.capture_simultaneous_original_frame();
    game.close_simultaneous_action(opened);
    game.with_simultaneous_original_frame(&frame, |game| {
        game.move_object_by_game_rule(sources[0], Zone::Graveyard)
            .unwrap();
        assert!(game.is_phased_out(targets[0]));
        Ok::<_, ()>(())
    })
    .unwrap();
    assert!(!game.is_phased_out(targets[0]));
    assert!(game.is_phased_out(targets[1]));
}

#[test]
fn releasing_a_lookback_owned_scope_returns_all_held_participants_together() {
    let (mut game, sources, targets) = setup();
    let lookback = game.trigger_source_lookback_snapshots();
    game.set_simultaneous_event_lookback(Some(lookback));
    for source in sources {
        game.move_object_by_game_rule(source, Zone::Graveyard)
            .unwrap();
    }
    assert!(targets.iter().all(|target| game.is_phased_out(*target)));
    game.set_simultaneous_event_lookback(None);
    assert!(targets.iter().all(|target| !game.is_phased_out(*target)));
    let events = game.take_pending_trigger_events();
    let batches = events
        .iter()
        .filter(|event| event.downcast::<PermanentPhasedInEvent>().is_some())
        .map(|event| event.simultaneous_batch())
        .collect::<Vec<_>>();
    assert_eq!(batches.len(), 2);
    assert_eq!(batches[0], batches[1]);
}

#[test]
fn the_departure_and_its_later_phase_return_are_distinct_library_visibility_boundaries() {
    let player = PlayerId(0);
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let rule = CardDefinitionBuilder::new(CardId::new(), "Library reveal rule")
        .card_types(vec![crate::types::CardType::Artifact])
        .with_ability(Ability::static_ability(
            StaticAbility::all_players_look_at_your_top_library_card(),
        ))
        .build();
    let source = game.create_object_from_definition(&rule, player, Zone::Battlefield);
    let held = game.create_object_from_definition(&rule, player, Zone::Battlefield);
    game.phase_out(held);
    game.hold_phased_out_until_source_leaves(held, source);
    let card = CardDefinitionBuilder::new(CardId::new(), "Revealed library top").build();
    let old_top = game.create_object_from_definition(&card, player, Zone::Library);
    let stable = game.object(old_top).unwrap().stable_id;
    game.refresh_continuous_state().unwrap();
    let opened = game.open_simultaneous_action();
    game.move_object_by_game_rule(source, Zone::Graveyard)
        .unwrap();
    assert!(game.is_phased_out(held));
    game.close_simultaneous_action(opened);
    assert!(!game.is_phased_out(held));
    let new_top = *game.player(player).unwrap().library.last().unwrap();
    assert_ne!(
        new_top, old_top,
        "the reveal stopped after departure and before the second one-shot action"
    );
    assert_eq!(game.object(new_top).unwrap().stable_id, stable);
    game.validate_zone_consistency().unwrap();
}

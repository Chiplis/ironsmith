use super::*;
use crate::cards::builders::CardDefinitionBuilder;
use crate::effects::cards::{LibraryCardPosition, position_library_card};

fn setup(seed: u64, owner: PlayerId) -> (GameState, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    game.set_random_seed(seed);
    let definition = CardDefinitionBuilder::new(CardId::new(), "Library subject").build();
    for _ in 0..8 {
        game.create_object_from_definition(&definition, owner, Zone::Library);
    }
    let positioned = game.create_object_from_definition(&definition, owner, Zone::Library);
    let incoming = game.create_object_from_definition(&definition, owner, Zone::Battlefield);
    (game, positioned, incoming)
}

#[test]
fn top_bottom_and_fixed_positions_include_later_shuffled_arrivals() {
    for seed in 1..=24 {
        for owner in [PlayerId(0), PlayerId(1)] {
            for position in [
                LibraryCardPosition::Top,
                LibraryCardPosition::Bottom,
                LibraryCardPosition::NthFromTop(3),
            ] {
                for arrives_first in [false, true] {
                    let (mut game, positioned, incoming) = setup(seed, owner);
                    let opened = game.open_simultaneous_action();
                    if arrives_first {
                        game.move_object_by_game_rule(incoming, Zone::Library)
                            .unwrap();
                    }
                    position_library_card(
                        &mut game,
                        owner,
                        positioned,
                        position,
                        "specified position",
                    );
                    if !arrives_first {
                        game.move_object_by_game_rule(incoming, Zone::Library)
                            .unwrap();
                    }
                    game.shuffle_player_library(owner);
                    game.close_simultaneous_action(opened);
                    let library = &game.player(owner).unwrap().library;
                    let index = match position {
                        LibraryCardPosition::Top => library.len() - 1,
                        LibraryCardPosition::Bottom => 0,
                        LibraryCardPosition::NthFromTop(n) => library.len() - n,
                    };
                    assert_eq!(
                        library[index], positioned,
                        "seed={seed}, owner={owner:?}, position={position:?}, arrives_first={arrives_first}"
                    );
                    assert_eq!(library.len(), 10);
                    game.validate_zone_consistency().unwrap();
                }
            }
        }
    }
}

#[test]
fn multiple_placements_keep_their_chosen_order_and_shuffle_only_the_complement() {
    for seed in 1..=24 {
        let owner = PlayerId(0);
        let (mut game, a, incoming) = setup(seed, owner);
        let b = game.player(owner).unwrap().library[0];
        let bottom = game.player(owner).unwrap().library[1];
        let opened = game.open_simultaneous_action();
        crate::effects::cards::arrange_library_cards(
            &mut game,
            owner,
            &[a, b],
            &[bottom],
            "chosen partitions",
        );
        let shuffled = game
            .move_object_by_game_rule(incoming, Zone::Library)
            .unwrap();
        game.shuffle_player_library(owner);
        game.close_simultaneous_action(opened);
        let library = &game.player(owner).unwrap().library;
        assert_eq!(&library[library.len() - 2..], &[b, a]);
        assert_eq!(library[0], bottom);
        assert!(library.contains(&shuffled));
        let operations = game.crypto_audit_operations_since(0);
        let shuffle = operations
            .iter()
            .rev()
            .find_map(|operation| match operation {
                HiddenInfoOperation::LibraryShuffle { before_order, .. } => Some(before_order),
                _ => None,
            })
            .unwrap();
        assert!(!shuffle.contains(&a) && !shuffle.contains(&b) && !shuffle.contains(&bottom));
        assert!(shuffle.contains(&shuffled));
    }
}

#[test]
fn an_independent_later_action_randomizes_the_previously_positioned_card() {
    let mut moved = 0;
    for seed in 1..=24 {
        let owner = PlayerId(0);
        let (mut game, card, _) = setup(seed, owner);
        let opened = game.open_simultaneous_action();
        position_library_card(
            &mut game,
            owner,
            card,
            LibraryCardPosition::Top,
            "earlier action",
        );
        game.close_simultaneous_action(opened);
        let opened = game.open_simultaneous_action();
        game.shuffle_player_library(owner);
        game.close_simultaneous_action(opened);
        moved += usize::from(game.player(owner).unwrap().library.last() != Some(&card));
    }
    assert!(
        moved > 12,
        "placement ownership must not leak to a later instruction"
    );
}

#[test]
fn native_clones_and_retained_original_frames_keep_separate_placement_ownership() {
    let owner = PlayerId(0);
    let (mut game, a, _) = setup(17, owner);
    let b = game.player(owner).unwrap().library[0];
    let opened = game.open_simultaneous_action();
    position_library_card(
        &mut game,
        owner,
        a,
        LibraryCardPosition::Top,
        "parent original",
    );
    let frame = game.capture_simultaneous_original_frame();
    let mut clone = game.clone();
    position_library_card(&mut clone, owner, b, LibraryCardPosition::Top, "clone only");
    clone.shuffle_player_library(owner);
    assert_eq!(clone.player(owner).unwrap().library.last(), Some(&b));
    game.shuffle_player_library(owner);
    assert_eq!(game.player(owner).unwrap().library.last(), Some(&a));
    game.close_simultaneous_action(opened);
    let child = game.open_simultaneous_action();
    position_library_card(
        &mut game,
        owner,
        b,
        LibraryCardPosition::Top,
        "added child instruction",
    );
    game.shuffle_player_library(owner);
    assert_eq!(game.player(owner).unwrap().library.last(), Some(&b));
    game.close_simultaneous_action(child);
    game.with_simultaneous_original_frame(&frame, |game| {
        game.shuffle_player_library(owner);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(game.player(owner).unwrap().library.last(), Some(&a));
    game.validate_zone_consistency().unwrap();
}

#[test]
fn a_retained_revealed_top_is_not_a_member_of_the_hidden_shuffle() {
    for owner in [PlayerId(0), PlayerId(1)] {
        let (mut game, top, incoming) = setup(19, owner);
        let rule = CardDefinitionBuilder::new(CardId::new(), "Public library rule")
            .card_types(vec![crate::types::CardType::Artifact])
            .with_ability(crate::ability::Ability::static_ability(
                StaticAbility::all_players_look_at_your_top_library_card(),
            ))
            .build();
        game.create_object_from_definition(&rule, owner, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        let info = HiddenCardInfo {
            incarnation: Some(5), owner, zone: Zone::Library, slot: 7,
            commitment: "retained commitment".into(), origin_slot: Some(2),
            origin_commitment: Some("origin commitment".into()), public_slot: Some(3),
            public_commitment: Some("public commitment".into()),
        };
        game.set_hidden_card_info(top, info.clone());
        game.mark_hidden_cards_publicly_revealed(&[top]);
        let checkpoint = game.crypto_audit_checkpoint();
        let opened = game.open_simultaneous_action();
        position_library_card(&mut game, owner, top, LibraryCardPosition::Top, "retained top");
        game.move_object_by_game_rule(incoming, Zone::Library).unwrap();
        game.shuffle_player_library(owner);
        game.close_simultaneous_action(opened);
        assert_eq!(game.player(owner).unwrap().library.last(), Some(&top));
        assert_eq!(game.hidden_card_info(top), Some(&info));
        assert!(game.is_publicly_revealed_hidden_card(top));
        for operation in game.crypto_audit_operations_since(checkpoint) {
            match operation {
                HiddenInfoOperation::LibraryShuffle { before_order, after_order, input_commitments, .. } => {
                    assert!(!before_order.contains(&top) && !after_order.contains(&top));
                    assert!(!input_commitments.iter().any(|commitment| commitment == "public commitment"));
                }
                HiddenInfoOperation::HiddenMove { old_object_id, .. } => assert_ne!(old_object_id, top),
                _ => {}
            }
        }
        game.validate_zone_consistency().unwrap();
    }
}

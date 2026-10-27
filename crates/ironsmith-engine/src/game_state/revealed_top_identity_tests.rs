use super::*;
use crate::ability::Ability;
use crate::cards::builders::CardDefinitionBuilder;
use crate::effect::Effect;
use crate::effects::{ExecutionContext, MoveToZoneEffect};
use crate::target::ChooseSpec;

fn setup(player: PlayerId, visibility: u8) -> (GameState, ObjectId, ObjectId) {
    let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
    let rule = match visibility {
        0 => None,
        1 => Some(StaticAbility::look_at_top_card_of_library()),
        2 => Some(StaticAbility::all_players_look_at_your_top_library_card()),
        3 => Some(StaticAbility::all_players_look_at_top_cards_of_libraries()),
        _ => unreachable!(),
    };
    let mut source = CardDefinitionBuilder::new(CardId::new(), "Library visibility source")
        .card_types(vec![crate::types::CardType::Artifact]);
    if let Some(rule) = rule {
        source = source.with_ability(Ability::static_ability(rule));
    }
    let source = game.create_object_from_definition(&source.build(), player, Zone::Battlefield);
    let card = CardDefinitionBuilder::new(CardId::new(), "Library identity subject")
        .card_types(vec![crate::types::CardType::Creature])
        .build();
    let top = game.create_object_from_definition(&card, player, Zone::Library);
    game.refresh_continuous_state().unwrap();
    (game, source, top)
}
fn cover_and_uncover(game: &mut GameState, player: PlayerId, source: ObjectId) {
    let cover = CardDefinitionBuilder::new(CardId::new(), "Covering card").build();
    let cover = game.create_object_from_definition(&cover, player, Zone::Hand);
    let mut ctx = ExecutionContext::new_default(source, player);
    crate::effects::execute_effect(
        game,
        &Effect::new(MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(cover),
            Zone::Library,
            true,
        )),
        &mut ctx,
    )
    .unwrap();
    let cover = *game.player(player).unwrap().library.last().unwrap();
    crate::effects::execute_effect(
        game,
        &Effect::new(MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(cover),
            Zone::Hand,
            false,
        )),
        &mut ctx,
    )
    .unwrap();
}

#[test]
fn public_top_epochs_are_distinct_from_private_look_permissions() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for visibility in 0..4 {
            let (mut game, source, old) = setup(player, visibility);
            let stable = game.object(old).unwrap().stable_id;
            cover_and_uncover(&mut game, player, source);
            let top = *game.player(player).unwrap().library.last().unwrap();
            assert_eq!(game.object(top).unwrap().stable_id, stable);
            assert_eq!(top != old, visibility >= 2);
            game.refresh_continuous_state().unwrap();
            assert_eq!(*game.player(player).unwrap().library.last().unwrap(), top);
            game.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn public_top_identity_waits_for_announcement_and_is_clone_local() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let (game, source, old) = setup(player, 2);
        let mut branch = game.clone();
        let owner = LibraryTopAnnouncement::Cast(ObjectId(900));
        branch.begin_library_top_announcement(owner);
        cover_and_uncover(&mut branch, player, source);
        assert_eq!(*branch.player(player).unwrap().library.last().unwrap(), old);
        branch.finish_library_top_announcement(owner);
        assert_ne!(*branch.player(player).unwrap().library.last().unwrap(), old);
        assert_eq!(*game.player(player).unwrap().library.last().unwrap(), old);
        assert!(game.object(old).is_some());
    }
}

#[test]
fn removing_one_of_two_reveal_sources_keeps_identity_but_phasing_the_last_breaks_it() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let (mut game, source, old) = setup(player, 2);
        let extra = CardDefinitionBuilder::new(CardId::new(), "Second reveal source")
            .with_ability(Ability::static_ability(
                StaticAbility::all_players_look_at_your_top_library_card(),
            ))
            .build();
        let extra = game.create_object_from_definition(&extra, player, Zone::Battlefield);
        game.refresh_continuous_state().unwrap();
        game.phase_out(source);
        game.refresh_continuous_state().unwrap();
        assert_eq!(*game.player(player).unwrap().library.last().unwrap(), old);
        game.phase_out(extra);
        assert_eq!(*game.player(player).unwrap().library.last().unwrap(), old);
        game.phase_in(source);
        game.refresh_continuous_state().unwrap();
        let new = *game.player(player).unwrap().library.last().unwrap();
        assert_ne!(new, old);
        game.phase_in(extra);
        game.refresh_continuous_state().unwrap();
        assert_eq!(*game.player(player).unwrap().library.last().unwrap(), new);
        game.validate_zone_consistency().unwrap();
    }
}

#[test]
fn simultaneous_library_arrivals_do_not_reveal_an_intermediate_top() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let (mut game, source, _) = setup(player, 2);
        let card =
            CardDefinitionBuilder::new(CardId::new(), "Simultaneous library arrival").build();
        for _ in 0..2 {
            game.create_object_from_definition(&card, player, Zone::Hand);
        }
        let mut ctx = ExecutionContext::new_default(source, player);
        crate::effects::execute_effect(
            &mut game,
            &Effect::new(MoveToZoneEffect::new(
                ChooseSpec::All(crate::target::ObjectFilter::default().in_zone(Zone::Hand)),
                Zone::Library,
                true,
            )),
            &mut ctx,
        )
        .unwrap();
        let library = game.player(player).unwrap().library.to_vec();
        assert_eq!(library.len(), 3);
        let intermediate = library[1];
        let top = library[2];
        crate::effects::execute_effect(
            &mut game,
            &Effect::new(MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(top),
                Zone::Hand,
                false,
            )),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            *game.player(player).unwrap().library.last().unwrap(),
            intermediate,
            "the lower simultaneous arrival was never revealed before this instruction"
        );
    }
}

#[test]
fn revealed_hidden_library_identity_keeps_its_commitment_and_advances_known_generation() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for generation in [None, Some(0), Some(7)] {
            let (mut game, source, old) = setup(player, 2);
            let stable = game.object(old).unwrap().stable_id;
            game.set_hidden_card_info(
                old,
                HiddenCardInfo {
                    incarnation: generation,
                    owner: player,
                    zone: Zone::Library,
                    slot: 7,
                    commitment: "current-commitment".into(),
                    origin_slot: Some(2),
                    origin_commitment: Some("original-commitment".into()),
                    public_slot: Some(3),
                    public_commitment: Some("public-commitment".into()),
                },
            );
            game.mark_hidden_cards_publicly_revealed(&[old]);
            let checkpoint = game.crypto_audit_checkpoint();
            cover_and_uncover(&mut game, player, source);
            let new = *game.player(player).unwrap().library.last().unwrap();
            assert_ne!(
                new, old,
                "hidden-tracked cards obey the same CR401.6 identity rule"
            );
            assert_eq!(game.object(new).unwrap().stable_id, stable);
            assert!(game.hidden_card_info(old).is_none());
            let info = game.hidden_card_info(new).unwrap();
            assert_eq!(info.incarnation, generation.map(|value| value + 1));
            assert_eq!(info.slot, 7);
            assert_eq!(info.commitment, "current-commitment");
            assert_eq!(info.origin_slot, Some(2));
            assert_eq!(
                info.origin_commitment.as_deref(),
                Some("original-commitment")
            );
            assert_eq!(info.public_slot, Some(3));
            assert_eq!(info.public_commitment.as_deref(), Some("public-commitment"));
            assert!(!game.is_publicly_revealed_hidden_card(old));
            assert!(game.crypto_audit_operations_since(checkpoint).iter().any(|operation| matches!(operation,
                HiddenInfoOperation::HiddenMove { old_object_id, new_object_id, from: Zone::Library, to: Zone::Library, .. }
                if *old_object_id == old && *new_object_id == new)));
            game.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn shuffling_revealed_top_renews_it_even_when_it_returns_to_the_same_position() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for seed in 0..12 {
            let (mut game, _, _) = setup(player, 2);
            let card =
                CardDefinitionBuilder::new(CardId::new(), "Shuffle identity subject").build();
            for _ in 0..2 {
                game.create_object_from_definition(&card, player, Zone::Library);
            }
            let old = *game.player(player).unwrap().library.last().unwrap();
            let stable = game.object(old).unwrap().stable_id;
            game.set_random_seed(seed);
            game.shuffle_player_library(player);
            let current = game.find_object_by_stable_id(stable).unwrap();
            assert_ne!(
                current, old,
                "CR701.20d renews every revealed shuffled card, seed={seed}"
            );
            assert!(game.player(player).unwrap().library.contains(&current));
            game.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn simultaneous_reveal_source_exchange_has_no_concealment_gap() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let (mut game, source, top) = setup(player, 2);
        let extra = CardDefinitionBuilder::new(CardId::new(), "Incoming reveal source")
            .with_ability(Ability::static_ability(
                StaticAbility::all_players_look_at_your_top_library_card(),
            ))
            .build();
        let extra = game.create_object_from_definition(&extra, player, Zone::Battlefield);
        game.phase_out(extra);
        game.refresh_continuous_state().unwrap();
        game.phase_simultaneously(&[source], &[extra]);
        game.refresh_continuous_state().unwrap();
        assert_eq!(
            *game.player(player).unwrap().library.last().unwrap(),
            top,
            "one reveal source remains present throughout the simultaneous exchange"
        );
    }
}

#[test]
fn public_top_epochs_follow_the_reveal_sources_current_controller() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let other = PlayerId::from_index((1 - player.index()) as u8);
        let (mut game, source, top) = setup(player, 2);
        let card = CardDefinitionBuilder::new(CardId::new(), "Other player's top").build();
        let other_top = game.create_object_from_definition(&card, other, Zone::Library);
        let mut ctx = ExecutionContext::new_default(source, player);
        for controller in [other, player] {
            crate::effects::execute_effect(
                &mut game,
                &Effect::new(crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Specific(source),
                    crate::continuous::Modification::ChangeController(controller),
                    crate::effect::Until::EndOfTurn,
                )),
                &mut ctx,
            )
            .unwrap();
        }
        assert_ne!(*game.player(player).unwrap().library.last().unwrap(), top);
        assert_eq!(
            *game.player(other).unwrap().library.last().unwrap(),
            other_top,
            "the other top is hidden now; its new identity is assigned when it is revealed again"
        );
        crate::effects::execute_effect(
            &mut game,
            &Effect::new(crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Specific(source),
                crate::continuous::Modification::ChangeController(other),
                crate::effect::Until::EndOfTurn,
            )),
            &mut ctx,
        )
        .unwrap();
        assert_ne!(
            *game.player(other).unwrap().library.last().unwrap(),
            other_top
        );
    }
}

#[test]
fn hidden_top_shuffle_journal_replays_permutation_before_identity_renewals() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        for seed in 0..12 {
            let (mut game, _, _) = setup(player, 2);
            let card = CardDefinitionBuilder::new(CardId::new(), "Journal shuffle subject").build();
            for _ in 0..2 {
                game.create_object_from_definition(&card, player, Zone::Library);
            }
            let before = game.player(player).unwrap().library.to_vec();
            for (index, id) in before.iter().enumerate() {
                game.set_hidden_card_info(
                    *id,
                    HiddenCardInfo {
                        incarnation: Some(index as u64 + 1),
                        owner: player,
                        zone: Zone::Library,
                        slot: index as u16,
                        commitment: format!("ciphertext-{index}"),
                        origin_slot: Some(index as u16),
                        origin_commitment: Some(format!("original-{index}")),
                        public_slot: None,
                        public_commitment: None,
                    },
                );
            }
            let checkpoint = game.crypto_audit_checkpoint();
            game.set_random_seed(seed);
            game.shuffle_player_library(player);
            let operations = game.crypto_audit_operations_since(checkpoint);
            let mut replay = None;
            for operation in &operations {
                match operation {
                    HiddenInfoOperation::LibraryShuffle {
                        before_order,
                        after_order,
                        ..
                    } => {
                        assert_eq!(before_order, &before);
                        assert!(replay.is_none());
                        replay = Some(after_order.clone());
                    }
                    HiddenInfoOperation::HiddenMove {
                        old_object_id,
                        new_object_id,
                        from,
                        to,
                        ..
                    } => {
                        assert_eq!((*from, *to), (Zone::Library, Zone::Library));
                        let replay = replay
                            .as_mut()
                            .expect("the permutation must precede its reveal identity changes");
                        let position = replay.iter().position(|id| id == old_object_id).unwrap();
                        replay[position] = *new_object_id;
                        assert!(game.hidden_card_info(*old_object_id).is_none());
                    }
                    _ => {}
                }
            }
            assert_eq!(
                replay.unwrap(),
                game.player(player).unwrap().library.to_vec(),
                "seed={seed}"
            );
            assert_eq!(game.hidden_card_entries().count(), 3);
            game.validate_zone_consistency().unwrap();
        }
    }
}

#[test]
fn hidden_library_identity_generation_exhaustion_rejects_before_mutation() {
    let player = PlayerId::from_index(0);
    let (mut game, _, old) = setup(player, 2);
    game.set_hidden_card_info(
        old,
        HiddenCardInfo {
            incarnation: Some(u64::MAX),
            owner: player,
            zone: Zone::Library,
            slot: 0,
            commitment: "exhausted-generation".into(),
            origin_slot: None,
            origin_commitment: None,
            public_slot: None,
            public_commitment: None,
        },
    );
    let before = game.hidden_card_info(old).unwrap().clone();
    let ids = game.next_object_id_counter();
    let checkpoint = game.crypto_audit_checkpoint();
    let (root, meter) = game.begin_token_resource_scope();
    assert!(game.reidentify_reordered_library_card(old).is_none());
    assert_eq!(game.next_object_id_counter(), ids);
    assert_eq!(game.hidden_card_info(old), Some(&before));
    assert_eq!(game.player(player).unwrap().library.last(), Some(&old));
    assert!(game.crypto_audit_operations_since(checkpoint).is_empty());
    assert!(game.token_resource_failure().is_some());
    game.end_token_resource_scope(root, &meter);
}

#[test]
fn payment_projection_keeps_the_new_top_hidden_and_does_not_renew_its_identity() {
    for player in [PlayerId::from_index(0), PlayerId::from_index(1)] {
        let (mut game, source, old) = setup(player, 2);
        let card = CardDefinitionBuilder::new(CardId::new(), "Casting top card").build();
        let cover = game.create_object_from_definition(&card, player, Zone::Hand);
        let mut ctx = ExecutionContext::new_default(source, player);
        crate::effects::execute_effect(
            &mut game,
            &Effect::new(MoveToZoneEffect::new(
                ChooseSpec::SpecificObject(cover),
                Zone::Library,
                true,
            )),
            &mut ctx,
        )
        .unwrap();
        let casting = *game.player(player).unwrap().library.last().unwrap();
        let mut preview = game.clone();
        let ids = preview.next_object_id_counter();
        preview.project_spell_for_payment(casting);
        preview.refresh_continuous_state().unwrap();
        assert_eq!(
            *preview.player(player).unwrap().library.last().unwrap(),
            old
        );
        assert!(!preview.static_library_top_visible_during_announcements(player));
        assert_eq!(preview.next_object_id_counter(), ids);
        assert_eq!(
            *game.player(player).unwrap().library.last().unwrap(),
            casting
        );
        preview.validate_zone_consistency().unwrap();
        game.validate_zone_consistency().unwrap();
    }
}

#[test]
fn simultaneous_public_identity_renewals_follow_public_player_order() {
    let player = PlayerId::from_index(0);
    let other = PlayerId::from_index(1);
    let (mut game, source, first) = setup(player, 3);
    let card = CardDefinitionBuilder::new(CardId::new(), "Second player's top").build();
    let second = game.create_object_from_definition(&card, other, Zone::Library);
    game.refresh_continuous_state().unwrap();
    game.phase_out(source);
    let next = game.next_object_id_counter();
    game.phase_in(source);
    let first_new = *game.player(player).unwrap().library.last().unwrap();
    let second_new = *game.player(other).unwrap().library.last().unwrap();
    assert_eq!(first_new.0, next);
    assert_eq!(second_new.0, next + 1);
    assert_ne!(first_new, first);
    assert_ne!(second_new, second);
}

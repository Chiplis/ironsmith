use super::*;
use crate::cards::CardDefinitionBuilder;
use crate::ids::CardId;
use crate::types::CardType;

#[test]
fn current_turn_extra_provenance_tracks_the_selected_turn_after_queue_consumption() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    assert_eq!(game.turn.active_player, alice);
    assert!(!game.turn_store.current_turn_is_extra);

    game.turn_store.extra_turns.push(alice);
    game.next_turn();

    assert_eq!(game.turn.active_player, alice);
    assert!(game.turn_store.extra_turns.is_empty());
    assert!(
        game.turn_store.current_turn_is_extra,
        "the active turn must retain its provenance after it leaves the queue"
    );

    game.next_turn();

    assert_eq!(game.turn.active_player, bob);
    assert!(!game.turn_store.current_turn_is_extra);
}

#[test]
fn skipped_extra_turn_does_not_mark_the_following_normal_turn_as_extra() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn_store.extra_turns.push(alice);
    game.turn_store.skip_next_turn.insert(alice);
    game.next_turn();

    assert_eq!(game.turn.active_player, bob);
    assert!(game.turn_store.extra_turns.is_empty());
    assert!(!game.turn_store.current_turn_is_extra);
}

#[test]
fn grand_melee_keeps_real_extra_turn_provenance_separate_from_lane_scheduling() {
    let mut game = GameState::new(
        vec![
            "Alice".into(),
            "Bob".into(),
            "Charlie".into(),
            "Dana".into(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let seats = (0..4).map(PlayerId::from_index).collect::<Vec<_>>();
    game.restore_grand_melee_with_starting_player(seats, alice)
        .expect("four-player Grand Melee should initialize");

    game.turn_store.extra_turns.push(alice);
    game.next_turn();

    assert_eq!(game.turn.active_player, alice);
    assert!(game.turn_store.current_turn_is_extra);

    game.next_turn();

    assert_eq!(game.turn.active_player, bob);
    assert!(
        !game.turn_store.current_turn_is_extra,
        "Grand Melee's internal lane queue must not label an ordinary turn extra"
    );
}

#[test]
fn name_sticker_character_count_is_case_insensitive_and_additive() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let card = crate::card::CardBuilder::new(CardId::from_raw(70_001), "Sticker Host")
        .card_types(vec![CardType::Enchantment])
        .build();
    let host = game.create_object_from_card(&card, alice, Zone::Battlefield);

    game.put_name_sticker_on_object(host, "Cool");
    game.put_name_sticker_on_object(host, "ROBOT");
    game.put_sticker_on_object(host, KeywordActionKind::ArtSticker);

    assert_eq!(game.name_sticker_character_count_on_object(host, 'o'), 4);
    assert_eq!(game.name_sticker_character_count_on_object(host, 'O'), 4);
    assert_eq!(
        game.sticker_count_on_object(host, KeywordActionKind::Sticker, None),
        3
    );

    let ctx = crate::effects::ExecutionContext::new_default(host, alice);
    let value = crate::effect::Value::NameStickerCharacterCountOnSource {
        character: 'o',
        surface: None,
    };
    assert_eq!(crate::effects::resolve_value(&game, &value, &ctx), Ok(4));
}

#[test]
fn shuffle_slice_marks_irreversible_random_usage() {
    let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let before = game.irreversible_random_count();
    let mut values = vec![1, 2, 3, 4];

    game.shuffle_slice(&mut values);

    assert_eq!(
        game.irreversible_random_count(),
        before + 1,
        "gameplay shuffles should mark the action chain as irreversible"
    );
}

#[test]
fn cloned_hypothetical_state_does_not_burn_real_object_ids() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let first = game.new_object_id();
    let next_before_clone = game.next_object_id_counter();

    let mut hypothetical = game.clone();
    assert_eq!(
        hypothetical.new_object_id(),
        ObjectId::from_raw(next_before_clone)
    );
    assert_eq!(
        hypothetical.new_object_id(),
        ObjectId::from_raw(next_before_clone + 1)
    );

    assert_eq!(game.next_object_id_counter(), next_before_clone);
    assert_eq!(game.new_object_id(), ObjectId::from_raw(first.0 + 1));
}

#[test]
fn cloned_state_shares_battlefield_flags_until_mutation() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let first = ObjectId::from_raw(10_001);
    let second = ObjectId::from_raw(10_002);

    game.keep_damage_marked(first);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));

    hypothetical.keep_damage_marked(second);

    assert!(!Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));
    assert!(game.damage_persists_on(first));
    assert!(!game.damage_persists_on(second));
    assert!(hypothetical.damage_persists_on(first));
    assert!(hypothetical.damage_persists_on(second));
}

#[test]
fn cloned_state_cows_regeneration_shields() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let object = ObjectId::from_raw(10_003);

    game.add_regeneration_shield(object, 2);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));
    assert!(hypothetical.use_regeneration_shield(object));

    assert!(!Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));
    assert_eq!(game.regeneration_shield_count(object), 2);
    assert_eq!(game.regenerated_this_turn_count(object), 0);
    assert_eq!(hypothetical.regeneration_shield_count(object), 1);
    assert_eq!(hypothetical.regenerated_this_turn_count(object), 1);
}

#[test]
fn cloned_state_cows_hot_battlefield_maps() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let tapped = ObjectId::from_raw(10_021);
    let damaged = ObjectId::from_raw(10_022);
    let summoning_sick = ObjectId::from_raw(10_023);
    let monstrous = ObjectId::from_raw(10_024);
    let suspected = ObjectId::from_raw(10_025);

    game.tap(tapped);
    game.mark_damage(damaged, 2);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));

    hypothetical.set_summoning_sick(summoning_sick);
    hypothetical.mark_damage(damaged, 3);
    hypothetical.set_monstrous(monstrous);
    hypothetical.set_suspected(suspected);

    assert!(!Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));
    assert!(game.is_tapped(tapped));
    assert_eq!(game.damage_on(damaged), 2);
    assert!(!game.is_summoning_sick(summoning_sick));
    assert!(!game.is_monstrous(monstrous));
    assert!(!game.is_suspected(suspected));

    assert!(hypothetical.is_tapped(tapped));
    assert_eq!(hypothetical.damage_on(damaged), 5);
    assert!(hypothetical.is_summoning_sick(summoning_sick));
    assert!(hypothetical.is_monstrous(monstrous));
    assert!(hypothetical.is_suspected(suspected));
}

#[test]
fn summoning_sickness_change_stays_local_for_granted_static_abilities() {
    use crate::ability::Ability;
    use crate::card::PowerToughness;
    use crate::static_abilities::{StaticAbility, StaticAbilityId};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let banner = CardDefinitionBuilder::new(CardId::from_raw(10_026), "Haste Banner")
        .card_types(vec![CardType::Artifact])
        .with_ability(Ability::static_ability(StaticAbility::grant_ability(
            ObjectFilter::creature().you_control(),
            StaticAbility::haste(),
        )))
        .build();
    let bear = CardDefinitionBuilder::new(CardId::from_raw(10_027), "Cache Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    game.create_object_from_definition(&banner, alice, Zone::Battlefield);
    let bear_id = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    game.set_summoning_sick(bear_id);
    game.refresh_continuous_state();

    assert!(game.continuous_state_is_clean());
    assert!(game.current_has_static_ability_id(bear_id, StaticAbilityId::Haste));

    game.remove_summoning_sickness(bear_id);

    assert!(
        game.continuous_state_is_clean(),
        "granted static abilities alone should not force global continuous-state invalidation"
    );
}

#[test]
fn turn_context_neutral_effects_reuse_characteristic_cache_across_phase_changes() {
    use crate::card::PowerToughness;
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = CardDefinitionBuilder::new(CardId::from_raw(10_028), "Plain Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let bear = CardDefinitionBuilder::new(CardId::from_raw(10_029), "Cache Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let bear_id = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            source_id,
            alice,
            EffectTarget::AllCreatures,
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
        ));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_characteristics(bear_id)
            .expect("bear should have characteristics")
            .power,
        Some(3)
    );
    let after_first_lookup = game.work_counters();

    game.turn.phase = Phase::FirstMain;
    game.turn.step = None;

    assert_eq!(
        game.calculated_characteristics(bear_id)
            .expect("bear should have characteristics")
            .power,
        Some(3)
    );
    let after_phase_change_lookup = game.work_counters();
    assert_eq!(
        after_phase_change_lookup.characteristics_full_recomputes,
        after_first_lookup.characteristics_full_recomputes,
        "turn-context-neutral effects should not force a new characteristic pass"
    );
}

#[test]
fn absent_payment_restriction_skips_layered_battlefield_scans() {
    use crate::card::PowerToughness;
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::from_raw(10_032), "Layered Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let ids = (0..96)
        .map(|_| game.create_object_from_definition(&creature, alice, Zone::Battlefield))
        .collect::<Vec<_>>();
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            ids[0],
            alice,
            EffectTarget::AllCreatures,
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
        ));
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            ids[1],
            alice,
            EffectTarget::AllCreatures,
            Modification::AddAbility(StaticAbility::flying()),
        ));
    game.refresh_continuous_state();
    let before = game.work_counters();

    for _ in 0..4 {
        assert!(!game.player_cant_pay_life_to_cast_or_activate(alice));
        assert!(!game.player_cant_sacrifice_nonland_to_cast_or_activate(alice));
    }

    let after = game.work_counters();
    assert_eq!(
        after.characteristics_full_recomputes, before.characteristics_full_recomputes,
        "an absent restriction should not calculate every permanent's characteristics"
    );
    assert_eq!(
        after.dependency_sorts, before.dependency_sorts,
        "an absent restriction should not run per-permanent dependency sorts"
    );
}

#[test]
fn payment_restriction_presence_cache_invalidates_for_printed_ability_changes() {
    use crate::ability::Ability;
    use crate::card::PowerToughness;
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::from_raw(10_033), "Restriction Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    game.refresh_continuous_state();
    assert!(!game.player_cant_pay_life_to_cast_or_activate(alice));

    game.object_mut(source)
        .expect("restriction source should exist")
        .abilities_mut()
        .push(Ability::static_ability(
            StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
        ));

    assert!(game.player_cant_pay_life_to_cast_or_activate(alice));
    assert!(game.player_cant_sacrifice_nonland_to_cast_or_activate(alice));
}

#[test]
fn continuously_granted_payment_restriction_uses_layered_characteristics() {
    use crate::card::PowerToughness;
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::from_raw(10_034), "Granted Restriction")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    game.refresh_continuous_state();
    assert!(!game.player_cant_pay_life_to_cast_or_activate(alice));

    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Specific(target),
            Modification::AddAbility(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
            ),
        ));

    assert!(game.player_cant_pay_life_to_cast_or_activate(alice));
    assert!(game.player_cant_sacrifice_nonland_to_cast_or_activate(alice));
}

#[test]
fn temporary_static_grant_participates_in_sparse_cant_scan() {
    use crate::card::PowerToughness;
    use crate::static_abilities::{StaticAbility, StaticAbilityId};
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::from_raw(10_035), "Temporary Restriction")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
        target,
        StaticAbilityId::CantBlock,
        Some(StaticAbility::cant_block()),
    );
    game.refresh_continuous_state();

    game.update_cant_effects();

    assert!(
        !game.can_block(target),
        "a temporary can't-block grant must populate the cant tracker"
    );
}

#[test]
fn active_level_grant_participates_in_sparse_cant_scan() {
    use crate::ability::LevelAbility;
    use crate::card::PowerToughness;
    use crate::object::CounterType;
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::from_raw(10_036), "Leveled Restriction")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_level_abilities(vec![
            LevelAbility::new(1, None).with_ability(StaticAbility::cant_block()),
        ])
        .build();
    let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    let _ = game.add_counters(target, CounterType::Level, 1);
    game.refresh_continuous_state();

    game.update_cant_effects();

    assert!(
        !game.can_block(target),
        "an active level-granted can't-block ability must populate the cant tracker"
    );
}

#[test]
fn set_abilities_removal_uses_layered_cant_scan() {
    use crate::ability::Ability;
    use crate::card::PowerToughness;
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::static_abilities::StaticAbility;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let creature = CardDefinitionBuilder::new(CardId::from_raw(10_037), "Replaced Restriction")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .with_ability(Ability::static_ability(StaticAbility::cant_block()))
        .build();
    let target = game.create_object_from_definition(&creature, alice, Zone::Battlefield);
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            target,
            alice,
            EffectTarget::Specific(target),
            Modification::SetAbilities(vec![Ability::static_ability(StaticAbility::flying())]),
        ));
    game.refresh_continuous_state();

    game.update_cant_effects();

    assert!(
        game.can_block(target),
        "SetAbilities must be able to remove a printed can't-block restriction"
    );
}

#[test]
fn active_player_filters_still_invalidate_characteristic_cache() {
    use crate::card::PowerToughness;
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = CardDefinitionBuilder::new(CardId::from_raw(10_030), "Active Anthem")
        .card_types(vec![CardType::Enchantment])
        .build();
    let bear = CardDefinitionBuilder::new(CardId::from_raw(10_031), "Cache Bear")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let source_id = game.create_object_from_definition(&source, alice, Zone::Battlefield);
    let bear_id = game.create_object_from_definition(&bear, alice, Zone::Battlefield);
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            source_id,
            alice,
            EffectTarget::Filter(ObjectFilter::creature().controlled_by(PlayerFilter::Active)),
            Modification::ModifyPowerToughness {
                power: 1,
                toughness: 1,
            },
        ));
    game.refresh_continuous_state();

    assert_eq!(
        game.calculated_characteristics(bear_id)
            .expect("bear should have characteristics")
            .power,
        Some(3)
    );

    game.turn.active_player = bob;

    assert_eq!(
        game.calculated_characteristics(bear_id)
            .expect("bear should have characteristics")
            .power,
        Some(2),
        "active-player-sensitive filters must recompute after active player changes"
    );
}

#[test]
fn cloned_state_cows_extended_battlefield_flags() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = crate::ids::PlayerId::from_index(0);
    let renowned = ObjectId::from_raw(10_004);
    let face_down = ObjectId::from_raw(10_005);
    let transformed = ObjectId::from_raw(10_006);
    let phased = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::new(), "Phasing Probe")
            .card_types(vec![CardType::Artifact])
            .build(),
        alice,
        Zone::Battlefield,
    );

    game.set_renowned(renowned);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));

    hypothetical.set_face_down(face_down);
    hypothetical.set_manifested(face_down);
    hypothetical.mark_transformed(transformed);
    hypothetical.phase_out(phased);

    assert!(!Arc::ptr_eq(
        &game.battlefield_flags,
        &hypothetical.battlefield_flags
    ));
    assert!(game.is_renowned(renowned));
    assert!(!game.is_face_down(face_down));
    assert!(!game.is_manifested(face_down));
    assert_eq!(game.transform_count(transformed), 0);
    assert!(!game.is_phased_out(phased));

    assert!(hypothetical.is_renowned(renowned));
    assert!(hypothetical.is_face_down(face_down));
    assert!(hypothetical.is_manifested(face_down));
    assert_eq!(hypothetical.transform_count(transformed), 1);
    assert!(hypothetical.is_phased_out(phased));
    assert_eq!(
        hypothetical.phased_out_ids().collect::<Vec<_>>(),
        vec![phased]
    );
}

#[test]
fn auspicious_starrix_mutation_count_tracks_recorded_mutate_events() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = crate::ids::PlayerId::from_index(0);
    let starrix = ObjectId::from_raw(10_008);

    for _ in 0..2 {
        game.record_turn_history_event(&crate::triggers::TriggerEvent::new(
            crate::events::other::MutatedEvent::new(starrix, alice),
            crate::provenance::ProvNodeId::default(),
        ));
    }
    assert_eq!(game.mutation_count(starrix), 2);

    game.clear_battlefield_state(starrix);
    assert_eq!(game.mutation_count(starrix), 0);
}

#[test]
fn cloned_state_cows_cast_permission_flags() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let madness = ObjectId::from_raw(10_008);
    let foretold = ObjectId::from_raw(10_009);
    let adventure = ObjectId::from_raw(10_010);

    game.set_madness_exiled(madness);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.cast_permission_flags,
        &hypothetical.cast_permission_flags
    ));

    hypothetical.set_foretold(foretold);
    hypothetical.set_adventure_exiled(adventure);

    assert!(!Arc::ptr_eq(
        &game.cast_permission_flags,
        &hypothetical.cast_permission_flags
    ));
    assert!(game.is_madness_exiled(madness));
    assert!(!game.is_foretold(foretold));
    assert!(!game.is_adventure_exiled(adventure));
    assert!(hypothetical.is_madness_exiled(madness));
    assert!(hypothetical.is_foretold(foretold));
    assert!(hypothetical.is_adventure_exiled(adventure));
}

#[test]
fn cloned_state_cows_exile_tracking() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = ObjectId::from_raw(10_011);
    let exiled = ObjectId::from_raw(10_012);
    let plotted = ObjectId::from_raw(10_013);
    let imprinted = ObjectId::from_raw(10_014);

    game.add_exiled_with_source_link(source, exiled);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.exile_tracking,
        &hypothetical.exile_tracking
    ));

    hypothetical.set_plotted_on_turn(plotted, alice, 7);
    hypothetical.imprint_card(source, imprinted);
    hypothetical.grant_face_down_exile_view(exiled, alice);
    hypothetical.mark_return_exiled_when_source_leaves(source);
    let group_id = hypothetical.create_linked_exile_group(
        vec![StableId::from(source)],
        Zone::Battlefield,
        false,
    );

    assert!(!Arc::ptr_eq(
        &game.exile_tracking,
        &hypothetical.exile_tracking
    ));
    assert_eq!(game.get_exiled_with_source_links(source), &[exiled]);
    assert_eq!(game.plotted_by(plotted), None);
    assert!(!game.has_imprinted_cards(source));
    assert!(!game.can_player_look_at_face_down_exiled_card(exiled, alice));
    assert_eq!(
        game.return_exiled_when_source_leaves_ids()
            .copied()
            .collect::<Vec<_>>(),
        Vec::<ObjectId>::new()
    );

    assert_eq!(hypothetical.get_exiled_with_source_links(source), &[exiled]);
    assert_eq!(hypothetical.plotted_by(plotted), Some(alice));
    assert_eq!(hypothetical.plotted_turn(plotted), Some(7));
    assert_eq!(hypothetical.get_imprinted_cards(source), &[imprinted]);
    assert!(hypothetical.can_player_look_at_face_down_exiled_card(exiled, alice));
    assert_eq!(
        hypothetical
            .return_exiled_when_source_leaves_ids()
            .copied()
            .collect::<Vec<_>>(),
        vec![source]
    );
    assert!(hypothetical.take_linked_exile_group(group_id).is_some());
}

#[test]
fn cloned_state_cows_object_annotations() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = ObjectId::from_raw(10_015);
    let other_source = ObjectId::from_raw(10_016);

    assert_eq!(game.note_life_total_for_source(source, alice), Some(20));
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.object_annotations,
        &hypothetical.object_annotations
    ));

    assert_eq!(
        hypothetical.note_life_total_for_source(other_source, alice),
        Some(20)
    );

    assert!(!Arc::ptr_eq(
        &game.object_annotations,
        &hypothetical.object_annotations
    ));
    assert_eq!(game.noted_life_total_for_source(source), Some(20));
    assert_eq!(game.noted_life_total_for_source(other_source), None);
    assert_eq!(hypothetical.noted_life_total_for_source(source), Some(20));
    assert_eq!(
        hypothetical.noted_life_total_for_source(other_source),
        Some(20)
    );
}

#[test]
fn cloned_state_cows_commander_tracking() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let commander = ObjectId::from_raw(10_017);

    game.set_as_commander(commander, alice);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.commander_tracking,
        &hypothetical.commander_tracking
    ));

    hypothetical.record_commander_cast_from_command_zone(commander);
    hypothetical.decline_commander_command_zone_move(commander);

    assert!(!Arc::ptr_eq(
        &game.commander_tracking,
        &hypothetical.commander_tracking
    ));
    assert!(game.commander_objects().contains(&commander));
    assert!(hypothetical.commander_objects().contains(&commander));
    assert_eq!(game.commander_cast_count(commander), 0);
    assert!(!game.commander_command_zone_move_declined(commander));
    assert_eq!(hypothetical.commander_cast_count(commander), 1);
    assert!(hypothetical.commander_command_zone_move_declined(commander));
}

#[test]
fn cloned_state_cows_combat_transients() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = ObjectId::from_raw(10_018);
    let soulbond_left = ObjectId::from_raw(10_019);
    let soulbond_right = ObjectId::from_raw(10_020);

    game.record_ninjutsu_attack_target(source, crate::combat_state::AttackTarget::Player(bob));
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.combat_transients,
        &hypothetical.combat_transients
    ));

    hypothetical.record_sneak_attack_target(source, crate::combat_state::AttackTarget::Player(bob));
    hypothetical.record_combat_damage_player_batch_hit(source, bob);
    hypothetical.mark_speed_increase_triggered_this_turn(alice);
    hypothetical
        .combat_transients_mut()
        .soulbond_pairs
        .insert(soulbond_left, soulbond_right);

    assert!(!Arc::ptr_eq(
        &game.combat_transients,
        &hypothetical.combat_transients
    ));
    assert_eq!(
        game.last_ninjutsu_attack_target(source).cloned(),
        Some(crate::combat_state::AttackTarget::Player(bob))
    );
    assert_eq!(game.last_sneak_attack_target(source), None);
    assert!(game.combat_damage_player_batch_hits().is_empty());
    assert!(!game.speed_increase_triggered_this_turn(alice));
    assert!(!game.soulbond_pairs().contains_key(&soulbond_left));

    assert_eq!(
        hypothetical.last_ninjutsu_attack_target(source).cloned(),
        Some(crate::combat_state::AttackTarget::Player(bob))
    );
    assert_eq!(
        hypothetical.last_sneak_attack_target(source).cloned(),
        Some(crate::combat_state::AttackTarget::Player(bob))
    );
    assert_eq!(
        hypothetical.combat_damage_player_batch_hits(),
        &[(source, bob)]
    );
    assert!(hypothetical.speed_increase_triggered_this_turn(alice));
    assert_eq!(
        hypothetical.soulbond_pairs().get(&soulbond_left).copied(),
        Some(soulbond_right)
    );
}

#[test]
fn cloned_state_cows_auxiliary_tracking() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.set_draft_noted_highest_number(alice, "Cogwork Librarian", 1);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(
        &game.auxiliary_tracking,
        &hypothetical.auxiliary_tracking
    ));

    hypothetical.set_draft_noted_highest_number(alice, "Lore Seeker", 2);
    hypothetical.create_hidden_card_placeholder(alice, Zone::Library, 7, "commitment".to_string());
    hypothetical.set_active_dungeon(
        alice,
        crate::dungeon::ActiveDungeonProgress::new("Lost Mine of Phandelver", "Cave Entrance"),
    );
    hypothetical.record_completed_dungeon(alice, "Tomb of Annihilation");
    hypothetical.add_player_control(
        bob,
        alice,
        PlayerControlStart::Immediate,
        PlayerControlDuration::UntilEndOfTurn,
        None,
    );
    hypothetical.add_scoped_player_control(bob, alice, None);
    hypothetical.add_combat_choice_control(bob, true, false);

    assert!(!Arc::ptr_eq(
        &game.auxiliary_tracking,
        &hypothetical.auxiliary_tracking
    ));
    assert_eq!(
        game.draft_noted_highest_number(alice, "Cogwork Librarian"),
        1
    );
    assert_eq!(game.draft_noted_highest_number(alice, "Lore Seeker"), 0);
    assert_eq!(game.hidden_card_entries().count(), 0);
    assert!(game.active_dungeon(alice).is_none());
    assert!(game.completed_dungeons(alice).is_empty());
    assert_eq!(game.controlling_player_for(alice), alice);
    assert_eq!(game.combat_choice_controller_for_attackers(), None);

    assert_eq!(
        hypothetical.draft_noted_highest_number(alice, "Cogwork Librarian"),
        1
    );
    assert_eq!(
        hypothetical.draft_noted_highest_number(alice, "Lore Seeker"),
        2
    );
    assert_eq!(hypothetical.hidden_card_entries().count(), 1);
    assert_eq!(
        hypothetical
            .active_dungeon(alice)
            .map(|progress| progress.room_name.as_str()),
        Some("Cave Entrance")
    );
    assert_eq!(
        hypothetical.completed_dungeons(alice),
        &["Tomb of Annihilation".to_string()]
    );
    assert_eq!(hypothetical.controlling_player_for(alice), bob);
    assert_eq!(
        hypothetical.combat_choice_controller_for_attackers(),
        Some(bob)
    );
}

#[test]
fn face_down_exile_visibility_follows_current_player_control() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = CardDefinitionBuilder::new(CardId::from_raw(98_003), "Exile Probe").build();
    let exiled = game.create_object_from_definition(&card, alice, Zone::Exile);
    game.set_face_down(exiled);
    game.grant_face_down_exile_view(exiled, alice);

    assert!(game.can_player_look_at_face_down_exiled_card(exiled, alice));
    assert!(!game.can_player_look_at_face_down_exiled_card(exiled, bob));

    let control_token = game.add_scoped_player_control(bob, alice, None);
    assert!(game.can_player_look_at_face_down_exiled_card(exiled, alice));
    assert!(game.can_player_look_at_face_down_exiled_card(exiled, bob));

    game.remove_scoped_player_control(control_token);
    assert!(game.can_player_look_at_face_down_exiled_card(exiled, alice));
    assert!(!game.can_player_look_at_face_down_exiled_card(exiled, bob));
}

#[test]
fn cloned_state_cows_choice_store() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source = ObjectId::from_raw(10_026);

    game.set_chosen_color(source, crate::color::Color::Blue);
    let mut hypothetical = game.clone();

    assert!(Arc::ptr_eq(&game.choice_store, &hypothetical.choice_store));

    hypothetical.set_chosen_player(source, alice);
    hypothetical.set_chosen_card_type(source, CardType::Creature);
    hypothetical.set_chosen_named_option(source, "left".to_string());
    hypothetical.record_ability_mode_choice(source, 0, 1, false);

    assert!(!Arc::ptr_eq(&game.choice_store, &hypothetical.choice_store));
    assert_eq!(game.chosen_color(source), Some(crate::color::Color::Blue));
    assert_eq!(game.chosen_player(source), None);
    assert_eq!(game.chosen_card_type(source), None);
    assert_eq!(game.chosen_named_option(source), None);
    assert!(!game.ability_mode_was_chosen(source, 0, 1, false));

    assert_eq!(
        hypothetical.chosen_color(source),
        Some(crate::color::Color::Blue)
    );
    assert_eq!(hypothetical.chosen_player(source), Some(alice));
    assert_eq!(
        hypothetical.chosen_card_type(source),
        Some(CardType::Creature)
    );
    assert_eq!(hypothetical.chosen_named_option(source), Some("left"));
    assert!(hypothetical.ability_mode_was_chosen(source, 0, 1, false));
}

#[test]
fn crypto_audit_journal_records_hidden_library_to_hand_move() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let hidden =
        game.create_hidden_card_placeholder(alice, Zone::Library, 7, "alice-slot-7".to_string());
    let checkpoint = game.crypto_audit_checkpoint();

    let drawn = game.draw_cards(alice, 1);

    assert_eq!(drawn.len(), 1);
    let hand_id = drawn[0];
    let operations = game.crypto_audit_operations_since(checkpoint);
    assert!(operations.iter().any(|operation| {
        matches!(
            operation,
            HiddenInfoOperation::HiddenMove {
                owner,
                old_object_id,
                new_object_id,
                from,
                to,
                slot,
                commitment,
            } if *owner == alice
                && *old_object_id == hidden
                && *new_object_id == hand_id
                && *from == Zone::Library
                && *to == Zone::Hand
                && *slot == 7
                && commitment == "alice-slot-7"
        )
    }));
}

#[test]
fn ui_zone_transition_feed_records_central_moves() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let spell = CardDefinitionBuilder::new(CardId::from_raw(42), "Stack Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let hand_id = game.create_object_from_definition(&spell, alice, Zone::Hand);

    let stack_id = game
        .move_object_by_effect(hand_id, Zone::Stack)
        .expect("spell should move to stack");
    let graveyard_id = game
        .move_object_by_effect(stack_id, Zone::Graveyard)
        .expect("spell should move to graveyard");

    let transitions: Vec<_> = game.ui_zone_transitions().collect();
    assert!(
        transitions.iter().any(|transition| {
            transition.old_object_id == hand_id
                && transition.new_object_id == stack_id
                && transition.from == Zone::Hand
                && transition.to == Zone::Stack
        }),
        "expected hand-to-stack transition, got {transitions:?}"
    );
    assert!(
        transitions.iter().any(|transition| {
            transition.old_object_id == stack_id
                && transition.new_object_id == graveyard_id
                && transition.from == Zone::Stack
                && transition.to == Zone::Graveyard
        }),
        "expected stack-to-graveyard transition, got {transitions:?}"
    );
}

#[test]
fn ordinary_return_from_exile_uses_transform_like_default_face() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let front_id = CardId::from_raw(79_400);
    let back_id = CardId::from_raw(79_401);

    let mut front = CardDefinitionBuilder::new(front_id, "Default Face Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(crate::card::PowerToughness::fixed(2, 2))
        .build();
    front.card.other_face = Some(back_id);
    front.card.other_face_name = Some("Back Face Land".to_string());
    front.card.linked_face_layout = LinkedFaceLayout::TransformLike;

    let mut back = CardDefinitionBuilder::new(back_id, "Back Face Land")
        .card_types(vec![CardType::Land])
        .build();
    back.card.other_face = Some(front_id);
    back.card.other_face_name = Some("Default Face Creature".to_string());
    back.card.linked_face_layout = LinkedFaceLayout::TransformLike;

    game.register_linked_face_definition(&front);
    game.register_linked_face_definition(&back);

    let back_permanent = game.create_object_from_definition(&back, alice, Zone::Battlefield);
    let exiled_back = game
        .move_object_by_effect(back_permanent, Zone::Exile)
        .expect("back face should move to exile");
    let returned_back = game
        .move_object_by_effect(exiled_back, Zone::Battlefield)
        .expect("back face should return to the battlefield");
    let returned = game
        .object(returned_back)
        .expect("returned permanent should exist");
    assert_eq!(returned.name, "Default Face Creature");
    assert!(returned.card_types.contains(&CardType::Creature));
    assert!(!returned.card_types.contains(&CardType::Land));

    let front_permanent = game.create_object_from_definition(&front, alice, Zone::Battlefield);
    let exiled_front = game
        .move_object_by_effect(front_permanent, Zone::Exile)
        .expect("front face should move to exile");
    let returned_front = game
        .move_object_by_effect(exiled_front, Zone::Battlefield)
        .expect("front face should return to the battlefield");
    assert_eq!(
        game.object(returned_front)
            .expect("returned front face should exist")
            .name,
        "Default Face Creature"
    );
}

#[test]
fn crypto_audit_journal_records_library_shuffle() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    game.create_hidden_card_placeholder(alice, Zone::Library, 0, "slot-0".to_string());
    game.create_hidden_card_placeholder(alice, Zone::Library, 1, "slot-1".to_string());
    game.create_hidden_card_placeholder(alice, Zone::Library, 2, "slot-2".to_string());
    let before_order = game.player(alice).expect("alice").library.clone();
    let before_random = game.irreversible_random_count();
    let checkpoint = game.crypto_audit_checkpoint();

    game.shuffle_player_library(alice);

    let after_order = game.player(alice).expect("alice").library.clone();
    let operations = game.crypto_audit_operations_since(checkpoint);
    assert!(operations.iter().any(|operation| {
        matches!(
            operation,
            HiddenInfoOperation::LibraryShuffle {
                player,
                before_order: recorded_before,
                after_order: recorded_after,
                random_count_before,
                random_count_after,
            } if *player == alice
                && *recorded_before == before_order
                && *recorded_after == after_order
                && *random_count_before == before_random
                && *random_count_after == before_random + 1
        )
    }));
}

#[test]
fn transcript_library_shuffle_order_is_localized_to_live_pre_shuffle_order() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let first = game.create_hidden_card_placeholder(alice, Zone::Library, 0, "slot-0".to_string());
    let second = game.create_hidden_card_placeholder(alice, Zone::Library, 1, "slot-1".to_string());
    let third = game.create_hidden_card_placeholder(alice, Zone::Library, 2, "slot-2".to_string());
    let transcript_before = vec![
        ObjectId::from_raw(10_001),
        ObjectId::from_raw(10_002),
        ObjectId::from_raw(10_003),
    ];
    let transcript_after = vec![
        transcript_before[2],
        transcript_before[0],
        transcript_before[1],
    ];

    game.queue_transcript_library_shuffle_order(alice, transcript_before, transcript_after);
    game.shuffle_player_library(alice);

    assert_eq!(
        game.player(alice).expect("alice").library,
        vec![third, first, second],
        "queued transcript order should map by before-order position onto live object ids"
    );
}

#[test]
fn stack_to_battlefield_preserves_cast_x_value_for_permanent() {
    use crate::card::CardBuilder;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(99), "X Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let stack_id = game.create_object_from_card(&card, alice, Zone::Stack);
    game.object_mut(stack_id).expect("stack object").x_value = Some(3);

    let battlefield_id = game
        .move_object_by_effect(stack_id, Zone::Battlefield)
        .expect("creature should enter");

    assert_eq!(
        game.object(battlefield_id).expect("permanent").x_value,
        Some(3)
    );

    let graveyard_id = game
        .move_object_by_effect(battlefield_id, Zone::Graveyard)
        .expect("permanent should move to graveyard");
    assert_eq!(game.object(graveyard_id).expect("card").x_value, None);
}

#[test]
fn instant_and_sorcery_cards_remain_in_their_previous_zone_instead_of_entering() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let instant = CardDefinitionBuilder::new(CardId::from_raw(79_410), "Blocked Instant")
        .card_types(vec![CardType::Instant])
        .build();
    let sorcery = CardDefinitionBuilder::new(CardId::from_raw(79_411), "Blocked Sorcery")
        .card_types(vec![CardType::Sorcery])
        .build();
    let instant_id = game.create_object_from_definition(&instant, alice, Zone::Hand);
    let sorcery_id = game.create_object_from_definition(&sorcery, alice, Zone::Graveyard);

    assert_eq!(
        game.move_object_by_effect(instant_id, Zone::Battlefield),
        None
    );
    assert_eq!(
        game.move_object_by_game_rule(sorcery_id, Zone::Battlefield),
        None
    );
    assert_eq!(
        game.object(instant_id).expect("instant remains").zone,
        Zone::Hand
    );
    assert_eq!(
        game.object(sorcery_id).expect("sorcery remains").zone,
        Zone::Graveyard
    );
    assert!(!game.battlefield.contains(&instant_id));
    assert!(!game.battlefield.contains(&sorcery_id));
}

#[test]
fn crypto_audit_journal_records_hidden_library_reorder() {
    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bottom = game.create_hidden_card_placeholder(alice, Zone::Library, 0, "slot-0".to_string());
    let top = game.create_hidden_card_placeholder(alice, Zone::Library, 1, "slot-1".to_string());
    let checkpoint = game.crypto_audit_checkpoint();

    assert!(game.set_player_library_order_with_audit(alice, vec![top, bottom], "test reorder",));

    let operations = game.crypto_audit_operations_since(checkpoint);
    assert!(operations.iter().any(|operation| {
        matches!(
            operation,
            HiddenInfoOperation::LibraryReorder {
                player,
                before_order,
                after_order,
                reason,
            } if *player == alice
                && *before_order == vec![bottom, top]
                && *after_order == vec![top, bottom]
                && reason == "test reorder"
        )
    }));
}

#[test]
fn production_effects_do_not_mutate_player_library_directly() {
    fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
    }

    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rs_files(&manifest_dir.join("src/effects"), &mut files);
    collect_rs_files(&manifest_dir.join("src/events"), &mut files);

    let forbidden = [
        ".library.push(",
        ".library.insert(",
        ".library.remove(",
        ".library.retain(",
        ".library.splice(",
        "player.library =",
    ];
    let mut violations = Vec::new();
    for path in files {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .unwrap_or(source.as_str());
        for (index, line) in production_source.lines().enumerate() {
            if forbidden.iter().any(|pattern| line.contains(pattern)) {
                violations.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(&manifest_dir).unwrap_or(&path).display(),
                    index + 1,
                    line.trim()
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "production hidden-library code must use GameState audited order helpers:\n{}",
        violations.join("\n")
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn creatures_controlled_by_includes_animated_land() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::definitions::basic_mountain;
    use crate::effect::Effect;
    use crate::effects::EarthbendEffect;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::CardId;
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(200), "Kyoshi")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let land_id = game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);

    let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
    let mut ctx = ExecutionContext::new_default(source_id, alice);
    execute_effect(&mut game, &effect, &mut ctx).expect("earthbend should resolve");

    let creatures = game.creatures_controlled_by(alice);
    assert!(
        creatures.contains(&land_id),
        "animated lands should be counted by creature-control helpers"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn current_characteristic_helpers_reflect_animation() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::definitions::basic_mountain;
    use crate::effect::Effect;
    use crate::effects::EarthbendEffect;
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbilityId;
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let source_card = CardBuilder::new(CardId::from_raw(201), "Kyoshi")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let land_id = game.create_object_from_definition(&basic_mountain(), alice, Zone::Battlefield);

    let effect = Effect::new(EarthbendEffect::new(ChooseSpec::SpecificObject(land_id), 8));
    let mut ctx = ExecutionContext::new_default(source_id, alice);
    execute_effect(&mut game, &effect, &mut ctx).expect("earthbend should resolve");

    assert!(game.current_is_creature(land_id));
    assert!(
        game.current_card_types(land_id)
            .is_some_and(|types| types.contains(&CardType::Creature))
    );
    assert_eq!(game.current_power(land_id), Some(8));
    assert_eq!(game.current_toughness(land_id), Some(8));
    assert!(game.current_has_static_ability_id(land_id, StaticAbilityId::Haste));
}

#[test]
fn current_subtypes_reflect_graveyard_effects_and_changeling() {
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::Subtype;
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let _beacon_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(202), "Graveyard Beacon")
            .card_types(vec![CardType::Artifact])
            .with_ability(Ability::static_ability(StaticAbility::add_subtypes(
                ObjectFilter::default()
                    .in_zone(Zone::Graveyard)
                    .owned_by(PlayerFilter::You)
                    .with_type(CardType::Creature),
                vec![Subtype::Wizard],
            )))
            .build(),
        alice,
        Zone::Battlefield,
    );

    let graveyard_creature_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(203), "Vanilla Bear")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build(),
        alice,
        Zone::Graveyard,
    );

    assert!(game.current_has_subtype(graveyard_creature_id, Subtype::Wizard));
    assert!(
        game.current_subtypes(graveyard_creature_id)
            .is_some_and(|subtypes| subtypes.contains(&Subtype::Wizard))
    );

    let changeling_spell_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(204), "Velis Probe")
            .card_types(vec![CardType::Kindred, CardType::Instant])
            .with_ability(Ability::static_ability(StaticAbility::changeling()))
            .build(),
        alice,
        Zone::Graveyard,
    );

    assert!(game.current_has_subtype(changeling_spell_id, Subtype::Wizard));
    assert!(game.current_has_subtype(changeling_spell_id, Subtype::Elf));
    assert!(
        game.current_subtypes(changeling_spell_id)
            .is_some_and(|subtypes| subtypes.contains(&Subtype::Wizard))
    );
}

#[test]
fn battlefield_changeling_uses_layered_type_effects() {
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::{ContinuousEffect, EffectTarget, Modification};
    use crate::effect::Until;
    use crate::static_abilities::{StaticAbility, StaticAbilityId};
    use crate::types::{Subtype, SubtypeFamily};
    use crate::zone::Zone;

    let mut game = GameState::new(vec!["Alice".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let source_id = game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(205), "Layer Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let changeling_id = game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(206), "Changeling Probe")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Shapeshifter])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::changeling()))
            .build(),
        alice,
        Zone::Battlefield,
    );

    assert!(game.current_has_subtype(changeling_id, Subtype::Goblin));

    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::new(
            source_id,
            alice,
            EffectTarget::Specific(changeling_id),
            Modification::RemoveAllSubtypesOfFamily(SubtypeFamily::Creature),
        )
        .until(Until::EndOfTurn),
    );

    assert!(!game.current_has_subtype(changeling_id, Subtype::Shapeshifter));
    assert!(game.current_has_static_ability_id(changeling_id, StaticAbilityId::Changeling));

    let mut ability_loss_game = GameState::new(vec!["Alice".to_string()], 20);
    let source_id = ability_loss_game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(207), "Ability Loss Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build(),
        alice,
        Zone::Battlefield,
    );
    let changeling_id = ability_loss_game.create_object_from_definition(
        &CardDefinitionBuilder::new(CardId::from_raw(208), "Ability Loss Changeling")
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Shapeshifter])
            .power_toughness(PowerToughness::fixed(2, 2))
            .with_ability(Ability::static_ability(StaticAbility::changeling()))
            .build(),
        alice,
        Zone::Battlefield,
    );
    ability_loss_game
        .effect_store
        .continuous_effects
        .add_effect(
            ContinuousEffect::new(
                source_id,
                alice,
                EffectTarget::Specific(changeling_id),
                Modification::RemoveAllAbilities,
            )
            .until(Until::EndOfTurn),
        );

    assert!(ability_loss_game.current_has_subtype(changeling_id, Subtype::Goblin));
    assert!(
        !ability_loss_game
            .current_has_static_ability_id(changeling_id, StaticAbilityId::Changeling)
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn azusa_after_first_land_grants_two_remaining_land_plays() {
    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);

    let azusa = CardDefinitionBuilder::new(CardId::new(), "Azusa, Lost but Seeking")
        .card_types(vec![CardType::Creature])
        .parse_text("You may play two additional lands on each of your turns.")
        .expect("Azusa text should parse");

    game.player_mut(alice)
        .expect("alice should exist")
        .record_land_play();
    assert!(
        !game
            .player(alice)
            .expect("alice should exist")
            .can_play_land(),
        "a player who already played a land should be out of normal land plays"
    );

    game.create_object_from_definition(&azusa, alice, Zone::Battlefield);
    game.refresh_continuous_state();

    assert_eq!(
        game.player(alice)
            .expect("alice should exist")
            .land_plays_per_turn,
        3,
        "Azusa should raise the land-play limit to three total for the turn"
    );
    assert!(
        game.player(alice)
            .expect("alice should exist")
            .can_play_land(),
        "after Azusa enters, the player should still have two land plays remaining"
    );

    game.player_mut(alice)
        .expect("alice should exist")
        .record_land_play();
    assert!(
        game.player(alice)
            .expect("alice should exist")
            .can_play_land(),
        "the second land play after Azusa should still leave one more available"
    );

    game.player_mut(alice)
        .expect("alice should exist")
        .record_land_play();
    assert!(
        !game
            .player(alice)
            .expect("alice should exist")
            .can_play_land(),
        "the third total land play should exhaust Azusa's extra allowance"
    );
}

#[test]
fn filtered_activation_mana_spend_permissions_match_allowed_sources() {
    use crate::card::CardBuilder;
    use crate::effect::ManaSpendPermission;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let creature_card = CardBuilder::new(CardId::from_raw(300), "Test Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let artifact_card = CardBuilder::new(CardId::from_raw(301), "Test Artifact")
        .card_types(vec![CardType::Artifact])
        .build();

    let alice_creature = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let bob_creature = game.create_object_from_card(&creature_card, bob, Zone::Battlefield);
    let alice_artifact = game.create_object_from_card(&artifact_card, alice, Zone::Battlefield);

    game.effect_store
        .mana_spend_effects
        .permissions
        .push(ActiveManaSpendPermission {
            permission: ManaSpendPermission::any_color_for_activation(
                crate::target::PlayerFilter::You,
                crate::target::ObjectFilter::creature().you_control(),
            ),
            controller: alice,
            source: ManaSpendPermissionSource::StaticAbility,
        });

    assert!(game.can_spend_mana_as_any_color(alice, Some(alice_creature)));
    assert!(!game.can_spend_mana_as_any_color(alice, Some(alice_artifact)));
    assert!(!game.can_spend_mana_as_any_color(alice, Some(bob_creature)));
    assert!(!game.can_spend_mana_as_any_color(bob, Some(bob_creature)));
}

#[test]
fn source_filtered_mana_spend_permissions_match_mana_sources() {
    use crate::card::CardBuilder;
    use crate::effect::ManaSpendPermission;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let creature_card = CardBuilder::new(CardId::from_raw(302), "Test Creature")
        .card_types(vec![CardType::Creature])
        .build();
    let snow_land_card = CardBuilder::new(CardId::from_raw(303), "Snow Land")
        .supertypes(vec![crate::types::Supertype::Snow])
        .card_types(vec![CardType::Land])
        .build();
    let land_card = CardBuilder::new(CardId::from_raw(304), "Regular Land")
        .card_types(vec![CardType::Land])
        .build();

    let alice_creature = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let alice_snow_land = game.create_object_from_card(&snow_land_card, alice, Zone::Battlefield);
    let alice_land = game.create_object_from_card(&land_card, alice, Zone::Battlefield);

    game.effect_store
        .mana_spend_effects
        .permissions
        .push(ActiveManaSpendPermission {
            permission: ManaSpendPermission::any_color_for_activation(
                crate::target::PlayerFilter::You,
                crate::target::ObjectFilter::creature().you_control(),
            )
            .with_mana_source_filter(
                crate::target::ObjectFilter::default()
                    .with_supertype(crate::types::Supertype::Snow),
            ),
            controller: alice,
            source: ManaSpendPermissionSource::StaticAbility,
        });

    assert!(!game.can_spend_mana_as_any_color(alice, Some(alice_creature)));
    assert!(game.can_spend_mana_as_any_color_from_mana_source(
        alice,
        Some(alice_creature),
        alice_snow_land
    ));
    assert!(!game.can_spend_mana_as_any_color_from_mana_source(
        alice,
        Some(alice_creature),
        alice_land
    ));
    assert!(!game.can_spend_mana_as_any_color_from_mana_source(
        alice,
        Some(alice_land),
        alice_snow_land
    ));
    assert!(!game.can_spend_mana_as_any_color_from_mana_source(
        bob,
        Some(alice_creature),
        alice_snow_land
    ));
}

#[test]
fn source_filtered_casting_permission_matches_stack_spell_origin_snapshot() {
    use crate::card::CardBuilder;
    use crate::effect::ManaSpendPermission;
    use crate::object::CounterType;

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let bear_card = CardBuilder::new(CardId::from_raw(305), "Grizzly Bears")
        .card_types(vec![CardType::Creature])
        .build();
    let snow_land_card = CardBuilder::new(CardId::from_raw(306), "Snow Land")
        .supertypes(vec![crate::types::Supertype::Snow])
        .card_types(vec![CardType::Land])
        .build();

    let exiled_bear = game.create_object_from_card(&bear_card, bob, Zone::Exile);
    game.object_mut(exiled_bear)
        .expect("exiled bear")
        .add_counters(CounterType::Ice, 1);
    let snow_land = game.create_object_from_card(&snow_land_card, alice, Zone::Battlefield);

    let mut spell_filter = crate::target::ObjectFilter {
        zone: Some(Zone::Exile),
        owner: Some(crate::target::PlayerFilter::Opponent),
        with_counter: Some(crate::filter::CounterConstraint::Typed(CounterType::Ice)),
        ..crate::target::ObjectFilter::default()
    };
    spell_filter.excluded_card_types.push(CardType::Land);

    game.effect_store
        .mana_spend_effects
        .permissions
        .push(ActiveManaSpendPermission {
            permission: ManaSpendPermission::any_color_from_sources_for_casting_matching(
                crate::target::PlayerFilter::You,
                spell_filter,
                crate::target::ObjectFilter::default()
                    .with_supertype(crate::types::Supertype::Snow),
            ),
            controller: alice,
            source: ManaSpendPermissionSource::StaticAbility,
        });

    let origin_snapshot =
        ObjectSnapshot::from_object(game.object(exiled_bear).expect("origin"), &game);
    let stack_bear = game
        .move_object_by_effect(exiled_bear, Zone::Stack)
        .expect("spell should move to stack");
    game.set_cast_origin_snapshot(stack_bear, origin_snapshot);

    assert!(
        game.has_source_filtered_mana_spend_permission(alice, Some(stack_bear)),
        "stack spell should match its exiled origin snapshot"
    );
    assert!(game.can_spend_mana_as_any_color_from_mana_source(alice, Some(stack_bear), snow_land));
}

#[test]
fn current_controller_skips_unrelated_stop_controlling_effects_before_duration_check() {
    use crate::card::{CardBuilder, PowerToughness};

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::from_raw(400), "Control Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let target_card = CardBuilder::new(CardId::from_raw(401), "Control Target")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();

    let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let target_id = game.create_object_from_card(&target_card, bob, Zone::Battlefield);

    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::gain_control(source_id, alice, target_id, alice)
            .until(Until::YouStopControllingThis),
    );

    assert_eq!(game.current_controller(source_id), Some(alice));
    assert_eq!(game.current_controller(target_id), Some(alice));
}

#[test]
fn stop_controlling_duration_does_not_self_justify_control_effect() {
    use crate::card::{CardBuilder, PowerToughness};

    let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let source_card = CardBuilder::new(CardId::from_raw(402), "Self Referencing Source")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(3, 3))
        .build();
    let source_id = game.create_object_from_card(&source_card, bob, Zone::Battlefield);

    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::gain_control(source_id, alice, source_id, alice)
            .until(Until::YouStopControllingThis),
    );

    assert_eq!(game.current_controller(source_id), Some(bob));
}

struct PendingBattleProtectorDecisionMaker {
    prompted: bool,
}

impl crate::decision::DecisionMaker for PendingBattleProtectorDecisionMaker {
    fn awaiting_choice(&self) -> bool {
        self.prompted
    }

    fn decide_options(
        &mut self,
        _game: &GameState,
        context: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        assert_eq!(
            context.description,
            "Choose a player to protect this battle"
        );
        assert_eq!(context.options.len(), 2);
        self.prompted = true;
        Vec::new()
    }
}

#[test]
fn pending_siege_protector_choice_does_not_commit_entry() {
    use crate::card::CardBuilder;

    let mut game = GameState::new(
        vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ],
        20,
    );
    let alice = PlayerId::from_index(0);
    let siege = CardBuilder::new(CardId::from_raw(403), "Pending Siege")
        .card_types(vec![CardType::Battle])
        .subtypes(vec![crate::types::Subtype::Siege])
        .defense(4)
        .build();
    let siege_id = game.create_object_from_card(&siege, alice, Zone::Hand);
    let mut decision_maker = PendingBattleProtectorDecisionMaker { prompted: false };

    let result = game.move_object_with_etb_processing_with_dm(
        siege_id,
        Zone::Battlefield,
        &mut decision_maker,
    );

    assert!(result.is_none());
    assert!(decision_maker.prompted);
    assert_eq!(
        game.object(siege_id).map(|object| object.zone),
        Some(Zone::Hand)
    );
    assert_eq!(
        game.counter_count(siege_id, crate::object::CounterType::Defense),
        0
    );
    assert_eq!(game.battle_protector(siege_id), None);
}

#[test]
fn multiplayer_leave_game_applies_800_4a_in_order_without_owned_object_zone_changes() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::EventKind;

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let creature = CardBuilder::new(CardId::from_raw(50_001), "Leave Game Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let spell = CardBuilder::new(CardId::from_raw(50_002), "Borrowed Stack Card")
        .card_types(vec![CardType::Sorcery])
        .build();

    let owned_permanent = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let owned_hand_card = game.create_object_from_card(&spell, alice, Zone::Hand);
    let borrowed_permanent = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let surviving_source = game.create_object_from_card(&creature, charlie, Zone::Battlefield);
    let borrowed_spell = game.create_object_from_card(&spell, bob, Zone::Stack);
    let borrowed_spell_stable = game.object(borrowed_spell).expect("spell").stable_id;

    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::gain_control(owned_permanent, alice, borrowed_permanent, alice)
            .until(Until::Forever),
    );
    game.push_to_stack(StackEntry::new(borrowed_spell, alice));
    let mut copied_ability = StackEntry::new(surviving_source, alice);
    copied_ability.is_ability = true;
    game.push_to_stack(copied_ability);
    game.effect_store.replacement_effects.add_effect(
        crate::replacement::ReplacementEffect::prevent_damage(surviving_source, alice, 1),
    );
    game.effect_store.prevention_effects.add_shield(
        crate::prevention::PreventionShield::prevent_next_n(
            surviving_source,
            alice,
            crate::prevention::PreventionTarget::You,
            1,
        ),
    );
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.turn_store.extra_turns.extend([charlie, alice]);
    game.set_monarch(Some(alice));
    game.set_initiative(Some(alice));
    game.effect_store.pending_trigger_events.clear();

    assert!(game.mark_player_lost(alice));

    assert!(game.object(owned_permanent).is_none());
    assert!(game.object(owned_hand_card).is_none());
    assert_eq!(game.current_controller(borrowed_permanent), Some(bob));
    assert!(game.stack.is_empty());
    let exiled_spell = game
        .find_object_by_stable_id(borrowed_spell_stable)
        .expect("remaining controlled card spell should be exiled");
    assert_eq!(
        game.object(exiled_spell).map(|object| object.zone),
        Some(Zone::Exile)
    );
    assert_eq!(game.turn.active_player, alice, "the current turn continues");
    assert_eq!(game.turn.priority_player, Some(bob));
    assert_eq!(game.turn_store.extra_turns, vec![charlie]);
    assert_eq!(game.monarch, Some(bob));
    assert_eq!(game.initiative, Some(bob));
    assert!(game.object(surviving_source).is_some());
    assert_eq!(game.effect_store.replacement_effects.effects().len(), 1);
    assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    assert_eq!(
        game.effect_store
            .pending_trigger_events
            .iter()
            .filter(|event| event.kind() == EventKind::ZoneChange)
            .count(),
        1,
        "only exiling the remaining controlled spell is a zone change"
    );
    assert!(
        game.effect_store
            .pending_trigger_events
            .iter()
            .any(|event| {
                event.kind() == EventKind::PlayerLosesGame && event.player() == Some(alice)
            })
    );
}

#[test]
fn multiplayer_leave_game_active_turn_continues_without_an_active_player() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    assert!(game.leave_game(alice));

    assert_eq!(
        game.turn.active_player, alice,
        "the departed player's turn remains the turn-progression anchor"
    );
    assert!(
        game.active_player().is_none(),
        "CR 800.4j says the rest of this turn has no active player"
    );
    assert!(
        game.active_player_mut().is_none(),
        "mutable active-player access must not expose departed live storage"
    );
    let filter_context = game.filter_context_for(bob, None);
    assert_eq!(
        filter_context.active_player, None,
        "object/player filters must not retain the departed turn anchor as an active player"
    );
    assert!(
        !crate::target::PlayerFilter::Active.matches_player(alice, &filter_context),
        "shared player-filter matching must observe the rules-active player, not the turn anchor"
    );
    assert!(
        !crate::triggers::player_filter_matches_with_context(
            &crate::target::PlayerFilter::Active,
            alice,
            bob,
            &game,
            None,
        ),
        "trigger matchers must not treat the departed player as active"
    );
    let context = crate::effects::ExecutionContext::new_default(ObjectId::from_raw(50_011), bob);
    assert!(
        crate::effects::helpers::resolve_player_filter(
            &game,
            &crate::target::PlayerFilter::Active,
            &context,
        )
        .is_err(),
        "effects cannot resolve an active-player recipient after that player leaves"
    );
    assert_eq!(
        game.turn.priority_player,
        Some(bob),
        "priority passes to the next player still in the game"
    );
}

#[test]
fn multiplayer_leave_game_clamps_next_turn_ability_grants_to_the_would_be_boundary() {
    use crate::effects::{EffectExecutor, ExecutionContext, GrantEffect, ResolvedTarget};
    use crate::grant::{GrantDuration, Grantable};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ChooseSpec, ObjectFilter};

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let source_card = crate::card::CardBuilder::new(CardId::from_raw(50_009), "Grant Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let target_card = crate::card::CardBuilder::new(CardId::from_raw(50_010), "Grant Target")
        .card_types(vec![CardType::Creature])
        .build();
    let source = game.create_object_from_card(&source_card, bob, Zone::Battlefield);
    let target = game.create_object_from_card(&target_card, charlie, Zone::Hand);
    let flash = StaticAbility::flash();
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);

    let mut context = ExecutionContext::new_default(source, bob);
    context.targets = vec![ResolvedTarget::Object(target)];
    GrantEffect::new(
        Grantable::ability(flash.clone()),
        ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
        GrantDuration::UntilYourNextTurnEnd,
    )
    .execute(&mut game, &mut context)
    .expect("the turn-relative grant should resolve");
    assert!(game.effect_store.grant_registry.card_has_granted_ability(
        &game,
        target,
        Zone::Hand,
        charlie,
        &flash,
    ));

    assert!(game.leave_game(bob));
    assert!(
        game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            target,
            Zone::Hand,
            charlie,
            &flash,
        ),
        "CR 800.4m keeps the effect until Bob's next turn would begin"
    );

    game.next_turn();
    assert_eq!(game.turn.active_player, charlie);
    assert!(
        !game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            target,
            Zone::Hand,
            charlie,
            &flash,
        ),
        "the grant must expire when Bob's next turn would have begun"
    );
}

#[test]
fn multiplayer_leave_game_800_4i_k_m_preserves_lki_and_expires_at_would_be_turn() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::LifeLossEvent;

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let creature = CardBuilder::new(CardId::from_raw(50_005), "Duration Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&creature, bob, Zone::Battlefield);
    let target = game.create_object_from_card(&creature, charlie, Zone::Battlefield);
    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::pump(source, bob, target, 2, 2, Until::YourNextTurn)
            .with_expires_end_of_turn(game.turn.turn_number),
    );
    game.add_goad_effect(target, bob, Until::YourNextTurn, source);
    game.add_restriction_effect(
        crate::effect::Restriction::Attack(crate::target::ObjectFilter::specific(target)),
        Until::YourNextTurn,
        source,
        bob,
        None,
    );
    game.player_mut(bob).expect("Bob").life = 7;
    game.record_turn_history_event(&crate::triggers::TriggerEvent::new(
        LifeLossEvent::new(bob, 3, false),
        crate::provenance::ProvNodeId::default(),
    ));
    game.turn.active_player = alice;
    game.turn.priority_player = Some(alice);
    game.set_monarch(Some(bob));
    game.set_initiative(Some(bob));
    game.refresh_continuous_state();
    assert_eq!(game.calculated_power(target), Some(4));

    assert!(game.leave_game(bob));

    assert_eq!(game.player(bob).map(|player| player.life), Some(7));
    assert_eq!(
        game.turn_store
            .turn_history
            .total_life_lost_for_players(&[bob]),
        3,
        "actions by the departed player remain observable"
    );
    assert_eq!(game.calculated_power(target), Some(4));
    assert!(game.effect_store.goad_effects[0].is_active(&game, game.turn.turn_number));
    assert!(game.effect_store.restriction_effects[0].is_active(&game, game.turn.turn_number));
    assert_eq!(game.monarch, Some(alice));
    assert_eq!(game.initiative, Some(alice));

    game.next_turn();

    assert_eq!(
        game.turn.active_player, charlie,
        "Bob's future turn is skipped"
    );
    assert_eq!(game.calculated_power(target), Some(2));
    assert!(game.effect_store.goad_effects.is_empty());
    assert!(game.effect_store.restriction_effects.is_empty());
}

#[test]
fn multiplayer_leave_game_800_4i_freezes_player_information_before_owned_objects_leave() {
    use crate::card::CardBuilder;

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let bob = PlayerId::from_index(1);
    let card = CardBuilder::new(CardId::from_raw(50_006), "Departing Hand Card").build();
    let first = game.create_object_from_card(&card, bob, Zone::Hand);
    let second = game.create_object_from_card(&card, bob, Zone::Hand);
    game.player_mut(bob).expect("Bob").life = 7;

    assert!(game.leave_game(bob));

    assert!(game.object(first).is_none());
    assert!(game.object(second).is_none());
    let lki = game
        .player_last_known_information(bob)
        .expect("departed player LKI should be retained");
    assert_eq!(lki.life, 7);
    assert_eq!(lki.hand.len(), 2);
    assert!(lki.has_left_game);
    assert_eq!(game.player(bob).map(|player| player.hand.len()), Some(2));

    game.player_mut(bob)
        .expect("live storage remains addressable")
        .life = 1;
    assert_eq!(
        game.player_last_known_information(bob)
            .expect("LKI remains frozen")
            .life,
        7,
        "post-departure storage changes must not rewrite player LKI"
    );
    assert_eq!(game.player(bob).map(|player| player.life), Some(7));
}

#[test]
fn multiplayer_leave_game_800_4i_retains_actions_and_expires_last_turn_at_would_be_boundary() {
    use crate::card::CardBuilder;
    use crate::effect::Condition;
    use crate::effects::ExecutionContext;
    use crate::events::LifeLossEvent;
    use crate::events::spells::SpellCastEvent;

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let spell_card = CardBuilder::new(CardId::from_raw(50_007), "Historical Spell")
        .card_types(vec![CardType::Instant])
        .build();
    let source_card = CardBuilder::new(CardId::from_raw(50_008), "History Observer")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
    let lost_life_last_turn = Condition::TurnHistory(
        ironsmith_core::TurnHistoryCondition::PlayerLostLifeLastTurn(
            crate::target::PlayerFilter::Specific(bob),
        ),
    );
    let context = ExecutionContext::new_default(source, alice);

    game.next_turn();
    assert_eq!(game.turn.active_player, bob);
    let spell = game.create_object_from_card(&spell_card, bob, Zone::Stack);
    let snapshot = ObjectSnapshot::from_object(game.object(spell).expect("spell"), &game);
    game.record_turn_history_event(&crate::triggers::TriggerEvent::new(
        SpellCastEvent::new_with_snapshot(spell, bob, Zone::Hand, snapshot),
        crate::provenance::ProvNodeId::default(),
    ));
    game.record_turn_history_event(&crate::triggers::TriggerEvent::new(
        LifeLossEvent::new(bob, 3, false),
        crate::provenance::ProvNodeId::default(),
    ));
    game.move_object_by_effect(spell, Zone::Graveyard);
    game.next_turn();
    assert_eq!(game.turn.active_player, charlie);

    assert!(game.leave_game(bob));
    assert_eq!(
        game.action_history_for_player(bob)
            .filter(|record| {
                record
                    .event
                    .downcast::<SpellCastEvent>()
                    .is_some_and(|event| event.caster == bob)
            })
            .count(),
        1,
        "unqualified action history remains available for departed players"
    );
    assert_eq!(
        game.last_turn_history_for_player(bob)
            .map(|history| history.spells_cast_by_player(bob)),
        Some(1)
    );
    assert!(
        crate::condition_eval::evaluate_condition_resolution(
            &game,
            &lost_life_last_turn,
            &context,
        )
        .expect("last-turn condition should evaluate")
    );

    game.next_turn();
    assert_eq!(game.turn.active_player, alice);
    assert_eq!(
        game.last_turn_history_for_player(bob)
            .map(|history| history.spells_cast_by_player(bob)),
        Some(1),
        "Bob's last-turn actions remain visible before his turn would begin"
    );

    game.next_turn();
    assert_eq!(game.turn.active_player, charlie);
    assert!(
        game.last_turn_history_for_player(bob).is_none(),
        "Bob's last-turn window expires exactly when his next turn would have begun"
    );
    assert!(
        !crate::condition_eval::evaluate_condition_resolution(
            &game,
            &lost_life_last_turn,
            &context,
        )
        .expect("expired last-turn condition should evaluate")
    );
    assert_eq!(
        game.action_history_for_player(bob)
            .filter(|record| {
                record
                    .event
                    .downcast::<SpellCastEvent>()
                    .is_some_and(|event| event.caster == bob)
            })
            .count(),
        1,
        "ordinary action history does not expire with the narrower last-turn window"
    );
}

#[test]
fn multiplayer_leave_game_800_4c_exiles_when_default_controller_already_left() {
    use crate::card::{CardBuilder, PowerToughness};

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let creature = CardBuilder::new(CardId::from_raw(50_003), "Orphaned Permanent")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&creature, charlie, Zone::Battlefield);
    let permanent = game.create_object_from_card(&creature, alice, Zone::Battlefield);
    let stable_id = game.object(permanent).expect("permanent").stable_id;
    game.effect_store.continuous_effects.add_effect(
        ContinuousEffect::gain_control(source, charlie, permanent, bob).until(Until::Forever),
    );
    game.player_mut(alice).expect("Alice").has_left_game = true;
    assert_eq!(game.current_controller(permanent), Some(bob));

    assert!(game.leave_game(bob));

    let exiled = game
        .find_object_by_stable_id(stable_id)
        .expect("orphaned permanent should still be represented in exile");
    assert_eq!(
        game.object(exiled).map(|object| object.zone),
        Some(Zone::Exile)
    );
}

#[test]
fn multiplayer_leave_game_routes_800_4g_h_choices_and_rejects_800_4f_costs() {
    use crate::card::CardBuilder;

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let charlie = PlayerId::from_index(2);
    let card = CardBuilder::new(CardId::from_raw(50_004), "Choice Source")
        .card_types(vec![CardType::Artifact])
        .build();
    let source = game.create_object_from_card(&card, charlie, Zone::Battlefield);
    game.effect_store.pending_replacement_choice = Some(PendingReplacementChoice {
        event: crate::events::Event::player_loses_game(alice),
        applicable_effects: Vec::new(),
        applied_effects: HashSet::new(),
        applied_effect_keys: HashSet::new(),
        player: alice,
    });

    assert!(game.leave_game(alice));

    assert_eq!(game.controlling_player_for(charlie), charlie);
    assert_eq!(game.controlling_player_for(alice), bob);
    assert!(
        game.effect_store.pending_replacement_choice.is_none(),
        "a pending choice with no surviving applicable effects should be discarded"
    );
    assert!(
        crate::cost::can_pay_cost(&game, source, alice, &crate::cost::TotalCost::free()).is_err()
    );
}

#[test]
fn enchanted_combat_history_uses_the_interval_since_controllers_previous_upkeep() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::combat::{AttackEventTarget, CreatureAttackedEvent};
    use crate::object::AttachmentTarget;

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let creature_card = CardBuilder::new(CardId::from_raw(50_005), "Tracked Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let aura_card = CardBuilder::new(CardId::from_raw(50_006), "Tracking Aura")
        .card_types(vec![CardType::Enchantment])
        .build();
    let creature = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let aura = game.create_object_from_card(&aura_card, alice, Zone::Battlefield);
    game.object_mut(aura).expect("aura").attached_to = Some(AttachmentTarget::Object(creature));

    game.turn.turn_number = 1;
    game.mark_upkeep_began(alice);
    game.record_turn_history_event(&crate::triggers::TriggerEvent::new(
        CreatureAttackedEvent::new(creature, AttackEventTarget::Player(bob)),
        crate::provenance::ProvNodeId::default(),
    ));
    game.turn.turn_number = 2;
    game.mark_upkeep_began(alice);
    assert!(game.enchanted_permanent_attacked_or_blocked_since_last_upkeep(aura, alice));

    game.turn.turn_number = 3;
    game.mark_upkeep_began(alice);
    assert!(
        !game.enchanted_permanent_attacked_or_blocked_since_last_upkeep(aura, alice),
        "an attack before the previous upkeep must age out of the predicate"
    );
}

#[test]
fn source_block_history_tracks_blocker_and_blocked_attacker_separately() {
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::combat::CreatureBlockedEvent;

    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let alice = PlayerId::from_index(0);
    let creature_card = CardBuilder::new(CardId::from_raw(50_007), "Tracked Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let blocker = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);
    let attacker = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

    game.turn.turn_number = 1;
    game.mark_upkeep_began(alice);
    game.record_turn_history_event(&crate::triggers::TriggerEvent::new(
        CreatureBlockedEvent::new(blocker, attacker),
        crate::provenance::ProvNodeId::default(),
    ));
    game.turn.turn_number = 2;
    game.mark_upkeep_began(alice);
    assert!(game.source_blocked_or_became_blocked_since_last_upkeep(blocker, alice));
    assert!(game.source_blocked_or_became_blocked_since_last_upkeep(attacker, alice));

    game.turn.turn_number = 3;
    game.mark_upkeep_began(alice);
    assert!(!game.source_blocked_or_became_blocked_since_last_upkeep(blocker, alice));
    assert!(!game.source_blocked_or_became_blocked_since_last_upkeep(attacker, alice));
}

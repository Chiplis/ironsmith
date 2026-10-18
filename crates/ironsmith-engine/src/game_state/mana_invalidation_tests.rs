use super::*;
use crate::ability::Ability;
use crate::card::PowerToughness;
use crate::cards::CardDefinitionBuilder;
use crate::effect::{Value, ValueComparisonOperator};
use crate::ids::CardId;
use crate::mana::ManaSymbol;
use crate::static_abilities::{Anthem, AnthemCountExpression, AnthemValue, StaticAbility};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;

fn fixture(anthem: Anthem) -> (GameState, ObjectId, PlayerId) {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let player = PlayerId::from_index(0);
    let card = CardDefinitionBuilder::new(CardId::new(), "Mana invalidation fixture")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let id = game.create_object_from_definition(&card, player, Zone::Battlefield);
    game.object_mut(id)
        .unwrap()
        .abilities_mut()
        .push(Ability::static_ability(StaticAbility::new(anthem)));
    game.refresh_continuous_state().unwrap();
    (game, id, player)
}

fn assert_matches_full(game: &mut GameState) {
    game.refresh_continuous_state().unwrap();
    let mut full = game.clone();
    full.mark_continuous_state_dirty();
    full.refresh_continuous_state().unwrap();
    for id in game.battlefield.iter().copied() {
        assert_eq!(
            format!("{:?}", game.current_characteristics(id)),
            format!("{:?}", full.current_characteristics(id)),
            "characteristics of {id:?} must agree with a full rebuild"
        );
        assert_eq!(game.current_controller(id), full.current_controller(id));
    }
}

#[test]
fn mana_only_changes_keep_tap_dependent_characteristics_but_taps_still_invalidate() {
    let mut filter = ObjectFilter::creature();
    filter.tapped = true;
    let (mut game, id, player) = fixture(Anthem::new(filter, 1, 1));
    assert!(game.continuous_effects_are_tap_sensitive());
    assert_eq!(game.current_characteristics(id).unwrap().power, Some(2));
    let before = game.work_counters();
    for symbol in [ManaSymbol::Green, ManaSymbol::Blue, ManaSymbol::Colorless] {
        game.with_player_mana_mut(player, |p| p.mana_pool.add(symbol, 1));
        assert!(game.continuous_state_is_clean());
        assert_matches_full(&mut game);
    }
    assert_eq!(
        game.work_counters().continuous_global_invalidations,
        before.continuous_global_invalidations
    );
    game.tap(id);
    assert!(!game.continuous_state_is_clean());
    assert_matches_full(&mut game);
    assert_eq!(game.current_characteristics(id).unwrap().power, Some(3));
}

#[test]
fn mana_anthem_tracks_total_and_colored_reads_across_add_spend_and_clone() {
    for count in [
        AnthemCountExpression::TotalUnspentMana(PlayerFilter::You),
        AnthemCountExpression::UnspentMana {
            player: PlayerFilter::You,
            symbol: ManaSymbol::Green,
        },
        AnthemCountExpression::TotalUnspentMana(PlayerFilter::Opponent),
    ] {
        let value = AnthemValue::scaled(1, count.clone());
        let (mut game, id, owner) =
            fixture(Anthem::for_source(0, 0).with_values(value.clone(), value));
        let payer = if matches!(
            count,
            AnthemCountExpression::TotalUnspentMana(PlayerFilter::Opponent)
        ) {
            PlayerId::from_index(1)
        } else {
            owner
        };
        assert_eq!(game.current_characteristics(id).unwrap().power, Some(2));
        // The warmed clone must carry the dependencies of its retained caches.
        game = game.clone();
        for amount in [1, 4, 0, 2, 0] {
            game.with_player_mana_mut(payer, |p| p.mana_pool.green = amount);
            assert!(!game.continuous_state_is_clean());
            assert_matches_full(&mut game);
            assert_eq!(
                game.current_characteristics(id).unwrap().power,
                Some(2 + amount as i32)
            );
        }
    }
}

#[test]
fn inactive_mana_condition_becomes_active_then_inactive_again() {
    let condition = crate::ConditionExpr::ValueComparison {
        left: Value::UnspentMana(PlayerFilter::You),
        operator: ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    };
    let (mut game, id, player) = fixture(Anthem::for_source(3, 3).with_condition(condition));
    for amount in [1, 0, 2, 0] {
        game.with_player_mana_mut(player, |p| p.mana_pool.blue = amount);
        assert!(!game.continuous_state_is_clean());
        assert_matches_full(&mut game);
        assert_eq!(
            game.current_characteristics(id).unwrap().power,
            Some(if amount == 0 { 2 } else { 5 })
        );
    }
}

#[test]
fn mana_expression_in_object_filter_tracks_reads() {
    let mut filter = ObjectFilter::creature();
    filter.mana_value = Some(crate::filter::Comparison::EqualExpr(Box::new(
        Value::UnspentMana(PlayerFilter::You),
    )));
    // The fixture has mana value zero; changing toughness cannot feed back into matching.
    let (mut game, id, player) = fixture(Anthem::new(filter, 0, 3));
    assert_eq!(game.current_characteristics(id).unwrap().toughness, Some(5));
    for amount in [2, 0, 3, 0] {
        game.with_player_mana_mut(player, |p| p.mana_pool.red = amount);
        assert!(!game.continuous_state_is_clean());
        assert_matches_full(&mut game);
        assert_eq!(
            game.current_characteristics(id).unwrap().toughness,
            Some(if amount == 0 { 5 } else { 2 })
        );
    }
}

#[test]
fn unrelated_player_changes_are_not_acknowledged_by_mana_credit() {
    let (mut game, _, player) = fixture(Anthem::for_source(1, 1));
    game.player_mut(player).unwrap().life -= 1;
    game.with_player_mana_mut(player, |p| p.mana_pool.green += 1);
    assert!(!game.continuous_state_is_clean());
    assert_matches_full(&mut game);
    game.with_player_mana_mut(player, |p| p.mana_pool.green += 1);
    assert!(game.continuous_state_is_clean());
    // Bypassing player_mut still changes the journal observed at the next read.
    game.players.get_mut(player.index()).unwrap().life -= 1;
    game.with_player_mana_mut(player, |p| p.mana_pool.green += 1);
    assert!(!game.continuous_state_is_clean());
    assert_matches_full(&mut game);
}

#[test]
fn mana_scalar_domain_validation_still_runs_on_retained_cache() {
    let (mut game, _, player) = fixture(Anthem::for_source(1, 1));
    game.with_player_mana_mut(player, |p| p.mana_pool.green = u32::MAX);
    assert!(matches!(
        game.refresh_continuous_state(),
        Err(crate::static_ability_processor::StaticEffectDiscoveryError::ScalarRange { .. })
    ));
    game.with_player_mana_mut(player, |p| p.mana_pool.green = 0);
    assert_matches_full(&mut game);
}

#[test]
fn ordinary_pool_reads_do_not_create_cached_dependencies() {
    let (mut game, _, player) = fixture(Anthem::for_source(1, 1));
    assert_eq!(game.player_mana_pool(player).unwrap().total(), 0);
    game.with_player_mana_mut(player, |p| p.mana_pool.green = 1);
    assert!(game.continuous_state_is_clean());
    assert_matches_full(&mut game);
}

#[test]
fn recomputation_discards_dependencies_of_removed_mana_readers() {
    let value = AnthemValue::scaled(
        1,
        AnthemCountExpression::TotalUnspentMana(PlayerFilter::You),
    );
    let (mut game, id, player) =
        fixture(Anthem::for_source(0, 0).with_values(value.clone(), value));
    assert!(game.runtime_cache.derived_state_read_mana.get());
    game.object_mut(id).unwrap().abilities_mut().clear();
    game.refresh_continuous_state().unwrap();
    game.with_player_mana_mut(player, |p| p.mana_pool.green = 2);
    assert!(game.continuous_state_is_clean());
    assert_matches_full(&mut game);
    assert_eq!(game.current_characteristics(id).unwrap().power, Some(2));
}

#[test]
fn checked_stack_insertion_and_cleanup_restore_state_when_discovery_fails() {
    let (mut game, object, player) = fixture(Anthem::for_source(1, 1));
    game.mark_damage(object, 1);
    game.with_player_mana_mut(player, |p| p.mana_pool.green = u32::MAX);
    let entry = StackEntry::ability(object, player, Vec::new());
    assert!(game.try_push_to_stack(entry.clone()).is_err());
    assert!(game.stack.is_empty(), "failed discovery must not publish the stack entry");
    assert_eq!(game.damage_on(object), 1);
    assert!(crate::turn::try_execute_cleanup_step(&mut game).is_err());
    assert_eq!(game.damage_on(object), 1, "failed cleanup must restore marked damage");
    game.with_player_mana_mut(player, |p| p.mana_pool.green = 0);
    game.try_push_to_stack(entry).unwrap();
    assert_eq!(game.stack.len(), 1);
    crate::turn::try_execute_cleanup_step(&mut game).unwrap();
    assert_eq!(game.damage_on(object), 0);
}

#[test]
fn failed_sticker_refresh_leaves_each_sticker_available_and_retries_cleanly() {
    let (mut game, object, player) = fixture(Anthem::for_source(1, 1));
    let pt = game.add_accessible_pt_sticker(player, 1, 4, 5);
    let ability = game.add_accessible_ability_sticker(player, 1,
        vec![Ability::static_ability(StaticAbility::vigilance())], "Vigilance");
    let name = game.add_accessible_name_sticker(player, "Bright");
    let stable_id = game.object(object).unwrap().stable_id;
    game.with_player_mana_mut(player, |p| p.mana_pool.green = u32::MAX);
    assert!(game.apply_pt_sticker(object, pt).is_none());
    assert!(game.apply_ability_sticker(object, ability).is_none());
    assert!(game.apply_available_name_sticker(player, object, name, 0).is_none());
    assert!(game.object_annotations.object_stickers.get(&stable_id).is_none());
    assert!(game.available_pt_stickers(player).iter().any(|entry| entry.0 == pt));
    assert!(game.available_ability_stickers(player).iter().any(|entry| entry.id == ability));
    assert!(game.available_name_stickers(player).iter().any(|entry| entry.0 == name));
    game.with_player_mana_mut(player, |p| p.mana_pool.green = 0);
    assert!(game.apply_pt_sticker(object, pt).is_some());
    assert!(game.apply_ability_sticker(object, ability).is_some());
    assert!(game.apply_available_name_sticker(player, object, name, 0).is_some());
    assert_eq!(game.object_annotations.object_stickers[&stable_id].len(), 3);
    assert_eq!(game.current_characteristics(object).unwrap().power, Some(5));
    assert_eq!(game.current_characteristics(object).unwrap().toughness, Some(6));
}

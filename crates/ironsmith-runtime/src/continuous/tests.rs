use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::effect::{EventValueSpec, Value};
use crate::game_state::GameState;
use crate::ids::{CardId, PlayerId};
use crate::mana::{ManaCost, ManaSymbol};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::CardType;
use crate::zone::Zone;
// Tests use the new StaticAbility type (already imported as StaticAbility in the module)

fn dynamic_value_test_game() -> GameState {
    GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
}

#[test]
fn affected_object_counter_duration_tracks_the_resolved_specific_object() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let land = CardBuilder::new(CardId::from_raw(9089), "Countered Land")
        .card_types(vec![CardType::Land])
        .build();
    let land_id = game.create_object_from_card(&land, alice, Zone::Battlefield);
    game.object_mut(land_id)
        .expect("land should exist")
        .counters
        .insert(CounterType::Flood, 1);

    let effect = ContinuousEffect::new(
        land_id,
        alice,
        EffectTarget::Specific(land_id),
        Modification::AddSubtypes(vec![Subtype::Island]),
    )
    .until(Until::ForAsLongAs(
        ironsmith_core::ContinuousDurationPredicate::ObjectHasCounter {
            object: ironsmith_core::ContinuousDurationObject::Specific(land_id),
            counter_type: CounterType::Flood,
            minimum: 1,
        },
    ));

    assert!(continuous_effect_duration_and_condition_are_active(
        &effect, &game
    ));

    game.object_mut(land_id)
        .expect("land should still exist")
        .counters
        .remove(&CounterType::Flood);
    assert!(!continuous_effect_duration_and_condition_are_active(
        &effect, &game
    ));
}

fn add_dynamic_base_pt(
    game: &mut GameState,
    permanent: ObjectId,
    controller: PlayerId,
    power: Value,
    toughness: Value,
) {
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            permanent,
            controller,
            EffectTarget::Specific(permanent),
            Modification::SetPowerToughness {
                power,
                toughness,
                sublayer: PtSublayer::CharacteristicDefining,
            },
        ));
}

#[test]
fn dynamic_hand_size_characteristic_updates_before_state_based_actions() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let maro = CardBuilder::new(CardId::from_raw(9090), "Dynamic Hand Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .build();
    let maro_id = game.create_object_from_card(&maro, alice, Zone::Battlefield);
    add_dynamic_base_pt(
        &mut game,
        maro_id,
        alice,
        Value::CardsInHand(PlayerFilter::You),
        Value::CardsInHand(PlayerFilter::You),
    );

    let held = CardBuilder::new(CardId::from_raw(9091), "Held Card").build();
    let held_id = game.create_object_from_card(&held, alice, Zone::Hand);
    assert_eq!(game.calculated_power(maro_id), Some(1));
    assert_eq!(game.calculated_toughness(maro_id), Some(1));
    assert!(
        !crate::rules::check_state_based_actions(&game)
            .contains(&crate::rules::StateBasedAction::ObjectDies(maro_id))
    );

    game.move_object(
        held_id,
        Zone::Graveyard,
        crate::events::cause::EventCause::from_game_rule(),
    )
    .expect("held card should move to the graveyard");
    assert_eq!(game.calculated_power(maro_id), Some(0));
    assert_eq!(game.calculated_toughness(maro_id), Some(0));
    assert!(
        crate::rules::check_state_based_actions(&game)
            .contains(&crate::rules::StateBasedAction::ObjectDies(maro_id)),
        "state-based actions must observe the current hand-size CDA value"
    );
}

#[test]
fn dynamic_life_graveyard_and_devotion_values_track_current_state() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let dynamic = |id, name: &str, mana_cost| {
        CardBuilder::new(CardId::from_raw(id), name)
            .mana_cost(mana_cost)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 1))
            .build()
    };

    let life_id = game.create_object_from_card(
        &dynamic(9092, "Life Creature", ManaCost::new()),
        alice,
        Zone::Battlefield,
    );
    add_dynamic_base_pt(
        &mut game,
        life_id,
        alice,
        Value::LifeTotal(PlayerFilter::You),
        Value::Fixed(1),
    );
    let grave_id = game.create_object_from_card(
        &dynamic(9093, "Grave Creature", ManaCost::new()),
        alice,
        Zone::Battlefield,
    );
    add_dynamic_base_pt(
        &mut game,
        grave_id,
        alice,
        Value::CardsInGraveyard(PlayerFilter::You),
        Value::Fixed(1),
    );
    let devotion_id = game.create_object_from_card(
        &dynamic(
            9094,
            "Devotion Creature",
            ManaCost::from_symbols(vec![ManaSymbol::Blue, ManaSymbol::Blue]),
        ),
        alice,
        Zone::Battlefield,
    );
    add_dynamic_base_pt(
        &mut game,
        devotion_id,
        alice,
        Value::Devotion {
            player: PlayerFilter::You,
            color: crate::color::Color::Blue,
        },
        Value::Fixed(1),
    );

    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(9095), "Buried Card").build(),
        alice,
        Zone::Graveyard,
    );
    assert_eq!(game.calculated_power(life_id), Some(20));
    assert_eq!(game.calculated_power(grave_id), Some(1));
    assert_eq!(game.calculated_power(devotion_id), Some(2));

    assert_eq!(game.lose_life(alice, 5), 5);
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(9096), "Another Buried Card").build(),
        alice,
        Zone::Graveyard,
    );
    game.create_object_from_card(
        &CardBuilder::new(CardId::from_raw(9097), "Blue Permanent")
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .card_types(vec![CardType::Enchantment])
            .build(),
        alice,
        Zone::Battlefield,
    );
    assert_eq!(game.calculated_power(life_id), Some(15));
    assert_eq!(game.calculated_power(grave_id), Some(2));
    assert_eq!(game.calculated_power(devotion_id), Some(3));
}

#[test]
fn colored_mana_symbol_aggregates_track_battlefield_and_graveyard_scopes() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);

    let primalcrux = CardBuilder::new(CardId::from_raw(9160), "Primalcrux Probe")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Green,
            ManaSymbol::Green,
        ]))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .build();
    let primalcrux_id = game.create_object_from_card(&primalcrux, alice, Zone::Battlefield);
    let battlefield_green = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ChooseSpec::All(ObjectFilter::permanent().you_control())),
        color: crate::color::Color::Green,
    };
    add_dynamic_base_pt(
        &mut game,
        primalcrux_id,
        alice,
        battlefield_green.clone(),
        battlefield_green,
    );

    let hybrid_green = CardBuilder::new(CardId::from_raw(9161), "Hybrid Green")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Green, ManaSymbol::White],
            vec![ManaSymbol::Generic(2), ManaSymbol::Green],
        ]))
        .card_types(vec![CardType::Enchantment])
        .build();
    game.create_object_from_card(&hybrid_green, alice, Zone::Battlefield);
    let opponents_green = CardBuilder::new(CardId::from_raw(9162), "Opponent Green")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Green,
            ManaSymbol::Green,
            ManaSymbol::Green,
        ]))
        .card_types(vec![CardType::Enchantment])
        .build();
    game.create_object_from_card(&opponents_green, bob, Zone::Battlefield);
    assert_eq!(game.calculated_power(primalcrux_id), Some(4));

    let umbra = CardBuilder::new(CardId::from_raw(9163), "Umbra Probe")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .build();
    let umbra_id = game.create_object_from_card(&umbra, alice, Zone::Battlefield);
    let mut your_graveyard = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    your_graveyard.set_explicit_card_noun(true);
    let graveyard_black = Value::ManaSymbolsInManaCostOf {
        spec: Box::new(ChooseSpec::All(your_graveyard)),
        color: crate::color::Color::Black,
    };
    add_dynamic_base_pt(
        &mut game,
        umbra_id,
        alice,
        graveyard_black.clone(),
        graveyard_black,
    );

    let double_black = CardBuilder::new(CardId::from_raw(9164), "Double Black")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Black,
            ManaSymbol::Black,
        ]))
        .build();
    game.create_object_from_card(&double_black, alice, Zone::Graveyard);
    let hybrid_black = CardBuilder::new(CardId::from_raw(9165), "Hybrid Black")
        .mana_cost(ManaCost::from_pips(vec![
            vec![ManaSymbol::Black, ManaSymbol::Red],
            vec![ManaSymbol::Black, ManaSymbol::Life(2)],
        ]))
        .build();
    let hybrid_black_id = game.create_object_from_card(&hybrid_black, alice, Zone::Graveyard);
    let opponents_black = CardBuilder::new(CardId::from_raw(9166), "Opponent Black")
        .mana_cost(ManaCost::from_symbols(vec![
            ManaSymbol::Black,
            ManaSymbol::Black,
            ManaSymbol::Black,
        ]))
        .build();
    game.create_object_from_card(&opponents_black, bob, Zone::Graveyard);
    assert_eq!(game.calculated_power(umbra_id), Some(4));

    game.move_object(
        hybrid_black_id,
        Zone::Exile,
        crate::events::cause::EventCause::from_game_rule(),
    )
    .expect("graveyard card should move to exile");
    assert_eq!(game.calculated_power(umbra_id), Some(2));
}

#[test]
fn total_and_greatest_power_values_use_current_layered_characteristics() {
    fn aggregate_game(value: Value) -> (GameState, ObjectId, ObjectId) {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let dynamic = CardBuilder::new(CardId::from_raw(9098), "Aggregate Creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 1))
            .build();
        let dynamic_id = game.create_object_from_card(&dynamic, alice, Zone::Battlefield);
        add_dynamic_base_pt(&mut game, dynamic_id, alice, value, Value::Fixed(1));
        let two = CardBuilder::new(CardId::from_raw(9099), "Two Power")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let two_id = game.create_object_from_card(&two, alice, Zone::Battlefield);
        let five = CardBuilder::new(CardId::from_raw(9100), "Five Power")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(5, 5))
            .build();
        game.create_object_from_card(&five, alice, Zone::Battlefield);
        (game, dynamic_id, two_id)
    }

    let filter = ObjectFilter::creature().you_control().other();
    let (mut total_game, total_id, two_id) = aggregate_game(Value::TotalPower(filter.clone()));
    assert_eq!(total_game.calculated_power(total_id), Some(7));
    total_game
        .effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::pump(
            two_id,
            PlayerId::from_index(0),
            two_id,
            4,
            0,
            Until::Forever,
        ));
    assert_eq!(total_game.calculated_power(two_id), Some(6));
    assert_eq!(total_game.calculated_power(total_id), Some(11));

    let (mut greatest_game, greatest_id, two_id) = aggregate_game(Value::GreatestPower(filter));
    assert_eq!(greatest_game.calculated_power(greatest_id), Some(5));
    greatest_game
        .effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::pump(
            two_id,
            PlayerId::from_index(0),
            two_id,
            4,
            0,
            Until::Forever,
        ));
    assert_eq!(greatest_game.calculated_power(greatest_id), Some(6));
}

#[test]
#[should_panic(expected = "unsupported continuous-effect value")]
fn resolution_only_dynamic_values_are_rejected_in_layer_calculation() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(9101), "Invalid Dynamic Creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 1))
        .build();
    let id = game.create_object_from_card(&card, alice, Zone::Battlefield);
    add_dynamic_base_pt(
        &mut game,
        id,
        alice,
        Value::EventValue(EventValueSpec::Amount),
        Value::Fixed(1),
    );
    let _ = game.calculated_power(id);
}

#[test]
fn test_layer_ordering() {
    assert!(Layer::Copy < Layer::Control);
    assert!(Layer::Control < Layer::Text);
    assert!(Layer::Text < Layer::Type);
    assert!(Layer::Type < Layer::Color);
    assert!(Layer::Color < Layer::Ability);
    assert!(Layer::Ability < Layer::PowerToughness);
}

#[test]
fn test_pt_sublayer_ordering() {
    // Per Rule 613.4, counters are part of 7c (Modifying), not a separate sublayer
    assert!(PtSublayer::CharacteristicDefining < PtSublayer::Setting);
    assert!(PtSublayer::Setting < PtSublayer::Modifying);
    assert!(PtSublayer::Modifying < PtSublayer::Switching);
    // There is no separate Counters sublayer - they're applied within Modifying
}

#[test]
fn test_modification_layer() {
    assert_eq!(
        Modification::CopyOf {
            target_id: ObjectId::from_raw(1),
            copiable_values: Box::new(crate::snapshot::CopiableValues::default()),
            preserve_source_abilities: false,
            name_override: None,
            name_override_surface: None,
            add_supertypes: Vec::new(),
        }
        .layer(),
        Layer::Copy
    );
    assert_eq!(
        Modification::ChangeController(PlayerId::from_index(0)).layer(),
        Layer::Control
    );
    assert_eq!(
        Modification::AddCardTypes(vec![CardType::Creature]).layer(),
        Layer::Type
    );
    assert_eq!(
        Modification::AddColors(ColorSet::WHITE).layer(),
        Layer::Color
    );
    assert_eq!(
        Modification::AddAbility(StaticAbility::flying()).layer(),
        Layer::Ability
    );
    assert_eq!(
        Modification::ModifyPowerToughness {
            power: 2,
            toughness: 2
        }
        .layer(),
        Layer::PowerToughness
    );
}

#[test]
fn card_type_setting_prunes_incompatible_subtypes_but_preserves_spell_types() {
    let mut card_types = vec![CardType::Enchantment, CardType::Creature].into();
    let mut subtypes = vec![Subtype::Aura, Subtype::Soldier].into();

    replace_card_types_and_prune_subtypes(&mut card_types, &mut subtypes, &[CardType::Enchantment]);
    assert_eq!(&*card_types, &[CardType::Enchantment]);
    assert_eq!(&*subtypes, &[Subtype::Aura]);

    let mut card_types = vec![CardType::Instant].into();
    let mut subtypes = vec![Subtype::Arcane].into();
    replace_card_types_and_prune_subtypes(&mut card_types, &mut subtypes, &[CardType::Creature]);
    assert!(card_types.contains(&CardType::Creature));
    assert!(card_types.contains(&CardType::Instant));
    assert_eq!(&*subtypes, &[Subtype::Arcane]);
}

#[test]
fn land_subtype_setting_replaces_only_prior_land_subtypes() {
    let mut subtypes = vec![Subtype::Island, Subtype::Forest, Subtype::Saga].into();
    replace_subtypes_in_family(
        &mut subtypes,
        &[Subtype::Mountain, Subtype::Plains],
        SubtypeFamily::Land,
    );
    assert_eq!(
        &*subtypes,
        &[Subtype::Saga, Subtype::Mountain, Subtype::Plains]
    );
}

#[test]
fn test_effect_manager() {
    let mut manager = ContinuousEffectManager::new();

    let effect1 = ContinuousEffect::pump(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        ObjectId::from_raw(2),
        2,
        2,
        Until::EndOfTurn,
    );

    let effect2 = ContinuousEffect::grant_ability(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        ObjectId::from_raw(2),
        StaticAbility::flying(),
        Until::EndOfTurn,
    );

    let id1 = manager.add_effect(effect1);
    let _id2 = manager.add_effect(effect2);

    assert_eq!(manager.effects_sorted().len(), 2);

    // Effects should be sorted by layer
    let sorted = manager.effects_sorted();
    assert_eq!(sorted[0].modification.layer(), Layer::Ability);
    assert_eq!(sorted[1].modification.layer(), Layer::PowerToughness);

    // Remove one effect
    manager.remove_effect(id1);
    assert_eq!(manager.effects_sorted().len(), 1);

    // Remaining effect should be the ability grant
    assert!(matches!(
        manager.effects_sorted()[0].modification,
        Modification::AddAbility(_)
    ));
}

#[test]
fn test_end_of_turn_cleanup() {
    let mut manager = ContinuousEffectManager::new();

    // Add a permanent effect
    let permanent = ContinuousEffect::new(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        EffectTarget::AllCreatures,
        Modification::ModifyPowerToughness {
            power: 1,
            toughness: 1,
        },
    );

    // Add an until-end-of-turn effect
    let temporary = ContinuousEffect::pump(
        ObjectId::from_raw(2),
        PlayerId::from_index(0),
        ObjectId::from_raw(3),
        3,
        3,
        Until::EndOfTurn,
    );

    manager.add_effect(permanent);
    manager.add_effect(temporary);

    assert_eq!(manager.effects_sorted().len(), 2);

    manager.cleanup_end_of_turn();

    assert_eq!(manager.effects_sorted().len(), 1);
    assert!(matches!(
        manager.effects_sorted()[0].duration,
        Until::Forever
    ));
}

#[test]
fn test_timestamp_ordering() {
    let mut manager = ContinuousEffectManager::new();

    // Add two effects in the same layer
    let effect1 = ContinuousEffect::new(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        EffectTarget::Specific(ObjectId::from_raw(10)),
        Modification::SetColors(ColorSet::WHITE),
    );

    manager.advance_timestamp(); // Force different timestamps

    let effect2 = ContinuousEffect::new(
        ObjectId::from_raw(2),
        PlayerId::from_index(0),
        EffectTarget::Specific(ObjectId::from_raw(10)),
        Modification::SetColors(ColorSet::BLACK),
    );

    manager.add_effect(effect1);
    manager.add_effect(effect2);

    let sorted = manager.effects_sorted();
    assert_eq!(sorted.len(), 2);

    // Earlier timestamp should come first
    assert!(sorted[0].timestamp < sorted[1].timestamp);
}

#[test]
fn test_ability_granting_counters() {
    use crate::static_abilities::StaticAbilityId;

    // Create a creature token with a deathtouch counter
    let mut creature = Object::new_token(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        "Test Creature".to_string(),
        vec![CardType::Creature],
        Vec::new(),
        Some(2),
        Some(2),
        ColorSet::GREEN,
    );
    creature.add_counters(CounterType::Deathtouch, 1);

    // Calculate characteristics
    let mut chars = CalculatedCharacteristics {
        name: creature.name.clone(),
        mana_cost: creature.mana_cost_owned(),
        compiled_card_text: creature.compiled_card_text.clone(),
        power: creature.base_power.as_ref().map(|p| p.base_value()),
        toughness: creature.base_toughness.as_ref().map(|t| t.base_value()),
        card_types: creature.card_types.clone(),
        subtypes: creature.subtypes.clone(),
        supertypes: creature.supertypes.clone(),
        world_supertype_since: None,
        colors: creature.colors(),
        loyalty: creature.base_loyalty,
        abilities: creature.abilities.clone().into(),
        static_abilities: extract_static_abilities(&creature.abilities).into(),
        ability_gain_prohibitions: Vec::new(),
        aura_attach_filter: creature.aura_attach_filter_owned(),
        controller: creature.owner,
    };

    // Add abilities from counters
    add_abilities_from_counters(&creature, &mut chars);

    // Should have deathtouch ability
    assert!(
        chars
            .static_abilities
            .iter()
            .any(|a| a.id() == StaticAbilityId::Deathtouch),
        "Creature with deathtouch counter should have deathtouch ability"
    );
    assert!(
        extract_static_abilities(&chars.abilities)
            .iter()
            .any(|a| a.id() == StaticAbilityId::Deathtouch),
        "ability-counter grants must survive static ability extraction from abilities"
    );
}

#[test]
fn test_multiple_ability_counters() {
    use crate::static_abilities::StaticAbilityId;

    // Create a creature token with multiple ability counters
    let mut creature = Object::new_token(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        "Test Creature".to_string(),
        vec![CardType::Creature],
        Vec::new(),
        Some(2),
        Some(2),
        ColorSet::GREEN,
    );
    creature.add_counters(CounterType::Flying, 1);
    creature.add_counters(CounterType::Trample, 1);
    creature.add_counters(CounterType::Vigilance, 1);

    let mut chars = CalculatedCharacteristics {
        name: creature.name.clone(),
        mana_cost: creature.mana_cost_owned(),
        compiled_card_text: creature.compiled_card_text.clone(),
        power: None,
        toughness: None,
        card_types: creature.card_types.clone(),
        subtypes: Vec::new().into(),
        supertypes: Vec::new().into(),
        world_supertype_since: None,
        colors: ColorSet::COLORLESS,
        loyalty: creature.base_loyalty,
        abilities: Vec::new().into(),
        static_abilities: Vec::new().into(),
        ability_gain_prohibitions: Vec::new(),
        aura_attach_filter: creature.aura_attach_filter_owned(),
        controller: creature.owner,
    };

    add_abilities_from_counters(&creature, &mut chars);

    // Should have all three abilities
    assert!(
        chars
            .static_abilities
            .iter()
            .any(|a| a.id() == StaticAbilityId::Flying)
    );
    assert!(
        chars
            .static_abilities
            .iter()
            .any(|a| a.id() == StaticAbilityId::Trample)
    );
    assert!(
        chars
            .static_abilities
            .iter()
            .any(|a| a.id() == StaticAbilityId::Vigilance)
    );
    assert_eq!(chars.static_abilities.len(), 3);

    let extracted = extract_static_abilities(&chars.abilities);
    assert!(extracted.iter().any(|a| a.id() == StaticAbilityId::Flying));
    assert!(extracted.iter().any(|a| a.id() == StaticAbilityId::Trample));
    assert!(
        extracted
            .iter()
            .any(|a| a.id() == StaticAbilityId::Vigilance)
    );
}

#[test]
fn test_no_duplicate_abilities_from_counters() {
    use crate::static_abilities::StaticAbilityId;

    // Create a creature token that already has flying
    let mut creature = Object::new_token(
        ObjectId::from_raw(1),
        PlayerId::from_index(0),
        "Test Creature".to_string(),
        vec![CardType::Creature],
        Vec::new(),
        Some(2),
        Some(2),
        ColorSet::GREEN,
    );
    creature.add_counters(CounterType::Flying, 1);

    // Start with flying ability already present
    let mut chars = CalculatedCharacteristics {
        name: creature.name.clone(),
        mana_cost: creature.mana_cost_owned(),
        compiled_card_text: creature.compiled_card_text.clone(),
        power: None,
        toughness: None,
        card_types: creature.card_types.clone(),
        subtypes: Vec::new().into(),
        supertypes: Vec::new().into(),
        world_supertype_since: None,
        colors: ColorSet::COLORLESS,
        loyalty: creature.base_loyalty,
        abilities: Vec::new().into(),
        static_abilities: vec![StaticAbility::flying()].into(), // Already has flying
        ability_gain_prohibitions: Vec::new(),
        aura_attach_filter: creature.aura_attach_filter_owned(),
        controller: creature.owner,
    };

    add_abilities_from_counters(&creature, &mut chars);

    // Should still only have one flying ability (no duplicate)
    let flying_count = chars
        .static_abilities
        .iter()
        .filter(|a| a.id() == StaticAbilityId::Flying)
        .count();
    assert_eq!(flying_count, 1, "Should not add duplicate flying ability");
}

#[test]
fn layer_six_preserves_distinct_static_ability_instances_and_dedups_the_same_instance() {
    use crate::ability::{Ability, AbilityKind};
    use crate::static_abilities::StaticAbilityId;

    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Layered Flanker")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let creature = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let printed = StaticAbility::flanking();
    game.object_mut(creature)
        .expect("creature should exist")
        .abilities_mut()
        .push(Ability::static_ability(printed.clone()));

    let granted = StaticAbility::flanking();
    for _ in 0..2 {
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                creature,
                alice,
                EffectTarget::Specific(creature),
                Modification::AddAbility(granted.clone()),
            ));
    }

    let calculated = game
        .calculated_characteristics(creature)
        .expect("creature should have calculated characteristics");
    let flanking = calculated
        .abilities
        .iter()
        .filter_map(|ability| match &ability.kind {
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::Flanking =>
            {
                Some(static_ability.instance_id())
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        flanking.len(),
        2,
        "printed and granted instances both remain"
    );
    assert!(flanking.contains(&printed.instance_id()));
    assert!(flanking.contains(&granted.instance_id()));
}

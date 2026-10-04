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
fn batched_effect_filter_context_preserves_source_tags_and_refreshes_between_passes() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Tagged recipient")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 2))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let first = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let second = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let order = vec![source, first, second];
    let effect = ContinuousEffect::new(source, alice,
        EffectTarget::Filter(ObjectFilter::tagged("selected")),
        Modification::AddSubtypes(vec![Subtype::Ninja]));
    for (expected_index, selected) in [(1, first), (2, second)] {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(game.object(selected).unwrap(), &game);
        game.object_mut(source).unwrap().cast_tagged_objects.insert("selected".into(), vec![snapshot]);
        let chars: HashMap<_, _> = order.iter().map(|id| (*id,
            initial_characteristics(game.object(*id).unwrap(), game.turn.turn_number))).collect();
        let result = affected_objects_for_effect(&effect, Layer::Type, &order, game.objects_map(),
            &chars, &HashSet::new(), true, false, true, &game, &mut Vec::new());
        assert_eq!(result, vec![(expected_index, selected)]);
        // The single-object path independently constructs its context. Both
        // paths must agree, including after the source's retained tag changes.
        let independent: Vec<_> = order.iter().enumerate().filter_map(|(index, id)| {
            effect_target_applies_to_direct(&effect, game.object(*id).unwrap(), &chars[id],
                game.objects_map(), &game, &std::cell::OnceCell::new()).then_some((index, *id))
        }).collect();
        assert_eq!(result, independent);
    }
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
fn dynamic_pt_uses_greatest_mana_value_from_matching_spell_cast_history() {
    use crate::events::spells::SpellCastEvent;
    use crate::provenance::ProvNodeId;
    use crate::snapshot::ObjectSnapshot;
    use crate::triggers::TriggerEvent;

    fn stage_spell(
        game: &mut GameState,
        id: ObjectId,
        caster: PlayerId,
        card_type: CardType,
        mana_value: u8,
    ) {
        let mut snapshot = ObjectSnapshot::for_testing(id, caster, "Historical Spell")
            .with_card_types(vec![card_type]);
        snapshot.zone = Zone::Stack;
        snapshot.mana_cost = Some(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            mana_value,
        )]]));
        let event = TriggerEvent::new_with_provenance(
            SpellCastEvent::new_with_snapshot(id, caster, Zone::Hand, snapshot),
            ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    stage_spell(
        &mut game,
        ObjectId::from_raw(92_001),
        alice,
        CardType::Instant,
        3,
    );
    stage_spell(
        &mut game,
        ObjectId::from_raw(92_002),
        alice,
        CardType::Sorcery,
        7,
    );
    stage_spell(
        &mut game,
        ObjectId::from_raw(92_003),
        alice,
        CardType::Creature,
        9,
    );
    stage_spell(
        &mut game,
        ObjectId::from_raw(92_004),
        bob,
        CardType::Instant,
        11,
    );

    let token = CardBuilder::new(CardId::from_raw(92_010), "History Elemental")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(0, 0))
        .build();
    let token_id = game.create_object_from_card(&token, alice, Zone::Battlefield);

    let mut spell_history = ObjectFilter {
        zone: Some(Zone::Stack),
        cast_by: Some(PlayerFilter::You),
        cast_this_turn: true,
        any_of: vec![
            ObjectFilter::default().with_type(CardType::Instant),
            ObjectFilter::default().with_type(CardType::Sorcery),
        ],
        ..ObjectFilter::default()
    };
    spell_history.set_conjunctive_set_surface(true);
    let amount = Value::GreatestManaValue(spell_history);
    add_dynamic_base_pt(&mut game, token_id, alice, amount.clone(), amount);

    assert_eq!(game.calculated_power(token_id), Some(7));
    assert_eq!(game.calculated_toughness(token_id), Some(7));
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
        linked_face_mana_value: creature.linked_face_mana_value(),
        compiled_card_text: creature.compiled_card_text.clone(),
        ability_labels: creature.ability_labels.clone(),
        base_power: creature.base_power.as_ref().map(|p| p.base_value()),
        base_toughness: creature.base_toughness.as_ref().map(|t| t.base_value()),
        power: creature.base_power.as_ref().map(|p| p.base_value()),
        toughness: creature.base_toughness.as_ref().map(|t| t.base_value()),
        card_types: creature.card_types.clone(),
        subtypes: creature.subtypes.clone(),
        supertypes: creature.supertypes.clone(),
        world_supertype_since: None,
        colors: creature.colors(),
        loyalty: creature.base_loyalty,
        defense: creature.base_defense,
        abilities: creature.abilities.clone().into(),
        static_abilities: extract_static_abilities(&creature.abilities).into(),
        numeric_range_error: None,
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
        linked_face_mana_value: creature.linked_face_mana_value(),
        compiled_card_text: creature.compiled_card_text.clone(),
        ability_labels: creature.ability_labels.clone(),
        base_power: None,
        base_toughness: None,
        power: None,
        toughness: None,
        card_types: creature.card_types.clone(),
        subtypes: Vec::new().into(),
        supertypes: Vec::new().into(),
        world_supertype_since: None,
        colors: ColorSet::COLORLESS,
        loyalty: creature.base_loyalty,
        defense: creature.base_defense,
        abilities: Vec::new().into(),
        static_abilities: Vec::new().into(),
        numeric_range_error: None,
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
fn test_counter_flying_preserves_independent_redundant_instances() {
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

    // CR 113.2c and 122.1b preserve both ability occurrences; 702.9c
    // makes their evasion redundant without deleting either occurrence.
    let printed_flying = StaticAbility::flying();
    let printed_id = printed_flying.instance_id();
    let mut chars = CalculatedCharacteristics {
        name: creature.name.clone(),
        mana_cost: creature.mana_cost_owned(),
        linked_face_mana_value: creature.linked_face_mana_value(),
        compiled_card_text: creature.compiled_card_text.clone(),
        ability_labels: creature.ability_labels.clone(),
        base_power: None,
        base_toughness: None,
        power: None,
        toughness: None,
        card_types: creature.card_types.clone(),
        subtypes: Vec::new().into(),
        supertypes: Vec::new().into(),
        world_supertype_since: None,
        colors: ColorSet::COLORLESS,
        loyalty: creature.base_loyalty,
        defense: creature.base_defense,
        abilities: vec![crate::ability::Ability::static_ability(printed_flying.clone())].into(),
        static_abilities: vec![printed_flying].into(),
        numeric_range_error: None,
        ability_gain_prohibitions: Vec::new(),
        aura_attach_filter: creature.aura_attach_filter_owned(),
        controller: creature.owner,
    };

    add_abilities_from_counters(&creature, &mut chars);

    // Redundancy of flying does not merge independent printed/counter origins.
    let flying_count = chars
        .static_abilities
        .iter()
        .filter(|a| a.id() == StaticAbilityId::Flying)
        .count();
    assert_eq!(flying_count, 2, "printed flying and counter-granted flying remain independent");
    let flying = chars.abilities.iter().enumerate().filter_map(|(slot, ability)| {
        let crate::ability::AbilityKind::Static(ability) = &ability.kind else { return None; };
        (ability.id() == StaticAbilityId::Flying).then(|| (
            ability.instance_id(), chars.abilities.origin(slot).expect("every ability has its origin")))
    }).collect::<Vec<_>>();
    assert_eq!(flying.len(), 2);
    assert_eq!(flying[0].0, printed_id, "counter addition preserves the printed instance");
    assert_ne!(flying[0].0, flying[1].0);
    assert!(matches!(flying[0].1, AbilityOrigin::Printed(0)));
    assert!(matches!(flying[1].1, AbilityOrigin::Counter { .. }));
}

#[test]
fn layer_six_preserves_independent_grants_even_with_the_same_static_instance() {
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
        3,
        "printed ability and both independent continuous grants remain"
    );
    assert!(flanking.contains(&printed.instance_id()));
    assert!(flanking.contains(&granted.instance_id()));
    let origins = (0..calculated.abilities.len()).filter_map(|index| calculated.abilities.origin(index))
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(origins.len(), 3, "printed and independently registered grants have distinct origins");
}

#[test]
fn unrelated_targets_skip_recursive_graveyard_condition_queries() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Conditional Source")
        .card_types(vec![CardType::Creature])
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let mut graveyard = Vec::new();
    for card_type in [
        CardType::Artifact,
        CardType::Land,
        CardType::Instant,
        CardType::Sorcery,
    ] {
        let card = CardBuilder::new(CardId::new(), "Graveyard Probe")
            .card_types(vec![card_type])
            .build();
        graveyard.push(game.create_object_from_card(&card, alice, Zone::Graveyard));
    }
    let probe = graveyard[0];
    let chars = game.calculated_characteristics(probe).unwrap();
    let effect = ContinuousEffect::new(
        source,
        alice,
        EffectTarget::Source,
        Modification::AddAbility(StaticAbility::flying()),
    )
    .with_condition(crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
        player: PlayerFilter::You,
        count: 4,
    });
    for target in [
        EffectTarget::Source,
        EffectTarget::Specific(source),
        EffectTarget::AllPermanents,
        EffectTarget::AllCreatures,
        EffectTarget::AttachedTo(source),
    ] {
        let mut effect = effect.clone();
        effect.applies_to = target;
        let before = game.work_counters().characteristics_full_recomputes;
        assert!(!effect_applies_to_direct(
            &effect,
            game.object(probe).unwrap(),
            &chars,
            game.objects_map(),
            &game.battlefield,
            game.commander_objects(),
            &game
        ));
        assert_eq!(
            game.work_counters().characteristics_full_recomputes,
            before,
            "unrelated targets must not evaluate the graveyard condition"
        );
    }
    let source_chars = game.calculated_characteristics(source).unwrap();
    assert!(effect_applies_to_direct(
        &effect,
        game.object(source).unwrap(),
        &source_chars,
        game.objects_map(),
        &game.battlefield,
        game.commander_objects(),
        &game
    ));
}

fn delirium_layer_fixture() -> (GameState, ObjectId, Vec<ObjectId>) {
    use crate::static_abilities::{Anthem, GrantAbility};
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(91_000), "Conditional creature")
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(1, 1))
        .build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let condition = crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
        player: PlayerFilter::You,
        count: 4,
    };
    game.object_mut(source).unwrap().abilities = vec![
        Ability::static_ability(StaticAbility::new(
            Anthem::for_source(2, 2).with_condition(condition.clone()),
        )),
        Ability::static_ability(StaticAbility::new(
            GrantAbility::source(StaticAbility::flying()).with_condition(condition.clone()),
        )),
        Ability::static_ability(StaticAbility::new(
            GrantAbility::source(StaticAbility::must_attack()).with_condition(condition),
        )),
    ]
    .into();
    let mut graveyard = Vec::new();
    for (index, card_type) in [CardType::Land, CardType::Instant, CardType::Sorcery]
        .into_iter()
        .enumerate()
    {
        let card = CardBuilder::new(CardId::from_raw(91_001 + index as u32), "Graveyard card")
            .card_types(vec![card_type])
            .build();
        graveyard.push(game.create_object_from_card(&card, alice, Zone::Graveyard));
    }
    (game, source, graveyard)
}

fn assert_delirium_characteristics(game: &GameState, source: ObjectId, active: bool) {
    use crate::static_abilities::StaticAbilityId;
    let chars = game.calculated_characteristics(source).unwrap();
    assert_eq!(
        (chars.power, chars.toughness),
        if active {
            (Some(3), Some(3))
        } else {
            (Some(1), Some(1))
        }
    );
    for ability in [StaticAbilityId::Flying, StaticAbilityId::MustAttack] {
        assert_eq!(
            chars.static_abilities.iter().any(|a| a.id() == ability),
            active
        );
    }
}

#[test]
fn delirium_ability_ordering_preserves_graveyard_type_changes_and_zone_changes() {
    let (mut game, source, graveyard) = delirium_layer_fixture();
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, false);

    // A real layer-4 effect makes the land also an artifact. Printed types
    // alone would incorrectly leave delirium off.
    let type_effect = game
        .effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            source,
            PlayerId::from_index(0),
            EffectTarget::Specific(graveyard[0]),
            Modification::AddCardTypes(vec![CardType::Artifact]),
        ));
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, true);
    game.effect_store
        .continuous_effects
        .remove_effect(type_effect);
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, false);

    let artifact = CardBuilder::new(CardId::from_raw(91_010), "Fourth type")
        .card_types(vec![CardType::Artifact])
        .build();
    let artifact =
        game.create_object_from_card(&artifact, PlayerId::from_index(0), Zone::Graveyard);
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, true);
    game.turn.phase = crate::game_state::Phase::Combat;
    game.turn.step = Some(crate::game_state::Step::BeginCombat);
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, true);
    game.move_object_by_effect(artifact, Zone::Exile).unwrap();
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, false);
}

#[test]
fn prewarming_does_not_publish_an_enclosing_characteristic_snapshot() {
    let (mut game, source, graveyard) = delirium_layer_fixture();
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
        source, PlayerId::from_index(0), EffectTarget::Specific(graveyard[0]),
        Modification::AddCardTypes(vec![CardType::Artifact])));
    game.update_static_ability_effects().unwrap();
    let initial = initial_characteristics(game.object(source).unwrap(), game.turn.turn_number);
    assert_eq!(initial.power, Some(1));
    let guard = CharacteristicCalculationGuard::begin(&game, source, &initial);
    game.prewarm_calculated_characteristics(&[source, graveyard[0]]);
    assert_eq!(game.calculated_power(source), Some(1), "nested queries retain the current layer view");
    drop(guard);
    assert_eq!(game.calculated_power(source), Some(3));
    let before = game.work_counters().characteristics_full_recomputes;
    assert_eq!(game.calculated_power(source), Some(3));
    assert_eq!(game.work_counters().characteristics_full_recomputes, before,
        "final characteristics still use the ordinary cache");
}

#[test]
fn derived_view_memos_retain_layer_context_without_publishing_intermediate_results() {
    use crate::derived_view::DerivedGameView;
    let mut observations = Vec::new();
    for explicit_effects in [false, true] {
        for initially_warm in [false, true] {
            for prewarm in [false, true] {
                let (mut game, source, graveyard) = delirium_layer_fixture();
                let alice = PlayerId::from_index(0);
                game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                    source, alice, EffectTarget::Specific(graveyard[0]),
                    Modification::AddCardTypes(vec![CardType::Artifact])));
                game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                    source, alice, EffectTarget::Specific(source), Modification::AddAbilityGeneric(
                        Ability::activated(crate::cost::TotalCost::free(),
                            vec![crate::effect::Effect::gain_life(1)]))));
                game.update_static_ability_effects().unwrap();
                let view = if explicit_effects {
                    DerivedGameView::from_effects(&game, game.try_all_continuous_effects().unwrap())
                } else { DerivedGameView::from_refreshed_state(&game) };
                let read = || {
                    let power = view.calculated_characteristics(source).unwrap().power;
                    let flying = view.abilities_rc(source).unwrap().iter().any(|ability|
                        matches!(&ability.kind, AbilityKind::Static(ability)
                            if ability.id() == crate::static_abilities::StaticAbilityId::Flying));
                    let static_flying = view.static_abilities_rc(source).unwrap().iter().any(|ability|
                        ability.id() == crate::static_abilities::StaticAbilityId::Flying);
                    let activated = view.ability_index_summary(source).unwrap()
                        .activated_ability_indices().len();
                    (power, flying, static_flying, activated)
                };
                if initially_warm { assert_eq!(read(), (Some(3), true, true, 1)); }
                let complete = game.calculated_characteristics(source).unwrap();
                let initial = initial_characteristics(game.object(source).unwrap(), game.turn.turn_number);
                let creature_filter = crate::filter::ObjectFilter::creature()
                    .in_zone(crate::zone::Zone::Battlefield);
                let controlled_filter = creature_filter.clone().you_control();
                let filter_context = game.filter_context_for(alice, None);
                let candidate_presence = || (
                    view.candidate_ids_for_filter_with_context(&creature_filter, &filter_context)
                        .contains(&source),
                    view.candidate_ids_for_filter_with_context(&controlled_filter, &filter_context)
                        .contains(&source));
                if initially_warm { assert_eq!(candidate_presence(), (true, true)); }
                let guard = CharacteristicCalculationGuard::begin(&game, source, &initial);
                if prewarm { view.prewarm_characteristics_forced(&[source, graveyard[0]]); }
                let during = read();
                assert_eq!(candidate_presence(), (true, true));
                guard.update(&complete);
                assert_eq!(read(), (Some(3), true, true, 1),
                    "updated completed layer: explicit={explicit_effects},warm={initially_warm},prewarm={prewarm}");
                let mut changed = initial.clone();
                changed.controller = PlayerId::from_index(1);
                changed.card_types = vec![CardType::Artifact].into();
                guard.update(&changed);
                assert_eq!(read(), (Some(1), false, false, 0));
                assert_eq!(candidate_presence(), (false, false),
                    "updated artifact/controller: explicit={explicit_effects},warm={initially_warm},prewarm={prewarm}");
                guard.update(&initial);
                assert_eq!(read(), (Some(1), false, false, 0));
                assert_eq!(candidate_presence(), (true, true));
                drop(guard);
                let after = read();
                assert_eq!(candidate_presence(), (true, true));
                observations.push((explicit_effects, initially_warm, prewarm, during, after));
            }
        }
    }
    assert!(observations.iter().all(|(_, _, _, during, after)|
        *during == (Some(1), false, false, 0) && *after == (Some(3), true, true, 1)),
        "{observations:?}");
}

#[test]
fn recursive_characteristic_context_is_scoped_to_its_game_snapshot() {
    use crate::derived_view::DerivedGameView;
    let mut observations = Vec::new();
    for explicit_effects in [false, true] {
        for initially_warm in [false, true] {
            let (mut game, source, graveyard) = delirium_layer_fixture();
            let alice = PlayerId::from_index(0);
            game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                source, alice, EffectTarget::Specific(graveyard[0]),
                Modification::AddCardTypes(vec![CardType::Artifact])));
            game.refresh_continuous_state().unwrap();
            let mut other = game.clone();
            other.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                source, alice, EffectTarget::Specific(source), Modification::ModifyPower(4)));
            other.refresh_continuous_state().unwrap();
            assert_eq!(game.calculated_characteristics(source).unwrap().power, Some(3));
            let other_effects = other.try_all_continuous_effects().unwrap();
            let view = if explicit_effects {
                DerivedGameView::from_effects(&other, other_effects.clone())
            } else { DerivedGameView::from_refreshed_state(&other) };
            let read_other = || (
                other.calculated_characteristics(source).unwrap().power,
                other.calculated_characteristics_with_effects(source, &other_effects).unwrap().power,
                view.calculated_characteristics(source).unwrap().power);
            if initially_warm { assert_eq!(read_other(), (Some(7), Some(7), Some(7))); }
            let initial = initial_characteristics(game.object(source).unwrap(), game.turn.turn_number);
            let guard = CharacteristicCalculationGuard::begin(&game, source, &initial);
            assert_eq!(game.calculated_characteristics(source).unwrap().power, Some(1),
                "recursive reads in the owning snapshot must retain its current layer");
            let during = read_other();
            drop(guard);
            let after = read_other();
            observations.push((explicit_effects, initially_warm, during, after));
        }
    }
    assert!(observations.iter().all(|(_, _, during, after)|
        *during == (Some(7), Some(7), Some(7)) && *after == (Some(7), Some(7), Some(7))),
        "{observations:?}");
}

#[test]
fn replacement_publication_preserves_complete_characteristic_cache() {
    let mut observations = Vec::new();
    for stage in ["pure", "static", "replacement", "cant", "full"] {
        for warm_graveyard in [false, true] {
            let (mut game, source, graveyard) = delirium_layer_fixture();
            game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                source, PlayerId::from_index(0), EffectTarget::Specific(graveyard[0]),
                Modification::AddCardTypes(vec![CardType::Artifact])));
            if stage == "full" { game.refresh_continuous_state().unwrap(); }
            else if stage != "pure" {
                game.update_static_ability_effects().unwrap();
                if stage == "replacement" { game.update_replacement_effects().unwrap(); }
                if stage == "cant" { game.update_cant_effects(); }
            }
            if warm_graveyard { let _types = game.calculated_card_types(graveyard[0]); }
            let cached = game.calculated_characteristics(source).unwrap();
            let effects = game.try_all_continuous_effects().unwrap();
            let direct = game.calculated_characteristics_with_effects(source, &effects).unwrap();
            let grave_types = game.calculated_card_types(graveyard[0]);
            let direct_after_grave = game.calculated_characteristics_with_effects(source, &effects).unwrap();
            assert!(grave_types.contains(&CardType::Artifact), "stage={stage},warm={warm_graveyard}");
            observations.push((stage, warm_graveyard, cached.power, direct.power, direct_after_grave.power));
        }
    }
    assert!(observations.iter().all(|(_, _, cached, direct, after)|
        *cached == Some(3) && *direct == Some(3) && *after == Some(3)), "{observations:?}");
}

#[test]
fn delirium_ability_ordering_preserves_source_ability_removal() {
    let (mut game, source, graveyard) = delirium_layer_fixture();
    game.effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            source,
            PlayerId::from_index(0),
            EffectTarget::Specific(graveyard[0]),
            Modification::AddCardTypes(vec![CardType::Artifact]),
        ));
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, true);
    let removal = game
        .effect_store
        .continuous_effects
        .add_effect(ContinuousEffect::new(
            ObjectId::from_raw(91_020),
            PlayerId::from_index(0),
            EffectTarget::Specific(source),
            Modification::RemoveAllAbilities,
        ));
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, false);
    game.effect_store.continuous_effects.remove_effect(removal);
    game.refresh_continuous_state().unwrap();
    assert_delirium_characteristics(&game, source, true);
}


fn check_copy_cost_layer_filter(exact_cost: bool, fallback: bool) {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let original_cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(5)]);
    let copied_cost = ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]);
    let original = CardBuilder::new(CardId::new(), "Original creature")
        .card_types(vec![CardType::Creature]).mana_cost(original_cost)
        .power_toughness(PowerToughness::fixed(1, 1)).build();
    let copied = CardBuilder::new(CardId::new(), "Copied creature")
        .card_types(vec![CardType::Creature]).mana_cost(copied_cost.clone())
        .power_toughness(PowerToughness::fixed(4, 4)).build();
    let original_id = game.create_object_from_card(&original, alice, Zone::Battlefield);
    let copied_id = game.create_object_from_card(&copied, alice, Zone::Battlefield);
    let values = crate::snapshot::CopiableValues::from_object(game.object(copied_id).unwrap());
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
        original_id, alice, EffectTarget::Specific(original_id), Modification::CopyOf {
            target_id: copied_id, copiable_values: Box::new(values),
            preserve_source_abilities: false, name_override: None,
            name_override_surface: None, add_supertypes: Vec::new(),
        },
    ).until(Until::EndOfTurn));
    let mut filter = ObjectFilter::default();
    filter.specific = Some(original_id);
    if exact_cost { filter.exact_mana_cost = Some(copied_cost); }
    else { filter.mana_value = Some(crate::filter::Comparison::Equal(2)); }
    if fallback { filter.any_of = vec![ObjectFilter::default()]; }
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
        original_id, alice, EffectTarget::Filter(filter),
        Modification::ModifyPowerToughness { power: 3, toughness: 3 },
    ));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_power(original_id), Some(7), "filter sees copied cost");
    assert_eq!(crate::filter::object_current_mana_value(&game, original_id), 2);
    game.effect_store.continuous_effects.cleanup_end_of_turn();
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_power(original_id), Some(1), "filter no longer matches after copy expiry");
    assert_eq!(crate::filter::object_current_mana_value(&game, original_id), 5);
}

#[test]
fn temporary_copy_cost_selects_direct_mana_value_filter() {
    check_copy_cost_layer_filter(false, false);
}

#[test]
fn temporary_copy_cost_selects_fallback_mana_value_filter() {
    check_copy_cost_layer_filter(false, true);
}

#[test]
fn temporary_copy_cost_selects_direct_exact_cost_filter() {
    check_copy_cost_layer_filter(true, false);
}

#[test]
fn temporary_copy_cost_selects_fallback_exact_cost_filter() {
    check_copy_cost_layer_filter(true, true);
}


#[test]
fn legacy_enchant_materialization_keeps_identity_across_reads_and_copy() {
    fn enchant_id(chars: &CalculatedCharacteristics) -> crate::static_abilities::StaticAbilityInstanceId {
        let abilities: Vec<_> = chars.static_abilities.iter()
            .filter(|ability| ability.enchant_filter().is_some()).collect();
        assert_eq!(abilities.len(), 1, "exactly one enchant ability is present");
        abilities[0].instance_id()
    }

    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(9891), "Legacy Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura]).build();
    let mut aura = Object::from_card(ObjectId::from_raw(9891), &card, alice, Zone::Hand);
    let filter = crate::object::AuraAttachmentFilter::from(ObjectFilter::creature());
    aura.aura_attach_filter = Some(filter.clone().into());
    let checkpoint = aura.clone();
    let first = initial_text_box_characteristics(&aura);
    let repeated = initial_text_box_characteristics(&aura);
    let restored = initial_text_box_characteristics(&checkpoint);

    let values = CopiableValues::from_object(&aura);
    let mut copy_first = initial_text_box_characteristics(&aura);
    let mut copy_second = initial_text_box_characteristics(&aura);
    copy_characteristics_from_copiable_values(
        &values, &mut copy_first, false, &None, &None, &[], None);
    copy_characteristics_from_copiable_values(
        &values, &mut copy_second, false, &None, &None, &[], None);

    let creature_card = CardBuilder::new(CardId::from_raw(9892), "Bestow Creature")
        .card_types(vec![CardType::Enchantment, CardType::Creature]).build();
    let mut bestow = Object::from_card(ObjectId::from_raw(9892), &creature_card, alice, Zone::Stack);
    bestow.apply_bestow_cast_overlay();
    let bestow_first = initial_text_box_characteristics(&bestow);
    let bestow_second = initial_text_box_characteristics(&bestow.clone());
    assert!(bestow.end_bestow_cast_overlay());
    assert!(initial_text_box_characteristics(&bestow).static_abilities.iter()
        .all(|ability| ability.enchant_filter().is_none()),
        "ending bestow removes the overlay enchant ability");

    let observations = [
        ("legacy repeated read", enchant_id(&first), enchant_id(&repeated)),
        ("checkpoint clone", enchant_id(&first), enchant_id(&restored)),
        ("copy materialization", enchant_id(&copy_first), enchant_id(&copy_second)),
        ("bestow overlay", enchant_id(&bestow_first), enchant_id(&bestow_second)),
    ];
    assert!(observations.iter().all(|(_, first, next)| first == next),
        "reading the same ability occurrence must preserve its identity: {observations:?}");
}


#[test]
fn synthesized_continuous_and_counter_abilities_keep_identity_across_reads() {
    use crate::static_abilities::StaticAbilityId;
    let mut observations = Vec::new();
    for (modification, expected) in [
        (Modification::restriction(RestrictionKind::CantBeBlocked), StaticAbilityId::Unblockable),
        (Modification::restriction(RestrictionKind::CantAttack), StaticAbilityId::CantAttack),
        (Modification::restriction(RestrictionKind::CantBlock), StaticAbilityId::CantBlock),
        (Modification::restriction(RestrictionKind::DoesntUntap), StaticAbilityId::DoesntUntap),
        (Modification::SetAuraAttachmentFilter(
            crate::object::AuraAttachmentFilter::from(ObjectFilter::creature()).into()),
            StaticAbilityId::Enchant),
    ] {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Restriction Recipient")
            .card_types(vec![CardType::Creature]).build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            object, alice, EffectTarget::Specific(object), modification));
        let effects = game.try_all_continuous_effects().expect("finite effect discovery");
        let checkpoint = game.clone();
        let first = game.calculated_characteristics_with_effects(object, &effects)
            .expect("first calculation has recipient");
        let repeated = game.calculated_characteristics_with_effects(object, &effects)
            .expect("repeated calculation has recipient");
        let restored = checkpoint.calculated_characteristics_with_effects(object, &effects)
            .expect("checkpoint calculation has recipient");
        let ids = [&first, &repeated, &restored].map(|chars| {
            let abilities: Vec<_> = chars.static_abilities.iter()
                .filter(|ability| ability.id() == expected).collect();
            assert_eq!(abilities.len(), 1, "one synthesized {expected:?} ability");
            abilities[0].instance_id()
        });
        observations.push((format!("{expected:?}"), ids));
    }

    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Counter Recipient")
        .card_types(vec![CardType::Creature]).build();
    let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.object_mut(object).expect("counter recipient exists").add_counters(CounterType::Flying, 1);
    let checkpoint = game.clone();
    let counter_ids = [&game, &game, &checkpoint].map(|state| {
        let chars = state.calculated_characteristics_with_effects(object, &[])
            .expect("counter calculation has recipient");
        let abilities: Vec<_> = chars.static_abilities.iter()
            .filter(|ability| ability.id() == StaticAbilityId::Flying).collect();
        assert_eq!(abilities.len(), 1, "one flying ability from counter");
        abilities[0].instance_id()
    });
    observations.push(("Flying counter".to_string(), counter_ids));
    assert!(observations.iter().all(|(_, ids)| ids[0] == ids[1] && ids[0] == ids[2]),
        "recalculating registered ability occurrences must preserve identity: {observations:?}");
}


#[test]
fn enchant_metadata_preserves_explicit_abilities_independent_occurrences_and_bestow_copy() {
    fn enchant_id(chars: &CalculatedCharacteristics) -> crate::static_abilities::StaticAbilityInstanceId {
        let abilities: Vec<_> = chars.static_abilities.iter()
            .filter(|a| a.enchant_filter().is_some()).collect();
        assert_eq!(abilities.len(), 1, "one enchant occurrence");
        abilities[0].instance_id()
    }
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Metadata Aura")
        .card_types(vec![CardType::Enchantment]).subtypes(vec![Subtype::Aura]).build();
    let filter = crate::object::AuraAttachmentFilter::from(ObjectFilter::creature());
    let mut first = Object::from_card(ObjectId::from_raw(9893), &card, alice, Zone::Hand);
    let mut second = Object::from_card(ObjectId::from_raw(9894), &card, alice, Zone::Hand);
    first.aura_attach_filter = Some(filter.clone().into());
    second.aura_attach_filter = Some(filter.clone().into());
    assert_ne!(enchant_id(&initial_text_box_characteristics(&first)),
        enchant_id(&initial_text_box_characteristics(&second)),
        "independent metadata registrations remain distinct");
    let explicit = StaticAbility::enchant(filter);
    first.abilities = std::sync::Arc::new(vec![crate::ability::Ability::static_ability(explicit.clone())]);
    assert_eq!(enchant_id(&initial_text_box_characteristics(&first)), explicit.instance_id(),
        "metadata does not replace an existing printed enchant ability");

    let creature = CardBuilder::new(CardId::new(), "Bestow Copy Source")
        .card_types(vec![CardType::Creature, CardType::Enchantment]).build();
    let mut bestow = Object::from_card(ObjectId::from_raw(9895), &creature, alice, Zone::Stack);
    bestow.apply_bestow_cast_overlay();
    assert_eq!(initial_text_box_characteristics(&bestow).static_abilities.iter()
        .filter(|a| a.enchant_filter().is_some()).count(), 1);
    let underlying = CopiableValues::from_object(&bestow);
    assert!(underlying.aura_attach_filter.is_none(), "temporary bestow filter is not copied");
    assert!(underlying.abilities.iter().all(|ability| !matches!(
        &ability.kind, crate::ability::AbilityKind::Static(a) if a.enchant_filter().is_some()
    )), "underlying copiable abilities omit temporary bestow enchant");
    assert!(underlying.card_types.contains(&CardType::Creature));
    assert!(!underlying.subtypes.contains(&Subtype::Aura));
}

#[test]
fn registered_restrictions_preserve_independent_occurrences_and_removal() {
    for kind in [RestrictionKind::CantBeBlocked, RestrictionKind::CantAttack,
        RestrictionKind::CantBlock, RestrictionKind::DoesntUntap] {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Restriction Recipient")
            .card_types(vec![CardType::Creature]).build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let first = RegisteredRestriction::new(kind);
        let second = RegisteredRestriction::new(kind);
        let expected = first.ability().id();
        let original_ids = [first.ability().instance_id(), second.ability().instance_id()];
        assert_ne!(original_ids[0], original_ids[1], "independent registrations");
        let first_effect = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            object, alice, EffectTarget::Specific(object), Modification::Restriction(first)));
        let second_effect = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            object, alice, EffectTarget::Specific(object), Modification::Restriction(second)));
        let read_ids = |state: &GameState| {
            let effects = state.try_all_continuous_effects().expect("finite discovery");
            let chars = state.calculated_characteristics_with_effects(object, &effects)
                .expect("registered recipient exists");
            chars.static_abilities.iter().filter(|ability| ability.id() == expected)
                .map(|ability| ability.instance_id()).collect::<Vec<_>>()
        };
        let checkpoint = game.clone();
        assert_eq!(read_ids(&game), original_ids, "both occurrences survive calculation");
        assert_eq!(read_ids(&game), original_ids, "refresh preserves occurrences");
        assert_eq!(read_ids(&checkpoint), original_ids, "checkpoint preserves occurrences");
        game.effect_store.continuous_effects.remove_effect(first_effect);
        assert_eq!(read_ids(&game), [original_ids[1]], "removal affects only its occurrence");
        assert_eq!(read_ids(&checkpoint), original_ids, "snapshot is isolated from removal");
        game.effect_store.continuous_effects.remove_effect(second_effect);
        assert!(read_ids(&game).is_empty(), "no restriction remains after both removals");
        let replacement = RegisteredRestriction::new(kind);
        let new_id = replacement.ability().instance_id();
        assert!(!original_ids.contains(&new_id), "new registration is a new occurrence");
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            object, alice, EffectTarget::Specific(object), Modification::Restriction(replacement)));
        assert_eq!(read_ids(&game), [new_id]);
    }
}

#[test]
fn keyword_counter_occurrences_preserve_printed_abilities_and_lifetime() {
    use crate::static_abilities::StaticAbilityId;
    for printed in [false, true] {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Counter Recipient")
            .card_types(vec![CardType::Creature]).build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let printed_flying = StaticAbility::flying();
        let printed_id = printed_flying.instance_id();
        if printed {
            game.object_mut(object).expect("recipient exists").abilities = std::sync::Arc::new(
                vec![crate::ability::Ability::static_ability(printed_flying)]);
        }
        game.object_mut(object).expect("recipient exists").counters.insert(CounterType::Flying, 2);
        let read = |state: &GameState| {
            let effects = state.try_all_continuous_effects().expect("finite counter discovery");
            let chars = state.calculated_characteristics_with_effects(object, &effects)
                .expect("counter recipient exists");
            chars.abilities.iter().enumerate().filter_map(|(index, ability)| {
                let AbilityKind::Static(ability) = &ability.kind else { return None; };
                (ability.id() == StaticAbilityId::Flying).then(|| (
                    ability.instance_id(), chars.abilities.origin(index)
                        .expect("every calculated ability retains its origin").clone()))
            }).collect::<Vec<_>>()
        };
        let original = read(&game);
        assert_eq!(original.len(), 2 + usize::from(printed),
            "each counter is independent of other counters and printed flying");
        let counter_ids: Vec<_> = original.iter().filter(|(id, _)| !printed || *id != printed_id)
            .map(|(id, _)| *id).collect();
        assert_ne!(counter_ids[0], counter_ids[1], "independent counter payloads");
        for (id, origin) in &original {
            if printed && *id == printed_id {
                assert!(matches!(origin, AbilityOrigin::Printed(0)), "printed origin is preserved");
            } else {
                assert!(!matches!(origin, AbilityOrigin::Printed(_) | AbilityOrigin::Effect { .. }),
                    "counter occurrence cannot inherit a printed index or another effect: {origin:?}");
                assert_eq!(origin.granting_source(), None, "a counter has no external grantor");
            }
        }
        let checkpoint = game.clone();
        assert_eq!(read(&game), original, "refresh preserves counter occurrences");
        assert_eq!(read(&checkpoint), original, "checkpoint preserves counter occurrences");
        assert_eq!(game.object_mut(object).expect("recipient exists")
            .remove_counters(CounterType::Flying, 1), 1);
        let remaining = read(&game);
        assert_eq!(remaining.len(), 1 + usize::from(printed));
        assert!(remaining.iter().all(|entry| original.contains(entry)),
            "partial removal retains surviving identity and origin");
        assert_eq!(read(&checkpoint), original, "removal does not mutate the checkpoint");
        assert_eq!(game.object_mut(object).expect("recipient exists")
            .remove_counters(CounterType::Flying, 1), 1);
        assert_eq!(read(&game).len(), usize::from(printed));
        game.object_mut(object).expect("recipient exists").add_counters(CounterType::Flying, 1);
        let replacement = read(&game);
        assert_eq!(replacement.len(), 1 + usize::from(printed));
        for entry in replacement {
            if printed && entry.0 == printed_id {
                assert!(original.contains(&entry), "printed identity survives counter mutations");
            } else {
                assert!(!original.iter().any(|old| old.0 == entry.0 || old.1 == entry.1),
                    "new counter must not reuse a removed occurrence or origin");
            }
        }
    }
}

#[test]
fn cloned_restriction_payload_keeps_each_registration_origin() {
    for kind in [RestrictionKind::CantBeBlocked, RestrictionKind::CantAttack,
        RestrictionKind::CantBlock, RestrictionKind::DoesntUntap] {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Restriction Recipient")
            .card_types(vec![CardType::Creature]).build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let payload = RegisteredRestriction::new(kind);
        let expected = payload.ability().id();
        let template = Modification::Restriction(payload);
        let first = game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            object, alice, EffectTarget::Specific(object), template.clone()));
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            object, alice, EffectTarget::Specific(object), template));
        let read = |state: &GameState| {
            let effects = state.try_all_continuous_effects().expect("finite effect discovery");
            let chars = state.calculated_characteristics_with_effects(object, &effects)
                .expect("restriction recipient exists");
            chars.abilities.iter().enumerate().filter_map(|(index, ability)| {
                let AbilityKind::Static(ability) = &ability.kind else { return None; };
                (ability.id() == expected).then(|| (ability.instance_id(),
                    chars.abilities.origin(index).expect("registered grant has origin").clone()))
            }).collect::<Vec<_>>()
        };
        let original = read(&game);
        assert_eq!(original.len(), 2, "each registration survives a shared payload");
        assert_ne!(original[0].1, original[1].1, "different effects have different origins");
        assert_eq!(read(&game), original, "refresh preserves both registrations");
        let checkpoint = game.clone();
        assert_eq!(read(&checkpoint), original, "checkpoint retains both origins");
        game.effect_store.continuous_effects.remove_effect(first);
        assert_eq!(read(&game), [original[1].clone()], "only removed effect disappears");
        assert_eq!(read(&checkpoint), original, "checkpoint retains removed registration");
    }
}

#[test]
fn ability_counter_timestamp_rebases_same_kind_without_merging_occurrences() {
    for same_kind in [false, true] {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Layer Counter Recipient")
            .card_types(vec![CardType::Creature]).build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let exalted = CounterType::Named("exalted".into());
        let triggered_count = |state: &GameState| {
            let effects = state.try_all_continuous_effects().expect("finite layer discovery");
            let chars = state.calculated_characteristics_with_effects(object, &effects)
                .expect("counter recipient exists");
            chars.abilities.iter().filter(|ability| matches!(
                ability.kind, AbilityKind::Triggered(_))).count()
        };
        game.add_counters(object, exalted, 1).expect("positive placement produces event");
        assert_eq!(triggered_count(&game), 1);
        let original_timestamp = game.effect_store.continuous_effects
            .get_counter_timestamp(object, exalted).expect("placement records timestamp");
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            object, alice, EffectTarget::Specific(object), Modification::RemoveAllAbilities));
        assert_eq!(triggered_count(&game), 0, "later ability loss removes older counter ability");
        let later_kind = if same_kind { exalted } else { CounterType::Flying };
        game.add_counters(object, later_kind, 1).expect("later positive placement produces event");
        let updated_timestamp = game.effect_store.continuous_effects
            .get_counter_timestamp(object, exalted).expect("original counter remains");
        if same_kind {
            assert!(updated_timestamp > original_timestamp,
                "CR 613.7c rebases every counter of the same kind");
            assert_eq!(triggered_count(&game), 2,
                "both independent exalted counter abilities apply after older ability loss");
        } else {
            assert_eq!(updated_timestamp, original_timestamp,
                "placing another kind does not rebase exalted counters");
            assert_eq!(triggered_count(&game), 0,
                "older exalted remains removed when only a different kind was placed later");
        }
    }
}

#[test]
fn derived_ability_view_preserves_keyword_counters_without_continuous_effects() {
    use crate::static_abilities::StaticAbilityId;
    for (counter, expected) in [
        (CounterType::Flying, StaticAbilityId::Flying),
        (CounterType::Haste, StaticAbilityId::Haste),
        (CounterType::Hexproof, StaticAbilityId::Hexproof),
        (CounterType::Indestructible, StaticAbilityId::Indestructible),
    ] {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Counter Query Recipient")
            .card_types(vec![CardType::Creature]).build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.add_counters(object, counter, 1).expect("positive placement produces an event");
        game.refresh_continuous_state().expect("finite complete refresh succeeds");
        let effects = game.try_all_continuous_effects().expect("finite effect discovery");
        assert!(effects.is_empty(), "counter ability must work without a continuous instruction");
        let chars = game.calculated_characteristics_with_effects(object, &effects)
            .expect("counter recipient exists");
        assert!(chars.static_abilities.iter().any(|ability| ability.id() == expected),
            "full layer calculation grants the keyword");
        let view = crate::derived_view::DerivedGameView::new(&game);
        assert!(view.object_has_static_ability_id(object, expected),
            "derived ability queries must retain the calculated counter keyword {expected:?}");
        let abilities = view.abilities_rc(object).expect("view recipient exists");
        assert_eq!(abilities.iter().filter(|ability| matches!(&ability.kind,
            AbilityKind::Static(ability) if ability.id() == expected)).count(), 1,
            "derived ability list retains the counter keyword");
    }
}



#[test]
fn keyword_counter_abilities_apply_in_every_card_zone() {
    use crate::static_abilities::StaticAbilityId;
    let cases = [
        (
            CounterType::Deathtouch,
            Some(StaticAbilityId::Deathtouch),
            0,
        ),
        (CounterType::Flying, Some(StaticAbilityId::Flying), 0),
        (
            CounterType::FirstStrike,
            Some(StaticAbilityId::FirstStrike),
            0,
        ),
        (
            CounterType::DoubleStrike,
            Some(StaticAbilityId::DoubleStrike),
            0,
        ),
        (CounterType::Hexproof, Some(StaticAbilityId::Hexproof), 0),
        (
            CounterType::Indestructible,
            Some(StaticAbilityId::Indestructible),
            0,
        ),
        (CounterType::Lifelink, Some(StaticAbilityId::Lifelink), 0),
        (CounterType::Menace, Some(StaticAbilityId::Menace), 0),
        (CounterType::Reach, Some(StaticAbilityId::Reach), 0),
        (CounterType::Trample, Some(StaticAbilityId::Trample), 0),
        (CounterType::Vigilance, Some(StaticAbilityId::Vigilance), 0),
        (CounterType::Haste, Some(StaticAbilityId::Haste), 0),
        (CounterType::Decayed, Some(StaticAbilityId::CantBlock), 2),
        (CounterType::Named("exalted".into()), None, 2),
    ];
    for zone in [
        Zone::Battlefield,
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
        Zone::Stack,
    ] {
        for (kind, keyword, triggers) in cases {
            let mut game = dynamic_value_test_game();
            let alice = PlayerId::from_index(0);
            let card = CardBuilder::new(CardId::new(), "Keyword Counter Recipient")
                .card_types(vec![CardType::Creature])
                .build();
            let object = game.create_object_from_card(&card, alice, zone);
            // Construct an existing counter-bearing card; moving a card between zones
            // would remove its counters and would test a different rule.
            game.object_mut(object)
                .expect("recipient exists")
                .counters
                .insert(kind, 2);
            let effects = game.try_all_continuous_effects().expect("finite discovery");
            let chars = game
                .calculated_characteristics_with_effects(object, &effects)
                .expect("counter-bearing card exists");
            let view = crate::derived_view::DerivedGameView::new(&game);
            let abilities = view.abilities_rc(object).expect("derived card exists");
            if let Some(keyword) = keyword {
                assert_eq!(
                    chars
                        .static_abilities
                        .iter()
                        .filter(|a| a.id() == keyword)
                        .count(),
                    2,
                    "two independent {kind:?} counters grant abilities in {zone:?}"
                );
                assert!(
                    view.object_has_static_ability_id(object, keyword),
                    "derived keyword query must agree in {zone:?}"
                );
                assert_eq!(
                    abilities
                        .iter()
                        .filter(|a| matches!(&a.kind,
                    AbilityKind::Static(a) if a.id() == keyword))
                        .count(),
                    2
                );
            }
            assert_eq!(
                abilities
                    .iter()
                    .filter(|a| matches!(&a.kind, AbilityKind::Triggered(_)))
                    .count(),
                triggers,
                "{kind:?} in {zone:?}"
            );
        }
    }
}

#[test]
fn shadow_counters_affect_real_blocking_queries_and_counter_removal() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let card = CardBuilder::new(CardId::new(), "Shadow Counter Combatant")
        .card_types(vec![CardType::Creature])
        .build();
    let attacker = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let blocker = game.create_object_from_card(&card, bob, Zone::Battlefield);
    let normal_attacker = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let shadow = CounterType::Named("shadow".into());
    let legal = |state: &GameState, attacker, blocker| {
        crate::rules::combat::can_block(
            state.object(attacker).expect("attacker exists"),
            state.object(blocker).expect("blocker exists"),
            state,
        )
    };
    assert!(
        legal(&game, attacker, blocker),
        "ordinary creatures can block"
    );
    game.add_counters(attacker, shadow, 2)
        .expect("attacker counter placement");
    assert!(
        !legal(&game, attacker, blocker),
        "normal cannot block shadow"
    );
    game.add_counters(blocker, shadow, 1)
        .expect("blocker counter placement");
    assert!(legal(&game, attacker, blocker), "shadow blocks shadow");
    assert!(
        !legal(&game, normal_attacker, blocker),
        "shadow cannot block ordinary"
    );
    assert_eq!(
        game.remove_counters(attacker, shadow, 1, None, None)
            .expect("attacker counter removed")
            .0,
        1
    );
    assert!(
        legal(&game, attacker, blocker),
        "surviving shadow counter still grants ability"
    );
    assert_eq!(
        game.remove_counters(attacker, shadow, 1, None, None)
            .expect("attacker counter removed")
            .0,
        1
    );
    assert!(
        !legal(&game, attacker, blocker),
        "shadow blocker cannot block former shadow"
    );
    assert_eq!(
        game.remove_counters(blocker, shadow, 1, None, None)
            .expect("blocker counter removed")
            .0,
        1
    );
    assert!(legal(&game, attacker, blocker), "both keywords gone");
}

#[test]
fn shadow_counter_abilities_apply_in_every_card_zone() {
    use crate::static_abilities::StaticAbilityId;
    for zone in [
        Zone::Battlefield,
        Zone::Hand,
        Zone::Library,
        Zone::Graveyard,
        Zone::Exile,
        Zone::Command,
        Zone::Stack,
    ] {
        let mut game = dynamic_value_test_game();
        let card = CardBuilder::new(CardId::new(), "Shadow Counter Recipient")
            .card_types(vec![CardType::Creature])
            .build();
        let object = game.create_object_from_card(&card, PlayerId::from_index(0), zone);
        game.object_mut(object)
            .expect("recipient exists")
            .counters
            .insert(CounterType::Named("shadow".into()), 2);
        let effects = game.try_all_continuous_effects().expect("finite discovery");
        let chars = game
            .calculated_characteristics_with_effects(object, &effects)
            .expect("counter-bearing card exists");
        assert_eq!(
            chars
                .static_abilities
                .iter()
                .filter(|a| a.id() == StaticAbilityId::Shadow)
                .count(),
            2,
            "shadow in {zone:?}"
        );
        let view = crate::derived_view::DerivedGameView::new(&game);
        assert!(view.object_has_static_ability_id(object, StaticAbilityId::Shadow));
        assert_eq!(
            view.abilities_rc(object)
                .expect("derived card exists")
                .iter()
                .filter(|a| matches!(&a.kind, AbilityKind::Static(a)
                if a.id() == StaticAbilityId::Shadow))
                .count(),
            2
        );
    }
}


#[test]
fn native_clone_preserves_expired_duration_and_allocator_history() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::from_raw(99301), "Duration snapshot land")
        .card_types(vec![CardType::Land])
        .build();
    let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
    game.object_mut(object)
        .expect("land exists")
        .counters
        .insert(CounterType::Flood, 1);
    let effect = ContinuousEffect::new(
        object,
        alice,
        EffectTarget::Specific(object),
        Modification::AddSubtypes(vec![Subtype::Island]),
    )
    .until(Until::ForAsLongAs(
        ironsmith_core::ContinuousDurationPredicate::ObjectHasCounter {
            object: ironsmith_core::ContinuousDurationObject::Specific(object),
            counter_type: CounterType::Flood,
            minimum: 1,
        },
    ));
    let manager = &mut game.effect_store.continuous_effects;
    let discarded = manager.add_effect(effect.clone());
    manager.remove_effect(discarded);
    let group = manager.next_effect_group_id();
    let id = manager.add_effect(effect.with_group(group));
    let registered = manager.effects()[0].clone();
    assert!(continuous_effect_duration_is_active(&registered, &game));
    game.object_mut(object)
        .expect("land exists")
        .counters
        .remove(&CounterType::Flood);
    assert!(!continuous_effect_duration_is_active(&registered, &game));
    let snapshot = game.effect_store.continuous_effects.clone();
    assert_eq!(
        *snapshot.latched_duration_states.borrow().get(&id).unwrap(),
        ContinuousDurationLatch::Expired
    );
    game.object_mut(object)
        .expect("land exists")
        .counters
        .insert(CounterType::Flood, 1);

    // The duration's predicate is true again, but restoring must not restart it.
    let restored = snapshot.clone();
    game.effect_store.continuous_effects = restored;
    assert!(!continuous_effect_duration_is_active(&registered, &game));
    let manager = &mut game.effect_store.continuous_effects;
    assert_eq!(
        manager
            .add_effect(ContinuousEffect::new(
                object,
                alice,
                EffectTarget::Specific(object),
                Modification::ChangeController(alice)
            ))
            .0,
        snapshot.next_id
    );
    assert_eq!(manager.next_effect_group_id().0, snapshot.next_group_id + 1);
    assert_eq!(
        manager.effects()[0],
        registered,
        "old identity and payload survive later registration"
    );
    assert_eq!(
        manager.current_timestamp(),
        snapshot.current_timestamp + 1
    );
}


#[cfg(feature = "serialization")]
#[test]
fn complete_modification_schema_round_trip_keeps_payloads_and_control_semantics() {
    type Model = ContinuousModification<String, String, String, String, String, String>;
    let literal = Model::ChangeController(PlayerId::from_index(1));
    let relative = Model::ChangeControllerToEffectController;
    let values = vec![
        literal.clone(),
        relative.clone(),
        Model::CopyOf {
            target_id: ObjectId::from_raw(99311),
            copiable_values: Box::new("copy with retained abilities and occurrence IDs".into()),
            preserve_source_abilities: true,
            name_override: Some("Copied name".into()),
            name_override_surface: None,
            add_supertypes: vec![Supertype::Legendary],
        },
        Model::SetTextBox("text with retained ability instances and labels".into()),
        Model::SetAuraAttachmentFilter("attachment filter with enchant occurrence".into()),
        Model::AddAbility("static occurrence".into()),
        Model::RemoveAbility("removed static occurrence".into()),
        Model::AddAbilityGeneric("activated/triggered occurrence".into()),
        Model::SetAbilities(vec!["first occurrence".into(), "second occurrence".into()]),
        Model::RemoveAbilityGeneric {
            ability: "lost occurrence".into(),
            mode: ironsmith_core::AbilityLossMode::Lose,
        },
        Model::Restriction("restriction with canonical ability occurrence".into()),
        Model::CopyActivatedAbilities {
            filter: ObjectFilter::creature().you_control(),
            counter: Some(CounterType::Flood),
            include_mana: true,
            only_loyalty: true,
            exclude_source_name: true,
            exclude_source_id: true,
            force_once_each_turn: true,
        },
        Model::ModifyPowerToughnessByColorCount {
            power_multiplier: -2,
            toughness_multiplier: 3,
        },
        Model::SetPowerToughness {
            power: Value::Fixed(7),
            toughness: Value::Fixed(9),
            sublayer: PtSublayer::CharacteristicDefining,
        },
    ];
    assert_ne!(
        serde_json::to_value(&literal).unwrap(),
        serde_json::to_value(&relative).unwrap(),
        "literal and effect-relative control are different wire semantics"
    );
    for original in values {
        let mapped = original
            .clone()
            .try_map_payloads(
                |v| Ok::<_, String>(format!("static:{v}")),
                |v| Ok::<_, String>(format!("ability:{v}")),
                |v| Ok::<_, String>(format!("copy:{v}")),
                |v| Ok::<_, String>(format!("text:{v}")),
                |v| Ok::<_, String>(format!("restriction:{v}")),
                |v| Ok::<_, String>(format!("attachment:{v}")),
            )
            .expect("every payload can be encoded");
        let serialized = serde_json::to_string(&mapped).expect("complete model serializes");
        let decoded: Model =
            serde_json::from_str(&serialized).expect("complete model deserializes");
        let restored = decoded
            .try_map_payloads(
                |v| {
                    Ok::<_, String>(
                        v.strip_prefix("static:")
                            .expect("static converter")
                            .to_owned(),
                    )
                },
                |v| {
                    Ok::<_, String>(
                        v.strip_prefix("ability:")
                            .expect("ability converter")
                            .to_owned(),
                    )
                },
                |v| Ok::<_, String>(v.strip_prefix("copy:").expect("copy converter").to_owned()),
                |v| Ok::<_, String>(v.strip_prefix("text:").expect("text converter").to_owned()),
                |v| {
                    Ok::<_, String>(
                        v.strip_prefix("restriction:")
                            .expect("restriction converter")
                            .to_owned(),
                    )
                },
                |v| {
                    Ok::<_, String>(
                        v.strip_prefix("attachment:")
                            .expect("attachment converter")
                            .to_owned(),
                    )
                },
            )
            .expect("payloads can be restored");
        assert_eq!(
            restored, original,
            "conversion and JSON cannot drop scalar or retained payload fields"
        );
    }
}

#[test]
fn complete_modification_schema_propagates_nested_payload_failure() {
    type Model = ContinuousModification<u8, u8, u8, u8, u8, u8>;
    let original = Model::SetAbilities(vec![1, 2, 3]);
    let mut seen = Vec::new();
    let result = original.try_map_payloads(
        Ok::<_, &'static str>,
        |value| {
            seen.push(value);
            if value == 2 {
                Err("unsupported retained ability")
            } else {
                Ok(value)
            }
        },
        Ok::<_, &'static str>,
        Ok::<_, &'static str>,
        Ok::<_, &'static str>,
        Ok::<_, &'static str>,
    );
    assert_eq!(result, Err("unsupported retained ability"));
    assert_eq!(
        seen,
        vec![1, 2],
        "a failure cannot be converted to an incomplete successful ability list"
    );
}


#[cfg(feature = "serialization")]
fn retained_copy_schema_fixture() -> crate::snapshot::RetainedCopiableValues<String> {
    crate::snapshot::RetainedCopiableValues {
        name: "Retained characteristics".into(),
        mana_cost: Some(ManaCost::from_symbols(vec![
            ManaSymbol::Generic(3),
            ManaSymbol::White,
        ])),
        compiled_card_text: "Complete copied text".into(),
        ability_labels: vec!["First label".into(), "Second label".into()],
        power: Some(-2),
        toughness: Some(7),
        card_types: vec![CardType::Creature, CardType::Enchantment],
        subtypes: vec![Subtype::Aura],
        supertypes: vec![Supertype::Legendary],
        colors: crate::ColorSet::WHITE,
        loyalty: Some(5),
        defense: Some(6),
        abilities: vec!["first payload".into(), "second payload".into()],
        aura_attach_filter: Some(crate::object::AuraAttachmentFilter::from(
            ObjectFilter::creature(),
        )),
    }
}

#[cfg(feature = "serialization")]
#[test]
fn retained_copy_text_schema_preserves_complete_metadata_and_requires_abilities() {
    let copy = retained_copy_schema_fixture();
    let json = serde_json::to_value(&copy).unwrap();
    let restored: crate::snapshot::RetainedCopiableValues<String> =
        serde_json::from_value(json.clone()).unwrap();
    assert_eq!(restored, copy);
    let mut missing = json;
    missing.as_object_mut().unwrap().remove("abilities");
    assert!(
        serde_json::from_value::<crate::snapshot::RetainedCopiableValues<String>>(missing).is_err()
    );
    let overlay = RetainedTextBoxOverlay {
        compiled_card_text: copy.compiled_card_text.clone(),
        abilities: copy.abilities.clone(),
        ability_labels: copy.ability_labels.clone(),
    };
    let mut json = serde_json::to_value(&overlay).unwrap();
    assert_eq!(
        serde_json::from_value::<RetainedTextBoxOverlay<String>>(json.clone()).unwrap(),
        overlay
    );
    json.as_object_mut().unwrap().remove("abilities");
    assert!(serde_json::from_value::<RetainedTextBoxOverlay<String>>(json).is_err());
    let ability = Ability::static_ability(StaticAbility::flying());
    let expected_id = match &ability.kind {
        AbilityKind::Static(value) => value.instance_id(),
        _ => unreachable!(),
    };
    let native = CopiableValues {
        name: copy.name,
        mana_cost: copy.mana_cost,
        compiled_card_text: copy.compiled_card_text,
        ability_labels: copy.ability_labels,
        power: copy.power,
        toughness: copy.toughness,
        card_types: copy.card_types,
        subtypes: copy.subtypes,
        supertypes: copy.supertypes,
        colors: copy.colors,
        loyalty: copy.loyalty,
        defense: copy.defense,
        abilities: std::sync::Arc::new(vec![ability]),
        aura_attach_filter: copy.aura_attach_filter,
    };
    let model = crate::snapshot::RetainedCopiableValues::from(native.clone());
    let recovered = CopiableValues::from(model);
    assert_eq!(recovered, native);
    match &recovered.abilities[0].kind {
        AbilityKind::Static(value) => assert_eq!(
            value.instance_id(),
            expected_id,
            "native conversion preserves occurrence aliases"
        ),
        _ => panic!("static payload lost"),
    }
}

#[cfg(feature = "serialization")]
#[test]
fn retained_copy_text_schema_propagates_nested_ability_failure() {
    let copy = retained_copy_schema_fixture();
    let overlay = RetainedTextBoxOverlay {
        compiled_card_text: copy.compiled_card_text.clone(),
        abilities: copy.abilities.clone(),
        ability_labels: copy.ability_labels.clone(),
    };
    let reject_second = |payload: String| -> Result<usize, &'static str> {
        if payload == "first payload" {
            Ok(1)
        } else {
            Err("nested payload cannot encode")
        }
    };
    assert_eq!(
        copy.try_map_abilities(reject_second),
        Err("nested payload cannot encode")
    );
    assert_eq!(
        overlay.try_map_abilities(reject_second),
        Err("nested payload cannot encode")
    );
}

#[cfg(feature = "serialization")]
#[test]
fn retained_copy_text_schema_requires_explicit_optional_characteristics() {
    let original = serde_json::to_value(retained_copy_schema_fixture()).unwrap();
    let fields = [
        "mana_cost",
        "power",
        "toughness",
        "loyalty",
        "defense",
        "aura_attach_filter",
    ];
    let accepted_missing = fields
        .iter()
        .filter(|field| {
            let mut value = original.clone();
            value.as_object_mut().unwrap().remove(**field);
            serde_json::from_value::<crate::snapshot::RetainedCopiableValues<String>>(value).is_ok()
        })
        .copied()
        .collect::<Vec<_>>();
    assert!(
        accepted_missing.is_empty(),
        "missing characteristics silently defaulted: {accepted_missing:?}"
    );
    for field in fields {
        let mut value = original.clone();
        value[field] = serde_json::Value::Null;
        let restored: crate::snapshot::RetainedCopiableValues<String> =
            serde_json::from_value(value.clone())
                .expect("explicit null is a legitimate absent characteristic");
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            value,
            "explicit null preserves the complete payload"
        );
    }
}

#[test]
fn retained_restriction_models_restore_native_rules_and_functional_zones() {
    use crate::static_abilities::StaticAbilityId;
    let cases = [
        (
            RestrictionKind::CantBeBlocked,
            StaticAbilityId::Unblockable,
            [true, true, false],
        ),
        (
            RestrictionKind::CantAttack,
            StaticAbilityId::CantAttack,
            [false, true, true],
        ),
        (
            RestrictionKind::CantBlock,
            StaticAbilityId::CantBlock,
            [true, false, true],
        ),
        (
            RestrictionKind::DoesntUntap,
            StaticAbilityId::DoesntUntap,
            [true, true, true],
        ),
    ];
    let missing = cases
        .iter()
        .filter(|(kind, _, _)| {
            RegisteredRestriction::new(*kind)
                .ability()
                .compiled_model()
                .is_none()
        })
        .map(|(kind, _, _)| *kind)
        .collect::<Vec<_>>();
    assert!(
        missing.is_empty(),
        "native restriction payloads have no canonical model: {missing:?}"
    );
    let alice = PlayerId::from_index(0);
    for (kind, id, expected_rules) in cases {
        let original = RegisteredRestriction::new(kind).ability().clone();
        let model = original
            .compiled_model()
            .expect("complete restriction model");
        let restored = StaticAbility::from_model(model.clone());
        assert_eq!(restored.id(), id);
        assert_eq!(restored.display(), original.display());
        assert_eq!(restored.has_defender(), original.has_defender());
        assert_eq!(restored.is_unblockable(), original.is_unblockable());
        assert_eq!(restored.affects_untap(), original.affects_untap());
        assert_eq!(restored.is_keyword(), original.is_keyword());
        assert_eq!(
            restored.may_generate_continuous_effects(),
            original.may_generate_continuous_effects()
        );
        use ironsmith_core::functional_zones::StaticAbilityFunctionalZones;
        assert_eq!(
            restored.default_functional_zones(),
            original.default_functional_zones()
        );
        for ability in [original, restored] {
            let mut game = dynamic_value_test_game();
            let card = CardBuilder::new(CardId::new(), "Restriction rules fixture")
                .card_types(vec![CardType::Creature])
                .build();
            let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
            ability.apply_restrictions(&mut game, source, alice);
            let tracker = &game.effect_store.cant_effects;
            assert_eq!(
                [
                    tracker.can_attack(source),
                    tracker.can_block(source),
                    tracker.can_be_blocked(source)
                ],
                expected_rules
            );
        }
    }
}

#[test]
fn registered_cant_attack_is_not_bypassed_by_defender_permission() {
    use crate::ability::Ability;
    use crate::static_abilities::StaticAbilityId;
    for (defender, restored) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut game = dynamic_value_test_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Attack restriction recipient")
            .card_types(vec![CardType::Creature])
            .build();
        let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let abilities = std::sync::Arc::make_mut(
            &mut game.object_mut(object).expect("recipient exists").abilities,
        );
        abilities.push(Ability::static_ability(
            StaticAbility::can_attack_as_though_no_defender(),
        ));
        if defender {
            abilities.push(Ability::static_ability(StaticAbility::defender()));
        }
        game.refresh_continuous_state()
            .expect("finite baseline refresh");
        assert!(
            game.effect_store.cant_effects.can_attack(object),
            "permission bypasses only Defender"
        );
        let mut restriction = RegisteredRestriction::new(RestrictionKind::CantAttack);
        if restored {
            restriction.ability = StaticAbility::from_model(
                restriction
                    .ability()
                    .compiled_model()
                    .expect("complete attack restriction model")
                    .clone(),
            );
        }
        let effect = game
            .effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                object,
                alice,
                EffectTarget::Specific(object),
                Modification::Restriction(restriction),
            ));
        game.refresh_continuous_state()
            .expect("finite restricted refresh");
        assert!(
            !game.effect_store.cant_effects.can_attack(object),
            "independent can't attack survives Defender permission (printed defender: {defender})"
        );
        assert_eq!(
            game.current_has_static_ability_id(object, StaticAbilityId::Defender),
            defender,
            "an attack prohibition must not grant the Defender keyword"
        );
        assert!(game.current_has_static_ability_id(object, StaticAbilityId::CantAttack));
        game.effect_store.continuous_effects.remove_effect(effect);
        game.refresh_continuous_state()
            .expect("finite removal refresh");
        assert!(
            game.effect_store.cant_effects.can_attack(object),
            "removing only the independent prohibition restores permission"
        );
        assert_eq!(
            game.current_has_static_ability_id(object, StaticAbilityId::Defender),
            defender
        );
    }
}

#[test]
fn retained_metadata_schema_preserves_occurrences_and_rejects_inconsistent_payloads() {
    use crate::object::{AuraAttachmentMetadata, RetainedAuraAttachmentMetadata};
    let kinds = [
        RestrictionKind::CantBeBlocked,
        RestrictionKind::CantAttack,
        RestrictionKind::CantBlock,
        RestrictionKind::DoesntUntap,
    ];
    for kind in kinds {
        let original = RegisteredRestriction::new(kind);
        let retained = RetainedRestriction::from(original.clone());
        assert_eq!(
            retained.ability.instance_id(),
            original.ability().instance_id()
        );
        let restored =
            RegisteredRestriction::try_from(retained).expect("consistent native payload");
        assert_eq!(
            restored.ability().instance_id(),
            original.ability().instance_id()
        );
        assert_eq!(restored.kind(), kind);
        for other in kinds {
            let value = RetainedRestriction {
                kind,
                ability: RegisteredRestriction::new(other).ability().clone(),
            };
            assert_eq!(
                RegisteredRestriction::try_from(value).is_ok(),
                kind == other
            );
        }
    }
    assert!(
        RegisteredRestriction::try_from(RetainedRestriction {
            kind: RestrictionKind::CantAttack,
            ability: StaticAbility::defender()
        })
        .is_err(),
        "Defender cannot stand in for an independent prohibition"
    );
    let creature = crate::object::AuraAttachmentFilter::from(ObjectFilter::creature());
    let land = crate::object::AuraAttachmentFilter::from(ObjectFilter::land());
    let original = AuraAttachmentMetadata::from(creature.clone());
    let retained = RetainedAuraAttachmentMetadata::from(original);
    let occurrence = retained.enchant_ability.instance_id();
    let restored = AuraAttachmentMetadata::try_from(retained).expect("matching enchant filter");
    let retained = RetainedAuraAttachmentMetadata::from(restored);
    assert_eq!(retained.filter, creature);
    assert_eq!(retained.enchant_ability.instance_id(), occurrence);
    for ability in [StaticAbility::enchant(land), StaticAbility::flying()] {
        assert!(
            AuraAttachmentMetadata::try_from(RetainedAuraAttachmentMetadata {
                filter: creature.clone(),
                enchant_ability: ability
            })
            .is_err(),
            "wrong filter and missing enchant reject"
        );
    }
    let error = RetainedRestriction {
        kind: RestrictionKind::CantAttack,
        ability: 1,
    }
    .try_map_ability(|_| Err::<String, _>("restriction failure"));
    assert_eq!(error.unwrap_err(), "restriction failure");
    let error = RetainedAuraAttachmentMetadata {
        filter: creature,
        enchant_ability: 1,
    }
    .try_map_ability(|_| Err::<String, _>("attachment failure"));
    assert_eq!(error.unwrap_err(), "attachment failure");
}

#[cfg(feature = "serialization")]
#[test]
fn retained_metadata_schema_json_requires_every_payload_field() {
    use crate::object::RetainedAuraAttachmentMetadata;
    let restriction = RetainedRestriction {
        kind: RestrictionKind::CantAttack,
        ability: "occurrence".to_owned(),
    };
    let attachment = RetainedAuraAttachmentMetadata {
        filter: crate::object::AuraAttachmentFilter::from(ObjectFilter::creature()),
        enchant_ability: "occurrence".to_owned(),
    };
    let r = serde_json::to_value(restriction).unwrap();
    let a = serde_json::to_value(attachment).unwrap();
    let round_r: RetainedRestriction<String> = serde_json::from_value(r.clone()).unwrap();
    let round_a: RetainedAuraAttachmentMetadata<String> =
        serde_json::from_value(a.clone()).unwrap();
    assert_eq!(serde_json::to_value(round_r).unwrap(), r);
    assert_eq!(serde_json::to_value(round_a).unwrap(), a);
    for field in ["kind", "ability"] {
        let mut missing = r.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<RetainedRestriction<String>>(missing).is_err());
        let mut null = r.clone();
        null[field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<RetainedRestriction<String>>(null).is_err());
    }
    for field in ["filter", "enchant_ability"] {
        let mut missing = a.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<RetainedAuraAttachmentMetadata<String>>(missing).is_err());
        let mut null = a.clone();
        null[field] = serde_json::Value::Null;
        assert!(serde_json::from_value::<RetainedAuraAttachmentMetadata<String>>(null).is_err());
    }
}


#[test]
fn retained_temporary_registration_expiry_matches_all_layer_routes_and_preserves_origin() {
    use crate::object::TemporaryStaticAbilityGrant;
    use crate::static_abilities::StaticAbilityId;
    let mut game = dynamic_value_test_game();
    let alice = game.players[0].id;
    let card = CardBuilder::new(CardId::new(), "Retained grant recipient")
        .card_types(vec![CardType::Creature]).build();
    let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let other = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let store = &mut game.object_mut(source).unwrap().temporary_static_ability_grants;
    for (ability, expires_end_of_turn) in [(StaticAbilityId::Deathtouch, 2), (StaticAbilityId::Haste, 4)] {
        store.push(TemporaryStaticAbilityGrant { ability, ability_payload: None, expires_end_of_turn: Some(expires_end_of_turn) });
    }
    let expected_origin = store.origin(1).unwrap().clone();
    game.refresh_continuous_state().unwrap();
    assert!(game.current_has_static_ability_id(source, StaticAbilityId::Deathtouch));
    game.next_turn();
    game.next_turn();
    assert_eq!(game.turn.turn_number, 3);
    // A checkpoint can retain expired registrations. All query routes must
    // respect lifetime without requiring cleanup to prune their stored payloads.
    assert_eq!(game.object(source).unwrap().temporary_static_ability_grants.len(), 2);
    game.refresh_continuous_state().unwrap();
    let manager = game.effect_store.continuous_effects.calculate_characteristics(
        source, game.objects_map(), &game.battlefield, &game).unwrap();
    let direct = calculate_characteristics_with_effects(source, game.objects_map(),
        &[], &game.battlefield, game.commander_objects(), &game).unwrap();
    let batch = calculate_characteristics_batch_with_effects(&[source, other],
        game.objects_map(), &[], &game.battlefield, game.commander_objects(), &game);
    for chars in [&manager, &direct, batch.get(&source).unwrap()] {
        assert!(!chars.static_abilities.iter().any(|ability| ability.id() == StaticAbilityId::Deathtouch));
        assert!(chars.static_abilities.iter().any(|ability| ability.id() == StaticAbilityId::Haste));
        assert_eq!(chars.abilities.len(), 1);
        assert_eq!(chars.abilities.origin(0), Some(&AbilityOrigin::Temporary(expected_origin.clone())));
    }
    game.prewarm_calculated_characteristics(&[source, other]);
    assert!(!game.current_has_static_ability_id(source, StaticAbilityId::Deathtouch));
    assert!(game.current_has_static_ability_id(source, StaticAbilityId::Haste));
    assert_eq!(game.object(source).unwrap().temporary_static_ability_grants.len(), 2);
}

#[test]
fn ability_copying_respects_donor_zone_and_linked_identity() {
    for (triggered, linked) in [(false, false), (true, false), (false, true), (true, true)] {
        let mut game = dynamic_value_test_game();
        let payer = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Ability donor")
            .card_types(vec![CardType::Creature]).build();
        let mut linked_donor = None;
        for zone in [Zone::Battlefield, Zone::Hand, Zone::Graveyard, Zone::Exile, Zone::Exile] {
            let id = game.create_object_from_card(&card, payer, zone);
            if zone == Zone::Exile { linked_donor.get_or_insert(id); }
            let ability = if triggered {
                crate::ability::Ability::triggered(
                    crate::triggers::Trigger::player_taps_for_mana(PlayerFilter::You, ObjectFilter::land()),
                    vec![crate::effect::Effect::add_mana(vec![ManaSymbol::Green])],
                )
            } else {
                crate::ability::Ability::activated(crate::cost::TotalCost::free(),
                    vec![crate::effect::Effect::gain_life(1)])
            };
            game.object_mut(id).unwrap().abilities_mut().push(ability);
        }
        let receiver_card = CardBuilder::new(CardId::new(), "Ability receiver")
            .card_types(vec![CardType::Artifact]).build();
        let receiver = game.create_object_from_card(&receiver_card, payer, Zone::Battlefield);
        let filter = if linked {
            game.add_exiled_with_source_link(receiver, linked_donor.unwrap());
            ObjectFilter::creature().in_zone(Zone::Exile).match_tagged(
                crate::tag::SOURCE_EXILED_TAG, crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            )
        } else { ObjectFilter::creature().in_zone(Zone::Exile) };
        let modification = if triggered {
            Modification::CopyTriggeredAbilities { filter, exclude_source_name: false, exclude_source_id: true }
        } else {
            Modification::CopyActivatedAbilities { filter, counter: None, include_mana: true,
                only_loyalty: false, exclude_source_name: false, exclude_source_id: true,
                force_once_each_turn: false }
        };
        game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
            receiver, payer, EffectTarget::Specific(receiver), modification,
        ));
        game.refresh_continuous_state().unwrap();
        let chars = game.calculated_characteristics(receiver).unwrap();
        assert_eq!(chars.abilities.len(), if linked { 1 } else { 2 }, "only matching donors contribute");
    }
}


#[test]
fn intrinsic_land_mana_obeys_ability_layer_operations_on_every_calculation_route() {
    for land_type_change in [false, true] {
        for operation in 0..4 {
            let mut game = dynamic_value_test_game();
            let alice = PlayerId::from_index(0);
            let card = CardBuilder::new(CardId::new(), "Intrinsic mana layer recipient")
                .card_types(vec![CardType::Land, CardType::Creature])
                .subtypes(vec![Subtype::Forest])
                .power_toughness(PowerToughness::fixed(1, 1)).build();
            let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
            let origin = StaticAbility::set_land_subtypes(ObjectFilter::land(), vec![Subtype::Island]);
            let watcher = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Land type layer source")
                .card_types(vec![CardType::Enchantment]);
            let watcher = if land_type_change {
                watcher.with_ability(Ability::static_ability(origin.clone()))
            } else { watcher }.build();
            let watcher = game.create_object_from_definition(&watcher, alice, Zone::Battlefield);
            if land_type_change {
                let origin = game.object(watcher).unwrap().abilities.iter().find_map(|ability| {
                    match &ability.kind {
                        AbilityKind::Static(ability) if ability.id() == crate::static_abilities::StaticAbilityId::SetLandSubtypes => Some(ability.clone()),
                        _ => None,
                    }
                }).expect("actual source owns its materialized land-type static instance");
                for modification in [Modification::SetSubtypes(vec![Subtype::Island]), Modification::RemoveAllAbilities] {
                    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                        watcher, alice, EffectTarget::Specific(object), modification)
                        .with_originating_static_ability(origin.clone()));
                }
            }
            let modification = match operation {
                0 => Modification::RemoveAllAbilities,
                1 => Modification::RemoveAllAbilitiesExceptMana,
                2 => Modification::SetAbilities(Vec::new()),
                _ => Modification::SetAbilities(vec![Ability::static_ability(StaticAbility::flying())]),
            };
            game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                watcher, alice, EffectTarget::Specific(object), modification));
            let granted = Ability::basic_land_mana(Subtype::Swamp).unwrap();
            game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
                watcher, alice, EffectTarget::Specific(object), Modification::AddAbilityGeneric(granted.clone())));
            let effects = game.effect_store.continuous_effects.effects().to_vec();
            let manager = game.effect_store.continuous_effects.calculate_characteristics(
                object, game.objects_map(), &game.battlefield, &game).unwrap();
            let direct = calculate_characteristics_with_effects(object, game.objects_map(),
                &effects, &game.battlefield, game.commander_objects(), &game).unwrap();
            let batch = calculate_characteristics_batch_with_effects(&[object, watcher],
                game.objects_map(), &effects, &game.battlefield, game.commander_objects(), &game);
            for (route, chars) in [("manager", &manager), ("direct", &direct), ("batch", batch.get(&object).unwrap())] {
                let intrinsic = Ability::basic_land_mana(if land_type_change { Subtype::Island } else { Subtype::Forest }).unwrap();
                assert_eq!(chars.abilities.contains(&intrinsic), operation == 1,
                    "intrinsic mana participates in the operation: {route}, operation={operation}, land_type_change={land_type_change}; subtypes={:?}; abilities={:?}", chars.subtypes, chars.abilities.as_slice());
                assert!(chars.abilities.contains(&granted), "later separate mana grant survives: {route}");
                assert_eq!(chars.abilities.contains(&Ability::static_ability(StaticAbility::flying())), operation == 3);
                if land_type_change {
                    assert!(chars.subtypes.contains(&Subtype::Island), "land-type operation actually applied: {route}");
                    assert!(!chars.subtypes.contains(&Subtype::Forest), "old land type is gone: {route}");
                    assert!(!chars.abilities.contains(&Ability::basic_land_mana(Subtype::Forest).unwrap()), "old land-type mana removed");
                }
            }
        }
    }
}


#[test]
fn intrinsic_land_mana_and_equal_continuous_grant_remain_independent() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = CardBuilder::new(CardId::new(), "Intrinsic land and independent equal grant")
        .card_types(vec![CardType::Land]).subtypes(vec![Subtype::Forest]).build();
    let object = game.create_object_from_card(&card, alice, Zone::Battlefield);
    let intrinsic = Ability::basic_land_mana(Subtype::Forest).unwrap();
    game.effect_store.continuous_effects.add_effect(ContinuousEffect::new(
        object, alice, EffectTarget::Specific(object), Modification::AddAbilityGeneric(intrinsic.clone())));
    let effects = game.effect_store.continuous_effects.effects().to_vec();
    let chars = calculate_characteristics_with_effects(object, game.objects_map(),
        &effects, &game.battlefield, game.commander_objects(), &game).unwrap();
    let indices: Vec<_> = chars.abilities.iter().enumerate()
        .filter_map(|(index, ability)| (ability == &intrinsic).then_some(index)).collect();
    assert_eq!(indices.len(), 2, "inherent rule and separate equal grant are independent occurrences");
    assert_eq!(chars.abilities.origin(indices[0]), Some(&AbilityOrigin::IntrinsicBasicLandMana(Subtype::Forest)));
    assert!(matches!(chars.abilities.origin(indices[1]), Some(AbilityOrigin::Effect { .. })));
    game.refresh_continuous_state().unwrap();
    assert_eq!(game.current_abilities(object).unwrap().iter().filter(|ability| *ability == &intrinsic).count(), 2);
}

#[test]
fn intrinsic_land_mana_fast_path_preserves_level_grant_dispatch_indices() {
    let mut game = dynamic_value_test_game();
    let alice = PlayerId::from_index(0);
    let card = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Intrinsic mana with level grant")
        .card_types(vec![CardType::Land, CardType::Creature]).subtypes(vec![Subtype::Forest])
        .with_ability(Ability::static_ability(StaticAbility::with_level_abilities(vec![
            crate::ability::LevelAbility::new(1, None).with_ability(StaticAbility::haste())
        ]))).build();
    let object = game.create_object_from_definition(&card, alice, Zone::Battlefield);
    game.add_counters(object, CounterType::Level, 1).unwrap();
    game.refresh_continuous_state().unwrap();
    let effects = game.try_all_continuous_effects().unwrap();
    assert!(effects.is_empty(), "level keyword grant exercises the path without continuous instructions");
    let view = crate::derived_view::DerivedGameView::new(&game);
    assert!(!view.requires_battlefield_characteristic_calculation(object), "exercise the fast path");
    let advertised = view.abilities_rc(object).unwrap();
    let chars = calculate_characteristics_with_effects(object, game.objects_map(),
        &effects, &game.battlefield, game.commander_objects(), &game).unwrap();
    assert_eq!(advertised.as_slice(), chars.abilities.as_slice(), "advertised and layer dispatch index spaces agree");
    let sparse = unmodified_ability_occurrences(game.object(object).unwrap(), game.turn.turn_number);
    assert_eq!(sparse.as_slice(), chars.abilities.as_slice(), "sparse occurrence scan preserves the same ability sequence");
    for (index, ability) in advertised.iter().enumerate() {
        assert_eq!(game.current_ability(object, index).as_ref(), Some(ability), "dispatch index {index}");
    }
}

#[test]
fn base_pt_boundary_agrees_across_all_evaluators_and_departure_snapshots() {
    use crate::effect::{Effect, Until};
    use crate::effects::{EffectContext, execute_effect};
    use crate::filter::{Comparison, FilterContext, ObjectFilterExt};
    use crate::target::ChooseSpec;
    let alice = PlayerId::from_index(0);
    for setting in [false, true] {
        let mut game = dynamic_value_test_game();
        let card = CardBuilder::new(CardId::new(), "Layer boundary witness")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(0, 0))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let other = game.create_object_from_card(&card, alice, Zone::Battlefield);
        add_dynamic_base_pt(
            &mut game,
            source,
            alice,
            Value::LifeTotal(PlayerFilter::You),
            Value::Fixed(8),
        );
        fn apply(game: &mut GameState, source: ObjectId, effect: Effect) {
            execute_effect(
                game,
                &effect,
                &mut EffectContext::new_default(source, PlayerId::from_index(0)),
            )
            .unwrap();
        }
        if setting {
            apply(
                &mut game,
                source,
                Effect::set_base_power_toughness(
                    3,
                    7,
                    ChooseSpec::SpecificObject(source),
                    Until::EndOfTurn,
                ),
            );
        }
        apply(
            &mut game,
            source,
            Effect::pump(2, 0, ChooseSpec::SpecificObject(source), Until::EndOfTurn),
        );
        apply(
            &mut game,
            source,
            Effect::put_counters(
                CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::SpecificObject(source),
            ),
        );
        // Warm the cache, then change a CDA dependency through a real action.
        assert_eq!(
            game.calculated_characteristics(source).unwrap().base_power,
            Some(if setting { 3 } else { 20 })
        );
        apply(&mut game, source, Effect::gain_life(3));
        let expected_base = if setting { (3, 7) } else { (23, 8) };
        let switched = ContinuousEffect::new(
            source,
            alice,
            EffectTarget::Specific(source),
            Modification::SwitchPowerToughness,
        );
        game.effect_store.continuous_effects.add_effect(switched);
        let effects = game.all_continuous_effects();
        let direct = game
            .calculated_characteristics_with_effects(source, &effects)
            .unwrap();
        let batch = game.calculated_characteristics_batch_with_effects(&[source, other], &effects);
        let mut manager = ContinuousEffectManager::new();
        for effect in effects {
            manager.add_effect(effect);
        }
        let legacy = manager
            .calculate_characteristics(source, game.objects_map(), &game.battlefield, &game)
            .unwrap();
        for chars in [&direct, &batch[&source], &legacy] {
            assert_eq!(
                (chars.base_power, chars.base_toughness),
                (Some(expected_base.0), Some(expected_base.1))
            );
            assert_eq!(
                (chars.power, chars.toughness),
                (Some(expected_base.1 + 1), Some(expected_base.0 + 3))
            );
        }
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).unwrap(),
            &game,
        );
        assert_eq!(
            (snapshot.base_power, snapshot.base_toughness),
            (Some(expected_base.0), Some(expected_base.1))
        );
        let filter = ObjectFilter::creature().with_base_power(Comparison::Equal(expected_base.0));
        let ctx = FilterContext::new(alice);
        assert!(filter.matches(game.object(source).unwrap(), &ctx, &game));
        assert!(filter.matches_snapshot(&snapshot, &ctx, &game));
        let mut larger = ObjectFilter::creature();
        larger.power_greater_than_base_power = true;
        assert_eq!(
            larger.matches_snapshot(&snapshot, &ctx, &game),
            expected_base.1 + 1 > expected_base.0
        );
        game.move_object_by_game_rule(source, Zone::Graveyard)
            .unwrap();
        let departure =
            crate::effects::helpers::latest_zone_change_snapshot_for_object(&game, source).unwrap();
        assert_eq!(
            (departure.base_power, departure.base_toughness),
            (Some(expected_base.0), Some(expected_base.1))
        );
    }
}

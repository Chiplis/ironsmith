use super::*;
// Tests use the new StaticAbility type (already imported as StaticAbility in the module)

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
        compiled_card_text: creature.compiled_card_text.clone(),
        power: creature.base_power.as_ref().map(|p| p.base_value()),
        toughness: creature.base_toughness.as_ref().map(|t| t.base_value()),
        card_types: creature.card_types.clone(),
        subtypes: creature.subtypes.clone(),
        supertypes: creature.supertypes.clone(),
        colors: creature.colors(),
        abilities: creature.abilities.clone().into(),
        static_abilities: extract_static_abilities(&creature.abilities).into(),
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
        compiled_card_text: creature.compiled_card_text.clone(),
        power: None,
        toughness: None,
        card_types: creature.card_types.clone(),
        subtypes: Vec::new().into(),
        supertypes: Vec::new().into(),
        colors: ColorSet::COLORLESS,
        abilities: Vec::new().into(),
        static_abilities: Vec::new().into(),
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
        compiled_card_text: creature.compiled_card_text.clone(),
        power: None,
        toughness: None,
        card_types: creature.card_types.clone(),
        subtypes: Vec::new().into(),
        supertypes: Vec::new().into(),
        colors: ColorSet::COLORLESS,
        abilities: Vec::new().into(),
        static_abilities: vec![StaticAbility::flying()].into(), // Already has flying
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

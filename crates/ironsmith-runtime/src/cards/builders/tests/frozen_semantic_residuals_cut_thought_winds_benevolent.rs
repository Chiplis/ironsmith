#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

fn simple_definition(name: &str, card_types: Vec<CardType>) -> CardDefinition {
    let mut builder =
        CardDefinitionBuilder::new(CardId::new(), name).card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(2, 2));
    }
    builder.build()
}

/// Peel only transparent provenance wrappers. These wrappers preserve the
/// exact result ID/tag used by execution, so structural tests should inspect
/// through them rather than requiring the semantic leaf at segment top level.
fn transparent_provenance_leaf(effect: &crate::effect::Effect) -> &crate::effect::Effect {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return transparent_provenance_leaf(&with_id.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return transparent_provenance_leaf(&tagged.effect);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return transparent_provenance_leaf(&tagged.effect);
    }
    effect
}

#[test]
fn cut_short_keeps_convoke_first_and_activated_planeswalker_legality() {
    let definition = parse_oracle_card_definition("Cut Short");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Convoke\nDestroy target planeswalker that was activated this turn or tapped creature."
    );
    let destroy = definition
        .spell_effect
        .as_ref()
        .expect("Cut Short should be a spell")
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| {
            transparent_provenance_leaf(effect).downcast_ref::<crate::effects::DestroyEffect>()
        })
        .expect("Cut Short should lower to DestroyEffect");
    let ChooseSpec::Object(filter) = destroy.spec.base() else {
        panic!("Cut Short should target one object: {destroy:#?}");
    };
    assert!(filter.any_of.iter().any(|branch| {
        branch.card_types == [CardType::Planeswalker] && branch.ability_activated_this_turn
    }));
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| { branch.card_types == [CardType::Creature] && branch.tapped })
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let planeswalker = game.create_object_from_definition(
        &simple_definition("Test Walker", vec![CardType::Planeswalker]),
        alice,
        Zone::Battlefield,
    );
    let creature = game.create_object_from_definition(
        &simple_definition("Test Creature", vec![CardType::Creature]),
        alice,
        Zone::Battlefield,
    );
    let ctx = crate::filter::FilterContext::new(alice);
    assert!(!filter.matches(game.object(planeswalker).unwrap(), &ctx, &game));
    game.record_ability_activation(planeswalker, 0);
    assert!(filter.matches(game.object(planeswalker).unwrap(), &ctx, &game));
    assert!(!filter.matches(game.object(creature).unwrap(), &ctx, &game));
    game.tap(creature);
    assert!(filter.matches(game.object(creature).unwrap(), &ctx, &game));
}

#[test]
fn thought_distortion_keeps_shared_noncreature_nonland_hand_and_graveyard_filter() {
    let definition = parse_oracle_card_definition("Thought Distortion");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "This spell can't be countered.\nTarget opponent reveals their hand. Exile all noncreature, nonland cards from that player's hand and graveyard."
    );
    let exile = definition
        .spell_effect
        .as_ref()
        .expect("Thought Distortion should be a spell")
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| {
            transparent_provenance_leaf(effect).downcast_ref::<crate::effects::ExileEffect>()
        })
        .expect("Thought Distortion should retain typed exile semantics");
    let ChooseSpec::All(filter) = &exile.spec else {
        panic!("Thought Distortion should exile exhaustively: {exile:#?}");
    };
    assert_eq!(
        filter.excluded_card_types,
        [CardType::Creature, CardType::Land]
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.zone == Some(Zone::Hand))
    );
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.zone == Some(Zone::Graveyard))
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let hand_spell = game.create_object_from_definition(
        &simple_definition("Hand Spell", vec![CardType::Sorcery]),
        bob,
        Zone::Hand,
    );
    let grave_spell = game.create_object_from_definition(
        &simple_definition("Grave Spell", vec![CardType::Instant]),
        bob,
        Zone::Graveyard,
    );
    let grave_creature = game.create_object_from_definition(
        &simple_definition("Grave Creature", vec![CardType::Creature]),
        bob,
        Zone::Graveyard,
    );
    let ctx = crate::filter::FilterContext::new(alice)
        .with_opponents(vec![bob])
        .with_target_players(vec![bob]);
    assert!(filter.matches(game.object(hand_spell).unwrap(), &ctx, &game));
    assert!(filter.matches(game.object(grave_spell).unwrap(), &ctx, &game));
    assert!(!filter.matches(game.object(grave_creature).unwrap(), &ctx, &game));
}

#[test]
fn winds_of_rath_excludes_exactly_creatures_with_an_aura_attached() {
    let definition = parse_oracle_card_definition("Winds of Rath");
    assert_eq!(
        canonical_compiled_lines(&definition),
        ["Destroy all creatures that aren't enchanted. They can't be regenerated."]
    );
    let destroy = definition
        .spell_effect
        .as_ref()
        .expect("Winds of Rath should be a spell")
        .flattened_default_effects()
        .into_iter()
        .find_map(|effect| {
            transparent_provenance_leaf(effect)
                .downcast_ref::<crate::effects::DestroyNoRegenerationEffect>()
        })
        .expect("Winds should retain no-regeneration destruction");
    let ChooseSpec::All(filter) = &destroy.spec else {
        panic!("Winds should destroy an exhaustive set: {destroy:#?}");
    };
    let forbidden = filter
        .without_attached_object
        .as_deref()
        .expect("nonenchanted selector should retain an Aura exclusion");
    assert_eq!(forbidden.card_types, [CardType::Enchantment]);
    assert_eq!(forbidden.subtypes, [Subtype::Aura]);

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let plain = game.create_object_from_definition(
        &simple_definition("Plain Creature", vec![CardType::Creature]),
        alice,
        Zone::Battlefield,
    );
    let enchanted = game.create_object_from_definition(
        &simple_definition("Enchanted Creature", vec![CardType::Creature]),
        alice,
        Zone::Battlefield,
    );
    let aura_definition = CardDefinitionBuilder::new(CardId::new(), "Test Aura")
        .card_types(vec![CardType::Enchantment])
        .subtypes(vec![Subtype::Aura])
        .build();
    let aura = game.create_object_from_definition(&aura_definition, alice, Zone::Battlefield);
    assert!(
        game.attach_object_to_target(aura, crate::object::AttachmentTarget::Object(enchanted),)
    );
    let ctx = crate::filter::FilterContext::new(alice);
    assert!(filter.matches(game.object(plain).unwrap(), &ctx, &game));
    assert!(!filter.matches(game.object(enchanted).unwrap(), &ctx, &game));
}

#[test]
fn benevolent_blessing_preserves_only_controlled_existing_aura_equipment_attachments() {
    let definition = parse_oracle_card_definition("Benevolent Blessing");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        "Flash\nEnchant creature\nAs this Aura enters, choose a color.\nEnchanted creature has protection from the chosen color. This effect doesn't remove Auras and Equipment you control that are already attached to it."
    );
    let exception_is_typed = definition.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        matches!(
            static_ability.compiled_model().map(|model| &model.payload),
            Some(ironsmith_core::StaticAbilityPayload::AttachedAbilityGrant(grant))
                if grant.protection_does_not_remove_controlled_attachments
        )
    });
    assert!(
        exception_is_typed,
        "protection exception must not be text-only"
    );

    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let host = game.create_object_from_definition(
        &simple_definition("Protected Creature", vec![CardType::Creature]),
        alice,
        Zone::Battlefield,
    );
    let white_equipment = |name: &str| {
        CardDefinitionBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Artifact])
            .subtypes(vec![Subtype::Equipment])
            .build()
    };
    let own_equipment = game.create_object_from_definition(
        &white_equipment("Own Equipment"),
        alice,
        Zone::Battlefield,
    );
    let opposing_equipment = game.create_object_from_definition(
        &white_equipment("Opposing Equipment"),
        bob,
        Zone::Battlefield,
    );
    assert!(
        game.attach_object_to_target(own_equipment, crate::object::AttachmentTarget::Object(host),)
    );
    assert!(game.attach_object_to_target(
        opposing_equipment,
        crate::object::AttachmentTarget::Object(host),
    ));
    let blessing = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    assert!(game.attach_object_to_target(blessing, crate::object::AttachmentTarget::Object(host),));
    game.set_chosen_color(blessing, Color::White);

    let actions = crate::rules::check_state_based_actions(&game);
    assert!(
        !actions
            .contains(&crate::rules::StateBasedAction::AttachmentBecomesUnattached(own_equipment,))
    );
    assert!(!actions.contains(&crate::rules::StateBasedAction::AuraFallsOff(blessing)));
    assert!(actions.contains(
        &crate::rules::StateBasedAction::AttachmentBecomesUnattached(opposing_equipment,)
    ));
}

use super::*;

#[test]
fn fully_described_creatures_keep_authored_fields_instead_of_compact_templates() {
    for (description, subtypes) in [
        ("a legendary 1/1 green Squirrel creature token named Blue", vec![Subtype::Squirrel]),
        ("a 3/3 green Elephant creature token", vec![Subtype::Elephant]),
        ("a 0/4 colorless Wall artifact creature token", vec![Subtype::Wall]),
        ("a 2/2 white Astartes Warrior creature token with vigilance", vec![Subtype::Astartes, Subtype::Warrior]),
        ("a 0/1 colorless Eldrazi Spawn creature token", vec![Subtype::Eldrazi, Subtype::Spawn]),
        ("a 1/1 colorless Eldrazi Scion creature token", vec![Subtype::Eldrazi, Subtype::Scion]),
    ] {
        let TokenDefinitionSpec::Creature(creature) = parse_token_definition_shape_text(description).unwrap() else {
            panic!("a described creature must retain its authored fields: {description}");
        };
        assert_eq!(creature.subtypes, subtypes);
        let roles = creature.text_roles.unwrap();
        assert_eq!(roles.subtypes, ironsmith_core::TokenWordRole::Authored);
        if creature.subtypes == [Subtype::Squirrel] {
            assert_eq!(creature.name, "Blue");
            assert_eq!(creature.colors, ColorSet::GREEN);
            assert!(creature.legendary);
            assert_eq!(roles.name, ironsmith_core::TokenNameTextRole::Explicit);
        }
        if creature.subtypes == [Subtype::Wall] {
            assert!(!creature.keywords.contains(&TokenKeywordShape::Defender));
        }
    }
}

#[test]
fn predefined_token_modifiers_preserve_literal_fields_beside_the_typed_template() {
    let TokenDefinitionSpec::ModifiedBuiltin(shape) = parse_token_definition_shape_text(
        "a legendary blue Heartwood token named Red with protection from red").unwrap() else {
        panic!("explicit modifications to a predefined token");
    };
    assert_eq!(shape.template, BuiltinTokenShape::Heartwood);
    assert_eq!(shape.name.as_deref(), Some("Red"));
    assert_eq!(shape.colors, Some(ColorSet::BLUE));
    assert_eq!(shape.supertypes, vec![ironsmith_core::Supertype::Legendary]);
    assert!(shape.additional_subtypes.is_empty());
    assert_eq!(shape.keywords, vec![TokenKeywordShape::ProtectionFromColors(ColorSet::RED)]);
    let roles = shape.text_roles();
    assert_eq!(roles.name, ironsmith_core::TokenNameTextRole::Explicit);
    assert_eq!(roles.colors, ironsmith_core::TokenWordRole::Authored);
    assert_eq!(roles.subtypes, ironsmith_core::TokenWordRole::RulesImplied);
    assert_eq!(roles.abilities, ironsmith_core::TokenWordRole::RulesImplied);
    assert_eq!(shape.keyword_words, ironsmith_core::TokenWordRole::Authored);

    assert_eq!(parse_token_definition_shape_text("a Heartwood token"),
        Some(TokenDefinitionSpec::Builtin(BuiltinTokenShape::Heartwood)));
    let TokenDefinitionSpec::ModifiedBuiltin(shape) = parse_token_definition_shape_text(
        "a Forest land Food token").unwrap() else { panic!("added subtype facts"); };
    assert_eq!(shape.additional_card_types, vec![CardType::Land]);
    assert_eq!(shape.additional_subtypes, vec![Subtype::Forest]);
    assert_eq!(shape.text_roles().subtypes, ironsmith_core::TokenWordRole::Unrecorded);
}

#[test]
fn token_shape_preserves_vehicle_crew_and_named_creature_facts() {
    let vehicle = parse_token_definition_shape_text(
        "3/3 colorless artifact Vehicle token named Airship with flying and crew 2",
    )
    .unwrap();
    assert!(matches!(
        vehicle,
        TokenDefinitionSpec::Vehicle(VehicleTokenShape {
            name,
            power_toughness: Some((3, 3)),
            colorless: true,
            flying: true,
            crew_amount: Some(2),
            ..
        }) if name == "Airship"
    ));

    let creature = parse_token_definition_shape_text(
        "0/0 colorless Construct artifact creature token named Twin that's attacking.",
    )
    .unwrap();
    assert!(matches!(
        creature,
        TokenDefinitionSpec::Creature(CreatureTokenShape { name, .. }) if name == "Twin"
    ));
}

#[test]
fn token_shape_preserves_source_chosen_color_and_creature_type_references() {
    for text in [
        "2/2 creature token of the chosen color and type",
        "2/2 creature token of that color and type",
    ] {
        let shape = parse_token_definition_shape_text(text).unwrap();
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token for {text}");
        };
        assert!(creature.use_source_chosen_color, "{text}: {creature:#?}");
        assert!(
            creature.use_source_chosen_creature_type,
            "{text}: {creature:#?}"
        );
    }
}

#[test]
fn token_shape_preserves_multitype_creature_metadata() {
    let shape = parse_token_definition_shape_text(
        "2/2 black Zombie Employee artifact creature token with flying",
    )
    .unwrap();
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(
        creature.card_types,
        vec![CardType::Artifact, CardType::Creature]
    );
    assert_eq!(creature.subtypes, vec![Subtype::Zombie, Subtype::Employee]);
    assert_eq!(creature.name, "Zombie Employee");
    assert_eq!(creature.colors, ColorSet::BLACK);
    assert_eq!(creature.keywords, vec![TokenKeywordShape::Flying]);
}

#[test]
fn token_shape_preserves_land_creature_card_types() {
    let shape = parse_token_definition_shape_text("1/1 green Forest Dryad land creature token")
        .expect("land creature token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(
        creature.card_types,
        vec![CardType::Land, CardType::Creature]
    );
    assert_eq!(creature.subtypes, vec![Subtype::Forest, Subtype::Dryad]);

    let ordinary = parse_token_definition_shape_text("1/1 green Forest Dryad creature token")
        .expect("ordinary creature token should parse");
    let TokenDefinitionSpec::Creature(ordinary) = ordinary else {
        panic!("expected creature token shape");
    };
    assert_eq!(ordinary.card_types, vec![CardType::Creature]);
}

#[test]
fn token_shape_preserves_all_colors_surfaces() {
    let all_colors = ColorSet::WHITE
        .union(ColorSet::BLUE)
        .union(ColorSet::BLACK)
        .union(ColorSet::RED)
        .union(ColorSet::GREEN);
    let shape = parse_token_definition_shape_text("2/2 all colors Elemental creature token")
        .expect("all-colors token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(creature.colors, all_colors);

    let suffix = lex_line("that's all colors", 0).expect("postnominal color suffix");
    assert_eq!(
        parse_postnominal_token_colors_tokens(&suffix),
        Some(all_colors)
    );
}

#[test]
fn creature_token_shape_preserves_generic_ward_cost() {
    let shape = parse_token_definition_shape_text("1/1 white Human creature token with ward {2}")
        .expect("ward token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(creature.keywords, vec![TokenKeywordShape::WardGeneric(2)]);
}

#[test]
fn creature_token_shape_preserves_dalek_subtype() {
    let shape =
        parse_token_definition_shape_text("3/3 black Dalek artifact creature token with menace")
            .expect("Dalek token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };

    assert_eq!(creature.subtypes, vec![Subtype::Dalek]);
    assert_eq!(creature.name, "Dalek");
    assert_eq!(creature.keywords, vec![TokenKeywordShape::Menace]);
}

#[test]
fn leading_artifact_token_name_preserves_apostrophe_and_subtype() {
    let shape = parse_token_definition_shape_text(
        "Tamiyo's Notebook, a legendary colorless Book artifact token with \"{T}: Draw a card.\"",
    )
    .expect("leading named artifact token should parse");
    let TokenDefinitionSpec::Artifact(artifact) = shape else {
        panic!("expected artifact token shape");
    };
    assert_eq!(artifact.name, "Tamiyo's Notebook");
    assert_eq!(artifact.subtypes, vec![Subtype::Book]);
    assert!(artifact.legendary);
}

#[test]
fn appositive_artifact_token_name_preserves_internal_comma_and_color() {
    let shape = parse_token_definition_shape_text(
        "Icingdeath, Frost Tongue, a legendary white Equipment artifact token",
    )
    .expect("appositive named artifact token should parse");
    let TokenDefinitionSpec::Artifact(artifact) = shape else {
        panic!("expected artifact token shape");
    };
    assert_eq!(artifact.name, "Icingdeath, Frost Tongue");
    assert_eq!(artifact.subtypes, vec![Subtype::Equipment]);
    assert_eq!(artifact.colors, ColorSet::WHITE);
    assert!(artifact.legendary);
}

#[test]
fn appositive_creature_token_name_can_start_with_the_and_contain_subtypes() {
    let shape = parse_token_definition_shape_text(
        "The Tiger God, a legendary 4/4 green Cat God creature token",
    )
    .expect("article-prefixed appositive named creature token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(creature.name, "The Tiger God");
    assert_eq!(creature.subtypes, vec![Subtype::God, Subtype::Cat]);
    assert_eq!(creature.power_toughness, (4, 4));
    assert_eq!(creature.colors, ColorSet::GREEN);
    assert!(creature.legendary);
}

#[test]
fn appositive_named_construct_uses_the_name_not_the_subtype() {
    let shape = parse_token_definition_shape_text(
            "Mechtitan, a legendary 10/10 Construct artifact creature token with flying and haste that's all colors",
        )
        .expect("named Construct token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected named creature token shape");
    };
    assert_eq!(creature.name, "Mechtitan");
    assert_eq!(creature.power_toughness, (10, 10));
    assert!(creature.subtypes.contains(&Subtype::Construct));
    assert!(creature.legendary);
}

#[test]
fn token_shape_accepts_hyphenated_creature_subtype() {
    let shape = parse_token_definition_shape_text(
        "a 2/2 colorless Assembly-Worker artifact creature token",
    )
    .expect("hyphenated creature subtype token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(creature.power_toughness, (2, 2));
    assert!(creature.card_types.contains(&CardType::Artifact));
    assert!(creature.card_types.contains(&CardType::Creature));

    assert!(matches!(
        parse_token_definition_shape_text("2/2 colorless Assembly-Worker artifact creature"),
        Some(TokenDefinitionSpec::Creature(_))
    ));
}

#[test]
fn construct_artifact_scaling_requires_explicit_rules_text() {
    let dynamic =
        parse_token_definition_shape_text("X/X colorless Construct artifact creature token")
            .expect("dynamic Construct token should parse");
    assert!(matches!(
        dynamic,
        TokenDefinitionSpec::Construct(ConstructTokenShape {
            power_toughness: (0, 0),
            artifact_scaling: None,
        })
    ));

    let explicit = parse_token_definition_shape_text(
            "colorless Construct artifact creature token with \"This token's power and toughness are each equal to the number of artifacts you control.\"",
        )
        .expect("explicit artifact-scaling Construct should parse");
    assert!(matches!(
        explicit,
        TokenDefinitionSpec::Construct(ConstructTokenShape {
            artifact_scaling: Some(ConstructArtifactScalingShape::CharacteristicDefining),
            ..
        })
    ));

    let explicit_plus = parse_token_definition_shape_text(
            "0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
        )
        .expect("explicit artifact-pump Construct should parse");
    assert!(matches!(
        explicit_plus,
        TokenDefinitionSpec::Construct(ConstructTokenShape {
            artifact_scaling: Some(ConstructArtifactScalingShape::GetsPlusOnePerArtifact),
            ..
        })
    ));
}

#[test]
fn creature_token_shape_keeps_embedded_dies_creation_rule() {
    let shape = parse_token_definition_shape_text(
        "1/1 green Boar creature token with \"When this token dies, create a Food token.\"",
    )
    .unwrap();
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(
        creature.rules.token_rules.embedded_rules,
        vec![
            crate::model::token_definition::TokenEmbeddedRuleShape::DiesCreateBuiltinToken {
                token: BuiltinTokenShape::Food,
                count: 1,
            }
        ]
    );
}

#[test]
fn qualified_blocking_rule_is_typed_without_unconditional_fallbacks() {
    let shape = parse_token_definition_shape_text(
            "a 1/1 colorless Spirit creature token with \"This token can't block or be blocked by non-Spirit creatures.\"",
        )
        .expect("qualified Spirit blocking token should parse");
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };

    assert_eq!(creature.rules.combat_restriction, None);
    assert_eq!(
            creature.rules.token_rules.embedded_rules,
            vec![
                crate::model::token_definition::TokenEmbeddedRuleShape::CantBlockOrBeBlockedByNonSubtypeCreatures {
                    subtype: Subtype::Spirit,
                }
            ]
        );

    for (text, expected) in [
        (
            "a 1/1 creature token with \"This token can't block.\"",
            TokenCombatRestrictionShape::CantBlock,
        ),
        (
            "a 1/1 creature token with \"This token can't be blocked.\"",
            TokenCombatRestrictionShape::Unblockable,
        ),
    ] {
        let shape = parse_token_definition_shape_text(text)
            .expect("ordinary unconditional blocking rule should parse");
        let TokenDefinitionSpec::Creature(creature) = shape else {
            panic!("expected creature token shape");
        };
        assert_eq!(creature.rules.combat_restriction, Some(expected));
    }
}

#[test]
fn leading_named_token_shape_binds_quoted_rule_self_reference() {
    let shape = parse_token_definition_shape_text(
            "Zabu, a legendary 2/2 green Cat creature token with \"Landfall — Whenever a land you control enters, put a +1/+1 counter on Zabu.\"",
        )
        .unwrap();
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(creature.name, "Zabu");
    assert_eq!(
        creature.rules.token_rules.embedded_rules,
        vec![
            crate::model::token_definition::TokenEmbeddedRuleShape::LandEntersPutCountersOnSelf {
                counter_type: crate::object::CounterType::PlusOnePlusOne,
                count: 1,
            }
        ]
    );
}

#[test]
fn creature_token_shape_distinguishes_referenced_card_name_from_token_name() {
    let mut tokens = lex_line(
            "Jumblebones, a legendary 2/1 black Skeleton creature with \"Jumblebones can't block\" and \"When Jumblebones leaves the battlefield, return target card named Ozox, the Clattering King from your graveyard to your hand.\"",
            0,
        )
        .unwrap();
    assert!(tokens.last().is_some_and(OwnedLexToken::is_quote));
    tokens.pop();

    let shape = parse_token_definition_shape_tokens(&tokens).unwrap();
    let TokenDefinitionSpec::Creature(creature) = shape else {
        panic!("expected creature token shape");
    };
    assert_eq!(
        creature.rules.leaves_return_named_to_hand.as_deref(),
        Some("Ozox, the Clattering King")
    );
    assert_eq!(
        creature.rules.authored_inline_rules,
        vec![
            CreatureTokenInlineRulePresentation {
                kind: CreatureTokenInlineRuleKind::CombatRestriction,
                self_surface: Some(SourceReferenceSurface::FullName("Jumblebones".into())),
            },
            CreatureTokenInlineRulePresentation {
                kind: CreatureTokenInlineRuleKind::LeavesReturnNamedToHand,
                self_surface: Some(SourceReferenceSurface::FullName("Jumblebones".into())),
            },
        ],
        "specialized quoted abilities must retain authored order and named self surface"
    );
}

#[test]
fn canonical_token_names_are_typed_complete_leaves() {
    for (text, expected) in [("Heartwood", BuiltinTokenShape::Heartwood),
        ("Vibranium", BuiltinTokenShape::Vibranium), ("Gingerbrute", BuiltinTokenShape::Gingerbrute),
        ("Mutavault", BuiltinTokenShape::Mutavault), ("Spellgorger Weird", BuiltinTokenShape::SpellgorgerWeird),
        ("Tarmogoyf", BuiltinTokenShape::Tarmogoyf)] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert_eq!(parse_token_definition_shape_tokens(&tokens), Some(TokenDefinitionSpec::Builtin(expected)));
    }
    for text in ["Heartwood with flying", "Vibranium and draw a card", "Gingerbrute which is legendary",
        "Mutavault with a charge counter", "Spellgorger Weird named Something Else"] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(super::super::rules::parse_canonical_named_token_shape(&tokens).is_none(), "{text}");
    }
}

#[test]
fn token_name_words_and_quoted_protection_colors_do_not_become_description_characteristics() {
    for text in [
        "Red Elf, a legendary 2/2 blue Human creature token with protection from green",
        "a 2/2 blue Human creature token named Red Elf with protection from green",
    ] {
        let TokenDefinitionSpec::Creature(shape) = parse_token_definition_shape_text(text).unwrap() else { panic!("described creature"); };
        assert_eq!(shape.name, "Red Elf");
        assert_eq!(shape.subtypes, vec![crate::types::Subtype::Human]);
        assert_eq!(shape.colors, crate::color::ColorSet::BLUE);
        let roles = shape.text_roles.unwrap();
        assert_eq!(roles.name, ironsmith_core::TokenNameTextRole::Explicit);
        assert_eq!(roles.colors, ironsmith_core::TokenWordRole::Authored);
    }
    let TokenDefinitionSpec::Creature(shape) = parse_token_definition_shape_text("a 2/2 blue Human creature token").unwrap() else { panic!("described creature"); };
    assert_eq!(shape.text_roles.unwrap().name, ironsmith_core::TokenNameTextRole::SubtypeDerived);
    for (text, expected) in [("that are all colors", ironsmith_core::TokenWordRole::RulesImplied),
        ("that are white, blue, black, red, and green", ironsmith_core::TokenWordRole::Authored)]
    {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        let all = crate::color::ColorSet::WHITE.union(crate::color::ColorSet::BLUE).union(crate::color::ColorSet::BLACK)
            .union(crate::color::ColorSet::RED).union(crate::color::ColorSet::GREEN);
        assert_eq!(parse_postnominal_token_color_words_tokens(&tokens), Some((all, expected)));
    }
}

#[test]
fn quoted_rule_words_do_not_define_outer_template_characteristics_or_names() {
    let artifact = parse_token_definition_shape_text(
        "a colorless artifact token with \"{T}: Target Equipment becomes legendary until end of turn.\"").unwrap();
    let TokenDefinitionSpec::Artifact(artifact) = artifact else { panic!("outer artifact description"); };
    assert!(artifact.subtypes.is_empty());
    assert!(!artifact.legendary);
    let enchantment = parse_token_definition_shape_text(
        "a blue enchantment token with \"{T}: Target Saga becomes a Vehicle artifact until end of turn.\"").unwrap();
    let TokenDefinitionSpec::Enchantment(enchantment) = enchantment else { panic!("outer enchantment description"); };
    assert!(enchantment.subtypes.is_empty());
    assert!(!enchantment.legendary);
    assert_eq!(enchantment.colors, crate::color::ColorSet::BLUE);
    let creature = parse_token_definition_shape_text(
        "a 2/2 blue Human creature token with \"Creatures named Elf get +1/+1.\"").unwrap();
    let TokenDefinitionSpec::Creature(creature) = creature else { panic!("outer creature description"); };
    assert_eq!(creature.subtypes, vec![crate::types::Subtype::Human]);
    assert_eq!(creature.text_roles.unwrap().name, ironsmith_core::TokenNameTextRole::SubtypeDerived);
    let flying_name = parse_token_definition_shape_text("a 2/2 blue Human creature token named Flying").unwrap();
    let TokenDefinitionSpec::Creature(flying_name) = flying_name else { panic!("explicit keyword-spelled name"); };
    assert_eq!(flying_name.name, "Flying");
    assert!(!flying_name.keywords.contains(&TokenKeywordShape::Flying));
}

#[test]
fn vehicle_characteristics_and_keywords_use_only_their_own_description() {
    let vehicle = parse_token_definition_shape_text(
        "a legendary 3/3 blue artifact Vehicle token with \"{T}: Target creature gains flying and crew 4 until end of turn.\"").unwrap();
    let TokenDefinitionSpec::Vehicle(vehicle) = vehicle else { panic!("outer vehicle description"); };
    assert!(vehicle.legendary);
    assert_eq!(vehicle.colors, crate::color::ColorSet::BLUE);
    assert!(!vehicle.flying);
    assert_eq!(vehicle.crew_amount, None);
    let vehicle = parse_token_definition_shape_text("a 3/3 blue artifact Vehicle token with flying and crew 2").unwrap();
    let TokenDefinitionSpec::Vehicle(vehicle) = vehicle else { panic!("own vehicle keywords"); };
    assert!(vehicle.flying);
    assert_eq!(vehicle.crew_amount, Some(2));
}

#[test]
fn named_keywords_and_quoted_grants_do_not_set_intrinsic_keyword_flags() {
    for name in ["Hexproof", "Indestructible", "Banding", "Changeling"] {
        let TokenDefinitionSpec::Creature(shape) = parse_token_definition_shape_text(
            &format!("a 2/2 blue Human creature token named {name}")).unwrap() else { panic!("creature description"); };
        assert_eq!(shape.name, name);
        assert!(!shape.rules.hexproof && !shape.rules.indestructible && !shape.rules.banding && !shape.rules.changeling);
    }
    let TokenDefinitionSpec::Creature(shape) = parse_token_definition_shape_text(
        "a 2/2 blue Human creature token with \"{T}: Target creature gains indestructible and double strike until end of turn.\"").unwrap()
    else { panic!("creature description"); };
    assert!(!shape.rules.indestructible && !shape.rules.double_strike && !shape.rules.first_strike);
}

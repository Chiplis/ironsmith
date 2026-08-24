use super::*;

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

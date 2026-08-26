use crate::lexer::lex_line;

use super::*;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("lex fixture")
}

#[test]
fn rest_shape_strips_verb_and_returns_typed_copy_exception() {
    let shape = parse_become_rest_shape(&lex(
        "becomes a copy of target creature except its name is Relic and it has this ability",
    ));
    assert!(shape.copy_exception.unwrap().preserve_source_abilities);
    assert_eq!(
        parser_token_word_refs(&shape.body_tokens),
        ["a", "copy", "of", "target", "creature"]
    );
}

#[test]
fn rest_shape_separates_copy_exception_from_duration() {
    let shape = parse_become_rest_shape(&lex(
        "becomes a copy of target creature until end of turn, except it has flying",
    ));
    let exception = shape.copy_exception.expect("copy exception");
    assert_eq!(
        parser_token_word_refs(
            exception
                .granted_ability_tokens
                .as_deref()
                .expect("granted ability tokens")
        ),
        ["flying"]
    );
    assert_eq!(
        parser_token_word_refs(&shape.body_tokens),
        [
            "a", "copy", "of", "target", "creature", "until", "end", "of", "turn"
        ]
    );
}

#[test]
fn copy_exception_preserves_name_pt_keyword_and_source_ability() {
    let shape = parse_become_rest_shape(&lex(
        "becomes a copy of up to one other target creature until end of turn, except his name is Hulkling, Young Avenger, he's 4/4, and he has flying and this ability",
    ));
    let exception = shape.copy_exception.expect("copy exception");
    assert_eq!(
        exception.name_override.as_deref(),
        Some("Hulkling, Young Avenger")
    );
    assert_eq!(exception.set_base_power_toughness, Some((4, 4)));
    assert!(exception.preserve_source_abilities);
    assert_eq!(
        parser_token_word_refs(
            exception
                .granted_ability_tokens
                .as_deref()
                .expect("granted ability tokens")
        ),
        ["flying"]
    );
}

#[test]
fn structured_copy_exceptions_preserve_typed_characteristic_bundles() {
    let vehicle = parse_become_copy_exception_shape(&lex(
        "it's 4/3, it's a Vehicle artifact in addition to its other types, and it has flying",
    ))
    .expect("vehicle copy exception");
    assert_eq!(vehicle.set_base_power_toughness, Some((4, 3)));
    assert_eq!(vehicle.add_card_types, [CardType::Artifact]);
    assert_eq!(vehicle.add_subtypes, [Subtype::Vehicle]);
    assert_eq!(
        parser_token_word_refs(
            vehicle
                .granted_ability_tokens
                .as_deref()
                .expect("flying tokens")
        ),
        ["flying"]
    );

    let named = parse_become_copy_exception_shape(&lex(
            "his name is Taskmaster, Mercenary Mimic and he's a legendary Human Mercenary Villain creature",
        ))
        .expect("named type-line copy exception");
    assert_eq!(
        named.name_override.as_deref(),
        Some("Taskmaster, Mercenary Mimic")
    );
    assert_eq!(named.add_supertypes, [Supertype::Legendary]);
    assert_eq!(named.set_card_types, [CardType::Creature]);
    assert_eq!(
        named.set_subtypes,
        [Subtype::Human, Subtype::Mercenary, Subtype::Villain]
    );

    let preserved = parse_become_copy_exception_shape(&lex("it's 7/5 and it has this ability"))
        .expect("preserved ability copy exception");
    assert_eq!(preserved.set_base_power_toughness, Some((7, 5)));
    assert!(preserved.preserve_source_abilities);

    let named_preserved = parse_become_copy_exception_shape(&lex(
        "her name is Irma, Part-Time Mutant and she has this ability",
    ))
    .expect("named preserved ability copy exception");
    assert_eq!(
        named_preserved.name_override.as_deref(),
        Some("Irma, Part-Time Mutant")
    );
    assert!(named_preserved.preserve_source_abilities);
    assert_eq!(
        named_preserved.surface.as_deref(),
        Some("her name is Irma, Part-Time Mutant and she has this ability")
    );
}

#[test]
fn structured_copy_exception_combines_name_type_pt_and_keyword() {
    let exception = parse_become_copy_exception_shape(&lex(
        "his name is Mirror Adept, he's a legendary 4/4 Human Villain creature in addition to his other types, and he has vigilance",
    ))
    .expect("combined named copy exception");

    assert_eq!(exception.name_override.as_deref(), Some("Mirror Adept"));
    assert_eq!(exception.add_supertypes, [Supertype::Legendary]);
    assert_eq!(exception.add_card_types, [CardType::Creature]);
    assert_eq!(exception.add_subtypes, [Subtype::Human, Subtype::Villain]);
    assert_eq!(exception.set_base_power_toughness, Some((4, 4)));
    assert_eq!(
        parser_token_word_refs(
            exception
                .granted_ability_tokens
                .as_deref()
                .expect("vigilance tokens")
        ),
        ["vigilance"]
    );
}

#[test]
fn structured_copy_exception_combines_name_colors_types_and_pt() {
    let exception = parse_become_copy_exception_shape(&lex(
        "its name is Final Form, it's 4/4, and it's a legendary blue and black Zombie creature in addition to its other colors and types",
    ))
    .expect("combined color and type copy exception");

    assert_eq!(exception.name_override.as_deref(), Some("Final Form"));
    assert_eq!(exception.add_supertypes, [Supertype::Legendary]);
    assert_eq!(exception.add_colors, ColorSet::BLUE.union(ColorSet::BLACK));
    assert_eq!(exception.add_card_types, [CardType::Creature]);
    assert_eq!(exception.add_subtypes, [Subtype::Zombie]);
    assert_eq!(exception.set_base_power_toughness, Some((4, 4)));

    assert!(parse_become_copy_exception_shape(&lex(
        "its name is Final Form, it's 4/4, and it's a legendary blueishly Zombie creature in addition to its other colors and types",
    ))
    .is_none());
}

#[test]
fn no_name_copular_copy_exceptions_share_the_typed_path() {
    let dermotaxi = parse_become_copy_exception_shape(&lex(
        "it's a Vehicle artifact in addition to its other types",
    ))
    .expect("Dermotaxi exception");
    assert_eq!(dermotaxi.add_card_types, [CardType::Artifact]);
    assert_eq!(dermotaxi.add_subtypes, [Subtype::Vehicle]);

    let mimeoplasm = parse_become_copy_exception_shape(&lex("it's 0/0 and has this ability"))
        .expect("Mimeoplasm exception");
    assert_eq!(mimeoplasm.set_base_power_toughness, Some((0, 0)));
    assert!(mimeoplasm.preserve_source_abilities);

    let mindlink = parse_become_copy_exception_shape(&lex(
        "it's 4/3, it's a Vehicle artifact in addition to its other types, and it has flying",
    ))
    .expect("Mindlink Mech exception");
    assert_eq!(mindlink.set_base_power_toughness, Some((4, 3)));
    assert_eq!(mindlink.add_card_types, [CardType::Artifact]);
    assert_eq!(mindlink.add_subtypes, [Subtype::Vehicle]);
    assert_eq!(
        parser_token_word_refs(
            mindlink
                .granted_ability_tokens
                .as_deref()
                .expect("Mindlink flying tokens")
        ),
        ["flying"]
    );

    let mirror =
        parse_become_copy_exception_shape(&lex("it's an artifact in addition to its other types"))
            .expect("Mirror of the Forebears exception");
    assert_eq!(mirror.add_card_types, [CardType::Artifact]);
    assert!(mirror.add_subtypes.is_empty());

    let volrath = parse_become_copy_exception_shape(&lex("it's 7/5 and it has this ability"))
        .expect("Volrath exception");
    assert_eq!(volrath.set_base_power_toughness, Some((7, 5)));
    assert!(volrath.preserve_source_abilities);
}

#[test]
fn possessive_its_name_surface_is_not_treated_as_a_copula() {
    let exception =
        parse_become_copy_exception_shape(&lex("its name is Relic and it has this ability"))
            .expect("possessive name exception");
    assert_eq!(exception.name_override.as_deref(), Some("Relic"));
    assert!(exception.preserve_source_abilities);
}

#[test]
fn body_shape_classifies_exact_copy_aura_and_equal_surfaces() {
    let copy_tokens = lex("a copy of target creature");
    assert!(matches!(
        parse_become_body_surface_shape(&copy_tokens).copy_source,
        BecomeCopySourceShape::Source(_)
    ));
    let aura_tokens = lex("an Aura with enchant creature you control");
    assert!(
        parse_become_body_surface_shape(&aura_tokens)
            .aura
            .unwrap()
            .attachment_you_control
    );
    let equal_tokens = lex("equal to this power and toughness");
    assert!(parse_become_body_surface_shape(&equal_tokens).equal_to_source_power_toughness);
}

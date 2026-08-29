use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_controller_owner_and_base_pt_subjects() {
    let tokens = lex_line("the controller of target creature", 0).expect("lex");
    let shape = parse_controller_owner_subject_tokens(&tokens).expect("controller subject");
    assert_eq!(shape.subject, SubjectAst::Player(PlayerAst::ItsController));

    let tokens = lex_line("that spell or ability's controller", 0).expect("lex");
    let shape = parse_controller_owner_subject_tokens(&tokens)
        .expect("triggering stack-object controller subject");
    assert_eq!(shape.subject, SubjectAst::TriggeringSourceController);
    assert!(matches!(
        shape.target,
        TargetAst::Tagged(ref tag, None) if tag.as_str() == "triggering_source"
    ));

    let tokens = lex_line("target creature's base power and toughness", 0).expect("lex");
    let shape = parse_base_power_toughness_subject_tokens(&tokens).expect("base pt subject");
    assert!(!shape.target_tokens.is_empty());
}

#[test]
fn owner_of_target_keeps_heterogeneous_zone_union() {
    let tokens = lex_line(
        "the owner of target spell, nonland permanent, or card in a graveyard",
        0,
    )
    .expect("lex owner subject");
    let shape = parse_controller_owner_subject_tokens(&tokens).expect("owner subject");
    assert_eq!(shape.subject, SubjectAst::Player(PlayerAst::ItsOwner));
    let TargetAst::Object(filter, explicit_target, _) = shape.target else {
        panic!("expected object target union");
    };
    assert!(explicit_target.is_some());
    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(crate::Zone::Stack)
            && branch.stack_kind == Some(crate::filter::StackObjectKind::Spell)
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.zone == Some(crate::Zone::Battlefield)
            && branch.excluded_card_types == [crate::CardType::Land]
    }));
    assert!(
        filter
            .any_of
            .iter()
            .any(|branch| branch.zone == Some(crate::Zone::Graveyard))
    );
}

#[test]
fn named_possessive_controller_subject_persists_source_identity() {
    let tokens = lex_line("Hold for Ransom's controller", 0).expect("lex named controller");
    let context = crate::parse_context::ParseContext::for_fragment(
        "Hold for Ransom",
        Vec::new(),
        Vec::new(),
        "Hold for Ransom's controller",
    );
    let shape = parse_controller_owner_subject_tokens_with_context(context.view(), &tokens)
        .expect("controller subject");
    assert_eq!(shape.subject, SubjectAst::Player(PlayerAst::ItsController));
    let TargetAst::Object(filter, None, None) = shape.target else {
        panic!("named source must persist in an object-backed source target");
    };
    assert!(filter.source);
    assert_eq!(
        filter.source_surface,
        Some(crate::target::SourceReferenceSurface::FullName(
            "Hold for Ransom".to_string()
        ))
    );
}

#[test]
fn parses_become_pt_tails() {
    let words = [
        "red",
        "dragon",
        "with",
        "base",
        "power",
        "and",
        "toughness",
        "x/x",
    ];
    let shape = parse_become_base_pt_words(&words).expect("base pt tail");
    assert_eq!(shape.power, Value::X);
    assert_eq!(shape.toughness, Value::X);

    let words = [
        "a",
        "green",
        "and",
        "blue",
        "fractal",
        "with",
        "base",
        "power",
        "and",
        "toughness",
        "each",
        "equal",
        "to",
        "x",
        "plus",
        "1",
    ];
    let shape = parse_become_base_pt_words(&words).expect("dynamic base pt tail");
    let expected = Value::Add(Box::new(Value::X), Box::new(Value::Fixed(1)));
    assert_eq!(shape.power, expected);
    assert_eq!(shape.toughness, expected);

    let words = [
        "creature",
        "with",
        "power",
        "and",
        "toughness",
        "each",
        "equal",
        "to",
        "their",
        "mana",
        "value",
    ];
    assert!(parse_become_iterated_mana_value_pt_words(&words).is_some());
}

#[test]
fn parses_filtered_per_object_animation_shapes() {
    let cases = [
        (
            "Each noncreature artifact is an artifact creature with power and toughness each equal to its mana value.",
            false,
            false,
        ),
        (
            "Each planeswalker with one or more loyalty counters on it loses all abilities and is a creature with power and toughness each equal to the number of loyalty counters on it.",
            true,
            false,
        ),
        (
            "It's an artifact creature with power and toughness each equal to its mana value.",
            false,
            true,
        ),
    ];

    for (text, removes_all_abilities, dependent_subject) in cases {
        let tokens = lex_line(text, 0).expect("lex animation");
        let shape = parse_filtered_object_animation_tokens(&tokens)
            .unwrap_or_else(|| panic!("animation shape should parse: {text}"));
        assert_eq!(shape.removes_all_abilities, removes_all_abilities, "{text}");
        assert_eq!(shape.dependent_subject, dependent_subject, "{text}");
        assert!(!shape.preserve_other_types, "{text}: {shape:#?}");
        assert!(
            shape
                .descriptor
                .card_types
                .contains(&crate::types::CardType::Creature),
            "{text}: {shape:#?}"
        );
        if text.contains("loyalty") {
            assert!(
                matches!(shape.power, Value::CountersOn(ref spec, Some(crate::CounterType::Loyalty)) if matches!(spec.as_ref(), ChooseSpec::Iterated)),
                "{shape:#?}"
            );
        } else {
            assert!(
                matches!(shape.power, Value::ManaValueOf(ref spec) if matches!(spec.as_ref(), ChooseSpec::Iterated)),
                "{shape:#?}"
            );
        }
    }
}

#[test]
fn parses_filtered_leading_pt_animation_in_addition_to_other_types() {
    let text = "Each non-Equipment artifact and non-Aura enchantment you control with mana value 4 or greater is a 4/4 Elemental creature in addition to its other types.";
    let tokens = lex_line(text, 0).expect("lex animation");
    let shape = parse_filtered_object_animation_tokens(&tokens)
        .expect("leading-P/T additive animation should parse");

    assert!(shape.preserve_other_types, "{shape:#?}");
    assert_eq!(shape.power, Value::Fixed(4));
    assert_eq!(shape.toughness, Value::Fixed(4));
    assert!(
        shape
            .descriptor
            .card_types
            .contains(&crate::types::CardType::Creature),
        "{shape:#?}"
    );
    assert!(
        shape
            .descriptor
            .subtypes
            .contains(&crate::types::Subtype::Elemental),
        "{shape:#?}"
    );
}

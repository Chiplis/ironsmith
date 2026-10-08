//! Complete-filter dispatch expectations, authored but not run.
use super::*;

fn readers() -> [fn(&[OwnedLexToken], bool) -> Result<ObjectFilter, CardTextError>; 4] {
    [
        parse_object_filter_with_grammar_entrypoint,
        crate::grammar::filters::parse_object_filter_with_grammar_entrypoint_lexed,
        crate::object_filters::parse_object_filter,
        crate::object_filters::parse_object_filter_lexed,
    ]
}

fn suspended() -> ObjectFilter {
    ObjectFilter::default().in_zone(Zone::Exile)
        .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
        .with_counter_type(crate::object::CounterType::Time)
}

#[test]
fn complete_union_is_owned_before_characteristic_classification() {
    for (text, put) in [
        ("permanent or suspended card", false),
        ("target permanent or suspended card", false),
        ("all permanents or suspended cards", false),
        ("permanent with a time counter on it or suspended card", true),
        ("target permanent with a time counter on it or suspended card", true),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        for reader in readers() {
            for other in [false, true] {
                let permanent = if put {
                    ObjectFilter::permanent().with_counter_type(crate::object::CounterType::Time)
                } else { ObjectFilter::permanent() };
                let expected = ObjectFilter {
                    other, any_of: vec![permanent, suspended()], ..ObjectFilter::default()
                };
                let actual = reader(&tokens, other).unwrap();
                assert_eq!(actual, expected, "{text}, other={other}");
                if text.starts_with("all ") {
                    assert_eq!(actual.set_quantifier_surface(), Some(ironsmith_core::SetQuantifierSurface::All));
                }
            }
        }
    }
}

#[test]
fn each_arm_keeps_its_own_zone_controller_owner_and_counter_constraints() {
    for (text, expected) in [
        ("permanent you control or suspended card you own", ObjectFilter {
            any_of: vec![ObjectFilter::permanent().you_control(), suspended().owned_by(PlayerFilter::You)],
            ..ObjectFilter::default()
        }),
        ("suspended card you own and each other permanent you control with a time counter on it", ObjectFilter {
            any_of: vec![suspended().owned_by(PlayerFilter::You), ObjectFilter {
                other: true,
                ..ObjectFilter::permanent().you_control().with_counter_type(crate::object::CounterType::Time)
            }],
            ..ObjectFilter::default()
        }),
        ("nonland permanent or suspended card", ObjectFilter {
            any_of: vec![ObjectFilter {
                excluded_card_types: vec![CardType::Land], ..ObjectFilter::permanent()
            }, suspended()],
            ..ObjectFilter::default()
        }),
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        for reader in readers() {
            assert_eq!(reader(&tokens, false).unwrap(), expected, "{text}");
        }
    }
}

#[test]
fn incomplete_union_never_falls_through_to_a_partial_relational_noun() {
    for text in [
        "permanent or suspended card unknown",
        "permanent unknown or suspended card",
        "permanent or suspended card you control",
        "permanent with a charge counter on it or suspended card",
        "permanent with two time counters on it or suspended card",
        "permanent or suspended card in your hand",
        "all permanents or suspended cards in your hand",
        "permanent or suspended creature",
        "permanent or or suspended card",
        "permanent or suspended card or",
        "permanent or suspended card or creature",
        "permanent: or suspended card",
        "permanent or suspended: card",
        "permanent or suspended card: you own",
        "permanent; or suspended card",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        for reader in readers() {
            assert!(reader(&tokens, false).is_err(), "accepted incomplete union: {text}");
        }
    }
    // The repair must not revive the historical permissive fallback for a
    // completely unrelated characteristic-only noun and unknown qualifier.
    let tokens = crate::lexer::lex_line("creature unknown", 0).unwrap();
    assert!(parse_object_filter_with_grammar_entrypoint(&tokens, false).is_err());
}

#[test]
fn a_nested_union_is_left_to_its_enclosing_relation_owner() {
    for text in [
        "spell that targets a permanent or suspended card",
        "permanent that targets a permanent or suspended card",
        "all permanents that target a permanent or suspended card",
        "permanent you control that targets a suspended card or permanent",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(crate::grammar::filters::reference_tag_stage::parse_complete_permanent_or_suspended_card_filter(&tokens, false).is_none());
    }
}

#[test]
fn a_top_level_union_cannot_lose_a_following_relation_or_qualifier() {
    let relational_readers: [fn(&[OwnedLexToken], bool) -> Result<ObjectFilter, CardTextError>; 2] = [
        crate::grammar::filters::reference_tag_stage::parse_object_filter,
        crate::grammar::filters::reference_tag_stage::parse_object_filter_permissive,
    ];
    for text in [
        "permanent or suspended card that targets a creature",
        "target permanent or suspended card that targets a creature",
        "permanent or suspended card that targets you",
        "permanent or suspended card that targets only a single creature",
        "permanent or suspended card that targets two creatures",
        "all permanents or suspended cards that target a creature",
        "suspended card or permanent that targets a creature",
        "suspended card and each other permanent that targets a creature",
        "permanent with a time counter on it or suspended card that targets a creature",
        "permanent or suspended card that targets a permanent or suspended card",
        "permanent or suspended card attached to a creature that targets you",
        "permanent or suspended card other than this creature that targets you",
        "permanent or suspended card with a single target",
        "permanent or suspended card attached to a creature",
        "permanent or suspended card other than this creature",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        for other in [false, true] {
            for reader in readers().into_iter().chain(relational_readers) {
                assert!(reader(&tokens, other).is_err(), "lost union tail: {text}, other={other}");
            }
        }
    }
}

#[test]
fn an_enclosing_target_relation_keeps_its_supported_union_operand() {
    // The standalone suspended fragment needs an explicit recognized predicate
    // in the existing relation reader. This does not expand that reader's grammar.
    for text in [
        "spell that targets a suspended card with a time counter on it or a permanent",
        "permanent that targets a suspended card with a time counter on it or a permanent",
    ] {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        assert!(crate::grammar::filters::reference_tag_stage::parse_complete_permanent_or_suspended_card_filter(&tokens, false).is_none());
        for reader in readers() {
            for other in [false, true] {
                let actual = reader(&tokens, other).unwrap();
                assert_eq!(actual.zone, Some(Zone::Stack), "{text}");
                assert_eq!(actual.other, other, "{text}");
                assert!(actual.any_of.is_empty(), "the union must stay inside the relation: {text}");
                assert_eq!(actual.targets_object.as_deref(), Some(&ObjectFilter {
                    any_of: vec![suspended(), ObjectFilter::permanent()], ..ObjectFilter::default()
                }), "{text}");
                assert!(actual.targets_player.is_none());
                assert!(actual.targets_only_object.is_none());
                assert!(actual.targets_only_player.is_none());
                assert!(actual.target_count.is_none(), "an article does not impose target arity");
            }
        }
    }
}

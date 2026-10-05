use super::*;
use crate::lexer::lex_line;

#[test]
fn sacrifice_segments_preserve_source_and_choice_shapes() {
    let source = lex_line("sacrifice this creature", 0).unwrap();
    assert_eq!(
        parse_sacrifice_segment_tokens(&source, |_| None).unwrap(),
        ActivationCostSegmentCst::SacrificeSelf { surface: None }
    );
    let token_source = lex_line("sacrifice this token", 0).unwrap();
    assert_eq!(
        parse_sacrifice_segment_tokens(&token_source, |_| None).unwrap(),
        ActivationCostSegmentCst::SacrificeSelf { surface: None }
    );
    let chosen = lex_line("sacrifice up to two other artifacts", 0).unwrap();
    assert_eq!(
        parse_sacrifice_segment_tokens(&chosen, |_| None).unwrap(),
        ActivationCostSegmentCst::SacrificeChosen {
            count: ChoiceCount::up_to(2),
            filter: ObjectFilter {
                other: true,
                ..ObjectFilter::artifact()
            },
        }
    );

    let dynamic = lex_line("sacrifice X Goats", 0).unwrap();
    let ActivationCostSegmentCst::SacrificeChosen { count, filter } =
        parse_sacrifice_segment_tokens(&dynamic, |_| None).unwrap()
    else {
        panic!("expected a typed dynamic sacrifice cost");
    };
    assert!(count.is_dynamic_x());
    assert_eq!(filter.subtypes, [crate::types::Subtype::Goat]);

    let prism = lex_line("sacrifice a Prism token", 0).unwrap();
    let ActivationCostSegmentCst::SacrificeChosen { count, filter } =
        parse_sacrifice_segment_tokens(&prism, |_| None).unwrap()
    else {
        panic!("expected a typed Prism-token sacrifice cost");
    };
    assert_eq!(count, ChoiceCount::exactly(1));
    assert_eq!(filter.subtypes, [crate::types::Subtype::Prism]);
    assert!(filter.token);

    let all = lex_line("sacrifice all lands", 0).unwrap();
    assert_eq!(
        parse_sacrifice_segment_tokens(&all, |_| None).unwrap(),
        ActivationCostSegmentCst::SacrificeAll {
            filter: ObjectFilter::land(),
        }
    );

    let missing = lex_line("sacrifice", 0).unwrap();
    let error = parse_sacrifice_segment_tokens(&missing, |_| None).unwrap_err();
    let message = error.to_string().to_ascii_lowercase();
    assert!(message.contains("sacrifice"), "{message}");
    assert!(message.contains("filter"), "{message}");
}

#[test]
fn discard_segments_preserve_card_named_and_disjunction_shapes() {
    let cards = lex_line("discard two artifact cards", 0).unwrap();
    assert_eq!(
        parse_discard_segment_tokens(&cards).unwrap(),
        ActivationCostSegmentCst::DiscardFiltered {
            count: 2,
            card_types: vec![CardType::Artifact],
            supertypes: Vec::new(),
            filter: None,
            random: false,
            name: None,
            other: false,
        }
    );

    let named = lex_line("discard a card named black lotus", 0).unwrap();
    assert_eq!(
        parse_discard_segment_tokens(&named).unwrap(),
        ActivationCostSegmentCst::DiscardFiltered {
            count: 1,
            card_types: Vec::new(),
            supertypes: Vec::new(),
            filter: None,
            random: false,
            name: Some("black lotus".to_string()),
            other: false,
        }
    );

    let disjunction = lex_line("discard an artifact or creature card", 0).unwrap();
    let ActivationCostSegmentCst::DiscardFiltered { filter, .. } =
        parse_discard_segment_tokens(&disjunction).unwrap()
    else {
        panic!("expected typed discard filter");
    };
    assert_eq!(filter.unwrap().any_of.len(), 2);
}

#[test]
fn unattach_and_tap_segments_return_typed_filters() {
    let unattach = lex_line("unattach an equipment from this creature", 0).unwrap();
    assert_eq!(
        parse_unattach_segment_tokens(&unattach, |words| {
            leaf::parse_leaf_this_source_reference_words(words).is_some()
        })
        .unwrap(),
        ActivationCostSegmentCst::UnattachChosen {
            count: 1,
            filter: ObjectFilter {
                attached_to_object: Some(Box::new(ObjectFilter::source())),
                ..ObjectFilter::artifact().with_subtype(crate::types::Subtype::Equipment)
            },
        }
    );

    let tap = lex_line("tap two other untapped creatures you control", 0).unwrap();
    assert_eq!(
        parse_tap_chosen_segment_tokens(&tap).unwrap(),
        ActivationCostSegmentCst::TapChosen {
            count: ChoiceCount::exactly(2),
            filter: ObjectFilter {
                other: true,
                untapped: true,
                ..ObjectFilter::creature().you_control()
            },
        }
    );

    let source = lex_line("unattach this source", 0).unwrap();
    assert_eq!(
        parse_unattach_segment_tokens(&source, |words| {
            leaf::parse_leaf_this_source_reference_words(words).is_some()
        })
        .unwrap(),
        ActivationCostSegmentCst::UnattachChosen {
            count: 1,
            filter: ObjectFilter::source(),
        }
    );
}

#[test]
fn tap_x_untapped_costs_preserve_exact_variable_count_and_filter() {
    for (text, expected) in [
        (
            "Tap X untapped artifacts you control",
            ObjectFilter::artifact().you_control(),
        ),
        (
            "Tap X untapped creatures you control",
            ObjectFilter::creature().you_control(),
        ),
        (
            "Tap X untapped Knights you control",
            ObjectFilter::default()
                .with_subtype(Subtype::Knight)
                .you_control(),
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let ActivationCostSegmentCst::TapChosen { count, filter } =
            parse_tap_chosen_segment_tokens(&tokens).unwrap()
        else {
            panic!("expected a tap-chosen cost");
        };
        assert_eq!(count, ChoiceCount::dynamic_x());
        assert_eq!(
            filter,
            ObjectFilter {
                zone: Some(Zone::Battlefield),
                untapped: true,
                ..expected
            }
        );
    }
    for text in [
        "Tap X untapped",
        "Tap X tapped creatures you control",
        "Tap X untapped creatures you control at random",
    ] {
        assert!(
            parse_tap_chosen_segment_tokens(&lex_line(text, 0).unwrap()).is_err(),
            "{text}"
        );
    }
}

#[test]
fn chosen_untap_and_attachment_tap_costs_preserve_count_scope_and_identity() {
    for (text, count, opponent) in [
        ("Untap a tapped land an opponent controls", 1, true),
        ("Untap two tapped blue creatures you control", 2, false),
        ("Untap fifteen tapped creatures you control", 15, false),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let ActivationCostSegmentCst::UntapChosen {
            count: parsed,
            filter,
        } = parse_untap_chosen_segment_tokens(&tokens).unwrap()
        else {
            panic!("typed untap cost");
        };
        assert_eq!(parsed, ChoiceCount::exactly(count));
        assert!(filter.tapped);
        assert!(!filter.untapped);
        assert_eq!(
            filter.controller,
            Some(if opponent {
                crate::target::PlayerFilter::Opponent
            } else {
                crate::target::PlayerFilter::You
            })
        );
    }
    for (text, expected_tag) in [
        ("Tap enchanted land", "enchanted"),
        ("Tap enchanted creature", "enchanted"),
        (
            "Tap granting permanent",
            crate::tag::CompilerReferenceTag::GrantingSource.as_str(),
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let ActivationCostSegmentCst::TapChosen { count, filter } =
            parse_tap_chosen_segment_tokens(&tokens).unwrap()
        else {
            panic!("typed tap cost");
        };
        assert_eq!(count, ChoiceCount::exactly(1));
        assert!(filter.untapped);
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == expected_tag)
        );
    }
    for text in ["Untap", "Untap two tapped", "Tap enchanted nonsense"] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(if text.starts_with("Untap") {
            parse_untap_chosen_segment_tokens(&tokens).is_err()
        } else {
            parse_tap_chosen_segment_tokens(&tokens).is_err()
        });
    }
}

#[test]
fn complete_discard_selectors_preserve_color_historic_x_and_other() {
    let parse = |text| parse_discard_segment_tokens(&lex_line(text, 0).unwrap()).unwrap();
    let ActivationCostSegmentCst::DiscardFiltered {
        filter: Some(nonblack),
        ..
    } = parse("discard a nonblack card")
    else {
        panic!("missing nonblack filter")
    };
    assert!(
        nonblack
            .excluded_colors
            .contains(crate::color::Color::Black)
    );
    let ActivationCostSegmentCst::DiscardFiltered {
        filter: Some(historic),
        ..
    } = parse("discard a historic card")
    else {
        panic!("missing historic filter")
    };
    assert!(historic.historic);
    let ActivationCostSegmentCst::DiscardFiltered {
        filter: Some(mana),
        count: 1,
        ..
    } = parse("discard a card with mana value x")
    else {
        panic!("missing X filter")
    };
    assert!(
        matches!(mana.mana_value, Some(crate::filter::Comparison::EqualExpr(value)) if matches!(value.unhinted(), crate::effect::Value::X))
    );
    assert!(matches!(
        parse("discard x cards"),
        ActivationCostSegmentCst::DiscardValue {
            count: crate::effect::Value::X,
            ..
        }
    ));
    assert!(matches!(
        parse("discard another card"),
        ActivationCostSegmentCst::DiscardFiltered { other: true, .. }
    ));
    for text in [
        "discard a card with mana value",
        "discard a creature from your graveyard",
        "discard a card and draw a card",
    ] {
        assert!(
            parse_discard_segment_tokens(&lex_line(text, 0).unwrap()).is_err(),
            "{text}"
        );
    }
}

#[test]
fn latest_draw_discard_cost_keeps_exact_player_and_history_predicate() {
    let tokens = lex_line("Discard the last card you drew this turn", 0).unwrap();
    let ActivationCostSegmentCst::DiscardFiltered {
        count,
        filter: Some(filter),
        ..
    } = parse_discard_segment_tokens(&tokens).unwrap()
    else {
        panic!("latest-draw filter");
    };
    assert_eq!(count, 1);
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert_eq!(filter.owner, Some(crate::target::PlayerFilter::You));
    assert_eq!(
        filter.last_drawn_this_turn,
        Some(crate::target::PlayerFilter::You)
    );
    assert!(
        parse_discard_segment_tokens(
            &lex_line("Discard the last card you drew this turn or any card", 0).unwrap()
        )
        .is_err()
    );
}

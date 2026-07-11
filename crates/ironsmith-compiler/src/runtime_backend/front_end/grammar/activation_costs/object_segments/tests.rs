use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

#[test]
fn sacrifice_segments_preserve_source_and_choice_shapes() {
    let source = lex_line("sacrifice this creature", 0).unwrap();
    assert_eq!(
        parse_sacrifice_segment_tokens(&source, |_| false).unwrap(),
        ActivationCostSegmentCst::SacrificeSelf
    );
    let chosen = lex_line("sacrifice up to two other artifacts", 0).unwrap();
    assert_eq!(
        parse_sacrifice_segment_tokens(&chosen, |_| false).unwrap(),
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
        parse_sacrifice_segment_tokens(&dynamic, |_| false).unwrap()
    else {
        panic!("expected a typed dynamic sacrifice cost");
    };
    assert!(count.is_dynamic_x());
    assert_eq!(filter.subtypes, [crate::types::Subtype::Goat]);

    let missing = lex_line("sacrifice", 0).unwrap();
    let error = parse_sacrifice_segment_tokens(&missing, |_| false).unwrap_err();
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
            filter: ObjectFilter::artifact().with_subtype(crate::types::Subtype::Equipment),
        }
    );

    let tap = lex_line("tap two other untapped creatures you control", 0).unwrap();
    assert_eq!(
        parse_tap_chosen_segment_tokens(&tap).unwrap(),
        ActivationCostSegmentCst::TapChosen {
            count: 2,
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

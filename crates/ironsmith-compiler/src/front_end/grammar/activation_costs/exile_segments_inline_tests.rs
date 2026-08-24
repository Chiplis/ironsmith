use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn exile_segments_preserve_zone_top_and_named_shapes() {
    let top = lex_line("exile the top three cards of your library", 0).unwrap();
    assert_eq!(
        parse_exile_segment_tokens(&top, |_| false).unwrap(),
        ActivationCostSegmentCst::ExileTopLibrary { count: 3 }
    );
    let hand = lex_line("exile a red card from your hand", 0).unwrap();
    assert_eq!(
        parse_exile_segment_tokens(&hand, |_| false).unwrap(),
        ActivationCostSegmentCst::ExileFromHand {
            count: 1,
            color_filter: Some(ColorSet::RED),
        }
    );
    let named = lex_line(
        "exile this card and artifacts you control named foo and bar",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_exile_segment_tokens(&named, |_| false).unwrap(),
        ActivationCostSegmentCst::ExileSelfAndNamedArtifacts {
            names: vec!["foo".to_string(), "bar".to_string()],
        }
    );

    let top_creature = lex_line("exile the top creature card of your graveyard", 0).unwrap();
    assert!(matches!(
        parse_exile_segment_tokens(&top_creature, |_| false).unwrap(),
        ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter,
            top_only: true,
            turn_face_up: false,
        } if choice_count == crate::effect::ChoiceCount::exactly(1)
            && filter.zone == Some(Zone::Graveyard)
            && filter.card_types == [crate::types::CardType::Creature]
    ));

    let face_up = lex_line("exile a face-down permanent you control face up", 0).unwrap();
    assert!(matches!(
        parse_exile_segment_tokens(&face_up, |_| false).unwrap(),
        ActivationCostSegmentCst::ExileChosen {
            choice_count,
            filter,
            top_only: false,
            turn_face_up: true,
        } if choice_count == crate::effect::ChoiceCount::exactly(1)
            && filter.face_down == Some(true)
            && filter.controller == Some(PlayerFilter::You)
    ));

    let compound = lex_line(
        "exile this Vehicle and four other artifact creatures and/or Vehicles you control",
        0,
    )
    .unwrap();
    let parsed_compound = parse_exile_segment_tokens(&compound, |_| false).unwrap();
    let parsed_compound_debug = format!("{parsed_compound:#?}");
    assert!(
        matches!(
            parsed_compound,
            ActivationCostSegmentCst::ExileSourceAndChosen {
                source_filter,
                choice_count,
                filter,
            } if source_filter.source
                && matches!(
                    source_filter.source_surface,
                    Some(crate::target::SourceReferenceSurface::ThisPermanentType(ref text))
                        if text == "this Vehicle"
                )
                && choice_count == crate::effect::ChoiceCount::exactly(4)
                && filter.other
                && filter.controller == Some(PlayerFilter::You)
                && filter.any_of.iter().any(|arm|
                    arm.card_types.contains(&crate::types::CardType::Artifact)
                        && arm.card_types.contains(&crate::types::CardType::Creature)
                )
                && filter.any_of.iter().any(|arm|
                    arm.subtypes.contains(&crate::types::Subtype::Vehicle)
                )
        ),
        "{parsed_compound_debug}"
    );

    let ordinary_face_up = lex_line("exile a face-up permanent you control", 0).unwrap();
    assert!(matches!(
        parse_exile_segment_tokens(&ordinary_face_up, |_| false).unwrap(),
        ActivationCostSegmentCst::ExileChosen {
            filter,
            turn_face_up: false,
            ..
        } if filter.face_down == Some(false)
    ));

    let ordinary = lex_line("exile a creature card from your graveyard", 0).unwrap();
    assert!(matches!(
        parse_exile_segment_tokens(&ordinary, |_| false).unwrap(),
        ActivationCostSegmentCst::ExileChosen {
            top_only: false,
            ..
        }
    ));
}

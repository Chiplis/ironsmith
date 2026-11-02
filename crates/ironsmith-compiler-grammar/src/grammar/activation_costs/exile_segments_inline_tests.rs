use super::super::super::super::lexer::lex_line;
use super::*;

#[test]
fn hand_or_graveyard_exile_is_one_exact_choice_across_both_owned_zones() {
    for text in ["exile exactly two cards from your hand and/or graveyard", "exile two blue cards from your hand and/or graveyard"] {
        let tokens = lex_line(text, 0).unwrap();
        let ActivationCostSegmentCst::ExileChosen { choice_count, filter, .. } = parse_exile_segment_tokens(&tokens, |_| false).unwrap() else { panic!("missing union choice"); };
        assert_eq!(choice_count, crate::effect::ChoiceCount::exactly(2));
        assert_eq!(filter.any_of.len(), 2);
        for (arm, zone) in filter.any_of.iter().zip([Zone::Hand, Zone::Graveyard]) {
            assert_eq!(arm.zone, Some(zone));
            assert_eq!(arm.owner, Some(PlayerFilter::You));
            if text.contains("blue") { assert_eq!(arm.colors, Some(ColorSet::BLUE)); }
        }
    }
}

#[test]
fn repeated_exile_price_retains_the_multiplier_on_announced_x() {
    let tokens = lex_line("exile exactly two times X cards from your hand and/or graveyard", 0).unwrap();
    let ActivationCostSegmentCst::ExileChosen { count_value, filter, .. } = parse_exile_segment_tokens(&tokens, |_| false).unwrap() else { panic!("missing scaled exile selection"); };
    assert_eq!(count_value, Some(crate::effect::Value::XTimes(2)));
    assert_eq!(filter.any_of.len(), 2);
}

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
            count_value: None,
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
            count_value: None,
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

#[test]
fn coordinated_exile_cost_shares_origin_and_preserves_literal_names() {
    for (zone, zone_word) in [(Zone::Graveyard, "graveyard"), (Zone::Hand, "hand")] {
        let tokens = lex_line(
            &format!(
                "Exile this card and two other cards named Say Its Name from your {zone_word}"
            ),
            0,
        )
        .unwrap();
        let ActivationCostSegmentCst::ExileSourceAndChosen {
            source_filter,
            choice_count,
            filter,
        } = parse_exile_segment_tokens(&tokens, |_| false).unwrap()
        else {
            panic!("expected compound exile cost")
        };
        assert_eq!(source_filter.zone, Some(zone));
        assert!(source_filter.source);
        assert_eq!(filter.zone, Some(zone));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
        assert_eq!(filter.name.as_deref(), Some("say its name"));
        assert!(filter.other);
        assert_eq!(choice_count, crate::effect::ChoiceCount::exactly(2));
    }
}

#[test]
fn exile_cost_count_tracks_opponents_instead_of_defaulting_to_one() {
    let tokens = lex_line("Exile cards from your graveyard equal to the number of opponents you have", 0).unwrap();
    let cost = parse_exile_segment_tokens(&tokens, |_| false).unwrap();
    assert!(matches!(cost, ActivationCostSegmentCst::ExileChosen {
        count_value: Some(crate::effect::Value::CountPlayers(PlayerFilter::Opponent)),
        filter, ..
    } if filter.zone == Some(Zone::Graveyard) && filter.owner == Some(PlayerFilter::You)));
}

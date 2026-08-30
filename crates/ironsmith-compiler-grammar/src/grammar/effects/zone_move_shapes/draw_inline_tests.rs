use super::*;
use crate::lexer::lex_line;

fn tokens(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).unwrap()
}

#[test]
fn parses_draw_heads_and_counts() {
    let head_tokens = tokens("that many cards minus one");
    let head = parse_draw_head_shape(&head_tokens).unwrap();
    assert_eq!(
        head.count,
        DrawHeadCountShape::Resolved(Value::EventValueOffset(EventValueSpec::Amount, -1))
    );
    assert_eq!(head.parsed_offset, Some(DrawCardCountOffset::MinusOne));
    assert_eq!(
        parse_draw_known_count_shape(&tokens("times this spell was kicked")),
        Some(DrawKnownCountShape::KickCount)
    );
}

#[test]
fn preserves_filtered_prior_action_counts() {
    for (text, expected_filter) in [
        (
            "creature card put into their graveyard this way",
            crate::target::ObjectFilter::creature()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::IteratedPlayer),
        ),
        (
            "land card put into their graveyard this way",
            crate::target::ObjectFilter::land()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::IteratedPlayer),
        ),
    ] {
        let parsed = parse_draw_this_way_metric_shape(&tokens(text));
        let expected = Value::Count(expected_filter.match_tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            TaggedOpbjectRelation::IsTaggedObject,
        ));
        assert_eq!(parsed, Some(expected), "{text}");
    }
}

#[test]
fn parses_equal_to_named_counters_removed_this_way_as_typed_metric() {
    let parsed = parse_draw_equal_this_way_metric_shape(&tokens(
        "equal to the number of stun counters removed this way",
    ))
    .expect("typed removed-counter metric");
    assert!(
        parsed.has_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay,),
        "{parsed:#?}"
    );
    let Value::PendingPriorEffectMetric(query) = parsed.unhinted() else {
        panic!("expected pending prior-effect metric, got {parsed:#?}");
    };
    assert_eq!(
        query.action,
        Some(ironsmith_core::PriorEffectAction::Removed)
    );
    assert_eq!(query.counter_type, Some(crate::object::CounterType::Stun));
}

#[test]
fn preserves_opponents_dealt_damage_this_way_count_surface() {
    let parsed = parse_draw_equal_this_way_metric_shape(&tokens(
        "equal to the number of opponents dealt damage this way",
    ))
    .expect("grouped damaged-opponent count");

    assert!(
        parsed.has_surface_hint(ironsmith_core::ValueSurfaceHint::OpponentsDealtDamageThisWay,),
        "{parsed:#?}"
    );
    assert!(matches!(
        parsed.unhinted(),
        Value::PendingEffectMetric {
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::Count,
        }
    ));
}

#[test]
fn parses_article_draw_head() {
    let draw_tokens = tokens("a card.");
    let head = parse_draw_head_shape(&draw_tokens).expect("article draw head");
    assert_eq!(head.count, DrawHeadCountShape::Resolved(Value::Fixed(1)));
    assert!(!head.additional);
    assert!(head.tail_tokens.is_empty());

    let additional_tokens = tokens("an additional card.");
    let additional =
        parse_draw_head_shape(&additional_tokens).expect("additional article draw head");
    assert_eq!(
        additional.count,
        DrawHeadCountShape::Resolved(Value::Fixed(1))
    );
    assert!(additional.additional);
    assert!(additional.tail_tokens.is_empty());

    let two_tokens = tokens("two additional cards.");
    let two = parse_draw_head_shape(&two_tokens).expect("additional counted draw head");
    assert_eq!(two.count, DrawHeadCountShape::Resolved(Value::Fixed(2)));
    assert!(two.additional);
    assert!(two.tail_tokens.is_empty());
}

#[test]
fn parses_draw_equal_and_counter_references() {
    assert!(matches!(
        parse_draw_equal_shape(&tokens("equal to the power of target creature")),
        Some(DrawEqualShape::StatOfTarget {
            stat: DrawEqualStat::Power,
            ..
        })
    ));
    assert!(matches!(
        parse_draw_counter_reference_shape(&tokens("charge counters on this artifact")),
        Some(Value::CountersOnSource(_))
    ));
}

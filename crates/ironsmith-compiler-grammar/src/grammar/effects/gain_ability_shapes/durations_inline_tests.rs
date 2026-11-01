use super::*;
use crate::cards::builders::SourcePredicateAst;
use crate::lexer::lex_line;

#[test]
fn parses_turn_conditional_and_quoted_durations() {
    let combat_duration =
        parse_simple_ability_duration_shape(&["flying", "until", "end", "of", "combat"]).unwrap();
    assert_eq!(combat_duration.start, 1);
    assert_eq!(combat_duration.len, 4);
    assert_eq!(combat_duration.duration, Until::EndOfCombat);

    let duration =
        parse_simple_ability_duration_shape(&["flying", "until", "your", "next", "upkeep"])
            .unwrap();
    assert_eq!(duration.start, 1);
    assert_eq!(duration.len, 4);
    assert_eq!(duration.duration, Until::YourNextUpkeep);

    let source_lifetime = parse_simple_ability_duration_shape(&[
        "all",
        "abilities",
        "for",
        "as",
        "long",
        "as",
        "this",
        "creature",
        "remains",
        "on",
        "the",
        "battlefield",
    ])
    .unwrap();
    assert_eq!(source_lifetime.start, 2);
    assert_eq!(source_lifetime.duration, Until::ForAsLongAs(
        ironsmith_core::ContinuousDurationPredicate::ObjectOnBattlefield(
            ironsmith_core::ContinuousDurationObject::Source,
        ),
    ));

    let tapped_tokens = lex_line("flying for as long as this creature remains tapped.", 0).unwrap();
    let tapped = parse_source_tapped_gain_duration_shape(&tapped_tokens).unwrap();
    assert_eq!(tapped.start, 1);
    assert_eq!(tapped.duration, Until::SourceUntaps);
    assert_eq!(
        tapped.condition,
        Some(PredicateAst::Source(SourcePredicateAst::SourceIsTapped))
    );

    let near_miss = lex_line("flying for as long as this creature remains untapped.", 0).unwrap();
    assert!(parse_source_tapped_gain_duration_shape(&near_miss).is_none());

    let tokens = lex_line(
        "Target creature gains \"Whenever it attacks, draw a card.\" until end of turn.",
        0,
    )
    .unwrap();
    let gain_token = primitives::find_prefix(&tokens, || primitives::kw("gains"))
        .map(|(offset, _, _)| offset)
        .unwrap();
    assert_eq!(
        parse_quoted_gain_duration_shape(&tokens, gain_token)
            .unwrap()
            .duration,
        Until::EndOfTurn
    );
}

#[test]
fn parses_earlier_of_end_of_turn_and_any_player_roll_result() {
    let duration = parse_simple_ability_duration_shape(&[
        "flying",
        "until",
        "end",
        "of",
        "turn",
        "or",
        "until",
        "any",
        "player",
        "rolls",
        "a",
        "1",
        "whichever",
        "comes",
        "first",
    ])
    .expect("compound roll-linked duration");

    assert_eq!(duration.start, 1);
    assert_eq!(duration.len, 14);
    assert_eq!(
        duration.duration,
        Until::EndOfTurnOrAnyPlayerRolls {
            result: 1,
            matching_rolls_observed: 0,
        }
    );

    assert!(
        parse_simple_ability_duration_shape(&[
            "flying", "until", "end", "of", "turn", "or", "until", "any", "player", "rolls", "a",
            "1",
        ])
        .is_some_and(|shape| shape.duration == Until::EndOfTurn),
        "an incomplete compound suffix must retain the ordinary end-of-turn duration"
    );
}

#[test]
fn parses_leading_affected_object_counter_duration_before_real_grant_verb() {
    let tokens = lex_line(
            "For as long as that creature has a bounty counter on it, it has \"When this creature dies, draw a card.\"",
            0,
        )
        .unwrap();
    let shape = parse_leading_affected_object_counter_duration_shape(&tokens)
        .expect("counter-linked grant duration should parse");

    assert_eq!(shape.consumed_words, 12);
    assert_eq!(
        shape.duration,
        Until::ForAsLongAs(
            ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                crate::object::CounterType::Bounty
            )
        )
    );

    let normalized_without_comma = lex_line(
            "For as long as that land has a blaze counter on it it has \"At the beginning of your upkeep, this land deals 1 damage to you.\"",
            0,
        )
        .unwrap();
    assert!(
        parse_leading_affected_object_counter_duration_shape(&normalized_without_comma).is_some(),
        "multi-sentence normalization may omit the duration comma"
    );
}

#[test]
fn snc_quoted_mana_grant_has_exact_source_cast_duration() {
    let tokens = lex_line("Target land gains \"{T}: Add {U}, {B}, or {R}\" until this card is cast from exile.", 0).unwrap();
    let parsed = parse_quoted_gain_duration_shape(&tokens, 2).unwrap();
    assert!(matches!(parsed.duration, Until::ObjectIsCast {
        object: ironsmith_core::ContinuousDurationObject::Tagged(ref tag),
        from_zone: crate::zone::Zone::Exile,
    } if tag.as_str() == ironsmith_core::tag::SOURCE_EXILED_SELF_TAG));
    assert!(parse_simple_ability_duration_shape(&["until", "that", "card", "is", "cast", "from", "exile"]).is_none(),
        "an unrelated object must not silently become this source");
}

#[test]
fn quoted_trigger_boundary_uses_existing_durations_without_rebinding_source() {
    for suffix in ["until end of turn", "for as long as this creature remains on the battlefield"] {
        let text=format!("Target creature gains \"Whenever this creature attacks, you gain 1 life.\" {suffix}.");
        let tokens=lex_line(&text,0).unwrap();
        let parsed=parse_quoted_gain_duration_shape(&tokens,2).unwrap();
        assert_eq!(tokens[parsed.close_quote_token].kind,crate::lexer::TokenKind::Quote);
        assert!(tokens[..parsed.close_quote_token].iter().any(|token| token.is_word("gain")));
        if suffix.starts_with("for as") {
            assert_eq!(parsed.duration,Until::while_source_remains_on_battlefield(),"existing exact source duration must keep its prior identity");
        }
    }
    for suffix in ["until end of turn {R}", "until end of turn and draw a card"] {
        let text=format!("Target creature gains \"Whenever this creature attacks, you gain 1 life.\" {suffix}.");
        assert!(parse_quoted_gain_duration_shape(&lex_line(&text,0).unwrap(),2).is_none());
    }
}

#[test]
fn quoted_duration_atoms_remain_typed_references_until_contextual_binding() {
    for suffix in ["for as long as this card remains exiled","for as long as that creature remains on the battlefield","for as long as this card remains exiled and that creature remains on the battlefield","for as long as that creature remains on the battlefield and this card remains exiled"] {
        let text=format!("Target artifact creature gains \"Flying\" {suffix}.");
        let parsed=parse_quoted_gain_duration_shape(&lex_line(&text,0).unwrap(),3).unwrap();
        let debug=format!("{:?}",parsed.duration);
        if suffix.contains("this card") {assert!(debug.contains("SourceInZone(Exile)"));}
        if suffix.contains("that creature") {assert!(debug.contains(crate::tag::CompilerReferenceTag::DurationReferencedCreature.as_str()));assert!(!debug.contains("AffectedObject"));}
    }
    for suffix in ["for as long as that card remains exiled","for as long as this card remains exiled {R}","for as long as that creature remains on the battlefield and draw a card"] {
        let text=format!("Target creature gains \"Flying\" {suffix}.");assert!(parse_quoted_gain_duration_shape(&lex_line(&text,0).unwrap(),2).is_none());
    }
}

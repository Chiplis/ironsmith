use super::*;

#[test]
fn independent_effect_sentences_keep_distinct_source_groups() {
    let tokens = lex_line(
        "Put a +1/+1 counter on this creature. Each opponent loses 1 life.",
        0,
    )
    .expect("independent trigger body should lex");
    let parsed = parse_effect_sentences_preserving_source_boundaries(&tokens)
        .expect("independent trigger body should preserve source boundaries");
    assert!(
        matches!(
            parsed.as_slice(),
            [
                EffectAst::SourceSentence { .. },
                EffectAst::SourceSentence { .. }
            ]
        ),
        "independent Oracle sentences must retain separate typed provenance: {parsed:#?}"
    );
    let normalized = crate::effect_ast_normalization::normalize_effects_ast(&parsed);
    assert!(
        matches!(
            normalized.as_slice(),
            [
                EffectAst::SourceSentence { .. },
                EffectAst::SourceSentence { .. }
            ]
        ),
        "representation normalization must preserve source provenance: {normalized:#?}"
    );
}

#[test]
fn moved_object_followup_keeps_prior_leading_then_boundary() {
    let tokens = lex_line(
            "Draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
            0,
        )
        .expect("linked optional entry procedure should lex");
    let parsed = parse_effect_sentences_preserving_source_boundaries(&tokens)
        .expect("linked optional entry procedure should preserve source boundaries");
    let [
        EffectAst::SourceSentence {
            leading_then: false,
            ..
        },
        EffectAst::SourceSentence {
            effects,
            leading_then: true,
            ..
        },
    ] = parsed.as_slice()
    else {
        panic!("expected draw and linked deployment source groups: {parsed:#?}");
    };
    let [
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects,
        },
    ] = effects.as_slice()
    else {
        panic!("entry follow-up must remain inside the optional procedure: {effects:#?}");
    };
    assert!(matches!(
        effects.as_slice(),
        [
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    battlefield_tapped: true,
                    battlefield_attacking: true,
                    ..
                },
                ..
            }),
            EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Tagged(tag, _),
                    duration: Until::EndOfTurn,
                    ..
                },
                ..
            }),
        ] if tag.as_str() == IT_TAG
    ));
}

#[test]
fn flat_fallback_keeps_leading_then_on_the_matching_for_each_filter() {
    let tokens = lex_line(
            "Exile up to one target Assassin creature card from your graveyard with a memory counter on it. Then for each creature card you own in exile with a memory counter on it, create a tapped and attacking token that's a copy of it. Exile those tokens at end of combat.",
            0,
        )
        .expect("multi-sentence effect body should lex");
    let sentences = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|sentence| sentence.to_vec())
        .collect::<Vec<_>>();
    let parsed = parse_effect_sentences_lexed(&tokens)
        .expect("multi-sentence effect body should parse flat");
    let surfaced = preserve_flat_leading_then_for_each_surface(&sentences, parsed);
    let filter = first_for_each_object_filter(&surfaced)
        .expect("the exiled memory-card iterator should survive the flat parse");

    assert!(filter.has_for_each_leading_then_surface());
    assert_eq!(filter.zone, Some(Zone::Exile));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert_eq!(
        filter.with_counter,
        Some(crate::filter::CounterConstraint::Typed(
            crate::object::CounterType::Named("memory".into())
        ))
    );
}

#[test]
fn ordinary_for_each_sentence_does_not_gain_leading_then_surface() {
    let tokens = lex_line(
            "For each creature card you own in exile with a memory counter on it, create a token that's a copy of it.",
            0,
        )
        .expect("ordinary for-each effect should lex");
    let sentences = split_lexed_sentences(&tokens)
        .into_iter()
        .map(|sentence| sentence.to_vec())
        .collect::<Vec<_>>();
    let parsed =
        parse_effect_sentences_lexed(&tokens).expect("ordinary for-each effect should parse");
    let surfaced = preserve_flat_leading_then_for_each_surface(&sentences, parsed);
    let filter = first_for_each_object_filter(&surfaced)
        .expect("ordinary for-each iterator should remain typed");

    assert!(!filter.has_for_each_leading_then_surface());
}

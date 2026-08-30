use super::*;

/// Recognize prior-result clauses whose verbs are not object-filter
/// predicates in the general condition grammar (active reveal/cast/search,
/// "put into exile", damage prevention, and counter removal).
pub(super) fn parse_direct_prior_effect_result_surface(
    tokens: &[OwnedLexToken],
) -> Option<PriorEffectResultSurface> {
    if counted_shared_characteristic(tokens).is_some() {
        // Cross-object cardinality and pairwise characteristic sharing belong
        // to the typed predicate grammar. The direct action/filter grammar
        // cannot represent either fact and therefore is not a candidate.
        return None;
    }
    let words = normalized_word_tokens(tokens);
    let normalized_words = words
        .iter()
        .map(OwnedLexToken::parser_text)
        .collect::<Vec<_>>();
    if normalized_words.len() < 3
        || !crate::word_primitives::parse_sequence_suffix(&normalized_words, &["this", "way"])
    {
        return None;
    }
    let one_or_more =
        crate::word_primitives::parse_sequence_prefix(&normalized_words, &["one", "or", "more"]);
    let ordinary_quantifier = if one_or_more {
        PriorEffectResultQuantifier::OneOrMore
    } else {
        PriorEffectResultQuantifier::One
    };

    if crate::word_primitives::parse_any_sequence_complete(
        &normalized_words,
        &[
            &["it", "connives", "this", "way"],
            &["it", "connive", "this", "way"],
        ],
    ) {
        return Some(PriorEffectResultSurface::new(
            PriorEffectAction::Connived,
            crate::target::ObjectFilter::default(),
            PriorEffectResultActor::It,
            PriorEffectResultQuantifier::ActionOnly,
        ));
    }

    if normalized_words.first() == Some(&"you") {
        let (action, verb_len, action_only) = match normalized_words.get(1).copied()? {
            "cast" => (PriorEffectAction::Cast, 1, false),
            "discard" | "discarded" => (PriorEffectAction::Discarded, 1, false),
            "exile" | "exiled" => (PriorEffectAction::Exiled, 1, false),
            "mill" | "milled" => (PriorEffectAction::Milled, 1, false),
            "reveal" | "revealed" => (PriorEffectAction::Revealed, 1, false),
            "sacrifice" | "sacrificed" => (PriorEffectAction::Sacrificed, 1, false),
            "tap" | "tapped" => (PriorEffectAction::Tapped, 1, false),
            "search" | "searched" => (PriorEffectAction::Searched, 1, true),
            _ => return None,
        };
        let filter_tokens = if action_only {
            &tokens[0..0]
        } else {
            // The result prefix is lexicalized as ordinary words, so the
            // first two raw tokens are the actor and action as well.
            let this_way_idx =
                crate::slice_primitives::select_position(tokens, |token| token.is_word("this"))?;
            &tokens[1 + verb_len..this_way_idx]
        };
        let normalized_filter_tokens = normalized_word_tokens(filter_tokens);
        let filter_words = normalized_filter_tokens
            .iter()
            .map(OwnedLexToken::parser_text)
            .collect::<Vec<_>>();
        let active_one_or_more =
            crate::word_primitives::parse_sequence_prefix(&filter_words, &["one", "or", "more"]);
        let filter = if action_only {
            crate::target::ObjectFilter::default()
        } else {
            parse_prior_result_object_filter(filter_tokens)?
        };
        return Some(PriorEffectResultSurface::new(
            action,
            filter,
            PriorEffectResultActor::You,
            if action_only {
                PriorEffectResultQuantifier::ActionOnly
            } else if active_one_or_more {
                PriorEffectResultQuantifier::OneOrMore
            } else {
                PriorEffectResultQuantifier::One
            },
        ));
    }

    let copula_idx = crate::slice_primitives::select_position(tokens, |token| {
        token
            .as_word()
            .is_some_and(|word| matches!(word, "is" | "are" | "was" | "were"))
    })?;
    let after = tokens[copula_idx + 1..]
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    let action = if crate::word_primitives::parse_sequence_prefix(&after, &["put", "into", "exile"])
    {
        PriorEffectAction::Exiled
    } else if crate::word_primitives::parse_any_sequence_prefix(
        &after,
        &[
            &["put", "onto", "the", "battlefield"],
            &["put", "onto", "battlefield"],
        ],
    ) {
        PriorEffectAction::PutOntoBattlefield
    } else if after.first() == Some(&"removed") {
        PriorEffectAction::Removed
    } else if after.first() == Some(&"prevented") {
        PriorEffectAction::Prevented
    } else if after.first() == Some(&"countered") {
        PriorEffectAction::Countered
    } else if crate::word_primitives::parse_any_sequence_prefix(
        &after,
        &[
            &["returned", "to", "its", "owners", "hand"],
            &["returned", "to", "its", "owner's", "hand"],
            &["returned", "to", "their", "owners", "hands"],
            &["returned", "to", "their", "owners'", "hands"],
        ],
    ) {
        // This is an outcome predicate, not a present-zone characteristic:
        // "that card is returned to its owner's hand this way" must observe
        // whether the preceding return actually moved that exact object.
        PriorEffectAction::Returned
    } else {
        return None;
    };
    let non_object_subject = tokens[..copula_idx].iter().any(|token| {
        token.as_word().is_some_and(|word| {
            matches!(
                word,
                "ability" | "abilities" | "counter" | "counters" | "damage"
            )
        })
    });
    let mut filter = if non_object_subject {
        crate::target::ObjectFilter::default()
    } else {
        parse_prior_result_object_filter(&tokens[..copula_idx]).unwrap_or_default()
    };
    let subject_words = tokens[..copula_idx]
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    if crate::word_primitives::parse_sequence_prefix(&subject_words, &["that", "card"])
        && filter.demonstrative_antecedent_surface().is_none()
    {
        filter.set_demonstrative_antecedent_surface(Some(
            ironsmith_core::DemonstrativeAntecedentSurface::Card,
        ));
    }
    let has_subject = tokens[..copula_idx]
        .iter()
        .any(|token| token.as_word().is_some());
    let action_only = non_object_subject || !has_subject;
    Some(PriorEffectResultSurface::new(
        action,
        filter,
        PriorEffectResultActor::Passive,
        if action_only {
            PriorEffectResultQuantifier::ActionOnly
        } else {
            ordinary_quantifier
        },
    ))
}

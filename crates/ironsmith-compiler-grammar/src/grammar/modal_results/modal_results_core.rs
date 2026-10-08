use super::*;

/// Recognize prior-result clauses whose verbs are not object-filter
/// predicates in the general condition grammar (active reveal/cast/search,
/// "put into exile", damage prevention, and counter removal).
pub(super) fn parse_direct_prior_effect_result_surface(
    tokens: &[OwnedLexToken],
) -> Option<PriorEffectResultSurface> {
    if let Some((count, characteristic)) = counted_shared_characteristic(tokens) {
        // Count and pairwise sharing are collection constraints. Remove only
        // those proven words before parsing the independent card filter.
        let normalized = normalized_word_tokens(tokens);
        let words = normalized.iter().map(OwnedLexToken::parser_text).collect::<Vec<_>>();
        let relative = words.windows(2).position(|pair| matches!(pair, ["that", "share" | "shares"]))?;
        let tail = words.len().checked_sub(4)?;
        if relative <= 1 || tail <= relative + 2 || words[tail + 2..] != ["this", "way"]
            || !matches!(words[tail], "were" | "was" | "are" | "is") { return None; }
        let relation_matches = match characteristic {
            ObjectCharacteristic::Color => words[relative + 2..tail] == ["color"],
            ObjectCharacteristic::CardType => words[relative + 2..tail] == ["card", "type"],
            ObjectCharacteristic::PermanentType => words[relative + 2..tail] == ["permanent", "type"],
            ObjectCharacteristic::Subtype(crate::types::SubtypeFamily::Creature) => words[relative + 2..tail] == ["creature", "type"],
            ObjectCharacteristic::Subtype(crate::types::SubtypeFamily::Land) => words[relative + 2..tail] == ["land", "type"],
            ObjectCharacteristic::ManaValue => words[relative + 2..tail] == ["mana", "value"],
            _ => false,
        };
        if !relation_matches { return None; }
        let (action, action_start) = crate::grammar::shared_util::value_helper_shapes::parse_prior_effect_action(&words[tail..tail + 2])?;
        if action_start != 0 { return None; }
        let mut filter = crate::grammar::primitives::probe_shape(
            super::super::filters::parse_object_filter_with_grammar_entrypoint_lexed(&normalized[1..relative], false),
        )?;
        filter.zone = None;
        filter.set_prior_effect_action_surface(None);
        return Some(PriorEffectResultSurface::new(action, filter,
            PriorEffectResultActor::Passive, PriorEffectResultQuantifier::OneOrMore)
            .with_count_sharing(count, characteristic));
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
    // Keep this complete destination phrase separate from a generic "did".
    // Revealing a chosen card and moving the remainder can both succeed even
    // when the selected card's hand move was prevented or redirected.
    if let Some(mut tail) = normalized_words.strip_prefix(&["you"]) {
        let negated = if let Some(rest) = tail.strip_prefix(&["did", "not"]) {
            tail = rest;
            true
        } else if tail
            .first()
            .is_some_and(|word| matches!(*word, "didnt" | "didn't"))
        {
            tail = &tail[1..];
            true
        } else {
            false
        };
        if tail == ["put", "card", "into", "your", "hand", "this", "way"] {
            let mut surface = PriorEffectResultSurface::new(
                PriorEffectAction::PutIntoHand,
                crate::target::ObjectFilter::default(),
                PriorEffectResultActor::You,
                PriorEffectResultQuantifier::One,
            );
            surface.negated = negated;
            return Some(surface);
        }
    }
    // Qualified death results retain the creature's characteristics at the
    // actual battlefield departure, rather than testing the graveyard card.
    if let Some(dies) = tokens
        .iter()
        .position(|token| token.is_any_word(&["dies", "died"]))
    {
        let tail = tokens[dies + 1..]
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        let subject = &tokens[..dies];
        if tail == ["this", "way"]
            && !subject
                .first()
                .is_some_and(|token| token.is_any_word(&["that", "it"]))
            && let Some(mut filter) = parse_prior_result_object_filter(subject)
        {
            if !filter
                .card_types
                .contains(&crate::types::CardType::Creature)
            {
                return None;
            }
            filter.zone = None;
            return Some(PriorEffectResultSurface::new(
                PriorEffectAction::Died,
                filter,
                PriorEffectResultActor::Passive,
                PriorEffectResultQuantifier::One,
            ));
        }
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
    } else if crate::word_primitives::parse_any_sequence_prefix(
        &after,
        &[
            &["put", "into", "a", "graveyard"],
            &["put", "into", "the", "graveyard"],
            &["put", "into", "graveyard"],
        ],
    ) {
        PriorEffectAction::PutIntoGraveyard
    } else if after == ["dealt", "damage", "this", "way"] {
        // Damage result conditions examine only recipients of actual damage.
        // Player recipients have their own result predicate below.
        parse_prior_result_object_filter(&tokens[..copula_idx])?;
        PriorEffectAction::DealtDamage
    } else if after == ["moved", "this", "way"] {
        let subject = crate::grammar::primitives::strip_lexed_prefix_phrase(
            &tokens[..copula_idx], &["one", "or", "more"])?;
        let kind_tokens = crate::grammar::primitives::strip_lexed_suffix_phrase(subject, &["counters"])?;
        PriorEffectAction::CountersMoved(crate::util::parse_counter_type_from_tokens(kind_tokens)?)
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
    let mut surface = PriorEffectResultSurface::new(
        action,
        filter,
        PriorEffectResultActor::Passive,
        if action_only {
            PriorEffectResultQuantifier::ActionOnly
        } else {
            ordinary_quantifier
        },
    );
    surface.put_into_exile_surface =
        crate::word_primitives::parse_sequence_prefix(&after, &["put", "into", "exile"])
            && tokens[copula_idx].is_any_word(&["is", "are"]);
    Some(surface)
}

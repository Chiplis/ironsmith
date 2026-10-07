use super::*;

/// This cross-zone phrase is a complete typed union, even when its caller has
/// already consumed `target`. Do not send a malformed instance to the tolerant
/// relational noun scanner after the complete owner rejects it.
pub(crate) fn parse_complete_permanent_or_suspended_card_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<Result<ObjectFilter, CardTextError>> {
    if tokens.first().is_some_and(|token| token.is_word("all")) {
        return parse_complete_permanent_or_suspended_card_filter(&tokens[1..], other)
            .map(|result| result.map(|mut filter| {
                filter.set_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::All));
                filter
            }));
    }
    let words = parser_token_word_refs(tokens);
    // Only the noun phrase before the first targeting relation can establish
    // this owner. A relation enclosing the union remains with its own reader,
    // but a union followed by a relation must consume or reject that full tail.
    let noun_end = words.windows(2)
        .position(|part| matches!(part, ["that", "target" | "targets"]))
        .unwrap_or(words.len());
    let noun_words = &words[..noun_end];
    let head = noun_words.iter().copied().skip_while(|word| {
        is_article(word) || matches!(*word, "each" | "other" | "another" | "target" | "targets" | "nonland")
    }).next();
    // A nested union belongs to its enclosing relation first, for example a
    // `spell that targets a permanent or suspended card`.
    if !matches!(head, Some("permanent" | "permanents" | "suspended"))
        || !noun_words.iter().any(|word| matches!(*word, "permanent" | "permanents"))
        || !noun_words.contains(&"suspended")
        || !noun_words.iter().any(|word| matches!(*word, "or" | "and" | "and/or"))
    {
        return None;
    }
    // Word projection must not erase punctuation or empty extra arms. The
    // established owner consumes the complete qualifiers of both noun arms.
    let complete_tokens = tokens.iter().all(|token| token.as_word().is_some())
        && words.iter().filter(|word| matches!(**word, "or" | "and" | "and/or")).count() == 1;
    let parsed = if complete_tokens {
        parse_permanent_or_suspended_card_disjunction(tokens)
    } else {
        None
    };
    Some(parsed.map(|mut filter| {
        filter.other = other;
        filter
    }).ok_or_else(|| CardTextError::ParseError(format!(
        "unsupported complete permanent-or-suspended filter: {}",
        crate::lexer::render_token_slice(tokens)
    ))))
}

pub(super) fn strip_other_than_basic_land_cards_clause(
    all_words: &mut Vec<&str>,
    segment_tokens: &mut Vec<OwnedLexToken>,
) -> bool {
    let mut idx = 0usize;
    while idx + 3 < all_words.len() {
        if parse_phrase_at_head(&all_words[idx..], OTHER_THAN_BASIC_LAND_PREFIX).is_none() {
            idx += 1;
            continue;
        }

        let mut end = idx + 4;
        if all_words
            .get(end)
            .is_some_and(|word| parse_word_choice(word, CARD_OR_CARDS_WORDS).is_some())
        {
            end += 1;
        }
        all_words.drain(idx..end);
        strip_other_than_basic_land_cards_tokens(segment_tokens);
        return true;
    }

    false
}

pub(super) fn strip_other_than_basic_land_cards_tokens(segment_tokens: &mut Vec<OwnedLexToken>) {
    let mut idx = 0usize;
    while idx + 3 < segment_tokens.len() {
        let word_at = |offset: usize| segment_tokens.get(offset).and_then(OwnedLexToken::as_word);
        if word_at(idx) != Some("other") || word_at(idx + 1) != Some("than") {
            idx += 1;
            continue;
        }

        let mut end = idx + 2;
        if word_at(end).is_some_and(is_article) {
            end += 1;
        }
        if word_at(end) != Some("basic") || word_at(end + 1) != Some("land") {
            idx += 1;
            continue;
        }
        end += 2;
        if word_at(end).is_some_and(|word| parse_word_choice(word, CARD_OR_CARDS_WORDS).is_some()) {
            end += 1;
        }
        segment_tokens.drain(idx..end);
        return;
    }
}

pub(super) fn parse_permanent_or_suspended_card_disjunction(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let or_segments = split_lexed_slices_on_or(tokens);
    let segments = if or_segments.len() == 2 {
        or_segments
    } else {
        // Count expressions commonly coordinate the two disjoint domains
        // additively: "each suspended card ... and each other permanent ...".
        // They still need inclusive `any_of` semantics; flattening them gives
        // the exile arm battlefield/controller constraints from the permanent
        // arm.
        primitives::split_lexed_slices_on_list_conjunction(tokens)
    };
    if segments.len() != 2 {
        return None;
    }

    let (left_kind, left_filter) = parse_permanent_or_suspended_card_arm(segments[0])?;
    let (right_kind, right_filter) = parse_permanent_or_suspended_card_arm(segments[1])?;
    if left_kind == right_kind {
        return None;
    }

    Some(ObjectFilter {
        any_of: vec![left_filter, right_filter],
        ..ObjectFilter::default()
    })
}

pub(super) fn parse_permanent_or_suspended_card_arm(
    tokens: &[OwnedLexToken],
) -> Option<(PermanentOrSuspendedCardArm, ObjectFilter)> {
    let words = non_article_parser_word_refs(tokens);
    let mut words = words.as_slice();
    if words.first().is_some_and(|word| *word == "each") {
        words = &words[1..];
    }
    let arm_other = words
        .first()
        .is_some_and(|word| matches!(*word, "other" | "another"));
    if arm_other {
        words = &words[1..];
    }
    let words = if words
        .first()
        .is_some_and(|word| parse_word_choice(word, TARGET_OR_TARGETS_WORDS).is_some())
    {
        &words[1..]
    } else {
        words
    };

    let leading_nonland = words.first() == Some(&"nonland");
    let noun_words = if leading_nonland { &words[1..] } else { words };

    match noun_words.first().copied() {
        Some("permanent" | "permanents") => {
            let mut filter = ObjectFilter::permanent();
            if leading_nonland {
                filter.excluded_card_types.push(CardType::Land);
            }
            filter.other = arm_other;
            consume_permanent_or_suspended_card_tail(noun_words, 1, &mut filter, true, true)?;
            Some((PermanentOrSuspendedCardArm::Permanent, filter))
        }
        Some("suspended") if !leading_nonland => {
            let card_word = noun_words.get(1).copied()?;
            if !matches!(card_word, "card" | "cards") {
                return None;
            }
            let mut filter = ObjectFilter::default()
                .in_zone(Zone::Exile)
                .with_alternative_cast(crate::filter::AlternativeCastKind::Suspend)
                .with_counter_type(crate::object::CounterType::Time);
            filter.other = arm_other;
            consume_permanent_or_suspended_card_tail(noun_words, 2, &mut filter, false, true)?;
            Some((PermanentOrSuspendedCardArm::SuspendedCard, filter))
        }
        _ => None,
    }
}

pub(super) fn consume_permanent_or_suspended_card_tail(
    words: &[&str],
    mut idx: usize,
    filter: &mut ObjectFilter,
    allow_controller: bool,
    allow_owner: bool,
) -> Option<()> {
    while idx < words.len() {
        if allow_controller
            && words.get(idx) == Some(&"you")
            && words.get(idx + 1) == Some(&"control")
        {
            filter.controller = Some(PlayerFilter::You);
            idx += 2;
            continue;
        }
        if allow_owner && words.get(idx) == Some(&"you") && words.get(idx + 1) == Some(&"own") {
            filter.owner = Some(PlayerFilter::You);
            idx += 2;
            continue;
        }
        if words.get(idx) == Some(&"with")
            && words.get(idx + 1) == Some(&"time")
            && words
                .get(idx + 2)
                .is_some_and(|word| matches!(*word, "counter" | "counters"))
        {
            filter.with_counter = Some(crate::filter::CounterConstraint::Typed(
                crate::object::CounterType::Time,
            ));
            idx += 3;
            if words.get(idx) == Some(&"on")
                && words
                    .get(idx + 1)
                    .is_some_and(|word| matches!(*word, "it" | "them"))
            {
                idx += 2;
            }
            continue;
        }
        return None;
    }
    Some(())
}

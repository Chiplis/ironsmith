use super::*;

pub(crate) fn parse_discard_trigger_card_filter(
    after_discard_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let remainder = trim_commas(after_discard_tokens);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let remainder_words = crate::cards::builders::compiler::token_word_refs(&remainder);
    let Some(card_word_idx) =
        find_index(&remainder_words, |word| *word == "card" || *word == "cards")
    else {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card keyword (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let qualifier_end =
        token_index_for_word_index(&remainder, card_word_idx).unwrap_or(remainder.len());
    let qualifier_tokens = trim_commas(&remainder[..qualifier_end]);
    let mut qualifier_tokens = strip_leading_articles(&qualifier_tokens);
    if qualifier_tokens.len() >= 2
        && qualifier_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .and_then(parse_cardinal_u32)
            .is_some()
        && qualifier_tokens
            .get(1)
            .is_some_and(|token| token.is_word("or"))
    {
        qualifier_tokens = qualifier_tokens[2..].to_vec();
    } else if qualifier_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .and_then(parse_cardinal_u32)
        .is_some()
    {
        qualifier_tokens = qualifier_tokens[1..].to_vec();
    }

    let trailing_tokens = if card_word_idx + 1 < remainder_words.len() {
        let trailing_start =
            token_index_for_word_index(&remainder, card_word_idx + 1).unwrap_or(remainder.len());
        trim_commas(&remainder[trailing_start..])
    } else {
        Vec::new()
    };
    if !trailing_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing discard trigger clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if qualifier_tokens.is_empty() {
        return Ok(None);
    }

    let qualifier_words = crate::cards::builders::compiler::token_word_refs(&qualifier_tokens);
    if qualifier_words.as_slice() == ["one", "or", "more"] {
        return Ok(None);
    }

    if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
        return Ok(Some(filter));
    }

    let mut fallback = ObjectFilter::default();
    let mut parsed_any = false;
    for word in qualifier_words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(non_type) = parse_non_type(word) {
            if !slice_contains(&fallback.excluded_card_types, &non_type) {
                fallback.excluded_card_types.push(non_type);
            }
            parsed_any = true;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            if !slice_contains(&fallback.card_types, &card_type) {
                fallback.card_types.push(card_type);
            }
            parsed_any = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if parsed_any {
        Ok(Some(fallback))
    } else {
        Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )))
    }
}

pub(crate) fn parse_subtype_list_enters_trigger_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = if words.len() >= 2
        && words[words.len() - 2] == "you"
        && words[words.len() - 1] == "control"
    {
        (Some(PlayerFilter::You), words.len() - 2)
    } else if words.len() >= 2
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 2)
    } else if words.len() >= 3
        && words[words.len() - 3] == "an"
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 3)
    } else {
        (None, words.len())
    };

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    Some(filter)
}

pub(crate) fn parse_possessive_clause_player_filter(words: &[&str]) -> PlayerFilter {
    let attached_controller_filter =
        |tag: &str| PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(tag)));
    let normalized_words = words
        .iter()
        .map(|word| {
            str_strip_suffix(word, "'s")
                .or_else(|| str_strip_suffix(word, "’s"))
                .or_else(|| str_strip_suffix(word, "s'"))
                .or_else(|| str_strip_suffix(word, "s’"))
                .unwrap_or(word)
        })
        .collect::<Vec<_>>();
    let has_attached_controller = |subject: &str| {
        find_window_by(&normalized_words, 3, |window| {
            window[0] == subject
                && matches!(
                    window[1],
                    "creature"
                        | "creatures"
                        | "permanent"
                        | "permanents"
                        | "artifact"
                        | "artifacts"
                        | "enchantment"
                        | "enchantments"
                        | "land"
                        | "lands"
                )
                && window[2] == "controller"
        })
        .is_some()
    };

    if contains_word_sequence(&normalized_words, &["enchanted", "player"])
        || contains_word_sequence(&normalized_words, &["enchanted", "players"])
    {
        return PlayerFilter::TaggedPlayer(TagKey::from("enchanted"));
    }
    if has_attached_controller("enchanted") {
        return attached_controller_filter("enchanted");
    }
    if has_attached_controller("equipped") {
        return attached_controller_filter("equipped");
    }

    // "each player" / "a player" / "that player" should resolve to Any,
    // even if "opponent" appears elsewhere in the clause text.  Check for
    // explicit "each/a/that player" before falling through to the opponent
    // keyword scan.
    let has_each_player = contains_word_sequence(&normalized_words, &["each", "player"]);
    if contains_your_team_words(words) || slice_contains(&words, &"your") {
        PlayerFilter::You
    } else if has_each_player {
        PlayerFilter::Any
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn parse_subject_clause_player_filter(words: &[&str]) -> PlayerFilter {
    if contains_your_team_words(words) || slice_contains(&words, &"you") {
        PlayerFilter::You
    } else if contains_word_sequence(words, &["enchanted", "player"])
        || contains_word_sequence(words, &["enchanted", "players"])
    {
        PlayerFilter::TaggedPlayer(TagKey::from("enchanted"))
    } else if contains_word_sequence(words, &["chosen", "player"])
        || contains_word_sequence(words, &["chosen", "players"])
    {
        PlayerFilter::ChosenPlayer
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn contains_opponent_word(words: &[&str]) -> bool {
    words
        .iter()
        .any(|word| matches!(*word, "opponent" | "opponents"))
}

pub(crate) fn contains_your_team_words(words: &[&str]) -> bool {
    contains_any_word_sequence(words, &[&["your", "team"], &["on", "your", "team"]])
}

pub(crate) fn parse_trigger_subject_player_filter(subject: &[&str]) -> Option<PlayerFilter> {
    if subject == ["you"] {
        return Some(PlayerFilter::You);
    }
    if subject == ["the", "chosen", "player"] || subject == ["chosen", "player"] {
        return Some(PlayerFilter::ChosenPlayer);
    }
    if slice_starts_with(&subject, &["the", "player", "who", "cast"])
        || slice_starts_with(&subject, &["player", "who", "cast"])
    {
        return Some(PlayerFilter::EffectController);
    }
    if subject == ["a", "player"]
        || subject == ["any", "player"]
        || subject == ["player"]
        || subject == ["one", "or", "more", "players"]
    {
        return Some(PlayerFilter::Any);
    }
    if subject == ["an", "opponent"]
        || subject == ["opponent"]
        || subject == ["opponents"]
        || subject == ["your", "opponents"]
        || subject == ["one", "of", "your", "opponents"]
        || subject == ["one", "or", "more", "of", "your", "opponents"]
        || subject == ["one", "of", "the", "opponents"]
        || subject == ["one", "or", "more", "opponents"]
        || subject == ["each", "opponent"]
    {
        return Some(PlayerFilter::Opponent);
    }
    if slice_ends_with(&subject, &["on", "your", "team"])
        && subject
            .iter()
            .any(|word| matches!(*word, "player" | "players"))
    {
        return Some(PlayerFilter::You);
    }
    None
}

pub(crate) fn split_target_clause_before_comma(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let tokens = trim_commas(tokens);
    if let Some(comma_idx) = find_index(&tokens, |token| token.is_comma()) {
        trim_commas(&tokens[..comma_idx])
    } else {
        tokens
    }
}

pub(crate) fn parse_shuffle_trigger_subject(
    subject: &[&str],
) -> Option<(PlayerFilter, bool, bool)> {
    if let Some(player) = parse_trigger_subject_player_filter(subject) {
        return Some((player, false, false));
    }

    if !(slice_starts_with(&subject, &["a", "spell", "or", "ability", "causes"])
        && subject.last().copied() == Some("to")
        && subject.len() > 6)
    {
        return None;
    }

    let caused_player_words = &subject[5..subject.len() - 1];
    if caused_player_words == ["its", "controller"] {
        return Some((PlayerFilter::Any, true, true));
    }

    parse_trigger_subject_player_filter(caused_player_words).map(|player| (player, true, false))
}

pub(crate) fn parse_spell_or_ability_controller_tail(words: &[&str]) -> Option<PlayerFilter> {
    let (prefix_len, controller_end) = match words {
        ["a", "spell", "or", "ability", ..] => (4usize, words.len()),
        ["spell", "or", "ability", ..] => (3usize, words.len()),
        _ => return None,
    };

    if controller_end <= prefix_len + 1 {
        return None;
    }
    if !matches!(words.last().copied(), Some("control") | Some("controls")) {
        return None;
    }

    let controller_words = &words[prefix_len..controller_end - 1];
    parse_trigger_subject_player_filter(controller_words)
}

pub(crate) fn parse_trigger_subject_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = crate::cards::builders::compiler::token_word_refs(subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| matches!(*word, "that" | "which" | "who" | "whom"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    parse_object_filter(subject_tokens, other)
        .map(Some)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                crate::cards::builders::compiler::token_word_refs(subject_tokens).join(" ")
            ))
        })
}

pub(crate) fn trigger_subject_player_selector(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more(subject_tokens);
    let subject_words = crate::cards::builders::compiler::token_word_refs(subject_tokens);
    parse_trigger_subject_player_filter(&subject_words)
}

pub(crate) fn attacking_filter_for_player(player: PlayerFilter) -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    if !matches!(player, PlayerFilter::Any) {
        filter.controller = Some(player);
    }
    filter
}

pub(crate) fn parse_attack_trigger_subject_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    let Some(mut filter) = parse_trigger_subject_filter(subject_tokens)? else {
        return Ok(None);
    };

    // Attack/combat-trigger subjects are creatures by default even when
    // expressed only as a subtype ("a Sliver", "one or more Goblins", etc.).
    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    }

    Ok(Some(filter))
}

pub(crate) fn strip_leading_one_or_more_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let words = ActivationRestrictionCompatWords::new(tokens);
    if words.slice_eq(0, &["one", "or", "more"]) {
        let start = words.token_index_for_word_index(3).unwrap_or(tokens.len());
        &tokens[start..]
    } else {
        tokens
    }
}

pub(crate) fn parse_subtype_list_enters_trigger_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    let words = words.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = if words.len() >= 2
        && words[words.len() - 2] == "you"
        && words[words.len() - 1] == "control"
    {
        (Some(PlayerFilter::You), words.len() - 2)
    } else if words.len() >= 2
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 2)
    } else if words.len() >= 3
        && words[words.len() - 3] == "an"
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 3)
    } else {
        (None, words.len())
    };

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    Some(filter)
}

pub(crate) fn parse_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| matches!(*word, "that" | "which" | "who" | "whom"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    if contains_word_sequence(
        &subject_words,
        &["power", "greater", "than", "its", "base", "power"],
    ) && subject_words
        .iter()
        .any(|word| matches!(*word, "creature" | "creatures"))
    {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.power_greater_than_base_power = true;
        if other {
            filter.other = true;
        }
        if contains_word_sequence(&subject_words, &["you", "control"]) {
            filter.controller = Some(PlayerFilter::You);
        } else if contains_any_word_sequence(
            &subject_words,
            &[&["opponents", "control"], &["opponent", "controls"]],
        ) {
            filter.controller = Some(PlayerFilter::Opponent);
        }
        return Ok(Some(filter));
    }

    let mut normalized_subject_tokens = subject_tokens.to_vec();
    if find_window_by(&normalized_subject_tokens, 2, |window| {
        window[0].is_word("each") && window[1].is_word("with")
    })
    .is_some()
    {
        let mut normalized = Vec::with_capacity(normalized_subject_tokens.len());
        let mut idx = 0usize;
        while idx < normalized_subject_tokens.len() {
            if normalized_subject_tokens[idx].is_word("each")
                && normalized_subject_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("with"))
            {
                idx += 1;
                continue;
            }
            normalized.push(normalized_subject_tokens[idx].clone());
            idx += 1;
        }
        normalized_subject_tokens = normalized;
    }

    let mut controller_override = None;
    let word_view = ActivationRestrictionCompatWords::new(&normalized_subject_tokens);
    let normalized_words = word_view.to_word_refs();
    let controller_phrase = if let Some(idx) =
        find_word_sequence_start(&normalized_words, &["you", "control"])
            .filter(|idx| idx + 2 < normalized_words.len())
    {
        controller_override = Some(PlayerFilter::You);
        Some((idx, 2usize))
    } else if let Some(idx) = find_word_sequence_start(&normalized_words, &["opponents", "control"])
        .filter(|idx| idx + 2 < normalized_words.len())
    {
        controller_override = Some(PlayerFilter::Opponent);
        Some((idx, 2usize))
    } else if let Some(idx) = find_word_sequence_start(&normalized_words, &["opponent", "controls"])
        .filter(|idx| idx + 2 < normalized_words.len())
    {
        controller_override = Some(PlayerFilter::Opponent);
        Some((idx, 2usize))
    } else {
        None
    };

    if let Some((word_idx, len)) = controller_phrase
        && let Some(start) = token_index_for_word_index(&normalized_subject_tokens, word_idx)
        && let Some(end) = token_index_for_word_index(&normalized_subject_tokens, word_idx + len)
    {
        normalized_subject_tokens.drain(start..end);
    }

    parse_object_filter_lexed(&normalized_subject_tokens, other)
        .map(|mut filter| {
            if filter.zone.is_none()
                && filter.tagged_constraints.is_empty()
                && filter.specific.is_none()
                && !filter.source
            {
                filter.zone = Some(Zone::Battlefield);
            }
            if let Some(controller) = controller_override {
                filter.controller = Some(controller);
                filter.zone.get_or_insert(Zone::Battlefield);
            }
            Some(filter)
        })
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                subject_words.join(" ")
            ))
        })
}

pub(crate) fn trigger_subject_player_selector_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    parse_trigger_subject_player_filter(&subject_words)
}

pub(crate) fn parse_attack_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector_lexed(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    let Some(mut filter) = parse_trigger_subject_filter_lexed(subject_tokens)? else {
        return Ok(None);
    };

    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    }

    Ok(Some(filter))
}

pub(crate) fn parse_exact_spell_count_each_turn(words: &[&str]) -> Option<u32> {
    for (ordinal, count) in [
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        if contains_word_sequence(words, &[ordinal, "spell", "cast", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "spell", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "spell", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "spell", "each", "turn"])
        {
            return Some(count);
        }
    }
    None
}

pub(crate) fn parse_exact_draw_count_each_turn(words: &[&str]) -> Option<u32> {
    if contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "their", "draw",
            "step",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "their", "draw",
            "step",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "your", "draw",
            "step",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "your", "draw",
            "step",
        ],
    ) {
        return Some(2);
    }

    for (ordinal, count) in [
        ("second", 2u32),
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        if contains_word_sequence(words, &[ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &[ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &[ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "cards", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "cards", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "cards", "this", "turn"])
        {
            return Some(count);
        }
    }
    None
}

pub(crate) fn has_first_spell_each_turn_pattern(words: &[&str]) -> bool {
    let has_turn_context = contains_word_sequence(words, &["each", "turn"])
        || contains_word_sequence(words, &["this", "turn"])
        || contains_word_sequence(words, &["of", "a", "turn"])
        || contains_word_sequence(words, &["during", "your", "turn"])
        || contains_word_sequence(words, &["during", "their", "turn"])
        || contains_word_sequence(words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(words, &["during", "opponents", "turn"])
        || contains_word_sequence(words, &["during", "each", "opponents", "turn"]);
    if !has_turn_context {
        return false;
    }

    for (idx, word) in words.iter().enumerate() {
        if *word != "first" {
            continue;
        }
        let window_end = (idx + 5).min(words.len());
        if words[idx + 1..window_end]
            .iter()
            .any(|candidate| *candidate == "spell" || *candidate == "spells")
        {
            return true;
        }
    }
    false
}

pub(crate) fn has_second_spell_turn_pattern(words: &[&str]) -> bool {
    contains_word_sequence(words, &["second", "spell", "cast", "this", "turn"])
        || contains_word_sequence(words, &["second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["your", "second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["their", "second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["your", "second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["their", "second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["second", "spell", "during", "your", "turn"])
        || contains_word_sequence(words, &["second", "spell", "during", "their", "turn"])
        || contains_word_sequence(
            words,
            &["second", "spell", "during", "an", "opponents", "turn"],
        )
        || contains_word_sequence(words, &["second", "spell", "during", "opponents", "turn"])
        || contains_word_sequence(
            words,
            &["second", "spell", "during", "each", "opponents", "turn"],
        )
}

pub(crate) fn parse_spell_activity_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if !slice_contains(&clause_words, &"spell") && !slice_contains(&clause_words, &"spells") {
        return Ok(None);
    }

    let cast_idx = find_index(tokens, |token| {
        token.is_word("cast") || token.is_word("casts")
    });
    let copy_idx = find_index(tokens, |token| {
        token.is_word("copy") || token.is_word("copies")
    });
    if cast_idx.is_none() && copy_idx.is_none() {
        return Ok(None);
    }

    let mut actor = parse_subject_clause_player_filter(&clause_words);
    let during_their_turn = contains_word_sequence(&clause_words, &["during", "their", "turn"])
        || contains_word_sequence(&clause_words, &["during", "that", "players", "turn"]);
    let mut during_turn = if contains_word_sequence(&clause_words, &["during", "your", "turn"]) {
        Some(PlayerFilter::You)
    } else if contains_word_sequence(&clause_words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "each", "opponents", "turn"])
    {
        Some(PlayerFilter::Opponent)
    } else {
        None
    };
    if during_their_turn {
        if matches!(actor, PlayerFilter::Any) {
            actor = PlayerFilter::Active;
            during_turn = None;
        } else if during_turn.is_none() {
            during_turn = Some(actor.clone());
        }
    }
    let has_other_than_first_spell_pattern =
        contains_word_sequence(&clause_words, &["other", "than", "your", "first", "spell"])
            || contains_word_sequence(&clause_words, &["other", "than", "the", "first", "spell"])
            || (contains_word_sequence(&clause_words, &["other", "than", "the", "first"])
                && slice_contains(&clause_words, &"spell")
                && slice_contains(&clause_words, &"casts")
                && slice_contains(&clause_words, &"turn"));
    let second_spell_turn_pattern = has_second_spell_turn_pattern(&clause_words);
    let first_spell_each_turn =
        !has_other_than_first_spell_pattern && has_first_spell_each_turn_pattern(&clause_words);
    let exact_spells_this_turn = parse_exact_spell_count_each_turn(&clause_words)
        .or_else(|| first_spell_each_turn.then_some(1))
        .or_else(|| {
            (!has_other_than_first_spell_pattern && second_spell_turn_pattern).then_some(2)
        });
    let min_spells_this_turn = if exact_spells_this_turn.is_some() {
        None
    } else if has_other_than_first_spell_pattern {
        Some(2)
    } else {
        None
    };
    let from_not_hand =
        contains_word_sequence(
            &clause_words,
            &["from", "anywhere", "other", "than", "your", "hand"],
        ) || contains_word_sequence(
            &clause_words,
            &["from", "anywhere", "other", "than", "their", "hand"],
        ) || contains_word_sequence(
            &clause_words,
            &["from", "anywhere", "other", "than", "hand"],
        ) || find_word_sequence_start(&clause_words, &["from", "anywhere", "other", "than"])
            .is_some_and(|idx| {
                clause_words[idx + 4..]
                    .iter()
                    .take(4)
                    .any(|word| *word == "hand")
            });

    let parse_filter =
        |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
            let filter_tokens = if let Some(idx) = find_index(filter_tokens, |token| {
                token.is_word("during") || token.is_word("other")
            }) {
                &filter_tokens[..idx]
            } else {
                filter_tokens
            };
            let filter_tokens = if let Some(idx) =
                find_index(filter_tokens, |token| token.is_word("from")).filter(|idx| {
                    filter_tokens
                        .get(idx + 1)
                        .is_some_and(|token| token.is_word("anywhere"))
                }) {
                &filter_tokens[..idx]
            } else {
                filter_tokens
            };
            let filter_words: Vec<&str> = filter_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect();
            let is_unqualified_spell = filter_words.as_slice() == ["a", "spell"]
                || filter_words.as_slice() == ["spells"]
                || filter_words.as_slice() == ["spell"];
            if filter_tokens.is_empty() || is_unqualified_spell {
                Ok(None)
            } else {
                let parse_spell_origin_zone_filter = || -> Option<ObjectFilter> {
                    let zone = if slice_contains(&filter_words, &"graveyard") {
                        Some(Zone::Graveyard)
                    } else if slice_contains(&filter_words, &"exile") {
                        Some(Zone::Exile)
                    } else {
                        None
                    }?;
                    let mentions_spell = slice_contains(&filter_words, &"spell")
                        || slice_contains(&filter_words, &"spells");
                    if !mentions_spell {
                        return None;
                    }
                    let mut filter = ObjectFilter::spell().in_zone(zone);
                    if slice_contains(&filter_words, &"your") {
                        filter.owner = Some(actor.clone());
                    } else if slice_contains(&filter_words, &"opponent")
                        || slice_contains(&filter_words, &"their")
                    {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    Some(filter)
                };
                let compact_words = filter_words
                    .iter()
                    .copied()
                    .filter(|word| !is_article(word))
                    .collect::<Vec<_>>();
                if compact_words
                    .last()
                    .is_some_and(|last| *last == "spell" || *last == "spells")
                {
                    let mut qualifier_words = compact_words.clone();
                    qualifier_words.pop();
                    let qualifier_words = qualifier_words
                        .into_iter()
                        .filter(|word| *word != "or" && *word != "and")
                        .collect::<Vec<_>>();
                    if matches!(
                        qualifier_words.as_slice(),
                        ["of", "the", "chosen", "color"] | ["of", "chosen", "color"]
                    ) {
                        return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                    }
                }
                match parse_object_filter(filter_tokens, false) {
                    Ok(filter) => Ok(Some(filter)),
                    Err(err) => {
                        let mut compact_words = compact_words;
                        if compact_words
                            .last()
                            .is_some_and(|last| *last == "spell" || *last == "spells")
                        {
                            compact_words.pop();
                            let color_words = compact_words
                                .into_iter()
                                .filter(|word| *word != "or" && *word != "and")
                                .collect::<Vec<_>>();
                            if !color_words.is_empty()
                                && color_words.iter().all(|word| parse_color(word).is_some())
                            {
                                let mut colors = ColorSet::new();
                                for word in color_words {
                                    colors = colors
                                        .union(parse_color(word).expect("validated color word"));
                                }
                                let mut filter = ObjectFilter::spell();
                                filter.colors = Some(colors);
                                return Ok(Some(filter));
                            }
                            if matches!(
                                color_words.as_slice(),
                                ["of", "the", "chosen", "color"] | ["of", "chosen", "color"]
                            ) {
                                return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                            }
                        }
                        if let Some(origin_filter) = parse_spell_origin_zone_filter() {
                            Ok(Some(origin_filter))
                        } else {
                            Err(err)
                        }
                    }
                }
            }
        };

    if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
        let (first, second, first_is_cast) = if cast < copy {
            (cast, copy, true)
        } else {
            (copy, cast, false)
        };
        let between_words =
            crate::cards::builders::compiler::token_word_refs(&tokens[first + 1..second]);
        if between_words.as_slice() == ["or"] {
            let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
            let cast_trigger = TriggerSpec::SpellCast {
                filter: filter.clone(),
                caster: actor.clone(),
                during_turn: during_turn.clone(),
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            };
            let copied_trigger = TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            };
            return Ok(Some(if first_is_cast {
                TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
            } else {
                TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
            }));
        }
    }

    if let Some(cast) = cast_idx {
        let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
        if filter_tokens.is_empty() {
            let mut prefix_tokens = &tokens[..cast];
            while let Some(last_word) = prefix_tokens.last().and_then(OwnedLexToken::as_word) {
                if matches!(last_word, "is" | "are" | "was" | "were" | "be" | "been") {
                    prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                } else {
                    break;
                }
            }
            let has_spell_noun = prefix_tokens
                .iter()
                .any(|token| token.is_word("spell") || token.is_word("spells"));
            if has_spell_noun {
                filter_tokens = prefix_tokens;
            }
        }
        let filter = parse_filter(filter_tokens)?;
        return Ok(Some(TriggerSpec::SpellCast {
            filter,
            caster: actor,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        }));
    }

    if let Some(copy) = copy_idx {
        let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
        return Ok(Some(TriggerSpec::SpellCopied {
            filter,
            copier: actor,
        }));
    }

    Ok(None)
}

pub(crate) fn is_spawn_scion_token_mana_reminder(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    let starts_with_token_pronoun = matches!(
        words.as_slice(),
        ["they", "have", ..]
            | ["it", "has", ..]
            | ["this", "token", "has", ..]
            | ["those", "tokens", "have", ..]
    );
    starts_with_token_pronoun
        && words.iter().any(|word| *word == "sacrifice")
        && words.iter().any(|word| *word == "add")
        && words.iter().any(|word| *word == "c")
}

pub(crate) fn is_round_up_each_time_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    matches!(words.as_slice(), ["round", "up", "each", "time", ..])
}

pub(crate) enum MayCastItVerb {
    Cast,
    Play,
}

pub(crate) struct MayCastTaggedSpec {
    pub(crate) verb: MayCastItVerb,
    pub(crate) as_copy: bool,
    pub(crate) without_paying_mana_cost: bool,
    pub(crate) predicate: Option<PredicateAst>,
    pub(crate) cost_reduction: Option<ManaCost>,
}

pub(crate) fn parse_may_cast_it_sentence(tokens: &[OwnedLexToken]) -> Option<MayCastTaggedSpec> {
    let mut clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }

    if slice_starts_with(&clause_words, &["if", "you", "do"]) {
        clause_words = clause_words[3..].to_vec();
        while clause_words
            .first()
            .is_some_and(|word| *word == "then" || *word == "and")
        {
            clause_words.remove(0);
        }
    }

    if clause_words.len() < 4 || clause_words[0] != "you" || clause_words[1] != "may" {
        return None;
    }

    let verb = match clause_words[2] {
        "cast" => MayCastItVerb::Cast,
        "play" => MayCastItVerb::Play,
        _ => return None,
    };

    let rest = &clause_words[3..];
    let (as_copy, consumed) = if slice_starts_with(&rest, &["it"]) {
        (false, 1usize)
    } else if slice_starts_with(&rest, &["the", "exiled", "card"]) {
        (false, 3usize)
    } else if slice_starts_with(&rest, &["the", "copy"])
        || slice_starts_with(&rest, &["that", "copy"])
        || slice_starts_with(&rest, &["a", "copy"])
    {
        (true, 2usize)
    } else {
        return None;
    };

    let tail = &rest[consumed..];
    if tail.is_empty() {
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: false,
            predicate: None,
            cost_reduction: None,
        });
    }
    if tail == ["without", "paying", "its", "mana", "cost"] {
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: true,
            predicate: None,
            cost_reduction: None,
        });
    }
    if let [
        "without",
        "paying",
        "its",
        "mana",
        "cost",
        "if",
        "its",
        "mana",
        "value",
        "is",
        parity,
    ] = tail
    {
        let parity = match *parity {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return None,
        };
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: true,
            predicate: Some(PredicateAst::ItMatches(
                ObjectFilter::default().with_mana_value_parity(parity),
            )),
            cost_reduction: None,
        });
    }
    None
}

pub(crate) fn parse_copy_reference_cost_reduction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<ManaCost> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return None;
    }
    if !(slice_starts_with(&clause_words, &["that", "copy", "costs"])
        || slice_starts_with(&clause_words, &["the", "copy", "costs"])
        || slice_starts_with(&clause_words, &["a", "copy", "costs"]))
    {
        return None;
    }

    let less_idx = find_index(&clause_words, |word| *word == "less")?;
    if clause_words.get(less_idx + 1).copied() != Some("to")
        || clause_words.get(less_idx + 2).copied() != Some("cast")
    {
        return None;
    }

    let costs_token_idx = find_index(tokens, |token| token.is_word("costs"))?;
    let less_token_idx = find_index(tokens, |token| token.is_word("less"))?;
    if less_token_idx <= costs_token_idx + 1 {
        return None;
    }
    let reduction_tokens = trim_commas(&tokens[costs_token_idx + 1..less_token_idx]).to_vec();
    let (reduction, consumed) = parse_cost_modifier_mana_cost(&reduction_tokens)?;
    if consumed != reduction_tokens.len() {
        return None;
    }
    Some(reduction)
}

pub(crate) fn build_may_cast_tagged_effect(spec: &MayCastTaggedSpec) -> EffectAst {
    let cast = EffectAst::CastTagged {
        tag: TagKey::from(IT_TAG),
        allow_land: matches!(spec.verb, MayCastItVerb::Play),
        as_copy: spec.as_copy,
        without_paying_mana_cost: spec.without_paying_mana_cost,
        cost_reduction: spec.cost_reduction.clone(),
    };
    let may = EffectAst::May {
        effects: vec![cast],
    };
    if let Some(predicate) = &spec.predicate {
        EffectAst::Conditional {
            predicate: predicate.clone(),
            if_true: vec![may],
            if_false: Vec::new(),
        }
    } else {
        may
    }
}

pub(crate) fn is_simple_copy_reference_sentence(tokens: &[OwnedLexToken]) -> bool {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    matches!(
        clause_words.as_slice(),
        ["copy", "it"]
            | ["copy", "this"]
            | ["copy", "that"]
            | ["copy", "that", "card"]
            | ["copy", "the", "exiled", "card"]
    )
}

pub(crate) fn token_name_mentions_eldrazi_spawn_or_scion(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.matches("eldrazi").next().is_some() && lower.matches("spawn").next().is_some())
        || (lower.matches("eldrazi").next().is_some() && lower.matches("scion").next().is_some())
}

pub(crate) fn effect_creates_eldrazi_spawn_or_scion(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateTokenWithMods { name, .. } => {
            token_name_mentions_eldrazi_spawn_or_scion(name)
        }
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_eldrazi_spawn_or_scion) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn effect_creates_any_token(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateTokenWithMods { .. }
        | EffectAst::CreateTokenCopy { .. }
        | EffectAst::CreateTokenCopyFromSource { .. }
        | EffectAst::Populate { .. } => true,
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_any_token) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn last_created_token_info(effects: &[EffectAst]) -> Option<(String, PlayerAst)> {
    for effect in effects.iter().rev() {
        if let Some(info) = created_token_info_from_effect(effect) {
            return Some(info);
        }
    }
    None
}

pub(crate) fn created_token_info_from_effect(effect: &EffectAst) -> Option<(String, PlayerAst)> {
    match effect {
        EffectAst::CreateTokenWithMods { name, player, .. } => Some((name.clone(), *player)),
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, true, |nested| {
                if found.is_none() {
                    found = last_created_token_info(nested);
                }
            });
            found
        }
    }
}

pub(crate) fn title_case_token_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

pub(crate) fn controller_filter_for_token_player(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

pub(crate) fn parse_sentence_exile_that_token_when_source_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.len() < 6 || !matches!(clause_words.first().copied(), Some("exile" | "exiles"))
    {
        return None;
    }
    let when_idx = find_index(&clause_words, |word| *word == "when")?;
    if when_idx < 2 || when_idx + 3 >= clause_words.len() {
        return None;
    }
    if !slice_ends_with(&clause_words, &["leaves", "the", "battlefield"]) {
        return None;
    }
    let object_words = &clause_words[1..when_idx];
    let is_created_token_reference = object_words == ["that", "token"]
        || object_words == ["those", "tokens"]
        || object_words == ["them"]
        || object_words == ["it"];
    if !is_created_token_reference {
        return None;
    }
    let subject_words = &clause_words[when_idx + 1..clause_words.len() - 3];
    if !is_source_reference_words(subject_words) {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::ExileWhenSourceLeaves {
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    })
}

pub(crate) fn parse_sentence_sacrifice_source_when_that_token_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = crate::cards::builders::compiler::token_word_refs(tokens);
    if clause_words.len() < 8 || !matches!(clause_words[0], "sacrifice" | "sacrifices") {
        return None;
    }
    let when_idx = find_index(&clause_words, |word| *word == "when")?;
    if when_idx < 2 || when_idx + 4 > clause_words.len() {
        return None;
    }
    let subject_words = &clause_words[1..when_idx];
    if !is_source_reference_words(subject_words) {
        return None;
    }
    if clause_words[when_idx + 1..] != ["that", "token", "leaves", "the", "battlefield"] {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::SacrificeSourceWhenLeaves {
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    })
}

pub(crate) fn is_generic_token_reminder_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    if slice_starts_with(&words, &["it", "has"]) || slice_starts_with(&words, &["they", "have"]) {
        return true;
    }
    if slice_starts_with(&words, &["when", "it"])
        || slice_starts_with(&words, &["whenever", "it"])
        || slice_starts_with(&words, &["when", "they"])
        || slice_starts_with(&words, &["whenever", "they"])
    {
        return true;
    }
    if slice_starts_with(&words, &["its", "power"])
        || slice_starts_with(&words, &["its", "power", "and", "toughness"])
        || slice_starts_with(&words, &["its", "toughness"])
    {
        return true;
    }
    let delayed_lifecycle_reference = matches!(words.first().copied(), Some("exile" | "sacrifice"))
        && (is_beginning_of_end_step_words(&words) || is_end_of_combat_words(&words))
        && (slice_contains(&words, &"token")
            || slice_contains(&words, &"tokens")
            || slice_contains(&words, &"it")
            || slice_contains(&words, &"them"));
    if delayed_lifecycle_reference {
        return true;
    }
    slice_starts_with(&words, &["when", "this", "token"])
        || slice_starts_with(&words, &["whenever", "this", "token"])
        || slice_starts_with(&words, &["this", "token"])
        || slice_starts_with(&words, &["those", "tokens"])
}

pub(crate) fn strip_embedded_token_rules_text(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words_all = crate::cards::builders::compiler::token_word_refs(tokens);
    if !slice_contains(&words_all, &"create") || !slice_contains(&words_all, &"token") {
        return tokens.to_vec();
    }
    let Some(with_idx) = find_index(tokens, |token| token.is_word("with")) else {
        return tokens.to_vec();
    };
    let next_word = tokens.get(with_idx + 1).and_then(OwnedLexToken::as_word);
    if matches!(next_word, Some("t")) {
        return tokens[..with_idx].to_vec();
    }
    tokens.to_vec()
}

pub(crate) fn append_token_reminder_to_last_create_effect(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) -> bool {
    let reminder_word_storage = tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::ManaGroup => {
                let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
                (!inner.is_empty()).then(|| inner.to_ascii_lowercase())
            }
            _ => token.as_word().map(|word| word.to_ascii_lowercase()),
        })
        .collect::<Vec<_>>();
    let mut reminder_words = reminder_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut prepend_with = false;
    if slice_starts_with(&reminder_words, &["it", "has"])
        || slice_starts_with(&reminder_words, &["they", "have"])
    {
        reminder_words = reminder_words[2..].to_vec();
        prepend_with = true;
    }
    if slice_starts_with(&reminder_words, &["when", "it"]) {
        let mut rewritten = vec!["when", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if slice_starts_with(&reminder_words, &["whenever", "it"]) {
        let mut rewritten = vec!["whenever", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if slice_starts_with(&reminder_words, &["when", "they"]) {
        let mut rewritten = vec!["when", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if slice_starts_with(&reminder_words, &["whenever", "they"]) {
        let mut rewritten = vec!["whenever", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    }
    if reminder_words.is_empty() {
        return false;
    }
    let reminder = if prepend_with {
        format!("with {}", reminder_words.join(" "))
    } else {
        reminder_words.join(" ")
    };
    for effect in effects.iter_mut().rev() {
        if append_token_reminder_to_effect(Some(effect), &reminder, &reminder_words) {
            return true;
        }
    }
    false
}

pub(crate) fn append_token_reminder_to_effect(
    effect: Option<&mut EffectAst>,
    reminder: &str,
    reminder_words: &[&str],
) -> bool {
    fn parse_dynamic_token_pt_reminder(reminder_words: &[&str]) -> Option<(Value, Value)> {
        use super::super::util::parse_value;

        let parse_rhs = |words: &[&str]| {
            let tokens = words
                .iter()
                .map(|word| OwnedLexToken::synthetic_word((*word).to_string()))
                .collect::<Vec<_>>();
            let (value, used) = parse_value(&tokens)?;
            (used == words.len()).then_some(value)
        };

        if let Some(rhs_words) = slice_strip_prefix(
            reminder_words,
            &[
                "its",
                "power",
                "and",
                "toughness",
                "are",
                "each",
                "equal",
                "to",
            ],
        ) {
            let value = parse_rhs(rhs_words)?;
            return Some((value.clone(), value));
        }
        let mut and_idx = None;
        let mut idx = 0usize;
        while idx < reminder_words.len() {
            if reminder_words[idx] == "and" {
                and_idx = Some(idx);
                break;
            }
            idx += 1;
        }
        if let Some(and_idx) = and_idx {
            let left = &reminder_words[..and_idx];
            let right = &reminder_words[and_idx + 1..];
            let power_words = slice_strip_prefix(left, &["its", "power", "is", "equal", "to"])?;
            let toughness_words =
                slice_strip_prefix(right, &["its", "toughness", "is", "equal", "to"])?;
            return Some((parse_rhs(power_words)?, parse_rhs(toughness_words)?));
        }

        None
    }

    let Some(effect) = effect else {
        return false;
    };
    match effect {
        EffectAst::CreateTokenCopy {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::Populate {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            if reminder_words == ["haste"] {
                *has_haste = true;
                return true;
            }
            let (sacrifice_next_end_step, exile_next_end_step) =
                parse_next_end_step_token_delay_flags(reminder_words);
            if sacrifice_next_end_step {
                *sacrifice_at_next_end_step = true;
            }
            if exile_next_end_step {
                *exile_at_next_end_step = true;
            }
            let exile_end_of_combat =
                slice_contains(&reminder_words, &"exile") && is_end_of_combat_words(reminder_words);
            if exile_end_of_combat {
                *exile_at_end_of_combat = true;
            }
            *has_haste
                || *sacrifice_at_next_end_step
                || *exile_at_next_end_step
                || *exile_at_end_of_combat
        }
        EffectAst::CreateTokenWithMods {
            name,
            dynamic_power_toughness,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            if let Some((power, toughness)) = parse_dynamic_token_pt_reminder(reminder_words) {
                *dynamic_power_toughness = Some((power, toughness));
                return true;
            }
            if !name.chars().last().is_some_and(|ch| ch == ' ') {
                name.push(' ');
            }
            name.push_str(reminder);
            let (sacrifice_next_end_step, exile_next_end_step) =
                parse_next_end_step_token_delay_flags(reminder_words);
            if sacrifice_next_end_step {
                *sacrifice_at_next_end_step = true;
            }
            if exile_next_end_step {
                *exile_at_next_end_step = true;
            }
            let exile_end_of_combat =
                slice_contains(&reminder_words, &"exile") && is_end_of_combat_words(reminder_words);
            if exile_end_of_combat {
                *exile_at_end_of_combat = true;
            }
            let sacrifice_end_of_combat = slice_contains(&reminder_words, &"sacrifice")
                && is_end_of_combat_words(reminder_words);
            if sacrifice_end_of_combat {
                *sacrifice_at_end_of_combat = true;
            }
            true
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, false, |nested| {
                if !applied {
                    applied = append_token_reminder_to_effect(
                        nested.last_mut(),
                        reminder,
                        reminder_words,
                    );
                }
            });
            applied
        }
    }
}

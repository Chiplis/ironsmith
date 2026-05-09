use super::*;
use crate::runtime_backend::sentences::effect_sentences::lex_chain_helpers::{
    find_verb_lexed, has_effect_head_without_verb_lexed,
};
use crate::runtime_backend::sentences::effect_sentences::subject_verb_primitives::try_build_unless;

fn trim_trailing_discard_alternative_action(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_word("or") {
            continue;
        }

        let alternative_tokens = trim_commas(&tokens[idx + 1..]);
        if alternative_tokens.is_empty() {
            continue;
        }

        let starts_new_action = find_verb_lexed(&alternative_tokens)
            .is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb_lexed(&alternative_tokens);
        if starts_new_action {
            return trim_commas(&tokens[..idx]);
        }
    }

    trim_commas(tokens)
}

fn wrap_unless_escaped(effect: EffectAst, unless_escaped: bool) -> EffectAst {
    if unless_escaped {
        EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellEscaped,
            if_true: Vec::new(),
            if_false: vec![effect],
        }
    } else {
        effect
    }
}

fn parse_unless_mana_spent_to_cast_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let [mana_token, rest @ ..] = tokens else {
        return None;
    };
    if mana_token.kind != crate::runtime_backend::lexer::TokenKind::ManaGroup {
        return None;
    }
    let words = crate::runtime_backend::token_word_refs(rest);
    if words.as_slice() != ["was", "spent", "to", "cast", "it"]
        && words.as_slice() != ["was", "spent", "to", "cast", "this", "spell"]
    {
        return None;
    }
    let symbols = parse_mana_symbol_group(mana_token.slice.as_str()).ok()?;
    let [symbol] = symbols.as_slice() else {
        return None;
    };
    Some(PredicateAst::ManaSpentToCastThisSpellAtLeast {
        amount: 1,
        symbol: Some(*symbol),
    })
}

pub(crate) fn parse_sacrifice(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
    target: Option<TargetAst>,
) -> Result<EffectAst, CardTextError> {
    let mut tokens = tokens;
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let mut normalized_words = clause_words.as_slice();
    let mut unless_escaped = false;
    if let Some(unless_idx) = find_index(&normalized_words, |word| *word == "unless") {
        let tail = &normalized_words[unless_idx..];
        if tail == ["unless", "it", "escaped"] {
            unless_escaped = true;
            let cut_idx = token_index_for_word_index(tokens, unless_idx).unwrap_or(tokens.len());
            tokens = &tokens[..cut_idx];
            normalized_words = &normalized_words[..unless_idx];
        } else {
            let Some(unless_token_idx) =
                find_index(tokens, |token: &OwnedLexToken| token.is_word("unless"))
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported sacrifice-unless clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let sacrifice_tokens = trim_commas(&tokens[..unless_token_idx]);
            let base = parse_sacrifice(&sacrifice_tokens, subject.clone(), target.clone())?;
            if let Some(predicate) =
                parse_unless_mana_spent_to_cast_predicate(&tokens[unless_token_idx + 1..])
            {
                return Ok(EffectAst::Conditional {
                    predicate,
                    if_true: Vec::new(),
                    if_false: vec![base],
                });
            }
            if let Some(unless_effect) = try_build_unless(vec![base], tokens, unless_token_idx)? {
                return Ok(unless_effect);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported sacrifice-unless clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }
    let has_greatest_mana_value = grammar::contains_word(tokens, "greatest")
        && grammar::contains_word(tokens, "mana")
        && grammar::contains_word(tokens, "value");
    if has_greatest_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported greatest-mana-value sacrifice clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }
    let has_for_each_graveyard_history = grammar::contains_word(tokens, "for")
        && grammar::contains_word(tokens, "each")
        && grammar::contains_word(tokens, "graveyard")
        && grammar::contains_word(tokens, "turn");
    if has_for_each_graveyard_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported graveyard-history sacrifice clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }

    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    if tokens
        .first()
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        let mut idx = 1usize;
        let mut other = false;
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_word("other") || token.is_word("another"))
        {
            other = true;
            idx += 1;
        }
        let mut filter = parse_object_filter_lexed(&tokens[idx..], other)?;
        if other {
            filter.other = true;
        }
        return Ok(wrap_unless_escaped(
            EffectAst::subject_verb_sacrifice_all(player, filter),
            unless_escaped,
        ));
    }

    let mut idx = 0;
    let mut count = 1u32;
    let mut other = false;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        other = true;
        idx += 1;
    }
    if count == 1
        && let Some((value, used)) = parse_number(&tokens[idx..])
    {
        count = value;
        idx += used;
    }

    // Split off a trailing "for each ..." suffix before parsing the filter.
    let remaining_tokens = &tokens[idx..];
    let for_each_idx = grammar::find_prefix(remaining_tokens, || grammar::phrase(&["for", "each"]))
        .map(|(idx, _, _)| idx);

    let (object_tokens, for_each_filter) = if let Some(fe_idx) = for_each_idx {
        let fe_count_tokens = &remaining_tokens[fe_idx..];
        let fe_value = parse_get_for_each_count_value(fe_count_tokens)?;
        (&remaining_tokens[..fe_idx], fe_value)
    } else {
        (remaining_tokens, None)
    };

    let filter_words = ZoneHandlerNormalizedWords::new(object_tokens);
    let suffix_word_count =
        if grammar::words_match_suffix(object_tokens, &["of", "their", "choice"]).is_some()
            || grammar::words_match_suffix(object_tokens, &["of", "your", "choice"]).is_some()
            || grammar::words_match_suffix(object_tokens, &["of", "its", "choice"]).is_some()
        {
            3usize
        } else if grammar::words_match_suffix(object_tokens, &["of", "his", "or", "her", "choice"])
            .is_some()
        {
            5usize
        } else {
            0usize
        };
    let filter_tokens = if suffix_word_count == 0 {
        object_tokens
    } else {
        let keep_words = filter_words
            .to_word_refs()
            .len()
            .saturating_sub(suffix_word_count);
        let cut_idx = filter_words
            .token_index_for_word_index(keep_words)
            .unwrap_or(object_tokens.len());
        &object_tokens[..cut_idx]
    };
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after chooser suffix (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    let mut filter = if matches!(filter_words.as_slice(), ["it"] | ["that", "card"]) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter_lexed(filter_tokens, other)?
    };
    if other {
        filter.other = true;
    }
    if filter.source && count != 1 {
        return Err(CardTextError::ParseError(format!(
            "source sacrifice only supports count 1 (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let sacrifice_words = crate::runtime_backend::token_word_refs(tokens);
    let excludes_attached_object = find_window_by(&sacrifice_words, 3, |window| {
        matches!(
            window,
            ["than", "enchanted", "creature"]
                | ["than", "enchanted", "permanent"]
                | ["than", "equipped", "creature"]
                | ["than", "equipped", "permanent"]
        )
    })
    .is_some();
    if excludes_attached_object
        && filter.controller.is_none()
        && let Some(controller) = controller_filter_for_token_player(player)
    {
        filter.controller = Some(controller);
    }

    let sacrifice = EffectAst::subject_verb_sacrifice(player, filter, count, target);

    // Wrap in ForEachObject when the clause has a "for each <filter>" suffix,
    // e.g. "sacrifices a land for each card in your hand".
    let effect = if let Some(Value::Count(fe_filter)) = for_each_filter {
        EffectAst::ForEachObject {
            filter: fe_filter,
            effects: vec![sacrifice],
        }
    } else {
        sacrifice
    };
    Ok(wrap_unless_escaped(effect, unless_escaped))
}

pub(crate) fn parse_discard(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if matches!(
        clause_words.as_slice(),
        ["hand"] | ["your", "hand"] | ["their", "hand"] | ["that", "players", "hand"]
    ) {
        return Ok(EffectAst::subject_verb_discard_hand(player));
    }

    if matches!(clause_words.as_slice(), ["it"] | ["that", "card"]) {
        let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        tagged_filter.zone = Some(Zone::Hand);
        return Ok(EffectAst::subject_verb_discard(
            player,
            Value::Fixed(1),
            false,
            false,
            Some(tagged_filter),
            None,
        ));
    }

    let any_number = clause_words
        .as_slice()
        .starts_with(&["any", "number", "of"]);
    let count_tokens =
        if let Some((_, rest)) = grammar::words_match_any_prefix(tokens, UP_TO_PREFIXES) {
            rest
        } else if any_number {
            &tokens[token_index_for_word_index(tokens, 3).unwrap_or(tokens.len())..]
        } else {
            tokens
        };
    let count_offset = tokens.len().saturating_sub(count_tokens.len());
    let uses_all_count = count_tokens
        .first()
        .is_some_and(|token| token.is_word("all"));
    let (mut count, used) = if uses_all_count {
        (Value::Fixed(0), count_offset + 1)
    } else if any_number {
        (Value::Fixed(0), count_offset)
    } else {
        let (count, used_relative) = parse_value(count_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        (count, count_offset + used_relative)
    };

    let rest = &tokens[used..];
    let rest_words = crate::runtime_backend::token_word_refs(rest);
    let Some(card_word_idx) = find_index(&rest_words, |word| *word == "card" || *word == "cards")
    else {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    };

    let card_token_idx = token_index_for_word_index(rest, card_word_idx).unwrap_or(rest.len());
    let qualifier_tokens = trim_commas(&rest[..card_token_idx]);
    let mut discard_filter = None;
    if !qualifier_tokens.is_empty()
        && crate::runtime_backend::token_word_refs(&qualifier_tokens).as_slice() != ["the"]
    {
        let mut filter = if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
            filter
        } else if let Some(filter) = parse_discard_chosen_color_qualifier_filter(&qualifier_tokens)
        {
            filter
        } else if let Some(filter) = parse_discard_color_qualifier_filter(&qualifier_tokens) {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported discard card qualifier (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        filter.zone = Some(Zone::Hand);
        if uses_all_count
            && let Some(owner) = discard_subject_owner_filter(subject)
            && filter.owner.is_none()
        {
            filter.owner = Some(owner);
        }
        discard_filter = Some(filter);
    }

    let trailing_tokens_storage = if card_word_idx + 1 < rest_words.len() {
        let trailing_token_idx =
            token_index_for_word_index(rest, card_word_idx + 1).unwrap_or(rest.len());
        trim_trailing_discard_alternative_action(&rest[trailing_token_idx..])
    } else {
        Vec::new()
    };
    let trailing_tokens = trailing_tokens_storage.as_slice();
    if let Some(dynamic_count) = parse_get_for_each_count_value(trailing_tokens)? {
        count = dynamic_count;
        return Ok(EffectAst::subject_verb_discard(
            player,
            count,
            false,
            any_number,
            discard_filter,
            None,
        ));
    }
    let trailing_words = crate::runtime_backend::token_word_refs(trailing_tokens);
    let random = trailing_words.as_slice() == ["at", "random"];
    if !trailing_words.is_empty() && !random {
        let trailing_filter = if let Ok(filter) = parse_object_filter(trailing_tokens, false) {
            Some(filter)
        } else if let Some(filter) = parse_discard_chosen_color_qualifier_filter(trailing_tokens) {
            Some(filter)
        } else if let Some(filter) = parse_discard_color_qualifier_filter(trailing_tokens) {
            Some(filter)
        } else {
            None
        };

        if let Some(mut filter) = trailing_filter {
            filter.zone = Some(Zone::Hand);
            if uses_all_count
                && let Some(owner) = discard_subject_owner_filter(subject)
                && filter.owner.is_none()
            {
                filter.owner = Some(owner);
            }
            discard_filter = Some(filter);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discard clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if uses_all_count {
        count = if let Some(filter) = discard_filter.as_ref() {
            Value::Count(filter.clone())
        } else if let Some(owner) = discard_subject_owner_filter(subject) {
            Value::CardsInHand(owner)
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            )));
        };
    }

    Ok(EffectAst::subject_verb_discard(
        player,
        count,
        random,
        any_number,
        discard_filter,
        None,
    ))
}

pub(crate) fn parse_discard_color_qualifier_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let qualifier_words: Vec<&str> = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if qualifier_words.is_empty() {
        return None;
    }

    let mut colors = crate::color::ColorSet::new();
    let mut saw_color = false;
    for word in qualifier_words {
        if word == "or" {
            continue;
        }
        let color = parse_color(word)?;
        colors = colors.union(color);
        saw_color = true;
    }

    if !saw_color {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.colors = Some(colors);
    Some(filter)
}

pub(crate) fn parse_discard_chosen_color_qualifier_filter(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let qualifier_words: Vec<&str> = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !matches!(
        qualifier_words.as_slice(),
        ["of", "that", "color"]
            | ["that", "color"]
            | ["of", "the", "chosen", "color"]
            | ["the", "chosen", "color"]
    ) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.chosen_color = true;
    Some(filter)
}

pub(crate) fn discard_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::target_opponent())
        }
        Some(SubjectAst::Player(PlayerAst::That)) => Some(PlayerFilter::IteratedPlayer),
        Some(SubjectAst::Player(PlayerAst::You)) => Some(PlayerFilter::You),
        _ => None,
    }
}

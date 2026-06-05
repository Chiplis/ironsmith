use super::*;
use crate::runtime_backend::sentences::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape,
};
use crate::runtime_backend::sentences::effect_sentences::lex_chain_helpers::{
    find_verb_lexed, has_effect_head_without_verb_lexed,
};
use crate::runtime_backend::sentences::effect_sentences::subject_verb_primitives::{
    SubjectVerbPrimitiveClause, rewrite_unless_cost_source_values_to_it_tag, try_build_unless,
};

const DISCARD_SAME_MANA_VALUE_FILTER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["with", "that", "spells", "mana", "value"],
            &["with", "that", "spell's", "mana", "value"],
            &[
                "with", "the", "same", "mana", "value", "as", "that", "spell",
            ],
            &["with", "same", "mana", "value", "as", "that", "spell"],
        ]
);
const SACRIFICE_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const SACRIFICE_UNLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["unless"]);
const SACRIFICE_UNLESS_ESCAPED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["unless", "it", "escaped"]);
const SACRIFICE_UNLESS_OPPONENT_DAMAGED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["an", "opponent", "was", "dealt", "damage", "this", "turn"]);
const MANA_SPENT_TO_CAST_SELF_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["was", "spent", "to", "cast", "it"],
            &["was", "spent", "to", "cast", "this", "spell"],
        ]
);
const SACRIFICE_ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const SACRIFICE_OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["other"], &["another"]]);
const SACRIFICE_ANOTHER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["another"]);
const GREATEST_MANA_VALUE_AMONG_WORDS: &[&str] =
    &["with", "the", "greatest", "mana", "value", "among"];
const GREATEST_MANA_VALUE_AMONG_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & GREATEST_MANA_VALUE_AMONG_WORDS);
const GREATEST_POWER_AMONG_WORDS: &[&str] = &["with", "the", "greatest", "power", "among"];
const GREATEST_POWER_AMONG_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & GREATEST_POWER_AMONG_WORDS);
const CHOICE_SUFFIX_THREE_WORD_PATTERNS: &[&[&str]] = &[
    &["of", "their", "choice"],
    &["of", "your", "choice"],
    &["of", "its", "choice"],
];
const CHOICE_SUFFIX_FIVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["of", "his", "or", "her", "choice"]);
const TAGGED_IT_OR_CARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["that", "card"], &["that", "token"]]);
const TAGGED_TOKEN_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that", "token"]);
const ATTACHED_OBJECT_EXCLUSION_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["than", "enchanted", "creature"],
            &["than", "enchanted", "permanent"],
            &["than", "equipped", "creature"],
            &["than", "equipped", "permanent"],
        ]]
);
const DISCARD_HAND_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["hand"],
            &["your", "hand"],
            &["their", "hand"],
            &["that", "players", "hand"],
        ]
);
const DISCARD_THOSE_CARDS_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["those", "cards"]);
const DISCARD_ALL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["all"]);
const DISCARD_CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const DISCARD_THE_QUALIFIER_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const DISCARD_AT_RANDOM_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["at", "random"]);
const DISCARD_WITH_THAT_NAME_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["with", "that", "name"]);
const DISCARD_COLOR_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const DISCARD_CHOSEN_COLOR_PATTERNS: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["of", "that", "color"],
            &["that", "color"],
            &["of", "the", "chosen", "color"],
            &["the", "chosen", "color"],
        ]
);

fn trim_trailing_discard_alternative_action(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    for (idx, token) in tokens.iter().enumerate() {
        if !SACRIFICE_OR_WORD_PATTERN.matches_token(token) {
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

fn discard_value_from_choice_count(count: crate::effect::ChoiceCount) -> Option<(Value, bool)> {
    if count.is_any_number() {
        return Some((Value::Fixed(0), true));
    }
    if count.is_dynamic_x() {
        return Some((Value::X, false));
    }
    if count.min == 0
        && let Some(max) = count.max
    {
        return Some((Value::Fixed(max as i32), false));
    }
    if count.min == count.max? {
        return Some((Value::Fixed(count.min as i32), false));
    }
    None
}

fn parse_discard_count_prefix(tokens: &[OwnedLexToken]) -> Option<(Value, bool, usize)> {
    let (choice_count, used) =
        crate::runtime_backend::util::parse_choice_count_token_prefix_consumed(tokens)?;
    let (value, any_number) = discard_value_from_choice_count(choice_count)?;
    Some((value, any_number, used))
}

fn parse_discard_number_of_cards_equal_count(
    tokens: &[OwnedLexToken],
) -> Option<(Value, usize)> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let prefix_len = [
        &["a", "number", "of", "cards", "equal", "to"][..],
        &["the", "number", "of", "cards", "equal", "to"],
        &["number", "of", "cards", "equal", "to"],
    ]
    .iter()
    .find_map(|prefix| words.starts_with(prefix).then_some(prefix.len()))?;
    let value_token_idx = token_index_for_word_index(tokens, prefix_len)?;
    let (value, used_value_tokens) = parse_value(&tokens[value_token_idx..])?;
    Some((value, value_token_idx + used_value_tokens))
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

fn parse_trailing_discard_same_mana_value_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !DISCARD_SAME_MANA_VALUE_FILTER_PATTERN.matches_words(&words) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter
        .tagged_constraints
        .push(crate::target::TaggedObjectConstraint {
            tag: crate::TagKey::from("triggering"),
            relation: crate::target::TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    Some(filter)
}

fn parse_unless_mana_spent_to_cast_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let [mana_token, rest @ ..] = tokens else {
        return None;
    };
    if mana_token.kind != crate::runtime_backend::lexer::TokenKind::ManaGroup {
        return None;
    }
    let words = crate::runtime_backend::token_word_refs(rest);
    if !MANA_SPENT_TO_CAST_SELF_PATTERN.matches_words(&words) {
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

fn split_greatest_mana_value_among_clause(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_aggregate_among_clause(
        tokens,
        GREATEST_MANA_VALUE_AMONG_WORDS,
        GREATEST_MANA_VALUE_AMONG_PATTERN,
    )
}

fn split_greatest_power_among_clause(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], &[OwnedLexToken])> {
    split_aggregate_among_clause(tokens, GREATEST_POWER_AMONG_WORDS, GREATEST_POWER_AMONG_PATTERN)
}

fn split_aggregate_among_clause<'a>(
    tokens: &'a [OwnedLexToken],
    marker_words: &[&str],
    marker_pattern: ClauseShape<'static>,
) -> Option<(&'a [OwnedLexToken], &'a [OwnedLexToken])> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let marker_start = words
        .windows(marker_words.len())
        .position(|window| marker_pattern.matches_words(window))?;
    let marker_end = marker_start + marker_words.len();
    let before_idx = token_index_for_word_index(tokens, marker_start)?;
    let after_idx = token_index_for_word_index(tokens, marker_end)?;
    Some((&tokens[..before_idx], &tokens[after_idx..]))
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
    if let Some(unless_idx) = find_index(&normalized_words, |word| {
        SACRIFICE_UNLESS_WORD_PATTERN.matches_words(&[*word])
    }) {
        let tail = &normalized_words[unless_idx..];
        if SACRIFICE_UNLESS_ESCAPED_PATTERN.matches_words(tail) {
            unless_escaped = true;
            let cut_idx = token_index_for_word_index(tokens, unless_idx).unwrap_or(tokens.len());
            tokens = &tokens[..cut_idx];
            normalized_words = &normalized_words[..unless_idx];
        } else {
            let Some(unless_token_idx) = find_index(tokens, |token: &OwnedLexToken| {
                SACRIFICE_UNLESS_WORD_PATTERN.matches_token(token)
            }) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported sacrifice-unless clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let sacrifice_tokens = trim_commas(&tokens[..unless_token_idx]);
            let sacrifice_refs_it = TAGGED_IT_OR_CARD_PATTERN
                .matches_words(&crate::runtime_backend::token_word_refs(&sacrifice_tokens));
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
            let unless_words =
                crate::runtime_backend::token_word_refs(&tokens[unless_token_idx + 1..]);
            if SACRIFICE_UNLESS_OPPONENT_DAMAGED_PATTERN.matches_words(&unless_words) {
                return Ok(EffectAst::Conditional {
                    predicate: PredicateAst::OpponentLostLifeThisTurn,
                    if_true: Vec::new(),
                    if_false: vec![base],
                });
            }
            if let Some(mut unless_effect) = try_build_unless(
                vec![base],
                SubjectVerbPrimitiveClause::new(tokens),
                unless_token_idx,
            )? {
                if sacrifice_refs_it {
                    rewrite_unless_cost_source_values_to_it_tag(&mut unless_effect);
                }
                return Ok(unless_effect);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported sacrifice-unless clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
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
        .is_some_and(|token| SACRIFICE_ALL_OR_EACH_WORD_PATTERN.matches_token(token))
    {
        let mut idx = 1usize;
        let mut other = false;
        if tokens
            .get(idx)
            .is_some_and(|token| SACRIFICE_OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token))
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
        .is_some_and(|token| SACRIFICE_ANOTHER_WORD_PATTERN.matches_token(token))
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
    let mut greatest_mana_value_reference_filter = None;
    let mut greatest_power_reference_filter = None;
    let object_clause_tokens = if let Some((base_object_tokens, among_tokens)) =
        split_greatest_mana_value_among_clause(remaining_tokens)
    {
        if among_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object set after greatest mana value among (clause: '{}')",
                normalized_words.join(" ")
            )));
        }
        let among_filter = parse_object_filter_lexed(among_tokens, false)?;
        greatest_mana_value_reference_filter = Some(among_filter);
        base_object_tokens
    } else if let Some((base_object_tokens, among_tokens)) =
        split_greatest_power_among_clause(remaining_tokens)
    {
        if among_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object set after greatest power among (clause: '{}')",
                normalized_words.join(" ")
            )));
        }
        let among_filter = parse_object_filter_lexed(among_tokens, false)?;
        greatest_power_reference_filter = Some(among_filter);
        base_object_tokens
    } else {
        remaining_tokens
    };

    if object_clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in clause (clause: '{}')",
            normalized_words.join(" ")
        )));
    }
    let for_each_idx =
        grammar::find_prefix(object_clause_tokens, || grammar::phrase(&["for", "each"]))
            .map(|(idx, _, _)| idx);

    let (object_tokens, for_each_filter) = if let Some(fe_idx) = for_each_idx {
        let fe_count_tokens = &object_clause_tokens[fe_idx..];
        let fe_value = parse_get_for_each_count_value(fe_count_tokens)?;
        (&object_clause_tokens[..fe_idx], fe_value)
    } else {
        (object_clause_tokens, None)
    };

    let filter_words = ZoneHandlerNormalizedWords::new(object_tokens);
    let suffix_word_count = if CHOICE_SUFFIX_THREE_WORD_PATTERNS
        .iter()
        .any(|suffix| grammar::words_match_suffix(object_tokens, suffix).is_some())
    {
        3usize
    } else if CHOICE_SUFFIX_FIVE_WORD_PATTERN.matches_words(&filter_words.to_word_refs()) {
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
    let mut filter = if TAGGED_IT_OR_CARD_PATTERN.matches_words(&filter_words) {
        let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        tagged_filter.zone = Some(Zone::Battlefield);
        if TAGGED_TOKEN_PATTERN.matches_words(&filter_words) {
            tagged_filter.token = true;
        }
        tagged_filter
    } else {
        parse_object_filter_lexed(filter_tokens, other)?
    };
    if other {
        filter.other = true;
    }
    if let Some(among_filter) = greatest_mana_value_reference_filter {
        filter.mana_value = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::GreatestManaValue(among_filter),
        )));
    }
    if let Some(among_filter) = greatest_power_reference_filter {
        filter.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
            Value::GreatestPower(among_filter),
        )));
    }
    if filter.source && count != 1 {
        return Err(CardTextError::ParseError(format!(
            "source sacrifice only supports count 1 (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let sacrifice_words = crate::runtime_backend::token_word_refs(tokens);
    let excludes_attached_object =
        ATTACHED_OBJECT_EXCLUSION_PATTERN.matches_words(&sacrifice_words);
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
    if DISCARD_HAND_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_discard_hand(player));
    }

    if TAGGED_IT_OR_CARD_PATTERN.matches_words(&clause_words) {
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

    if DISCARD_THOSE_CARDS_PATTERN.matches_words(&clause_words) {
        let mut tagged_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        tagged_filter.zone = Some(Zone::Hand);
        return Ok(EffectAst::subject_verb_discard(
            player,
            Value::Count(tagged_filter.clone()),
            false,
            false,
            Some(tagged_filter),
            None,
        ));
    }

    if let Some((count, used)) = parse_discard_number_of_cards_equal_count(tokens) {
        let trailing_tokens = trim_commas(&tokens[used..]);
        let trailing_words = crate::runtime_backend::token_word_refs(&trailing_tokens);
        let random = DISCARD_AT_RANDOM_PATTERN.matches_words(&trailing_words);
        if !trailing_words.is_empty() && !random {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing discard clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(EffectAst::subject_verb_discard(
            player, count, random, false, None, None,
        ));
    }

    let uses_all_count = tokens
        .first()
        .is_some_and(|token| DISCARD_ALL_WORD_PATTERN.matches_token(token));
    let (mut count, any_number, used) = if uses_all_count {
        (Value::Fixed(0), false, 1)
    } else if let Some((count, any_number, used)) = parse_discard_count_prefix(tokens) {
        (count, any_number, used)
    } else {
        let (count, used) = parse_value(tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing discard count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        (count, false, used)
    };

    let rest = &tokens[used..];
    let rest_words = crate::runtime_backend::token_word_refs(rest);
    let Some(card_word_idx) = find_index(&rest_words, |word| {
        DISCARD_CARD_OR_CARDS_WORD_PATTERN.matches_words(&[*word])
    }) else {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    };

    let card_token_idx = token_index_for_word_index(rest, card_word_idx).unwrap_or(rest.len());
    let qualifier_tokens = trim_commas(&rest[..card_token_idx]);
    let mut discard_filter = None;
    if !qualifier_tokens.is_empty()
        && !DISCARD_THE_QUALIFIER_PATTERN
            .matches_words(&crate::runtime_backend::token_word_refs(&qualifier_tokens))
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
    let random = DISCARD_AT_RANDOM_PATTERN.matches_words(&trailing_words);
    if !trailing_words.is_empty() && !random {
        let trailing_filter = if let Ok(filter) = parse_object_filter(trailing_tokens, false) {
            Some(filter)
        } else if DISCARD_WITH_THAT_NAME_PATTERN.matches_words(&trailing_words) {
            let mut filter = ObjectFilter::default();
            filter.name = Some("{chosen name}".to_string());
            Some(filter)
        } else if let Some(filter) = parse_discard_chosen_color_qualifier_filter(trailing_tokens) {
            Some(filter)
        } else if let Some(filter) = parse_trailing_discard_same_mana_value_filter(trailing_tokens)
        {
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
    let qualifier_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    if qualifier_words.is_empty() {
        return None;
    }

    let mut colors = crate::color::ColorSet::new();
    let mut saw_color = false;
    for word in qualifier_words {
        if DISCARD_COLOR_OR_WORD_PATTERN.matches_word(word) {
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
    let qualifier_words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    if !DISCARD_CHOSEN_COLOR_PATTERNS.matches_words(qualifier_words.as_slice()) {
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

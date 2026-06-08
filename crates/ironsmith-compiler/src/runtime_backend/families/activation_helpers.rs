use crate::effect::Value;
use crate::host::{CardTextError, EffectAst, OwnedLexToken, PlayerAst, SubjectAst};
use crate::mana::ManaSymbol;
use crate::target::ObjectFilter;

use super::activation_and_restrictions::activated_line_core::parse_devotion_value_from_add_clause;
use super::effect_sentences::clause_pattern_helpers::extract_subject_player;
use super::grammar::structure::parse_trailing_instead_if_predicate_lexed;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_add_mana_that_much_value,
    parse_dynamic_cost_modifier_value, parse_where_x_is_number_of_filter_value,
};
use super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};
use super::lexer::{
    LexedClause, TokenWordView, token_word_refs, word_slice_contains_all_words,
    word_slice_contains_any_word, word_slice_contains_phrase, word_slice_ends_with,
    word_slice_ends_with_any, word_slice_eq, word_slice_starts_with,
};
pub(crate) use super::object_filters::is_comparison_or_delimiter;
use super::object_filters::parse_object_filter;
pub(crate) use super::util::{
    contains_discard_source_phrase, contains_from_command_zone_phrase,
    contains_source_from_your_graveyard_phrase, contains_source_from_your_hand_phrase,
    find_activation_cost_start, is_article, is_basic_color_word,
    is_source_from_your_graveyard_words, join_sentences_with_period, mana_pips_from_token,
    non_article_token_word_refs, non_article_word_refs, parse_mana_symbol,
    parse_next_end_step_token_delay_flags, parse_subtype_flexible, parse_value,
    split_cost_segments, strip_leading_article_tokens, token_index_for_word_index, trim_commas,
    trim_edge_punctuation_tokens, value_contains_unbound_x, word_refs_at_is_article,
    word_refs_except,
};
pub(crate) use super::value_helpers::{
    parse_equal_to_aggregate_filter_value, parse_filter_comparison_tokens,
};

fn push_unique_color(colors: &mut Vec<crate::color::Color>, color: crate::color::Color) {
    crate::slice_primitives::push_unique(colors, color);
}

fn first_non_comma_token_index(tokens: &[OwnedLexToken]) -> usize {
    crate::slice_primitives::find_index(tokens, |token| !token.is_comma()).unwrap_or(tokens.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddManaChoiceKind {
    AnyOneColor,
    AnyColor,
    AnyOneType,
    AnyType,
}

impl AddManaChoiceKind {
    fn any_one(self) -> bool {
        matches!(self, Self::AnyOneColor | Self::AnyOneType)
    }

    fn allow_colorless(self) -> bool {
        matches!(self, Self::AnyOneType | Self::AnyType)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AddManaChoiceClause<'a> {
    kind: AddManaChoiceKind,
    tail_tokens: &'a [OwnedLexToken],
}

const ADD_MANA_THAT_COLOR_AMOUNT_PREFIX: &[&str] =
    &["an", "amount", "of", "mana", "of", "that", "color"];
const ADD_MANA_ONE_THAT_COLOR_PREFIX: &[&str] = &["one", "mana", "of", "that", "color"];
const MANA_OF_CHOSEN_COLOR_SUFFIXES: &[&[&str]] = &[&["mana", "of", "the"], &["mana", "of"]];
const FOR_EACH_REMOVED_THIS_WAY_PREFIX: &[&str] = &["for", "each"];
const FOR_EACH_REMOVED_THIS_WAY_SUFFIX: &[&str] = &["removed", "this", "way"];
const CHOSEN_COLOR_MANA_TAIL_PREFIX: &[&str] =
    &["or", "one", "mana", "of", "the", "chosen", "color"];
const MANA_POOL_START_PREFIX: &[&str] = &["to"];
const LAND_PRODUCE_SUBJECT_PREFIX: &[&str] = &["that"];
const CHOSEN_COLOR_PHRASE: &[&str] = &["chosen", "color"];
const FOR_EACH_PREFIX: &[&str] = &["for", "each"];
const FOR_EACH_COLOR_AMONG_PHRASE: &[&str] = &["for", "each", "color", "among"];
const ADD_ONE_MANA_OF_THAT_COLOR_PHRASE: &[&str] = &["add", "one", "mana", "of", "that", "color"];
const ANY_COMBINATION_OF_PHRASE: &[&str] = &["any", "combination", "of"];
const COULD_PRODUCE_PHRASE: &[&str] = &["could", "produce"];
const MANA_POOL_TAIL_WORDS: &[&str] = &[
    "to", "your", "their", "its", "that", "player", "players", "player's", "players'", "mana",
    "pool",
];
const SIMPLE_MANA_FILLER_WORDS: &[&str] = &["mana", "to", "your", "pool"];
const MANA_CHOICE_TAIL_WORDS: &[&str] = &["to", "your", "their", "its", "mana", "pool"];
const MANA_OPTION_SEPARATOR_WORDS: &[&str] = &[
    "and", "or", "and/or", "mana", "to", "your", "their", "its", "pool",
];

fn activation_find_phrase_start(clause: LexedClause<'_>, phrase: &[&str]) -> Option<usize> {
    let phrase_len = phrase.len();
    let words = clause.word_refs();
    if phrase_len == 0 || words.len() < phrase_len {
        return None;
    }
    words
        .windows(phrase_len)
        .position(|window| word_slice_eq(window, phrase))
}

fn activation_token_slice_prefix_at_matches_phrase(
    tokens: &[OwnedLexToken],
    index: usize,
    prefix: &[&str],
) -> bool {
    let Some(tail) = tokens.get(index..) else {
        return false;
    };
    let words = TokenWordView::new(tail).to_word_refs();
    word_slice_starts_with(&words, prefix)
}

fn activation_words_contain_all(words: &[&str], required: &[&str]) -> bool {
    word_slice_contains_all_words(words, required)
}

fn activation_words_contain_any(words: &[&str], candidates: &[&str]) -> bool {
    word_slice_contains_any_word(words, candidates)
}

fn activation_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    word_slice_contains_phrase(words, phrase)
}

fn activation_words_start_with(words: &[&str], prefix: &[&str]) -> bool {
    word_slice_starts_with(words, prefix)
}

fn activation_words_end_with_any(words: &[&str], suffixes: &[&[&str]]) -> bool {
    word_slice_ends_with_any(words, suffixes)
}

fn activation_word_is_any(word: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|candidate| word == *candidate)
}

fn activation_token_is(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|_| token.parser_text() == expected)
}

fn parse_add_mana_choice_clause(tokens: &[OwnedLexToken]) -> Option<AddManaChoiceClause<'_>> {
    const MANA_CHOICE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "choice",
            LexCaptureKind::OneOfPhrase(&[
                &["any", "one", "color"],
                &["any", "color"],
                &["one", "color"],
                &["any", "one", "type"],
                &["any", "type"],
                &["one", "type"],
            ]),
        ),
        LexPattern::tail("tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = MANA_CHOICE_PATTERN.find_in_clause(clause)?;
    let choice_clause = matched.capture_clause_by_role(LexCaptureRole::Object, clause)?;
    let tail_clause = matched.capture_clause_by_role(LexCaptureRole::Tail, clause)?;
    let choice_words = choice_clause.word_refs();
    let kind = match choice_words.as_slice() {
        ["any", "one", "color"] => AddManaChoiceKind::AnyOneColor,
        ["any", "color"] | ["one", "color"] => AddManaChoiceKind::AnyColor,
        ["any", "one", "type"] => AddManaChoiceKind::AnyOneType,
        ["any", "type"] | ["one", "type"] => AddManaChoiceKind::AnyType,
        _ => return None,
    };

    Some(AddManaChoiceClause {
        kind,
        tail_tokens: trim_leading_commas(tail_clause.tokens()),
    })
}

pub(crate) fn parse_add_mana(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause = LexedClause::new(tokens);
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[OwnedLexToken]|
     -> Result<Option<EffectAst>, CardTextError> {
        let tail_words = TokenWordView::new(tail_tokens).to_word_refs();
        if tail_words.first().copied() != Some("instead")
            || tail_words.get(1).copied() != Some("if")
        {
            return Ok(None);
        }
        let predicate =
            parse_trailing_instead_if_predicate_lexed(tail_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported trailing mana clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        Ok(Some(EffectAst::Conditional {
            predicate,
            if_true: vec![base_effect],
            if_false: Vec::new(),
        }))
    };

    let has_card_word = clause_words
        .iter()
        .any(|word| activation_word_is_any(word, &["card", "cards"]));
    if let Some(colors_among) = parse_add_mana_colors_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_colors_among(
            player,
            colors_among,
        ));
    }
    if has_card_word && activation_words_contain_all(&clause_words, &["exiled", "colors"]) {
        return Ok(EffectAst::subject_verb_add_mana_imprinted_colors());
    }

    if activation_words_contain_any(&clause_words, &["commander", "commanders"])
        && activation_words_contain_all(&clause_words, &["color", "identity"])
    {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_commander_identity(
            player, amount,
        ));
    }

    if activation_words_contain_phrase(&clause_words, &["different", "colors"]) {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_any_color_with_distinct(
            player, amount, None, true,
        ));
    }

    if let Some(available_colors) = parse_any_combination_mana_colors(tokens)? {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            amount,
            Some(available_colors),
        ));
    }

    if let Some(available_colors) = parse_or_mana_color_choices(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_any_color(
            player,
            Value::Fixed(1),
            Some(available_colors),
        ));
    }

    let has_explicit_symbol = tokens
        .iter()
        .any(|token| mana_pips_from_token(token).is_some());
    if !has_explicit_symbol
        && let Some(chosen_idx) = activation_find_phrase_start(clause, CHOSEN_COLOR_PHRASE)
    {
        let references_mana_of_chosen_color =
            clause
                .between_word_range(0, chosen_idx)
                .is_some_and(|prefix| {
                    activation_words_end_with_any(
                        &prefix.word_refs(),
                        MANA_OF_CHOSEN_COLOR_SUFFIXES,
                    )
                });
        if references_mana_of_chosen_color {
            let tail_words = &clause_words[chosen_idx + 2..];
            let has_only_pool_tail = tail_words.is_empty()
                || tail_words
                    .iter()
                    .all(|word| activation_word_is_any(word, MANA_POOL_TAIL_WORDS));
            if has_only_pool_tail {
                let amount = parse_value(tokens)
                    .map(|(value, _)| value)
                    .unwrap_or(Value::Fixed(1));
                return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                    player, amount, None,
                ));
            }
        }
    }
    if activation_words_start_with(&clause_words, ADD_MANA_ONE_THAT_COLOR_PREFIX) {
        let tail_start = clause_word_view
            .token_index_after_words(5)
            .unwrap_or(tokens.len());
        let tail_tokens = trim_leading_commas(&tokens[tail_start..]);
        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player,
                Value::Fixed(1),
                None,
            ));
        }
        if let Some(amount) = parse_dynamic_cost_modifier_value(tail_tokens)? {
            return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                player, amount, None,
            ));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic chosen-color mana amount (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if activation_words_start_with(&clause_words, ADD_MANA_THAT_COLOR_AMOUNT_PREFIX) {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }

    if let Some(mana_choice) = parse_add_mana_choice_clause(tokens) {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let any_one = mana_choice.kind.any_one();
        let any_type = mana_choice.kind.allow_colorless();
        let tail_tokens = mana_choice.tail_tokens;

        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if let Some(filter) = parse_land_could_produce_filter(tail_tokens)? {
            return Ok(EffectAst::subject_verb_add_mana_from_land_could_produce(
                player, amount, filter, any_type, any_one,
            ));
        }

        if matches!(amount, Value::X)
            && let Some(dynamic_amount) = parse_where_x_is_number_of_filter_value(tail_tokens)
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        let tail_words = token_word_refs(tail_tokens);
        let chosen_by_player_tail = matches!(
            tail_words.as_slice(),
            ["they", "choose"]
                | ["that", "player", "chooses"]
                | ["they", "choose", "to", "their", "mana", "pool"]
                | ["that", "player", "chooses", "to", "their", "mana", "pool"]
        );
        if chosen_by_player_tail {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }
        if activation_words_start_with(&tail_words, FOR_EACH_REMOVED_THIS_WAY_PREFIX)
            && word_slice_ends_with(tail_words.as_slice(), FOR_EACH_REMOVED_THIS_WAY_SUFFIX)
            && let Some(dynamic_amount) = parse_dynamic_cost_modifier_value(tail_tokens)?
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        if tail_words.first().copied() == Some("among") {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::subject_verb_add_mana_any_one_color(
                    player, amount,
                ));
            }
            return Ok(EffectAst::subject_verb_add_mana_any_color(
                player, amount, None,
            ));
        }

        let base_effect = if any_one {
            EffectAst::subject_verb_add_mana_any_one_color(player, amount)
        } else {
            EffectAst::subject_verb_add_mana_any_color(player, amount, None)
        };
        if let Some(conditional) = wrap_instead_if_tail(base_effect, tail_tokens)? {
            return Ok(conditional);
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let for_each_idx = (0..tokens.len().saturating_sub(1)).find(|&token_idx| {
        activation_token_slice_prefix_at_matches_phrase(tokens, token_idx, FOR_EACH_PREFIX)
    });
    let mana_scan_end = for_each_idx.unwrap_or(tokens.len());

    let mut mana = Vec::new();
    let mut last_mana_idx = None;
    for (idx, token) in tokens[..mana_scan_end].iter().enumerate() {
        if let Some(group) = mana_pips_from_token(token) {
            mana.extend(group);
            last_mana_idx = Some(idx);
            continue;
        }
        if token
            .as_word()
            .is_some_and(|word| activation_word_is_any(word, SIMPLE_MANA_FILLER_WORDS))
        {
            continue;
        }
    }

    if !mana.is_empty() {
        if let Some(amount) = parse_add_mana_that_much_value(tokens) {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_devotion_value_from_add_clause(tokens)? {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(for_each_idx) = for_each_idx {
            let amount_tokens = &tokens[for_each_idx..];
            let amount = parse_dynamic_cost_modifier_value(amount_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic mana amount (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        if let Some(amount) = parse_equal_to_aggregate_filter_value(tokens)
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
        {
            return Ok(EffectAst::subject_verb_add_mana_scaled(
                player, mana, amount,
            ));
        }
        let trailing_words = last_mana_idx
            .map(|last_idx| token_word_refs(&tokens[last_idx + 1..]))
            .unwrap_or_default();
        if !trailing_words.is_empty() {
            let chosen_color_tail =
                activation_words_start_with(&trailing_words, CHOSEN_COLOR_MANA_TAIL_PREFIX);
            let pool_tail = if chosen_color_tail {
                trailing_words[7..].to_vec()
            } else {
                Vec::new()
            };
            let has_only_pool_tail = chosen_color_tail
                && (pool_tail.is_empty()
                    || pool_tail
                        .iter()
                        .all(|word| activation_word_is_any(word, MANA_CHOICE_TAIL_WORDS)));
            if chosen_color_tail && has_only_pool_tail {
                if mana.len() != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with multiple symbols (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                let Some(color) = mana_symbol_to_color(mana[0]) else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with non-colored symbol (clause: '{}')",
                        clause_words.join(" ")
                    )));
                };
                return Ok(EffectAst::subject_verb_add_mana_chosen_color(
                    player,
                    Value::Fixed(1),
                    Some(color),
                ));
            }
        }
        let has_only_pool_tail = !trailing_words.is_empty()
            && trailing_words
                .iter()
                .all(|word| activation_word_is_any(word, MANA_CHOICE_TAIL_WORDS));
        let has_only_instead_tail = word_slice_eq(&trailing_words, &["instead"]);
        if !trailing_words.is_empty() && !has_only_pool_tail && !has_only_instead_tail {
            if let Some(last_idx) = last_mana_idx
                && let Some(conditional) = wrap_instead_if_tail(
                    EffectAst::subject_verb_add_mana(player, mana.clone()),
                    trim_leading_commas(&tokens[last_idx + 1..]),
                )?
            {
                return Ok(conditional);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(EffectAst::subject_verb_add_mana(player, mana));
    }

    Err(CardTextError::ParseError(format!(
        "missing mana symbols (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_add_mana_colors_among_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words = token_word_refs(tokens);
    if activation_find_phrase_start(clause, FOR_EACH_COLOR_AMONG_PHRASE).is_none()
        || activation_find_phrase_start(clause, ADD_ONE_MANA_OF_THAT_COLOR_PHRASE).is_none()
    {
        return Ok(None);
    }

    let mut among_token_idx = None;
    let mut add_token_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if activation_token_is(token, "among") && among_token_idx.is_none() {
            among_token_idx = Some(idx);
        }
        if activation_token_is(token, "add") && add_token_idx.is_none() {
            add_token_idx = Some(idx);
        }
    }
    let (Some(among_token_idx), Some(add_token_idx)) = (among_token_idx, add_token_idx) else {
        return Ok(None);
    };
    if add_token_idx <= among_token_idx + 1 {
        return Ok(None);
    }

    let filter_tokens = trim_commas(&tokens[among_token_idx + 1..add_token_idx]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false)?;
    Ok(Some(filter))
}

pub(crate) fn mana_symbol_to_color(symbol: ManaSymbol) -> Option<crate::color::Color> {
    match symbol {
        ManaSymbol::White => Some(crate::color::Color::White),
        ManaSymbol::Blue => Some(crate::color::Color::Blue),
        ManaSymbol::Black => Some(crate::color::Color::Black),
        ManaSymbol::Red => Some(crate::color::Color::Red),
        ManaSymbol::Green => Some(crate::color::Color::Green),
        _ => None,
    }
}

pub(crate) fn parse_or_mana_color_choices(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !activation_words_contain_all(&clause_words, &["or"]) {
        return Ok(None);
    }

    let mut colors = Vec::new();
    let mut has_or = false;
    for token in tokens {
        if activation_token_is(token, "or") {
            has_or = true;
            continue;
        }
        if let Some(group) = mana_pips_from_token(token) {
            for symbol in group {
                let Some(color) = mana_symbol_to_color(symbol) else {
                    return Ok(None);
                };
                push_unique_color(&mut colors, color);
            }
            continue;
        }
        let Some(word) = token.as_word() else {
            continue;
        };
        if activation_word_is_any(word, MANA_CHOICE_TAIL_WORDS) {
            continue;
        }
        return Ok(None);
    }

    if !has_or || colors.len() < 2 {
        return Ok(None);
    }

    Ok(Some(colors))
}

pub(crate) fn parse_any_combination_mana_colors(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let Some(combination_idx) =
        activation_find_phrase_start(LexedClause::new(tokens), ANY_COMBINATION_OF_PHRASE)
    else {
        return Ok(None);
    };

    let mut colors = Vec::new();
    for word in &clause_words[combination_idx + 3..] {
        if *word == "where" {
            break;
        }
        if activation_word_is_any(word, MANA_OPTION_SEPARATOR_WORDS) {
            continue;
        }
        if activation_word_is_any(word, &["color", "colors"]) {
            for color in [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ] {
                push_unique_color(&mut colors, color);
            }
            continue;
        }
        let symbol = parse_mana_symbol(word).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported restricted mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        let color = mana_symbol_to_color(symbol).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported non-colored mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        push_unique_color(&mut colors, color);
    }

    if colors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing color options in any-combination mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(colors))
}

pub(crate) fn trim_leading_commas(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let start = first_non_comma_token_index(tokens);
    &tokens[start..]
}

pub(crate) fn is_mana_pool_tail_tokens(tokens: &[OwnedLexToken]) -> bool {
    let words_view = TokenWordView::new(tokens);
    let words = words_view.to_word_refs();
    if !activation_words_start_with(&words, MANA_POOL_START_PREFIX)
        || !activation_words_contain_all(&words, &["mana", "pool"])
    {
        return false;
    }
    words
        .iter()
        .all(|word| activation_word_is_any(word, MANA_POOL_TAIL_WORDS))
}

pub(crate) fn parse_land_could_produce_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words_view = TokenWordView::new(tokens);
    let words = words_view.to_word_refs();
    let clause = LexedClause::new(tokens);
    if words.len() < 3 || !activation_words_start_with(&words, LAND_PRODUCE_SUBJECT_PREFIX) {
        return Ok(None);
    }

    let marker_word_idx =
        if let Some(could_idx) = activation_find_phrase_start(clause, COULD_PRODUCE_PHRASE) {
            if could_idx + 2 != words.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing mana clause (tail: '{}')",
                    words.join(" ")
                )));
            }
            could_idx
        } else {
            let Some(produced_idx) = words.iter().position(|word| *word == "produced") else {
                return Ok(None);
            };
            if produced_idx + 1 != words.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing mana clause (tail: '{}')",
                    words.join(" ")
                )));
            }
            produced_idx
        };

    let marker_token_idx =
        token_index_for_word_index(tokens, marker_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing mana production marker in tail '{}'",
                words.join(" ")
            ))
        })?;
    let filter_tokens = trim_leading_commas(&tokens[1..marker_token_idx]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing land filter in mana clause (tail: '{}')",
            words.join(" ")
        )));
    }
    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(filter))
}

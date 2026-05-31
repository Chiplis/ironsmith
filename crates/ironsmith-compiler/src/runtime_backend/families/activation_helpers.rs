use crate::effect::Value;
use crate::host::{CardTextError, EffectAst, OwnedLexToken, PlayerAst, SubjectAst};
use crate::mana::ManaSymbol;
use crate::target::ObjectFilter;

use super::activation_and_restrictions::activated_line_core::parse_devotion_value_from_add_clause;
use super::effect_sentences::clause_pattern_helpers::{
    ClauseShape, clause_shape, extract_subject_player,
};
use super::grammar::structure::parse_trailing_instead_if_predicate_lexed;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_add_mana_that_much_value,
    parse_dynamic_cost_modifier_value, parse_where_x_is_number_of_filter_value,
};
use super::lexer::{TokenWordView, token_word_refs};
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

const ADD_MANA_IMPRINTED_COLORS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["exiled", "colors"]);

const ADD_MANA_COMMANDER_IDENTITY_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words &[&["commander", "commanders"]];
    contains_words &["color", "identity"]
);

const ADD_MANA_THAT_COLOR_AMOUNT_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["an", "amount", "of", "mana", "of", "that", "color"]);

const ADD_MANA_ANY_ONE_COLOR_OR_TYPE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["any", "one", "color"], &["any", "one", "type"]]]);
const ADD_MANA_ANY_COLOR_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["any", "color"], &["one", "color"]]]);
const ADD_MANA_ANY_TYPE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["any", "type"], &["one", "type"]]]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const MANA_OF_CHOSEN_COLOR_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["mana", "of", "the"], &["mana", "of"]]);
const COLOR_OR_TYPE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["color"], &["type"]]);
const COLOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["color"]);
const TYPE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["type"]);
const FOR_EACH_REMOVED_THIS_WAY_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["for", "each"]; suffix & ["removed", "this", "way"]);
const AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const ADD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["add"]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const OR_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["or"]);
const WHERE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["where"]);
const INSTEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const INSTEAD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const CHOSEN_COLOR_MANA_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["or", "one", "mana", "of", "the", "chosen", "color"]);
const SIMPLE_MANA_POOL_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["mana", "pool"]);
const MANA_POOL_START_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["to"]);
const LAND_PRODUCE_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["that"]);
const PRODUCED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["produced"]);
const CHOSEN_COLOR_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["chosen", "color"]);
const CHOSEN_COLOR_PHRASE_LEN: usize = 2;
const FOR_EACH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["for", "each"]);
const FOR_EACH_COLOR_AMONG_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["for", "each", "color", "among"]);
const FOR_EACH_COLOR_AMONG_PHRASE_LEN: usize = 4;
const ADD_ONE_MANA_OF_THAT_COLOR_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["add", "one", "mana", "of", "that", "color"]);
const ADD_ONE_MANA_OF_THAT_COLOR_PHRASE_LEN: usize = 6;
const ANY_COMBINATION_OF_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["any", "combination", "of"]);
const ANY_COMBINATION_OF_PHRASE_LEN: usize = 3;
const COULD_PRODUCE_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["could", "produce"]);
const COULD_PRODUCE_PHRASE_LEN: usize = 2;
const MANA_POOL_TAIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["to"],
            &["your"],
            &["their"],
            &["its"],
            &["that"],
            &["player"],
            &["players"],
            &["player's"],
            &["players'"],
            &["mana"],
            &["pool"],
        ]
);
const SIMPLE_MANA_FILLER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["mana"], &["to"], &["your"], &["pool"]]);
const MANA_CHOICE_TAIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["to"],
            &["your"],
            &["their"],
            &["its"],
            &["mana"],
            &["pool"],
        ]
);
const MANA_OPTION_SEPARATOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["and"],
            &["or"],
            &["and/or"],
            &["mana"],
            &["to"],
            &["your"],
            &["their"],
            &["its"],
            &["pool"],
        ]
);
const COLOR_OR_COLORS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["color"], &["colors"]]);

fn activation_find_phrase_start(
    words: &[&str],
    phrase_len: usize,
    shape: &ClauseShape<'static>,
) -> Option<usize> {
    if phrase_len == 0 || words.len() < phrase_len {
        return None;
    }
    words
        .windows(phrase_len)
        .position(|window| shape.matches_words(window))
}

fn activation_token_slice_prefix_at_matches_shape(
    tokens: &[OwnedLexToken],
    index: usize,
    shape: &ClauseShape<'static>,
) -> bool {
    tokens
        .get(index..)
        .is_some_and(|tail| shape.matches_words(&TokenWordView::new(tail).to_word_refs()))
}

pub(crate) fn parse_add_mana(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let clause_word_view = TokenWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let wrap_instead_if_tail = |base_effect: EffectAst,
                                tail_tokens: &[OwnedLexToken]|
     -> Result<Option<EffectAst>, CardTextError> {
        let tail_words = TokenWordView::new(tail_tokens).to_word_refs();
        if !INSTEAD_WORD_PATTERN.matches_word_at(&tail_words, 0)
            || !IF_WORD_PATTERN.matches_word_at(&tail_words, 1)
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
        .any(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word));
    if let Some(colors_among) = parse_add_mana_colors_among_filter(tokens)? {
        return Ok(EffectAst::subject_verb_add_mana_colors_among(
            player,
            colors_among,
        ));
    }
    if has_card_word && ADD_MANA_IMPRINTED_COLORS_PATTERN.matches_words(&clause_words) {
        return Ok(EffectAst::subject_verb_add_mana_imprinted_colors());
    }

    if ADD_MANA_COMMANDER_IDENTITY_PATTERN.matches_words(&clause_words) {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_commander_identity(
            player, amount,
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
        && let Some(chosen_idx) = activation_find_phrase_start(
            &clause_words,
            CHOSEN_COLOR_PHRASE_LEN,
            &CHOSEN_COLOR_PHRASE_PATTERN,
        )
    {
        let prefix = &clause_words[..chosen_idx];
        let references_mana_of_chosen_color =
            MANA_OF_CHOSEN_COLOR_PREFIX_PATTERN.matches_words(prefix);
        if references_mana_of_chosen_color {
            let tail_words = &clause_words[chosen_idx + 2..];
            let has_only_pool_tail = tail_words.is_empty()
                || tail_words
                    .iter()
                    .all(|word| MANA_POOL_TAIL_WORD_PATTERN.matches_word(word));
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
    if ADD_MANA_THAT_COLOR_AMOUNT_PATTERN.matches_words(&clause_words) {
        let amount = parse_devotion_value_from_add_clause(tokens)?
            .or_else(|| parse_add_mana_equal_amount_value(tokens))
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::subject_verb_add_mana_chosen_color(
            player, amount, None,
        ));
    }

    let any_one = ADD_MANA_ANY_ONE_COLOR_OR_TYPE_PATTERN.matches_words(&clause_words);
    let any_color = ADD_MANA_ANY_COLOR_PATTERN.matches_words(&clause_words);
    let any_type = ADD_MANA_ANY_TYPE_PATTERN.matches_words(&clause_words);
    if any_color || any_type {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let allow_colorless = any_type;
        let phrase_end = crate::slice_primitives::find_index(tokens, |token| {
            token.as_word().is_some_and(|word| {
                COLOR_OR_TYPE_WORD_PATTERN.matches_word(word)
                    && ((COLOR_WORD_PATTERN.matches_word(word) && any_color)
                        || (TYPE_WORD_PATTERN.matches_word(word) && any_type))
            })
        })
        .map(|idx| idx + 1)
        .unwrap_or(tokens.len());
        let tail_tokens = trim_leading_commas(&tokens[phrase_end..]);

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
                player,
                amount,
                filter,
                allow_colorless,
                any_one,
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
        if FOR_EACH_REMOVED_THIS_WAY_PATTERN.matches_words(&tail_words)
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

        if tail_words
            .first()
            .is_some_and(|word| AMONG_WORD_PATTERN.matches_word(word))
        {
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

    let mut for_each_idx = None;
    let mut token_idx = 0usize;
    while token_idx + 1 < tokens.len() {
        if activation_token_slice_prefix_at_matches_shape(
            tokens,
            token_idx,
            &FOR_EACH_PREFIX_PATTERN,
        ) {
            for_each_idx = Some(token_idx);
            break;
        }
        token_idx += 1;
    }
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
            .is_some_and(|word| SIMPLE_MANA_FILLER_WORD_PATTERN.matches_word(word))
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
            let chosen_color_tail = CHOSEN_COLOR_MANA_TAIL_PATTERN.matches_words(&trailing_words);
            let pool_tail = if chosen_color_tail {
                trailing_words[7..].to_vec()
            } else {
                Vec::new()
            };
            let has_only_pool_tail = chosen_color_tail
                && (pool_tail.is_empty()
                    || pool_tail
                        .iter()
                        .all(|word| MANA_CHOICE_TAIL_WORD_PATTERN.matches_word(word)));
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
                .all(|word| MANA_CHOICE_TAIL_WORD_PATTERN.matches_word(word));
        let has_only_instead_tail = INSTEAD_TAIL_PATTERN.matches_words(&trailing_words);
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
    let words = token_word_refs(tokens);
    if activation_find_phrase_start(
        &words,
        FOR_EACH_COLOR_AMONG_PHRASE_LEN,
        &FOR_EACH_COLOR_AMONG_PHRASE_PATTERN,
    )
    .is_none()
        || activation_find_phrase_start(
            &words,
            ADD_ONE_MANA_OF_THAT_COLOR_PHRASE_LEN,
            &ADD_ONE_MANA_OF_THAT_COLOR_PHRASE_PATTERN,
        )
        .is_none()
    {
        return Ok(None);
    }

    let mut among_token_idx = None;
    let mut add_token_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if AMONG_WORD_PATTERN.matches_token(token) && among_token_idx.is_none() {
            among_token_idx = Some(idx);
        }
        if ADD_WORD_PATTERN.matches_token(token) && add_token_idx.is_none() {
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
    if !OR_MARKER_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let mut colors = Vec::new();
    let mut has_or = false;
    for token in tokens {
        if OR_WORD_PATTERN.matches_token(token) {
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
        if MANA_CHOICE_TAIL_WORD_PATTERN.matches_word(word) {
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
    let Some(combination_idx) = activation_find_phrase_start(
        &clause_words,
        ANY_COMBINATION_OF_PHRASE_LEN,
        &ANY_COMBINATION_OF_PHRASE_PATTERN,
    ) else {
        return Ok(None);
    };

    let mut colors = Vec::new();
    for word in &clause_words[combination_idx + 3..] {
        if WHERE_WORD_PATTERN.matches_word(word) {
            break;
        }
        if MANA_OPTION_SEPARATOR_WORD_PATTERN.matches_word(word) {
            continue;
        }
        if COLOR_OR_COLORS_WORD_PATTERN.matches_word(word) {
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
    if !MANA_POOL_START_PATTERN.matches_words(&words)
        || !SIMPLE_MANA_POOL_TAIL_PATTERN.matches_words(&words)
    {
        return false;
    }
    words
        .iter()
        .all(|word| MANA_POOL_TAIL_WORD_PATTERN.matches_word(word))
}

pub(crate) fn parse_land_could_produce_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words_view = TokenWordView::new(tokens);
    let words = words_view.to_word_refs();
    if words.len() < 3 || !LAND_PRODUCE_SUBJECT_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let marker_word_idx = if let Some(could_idx) = activation_find_phrase_start(
        &words,
        COULD_PRODUCE_PHRASE_LEN,
        &COULD_PRODUCE_PHRASE_PATTERN,
    ) {
        if could_idx + 2 != words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (tail: '{}')",
                words.join(" ")
            )));
        }
        could_idx
    } else {
        let mut produced_idx = None;
        let mut idx = 0usize;
        while idx < words.len() {
            if PRODUCED_WORD_PATTERN.matches_word(words[idx]) {
                produced_idx = Some(idx);
                break;
            }
            idx += 1;
        }
        let Some(produced_idx) = produced_idx else {
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

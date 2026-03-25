use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::TextSpan;
use crate::cards::builders::{
    AdditionalCostChoiceOptionAst, CardTextError, IT_TAG, KeywordAction, ParsedAbility, PlayerAst,
    ReferenceImports, TargetAst,
};
use crate::cost::OptionalCost;
use crate::cost::TotalCost;
use crate::costs::Cost;
use crate::effect::{Effect, EventValueSpec, Value};
use crate::filter::AlternativeCastKind;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::{StaticAbility, StaticAbilityId};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;
use crate::{ChoiceCount, PowerToughness, PtValue, TagKey};

use super::activation_and_restrictions::{parse_ability_phrase, parse_activation_cost};
use super::clause_support::parse_effect_sentences_lexed;
use super::effect_sentences::{find_verb, parse_subtype_word, parse_supertype_word};
use super::keyword_static::keyword_action_to_static_ability;
use super::keyword_static::parse_this_spell_cost_condition;
use super::lexer::{OwnedLexToken, TokenKind, lex_line};
use super::native_tokens::LowercaseWordView;
use super::object_filters::{parse_object_filter, split_on_or};

fn lowercase_word_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut lowered = tokens.to_vec();
    for token in &mut lowered {
        if let Some(word) = token.word_mut() {
            *word = word.to_ascii_lowercase();
        }
    }
    lowered
}

#[cfg(test)]
fn push_tokenized_words(
    slice: &str,
    span: TextSpan,
    in_mana_braces: bool,
    out: &mut Vec<OwnedLexToken>,
) {
    let mut buffer = String::new();
    let mut word_start: Option<usize> = None;
    let mut word_end = span.start;
    let chars: Vec<(usize, char)> = slice.char_indices().collect();

    let flush = |buffer: &mut String,
                 out: &mut Vec<OwnedLexToken>,
                 word_start: &mut Option<usize>,
                 word_end: &mut usize| {
        if !buffer.is_empty() {
            let start = word_start.unwrap_or(span.start);
            out.push(OwnedLexToken::word(
                buffer.clone(),
                TextSpan {
                    line: span.line,
                    start,
                    end: *word_end,
                },
            ));
            buffer.clear();
        }
        *word_start = None;
    };

    for (idx, (rel_idx, mut ch)) in chars.iter().copied().enumerate() {
        if ch == '−' {
            ch = '-';
        }
        let abs_idx = span.start + rel_idx;
        let prev = if idx > 0 { chars[idx - 1].1 } else { '\0' };
        let next = if idx + 1 < chars.len() {
            chars[idx + 1].1
        } else {
            '\0'
        };
        let is_counter_char = match ch {
            '+' | '-' => next.is_ascii_digit() || next == 'x' || next == 'X',
            '/' => {
                (prev.is_ascii_digit() || prev == 'x' || prev == 'X')
                    && (next.is_ascii_digit()
                        || next == '-'
                        || next == '+'
                        || next == 'x'
                        || next == 'X')
            }
            _ => false,
        };
        let is_mana_hybrid_slash = ch == '/' && in_mana_braces;

        if ch.is_ascii_alphanumeric() || is_counter_char || is_mana_hybrid_slash {
            if word_start.is_none() {
                word_start = Some(abs_idx);
            }
            word_end = abs_idx + ch.len_utf8();
            buffer.push(ch.to_ascii_lowercase());
            continue;
        }

        if matches!(ch, '\'' | '’' | '‘') {
            if word_start.is_some() {
                word_end = abs_idx + ch.len_utf8();
            }
            continue;
        }

        flush(&mut buffer, out, &mut word_start, &mut word_end);
    }

    flush(&mut buffer, out, &mut word_start, &mut word_end);
}

#[cfg(test)]
fn test_tokens_from_lexed(lexed: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut tokens = Vec::new();
    let mut idx = 0usize;
    while idx < lexed.len() {
        let token = &lexed[idx];
        match token.kind {
            TokenKind::Word => {
                push_tokenized_words(token.slice.as_str(), token.span, false, &mut tokens);
            }
            TokenKind::Tilde => tokens.push(OwnedLexToken::word("this", token.span)),
            TokenKind::ManaGroup => {
                let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
                if !inner.is_empty() {
                    push_tokenized_words(
                        inner,
                        TextSpan {
                            line: token.span.line,
                            start: token.span.start.saturating_add(1),
                            end: token.span.end.saturating_sub(1),
                        },
                        true,
                        &mut tokens,
                    );
                }
            }
            TokenKind::Dash
                if lexed
                    .get(idx + 1)
                    .is_some_and(|next| next.kind == TokenKind::Word)
                    && token.span.end == lexed[idx + 1].span.start =>
            {
                let next = &lexed[idx + 1];
                let combined = format!("-{}", next.slice);
                push_tokenized_words(
                    combined.as_str(),
                    TextSpan {
                        line: token.span.line,
                        start: token.span.start,
                        end: next.span.end,
                    },
                    false,
                    &mut tokens,
                );
                idx += 1;
            }
            TokenKind::Comma => tokens.push(OwnedLexToken::comma(token.span)),
            TokenKind::Period => tokens.push(OwnedLexToken::period(token.span)),
            TokenKind::Colon => tokens.push(OwnedLexToken::colon(token.span)),
            TokenKind::Semicolon => tokens.push(OwnedLexToken::semicolon(token.span)),
            TokenKind::Quote => {
                if matches!(token.slice.as_str(), "\"" | "“" | "”") {
                    tokens.push(OwnedLexToken::quote(token.span));
                }
            }
            TokenKind::Half => tokens.push(OwnedLexToken::word("1/2", token.span)),
            _ => {}
        }
        idx += 1;
    }

    tokens
}

#[cfg(test)]
pub(crate) fn tokenize_line(line: &str, line_index: usize) -> Vec<OwnedLexToken> {
    let lexed = lex_line(line, line_index).expect("test tokenization helper should lex input");
    test_tokens_from_lexed(&lexed)
}

pub(crate) fn words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    tokens.iter().filter_map(OwnedLexToken::as_word).collect()
}

pub(crate) fn is_article(word: &str) -> bool {
    matches!(word, "a" | "an" | "the")
}

fn strip_possessive_suffix(word: &str) -> &str {
    word.strip_suffix("'s")
        .or_else(|| word.strip_suffix("’s"))
        .or_else(|| word.strip_suffix("s'"))
        .or_else(|| word.strip_suffix("s’"))
        .unwrap_or(word)
}

const SENTENCE_HELPER_TAG_PREFIX: &str = "__sentence_helper_";

pub(crate) fn helper_tag_for_tokens(tokens: &[OwnedLexToken], prefix: &str) -> TagKey {
    let span = match (tokens.first(), tokens.last()) {
        (Some(first), Some(last)) => {
            let first_span = first.span();
            let last_span = last.span();
            TextSpan {
                line: first_span.line,
                start: first_span.start,
                end: last_span.end,
            }
        }
        _ => TextSpan {
            line: 0,
            start: 0,
            end: 0,
        },
    };

    TagKey::from(format!(
        "{SENTENCE_HELPER_TAG_PREFIX}{prefix}_l{}_s{}_e{}",
        span.line, span.start, span.end
    ))
}

pub(crate) fn is_sentence_helper_tag(tag: &str, prefix: &str) -> bool {
    let Some(rest) = tag.strip_prefix(SENTENCE_HELPER_TAG_PREFIX) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(prefix) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix("_l") else {
        return false;
    };
    let mut parts = rest.split("_s");
    let Some(line) = parts.next() else {
        return false;
    };
    let Some(rest) = parts.next() else {
        return false;
    };
    let mut parts = rest.split("_e");
    let Some(start) = parts.next() else {
        return false;
    };
    let Some(end) = parts.next() else {
        return false;
    };
    parts.next().is_none()
        && line.parse::<usize>().is_ok()
        && start.parse::<usize>().is_ok()
        && end.parse::<usize>().is_ok()
}

pub(crate) fn classify_instead_followup_text(
    text: &str,
) -> crate::cards::builders::InsteadSemantics {
    let normalized = text.to_ascii_lowercase();

    if !normalized.contains(" instead") {
        return crate::cards::builders::InsteadSemantics::NonReplacement;
    }

    if normalized.contains(" would ")
        || normalized.contains(" instead of ")
        || normalized.contains("the next time")
    {
        return crate::cards::builders::InsteadSemantics::FutureReplacement;
    }

    crate::cards::builders::InsteadSemantics::SelfReplacement
}

pub(crate) fn find_first_sacrifice_cost_choice_tag(mana_cost: &TotalCost) -> Option<TagKey> {
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.tag.as_str().starts_with("sacrifice_cost_") {
            return Some(choose.tag.clone());
        }
    }
    None
}

pub(crate) fn find_last_exile_cost_choice_tag(mana_cost: &TotalCost) -> Option<TagKey> {
    let mut found = None;
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.tag.as_str().starts_with("exile_cost_") {
            found = Some(choose.tag.clone());
        }
    }
    found
}

pub(crate) fn value_contains_unbound_x(value: &Value) -> bool {
    match value {
        Value::X | Value::XTimes(_) => true,
        Value::Scaled(value, _) => value_contains_unbound_x(value),
        Value::Add(left, right) => {
            value_contains_unbound_x(left) || value_contains_unbound_x(right)
        }
        _ => false,
    }
}

pub(crate) fn replace_unbound_x_with_value(
    value: Value,
    replacement: &Value,
    clause: &str,
) -> Result<Value, CardTextError> {
    let _ = clause;
    match value {
        Value::X => Ok(replacement.clone()),
        Value::XTimes(multiplier) => {
            if multiplier == 1 {
                return Ok(replacement.clone());
            }
            if let Value::Fixed(fixed) = replacement {
                return Ok(Value::Fixed(fixed * multiplier));
            }
            Ok(Value::Scaled(Box::new(replacement.clone()), multiplier))
        }
        Value::Scaled(value, multiplier) => Ok(Value::Scaled(
            Box::new(replace_unbound_x_with_value(*value, replacement, clause)?),
            multiplier,
        )),
        Value::Add(left, right) => Ok(Value::Add(
            Box::new(replace_unbound_x_with_value(*left, replacement, clause)?),
            Box::new(replace_unbound_x_with_value(*right, replacement, clause)?),
        )),
        other => Ok(other),
    }
}

pub(crate) fn starts_with_activation_cost(tokens: &[OwnedLexToken]) -> bool {
    let Some(first_token) = tokens.first() else {
        return false;
    };
    if mana_pips_from_token(first_token).is_some() {
        return true;
    }
    let Some(word) = first_token.as_word() else {
        return false;
    };
    if matches!(
        word,
        "tap"
            | "t"
            | "pay"
            | "discard"
            | "mill"
            | "sacrifice"
            | "put"
            | "remove"
            | "exile"
            | "return"
            | "e"
    ) {
        return true;
    }
    if word.contains('/') {
        return parse_mana_symbol_group(word).is_ok();
    }
    false
}

pub(crate) fn find_activation_cost_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    (0..tokens.len()).find(|idx| starts_with_activation_cost(&tokens[*idx..]))
}

pub(crate) fn contains_source_from_your_graveyard_phrase(words: &[&str]) -> bool {
    words.windows(5).any(|window| {
        (window[0] == "this" || window[0] == "thiss")
            && matches!(window[1], "card" | "creature" | "permanent")
            && window[2] == "from"
            && window[3] == "your"
            && window[4] == "graveyard"
    })
}

pub(crate) fn contains_source_from_your_hand_phrase(words: &[&str]) -> bool {
    words.windows(5).any(|window| {
        (window[0] == "this" || window[0] == "thiss")
            && matches!(window[1], "card" | "creature" | "permanent")
            && window[2] == "from"
            && window[3] == "your"
            && window[4] == "hand"
    }) || words.windows(4).any(|window| {
        (window[0] == "this" || window[0] == "thiss")
            && window[1] == "from"
            && window[2] == "your"
            && window[3] == "hand"
    })
}

pub(crate) fn contains_discard_source_phrase(words: &[&str]) -> bool {
    words
        .windows(3)
        .any(|window| window == ["discard", "this", "card"])
}

pub(crate) fn is_basic_color_word(word: &str) -> bool {
    matches!(
        word,
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}

pub(crate) fn join_sentences_with_period(sentences: &[Vec<OwnedLexToken>]) -> Vec<OwnedLexToken> {
    let mut joined = Vec::new();
    for (idx, sentence) in sentences.iter().enumerate() {
        if idx > 0 {
            joined.push(OwnedLexToken::period(TextSpan::synthetic()));
        }
        joined.extend(sentence.clone());
    }
    joined
}

pub(crate) fn split_cost_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token.is_comma() || token.is_word("and") {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn parse_next_end_step_token_delay_flags(tail_words: &[&str]) -> (bool, bool) {
    let has_beginning_of_end_step = tail_words
        .windows(6)
        .any(|window| window == ["beginning", "of", "the", "next", "end", "step"])
        || tail_words
            .windows(5)
            .any(|window| window == ["beginning", "of", "next", "end", "step"])
        || tail_words
            .windows(5)
            .any(|window| window == ["beginning", "of", "the", "end", "step"])
        || tail_words
            .windows(4)
            .any(|window| window == ["beginning", "of", "end", "step"]);
    if !has_beginning_of_end_step {
        return (false, false);
    }

    let has_sacrifice_reference = tail_words.contains(&"sacrifice")
        && (tail_words.contains(&"token")
            || tail_words.contains(&"tokens")
            || tail_words.contains(&"permanent")
            || tail_words.contains(&"permanents")
            || tail_words.contains(&"it")
            || tail_words.contains(&"them"));
    let has_exile_reference = tail_words.contains(&"exile")
        && (tail_words.contains(&"token")
            || tail_words.contains(&"tokens")
            || tail_words.contains(&"permanent")
            || tail_words.contains(&"permanents")
            || tail_words.contains(&"it")
            || tail_words.contains(&"them"));

    (has_sacrifice_reference, has_exile_reference)
}

pub(crate) fn token_index_for_word_index(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<usize> {
    LowercaseWordView::new(tokens).token_index_for_word_index(word_index)
}

pub(crate) fn trim_commas(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && tokens[start].is_comma() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_comma() {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

pub(crate) fn parser_stacktrace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_STACKTRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn parser_trace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_TRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn parser_trace(stage: &str, tokens: &[OwnedLexToken]) {
    if !parser_trace_enabled() {
        return;
    }
    eprintln!(
        "[parser-flow] stage={stage} clause='{}'",
        words(tokens).join(" ")
    );
}

pub(crate) fn parser_trace_stack(stage: &str, tokens: &[OwnedLexToken]) {
    if !parser_stacktrace_enabled() {
        return;
    }
    eprintln!(
        "[parser-trace] stage={stage} clause='{}'",
        words(tokens).join(" ")
    );
    eprintln!("{}", std::backtrace::Backtrace::force_capture());
}

pub(crate) fn split_on_period(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut quote_depth = 0u32;

    for token in tokens {
        if token.is_quote() {
            quote_depth = if quote_depth == 0 { 1 } else { 0 };
            current.push(token.clone());
        } else if token.is_period() && quote_depth == 0 {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn split_on_comma(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token.is_comma() {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn split_on_comma_or_semicolon(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token.is_comma() || token.is_semicolon() {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn starts_with_until_end_of_turn(words: &[&str]) -> bool {
    words.starts_with(&["until", "end", "of", "turn"])
}

pub(crate) fn is_until_end_of_turn(words: &[&str]) -> bool {
    words == ["until", "end", "of", "turn"]
}

pub(crate) fn ends_with_until_end_of_turn(words: &[&str]) -> bool {
    words.ends_with(&["until", "end", "of", "turn"])
}

pub(crate) fn contains_until_end_of_turn(words: &[&str]) -> bool {
    words.windows(4).any(is_until_end_of_turn)
}

pub(crate) fn is_untap_during_each_other_players_untap_step_words(words: &[&str]) -> bool {
    if words.first().copied() != Some("untap") {
        return false;
    }
    words.windows(6).any(|window| {
        window == ["during", "each", "other", "player", "untap", "step"]
            || window == ["during", "each", "other", "player's", "untap", "step"]
            || window == ["during", "each", "other", "players", "untap", "step"]
    }) || words
        .windows(7)
        .any(|window| window == ["during", "each", "other", "player", "s", "untap", "step"])
}

pub(crate) fn map_span_to_original(
    span: TextSpan,
    normalized_line: &str,
    original_line: &str,
    char_map: &[usize],
) -> TextSpan {
    fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
        if byte_idx == 0 {
            return 0;
        }
        let clamped = byte_idx.min(text.len());
        text[..clamped].chars().count()
    }

    let start_char = byte_to_char_index(normalized_line, span.start);
    let end_char = byte_to_char_index(normalized_line, span.end);
    if start_char >= char_map.len() {
        return span;
    }
    let start_orig = char_map[start_char];
    let end_orig = if end_char == 0 || end_char - 1 >= char_map.len() {
        start_orig
    } else {
        let last_char_idx = end_char - 1;
        let last_orig = char_map[last_char_idx];
        let last_len = original_line[last_orig..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(0);
        last_orig + last_len
    };

    TextSpan {
        line: span.line,
        start: start_orig,
        end: end_orig,
    }
}

pub(crate) fn split_on_and(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token.is_word("and") {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn parse_card_type(word: &str) -> Option<CardType> {
    match word {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

pub(crate) fn parse_mana_symbol_word_flexible(word: &str) -> Option<ManaSymbol> {
    match word {
        "white" => Some(ManaSymbol::White),
        "blue" => Some(ManaSymbol::Blue),
        "black" => Some(ManaSymbol::Black),
        "red" => Some(ManaSymbol::Red),
        "green" => Some(ManaSymbol::Green),
        "colorless" => Some(ManaSymbol::Colorless),
        _ => None,
    }
}

pub(crate) fn parse_color(word: &str) -> Option<crate::color::ColorSet> {
    crate::color::Color::from_name(word).map(crate::color::ColorSet::from_color)
}

pub(crate) fn parse_non_type(word: &str) -> Option<CardType> {
    let rest = word.strip_prefix("non")?;
    parse_card_type(rest)
}

pub(crate) fn parse_non_supertype(word: &str) -> Option<Supertype> {
    let rest = word.strip_prefix("non")?;
    parse_supertype_word(rest)
}

pub(crate) fn parse_non_color(word: &str) -> Option<crate::color::ColorSet> {
    let rest = word.strip_prefix("non")?;
    parse_color(rest)
}

pub(crate) fn parse_non_subtype(word: &str) -> Option<Subtype> {
    let rest = word.strip_prefix("non")?;
    parse_subtype_flexible(rest)
}

pub(crate) fn parse_subtype_flexible(word: &str) -> Option<Subtype> {
    parse_subtype_word(word)
        .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
        .or_else(|| {
            word.strip_suffix("ves")
                .and_then(|stem| parse_subtype_word(&format!("{stem}f")))
        })
        .or_else(|| {
            word.strip_suffix("ves")
                .and_then(|stem| parse_subtype_word(&format!("{stem}fe")))
        })
}

pub(crate) fn is_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    if words[0] != "this" && words[0] != "thiss" {
        return false;
    }

    if words.len() == 1 {
        return true;
    }

    if words.len() > 2 && words[1] == "of" {
        return true;
    }

    if words.len() != 2 {
        return false;
    }

    match words[1] {
        "source" | "spell" | "permanent" | "card" | "creature" => true,
        other => parse_card_type(other).is_some() || parse_subtype_flexible(other).is_some(),
    }
}

pub(crate) fn is_demonstrative_object_head(word: &str) -> bool {
    if matches!(
        word,
        "creature"
            | "creatures"
            | "permanent"
            | "permanents"
            | "card"
            | "cards"
            | "spell"
            | "spells"
            | "source"
            | "sources"
            | "token"
            | "tokens"
    ) {
        return true;
    }
    if parse_card_type(word).is_some() {
        return true;
    }
    if let Some(singular) = word.strip_suffix('s') {
        return parse_card_type(singular).is_some();
    }
    false
}

pub(crate) fn is_outlaw_word(word: &str) -> bool {
    matches!(word, "outlaw" | "outlaws")
}

pub(crate) fn is_non_outlaw_word(word: &str) -> bool {
    matches!(
        word,
        "nonoutlaw" | "nonoutlaws" | "non-outlaw" | "non-outlaws"
    )
}

pub(crate) fn push_outlaw_subtypes(out: &mut Vec<Subtype>) {
    for subtype in [
        Subtype::Assassin,
        Subtype::Mercenary,
        Subtype::Pirate,
        Subtype::Rogue,
        Subtype::Warlock,
    ] {
        if !out.contains(&subtype) {
            out.push(subtype);
        }
    }
}

pub(crate) fn is_permanent_type(card_type: CardType) -> bool {
    matches!(
        card_type,
        CardType::Artifact
            | CardType::Creature
            | CardType::Enchantment
            | CardType::Land
            | CardType::Planeswalker
            | CardType::Battle
    )
}

pub(crate) fn parse_zone_word(word: &str) -> Option<Zone> {
    match word {
        "battlefield" => Some(Zone::Battlefield),
        "graveyard" | "graveyards" => Some(Zone::Graveyard),
        "hand" | "hands" => Some(Zone::Hand),
        "library" | "libraries" => Some(Zone::Library),
        "exile" | "exiled" => Some(Zone::Exile),
        "stack" => Some(Zone::Stack),
        _ => None,
    }
}

pub(crate) fn parse_alternative_cast_words(words: &[&str]) -> Option<(AlternativeCastKind, usize)> {
    match words {
        ["flashback", ..] => Some((AlternativeCastKind::Flashback, 1)),
        ["jump", "start", ..] => Some((AlternativeCastKind::JumpStart, 2)),
        ["jumpstart", ..] => Some((AlternativeCastKind::JumpStart, 1)),
        ["escape", ..] => Some((AlternativeCastKind::Escape, 1)),
        ["madness", ..] => Some((AlternativeCastKind::Madness, 1)),
        ["miracle", ..] => Some((AlternativeCastKind::Miracle, 1)),
        _ => None,
    }
}

pub(crate) fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    let (power, toughness) = word.split_once('/')?;
    if power.starts_with('+')
        || toughness.starts_with('+')
        || power.starts_with('-')
        || toughness.starts_with('-')
    {
        return None;
    }
    let power = power.parse::<i32>().ok()?;
    let toughness = toughness.parse::<i32>().ok()?;
    Some((power, toughness))
}

pub(crate) fn intern_counter_name(word: &str) -> &'static str {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static INTERNER: OnceLock<Mutex<HashMap<String, &'static str>>> = OnceLock::new();

    let map = INTERNER.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = map.lock().expect("counter name interner lock poisoned");
    if let Some(existing) = map.get(word) {
        return *existing;
    }

    let leaked: &'static str = Box::leak(word.to_string().into_boxed_str());
    map.insert(word.to_string(), leaked);
    leaked
}

pub(crate) fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    match word {
        "+1/+1" => Some(CounterType::PlusOnePlusOne),
        "-1/-1" => Some(CounterType::MinusOneMinusOne),
        "-0/-1" => Some(CounterType::MinusOneMinusOne),
        "+1/+0" => Some(CounterType::PlusOnePlusZero),
        "+0/+1" => Some(CounterType::PlusZeroPlusOne),
        "+1/+2" => Some(CounterType::PlusOnePlusTwo),
        "+2/+2" => Some(CounterType::PlusTwoPlusTwo),
        "-0/-2" => Some(CounterType::MinusZeroMinusTwo),
        "-2/-2" => Some(CounterType::MinusTwoMinusTwo),
        "deathtouch" => Some(CounterType::Deathtouch),
        "flying" => Some(CounterType::Flying),
        "haste" => Some(CounterType::Haste),
        "hexproof" => Some(CounterType::Hexproof),
        "indestructible" => Some(CounterType::Indestructible),
        "lifelink" => Some(CounterType::Lifelink),
        "menace" => Some(CounterType::Menace),
        "reach" => Some(CounterType::Reach),
        "trample" => Some(CounterType::Trample),
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
        "stun" => Some(CounterType::Stun),
        "void" => Some(CounterType::Void),
        "depletion" => Some(CounterType::Depletion),
        "storage" => Some(CounterType::Storage),
        "ki" => Some(CounterType::Ki),
        "energy" => Some(CounterType::Energy),
        "age" => Some(CounterType::Age),
        "finality" => Some(CounterType::Finality),
        "time" => Some(CounterType::Time),
        "brain" => Some(CounterType::Brain),
        "burden" => Some(CounterType::Named(intern_counter_name("burden"))),
        "level" => Some(CounterType::Level),
        "lore" => Some(CounterType::Lore),
        "luck" => Some(CounterType::Luck),
        "oil" => Some(CounterType::Oil),
        _ => None,
    }
}

pub(crate) fn parse_counter_type_from_tokens(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    let token_word_view =
        crate::cards::builders::parser::native_tokens::LowercaseWordView::new(tokens);
    let token_words = token_word_view.to_word_refs();

    if let Some(counter_idx) = token_words
        .iter()
        .position(|word| *word == "counter" || *word == "counters")
    {
        if counter_idx == 0 {
            return None;
        }

        let prev = token_words[counter_idx - 1];
        if let Some(counter_type) = parse_counter_type_word(prev) {
            return Some(counter_type);
        }

        if prev == "strike" && counter_idx >= 2 {
            match token_words[counter_idx - 2] {
                "double" => return Some(CounterType::DoubleStrike),
                "first" => return Some(CounterType::FirstStrike),
                _ => {}
            }
        }

        if matches!(
            prev,
            "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six" | "another"
        ) {
            return None;
        }

        if prev.chars().all(|c| c.is_ascii_alphabetic()) {
            return Some(CounterType::Named(intern_counter_name(prev)));
        }
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterKeywordConstraint {
    Static(StaticAbilityId),
    Marker(&'static str),
}

fn keyword_action_to_filter_constraint(action: KeywordAction) -> Option<FilterKeywordConstraint> {
    use FilterKeywordConstraint::{Marker, Static};

    if let KeywordAction::Landwalk(kind) = action {
        let constraint = match kind {
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Island,
                snow: false,
            } => Marker("islandwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Swamp,
                snow: false,
            } => Marker("swampwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Mountain,
                snow: false,
            } => Marker("mountainwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Forest,
                snow: false,
            } => Marker("forestwalk"),
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Plains,
                snow: false,
            } => Marker("plainswalk"),
            _ => Static(StaticAbilityId::Landwalk),
        };
        return Some(constraint);
    }

    let static_id = keyword_action_to_static_ability(action)?.id();
    match static_id {
        StaticAbilityId::Flying
        | StaticAbilityId::Menace
        | StaticAbilityId::Hexproof
        | StaticAbilityId::Haste
        | StaticAbilityId::FirstStrike
        | StaticAbilityId::DoubleStrike
        | StaticAbilityId::Deathtouch
        | StaticAbilityId::Lifelink
        | StaticAbilityId::Vigilance
        | StaticAbilityId::Trample
        | StaticAbilityId::Reach
        | StaticAbilityId::Defender
        | StaticAbilityId::Flash
        | StaticAbilityId::Indestructible
        | StaticAbilityId::Shroud
        | StaticAbilityId::Wither
        | StaticAbilityId::Infect
        | StaticAbilityId::Fear
        | StaticAbilityId::Intimidate
        | StaticAbilityId::Shadow
        | StaticAbilityId::Horsemanship
        | StaticAbilityId::Flanking
        | StaticAbilityId::Changeling => Some(Static(static_id)),
        _ => None,
    }
}

pub(crate) fn parse_filter_keyword_constraint_words(
    words: &[&str],
) -> Option<(FilterKeywordConstraint, usize)> {
    if words.is_empty() {
        return None;
    }
    if words.len() >= 2 && words[0] == "mana" && matches!(words[1], "ability" | "abilities") {
        return Some((FilterKeywordConstraint::Marker("mana ability"), 2));
    }
    if words[0] == "cycling" || words[0].ends_with("cycling") {
        return Some((FilterKeywordConstraint::Marker("cycling"), 1));
    }
    if words.len() >= 2 && words[0] == "basic" && words[1] == "landcycling" {
        return Some((FilterKeywordConstraint::Marker("cycling"), 2));
    }

    let max_len = words.len().min(4);
    for len in (1..=max_len).rev() {
        let tokens = words[..len]
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let Some(action) = parse_ability_phrase(&tokens) else {
            continue;
        };
        if let Some(constraint) = keyword_action_to_filter_constraint(action) {
            return Some((constraint, len));
        }
    }
    None
}

pub(crate) fn parse_filter_counter_constraint_words(
    words: &[&str],
) -> Option<(crate::filter::CounterConstraint, usize)> {
    if words.len() < 3 {
        return None;
    }
    let counter_idx = words
        .iter()
        .position(|word| *word == "counter" || *word == "counters")?;
    if words.get(counter_idx + 1) != Some(&"on") {
        return None;
    }
    if !words
        .get(counter_idx + 2)
        .is_some_and(|word| matches!(*word, "it" | "them"))
    {
        return None;
    }

    let descriptor_words = words[..counter_idx]
        .iter()
        .copied()
        .filter(|word| !matches!(*word, "a" | "an" | "one" | "or" | "more"))
        .collect::<Vec<_>>();
    let consumed = counter_idx + 3;
    if descriptor_words.is_empty() {
        return Some((crate::filter::CounterConstraint::Any, consumed));
    }
    let descriptor_tokens = descriptor_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let counter_type = if descriptor_tokens.len() == 1 {
        parse_counter_type_word(descriptor_words[0])?
    } else {
        parse_counter_type_from_tokens(&descriptor_tokens)?
    };
    Some((
        crate::filter::CounterConstraint::Typed(counter_type),
        consumed,
    ))
}

pub(crate) fn apply_filter_keyword_constraint(
    filter: &mut ObjectFilter,
    constraint: FilterKeywordConstraint,
    excluded: bool,
) {
    match constraint {
        FilterKeywordConstraint::Static(ability_id) => {
            if excluded {
                if !filter.excluded_static_abilities.contains(&ability_id) {
                    filter.excluded_static_abilities.push(ability_id);
                }
            } else if !filter.static_abilities.contains(&ability_id) {
                filter.static_abilities.push(ability_id);
            }
        }
        FilterKeywordConstraint::Marker(marker) => {
            if excluded {
                if !filter
                    .excluded_ability_markers
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(marker))
                {
                    filter.excluded_ability_markers.push(marker.to_string());
                }
            } else if !filter
                .ability_markers
                .iter()
                .any(|value| value.eq_ignore_ascii_case(marker))
            {
                filter.ability_markers.push(marker.to_string());
            }
        }
    }
}

pub(crate) fn parse_flashback_keyword_line(tokens: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    let words_all = words(tokens);
    if words_all.first().copied() != Some("flashback") {
        return None;
    }
    let (cost, consumed) = leading_mana_symbols_to_oracle(&words_all[1..])?;
    let mut text = format!("Flashback {cost}");
    let tail = &words_all[1 + consumed..];
    if !tail.is_empty() {
        let mut tail_text = tail.join(" ");
        if let Some(first) = tail_text.chars().next() {
            let upper = first.to_ascii_uppercase().to_string();
            let rest = &tail_text[first.len_utf8()..];
            tail_text = format!("{upper}{rest}");
        }
        text.push_str(", ");
        text.push_str(&tail_text);
    }
    Some(vec![KeywordAction::MarkerText(text)])
}

pub(crate) fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    let upper = part.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return Err(CardTextError::ParseError("empty mana symbol".to_string()));
    }

    if upper.chars().all(|ch| ch.is_ascii_digit()) {
        let value = upper.parse::<u8>().map_err(|_| {
            CardTextError::ParseError(format!("invalid generic mana symbol '{part}'"))
        })?;
        return Ok(ManaSymbol::Generic(value));
    }

    match upper.as_str() {
        "W" => Ok(ManaSymbol::White),
        "U" => Ok(ManaSymbol::Blue),
        "B" => Ok(ManaSymbol::Black),
        "R" => Ok(ManaSymbol::Red),
        "G" => Ok(ManaSymbol::Green),
        "C" => Ok(ManaSymbol::Colorless),
        "S" => Ok(ManaSymbol::Snow),
        "X" => Ok(ManaSymbol::X),
        "P" => Ok(ManaSymbol::Life(2)),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported mana symbol '{part}'"
        ))),
    }
}

fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    raw.split('/').map(parse_mana_symbol).collect()
}

pub(crate) fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return Ok(ManaCost::new());
    }

    let mut pips = Vec::new();
    let mut current = String::new();
    let mut in_brace = false;

    for ch in trimmed.chars() {
        if ch == '{' {
            in_brace = true;
            current.clear();
            continue;
        }
        if ch == '}' {
            if !in_brace {
                continue;
            }
            in_brace = false;
            if current.is_empty() {
                continue;
            }
            let alternatives = parse_mana_symbol_group(&current)?;
            if !alternatives.is_empty() {
                pips.push(alternatives);
            }
            continue;
        }
        if in_brace {
            current.push(ch);
        }
    }

    Ok(ManaCost::from_pips(pips))
}

pub(crate) fn parse_number_or_x_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?.to_ascii_lowercase();

    if word == "x" {
        return Some((Value::X, 1));
    }

    if let Ok(value) = word.parse::<u32>() {
        return Some((Value::Fixed(value as i32), 1));
    }

    let value = match word.as_str() {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return None,
    };

    Some((Value::Fixed(value), 1))
}

pub(crate) fn parse_number_or_x_value_lexed(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    parse_number_or_x_value(tokens)
}

pub(crate) fn parse_number_word_i32(word: &str) -> Option<i32> {
    if let Ok(value) = word.parse::<i32>() {
        return Some(value);
    }

    match word {
        "zero" => Some(0),
        "a" | "an" | "one" => Some(1),
        "two" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        "eleven" => Some(11),
        "twelve" => Some(12),
        "thirteen" => Some(13),
        "fourteen" => Some(14),
        "fifteen" => Some(15),
        "sixteen" => Some(16),
        "seventeen" => Some(17),
        "eighteen" => Some(18),
        "nineteen" => Some(19),
        "twenty" => Some(20),
        _ => None,
    }
}

pub(crate) fn parse_number_word_u32(word: &str) -> Option<u32> {
    parse_number_word_i32(word).and_then(|value| value.try_into().ok())
}

fn parse_value_expr_term_words(words: &[&str]) -> Option<(Value, usize)> {
    if words.is_empty() {
        return None;
    }

    if matches!(
        words.get(..2),
        Some(["that", "many"]) | Some(["that", "much"]) | Some(["that", "amount"])
    ) {
        return Some((Value::EventValue(EventValueSpec::Amount), 2));
    }
    if matches!(
        words.get(..5),
        Some(["that", "amount", "of", "excess", "damage"])
    ) {
        return Some((Value::EventValue(EventValueSpec::Amount), 5));
    }
    if matches!(words.get(..4), Some(["that", "much", "excess", "damage"])) {
        return Some((Value::EventValue(EventValueSpec::Amount), 4));
    }
    if matches!(words.get(..2), Some(["damage", "dealt"])) {
        return Some((Value::EventValue(EventValueSpec::Amount), 2));
    }
    if matches!(words.get(..3), Some(["the", "damage", "dealt"])) {
        return Some((Value::EventValue(EventValueSpec::Amount), 3));
    }

    if words[0] == "x" {
        return Some((Value::X, 1));
    }

    if let Some(value) = parse_number_word_i32(words[0]) {
        return Some((Value::Fixed(value), 1));
    }

    if matches!(
        words.get(..2),
        Some(["its", "power"]) | Some(["this", "power"]) | Some(["thiss", "power"])
    ) {
        return Some((Value::SourcePower, 2));
    }
    if matches!(
        words.get(..3),
        Some(["this", "creature", "power"])
            | Some(["thiss", "creature", "power"])
            | Some(["this", "creatures", "power"])
            | Some(["thiss", "creatures", "power"])
    ) {
        return Some((Value::SourcePower, 3));
    }
    if matches!(
        words.get(..2),
        Some(["its", "toughness"]) | Some(["this", "toughness"]) | Some(["thiss", "toughness"])
    ) {
        return Some((Value::SourceToughness, 2));
    }
    if matches!(
        words.get(..3),
        Some(["this", "creature", "toughness"])
            | Some(["thiss", "creature", "toughness"])
            | Some(["this", "creatures", "toughness"])
            | Some(["thiss", "creatures", "toughness"])
    ) {
        return Some((Value::SourceToughness, 3));
    }
    if matches!(
        words.get(..3),
        Some(["its", "mana", "value"])
            | Some(["this", "mana", "value"])
            | Some(["thiss", "mana", "value"])
    ) {
        return Some((Value::ManaValueOf(Box::new(ChooseSpec::Source)), 3));
    }
    if matches!(
        words.get(..4),
        Some(["this", "creature", "mana", "value"])
            | Some(["thiss", "creature", "mana", "value"])
            | Some(["this", "creatures", "mana", "value"])
            | Some(["thiss", "creatures", "mana", "value"])
    ) {
        return Some((Value::ManaValueOf(Box::new(ChooseSpec::Source)), 4));
    }

    let mut idx = 0usize;
    if words[idx] == "the" {
        idx += 1;
    }
    if words.get(idx).copied() != Some("number") || words.get(idx + 1).copied() != Some("of") {
        return None;
    }
    idx += 2;

    let mut counter_idx = idx;
    if words
        .get(counter_idx)
        .is_some_and(|word| is_article(word) || *word == "one")
    {
        counter_idx += 1;
    }

    let mut parsed_counter_type = None;
    if let Some(word) = words.get(counter_idx).copied()
        && let Some(counter_type) = parse_counter_type_word(word)
    {
        parsed_counter_type = Some(counter_type);
        counter_idx += 1;
    }

    if matches!(
        words.get(counter_idx).copied(),
        Some("counter" | "counters")
    ) && words.get(counter_idx + 1).copied() == Some("on")
    {
        let reference_start = counter_idx + 2;
        let mut reference_end = reference_start;
        while reference_end < words.len() && !matches!(words[reference_end], "plus" | "minus") {
            reference_end += 1;
        }
        let reference = &words[reference_start..reference_end];
        if matches!(
            reference,
            ["it"]
                | ["this"]
                | ["this", "card"]
                | ["this", "creature"]
                | ["this", "permanent"]
                | ["this", "source"]
                | ["this", "artifact"]
                | ["this", "land"]
                | ["this", "enchantment"]
        ) {
            let value = match parsed_counter_type {
                Some(counter_type) => Value::CountersOnSource(counter_type),
                None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
            };
            return Some((value, reference_end));
        }
        if matches!(
            reference,
            ["that"]
                | ["that", "card"]
                | ["that", "creature"]
                | ["that", "permanent"]
                | ["that", "object"]
                | ["those"]
                | ["those", "cards"]
                | ["those", "creatures"]
                | ["those", "permanents"]
        ) {
            let value = Value::CountersOn(
                Box::new(ChooseSpec::Tagged(TagKey::from(
                    crate::cards::builders::IT_TAG,
                ))),
                parsed_counter_type,
            );
            return Some((value, reference_end));
        }
    }

    let filter_start = idx;
    let mut filter_end = filter_start;
    while filter_end < words.len() && !matches!(words[filter_end], "plus" | "minus") {
        filter_end += 1;
    }
    if filter_end <= filter_start {
        return None;
    }
    let filter_words = &words[filter_start..filter_end];
    if (filter_words.contains(&"spell") || filter_words.contains(&"spells"))
        && (filter_words.contains(&"cast") || filter_words.contains(&"casts"))
        && filter_words.contains(&"this")
        && filter_words.contains(&"turn")
    {
        let suffix_patterns: &[(&[&str], PlayerFilter)] = &[
            (
                &["theyve", "cast", "this", "turn"],
                PlayerFilter::IteratedPlayer,
            ),
            (
                &["they", "cast", "this", "turn"],
                PlayerFilter::IteratedPlayer,
            ),
            (
                &["that", "player", "cast", "this", "turn"],
                PlayerFilter::IteratedPlayer,
            ),
            (&["youve", "cast", "this", "turn"], PlayerFilter::You),
            (&["you", "cast", "this", "turn"], PlayerFilter::You),
            (
                &["an", "opponent", "has", "cast", "this", "turn"],
                PlayerFilter::Opponent,
            ),
            (
                &["opponent", "has", "cast", "this", "turn"],
                PlayerFilter::Opponent,
            ),
            (
                &["opponents", "have", "cast", "this", "turn"],
                PlayerFilter::Opponent,
            ),
            (&["cast", "this", "turn"], PlayerFilter::Any),
        ];
        for (suffix, player) in suffix_patterns {
            if !filter_words.ends_with(suffix) {
                continue;
            }
            let count_filter_tokens = filter_words
                [..filter_words.len().saturating_sub(suffix.len())]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            if let Ok(filter) = parse_object_filter(&count_filter_tokens, false) {
                let exclude_source = count_filter_tokens
                    .iter()
                    .any(|token| token.is_word("other"));
                return Some((
                    Value::SpellsCastThisTurnMatching {
                        player: player.clone(),
                        filter,
                        exclude_source,
                    },
                    filter_end,
                ));
            }
        }
    }
    let filter_tokens = words[filter_start..filter_end]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let filter = parse_object_filter(&filter_tokens, false).ok()?;
    Some((Value::Count(filter), filter_end))
}

pub(crate) fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
    let (mut value, mut used) = parse_value_expr_term_words(words)?;

    while used < words.len() {
        let operator = words[used];
        if !matches!(operator, "plus" | "minus") {
            break;
        }

        let (rhs, rhs_used) = parse_value_expr_term_words(&words[used + 1..])?;
        used += 1 + rhs_used;

        let rhs = if operator == "minus" {
            match rhs {
                Value::Fixed(fixed) => Value::Fixed(-fixed),
                _ => return None,
            }
        } else {
            rhs
        };

        value = Value::Add(Box::new(value), Box::new(rhs));
    }

    Some((value, used))
}

pub(crate) fn parse_value_expr(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let word_view = LowercaseWordView::new(tokens);
    let words = word_view.to_word_refs();
    let (value, used_words) = parse_value_expr_words(&words)?;
    let used = token_index_for_word_index(tokens, used_words).unwrap_or(tokens.len());
    Some((value, used))
}

pub(crate) fn parse_value(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    parse_value_expr(tokens)
}

fn is_that_player_or_that_objects_controller_phrase(words: &[&str]) -> bool {
    words.len() >= 6
        && words[0] == "that"
        && words[1] == "player"
        && words[2] == "or"
        && words[3] == "that"
        && matches!(
            words[4],
            "creatures" | "permanents" | "planeswalkers" | "sources" | "spells"
        )
        && words[5] == "controller"
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubjectAst {
    Player(PlayerAst),
    This,
}

pub(crate) fn parse_subject(tokens: &[OwnedLexToken]) -> SubjectAst {
    let word_view = LowercaseWordView::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return SubjectAst::This;
    }

    let mut start = 0usize;
    if words.starts_with(&["any", "number", "of"]) {
        start = 3;
    }

    let mut slice = &words[start..];
    while slice
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        slice = &slice[1..];
    }
    if let Some(have_idx) = slice
        .iter()
        .position(|word| *word == "have" || *word == "has")
    {
        if have_idx + 1 < slice.len() {
            slice = &slice[have_idx + 1..];
        }
    }

    if slice.starts_with(&["you"]) || slice.starts_with(&["your"]) {
        return SubjectAst::Player(PlayerAst::You);
    }

    if slice.starts_with(&["target", "opponent"]) || slice.starts_with(&["target", "opponents"]) {
        return SubjectAst::Player(PlayerAst::TargetOpponent);
    }

    if slice.starts_with(&["target", "player"]) || slice.starts_with(&["target", "players"]) {
        return SubjectAst::Player(PlayerAst::Target);
    }

    if slice.starts_with(&["opponent"])
        || slice.starts_with(&["opponents"])
        || slice.starts_with(&["an", "opponent"])
    {
        return SubjectAst::Player(PlayerAst::Opponent);
    }

    if slice.starts_with(&["defending", "player"]) || slice.ends_with(&["defending", "player"]) {
        return SubjectAst::Player(PlayerAst::Defending);
    }
    if slice.starts_with(&["attacking", "player"])
        || slice.starts_with(&["the", "attacking", "player"])
        || slice.ends_with(&["attacking", "player"])
    {
        return SubjectAst::Player(PlayerAst::Attacking);
    }

    if slice.starts_with(&["that", "player"]) || slice.starts_with(&["the", "player"]) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if slice.starts_with(&["the", "chosen", "player"])
        || slice.starts_with(&["chosen", "player"])
        || slice.starts_with(&["the", "chosen", "players"])
        || slice.starts_with(&["chosen", "players"])
    {
        return SubjectAst::Player(PlayerAst::Chosen);
    }

    if is_that_player_or_that_objects_controller_phrase(slice) {
        return SubjectAst::Player(PlayerAst::ThatPlayerOrTargetController);
    }

    if slice.starts_with(&["that", "players"]) || slice.starts_with(&["their"]) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if slice.starts_with(&["the", "owners", "of", "those", "cards"])
        || slice.starts_with(&["owners", "of", "those", "cards"])
        || slice.starts_with(&["the", "owners", "of", "those", "objects"])
        || slice.starts_with(&["owners", "of", "those", "objects"])
    {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }

    if slice.len() >= 3
        && slice[0] == "that"
        && (slice[2] == "controller" || slice[2] == "owner")
        && (slice[1] == "creatures"
            || slice[1] == "permanents"
            || slice[1] == "sources"
            || slice[1] == "spells")
    {
        let player = if slice[2] == "owner" {
            PlayerAst::ItsOwner
        } else {
            PlayerAst::ItsController
        };
        return SubjectAst::Player(player);
    }

    if slice.starts_with(&["its", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if slice.starts_with(&["its", "owner"]) || slice.starts_with(&["their", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if slice.ends_with(&["its", "controller"]) || slice.ends_with(&["their", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if slice.ends_with(&["its", "owner"]) || slice.ends_with(&["their", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }

    if slice.starts_with(&["this"]) || slice.starts_with(&["thiss"]) {
        return SubjectAst::This;
    }

    SubjectAst::This
}

pub(crate) fn span_from_tokens(tokens: &[OwnedLexToken]) -> Option<TextSpan> {
    let first = tokens.first()?;
    let last = tokens.last()?;
    let first_span = first.span();
    let last_span = last.span();
    Some(TextSpan {
        line: first_span.line,
        start: first_span.start,
        end: last_span.end,
    })
}

pub(crate) fn parse_number(tokens: &[OwnedLexToken]) -> Option<(u32, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?.to_ascii_lowercase();

    if let Ok(value) = word.parse::<u32>() {
        return Some((value, 1));
    }

    let value = match word.as_str() {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return None,
    };

    Some((value, 1))
}

pub(crate) fn parse_target_count_range_prefix(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, usize)> {
    let (first, first_used) = parse_number(tokens)?;
    let or_idx = first_used;
    if !tokens.get(or_idx).is_some_and(|token| token.is_word("or")) {
        return None;
    }
    let (second, second_used) = parse_number(&tokens[or_idx + 1..])?;
    if second < first {
        return None;
    }
    Some((
        ChoiceCount {
            min: first as usize,
            max: Some(second as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        },
        first_used + 1 + second_used,
    ))
}

pub(crate) fn wrap_target_count(target: TargetAst, target_count: Option<ChoiceCount>) -> TargetAst {
    if let Some(count) = target_count {
        TargetAst::WithCount(Box::new(target), count)
    } else {
        target
    }
}

fn choice_count_from_value(value: &Value, up_to: bool) -> ChoiceCount {
    match value {
        Value::X => {
            if up_to {
                ChoiceCount::up_to_dynamic_x()
            } else {
                ChoiceCount::dynamic_x()
            }
        }
        Value::Fixed(count) => {
            let count = (*count).max(0) as usize;
            if up_to {
                ChoiceCount::up_to(count)
            } else {
                ChoiceCount::exactly(count)
            }
        }
        other => unreachable!("unsupported target-count value {other:?}"),
    }
}

pub(crate) fn is_source_from_your_graveyard_words(words: &[&str]) -> bool {
    if words.len() < 4 {
        return false;
    }

    let starts_with_this = words[0] == "this" || words[0] == "thiss";
    let references_source_noun =
        words.contains(&"card") || words.contains(&"creature") || words.contains(&"permanent");

    starts_with_this
        && references_source_noun
        && words.contains(&"from")
        && words.contains(&"your")
        && words.contains(&"graveyard")
}

pub(crate) fn parse_target_phrase(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    let all_words = words(tokens);
    if matches!(
        all_words.as_slice(),
        ["up", "to", _, "target"]
            | ["up", "to", _, "targets"]
            | ["each", "of", "up", "to", _, "target"]
            | ["each", "of", "up", "to", _, "targets"]
    ) {
        let number_word = if all_words[0] == "each" {
            all_words[4]
        } else {
            all_words[2]
        };
        if let Some(count) = parse_number_word_u32(number_word) {
            return Ok(TargetAst::WithCount(
                Box::new(TargetAst::AnyTarget(span_from_tokens(tokens))),
                ChoiceCount::up_to(count as usize),
            ));
        }
    }

    match parse_target_phrase_inner(tokens) {
        Ok(target) => Ok(target),
        Err(err) => {
            if matches!(all_words.first().copied(), Some("during" | "if" | "until")) {
                for word_start in (1..all_words.len()).rev() {
                    let Some(token_start) = token_index_for_word_index(tokens, word_start) else {
                        continue;
                    };
                    let candidate = trim_commas(&tokens[token_start..]);
                    let candidate_words = words(&candidate);
                    if candidate_words.is_empty() {
                        continue;
                    }
                    if matches!(
                        candidate_words.first().copied(),
                        Some("and" | "during" | "if" | "then" | "until")
                    ) {
                        continue;
                    }
                    if let Ok(target) = parse_target_phrase_inner(&candidate) {
                        return Ok(target);
                    }
                }
            }
            Err(err)
        }
    }
}

fn parse_target_phrase_inner(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    let mut tokens = tokens;
    while tokens.first().is_some_and(|token| token.is_word("then")) {
        tokens = &tokens[1..];
    }
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target phrase".to_string(),
        ));
    }

    let mut random_choice = false;
    let token_word_view = LowercaseWordView::new(tokens);
    let token_words = token_word_view.to_word_refs();
    if token_words.contains(&"defending")
        && token_words.contains(&"player")
        && token_words.contains(&"choice")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported defending player's choice target phrase '{}'",
            token_words.join(" ")
        )));
    }
    if token_words.ends_with(&["chosen", "at", "random"])
        && let Some(random_idx) = token_word_view.token_index_for_word_index(token_words.len() - 3)
    {
        tokens = &tokens[..random_idx];
        random_choice = true;
    }

    let mut idx = 0;
    let mut other = false;
    let span = span_from_tokens(tokens);
    let mut target_count: Option<ChoiceCount> = None;
    let mut explicit_target = false;

    let all_words = words(tokens);
    if matches!(
        all_words.as_slice(),
        ["any"] | ["any", "target"] | ["any", "targets"]
    ) {
        return Ok(TargetAst::AnyTarget(span));
    }
    if matches!(
        all_words.as_slice(),
        ["any", "other"] | ["any", "other", "target"] | ["any", "other", "targets"]
    ) {
        return Ok(TargetAst::AnyOtherTarget(span));
    }
    if all_words.starts_with(&["up", "to"])
        && matches!(all_words.last().copied(), Some("target") | Some("targets"))
        && let Some((value, _)) = parse_number_or_x_value(&tokens[2..])
    {
        let target_words = words(&tokens[3..]);
        let target = if matches!(
            target_words.as_slice(),
            ["other", "target"] | ["other", "targets"]
        ) {
            TargetAst::AnyOtherTarget(span)
        } else {
            TargetAst::AnyTarget(span)
        };
        return Ok(TargetAst::WithCount(
            Box::new(target),
            choice_count_from_value(&value, true),
        ));
    }
    if all_words
        .first()
        .is_some_and(|word| matches!(*word, "it" | "them"))
        && all_words.get(1).is_some_and(|word| *word == "with")
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[2..])
        && consumed == all_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            target_count,
        ));
    }
    if all_words.as_slice() == ["that", "permanent"] || all_words.as_slice() == ["that", "creature"]
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }

    let remaining_words: Vec<&str> = all_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if remaining_words.len() >= 2
        && remaining_words[0] == "chosen"
        && is_demonstrative_object_head(remaining_words[1])
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["equipped", "creature"]
        || remaining_words.as_slice() == ["equipped", "creatures"]
        || remaining_words.as_slice() == ["enchanted", "creature"]
        || remaining_words.as_slice() == ["enchanted", "creatures"]
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if matches!(
        remaining_words.as_slice(),
        [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell",
            "additional",
            "cost"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell's",
            "additional",
            "cost"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell’s",
            "additional",
            "cost"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell's",
            "additional",
            "costs"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell’s",
            "additional",
            "costs"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell",
            "s",
            "additional",
            "cost"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell",
            "s",
            "additional",
            "costs"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spells",
            "additional",
            "cost"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spell",
            "additional",
            "costs"
        ] | [
            "creature",
            "tapped",
            "to",
            "pay",
            "this",
            "spells",
            "additional",
            "costs"
        ]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("tap_cost_0"), span),
            target_count,
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        target_count = Some(ChoiceCount::any_number());
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((value, used)) = parse_number_or_x_value(&tokens[idx..]) {
            target_count = Some(choice_count_from_value(&value, true));
            idx += used;
        } else {
            let next_word = tokens
                .get(idx)
                .and_then(OwnedLexToken::as_word)
                .unwrap_or("?");
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic or missing target count after 'up to' (found '{next_word}' in clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    } else if let Some((count, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        target_count = Some(count);
        idx += used;
    } else if let Some((value, used)) = parse_number_or_x_value(&tokens[idx..]) {
        let next_is_target = tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"));
        let next_is_other_target = tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("other"))
            && tokens
                .get(idx + used + 1)
                .is_some_and(|token| token.is_word("target"));
        let mut object_selector_idx = idx + used;
        while tokens
            .get(object_selector_idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| {
                matches!(
                    word,
                    "tapped"
                        | "untapped"
                        | "attacking"
                        | "nonattacking"
                        | "blocked"
                        | "unblocked"
                        | "blocking"
                        | "nonblocking"
                        | "non"
                        | "other"
                        | "another"
                        | "nonartifact"
                        | "noncreature"
                        | "nonland"
                        | "nontoken"
                        | "legendary"
                        | "basic"
                )
            })
        {
            object_selector_idx += 1;
        }
        let next_is_object_selector = tokens
            .get(object_selector_idx)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| {
                matches!(
                    word,
                    "card"
                        | "cards"
                        | "permanent"
                        | "permanents"
                        | "creature"
                        | "creatures"
                        | "spell"
                        | "spells"
                        | "source"
                        | "sources"
                        | "token"
                        | "tokens"
                ) || parse_card_type(word).is_some()
                    || parse_non_type(word).is_some()
                    || parse_subtype_word(word).is_some()
                    || word
                        .strip_suffix('s')
                        .and_then(parse_subtype_word)
                        .is_some()
            });
        if next_is_target || next_is_other_target || next_is_object_selector {
            target_count = Some(choice_count_from_value(&value, false));
            idx += used;
        }
    }

    if random_choice {
        target_count = Some(target_count.unwrap_or_default().at_random());
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    while tokens
        .get(idx)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(is_article)
    {
        idx += 1;
    }

    let mut saw_top_prefix = false;
    if tokens.get(idx).is_some_and(|token| token.is_word("top")) {
        saw_top_prefix = true;
        let count_idx = idx + 1;

        if let Some((value, used)) = parse_number_or_x_value(&tokens[count_idx..]) {
            let mut object_selector_idx = count_idx + used;
            while tokens
                .get(object_selector_idx)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| {
                    matches!(
                        word,
                        "tapped"
                            | "untapped"
                            | "attacking"
                            | "nonattacking"
                            | "blocked"
                            | "unblocked"
                            | "blocking"
                            | "nonblocking"
                            | "non"
                            | "other"
                            | "another"
                            | "nonartifact"
                            | "noncreature"
                            | "nonland"
                            | "nontoken"
                            | "legendary"
                            | "basic"
                    )
                })
            {
                object_selector_idx += 1;
            }
            let next_is_object_selector = tokens
                .get(object_selector_idx)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| {
                    matches!(
                        word,
                        "card"
                            | "cards"
                            | "permanent"
                            | "permanents"
                            | "creature"
                            | "creatures"
                            | "spell"
                            | "spells"
                            | "source"
                            | "sources"
                            | "token"
                            | "tokens"
                    ) || parse_card_type(word).is_some()
                        || parse_non_type(word).is_some()
                        || parse_subtype_word(word).is_some()
                        || word
                            .strip_suffix('s')
                            .and_then(parse_subtype_word)
                            .is_some()
                });
            if next_is_object_selector {
                target_count = Some(choice_count_from_value(&value, false));
                idx = count_idx + used;
            }
        }
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("other"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        other = true;
        explicit_target = true;
        idx += 2;
    } else {
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            idx += 1;
        }

        if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
            explicit_target = true;
            idx += 1;
        }
    }

    if let Some(ordinal_word) = tokens.get(idx).and_then(OwnedLexToken::as_word)
        && matches!(
            ordinal_word,
            "first"
                | "second"
                | "third"
                | "fourth"
                | "fifth"
                | "sixth"
                | "seventh"
                | "eighth"
                | "ninth"
                | "tenth"
        )
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        if ordinal_word != "first" {
            other = true;
        }
        explicit_target = true;
        idx += 2;
    }

    let words_all = words(&tokens[idx..]);
    if words_all.as_slice() == ["any", "target"] {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }
    if words_all.as_slice() == ["any", "other", "target"] {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = words(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let target_span = if explicit_target { span } else { None };

    if remaining_words.is_empty() && explicit_target {
        return Ok(wrap_target_count(
            if other {
                TargetAst::AnyOtherTarget(span)
            } else {
                TargetAst::AnyTarget(span)
            },
            target_count,
        ));
    }
    if other && matches!(remaining_words.as_slice(), ["target"] | ["targets"]) {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }
    if matches!(remaining_words.as_slice(), ["target"] | ["targets"]) {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let bare_top_library_shorthand = saw_top_prefix
        && !remaining_words.contains(&"library")
        && (matches!(remaining_words.as_slice(), ["top", "card"] | ["card"])
            || (target_count.is_some() && matches!(remaining_words.as_slice(), ["cards"])));
    if bare_top_library_shorthand {
        let mut filter = ObjectFilter::default().in_zone(Zone::Library);
        filter.owner = Some(PlayerFilter::You);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["player", "on", "your", "team"]
        || remaining_words.as_slice() == ["players", "on", "your", "team"]
    {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }
    if other
        && (remaining_words.as_slice() == ["player"] || remaining_words.as_slice() == ["players"])
    {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::NotYou, target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["player"] || remaining_words.as_slice() == ["players"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Any, target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["enchanted", "player"]
        || remaining_words.as_slice() == ["enchanted", "players"]
    {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::TaggedPlayer(TagKey::from("enchanted")), target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["that", "player"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["chosen", "player"]
        || remaining_words.as_slice() == ["chosen", "players"]
    {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::ChosenPlayer, target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["that", "opponent"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_opponent(), target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["defending", "player"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Defending, target_span),
            target_count,
        ));
    }
    let second_word_is_object_head = remaining_words.get(1).is_some_and(|word| {
        let normalized = strip_possessive_suffix(word);
        matches!(
            normalized,
            "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "spell"
                | "spells"
                | "source"
                | "sources"
                | "card"
                | "cards"
        ) || parse_card_type(normalized).is_some()
            || normalized
                .strip_suffix('s')
                .is_some_and(|singular| parse_card_type(singular).is_some())
    });
    if remaining_words.len() >= 3
        && remaining_words[0] == "that"
        && second_word_is_object_head
        && matches!(
            remaining_words[2],
            "controller" | "controllers" | "owner" | "owners"
        )
    {
        let player = if remaining_words[2].starts_with("owner") {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        } else {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        };
        return Ok(wrap_target_count(
            TargetAst::Player(player, target_span),
            target_count,
        ));
    }
    if remaining_words.len() >= 5
        && remaining_words[0] == "that"
        && second_word_is_object_head
        && remaining_words[2] == "or"
        && is_demonstrative_object_head(remaining_words[3])
        && matches!(
            remaining_words[4],
            "controller" | "controllers" | "owner" | "owners"
        )
    {
        let player = if remaining_words[4].starts_with("owner") {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        } else {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        };
        return Ok(wrap_target_count(
            TargetAst::Player(player, target_span),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["its", "controller"])
        || remaining_words.starts_with(&["its", "controllers"])
        || remaining_words.starts_with(&["their", "controller"])
        || remaining_words.starts_with(&["their", "controllers"])
    {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["its", "owner"])
        || remaining_words.starts_with(&["its", "owners"])
        || remaining_words.starts_with(&["their", "owner"])
        || remaining_words.starts_with(&["their", "owners"])
    {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["you"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["opponent"] || remaining_words.as_slice() == ["opponents"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["spell"] || remaining_words.as_slice() == ["spells"] {
        return Ok(wrap_target_count(
            TargetAst::Spell(target_span),
            target_count,
        ));
    }

    if remaining_words
        .first()
        .is_some_and(|word| matches!(*word, "it" | "them"))
        && remaining_words.get(1).is_some_and(|word| *word == "with")
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, span),
            target_count,
        ));
    }

    if is_source_reference_words(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Source(target_span),
            target_count,
        ));
    }
    if is_source_from_your_graveyard_words(&remaining_words) {
        let mut source_filter = ObjectFilter::source().in_zone(Zone::Graveyard);
        source_filter.owner = Some(PlayerFilter::You);
        return Ok(wrap_target_count(
            TargetAst::Object(source_filter, target_span, None),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["thiss", "power", "and", "toughness"])
        || remaining_words.starts_with(&["this", "power", "and", "toughness"])
        || remaining_words.as_slice() == ["thiss", "power"]
        || remaining_words.as_slice() == ["this", "power"]
        || remaining_words.as_slice() == ["thiss", "toughness"]
        || remaining_words.as_slice() == ["this", "toughness"]
        || remaining_words.as_slice() == ["thiss", "base", "power", "and", "toughness"]
        || remaining_words.as_slice() == ["this", "base", "power", "and", "toughness"]
    {
        return Ok(wrap_target_count(
            TargetAst::Source(target_span),
            target_count,
        ));
    }

    if remaining_words.first().is_some_and(|word| *word == "it")
        && remaining_words
            .iter()
            .skip(1)
            .all(|word| *word == "instead" || *word == "this" || *word == "way")
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["itself"] {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if matches!(
        remaining_words.as_slice(),
        ["them"] | ["him"] | ["her"] | ["that", "player"]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }

    let attacking_you_or_your_planeswalker = matches!(
        remaining_words.as_slice(),
        [
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control"
        ] | [
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls"
        ] | [
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control"
        ] | [
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls"
        ] | [
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ] | [
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ]
    );
    if attacking_you_or_your_planeswalker {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.card_types.push(CardType::Creature);
        filter.attacking = true;
        filter.controller = Some(PlayerFilter::Opponent);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    let opponent_or_planeswalker = matches!(
        remaining_words.as_slice(),
        ["opponent", "or", "planeswalker"]
            | ["opponents", "or", "planeswalkers"]
            | ["planeswalker", "or", "opponent"]
            | ["planeswalkers", "or", "opponents"]
    );
    if opponent_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let player_or_planeswalker_its_attacking = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["player", "or", "planeswalker"]
                | ["players", "or", "planeswalkers"]
                | ["planeswalker", "or", "player"]
                | ["planeswalkers", "or", "players"]
        )
    }) && remaining_words.contains(&"attacking")
        && (remaining_words.contains(&"its")
            || remaining_words.contains(&"it")
            || remaining_words.contains(&"thats")
            || remaining_words.contains(&"that"));
    if player_or_planeswalker_its_attacking {
        return Ok(wrap_target_count(
            TargetAst::AttackedPlayerOrPlaneswalker(target_span),
            target_count,
        ));
    }

    let player_or_planeswalker = matches!(
        remaining_words.as_slice(),
        ["player", "or", "planeswalker"]
            | ["players", "or", "planeswalkers"]
            | ["planeswalker", "or", "player"]
            | ["planeswalkers", "or", "players"]
    );
    if player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if matches!(
        remaining_words.as_slice(),
        ["permanent", "or", "player"]
            | ["permanents", "or", "players"]
            | ["player", "or", "permanent"]
            | ["players", "or", "permanents"]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }

    let creature_or_player = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["creature", "or", "player"]
                | ["creatures", "or", "players"]
                | ["player", "or", "creature"]
                | ["players", "or", "creatures"]
                | ["creature", "and", "player"]
                | ["creatures", "and", "players"]
                | ["player", "and", "creature"]
                | ["players", "and", "creatures"]
                | ["creature", "and/or", "player"]
                | ["creatures", "and/or", "players"]
                | ["player", "and/or", "creature"]
                | ["players", "and/or", "creatures"]
        )
    }) || remaining_words.windows(4).any(|window| {
        matches!(
            window,
            ["creature", "and", "or", "player"]
                | ["creatures", "and", "or", "players"]
                | ["player", "and", "or", "creature"]
                | ["players", "and", "or", "creatures"]
        )
    });
    if creature_or_player {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let mixed_object_player_target = remaining_words.contains(&"player")
        && remaining_words.contains(&"planeswalker")
        && remaining_words.contains(&"token");
    if mixed_object_player_target {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-token/player/planeswalker target phrase (clause: '{}')",
            remaining_words.join(" ")
        )));
    }

    let mut filter = parse_object_filter(remaining, other)?;
    if filter.with_counter.is_none()
        && remaining_words
            .first()
            .is_some_and(|word| matches!(*word, "it" | "them"))
        && remaining_words.get(1).is_some_and(|word| *word == "with")
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        filter.with_counter = Some(counter_constraint);
    }
    let it_span = if filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        tokens
            .iter()
            .rev()
            .find(|token| token.is_word("it"))
            .map(OwnedLexToken::span)
    } else {
        None
    };
    Ok(wrap_target_count(
        TargetAst::Object(filter, target_span, it_span),
        target_count,
    ))
}

pub(crate) fn parse_saga_chapter_prefix(line: &str) -> Option<(Vec<u32>, &str)> {
    let (prefix, rest) = line.split_once('—').or_else(|| line.split_once(" - "))?;

    let mut chapters = Vec::new();
    for part in prefix.split(',') {
        let roman = part.trim();
        if roman.is_empty() {
            continue;
        }
        chapters.push(roman_to_int(roman)?);
    }

    (!chapters.is_empty()).then_some((chapters, rest.trim()))
}

fn roman_to_int(roman: &str) -> Option<u32> {
    match roman {
        "i" => Some(1),
        "ii" => Some(2),
        "iii" => Some(3),
        "iv" => Some(4),
        "v" => Some(5),
        "vi" => Some(6),
        _ => None,
    }
}

pub(crate) fn parse_level_header(line: &str) -> Option<(u32, Option<u32>)> {
    let lower = line.trim().to_ascii_lowercase();
    let rest = lower.strip_prefix("level ")?;
    let token = rest.split_whitespace().next()?;
    if let Some(without_plus) = token.strip_suffix('+') {
        let min = without_plus.parse::<u32>().ok()?;
        return Some((min, None));
    }
    if let Some((start, end)) = token.split_once('-') {
        let min = start.parse::<u32>().ok()?;
        let max = end.parse::<u32>().ok()?;
        return Some((min, Some(max)));
    }
    let value = token.parse::<u32>().ok()?;
    Some((value, Some(value)))
}

pub(crate) fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    let trimmed = raw.trim();
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let power = parse_pt_value(parts[0].trim())?;
    let toughness = parse_pt_value(parts[1].trim())?;
    Some(PowerToughness::new(power, toughness))
}

fn parse_pt_value(raw: &str) -> Option<PtValue> {
    if raw == ".5" || raw == "0.5" {
        return Some(PtValue::Fixed(0));
    }
    if raw == "*" {
        return Some(PtValue::Star);
    }
    if let Some(stripped) = raw.strip_prefix("*+") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Some(stripped) = raw.strip_suffix("+*") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    raw.parse::<i32>().ok().map(PtValue::Fixed)
}

pub(crate) fn parse_level_up_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_view = LowercaseWordView::new(tokens);
    if !word_view.starts_with(&["level", "up"]) {
        return Ok(None);
    }

    let (mana_cost, _) = leading_mana_cost_from_tokens(tokens.get(2..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("level up missing mana cost".to_string()))?;
    let level_up_text = format!("Level up {}", mana_cost.to_oracle());

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(mana_cost),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::put_counters_on_source(CounterType::Level, 1),
                ]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(level_up_text),
        },
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_level_up_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_level_up_line(tokens)
}

pub(crate) fn preserve_keyword_prefix_for_parse(prefix: &str) -> bool {
    let words: Vec<&str> = prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    let Some(first) = words.first().copied() else {
        return false;
    };

    matches!(
        first,
        "buyback"
            | "bestow"
            | "cycling"
            | "echo"
            | "equip"
            | "escape"
            | "flashback"
            | "boast"
            | "modular"
            | "replicate"
            | "reinforce"
            | "renew"
            | "spectacle"
            | "strive"
            | "surge"
            | "suspend"
            | "ward"
    )
}

pub(crate) fn parse_self_free_cast_alternative_cost_line(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    let is_self_free_cast_clause = clause_words
        == [
            "you", "may", "cast", "this", "spell", "without", "paying", "its", "mana", "cost",
        ]
        || clause_words
            == [
                "you", "may", "cast", "this", "spell", "without", "paying", "this", "spells",
                "mana", "cost",
            ];
    if !is_self_free_cast_clause {
        return None;
    }
    Some(AlternativeCastingMethod::alternative_cost(
        "Parsed alternative cost",
        None,
        Vec::new(),
    ))
}

pub(crate) fn parse_self_free_cast_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
) -> Option<AlternativeCastingMethod> {
    parse_self_free_cast_alternative_cost_line(tokens)
}

fn leading_mana_symbols_to_oracle(words_all: &[&str]) -> Option<(String, usize)> {
    let mut symbols = Vec::new();
    let mut consumed = 0usize;
    for word in words_all {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        symbols.push(symbol);
        consumed += 1;
    }
    if symbols.is_empty() {
        return None;
    }
    Some((ManaCost::from_symbols(symbols).to_oracle(), consumed))
}

pub(crate) fn mana_pips_from_token(token: &OwnedLexToken) -> Option<Vec<ManaSymbol>> {
    match token.kind {
        TokenKind::Word => parse_mana_symbol(token.slice.as_str())
            .ok()
            .map(|symbol| vec![symbol]),
        TokenKind::ManaGroup => {
            let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
            if inner.is_empty() {
                return None;
            }
            parse_mana_symbol_group(inner)
                .ok()
                .filter(|group| !group.is_empty())
        }
        _ => None,
    }
}

pub(crate) fn leading_mana_cost_from_tokens(tokens: &[OwnedLexToken]) -> Option<(ManaCost, usize)> {
    let mut pips = Vec::new();
    let mut consumed = 0usize;
    for token in tokens {
        let Some(group) = mana_pips_from_token(token) else {
            break;
        };
        pips.push(group);
        consumed += 1;
    }
    if pips.is_empty() {
        return None;
    }
    Some((ManaCost::from_pips(pips), consumed))
}

pub(crate) fn parse_madness_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("madness")) {
        return Ok(None);
    }

    let cost_tokens = tokens.get(1..).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let cost_end = cost_tokens
        .iter()
        .position(|token| token.is_comma())
        .unwrap_or(cost_tokens.len());
    let cost_tokens = &cost_tokens[..cost_end];
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(cost_tokens)?;
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("madness keyword missing mana symbols".to_string())
    })?;

    Ok(Some(AlternativeCastingMethod::Madness { cost: mana_cost }))
}

pub(crate) fn parse_madness_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_madness_line(tokens)
}

pub(crate) fn parse_buyback_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("buyback")) {
        return Ok(None);
    }

    if tokens.get(1).is_some_and(|token| token.is_word("costs")) {
        return Ok(None);
    }

    let tail = tokens.get(1..).unwrap_or_default();
    if tail.is_empty() {
        return Err(CardTextError::ParseError(
            "buyback keyword missing cost".to_string(),
        ));
    }

    let reminder_start = tail
        .windows(3)
        .position(|window| {
            window[0].is_word("you") && window[1].is_word("may") && window[2].is_word("pay")
        })
        .or_else(|| {
            tail.windows(2)
                .position(|window| window[0].is_word("you") && window[1].is_word("may"))
        })
        .unwrap_or(tail.len());
    let cost_tokens = trim_commas(&tail[..reminder_start]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "buyback keyword missing cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(&cost_tokens)?;
    Ok(Some(OptionalCost::buyback(total_cost)))
}

pub(crate) fn parse_buyback_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_buyback_line(tokens)
}

pub(crate) fn parse_bargain_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    let clause_view = LowercaseWordView::new(tokens);
    let clause_words = clause_view.to_word_refs();
    if clause_words.first().copied() != Some("bargain") {
        return Ok(None);
    }

    let filter = crate::target::ObjectFilter {
        zone: Some(crate::zone::Zone::Battlefield),
        controller: Some(crate::target::PlayerFilter::You),
        any_of: vec![
            crate::target::ObjectFilter::artifact(),
            crate::target::ObjectFilter::enchantment(),
            crate::target::ObjectFilter::default().token(),
        ],
        ..Default::default()
    };

    Ok(Some(OptionalCost::custom(
        "Bargain",
        TotalCost::from_cost(Cost::sacrifice(filter)),
    )))
}

pub(crate) fn parse_bargain_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_bargain_line(tokens)
}

pub(crate) fn parse_optional_cost_keyword_line(
    tokens: &[OwnedLexToken],
    keyword: &str,
    constructor: fn(TotalCost) -> OptionalCost,
) -> Result<Option<OptionalCost>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word(keyword)) {
        return Ok(None);
    }

    let tail = tokens.get(1..).unwrap_or_default();
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} keyword missing cost"
        )));
    }

    let reminder_start = tail
        .windows(3)
        .position(|window| {
            window[0].is_word("you") && window[1].is_word("may") && window[2].is_word("pay")
        })
        .or_else(|| {
            tail.windows(2)
                .position(|window| window[0].is_word("you") && window[1].is_word("may"))
        })
        .unwrap_or(tail.len());
    let sentence_end = tail
        .iter()
        .position(|token| token.is_period())
        .unwrap_or(tail.len());
    let end = reminder_start.min(sentence_end);
    let cost_tokens = trim_commas(&tail[..end]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} keyword missing cost"
        )));
    }

    let total_cost = parse_activation_cost(&cost_tokens)?;
    Ok(Some(constructor(total_cost)))
}

pub(crate) fn parse_kicker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "kicker", OptionalCost::kicker)
}

pub(crate) fn parse_kicker_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_kicker_line(tokens)
}

pub(crate) fn parse_multikicker_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "multikicker", OptionalCost::multikicker)
}

pub(crate) fn parse_multikicker_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_multikicker_line(tokens)
}

pub(crate) fn parse_squad_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "squad", OptionalCost::squad)
}

pub(crate) fn parse_squad_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_squad_line(tokens)
}

pub(crate) fn parse_offspring_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "offspring", OptionalCost::offspring)
}

pub(crate) fn parse_offspring_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_offspring_line(tokens)
}

pub(crate) fn parse_entwine_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "entwine", OptionalCost::entwine)
}

pub(crate) fn parse_entwine_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_entwine_line(tokens)
}

pub(crate) fn parse_morph_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_view = LowercaseWordView::new(tokens);
    let Some(first_word) = word_view.first() else {
        return Ok(None);
    };

    let is_megamorph = match first_word {
        "morph" => false,
        "megamorph" => true,
        _ => return Ok(None),
    };

    let Some((cost, consumed_cost_tokens)) =
        leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
    else {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "{mechanic} keyword missing mana cost"
        )));
    };
    let consumed = 1 + consumed_cost_tokens;

    let trailing_view = LowercaseWordView::new(tokens.get(consumed..).unwrap_or_default());
    let trailing_words = trailing_view.to_word_refs();
    if !trailing_words.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing {mechanic} clause (line: '{}')",
            trailing_words.join(" ")
        )));
    }

    let label = if is_megamorph { "Megamorph" } else { "Morph" };
    let text = format!("{label} {}", cost.to_oracle());
    let static_ability = if is_megamorph {
        StaticAbility::megamorph(cost)
    } else {
        StaticAbility::morph(cost)
    };

    Ok(Some(ParsedAbility {
        ability: Ability::static_ability(static_ability).with_text(&text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_morph_keyword_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_morph_keyword_line(tokens)
}

pub(crate) fn parse_escape_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("escape")) {
        return Ok(None);
    }

    let cost_start = 1usize;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let comma_idx = tokens[cost_start..]
        .iter()
        .position(|token| token.is_comma())
        .map(|idx| cost_start + idx)
        .ok_or_else(|| {
            CardTextError::ParseError("escape keyword missing exile clause separator".to_string())
        })?;
    if comma_idx <= cost_start {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(&tokens[cost_start..comma_idx])?;
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("escape keyword missing mana symbols".to_string())
    })?;

    let tail_tokens = trim_commas(&tokens[comma_idx + 1..]);
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "escape keyword missing exile clause".to_string(),
        ));
    }

    let tail_words = words(&tail_tokens);
    if tail_words.first().copied() != Some("exile") {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    let Some((exile_count, used)) = parse_number_or_x_value(&tail_tokens[1..]) else {
        return Err(CardTextError::ParseError(format!(
            "escape keyword missing exile count (clause: '{}')",
            tail_words.join(" ")
        )));
    };
    let Value::Fixed(exile_count) = exile_count else {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape exile count (clause: '{}')",
            tail_words.join(" ")
        )));
    };
    let mut idx = 1 + used;
    if tail_words.get(idx).copied() == Some("other") {
        idx += 1;
    }
    if !matches!(tail_words.get(idx).copied(), Some("card") | Some("cards")) {
        return Err(CardTextError::ParseError(format!(
            "escape keyword missing exiled card noun (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    idx += 1;
    if tail_words.get(idx..idx + 3) != Some(&["from", "your", "graveyard"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    if idx + 3 != tail_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing escape clause segment (clause: '{}')",
            tail_words.join(" ")
        )));
    }

    Ok(Some(AlternativeCastingMethod::Escape {
        cost: Some(mana_cost),
        exile_count: exile_count as u32,
    }))
}

pub(crate) fn parse_escape_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_escape_line(tokens)
}

pub(crate) fn parse_flashback_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("flashback"))
    {
        return Ok(None);
    }

    let cost_tokens = tokens.get(1..).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "flashback keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(cost_tokens)?;
    if total_cost.mana_cost().is_none() {
        return Err(CardTextError::ParseError(
            "flashback keyword missing mana symbols".to_string(),
        ));
    }

    Ok(Some(AlternativeCastingMethod::Flashback { total_cost }))
}

pub(crate) fn parse_flashback_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_flashback_line(tokens)
}

pub(crate) fn parse_harmonize_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("harmonize"))
    {
        return Ok(None);
    }

    let cost_tokens = tokens.get(1..).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "harmonize keyword missing mana cost".to_string(),
        ));
    }

    let total_cost = parse_activation_cost(cost_tokens)?;
    if total_cost.mana_cost().is_none() {
        return Err(CardTextError::ParseError(
            "harmonize keyword missing mana symbols".to_string(),
        ));
    }

    Ok(Some(AlternativeCastingMethod::Harmonize { total_cost }))
}

pub(crate) fn parse_harmonize_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_harmonize_line(tokens)
}

pub(crate) fn parse_warp_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("warp")) {
        return Ok(None);
    }

    let (cost, _) = leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
        .ok_or_else(|| CardTextError::ParseError("warp keyword missing mana cost".to_string()))?;
    Ok(Some(AlternativeCastingMethod::Warp { cost }))
}

pub(crate) fn parse_warp_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_warp_line(tokens)
}

pub(crate) fn parse_bestow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("bestow")) {
        return Ok(None);
    }

    let (mana_cost, consumed_mana_tokens) =
        leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default()).ok_or_else(|| {
            CardTextError::ParseError("bestow keyword missing mana cost".to_string())
        })?;
    let mut total_cost = TotalCost::mana(mana_cost.clone());
    let consumed_mana_tokens = consumed_mana_tokens.min(tokens.len().saturating_sub(1));

    let mut cost_tokens = tokens
        .get(1..1 + consumed_mana_tokens)
        .unwrap_or_default()
        .to_vec();
    let tail_tokens = tokens.get(1 + consumed_mana_tokens..).unwrap_or_default();
    if tail_tokens.first().is_some_and(|token| token.is_comma()) {
        let clause_end = tail_tokens
            .iter()
            .position(|token| token.is_period())
            .unwrap_or(tail_tokens.len());
        let clause_tokens = trim_commas(&tail_tokens[..clause_end]).to_vec();
        let clause_words = words(&clause_tokens);
        if !clause_words.is_empty() && clause_words[0] != "if" {
            cost_tokens.extend(clause_tokens);
        }
    }

    if let Ok(parsed_total_cost) = parse_activation_cost(&cost_tokens) {
        total_cost = parsed_total_cost;
        if total_cost.mana_cost().is_none() {
            let mut components = total_cost.costs().to_vec();
            components.insert(0, crate::costs::Cost::mana(mana_cost));
            total_cost = TotalCost::from_costs(components);
        }
    }

    Ok(Some(AlternativeCastingMethod::Bestow { total_cost }))
}

pub(crate) fn parse_bestow_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_bestow_line(tokens)
}

pub(crate) fn parse_transmute_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_view = LowercaseWordView::new(tokens);
    let words_all = word_view.to_word_refs();
    if words_all.first().copied() != Some("transmute") {
        return Ok(None);
    }
    if words_all
        .iter()
        .any(|word| *word == "has" || *word == "have")
    {
        return Ok(None);
    }

    let Some((base_mana_cost, _consumed_cost_tokens)) =
        leading_mana_cost_from_tokens(tokens.get(1..).unwrap_or_default())
    else {
        return Err(CardTextError::ParseError(format!(
            "transmute keyword missing mana cost (clause: '{}')",
            words_all.join(" ")
        )));
    };
    let base_cost = TotalCost::mana(base_mana_cost.clone());
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut parsed_mana_value: Option<u32> = None;
    for idx in 0..word_view.len().saturating_sub(2) {
        if word_view.slice_eq(idx, &["mana", "value"]) {
            let start = word_view
                .token_index_for_word_index(idx + 2)
                .unwrap_or(tokens.len());
            parsed_mana_value =
                parse_number_or_x_value(&tokens[start..]).and_then(|(value, _)| match value {
                    Value::Fixed(n) if n >= 0 => Some(n as u32),
                    _ => None,
                });
            if parsed_mana_value.is_some() {
                break;
            }
        }
    }
    let filter = if let Some(mana_value) = parsed_mana_value {
        ObjectFilter::default().with_mana_value(crate::filter::Comparison::Equal(mana_value as i32))
    } else {
        ObjectFilter::default().with_mana_value(crate::filter::Comparison::EqualExpr(Box::new(
            crate::effect::Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Source)),
        )))
    };
    let text = format!("Transmute {}", base_mana_cost.to_oracle());

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::search_library_to_hand(filter, true),
                ]),
                choices: Vec::new(),
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: Vec::new(),
                activation_restrictions: Vec::new(),
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(text),
        },
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_transmute_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_transmute_line(tokens)
}

pub(crate) fn parse_reinforce_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_view = LowercaseWordView::new(tokens);
    let words_all = words_view.to_word_refs();
    if words_all.first().copied() != Some("reinforce") {
        return Ok(None);
    }
    if words_all
        .iter()
        .any(|word| *word == "has" || *word == "have")
    {
        return Ok(None);
    }

    let Some((amount_value, used_amount)) =
        parse_number_or_x_value(tokens.get(1..).unwrap_or_default())
    else {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing counter amount (clause: '{}')",
            words_all.join(" ")
        )));
    };
    let Value::Fixed(amount) = amount_value else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reinforce amount (clause: '{}')",
            words_all.join(" ")
        )));
    };

    let cost_start = 1 + used_amount;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana cost (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let Some((base_mana_cost, _consumed_cost_tokens)) =
        leading_mana_cost_from_tokens(tokens.get(cost_start..).unwrap_or_default())
    else {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana symbols (clause: '{}')",
            words_all.join(" ")
        )));
    };
    let base_cost = TotalCost::mana(base_mana_cost.clone());
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut creature_filter = ObjectFilter::default();
    creature_filter.zone = Some(Zone::Battlefield);
    creature_filter.card_types.push(CardType::Creature);

    let target = ChooseSpec::target(ChooseSpec::Object(creature_filter));
    let effect = Effect::put_counters(CounterType::PlusOnePlusOne, amount, target);

    let cost_text = base_mana_cost.to_oracle();
    let render_text = format!("Reinforce {amount} {cost_text}");

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![effect]),
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(render_text),
        },
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_reinforce_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_reinforce_line(tokens)
}

pub(crate) fn parse_cast_this_spell_only_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_word_view = LowercaseWordView::new(tokens);
    let line_words = line_word_view.to_word_refs();
    if !line_words.starts_with(&["cast", "this", "spell", "only"]) {
        return Ok(None);
    }

    let tail = &line_words[4..];
    let declare_attackers_tails: &[&[&str]] = &[
        &["during", "the", "declare", "attackers", "step"],
        &["during", "declare", "attackers", "step"],
    ];
    let declare_attackers_if_attacked_tails: &[&[&str]] = &[
        &[
            "during",
            "the",
            "declare",
            "attackers",
            "step",
            "and",
            "only",
            "if",
            "youve",
            "been",
            "attacked",
            "this",
            "step",
        ],
        &[
            "during",
            "declare",
            "attackers",
            "step",
            "and",
            "only",
            "if",
            "youve",
            "been",
            "attacked",
            "this",
            "step",
        ],
    ];

    let restriction = if declare_attackers_tails.contains(&tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_declare_attackers_step(),
            "Cast this spell only during the declare attackers step.",
        ))
    } else if declare_attackers_if_attacked_tails.contains(&tail) {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_declare_attackers_step_if_you_were_attacked_this_step(),
            "Cast this spell only during the declare attackers step and only if you've been attacked this step.",
        ))
    } else if tail == ["during", "combat"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat(),
            "Cast this spell only during combat.",
        ))
    } else if tail == ["during", "combat", "before", "blockers", "are", "declared"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_before_blockers_are_declared(),
            "Cast this spell only during combat before blockers are declared.",
        ))
    } else if tail == ["during", "combat", "after", "blockers", "are", "declared"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_after_blockers_are_declared(),
            "Cast this spell only during combat after blockers are declared.",
        ))
    } else if tail
        == [
            "during", "combat", "on", "your", "turn", "before", "blockers", "are", "declared",
        ]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_on_your_turn_before_blockers_are_declared(),
            "Cast this spell only during combat on your turn before blockers are declared.",
        ))
    } else if tail == ["during", "combat", "on", "an", "opponents", "turn"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_combat_on_opponents_turn(
            ),
            "Cast this spell only during combat on an opponent's turn.",
        ))
    } else if tail == ["before", "attackers", "are", "declared"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::before_attackers_are_declared(),
            "Cast this spell only before attackers are declared.",
        ))
    } else if tail == ["before", "the", "combat", "damage", "step"]
        || tail == ["before", "combat", "damage", "step"]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::before_combat_damage_step(),
            "Cast this spell only before the combat damage step.",
        ))
    } else if tail == ["during", "an", "opponents", "upkeep"]
        || tail == ["during", "opponents", "upkeep"]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_opponents_upkeep(),
            "Cast this spell only during an opponent's upkeep.",
        ))
    } else if tail
        == [
            "during",
            "an",
            "opponents",
            "turn",
            "after",
            "their",
            "upkeep",
            "step",
        ]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_opponents_turn_after_upkeep(),
            "Cast this spell only during an opponent's turn after their upkeep step.",
        ))
    } else if tail == ["during", "your", "end", "step"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::during_your_end_step(),
            "Cast this spell only during your end step.",
        ))
    } else if tail == ["if", "youve", "cast", "another", "spell", "this", "turn"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_cast_another_spell_this_turn(),
            "Cast this spell only if you've cast another spell this turn.",
        ))
    } else if tail
        == [
            "if", "youve", "cast", "another", "green", "spell", "this", "turn",
        ]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_cast_another_green_spell_this_turn(),
            "Cast this spell only if you've cast another green spell this turn.",
        ))
    } else if tail
        == [
            "if", "an", "opponent", "cast", "a", "creature", "spell", "this", "turn",
        ]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_opponent_cast_creature_spell_this_turn(),
            "Cast this spell only if an opponent cast a creature spell this turn.",
        ))
    } else if tail == ["if", "a", "creature", "is", "attacking", "you"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_creature_is_attacking_you(),
            "Cast this spell only if a creature is attacking you.",
        ))
    } else if tail == ["after", "combat"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::after_combat(),
            "Cast this spell only after combat.",
        ))
    } else if tail
        == [
            "if",
            "no",
            "permanents",
            "named",
            "tidal",
            "influence",
            "are",
            "on",
            "the",
            "battlefield",
        ]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_no_permanents_named_on_battlefield("Tidal Influence"),
            "Cast this spell only if no permanents named Tidal Influence are on the battlefield.",
        ))
    } else if tail == ["if", "you", "control", "a", "snow", "land"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_snow_land(),
            "Cast this spell only if you control a snow land.",
        ))
    } else if tail
        == [
            "if",
            "you",
            "control",
            "fewer",
            "creatures",
            "than",
            "each",
            "opponent",
        ]
    {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_fewer_creatures_than_each_opponent(),
            "Cast this spell only if you control fewer creatures than each opponent.",
        ))
    } else if tail == ["if", "you", "control", "two", "or", "more", "doctors"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_subtype_or_more(
                Subtype::Doctor,
                2,
            ),
            "Cast this spell only if you control two or more Doctors.",
        ))
    } else if tail == ["if", "you", "control", "two", "or", "more", "vampires"] {
        Some((
            crate::static_abilities::ThisSpellCastRestrictionKind::if_you_control_subtype_or_more(
                Subtype::Vampire,
                2,
            ),
            "Cast this spell only if you control two or more Vampires.",
        ))
    } else {
        None
    };

    Ok(restriction.map(|(kind, text)| StaticAbility::this_spell_cast_restriction(kind, text)))
}

pub(crate) fn parse_cast_this_spell_only_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    parse_cast_this_spell_only_line(tokens)
}

pub(crate) fn parse_you_may_rather_than_spell_cost_line(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !(tokens.first().is_some_and(|token| token.is_word("you"))
        && tokens.get(1).is_some_and(|token| token.is_word("may")))
    {
        return Ok(None);
    }
    let Some(rather_idx) = tokens.iter().position(|token| token.is_word("rather")) else {
        return Ok(None);
    };
    let rather_tail_view = LowercaseWordView::new(tokens.get(rather_idx + 1..).unwrap_or_default());
    let rather_tail = rather_tail_view.to_word_refs();
    let is_spell_cost_clause = rather_tail.starts_with(&["than", "pay", "this"])
        && rather_tail.contains(&"mana")
        && rather_tail.contains(&"cost")
        && (rather_tail.contains(&"spell") || rather_tail.contains(&"spells"));
    if !is_spell_cost_clause {
        return Ok(None);
    }
    let cost_clause_end = (rather_idx + 1..tokens.len())
        .rfind(|idx| {
            tokens[*idx]
                .as_word()
                .is_some_and(|word| word == "cost" || word == "costs")
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "alternative cost line missing terminal cost word (line: '{}')",
                line
            ))
        })?;
    let trailing_words = words(&tokens[cost_clause_end + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing clause after alternative cost (line: '{}', trailing: '{}')",
            line,
            trailing_words.join(" ")
        )));
    }
    let cost_tokens = tokens.get(2..rather_idx).unwrap_or_default();
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "alternative cost line missing cost clause".to_string(),
        ));
    }
    let total_cost = parse_activation_cost(cost_tokens)?;
    Ok(Some(AlternativeCastingMethod::Composed {
        name: "Parsed alternative cost",
        total_cost,
        condition: None,
    }))
}

pub(crate) fn parse_you_may_rather_than_spell_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_you_may_rather_than_spell_cost_line(tokens, line)
}

pub(crate) fn parse_additional_cost_choice_options(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst>>, CardTextError> {
    fn render_option_text(tokens: &[OwnedLexToken]) -> String {
        tokens
            .iter()
            .filter(|token| !matches!(token.kind, TokenKind::Comma | TokenKind::Period))
            .map(|token| token.slice.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    let clause_view = LowercaseWordView::new(tokens);
    let clause_words = clause_view.to_word_refs();
    if clause_words
        .windows(3)
        .any(|window| window == ["one", "or", "more"])
    {
        return Ok(None);
    }
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let option_tokens = split_on_or(tokens);
    if option_tokens.len() < 2 {
        return Ok(None);
    }

    let mut normalized_options = Vec::new();
    for mut option in option_tokens {
        while option
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            option.remove(0);
        }
        let option = trim_commas(&option).to_vec();
        if option.is_empty() {
            continue;
        }
        normalized_options.push(option);
    }

    if normalized_options.len() < 2 {
        return Ok(None);
    }

    let normalized_options = normalized_options
        .into_iter()
        .map(|option| lowercase_word_tokens(&option))
        .collect::<Vec<_>>();

    if normalized_options.iter().any(|option| {
        find_verb(option).is_none() && !option.first().is_some_and(|token| token.is_word("behold"))
    }) {
        return Ok(None);
    }

    let mut options = Vec::new();
    for option in normalized_options {
        let effects = parse_effect_sentences_lexed(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                render_option_text(&option)
            )));
        }
        options.push(AdditionalCostChoiceOptionAst {
            description: render_option_text(&option),
            effects,
        });
    }

    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(options))
}

pub(crate) fn parse_additional_cost_choice_options_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst>>, CardTextError> {
    parse_additional_cost_choice_options(tokens)
}

fn trap_condition_from_this_spell_cost_condition(
    condition: &crate::static_abilities::ThisSpellCostCondition,
) -> Option<crate::TrapCondition> {
    match condition {
        crate::static_abilities::ThisSpellCostCondition::OpponentCastSpellsThisTurnOrMore(
            count,
        ) => Some(crate::TrapCondition::OpponentCastSpells { count: *count }),
        crate::static_abilities::ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(
            _,
        ) => Some(crate::TrapCondition::CreatureDealtDamageToYou),
        _ => None,
    }
}

fn simple_trap_cost_from_alternative_method(method: &AlternativeCastingMethod) -> Option<ManaCost> {
    let AlternativeCastingMethod::Composed { total_cost, .. } = method else {
        return None;
    };
    if total_cost.non_mana_costs().next().is_some() {
        return None;
    }
    Some(
        total_cost
            .mana_cost()
            .cloned()
            .unwrap_or_else(ManaCost::new),
    )
}

pub(crate) fn parse_if_conditional_alternative_cost_line(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !clause_words.starts_with(&["if"]) {
        return Ok(None);
    }

    let (condition_tokens, tail_tokens) =
        if let Some(comma_idx) = tokens.iter().position(|token| token.is_comma()) {
            (
                trim_commas(&tokens[1..comma_idx]),
                trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default()),
            )
        } else if let Some(may_idx) = tokens.windows(3).position(|window| {
            window[0].is_word("you") && window[1].is_word("may") && window[2].is_word("pay")
        }) {
            (
                trim_commas(&tokens[1..may_idx]),
                trim_commas(&tokens[may_idx..]),
            )
        } else {
            return Ok(None);
        };
    if parse_self_free_cast_alternative_cost_line(&tail_tokens).is_none()
        && parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)?.is_none()
    {
        return Ok(None);
    }
    let condition = if let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) {
        condition
    } else {
        let condition_words_view = LowercaseWordView::new(&condition_tokens);
        let condition_words = condition_words_view.to_word_refs();
        if (condition_words.starts_with(&["youve", "been", "dealt", "damage", "by"])
            || condition_words.starts_with(&["you", "have", "been", "dealt", "damage", "by"]))
            && condition_words.ends_with(&["creatures", "this", "turn"])
        {
            let count_start = if condition_words.first().copied() == Some("youve") {
                5usize
            } else {
                6usize
            };
            if let Some((n, _)) =
                parse_number(condition_tokens.get(count_start..).unwrap_or_default())
            {
                crate::static_abilities::ThisSpellCostCondition::YouWereDealtDamageByCreaturesThisTurnOrMore(n)
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported this-spell cost condition (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported this-spell cost condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    };

    if parse_self_free_cast_alternative_cost_line(&tail_tokens).is_some() {
        let method = AlternativeCastingMethod::alternative_cost_with_condition(
            "Parsed alternative cost",
            None,
            Vec::new(),
            condition,
        );
        if let Some(trap_condition) = method
            .cast_condition()
            .and_then(trap_condition_from_this_spell_cost_condition)
            && let Some(cost) = simple_trap_cost_from_alternative_method(&method)
        {
            return Ok(Some(AlternativeCastingMethod::trap(
                "Trap",
                cost,
                trap_condition,
            )));
        }
        return Ok(Some(method));
    }

    let Some(method) = parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)? else {
        return Ok(None);
    };
    let method = method.with_cast_condition(condition);
    if let Some(trap_condition) = method
        .cast_condition()
        .and_then(trap_condition_from_this_spell_cost_condition)
        && let Some(cost) = simple_trap_cost_from_alternative_method(&method)
    {
        return Ok(Some(AlternativeCastingMethod::trap(
            "Trap",
            cost,
            trap_condition,
        )));
    }
    Ok(Some(method))
}

pub(crate) fn parse_if_conditional_alternative_cost_line_lexed(
    tokens: &[OwnedLexToken],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    parse_if_conditional_alternative_cost_line(tokens, line)
}

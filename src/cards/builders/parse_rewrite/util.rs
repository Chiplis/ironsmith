use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::TextSpan;
use crate::cards::builders::{
    AdditionalCostChoiceOptionAst, CardTextError, ParsedAbility, ReferenceImports, Token,
    find_verb, parse_activation_cost, split_on_or,
};
use crate::cost::OptionalCost;
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::effect::Value;
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use crate::{PowerToughness, PtValue};

use super::clause_support::rewrite_parse_effect_sentences;
use super::ported_keyword_static::parse_this_spell_cost_condition;

pub(crate) fn tokenize_line(line: &str, line_index: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut word_start: Option<usize> = None;
    let mut word_end: usize = 0;
    let mut in_mana_braces = false;

    let flush = |buffer: &mut String,
                 tokens: &mut Vec<Token>,
                 word_start: &mut Option<usize>,
                 word_end: &mut usize| {
        if !buffer.is_empty() {
            let start = word_start.unwrap_or(0);
            tokens.push(Token::Word(
                buffer.clone(),
                TextSpan {
                    line: line_index,
                    start,
                    end: *word_end,
                },
            ));
            buffer.clear();
        }
        *word_start = None;
        *word_end = 0;
    };

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    for (idx, (byte_idx, mut ch)) in chars.iter().copied().enumerate() {
        if ch == '−' {
            ch = '-';
        }
        if ch == '{' {
            flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
            in_mana_braces = true;
            continue;
        }
        if ch == '}' {
            flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
            in_mana_braces = false;
            continue;
        }
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
                word_start = Some(byte_idx);
            }
            word_end = byte_idx + ch.len_utf8();
            buffer.push(ch.to_ascii_lowercase());
            continue;
        }

        if ch == '"' || ch == '“' || ch == '”' {
            flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
            tokens.push(Token::Quote(TextSpan {
                line: line_index,
                start: byte_idx,
                end: byte_idx + ch.len_utf8(),
            }));
            continue;
        }

        if ch == '\'' || ch == '’' || ch == '‘' {
            if word_start.is_some() {
                word_end = byte_idx + ch.len_utf8();
            }
            continue;
        }

        flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);

        let span = TextSpan {
            line: line_index,
            start: byte_idx,
            end: byte_idx + ch.len_utf8(),
        };

        match ch {
            ',' => tokens.push(Token::Comma(span)),
            '.' => tokens.push(Token::Period(span)),
            ':' => tokens.push(Token::Colon(span)),
            ';' => tokens.push(Token::Semicolon(span)),
            _ => {}
        }
    }

    flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
    tokens
}

pub(crate) fn words(tokens: &[Token]) -> Vec<&str> {
    tokens
        .iter()
        .filter_map(|token| match token {
            Token::Word(word, _) => Some(word.as_str()),
            _ => None,
        })
        .collect()
}

pub(crate) fn trim_commas(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && matches!(tokens[start], Token::Comma(_)) {
        start += 1;
    }
    while end > start && matches!(tokens[end - 1], Token::Comma(_)) {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

pub(crate) fn split_on_period(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut quote_depth = 0u32;

    for token in tokens {
        if matches!(token, Token::Quote(_)) {
            quote_depth = if quote_depth == 0 { 1 } else { 0 };
            current.push(token.clone());
        } else if matches!(token, Token::Period(_)) && quote_depth == 0 {
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

pub(crate) fn parse_number_or_x_value(tokens: &[Token]) -> Option<(Value, usize)> {
    let token = tokens.first()?;
    let word = match token {
        Token::Word(word, _) => word.as_str(),
        _ => return None,
    };

    if word == "x" {
        return Some((Value::X, 1));
    }

    if let Ok(value) = word.parse::<u32>() {
        return Some((Value::Fixed(value as i32), 1));
    }

    let value = match word {
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
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let token_words = words(tokens);
    if token_words.len() < 3 || token_words[0] != "level" || token_words[1] != "up" {
        return Ok(None);
    }

    let mut symbols = Vec::new();
    for word in token_words.iter().skip(2) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            symbols.push(symbol);
        }
    }

    if symbols.is_empty() {
        return Err(CardTextError::ParseError(
            "level up missing mana cost".to_string(),
        ));
    }

    let pips = symbols.into_iter().map(|symbol| vec![symbol]).collect();
    let mana_cost = ManaCost::from_pips(pips);
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
    tokens: &[Token],
) -> Option<AlternativeCastingMethod> {
    let clause_words = words(tokens);
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

pub(crate) fn parse_madness_line(
    tokens: &[Token],
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
        .position(|token| matches!(token, Token::Comma(_)))
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

pub(crate) fn parse_buyback_line(tokens: &[Token]) -> Result<Option<OptionalCost>, CardTextError> {
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

pub(crate) fn parse_optional_cost_keyword_line(
    tokens: &[Token],
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
        .position(|token| matches!(token, Token::Period(_)))
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

pub(crate) fn parse_kicker_line(tokens: &[Token]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "kicker", OptionalCost::kicker)
}

pub(crate) fn parse_multikicker_line(
    tokens: &[Token],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "multikicker", OptionalCost::multikicker)
}

pub(crate) fn parse_squad_line(tokens: &[Token]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "squad", OptionalCost::squad)
}

pub(crate) fn parse_offspring_line(
    tokens: &[Token],
) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "offspring", OptionalCost::offspring)
}

pub(crate) fn parse_entwine_line(tokens: &[Token]) -> Result<Option<OptionalCost>, CardTextError> {
    parse_optional_cost_keyword_line(tokens, "entwine", OptionalCost::entwine)
}

pub(crate) fn parse_morph_keyword_line(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(first_word) = tokens.first().and_then(Token::as_word) else {
        return Ok(None);
    };

    let is_megamorph = match first_word {
        "morph" => false,
        "megamorph" => true,
        _ => return Ok(None),
    };

    let mut symbols = Vec::new();
    let mut consumed = 1usize;
    for token in tokens.iter().skip(1) {
        let Some(word) = token.as_word() else {
            break;
        };
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        symbols.push(symbol);
        consumed += 1;
    }

    if symbols.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "{mechanic} keyword missing mana cost"
        )));
    }

    let trailing_words = words(&tokens[consumed..]);
    if !trailing_words.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing {mechanic} clause (line: '{}')",
            trailing_words.join(" ")
        )));
    }

    let cost = ManaCost::from_symbols(symbols);
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

pub(crate) fn parse_escape_line(
    tokens: &[Token],
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
        .position(|token| matches!(token, Token::Comma(_)))
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

pub(crate) fn parse_flashback_line(
    tokens: &[Token],
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

pub(crate) fn parse_warp_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("warp")) {
        return Ok(None);
    }

    let words_all = words(tokens);
    let (cost_text, _) = leading_mana_symbols_to_oracle(&words_all[1..])
        .ok_or_else(|| CardTextError::ParseError("warp keyword missing mana cost".to_string()))?;
    let cost = parse_scryfall_mana_cost(&cost_text).map_err(|err| {
        CardTextError::ParseError(format!("invalid warp mana cost '{cost_text}': {err:?}"))
    })?;
    Ok(Some(AlternativeCastingMethod::Warp { cost }))
}

pub(crate) fn parse_bestow_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("bestow")) {
        return Ok(None);
    }

    let words_all = words(tokens);
    let (mana_cost_text, mana_word_count) = leading_mana_symbols_to_oracle(&words_all[1..])
        .ok_or_else(|| CardTextError::ParseError("bestow keyword missing mana cost".to_string()))?;
    let mana_cost = parse_scryfall_mana_cost(&mana_cost_text).map_err(|err| {
        CardTextError::ParseError(format!(
            "invalid bestow mana cost '{mana_cost_text}': {err:?}"
        ))
    })?;
    let mut total_cost = TotalCost::mana(mana_cost.clone());

    let mut consumed_mana_tokens = 0usize;
    for token in tokens.iter().skip(1) {
        let Some(word) = token.as_word() else {
            break;
        };
        if parse_mana_symbol(word).is_ok() {
            consumed_mana_tokens += 1;
            continue;
        }
        break;
    }
    if consumed_mana_tokens == 0 {
        consumed_mana_tokens = mana_word_count;
    }
    consumed_mana_tokens = consumed_mana_tokens.min(tokens.len().saturating_sub(1));

    let mut cost_tokens = tokens
        .get(1..1 + consumed_mana_tokens)
        .unwrap_or_default()
        .to_vec();
    let tail_tokens = tokens.get(1 + consumed_mana_tokens..).unwrap_or_default();
    if tail_tokens
        .first()
        .is_some_and(|token| matches!(token, Token::Comma(_)))
    {
        let clause_end = tail_tokens
            .iter()
            .position(|token| matches!(token, Token::Period(_)))
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

pub(crate) fn parse_transmute_line(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_all = words(tokens);
    if words_all.first().copied() != Some("transmute") {
        return Ok(None);
    }
    if words_all
        .iter()
        .any(|word| *word == "has" || *word == "have")
    {
        return Ok(None);
    }

    let mut consumed = 1usize;
    while consumed < tokens.len() {
        let Some(word) = tokens[consumed].as_word() else {
            break;
        };
        if parse_mana_symbol(word).is_ok() {
            consumed += 1;
        } else {
            break;
        }
    }
    if consumed <= 1 {
        return Err(CardTextError::ParseError(format!(
            "transmute keyword missing mana cost (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let cost_tokens = &tokens[1..consumed];
    let base_cost = parse_activation_cost(cost_tokens)?;
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut parsed_mana_value: Option<u32> = None;
    for idx in 0..tokens.len().saturating_sub(2) {
        if tokens[idx].is_word("mana") && tokens[idx + 1].is_word("value") {
            parsed_mana_value =
                parse_number_or_x_value(&tokens[idx + 2..]).and_then(|(value, _)| match value {
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
    let text = format!(
        "Transmute {}",
        base_cost
            .mana_cost()
            .map(|cost| cost.to_oracle())
            .unwrap_or_else(|| words(cost_tokens).join(" "))
    );

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

pub(crate) fn parse_reinforce_line(
    tokens: &[Token],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_all = words(tokens);
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

    let mut cost_end = cost_start;
    while cost_end < tokens.len() {
        let Some(word) = tokens[cost_end].as_word() else {
            break;
        };
        if parse_mana_symbol(word).is_ok() {
            cost_end += 1;
        } else {
            break;
        }
    }
    if cost_end == cost_start {
        return Err(CardTextError::ParseError(format!(
            "reinforce line missing mana symbols (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let cost_tokens = &tokens[cost_start..cost_end];
    let base_cost = parse_activation_cost(cost_tokens)?;
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut creature_filter = ObjectFilter::default();
    creature_filter.zone = Some(Zone::Battlefield);
    creature_filter.card_types.push(CardType::Creature);

    let target = ChooseSpec::target(ChooseSpec::Object(creature_filter));
    let effect = Effect::put_counters(CounterType::PlusOnePlusOne, amount, target);

    let cost_text = base_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| words(cost_tokens).join(" "));
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

pub(crate) fn parse_cast_this_spell_only_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
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

pub(crate) fn parse_you_may_rather_than_spell_cost_line(
    tokens: &[Token],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !(tokens
        .first()
        .is_some_and(|token| matches!(token, Token::Word(word, _) if word == "you"))
        && tokens
            .get(1)
            .is_some_and(|token| matches!(token, Token::Word(word, _) if word == "may")))
    {
        return Ok(None);
    }
    let Some(rather_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Word(word, _) if word == "rather"))
    else {
        return Ok(None);
    };
    let rather_tail = words(tokens.get(rather_idx + 1..).unwrap_or_default());
    let is_spell_cost_clause = rather_tail.starts_with(&["than", "pay", "this"])
        && rather_tail.contains(&"mana")
        && rather_tail.contains(&"cost")
        && (rather_tail.contains(&"spell") || rather_tail.contains(&"spells"));
    if !is_spell_cost_clause {
        return Ok(None);
    }
    let cost_clause_end = (rather_idx + 1..tokens.len())
        .rfind(|idx| {
            matches!(
                tokens[*idx],
                Token::Word(ref word, _) if word == "cost" || word == "costs"
            )
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

pub(crate) fn parse_additional_cost_choice_options(
    tokens: &[Token],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst>>, CardTextError> {
    let clause_words = words(tokens);
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

    if normalized_options
        .iter()
        .any(|option| find_verb(option).is_none())
    {
        return Ok(None);
    }

    let mut options = Vec::new();
    for option in normalized_options {
        let effects = rewrite_parse_effect_sentences(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                words(&option).join(" ")
            )));
        }
        options.push(AdditionalCostChoiceOptionAst {
            description: words(&option).join(" "),
            effects,
        });
    }

    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(options))
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
    tokens: &[Token],
    line: &str,
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["if"]) {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(&tokens[1..comma_idx]);
    let tail_tokens = trim_commas(tokens.get(comma_idx + 1..).unwrap_or_default());
    if parse_self_free_cast_alternative_cost_line(&tail_tokens).is_none()
        && parse_you_may_rather_than_spell_cost_line(&tail_tokens, line)?.is_none()
    {
        return Ok(None);
    }
    let Some(condition) = parse_this_spell_cost_condition(&condition_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported this-spell cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
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

use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenBoundary {
    pub(crate) token: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WordBoundary {
    pub(crate) word: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellCostIncreaseHead {
    pub(crate) line_start: TokenBoundary,
    pub(crate) costs: TokenBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OptionalLifeReductionWords {
    pub(crate) pay: WordBoundary,
    pub(crate) those_spells: WordBoundary,
    pub(crate) costs: WordBoundary,
    pub(crate) payment_has_life: bool,
    pub(crate) those_spells_paid_life_this_way: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerAbilityCostWords {
    pub(crate) activate: WordBoundary,
    pub(crate) costs: WordBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdditionalCostSpellFilter<'a> {
    pub(crate) spell_filter_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ActivatedAbilityCostIncrease<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) additional_cost_tokens: &'a [OwnedLexToken],
}

pub(crate) fn parse_additional_cost_spell_filter(
    tokens: &[OwnedLexToken],
) -> Option<AdditionalCostSpellFilter<'_>> {
    let (parsed, _) = primitives::parse_prefix(tokens, parse_additional_cost_spell_filter_lexed)?;
    Some(parsed)
}

pub(crate) fn parse_activated_ability_cost_increase(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedAbilityCostIncrease<'_>> {
    let (parsed, _) =
        primitives::parse_prefix(tokens, parse_activated_ability_cost_increase_lexed)?;
    Some(parsed)
}

pub(crate) fn parse_spell_cost_increase_head(
    tokens: &[OwnedLexToken],
) -> Option<SpellCostIncreaseHead> {
    let line_start = phrase_start(tokens, &["this", "spell", "costs"])?;
    let relative_costs = first_token_word(&tokens[line_start.token..], &["cost", "costs"])?;
    Some(SpellCostIncreaseHead {
        line_start,
        costs: TokenBoundary {
            token: line_start.token + relative_costs.token,
        },
    })
}

pub(crate) fn parse_if_cost_condition_comma(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    first_comma(tokens)
}

pub(crate) fn parse_cost_verb(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    first_token_word(tokens, &["cost", "costs"])
}

pub(crate) fn parse_trailing_cost_condition_if(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["if"])
}

pub(crate) fn parse_cost_prefix_subject_comma(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    first_comma(tokens)
}

pub(crate) fn parse_optional_life_subject_is_permanent(words: &[&str]) -> bool {
    first_word(words, &["permanent"]).is_some()
}

pub(crate) fn parse_optional_life_reduction_words(
    words: &[&str],
) -> Option<OptionalLifeReductionWords> {
    let pay = first_word(words, &["pay"])?;
    let payment_words = &words[pay.word + 1..];
    let payment_has_life = first_word(payment_words, &["life"]).is_some();
    let those_spells = phrase_start_words(words, &["those", "spells"])?;
    let relative_costs = first_word(&words[those_spells.word..], &["cost", "costs"])?;
    let those_spells_paid_life_this_way = phrase_start_words(
        &words[those_spells.word..],
        &["paid", "life", "this", "way"],
    )
    .is_some();
    Some(OptionalLifeReductionWords {
        pay,
        those_spells,
        costs: WordBoundary {
            word: those_spells.word + relative_costs.word,
        },
        payment_has_life,
        those_spells_paid_life_this_way,
    })
}

fn parse_additional_cost_spell_filter_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AdditionalCostSpellFilter<'a>> {
    primitives::phrase(&["as", "an", "additional", "cost", "to", "cast"]).parse_next(input)?;
    let spell_filter_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((primitives::kw("spell"), primitives::kw("spells")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("spell"), primitives::kw("spells"))).parse_next(input)?;
    Ok(AdditionalCostSpellFilter {
        spell_filter_tokens,
    })
}

fn parse_activated_ability_cost_increase_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ActivatedAbilityCostIncrease<'a>> {
    primitives::phrase(&["activated", "abilities", "of"]).parse_next(input)?;
    let subject_tokens = repeat_till(
        1..,
        any.void(),
        peek(alt((primitives::kw("cost"), primitives::kw("costs")))),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    alt((primitives::kw("cost"), primitives::kw("costs"))).parse_next(input)?;
    primitives::phrase(&["an", "additional"]).parse_next(input)?;
    let additional_cost_tokens = repeat_till(
        1..,
        any.void(),
        peek(primitives::phrase(&["to", "activate"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["to", "activate"]).parse_next(input)?;
    Ok(ActivatedAbilityCostIncrease {
        subject_tokens,
        additional_cost_tokens,
    })
}

pub(crate) fn parse_cost_modifier_cast_marker(words: &[&str]) -> bool {
    first_word(words, &["cast"]).is_some()
}

pub(crate) fn parse_spells_subject(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    first_token_word(tokens, &["spell", "spells"])
}

pub(crate) fn parse_cost_direction_if_boundary(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["if"])
}

pub(crate) fn parse_spell_and_abilities_separator(
    tokens: &[OwnedLexToken],
) -> Option<TokenBoundary> {
    phrase_start(tokens, &["and", "abilities"])
}

pub(crate) fn parse_player_ability_cost_words(words: &[&str]) -> Option<PlayerAbilityCostWords> {
    let activate = first_word(words, &["activate", "activates"])?;
    let relative_costs = first_word(&words[activate.word + 1..], &["cost", "costs"])?;
    Some(PlayerAbilityCostWords {
        activate,
        costs: WordBoundary {
            word: activate.word + 1 + relative_costs.word,
        },
    })
}

pub(crate) fn parse_relative_target_clause(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    phrase_start_any(
        tokens,
        &[
            &["that", "target"] as &[&str],
            &["that", "targets"] as &[&str],
        ],
    )
}

pub(crate) fn parse_trailing_target_condition_if(words: &[&str]) -> Option<WordBoundary> {
    first_word(words, &["if"])
}

pub(crate) fn parse_last_cost_verb(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    let mut last = None;
    loop {
        let token = initial_len.saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let Ok(candidate) = parsed else {
            break;
        };
        if candidate
            .as_word()
            .is_some_and(|word| matches!(word, "cost" | "costs"))
        {
            last = Some(TokenBoundary { token });
        }
    }
    last
}

pub(crate) fn parse_dynamic_cost_each_word(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    first_token_word(tokens, &["each"])
}

fn first_token_word(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<TokenBoundary> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let token = initial_len.saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        let candidate = parsed.ok()?;
        let Some(candidate_word) = candidate.as_word() else {
            continue;
        };
        if expected.iter().any(|word| candidate_word == *word) {
            return Some(TokenBoundary { token });
        }
    }
}

fn first_word(words: &[&str], expected: &[&str]) -> Option<WordBoundary> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let word = initial_len.saturating_sub(input.len());
        let parsed: WResult<&str> = any.parse_next(&mut input);
        let candidate = parsed.ok()?;
        if expected
            .iter()
            .any(|expected_word| candidate == *expected_word)
        {
            return Some(WordBoundary { word });
        }
    }
}

fn first_comma(tokens: &[OwnedLexToken]) -> Option<TokenBoundary> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let token = initial_len.saturating_sub(input.len());
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        if parsed.ok()?.is_comma() {
            return Some(TokenBoundary { token });
        }
    }
}

fn phrase_start(
    tokens: &[OwnedLexToken],
    expected: &'static [&'static str],
) -> Option<TokenBoundary> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let token = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::phrase(expected)
            .parse_next(&mut candidate)
            .is_ok()
        {
            return Some(TokenBoundary { token });
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

fn phrase_start_any(
    tokens: &[OwnedLexToken],
    alternatives: &[&'static [&'static str]],
) -> Option<TokenBoundary> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let token = initial_len.saturating_sub(input.len());
        if alternatives.iter().any(|expected| {
            let mut candidate = input.clone();
            primitives::phrase(expected)
                .parse_next(&mut candidate)
                .is_ok()
        }) {
            return Some(TokenBoundary { token });
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

fn phrase_start_words(words: &[&str], expected: &[&str]) -> Option<WordBoundary> {
    let mut input: primitives::WordSliceInput<'_> = words;
    let initial_len = input.len();
    loop {
        let word = initial_len.saturating_sub(input.len());
        let mut candidate = input;
        let mut expected_input = expected;
        let mut matched = true;
        loop {
            let parsed_expected: WResult<&str> = any.parse_next(&mut expected_input);
            let Ok(expected_word) = parsed_expected else {
                break;
            };
            let parsed_actual: WResult<&str> = any.parse_next(&mut candidate);
            if parsed_actual.ok() != Some(expected_word) {
                matched = false;
                break;
            }
        }
        if matched {
            return Some(WordBoundary { word });
        }
        let parsed: WResult<&str> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::builders::TextSpan;

    fn tokens(words: &[&str]) -> Vec<OwnedLexToken> {
        words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect()
    }

    #[test]
    fn parses_spell_and_player_cost_heads() {
        let line = tokens(&["this", "spell", "costs", "two", "more"]);
        assert_eq!(
            parse_spell_cost_increase_head(&line),
            Some(SpellCostIncreaseHead {
                line_start: TokenBoundary { token: 0 },
                costs: TokenBoundary { token: 2 },
            })
        );
        let words = ["abilities", "your", "opponents", "activate", "cost", "more"];
        assert_eq!(
            parse_player_ability_cost_words(&words).unwrap().costs.word,
            4
        );
    }

    #[test]
    fn parses_optional_life_reduction_boundaries() {
        let words = [
            "you", "may", "pay", "2", "life", "those", "spells", "cost", "less",
        ];
        let shape = parse_optional_life_reduction_words(&words).unwrap();
        assert_eq!(shape.pay.word, 2);
        assert_eq!(shape.those_spells.word, 5);
        assert_eq!(shape.costs.word, 7);
        assert!(shape.payment_has_life);
        assert!(!shape.those_spells_paid_life_this_way);
    }

    #[test]
    fn parses_relative_target_and_last_cost() {
        let line = tokens(&["spells", "that", "target", "you", "cost", "one", "more"]);
        assert_eq!(
            parse_relative_target_clause(&line),
            Some(TokenBoundary { token: 1 })
        );
        assert_eq!(
            parse_last_cost_verb(&line),
            Some(TokenBoundary { token: 4 })
        );
    }

    #[test]
    fn parses_typed_additional_cost_subjects() {
        let line = tokens(&[
            "as",
            "an",
            "additional",
            "cost",
            "to",
            "cast",
            "creature",
            "spells",
            "you",
            "may",
            "pay",
            "life",
        ]);
        let parsed = parse_additional_cost_spell_filter(&line).unwrap();
        assert_eq!(parsed.spell_filter_tokens.len(), 1);

        let line = tokens(&[
            "activated",
            "abilities",
            "of",
            "artifacts",
            "cost",
            "an",
            "additional",
            "two",
            "life",
            "to",
            "activate",
        ]);
        let parsed = parse_activated_ability_cost_increase(&line).unwrap();
        assert_eq!(parsed.subject_tokens.len(), 1);
        assert_eq!(parsed.additional_cost_tokens.len(), 2);
    }
}

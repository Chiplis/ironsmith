use winnow::combinator::{alt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::{any, rest};

use crate::color::Color;
use crate::effect::Value;
use crate::mana::ManaCost;
use crate::target::ObjectFilter;
use crate::target::PlayerFilter;

use super::super::lexer::{LexStream, OwnedLexToken, parser_token_word_refs, trim_lexed_commas};
use super::primitives::{self, TokenWordView, WordSliceInput};

#[path = "activated_lines/x_and_loyalty_facts.rs"]
mod x_and_loyalty_facts;
pub use x_and_loyalty_facts::*;

#[path = "activated_lines/blocking_and_cycling.rs"]
mod blocking_and_cycling;
pub use blocking_and_cycling::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActivatedLineSplit<'a> {
    pub before_colon: &'a [OwnedLexToken],
    pub after_colon: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryManaClauseKind {
    Standard,
    ColorsAmong,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimaryManaClauseSpec<'a> {
    pub kind: PrimaryManaClauseKind,
    pub mana_tokens: &'a [OwnedLexToken],
    pub subject_tokens: Option<&'a [OwnedLexToken]>,
    pub has_for_each: bool,
    pub requires_general_effect: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivatedDevotionParseError {
    UnsupportedPlayer,
    MissingColorAfterDevotion,
    MissingColor,
    UnsupportedColor(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntersTappedLineShape {
    NoMatch,
    NegatedUntap,
    MixedNegatedUntap,
    EntersTapped,
    AttackingVariant,
    UnsupportedTrailing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CostReductionLineHead<'a> {
    ThisCost {
        amount_tokens: &'a [OwnedLexToken],
        diagnostic_amount_word: &'a str,
        diagnostic_tail: String,
    },
    ActivatedAbilitiesOf {
        subject_tokens: &'a [OwnedLexToken],
        amount_tokens: &'a [OwnedLexToken],
    },
    ThisAbility {
        amount_tokens: &'a [OwnedLexToken],
    },
    ThisSpell {
        amount_tokens: &'a [OwnedLexToken],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThisCostReductionRemainder {
    ForEach,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedAbilitiesReductionRemainder {
    Unbounded,
    MinimumOneMana,
    MinimumOneManaAbilityActivationCost,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThisAbilityReductionRemainder<'a> {
    Unconditional,
    Targets {
        count_and_filter_tokens: &'a [OwnedLexToken],
    },
    ForEach {
        filter_tokens: &'a [OwnedLexToken],
    },
    UnsupportedCondition,
    NotReduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThisSpellReductionRemainder {
    NotReduction,
    General,
    CardTypesInGraveyard,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NextSpellCostReductionSpec {
    pub spell_filter: ObjectFilter,
    pub reduction: ManaCost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineActivatedSentenceKind {
    ThisAbilityCostReduction,
    NextSpellCostReduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThisAbilityCostReference;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OncePerTurnRestrictionNormalization {
    Redundant,
    Residual(String),
}

fn parse_activated_line_split<'a>(input: &mut LexStream<'a>) -> WResult<ActivatedLineSplit<'a>> {
    let before_colon = repeat_till(0.., any.void(), peek(primitives::colon()))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::colon().parse_next(input)?;
    let after_colon = rest.parse_next(input)?;
    Ok(ActivatedLineSplit {
        before_colon,
        after_colon,
    })
}

pub fn parse_activated_line_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedLineSplit<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_activated_line_split,
        "activated-line-split",
    )
}

fn word_phrase<'a>(
    expected: &'static [&'static str],
) -> impl Parser<WordSliceInput<'a>, (), ErrMode<ContextError>> {
    move |input: &mut WordSliceInput<'a>| {
        for word in expected {
            primitives::word_slice_exact(word)
                .void()
                .parse_next(input)?;
        }
        Ok(())
    }
}

fn word_phrase_present(words: &[&str], expected: &'static [&'static str]) -> bool {
    let mut input: WordSliceInput<'_> = words;
    repeat_till(0.., any.void(), peek(word_phrase(expected)))
        .map(|((), ())| ())
        .parse_next(&mut input)
        .is_ok()
}

fn word_present(words: &[&str], expected: &'static str) -> bool {
    let mut input: WordSliceInput<'_> = words;
    repeat_till(
        0..,
        any.void(),
        peek(primitives::word_slice_exact(expected)),
    )
    .map(|((), _)| ())
    .parse_next(&mut input)
    .is_ok()
}

fn parse_add_word(input: &mut WordSliceInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("add"),
        primitives::word_slice_exact("adds"),
    ))
    .void()
    .parse_next(input)
}

fn parse_primary_mana_head(input: &mut WordSliceInput<'_>) -> WResult<usize> {
    alt((
        (word_phrase(&["that", "player"]), parse_add_word).value(2),
        (word_phrase(&["target", "player"]), parse_add_word).value(2),
        (primitives::word_slice_exact("you"), parse_add_word).value(1),
        parse_add_word.value(0),
    ))
    .parse_next(input)
}

fn mana_clause_needs_general_effect(words: &[&str]) -> bool {
    let has_imprinted_colors = word_present(words, "exiled")
        && (word_present(words, "card") || word_present(words, "cards"))
        && (word_present(words, "color") || word_present(words, "colors"));
    let has_any_combination = word_phrase_present(words, &["any", "combination", "of"]);
    let has_any_choice = has_any_combination
        || (word_present(words, "any")
            && (word_present(words, "color") || word_present(words, "type")));
    let uses_commander_identity = word_present(words, "identity")
        && (word_present(words, "commander") || word_present(words, "commanders"));

    has_imprinted_colors
        || has_any_choice
        || word_present(words, "or")
        || (word_present(words, "chosen") && word_present(words, "color"))
        || uses_commander_identity
}

pub fn parse_primary_mana_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PrimaryManaClauseSpec<'_>> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let colors_among = word_phrase_present(&words, &["for", "each", "color", "among"])
        && word_phrase_present(&words, &["add", "one", "mana", "of", "that", "color"]);

    if colors_among {
        return Some(PrimaryManaClauseSpec {
            kind: PrimaryManaClauseKind::ColorsAmong,
            mana_tokens: tokens,
            subject_tokens: None,
            has_for_each: true,
            requires_general_effect: true,
        });
    }

    let mut input: WordSliceInput<'_> = &words;
    let add_word = crate::grammar::primitives::take_leaf(&mut input, parse_primary_mana_head)?;
    let add_token = view.token_span_for_words(add_word, add_word + 1)?.start;
    let mana_tokens = tokens.get(add_token + 1..)?;
    let mana_words = TokenWordView::new(mana_tokens).word_refs();
    Some(PrimaryManaClauseSpec {
        kind: PrimaryManaClauseKind::Standard,
        mana_tokens,
        subject_tokens: (add_token > 0).then_some(&tokens[..add_token]),
        has_for_each: word_phrase_present(&mana_words, &["for", "each"]),
        requires_general_effect: mana_clause_needs_general_effect(&mana_words),
    })
}

fn parse_devotion_owner(words: &[&str]) -> Option<PlayerFilter> {
    if words.len() >= 2 {
        let tail = &words[words.len() - 2..];
        let target_player = alt((
            word_phrase(&["that", "players"]),
            word_phrase(&["that", "player"]),
            word_phrase(&["that", "player's"]),
            word_phrase(&["that", "players'"]),
        ))
        .value(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
        if let Some(player) = primitives::parse_full_word_slice(tail, target_player) {
            return Some(player);
        }
    }

    let tail = words.get(words.len().checked_sub(1)?..)?;
    primitives::parse_full_word_slice(
        tail,
        alt((
            primitives::word_slice_exact("your").value(PlayerFilter::You),
            primitives::word_slice_exact("their").value(PlayerFilter::IteratedPlayer),
            alt((
                primitives::word_slice_exact("opponent"),
                primitives::word_slice_exact("opponents"),
            ))
            .value(PlayerFilter::Opponent),
        )),
    )
}

pub fn parse_activated_devotion_value_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, ActivatedDevotionParseError> {
    let Some((devotion_token, _, _)) =
        primitives::find_prefix(tokens, || primitives::kw("devotion"))
    else {
        return Ok(None);
    };
    let owner_words = parser_token_word_refs(&tokens[..devotion_token]);
    let player =
        parse_devotion_owner(&owner_words).ok_or(ActivatedDevotionParseError::UnsupportedPlayer)?;

    let after_devotion = &tokens[devotion_token + 1..];
    let Some((relative_to, _, _)) =
        primitives::find_prefix(after_devotion, || primitives::kw("to"))
    else {
        return Err(ActivatedDevotionParseError::MissingColorAfterDevotion);
    };
    let color_tokens = &after_devotion[relative_to + 1..];
    let color_words = parser_token_word_refs(color_tokens);
    let mut color_input: WordSliceInput<'_> = &color_words;
    if word_phrase(&["that", "color"])
        .parse_next(&mut color_input)
        .is_ok()
    {
        return Ok(Some(Value::DevotionToChosenColor(player)));
    }

    let color_word = color_words
        .first()
        .copied()
        .ok_or(ActivatedDevotionParseError::MissingColor)?;
    let color = Color::from_name(color_word)
        .ok_or_else(|| ActivatedDevotionParseError::UnsupportedColor(color_word.to_string()))?;
    Ok(Some(Value::Devotion { player, color }))
}

fn words_are_exact(words: &[&str], expected: &'static [&'static str]) -> bool {
    primitives::parse_full_word_slice(words, word_phrase(expected)).is_some()
}

fn negated_untap_fact(words: &[&str]) -> bool {
    let has_untap = word_present(words, "untap") || word_present(words, "untaps");
    let has_negation = word_present(words, "doesnt")
        || word_present(words, "dont")
        || word_present(words, "cant")
        || word_phrase_present(words, &["does", "not"])
        || word_phrase_present(words, &["do", "not"])
        || word_phrase_present(words, &["can", "not"]);
    has_untap && has_negation
}

pub fn parse_enters_tapped_line_shape(tokens: &[OwnedLexToken]) -> EntersTappedLineShape {
    let words = parser_token_word_refs(tokens);
    if words.is_empty() {
        return EntersTappedLineShape::NoMatch;
    }
    let has_enters_tapped = word_present(&words, "enters") && word_present(&words, "tapped");
    if negated_untap_fact(&words) {
        return if has_enters_tapped {
            EntersTappedLineShape::MixedNegatedUntap
        } else {
            EntersTappedLineShape::NegatedUntap
        };
    }

    let mut prefix_input: WordSliceInput<'_> = &words;
    if primitives::word_slice_exact("this")
        .parse_next(&mut prefix_input)
        .is_err()
        || !has_enters_tapped
    {
        return EntersTappedLineShape::NoMatch;
    }

    let mut tapped_input: WordSliceInput<'_> = &words;
    let before_tapped = match repeat_till(
        0..,
        any.void(),
        peek(primitives::word_slice_exact("tapped")),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(&mut tapped_input)
    {
        Ok(before) => before,
        Err(_) => return EntersTappedLineShape::NoMatch,
    };
    let trailing = &words[before_tapped.len() + 1..];
    if trailing.is_empty() {
        EntersTappedLineShape::EntersTapped
    } else if words_are_exact(trailing, &["attacking"])
        || words_are_exact(trailing, &["and", "attacking"])
    {
        EntersTappedLineShape::AttackingVariant
    } else {
        EntersTappedLineShape::UnsupportedTrailing
    }
}

fn parse_cost_keyword(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("cost"), primitives::kw("costs")))
        .void()
        .parse_next(input)
}

fn parse_activated_abilities_cost_head<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CostReductionLineHead<'a>> {
    primitives::phrase(&["activated", "abilities", "of"]).parse_next(input)?;
    let subject_tokens = repeat_till(1.., any.void(), peek(parse_cost_keyword))
        .map(|((), ())| ())
        .take()
        .parse_next(input)?;
    parse_cost_keyword.parse_next(input)?;
    let amount_tokens = rest.parse_next(input)?;
    Ok(CostReductionLineHead::ActivatedAbilitiesOf {
        subject_tokens: trim_lexed_commas(subject_tokens),
        amount_tokens: trim_lexed_commas(amount_tokens),
    })
}

pub fn parse_cost_reduction_line_head_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CostReductionLineHead<'_>> {
    let words = crate::lexer::token_word_refs(tokens);
    let head = LexStream::new(tokens);

    let mut this_cost = head.clone();
    if primitives::phrase(&["this", "cost", "is", "reduced", "by"])
        .parse_next(&mut this_cost)
        .is_ok()
        && words.len() > 6
    {
        let amount_tokens = trim_lexed_commas(&tokens[5..]);
        return Some(CostReductionLineHead::ThisCost {
            amount_tokens,
            diagnostic_amount_word: words[5],
            diagnostic_tail: words[6..].join(" "),
        });
    }

    if let Ok(parsed) = parse_activated_abilities_cost_head.parse_next(&mut head.clone())
        && !parsed_subject_is_empty(&parsed)
    {
        return Some(parsed);
    }

    let mut this_ability = head.clone();
    if primitives::phrase(&["this", "ability", "costs"])
        .parse_next(&mut this_ability)
        .is_ok()
    {
        return Some(CostReductionLineHead::ThisAbility {
            amount_tokens: trim_lexed_commas(&tokens[3..]),
        });
    }

    let mut this_spell = head;
    if primitives::phrase(&["this", "spell", "costs"])
        .parse_next(&mut this_spell)
        .is_ok()
    {
        return Some(CostReductionLineHead::ThisSpell {
            amount_tokens: &tokens[3..],
        });
    }
    None
}

fn parsed_subject_is_empty(parsed: &CostReductionLineHead<'_>) -> bool {
    matches!(
        parsed,
        CostReductionLineHead::ActivatedAbilitiesOf { subject_tokens, .. }
            if subject_tokens.is_empty()
    )
}

pub fn parse_this_cost_reduction_remainder_tokens(
    tokens: &[OwnedLexToken],
) -> ThisCostReductionRemainder {
    let words = parser_token_word_refs(tokens);
    if word_present(&words, "for") && word_present(&words, "each") {
        ThisCostReductionRemainder::ForEach
    } else {
        ThisCostReductionRemainder::Other
    }
}

pub fn parse_activated_abilities_reduction_remainder_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ActivatedAbilitiesReductionRemainder> {
    let words = parser_token_word_refs(tokens);
    let mut input: WordSliceInput<'_> = &words;
    crate::grammar::primitives::take_leaf(&mut input, word_phrase(&["less", "to", "activate"]))?;

    let uses_ability_activation_cost = word_phrase_present(
        &words,
        &[
            "this",
            "effect",
            "cant",
            "reduce",
            "the",
            "mana",
            "in",
            "that",
            "abilitys",
            "activation",
            "cost",
            "to",
            "less",
            "than",
            "one",
            "mana",
        ],
    );
    let uses_that_cost = word_phrase_present(
        &words,
        &[
            "this", "effect", "cant", "reduce", "the", "mana", "in", "that", "cost", "to", "less",
            "than", "one", "mana",
        ],
    );
    Some(if uses_ability_activation_cost {
        ActivatedAbilitiesReductionRemainder::MinimumOneManaAbilityActivationCost
    } else if uses_that_cost {
        ActivatedAbilitiesReductionRemainder::MinimumOneMana
    } else {
        ActivatedAbilitiesReductionRemainder::Unbounded
    })
}

pub fn parse_this_ability_reduction_remainder_tokens(
    tokens: &[OwnedLexToken],
) -> ThisAbilityReductionRemainder<'_> {
    let words = parser_token_word_refs(tokens);
    if words_are_exact(&words, &["less", "to", "activate"]) {
        return ThisAbilityReductionRemainder::Unconditional;
    }

    let view = TokenWordView::new(tokens);
    let view_words = view.word_refs();
    let mut conditional: WordSliceInput<'_> = &view_words;
    if word_phrase(&["less", "to", "activate", "if"])
        .parse_next(&mut conditional)
        .is_ok()
    {
        if word_phrase(&["it", "targets"])
            .parse_next(&mut conditional)
            .is_ok()
        {
            let first = view.token_index_after_words(6).unwrap_or(tokens.len());
            return ThisAbilityReductionRemainder::Targets {
                count_and_filter_tokens: trim_lexed_commas(&tokens[first..]),
            };
        }
        return ThisAbilityReductionRemainder::UnsupportedCondition;
    }

    let mut per_each: WordSliceInput<'_> = &view_words;
    if word_phrase(&["less", "to", "activate", "for", "each"])
        .parse_next(&mut per_each)
        .is_ok()
    {
        let first = view.token_index_after_words(5).unwrap_or(tokens.len());
        return ThisAbilityReductionRemainder::ForEach {
            filter_tokens: trim_lexed_commas(&tokens[first..]),
        };
    }
    ThisAbilityReductionRemainder::NotReduction
}

pub fn parse_this_spell_reduction_remainder_tokens(
    tokens: &[OwnedLexToken],
) -> ThisSpellReductionRemainder {
    let words = parser_token_word_refs(tokens);
    if !word_present(&words, "less") {
        return ThisSpellReductionRemainder::NotReduction;
    }
    if word_present(&words, "each")
        && word_phrase_present(&words, &["card", "type"])
        && word_present(&words, "graveyard")
    {
        ThisSpellReductionRemainder::CardTypesInGraveyard
    } else {
        ThisSpellReductionRemainder::General
    }
}

fn parse_next_spell_cost_reduction<'a>(
    input: &mut LexStream<'a>,
) -> WResult<NextSpellCostReductionSpec> {
    primitives::phrase(&["the", "next"]).parse_next(input)?;
    let spell_filter_tokens = repeat_till(0.., any.void(), peek(primitives::kw("spell")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::phrase(&["spell", "you", "cast", "this", "turn"]).parse_next(input)?;
    let _: &[OwnedLexToken] = repeat_till(0.., any.void(), peek(primitives::kw("costs")))
        .map(|((), _)| ())
        .take()
        .parse_next(input)?;
    primitives::kw("costs").parse_next(input)?;
    let reduction = super::leaf::parse_leaf_mana_cost_prefix_lexed
        .parse_next(input)?
        .cost;
    primitives::phrase(&["less", "to", "cast"]).parse_next(input)?;
    let _: Vec<&OwnedLexToken> = repeat(0.., any).parse_next(input)?;
    Ok(NextSpellCostReductionSpec {
        spell_filter: super::filters::parse_spell_filter_with_grammar_entrypoint_lexed(
            trim_lexed_commas(spell_filter_tokens),
        ),
        reduction,
    })
}

pub fn parse_next_spell_cost_reduction_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NextSpellCostReductionSpec> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_next_spell_cost_reduction,
        "next-spell-cost-reduction",
    )
}

fn parse_this_ability_cost_reference_prefix_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<ThisAbilityCostReference> {
    primitives::phrase(&["this", "ability", "costs"]).parse_next(input)?;
    Ok(ThisAbilityCostReference)
}

pub fn parse_this_ability_cost_reference_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ThisAbilityCostReference> {
    primitives::parse_prefix(tokens, parse_this_ability_cost_reference_prefix_lexed)
        .map(|(parsed, _)| parsed)
}

pub fn parse_inline_activated_sentence_kind_tokens(
    tokens: &[OwnedLexToken],
) -> Option<InlineActivatedSentenceKind> {
    let words = parser_token_word_refs(tokens);
    let mut input: WordSliceInput<'_> = &words;
    if word_phrase(&["this", "ability", "costs"])
        .parse_next(&mut input)
        .is_ok()
        && word_phrase_present(&words, &["less", "to", "activate"])
    {
        return Some(InlineActivatedSentenceKind::ThisAbilityCostReduction);
    }

    let mut input: WordSliceInput<'_> = &words;
    if word_phrase(&["the", "next"]).parse_next(&mut input).is_ok()
        && word_present(&words, "spell")
        && word_present(&words, "costs")
        && word_present(&words, "less")
        && word_present(&words, "cast")
    {
        return Some(InlineActivatedSentenceKind::NextSpellCostReduction);
    }
    None
}

pub fn parse_exhaust_once_restriction_tokens(tokens: &[OwnedLexToken]) -> Option<()> {
    let words = parser_token_word_refs(tokens);
    primitives::parse_full_word_slice(
        &words,
        word_phrase(&["activate", "each", "exhaust", "ability", "only", "once"]),
    )
}

fn remove_once_per_turn_tail(words: &mut Vec<String>) {
    let mut index = 0usize;
    while index + 5 <= words.len() {
        let refs = words[index..]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let mut input: WordSliceInput<'_> = &refs;
        if word_phrase(&["and", "only", "once", "each", "turn"])
            .parse_next(&mut input)
            .is_ok()
        {
            words.drain(index..index + 5);
        } else {
            index += 1;
        }
    }
}

pub fn parse_once_per_turn_restriction_normalization_tokens(
    tokens: &[OwnedLexToken],
) -> OncePerTurnRestrictionNormalization {
    let mut words = crate::lexer::token_word_refs(tokens)
        .into_iter()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let refs = words.iter().map(String::as_str).collect::<Vec<_>>();
    if words_are_exact(&refs, &["activate", "only", "once", "each", "turn"]) {
        return OncePerTurnRestrictionNormalization::Redundant;
    }
    let mut prefix: WordSliceInput<'_> = &refs;
    let drop_prefix = word_phrase(&["activate", "only", "once", "each", "turn", "and"])
        .parse_next(&mut prefix)
        .is_ok();
    drop(refs);
    if drop_prefix {
        words.drain(0..6);
    }
    remove_once_per_turn_tail(&mut words);
    if words.is_empty() {
        OncePerTurnRestrictionNormalization::Redundant
    } else {
        OncePerTurnRestrictionNormalization::Residual(words.join(" "))
    }
}

#[cfg(test)]
#[path = "activated_lines_inline_tests.rs"]
mod tests;

use std::ops::Range;

use winnow::combinator::{alt, eof};
use winnow::error::{ContextError, ErrMode, ModalResult as WResult};
use winnow::prelude::*;
use winnow::token::any;

use crate::mana::ManaCost;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenKind, TokenWordView};

use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeywordNormalizedWords {
    pub(crate) words: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum KeywordCumulativeUpkeepCostSurface {
    Empty,
    AddMana(ManaCost),
    Mana(ManaCost),
    ManaOrMana { left: ManaCost, right: ManaCost },
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeywordSingleGraveyardBottomPayment {
    pub(crate) count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct KeywordCostActionSurface {
    pub(crate) mana_cost: Option<ManaCost>,
    pub(crate) has_payload: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordDamageSubjectKind {
    It,
    SourceCandidate { word_count: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeywordDamageSubjectSplit {
    pub(crate) action_first: usize,
    pub(crate) subject: KeywordDamageSubjectKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormalizedWordAtom<'a> {
    Cant,
    Youve,
    Original(&'a str),
}

fn parse_normalized_word_atom<'a>(
    input: &mut primitives::WordSliceInput<'a>,
) -> WResult<NormalizedWordAtom<'a>> {
    alt((
        alt((
            (
                primitives::word_slice_exact("can"),
                primitives::word_slice_exact("t"),
            )
                .value(NormalizedWordAtom::Cant),
            primitives::word_slice_exact("cannot").value(NormalizedWordAtom::Cant),
            primitives::word_slice_exact("can't").value(NormalizedWordAtom::Cant),
            primitives::word_slice_exact("cant").value(NormalizedWordAtom::Cant),
        )),
        alt((
            (
                primitives::word_slice_exact("you"),
                primitives::word_slice_exact("ve"),
            )
                .value(NormalizedWordAtom::Youve),
            primitives::word_slice_exact("you've").value(NormalizedWordAtom::Youve),
            primitives::word_slice_exact("youve").value(NormalizedWordAtom::Youve),
        )),
        any.map(NormalizedWordAtom::Original),
    ))
    .parse_next(input)
}

pub(crate) fn parse_normalized_keyword_words_tokens(
    tokens: &[OwnedLexToken],
) -> KeywordNormalizedWords {
    let view = TokenWordView::new(tokens);
    let word_refs = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &word_refs;
    let mut words = Vec::with_capacity(word_refs.len());
    while !input.is_empty() {
        let normalized = parse_normalized_word_atom
            .parse_next(&mut input)
            .expect("word atom parser always consumes a word");
        words.push(match normalized {
            NormalizedWordAtom::Cant => "cant".to_string(),
            NormalizedWordAtom::Youve => "youve".to_string(),
            NormalizedWordAtom::Original(word) => word.to_string(),
        });
    }
    KeywordNormalizedWords { words }
}

fn parse_cumulative_add_mana<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordCumulativeUpkeepCostSurface> {
    primitives::kw("add").parse_next(input)?;
    let cost = leaf::parse_leaf_mana_cost_prefix_lexed
        .parse_next(input)?
        .cost;
    eof.parse_next(input)?;
    Ok(KeywordCumulativeUpkeepCostSurface::AddMana(cost))
}

fn parse_cumulative_mana_or_mana<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordCumulativeUpkeepCostSurface> {
    let left = leaf::parse_leaf_mana_cost_prefix_lexed
        .parse_next(input)?
        .cost;
    primitives::kw("or").parse_next(input)?;
    let right = leaf::parse_leaf_mana_cost_prefix_lexed
        .parse_next(input)?
        .cost;
    eof.parse_next(input)?;
    Ok(KeywordCumulativeUpkeepCostSurface::ManaOrMana { left, right })
}

fn parse_cumulative_mana<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordCumulativeUpkeepCostSurface> {
    let cost = leaf::parse_leaf_mana_cost_prefix_lexed
        .parse_next(input)?
        .cost;
    eof.parse_next(input)?;
    Ok(KeywordCumulativeUpkeepCostSurface::Mana(cost))
}

pub(crate) fn parse_cumulative_upkeep_cost_surface_tokens(
    tokens: &[OwnedLexToken],
) -> KeywordCumulativeUpkeepCostSurface {
    if tokens.is_empty() {
        return KeywordCumulativeUpkeepCostSurface::Empty;
    }
    for parser in [
        parse_cumulative_add_mana,
        parse_cumulative_mana_or_mana,
        parse_cumulative_mana,
    ] {
        if let Ok(parsed) = primitives::parse_all(tokens, parser, "cumulative-upkeep-cost") {
            return parsed;
        }
    }
    KeywordCumulativeUpkeepCostSurface::Text
}

fn parse_dynamic_life_tail<'a>(input: &mut LexStream<'a>) -> WResult<Range<usize>> {
    let initial_len = input.len();
    primitives::kw("and").parse_next(input)?;
    let value_start = initial_len.saturating_sub(input.len());
    if input.len() < 2 {
        return Err(primitives::backtrack_err(
            "dynamic mana payment tail",
            "a value followed by life",
        ));
    }
    while input.len() > 1 {
        let _: &'a OwnedLexToken = any.parse_next(input)?;
    }
    let value_end = initial_len.saturating_sub(input.len());
    primitives::kw("life").parse_next(input)?;
    eof.parse_next(input)?;
    Ok(value_start..value_end)
}

pub(crate) fn parse_keyword_dynamic_life_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<Range<usize>> {
    primitives::parse_all(tokens, parse_dynamic_life_tail, "keyword-dynamic-life-tail").ok()
}

fn parse_single_graveyard_bottom_payment<'a>(
    input: &mut LexStream<'a>,
) -> WResult<KeywordSingleGraveyardBottomPayment> {
    primitives::kw("put").parse_next(input)?;
    let count = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    if count == 0 {
        return Err(primitives::backtrack_err(
            "single-graveyard payment",
            "positive card count",
        ));
    }
    alt((primitives::kw("card"), primitives::kw("cards")))
        .void()
        .parse_next(input)?;
    primitives::phrase(&[
        "from",
        "a",
        "single",
        "graveyard",
        "on",
        "the",
        "bottom",
        "of",
    ])
    .parse_next(input)?;
    alt((primitives::kw("its"), primitives::kw("their")))
        .void()
        .parse_next(input)?;

    let mut owner = false;
    let mut library = false;
    while !input.is_empty() {
        let token: &'a OwnedLexToken = any.parse_next(input)?;
        owner |= token.is_any_word(&["owner", "owners", "owner's", "owners'"]);
        library |= token.is_word("library");
    }
    if !owner || !library {
        return Err(primitives::backtrack_err(
            "single-graveyard payment",
            "owner's library",
        ));
    }
    Ok(KeywordSingleGraveyardBottomPayment { count })
}

pub(crate) fn parse_single_graveyard_bottom_payment_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordSingleGraveyardBottomPayment> {
    primitives::parse_all(
        tokens,
        parse_single_graveyard_bottom_payment,
        "single-graveyard-bottom-payment",
    )
    .ok()
}

fn parse_dynamic_word<'a>(
    input: &mut primitives::WordSliceInput<'a>,
    expected: &str,
) -> Result<&'a str, ErrMode<ContextError>> {
    let word: &str = any.parse_next(input)?;
    if word == expected {
        Ok(word)
    } else {
        Err(primitives::backtrack_err(
            "keyword head",
            "requested keyword",
        ))
    }
}

pub(crate) fn parse_keyword_cost_action_surface_tokens(
    tokens: &[OwnedLexToken],
    keyword: &'static str,
) -> Option<KeywordCostActionSurface> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let mut input: primitives::WordSliceInput<'_> = &words;
    parse_dynamic_word(&mut input, keyword).ok()?;
    if input
        .first()
        .is_some_and(|word| matches!(*word, "cost" | "costs"))
    {
        return None;
    }

    let first_payload = view.token_index_after_words(1)?;
    let mut mana_tokens = &tokens[first_payload..];
    while mana_tokens
        .first()
        .is_some_and(|token| matches!(token.kind, TokenKind::Dash | TokenKind::EmDash))
    {
        mana_tokens = &mana_tokens[1..];
    }
    let mana_cost = leaf::parse_leaf_mana_cost_prefix_tokens(mana_tokens).map(|prefix| prefix.cost);
    Some(KeywordCostActionSurface {
        mana_cost,
        has_payload: !input.is_empty(),
    })
}

fn parse_damage_verb(input: &mut primitives::WordSliceInput<'_>) -> WResult<()> {
    alt((
        primitives::word_slice_exact("deal"),
        primitives::word_slice_exact("deals"),
    ))
    .void()
    .parse_next(input)
}

pub(crate) fn parse_keyword_damage_subject_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<KeywordDamageSubjectSplit> {
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    if words.len() < 2 {
        return None;
    }
    let mut first_action: primitives::WordSliceInput<'_> = &words[1..];
    if words[0] == "it" && parse_damage_verb.parse_next(&mut first_action).is_ok() {
        let action_first = view.token_span_for_words(1, 2)?.start;
        return Some(KeywordDamageSubjectSplit {
            action_first,
            subject: KeywordDamageSubjectKind::It,
        });
    }

    for word_count in 1..words.len() {
        let mut candidate: primitives::WordSliceInput<'_> = &words[word_count..];
        if parse_damage_verb.parse_next(&mut candidate).is_ok() {
            let action_first = view.token_span_for_words(word_count, word_count + 1)?.start;
            return Some(KeywordDamageSubjectSplit {
                action_first,
                subject: KeywordDamageSubjectKind::SourceCandidate { word_count },
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    fn lex(raw: &str) -> Vec<OwnedLexToken> {
        lex_line(raw, 0).unwrap()
    }

    #[test]
    fn normalizes_contractions_with_typed_word_parser() {
        assert_eq!(
            parse_normalized_keyword_words_tokens(&lex("You can't and you've")).words,
            ["you", "cant", "and", "youve"]
        );
    }

    #[test]
    fn classifies_cumulative_upkeep_cost_surfaces() {
        assert!(matches!(
            parse_cumulative_upkeep_cost_surface_tokens(&lex("Add {G}")),
            KeywordCumulativeUpkeepCostSurface::AddMana(_)
        ));
        assert!(matches!(
            parse_cumulative_upkeep_cost_surface_tokens(&lex("{W} or {U}")),
            KeywordCumulativeUpkeepCostSurface::ManaOrMana { .. }
        ));
        assert_eq!(
            parse_keyword_dynamic_life_tail_tokens(&lex("and three life")),
            Some(1..2)
        );
    }

    #[test]
    fn parses_single_graveyard_payment_and_keyword_cost_head() {
        assert_eq!(
            parse_single_graveyard_bottom_payment_tokens(&lex(
                "Put two cards from a single graveyard on the bottom of their owner's library"
            )),
            Some(KeywordSingleGraveyardBottomPayment { count: 2 })
        );
        let surface =
            parse_keyword_cost_action_surface_tokens(&lex("Unearth {2}{B}"), "unearth").unwrap();
        assert!(surface.mana_cost.is_some());
        assert!(surface.has_payload);
    }

    #[test]
    fn locates_damage_subject_action_boundary() {
        let split = parse_keyword_damage_subject_split_tokens(&lex("This creature deals 2 damage"))
            .unwrap();
        assert_eq!(
            split.subject,
            KeywordDamageSubjectKind::SourceCandidate { word_count: 2 }
        );
        assert_eq!(split.action_first, 2);
    }
}

use winnow::combinator::{alt, eof};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::types::SubtypeFamily;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistentAnthemTailHead {
    pub tokens: Vec<OwnedLexToken>,
    pub get_token: usize,
    pub modifier_word: String,
    pub tail_start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuingSegmentShape<'a> {
    CantBlock,
    CantAttackAlone,
    MustAttack,
    CantBeBlockedByMoreThan(usize),
    SetColor { color_word: &'a str },
    BeEverySubtype(SubtypeFamily),
    Lose { ability_tokens: &'a [OwnedLexToken] },
    Have { ability_tokens: &'a [OwnedLexToken] },
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordActivatedSplit<'a> {
    pub keyword_tokens: &'a [OwnedLexToken],
    pub activated_tokens: &'a [OwnedLexToken],
}

pub fn parse_persistent_anthem_tail_head(
    tokens: &[OwnedLexToken],
) -> Option<PersistentAnthemTailHead> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if primitives::find_prefix(tokens, || {
        primitives::phrase(&["until", "end", "of", "turn"])
    })
    .is_some()
    {
        return None;
    }
    let get_token = find_word(tokens, 0, parse_get_word)?;
    let mut work_tokens = tokens.to_vec();
    if primitives::parse_prefix(
        work_tokens.get(get_token + 1..)?,
        (
            alt((primitives::kw("a"), primitives::kw("an"))),
            primitives::kw("additional"),
        )
            .void(),
    )
    .is_some()
    {
        work_tokens.drain(get_token + 1..get_token + 3);
    }
    let mut modifier_input = LexStream::new(work_tokens.get(get_token + 1..)?);
    let modifier_word =
        crate::grammar::primitives::take_leaf(&mut modifier_input, primitives::word_text)?
            .to_string();
    Some(PersistentAnthemTailHead {
        tokens: work_tokens,
        get_token,
        modifier_word,
        tail_start: get_token + 2,
    })
}

pub fn parse_direct_have_tail(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (_, rest) = primitives::parse_prefix(
        tokens,
        alt((
            (primitives::kw("and"), parse_have_word).void(),
            parse_have_word,
        )),
    )?;
    Some(trim_lexed_commas(rest))
}

pub fn parse_continuing_segment_shape(tokens: &[OwnedLexToken]) -> ContinuingSegmentShape<'_> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    if parse_complete_any_phrase(
        tokens,
        &[
            &["cant", "block"],
            &["can't", "block"],
            &["cannot", "block"],
            &["can", "t", "block"],
        ],
    ) {
        return ContinuingSegmentShape::CantBlock;
    }
    if parse_complete_any_phrase(
        tokens,
        &[
            &["cant", "attack", "alone"],
            &["can't", "attack", "alone"],
            &["cannot", "attack", "alone"],
            &["can", "t", "attack", "alone"],
        ],
    ) {
        return ContinuingSegmentShape::CantAttackAlone;
    }
    if parse_complete_any_phrase(
        tokens,
        &[
            &["attacks", "each", "combat", "if", "able"],
            &["attack", "each", "combat", "if", "able"],
            &["and", "attack", "each", "combat", "if", "able"],
            &["and", "attacks", "each", "combat", "if", "able"],
        ],
    ) {
        return ContinuingSegmentShape::MustAttack;
    }
    if let Ok(maximum) = primitives::parse_all(
        tokens,
        parse_cant_be_blocked_maximum,
        "cant-be-blocked maximum",
    ) {
        return ContinuingSegmentShape::CantBeBlockedByMoreThan(maximum);
    }
    if let Ok(color_word) =
        primitives::parse_all(tokens, parse_color_assignment, "anthem color assignment")
    {
        return ContinuingSegmentShape::SetColor { color_word };
    }
    if let Some((_, subtype_tokens)) = primitives::parse_prefix(
        tokens,
        alt(((primitives::kw("and"), parse_be_word).void(), parse_be_word)),
    ) && let Some(family) = super::parse_every_subtype_family_tokens(subtype_tokens)
    {
        return ContinuingSegmentShape::BeEverySubtype(family);
    }
    if let Some((_, ability_tokens)) = primitives::parse_prefix(tokens, parse_lose_word) {
        return ContinuingSegmentShape::Lose {
            ability_tokens: trim_lexed_commas(ability_tokens),
        };
    }
    if let Some((_, ability_tokens)) = primitives::parse_prefix(tokens, parse_have_word) {
        return ContinuingSegmentShape::Have {
            ability_tokens: trim_lexed_commas(ability_tokens),
        };
    }
    ContinuingSegmentShape::Other
}

pub fn strip_must_attack_suffix(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let (head, _) = primitives::split_lexed_once_before_suffix(tokens, 1, || {
        primitives::any_phrase(&[
            &["and", "attacks", "each", "combat", "if", "able"],
            &["and", "attack", "each", "combat", "if", "able"],
            &["attacks", "each", "combat", "if", "able"],
            &["attack", "each", "combat", "if", "able"],
        ])
    })?;
    let head = trim_lexed_commas(head);
    (!head.is_empty()).then_some(head)
}

pub fn split_keyword_and_activated(tokens: &[OwnedLexToken]) -> Option<KeywordActivatedSplit<'_>> {
    let tokens = super::trim_anthem_clause_tokens(tokens);
    let colon =
        primitives::find_prefix(tokens, || primitives::token_kind(TokenKind::Colon).void())?.0;
    let and_token = last_and_before(tokens, colon)?;
    let keyword_tokens = trim_lexed_commas(&tokens[..and_token]);
    let activated_tokens = trim_lexed_commas(&tokens[and_token + 1..]);
    (!keyword_tokens.is_empty() && !activated_tokens.is_empty()).then_some(KeywordActivatedSplit {
        keyword_tokens,
        activated_tokens,
    })
}

fn parse_cant_be_blocked_maximum(input: &mut LexStream<'_>) -> WResult<usize> {
    primitives::any_phrase(&[
        &["cant", "be", "blocked", "by"],
        &["can't", "be", "blocked", "by"],
        &["cannot", "be", "blocked", "by"],
        &["can", "t", "be", "blocked", "by"],
    ])
    .parse_next(input)?;
    let maximum = alt((
        (
            primitives::phrase(&["more", "than"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, count)| count),
        (
            leaf::parse_leaf_number_prefix_lexed,
            primitives::phrase(&["or", "more"]),
        )
            .map(|(count, _)| count.saturating_sub(1)),
        (
            primitives::phrase(&["at", "least"]),
            leaf::parse_leaf_number_prefix_lexed,
        )
            .map(|(_, count)| count.saturating_sub(1)),
    ))
    .parse_next(input)?;
    alt((primitives::kw("creature"), primitives::kw("creatures"))).parse_next(input)?;
    eof.parse_next(input)?;
    usize::try_from(maximum).map_err(|_| primitives::backtrack_err("blocker count", "usize"))
}

fn parse_color_assignment<'a>(input: &mut LexStream<'a>) -> WResult<&'a str> {
    parse_be_word.parse_next(input)?;
    let color_word = primitives::word_text(input)?;
    eof.parse_next(input)?;
    Ok(color_word)
}

fn find_word<'a>(
    tokens: &'a [OwnedLexToken],
    start: usize,
    mut parser: impl Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
) -> Option<usize> {
    let search = tokens.get(start..)?;
    let mut input = LexStream::new(search);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser.parse_next(&mut candidate).is_ok() {
            return Some(start + offset);
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        parsed.ok()?;
    }
}

fn parse_complete_any_phrase(
    tokens: &[OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> bool {
    primitives::parse_all(
        tokens,
        primitives::any_phrase(phrases),
        "anthem trailing shape",
    )
    .is_ok()
}

fn parse_get_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("get"), primitives::kw("gets")))
        .void()
        .parse_next(input)
}

fn parse_have_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("have"), primitives::kw("has")))
        .void()
        .parse_next(input)
}

fn parse_lose_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("lose"), primitives::kw("loses")))
        .void()
        .parse_next(input)
}

fn parse_be_word(input: &mut LexStream<'_>) -> WResult<()> {
    alt((primitives::kw("is"), primitives::kw("are")))
        .void()
        .parse_next(input)
}

fn last_and_before(tokens: &[OwnedLexToken], end: usize) -> Option<usize> {
    let mut input = LexStream::new(tokens.get(..end)?);
    let initial_len = input.len();
    let mut last = None;
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if primitives::kw("and").parse_next(&mut candidate).is_ok() {
            last = Some(offset);
        }
        let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
        if parsed.is_err() {
            return last;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        lex_line(text, 0).expect("lex fixture")
    }

    #[test]
    fn classifies_restriction_and_blocker_segments() {
        assert_eq!(
            parse_continuing_segment_shape(&lex("can't block")),
            ContinuingSegmentShape::CantBlock
        );
        assert_eq!(
            parse_continuing_segment_shape(&lex("can't be blocked by more than one creature")),
            ContinuingSegmentShape::CantBeBlockedByMoreThan(1)
        );
        assert_eq!(
            parse_continuing_segment_shape(&lex("is every creature type")),
            ContinuingSegmentShape::BeEverySubtype(SubtypeFamily::Creature)
        );
        assert_eq!(
            parse_continuing_segment_shape(&lex("and is every land type")),
            ContinuingSegmentShape::BeEverySubtype(SubtypeFamily::Land)
        );
    }
}

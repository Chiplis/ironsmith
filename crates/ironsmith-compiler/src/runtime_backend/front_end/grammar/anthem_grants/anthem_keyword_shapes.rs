use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use crate::color::ColorSet;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnthemKeywordOrder {
    KeywordBeforeAnthem,
    AnthemBeforeKeyword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnthemKeywordHead {
    pub(crate) get_token: usize,
    pub(crate) have_token: usize,
    pub(crate) order: AnthemKeywordOrder,
    pub(crate) pre_grant_is_temporary: bool,
    pub(crate) clause_tail_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KeywordBeforeAnthemShape<'a> {
    pub(crate) subject_tokens: &'a [OwnedLexToken],
    pub(crate) keyword_tokens: &'a [OwnedLexToken],
    pub(crate) anthem_tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnthemKeywordColorSegment {
    pub(crate) is_token: usize,
    pub(crate) color: ColorSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnthemKeywordCompoundSplit {
    pub(crate) split_token: usize,
    pub(crate) tail_start: usize,
    pub(crate) second_get_token: Option<usize>,
    pub(crate) second_tail_end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AnthemKeywordTrailingCondition<'a> {
    pub(crate) ability_tokens: &'a [OwnedLexToken],
    pub(crate) condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnthemKeywordTrailingConditionError {
    MissingAbility,
    MissingCondition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenTailSplit<'a> {
    pub(crate) head_tokens: &'a [OwnedLexToken],
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ColonTailSplit {
    pub(crate) colon_token: usize,
    pub(crate) last_and_before_colon: Option<usize>,
}

pub(crate) fn parse_anthem_keyword_head(tokens: &[OwnedLexToken]) -> Option<AnthemKeywordHead> {
    let get_token = first_token(tokens, parse_get_word)?;
    let have_token = first_token(tokens, parse_have_word)?;
    if get_token == have_token {
        return None;
    }
    let order = if have_token < get_token {
        AnthemKeywordOrder::KeywordBeforeAnthem
    } else {
        AnthemKeywordOrder::AnthemBeforeKeyword
    };
    let pre_grant_is_temporary = order == AnthemKeywordOrder::AnthemBeforeKeyword
        && contains_parser(&tokens[..have_token], || {
            primitives::phrase(&["until", "end", "of", "turn"]).void()
        });
    let clause_tail_end = if order == AnthemKeywordOrder::AnthemBeforeKeyword
        && have_token > get_token + 2
        && token_matches(&tokens[have_token - 1], parse_and_word)
    {
        have_token - 1
    } else {
        have_token
    };
    Some(AnthemKeywordHead {
        get_token,
        have_token,
        order,
        pre_grant_is_temporary,
        clause_tail_end,
    })
}

pub(crate) fn parse_keyword_before_anthem_shape(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> Option<KeywordBeforeAnthemShape<'_>> {
    if head.order != AnthemKeywordOrder::KeywordBeforeAnthem {
        return None;
    }
    let subject_tokens = trim_edge_punctuation(tokens.get(..head.have_token)?);
    let mut keyword_tokens =
        trim_edge_punctuation(tokens.get(head.have_token.saturating_add(1)..head.get_token)?);
    if keyword_tokens
        .first()
        .is_some_and(|token| token_matches(token, parse_and_word))
    {
        keyword_tokens = trim_edge_punctuation(&keyword_tokens[1..]);
    }
    if keyword_tokens
        .last()
        .is_some_and(|token| token_matches(token, parse_and_word))
    {
        keyword_tokens =
            trim_edge_punctuation(&keyword_tokens[..keyword_tokens.len().saturating_sub(1)]);
    }
    if subject_tokens.is_empty() || keyword_tokens.is_empty() {
        return None;
    }
    Some(KeywordBeforeAnthemShape {
        subject_tokens,
        keyword_tokens,
        anthem_tail_tokens: tokens.get(head.get_token..)?,
    })
}

pub(crate) fn parse_anthem_keyword_color_segment(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> Option<AnthemKeywordColorSegment> {
    if head.order != AnthemKeywordOrder::AnthemBeforeKeyword {
        return None;
    }
    let start = head.get_token.saturating_add(2);
    let is_token = first_token_between(tokens, start, head.have_token, parse_is_word)?;
    let color_word = tokens.get(is_token + 1)?.as_word()?;
    let color = leaf::parse_leaf_color_complete(color_word).ok()?;
    Some(AnthemKeywordColorSegment { is_token, color })
}

pub(crate) fn parse_anthem_keyword_compound_split(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> Option<AnthemKeywordCompoundSplit> {
    if head.order != AnthemKeywordOrder::AnthemBeforeKeyword {
        return None;
    }
    if modifier_tail_is_attached_for_each_count(tokens, head) {
        return None;
    }
    let split_end = head.have_token.saturating_sub(1);
    let split_token = first_token_between(
        tokens,
        head.get_token.saturating_add(2),
        split_end,
        parse_and_word,
    )?;
    let tail_start = split_token + 1;
    let second_get_token = first_token_between(tokens, tail_start, head.have_token, parse_get_word);
    let second_tail_end = match second_get_token {
        Some(second_get)
            if head.have_token > second_get + 2
                && token_matches(&tokens[head.have_token - 1], parse_and_word) =>
        {
            head.have_token - 1
        }
        _ => head.have_token,
    };
    Some(AnthemKeywordCompoundSplit {
        split_token,
        tail_start,
        second_get_token,
        second_tail_end,
    })
}

fn modifier_tail_is_attached_for_each_count(
    tokens: &[OwnedLexToken],
    head: AnthemKeywordHead,
) -> bool {
    let Some(modifier) = super::parse_modifier_shape(tokens, head.get_token, head.clause_tail_end)
    else {
        return false;
    };
    let Some(super::AnthemTailShape::ForEach(tail)) = super::parse_tail_shape(modifier.tail_tokens)
    else {
        return false;
    };
    let Some(rest) = super::parse_for_each_rest(tail) else {
        return false;
    };
    matches!(
        super::parse_for_each_special_shape(rest),
        Some(super::ForEachSpecialShape::AttachedToSource { .. })
    )
}

pub(crate) fn split_anthem_keyword_trailing_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<AnthemKeywordTrailingCondition<'_>>, AnthemKeywordTrailingConditionError> {
    let tokens = trim_edge_punctuation(tokens);
    let Some((as_token, condition_start)) = first_phrase(tokens, parse_as_long_as) else {
        return Ok(None);
    };
    let ability_tokens = trim_edge_punctuation(&tokens[..as_token]);
    if ability_tokens.is_empty() {
        return Err(AnthemKeywordTrailingConditionError::MissingAbility);
    }
    let condition_tokens = trim_edge_punctuation(&tokens[condition_start..]);
    if condition_tokens.is_empty() {
        return Err(AnthemKeywordTrailingConditionError::MissingCondition);
    }
    Ok(Some(AnthemKeywordTrailingCondition {
        ability_tokens,
        condition_tokens,
    }))
}

pub(crate) fn split_anthem_keyword_and_is(tokens: &[OwnedLexToken]) -> Option<TokenTailSplit<'_>> {
    split_adjacent_pair(tokens, parse_and_word, parse_is_word, 1, true)
}

pub(crate) fn split_anthem_keyword_and_have(
    tokens: &[OwnedLexToken],
) -> Option<TokenTailSplit<'_>> {
    split_adjacent_pair(tokens, parse_and_word, parse_have_word, 2, false)
}

pub(crate) fn parse_colon_tail_split(tokens: &[OwnedLexToken]) -> Option<ColonTailSplit> {
    let colon_token = first_kind(tokens, TokenKind::Colon)?;
    let last_and_before_colon = last_token_before(tokens, colon_token, parse_and_word);
    Some(ColonTailSplit {
        colon_token,
        last_and_before_colon,
    })
}

fn split_adjacent_pair<'a>(
    tokens: &'a [OwnedLexToken],
    first: fn(&mut LexStream<'a>) -> WResult<()>,
    second: fn(&mut LexStream<'a>) -> WResult<()>,
    tail_offset: usize,
    require_head: bool,
) -> Option<TokenTailSplit<'a>> {
    let tokens = trim_edge_punctuation(tokens);
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if first(&mut candidate).is_ok() && second(&mut candidate).is_ok() {
            let head_tokens = trim_edge_punctuation(&tokens[..offset]);
            let tail_tokens = trim_edge_punctuation(&tokens[offset + tail_offset..]);
            if (require_head && head_tokens.is_empty()) || tail_tokens.is_empty() {
                return None;
            }
            return Some(TokenTailSplit {
                head_tokens,
                tail_tokens,
            });
        }
        take_token(&mut input).ok()?;
    }
}

fn first_token(
    tokens: &[OwnedLexToken],
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<usize> {
    first_token_between(tokens, 0, tokens.len(), parser)
}

fn first_token_between(
    tokens: &[OwnedLexToken],
    start: usize,
    end: usize,
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<usize> {
    if start >= end || end > tokens.len() {
        return None;
    }
    let mut input = LexStream::new(&tokens[start..end]);
    let initial_len = input.len();
    loop {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser(&mut candidate).is_ok() {
            return Some(start + offset);
        }
        take_token(&mut input).ok()?;
    }
}

fn first_phrase(
    tokens: &[OwnedLexToken],
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<(usize, usize)> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    loop {
        let start = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser(&mut candidate).is_ok() {
            return Some((start, initial_len.saturating_sub(candidate.len())));
        }
        take_token(&mut input).ok()?;
    }
}

fn first_kind(tokens: &[OwnedLexToken], kind: TokenKind) -> Option<usize> {
    let mut input = LexStream::new(tokens);
    let initial_len = input.len();
    while let Ok(token) = take_token(&mut input) {
        if token.kind == kind {
            return Some(initial_len.saturating_sub(input.len() + 1));
        }
    }
    None
}

fn last_token_before(
    tokens: &[OwnedLexToken],
    end: usize,
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> Option<usize> {
    let mut input = LexStream::new(tokens.get(..end)?);
    let initial_len = input.len();
    let mut found = None;
    while !input.is_empty() {
        let offset = initial_len.saturating_sub(input.len());
        let mut candidate = input.clone();
        if parser(&mut candidate).is_ok() {
            found = Some(offset);
        }
        take_token(&mut input).ok()?;
    }
    found
}

fn contains_parser<'a, P, F>(tokens: &'a [OwnedLexToken], make_parser: F) -> bool
where
    F: Fn() -> P,
    P: Parser<LexStream<'a>, (), winnow::error::ErrMode<winnow::error::ContextError>>,
{
    let mut input = LexStream::new(tokens);
    loop {
        let mut candidate = input.clone();
        if make_parser().parse_next(&mut candidate).is_ok() {
            return true;
        }
        if take_token(&mut input).is_err() {
            return false;
        }
    }
}

fn token_matches(
    token: &OwnedLexToken,
    parser: for<'a> fn(&mut LexStream<'a>) -> WResult<()>,
) -> bool {
    let mut input = LexStream::new(std::slice::from_ref(token));
    parser(&mut input).is_ok()
}

fn trim_edge_punctuation(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1].kind,
            TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
        )
    {
        end -= 1;
    }
    &tokens[start..end]
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

fn parse_and_word(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("and").void().parse_next(input)
}

fn parse_is_word(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::kw("is").void().parse_next(input)
}

fn parse_as_long_as(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::phrase(&["as", "long", "as"])
        .void()
        .parse_next(input)
}

fn take_token<'a>(input: &mut LexStream<'a>) -> WResult<&'a OwnedLexToken> {
    any.parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    #[test]
    fn parses_both_anthem_keyword_orders() {
        let tokens = lex_line(
            "Equipped creature has first strike and gets +1/+0 for each instant and sorcery card in your graveyard.",
            0,
        )
        .unwrap();
        let head = parse_anthem_keyword_head(&tokens).unwrap();
        assert_eq!(head.order, AnthemKeywordOrder::KeywordBeforeAnthem);
        let shape = parse_keyword_before_anthem_shape(&tokens, head).unwrap();
        assert!(!shape.subject_tokens.is_empty());
        assert_eq!(
            crate::runtime_backend::lexer::parser_token_word_refs(shape.keyword_tokens),
            ["first", "strike"]
        );
        assert_eq!(shape.anthem_tail_tokens[0].parser_text(), "gets");

        let tokens = lex_line("Creatures you control get +1/+1 and have flying.", 0).unwrap();
        let head = parse_anthem_keyword_head(&tokens).unwrap();
        assert_eq!(head.order, AnthemKeywordOrder::AnthemBeforeKeyword);
    }

    #[test]
    fn parses_color_and_compound_segments() {
        let tokens =
            lex_line("This creature gets +1/+1, is red, and has {T}: Add {R}.", 0).unwrap();
        let head = parse_anthem_keyword_head(&tokens).unwrap();
        assert_eq!(
            parse_anthem_keyword_color_segment(&tokens, head)
                .unwrap()
                .color,
            ColorSet::RED
        );

        let tokens = lex_line(
            "Creatures you control get +1/+1 and are red and have flying.",
            0,
        )
        .unwrap();
        let head = parse_anthem_keyword_head(&tokens).unwrap();
        assert!(parse_anthem_keyword_color_segment(&tokens, head).is_none());

        let tokens = lex_line(
            "Creatures you control get +1/+1 and are red, and creatures you control get +0/+1 and have flying.",
            0,
        )
        .unwrap();
        let head = parse_anthem_keyword_head(&tokens).unwrap();
        assert!(parse_anthem_keyword_compound_split(&tokens, head).is_some());

        let attached_count = lex_line(
            "Equipped creature gets +1/+1 for each Aura and Equipment attached to it and has ward {2}.",
            0,
        )
        .unwrap();
        let head = parse_anthem_keyword_head(&attached_count).unwrap();
        assert!(parse_anthem_keyword_compound_split(&attached_count, head).is_none());
    }

    #[test]
    fn splits_conditions_additions_and_activated_tails() {
        let tokens = lex_line("flying as long as you control an artifact", 0).unwrap();
        let split = split_anthem_keyword_trailing_condition(&tokens)
            .unwrap()
            .unwrap();
        assert_eq!(split.ability_tokens.len(), 1);
        assert!(!split.condition_tokens.is_empty());

        let tokens = lex_line("flying and is red", 0).unwrap();
        assert!(split_anthem_keyword_and_is(&tokens).is_some());

        let tokens = lex_line("flying and has {T}: Add {G}.", 0).unwrap();
        let split = split_anthem_keyword_and_have(&tokens).unwrap();
        assert!(parse_colon_tail_split(split.tail_tokens).is_some());
    }
}

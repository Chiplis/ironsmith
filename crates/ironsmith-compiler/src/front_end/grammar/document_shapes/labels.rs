use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::super::primitives;
use crate::lexer::{LexStream, OwnedLexToken, TokenKind};
use crate::token_primitives::split_em_dash_label_prefix_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreservedKeywordLabelKind {
    CostOrCasting,
    Activated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LabelPrefixKind {
    PreservedKeyword(PreservedKeywordLabelKind),
    CouncilChoice,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NumericResultPrefixShape;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatementLabelSplitShape<'a> {
    pub(crate) label_tokens: &'a [OwnedLexToken],
    pub(crate) body_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StatementLabelStripShape<'a> {
    pub(crate) body_tokens: &'a [OwnedLexToken],
    pub(crate) stripped_labels: usize,
}

pub(crate) fn parse_label_prefix_kind_tokens(tokens: &[OwnedLexToken]) -> Option<LabelPrefixKind> {
    primitives::parse_prefix(tokens, council_choice_label)
        .map(|((), _)| LabelPrefixKind::CouncilChoice)
        .or_else(|| {
            primitives::parse_prefix(tokens, preserved_keyword_label)
                .map(|(kind, _)| LabelPrefixKind::PreservedKeyword(kind))
        })
}

pub(crate) fn parse_preserved_keyword_label_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PreservedKeywordLabelKind> {
    match parse_label_prefix_kind_tokens(tokens)? {
        LabelPrefixKind::PreservedKeyword(kind) => Some(kind),
        LabelPrefixKind::CouncilChoice => None,
    }
}

pub(crate) fn parse_numeric_result_prefix_tokens(
    tokens: &[OwnedLexToken],
) -> Option<NumericResultPrefixShape> {
    if matches!(tokens, [number, pipe, ..]
        if number.kind == TokenKind::Number && pipe.kind == TokenKind::Pipe)
    {
        return Some(NumericResultPrefixShape);
    }
    if tokens
        .first()
        .is_some_and(token_is_compact_ascii_numeric_range)
        && tokens
            .get(1)
            .is_some_and(|token| token.kind == TokenKind::Pipe)
    {
        return Some(NumericResultPrefixShape);
    }
    let (_, remaining) = primitives::parse_prefix(tokens, numeric_result_head)?;
    primitives::find_prefix(remaining, || primitives::token_kind(TokenKind::Pipe).void())?;
    Some(NumericResultPrefixShape)
}

fn token_is_compact_ascii_numeric_range(token: &OwnedLexToken) -> bool {
    if token.kind != TokenKind::Word {
        return false;
    }
    let Some((min, max)) = token.parser_text().split_once('-') else {
        return false;
    };
    if min.is_empty()
        || max.is_empty()
        || !min.bytes().all(|byte| byte.is_ascii_digit())
        || !max.bytes().all(|byte| byte.is_ascii_digit())
    {
        return false;
    }
    matches!(
        (min.parse::<i32>(), max.parse::<i32>()),
        (Ok(min), Ok(max)) if min <= max
    )
}

pub(crate) fn parse_statement_label_split_tokens(
    tokens: &[OwnedLexToken],
) -> Option<StatementLabelSplitShape<'_>> {
    if parse_numeric_result_prefix_tokens(tokens).is_some() {
        return None;
    }
    let (label_tokens, body_tokens) = split_em_dash_label_prefix_tokens(tokens)?;
    (!label_tokens.is_empty() && !body_tokens.is_empty()).then_some(StatementLabelSplitShape {
        label_tokens,
        body_tokens,
    })
}

pub(crate) fn parse_statement_label_strip_tokens(
    mut tokens: &[OwnedLexToken],
) -> StatementLabelStripShape<'_> {
    let mut stripped_labels = 0;
    while let Some(split) = parse_statement_label_split_tokens(tokens) {
        if parse_preserved_keyword_label_tokens(split.label_tokens).is_some() {
            break;
        }
        stripped_labels += 1;
        tokens = split.body_tokens;
    }
    StatementLabelStripShape {
        body_tokens: tokens,
        stripped_labels,
    }
}

fn numeric_result_head(input: &mut LexStream<'_>) -> WResult<()> {
    primitives::token_kind(TokenKind::Number)
        .void()
        .parse_next(input)?;
    winnow::combinator::alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
    ))
    .void()
    .parse_next(input)?;
    primitives::token_kind(TokenKind::Number)
        .void()
        .parse_next(input)
}

fn council_choice_label(input: &mut LexStream<'_>) -> WResult<()> {
    winnow::combinator::alt((
        primitives::phrase(&["will", "of", "the", "council"]),
        primitives::phrase(&["council's", "dilemma"]),
        primitives::phrase(&["secret", "council"]),
    ))
    .parse_next(input)
}

fn preserved_keyword_label(input: &mut LexStream<'_>) -> WResult<PreservedKeywordLabelKind> {
    let head = primitives::word_parser_text.parse_next(input)?;
    match head {
        "buyback" | "blitz" | "bestow" | "cumulative" | "cycling" | "echo" | "equip" | "epic"
        | "escape" | "escalate" | "eternalize" | "evoke" | "flashback" | "kicker"
        | "multikicker" | "modular" | "morph" | "megamorph" | "prototype" | "replicate"
        | "reinforce" | "splice" | "squad" | "spectacle" | "strive" | "surge" | "suspend"
        | "ward" => Ok(PreservedKeywordLabelKind::CostOrCasting),
        "boast" | "renew" => Ok(PreservedKeywordLabelKind::Activated),
        _ => Err(primitives::backtrack_err(
            "keyword label",
            "known keyword label head",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn classifies_keyword_and_council_labels() {
        let kicker = lex_line("Kicker", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&kicker),
            Some(LabelPrefixKind::PreservedKeyword(
                PreservedKeywordLabelKind::CostOrCasting
            ))
        );
        let prototype = lex_line("Prototype {2}{R}", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&prototype),
            Some(LabelPrefixKind::PreservedKeyword(
                PreservedKeywordLabelKind::CostOrCasting
            ))
        );
        let council = lex_line("Council's dilemma", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&council),
            Some(LabelPrefixKind::CouncilChoice)
        );

        let council_with_dash = lex_line("Council's dilemma —", 0).unwrap();
        assert_eq!(
            parse_label_prefix_kind_tokens(&council_with_dash),
            Some(LabelPrefixKind::CouncilChoice)
        );
    }

    #[test]
    fn statement_label_parser_preserves_numeric_result_tables_and_keyword_labels() {
        let numeric = lex_line("1—4 | Create a token", 0).unwrap();
        assert!(parse_numeric_result_prefix_tokens(&numeric).is_some());
        assert!(parse_statement_label_split_tokens(&numeric).is_none());

        let compact_ascii_numeric = lex_line("1-9 | Draw a card", 0).unwrap();
        assert!(parse_numeric_result_prefix_tokens(&compact_ascii_numeric).is_some());
        assert!(parse_statement_label_split_tokens(&compact_ascii_numeric).is_none());

        let exact_numeric_with_label = lex_line("1 | Trapped! — You lose 3 life", 0).unwrap();
        assert!(parse_numeric_result_prefix_tokens(&exact_numeric_with_label).is_some());
        assert!(parse_statement_label_split_tokens(&exact_numeric_with_label).is_none());

        let labeled = lex_line("Landfall — Draw a card", 0).unwrap();
        let stripped = parse_statement_label_strip_tokens(&labeled);
        assert_eq!(stripped.stripped_labels, 1);
        assert_eq!(stripped.body_tokens[0].parser_text(), "draw");

        let keyword = lex_line("Kicker — Draw a card", 0).unwrap();
        let preserved = parse_statement_label_strip_tokens(&keyword);
        assert_eq!(preserved.stripped_labels, 0);
        assert_eq!(preserved.body_tokens, keyword.as_slice());
    }
}

use winnow::combinator::{alt, repeat};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::rest;

use crate::mana::ManaCost;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenKind};

use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlashbackKeywordLineSpec<'a> {
    pub(crate) cost: ManaCost,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlashbackCostClause<'a> {
    Missing,
    UnsupportedCostsClause(&'a [OwnedLexToken]),
    Cost(&'a [OwnedLexToken]),
}

fn parse_flashback_keyword_line_spec<'a>(
    input: &mut LexStream<'a>,
) -> Result<FlashbackKeywordLineSpec<'a>, ErrMode<ContextError>> {
    primitives::kw("flashback").parse_next(input)?;
    let cost = leaf::parse_leaf_mana_cost_prefix_lexed
        .parse_next(input)?
        .cost;
    let tail_tokens = rest.parse_next(input)?;

    Ok(FlashbackKeywordLineSpec { cost, tail_tokens })
}

pub(crate) fn parse_flashback_keyword_line_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<FlashbackKeywordLineSpec<'_>> {
    primitives::parse_all(
        tokens,
        parse_flashback_keyword_line_spec,
        "flashback-keyword-line",
    )
    .ok()
}

pub(crate) fn parse_flashback_cost_clause_tokens(
    tokens: &[OwnedLexToken],
) -> Option<FlashbackCostClause<'_>> {
    primitives::parse_all(tokens, parse_flashback_cost_clause, "flashback-cost-clause").ok()
}

fn parse_flashback_cost_clause<'a>(
    input: &mut LexStream<'a>,
) -> Result<FlashbackCostClause<'a>, ErrMode<ContextError>> {
    primitives::kw("flashback").parse_next(input)?;
    repeat::<_, _, (), _, _>(
        0..,
        alt((
            primitives::token_kind(TokenKind::Dash),
            primitives::token_kind(TokenKind::EmDash),
        ))
        .void(),
    )
    .parse_next(input)?;
    let cost_tokens: &'a [OwnedLexToken] = rest.parse_next(input)?;
    if cost_tokens.is_empty() {
        return Ok(FlashbackCostClause::Missing);
    }
    if primitives::parse_prefix(cost_tokens, primitives::kw("costs")).is_some() {
        return Ok(FlashbackCostClause::UnsupportedCostsClause(cost_tokens));
    }
    Ok(FlashbackCostClause::Cost(cost_tokens))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::{TokenWordView, lex_line};

    #[test]
    fn parses_grouped_flashback_cost_and_tail() {
        let tokens = lex_line("Flashback {2}{B}, only as a sorcery.", 0).unwrap();
        let spec = parse_flashback_keyword_line_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.cost.to_oracle(), "{2}{B}");
        assert_eq!(
            TokenWordView::new(spec.tail_tokens).word_refs(),
            ["only", "as", "a", "sorcery"]
        );
    }

    #[test]
    fn preserves_hybrid_pip_grouping() {
        let tokens = lex_line("Flashback {2/W}{U}", 0).unwrap();
        let spec = parse_flashback_keyword_line_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.cost.to_oracle(), "{2/W}{U}");
        assert_eq!(spec.cost.pips()[0].len(), 2);
        assert!(spec.tail_tokens.is_empty());
    }

    #[test]
    fn rejects_missing_cost() {
        let tokens = lex_line("Flashback", 0).unwrap();
        assert!(parse_flashback_keyword_line_spec_lexed(&tokens).is_none());
        assert_eq!(
            parse_flashback_cost_clause_tokens(&tokens),
            Some(FlashbackCostClause::Missing)
        );
    }

    #[test]
    fn preserves_non_mana_flashback_cost_tokens_for_cost_lowering() {
        let tokens = lex_line("Flashback—Pay 3 life", 0).unwrap();
        let Some(FlashbackCostClause::Cost(cost_tokens)) =
            parse_flashback_cost_clause_tokens(&tokens)
        else {
            panic!("expected flashback cost tokens");
        };
        assert_eq!(
            TokenWordView::new(cost_tokens).word_refs(),
            ["pay", "3", "life"]
        );
    }
}

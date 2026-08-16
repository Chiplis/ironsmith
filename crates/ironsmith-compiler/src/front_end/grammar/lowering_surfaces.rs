use crate::mana::ManaSymbol;
use winnow::combinator::{alt, opt, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::lexer::{LexStream, OwnedLexToken};
use super::{leaf, primitives};

#[path = "lowering_surfaces/statement_replacements.rs"]
mod statement_replacements;

pub(crate) use statement_replacements::parse_statement_replacement_surface_tokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CreatureTypeChoiceBuff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ThisSpellCostSurface {
    pub(crate) reduction_cap: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PriorCreatedTokenReference;

pub(crate) fn parse_prior_created_token_reference_words(
    words: &[&str],
) -> Option<PriorCreatedTokenReference> {
    primitives::parse_full_word_slice(
        words,
        alt((
            primitives::word_slice_exact("those").value(PriorCreatedTokenReference),
            (
                primitives::word_slice_exact("of"),
                primitives::word_slice_exact("those"),
            )
                .value(PriorCreatedTokenReference),
        )),
    )
}

pub(crate) fn parse_creature_type_choice_buff_tokens(
    tokens: &[OwnedLexToken],
) -> Option<CreatureTypeChoiceBuff> {
    primitives::parse_all(
        tokens,
        parse_creature_type_choice_buff_lexed,
        "creature-type choice buff",
    )
    .ok()
}

fn parse_creature_type_choice_buff_lexed(
    input: &mut LexStream<'_>,
) -> WResult<CreatureTypeChoiceBuff> {
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        primitives::phrase(&["creature", "type", "of", "your", "choice"]),
    )
    .parse_next(input)?;
    repeat_till::<_, _, (), _, _, _, _>(
        0..,
        any.void(),
        alt((primitives::kw("get"), primitives::kw("gets"))),
    )
    .parse_next(input)?;
    consume_tail(input)?;
    Ok(CreatureTypeChoiceBuff)
}

fn parse_generic_mana_cap(input: &mut LexStream<'_>) -> WResult<i32> {
    let symbols = leaf::parse_leaf_mana_group_token.parse_next(input)?;
    if symbols.len() == 1
        && let Some(ManaSymbol::Generic(amount)) = symbols.first().copied()
    {
        return Ok(i32::from(amount));
    }
    Err(primitives::backtrack_err(
        "this-spell cost surface",
        "one generic mana symbol after 'by more than'",
    ))
}

pub(crate) fn parse_this_spell_cost_surface_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ThisSpellCostSurface> {
    primitives::parse_all(
        tokens,
        parse_this_spell_cost_surface_lexed,
        "this-spell cost surface",
    )
    .ok()
}

fn parse_this_spell_cost_surface_lexed(input: &mut LexStream<'_>) -> WResult<ThisSpellCostSurface> {
    alt((
        primitives::phrase(&["this", "spell", "costs"]),
        primitives::phrase(&["this", "spell", "cost"]),
    ))
    .parse_next(input)?;
    let reduction_cap = opt((
        repeat_till::<_, _, (), _, _, _, _>(
            0..,
            any.void(),
            primitives::phrase(&["by", "more", "than"]),
        ),
        parse_generic_mana_cap,
    )
        .map(|(_, cap)| cap))
    .parse_next(input)?;
    consume_tail(input)?;
    Ok(ThisSpellCostSurface { reduction_cap })
}

fn consume_tail(input: &mut LexStream<'_>) -> WResult<()> {
    repeat::<_, _, (), _, _>(0.., any.void()).parse_next(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_this_spell_cost_and_cap() {
        let tokens = lex_line(
            "This spell costs {1} less to cast, but not by more than {3}.",
            0,
        )
        .unwrap();
        let parsed = parse_this_spell_cost_surface_tokens(&tokens).unwrap();
        assert_eq!(parsed.reduction_cap, Some(3));
    }

    #[test]
    fn recognizes_creature_type_choice_buff() {
        let tokens = lex_line(
            "Creatures of the creature type of your choice get +1/+1 until end of turn.",
            0,
        )
        .unwrap();
        assert!(parse_creature_type_choice_buff_tokens(&tokens).is_some());
    }

    #[test]
    fn recognizes_clash_replacement_surface() {
        let tokens = lex_line(
            "Clash with an opponent. If you win, put it on top of its owner's library instead.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_statement_replacement_surface_tokens(&tokens),
            Some(crate::model::facts::StatementReplacementSurfaceKind::ClashWinTopOfLibrary)
        );
    }

    #[test]
    fn parses_prior_created_token_reference_as_typed_fact() {
        assert_eq!(
            parse_prior_created_token_reference_words(&["of", "those"]),
            Some(PriorCreatedTokenReference)
        );
        assert_eq!(
            parse_prior_created_token_reference_words(&["those", "tokens"]),
            None
        );
    }
}

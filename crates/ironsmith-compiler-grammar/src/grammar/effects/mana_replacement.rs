use winnow::combinator::opt;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use crate::lexer::{LexStream, OwnedLexToken};
use crate::mana::ManaSymbol;

use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ManaReplacementClauseSpec {
    pub replacement_mana: ManaSymbol,
}

fn parse_replacement_mana_symbol<'a>(
    input: &mut LexStream<'a>,
) -> Result<ManaSymbol, ErrMode<ContextError>> {
    let pip = leaf::parse_leaf_surface_mana_pip_lexed
        .parse_next(input)?
        .into_pip();
    let [symbol] = pip.as_slice() else {
        return Err(primitives::backtrack_err(
            "mana replacement symbol",
            "one colored or colorless mana symbol",
        ));
    };
    if matches!(
        symbol,
        ManaSymbol::White
            | ManaSymbol::Blue
            | ManaSymbol::Black
            | ManaSymbol::Red
            | ManaSymbol::Green
            | ManaSymbol::Colorless
    ) {
        Ok(*symbol)
    } else {
        Err(primitives::backtrack_err(
            "mana replacement symbol",
            "one colored or colorless mana symbol",
        ))
    }
}

fn parse_mana_replacement_clause<'a>(
    input: &mut LexStream<'a>,
) -> Result<ManaReplacementClauseSpec, ErrMode<ContextError>> {
    primitives::phrase(&["until", "end", "of", "turn"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&[
        "if", "you", "tap", "a", "land", "you", "control", "for", "mana",
    ])
    .parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "produces"]).parse_next(input)?;
    let replacement_mana = parse_replacement_mana_symbol(input)?;
    primitives::phrase(&["instead", "of", "any", "other", "type"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;

    Ok(ManaReplacementClauseSpec { replacement_mana })
}

/// "If a land is tapped for two or more mana, it produces {C} instead of any
/// other type and amount." (Damping Sphere)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TappedForAmountManaReplacementSpec<'a> {
    pub source_tokens: &'a [OwnedLexToken],
    pub minimum_amount: u32,
    pub replacement_mana: ManaSymbol,
}

fn parse_tapped_for_amount_mana_replacement<'a>(
    input: &mut LexStream<'a>,
) -> Result<TappedForAmountManaReplacementSpec<'a>, ErrMode<ContextError>> {
    primitives::kw("if").parse_next(input)?;
    opt(winnow::combinator::alt((
        primitives::kw("a"),
        primitives::kw("an"),
    )))
    .parse_next(input)?;
    let source_tokens: &'a [OwnedLexToken] = winnow::combinator::repeat_till::<_, _, (), _, _, _, _>(
        1..,
        winnow::token::any.void(),
        winnow::combinator::peek(primitives::phrase(&["is", "tapped", "for"])),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    primitives::phrase(&["is", "tapped", "for"]).parse_next(input)?;
    let minimum_amount = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["or", "more", "mana"]).parse_next(input)?;
    opt(primitives::comma()).parse_next(input)?;
    primitives::phrase(&["it", "produces"]).parse_next(input)?;
    let replacement_mana = parse_replacement_mana_symbol(input)?;
    primitives::phrase(&["instead", "of", "any", "other", "type", "and", "amount"])
        .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(TappedForAmountManaReplacementSpec {
        source_tokens: crate::lexer::trim_lexed_commas(source_tokens),
        minimum_amount,
        replacement_mana,
    })
}

pub fn parse_tapped_for_amount_mana_replacement_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TappedForAmountManaReplacementSpec<'_>> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_tapped_for_amount_mana_replacement,
        "tapped-for-amount mana replacement",
    )
}

pub fn parse_mana_replacement_clause_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaReplacementClauseSpec> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_mana_replacement_clause,
        "mana-replacement-clause",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_typed_replacement_symbol() {
        let tokens = lex_line(
            "Until end of turn, if you tap a land you control for mana, it produces {U} instead of any other type.",
            0,
        )
        .unwrap();
        let spec = parse_mana_replacement_clause_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.replacement_mana, ManaSymbol::Blue);
    }

    #[test]
    fn accepts_each_colored_and_colorless_symbol() {
        for (raw, expected) in [
            ("{W}", ManaSymbol::White),
            ("{U}", ManaSymbol::Blue),
            ("{B}", ManaSymbol::Black),
            ("{R}", ManaSymbol::Red),
            ("{G}", ManaSymbol::Green),
            ("{C}", ManaSymbol::Colorless),
        ] {
            let line = format!(
                "Until end of turn, if you tap a land you control for mana, it produces {raw} instead of any other type."
            );
            let tokens = lex_line(&line, 0).unwrap();
            assert_eq!(
                parse_mana_replacement_clause_spec_lexed(&tokens)
                    .unwrap()
                    .replacement_mana,
                expected
            );
        }
    }

    #[test]
    fn rejects_generic_and_hybrid_replacement_pips() {
        for raw in ["{2}", "{W/U}"] {
            let line = format!(
                "Until end of turn, if you tap a land you control for mana, it produces {raw} instead of any other type."
            );
            let tokens = lex_line(&line, 0).unwrap();
            assert!(parse_mana_replacement_clause_spec_lexed(&tokens).is_none());
        }
    }
}

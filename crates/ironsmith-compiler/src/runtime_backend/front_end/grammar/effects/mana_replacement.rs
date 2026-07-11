use winnow::combinator::opt;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use crate::mana::ManaSymbol;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken};

use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ManaReplacementClauseSpec {
    pub(crate) replacement_mana: ManaSymbol,
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

pub(crate) fn parse_mana_replacement_clause_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ManaReplacementClauseSpec> {
    primitives::parse_all(
        tokens,
        parse_mana_replacement_clause,
        "mana-replacement-clause",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

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

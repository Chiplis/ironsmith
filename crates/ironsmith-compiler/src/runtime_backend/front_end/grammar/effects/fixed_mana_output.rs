use winnow::combinator::{alt, opt, repeat};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;

use crate::mana::ManaSymbol;
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken};

use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixedManaOutputClauseSpec {
    pub(crate) mana: Vec<ManaSymbol>,
}

fn commas<'a>(input: &mut LexStream<'a>) -> Result<(), ErrMode<ContextError>> {
    repeat::<_, _, (), _, _>(0.., primitives::comma().void()).parse_next(input)
}

fn parse_fixed_mana_output_clause<'a>(
    input: &mut LexStream<'a>,
) -> Result<FixedManaOutputClauseSpec, ErrMode<ContextError>> {
    commas.parse_next(input)?;
    opt((primitives::kw("you"), commas).void()).parse_next(input)?;
    alt((primitives::kw("add"), primitives::kw("adds")))
        .void()
        .parse_next(input)?;
    let mana = leaf::parse_leaf_fixed_mana_output_lexed.parse_next(input)?;

    Ok(FixedManaOutputClauseSpec { mana })
}

pub(crate) fn parse_fixed_mana_output_clause_spec_lexed(
    tokens: &[OwnedLexToken],
) -> Option<FixedManaOutputClauseSpec> {
    primitives::parse_all(
        tokens,
        parse_fixed_mana_output_clause,
        "fixed-mana-output-clause",
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::lexer::lex_line;

    #[test]
    fn parses_fixed_add_output_and_flattens_single_symbol_groups() {
        let tokens = lex_line("Add {W}{U}, {B}.", 0).unwrap();
        let spec = parse_fixed_mana_output_clause_spec_lexed(&tokens).unwrap();

        assert_eq!(
            spec.mana,
            vec![ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black]
        );
    }

    #[test]
    fn accepts_optional_you_and_punctuation() {
        let tokens = lex_line("You, add {G}.", 0).unwrap();
        let spec = parse_fixed_mana_output_clause_spec_lexed(&tokens).unwrap();

        assert_eq!(spec.mana, vec![ManaSymbol::Green]);
    }

    #[test]
    fn fixed_output_rejects_hybrid_and_legacy_bare_symbols() {
        for raw in ["Add {W/U}.", "Add w."] {
            let tokens = lex_line(raw, 0).unwrap();
            assert!(parse_fixed_mana_output_clause_spec_lexed(&tokens).is_none());
        }
    }
}

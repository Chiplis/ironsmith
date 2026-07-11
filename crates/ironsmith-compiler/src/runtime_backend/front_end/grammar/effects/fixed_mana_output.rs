use winnow::combinator::{alt, opt, peek, repeat, repeat_till};
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::token::any;

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

fn parse_token_tap_add_single_mana_symbol_word_slice(
    input: &mut primitives::WordSliceInput<'_>,
) -> Result<ManaSymbol, ErrMode<ContextError>> {
    let before_add = repeat_till(
        0..,
        any.void(),
        peek(primitives::word_slice_exact("add")).void(),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    primitives::word_slice_exact("add")
        .void()
        .parse_next(input)?;
    if !before_add.iter().any(|word| *word == "t") {
        return Err(primitives::backtrack_err(
            "token tap-add mana",
            "tap symbol before add",
        ));
    }

    let Some((symbol_word, rest)) = input.split_first() else {
        return Err(primitives::backtrack_err(
            "token tap-add mana",
            "single mana symbol after add",
        ));
    };
    let symbol = leaf::parse_leaf_bare_mana_symbol_complete(symbol_word).map_err(|_| {
        primitives::backtrack_err("token tap-add mana", "single mana symbol after add")
    })?;
    if matches!(symbol, ManaSymbol::Generic(_) | ManaSymbol::X) {
        return Err(primitives::backtrack_err(
            "token tap-add mana",
            "non-generic, non-X mana symbol",
        ));
    }
    *input = rest;
    Ok(symbol)
}

pub(crate) fn parse_token_tap_add_single_mana_symbol_words(words: &[&str]) -> Option<ManaSymbol> {
    let mut input: primitives::WordSliceInput<'_> = words;
    parse_token_tap_add_single_mana_symbol_word_slice
        .parse_next(&mut input)
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

    #[test]
    fn token_definition_requires_t_before_first_add() {
        assert_eq!(
            parse_token_tap_add_single_mana_symbol_words(&[
                "plant", "with", "t", "then", "add", "g", "mana"
            ]),
            Some(ManaSymbol::Green)
        );
        assert_eq!(
            parse_token_tap_add_single_mana_symbol_words(&["t", "add", "s"]),
            Some(ManaSymbol::Snow)
        );
        assert!(parse_token_tap_add_single_mana_symbol_words(&["add", "g", "t"]).is_none());
    }

    #[test]
    fn token_definition_rejects_generic_x_and_hybrid() {
        for symbol in ["2", "x", "w/u"] {
            assert!(
                parse_token_tap_add_single_mana_symbol_words(&["token", "t", "add", symbol])
                    .is_none()
            );
        }
    }
}

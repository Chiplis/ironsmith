use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::mana::ManaSymbol;

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PerAttackerCantTaxFact {
    pub(crate) amount: u32,
}

pub(crate) fn parse_per_attacker_cant_tax_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PerAttackerCantTaxFact> {
    primitives::parse_all(
        tokens,
        parse_per_attacker_cant_tax_lexed,
        "per-attacker cant tax",
    )
    .ok()
}

fn parse_per_attacker_cant_tax_lexed(input: &mut LexStream<'_>) -> WResult<PerAttackerCantTaxFact> {
    primitives::kw("creatures").parse_next(input)?;
    alt((
        primitives::kw("can't"),
        primitives::kw("cant"),
        primitives::kw("cannot"),
    ))
    .parse_next(input)?;
    primitives::phrase(&["attack", "you", "unless", "their", "controller", "pays"])
        .parse_next(input)?;
    let amount = parse_generic_mana_amount.parse_next(input)?;
    primitives::phrase(&["for", "each", "creature", "they", "control"]).parse_next(input)?;
    alt((primitives::kw("that's"), primitives::kw("thats"))).parse_next(input)?;
    primitives::phrase(&["attacking", "you"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PerAttackerCantTaxFact { amount })
}

fn parse_generic_mana_amount(input: &mut LexStream<'_>) -> WResult<u32> {
    let pip = leaf::parse_leaf_surface_mana_pip_lexed.parse_next(input)?;
    let symbol = match pip {
        leaf::LeafManaPipToken::ManaGroup(symbols) => match symbols.as_slice() {
            [symbol] => *symbol,
            _ => {
                return Err(primitives::backtrack_err(
                    "per-attacker tax",
                    "one generic mana symbol",
                ));
            }
        },
        leaf::LeafManaPipToken::LegacyBare(symbol) => symbol,
    };
    match symbol {
        ManaSymbol::Generic(amount) => Ok(u32::from(amount)),
        _ => Err(primitives::backtrack_err(
            "per-attacker tax",
            "generic mana amount",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn parses_typed_per_attacker_tax() {
        for (raw, amount) in [
            (
                "Creatures can't attack you unless their controller pays {2} for each creature they control that's attacking you.",
                2,
            ),
            (
                "Creatures cannot attack you unless their controller pays 1 for each creature they control thats attacking you",
                1,
            ),
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            assert_eq!(
                parse_per_attacker_cant_tax_tokens(&tokens),
                Some(PerAttackerCantTaxFact { amount })
            );
        }
    }

    #[test]
    fn rejects_non_generic_or_incomplete_taxes() {
        for raw in [
            "Creatures can't attack you unless their controller pays {W} for each creature they control that's attacking you.",
            "Creatures can't attack you unless their controller pays {2}.",
        ] {
            let tokens = lex_line(raw, 0).unwrap();
            assert_eq!(parse_per_attacker_cant_tax_tokens(&tokens), None);
        }
    }
}

use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::mana::ManaSymbol;

use super::super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerAttackerCantTaxFact {
    pub amount: u32,
    /// Oracle's planeswalker-inclusive wording. It selects both the restriction
    /// surface and the shorter "for each of those creatures" payment phrasing.
    pub covers_planeswalkers: bool,
}

pub fn parse_per_attacker_cant_tax_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PerAttackerCantTaxFact> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_per_attacker_cant_tax_lexed,
        "per-attacker cant tax",
    )
}

fn parse_per_attacker_cant_tax_lexed(input: &mut LexStream<'_>) -> WResult<PerAttackerCantTaxFact> {
    primitives::kw("creatures").parse_next(input)?;
    alt((
        primitives::kw("can't"),
        primitives::kw("cant"),
        primitives::kw("cannot"),
    ))
    .parse_next(input)?;
    let covers_planeswalkers = alt((
        primitives::phrase(&[
            "attack",
            "you",
            "or",
            "planeswalkers",
            "you",
            "control",
            "unless",
            "their",
            "controller",
            "pays",
        ])
        .map(|_| true),
        primitives::phrase(&["attack", "you", "unless", "their", "controller", "pays"])
            .map(|_| false),
    ))
    .parse_next(input)?;
    let amount = parse_generic_mana_amount.parse_next(input)?;
    alt((
        primitives::phrase(&["for", "each", "of", "those", "creatures"]).void(),
        (
            primitives::phrase(&["for", "each", "creature", "they", "control"]),
            alt((primitives::kw("that's"), primitives::kw("thats"))),
            primitives::phrase(&["attacking", "you"]),
        )
            .void(),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(PerAttackerCantTaxFact {
        amount,
        covers_planeswalkers,
    })
}

/// The mana part of a general per-attacker tax.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackTaxManaAmount {
    Generic(u32),
    /// "{X}": the X announced for the effect that creates the tax (War Tax).
    X,
}

/// "creatures can't attack [you | you or planeswalkers you control |
/// planeswalkers you control] unless their controller pays <mana and/or N
/// life> for each ...". The per-attacker wording must agree with the scope:
/// "for each of those creatures" (you), "for each creature they control
/// that's attacking a planeswalker you control" (planeswalkers only), or "for
/// each attacking creature they control" (every attack). CR 508.1g-h.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneralAttackTaxFact {
    pub defenders: ironsmith_core::value_model::AttackTaxDefenders,
    pub mana: Option<AttackTaxManaAmount>,
    pub life: u32,
}

pub fn parse_general_attack_tax_tokens(tokens: &[OwnedLexToken]) -> Option<GeneralAttackTaxFact> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_general_attack_tax_lexed,
        "general per-attacker attack tax",
    )
}

fn parse_general_attack_tax_lexed(input: &mut LexStream<'_>) -> WResult<GeneralAttackTaxFact> {
    use ironsmith_core::value_model::AttackTaxDefenders;

    primitives::kw("creatures").parse_next(input)?;
    alt((
        primitives::kw("can't"),
        primitives::kw("cant"),
        primitives::kw("cannot"),
    ))
    .parse_next(input)?;
    primitives::kw("attack").parse_next(input)?;
    let defenders = alt((
        primitives::phrase(&["you", "or", "planeswalkers", "you", "control"])
            .map(|_| AttackTaxDefenders::ControllerOrPlaneswalkers),
        primitives::phrase(&["planeswalkers", "you", "control"])
            .map(|_| AttackTaxDefenders::ControllerPlaneswalkers),
        primitives::kw("you").map(|_| AttackTaxDefenders::Controller),
        winnow::combinator::empty.value(AttackTaxDefenders::Anyone),
    ))
    .parse_next(input)?;
    primitives::phrase(&["unless", "their", "controller", "pays"]).parse_next(input)?;
    let (mana, life) = parse_attack_tax_payment.parse_next(input)?;
    primitives::phrase(&["for", "each"]).parse_next(input)?;
    match defenders {
        AttackTaxDefenders::Controller | AttackTaxDefenders::ControllerOrPlaneswalkers => {
            primitives::phrase(&["of", "those", "creatures"]).parse_next(input)?;
        }
        AttackTaxDefenders::ControllerPlaneswalkers => {
            (
                primitives::phrase(&["creature", "they", "control"]),
                alt((primitives::kw("that's"), primitives::kw("thats"))),
                primitives::phrase(&["attacking", "a", "planeswalker", "you", "control"]),
            )
                .parse_next(input)?;
        }
        AttackTaxDefenders::Anyone => {
            primitives::phrase(&["attacking", "creature", "they", "control"]).parse_next(input)?;
        }
    }
    primitives::sentence_end().parse_next(input)?;
    Ok(GeneralAttackTaxFact {
        defenders,
        mana,
        life,
    })
}

impl GeneralAttackTaxFact {
    /// The resolving-effect rule ("this turn, ..." / "until your next turn,
    /// ..."). An {X} amount is the effect's announced X, fixed on resolution.
    pub fn into_rule(self) -> ironsmith_core::value_model::AttackTaxRule {
        ironsmith_core::value_model::AttackTaxRule {
            attackers: ironsmith_core::ObjectFilter::creature(),
            defenders: self.defenders,
            mana_per_attacker: match self.mana {
                Some(AttackTaxManaAmount::Generic(amount)) => {
                    ironsmith_core::Value::Fixed(i32::try_from(amount).unwrap_or(i32::MAX))
                }
                Some(AttackTaxManaAmount::X) => ironsmith_core::Value::X,
                None => ironsmith_core::Value::Fixed(0),
            },
            life_per_attacker: self.life,
        }
    }
}

/// "{1}", "{X}", "2 life", or "{1} and 2 life".
fn parse_attack_tax_payment(
    input: &mut LexStream<'_>,
) -> WResult<(Option<AttackTaxManaAmount>, u32)> {
    let life_only = (primitives::number_token, primitives::kw("life"));
    alt((
        life_only.map(|(life, _)| (None, life)),
        (
            parse_tax_mana_amount,
            winnow::combinator::opt((
                primitives::kw("and"),
                primitives::number_token,
                primitives::kw("life"),
            )),
        )
            .map(|(mana, life)| (Some(mana), life.map_or(0, |(_, life, _)| life))),
    ))
    .parse_next(input)
}

fn parse_tax_mana_amount(input: &mut LexStream<'_>) -> WResult<AttackTaxManaAmount> {
    let pip = leaf::parse_leaf_surface_mana_pip_lexed.parse_next(input)?;
    let symbol = match pip {
        leaf::LeafManaPipToken::ManaGroup(symbols) => match symbols.as_slice() {
            [symbol] => *symbol,
            _ => {
                return Err(primitives::backtrack_err(
                    "per-attacker tax",
                    "one generic or X mana symbol",
                ));
            }
        },
        leaf::LeafManaPipToken::LegacyBare(symbol) => symbol,
    };
    match symbol {
        ManaSymbol::Generic(amount) => Ok(AttackTaxManaAmount::Generic(u32::from(amount))),
        ManaSymbol::X => Ok(AttackTaxManaAmount::X),
        _ => Err(primitives::backtrack_err(
            "per-attacker tax",
            "generic or X mana amount",
        )),
    }
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
#[path = "attack_tax_inline_tests.rs"]
mod tests;

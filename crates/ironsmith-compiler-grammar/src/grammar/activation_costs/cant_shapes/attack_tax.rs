use winnow::combinator::{alt, empty, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::effect::Restriction;
use crate::lexer::{LexStream, OwnedLexToken};
use crate::mana::ManaSymbol;
use crate::token_primitives::top_level_token_indices;
use ironsmith_core::value_model::AttackTaxDefenders;

use super::super::super::{leaf, primitives};

/// The payer, defender and per-creature wording belong to one clause. Keep the
/// payment and optional X definition intact for static abilities with full costs.
pub(crate) struct AttackTaxClause<'a> {
    pub attackers: &'a [OwnedLexToken],
    pub defenders: AttackTaxDefenders,
    pub payment: &'a [OwnedLexToken],
    pub binding: Option<&'a [OwnedLexToken]>,
    pub singular_payer: bool,
    singular_planeswalker: bool,
}

pub(crate) fn parse_resolving_attack_tax(tokens: &[OwnedLexToken]) -> Option<Restriction> {
    crate::word_claims::speculative_keep_if(
        || parse_attack_tax_clause(tokens)?.resolving_restriction(),
        Option::is_some,
    )
}

pub(crate) fn parse_attack_tax_clause(tokens: &[OwnedLexToken]) -> Option<AttackTaxClause<'_>> {
    use AttackTaxDefenders::*;
    let tokens = match tokens {
        [body @ .., end] if end.is_period() => body,
        _ => tokens,
    };
    let cant = top_level_token_indices(tokens, |token| {
        matches!(token.parser_text(), "can't" | "cant" | "cannot")
    })
    .next()?;
    let (attackers, head) = tokens.split_at(cant);
    if attackers.is_empty() {
        return None;
    }
    let ((_, _, (defenders, singular_planeswalker), _, singular_payer, _), body) =
        primitives::parse_prefix(
            head,
            (
                alt((
                    primitives::kw("can't"),
                    primitives::kw("cant"),
                    primitives::kw("cannot"),
                )),
                primitives::kw("attack"),
                alt((
                    primitives::phrase(&["you", "or", "planeswalkers", "you", "control"])
                        .value((ControllerOrPlaneswalkers, false)),
                    primitives::phrase(&["you", "or", "a", "planeswalker", "you", "control"])
                        .value((ControllerOrPlaneswalkers, true)),
                    primitives::phrase(&["planeswalkers", "you", "control"])
                        .value((ControllerPlaneswalkers, false)),
                    primitives::kw("you").value((Controller, false)),
                    empty.value((Anyone, false)),
                )),
                primitives::kw("unless"),
                alt((
                    primitives::kw("its").value(true),
                    primitives::kw("their").value(false),
                )),
                primitives::phrase(&["controller", "pays"]),
            ),
        )?;
    let (body, binding) =
        match top_level_token_indices(body, |token| token.parser_text() == "where").next() {
            Some(at) => (
                crate::lexer::trim_lexed_commas(&body[..at]),
                Some(&body[at..]),
            ),
            None => (body, None),
        };
    let per = top_level_token_indices(body, |token| token.parser_text() == "for")
        .filter(|&at| {
            body.get(at + 1)
                .is_some_and(|next| next.parser_text() == "each")
        })
        .last();
    let payment = if let Some(at) = per {
        let tail = &body[at + 2..];
        let words = tail
            .iter()
            .map(OwnedLexToken::parser_text)
            .collect::<Vec<_>>();
        let matches_scope = match words.as_slice() {
            ["of", "those", "creatures"] => {
                matches!(defenders, Controller | ControllerOrPlaneswalkers)
            }
            ["attacking", "creature", "they", "control"] => defenders == Anyone,
            [
                "creature",
                "they",
                "control",
                "that's" | "thats",
                "attacking",
                target @ ..,
            ] => match target {
                ["you"] => defenders == Controller,
                ["a", "planeswalker", "you", "control"] => defenders == ControllerPlaneswalkers,
                _ => false,
            },
            _ => false,
        };
        if !matches_scope {
            return None;
        }
        crate::word_claims::claim_tokens(&body[at..]);
        &body[..at]
    } else if singular_payer {
        body
    } else {
        return None;
    };
    Some(AttackTaxClause {
        attackers,
        defenders,
        payment,
        binding,
        singular_payer,
        singular_planeswalker,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerAttackerCantTaxFact {
    pub amount: u32,
    /// Oracle's planeswalker-inclusive wording. It selects both the restriction
    /// surface and the shorter "for each of those creatures" payment phrasing.
    pub covers_planeswalkers: bool,
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
    pub defenders: AttackTaxDefenders,
    pub mana: Option<AttackTaxManaAmount>,
    pub life: u32,
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

pub fn parse_per_attacker_cant_tax_tokens(
    tokens: &[OwnedLexToken],
) -> Option<PerAttackerCantTaxFact> {
    crate::word_claims::speculative_keep_if(
        || {
            let clause = parse_attack_tax_clause(tokens)?;
            clause.fixed_controller_tax(clause.general_tax()?)
        },
        Option::is_some,
    )
}

pub fn parse_general_attack_tax_tokens(tokens: &[OwnedLexToken]) -> Option<GeneralAttackTaxFact> {
    crate::word_claims::speculative_keep_if(
        || parse_attack_tax_clause(tokens)?.general_tax(),
        Option::is_some,
    )
}

impl AttackTaxClause<'_> {
    pub(crate) fn general_tax(&self) -> Option<GeneralAttackTaxFact> {
        if self.binding.is_some()
            || self.singular_payer
            || !matches!(self.attackers, [noun] if noun.is_word("creatures"))
        {
            return None;
        }
        let (mana, life) = primitives::probe_all(
            self.payment,
            parse_attack_tax_payment,
            "per-attacker payment",
        )?;
        Some(GeneralAttackTaxFact {
            defenders: self.defenders,
            mana,
            life,
        })
    }

    // Keep the fixed-tax artifact and its authored player/planeswalker wording.
    pub(crate) fn fixed_controller_tax(
        &self,
        tax: GeneralAttackTaxFact,
    ) -> Option<PerAttackerCantTaxFact> {
        use AttackTaxDefenders::*;
        if self.singular_planeswalker
            || !matches!(self.defenders, Controller | ControllerOrPlaneswalkers)
            || self.payment.len() != 1
        {
            return None;
        }
        let AttackTaxManaAmount::Generic(amount) = tax.mana? else {
            return None;
        };
        Some(PerAttackerCantTaxFact {
            amount,
            covers_planeswalkers: self.defenders == ControllerOrPlaneswalkers,
        })
    }

    fn resolving_restriction(&self) -> Option<Restriction> {
        let tax = self.general_tax()?;
        Some(if let Some(fixed) = self.fixed_controller_tax(tax) {
            Restriction::attack_you_unless_controller_pays_per_attacker(
                fixed.amount,
                fixed.covers_planeswalkers,
            )
        } else {
            Restriction::attack_tax(tax.into_rule())
        })
    }
}

fn parse_attack_tax_payment(
    input: &mut LexStream<'_>,
) -> WResult<(Option<AttackTaxManaAmount>, u32)> {
    let mana =
        leaf::parse_leaf_surface_mana_pip_lexed.verify_map(|pip| match pip.into_pip().as_slice() {
            [ManaSymbol::Generic(n)] => Some(AttackTaxManaAmount::Generic(u32::from(*n))),
            [ManaSymbol::X] => Some(AttackTaxManaAmount::X),
            _ => None,
        });
    let life = || (primitives::number_token, primitives::kw("life")).map(|(n, _)| n);
    alt((
        life().map(|n| (None, n)),
        (mana, opt((primitives::kw("and"), life())))
            .map(|(pip, life)| (Some(pip), life.map_or(0, |(_, n)| n))),
    ))
    .parse_next(input)
}

#[cfg(test)]
#[path = "attack_tax_inline_tests.rs"]
mod tests;

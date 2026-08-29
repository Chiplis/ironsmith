use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, rest, take_till};

use crate::mana::ManaCost;
use crate::types::Subtype;

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind, trim_lexed_commas};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EquipQualifierSpec<'a> {
    pub tokens: &'a [OwnedLexToken],
    pub subtypes: Vec<Subtype>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EquipLineSpec<'a> {
    MissingCost,
    Mana {
        cost: ManaCost,
    },
    QualifiedCost {
        qualifier: EquipQualifierSpec<'a>,
        cost_tokens: &'a [OwnedLexToken],
        mana_prefix: ManaCost,
        exact_mana_cost: bool,
    },
    ActivationCost {
        cost_tokens: &'a [OwnedLexToken],
    },
}

pub fn parse_equip_line_spec_tokens(tokens: &[OwnedLexToken]) -> Option<EquipLineSpec<'_>> {
    primitives::parse_prefix(tokens, parse_equip_line_spec_lexed).map(|(spec, _)| spec)
}

fn parse_equip_line_spec_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EquipLineSpec<'a>> {
    primitives::kw("equip").parse_next(input)?;
    let body = take_till(0.., is_equip_suffix_boundary).parse_next(input)?;
    let body = trim_lexed_commas(body);
    if body.is_empty() {
        return Ok(EquipLineSpec::MissingCost);
    }
    primitives::parse_all(body, parse_equip_body_lexed, "equip-line-body")
        .map_err(|_| primitives::backtrack_err("equip line", "typed equip cost"))
}

fn parse_equip_body_lexed<'a>(input: &mut LexStream<'a>) -> WResult<EquipLineSpec<'a>> {
    alt((
        parse_exact_equip_mana_cost,
        parse_qualified_equip_cost,
        parse_general_equip_activation_cost,
    ))
    .parse_next(input)
}

fn parse_exact_equip_mana_cost<'a>(input: &mut LexStream<'a>) -> WResult<EquipLineSpec<'a>> {
    let prefix = leaf::parse_leaf_mana_cost_prefix_lexed.parse_next(input)?;
    eof.parse_next(input)?;
    Ok(EquipLineSpec::Mana { cost: prefix.cost })
}

fn parse_qualified_equip_cost<'a>(input: &mut LexStream<'a>) -> WResult<EquipLineSpec<'a>> {
    let qualifier_tokens = repeat_till(
        1..,
        any.void(),
        peek(leaf::parse_leaf_mana_cost_prefix_lexed),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let qualifier = primitives::parse_all(
        trim_lexed_commas(qualifier_tokens),
        parse_equip_qualifier_lexed,
        "equip-target-qualifier",
    )
    .map_err(|_| primitives::backtrack_err("equip qualifier", "one or more creature subtypes"))?;
    let ((mana_prefix, trailing), cost_tokens) = (leaf::parse_leaf_mana_cost_prefix_lexed, rest)
        .with_taken()
        .parse_next(input)?;
    Ok(EquipLineSpec::QualifiedCost {
        qualifier: EquipQualifierSpec {
            tokens: trim_lexed_commas(qualifier_tokens),
            subtypes: qualifier,
        },
        cost_tokens: trim_lexed_commas(cost_tokens),
        mana_prefix: mana_prefix.cost,
        exact_mana_cost: trailing.is_empty(),
    })
}

fn parse_general_equip_activation_cost<'a>(
    input: &mut LexStream<'a>,
) -> WResult<EquipLineSpec<'a>> {
    peek(alt((
        leaf::parse_leaf_mana_cost_prefix_lexed.void(),
        alt((
            primitives::kw("tap").void(),
            primitives::kw("t").void(),
            primitives::kw("pay").void(),
            primitives::kw("discard").void(),
            primitives::kw("sacrifice").void(),
            primitives::kw("exile").void(),
            primitives::kw("return").void(),
            primitives::kw("remove").void(),
            primitives::kw("behold").void(),
        )),
    )))
    .parse_next(input)?;
    let cost_tokens = rest.parse_next(input)?;
    Ok(EquipLineSpec::ActivationCost { cost_tokens })
}

fn parse_equip_qualifier_lexed<'a>(input: &mut LexStream<'a>) -> WResult<Vec<Subtype>> {
    let first = parse_equip_subtype_lexed.parse_next(input)?;
    let trailing: Vec<Subtype> = repeat(
        0..,
        (
            repeat::<_, _, (), _, _>(
                0..,
                alt((
                    primitives::comma().void(),
                    primitives::kw("or").void(),
                    primitives::kw("and").void(),
                    primitives::kw("and/or").void(),
                )),
            ),
            parse_equip_subtype_lexed,
        )
            .map(|(_, subtype)| subtype),
    )
    .parse_next(input)?;
    opt(primitives::kw("creature")).parse_next(input)?;
    eof.parse_next(input)?;

    let mut subtypes = Vec::with_capacity(trailing.len() + 1);
    subtypes.push(first);
    trailing.into_iter().for_each(|subtype| {
        crate::slice_primitives::push_unique(&mut subtypes, subtype);
    });
    Ok(subtypes)
}

fn parse_equip_subtype_lexed<'a>(input: &mut LexStream<'a>) -> WResult<Subtype> {
    let word = primitives::word_parser_text.parse_next(input)?;
    leaf::parse_leaf_subtype_flexible_complete(word)
        .map_err(|_| primitives::backtrack_err("equip subtype", "known subtype"))
}

fn is_equip_suffix_boundary(token: &OwnedLexToken) -> bool {
    matches!(token.kind, TokenKind::LParen | TokenKind::Period)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_plain_qualified_and_general_costs() {
        let plain = lex_line("Equip {2}{W}", 0).unwrap();
        assert!(matches!(
            parse_equip_line_spec_tokens(&plain),
            Some(EquipLineSpec::Mana { .. })
        ));

        let qualified = lex_line("Equip Shaman, Warlock, or Wizard {1}", 0).unwrap();
        let Some(EquipLineSpec::QualifiedCost {
            qualifier,
            cost_tokens,
            exact_mana_cost,
            ..
        }) = parse_equip_line_spec_tokens(&qualified)
        else {
            panic!("expected qualified equip cost");
        };
        assert_eq!(
            qualifier.subtypes,
            vec![Subtype::Shaman, Subtype::Warlock, Subtype::Wizard]
        );
        assert_eq!(render_token_slice(cost_tokens), "{1}");
        assert!(exact_mana_cost);

        let alternative = lex_line("Equip {2} or {B}", 0).unwrap();
        assert!(matches!(
            parse_equip_line_spec_tokens(&alternative),
            Some(EquipLineSpec::ActivationCost { .. })
        ));
    }

    #[test]
    fn stops_before_reminder_and_followup_sentences() {
        let reminder = lex_line(
            "Equip {1} ({1}: Attach to target creature you control. Equip only as a sorcery.)",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse_equip_line_spec_tokens(&reminder),
            Some(EquipLineSpec::Mana { .. })
        ));

        let followup = lex_line("Equip {0}. Activate only once each turn.", 0).unwrap();
        assert!(matches!(
            parse_equip_line_spec_tokens(&followup),
            Some(EquipLineSpec::Mana { .. })
        ));
    }
}

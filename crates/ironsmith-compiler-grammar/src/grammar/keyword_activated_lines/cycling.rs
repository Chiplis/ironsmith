use winnow::combinator::{alt, eof, opt, peek, repeat, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, literal, take_until};

use crate::color::ColorSet;
use crate::mana::ManaCost;
use crate::types::{CardType, Subtype, Supertype};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::activated_lines::parse_cycling_marker_word;
use super::super::activation_costs::{
    ActivationCostSegmentKind, parse_activation_cost_segment_kind_tokens,
};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CyclingKeywordCostKind {
    Mana(ManaCost),
    PayLife { amount: u32 },
    Activation { head: ActivationCostSegmentKind },
}

impl CyclingKeywordCostKind {
    pub fn mana_cost(&self) -> Option<&ManaCost> {
        match self {
            Self::Mana(cost) => Some(cost),
            Self::PayLife { .. } | Self::Activation { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclingKeywordCostGroup<'a> {
    pub keyword_tokens: &'a [OwnedLexToken],
    pub cost_tokens: &'a [OwnedLexToken],
    pub cost_kind: CyclingKeywordCostKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CyclingFilterSpec {
    pub supertypes: Vec<Supertype>,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub colors: Option<ColorSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CyclingSearchSpec {
    Draw,
    Search(CyclingFilterSpec),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CyclingSearchParseError {
    MissingKeyword,
    UnsupportedRoot(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum CyclingFilterAtom {
    Supertype(Supertype),
    CardType(CardType),
    Subtype(Subtype),
    Colors(ColorSet),
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CyclingKeywordShape {
    prefix_atoms: Vec<CyclingFilterAtom>,
    root: String,
}

pub fn parse_cycling_keyword_cost_groups_tokens(
    tokens: &[OwnedLexToken],
) -> Vec<CyclingKeywordCostGroup<'_>> {
    let mut input = LexStream::new(tokens);
    parse_cycling_keyword_cost_groups_lexed
        .parse_next(&mut input)
        .unwrap_or_default()
}

pub fn parse_cycling_search_spec_tokens(
    tokens: &[OwnedLexToken],
) -> Result<CyclingSearchSpec, CyclingSearchParseError> {
    let shape = primitives::parse_all(
        tokens,
        parse_cycling_keyword_shape_lexed,
        "cycling-keyword-shape",
    )
    .map_err(|_| CyclingSearchParseError::MissingKeyword)?;

    if shape.root.is_empty() {
        return Ok(CyclingSearchSpec::Draw);
    }

    let root_atom = classify_cycling_root_word(&shape.root);
    if root_atom == CyclingFilterAtom::Ignored {
        return Err(CyclingSearchParseError::UnsupportedRoot(shape.root));
    }
    let filter = shape
        .prefix_atoms
        .into_iter()
        .chain(std::iter::once(root_atom))
        .fold(CyclingFilterSpec::default(), add_cycling_filter_atom);
    Ok(CyclingSearchSpec::Search(filter))
}

fn parse_cycling_keyword_cost_groups_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<Vec<CyclingKeywordCostGroup<'a>>> {
    repeat::<_, _, (), _, _>(
        0..,
        alt((primitives::comma().void(), primitives::semicolon().void())),
    )
    .parse_next(input)?;
    let first = parse_cycling_keyword_cost_group_lexed.parse_next(input)?;
    let trailing: Vec<CyclingKeywordCostGroup<'a>> = repeat(
        0..,
        (
            alt((primitives::comma().void(), primitives::semicolon().void())),
            parse_cycling_keyword_cost_group_lexed,
        )
            .map(|(_, group)| group),
    )
    .parse_next(input)?;
    let mut groups = Vec::with_capacity(trailing.len() + 1);
    groups.push(first);
    groups.extend(trailing);
    Ok(groups)
}

fn parse_cycling_keyword_cost_group_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CyclingKeywordCostGroup<'a>> {
    let (((), _), keyword_tokens) = repeat_till(
        0..,
        any.verify(|token: &&OwnedLexToken| token.as_word().is_some())
            .void(),
        parse_cycling_marker_token,
    )
    .with_taken()
    .parse_next(input)?;
    opt(alt((
        primitives::token_kind(TokenKind::Dash),
        primitives::token_kind(TokenKind::EmDash),
    )))
    .parse_next(input)?;
    let (cost_kind, cost_tokens) = alt((
        parse_cycling_pay_life_cost_lexed,
        parse_cycling_mana_cost_lexed,
        parse_cycling_activation_cost_lexed,
    ))
    .parse_next(input)?;
    Ok(CyclingKeywordCostGroup {
        keyword_tokens,
        cost_tokens,
        cost_kind,
    })
}

fn parse_cycling_activation_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(CyclingKeywordCostKind, &'a [OwnedLexToken])> {
    let cost_tokens = repeat_till::<_, _, (), _, _, _, _>(
        1..,
        any.void(),
        peek(alt((
            primitives::period().void(),
            primitives::token_kind(TokenKind::LParen).void(),
            eof.void(),
        ))),
    )
    .map(|((), ())| ())
    .take()
    .parse_next(input)?;
    let head = parse_activation_cost_segment_kind_tokens(cost_tokens);
    if head == ActivationCostSegmentKind::BareSymbol {
        return Err(primitives::backtrack_err(
            "cycling activation cost",
            "typed non-mana activation cost head",
        ));
    }
    Ok((CyclingKeywordCostKind::Activation { head }, cost_tokens))
}

fn parse_cycling_pay_life_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(CyclingKeywordCostKind, &'a [OwnedLexToken])> {
    let ((_, amount, _), cost_tokens) = (
        primitives::kw("pay"),
        leaf::parse_leaf_number_prefix_lexed,
        primitives::kw("life"),
    )
        .with_taken()
        .parse_next(input)?;
    Ok((CyclingKeywordCostKind::PayLife { amount }, cost_tokens))
}

fn parse_cycling_mana_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(CyclingKeywordCostKind, &'a [OwnedLexToken])> {
    alt((
        parse_cycling_mana_before_reminder_cost_lexed,
        leaf::parse_leaf_mana_cost_prefix_lexed
            .with_taken()
            .map(|(prefix, tokens)| (CyclingKeywordCostKind::Mana(prefix.cost), tokens)),
    ))
    .parse_next(input)
}

fn parse_cycling_mana_before_reminder_cost_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<(CyclingKeywordCostKind, &'a [OwnedLexToken])> {
    repeat_till::<_, _, Vec<leaf::LeafManaPipToken>, _, _, _, _>(
        1..,
        leaf::parse_leaf_surface_mana_pip_lexed,
        peek((
            primitives::token_kind(TokenKind::Number),
            primitives::comma(),
            primitives::kw("discard"),
        )),
    )
    .map(|(pips, _)| {
        ManaCost::from_pips(
            pips.into_iter()
                .map(leaf::LeafManaPipToken::into_pip)
                .collect(),
        )
    })
    .with_taken()
    .map(|(cost, tokens)| (CyclingKeywordCostKind::Mana(cost), tokens))
    .parse_next(input)
}

fn parse_cycling_marker_token<'a>(input: &mut LexStream<'a>) -> WResult<&'a str> {
    primitives::word_parser_text
        .verify(|word: &str| parse_cycling_marker_word(word).is_some())
        .parse_next(input)
}

fn parse_cycling_keyword_shape_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CyclingKeywordShape> {
    let (prefix_atoms, marker_word) = repeat_till(
        0..,
        parse_cycling_filter_atom_lexed,
        parse_cycling_marker_token,
    )
    .parse_next(input)?;
    eof.parse_next(input)?;
    let root = parse_cycling_root_complete(marker_word).ok_or_else(|| {
        primitives::backtrack_err("cycling marker root", "word ending in cycling")
    })?;
    Ok(CyclingKeywordShape {
        prefix_atoms,
        root: root.to_string(),
    })
}

fn parse_cycling_filter_atom_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CyclingFilterAtom> {
    let word = primitives::word_parser_text.parse_next(input)?;
    Ok(classify_cycling_filter_word(word))
}

fn classify_cycling_filter_word(word: &str) -> CyclingFilterAtom {
    if let Ok(supertype) = leaf::parse_leaf_supertype_complete(word) {
        return CyclingFilterAtom::Supertype(supertype);
    }
    if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
        return CyclingFilterAtom::CardType(card_type);
    }
    if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
        return CyclingFilterAtom::Subtype(subtype);
    }
    if let Ok(colors) = leaf::parse_leaf_color_complete(word) {
        return CyclingFilterAtom::Colors(colors);
    }
    CyclingFilterAtom::Ignored
}

fn classify_cycling_root_word(word: &str) -> CyclingFilterAtom {
    if let Ok(card_type) = leaf::parse_leaf_card_type_complete(word) {
        return CyclingFilterAtom::CardType(card_type);
    }
    if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(word) {
        return CyclingFilterAtom::Subtype(subtype);
    }
    if let Ok(colors) = leaf::parse_leaf_color_complete(word) {
        return CyclingFilterAtom::Colors(colors);
    }
    CyclingFilterAtom::Ignored
}

fn add_cycling_filter_atom(
    mut spec: CyclingFilterSpec,
    atom: CyclingFilterAtom,
) -> CyclingFilterSpec {
    match atom {
        CyclingFilterAtom::Supertype(supertype) => {
            crate::slice_primitives::push_unique(&mut spec.supertypes, supertype);
        }
        CyclingFilterAtom::CardType(card_type) => {
            crate::slice_primitives::push_unique(&mut spec.card_types, card_type);
        }
        CyclingFilterAtom::Subtype(subtype) => {
            crate::slice_primitives::push_unique(&mut spec.subtypes, subtype);
            if subtype.is_land_subtype() {
                crate::slice_primitives::push_unique(&mut spec.card_types, CardType::Land);
            }
        }
        CyclingFilterAtom::Colors(colors) => {
            spec.colors = Some(spec.colors.map_or(colors, |current| current.union(colors)));
        }
        CyclingFilterAtom::Ignored => {}
    }
    spec
}

fn parse_cycling_root_complete(word: &str) -> Option<&str> {
    let mut input = word;
    parse_cycling_root_surface.parse_next(&mut input).ok()
}

fn parse_cycling_root_surface<'a>(input: &mut &'a str) -> WResult<&'a str> {
    let root = take_until(0.., "cycling").parse_next(input)?;
    literal("cycling").parse_next(input)?;
    eof.parse_next(input)?;
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::{lex_line, render_token_slice};
    use super::*;

    #[test]
    fn parses_mana_dual_and_pay_life_cost_groups() {
        let mana = lex_line("Cycling {2}", 0).unwrap();
        let groups = parse_cycling_keyword_cost_groups_tokens(&mana);
        assert_eq!(groups.len(), 1);
        assert!(matches!(
            groups[0].cost_kind,
            CyclingKeywordCostKind::Mana(_)
        ));

        let dual = lex_line("Forestcycling {2}, plainscycling {2}", 0).unwrap();
        let groups = parse_cycling_keyword_cost_groups_tokens(&dual);
        assert_eq!(groups.len(), 2);
        assert_eq!(
            render_token_slice(groups[1].keyword_tokens),
            "plainscycling"
        );

        let life = lex_line("Cycling—Pay 2 life. (Pay 2 life: Draw a card.)", 0).unwrap();
        let groups = parse_cycling_keyword_cost_groups_tokens(&life);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].cost_kind,
            CyclingKeywordCostKind::PayLife { amount: 2 }
        );
        assert_eq!(render_token_slice(groups[0].cost_tokens), "Pay 2 life");

        let sacrifice = lex_line(
            "Cycling—Sacrifice a land. (Sacrifice a land, Discard this card: Draw a card.)",
            0,
        )
        .unwrap();
        let groups = parse_cycling_keyword_cost_groups_tokens(&sacrifice);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].cost_kind,
            CyclingKeywordCostKind::Activation {
                head: ActivationCostSegmentKind::Sacrifice
            }
        );
        assert_eq!(
            render_token_slice(groups[0].cost_tokens),
            "Sacrifice a land"
        );

        let flattened_reminder =
            lex_line("Cycling {2} 2, Discard this card: Draw a card", 0).unwrap();
        let groups = parse_cycling_keyword_cost_groups_tokens(&flattened_reminder);
        assert_eq!(groups.len(), 1);
        assert_eq!(render_token_slice(groups[0].cost_tokens), "{2}");
    }

    #[test]
    fn returns_typed_search_filters() {
        let basic = lex_line("Basic landcycling", 0).unwrap();
        let spec = parse_cycling_search_spec_tokens(&basic).unwrap();
        assert!(matches!(
            spec,
            CyclingSearchSpec::Search(CyclingFilterSpec {
                supertypes,
                card_types,
                ..
            }) if supertypes == vec![Supertype::Basic] && card_types == vec![CardType::Land]
        ));

        let plain = lex_line("Cycling", 0).unwrap();
        assert_eq!(
            parse_cycling_search_spec_tokens(&plain),
            Ok(CyclingSearchSpec::Draw)
        );

        let sliver = lex_line("Slivercycling", 0).unwrap();
        assert!(matches!(
            parse_cycling_search_spec_tokens(&sliver),
            Ok(CyclingSearchSpec::Search(CyclingFilterSpec { subtypes, .. }))
                if subtypes == vec![Subtype::Sliver]
        ));
    }
}

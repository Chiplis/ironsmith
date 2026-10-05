use winnow::combinator::{alt, eof, opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, take_till};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::{leaf, primitives};
use crate::types::{CardType, Subtype};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraftMaterialKind {
    CardType { card_type: CardType, count: u32 },
    Subtype { subtype: Subtype, count: u32 },
    OneOrMore,
    RedInstantOrSorcery { minimum: u32 },
    Unsupported,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CraftLineSpec<'a> {
    pub material: CraftMaterialKind,
    pub material_tokens: &'a [OwnedLexToken],
    pub cost_tokens: &'a [OwnedLexToken],
}

pub fn parse_craft_line_spec_tokens(tokens: &[OwnedLexToken]) -> Option<CraftLineSpec<'_>> {
    primitives::parse_prefix(tokens, parse_craft_line_spec_lexed).map(|(spec, _)| spec)
}

fn parse_craft_line_spec_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CraftLineSpec<'a>> {
    primitives::phrase(&["craft", "with"]).parse_next(input)?;
    let material_tokens = repeat_till(
        1..,
        any.void(),
        peek(leaf::parse_leaf_mana_cost_prefix_lexed),
    )
    .map(|((), _)| ())
    .take()
    .parse_next(input)?;
    let material = primitives::parse_all(
        material_tokens,
        parse_craft_material_kind_lexed,
        "craft-material-kind",
    )
    .unwrap_or(CraftMaterialKind::Unsupported);

    let ((_, _), cost_tokens) = (
        leaf::parse_leaf_mana_cost_prefix_lexed,
        take_till(0.., is_craft_suffix_boundary),
    )
        .with_taken()
        .parse_next(input)?;

    Ok(CraftLineSpec {
        material,
        material_tokens,
        cost_tokens,
    })
}

fn parse_craft_material_kind_lexed<'a>(input: &mut LexStream<'a>) -> WResult<CraftMaterialKind> {
    alt((
        (primitives::phrase(&["one", "or", "more"]), eof).value(CraftMaterialKind::OneOrMore),
        parse_red_instant_or_sorcery_material,
        parse_counted_material,
    ))
    .parse_next(input)
}

// Cardinality is data, never a separate branch for each printed count. Compound
// material requirements remain unsupported until they have their own semantics.
fn parse_counted_material<'a>(input: &mut LexStream<'a>) -> WResult<CraftMaterialKind> {
    let count = opt(leaf::parse_leaf_number_prefix_lexed).parse_next(input)?.unwrap_or(1);
    if count == 0 {
        return Err(primitives::backtrack_err("craft material", "positive material count"));
    }
    let word = primitives::word_parser_text.parse_next(input)?;
    let material = match word {
        "artifact" | "artifacts" => CraftMaterialKind::CardType { card_type: CardType::Artifact, count },
        "creature" | "creatures" => CraftMaterialKind::CardType { card_type: CardType::Creature, count },
        _ => CraftMaterialKind::Subtype {
            subtype: leaf::parse_leaf_subtype_flexible_complete(word)
                .map_err(|_| primitives::backtrack_err("craft material", "card type or subtype"))?,
            count,
        },
    };
    eof.parse_next(input)?;
    Ok(material)
}

fn parse_red_instant_or_sorcery_material<'a>(
    input: &mut LexStream<'a>,
) -> WResult<CraftMaterialKind> {
    let minimum = leaf::parse_leaf_number_prefix_lexed.parse_next(input)?;
    primitives::phrase(&["or", "more", "red", "instant"]).parse_next(input)?;
    alt((
        primitives::phrase(&["and", "or", "sorcery", "cards"]),
        primitives::phrase(&["and/or", "sorcery", "cards"]),
        primitives::phrase(&["or", "sorcery", "cards"]),
    ))
    .parse_next(input)?;
    eof.parse_next(input)?;
    Ok(CraftMaterialKind::RedInstantOrSorcery { minimum })
}

fn is_craft_suffix_boundary(token: &OwnedLexToken) -> bool {
    matches!(token.kind, TokenKind::LParen | TokenKind::Period)
}

#[cfg(test)]
mod tests {
    use super::super::super::super::lexer::lex_line;
    use super::*;

    fn parse(raw: &str) -> CraftLineSpec<'static> {
        let tokens = Box::leak(lex_line(raw, 0).unwrap().into_boxed_slice());
        parse_craft_line_spec_tokens(tokens).unwrap()
    }

    #[test]
    fn parses_supported_material_kinds_and_cost_spans() {
        let artifact = parse("Craft with artifact {3}{W}{W}");
        assert_eq!(artifact.material, CraftMaterialKind::CardType { card_type: CardType::Artifact, count: 1 });
        assert_eq!(artifact.cost_tokens.len(), 3);

        let creature = parse("Craft with creature {5}{G}{G}");
        assert_eq!(creature.material, CraftMaterialKind::CardType { card_type: CardType::Creature, count: 1 });

        let any = parse("Craft with one or more {5}");
        assert_eq!(any.material, CraftMaterialKind::OneOrMore);

        let red = parse("Craft with four or more red instant and/or sorcery cards {3}{R}{R}");
        assert_eq!(
            red.material,
            CraftMaterialKind::RedInstantOrSorcery { minimum: 4 }
        );
    }

    #[test]
    fn parses_subtypes_and_arbitrary_fixed_material_counts() {
        for (text, subtype) in [("Craft with Cave {5}{G}", Subtype::Cave), ("Craft with Island {3}{U}", Subtype::Island)] {
            assert_eq!(parse(text).material, CraftMaterialKind::Subtype { subtype, count: 1 });
        }
        for (word, count) in [("two", 2), ("three", 3), ("seven", 7)] {
            assert_eq!(parse(&format!("Craft with {word} creatures {{5}}{{B}}")).material,
                CraftMaterialKind::CardType { card_type: CardType::Creature, count });
        }
        for clause in ["zero creatures", "two artifacts and two creatures", "four or more creatures with different names"] {
            assert_eq!(parse(&format!("Craft with {clause} {{5}}")).material, CraftMaterialKind::Unsupported);
        }
    }

    #[test]
    fn stops_before_reminder_text() {
        let spec = parse(
            "Craft with artifact {2}{R} ({2}{R}, Exile this artifact: Return this transformed.)",
        );
        assert_eq!(spec.cost_tokens.len(), 2);
    }
}

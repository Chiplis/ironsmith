use winnow::combinator::{alt, eof, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::{any, take_till};

use super::super::super::lexer::{LexStream, OwnedLexToken, TokenKind};
use super::super::{leaf, primitives};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CraftMaterialKind {
    Artifact,
    Creature,
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
        (primitives::kw("artifact"), eof).value(CraftMaterialKind::Artifact),
        (primitives::kw("creature"), eof).value(CraftMaterialKind::Creature),
        (primitives::phrase(&["one", "or", "more"]), eof).value(CraftMaterialKind::OneOrMore),
        parse_red_instant_or_sorcery_material,
    ))
    .parse_next(input)
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
        assert_eq!(artifact.material, CraftMaterialKind::Artifact);
        assert_eq!(artifact.cost_tokens.len(), 3);

        let creature = parse("Craft with creature {5}{G}{G}");
        assert_eq!(creature.material, CraftMaterialKind::Creature);

        let any = parse("Craft with one or more {5}");
        assert_eq!(any.material, CraftMaterialKind::OneOrMore);

        let red = parse("Craft with four or more red instant and/or sorcery cards {3}{R}{R}");
        assert_eq!(
            red.material,
            CraftMaterialKind::RedInstantOrSorcery { minimum: 4 }
        );
    }

    #[test]
    fn stops_before_reminder_text() {
        let spec = parse(
            "Craft with artifact {2}{R} ({2}{R}, Exile this artifact: Return this transformed.)",
        );
        assert_eq!(spec.cost_tokens.len(), 2);
    }
}

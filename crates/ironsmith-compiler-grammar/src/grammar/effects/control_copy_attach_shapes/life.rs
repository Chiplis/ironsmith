use winnow::combinator::{alt, eof};
use winnow::prelude::*;

use crate::grammar::{leaf, permission_shapes, primitives};
use crate::lexer::{LexStream, OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactLifeSurface {
    Fixed(u32),
    LoseGame,
    NoLifePrevention,
}

#[derive(Debug, Clone, Copy)]
pub struct LifeSurfaceShape<'a> {
    pub exact: Option<ExactLifeSurface>,
    pub remap_its_source_stat: bool,
    pub unsupported_shuffle_graveyard: bool,
    pub unless_tail: Option<&'a [OwnedLexToken]>,
}

fn exact_no_life(input: &mut LexStream<'_>) -> winnow::error::ModalResult<()> {
    alt((
        primitives::phrase(&["no", "life", "instead"]),
        primitives::phrase(&["no", "life", "this", "turn", "instead"]),
        primitives::phrase(&["no", "life"]),
    ))
    .void()
    .parse_next(input)?;
    eof.void().parse_next(input)
}

fn parse_exact_life_surface(tokens: &[OwnedLexToken]) -> Option<ExactLifeSurface> {
    let tokens = trim_lexed_commas(tokens);
    if permission_shapes::exact_tokens(tokens, &["the", "game"]) {
        return Some(ExactLifeSurface::LoseGame);
    }
    if exact_no_life.parse(LexStream::new(tokens)).is_ok() {
        return Some(ExactLifeSurface::NoLifePrevention);
    }
    let parsed = leaf::parse_leaf_number_prefix_tokens(tokens)?;
    let (amount, consumed) = parsed.into_fixed()?;
    let mut rest = LexStream::new(tokens.get(consumed..)?);
    crate::grammar::primitives::take_leaf(&mut rest, primitives::kw("life"))?;
    if !rest.is_empty() {
        return None;
    }
    Some(ExactLifeSurface::Fixed(amount))
}

pub fn parse_life_surface_shape(tokens: &[OwnedLexToken]) -> LifeSurfaceShape<'_> {
    let unless_tail =
        primitives::parse_prefix(trim_lexed_commas(tokens), primitives::kw("unless").void())
            .map(|(_, rest)| trim_lexed_commas(rest));
    LifeSurfaceShape {
        exact: parse_exact_life_surface(tokens),
        remap_its_source_stat: [
            &["its", "power"][..],
            &["its", "toughness"][..],
            &["its", "mana", "value"][..],
        ]
        .into_iter()
        .any(|phrase| permission_shapes::contains_tokens(tokens, phrase)),
        unsupported_shuffle_graveyard: permission_shapes::contains_tokens(
            tokens,
            &["then", "shuffle", "your", "graveyard", "into", "your"],
        ) && primitives::contains_word(tokens, "library"),
        unless_tail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_exact_and_source_stat_life_surfaces() {
        let fixed = lex_line("twenty one life", 0).unwrap();
        assert_eq!(
            parse_life_surface_shape(&fixed).exact,
            Some(ExactLifeSurface::Fixed(21))
        );
        let stat = lex_line("life equal to its power", 0).unwrap();
        assert!(parse_life_surface_shape(&stat).remap_its_source_stat);
        let game = lex_line("the game", 0).unwrap();
        assert_eq!(
            parse_life_surface_shape(&game).exact,
            Some(ExactLifeSurface::LoseGame)
        );
    }
}

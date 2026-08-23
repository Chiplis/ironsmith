use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

use super::super::super::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChosenTypeReferenceSurface {
    ThatType,
    ChosenType,
}

fn chosen_type_reference<'a>(input: &mut LexStream<'a>) -> WResult<ChosenTypeReferenceSurface> {
    let suffix = || {
        alt((
            primitives::phrase(&["of", "the", "chosen", "type"])
                .value(ChosenTypeReferenceSurface::ChosenType),
            primitives::phrase(&["of", "chosen", "type"])
                .value(ChosenTypeReferenceSurface::ChosenType),
            primitives::phrase(&["of", "that", "type"]).value(ChosenTypeReferenceSurface::ThatType),
        ))
    };
    repeat_till(0.., any.void(), peek(suffix()).void())
        .map(|((), ())| ())
        .parse_next(input)?;
    suffix().parse_next(input)
}

pub fn parse_chosen_type_reference_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ChosenTypeReferenceSurface> {
    primitives::parse_prefix(tokens, chosen_type_reference).map(|(surface, _)| surface)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn recognizes_land_filter_type_references_without_inferring_the_type_domain() {
        let that_type = lex_line("each land you control of that type", 0).unwrap();
        assert_eq!(
            parse_chosen_type_reference_tokens(&that_type),
            Some(ChosenTypeReferenceSurface::ThatType)
        );

        let chosen_type = lex_line("lands of the chosen type you control", 0).unwrap();
        assert_eq!(
            parse_chosen_type_reference_tokens(&chosen_type),
            Some(ChosenTypeReferenceSurface::ChosenType)
        );

        let unrelated = lex_line("each land you control", 0).unwrap();
        assert_eq!(parse_chosen_type_reference_tokens(&unrelated), None);
    }

    #[test]
    fn land_domain_turns_that_type_into_a_chosen_land_type_filter() {
        let tokens = lex_line("each land you control of that type", 0).unwrap();
        let filter = super::super::parse_object_filter(&tokens, false).unwrap();
        assert!(filter.chosen_land_type, "{filter:#?}");
        assert!(!filter.chosen_creature_type, "{filter:#?}");
    }
}

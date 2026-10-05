//! Strict ownership of a source-filtered damage prevention prohibition.
use super::super::primitives;
use crate::lexer::{LexStream, OwnedLexToken};
use winnow::combinator::{opt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilteredUnpreventability<'a> {
    pub sources: &'a [OwnedLexToken],
    pub combat_only: bool,
}

pub fn parse_filtered_unpreventability(
    tokens: &[OwnedLexToken],
) -> Option<FilteredUnpreventability<'_>> {
    primitives::probe_all(
        tokens,
        filtered_unpreventability,
        "source-filtered unpreventable damage",
    )
}
fn filtered_unpreventability<'a>(
    input: &mut LexStream<'a>,
) -> WResult<FilteredUnpreventability<'a>> {
    let combat_only = opt(primitives::kw("combat")).parse_next(input)?.is_some();
    primitives::phrase(&["damage", "that", "would", "be", "dealt", "by"]).parse_next(input)?;
    let sources =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::kw("can't")))
            .map(|((), _)| ())
            .take()
            .parse_next(input)?;
    primitives::phrase(&["can't", "be", "prevented"]).parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(FilteredUnpreventability {
        sources,
        combat_only,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn owns_only_complete_source_filtered_prevention_prohibitions() {
        for (text, combat) in [
            (
                "Damage that would be dealt by this creature can't be prevented.",
                false,
            ),
            (
                "Combat damage that would be dealt by creatures you control can't be prevented.",
                true,
            ),
        ] {
            let tokens = lex_line(text, 0).unwrap();
            let parsed = parse_filtered_unpreventability(&tokens).unwrap();
            assert_eq!(parsed.combat_only, combat);
            assert!(!parsed.sources.is_empty());
        }
        for text in [
            "Damage that would be dealt by this creature can't be prevented or dealt instead to another player.",
            "Damage that would be dealt to this creature can't be prevented.",
            "If this creature would deal damage, that damage can't be prevented.",
        ] {
            assert!(
                parse_filtered_unpreventability(&lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }
}

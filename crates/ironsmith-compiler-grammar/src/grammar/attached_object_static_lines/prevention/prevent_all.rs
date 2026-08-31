use winnow::combinator::alt;
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};

use super::super::subjects::{semantic_finish, semantic_phrase};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachedPreventAllKind {
    DamageDealtToAndBy,
    DamageDealtBy,
    CombatDamageDealtBy,
    DamageDealtTo,
}

pub fn parse_attached_prevent_all_tokens(
    tokens: &[OwnedLexToken],
) -> Option<AttachedPreventAllKind> {
    crate::grammar::primitives::probe_all(
        tokens,
        parse_attached_prevent_all_lexed,
        "attached prevent-all line",
    )
}

fn parse_attached_prevent_all_lexed<'a>(
    input: &mut LexStream<'a>,
) -> WResult<AttachedPreventAllKind> {
    let kind = alt((
        semantic_phrase(&[
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "and",
            "dealt",
            "by",
            "enchanted",
            "creature",
        ])
        .value(AttachedPreventAllKind::DamageDealtToAndBy),
        semantic_phrase(&[
            "prevent",
            "all",
            "combat",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "by",
            "enchanted",
            "creature",
        ])
        .value(AttachedPreventAllKind::CombatDamageDealtBy),
        semantic_phrase(&[
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "by",
            "enchanted",
            "creature",
        ])
        .value(AttachedPreventAllKind::DamageDealtBy),
        semantic_phrase(&[
            "prevent",
            "all",
            "damage",
            "that",
            "would",
            "be",
            "dealt",
            "to",
            "enchanted",
            "creature",
        ])
        .value(AttachedPreventAllKind::DamageDealtTo),
    ))
    .parse_next(input)?;
    semantic_finish(input)?;
    Ok(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_single_and_combined_attached_prevention_shapes() {
        let combat = lex_line(
            "Prevent all combat damage that would be dealt by enchanted creature.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_attached_prevent_all_tokens(&combat),
            Some(AttachedPreventAllKind::CombatDamageDealtBy)
        );

        let combined = lex_line(
            "Prevent all damage that would be dealt to and dealt by enchanted creature.",
            0,
        )
        .unwrap();
        assert_eq!(
            parse_attached_prevent_all_tokens(&combined),
            Some(AttachedPreventAllKind::DamageDealtToAndBy)
        );
    }
}

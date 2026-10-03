use std::ops::Range;

use winnow::Parser;
use winnow::combinator::alt;

use crate::lexer::OwnedLexToken;

use super::super::primitives;

/// A passive noncombat-damage event with no restriction on its source.
/// Player/object resolution remains with the semantic trigger assembler.
pub fn parse_passive_noncombat_damage_recipient(tokens: &[OwnedLexToken]) -> Option<Range<usize>> {
    let (end, _, rest) = primitives::find_prefix(tokens, || {
        (
            alt((primitives::kw("is"), primitives::kw("are"))),
            primitives::phrase(&["dealt", "noncombat", "damage"]),
            primitives::sentence_end(),
        )
            .void()
    })?;
    (end > 0 && rest.is_empty()).then_some(0..end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn passive_noncombat_damage_requires_the_complete_qualified_event() {
        for (text, words) in [
            ("an opponent is dealt noncombat damage", 2),
            ("one or more opponents are dealt noncombat damage", 4),
        ] {
            assert_eq!(
                parse_passive_noncombat_damage_recipient(&lex_line(text, 0).unwrap()),
                Some(0..words)
            );
        }
        for text in [
            "an opponent is dealt combat damage",
            "an opponent is dealt noncombat damage by a creature",
            "an opponent is dealt noncombat damage during your turn",
        ] {
            assert!(
                parse_passive_noncombat_damage_recipient(&lex_line(text, 0).unwrap()).is_none()
            );
        }
    }
}

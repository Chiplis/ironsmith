use std::ops::Range;

use winnow::Parser;

use crate::events::KeywordActionKind;
use crate::lexer::OwnedLexToken;

use super::super::primitives;

/// Two complete keyword-action verbs sharing a subject. The caller resolves
/// the subject as a player before constructing either event predicate.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedKeywordActionAlternatives {
    pub subject: Range<usize>,
    pub left: KeywordActionKind,
    pub right: KeywordActionKind,
}

pub fn parse_shared_keyword_action_alternatives(
    tokens: &[OwnedLexToken],
) -> Option<SharedKeywordActionAlternatives> {
    let (subject_end, (left, _, right, _), rest) = primitives::find_prefix(tokens, || {
        (
            primitives::word_text.verify_map(KeywordActionKind::from_trigger_word),
            primitives::kw("or"),
            primitives::word_text.verify_map(KeywordActionKind::from_trigger_word),
            primitives::sentence_end(),
        )
    })?;
    if subject_end == 0 || !rest.is_empty() {
        return None;
    }
    Some(SharedKeywordActionAlternatives {
        subject: 0..subject_end,
        left,
        right,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn keyword_alternatives_retain_shared_subject_and_typed_actions() {
        let tokens = lex_line("an opponent scries or surveils", 0).unwrap();
        let parsed = parse_shared_keyword_action_alternatives(&tokens).unwrap();
        assert_eq!(parsed.subject, 0..2);
        assert_eq!(parsed.left, KeywordActionKind::Scry);
        assert_eq!(parsed.right, KeywordActionKind::Surveil);
    }

    #[test]
    fn keyword_alternatives_do_not_consume_explicit_subjects_or_qualifiers() {
        for text in [
            "you scry or an opponent surveils",
            "you scry or surveil during your turn",
            "you cycle or discard another card",
            "scry or surveil",
        ] {
            assert!(
                parse_shared_keyword_action_alternatives(&lex_line(text, 0).unwrap()).is_none(),
                "{text} must be handled by its complete clause grammar"
            );
        }
    }
}

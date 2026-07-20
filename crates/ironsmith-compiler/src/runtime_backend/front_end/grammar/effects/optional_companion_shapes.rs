use super::super::super::lexer::{OwnedLexToken, trim_lexed_commas};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptionalCompanionJoin {
    Each,
    Both,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SharedSubjectOptionalCompanionShape<'a> {
    pub(crate) first_subject_tokens: &'a [OwnedLexToken],
    pub(crate) companion_tokens: &'a [OwnedLexToken],
    pub(crate) action_tokens: &'a [OwnedLexToken],
    pub(crate) join: OptionalCompanionJoin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadingOptionalCompanionVerb {
    Destroy,
    Tap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingOptionalCompanionShape<'a> {
    pub(crate) verb: LeadingOptionalCompanionVerb,
    pub(crate) first_target_tokens: &'a [OwnedLexToken],
    pub(crate) companion_tokens: &'a [OwnedLexToken],
}

fn word(token: &OwnedLexToken) -> Option<&str> {
    token.as_word().map(|_| token.parser_text())
}

fn is_cant(token: &OwnedLexToken) -> bool {
    matches!(word(token), Some("cant" | "can't"))
}

fn optional_companion_separator(tokens: &[OwnedLexToken]) -> Option<usize> {
    tokens.windows(3).position(|window| {
        word(&window[0]) == Some("and")
            && word(&window[1]) == Some("up")
            && word(&window[2]) == Some("to")
    })
}

fn explicit_target_phrase_count(tokens: &[OwnedLexToken]) -> usize {
    tokens
        .iter()
        .filter(|token| token.is_word("target"))
        .count()
}

/// Captures a shared predicate whose subjects are one required/source object
/// plus an independently optional target, for example:
///
/// `this creature and up to one other target creature each get ...`
///
/// The shape deliberately owns only coordination. Target counts, `other`,
/// controller restrictions, the action, and the duration stay delegated to
/// their existing semantic parsers.
pub(crate) fn parse_shared_subject_optional_companion_shape(
    tokens: &[OwnedLexToken],
) -> Option<SharedSubjectOptionalCompanionShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let separator = optional_companion_separator(tokens)?;
    let first_subject_tokens = trim_lexed_commas(&tokens[..separator]);
    if first_subject_tokens.is_empty() {
        return None;
    }

    let companion_and_action = &tokens[separator + 1..];
    let (action_idx, action_word_idx, join) = companion_and_action
        .windows(2)
        .enumerate()
        .find_map(|(idx, window)| match (word(&window[0]), word(&window[1])) {
            (Some("each"), Some("get" | "gets")) => {
                Some((idx, idx + 1, OptionalCompanionJoin::Each))
            }
            (Some("both"), Some("gain" | "gains")) => {
                Some((idx, idx + 1, OptionalCompanionJoin::Both))
            }
            _ => None,
        })
        .or_else(|| {
            companion_and_action
                .iter()
                .position(is_cant)
                .map(|idx| (idx, idx, OptionalCompanionJoin::Plain))
        })?;

    let companion_tokens = trim_lexed_commas(&companion_and_action[..action_idx]);
    let action_tokens = trim_lexed_commas(&companion_and_action[action_word_idx..]);
    if companion_tokens.is_empty()
        || action_tokens.is_empty()
        || !companion_tokens.iter().any(|token| token.is_word("target"))
    {
        return None;
    }

    Some(SharedSubjectOptionalCompanionShape {
        first_subject_tokens,
        companion_tokens,
        action_tokens,
        join,
    })
}

/// Captures a leading action applied to one required/source object and one
/// independently optional target, such as `tap it and up to one target ...`
/// or `destroy target artifact and up to one other target artifact`.
pub(crate) fn parse_leading_optional_companion_shape(
    tokens: &[OwnedLexToken],
) -> Option<LeadingOptionalCompanionShape<'_>> {
    let tokens = trim_lexed_commas(tokens);
    let (verb, body) = match tokens.first().and_then(word) {
        Some("destroy") => (LeadingOptionalCompanionVerb::Destroy, &tokens[1..]),
        Some("tap") => (LeadingOptionalCompanionVerb::Tap, &tokens[1..]),
        _ => return None,
    };
    let separator = optional_companion_separator(body)?;
    let first_target_tokens = trim_lexed_commas(&body[..separator]);
    let companion_tokens = trim_lexed_commas(&body[separator + 1..]);
    // This grammar owns a two-subject companion shape. In an Oxford-comma
    // list of three or more explicit targets, the final `and up to` would
    // otherwise make every earlier target look like one compound first
    // subject. Leave those lists to the generic multi-target fanout parser,
    // which gives every repeated target phrase its own choice slot and wraps
    // all of the resulting actions in one coordinated sequence.
    if explicit_target_phrase_count(body) > 2 {
        return None;
    }
    if first_target_tokens.is_empty()
        || companion_tokens.is_empty()
        || !companion_tokens.iter().any(|token| token.is_word("target"))
    {
        return None;
    }
    Some(LeadingOptionalCompanionShape {
        verb,
        first_target_tokens,
        companion_tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::{lex_line, render_token_slice};

    #[test]
    fn captures_source_optional_target_and_shared_predicate_separately() {
        let tokens = lex_line(
            "This creature and up to one other target creature you control each get +3/+3 until end of turn.",
            0,
        )
        .unwrap();
        let shape = parse_shared_subject_optional_companion_shape(&tokens).unwrap();
        assert_eq!(
            render_token_slice(shape.first_subject_tokens),
            "This creature"
        );
        assert_eq!(
            render_token_slice(shape.companion_tokens),
            "up to one other target creature you control"
        );
        assert_eq!(
            render_token_slice(shape.action_tokens),
            "get +3/+3 until end of turn."
        );
        assert_eq!(shape.join, OptionalCompanionJoin::Each);
    }

    #[test]
    fn captures_named_source_with_optional_target() {
        let tokens = lex_line(
            "Lockjaw and up to one other target creature you control can't be blocked this turn.",
            0,
        )
        .unwrap();
        let shape = parse_shared_subject_optional_companion_shape(&tokens).unwrap();
        assert_eq!(render_token_slice(shape.first_subject_tokens), "Lockjaw");
        assert_eq!(
            render_token_slice(shape.companion_tokens),
            "up to one other target creature you control"
        );
        assert_eq!(
            render_token_slice(shape.action_tokens),
            "can't be blocked this turn."
        );
        assert_eq!(shape.join, OptionalCompanionJoin::Plain);
    }

    #[test]
    fn captures_leading_action_without_absorbing_the_optional_count() {
        let tokens = lex_line(
            "Destroy target artifact or enchantment and up to one other target artifact or enchantment.",
            0,
        )
        .unwrap();
        let shape = parse_leading_optional_companion_shape(&tokens).unwrap();
        assert_eq!(shape.verb, LeadingOptionalCompanionVerb::Destroy);
        assert_eq!(
            render_token_slice(shape.first_target_tokens),
            "target artifact or enchantment"
        );
        assert_eq!(
            render_token_slice(shape.companion_tokens),
            "up to one other target artifact or enchantment."
        );
    }

    #[test]
    fn leading_pair_does_not_claim_a_multi_slot_shared_action() {
        let tokens = lex_line(
            "Destroy up to one target artifact, up to one target creature, and up to one target enchantment.",
            0,
        )
        .unwrap();
        assert!(parse_leading_optional_companion_shape(&tokens).is_none());
    }
}

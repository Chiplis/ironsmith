use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::{line_families, permission_shapes, primitives};
use crate::runtime_backend::lexer::{LexStream, OwnedLexToken, TokenWordView, lex_line};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionalFollowupActor {
    You,
    They,
    ThatPlayer,
    ThePlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConditionalFollowupMatch<'a> {
    pub(crate) actor: ConditionalFollowupActor,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LeadingMayActor {
    You,
    ThatPlayer,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LeadingMayActionMatch<'a> {
    pub(crate) actor: LeadingMayActor,
    pub(crate) verb: &'static str,
    pub(crate) tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordMarkerKind {
    Prototype,
    MoreThanMeetsTheEye,
    TicketSticker,
    Compleated,
    Dredge,
}

pub(crate) fn parse_word_prefix_presence(words: &TokenWordView<'_>, expected: &[&str]) -> bool {
    permission_shapes::prefix_words(&words.word_refs(), expected)
}

pub(crate) fn parse_any_word_prefix_presence(
    words: &TokenWordView<'_>,
    expected: &[&[&str]],
) -> bool {
    for prefix in expected {
        if permission_shapes::prefix_words(&words.word_refs(), prefix) {
            return true;
        }
    }
    false
}

fn conditional_followup_actor(input: &mut LexStream<'_>) -> WResult<ConditionalFollowupActor> {
    alt((
        primitives::phrase(&["if", "you", "do"]).value(ConditionalFollowupActor::You),
        primitives::phrase(&["if", "they", "do"]).value(ConditionalFollowupActor::They),
        primitives::phrase(&["if", "that", "player", "does"])
            .value(ConditionalFollowupActor::ThatPlayer),
        primitives::phrase(&["if", "the", "player", "does"])
            .value(ConditionalFollowupActor::ThePlayer),
    ))
    .parse_next(input)
}

pub(crate) fn parse_conditional_followup_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalFollowupMatch<'_>> {
    let (actor, tail_tokens) = primitives::parse_prefix(tokens, conditional_followup_actor)?;
    Some(ConditionalFollowupMatch { actor, tail_tokens })
}

pub(crate) fn has_nonconditional_instead(tokens: &[OwnedLexToken]) -> bool {
    if primitives::parse_prefix(tokens, primitives::kw("if")).is_some() {
        return false;
    }
    primitives::find_prefix(tokens, || primitives::kw("instead").void()).is_some()
}

fn may_actor(input: &mut LexStream<'_>) -> WResult<LeadingMayActor> {
    alt((
        primitives::phrase(&["you", "may"]).value(LeadingMayActor::You),
        primitives::phrase(&["that", "player", "may"]).value(LeadingMayActor::ThatPlayer),
        primitives::phrase(&["they", "may"]).value(LeadingMayActor::ThatPlayer),
        primitives::kw("may").value(LeadingMayActor::Default),
    ))
    .parse_next(input)
}

fn allowed_verb(
    input: &mut LexStream<'_>,
    verbs: &'static [&'static str],
) -> WResult<&'static str> {
    let parsed = primitives::word_parser_text.parse_next(input)?;
    for verb in verbs {
        if parsed == *verb {
            return Ok(*verb);
        }
    }
    Err(primitives::backtrack_err(
        "leading may action",
        "allowed action verb",
    ))
}

pub(crate) fn parse_leading_may_action_tokens<'a>(
    tokens: &'a [OwnedLexToken],
    verbs: &'static [&'static str],
    allow_bare: bool,
) -> Option<LeadingMayActionMatch<'a>> {
    let parser = |input: &mut LexStream<'a>| -> WResult<(LeadingMayActor, &'static str)> {
        let actor = if allow_bare {
            opt(may_actor)
                .parse_next(input)?
                .unwrap_or(LeadingMayActor::Default)
        } else {
            may_actor.parse_next(input)?
        };
        let verb = allowed_verb(input, verbs)?;
        Ok((actor, verb))
    };
    let ((actor, verb), tail_tokens) = primitives::parse_prefix(tokens, parser)?;
    Some(LeadingMayActionMatch {
        actor,
        verb,
        tail_tokens,
    })
}

pub(crate) fn parse_keyword_marker_text(text: &str) -> Option<KeywordMarkerKind> {
    let lowered = text.trim_start().to_ascii_lowercase();
    let input = lowered.as_str();
    if input == "compleated" {
        return Some(KeywordMarkerKind::Compleated);
    }
    for (mut prefix, kind) in [
        ("prototype ", KeywordMarkerKind::Prototype),
        (
            "more than meets the eye ",
            KeywordMarkerKind::MoreThanMeetsTheEye,
        ),
        ("dredge ", KeywordMarkerKind::Dredge),
    ] {
        let mut candidate = input;
        let matched: WResult<&str> = prefix.parse_next(&mut candidate);
        if matched.is_ok() {
            return Some(kind);
        }
    }
    let tokens = lex_line(input, 0).ok()?;
    line_families::parse_sticker_ticket_marker(&tokens).map(|_| KeywordMarkerKind::TicketSticker)
}

pub(crate) fn recognizes_ticket_sticker_marker(text: &str) -> bool {
    parse_keyword_marker_text(text) == Some(KeywordMarkerKind::TicketSticker)
}

pub(crate) fn recognizes_core_keyword_marker(text: &str) -> bool {
    matches!(
        parse_keyword_marker_text(text),
        Some(
            KeywordMarkerKind::Prototype
                | KeywordMarkerKind::MoreThanMeetsTheEye
                | KeywordMarkerKind::TicketSticker
        )
    )
}

pub(crate) fn recognizes_static_keyword_marker(text: &str) -> bool {
    parse_keyword_marker_text(text).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_conditional_and_may_action_prefixes() {
        let tokens = lex_line("If that player does, put it on top.", 0).expect("lex");
        let followup = parse_conditional_followup_tokens(&tokens).expect("followup");
        assert_eq!(followup.actor, ConditionalFollowupActor::ThatPlayer);
        assert_eq!(
            TokenWordView::new(followup.tail_tokens).first(),
            Some("put")
        );

        let tokens = lex_line("They may reveal that card.", 0).expect("lex");
        let action =
            parse_leading_may_action_tokens(&tokens, &["reveal"], false).expect("may action");
        assert_eq!(action.actor, LeadingMayActor::ThatPlayer);
        assert_eq!(action.verb, "reveal");
    }

    #[test]
    fn classifies_keyword_markers() {
        assert_eq!(
            parse_keyword_marker_text("Prototype {3}{U}"),
            Some(KeywordMarkerKind::Prototype)
        );
        assert!(recognizes_ticket_sticker_marker("{TK}{TK} — Prize sticker"));
        assert!(recognizes_static_keyword_marker("Compleated"));
        assert!(!recognizes_core_keyword_marker("Dredge 4"));
    }
}

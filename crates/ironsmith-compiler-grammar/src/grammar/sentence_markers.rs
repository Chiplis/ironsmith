use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;

use super::{line_families, permission_shapes, primitives};
use crate::lexer::{LexStream, OwnedLexToken, parser_token_word_refs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalFollowupActor {
    You,
    They,
    ThatPlayer,
    ThePlayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConditionalFollowupMatch<'a> {
    pub actor: ConditionalFollowupActor,
    pub tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeadingMayActor {
    You,
    ThatPlayer,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeadingMayActionMatch<'a> {
    pub actor: LeadingMayActor,
    pub verb: &'static str,
    pub tail_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordMarkerKind {
    Prototype,
    MoreThanMeetsTheEye,
    TicketSticker,
    Compleated,
    Dredge,
    SpaceSculptor,
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

pub fn parse_conditional_followup_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ConditionalFollowupMatch<'_>> {
    let (actor, tail_tokens) = primitives::parse_prefix(tokens, conditional_followup_actor)?;
    Some(ConditionalFollowupMatch { actor, tail_tokens })
}

pub fn has_nonconditional_instead(tokens: &[OwnedLexToken]) -> bool {
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

pub fn parse_leading_may_action_tokens<'a>(
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

const PROTOTYPE_HEAD: &[&str] = &["prototype"];
const MORE_THAN_MEETS_THE_EYE_HEAD: &[&str] = &["more", "than", "meets", "the", "eye"];
const DREDGE_HEAD: &[&str] = &["dredge"];

/// A keyword marker line ("Prototype {3}{U}", "Compleated", "{TK}{TK} — Prize
/// sticker"), read from the line's tokens.
pub fn parse_keyword_marker_tokens(tokens: &[OwnedLexToken]) -> Option<KeywordMarkerKind> {
    let words = parser_token_word_refs(tokens);
    if permission_shapes::exact_words(&words, &["compleated"]) {
        return Some(KeywordMarkerKind::Compleated);
    }
    if permission_shapes::exact_words(&words, &["space", "sculptor"]) {
        return Some(KeywordMarkerKind::SpaceSculptor);
    }
    for (head, kind) in [
        (PROTOTYPE_HEAD, KeywordMarkerKind::Prototype),
        (
            MORE_THAN_MEETS_THE_EYE_HEAD,
            KeywordMarkerKind::MoreThanMeetsTheEye,
        ),
        (DREDGE_HEAD, KeywordMarkerKind::Dredge),
    ] {
        if let Some(((), tail_tokens)) = primitives::parse_prefix(tokens, primitives::phrase(head))
            && marker_payload_follows(tokens, tail_tokens)
        {
            return Some(kind);
        }
    }
    line_families::parse_sticker_ticket_marker(tokens).map(|_| KeywordMarkerKind::TicketSticker)
}

/// A marker's payload stands apart from its keyword ("Dredge 4"); the keyword
/// alone, or with punctuation glued on, is not a marker.
fn marker_payload_follows(tokens: &[OwnedLexToken], tail_tokens: &[OwnedLexToken]) -> bool {
    let Some(next) = tail_tokens.first() else {
        return false;
    };
    let head_len = tokens.len() - tail_tokens.len();
    head_len > 0 && tokens[head_len - 1].span.end < next.span.start
}

pub fn recognizes_ticket_sticker_marker_tokens(tokens: &[OwnedLexToken]) -> bool {
    parse_keyword_marker_tokens(tokens) == Some(KeywordMarkerKind::TicketSticker)
}

pub fn recognizes_core_keyword_marker_tokens(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        parse_keyword_marker_tokens(tokens),
        Some(
            KeywordMarkerKind::Prototype
                | KeywordMarkerKind::MoreThanMeetsTheEye
                | KeywordMarkerKind::TicketSticker
                | KeywordMarkerKind::SpaceSculptor
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

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
        let marker = |text: &str| parse_keyword_marker_tokens(&lex_line(text, 0).expect("lex"));
        assert_eq!(
            marker("Prototype {3}{U}"),
            Some(KeywordMarkerKind::Prototype)
        );
        assert!(recognizes_ticket_sticker_marker_tokens(
            &lex_line("{TK}{TK} — Prize sticker", 0).expect("lex")
        ));
        assert_eq!(marker("Compleated"), Some(KeywordMarkerKind::Compleated));
        assert_eq!(
            marker("Space sculptor"),
            Some(KeywordMarkerKind::SpaceSculptor)
        );
        assert_eq!(marker("Dredge 4"), Some(KeywordMarkerKind::Dredge));
        assert!(!recognizes_core_keyword_marker_tokens(
            &lex_line("Dredge 4", 0).expect("lex")
        ));
        assert_eq!(marker("Dredge"), None, "the keyword alone is not a marker");
    }
}

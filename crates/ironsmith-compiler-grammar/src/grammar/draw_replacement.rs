//! A draw replacement owns its complete following program, including result
//! continuations. Only the outer instead marker is removed from that program.
use super::primitives::{self, comma, kw, phrase};
use crate::lexer::{LexStream, OwnedLexToken};
use winnow::combinator::{alt, opt, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;

pub(super) const EXCEPT_FIRST_YOU: &[&str] = &[
    "except", "the", "first", "one", "you", "draw", "in", "each", "of", "your", "draw", "steps",
];
pub(super) const EXCEPT_FIRST_THEY: &[&str] = &[
    "except", "the", "first", "one", "they", "draw", "in", "each", "of", "their", "draw", "steps",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawReplacementPlayer {
    You,
    Opponent,
    Any,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawReplacementProgram {
    pub player: DrawReplacementPlayer,
    pub except_first_of_draw_step: bool,
    pub empty_library: bool,
    pub optional: bool,
    pub skip: bool,
    pub body: Vec<OwnedLexToken>,
    /// "If a cycling ability of <object> would cause you to draw a card":
    /// the object phrase naming the cycling ability's source.
    pub cycling_source: Option<Vec<OwnedLexToken>>,
    /// "If you would draw a card during your draw step" (Island Sanctuary):
    /// only draws proposed while it is the drawer's own draw step.
    pub during_your_draw_step: bool,
}
fn cycling_cause_header<'a>(input: &mut LexStream<'a>) -> WResult<&'a [OwnedLexToken]> {
    const CAUSE: &[&str] = &["would", "cause", "you", "to", "draw", "a", "card"];
    phrase(&["if", "a", "cycling", "ability", "of"]).parse_next(input)?;
    let source =
        repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), primitives::peek(phrase(CAUSE)))
            .take()
            .parse_next(input)?;
    phrase(CAUSE).parse_next(input)?;
    comma().parse_next(input)?;
    Ok(source)
}

#[derive(Clone, Copy, PartialEq)]
enum DrawGate {
    ExceptFirst,
    EmptyLibrary,
    DuringDrawStep,
}

fn header(input: &mut LexStream<'_>) -> WResult<(DrawReplacementPlayer, Option<DrawGate>)> {
    kw("if").parse_next(input)?;
    let player = alt((
        kw("you").value(DrawReplacementPlayer::You),
        phrase(&["an", "opponent"]).value(DrawReplacementPlayer::Opponent),
        phrase(&["a", "player"]).value(DrawReplacementPlayer::Any),
    ))
    .parse_next(input)?;
    phrase(&["would", "draw", "a", "card"]).parse_next(input)?;
    let gate = if player == DrawReplacementPlayer::You {
        opt(alt((
            phrase(EXCEPT_FIRST_YOU).value(DrawGate::ExceptFirst),
            phrase(&["while", "your", "library", "has", "no", "cards", "in", "it"])
                .value(DrawGate::EmptyLibrary),
            phrase(&["during", "your", "draw", "step"]).value(DrawGate::DuringDrawStep),
        )))
        .parse_next(input)?
    } else {
        opt(phrase(EXCEPT_FIRST_THEY).value(DrawGate::ExceptFirst)).parse_next(input)?
    };
    if gate == Some(DrawGate::EmptyLibrary) {
        alt((comma().void(), primitives::peek(kw("instead")).void())).parse_next(input)?;
    } else {
        comma().parse_next(input)?;
    }
    Ok((player, gate))
}

/// Strip one outer marker while preserving later sentences and quoted instructions.
pub(crate) fn replacement_body(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let after_instead =
        primitives::parse_prefix(tokens, (kw("instead"), opt(comma()))).map(|(_, rest)| rest);
    let body = after_instead.unwrap_or(tokens);
    let mut quoted = false;
    let end = body
        .iter()
        .position(|token| {
            if token.is_quote() {
                quoted = !quoted;
            }
            !quoted && token.is_period()
        })
        .unwrap_or(body.len());
    let trailing = end > 0 && !quoted && body[end - 1].is_word("instead");
    if after_instead.is_some() == trailing
        || end == 0
        || body.first().is_some_and(|token| token.is_word("instead"))
    {
        return None;
    }
    let mut result = body.to_vec();
    if trailing {
        result.remove(end - 1);
    }
    Some(result)
}

pub(crate) fn first_draw_each_turn(tokens: &[OwnedLexToken]) -> Option<(bool, Vec<OwnedLexToken>)> {
    let ((_, exception, _), body) = primitives::parse_prefix(
        tokens,
        (
            phrase(&[
                "the", "first", "time", "you", "would", "draw", "a", "card", "each", "turn",
            ]),
            opt(phrase(&[
                "except", "the", "first", "card", "you", "draw", "during", "each", "of", "your",
                "draw", "steps",
            ])),
            comma(),
        ),
    )?;
    let body = replacement_body(crate::lexer::trim_lexed_commas(body))?;
    (!body.is_empty()).then_some((exception.is_some(), body))
}

fn skip_draw(input: &mut LexStream<'_>) -> WResult<()> {
    alt((
        phrase(&["that", "player", "skips"]),
        (opt(kw("you")), kw("skip")).void(),
    ))
    .parse_next(input)?;
    phrase(&["that", "draw"]).parse_next(input)
}

pub fn parse_draw_replacement_program(tokens: &[OwnedLexToken]) -> Option<DrawReplacementProgram> {
    let ((player, gate), body, cycling_source) =
        if let Some((source, rest)) = primitives::parse_prefix(tokens, cycling_cause_header) {
            (
                (DrawReplacementPlayer::You, None),
                rest,
                Some(source.to_vec()),
            )
        } else {
            let (header, body) = primitives::parse_prefix(tokens, header)?;
            (header, body, None)
        };

    let before = primitives::strip_lexed_prefix_phrase(body, &["you", "may"]);
    let mut body = replacement_body(before.unwrap_or(body))?;
    let after = primitives::strip_lexed_prefix_phrase(&body, &["you", "may"]);
    let optional = before.is_some() || after.is_some();
    if let Some(rest) = after {
        if before.is_some() {
            return None;
        }
        body.drain(..body.len() - rest.len());
    }
    if primitives::starts_with_any_phrase(&body, &[&["they", "may"], &["that", "player", "may"]])
        || (optional
            && (player != DrawReplacementPlayer::You || gate == Some(DrawGate::ExceptFirst)))
    {
        return None;
    }
    let mut skip = false;
    if let Some(((), rest)) = primitives::parse_prefix(&body, skip_draw) {
        if primitives::probe_all(rest, primitives::sentence_end(), "skip proposed draw").is_some() {
            // Cancelling this proposed draw consumes it without scheduling a later skip.
            skip = true;
            body.clear();
        } else if let Some((_, followup)) = primitives::parse_prefix(
            rest,
            (primitives::period(), phrase(&["if", "you", "do"]), comma()),
        ) {
            // Accepting the replacement already satisfies "if you do".
            body.drain(..body.len() - followup.len());
        }
    }
    if body.is_empty() && !skip {
        return None;
    }
    Some(DrawReplacementProgram {
        player,
        optional,
        skip,
        body,
        cycling_source,
        except_first_of_draw_step: gate == Some(DrawGate::ExceptFirst),
        empty_library: gate == Some(DrawGate::EmptyLibrary),
        during_your_draw_step: gate == Some(DrawGate::DuringDrawStep),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn empty_library_replacement_requires_one_marker_and_an_instruction() {
        let valid = "If you would draw a card while your library has no cards in it, instead put five +1/+1 counters on this creature.";
        assert!(
            parse_draw_replacement_program(&lex_line(valid, 0).unwrap())
                .is_some_and(|shape| shape.empty_library)
        );
        let optional = "If you would draw a card while your library has no cards in it, instead, you may gain 2 life.";
        let shape = parse_draw_replacement_program(&lex_line(optional, 0).unwrap()).unwrap();
        assert!(shape.empty_library && shape.optional);
        for text in [
            "If you would draw a card while your library has no cards in it, put five +1/+1 counters on this creature.",
            "If you would draw a card while your library has no cards in it, instead instead put five +1/+1 counters on this creature.",
            "If you would draw a card while your library has no cards in it, instead put five +1/+1 counters on this creature instead.",
            "If you would draw a card while your library has no cards in it, instead.",
        ] {
            assert!(
                parse_draw_replacement_program(&lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
    }

    #[test]
    fn only_the_outer_instead_marks_the_replacement() {
        let header = "The first time you would draw a card each turn,";
        let quoted = r#"target creature gains "If you would gain life, draw a card instead. Then discard a card.""#;
        let continuation = "If you do, gain 2 life.";
        let text = format!("{header} {quoted} instead. {continuation}");
        let (_, body) = first_draw_each_turn(&lex_line(&text, 0).unwrap()).unwrap();
        let expected = format!("{quoted}. {continuation}");
        assert_eq!(
            crate::lexer::render_token_slice(&body),
            crate::lexer::render_token_slice(&lex_line(&expected, 0).unwrap())
        );
        for body in [
            format!("{quoted}. {continuation}"),
            "gain 2 life. Draw a card instead.".into(),
            "instead gain 2 life instead.".into(),
        ] {
            assert!(
                first_draw_each_turn(&lex_line(&format!("{header} {body}"), 0).unwrap()).is_none()
            );
        }
    }

    #[test]
    fn outer_instead_can_precede_or_follow_a_complete_owned_program() {
        for text in [
            "If you would draw a card, instead look at the top three cards of your library. Put one of those cards into your hand and the rest on the bottom of your library in any order.",
            "If you would draw a card, look at the top three cards of your library instead. Put one of those cards into your hand and the rest on the bottom of your library in any order.",
        ] {
            let parsed = parse_draw_replacement_program(&lex_line(text, 0).unwrap()).unwrap();
            assert!(!parsed.optional && !parsed.skip);
            assert_eq!(parsed.player, DrawReplacementPlayer::You);
            assert!(!parsed.body.iter().any(|token| token.matches_word("instead")));
            assert!(parsed.body.iter().any(|token| token.matches_word("bottom")));
        }
    }
    #[test]
    fn cancellation_is_optional_only_for_the_authored_chooser_and_current_draw() {
        let parsed = parse_draw_replacement_program(
            &lex_line(
                "If you would draw a card, you may skip that draw instead.",
                0,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(parsed.skip && parsed.optional && parsed.body.is_empty());
        let parsed = parse_draw_replacement_program(
            &lex_line(
                "If a player would draw a card, that player skips that draw instead.",
                0,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(parsed.skip && !parsed.optional);
        assert_eq!(parsed.player, DrawReplacementPlayer::Any);
        for text in [
            "If you would draw a card, instead skip that draw instead.",
            "If an opponent would draw two or more cards, instead you and that player each draw a card.",
            "If a player would draw a card, you may skip that draw instead.",
        ] {
            assert!(
                parse_draw_replacement_program(&lex_line(text, 0).unwrap()).is_none(),
                "{text}"
            );
        }
        let parsed = parse_draw_replacement_program(
            &lex_line(
                "If you would draw a card during your draw step, instead skip that draw.",
                0,
            )
            .unwrap(),
        )
        .unwrap();
        assert!(parsed.skip && parsed.during_your_draw_step && !parsed.optional);
    }
    #[test]
    fn skip_with_if_you_do_followup_replaces_the_draw_with_the_followup() {
        let parsed = parse_draw_replacement_program(&lex_line("If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk.", 0).unwrap()).unwrap();
        assert!(parsed.optional && parsed.during_your_draw_step && !parsed.skip);
        assert!(
            parsed
                .body
                .first()
                .is_some_and(|token| token.is_word("until"))
        );
        assert!(!parsed.body.iter().any(|token| token.matches_word("skip")));
    }
    #[test]
    fn empty_library_and_first_draw_exception_remain_separate_matcher_gates() {
        let parsed = parse_draw_replacement_program(&lex_line("If you would draw a card while your library has no cards in it, instead return a creature card from your graveyard to the battlefield. If you can't, you lose the game.", 0).unwrap()).unwrap();
        assert!(parsed.empty_library);
        assert!(!parsed.except_first_of_draw_step);
        let parsed = parse_draw_replacement_program(&lex_line("If a player would draw a card except the first one they draw in each of their draw steps, that player discards a card instead. If the player discards a card this way, they draw a card. If the player doesn't discard a card this way, they mill a card.", 0).unwrap()).unwrap();
        assert!(!parsed.empty_library && parsed.except_first_of_draw_step);
        assert!(parsed.body.iter().any(|token| token.matches_word("mill")));
    }
}

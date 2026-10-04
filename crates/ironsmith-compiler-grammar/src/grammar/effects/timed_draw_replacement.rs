//! Strict owners for a duration-bound replacement of a single draw proposal.
//! The following program remains inside registration, including later sentences.
use winnow::combinator::{alt, peek, repeat_till};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use winnow::token::any;
use crate::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

pub struct TimedDrawReplacement<'a> {
    pub player: &'a [OwnedLexToken],
    pub one_shot: bool,
    pub body: Vec<OwnedLexToken>,
}
fn header<'a>(input: &mut LexStream<'a>) -> WResult<(&'a [OwnedLexToken], bool)> {
    let one_shot = alt((
        primitives::phrase(&["the", "next", "time"]).value(true),
        (primitives::phrase(&["until", "end", "of", "turn"]), primitives::comma(), primitives::kw("if")).value(false),
    )).parse_next(input)?;
    let player = repeat_till::<_, _, (), _, _, _, _>(1.., any.void(), peek(primitives::phrase(&["would", "draw", "a", "card"])))
        .map(|((), _)| ()).take().parse_next(input)?;
    primitives::phrase(&["would", "draw", "a", "card"]).parse_next(input)?;
    if one_shot { primitives::phrase(&["this", "turn"]).parse_next(input)?; }
    primitives::comma().parse_next(input)?;
    Ok((player, one_shot))
}
pub fn parse_timed_draw_replacement(tokens: &[OwnedLexToken]) -> Option<TimedDrawReplacement<'_>> {
    let ((player, one_shot), body) = primitives::parse_prefix(tokens, header)?;
    let (leading, body) = if let Some((_, rest)) = primitives::parse_prefix(body, primitives::kw("instead")) {
        (true, rest)
    } else { (false, body) };
    let mut quoted = false;
    let end = body.iter().position(|token| {
        if token.is_quote() { quoted = !quoted; }
        !quoted && token.is_period()
    }).unwrap_or(body.len());
    let trailing = end > 0 && body[end - 1].is_word("instead");
    if leading == trailing { return None; }
    let mut body = body.to_vec();
    if trailing { body.remove(end - 1); }
    // Skipping this proposed draw is the empty original of the replacement;
    // it does not create an additional skipped future draw step or draw card.
    if let Some((_, rest)) = primitives::parse_prefix(&body,
        primitives::phrase(&["that", "player", "skips", "that", "draw", "and"])) {
        body = rest.to_vec();
    }
    if body.is_empty() || primitives::parse_prefix(&body, primitives::phrase(&["you", "may"])).is_some()
        || primitives::parse_prefix(&body, primitives::phrase(&["they", "may"])).is_some() { return None; }
    Some(TimedDrawReplacement { player, one_shot, body })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{lex_line, parser_token_word_refs};
    #[test]
    fn strict_headers_keep_next_time_and_whole_turn_distinct() {
        let tokens = lex_line("The next time you would draw a card this turn, this enchantment deals 2 damage to any target instead.", 0).unwrap();
        let shape = parse_timed_draw_replacement(&tokens).unwrap(); assert!(shape.one_shot); assert_eq!(parser_token_word_refs(shape.player), ["you"]);
        let tokens = lex_line("Until end of turn, if target player would draw a card, instead that player skips that draw and you draw a card.", 0).unwrap();
        let shape = parse_timed_draw_replacement(&tokens).unwrap(); assert!(!shape.one_shot); assert_eq!(parser_token_word_refs(&shape.body), ["you", "draw", "a", "card"]);
    }
    #[test]
    fn complete_later_permission_belongs_inside_the_replacement_program() {
        let tokens = lex_line("The next time they would draw a card this turn, instead they exile the top card of their library. They may play it this turn.", 0).unwrap();
        let shape = parse_timed_draw_replacement(&tokens).unwrap(); assert!(shape.body.iter().any(|token| token.is_word("play")));
        assert_eq!(parser_token_word_refs(shape.player), ["they"]);
    }
    #[test]
    fn batch_first_draw_exception_optional_and_missing_duration_are_not_broadened() {
        for text in [
            "If an opponent would draw two or more cards, instead you and that player each draw a card.",
            "The next time you would draw a card, instead gain 5 life.",
            "The first time you would draw a card each turn, instead draw four cards.",
            "The next time you would draw a card this turn, you may gain 5 life instead.",
            "The next time you would draw a card this turn, instead gain 5 life instead.",
        ] { assert!(parse_timed_draw_replacement(&lex_line(text, 0).unwrap()).is_none(), "{text}"); }
    }
}

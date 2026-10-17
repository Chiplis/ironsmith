//! A draw replacement owns its complete following program, including result
//! continuations. Only the outer instead marker is removed from that program.
use winnow::combinator::{alt, opt};
use winnow::error::ModalResult as WResult;
use winnow::prelude::*;
use crate::lexer::{LexStream, OwnedLexToken};
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawReplacementPlayer { You, Opponent, Any }
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
/// "if a cycling ability of <object> would cause you to draw a card ,"
fn cycling_cause_header(tokens: &[OwnedLexToken]) -> Option<(Vec<OwnedLexToken>, &[OwnedLexToken])> {
    const PREFIX: [&str; 5] = ["if", "a", "cycling", "ability", "of"];
    const TAIL: [&str; 7] = ["would", "cause", "you", "to", "draw", "a", "card"];
    if tokens.len() < PREFIX.len() || !tokens.iter().zip(PREFIX).all(|(token, word)| token.is_word(word)) {
        return None;
    }
    let rest = &tokens[PREFIX.len()..];
    let at = (1..rest.len()).find(|&idx| {
        rest.len() >= idx + TAIL.len()
            && rest[idx..idx + TAIL.len()].iter().zip(TAIL).all(|(token, word)| token.is_word(word))
    })?;
    let source = rest[..at].to_vec();
    let after = &rest[at + TAIL.len()..];
    if !after.first()?.is_comma() { return None; }
    Some((source, &after[1..]))
}
fn header(input: &mut LexStream<'_>) -> WResult<(DrawReplacementPlayer, bool, bool, bool)> {
    primitives::kw("if").parse_next(input)?;
    let player = alt((primitives::kw("you").value(DrawReplacementPlayer::You),
        primitives::phrase(&["an", "opponent"]).value(DrawReplacementPlayer::Opponent),
        primitives::phrase(&["a", "player"]).value(DrawReplacementPlayer::Any))).parse_next(input)?;
    primitives::phrase(&["would", "draw", "a", "card"]).parse_next(input)?;
    let except_first = if player == DrawReplacementPlayer::You {
        opt(primitives::phrase(&["except", "the", "first", "one", "you", "draw", "in", "each", "of", "your", "draw", "steps"])).parse_next(input)?.is_some()
    } else {
        opt(primitives::phrase(&["except", "the", "first", "one", "they", "draw", "in", "each", "of", "their", "draw", "steps"])).parse_next(input)?.is_some()
    };
    let empty_library = if player == DrawReplacementPlayer::You {
        opt(primitives::phrase(&["while", "your", "library", "has", "no", "cards", "in", "it"])).parse_next(input)?.is_some()
    } else { false };
    let during_draw_step = if player == DrawReplacementPlayer::You && !except_first && !empty_library {
        opt(primitives::phrase(&["during", "your", "draw", "step"])).parse_next(input)?.is_some()
    } else { false };
    primitives::comma().parse_next(input)?;
    Ok((player, except_first, empty_library, during_draw_step))
}
pub fn parse_draw_replacement_program(tokens: &[OwnedLexToken]) -> Option<DrawReplacementProgram> {
    let ((player, except_first_of_draw_step, empty_library, during_your_draw_step), mut body, cycling_source) =
        if let Some((source, rest)) = cycling_cause_header(tokens) {
            ((DrawReplacementPlayer::You, false, false, false), rest, Some(source))
        } else {
            let (header, body) = primitives::parse_prefix(tokens, header)?;
            (header, body, None)
        };
    let mut optional = if let Some(((), rest)) = primitives::parse_prefix(body, primitives::phrase(&["you", "may"])) {
        body = rest; true
    } else { false };
    let leading = if let Some((_, rest)) = primitives::parse_prefix(body, primitives::kw("instead")) {
        body = rest; true
    } else { false };
    if let Some(((), rest)) = primitives::parse_prefix(body, primitives::phrase(&["you", "may"])) {
        if optional { return None; }
        optional = true; body = rest;
    }
    // A different optional chooser needs a scoped optional replacement, not
    // an always-replaced draw containing a possibly declined inner effect.
    if [["they", "may"].as_slice(), ["that", "player", "may"].as_slice()]
        .into_iter().any(|words| body.iter().zip(words).all(|(token, word)| token.is_word(word)) && body.len() >= words.len())
    { return None; }
    // Existing optional/conditional executable payloads are scoped to you.
    // Do not erase a first-draw exception or a different optional chooser.
    if (optional || empty_library) && (player != DrawReplacementPlayer::You || except_first_of_draw_step) { return None; }
    let mut quoted = false;
    let sentence_end = body.iter().position(|token| {
        if token.is_quote() { quoted = !quoted; }
        !quoted && token.is_period()
    }).unwrap_or(body.len());
    let trailing = sentence_end > 0 && body[sentence_end - 1].is_word("instead");
    if leading == trailing { return None; }
    let mut body = body.to_vec();
    if trailing { body.remove(sentence_end - 1); }
    const SKIP_PHRASES: [&[&str]; 3] = [
        &["skip", "that", "draw"], &["you", "skip", "that", "draw"],
        &["that", "player", "skips", "that", "draw"],
    ];
    // "instead you may skip that draw. If you do, <effects>" (Island
    // Sanctuary): skipping the proposed draw and running the follow-up is
    // exactly replacing that draw with the follow-up (CR 614.1a), so the
    // follow-up becomes the replacement program.
    let skip_then_if_you_do = SKIP_PHRASES.iter().find_map(|words| {
        let after = body.get(words.len()..)?;
        (body.iter().zip(words.iter()).all(|(token, word)| token.is_word(word))
            && after.first()?.is_period()
            && after.len() > 4
            && after[1].is_word("if") && after[2].is_word("you") && after[3].is_word("do")
            && after[4].is_comma())
        .then(|| after[5..].to_vec())
    });
    if let Some(followup) = skip_then_if_you_do {
        if followup.is_empty() { return None; }
        return Some(DrawReplacementProgram { player, except_first_of_draw_step, empty_library, optional, skip: false, body: followup, cycling_source, during_your_draw_step });
    }
    let skip = SKIP_PHRASES.into_iter().any(|words| primitives::probe_all(&body,
        (primitives::phrase(words), primitives::sentence_end()).void(), "skip this proposed draw").is_some());
    // A cancellation has no future skipped draw. Its empty replacement program
    // consumes just this proposal under the ordinary replacement identity.
    if skip { body.clear(); }
    if body.is_empty() && !skip { return None; }
    Some(DrawReplacementProgram { player, except_first_of_draw_step, empty_library, optional, skip, body, cycling_source, during_your_draw_step })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    #[test]
    fn outer_instead_can_precede_or_follow_a_complete_owned_program() {
        for text in [
            "If you would draw a card, instead look at the top three cards of your library. Put one of those cards into your hand and the rest on the bottom of your library in any order.",
            "If you would draw a card, look at the top three cards of your library instead. Put one of those cards into your hand and the rest on the bottom of your library in any order.",
        ] {
            let parsed = parse_draw_replacement_program(&lex_line(text, 0).unwrap()).unwrap();
            assert!(!parsed.optional && !parsed.skip); assert_eq!(parsed.player, DrawReplacementPlayer::You);
            assert!(!parsed.body.iter().any(|token| token.is_word("instead")));
            assert!(parsed.body.iter().any(|token| token.is_word("bottom")));
        }
    }
    #[test]
    fn cancellation_is_optional_only_for_the_authored_chooser_and_current_draw() {
        let parsed = parse_draw_replacement_program(&lex_line("If you would draw a card, you may skip that draw instead.", 0).unwrap()).unwrap();
        assert!(parsed.skip && parsed.optional && parsed.body.is_empty());
        let parsed = parse_draw_replacement_program(&lex_line("If a player would draw a card, that player skips that draw instead.", 0).unwrap()).unwrap();
        assert!(parsed.skip && !parsed.optional); assert_eq!(parsed.player, DrawReplacementPlayer::Any);
        for text in [
            "If you would draw a card, instead skip that draw instead.",
            "If an opponent would draw two or more cards, instead you and that player each draw a card.",
            "If a player would draw a card, you may skip that draw instead.",
        ] { assert!(parse_draw_replacement_program(&lex_line(text, 0).unwrap()).is_none(), "{text}"); }
        let parsed = parse_draw_replacement_program(&lex_line("If you would draw a card during your draw step, instead skip that draw.", 0).unwrap()).unwrap();
        assert!(parsed.skip && parsed.during_your_draw_step && !parsed.optional);
    }
    #[test]
    fn skip_with_if_you_do_followup_replaces_the_draw_with_the_followup() {
        let parsed = parse_draw_replacement_program(&lex_line("If you would draw a card during your draw step, instead you may skip that draw. If you do, until your next turn, you can't be attacked except by creatures with flying and/or islandwalk.", 0).unwrap()).unwrap();
        assert!(parsed.optional && parsed.during_your_draw_step && !parsed.skip);
        assert!(parsed.body.first().is_some_and(|token| token.is_word("until")));
        assert!(!parsed.body.iter().any(|token| token.is_word("skip")));
    }
    #[test]
    fn empty_library_and_first_draw_exception_remain_separate_matcher_gates() {
        let parsed = parse_draw_replacement_program(&lex_line("If you would draw a card while your library has no cards in it, instead return a creature card from your graveyard to the battlefield. If you can't, you lose the game.", 0).unwrap()).unwrap();
        assert!(parsed.empty_library); assert!(!parsed.except_first_of_draw_step);
        let parsed = parse_draw_replacement_program(&lex_line("If a player would draw a card except the first one they draw in each of their draw steps, that player discards a card instead. If the player discards a card this way, they draw a card. If the player doesn't discard a card this way, they mill a card.", 0).unwrap()).unwrap();
        assert!(!parsed.empty_library && parsed.except_first_of_draw_step);
        assert!(parsed.body.iter().any(|token| token.is_word("mill")));
    }
}

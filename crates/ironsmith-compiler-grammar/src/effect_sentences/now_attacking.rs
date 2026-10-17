//! "<attacking creatures> are now attacking <that player | you>"
//! (Portal Manipulator): each named creature keeps attacking but now attacks
//! the stated player, with no choice (CR 506.4). A creature whose controller
//! couldn't attack that player keeps its attack.
use winnow::combinator::alt;
use winnow::prelude::*;

use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::grammar::primitives;
use crate::lexer::{LexStream, OwnedLexToken};

fn now_attacking_player(input: &mut LexStream<'_>) -> winnow::error::ModalResult<PlayerAst> {
    primitives::phrase(&["now", "attacking"]).parse_next(input)?;
    let player = alt((
        primitives::phrase(&["that", "player"]).value(PlayerAst::That),
        primitives::kw("you").value(PlayerAst::You),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(player)
}

/// "attacking <that player | you>" with no "now": the creature becomes an
/// attacking creature (CR 506.3d) rather than having an attack redirected.
fn attacking_player(input: &mut LexStream<'_>) -> winnow::error::ModalResult<PlayerAst> {
    primitives::kw("attacking").parse_next(input)?;
    let player = alt((
        primitives::phrase(&["that", "player"]).value(PlayerAst::That),
        primitives::kw("you").value(PlayerAst::You),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(player)
}

/// "attacking a player or planeswalker attacked by that opponent" (the
/// preprocessed Tahngarth, First Mate pair): a choice among what that player
/// is attacking.
fn attacking_one_attacked_by(input: &mut LexStream<'_>) -> winnow::error::ModalResult<PlayerAst> {
    primitives::kw("attacking").parse_next(input)?;
    primitives::phrase(&["a", "player", "or", "planeswalker", "attacked", "by"]).parse_next(input)?;
    let player = alt((
        primitives::phrase(&["that", "player"]).value(PlayerAst::That),
        primitives::phrase(&["that", "opponent"]).value(PlayerAst::That),
    ))
    .parse_next(input)?;
    primitives::sentence_end().parse_next(input)?;
    Ok(player)
}

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::lexer::trim_lexed_commas(tokens);
    // "tap it, and it's attacking that player" (Midnight Crusader Shuttle).
    if let Some(first) = tokens.first()
        && (first.is_word("it's") || first.is_word("its"))
        && let Some(player) =
            primitives::probe_all(&tokens[1..], attacking_player, "attacking-player")
    {
        let it = [OwnedLexToken::word("it".to_string(), crate::cards::builders::TextSpan::synthetic())];
        let target = crate::util::parse_target_phrase(&it)?;
        return Ok(Some(EffectAst::subject_verb_attacking_player(target, player)));
    }
    let Some((copula, _, rest)) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("is"), primitives::kw("are"))).void()
    }) else {
        return Ok(None);
    };
    if copula == 0 {
        return Ok(None);
    }
    if let Some(player) =
        primitives::probe_all(rest, attacking_one_attacked_by, "attacking-one-attacked-by")
    {
        let target = crate::util::parse_target_phrase(&tokens[..copula])?;
        return Ok(Some(EffectAst::subject_verb_attacking_one_attacked_by(target, player)));
    }
    if let Some(player) = primitives::probe_all(rest, attacking_player, "attacking-player") {
        let target = crate::util::parse_target_phrase(&tokens[..copula])?;
        return Ok(Some(EffectAst::subject_verb_attacking_player(target, player)));
    }
    let Some(player) = primitives::probe_all(rest, now_attacking_player, "now-attacking-player")
    else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(&tokens[..copula])?;
    Ok(Some(EffectAst::subject_verb_now_attacking_player(target, player)))
}

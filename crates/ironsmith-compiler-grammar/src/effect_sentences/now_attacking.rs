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

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::lexer::trim_lexed_commas(tokens);
    let Some((copula, _, rest)) = primitives::find_prefix(tokens, || {
        alt((primitives::kw("is"), primitives::kw("are"))).void()
    }) else {
        return Ok(None);
    };
    if copula == 0 {
        return Ok(None);
    }
    let Some(player) = primitives::probe_all(rest, now_attacking_player, "now-attacking-player")
    else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(&tokens[..copula])?;
    Ok(Some(EffectAst::subject_verb_now_attacking_player(target, player)))
}

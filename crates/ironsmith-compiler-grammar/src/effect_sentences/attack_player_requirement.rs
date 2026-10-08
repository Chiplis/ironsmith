//! "Target creature [an opponent controls] attacks <you | target opponent |
//! target player> this turn if able" (CR 508.1d): a requirement to attack
//! one specific player this turn.
use crate::cards::builders::{
    CardTextError, EffectAst, KeywordActionAst, PlayerAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TargetAst,
};
use crate::lexer::OwnedLexToken;
use crate::target::PlayerFilter;

pub(crate) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("target")) {
        return Ok(None);
    }
    let Some(attacks) = tokens.iter().position(|token| token.is_word("attacks")) else {
        return Ok(None);
    };
    let tail = &tokens[attacks + 1..];
    let tail_words = crate::lexer::parser_token_word_refs(tail);
    // "target creature an opponent controls attacks during its controller's
    // next combat phase if able" (Trench Behemoth): no player is named; the
    // requirement waits for that creature's controller's next combat.
    if matches!(
        tail_words.as_slice(),
        ["during", "its", "controllers" | "controller's" | "controller’s", "next", "combat", "phase", "if", "able"]
    ) && attacks > 0
    {
        let target = crate::util::parse_target_phrase(&tokens[..attacks])?;
        if !matches!(target, TargetAst::Object(..)) {
            return Ok(None);
        }
        return Ok(Some(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::KeywordActions(KeywordActionAst::MustAttackPlayerThisTurn {
                target,
                player: TargetAst::Player(PlayerFilter::Any, None),
                controllers_next_combat: true,
            }),
        )));
    }
    let Some(player_words) = tail_words.strip_suffix(&["this", "turn", "if", "able"][..]) else {
        return Ok(None);
    };
    if player_words.is_empty() || attacks == 0 {
        return Ok(None);
    }
    let player_tokens = &tail[..tail.len() - 4];
    let player = if player_words == ["you"] {
        TargetAst::Player(PlayerFilter::You, None)
    } else if matches!(player_words, ["target", "opponent" | "player"]) {
        match crate::util::parse_target_phrase(player_tokens)? {
            target @ TargetAst::Player(..) => target,
            _ => return Ok(None),
        }
    } else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(&tokens[..attacks])?;
    if !matches!(target, TargetAst::Object(..)) {
        return Ok(None);
    }
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::KeywordActions(KeywordActionAst::MustAttackPlayerThisTurn {
            target,
            player,
            controllers_next_combat: false,
        }),
    )))
}

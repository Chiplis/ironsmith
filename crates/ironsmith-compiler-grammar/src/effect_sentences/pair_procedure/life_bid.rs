//! A life auction for control of a target (Illicit Auction) is one procedure
//! of five sentences: the bid, the opening bid, the rounds in turn order, the
//! end of the bidding, and the high bidder's payment and reward. Read apart,
//! the later sentences have no action of their own.
use super::*;
use crate::cards::builders::VoteEffectAst;

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(window) = sentences.get(sentence_idx..sentence_idx + 5) else {
        return Ok(None);
    };
    let lowered = window
        .iter()
        .map(SentenceInput::lowered)
        .collect::<Vec<_>>();
    if let Some(shape) = crate::grammar::effects::parse_untargeted_life_bid_sentences(&lowered) {
        // "The high bidder loses life equal to the high bid and draws four
        // cards" (Pain's Reward): the reward is the high bidder's own
        // action, executed with that player bound as the actor.
        let mut reward = shape.reward.to_vec();
        if let Some(first) = reward.first_mut()
            && let Some(word) = first.as_word()
            && word.len() > 2
            && word.ends_with('s')
            && !word.ends_with("ss")
        {
            let stem = word[..word.len() - 1].to_string();
            *first = crate::lexer::OwnedLexToken::word(stem, first.span.clone());
        }
        let winner_effects = crate::effect_sentences::chain_carry::parse_effect_chain(&reward)?;
        if winner_effects.is_empty() {
            return Ok(None);
        }
        return Ok(Some(vec![EffectAst::Votes(VoteEffectAst::BidLife {
            target: None,
            starting_bid: shape.starting_bid,
            winner_effects,
            opponent: None,
            controller_must_win: false,
        })]));
    }
    let Some(shape) = crate::grammar::effects::parse_life_bid_sentences(&lowered) else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(shape.target)?;
    Ok(Some(vec![EffectAst::Votes(VoteEffectAst::BidLife {
        target: Some(target.clone()),
        starting_bid: Some(0),
        opponent: None,
        controller_must_win: false,
        winner_effects: vec![EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            target,
            crate::effect::Until::Forever,
        )],
    })]))
}

/// "You and target spell's controller bid life. ... If you win the bidding,
/// counter that spell." (Mages' Contest): the object whose controller bids is
/// declared first, so the reward's "that spell" names it.
pub(super) fn read_two_player(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(window) = sentences.get(sentence_idx..sentence_idx + 6) else {
        return Ok(None);
    };
    let lowered = window
        .iter()
        .map(SentenceInput::lowered)
        .collect::<Vec<_>>();
    let Some(shape) = crate::grammar::effects::parse_two_player_life_bid_sentences(&lowered) else {
        return Ok(None);
    };
    // "<object>'s controller": the bidder is the controller of a declared
    // object target.
    let Some((controller, object)) = shape.opponent.split_last() else {
        return Ok(None);
    };
    if !controller.is_word("controller") {
        return Ok(None);
    }
    let Some((possessed, head)) = object.split_last() else {
        return Ok(None);
    };
    let Some(word) = possessed.as_word() else {
        return Ok(None);
    };
    // "spell's" (or its normalized "spells") before "controller".
    let stem = match crate::util::strip_possessive_suffix(word) {
        stripped if stripped != word => stripped,
        _ => match word.strip_suffix('s') {
            Some(stem) if !stem.is_empty() => stem,
            _ => return Ok(None),
        },
    };
    let mut object_tokens = head.to_vec();
    object_tokens.push(crate::lexer::OwnedLexToken::word(
        stem.to_string(),
        possessed.span.clone(),
    ));
    let Ok(target) = crate::util::parse_target_phrase(&object_tokens) else {
        return Ok(None);
    };
    if !matches!(target, TargetAst::Object(_, Some(_), _) | TargetAst::Spell(Some(_))) {
        return Ok(None);
    }
    let winner_effects = crate::effect_sentences::chain_carry::parse_effect_chain(shape.reward)?;
    if winner_effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![
        EffectAst::subject_verb_explicit_target_only(target),
        EffectAst::Votes(VoteEffectAst::BidLife {
            target: None,
            starting_bid: shape.starting_bid,
            winner_effects,
            opponent: Some(PlayerAst::ItsController),
            controller_must_win: true,
        }),
    ]))
}

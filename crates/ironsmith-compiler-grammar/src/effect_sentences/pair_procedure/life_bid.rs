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
        })]));
    }
    let Some(shape) = crate::grammar::effects::parse_life_bid_sentences(&lowered) else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(shape.target)?;
    Ok(Some(vec![EffectAst::Votes(VoteEffectAst::BidLife {
        target: Some(target.clone()),
        starting_bid: Some(0),
        winner_effects: vec![EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            target,
            crate::effect::Until::Forever,
        )],
    })]))
}

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
    let Some(shape) = crate::grammar::effects::parse_life_bid_sentences(&lowered) else {
        return Ok(None);
    };
    let target = crate::util::parse_target_phrase(shape.target)?;
    Ok(Some(vec![EffectAst::Votes(VoteEffectAst::BidLife {
        target: target.clone(),
        starting_bid: 0,
        winner_effects: vec![EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            target,
            crate::effect::Until::Forever,
        )],
    })]))
}

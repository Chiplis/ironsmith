//! A named vote followed by an instruction over every winning option.
//!
//! "starting with you, each player votes for blue, black, red, or green.
//! This creature gains protection from each color with the most votes or
//! tied for most votes." (Council Guardian)
//!
//! The follow-up quantifies over the vote's named options: it applies once to
//! each option that got the most votes or tied for the most (CR 701.38).
//! Each application is the instruction with that option named in place of
//! the quantifier, gated by the existing "<option> gets more votes or ties"
//! vote predicate, so the vote sequence lowering binds it to this vote's
//! tally exactly as it binds an authored "If <option> gets more votes or the
//! vote is tied" sentence.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, PredicateAst, TextSpan, VoteEffectAst,
};
use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;

pub(super) struct VoteOptionSetGroup {
    effects: Vec<EffectAst>,
    closed: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

const VOTE_WINNER_SUFFIX: &[&str] = &[
    "with", "the", "most", "votes", "or", "tied", "for", "most", "votes",
];

fn named_vote_options(effects: &[EffectAst]) -> Option<Vec<String>> {
    effects.iter().find_map(|effect| match effect {
        EffectAst::Votes(VoteEffectAst::VoteStart { options, .. }) => Some(options.clone()),
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. } => named_vote_options(effects),
        _ => None,
    })
}

/// "<prefix> each <noun> with the most votes or tied for most votes": the
/// instruction's tokens before the quantifier.
fn winner_quantified_prefix(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    // each <noun> <suffix>
    let quantifier_len = 2 + VOTE_WINNER_SUFFIX.len();
    let start = tokens.len().checked_sub(quantifier_len)?;
    let (prefix, quantifier) = tokens.split_at(start);
    let (_, rest) = primitives::parse_prefix(quantifier, primitives::kw("each"))?;
    let (_, rest) = rest.split_first()?;
    let (_, rest) = primitives::parse_prefix(rest, primitives::phrase(VOTE_WINNER_SUFFIX))?;
    (rest.is_empty() && !prefix.is_empty()).then_some(prefix)
}

fn option_instruction(
    prefix: &[OwnedLexToken],
    option: &str,
) -> Result<EffectAst, CardTextError> {
    let mut tokens = prefix.to_vec();
    tokens.push(OwnedLexToken::word(option, TextSpan::synthetic()));
    tokens.push(OwnedLexToken::period(TextSpan::synthetic()));
    let if_true = super::parse_effect_sentence_lexed(&tokens)?;
    Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::VoteOptionGetsMoreVotesOrTied {
            option: option.to_string(),
        },
        if_true,
        if_false: Vec::new(),
    }))
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<VoteOptionSetGroup>, CardTextError> {
    let (Some(vote), Some(next)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    if winner_quantified_prefix(next.lowered()).is_none() {
        return Ok(None);
    }
    let Ok(effects) = super::parse_effect_sentence_lexed(vote.lowered()) else {
        return Ok(None);
    };
    if named_vote_options(&effects).is_none() {
        return Ok(None);
    }
    Ok(Some(VoteOptionSetGroup {
        effects,
        closed: false,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    group: &mut VoteOptionSetGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.closed {
        return Ok(false);
    }
    let Some(prefix) = winner_quantified_prefix(sentence.lowered()) else {
        return Ok(false);
    };
    let Some(options) = named_vote_options(&group.effects) else {
        return Ok(false);
    };
    for option in &options {
        let instruction = option_instruction(prefix, option)?;
        group.effects.push(instruction);
    }
    group.closed = true;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: VoteOptionSetGroup) -> Vec<EffectAst> {
    group.effects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winner_quantifier_leaves_the_instruction_prefix() {
        let tokens = crate::lexer::lex_line(
            "this creature gains protection from each color with the most votes or tied for most votes.",
            0,
        )
        .unwrap();
        let prefix = winner_quantified_prefix(&tokens).expect("quantified instruction");
        assert_eq!(
            crate::lexer::token_word_refs(prefix),
            vec!["this", "creature", "gains", "protection", "from"]
        );
    }
}

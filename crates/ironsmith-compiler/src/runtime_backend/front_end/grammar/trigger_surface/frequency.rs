use super::super::super::lexer::OwnedLexToken;
use super::super::primitives;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BecomesTappedDuringYourTurn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerFrequencySurface {
    pub(crate) first_time_each_or_this_turn: bool,
    pub(crate) becomes_crewed: bool,
    pub(crate) do_this_limit_each_turn: Option<u32>,
}

fn has_phrase(tokens: &[OwnedLexToken], phrase: &'static [&'static str]) -> bool {
    primitives::find_prefix(tokens, || primitives::phrase(phrase)).is_some()
}

pub(crate) fn parse_becomes_tapped_during_your_turn_tokens(
    tokens: &[OwnedLexToken],
) -> Option<BecomesTappedDuringYourTurn> {
    has_phrase(tokens, &["becomes", "tapped", "during", "your", "turn"])
        .then_some(BecomesTappedDuringYourTurn)
}

pub(crate) fn parse_do_this_only_each_turn_limit_tokens(tokens: &[OwnedLexToken]) -> Option<u32> {
    if has_phrase(tokens, &["do", "this", "only", "once", "each", "turn"]) {
        Some(1)
    } else if has_phrase(tokens, &["do", "this", "only", "twice", "each", "turn"]) {
        Some(2)
    } else {
        None
    }
}

pub(crate) fn parse_trigger_frequency_tokens(tokens: &[OwnedLexToken]) -> TriggerFrequencySurface {
    TriggerFrequencySurface {
        first_time_each_or_this_turn: has_phrase(
            tokens,
            &["for", "the", "first", "time", "each", "turn"],
        ) || has_phrase(
            tokens,
            &["for", "the", "first", "time", "this", "turn"],
        ),
        becomes_crewed: has_phrase(tokens, &["becomes", "crewed"]),
        do_this_limit_each_turn: parse_do_this_only_each_turn_limit_tokens(tokens),
    }
}

pub(crate) fn parse_trigger_frequency_condition_tokens(
    tokens: &[OwnedLexToken],
    max_triggers_per_turn: Option<u32>,
) -> Option<crate::ConditionExpr> {
    max_triggers_per_turn.map(|limit| {
        let frequency = parse_trigger_frequency_tokens(tokens);
        if limit == 1 && frequency.first_time_each_or_this_turn && frequency.becomes_crewed {
            crate::ConditionExpr::SourceFirstCrewedThisTurn
        } else if limit == 1 && frequency.first_time_each_or_this_turn {
            crate::ConditionExpr::FirstTimeThisTurn
        } else if frequency.do_this_limit_each_turn.is_some() {
            crate::ConditionExpr::DoThisMaxTimesEachTurn(limit)
        } else {
            crate::ConditionExpr::MaxTimesEachTurn(limit)
        }
    })
}

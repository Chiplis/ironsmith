//! A multi-sentence procedure in progress in the sentence loop.
//!
//! The loop parses one sentence at a time. Some sentences bind a group of
//! cards that the next sentences refer to — the cards looked at, the cards a
//! traversal revealed — and those sentences form a procedure: consecutive
//! statements over one group. The loop keeps at most one procedure open; the
//! kinds live in their own modules, and this one only says which is open.

use super::dispatch_entry::SentenceInput;
use super::{
    consult_procedure, copy_cast_procedure, exiled_top_procedure, graveyard_cast_procedure,
    assign_unblocked_procedure, extra_targets_procedure, each_reveal_procedure, chosen_hand_reveal_procedure, tiered_payment_procedure, gained_control_followup_procedure, exchange_blocks_procedure, hand_procedure, keyword_choice_procedure, player_option_choice_procedure, vote_option_set_procedure, chosen_counter_kind_procedure, repeat_choice_procedure, distinct_player_assignment_procedure, looked_procedure, search_partition_procedure, mill_procedure, pair_procedure, rider_procedure,
};
use crate::cards::builders::{CardTextError, EffectAst};

/// The name a statement with its riders reports; the document composers read
/// it, since such a statement opening a document makes the document one block.
pub const RIDDEN_STATEMENT: &str = "ridden-statement";

pub(super) enum Procedure {
    Looked(looked_procedure::ViewedGroup),
    Consulted(consult_procedure::ConsultedGroup),
    Milled(mill_procedure::MilledGroup),
    GraveyardCast(graveyard_cast_procedure::GraveyardCastGroup),
    CopyCast(copy_cast_procedure::CopyCastGroup),
    ExiledTop(exiled_top_procedure::ExiledTopGroup),
    Pair(pair_procedure::PairGroup),
    Ridden(rider_procedure::RiddenStatement),
    Hand(hand_procedure::HandGroup),
    KeywordChoice(keyword_choice_procedure::KeywordChoiceGroup),
    SearchPartition(search_partition_procedure::SearchPartitionGroup),
    AssignUnblocked(assign_unblocked_procedure::AssignUnblockedGroup),
    ExtraTargets(extra_targets_procedure::ExtraTargetsGroup),
    EachReveal(each_reveal_procedure::EachRevealGroup),
    ChosenHandReveal(chosen_hand_reveal_procedure::ChosenHandRevealGroup),
    TieredPayment(tiered_payment_procedure::TieredPaymentGroup),
    GainedControlFollowup(gained_control_followup_procedure::GainedControlFollowupGroup),
    ExchangeBlocks(exchange_blocks_procedure::ExchangeBlocksGroup),
    PlayerOptionChoice(player_option_choice_procedure::PlayerOptionChoiceGroup),
    VoteOptionSet(vote_option_set_procedure::VoteOptionSetGroup),
    ChosenCounterKind(chosen_counter_kind_procedure::ChosenCounterKindGroup),
    RepeatChoice(repeat_choice_procedure::RepeatChoiceGroup),
    DistinctPlayerAssignment(distinct_player_assignment_procedure::DistinctPlayerAssignmentGroup),
}

/// A closed procedure: its effects and the sentences it consumed.
pub(super) struct Closed {
    pub(super) effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

/// Every procedure that opens at this sentence, in rank order. A family's
/// error is set aside and stands only when no procedure opens: the registry
/// ran every rule and raised a committed error only when none matched.
fn open_all(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Vec<Procedure>, CardTextError> {
    let mut opened = Vec::new();
    let mut deferred: Option<CardTextError> = None;
    let mut consider = |opening: Result<Option<Procedure>, CardTextError>| match opening {
        Ok(Some(procedure)) => opened.push(procedure),
        Ok(None) => {}
        Err(error) => {
            deferred.get_or_insert(error);
        }
    };
    // Fixed-shape statements were ranked ahead of every other program.
    consider(pair_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Pair)));
    consider(
        search_partition_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::SearchPartition)),
    );
    consider(Ok(
        looked_procedure::open(sentences, sentence_idx).map(Procedure::Looked)
    ));
    consider(
        consult_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::Consulted)),
    );
    consider(
        mill_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Milled)),
    );
    consider(
        graveyard_cast_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::GraveyardCast)),
    );
    consider(
        copy_cast_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::CopyCast)),
    );
    consider(
        exiled_top_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::ExiledTop)),
    );
    consider(
        rider_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Ridden)),
    );
    consider(hand_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Hand)));
    consider(
        assign_unblocked_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::AssignUnblocked)),
    );
    consider(
        extra_targets_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::ExtraTargets)),
    );
    consider(
        each_reveal_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::EachReveal)),
    );
    consider(
        chosen_hand_reveal_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::ChosenHandReveal)),
    );
    consider(
        tiered_payment_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::TieredPayment)),
    );
    consider(
        gained_control_followup_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::GainedControlFollowup)),
    );
    consider(
        exchange_blocks_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::ExchangeBlocks)),
    );
    consider(
        keyword_choice_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::KeywordChoice)),
    );
    consider(
        player_option_choice_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::PlayerOptionChoice)),
    );
    consider(
        vote_option_set_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::VoteOptionSet)),
    );
    consider(
        chosen_counter_kind_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::ChosenCounterKind)),
    );
    consider(
        repeat_choice_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::RepeatChoice)),
    );
    consider(
        distinct_player_assignment_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::DistinctPlayerAssignment)),
    );
    match (opened.is_empty(), deferred) {
        (true, Some(error)) => Err(error),
        _ => Ok(opened),
    }
}

/// Continue the open procedure with this sentence; false when the sentence is
/// not one of its statements.
pub(super) fn continue_with(
    procedure: &mut Procedure,
    sentence: &SentenceInput,
    rest: &[SentenceInput],
) -> Result<bool, CardTextError> {
    let following = rest.first();
    match procedure {
        Procedure::Looked(group) => looked_procedure::continue_with(group, sentence, rest),
        Procedure::Consulted(group) => consult_procedure::continue_with(group, sentence),
        Procedure::Milled(group) => mill_procedure::continue_with(group, sentence, following),
        Procedure::GraveyardCast(group) => graveyard_cast_procedure::continue_with(group, sentence),
        Procedure::CopyCast(group) => copy_cast_procedure::continue_with(group, sentence),
        Procedure::ExiledTop(group) => exiled_top_procedure::continue_with(group, sentence),
        Procedure::Pair(group) => pair_procedure::continue_with(group, sentence),
        Procedure::Ridden(group) => rider_procedure::continue_with(group, sentence),
        Procedure::Hand(group) => hand_procedure::continue_with(group, sentence),
        Procedure::KeywordChoice(group) => keyword_choice_procedure::continue_with(group, sentence),
        Procedure::SearchPartition(group) => search_partition_procedure::continue_with(group, sentence),
        Procedure::AssignUnblocked(group) => assign_unblocked_procedure::continue_with(group, sentence),
        Procedure::ExtraTargets(group) => extra_targets_procedure::continue_with(group, sentence),
        Procedure::EachReveal(group) => each_reveal_procedure::continue_with(group, sentence),
        Procedure::ChosenHandReveal(group) => {
            chosen_hand_reveal_procedure::continue_with(group, sentence)
        }
        Procedure::TieredPayment(group) => tiered_payment_procedure::continue_with(group, sentence),
        Procedure::GainedControlFollowup(group) => {
            gained_control_followup_procedure::continue_with(group, sentence)
        }
        Procedure::ExchangeBlocks(group) => exchange_blocks_procedure::continue_with(group, sentence),
        Procedure::PlayerOptionChoice(group) => {
            player_option_choice_procedure::continue_with(group, sentence)
        }
        Procedure::VoteOptionSet(group) => vote_option_set_procedure::continue_with(group, sentence),
        Procedure::ChosenCounterKind(group) => {
            chosen_counter_kind_procedure::continue_with(group, sentence)
        }
        Procedure::RepeatChoice(group) => repeat_choice_procedure::continue_with(group, sentence),
        Procedure::DistinctPlayerAssignment(group) => {
            distinct_player_assignment_procedure::continue_with(group, sentence)
        }
    }
}

pub(super) fn finish(procedure: Procedure) -> Closed {
    match procedure {
        Procedure::Looked(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: looked_procedure::finish(group),
        },
        Procedure::Consulted(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: consult_procedure::finish(group),
        },
        Procedure::Milled(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: mill_procedure::finish(group),
        },
        Procedure::GraveyardCast(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: graveyard_cast_procedure::finish(group),
        },
        Procedure::CopyCast(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: copy_cast_procedure::finish(group),
        },
        Procedure::ExiledTop(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: exiled_top_procedure::finish(group),
        },
        Procedure::Pair(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: pair_procedure::finish(group),
        },
        Procedure::Ridden(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: rider_procedure::finish(group),
        },
        Procedure::Hand(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: hand_procedure::finish(group),
        },
        Procedure::KeywordChoice(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: keyword_choice_procedure::finish(group),
        },
        Procedure::SearchPartition(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: search_partition_procedure::finish(group),
        },
        Procedure::AssignUnblocked(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: assign_unblocked_procedure::finish(group),
        },
        Procedure::ExtraTargets(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: extra_targets_procedure::finish(group),
        },
        Procedure::EachReveal(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: each_reveal_procedure::finish(group),
        },
        Procedure::ChosenHandReveal(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: chosen_hand_reveal_procedure::finish(group),
        },
        Procedure::TieredPayment(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: tiered_payment_procedure::finish(group),
        },
        Procedure::GainedControlFollowup(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: gained_control_followup_procedure::finish(group),
        },
        Procedure::ExchangeBlocks(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: exchange_blocks_procedure::finish(group),
        },
        Procedure::PlayerOptionChoice(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: player_option_choice_procedure::finish(group),
        },
        Procedure::VoteOptionSet(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: vote_option_set_procedure::finish(group),
        },
        Procedure::ChosenCounterKind(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: chosen_counter_kind_procedure::finish(group),
        },
        Procedure::RepeatChoice(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: repeat_choice_procedure::finish(group),
        },
        Procedure::DistinctPlayerAssignment(group) => Closed {
            first_sentence: group.first_sentence,
            consumed: group.consumed,
            effects: distinct_player_assignment_procedure::finish(group),
        },
    }
}

pub(super) fn kind(procedure: &Procedure) -> &'static str {
    match procedure {
        Procedure::Looked(_) => "looked",
        Procedure::Consulted(_) => "consult",
        Procedure::Milled(_) => "milled",
        Procedure::GraveyardCast(_) => "graveyard-cast",
        Procedure::CopyCast(_) => "copy-cast",
        Procedure::ExiledTop(_) => "exiled-top",
        Procedure::Pair(_) => "pair",
        Procedure::Ridden(_) => "ridden",
        Procedure::Hand(_) => "hand",
        Procedure::KeywordChoice(_) => "keyword-choice",
        Procedure::SearchPartition(_) => "search-partition",
        Procedure::AssignUnblocked(_) => "assign-unblocked",
        Procedure::ExtraTargets(_) => "extra-targets",
        Procedure::EachReveal(_) => "each-reveal",
        Procedure::ChosenHandReveal(_) => "chosen-hand-reveal",
        Procedure::TieredPayment(_) => "tiered-payment",
        Procedure::GainedControlFollowup(_) => "gained-control-followup",
        Procedure::ExchangeBlocks(_) => "exchange-blocks",
        Procedure::PlayerOptionChoice(_) => "player-option-choice",
        Procedure::VoteOptionSet(_) => "vote-option-set",
        Procedure::ChosenCounterKind(_) => "chosen-counter-kind",
        Procedure::RepeatChoice(_) => "repeat-choice",
        Procedure::DistinctPlayerAssignment(_) => "distinct-player-assignment",
    }
}

fn name(procedure: &Procedure) -> &'static str {
    match procedure {
        Procedure::Looked(_) => "looked-procedure",
        Procedure::Consulted(_) => "consult-procedure",
        Procedure::Milled(_) => "milled-procedure",
        Procedure::GraveyardCast(_) => "graveyard-cast-procedure",
        Procedure::CopyCast(_) => "copy-cast-procedure",
        Procedure::ExiledTop(_) => "exiled-top-procedure",
        Procedure::Pair(_) => "pair-procedure",
        Procedure::Ridden(_) => RIDDEN_STATEMENT,
        Procedure::Hand(_) => "hand-procedure",
        Procedure::KeywordChoice(_) => "keyword-choice-procedure",
        Procedure::SearchPartition(_) => "search-partition-procedure",
        Procedure::AssignUnblocked(_) => "assign-unblocked-procedure",
        Procedure::ExtraTargets(_) => "extra-targets-procedure",
        Procedure::EachReveal(_) => "each-reveal-procedure",
        Procedure::ChosenHandReveal(_) => "chosen-hand-reveal-procedure",
        Procedure::TieredPayment(_) => "tiered-payment-procedure",
        Procedure::GainedControlFollowup(_) => "gained-control-followup-procedure",
        Procedure::ExchangeBlocks(_) => "exchange-blocks-procedure",
        Procedure::PlayerOptionChoice(_) => "player-option-choice-procedure",
        Procedure::VoteOptionSet(_) => "vote-option-set-procedure",
        Procedure::ChosenCounterKind(_) => "chosen-counter-kind-procedure",
        Procedure::RepeatChoice(_) => "repeat-choice-procedure",
        Procedure::DistinctPlayerAssignment(_) => "distinct-player-assignment-procedure",
    }
}

fn feature_tag(procedure: &Procedure) -> Option<&'static str> {
    match procedure {
        Procedure::CopyCast(group) => Some(copy_cast_procedure::feature_tag(group)),
        Procedure::ExiledTop(group) => Some(exiled_top_procedure::feature_tag(group)),
        Procedure::Pair(group) => Some(pair_procedure::feature_tag(group)),
        _ => None,
    }
}

/// Recognize the procedure opening at this sentence and running as far as its
/// statements go, in the shape the sequence registry reported a program: the
/// effects and how many sentences they consumed. When more than one procedure
/// opens, the one reading the most sentences is the document's, as the
/// registry kept the rule consuming the longest complete program; a tie keeps
/// the higher-ranked one. A candidate whose continuation errs is dropped, and
/// its error stands only when no candidate completes.
/// A leading state predicate decorates an independently recognized complete
/// procedure. Its optional alternative is another complete procedure, not a
/// fixed number or arrangement of library statements.
pub(super) fn recognize(
    sentences: &[SentenceInput], sentence_idx: usize,
) -> Result<Option<super::sequence_rules::DocumentProgramMatch>, CardTextError> {
    if let Some(matched) = crate::word_claims::speculative_keep_if(
        || recognize_conditional_program(sentences, sentence_idx),
        |result| matches!(result, Ok(Some(_))),
    )? { return Ok(Some(matched)); }
    recognize_unconditioned(sentences, sentence_idx)
}

fn recognize_conditional_program(
    sentences: &[SentenceInput], start: usize,
) -> Result<Option<super::sequence_rules::DocumentProgramMatch>, CardTextError> {
    use crate::grammar::effects::control_flow::{recognize_control_flow, RecognizedControlFlowAst};
    use crate::model::{ControlFlowNodeAst, NestedProgramAst, NestedProgramKindAst};
    let Some(first) = sentences.get(start) else { return Ok(None); };
    if !first.lowered().first().is_some_and(|token| token.is_word("if")) { return Ok(None); }
    let crate::recognition::ParseOutcome::Match(plan) = recognize_control_flow(first.lowered()) else { return Ok(None); };
    let plan = plan.value;
    if !matches!(&plan.structure, RecognizedControlFlowAst::Condition { reflexive: false, .. }) { return Ok(None); }
    let mut nested = vec![SentenceInput::from_lexed(plan.body_tokens)];
    nested.extend(sentences[start + 1..].iter().map(|sentence| SentenceInput::from_lexed(sentence.lexed())));
    let Some(consequence) = recognize(&nested, 0)? else { return Ok(None); };
    if consequence.consumed_sentences < 2 { return Ok(None); }
    let mut consumed = consequence.consumed_sentences;
    let Some(mut control) = plan.into_ast(consequence.effects) else { return Ok(None); };
    if let Some(otherwise) = sentences.get(start + consumed) {
        if let Some(((), tail)) = crate::grammar::primitives::parse_prefix(
            otherwise.lowered(), crate::grammar::primitives::phrase(&["otherwise"]))
        {
            let mut alternative = vec![SentenceInput::from_lexed(crate::lexer::trim_lexed_commas(tail))];
            alternative.extend(sentences[start + consumed + 1..].iter().map(|sentence| SentenceInput::from_lexed(sentence.lexed())));
            if let Some(branch) = recognize(&alternative, 0)? {
                if branch.consumed_sentences >= 2 {
                    let ControlFlowNodeAst::Condition { condition, consequence_program, reflexive, .. } = control.node else { unreachable!() };
                    let alternative_program = control.programs.len();
                    control.programs.push(NestedProgramAst::new(NestedProgramKindAst::Alternative, branch.effects));
                    control = crate::model::CompilerControlFlowAst::new(control.semantic,
                        ControlFlowNodeAst::Condition { condition, consequence_program, alternative_program: Some(alternative_program), reflexive }, control.programs, control.provenance).map_err(|error| CardTextError::ParseError(format!("invalid conditional procedure scope: {error:?}")))?;
                    consumed += branch.consumed_sentences;
                }
            }
        }
    }
    Ok(Some(super::sequence_rules::DocumentProgramMatch {
        name: "conditional-document-program", feature_tag: None,
        consumed_sentences: consumed, effects: vec![crate::cards::builders::EffectAst::ControlFlow(Box::new(control))],
    }))
}

fn recognize_unconditioned(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<super::sequence_rules::DocumentProgramMatch>, CardTextError> {
    let opened = open_all(sentences, sentence_idx)?;
    if opened.is_empty() {
        crate::parse_trace::event(format!(
            "no procedure opens at sentence {sentence_idx} of {}: {}",
            sentences.len(),
            sentences
                .iter()
                .map(|sentence| {
                    crate::lexer::token_word_refs(sentence.lowered())
                        .into_iter()
                        .take(7)
                        .collect::<Vec<_>>()
                        .join(" ")
                })
                .collect::<Vec<_>>()
                .join(" | ")
        ));
        return Ok(None);
    }
    let mut best: Option<super::sequence_rules::DocumentProgramMatch> = None;
    let mut deferred: Option<CardTextError> = None;
    for mut procedure in opened {
        let mut continued = Ok(());
        // A repeated look procedure opens with both the initial look and the
        // repeated round already consumed. Resume at its next sentence.
        let opened_count = match &procedure {
            Procedure::RepeatChoice(group) => group.consumed,
            _ => 1,
        };
        for (offset, sentence) in sentences[sentence_idx + opened_count..].iter().enumerate() {
            let rest = sentences.get(sentence_idx + opened_count + 1 + offset..).unwrap_or(&[]);
            match continue_with(&mut procedure, sentence, rest) {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    continued = Err(error);
                    break;
                }
            }
        }
        if let Err(error) = continued {
            deferred.get_or_insert(error);
            continue;
        }
        let name = name(&procedure);
        let feature_tag = feature_tag(&procedure);
        let closed = finish(procedure);
        crate::parse_trace::event(format!(
            "{name}: {} statements -> {}",
            closed.consumed,
            super::dispatch_entry::summarize_effects(&closed.effects)
        ));
        let candidate = super::sequence_rules::DocumentProgramMatch {
            name,
            feature_tag,
            consumed_sentences: closed.consumed,
            effects: closed.effects,
        };
        match &best {
            Some(current) if current.consumed_sentences >= candidate.consumed_sentences => {
                if current.consumed_sentences == candidate.consumed_sentences
                    && current.effects != candidate.effects
                {
                    crate::parse_trace::event(format!(
                        "procedures tie at {} statements: {} kept over {}",
                        current.consumed_sentences, current.name, candidate.name
                    ));
                }
            }
            _ => best = Some(candidate),
        }
    }
    match (best, deferred) {
        (Some(matched), _) => Ok(Some(matched)),
        (None, Some(error)) => Err(error),
        (None, None) => Ok(None),
    }
}

#[cfg(test)]
#[path = "procedures/chosen_quality_scope_tests.rs"]
mod chosen_quality_scope_tests;

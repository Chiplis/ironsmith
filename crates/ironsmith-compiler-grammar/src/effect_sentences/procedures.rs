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
    hand_procedure, looked_procedure, mill_procedure, pair_procedure, rider_procedure,
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
    consider(Ok(looked_procedure::open(sentences, sentence_idx).map(Procedure::Looked)));
    consider(consult_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Consulted)));
    consider(mill_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Milled)));
    consider(
        graveyard_cast_procedure::open(sentences, sentence_idx)
            .map(|group| group.map(Procedure::GraveyardCast)),
    );
    consider(copy_cast_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::CopyCast)));
    consider(exiled_top_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::ExiledTop)));
    consider(rider_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Ridden)));
    consider(hand_procedure::open(sentences, sentence_idx).map(|group| group.map(Procedure::Hand)));
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
pub(super) fn recognize(
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
        for (offset, sentence) in sentences[sentence_idx + 1..].iter().enumerate() {
            let rest = sentences.get(sentence_idx + 2 + offset..).unwrap_or(&[]);
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

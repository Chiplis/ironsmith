//! "Each player chooses a card in their hand. Then each player reveals their
//! chosen card. The owner of each creature card revealed this way with the
//! lowest mana value puts it onto the battlefield." (Stronghold Gambit)
//!
//! Each participant's choice from their own hand is private: the ordinary
//! participant choice from a hidden hand keeps every other peer looking at
//! placeholders until the joint reveal makes the chosen cards public. The
//! choices accumulate in one participant-choice pool, the reveal shows that
//! pool, and the following statements read the revealed pool — optionally
//! narrowed to the cards with the lowest or highest mana value among the
//! matching revealed cards (compared across every player's revealed card).

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ForEachEffectAst, ObjectChoiceEffectAst, ObjectFilter,
    PlayerAst, ReturnControllerAst, Value,
};
use crate::filter::Comparison;
use crate::target::{PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

pub(super) struct ChosenHandRevealGroup {
    pool: crate::tag::TagRef,
    revealed: bool,
    effects: Vec<EffectAst>,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn words(sentence: &SentenceInput) -> Vec<&str> {
    let words = crate::lexer::token_word_refs(sentence.lowered());
    match words.as_slice() {
        ["then", rest @ ..] => rest.to_vec(),
        _ => words,
    }
}

/// "each player chooses a card in/from their hand".
fn is_opening(sentence: &SentenceInput) -> bool {
    matches!(
        words(sentence).as_slice(),
        [
            "each",
            "player",
            "chooses" | "choose",
            "a",
            "card",
            "in" | "from",
            "their",
            "hand"
        ]
    )
}

/// "each player reveals their chosen card".
fn is_reveal(sentence: &SentenceInput) -> bool {
    matches!(
        words(sentence).as_slice(),
        [
            "each",
            "player",
            "reveals" | "reveal",
            "their" | "the",
            "chosen",
            "card" | "cards"
        ]
    )
}

fn in_pool(mut filter: ObjectFilter, pool: &crate::tag::TagRef) -> ObjectFilter {
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: pool.clone().into(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    filter
}

/// "the owner of each <filter> revealed this way [with the lowest/highest
/// mana value] puts it onto the battlefield".
fn statement(
    sentence: &SentenceInput,
    pool: &crate::tag::TagRef,
) -> Result<Option<EffectAst>, CardTextError> {
    const HEAD: &[&str] = &["the", "owner", "of", "each"];
    const TAIL: &[&str] = &["puts", "it", "onto", "the", "battlefield"];
    const THIS_WAY: &[&str] = &["revealed", "this", "way"];
    let lowered = sentence.lowered();
    let view = crate::lexer::TokenWordView::new(lowered);
    let all = view.word_refs();
    let offset = usize::from(all.first().is_some_and(|word| *word == "then"));
    let words = &all[offset..];
    let tail_matches = words.ends_with(TAIL)
        || words.ends_with(&["put", "it", "onto", "the", "battlefield"]);
    if !words.starts_with(HEAD) || !tail_matches || words.len() <= HEAD.len() + TAIL.len() {
        return Ok(None);
    }
    let body = &words[HEAD.len()..words.len() - TAIL.len()];
    let Some(this_way) = body
        .windows(THIS_WAY.len())
        .position(|window| window == THIS_WAY)
    else {
        return Ok(None);
    };
    if this_way == 0 {
        return Ok(None);
    }
    let extremum = match &body[this_way + THIS_WAY.len()..] {
        [] => None,
        ["with", "the", "lowest" | "least" | "smallest", "mana", "value"] => Some(false),
        ["with", "the", "highest" | "greatest", "mana", "value"] => Some(true),
        _ => return Ok(None),
    };
    let start = offset + HEAD.len();
    let Some(range) = view.token_span_for_words(start, start + this_way) else {
        return Ok(None);
    };
    let mut filter = crate::object_filters::parse_object_filter(&lowered[range], false)?;
    filter.zone = Some(Zone::Hand);
    filter = in_pool(filter, pool);
    if let Some(highest) = extremum {
        let among = filter.clone();
        let bound = if highest {
            Value::GreatestManaValue(among)
        } else {
            Value::LeastManaValue(among)
        };
        filter.mana_value = Some(Comparison::EqualExpr(Box::new(bound)));
    }
    // Each owner puts their own card onto the battlefield, under their
    // control (CR 110.2a).
    Ok(Some(EffectAst::subject_verb_put_all_onto_battlefield(
        filter,
        false,
        false,
        ReturnControllerAst::Owner,
    )))
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<ChosenHandRevealGroup>, CardTextError> {
    let Some(sentence) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    if !is_opening(sentence) {
        return Ok(None);
    }
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if !is_reveal(next) {
        return Ok(None);
    }
    let anchor = sentence
        .lowered()
        .first()
        .map(|token| token.span)
        .unwrap_or_else(crate::diagnostics::TextSpan::synthetic);
    let pool = crate::tag::CompilerProvenanceTag::ParticipantChoice {
        line: anchor.line,
        start: anchor.start,
    }
    .key();
    let choose = EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter: ObjectFilter::default()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::IteratedPlayer),
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::That,
        tag: pool.clone(),
    });
    Ok(Some(ChosenHandRevealGroup {
        effects: vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
            effects: vec![choose],
        })],
        pool,
        revealed: false,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    group: &mut ChosenHandRevealGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if !group.revealed {
        if !is_reveal(sentence) {
            return Ok(false);
        }
        // One joint reveal of every chosen card.
        group
            .effects
            .push(EffectAst::subject_verb_reveal_tagged(group.pool.clone()));
        group.revealed = true;
        group.consumed += 1;
        return Ok(true);
    }
    let Some(effect) = statement(sentence, &group.pool)? else {
        return Ok(false);
    };
    group.effects.push(effect);
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: ChosenHandRevealGroup) -> Vec<EffectAst> {
    group.effects
}

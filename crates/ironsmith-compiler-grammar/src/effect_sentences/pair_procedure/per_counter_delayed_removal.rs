//! "Distribute three -1/-1 counters among one, two, or three target
//! creatures. For each -1/-1 counter you put on a creature this way, remove a
//! -1/-1 counter from that creature at the beginning of the next end step."
//! (Shambling Swarm, Bounty of the Hunt with "cleanup step"): each counter
//! actually put creates its own delayed removal from the creature it was put
//! on. The count is read per creature from the placement's own receipt as the
//! instruction resolves (counters put, not counters it already had), and each
//! registration captures that creature.
use super::*;
use crate::cards::builders::{DelayedEffectAst, ForEachEffectAst};
use crate::lexer::{OwnedLexToken, TokenWordView};
use crate::util::trim_edge_punctuation_tokens;

fn counter_type_between(
    tokens: &[OwnedLexToken],
    view: &TokenWordView<'_>,
    start: usize,
    end: usize,
) -> Option<crate::object::CounterType> {
    if end <= start {
        return None;
    }
    let range = view.token_span_for_words(start, end)?;
    crate::util::parse_counter_type_from_tokens(&tokens[range])
}

pub(super) fn read(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let first_words = crate::lexer::token_word_refs(first.lowered());
    if first_words.first() != Some(&"distribute") {
        return Ok(None);
    }
    let tokens = trim_edge_punctuation_tokens(second.lowered());
    let view = TokenWordView::new(tokens);
    let words = view.to_word_refs();
    let Some(rest) = words.strip_prefix(&["for", "each"][..]) else {
        return Ok(None);
    };
    let Some(put_counter) = rest.iter().position(|word| *word == "counter") else {
        return Ok(None);
    };
    let Some(put_kind) = counter_type_between(tokens, &view, 2, 2 + put_counter) else {
        return Ok(None);
    };
    let after_put = &rest[put_counter + 1..];
    let Some(after_put) = after_put
        .strip_prefix(&["you", "put", "on", "a"][..])
        .or_else(|| after_put.strip_prefix(&["you", "put", "on", "an"][..]))
    else {
        return Ok(None);
    };
    let Some(this_way) = after_put.windows(2).position(|window| window == ["this", "way"]) else {
        return Ok(None);
    };
    let noun_start = words.len() - after_put.len();
    let noun_tokens = view
        .token_span_for_words(noun_start, noun_start + this_way)
        .map(|range| &tokens[range]);
    let Some(noun_tokens) = noun_tokens.filter(|tokens| !tokens.is_empty()) else {
        return Ok(None);
    };
    let removal = &after_put[this_way + 2..];
    let Some(removal_rest) = removal
        .strip_prefix(&["remove", "a"][..])
        .or_else(|| removal.strip_prefix(&["remove", "an"][..]))
    else {
        return Ok(None);
    };
    let Some(remove_counter) = removal_rest.iter().position(|word| *word == "counter") else {
        return Ok(None);
    };
    let remove_start = words.len() - removal_rest.len();
    if counter_type_between(tokens, &view, remove_start, remove_start + remove_counter)
        != Some(put_kind)
    {
        return Ok(None);
    }
    let timing = &removal_rest[remove_counter + 1..];
    let Some(timing) = timing.strip_prefix(&["from", "that"][..]) else {
        return Ok(None);
    };
    let Some(timing) = timing
        .iter()
        .position(|word| *word == "at")
        .map(|at| &timing[at..])
    else {
        return Ok(None);
    };
    let cleanup = match timing {
        ["at", "the", "beginning", "of", "the", "next", "end", "step"] => false,
        ["at", "the", "beginning", "of", "the", "next", "cleanup", "step"] => true,
        _ => return Ok(None),
    };

    let put = crate::effect_sentences::parse_effect_sentence_lexed(first.lowered())?;
    if put.is_empty() {
        return Ok(None);
    }
    let recipients = crate::object_filters::parse_object_filter(noun_tokens, false)?
        .in_zone(Zone::Battlefield);
    let mut query = ironsmith_core::PriorEffectMetricQuery::new(
        ironsmith_core::EffectMetricSource::Outcome,
        ironsmith_core::EffectMetric::CountersPut,
    );
    query.action = Some(ironsmith_core::PriorEffectAction::CountersPut);
    query.counter_type = Some(put_kind);
    query.filter = Some(ObjectFilter::default().match_tagged(
        crate::tag::CompilerReferenceTag::It.key(),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    ));
    // A delayed body resolves after the iteration has ended. Capture this
    // recipient under its own tag rather than retaining an Iterated target.
    let recipient = crate::util::helper_tag_for_tokens(tokens, "counter_removal_recipient");
    let capture = EffectAst::subject_verb_tag_matching_objects(
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()),
        vec![Zone::Battlefield], recipient.clone(),
    );
    let removal = EffectAst::subject_verb_remove_up_to_any_counters(
        Value::Fixed(1),
        TargetAst::Tagged(recipient, None),
        Some(put_kind),
        false,
    );
    let delayed = if cleanup {
        DelayedEffectAst::DelayedUntilNextCleanupStep {
            player: PlayerFilter::Any,
            effects: vec![removal],
        }
    } else {
        DelayedEffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![removal],
        }
    };
    let mut effects = put;
    effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachObject {
        filter: recipients,
        effects: vec![capture, EffectAst::ForEach(ForEachEffectAst::RepeatEffects {
            count: Value::PendingPriorEffectMetric(query),
            effects: vec![EffectAst::Delayed(delayed)],
        })],
    }));
    Ok(Some(effects))
}

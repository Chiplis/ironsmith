//! "Choose a counter on a permanent you control. Put a counter of that kind
//! on target permanent you control if it doesn't have a counter of that kind
//! on it." (Aven Courier) / "Choose a kind of counter on a creature you
//! control. Put a counter of that kind on each other creature you control."
//! (Contractual Safeguard)
//!
//! The first statement chooses one counter (an object and a kind of counter
//! on it); the second puts a counter of that kind on its recipients. Both
//! are one instruction: the kind exists only between them.

use super::dispatch_entry::SentenceInput;
use crate::cards::builders::{
    CardTextError, CounterActionAst, EffectAst, PlayerAst, SubjectVerbActionAst,
    SubjectVerbRoleAst, TargetAst,
};
use crate::grammar::primitives;
use crate::lexer::OwnedLexToken;
use crate::target::ObjectFilter;
use crate::zone::Zone;

pub(super) struct ChosenCounterKindGroup {
    kind_source: ObjectFilter,
    effects: Vec<EffectAst>,
    closed: bool,
    pub(super) first_sentence: usize,
    pub(super) consumed: usize,
}

fn trim(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    crate::util::trim_edge_punctuation_tokens(tokens)
}

fn on_battlefield(mut filter: ObjectFilter) -> ObjectFilter {
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    filter
}

/// "choose a counter on <objects>" / "choose a kind of counter on <objects>"
fn kind_source(sentence: &SentenceInput) -> Option<ObjectFilter> {
    let tokens = trim(sentence.lowered());
    let (_, rest) = primitives::parse_prefix(
        tokens,
        winnow::combinator::alt((
            primitives::phrase(&["choose", "a", "counter", "on"]),
            primitives::phrase(&["choose", "a", "kind", "of", "counter", "on"]),
        )),
    )?;
    if rest.is_empty() {
        return None;
    }
    let filter =
        primitives::probe_shape(crate::object_filters::parse_object_filter_lexed(rest, false))?;
    Some(on_battlefield(filter))
}

const ABSENT_SUFFIXES: &[&[&str]] = &[
    &[
        "if", "it", "doesn't", "have", "a", "counter", "of", "that", "kind", "on", "it",
    ],
    &[
        "if", "it", "doesnt", "have", "a", "counter", "of", "that", "kind", "on", "it",
    ],
    &[
        "if", "it", "does", "not", "have", "a", "counter", "of", "that", "kind", "on", "it",
    ],
];

/// "put a counter of that kind on <recipients> [if it doesn't have a counter
/// of that kind on it]"
fn put_that_kind(
    sentence: &SentenceInput,
    kind_source: &ObjectFilter,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim(sentence.lowered());
    let Some((_, rest)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["put", "a", "counter", "of", "that", "kind", "on"]),
    ) else {
        return Ok(None);
    };
    let mut recipients = rest;
    let mut only_if_absent = false;
    for suffix in ABSENT_SUFFIXES {
        let Some(start) = recipients.len().checked_sub(suffix.len()) else {
            continue;
        };
        if primitives::parse_prefix(&recipients[start..], primitives::phrase(*suffix))
            .is_some_and(|(_, after)| after.is_empty())
        {
            recipients = crate::lexer::trim_lexed_commas(&recipients[..start]);
            only_if_absent = true;
            break;
        }
    }
    if recipients.is_empty() {
        return Ok(None);
    }
    let (target, each, exclude_kind_object) = if let Some((_, filter_tokens)) =
        primitives::parse_prefix(recipients, primitives::phrase(&["each", "other"]))
    {
        let filter = crate::object_filters::parse_object_filter_lexed(filter_tokens, false)?;
        (None, Some(on_battlefield(filter)), true)
    } else if let Some((_, filter_tokens)) =
        primitives::parse_prefix(recipients, primitives::kw("each"))
    {
        let filter = crate::object_filters::parse_object_filter_lexed(filter_tokens, false)?;
        (None, Some(on_battlefield(filter)), false)
    } else {
        let target: TargetAst = crate::util::parse_target_phrase(recipients)?;
        (Some(target), None, false)
    };
    Ok(Some(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfKindChosenFrom {
            kind_source: kind_source.clone(),
            target,
            each,
            exclude_kind_object,
            only_if_absent,
        }),
    )))
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<ChosenCounterKindGroup>, CardTextError> {
    let (Some(choice), Some(next)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let Some(kind_source) = kind_source(choice) else {
        return Ok(None);
    };
    // Only a choice followed by its put forms the procedure.
    if primitives::parse_prefix(
        trim(next.lowered()),
        primitives::phrase(&["put", "a", "counter", "of", "that", "kind", "on"]),
    )
    .is_none()
    {
        return Ok(None);
    }
    Ok(Some(ChosenCounterKindGroup {
        kind_source,
        effects: Vec::new(),
        closed: false,
        first_sentence: sentence_idx,
        consumed: 1,
    }))
}

pub(super) fn continue_with(
    group: &mut ChosenCounterKindGroup,
    sentence: &SentenceInput,
) -> Result<bool, CardTextError> {
    if group.closed {
        return Ok(false);
    }
    let Some(effect) = put_that_kind(sentence, &group.kind_source)? else {
        return Ok(false);
    };
    group.effects.push(effect);
    group.closed = true;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: ChosenCounterKindGroup) -> Vec<EffectAst> {
    group.effects
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sentences(text: &str) -> Vec<SentenceInput> {
        let tokens = crate::lexer::lex_line(text, 0).unwrap();
        crate::lexer::split_lexed_sentences(&tokens)
            .into_iter()
            .map(SentenceInput::from_lexed)
            .collect()
    }

    #[test]
    fn choice_and_put_form_one_instruction() {
        let inputs = sentences(
            "Choose a kind of counter on a creature you control. Put a counter of that kind on each other creature you control.",
        );
        let mut group = open(&inputs, 0).unwrap().expect("procedure opens");
        assert!(continue_with(&mut group, &inputs[1]).unwrap());
        let effects = finish(group);
        assert!(
            format!("{effects:?}").contains("exclude_kind_object: true"),
            "{effects:?}"
        );
    }
}

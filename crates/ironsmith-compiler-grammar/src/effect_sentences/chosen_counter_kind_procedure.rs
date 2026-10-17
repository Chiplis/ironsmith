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
    /// "For each non-Saga permanent, choose a counter on it." (The Caves of
    /// Androzani): the choice and its put repeat for each matching object,
    /// whose counters are the kind source.
    for_each: Option<ObjectFilter>,
    /// "choose a kind of counter at random [that <x> doesn't have on it]
    /// from among <kinds>" (Crystalline Giant): the kind comes from a fixed
    /// list instead of counters on objects.
    random_kinds: Option<RandomKindChoice>,
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

/// The iterated object of an enclosing "for each" loop.
fn iterated_object_filter() -> ObjectFilter {
    ObjectFilter::default().match_tagged(
        crate::tag::CompilerReferenceTag::It.key(),
        crate::filter::TaggedOpbjectRelation::IsTaggedObject,
    )
}

/// "for each <objects>, choose a counter on it"
fn for_each_kind_source(sentence: &SentenceInput) -> Option<ObjectFilter> {
    let tokens = trim(sentence.lowered());
    let (_, rest) = primitives::parse_prefix(tokens, primitives::phrase(&["for", "each"]))?;
    let comma = rest.iter().position(|token| token.is_comma())?;
    let tail = crate::lexer::token_word_refs(&rest[comma + 1..]);
    if !matches!(
        tail.as_slice(),
        ["choose", "a", "counter", "on", "it"] | ["choose", "a", "kind", "of", "counter", "on", "it"]
    ) {
        return None;
    }
    let filter = primitives::probe_shape(crate::object_filters::parse_object_filter_lexed(
        &rest[..comma],
        false,
    ))?;
    Some(on_battlefield(filter))
}

struct RandomKindChoice {
    kinds: Vec<crate::object::CounterType>,
    /// Words naming the object whose existing kinds are excluded.
    excluded_holder: Option<Vec<String>>,
}

fn split_list_items(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut items = Vec::new();
    for piece in tokens.split(|token| token.is_comma()) {
        let mut piece = trim(piece);
        while piece
            .first()
            .is_some_and(|token| token.is_any_word(&["and", "or"]))
        {
            piece = trim(&piece[1..]);
        }
        if piece.is_empty() {
            continue;
        }
        // "a and b" / "a or b" without a serial comma.
        if let Some(idx) = piece
            .iter()
            .position(|token| token.is_any_word(&["and", "or"]))
        {
            items.push(trim(&piece[..idx]));
            items.push(trim(&piece[idx + 1..]));
        } else {
            items.push(piece);
        }
    }
    items
}

/// "choose a kind of counter at random [that <holder> doesn't have on it]
/// from among <kind>, <kind>, ..., and <kind>"
fn random_kind_choice(sentence: &SentenceInput) -> Option<RandomKindChoice> {
    let tokens = trim(sentence.lowered());
    let (_, rest) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["choose", "a", "kind", "of", "counter", "at", "random"]),
    )?;
    let from = rest.windows(2).position(|pair| {
        pair[0].is_word("from") && pair[1].is_word("among")
    })?;
    let qualifier = trim(&rest[..from]);
    let list = trim(&rest[from + 2..]);
    let excluded_holder = if qualifier.is_empty() {
        None
    } else {
        let words = crate::lexer::token_word_refs(qualifier);
        let tail: &[&[&str]] = &[
            &["doesn't", "have", "on", "it"],
            &["doesnt", "have", "on", "it"],
            &["does", "not", "have", "on", "it"],
        ];
        let (first, holder) = words.split_first()?;
        if *first != "that" {
            return None;
        }
        let holder = tail.iter().find_map(|suffix| {
            holder
                .len()
                .checked_sub(suffix.len())
                .filter(|start| *start > 0 && holder[*start..] == **suffix)
                .map(|start| &holder[..start])
        })?;
        Some(holder.iter().map(|word| word.to_string()).collect())
    };
    let items = split_list_items(list);
    if items.len() < 2 {
        return None;
    }
    let kinds = items
        .into_iter()
        .map(crate::grammar::filters::parse_counter_type_from_tokens)
        .collect::<Option<Vec<_>>>()?;
    Some(RandomKindChoice {
        kinds,
        excluded_holder,
    })
}

/// "put a counter of that kind on <recipient>" after a random kind choice.
fn put_random_kind(
    sentence: &SentenceInput,
    choice: &RandomKindChoice,
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim(sentence.lowered());
    let Some((optional, recipient)) = put_that_kind_prefix(tokens) else {
        return Ok(None);
    };
    if recipient.is_empty() {
        return Ok(None);
    }
    // The excluded kinds are those on the recipient itself.
    if let Some(holder) = &choice.excluded_holder {
        let recipient_words = crate::lexer::token_word_refs(recipient);
        let same = holder.len() == recipient_words.len()
            && holder
                .iter()
                .zip(&recipient_words)
                .all(|(a, b)| a.as_str() == *b);
        if !same && !matches!(recipient_words.as_slice(), ["it"]) {
            return Ok(None);
        }
    }
    let target: TargetAst = crate::util::parse_target_phrase(recipient)?;
    let put = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfRandomKind {
            kinds: choice.kinds.clone(),
            target,
            only_absent_kinds: choice.excluded_holder.is_some(),
        }),
    );
    Ok(Some(if optional {
        EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May {
            effects: vec![put],
        })
    } else {
        put
    }))
}

const PUT_THAT_KIND_PREFIXES: &[&[&str]] = &[
    &["put", "a", "counter", "of", "that", "kind", "on"],
    &["put", "an", "additional", "counter", "of", "that", "kind", "on"],
    &["you", "may", "put", "a", "counter", "of", "that", "kind", "on"],
    &["you", "may", "put", "an", "additional", "counter", "of", "that", "kind", "on"],
];

fn put_that_kind_prefix(tokens: &[OwnedLexToken]) -> Option<(bool, &[OwnedLexToken])> {
    PUT_THAT_KIND_PREFIXES.iter().find_map(|phrase| {
        primitives::parse_prefix(tokens, primitives::phrase(*phrase))
            .map(|(_, rest)| (phrase[0] == "you", rest))
    })
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
    let Some((optional, rest)) = put_that_kind_prefix(tokens) else {
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
    let put = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Counters(CounterActionAst::PutCounterOfKindChosenFrom {
            kind_source: kind_source.clone(),
            target,
            each,
            exclude_kind_object,
            only_if_absent,
        }),
    );
    Ok(Some(if optional {
        EffectAst::Permissions(crate::cards::builders::PermissionEffectAst::May {
            effects: vec![put],
        })
    } else {
        put
    }))
}

pub(super) fn open(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<ChosenCounterKindGroup>, CardTextError> {
    let (Some(choice), Some(next)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let mut random_kinds = None;
    let (kind_source, for_each) = if let Some(choice) = random_kind_choice(choice) {
        random_kinds = Some(choice);
        (ObjectFilter::default(), None)
    } else if let Some(kind_source) = kind_source(choice) {
        (kind_source, None)
    } else if let Some(for_each) = for_each_kind_source(choice) {
        (iterated_object_filter(), Some(for_each))
    } else {
        return Ok(None);
    };
    // Only a choice followed by its put forms the procedure.
    if put_that_kind_prefix(trim(next.lowered())).is_none() {
        return Ok(None);
    }
    Ok(Some(ChosenCounterKindGroup {
        kind_source,
        for_each,
        random_kinds,
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
    let effect = if let Some(choice) = &group.random_kinds {
        put_random_kind(sentence, choice)?
    } else {
        put_that_kind(sentence, &group.kind_source)?
    };
    let Some(effect) = effect else {
        return Ok(false);
    };
    group.effects.push(effect);
    group.closed = true;
    group.consumed += 1;
    Ok(true)
}

pub(super) fn finish(group: ChosenCounterKindGroup) -> Vec<EffectAst> {
    match group.for_each {
        Some(filter) => vec![EffectAst::ForEach(
            crate::cards::builders::ForEachEffectAst::ForEachObject {
                filter,
                effects: group.effects,
            },
        )],
        None => group.effects,
    }
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

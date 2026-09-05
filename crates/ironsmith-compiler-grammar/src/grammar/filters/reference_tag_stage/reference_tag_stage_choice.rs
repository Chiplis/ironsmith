use super::*;

pub(super) fn try_apply_no_shared_creature_type_with_chosen_creature_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        [
            "that", "doesn't", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "doesnt", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "don't", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "dont", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
        [
            "that", "do", "not", "share", "creature", "type", "with", "chosen", "creature", "they",
            "control",
        ]
        .as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        filter
            .no_shared_creature_types_with
            .push(ObjectFilter::tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            ));
        all_words.drain(fact.span.start..fact.span.start + phrase.len());
        return true;
    }
    false
}

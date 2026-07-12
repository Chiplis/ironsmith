use std::ops::Range;

use super::{word_slice_has_prefix, word_slice_is};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MayCastSurfaceSubject {
    You,
    ExiledCardsOwner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MayCastSurfaceVerb {
    Cast,
    Play,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MayCastSurfaceReference {
    It,
    ThatCard,
    ExiledCard,
    RevealedCard,
    Copy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MayCastManaValueParity {
    Odd,
    Even,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MayCastTailSurface {
    None,
    WithoutPayingManaCost,
    ManaValueAtMost { value_words: Range<usize> },
    ManaValueParity(MayCastManaValueParity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MayCastSentenceFacts {
    pub(crate) subject: MayCastSurfaceSubject,
    pub(crate) verb: MayCastSurfaceVerb,
    pub(crate) reference: MayCastSurfaceReference,
    pub(crate) tail: MayCastTailSurface,
}

pub(crate) fn parse_may_cast_sentence_facts(words: &[&str]) -> Option<MayCastSentenceFacts> {
    let mut start = skip_leading_connectors(words, 0);
    if word_slice_has_prefix(&words[start..], &["if", "you", "do"]) {
        start += 3;
        start = skip_leading_connectors(words, start);
    }
    let clause = words.get(start..)?;

    let (subject, verb_offset) =
        if clause.len() >= 4 && word_slice_has_prefix(clause, &["you", "may"]) {
            (MayCastSurfaceSubject::You, 2usize)
        } else if clause.len() >= 7
            && word_slice_has_prefix(clause, &["the", "exiled", "cards", "owner", "may"])
        {
            (MayCastSurfaceSubject::ExiledCardsOwner, 5usize)
        } else {
            return None;
        };
    let verb = match clause.get(verb_offset).copied()? {
        "cast" => MayCastSurfaceVerb::Cast,
        "play" => MayCastSurfaceVerb::Play,
        _ => return None,
    };

    let reference_start = verb_offset + 1;
    let reference_words = clause.get(reference_start..)?;
    let (reference, consumed) = if word_slice_has_prefix(reference_words, &["it"]) {
        (MayCastSurfaceReference::It, 1usize)
    } else if word_slice_has_prefix(reference_words, &["that", "card"]) {
        (MayCastSurfaceReference::ThatCard, 2usize)
    } else if word_slice_has_prefix(reference_words, &["the", "exiled", "card"]) {
        (MayCastSurfaceReference::ExiledCard, 3usize)
    } else if starts_with_any(
        reference_words,
        &[&["the", "revealed", "card"], &["that", "revealed", "card"]],
    ) {
        (MayCastSurfaceReference::RevealedCard, 3usize)
    } else if starts_with_any(
        reference_words,
        &[&["the", "copy"], &["that", "copy"], &["a", "copy"]],
    ) {
        (MayCastSurfaceReference::Copy, 2usize)
    } else {
        return None;
    };

    let tail_start = start + reference_start + consumed;
    let tail_words = words.get(tail_start..)?;
    let tail = if tail_words.is_empty() {
        MayCastTailSurface::None
    } else if word_slice_is(tail_words, &["without", "paying", "its", "mana", "cost"]) {
        MayCastTailSurface::WithoutPayingManaCost
    } else if word_slice_has_prefix(
        tail_words,
        &[
            "without", "paying", "its", "mana", "cost", "if", "its", "a", "spell", "with", "mana",
            "value", "less", "than", "or", "equal", "to",
        ],
    ) {
        MayCastTailSurface::ManaValueAtMost {
            value_words: tail_start + 17..words.len(),
        }
    } else if let [
        "without",
        "paying",
        "its",
        "mana",
        "cost",
        "if",
        "its",
        "mana",
        "value",
        "is",
        parity,
    ] = tail_words
    {
        MayCastTailSurface::ManaValueParity(match *parity {
            "odd" => MayCastManaValueParity::Odd,
            "even" => MayCastManaValueParity::Even,
            _ => return None,
        })
    } else {
        return None;
    };

    Some(MayCastSentenceFacts {
        subject,
        verb,
        reference,
        tail,
    })
}

fn skip_leading_connectors(words: &[&str], mut start: usize) -> usize {
    while words
        .get(start)
        .is_some_and(|word| matches!(*word, "then" | "and"))
    {
        start += 1;
    }
    start
}

fn starts_with_any(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| word_slice_has_prefix(words, expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_may_cast_facts_preserve_subject_reference_and_free_cast() {
        let facts = parse_may_cast_sentence_facts(&[
            "if", "you", "do", "then", "the", "exiled", "cards", "owner", "may", "play", "that",
            "card", "without", "paying", "its", "mana", "cost",
        ])
        .unwrap();
        assert_eq!(facts.subject, MayCastSurfaceSubject::ExiledCardsOwner);
        assert_eq!(facts.verb, MayCastSurfaceVerb::Play);
        assert_eq!(facts.reference, MayCastSurfaceReference::ThatCard);
        assert_eq!(facts.tail, MayCastTailSurface::WithoutPayingManaCost);
    }

    #[test]
    fn typed_may_cast_facts_return_value_expression_boundary() {
        let words = [
            "you", "may", "cast", "the", "exiled", "card", "without", "paying", "its", "mana",
            "cost", "if", "its", "a", "spell", "with", "mana", "value", "less", "than", "or",
            "equal", "to", "thiss", "power",
        ];
        let facts = parse_may_cast_sentence_facts(&words).unwrap();
        assert_eq!(
            facts.tail,
            MayCastTailSurface::ManaValueAtMost {
                value_words: 23..25
            }
        );
    }

    #[test]
    fn typed_may_cast_facts_reject_unrecognized_tail() {
        assert!(
            parse_may_cast_sentence_facts(&["you", "may", "cast", "it", "next", "turn"]).is_none()
        );
    }
}

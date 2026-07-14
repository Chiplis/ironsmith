use crate::runtime_backend::lexer::OwnedLexToken;

use super::{
    TriggerControllerReference, exact_phrase_occurs, exact_word_occurs, word_slice_is,
    word_slice_is_any,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellOriginSurface {
    Graveyard,
    Exile,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpellOwnerSurface {
    SubjectActor,
    Opponent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellActivitySurfaceFacts {
    pub(crate) has_spell_noun: bool,
    pub(crate) during_their_turn: bool,
    pub(crate) during_turn: Option<TriggerControllerReference>,
    pub(crate) exact_spells_this_turn: Option<u32>,
    pub(crate) min_spells_this_turn: Option<u32>,
    pub(crate) from_not_hand: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DrawTurnSurfaceFacts {
    pub(crate) exact_draws_this_turn: Option<u32>,
    pub(crate) draw_numbers_this_turn: Vec<u32>,
    pub(crate) except_first_in_draw_step: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellFilterSurfaceFacts<'a> {
    pub(crate) is_unqualified_spell: bool,
    pub(crate) has_spell_noun: bool,
    pub(crate) origin: Option<SpellOriginSurface>,
    pub(crate) owner: Option<SpellOwnerSurface>,
    pub(crate) qualifier_words: Option<Vec<&'a str>>,
    pub(crate) chosen_color_qualifier: bool,
}

pub(crate) fn parse_spell_activity_surface_facts(words: &[&str]) -> SpellActivitySurfaceFacts {
    let has_spell_noun = exact_word_occurs(words, &["spell", "spells"]);
    let during_their_turn = any_sequence_present(
        words,
        &[
            &["during", "their", "turn"],
            &["during", "that", "players", "turn"],
        ],
    );
    let during_turn = if exact_phrase_occurs(words, &["during", "your", "turn"]) {
        Some(TriggerControllerReference::You)
    } else if any_sequence_present(words, opponent_turn_phrases()) {
        Some(TriggerControllerReference::Opponent)
    } else {
        None
    };
    let other_than_first_spell =
        any_sequence_present(
            words,
            &[
                &["other", "than", "your", "first", "spell"],
                &["other", "than", "the", "first", "spell"],
            ],
        ) || (exact_phrase_occurs(words, &["other", "than", "the", "first"])
            && all_words_present(words, &["spell", "casts", "turn"]));
    let first_spell_each_turn = !other_than_first_spell && first_spell_turn_surface(words);
    let second_spell_each_turn = !other_than_first_spell && second_spell_turn_surface(words);
    let exact_spells_this_turn = exact_spell_count_surface(words)
        .or_else(|| first_spell_each_turn.then_some(1))
        .or_else(|| second_spell_each_turn.then_some(2));

    SpellActivitySurfaceFacts {
        has_spell_noun,
        during_their_turn,
        during_turn,
        exact_spells_this_turn,
        min_spells_this_turn: (exact_spells_this_turn.is_none() && other_than_first_spell)
            .then_some(2),
        from_not_hand: cast_from_outside_hand_surface(words),
    }
}

pub(crate) fn parse_draw_turn_surface_facts(words: &[&str]) -> DrawTurnSurfaceFacts {
    let except_first_in_draw_step = draw_except_first_surface(words);
    let draw_numbers_this_turn = if except_first_in_draw_step {
        Vec::new()
    } else {
        draw_number_set_surface(words)
    };
    let exact_draws_this_turn = match draw_numbers_this_turn.as_slice() {
        [card_number] => Some(*card_number),
        _ if except_first_in_draw_step => Some(2),
        _ => None,
    };
    DrawTurnSurfaceFacts {
        exact_draws_this_turn,
        draw_numbers_this_turn,
        except_first_in_draw_step,
    }
}

pub(crate) fn parse_spell_filter_surface_facts<'a>(
    words: &[&'a str],
) -> SpellFilterSurfaceFacts<'a> {
    let is_unqualified_spell =
        word_slice_is_any(words, &[&["a", "spell"], &["spells"], &["spell"]]);
    let has_spell_noun = exact_word_occurs(words, &["spell", "spells"]);
    let origin = if exact_word_occurs(words, &["graveyard"]) {
        Some(SpellOriginSurface::Graveyard)
    } else if exact_word_occurs(words, &["exile"]) {
        Some(SpellOriginSurface::Exile)
    } else if exact_word_occurs(words, &["hand"]) {
        Some(SpellOriginSurface::Hand)
    } else {
        None
    };
    let owner = if exact_word_occurs(words, &["your"]) {
        Some(SpellOwnerSurface::SubjectActor)
    } else if exact_word_occurs(words, &["opponent", "their"]) {
        Some(SpellOwnerSurface::Opponent)
    } else {
        None
    };

    let mut compact_words = words
        .iter()
        .copied()
        .filter(|word| !matches!(*word, "a" | "an" | "the"))
        .collect::<Vec<_>>();
    let qualifier_words = compact_words
        .last()
        .is_some_and(|word| matches!(*word, "spell" | "spells"))
        .then(|| {
            compact_words.pop();
            compact_words
                .into_iter()
                .filter(|word| !matches!(*word, "or" | "and"))
                .collect::<Vec<_>>()
        });
    let chosen_color_qualifier = qualifier_words.as_deref().is_some_and(|qualifier| {
        word_slice_is_any(
            qualifier,
            &[
                &["of", "the", "chosen", "color"],
                &["of", "chosen", "color"],
            ],
        )
    });

    SpellFilterSurfaceFacts {
        is_unqualified_spell,
        has_spell_noun,
        origin,
        owner,
        qualifier_words,
        chosen_color_qualifier,
    }
}

pub(crate) fn spell_activity_words_are_or_separator(words: &[&str]) -> bool {
    word_slice_is(words, &["or"])
}

pub(crate) fn trim_trailing_spell_auxiliary_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let mut end = tokens.len();
    while tokens[..end]
        .last()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "is" | "are" | "was" | "were" | "be" | "been"))
    {
        end -= 1;
    }
    &tokens[..end]
}

pub(crate) fn spell_tokens_have_noun(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| matches!(word, "spell" | "spells"))
}

fn exact_spell_count_surface(words: &[&str]) -> Option<u32> {
    for (ordinal, count) in ordinal_counts(3) {
        let patterns: &[&[&str]] = &[
            &[ordinal, "spell", "cast", "this", "turn"],
            &[ordinal, "spell", "this", "turn"],
            &["your", ordinal, "spell", "each", "turn"],
            &["their", ordinal, "spell", "each", "turn"],
            &["your", ordinal, "spell", "this", "turn"],
            &["their", ordinal, "spell", "this", "turn"],
            &[ordinal, "spell", "each", "turn"],
        ];
        if any_sequence_present(words, patterns) {
            return Some(count);
        }
    }
    None
}

fn draw_number_set_surface(words: &[&str]) -> Vec<u32> {
    let Some(card_idx) = words.iter().enumerate().find_map(|(idx, word)| {
        (matches!(*word, "card" | "cards")
            && matches!(words.get(idx + 1..idx + 3), Some(["each" | "this", "turn"])))
        .then_some(idx)
    }) else {
        return Vec::new();
    };
    let ordinal_phrase = &words[..card_idx];
    if ordinal_phrase.iter().any(|word| {
        ironsmith_core::parse_ordinal_word(word).is_none()
            && !matches!(*word, "your" | "their" | "the" | "a" | "or" | "and")
    }) {
        return Vec::new();
    }
    let mut card_numbers = ordinal_phrase
        .iter()
        .filter_map(|word| ironsmith_core::parse_ordinal_word(word))
        .filter(|number| *number > 0)
        .collect::<Vec<_>>();
    card_numbers.sort_unstable();
    card_numbers.dedup();
    card_numbers
}

fn first_spell_turn_surface(words: &[&str]) -> bool {
    let turn_context = any_sequence_present(
        words,
        &[
            &["each", "turn"],
            &["this", "turn"],
            &["of", "a", "turn"],
            &["during", "your", "turn"],
            &["during", "their", "turn"],
        ],
    ) || any_sequence_present(words, opponent_turn_phrases());
    if !turn_context {
        return false;
    }

    words.iter().enumerate().any(|(index, word)| {
        *word == "first"
            && words[index + 1..(index + 5).min(words.len())]
                .iter()
                .any(|candidate| matches!(*candidate, "spell" | "spells"))
    })
}

fn second_spell_turn_surface(words: &[&str]) -> bool {
    any_sequence_present(
        words,
        &[
            &["second", "spell", "cast", "this", "turn"],
            &["second", "spell", "this", "turn"],
            &["your", "second", "spell", "each", "turn"],
            &["their", "second", "spell", "each", "turn"],
            &["your", "second", "spell", "this", "turn"],
            &["their", "second", "spell", "this", "turn"],
            &["second", "spell", "each", "turn"],
            &["second", "spell", "during", "your", "turn"],
            &["second", "spell", "during", "their", "turn"],
            &["second", "spell", "during", "an", "opponents", "turn"],
            &["second", "spell", "during", "opponents", "turn"],
            &["second", "spell", "during", "each", "opponents", "turn"],
        ],
    )
}

fn cast_from_outside_hand_surface(words: &[&str]) -> bool {
    if any_sequence_present(
        words,
        &[
            &["from", "anywhere", "other", "than", "your", "hand"],
            &["from", "anywhere", "other", "than", "their", "hand"],
            &["from", "anywhere", "other", "than", "hand"],
        ],
    ) {
        return true;
    }

    if words.len() < 4 {
        return false;
    }
    let mut first = None;
    for index in 0..=words.len() - 4 {
        if word_slice_is(
            &words[index..index + 4],
            &["from", "anywhere", "other", "than"],
        ) {
            first = Some(index);
            break;
        }
    }
    first.is_some_and(|index| {
        words[index + 4..]
            .iter()
            .take(4)
            .any(|word| *word == "hand")
    })
}

fn draw_except_first_surface(words: &[&str]) -> bool {
    any_sequence_present(
        words,
        &[
            &[
                "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of",
                "their", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "they", "draw", "in", "each", "of",
                "their", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "one", "you", "draw", "in", "each", "of",
                "your", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "you", "draw", "in", "each", "of",
                "your", "draw", "steps",
            ],
            &[
                "a", "card", "except", "the", "first", "one", "they", "draw", "in", "their",
                "draw", "step",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "they", "draw", "in", "their",
                "draw", "step",
            ],
            &[
                "a", "card", "except", "the", "first", "one", "you", "draw", "in", "your", "draw",
                "step",
            ],
            &[
                "a", "card", "except", "the", "first", "card", "you", "draw", "in", "your", "draw",
                "step",
            ],
        ],
    )
}

fn opponent_turn_phrases() -> &'static [&'static [&'static str]] {
    &[
        &["during", "an", "opponents", "turn"],
        &["during", "an", "opponent's", "turn"],
        &["during", "an", "opponent", "s", "turn"],
        &["during", "opponents", "turn"],
        &["during", "opponent's", "turn"],
        &["during", "opponent", "s", "turn"],
        &["during", "each", "opponents", "turn"],
        &["during", "each", "opponent's", "turn"],
        &["during", "each", "opponent", "s", "turn"],
    ]
}

fn ordinal_counts(first: u32) -> impl Iterator<Item = (&'static str, u32)> {
    [
        ("second", 2),
        ("third", 3),
        ("fourth", 4),
        ("fifth", 5),
        ("sixth", 6),
        ("seventh", 7),
        ("eighth", 8),
        ("ninth", 9),
        ("tenth", 10),
    ]
    .into_iter()
    .filter(move |(_, count)| *count >= first)
}

fn any_sequence_present(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|phrase| exact_phrase_occurs(words, phrase))
}

fn all_words_present(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .all(|expected_word| exact_word_occurs(words, &[*expected_word]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_spell_activity_facts_preserve_turn_counts_and_origin() {
        let facts = parse_spell_activity_surface_facts(&[
            "whenever", "you", "cast", "your", "third", "spell", "each", "turn",
        ]);
        assert!(facts.has_spell_noun);
        assert_eq!(facts.exact_spells_this_turn, Some(3));
        assert_eq!(facts.min_spells_this_turn, None);

        let outside_hand = parse_spell_activity_surface_facts(&[
            "whenever", "you", "cast", "a", "spell", "from", "anywhere", "other", "than", "your",
            "hand",
        ]);
        assert!(outside_hand.from_not_hand);
    }

    #[test]
    fn typed_draw_facts_preserve_draw_step_exception() {
        let facts = parse_draw_turn_surface_facts(&[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ]);
        assert!(facts.except_first_in_draw_step);
        assert_eq!(facts.exact_draws_this_turn, Some(2));
        assert!(facts.draw_numbers_this_turn.is_empty());
    }

    #[test]
    fn typed_draw_facts_preserve_numbered_draw_sets() {
        let facts = parse_draw_turn_surface_facts(&[
            "your", "first", "or", "second", "card", "each", "turn",
        ]);
        assert_eq!(facts.exact_draws_this_turn, None);
        assert_eq!(facts.draw_numbers_this_turn, vec![1, 2]);
    }

    #[test]
    fn typed_spell_filter_facts_preserve_color_origin_and_owner() {
        let chosen = parse_spell_filter_surface_facts(&["of", "the", "chosen", "color", "spells"]);
        assert!(chosen.chosen_color_qualifier);

        let graveyard =
            parse_spell_filter_surface_facts(&["a", "spell", "from", "your", "graveyard"]);
        assert_eq!(graveyard.origin, Some(SpellOriginSurface::Graveyard));
        assert_eq!(graveyard.owner, Some(SpellOwnerSurface::SubjectActor));
    }
}

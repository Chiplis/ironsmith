use crate::lexer::OwnedLexToken;

use super::{
    TriggerControllerReference, exact_phrase_occurs, exact_word_occurs,
    parse_trigger_controller_reference, word_slice_is, word_slice_is_any,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerSubjectSurfaceFacts {
    pub(crate) player: Option<TriggerControllerReference>,
    pub(crate) contains_you: bool,
    pub(crate) contains_enchanted_player: bool,
    pub(crate) contains_chosen_player: bool,
    pub(crate) contains_opponent: bool,
    pub(crate) on_your_team: bool,
    pub(crate) any_source: bool,
    pub(crate) relative_pronoun: bool,
    pub(crate) power_greater_than_base_power: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShuffleTriggerSubjectFacts {
    pub(crate) player: TriggerControllerReference,
    pub(crate) caused_by_spell_or_ability: bool,
    pub(crate) use_effect_controller: bool,
}

pub(crate) fn parse_trigger_subject_surface_facts(words: &[&str]) -> TriggerSubjectSurfaceFacts {
    TriggerSubjectSurfaceFacts {
        player: parse_trigger_controller_reference(words),
        contains_you: exact_word_occurs(words, &["you"]),
        contains_enchanted_player: any_sequence_present(
            words,
            &[&["enchanted", "player"], &["enchanted", "players"]],
        ),
        contains_chosen_player: any_sequence_present(
            words,
            &[&["chosen", "player"], &["chosen", "players"]],
        ),
        contains_opponent: exact_word_occurs(words, &["opponent", "opponents"]),
        on_your_team: any_sequence_present(words, &[&["your", "team"], &["on", "your", "team"]]),
        any_source: word_slice_is_any(words, &[&["a", "source"], &["source"], &["any", "source"]]),
        relative_pronoun: exact_word_occurs(words, &["that", "which", "who", "whom"]),
        power_greater_than_base_power: exact_phrase_occurs(
            words,
            &["power", "greater", "than", "its", "base", "power"],
        ) && exact_word_occurs(words, &["creature", "creatures"]),
    }
}

pub(crate) fn parse_shuffle_trigger_subject_facts(
    words: &[&str],
) -> Option<ShuffleTriggerSubjectFacts> {
    if let Some(player) = parse_trigger_controller_reference(words) {
        return Some(ShuffleTriggerSubjectFacts {
            player,
            caused_by_spell_or_ability: false,
            use_effect_controller: false,
        });
    }

    if words.len() <= 6
        || !starts_with_phrase(words, &["a", "spell", "or", "ability", "causes"])
        || !ends_with_phrase(words, &["to"])
    {
        return None;
    }
    let caused_player_words = &words[5..words.len() - 1];
    if word_slice_is(caused_player_words, &["its", "controller"]) {
        return Some(ShuffleTriggerSubjectFacts {
            player: TriggerControllerReference::AnyPlayer,
            caused_by_spell_or_ability: true,
            use_effect_controller: true,
        });
    }
    parse_trigger_controller_reference(caused_player_words).map(|player| {
        ShuffleTriggerSubjectFacts {
            player,
            caused_by_spell_or_ability: true,
            use_effect_controller: false,
        }
    })
}

pub(crate) fn trigger_words_are_one_or_more(words: &[&str]) -> bool {
    word_slice_is(words, &["one", "or", "more"])
}

pub(crate) fn trigger_word_is_connector(word: &str) -> bool {
    matches!(word, "and" | "or")
}

pub(crate) fn trigger_word_is_other_modifier(word: &str) -> bool {
    matches!(word, "another" | "other")
}

pub(crate) fn normalize_each_with_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = Vec::with_capacity(tokens.len());
    let mut index = 0usize;
    while index < tokens.len() {
        let skip_each = tokens
            .get(index)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word == "each")
            && tokens
                .get(index + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| word == "with");
        if !skip_each {
            normalized.push(tokens[index].clone());
        }
        index += 1;
    }
    normalized
}

fn any_sequence_present(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|phrase| exact_phrase_occurs(words, phrase))
}

fn starts_with_phrase(words: &[&str], expected: &[&str]) -> bool {
    words
        .get(..expected.len())
        .is_some_and(|prefix| word_slice_is(prefix, expected))
}

fn ends_with_phrase(words: &[&str], expected: &[&str]) -> bool {
    words
        .len()
        .checked_sub(expected.len())
        .and_then(|first| words.get(first..))
        .is_some_and(|suffix| word_slice_is(suffix, expected))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::{TokenWordView, lex_line};

    #[test]
    fn typed_subject_facts_preserve_players_sources_and_power_conditions() {
        let opponent = parse_trigger_subject_surface_facts(&["one", "or", "more", "opponents"]);
        assert_eq!(opponent.player, Some(TriggerControllerReference::Opponent));
        assert!(opponent.contains_opponent);

        let source = parse_trigger_subject_surface_facts(&["any", "source"]);
        assert!(source.any_source);

        let power = parse_trigger_subject_surface_facts(&[
            "a", "creature", "with", "power", "greater", "than", "its", "base", "power",
        ]);
        assert!(power.power_greater_than_base_power);
    }

    #[test]
    fn typed_subject_normalization_removes_only_each_before_with() {
        let tokens = lex_line("creatures each with flying", 0).unwrap();
        let normalized = normalize_each_with_tokens(&tokens);
        assert_eq!(
            TokenWordView::new(&normalized).word_refs(),
            ["creatures", "with", "flying"]
        );
    }

    #[test]
    fn typed_shuffle_subject_facts_preserve_controller_provenance() {
        let facts = parse_shuffle_trigger_subject_facts(&[
            "a",
            "spell",
            "or",
            "ability",
            "causes",
            "its",
            "controller",
            "to",
        ])
        .unwrap();
        assert_eq!(facts.player, TriggerControllerReference::AnyPlayer);
        assert!(facts.caused_by_spell_or_ability);
        assert!(facts.use_effect_controller);
    }
}

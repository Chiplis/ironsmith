use std::ops::Range;

use winnow::Parser;

use crate::events::KeywordActionKind;
use crate::lexer::OwnedLexToken;

use super::super::primitives;

/// Two complete keyword-action verbs sharing a subject. The caller resolves
/// the subject as a player before constructing either event predicate.
#[derive(Debug, Clone, PartialEq)]
pub struct SharedKeywordActionAlternatives {
    pub subject: Range<usize>,
    pub left: KeywordActionKind,
    pub right: KeywordActionKind,
}

pub fn parse_shared_keyword_action_alternatives(
    tokens: &[OwnedLexToken],
) -> Option<SharedKeywordActionAlternatives> {
    let (subject_end, (left, _, right, _), rest) = primitives::find_prefix(tokens, || {
        (
            primitives::word_text.verify_map(KeywordActionKind::from_trigger_word),
            primitives::kw("or"),
            primitives::word_text.verify_map(KeywordActionKind::from_trigger_word),
            primitives::sentence_end(),
        )
    })?;
    if subject_end == 0 || !rest.is_empty() {
        return None;
    }
    Some(SharedKeywordActionAlternatives {
        subject: 0..subject_end,
        left,
        right,
    })
}

/// Three or more keyword-action verbs in one serial list sharing a subject:
/// "you waterbend, earthbend, firebend, or airbend" (Avatar Aang). Returns
/// the subject's token range and the listed actions in order.
pub fn parse_shared_keyword_action_list(
    tokens: &[OwnedLexToken],
) -> Option<(Range<usize>, Vec<KeywordActionKind>)> {
    let mut end = tokens.len();
    while end > 0 && tokens[end - 1].is_period() {
        end -= 1;
    }
    let tokens = &tokens[..end];
    let action_at = |index: usize| {
        tokens
            .get(index)
            .and_then(|token| token.as_word())
            .and_then(KeywordActionKind::from_trigger_word)
    };
    'start: for start in 1..tokens.len() {
        let Some(first) = action_at(start) else {
            continue;
        };
        let mut actions = vec![first];
        let mut index = start + 1;
        loop {
            if tokens.get(index).is_some_and(|token| token.is_comma()) {
                index += 1;
            }
            let last = tokens.get(index).is_some_and(|token| token.is_word("or"));
            if last {
                index += 1;
            }
            let Some(action) = action_at(index) else {
                continue 'start;
            };
            actions.push(action);
            index += 1;
            if last {
                break;
            }
        }
        if index == tokens.len() && actions.len() >= 3 {
            return Some((0..start, actions));
        }
    }
    None
}

/// One randomized player action a trigger can observe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomActionTriggerArm {
    Action(KeywordActionKind),
    /// "roll one or more dice" (CR 706): the die-roll event family.
    RollDice,
}

fn random_action_arm(words: &[&str]) -> Option<RandomActionTriggerArm> {
    use RandomActionTriggerArm::{Action, RollDice};
    Some(match words {
        ["discard" | "discards", "one", "or", "more", "cards", "at", "random"] => {
            Action(KeywordActionKind::DiscardAtRandom)
        }
        ["select" | "selects", "one", "or", "more", "targets", "at", "random"] => {
            Action(KeywordActionKind::SelectTargetsAtRandom)
        }
        ["choose" | "chooses", "one", "or", "more", "modes", "at", "random"] => {
            Action(KeywordActionKind::ChooseModesAtRandom)
        }
        ["flip" | "flips", "one", "or", "more", "coins"] => Action(KeywordActionKind::FlipCoins),
        ["roll" | "rolls", "one", "or", "more", "dice"] => RollDice,
        _ => return None,
    })
}

/// A serial list of randomized actions sharing one player subject: "you
/// discard one or more cards at random, select one or more targets at
/// random, choose one or more modes at random, flip one or more coins, or
/// roll one or more dice" (Chira, All In). Every arm must be a complete
/// random action; returns the subject's token range and the arms in order.
pub fn parse_shared_random_action_list(
    tokens: &[OwnedLexToken],
) -> Option<(Range<usize>, Vec<RandomActionTriggerArm>)> {
    let mut end = tokens.len();
    while end > 0 && tokens[end - 1].is_period() {
        end -= 1;
    }
    let tokens = &tokens[..end];
    if tokens.iter().any(|token| token.as_word().is_none() && !token.is_comma()) {
        return None;
    }
    // Arms are separated by commas and a list-final "or"; the "or" inside
    // "one or more" belongs to its arm.
    // Locating the arm separators reads nothing yet.
    let segments = crate::word_claims::probe_only(|| {
        let mut segments = Vec::new();
        let mut start = 0;
        for (index, token) in tokens.iter().enumerate() {
            let inner_or = token.is_word("or")
                && index > 0
                && tokens[index - 1].is_word("one")
                && tokens.get(index + 1).is_some_and(|next| next.is_word("more"));
            if token.is_comma() || (token.is_word("or") && !inner_or) {
                if start < index {
                    segments.push(start..index);
                }
                start = index + 1;
            }
        }
        if start < tokens.len() {
            segments.push(start..tokens.len());
        }
        segments
    });
    if segments.len() < 2 {
        return None;
    }
    let words = |range: &Range<usize>| -> Vec<&str> {
        tokens[range.clone()].iter().filter_map(OwnedLexToken::as_word).collect()
    };
    let first = segments[0].clone();
    let first_words = words(&first);
    let (subject_len, first_arm) = (1..first_words.len())
        .find_map(|split| random_action_arm(&first_words[split..]).map(|arm| (split, arm)))?;
    let mut arms = vec![first_arm];
    for segment in &segments[1..] {
        arms.push(random_action_arm(&words(segment))?);
    }
    Some((first.start..first.start + subject_len, arms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn keyword_alternatives_retain_shared_subject_and_typed_actions() {
        let tokens = lex_line("an opponent scries or surveils", 0).unwrap();
        let parsed = parse_shared_keyword_action_alternatives(&tokens).unwrap();
        assert_eq!(parsed.subject, 0..2);
        assert_eq!(parsed.left, KeywordActionKind::Scry);
        assert_eq!(parsed.right, KeywordActionKind::Surveil);
    }

    #[test]
    fn keyword_alternatives_do_not_consume_explicit_subjects_or_qualifiers() {
        for text in [
            "you scry or an opponent surveils",
            "you scry or surveil during your turn",
            "you cycle or discard another card",
            "scry or surveil",
        ] {
            assert!(
                parse_shared_keyword_action_alternatives(&lex_line(text, 0).unwrap()).is_none(),
                "{text} must be handled by its complete clause grammar"
            );
        }
    }
}

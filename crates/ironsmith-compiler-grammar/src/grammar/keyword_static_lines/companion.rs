use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::types::Subtype;

/// Parse the closed Companion deck-building condition family from the lexed
/// rule. The optional keyword label and the standard explanatory reminder are
/// presentation; the returned value is the typed deck constraint.
pub fn parse_companion_deck_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ironsmith_core::CompanionDeckCondition> {
    let words = parser_token_word_refs(tokens);
    let start = usize::from(crate::word_primitives::parse_sequence_prefix(
        &words,
        &["companion"],
    ));
    let end = crate::word_primitives::parse_sequence_start(&words, &["if", "this", "card"])
        .unwrap_or(words.len());
    let condition = words.get(start..end)?;
    // The family is closed: exactly one authored sentence per constraint, so
    // the sentence that matches completely names the constraint.
    for (index, shape) in CONDITION_SHAPES.iter().enumerate() {
        if crate::word_primitives::parse_sequence_complete(condition, shape) {
            return Some(condition_for(index));
        }
    }
    None
}

/// The authored condition sentences, one per typed deck constraint; the
/// constraint for `CONDITION_SHAPES[k]` is the `k`th arm of `condition_for`.
const CONDITION_SHAPES: &[&[&str]] = &[
    &[
        "your", "starting", "deck", "contains", "only", "cards", "with", "even", "mana", "values",
    ],
    &[
        "no", "card", "in", "your", "starting", "deck", "has", "more", "than", "one", "of", "the",
        "same", "mana", "symbol", "in", "its", "mana", "cost",
    ],
    &[
        "each",
        "creature",
        "card",
        "in",
        "your",
        "starting",
        "deck",
        "is",
        "a",
        "cat",
        "elemental",
        "nightmare",
        "dinosaur",
        "or",
        "beast",
        "card",
    ],
    &[
        "your", "starting", "deck", "contains", "only", "cards", "with", "mana", "value", "3",
        "or", "greater", "and", "land", "cards",
    ],
    &[
        "each",
        "permanent",
        "card",
        "in",
        "your",
        "starting",
        "deck",
        "has",
        "mana",
        "value",
        "2",
        "or",
        "less",
    ],
    &[
        "each",
        "nonland",
        "card",
        "in",
        "your",
        "starting",
        "deck",
        "has",
        "a",
        "different",
        "name",
    ],
    &[
        "your", "starting", "deck", "contains", "only", "cards", "with", "odd", "mana", "values",
        "and", "land", "cards",
    ],
    &[
        "each", "nonland", "card", "in", "your", "starting", "deck", "shares", "a", "card", "type",
    ],
    &[
        "your", "starting", "deck", "contains", "at", "least", "twenty", "cards", "more", "than",
        "the", "minimum", "deck", "size",
    ],
    &[
        "each",
        "permanent",
        "card",
        "in",
        "your",
        "starting",
        "deck",
        "has",
        "an",
        "activated",
        "ability",
    ],
];

fn condition_for(index: usize) -> ironsmith_core::CompanionDeckCondition {
    match index {
        0 => ironsmith_core::CompanionDeckCondition::OnlyManaValueParity {
            even: true,
            lands_are_exempt: false,
        },
        1 => ironsmith_core::CompanionDeckCondition::NoRepeatedManaSymbols,
        2 => ironsmith_core::CompanionDeckCondition::CreatureSubtypes(vec![
            Subtype::Cat,
            Subtype::Elemental,
            Subtype::Nightmare,
            Subtype::Dinosaur,
            Subtype::Beast,
        ]),
        3 => ironsmith_core::CompanionDeckCondition::NonlandManaValueAtLeast(3),
        4 => ironsmith_core::CompanionDeckCondition::PermanentManaValueAtMost(2),
        5 => ironsmith_core::CompanionDeckCondition::UniqueNonlandNames,
        6 => ironsmith_core::CompanionDeckCondition::OnlyManaValueParity {
            even: false,
            lands_are_exempt: true,
        },
        7 => ironsmith_core::CompanionDeckCondition::SharedNonlandCardType,
        8 => ironsmith_core::CompanionDeckCondition::CardsAboveMinimumDeckSize(20),
        9 => ironsmith_core::CompanionDeckCondition::PermanentsHaveActivatedAbility,
        _ => unreachable!("every condition shape has a constraint"),
    }
}

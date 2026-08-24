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

    if crate::word_primitives::parse_sequence_complete(
        condition,
        &[
            "your", "starting", "deck", "contains", "only", "cards", "with", "even", "mana",
            "values",
        ],
    ) {
        return Some(
            ironsmith_core::CompanionDeckCondition::OnlyManaValueParity {
                even: true,
                lands_are_exempt: false,
            },
        );
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
        &[
            "no", "card", "in", "your", "starting", "deck", "has", "more", "than", "one", "of",
            "the", "same", "mana", "symbol", "in", "its", "mana", "cost",
        ],
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::NoRepeatedManaSymbols);
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
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
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::CreatureSubtypes(
            vec![
                Subtype::Cat,
                Subtype::Elemental,
                Subtype::Nightmare,
                Subtype::Dinosaur,
                Subtype::Beast,
            ],
        ));
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
        &[
            "your", "starting", "deck", "contains", "only", "cards", "with", "mana", "value", "3",
            "or", "greater", "and", "land", "cards",
        ],
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::NonlandManaValueAtLeast(3));
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
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
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::PermanentManaValueAtMost(2));
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
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
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::UniqueNonlandNames);
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
        &[
            "your", "starting", "deck", "contains", "only", "cards", "with", "odd", "mana",
            "values", "and", "land", "cards",
        ],
    ) {
        return Some(
            ironsmith_core::CompanionDeckCondition::OnlyManaValueParity {
                even: false,
                lands_are_exempt: true,
            },
        );
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
        &[
            "each", "nonland", "card", "in", "your", "starting", "deck", "shares", "a", "card",
            "type",
        ],
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::SharedNonlandCardType);
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
        &[
            "your", "starting", "deck", "contains", "at", "least", "twenty", "cards", "more",
            "than", "the", "minimum", "deck", "size",
        ],
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::CardsAboveMinimumDeckSize(20));
    }
    if crate::word_primitives::parse_sequence_complete(
        condition,
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
    ) {
        return Some(ironsmith_core::CompanionDeckCondition::PermanentsHaveActivatedAbility);
    }
    None
}

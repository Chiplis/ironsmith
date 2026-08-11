use crate::runtime_backend::front_end::lexer::{OwnedLexToken, parser_token_word_refs};

use super::super::super::leaf;
use super::common;

const TOKEN_SACRIFICE_PREFIXES: &[&[&str]] = &[
    &["sacrifice", "it"],
    &["sacrifice", "them"],
    &["sacrifice", "that", "token"],
    &["sacrifice", "those", "tokens"],
];
const TOKEN_EXILE_PREFIXES: &[&[&str]] = &[&["exile", "it"], &["exile", "them"]];
const DELAYED_END_STEP_SACRIFICE_PREFIXES: &[&[&str]] = &[
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "end",
        "step",
        "sacrifice",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
        "sacrifice",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "next",
        "end",
        "step",
        "sacrifice",
    ],
];
const DELAYED_END_STEP_EXILE_PREFIXES: &[&[&str]] = &[
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "end",
        "step",
        "exile",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "the",
        "next",
        "end",
        "step",
        "exile",
    ],
    &[
        "at",
        "the",
        "beginning",
        "of",
        "next",
        "end",
        "step",
        "exile",
    ],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TokenCopyModifierKind {
    GainHasteUntilEndOfTurn,
    HasHaste,
    EnterTappedAndAttacking,
    EnterTappedAndAttackingThatPlayer,
    SacrificeAtNextEndStep,
    SacrificeAtNextUpkeep,
    ExileAtNextEndStep,
}

fn non_article_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    parser_token_word_refs(tokens)
        .into_iter()
        .filter(|word| leaf::parse_leaf_article_complete(word).is_err())
        .collect()
}

fn has_terminal_phrase(words: &[&str], phrase: &[&str]) -> bool {
    common::word_offset(words, phrase)
        .is_some_and(|start| start.saturating_add(phrase.len()) == words.len())
}

pub(crate) fn parse_token_copy_modifier_kind(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyModifierKind> {
    let words = non_article_words(tokens);
    if common::exact_any(
        &words,
        &[
            &["it", "gains", "haste", "until", "end", "of", "turn"],
            &["they", "gain", "haste", "until", "end", "of", "turn"],
            &[
                "that", "token", "gains", "haste", "until", "end", "of", "turn",
            ],
            &[
                "those", "tokens", "gain", "haste", "until", "end", "of", "turn",
            ],
            &["token", "gains", "haste", "until", "end", "of", "turn"],
            &["tokens", "gain", "haste", "until", "end", "of", "turn"],
        ],
    ) {
        return Some(TokenCopyModifierKind::GainHasteUntilEndOfTurn);
    }
    if common::exact_any(
        &words,
        &[
            &["it", "has", "haste"],
            &["they", "have", "haste"],
            &["that", "token", "gains", "haste"],
            &["those", "tokens", "gain", "haste"],
            &["token", "gains", "haste"],
            &["tokens", "gain", "haste"],
            &["token", "created", "this", "way", "has", "haste"],
            &["tokens", "created", "this", "way", "have", "haste"],
            &["token", "created", "this", "way", "gains", "haste"],
            &["tokens", "created", "this", "way", "gain", "haste"],
        ],
    ) {
        return Some(TokenCopyModifierKind::HasHaste);
    }
    if common::exact_any(
        &words,
        &[
            &["it", "enters", "tapped", "and", "attacking"],
            &["they", "enter", "tapped", "and", "attacking"],
            &["token", "enters", "tapped", "and", "attacking"],
            &["tokens", "enter", "tapped", "and", "attacking"],
            &[
                "token",
                "created",
                "this",
                "way",
                "enters",
                "tapped",
                "and",
                "attacking",
            ],
            &[
                "tokens",
                "created",
                "this",
                "way",
                "enter",
                "tapped",
                "and",
                "attacking",
            ],
        ],
    ) {
        return Some(TokenCopyModifierKind::EnterTappedAndAttacking);
    }
    // A named token's follow-up uses its name as the subject: "Ragavan
    // enters tapped and attacking." The applier only binds this to an
    // immediately preceding token creation, so a bare single-word subject is
    // unambiguous here.
    if words.len() == 5 && words[1..] == ["enters", "tapped", "and", "attacking"] {
        return Some(TokenCopyModifierKind::EnterTappedAndAttacking);
    }
    if common::exact_any(
        &words,
        &[
            &[
                "it",
                "enters",
                "tapped",
                "and",
                "attacking",
                "that",
                "player",
            ],
            &[
                "token",
                "enters",
                "tapped",
                "and",
                "attacking",
                "that",
                "player",
            ],
            &[
                "tokens",
                "enter",
                "tapped",
                "and",
                "attacking",
                "that",
                "player",
            ],
        ],
    ) {
        return Some(TokenCopyModifierKind::EnterTappedAndAttackingThatPlayer);
    }

    if common::prefix_any(&words, TOKEN_SACRIFICE_PREFIXES)
        && has_terminal_phrase(&words, &["at", "beginning", "of", "next", "end", "step"])
    {
        return Some(TokenCopyModifierKind::SacrificeAtNextEndStep);
    }
    if common::prefix_any(&words, TOKEN_SACRIFICE_PREFIXES)
        && has_terminal_phrase(&words, &["next", "upkeep"])
    {
        return Some(TokenCopyModifierKind::SacrificeAtNextUpkeep);
    }
    if common::prefix_any(&words, TOKEN_EXILE_PREFIXES)
        && has_terminal_phrase(&words, &["at", "beginning", "of", "next", "end", "step"])
    {
        return Some(TokenCopyModifierKind::ExileAtNextEndStep);
    }
    if common::prefix_any(&words, DELAYED_END_STEP_SACRIFICE_PREFIXES) {
        return Some(TokenCopyModifierKind::SacrificeAtNextEndStep);
    }
    if common::prefix_any(&words, DELAYED_END_STEP_EXILE_PREFIXES) {
        return Some(TokenCopyModifierKind::ExileAtNextEndStep);
    }
    None
}

#[cfg(test)]
mod tests {
    use crate::runtime_backend::front_end::lexer::lex_line;

    use super::*;

    #[test]
    fn token_copy_modifier_returns_typed_followup_kind() {
        let haste = lex_line("It gains haste until end of turn.", 0).expect("lex fixture");
        assert_eq!(
            parse_token_copy_modifier_kind(&haste),
            Some(TokenCopyModifierKind::GainHasteUntilEndOfTurn)
        );
        let plural_haste =
            lex_line("Those tokens gain haste.", 0).expect("plural token haste fixture");
        assert_eq!(
            parse_token_copy_modifier_kind(&plural_haste),
            Some(TokenCopyModifierKind::HasHaste)
        );
        let temporary_plural_haste = lex_line("Those tokens gain haste until end of turn.", 0)
            .expect("temporary plural token haste fixture");
        assert_eq!(
            parse_token_copy_modifier_kind(&temporary_plural_haste),
            Some(TokenCopyModifierKind::GainHasteUntilEndOfTurn)
        );

        let sacrifice = lex_line("Sacrifice it at the beginning of the next end step.", 0)
            .expect("lex fixture");
        assert_eq!(
            parse_token_copy_modifier_kind(&sacrifice),
            Some(TokenCopyModifierKind::SacrificeAtNextEndStep)
        );

        let conditional = lex_line(
            "Sacrifice it at the beginning of the next end step if it has mana value 3 or less.",
            0,
        )
        .expect("conditional delayed sacrifice fixture");
        assert_eq!(
            parse_token_copy_modifier_kind(&conditional),
            None,
            "a behavior-bearing suffix must be parsed by the delayed-action grammar"
        );

        let attacking =
            lex_line("The token enters tapped and attacking that player.", 0).expect("lex fixture");
        assert_eq!(
            parse_token_copy_modifier_kind(&attacking),
            Some(TokenCopyModifierKind::EnterTappedAndAttackingThatPlayer)
        );
    }
}

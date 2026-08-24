use super::*;

pub fn parse_token_copy_modifier_kind(tokens: &[OwnedLexToken]) -> Option<TokenCopyModifierKind> {
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
            &["it", "gains", "haste"],
            &["they", "have", "haste"],
            &["they", "gain", "haste"],
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
    if words.len() == 5
        && crate::word_primitives::parse_sequence_complete(
            &words[1..],
            &["enters", "tapped", "and", "attacking"],
        )
    {
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

use super::*;

pub fn is_negated_untap_clause(words: &[&str]) -> bool {
    effect_grammar::is_negated_untap_clause_words(words)
}

pub fn parse_token_copy_modifier_sentence(tokens: &[OwnedLexToken]) -> Option<TokenCopyFollowup> {
    parse_token_copy_modifier_sentence_lexed(tokens)
}

pub fn parse_token_copy_modifier_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    use effect_grammar::labeled_dispatch::TokenCopyModifierKind as Kind;

    Some(
        match effect_grammar::labeled_dispatch::parse_token_copy_modifier_kind(tokens)? {
            Kind::GainHasteUntilEndOfTurn => TokenCopyFollowup::GainHasteUntilEndOfTurn(
                token_copy_leading_reference_surface(tokens)?,
            ),
            Kind::HasHaste => {
                TokenCopyFollowup::HasHaste(token_copy_leading_reference_surface(tokens)?)
            }
            Kind::EnterTappedAndAttacking => TokenCopyFollowup::EnterTappedAndAttacking,
            Kind::EnterTappedAndAttackingThatPlayer => {
                TokenCopyFollowup::EnterTappedAndAttackingThatPlayer
            }
            Kind::SacrificeAtNextEndStep => TokenCopyFollowup::SacrificeAtNextEndStep(
                token_copy_action_reference_surface(tokens, "sacrifice")?,
            ),
            Kind::SacrificeAtNextUpkeep => TokenCopyFollowup::SacrificeAtNextUpkeep,
            Kind::ExileAtNextEndStep => TokenCopyFollowup::ExileAtNextEndStep(
                token_copy_action_reference_surface(tokens, "exile")?,
            ),
        },
    )
}

use crate::effect::TokenCopyReferenceSurface as Surface;

/// The authored token references, longest phrase first so that a longer
/// reference ("the token created this way") is never read as its prefix
/// ("the token"); each phrase group names one surface.
const TOKEN_REFERENCE_SURFACES: &[(&[&[&str]], Surface)] = &[
    (
        &[
            &["the", "token", "created", "this", "way"],
            &["token", "created", "this", "way"],
        ],
        Surface::TokenCreatedThisWay,
    ),
    (
        &[
            &["the", "tokens", "created", "this", "way"],
            &["tokens", "created", "this", "way"],
        ],
        Surface::TokensCreatedThisWay,
    ),
    (&[&["that", "token"]], Surface::ThatToken),
    (
        &[&["those", "tokens"], &["those", "token"]],
        Surface::ThoseTokens,
    ),
    (&[&["the", "token"], &["token"]], Surface::TheToken),
    (&[&["the", "tokens"], &["tokens"]], Surface::TheTokens),
];

fn token_copy_reference_surface_at(
    words: &[&str],
    start: usize,
) -> Option<crate::effect::TokenCopyReferenceSurface> {
    let words = words.get(start..)?;
    for (phrases, surface) in TOKEN_REFERENCE_SURFACES {
        if crate::word_primitives::parse_any_sequence_prefix(words, phrases) {
            return Some(*surface);
        }
    }
    match words.first().copied()? {
        "it" => Some(Surface::It),
        "they" | "them" => Some(Surface::They),
        _ => None,
    }
}

pub fn token_copy_leading_reference_surface(
    tokens: &[OwnedLexToken],
) -> Option<crate::effect::TokenCopyReferenceSurface> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    token_copy_reference_surface_at(&words, 0)
}

pub fn token_copy_action_reference_surface(
    tokens: &[OwnedLexToken],
    action: &str,
) -> Option<crate::effect::TokenCopyReferenceSurface> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let action_idx = crate::slice_primitives::select_last_position(&words, |word| *word == action)?;
    token_copy_reference_surface_at(&words, action_idx + 1)
}

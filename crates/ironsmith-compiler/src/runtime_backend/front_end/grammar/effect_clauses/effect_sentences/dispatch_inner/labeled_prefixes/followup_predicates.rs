use super::*;

pub(crate) fn is_negated_untap_clause(words: &[&str]) -> bool {
    effect_grammar::is_negated_untap_clause_words(words)
}

pub(crate) fn parse_token_copy_modifier_sentence(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    parse_token_copy_modifier_sentence_lexed(tokens)
}

pub(crate) fn parse_token_copy_modifier_sentence_lexed(
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

fn token_copy_reference_surface_at(
    words: &[&str],
    start: usize,
) -> Option<crate::effect::TokenCopyReferenceSurface> {
    use crate::effect::TokenCopyReferenceSurface as Surface;

    let words = words.get(start..)?;
    if words.starts_with(&["the", "token", "created", "this", "way"])
        || words.starts_with(&["token", "created", "this", "way"])
    {
        return Some(Surface::TokenCreatedThisWay);
    }
    if words.starts_with(&["the", "tokens", "created", "this", "way"])
        || words.starts_with(&["tokens", "created", "this", "way"])
    {
        return Some(Surface::TokensCreatedThisWay);
    }
    if words.starts_with(&["that", "token"]) {
        return Some(Surface::ThatToken);
    }
    if words.starts_with(&["those", "tokens"]) || words.starts_with(&["those", "token"]) {
        return Some(Surface::ThoseTokens);
    }
    if words.starts_with(&["the", "token"]) || words.starts_with(&["token"]) {
        return Some(Surface::TheToken);
    }
    if words.starts_with(&["the", "tokens"]) || words.starts_with(&["tokens"]) {
        return Some(Surface::TheTokens);
    }
    match words.first().copied()? {
        "it" => Some(Surface::It),
        "they" | "them" => Some(Surface::They),
        _ => None,
    }
}

pub(crate) fn token_copy_leading_reference_surface(
    tokens: &[OwnedLexToken],
) -> Option<crate::effect::TokenCopyReferenceSurface> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    token_copy_reference_surface_at(&words, 0)
}

pub(crate) fn token_copy_action_reference_surface(
    tokens: &[OwnedLexToken],
    action: &str,
) -> Option<crate::effect::TokenCopyReferenceSurface> {
    let words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    let action_idx = words.iter().rposition(|word| *word == action)?;
    token_copy_reference_surface_at(&words, action_idx + 1)
}

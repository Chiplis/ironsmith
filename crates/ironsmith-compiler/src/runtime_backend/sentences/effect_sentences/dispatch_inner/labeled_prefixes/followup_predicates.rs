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
            Kind::GainHasteUntilEndOfTurn => TokenCopyFollowup::GainHasteUntilEndOfTurn,
            Kind::HasHaste => TokenCopyFollowup::HasHaste,
            Kind::EnterTappedAndAttacking => TokenCopyFollowup::EnterTappedAndAttacking,
            Kind::SacrificeAtNextEndStep => TokenCopyFollowup::SacrificeAtNextEndStep,
            Kind::SacrificeAtNextUpkeep => TokenCopyFollowup::SacrificeAtNextUpkeep,
            Kind::ExileAtNextEndStep => TokenCopyFollowup::ExileAtNextEndStep,
        },
    )
}

//! Complete combat-participant predicates. No card-name dispatch and no
//! acceptance of an unconsumed qualification on a known prefix.
use super::*;
use ironsmith_core::CombatParticipantCondition as Combat;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if tokens.iter().any(|token| token.as_word().is_none()) { return None; }
    let words = crate::lexer::token_word_refs(tokens);
    let condition = match words.as_slice() {
        ["youre" | "you're", "the", "defending", "player"]
        | ["you", "are", "the", "defending", "player"] => Combat::YouAreDefendingPlayer,
        ["they", "attacked", "you", "and/or", "a", "planeswalker", "you", "control"] =>
            Combat::AttackingPlayerAttackedYouOrYourPlaneswalker,
        ["they", "arent" | "aren't", "attacking", "you"]
        | ["they", "are", "not", "attacking", "you"] => Combat::AttackingPlayerIsNotAttackingYou,
        ["one", "or", "more", "players", "being", "attacked", "are", "poisoned"] =>
            Combat::AnyAttackedPlayerIsPoisoned,
        ["its" | "it's", "attacking", "the", "player", "with", "the", "most", "life", "or", "tied", "for", "most", "life"] =>
            Combat::TriggeringCreatureAttacksMostLifePlayer,
        _ => return None,
    };
    Some(PredicateAst::Triggering(TriggeringPredicateAst::CombatParticipant(condition)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn combat_participant_readings_consume_the_whole_predicate() {
        for text in [
            "you're the defending player",
            "they attacked you and/or a planeswalker you control",
            "they aren't attacking you",
            "one or more players being attacked are poisoned",
            "it's attacking the player with the most life or tied for most life",
        ] {
            assert!(parse(&crate::lexer::lex_line(text, 0).unwrap()).is_some(), "{text}");
            assert!(parse(&crate::lexer::lex_line(&format!("{text} except during your turn"), 0).unwrap()).is_none());
            assert!(parse(&crate::lexer::lex_line(&format!("{text} {{3}}"), 0).unwrap()).is_none());
        }
        assert!(parse(&crate::lexer::lex_line("they attacked you and/or a battle you protect", 0).unwrap()).is_none());
        assert!(parse(&crate::lexer::lex_line("they aren't attacking you {T}", 0).unwrap()).is_none());
    }
}

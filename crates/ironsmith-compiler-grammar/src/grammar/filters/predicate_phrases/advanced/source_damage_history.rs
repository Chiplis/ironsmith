//! "as long as it hasn't dealt damage yet" (Karakyk Guardian, Palladia-Mors):
//! the source permanent's damage history since it entered the battlefield.
use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if tokens.iter().any(|token| token.as_word().is_none()) {
        return None;
    }
    let words = crate::lexer::token_word_refs(tokens);
    let subject_len = match words.as_slice() {
        // "they" is the singular pronoun of a two-headed legend's name
        // ("Ruric Thar has hexproof as long as they haven't dealt ...").
        ["it" | "they", ..] => 1,
        ["this", "creature" | "permanent", ..] => 2,
        _ => return None,
    };
    let dealt = PredicateAst::Source(SourcePredicateAst::SourceHasDealtDamageSinceEntered);
    let dealt_combat =
        PredicateAst::Source(SourcePredicateAst::SourceHasDealtCombatDamageSinceEntered);
    match &words[subject_len..] {
        ["hasnt" | "hasn't" | "havent" | "haven't", "dealt", "damage", "yet"]
        | ["has" | "have", "not", "dealt", "damage", "yet"] => {
            Some(PredicateAst::Not(Box::new(dealt)))
        }
        ["hasnt" | "hasn't" | "havent" | "haven't", "dealt", "combat", "damage", "yet"]
        | ["has" | "have", "not", "dealt", "combat", "damage", "yet"] => {
            Some(PredicateAst::Not(Box::new(dealt_combat)))
        }
        ["has" | "have", "dealt", "damage"] => Some(dealt),
        ["has" | "have", "dealt", "combat", "damage"] => Some(dealt_combat),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_history_predicate_reads_the_whole_clause() {
        for text in ["it hasn't dealt damage yet", "this creature hasn't dealt damage yet"] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(matches!(parse(&tokens), Some(PredicateAst::Not(_))), "{text}");
        }
        let tokens = crate::lexer::lex_line("it hasn't dealt combat damage yet", 0).unwrap();
        assert!(
            matches!(parse(&tokens), Some(PredicateAst::Not(inner)) if matches!(*inner,
                PredicateAst::Source(SourcePredicateAst::SourceHasDealtCombatDamageSinceEntered))),
            "combat-only history is its own fact"
        );
    }
}

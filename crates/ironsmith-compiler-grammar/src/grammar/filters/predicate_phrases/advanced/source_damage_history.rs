//! "as long as it hasn't dealt damage yet" (Karakyk Guardian, Palladia-Mors):
//! the source permanent's damage history since it entered the battlefield.
use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if tokens.iter().any(|token| token.as_word().is_none()) {
        return None;
    }
    let words = crate::lexer::token_word_refs(tokens);
    let subject_len = match words.as_slice() {
        ["it", ..] | ["this", "creature" | "permanent", ..] => {
            if words[0] == "it" { 1 } else { 2 }
        }
        _ => return None,
    };
    let dealt = PredicateAst::Source(SourcePredicateAst::SourceHasDealtDamageSinceEntered);
    match &words[subject_len..] {
        ["hasnt" | "hasn't", "dealt", "damage", "yet"]
        | ["has", "not", "dealt", "damage", "yet"] => Some(PredicateAst::Not(Box::new(dealt))),
        ["has", "dealt", "damage"] => Some(dealt),
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
        assert!(parse(&tokens).is_none(), "combat-only history is a different fact");
    }
}

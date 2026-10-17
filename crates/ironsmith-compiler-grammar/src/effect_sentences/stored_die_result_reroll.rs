//! "You may reroll any number of this creature's stored results." (Centaur of
//! Attention): the controller chooses any number of the die results stored on
//! the source, from none to all, and rerolls them. "May" adds nothing to a
//! choice that already includes none.
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::{OwnedLexToken, parser_token_word_refs};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = parser_token_word_refs(tokens);
    let rest = words
        .strip_prefix(&["you", "may"][..])
        .unwrap_or(words.as_slice());
    let Some(rest) = rest.strip_prefix(&["reroll", "any", "number", "of"][..]) else {
        return Ok(None);
    };
    let Some(reference) = rest.strip_suffix(&["stored", "results"][..]) else {
        return Ok(None);
    };
    let names_source = matches!(
        reference,
        ["its"]
            | ["this", "creature's" | "creatures" | "permanent's" | "permanents"]
            | ["this", "creature" | "permanent", "s"]
    );
    if !names_source {
        return Ok(None);
    }
    Ok(Some(EffectAst::RerollStoredDieResults(
        ironsmith_core::RerollStoredDieResultsEffect::new(crate::target::PlayerFilter::You),
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_stored_result_reroll() {
        let tokens = crate::lexer::lex_line(
            "you may reroll any number of this creature's stored results.",
            0,
        )
        .unwrap();
        assert!(matches!(
            parse(&tokens).unwrap(),
            Some(EffectAst::RerollStoredDieResults(_))
        ));
        let tokens = crate::lexer::lex_line("you may reroll any number of dice.", 0).unwrap();
        assert_eq!(parse(&tokens).unwrap(), None);
    }
}

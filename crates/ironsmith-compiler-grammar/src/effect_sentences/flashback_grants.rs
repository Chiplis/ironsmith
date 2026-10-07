//! Cost-bearing flashback grants share the printed keyword's cost parser.
//! A grant is a permission on the targeted card in its current zone; it is
//! never an activated ability and never an extra cost on the source.

use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::{OwnedLexToken, TokenKind};

pub(crate) fn fixed_grant_parts(
    tokens: &[OwnedLexToken],
) -> Result<Option<(&[OwnedLexToken], crate::model::CompilerAlternativeCastingMethod)>, CardTextError> {
    let Some(gain) = tokens.iter().position(|token| token.is_any_word(&["gain", "gains"])) else {
        return Ok(None);
    };
    if gain == 0 || !tokens.get(gain + 1).is_some_and(|token| token.is_word("flashback")) {
        return Ok(None);
    }
    let Some(until) = crate::slice_primitives::find_window_by(tokens, 4, |window| {
        window[0].is_word("until") && window[1].is_word("end")
            && window[2].is_word("of") && window[3].is_word("turn")
    }) else { return Ok(None); };
    if until <= gain + 2 || tokens[until + 4..].iter().any(|token| !matches!(token.kind, TokenKind::Period)) {
        return Ok(None);
    }
    let Some(method) = crate::util::parse_flashback_line(&tokens[gain + 1..until])? else {
        return Ok(None);
    };
    Ok(Some((&tokens[..gain], method)))
}

pub(crate) fn parse_fixed_flashback_grant(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((subject, method)) = fixed_grant_parts(tokens)? else { return Ok(None); };
    Ok(Some(vec![EffectAst::subject_verb_grant_to_target(
        super::parse_target_phrase(subject)?,
        crate::model::CompilerGrantableCore::AlternativeCast(method),
        crate::grant::GrantDuration::UntilEndOfTurn,
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_price_keeps_exact_cost_target_zone_and_duration() {
        let tokens = crate::lexer::lex_line("Target instant or sorcery card in your graveyard gains flashback {2}{R}{G} until end of turn.", 0).unwrap();
        let (subject, method) = fixed_grant_parts(&tokens).unwrap().unwrap();
        assert!(crate::lexer::token_word_refs(subject).ends_with(&["in", "your", "graveyard"]));
        assert_eq!(method.mana_cost().unwrap().to_oracle(), "{2}{R}{G}");
        let debug = format!("{:?}", parse_fixed_flashback_grant(&tokens).unwrap().unwrap());
        assert!(debug.contains("Flashback") && debug.contains("Graveyard") && debug.contains("UntilEndOfTurn"), "{debug}");
        for text in [
            "Target card gains flashback until end of turn.",
            "Target card gains flashback {2}{G} until your next turn.",
            "Target card gains flashback {2}{G} until end of turn if it is a creature.",
        ] {
            assert!(fixed_grant_parts(&crate::lexer::lex_line(text, 0).unwrap()).unwrap().is_none(), "{text}");
        }
    }
}

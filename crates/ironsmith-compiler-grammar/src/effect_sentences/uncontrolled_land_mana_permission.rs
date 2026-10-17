//! "Until end of turn, you may tap lands you don't control for mana."
//! (Piracy): a this-turn player permission to activate the {T} mana
//! abilities of lands controlled by other players. A following "Spend this
//! mana only to ..." sentence wraps it like any mana-producing instruction.
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::{OwnedLexToken, parser_token_word_refs};

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = parser_token_word_refs(tokens);
    const BODY: &[&str] = &["you", "may", "tap", "lands", "you"];
    const TAIL: &[&str] = &["control", "for", "mana"];
    let body = words
        .strip_prefix(&["until", "end", "of", "turn"][..])
        .or_else(|| words.strip_prefix(&["until", "the", "end", "of", "turn"][..]))
        .map(|rest| (rest, true))
        .unwrap_or((words.as_slice(), false));
    let (rest, leading_duration) = body;
    let rest = if leading_duration {
        rest
    } else {
        let Some(rest) = rest.strip_suffix(&["this", "turn"][..]) else {
            return Ok(None);
        };
        rest
    };
    let Some(rest) = rest.strip_prefix(BODY) else {
        return Ok(None);
    };
    let Some(rest) = rest
        .strip_prefix(&["dont"][..])
        .or_else(|| rest.strip_prefix(&["don't"][..]))
        .or_else(|| rest.strip_prefix(&["do", "not"][..]))
    else {
        return Ok(None);
    };
    if rest != TAIL {
        return Ok(None);
    }
    Ok(Some(EffectAst::GrantTapUncontrolledLandsForMana))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_this_turn_uncontrolled_land_mana_permission() {
        for text in [
            "Until end of turn, you may tap lands you don't control for mana.",
            "You may tap lands you don't control for mana this turn.",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert_eq!(
                parse(&tokens).unwrap(),
                Some(EffectAst::GrantTapUncontrolledLandsForMana),
                "{text}"
            );
        }
        let tokens = crate::lexer::lex_line("You may tap lands you don't control for mana.", 0).unwrap();
        assert_eq!(parse(&tokens).unwrap(), None, "an undated permission is not this reading");
    }
}

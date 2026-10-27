use super::*;

/// "The next time target land would be destroyed this turn, remove all
/// damage marked on it instead." (Pyramids): a one-shot replacement of the
/// target's next destruction this turn by removing its damage. It is not
/// regeneration (no tap, no removal from combat).
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let full_recovery = [
        &["would", "be", "destroyed", "this", "turn", "instead", "tap", "it", "remove", "it", "from", "combat", "and", "heal", "all", "damage", "on", "it"][..],
        &["would", "be", "destroyed", "this", "turn", "instead", "tap", "it", "remove", "it", "from", "combat", "and", "remove", "all", "damage", "marked", "on", "it"][..],
    ].iter().any(|tail| words.ends_with(tail));
    if words.starts_with(&["the", "next", "time"]) && full_recovery {
        let time = tokens.iter().position(|token| token.is_word("time")).unwrap();
        let would = tokens.iter().position(|token| token.is_word("would")).unwrap();
        if would > time + 1 {
            let target = crate::util::parse_target_phrase(&tokens[time + 1..would])?;
            return Ok(Some(EffectAst::subject_verb_destruction_recovery(target)));
        }
    }
    if !words.starts_with(&["the", "next", "time"])
        || !words.ends_with(&[
            "would", "be", "destroyed", "this", "turn", "remove", "all", "damage", "marked", "on",
            "it", "instead",
        ])
    {
        return Ok(None);
    }
    let Some(time) = tokens.iter().position(|token| token.is_word("time")) else {
        return Ok(None);
    };
    let Some(would) = tokens.iter().position(|token| token.is_word("would")) else {
        return Ok(None);
    };
    if would <= time + 1 {
        return Ok(None);
    }
    let subject = &tokens[time + 1..would];
    if !subject.iter().any(|token| token.is_word("target")) {
        return Ok(None);
    }
    let target = crate::util::parse_target_phrase(subject)?;
    Ok(Some(EffectAst::subject_verb_destruction_damage_removal_shield(target)))
}

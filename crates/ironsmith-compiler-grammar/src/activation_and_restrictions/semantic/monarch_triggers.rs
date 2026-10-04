use super::*;
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Option<TriggerSpec> {
    let words = crate::lexer::token_word_refs(tokens);
    let subject = words
        .strip_suffix(&["become", "the", "monarch"])
        .or_else(|| words.strip_suffix(&["becomes", "the", "monarch"]))
        .or_else(|| words.strip_suffix(&["become", "monarch"]))
        .or_else(|| words.strip_suffix(&["becomes", "monarch"]))?;
    let player = match subject {
        ["you"] => PlayerFilter::You,
        ["an", "opponent"] | ["opponent"] => PlayerFilter::Opponent,
        ["a" | "any", "player"] => PlayerFilter::Any,
        ["that", "player"] => PlayerFilter::IteratedPlayer,
        _ => return None,
    };
    Some(TriggerSpec::PlayerBecomesMonarch(player))
}

use super::*;

pub(super) fn parse_loyalty_ability_trigger_tail_lexed(
    tail_tokens: &[OwnedLexToken],
    tail_words: &[&str],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(tail) = trigger_grammar::parse_loyalty_ability_tail(tail_tokens) else {
        return Ok(None);
    };
    let owner_tokens = &tail_tokens[tail.owner];
    let owner_filter = parse_object_filter_lexed(owner_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported loyalty-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    Ok(Some(owner_filter))
}

/// Split a trailing "or (a) player(s)" from a counter-recipient phrase
/// ("Whenever you put one or more counters on a permanent or player").
pub(super) fn split_counter_recipient_or_player(
    tokens: &[OwnedLexToken],
) -> (&[OwnedLexToken], bool) {
    let words = crate::lexer::token_word_refs(tokens);
    if words.len() != tokens.len() {
        return (tokens, false);
    }
    for suffix in [
        &["or", "a", "player"][..],
        &["or", "player"][..],
        &["or", "players"][..],
    ] {
        if words.len() > suffix.len() && &words[words.len() - suffix.len()..] == suffix {
            return (&tokens[..tokens.len() - suffix.len()], true);
        }
    }
    (tokens, false)
}

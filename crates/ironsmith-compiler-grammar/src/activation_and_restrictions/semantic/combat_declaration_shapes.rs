//! Complete combat-declaration clauses with a shared, typed participant.
use super::*;

fn subject(tokens: &[OwnedLexToken]) -> Result<ObjectFilter, CardTextError> {
    if let Some(filter) = parse_attack_trigger_subject_filter_lexed(tokens)? {
        return Ok(filter);
    }
    let words = crate::lexer::token_word_refs(tokens);
    if is_source_reference_words(&words)
        || source_reference_surface_for_trigger_subject(tokens).is_some()
    {
        return Ok(ObjectFilter::source());
    }
    Err(CardTextError::ParseError(
        "unknown combat-event participant".into(),
    ))
}
pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<TriggerSpec>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    if let Some(prefix) = words.strip_suffix(&["attacks", "a", "player", "alone"]) {
        if prefix.is_empty() {
            return Ok(None);
        }
        let end = trigger_word_token_start(tokens, prefix.len()).unwrap_or(tokens.len());
        return Ok(Some(TriggerSpec::AttacksPlayerAlone(subject(
            &tokens[..end],
        )?)));
    }
    if words.starts_with(&["one", "or", "more"]) {
        let (prefix, fight) =
            if let Some(prefix) = words.strip_suffix(&["fight", "or", "become", "blocked"]) {
                (prefix, true)
            } else if let Some(prefix) = words.strip_suffix(&["become", "blocked"]) {
                (prefix, false)
            } else {
                return Ok(None);
            };
        if prefix.len() <= 3 {
            return Ok(None);
        }
        let start = trigger_word_token_start(tokens, 3).unwrap_or(tokens.len());
        let end = trigger_word_token_start(tokens, prefix.len()).unwrap_or(tokens.len());
        let filter = subject(&tokens[start..end])?;
        let blocked = TriggerSpec::BecomesBlockedOneOrMore(filter.clone());
        return Ok(Some(if fight {
            TriggerSpec::Either(
                Box::new(TriggerSpec::KeywordActionOneOrMore {
                    action: crate::events::KeywordActionKind::Fight,
                    player: PlayerFilter::Any,
                    source_filter: filter,
                }),
                Box::new(blocked),
            )
        } else {
            blocked
        }));
    }
    // A per-pair relation owns both object filters. An "or" within the
    // blocked object's color/type filter is not an alternative event arm.
    if let Some(blocks) = words.iter().position(|word| *word == "blocks")
        && blocks > 0
        && blocks + 1 < words.len()
        && !words[blocks + 1..]
            .iter()
            .any(|word| matches!(*word, "blocks" | "becomes" | "power" | "turn" | "combat"))
        && words[blocks + 1] != "or"
        && !words[..blocks].contains(&"or")
    {
        let end = trigger_word_token_start(tokens, blocks).unwrap_or(tokens.len());
        let start = trigger_word_token_start(tokens, blocks + 1).unwrap_or(tokens.len());
        let blocker = subject(&tokens[..end])?;
        // Leave self and quantified forms to their established CR509 owners.
        if blocker.source || has_leading_one_or_more(&tokens[start..]) {
            return Ok(None);
        }
        let blocked = parse_object_filter_lexed(&tokens[start..], false)?;
        return Ok(Some(TriggerSpec::BlocksObject { blocker, blocked }));
    }
    Ok(None)
}

//! Active-voice causal events keep the causing controller separate from the
//! affected spell, permanent, card, or player. The cause is event-time data.
use super::*;

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<TriggerSpec>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let Some(rest) = words.strip_prefix(&["a", "spell", "or", "ability"]) else { return Ok(None); };
    let (controller, actor_len) = if rest.starts_with(&["you", "control"]) { (PlayerFilter::You, 2) }
        else if rest.starts_with(&["an", "opponent", "controls"]) { (PlayerFilter::Opponent, 3) }
        else if rest.starts_with(&["a", "player", "controls"]) { (PlayerFilter::Any, 3) }
        else { return Ok(None); };
    let verb = 4 + actor_len;
    let suffix = &words[verb..];
    let word_positions = crate::lexer::parser_token_word_positions(tokens);
    let after = |count: usize| -> &[OwnedLexToken] {
        &tokens[word_positions.get(verb + count).map(|(index, _)| *index).unwrap_or(tokens.len())..]
    };
    if suffix.starts_with(&["causes", "you", "to", "discard"]) {
        let card_words = &suffix[4..];
        let (filter, one_or_more) = match card_words {
            ["a", "card"] => (None, false),
            ["cards"] | ["one", "or", "more", "cards"] => (None, true),
            ["this", "card"] => (Some(ObjectFilter::source()), false),
            _ => return Ok(None),
        };
        return Ok(Some(TriggerSpec::PlayerDiscardsCard {
            player: PlayerFilter::You, filter, cause_controller: Some(controller),
            effect_like_only: true, one_or_more,
        }));
    }
    let trigger = if suffix.first() == Some(&"counters") {
        if suffix[1..] != ["a", "spell"] { return Ok(None); }
        TriggerSpec::SpellCountered { filter: None, controller: PlayerFilter::Any }
    } else if suffix.first() == Some(&"destroys") {
        let target_tokens = after(1);
        if target_tokens.is_empty() { return Ok(None); }
        let filter = parse_object_filter_lexed(target_tokens, false)?;
        if filter == ObjectFilter::default() { return Err(CardTextError::ParseError("unqualified destruction subject".into())); }
        TriggerSpec::PermanentDestroyed(filter)
    } else { return Ok(None); };
    let actor = match controller { PlayerFilter::You => "you control", PlayerFilter::Opponent => "an opponent controls", _ => "a player controls" };
    Ok(Some(TriggerSpec::ConditionQualified {
        trigger: Box::new(trigger),
        condition: crate::cards::builders::PredicateAst::Triggering(
            crate::cards::builders::TriggeringPredicateAst::TriggeringEventCausedBy { controller, effect_like_only: true }),
        surface: format!("by a spell or ability {actor}"),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cause_role_is_not_the_countered_spell_controller_and_grouped_discard_keeps_cause() {
        for (text, expected) in [
            ("a spell or ability you control counters a spell", "SpellCountered"),
            ("a spell or ability an opponent controls destroys a noncreature permanent you control", "PermanentDestroyed"),
        ] {
            let parsed = parse(&crate::lexer::lex_line(text,0).unwrap()).unwrap().unwrap();
            let TriggerSpec::ConditionQualified { trigger, condition, .. } = parsed else { panic!("event-time qualification"); };
            assert!(format!("{trigger:?}").contains(expected)); assert!(format!("{condition:?}").contains("TriggeringEventCausedBy"));
        }
        let parsed = parse(&crate::lexer::lex_line("a spell or ability an opponent controls causes you to discard cards",0).unwrap()).unwrap().unwrap();
        assert!(matches!(parsed, TriggerSpec::PlayerDiscardsCard { one_or_more: true, cause_controller: Some(PlayerFilter::Opponent), effect_like_only: true, .. }));
        assert!(parse(&crate::lexer::lex_line("a spell or ability an unknown player controls counters a spell",0).unwrap()).unwrap().is_none());
    }
}

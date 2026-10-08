use super::*;

/// Exact normalized source-zone clause shared with the cost-prefix owner.
pub fn parse_source_command_or_battlefield_condition_tokens(tokens: &[OwnedLexToken]) -> bool {
    if !tokens.iter().all(|token| token.as_word().is_some()) { return false; }
    let words = crate::lexer::parser_token_word_refs(tokens);
    let Some(is) = words.iter().position(|word| *word == "is") else { return false; };
    words.first() == Some(&"this")
        && crate::util::is_source_reference_words(&words[..is])
        && words[is + 1..] == ["in", "the", "command", "zone", "or", "on", "the", "battlefield"]
}

pub fn parse_static_functional_zones_tokens(tokens: &[OwnedLexToken]) -> Option<Vec<Zone>> {
    let body = crate::grammar::document_shapes::parse_statement_label_strip_tokens(tokens).body_tokens;
    if let Some(prefix) = crate::grammar::abilities::split_as_long_as_condition_prefix_lexed(body)
        && parse_source_command_or_battlefield_condition_tokens(prefix.condition_tokens)
    {
        return Some(vec![Zone::Command, Zone::Battlefield]);
    }
    if has_any_phrase(tokens, SOURCE_NOT_ON_BATTLEFIELD_PHRASES) {
        return Some(vec![
            Zone::Hand,
            Zone::Stack,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Library,
            Zone::Command,
        ]);
    }
    if has_any_phrase(tokens, STATIC_LIBRARY_SEARCH_ZONE_PHRASES)
        && has_phrase(tokens, FROM_YOUR_LIBRARY_PHRASE)
    {
        return Some(vec![Zone::Library]);
    }
    if has_any_phrase(tokens, CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PHRASES) {
        return Some(vec![Zone::Graveyard]);
    }
    if has_any_phrase(tokens, CAST_OR_PLAY_SELF_FROM_EXILE_PHRASES) {
        return Some(vec![Zone::Exile]);
    }
    if has_phrase(tokens, CAUSES_YOU_TO_DISCARD_THIS_CARD_PHRASE)
        && has_phrase(tokens, INSTEAD_OF_PUTTING_IT_INTO_YOUR_GRAVEYARD_PHRASE)
    {
        return Some(vec![Zone::Hand]);
    }

    let zones = STATIC_ZONE_HINT_PHRASES
        .iter()
        .filter(|(phrase, _)| has_phrase(tokens, phrase))
        .map(|(_, zone)| *zone)
        .collect::<Vec<_>>();
    (!zones.is_empty()).then_some(zones)
}

#[cfg(test)]
mod command_zone_scope_tests {
    use super::*;
    #[test]
    fn source_clause_is_consumed_completely() {
        for source in ["this", "this creature", "this permanent", "this card"] {
            let tokens = crate::lexer::lex_line(&format!("{source} is in the command zone or on the battlefield"), 0).unwrap();
            assert!(parse_source_command_or_battlefield_condition_tokens(&tokens));
        }
        for text in [
            "another creature is in the command zone or on the battlefield",
            "this creature is in the command zone or on the battlefield and is red",
            "this creature is in the command zone or on the stack",
            "this creature is in the command zone or on the battlefield unless tapped",
            "this creature is in the command zone or + on the battlefield",
        ] {
            let tokens = crate::lexer::lex_line(text, 0).unwrap();
            assert!(!parse_source_command_or_battlefield_condition_tokens(&tokens), "{text}");
        }
    }
}

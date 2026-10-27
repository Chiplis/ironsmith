use super::*;

pub(super) fn parse_trigger_zone_hint_tokens(tokens: &[OwnedLexToken]) -> Option<Zone> {
    for (phrase, zone) in TRIGGER_ZONE_HINT_PHRASES {
        if has_phrase(tokens, phrase) {
            return Some(*zone);
        }
    }
    None
}

pub fn parse_trigger_functional_zone_facts_tokens(
    tokens: &[OwnedLexToken],
) -> TriggerFunctionalZoneFacts {
    // A quoted granted ability has its own source and functional zone.
    // Its restrictions cannot move the enclosing trigger out of the battlefield.
    let mut quoted = false;
    let outer = tokens.iter().filter_map(|token| {
        if token.is_quote() { quoted = !quoted; None }
        else if quoted { None } else { Some(token.clone()) }
    }).collect::<Vec<_>>();
    let tokens = outer.as_slice();
    TriggerFunctionalZoneFacts {
        excluded_zones: split_ability_function_exclusion(tokens)
            .map(|(_, zone)| vec![zone]).unwrap_or_default(),
        explicit_zone: parse_trigger_zone_hint_tokens(tokens),
        explicit_zone_or_battlefield: has_any_phrase(
            tokens,
            TRIGGER_ZONE_OR_BATTLEFIELD_HINT_PHRASES,
        ),
        returns_self_from_graveyard: has_any_phrase(tokens, RETURN_SELF_FROM_GRAVEYARD_PHRASES),
        discards_this_card: has_phrase(tokens, DISCARD_THIS_CARD_PHRASE),
    }
}

/// A final instruction describing this ability's own zone scope is metadata,
/// rather than an effect performed when it resolves (CR 113.6c).
pub fn split_ability_function_exclusion(
    tokens: &[OwnedLexToken],
) -> Option<(&[OwnedLexToken], Zone)> {
    let sentences = crate::grammar::structure::split_lexed_sentences(tokens);
    let sentence = *sentences.last()?;
    let words = crate::lexer::parser_token_word_refs(sentence);
    let tail = match words.as_slice() {
        ["this", "ability", "doesn't" | "doesnt", "function", tail @ ..] => tail,
        ["this", "ability", "does", "not", "function", tail @ ..] => tail,
        _ => return None,
    };
    let zone = match tail {
        ["on", "the", "battlefield"] => Zone::Battlefield,
        ["on", "the", "stack"] => Zone::Stack,
        ["in", "your" | "a" | "the", "graveyard"] => Zone::Graveyard,
        ["in", "your" | "a" | "the", "hand"] => Zone::Hand,
        ["in", "your" | "a" | "the", "library"] => Zone::Library,
        ["in", "exile"] => Zone::Exile,
        ["in", "the", "command", "zone"] => Zone::Command,
        ["in", "the", "ante", "zone"] => Zone::Ante,
        ["outside", "the", "game"] => Zone::OutsideGame,
        _ => return None,
    };
    let start = tokens.iter().position(|token| std::ptr::eq(token, &sentence[0]))?;
    Some((&tokens[..start], zone))
}

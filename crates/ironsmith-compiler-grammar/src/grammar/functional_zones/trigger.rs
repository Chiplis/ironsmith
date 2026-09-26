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
    TriggerFunctionalZoneFacts {
        explicit_zone: parse_trigger_zone_hint_tokens(tokens),
        explicit_zone_or_battlefield: has_any_phrase(
            tokens,
            TRIGGER_ZONE_OR_BATTLEFIELD_HINT_PHRASES,
        ),
        returns_self_from_graveyard: has_any_phrase(tokens, RETURN_SELF_FROM_GRAVEYARD_PHRASES),
        discards_this_card: has_phrase(tokens, DISCARD_THIS_CARD_PHRASE),
    }
}

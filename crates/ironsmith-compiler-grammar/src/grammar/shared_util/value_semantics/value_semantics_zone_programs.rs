use super::*;

pub(super) fn commander_owner_from_battlefield_or_command_zone_words(
    words: &[&str],
) -> Option<PlayerFilter> {
    if words == COMMANDER_YOU_OWN_BATTLEFIELD_OR_COMMAND_ZONE_PHRASE {
        return Some(PlayerFilter::You);
    }
    if words_match_any_phrase(
        words,
        COMMANDER_ITERATED_PLAYER_OWNS_BATTLEFIELD_OR_COMMAND_ZONE_PHRASES,
    ) {
        return Some(PlayerFilter::IteratedPlayer);
    }
    None
}

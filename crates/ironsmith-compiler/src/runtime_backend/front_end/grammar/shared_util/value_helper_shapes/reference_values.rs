use super::*;

const ITERATED_PLAYER_MARKER_WORDS: &[&str] = &["they", "their", "theyve", "each"];
const SOURCE_COUNTER_REFERENCE_PHRASES: &[&[&str]] = &[
    &["it"],
    &["this"],
    &["this", "artifact"],
    &["this", "creature"],
    &["this", "enchantment"],
    &["this", "equipment"],
    &["this", "land"],
    &["this", "permanent"],
    &["this", "source"],
];
const TAGGED_COUNTER_REFERENCE_PHRASES: &[&[&str]] = &[
    &["that"],
    &["that", "creature"],
    &["that", "permanent"],
    &["that", "object"],
    &["those"],
    &["those", "creatures"],
    &["those", "permanents"],
];

fn parse_value_player_reference(words: &[&str]) -> PlayerFilter {
    if has_any(words, &["you", "your", "youve"]) {
        PlayerFilter::You
    } else if has_any(words, &["opponent", "opponents"]) {
        PlayerFilter::Opponent
    } else if has_any(words, ITERATED_PLAYER_MARKER_WORDS)
        || permission_shapes::find_words(words, &["that", "player"]).is_some()
        || permission_shapes::find_words(words, &["that", "players"]).is_some()
    {
        PlayerFilter::IteratedPlayer
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn parse_cards_discarded_this_turn_player(words: &[&str]) -> Option<PlayerFilter> {
    (has_word(words, "cards")
        && has_word(words, "discarded")
        && has_word(words, "this")
        && has_word(words, "turn"))
    .then(|| parse_value_player_reference(words))
}

pub(crate) fn parse_commander_cast_count_player(words: &[&str]) -> Option<PlayerFilter> {
    (has_word(words, "cast")
        && has_any(words, &["commander", "commanders"])
        && permission_shapes::find_words(words, &["from", "the", "command", "zone"]).is_some()
        && has_word(words, "game"))
    .then(|| parse_value_player_reference(words))
}

pub(crate) fn parse_cards_in_hand_player(words: &[&str]) -> Option<PlayerFilter> {
    if !has_word(words, "cards") || !has_word(words, "in") || !has_any(words, &["hand", "hands"]) {
        return None;
    }
    if has_word(words, "your") {
        return Some(PlayerFilter::You);
    }
    if has_word(words, "their")
        || permission_shapes::find_words(words, &["that", "player"]).is_some()
        || permission_shapes::find_words(words, &["that", "players"]).is_some()
        || permission_shapes::find_words(words, &["the", "chosen"]).is_some()
    {
        return Some(PlayerFilter::IteratedPlayer);
    }
    if has_any(words, &["opponent", "opponents"]) {
        return Some(PlayerFilter::Opponent);
    }
    None
}

pub(crate) fn parse_party_size_player(words: &[&str]) -> Option<PlayerFilter> {
    (permission_shapes::exact_words(words, &["creatures", "in", "your", "party"])
        || permission_shapes::exact_words(words, &["creature", "in", "your", "party"]))
    .then_some(PlayerFilter::You)
}

pub(crate) fn parse_counter_reference_value_shape(
    words: &[&str],
) -> Option<CounterReferenceValueShape> {
    if !permission_shapes::prefix_words(words, &["equal", "to"]) {
        return None;
    }
    let mut index = 2usize;
    if permission_shapes::starts_at_words(words, index, &["the"]) {
        index += 1;
    }
    if !permission_shapes::starts_at_words(words, index, &["number", "of"]) {
        return None;
    }
    index += 2;
    if words
        .get(index)
        .is_some_and(|word| is_article(word) || *word == "one")
    {
        index += 1;
    }

    let counter_offset = words
        .get(index..)?
        .iter()
        .position(|word| matches!(*word, "counter" | "counters"))?;
    if counter_offset > 2 {
        return None;
    }
    let counter_idx = index + counter_offset;
    let counter_type = (counter_idx > index)
        .then(|| parse_counter_type_words(&words[index..=counter_idx]))
        .flatten();
    index = counter_idx + 1;
    if !permission_shapes::starts_at_words(words, index, &["on"]) {
        return None;
    }
    index += 1;
    let reference_words = words.get(index..)?;
    if reference_words.is_empty() {
        return None;
    }

    let source_surface = source_reference_surface_for_words(reference_words).or_else(|| {
        (reference_words.len() > 1)
            .then(|| this_source_surface_for_words(reference_words))
            .flatten()
    });
    let reference = if SOURCE_COUNTER_REFERENCE_PHRASES
        .iter()
        .any(|phrase| permission_shapes::exact_words(reference_words, phrase))
        || source_surface.is_some()
    {
        CounterValueReference::Source(source_surface)
    } else if TAGGED_COUNTER_REFERENCE_PHRASES
        .iter()
        .any(|phrase| permission_shapes::exact_words(reference_words, phrase))
    {
        CounterValueReference::Tagged
    } else {
        return None;
    };

    Some(CounterReferenceValueShape {
        counter_type,
        reference,
    })
}

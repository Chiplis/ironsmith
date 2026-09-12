use super::*;

pub fn parse_activated_functional_zones_tokens(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[&[OwnedLexToken]],
) -> Vec<Zone> {
    if effect_sentences.iter().any(|sentence| {
        abilities::is_any_player_may_activate_sentence_lexed(sentence)
            && primitives::find_prefix(sentence, || primitives::phrase(&["on", "the", "stack"]))
                .is_some()
    }) {
        return vec![Zone::Stack];
    }

    let cost_words = activation_source_words(cost_tokens);
    let effect_words = effect_sentences
        .iter()
        .map(|sentence| activation_source_words(sentence))
        .collect::<Vec<_>>();
    let any_effect = |predicate: fn(&[&str]) -> bool| {
        effect_words.iter().any(|words| predicate(words.as_slice()))
    };

    // An explicit activation location applies even when the move itself omits
    // its origin (for example, an ability that exiles its source).
    if let Some(zones) = effect_words
        .iter()
        .find_map(|words| explicit_activation_zones(words))
    {
        return zones;
    }

    // CR 113.6m: inspect source moves in payment/resolution order. A cost that
    // discards or exiles the source establishes its starting zone before a
    // later effect returns it. Looking for an arbitrary graveyard phrase in
    // the entire ability incorrectly overrides that earlier move.
    if let Some(zones) = source_move_zones(&cost_words).or_else(|| {
        effect_words
            .iter()
            .find_map(|words| source_move_zones(words))
    }) {
        return zones;
    }

    let returns_source_from_graveyard_or_exile = effect_words.iter().any(|words| {
        reference_shapes::contains_source_from_your_graveyard(words)
            && crate::word_primitives::any_sequence_occurs(
                words,
                &[
                    &["graveyard", "or", "from", "exile"],
                    &["graveyard", "or", "exile"],
                ],
            )
    });

    if returns_source_from_graveyard_or_exile {
        vec![Zone::Graveyard, Zone::Exile]
    } else if reference_shapes::contains_source_from_your_graveyard(&cost_words)
        || any_effect(reference_shapes::contains_source_from_your_graveyard)
    {
        vec![Zone::Graveyard]
    } else if reference_shapes::contains_source_from_command_zone(&cost_words)
        || any_effect(reference_shapes::contains_source_from_command_zone)
        || effect_sentences
            .iter()
            .any(|sentence| contains_named_source_command_zone_move(sentence))
    {
        vec![Zone::Command]
    } else if reference_shapes::contains_source_from_your_hand(&cost_words)
        || reference_shapes::contains_discard_source(&cost_words)
        || any_effect(reference_shapes::contains_source_from_your_hand)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

/// The source reference must be the direct object of a move. In particular,
/// "return target card ..." and "cards gain ... this card ..." do not qualify.
fn source_end(words: &[&str], start: usize) -> Option<usize> {
    if !matches!(words.get(start), Some(&"this" | &"thiss")) {
        return None;
    }
    let mut end = start + 1;
    if matches!(
        words.get(end),
        Some(
            &"card"
                | &"creature"
                | &"permanent"
                | &"artifact"
                | &"enchantment"
                | &"land"
                | &"planeswalker"
                | &"battle"
                | &"aura"
                | &"source"
                | &"token"
        )
    ) {
        end += 1;
    }
    Some(end)
}

fn zone_at(words: &[&str], mut index: usize) -> Option<(Zone, usize)> {
    if matches!(words.get(index), Some(&"your" | &"its" | &"their")) {
        index += 1;
    }
    if matches!(words.get(index), Some(&"owners" | &"owner's" | &"owner")) {
        index += 1;
    }
    let zone = match words.get(index)? {
        &"graveyard" => Zone::Graveyard,
        &"hand" => Zone::Hand,
        &"exile" => Zone::Exile,
        &"battlefield" => Zone::Battlefield,
        &"library" => Zone::Library,
        &"stack" => Zone::Stack,
        &"command" if words.get(index + 1) == Some(&"zone") => {
            return Some((Zone::Command, index + 2));
        }
        _ => return None,
    };
    Some((zone, index + 1))
}

fn origin_zones(words: &[&str], index: usize) -> Option<Vec<Zone>> {
    let (zone, end) = zone_at(words, index)?;
    let mut zones = vec![zone];
    if words.get(end) == Some(&"or") {
        let next = end + 1 + usize::from(words.get(end + 1) == Some(&"from"));
        if let Some((other, _)) = zone_at(words, next) {
            if !zones.contains(&other) {
                zones.push(other);
            }
        }
    }
    Some(zones)
}

fn explicit_activation_zones(words: &[&str]) -> Option<Vec<Zone>> {
    let start = words
        .windows(3)
        .position(|part| part == ["activate", "only", "if"])?
        + 3;
    let end = source_end(words, start)?;
    if words.get(end) != Some(&"is") || !matches!(words.get(end + 1), Some(&"in" | &"on")) {
        return None;
    }
    let (zone, end) = zone_at(words, end + 2)?;
    let mut zones = vec![zone];
    if words.get(end) == Some(&"or") {
        let mut next = end + 1;
        if matches!(words.get(next), Some(&"in" | &"on")) {
            next += 1;
        }
        if let Some((other, _)) = zone_at(words, next) {
            zones.push(other);
        }
    }
    Some(zones)
}

fn source_move_zones(words: &[&str]) -> Option<Vec<Zone>> {
    for (index, verb) in words.iter().enumerate() {
        if !matches!(
            *verb,
            "return" | "put" | "shuffle" | "exile" | "discard" | "sacrifice"
        ) {
            continue;
        }
        let Some(end) = source_end(words, index + 1) else {
            continue;
        };
        // A shared origin may follow coordinated objects or the destination:
        // "return this card and target land card from your graveyard ...";
        // "shuffle this card into your library from your graveyard".
        for offset in end..words.len() {
            let word = words[offset];
            if word == "from" {
                if let Some(zones) = origin_zones(words, offset + 1) {
                    return Some(zones);
                }
            }
            // Never borrow an origin from another action, condition, or grant.
            if matches!(
                word,
                "return"
                    | "put"
                    | "shuffle"
                    | "exile"
                    | "discard"
                    | "sacrifice"
                    | "draw"
                    | "create"
                    | "gain"
                    | "gains"
                    | "if"
                    | "when"
                    | "whenever"
                    | "then"
                    | "activate"
            ) {
                break;
            }
        }
        return Some(vec![if *verb == "discard" {
            Zone::Hand
        } else {
            Zone::Battlefield
        }]);
    }
    None
}

// A quoted ability has its own source. Its zone must not relocate the ability
// granting it; inference will run separately when the quoted line is parsed.
fn activation_source_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    let mut words = Vec::new();
    let mut quoted = false;
    let mut start = 0;
    for (index, token) in tokens.iter().enumerate() {
        if token.is_quote() {
            if !quoted {
                words.extend(normalized_activated_zone_words(&tokens[start..index]));
            }
            quoted = !quoted;
            start = index + 1;
        }
    }
    if !quoted {
        words.extend(normalized_activated_zone_words(&tokens[start..]));
    }
    words
}

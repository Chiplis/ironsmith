//! Complete zone-change surfaces. Subjects are left to the shared object-filter
//! reader; this grammar owns only movement, zone possessors, and timing.

use crate::target::PlayerFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneEndpoint {
    pub zone: Zone,
    pub owner: Option<PlayerFilter>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ZoneOrigin {
    Any,
    Except(Zone),
    Zones(Vec<ZoneEndpoint>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZoneChangeSurface {
    pub subject_word_end: usize,
    pub origin: Option<ZoneOrigin>,
    pub destination: Option<ZoneEndpoint>,
    pub returned: bool,
    pub during_turn: Option<PlayerFilter>,
}

fn endpoint(words: &[&str]) -> Option<ZoneEndpoint> {
    let (noun, prefix) = words.split_last()?;
    let zone = match *noun {
        "battlefield" => Zone::Battlefield,
        "graveyard" => Zone::Graveyard,
        "hand" => Zone::Hand,
        "library" => Zone::Library,
        "exile" => Zone::Exile,
        _ => return None,
    };
    let owner = match prefix {
        [] | ["a"] | ["the"] => None,
        ["your"] => Some(PlayerFilter::You),
        ["an", "opponent's" | "opponents" | "opponent"] => Some(PlayerFilter::Opponent),
        ["a", "player's" | "players" | "player"] | ["its", "owner's" | "owners" | "owner"] => {
            Some(PlayerFilter::Any)
        }
        _ => return None,
    };
    // Battlefield and exile are shared zones, not somebody's private zone.
    if owner.is_some() && matches!(zone, Zone::Battlefield | Zone::Exile) {
        return None;
    }
    Some(ZoneEndpoint { zone, owner })
}

fn origin(words: &[&str]) -> Option<ZoneOrigin> {
    if words == ["anywhere"] {
        return Some(ZoneOrigin::Any);
    }
    if let Some(rest) = words
        .strip_prefix(&["anywhere", "other", "than"])
        .or_else(|| words.strip_prefix(&["anywhere", "except"]))
    {
        let excluded = endpoint(rest)?;
        return excluded
            .owner
            .is_none()
            .then_some(ZoneOrigin::Except(excluded.zone));
    }
    let mut zones = Vec::new();
    for part in words.split(|word| *word == "or") {
        let mut zone = endpoint(part)?;
        if let Some(first) = zones.first() {
            let first: &ZoneEndpoint = first;
            if zone.owner.is_none() {
                zone.owner = first.owner.clone();
            }
            // Different owners for different origins require a union, rather
            // than flattening those constraints into one cross-product.
            if zone.owner != first.owner {
                return None;
            }
        }
        zones.push(zone);
    }
    (!zones.is_empty()).then_some(ZoneOrigin::Zones(zones))
}

pub fn parse_zone_change_surface(words: &[&str]) -> Option<ZoneChangeSurface> {
    let (words, during_turn) = if let Some(rest) = words.strip_suffix(&["during", "your", "turn"]) {
        (rest, Some(PlayerFilter::You))
    } else if let Some(rest) = words
        .strip_suffix(&["during", "an", "opponent's", "turn"])
        .or_else(|| words.strip_suffix(&["during", "an", "opponents", "turn"]))
    {
        (rest, Some(PlayerFilter::Opponent))
    } else {
        (words, None)
    };
    for index in 1..words.len() {
        let tail = &words[index..];
        if matches!(tail[0], "leave" | "leaves") {
            return Some(ZoneChangeSurface {
                subject_word_end: index,
                origin: Some(origin(&tail[1..])?),
                destination: None,
                returned: false,
                during_turn,
            });
        }
        let ["is" | "are", verb, preposition, rest @ ..] = tail else {
            continue;
        };
        let returned = match (*verb, *preposition) {
            ("put", "into") => false,
            ("returned", "to") => true,
            _ => continue,
        };
        let from = rest.iter().position(|word| *word == "from");
        let destination = endpoint(&rest[..from.unwrap_or(rest.len())])?;
        if returned && destination.zone != Zone::Hand {
            continue;
        }
        return Some(ZoneChangeSurface {
            subject_word_end: index,
            origin: match from {
                Some(index) => Some(origin(&rest[index + 1..])?),
                None => None,
            },
            destination: Some(destination),
            returned,
            during_turn,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Option<ZoneChangeSurface> {
        parse_zone_change_surface(&text.split_whitespace().collect::<Vec<_>>())
    }

    #[test]
    fn zone_endpoints_preserve_owner_origin_and_event_time() {
        let parsed =
            parse("one or more artifacts you control leave the battlefield during your turn")
                .unwrap();
        assert_eq!(parsed.subject_word_end, 6);
        assert_eq!(parsed.during_turn, Some(PlayerFilter::You));
        assert_eq!(
            parsed.origin,
            Some(ZoneOrigin::Zones(vec![ZoneEndpoint {
                zone: Zone::Battlefield,
                owner: None
            }]))
        );
        let parsed =
            parse("a Desert card is put into your graveyard from your hand or library").unwrap();
        assert_eq!(parsed.destination.unwrap().owner, Some(PlayerFilter::You));
        assert_eq!(
            parsed.origin,
            Some(ZoneOrigin::Zones(vec![
                ZoneEndpoint {
                    zone: Zone::Hand,
                    owner: Some(PlayerFilter::You)
                },
                ZoneEndpoint {
                    zone: Zone::Library,
                    owner: Some(PlayerFilter::You)
                },
            ]))
        );
        assert_eq!(parse("an artifact card is put into your graveyard from anywhere other than the battlefield").unwrap().origin, Some(ZoneOrigin::Except(Zone::Battlefield)));
    }

    #[test]
    fn movement_grammar_consumes_all_qualifiers_without_cross_producting_owners() {
        for text in [
            "a permanent is returned to hand during combat",
            "a permanent is put into a graveyard by a spell",
            "a card leaves a graveyard for the first time each turn",
            "a card is put into exile from your hand or an opponent's library",
            "a card is put into your exile",
        ] {
            assert!(parse(text).is_none(), "{text}");
        }
    }
}

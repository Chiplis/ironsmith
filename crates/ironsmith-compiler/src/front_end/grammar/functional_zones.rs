use crate::zone::Zone;

use super::super::lexer::{OwnedLexToken, is_authored_proper_name_phrase};
use super::shared_util::reference_shapes;
use super::{abilities, leaf, primitives};

const STATIC_LIBRARY_SEARCH_ZONE_PHRASES: &[&[&str]] = &[
    &["while", "youre", "searching", "your", "library"],
    &["while", "you're", "searching", "your", "library"],
];
const FROM_YOUR_LIBRARY_PHRASE: &[&str] = &["from", "your", "library"];
const CAST_OR_PLAY_SELF_FROM_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["cast", "this", "card", "from", "your", "graveyard"],
    &["cast", "this", "spell", "from", "your", "graveyard"],
    &["cast", "this", "permanent", "from", "your", "graveyard"],
    &["cast", "this", "creature", "from", "your", "graveyard"],
    &["cast", "this", "artifact", "from", "your", "graveyard"],
    &["cast", "this", "enchantment", "from", "your", "graveyard"],
    &["play", "this", "card", "from", "your", "graveyard"],
    &["play", "this", "permanent", "from", "your", "graveyard"],
];
const CAST_OR_PLAY_SELF_FROM_EXILE_PHRASES: &[&[&str]] = &[
    &["cast", "this", "card", "from", "exile"],
    &["play", "this", "card", "from", "exile"],
];
const CAUSES_YOU_TO_DISCARD_THIS_CARD_PHRASE: &[&str] =
    &["causes", "you", "to", "discard", "this", "card"];
const INSTEAD_OF_PUTTING_IT_INTO_YOUR_GRAVEYARD_PHRASE: &[&str] = &[
    "instead",
    "of",
    "putting",
    "it",
    "into",
    "your",
    "graveyard",
];
const SOURCE_NOT_ON_BATTLEFIELD_PHRASES: &[&[&str]] = &[
    &["this", "creature", "isn't", "on", "the", "battlefield"],
    &["this", "permanent", "isn't", "on", "the", "battlefield"],
    &["this", "card", "isn't", "on", "the", "battlefield"],
    &["this", "isn't", "on", "the", "battlefield"],
    &["it", "isn't", "on", "the", "battlefield"],
    &["this", "creature", "isnt", "on", "the", "battlefield"],
    &["this", "permanent", "isnt", "on", "the", "battlefield"],
    &["this", "card", "isnt", "on", "the", "battlefield"],
    &["this", "isnt", "on", "the", "battlefield"],
    &["it", "isnt", "on", "the", "battlefield"],
    &["this", "creature", "is", "not", "on", "the", "battlefield"],
    &["this", "permanent", "is", "not", "on", "the", "battlefield"],
    &["this", "card", "is", "not", "on", "the", "battlefield"],
    &["this", "is", "not", "on", "the", "battlefield"],
    &["it", "is", "not", "on", "the", "battlefield"],
];
const STATIC_ZONE_HINT_PHRASES: &[(&[&str], Zone)] = &[
    (&["this", "card", "is", "in", "your", "hand"], Zone::Hand),
    (
        &["there", "is", "this", "card", "in", "your", "hand"],
        Zone::Hand,
    ),
    (
        &["this", "card", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "card", "is", "in", "a", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "creature", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "permanent", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "object", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["there", "is", "this", "card", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["this", "card", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (
        &["there", "is", "this", "card", "in", "your", "library"],
        Zone::Library,
    ),
    (&["this", "card", "is", "in", "exile"], Zone::Exile),
    (&["there", "is", "this", "card", "in", "exile"], Zone::Exile),
    (
        &["this", "card", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
    (
        &[
            "there", "is", "this", "card", "in", "the", "command", "zone",
        ],
        Zone::Command,
    ),
];
const TRIGGER_ZONE_HINT_PHRASES: &[(&[&str], Zone)] = &[
    (&["if", "this", "is", "in", "your", "hand"], Zone::Hand),
    (
        &["if", "this", "card", "is", "in", "your", "hand"],
        Zone::Hand,
    ),
    (
        &["if", "this", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "card", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &[
            "if",
            "this",
            "card",
            "is",
            "the",
            "only",
            "creature",
            "card",
            "in",
            "your",
            "graveyard",
        ],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "creature", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "permanent", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "object", "is", "in", "your", "graveyard"],
        Zone::Graveyard,
    ),
    (
        &["if", "this", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (
        &["if", "this", "card", "is", "in", "your", "library"],
        Zone::Library,
    ),
    (&["if", "this", "is", "in", "exile"], Zone::Exile),
    (&["if", "this", "card", "is", "in", "exile"], Zone::Exile),
    (&["if", "this", "card", "is", "exiled"], Zone::Exile),
    (
        &["if", "this", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
    (
        &["if", "this", "card", "is", "in", "the", "command", "zone"],
        Zone::Command,
    ),
];
const RETURN_SELF_FROM_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["return", "this", "from", "your", "graveyard"],
    &["return", "this", "card", "from", "your", "graveyard"],
];
const DISCARD_THIS_CARD_PHRASE: &[&str] = &["discard", "this", "card"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerFunctionalZoneFacts {
    pub explicit_zone: Option<Zone>,
    pub returns_self_from_graveyard: bool,
    pub discards_this_card: bool,
}

#[cfg(test)]
mod tests;

#[path = "functional_zones/trigger_programs.rs"]
mod trigger_programs;
pub use trigger_programs::parse_trigger_functional_zone_facts_tokens;
use trigger_programs::parse_trigger_zone_hint_tokens;
#[path = "functional_zones/object_action_programs.rs"]
mod object_action_programs;
pub use object_action_programs::parse_static_functional_zones_tokens;
#[path = "functional_zones/core_programs.rs"]
mod core_programs;
use core_programs::{has_any_phrase, has_phrase};
#[path = "functional_zones/permission_programs.rs"]
mod permission_programs;
use permission_programs::normalized_activated_zone_words;
#[path = "functional_zones/reference_programs.rs"]
mod reference_programs;
use reference_programs::{
    contains_named_source_command_zone_move, is_named_or_normalized_source_surface,
    trim_named_source_surface,
};

#[path = "functional_zones/activated_zone_resolution.rs"]
mod activated_zone_resolution;
pub use activated_zone_resolution::parse_activated_functional_zones_tokens;

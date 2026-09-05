use crate::effect::{EventValueSpec, Value};
use crate::grammar::{filters::parse_counter_type_words, leaf};
use crate::lexer::{OwnedLexToken, TokenWordView, synthetic_word_tokens};
use crate::object_filters::parse_object_filter_words;
use crate::target::{
    ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter, SacrificedObjectKind,
    SourceReferenceSurface,
};
use crate::util::{
    possessive_normalized_word_refs, source_choose_spec_for_surface,
    source_reference_surface_for_possessive_words, source_reference_surface_for_words,
    this_source_surface_for_words,
};
use crate::{Color, TagKey};
use ironsmith_core::ValueSurfaceHint;

use super::super::permission_shapes;
use super::value_helper_shapes;

const EVENT_AMOUNT_PREFIXES: &[(&[&str], usize)] = &[
    (&["that", "many"], 2),
    (&["that", "much"], 2),
    (&["that", "amount"], 2),
    (&["the", "amount", "of", "e", "paid", "this", "way"], 7),
    (&["amount", "of", "e", "paid", "this", "way"], 6),
    (&["that", "amount", "of", "excess", "damage"], 5),
    (&["that", "much", "excess", "damage"], 4),
    (&["the", "excess"], 2),
];

const DAMAGE_EVENT_AMOUNT_PREFIXES: &[(&[&str], usize)] = &[
    (&["damage", "dealt", "this", "way"], 4),
    (&["the", "damage", "dealt", "this", "way"], 5),
    (&["damage", "dealt"], 2),
    (&["the", "damage", "dealt"], 3),
    (&["that", "damage"], 2),
];

const COLORS_SPENT_PREFIXES: &[&[&str]] = &[
    &[
        "the", "number", "of", "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
    ],
    &[
        "number", "of", "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
    ],
    &[
        "the", "number", "of", "colors", "of", "mana", "used", "to", "cast", "this", "spell",
    ],
    &[
        "number", "of", "colors", "of", "mana", "used", "to", "cast", "this", "spell",
    ],
    &[
        "colors", "of", "mana", "spent", "to", "cast", "this", "spell",
    ],
    &[
        "colors", "of", "mana", "used", "to", "cast", "this", "spell",
    ],
    &[
        "the", "number", "of", "colors", "of", "mana", "spent", "to", "cast", "it",
    ],
    &[
        "number", "of", "colors", "of", "mana", "spent", "to", "cast", "it",
    ],
    &[
        "the", "number", "of", "colors", "of", "mana", "used", "to", "cast", "it",
    ],
    &[
        "number", "of", "colors", "of", "mana", "used", "to", "cast", "it",
    ],
    &["colors", "of", "mana", "spent", "to", "cast", "it"],
    &["colors", "of", "mana", "used", "to", "cast", "it"],
];

const TAGGED_POWER_PREFIXES: &[&[&str]] = &[
    &["that", "creature", "power"],
    &["that", "creatures", "power"],
    &["that", "card", "power"],
    &["that", "cards", "power"],
    &["that", "object", "power"],
    &["that", "objects", "power"],
    &["the", "exiled", "card", "power"],
    &["the", "exiled", "card's", "power"],
    &["the", "exiled", "cards", "power"],
    &["exiled", "card", "power"],
    &["exiled", "card's", "power"],
    &["exiled", "cards", "power"],
    &["the", "exploited", "creature", "power"],
    &["the", "exploited", "creatures", "power"],
    &["exploited", "creature", "power"],
    &["exploited", "creatures", "power"],
    &["the", "sacrificed", "creature", "power"],
    &["the", "sacrificed", "creatures", "power"],
    &["sacrificed", "creature", "power"],
    &["sacrificed", "creatures", "power"],
    &["the", "amassed", "army", "power"],
    &["the", "amassed", "armys", "power"],
    &["amassed", "army", "power"],
    &["amassed", "armys", "power"],
    &["the", "army", "you", "amassed", "power"],
    &["army", "you", "amassed", "power"],
];

const TAGGED_TOUGHNESS_PREFIXES: &[&[&str]] = &[
    &["that", "creature", "toughness"],
    &["that", "creatures", "toughness"],
    &["that", "card", "toughness"],
    &["that", "cards", "toughness"],
    &["that", "object", "toughness"],
    &["that", "objects", "toughness"],
    &["the", "exiled", "card", "toughness"],
    &["the", "exiled", "card's", "toughness"],
    &["the", "exiled", "cards", "toughness"],
    &["exiled", "card", "toughness"],
    &["exiled", "card's", "toughness"],
    &["exiled", "cards", "toughness"],
    &["the", "exploited", "creature", "toughness"],
    &["the", "exploited", "creatures", "toughness"],
    &["exploited", "creature", "toughness"],
    &["exploited", "creatures", "toughness"],
    &["the", "sacrificed", "creature", "toughness"],
    &["the", "sacrificed", "creatures", "toughness"],
    &["sacrificed", "creature", "toughness"],
    &["sacrificed", "creatures", "toughness"],
    &["the", "amassed", "army", "toughness"],
    &["the", "amassed", "armys", "toughness"],
    &["amassed", "army", "toughness"],
    &["amassed", "armys", "toughness"],
    &["the", "army", "you", "amassed", "toughness"],
    &["army", "you", "amassed", "toughness"],
];

const TAGGED_MANA_VALUE_PREFIXES: &[&[&str]] = &[
    &["that", "spell", "mana", "value"],
    &["that", "spell's", "mana", "value"],
    &["that", "spells", "mana", "value"],
    &["that", "permanent", "mana", "value"],
    &["that", "permanent's", "mana", "value"],
    &["that", "permanents", "mana", "value"],
    &["that", "equipment", "mana", "value"],
    &["that", "equipment's", "mana", "value"],
    &["that", "equipments", "mana", "value"],
    &["that", "object", "mana", "value"],
    &["that", "object's", "mana", "value"],
    &["that", "objects", "mana", "value"],
    &["the", "card", "mana", "value"],
    &["the", "card's", "mana", "value"],
    &["the", "cards", "mana", "value"],
    &["that", "card", "mana", "value"],
    &["that", "card's", "mana", "value"],
    &["that", "cards", "mana", "value"],
    &["the", "sacrificed", "creature", "mana", "value"],
    &["the", "sacrificed", "creatures", "mana", "value"],
    &["the", "sacrificed", "artifact", "mana", "value"],
    &["the", "sacrificed", "artifacts", "mana", "value"],
    &["the", "sacrificed", "enchantment", "mana", "value"],
    &["the", "sacrificed", "enchantments", "mana", "value"],
    &["the", "sacrificed", "permanent", "mana", "value"],
    &["the", "sacrificed", "permanents", "mana", "value"],
    &["sacrificed", "creature", "mana", "value"],
    &["sacrificed", "creatures", "mana", "value"],
    &["sacrificed", "artifact", "mana", "value"],
    &["sacrificed", "artifacts", "mana", "value"],
    &["sacrificed", "enchantment", "mana", "value"],
    &["sacrificed", "enchantments", "mana", "value"],
    &["sacrificed", "permanent", "mana", "value"],
    &["sacrificed", "permanents", "mana", "value"],
    &["the", "amassed", "army", "mana", "value"],
    &["the", "amassed", "armys", "mana", "value"],
    &["amassed", "army", "mana", "value"],
    &["amassed", "armys", "mana", "value"],
    &["the", "mana", "value", "of", "the", "amassed", "army"],
    &["the", "mana", "value", "of", "the", "amassed", "armys"],
    &["mana", "value", "of", "the", "amassed", "army"],
    &["mana", "value", "of", "the", "amassed", "armys"],
    &[
        "the", "mana", "value", "of", "the", "army", "you", "amassed",
    ],
    &["mana", "value", "of", "the", "army", "you", "amassed"],
];

fn tagged_characteristic_reference_tag(words: &[&str]) -> crate::tag::CompilerReferenceTag {
    if words.contains(&"exiled") {
        crate::tag::CompilerReferenceTag::SourceExiled
    } else if words.contains(&"exploited") {
        crate::tag::CompilerReferenceTag::Exploited
    } else {
        crate::tag::CompilerReferenceTag::It
    }
}

fn sacrificed_object_kind(words: &[&str]) -> Option<SacrificedObjectKind> {
    let sacrificed = crate::word_primitives::parse_sequence_start(words, &["sacrificed"])?;
    match words.get(sacrificed + 1).copied()? {
        "creature" | "creatures" | "creature's" => Some(SacrificedObjectKind::Creature),
        "artifact" | "artifacts" | "artifact's" => Some(SacrificedObjectKind::Artifact),
        "enchantment" | "enchantments" | "enchantment's" => Some(SacrificedObjectKind::Enchantment),
        "permanent" | "permanents" | "permanent's" => Some(SacrificedObjectKind::Permanent),
        _ => None,
    }
}

fn with_sacrificed_object_surface(value: Value, words: &[&str]) -> Value {
    match sacrificed_object_kind(words) {
        Some(kind) => value.with_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
        None => value,
    }
}

pub fn colored_mana_symbols_in_costs(words: &[&str]) -> Option<(Value, usize)> {
    let mut idx = if permission_shapes::prefix_words(words, &["for", "each"]) {
        2
    } else if permission_shapes::prefix_words(words, &["each"]) {
        1
    } else {
        let mut number_idx = usize::from(words.first() == Some(&"the"));
        if !permission_shapes::starts_at_words(words, number_idx, &["number", "of"]) {
            return None;
        }
        number_idx += 2;
        number_idx
    };

    let color = Color::from_name(words.get(idx).copied()?)?;
    idx += 1;
    if words.get(idx) != Some(&"mana")
        || !words
            .get(idx + 1)
            .is_some_and(|word| matches!(*word, "symbol" | "symbols"))
        || words.get(idx + 2) != Some(&"in")
    {
        return None;
    }
    idx += 3;
    if words
        .get(idx)
        .is_some_and(|word| matches!(*word, "the" | "a" | "an"))
    {
        idx += 1;
    }

    if words.get(idx) == Some(&"mana")
        && words
            .get(idx + 1)
            .is_some_and(|word| matches!(*word, "cost" | "costs"))
        && words.get(idx + 2) == Some(&"of")
    {
        let filter_start = idx + 3;
        let filter_end = filter_start + value_boundary(&words[filter_start..]);
        let filter = parse_object_filter_words(&words[filter_start..filter_end], false).ok()?;
        return Some((
            Value::ManaSymbolsInManaCostOf {
                spec: Box::new(ChooseSpec::All(filter)),
                color,
            },
            filter_end,
        ));
    }

    let kind = sacrificed_object_kind(words.get(idx..idx + 2)?)?;
    idx += 2;
    if !permission_shapes::starts_at_words(words, idx, &["mana", "cost"]) {
        return None;
    }
    idx += 2;

    Some((
        Value::ManaSymbolsInManaCostOf {
            spec: Box::new(ChooseSpec::Tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            )),
            color,
        }
        .with_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
        idx,
    ))
}

fn sacrificed_postpositive_characteristic_prefix(
    words: &[&str],
    characteristic: &[&str],
) -> Option<(SacrificedObjectKind, usize)> {
    let mut idx = usize::from(words.first() == Some(&"the"));
    if !permission_shapes::starts_at_words(words, idx, characteristic) {
        return None;
    }
    idx += characteristic.len();
    if words.get(idx) != Some(&"of") {
        return None;
    }
    idx += 1;
    if words
        .get(idx)
        .is_some_and(|word| matches!(*word, "the" | "a" | "an"))
    {
        idx += 1;
    }
    if words.get(idx) != Some(&"sacrificed") {
        return None;
    }
    let kind = match words.get(idx + 1).copied()? {
        "creature" | "creatures" => SacrificedObjectKind::Creature,
        "artifact" | "artifacts" => SacrificedObjectKind::Artifact,
        "enchantment" | "enchantments" => SacrificedObjectKind::Enchantment,
        "permanent" | "permanents" => SacrificedObjectKind::Permanent,
        _ => return None,
    };
    Some((kind, idx + 2))
}

const REVEALED_MANA_VALUE_PREFIXES: &[&[&str]] = &[
    &["the", "revealed", "card", "mana", "value"],
    &["the", "revealed", "cards", "mana", "value"],
    &["revealed", "card", "mana", "value"],
    &["revealed", "cards", "mana", "value"],
    &["the", "mana", "value", "of", "the", "revealed", "card"],
    &["the", "mana", "value", "of", "the", "revealed", "cards"],
    &["mana", "value", "of", "the", "revealed", "card"],
    &["mana", "value", "of", "the", "revealed", "cards"],
];

const EXILED_MANA_VALUE_PREFIXES: &[&[&str]] = &[
    &["the", "exiled", "card", "mana", "value"],
    &["the", "exiled", "cards", "mana", "value"],
    &["exiled", "card", "mana", "value"],
    &["exiled", "cards", "mana", "value"],
    &["the", "mana", "value", "of", "the", "exiled", "card"],
    &["the", "mana", "value", "of", "the", "exiled", "cards"],
    &["mana", "value", "of", "the", "exiled", "card"],
    &["mana", "value", "of", "the", "exiled", "cards"],
    &["the", "exiled", "spell", "mana", "value"],
    &["the", "exiled", "spells", "mana", "value"],
    &["exiled", "spell", "mana", "value"],
    &["exiled", "spells", "mana", "value"],
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Rounding {
    Down,
    Up,
}

fn parse_devotion_value_words(words: &[&str]) -> Option<(Value, usize)> {
    let (player, devotion_index) = if permission_shapes::prefix_words(words, &["your", "devotion"])
    {
        (PlayerFilter::You, 1)
    } else if permission_shapes::prefix_words(words, &["their", "devotion"]) {
        (PlayerFilter::IteratedPlayer, 1)
    } else if permission_shapes::prefix_words(words, &["opponent", "devotion"])
        || permission_shapes::prefix_words(words, &["opponents", "devotion"])
    {
        (PlayerFilter::Opponent, 1)
    } else if permission_shapes::prefix_words(words, &["that", "player", "devotion"])
        || permission_shapes::prefix_words(words, &["that", "players", "devotion"])
    {
        (PlayerFilter::target_player(), 2)
    } else {
        return None;
    };

    let to_index = devotion_index + 1;
    if words.get(to_index) != Some(&"to") {
        return None;
    }
    let color_index = to_index + 1;
    if words.get(color_index..color_index + 2) == Some(["that", "color"].as_slice()) {
        return Some((Value::DevotionToChosenColor(player), color_index + 2));
    }
    let color = Color::from_name(words.get(color_index).copied()?)?;
    Some((Value::Devotion { player, color }, color_index + 1))
}

pub fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
    if let Some(parsed) = parse_whichever_is_greater_value_words(words) {
        return Some(parsed);
    }
    let (mut value, mut used) = parse_value_expr_term_words(words)?;
    while used < words.len() {
        let (subtract, operator_words, in_excess_of) = if words.get(used) == Some(&"plus") {
            (false, 1, false)
        } else if words.get(used) == Some(&"minus") {
            (true, 1, false)
        } else if words.get(used..used + 3) == Some(["in", "excess", "of"].as_slice()) {
            (true, 3, true)
        } else {
            break;
        };
        let (rhs, rhs_used) = parse_value_expr_term_words(&words[used + operator_words..])?;
        used += operator_words + rhs_used;
        let rhs = if subtract {
            match rhs {
                Value::Fixed(fixed) => Value::Fixed(-fixed),
                Value::X => Value::XTimes(-1),
                Value::XTimes(multiplier) => Value::XTimes(-multiplier),
                other => Value::Scaled(Box::new(other), -1),
            }
        } else {
            rhs
        };
        value = Value::Add(Box::new(value), Box::new(rhs));
        if in_excess_of {
            value = value.with_surface_hint(ValueSurfaceHint::InExcessOf);
        }
    }
    Some((value, used))
}

fn parse_whichever_is_greater_value_words(words: &[&str]) -> Option<(Value, usize)> {
    const SUFFIX: &[&str] = &["whichever", "is", "greater"];
    let body_len = words.len().checked_sub(SUFFIX.len())?;
    if words.get(body_len..) != Some(SUFFIX) {
        return None;
    }
    let body = words.get(..body_len)?;
    for split in (1..body.len()).rev() {
        if body.get(split) != Some(&"or") {
            continue;
        }
        let (left, left_used) = parse_value_expr_words(&body[..split])?;
        let (right, right_used) = parse_value_expr_words(&body[split + 1..])?;
        if left_used != split || right_used != body.len() - split - 1 {
            continue;
        }
        let minimum = Value::Min(Box::new(left.clone()), Box::new(right.clone()));
        let total = Value::Add(Box::new(left), Box::new(right));
        let maximum = Value::Add(
            Box::new(total),
            Box::new(Value::Scaled(Box::new(minimum), -1)),
        )
        .with_surface_hint(ValueSurfaceHint::WhicheverIsGreater);
        return Some((maximum, words.len()));
    }
    None
}

pub fn parse_value_expr_tokens(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    let (value, used_words) = parse_value_expr_words(&words)?;
    let used_tokens = word_view
        .token_start_indices()
        .get(used_words)
        .copied()
        .unwrap_or(tokens.len());
    Some((value, used_tokens))
}

#[cfg(test)]
#[path = "value_expr_inline_tests.rs"]
mod tests;

#[path = "value_expr/value_expr_counter.rs"]
mod value_expr_counter_programs;
use value_expr_counter_programs::{
    first_counter_word, is_source_counter_reference, is_tagged_counter_reference,
};
#[path = "value_expr/value_expr_core.rs"]
mod value_expr_core_programs;
use value_expr_core_programs::{
    exact_one_of, first_rounding, parse_number_of_value, parse_value_expr_term_words, prefix_len,
    rounded_half, value_boundary,
};
#[path = "value_expr/value_expr_reference.rs"]
mod value_expr_reference_programs;
use value_expr_reference_programs::parse_source_controller_graveyard_filter;

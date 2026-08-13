use crate::cards::builders::IT_TAG;
use crate::effect::{EventValueSpec, Value};
use crate::grammar::{filters::parse_counter_type_words, leaf};
use crate::lexer::{OwnedLexToken, TokenWordView, synthetic_word_tokens};
use crate::object_filters::parse_object_filter_words;
use crate::util::{
    possessive_normalized_word_refs, source_choose_spec_for_surface,
    source_reference_surface_for_possessive_words, source_reference_surface_for_words,
    this_source_surface_for_words,
};
use crate::target::{
    ChooseSpec, ChooseSpecSurfaceHint, ObjectFilter, PlayerFilter, SacrificedObjectKind,
    SourceReferenceSurface,
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

fn tagged_characteristic_reference_tag(words: &[&str]) -> &'static str {
    if words.contains(&"exiled") {
        crate::tag::SOURCE_EXILED_TAG
    } else if words.contains(&"exploited") {
        crate::tag::EXPLOITED_TAG
    } else {
        IT_TAG
    }
}

fn sacrificed_object_kind(words: &[&str]) -> Option<SacrificedObjectKind> {
    words.windows(2).find_map(|pair| {
        if pair[0] != "sacrificed" {
            return None;
        }
        match pair[1] {
            "creature" | "creatures" | "creature's" => Some(SacrificedObjectKind::Creature),
            "artifact" | "artifacts" | "artifact's" => Some(SacrificedObjectKind::Artifact),
            "enchantment" | "enchantments" | "enchantment's" => {
                Some(SacrificedObjectKind::Enchantment)
            }
            "permanent" | "permanents" | "permanent's" => Some(SacrificedObjectKind::Permanent),
            _ => None,
        }
    })
}

fn with_sacrificed_object_surface(value: Value, words: &[&str]) -> Value {
    match sacrificed_object_kind(words) {
        Some(kind) => value.with_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
        None => value,
    }
}

pub(crate) fn colored_mana_symbols_in_costs(words: &[&str]) -> Option<(Value, usize)> {
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
            spec: Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
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

pub(crate) fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
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

pub(crate) fn parse_value_expr_tokens(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
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

fn parse_value_expr_term_words(words: &[&str]) -> Option<(Value, usize)> {
    if words.is_empty() {
        return None;
    }
    if let Some(devotion) = parse_devotion_value_words(words) {
        return Some(devotion);
    }
    for (phrase, player) in [
        (
            &[
                "the", "number", "of", "cards", "in", "the", "hand", "of", "the", "opponent",
                "with", "the", "most", "cards", "in", "hand",
            ][..],
            PlayerFilter::Opponent,
        ),
        (
            &[
                "the", "number", "of", "cards", "in", "the", "hand", "of", "an", "opponent",
                "with", "the", "most", "cards", "in", "hand",
            ][..],
            PlayerFilter::Opponent,
        ),
        (
            &[
                "the", "number", "of", "cards", "in", "the", "hand", "of", "the", "player", "with",
                "the", "most", "cards", "in", "hand",
            ][..],
            PlayerFilter::Any,
        ),
    ] {
        if permission_shapes::prefix_words(words, phrase) {
            return Some((Value::MaxCardsInHand(player), phrase.len()));
        }
    }
    if let Some(value) = colored_mana_symbols_in_costs(words) {
        return Some(value);
    }
    if permission_shapes::prefix_words(words, &["half"]) {
        if let Some((round_idx, rounding)) = first_rounding(&words[1..]) {
            let round_idx = round_idx + 1;
            let (base, used_inner) = parse_value_expr_term_words(&words[1..round_idx])?;
            if used_inner != round_idx - 1 {
                return None;
            }
            return Some((rounded_half(base, rounding), round_idx + 2));
        }
        let (base, used_inner) = parse_value_expr_term_words(&words[1..])?;
        let used = 1 + used_inner;
        if permission_shapes::starts_at_words(words, used, &["rounded", "down"]) {
            return Some((rounded_half(base, Rounding::Down), used + 2));
        }
        if permission_shapes::starts_at_words(words, used, &["rounded", "up"]) {
            return Some((rounded_half(base, Rounding::Up), used + 2));
        }
    }

    if let Some((_, used)) = DAMAGE_EVENT_AMOUNT_PREFIXES
        .iter()
        .find(|(expected, _)| permission_shapes::prefix_words(words, expected))
    {
        return Some((
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ValueSurfaceHint::DamageDealt),
            *used,
        ));
    }

    if let Some(used) = prefix_len(
        words,
        &[&["the", "result"], &["that", "result"], &["result"]],
    ) {
        return Some((
            Value::EventValue(EventValueSpec::Amount)
                .with_surface_hint(ValueSurfaceHint::PriorEffectResult),
            used,
        ));
    }

    if let Some((_, used)) = EVENT_AMOUNT_PREFIXES
        .iter()
        .find(|(expected, _)| permission_shapes::prefix_words(words, expected))
    {
        return Some((Value::EventValue(EventValueSpec::Amount), *used));
    }
    if permission_shapes::prefix_words(words, &["the", "other", "result"]) {
        return Some((
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::OtherNumber,
            },
            3,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "amount", "of", "mana", "spent", "to", "cast", "that", "spell",
            ],
            &[
                "amount", "of", "mana", "spent", "to", "cast", "that", "spell",
            ],
        ],
    ) {
        return Some((Value::ManaSpentToCastTriggeringObject, used));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "excess", "damage", "dealt", "to", "that", "creature", "this", "way",
            ],
            &[
                "excess", "damage", "dealt", "to", "that", "creature", "this", "way",
            ],
            &["the", "excess", "damage", "dealt", "this", "way"],
            &["excess", "damage", "dealt", "this", "way"],
            &["that", "amount", "of", "excess", "damage"],
        ],
    ) {
        return Some((
            Value::PendingEffectMetric {
                source: ironsmith_core::EffectMetricSource::Outcome,
                metric: ironsmith_core::EffectMetric::ExcessDamage,
            },
            used,
        ));
    }
    if words.len() >= 5
        && (permission_shapes::prefix_words(words, &["the", "number", "of"])
            || permission_shapes::prefix_words(words, &["number", "of"]))
        && permission_shapes::suffix_words(words, &["removed", "this", "way"])
    {
        return Some((Value::EventValue(EventValueSpec::Amount), words.len()));
    }
    if permission_shapes::prefix_words(words, &["twice", "x"]) {
        return Some((Value::XTimes(2), 2));
    }
    if permission_shapes::prefix_words(words, &["twice"]) {
        let (value, used) = parse_value_expr_term_words(&words[1..])?;
        return Some((Value::Scaled(Box::new(value), 2), used + 1));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "number", "of", "times", "this", "creature", "has", "mutated",
            ],
            &[
                "the",
                "number",
                "of",
                "times",
                "this",
                "permanent",
                "has",
                "mutated",
            ],
            &[
                "number", "of", "times", "this", "creature", "has", "mutated",
            ],
            &[
                "number",
                "of",
                "times",
                "this",
                "permanent",
                "has",
                "mutated",
            ],
            &[
                "number", "of", "times", "this", "creature", "has", "mutated",
            ],
            &["number", "of", "times", "this", "has", "mutated"],
            &["times", "this", "creature", "has", "mutated"],
            &["times", "this", "permanent", "has", "mutated"],
            &["times", "this", "has", "mutated"],
        ],
    ) {
        return Some((Value::SourceMutationCount, used));
    }
    if permission_shapes::prefix_words(words, &["x"]) {
        return Some((Value::X, 1));
    }
    if let Ok(value) = leaf::parse_number_i32_complete(words[0]) {
        return Some((Value::Fixed(value), 1));
    }

    for (characteristic, constructor) in [
        (
            &["mana", "value"][..],
            Value::ManaValueOf as fn(Box<ChooseSpec>) -> Value,
        ),
        (
            &["power"][..],
            Value::PowerOf as fn(Box<ChooseSpec>) -> Value,
        ),
        (
            &["toughness"][..],
            Value::ToughnessOf as fn(Box<ChooseSpec>) -> Value,
        ),
    ] {
        if let Some((kind, used)) =
            sacrificed_postpositive_characteristic_prefix(words, characteristic)
        {
            return Some((
                constructor(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
                    .with_surface_hint(ValueSurfaceHint::SacrificedObject(kind)),
                used,
            ));
        }
    }

    if let Some(used) = prefix_len(
        words,
        &[
            &["the", "amount", "of", "unspent", "mana", "you", "have"],
            &["amount", "of", "unspent", "mana", "you", "have"],
            &["unspent", "mana", "you", "have"],
        ],
    ) {
        return Some((Value::UnspentMana(PlayerFilter::You), used));
    }
    if permission_shapes::prefix_words(words, &["your", "life", "total"]) {
        return Some((Value::LifeTotal(PlayerFilter::You), 3));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &[
                "the", "amount", "of", "life", "you", "gained", "this", "turn",
            ],
            &["amount", "of", "life", "you", "gained", "this", "turn"],
        ],
    ) {
        return Some((Value::LifeGainedThisTurn(PlayerFilter::You), used));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["target", "players", "life", "total"],
            &["target", "player", "life", "total"],
            &["that", "players", "life", "total"],
            &["that", "player", "life", "total"],
        ],
    ) {
        return Some((Value::LifeTotal(PlayerFilter::target_player()), used));
    }
    if permission_shapes::prefix_words(words, &["your", "speed"]) {
        return Some((Value::Speed(PlayerFilter::You), 2));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["target", "players", "speed"],
            &["target", "player", "speed"],
            &["that", "players", "speed"],
            &["that", "player", "speed"],
        ],
    ) {
        return Some((Value::Speed(PlayerFilter::target_player()), used));
    }

    for source_len in (1..words.len()).rev() {
        if let Some(surface) = source_reference_surface_for_possessive_words(&words[..source_len]) {
            match words.get(source_len).copied() {
                Some("power") => {
                    return Some((
                        Value::PowerOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 1,
                    ));
                }
                Some("toughness") => {
                    return Some((
                        Value::ToughnessOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 1,
                    ));
                }
                Some("mana")
                    if permission_shapes::starts_at_words(words, source_len + 1, &["value"]) =>
                {
                    return Some((
                        Value::ManaValueOf(Box::new(source_choose_spec_for_surface(surface))),
                        source_len + 2,
                    ));
                }
                _ => {}
            }
        }
    }

    if permission_shapes::prefix_words(words, &["its", "power"]) {
        return Some((
            Value::PowerOf(Box::new(
                ChooseSpec::Tagged(TagKey::from(IT_TAG)).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            2,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["this", "power"],
            &["thiss", "power"],
            &["this", "creature", "power"],
            &["thiss", "creature", "power"],
            &["this", "creatures", "power"],
            &["thiss", "creatures", "power"],
        ],
    ) {
        return Some((Value::SourcePower, used));
    }
    if permission_shapes::prefix_words(words, &["his", "power"]) {
        return Some((
            Value::SourcePower
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::MasculineSourcePossessive),
            2,
        ));
    }
    if permission_shapes::prefix_words(words, &["her", "power"]) {
        return Some((
            Value::SourcePower
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::FeminineSourcePossessive),
            2,
        ));
    }
    if permission_shapes::prefix_words(words, &["its", "toughness"]) {
        return Some((
            Value::ToughnessOf(Box::new(
                ChooseSpec::Tagged(TagKey::from(IT_TAG)).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            2,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["this", "toughness"],
            &["thiss", "toughness"],
            &["this", "creature", "toughness"],
            &["thiss", "creature", "toughness"],
            &["this", "creatures", "toughness"],
            &["thiss", "creatures", "toughness"],
        ],
    ) {
        return Some((Value::SourceToughness, used));
    }
    if permission_shapes::prefix_words(words, &["its", "mana", "value"]) {
        return Some((
            Value::ManaValueOf(Box::new(
                ChooseSpec::Tagged(TagKey::from(IT_TAG)).with_surface_hint(
                    ChooseSpecSurfaceHint::SourceReference(
                        SourceReferenceSurface::ThisPermanentType("it".to_string()),
                    ),
                ),
            )),
            3,
        ));
    }
    if let Some(used) = prefix_len(
        words,
        &[
            &["this", "mana", "value"],
            &["thiss", "mana", "value"],
            &["this", "creature", "mana", "value"],
            &["thiss", "creature", "mana", "value"],
            &["this", "creatures", "mana", "value"],
            &["thiss", "creatures", "mana", "value"],
        ],
    ) {
        return Some((
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)))),
            used,
        ));
    }
    // In an attack-group trigger, plural "their" denotes the creatures that
    // jointly satisfied the one-or-more trigger, not every creature currently
    // on the battlefield.  Trigger queuing captures that exact group under
    // ATTACKING_GROUP_TAG so the value remains stable while the ability is on
    // the stack.
    if permission_shapes::prefix_words(words, &["their", "total", "power"]) {
        return Some((
            Value::TotalPower(crate::target::ObjectFilter::tagged(
                ironsmith_core::ATTACKING_GROUP_TAG,
            )),
            3,
        ));
    }
    if let Some(used) = prefix_len(words, COLORS_SPENT_PREFIXES) {
        return Some((Value::ColorsOfManaSpentToCastThisSpell, used));
    }
    const ITERATED_PLAYER_EXILED_OBJECT_POWER: &[&str] =
        &["the", "power", "of", "the", "creature", "they", "exiled"];
    if permission_shapes::prefix_words(words, ITERATED_PLAYER_EXILED_OBJECT_POWER) {
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::FirstPower,
        )
        .with_filter(ObjectFilter::creature())
        .with_player(PlayerFilter::IteratedPlayer)
        .with_action(ironsmith_core::PriorEffectAction::Exiled);
        return Some((
            Value::PendingPriorEffectMetric(query),
            ITERATED_PLAYER_EXILED_OBJECT_POWER.len(),
        ));
    }
    if let Some(used) = prefix_len(words, TAGGED_POWER_PREFIXES) {
        let tag = tagged_characteristic_reference_tag(&words[..used]);
        return Some((
            with_sacrificed_object_surface(
                Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(tag)))),
                &words[..used],
            ),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, TAGGED_TOUGHNESS_PREFIXES) {
        let tag = tagged_characteristic_reference_tag(&words[..used]);
        return Some((
            with_sacrificed_object_surface(
                Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(tag)))),
                &words[..used],
            ),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, EXILED_MANA_VALUE_PREFIXES) {
        return Some((
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                crate::tag::SOURCE_EXILED_TAG,
            )))),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, REVEALED_MANA_VALUE_PREFIXES) {
        return Some((
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                "__public_revealed",
            ))))
            .with_surface_hint(ValueSurfaceHint::RevealedCardReference),
            used,
        ));
    }
    if let Some(used) = prefix_len(words, TAGGED_MANA_VALUE_PREFIXES) {
        return Some((
            with_sacrificed_object_surface(
                Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)))),
                &words[..used],
            ),
            used,
        ));
    }
    if let Some(value) = value_helper_shapes::parse_aggregate_scope_value_words(words) {
        return Some((value, words.len()));
    }
    if let Some(value) = value_helper_shapes::parse_spells_cast_this_turn_value_words(words) {
        return Some((value, words.len()));
    }

    parse_number_of_value(words)
}

fn parse_number_of_value(words: &[&str]) -> Option<(Value, usize)> {
    let mut idx = usize::from(permission_shapes::prefix_words(words, &["the"]));
    if !permission_shapes::starts_at_words(words, idx, &["number", "of"]) {
        return None;
    }
    idx += 2;
    // A singular discarded-card characteristic is a metric over the exact
    // result of the preceding discard, not a count of live objects matching
    // the words `card types`. Keeping the action on the pending query lets
    // reference resolution bind it to the producing discard effect.
    const DISCARDED_CARD_TYPES: &[&str] = &["card", "types", "the", "discarded", "card", "has"];
    if permission_shapes::starts_at_words(words, idx, DISCARDED_CARD_TYPES) {
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::CardTypesAmong,
        )
        .with_action(ironsmith_core::PriorEffectAction::Discarded);
        return Some((
            Value::PendingPriorEffectMetric(query),
            idx + DISCARDED_CARD_TYPES.len(),
        ));
    }
    for visit_surface in [
        &["attractions", "youve", "visited", "this", "turn"][..],
        &["attractions", "you've", "visited", "this", "turn"][..],
        &["attraction", "youve", "visited", "this", "turn"][..],
        &["attraction", "you've", "visited", "this", "turn"][..],
    ] {
        if permission_shapes::starts_at_words(words, idx, visit_surface) {
            return Some((
                Value::AttractionsVisitedThisTurn(PlayerFilter::You),
                idx + visit_surface.len(),
            ));
        }
    }
    if let Some(character_word) = words.get(idx)
        && let Some(character) = character_word
            .strip_suffix("'s")
            .or_else(|| character_word.strip_suffix("’s"))
            .or_else(|| character_word.strip_suffix('s'))
        && character.chars().count() == 1
        && character.chars().all(|character| character.is_alphabetic())
        && words.get(idx + 1..idx + 5) == Some(&["in", "name", "stickers", "on"][..])
    {
        let reference_start = idx + 5;
        let reference_end = value_boundary(&words[reference_start..]) + reference_start;
        let reference = words.get(reference_start..reference_end)?;
        let surface = source_reference_surface_for_words(reference)
            .or_else(|| this_source_surface_for_words(reference))?;
        return Some((
            Value::NameStickerCharacterCountOnSource {
                character: character.chars().next()?.to_ascii_lowercase(),
                surface: Some(surface),
            },
            reference_end,
        ));
    }
    let mut counter_descriptor_start = idx;
    if words
        .get(counter_descriptor_start)
        .is_some_and(|word| leaf::parse_leaf_article_complete(word).is_ok())
        || permission_shapes::starts_at_words(words, counter_descriptor_start, &["one"])
    {
        counter_descriptor_start += 1;
    }
    if let Some(counter_idx) = first_counter_word(&words[counter_descriptor_start..])
        .map(|relative| counter_descriptor_start + relative)
        .filter(|counter_idx| counter_idx.saturating_sub(counter_descriptor_start) <= 2)
        && let Some(counter_type) = (counter_idx > counter_descriptor_start)
            .then(|| parse_counter_type_words(&words[counter_descriptor_start..=counter_idx]))
            .flatten()
    {
        if permission_shapes::starts_at_words(words, counter_idx + 1, &["you", "have"]) {
            return Some((
                Value::PlayerCounters(PlayerFilter::You, counter_type),
                counter_idx + 3,
            ));
        }
        if words
            .get(counter_idx + 1)
            .is_some_and(|word| matches!(*word, "youve" | "you've"))
        {
            return Some((
                Value::PlayerCounters(PlayerFilter::You, counter_type),
                counter_idx + 2,
            ));
        }
    }
    if let Some(counter_idx) = first_counter_word(&words[counter_descriptor_start..])
        .map(|relative| counter_descriptor_start + relative)
        .filter(|counter_idx| counter_idx.saturating_sub(counter_descriptor_start) <= 2)
        && permission_shapes::starts_at_words(words, counter_idx + 1, &["on"])
    {
        let parsed_counter_type = (counter_idx > counter_descriptor_start)
            .then(|| parse_counter_type_words(&words[counter_descriptor_start..=counter_idx]))
            .flatten();
        let reference_start = counter_idx + 2;
        let reference_end = value_boundary(&words[reference_start..]) + reference_start;
        let reference = &words[reference_start..reference_end];
        if is_source_counter_reference(reference) {
            let value = match parsed_counter_type {
                Some(counter_type) => {
                    if let Some(surface) = source_reference_surface_for_words(reference) {
                        Value::CountersOn(
                            Box::new(source_choose_spec_for_surface(surface)),
                            Some(counter_type),
                        )
                    } else {
                        Value::CountersOnSource(counter_type)
                    }
                }
                None => Value::CountersOn(
                    Box::new(
                        source_reference_surface_for_words(reference)
                            .map(source_choose_spec_for_surface)
                            .unwrap_or(ChooseSpec::Source),
                    ),
                    None,
                ),
            };
            return Some((value, reference_end));
        }
        if let Some(surface) = source_reference_surface_for_words(reference) {
            return Some((
                Value::CountersOn(
                    Box::new(source_choose_spec_for_surface(surface)),
                    parsed_counter_type,
                ),
                reference_end,
            ));
        }
        if is_tagged_counter_reference(reference) {
            return Some((
                Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    parsed_counter_type,
                ),
                reference_end,
            ));
        }
        if let Ok(filter) = parse_object_filter_words(reference, false) {
            return Some((
                Value::CountersOn(Box::new(ChooseSpec::All(filter)), parsed_counter_type),
                reference_end,
            ));
        }
    }

    let filter_start = idx;
    let filter_end = value_boundary(&words[filter_start..]) + filter_start;
    if filter_end <= filter_start {
        return None;
    }
    let filter_words = &words[filter_start..filter_end];
    let history_tokens = synthetic_word_tokens(filter_words);
    if let Some(value) = super::value_semantics::parse_turn_history_count_value(&history_tokens) {
        return Some((value, filter_end));
    }
    // Keep a qualifying hand-size predicate attached to the players it
    // describes. The generic object-filter fallback below otherwise turns
    // this into a count of cards across every player's hand.
    if let Some((players, minimum)) =
        super::value_semantics::parse_players_with_cards_in_hand_at_least(&history_tokens)
    {
        return Some((
            Value::CountPlayersWithCardsInHandAtLeast(players, minimum),
            filter_end,
        ));
    }
    if exact_one_of(
        filter_words,
        &[
            &["creatures", "in", "your", "party"],
            &["creature", "in", "your", "party"],
        ],
    ) {
        return Some((Value::PartySize(PlayerFilter::You), filter_end));
    }
    if let Some(value) = value_helper_shapes::parse_aggregate_scope_value_words(filter_words) {
        return Some((value, filter_end));
    }
    if let Some(value) = value_helper_shapes::parse_spells_cast_this_turn_value_words(filter_words)
    {
        return Some((value, filter_end));
    }
    // In an amount modifying a player-directed action, plural `them` is the
    // same player antecedent. Curses are player attachments, so keep that
    // relation typed instead of allowing the generic object-pronoun parser to
    // manufacture an attached card selector.
    if matches!(filter_words, ["curse" | "curses", "attached", "to", "them"]) {
        let mut filter = ObjectFilter::default().with_subtype(crate::Subtype::Curse);
        filter.zone = Some(crate::zone::Zone::Battlefield);
        filter.attached_to_player = Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
        return Some((Value::Count(filter), filter_end));
    }
    // A possessive target-controller hand is a player-relative zone scope,
    // not a characteristic on the counted cards. Parse it before the generic
    // object-filter fallback can absorb `that creature` as a Creature type.
    let possessive = possessive_normalized_word_refs(filter_words);
    if matches!(
        possessive.as_slice(),
        [
            "cards" | "card",
            "in",
            "that",
            "creature" | "creatures" | "permanent" | "permanents" | "object" | "objects",
            "controller" | "controllers",
            "hand" | "hands"
        ]
    ) {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(crate::zone::Zone::Hand);
        filter.owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target));
        return Some((Value::Count(filter), filter_end));
    }
    // The generic value-expression path runs before several effect-specific
    // value parsers. Preserve every typed prior-action link here so numeric
    // phrases such as "the number of creatures destroyed this way" and
    // "twice the number of Mountains returned this way" do not collapse to
    // live-zone object counts.
    if permission_shapes::find_words(filter_words, &["this", "way"]).is_some() {
        let mut for_each_words = Vec::with_capacity(filter_words.len() + 2);
        for_each_words.extend(["for", "each"]);
        for_each_words.extend(filter_words.iter().copied());
        if let Some((value @ Value::PendingPriorEffectMetric(_), used)) =
            super::count_shapes::parse_for_each_count_value_words(&for_each_words)
            && used == for_each_words.len()
        {
            return Some((value, filter_end));
        }
    }
    if let Some(mut filter) = parse_source_controller_graveyard_filter(filter_words) {
        filter.zone = Some(crate::zone::Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        return Some((Value::Count(filter), filter_end));
    }
    let filter = parse_object_filter_words(filter_words, false).ok()?;
    let mut value = Value::Count(filter);
    if value_helper_shapes::has_that_player_possessive(filter_words) {
        value = value.with_surface_hint(ValueSurfaceHint::ThatPlayerPossessive);
    }
    Some((value, filter_end))
}

fn parse_source_controller_graveyard_filter(words: &[&str]) -> Option<crate::target::ObjectFilter> {
    const POSSESSIVE_GRAVEYARD_SUFFIXES: &[&[&str]] = &[
        &["in", "its", "controller", "graveyard"],
        &["in", "its", "controllers", "graveyard"],
    ];
    let suffix = POSSESSIVE_GRAVEYARD_SUFFIXES
        .iter()
        .find(|suffix| permission_shapes::suffix_words(words, suffix))?;
    let object_words = words.get(..words.len().checked_sub(suffix.len())?)?;
    (!object_words.is_empty())
        .then(|| parse_object_filter_words(object_words, false).ok())
        .flatten()
}

fn first_rounding(words: &[&str]) -> Option<(usize, Rounding)> {
    let down =
        permission_shapes::find_words(words, &["rounded", "down"]).map(|idx| (idx, Rounding::Down));
    let up =
        permission_shapes::find_words(words, &["rounded", "up"]).map(|idx| (idx, Rounding::Up));
    match (down, up) {
        (Some(down), Some(up)) => Some(if down.0 <= up.0 { down } else { up }),
        (Some(down), None) => Some(down),
        (None, Some(up)) => Some(up),
        (None, None) => None,
    }
}

fn rounded_half(base: Value, rounding: Rounding) -> Value {
    match rounding {
        Rounding::Down => Value::HalfRoundedDown(Box::new(base)),
        Rounding::Up => Value::HalfRoundedDown(Box::new(Value::Add(
            Box::new(base),
            Box::new(Value::Fixed(1)),
        ))),
    }
}

fn prefix_len(words: &[&str], alternatives: &[&[&str]]) -> Option<usize> {
    alternatives
        .iter()
        .find(|expected| permission_shapes::prefix_words(words, expected))
        .map(|expected| expected.len())
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

fn value_boundary(words: &[&str]) -> usize {
    let arithmetic = ["plus", "minus"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
        .unwrap_or(words.len());
    let in_excess =
        permission_shapes::find_words(words, &["in", "excess", "of"]).unwrap_or(words.len());
    // A "from <zone>" right after a controller/owner clause is the enclosing
    // effect's movement source, never part of the count basis: "the number
    // of lands you control from your hand onto the battlefield" must count
    // battlefield lands, not land cards in hand.
    let movement_source = words
        .windows(2)
        .enumerate()
        .find(|(_, pair)| {
            matches!(pair[0], "control" | "controls" | "own" | "owns") && pair[1] == "from"
        })
        .map(|(idx, _)| idx + 1)
        .unwrap_or(words.len());
    arithmetic.min(in_excess).min(movement_source)
}

fn first_counter_word(words: &[&str]) -> Option<usize> {
    ["counter", "counters"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
}

fn is_source_counter_reference(words: &[&str]) -> bool {
    exact_one_of(
        words,
        &[
            &["it"],
            &["this"],
            &["this", "card"],
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
            &["this", "artifact"],
            &["this", "land"],
            &["this", "enchantment"],
        ],
    )
}

fn is_tagged_counter_reference(words: &[&str]) -> bool {
    exact_one_of(
        words,
        &[
            &["that"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
            &["those"],
            &["those", "cards"],
            &["those", "creatures"],
            &["those", "permanents"],
        ],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object::CounterType;
    use crate::runtime_backend::lexer::lex_line;
    use crate::target::SourceReferenceSurface;

    #[test]
    fn parses_rounded_and_tagged_value_expressions() {
        assert_eq!(
            parse_value_expr_words(&["half", "x", "rounded", "up"]),
            Some((
                Value::HalfRoundedDown(Box::new(Value::Add(
                    Box::new(Value::X),
                    Box::new(Value::Fixed(1)),
                ))),
                4,
            ))
        );
        assert_eq!(
            parse_value_expr_words(&["the", "exploited", "creature", "power"]),
            Some((
                Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                    crate::tag::EXPLOITED_TAG,
                )))),
                4,
            ))
        );
    }

    #[test]
    fn counted_cards_in_target_creature_controller_hand_keep_player_scope() {
        let tokens = lex_line(
            "the number of cards in that creature's controller's hand",
            0,
        )
        .expect("target-controller hand count should lex");
        let (value, used) =
            parse_value_expr_tokens(&tokens).expect("target-controller hand count should parse");
        assert_eq!(used, tokens.len());
        let Value::Count(filter) = value else {
            panic!("expected typed object count: {value:?}");
        };
        assert_eq!(filter.zone, Some(crate::zone::Zone::Hand));
        assert!(filter.card_types.is_empty());
        assert_eq!(
            filter.owner,
            Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target))
        );

        let ordinary = lex_line("the number of creature cards in all players' hands", 0)
            .expect("ordinary hand count should lex");
        let (ordinary, _) = parse_value_expr_tokens(&ordinary).expect("ordinary count");
        let Value::Count(ordinary) = ordinary else {
            panic!("expected ordinary object count: {ordinary:?}");
        };
        assert_eq!(ordinary.card_types, vec![crate::CardType::Creature]);
        assert_ne!(ordinary.owner, filter.owner);
    }

    #[test]
    fn counted_curses_attached_to_them_keep_player_attachment_scope() {
        let tokens = lex_line("the number of Curses attached to them", 0)
            .expect("attached Curse count should lex");
        let (value, used) =
            parse_value_expr_tokens(&tokens).expect("attached Curse count should parse");
        assert_eq!(used, tokens.len());
        let Value::Count(filter) = value else {
            panic!("expected attached-Curse object count: {value:?}");
        };
        assert!(filter.attached_to_object.is_none());
        assert_eq!(
            filter.attached_to_player,
            Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)))
        );

        let object_attachments = lex_line("the number of Auras attached to them", 0)
            .expect("object attachment near miss should lex");
        let (object_attachments, _) = parse_value_expr_tokens(&object_attachments)
            .expect("object attachment near miss should still parse");
        let Value::Count(object_attachments) = object_attachments else {
            panic!("expected ordinary object attachment count");
        };
        assert!(object_attachments.attached_to_player.is_none());
    }

    #[test]
    fn parses_character_count_in_source_name_stickers() {
        let tokens = lex_line("the number of o's in name stickers on this enchantment", 0)
            .expect("name-sticker character-count fixture should lex");
        let (value, used) =
            parse_value_expr_tokens(&tokens).expect("name-sticker character count should parse");
        assert_eq!(used, tokens.len());
        assert_eq!(
            value,
            Value::NameStickerCharacterCountOnSource {
                character: 'o',
                surface: Some(SourceReferenceSurface::ThisPermanentType(
                    "this enchantment".to_string()
                )),
            }
        );
    }

    #[test]
    fn possessive_it_characteristics_keep_the_object_antecedent() {
        assert_eq!(
            parse_value_expr_words(&["its", "power"]),
            Some((
                Value::PowerOf(Box::new(
                    ChooseSpec::Tagged(TagKey::from(IT_TAG)).with_surface_hint(
                        ChooseSpecSurfaceHint::SourceReference(
                            SourceReferenceSurface::ThisPermanentType("it".to_string()),
                        ),
                    ),
                )),
                2,
            ))
        );
        assert_eq!(
            parse_value_expr_words(&["its", "toughness"]),
            Some((
                Value::ToughnessOf(Box::new(
                    ChooseSpec::Tagged(TagKey::from(IT_TAG)).with_surface_hint(
                        ChooseSpecSurfaceHint::SourceReference(
                            SourceReferenceSurface::ThisPermanentType("it".to_string()),
                        ),
                    ),
                )),
                2,
            ))
        );
        assert_eq!(
            parse_value_expr_words(&["this", "creatures", "toughness"]),
            Some((Value::SourceToughness, 3))
        );
    }

    #[test]
    fn parses_maximum_hand_size_as_a_bound_player_aggregate() {
        assert_eq!(
            parse_value_expr_words(&[
                "the", "number", "of", "cards", "in", "the", "hand", "of", "the", "opponent",
                "with", "the", "most", "cards", "in", "hand",
            ]),
            Some((Value::MaxCardsInHand(PlayerFilter::Opponent), 16)),
        );
    }

    #[test]
    fn preserves_token_boundary_for_value_prefixes() {
        let tokens = lex_line("x plus two cards", 0).expect("lex fixture");
        assert_eq!(
            parse_value_expr_tokens(&tokens),
            Some((Value::Add(Box::new(Value::X), Box::new(Value::Fixed(2))), 3,))
        );
    }

    #[test]
    fn parses_dynamic_subtraction_and_in_excess_of_as_composable_values() {
        for (operator, in_excess_of) in [
            (["minus"].as_slice(), false),
            (["in", "excess", "of"].as_slice(), true),
        ] {
            let mut words = vec!["number", "of", "creatures", "you", "control"];
            words.extend_from_slice(operator);
            words.extend([
                "number",
                "of",
                "creatures",
                "target",
                "opponent",
                "controls",
            ]);

            let (value, used) =
                parse_value_expr_words(&words).expect("dynamic difference should parse");
            assert_eq!(used, words.len());
            assert_eq!(
                value.has_surface_hint(ValueSurfaceHint::InExcessOf),
                in_excess_of,
            );
            let Value::Add(left, right) = value.unhinted() else {
                panic!("difference should be represented as composable addition");
            };
            assert!(matches!(left.as_ref(), Value::Count(_)));
            assert!(
                matches!(right.as_ref(), Value::Scaled(inner, -1) if matches!(inner.as_ref(), Value::Count(_)))
            );
        }
    }

    #[test]
    fn hand_count_preserves_authored_that_player_possessive() {
        for (owner_words, expected_hint) in [
            (["that", "players"].as_slice(), true),
            (["their"].as_slice(), false),
        ] {
            let mut words = vec!["the", "number", "of", "cards", "in"];
            words.extend_from_slice(owner_words);
            words.push("hand");

            let (value, used) =
                parse_value_expr_words(&words).expect("player-relative hand count should parse");
            assert_eq!(used, words.len());
            assert_eq!(
                value.has_surface_hint(ValueSurfaceHint::ThatPlayerPossessive),
                expected_hint,
                "{words:?}: {value:#?}"
            );
            assert!(matches!(
                value.unhinted(),
                Value::Count(filter)
                    if filter.zone == Some(crate::zone::Zone::Hand)
                        && filter.owner == Some(PlayerFilter::IteratedPlayer)
            ));
        }
    }

    #[test]
    fn parses_triggering_cast_mana_and_excess_damage_values() {
        assert_eq!(
            parse_value_expr_words(&["the", "excess"]),
            Some((Value::EventValue(EventValueSpec::Amount), 2))
        );
        assert_eq!(
            parse_value_expr_words(&[
                "the", "amount", "of", "mana", "spent", "to", "cast", "that", "spell",
            ]),
            Some((Value::ManaSpentToCastTriggeringObject, 9))
        );
        assert_eq!(
            parse_value_expr_words(&[
                "the", "excess", "damage", "dealt", "to", "that", "creature", "this", "way",
            ]),
            Some((
                Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::ExcessDamage,
                },
                9,
            ))
        );
    }

    #[test]
    fn parses_life_total_player_counter_and_source_controller_graveyard_values() {
        assert_eq!(
            parse_value_expr_words(&["your", "life", "total"]),
            Some((Value::LifeTotal(PlayerFilter::You), 3))
        );
        assert_eq!(
            parse_value_expr_words(&[
                "the", "amount", "of", "life", "you", "gained", "this", "turn",
            ]),
            Some((Value::LifeGainedThisTurn(PlayerFilter::You), 8))
        );
        assert_eq!(
            parse_value_expr_words(&[
                "the",
                "number",
                "of",
                "experience",
                "counters",
                "you",
                "have",
            ]),
            Some((
                Value::PlayerCounters(PlayerFilter::You, CounterType::Experience),
                7,
            ))
        );

        let (value, used) = parse_value_expr_words(&[
            "the",
            "number",
            "of",
            "creature",
            "cards",
            "in",
            "its",
            "controller",
            "graveyard",
        ])
        .expect("source-controller graveyard count");
        assert_eq!(used, 9);
        let Value::Count(filter) = value else {
            panic!("expected object count");
        };
        assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
        assert_eq!(filter.zone, Some(crate::zone::Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
    }

    #[test]
    fn count_value_preserves_player_or_planeswalker_controller_reference() {
        let words = [
            "the",
            "number",
            "of",
            "creatures",
            "that",
            "opponent",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "controls",
        ];
        let (value, used) =
            parse_value_expr_words(&words).expect("controller-relative count should parse");

        assert_eq!(used, words.len());
        let Value::Count(filter) = value else {
            panic!("expected an object count");
        };
        assert_eq!(filter.card_types, vec![crate::types::CardType::Creature]);
        assert_eq!(
            filter.controller,
            Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        );
        assert!(
            !filter
                .card_types
                .contains(&crate::types::CardType::Planeswalker)
        );
    }

    #[test]
    fn generic_number_of_value_preserves_tapped_this_way_link() {
        let (value, used) =
            parse_value_expr_words(&["the", "number", "of", "creatures", "tapped", "this", "way"])
                .expect("tapped-this-way count");

        assert_eq!(used, 7);
        let Value::PendingPriorEffectMetric(query) = value else {
            panic!("expected typed prior-effect metric");
        };
        assert_eq!(
            query.source,
            ironsmith_core::EffectMetricSource::AffectedObjects
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Tapped)
        );
        assert_eq!(
            query.filter.expect("creature filter").card_types,
            vec![crate::types::CardType::Creature]
        );
    }

    #[test]
    fn discarded_card_type_count_binds_to_the_discard_result() {
        let words = [
            "the",
            "number",
            "of",
            "card",
            "types",
            "the",
            "discarded",
            "card",
            "has",
        ];
        let (value, used) =
            parse_value_expr_words(&words).expect("discarded-card type count should parse");

        assert_eq!(used, words.len());
        let Value::PendingPriorEffectMetric(query) = value else {
            panic!("expected a typed prior-effect metric, got {value:?}");
        };
        assert_eq!(
            query.source,
            ironsmith_core::EffectMetricSource::AffectedObjects
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::CardTypesAmong);
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Discarded)
        );
    }

    #[test]
    fn iterated_players_exiled_creature_power_keeps_partitioned_provenance() {
        let words = ["the", "power", "of", "the", "creature", "they", "exiled"];
        let (value, used) =
            parse_value_expr_words(&words).expect("per-player exiled creature power should parse");

        assert_eq!(used, words.len());
        let Value::PendingPriorEffectMetric(query) = value else {
            panic!("expected a typed prior-effect metric")
        };
        assert_eq!(query.metric, ironsmith_core::EffectMetric::FirstPower);
        assert_eq!(query.player, Some(PlayerFilter::IteratedPlayer));
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Exiled)
        );
        assert_eq!(
            query.filter.expect("creature filter").card_types,
            vec![crate::types::CardType::Creature]
        );
    }

    #[test]
    fn generic_number_of_value_keeps_hand_threshold_on_qualified_players() {
        let words = [
            "the",
            "number",
            "of",
            "your",
            "opponents",
            "with",
            "four",
            "or",
            "more",
            "cards",
            "in",
            "hand",
        ];
        let (value, used) =
            parse_value_expr_words(&words).expect("qualified player count should parse");

        assert_eq!(used, words.len());
        assert_eq!(
            value,
            Value::CountPlayersWithCardsInHandAtLeast(PlayerFilter::Opponent, 4)
        );
    }

    #[test]
    fn sacrificed_characteristic_values_keep_identity_and_typed_surface() {
        let sacrificed_creature =
            Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
                .with_surface_hint(ValueSurfaceHint::SacrificedObject(
                    SacrificedObjectKind::Creature,
                ));
        assert_eq!(
            parse_value_expr_words(&["the", "sacrificed", "creature", "toughness"]),
            Some((sacrificed_creature, 4))
        );

        let sacrificed_permanent =
            Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
                .with_surface_hint(ValueSurfaceHint::SacrificedObject(
                    SacrificedObjectKind::Permanent,
                ));
        assert_eq!(
            parse_value_expr_words(&[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "permanent",
            ]),
            Some((sacrificed_permanent, 7))
        );

        let red_symbols = Value::ManaSymbolsInManaCostOf {
            spec: Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))),
            color: Color::Red,
        }
        .with_surface_hint(ValueSurfaceHint::SacrificedObject(
            SacrificedObjectKind::Creature,
        ));
        assert_eq!(
            parse_value_expr_words(&[
                "the",
                "number",
                "of",
                "red",
                "mana",
                "symbols",
                "in",
                "the",
                "sacrificed",
                "creatures",
                "mana",
                "cost",
            ]),
            Some((red_symbols, 12))
        );
    }

    #[test]
    fn parses_colored_mana_symbols_across_filtered_scopes() {
        let battlefield_words = [
            "the",
            "number",
            "of",
            "green",
            "mana",
            "symbols",
            "in",
            "the",
            "mana",
            "costs",
            "of",
            "permanents",
            "you",
            "control",
        ];
        let (value, used) = parse_value_expr_words(&battlefield_words)
            .expect("battlefield mana-symbol aggregate should parse");
        assert_eq!(used, battlefield_words.len());
        let Value::ManaSymbolsInManaCostOf { spec, color } = value else {
            panic!("expected structured mana-symbol value");
        };
        assert_eq!(color, Color::Green);
        let ChooseSpec::All(filter) = spec.unhinted() else {
            panic!("expected aggregate object scope");
        };
        assert_eq!(filter.zone, Some(crate::zone::Zone::Battlefield));
        assert_eq!(filter.controller, Some(PlayerFilter::You));

        let graveyard_words = [
            "the",
            "number",
            "of",
            "black",
            "mana",
            "symbols",
            "in",
            "the",
            "mana",
            "costs",
            "of",
            "cards",
            "in",
            "your",
            "graveyard",
        ];
        let (value, used) = parse_value_expr_words(&graveyard_words)
            .expect("graveyard mana-symbol aggregate should parse");
        assert_eq!(used, graveyard_words.len());
        let Value::ManaSymbolsInManaCostOf { spec, color } = value else {
            panic!("expected structured mana-symbol value");
        };
        assert_eq!(color, Color::Black);
        let ChooseSpec::All(filter) = spec.unhinted() else {
            panic!("expected aggregate object scope");
        };
        assert_eq!(filter.zone, Some(crate::zone::Zone::Graveyard));
        assert_eq!(filter.owner, Some(PlayerFilter::You));
    }

    #[test]
    fn explicit_revealed_card_mana_value_keeps_reference_surface() {
        let (value, used) = parse_value_expr_words(&["the", "revealed", "card", "mana", "value"])
            .expect("revealed-card mana value");

        assert_eq!(used, 5);
        assert!(value.has_surface_hint(ValueSurfaceHint::RevealedCardReference));
        assert!(matches!(
            value.unhinted(),
            Value::ManaValueOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "__public_revealed")
        ));
    }

    #[test]
    fn its_characteristic_keeps_the_pronoun_on_the_object_reference() {
        let (value, used) =
            parse_value_expr_words(&["its", "mana", "value"]).expect("possessive mana value");
        assert_eq!(used, 3);
        let Value::ManaValueOf(spec) = value else {
            panic!("expected a typed mana-value reference");
        };
        assert!(matches!(
            spec.base(),
            ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG
        ));
        assert_eq!(
            spec.source_reference_surface(),
            Some(&SourceReferenceSurface::ThisPermanentType("it".to_string()))
        );
    }

    #[test]
    fn devotion_is_a_typed_value_expression_only_with_a_proven_owner_and_color() {
        assert_eq!(
            parse_value_expr_words(&["your", "devotion", "to", "black"]),
            Some((
                Value::Devotion {
                    player: PlayerFilter::You,
                    color: Color::Black,
                },
                4,
            ))
        );
        assert_eq!(
            parse_value_expr_words(&["their", "devotion", "to", "blue"]),
            Some((
                Value::Devotion {
                    player: PlayerFilter::IteratedPlayer,
                    color: Color::Blue,
                },
                4,
            ))
        );
        assert_eq!(
            parse_value_expr_words(&["your", "devotion", "to", "that", "color"]),
            Some((Value::DevotionToChosenColor(PlayerFilter::You), 5))
        );
        assert_eq!(
            parse_value_expr_words(&["your", "devotion", "for", "black"]),
            None,
            "near-miss prepositions must not become a devotion value"
        );
    }

    #[test]
    fn whichever_is_greater_builds_an_executable_maximum() {
        let words = [
            "the",
            "number",
            "of",
            "zombies",
            "you",
            "control",
            "or",
            "the",
            "number",
            "of",
            "zombie",
            "cards",
            "in",
            "your",
            "graveyard",
            "whichever",
            "is",
            "greater",
        ];
        let (value, used) = parse_value_expr_words(&words).expect("maximum value should parse");
        assert_eq!(used, words.len());
        assert!(value.has_surface_hint(ValueSurfaceHint::WhicheverIsGreater));
        assert!(matches!(
            value.unhinted(),
            Value::Add(total, negative_minimum)
                if matches!(total.as_ref(), Value::Add(_, _))
                    && matches!(
                        negative_minimum.as_ref(),
                        Value::Scaled(minimum, -1)
                            if matches!(minimum.as_ref(), Value::Min(_, _))
                    )
        ));
    }
}

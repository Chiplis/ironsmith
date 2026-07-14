use crate::cards::builders::IT_TAG;
use crate::effect::{EventValueSpec, Value};
use crate::runtime_backend::grammar::{filters::parse_counter_type_words, leaf};
use crate::runtime_backend::lexer::{OwnedLexToken, TokenWordView, synthetic_word_tokens};
use crate::runtime_backend::object_filters::parse_object_filter_words;
use crate::runtime_backend::util::{
    source_choose_spec_for_surface, source_reference_surface_for_possessive_words,
    source_reference_surface_for_words,
};
use crate::target::{ChooseSpec, PlayerFilter, SacrificedObjectKind};
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
    (&["the", "result"], 2),
    (&["that", "result"], 2),
    (&["result"], 1),
];

const DAMAGE_EVENT_AMOUNT_PREFIXES: &[(&[&str], usize)] = &[
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

fn colored_mana_symbols_in_sacrificed_cost(words: &[&str]) -> Option<(Value, usize)> {
    let mut idx = usize::from(words.first() == Some(&"the"));
    if !permission_shapes::starts_at_words(words, idx, &["number", "of"]) {
        return None;
    }
    idx += 2;

    let color = Color::from_name(words.get(idx).copied()?)?;
    idx += 1;
    if !permission_shapes::starts_at_words(words, idx, &["mana", "symbols", "in"]) {
        return None;
    }
    idx += 3;
    if words
        .get(idx)
        .is_some_and(|word| matches!(*word, "the" | "a" | "an"))
    {
        idx += 1;
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

pub(crate) fn parse_value_expr_words(words: &[&str]) -> Option<(Value, usize)> {
    let (mut value, mut used) = parse_value_expr_term_words(words)?;
    while used < words.len() {
        let operator = *words.get(used)?;
        if !permission_shapes::exact_words(&[operator], &["plus"])
            && !permission_shapes::exact_words(&[operator], &["minus"])
        {
            break;
        }
        let (rhs, rhs_used) = parse_value_expr_term_words(&words[used + 1..])?;
        used += 1 + rhs_used;
        let rhs = if permission_shapes::exact_words(&[operator], &["minus"]) {
            match rhs {
                Value::Fixed(fixed) => Value::Fixed(-fixed),
                _ => return None,
            }
        } else {
            rhs
        };
        value = Value::Add(Box::new(value), Box::new(rhs));
    }
    Some((value, used))
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
    if let Some(value) = colored_mana_symbols_in_sacrificed_cost(words) {
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
            &["times", "this", "creature", "has", "mutated"],
            &["times", "this", "permanent", "has", "mutated"],
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

    if let Some(used) = prefix_len(
        words,
        &[
            &["its", "power"],
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
    if let Some(used) = prefix_len(
        words,
        &[
            &["its", "toughness"],
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
    if let Some(used) = prefix_len(
        words,
        &[
            &["its", "mana", "value"],
            &["this", "mana", "value"],
            &["thiss", "mana", "value"],
            &["this", "creature", "mana", "value"],
            &["thiss", "creature", "mana", "value"],
            &["this", "creatures", "mana", "value"],
            &["thiss", "creatures", "mana", "value"],
        ],
    ) {
        return Some((Value::ManaValueOf(Box::new(ChooseSpec::Source)), used));
    }
    if let Some(used) = prefix_len(words, COLORS_SPENT_PREFIXES) {
        return Some((Value::ColorsOfManaSpentToCastThisSpell, used));
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
    if let Some(mut filter) = parse_source_controller_graveyard_filter(filter_words) {
        filter.zone = Some(crate::zone::Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        return Some((Value::Count(filter), filter_end));
    }
    let filter = parse_object_filter_words(filter_words, false).ok()?;
    Some((Value::Count(filter), filter_end))
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
    ["plus", "minus"]
        .iter()
        .filter_map(|word| permission_shapes::find_words(words, &[*word]))
        .min()
        .unwrap_or(words.len())
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
    use crate::runtime_backend::lexer::lex_line;

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
    fn preserves_token_boundary_for_value_prefixes() {
        let tokens = lex_line("x plus two cards", 0).expect("lex fixture");
        assert_eq!(
            parse_value_expr_tokens(&tokens),
            Some((Value::Add(Box::new(Value::X), Box::new(Value::Fixed(2))), 3,))
        );
    }

    #[test]
    fn parses_triggering_cast_mana_and_excess_damage_values() {
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
}

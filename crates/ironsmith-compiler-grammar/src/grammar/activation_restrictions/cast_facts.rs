use crate::filter::{Comparison, ParityRequirement};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

use super::super::leaf;
use super::clause_facts::{exact, exact_any, prefix, prefix_remainder, suffix};
use super::{ActivationCastLimitQualifier, parse_activation_cast_limit_qualifier_words};

#[derive(Debug, Clone, PartialEq)]
pub enum CantCastRestrictionFact {
    CastSpells(PlayerFilter),
    CastCreatureSpells(PlayerFilter),
    CastSpellsMatching {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
    CastMoreThanOneMatching {
        player: PlayerFilter,
        filter: ObjectFilter,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerActivationRestrictionTailFact {
    CastSpellsMatching(ObjectFilter),
    CastSpells,
    ActivateNonManaAbilities,
    ActivateAbilitiesOf {
        filter: ObjectFilter,
        non_mana_only: bool,
    },
}

pub fn parse_cant_cast_restriction_fact_words(words: &[&str]) -> Option<CantCastRestrictionFact> {
    if let Some(filter) = parse_spell_subject_cant_be_cast_filter_words(words) {
        return Some(CantCastRestrictionFact::CastSpellsMatching {
            player: PlayerFilter::Any,
            filter,
        });
    }

    if let Some(subject) = super::parse_cant_cast_subject_words(words) {
        let mut tail = &words[subject.consumed..];
        if let Some(rest) = strip_iterated_next_turn_suffix(tail) {
            tail = rest;
        }

        if let Some(filter) = parse_cast_additional_limit_filter_words(tail) {
            return Some(CantCastRestrictionFact::CastMoreThanOneMatching {
                player: subject.player,
                filter,
            });
        }

        let cant_tail = strip_cant_word(tail)?;
        if exact_any(
            cant_tail,
            &[&["cast", "spells"], &["cast", "spells", "this", "turn"]],
        ) {
            return Some(CantCastRestrictionFact::CastSpells(subject.player));
        }
        if prefix(cant_tail, &["cast", "spells", "with"])
            && suffix(cant_tail, &["mana", "values"])
            && cant_tail.len() >= 6
        {
            let parity = match cant_tail.get(3).copied()? {
                "odd" => ParityRequirement::Odd,
                "even" => ParityRequirement::Even,
                _ => return None,
            };
            return Some(CantCastRestrictionFact::CastSpellsMatching {
                player: subject.player,
                filter: ObjectFilter::spell().with_mana_value_parity(parity),
            });
        }
        if exact_any(
            cant_tail,
            &[
                &["cast", "creature", "spells"],
                &["cast", "creature", "spells", "this", "turn"],
            ],
        ) {
            return Some(CantCastRestrictionFact::CastCreatureSpells(subject.player));
        }
        if let Some(mut cast_tail) = prefix_remainder(cant_tail, &["cast"])
            && let Some(ActivationCastLimitQualifier { filter, consumed }) =
                parse_activation_cast_limit_qualifier_words(cast_tail)
        {
            cast_tail = cast_tail.get(consumed..)?;
            if matches!(cast_tail.first().copied(), Some("spell" | "spells")) {
                cast_tail = &cast_tail[1..];
                if let Some(rest) = prefix_remainder(cast_tail, &["this", "turn"]) {
                    cast_tail = rest;
                }
                if cast_tail.is_empty() {
                    return Some(CantCastRestrictionFact::CastSpellsMatching {
                        player: subject.player,
                        filter,
                    });
                }
            }
        }
        if let Some(filter) = parse_cast_more_than_one_limit_filter_words(cant_tail) {
            return Some(CantCastRestrictionFact::CastMoreThanOneMatching {
                player: subject.player,
                filter,
            });
        }
        return None;
    }

    parse_cast_additional_limit_filter_words(words).map(|filter| {
        CantCastRestrictionFact::CastMoreThanOneMatching {
            player: PlayerFilter::Any,
            filter,
        }
    })
}

pub fn parse_spell_subject_cant_be_cast_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let subject = super::clause_facts::suffix_remainder(words, &["cant", "be", "cast"])?;
    (!subject.is_empty()).then(|| parse_spell_restriction_subject_filter_words(subject))?
}

pub fn parse_spell_restriction_subject_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::spell();
    let mut input = words;

    if let Some(rest) = prefix_remainder(input, &["noncreature"]) {
        filter = filter.without_type(crate::types::CardType::Creature);
        input = rest;
    } else if let Some(rest) = prefix_remainder(input, &["non", "creature"]) {
        filter = filter.without_type(crate::types::CardType::Creature);
        input = rest;
    } else if !matches!(input.first().copied(), Some("spell" | "spells")) {
        let term = singular(input.first().copied()?);
        if let Ok(card_type) = leaf::parse_leaf_card_type_complete(term) {
            filter = filter.with_type(card_type);
            input = &input[1..];
        } else if let Ok(subtype) = leaf::parse_leaf_subtype_flexible_complete(term) {
            filter = filter.with_subtype(subtype);
            input = &input[1..];
        }
    }

    if !matches!(input.first().copied(), Some("spell" | "spells")) {
        return None;
    }
    input = &input[1..];

    while !input.is_empty() {
        input = prefix_remainder(input, &["with"])?;
        if let Some(rest) = prefix_remainder(input, &["mana", "value"]) {
            let value = leaf::parse_number_i32_complete(rest.first().copied()?).ok()?;
            let after_value = &rest[1..];
            if let Some(tail) = prefix_remainder(after_value, &["or", "greater"]) {
                filter = filter.with_mana_value(Comparison::GreaterThanOrEqual(value));
                input = tail;
            } else if let Some(tail) = prefix_remainder(after_value, &["or", "less"]) {
                filter = filter.with_mana_value(Comparison::LessThanOrEqual(value));
                input = tail;
            } else {
                filter = filter.with_mana_value(Comparison::Equal(value));
                input = after_value;
            }
            continue;
        }
        if let Some(rest) = parse_x_in_mana_cost_prefix(input) {
            filter.has_x_in_cost = true;
            input = rest;
            continue;
        }
        if let Some(rest) = prefix_remainder(input, &["the", "chosen", "name"])
            .or_else(|| prefix_remainder(input, &["chosen", "name"]))
        {
            filter.name = Some("{chosen name}".to_string());
            input = rest;
            continue;
        }
        return None;
    }
    Some(filter)
}

pub fn parse_cast_more_than_one_limit_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut input = prefix_remainder(words, &["cast", "more", "than", "one"])?;
    let filter = if matches!(input.first().copied(), Some("spell")) {
        ObjectFilter::default()
    } else {
        let parsed = parse_activation_cast_limit_qualifier_words(input)?;
        input = input.get(parsed.consumed..)?;
        parsed.filter
    };
    exact(input, &["spell", "each", "turn"]).then_some(filter)
}

pub fn parse_cast_additional_limit_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut input = prefix_remainder(words, &["who", "has"]).unwrap_or(words);
    input = prefix_remainder(input, &["cast"])?;
    if matches!(input.first().copied(), Some("a" | "an")) {
        input = &input[1..];
    }
    let first = parse_activation_cast_limit_qualifier_words(input)?;
    input = input.get(first.consumed..)?;
    input = prefix_remainder(input, &["spell"])?;
    if let Some(rest) = prefix_remainder(input, &["this", "turn"]) {
        input = rest;
    }
    input = prefix_remainder(input, &["cant", "cast", "additional"])?;
    let second = parse_activation_cast_limit_qualifier_words(input)?;
    if second.filter != first.filter {
        return None;
    }
    input = input.get(second.consumed..)?;
    exact(input, &["spells"]).then_some(first.filter)
}

pub fn parse_cast_restriction_tail_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    if let Some(rest) = prefix_remainder(words, &["cast"])
        && let Some(mut filter) = parse_spell_restriction_subject_filter_words(rest)
    {
        filter.zone = None;
        filter.stack_kind = None;
        return Some(filter);
    }
    if exact(words, &["cast", "spells"]) {
        return Some(ObjectFilter::default());
    }
    if exact_any(
        words,
        &[
            &["cast", "spells", "of", "the", "chosen", "type"],
            &[
                "cast", "spells", "of", "the", "chosen", "type", "this", "turn",
            ],
        ],
    ) {
        return Some(ObjectFilter::default().of_chosen_card_type());
    }
    let tail = prefix_remainder(words, &["cast"])?;
    let tail = super::clause_facts::suffix_remainder(tail, &["spells"])?;
    if tail.is_empty() {
        return None;
    }
    let parsed = parse_activation_cast_limit_qualifier_words(tail)?;
    (parsed.consumed == tail.len()).then_some(parsed.filter)
}

pub fn parse_card_type_list_filter_words(
    words: &[&str],
    zone: Option<Zone>,
) -> Option<ObjectFilter> {
    let mut filters = Vec::new();
    for word in words {
        if is_card_type_list_noise_word(word) {
            continue;
        }
        let card_type = leaf::parse_leaf_card_type_complete(singular(word)).ok()?;
        let mut filter = ObjectFilter::default();
        filter.zone = zone;
        filter.card_types.push(card_type);
        filters.push(filter);
    }
    if filters.len() == 1 {
        return filters.pop();
    }
    if filters.is_empty() {
        return None;
    }
    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = filters;
    Some(disjunction)
}

pub fn parse_player_activation_restriction_tail_words(
    words: &[&str],
) -> Option<PlayerActivationRestrictionTailFact> {
    if let Some(filter) = parse_cast_restriction_tail_filter_words(words) {
        return Some(PlayerActivationRestrictionTailFact::CastSpellsMatching(
            filter,
        ));
    }
    if exact_any(
        words,
        &[&["cast", "spells"], &["cast", "spells", "this", "turn"]],
    ) {
        return Some(PlayerActivationRestrictionTailFact::CastSpells);
    }
    if exact(
        words,
        &[
            "activate",
            "abilities",
            "that",
            "arent",
            "mana",
            "abilities",
        ],
    ) {
        return Some(PlayerActivationRestrictionTailFact::ActivateNonManaAbilities);
    }
    let owner_words = prefix_remainder(words, &["activate", "abilities", "of"])?;
    let non_mana_only = suffix(words, &["unless", "theyre", "mana", "abilities"]);
    let filter = parse_card_type_list_filter_words(owner_words, Some(Zone::Battlefield))?;
    Some(PlayerActivationRestrictionTailFact::ActivateAbilitiesOf {
        filter,
        non_mana_only,
    })
}

fn strip_cant_word<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    match words.first().copied()? {
        "cant" | "can't" | "cannot" => Some(&words[1..]),
        _ => None,
    }
}

fn strip_iterated_next_turn_suffix<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    super::clause_facts::suffix_remainder(words, &["during", "that", "players", "next", "turn"])
        .or_else(|| {
            super::clause_facts::suffix_remainder(
                words,
                &["during", "that", "player", "s", "next", "turn"],
            )
        })
}

fn parse_x_in_mana_cost_prefix<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    [
        &["x", "in", "their", "mana", "cost"][..],
        &["x", "in", "their", "mana", "costs"][..],
        &["x", "in", "its", "mana", "cost"][..],
        &["x", "in", "its", "mana", "costs"][..],
    ]
    .iter()
    .find_map(|phrase| prefix_remainder(words, phrase))
}

fn is_card_type_list_noise_word(word: &str) -> bool {
    matches!(
        word,
        "a" | "an" | "the" | "or" | "and" | "," | "unless" | "theyre" | "mana" | "abilities"
    )
}

fn singular(word: &str) -> &str {
    word.strip_suffix('s').unwrap_or(word)
}

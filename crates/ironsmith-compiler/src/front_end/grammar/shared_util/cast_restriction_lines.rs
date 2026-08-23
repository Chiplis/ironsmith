use crate::cards::builders::CardTextError;
use crate::grammar::{conditions, permission_shapes};
use crate::lexer::{OwnedLexToken, TokenWordView};
use crate::static_abilities::{StaticAbility, ThisSpellCastRestrictionKind as Restriction};
use crate::target::ObjectFilter;
use crate::types::Subtype;
use crate::zone::Zone;

pub fn parse_cast_this_spell_only(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = TokenWordView::new(tokens).word_refs();
    if !permission_shapes::prefix_words(&words, &["cast", "this", "spell", "only"]) {
        return Ok(None);
    }
    let tail = words.get(4..).unwrap_or_default();
    if let Some(restriction) = named_permanent_restriction(tail) {
        return Ok(Some(restriction));
    }
    if let Some((kind, text)) = control_quantity_restriction(tail) {
        return Ok(Some(StaticAbility::this_spell_cast_restriction(kind, text)));
    }
    Ok(fixed_restriction(tail)
        .map(|(kind, text)| StaticAbility::this_spell_cast_restriction(kind, text)))
}

fn named_permanent_restriction(words: &[&str]) -> Option<StaticAbility> {
    if words.len() <= 8
        || !permission_shapes::prefix_words(words, &["if", "no", "permanents", "named"])
        || !permission_shapes::suffix_words(words, &["are", "on", "the", "battlefield"])
    {
        return None;
    }
    let card_name = title_case(&words[4..words.len() - 4]);
    Some(StaticAbility::this_spell_cast_restriction(
        Restriction::if_no_permanents_named_on_battlefield(&card_name),
        format!("Cast this spell only if no permanents named {card_name} are on the battlefield."),
    ))
}

fn control_quantity_restriction(words: &[&str]) -> Option<(Restriction, String)> {
    let control_words = words.strip_prefix(&["if"])?;
    let parsed = conditions::parse_control_condition_words(
        control_words,
        conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;
    if !parsed.has_explicit_quantity() {
        return None;
    }
    let count = parsed.strict_at_least_count()?;
    let count_text = parsed.quantity_text();
    let filter_text = parsed.object_text();
    let subtype = single_subtype(&parsed.filter)?;
    Some((
        Restriction::if_you_control_subtype_or_more(subtype, count),
        format!(
            "Cast this spell only if you control {} {}.",
            count_text,
            title_case(&filter_text.split_whitespace().collect::<Vec<_>>())
        ),
    ))
}

fn fixed_restriction(words: &[&str]) -> Option<(Restriction, &'static str)> {
    if exact_one_of(
        words,
        &[
            &["during", "the", "declare", "attackers", "step"],
            &["during", "declare", "attackers", "step"],
        ],
    ) {
        return Some((
            Restriction::during_declare_attackers_step(),
            "Cast this spell only during the declare attackers step.",
        ));
    }
    if exact_one_of(
        words,
        &[
            &[
                "during",
                "the",
                "declare",
                "attackers",
                "step",
                "and",
                "only",
                "if",
                "youve",
                "been",
                "attacked",
                "this",
                "step",
            ],
            &[
                "during",
                "declare",
                "attackers",
                "step",
                "and",
                "only",
                "if",
                "youve",
                "been",
                "attacked",
                "this",
                "step",
            ],
        ],
    ) {
        return Some((
            Restriction::during_declare_attackers_step_if_you_were_attacked_this_step(),
            "Cast this spell only during the declare attackers step and only if you've been attacked this step.",
        ));
    }

    let entries: &[(&[&str], fn() -> Restriction, &str)] = &[
        (
            &["during", "combat"],
            Restriction::during_combat,
            "Cast this spell only during combat.",
        ),
        (
            &["during", "combat", "before", "blockers", "are", "declared"],
            Restriction::during_combat_before_blockers_are_declared,
            "Cast this spell only during combat before blockers are declared.",
        ),
        (
            &["during", "combat", "after", "blockers", "are", "declared"],
            Restriction::during_combat_after_blockers_are_declared,
            "Cast this spell only during combat after blockers are declared.",
        ),
        (
            &[
                "during", "combat", "on", "your", "turn", "before", "blockers", "are", "declared",
            ],
            Restriction::during_combat_on_your_turn_before_blockers_are_declared,
            "Cast this spell only during combat on your turn before blockers are declared.",
        ),
        (
            &["during", "combat", "on", "an", "opponents", "turn"],
            Restriction::during_combat_on_opponents_turn,
            "Cast this spell only during combat on an opponent's turn.",
        ),
        (
            &["before", "attackers", "are", "declared"],
            Restriction::before_attackers_are_declared,
            "Cast this spell only before attackers are declared.",
        ),
        (
            &[
                "during",
                "an",
                "opponents",
                "turn",
                "after",
                "their",
                "upkeep",
                "step",
            ],
            Restriction::during_opponents_turn_after_upkeep,
            "Cast this spell only during an opponent's turn after their upkeep step.",
        ),
        (
            &["during", "your", "end", "step"],
            Restriction::during_your_end_step,
            "Cast this spell only during your end step.",
        ),
        (
            &["if", "youve", "cast", "another", "spell", "this", "turn"],
            Restriction::if_you_cast_another_spell_this_turn,
            "Cast this spell only if you've cast another spell this turn.",
        ),
        (
            &[
                "if", "youve", "cast", "another", "green", "spell", "this", "turn",
            ],
            Restriction::if_you_cast_another_green_spell_this_turn,
            "Cast this spell only if you've cast another green spell this turn.",
        ),
        (
            &[
                "if", "an", "opponent", "cast", "a", "creature", "spell", "this", "turn",
            ],
            Restriction::if_opponent_cast_creature_spell_this_turn,
            "Cast this spell only if an opponent cast a creature spell this turn.",
        ),
        (
            &["if", "a", "creature", "is", "attacking", "you"],
            Restriction::if_creature_is_attacking_you,
            "Cast this spell only if a creature is attacking you.",
        ),
        (
            &["after", "combat"],
            Restriction::after_combat,
            "Cast this spell only after combat.",
        ),
        (
            &["if", "you", "control", "a", "snow", "land"],
            Restriction::if_you_control_snow_land,
            "Cast this spell only if you control a snow land.",
        ),
        (
            &[
                "if",
                "you",
                "control",
                "fewer",
                "creatures",
                "than",
                "each",
                "opponent",
            ],
            Restriction::if_you_control_fewer_creatures_than_each_opponent,
            "Cast this spell only if you control fewer creatures than each opponent.",
        ),
    ];
    for (expected, constructor, text) in entries {
        if permission_shapes::exact_words(words, expected) {
            return Some((constructor(), text));
        }
    }
    if exact_one_of(
        words,
        &[
            &["before", "the", "combat", "damage", "step"],
            &["before", "combat", "damage", "step"],
        ],
    ) {
        return Some((
            Restriction::before_combat_damage_step(),
            "Cast this spell only before the combat damage step.",
        ));
    }
    if exact_one_of(
        words,
        &[
            &["during", "an", "opponents", "upkeep"],
            &["during", "opponents", "upkeep"],
        ],
    ) {
        return Some((
            Restriction::during_opponents_upkeep(),
            "Cast this spell only during an opponent's upkeep.",
        ));
    }
    None
}

fn single_subtype(filter: &ObjectFilter) -> Option<Subtype> {
    if filter.subtypes.len() == 1
        && filter.excluded_subtypes.is_empty()
        && filter.supertypes.is_empty()
        && filter.excluded_supertypes.is_empty()
        && filter.colors.is_none()
        && !filter.colorless
        && !filter.multicolored
        && !filter.monocolored
        && !filter.token
        && !filter.nontoken
        && !filter.tapped
        && !filter.untapped
        && !filter.other
    {
        filter.subtypes.first().copied()
    } else {
        None
    }
}

fn exact_one_of(words: &[&str], alternatives: &[&[&str]]) -> bool {
    alternatives
        .iter()
        .any(|expected| permission_shapes::exact_words(words, expected))
}

fn title_case(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            format!("{}{}", first.to_ascii_uppercase(), chars.as_str())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    #[test]
    fn parses_fixed_cast_restriction() {
        let tokens = lex_line("Cast this spell only during combat.", 0).unwrap();
        assert!(parse_cast_this_spell_only(&tokens).unwrap().is_some());
    }
}

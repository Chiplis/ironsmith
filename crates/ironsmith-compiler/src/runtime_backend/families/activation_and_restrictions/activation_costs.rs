use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};

const IF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if"]);
struct StaticAbilityShapeEntry {
    pattern: ClauseShape<'static>,
    build: fn() -> StaticAbility,
}

enum StaticAbilityShapeResolution {
    Ability(StaticAbility),
    Decline,
}

const MAX_SPEED_CANT_ATTACK_OR_BLOCK_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this", "cant", "attack", "or", "block", "unless", "you", "have", "max", "speed",
            ],
            &[
                "this", "creature", "cant", "attack", "or", "block", "unless", "you", "have",
                "max", "speed",
            ],
        ]
);
const DIRECT_TEMPORARY_CAST_RESTRICTION_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["your", "opponents", "cant", "cast"],
            &["each", "opponent", "cant", "cast"],
            &["each", "player", "cant", "cast"],
            &["players", "cant", "cast"],
            &["target", "player", "cant", "cast"],
            &["you", "cant", "cast"],
        ];
    contains_words & ["this", "turn"]
);
const UNLESS_WORD: &str = "unless";
const WHO_WORD: &str = "who";
const AND_WORD: &str = "and";
const OR_WORD: &str = "or";
const GET_OR_GETS_WORDS: &[&str] = &["get", "gets"];
const UNTAP_WORD: &str = "untap";
const CREATURE_OR_CREATURES_WORDS: &[&str] = &["creature", "creatures"];
const WALL_OR_WALLS_WORDS: &[&str] = &["wall", "walls"];
const ARTIFACT_WORD: &str = "artifact";
const SELF_SUBJECT_PHRASES: &[&[&str]] = &[&["this", "creature"], &["this"]];
const BLOCK_WORD: &str = "block";
const TRANSFORM_WORD: &str = "transform";
const MANA_VALUES_WORDS: &[&str] = &["mana", "values"];
const OPPONENTS_CANT_CAST_SPELLS_WITH_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["your", "opponents", "cant", "cast", "spells", "with"]);
const OPPONENTS_CANT_BLOCK_WITH_CREATURES_WITH_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "your",
            "opponents",
            "cant",
            "block",
            "with",
            "creatures",
            "with",
        ]
);
const EVEN_COUNTERS_ON_IT_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["even", "number", "of", "counters", "on", "it"]);
const THIS_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "this", "creature", "cant", "attack", "or", "block", "unless",
            ],
            &["this", "cant", "attack", "or", "block", "unless"],
        ]
);
const THIS_CREATURE_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "this", "creature", "cant", "attack", "or", "block", "unless"
        ]
);
const THIS_SELF_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "cant", "attack", "or", "block", "unless"]);
const IF_SOURCE_YOU_CONTROL_DOUBLE_MANA_VALUE_INSTEAD_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["if", "source", "you", "control", "with"]; suffix & ["instead"]; contains_words & ["mana", "value", "double"]);
const IF_PLAYER_WOULD_GAIN_NO_LIFE_INSTEAD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "if", "a", "player", "would", "gain", "life", "that", "player", "gains", "no",
                "life", "instead",
            ],
            &[
                "if", "a", "player", "would", "gain", "life", "they", "gain", "no", "life",
                "instead",
            ],
        ]
);
const CANONICAL_NEGATED_RESTRICTION_STATIC_ABILITY_PATTERNS: &[StaticAbilityShapeEntry] = &[
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["players", "cant", "gain", "life"]),
        build: StaticAbility::players_cant_gain_life,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["players", "cant", "search", "libraries"]),
        build: StaticAbility::players_cant_search,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["damage", "cant", "be", "prevented"]),
        build: StaticAbility::damage_cant_be_prevented,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["you", "cant", "lose", "the", "game"]),
        build: StaticAbility::you_cant_lose_game,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["your", "opponents", "cant", "win", "the", "game"]),
        build: StaticAbility::opponents_cant_win_game,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["your", "life", "total", "cant", "change"]),
        build: StaticAbility::your_life_total_cant_change,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["your", "opponents", "cant", "cast", "spells"]),
        build: StaticAbility::opponents_cant_cast_spells,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(
            exact
                & [
                    "your",
                    "opponents",
                    "cant",
                    "draw",
                    "more",
                    "than",
                    "one",
                    "card",
                    "each",
                    "turn",
                ]
        ),
        build: StaticAbility::opponents_cant_draw_extra_cards,
    },
];
const DIRECT_CANT_STATIC_ABILITY_PATTERNS: &[StaticAbilityShapeEntry] = &[
    StaticAbilityShapeEntry {
        pattern: clause_shape!(
            exact & ["counters", "cant", "be", "put", "on", "this", "permanent"]
        ),
        build: StaticAbility::cant_have_counters_placed,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["this", "spell", "cant", "be", "countered"]),
        build: StaticAbility::cant_be_countered_ability,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(
            exact_any
                & [
                    &["this", "creature", "cant", "attack"],
                    &["this", "token", "cant", "attack"],
                    &["this", "cant", "attack"]
                ]
        ),
        build: StaticAbility::cant_attack,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(
            exact_any
                & [
                    &["this", "creature", "cant", "block"],
                    &["this", "token", "cant", "block"],
                    &["this", "cant", "block"]
                ]
        ),
        build: StaticAbility::cant_block,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(exact & ["this", "creature", "cant", "attack", "its", "owner"]),
        build: StaticAbility::cant_attack_its_owner,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(
            exact & ["permanents", "you", "control", "cant", "be", "sacrificed"]
        ),
        build: StaticAbility::permanents_you_control_cant_be_sacrificed,
    },
    StaticAbilityShapeEntry {
        pattern: clause_shape!(
            exact_any
                & [
                    &["this", "creature", "cant", "be", "blocked"],
                    &["this", "token", "cant", "be", "blocked"],
                    &["this", "cant", "be", "blocked"],
                    &["cant", "be", "blocked"]
                ]
        ),
        build: StaticAbility::unblockable,
    },
];
const TEMPORARY_UNBLOCKABLE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "cant", "be", "blocked", "this", "turn"],
            &["this", "cant", "be", "blocked", "this", "turn"],
            &["cant", "be", "blocked", "this", "turn"],
        ]
);
const SOURCE_CANT_ATTACK_ALONE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "cant", "attack", "alone"],
            &["this", "token", "cant", "attack", "alone"],
            &["this", "cant", "attack", "alone"],
        ]
);
const SOURCE_CANT_ATTACK_OR_BLOCK_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "cant", "attack", "or", "block"],
            &["this", "token", "cant", "attack", "or", "block"],
            &["this", "cant", "attack", "or", "block"],
        ]
);
const SOURCE_CANT_ATTACK_OR_BLOCK_ALONE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "cant", "attack", "or", "block", "alone"],
            &["this", "token", "cant", "attack", "or", "block", "alone"],
            &["this", "cant", "attack", "or", "block", "alone"],
        ]
);
const LOSE_UNSPENT_MANA_STEPS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["lose", "unspent"]; contains_phrases & [&["mana", "as", "steps"]]);
const LOSE_THIS_MANA_STEPS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["lose", "this", "mana", "as", "steps"]);
const ATTACK_OR_BLOCK_TAIL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["attack", "or", "block"]);
const CANT_RESTRICTION_OR_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["cast"], &["activate"], &["attack"], &["block"], &["be"]]);
const THIS_CANT_ATTACK_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "cant", "attack"],
            &["this", "cant", "attack"]
        ]
);
const THIS_CANT_ATTACK_UNLESS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "cant", "attack", "unless"],
            &["this", "cant", "attack", "unless"],
        ]
);
const THIS_CREATURE_CANT_ATTACK_UNLESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "creature", "cant", "attack", "unless"]);
const THIS_SELF_CANT_ATTACK_UNLESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "cant", "attack", "unless"]);
const CAST_CREATURE_SPELL_THIS_TURN_UNLESS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &[
                "unless", "youve", "cast", "a", "creature", "spell", "this", "turn"
            ],
            &[
                "unless", "you", "ve", "cast", "a", "creature", "spell", "this", "turn"
            ],
            &[
                "unless", "youve", "cast", "creature", "spell", "this", "turn"
            ],
            &[
                "unless", "you", "ve", "cast", "creature", "spell", "this", "turn"
            ],
        ]
);
const CAST_NONCREATURE_SPELL_THIS_TURN_UNLESS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &[
                "unless",
                "youve",
                "cast",
                "a",
                "noncreature",
                "spell",
                "this",
                "turn"
            ],
            &[
                "unless",
                "you",
                "ve",
                "cast",
                "a",
                "noncreature",
                "spell",
                "this",
                "turn"
            ],
            &[
                "unless",
                "youve",
                "cast",
                "noncreature",
                "spell",
                "this",
                "turn"
            ],
            &[
                "unless",
                "you",
                "ve",
                "cast",
                "noncreature",
                "spell",
                "this",
                "turn"
            ],
        ]
);
const COLLECTIVE_RESTRAINT_ATTACK_TAX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "creatures",
            "cant",
            "attack",
            "you",
            "unless",
            "their",
            "controller",
            "pays",
            "x",
            "for",
            "each",
            "creature",
            "they",
            "control",
            "thats",
            "attacking",
            "you",
        ];
    suffix_any
        & [
            &[
                "where", "x", "is", "the", "number", "of", "basic", "land", "types", "among",
                "lands", "you", "control",
            ],
            &[
                "where", "x", "is", "the", "number", "of", "basic", "land", "type", "among",
                "lands", "you", "control",
            ],
        ]
);
const CANT_BE_BLOCKED_BY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "cant", "be", "blocked", "by"],
            &["this", "cant", "be", "blocked", "by"],
            &["cant", "be", "blocked", "by"],
        ]
);
const THIS_CREATURE_CANT_BE_BLOCKED_BY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "creature", "cant", "be", "blocked", "by"]);
const THIS_CANT_BE_BLOCKED_BY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "cant", "be", "blocked", "by"]);
const CANT_BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "cant", "be", "blocked", "except", "by"],
            &["this", "cant", "be", "blocked", "except", "by"],
            &["cant", "be", "blocked", "except", "by"],
        ]
);
const THIS_CREATURE_CANT_BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "creature", "cant", "be", "blocked", "except", "by"]);
const THIS_CANT_BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "cant", "be", "blocked", "except", "by"]);
const WITH_POWER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["with", "power"]);
const WITH_FLYING_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with", "flying"]);
const YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "control"]);
const CONTROL_MORE_CREATURES_THAN_DEFENDING_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you",
            "control",
            "more",
            "creatures",
            "than",
            "defending",
            "player",
        ]
);
const CONTROL_MORE_LANDS_THAN_DEFENDING_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "you",
            "control",
            "more",
            "lands",
            "than",
            "defending",
            "player",
        ]
);
const CONTROL_ANOTHER_CREATURE_WITH_POWER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "control", "another", "creature", "with", "power"]);
const CONTROL_A_CREATURE_WITH_POWER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "control", "a", "creature", "with", "power"]);
const CARDS_IN_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cards", "in", "your", "graveyard"]);
const ISLANDS_ON_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["islands", "on", "the", "battlefield"],
            &["islands", "on", "battlefield"],
        ]
);
const ISLANDS_WORD: &str = "islands";
const CARDS_IN_THEIR_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cards", "in", "their", "graveyard"]);
const CARDS_IN_EXILE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cards", "in", "exile"]);
const MOUNTAIN_ON_BATTLEFIELD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["there", "is", "a", "mountain", "on", "the", "battlefield"],
            &["there", "is", "a", "mountain", "on", "battlefield"],
            &["there", "is", "mountain", "on", "battlefield"],
        ]
);
const DEFENDING_PLAYER_POISONED_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["defending", "player", "is", "poisoned"]);
const DEFENDING_PLAYER_CONTROLS_ENCHANTMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "defending",
                "player",
                "controls",
                "an",
                "enchantment",
                "or",
                "an",
                "enchanted",
                "permanent",
            ],
            &[
                "defending",
                "player",
                "controls",
                "enchantment",
                "or",
                "enchanted",
                "permanent",
            ],
        ]
);
const OTHER_CREATURES_ATTACK_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["other", "creatures", "attack"]);
const CREATURE_WITH_GREATER_POWER_ALSO_ATTACKS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "a", "creature", "with", "greater", "power", "also", "attacks"
        ]
);
const BLACK_OR_GREEN_CREATURE_ALSO_ATTACKS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["a", "black", "or", "green", "creature", "also", "attacks"]);
const OPPONENT_DEALT_DAMAGE_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "an", "opponent", "has", "been", "dealt", "damage", "this", "turn",
        ]
);
const SACRIFICE_LAND_ATTACK_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "sacrifice", "a", "land"],
            &["you", "sacrifice", "land"],
        ]
);
const RETURN_ENCHANTMENT_ATTACK_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "you",
                "return",
                "an",
                "enchantment",
                "you",
                "control",
                "to",
                "its",
                "owners",
                "hand",
            ],
            &[
                "you",
                "return",
                "enchantment",
                "you",
                "control",
                "to",
                "its",
                "owners",
                "hand",
            ],
            &[
                "you",
                "return",
                "an",
                "enchantment",
                "you",
                "control",
                "to",
                "its",
                "owner",
                "s",
                "hand",
            ],
        ]
);
const PAY_PER_PLUS_ONE_COUNTER_ATTACK_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "you", "pay", "1", "for", "each", "+1/+1", "counter", "on", "it"
            ],
            &[
                "you", "pay", "1", "for", "each", "1/1", "counter", "on", "it"
            ],
        ]
);
fn activation_cost_shape_matches_words<'a>(words: &[&str], shape: ClauseShape<'a>) -> bool {
    shape.matches_word_slice(words)
}

fn activation_word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn activation_token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|_| activation_word_is_any(token.parser_text(), expected))
}

fn activation_token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    activation_token_word_is_any(token, &[expected])
}

fn activation_word_at_is_any(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words
        .get(idx)
        .is_some_and(|word| activation_word_is_any(word, expected))
}

fn activation_word_at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    activation_word_at_is_any(words, idx, &[expected])
}

fn activation_words_eq(words: &[&str], expected: &[&str]) -> bool {
    words == expected
}

fn activation_words_eq_any(words: &[&str], phrases: &[&[&str]]) -> bool {
    phrases
        .iter()
        .any(|phrase| activation_words_eq(words, phrase))
}

fn activation_words_contains(words: &[&str], expected: &str) -> bool {
    words.contains(&expected)
}

fn cant_attack_unless_tail<'a>(words: &'a [&str]) -> Option<&'a [&'a str]> {
    if activation_cost_shape_matches_words(words, THIS_CREATURE_CANT_ATTACK_UNLESS_PREFIX_PATTERN) {
        Some(&words[5..])
    } else if activation_cost_shape_matches_words(
        words,
        THIS_SELF_CANT_ATTACK_UNLESS_PREFIX_PATTERN,
    ) {
        Some(&words[4..])
    } else {
        None
    }
}

fn cant_attack_or_block_unless_tail<'a>(words: &'a [&str]) -> Option<&'a [&'a str]> {
    if activation_cost_shape_matches_words(
        words,
        THIS_CREATURE_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN,
    ) {
        Some(&words[7..])
    } else if activation_cost_shape_matches_words(
        words,
        THIS_SELF_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN,
    ) {
        Some(&words[6..])
    } else {
        None
    }
}

fn parse_activation_count_words(words: &[&str]) -> Option<(u32, usize)> {
    let word = words.first()?.to_ascii_lowercase();
    if let Ok(value) = word.parse::<u32>() {
        return Some((value, 1));
    }
    match word.as_str() {
        "once" => return Some((1, 1)),
        "twice" => return Some((2, 1)),
        _ => {}
    }
    let trimmed_trailing_punctuation = word.trim_end_matches(|ch: char| !ch.is_ascii_digit());
    if trimmed_trailing_punctuation.len() < word.len()
        && !trimmed_trailing_punctuation.is_empty()
        && trimmed_trailing_punctuation
            .chars()
            .all(|ch| ch.is_ascii_digit())
        && let Ok(value) = trimmed_trailing_punctuation.parse::<u32>()
    {
        return Some((value, 1));
    }
    ironsmith_core::parse_cardinal_words(words)
}

fn parse_greater_than_or_equal_count_prefix_from_words(words: &[&str]) -> Option<(u32, usize)> {
    if words.starts_with(&["at", "least"]) {
        let (count, used) = parse_activation_count_words(words.get(2..).unwrap_or_default())?;
        return Some((count, used + 2));
    }
    if words
        .first()
        .is_some_and(|word| activation_word_is_any(word, &["more", "greater"]))
        && words.get(1).copied() == Some("than")
    {
        let (count, used) = parse_activation_count_words(words.get(2..).unwrap_or_default())?;
        return Some((count.saturating_add(1), used + 2));
    }
    let (count, used) = parse_activation_count_words(words)?;
    if words
        .first()
        .is_some_and(|word| activation_word_is_any(word, &["a", "an"]))
    {
        return Some((1, used));
    }
    if words.get(used).copied() == Some("or")
        && words
            .get(used + 1)
            .is_some_and(|word| activation_word_is_any(word, &["more", "greater"]))
    {
        return Some((count, used + 2));
    }
    None
}

fn parse_exact_count_from_words(words: &[&str]) -> Option<(u32, usize)> {
    parse_activation_count_words(words)
}

fn player_controls_at_least_condition_from_tail(tail: &[&str]) -> Option<crate::ConditionExpr> {
    let control_condition =
        crate::runtime_backend::grammar::conditions::parse_control_condition_words(
            tail,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: false,
                allow_defending_player: false,
                bind_filter_controller_to_subject: true,
                allow_different_powers_tail: false,
                default_filter_zone: Some(Zone::Battlefield),
            },
        )?;
    if control_condition.quantity_token_count == 0 {
        return None;
    }
    let count = control_condition.at_least_count()?;
    Some(crate::ConditionExpr::PlayerHasAtLeast {
        player: control_condition.player_filter?,
        filter: control_condition.filter,
        count,
    })
}

fn source_control_condition_from_tail(tail: &[&str]) -> Option<crate::ConditionExpr> {
    let control_condition =
        crate::runtime_backend::grammar::conditions::parse_control_condition_words(
            tail,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: false,
                allow_defending_player: false,
                bind_filter_controller_to_subject: true,
                allow_different_powers_tail: false,
                default_filter_zone: Some(Zone::Battlefield),
            },
        )?;
    let count = control_condition.at_least_count()?;
    if count > 1 {
        return Some(crate::ConditionExpr::PlayerHasAtLeast {
            player: control_condition.player_filter?,
            filter: control_condition.filter,
            count,
        });
    }
    Some(crate::ConditionExpr::YouControl(control_condition.filter))
}

fn defending_player_controls_filter_from_tail(tail: &[&str]) -> Option<ObjectFilter> {
    let control_condition =
        crate::runtime_backend::grammar::conditions::parse_control_condition_words(
            tail,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: false,
                allow_defending_player: true,
                bind_filter_controller_to_subject: false,
                allow_different_powers_tail: false,
                default_filter_zone: Some(Zone::Battlefield),
            },
        )?;
    if control_condition.player_filter != Some(crate::target::PlayerFilter::Defending)
        || control_condition.at_least_count()? > 1
    {
        return None;
    }
    Some(control_condition.filter)
}

fn control_creature_with_power_condition_from_tail(tail: &[&str]) -> Option<crate::ConditionExpr> {
    let (other, prefix_len) = if let Some(prefix_len) =
        CONTROL_ANOTHER_CREATURE_WITH_POWER_PREFIX_PATTERN.matched_prefix_len(tail)
    {
        (true, prefix_len)
    } else if let Some(prefix_len) =
        CONTROL_A_CREATURE_WITH_POWER_PREFIX_PATTERN.matched_prefix_len(tail)
    {
        (false, prefix_len)
    } else {
        return None;
    };
    let comparison_words = tail.get(prefix_len..)?;
    let (comparison, used) =
        parse_filter_comparison_tokens("power", comparison_words, tail).ok()??;
    if used != comparison_words.len() {
        return None;
    }
    let mut filter = ObjectFilter::creature()
        .you_control()
        .with_power(comparison);
    if other {
        filter = filter.other();
    }
    Some(crate::ConditionExpr::YouControl(filter))
}

fn ability_from_shape_table(
    words: &[&str],
    table: &[StaticAbilityShapeEntry],
) -> Option<StaticAbility> {
    table
        .iter()
        .find(|entry| activation_cost_shape_matches_words(words, entry.pattern))
        .map(|entry| (entry.build)())
}

fn canonical_negated_restriction_static_ability(words: &[&str]) -> Option<StaticAbility> {
    ability_from_shape_table(words, CANONICAL_NEGATED_RESTRICTION_STATIC_ABILITY_PATTERNS)
}

fn direct_cant_static_ability(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> Option<StaticAbilityShapeResolution> {
    if activation_cost_shape_matches_words(words, TEMPORARY_UNBLOCKABLE_PATTERN) {
        return Some(StaticAbilityShapeResolution::Decline);
    }
    if let Some(ability) = ability_from_shape_table(words, DIRECT_CANT_STATIC_ABILITY_PATTERNS) {
        return Some(StaticAbilityShapeResolution::Ability(ability));
    }
    if activation_cost_shape_matches_words(words, SOURCE_CANT_ATTACK_ALONE_PATTERN) {
        return Some(StaticAbilityShapeResolution::Ability(
            StaticAbility::restriction(
                crate::effect::Restriction::attack_alone(ObjectFilter::source()),
                format_negated_restriction_display(tokens),
            ),
        ));
    }
    if activation_cost_shape_matches_words(words, SOURCE_CANT_ATTACK_OR_BLOCK_PATTERN) {
        return Some(StaticAbilityShapeResolution::Ability(
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                format_negated_restriction_display(tokens),
            ),
        ));
    }
    if activation_cost_shape_matches_words(words, SOURCE_CANT_ATTACK_OR_BLOCK_ALONE_PATTERN) {
        return Some(StaticAbilityShapeResolution::Ability(
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
                format_negated_restriction_display(tokens),
            ),
        ));
    }
    None
}

pub(crate) fn parse_cant_clauses(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    // Multi-sentence lines ("Damage can't be prevented this turn. Stomp deals
    // 2 damage to any target.") are effect sequences, not single static
    // restrictions; the duration stripper would otherwise swallow the period
    // and merge the sentences. Decline so the statement path splits them.
    if tokens.iter().enumerate().any(|(idx, token)| {
        matches!(token.kind, crate::runtime_backend::lexer::TokenKind::Period)
            && tokens[idx + 1..]
                .iter()
                .any(|later| later.as_word().is_some())
    }) {
        return Ok(None);
    }

    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder.as_slice() != tokens
    {
        let Some(abilities) = parse_cant_clauses(&remainder)? else {
            return Ok(None);
        };
        let conditioned = abilities
            .into_iter()
            .map(|ability| {
                ability
                    .clone()
                    .with_condition(condition.clone())
                    .unwrap_or(ability)
            })
            .collect::<Vec<_>>();
        return Ok(Some(conditioned));
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if activation_cost_shape_matches_words(
        &normalized_words,
        IF_PLAYER_WOULD_GAIN_NO_LIFE_INSTEAD_PATTERN,
    ) {
        return Ok(Some(vec![StaticAbility::restriction(
            crate::effect::Restriction::gain_life(PlayerFilter::Any),
            "If a player would gain life, that player gains no life instead".to_string(),
        )]));
    }
    if activation_cost_shape_matches_words(&normalized_words, IF_PREFIX_PATTERN) {
        return Ok(None);
    }
    if is_one_shot_mana_retention_cant_clause(&normalized_words) {
        return Ok(None);
    }
    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && remainder.len() < tokens.len()
    {
        let remainder_words_storage = normalize_cant_words(&remainder);
        let remainder_words = remainder_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if is_mana_retention_cant_clause(&remainder_words) {
            return Ok(None);
        }
    }
    // "Players/You don't lose unspent [color] mana as steps and phases end."
    // Parsed before the and-splitting below tears apart "steps and phases end".
    if let Some(ability) = parse_unspent_mana_retention_static(tokens, &normalized_words) {
        return Ok(Some(vec![ability]));
    }
    if activation_cost_shape_matches_words(
        &normalized_words,
        MAX_SPEED_CANT_ATTACK_OR_BLOCK_PATTERN,
    ) {
        let max_speed = crate::ConditionExpr::ValueComparison {
            left: crate::effect::Value::Speed(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(4),
        };
        return Ok(Some(vec![
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                "This creature can't attack or block".to_string(),
            )
            .with_condition(crate::ConditionExpr::Not(Box::new(max_speed))),
        ]));
    }
    let is_direct_temporary_cast_restriction =
        activation_cost_shape_matches_words(
            &normalized_words,
            DIRECT_TEMPORARY_CAST_RESTRICTION_PATTERN,
        ) && !activation_words_contains(&normalized_words, UNLESS_WORD)
            && !activation_words_contains(&normalized_words, WHO_WORD);
    if is_direct_temporary_cast_restriction {
        return Ok(None);
    }

    if activation_words_contains(&crate::runtime_backend::token_word_refs(tokens), AND_WORD)
        && let Some((neg_start, _)) = find_negation_span(tokens)
        && tokens[..neg_start]
            .iter()
            .any(|token| activation_token_word_is_any(token, GET_OR_GETS_WORDS))
    {
        return Ok(None);
    }

    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if let Some(segments) = split_cant_clause_on_or(tokens) {
        let mut abilities = Vec::new();
        for segment in segments {
            let Some(ability) = parse_cant_clause(&segment)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::runtime_backend::token_word_refs(&segment).join(" ")
                )));
            };
            abilities.push(ability);
        }
        if !abilities.is_empty() {
            return Ok(Some(abilities));
        }
    }

    if activation_words_contains(&crate::runtime_backend::token_word_refs(tokens), AND_WORD) {
        let segments = grammar::split_lexed_slices_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let negated_anchor = segments
            .iter()
            .position(|segment| find_negation_span(segment).is_some());
        let shared_negated_tail = negated_anchor.and_then(|anchor_idx| {
            find_negation_span(&segments[anchor_idx])
                .map(|(neg_start, _)| segments[anchor_idx][neg_start..].to_vec())
        });
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut abilities = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            let mut expanded = segment.to_vec();
            if find_negation_span(segment).is_none() {
                if let Some(anchor_idx) = negated_anchor
                    && idx < anchor_idx
                    && let Some(tail) = &shared_negated_tail
                {
                    expanded.extend(tail.iter().cloned());
                } else {
                    continue;
                }
            } else if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().cloned());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(ability) = parse_cant_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::runtime_backend::token_word_refs(segment).join(" ")
                )));
            };
            abilities.push(ability);
        }

        if abilities.is_empty() {
            return Ok(None);
        }
        return Ok(Some(abilities));
    }

    parse_cant_clause(tokens).map(|ability| ability.map(|ability| vec![ability]))
}

pub(crate) fn split_cant_clause_on_or(tokens: &[OwnedLexToken]) -> Option<Vec<Vec<OwnedLexToken>>> {
    let (neg_start, neg_end) = find_negation_span(tokens)?;
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if activation_cost_shape_matches_words(&remainder_words, ATTACK_OR_BLOCK_TAIL_PREFIX_PATTERN) {
        return None;
    }
    let or_idx = find_index(&remainder_tokens, |token: &OwnedLexToken| {
        activation_token_word_is(token, OR_WORD)
    })?;
    let tail = trim_commas(&remainder_tokens[or_idx + 1..]);
    let tail_words = crate::runtime_backend::token_word_refs(&tail);
    let starts_new_restriction =
        activation_cost_shape_matches_words(&tail_words, CANT_RESTRICTION_OR_TAIL_PATTERN);
    if !starts_new_restriction {
        return None;
    }

    let negation_tokens = tokens[neg_start..neg_end].to_vec();
    let mut first = subject_tokens.clone();
    first.extend(negation_tokens.iter().cloned());
    first.extend(trim_commas(&remainder_tokens[..or_idx]).iter().cloned());

    let mut second = subject_tokens.clone();
    second.extend(negation_tokens.iter().cloned());
    second.extend(tail.iter().cloned());

    Some(vec![first, second])
}

fn is_mana_retention_cant_clause(words: &[&str]) -> bool {
    let Some((&"you", rest)) = words.split_first() else {
        return false;
    };
    let rest = match rest {
        ["dont", tail @ ..] | ["don't", tail @ ..] | ["do", "not", tail @ ..] => tail,
        _ => return false,
    };
    activation_cost_shape_matches_words(rest, LOSE_UNSPENT_MANA_STEPS_PATTERN)
        || activation_cost_shape_matches_words(rest, LOSE_THIS_MANA_STEPS_PATTERN)
}

/// "You don't lose this mana as steps and phases end" — the duration-scoped
/// one-shot variant handled by the effect path, not a static restriction.
fn is_one_shot_mana_retention_cant_clause(words: &[&str]) -> bool {
    let Some((&"you", rest)) = words.split_first() else {
        return false;
    };
    let rest = match rest {
        ["dont", tail @ ..] | ["don't", tail @ ..] | ["do", "not", tail @ ..] => tail,
        _ => return false,
    };
    activation_cost_shape_matches_words(rest, LOSE_THIS_MANA_STEPS_PATTERN)
}

/// "Players/You don't lose unspent [color] mana as steps and phases end."
/// (Upwelling, Omnath Locus of Mana, Leyline Tyrant.)
fn parse_unspent_mana_retention_static(
    tokens: &[OwnedLexToken],
    words: &[&str],
) -> Option<StaticAbility> {
    let (subject, rest): (PlayerFilter, &[&str]) = match words {
        ["you", rest @ ..] => (PlayerFilter::You, rest),
        ["players", rest @ ..] | ["each", "player", rest @ ..] => (PlayerFilter::Any, rest),
        _ => return None,
    };
    let rest = match rest {
        ["dont", tail @ ..] | ["don't", tail @ ..] | ["do", "not", tail @ ..] => tail,
        _ => return None,
    };
    let color = parse_unspent_mana_retention_tail(rest)?;
    Some(StaticAbility::restriction(
        crate::effect::Restriction::lose_unspent_mana(subject, color),
        format_negated_restriction_display(tokens),
    ))
}

pub(crate) fn parse_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder.as_slice() != tokens
    {
        let Some(ability) = parse_cant_clause(&remainder)? else {
            return Ok(None);
        };
        #[cfg(not(feature = "serialization"))]
        {
            let conditioned = ability.clone().with_condition(condition.clone());
            return Ok(Some(conditioned));
        }
        #[cfg(feature = "serialization")]
        {
            let conditioned = ability.clone().with_condition(condition.clone());
            return Ok(conditioned);
        }
    }
    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && !remainder.is_empty()
        && remainder.len() < tokens.len()
    {
        let remainder_words_storage = normalize_cant_words(&remainder);
        let remainder_words = remainder_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if activation_words_contains(&remainder_words, UNTAP_WORD) {
            return Ok(None);
        }
    }
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if is_one_shot_mana_retention_cant_clause(&normalized) {
        return Ok(None);
    }
    if let Some(ability) = parse_unspent_mana_retention_static(tokens, &normalized) {
        return Ok(Some(ability));
    }

    if let Some(rest) = slice_strip_prefix(
        &normalized,
        &[
            "creatures",
            "cant",
            "attack",
            "you",
            "unless",
            "their",
            "controller",
            "pays",
        ],
    ) && rest.get(1..)
        == Some(&[
            "for",
            "each",
            "creature",
            "they",
            "control",
            "thats",
            "attacking",
            "you",
        ])
    {
        if let Some(amount) = parse_named_number(rest[0]) {
            return Ok(Some(
                StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(amount),
            ));
        }
    }

    if activation_cost_shape_matches_words(&normalized, COLLECTIVE_RESTRAINT_ATTACK_TAX_PATTERN) {
        return Ok(Some(
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control(),
        ));
    }

    if activation_cost_shape_matches_words(&normalized, CANT_BE_BLOCKED_BY_PREFIX_PATTERN) {
        let mut idx = if activation_cost_shape_matches_words(
            &normalized,
            THIS_CREATURE_CANT_BE_BLOCKED_BY_PREFIX_PATTERN,
        ) {
            6
        } else if activation_cost_shape_matches_words(
            &normalized,
            THIS_CANT_BE_BLOCKED_BY_PREFIX_PATTERN,
        ) {
            5
        } else {
            4
        };
        if activation_word_at_is_any(&normalized, idx, CREATURE_OR_CREATURES_WORDS) {
            idx += 1;
        }
        if let Some((minimum_blockers, used)) = parse_greater_than_or_equal_quantity_prefix_words(
            &normalized[idx..],
            false,
            false,
            "cant-be-blocked blocker threshold",
        )
        .ok()
        .flatten()
            && minimum_blockers > 0
        {
            let noun = normalized.get(idx + used).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if !activation_word_is_any(noun, CREATURE_OR_CREATURES_WORDS) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            if idx + used + 1 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked max-blockers clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            return Ok(Some(StaticAbility::cant_be_blocked_by_more_than(
                (minimum_blockers - 1) as usize,
            )));
        }
        if activation_cost_shape_matches_words(&normalized[idx..], WITH_POWER_PREFIX_PATTERN) {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![OwnedLexToken::synthetic_word(amount_word)];
            let (threshold, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1
                || !activation_word_at_is(&normalized, idx + 3, OR_WORD)
                || idx + 5 != normalized.len()
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }

            return match normalized.get(idx + 4) {
                Some(&"less") => Ok(Some(StaticAbility::cant_be_blocked_by_power_or_less(
                    threshold as i32,
                ))),
                Some(&"greater") | Some(&"more") => Ok(Some(
                    StaticAbility::cant_be_blocked_by_power_or_greater(threshold as i32),
                )),
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                ))),
            };
        }

        if activation_cost_shape_matches_words(&normalized[idx..], WITH_FLYING_PATTERN)
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature()
                        .with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by creatures with flying".to_string(),
            )));
        }
        if let Some(color_word) = normalized.get(idx).copied()
            && activation_word_at_is_any(&normalized, idx + 1, CREATURE_OR_CREATURES_WORDS)
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked by {color_word} creatures"),
            )));
        }

        if activation_word_at_is_any(&normalized, idx, WALL_OR_WALLS_WORDS)
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by walls".to_string(),
            )));
        }
    }

    if activation_cost_shape_matches_words(&normalized, CANT_BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN) {
        let idx = if activation_cost_shape_matches_words(
            &normalized,
            THIS_CREATURE_CANT_BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN,
        ) {
            7
        } else if activation_cost_shape_matches_words(
            &normalized,
            THIS_CANT_BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN,
        ) {
            6
        } else {
            5
        };
        if let Some((min_blockers, used)) =
            parse_greater_than_or_equal_count_prefix_from_words(&normalized[idx..])
            && activation_word_at_is_any(&normalized, idx + used, CREATURE_OR_CREATURES_WORDS)
            && idx + used + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::cant_be_blocked_except_by_n_or_more(
                min_blockers as usize,
            )));
        }
        if let Some(color_word) = normalized.get(idx)
            && activation_word_at_is_any(&normalized, idx + 1, CREATURE_OR_CREATURES_WORDS)
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked except by {color_word} creatures"),
            )));
        }
        if activation_word_at_is(&normalized, idx, ARTIFACT_WORD)
            && activation_word_at_is_any(&normalized, idx + 1, CREATURE_OR_CREATURES_WORDS)
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_type(CardType::Artifact),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by artifact creatures".to_string(),
            )));
        }
        if activation_word_at_is_any(&normalized, idx, WALL_OR_WALLS_WORDS)
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by walls".to_string(),
            )));
        }
    }

    let cant_attack_unless_cast_creature_spell_tail = activation_cost_shape_matches_words(
        &normalized,
        CAST_CREATURE_SPELL_THIS_TURN_UNLESS_TAIL_PATTERN,
    );
    let cant_attack_unless_cast_noncreature_spell_tail = activation_cost_shape_matches_words(
        &normalized,
        CAST_NONCREATURE_SPELL_THIS_TURN_UNLESS_TAIL_PATTERN,
    );
    if cant_attack_unless_cast_creature_spell_tail
        && activation_cost_shape_matches_words(&normalized, THIS_CANT_ATTACK_PREFIX_PATTERN)
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn(),
        ));
    }
    if cant_attack_unless_cast_noncreature_spell_tail
        && activation_cost_shape_matches_words(&normalized, THIS_CANT_ATTACK_PREFIX_PATTERN)
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn(),
        ));
    }

    let starts_with_this_cant_attack_unless =
        activation_cost_shape_matches_words(&normalized, THIS_CANT_ATTACK_UNLESS_PREFIX_PATTERN);
    if starts_with_this_cant_attack_unless && let Some(tail) = cant_attack_unless_tail(&normalized)
    {
        let static_text = format!("Can't attack unless {}", tail.join(" "));
        let static_with = |condition| {
            Ok(Some(StaticAbility::cant_attack_unless_condition(
                condition,
                static_text.clone(),
            )))
        };

        if activation_cost_shape_matches_words(
            tail,
            CONTROL_MORE_CREATURES_THAN_DEFENDING_PLAYER_PATTERN,
        ) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Creature),
                ),
            );
        }
        if activation_cost_shape_matches_words(
            tail,
            CONTROL_MORE_LANDS_THAN_DEFENDING_PLAYER_PATTERN,
        ) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Land),
                ),
            );
        }
        if let Some(condition) = control_creature_with_power_condition_from_tail(tail) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(condition),
            );
        }
        if let Some(condition) = source_control_condition_from_tail(tail) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(condition),
            );
        }
        if activation_cost_shape_matches_words(tail, MOUNTAIN_ON_BATTLEFIELD_PATTERN) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Mountain),
                    count: 1,
                },
            );
        }
        if let ["there", "are", rest @ ..] = tail
            && let Some((value, used)) = parse_greater_than_or_equal_count_prefix_from_words(rest)
            && activation_cost_shape_matches_words(&rest[used..], CARDS_IN_YOUR_GRAVEYARD_PATTERN)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(
                    value,
                ),
            );
        }
        if let ["there", "are", rest @ ..] = tail
            && let Some((value, used)) = parse_greater_than_or_equal_count_prefix_from_words(rest)
            && activation_cost_shape_matches_words(&rest[used..], ISLANDS_ON_BATTLEFIELD_PATTERN)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Island),
                    count: value,
                },
            );
        }
        if activation_cost_shape_matches_words(tail, DEFENDING_PLAYER_POISONED_PATTERN) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsPoisoned,
                ),
            );
        }
        if let ["defending", "player", "has", rest @ ..] = tail
            && let Some((value, used)) = parse_greater_than_or_equal_count_prefix_from_words(rest)
            && activation_cost_shape_matches_words(&rest[used..], CARDS_IN_THEIR_GRAVEYARD_PATTERN)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::HasCardsInGraveyardOrMore(
                        value,
                    ),
                ),
            );
        }
        if activation_cost_shape_matches_words(tail, DEFENDING_PLAYER_CONTROLS_ENCHANTMENT_PATTERN)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::ControlsEnchantmentOrEnchantedPermanent,
                ),
            );
        }
        if let Some(filter) = defending_player_controls_filter_from_tail(tail) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(filter),
                ),
            );
        }
        if let Some((value, used)) = parse_greater_than_or_equal_count_prefix_from_words(tail)
            && activation_cost_shape_matches_words(
                &tail[used..],
                OTHER_CREATURES_ATTACK_TAIL_PATTERN,
            )
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(
                        value,
                    ),
                ),
            );
        }
        if activation_cost_shape_matches_words(
            tail,
            CREATURE_WITH_GREATER_POWER_ALSO_ATTACKS_PATTERN,
        ) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::CreatureWithGreaterPowerAlsoAttacks,
                ),
            );
        }
        if activation_cost_shape_matches_words(tail, BLACK_OR_GREEN_CREATURE_ALSO_ATTACKS_PATTERN) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::BlackOrGreenCreatureAlsoAttacks,
                ),
            );
        }
        if activation_cost_shape_matches_words(tail, OPPONENT_DEALT_DAMAGE_THIS_TURN_PATTERN) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn,
            );
        }
        if let Some(condition) = player_controls_at_least_condition_from_tail(tail) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(condition),
            );
        }
        if activation_cost_shape_matches_words(tail, SACRIFICE_LAND_ATTACK_COST_PATTERN) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land(),
                        count: 1,
                    },
                ),
            );
        }
        if let ["you", "sacrifice", rest @ ..] = tail
            && let Some((value, used)) = parse_exact_count_from_words(rest)
            && activation_word_at_is(rest, used, ISLANDS_WORD)
            && used + 1 == rest.len()
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land().with_subtype(Subtype::Island),
                        count: value,
                    },
                ),
            );
        }
        if activation_cost_shape_matches_words(tail, RETURN_ENCHANTMENT_ATTACK_COST_PATTERN) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::ReturnPermanentsToOwnersHand {
                        filter: ObjectFilter::enchantment(),
                        count: 1,
                    },
                ),
            );
        }
        if activation_cost_shape_matches_words(tail, PAY_PER_PLUS_ONE_COUNTER_ATTACK_COST_PATTERN) {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::PayGenericPerSourceCounter {
                        counter_type: crate::object::CounterType::PlusOnePlusOne,
                        amount_per_counter: 1,
                    },
                ),
            );
        }
        if let Some(player_status) =
            crate::runtime_backend::grammar::conditions::parse_player_status_condition_words(tail)
            && player_status.player == PlayerFilter::Defending
            && player_status.status
                == crate::runtime_backend::grammar::conditions::PlayerStatusAst::Monarch
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsMonarch,
                ),
            );
        }
    }

    if let Some((neg_start, neg_end)) = find_negation_span(tokens) {
        let subject_tokens = trim_commas(&tokens[..neg_start]);
        let remainder_tokens = trim_commas(&tokens[neg_end..]);
        let remainder_words_storage = normalize_cant_words(&remainder_tokens);
        let remainder_words = remainder_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
        if activation_words_eq_any(&subject_words, SELF_SUBJECT_PHRASES)
            && remainder_words
                .first()
                .is_some_and(|word| activation_word_is_any(word, &[BLOCK_WORD]))
            && remainder_words.len() > 1
        {
            let attacker_tokens = trim_commas(&remainder_tokens[1..]);
            let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported blocker restriction filter (clause: '{}')",
                        normalized.join(" ")
                    ))
                })?;
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::source(),
                    attacker_filter,
                ),
                format!(
                    "this creature can't block {}",
                    crate::runtime_backend::token_word_refs(&attacker_tokens).join(" ")
                ),
            )));
        }
        if activation_words_eq(&remainder_words, &[TRANSFORM_WORD]) {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Ok(None);
            };
            let subject_text = crate::runtime_backend::token_word_refs(&subject_tokens).join(" ");
            if subject_text.is_empty() {
                return Ok(None);
            }
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::transform(filter),
                format!("{subject_text} can't transform"),
            )));
        }
    }

    if activation_cost_shape_matches_words(&normalized, OPPONENTS_CANT_CAST_SPELLS_WITH_PATTERN)
        && normalized.len() >= 8
        && normalized
            .get(6..8)
            .is_some_and(|words| activation_words_eq(words, MANA_VALUES_WORDS))
    {
        let parity = match normalized[5] {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return Ok(None),
        };
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::cast_spells_matching(
                PlayerFilter::Opponent,
                ObjectFilter::spell().with_mana_value_parity(parity),
            ),
            format_negated_restriction_display(tokens),
        )));
    }

    if activation_cost_shape_matches_words(
        &normalized,
        OPPONENTS_CANT_BLOCK_WITH_CREATURES_WITH_PATTERN,
    ) && normalized.len() >= 10
        && normalized
            .get(8..10)
            .is_some_and(|words| activation_words_eq(words, MANA_VALUES_WORDS))
    {
        let parity = match normalized[7] {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return Ok(None),
        };
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::block(
                ObjectFilter::creature()
                    .opponent_controls()
                    .with_mana_value_parity(parity),
            ),
            format_negated_restriction_display(tokens),
        )));
    }

    if activation_cost_shape_matches_words(
        &normalized,
        THIS_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN,
    ) && activation_cost_shape_matches_words(&normalized, EVEN_COUNTERS_ON_IT_SUFFIX_PATTERN)
    {
        return Ok(Some(StaticAbility::rule_fallback_text(
            format_negated_restriction_display(tokens),
        )));
    }

    if activation_cost_shape_matches_words(
        &normalized,
        THIS_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN,
    ) && let Some(tail) = cant_attack_or_block_unless_tail(&normalized)
        && let ["there", "are", rest @ ..] = tail
        && let Some((count, used)) = parse_greater_than_or_equal_count_prefix_from_words(rest)
        && activation_cost_shape_matches_words(&rest[used..], CARDS_IN_EXILE_PATTERN)
    {
        let condition =
            crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::Count(
                    ObjectFilter::default().in_zone(Zone::Exile).nontoken(),
                ),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(count as i32),
            }));
        return Ok(Some(
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                format_negated_restriction_display(tokens),
            )
            .with_condition(condition)
            .unwrap_or_else(|| {
                StaticAbility::restriction(
                    crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                    format_negated_restriction_display(tokens),
                )
            }),
        ));
    }

    if activation_cost_shape_matches_words(
        &normalized,
        THIS_CANT_ATTACK_OR_BLOCK_UNLESS_PREFIX_PATTERN,
    ) && let Some(tail) = cant_attack_or_block_unless_tail(&normalized)
        && let Some(control_condition) = player_controls_at_least_condition_from_tail(tail)
    {
        let condition = crate::ConditionExpr::Not(Box::new(control_condition));
        return Ok(Some(
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                format_negated_restriction_display(tokens),
            )
            .with_condition(condition)
            .unwrap_or_else(|| {
                StaticAbility::restriction(
                    crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                    format_negated_restriction_display(tokens),
                )
            }),
        ));
    }

    if activation_cost_shape_matches_words(
        &normalized,
        IF_SOURCE_YOU_CONTROL_DOUBLE_MANA_VALUE_INSTEAD_PATTERN,
    ) {
        return Ok(Some(StaticAbility::rule_fallback_text(
            crate::runtime_backend::token_word_refs(tokens).join(" "),
        )));
    }

    if let Some(parsed) = parse_cant_restriction_clause(tokens)?
        && parsed.target.is_none()
        && matches!(
            parsed.restriction,
            crate::effect::Restriction::GainLife(_)
                | crate::effect::Restriction::SearchLibraries(_)
                | crate::effect::Restriction::CastSpellsMatching(_, _)
                | crate::effect::Restriction::ActivateNonManaAbilities(_)
                | crate::effect::Restriction::ActivateAbilitiesOf(_)
                | crate::effect::Restriction::ActivateTapAbilitiesOf(_)
                | crate::effect::Restriction::ActivateNonManaAbilitiesOf(_)
                | crate::effect::Restriction::CastMoreThanOneSpellEachTurn(_, _)
                | crate::effect::Restriction::DrawCards(_)
                | crate::effect::Restriction::DrawExtraCards(_)
                | crate::effect::Restriction::LoseLife(_)
                | crate::effect::Restriction::ChangeLifeTotal(_)
                | crate::effect::Restriction::LoseGame(_)
                | crate::effect::Restriction::WinGame(_)
                | crate::effect::Restriction::PreventDamage
        )
    {
        let ability =
            canonical_negated_restriction_static_ability(&normalized).unwrap_or_else(|| {
                StaticAbility::restriction(
                    parsed.restriction,
                    format_negated_restriction_display(tokens),
                )
            });
        return Ok(Some(ability));
    }

    if let Some(resolution) = direct_cant_static_ability(&normalized, tokens) {
        match resolution {
            StaticAbilityShapeResolution::Ability(ability) => return Ok(Some(ability)),
            StaticAbilityShapeResolution::Decline => return Ok(None),
        }
    }

    if let Some(parsed) = parse_negated_object_restriction_clause(tokens)?
        && parsed.target.is_none()
    {
        return Ok(Some(StaticAbility::restriction(
            parsed.restriction,
            format_negated_restriction_display(tokens),
        )));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::super::super::util::tokenize_line;
    use super::*;

    #[test]
    fn parse_cant_attack_or_block_unless_cards_in_exile_condition() {
        let tokens = tokenize_line(
            "This creature can't attack or block unless there are seven or more cards in exile.",
            0,
        );

        let abilities = parse_cant_clauses(&tokens)
            .expect("cant-attack-or-block-unless-exile-count should parse")
            .expect("expected a static restriction");

        assert_eq!(abilities.len(), 1);
        let debug = format!("{:?}", abilities[0]);
        assert!(debug.contains("AttackOrBlock"), "{debug}");
        assert!(debug.contains("ValueComparison"), "{debug}");
        assert!(debug.contains("GreaterThanOrEqual"), "{debug}");
        assert!(debug.contains("Fixed(7)"), "{debug}");
        assert!(debug.contains("Exile"), "{debug}");

        let display = abilities[0].display().to_ascii_lowercase();
        assert!(
            display.contains("can't attack or block unless there are seven or more cards in exile")
                || display
                    .contains("cant attack or block unless there are seven or more cards in exile"),
            "expected original conditional attack/block restriction text, got {display}"
        );
    }

    #[test]
    fn cant_attack_unless_control_tails_use_shared_capture_condition_shape() {
        let simple = source_control_condition_from_tail(&["you", "control", "an", "artifact"])
            .expect("simple control condition should parse");
        let debug = format!("{simple:?}");
        assert!(debug.contains("YouControl"), "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");

        let threshold =
            source_control_condition_from_tail(&["you", "control", "seven", "or", "more", "lands"])
                .expect("threshold control condition should parse");
        let debug = format!("{threshold:?}");
        assert!(debug.contains("PlayerHasAtLeast"), "{debug}");
        assert!(debug.contains("Land"), "{debug}");
        assert!(debug.contains("count: 7"), "{debug}");

        let qualified =
            source_control_condition_from_tail(&["you", "control", "another", "artifact"])
                .expect("qualified control condition should parse");
        let debug = format!("{qualified:?}");
        assert!(debug.contains("YouControl"), "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("other: true"), "{debug}");

        let subtype_union = source_control_condition_from_tail(&[
            "you", "control", "a", "knight", "or", "a", "soldier",
        ])
        .expect("subtype union control condition should parse");
        let debug = format!("{subtype_union:?}");
        assert!(debug.contains("YouControl"), "{debug}");
        assert!(debug.contains("Knight"), "{debug}");
        assert!(debug.contains("Soldier"), "{debug}");

        let sized_creature =
            source_control_condition_from_tail(&["you", "control", "a", "1/1", "creature"])
                .expect("sized creature control condition should parse");
        let debug = format!("{sized_creature:?}");
        assert!(debug.contains("YouControl"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("power: Some(Equal(1))"), "{debug}");
        assert!(debug.contains("toughness: Some(Equal(1))"), "{debug}");
    }

    #[test]
    fn parse_cant_attack_unless_routes_control_tail_through_capture_shape() {
        let cases = [
            (
                "This creature can't attack unless you control another artifact.",
                "other: true",
            ),
            (
                "This creature can't attack unless you control seven or more lands.",
                "PlayerHasAtLeast",
            ),
        ];

        for (text, expected_debug) in cases {
            let tokens = tokenize_line(text, 0);
            let abilities = parse_cant_clauses(&tokens)
                .expect("cant-attack-unless-control condition should parse")
                .expect("expected a static restriction");

            assert_eq!(abilities.len(), 1, "{text}");
            let debug = format!("{:?}", abilities[0]);
            assert!(debug.contains("CantAttackUnlessCondition"), "{debug}");
            assert!(debug.contains(expected_debug), "{debug}");
        }
    }

    #[test]
    fn cant_attack_unless_defending_player_control_tails_use_shared_capture_condition_shape() {
        let cases = [
            (
                &["defending", "player", "controls", "an", "island"][..],
                "Island",
            ),
            (
                &["defending", "player", "controls", "a", "snow", "land"],
                "Snow",
            ),
            (
                &[
                    "defending",
                    "player",
                    "controls",
                    "a",
                    "creature",
                    "with",
                    "flying",
                ],
                "Flying",
            ),
            (
                &["defending", "player", "controls", "a", "blue", "permanent"],
                "colors: Some",
            ),
        ];

        for (tail, expected_debug) in cases {
            let filter = defending_player_controls_filter_from_tail(tail)
                .expect("defending-player controls tail should parse");
            let debug = format!("{filter:?}");
            assert!(debug.contains(expected_debug), "{debug}");
        }
    }

    #[test]
    fn parse_cant_attack_unless_routes_defending_player_control_tail_through_capture_shape() {
        let cases = [
            (
                "This creature can't attack unless defending player controls an Island.",
                "Island",
            ),
            (
                "This creature can't attack unless defending player controls a snow land.",
                "Snow",
            ),
            (
                "This creature can't attack unless defending player controls a creature with flying.",
                "Flying",
            ),
            (
                "This creature can't attack unless defending player controls a blue permanent.",
                "colors: Some",
            ),
        ];

        for (text, expected_debug) in cases {
            let tokens = tokenize_line(text, 0);
            let abilities = parse_cant_clauses(&tokens)
                .expect("cant-attack-unless-defending-player-controls condition should parse")
                .expect("expected a static restriction");

            assert_eq!(abilities.len(), 1, "{text}");
            let debug = format!("{:?}", abilities[0]);
            assert!(debug.contains("DefendingPlayerCondition"), "{debug}");
            assert!(debug.contains(expected_debug), "{debug}");
        }
    }

    #[test]
    fn parse_this_token_cant_be_blocked_clause() {
        let tokens = tokenize_line("This token can't be blocked.", 0);

        let abilities = parse_cant_clauses(&tokens)
            .expect("this-token-cant-be-blocked clause should parse")
            .expect("expected unblockable static ability");

        assert_eq!(abilities.len(), 1);
        let display = abilities[0].display().to_ascii_lowercase();
        let debug = format!("{:?}", abilities[0]).to_ascii_lowercase();
        assert!(
            display.contains("can't be blocked")
                || display.contains("cant be blocked")
                || display.contains("unblockable")
                || debug.contains("unblockable"),
            "expected unblockable static ability, display={display}, debug={debug}"
        );
    }
}

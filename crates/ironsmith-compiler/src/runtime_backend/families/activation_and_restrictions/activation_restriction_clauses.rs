use super::*;
use crate::runtime_backend::grammar::activation_restrictions::{
    RestrictionSurface, restriction_surface,
};

const PLAYERS_CANT_LOSE_OR_WIN_GAME_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    prefix
        & [
            "players", "cant", "lose", "the", "game", "or", "win", "the", "game",
        ]
);
const BE_TARGET_OF_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["be", "the", "target", "of"]);
const SPELLS_OR_ABILITIES_FROM_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["spells", "or", "abilities", "from"]);
const CANT_WIN_GAME_TAIL_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    contains_any_phrases
        & [&[
            &["or", "win", "the", "game", "this"],
            &["or", "win", "the", "game"]
        ]]
);
const OPPONENTS_CANT_BLOCK_ODD_EVEN_MV_PATTERN: RestrictionSurface<'static> = restriction_surface!(prefix & ["your", "opponents", "cant", "block", "with", "creatures", "with"]; suffix & ["mana", "values"]);
const PLAYERS_CANT_GAIN_LIFE_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["players", "cant", "gain", "life"]);
const PLAYERS_CANT_SEARCH_LIBRARIES_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["players", "cant", "search", "libraries"]);
const PLAYERS_CANT_DRAW_CARDS_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["players", "cant", "draw", "cards"]);
const PLAYERS_CANT_DRAW_MORE_THAN_ONE_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact
        & [
            "players", "cant", "draw", "more", "than", "one", "card", "each", "turn"
        ]
);
const DAMAGE_CANT_BE_PREVENTED_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["damage", "cant", "be", "prevented"]);
const DAMAGED_THIS_WAY_TAG: &str = "damaged_0";
const YOU_CANT_LOSE_GAME_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["you", "cant", "lose", "the", "game"]);
const OPPONENTS_CANT_WIN_GAME_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["your", "opponents", "cant", "win", "the", "game"]);
const YOUR_LIFE_TOTAL_CANT_CHANGE_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["your", "life", "total", "cant", "change"]);
const OPPONENTS_CANT_DRAW_MORE_THAN_ONE_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &[
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
            ],
            &[
                "each", "opponent", "cant", "draw", "more", "than", "one", "card", "each", "turn",
            ],
        ]
);
const YOU_CANT_GAIN_LIFE_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["you", "cant", "gain", "life"]);
const YOU_CANT_SEARCH_LIBRARIES_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["you", "cant", "search", "libraries"]);
const YOU_CANT_DRAW_CARDS_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["you", "cant", "draw", "cards"]);
const YOU_CANT_BECOME_MONARCH_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["you", "cant", "become", "the", "monarch"],
            &["you", "cant", "become", "monarch"],
            &["you", "cant", "become", "the", "monarch", "this", "turn"],
            &["you", "cant", "become", "monarch", "this", "turn"],
        ]
);
const ITERATED_PLAYER_CANT_GAIN_LIFE_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["they", "cant", "gain", "life"],
            &["that", "player", "cant", "gain", "life"],
        ]
);
const OPPONENTS_CANT_GAIN_LIFE_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["opponents", "cant", "gain", "life"]);
const CANT_CAST_SPELLS_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["cast", "spells"], &["cast", "spells", "this", "turn"]]);
const CANT_CAST_CREATURE_SPELLS_TAIL_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["cast", "creature", "spells"],
            &["cast", "creature", "spells", "this", "turn"],
        ]
);
const CANT_CAST_SPELLS_OF_CHOSEN_TYPE_TAIL_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["cast", "spells", "of", "the", "chosen", "type"],
            &[
                "cast", "spells", "of", "the", "chosen", "type", "this", "turn",
            ],
        ]
);
const CANT_CAST_SPELLS_WITH_PARITY_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["cast", "spells", "with"]; suffix & ["mana", "values"]);
const THIS_TURN_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["this", "turn"]);
const CANT_CAST_ADDITIONAL_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["cant", "cast", "additional"]);
const SPELL_CANT_BE_CAST_SUFFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(suffix & ["cant", "be", "cast"]);
const NON_CREATURE_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["non", "creature"]);
const MANA_VALUE_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["mana", "value"]);
const CAST_MORE_THAN_ONE_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["cast", "more", "than", "one"]);
const CAST_SPELLS_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["cast", "spells"]);
const SPELL_EACH_TURN_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["spell", "each", "turn"]);
const CARD_TYPE_LIST_IGNORED_WORD_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["a"],
            &["an"],
            &["the"],
            &["or"],
            &["and"],
            &[","],
            &["unless"],
            &["theyre"],
            &["mana"],
            &["abilities"],
        ]
);
const CAST_WORD: &str = "cast";
const SPELL_WORD: &str = "spell";
const SPELLS_WORD: &str = "spells";
const SPELL_OR_SPELLS_WORDS: &[&str] = &["spell", "spells"];
const NONCREATURE_WORD: &str = "noncreature";
const WITH_WORD: &str = "with";
const ARTICLE_WORDS: &[&str] = &["a", "an"];
const X_IN_MANA_COST_PREFIX_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    prefix_any
        & [
            &["x", "in", "their", "mana", "cost"],
            &["x", "in", "their", "mana", "costs"],
            &["x", "in", "its", "mana", "cost"],
            &["x", "in", "its", "mana", "costs"],
        ]
);
const WHO_HAS_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["who", "has"]);
const ACTIVATE_ABILITIES_THAT_ARENT_MANA_TAIL_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact
        & [
            "activate",
            "abilities",
            "that",
            "arent",
            "mana",
            "abilities"
        ]
);
const ACTIVATE_ABILITIES_OF_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["activate", "abilities", "of"]);
const UNLESS_MANA_ABILITIES_SUFFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(suffix & ["unless", "theyre", "mana", "abilities"]);
const ACTIVATED_ABILITIES_OWNER_SUFFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(suffix & ["activated", "abilities"]);
const ACTIVATED_ABILITIES_TAP_COST_OWNER_SUFFIX_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    suffix
        & [
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
        ]
);
const ACTIVATED_ABILITIES_OF_OWNER_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["activated", "abilities", "of"]);
const ACTIVATED_ABILITIES_TAP_COST_OF_OWNER_PREFIX_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    prefix
        & [
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
            "of",
        ]
);
const IT_OWNER_REFERENCE_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["it"], &["its"], &["them"], &["their"]]);
const POSSESSIVE_ACTIVATED_ABILITIES_PREFIX_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    prefix_any
        & [
            &["its", "activated", "abilities"],
            &["their", "activated", "abilities"],
        ]
);
const PLAYERS_DEALT_DAMAGE_THIS_WAY_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["players", "dealt", "damage", "this", "way"]);
const THAT_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["that", "player"]);
const YOUR_OPPONENTS_WHO_HAVE_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["your", "opponents", "who", "have"]);
const EACH_PLAYER_WHO_HAS_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["each", "player", "who", "has"]);
const EACH_OPPONENT_WHO_HAS_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["each", "opponent", "who", "has"]);
const YOUR_OPPONENTS_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["your", "opponents"]);
const EACH_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["each", "player"]);
const EACH_OPPONENT_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["each", "opponent"]);
const YOU_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["you"]);
const NEGATED_RESTRICTION_OPPONENT_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["your", "opponents"], &["opponents"]]);
const ITERATED_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["that", "player"], &["they"]]);
const DAMAGED_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["players", "dealt", "damage", "this", "way"]);
const OPPONENT_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["your", "opponents"],
            &["each", "opponent"],
            &["opponents"]
        ]
);
const ANY_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["players"], &["each", "player"]]);
const ENCHANTED_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["enchanted", "player"]);
const DEFENDING_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["defending", "player"]);
const ATTACKING_PLAYER_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["attacking", "player"]);
const CONTROLLER_OF_IT_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["its", "controller"], &["their", "controller"]]);
const OWNER_OF_IT_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["its", "owner"], &["their", "owner"]]);
const PLAYER_GAIN_LIFE_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["gain", "life"]);
const PLAYER_SEARCH_LIBRARIES_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["search", "libraries"]);
const PLAYER_LOSE_GAME_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["lose", "the", "game"]);
const PLAYER_LOSE_LIFE_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["lose", "life"]);
const PLAYER_WIN_GAME_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["win", "the", "game"]);
const PLAYER_DRAW_CARDS_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["draw", "cards"]);
const PLAYER_DRAW_MORE_THAN_ONE_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["draw", "more", "than", "one", "card"]);
const PLAYER_GET_POISON_COUNTERS_TAIL_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    prefix_any
        & [
            &["get", "additional", "poison", "counters"],
            &["get", "poison", "counters"],
        ]
);
const PLAYER_CAST_MORE_THAN_ONE_SPELL_EACH_TURN_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["cast", "more", "than", "one", "spell", "each", "turn"]);
const BE_PREVENTED_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["be", "prevented"]);
const DAMAGE_CAUSE_YOU_LOSE_LIFE_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["cause", "you", "to", "lose", "life"]);
const DAMAGE_CAUSE_PLAYERS_LOSE_LIFE_TAIL_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    prefix_any
        & [
            &["cause", "players", "to", "lose", "life"],
            &["cause", "each", "player", "to", "lose", "life"],
        ]
);
const DAMAGE_CAUSE_THAT_PLAYER_LOSE_LIFE_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["cause", "that", "player", "to", "lose", "life"]);
const ATTACK_YOU_OR_PLANESWALKERS_YOU_CONTROL_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["attack", "you", "or", "planeswalkers", "you", "control"]);
const BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["be", "blocked", "except", "by"]);
const BE_BLOCKED_BY_PREFIX_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["be", "blocked", "by"]);
const BE_ACTIVATED_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["be", "activated"], &["be", "activated", "this", "turn"]]);
const BE_ACTIVATED_UNLESS_MANA_ABILITIES_TAIL_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact & ["be", "activated", "unless", "theyre", "mana", "abilities"]);
const LOSE_UNSPENT_MANA_STEPS_PATTERN: RestrictionSurface<'static> = restriction_surface!(prefix & ["lose", "unspent"]; contains_phrases & [&["mana", "as", "steps"]]);
const LOSE_THIS_MANA_STEPS_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["lose", "this", "mana", "as", "steps"]);
const NON_WORD: &str = "non";
const AND_OR_CONNECTOR_WORDS: &[&str] = &["or", "and"];
const DURING_WORD: &str = "during";
const AND_OR_WORD: &str = "and/or";
const AND_OR_PHRASE_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(prefix & ["and", "or"]);
const DEALT_DAMAGE_THIS_WAY_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(contains_phrases & [&["dealt", "damage", "this", "way"]]);
const CANT_WORDS: &[&str] = &["cant", "can't", "cannot"];
const CAN_WORD: &str = "can";
const T_WORD: &str = "t";
const DOESNT_OR_DONT_WORDS: &[&str] = &["doesnt", "dont"];
const CONTROL_OR_OWN_WORDS: &[&str] = &["control", "controls", "own", "owns"];
const DOES_DO_CAN_WORDS: &[&str] = &["does", "do", "can"];
const DOES_OR_DO_WORDS: &[&str] = &["does", "do"];
const NOT_WORD: &str = "not";
const DONT_LOSE_THIS_MANA_STEPS_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["you", "dont", "lose", "this", "mana", "as", "steps"],
            &["you", "don't", "lose", "this", "mana", "as", "steps"],
            &[
                "you", "dont", "lose", "this", "mana", "as", "steps", "and", "phases", "end",
            ],
            &[
                "you", "don't", "lose", "this", "mana", "as", "steps", "and", "phases", "end",
            ],
        ]
);
const PLAYER_RESTRICTION_SUBJECT_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any
        & [
            &["you"],
            &["your", "opponents"],
            &["opponents"],
            &["players"],
            &["each", "player"],
            &["enchanted", "player"],
        ]
);
const DAMAGE_RESTRICTION_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["damage"], &["the", "damage"], &["that", "damage"]]);
const TAGGED_OBJECT_PRONOUN_SUBJECT_PATTERN: RestrictionSurface<'static> =
    restriction_surface!(exact_any & [&["it"], &["they"], &["them"], &["itself"], &["themselves"]]);
const BLOCK_WORD: &str = "block";
const POWER_OR_TOUGHNESS_SUBJECT_PATTERN: RestrictionSurface<'static> = restriction_surface!(
    exact_any & [&["power", "or", "toughness"], &["toughness", "or", "power"]]
);
const EFFECT_ACTION_RESTRICTION_TAIL_HEAD_WORDS: &[&str] = &[
    "put",
    "draw",
    "reveal",
    "look",
    "search",
    "create",
    "return",
    "exile",
    "sacrifice",
    "discard",
    "gain",
    "lose",
];
const SIMPLE_NEGATED_OBJECT_TAIL_PATTERNS: &[(
    RestrictionSurface<'static>,
    ObjectRestrictionTailKind,
)] = &[
    (
        restriction_surface!(exact_any & [&["attack"], &["attack", "this", "turn"]]),
        ObjectRestrictionTailKind::Attack,
    ),
    (
        restriction_surface!(
            exact_any & [&["attack", "alone"], &["attack", "alone", "this", "turn"]]
        ),
        ObjectRestrictionTailKind::AttackAlone,
    ),
    (
        restriction_surface!(
            exact_any
                & [
                    &["attack", "or", "block"],
                    &["attack", "or", "block", "this", "turn"]
                ]
        ),
        ObjectRestrictionTailKind::AttackOrBlock,
    ),
    (
        restriction_surface!(
            exact_any
                & [
                    &["attack", "or", "block", "alone"],
                    &["attack", "or", "block", "alone", "this", "turn"]
                ]
        ),
        ObjectRestrictionTailKind::AttackOrBlockAlone,
    ),
    (
        restriction_surface!(exact_any & [&["block"], &["block", "this", "turn"]]),
        ObjectRestrictionTailKind::Block,
    ),
    (
        restriction_surface!(
            exact_any & [&["block", "alone"], &["block", "alone", "this", "turn"]]
        ),
        ObjectRestrictionTailKind::BlockAlone,
    ),
    (
        restriction_surface!(exact_any & [&["be", "blocked"], &["be", "blocked", "this", "turn"]]),
        ObjectRestrictionTailKind::BeBlocked,
    ),
    (
        restriction_surface!(exact & ["be", "destroyed"]),
        ObjectRestrictionTailKind::BeDestroyed,
    ),
    (
        restriction_surface!(
            exact_any
                & [
                    &["be", "regenerated"],
                    &["be", "regenerated", "this", "turn"]
                ]
        ),
        ObjectRestrictionTailKind::BeRegenerated,
    ),
    (
        restriction_surface!(exact & ["be", "sacrificed"]),
        ObjectRestrictionTailKind::BeSacrificed,
    ),
    (
        restriction_surface!(exact & ["be", "countered"]),
        ObjectRestrictionTailKind::BeCountered,
    ),
    (
        restriction_surface!(exact & ["transform"]),
        ObjectRestrictionTailKind::Transform,
    ),
    (
        restriction_surface!(
            exact_any
                & [
                    &["phase", "out"],
                    &["phase", "out", "this", "turn"],
                    &["phases", "out"]
                ]
        ),
        ObjectRestrictionTailKind::PhaseOut,
    ),
    (
        restriction_surface!(exact & ["be", "targeted"]),
        ObjectRestrictionTailKind::BeTargeted,
    ),
];

fn player_negated_restriction_subject(words: &[&str]) -> Option<PlayerFilter> {
    if restriction_surface_accepts(words, YOU_PLAYER_SUBJECT_PATTERN) {
        Some(PlayerFilter::You)
    } else if restriction_surface_accepts(words, NEGATED_RESTRICTION_OPPONENT_SUBJECT_PATTERN) {
        Some(PlayerFilter::Opponent)
    } else if restriction_surface_accepts(words, ANY_PLAYER_SUBJECT_PATTERN) {
        Some(PlayerFilter::Any)
    } else if restriction_surface_accepts(words, ENCHANTED_PLAYER_SUBJECT_PATTERN) {
        Some(PlayerFilter::TaggedPlayer(TagKey::from("enchanted")))
    } else {
        None
    }
}

fn restriction_surface_accepts<'a>(words: &[&str], shape: RestrictionSurface<'a>) -> bool {
    shape.accepts_words(words)
}

fn activation_restriction_word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn activation_restriction_word_is(word: &str, expected: &str) -> bool {
    activation_restriction_word_is_any(word, &[expected])
}

fn activation_restriction_word_at_is_any(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words
        .get(idx)
        .is_some_and(|word| activation_restriction_word_is_any(word, expected))
}

fn activation_restriction_word_at_is(words: &[&str], idx: usize, expected: &str) -> bool {
    activation_restriction_word_at_is_any(words, idx, &[expected])
}

fn activation_restriction_token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| activation_restriction_word_is(word, expected))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectRestrictionTailKind {
    Attack,
    AttackAlone,
    AttackOrBlock,
    AttackOrBlockAlone,
    Block,
    BlockAlone,
    BeBlocked,
    BeDestroyed,
    BeRegenerated,
    BeSacrificed,
    BeCountered,
    Transform,
    PhaseOut,
    BeTargeted,
}

fn simple_negated_object_restriction(
    words: &[&str],
    filter: &ObjectFilter,
) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    let (_, kind) = SIMPLE_NEGATED_OBJECT_TAIL_PATTERNS
        .iter()
        .find(|(pattern, _)| restriction_surface_accepts(words, *pattern))?;
    Some(match kind {
        ObjectRestrictionTailKind::Attack => Restriction::attack(filter.clone()),
        ObjectRestrictionTailKind::AttackAlone => Restriction::attack_alone(filter.clone()),
        ObjectRestrictionTailKind::AttackOrBlock => Restriction::attack_or_block(filter.clone()),
        ObjectRestrictionTailKind::AttackOrBlockAlone => {
            Restriction::attack_or_block_alone(filter.clone())
        }
        ObjectRestrictionTailKind::Block => Restriction::block(filter.clone()),
        ObjectRestrictionTailKind::BlockAlone => Restriction::block_alone(filter.clone()),
        ObjectRestrictionTailKind::BeBlocked => Restriction::be_blocked(filter.clone()),
        ObjectRestrictionTailKind::BeDestroyed => Restriction::be_destroyed(filter.clone()),
        ObjectRestrictionTailKind::BeRegenerated => Restriction::be_regenerated(filter.clone()),
        ObjectRestrictionTailKind::BeSacrificed => Restriction::be_sacrificed(filter.clone()),
        ObjectRestrictionTailKind::BeCountered => Restriction::be_countered(filter.clone()),
        ObjectRestrictionTailKind::Transform => Restriction::transform(filter.clone()),
        ObjectRestrictionTailKind::PhaseOut => Restriction::phase_out(filter.clone()),
        ObjectRestrictionTailKind::BeTargeted => Restriction::be_targeted(filter.clone()),
    })
}

fn source_filtered_target_restriction(
    tokens: &[OwnedLexToken],
    words: &[&str],
    target_filter: &ObjectFilter,
) -> Result<Option<crate::effect::Restriction>, CardTextError> {
    if words.len() < 6 || !BE_TARGET_OF_PREFIX_PATTERN.accepts_words(words) {
        return Ok(None);
    }

    let Some(source_marker_start) = SPELLS_OR_ABILITIES_FROM_PATTERN
        .locate_window(&words[4..], 4)
        .map(|idx| idx + 4)
    else {
        return source_only_target_restriction(tokens, words, target_filter);
    };
    let source_words_start = source_marker_start + 4;
    if source_words_start >= words.len() {
        return Ok(None);
    }

    let source_words_end = if words
        .last()
        .is_some_and(|word| matches!(*word, "source" | "sources"))
    {
        words.len() - 1
    } else {
        words.len()
    };
    if source_words_start >= source_words_end {
        return Ok(None);
    }

    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let spell_filter = if source_marker_start > 4 {
        let spell_token_start = word_view.token_index_after_words(4).unwrap_or(tokens.len());
        let spell_token_end = word_view
            .token_index_after_words(source_marker_start)
            .unwrap_or(tokens.len());
        let spell_tokens = trim_commas(&tokens[spell_token_start..spell_token_end]);
        Some(
            match parse_object_filter(&spell_tokens, false).ok() {
                Some(filter) => Some(filter),
                None => parse_subject_object_filter(&spell_tokens)?,
            }
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported source-filtered target restriction tail (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?,
        )
    } else {
        None
    };
    let source_token_start = word_view
        .token_index_after_words(source_words_start)
        .unwrap_or(tokens.len());
    let source_token_end = word_view
        .token_index_after_words(source_words_end)
        .unwrap_or(tokens.len());
    let source_tokens = trim_commas(&tokens[source_token_start..source_token_end]);

    let source_filter = match parse_object_filter(&source_tokens, false).ok() {
        Some(filter) => Some(filter),
        None => parse_subject_object_filter(&source_tokens)?,
    }
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported source-filtered target restriction tail (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    if spell_filter
        .as_ref()
        .is_some_and(|filter| filter != &source_filter)
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported source-filtered target restriction tail (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(crate::effect::Restriction::be_targeted_from(
        target_filter.clone(),
        source_filter,
    )))
}

fn source_only_target_restriction(
    tokens: &[OwnedLexToken],
    words: &[&str],
    target_filter: &ObjectFilter,
) -> Result<Option<crate::effect::Restriction>, CardTextError> {
    if !words
        .last()
        .is_some_and(|word| matches!(*word, "spell" | "spells"))
    {
        return Ok(None);
    }

    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let source_token_start = word_view.token_index_after_words(4).unwrap_or(tokens.len());
    let source_tokens = trim_commas(&tokens[source_token_start..]);
    let descriptor_tokens = source_tokens
        .get(..source_tokens.len().saturating_sub(1))
        .unwrap_or_default();
    let mut source_filter = match parse_object_filter(&source_tokens, false).ok() {
        Some(filter) => Some(filter),
        None => parse_subject_object_filter(&source_tokens)?,
    }
    .or_else(
        || match parse_object_filter(descriptor_tokens, false).ok() {
            Some(filter) => Some(filter),
            None => parse_subject_object_filter(descriptor_tokens)
                .ok()
                .flatten(),
        },
    )
    .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported source-filtered target restriction tail (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    source_filter.zone = Some(crate::zone::Zone::Stack);
    source_filter.stack_kind = Some(crate::filter::StackObjectKind::Spell);

    Ok(Some(crate::effect::Restriction::be_targeted_from(
        target_filter.clone(),
        source_filter,
    )))
}

fn player_negated_restriction_from_tail(
    words: &[&str],
    player: PlayerFilter,
) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    if let Some(spell_filter) = parse_cast_restriction_tail_filter(words) {
        Some(Restriction::cast_spells_matching(player, spell_filter))
    } else if restriction_surface_accepts(words, PLAYER_GAIN_LIFE_TAIL_PATTERN) {
        Some(Restriction::gain_life(player))
    } else if restriction_surface_accepts(words, PLAYER_SEARCH_LIBRARIES_TAIL_PATTERN) {
        Some(Restriction::search_libraries(player))
    } else if restriction_surface_accepts(words, PLAYER_LOSE_GAME_TAIL_PATTERN) {
        Some(Restriction::lose_game(player))
    } else if restriction_surface_accepts(words, PLAYER_LOSE_LIFE_TAIL_PATTERN) {
        Some(Restriction::lose_life(player))
    } else if restriction_surface_accepts(words, PLAYER_WIN_GAME_TAIL_PATTERN) {
        Some(Restriction::win_game(player))
    } else if restriction_surface_accepts(words, PLAYER_DRAW_CARDS_TAIL_PATTERN) {
        Some(Restriction::draw_cards(player))
    } else if restriction_surface_accepts(words, PLAYER_DRAW_MORE_THAN_ONE_TAIL_PATTERN) {
        Some(Restriction::draw_extra_cards(player))
    } else if restriction_surface_accepts(words, PLAYER_GET_POISON_COUNTERS_TAIL_PATTERN) {
        Some(Restriction::poison_counters(player))
    } else if restriction_surface_accepts(
        words,
        PLAYER_CAST_MORE_THAN_ONE_SPELL_EACH_TURN_TAIL_PATTERN,
    ) {
        Some(Restriction::cast_more_than_one_spell_each_turn(player))
    } else if restriction_surface_accepts(words, CANT_CAST_SPELLS_TAIL_PATTERN) {
        Some(Restriction::cast_spells_matching(
            player,
            ObjectFilter::spell(),
        ))
    } else {
        None
    }
}

fn damage_cause_life_loss_restriction_from_tail(
    words: &[&str],
) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    if restriction_surface_accepts(words, DAMAGE_CAUSE_YOU_LOSE_LIFE_TAIL_PATTERN) {
        Some(Restriction::damage_cause_life_loss(PlayerFilter::You))
    } else if restriction_surface_accepts(words, DAMAGE_CAUSE_PLAYERS_LOSE_LIFE_TAIL_PATTERN) {
        Some(Restriction::damage_cause_life_loss(PlayerFilter::Any))
    } else if restriction_surface_accepts(words, DAMAGE_CAUSE_THAT_PLAYER_LOSE_LIFE_TAIL_PATTERN) {
        Some(Restriction::damage_cause_life_loss(
            PlayerFilter::IteratedPlayer,
        ))
    } else {
        None
    }
}

pub(crate) fn format_negated_restriction_display(tokens: &[OwnedLexToken]) -> String {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let mut out = Vec::with_capacity(words.len());
    let mut idx = 0usize;
    while idx < words.len() {
        match (words[idx], words.get(idx + 1).copied()) {
            ("cant", _) => {
                out.push("can't".to_string());
                idx += 1;
            }
            ("can", Some("not")) => {
                out.push("can't".to_string());
                idx += 2;
            }
            ("does", Some("not")) => {
                out.push("doesn't".to_string());
                idx += 2;
            }
            ("do", Some("not")) => {
                out.push("don't".to_string());
                idx += 2;
            }
            ("non", Some("phyrexian")) => {
                out.push("non-phyrexian".to_string());
                idx += 2;
            }
            ("aura", _) => {
                out.push("Aura".to_string());
                idx += 1;
            }
            _ => {
                out.push(words[idx].to_string());
                idx += 1;
            }
        }
    }
    out.join(" ")
}

pub(crate) fn parse_cant_restrictions(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<ParsedCantRestriction>>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if restriction_surface_accepts(&normalized, PLAYERS_CANT_LOSE_OR_WIN_GAME_PATTERN) {
        return Ok(Some(vec![
            ParsedCantRestriction {
                restriction: crate::effect::Restriction::lose_game(PlayerFilter::Any),
                target: None,
            },
            ParsedCantRestriction {
                restriction: crate::effect::Restriction::win_game(PlayerFilter::Any),
                target: None,
            },
        ]));
    }

    let words = crate::runtime_backend::token_word_refs(tokens);
    if is_mana_retention_negated_clause(&words) {
        return Ok(None);
    }

    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if crate::runtime_backend::lexer::contains_token_word(tokens, "and") {
        let segments = grammar::split_lexed_slices_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut restrictions = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.to_vec();
            if idx > 0
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
            let Some(restriction) = parse_cant_restriction_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant restriction segment (clause: '{}')",
                    crate::runtime_backend::token_word_refs(segment).join(" ")
                )));
            };
            let segment_words = normalize_cant_words(segment);
            let segment_word_refs = segment_words.iter().map(String::as_str).collect::<Vec<_>>();
            let has_or_win_tail =
                restriction_surface_accepts(&segment_word_refs, CANT_WIN_GAME_TAIL_PATTERN);
            if has_or_win_tail
                && let crate::effect::Restriction::LoseGame(player_filter) =
                    restriction.restriction.clone()
            {
                restrictions.push(ParsedCantRestriction {
                    restriction: crate::effect::Restriction::win_game(player_filter),
                    target: None,
                });
            }
            restrictions.push(restriction);
        }

        if restrictions.is_empty() {
            return Ok(None);
        }
        return Ok(Some(restrictions));
    }

    parse_cant_restriction_clause(tokens).map(|restriction| restriction.map(|r| vec![r]))
}

pub(crate) fn parse_cant_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let words = crate::runtime_backend::token_word_refs(tokens);
    if is_mana_retention_negated_clause(&words) {
        return Ok(None);
    }

    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && !remainder.is_empty()
        && remainder.len() < tokens.len()
    {
        return parse_cant_restriction_clause(&remainder);
    }

    if let Some(parsed) = parse_player_negated_restriction_clause(tokens)? {
        return Ok(Some(parsed));
    }

    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let restriction = if let Some(parsed) = parse_cant_cast_restriction_words(&normalized) {
        parsed
    } else {
        if restriction_surface_accepts(&normalized, OPPONENTS_CANT_BLOCK_ODD_EVEN_MV_PATTERN)
            && let Some(parity) = normalized.get(7).copied()
        {
            let parity = match parity {
                "odd" => crate::filter::ParityRequirement::Odd,
                "even" => crate::filter::ParityRequirement::Even,
                _ => return parse_negated_object_restriction_clause(tokens),
            };
            return Ok(Some(ParsedCantRestriction {
                restriction: Restriction::block(
                    ObjectFilter::creature()
                        .opponent_controls()
                        .with_mana_value_parity(parity),
                ),
                target: None,
            }));
        }
        if restriction_surface_accepts(&normalized, PLAYERS_CANT_GAIN_LIFE_PATTERN) {
            Restriction::gain_life(PlayerFilter::Any)
        } else if restriction_surface_accepts(&normalized, PLAYERS_CANT_SEARCH_LIBRARIES_PATTERN) {
            Restriction::search_libraries(PlayerFilter::Any)
        } else if restriction_surface_accepts(&normalized, PLAYERS_CANT_DRAW_CARDS_PATTERN) {
            Restriction::draw_cards(PlayerFilter::Any)
        } else if restriction_surface_accepts(&normalized, PLAYERS_CANT_DRAW_MORE_THAN_ONE_PATTERN)
        {
            Restriction::draw_extra_cards(PlayerFilter::Any)
        } else if restriction_surface_accepts(&normalized, DAMAGE_CANT_BE_PREVENTED_PATTERN) {
            Restriction::prevent_damage()
        } else if restriction_surface_accepts(&normalized, YOU_CANT_LOSE_GAME_PATTERN) {
            Restriction::lose_game(PlayerFilter::You)
        } else if restriction_surface_accepts(&normalized, OPPONENTS_CANT_WIN_GAME_PATTERN) {
            Restriction::win_game(PlayerFilter::Opponent)
        } else if restriction_surface_accepts(&normalized, YOUR_LIFE_TOTAL_CANT_CHANGE_PATTERN) {
            Restriction::change_life_total(PlayerFilter::You)
        } else if restriction_surface_accepts(
            &normalized,
            OPPONENTS_CANT_DRAW_MORE_THAN_ONE_PATTERN,
        ) {
            Restriction::draw_extra_cards(PlayerFilter::Opponent)
        } else if restriction_surface_accepts(&normalized, YOU_CANT_GAIN_LIFE_PATTERN) {
            Restriction::gain_life(PlayerFilter::You)
        } else if restriction_surface_accepts(&normalized, YOU_CANT_SEARCH_LIBRARIES_PATTERN) {
            Restriction::search_libraries(PlayerFilter::You)
        } else if restriction_surface_accepts(&normalized, YOU_CANT_DRAW_CARDS_PATTERN) {
            Restriction::draw_cards(PlayerFilter::You)
        } else if restriction_surface_accepts(&normalized, YOU_CANT_BECOME_MONARCH_PATTERN) {
            Restriction::become_monarch(PlayerFilter::You)
        } else if restriction_surface_accepts(&normalized, ITERATED_PLAYER_CANT_GAIN_LIFE_PATTERN) {
            Restriction::gain_life(PlayerFilter::IteratedPlayer)
        } else if restriction_surface_accepts(&normalized, OPPONENTS_CANT_GAIN_LIFE_PATTERN) {
            Restriction::gain_life(PlayerFilter::Opponent)
        } else {
            return parse_negated_object_restriction_clause(tokens);
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target: None,
    }))
}

fn is_mana_retention_negated_clause(words: &[&str]) -> bool {
    let Some((&"you", rest)) = words.split_first() else {
        return false;
    };
    let rest = match rest {
        ["dont", tail @ ..] | ["don't", tail @ ..] => tail,
        ["do", "not", tail @ ..] => tail,
        _ => return false,
    };
    if is_mana_retention_tail(rest) {
        return true;
    }
    match rest {
        [
            "lose",
            "this",
            "mana",
            "as",
            "steps",
            "and",
            "phases",
            "end",
        ]
        | [
            "lose",
            "unspent",
            "mana",
            "as",
            "steps",
            "and",
            "phases",
            "end",
        ]
        | [
            "lose",
            "unspent",
            "white",
            "mana",
            "as",
            "steps",
            "and",
            "phases",
            "end",
        ]
        | [
            "lose",
            "unspent",
            "blue",
            "mana",
            "as",
            "steps",
            "and",
            "phases",
            "end",
        ]
        | [
            "lose",
            "unspent",
            "black",
            "mana",
            "as",
            "steps",
            "and",
            "phases",
            "end",
        ]
        | [
            "lose",
            "unspent",
            "red",
            "mana",
            "as",
            "steps",
            "and",
            "phases",
            "end",
        ]
        | [
            "lose",
            "unspent",
            "green",
            "mana",
            "as",
            "steps",
            "and",
            "phases",
            "end",
        ]
        | ["lose", "this", "mana", "as", "steps"]
        | ["lose", "unspent", "mana", "as", "steps"]
        | ["lose", "unspent", "white", "mana", "as", "steps"]
        | ["lose", "unspent", "blue", "mana", "as", "steps"]
        | ["lose", "unspent", "black", "mana", "as", "steps"]
        | ["lose", "unspent", "red", "mana", "as", "steps"]
        | ["lose", "unspent", "green", "mana", "as", "steps"] => true,
        _ => false,
    }
}

fn is_mana_retention_tail(words: &[&str]) -> bool {
    restriction_surface_accepts(words, LOSE_UNSPENT_MANA_STEPS_PATTERN)
        || restriction_surface_accepts(words, LOSE_THIS_MANA_STEPS_PATTERN)
}

/// Parse "lose unspent [color] mana as steps [and phases end]" tails.
/// Returns `Some(color)` on a match; the inner option is the retained color
/// scope (`None` retains the whole pool).
pub(crate) fn parse_unspent_mana_retention_tail(
    words: &[&str],
) -> Option<Option<crate::color::Color>> {
    crate::runtime_backend::grammar::activation_restrictions::parse_unspent_mana_retention_tail_words(
        words,
    )
    .map(|parsed| parsed.color)
}

pub(crate) fn parse_cant_cast_restriction_words(
    words: &[&str],
) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    if let Some(spell_filter) = parse_spell_subject_cant_be_cast_filter(words) {
        return Some(Restriction::cast_spells_matching(
            PlayerFilter::Any,
            spell_filter,
        ));
    }

    if let Some((player, used)) = parse_cant_cast_subject(words) {
        let mut tail = &words[used..];
        match tail {
            [rest @ .., "during", "that", "players", "next", "turn"]
            | [rest @ .., "during", "that", "player", "s", "next", "turn"] => {
                tail = rest;
            }
            _ => {}
        }

        if let Some(spell_filter) = parse_cast_additional_limit_filter(tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }

        if !tail
            .first()
            .is_some_and(|word| activation_restriction_word_is_any(word, CANT_WORDS))
        {
            return None;
        }
        let cant_tail = &tail[1..];

        if restriction_surface_accepts(cant_tail, CANT_CAST_SPELLS_TAIL_PATTERN) {
            return Some(Restriction::cast_spells(player));
        }
        if cant_tail.len() >= 6
            && restriction_surface_accepts(cant_tail, CANT_CAST_SPELLS_WITH_PARITY_PATTERN)
        {
            let parity = cant_tail[3];
            let parity = match parity {
                "odd" => crate::filter::ParityRequirement::Odd,
                "even" => crate::filter::ParityRequirement::Even,
                _ => return None,
            };
            return Some(Restriction::cast_spells_matching(
                player,
                ObjectFilter::spell().with_mana_value_parity(parity),
            ));
        }
        if restriction_surface_accepts(cant_tail, CANT_CAST_CREATURE_SPELLS_TAIL_PATTERN) {
            return Some(Restriction::cast_creature_spells(player));
        }
        if cant_tail
            .first()
            .is_some_and(|word| activation_restriction_word_is(word, CAST_WORD))
        {
            let mut idx = 1usize;
            if let Some((spell_filter, used)) = parse_cast_limit_qualifier(&cant_tail[idx..]) {
                idx += used;
                if activation_restriction_word_at_is_any(cant_tail, idx, SPELL_OR_SPELLS_WORDS) {
                    idx += 1;
                    if restriction_surface_accepts(&cant_tail[idx..], THIS_TURN_PREFIX_PATTERN) {
                        idx += 2;
                    }
                    if idx == cant_tail.len() {
                        return Some(Restriction::cast_spells_matching(player, spell_filter));
                    }
                }
            }
        }
        if let Some(spell_filter) = parse_cast_more_than_one_limit_filter(cant_tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }
        return None;
    }

    if let Some(spell_filter) = parse_cast_additional_limit_filter(words) {
        return Some(restriction_from_cast_limit_filter(
            PlayerFilter::Any,
            spell_filter,
        ));
    }

    None
}

fn parse_spell_subject_cant_be_cast_filter(words: &[&str]) -> Option<ObjectFilter> {
    if !restriction_surface_accepts(words, SPELL_CANT_BE_CAST_SUFFIX_PATTERN) || words.len() <= 3 {
        return None;
    }
    parse_spell_restriction_subject_filter(&words[..words.len() - 3])
}

fn parse_spell_restriction_subject_filter(words: &[&str]) -> Option<ObjectFilter> {
    let mut filter = ObjectFilter::spell();
    let mut idx = 0usize;

    if activation_restriction_word_at_is(words, idx, NONCREATURE_WORD) {
        filter = filter.without_type(CardType::Creature);
        idx += 1;
    } else if restriction_surface_accepts(&words[idx..], NON_CREATURE_PREFIX_PATTERN) {
        filter = filter.without_type(CardType::Creature);
        idx += 2;
    } else if !activation_restriction_word_at_is_any(words, idx, SPELL_OR_SPELLS_WORDS) {
        let term = words.get(idx).copied()?;
        if let Some(card_type) = parse_card_type(term.trim_end_matches('s')) {
            filter = filter.with_type(card_type);
            idx += 1;
        } else if let Some(subtype) = parse_subtype_word(term.trim_end_matches('s')) {
            filter = filter.with_subtype(subtype);
            idx += 1;
        }
    }

    if !activation_restriction_word_at_is_any(words, idx, SPELL_OR_SPELLS_WORDS) {
        return None;
    }
    idx += 1;

    while idx < words.len() {
        if !activation_restriction_word_at_is(words, idx, WITH_WORD) {
            return None;
        }
        idx += 1;

        if restriction_surface_accepts(&words[idx..], MANA_VALUE_PREFIX_PATTERN) {
            let value = words.get(idx + 2)?.parse::<i32>().ok()?;
            let comparison = match (words.get(idx + 3).copied(), words.get(idx + 4).copied()) {
                (Some("or"), Some("greater")) => {
                    idx += 5;
                    crate::filter::Comparison::GreaterThanOrEqual(value)
                }
                (Some("or"), Some("less")) => {
                    idx += 5;
                    crate::filter::Comparison::LessThanOrEqual(value)
                }
                (None, None) => {
                    idx += 3;
                    crate::filter::Comparison::Equal(value)
                }
                _ => return None,
            };
            filter = filter.with_mana_value(comparison);
            continue;
        }

        if restriction_surface_accepts(&words[idx..], X_IN_MANA_COST_PREFIX_PATTERN) {
            filter.has_x_in_cost = true;
            idx += 5;
            continue;
        }

        match &words[idx..] {
            ["the", "chosen", "name", rest @ ..] | ["chosen", "name", rest @ ..] => {
                filter.name = Some("{chosen name}".to_string());
                idx = words.len() - rest.len();
                continue;
            }
            _ => {}
        }

        return None;
    }

    Some(filter)
}

pub(crate) fn parse_cant_cast_subject(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    if restriction_surface_accepts(words, PLAYERS_DEALT_DAMAGE_THIS_WAY_SUBJECT_PATTERN) {
        return Some((PlayerFilter::TaggedPlayer(TagKey::from("damaged_0")), 5));
    }
    if restriction_surface_accepts(words, THAT_PLAYER_SUBJECT_PATTERN) {
        return Some((PlayerFilter::IteratedPlayer, 2));
    }
    if restriction_surface_accepts(words, YOUR_OPPONENTS_WHO_HAVE_SUBJECT_PATTERN) {
        return Some((PlayerFilter::Opponent, 4));
    }
    if restriction_surface_accepts(words, EACH_PLAYER_WHO_HAS_SUBJECT_PATTERN) {
        return Some((PlayerFilter::Any, 4));
    }
    if restriction_surface_accepts(words, EACH_OPPONENT_WHO_HAS_SUBJECT_PATTERN) {
        return Some((PlayerFilter::Opponent, 4));
    }
    if restriction_surface_accepts(words, YOUR_OPPONENTS_SUBJECT_PATTERN) {
        return Some((PlayerFilter::Opponent, 2));
    }
    if restriction_surface_accepts(words, EACH_PLAYER_SUBJECT_PATTERN) {
        return Some((PlayerFilter::Any, 2));
    }
    if restriction_surface_accepts(words, EACH_OPPONENT_SUBJECT_PATTERN) {
        return Some((PlayerFilter::Opponent, 2));
    }
    match words.first().copied() {
        Some("players") => Some((PlayerFilter::Any, 1)),
        Some("opponents") => Some((PlayerFilter::Opponent, 1)),
        Some("they") => Some((PlayerFilter::IteratedPlayer, 1)),
        Some("you") => Some((PlayerFilter::You, 1)),
        _ => None,
    }
}

pub(crate) fn parse_cast_more_than_one_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    if !restriction_surface_accepts(words, CAST_MORE_THAN_ONE_PREFIX_PATTERN) {
        return None;
    }
    let mut idx = 4usize;
    let (spell_filter, consumed) = if activation_restriction_word_at_is(words, idx, SPELL_WORD) {
        (ObjectFilter::default(), 0usize)
    } else {
        parse_cast_limit_qualifier(&words[idx..])?
    };
    idx += consumed;

    if !restriction_surface_accepts(&words[idx..], SPELL_EACH_TURN_TAIL_PATTERN)
        || idx + 3 != words.len()
    {
        return None;
    }

    Some(spell_filter)
}

pub(crate) fn parse_cast_additional_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    let mut idx = 0usize;
    if restriction_surface_accepts(words, WHO_HAS_PREFIX_PATTERN) {
        idx += 2;
    }

    if !activation_restriction_word_at_is(words, idx, CAST_WORD) {
        return None;
    }
    idx += 1;
    if activation_restriction_word_at_is_any(words, idx, ARTICLE_WORDS) {
        idx += 1;
    }

    let (first_filter, first_used) = parse_cast_limit_qualifier(&words[idx..])?;
    idx += first_used;

    if !activation_restriction_word_at_is(words, idx, SPELL_WORD) {
        return None;
    }
    idx += 1;

    if restriction_surface_accepts(&words[idx..], THIS_TURN_PREFIX_PATTERN) {
        idx += 2;
    }

    if !restriction_surface_accepts(&words[idx..], CANT_CAST_ADDITIONAL_PREFIX_PATTERN) {
        return None;
    }
    idx += 3;

    let (second_filter, second_used) = parse_cast_limit_qualifier(&words[idx..])?;
    if second_filter != first_filter {
        return None;
    }
    idx += second_used;

    if !activation_restriction_word_at_is(words, idx, SPELLS_WORD) || idx + 1 != words.len() {
        return None;
    }

    Some(first_filter)
}

pub(crate) fn parse_cast_limit_qualifier(words: &[&str]) -> Option<(ObjectFilter, usize)> {
    crate::runtime_backend::grammar::activation_restrictions::parse_activation_cast_limit_qualifier_words(words)
        .map(|parsed| (parsed.filter, parsed.consumed))
}

pub(crate) fn strip_static_restriction_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::ConditionExpr, Vec<OwnedLexToken>)>, CardTextError> {
    use crate::runtime_backend::grammar::activation_restrictions::{
        StaticRestrictionConditionKind, StaticRestrictionConditionShape,
        parse_source_attached_to_creature_condition_tokens,
        parse_static_restriction_condition_shape_tokens,
    };

    let Some(shape) = parse_static_restriction_condition_shape_tokens(tokens) else {
        return Ok(None);
    };
    match shape {
        StaticRestrictionConditionShape::Timing {
            timing,
            remainder_first,
            remainder_end,
        } => Ok(Some((
            crate::ConditionExpr::ActivationTiming(timing),
            trim_commas(&tokens[remainder_first..remainder_end]).to_vec(),
        ))),
        StaticRestrictionConditionShape::Condition {
            kind,
            condition,
            remainder_first,
        } => {
            let condition_tokens = trim_commas(&tokens[condition.first..condition.end]);
            let condition = match parse_static_condition_clause(&condition_tokens) {
                Ok(condition) => condition,
                Err(_) if kind == StaticRestrictionConditionKind::If => return Ok(None),
                Err(_) if parse_source_attached_to_creature_condition_tokens(&condition_tokens) => {
                    crate::ConditionExpr::SourceIsEquipped
                }
                Err(_) => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported static condition clause (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
            };
            Ok(Some((
                condition,
                trim_commas(&tokens[remainder_first..]).to_vec(),
            )))
        }
    }
}

pub(crate) fn parse_player_negated_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let Some((player, target)) = parse_player_restriction_subject(&subject_tokens)? else {
        return Ok(None);
    };
    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Ok(None);
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if let Some(spell_filter) = parse_cast_restriction_tail_filter(&remainder_words) {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::cast_spells_matching(player, spell_filter),
            target,
        }));
    }
    if restriction_surface_accepts(&remainder_words, CANT_CAST_SPELLS_TAIL_PATTERN) {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::cast_spells(player),
            target,
        }));
    }
    if restriction_surface_accepts(
        &remainder_words,
        ACTIVATE_ABILITIES_THAT_ARENT_MANA_TAIL_PATTERN,
    ) {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::activate_non_mana_abilities(player),
            target,
        }));
    }
    if restriction_surface_accepts(&remainder_words, ACTIVATE_ABILITIES_OF_PREFIX_PATTERN) {
        let Some(mut filter) =
            parse_card_type_list_filter(&remainder_words[3..], Some(Zone::Battlefield))
        else {
            return Ok(None);
        };
        filter.controller = Some(player);
        let restriction = if restriction_surface_accepts(
            &remainder_words,
            UNLESS_MANA_ABILITIES_SUFFIX_PATTERN,
        ) {
            Restriction::activate_non_mana_abilities_of(filter)
        } else {
            Restriction::activate_abilities_of(filter)
        };
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target,
        }));
    }

    Ok(None)
}

pub(crate) fn parse_player_restriction_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerFilter, Option<TargetAst>)>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    if starts_with_target_indicator(subject_tokens) {
        let target = parse_target_phrase(subject_tokens)?;
        if let TargetAst::Player(player, span) = &target {
            return Ok(Some((
                target_ast_player_filter(player.clone(), span.clone()),
                Some(target),
            )));
        }
        return Ok(None);
    }

    let normalized_storage = normalize_cant_words(subject_tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if restriction_surface_accepts(&normalized, YOU_PLAYER_SUBJECT_PATTERN) {
        return Ok(Some((PlayerFilter::You, None)));
    }
    if restriction_surface_accepts(&normalized, ITERATED_PLAYER_SUBJECT_PATTERN) {
        return Ok(Some((PlayerFilter::IteratedPlayer, None)));
    }
    if restriction_surface_accepts(&normalized, DAMAGED_PLAYER_SUBJECT_PATTERN) {
        return Ok(Some((
            PlayerFilter::TaggedPlayer(TagKey::from("damaged_0")),
            None,
        )));
    }
    if restriction_surface_accepts(&normalized, OPPONENT_PLAYER_SUBJECT_PATTERN) {
        return Ok(Some((PlayerFilter::Opponent, None)));
    }
    if restriction_surface_accepts(&normalized, ANY_PLAYER_SUBJECT_PATTERN) {
        return Ok(Some((PlayerFilter::Any, None)));
    }
    if restriction_surface_accepts(&normalized, DEFENDING_PLAYER_SUBJECT_PATTERN) {
        return Ok(Some((PlayerFilter::Defending, None)));
    }
    if restriction_surface_accepts(&normalized, ATTACKING_PLAYER_SUBJECT_PATTERN) {
        return Ok(Some((PlayerFilter::Attacking, None)));
    }
    if restriction_surface_accepts(&normalized, CONTROLLER_OF_IT_SUBJECT_PATTERN) {
        return Ok(Some((
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG))),
            None,
        )));
    }
    if restriction_surface_accepts(&normalized, OWNER_OF_IT_SUBJECT_PATTERN) {
        return Ok(Some((
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG))),
            None,
        )));
    }

    let player = match parse_subject(subject_tokens) {
        crate::cards::builders::SubjectAst::Player(PlayerAst::You | PlayerAst::Implicit) => {
            PlayerFilter::You
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Opponent) => PlayerFilter::Opponent,
        crate::cards::builders::SubjectAst::Player(PlayerAst::That) => PlayerFilter::IteratedPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Defending) => PlayerFilter::Defending,
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsController) => {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsOwner) => {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Chosen) => PlayerFilter::ChosenPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Attacking) => PlayerFilter::Attacking,
        crate::cards::builders::SubjectAst::Player(PlayerAst::MostLifeTied) => {
            PlayerFilter::MostLifeTied
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::LowestLifeTied) => {
            PlayerFilter::LowestLifeTied
        }
        _ => return Ok(None),
    };
    Ok(Some((player, None)))
}

pub(crate) fn target_ast_player_filter(
    player: PlayerFilter,
    span: Option<TextSpan>,
) -> PlayerFilter {
    if span.is_some() {
        match player {
            PlayerFilter::Any => PlayerFilter::target_player(),
            PlayerFilter::Opponent => PlayerFilter::target_opponent(),
            other => other,
        }
    } else {
        player
    }
}

pub(crate) fn parse_cast_restriction_tail_filter(words: &[&str]) -> Option<ObjectFilter> {
    if words
        .first()
        .is_some_and(|word| activation_restriction_word_is(word, CAST_WORD))
        && let Some(mut filter) = parse_spell_restriction_subject_filter(&words[1..])
    {
        filter.zone = None;
        filter.stack_kind = None;
        return Some(filter);
    }
    if restriction_surface_accepts(words, CAST_SPELLS_PATTERN) {
        return Some(ObjectFilter::default());
    }
    if restriction_surface_accepts(words, CANT_CAST_SPELLS_OF_CHOSEN_TYPE_TAIL_PATTERN) {
        return Some(ObjectFilter::default().of_chosen_card_type());
    }
    if words.first() != Some(&"cast") || words.last() != Some(&"spells") || words.len() < 3 {
        return None;
    }
    let tail = &words[1..words.len() - 1];
    let (filter, used) = parse_cast_limit_qualifier(tail)?;
    (used == tail.len()).then_some(filter)
}

pub(crate) fn parse_card_type_list_filter(
    words: &[&str],
    zone: Option<Zone>,
) -> Option<ObjectFilter> {
    let cleaned = words
        .iter()
        .copied()
        .filter(|word| !restriction_surface_accepts(&[*word], CARD_TYPE_LIST_IGNORED_WORD_PATTERN))
        .collect::<Vec<_>>();
    if cleaned.is_empty() {
        return None;
    }

    let mut filters = Vec::new();
    for word in cleaned {
        let normalized = word.trim_end_matches('s');
        let card_type = parse_card_type(normalized)?;
        let mut filter = ObjectFilter::default();
        filter.zone = zone;
        filter.card_types.push(card_type);
        filters.push(filter);
    }
    if filters.len() == 1 {
        return filters.pop();
    }
    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = filters;
    Some(disjunction)
}

fn parse_and_or_disjunction_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let mut separator_indices = Vec::new();
    let mut idx = 0usize;
    while idx < tokens.len() {
        if activation_restriction_token_word_is(&tokens[idx], AND_OR_WORD) {
            separator_indices.push((idx, idx + 1));
            idx += 1;
            continue;
        }
        let tail_words = crate::runtime_backend::token_word_refs(&tokens[idx..]);
        if restriction_surface_accepts(&tail_words, AND_OR_PHRASE_PATTERN) {
            separator_indices.push((idx, idx + 2));
            idx += 2;
            continue;
        }
        idx += 1;
    }
    if separator_indices.is_empty() {
        return Ok(None);
    }

    let mut segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    let mut start = 0usize;
    for (separator_start, separator_end) in separator_indices {
        let segment = trim_commas(&tokens[start..separator_start]);
        if !segment.is_empty() {
            segments.push(segment.to_vec());
        }
        start = separator_end;
    }
    let tail = trim_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail.to_vec());
    }

    if segments.len() < 2 {
        return Ok(None);
    }

    let mut filters = Vec::with_capacity(segments.len());
    for segment in segments {
        let Some(filter) = parse_subject_object_filter(&segment)?
            .or_else(|| parse_object_filter(&segment, false).ok())
        else {
            return Ok(None);
        };
        filters.push(filter);
    }

    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = filters;
    Ok(Some(disjunction))
}

fn invert_except_by_blocker_filter(allowed: &ObjectFilter) -> Option<ObjectFilter> {
    let clauses: Vec<&ObjectFilter> = if allowed.any_of.is_empty() {
        vec![allowed]
    } else {
        allowed.any_of.iter().collect()
    };
    if clauses.is_empty() {
        return None;
    }

    let mut disallowed = ObjectFilter::creature();
    for clause in clauses {
        if !clause.any_of.is_empty() {
            return None;
        }

        if !clause.card_types.is_empty()
            && !clause.card_types.contains(&CardType::Creature)
            && clause.card_types.len() == 1
        {
            disallowed = disallowed.without_type(clause.card_types[0]);
        }

        for subtype in &clause.subtypes {
            disallowed = disallowed.without_subtype(*subtype);
        }
        for ability in &clause.static_abilities {
            disallowed = disallowed.without_static_ability(*ability);
        }
        if let Some(colors) = clause.colors {
            disallowed = disallowed.without_colors(colors);
        }
    }

    Some(disallowed)
}

pub(crate) fn restriction_from_cast_limit_filter(
    player: PlayerFilter,
    spell_filter: ObjectFilter,
) -> crate::effect::Restriction {
    crate::effect::Restriction::cast_more_than_one_spell_each_turn_matching(player, spell_filter)
}

pub(crate) fn parse_negated_object_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let words = crate::runtime_backend::token_word_refs(tokens);
    if restriction_surface_accepts(&words, DONT_LOSE_THIS_MANA_STEPS_PATTERN) {
        return Ok(None);
    }

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let subject_words_storage = normalize_cant_words(&subject_tokens);
    let subject_words = subject_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let bare_other_choice =
        crate::runtime_backend::grammar::choices::parse_chosen_cant_block_shape(tokens)
            .ok()
            .flatten()
            .is_some_and(|shape| shape.bare_other_reference);
    if subject_words
        .first()
        .is_some_and(|word| activation_restriction_word_is(word, "if"))
    {
        return Ok(None);
    }

    let (mut filter, mut target, ability_scope) =
        if let Some(parsed) = parse_activated_ability_subject(&subject_tokens)? {
            (parsed.filter, parsed.target, Some(parsed.scope))
        } else if starts_with_target_indicator(&subject_tokens) {
            let target = parse_target_phrase(&subject_tokens)?;
            let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported target restriction subject (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
            ensure_it_tagged_constraint(&mut filter);
            (filter, Some(target), None)
        } else if subject_tokens.is_empty() {
            // Supports carried clauses like "... and can't be blocked this turn."
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
            (
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                Some(target),
                None,
            )
        } else if bare_other_choice {
            (
                ObjectFilter::creature().not_tagged(TagKey::from(IT_TAG)),
                None,
                None,
            )
        } else if restriction_surface_accepts(&subject_words, PLAYER_RESTRICTION_SUBJECT_PATTERN) {
            (ObjectFilter::default(), None, None)
        } else {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported subject in negated restriction clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            };
            (filter, None, None)
        };
    if restriction_surface_accepts(&words, DEALT_DAMAGE_THIS_WAY_PATTERN)
        && !filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == DAMAGED_THIS_WAY_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(DAMAGED_THIS_WAY_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing restriction tail in negated restriction clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let player_subject = player_negated_restriction_subject(&subject_words);
    if let Some(player) = player_subject {
        if let Some(color) = parse_unspent_mana_retention_tail(&remainder_words) {
            return Ok(Some(ParsedCantRestriction {
                restriction: Restriction::lose_unspent_mana(player, color),
                target: None,
            }));
        }
        if is_mana_retention_tail(&remainder_words) {
            return Ok(None);
        }
        let Some(restriction) = player_negated_restriction_from_tail(&remainder_words, player)
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player negated restriction tail (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        };
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target: None,
        }));
    }

    if subject_tokens.is_empty() && is_supported_untap_restriction_tail(&remainder_words) {
        filter = ObjectFilter::source();
        target = None;
    }

    if restriction_surface_accepts(&subject_words, DAMAGE_RESTRICTION_SUBJECT_PATTERN)
        && restriction_surface_accepts(&remainder_words, BE_PREVENTED_TAIL_PATTERN)
    {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::prevent_damage(),
            target: None,
        }));
    }
    if restriction_surface_accepts(&subject_words, DAMAGE_RESTRICTION_SUBJECT_PATTERN)
        && let Some(restriction) = damage_cause_life_loss_restriction_from_tail(&remainder_words)
    {
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target: None,
        }));
    }
    if let Some(restriction) = simple_negated_object_restriction(&remainder_words, &filter) {
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target,
        }));
    }
    if let Some(restriction) =
        source_filtered_target_restriction(&remainder_tokens, &remainder_words, &filter)?
    {
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target,
        }));
    }

    let restriction = if restriction_surface_accepts(
        &remainder_words,
        ATTACK_YOU_OR_PLANESWALKERS_YOU_CONTROL_TAIL_PATTERN,
    ) {
        Restriction::attack_player_or_planeswalkers_controlled_by(filter, PlayerFilter::You)
    } else if restriction_surface_accepts(&remainder_words, BE_BLOCKED_EXCEPT_BY_PREFIX_PATTERN)
        && remainder_words.len() > 4
    {
        let blocker_tokens = trim_commas(&remainder_tokens[4..]);
        let allowed_blocker_filter = parse_subject_object_filter(&blocker_tokens)?
            .or_else(|| parse_object_filter(&blocker_tokens, false).ok())
            .or(parse_and_or_disjunction_filter(&blocker_tokens)?)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        let blocker_filter =
            invert_except_by_blocker_filter(&allowed_blocker_filter).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported except-by blocker filter (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        Restriction::block_specific_attacker(blocker_filter, filter)
    } else if restriction_surface_accepts(&remainder_words, BE_BLOCKED_BY_PREFIX_PATTERN)
        && remainder_words.len() > 3
    {
        let blocker_tokens = trim_commas(&remainder_tokens[3..]);
        let blocker_filter = parse_subject_object_filter(&blocker_tokens)?
            .or_else(|| parse_object_filter(&blocker_tokens, false).ok())
            .or(parse_and_or_disjunction_filter(&blocker_tokens)?)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        Restriction::block_specific_attacker(blocker_filter, filter)
    } else if restriction_surface_accepts(&remainder_words, BE_ACTIVATED_TAIL_PATTERN) {
        match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) => {
                Restriction::activate_tap_abilities_of(filter)
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
        }
    } else if restriction_surface_accepts(
        &remainder_words,
        BE_ACTIVATED_UNLESS_MANA_ABILITIES_TAIL_PATTERN,
    ) {
        match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_non_mana_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) | None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                )));
            }
        }
    } else if remainder_words
        .first()
        .is_some_and(|word| activation_restriction_word_is(word, BLOCK_WORD))
        && remainder_words.len() > 1
    {
        let attacker_tokens = trim_commas(&remainder_tokens[1..]);
        let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
            .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
            .or(parse_and_or_disjunction_filter(&attacker_tokens)?)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        Restriction::block_specific_attacker(filter, attacker_filter)
    } else if is_supported_untap_restriction_tail(&remainder_words) {
        Restriction::untap(filter)
    } else {
        if remainder_words.first().is_some_and(|word| {
            activation_restriction_word_is_any(word, EFFECT_ACTION_RESTRICTION_TAIL_HEAD_WORDS)
        }) {
            return Ok(None);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported negated restriction tail (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivatedAbilityScope {
    All,
    TapCostOnly,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedActivatedAbilitySubject {
    filter: ObjectFilter,
    target: Option<TargetAst>,
    scope: ActivatedAbilityScope,
}

pub(crate) fn strip_trailing_possessive_token(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    crate::runtime_backend::grammar::activation_restrictions::parse_activation_possessive_owner_tokens(tokens)
}

pub(crate) fn parse_activated_ability_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedActivatedAbilitySubject>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let subject_words_storage = normalize_cant_words(tokens);
    let subject_words = subject_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let owner_tokens = if restriction_surface_accepts(
        &subject_words,
        ACTIVATED_ABILITIES_OWNER_SUFFIX_PATTERN,
    ) {
        let owner_word_len = subject_words.len().saturating_sub(2);
        if owner_word_len == 0 {
            return Ok(None);
        }
        let owner_end = word_view
            .token_index_after_words(owner_word_len)
            .unwrap_or(tokens.len());
        trim_commas(&tokens[..owner_end])
    } else if restriction_surface_accepts(
        &subject_words,
        ACTIVATED_ABILITIES_TAP_COST_OWNER_SUFFIX_PATTERN,
    ) {
        let owner_word_len = subject_words.len().saturating_sub(7);
        if owner_word_len == 0 {
            return Ok(None);
        }
        let owner_end = word_view
            .token_index_after_words(owner_word_len)
            .unwrap_or(tokens.len());
        trim_commas(&tokens[..owner_end])
    } else if restriction_surface_accepts(
        &subject_words,
        ACTIVATED_ABILITIES_OF_OWNER_PREFIX_PATTERN,
    ) {
        let Some(owner_start) = crate::runtime_backend::grammar::activation_restrictions::parse_activation_word_span_tokens(tokens, 3, 4).map(|span| span.first) else {
            return Ok(None);
        };
        trim_commas(&tokens[owner_start..])
    } else if restriction_surface_accepts(
        &subject_words,
        ACTIVATED_ABILITIES_TAP_COST_OF_OWNER_PREFIX_PATTERN,
    ) {
        let Some(owner_start) = crate::runtime_backend::grammar::activation_restrictions::parse_activation_word_span_tokens(tokens, 8, 9).map(|span| span.first) else {
            return Ok(None);
        };
        trim_commas(&tokens[owner_start..])
    } else {
        return Ok(None);
    };

    let scope = if restriction_surface_accepts(
        &subject_words,
        ACTIVATED_ABILITIES_TAP_COST_OWNER_SUFFIX_PATTERN,
    ) || restriction_surface_accepts(
        &subject_words,
        ACTIVATED_ABILITIES_TAP_COST_OF_OWNER_PREFIX_PATTERN,
    ) {
        ActivatedAbilityScope::TapCostOnly
    } else {
        ActivatedAbilityScope::All
    };

    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let normalized_owner_tokens = strip_trailing_possessive_token(&owner_tokens);

    let owner_word_view = ActivationRestrictionCompatWords::new(&normalized_owner_tokens);
    let owner_words = owner_word_view.to_word_refs();
    if restriction_surface_accepts(&owner_words, IT_OWNER_REFERENCE_PATTERN) {
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            target: Some(TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(tokens),
            )),
            scope,
        }));
    }

    if starts_with_target_indicator(&normalized_owner_tokens) {
        let target = parse_target_phrase(&normalized_owner_tokens)?;
        let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported target restriction subject (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;
        ensure_it_tagged_constraint(&mut filter);
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter,
            target: Some(target),
            scope,
        }));
    }

    let Some(filter) = parse_subject_object_filter(&normalized_owner_tokens)?
        .or_else(|| parse_object_filter(&normalized_owner_tokens, false).ok())
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject in negated restriction clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    Ok(Some(ParsedActivatedAbilitySubject {
        filter,
        target: None,
        scope,
    }))
}

pub(crate) fn ensure_it_tagged_constraint(filter: &mut ObjectFilter) {
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

pub(crate) fn starts_with_possessive_activated_ability_subject(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    restriction_surface_accepts(&words, POSSESSIVE_ACTIVATED_ABILITIES_PREFIX_PATTERN)
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedCantRestriction {
    pub(crate) restriction: crate::effect::Restriction,
    pub(crate) target: Option<TargetAst>,
}

pub(crate) fn starts_with_target_indicator(tokens: &[OwnedLexToken]) -> bool {
    let mut idx = 0usize;
    if token_slice_at_is(tokens, idx, "any")
        && token_slice_at_is(tokens, idx + 1, "number")
        && token_slice_at_is(tokens, idx + 2, "of")
    {
        idx += 3;
    }

    if token_slice_at_is(tokens, idx, "up") && token_slice_at_is(tokens, idx + 1, "to") {
        if let Some((_, used)) = parse_choice_count_token_prefix_consumed(&tokens[idx..]) {
            idx += used;
        }
    } else if let Some((_, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        idx += used;
    } else if let Some((_, used)) = parse_number(&tokens[idx..])
        && token_slice_at_is(tokens, idx + used, "target")
    {
        idx += used;
    } else if token_slice_at_is(tokens, idx, "x") && token_slice_at_is(tokens, idx + 1, "target") {
        idx += 1;
    }

    if token_slice_at_is(tokens, idx, "on") {
        idx += 1;
    }

    if token_slice_at_is(tokens, idx, "another") {
        idx += 1;
    }

    token_slice_at_is(tokens, idx, "target")
}

pub(crate) fn find_negation_span(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    crate::runtime_backend::grammar::activation_restrictions::parse_activation_negation_span_tokens(
        tokens,
    )
    .map(|span| (span.first, span.end))
}

pub(crate) fn parse_subject_object_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if restriction_surface_accepts(&normalized_words, DAMAGE_RESTRICTION_SUBJECT_PATTERN) {
        return Ok(Some(ObjectFilter::default()));
    }
    if restriction_surface_accepts(&normalized_words, TAGGED_OBJECT_PRONOUN_SUBJECT_PATTERN) {
        return Ok(Some(ObjectFilter::tagged(TagKey::from(IT_TAG))));
    }

    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if find_window_by(&words_all, 3, |window| {
        restriction_surface_accepts(window, POWER_OR_TOUGHNESS_SUBJECT_PATTERN)
    })
    .is_some()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject object filter (clause: '{}')",
            words_all.join(" ")
        )));
    }

    if let Ok(filter) = parse_object_filter(tokens, false)
        && filter != ObjectFilter::default()
    {
        return Ok(Some(filter));
    }

    let target = parse_target_phrase(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject target phrase (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;

    Ok(target_ast_to_object_filter(target))
}

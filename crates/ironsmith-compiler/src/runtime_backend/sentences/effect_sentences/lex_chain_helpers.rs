#![allow(dead_code)]

use super::super::clause_support::parse_ability_line_lexed;
use super::super::grammar::primitives as grammar;
use super::super::lexer::{
    OwnedLexToken, TokenKind, contains_token_kind, token_slice_at_is_any, token_slice_first_is,
    token_word_refs, trim_lexed_commas, word_slice_at_is, word_slice_contains_any_phrase,
    word_slice_contains_phrase, word_slice_contains_word, word_slice_ends_with,
    word_slice_ends_with_any, word_slice_eq_any, word_slice_first_is, word_slice_first_is_any,
    word_slice_starts_with, word_slice_starts_with_any,
};
use super::super::util::{parse_zone_word, trim_commas};
use super::chain_carry::{Verb, find_verb};
use super::clause_pattern_helpers::{
    parse_can_attack_as_though_no_defender_clause, parse_prevent_all_damage_clause,
    parse_prevent_next_damage_clause,
};
use super::clause_primitives::{
    parse_attack_or_block_this_turn_if_able_clause, parse_attack_this_turn_if_able_clause,
    parse_must_block_if_able_clause,
};

const EACH_PLAYER_OR_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["each", "player"],
    &["each", "players"],
    &["each", "opponent"],
    &["each", "opponents"],
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
];
const CAN_WORD: &str = "can";
const EXCHANGE_WORD: &str = "exchange";
const PREVENT_WORD: &str = "prevent";
const THE_WORD: &str = "the";
const NEXT_WORD: &str = "next";
const DAMAGE_WORD: &str = "damage";
const TOUGHNESS_WORD: &str = "toughness";
const UNTIL_WORD: &str = "until";
const UNLESS_WORD: &str = "unless";
const OF_WORD: &str = "of";
const OR_WORD: &str = "or";
const AND_WORD: &str = "and";
const THEN_WORD: &str = "then";
const TARGET_WORD: &str = "target";
const EXILE_WORD: &str = "exile";
const GRAVEYARD_WORDS: &[&str] = &["graveyard", "graveyards"];
const THAT_PLAYER_POSSESSIVE_GRAVEYARD_PREFIXES: &[&[&str]] = &[
    &["exile", "that", "player", "graveyard"],
    &["exile", "that", "players", "graveyard"],
    &["exile", "that", "player's", "graveyard"],
];

const ATTACK_OR_ATTACKS_WORDS: &[&str] = &["attack", "attacks"];
const BLOCK_OR_BLOCKS_WORDS: &[&str] = &["block", "blocks"];
const UNTIL_OR_DURING_WORDS: &[&str] = &["until", "during"];
const TARGET_CARD_TYPE_WORDS: &[&str] = &[
    "artifact",
    "battle",
    "creature",
    "enchantment",
    "instant",
    "land",
    "planeswalker",
    "sorcery",
];
const ARTICLE_OR_QUANTIFIER_WORDS: &[&str] = &["a", "an", "the", "all", "each"];
const SHARED_SUBJECT_MODIFIER_WORDS: &[&str] = &["get", "gets", "become", "becomes"];
const GAIN_HAVE_LOSE_WORDS: &[&str] = &["gain", "gains", "has", "have", "lose", "loses"];
struct VerbShapeEntry {
    words: &'static [&'static str],
    verb: Verb,
}

const END_WORDS: &[&str] = &["end", "ends"];
const TURN_OR_COMBAT_WORDS: &[&str] = &["turn", "combat"];
const COUNTER_OR_COUNTERS_WORDS: &[&str] = &["counter", "counters"];
const COUNTER_NOUN_CONTEXT_WORDS: &[&str] = &["on", "from", "among"];
const BACKREF_COUNTER_TARGET_PHRASES: &[&[&str]] = &[&["on", "it"], &["on", "them"]];
const RETURN_BATTLEFIELD_ATTACHED_BACKREF_PHRASES: &[&[&str]] = &[
    &["attached", "to", "it"],
    &["attached", "to", "them"],
    &["attached", "to", "that", "card"],
    &["attached", "to", "that", "creature"],
    &["attached", "to", "that", "object"],
    &["attached", "to", "that", "permanent"],
    &["attached", "to", "those", "cards"],
    &["attached", "to", "those", "creatures"],
    &["attached", "to", "those", "objects"],
    &["attached", "to", "those", "permanents"],
];
const AT_THE_PHRASE: &[&str] = &["at", "the"];
const VERB_SHAPES: &[VerbShapeEntry] = &[
    VerbShapeEntry {
        words: &["adds", "add"],
        verb: Verb::Add,
    },
    VerbShapeEntry {
        words: &["moves", "move"],
        verb: Verb::Move,
    },
    VerbShapeEntry {
        words: &["deals", "deal"],
        verb: Verb::Deal,
    },
    VerbShapeEntry {
        words: &["draws", "draw"],
        verb: Verb::Draw,
    },
    VerbShapeEntry {
        words: &["counters", "counter"],
        verb: Verb::Counter,
    },
    VerbShapeEntry {
        words: &["destroys", "destroy"],
        verb: Verb::Destroy,
    },
    VerbShapeEntry {
        words: &["exiles", "exile"],
        verb: Verb::Exile,
    },
    VerbShapeEntry {
        words: &["reveals", "reveal"],
        verb: Verb::Reveal,
    },
    VerbShapeEntry {
        words: &["looks", "look"],
        verb: Verb::Look,
    },
    VerbShapeEntry {
        words: &["loses", "lose"],
        verb: Verb::Lose,
    },
    VerbShapeEntry {
        words: &["gains", "gain"],
        verb: Verb::Gain,
    },
    VerbShapeEntry {
        words: &["puts", "put"],
        verb: Verb::Put,
    },
    VerbShapeEntry {
        words: &["sacrifices", "sacrifice"],
        verb: Verb::Sacrifice,
    },
    VerbShapeEntry {
        words: &["creates", "create"],
        verb: Verb::Create,
    },
    VerbShapeEntry {
        words: &["investigates", "investigate"],
        verb: Verb::Investigate,
    },
    VerbShapeEntry {
        words: &["proliferates", "proliferate"],
        verb: Verb::Proliferate,
    },
    VerbShapeEntry {
        words: &["taps", "tap"],
        verb: Verb::Tap,
    },
    VerbShapeEntry {
        words: &["unattaches", "unattach"],
        verb: Verb::Unattach,
    },
    VerbShapeEntry {
        words: &["attaches", "attach"],
        verb: Verb::Attach,
    },
    VerbShapeEntry {
        words: &["untaps", "untap"],
        verb: Verb::Untap,
    },
    VerbShapeEntry {
        words: &["scries", "scry"],
        verb: Verb::Scry,
    },
    VerbShapeEntry {
        words: &["discards", "discard"],
        verb: Verb::Discard,
    },
    VerbShapeEntry {
        words: &["transforms", "transform"],
        verb: Verb::Transform,
    },
    VerbShapeEntry {
        words: &["converts", "convert"],
        verb: Verb::Convert,
    },
    VerbShapeEntry {
        words: &["flips", "flip"],
        verb: Verb::Flip,
    },
    VerbShapeEntry {
        words: &["rolls", "roll"],
        verb: Verb::Roll,
    },
    VerbShapeEntry {
        words: &["regenerates", "regenerate"],
        verb: Verb::Regenerate,
    },
    VerbShapeEntry {
        words: &["mills", "mill"],
        verb: Verb::Mill,
    },
    VerbShapeEntry {
        words: &["gets", "get"],
        verb: Verb::Get,
    },
    VerbShapeEntry {
        words: &["removes", "remove"],
        verb: Verb::Remove,
    },
    VerbShapeEntry {
        words: &["returns", "return"],
        verb: Verb::Return,
    },
    VerbShapeEntry {
        words: &["exchanges", "exchange"],
        verb: Verb::Exchange,
    },
    VerbShapeEntry {
        words: &["becomes", "become"],
        verb: Verb::Become,
    },
    VerbShapeEntry {
        words: &["switches", "switch"],
        verb: Verb::Switch,
    },
    VerbShapeEntry {
        words: &["skips", "skip"],
        verb: Verb::Skip,
    },
    VerbShapeEntry {
        words: &["surveils", "surveil"],
        verb: Verb::Surveil,
    },
    VerbShapeEntry {
        words: &["incubates", "incubate"],
        verb: Verb::Incubate,
    },
    VerbShapeEntry {
        words: &["shuffles", "shuffle"],
        verb: Verb::Shuffle,
    },
    VerbShapeEntry {
        words: &["reorders", "reorder"],
        verb: Verb::Reorder,
    },
    VerbShapeEntry {
        words: &["pays", "pay"],
        verb: Verb::Pay,
    },
    VerbShapeEntry {
        words: &["takes", "take"],
        verb: Verb::Take,
    },
    VerbShapeEntry {
        words: &["detains", "detain"],
        verb: Verb::Detain,
    },
    VerbShapeEntry {
        words: &["goads", "goad"],
        verb: Verb::Goad,
    },
    VerbShapeEntry {
        words: &["suspects", "suspect"],
        verb: Verb::Suspect,
    },
    VerbShapeEntry {
        words: &["ends", "end"],
        verb: Verb::End,
    },
];
const PREVENT_NEXT_DAMAGE_CORE_PREFIX: &[&str] = &["prevent"];
const PREVENT_NEXT_DAMAGE_CORE_SUFFIX: &[&str] = &["this", "turn"];
const PREVENT_NEXT_DAMAGE_CORE_REQUIRED: &[&str] = &["next", "damage"];
const ATTACK_OR_BLOCK_IF_ABLE_TAIL_PHRASES: &[&[&str]] = &[
    &["attack", "or", "block", "this", "turn", "if", "able"],
    &["attacks", "or", "blocks", "this", "turn", "if", "able"],
    &["attacks", "or", "block", "this", "turn", "if", "able"],
    &["attack", "or", "blocks", "this", "turn", "if", "able"],
];

fn verb_from_word(word: &str) -> Option<Verb> {
    VERB_SHAPES
        .iter()
        .find(|entry| entry.words.contains(&word))
        .map(|entry| entry.verb)
}
const ATTACK_IF_ABLE_TAIL_PHRASES: &[&[&str]] = &[
    &["attack", "this", "turn", "if", "able"],
    &["attacks", "this", "turn", "if", "able"],
];
const BLOCK_IF_ABLE_TAIL_PHRASES: &[&[&str]] = &[
    &["block", "this", "turn", "if", "able"],
    &["blocks", "this", "turn", "if", "able"],
];
const CANT_WORDS: &[&str] = &["cant", "can't", "cannot"];

const INLINE_TOKEN_RULES_TAIL_PREFIXES: &[&[&str]] = &[
    &["when"],
    &["whenever"],
    &["when", "this", "token"],
    &["whenever", "this", "token"],
    &["this", "token"],
    &["that", "token"],
    &["those", "tokens"],
    &["except", "it"],
    &["except", "they"],
    &["except", "its"],
    &["except", "their"],
    &["this", "creature"],
    &["that", "creature"],
    &["at", "the", "beginning"],
    &["at", "beginning"],
    &["sacrifice", "this", "token"],
    &["sacrifice", "that", "token"],
    &["sacrifice", "this", "permanent"],
    &["sacrifice", "that", "permanent"],
    &["sacrifice", "it"],
    &["sacrifice", "them"],
    &["it", "has"],
    &["it", "gains"],
    &["they", "have"],
    &["they", "gain"],
    &["equip"],
    &["equipped", "creature"],
    &["enchanted", "creature"],
    &["r"],
    &["t"],
];

const DESTROY_EXILE_GAIN_CONTROL_ALL_PREFIXES: &[&[&str]] = &[
    &["destroy", "all"],
    &["exile", "all"],
    &["gain", "control", "of", "all"],
];
const GENERIC_FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"], &["each"]];
const REST_PREFIXES: &[&[&str]] = &[&["the", "rest"], &["rest"]];
const PHASES_END_PREFIXES: &[&[&str]] = &[&["phases", "end"]];
const CLASH_PREFIXES: &[&[&str]] = &[&["clash"], &["clashes"]];
const THAT_MANY_FOLLOWUP_PREFIXES: &[&[&str]] = &[
    &["draw", "that", "many"],
    &["draws", "that", "many"],
    &["discard", "that", "many"],
    &["discards", "that", "many"],
    &["create", "that", "many"],
    &["creates", "that", "many"],
];
const LIFE_EQUAL_TO_THAT_PREFIXES: &[&[&str]] = &[
    &["you", "gain", "life", "equal", "to", "that"],
    &["you", "gain", "life", "equal", "to", "its"],
    &["you", "gain", "life", "equal", "to", "their"],
    &["you", "lose", "life", "equal", "to", "that"],
    &["you", "lose", "life", "equal", "to", "its"],
    &["you", "lose", "life", "equal", "to", "their"],
    &["gain", "life", "equal", "to", "that"],
    &["gain", "life", "equal", "to", "its"],
    &["gain", "life", "equal", "to", "their"],
    &["gains", "life", "equal", "to", "that"],
    &["gains", "life", "equal", "to", "its"],
    &["gains", "life", "equal", "to", "their"],
    &["lose", "life", "equal", "to", "that"],
    &["lose", "life", "equal", "to", "its"],
    &["lose", "life", "equal", "to", "their"],
    &["loses", "life", "equal", "to", "that"],
    &["loses", "life", "equal", "to", "its"],
    &["loses", "life", "equal", "to", "their"],
];
const DEAL_DAMAGE_EQUAL_TO_PREFIXES: &[&[&str]] = &[
    &["it", "deal", "damage", "equal", "to"],
    &["it", "deals", "damage", "equal", "to"],
    &["that", "creature", "deal", "damage", "equal", "to"],
    &["that", "creature", "deals", "damage", "equal", "to"],
    &["that", "objects", "deal", "damage", "equal", "to"],
    &["that", "objects", "deals", "damage", "equal", "to"],
];
const PUT_PREFIXES: &[&[&str]] = &[&["put"], &["puts"]];
const PUT_BACK_PREFIXES: &[&[&str]] = &[
    &["put", "it", "back"],
    &["put", "them", "back"],
    &["puts", "it", "back"],
    &["puts", "them", "back"],
];
const INLINE_TOKEN_RULES_CONTEXT_PHRASES: &[&[&str]] = &[
    &["when", "this", "token"],
    &["whenever", "this", "token"],
    &["at", "the", "beginning", "of"],
];
const EXCEPT_COPY_TOKEN_CONTEXT_WORDS: &[&str] = &["except", "copy", "token"];
const CURRENT_CARD_TYPE_LIST_MARKER_WORDS: &[&str] = &["or", "and/or"];
const PREVENT_NEXT_DAMAGE_TAIL_PREFIX: &[&str] = &["that", "would", "be", "dealt", "to"];
const THIS_TURN_SUFFIX: &[&str] = &["this", "turn"];
const PREVENT_ALL_DAMAGE_DURATION_FIRST_PREFIX: &[&str] = &[
    "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
];
const PREVENT_ALL_DAMAGE_TARGET_FIRST_PREFIX: &[&str] = &[
    "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
];
const CAN_ATTACK_AS_THOUGH_NO_DEFENDER_PREFIX: &[&str] = &["can", "attack"];
const CAN_ATTACK_AS_THOUGH_NO_DEFENDER_SUFFIX: &[&str] = &["defender"];
const AS_THOUGH_PHRASE: &[&str] = &["as", "though"];
const CAN_ATTACK_AS_THOUGH_NO_DEFENDER_REQUIRED: &[&str] = &["turn", "have"];
const IF_ABLE_SUFFIX: &[&str] = &["if", "able"];
const PHASE_WORD_TAIL_SUFFIXES: &[&[&str]] = &[
    &["phase", "out"],
    &["phases", "out"],
    &["phase", "in"],
    &["phases", "in"],
];
const CHOOSE_TARGET_PRELUDE_PREFIXES: &[&[&str]] = &[&["choose"], &["chooses"]];
const POWER_TOUGHNESS_AXIS_SUFFIXES: &[&[&str]] =
    &[&["power"], &["total", "power"], &["base", "power"]];
const BECOME_WITH_QUOTED_ABILITY_CONTEXT_WORDS: &[&str] = &["becomes", "with"];
const REPEAT_THIS_PROCESS_PHRASES: &[&[&str]] = &[
    &["repeat", "this", "process"],
    &["and", "repeat", "this", "process"],
];
const BACK_REFERENCE_WORDS: &[&str] = &["that", "it", "them", "its"];
const PUT_OR_DOUBLE_COUNTER_FOLLOWUP_PREFIXES: &[&[&str]] = &[&["put"], &["double"]];
const NONVERB_EFFECT_HEAD_WORDS: &[&str] = &[
    "double",
    "distribute",
    "support",
    "bolster",
    "adapt",
    "open",
    "manifest",
    "connive",
    "endure",
    "endures",
    "explore",
    "explores",
    "earthbend",
    "harness",
    "harnesses",
];
const KEYWORD_ACTION_EFFECT_HEAD_WORDS: &[&str] = &[
    "adapt",
    "adapts",
    "bolster",
    "bolsters",
    "connive",
    "connives",
    "earthbend",
    "earthbends",
    "harness",
    "harnesses",
    "endure",
    "endures",
    "explore",
    "explores",
    "manifest",
    "manifests",
    "open",
    "opens",
    "support",
    "supports",
];
const ATTACH_OR_ATTACHES_WORDS: &[&str] = &["attach", "attaches"];
const DEAL_OR_DEALS_WORDS: &[&str] = &["deal", "deals"];
const DEAL_DAMAGE_EQUAL_TOTAL_MANA_VALUE_REQUIRED: &[&str] =
    &["damage", "equal", "total", "mana", "value"];
const DEAL_DAMAGE_FOLLOWUP_REQUIRED: &[&str] = &["damage"];
const RETURN_WITH_COUNTER_FOLLOWUP_PREFIX: &[&str] = &["return"];
const THAT_PLAYER_CONTROLS_PHRASE: &[&str] = &["that", "player", "controls"];

fn token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word().is_some_and(|word| word == expected)
}

fn words_contain_all(words: &[&str], required: &[&str]) -> bool {
    required
        .iter()
        .all(|word| word_slice_contains_word(words, word))
}

fn words_contain_any(words: &[&str], choices: &[&str]) -> bool {
    choices
        .iter()
        .any(|word| word_slice_contains_word(words, word))
}

fn find_word_matching_any(words: &[&str], choices: &[&str]) -> Option<usize> {
    words.iter().position(|word| choices.contains(word))
}

pub(crate) fn strip_leading_instead_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    if !token_slice_first_is(tokens, "instead") || token_slice_at_is_any(tokens, 1, &["of", "if"]) {
        return None;
    }

    let stripped = trim_commas(&tokens[1..]);
    if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    }
}

pub(crate) fn strip_leading_instead_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    if !token_slice_first_is(tokens, "instead") || token_slice_at_is_any(tokens, 1, &["of", "if"]) {
        return None;
    }

    let stripped = trim_lexed_commas(&tokens[1..]);
    if stripped.is_empty() {
        None
    } else {
        Some(stripped)
    }
}

fn is_basic_color_word(word: &str) -> bool {
    matches!(
        word,
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}

fn is_card_type_word(word: &str) -> bool {
    matches!(
        word,
        "artifact"
            | "artifacts"
            | "battle"
            | "battles"
            | "creature"
            | "creatures"
            | "enchantment"
            | "enchantments"
            | "instant"
            | "instants"
            | "land"
            | "lands"
            | "planeswalker"
            | "planeswalkers"
            | "sorcery"
            | "sorceries"
            | "kindred"
    )
}

fn is_shared_card_type_list_noun(word: &str) -> bool {
    matches!(
        word,
        "card" | "cards" | "spell" | "spells" | "permanent" | "permanents"
    )
}

fn starts_with_each_player_or_opponent(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_match_any_prefix(tokens, EACH_PLAYER_OR_OPPONENT_PREFIXES).is_some()
}

pub(crate) fn starts_with_inline_token_rules_tail(tokens: &[OwnedLexToken]) -> bool {
    grammar::words_match_any_prefix(tokens, INLINE_TOKEN_RULES_TAIL_PREFIXES).is_some()
}

fn starts_with_inline_token_rules_continuation(words: &[&str]) -> bool {
    matches!(
        words.first().copied(),
        Some(
            "it" | "they"
                | "that"
                | "those"
                | "this"
                | "gain"
                | "gains"
                | "draw"
                | "draws"
                | "add"
                | "deal"
                | "deals"
                | "destroy"
                | "destroys"
                | "exile"
                | "exiles"
                | "return"
                | "returns"
                | "tap"
                | "untap"
                | "sacrifice"
                | "create"
                | "put"
                | "fights"
                | "fight"
        )
    )
}

fn starts_with_nonverb_effect_head(words: &[&str]) -> bool {
    matches!(
        words,
        ["choose" | "chooses", ..]
            | ["you", "choose" | "chooses", ..]
            | ["that", "player" | "players", "choose" | "chooses", ..]
            | ["the", "voter", "choose" | "chooses", ..]
            | [
                "target",
                "player" | "players" | "opponent" | "opponents",
                "choose" | "chooses",
                ..
            ]
            | ["after", "this", "phase", ..]
            | ["after", "this", "main", "phase", ..]
    ) || words.first().is_some_and(|word| {
        matches!(
            *word,
            "double"
                | "distribute"
                | "support"
                | "bolster"
                | "adapt"
                | "open"
                | "manifest"
                | "populate"
                | "connive"
                | "endure"
                | "endures"
                | "explore"
                | "explores"
                | "earthbend"
                | "harness"
                | "harnesses"
        )
    }) || words
        .iter()
        .any(|word| KEYWORD_ACTION_EFFECT_HEAD_WORDS.contains(word))
}

fn is_cant_restriction_clause_words(words: &[&str]) -> bool {
    words.iter().any(|word| CANT_WORDS.contains(word))
        && words.iter().any(|word| {
            ATTACK_OR_ATTACKS_WORDS.contains(word) || BLOCK_OR_BLOCKS_WORDS.contains(word)
        })
}

fn starts_with_player_may_clause_lexed(words: &[&str]) -> bool {
    matches!(
        words,
        ["you", "may", ..]
            | ["they", "may", ..]
            | ["the", "player" | "players", "may", ..]
            | ["that", "player" | "players", "may", ..]
            | ["that", "opponent" | "opponents", "may", ..]
            | ["target", "player" | "players", "may", ..]
            | ["target", "opponent" | "opponents", "may", ..]
            | ["defending", "player", "may", ..]
            | ["attacking", "player", "may", ..]
    )
}

pub(crate) fn is_token_creation_context(tokens: &[OwnedLexToken]) -> bool {
    token_slice_first_is(tokens, "create")
        && (grammar::contains_word(tokens, "token") || grammar::contains_word(tokens, "tokens"))
}

fn has_inline_token_rules_context(words: &[&str]) -> bool {
    word_slice_contains_any_phrase(words, INLINE_TOKEN_RULES_CONTEXT_PHRASES)
        || words_contain_all(words, EXCEPT_COPY_TOKEN_CONTEXT_WORDS)
}

fn should_keep_and_for_token_rules(current: &[OwnedLexToken], remaining: &[OwnedLexToken]) -> bool {
    should_keep_and_for_token_rules_lexed(current, remaining)
}

fn should_keep_and_for_attachment_object_list(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_attachment_object_list_lexed(current, remaining)
}

fn should_keep_and_for_each_player_may_clause(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_each_player_may_clause_lexed(current, remaining)
}

fn should_keep_and_for_put_rest_clause(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_put_rest_clause_lexed(current, remaining)
}

fn should_keep_and_for_steps_and_phases_end(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_steps_and_phases_end_lexed(current, remaining)
}

fn should_keep_and_for_exchange_zones(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_exchange_zones_lexed(current, remaining)
}

fn should_keep_and_for_card_type_list(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    should_keep_and_for_card_type_list_lexed(current, remaining)
}

pub(crate) fn split_effect_chain_on_and(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if token_word_is(token, AND_WORD) {
            let prev_word = current.last().and_then(OwnedLexToken::as_word);
            let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
            let is_color_pair = prev_word.zip(next_word).is_some_and(|(left, right)| {
                is_basic_color_word(left) && is_basic_color_word(right)
            });
            if is_color_pair
                || should_keep_and_for_token_rules(&current, &tokens[idx + 1..])
                || should_keep_and_for_attachment_object_list(&current, &tokens[idx + 1..])
                || should_keep_and_for_each_player_may_clause(&current, &tokens[idx + 1..])
                || should_keep_and_for_put_rest_clause(&current, &tokens[idx + 1..])
                || should_keep_and_for_steps_and_phases_end(&current, &tokens[idx + 1..])
                || should_keep_and_for_exchange_zones(&current, &tokens[idx + 1..])
                || should_keep_and_for_card_type_list(&current, &tokens[idx + 1..])
                || should_keep_and_for_become_with_quoted_ability(&current, &tokens[idx + 1..])
            {
                current.push(token.clone());
                continue;
            }
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn find_verb_lexed(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    let words = token_word_refs(tokens);
    find_verb_words_lexed(&words)
}

pub(crate) fn find_verb_words_lexed(words: &[&str]) -> Option<(Verb, usize)> {
    for (idx, word) in words.iter().enumerate() {
        let lower = word.to_ascii_lowercase();
        if END_WORDS.contains(&lower.as_str())
            && words
                .get(idx.saturating_sub(1))
                .is_some_and(|word| *word == UNTIL_WORD)
            && words.get(idx + 1).is_some_and(|word| *word == OF_WORD)
            && words
                .get(idx + 2)
                .is_some_and(|word| TURN_OR_COMBAT_WORDS.contains(word))
        {
            continue;
        }
        if COUNTER_OR_COUNTERS_WORDS.contains(&lower.as_str())
            && words
                .get(idx + 1)
                .is_some_and(|word| COUNTER_NOUN_CONTEXT_WORDS.contains(word))
        {
            continue;
        }
        let Some(local) = verb_from_word(&lower) else {
            continue;
        };
        return Some((local, idx));
    }

    None
}

fn should_keep_and_for_token_rules_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = token_word_refs(current);
    if current_words.is_empty() {
        return false;
    }
    if !is_token_creation_context(current) && !has_inline_token_rules_context(&current_words) {
        return false;
    }
    starts_with_inline_token_rules_tail(remaining)
}

fn should_keep_and_for_attachment_object_list_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = token_word_refs(current);
    let remaining_words = token_word_refs(remaining);
    if current_words.is_empty() || remaining_words.is_empty() {
        return false;
    }

    let starts_attachment_subject = remaining_words.first().is_some_and(|word| {
        matches!(
            *word,
            "aura"
                | "auras"
                | "equipment"
                | "equipments"
                | "enchantment"
                | "enchantments"
                | "artifact"
                | "artifacts"
        )
    });
    if !starts_attachment_subject || !grammar::contains_word(remaining, "attached") {
        return false;
    }

    grammar::words_match_any_prefix(current, DESTROY_EXILE_GAIN_CONTROL_ALL_PREFIXES).is_some()
}

fn should_keep_and_for_each_player_may_clause_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = token_word_refs(current);
    if current_words.is_empty() || !grammar::contains_word(current, "may") {
        return false;
    }

    if !starts_with_each_player_or_opponent(current) {
        return false;
    }

    if remaining.is_empty() {
        return false;
    }
    if grammar::words_match_any_prefix(remaining, GENERIC_FOR_EACH_PREFIXES).is_some() {
        return false;
    }

    true
}

fn should_keep_and_for_put_rest_clause_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }

    let current_words = token_word_refs(current);
    if current_words.is_empty() {
        return false;
    }

    let starts_with_rest = grammar::words_match_any_prefix(remaining, REST_PREFIXES).is_some();
    if !starts_with_rest {
        return false;
    }

    grammar::contains_word(current, "put")
        && grammar::contains_word(current, "into")
        && grammar::contains_word(current, "hand")
}

fn should_keep_and_for_steps_and_phases_end_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    grammar::words_match_suffix(current, &["as", "steps"]).is_some()
        && grammar::words_match_any_prefix(remaining, PHASES_END_PREFIXES).is_some()
}

fn should_keep_and_for_exchange_zones_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    let current_words = token_word_refs(current);
    let remaining_words = token_word_refs(remaining);
    word_slice_first_is(&current_words, EXCHANGE_WORD)
        && current_words
            .iter()
            .any(|word| parse_zone_word(word).is_some())
        && remaining_words
            .first()
            .is_some_and(|word| parse_zone_word(word).is_some())
}

fn should_keep_and_for_card_type_list_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }

    let current_words = token_word_refs(current);
    let remaining_words = token_word_refs(remaining);
    if current_words.is_empty() || remaining_words.is_empty() {
        return false;
    }

    if !remaining_words
        .first()
        .is_some_and(|word| is_card_type_word(word))
    {
        return false;
    }
    if !remaining_words
        .iter()
        .any(|word| is_shared_card_type_list_noun(word))
    {
        return false;
    }

    let current_last_type = current_words
        .iter()
        .rev()
        .find(|word| !ARTICLE_OR_QUANTIFIER_WORDS.contains(word))
        .is_some_and(|word| is_card_type_word(word));
    if !current_last_type {
        return false;
    }

    let current_has_type = current_words.iter().any(|word| is_card_type_word(word));
    let current_has_list_marker = contains_token_kind(current, TokenKind::Comma)
        || words_contain_any(&current_words, CURRENT_CARD_TYPE_LIST_MARKER_WORDS);

    current_has_type && current_has_list_marker
}

fn is_prevent_next_damage_clause_words_lexed(words: &[&str]) -> bool {
    if !word_slice_starts_with(words, PREVENT_NEXT_DAMAGE_CORE_PREFIX)
        || !word_slice_ends_with(words, PREVENT_NEXT_DAMAGE_CORE_SUFFIX)
        || !words_contain_all(words, PREVENT_NEXT_DAMAGE_CORE_REQUIRED)
        || !word_slice_first_is(words, PREVENT_WORD)
    {
        return false;
    }

    let next_idx = 1 + usize::from(word_slice_at_is(words, 1, THE_WORD));
    if !word_slice_at_is(words, next_idx, NEXT_WORD) {
        return false;
    }

    // "next" then exactly one wildcard word then "damage".
    let idx = next_idx + 2;
    if words.get(next_idx + 1).is_none() {
        return false;
    }

    word_slice_at_is(words, idx, DAMAGE_WORD)
        && word_slice_starts_with(&words[idx + 1..], PREVENT_NEXT_DAMAGE_TAIL_PREFIX)
        && word_slice_ends_with(words, THIS_TURN_SUFFIX)
        && words.len() > idx + 7
}

fn is_prevent_all_damage_clause_words_lexed(words: &[&str]) -> bool {
    if word_slice_starts_with(words, PREVENT_ALL_DAMAGE_DURATION_FIRST_PREFIX) {
        return words.len() > 11;
    }

    word_slice_starts_with(words, PREVENT_ALL_DAMAGE_TARGET_FIRST_PREFIX)
        && word_slice_ends_with(words, THIS_TURN_SUFFIX)
        && words.len() > 9
}

fn is_can_attack_as_though_no_defender_clause_words_lexed(words: &[&str]) -> bool {
    let Some(can_idx) = find_word_matching_any(words, &[CAN_WORD]) else {
        return false;
    };
    let tail = &words[can_idx..];
    word_slice_starts_with(tail, CAN_ATTACK_AS_THOUGH_NO_DEFENDER_PREFIX)
        && word_slice_ends_with(tail, CAN_ATTACK_AS_THOUGH_NO_DEFENDER_SUFFIX)
        && word_slice_contains_phrase(tail, AS_THOUGH_PHRASE)
        && words_contain_all(tail, CAN_ATTACK_AS_THOUGH_NO_DEFENDER_REQUIRED)
}

fn is_attack_or_block_this_turn_if_able_clause_words_lexed(words: &[&str]) -> bool {
    let Some(attack_idx) = find_word_matching_any(words, ATTACK_OR_ATTACKS_WORDS) else {
        return false;
    };
    word_slice_eq_any(&words[attack_idx..], ATTACK_OR_BLOCK_IF_ABLE_TAIL_PHRASES)
}

fn is_attack_this_turn_if_able_clause_words_lexed(words: &[&str]) -> bool {
    let Some(attack_idx) = find_word_matching_any(words, ATTACK_OR_ATTACKS_WORDS) else {
        return false;
    };
    word_slice_eq_any(&words[attack_idx..], ATTACK_IF_ABLE_TAIL_PHRASES)
}

fn is_must_block_if_able_clause_words_lexed(words: &[&str]) -> bool {
    if matches!(
        words,
        ["all", "creatures", "able", "to", "block", .., "do", "so"]
    ) {
        return true;
    }

    let Some(block_idx) = find_word_matching_any(words, BLOCK_OR_BLOCKS_WORDS) else {
        return false;
    };
    if block_idx == 0 || block_idx + 1 >= words.len() {
        return false;
    }

    let tail = &words[block_idx..];
    word_slice_eq_any(tail, BLOCK_IF_ABLE_TAIL_PHRASES)
        || word_slice_ends_with(tail, IF_ABLE_SUFFIX)
}

fn is_phase_clause_words_lexed(words: &[&str]) -> bool {
    word_slice_ends_with_any(words, PHASE_WORD_TAIL_SUFFIXES) && words.len() >= 3
}

fn is_choose_target_prelude_clause_words_lexed(words: &[&str]) -> bool {
    word_slice_starts_with_any(words, CHOOSE_TARGET_PRELUDE_PREFIXES)
        && word_slice_contains_word(words, TARGET_WORD)
}

fn should_keep_and_for_power_toughness_axis_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    let current_words = token_word_refs(current);
    let remaining_words = token_word_refs(remaining);
    word_slice_ends_with_any(&current_words, POWER_TOUGHNESS_AXIS_SUFFIXES)
        && word_slice_first_is(&remaining_words, TOUGHNESS_WORD)
}

fn should_keep_and_for_become_with_quoted_ability(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    let current_words = token_word_refs(current);
    if !words_contain_all(&current_words, BECOME_WITH_QUOTED_ABILITY_CONTEXT_WORDS) {
        return false;
    }
    remaining
        .first()
        .is_some_and(|token| token.kind == TokenKind::Quote)
        || starts_with_inline_token_rules_tail(remaining)
}

fn should_keep_and_for_shared_subject_gain_clause(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    let current_words = token_word_refs(current);
    let has_shared_subject_modifier = current_words
        .iter()
        .any(|word| SHARED_SUBJECT_MODIFIER_WORDS.contains(word));
    if !has_shared_subject_modifier {
        return false;
    }
    remaining
        .iter()
        .find_map(OwnedLexToken::as_word)
        .is_some_and(|word| GAIN_HAVE_LOSE_WORDS.contains(&word))
}

pub(crate) fn split_effect_chain_on_and_lexed(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut start = 0usize;

    for (idx, token) in tokens.iter().enumerate() {
        if !token_word_is(token, AND_WORD) {
            continue;
        }
        let current = trim_lexed_commas(&tokens[start..idx]);
        let remaining = trim_lexed_commas(&tokens[idx + 1..]);
        let prev_word = current.iter().rev().find_map(OwnedLexToken::as_word);
        let next_word = remaining.iter().find_map(OwnedLexToken::as_word);
        let is_color_pair = prev_word
            .zip(next_word)
            .is_some_and(|(left, right)| is_basic_color_word(left) && is_basic_color_word(right));
        if is_color_pair
            || should_keep_and_for_token_rules_lexed(current, remaining)
            || should_keep_and_for_attachment_object_list_lexed(current, remaining)
            || should_keep_and_for_each_player_may_clause_lexed(current, remaining)
            || should_keep_and_for_put_rest_clause_lexed(current, remaining)
            || should_keep_and_for_steps_and_phases_end_lexed(current, remaining)
            || should_keep_and_for_exchange_zones_lexed(current, remaining)
            || should_keep_and_for_card_type_list_lexed(current, remaining)
            || should_keep_and_for_power_toughness_axis_lexed(current, remaining)
            || should_keep_and_for_become_with_quoted_ability(current, remaining)
            || should_keep_and_for_shared_subject_gain_clause(current, remaining)
        {
            continue;
        }
        if !current.is_empty() {
            segments.push(current);
        }
        start = idx + 1;
    }

    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

pub(crate) fn has_effect_head_without_verb(tokens: &[OwnedLexToken]) -> bool {
    let token_words = token_word_refs(tokens);
    if word_slice_eq_any(&token_words, REPEAT_THIS_PROCESS_PHRASES) {
        return true;
    }

    if starts_with_nonverb_effect_head(&token_words) {
        return true;
    }

    parse_prevent_next_damage_clause(tokens)
        .ok()
        .flatten()
        .is_some()
        || parse_prevent_all_damage_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_can_attack_as_though_no_defender_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_attack_or_block_this_turn_if_able_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_attack_this_turn_if_able_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_must_block_if_able_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || is_cant_restriction_clause_words(&token_words)
}

pub(crate) fn has_effect_head_without_verb_lexed(tokens: &[OwnedLexToken]) -> bool {
    let token_words = token_word_refs(tokens);
    if word_slice_eq_any(&token_words, REPEAT_THIS_PROCESS_PHRASES) {
        return true;
    }

    if starts_with_nonverb_effect_head(&token_words) {
        return true;
    }

    is_prevent_next_damage_clause_words_lexed(&token_words)
        || is_prevent_all_damage_clause_words_lexed(&token_words)
        || is_can_attack_as_though_no_defender_clause_words_lexed(&token_words)
        || is_attack_or_block_this_turn_if_able_clause_words_lexed(&token_words)
        || is_attack_this_turn_if_able_clause_words_lexed(&token_words)
        || is_must_block_if_able_clause_words_lexed(&token_words)
        || is_phase_clause_words_lexed(&token_words)
        || is_choose_target_prelude_clause_words_lexed(&token_words)
        || is_cant_restriction_clause_words(&token_words)
}

pub(crate) fn segment_has_effect_head_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    find_verb_lexed(tokens).is_some()
        || has_effect_head_without_verb_lexed(tokens)
        || starts_with_player_may_clause_lexed(&words)
}

pub(crate) fn segment_has_effect_head(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(tokens);
    find_verb(tokens).is_some()
        || has_effect_head_without_verb(tokens)
        || starts_with_player_may_clause_lexed(&words)
}

pub(crate) fn split_segments_on_comma_then(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let segment_refs = segments.iter().map(Vec::as_slice).collect::<Vec<_>>();
    split_segments_on_comma_then_lexed(segment_refs)
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect()
}

pub(crate) fn split_segments_on_comma_effect_head(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let segment_refs = segments.iter().map(Vec::as_slice).collect::<Vec<_>>();
    split_segments_on_comma_effect_head_lexed(segment_refs)
        .into_iter()
        .map(|segment| segment.to_vec())
        .collect()
}

pub(crate) fn split_segments_on_comma_then_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let starts_with_for_each_player_or_opponent = starts_with_each_player_or_opponent(segment);
        let mut split_point = None;
        let mut inside_quotes = false;
        for i in 0..segment.len().saturating_sub(1) {
            if segment[i].kind == TokenKind::Quote {
                inside_quotes = !inside_quotes;
                continue;
            }
            if !inside_quotes
                && (matches!(segment[i].kind, TokenKind::Comma)
                    && segment
                        .get(i + 1)
                        .is_some_and(|token| token_word_is(token, THEN_WORD))
                    || token_word_is(&segment[i], THEN_WORD))
            {
                let then_idx = if token_word_is(&segment[i], THEN_WORD) {
                    i
                } else {
                    i + 1
                };
                let before_then = trim_lexed_commas(&segment[..i]);
                let before_words = token_word_refs(before_then);
                let starts_with_clash =
                    grammar::words_match_any_prefix(before_then, CLASH_PREFIXES).is_some();
                let after_then = trim_lexed_commas(&segment[then_idx + 1..]);
                let after_words = token_word_refs(after_then);
                let has_back_ref = words_contain_any(&after_words, BACK_REFERENCE_WORDS);
                let has_nonverb_effect_head =
                    word_slice_first_is_any(&after_words, NONVERB_EFFECT_HEAD_WORDS);
                let has_effect_head = find_verb_lexed(after_then).is_some()
                    || parse_ability_line_lexed(after_then).is_some()
                    || has_nonverb_effect_head;
                let allow_backref_split = has_back_ref
                    && word_slice_starts_with_any(
                        &after_words,
                        PUT_OR_DOUBLE_COUNTER_FOLLOWUP_PREFIXES,
                    )
                    && words_contain_any(&after_words, COUNTER_OR_COUNTERS_WORDS);
                let allow_attach_followup =
                    word_slice_first_is_any(&after_words, ATTACH_OR_ATTACHES_WORDS);
                let allow_that_many_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && grammar::words_match_any_prefix(after_then, THAT_MANY_FOLLOWUP_PREFIXES)
                        .is_some();
                let allow_gain_or_lose_life_equal_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && grammar::words_match_any_prefix(after_then, LIFE_EQUAL_TO_THAT_PREFIXES)
                            .is_some();
                let allow_deal_damage_equal_power_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && grammar::words_match_any_prefix(
                            after_then,
                            DEAL_DAMAGE_EQUAL_TO_PREFIXES,
                        )
                        .is_some();
                let allow_deal_damage_equal_total_mana_value_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && words_contain_any(&after_words, DEAL_OR_DEALS_WORDS)
                        && words_contain_all(
                            &after_words,
                            DEAL_DAMAGE_EQUAL_TOTAL_MANA_VALUE_REQUIRED,
                        );
                let allow_for_each_damage_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, GENERIC_FOR_EACH_PREFIXES)
                        .is_some()
                    && words_contain_any(&after_words, DEAL_OR_DEALS_WORDS)
                    && words_contain_all(&after_words, DEAL_DAMAGE_FOLLOWUP_REQUIRED);
                let allow_target_pump_for_each_that_player_followup = has_back_ref
                    && !starts_with_for_each_player_or_opponent
                    && (word_slice_first_is(&after_words, TARGET_WORD)
                        || word_slice_starts_with(&after_words, &["up", "to"]))
                    && words_contain_any(&after_words, SHARED_SUBJECT_MODIFIER_WORDS)
                    && word_slice_contains_phrase(&after_words, THAT_PLAYER_CONTROLS_PHRASE);
                let allow_return_with_counter_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && word_slice_starts_with(&after_words, RETURN_WITH_COUNTER_FOLLOWUP_PREFIX)
                    && words_contain_any(&after_words, COUNTER_OR_COUNTERS_WORDS)
                    && word_slice_contains_any_phrase(&after_words, BACKREF_COUNTER_TARGET_PHRASES);
                let allow_return_battlefield_attached_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && word_slice_starts_with(
                            &after_words,
                            RETURN_WITH_COUNTER_FOLLOWUP_PREFIX,
                        )
                        && word_slice_contains_word(&after_words, "battlefield")
                        && word_slice_contains_any_phrase(
                            &after_words,
                            RETURN_BATTLEFIELD_ATTACHED_BACKREF_PHRASES,
                        );
                let allow_put_battlefield_with_counter_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && grammar::words_match_any_prefix(after_then, PUT_PREFIXES).is_some()
                        && grammar::contains_word(after_then, "battlefield")
                        && after_words
                            .iter()
                            .any(|word| COUNTER_OR_COUNTERS_WORDS.contains(word))
                        && word_slice_contains_any_phrase(
                            &after_words,
                            BACKREF_COUNTER_TARGET_PHRASES,
                        );
                let allow_put_into_hand_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, PUT_PREFIXES).is_some()
                    && grammar::contains_word(after_then, "into")
                    && grammar::contains_word(after_then, "hand");
                let allow_put_back_in_any_order_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, PUT_BACK_PREFIXES).is_some()
                    && grammar::contains_word(after_then, "any")
                    && grammar::contains_word(after_then, "order");
                let allow_exile_that_player_graveyard_followup = has_back_ref
                    && word_slice_first_is(&after_words, EXILE_WORD)
                    && word_slice_contains_any_phrase(
                        &after_words,
                        THAT_PLAYER_POSSESSIVE_GRAVEYARD_PREFIXES,
                    )
                    && words_contain_any(&after_words, GRAVEYARD_WORDS);
                let continues_inline_consult_bottom_remainder =
                    grammar::words_match_any_prefix(after_then, PUT_PREFIXES).is_some()
                        && word_slice_contains_word(&after_words, "rest")
                        && word_slice_contains_word(&after_words, "bottom")
                        && word_slice_contains_word(&after_words, "library")
                        && word_slice_contains_word(&before_words, "reveal")
                        && word_slice_contains_word(&before_words, "top")
                        && word_slice_contains_word(&before_words, "library");
                let allow_clash_followup = starts_with_clash;
                if has_effect_head
                    && !continues_inline_consult_bottom_remainder
                    && (!has_back_ref || allow_backref_split)
                    || has_effect_head && allow_clash_followup
                    || has_effect_head && allow_attach_followup
                    || has_effect_head && allow_that_many_followup
                    || has_effect_head && allow_gain_or_lose_life_equal_followup
                    || has_effect_head && allow_deal_damage_equal_power_followup
                    || has_effect_head && allow_deal_damage_equal_total_mana_value_followup
                    || has_effect_head && allow_for_each_damage_followup
                    || has_effect_head && allow_target_pump_for_each_that_player_followup
                    || has_effect_head && allow_return_with_counter_followup
                    || has_effect_head && allow_return_battlefield_attached_followup
                    || has_effect_head && allow_put_battlefield_with_counter_followup
                    || has_effect_head && allow_put_into_hand_followup
                    || has_effect_head && allow_put_back_in_any_order_followup
                    || has_effect_head && allow_exile_that_player_graveyard_followup
                {
                    split_point = Some(i);
                    break;
                }
            }
        }
        if let Some(idx) = split_point {
            let then_idx = if token_word_is(&segment[idx], THEN_WORD) {
                idx
            } else {
                idx + 1
            };
            let first_part = trim_lexed_commas(&segment[..idx]);
            let second_part = trim_lexed_commas(&segment[then_idx + 1..]);
            if !first_part.is_empty() {
                result.push(first_part);
            }
            if !second_part.is_empty() {
                result.push(second_part);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

pub(crate) fn split_segments_on_comma_effect_head_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let mut start = 0usize;
        let mut split_any = false;
        let mut inside_quotes = false;

        for idx in 0..segment.len() {
            if segment[idx].kind == TokenKind::Quote {
                inside_quotes = !inside_quotes;
                continue;
            }
            if inside_quotes || !matches!(segment[idx].kind, TokenKind::Comma) {
                continue;
            }
            let before = trim_lexed_commas(&segment[start..idx]);
            let after = trim_lexed_commas(&segment[idx + 1..]);
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let before_has_verb = find_verb_lexed(before).is_some();
            let after_starts_effect = find_verb_lexed(after)
                .is_some_and(|(_, verb_idx)| verb_idx == 0)
                || has_effect_head_without_verb_lexed(after);
            let before_words = token_word_refs(before);
            let after_words = token_word_refs(after);
            let duration_trigger_prefix =
                word_slice_first_is_any(&before_words, UNTIL_OR_DURING_WORDS)
                    && (grammar::contains_word(before, "whenever")
                        || grammar::contains_word(before, "when")
                        || word_slice_contains_phrase(&before_words, AT_THE_PHRASE));
            if word_slice_first_is(&before_words, UNLESS_WORD) || duration_trigger_prefix {
                continue;
            }
            if grammar::contains_word(before, "search") && grammar::contains_word(before, "library")
            {
                continue;
            }
            if grammar::contains_word(before, "target")
                && (word_slice_first_is_any(&after_words, TARGET_CARD_TYPE_WORDS)
                    || (word_slice_first_is(&after_words, OR_WORD)
                        && after_words
                            .get(1)
                            .is_some_and(|word| TARGET_CARD_TYPE_WORDS.contains(word))))
                && !is_cant_restriction_clause_words(&after_words)
            {
                continue;
            }
            let is_inline_token_rules_split = (is_token_creation_context(before)
                || has_inline_token_rules_context(&before_words))
                && (starts_with_inline_token_rules_tail(after)
                    || starts_with_inline_token_rules_continuation(&after_words));
            if is_inline_token_rules_split {
                continue;
            }
            if before_has_verb && after_starts_effect {
                result.push(before);
                start = idx + 1;
                split_any = true;
            }
        }
        if split_any {
            let tail = trim_lexed_commas(&segment[start..]);
            if !tail.is_empty() {
                result.push(tail);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

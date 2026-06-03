#![allow(dead_code)]

use super::super::clause_support::parse_ability_line_lexed;
use super::super::grammar::primitives as grammar;
use super::super::lexer::{
    OwnedLexToken, TokenKind, contains_token_kind, token_slice_at_is_any, token_slice_first_is,
    token_word_refs, trim_lexed_commas,
};
use super::super::util::{parse_zone_word, trim_commas};
use super::chain_carry::{Verb, find_verb};
use super::clause_pattern_helpers::{
    ClauseShape, clause_shape, parse_can_attack_as_though_no_defender_clause,
    parse_prevent_all_damage_clause, parse_prevent_next_damage_clause,
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
const CAN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["can"]);
const EXCHANGE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exchange"]);
const PREVENT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["prevent"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const NEXT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["next"]);
const DAMAGE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["damage"]);
const ATTACK_OR_ATTACKS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attack"], &["attacks"]]);
const BLOCK_OR_BLOCKS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["block"], &["blocks"]]);
const TOUGHNESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["toughness"]);
const UNTIL_OR_DURING_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["until"], &["during"]]);
const UNLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["unless"]);
const TARGET_CARD_TYPE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["artifact"],
            &["battle"],
            &["creature"],
            &["enchantment"],
            &["instant"],
            &["land"],
            &["planeswalker"],
            &["sorcery"],
        ]
);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const ARTICLE_OR_QUANTIFIER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["the"], &["all"], &["each"]]);
const SHARED_SUBJECT_MODIFIER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"], &["become"], &["becomes"]]);
const GAIN_HAVE_LOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["gain"],
            &["gains"],
            &["has"],
            &["have"],
            &["lose"],
            &["loses"],
        ]
);
struct VerbShapeEntry {
    pattern: ClauseShape<'static>,
    verb: Verb,
}

const END_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["end"], &["ends"]]);
const UNTIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["until"]);
const OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const TURN_OR_COMBAT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["turn"], &["combat"]]);
const COUNTER_NOUN_CONTEXT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["on"], &["from"], &["among"]]);
const BACKREF_COUNTER_TARGET_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["on", "it"], &["on", "them"]]]);
const AT_THE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["at", "the"]]);
const VERB_SHAPES: &[VerbShapeEntry] = &[
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["adds"], &["add"]]),
        verb: Verb::Add,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["moves"], &["move"]]),
        verb: Verb::Move,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["deals"], &["deal"]]),
        verb: Verb::Deal,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["draws"], &["draw"]]),
        verb: Verb::Draw,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["counters"], &["counter"]]),
        verb: Verb::Counter,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["destroys"], &["destroy"]]),
        verb: Verb::Destroy,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["exiles"], &["exile"]]),
        verb: Verb::Exile,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["reveals"], &["reveal"]]),
        verb: Verb::Reveal,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["looks"], &["look"]]),
        verb: Verb::Look,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["loses"], &["lose"]]),
        verb: Verb::Lose,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["gains"], &["gain"]]),
        verb: Verb::Gain,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["puts"], &["put"]]),
        verb: Verb::Put,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["sacrifices"], &["sacrifice"]]),
        verb: Verb::Sacrifice,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["creates"], &["create"]]),
        verb: Verb::Create,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["investigates"], &["investigate"]]),
        verb: Verb::Investigate,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["proliferates"], &["proliferate"]]),
        verb: Verb::Proliferate,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["taps"], &["tap"]]),
        verb: Verb::Tap,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["attaches"], &["attach"]]),
        verb: Verb::Attach,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["untaps"], &["untap"]]),
        verb: Verb::Untap,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["scries"], &["scry"]]),
        verb: Verb::Scry,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["discards"], &["discard"]]),
        verb: Verb::Discard,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["transforms"], &["transform"]]),
        verb: Verb::Transform,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["converts"], &["convert"]]),
        verb: Verb::Convert,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["flips"], &["flip"]]),
        verb: Verb::Flip,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["rolls"], &["roll"]]),
        verb: Verb::Roll,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["regenerates"], &["regenerate"]]),
        verb: Verb::Regenerate,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["mills"], &["mill"]]),
        verb: Verb::Mill,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["gets"], &["get"]]),
        verb: Verb::Get,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["removes"], &["remove"]]),
        verb: Verb::Remove,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["returns"], &["return"]]),
        verb: Verb::Return,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["exchanges"], &["exchange"]]),
        verb: Verb::Exchange,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["becomes"], &["become"]]),
        verb: Verb::Become,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["switches"], &["switch"]]),
        verb: Verb::Switch,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["skips"], &["skip"]]),
        verb: Verb::Skip,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["surveils"], &["surveil"]]),
        verb: Verb::Surveil,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["incubates"], &["incubate"]]),
        verb: Verb::Incubate,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["shuffles"], &["shuffle"]]),
        verb: Verb::Shuffle,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["reorders"], &["reorder"]]),
        verb: Verb::Reorder,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["pays"], &["pay"]]),
        verb: Verb::Pay,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["takes"], &["take"]]),
        verb: Verb::Take,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["detains"], &["detain"]]),
        verb: Verb::Detain,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["goads"], &["goad"]]),
        verb: Verb::Goad,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["suspects"], &["suspect"]]),
        verb: Verb::Suspect,
    },
    VerbShapeEntry {
        pattern: clause_shape!(exact_any & [&["ends"], &["end"]]),
        verb: Verb::End,
    },
];
const PREVENT_NEXT_DAMAGE_CORE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["prevent"];
    suffix & ["this", "turn"];
    contains_words & ["next", "damage"]
);
const ATTACK_OR_BLOCK_IF_ABLE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["attack", "or", "block", "this", "turn", "if", "able"],
            &["attacks", "or", "blocks", "this", "turn", "if", "able"],
            &["attacks", "or", "block", "this", "turn", "if", "able"],
            &["attack", "or", "blocks", "this", "turn", "if", "able"],
        ]
);

fn token_word_matches_shape(token: Option<&OwnedLexToken>, shape: &ClauseShape<'static>) -> bool {
    token.is_some_and(|token| shape.matches_token(token))
}

fn verb_from_word(word: &str) -> Option<Verb> {
    VERB_SHAPES
        .iter()
        .find(|entry| entry.pattern.matches_word(word))
        .map(|entry| entry.verb)
}
const ATTACK_IF_ABLE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["attack", "this", "turn", "if", "able"],
            &["attacks", "this", "turn", "if", "able"],
        ]
);
const BLOCK_IF_ABLE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["block", "this", "turn", "if", "able"],
            &["blocks", "this", "turn", "if", "able"],
        ]
);

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
const INLINE_TOKEN_RULES_CONTEXT_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["when", "this", "token"],
            &["whenever", "this", "token"],
            &["at", "the", "beginning", "of"],
        ]]
);
const EXCEPT_COPY_TOKEN_CONTEXT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["except", "copy", "token"]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const THEN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["then"]);
const COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const CURRENT_CARD_TYPE_LIST_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["or", "and/or"]]);
const PREVENT_NEXT_DAMAGE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["that", "would", "be", "dealt", "to"]);
const THIS_TURN_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["this", "turn"]);
const PREVENT_ALL_DAMAGE_DURATION_FIRST_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "prevent", "all", "damage", "that", "would", "be", "dealt", "this", "turn", "to",
        ]
);
const PREVENT_ALL_DAMAGE_TARGET_FIRST_PATTERN: ClauseShape<'static> = clause_shape!(prefix & [
    "prevent", "all", "damage", "that", "would", "be", "dealt", "to",
]; suffix & ["this", "turn"]);
const CAN_ATTACK_AS_THOUGH_NO_DEFENDER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["can", "attack"];
    suffix & ["defender"];
    contains_phrases & [&["as", "though"]];
    contains_words & ["turn", "have"]
);
const IF_ABLE_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["if", "able"]);
const PHASE_WORD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["phase", "out"],
            &["phases", "out"],
            &["phase", "in"],
            &["phases", "in"],
        ]
);
const CHOOSE_TARGET_PRELUDE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["choose"], &["chooses"]]; contains_words & ["target"]);
const POWER_TOUGHNESS_AXIS_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["power"], &["total", "power"], &["base", "power"]]);
const BECOME_WITH_QUOTED_ABILITY_CONTEXT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["becomes", "with"]);
const REPEAT_THIS_PROCESS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["repeat", "this", "process"],
            &["and", "repeat", "this", "process"]
        ]
);
const BACK_REFERENCE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["that", "it", "them", "its"]]);
const PUT_OR_DOUBLE_COUNTER_FOLLOWUP_PATTERN: ClauseShape<'static> = clause_shape!(prefix_any & [&["put"], &["double"]]; contains_any_words & [&["counter", "counters"]]);
const NONVERB_EFFECT_HEAD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["double"],
            &["distribute"],
            &["support"],
            &["bolster"],
            &["adapt"],
            &["open"],
            &["manifest"],
            &["connive"],
            &["earthbend"],
        ]
);
const ATTACH_OR_ATTACHES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attach"], &["attaches"]]);
const DEAL_DAMAGE_EQUAL_TOTAL_MANA_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(contains_any_words & [&["deal", "deals"]]; contains_words & ["damage", "equal", "total", "mana", "value"]);
const DEAL_DAMAGE_FOLLOWUP_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["deal", "deals"]]; contains_words & ["damage"]);
const RETURN_WITH_COUNTER_FOLLOWUP_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["return"]; contains_any_words & [&["counter", "counters"]]);

fn find_word_matching_shape(words: &[&str], shape: &ClauseShape<'static>) -> Option<usize> {
    words.iter().position(|word| shape.matches_word(word))
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
                | "earthbend"
        )
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
    INLINE_TOKEN_RULES_CONTEXT_PATTERN.matches_words(words)
        || EXCEPT_COPY_TOKEN_CONTEXT_PATTERN.matches_words(words)
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
        if AND_WORD_PATTERN.matches_token(token) {
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
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        let lower = word.to_ascii_lowercase();
        if END_WORD_PATTERN.matches_word(&lower)
            && tokens
                .get(idx.saturating_sub(1))
                .is_some_and(|token| UNTIL_WORD_PATTERN.matches_token(token))
            && tokens
                .get(idx + 1)
                .is_some_and(|token| OF_WORD_PATTERN.matches_token(token))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| TURN_OR_COMBAT_WORD_PATTERN.matches_token(token))
        {
            continue;
        }
        if COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(&lower)
            && tokens
                .get(idx + 1)
                .is_some_and(|token| COUNTER_NOUN_CONTEXT_WORD_PATTERN.matches_token(token))
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
    EXCHANGE_WORD_PATTERN.matches_first_word(&current_words)
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
        .find(|word| !ARTICLE_OR_QUANTIFIER_WORD_PATTERN.matches_word(word))
        .is_some_and(|word| is_card_type_word(word));
    if !current_last_type {
        return false;
    }

    let current_has_type = current_words.iter().any(|word| is_card_type_word(word));
    let current_has_list_marker = contains_token_kind(current, TokenKind::Comma)
        || CURRENT_CARD_TYPE_LIST_MARKER_PATTERN.matches_words(&current_words);

    current_has_type && current_has_list_marker
}

fn is_prevent_next_damage_clause_words_lexed(words: &[&str]) -> bool {
    if !PREVENT_NEXT_DAMAGE_CORE_PATTERN.matches_words(words)
        || !PREVENT_WORD_PATTERN.matches_first_word(words)
    {
        return false;
    }

    let mut idx = 1usize;
    if THE_WORD_PATTERN.matches_word_at(words, idx) {
        idx += 1;
    }
    if !NEXT_WORD_PATTERN.matches_word_at(words, idx) {
        return false;
    }
    idx += 1;

    if words.get(idx).is_none() {
        return false;
    }
    idx += 1;

    DAMAGE_WORD_PATTERN.matches_word_at(words, idx)
        && PREVENT_NEXT_DAMAGE_TAIL_PATTERN.matches_words(&words[idx + 1..])
        && THIS_TURN_SUFFIX_PATTERN.matches_words(words)
        && words.len() > idx + 7
}

fn is_prevent_all_damage_clause_words_lexed(words: &[&str]) -> bool {
    if PREVENT_ALL_DAMAGE_DURATION_FIRST_PATTERN.matches_words(words) {
        return words.len() > 11;
    }

    PREVENT_ALL_DAMAGE_TARGET_FIRST_PATTERN.matches_words(words) && words.len() > 9
}

fn is_can_attack_as_though_no_defender_clause_words_lexed(words: &[&str]) -> bool {
    let Some(can_idx) = find_word_matching_shape(words, &CAN_WORD_PATTERN) else {
        return false;
    };
    let tail = &words[can_idx..];
    CAN_ATTACK_AS_THOUGH_NO_DEFENDER_TAIL_PATTERN.matches_words(tail)
}

fn is_attack_or_block_this_turn_if_able_clause_words_lexed(words: &[&str]) -> bool {
    let Some(attack_idx) = find_word_matching_shape(words, &ATTACK_OR_ATTACKS_WORD_PATTERN) else {
        return false;
    };
    ATTACK_OR_BLOCK_IF_ABLE_TAIL_PATTERN.matches_words(&words[attack_idx..])
}

fn is_attack_this_turn_if_able_clause_words_lexed(words: &[&str]) -> bool {
    let Some(attack_idx) = find_word_matching_shape(words, &ATTACK_OR_ATTACKS_WORD_PATTERN) else {
        return false;
    };
    ATTACK_IF_ABLE_TAIL_PATTERN.matches_words(&words[attack_idx..])
}

fn is_must_block_if_able_clause_words_lexed(words: &[&str]) -> bool {
    if matches!(
        words,
        ["all", "creatures", "able", "to", "block", .., "do", "so"]
    ) {
        return true;
    }

    let Some(block_idx) = find_word_matching_shape(words, &BLOCK_OR_BLOCKS_WORD_PATTERN) else {
        return false;
    };
    if block_idx == 0 || block_idx + 1 >= words.len() {
        return false;
    }

    let tail = &words[block_idx..];
    BLOCK_IF_ABLE_TAIL_PATTERN.matches_words(tail) || IF_ABLE_SUFFIX_PATTERN.matches_words(tail)
}

fn is_phase_clause_words_lexed(words: &[&str]) -> bool {
    PHASE_WORD_TAIL_PATTERN.matches_words(words) && words.len() >= 3
}

fn is_choose_target_prelude_clause_words_lexed(words: &[&str]) -> bool {
    CHOOSE_TARGET_PRELUDE_PATTERN.matches_words(words)
}

fn should_keep_and_for_power_toughness_axis_lexed(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    let current_words = token_word_refs(current);
    let remaining_words = token_word_refs(remaining);
    POWER_TOUGHNESS_AXIS_PATTERN.matches_words(&current_words)
        && TOUGHNESS_WORD_PATTERN.matches_first_word(&remaining_words)
}

fn should_keep_and_for_become_with_quoted_ability(
    current: &[OwnedLexToken],
    remaining: &[OwnedLexToken],
) -> bool {
    let current_words = token_word_refs(current);
    if !BECOME_WITH_QUOTED_ABILITY_CONTEXT_PATTERN.matches_words(&current_words) {
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
        .any(|word| SHARED_SUBJECT_MODIFIER_WORD_PATTERN.matches_word(word));
    if !has_shared_subject_modifier {
        return false;
    }
    remaining
        .iter()
        .find_map(OwnedLexToken::as_word)
        .is_some_and(|word| GAIN_HAVE_LOSE_WORD_PATTERN.matches_word(word))
}

pub(crate) fn split_effect_chain_on_and_lexed(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut start = 0usize;

    for (idx, token) in tokens.iter().enumerate() {
        if !AND_WORD_PATTERN.matches_token(token) {
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
    if REPEAT_THIS_PROCESS_PATTERN.matches_words(&token_words) {
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
}

pub(crate) fn has_effect_head_without_verb_lexed(tokens: &[OwnedLexToken]) -> bool {
    let token_words = token_word_refs(tokens);
    if REPEAT_THIS_PROCESS_PATTERN.matches_words(&token_words) {
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
                        .is_some_and(|token| THEN_WORD_PATTERN.matches_token(token))
                    || THEN_WORD_PATTERN.matches_token(&segment[i]))
            {
                let then_idx = if THEN_WORD_PATTERN.matches_token(&segment[i]) {
                    i
                } else {
                    i + 1
                };
                let before_then = trim_lexed_commas(&segment[..i]);
                let starts_with_clash =
                    grammar::words_match_any_prefix(before_then, CLASH_PREFIXES).is_some();
                let after_then = trim_lexed_commas(&segment[then_idx + 1..]);
                let after_words = token_word_refs(after_then);
                let has_back_ref = BACK_REFERENCE_WORD_PATTERN.matches_words(&after_words);
                let has_nonverb_effect_head =
                    NONVERB_EFFECT_HEAD_WORD_PATTERN.matches_first_word(&after_words);
                let has_effect_head = find_verb_lexed(after_then).is_some()
                    || parse_ability_line_lexed(after_then).is_some()
                    || has_nonverb_effect_head;
                let allow_backref_split = has_back_ref
                    && PUT_OR_DOUBLE_COUNTER_FOLLOWUP_PATTERN.matches_words(&after_words);
                let allow_attach_followup =
                    ATTACH_OR_ATTACHES_WORD_PATTERN.matches_first_word(&after_words);
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
                        && DEAL_DAMAGE_EQUAL_TOTAL_MANA_VALUE_PATTERN.matches_words(&after_words);
                let allow_for_each_damage_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, GENERIC_FOR_EACH_PREFIXES)
                        .is_some()
                    && DEAL_DAMAGE_FOLLOWUP_PATTERN.matches_words(&after_words);
                let allow_return_with_counter_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && RETURN_WITH_COUNTER_FOLLOWUP_PATTERN.matches_words(&after_words)
                    && BACKREF_COUNTER_TARGET_MARKER_PATTERN.matches_words(&after_words);
                let allow_put_battlefield_with_counter_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && grammar::words_match_any_prefix(after_then, PUT_PREFIXES).is_some()
                        && grammar::contains_word(after_then, "battlefield")
                        && after_words
                            .iter()
                            .any(|word| COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word(word))
                        && BACKREF_COUNTER_TARGET_MARKER_PATTERN.matches_words(&after_words);
                let allow_put_into_hand_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, PUT_PREFIXES).is_some()
                    && grammar::contains_word(after_then, "into")
                    && grammar::contains_word(after_then, "hand");
                let allow_put_back_in_any_order_followup = has_back_ref
                    && grammar::words_match_any_prefix(after_then, PUT_BACK_PREFIXES).is_some()
                    && grammar::contains_word(after_then, "any")
                    && grammar::contains_word(after_then, "order");
                let allow_clash_followup = starts_with_clash;
                if has_effect_head && (!has_back_ref || allow_backref_split)
                    || has_effect_head && allow_clash_followup
                    || has_effect_head && allow_attach_followup
                    || has_effect_head && allow_that_many_followup
                    || has_effect_head && allow_gain_or_lose_life_equal_followup
                    || has_effect_head && allow_deal_damage_equal_power_followup
                    || has_effect_head && allow_deal_damage_equal_total_mana_value_followup
                    || has_effect_head && allow_for_each_damage_followup
                    || has_effect_head && allow_return_with_counter_followup
                    || has_effect_head && allow_put_battlefield_with_counter_followup
                    || has_effect_head && allow_put_into_hand_followup
                    || has_effect_head && allow_put_back_in_any_order_followup
                {
                    split_point = Some(i);
                    break;
                }
            }
        }
        if let Some(idx) = split_point {
            let then_idx = if THEN_WORD_PATTERN.matches_token(&segment[idx]) {
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
            let duration_trigger_prefix = UNTIL_OR_DURING_WORD_PATTERN
                .matches_first_word(&before_words)
                && (grammar::contains_word(before, "whenever")
                    || grammar::contains_word(before, "when")
                    || AT_THE_MARKER_PATTERN.matches_words(&before_words));
            if UNLESS_WORD_PATTERN.matches_first_word(&before_words) || duration_trigger_prefix {
                continue;
            }
            if grammar::contains_word(before, "search") && grammar::contains_word(before, "library")
            {
                continue;
            }
            if grammar::contains_word(before, "target")
                && (TARGET_CARD_TYPE_WORD_PATTERN.matches_first_word(&after_words)
                    || (OR_WORD_PATTERN.matches_first_word(&after_words)
                        && TARGET_CARD_TYPE_WORD_PATTERN.matches_word_at(&after_words, 1)))
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

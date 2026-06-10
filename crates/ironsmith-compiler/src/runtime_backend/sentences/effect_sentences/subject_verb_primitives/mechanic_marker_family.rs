use super::*;
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom,
};

macro_rules! primitive_with_pattern_parser {
    ($id:literal, $priority:expr, $stage:ident, $hints:expr, $pattern_atoms:expr, $parser:expr, $pattern_parser:expr) => {
        SubjectVerbPrimitive::with_pattern_parser(
            $id,
            $priority,
            SubjectVerbPrimitiveStage::$stage,
            $hints,
            $pattern_atoms,
            $parser,
            $pattern_parser,
        )
    };
}

macro_rules! primitive_with_pattern {
    ($id:literal, $priority:expr, $stage:ident, $hints:expr, $pattern_atoms:expr, $parser:expr) => {
        SubjectVerbPrimitive::with_pattern(
            $id,
            $priority,
            SubjectVerbPrimitiveStage::$stage,
            $hints,
            $pattern_atoms,
            $parser,
        )
    };
}

const IMPLICIT_BECOME_HEADS: &[&[&str]] = &[
    &["it"],
    &["its"],
    &["it's"],
    &["it’s"],
    &["they"],
    &["they're"],
    &["they’re"],
    &["theyre"],
    &["this"],
    &["each"],
    &["it", "is"],
    &["they", "are"],
    &["this", "creature"],
    &["this", "permanent"],
    &["this", "land"],
    &["each", "of"],
];
const IMPLICIT_BECOME_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(IMPLICIT_BECOME_HEADS),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const FALLBACK_MECHANIC_MARKER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(MECHANIC_MARKER_PREFIXES),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const IF_TAGGED_CARDS_SHARE_TYPE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["if", "any", "of", "those", "cards"]),
    LexPattern::any_word(&["share", "shares"]),
    LexPattern::phrase(&["a", "card", "type", "with", "that", "spell"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const PUT_MULTIPLE_COUNTERS_ON_TARGET_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("put"),
    LexPattern::role_capture(
        "counters",
        LexCaptureRole::Object,
        LexCaptureKind::UntilLastPhrase(&["on"]),
    ),
    LexPattern::word("on"),
    LexPattern::role_capture(
        "target",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const TARGET_PLAYER_CHOOSES_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["target", "player"]),
    LexPattern::any_word(&["choose", "chooses"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const COUNTER_TARGET_SPELL_TURN_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["counter", "target", "spell"]),
    LexPattern::role_capture("condition", LexCaptureRole::Condition, LexCaptureKind::Rest),
];
const EXILE_TARGET_CREATURE_GREATEST_POWER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["exile", "target", "creature"]),
    LexPattern::role_capture("condition", LexCaptureRole::Condition, LexCaptureKind::Rest),
];
const COMMA_THEN_CHAIN_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const DRAW_THEN_CONNIVE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "draw_subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["draw"], &["draws"]]),
    ),
    LexPattern::any_word(&["draw", "draws"]),
    LexPattern::role_capture(
        "draw_amount",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture(
        "connive_subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["connive"], &["connives"]]),
    ),
    LexPattern::any_word(&["connive", "connives"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const GETS_THEN_FIGHTS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["get"], &["gets"]]),
    ),
    LexPattern::any_word(&["get", "gets"]),
    LexPattern::role_capture(
        "pump",
        LexCaptureRole::Modifier,
        LexCaptureKind::UntilAnyPhrase(&[&["fight"], &["fights"]]),
    ),
    LexPattern::any_word(&["fight", "fights"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const TRANSFORM_WITH_FOLLOWUP_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_word(&["transform", "convert"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const CANT_EFFECT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("cant"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const COMPOUND_DAMAGE_FANOUT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "source",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["deal"], &["deals"]]),
    ),
    LexPattern::any_word(&["deal", "deals"]),
    LexPattern::role_capture("damage", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const TARGET_AND_EACH_OTHER_FANOUT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "head",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(&[&["and", "each", "other"], &["and", "all", "other"]]),
    ),
    LexPattern::any_phrase(&[&["and", "each", "other"], &["and", "all", "other"]]),
    LexPattern::role_capture("fanout_target", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const TARGET_PLAYER_EXILES_CREATURE_GRAVEYARD_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(&[&["target", "player"], &["target", "opponent"]]),
    LexPattern::any_word(&["exile", "exiles"]),
    LexPattern::role_capture(
        "creature_clause",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(&[&["graveyard"], &["graveyards"]]),
    ),
    LexPattern::any_word(&["graveyard", "graveyards"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const SAME_NAME_GETS_FANOUT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["get"], &["gets"]]),
    ),
    LexPattern::any_word(&["get", "gets"]),
    LexPattern::role_capture("modifier", LexCaptureRole::Modifier, LexCaptureKind::Rest),
];
const GAIN_X_PLUS_LIFE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("gain"),
    LexPattern::role_capture(
        "base_amount",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilPhrase(&["plus"]),
    ),
    LexPattern::word("plus"),
    LexPattern::role_capture("bonus", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const GAIN_LIFE_EQUAL_TO_AGE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["gain", "life"]),
    LexPattern::role_capture(
        "amount",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilAnyPhrase(&[&["age", "counter"], &["age", "counters"]]),
    ),
    LexPattern::any_phrase(&[&["age", "counter"], &["age", "counters"]]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const DELAYED_NEXT_END_STEP_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["at", "the", "beginning", "of"]),
    LexPattern::role_capture(
        "step_owner",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["next", "end", "step"]),
    ),
    LexPattern::phrase(&["next", "end", "step"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const SEARCH_LIBRARY_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("search"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const EXILE_THEN_SHUFFLE_GRAVEYARD_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("exile"),
    LexPattern::role_capture(
        "exile_clause",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["then", "shuffle"]),
    ),
    LexPattern::phrase(&["then", "shuffle"]),
    LexPattern::role_capture("shuffle_clause", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const SHUFFLE_OBJECT_INTO_LIBRARY_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["shuffle"], &["shuffles"]]),
    ),
    LexPattern::any_word(&["shuffle", "shuffles"]),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["into"]),
    ),
    LexPattern::word("into"),
    LexPattern::role_capture("library", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const SHUFFLE_GRAVEYARD_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["shuffle"], &["shuffles"]]),
    ),
    LexPattern::any_word(&["shuffle", "shuffles"]),
    LexPattern::role_capture(
        "graveyard",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["into"]),
    ),
    LexPattern::word("into"),
    LexPattern::role_capture("library", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const EXILE_HAND_GRAVEYARD_BUNDLE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("exile"),
    LexPattern::role_capture(
        "hand_clause",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(&[&["graveyard"], &["graveyards"]]),
    ),
    LexPattern::any_word(&["graveyard", "graveyards"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const LOOK_AT_TOP_THEN_EXILE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["look", "at"]),
    LexPattern::role_capture(
        "look_clause",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["exile"]),
    ),
    LexPattern::word("exile"),
    LexPattern::role_capture("exile_clause", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const LOOK_AT_HAND_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["look", "at"]),
    LexPattern::role_capture(
        "player",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(&[&["hand"], &["hands"]]),
    ),
    LexPattern::any_word(&["hand", "hands"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const FOR_OR_EACH_PLAYER_DOESNT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(&[&["for"], &["then"], &["each"]]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const DELAYED_TRIGGER_HEADS: &[&[&str]] = &[&["if"], &["this"], &["when"], &["whenever"]];
const DELAYED_TRIGGER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(DELAYED_TRIGGER_HEADS),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const DESTROY_OR_EXILE_ALL_SPLIT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_word(&["destroy", "exile"]),
    LexPattern::word("all"),
    LexPattern::role_capture("objects", LexCaptureRole::Object, LexCaptureKind::Rest),
];
const EXILE_UP_TO_ONE_EACH_TARGET_TYPE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["exile", "up", "to", "one", "target"]),
    LexPattern::role_capture("targets", LexCaptureRole::Object, LexCaptureKind::Rest),
];
const EXILE_MULTI_TARGET_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("exile"),
    LexPattern::role_capture(
        "prefix",
        LexCaptureRole::Modifier,
        LexCaptureKind::UntilPhrase(&["target"]),
    ),
    LexPattern::word("target"),
    LexPattern::role_capture("targets", LexCaptureRole::Object, LexCaptureKind::Rest),
];
const DESTROY_MULTI_TARGET_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("destroy"),
    LexPattern::role_capture(
        "prefix",
        LexCaptureRole::Modifier,
        LexCaptureKind::UntilPhrase(&["target"]),
    ),
    LexPattern::word("target"),
    LexPattern::role_capture("targets", LexCaptureRole::Object, LexCaptureKind::Rest),
];

const EACH_OPPONENT_LOSES_X_AND_YOU_GAIN_X_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(EACH_OPPONENT_PREFIXES),
    LexPattern::role_capture(
        "drain",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["where", "x", "is"]),
    ),
    LexPattern::phrase(&["where", "x", "is"]),
    LexPattern::role_capture("where_value", LexCaptureRole::Amount, LexCaptureKind::Rest),
];
const SACRIFICE_AT_END_OF_COMBAT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("sacrifice"),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(END_OF_COMBAT_TIMING_PHRASES),
    ),
    LexPattern::any_phrase(END_OF_COMBAT_TIMING_PHRASES),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const OPTIONAL_THE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
const SACRIFICE_NEXT_END_STEP_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("sacrifice"),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["at", "the", "beginning", "of"]),
    ),
    LexPattern::phrase(&["at", "the", "beginning", "of"]),
    LexPattern::optional(OPTIONAL_THE_PATTERN_ATOMS),
    LexPattern::phrase(&["next", "end", "step"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const REMAIN_EXILED_ANY_OF_THOSE_CARDS_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["if", "any", "of", "those", "cards"]),
    LexPattern::phrase(&["remain", "exiled"]),
];
const REMAIN_EXILED_THOSE_CARDS_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["if", "those", "cards"]),
    LexPattern::phrase(&["remain", "exiled"]),
];
const REMAIN_EXILED_THAT_CARD_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["if", "that", "card"]),
    LexPattern::phrase(&["remains", "exiled"]),
];
const REMAIN_EXILED_IT_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["if", "it"]),
    LexPattern::phrase(&["remains", "exiled"]),
];
const REMAIN_EXILED_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    REMAIN_EXILED_ANY_OF_THOSE_CARDS_SEQUENCE,
    REMAIN_EXILED_THOSE_CARDS_SEQUENCE,
    REMAIN_EXILED_THAT_CARD_SEQUENCE,
    REMAIN_EXILED_IT_SEQUENCE,
];
const REMAIN_EXILED_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_sequence(REMAIN_EXILED_SEQUENCES),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const OPTIONAL_LEADING_CONNECTOR_PATTERN_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::any_word(&["and", "then"])];
const SHARED_DRAW_OPTIONAL_EACH: &[LexPatternAtom<'static>] = &[LexPattern::word("each")];
const SHARED_DRAW_ACTION_BOUNDARIES: &[&[&str]] = &[&["each"], &["draw"], &["draws"]];
const SHARED_DRAW_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::optional(OPTIONAL_LEADING_CONNECTOR_PATTERN_ATOMS),
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["and"]),
    ),
    LexPattern::word("and"),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(SHARED_DRAW_ACTION_BOUNDARIES),
    ),
    LexPattern::optional(SHARED_DRAW_OPTIONAL_EACH),
    LexPattern::any_word(&["draw", "draws"]),
    LexPattern::role_capture(
        "amount",
        LexCaptureRole::Amount,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const SHARED_LIFE_ACTION_BOUNDARIES: &[&[&str]] =
    &[&["each"], &["gain"], &["gains"], &["lose"], &["loses"]];
const SHARED_LIFE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::optional(OPTIONAL_LEADING_CONNECTOR_PATTERN_ATOMS),
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilPhrase(&["and"]),
    ),
    LexPattern::word("and"),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(SHARED_LIFE_ACTION_BOUNDARIES),
    ),
    LexPattern::optional(SHARED_DRAW_OPTIONAL_EACH),
    LexPattern::role_capture(
        "verb",
        LexCaptureRole::Action,
        LexCaptureKind::OneOf(&["gain", "gains", "lose", "loses"]),
    ),
    LexPattern::role_capture(
        "amount",
        LexCaptureRole::Amount,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const CHOOSE_PLAYER_TO_EFFECT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::optional(OPTIONAL_LEADING_CONNECTOR_PATTERN_ATOMS),
    LexPattern::role_capture(
        "action",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["to"]),
    ),
    LexPattern::word("to"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
const YOU_AND_ATTACKING_PLAYER_DRAW_LOSE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("you"),
    LexPattern::word("and"),
    LexPattern::optional(OPTIONAL_THE_PATTERN_ATOMS),
    LexPattern::phrase(&["attacking", "player"]),
    LexPattern::optional(SHARED_DRAW_OPTIONAL_EACH),
    LexPattern::any_word(&["draw", "draws"]),
    LexPattern::role_capture(
        "draw_amount",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilPhrase(&["and"]),
    ),
    LexPattern::word("and"),
    LexPattern::any_word(&["lose", "loses"]),
    LexPattern::role_capture(
        "lose_amount",
        LexCaptureRole::Modifier,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const OWNER_WORDS: &[&str] = &["owner's", "owners'", "owners", "owner"];
const HAND_WORDS: &[&str] = &["hand", "hands"];
const RETURN_HALF_CONTROLLED_TO_HAND_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::optional(OPTIONAL_LEADING_CONNECTOR_PATTERN_ATOMS),
    LexPattern::phrase(&["return", "half", "the"]),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["they", "control"]),
    ),
    LexPattern::phrase(&["they", "control", "to", "their"]),
    LexPattern::any_word(OWNER_WORDS),
    LexPattern::any_word(HAND_WORDS),
    LexPattern::phrase(&["rounded", "up"]),
];
const DEAL_WORDS: &[&str] = &["deal", "deals"];
const HALF_DAMAGE_FROM_THOSE_SPELLS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::optional(OPTIONAL_LEADING_CONNECTOR_PATTERN_ATOMS),
    LexPattern::role_capture(
        "source",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(&[&["deal"], &["deals"]]),
    ),
    LexPattern::any_word(DEAL_WORDS),
    LexPattern::phrase(&[
        "damage", "to", "that", "player", "equal", "to", "half", "the", "damage", "dealt", "by",
        "one", "of", "those",
    ]),
    LexPattern::role_capture(
        "card_type",
        LexCaptureRole::Object,
        LexCaptureKind::WordCount(1),
    ),
    LexPattern::phrase(&["spells", "this", "turn", "rounded", "down"]),
];
const DRAW_EXILED_HAND_ACTION_PHRASES: &[&[&str]] =
    &[&["shuffles", "then", "draws"], &["draw"], &["draws"]];
const HAND_OWNER_WORDS: &[&str] = &["your", "their"];
const DRAW_FOR_EACH_EXILED_HAND_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::optional(OPTIONAL_LEADING_CONNECTOR_PATTERN_ATOMS),
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(DRAW_EXILED_HAND_ACTION_PHRASES),
    ),
    LexPattern::any_phrase(DRAW_EXILED_HAND_ACTION_PHRASES),
    LexPattern::phrase(&["a", "card", "for", "each", "card", "exiled", "from"]),
    LexPattern::role_capture(
        "hand_owner",
        LexCaptureRole::Object,
        LexCaptureKind::OneOf(HAND_OWNER_WORDS),
    ),
    LexPattern::phrase(&["hand", "this", "way"]),
];
pub(crate) const PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES: &[SubjectVerbPrimitive] = &[
    primitive_with_pattern!(
        "implicit-become-clause",
        10,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("its"),
            LexRuleHeadHint::Single("it's"),
            LexRuleHeadHint::Single("it’s"),
            LexRuleHeadHint::Single("they"),
            LexRuleHeadHint::Single("they're"),
            LexRuleHeadHint::Single("they’re"),
            LexRuleHeadHint::Single("theyre"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("each"),
            LexRuleHeadHint::Pair("it", "is"),
            LexRuleHeadHint::Pair("they", "are"),
            LexRuleHeadHint::Pair("this", "creature"),
            LexRuleHeadHint::Pair("this", "permanent"),
            LexRuleHeadHint::Pair("this", "land"),
            LexRuleHeadHint::Pair("each", "of"),
        ],
        IMPLICIT_BECOME_PATTERN_ATOMS,
        parse_sentence_implicit_become_clause
    ),
    primitive_with_pattern!(
        "fallback-mechanic-marker",
        20,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("stand"),
            LexRuleHeadHint::Single("it"),
        ],
        FALLBACK_MECHANIC_MARKER_PATTERN_ATOMS,
        parse_sentence_fallback_mechanic_marker
    ),
    primitive_with_pattern_parser!(
        "target-gains-or-loses-all-creature-types",
        25,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("target"),
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("that")
        ],
        GAINS_OR_LOSES_ALL_CREATURE_TYPES_PATTERN_ATOMS,
        parse_sentence_gains_or_loses_all_creature_types,
        parse_sentence_gains_or_loses_all_creature_types_matched
    ),
    primitive_with_pattern!(
        "pump-creature-type-of-choice-pre",
        26,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("creatures"),
            LexRuleHeadHint::Single("target"),
        ],
        PUMP_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS,
        parse_sentence_pump_creature_type_of_choice
    ),
    primitive_with_pattern_parser!(
        "lose-draw-clash-repeat-process",
        27,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("you")],
        LOSE_DRAW_CLASH_REPEAT_PATTERN_ATOMS,
        parse_sentence_lose_draw_clash_repeat_process,
        parse_sentence_lose_draw_clash_repeat_process_matched
    ),
    primitive_with_pattern_parser!(
        "if-sacrifice-then-put-onto-battlefield-with-additional-counters",
        30,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        IF_SACRIFICE_THEN_PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS,
        parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence,
        parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "if-tagged-cards-remain-exiled",
        40,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        REMAIN_EXILED_PATTERN_ATOMS,
        parse_sentence_if_tagged_cards_remain_exiled,
        parse_sentence_if_tagged_cards_remain_exiled_matched
    ),
    primitive_with_pattern_parser!(
        "if-enters-with-additional-counter",
        50,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        IF_ENTERS_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS,
        parse_if_enters_with_additional_counter_sentence,
        parse_if_enters_with_additional_counter_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "tagged-enters-with-additional-counter",
        52,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("all"),
            LexRuleHeadHint::Single("each"),
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("that"),
        ],
        TAGGED_ENTERS_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS,
        parse_tagged_enters_with_additional_counter_sentence,
        parse_tagged_enters_with_additional_counter_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "if-any-tagged-cards-share-card-type-with-triggering-spell",
        55,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("if")],
        IF_TAGGED_CARDS_SHARE_TYPE_PATTERN_ATOMS,
        parse_if_any_tagged_cards_share_card_type_with_triggering_spell,
        parse_if_any_tagged_cards_share_card_type_with_triggering_spell_matched
    ),
    primitive_with_pattern_parser!(
        "put-onto-battlefield-with-additional-counters",
        60,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("put")],
        PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS,
        parse_put_onto_battlefield_with_additional_counters_sentence,
        parse_put_onto_battlefield_with_additional_counters_sentence_matched
    ),
    primitive_with_pattern!(
        "put-multiple-counters-on-target",
        70,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("put")],
        PUT_MULTIPLE_COUNTERS_ON_TARGET_PATTERN_ATOMS,
        parse_sentence_put_multiple_counters_on_target
    ),
    primitive_with_pattern_parser!(
        "put-sticker-on",
        80,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("put"),
            LexRuleHeadHint::Single("puts"),
        ],
        PUT_STICKER_ON_PATTERN_ATOMS,
        parse_sentence_put_sticker_on,
        parse_sentence_put_sticker_on_matched
    ),
    primitive_with_pattern_parser!(
        "you-and-target-player-each-draw",
        90,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        SHARED_DRAW_PATTERN_ATOMS,
        parse_sentence_you_and_target_player_each_draw,
        parse_you_and_target_player_each_draw_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "you-and-player-each-gain-or-lose-life",
        95,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        SHARED_LIFE_PATTERN_ATOMS,
        parse_sentence_you_and_player_each_gain_or_lose_life,
        parse_you_and_player_each_gain_or_lose_life_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "choose-player-to-effect",
        100,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("choose"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        CHOOSE_PLAYER_TO_EFFECT_PATTERN_ATOMS,
        parse_sentence_choose_player_to_effect,
        parse_sentence_choose_player_to_effect_matched
    ),
    primitive_with_pattern_parser!(
        "you-and-attacking-player-each-draw-and-lose",
        110,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("you")],
        YOU_AND_ATTACKING_PLAYER_DRAW_LOSE_PATTERN_ATOMS,
        parse_sentence_you_and_attacking_player_each_draw_and_lose,
        parse_sentence_you_and_attacking_player_each_draw_and_lose_matched
    ),
    primitive_with_pattern_parser!(
        "sacrifice-then-put-onto-battlefield-with-additional-counters",
        120,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        SACRIFICE_THEN_PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS,
        parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence,
        parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "sacrifice-it-next-end-step",
        130,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        SACRIFICE_NEXT_END_STEP_PATTERN_ATOMS,
        parse_sentence_sacrifice_it_next_end_step,
        parse_sentence_sacrifice_it_next_end_step_matched
    ),
    primitive_with_pattern_parser!(
        "sacrifice-at-end-of-combat",
        140,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        SACRIFICE_AT_END_OF_COMBAT_PATTERN_ATOMS,
        parse_sentence_sacrifice_at_end_of_combat,
        parse_sentence_sacrifice_at_end_of_combat_matched
    ),
    primitive_with_pattern!(
        "target-player-choose-then-put-on-top-library",
        160,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        TARGET_PLAYER_CHOOSES_PATTERN_ATOMS,
        parse_sentence_target_player_chooses_then_puts_on_top_of_library
    ),
    primitive_with_pattern!(
        "target-player-choose-then-you-put-it-onto-battlefield",
        170,
        PreDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        TARGET_PLAYER_CHOOSES_PATTERN_ATOMS,
        parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield
    ),
    primitive_with_pattern_parser!(
        "target-player-reveals-random-card-from-hand",
        180,
        PreDiagnostic,
        &[
            LexRuleHeadHint::Single("target"),
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("opponent"),
            LexRuleHeadHint::Single("that"),
        ],
        TARGET_PLAYER_REVEALS_RANDOM_CARD_FROM_HAND_PATTERN_ATOMS,
        parse_sentence_target_player_reveals_random_card_from_hand,
        parse_sentence_target_player_reveals_random_card_from_hand_matched
    ),
];

pub(crate) static PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX: LazyLock<LexRuleHintIndex> =
    LazyLock::new(|| {
        build_lex_rule_hint_index(PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES.len(), |idx| {
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES[idx]
                .head_hints
                .to_vec()
        })
    });

pub(crate) const POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES: &[SubjectVerbPrimitive] = &[
    primitive_with_pattern!(
        "exile-target-creature-with-greatest-power",
        10,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        EXILE_TARGET_CREATURE_GREATEST_POWER_PATTERN_ATOMS,
        parse_sentence_exile_target_creature_with_greatest_power
    ),
    primitive_with_pattern!(
        "counter-target-spell-thats-second-cast-this-turn",
        20,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("counter")],
        COUNTER_TARGET_SPELL_TURN_PATTERN_ATOMS,
        parse_sentence_counter_target_spell_thats_second_cast_this_turn
    ),
    primitive_with_pattern!(
        "counter-target-spell-if-it-was-kicked",
        30,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("counter")],
        COUNTER_TARGET_SPELL_TURN_PATTERN_ATOMS,
        parse_sentence_counter_target_spell_if_it_was_kicked
    ),
    primitive_with_pattern_parser!(
        "return-half-the-creatures-they-control-to-their-owners-hand",
        40,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("return"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        RETURN_HALF_CONTROLLED_TO_HAND_PATTERN_ATOMS,
        parse_sentence_return_half_the_creatures_they_control_to_their_owners_hand,
        parse_sentence_return_half_the_creatures_they_control_to_their_owners_hand_matched
    ),
    primitive_with_pattern_parser!(
        "destroy-creature-type-of-choice",
        50,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        DESTROY_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS,
        parse_sentence_destroy_creature_type_of_choice,
        parse_sentence_destroy_creature_type_of_choice_matched
    ),
    primitive_with_pattern_parser!(
        "pump-creature-type-of-choice",
        60,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("creatures"),
            LexRuleHeadHint::Single("target"),
        ],
        PUMP_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS,
        parse_sentence_pump_creature_type_of_choice,
        parse_sentence_pump_creature_type_of_choice_matched
    ),
    primitive_with_pattern_parser!(
        "must-attack-creature-type-of-choice",
        65,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("creatures")],
        MUST_ATTACK_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS,
        parse_sentence_must_attack_creature_type_of_choice,
        parse_sentence_must_attack_creature_type_of_choice_matched
    ),
    primitive_with_pattern_parser!(
        "return-multiple-targets",
        70,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        RETURN_MULTIPLE_TARGETS_PATTERN_ATOMS,
        parse_sentence_return_multiple_targets,
        parse_sentence_return_multiple_targets_matched
    ),
    primitive_with_pattern!(
        "choose-all-battlefield-graveyard-to-hand",
        80,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("choose")],
        CHOOSE_ALL_BATTLEFIELD_GRAVEYARD_PATTERN_ATOMS,
        parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand
    ),
    primitive_with_pattern!(
        "for-each-of-target-objects",
        90,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        FOR_EACH_TARGET_OBJECTS_PATTERN_ATOMS,
        parse_sentence_for_each_of_target_objects
    ),
    primitive_with_pattern_parser!(
        "return-creature-type-of-choice",
        100,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        RETURN_TARGETS_OF_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS,
        parse_sentence_return_targets_of_creature_type_of_choice,
        parse_sentence_return_targets_of_creature_type_of_choice_matched
    ),
    primitive_with_pattern!(
        "distribute-counters",
        110,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("distribute")],
        DISTRIBUTE_COUNTERS_PATTERN_ATOMS,
        parse_sentence_distribute_counters
    ),
    primitive_with_pattern_parser!(
        "keyword-then-chain",
        120,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        THEN_CHAIN_PATTERN_ATOMS,
        parse_sentence_keyword_then_chain,
        parse_sentence_keyword_then_chain_matched
    ),
    primitive_with_pattern_parser!(
        "chain-then-keyword",
        130,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        THEN_CHAIN_PATTERN_ATOMS,
        parse_sentence_chain_then_keyword,
        parse_sentence_chain_then_keyword_matched
    ),
    primitive_with_pattern_parser!(
        "exile-then-may-put-from-exile",
        140,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        EXILE_THEN_MAY_PUT_FROM_EXILE_PATTERN_ATOMS,
        parse_sentence_exile_then_may_put_from_exile,
        parse_sentence_exile_then_may_put_from_exile_matched
    ),
    primitive_with_pattern!(
        "exile-then-shuffle-graveyard-into-library",
        150,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        EXILE_THEN_SHUFFLE_GRAVEYARD_PATTERN_ATOMS,
        parse_exile_then_shuffle_graveyard_into_library_sentence
    ),
    primitive_with_pattern!(
        "exile-source-with-counters",
        160,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        EXILE_SOURCE_WITH_COUNTERS_PATTERN_ATOMS,
        parse_sentence_exile_source_with_counters
    ),
    primitive_with_pattern_parser!(
        "destroy-all-attached-to-target",
        170,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        DESTROY_ALL_ATTACHED_TO_TARGET_PATTERN_ATOMS,
        parse_sentence_destroy_all_attached_to_target,
        parse_sentence_destroy_all_attached_to_target_matched
    ),
    primitive_with_pattern!(
        "comma-then-chain-special",
        180,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        COMMA_THEN_CHAIN_PATTERN_ATOMS,
        parse_sentence_comma_then_chain_special
    ),
    primitive_with_pattern!(
        "destroy-then-land-controller-graveyard-count-damage",
        190,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        DESTROY_THEN_LAND_GRAVEYARD_DAMAGE_PATTERN_ATOMS,
        parse_sentence_destroy_then_land_controller_graveyard_count_damage
    ),
    primitive_with_pattern!(
        "draw-then-connive",
        200,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("draw")],
        DRAW_THEN_CONNIVE_PATTERN_ATOMS,
        parse_sentence_draw_then_connive
    ),
    primitive_with_pattern_parser!(
        "choose-then-do-same-for-filter",
        210,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("choose")],
        CHOOSE_THEN_DO_SAME_FOR_FILTER_PATTERN_ATOMS,
        parse_sentence_choose_then_do_same_for_filter,
        parse_choose_then_do_same_for_filter_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "choose-then-choose-objects",
        215,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("choose"),
            LexRuleHeadHint::Pair("you", "choose"),
        ],
        CHOOSE_THEN_CHOOSE_OBJECTS_PATTERN_ATOMS,
        parse_sentence_choose_then_choose_objects,
        parse_choose_then_choose_objects_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "return-then-do-same-for-subtypes",
        220,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        RETURN_THEN_DO_SAME_FOR_SUBTYPES_PATTERN_ATOMS,
        parse_sentence_return_then_do_same_for_subtypes,
        parse_return_then_do_same_for_subtypes_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "return-then-create",
        230,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("return")],
        RETURN_THEN_CREATE_PATTERN_ATOMS,
        parse_sentence_return_then_create,
        parse_sentence_return_then_create_matched
    ),
    primitive_with_pattern_parser!(
        "put-counter-sequence",
        240,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("put")],
        PUT_COUNTER_SEQUENCE_PATTERN_ATOMS,
        parse_sentence_put_counter_sequence,
        parse_sentence_put_counter_sequence_matched
    ),
    primitive_with_pattern!(
        "gets-then-fights",
        250,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("gets")],
        GETS_THEN_FIGHTS_PATTERN_ATOMS,
        parse_sentence_gets_then_fights
    ),
    primitive_with_pattern_parser!(
        "return-with-counters-on-it",
        260,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("return"),
            LexRuleHeadHint::Single("then"),
        ],
        RETURN_WITH_COUNTERS_ON_IT_PATTERN_ATOMS,
        parse_sentence_return_with_counters_on_it,
        parse_return_with_counters_on_it_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "each-player-return-with-additional-counter",
        270,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        EACH_PLAYER_RETURN_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS,
        parse_sentence_each_player_return_with_additional_counter,
        parse_each_player_return_with_additional_counter_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "sacrifice-any-number",
        280,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        SACRIFICE_ANY_NUMBER_PATTERN_ATOMS,
        parse_sentence_sacrifice_any_number,
        parse_sacrifice_any_number_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "sacrifice-one-or-more",
        290,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("sacrifice")],
        SACRIFICE_ONE_OR_MORE_PATTERN_ATOMS,
        parse_sentence_sacrifice_one_or_more,
        parse_sacrifice_one_or_more_sentence_matched
    ),
    primitive_with_pattern_parser!(
        "for-each-counter-kind-put-or-remove",
        320,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        FOR_EACH_COUNTER_KIND_PUT_OR_REMOVE_PATTERN_ATOMS,
        parse_sentence_for_each_counter_kind_put_or_remove,
        parse_sentence_for_each_counter_kind_put_or_remove_matched
    ),
    primitive_with_pattern!(
        "transform-with-followup",
        350,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("transform"),
            LexRuleHeadHint::Single("convert"),
        ],
        TRANSFORM_WITH_FOLLOWUP_PATTERN_ATOMS,
        parse_sentence_transform_with_followup
    ),
    primitive_with_pattern!(
        "cant-effect",
        370,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("cant")],
        CANT_EFFECT_PATTERN_ATOMS,
        parse_sentence_cant_effect
    ),
    primitive_with_pattern!(
        "compound-damage-fanout",
        380,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("deal"),
            LexRuleHeadHint::Single("deals"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("target"),
        ],
        COMPOUND_DAMAGE_FANOUT_PATTERN_ATOMS,
        parse_sentence_compound_damage_fanout
    ),
    primitive_with_pattern!(
        "shared-color-target-fanout",
        390,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("target"),
            LexRuleHeadHint::Pair("target", "radiance"),
        ],
        TARGET_AND_EACH_OTHER_FANOUT_PATTERN_ATOMS,
        parse_sentence_shared_color_target_fanout
    ),
    primitive_with_pattern!(
        "gain-x-plus-life",
        440,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("gain")],
        GAIN_X_PLUS_LIFE_PATTERN_ATOMS,
        parse_sentence_gain_x_plus_life
    ),
    primitive_with_pattern_parser!(
        "for-each-exiled-this-way",
        450,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        FOR_EACH_THIS_WAY_PATTERN_ATOMS,
        parse_sentence_for_each_exiled_this_way,
        parse_sentence_for_each_exiled_this_way_matched
    ),
    primitive_with_pattern_parser!(
        "for-each-put-into-graveyard-this-way",
        460,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        FOR_EACH_THIS_WAY_PATTERN_ATOMS,
        parse_sentence_for_each_put_into_graveyard_this_way,
        parse_sentence_for_each_put_into_graveyard_this_way_matched
    ),
    primitive_with_pattern_parser!(
        "draw-for-each-card-exiled-from-hand-this-way",
        470,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("draw"),
            LexRuleHeadHint::Single("draws"),
            LexRuleHeadHint::Single("that"),
            LexRuleHeadHint::Single("you"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        DRAW_FOR_EACH_EXILED_HAND_PATTERN_ATOMS,
        parse_sentence_draw_for_each_card_exiled_from_hand_this_way,
        parse_draw_for_each_card_exiled_from_hand_this_way_sentence_matched
    ),
    primitive_with_pattern!(
        "each-player-reveals-top-count-put-permanents-rest-graveyard",
        480,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        EACH_PLAYER_REVEALS_TOP_PUT_PERMANENTS_PATTERN_ATOMS,
        parse_sentence_each_player_reveals_top_count_put_permanents_onto_battlefield_rest_graveyard
    ),
    primitive_with_pattern!(
        "each-player-put-permanent-cards-exiled-with-source",
        490,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        EACH_PLAYER_PUT_PERMANENT_CARDS_EXILED_PATTERN_ATOMS,
        parse_sentence_each_player_put_permanent_cards_exiled_with_source
    ),
    primitive_with_pattern_parser!(
        "for-each-destroyed-this-way",
        500,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("for")],
        FOR_EACH_THIS_WAY_PATTERN_ATOMS,
        parse_sentence_for_each_destroyed_this_way,
        parse_sentence_for_each_destroyed_this_way_matched
    ),
    primitive_with_pattern!(
        "delayed-next-step-unless-pays",
        510,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("at")],
        DELAYED_NEXT_STEP_UNLESS_PAYS_PATTERN_ATOMS,
        parse_sentence_delayed_next_step_unless_pays
    ),
    primitive_with_pattern!(
        "search-delayed-next-upkeep-unless-pays-lose-game",
        520,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("search")],
        SEARCH_DELAYED_NEXT_UPKEEP_LOSE_GAME_PATTERN_ATOMS,
        parse_sentence_delayed_next_upkeep_unless_pays_lose_game
    ),
    primitive_with_pattern!(
        "search-library",
        540,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("search")],
        SEARCH_LIBRARY_PATTERN_ATOMS,
        parse_sentence_search_library
    ),
    primitive_with_pattern!(
        "shuffle-graveyard-into-library",
        550,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("shuffle")],
        SHUFFLE_GRAVEYARD_PATTERN_ATOMS,
        parse_sentence_shuffle_graveyard_into_library
    ),
    primitive_with_pattern!(
        "shuffle-object-into-library",
        560,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("shuffle")],
        SHUFFLE_OBJECT_INTO_LIBRARY_PATTERN_ATOMS,
        parse_sentence_shuffle_object_into_library
    ),
    primitive_with_pattern!(
        "exile-hand-and-graveyard-bundle",
        570,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        EXILE_HAND_GRAVEYARD_BUNDLE_PATTERN_ATOMS,
        parse_sentence_exile_hand_and_graveyard_bundle
    ),
    primitive_with_pattern!(
        "target-player-exiles-creature-and-graveyard",
        580,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        TARGET_PLAYER_EXILES_CREATURE_GRAVEYARD_PATTERN_ATOMS,
        parse_sentence_target_player_exiles_creature_and_graveyard
    ),
    primitive_with_pattern!(
        "look-at-top-then-exile-one",
        600,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("look")],
        LOOK_AT_TOP_THEN_EXILE_PATTERN_ATOMS,
        parse_sentence_look_at_top_then_exile_one
    ),
    primitive_with_pattern!(
        "look-at-hand",
        610,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("look")],
        LOOK_AT_HAND_PATTERN_ATOMS,
        parse_sentence_look_at_hand
    ),
    primitive_with_pattern!(
        "gain-life-equal-to-age",
        620,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("gain")],
        GAIN_LIFE_EQUAL_TO_AGE_PATTERN_ATOMS,
        parse_sentence_gain_life_equal_to_age
    ),
    primitive_with_pattern!(
        "for-each-player-doesnt",
        630,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("for"),
            LexRuleHeadHint::Single("then"),
            LexRuleHeadHint::Single("each"),
        ],
        FOR_OR_EACH_PLAYER_DOESNT_PATTERN_ATOMS,
        parse_sentence_for_each_player_doesnt
    ),
    primitive_with_pattern_parser!(
        "each-opponent-loses-x-and-you-gain-x",
        650,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("each")],
        EACH_OPPONENT_LOSES_X_AND_YOU_GAIN_X_PATTERN_ATOMS,
        parse_sentence_each_opponent_loses_x_and_you_gain_x,
        parse_sentence_each_opponent_loses_x_and_you_gain_x_matched
    ),
    primitive_with_pattern!(
        "same-name-target-fanout",
        700,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        TARGET_AND_EACH_OTHER_FANOUT_PATTERN_ATOMS,
        parse_sentence_same_name_target_fanout
    ),
    primitive_with_pattern!(
        "same-name-gets-fanout",
        710,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("target")],
        SAME_NAME_GETS_FANOUT_PATTERN_ATOMS,
        parse_sentence_same_name_gets_fanout
    ),
    primitive_with_pattern!(
        "delayed-next-end-step",
        720,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("at")],
        DELAYED_NEXT_END_STEP_PATTERN_ATOMS,
        parse_sentence_delayed_until_next_end_step
    ),
    primitive_with_pattern!(
        "delayed-when-that-dies-this-turn",
        730,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("when")],
        DELAYED_TRIGGER_PATTERN_ATOMS,
        parse_delayed_when_that_dies_this_turn_sentence
    ),
    primitive_with_pattern!(
        "delayed-trigger-this-turn",
        740,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("if"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("when"),
            LexRuleHeadHint::Single("whenever"),
        ],
        DELAYED_TRIGGER_PATTERN_ATOMS,
        parse_sentence_delayed_trigger_this_turn
    ),
    primitive_with_pattern!(
        "destroy-or-exile-all-split",
        750,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        DESTROY_OR_EXILE_ALL_SPLIT_PATTERN_ATOMS,
        parse_sentence_destroy_or_exile_all_split
    ),
    primitive_with_pattern!(
        "exile-up-to-one-each-target-type",
        760,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        EXILE_UP_TO_ONE_EACH_TARGET_TYPE_PATTERN_ATOMS,
        parse_sentence_exile_up_to_one_each_target_type
    ),
    primitive_with_pattern!(
        "exile-multi-target",
        770,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("exile")],
        EXILE_MULTI_TARGET_PATTERN_ATOMS,
        parse_sentence_exile_multi_target
    ),
    primitive_with_pattern!(
        "destroy-multi-target",
        780,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("destroy")],
        DESTROY_MULTI_TARGET_PATTERN_ATOMS,
        parse_sentence_destroy_multi_target
    ),
    primitive_with_pattern_parser!(
        "reveal-selected-cards-in-your-hand",
        790,
        PostDiagnostic,
        &[LexRuleHeadHint::Single("reveal")],
        REVEAL_SELECTED_CARDS_IN_YOUR_HAND_PATTERN_ATOMS,
        parse_sentence_reveal_selected_cards_in_your_hand,
        parse_sentence_reveal_selected_cards_in_your_hand_matched
    ),
    primitive_with_pattern_parser!(
        "damage-unless-controller-has-source-deal-damage",
        800,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("damage"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("it"),
        ],
        DAMAGE_UNLESS_CONTROLLER_HAS_SOURCE_DEAL_DAMAGE_PATTERN_ATOMS,
        parse_sentence_damage_unless_controller_has_source_deal_damage,
        parse_sentence_damage_unless_controller_has_source_deal_damage_matched
    ),
    primitive_with_pattern_parser!(
        "damage-to-that-player-unless-enchanted-attacked",
        810,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("damage"),
            LexRuleHeadHint::Single("this"),
        ],
        DAMAGE_TO_THAT_PLAYER_UNLESS_ENCHANTED_ATTACKED_PATTERN_ATOMS,
        parse_sentence_damage_to_that_player_unless_enchanted_attacked,
        parse_sentence_damage_to_that_player_unless_enchanted_attacked_matched
    ),
    primitive_with_pattern_parser!(
        "damage-to-that-player-half-damage-of-those-spells",
        820,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("damage"),
            LexRuleHeadHint::Single("it"),
            LexRuleHeadHint::Single("this"),
            LexRuleHeadHint::Single("and"),
            LexRuleHeadHint::Single("then"),
        ],
        HALF_DAMAGE_FROM_THOSE_SPELLS_PATTERN_ATOMS,
        parse_sentence_damage_to_that_player_half_damage_of_those_spells,
        parse_sentence_damage_to_that_player_half_damage_of_those_spells_matched
    ),
    primitive_with_pattern_parser!(
        "unless-pays",
        830,
        PostDiagnostic,
        &[
            LexRuleHeadHint::Single("unless"),
            LexRuleHeadHint::Single("for"),
            LexRuleHeadHint::Single("each")
        ],
        UNLESS_PAYS_PATTERN_ATOMS,
        parse_sentence_unless_pays,
        parse_sentence_unless_pays_matched
    ),
];

pub(crate) static POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX: LazyLock<LexRuleHintIndex> =
    LazyLock::new(|| {
        build_lex_rule_hint_index(POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES.len(), |idx| {
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES[idx]
                .head_hints
                .to_vec()
        })
    });

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::util::tokenize_line;

    #[test]
    fn parse_sentence_implicit_become_clause_handles_explicit_self_negative_type_with_duration() {
        let tokens = tokenize_line("this creature isn't a creature until end of turn.", 0);
        let effects =
            parse_sentence_implicit_become_clause(SubjectVerbPrimitiveClause::new(&tokens))
                .expect("parse should succeed")
                .expect("clause should be recognized");

        assert!(
            matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::RemoveCardTypes {
                            target: TargetAst::Source(_),
                            card_types,
                            duration: Until::EndOfTurn,
                        },
                    ..
                })] if card_types.as_slice() == [CardType::Creature]
            ),
            "expected explicit self negative-type clause to parse into source-scoped remove-card-types until end of turn, got {effects:?}"
        );
    }

    #[test]
    fn sentence_primitive_metadata_sets_stage_and_hints() {
        assert!(
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
                .iter()
                .all(
                    |primitive| primitive.stage == SubjectVerbPrimitiveStage::PreDiagnostic
                        && !primitive.head_hints.is_empty()
                )
        );
        assert!(
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES
                .iter()
                .all(
                    |primitive| primitive.stage == SubjectVerbPrimitiveStage::PostDiagnostic
                        && !primitive.head_hints.is_empty()
                )
        );
    }
}

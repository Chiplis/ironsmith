use crate::host::{EffectAst, TriggerSpec};

fn anthem_token_offset(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| predicate(token))
}

fn anthem_last_token_offset(
    tokens: &[OwnedLexToken],
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    crate::runtime_backend::grammar::primitives::rfind_token_index(tokens, |token| predicate(token))
}

fn anthem_token_offset_from(
    tokens: &[OwnedLexToken],
    start: usize,
    mut predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    crate::runtime_backend::grammar::primitives::find_token_index(&tokens[start..], |token| {
        predicate(token)
    })
    .map(|offset| start + offset)
}

fn anthem_token_offset_between(
    tokens: &[OwnedLexToken],
    start: usize,
    end: usize,
    predicate: impl FnMut(&OwnedLexToken) -> bool,
) -> Option<usize> {
    if start >= end {
        return None;
    }
    anthem_token_offset_from(&tokens[..end], start, predicate)
}

fn anthem_index_where(limit: usize, mut predicate: impl FnMut(usize) -> bool) -> Option<usize> {
    for idx in 0..limit {
        if predicate(idx) {
            return Some(idx);
        }
    }
    None
}

fn anthem_last_index_where(
    limit: usize,
    mut predicate: impl FnMut(usize) -> bool,
) -> Option<usize> {
    let mut idx = limit;
    while idx > 0 {
        idx -= 1;
        if predicate(idx) {
            return Some(idx);
        }
    }
    None
}

fn anthem_cant_be_blocked_max_blockers(words: &[&str]) -> Option<(u32, usize)> {
    if words.get(0..4) != Some(&["cant", "be", "blocked", "by"]) {
        return None;
    }
    let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(&words[4..]);
    let (minimum_blockers, used) = parse_greater_than_or_equal_quantity_prefix(
        &quantity_tokens,
        false,
        false,
        "cant-be-blocked blocker threshold",
    )
    .ok()
    .flatten()?;
    if minimum_blockers == 0 {
        return None;
    }
    let noun_idx = 4 + used;
    words
        .get(noun_idx)
        .is_some_and(|word| CREATURE_OR_CREATURES_WORD_PATTERN.matches_word(word))
        .then_some((minimum_blockers - 1, noun_idx + 1))
}

type AnthemNormalizedWords<'a> = crate::runtime_backend::grammar::primitives::TokenWordView<'a>;

#[derive(Debug, Clone, Copy)]
struct CantBeBlockedAsLongAsClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
    condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct CantBeBlockedClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct KeywordsAndCantBeBlockedClause<'a> {
    keyword_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct LandwalkBlockOverrideClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
    ability_word: &'a str,
}

#[derive(Debug, Clone, Copy)]
struct GrantedEscapeCostTail<'a> {
    exile_count_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct GrantedMiracleCostReductionTail<'a> {
    reduction_cost_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct CantBeBlockedByMoreThanClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
    blocker_threshold_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy)]
struct CanBlockAdditionalCreatureClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
    additional_count_tokens: &'a [OwnedLexToken],
}

const POWER_OR_TOUGHNESS_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["power", "or", "toughness"], &["toughness", "or", "power"],]]
);

const FIRST_SPELL_EACH_TURN_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "first", "spell", "you", "cast", "each", "turn"],
            &["first", "spell", "you", "cast", "each", "turn"],
        ]
);

const CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "long", "as"]);
const CANT_BE_BLOCKED_PHRASES: &[&[&str]] = &[
    &["cant", "be", "blocked"],
    &["can't", "be", "blocked"],
    &["cannot", "be", "blocked"],
    &["can", "t", "be", "blocked"],
];
const CANT_BE_BLOCKED_AS_LONG_AS_PHRASES: &[&[&str]] = &[
    &["cant", "be", "blocked", "as", "long", "as"],
    &["can't", "be", "blocked", "as", "long", "as"],
    &["cannot", "be", "blocked", "as", "long", "as"],
    &["can", "t", "be", "blocked", "as", "long", "as"],
];
const AND_CANT_BE_BLOCKED_PHRASES: &[&[&str]] = &[
    &["and", "cant", "be", "blocked"],
    &["and", "can't", "be", "blocked"],
    &["and", "cannot", "be", "blocked"],
    &["and", "can", "t", "be", "blocked"],
];
const CAN_BE_BLOCKED_AS_THOUGH_NO_ABILITY_PHRASES: &[&[&str]] = &[
    &["can", "be", "blocked", "as", "though", "they", "didnt", "have"],
    &["can", "be", "blocked", "as", "though", "they", "didn't", "have"],
];
const CANT_BE_BLOCKED_BY_PHRASES: &[&[&str]] = &[
    &["cant", "be", "blocked", "by"],
    &["can't", "be", "blocked", "by"],
    &["cannot", "be", "blocked", "by"],
    &["can", "t", "be", "blocked", "by"],
];
const CAN_BLOCK_PHRASE: &[&str] = &["can", "block"];
const ADDITIONAL_CREATURE_TAIL_PHRASES: &[&[&str]] =
    &[&["additional", "creature"], &["additional", "creatures"]];
const CREATURE_NOUN_PHRASES: &[&[&str]] = &[&["creature"], &["creatures"]];
const EACH_COMBAT_PHRASE: &[&str] = &["each", "combat"];
const GRANTED_ESCAPE_COST_PREFIX_PHRASES: &[&[&str]] = &[
    &[
        "the", "escape", "cost", "is", "equal", "to", "the", "cards", "mana", "cost", "plus",
    ],
    &[
        "its", "escape", "cost", "is", "equal", "to", "its", "mana", "cost", "plus",
    ],
];
const GRANTED_ESCAPE_EXILE_TAIL_PHRASE: &[&str] =
    &["other", "cards", "from", "your", "graveyard"];
const GRANTED_ESCAPE_EXILE_SINGULAR_TAIL_PHRASE: &[&str] =
    &["other", "card", "from", "your", "graveyard"];
const GRANTED_ESCAPE_EXILE_TAIL_PHRASES: &[&[&str]] = &[
    GRANTED_ESCAPE_EXILE_TAIL_PHRASE,
    GRANTED_ESCAPE_EXILE_SINGULAR_TAIL_PHRASE,
];
const GRANTED_MIRACLE_COST_REDUCED_PREFIX_PHRASES: &[&[&str]] = &[
    &[
        "the", "miracle", "cost", "is", "equal", "to", "its", "mana", "cost", "reduced", "by",
    ],
    &[
        "its", "miracle", "cost", "is", "equal", "to", "its", "mana", "cost", "reduced", "by",
    ],
];
const UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["until", "your", "next", "turn"]);
const ALL_CREATURES_BLOCK_THIS_CREATURE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "all",
                "creatures",
                "able",
                "to",
                "block",
                "this",
                "creature",
                "do",
                "so"
            ],
            &[
                "all",
                "creatures",
                "able",
                "to",
                "block",
                "this",
                "do",
                "so"
            ],
        ]
);
const ALL_CREATURES_BLOCK_ENCHANTED_CREATURE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "enchanted",
            "creature",
            "do",
            "so"
        ]
);
const CAN_ATTACK_AS_NO_DEFENDER_AS_LONG_AS_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [&[
            "can", "attack", "as", "though", "it", "didnt", "have", "defender", "as", "long", "as",
        ]]
);
const CAN_ATTACK_AS_NO_DEFENDER_AS_LONG_AS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "can", "attack", "as", "though", "it", "didnt", "have", "defender", "as", "long", "as",
        ]
);
const CAN_ATTACK_AS_NO_DEFENDER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [&[
            "can", "attack", "as", "though", "it", "didnt", "have", "defender"
        ]]
);
const CAN_ATTACK_AS_NO_DEFENDER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "can", "attack", "as", "though", "it", "didnt", "have", "defender"
        ]
);

const ALL_CREATURES_LOSE_FLYING_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["all", "creatures", "lose", "flying"]);

const ANTHEM_LOSE_ALL_ABILITIES_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["lose", "all", "abilities"],
            &["loses", "all", "abilities"]
        ]]
);
const ANTHEM_ALL_ABILITIES_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["all", "abilities"]]);
const ANTHEM_EXCEPT_MANA_ABILITIES_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["except", "mana", "abilities"]]);
const ANTHEM_UNTIL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["until"]);
const ANTHEM_BECOMES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["becomes"]);
const ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["base", "power", "and", "toughness"]);
const THIS_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["this", "creature"]);
const ANTHEM_GET_OR_GETS_CONTAINS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["get", "gets"]]);
const ANTHEM_HAVE_OR_HAS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["have"], &["has"]]);
const ANTHEM_HAVE_HAS_GAIN_GAINS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["have"], &["has"], &["gain"], &["gains"]]);
const ANTHEM_GAIN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["gain"]);
const ANTHEM_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const ANTHEM_AND_OR_COMMA_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &[","]]);
const ANTHEM_AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const ANTHEM_IN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["in"]);
const ANTHEM_TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const ANTHEM_DEVOTION_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["devotion"]);
const ANTHEM_AS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["as"]);
const ANTHEM_IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const ANTHEM_DIDNT_CONTRACTION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["didn't"]);
const ANTHEM_WHEN_OR_WHENEVER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["when"], &["whenever"]]);
const ANTHEM_TRIGGERED_SEGMENT_START_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["when"], &["whenever"], &["at", "the"]]);
const ANTHEM_AND_TRIGGERED_SEGMENT_START_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["and", "when"],
            &["and", "whenever"],
            &["and", "at", "the"]
        ]
);
const ANTHEM_MAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["may"]);
const ANTHEM_ENCHANTED_OR_EQUIPPED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enchanted"], &["equipped"]]);
const ANTHEM_YOUR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["your"]);
const ANTHEM_THEIR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["their"]);
const ANTHEM_OPPONENT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["opponent"], &["opponents"]]);
const ANTHEM_IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"]]);
const ANTHEM_COLOR_OR_COLORS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["color"], &["colors"]]);
const ANTHEM_BE_NEGATED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["isnt"], &["isn't"]]);
const ANTHEM_BE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const ANTHEM_NO_LONGER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["no", "longer"]);
const ANTHEM_NOT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["not"]);
const ANTHEM_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"]]);
const ANTHEM_ADDITIONAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["additional"]);
const METALCRAFT_LABEL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["metalcraft"]);
const ANTHEM_EACH_OR_EVERY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["each"], &["every"]]);
const ANTHEM_ATTACK_OR_ATTACKS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attack"], &["attacks"]]);
const ANTHEM_TARGET_CONTAINS_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["target"]);
const ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["attacks", "each", "combat", "if", "able"],
            &["attack", "each", "combat", "if", "able"],
            &["and", "attack", "each", "combat", "if", "able"],
            &["and", "attacks", "each", "combat", "if", "able"],
        ]
);
const ANTHEM_AND_HAVE_OR_HAS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["and", "have"], &["and", "has"]]);
const ANTHEM_HAVE_OR_HAS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["have"], &["has"]]);
const ANTHEM_CANT_ATTACK_ALONE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["cant", "attack", "alone"]);
const ANTHEM_CANT_BLOCK_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cant", "block"]);
const ANTHEM_BLITZ_KEYWORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["blitz"]);
const ANTHEM_EMERGE_KEYWORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["emerge"]);
const ANTHEM_EXPLOIT_KEYWORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exploit"]);
const ANTHEM_BLITZ_COST_EQUALS_MANA_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "the", "blitz", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
            &[
                "its", "blitz", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
        ]
);
const ANTHEM_EMERGE_COST_EQUALS_MANA_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "the", "emerge", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
            &[
                "its", "emerge", "cost", "is", "equal", "to", "its", "mana", "cost",
            ],
        ]
);
const ANTHEM_SPELL_CAST_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["spell", "cast"]);
const ANTHEM_IGNORED_REMINDER_KEYWORD_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["unearth"], &["conspire"]]);
const ANTHEM_FLASHBACK_COST_EQUALS_MANA_COST_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "its",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost"
        ]
);
const ANTHEM_NAMED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["named"]);
const ANTHEM_NAMED_END_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["and"],
            &["lose"],
            &["loses"],
            &["with"],
            &["it"],
            &["that"],
            &["those"],
            &["this"],
        ]
);
const ANTHEM_SUBJECT_ATTACHED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["equipped", "enchanted"]]);
const ANTHEM_MANA_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["mana"]);
const ANTHEM_MANA_VALUE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["mana", "value"]]);
const ANTHEM_GRANTED_KEYWORD_REJECT_SUBJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["can"],
            &["cant"],
            &["cannot"],
            &["attack"],
            &["attacks"],
            &["block"],
            &["blocks"],
            &["blocked"],
            &["blocking"],
            &["during"],
            &["until"],
            &["unless"],
            &["when"],
            &["whenever"],
            &["if"],
            &["though"],
        ]
);

const CANT_GAIN_ABILITY_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["cant", "have", "or", "gain"], &["cant", "gain"],]);
const PERMANENT_CARD_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["permanent", "card"], &["permanent", "cards"]]);
const EQUIPMENT_YOU_CONTROL_HAVE_EQUIP_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["equipment", "you", "control", "have", "equip"]);

const EACH_CREATURE_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["each", "creature"]);
const ANTHEM_IT_OR_THEM_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["them"]]);
const ANTHEM_FOR_EACH_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["for", "each"]);
const ANTHEM_AFFECTED_ATTACKED_THIS_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["time", "it", "has", "attacked", "this", "turn"]);
const ANTHEM_AFFECTED_COLORS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["of", "its", "colors"],
            &["of", "their", "colors"],
            &["color", "it", "is"],
            &["colors", "it", "is"],
        ]
);
const ANTHEM_WARD_PAY_LIFE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["ward", "pay"]; suffix & ["life"]);
const ANTHEM_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["basic", "land", "type", "among"]);
const ANTHEM_CREATURE_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["creature", "type", "among"],
            &["creature", "types", "among"]
        ]
);
const ANTHEM_ATTACHED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["attached"]);
const ANTHEM_ATTACHED_TO_SOURCE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["to", "it"],
            &["to", "this", "creature"],
            &["to", "this", "permanent"],
        ]
);
const ANTHEM_UNSPENT_GREEN_MANA_YOU_HAVE_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["unspent", "green", "mana", "you", "have"]);
const SOULBOND_SOURCE_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["this", "creature"]]);
const SOULBOND_BOTH_CREATURES_GET_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["both", "creatures", "get"]);
const SOULBOND_EACH_OF_THOSE_CREATURES_GETS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["each", "of", "those", "creatures", "gets"]);
const SOULBOND_BOTH_CREATURES_HAVE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["both", "creatures", "have"]);
const SOULBOND_EACH_OF_THOSE_CREATURES_HAS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["each", "of", "those", "creatures", "has"]);

const NO_CARDS_IN_YOUR_LIBRARY_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["there", "are", "no", "cards", "in", "your", "library"],
            &["your", "library", "has", "no", "cards", "in", "it"],
        ]
);
const OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "opponent", "lost", "life", "this", "turn"],
            &[
                "one",
                "or",
                "more",
                "opponents",
                "lost",
                "life",
                "this",
                "turn",
            ],
        ]
);
const YOU_NOT_CAST_SPELL_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "havent", "cast", "a", "spell", "this", "turn"],
            &["you", "have", "not", "cast", "a", "spell", "this", "turn"],
            &["you", "didnt", "cast", "a", "spell", "this", "turn"],
            &["you", "did", "not", "cast", "a", "spell", "this", "turn"],
        ]
);
const YOU_CAST_SPELL_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["youve", "cast", "a", "spell", "this", "turn"],
            &["you", "ve", "cast", "a", "spell", "this", "turn"],
            &["you", "have", "cast", "a", "spell", "this", "turn"],
            &["you", "cast", "a", "spell", "this", "turn"],
        ]
);
const SOURCE_IS_ON_BATTLEFIELD_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "is", "on", "the", "battlefield"],
            &["this", "permanent", "is", "on", "the", "battlefield"],
            &["this", "is", "on", "the", "battlefield"],
            &["it", "is", "on", "the", "battlefield"],
        ]
);
const SOURCE_DEVOURED_CREATURES_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "devoured", "a", "creature"],
            &["it", "devoured", "one", "or", "more", "creatures"],
            &["this", "creature", "devoured", "a", "creature"],
            &[
                "this",
                "creature",
                "devoured",
                "one",
                "or",
                "more",
                "creatures",
            ],
        ]
);
const SOURCE_IS_SOULBOND_PAIRED_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "is", "paired", "with", "another", "creature"],
            &[
                "this", "creature", "is", "paired", "with", "another", "creature",
            ],
            &["it", "is", "paired", "with", "another", "creature"],
        ]
);
const SOURCE_ATTACKED_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "attacked", "this", "turn"],
            &["this", "creature", "attacked", "this", "turn"],
            &["this", "permanent", "attacked", "this", "turn"],
            &["that", "creature", "attacked", "this", "turn"],
        ]
);
const YOU_ATTACKED_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "attacked", "this", "turn"]);
const SOURCE_ENTERED_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "entered", "this", "turn"],
            &["this", "creature", "entered", "this", "turn"],
            &["this", "permanent", "entered", "this", "turn"],
        ]
);
const YOUR_TURN_CONDITION_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it", "is", "your", "turn"], &["its", "your", "turn"],]);
const SOURCE_POWER_EVEN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["thiss", "power", "is", "even"],
            &["this", "power", "is", "even"],
        ]
);
const SOURCE_POWER_ODD_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["thiss", "power", "is", "odd"],
            &["this", "power", "is", "odd"],
        ]
);
const NOT_YOUR_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["it", "is", "not", "your", "turn"],
            &["its", "not", "your", "turn"],
        ]
);
const YOUR_LIFE_HALF_STARTING_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "your", "life", "total", "is", "less", "than", "or", "equal", "to", "half", "your",
            "starting", "life", "total",
        ]
);
const ANTHEM_AS_LONG_AS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "long", "as"]);
const WITH_BASE_POWER_TOUGHNESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["with", "base", "power", "and", "toughness"]);
const IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["in", "addition", "to", "its", "other"]);
const PAIRED_WITH_ANOTHER_CREATURE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["is", "paired", "with", "another", "creature"]);
const ANTHEM_TYPE_OR_TYPES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["type"], &["types"]]);
const CANT_BE_BLOCKED_WORDS_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["can't", "be", "blocked"],
            &["cant", "be", "blocked"],
            &["cannot", "be", "blocked"],
        ]
);
const HAS_OR_HAVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["has"], &["have"]]);
const SOURCE_SELF_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["it"]]);
const ENCHANTED_PLAYER_CONTROLS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["enchanted", "player", "controls"]);
const ATTACHED_CONDITION_SUBJECT_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["enchanted", "artifact"],
            &["enchanted", "creature"],
            &["enchanted", "land"],
            &["enchanted", "permanent"],
            &["equipped", "creature"],
            &["equipped", "permanent"],
        ]
);
const ANTHEM_SOURCE_PRONOUN_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["its"]]);
const SOURCE_IN_GRAVEYARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["in", "your", "graveyard"], &["in", "graveyard"]]);
const IN_YOUR_GRAVEYARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["in", "your", "graveyard"]);
const ANTHEM_GRAVEYARD_CONJUNCTION_SPLIT_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["and", "graveyard"];
    contains_any_words & [&["control", "controls", "own", "owns"]]
);
const ANTHEM_ENTERED_WORD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["entered"]);
const ANTHEM_OTHER_OR_ANOTHER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);
const YOU_COMMITTED_CRIME_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["youve", "committed", "a", "crime", "this", "turn"],
            &["you", "ve", "committed", "a", "crime", "this", "turn"],
            &["you", "have", "committed", "a", "crime", "this", "turn"],
        ]
);
const ON_SOURCE_COUNTER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["on", "it"],
            &["on", "this"],
            &["on", "him"],
            &["on", "her"],
        ]
);
const SUBJECT_CANT_BE_BLOCKED_REJECT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["as"],
            &["long"],
            &["if"],
            &["when"],
            &["whenever"],
            &["get"],
            &["gets"],
            &["gain"],
            &["gains"],
            &["have"],
            &["has"],
        ]
);
const ANTHEM_GET_OR_GETS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"]]);
const ANTHEM_GET_GETS_IS_ARE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["get"], &["gets"], &["is"], &["are"]]);
const ANTHEM_LOSE_OR_LOSES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["lose"], &["loses"]]);
const ANTHEM_ALL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["all"]);
const TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const CREATURE_OR_CREATURES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["creature"], &["creatures"]]);
const AN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["an"]);
const EVERY_SUBTYPE_FAMILY_TAILS: &[(&[&str], crate::types::SubtypeFamily)] = &[
    (
        &["every", "creature", "type"],
        crate::types::SubtypeFamily::Creature,
    ),
    (
        &["every", "creature", "types"],
        crate::types::SubtypeFamily::Creature,
    ),
    (
        &["every", "land", "type"],
        crate::types::SubtypeFamily::Land,
    ),
    (
        &["every", "land", "types"],
        crate::types::SubtypeFamily::Land,
    ),
    (
        &["every", "artifact", "type"],
        crate::types::SubtypeFamily::Artifact,
    ),
    (
        &["every", "artifact", "types"],
        crate::types::SubtypeFamily::Artifact,
    ),
    (
        &["every", "enchantment", "type"],
        crate::types::SubtypeFamily::Enchantment,
    ),
    (
        &["every", "enchantment", "types"],
        crate::types::SubtypeFamily::Enchantment,
    ),
    (
        &["every", "spell", "type"],
        crate::types::SubtypeFamily::Spell,
    ),
    (
        &["every", "spell", "types"],
        crate::types::SubtypeFamily::Spell,
    ),
    (
        &["every", "planeswalker", "type"],
        crate::types::SubtypeFamily::Planeswalker,
    ),
    (
        &["every", "planeswalker", "types"],
        crate::types::SubtypeFamily::Planeswalker,
    ),
];
const IF_IT_IS_COLOR_PREFIXES: &[&[&str]] = &[
    &["its"],
    &["it's"],
    &["it’s"],
    &["it", "is"],
    &["it", "s"],
    &["this", "creature", "is"],
    &["that", "creature", "is"],
];

#[derive(Clone, Copy)]
enum GrantedAlternativeCastKeyword {
    Flashback,
    Blitz,
    Emerge,
    Miracle,
    Escape,
}

fn parse_granted_alternative_cast_keyword(words: &[&str]) -> Option<GrantedAlternativeCastKeyword> {
    let [keyword] = words else {
        return None;
    };
    match *keyword {
        "flashback" => Some(GrantedAlternativeCastKeyword::Flashback),
        "blitz" => Some(GrantedAlternativeCastKeyword::Blitz),
        "emerge" => Some(GrantedAlternativeCastKeyword::Emerge),
        "miracle" => Some(GrantedAlternativeCastKeyword::Miracle),
        "escape" => Some(GrantedAlternativeCastKeyword::Escape),
        _ => None,
    }
}

fn anthem_find_prefix_shape_start(words: &[&str], shape: &ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|&idx| shape.matches_words(&words[idx..]))
}

fn anthem_find_slash_word(words: &[&str]) -> Option<usize> {
    words
        .iter()
        .position(|word| crate::string_primitives::contains_char(word, '/'))
}

fn first_spell_each_turn_subject(filter_words: &[&str]) -> Option<AnthemSubjectAst> {
    FIRST_SPELL_EACH_TURN_SUBJECT_PATTERN
        .matches_words(filter_words)
        .then(|| {
            AnthemSubjectAst::Filter(
                ObjectFilter::spell()
                    .cast_by(PlayerFilter::You)
                    .first_spell_cast_each_turn(),
            )
        })
}

fn first_spell_each_turn_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<AnthemSubjectAst>, CardTextError> {
    const THE_PREFIX: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
    const TAIL: &[&str] = &["you", "cast", "each", "turn"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::optional(THE_PREFIX),
        LexPattern::word("first"),
        LexPattern::object("spell_filter", LexCaptureKind::UntilPhrase(TAIL)),
        LexPattern::phrase(TAIL),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(filter_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause) else {
        return Ok(None);
    };
    let filter_tokens = filter_clause.trimmed().tokens();
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter_lexed(&filter_tokens, false)?;
    if filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        && filter.zone != Some(Zone::Stack)
    {
        return Ok(None);
    }
    filter.cast_by = Some(PlayerFilter::You);
    filter.first_spell_cast_each_turn = true;
    Ok(Some(AnthemSubjectAst::Filter(filter)))
}

fn parse_cant_be_blocked_as_long_as_clause(
    tokens: &[OwnedLexToken],
) -> Option<CantBeBlockedAsLongAsClause<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(CANT_BE_BLOCKED_AS_LONG_AS_PHRASES),
        ),
        LexPattern::any_phrase(CANT_BE_BLOCKED_AS_LONG_AS_PHRASES),
        LexPattern::role_capture("condition", LexCaptureRole::Condition, LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    let condition_clause = matched
        .capture_clause_by_role(LexCaptureRole::Condition, clause)?
        .trimmed();
    (!subject_clause.tokens().is_empty() && !condition_clause.tokens().is_empty()).then_some(
        CantBeBlockedAsLongAsClause {
            subject_tokens: subject_clause.tokens(),
            condition_tokens: condition_clause.tokens(),
        },
    )
}

fn parse_cant_be_blocked_clause(tokens: &[OwnedLexToken]) -> Option<CantBeBlockedClause<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(CANT_BE_BLOCKED_PHRASES),
        ),
        LexPattern::any_phrase(CANT_BE_BLOCKED_PHRASES),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    (!subject_clause.tokens().is_empty()).then_some(CantBeBlockedClause {
        subject_tokens: subject_clause.tokens(),
    })
}

fn parse_keywords_and_cant_be_blocked_clause(
    tokens: &[OwnedLexToken],
) -> Option<KeywordsAndCantBeBlockedClause<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "keywords",
            LexCaptureKind::UntilLastAnyPhrase(AND_CANT_BE_BLOCKED_PHRASES),
        ),
        LexPattern::any_phrase(AND_CANT_BE_BLOCKED_PHRASES),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let keyword_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    (!keyword_clause.tokens().is_empty()).then_some(KeywordsAndCantBeBlockedClause {
        keyword_tokens: keyword_clause.tokens(),
    })
}

fn parse_landwalk_block_override_clause(
    tokens: &[OwnedLexToken],
) -> Option<LandwalkBlockOverrideClause<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(CAN_BE_BLOCKED_AS_THOUGH_NO_ABILITY_PHRASES),
        ),
        LexPattern::any_phrase(CAN_BE_BLOCKED_AS_THOUGH_NO_ABILITY_PHRASES),
        LexPattern::object("ability", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    let ability_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    let ability_words = ability_clause.word_refs();
    let [ability_word] = ability_words.as_slice() else {
        return None;
    };
    (!subject_clause.tokens().is_empty()).then_some(LandwalkBlockOverrideClause {
        subject_tokens: subject_clause.tokens(),
        ability_word,
    })
}

fn parse_granted_escape_cost_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<GrantedEscapeCostTail<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(GRANTED_ESCAPE_COST_PREFIX_PHRASES),
        LexPattern::word("exile"),
        LexPattern::amount(
            "exile_count",
            LexCaptureKind::UntilAnyPhrase(GRANTED_ESCAPE_EXILE_TAIL_PHRASES),
        ),
        LexPattern::any_phrase(GRANTED_ESCAPE_EXILE_TAIL_PHRASES),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let count_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    (!count_clause.tokens().is_empty()).then_some(GrantedEscapeCostTail {
        exile_count_tokens: count_clause.tokens(),
    })
}

fn parse_granted_miracle_cost_reduction_tail_clause(
    tokens: &[OwnedLexToken],
) -> Option<GrantedMiracleCostReductionTail<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(GRANTED_MIRACLE_COST_REDUCED_PREFIX_PHRASES),
        LexPattern::amount("reduction_cost", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let cost_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    (!cost_clause.tokens().is_empty()).then_some(GrantedMiracleCostReductionTail {
        reduction_cost_tokens: cost_clause.tokens(),
    })
}

fn parse_cant_be_blocked_by_more_than_clause(
    tokens: &[OwnedLexToken],
) -> Option<CantBeBlockedByMoreThanClause<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(CANT_BE_BLOCKED_BY_PHRASES),
        ),
        LexPattern::any_phrase(CANT_BE_BLOCKED_BY_PHRASES),
        LexPattern::amount(
            "blocker_threshold",
            LexCaptureKind::UntilAnyPhrase(CREATURE_NOUN_PHRASES),
        ),
        LexPattern::any_phrase(CREATURE_NOUN_PHRASES),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    let threshold_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    (!subject_clause.tokens().is_empty() && !threshold_clause.tokens().is_empty()).then_some(
        CantBeBlockedByMoreThanClause {
            subject_tokens: subject_clause.tokens(),
            blocker_threshold_tokens: threshold_clause.tokens(),
        },
    )
}

fn parse_can_block_additional_creature_clause(
    tokens: &[OwnedLexToken],
) -> Option<CanBlockAdditionalCreatureClause<'_>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject("subject", LexCaptureKind::UntilPhrase(CAN_BLOCK_PHRASE)),
        LexPattern::phrase(CAN_BLOCK_PHRASE),
        LexPattern::amount(
            "additional_count",
            LexCaptureKind::UntilAnyPhrase(ADDITIONAL_CREATURE_TAIL_PHRASES),
        ),
        LexPattern::any_phrase(ADDITIONAL_CREATURE_TAIL_PHRASES),
        LexPattern::phrase(EACH_COMBAT_PHRASE),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    let count_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    (!subject_clause.tokens().is_empty() && !count_clause.tokens().is_empty()).then_some(
        CanBlockAdditionalCreatureClause {
            subject_tokens: subject_clause.tokens(),
            additional_count_tokens: count_clause.tokens(),
        },
    )
}

fn triggered_grant_effects_and_condition(
    trigger: &TriggerSpec,
    effects: &[EffectAst],
) -> Result<(Vec<EffectAst>, Option<crate::ConditionExpr>), CardTextError> {
    if let [
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        },
    ] = effects
        && if_false.is_empty()
    {
        let mut imports = ReferenceImports::default();
        imports.last_player_filter =
            crate::runtime_backend::compile_support::inferred_trigger_player_filter(trigger);
        let reference_env = crate::runtime_backend::reference_model::ReferenceEnv::from_imports(
            &imports, false, false, false, None,
        );
        let condition =
            crate::runtime_backend::compile_support::compile_condition_from_predicate_ast_with_env(
                predicate,
                &reference_env,
                None,
            )?;
        return Ok((if_true.clone(), Some(condition)));
    }

    Ok((effects.to_vec(), None))
}

pub(crate) fn parse_subject_cant_be_blocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    if subject_tokens
        .iter()
        .any(|token| token.is_comma() || AND_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if subject_words
        .first()
        .is_some_and(|word| SOURCE_SELF_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| SUBJECT_CANT_BE_BLOCKED_REJECT_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if POWER_OR_TOUGHNESS_SUBJECT_PATTERN.matches_words(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness cant-be-blocked subject (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(&subject_tokens))?;
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::KeywordAction(KeywordAction::Unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: None,
        },
    };
    Ok(Some(ability))
}

pub(crate) fn parse_subject_has_keywords_and_cant_be_blocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(has_idx) = anthem_last_token_offset(tokens, |token| {
        HAS_OR_HAVE_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    if has_idx == 0 || has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    let Some(parsed_tail) = parse_keywords_and_cant_be_blocked_clause(&ability_tokens) else {
        return Ok(None);
    };
    let keyword_tokens = parsed_tail.keyword_tokens;
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let keyword_actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if keyword_actions.is_empty() {
        return Ok(None);
    }

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut granted = Vec::new();
    for action in keyword_actions
        .into_iter()
        .chain(std::iter::once(KeywordAction::Unblockable))
    {
        granted.push(match &subject {
            AnthemSubjectAst::Source => match &condition {
                Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                    action,
                    condition: condition.clone(),
                },
                None => StaticAbilityAst::KeywordAction(action),
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: condition.clone(),
            },
        });
    }

    Ok(Some(granted))
}

pub(crate) fn parse_landwalk_as_though_block_override_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_landwalk_block_override_clause(tokens) else {
        return Ok(None);
    };
    if !is_landwalk_ability_word(parsed.ability_word) {
        return Ok(None);
    }

    let AnthemSubjectAst::Filter(filter) = parse_anthem_subject(parsed.subject_tokens)? else {
        return Ok(None);
    };

    let removed = StaticAbility::keyword_marker(parsed.ability_word);
    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::remove_ability(filter, removed),
    )))
}

fn is_landwalk_ability_word(word: &str) -> bool {
    matches!(
        parse_single_word_keyword_action(word),
        Some(KeywordAction::Landwalk(_))
    )
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_condition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_as_long_as_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = parsed.subject_tokens;
    let condition = parse_static_condition_clause(parsed.condition_tokens)?;

    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(&subject_tokens))?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalKeywordAction {
            action: KeywordAction::Unblockable,
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter,
            action: KeywordAction::Unblockable,
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

fn simple_card_types_from_control_filter(mut filter: ObjectFilter) -> Option<Vec<CardType>> {
    let mut card_types = if filter.all_card_types.is_empty() {
        Vec::new()
    } else {
        std::mem::take(&mut filter.all_card_types)
    };

    if !filter.card_types.is_empty() {
        if card_types.is_empty() {
            card_types = std::mem::take(&mut filter.card_types);
        } else if filter.card_types.len() == card_types.len()
            && filter
                .card_types
                .iter()
                .all(|card_type| card_types.contains(card_type))
        {
            filter.card_types.clear();
        } else {
            return None;
        }
    }

    if card_types.is_empty()
        || !card_types.iter().all(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Battle
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
            )
        })
    {
        return None;
    }

    filter.zone = None;
    (filter == ObjectFilter::default()).then_some(card_types)
}

fn defending_player_controlled_card_types_from_condition_tokens(
    condition_tokens: &[OwnedLexToken],
) -> Option<Vec<CardType>> {
    let condition = crate::runtime_backend::grammar::conditions::parse_control_condition(
        condition_tokens,
        crate::runtime_backend::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: true,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;

    if condition.player_filter != Some(PlayerFilter::Defending)
        || condition.requires_different_powers
        || condition.at_least_count()? > 1
    {
        return None;
    }

    simple_card_types_from_control_filter(condition.filter)
}

pub(crate) fn parse_subject_cant_be_blocked_as_long_as_defending_player_controls_card_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_as_long_as_clause(tokens) else {
        return Ok(None);
    };
    let Some(card_types) =
        defending_player_controlled_card_types_from_condition_tokens(parsed.condition_tokens)
    else {
        return Ok(None);
    };

    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| parse_anthem_subject(parsed.subject_tokens))?;
    let unblockable = if card_types.len() == 1 {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_type(card_types[0])
    } else {
        StaticAbility::cant_be_blocked_as_long_as_defending_player_controls_card_types(card_types)
    };
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::Static(unblockable),
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(unblockable)),
            condition: None,
        },
    };
    Ok(Some(ability))
}

pub(crate) fn parse_granted_keyword_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    fn extract_grant_spec_from_subject(
        subject_tokens: &[OwnedLexToken],
        grantable: crate::grant::Grantable,
    ) -> Result<Option<crate::grant::GrantSpec>, CardTextError> {
        let subject = parse_anthem_subject(subject_tokens)?;
        let AnthemSubjectAst::Filter(mut filter) = subject else {
            return Ok(None);
        };
        let zone = filter.zone.unwrap_or(Zone::Battlefield);
        filter.zone = None;
        Ok(Some(crate::grant::GrantSpec::new(grantable, filter, zone)))
    }

    fn parse_granted_escape_cost_tail(
        trailing_tokens: &[OwnedLexToken],
    ) -> Result<Option<u32>, CardTextError> {
        let trailing_word_refs = crate::runtime_backend::token_word_refs(trailing_tokens);
        let Some(parsed) = parse_granted_escape_cost_tail_clause(trailing_tokens) else {
            return Ok(None);
        };

        let Some((count, used)) = parse_number(parsed.exile_count_tokens) else {
            return Err(CardTextError::ParseError(format!(
                "escape cost clause missing exile count (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        };
        if used != parsed.exile_count_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported escape cost clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        Ok(Some(count as u32))
    }

    fn parse_granted_miracle_cost_reduction_tail(
        trailing_tokens: &[OwnedLexToken],
    ) -> Result<Option<u32>, CardTextError> {
        let trailing_word_refs = crate::runtime_backend::token_word_refs(trailing_tokens);
        let Some(parsed) = parse_granted_miracle_cost_reduction_tail_clause(trailing_tokens) else {
            return Ok(None);
        };

        let Some((cost, used)) =
            crate::runtime_backend::front_end::shared::util::leading_mana_cost_from_tokens(
                parsed.reduction_cost_tokens,
            )
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        };
        if used != parsed.reduction_cost_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        let generic = cost.generic_mana_total();
        if generic == 0 || cost.mana_value() != generic {
            return Err(CardTextError::ParseError(format!(
                "unsupported miracle cost reduction clause (clause: '{}')",
                trailing_word_refs.join(" ")
            )));
        }
        Ok(Some(generic))
    }

    fn parse_granted_alternative_cast_static(
        subject_tokens: &[OwnedLexToken],
        keyword_tokens: &[OwnedLexToken],
        trailing_tokens: &[OwnedLexToken],
        condition: Option<crate::ConditionExpr>,
    ) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
        let keyword_words = crate::runtime_backend::token_word_refs(keyword_tokens);
        let spec = match parse_granted_alternative_cast_keyword(&keyword_words) {
            Some(GrantedAlternativeCastKeyword::Flashback) => {
                let trailing_words = AnthemNormalizedWords::new(trailing_tokens);
                let trailing_word_refs = trailing_words.word_refs();
                if !ANTHEM_FLASHBACK_COST_EQUALS_MANA_COST_PATTERN
                    .matches_words(&trailing_word_refs)
                {
                    return Ok(None);
                }
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::flashback_from_cards_mana_cost(),
                )?
            }
            Some(GrantedAlternativeCastKeyword::Blitz) => {
                if !is_granted_blitz_cost_tail(trailing_tokens) {
                    return Ok(None);
                }
                return granted_blitz_abilities_from_subject(subject_tokens, condition);
            }
            Some(GrantedAlternativeCastKeyword::Emerge) => {
                if !is_granted_emerge_cost_tail(trailing_tokens) {
                    return Ok(None);
                }
                return granted_emerge_abilities_from_subject(subject_tokens, condition);
            }
            Some(GrantedAlternativeCastKeyword::Miracle) => {
                let Some(reduction) = parse_granted_miracle_cost_reduction_tail(trailing_tokens)?
                else {
                    return Ok(None);
                };
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::miracle_from_cards_mana_cost_reduced_by(reduction),
                )?
            }
            Some(GrantedAlternativeCastKeyword::Escape) => {
                let Some(exile_count) = parse_granted_escape_cost_tail(trailing_tokens)? else {
                    return Ok(None);
                };
                extract_grant_spec_from_subject(
                    subject_tokens,
                    crate::grant::Grantable::escape(exile_count),
                )?
            }
            None => None,
        };

        let Some(spec) = spec else {
            return Ok(None);
        };

        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        Ok(Some(vec![ability]))
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !clause_words
        .iter()
        .any(|word| HAS_OR_HAVE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let have_token_idx = anthem_last_token_offset(tokens, |token| {
        HAS_OR_HAVE_WORD_PATTERN.matches_token(token)
    })
    .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    if crate::runtime_backend::token_word_refs(&tokens[..have_token_idx])
        .iter()
        .any(|word| ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    if token_slice_starts_with(tokens, &["as", "long", "as"]) {
        let trailing_has = tokens[have_token_idx + 1..]
            .iter()
            .any(|token| HAS_OR_HAVE_WORD_PATTERN.matches_token(token));
        let trailing_get_or_be = tokens[have_token_idx + 1..]
            .iter()
            .any(|token| ANTHEM_GET_GETS_IS_ARE_WORD_PATTERN.matches_token(token));
        if !trailing_has && trailing_get_or_be {
            return Ok(None);
        }
    }

    let (prefix_condition, subject_start) =
        match parse_anthem_prefix_condition(tokens, have_token_idx) {
            Ok(parsed) => parsed,
            Err(_) => return Ok(None),
        };
    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if ANTHEM_SUBJECT_ATTACHED_MARKER_PATTERN.matches_words(&subject_words)
        || (ANTHEM_MANA_WORD_MARKER_PATTERN.matches_words(&subject_words)
            && !ANTHEM_MANA_VALUE_MARKER_PATTERN.matches_words(&subject_words))
    {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| ANTHEM_GRANTED_KEYWORD_REJECT_SUBJECT_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let tail_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let mut tail_tokens = tail_tokens;
    let mut trailing_clause_tokens: Vec<OwnedLexToken> = Vec::new();
    let tail_sentences =
        crate::runtime_backend::grammar::primitives::split_lexed_slices_on_period(&tail_tokens);
    if tail_sentences.len() > 1 {
        let leading = trim_edge_punctuation(tail_sentences[0]);
        let trailing = tail_sentences[1..]
            .iter()
            .flat_map(|sentence| trim_edge_punctuation(sentence))
            .collect::<Vec<_>>();
        trailing_clause_tokens = trailing;
        tail_tokens = leading;
    }

    let mut keyword_tokens = tail_tokens.clone();
    let mut suffix_condition = None;
    if let Some(idx) = anthem_find_prefix_shape_start(
        &crate::runtime_backend::token_word_refs(&tail_tokens),
        &ANTHEM_AS_LONG_AS_PREFIX_PATTERN,
    ) {
        if idx + 3 >= tail_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        keyword_tokens = trim_commas(&tail_tokens[..idx]);
        suffix_condition = Some(parse_static_condition_clause(&tail_tokens[idx + 3..])?);
    }
    if keyword_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut grants_must_attack = false;
    let keyword_words = crate::runtime_backend::token_word_refs(&keyword_tokens);
    if let Some(and_idx) = keyword_words
        .windows(5)
        .position(|window| ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(window))
    {
        keyword_tokens = trim_commas(&keyword_tokens[..and_idx]);
        grants_must_attack = true;
    }
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple static conditions are not supported in granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        (Some(cond), None) | (None, Some(cond)) => Some(cond),
        (None, None) => None,
    };

    let keyword_words = crate::runtime_backend::token_word_refs(&keyword_tokens);
    if ANTHEM_BLITZ_KEYWORD_PATTERN.matches_words(&keyword_words)
        && (trailing_clause_tokens.is_empty()
            || is_granted_blitz_cost_tail(&trailing_clause_tokens))
    {
        return granted_blitz_abilities_from_subject(&subject_tokens, condition);
    }
    if ANTHEM_EMERGE_KEYWORD_PATTERN.matches_words(&keyword_words)
        && (trailing_clause_tokens.is_empty()
            || is_granted_emerge_cost_tail(&trailing_clause_tokens))
    {
        return granted_emerge_abilities_from_subject(&subject_tokens, condition);
    }

    if !trailing_clause_tokens.is_empty() {
        if let Some(compiled) = parse_granted_alternative_cast_static(
            &subject_tokens,
            &keyword_tokens,
            &trailing_clause_tokens,
            condition.clone(),
        )? {
            return Ok(Some(compiled));
        }

        let keyword_words = crate::runtime_backend::token_word_refs(&keyword_tokens);
        let ignore_keyword_reminder =
            ANTHEM_IGNORED_REMINDER_KEYWORD_PATTERN.matches_words(&keyword_words);
        if !ignore_keyword_reminder {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if let Some(compiled) = parse_color_filtered_keyword_grants(
        &subject_tokens,
        &keyword_tokens,
        condition.clone(),
        &clause_words.join(" "),
    )? {
        return Ok(Some(compiled));
    }

    if ANTHEM_EXPLOIT_KEYWORD_PATTERN
        .matches_words(&crate::runtime_backend::token_word_refs(&keyword_tokens))
    {
        let subject = parse_anthem_subject(&subject_tokens)?;
        return Ok(Some(vec![grant_exploit_for_anthem_subject(
            &subject, condition,
        )]));
    }

    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    if actions.is_empty() {
        return Ok(None);
    }

    let attached_subject_filter =
        infer_attached_subject_filter_from_condition_expr(condition.as_ref());
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    let subject = first_spell_each_turn_subject(&subject_words)
        .map(Ok)
        .unwrap_or_else(|| {
            parse_anthem_subject_with_attached_fallback(
                &subject_tokens,
                attached_subject_filter.as_ref(),
            )
        })?;

    let grants_conspire = actions
        .iter()
        .filter(|action| matches!(action, KeywordAction::Conspire))
        .count();
    if grants_conspire > 0 {
        let mut compiled = Vec::new();
        for _ in 0..grants_conspire {
            match &subject {
                AnthemSubjectAst::Source => {
                    let ability =
                        StaticAbilityAst::Static(StaticAbility::keyword_marker("Conspire"));
                    if let Some(condition) = &condition {
                        compiled.push(StaticAbilityAst::ConditionalStaticAbility {
                            ability: Box::new(ability),
                            condition: condition.clone(),
                        });
                    } else {
                        compiled.push(ability);
                    }
                }
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantStaticAbility {
                        filter: filter.clone(),
                        ability: Box::new(StaticAbilityAst::Static(StaticAbility::keyword_marker(
                            "Conspire",
                        ))),
                        condition: condition.clone(),
                    });
                }
            }
        }
        return Ok(Some(compiled));
    }

    let mut mapped = Vec::new();
    let mut object_ability_grants = Vec::new();
    for action in actions {
        if action.lowers_to_static_ability() {
            mapped.push(action);
        } else if let Some(granted) = granted_object_ability_for_keyword_action(&action) {
            object_ability_grants.push(granted);
        } else {
            return Ok(None);
        }
    }
    if mapped.is_empty() && object_ability_grants.is_empty() && !grants_must_attack {
        return Ok(None);
    }

    let mut compiled = Vec::new();
    if grants_must_attack {
        match &subject {
            AnthemSubjectAst::Source => {
                if let Some(condition) = &condition {
                    compiled.push(StaticAbilityAst::ConditionalStaticAbility {
                        ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                        condition: condition.clone(),
                    });
                } else {
                    compiled.push(StaticAbilityAst::Static(StaticAbility::must_attack()));
                }
            }
            AnthemSubjectAst::Filter(filter) => {
                compiled.push(StaticAbilityAst::GrantStaticAbility {
                    filter: filter.clone(),
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
                    condition: condition.clone(),
                })
            }
        }
    }
    for action in mapped {
        let ast = match &subject {
            AnthemSubjectAst::Source => match &condition {
                Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                    action,
                    condition: condition.clone(),
                },
                None => StaticAbilityAst::KeywordAction(action),
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
                filter: filter.clone(),
                action,
                condition: condition.clone(),
            },
        };
        compiled.push(ast);
    }
    let grant_clause = ParsedAnthemClause {
        subject,
        power: AnthemValue::Fixed(0),
        toughness: AnthemValue::Fixed(0),
        condition,
    };
    for (ability, display) in object_ability_grants {
        compiled.push(grant_object_ability_for_anthem_subject(
            &grant_clause,
            ability,
            display,
        ));
    }
    Ok(Some(compiled))
}

pub(crate) fn parse_all_creatures_lose_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if ALL_CREATURES_LOSE_FLYING_PATTERN.matches_words(&words) {
        return Ok(Some(StaticAbilityAst::RemoveKeywordAction {
            filter: ObjectFilter::creature(),
            action: KeywordAction::Flying,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_subject_loses_keywords_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(lose_idx) = anthem_token_offset(tokens, |token| {
        ANTHEM_LOSE_OR_LOSES_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    if lose_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..lose_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens
        .first()
        .is_some_and(|token| TARGET_WORD_PATTERN.matches_token(token))
        || subject_tokens
            .iter()
            .any(|token| ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }
    let filter = match parse_object_filter(&subject_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let tail = trim_edge_punctuation(&tokens[lose_idx + 1..]);
    if tail.is_empty() {
        return Ok(None);
    }

    let mut loss_end = tail.len();
    let mut cant_tail: Option<Vec<OwnedLexToken>> = None;
    for (idx, token) in tail.iter().enumerate() {
        if !AND_WORD_PATTERN.matches_token(token) {
            continue;
        }
        let after_and = trim_edge_punctuation(&tail[idx + 1..]);
        let after_words_storage = normalize_cant_words(&after_and);
        let after_words = after_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if CANT_GAIN_ABILITY_TAIL_PATTERN.matches_words(&after_words) {
            loss_end = idx;
            cant_tail = Some(after_and);
            break;
        }
    }

    let loss_tokens = trim_edge_punctuation(&tail[..loss_end]);
    let Some(loss_actions) = parse_ability_line(&loss_tokens) else {
        return Ok(None);
    };

    let mut actions = loss_actions;
    if let Some(cant_tail) = cant_tail {
        let Some(gain_idx) = anthem_token_offset(&cant_tail, |token| {
            ANTHEM_GAIN_WORD_PATTERN.matches_token(token)
        }) else {
            return Ok(None);
        };
        let gain_tokens = trim_edge_punctuation(&cant_tail[gain_idx + 1..]);
        if gain_tokens.is_empty() {
            return Ok(None);
        }
        let Some(gain_actions) = parse_ability_line(&gain_tokens) else {
            return Ok(None);
        };
        actions.extend(gain_actions);
    }

    let clause_text = crate::runtime_backend::token_word_refs(tokens).join(" ");
    reject_unimplemented_keyword_actions(&actions, &clause_text)?;

    let mut result = Vec::new();
    for action in actions {
        if !action.lowers_to_static_ability() {
            return Ok(None);
        }
        if result.iter().any(|existing| {
            matches!(
                existing,
                StaticAbilityAst::RemoveKeywordAction {
                    filter: existing_filter,
                    action: existing_action,
                } if existing_filter == &filter && existing_action == &action
            )
        }) {
            continue;
        }
        result.push(StaticAbilityAst::RemoveKeywordAction {
            filter: filter.clone(),
            action,
        });
    }

    if result.is_empty() {
        return Ok(None);
    }
    Ok(Some(result))
}

pub(crate) fn parse_each_creature_cant_be_blocked_by_more_than_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(parsed) = parse_cant_be_blocked_by_more_than_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    if !EACH_CREATURE_SUBJECT_PREFIX_PATTERN.matches_words(&subject_words) {
        return Ok(None);
    }
    let Some((minimum_blockers, used)) = parse_greater_than_or_equal_quantity_prefix(
        parsed.blocker_threshold_tokens,
        false,
        false,
        "cant-be-blocked blocker threshold",
    )?
    else {
        return Ok(None);
    };
    if minimum_blockers == 0 || used != parsed.blocker_threshold_tokens.len() {
        return Ok(None);
    }
    let amount = minimum_blockers - 1;
    let filter_tokens_storage;
    let mut filter_tokens = parsed.subject_tokens;
    if filter_tokens
        .first()
        .is_some_and(|token| EACH_WORD_PATTERN.matches_token(token))
    {
        filter_tokens_storage = trim_commas(&filter_tokens[1..]);
        filter_tokens = &filter_tokens_storage;
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-be-blocked-by-more-than subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let granted = StaticAbility::cant_be_blocked_by_more_than(amount as usize);
    Ok(Some(StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(granted)),
        condition: None,
    }))
}

pub(crate) fn parse_each_creature_can_block_additional_creature_each_combat_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    // High Ground: "Each creature can block an additional creature each combat."
    let Some(parsed) = parse_can_block_additional_creature_clause(tokens) else {
        return Ok(None);
    };
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    if !EACH_CREATURE_SUBJECT_PREFIX_PATTERN.matches_words(&subject_words) {
        return Ok(None);
    };

    let additional = if parsed
        .additional_count_tokens
        .first()
        .is_some_and(|token| AN_WORD_PATTERN.matches_token(token))
        && parsed.additional_count_tokens.len() == 1
    {
        1usize
    } else if let Some((count, used)) = parse_number(parsed.additional_count_tokens)
        && used == parsed.additional_count_tokens.len()
    {
        count as usize
    } else {
        return Ok(None);
    };

    let filter_tokens_storage;
    let mut filter_tokens = parsed.subject_tokens;
    if filter_tokens
        .first()
        .is_some_and(|token| EACH_WORD_PATTERN.matches_token(token))
    {
        filter_tokens_storage = trim_commas(&filter_tokens[1..]);
        filter_tokens = &filter_tokens_storage;
    }
    let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported can-block-additional subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let granted = StaticAbility::can_block_additional_creature_each_combat(additional);
    Ok(Some(StaticAbilityAst::GrantStaticAbility {
        filter,
        ability: Box::new(StaticAbilityAst::Static(granted)),
        condition: None,
    }))
}

pub(crate) fn parse_lose_all_abilities_and_transform_base_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    fn title_case_words(words: &[&str]) -> String {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(is_idx) = ANTHEM_IS_OR_ARE_WORD_PATTERN.find_word(&words) else {
        return Ok(None);
    };
    let Some(with_idx) =
        anthem_find_prefix_shape_start(&words, &WITH_BASE_POWER_TOUGHNESS_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    if with_idx <= is_idx {
        return Ok(None);
    }

    let Some(pt_word) = words.get(with_idx + 5) else {
        return Ok(None);
    };
    let (power, toughness) = parse_pt_modifier(pt_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let has_lose_all = ANTHEM_LOSE_ALL_ABILITIES_PATTERN.matches_words(&words);
    if !has_lose_all {
        return Ok(None);
    }

    let subject_end = is_idx.min(
        ANTHEM_LOSE_OR_LOSES_WORD_PATTERN
            .find_word(&words)
            .unwrap_or(is_idx),
    );
    if subject_end == 0 {
        return Ok(None);
    }
    let subject_tokens = trim_commas(&tokens[..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let mut descriptor_words = non_article_word_refs_except(&words[is_idx + 1..with_idx], &["and"]);
    if descriptor_words.is_empty() {
        return Ok(None);
    }
    if ANTHEM_ALL_WORD_PATTERN.matches_first_word(&descriptor_words) {
        descriptor_words.remove(0);
    }
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_colors = ColorSet::new();
    let mut set_card_types: Vec<CardType> = Vec::new();
    let mut creature_subtypes: Vec<Subtype> = Vec::new();

    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !set_card_types.iter().any(|existing| *existing == card_type) {
                set_card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if !creature_subtypes
                .iter()
                .any(|existing| *existing == subtype)
            {
                creature_subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported transform descriptor '{}' (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if !creature_subtypes.is_empty()
        && !set_card_types
            .iter()
            .any(|existing| *existing == CardType::Creature)
    {
        set_card_types.push(CardType::Creature);
    }

    let mut set_name: Option<String> = None;
    let tail_words = &words[with_idx + 6..];
    if let Some(named_idx) = ANTHEM_NAMED_WORD_PATTERN.find_word(tail_words) {
        let end_idx = ANTHEM_NAMED_END_WORD_PATTERN
            .find_word(&tail_words[named_idx + 1..])
            .map(|idx| named_idx + 1 + idx)
            .unwrap_or(tail_words.len());
        if end_idx > named_idx + 1 {
            set_name = Some(title_case_words(&tail_words[named_idx + 1..end_idx]));
        }
    }

    let has_except_mana = ANTHEM_EXCEPT_MANA_ABILITIES_PATTERN.matches_words(&words);
    let mut abilities = vec![if has_except_mana {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    if !set_card_types.is_empty() {
        abilities.push(StaticAbility::set_card_types(
            filter.clone(),
            set_card_types,
        ));
    }
    if !creature_subtypes.is_empty() {
        abilities.push(StaticAbility::set_creature_subtypes(
            filter.clone(),
            creature_subtypes,
        ));
    }
    if !set_colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), set_colors));
    }
    if let Some(name) = set_name {
        abilities.push(StaticAbility::set_name(filter.clone(), name));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    ));

    Ok(Some(abilities))
}

pub(crate) fn parse_lose_all_abilities_and_base_pt_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if anthem_find_prefix_shape_start(&words, &WITH_BASE_POWER_TOUGHNESS_PREFIX_PATTERN).is_some()
        && ANTHEM_IS_OR_ARE_WORD_PATTERN.find_word(&words).is_some()
    {
        return Ok(None);
    }

    let lose_idx = ANTHEM_LOSE_OR_LOSES_WORD_PATTERN.find_word(&words);
    let Some(lose_idx) = lose_idx else {
        return Ok(None);
    };

    if !ANTHEM_ALL_ABILITIES_TAIL_PATTERN.matches_words(&words[lose_idx + 1..]) {
        return Ok(None);
    }
    if ANTHEM_UNTIL_WORD_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    if ANTHEM_BECOMES_WORD_PATTERN.matches_words(&words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported lose-all-abilities static becomes clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..lose_idx];
    let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let has_except_mana = ANTHEM_EXCEPT_MANA_ABILITIES_PATTERN.matches_words(&words);
    let mut abilities = vec![if has_except_mana {
        StaticAbility::remove_all_abilities_except_mana(filter.clone())
    } else {
        StaticAbility::remove_all_abilities(filter.clone())
    }];

    let have_idx = HAS_OR_HAVE_WORD_PATTERN.find_word(&words);
    if let Some(have_idx) = have_idx {
        let after_have = &words[have_idx + 1..];
        if ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(after_have)
            && let Some(modifier_token_idx) = anthem_find_slash_word(after_have)
            && let Some(modifier_token) = after_have.get(modifier_token_idx)
            && let Ok((power, toughness)) = parse_pt_modifier(modifier_token)
        {
            abilities.push(StaticAbility::set_base_power_toughness(
                filter, power, toughness,
            ));
        }
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_all_have_indestructible_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let have_idx = HAS_OR_HAVE_WORD_PATTERN.find_word(&words);
    let Some(have_idx) = have_idx else {
        return Ok(None);
    };
    if ANTHEM_GET_OR_GETS_CONTAINS_PATTERN.matches_words(&words[..have_idx]) {
        return Ok(None);
    }

    let have_token_idx = anthem_token_offset(tokens, |token| {
        ANTHEM_HAVE_OR_HAS_WORD_PATTERN.matches_token(token)
    })
    .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    let tail = trim_commas(&tokens[have_token_idx + 1..]);
    let Some(actions) = parse_ability_line(&tail) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &words.join(" "))?;
    if actions.len() != 1
        || !actions
            .first()
            .is_some_and(|action| matches!(action, KeywordAction::Indestructible))
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..have_token_idx], false)?;
    Ok(Some(StaticAbilityAst::GrantKeywordAction {
        filter,
        action: KeywordAction::Indestructible,
        condition: None,
    }))
}

#[derive(Debug, Clone)]
pub(crate) enum AnthemSubjectAst {
    Source,
    Filter(ObjectFilter),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAnthemClause {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) power: AnthemValue,
    pub(crate) toughness: AnthemValue,
    pub(crate) condition: Option<crate::ConditionExpr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AnimationSubtypeMode {
    Add,
    ReplaceCreatureTypes,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedGrantedTailAst {
    pub(crate) granted_static: Vec<StaticAbilityAst>,
    pub(crate) granted_keyword_actions: Vec<KeywordAction>,
    pub(crate) granted_object_abilities: Vec<(ParsedAbility, String)>,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticAnimationBundleAst {
    pub(crate) subject: AnthemSubjectAst,
    pub(crate) condition: Option<crate::ConditionExpr>,
    pub(crate) ensure_creature_type: bool,
    pub(crate) subtypes: Vec<Subtype>,
    pub(crate) subtype_mode: AnimationSubtypeMode,
    pub(crate) base_power_toughness: Option<(i32, i32)>,
    pub(crate) granted_tail: ParsedGrantedTailAst,
}

fn is_granted_blitz_cost_tail(trailing_tokens: &[OwnedLexToken]) -> bool {
    let trailing_words = AnthemNormalizedWords::new(trailing_tokens);
    let trailing_word_refs = trailing_words.word_refs();
    ANTHEM_BLITZ_COST_EQUALS_MANA_COST_PATTERN.matches_words(&trailing_word_refs)
}

fn is_granted_emerge_cost_tail(trailing_tokens: &[OwnedLexToken]) -> bool {
    let trailing_words = AnthemNormalizedWords::new(trailing_tokens);
    let trailing_word_refs = trailing_words.word_refs();
    ANTHEM_EMERGE_COST_EQUALS_MANA_COST_PATTERN.matches_words(&trailing_word_refs)
}

fn normalize_granted_alternative_spell_filter(
    mut filter: ObjectFilter,
) -> (ObjectFilter, Vec<Zone>) {
    let parsed_zone = filter.zone.unwrap_or(Zone::Battlefield);
    if parsed_zone == Zone::Stack || filter.stack_kind.is_some() {
        filter.zone = None;
        filter.stack_kind = None;
        filter.cast_by = None;
        return (filter, vec![Zone::Hand, Zone::Exile, Zone::Graveyard]);
    }

    filter.zone = None;
    (filter, vec![parsed_zone])
}

fn granted_blitz_abilities_from_subject(
    subject_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let subject = parse_anthem_subject(subject_tokens)?;
    let AnthemSubjectAst::Filter(filter) = subject else {
        return Ok(None);
    };
    let (filter, zones) = normalize_granted_alternative_spell_filter(filter);
    let mut abilities = Vec::new();
    for zone in zones {
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::blitz_from_cards_mana_cost(),
            filter.clone(),
            zone,
        );
        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition.clone() {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        abilities.push(ability);
    }
    Ok(Some(abilities))
}

fn granted_emerge_abilities_from_subject(
    subject_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let subject = parse_anthem_subject(subject_tokens)?;
    let AnthemSubjectAst::Filter(filter) = subject else {
        return Ok(None);
    };
    let subject_words = crate::runtime_backend::token_word_refs(subject_tokens);
    let (filter, zones) = if ANTHEM_SPELL_CAST_SUBJECT_PATTERN.matches_words(&subject_words) {
        let mut filter = filter;
        filter.zone = None;
        filter.stack_kind = None;
        filter.cast_by = None;
        (filter, vec![Zone::Hand])
    } else {
        normalize_granted_alternative_spell_filter(filter)
    };
    let mut abilities = Vec::new();
    for zone in zones {
        if zone != Zone::Hand {
            continue;
        }
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::emerge_from_cards_mana_cost(),
            filter.clone(),
            zone,
        );
        let mut ability = StaticAbilityAst::Static(StaticAbility::grants(spec));
        if let Some(condition) = condition.clone() {
            ability = StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            };
        }
        abilities.push(ability);
    }
    Ok((!abilities.is_empty()).then_some(abilities))
}

pub(crate) fn find_source_reference_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    let mut token_indices = Vec::new();
    let mut token_words = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if let Some(word) = token.as_word() {
            token_indices.push(idx);
            token_words.push(word);
        }
    }

    for word_start in 0..token_words.len() {
        if is_source_reference_words(&token_words[word_start..]) {
            return token_indices.get(word_start).copied();
        }
    }
    None
}

pub(crate) fn object_filter_specificity_score(filter: &ObjectFilter) -> usize {
    let mut score = 0usize;
    if !filter.any_of.is_empty() {
        score += 12;
        score += filter
            .any_of
            .iter()
            .map(object_filter_specificity_score)
            .sum::<usize>();
    }
    score += filter.tagged_constraints.len() * 20;
    score += filter.card_types.len() * 10;
    score += filter.all_card_types.len() * 10;
    score += filter.subtypes.len() * 8;
    score += filter.excluded_subtypes.len() * 8;
    score += usize::from(filter.controller.is_some()) * 6;
    score += usize::from(filter.owner.is_some()) * 6;
    score += usize::from(filter.zone.is_some()) * 4;
    score += usize::from(filter.other) * 3;
    score += usize::from(filter.token || filter.nontoken) * 3;
    score += usize::from(filter.tapped || filter.untapped) * 2;
    score += usize::from(
        filter.attacking
            || filter.nonattacking
            || filter.blocking
            || filter.nonblocking
            || filter.blocked
            || filter.unblocked,
    ) * 2;
    score += usize::from(filter.is_commander || filter.noncommander) * 2;
    score += usize::from(filter.colorless || filter.multicolored || filter.monocolored) * 2;
    score += usize::from(filter.with_counter.is_some() || filter.without_counter.is_some()) * 4;
    score += usize::from(filter.entered_battlefield_this_turn) * 2;
    score += usize::from(filter.entered_battlefield_controller.is_some()) * 2;
    score += usize::from(filter.was_dealt_damage_this_turn) * 2;
    score += usize::from(filter.dealt_damage_to_player_this_turn.is_some()) * 2;
    score += usize::from(!filter.excluded_card_types.is_empty()) * 2;
    score += usize::from(!filter.excluded_supertypes.is_empty()) * 2;
    score += usize::from(!filter.excluded_colors.is_empty()) * 2;
    score += usize::from(!filter.excluded_static_abilities.is_empty()) * 2;
    score += usize::from(!filter.excluded_ability_markers.is_empty()) * 2;
    score += usize::from(filter.colors.is_some()) * 2;
    score += usize::from(filter.chosen_color) * 3;
    score += usize::from(filter.chosen_creature_type) * 3;
    score += usize::from(filter.excluded_chosen_creature_type) * 3;
    score += usize::from(filter.power.is_some() || filter.toughness.is_some()) * 2;
    score
}

pub(crate) fn parse_best_object_filter_suffix(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, usize, ObjectFilter)> = None;
    for start in 0..tokens.len() {
        if tokens[start].as_word().is_none() {
            continue;
        }
        let mut other = false;
        let mut candidate = &tokens[start..];
        if candidate
            .first()
            .is_some_and(|token| ANTHEM_OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token))
        {
            other = true;
            candidate = &candidate[1..];
        }
        if candidate.is_empty() {
            continue;
        }
        let candidate_words = crate::runtime_backend::token_word_refs(candidate);
        if ANTHEM_IT_OR_THEM_PATTERN.matches_words(&candidate_words) {
            continue;
        }
        let Ok(filter) = parse_object_filter(candidate, other) else {
            continue;
        };
        let score = object_filter_specificity_score(&filter);
        if best
            .as_ref()
            .is_none_or(|(best_score, _, _)| score > *best_score)
        {
            best = Some((score, start, filter));
        }
    }
    best.map(|(_, start, filter)| {
        if start > 0 {
            crate::parse_loss::record(
                "suffix_object_filter_recovery",
                format!(
                    "parsed '{}' as suffix of '{}'",
                    crate::runtime_backend::token_word_refs(&tokens[start..]).join(" "),
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ),
            );
        }
        filter
    })
}

fn subject_branch_looks_type_like(filter: &ObjectFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
}

fn parse_shared_suffix_and_subject_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, ObjectFilter)> = None;

    for (and_idx, token) in tokens.iter().enumerate() {
        if !ANTHEM_AND_WORD_PATTERN.matches_token(token) {
            continue;
        }

        let left_branch = trim_commas(&tokens[..and_idx]);
        let right_tail = trim_commas(&tokens[and_idx + 1..]);
        if left_branch.is_empty() || right_tail.len() < 2 {
            continue;
        }

        let Ok(left_branch_filter) = parse_object_filter(&left_branch, false) else {
            continue;
        };
        if !subject_branch_looks_type_like(&left_branch_filter) {
            continue;
        }

        for split_idx in 1..right_tail.len() {
            let right_branch = trim_commas(&right_tail[..split_idx]);
            let shared_suffix = trim_commas(&right_tail[split_idx..]);
            if right_branch.is_empty() || shared_suffix.is_empty() {
                continue;
            }

            let Some(shared_head) = shared_suffix.first().and_then(OwnedLexToken::as_word) else {
                continue;
            };
            if !matches!(
                shared_head,
                "you"
                    | "your"
                    | "that"
                    | "those"
                    | "with"
                    | "without"
                    | "named"
                    | "in"
                    | "from"
                    | "on"
                    | "among"
                    | "under"
                    | "during"
            ) {
                continue;
            }

            let Ok(right_branch_filter) = parse_object_filter(&right_branch, false) else {
                continue;
            };
            if !subject_branch_looks_type_like(&right_branch_filter) {
                continue;
            }

            let mut left_full = left_branch.clone();
            left_full.extend(shared_suffix.iter().cloned());
            let mut right_full = right_branch.clone();
            right_full.extend(shared_suffix.iter().cloned());

            let Ok(left_filter) = parse_object_filter(&left_full, false) else {
                continue;
            };
            let Ok(right_filter) = parse_object_filter(&right_full, false) else {
                continue;
            };
            if left_filter == right_filter {
                continue;
            }

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![left_filter.clone(), right_filter.clone()];
            let score = object_filter_specificity_score(&left_filter)
                + object_filter_specificity_score(&right_filter)
                + shared_suffix.len();
            if best
                .as_ref()
                .is_none_or(|(best_score, _)| score > *best_score)
            {
                best = Some((score, disjunction));
            }
        }
    }

    best.map(|(_, filter)| filter)
}

pub(crate) fn parse_anthem_subject(
    tokens: &[OwnedLexToken],
) -> Result<AnthemSubjectAst, CardTextError> {
    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(tokens);
    if let Some(subject) = first_spell_each_turn_subject_tokens(tokens)? {
        return Ok(subject);
    }
    if FIRST_SPELL_EACH_TURN_SUBJECT_PATTERN.matches_words(&subject_words) {
        return Ok(AnthemSubjectAst::Filter(
            ObjectFilter::spell()
                .cast_by(PlayerFilter::You)
                .first_spell_cast_each_turn(),
        ));
    }
    if SOURCE_IT_PATTERN.matches_words(&subject_words) {
        return Ok(AnthemSubjectAst::Source);
    }
    if is_source_reference_words(&subject_words) {
        return Ok(AnthemSubjectAst::Source);
    }
    if let Ok(filter) = parse_object_filter(tokens, false)
        && filter.in_combat_with_source
    {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_enchanted_player_controls_subject(tokens)? {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_shared_suffix_and_subject_filter(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if let Some(filter) = parse_best_object_filter_suffix(tokens) {
        return Ok(AnthemSubjectAst::Filter(filter));
    }
    if find_source_reference_start(tokens).is_some() {
        return Ok(AnthemSubjectAst::Source);
    }
    Err(CardTextError::ParseError(format!(
        "unsupported anthem subject (clause: '{}')",
        crate::runtime_backend::token_word_refs(tokens).join(" ")
    )))
}

fn parse_enchanted_player_controls_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !ENCHANTED_PLAYER_CONTROLS_SUFFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    let enchanted_idx = words.len().saturating_sub(3);
    if enchanted_idx == 0 || enchanted_idx + 3 != words.len() {
        return Ok(None);
    }
    let Some(prefix_end) = token_index_for_word_index(tokens, enchanted_idx) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter(&tokens[..prefix_end], false)?;
    filter.controller = Some(PlayerFilter::TaggedPlayer(crate::TagKey::from("enchanted")));
    Ok(Some(filter))
}

fn infer_attached_subject_filter_from_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<ObjectFilter> {
    let condition_tokens = trim_edge_punctuation(tokens);
    let condition_words = crate::runtime_backend::token_word_refs(&condition_tokens);
    let attached_subject_len = ATTACHED_CONDITION_SUBJECT_PREFIX_PATTERN
        .matches_words(&condition_words)
        .then_some(2usize)?;
    let subject_end = token_index_for_word_index(&condition_tokens, attached_subject_len)?;
    parse_object_filter(&condition_tokens[..subject_end], false).ok()
}

fn parse_anthem_subject_with_attached_fallback(
    tokens: &[OwnedLexToken],
    attached_subject_filter: Option<&ObjectFilter>,
) -> Result<AnthemSubjectAst, CardTextError> {
    if SOURCE_IT_PATTERN.matches_words(&crate::runtime_backend::token_word_refs(tokens))
        && let Some(filter) = attached_subject_filter
    {
        return Ok(AnthemSubjectAst::Filter(filter.clone()));
    }
    parse_anthem_subject(tokens)
}

fn infer_attached_subject_filter_from_condition_expr(
    condition: Option<&crate::ConditionExpr>,
) -> Option<ObjectFilter> {
    match condition {
        Some(crate::ConditionExpr::EnchantedPermanentIsCreature)
        | Some(crate::ConditionExpr::EnchantedPermanentIsLand)
        | Some(crate::ConditionExpr::EnchantedPermanentIsEquipment)
        | Some(crate::ConditionExpr::EnchantedPermanentIsVehicle) => {
            Some(ObjectFilter::tagged("enchanted"))
        }
        _ => None,
    }
}

pub(crate) fn parse_static_quantity_prefix(
    tokens: &[OwnedLexToken],
    allow_default_one: bool,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    parse_quantity_comparison_prefix(tokens, allow_default_one, true, "static condition")
}

pub(crate) fn parse_permanent_card_count_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let token_words = crate::runtime_backend::token_word_refs(tokens);
    if !PERMANENT_CARD_PREFIX_PATTERN.matches_words(&token_words) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];

    for (idx, word) in token_words.iter().enumerate() {
        if let Some(zone) = parse_zone_word(word) {
            filter.zone = Some(zone);
            if idx > 0 {
                match token_words[idx - 1] {
                    "your" => filter.owner = Some(PlayerFilter::You),
                    "opponent" | "opponents" => filter.owner = Some(PlayerFilter::Opponent),
                    _ => {}
                }
            }
        }
    }

    filter.zone.map(|_| filter)
}

fn strip_static_condition_intro(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let word_storage = AnthemNormalizedWords::new(tokens);
    let words = word_storage.word_refs();
    let intro_word_count = if CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&words) {
        3
    } else if words
        .first()
        .is_some_and(|word| ANTHEM_AS_WORD_PATTERN.matches_word(word))
    {
        1
    } else {
        0
    };

    if intro_word_count == 0 {
        return tokens;
    }

    token_index_for_word_index(tokens, intro_word_count)
        .map(|token_idx| &tokens[token_idx..])
        .unwrap_or(tokens)
}

pub(crate) fn parse_static_condition_clause(
    tokens: &[OwnedLexToken],
) -> Result<crate::ConditionExpr, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let clause_word_storage = AnthemNormalizedWords::new(&tokens);
    let clause_words = clause_word_storage.word_refs();
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing condition clause after 'as long as'".to_string(),
        ));
    }

    if let Some(condition) = parse_cards_in_hand_static_condition(&tokens) {
        return Ok(condition);
    }

    if let Some(condition) = parse_life_total_static_condition(&tokens) {
        return Ok(condition);
    }

    if OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::OpponentLostLifeThisTurn);
    }

    if YOU_NOT_CAST_SPELL_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                player: PlayerFilter::You,
                count: 1,
            },
        )));
    }

    if YOU_CAST_SPELL_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
            player: PlayerFilter::You,
            count: 1,
        });
    }

    if NO_CARDS_IN_YOUR_LIBRARY_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(
                ObjectFilter::default()
                    .in_zone(Zone::Library)
                    .owned_by(PlayerFilter::You),
            ),
            comparison: crate::effect::Comparison::Equal(0),
            display: Some("there are no cards in your library".to_string()),
        });
    }

    if clause_words.len() >= 4 && clause_words.get(0..2) == Some(&["you", "have"]) {
        let tail_words = &clause_words[2..];
        if tail_words.last().copied() == Some("life") {
            let quantity_tokens = crate::runtime_backend::lexer::synthetic_word_tokens(
                &tail_words[..tail_words.len() - 1],
            );
            if let Some((life, used)) = parse_less_than_or_equal_quantity_prefix(
                &quantity_tokens,
                false,
                false,
                "life-total static condition",
            )
            .ok()
            .flatten()
                && used == tail_words.len() - 1
            {
                return Ok(crate::ConditionExpr::LifeTotalOrLess(life as i32));
            }
        }
    }

    if let Some(condition) = parse_devotion_static_condition(&clause_words)? {
        return Ok(condition);
    }

    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_subject_status_condition(&tokens)
            .and_then(|condition| condition.condition_expr())
    {
        return Ok(condition);
    }
    if SOURCE_IS_ON_BATTLEFIELD_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::SourceIsInZone(Zone::Battlefield));
    }
    if SOURCE_DEVOURED_CREATURES_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::SourceDevouredCreaturesOrMore(1));
    }
    if SOURCE_IS_SOULBOND_PAIRED_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::SourceIsSoulbondPaired);
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_subject_descriptor_condition(&tokens)
    {
        return Ok(condition.condition_expr(clause_words.join(" ")));
    }
    if SOURCE_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::SourceAttackedThisTurn);
    }
    if YOU_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::AttackedThisTurn);
    }
    if SOURCE_ENTERED_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        let mut filter = ObjectFilter::source();
        filter.entered_battlefield_this_turn = true;
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
            display: Some(clause_words.join(" ")),
        });
    }
    if YOUR_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::YourTurn);
    }
    if SOURCE_POWER_EVEN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Err(CardTextError::ParseError(
            "unsupported source power parity condition (clause: 'this power is even')".to_string(),
        ));
    }
    if SOURCE_POWER_ODD_CONDITION_PATTERN.matches_words(&clause_words) {
        return Err(CardTextError::ParseError(
            "unsupported source power parity condition (clause: 'this power is odd')".to_string(),
        ));
    }
    if NOT_YOUR_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::YourTurn,
        )));
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_player_status_condition(&tokens)
    {
        return Ok(condition.condition_expr());
    }
    if clause_words.len() >= 4
        && clause_words.first() == Some(&"x")
        && clause_words.get(1) == Some(&"is")
        && let Ok((comparison, used)) =
            parse_static_quantity_prefix(tokens.get(2..).unwrap_or_default(), false)
        && used + 2 == clause_words.len()
        && let Some(count) = match comparison {
            crate::effect::Comparison::GreaterThanOrEqual(value) if value >= 0 => {
                Some(value as u32)
            }
            crate::effect::Comparison::GreaterThan(value) if value >= -1 => {
                Some((value + 1) as u32)
            }
            _ => None,
        }
    {
        return Ok(crate::ConditionExpr::XValueAtLeast(count));
    }
    if let Some(condition) =
        crate::runtime_backend::grammar::conditions::parse_player_achievement_condition(&tokens)
    {
        return Ok(condition.condition_expr());
    }
    if let Some(condition) = parse_cards_drawn_this_turn_static_condition(&tokens) {
        return Ok(condition);
    }
    if YOUR_LIFE_HALF_STARTING_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(
            crate::ConditionExpr::PlayerLifeAtMostHalfStartingLifeTotal {
                player: PlayerFilter::You,
            },
        );
    }

    if let Some(is_idx) = ANTHEM_IS_OR_ARE_WORD_PATTERN.find_word(&clause_words) {
        let subject_words = &clause_words[..is_idx];
        let source_pronoun_subject =
            ANTHEM_SOURCE_PRONOUN_SUBJECT_PATTERN.matches_words(subject_words);
        if !subject_words.is_empty()
            && (is_source_reference_words(subject_words) || source_pronoun_subject)
        {
            let remainder_words = &clause_words[is_idx + 1..];
            if SOURCE_IN_GRAVEYARD_TAIL_PATTERN.matches_words(remainder_words) {
                let mut filter = ObjectFilter::source();
                filter.zone = Some(Zone::Graveyard);
                return Ok(crate::ConditionExpr::CountComparison {
                    count: AnthemCountExpression::MatchingFilter(filter),
                    comparison: crate::effect::Comparison::GreaterThanOrEqual(1),
                    display: Some(clause_words.join(" ")),
                });
            }
        }
    }

    if let Some(conjoined) = parse_conjoined_static_condition_clause(&tokens) {
        return Ok(conjoined);
    }

    if THERE_IS_OR_ARE_PREFIX_PATTERN.matches_words(&clause_words) {
        if let Some((metric, threshold)) = parse_graveyard_metric_threshold_condition(&tokens)? {
            if metric == crate::static_abilities::GraveyardCountMetric::CardTypes {
                return Ok(crate::ConditionExpr::PlayerHasCardTypesInGraveyardOrMore {
                    player: PlayerFilter::You,
                    count: threshold,
                });
            }
        }

        let quantified = &tokens[2..];
        let (comparison, used) = parse_static_quantity_prefix(
            quantified,
            clause_words
                .get(1)
                .is_some_and(|word| IS_WORD_PATTERN.matches_word(word)),
        )?;
        let mut filter_tokens = &quantified[used..];
        if filter_tokens
            .first()
            .is_some_and(|token| CARD_OR_CARDS_WORD_PATTERN.matches_token(token))
        {
            filter_tokens = &filter_tokens[1..];
        }
        if IN_YOUR_GRAVEYARD_TAIL_PATTERN
            .matches_words(&crate::runtime_backend::token_word_refs(filter_tokens))
        {
            let Some((operator, value)) =
                crate::runtime_backend::util::comparison_to_value_comparison_operator(comparison)
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported graveyard card-count condition (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            return Ok(crate::ConditionExpr::ValueComparison {
                left: crate::effect::Value::CardsInGraveyard(PlayerFilter::You),
                operator,
                right: crate::effect::Value::Fixed(value),
            });
        }
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object phrase in static condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let filter_word_view = AnthemNormalizedWords::new(filter_tokens);
        let filter_words = filter_word_view.word_refs();
        if filter_words.starts_with(&["different", "kinds", "of", "counters", "among"])
            && let Some(among_filter_start) = filter_word_view.token_index_for_word_index(5)
        {
            let filter = parse_object_filter(&filter_tokens[among_filter_start..], false).map_err(
                |_| {
                    CardTextError::ParseError(format!(
                        "unsupported distinct-counter-kind filter in static condition (clause: '{}')",
                        clause_words.join(" ")
                    ))
                },
            )?;
            return Ok(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::DistinctCounterTypesAmong(filter),
                comparison,
                display: Some(clause_words.join(" ")),
            });
        }
        if let Some(counter_word_idx) = COUNTER_OR_COUNTERS_WORD_PATTERN.find_word(&filter_words)
            && counter_word_idx > 0
            && filter_words
                .get(counter_word_idx + 1)
                .is_some_and(|word| ANTHEM_AMONG_WORD_PATTERN.matches_word(word))
            && let Some(counter_type) = parse_counter_type_word(filter_words[counter_word_idx - 1])
            && let Some(among_filter_start) =
                filter_word_view.token_index_for_word_index(counter_word_idx + 2)
        {
            let filter =
                parse_object_filter(&filter_tokens[among_filter_start..], false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported counter-among filter in static condition (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?;
            return Ok(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::CountersAmong(filter, counter_type),
                comparison,
                display: Some(clause_words.join(" ")),
            });
        }

        let filter = if let Some(in_idx) = ANTHEM_IN_WORD_PATTERN.find_word(&filter_words) {
            let subject_words = &filter_words[..in_idx];
            let zone_tail = &filter_words[in_idx..];
            if is_source_reference_words(subject_words)
                && SOURCE_IN_GRAVEYARD_TAIL_PATTERN.matches_words(zone_tail)
            {
                let mut filter = ObjectFilter::source();
                filter.zone = Some(Zone::Graveyard);
                filter
            } else {
                parse_permanent_card_count_filter(filter_tokens)
                    .or_else(|| parse_object_filter(filter_tokens, false).ok())
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported counted object phrase in static condition (clause: '{}')",
                            clause_words.join(" ")
                        ))
                    })?
            }
        } else {
            parse_permanent_card_count_filter(filter_tokens)
                .or_else(|| parse_object_filter(filter_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported counted object phrase in static condition (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?
        };
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    let count_condition_tokens = strip_static_condition_intro(&tokens);

    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            count_condition_tokens,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: true,
                allow_defending_player: false,
                bind_filter_controller_to_subject: true,
                allow_different_powers_tail: false,
                default_filter_zone: None,
            },
        )
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(control_condition.filter),
            comparison: control_condition.comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    if let Some(ownership_condition) =
        crate::runtime_backend::grammar::conditions::parse_ownership_condition(
            count_condition_tokens,
            crate::runtime_backend::grammar::conditions::OwnershipConditionOptions {
                allow_opponent_players: true,
                bind_filter_owner_to_subject: true,
                default_filter_zone: None,
            },
        )
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(ownership_condition.filter),
            comparison: ownership_condition.comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    if ANTHEM_ENTERED_WORD_MARKER_PATTERN.matches_words(&clause_words)
        && let Ok((comparison, used)) = parse_static_quantity_prefix(&tokens, true)
        && let Ok(filter) = parse_object_filter(
            &tokens[used..],
            tokens
                .get(used)
                .is_some_and(|token| ANTHEM_OTHER_OR_ANOTHER_WORD_PATTERN.matches_token(token)),
        )
        && (filter.entered_battlefield_this_turn || filter.entered_battlefield_controller.is_some())
    {
        return Ok(crate::ConditionExpr::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    if YOU_COMMITTED_CRIME_THIS_TURN_CONDITION_PATTERN.matches_words(&clause_words) {
        return Ok(crate::ConditionExpr::PlayerCommittedCrimeThisTurn {
            player: PlayerFilter::You,
        });
    }

    if let Some(has_idx) = HAS_OR_HAVE_WORD_PATTERN.find_word(&clause_words) {
        let subject_words = &clause_words[..has_idx];
        let source_pronoun_subject =
            ANTHEM_SOURCE_PRONOUN_SUBJECT_PATTERN.matches_words(subject_words);
        if !subject_words.is_empty()
            && (is_source_reference_words(subject_words) || source_pronoun_subject)
        {
            let quantified = &tokens[has_idx + 1..];
            let (comparison, used) = parse_static_quantity_prefix(quantified, true)?;
            let counter_tokens = &quantified[used..];
            let counter_words = crate::runtime_backend::token_word_refs(counter_tokens);
            let Some(counter_word_idx) = COUNTER_OR_COUNTERS_WORD_PATTERN.find_word(&counter_words)
            else {
                return Err(CardTextError::ParseError(format!(
                    "missing counter phrase in static condition (clause: '{}')",
                    clause_words.join(" ")
                )));
            };

            let counter_type = if counter_word_idx > 0 {
                parse_counter_type_word(counter_words[counter_word_idx - 1])
            } else {
                None
            };

            let tail = &counter_words[counter_word_idx + 1..];
            if !ON_SOURCE_COUNTER_TAIL_PATTERN.matches_words(tail) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported source-counter condition tail (clause: '{}')",
                    clause_words.join(" ")
                )));
            }

            let mut filter = ObjectFilter::source();
            filter.with_counter = Some(match counter_type {
                Some(counter_type) => crate::filter::CounterConstraint::Typed(counter_type),
                None => crate::filter::CounterConstraint::Any,
            });
            return Ok(crate::ConditionExpr::CountComparison {
                count: AnthemCountExpression::MatchingFilter(filter),
                comparison,
                display: Some(clause_words.join(" ")),
            });
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported static condition clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_devotion_static_condition(
    words: &[&str],
) -> Result<Option<crate::ConditionExpr>, CardTextError> {
    let Some(devotion_idx) = ANTHEM_DEVOTION_WORD_PATTERN.find_word(words) else {
        return Ok(None);
    };
    let Some(to_idx) = ANTHEM_TO_WORD_PATTERN
        .find_word(&words[devotion_idx + 1..])
        .map(|idx| devotion_idx + 1 + idx)
    else {
        return Ok(None);
    };
    let Some(is_idx) = IS_WORD_PATTERN
        .find_word(&words[to_idx + 1..])
        .map(|idx| to_idx + 1 + idx)
    else {
        return Ok(None);
    };

    let pre_devotion_words = &words[..devotion_idx];
    let player = if ANTHEM_YOUR_WORD_PATTERN.matches_last_word(pre_devotion_words) {
        PlayerFilter::You
    } else if ANTHEM_THEIR_WORD_PATTERN.matches_last_word(pre_devotion_words) {
        PlayerFilter::IteratedPlayer
    } else if ANTHEM_OPPONENT_WORD_PATTERN.matches_last_word(pre_devotion_words) {
        PlayerFilter::Opponent
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported devotion player in static condition (clause: '{}')",
            words.join(" ")
        )));
    };

    let mut devotion_values = Vec::new();
    for word in &words[to_idx + 1..is_idx] {
        if ANTHEM_AND_OR_COMMA_WORD_PATTERN.matches_word(word) {
            continue;
        }
        let Some(color) = crate::color::Color::from_name(word) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported devotion color '{}' in static condition (clause: '{}')",
                word,
                words.join(" ")
            )));
        };
        devotion_values.push(crate::effect::Value::Devotion {
            player: player.clone(),
            color,
        });
    }

    let mut devotion_values = devotion_values.into_iter();
    let Some(mut left) = devotion_values.next() else {
        return Err(CardTextError::ParseError(format!(
            "missing devotion color in static condition (clause: '{}')",
            words.join(" ")
        )));
    };
    for value in devotion_values {
        left = crate::effect::Value::Add(Box::new(left), Box::new(value));
    }

    let (operator, right_start) = match &words[is_idx + 1..] {
        ["less", "than", "or", "equal", "to", rest @ ..] => (
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            rest,
        ),
        ["less", "than", rest @ ..] => (crate::effect::ValueComparisonOperator::LessThan, rest),
        ["greater", "than", "or", "equal", "to", rest @ ..] => (
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            rest,
        ),
        ["greater", "than", rest @ ..] => {
            (crate::effect::ValueComparisonOperator::GreaterThan, rest)
        }
        ["equal", "to", rest @ ..] => (crate::effect::ValueComparisonOperator::Equal, rest),
        ["not", "equal", "to", rest @ ..] => {
            (crate::effect::ValueComparisonOperator::NotEqual, rest)
        }
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported devotion comparison in static condition (clause: '{}')",
                words.join(" ")
            )));
        }
    };

    let Some(amount_word) = right_start.first().copied() else {
        return Err(CardTextError::ParseError(format!(
            "missing devotion comparison value in static condition (clause: '{}')",
            words.join(" ")
        )));
    };
    let Some(amount) = parse_named_number(amount_word) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported devotion comparison value '{}' in static condition (clause: '{}')",
            amount_word,
            words.join(" ")
        )));
    };

    Ok(Some(crate::ConditionExpr::ValueComparison {
        left,
        operator,
        right: crate::effect::Value::Fixed(amount as i32),
    }))
}

fn parse_conjoined_static_condition_clause(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let and_positions = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| ANTHEM_AND_WORD_PATTERN.matches_word(word).then_some(idx))
        .collect::<Vec<_>>();
    for and_word_idx in and_positions {
        let and_token_idx = token_index_for_word_index(tokens, and_word_idx)?;
        let left_tokens = trim_commas(&tokens[..and_token_idx]);
        let right_tokens = trim_commas(&tokens[and_token_idx + 1..]);
        if left_tokens.is_empty() || right_tokens.is_empty() {
            continue;
        }
        let Ok(left) = parse_static_condition_clause(&left_tokens) else {
            continue;
        };
        let right = parse_conjoined_static_condition_clause(&right_tokens)
            .or_else(|| parse_static_condition_clause(&right_tokens).ok());
        if let Some(right) = right {
            return Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)));
        }
    }
    None
}

fn parse_cards_drawn_this_turn_static_condition(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let clause_word_view = AnthemNormalizedWords::new(tokens);
    let clause_words = clause_word_view.word_refs();
    let (player, count_start_word_idx) = match clause_words.as_slice() {
        ["youve", "drawn", ..] => (PlayerFilter::You, 2usize),
        ["you", "have", "drawn", ..] | ["you", "ve", "drawn", ..] => (PlayerFilter::You, 3usize),
        ["an", "opponent", "has", "drawn", ..] => (PlayerFilter::Opponent, 4usize),
        ["opponent", "has", "drawn", ..] => (PlayerFilter::Opponent, 3usize),
        ["opponents", "have", "drawn", ..] => (PlayerFilter::Opponent, 3usize),
        ["a", "player", "has", "drawn", ..] => (PlayerFilter::Any, 4usize),
        ["player", "has", "drawn", ..] => (PlayerFilter::Any, 3usize),
        ["players", "have", "drawn", ..] => (PlayerFilter::Any, 3usize),
        _ => return None,
    };

    let count_start_idx = clause_word_view.token_index_for_word_index(count_start_word_idx)?;
    let count_tokens = tokens.get(count_start_idx..)?;
    let (count, used) = parse_number(count_tokens)?;
    let tail_tokens = count_tokens.get(used..)?;
    let tail_word_view = AnthemNormalizedWords::new(tail_tokens);
    let tail_words = tail_word_view.word_refs();
    if !word_slice_eq_any(
        &tail_words,
        &[
            &["or", "more", "cards", "this", "turn"],
            &["or", "more", "card", "this", "turn"],
        ],
    ) {
        return None;
    }

    Some(crate::ConditionExpr::ValueComparison {
        left: crate::effect::Value::MaxCardsDrawnThisTurn(player),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(count as i32),
    })
}

fn parse_cards_in_hand_static_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    crate::runtime_backend::grammar::conditions::parse_player_cards_in_hand_condition(tokens)?
        .condition_expr()
}

fn parse_life_total_static_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(tokens)?
        .condition_expr()
}

pub(crate) fn parse_anthem_for_each_expression(
    tokens: &[OwnedLexToken],
) -> Result<AnthemCountExpression, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let token_words = crate::runtime_backend::token_word_refs(&tokens);
    if !ANTHEM_FOR_EACH_PREFIX_PATTERN.matches_words(&token_words) {
        return Err(CardTextError::ParseError(
            "missing 'for each' in anthem scaling clause".to_string(),
        ));
    }
    let rest = &tokens[2..];
    if rest.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object phrase after 'for each'".to_string(),
        ));
    }

    let rest_words = crate::runtime_backend::token_word_refs(rest);
    if ANTHEM_AFFECTED_ATTACKED_THIS_TURN_PATTERN.matches_words(&rest_words) {
        return Ok(AnthemCountExpression::AffectedAttackedThisTurn);
    }

    if ANTHEM_AFFECTED_COLORS_PATTERN.matches_words(&rest_words) {
        return Ok(AnthemCountExpression::ColorsOfAffected);
    }

    if ANTHEM_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&rest_words) {
        let filter_tokens = &rest[4..];
        let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported domain count filter (clause: '{}')",
                crate::runtime_backend::token_word_refs(&tokens).join(" ")
            ))
        })?;
        return Ok(AnthemCountExpression::BasicLandTypesAmong(filter));
    }

    if ANTHEM_CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&rest_words) {
        let filter_tokens = &rest[3..];
        let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported creature-type count filter (clause: '{}')",
                crate::runtime_backend::token_word_refs(&tokens).join(" ")
            ))
        })?;
        return Ok(AnthemCountExpression::CreatureTypesAmong(filter));
    }

    if let Some(attached_idx) = anthem_token_offset(rest, |token| {
        ANTHEM_ATTACHED_WORD_PATTERN.matches_token(token)
    }) {
        let filter_tokens = &rest[..attached_idx];
        let tail_words = crate::runtime_backend::token_word_refs(&rest[attached_idx + 1..]);
        if ANTHEM_ATTACHED_TO_SOURCE_TAIL_PATTERN.matches_words(&tail_words) {
            let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attached-object filter in anthem scaling clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(&tokens).join(" ")
                ))
            })?;
            return Ok(AnthemCountExpression::AttachedToSource(filter));
        }
    }

    if let Some(player) = parse_commander_cast_count_player(rest) {
        return Ok(AnthemCountExpression::CommanderCastCount(player));
    }

    if ANTHEM_UNSPENT_GREEN_MANA_YOU_HAVE_PATTERN.matches_words(&rest_words) {
        return Ok(AnthemCountExpression::UnspentMana {
            player: PlayerFilter::You,
            symbol: crate::mana::ManaSymbol::Green,
        });
    }

    if let Some(filter) = parse_compound_anthem_count_filter(rest) {
        return Ok(AnthemCountExpression::MatchingFilter(filter));
    }

    if let Some(counter_word_idx) = COUNTER_OR_COUNTERS_WORD_PATTERN.find_word(&rest_words)
        && counter_word_idx > 0
        && let Some(counter_type) = parse_counter_type_word(rest_words[counter_word_idx - 1])
    {
        let tail_words = &rest_words[counter_word_idx + 1..];
        if ON_SOURCE_COUNTER_TAIL_PATTERN.matches_words(tail_words) {
            return Ok(AnthemCountExpression::CountersOnSource(counter_type));
        }
    }

    let filter = parse_object_filter(rest, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported 'for each' filter in anthem clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(&tokens).join(" ")
        ))
    })?;
    Ok(AnthemCountExpression::MatchingFilter(filter))
}

fn parse_compound_anthem_count_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let filter_words = crate::runtime_backend::token_word_refs(tokens);
    let should_try_split =
        ANTHEM_GRAVEYARD_CONJUNCTION_SPLIT_MARKER_PATTERN.matches_words(&filter_words);
    if !should_try_split {
        return None;
    }

    let mut segments = Vec::new();
    let mut start = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if ANTHEM_AND_WORD_PATTERN.matches_token(token)
            && tokens
                .get(idx + 1)
                .is_some_and(|next| ANTHEM_EACH_OR_EVERY_WORD_PATTERN.matches_token(next))
        {
            if start == idx {
                return None;
            }
            segments.push(&tokens[start..idx]);
            start = idx + 1;
        }
    }
    if segments.is_empty() {
        return None;
    }
    segments.push(&tokens[start..]);

    let mut branches = Vec::new();
    for segment in segments {
        let mut segment = trim_commas(segment);
        if segment
            .first()
            .is_some_and(|token| ANTHEM_EACH_OR_EVERY_WORD_PATTERN.matches_token(token))
        {
            segment.drain(..1);
        }
        if segment.is_empty() {
            return None;
        }
        branches.push(parse_object_filter(&segment, false).ok()?);
    }

    if branches.len() < 2 {
        return None;
    }

    let mut combined = ObjectFilter::default();
    combined.any_of = branches;
    Some(combined)
}

pub(crate) fn parse_anthem_prefix_condition(
    tokens: &[OwnedLexToken],
    get_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    if token_slice_starts_with(tokens, &["during", "turns", "other", "than", "yours"]) {
        let subject_start = anthem_token_offset(&tokens[..get_idx], |token| token.is_comma())
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(5);
        return Ok((
            Some(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::YourTurn,
            ))),
            subject_start,
        ));
    }
    if token_slice_starts_with(tokens, &["during", "your", "turn"]) {
        let subject_start = anthem_token_offset(&tokens[..get_idx], |token| token.is_comma())
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(3);
        return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
    }

    if token_slice_starts_with(tokens, &["as", "long", "as"]) {
        let subject_start = anthem_token_offset(&tokens[..get_idx], |token| token.is_comma())
            .map(|idx| idx + 1)
            .or_else(|| infer_as_long_as_subject_start(tokens, get_idx))
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

fn infer_as_long_as_subject_start(tokens: &[OwnedLexToken], action_idx: usize) -> Option<usize> {
    if action_idx <= 3 {
        return None;
    }

    let mut word_token_indices = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_some() {
            word_token_indices.push(idx);
        }
    }
    if word_token_indices.is_empty() {
        return None;
    }

    let action_word_idx = word_token_indices
        .iter()
        .enumerate()
        .find_map(|(idx, token_idx)| (*token_idx == action_idx).then_some(idx))
        .unwrap_or(word_token_indices.len());
    if action_word_idx <= 3 {
        return None;
    }

    for subject_word_idx in 4..action_word_idx {
        let subject_start = word_token_indices[subject_word_idx];
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        if condition_tokens.is_empty() {
            continue;
        }
        if parse_static_condition_clause(&condition_tokens).is_err() {
            continue;
        }

        let subject_tokens = trim_commas(&tokens[subject_start..action_idx]);
        if subject_tokens.is_empty() {
            continue;
        }
        if parse_anthem_subject(&subject_tokens).is_ok() {
            return Some(subject_start);
        }
    }

    None
}

pub(crate) fn parse_anthem_clause(
    tokens: &[OwnedLexToken],
    get_idx: usize,
    tail_end: usize,
) -> Result<ParsedAnthemClause, CardTextError> {
    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, get_idx)?;
    let prefix_attached_subject =
        if subject_start > 3 && token_slice_starts_with(tokens, &["as", "long", "as"]) {
            infer_attached_subject_filter_from_condition_tokens(&tokens[3..subject_start])
        } else {
            None
        };
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing anthem subject (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let mut modifier_idx = get_idx + 1;
    if tokens
        .get(modifier_idx)
        .is_some_and(|token| ANTHEM_ARTICLE_WORD_PATTERN.matches_token(token))
        && tokens
            .get(modifier_idx + 1)
            .is_some_and(|token| ANTHEM_ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        modifier_idx += 2;
    }

    let modifier_tokens = &tokens[modifier_idx..tail_end];
    let modifier_words = AnthemNormalizedWords::new(modifier_tokens);
    let modifier_token = modifier_words.first().ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing power/toughness modifier in anthem clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    let (raw_power, raw_toughness) = parse_pt_modifier_values(modifier_token).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in anthem clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    let modifier_end = modifier_words.token_index_after_words(1).unwrap_or(1);
    let tail_tokens = trim_edge_punctuation(&modifier_tokens[modifier_end..]);
    let mut scale: Option<AnthemCountExpression> = None;
    let mut suffix_condition: Option<crate::ConditionExpr> = None;
    let mut suffix_attached_subject: Option<ObjectFilter> = None;
    if !tail_tokens.is_empty() {
        if token_slice_starts_with(&tail_tokens, &["for", "each"]) {
            scale = Some(parse_anthem_for_each_expression(&tail_tokens)?);
        } else if token_slice_starts_with(&tail_tokens, &["where", "x", "is"]) {
            let x_value = parse_value_binding_clause(&tail_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported where-x anthem clause (clause: '{}')",
                    crate::runtime_backend::token_word_refs(tokens).join(" ")
                ))
            })?;
            scale = Some(match x_value {
                Value::Count(filter) => AnthemCountExpression::MatchingFilter(filter),
                Value::GreatestManaValue(filter) => {
                    AnthemCountExpression::GreatestManaValueAmong(filter)
                }
                Value::CountersOnSource(counter_type) => {
                    AnthemCountExpression::CountersOnSource(counter_type)
                }
                Value::BasicLandTypesAmong(filter) => {
                    AnthemCountExpression::BasicLandTypesAmong(filter)
                }
                Value::CreatureTypesAmong(filter) => {
                    AnthemCountExpression::CreatureTypesAmong(filter)
                }
                Value::Speed(player) => AnthemCountExpression::PlayerSpeed(player),
                _ => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported where-x anthem value (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
            });
        } else if token_slice_starts_with(&tail_tokens, &["as", "long", "as"]) {
            suffix_attached_subject =
                infer_attached_subject_filter_from_condition_tokens(&tail_tokens[3..]);
            suffix_condition = Some(parse_static_condition_clause(&tail_tokens[3..])?);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing anthem clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
    }

    let attached_subject_filter = prefix_attached_subject
        .as_ref()
        .or(suffix_attached_subject.as_ref());
    let subject =
        parse_anthem_subject_with_attached_fallback(&subject_tokens, attached_subject_filter)?;

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_prefix), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    };

    let has_dynamic_component = matches!(raw_power, Value::X | Value::XTimes(_))
        || matches!(raw_toughness, Value::X | Value::XTimes(_));
    let scale_fixed_components = scale.is_some() && !has_dynamic_component;
    let resolve_anthem_value = |component: Value,
                                scale_expr: Option<&AnthemCountExpression>,
                                scale_fixed_components: bool|
     -> Result<AnthemValue, CardTextError> {
        match component {
            Value::Fixed(value) => Ok(match scale_expr {
                Some(scale_expr) if scale_fixed_components => {
                    AnthemValue::scaled(value, scale_expr.clone())
                }
                None => AnthemValue::Fixed(value),
                Some(_) => AnthemValue::Fixed(value),
            }),
            Value::X => {
                if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(1, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )))
                }
            }
            Value::XTimes(multiplier) => {
                if let Some(scale_expr) = scale_expr {
                    Ok(AnthemValue::scaled(multiplier, scale_expr.clone()))
                } else {
                    Err(CardTextError::ParseError(format!(
                        "unsupported X power/toughness modifier without count expression (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )))
                }
            }
            _ => Err(CardTextError::ParseError(format!(
                "invalid power/toughness modifier in anthem clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))),
        }
    };

    let mut power = resolve_anthem_value(raw_power, scale.as_ref(), scale_fixed_components)?;
    let mut toughness =
        resolve_anthem_value(raw_toughness, scale.as_ref(), scale_fixed_components)?;

    // When the anthem affects multiple creatures (subject is a filter rather
    // than "this creature"), any "attached to it" count expression refers to
    // the affected creature, not the anthem source.  Promote
    // AttachedToSource -> AttachedToAffected so the runtime evaluates the
    // count per-creature.
    if matches!(subject, AnthemSubjectAst::Filter(_)) {
        promote_attached_to_affected(&mut power);
        promote_attached_to_affected(&mut toughness);
    }

    parser_trace_stack("parse_static:anthem-clause:matched", tokens);
    Ok(ParsedAnthemClause {
        subject,
        power,
        toughness,
        condition,
    })
}

/// When an anthem targets a filter of creatures (not just the source),
/// "attached to it" refers to the affected creature, not the source.
/// Promote `AttachedToSource` -> `AttachedToAffected` in the anthem value.
fn promote_attached_to_affected(value: &mut AnthemValue) {
    if let AnthemValue::PerCount {
        count: count @ AnthemCountExpression::AttachedToSource(_),
        ..
    } = value
    {
        // Extract the inner filter and replace with AttachedToAffected.
        let AnthemCountExpression::AttachedToSource(filter) = std::mem::replace(
            count,
            AnthemCountExpression::AttachedToAffected(ObjectFilter::default()),
        ) else {
            unreachable!()
        };
        *count = AnthemCountExpression::AttachedToAffected(filter);
    }
}

pub(crate) fn build_anthem_static_ability(clause: &ParsedAnthemClause) -> StaticAbility {
    let mut anthem = match &clause.subject {
        AnthemSubjectAst::Source => Anthem::for_source(0, 0),
        AnthemSubjectAst::Filter(filter) => Anthem::new(filter.clone(), 0, 0),
    }
    .with_values(clause.power.clone(), clause.toughness.clone());

    if let Some(condition) = &clause.condition {
        anthem = anthem.with_condition(condition.clone());
    }

    StaticAbility::new(anthem)
}

#[derive(Debug)]
pub(crate) struct TypeColorAdditionClause {
    pub(crate) added_colors: ColorSet,
    pub(crate) set_colors: ColorSet,
    pub(crate) card_types: Vec<CardType>,
    pub(crate) subtypes: Vec<Subtype>,
}

pub(crate) fn parse_type_color_addition_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<TypeColorAdditionClause>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.len() < 7 || words.first() != Some(&"is") {
        return Ok(None);
    }

    let Some(addition_idx) =
        anthem_find_prefix_shape_start(&words, &IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    if addition_idx <= 1 {
        return Ok(None);
    }

    let scope_words = &words[addition_idx + 5..];
    let mut allow_colors = false;
    let mut allow_types = false;
    let mut segment_start = 0usize;
    for idx in 0..=scope_words.len() {
        let is_boundary =
            idx == scope_words.len() || ANTHEM_AND_WORD_PATTERN.matches_word(scope_words[idx]);
        if !is_boundary {
            continue;
        }
        if segment_start == idx {
            segment_start = idx + 1;
            continue;
        }
        let segment = &scope_words[segment_start..idx];
        segment_start = idx + 1;
        if segment.len() == 1 && ANTHEM_COLOR_OR_COLORS_WORD_PATTERN.matches_words(segment) {
            allow_colors = true;
            continue;
        }
        if ANTHEM_TYPE_OR_TYPES_WORD_PATTERN.matches_last_word(segment)
            && segment[..segment.len() - 1]
                .iter()
                .all(|word| is_type_scope_qualifier_word(word))
        {
            allow_types = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported in-addition scope in type/color clause (clause: '{}')",
            words.join(" ")
        )));
    }
    if !allow_colors && !allow_types {
        return Ok(None);
    }

    let descriptor_words = non_article_word_refs_except(&words[1..addition_idx], &["and"]);
    if descriptor_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing type/color descriptors in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut added_colors = ColorSet::new();
    let mut set_colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            if allow_colors {
                added_colors = added_colors.union(color);
            } else if allow_types {
                // "is black Zombie in addition to its other creature types"
                // sets color while only preserving existing types.
                set_colors = set_colors.union(color);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "color descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                    descriptor,
                    words.join(" ")
                )));
            }
            continue;
        }

        if let Some(card_type) = parse_card_type(descriptor) {
            if allow_types {
                if !card_types.iter().any(|existing| *existing == card_type) {
                    card_types.push(card_type);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "card type descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            if allow_types {
                if !subtypes.iter().any(|existing| *existing == subtype) {
                    subtypes.push(subtype);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "subtype descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in type/color addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if added_colors.is_empty()
        && set_colors.is_empty()
        && card_types.is_empty()
        && subtypes.is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing type/color additions in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(TypeColorAdditionClause {
        added_colors,
        set_colors,
        card_types,
        subtypes,
    }))
}

pub(crate) fn is_type_scope_qualifier_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || matches!(
            word,
            "card" | "creature" | "permanent" | "basic" | "legendary" | "snow" | "nonbasic"
        )
}

pub(crate) fn parse_soulbond_shared_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let Some(paired_word_idx) =
        anthem_find_prefix_shape_start(&clause_words, &PAIRED_WITH_ANOTHER_CREATURE_PREFIX_PATTERN)
            .filter(|idx| *idx >= 3)
    else {
        return Ok(None);
    };

    let subject_words = &clause_words[3..paired_word_idx];
    if subject_words.is_empty() {
        return Ok(None);
    }

    let source_like_subject = is_source_reference_words(subject_words)
        || SOULBOND_SOURCE_SUBJECT_PATTERN.matches_words(subject_words)
        || !subject_words.iter().any(|word| {
            matches!(
                *word,
                "enchanted" | "equipped" | "target" | "another" | "each" | "those"
            )
        });
    if !source_like_subject {
        return Ok(None);
    }

    let prefix_word_len = paired_word_idx + 5;
    let prefix_len = token_index_for_word_index(tokens, prefix_word_len).unwrap_or(tokens.len());

    let rest = trim_commas(&tokens[prefix_len..]);
    if rest.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing soulbond shared effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let rest_words = crate::runtime_backend::token_word_refs(&rest);
    let pt_modifier_idx = if SOULBOND_BOTH_CREATURES_GET_PREFIX_PATTERN.matches_words(&rest_words) {
        Some(3usize)
    } else if SOULBOND_EACH_OF_THOSE_CREATURES_GETS_PREFIX_PATTERN.matches_words(&rest_words) {
        Some(5usize)
    } else {
        None
    };
    if let Some(modifier_idx) = pt_modifier_idx {
        let modifier = *rest_words.get(modifier_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        let (power, toughness) = parse_pt_modifier(modifier).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(Some(vec![
            StaticAbility::soulbond_shared_power_toughness(power, toughness).into(),
        ]));
    }

    let ability_start = if SOULBOND_BOTH_CREATURES_HAVE_PREFIX_PATTERN.matches_words(&rest_words) {
        Some(3usize)
    } else if SOULBOND_EACH_OF_THOSE_CREATURES_HAS_PREFIX_PATTERN.matches_words(&rest_words) {
        Some(5usize)
    } else {
        None
    };
    if let Some(ability_start) = ability_start {
        let mut ability_tokens = trim_commas(&rest[ability_start..]);
        ability_tokens = trim_edge_punctuation(&ability_tokens);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing shared ability in soulbond clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let ability_words = crate::runtime_backend::token_word_refs(&ability_tokens);
        if ability_words
            == [
                "whenever",
                "this",
                "creature",
                "attacks",
                "each",
                "opponent",
                "mills",
                "cards",
                "equal",
                "to",
                "its",
                "toughness",
            ]
        {
            let display = display_text_for_tokens(&ability_tokens, false);
            let ability = parsed_triggered_ability(
                TriggerSpec::ThisAttacks,
                vec![EffectAst::subject_verb(
                    crate::cards::builders::SubjectVerbRoleAst::AffectedPlayer,
                    crate::cards::builders::PlayerAst::Opponent,
                    crate::cards::builders::SubjectVerbActionAst::Mill {
                        count: Value::ToughnessOf(Box::new(ChooseSpec::Source)),
                    },
                )],
                vec![Zone::Battlefield],
                Some(display.clone()),
                None,
                None,
                ReferenceImports::default(),
            );
            return Ok(Some(vec![StaticAbilityAst::SoulbondSharedObjectAbility {
                ability,
            }]));
        }

        if let Some(actions) = parse_ability_line(&ability_tokens) {
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let abilities: Vec<StaticAbility> = actions
                .into_iter()
                .filter_map(|action| keyword_action_to_static_ability(action))
                .collect();
            if abilities.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported shared ability in soulbond clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let shared = abilities
                .into_iter()
                .map(StaticAbility::soulbond_shared_ability)
                .map(StaticAbilityAst::from)
                .collect();
            return Ok(Some(shared));
        }

        if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, .. }) =
            parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &clause_words)?
        {
            return Ok(Some(vec![StaticAbilityAst::SoulbondSharedObjectAbility {
                ability,
            }]));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported shared ability in soulbond clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported soulbond shared clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_anthem_and_type_color_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if contains_until_end_of_turn(&words) {
        return Ok(None);
    }

    let get_idx = anthem_token_offset(tokens, |token| {
        ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
    });
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };

    let and_idx = anthem_token_offset_from(tokens, get_idx + 1, |token| {
        AND_WORD_PATTERN.matches_token(token)
    });
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };

    let addition_tokens = &tokens[and_idx + 1..];
    let Some(additions) = parse_type_color_addition_clause(addition_tokens)? else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let AnthemSubjectAst::Filter(filter) = &clause.subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported source-only type/color addition clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let mut result = vec![build_anthem_static_ability(&clause)];
    if !additions.set_colors.is_empty() {
        result.push(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ));
    }
    if !additions.added_colors.is_empty() {
        result.push(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ));
    }
    if !additions.card_types.is_empty() {
        result.push(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ));
    }
    if !additions.subtypes.is_empty() {
        result.push(StaticAbility::add_subtypes(
            filter.clone(),
            additions.subtypes,
        ));
    }
    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_keyword_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);

    let get_idx = anthem_token_offset(tokens, |token| {
        ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
    });
    let have_idx = ANTHEM_HAVE_OR_HAS_WORD_PATTERN.find_word(&clause_words);

    let (Some(get_idx), Some(have_idx)) = (get_idx, have_idx) else {
        return Ok(None);
    };

    if have_idx < get_idx {
        let have_token_idx = anthem_token_offset_between(tokens, 0, get_idx, |token| {
            ANTHEM_HAVE_OR_HAS_WORD_PATTERN.matches_token(token)
        });
        let Some(have_token_idx) = have_token_idx else {
            return Ok(None);
        };

        let subject_tokens = trim_edge_punctuation(&tokens[..have_token_idx]);
        if subject_tokens.is_empty() {
            return Ok(None);
        }
        let subject = parse_anthem_subject(&subject_tokens)?;

        let keyword_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..get_idx]);
        if keyword_tokens.is_empty() {
            return Ok(None);
        }
        let keyword_tokens = if keyword_tokens
            .first()
            .is_some_and(|token| AND_WORD_PATTERN.matches_token(token))
        {
            trim_edge_punctuation(&keyword_tokens[1..])
        } else {
            keyword_tokens
        };
        let keyword_tokens = if keyword_tokens
            .last()
            .is_some_and(|token| AND_WORD_PATTERN.matches_token(token))
        {
            trim_edge_punctuation(&keyword_tokens[..keyword_tokens.len().saturating_sub(1)])
        } else {
            keyword_tokens
        };
        if keyword_tokens.is_empty() {
            return Ok(None);
        }

        let Some(actions) = parse_ability_line(&keyword_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;

        let mut anthem_tokens = subject_tokens.clone();
        anthem_tokens.extend_from_slice(&tokens[get_idx..]);
        let Some(anthem) = parse_anthem_line(&anthem_tokens)? else {
            return Ok(None);
        };
        let mut result = vec![StaticAbilityAst::from(anthem)];
        let grant_clause = ParsedAnthemClause {
            subject,
            power: AnthemValue::Fixed(0),
            toughness: AnthemValue::Fixed(0),
            condition: None,
        };
        for action in actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
        {
            result.push(grant_keyword_action_for_anthem_subject(
                &grant_clause,
                action,
            ));
        }
        return Ok(Some(result));
    }

    if have_idx == get_idx {
        return Ok(None);
    }

    let have_token_idx = anthem_token_offset_from(tokens, get_idx + 1, |token| {
        ANTHEM_HAVE_OR_HAS_WORD_PATTERN.matches_token(token)
    });
    let Some(have_token_idx) = have_token_idx else {
        return Ok(None);
    };

    let pre_grant_words = crate::runtime_backend::token_word_refs(&tokens[..have_token_idx]);
    // "until end of turn" in the pump clause indicates a one-shot effect.
    // Ignore timing text that appears only inside a quoted granted ability.
    if contains_until_end_of_turn(&pre_grant_words) {
        return Ok(None);
    }

    if let Some(is_idx) =
        anthem_token_offset_between(tokens, get_idx + 2, have_token_idx, |token| {
            IS_WORD_PATTERN.matches_token(token)
        })
        && let Some(color_word) = tokens.get(is_idx + 1).and_then(OwnedLexToken::as_word)
        && let Some(color) = parse_color(color_word)
    {
        let clause = parse_anthem_clause(tokens, get_idx, is_idx)?;
        let filter = anthem_subject_filter(&clause.subject);
        let mut result = vec![build_anthem_static_ability(&clause).into()];
        let color_static = StaticAbility::set_colors(filter, color);
        let color_ast: StaticAbilityAst = color_static.into();
        result.push(match &clause.condition {
            Some(condition) => add_static_ability_ast_condition(color_ast, condition.clone())?,
            None => color_ast,
        });

        let ability_tokens_storage = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
        let ability_tokens = trim_outer_quotes(&ability_tokens_storage);
        if contains_token_kind(&ability_tokens, TokenKind::Colon) {
            let Some(parsed) = parse_activated_line(ability_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let display = display_text_for_tokens(ability_tokens, false);
            result.push(grant_object_ability_for_anthem_subject(
                &clause, parsed, display,
            ));
            return Ok(Some(result));
        }
    }

    if let Some(split_idx) = anthem_token_offset_between(
        tokens,
        get_idx + 2,
        have_token_idx.saturating_sub(1),
        |token| AND_WORD_PATTERN.matches_token(token),
    ) {
        let first_clause = parse_anthem_clause(tokens, get_idx, split_idx)?;
        let mut result = vec![build_anthem_static_ability(&first_clause).into()];

        let tail_start = split_idx + 1;
        let grant_clause = if let Some(second_get_idx) =
            anthem_token_offset_between(tokens, tail_start, have_token_idx, |token| {
                ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
            }) {
            let second_tail_end = if have_token_idx > second_get_idx + 2
                && tokens
                    .get(have_token_idx - 1)
                    .is_some_and(|token| AND_WORD_PATTERN.matches_token(token))
            {
                have_token_idx - 1
            } else {
                have_token_idx
            };
            let second_tokens = &tokens[tail_start..];
            let second_clause = parse_anthem_clause(
                second_tokens,
                second_get_idx - tail_start,
                second_tail_end - tail_start,
            )?;
            result.push(build_anthem_static_ability(&second_clause).into());
            second_clause
        } else {
            let subject_tokens = trim_edge_punctuation(&tokens[tail_start..have_token_idx]);
            if subject_tokens.is_empty() {
                return Ok(None);
            }
            ParsedAnthemClause {
                subject: parse_anthem_subject(&subject_tokens)?,
                power: AnthemValue::Fixed(0),
                toughness: AnthemValue::Fixed(0),
                condition: None,
            }
        };

        let ability_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
        let Some(actions) = parse_ability_line(&ability_tokens) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        for action in actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
        {
            result.push(grant_keyword_action_for_anthem_subject(
                &grant_clause,
                action,
            ));
        }
        return Ok(Some(result));
    }

    let mut ability_tokens = trim_edge_punctuation(&tokens[have_token_idx + 1..]);
    let mut trailing_condition: Option<crate::ConditionExpr> = None;
    if let Some(as_long_idx) = anthem_find_prefix_shape_start(
        &crate::runtime_backend::token_word_refs(&ability_tokens),
        &ANTHEM_AS_LONG_AS_PREFIX_PATTERN,
    ) {
        let as_token_idx =
            token_index_for_word_index(&ability_tokens, as_long_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map trailing 'as long as' keyword condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let condition_start_idx = token_index_for_word_index(&ability_tokens, as_long_idx + 3)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let ability_head = trim_edge_punctuation(&ability_tokens[..as_token_idx]);
        if ability_head.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing granted keyword list before trailing condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let condition_tokens = trim_edge_punctuation(&ability_tokens[condition_start_idx..]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        trailing_condition = Some(parse_static_condition_clause(&condition_tokens)?);
        ability_tokens = ability_head;
    }
    let mut trailing_type_color_addition: Option<TypeColorAdditionClause> = None;
    if let Some(and_is_idx) = anthem_index_where(ability_tokens.len().saturating_sub(1), |idx| {
        AND_WORD_PATTERN.matches_token(&ability_tokens[idx])
            && IS_WORD_PATTERN.matches_token(&ability_tokens[idx + 1])
    }) {
        let addition_tokens = trim_edge_punctuation(&ability_tokens[and_is_idx + 1..]);
        if let Some(additions) = parse_type_color_addition_clause(&addition_tokens)? {
            let keyword_head = trim_edge_punctuation(&ability_tokens[..and_is_idx]);
            if keyword_head.is_empty() {
                return Ok(None);
            }
            trailing_type_color_addition = Some(additions);
            ability_tokens = keyword_head;
        }
    }

    let mut keyword_actions: Vec<KeywordAction> = Vec::new();
    let mut granted_activated_ability: Option<ParsedAbility> = None;
    let mut granted_activated_display: Option<String> = None;

    let and_has_idx = anthem_index_where(ability_tokens.len().saturating_sub(1), |idx| {
        AND_WORD_PATTERN.matches_token(&ability_tokens[idx])
            && ANTHEM_HAVE_OR_HAS_WORD_PATTERN.matches_token(&ability_tokens[idx + 1])
    });
    if let Some(and_has_idx) = and_has_idx {
        let keyword_tokens = trim_edge_punctuation(&ability_tokens[..and_has_idx]);
        if !keyword_tokens.is_empty() {
            if let Some(actions) = parse_ability_line(&keyword_tokens) {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                keyword_actions.extend(
                    actions
                        .into_iter()
                        .filter(|action| action.lowers_to_static_ability()),
                );
            } else {
                return Ok(None);
            }
        }

        let ability_tail_tokens = trim_edge_punctuation(&ability_tokens[and_has_idx + 2..]);
        if !ability_tail_tokens.is_empty() {
            let mut handled_split_keyword_activation = false;
            if contains_token_kind(&ability_tail_tokens, TokenKind::Colon) {
                let colon_idx = anthem_token_offset(&ability_tail_tokens, |token| token.is_colon())
                    .expect("validated colon");
                if let Some(split_and_idx) = anthem_last_index_where(colon_idx, |idx| {
                    AND_WORD_PATTERN.matches_token(&ability_tail_tokens[idx])
                }) {
                    let trailing_keyword_tokens =
                        trim_edge_punctuation(&ability_tail_tokens[..split_and_idx]);
                    let activated_tail =
                        trim_edge_punctuation(&ability_tail_tokens[split_and_idx + 1..]);
                    if !trailing_keyword_tokens.is_empty() {
                        let Some(actions) = parse_ability_line(&trailing_keyword_tokens) else {
                            return Ok(None);
                        };
                        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                        keyword_actions.extend(
                            actions
                                .into_iter()
                                .filter(|action| action.lowers_to_static_ability()),
                        );
                    }
                    let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
                    let Some(parsed) = parse_activated_line(&activated_tail)? else {
                        if has_colon {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported granted activated ability in anthem clause (clause: '{}')",
                                clause_words.join(" ")
                            )));
                        }
                        return Ok(None);
                    };
                    let display = display_text_for_tokens(&activated_tail, false);
                    granted_activated_display = Some(display);
                    granted_activated_ability = Some(parsed);
                    handled_split_keyword_activation = true;
                }
            }
            if !handled_split_keyword_activation {
                let has_colon = contains_token_kind(&ability_tail_tokens, TokenKind::Colon);
                let Some(parsed) = parse_activated_line(&ability_tail_tokens)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&ability_tail_tokens, false);
                granted_activated_display = Some(display);
                granted_activated_ability = Some(parsed);
            }
        }
    } else if contains_token_kind(&ability_tokens, TokenKind::Colon) {
        let Some(colon_idx) = anthem_token_offset(&ability_tokens, |token| token.is_colon()) else {
            return Ok(None);
        };
        let Some(and_idx) = anthem_last_index_where(colon_idx, |idx| {
            AND_WORD_PATTERN.matches_token(&ability_tokens[idx])
        }) else {
            let activated_tail_storage = trim_edge_punctuation(&ability_tokens);
            let activated_tail = trim_outer_quotes(&activated_tail_storage);
            let Some(parsed) = parse_activated_line(activated_tail)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            };
            let display = display_text_for_tokens(activated_tail, false);
            granted_activated_display = Some(display);
            granted_activated_ability = Some(parsed);
            let clause_tail_end = if have_token_idx > get_idx + 2
                && tokens
                    .get(have_token_idx - 1)
                    .is_some_and(|token| AND_WORD_PATTERN.matches_token(token))
            {
                have_token_idx - 1
            } else {
                have_token_idx
            };
            let mut clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
            if let Some(condition) = trailing_condition {
                if clause.condition.is_some() {
                    return Err(CardTextError::ParseError(format!(
                        "multiple anthem conditions are not supported (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                clause.condition = Some(condition);
            }
            let mut result = vec![build_anthem_static_ability(&clause).into()];
            if let Some(ability) = granted_activated_ability {
                result.push(grant_object_ability_for_anthem_subject(
                    &clause,
                    ability,
                    granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
                ));
            }
            return Ok(Some(result));
        };
        let keyword_head = trim_edge_punctuation(&ability_tokens[..and_idx]);
        let activated_tail = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
        if keyword_head.is_empty() || activated_tail.is_empty() {
            return Ok(None);
        }
        let Some(actions) = parse_ability_line(&keyword_head) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect();
        let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
        let Some(parsed) = parse_activated_line(&activated_tail)? else {
            if has_colon {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated ability in anthem clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(None);
        };
        let display = display_text_for_tokens(&activated_tail, false);
        granted_activated_display = Some(display);
        granted_activated_ability = Some(parsed);
    } else if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &clause_words)?
    {
        granted_activated_display = Some(display);
        granted_activated_ability = Some(ability);
    } else if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        keyword_actions = actions
            .into_iter()
            .filter(|action| action.lowers_to_static_ability())
            .collect();
    } else {
        return Ok(None);
    }

    if keyword_actions.is_empty() && granted_activated_ability.is_none() {
        return Ok(None);
    }

    let clause_tail_end = if have_token_idx > get_idx + 2
        && tokens
            .get(have_token_idx - 1)
            .is_some_and(|token| AND_WORD_PATTERN.matches_token(token))
    {
        have_token_idx - 1
    } else {
        have_token_idx
    };
    let mut clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
    if let Some(condition) = trailing_condition {
        if clause.condition.is_some() {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        clause.condition = Some(condition);
    }
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    for action in keyword_actions {
        result.push(grant_keyword_action_for_anthem_subject(&clause, action));
    }
    if let Some(additions) = trailing_type_color_addition {
        push_type_color_additions_for_anthem_subject(&mut result, &clause, additions)?;
    }

    if let Some(ability) = granted_activated_ability {
        result.push(grant_object_ability_for_anthem_subject(
            &clause,
            ability,
            granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
        ));
    }

    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_goaded_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(get_idx) = anthem_token_offset(tokens, |token| {
        token.is_word("get") || token.is_word("gets")
    }) else {
        return Ok(None);
    };

    let Some(and_idx) = anthem_token_offset_from(tokens, get_idx + 1, |token| token.is_word("and"))
    else {
        return Ok(None);
    };

    let tail_tokens = trim_edge_punctuation(&tokens[and_idx + 1..]);
    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
    if !matches!(tail_words.as_slice(), ["is", "goaded"] | ["are", "goaded"]) {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let display_subject = attached_goaded_display_subject(&clause.subject).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported goaded anthem subject (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    Ok(Some(vec![
        build_anthem_static_ability(&clause).into(),
        crate::static_abilities::StaticAbility::attached_goaded_by_source_controller(format!(
            "{} is goaded",
            capitalize_display_subject(&display_subject)
        ))
        .into(),
    ]))
}

fn attached_goaded_display_subject(subject: &AnthemSubjectAst) -> Option<String> {
    let AnthemSubjectAst::Filter(filter) = subject else {
        return None;
    };
    let attachment = filter.tagged_constraints.iter().find_map(|constraint| {
        if !matches!(
            constraint.relation,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject
        ) {
            return None;
        }
        match constraint.tag.as_str() {
            "enchanted" => Some("enchanted"),
            "equipped" => Some("equipped"),
            _ => None,
        }
    })?;

    let noun = if filter.card_types.contains(&CardType::Creature) {
        "creature"
    } else {
        "permanent"
    };
    Some(format!("{attachment} {noun}"))
}

fn capitalize_display_subject(subject: &str) -> String {
    let mut chars = subject.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

fn push_type_color_additions_for_anthem_subject(
    result: &mut Vec<StaticAbilityAst>,
    clause: &ParsedAnthemClause,
    additions: TypeColorAdditionClause,
) -> Result<(), CardTextError> {
    let filter = anthem_subject_filter(&clause.subject);
    let condition = clause.condition.clone();
    let mut push_static = |ability: StaticAbility| -> Result<(), CardTextError> {
        let ast: StaticAbilityAst = ability.into();
        result.push(match &condition {
            Some(condition) => add_static_ability_ast_condition(ast, condition.clone())?,
            None => ast,
        });
        Ok(())
    };

    if !additions.set_colors.is_empty() {
        push_static(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ))?;
    }
    if !additions.added_colors.is_empty() {
        push_static(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ))?;
    }
    if !additions.card_types.is_empty() {
        push_static(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ))?;
    }
    if !additions.subtypes.is_empty() {
        push_static(StaticAbility::add_subtypes(filter, additions.subtypes))?;
    }

    Ok(())
}

fn merge_static_ability_ast_conditions(
    existing: Option<crate::ConditionExpr>,
    additional: crate::ConditionExpr,
) -> crate::ConditionExpr {
    match existing {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(additional)),
        None => additional,
    }
}

fn add_static_ability_ast_condition(
    ability: StaticAbilityAst,
    condition: crate::ConditionExpr,
) -> Result<StaticAbilityAst, CardTextError> {
    Ok(match ability {
        StaticAbilityAst::Static(_) | StaticAbilityAst::KeywordAction(_) => {
            StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(ability),
                condition,
            }
        }
        StaticAbilityAst::ConditionalStaticAbility {
            ability,
            condition: existing,
        } => StaticAbilityAst::ConditionalStaticAbility {
            ability,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        },
        StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: existing,
        } => StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition: crate::ConditionExpr::And(Box::new(existing), Box::new(condition)),
        },
        StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition: existing,
        } => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition: existing,
        } => StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedStaticAbilityGrant {
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedKeywordActionGrant {
            action,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedChosenLandwalkGrant {
            snow,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::GrantObjectAbility {
            filter,
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition: existing,
        } => StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display,
            condition: Some(merge_static_ability_ast_conditions(existing, condition)),
        },
        StaticAbilityAst::RemoveStaticAbility { .. }
        | StaticAbilityAst::RemoveKeywordAction { .. }
        | StaticAbilityAst::EquipmentKeywordActionsGrant { .. }
        | StaticAbilityAst::SoulbondSharedObjectAbility { .. }
        | StaticAbilityAst::AttachmentRestriction { .. } => {
            return Err(CardTextError::ParseError(
                "cannot apply leading static condition to unsupported static ability shape"
                    .to_string(),
            ));
        }
    })
}

pub(crate) fn parse_protection_from_colored_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !matches!(
        clause_words.as_slice(),
        [
            "protection",
            "from",
            "spells",
            "that",
            "are",
            "one",
            "or",
            "more",
            "colors"
        ]
    ) {
        return Ok(None);
    }

    let all_colors = crate::color::ColorSet::WHITE
        .union(crate::color::ColorSet::BLUE)
        .union(crate::color::ColorSet::BLACK)
        .union(crate::color::ColorSet::RED)
        .union(crate::color::ColorSet::GREEN);
    let mut filter = ObjectFilter::spell();
    filter.colors = Some(all_colors);
    Ok(Some(StaticAbility::protection(
        crate::ability::ProtectionFrom::Permanents(filter),
    )))
}

fn grant_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: StaticAbility,
) -> StaticAbilityAst {
    match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition: condition.clone(),
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter: filter.clone(),
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition: clause.condition.clone(),
        },
    }
}

fn parse_every_subtype_family_tail(words: &[&str]) -> Option<crate::types::SubtypeFamily> {
    EVERY_SUBTYPE_FAMILY_TAILS
        .iter()
        .find_map(|(phrase, family)| (*phrase == words).then_some(*family))
}

fn every_subtype_family_for_subject(
    subject: &AnthemSubjectAst,
    family: crate::types::SubtypeFamily,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    let base = match subject {
        AnthemSubjectAst::Source => {
            StaticAbility::add_all_subtypes_of_family(ObjectFilter::source(), family)
        }
        AnthemSubjectAst::Filter(filter) => {
            StaticAbility::add_all_subtypes_of_family(filter.clone(), family)
        }
    };

    let ability = condition
        .as_ref()
        .map(|cond| base.clone().with_condition(cond.clone()))
        .unwrap_or({
            #[cfg(not(feature = "serialization"))]
            {
                base
            }
            #[cfg(feature = "serialization")]
            {
                Some(base)
            }
        });
    #[cfg(not(feature = "serialization"))]
    {
        StaticAbilityAst::Static(ability)
    }
    #[cfg(feature = "serialization")]
    {
        StaticAbilityAst::Static(ability.expect("runtime static ability should exist"))
    }
}

fn grant_keyword_action_for_anthem_subject(
    clause: &ParsedAnthemClause,
    action: KeywordAction,
) -> StaticAbilityAst {
    match &clause.subject {
        AnthemSubjectAst::Source => match &clause.condition {
            Some(condition) => StaticAbilityAst::ConditionalKeywordAction {
                action,
                condition: condition.clone(),
            },
            None => StaticAbilityAst::KeywordAction(action),
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantKeywordAction {
            filter: filter.clone(),
            action,
            condition: clause.condition.clone(),
        },
    }
}

fn granted_object_ability_for_keyword_action(
    action: &KeywordAction,
) -> Option<(ParsedAbility, String)> {
    match action {
        KeywordAction::Afflict(amount) => Some((
            parsed_ability_from_ability(afflict_triggered_ability(*amount)),
            action.display_text(),
        )),
        _ => None,
    }
}

fn split_keyword_if_color_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if !token.is_comma() {
            continue;
        }
        let mut segment = trim_edge_punctuation(&tokens[start..idx]);
        while token_slice_first_is(&segment, "and") {
            segment = trim_edge_punctuation(&segment[1..]);
        }
        if !segment.is_empty() {
            segments.push(segment);
        }
        start = idx + 1;
    }
    let mut segment = trim_edge_punctuation(&tokens[start..]);
    while token_slice_first_is(&segment, "and") {
        segment = trim_edge_punctuation(&segment[1..]);
    }
    if !segment.is_empty() {
        segments.push(segment);
    }
    segments
}

fn parse_if_its_color_tail(tokens: &[OwnedLexToken]) -> Option<ColorSet> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    IF_IT_IS_COLOR_PREFIXES.iter().find_map(|prefix| {
        if words.len() == prefix.len() + 1 && words.starts_with(prefix) {
            parse_color(words[prefix.len()])
        } else {
            None
        }
    })
}

fn parse_keyword_if_color_segment(
    segment: &[OwnedLexToken],
    clause_text: &str,
) -> Result<Option<(Vec<KeywordAction>, ColorSet)>, CardTextError> {
    let Some(if_idx) =
        anthem_token_offset(segment, |token| ANTHEM_IF_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    let keyword_tokens = trim_edge_punctuation(&segment[..if_idx]);
    if keyword_tokens.is_empty() {
        return Ok(None);
    }
    let Some(color) = parse_if_its_color_tail(&segment[if_idx + 1..]) else {
        return Ok(None);
    };
    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, clause_text)?;
    let actions = actions
        .into_iter()
        .filter(|action| action.lowers_to_static_ability())
        .collect::<Vec<_>>();
    if actions.is_empty() {
        return Ok(None);
    }
    Ok(Some((actions, color)))
}

fn color_filtered_grant_filter(mut filter: ObjectFilter, color: ColorSet) -> ObjectFilter {
    let existing = filter.colors.unwrap_or(ColorSet::new());
    filter.colors = Some(existing.union(color));
    filter
}

fn source_color_condition(color: ColorSet) -> crate::ConditionExpr {
    let mut filter = ObjectFilter::source();
    filter.colors = Some(color);
    crate::ConditionExpr::SourceMatches(filter)
}

fn append_condition(
    condition: Option<crate::ConditionExpr>,
    next: crate::ConditionExpr,
) -> crate::ConditionExpr {
    match condition {
        Some(existing) => crate::ConditionExpr::And(Box::new(existing), Box::new(next)),
        None => next,
    }
}

fn parse_color_filtered_keyword_grants(
    subject_tokens: &[OwnedLexToken],
    keyword_tokens: &[OwnedLexToken],
    condition: Option<crate::ConditionExpr>,
    clause_text: &str,
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    if !crate::runtime_backend::token_word_refs(keyword_tokens)
        .iter()
        .any(|word| ANTHEM_IF_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let mut parsed_segments = Vec::new();
    for segment in split_keyword_if_color_segments(keyword_tokens) {
        let Some(parsed) = parse_keyword_if_color_segment(&segment, clause_text)? else {
            return Ok(None);
        };
        parsed_segments.push(parsed);
    }
    if parsed_segments.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(subject_tokens)?;
    let mut compiled = Vec::new();
    for (actions, color) in parsed_segments {
        for action in actions {
            match &subject {
                AnthemSubjectAst::Source => {
                    compiled.push(StaticAbilityAst::ConditionalKeywordAction {
                        action,
                        condition: append_condition(
                            condition.clone(),
                            source_color_condition(color),
                        ),
                    })
                }
                AnthemSubjectAst::Filter(filter) => {
                    compiled.push(StaticAbilityAst::GrantKeywordAction {
                        filter: color_filtered_grant_filter(filter.clone(), color),
                        action,
                        condition: condition.clone(),
                    });
                }
            }
        }
    }

    Ok(Some(compiled))
}

fn anthem_subject_filter(subject: &AnthemSubjectAst) -> ObjectFilter {
    match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter.clone(),
    }
}

fn grant_object_ability_for_anthem_subject(
    clause: &ParsedAnthemClause,
    ability: ParsedAbility,
    display: String,
) -> StaticAbilityAst {
    if let Some(filter) = attached_object_anthem_subject_filter(&clause.subject) {
        let subject = filter.description();
        return StaticAbilityAst::AttachedObjectAbilityGrant {
            ability,
            display: format!("{subject} has {display}"),
            condition: clause.condition.clone(),
        };
    }

    StaticAbilityAst::GrantObjectAbility {
        filter: anthem_subject_filter(&clause.subject),
        ability,
        display,
        condition: clause.condition.clone(),
    }
}

fn attached_object_anthem_subject_filter(subject: &AnthemSubjectAst) -> Option<&ObjectFilter> {
    let AnthemSubjectAst::Filter(filter) = subject else {
        return None;
    };
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| {
            matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
            ) && matches!(constraint.tag.as_str(), "enchanted" | "equipped")
        })
        .then_some(filter)
}

fn parsed_ability_from_ability(ability: Ability) -> ParsedAbility {
    ParsedAbility {
        ability: ability.into(),
        text: None,
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }
}

pub(crate) fn parse_equipment_you_control_have_equip_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = if let Some((label, body_tokens)) = split_em_dash_label_prefix(tokens) {
        if METALCRAFT_LABEL_PATTERN.matches_words(&[label.as_str()]) {
            body_tokens
        } else {
            tokens
        }
    } else {
        tokens
    };
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !EQUIPMENT_YOU_CONTROL_HAVE_EQUIP_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    let Some(as_word_idx) =
        anthem_find_prefix_shape_start(&words, &ANTHEM_AS_LONG_AS_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    let Some(as_idx) =
        anthem_token_offset(tokens, |token| ANTHEM_AS_WORD_PATTERN.matches_token(token))
    else {
        return Ok(None);
    };
    if as_word_idx < 5 || as_idx <= 5 {
        return Ok(None);
    }
    let cost_tokens = trim_edge_punctuation(&tokens[5..as_idx]);
    let condition = parse_static_condition_clause(&tokens[as_idx..])?;
    let total_cost = parse_activation_cost(&cost_tokens)?;
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let ability = ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::attach_to(target.clone()),
                ]),
                choices: vec![target],
                timing: crate::ability::ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some("Equip {0}".to_string()),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    };
    Ok(Some(vec![StaticAbilityAst::GrantObjectAbility {
        filter: ObjectFilter::default()
            .with_subtype(Subtype::Equipment)
            .you_control(),
        ability,
        display: "Equipment you control have equip {0}".to_string(),
        condition: Some(condition),
    }]))
}

fn parsed_exploit_ability() -> ParsedAbility {
    let effect_id = 0;
    let ability = Ability::triggered(
        Trigger::this_enters_battlefield(),
        vec![
            Effect::with_id(
                effect_id,
                Effect::may(vec![Effect::sacrifice(ObjectFilter::creature(), 1)]),
            ),
            Effect::if_then(
                effect_id,
                crate::effect::EffectPredicate::Happened,
                vec![Effect::emit_keyword_action_with_affected_object_memory_tag(
                    crate::events::KeywordActionKind::Exploit,
                    1,
                    crate::effect::EffectId(effect_id),
                    crate::tag::EXPLOITED_TAG,
                )],
            ),
        ],
    );
    ParsedAbility {
        ability: ability.into(),
        text: Some("Exploit".to_string()),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: Some(TriggerSpec::ThisEntersBattlefield),
    }
}

fn grant_exploit_for_anthem_subject(
    subject: &AnthemSubjectAst,
    condition: Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    StaticAbilityAst::GrantObjectAbility {
        filter: anthem_subject_filter(subject),
        ability: parsed_exploit_ability(),
        display: "exploit".to_string(),
        condition,
    }
}

fn parse_triggered_granted_ability(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let trigger_tokens = trim_edge_punctuation(tokens);
    if trigger_tokens.is_empty() {
        return Ok(None);
    }
    if !trigger_tokens
        .first()
        .is_some_and(|token| ANTHEM_WHEN_OR_WHENEVER_WORD_PATTERN.matches_token(token))
        && !is_at_trigger_intro(&trigger_tokens, 0)
    {
        return Ok(None);
    }

    let ability = match crate::runtime_backend::clause_support::parse_triggered_line_lexed(
        &trigger_tokens,
    )? {
        LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn,
        } => {
            let (effects, trigger_condition) =
                triggered_grant_effects_and_condition(&trigger, &effects)?;
            let max_condition = crate::runtime_backend::trigger_frequency_condition(
                Some(&crate::runtime_backend::lexer::token_word_refs(&trigger_tokens).join(" ")),
                max_triggers_per_turn,
            );
            let intervening_if = match (trigger_condition, max_condition) {
                (Some(left), Some(right)) => {
                    Some(crate::ConditionExpr::And(Box::new(left), Box::new(right)))
                }
                (Some(condition), None) | (None, Some(condition)) => Some(condition),
                (None, None) => None,
            };
            parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Battlefield],
                Some(crate::runtime_backend::token_word_refs(&trigger_tokens).join(" ")),
                intervening_if,
                None,
                ReferenceImports::default(),
            )
        }
        _ => return Ok(None),
    };
    if parsed_triggered_ability_is_empty(&ability) {
        return Err(CardTextError::ParseError(format!(
            "unsupported empty triggered granted ability clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(&trigger_tokens).join(" ")
        )));
    }
    Ok(Some(ability))
}

fn split_anthem_trailing_segments_preserving_granted_abilities(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    let mut preserve_commas = false;
    let mut inside_quotes = false;
    let mut idx = 0usize;

    while idx < tokens.len() {
        let token = &tokens[idx];
        if token.is_quote() {
            inside_quotes = !inside_quotes;
        }

        if !inside_quotes
            && ANTHEM_AND_WORD_PATTERN.matches_token(token)
            && !current.is_empty()
            && tokens[idx + 1..]
                .iter()
                .find(|next| !next.is_comma())
                .is_some_and(|next| next.is_quote())
        {
            segments.push(std::mem::take(&mut current));
            preserve_commas = false;
            idx += 1;
            continue;
        }

        if token.is_comma() && !preserve_commas {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            idx += 1;
            continue;
        }

        current.push(token.clone());
        let trimmed = trim_commas(&current);
        let segment_words = crate::runtime_backend::token_word_refs(&trimmed)
            .into_iter()
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        preserve_commas = inside_quotes || contains_token_kind(&trimmed, TokenKind::Colon) || {
            let segment_word_refs = segment_words.iter().map(String::as_str).collect::<Vec<_>>();
            ANTHEM_TRIGGERED_SEGMENT_START_PATTERN.matches_words(&segment_word_refs)
                || ANTHEM_AND_TRIGGERED_SEGMENT_START_PATTERN.matches_words(&segment_word_refs)
        };
        idx += 1;
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

fn parsed_triggered_ability_is_empty(ability: &ParsedAbility) -> bool {
    matches!(
        ability.kind(),
        AbilityKind::Triggered(triggered)
            if triggered.effects.is_empty()
                && ability
                    .effects_ast
                    .as_ref()
                    .is_none_or(|effects| effects.is_empty())
    )
}

fn parse_granted_keyword_fragment(segment: &[OwnedLexToken]) -> Option<Vec<KeywordAction>> {
    parse_ability_line(segment).or_else(|| {
        let words = crate::runtime_backend::token_word_refs(segment);
        (matches!(
            words.as_slice(),
            ["can't", "be", "blocked"] | ["cant", "be", "blocked"] | ["cannot", "be", "blocked"]
        ))
        .then(|| vec![KeywordAction::Unblockable])
    })
}

fn parse_granted_object_ability_segment(
    raw_segment: &[OwnedLexToken],
    clause_words: &[&str],
    attached_subject: bool,
) -> Result<Option<(ParsedAbility, String)>, CardTextError> {
    let sanitized_tokens = raw_segment
        .iter()
        .filter(|token| token.kind != TokenKind::Quote)
        .cloned()
        .collect::<Vec<_>>();
    let ability_tokens = trim_edge_punctuation(&sanitized_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    if let Some(actions) = parse_ability_line(&ability_tokens)
        && actions.len() == 1
        && let Some(granted) = nonstatic_keyword_action_as_granted_object_ability(
            actions.into_iter().next().expect("single action exists"),
        )
    {
        return Ok(Some(granted));
    }

    if attached_subject && contains_token_kind(&ability_tokens, TokenKind::Colon) {
        let Some(parsed) = parse_attached_granted_activated_line(raw_segment)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some((ability, display)));
    }

    if let Some(parsed) = parse_attached_nonstatic_keyword_ability(&ability_tokens)? {
        return Ok(Some(parsed));
    }

    if let Some(parsed) = parse_cycling_line(&ability_tokens)? {
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    if contains_token_kind(&ability_tokens, TokenKind::Colon) {
        let Some(parsed) = parse_activated_line(&ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        return Ok(Some((
            parsed,
            display_text_for_tokens(&ability_tokens, false),
        )));
    }

    Ok(None)
}

fn nonstatic_keyword_action_as_granted_object_ability(
    action: KeywordAction,
) -> Option<(ParsedAbility, String)> {
    match action {
        KeywordAction::Casualty(power) => {
            let mut creature_filter = ObjectFilter::creature().you_control();
            creature_filter.power = Some(crate::filter::Comparison::GreaterThanOrEqual(
                power as i32,
            ));
            let ability = Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::you_cast_this_spell(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::may(vec![
                            Effect::sacrifice(creature_filter, 1),
                            Effect::with_id(
                                0,
                                Effect::new(crate::effects::CopySpellEffect::single(
                                    ChooseSpec::Source,
                                )),
                            ),
                            Effect::may_choose_new_targets_player(
                                crate::effect::EffectId(0),
                                PlayerFilter::You,
                            ),
                        ]),
                    ]),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: Some(format!("keyword:casualty {power}")),
                }),
                functional_zones: vec![Zone::Stack],
            };
            Some((
                ParsedAbility {
                    ability: ability.into(),
                    text: Some(format!("Casualty {power}")),
                    effects_ast: None,
                    reference_imports: ReferenceImports::default(),
                    trigger_spec: None,
                },
                format!("Casualty {power}"),
            ))
        }
        _ => None,
    }
}

pub(crate) fn parse_heterogeneous_granted_tail(
    tail_tokens: &[OwnedLexToken],
    clause_words: &[&str],
    attached_subject: bool,
) -> Result<Option<ParsedGrantedTailAst>, CardTextError> {
    let mut parsed = ParsedGrantedTailAst::default();

    for raw_segment in split_anthem_trailing_segments_preserving_granted_abilities(tail_tokens) {
        let trimmed = trim_commas(&raw_segment);
        let mut segment = trim_edge_punctuation(&trimmed);
        while token_slice_first_is(&segment, "and") {
            let trimmed = trim_commas(&segment[1..]);
            segment = trim_edge_punctuation(&trimmed);
        }
        if segment.is_empty() {
            continue;
        }

        if let Some((ability, display)) =
            parse_granted_object_ability_segment(&segment, clause_words, attached_subject)?
        {
            parsed.granted_object_abilities.push((ability, display));
            continue;
        }

        if let Some(actions) = parse_granted_keyword_fragment(&segment) {
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            if let [KeywordAction::CumulativeUpkeep { total_cost, .. }] = actions.as_slice() {
                parsed.granted_object_abilities.push((
                    ParsedAbility {
                        ability: cumulative_upkeep_granted_ability(total_cost.clone()).into(),
                        text: Some(display_text_for_tokens(&segment, false)),
                        effects_ast: None,
                        reference_imports: ReferenceImports::default(),
                        trigger_spec: None,
                    },
                    display_text_for_tokens(&segment, false),
                ));
                continue;
            }

            let lowered = actions
                .into_iter()
                .filter(|action| action.lowers_to_static_ability())
                .collect::<Vec<_>>();
            if lowered.is_empty() {
                return Ok(None);
            }
            parsed.granted_keyword_actions.extend(lowered);
            continue;
        }

        let split_actions = split_lexed_slices_on_and(&segment)
            .into_iter()
            .map(trim_edge_punctuation)
            .filter(|part| !part.is_empty())
            .map(|part| parse_granted_keyword_fragment(&part))
            .collect::<Vec<_>>();
        if split_actions.len() > 1
            && split_actions.iter().all(|actions| {
                actions.as_ref().is_some_and(|actions| {
                    actions.iter().all(KeywordAction::lowers_to_static_ability)
                })
            })
        {
            for actions in split_actions.into_iter().flatten() {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                parsed.granted_keyword_actions.extend(actions);
            }
            continue;
        }

        if let Some(marker) = parse_static_text_marker_line(&segment) {
            parsed.granted_static.push(marker.into());
            continue;
        }

        let mut segment_with_period = segment.to_vec();
        segment_with_period.push(OwnedLexToken::period(
            crate::cards::builders::TextSpan::synthetic(),
        ));
        if let Some(marker) = parse_static_text_marker_line(&segment_with_period) {
            parsed.granted_static.push(marker.into());
            continue;
        }

        if let Some(abilities) = parse_static_ability_ast_line_lexed(&segment)? {
            parsed.granted_static.extend(abilities);
            continue;
        }

        return Ok(None);
    }

    if parsed.granted_static.is_empty()
        && parsed.granted_keyword_actions.is_empty()
        && parsed.granted_object_abilities.is_empty()
    {
        return Ok(None);
    }

    Ok(Some(parsed))
}

pub(crate) fn lower_granted_tail_for_anthem_subject(
    subject: &AnthemSubjectAst,
    condition: &Option<crate::ConditionExpr>,
    granted_tail: ParsedGrantedTailAst,
) -> Vec<StaticAbilityAst> {
    let wrapper_clause = ParsedAnthemClause {
        subject: subject.clone(),
        power: AnthemValue::Fixed(0),
        toughness: AnthemValue::Fixed(0),
        condition: condition.clone(),
    };
    let mut granted = Vec::new();
    if !granted_tail.granted_static.is_empty() {
        granted.extend(grant_static_anthem_abilities_for_subject(
            &wrapper_clause,
            granted_tail.granted_static,
        ));
    }
    for action in granted_tail.granted_keyword_actions {
        granted.push(grant_keyword_action_for_anthem_subject(
            &wrapper_clause,
            action,
        ));
    }
    for (ability, display) in granted_tail.granted_object_abilities {
        granted.push(grant_object_ability_for_anthem_subject(
            &wrapper_clause,
            ability,
            display,
        ));
    }
    granted
}

fn wrap_conditioned_animation_static_ability(
    ability: StaticAbility,
    condition: &Option<crate::ConditionExpr>,
) -> StaticAbilityAst {
    if let Some(condition) = condition {
        #[cfg(not(feature = "serialization"))]
        {
            return ability.with_condition(condition.clone()).into();
        }
        #[cfg(feature = "serialization")]
        {
            return ability
                .with_condition(condition.clone())
                .expect("runtime conditioned static ability should exist")
                .into();
        }
    }
    ability.into()
}

pub(crate) fn lower_static_animation_bundle(
    bundle: StaticAnimationBundleAst,
) -> Vec<StaticAbilityAst> {
    let filter = anthem_subject_filter(&bundle.subject);
    let mut lowered = Vec::new();

    if bundle.ensure_creature_type {
        lowered.push(wrap_conditioned_animation_static_ability(
            StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
            &bundle.condition,
        ));
    }
    if let Some((power, toughness)) = bundle.base_power_toughness {
        lowered.push(wrap_conditioned_animation_static_ability(
            StaticAbility::set_base_power_toughness(filter.clone(), power, toughness),
            &bundle.condition,
        ));
    }
    if !bundle.subtypes.is_empty() {
        let ability = match bundle.subtype_mode {
            AnimationSubtypeMode::Add => StaticAbility::add_subtypes(filter, bundle.subtypes),
            AnimationSubtypeMode::ReplaceCreatureTypes => {
                StaticAbility::set_creature_subtypes(filter, bundle.subtypes)
            }
        };
        lowered.push(wrap_conditioned_animation_static_ability(
            ability,
            &bundle.condition,
        ));
    }

    lowered.extend(lower_granted_tail_for_anthem_subject(
        &bundle.subject,
        &bundle.condition,
        bundle.granted_tail,
    ));

    lowered
}

fn grant_static_anthem_abilities_for_subject(
    clause: &ParsedAnthemClause,
    abilities: Vec<StaticAbilityAst>,
) -> Vec<StaticAbilityAst> {
    let mut granted = Vec::new();
    for ability in abilities {
        granted.push(match &clause.subject {
            AnthemSubjectAst::Source => match &clause.condition {
                Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(ability),
                    condition: condition.clone(),
                },
                None => ability,
            },
            AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                filter: filter.clone(),
                ability: Box::new(ability),
                condition: clause.condition.clone(),
            },
        });
    }
    granted
}

fn parse_continuing_anthem_granted_segment(
    clause: &ParsedAnthemClause,
    clause_words: &[&str],
    segment: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let sanitized_tokens = segment
        .iter()
        .filter(|token| token.kind != TokenKind::Quote)
        .cloned()
        .collect::<Vec<_>>();
    let ability_tokens = trim_edge_punctuation(&sanitized_tokens);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
        parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, clause_words)?
    {
        return Ok(Some(vec![grant_object_ability_for_anthem_subject(
            clause, ability, display,
        )]));
    }

    if let Some(actions) = parse_ability_line(&ability_tokens) {
        reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
        let granted = actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
            .collect::<Vec<_>>();
        if granted.is_empty() {
            return Ok(None);
        }
        return Ok(Some(
            granted
                .into_iter()
                .map(|ability| grant_for_anthem_subject(clause, ability))
                .collect(),
        ));
    }

    if let Some(marker) = parse_static_text_marker_line(&ability_tokens) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }

    let ability_words = crate::runtime_backend::token_word_refs(&ability_tokens)
        .into_iter()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    let ability_word_refs = ability_words.iter().map(String::as_str).collect::<Vec<_>>();
    if let [_, _, amount, _] = ability_word_refs.as_slice()
        && ANTHEM_WARD_PAY_LIFE_PATTERN.matches_words(&ability_word_refs)
        && let Some(amount) = parse_named_number(amount)
    {
        return Ok(Some(vec![grant_for_anthem_subject(
            clause,
            StaticAbility::ward(crate::cost::TotalCost::from_cost(crate::costs::Cost::life(
                amount,
            ))),
        )]));
    }

    let mut ability_tokens_with_period = ability_tokens.to_vec();
    ability_tokens_with_period.push(OwnedLexToken::period(
        crate::cards::builders::TextSpan::synthetic(),
    ));
    if let Some(amount) =
        super::grammar::abilities::parse_ward_pay_life_amount_lexed(&ability_tokens_with_period)
    {
        return Ok(Some(vec![grant_for_anthem_subject(
            clause,
            StaticAbility::ward(crate::cost::TotalCost::from_cost(crate::costs::Cost::life(
                amount,
            ))),
        )]));
    }
    if let Some(marker) = parse_static_text_marker_line(&ability_tokens_with_period) {
        return Ok(Some(vec![grant_for_anthem_subject(clause, marker)]));
    }

    if let Some(abilities) = parse_static_ability_ast_line_lexed(&ability_tokens)? {
        return Ok(Some(grant_static_anthem_abilities_for_subject(
            clause, abilities,
        )));
    }

    Ok(None)
}

pub(crate) fn parse_anthem_with_trailing_segments_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let Some(get_idx) = anthem_token_offset(tokens, |token| {
        ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    let mut work_tokens = tokens.to_vec();
    if work_tokens
        .get(get_idx + 1)
        .is_some_and(|token| ANTHEM_ARTICLE_WORD_PATTERN.matches_token(token))
        && work_tokens
            .get(get_idx + 2)
            .is_some_and(|token| ANTHEM_ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        work_tokens.drain(get_idx + 1..get_idx + 3);
    }

    let Some(pt_word) = work_tokens
        .get(get_idx + 1)
        .and_then(OwnedLexToken::as_word)
    else {
        return Ok(None);
    };
    if parse_pt_modifier(pt_word).is_err() {
        return Ok(None);
    }

    let clause = parse_anthem_clause(&work_tokens, get_idx, get_idx + 2)?;
    let tail_tokens = trim_commas(&work_tokens[get_idx + 2..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);
    let direct_have_tail = if ANTHEM_AND_HAVE_OR_HAS_TAIL_PATTERN.matches_words(&tail_words) {
        Some(trim_commas(&tail_tokens[2..]))
    } else if ANTHEM_HAVE_OR_HAS_TAIL_PATTERN.matches_words(&tail_words) {
        Some(trim_commas(&tail_tokens[1..]))
    } else {
        None
    };

    if let Some(grant_tail) = direct_have_tail {
        let mut extras: Vec<StaticAbilityAst> = Vec::new();
        for raw_segment in split_anthem_trailing_segments_preserving_granted_abilities(&grant_tail)
        {
            let trimmed = trim_commas(&raw_segment);
            let mut segment = trim_edge_punctuation(&trimmed);
            while token_slice_first_is(&segment, "and") {
                let trimmed = trim_commas(&segment[1..]);
                segment = trim_edge_punctuation(&trimmed);
            }
            if segment.is_empty() {
                continue;
            }

            if let Some(mut granted) =
                parse_continuing_anthem_granted_segment(&clause, &clause_words, &segment)?
            {
                extras.append(&mut granted);
                continue;
            }

            let segment_words_storage = normalize_cant_words(&segment);
            let segment_words = segment_words_storage
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            if ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(&segment_words) {
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
                continue;
            }
            if ANTHEM_CANT_ATTACK_ALONE_PATTERN.matches_words(&segment_words) {
                extras.push(
                    grant_for_anthem_subject(
                        &clause,
                        StaticAbility::restriction(
                            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
                            "This creature can't attack alone".to_string(),
                        ),
                    )
                    .into(),
                );
                continue;
            }

            return Ok(None);
        }

        if extras.is_empty() {
            return Ok(None);
        }

        let mut result = vec![build_anthem_static_ability(&clause).into()];
        result.extend(extras);
        return Ok(Some(result));
    }

    let mut extras: Vec<StaticAbilityAst> = Vec::new();
    let mut continuing_have_clause = false;
    for raw_segment in split_anthem_trailing_segments_preserving_granted_abilities(&tail_tokens) {
        let trimmed = trim_commas(&raw_segment);
        let mut segment = trim_edge_punctuation(&trimmed);
        while token_slice_first_is(&segment, "and") {
            let trimmed = trim_commas(&segment[1..]);
            segment = trim_edge_punctuation(&trimmed);
        }
        if segment.is_empty() {
            continue;
        }

        let segment_words_storage = normalize_cant_words(&segment);
        let segment_words = segment_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if ANTHEM_CANT_BLOCK_PATTERN.matches_words(&segment_words) {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::cant_block()).into());
            continue;
        }
        if ANTHEM_CANT_ATTACK_ALONE_PATTERN.matches_words(&segment_words) {
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::restriction(
                        crate::effect::Restriction::attack_alone(ObjectFilter::source()),
                        "This creature can't attack alone".to_string(),
                    ),
                )
                .into(),
            );
            continue;
        }
        if ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(&segment_words) {
            extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
            continue;
        }
        if let Some((count, used)) = anthem_cant_be_blocked_max_blockers(&segment_words) {
            if used != segment_words.len() {
                return Ok(None);
            }
            extras.push(
                grant_for_anthem_subject(
                    &clause,
                    StaticAbility::cant_be_blocked_by_more_than(count as usize),
                )
                .into(),
            );
            continue;
        }
        if segment_words.len() == 2 && IS_WORD_PATTERN.matches_word(segment_words[0]) {
            let Some(color) = parse_color(segment_words[1]) else {
                return Ok(None);
            };
            let filter = match &clause.subject {
                AnthemSubjectAst::Source => ObjectFilter::source(),
                AnthemSubjectAst::Filter(filter) => filter.clone(),
            };
            let mut set_colors = crate::static_abilities::SetColorsForFilter::new(filter, color);
            if let Some(condition) = &clause.condition {
                set_colors = set_colors.with_condition(condition.clone());
            }
            extras.push(StaticAbility::new(set_colors).into());
            continue;
        }

        if segment_words
            .first()
            .is_some_and(|word| ANTHEM_LOSE_OR_LOSES_WORD_PATTERN.matches_word(word))
        {
            let ability_token_storage = trim_commas(&segment[1..]);
            let ability_tokens = trim_edge_punctuation(&ability_token_storage);
            if ability_tokens.is_empty() {
                return Ok(None);
            }
            let Some(actions) = parse_ability_line(&ability_tokens) else {
                return Ok(None);
            };
            reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
            let removed = actions
                .into_iter()
                .filter_map(|action| keyword_action_to_static_ability(action))
                .collect::<Vec<_>>();
            if removed.is_empty() {
                return Ok(None);
            }
            for ability in removed {
                extras.push(match &clause.subject {
                    AnthemSubjectAst::Source => StaticAbilityAst::RemoveStaticAbility {
                        filter: ObjectFilter::source(),
                        ability: Box::new(StaticAbilityAst::Static(ability)),
                    },
                    AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
                        filter: filter.clone(),
                        ability: Box::new(StaticAbilityAst::RemoveStaticAbility {
                            filter: ObjectFilter::source(),
                            ability: Box::new(StaticAbilityAst::Static(ability)),
                        }),
                        condition: clause.condition.clone(),
                    },
                });
            }
            continue;
        }

        if segment_words
            .first()
            .is_some_and(|word| ANTHEM_HAVE_OR_HAS_WORD_PATTERN.matches_word(word))
        {
            let mut ability_tokens = trim_edge_punctuation(&segment[1..]);
            if ability_tokens.is_empty() {
                return Ok(None);
            }

            let mut grant_must_attack = false;
            let ability_words_storage = normalize_cant_words(&ability_tokens);
            let ability_words = ability_words_storage
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            if let Some(and_idx) = anthem_find_prefix_shape_start(
                &ability_words,
                &ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN,
            ) {
                let Some(and_token_idx) = token_index_for_word_index(&ability_tokens, and_idx)
                else {
                    return Ok(None);
                };
                let head = trim_commas(&ability_tokens[..and_token_idx]);
                if head.is_empty() {
                    return Ok(None);
                }
                ability_tokens = head.to_vec();
                grant_must_attack = true;
            }

            let mut granted_activated: Option<ParsedAbility> = None;
            let mut granted_activated_display: Option<String> = None;
            let split_keyword_and_activated = if contains_token_kind(
                &ability_tokens,
                TokenKind::Colon,
            ) {
                let Some(colon_idx) =
                    anthem_token_offset(&ability_tokens, |token| token.is_colon())
                else {
                    return Ok(None);
                };
                let and_idx = anthem_last_index_where(colon_idx, |idx| {
                    AND_WORD_PATTERN.matches_token(&ability_tokens[idx])
                });
                if let Some(and_idx) = and_idx {
                    let keyword_head = trim_edge_punctuation(&ability_tokens[..and_idx]);
                    let activated_tail = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
                    if keyword_head.is_empty() || activated_tail.is_empty() {
                        return Ok(None);
                    }
                    let Some(actions) = parse_ability_line(&keyword_head) else {
                        return Ok(None);
                    };
                    let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
                    let Some(parsed) = parse_activated_line(&activated_tail)? else {
                        if has_colon {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported granted activated ability in anthem clause (clause: '{}')",
                                clause_words.join(" ")
                            )));
                        }
                        return Ok(None);
                    };
                    let display = display_text_for_tokens(&activated_tail, false);
                    granted_activated_display = Some(display);
                    granted_activated = Some(parsed);
                    Some(actions)
                } else {
                    None
                }
            } else {
                None
            };
            let actions = if let Some(actions) = split_keyword_and_activated {
                Some(actions)
            } else if let Some(GrantedAbilityAst::ParsedObjectAbility { ability, display }) =
                parse_granted_activated_or_triggered_ability_for_gain(
                    &ability_tokens,
                    &clause_words,
                )?
            {
                granted_activated_display = Some(display);
                granted_activated = Some(ability);
                None
            } else if let Some(actions) = parse_ability_line(&ability_tokens) {
                Some(actions)
            } else if contains_token_kind(&ability_tokens, TokenKind::Colon) {
                let Some(colon_idx) =
                    anthem_token_offset(&ability_tokens, |token| token.is_colon())
                else {
                    return Ok(None);
                };
                let and_idx = anthem_last_index_where(colon_idx, |idx| {
                    AND_WORD_PATTERN.matches_token(&ability_tokens[idx])
                });
                let Some(and_idx) = and_idx else {
                    return Ok(None);
                };
                let keyword_head = trim_edge_punctuation(&ability_tokens[..and_idx]);
                let activated_tail = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
                if keyword_head.is_empty() || activated_tail.is_empty() {
                    return Ok(None);
                }
                let Some(actions) = parse_ability_line(&keyword_head) else {
                    return Ok(None);
                };
                let has_colon = contains_token_kind(&activated_tail, TokenKind::Colon);
                let Some(parsed) = parse_activated_line(&activated_tail)? else {
                    if has_colon {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported granted activated ability in anthem clause (clause: '{}')",
                            clause_words.join(" ")
                        )));
                    }
                    return Ok(None);
                };
                let display = display_text_for_tokens(&activated_tail, false);
                granted_activated_display = Some(display);
                granted_activated = Some(parsed);
                Some(actions)
            } else {
                None
            };

            if let Some(triggered) = parse_triggered_granted_ability(&ability_tokens)? {
                let display = format!(
                    "{} has {}",
                    clause_words.join(" "),
                    crate::runtime_backend::token_word_refs(&ability_tokens).join(" ")
                );
                extras.push(grant_object_ability_for_anthem_subject(
                    &clause, triggered, display,
                ));
            } else if let Some(actions) = actions {
                reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
                let granted = actions
                    .into_iter()
                    .filter_map(|action| keyword_action_to_static_ability(action))
                    .collect::<Vec<_>>();
                if granted.is_empty() {
                    return Ok(None);
                }
                for ability in granted {
                    extras.push(grant_for_anthem_subject(&clause, ability).into());
                }

                if let Some(activated) = granted_activated {
                    extras.push(grant_object_ability_for_anthem_subject(
                        &clause,
                        activated,
                        granted_activated_display.unwrap_or_else(|| clause_words.join(" ")),
                    ));
                }
            } else {
                return Ok(None);
            }

            if grant_must_attack {
                extras.push(grant_for_anthem_subject(&clause, StaticAbility::must_attack()).into());
            }
            continuing_have_clause = true;
            continue;
        }

        if continuing_have_clause
            && let Some(mut granted) =
                parse_continuing_anthem_granted_segment(&clause, &clause_words, &segment)?
        {
            extras.append(&mut granted);
            continue;
        }

        if let Some(triggered) = parse_triggered_granted_ability(&segment)? {
            let display = format!(
                "{} has {}",
                clause_words.join(" "),
                crate::runtime_backend::token_word_refs(&segment).join(" ")
            );
            extras.push(grant_object_ability_for_anthem_subject(
                &clause, triggered, display,
            ));
            continue;
        }

        return Ok(None);
    }

    if extras.is_empty() {
        return Ok(None);
    }

    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.extend(extras);
    Ok(Some(result))
}

pub(crate) fn parse_conditional_all_creatures_able_to_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let all_words_storage = normalize_cant_words(tokens);
    let all_words = all_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&all_words) {
        return Ok(None);
    }

    let Some(comma_idx) = anthem_token_offset(tokens, |token| token.is_comma()) else {
        return Ok(None);
    };
    if comma_idx <= 3 {
        return Ok(None);
    }

    let condition_tokens = trim_commas(&tokens[3..comma_idx]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let condition = parse_static_condition_clause(&condition_tokens)?;

    let remainder = trim_commas(&tokens[comma_idx + 1..]);
    let remainder_words_storage = normalize_cant_words(&remainder);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if ALL_CREATURES_BLOCK_THIS_CREATURE_TAIL_PATTERN.matches_words(&remainder_words) {
        return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
            condition,
        }));
    }

    if ALL_CREATURES_BLOCK_ENCHANTED_CREATURE_TAIL_PATTERN.matches_words(&remainder_words) {
        return Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
            display: "enchanted creature has this creature must be blocked if able".to_string(),
            condition: Some(condition),
        }));
    }

    Ok(None)
}

pub(crate) fn parse_source_can_attack_as_though_no_defender_as_long_as_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .map(|word| {
            if ANTHEM_DIDNT_CONTRACTION_WORD_PATTERN.matches_word(word) {
                "didnt"
            } else {
                word
            }
        })
        .collect::<Vec<_>>();
    if !CAN_ATTACK_AS_NO_DEFENDER_AS_LONG_AS_PATTERN.matches_words(&normalized) {
        return Ok(None);
    }
    let Some(can_idx) = anthem_find_prefix_shape_start(
        &normalized,
        &CAN_ATTACK_AS_NO_DEFENDER_AS_LONG_AS_PREFIX_PATTERN,
    ) else {
        return Ok(None);
    };
    if can_idx == 0 {
        return Ok(None);
    }

    let subject_end = token_index_for_word_index(tokens, can_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map conditional no-defender subject (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let condition_start = token_index_for_word_index(tokens, can_idx + 11).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map conditional no-defender condition (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let condition_tokens = trim_commas(&tokens[condition_start..]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let condition = parse_static_condition_clause(&condition_tokens)?;

    let subject = parse_anthem_subject(&subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub(crate) fn parse_as_long_as_condition_can_attack_as_though_no_defender_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let normalized = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .map(|word| {
            if ANTHEM_DIDNT_CONTRACTION_WORD_PATTERN.matches_word(word) {
                "didnt"
            } else {
                word
            }
        })
        .collect::<Vec<_>>();
    if !CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&normalized) {
        return Ok(None);
    }

    if !CAN_ATTACK_AS_NO_DEFENDER_PATTERN.matches_words(&normalized) {
        return Ok(None);
    }
    let Some(can_idx) =
        anthem_find_prefix_shape_start(&normalized, &CAN_ATTACK_AS_NO_DEFENDER_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    let Some(comma_idx) = anthem_token_offset(tokens, |token| token.is_comma()) else {
        return Ok(None);
    };
    let Some(can_token_idx) = token_index_for_word_index(tokens, can_idx) else {
        return Ok(None);
    };
    if comma_idx >= can_token_idx {
        return Ok(None);
    }

    let condition_tokens = trim_commas(&tokens[3..comma_idx]);
    if condition_tokens.is_empty() {
        return Ok(None);
    }
    let subject_tokens = trim_commas(&tokens[comma_idx + 1..can_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let condition = parse_static_condition_clause(&condition_tokens)?;
    let subject = parse_anthem_subject(&subject_tokens)?;
    let granted = match subject {
        AnthemSubjectAst::Source => StaticAbilityAst::ConditionalStaticAbility {
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition,
        },
        AnthemSubjectAst::Filter(filter) => StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(
                StaticAbility::can_attack_as_though_no_defender(),
            )),
            condition: Some(condition),
        },
    };
    Ok(Some(granted))
}

pub(crate) fn parse_gets_and_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(get_idx) = anthem_token_offset(tokens, |token| {
        ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    let Some(and_idx) = anthem_token_offset_from(tokens, get_idx + 1, |token| {
        AND_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    let Some(attack_idx) = anthem_token_offset_from(tokens, and_idx + 1, |token| {
        ANTHEM_ATTACK_OR_ATTACKS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };

    let attack_tail = crate::runtime_backend::token_word_refs(&tokens[attack_idx..]);
    if !ANTHEM_ATTACK_EACH_COMBAT_IF_ABLE_TAIL_PATTERN.matches_words(&attack_tail) {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    result.push(grant_for_anthem_subject(
        &clause,
        StaticAbility::must_attack(),
    ));

    if result.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "failed to parse gets-and-attacks clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(result))
}

pub(crate) fn parse_anthem_and_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if contains_until_end_of_turn(&clause_words) {
        return Ok(None);
    }

    let Some(get_idx) = anthem_token_offset(tokens, |token| {
        ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    let Some(and_idx) = anthem_token_offset_from(tokens, get_idx + 1, |token| {
        AND_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    let tail_tokens = trim_edge_punctuation(&tokens[and_idx + 1..]);
    let tail_words = crate::runtime_backend::token_word_refs(&tail_tokens);

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let mut result = vec![build_anthem_static_ability(&clause).into()];
    if CANT_BE_BLOCKED_WORDS_PATTERN.matches_words(&tail_words) {
        result.push(grant_for_anthem_subject(
            &clause,
            StaticAbility::unblockable(),
        ));
    } else if tail_words
        .first()
        .is_some_and(|word| ANTHEM_BE_WORD_PATTERN.matches_word(word))
    {
        let Some(family) = parse_every_subtype_family_tail(&tail_words[1..]) else {
            return Ok(None);
        };
        result.push(every_subtype_family_for_subject(
            &clause.subject,
            family,
            clause.condition.clone(),
        ));
    } else {
        return Ok(None);
    }

    Ok(Some(result))
}

pub(crate) fn parse_subject_is_every_subtype_family_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let all_words = crate::runtime_backend::token_word_refs(tokens);
    if all_words.len() < 4 || contains_until_end_of_turn(&all_words) {
        return Ok(None);
    }

    let mut condition: Option<crate::ConditionExpr> = None;
    let clause_tokens_buf = if CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&all_words) {
        let Some(comma_idx) = anthem_token_offset(tokens, |token| token.is_comma()) else {
            return Ok(None);
        };
        if comma_idx <= 3 {
            return Ok(None);
        }
        let condition_tokens = trim_commas(&tokens[3..comma_idx]);
        if condition_tokens.is_empty() {
            return Ok(None);
        }
        condition = Some(parse_static_condition_clause(&condition_tokens)?);
        Some(trim_commas(&tokens[comma_idx + 1..]))
    } else {
        None
    };
    let clause_tokens = clause_tokens_buf.as_deref().unwrap_or(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(clause_tokens);
    let Some(be_word_idx) = ANTHEM_IS_OR_ARE_WORD_PATTERN.find_word(&clause_words) else {
        return Ok(None);
    };
    if be_word_idx == 0 {
        return Ok(None);
    }

    let Some(family) = parse_every_subtype_family_tail(&clause_words[be_word_idx + 1..]) else {
        return Ok(None);
    };
    let Some(be_token_idx) = token_index_for_word_index(clause_tokens, be_word_idx) else {
        return Err(CardTextError::ParseError(format!(
            "unable to map subject in every-subtype-family clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let subject_tokens = trim_commas(&clause_tokens[..be_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    Ok(Some(every_subtype_family_for_subject(
        &subject, family, condition,
    )))
}

pub(crate) fn parse_anthem_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    // Targeted "gets +N/+N" text is usually a one-shot spell/ability effect,
    // not a global/static anthem.
    if ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&words) {
        return Ok(None);
    }
    // "until end of turn" indicates a temporary effect, not a permanent anthem.
    if contains_until_end_of_turn(&words) {
        return Ok(None);
    }

    let get_idx = anthem_token_offset(tokens, |token| {
        ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
    });
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };
    let mut modifier_idx = get_idx + 1;
    if tokens
        .get(modifier_idx)
        .is_some_and(|token| ANTHEM_ARTICLE_WORD_PATTERN.matches_token(token))
        && tokens
            .get(modifier_idx + 1)
            .is_some_and(|token| ANTHEM_ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        modifier_idx += 2;
    }
    let Some(modifier_word) = tokens.get(modifier_idx).and_then(OwnedLexToken::as_word) else {
        return Ok(None);
    };
    if parse_pt_modifier_values(modifier_word).is_err() {
        return Ok(None);
    }
    let clause = parse_anthem_clause(tokens, get_idx, tokens.len())?;
    Ok(Some(build_anthem_static_ability(&clause)))
}

fn trim_multi_anthem_subject_segment(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut segment = trim_edge_punctuation(tokens);
    while crate::runtime_backend::lexer::token_slice_last_is(&segment, "each") {
        segment = trim_edge_punctuation(&segment[..segment.len() - 1]);
    }
    segment
}

pub(crate) fn parse_multi_subject_anthem_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&words) || contains_until_end_of_turn(&words) {
        return Ok(None);
    }

    let Some(get_idx) = anthem_token_offset(tokens, |token| {
        ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    let mut modifier_idx = get_idx + 1;
    if tokens
        .get(modifier_idx)
        .is_some_and(|token| ANTHEM_ARTICLE_WORD_PATTERN.matches_token(token))
        && tokens
            .get(modifier_idx + 1)
            .is_some_and(|token| ANTHEM_ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        modifier_idx += 2;
    }
    let Some(modifier_word) = tokens.get(modifier_idx).and_then(OwnedLexToken::as_word) else {
        return Ok(None);
    };
    if parse_pt_modifier_values(modifier_word).is_err() {
        return Ok(None);
    }

    let Ok((_prefix_condition, subject_start)) = parse_anthem_prefix_condition(tokens, get_idx)
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    if subject_tokens.is_empty()
        || !crate::runtime_backend::lexer::contains_token_word(&subject_tokens, "and")
    {
        return Ok(None);
    }

    let mut segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    let mut segment_start = 0usize;
    for (idx, token) in subject_tokens.iter().enumerate() {
        if AND_WORD_PATTERN.matches_token(token) {
            let segment = trim_multi_anthem_subject_segment(&subject_tokens[segment_start..idx]);
            if segment.is_empty() {
                return Ok(None);
            }
            segments.push(segment);
            segment_start = idx + 1;
        }
    }
    let segment = trim_multi_anthem_subject_segment(&subject_tokens[segment_start..]);
    if segment.is_empty() {
        return Ok(None);
    }
    segments.push(segment);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut abilities = Vec::with_capacity(segments.len());
    for segment in segments {
        let mut clause_tokens = Vec::with_capacity(tokens.len());
        clause_tokens.extend_from_slice(&tokens[..subject_start]);
        clause_tokens.extend_from_slice(&segment);
        clause_tokens.extend_from_slice(&tokens[get_idx..]);
        let adjusted_get_idx = subject_start + segment.len();
        let clause =
            match parse_anthem_clause(&clause_tokens, adjusted_get_idx, clause_tokens.len()) {
                Ok(clause) => clause,
                Err(_) => return Ok(None),
            };
        abilities.push(build_anthem_static_ability(&clause));
    }

    Ok(Some(abilities))
}

pub(crate) fn parse_has_base_power_toughness_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    let Some(has_idx) = HAS_OR_HAVE_WORD_PATTERN.find_word(&words_all) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&subject_words) {
        return Ok(None);
    }
    if starts_with_until_end_of_turn(&subject_words)
        || UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN.matches_words(&subject_words)
    {
        return Ok(None);
    }

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5 || !ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(rest_words)
    {
        return Ok(None);
    }
    let tail = &rest_words[5..];
    if !tail.is_empty() {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let subject = parse_anthem_subject(&subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    Ok(Some(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    )))
}

fn is_negated_creature_tail(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    let is_creature_phrase = |tail: &[&str]| {
        matches!(
            tail,
            ["creature"] | ["creatures"] | ["a", "creature"] | ["an", "creature"]
        )
    };

    let be = words[0];
    if ANTHEM_BE_NEGATED_WORD_PATTERN.matches_word(be) {
        return is_creature_phrase(&words[1..]);
    }

    if ANTHEM_BE_WORD_PATTERN.matches_word(be) {
        if words
            .get(1)
            .is_some_and(|word| ANTHEM_NOT_WORD_PATTERN.matches_word(word))
        {
            return is_creature_phrase(&words[2..]);
        }
        if ANTHEM_NO_LONGER_PREFIX_PATTERN.matches_words(&words[1..]) {
            return is_creature_phrase(&words[3..]);
        }
    }

    false
}

pub(crate) fn parse_isnt_creature_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let all_words = crate::runtime_backend::token_word_refs(tokens);
    if all_words.len() < 3 {
        return Ok(None);
    }
    if ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&all_words)
        || contains_until_end_of_turn(&all_words)
    {
        return Ok(None);
    }

    let mut condition: Option<crate::ConditionExpr> = None;
    let clause_tokens_buf = if CANT_BE_BLOCKED_AS_LONG_AS_TAIL_PATTERN.matches_words(&all_words) {
        let Some(comma_idx) = anthem_token_offset(tokens, |token| token.is_comma()) else {
            return Ok(None);
        };
        if comma_idx <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                all_words.join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..comma_idx]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                all_words.join(" ")
            )));
        }
        condition = Some(parse_static_condition_clause(&condition_tokens)?);
        Some(trim_commas(&tokens[comma_idx + 1..]))
    } else {
        None
    };
    let mut clause_tokens_storage: Vec<OwnedLexToken> = Vec::new();
    let mut clause_tokens = clause_tokens_buf.as_deref().unwrap_or(tokens);

    let clause_words = crate::runtime_backend::token_word_refs(clause_tokens);
    if let Some(unless_word_idx) = clause_words.iter().position(|word| *word == "unless") {
        let unless_token_idx = token_index_for_word_index(clause_tokens, unless_word_idx)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map unless condition in isn't-a-creature clause (clause: '{}')",
                    all_words.join(" ")
                ))
            })?;
        let condition_tokens = trim_commas(&clause_tokens[unless_token_idx + 1..]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'unless' clause (clause: '{}')",
                all_words.join(" ")
            )));
        }
        let unless_condition = crate::ConditionExpr::Not(Box::new(parse_static_condition_clause(
            &condition_tokens,
        )?));
        condition = Some(match condition {
            Some(existing) => {
                crate::ConditionExpr::And(Box::new(existing), Box::new(unless_condition))
            }
            None => unless_condition,
        });
        clause_tokens_storage.extend(trim_commas(&clause_tokens[..unless_token_idx]));
        clause_tokens = &clause_tokens_storage;
    }

    let clause_words = crate::runtime_backend::token_word_refs(clause_tokens);
    if clause_words.len() < 3 {
        return Ok(None);
    }

    let Some(verb_word_idx) = ANTHEM_BE_WORD_PATTERN
        .find_word(&clause_words)
        .or_else(|| ANTHEM_BE_NEGATED_WORD_PATTERN.find_word(&clause_words))
    else {
        return Ok(None);
    };
    if !is_negated_creature_tail(&clause_words[verb_word_idx..]) {
        return Ok(None);
    }

    let verb_token_idx =
        token_index_for_word_index(clause_tokens, verb_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map subject in isn't-a-creature clause (clause: '{}')",
                all_words.join(" ")
            ))
        })?;
    let subject_tokens = trim_commas(&clause_tokens[..verb_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    let mut remove =
        crate::static_abilities::RemoveCardTypesForFilter::new(filter, vec![CardType::Creature]);
    if let Some(condition) = condition {
        remove = remove.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(remove)))
}

pub(crate) fn parse_has_base_power_toughness_and_granted_keywords_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(has_idx) = anthem_token_offset(tokens, |token| {
        ANTHEM_HAVE_OR_HAS_WORD_PATTERN.matches_token(token)
    }) else {
        return Ok(None);
    };
    if has_idx == 0 || has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if ANTHEM_TARGET_CONTAINS_PATTERN.matches_words(&subject_words) {
        return Ok(None);
    }
    if starts_with_until_end_of_turn(&subject_words)
        || UNTIL_YOUR_NEXT_TURN_PREFIX_PATTERN.matches_words(&subject_words)
    {
        return Ok(None);
    }

    let rest_tokens = trim_commas(&tokens[has_idx + 1..]);
    let rest_words = crate::runtime_backend::token_word_refs(&rest_tokens);
    if rest_words.len() < 8
        || !ANTHEM_BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(&rest_words)
    {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if !AND_WORD_PATTERN.matches_word(rest_words[5]) {
        return Ok(None);
    }
    if !ANTHEM_HAVE_HAS_GAIN_GAINS_WORD_PATTERN.matches_word(rest_words[6]) {
        return Ok(None);
    }

    let Some(ability_start_idx) = token_index_for_word_index(&rest_tokens, 7) else {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list after base power/toughness clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let ability_tokens = trim_commas(&rest_tokens[ability_start_idx..]);
    if ability_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list after base power/toughness clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let Some(actions) = parse_ability_line(&ability_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let granted = actions;
    if granted.is_empty() {
        return Ok(None);
    }

    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };

    let mut compiled = Vec::new();
    match subject {
        AnthemSubjectAst::Source => {
            let source_filter = if THIS_CREATURE_PREFIX_PATTERN.matches_words(&subject_words) {
                ObjectFilter::source().with_type(CardType::Creature)
            } else {
                ObjectFilter::source()
            };
            let set_base =
                StaticAbility::set_base_power_toughness(source_filter, power, toughness).into();
            compiled.push(if let Some(condition) = condition.clone() {
                StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(set_base),
                    condition,
                }
            } else {
                set_base
            });
            compiled.extend(granted.into_iter().map(|action| {
                if let Some(condition) = condition.clone() {
                    StaticAbilityAst::ConditionalKeywordAction { action, condition }
                } else {
                    StaticAbilityAst::KeywordAction(action)
                }
            }));
        }
        AnthemSubjectAst::Filter(filter) => {
            let set_base =
                StaticAbility::set_base_power_toughness(filter.clone(), power, toughness).into();
            compiled.push(if let Some(condition) = condition.clone() {
                StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(set_base),
                    condition,
                }
            } else {
                set_base
            });
            for action in granted {
                compiled.push(StaticAbilityAst::GrantKeywordAction {
                    filter: filter.clone(),
                    action,
                    condition: condition.clone(),
                });
            }
        }
    }

    Ok(Some(compiled))
}

pub(crate) fn parse_filter_has_granted_ability_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let mut deferred_error: Option<CardTextError> = None;
    let mut inside_quotes = false;
    for (has_idx, token) in tokens.iter().enumerate() {
        if token.is_quote() {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes {
            continue;
        }
        if !ANTHEM_HAVE_OR_HAS_WORD_PATTERN.matches_token(token) {
            continue;
        }
        if has_idx == 0 || has_idx + 1 >= tokens.len() {
            continue;
        }
        if tokens[..has_idx]
            .iter()
            .any(|token| ANTHEM_GET_OR_GETS_WORD_PATTERN.matches_token(token))
        {
            continue;
        }

        let (mut condition, subject_start) = match parse_anthem_prefix_condition(tokens, has_idx) {
            Ok(parsed) => parsed,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let subject_tokens = trim_commas(&tokens[subject_start..has_idx]);
        if subject_tokens.is_empty() {
            continue;
        }
        let subject_tokens_for_type_addition = if subject_tokens
            .last()
            .is_some_and(|token| AND_WORD_PATTERN.matches_token(token))
        {
            trim_commas(&subject_tokens[..subject_tokens.len().saturating_sub(1)])
        } else {
            subject_tokens.clone()
        };
        if let Some(is_idx) =
            anthem_token_offset_from(&subject_tokens_for_type_addition, 1, |token| {
                ANTHEM_IS_OR_ARE_WORD_PATTERN.matches_token(token)
            })
        {
            let base_subject_tokens = trim_commas(&subject_tokens_for_type_addition[..is_idx]);
            let addition_tokens = trim_commas(&subject_tokens_for_type_addition[is_idx..]);
            if !base_subject_tokens.is_empty()
                && let Some(additions) = parse_type_color_addition_clause(&addition_tokens)?
            {
                let base_subject = match parse_anthem_subject(&base_subject_tokens) {
                    Ok(subject) => subject,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                };
                let AnthemSubjectAst::Filter(filter) = &base_subject else {
                    continue;
                };
                let ability_tokens = trim_commas(&tokens[has_idx + 1..]);
                let attached_subject =
                    crate::runtime_backend::token_word_refs(&base_subject_tokens)
                        .first()
                        .is_some_and(|word| {
                            ANTHEM_ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word)
                        });
                let granted_tail = match parse_heterogeneous_granted_tail(
                    &ability_tokens,
                    &clause_words,
                    attached_subject,
                ) {
                    Ok(Some(tail)) => tail,
                    Ok(None) => continue,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                };
                let mut result = Vec::new();
                if !additions.set_colors.is_empty() {
                    result.push(
                        StaticAbility::set_colors(filter.clone(), additions.set_colors).into(),
                    );
                }
                if !additions.added_colors.is_empty() {
                    result.push(
                        StaticAbility::add_colors(filter.clone(), additions.added_colors).into(),
                    );
                }
                if !additions.card_types.is_empty() {
                    result.push(
                        StaticAbility::add_card_types(filter.clone(), additions.card_types).into(),
                    );
                }
                if !additions.subtypes.is_empty() {
                    result.push(
                        StaticAbility::add_subtypes(filter.clone(), additions.subtypes).into(),
                    );
                }
                result.extend(lower_granted_tail_for_anthem_subject(
                    &base_subject,
                    &condition,
                    granted_tail,
                ));
                if !result.is_empty() {
                    return Ok(Some(result));
                }
            }
        }
        let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
        if subject_words.iter().any(|word| {
            matches!(
                *word,
                "deal"
                    | "deals"
                    | "create"
                    | "creates"
                    | "draw"
                    | "draws"
                    | "destroy"
                    | "destroys"
                    | "exile"
                    | "exiles"
                    | "return"
                    | "returns"
                    | "sacrifice"
                    | "sacrifices"
                    | "put"
                    | "puts"
                    | "gain"
                    | "gains"
                    | "lose"
                    | "loses"
                    | "discard"
                    | "discards"
                    | "counter"
                    | "counters"
                    | "search"
                    | "reveals"
                    | "investigate"
                    | "investigates"
            )
        }) {
            continue;
        }
        if subject_words
            .iter()
            .any(|word| ANTHEM_MAY_WORD_PATTERN.matches_word(word))
        {
            continue;
        }

        let ability_tokens_raw = &tokens[has_idx + 1..];
        let mut ability_tokens = trim_commas(ability_tokens_raw);
        if !contains_token_kind(&ability_tokens, TokenKind::Quote) {
            let ability_words = crate::runtime_backend::token_word_refs(&ability_tokens);
            if let Some(as_long_as_idx) =
                anthem_find_prefix_shape_start(&ability_words, &ANTHEM_AS_LONG_AS_PREFIX_PATTERN)
                && as_long_as_idx > 0
                && let Some(condition_start) =
                    token_index_for_word_index(&ability_tokens, as_long_as_idx)
            {
                let condition_tokens = trim_commas(&ability_tokens[condition_start + 3..]);
                if !condition_tokens.is_empty() {
                    let parsed_condition = match parse_static_condition_clause(&condition_tokens) {
                        Ok(condition) => condition,
                        Err(err) => {
                            deferred_error.get_or_insert(err);
                            continue;
                        }
                    };
                    condition = Some(match condition {
                        Some(existing) => crate::ConditionExpr::And(
                            Box::new(existing),
                            Box::new(parsed_condition),
                        ),
                        None => parsed_condition,
                    });
                    ability_tokens = trim_commas(&ability_tokens[..condition_start]);
                }
            }
            let ability_words = crate::runtime_backend::token_word_refs(&ability_tokens);
            if let Some(if_idx) = ANTHEM_IF_WORD_PATTERN.find_word(&ability_words)
                && if_idx > 0
                && let Some(condition_start) = token_index_for_word_index(&ability_tokens, if_idx)
            {
                let condition_tokens = trim_commas(&ability_tokens[condition_start + 1..]);
                if !condition_tokens.is_empty() {
                    let parsed_condition = match parse_static_condition_clause(&condition_tokens) {
                        Ok(condition) => condition,
                        Err(err) => {
                            deferred_error.get_or_insert(err);
                            continue;
                        }
                    };
                    condition = Some(match condition {
                        Some(existing) => crate::ConditionExpr::And(
                            Box::new(existing),
                            Box::new(parsed_condition),
                        ),
                        None => parsed_condition,
                    });
                    ability_tokens = trim_commas(&ability_tokens[..condition_start]);
                }
            }
        }
        let ability_words = crate::runtime_backend::token_word_refs(&ability_tokens);
        let attached_subject = subject_words
            .first()
            .is_some_and(|word| ANTHEM_ENCHANTED_OR_EQUIPPED_WORD_PATTERN.matches_word(word));
        let ability_sentences =
            crate::runtime_backend::grammar::primitives::split_lexed_slices_on_period(
                &ability_tokens,
            );
        if ability_sentences.len() > 1 {
            let leading = trim_edge_punctuation(ability_sentences[0]);
            let trailing = ability_sentences[1..]
                .iter()
                .flat_map(|sentence| trim_edge_punctuation(sentence))
                .collect::<Vec<_>>();
            if ANTHEM_BLITZ_KEYWORD_PATTERN
                .matches_words(&crate::runtime_backend::token_word_refs(&leading))
                && is_granted_blitz_cost_tail(&trailing)
            {
                match granted_blitz_abilities_from_subject(&subject_tokens, condition.clone()) {
                    Ok(Some(grants)) => return Ok(Some(grants)),
                    Ok(None) => continue,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                }
            }
            if ANTHEM_EMERGE_KEYWORD_PATTERN
                .matches_words(&crate::runtime_backend::token_word_refs(&leading))
                && is_granted_emerge_cost_tail(&trailing)
            {
                match granted_emerge_abilities_from_subject(&subject_tokens, condition.clone()) {
                    Ok(Some(grants)) => return Ok(Some(grants)),
                    Ok(None) => continue,
                    Err(err) => {
                        deferred_error.get_or_insert(err);
                        continue;
                    }
                }
            }
        }
        if ANTHEM_EMERGE_KEYWORD_PATTERN.matches_words(&ability_words) {
            match granted_emerge_abilities_from_subject(&subject_tokens, condition.clone()) {
                Ok(Some(grants)) => return Ok(Some(grants)),
                Ok(None) => continue,
                Err(err) => {
                    deferred_error.get_or_insert(err);
                    continue;
                }
            }
        }
        let granted_tail = match parse_heterogeneous_granted_tail(
            &ability_tokens,
            &clause_words,
            attached_subject,
        ) {
            Ok(Some(tail)) => tail,
            Ok(None) => continue,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let attached_subject_filter =
            infer_attached_subject_filter_from_condition_expr(condition.as_ref());
        let subject = match parse_anthem_subject_with_attached_fallback(
            &subject_tokens,
            attached_subject_filter.as_ref(),
        ) {
            Ok(subject) => subject,
            Err(err) => {
                deferred_error.get_or_insert(err);
                continue;
            }
        };
        let granted = lower_granted_tail_for_anthem_subject(&subject, &condition, granted_tail);
        if granted.is_empty() {
            continue;
        }
        return Ok(Some(granted));
    }

    if let Some(err) = deferred_error {
        return Err(err);
    }
    Ok(None)
}

#[test]
fn attached_object_anthem_subject_uses_tagged_constraints() {
    let enchanted = AnthemSubjectAst::Filter(ObjectFilter::tagged("enchanted"));
    assert!(attached_object_anthem_subject_filter(&enchanted).is_some());

    let equipped = AnthemSubjectAst::Filter(ObjectFilter::tagged("equipped"));
    assert!(attached_object_anthem_subject_filter(&equipped).is_some());

    let creature = AnthemSubjectAst::Filter(ObjectFilter::creature());
    assert!(attached_object_anthem_subject_filter(&creature).is_none());
}

#[test]
fn keyword_and_unblockable_tail_keeps_multiple_captured_keywords() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "This creature has flying and vigilance and can't be blocked.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_subject_has_keywords_and_cant_be_blocked_line(&tokens)
        .expect("parser should not error")
        .expect("line should parse");

    assert!(matches!(
        parsed.as_slice(),
        [
            StaticAbilityAst::KeywordAction(KeywordAction::Flying),
            StaticAbilityAst::KeywordAction(KeywordAction::Vigilance),
            StaticAbilityAst::KeywordAction(KeywordAction::Unblockable),
        ]
    ));
}

#[test]
fn granted_escape_tail_captures_dynamic_exile_count() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "The escape cost is equal to the card's mana cost plus exile three other cards from your graveyard.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_granted_escape_cost_tail_clause(&tokens)
        .expect("escape tail should parse");
    let (count, used) =
        parse_number(parsed.exile_count_tokens).expect("captured count should parse");

    assert_eq!(count, 3);
    assert_eq!(used, parsed.exile_count_tokens.len());
}

#[test]
fn granted_miracle_tail_captures_dynamic_cost_reduction() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Its miracle cost is equal to its mana cost reduced by {4}.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_granted_miracle_cost_reduction_tail_clause(&tokens)
        .expect("miracle tail should parse");
    let (cost, used) =
        crate::runtime_backend::front_end::shared::util::leading_mana_cost_from_tokens(
            parsed.reduction_cost_tokens,
        )
        .expect("captured cost should parse");

    assert_eq!(cost.generic_mana_total(), 4);
    assert_eq!(used, parsed.reduction_cost_tokens.len());
}

#[test]
fn cant_be_blocked_by_more_than_clause_captures_subject_and_threshold() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Each creature you control with a +1/+1 counter on it can't be blocked by more than one creature.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_cant_be_blocked_by_more_than_clause(&tokens)
        .expect("max-blockers clause should parse");
    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    let (minimum_blockers, used) = parse_greater_than_or_equal_quantity_prefix(
        parsed.blocker_threshold_tokens,
        false,
        false,
        "test blocker threshold",
    )
    .expect("threshold should parse")
    .expect("threshold should be present");

    assert_eq!(
        subject_words.as_slice(),
        &[
            "each", "creature", "you", "control", "with", "a", "+1/+1", "counter", "on", "it"
        ]
    );
    assert_eq!(minimum_blockers, 2);
    assert_eq!(used, parsed.blocker_threshold_tokens.len());
}

#[test]
fn can_block_additional_creature_clause_captures_subject_and_count() {
    let tokens = crate::runtime_backend::lexer::lex_line(
        "Each creature you control can block two additional creatures each combat.",
        0,
    )
    .expect("line should lex");
    let parsed = parse_can_block_additional_creature_clause(&tokens)
        .expect("additional-blocker clause should parse");
    let subject_words = crate::runtime_backend::lexer::parser_token_word_refs(parsed.subject_tokens);
    let (count, used) = parse_number(parsed.additional_count_tokens)
        .expect("captured additional blocker count should parse");

    assert_eq!(subject_words.as_slice(), &["each", "creature", "you", "control"]);
    assert_eq!(count, 2);
    assert_eq!(used, parsed.additional_count_tokens.len());
}

#[test]
fn landwalk_override_tail_uses_keyword_action_parser() {
    assert!(is_landwalk_ability_word("islandwalk"));
    assert!(is_landwalk_ability_word("forestwalk"));
    assert!(!is_landwalk_ability_word("planeswalk"));
    assert!(!is_landwalk_ability_word("walk"));
}

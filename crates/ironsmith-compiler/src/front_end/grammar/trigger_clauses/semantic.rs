use super::*;
use crate::grammar::abilities as ability_grammar;
use crate::grammar::leaf;
use crate::grammar::trigger_clauses::{
    self as trigger_grammar, TriggerClauseAtom, TriggerClausePattern,
    trigger_clause_pattern as clause_shape,
};
use crate::lexer::{token_slice_at_is, token_slice_at_is_any};

type ClauseShape<'p> = TriggerClausePattern<'p>;

const THIS_DESTINATION_TRIGGER_NAME_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this"]);
const THIS_OR_IT_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["this"], &["it"]]);
const DAMAGER_NAMED_SOURCE_LEADING_EXCLUDED_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a"],
            &["an"],
            &["the"],
            &["target"],
            &["that"],
            &["this"],
            &["equipped"],
            &["enchanted"],
        ]
);
const GENERIC_DAMAGE_SOURCE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["creature"],
            &["creatures"],
            &["permanent"],
            &["permanents"],
            &["source"],
            &["sources"],
        ]
);

const PLAYERS_FINISH_VOTING_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["players", "finish", "voting"],
            &["players", "finished", "voting"],
        ]
);
const YOU_CYCLE_THIS_CARD_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "cycle", "this", "card"],
            &["you", "cycled", "this", "card"],
        ]
);
const YOU_CYCLE_OR_DISCARD_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "cycle", "or", "discard", "a", "card"],
            &["you", "cycle", "or", "discard", "card"],
        ]
);
const YOU_COMMIT_CRIME_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "commit", "a", "crime"]);
const OPPONENT_COMMITS_CRIME_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "opponent", "commits", "a", "crime"],
            &["opponent", "commits", "a", "crime"],
            &["opponents", "commit", "a", "crime"],
        ]
);
const PLAYER_COMMITS_CRIME_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "player", "commits", "a", "crime"],
            &["a", "player", "commit", "a", "crime"],
        ]
);
const YOU_UNLOCK_THIS_DOOR_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "unlock", "this", "door"],
            &["you", "unlocked", "this", "door"],
        ]
);
const THIS_CARD_BECOMES_PLOTTED_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "card", "becomes", "plotted"],
            &["this", "becomes", "plotted"],
            &["becomes", "plotted"],
        ]
);
const THE_RING_TEMPTS_YOU_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["the", "ring", "tempts", "you"]);
const CHAOS_ENSUES_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["chaos", "ensues"]);
const YOU_ENCOUNTER_PHENOMENON_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["you", "encounter"]);
const YOU_SET_THIS_SCHEME_IN_MOTION_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "set", "this", "scheme", "in", "motion"],
            &["you", "set", "this", "scheme", "in", "motion", "again"],
        ]
);
const THIS_BECOMES_TAPPED_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "becomes", "tapped"],
            &["this", "becomes", "tapped"],
            &["becomes", "tapped"],
        ]
);
const THIS_BECOMES_UNTAPPED_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "becomes", "untapped"],
            &["this", "becomes", "untapped"],
            &["becomes", "untapped"],
        ]
);
const THIS_BECOMES_MONSTROUS_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "becomes", "monstrous"],
            &["this", "permanent", "becomes", "monstrous"],
            &["this", "becomes", "monstrous"],
            &["becomes", "monstrous"],
        ]
);
const THIS_MUTATES_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "mutates"],
            &["this", "permanent", "mutates"],
            &["this", "mutates"],
            &["mutates"],
        ]
);
const THIS_TURNED_FACE_UP_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "is", "turned", "face", "up"],
            &["this", "permanent", "is", "turned", "face", "up"],
            &["this", "is", "turned", "face", "up"],
        ]
);
const YOU_GAIN_LIFE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "gain", "life"]);
const YOU_DRAW_CARD_TRIGGER_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const BEGINNING_END_STEP_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "end", "step"]);
const NEXT_END_STEP_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["next", "end", "step"]);
const BEGINNING_UPKEEP_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "upkeep"]);
const BEGINNING_DRAW_STEP_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "draw", "step"]);
const BEGINNING_COMBAT_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "combat"]);
const BEGINNING_MAIN_PHASE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "main"]);
const BEGINNING_FIRST_MAIN_PHASE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "first", "main", "phase"]);
const BEGINNING_SECOND_MAIN_PHASE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "second", "main", "phase"]);
const BEGINNING_PRECOMBAT_MAIN_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "precombat", "main"]);
const BEGINNING_POSTCOMBAT_MAIN_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "postcombat", "main"]);
const THIS_DAMAGE_SOURCE_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "source"],
            &["this"],
        ]
);
const EQUIPPED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["equipped", "creature"]);
const ENCHANTED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["enchanted", "creature"]);
const UNQUALIFIED_SPELL_FILTER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "spell"], &["spell"], &["spells"]]);
const CAST_OR_COPY_SEPARATOR_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const CRAFT_EXILED_FROM_BATTLEFIELD_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this",
                "creature",
                "is",
                "exiled",
                "from",
                "the",
                "battlefield",
                "while",
                "youre",
                "activating",
                "a",
                "craft",
                "ability",
            ],
            &[
                "this",
                "creature",
                "is",
                "exiled",
                "from",
                "the",
                "battlefield",
                "while",
                "you're",
                "activating",
                "a",
                "craft",
                "ability",
            ],
        ]
);
const FINAL_CHAPTER_ABILITY_RESOLVES_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "final", "chapter", "ability", "of"]; suffix & ["resolves"]);
const DAY_NIGHT_CHANGED_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["day", "becomes", "night", "or", "night", "becomes", "day"]);
const ATTACKS_YOU_OR_PLANESWALKER_YOU_CONTROL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "or", "a", "planeswalker", "you", "control"],
            &["you", "or", "planeswalker", "you", "control"],
        ]
);
const LIBRARY_SEARCH_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["their", "library"],
            &["your", "library"],
            &["a", "library"],
        ]
);
const LIBRARY_SHUFFLE_TARGET_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["their", "library"],
            &["your", "library"],
            &["a", "library"],
            &["that", "players", "library"],
        ]
);
const SIMPLE_SPELL_ACTIVITY_OBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells", "commander"]]);
const SIMPLE_SPELL_ACTIVITY_EXCLUDED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words
        & [&[
            "during", "turn", "first", "second", "third", "fourth", "fifth", "sixth", "seventh",
            "eighth", "ninth", "tenth",
        ]]
);
const SIMPLE_SPELL_ACTIVITY_EXCLUDED_PHRASE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["other", "than"], &["from", "anywhere"]]]);
const YOU_CAST_THIS_SPELL_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["cast", "this", "spell"], &["casts", "this", "spell"]]];
    contains_words & ["you"]
);
const GIFT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "gift"], &["gift"]]);
const ACTIVATED_ABILITY_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "ability"],
            &["abilities"],
            &["an", "ability", "that", "isnt", "a", "mana", "ability"],
            &["an", "ability", "that", "isn't", "a", "mana", "ability"],
            &["abilities", "that", "arent", "mana", "abilities"],
            &["abilities", "that", "aren't", "mana", "abilities"],
        ]
);
const MANA_ABILITY_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["mana"]);

fn split_activation_cost_tap_condition_tail_lexed<'w>(
    tail_tokens: &[OwnedLexToken],
    tail_words: &'w [&'w str],
) -> (Option<bool>, Vec<OwnedLexToken>, Vec<&'w str>) {
    let Some(condition) = trigger_grammar::parse_activation_cost_tap_condition(tail_tokens) else {
        return (None, tail_tokens.to_vec(), tail_words.to_vec());
    };
    (
        Some(condition.required),
        trim_commas(&tail_tokens[..condition.condition_token]),
        tail_words[..condition.condition_word].to_vec(),
    )
}
const COMBAT_DAMAGE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["combat", "damage"]);
const THIS_LEAVES_BATTLEFIELD_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["this", "leaves", "the", "battlefield"]);
const LEAVES_BATTLEFIELD_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["leaves", "the", "battlefield"],
            &["leave", "the", "battlefield"]
        ]
);
const LEAVES_BATTLEFIELD_WITHOUT_DYING_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["leaves", "the", "battlefield", "without", "dying"],
            &["leave", "the", "battlefield", "without", "dying"]
        ]
);
const ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["enters", "or", "leaves", "the", "battlefield"],
            &["enter", "or", "leave", "the", "battlefield"],
        ]
);
const OR_IS_PUT_INTO_EXILE_FROM_BATTLEFIELD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "or",
            "is",
            "put",
            "into",
            "exile",
            "from",
            "the",
            "battlefield"
        ]
);
const OR_IS_PUT_INTO_GRAVEYARD_FROM_BATTLEFIELD_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "or",
                "is",
                "put",
                "into",
                "a",
                "graveyard",
                "from",
                "the",
                "battlefield",
            ],
            &[
                "or",
                "is",
                "put",
                "into",
                "graveyard",
                "from",
                "the",
                "battlefield",
            ],
        ]
);
const OR_TRANSFORMS_INTO_TAIL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["or", "transforms", "into"], &["or", "transform", "into"]]);
const ONE_OR_MORE_QUANTIFIER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["one", "or", "more"]);
const OTHER_OR_ANOTHER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["another"], &["other"]]);
const OTHER_OR_ANOTHER_EXACT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["another"], &["other"]]);
const UNDER_YOUR_CONTROL_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["under", "your", "control"]]);
const UNDER_OPPONENT_CONTROL_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["under", "control"]; contains_any_words & [&["opponent", "opponents"]]);
const UNTAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["untapped"]);
const TAPPED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["tapped"]);
const OPPONENT_EXPENDS_WITH_ARTICLE_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["an", "opponent", "expends"],
            &["an", "opponent", "expend"],
        ]
);
const OPPONENT_EXPENDS_TRIGGER_PREFIX: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["opponent", "expends"], &["opponent", "expend"]]);
const YOU_EXPEND_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(prefix & ["you", "expend"]);
const CYCLE_CARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "card"], &["card"]]);
const CYCLE_ANOTHER_CARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["another", "card"]);
const EXERT_CREATURE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "creature"], &["creature"]]);
const CREW_VEHICLE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "vehicle"], &["vehicle"], &["vehicles"]]);
const SADDLE_MOUNT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "mount"], &["mount"], &["mounts"]]);
const DURING_YOUR_MAIN_PHASE_SUFFIX: &[&str] = &["during", "your", "main", "phase"];
const DURING_YOUR_MAIN_PHASE_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix DURING_YOUR_MAIN_PHASE_SUFFIX);
const FROM_YOUR_HAND_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["from", "your", "hand"]);
const YOU_OPEN_ATTRACTION_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "open", "an", "attraction"],
            &["you", "opens", "an", "attraction"],
            &["you", "opened", "an", "attraction"],
        ]
);
const YOU_CLAIM_ATTRACTION_PRIZE_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "claim", "the", "prize", "of", "an", "attraction"],
            &["you", "claims", "the", "prize", "of", "an", "attraction"],
            &["you", "claimed", "the", "prize", "of", "an", "attraction"],
        ]
);
const YOU_MANIFEST_DREAD_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "manifest", "dread"]);
const EXPLOIT_CREATURE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "creature"], &["creature"]]);
const THIS_EXPLOITS_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "exploits"],
            &["this", "creature", "exploits", "a", "creature"],
            &["this", "creature", "exploits", "creature"],
            &["this", "permanent", "exploits"],
            &["this", "permanent", "exploits", "a", "creature"],
            &["this", "permanent", "exploits", "creature"],
        ]
);
const YOU_COMPLETE_DUNGEON_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "complete", "a", "dungeon"],
            &["you", "completed", "a", "dungeon"],
            &["you", "completes", "a", "dungeon"],
        ]
);
const WINS_CLASH_TRIGGER_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["win", "a", "clash"],
            &["wins", "a", "clash"],
            &["won", "a", "clash"]
        ]
);
const YOU_CLASH_AND_WIN_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "clash", "and", "win"],
            &["you", "clash", "and", "you", "win"],
        ]
);
const ATTACKS_AND_IS_NOT_BLOCKED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["and", "isnt", "blocked"],
            &["and", "isn't", "blocked"],
            &["and", "is", "not", "blocked"],
            &["and", "isnt", "blocked", "this", "combat"],
            &["and", "isn't", "blocked", "this", "combat"],
            &["and", "is", "not", "blocked", "this", "combat"],
        ]
);
const ATTACKS_A_PLAYER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["a", "player"]);
const ATTACKS_YOU_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["you"]);
const ATTACKS_OPPONENT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "opponent"],
            &["opponent"],
            &["one", "of", "your", "opponents"],
            &["another", "one", "of", "your", "opponents"],
        ]
);
const ATTACKS_DEFENDING_PLAYER_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["the", "defending", "player"], &["defending", "player"]]);
const ATTACKS_OPPONENT_OR_PLANESWALKER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "one",
                "of",
                "your",
                "opponents",
                "or",
                "a",
                "planeswalker",
                "they",
                "control",
            ],
            &[
                "one",
                "of",
                "your",
                "opponents",
                "or",
                "a",
                "planeswalker",
                "an",
                "opponent",
                "controls",
            ],
        ]
);
const ATTACKS_ENCHANTED_PLAYER_OR_PLANESWALKER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "enchanted",
                "opponent",
                "or",
                "a",
                "planeswalker",
                "they",
                "control",
            ],
            &[
                "enchanted",
                "player",
                "or",
                "a",
                "planeswalker",
                "they",
                "control",
            ],
        ]
);
const ATTACKS_PLANESWALKER_OR_BATTLE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "planeswalker"], &["a", "battle"]]);

fn parse_one_or_more_planeswalker_attack_target(
    tail: &[&str],
) -> Option<ironsmith_core::AttackTargetRestriction> {
    match tail {
        ["one", "or", "more", "planeswalkers", "you", "control"] => Some(
            ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(PlayerFilter::You),
        ),
        [
            "one",
            "or",
            "more",
            "planeswalkers",
            "an",
            "opponent",
            "controls",
        ] => Some(
            ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(
                PlayerFilter::Opponent,
            ),
        ),
        _ => None,
    }
}

/// Parse the defender-first grouped surface
/// `a planeswalker <player> controls with one or more creatures`.
///
/// The singular defender matters: the trigger fires once for each attacked
/// planeswalker, rather than once across every planeswalker that player
/// protects in the declaration.
fn parse_planeswalker_attacked_with_one_or_more_creatures_target(
    tail: &[&str],
) -> Option<ironsmith_core::AttackTargetRestriction> {
    match tail {
        [
            "a",
            "planeswalker",
            "you",
            "control",
            "with",
            "one",
            "or",
            "more",
            "creatures",
        ] => Some(
            ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(PlayerFilter::You),
        ),
        [
            "a",
            "planeswalker",
            "an",
            "opponent",
            "controls",
            "with",
            "one",
            "or",
            "more",
            "creatures",
        ] => Some(
            ironsmith_core::AttackTargetRestriction::PlaneswalkerControlledBy(
                PlayerFilter::Opponent,
            ),
        ),
        _ => None,
    }
}
const THIS_BLOCKS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "blocks"],
            &["this", "token", "blocks"],
            &["this", "blocks"],
        ]
);
const THIS_BLOCKS_OR_BECOMES_BLOCKED_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "blocks", "or", "becomes", "blocked"],
            &["this", "token", "blocks", "or", "becomes", "blocked"],
            &["this", "blocks", "or", "becomes", "blocked"],
        ]
);
const THIS_BLOCKS_OR_BECOMES_BLOCKED_BY_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "this", "creature", "blocks", "or", "becomes", "blocked", "by",
            ],
            &["this", "token", "blocks", "or", "becomes", "blocked", "by",],
            &["this", "blocks", "or", "becomes", "blocked", "by"],
        ]
);
const THIS_BECOMES_BLOCKED_BY_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "becomes", "blocked", "by"],
            &["this", "token", "becomes", "blocked", "by"],
            &["this", "becomes", "blocked", "by"],
        ]
);
const EXPLORE_LAND_CARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "land", "card"], &["land", "card"]]);
const EXPLORE_NONLAND_CARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "nonland", "card"], &["nonland", "card"]]);
const BECOMES_TAPPED_TRIGGER_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix & ["becomes", "tapped"]);
const BECOMES_MONSTROUS_TRIGGER_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix & ["becomes", "monstrous"]);
const MUTATES_TRIGGER_SUFFIX: ClauseShape<'static> = clause_shape!(suffix & ["mutates"]);
const TURNED_FACE_UP_TRIGGER_SUFFIX: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["is", "turned", "face", "up"],
            &["are", "turned", "face", "up"],
        ]
);
const SPELL_OR_ABILITY_TARGET_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "spell", "or", "ability"],
            &["spell", "or", "ability"]
        ]
);
const SPELL_OR_ABILITY_YOU_CONTROL_EXILES_PERMANENTS_FROM_BATTLEFIELD_PATTERN: ClauseShape<
    'static,
> = clause_shape!(
    exact_any
        & [
            &[
                "a",
                "spell",
                "or",
                "ability",
                "you",
                "control",
                "exiles",
                "one",
                "or",
                "more",
                "permanents",
                "from",
                "the",
                "battlefield",
            ],
            &[
                "a",
                "spell",
                "or",
                "ability",
                "you",
                "control",
                "exile",
                "one",
                "or",
                "more",
                "permanents",
                "from",
                "the",
                "battlefield",
            ],
        ]
);
const BECOMES_TARGET_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "target", "of"]);
const SPELL_OR_SPELLS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["spell"], &["spells"]]);
const ATTACK_OR_ATTACKS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attack"], &["attacks"]]);

fn parse_attacks_player_who_controls_at_least_tail(words: &[&str]) -> Option<(u32, ObjectFilter)> {
    if words.len() == 8
        && words[0] == "a"
        && words[1] == "player"
        && words[2] == "who"
        && words[3] == "controls"
        && words[5] == "or"
        && words[6] == "more"
        && matches!(words[7], "land" | "lands")
    {
        let count = parse_number_word_u32(words[4]).or_else(|| words[4].parse::<u32>().ok())?;
        return Some((count, ObjectFilter::land()));
    }
    None
}
const ALONE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["alone"]);
const WHILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["while"]);
const SADDLED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["saddled"]);
const OR_ANOTHER_WORDS: &[&str] = &["or", "another"];
const OR_ANOTHER_PATTERN: ClauseShape<'static> = clause_shape!(exact & OR_ANOTHER_WORDS);
const THIS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this"]);
const THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const LEAVES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["leaves"]);
const BATTLEFIELD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["battlefield"]);
const THE_CREATURE_HAUNTS_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "creature"]; suffix & ["haunts"]);
const INTO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["into"]);
const SPELL_NOUN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell"], &["spells"]]);
const LINKING_BE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"], &["be"], &["been"]]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const ONLY_IT_ABILITY_TARGET_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "ability", "that", "targets", "only", "it"],
            &["ability", "that", "targets", "only", "it"],
        ]
);
const BACKUP_ABILITY_TARGET_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "backup", "ability"], &["backup", "ability"]]);
const SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "is", "dealt", "damage"],
            &["this", "creature", "is", "dealt", "combat", "damage"],
            &["this", "is", "dealt", "damage"],
            &["this", "is", "dealt", "combat", "damage"],
        ]
);
const SOURCE_DEALT_COMBAT_DAMAGE_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "is", "dealt", "combat", "damage"],
            &["this", "is", "dealt", "combat", "damage"],
        ]
);
const SOURCE_DEALS_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "deals"],
            &["this", "permanent", "deals"],
            &["this", "deals"],
        ]
);
const SOURCE_DEALS_DAMAGE_TO_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "deals", "damage", "to"],
            &["this", "permanent", "deals", "damage", "to"],
            &["this", "deals", "damage", "to"],
        ]
);
const SOURCE_DEALS_DAMAGE_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "deals", "damage"],
            &["this", "permanent", "deals", "damage"],
            &["this", "deals", "damage"],
        ]
);
const DAMAGE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["damage"]);
const DEALT_DAMAGE_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["is", "dealt", "damage"],
            &["are", "dealt", "damage"],
            &["was", "dealt", "damage"],
            &["were", "dealt", "damage"],
            &["be", "dealt", "damage"],
            &["been", "dealt", "damage"],
            &["re", "dealt", "damage"],
            &["youre", "dealt", "damage"],
            &["you're", "dealt", "damage"],
        ]
);
const DEALT_COMBAT_DAMAGE_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["is", "dealt", "combat", "damage"],
            &["are", "dealt", "combat", "damage"],
            &["was", "dealt", "combat", "damage"],
            &["were", "dealt", "combat", "damage"],
            &["be", "dealt", "combat", "damage"],
            &["been", "dealt", "combat", "damage"],
            &["re", "dealt", "combat", "damage"],
            &["youre", "dealt", "combat", "damage"],
            &["you're", "dealt", "combat", "damage"],
        ]
);
const DEALT_EXCESS_NONCOMBAT_DAMAGE_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["is", "dealt", "excess", "noncombat", "damage"],
            &["are", "dealt", "excess", "noncombat", "damage"],
            &["was", "dealt", "excess", "noncombat", "damage"],
            &["were", "dealt", "excess", "noncombat", "damage"],
        ]
);
const NONCOMBAT_DAMAGE_AMOUNT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["noncombat"]);
const DURING_YOUR_TURN_TRIGGER_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix & ["during", "your", "turn"]);
const DIES_DURING_YOUR_TURN_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix & ["dies", "during", "your", "turn"]);
const DIES_THIS_TURN_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix & ["dies", "this", "turn"]);
const YOU_GAIN_LIFE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["you", "gain", "life"]);
const LOSE_LIFE_TRIGGER_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["lose", "life"], &["loses", "life"]]);
const LOSE_GAME_TRIGGER_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["lose", "the", "game"], &["loses", "the", "game"]]);
const DRAW_A_CARD_TRIGGER_SUFFIX: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["draw", "a", "card"], &["draws", "a", "card"]]);
const OPPONENT_EFFECT_DISCARDS_THIS_CARD_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "a", "spell", "or", "ability", "an", "opponent", "controls", "causes", "you", "to",
            "discard", "this", "card",
        ]
);
const THIS_WAY_REVEAL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["this", "way"]);
const SOURCE_ARTIFACT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["artifact"]);
const SOURCE_CREATURE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["creature"]);
const SOURCE_ENCHANTMENT_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["enchantment"]);
const SOURCE_LAND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["land"]);
const SOURCE_PLANESWALKER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["planeswalker"]);
const LAND_OR_LANDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["land"], &["lands"]]);

fn parse_player_or_object_damage_recipient(
    target_tokens: &[OwnedLexToken],
) -> Option<(PlayerFilter, ObjectFilter, bool)> {
    let one_or_more = has_leading_one_or_more(target_tokens);
    let union_idx = target_tokens
        .iter()
        .enumerate()
        .find_map(|(index, token)| {
            if token.is_word("and/or") {
                return Some((index, true));
            }
            if !token.is_word("or") {
                return None;
            }
            let belongs_to_one_or_more = index > 0
                && target_tokens[index - 1].is_word("one")
                && target_tokens
                    .get(index + 1)
                    .is_some_and(|next| next.is_word("more"));
            (!belongs_to_one_or_more).then_some((index, false))
        })?;
    let (union_idx, is_and_or) = union_idx;
    let left_tokens = trim_commas(&target_tokens[..union_idx]);
    let right_tokens = trim_commas(&target_tokens[union_idx + 1..]);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return None;
    }

    let left_words = ActivationRestrictionCompatWords::new(&left_tokens).to_word_refs();
    if let Some(player) = parse_trigger_subject_player_filter(&left_words)
        && let Ok(mut filter) =
            parse_object_filter_lexed(strip_leading_one_or_more_lexed(&right_tokens), false)
    {
        if is_and_or {
            filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        }
        filter.set_union_one_or_more(one_or_more);
        return Some((player, filter, true));
    }

    let right_words = ActivationRestrictionCompatWords::new(&right_tokens).to_word_refs();
    if let Some(player) = parse_trigger_subject_player_filter(&right_words)
        && let Ok(mut filter) =
            parse_object_filter_lexed(strip_leading_one_or_more_lexed(&left_tokens), false)
    {
        if is_and_or {
            filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
        }
        filter.set_union_one_or_more(one_or_more);
        return Some((player, filter, false));
    }

    None
}

fn parse_damage_source_trigger_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, crate::triggers::DamageSourceSurface)>, CardTextError> {
    let one_or_more = has_leading_one_or_more(subject_tokens);
    let source_surface =
        crate::grammar::trigger_subjects::parse_damage_source_surface(subject_tokens);
    let Some(mut filter) = parse_trigger_subject_filter_lexed(subject_tokens)? else {
        return Ok(None);
    };
    if source_surface == crate::triggers::DamageSourceSurface::Source {
        // "Source" is a game-object domain, not a synonym for a battlefield
        // permanent. Keep parsed qualities such as color and controller while
        // allowing damage sources from any appropriate zone.
        filter.zone = None;
    }
    if ActivationRestrictionCompatWords::new(subject_tokens)
        .to_word_refs()
        .first()
        == Some(&"another")
    {
        filter.other = true;
    }
    filter.set_union_one_or_more(one_or_more);
    Ok(Some((filter, source_surface)))
}

fn preserve_trigger_filter_union_surface(
    filter: &mut ObjectFilter,
    subject_tokens: &[OwnedLexToken],
) {
    if subject_tokens.iter().any(|token| token.is_word("and/or")) {
        filter.set_union_connective(crate::filter::ObjectFilterUnionConnective::AndOr);
    }
}
const BECOMES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["becomes"]);
const COMBAT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["combat"]);
const YOU_CONTRACTION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["youre"], &["you're"]]);
const ONE_OR_MORE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["one", "or", "more"]);
const CARD_OR_CARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["card", "cards"]]);
const PERMANENT_OR_PERMANENTS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["permanent", "permanents"]]);
const ATTACHED_OBJECT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["enchanted"], &["equipped"]]);
const PLAYER_GETS_ONE_OR_MORE_ENERGY_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["one", "or", "more", "e"]);
const COUNTER_RECIPIENT_PREPOSITION_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["on"], &["onto"]]);
const PASSIVE_COUNTER_PUT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["counter", "is", "put", "on"],
            &["counter", "is", "put", "onto"],
            &["counters", "are", "put", "on"],
            &["counters", "are", "put", "onto"],
        ]]
);
const ONE_OR_MORE_COUNTERS_REMOVED_FROM_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["one", "or", "more", "counters", "are", "removed", "from"]);
const A_COUNTER_REMOVED_FROM_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["a", "counter", "is", "removed", "from"]);
const THIS_WAY_EXACT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "way"]);
const SHARED_SUBJECT_ETB_OR_COMBAT_DAMAGE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "battlefield", "or", "deal", "combat", "damage"],
            &["the", "battlefield", "or", "deals", "combat", "damage"],
        ]
);
const SHARED_SUBJECT_ETB_OR_ATTACK_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["or", "attack"],
            &["or", "attacks"],
            &["the", "battlefield", "or", "attack"],
            &["the", "battlefield", "or", "attacks"],
        ]
);
const SOURCE_KEYWORD_ACTION_TRAILING_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["become"],
            &["becomes"],
            &["became"],
            &["becoming"],
            &["has"],
            &["had"],
        ]
);
#[derive(Debug, Clone, Copy)]
struct TriggerSuffixShape {
    shape: ClauseShape<'static>,
    word_len: usize,
}

const fn trigger_suffix_shape(shape: ClauseShape<'static>, word_len: usize) -> TriggerSuffixShape {
    TriggerSuffixShape { shape, word_len }
}

fn token_trigger_pattern_accepts(tokens: &[OwnedLexToken], shape: &ClauseShape<'static>) -> bool {
    trigger_grammar::parse_trigger_surface_tokens(tokens, *shape)
}

fn trigger_pattern_accepts(words: &[&str], shape: ClauseShape<'static>) -> bool {
    trigger_grammar::parse_trigger_surface_words(words, shape)
}

fn trigger_word_accepts_pattern(word: &str, shape: ClauseShape<'static>) -> bool {
    trigger_pattern_accepts(&[word], shape)
}

fn token_matches_clause_shape(token: &OwnedLexToken, shape: ClauseShape<'static>) -> bool {
    token
        .as_word()
        .is_some_and(|word| trigger_word_accepts_pattern(word, shape))
}

fn trigger_word_at_accepts_pattern(
    words: &[&str],
    idx: usize,
    shape: ClauseShape<'static>,
) -> bool {
    words
        .get(idx)
        .is_some_and(|word| trigger_word_accepts_pattern(word, shape))
}

fn trigger_atom_token(tokens: &[OwnedLexToken], atom: TriggerClauseAtom) -> Option<usize> {
    trigger_grammar::parse_trigger_clause_atom_token(tokens, atom)
}

fn trigger_atom_word(words: &[&str], atom: TriggerClauseAtom) -> Option<usize> {
    trigger_grammar::parse_trigger_clause_atom_word(words, atom)
}

fn trigger_token_is_atom(token: &OwnedLexToken, atom: TriggerClauseAtom) -> bool {
    trigger_atom_token(std::slice::from_ref(token), atom) == Some(0)
}

fn trigger_keyword_action_word(
    words: &[&str],
    action: crate::events::KeywordActionKind,
) -> Option<usize> {
    trigger_grammar::parse_trigger_keyword_action_word(words, action)
}

fn parse_unpaid_cumulative_upkeep_player(words: &[&str]) -> Option<PlayerFilter> {
    if !crate::word_primitives::parse_sequence_suffix(words, &["cumulative", "upkeep"]) {
        return None;
    }
    let pay_word = crate::word_primitives::parse_sequence_start(words, &["pay"])?;
    let actor_end = match words.get(..pay_word)? {
        prefix
            if crate::word_primitives::parse_any_sequence_suffix(
                prefix,
                &[&["doesnt"], &["dont"]],
            ) =>
        {
            pay_word.checked_sub(1)?
        }
        prefix
            if crate::word_primitives::parse_any_sequence_suffix(
                prefix,
                &[&["does", "not"], &["do", "not"]],
            ) =>
        {
            pay_word.checked_sub(2)?
        }
        _ => return None,
    };
    let player = parse_trigger_subject_player_filter(words.get(..actor_end)?)?;
    let source_reference = words.get(pay_word + 1..words.len().checked_sub(2)?)?;
    let source_reference = crate::util::possessive_normalized_word_refs(source_reference);
    crate::util::source_reference_surface_for_possessive_words(&source_reference)?;
    Some(player)
}

fn trigger_word_token_start(tokens: &[OwnedLexToken], word_idx: usize) -> Option<usize> {
    trigger_grammar::parse_trigger_word_span_tokens(tokens, word_idx).map(|span| span.first)
}

fn find_phrase_shape(
    words: &[&str],
    phrase_len: usize,
    shape: ClauseShape<'static>,
) -> Option<usize> {
    trigger_grammar::find_trigger_surface_window(words, phrase_len, shape)
}

fn subject_starts_one_or_more(words: &[&str]) -> bool {
    trigger_pattern_accepts(words, ONE_OR_MORE_PREFIX_PATTERN)
}

fn subject_is_card_or_cards(words: &[&str]) -> bool {
    trigger_pattern_accepts(words, CARD_OR_CARDS_PATTERN)
}

fn subject_mentions_card(words: &[&str]) -> bool {
    trigger_pattern_accepts(words, CARD_OR_CARDS_WORD_PATTERN)
}

fn subject_mentions_permanent(words: &[&str]) -> bool {
    trigger_pattern_accepts(words, PERMANENT_OR_PERMANENTS_WORD_PATTERN)
}

const PUT_INTO_YOUR_GRAVEYARD_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(suffix & ["is", "put", "into", "your", "graveyard", "from", "anywhere"]),
        7,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "your",
                    "graveyard",
                    "from",
                    "anywhere"
                ]
        ),
        7,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["is", "put", "into", "your", "graveyard"]),
        5,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["are", "put", "into", "your", "graveyard"]),
        5,
    ),
];
const PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(suffix & ["is", "put", "into", "a", "graveyard", "from", "anywhere"]),
        7,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["are", "put", "into", "a", "graveyard", "from", "anywhere"]),
        7,
    ),
];
const PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE_EXCEPT_BATTLEFIELD_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "a",
                    "graveyard",
                    "from",
                    "anywhere",
                    "other",
                    "than",
                    "the",
                    "battlefield",
                ]
        ),
        11,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "a",
                    "graveyard",
                    "from",
                    "anywhere",
                    "other",
                    "than",
                    "the",
                    "battlefield",
                ]
        ),
        11,
    ),
];
const PUT_INTO_OPPONENT_GRAVEYARD_FROM_ANYWHERE_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "an",
                    "opponents",
                    "graveyard",
                    "from",
                    "anywhere",
                ]
        ),
        8,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "an",
                    "opponents",
                    "graveyard",
                    "from",
                    "anywhere",
                ]
        ),
        8,
    ),
];
const ATTACHED_OBJECT_PUT_INTO_GRAVEYARD_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(suffix & ["is", "put", "into", "graveyard"]),
        4,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["is", "put", "into", "a", "graveyard"]),
        5,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["are", "put", "into", "graveyard"]),
        4,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["are", "put", "into", "a", "graveyard"]),
        5,
    ),
];
const PUT_INTO_YOUR_GRAVEYARD_FROM_LIBRARY_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "your",
                    "graveyard",
                    "from",
                    "your",
                    "library",
                ]
        ),
        8,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "your",
                    "graveyard",
                    "from",
                    "your",
                    "library",
                ]
        ),
        8,
    ),
];
const PUT_INTO_YOUR_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "your",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        8,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "your",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        8,
    ),
];
const PUT_INTO_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(suffix & ["is", "put", "into", "graveyard", "from", "battlefield"]),
        6,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        7,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["is", "put", "into", "a", "graveyard", "from", "battlefield",]),
        7,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "a",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        8,
    ),
    trigger_suffix_shape(
        clause_shape!(suffix & ["are", "put", "into", "graveyard", "from", "battlefield"]),
        6,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        7,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "a",
                    "graveyard",
                    "from",
                    "battlefield",
                ]
        ),
        7,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "a",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        8,
    ),
];
const PUT_INTO_GRAVEYARD_OR_EXILE_FROM_BATTLEFIELD_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "a",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                    "or",
                    "is",
                    "put",
                    "into",
                    "exile",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        16,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                    "or",
                    "is",
                    "put",
                    "into",
                    "exile",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        15,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "a",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                    "or",
                    "are",
                    "put",
                    "into",
                    "exile",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        16,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                    "or",
                    "are",
                    "put",
                    "into",
                    "exile",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        15,
    ),
];
const PUT_INTO_OPPONENT_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES: &[TriggerSuffixShape] = &[
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "is",
                    "put",
                    "into",
                    "an",
                    "opponents",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        8,
    ),
    trigger_suffix_shape(
        clause_shape!(
            suffix
                & [
                    "are",
                    "put",
                    "into",
                    "an",
                    "opponents",
                    "graveyard",
                    "from",
                    "the",
                    "battlefield",
                ]
        ),
        8,
    ),
];

fn trigger_suffix_word_len(words: &[&str], suffixes: &[TriggerSuffixShape]) -> Option<usize> {
    suffixes
        .iter()
        .find(|suffix| trigger_pattern_accepts(words, suffix.shape))
        .map(|suffix| suffix.word_len)
}

fn trigger_subject_tokens_before_suffix(
    tokens: &[OwnedLexToken],
    total_word_len: usize,
    suffix_word_len: usize,
) -> &[OwnedLexToken] {
    let span =
        trigger_grammar::parse_subject_before_suffix_span(tokens, total_word_len, suffix_word_len);
    &tokens[span.first..span.end]
}

fn trigger_counter_descriptor_span<'a>(
    tokens: &'a [OwnedLexToken],
    start_word_idx: usize,
    counter_word_idx: usize,
    words: &[&str],
) -> Result<(&'a [OwnedLexToken], &'a [OwnedLexToken]), CardTextError> {
    let spans =
        trigger_grammar::parse_counter_descriptor_spans(tokens, start_word_idx, counter_word_idx)
            .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter descriptor in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    Ok((&tokens[spans.descriptor], &tokens[spans.with_counter]))
}

fn trigger_counter_type_from_descriptor(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    trigger_grammar::parse_trigger_counter_type(tokens)
}

fn trigger_counter_recipient_tokens(
    tokens: &[OwnedLexToken],
    object_word_start: usize,
    words: &[&str],
) -> Result<Vec<OwnedLexToken>, CardTextError> {
    let recipient = trigger_grammar::parse_counter_recipient_span(tokens, object_word_start)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter recipient in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    Ok(tokens[recipient.tokens].to_vec())
}

fn dealt_damage_suffix_subject_word_idx(words: &[&str]) -> Option<(usize, bool)> {
    if trigger_pattern_accepts(words, DEALT_COMBAT_DAMAGE_SUFFIX_PATTERN) {
        return Some((words.len().saturating_sub(4), true));
    }
    if trigger_pattern_accepts(words, DEALT_DAMAGE_SUFFIX_PATTERN) {
        return Some((words.len().saturating_sub(3), false));
    }
    None
}

fn dealt_excess_noncombat_damage_subject_word_idx(words: &[&str]) -> Option<usize> {
    trigger_pattern_accepts(words, DEALT_EXCESS_NONCOMBAT_DAMAGE_SUFFIX_PATTERN)
        .then(|| words.len().saturating_sub(5))
}

fn passive_damage_by_word_span(words: &[&str]) -> Option<(usize, usize)> {
    words.iter().enumerate().find_map(|(index, word)| {
        (matches!(*word, "is" | "are" | "was" | "were")
            && words.get(index + 1) == Some(&"dealt")
            && words.get(index + 2) == Some(&"damage")
            && words.get(index + 3) == Some(&"by"))
        .then_some((index, index + 4))
    })
}

fn parse_put_into_your_graveyard_from_exact_zone(
    tokens: &[OwnedLexToken],
    words: &[&str],
    suffixes: &[TriggerSuffixShape],
    from: Zone,
) -> Result<Option<TriggerSpec>, CardTextError> {
    let Some(suffix_word_len) = trigger_suffix_word_len(words, suffixes) else {
        return Ok(None);
    };
    let subject_tokens = trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
    let one_or_more = has_leading_one_or_more(subject_tokens);
    let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_view.to_word_refs();
    let mut filter = if is_source_reference_words(&subject_words) {
        source_reference_surface_for_trigger_subject(subject_tokens)
            .map(ObjectFilter::source_with_surface)
            .unwrap_or_else(ObjectFilter::source)
    } else {
        parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported card filter in put-into-your-graveyard-from-zone trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?
    };
    filter.zone = None;
    filter.controller = None;
    if filter.owner.is_none() {
        filter.owner = Some(PlayerFilter::You);
    }
    if subject_mentions_card(&subject_words) {
        filter.nontoken = true;
        filter.set_explicit_card_noun(true);
    }
    Ok(Some(TriggerSpec::PutIntoGraveyardFromZone {
        filter,
        from,
        one_or_more,
    }))
}

fn parse_passive_damage_source_filter(
    source_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<ObjectFilter, CardTextError> {
    let source_view = ActivationRestrictionCompatWords::new(source_tokens);
    let source_words = source_view.to_word_refs();
    let repeated_by_connectors = source_words
        .iter()
        .zip(source_words.iter().skip(1))
        .enumerate()
        .filter_map(|(index, (left, right))| (*left == "or" && *right == "by").then_some(index))
        .collect::<Vec<_>>();

    if repeated_by_connectors.is_empty() {
        return parse_object_filter_lexed(source_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported passive damage source filter in trigger clause (clause: '{}')",
                clause_words.join(" ")
            ))
        });
    }

    let mut branches = Vec::new();
    let mut branch_word_start = 0;
    for connector_word in repeated_by_connectors
        .iter()
        .copied()
        .chain(std::iter::once(source_words.len()))
    {
        let branch_token_start = trigger_word_token_start(source_tokens, branch_word_start)
            .unwrap_or(source_tokens.len());
        let branch_token_end =
            trigger_word_token_start(source_tokens, connector_word).unwrap_or(source_tokens.len());
        let branch_tokens =
            trim_edge_punctuation_tokens(&source_tokens[branch_token_start..branch_token_end]);
        if branch_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "empty passive damage source branch in trigger clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        branches.push(
            parse_object_filter_lexed(branch_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported passive damage source branch in trigger clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?,
        );
        branch_word_start = connector_word.saturating_add(2);
    }

    let mut source = ObjectFilter::default();
    source.any_of = branches;
    source.set_union_connective(crate::filter::ObjectFilterUnionConnective::Or);
    Ok(source)
}

pub fn strip_leading_trigger_intro(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if token_slice_at_is_any(tokens, 0, &["when", "whenever", "at"]) {
        &tokens[1..]
    } else {
        tokens
    }
}

fn leading_trigger_intro_surface(tokens: &[OwnedLexToken]) -> Option<TriggerIntroSurfaceAst> {
    if token_slice_at_is_any(tokens, 0, &["when"]) {
        Some(TriggerIntroSurfaceAst::When)
    } else if token_slice_at_is_any(tokens, 0, &["whenever"]) {
        Some(TriggerIntroSurfaceAst::Whenever)
    } else if token_slice_at_is_any(tokens, 0, &["at"]) {
        Some(TriggerIntroSurfaceAst::At)
    } else {
        None
    }
}

fn apply_leading_trigger_intro_surface(
    trigger: TriggerSpec,
    tokens: &[OwnedLexToken],
) -> TriggerSpec {
    let Some(intro) = leading_trigger_intro_surface(tokens) else {
        return trigger;
    };
    TriggerSpec::WithIntro {
        intro,
        trigger: Box::new(trigger),
    }
}

fn source_reference_surface_for_trigger_subject(
    tokens: &[OwnedLexToken],
) -> Option<crate::target::SourceReferenceSurface> {
    let tokens = if leading_trigger_intro_surface(tokens).is_some() {
        &tokens[1..]
    } else {
        tokens
    };
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let subject_words = non_article_word_refs(&word_view.to_word_refs());
    source_reference_surface_for_words(&subject_words)
        .or_else(|| this_source_surface_for_words(&subject_words))
}

fn parse_source_or_another_trigger_subject_filters(
    subject_tokens: &[OwnedLexToken],
) -> Option<(ObjectFilter, ObjectFilter)> {
    let word_view = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = word_view.to_word_refs();
    let shape = crate::grammar::trigger_subjects::parse_source_or_another_shape(&subject_words)?;
    let source_words = &subject_words[..shape.source_word_end];
    if !is_source_reference_words(source_words) {
        return None;
    }

    let source_token_end = trigger_word_token_start(subject_tokens, shape.source_word_end)?;
    let another_span = crate::grammar::trigger_subjects::parse_trigger_word_span(
        subject_tokens,
        shape.other_word,
    )?;
    let other_tokens = trim_edge_punctuation(&subject_tokens[another_span.end..]);
    if other_tokens.is_empty() {
        return None;
    }

    let source_filter =
        source_reference_surface_for_trigger_subject(&subject_tokens[..source_token_end])
            .map(ObjectFilter::source_with_surface)
            .unwrap_or_else(ObjectFilter::source);
    let other_filter = parse_object_filter_lexed(&other_tokens, true).ok()?;
    Some((source_filter, other_filter))
}

fn this_enters_battlefield_trigger_spec(
    surface: Option<crate::target::SourceReferenceSurface>,
    subject_number: ironsmith_core::trigger_model::TriggerSubjectNumber,
    origin_condition: Option<ironsmith_core::trigger_model::ZoneChangeOriginCondition>,
) -> TriggerSpec {
    match surface {
        Some(surface) => TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface,
            subject_number,
            origin_condition,
        },
        None => TriggerSpec::ThisEntersBattlefield { origin_condition },
    }
}

fn enter_trigger_subject_number(
    enter_word: &str,
) -> ironsmith_core::trigger_model::TriggerSubjectNumber {
    if enter_word == "enter" {
        ironsmith_core::trigger_model::TriggerSubjectNumber::Plural
    } else {
        ironsmith_core::trigger_model::TriggerSubjectNumber::Singular
    }
}

fn this_leaves_battlefield_trigger_spec(
    surface: Option<crate::target::SourceReferenceSurface>,
) -> TriggerSpec {
    match surface {
        Some(surface) => TriggerSpec::ThisLeavesBattlefieldWithSurface(surface),
        None => TriggerSpec::ThisLeavesBattlefield,
    }
}

fn this_transforms_trigger_spec(
    surface: Option<crate::target::SourceReferenceSurface>,
    destination_name: Option<String>,
) -> TriggerSpec {
    match surface {
        Some(surface) => TriggerSpec::ThisTransformsWithSurface {
            surface,
            destination_name,
        },
        None => TriggerSpec::ThisTransforms { destination_name },
    }
}

fn trigger_destination_name_from_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    let destination_words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
    if trigger_pattern_accepts(&destination_words, THIS_DESTINATION_TRIGGER_NAME_PATTERN) {
        return None;
    }

    let mut out = String::new();
    for token in tokens {
        if token.is_comma() {
            out.push(',');
            continue;
        }
        if token.as_word().is_none() {
            continue;
        }
        if !out.is_empty() && !crate::string_primitives::ends_with_char(&out, ' ') {
            out.push(' ');
        }
        out.push_str(token.slice.as_str());
    }
    let trimmed = out.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn transform_destination_name_after_into(
    _word_view: &ActivationRestrictionCompatWords<'_>,
    transforms_word_idx: usize,
    tokens: &[OwnedLexToken],
) -> Option<String> {
    let span = trigger_grammar::parse_transform_destination_span(tokens, transforms_word_idx)?;
    trigger_destination_name_from_tokens(&tokens[span.first..])
}

pub fn split_trigger_or_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    trigger_grammar::parse_trigger_or_split(tokens).map(|split| split.separator)
}

pub fn has_leading_one_or_more(tokens: &[OwnedLexToken]) -> bool {
    leading_one_or_more_prefix_len(tokens).is_some()
}

pub fn leading_one_or_more_prefix_len(tokens: &[OwnedLexToken]) -> Option<usize> {
    let (count, used) =
        parse_greater_than_or_equal_quantity_prefix(tokens, false, false, "trigger subject")
            .ok()
            .flatten()?;
    (count == 1).then_some(used)
}

pub fn parse_leading_or_more_quantifier(
    tokens: &[OwnedLexToken],
) -> Option<(u32, &[OwnedLexToken])> {
    let (count, used) =
        parse_greater_than_or_equal_quantity_prefix(tokens, false, false, "trigger quantifier")
            .ok()
            .flatten()?;
    Some((count, &tokens[used..]))
}

pub fn parse_leading_exactly_quantifier(
    tokens: &[OwnedLexToken],
) -> Option<(u32, &[OwnedLexToken])> {
    if !tokens.first()?.is_word("exactly") {
        return None;
    }
    let (count, used) = parse_number(&tokens[1..])?;
    Some((count, &tokens[1 + used..]))
}

/// Whether an intervening-"if" clause body (without the leading "if") is a
/// moved-or-cast origin condition ("it entered from your graveyard or you
/// cast it from your graveyard"). Such clauses scope the trigger event itself
/// and must stay with the trigger instead of becoming a standalone predicate.
pub fn clause_words_are_moved_or_cast_origin_condition(words: &[&str]) -> bool {
    let mut prefixed = Vec::with_capacity(words.len() + 1);
    prefixed.push("if");
    prefixed.extend_from_slice(words);
    parse_moved_or_cast_origin_condition(&prefixed).is_some()
}

/// Whether a parsed trigger spec carries a moved-or-cast origin condition
/// (possibly nested under an intro surface or an either-union).
pub fn trigger_spec_has_moved_or_cast_origin_condition(trigger: &TriggerSpec) -> bool {
    use ironsmith_core::trigger_model::ZoneChangeOriginCondition;
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            trigger_spec_has_moved_or_cast_origin_condition(trigger)
        }
        TriggerSpec::Either(left, right) => {
            trigger_spec_has_moved_or_cast_origin_condition(left)
                || trigger_spec_has_moved_or_cast_origin_condition(right)
        }
        TriggerSpec::EntersBattlefieldOneOrMore {
            origin_condition: Some(ZoneChangeOriginCondition::MovedFromOrCastFrom { .. }),
            ..
        }
        | TriggerSpec::EntersBattlefield {
            origin_condition: Some(ZoneChangeOriginCondition::MovedFromOrCastFrom { .. }),
            ..
        }
        | TriggerSpec::ThisEntersBattlefield {
            origin_condition: Some(ZoneChangeOriginCondition::MovedFromOrCastFrom { .. }),
        }
        | TriggerSpec::ThisEntersBattlefieldWithSurface {
            origin_condition: Some(ZoneChangeOriginCondition::MovedFromOrCastFrom { .. }),
            ..
        } => true,
        _ => false,
    }
}

fn parse_moved_or_cast_origin_condition(
    words: &[&str],
) -> Option<ironsmith_core::trigger_model::ZoneChangeOriginCondition> {
    use ironsmith_core::trigger_model::OriginConditionSubjectSurface;

    // Older wordings keep "the battlefield" between the enters verb and the
    // origin clause ("enters the battlefield, if it entered from ...").
    let initial_offset = usize::from(crate::word_primitives::parse_sequence_prefix(
        words,
        &["the", "battlefield"],
    )) * 2;
    let words = words.get(initial_offset..)?;

    let (subject_surface, origin_words) = if crate::word_primitives::parse_sequence_prefix(
        words,
        &["if", "one", "or", "more", "of", "them", "entered"],
    ) {
        (OriginConditionSubjectSurface::It, words.get(7..)?)
    } else if crate::word_primitives::parse_sequence_prefix(words, &["if", "it", "entered"]) {
        (OriginConditionSubjectSurface::It, words.get(3..)?)
    } else if words.get(..2) == Some(&["if", "that"][..])
        && matches!(
            words.get(2),
            Some(&"creature" | &"object" | &"permanent" | &"card" | &"token")
        )
        && words.get(3) == Some(&"entered")
    {
        (
            OriginConditionSubjectSurface::That(format!("that {}", words[2])),
            &words[4..],
        )
    } else {
        return None;
    };

    let separator = crate::word_primitives::parse_sequence_start(origin_words, &["or"])?;
    let moved_origin =
        trigger_grammar::parse_enters_origin_clause_words(origin_words.get(..separator)?)?;
    let cast_words = origin_words.get(separator + 1..)?;
    let (caster, cast_origin_words) = if cast_words.get(..2) == Some(&["was", "cast"][..])
        || cast_words.get(..2) == Some(&["were", "cast"][..])
    {
        (None, cast_words.get(2..)?)
    } else if cast_words.get(..3) == Some(&["you", "cast", "it"][..])
        || cast_words.get(..3) == Some(&["you", "cast", "them"][..])
    {
        (Some(PlayerFilter::You), cast_words.get(3..)?)
    } else {
        return None;
    };
    let cast_origin = trigger_grammar::parse_enters_origin_clause_words(cast_origin_words)?;
    (moved_origin.zone == cast_origin.zone && moved_origin.owner == cast_origin.owner).then_some(
        ironsmith_core::trigger_model::ZoneChangeOriginCondition::MovedFromOrCastFrom {
            zone: moved_origin.zone,
            zone_owner: moved_origin.owner,
            caster,
            subject_surface,
        },
    )
}

pub fn parse_trigger_clause_lexed(tokens: &[OwnedLexToken]) -> Result<TriggerSpec, CardTextError> {
    // `... while <condition>` qualifies the event itself. Preserve it as
    // a typed matcher wrapper before union parsing or the broad attack/
    // cast routes can accept only the event prefix and silently discard
    // the board-state requirement. Recursive parsing is safe because the
    // left slice no longer contains the `while` separator.
    if let Some(while_idx) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("while"))
        && while_idx > 0
        && while_idx + 1 < tokens.len()
    {
        let trigger_tokens = trim_edge_punctuation(&tokens[..while_idx]);
        let condition_tokens = trim_edge_punctuation(&tokens[while_idx + 1..]);
        let trigger = parse_trigger_clause_lexed(&trigger_tokens)?;
        let condition = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
            &condition_tokens,
        )?;
        return Ok(TriggerSpec::ConditionQualified {
            trigger: Box::new(trigger),
            condition,
            surface: crate::lexer::render_token_slice(&condition_tokens)
                .trim()
                .trim_end_matches('.')
                .to_string(),
        });
    }
    if let Some(trigger) = try_parse_passive_sacrificed_or_destroyed_lexed(tokens)? {
        return Ok(trigger);
    }
    if let Some(trigger) = try_parse_player_attack_with_aggregate_lexed(tokens)? {
        return Ok(trigger);
    }
    if let Some(trigger) = try_parse_player_puts_object_onto_battlefield_lexed(tokens)? {
        return Ok(trigger);
    }
    if let Some(trigger) = try_parse_player_attack_with_one_or_more_lexed(tokens)? {
        return Ok(trigger);
    }
    if let Some(trigger) = try_parse_source_and_another_attack_different_players(tokens) {
        return Ok(trigger);
    }
    if let Some(union) = try_parse_shared_player_attack_draw_cast_union_lexed(tokens)? {
        return Ok(union);
    }
    if let Some(union) = try_parse_repeated_intro_attack_union_lexed(tokens) {
        return Ok(union);
    }
    if let Some(union) = try_parse_trigger_union_lexed(tokens) {
        return Ok(union);
    }
    parse_trigger_clause_lexed_unstacked(tokens)
}

pub fn parse_trigger_clause_lexed_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    let authored_surface = crate::util::authored_named_source_reference_surface(context, tokens);
    let normalized = crate::util::normalize_source_reference_tokens_with_context(context, tokens)?;
    let mut trigger = parse_trigger_clause_lexed(&normalized)?;
    if let Some(surface) = authored_surface {
        restore_authored_source_trigger_surface(&mut trigger, &surface);
    }
    Ok(trigger)
}

pub(crate) fn restore_authored_source_trigger_surface(
    trigger: &mut TriggerSpec,
    surface: &crate::target::SourceReferenceSurface,
) {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. }
        | TriggerSpec::ConditionQualified { trigger, .. } => {
            restore_authored_source_trigger_surface(trigger, surface);
        }
        TriggerSpec::AnyOf(branches) => {
            for branch in branches {
                restore_authored_source_trigger_surface(branch, surface);
            }
        }
        TriggerSpec::Either(left, right) => {
            restore_authored_source_trigger_surface(left, surface);
            restore_authored_source_trigger_surface(right, surface);
        }
        TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface: current, ..
        }
        | TriggerSpec::ThisTransformsWithSurface {
            surface: current, ..
        } => *current = surface.clone(),
        TriggerSpec::ThisDies => {
            *trigger = TriggerSpec::Dies(ObjectFilter::source_with_surface(surface.clone()));
        }
        TriggerSpec::ThisLeavesBattlefieldWithSurface(current)
        | TriggerSpec::ThisDiesOrIsExiledWithSurface(current) => *current = surface.clone(),
        TriggerSpec::ThisDealsCombatDamageToPlayer {
            source_surface: current,
            ..
        } => *current = Some(surface.clone()),
        TriggerSpec::ThisAttacks => {
            *trigger = TriggerSpec::Attacks(ObjectFilter::source_with_surface(surface.clone()));
        }
        _ => {}
    }
}

/// Parse the causative ETB surface `a player puts <object> onto the battlefield`.
///
/// The player named by this wording is the entering object's controller after
/// the zone change. Lower this to the ordinary typed ETB trigger so the
/// trigger's existing player-reference export binds later `that player` and
/// `they` clauses to the triggering object's controller. Without this
/// preemption, the broad damage-trigger fallback can consume words from the
/// resolution clause and leave the payment action detached from its trigger.
fn try_parse_player_puts_object_onto_battlefield_lexed(
    raw_tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(strip_leading_trigger_intro(raw_tokens));
    let words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
    let Some(put_word) =
        crate::slice_primitives::select_position(&words, |word| matches!(*word, "put" | "puts"))
    else {
        return Ok(None);
    };
    let Some(player) = parse_trigger_subject_player_filter(&words[..put_word]) else {
        return Ok(None);
    };
    if player != PlayerFilter::Any || words.len() < put_word + 5 {
        return Ok(None);
    }
    let battlefield_suffix = ["onto", "the", "battlefield"];
    if !crate::word_primitives::parse_sequence_suffix(&words, &battlefield_suffix) {
        return Ok(None);
    }
    let subject_end_word = words.len() - battlefield_suffix.len();
    if subject_end_word <= put_word + 1 {
        return Ok(None);
    }
    let Some(subject_start_token) = trigger_word_token_start(tokens, put_word + 1) else {
        return Ok(None);
    };
    let Some(subject_end_token) = trigger_word_token_start(tokens, subject_end_word) else {
        return Ok(None);
    };
    let subject_tokens =
        trim_edge_punctuation_tokens(&tokens[subject_start_token..subject_end_token]);
    let mut filter = parse_object_filter_lexed(subject_tokens, false)?;
    filter.set_player_puts_onto_battlefield_surface(true);
    Ok(Some(apply_leading_trigger_intro_surface(
        TriggerSpec::EntersBattlefield {
            filter,
            cause_filter: None,
            origin_condition: None,
            during_turn: None,
        },
        raw_tokens,
    )))
}

/// Parse a comparison over the attacking group rather than each attacker.
/// `creatures with total power 12 or greater` must not become a creature
/// filter requiring every attacker to have power 12 or greater.
fn try_parse_player_attack_with_aggregate_lexed(
    raw_tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(strip_leading_trigger_intro(raw_tokens));
    let words = crate::lexer::token_word_refs(tokens);
    let Some(attack_word) = crate::slice_primitives::select_position(&words, |word| {
        matches!(*word, "attack" | "attacks")
    }) else {
        return Ok(None);
    };
    if words.get(attack_word + 1) != Some(&"with") {
        return Ok(None);
    }
    let Some(player) = parse_trigger_subject_player_filter(&words[..attack_word]) else {
        return Ok(None);
    };
    let Some(metric_word) =
        crate::word_primitives::parse_last_sequence_start(&words, &["with", "total", "power"])
    else {
        return Ok(None);
    };
    if metric_word <= attack_word + 2 || metric_word + 3 >= words.len() {
        return Ok(None);
    }
    let comparison_tail = &words[metric_word + 3..];
    let Some((comparison, consumed)) =
        crate::grammar::shared_util::value_semantics::parse_filter_comparison_tokens(
            "power",
            comparison_tail,
            &words,
        )?
    else {
        return Ok(None);
    };
    if consumed != comparison_tail.len() {
        return Ok(None);
    }
    let Some(subject_start) = trigger_word_token_start(tokens, attack_word + 2) else {
        return Ok(None);
    };
    let Some(subject_end) = trigger_word_token_start(tokens, metric_word) else {
        return Ok(None);
    };
    let subject_tokens = trim_edge_punctuation_tokens(&tokens[subject_start..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter_lexed(subject_tokens, false)?;
    if filter.controller.is_none() {
        filter.controller = Some(player);
    }
    filter.set_union_one_or_more(true);
    Ok(Some(TriggerSpec::AttacksOneOrMoreWithAggregate {
        filter,
        metric: crate::effect::ChoiceAggregateMetric::Power,
        comparison,
    }))
}

fn try_parse_passive_sacrificed_or_destroyed_lexed(
    raw_tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(strip_leading_trigger_intro(raw_tokens));
    let words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
    if words.len() < 6
        || !matches!(words.first(), Some(&"a" | &"an"))
        || !crate::word_primitives::parse_sequence_suffix(
            &words,
            &["is", "sacrificed", "or", "destroyed"],
        )
    {
        return Ok(None);
    }
    let Some(is_token) =
        crate::slice_primitives::select_position(tokens, |token| token.is_word("is"))
    else {
        return Ok(None);
    };
    if is_token <= 1 {
        return Ok(None);
    }
    let filter = parse_object_filter_lexed(&tokens[1..is_token], false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported passive sacrifice-or-destroy trigger filter (clause: '{}')",
            words.join(" ")
        ))
    })?;
    Ok(Some(apply_leading_trigger_intro_surface(
        TriggerSpec::AnyOf(vec![
            TriggerSpec::PermanentSacrificed(filter.clone()),
            TriggerSpec::PermanentDestroyed(filter),
        ]),
        raw_tokens,
    )))
}

/// Parse the player-first group-attack wording "you attack with one or more
/// <objects>". The ordinary attack grammar is subject-first ("creatures
/// attack"), so this inversion needs an explicit typed route.
fn try_parse_player_attack_with_one_or_more_lexed(
    raw_tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = trim_edge_punctuation_tokens(strip_leading_trigger_intro(raw_tokens));
    if tokens.iter().enumerate().any(|(index, token)| {
        token.is_word("and")
            && tokens
                .get(index + 1)
                .is_some_and(|next| trigger_token_is_atom(next, TriggerClauseAtom::TriggerIntro))
    }) {
        return Ok(None);
    }
    let words = crate::lexer::token_word_refs(tokens);
    if words.len() <= 6
        || !crate::word_primitives::parse_any_sequence_prefix(
            &words,
            &[
                &["you", "attack", "with", "one", "or", "more"],
                &["you", "attacks", "with", "one", "or", "more"],
            ],
        )
    {
        return Ok(None);
    }
    let Some(filter_start) = trigger_word_token_start(tokens, 6) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(&tokens[filter_start..], false)?;
    filter.controller = Some(PlayerFilter::You);
    filter.set_union_one_or_more(true);
    Ok(Some(apply_leading_trigger_intro_surface(
        TriggerSpec::AttacksOneOrMore(filter),
        raw_tokens,
    )))
}

fn try_parse_source_and_another_attack_different_players(
    raw_tokens: &[OwnedLexToken],
) -> Option<TriggerSpec> {
    let tokens = strip_leading_trigger_intro(raw_tokens);
    let tokens = trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "this",
            "creature",
            "and",
            "another",
            "creature",
            "attack",
            "different",
            "players",
        ],
    )
    .then_some(TriggerSpec::ThisAndAnotherAttackDifferentPlayers)
}

/// Parse a serial trigger union whose latter branches elide one shared player
/// subject: "an opponent attacks ... with N or more creatures, draws their
/// Nth card each turn, or casts their Nth spell each turn". Each branch stays
/// an independently executable trigger; this function only restores the
/// grammar-proven subject omitted after the first comma.
fn try_parse_shared_player_attack_draw_cast_union_lexed(
    raw_tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let tokens = strip_leading_trigger_intro(raw_tokens);
    let comma_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(index, token)| (token.kind == TokenKind::Comma).then_some(index))
        .collect::<Vec<_>>();
    if comma_indices.len() != 2 {
        return Ok(None);
    }
    let first_comma = comma_indices[0];
    let second_comma = comma_indices[1];
    if !tokens
        .get(second_comma + 1)
        .is_some_and(|token| token.is_word("or"))
    {
        return Ok(None);
    }
    let first = trim_edge_punctuation_tokens(&tokens[..first_comma]);
    let second = trim_edge_punctuation_tokens(&tokens[first_comma + 1..second_comma]);
    let third = trim_edge_punctuation_tokens(&tokens[second_comma + 2..]);

    let first_words = crate::lexer::token_word_refs(first);
    let Some(attacks_word) = crate::slice_primitives::select_position(&first_words, |word| {
        matches!(*word, "attack" | "attacks")
    }) else {
        return Ok(None);
    };
    let Some(with_offset) = crate::word_primitives::parse_sequence_start(
        first_words.get(attacks_word + 1..).unwrap_or_default(),
        &["with"],
    ) else {
        return Ok(None);
    };
    let with_word = attacks_word + 1 + with_offset;
    let Some(actor) = parse_trigger_subject_player_filter(&first_words[..attacks_word]) else {
        return Ok(None);
    };
    let Some(attacked) =
        parse_trigger_subject_player_filter(&first_words[attacks_word + 1..with_word])
    else {
        return Ok(None);
    };
    let Some(count_words) = first_words.get(with_word + 1..) else {
        return Ok(None);
    };
    let Some(or_more) = crate::word_primitives::parse_sequence_start(count_words, &["or"]) else {
        return Ok(None);
    };
    let Some((minimum, used)) = leaf::parse_leaf_number_prefix_words(&count_words[..or_more])
        .and_then(|number| number.into_fixed())
    else {
        return Ok(None);
    };
    if used != or_more
        || minimum < 2
        || count_words.get(or_more + 1) != Some(&"more")
        || count_words.len() <= or_more + 2
    {
        return Ok(None);
    }
    let filter_word_start = with_word + or_more + 3;
    let Some(filter_token_start) = trigger_word_token_start(first, filter_word_start) else {
        return Ok(None);
    };
    let Some(mut attack_filter) =
        parse_attack_trigger_subject_filter_lexed(&first[filter_token_start..])?
    else {
        return Ok(None);
    };
    attack_filter.controller = Some(actor.clone());
    attack_filter.attacking_player_or_planeswalker_controlled_by = Some(attacked.clone());
    attack_filter.targets_only_player = Some(attacked);
    attack_filter.set_union_one_or_more(true);
    let attack = TriggerSpec::AttacksOneOrMoreWithMinTotal {
        filter: attack_filter,
        min_total_attackers: minimum,
    };

    let second_words = crate::lexer::token_word_refs(second);
    if second_words.len() != 6
        || !crate::word_primitives::parse_choice_sequence_complete(
            &second_words[..2],
            &[&["draw", "draws"], &["their"]],
        )
        || second_words[3] != "card"
        || !crate::word_primitives::parse_sequence_suffix(&second_words, &["each", "turn"])
    {
        return Ok(None);
    }
    let Some(draw_number) = ironsmith_core::parse_ordinal_word(second_words[2]) else {
        return Ok(None);
    };
    let draw = TriggerSpec::PlayerDrawsNthCardEachTurn {
        player: actor.clone(),
        card_number: draw_number,
    };

    let third_words = crate::lexer::token_word_refs(third);
    if third_words.len() != 6
        || !crate::word_primitives::parse_choice_sequence_complete(
            &third_words[..2],
            &[&["cast", "casts"], &["their"]],
        )
        || third_words[3] != "spell"
        || !crate::word_primitives::parse_sequence_suffix(&third_words, &["each", "turn"])
    {
        return Ok(None);
    }
    let Some(spell_number) = ironsmith_core::parse_ordinal_word(third_words[2]) else {
        return Ok(None);
    };
    let cast = TriggerSpec::SpellCast {
        filter: None,
        mana_source_filter: None,
        caster: actor,
        timing: None,
        during_turn: None,
        min_spells_this_turn: None,
        exact_spells_this_turn: Some(spell_number),
        from_not_hand: false,
    };

    Ok(Some(apply_leading_trigger_intro_surface(
        TriggerSpec::AnyOf(vec![attack, draw, cast]),
        raw_tokens,
    )))
}

fn attacked_player_from_attack_trigger(trigger: &TriggerSpec) -> Option<PlayerFilter> {
    let filter = match trigger {
        TriggerSpec::WithIntro { trigger, .. } => {
            return attacked_player_from_attack_trigger(trigger);
        }
        TriggerSpec::Attacks(filter) | TriggerSpec::AttacksOneOrMore(filter) => filter,
        TriggerSpec::AttacksOneOrMoreWithMinTotal { filter, .. }
        | TriggerSpec::AttacksOneOrMoreWithExactTotal { filter, .. }
        | TriggerSpec::AttacksOneOrMoreWithAggregate { filter, .. } => filter,
        _ => return None,
    };
    filter
        .attacking_player_or_planeswalker_controlled_by
        .clone()
}

fn try_parse_relative_player_attack_branch(
    raw_tokens: &[OwnedLexToken],
    antecedent: PlayerFilter,
) -> Option<TriggerSpec> {
    let tokens = strip_leading_trigger_intro(raw_tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let attacks_idx = crate::slice_primitives::select_position(&words, |word| {
        matches!(*word, "attack" | "attacks")
    })?;
    if !crate::word_primitives::parse_sequence_complete(&words[..attacks_idx], &["they"]) {
        return None;
    }
    let tail = &words[attacks_idx + 1..];
    let filter = attacking_filter_for_player(antecedent);
    let trigger =
        if trigger_pattern_accepts(tail, ATTACKS_YOU_OR_PLANESWALKER_YOU_CONTROL_TAIL_PATTERN) {
            TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(filter)
        } else {
            return None;
        };
    Some(apply_leading_trigger_intro_surface(trigger, raw_tokens))
}

/// Preserve a repeated-intro attack union and resolve the right-hand "they"
/// to the player named by the first attack target:
/// "When you attack enchanted opponent ... or when they attack you ...".
fn try_parse_repeated_intro_attack_union_lexed(tokens: &[OwnedLexToken]) -> Option<TriggerSpec> {
    for (or_idx, token) in tokens.iter().enumerate() {
        if or_idx == 0
            || or_idx + 2 >= tokens.len()
            || !token.is_word("or")
            || !tokens[or_idx + 1].is_word("when") && !tokens[or_idx + 1].is_word("whenever")
        {
            continue;
        }

        let left_raw = &tokens[..or_idx];
        let right_raw = &tokens[or_idx + 1..];
        let left = parse_trigger_clause_lexed(strip_leading_trigger_intro(left_raw)).ok()?;
        let antecedent = attacked_player_from_attack_trigger(&left)?;
        let right = try_parse_relative_player_attack_branch(right_raw, antecedent)?;
        return Some(TriggerSpec::Either(
            Box::new(apply_leading_trigger_intro_surface(left, left_raw)),
            Box::new(right),
        ));
    }
    None
}

/// "Whenever A or B" over two distinct trigger events, where the right half
/// resumes with a bare verb sharing the left half's subject ("an opponent
/// sacrifices a nontoken permanent or discards a permanent card").
///
/// Deliberately narrow: only verbs with no dedicated union spec are split
/// here, so tuned single-spec unions (enters-or-attacks, damage recipients)
/// keep their existing routes.
fn try_parse_trigger_union_lexed(tokens: &[OwnedLexToken]) -> Option<TriggerSpec> {
    const UNION_RIGHT_VERBS: &[&str] = &["sacrifices", "discards", "leaves", "phases"];
    const UNION_RIGHT_PASSIVE_PREFIX: &[&str] = &["is", "put", "into", "exile"];
    for (idx, token) in tokens.iter().enumerate() {
        if idx == 0 || idx + 1 >= tokens.len() || !token.is_word("or") {
            continue;
        }
        let left = &tokens[..idx];
        let right = &tokens[idx + 1..];
        let right_words = crate::lexer::token_word_refs(right);
        // Only the exact "is put into exile" passive is unioned here — a
        // broader "is" gate steals natively paired shapes like
        // "enters the battlefield or is put into a graveyard".
        let passive_right =
            crate::word_primitives::parse_sequence_prefix(&right_words, UNION_RIGHT_PASSIVE_PREFIX);
        if !passive_right
            && !right_words
                .first()
                .is_some_and(|word| UNION_RIGHT_VERBS.iter().any(|verb| verb == word))
        {
            continue;
        }
        let Ok(left_spec) = parse_trigger_clause_lexed_unstacked(left) else {
            continue;
        };
        // A passive right half ("... or is put into exile") shares the whole
        // subject noun phrase, so try the LONGEST prefix first. An active-verb
        // right half ("... or discards a permanent card") only needs the bare
        // subject; longer prefixes there can join two verbs into one lenient
        // misparse ("sacrifices a discards a permanent card").
        let takes: Vec<usize> = if passive_right {
            (0..=left.len().min(4)).rev().collect()
        } else {
            (0..=left.len().min(4)).collect()
        };
        for take in takes {
            let mut candidate: Vec<OwnedLexToken> = left[..take].to_vec();
            candidate.extend_from_slice(right);
            if let Ok(right_spec) = parse_trigger_clause_lexed_unstacked(&candidate) {
                return Some(TriggerSpec::AnyOf(vec![left_spec, right_spec]));
            }
        }
    }
    None
}

#[path = "semantic/semantic_counter_programs.rs"]
mod semantic_counter_programs;
use semantic_counter_programs::{
    parse_loyalty_ability_trigger_tail_lexed, split_counter_recipient_or_player,
};
#[path = "semantic/semantic_trigger_programs.rs"]
mod semantic_trigger_programs;
use semantic_trigger_programs::{
    parse_ability_of_object_trigger_tail_lexed, parse_named_ability_trigger_tail_lexed,
    parse_possessive_ability_trigger_tail_lexed, parse_trigger_clause_lexed_unstacked,
};

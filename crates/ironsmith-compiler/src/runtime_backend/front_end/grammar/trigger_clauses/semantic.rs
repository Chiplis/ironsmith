use super::*;
use crate::runtime_backend::grammar::abilities as ability_grammar;
use crate::runtime_backend::grammar::trigger_clauses::{
    self as trigger_grammar, TriggerClauseAtom, TriggerClausePattern,
    trigger_clause_pattern as clause_shape,
};
use crate::runtime_backend::lexer::{token_slice_at_is, token_slice_at_is_any};

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
const BEGINNING_FIRST_MAIN_PHASE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "first", "main", "phase"]);
const BEGINNING_SECOND_MAIN_PHASE_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "second", "main", "phase"]);
const BEGINNING_PRECOMBAT_MAIN_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "precombat", "main"]);
const BEGINNING_POSTCOMBAT_MAIN_TRIGGER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["beginning", "postcombat", "main"]);
const DAMAGE_BY_THIS_TURN_DIES_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["this", "turn"]; contains_phrases & [&["dealt", "damage", "by"]]);
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

fn split_activation_cost_tap_condition_tail_lexed<'a, 'w>(
    tail_tokens: &'a [OwnedLexToken],
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
    exact
        & [
            "one",
            "of",
            "your",
            "opponents",
            "or",
            "a",
            "planeswalker",
            "they",
            "control",
        ]
);
const ATTACKS_PLANESWALKER_OR_BATTLE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "planeswalker"], &["a", "battle"]]);
const THIS_BLOCKS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["this", "creature", "blocks"], &["this", "blocks"]]);
const THIS_BLOCKS_OR_BECOMES_BLOCKED_TRIGGER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature", "blocks", "or", "becomes", "blocked"],
            &["this", "blocks", "or", "becomes", "blocked"],
        ]
);
const THIS_BLOCKS_OR_BECOMES_BLOCKED_BY_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "this", "creature", "blocks", "or", "becomes", "blocked", "by",
            ],
            &["this", "blocks", "or", "becomes", "blocked", "by"],
        ]
);
const THIS_BECOMES_BLOCKED_BY_TRIGGER_PREFIX: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "creature", "becomes", "blocked", "by"],
            &["this", "becomes", "blocked", "by"],
        ]
);
const EXPLORE_LAND_CARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "land", "card"], &["land", "card"]]);
const EXPLORE_NONLAND_CARD_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a", "nonland", "card"], &["nonland", "card"]]);
const NAME_STICKER_PUT_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["name", "sticker"]]; contains_words & ["on"]);
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
const DEALT_DAMAGE_BY_WORDS: &[&str] = &["dealt", "damage", "by"];
const DEALT_DAMAGE_BY_PATTERN: ClauseShape<'static> = clause_shape!(exact & DEALT_DAMAGE_BY_WORDS);
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
    let or_idx = trigger_atom_token(target_tokens, TriggerClauseAtom::Or)?;
    let left_tokens = trim_commas(&target_tokens[..or_idx]);
    let right_tokens = trim_commas(&target_tokens[or_idx + 1..]);
    if left_tokens.is_empty() || right_tokens.is_empty() {
        return None;
    }

    let left_words = ActivationRestrictionCompatWords::new(&left_tokens).to_word_refs();
    if let Some(player) = parse_trigger_subject_player_filter(&left_words)
        && let Ok(filter) =
            parse_object_filter_lexed(strip_leading_one_or_more_lexed(&right_tokens), false)
    {
        return Some((player, filter, true));
    }

    let right_words = ActivationRestrictionCompatWords::new(&right_tokens).to_word_refs();
    if let Some(player) = parse_trigger_subject_player_filter(&right_words)
        && let Ok(filter) =
            parse_object_filter_lexed(strip_leading_one_or_more_lexed(&left_tokens), false)
    {
        return Some((player, filter, false));
    }

    None
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

fn trigger_subject_tokens_before_suffix<'a>(
    tokens: &'a [OwnedLexToken],
    total_word_len: usize,
    suffix_word_len: usize,
) -> &'a [OwnedLexToken] {
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

pub(crate) fn strip_leading_trigger_intro(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
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

fn this_enters_battlefield_trigger_spec(
    surface: Option<crate::target::SourceReferenceSurface>,
) -> TriggerSpec {
    match surface {
        Some(surface) => TriggerSpec::ThisEntersBattlefieldWithSurface(surface),
        None => TriggerSpec::ThisEntersBattlefield,
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
        return current_source_reference_name();
    }

    if let Some(
        crate::target::SourceReferenceSurface::FullName(text)
        | crate::target::SourceReferenceSurface::ShortName(text)
        | crate::target::SourceReferenceSurface::ThisPermanentType(text),
    ) = source_reference_surface_for_span(span_from_tokens(tokens))
    {
        return Some(text);
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

pub(crate) fn split_trigger_or_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    trigger_grammar::parse_trigger_or_split(tokens).map(|split| split.separator)
}

pub(crate) fn has_leading_one_or_more(tokens: &[OwnedLexToken]) -> bool {
    leading_one_or_more_prefix_len(tokens).is_some()
}

pub(crate) fn leading_one_or_more_prefix_len(tokens: &[OwnedLexToken]) -> Option<usize> {
    let (count, used) =
        parse_greater_than_or_equal_quantity_prefix(tokens, false, false, "trigger subject")
            .ok()
            .flatten()?;
    (count == 1).then_some(used)
}

pub(crate) fn parse_leading_or_more_quantifier(
    tokens: &[OwnedLexToken],
) -> Option<(u32, &[OwnedLexToken])> {
    let (count, used) =
        parse_greater_than_or_equal_quantity_prefix(tokens, false, false, "trigger quantifier")
            .ok()
            .flatten()?;
    Some((count, &tokens[used..]))
}

pub(crate) fn parse_leading_exactly_quantifier(
    tokens: &[OwnedLexToken],
) -> Option<(u32, &[OwnedLexToken])> {
    if !tokens.first()?.is_word("exactly") {
        return None;
    }
    let (count, used) = parse_number(&tokens[1..])?;
    Some((count, &tokens[1 + used..]))
}

pub(crate) fn parse_trigger_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    fn parse_damage_by_dies_trigger_lexed(
        subject_tokens: &[OwnedLexToken],
        other: bool,
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if subject_words.len() < 8
            || !trigger_pattern_accepts(&subject_words, DAMAGE_BY_THIS_TURN_DIES_SUBJECT_PATTERN)
        {
            return Ok(None);
        }

        let Some(dealt_word_idx) = find_phrase_shape(
            &subject_words,
            DEALT_DAMAGE_BY_WORDS.len(),
            DEALT_DAMAGE_BY_PATTERN,
        ) else {
            return Ok(None);
        };

        let victim_end = trigger_word_token_start(subject_tokens, dealt_word_idx).unwrap_or(0);
        if victim_end == 0 || victim_end > subject_tokens.len() {
            return Ok(None);
        }

        let victim_tokens = trim_edge_punctuation_tokens(&subject_tokens[..victim_end]);
        let victim_tokens = strip_leading_article_tokens(victim_tokens);
        if victim_tokens.is_empty() {
            return Ok(None);
        }

        let damager_start_word_idx = dealt_word_idx + 3;
        let this_word_idx = subject_words.len() - 2;
        let damager_start = trigger_word_token_start(subject_tokens, damager_start_word_idx)
            .unwrap_or(subject_tokens.len());
        let damager_end =
            trigger_word_token_start(subject_tokens, this_word_idx).unwrap_or(subject_tokens.len());
        if damager_start >= damager_end || damager_end > subject_tokens.len() {
            return Ok(None);
        }

        let damager_tokens =
            trim_edge_punctuation_tokens(&subject_tokens[damager_start..damager_end]);
        let damager_word_view = ActivationRestrictionCompatWords::new(&damager_tokens);
        let damager_words = damager_word_view.to_word_refs();
        let has_named_source_words = !damager_words.is_empty()
            && !damager_words.first().is_some_and(|word| {
                trigger_word_accepts_pattern(word, DAMAGER_NAMED_SOURCE_LEADING_EXCLUDED_PATTERN)
            })
            && !damager_words
                .iter()
                .any(|word| trigger_word_accepts_pattern(word, GENERIC_DAMAGE_SOURCE_WORD_PATTERN));

        let damager = if trigger_pattern_accepts(&damager_words, THIS_DAMAGE_SOURCE_TRIGGER_PATTERN)
            || has_named_source_words
        {
            Some(DamageBySpec::ThisCreature)
        } else if trigger_pattern_accepts(
            &damager_words,
            EQUIPPED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN,
        ) {
            Some(DamageBySpec::EquippedCreature)
        } else if trigger_pattern_accepts(
            &damager_words,
            ENCHANTED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN,
        ) {
            Some(DamageBySpec::EnchantedCreature)
        } else {
            None
        };

        let Some(damager) = damager else {
            return Ok(None);
        };

        let victim = parse_object_filter_lexed(&victim_tokens, other).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damaged-by trigger victim filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        Ok(Some(TriggerSpec::DiesCreatureDealtDamageByThisTurn {
            victim,
            damager,
        }))
    }

    fn parse_simple_spell_activity_trigger_lexed(
        tokens: &[OwnedLexToken],
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        if !trigger_pattern_accepts(clause_words, SIMPLE_SPELL_ACTIVITY_OBJECT_PATTERN) {
            return Ok(None);
        }
        if trigger_pattern_accepts(clause_words, SIMPLE_SPELL_ACTIVITY_EXCLUDED_WORD_PATTERN)
            || trigger_pattern_accepts(clause_words, SIMPLE_SPELL_ACTIVITY_EXCLUDED_PHRASE_PATTERN)
        {
            return Ok(None);
        }

        let cast_idx = trigger_atom_token(tokens, TriggerClauseAtom::Cast);
        let copy_idx = trigger_atom_token(tokens, TriggerClauseAtom::Copy);
        if cast_idx.is_none() && copy_idx.is_none() {
            return Ok(None);
        }

        let actor = parse_subject_clause_player_filter(clause_words);
        let parse_filter =
            |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
                let filter_words = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_words.to_word_refs();
                let is_unqualified_spell =
                    trigger_pattern_accepts(&filter_words, UNQUALIFIED_SPELL_FILTER_PATTERN);
                if filter_tokens.is_empty() || is_unqualified_spell {
                    return Ok(None);
                }
                parse_object_filter_lexed(filter_tokens, false)
                    .map(Some)
                    .map_err(|err| {
                        CardTextError::ParseError(format!(
                            "unsupported spell trigger filter (clause: '{}') [{err:?}]",
                            filter_words.join(" ")
                        ))
                    })
            };

        if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
            let (first, second, first_is_cast) = if cast < copy {
                (cast, copy, true)
            } else {
                (copy, cast, false)
            };
            let between_view = ActivationRestrictionCompatWords::new(&tokens[first + 1..second]);
            let between_words = between_view.to_word_refs();
            if trigger_pattern_accepts(&between_words, CAST_OR_COPY_SEPARATOR_PATTERN) {
                let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
                let cast_trigger = TriggerSpec::SpellCast {
                    filter: filter.clone(),
                    caster: actor.clone(),
                    during_turn: None,
                    min_spells_this_turn: None,
                    exact_spells_this_turn: None,
                    from_not_hand: false,
                };
                let copied_trigger = TriggerSpec::SpellCopied {
                    filter,
                    copier: actor,
                };
                return Ok(Some(if first_is_cast {
                    TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
                } else {
                    TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
                }));
            }
        }

        if let Some(cast) = cast_idx {
            let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
            if filter_tokens.is_empty() {
                let mut prefix_tokens = &tokens[..cast];
                while let Some(last_word) = prefix_tokens.last().and_then(OwnedLexToken::as_word) {
                    if trigger_word_accepts_pattern(last_word, LINKING_BE_WORD_PATTERN) {
                        prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                    } else {
                        break;
                    }
                }
                let has_spell_noun = prefix_tokens
                    .iter()
                    .any(|token| token_matches_clause_shape(token, SPELL_NOUN_PATTERN));
                if has_spell_noun {
                    filter_tokens = prefix_tokens;
                }
            }
            let filter = parse_filter(filter_tokens)?;
            return Ok(Some(TriggerSpec::SpellCast {
                filter,
                caster: actor,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }));
        }

        if let Some(copy) = copy_idx {
            let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
            return Ok(Some(TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            }));
        }

        Ok(None)
    }

    fn parse_spell_countered_trigger_lexed(
        tokens: &[OwnedLexToken],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        let Some(spec) = ability_grammar::parse_spell_countered_trigger_spec_lexed(tokens) else {
            return Ok(None);
        };
        let filter = spec
            .filter_tokens
            .map(|filter_tokens| {
                let filter_words =
                    ActivationRestrictionCompatWords::new(filter_tokens).to_word_refs();
                parse_object_filter_lexed(filter_tokens, false).map_err(|err| {
                    CardTextError::ParseError(format!(
                        "unsupported spell-countered trigger filter (clause: '{}') [{err:?}]",
                        filter_words.join(" ")
                    ))
                })
            })
            .transpose()?;

        Ok(Some(TriggerSpec::SpellCountered {
            filter,
            controller: spec.controller,
        }))
    }

    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "empty trigger clause".to_string(),
        ));
    }

    if trigger_pattern_accepts(&words, CRAFT_EXILED_FROM_BATTLEFIELD_TRIGGER_PATTERN) {
        return Ok(
            TriggerSpec::ThisExiledFromBattlefieldDuringCostOfAbilityWithMarker {
                marker: "craft".to_string(),
            },
        );
    }

    if words.len() > 6
        && trigger_pattern_accepts(&words, FINAL_CHAPTER_ABILITY_RESOLVES_TRIGGER_PATTERN)
    {
        let mut filter =
            parse_object_filter_lexed(&tokens[5..tokens.len() - 1], false).map_err(|err| {
                CardTextError::ParseError(format!(
                    "unsupported final chapter trigger filter: {} [{err:?}]",
                    words[5..words.len() - 1].join(" ")
                ))
            })?;
        filter.zone.get_or_insert(Zone::Battlefield);
        return Ok(TriggerSpec::FinalChapterAbilityResolved(filter));
    }

    if trigger_pattern_accepts(&words, DAY_NIGHT_CHANGED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::DayNightChanged);
    }

    if let Some(enters_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Enter) {
        let tail = &tokens[enters_idx + 1..];
        let tail_words = ActivationRestrictionCompatWords::new(tail).to_word_refs();
        let shared_subject_or_combat_damage = trigger_pattern_accepts(
            &tail_words,
            SHARED_SUBJECT_ETB_OR_COMBAT_DAMAGE_TAIL_PATTERN,
        );
        if shared_subject_or_combat_damage {
            let or_idx = enters_idx + 3;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.extend_from_slice(&tokens[or_idx + 1..]);

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
        let shared_subject_or_attack =
            trigger_pattern_accepts(&tail_words, SHARED_SUBJECT_ETB_OR_ATTACK_TAIL_PATTERN);
        if shared_subject_or_attack {
            let or_idx = if trigger_pattern_accepts(&tail_words[..1], OR_WORD_PATTERN) {
                enters_idx + 1
            } else {
                enters_idx + 3
            };
            let attack_idx = or_idx + 1;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.push(tokens[attack_idx].clone());

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
    }

    if let Some(or_idx) = split_trigger_or_index(tokens) {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    if let Some(and_idx) = trigger_atom_token(tokens, TriggerClauseAtom::And)
        && tokens
            .get(and_idx + 1)
            .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::TriggerIntro))
    {
        let left_raw_tokens = &tokens[..and_idx];
        let right_raw_tokens = &tokens[and_idx + 1..];
        let left_tokens = strip_leading_trigger_intro(left_raw_tokens);
        let right_tokens = strip_leading_trigger_intro(right_raw_tokens);
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(
                Box::new(apply_leading_trigger_intro_surface(left, left_raw_tokens)),
                Box::new(apply_leading_trigger_intro_surface(right, right_raw_tokens)),
            ));
        }
    }

    if words.len() >= 2
        && trigger_word_at_accepts_pattern(&words, words.len() - 1, ALONE_WORD_PATTERN)
        && trigger_word_at_accepts_pattern(&words, words.len() - 2, ATTACK_OR_ATTACKS_PATTERN)
    {
        let attacks_word_idx = words.len().saturating_sub(2);
        let attacks_token_idx =
            trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksAlone(filter),
                None => TriggerSpec::AttacksAlone(ObjectFilter::source()),
            },
        );
    }

    if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
        let tail_words = &words[attacks_word_idx + 1..];
        if trigger_pattern_accepts(
            tail_words,
            ATTACKS_YOU_OR_PLANESWALKER_YOU_CONTROL_TAIL_PATTERN,
        ) {
            let attacks_token_idx =
                trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            let subject_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?
                .unwrap_or_else(ObjectFilter::source);
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            return Ok(if player_subject {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(subject_filter)
            } else {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControl(subject_filter)
            });
        }
    }

    if words.len() >= 3
        && trigger_word_at_accepts_pattern(&words, words.len() - 3, ATTACK_OR_ATTACKS_PATTERN)
        && trigger_word_at_accepts_pattern(&words, words.len() - 2, WHILE_WORD_PATTERN)
        && trigger_word_at_accepts_pattern(&words, words.len() - 1, SADDLED_WORD_PATTERN)
    {
        let attacks_word_idx = words.len().saturating_sub(3);
        let attacks_token_idx =
            trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksWhileSaddled(filter),
                None => TriggerSpec::ThisAttacksWhileSaddled,
            },
        );
    }

    if trigger_pattern_accepts(&words, YOU_CAST_THIS_SPELL_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if let Some(spell_countered_trigger) = parse_spell_countered_trigger_lexed(tokens)? {
        return Ok(spell_countered_trigger);
    }
    if let Some(spell_activity_trigger) = parse_simple_spell_activity_trigger_lexed(tokens, &words)?
    {
        return Ok(spell_activity_trigger);
    }
    if let Some(spell_activity_trigger) = parse_spell_activity_trigger(tokens)? {
        return Ok(spell_activity_trigger);
    }

    if let Some(play_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Play) {
        let subject_tokens = &tokens[..play_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let trimmed_object_tokens = trim_commas(&tokens[play_idx + 1..]);
            let object_tokens = strip_leading_articles(&trimmed_object_tokens);
            let object_word_view = ActivationRestrictionCompatWords::new(&object_tokens);
            let object_words = object_word_view.to_word_refs();
            if object_words
                .iter()
                .any(|word| trigger_word_accepts_pattern(word, LAND_OR_LANDS_PATTERN))
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerPlaysLand { player, filter });
            }
        }
    }

    if let Some(search_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Search) {
        let subject_tokens = &tokens[..search_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let searched_tokens = trim_commas(&tokens[search_idx + 1..]);
            let searched_word_view = ActivationRestrictionCompatWords::new(&searched_tokens);
            let searched_words = searched_word_view.to_word_refs();
            if trigger_pattern_accepts(&searched_words, LIBRARY_SEARCH_TARGET_PATTERN) {
                return Ok(TriggerSpec::PlayerSearchesLibrary(player));
            }
        }
    }

    if let Some(shuffle_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Shuffle) {
        let subject_tokens = &tokens[..shuffle_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let shuffled_tokens = trim_commas(&tokens[shuffle_idx + 1..]);
        let shuffled_word_view = ActivationRestrictionCompatWords::new(&shuffled_tokens);
        let shuffled_words = shuffled_word_view.to_word_refs();
        if trigger_pattern_accepts(&shuffled_words, LIBRARY_SHUFFLE_TARGET_PATTERN) {
            if let Some((player, caused_by_effect, source_controller_shuffles)) =
                parse_shuffle_trigger_subject(&subject_words)
            {
                return Ok(TriggerSpec::PlayerShufflesLibrary {
                    player,
                    caused_by_effect,
                    source_controller_shuffles,
                });
            }
        }
    }

    if let Some(give_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Give) {
        let subject_tokens = &tokens[..give_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let gifted_tokens = trim_commas(&tokens[give_idx + 1..]);
            let gifted_word_view = ActivationRestrictionCompatWords::new(&gifted_tokens);
            let gifted_words = gifted_word_view.to_word_refs();
            if trigger_pattern_accepts(&gifted_words, GIFT_TAIL_PATTERN) {
                return Ok(TriggerSpec::PlayerGivesGift(player));
            }
        }
    }

    if let Some(create_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Create) {
        let subject_tokens = &tokens[..create_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let object_tokens = trim_commas(&tokens[create_idx + 1..]);
            let one_or_more = has_leading_one_or_more(&object_tokens);
            let object_tokens = strip_leading_one_or_more_lexed(&object_tokens);
            let filter = parse_object_filter_lexed(object_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported token-created trigger filter (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            return Ok(TriggerSpec::TokensCreated {
                player,
                filter,
                one_or_more,
            });
        }
    }

    if let Some(tap_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Tap) {
        let subject_tokens = &tokens[..tap_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let after_tap = &tokens[tap_idx + 1..];
            if let Some(for_idx) = trigger_atom_token(after_tap, TriggerClauseAtom::For)
                && for_idx > 0
            {
                let object_tokens = trim_commas(&after_tap[..for_idx]);
                let object_tokens = strip_leading_articles(&object_tokens);
                if !object_tokens.is_empty()
                    && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
                {
                    return Ok(TriggerSpec::PlayerTapsForMana { player, filter });
                }
            }
        }
    }

    if let Some(tapped_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Tapped)
        && tapped_idx >= 2
        && tokens
            .get(tapped_idx.wrapping_sub(1))
            .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::IsOrAre))
    {
        let subject_tokens = &tokens[..tapped_idx - 1];
        let after_tapped = &tokens[tapped_idx + 1..];
        if crate::runtime_backend::lexer::contains_token_word(after_tapped, "for") {
            let object_tokens = trim_commas(subject_tokens);
            let object_tokens = strip_leading_articles(&object_tokens);
            if !object_tokens.is_empty()
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerTapsForMana {
                    player: PlayerFilter::Any,
                    filter,
                });
            }
        }
    }

    if let Some(activate_idx) = trigger_atom_word(&words, TriggerClauseAtom::Activate) {
        let subject_tokens = &tokens[..activate_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(activator) = parse_trigger_subject_player_filter(&subject_words) {
            let raw_tail_words = &words[activate_idx + 1..];
            let tail_tokens = &tokens[activate_idx + 1..];
            let (activation_cost_has_tap, ability_tail_tokens, ability_tail_words) =
                split_activation_cost_tap_condition_tail_lexed(tail_tokens, raw_tail_words);
            let ability_tail_tokens = ability_tail_tokens.as_slice();
            let tail_words = ability_tail_words.as_slice();
            if let Some(filter) =
                parse_loyalty_ability_trigger_tail_lexed(ability_tail_tokens, tail_words)?
            {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only: false,
                    loyalty_only: true,
                    activation_cost_has_tap,
                });
            }
            if let Some((owner_filter, marker)) =
                parse_possessive_ability_trigger_tail_lexed(ability_tail_tokens, tail_words)?
            {
                let filter = match marker {
                    Some(marker) => owner_filter.with_ability_marker(marker),
                    None => owner_filter,
                };
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only: false,
                    loyalty_only: false,
                    activation_cost_has_tap,
                });
            }
            if let Some((filter, non_mana_only)) =
                parse_ability_of_object_trigger_tail_lexed(ability_tail_tokens, tail_words)?
            {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only,
                    loyalty_only: false,
                    activation_cost_has_tap,
                });
            }
            if trigger_pattern_accepts(tail_words, ACTIVATED_ABILITY_TAIL_PATTERN) {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter: ObjectFilter::default(),
                    non_mana_only: trigger_pattern_accepts(tail_words, MANA_ABILITY_TAIL_PATTERN),
                    loyalty_only: false,
                    activation_cost_has_tap,
                });
            }
        }
    }

    let has_deal = trigger_atom_word(&words, TriggerClauseAtom::Deal).is_some();
    if has_deal && trigger_pattern_accepts(&words, COMBAT_DAMAGE_TRIGGER_PATTERN) {
        if let Some(deals_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Deal) {
            let subject_tokens = &tokens[..deals_idx];
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let one_or_more = has_leading_one_or_more(subject_tokens) || player_subject;
            let source_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?;
            if let Some(damage_idx_rel) =
                trigger_atom_token(&tokens[deals_idx + 1..], TriggerClauseAtom::Damage)
            {
                let damage_idx = deals_idx + 1 + damage_idx_rel;
                if let Some(to_idx_rel) =
                    trigger_atom_token(&tokens[damage_idx + 1..], TriggerClauseAtom::To)
                {
                    let to_idx = damage_idx + 1 + to_idx_rel;
                    let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
                    if target_tokens.is_empty() {
                        return Err(CardTextError::ParseError(format!(
                            "missing combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        )));
                    }
                    let target_word_view = ActivationRestrictionCompatWords::new(&target_tokens);
                    let target_words = target_word_view.to_word_refs();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(match source_filter {
                            Some(source) => {
                                if one_or_more {
                                    TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                                        source,
                                        player,
                                    }
                                } else {
                                    TriggerSpec::DealsCombatDamageToPlayer { source, player }
                                }
                            }
                            None => TriggerSpec::ThisDealsCombatDamageToPlayer { player },
                        });
                    }

                    if let Some((player, target_filter, player_first)) =
                        parse_player_or_object_damage_recipient(&target_tokens)
                    {
                        let player_trigger = match source_filter.clone() {
                            Some(source) => {
                                if one_or_more {
                                    TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                                        source,
                                        player,
                                    }
                                } else {
                                    TriggerSpec::DealsCombatDamageToPlayer { source, player }
                                }
                            }
                            None if player == PlayerFilter::Any => {
                                TriggerSpec::ThisDealsCombatDamageToPlayer { player }
                            }
                            None => {
                                return Err(CardTextError::ParseError(format!(
                                    "unsupported combat damage player recipient filter in trigger clause (clause: '{}')",
                                    words.join(" ")
                                )));
                            }
                        };
                        let object_trigger = match source_filter {
                            Some(source) => TriggerSpec::DealsCombatDamageTo {
                                source,
                                target: target_filter,
                            },
                            None => TriggerSpec::ThisDealsCombatDamageTo(target_filter),
                        };
                        return Ok(if player_first {
                            TriggerSpec::Either(Box::new(player_trigger), Box::new(object_trigger))
                        } else {
                            TriggerSpec::Either(Box::new(object_trigger), Box::new(player_trigger))
                        });
                    }

                    let target_tokens = strip_leading_one_or_more_lexed(&target_tokens);
                    let target_filter = parse_object_filter_lexed(target_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                    return Ok(match source_filter {
                        Some(source) => TriggerSpec::DealsCombatDamageTo {
                            source,
                            target: target_filter,
                        },
                        None => TriggerSpec::ThisDealsCombatDamageTo(target_filter),
                    });
                }
            }

            return Ok(match source_filter {
                Some(filter) => TriggerSpec::DealsCombatDamage(filter),
                None => TriggerSpec::ThisDealsCombatDamage,
            });
        }
        return Ok(TriggerSpec::ThisDealsCombatDamage);
    }

    if trigger_pattern_accepts(&words, THIS_LEAVES_BATTLEFIELD_TRIGGER_PATTERN)
        || (words.len() == 5
            && trigger_word_at_accepts_pattern(&words, 0, THIS_WORD_PATTERN)
            && trigger_word_at_accepts_pattern(&words, 2, LEAVES_WORD_PATTERN)
            && trigger_word_at_accepts_pattern(&words, 3, THE_WORD_PATTERN)
            && trigger_word_at_accepts_pattern(&words, 4, BATTLEFIELD_WORD_PATTERN))
    {
        let subject_word_count = if words.len() == 5 { 2 } else { 1 };
        let subject_token_end = trigger_word_token_start(tokens, subject_word_count)
            .unwrap_or(subject_word_count.min(tokens.len()));
        return Ok(this_leaves_battlefield_trigger_spec(
            source_reference_surface_for_trigger_subject(&tokens[..subject_token_end]),
        ));
    }

    if let Some(enters_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Enter)
        && trigger_pattern_accepts(&words, ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN)
    {
        let enters_token_idx =
            trigger_word_token_start(tokens, enters_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..enters_token_idx];
        if let Some(surface) = source_reference_surface_for_trigger_subject(
            strip_leading_trigger_intro(subject_tokens),
        ) {
            return Ok(TriggerSpec::Either(
                Box::new(this_enters_battlefield_trigger_spec(Some(surface.clone()))),
                Box::new(this_leaves_battlefield_trigger_spec(Some(surface))),
            ));
        }
        if token_trigger_pattern_accepts(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN) {
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::ThisEntersBattlefield),
                Box::new(TriggerSpec::ThisLeavesBattlefield),
            ));
        }
    }

    if let Some(leaves_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Leave)
        && trigger_pattern_accepts(&words[leaves_word_idx..], LEAVES_BATTLEFIELD_SUFFIX_PATTERN)
    {
        let leaves_token_idx =
            trigger_word_token_start(tokens, leaves_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..leaves_token_idx];

        if let Some(surface) = source_reference_surface_for_trigger_subject(
            strip_leading_trigger_intro(subject_tokens),
        ) {
            return Ok(this_leaves_battlefield_trigger_spec(Some(surface)));
        }

        if let Some(or_idx) = trigger_atom_token(subject_tokens, TriggerClauseAtom::Or) {
            let left_tokens = &subject_tokens[..or_idx];
            let mut right_tokens = &subject_tokens[or_idx + 1..];
            let left_words = non_article_word_refs(
                &ActivationRestrictionCompatWords::new(left_tokens).to_word_refs(),
            );
            if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                let mut other = false;
                if token_trigger_pattern_accepts(right_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN) {
                    other = true;
                    right_tokens = &right_tokens[1..];
                }
                let parsed_filter =
                    parse_object_filter_lexed(right_tokens, other)
                        .ok()
                        .or_else(|| {
                            parse_subtype_list_enters_trigger_filter_lexed(right_tokens, other)
                        });
                if let Some(filter) = parsed_filter {
                    return Ok(TriggerSpec::Either(
                        Box::new(TriggerSpec::ThisLeavesBattlefield),
                        Box::new(TriggerSpec::LeavesBattlefield(filter)),
                    ));
                }
            }
        }

        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if token_trigger_pattern_accepts(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let parsed_filter = parse_object_filter_lexed(filtered_subject_tokens, other)
            .ok()
            .or_else(|| {
                parse_subtype_list_enters_trigger_filter_lexed(filtered_subject_tokens, other)
            });
        if let Some(filter) = parsed_filter {
            return Ok(TriggerSpec::LeavesBattlefield(filter));
        }
    }

    if let Some(dies_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Die) {
        let dies_token_idx =
            trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..dies_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && trigger_pattern_accepts(
                &words[dies_word_idx + 1..],
                OR_IS_PUT_INTO_EXILE_FROM_BATTLEFIELD_TAIL_PATTERN,
            )
        {
            return Ok(TriggerSpec::ThisDiesOrIsExiled);
        }
    }

    if let Some(enters_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Enter) {
        let enters_token_idx =
            trigger_word_token_start(tokens, enters_word_idx).unwrap_or(tokens.len());
        if trigger_pattern_accepts(&words, ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN) {
            let subject_tokens = &tokens[..enters_token_idx];
            if let Some(surface) = source_reference_surface_for_trigger_subject(
                strip_leading_trigger_intro(subject_tokens),
            ) {
                return Ok(TriggerSpec::Either(
                    Box::new(this_enters_battlefield_trigger_spec(Some(surface.clone()))),
                    Box::new(this_leaves_battlefield_trigger_spec(Some(surface))),
                ));
            }
            if token_trigger_pattern_accepts(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN)
            {
                return Ok(TriggerSpec::Either(
                    Box::new(TriggerSpec::ThisEntersBattlefield),
                    Box::new(TriggerSpec::ThisLeavesBattlefield),
                ));
            }
        }

        let enters_origin =
            trigger_grammar::parse_enters_origin_clause_words(&words[enters_word_idx + 1..])
                .map(|origin| (origin.zone, origin.owner));
        if enters_word_idx == 0 {
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: ObjectFilter::default(),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield
            });
        }

        let subject_tokens = &tokens[..enters_token_idx];
        if trigger_pattern_accepts(
            &words[enters_word_idx + 1..],
            OR_IS_PUT_INTO_GRAVEYARD_FROM_BATTLEFIELD_TAIL_PATTERN,
        ) {
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::Either(
                    Box::new(this_enters_battlefield_trigger_spec(
                        source_reference_surface_for_trigger_subject(subject_tokens),
                    )),
                    Box::new(TriggerSpec::PutIntoGraveyardFromZone {
                        filter: ObjectFilter::source(),
                        from: Zone::Battlefield,
                        one_or_more: false,
                    }),
                ));
            }
        }
        if trigger_pattern_accepts(
            &words[enters_word_idx + 1..],
            OR_TRANSFORMS_INTO_TAIL_PREFIX_PATTERN,
        ) {
            let destination_name =
                transform_destination_name_after_into(&word_view, enters_word_idx + 2, tokens);
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::Either(
                    Box::new(this_enters_battlefield_trigger_spec(
                        source_reference_surface_for_trigger_subject(subject_tokens),
                    )),
                    Box::new(this_transforms_trigger_spec(
                        source_reference_surface_for_trigger_subject(subject_tokens),
                        destination_name,
                    )),
                ));
            }
        }
        if let Some(or_idx) = trigger_atom_token(subject_tokens, TriggerClauseAtom::Or) {
            let or_is_one_or_more_quantifier = or_idx == 1
                && subject_tokens
                    .first()
                    .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::One))
                && subject_tokens
                    .get(or_idx + 1)
                    .is_some_and(|token| trigger_token_is_atom(token, TriggerClauseAtom::More));
            if or_is_one_or_more_quantifier {
                // "one or more" is a quantifier for a single ETB trigger, not
                // a source-or-other-subject disjunction like "this creature or a token".
            } else {
                let left_tokens = &subject_tokens[..or_idx];
                let mut right_tokens = &subject_tokens[or_idx + 1..];
                let left_word_view = ActivationRestrictionCompatWords::new(left_tokens);
                let left_words = non_article_word_refs(&left_word_view.to_word_refs());
                if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                    let mut other = false;
                    if token_trigger_pattern_accepts(right_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
                    {
                        other = true;
                        right_tokens = &right_tokens[1..];
                    }
                    let parsed_filter = parse_object_filter_lexed(right_tokens, other)
                        .ok()
                        .or_else(|| {
                            parse_subtype_list_enters_trigger_filter_lexed(right_tokens, other)
                        });
                    if let Some(mut filter) = parsed_filter {
                        if trigger_pattern_accepts(&words, UNDER_YOUR_CONTROL_PATTERN) {
                            filter.controller = Some(PlayerFilter::You);
                        } else if trigger_pattern_accepts(&words, UNDER_OPPONENT_CONTROL_PATTERN) {
                            filter.controller = Some(PlayerFilter::Opponent);
                        }
                        let cause_filter =
                            if contains_window(&words, &["without", "being", "played"]) {
                                Some(crate::events::cause::CauseFilter::not_type(
                                    crate::events::cause::CauseType::SpecialAction,
                                ))
                            } else {
                                None
                            };
                        let right_trigger =
                            if trigger_pattern_accepts(&words, UNTAPPED_WORD_PATTERN) {
                                TriggerSpec::EntersBattlefieldUntapped {
                                    filter,
                                    cause_filter,
                                }
                            } else if trigger_pattern_accepts(&words, TAPPED_WORD_PATTERN) {
                                TriggerSpec::EntersBattlefieldTapped {
                                    filter,
                                    cause_filter,
                                }
                            } else {
                                TriggerSpec::EntersBattlefield {
                                    filter,
                                    cause_filter,
                                }
                            };
                        return Ok(TriggerSpec::Either(
                            Box::new(this_enters_battlefield_trigger_spec(
                                source_reference_surface_for_trigger_subject(left_tokens),
                            )),
                            Box::new(right_trigger),
                        ));
                    }
                }
            }
        }
        if token_trigger_pattern_accepts(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN) {
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: trigger_grammar::parse_source_trigger_subject_words(
                        &subject_words,
                    )
                    .filter,
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield
            });
        }
        if let Some(surface) = source_reference_surface_for_trigger_subject(subject_tokens) {
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: ObjectFilter::default(),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefieldWithSurface(surface)
            });
        }

        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if token_trigger_pattern_accepts(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let one_or_more = ActivationRestrictionCompatWords::new(filtered_subject_tokens)
            .to_word_refs()
            .get(..3)
            .is_some_and(|words| trigger_pattern_accepts(words, ONE_OR_MORE_QUANTIFIER_PATTERN));
        filtered_subject_tokens = strip_leading_one_or_more_lexed(filtered_subject_tokens);
        if token_trigger_pattern_accepts(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN)
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let parsed_filter = parse_object_filter_lexed(filtered_subject_tokens, other)
            .ok()
            .or_else(|| {
                parse_subtype_list_enters_trigger_filter_lexed(filtered_subject_tokens, other)
            });
        if let Some(mut filter) = parsed_filter {
            let cause_filter = if contains_window(&words, &["without", "being", "played"]) {
                Some(crate::events::cause::CauseFilter::not_type(
                    crate::events::cause::CauseType::SpecialAction,
                ))
            } else {
                None
            };
            if trigger_pattern_accepts(&words, UNDER_YOUR_CONTROL_PATTERN) {
                filter.controller = Some(PlayerFilter::You);
            } else if trigger_pattern_accepts(&words, UNDER_OPPONENT_CONTROL_PATTERN) {
                filter.controller = Some(PlayerFilter::Opponent);
            }
            if trigger_pattern_accepts(&words, UNTAPPED_WORD_PATTERN) {
                return Ok(TriggerSpec::EntersBattlefieldUntapped {
                    filter,
                    cause_filter,
                });
            }
            if trigger_pattern_accepts(&words, TAPPED_WORD_PATTERN) {
                return Ok(TriggerSpec::EntersBattlefieldTapped {
                    filter,
                    cause_filter,
                });
            }
            return Ok(if let Some((from, owner)) = enters_origin {
                TriggerSpec::EntersBattlefieldFromZone {
                    filter,
                    from,
                    owner,
                    one_or_more,
                    cause_filter,
                }
            } else if one_or_more {
                TriggerSpec::EntersBattlefieldOneOrMore {
                    filter,
                    cause_filter,
                }
            } else {
                TriggerSpec::EntersBattlefield {
                    filter,
                    cause_filter,
                }
            });
        }
    }

    if let Some(transforms_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Transform) {
        let transforms_token_idx =
            trigger_word_token_start(tokens, transforms_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..transforms_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && words
                .get(transforms_word_idx + 1)
                .is_some_and(|word| trigger_word_accepts_pattern(word, INTO_WORD_PATTERN))
        {
            let destination_name =
                transform_destination_name_after_into(&word_view, transforms_word_idx, tokens);
            return Ok(this_transforms_trigger_spec(
                source_reference_surface_for_trigger_subject(subject_tokens),
                destination_name,
            ));
        }
    }

    let (zone_change_words, during_turn) =
        if trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX) {
            (
                &words[..words.len().saturating_sub(3)],
                Some(PlayerFilter::You),
            )
        } else {
            (words.as_slice(), None)
        };

    if trigger_pattern_accepts(
        zone_change_words,
        SPELL_OR_ABILITY_YOU_CONTROL_EXILES_PERMANENTS_FROM_BATTLEFIELD_PATTERN,
    ) {
        return Ok(TriggerSpec::PutIntoExileFromZones {
            filter: ObjectFilter::permanent_card(),
            from: vec![Zone::Battlefield],
            one_or_more: true,
            during_turn,
            cause_filter: Some(
                crate::events::cause::CauseFilter::effect_like()
                    .with_controller(crate::events::cause::ControllerFilter::ContextController),
            ),
        });
    }

    for tail in [
        ["leave", "your", "graveyard"].as_slice(),
        ["leaves", "your", "graveyard"].as_slice(),
    ] {
        if trigger_pattern_accepts(zone_change_words, ClauseShape::new().suffix(tail)) {
            let subject_word_len = zone_change_words.len().saturating_sub(tail.len());
            let mut subject_tokens = trigger_word_token_start(tokens, subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let one_or_more = has_leading_one_or_more(subject_tokens);
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            let mut filter = if subject_is_card_or_cards(&subject_words) {
                ObjectFilter::default()
            } else {
                parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported filter in leave-your-graveyard trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            filter.zone = None;
            filter.controller = None;
            filter.owner = None;
            if subject_mentions_card(&subject_words) {
                filter.nontoken = true;
            }
            return Ok(TriggerSpec::CardsLeaveYourGraveyard {
                filter,
                one_or_more,
                during_your_turn: during_turn == Some(PlayerFilter::You),
            });
        }
    }

    if let Some(suffix_word_len) = trigger_suffix_word_len(
        zone_change_words,
        PUT_INTO_GRAVEYARD_OR_EXILE_FROM_BATTLEFIELD_SUFFIXES,
    ) {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, zone_change_words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
        let stripped_subject_words =
            ActivationRestrictionCompatWords::new(subject_tokens).to_word_refs();
        let mut filter = if subject_is_card_or_cards(&stripped_subject_words) {
            ObjectFilter::default()
        } else {
            parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-graveyard-or-exile-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?
        };
        filter.zone = None;
        filter.owner = None;
        if filter.controller.is_none()
            && (trigger_pattern_accepts(&subject_words, UNDER_YOUR_CONTROL_PATTERN)
                || contains_window(&subject_words, &["you", "control"]))
        {
            filter.controller = Some(PlayerFilter::You);
        }
        if subject_mentions_card(&subject_words) {
            filter.card_types.clear();
            filter.nontoken = true;
        }
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::PutIntoGraveyardFromZone {
                filter: filter.clone(),
                from: Zone::Battlefield,
                one_or_more,
            }),
            Box::new(TriggerSpec::PutIntoExileFromZones {
                filter,
                from: vec![Zone::Battlefield],
                one_or_more,
                during_turn,
                cause_filter: None,
            }),
        ));
    }

    for (tail, from_zones) in [
        (["is", "put", "into", "exile"].as_slice(), Vec::new()),
        (["are", "put", "into", "exile"].as_slice(), Vec::new()),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and",
                "or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and",
                "or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and/or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyards",
                "and/or",
                "the",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and",
                "or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and",
                "or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "is",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and/or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            [
                "are",
                "put",
                "into",
                "exile",
                "from",
                "graveyard",
                "and/or",
                "battlefield",
            ]
            .as_slice(),
            vec![Zone::Graveyard, Zone::Battlefield],
        ),
        (
            ["is", "put", "into", "exile", "from", "your", "hand"].as_slice(),
            vec![Zone::Hand],
        ),
        (
            ["are", "put", "into", "exile", "from", "your", "hand"].as_slice(),
            vec![Zone::Hand],
        ),
    ] {
        if trigger_pattern_accepts(zone_change_words, ClauseShape::new().suffix(tail)) {
            let from_your_hand = trigger_pattern_accepts(tail, FROM_YOUR_HAND_SUFFIX_PATTERN);
            let subject_word_len = zone_change_words.len().saturating_sub(tail.len());
            let subject_tokens = trigger_word_token_start(tokens, subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            let one_or_more = subject_starts_one_or_more(&subject_words);
            let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            let stripped_subject_words =
                ActivationRestrictionCompatWords::new(subject_tokens).to_word_refs();
            let mut filter = if subject_is_card_or_cards(&stripped_subject_words) {
                ObjectFilter::default()
            } else {
                parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported filter in put-into-exile-from-zones trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            filter.zone = None;
            filter.controller = None;
            filter.owner = if from_your_hand {
                Some(PlayerFilter::You)
            } else {
                None
            };
            if subject_mentions_card(&subject_words) {
                filter.card_types.clear();
                filter.nontoken = true;
            }
            return Ok(TriggerSpec::PutIntoExileFromZones {
                filter,
                from: from_zones,
                one_or_more,
                during_turn,
                cause_filter: None,
            });
        }
    }

    if let Some(suffix_word_len) = trigger_suffix_word_len(&words, PUT_INTO_YOUR_GRAVEYARD_SUFFIXES)
    {
        let mut subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let one_or_more = has_leading_one_or_more(subject_tokens);
        subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported card filter in put-into-your-graveyard trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        filter.zone = None;
        filter.controller = None;
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        if subject_mentions_permanent(&subject_words) {
            filter.card_types = ObjectFilter::permanent_card().card_types;
        }
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
        }
        return Ok(if one_or_more {
            TriggerSpec::PutIntoGraveyardOneOrMore(filter)
        } else {
            TriggerSpec::PutIntoGraveyard(filter)
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_A_GRAVEYARD_FROM_ANYWHERE_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        if is_source_reference_words(&subject_words) {
            return Ok(TriggerSpec::PutIntoGraveyard(ObjectFilter::source()));
        }
        if let Ok(filter) = parse_object_filter_lexed(subject_tokens, false) {
            return Ok(TriggerSpec::PutIntoGraveyard(filter));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported filter in put-into-graveyard-from-anywhere trigger clause (clause: '{}')",
            words.join(" ")
        )));
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_OPPONENT_GRAVEYARD_FROM_ANYWHERE_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if is_source_reference_words(&subject_words) {
            let mut filter = ObjectFilter::source();
            filter.owner = Some(PlayerFilter::Opponent);
            return Ok(if one_or_more {
                TriggerSpec::PutIntoGraveyardOneOrMore(filter)
            } else {
                TriggerSpec::PutIntoGraveyard(filter)
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-opponents-graveyard-from-anywhere trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        filter.owner = Some(PlayerFilter::Opponent);
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
        }
        return Ok(if one_or_more {
            TriggerSpec::PutIntoGraveyardOneOrMore(filter)
        } else {
            TriggerSpec::PutIntoGraveyard(filter)
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, ATTACHED_OBJECT_PUT_INTO_GRAVEYARD_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        if trigger_pattern_accepts(&subject_words, ATTACHED_OBJECT_PREFIX_PATTERN) {
            let one_or_more = subject_starts_one_or_more(&subject_words);
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in attached-object put-into-graveyard trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.owner = None;
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
                one_or_more,
            });
        }
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_YOUR_GRAVEYARD_FROM_LIBRARY_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard-from-library trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
        }
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Library,
            one_or_more,
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_YOUR_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if is_source_reference_words(&subject_words) {
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter: ObjectFilter::source(),
                from: Zone::Battlefield,
                one_or_more,
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
        }
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Battlefield,
            one_or_more,
        });
    }

    if let Some(suffix_word_len) =
        trigger_suffix_word_len(&words, PUT_INTO_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES)
    {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if is_source_reference_words(&subject_words) {
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter: ObjectFilter::source(),
                from: Zone::Battlefield,
                one_or_more,
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-a-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.owner = None;
        if subject_mentions_card(&subject_words) {
            filter.nontoken = true;
        }
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Battlefield,
            one_or_more,
        });
    }

    if let Some(suffix_word_len) = trigger_suffix_word_len(
        &words,
        PUT_INTO_OPPONENT_GRAVEYARD_FROM_BATTLEFIELD_SUFFIXES,
    ) {
        let subject_tokens =
            trigger_subject_tokens_before_suffix(tokens, words.len(), suffix_word_len);
        let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_view.to_word_refs();
        let one_or_more = subject_starts_one_or_more(&subject_words);
        if is_source_reference_words(&subject_words) {
            let mut filter = ObjectFilter::source();
            filter.owner = Some(PlayerFilter::Opponent);
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
                one_or_more,
            });
        }
        let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-opponents-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        filter.zone = None;
        filter.controller = None;
        filter.owner = Some(PlayerFilter::Opponent);
        return Ok(TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from: Zone::Battlefield,
            one_or_more,
        });
    }

    if let Some(put_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Put)
        && let Some(source_controller) = parse_trigger_subject_player_filter(&words[..put_word_idx])
        && let Some(counter_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Counter)
        && counter_word_idx > put_word_idx
        && words
            .get(counter_word_idx + 1..counter_word_idx + 2)
            .is_some_and(|preposition| {
                trigger_pattern_accepts(preposition, COUNTER_RECIPIENT_PREPOSITION_PATTERN)
            })
    {
        let descriptor_word_start = put_word_idx + 1;
        let (descriptor_span, counter_descriptor_tokens) = trigger_counter_descriptor_span(
            tokens,
            descriptor_word_start,
            counter_word_idx,
            &words,
        )?;
        let descriptor_words =
            ActivationRestrictionCompatWords::new(descriptor_span).to_word_refs();
        let one_or_more = trigger_pattern_accepts(&descriptor_words, ONE_OR_MORE_PREFIX_PATTERN);
        let counter_type = trigger_counter_type_from_descriptor(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 2;
        let object_tokens = trigger_counter_recipient_tokens(tokens, object_word_start, &words)?;
        let filter = parse_object_filter_lexed(&object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: Some(source_controller),
            one_or_more,
        });
    }

    if let Some(get_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Get)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..get_word_idx])
        && words.get(get_word_idx + 1..).is_some_and(|tail| {
            trigger_pattern_accepts(tail, PLAYER_GETS_ONE_OR_MORE_ENERGY_TAIL_PATTERN)
        })
    {
        return Ok(TriggerSpec::PlayerGetsCounters {
            player,
            counter_type: Some(CounterType::Energy),
            one_or_more: true,
        });
    }

    if let Some(get_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Get)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..get_word_idx])
        && let Some(counter_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Counter)
        && counter_word_idx > get_word_idx
    {
        let descriptor_word_start = get_word_idx + 1;
        let (descriptor_span, counter_descriptor_tokens) = trigger_counter_descriptor_span(
            tokens,
            descriptor_word_start,
            counter_word_idx,
            &words,
        )?;
        let descriptor_words =
            ActivationRestrictionCompatWords::new(descriptor_span).to_word_refs();
        let one_or_more = trigger_pattern_accepts(&descriptor_words, ONE_OR_MORE_PREFIX_PATTERN);
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        return Ok(TriggerSpec::PlayerGetsCounters {
            player,
            counter_type,
            one_or_more,
        });
    }

    if trigger_pattern_accepts(&words, PLAYERS_FINISH_VOTING_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if trigger_pattern_accepts(&words, YOU_CYCLE_THIS_CARD_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            player: PlayerFilter::You,
        });
    }

    if trigger_pattern_accepts(&words, YOU_CYCLE_OR_DISCARD_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Cycle,
                player: PlayerFilter::You,
                source_filter: None,
            }),
            Box::new(TriggerSpec::PlayerDiscardsCard {
                player: PlayerFilter::You,
                filter: None,
                cause_controller: None,
                effect_like_only: false,
                one_or_more: false,
            }),
        ));
    }

    if trigger_pattern_accepts(&words, YOU_COMMIT_CRIME_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if trigger_pattern_accepts(&words, OPPONENT_COMMITS_CRIME_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Opponent,
            source_filter: None,
        });
    }

    if trigger_pattern_accepts(&words, PLAYER_COMMITS_CRIME_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if let Some(trigger) = trigger_grammar::parse_fully_unlock_room_trigger(tokens) {
        return Ok(TriggerSpec::KeywordAction {
            action: trigger.action,
            player: trigger.player,
            source_filter: Some(trigger.source_filter),
        });
    }

    if trigger_pattern_accepts(&words, YOU_UNLOCK_THIS_DOOR_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::UnlockDoor,
            player: PlayerFilter::You,
        });
    }

    if trigger_pattern_accepts(&words, THIS_CARD_BECOMES_PLOTTED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Plot,
            player: PlayerFilter::You,
        });
    }

    if words.len() == 3
        && trigger_pattern_accepts(&words, YOU_EXPEND_TRIGGER_PREFIX)
        && let Some(amount) = parse_named_number(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::You,
            amount,
        });
    }

    if words.len() == 4
        && trigger_pattern_accepts(&words, OPPONENT_EXPENDS_WITH_ARTICLE_TRIGGER_PREFIX)
        && let Some(amount) = parse_named_number(words[3])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.len() == 3
        && trigger_pattern_accepts(&words, OPPONENT_EXPENDS_TRIGGER_PREFIX)
        && let Some(amount) = parse_named_number(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if trigger_pattern_accepts(&words, THE_RING_TEMPTS_YOU_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::RingTemptsYou,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if trigger_pattern_accepts(&words, CHAOS_ENSUES_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ChaosEnsues,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if let Some(cycle_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Cycle)
    {
        let subject_words = &words[..cycle_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let tail_words = &words[cycle_word_idx + 1..];
            if trigger_pattern_accepts(tail_words, CYCLE_CARD_TAIL_PATTERN) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: None,
                });
            }
            if trigger_pattern_accepts(tail_words, CYCLE_ANOTHER_CARD_TAIL_PATTERN) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: Some(ObjectFilter::default().other()),
                });
            }
        }
    }

    if let Some(exert_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Exert)
    {
        let subject = &words[..exert_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[exert_word_idx + 1..];
            if trigger_pattern_accepts(tail, EXERT_CREATURE_TAIL_PATTERN) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Exert,
                    player,
                    source_filter: Some(ObjectFilter::creature()),
                });
            }
        }
    }

    let (core_words, during_your_main_phase) =
        if trigger_pattern_accepts(&words, DURING_YOUR_MAIN_PHASE_SUFFIX_PATTERN) {
            (
                &words[..words.len() - DURING_YOUR_MAIN_PHASE_SUFFIX.len()],
                true,
            )
        } else {
            (words.as_slice(), false)
        };
    if let Some(saddle_word_idx) =
        trigger_keyword_action_word(core_words, crate::events::KeywordActionKind::Saddle)
        && let Some(or_word_idx) =
            trigger_atom_word(&core_words[saddle_word_idx + 1..], TriggerClauseAtom::Or)
                .map(|idx| saddle_word_idx + 1 + idx)
        && let Some(crew_word_idx) = trigger_keyword_action_word(
            &core_words[or_word_idx + 1..],
            crate::events::KeywordActionKind::Crew,
        )
        .map(|idx| or_word_idx + 1 + idx)
    {
        let subject_words = &core_words[..saddle_word_idx];
        let saddle_tail = &core_words[saddle_word_idx + 1..or_word_idx];
        let crew_tail = &core_words[crew_word_idx + 1..];
        if is_source_reference_words(subject_words)
            && trigger_pattern_accepts(saddle_tail, SADDLE_MOUNT_TAIL_PATTERN)
            && trigger_pattern_accepts(crew_tail, CREW_VEHICLE_TAIL_PATTERN)
        {
            let source_filter = source_reference_surface_for_words(subject_words)
                .or_else(|| this_source_surface_for_words(subject_words))
                .map(ObjectFilter::source_with_surface)
                .unwrap_or_else(ObjectFilter::source);
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::KeywordActionTaggedObject {
                    action: crate::events::KeywordActionKind::Saddle,
                    player: PlayerFilter::Any,
                    source_filter: source_filter.clone(),
                    object_tag: TagKey::from(IT_TAG),
                    object_filter: ObjectFilter::default()
                        .in_zone(Zone::Battlefield)
                        .with_subtype(Subtype::Mount),
                    during_your_main_phase,
                }),
                Box::new(TriggerSpec::KeywordActionTaggedObject {
                    action: crate::events::KeywordActionKind::Crew,
                    player: PlayerFilter::Any,
                    source_filter,
                    object_tag: TagKey::from(IT_TAG),
                    object_filter: ObjectFilter::default()
                        .in_zone(Zone::Battlefield)
                        .with_subtype(Subtype::Vehicle),
                    during_your_main_phase,
                }),
            ));
        }
    }

    if let Some(crew_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Crew)
    {
        let subject_words = &words[..crew_word_idx];
        let source_becomes_crewed = subject_words.last().is_some_and(|word| *word == "becomes")
            && is_source_reference_words(&subject_words[..subject_words.len().saturating_sub(1)]);
        let source_filter = if source_becomes_crewed {
            Some(ObjectFilter::default())
        } else if is_source_reference_words(subject_words) {
            Some(ObjectFilter::source())
        } else {
            let subject_end = word_view
                .token_index_after_words(crew_word_idx)
                .unwrap_or(crew_word_idx);
            parse_trigger_subject_filter_lexed(&tokens[..subject_end])?
        };
        if let Some(source_filter) = source_filter {
            let tail_start = word_view
                .token_index_after_words(crew_word_idx + 1)
                .unwrap_or(tokens.len());
            let tail_words = &words[crew_word_idx + 1..];
            let object_filter = if source_becomes_crewed {
                ObjectFilter::source().with_subtype(Subtype::Vehicle)
            } else if tail_words.is_empty()
                || trigger_pattern_accepts(tail_words, CREW_VEHICLE_TAIL_PATTERN)
            {
                ObjectFilter::default().with_subtype(Subtype::Vehicle)
            } else {
                let tail_tokens = trim_commas(tokens.get(tail_start..).unwrap_or_default());
                parse_object_filter_lexed(&tail_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported crew object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::KeywordActionTaggedObject {
                action: crate::events::KeywordActionKind::Crew,
                player: PlayerFilter::Any,
                source_filter,
                object_tag: TagKey::from(IT_TAG),
                object_filter,
                during_your_main_phase: false,
            });
        }
    }

    if let Some(explore_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Explore)
    {
        let subject_tokens = &tokens[..explore_word_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            let tail = &words[explore_word_idx + 1..];
            let revealed_filter = if tail.is_empty() {
                None
            } else if trigger_pattern_accepts(tail, EXPLORE_LAND_CARD_TAIL_PATTERN) {
                Some(ObjectFilter::default().with_type(crate::types::CardType::Land))
            } else if trigger_pattern_accepts(tail, EXPLORE_NONLAND_CARD_TAIL_PATTERN) {
                Some(ObjectFilter::default().without_type(crate::types::CardType::Land))
            } else {
                None
            };
            return Ok(match revealed_filter {
                Some(object_filter) => TriggerSpec::KeywordActionTaggedObject {
                    action: crate::events::KeywordActionKind::Explore,
                    player: PlayerFilter::Any,
                    source_filter: filter,
                    object_tag: TagKey::from("__public_revealed"),
                    object_filter,
                    during_your_main_phase: false,
                },
                None if tail.is_empty() => TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Explore,
                    player: PlayerFilter::Any,
                    source_filter: Some(filter),
                },
                None => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported explore trigger tail in trigger clause (clause: '{}')",
                        words.join(" ")
                    )));
                }
            });
        }
    }

    if let Some(fight_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Fight)
    {
        let subject_tokens = &tokens[..fight_word_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)?
            && words[fight_word_idx + 1..].is_empty()
        {
            return Ok(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Fight,
                player: PlayerFilter::Any,
                source_filter: Some(filter),
            });
        }
    }

    if let Some(put_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Put) {
        let subject = &words[..put_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[put_word_idx + 1..];
            if trigger_pattern_accepts(tail, NAME_STICKER_PUT_TAIL_PATTERN) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::NameSticker,
                    player,
                    source_filter: None,
                });
            }
        }
    }

    let becomes_tapped_words = if trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX) {
        &words[..words.len().saturating_sub(3)]
    } else {
        words.as_slice()
    };

    if trigger_pattern_accepts(becomes_tapped_words, BECOMES_TAPPED_TRIGGER_SUFFIX)
        && let Some(becomes_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Becomes)
    {
        let subject_tokens = &tokens[..becomes_idx];
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::PermanentBecomesTapped(filter),
            None => TriggerSpec::ThisBecomesTapped,
        });
    }

    if trigger_pattern_accepts(becomes_tapped_words, THIS_BECOMES_TAPPED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if trigger_pattern_accepts(&words, THIS_BECOMES_UNTAPPED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if trigger_pattern_accepts(&words, THIS_BECOMES_MONSTROUS_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }
    if words.len() == 5
        && trigger_word_at_accepts_pattern(&words, 0, THIS_WORD_PATTERN)
        && words[1].eq_ignore_ascii_case("class")
        && trigger_word_at_accepts_pattern(&words, 2, BECOMES_WORD_PATTERN)
        && words[3].eq_ignore_ascii_case("level")
        && parse_named_number(words[4]).is_some()
    {
        return Ok(TriggerSpec::CounterPutOn {
            filter: ObjectFilter::source(),
            counter_type: Some(CounterType::Level),
            source_controller: None,
            one_or_more: false,
        });
    }
    if trigger_pattern_accepts(&words, BECOMES_MONSTROUS_TRIGGER_SUFFIX)
        && words.len() > 2
        && source_reference_surface_for_words(&words[..words.len() - 2]).is_some()
    {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if trigger_pattern_accepts(&words, THIS_MUTATES_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisMutates);
    }
    if trigger_pattern_accepts(&words, MUTATES_TRIGGER_SUFFIX)
        && words.len() > 1
        && source_reference_surface_for_words(&words[..words.len() - 1]).is_some()
    {
        return Ok(TriggerSpec::ThisMutates);
    }

    if trigger_pattern_accepts(&words, THIS_TURNED_FACE_UP_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::ThisTurnedFaceUp);
    }

    if trigger_pattern_accepts(&words, TURNED_FACE_UP_TRIGGER_SUFFIX) {
        let subject_tokens = trigger_word_token_start(tokens, words.len().saturating_sub(4))
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::TurnedFaceUp(filter),
            None => TriggerSpec::ThisTurnedFaceUp,
        });
    }

    if let Some(becomes_idx) = trigger_atom_word(&words, TriggerClauseAtom::Becomes)
        && trigger_pattern_accepts(&words[becomes_idx + 1..], BECOMES_TARGET_OF_PREFIX_PATTERN)
    {
        let subject_words = &words[..becomes_idx];
        let subject_tokens = trigger_word_token_start(tokens, becomes_idx)
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        let subject_filter = parse_trigger_subject_filter_lexed(subject_tokens)?;
        let subject_is_source =
            subject_words.is_empty() || is_source_reference_words(subject_words);
        if subject_is_source {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words) {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: ObjectFilter::source(),
                    source_controller,
                });
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_ABILITY_TARGET_TAIL_PATTERN) {
                return Ok(TriggerSpec::ThisBecomesTargeted);
            }
            if trigger_pattern_accepts(tail_words, ONLY_IT_ABILITY_TARGET_TAIL_PATTERN) {
                let mut ability_filter = ObjectFilter::ability();
                ability_filter.target_count = Some(crate::effect::ChoiceCount::exactly(1));
                ability_filter.targets_only_object = Some(Box::new(ObjectFilter::source()));
                return Ok(TriggerSpec::ThisBecomesTargetedByStackObject(
                    ability_filter,
                ));
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_SPELLS_SUFFIX_PATTERN) {
                let tail_token_start =
                    trigger_word_token_start(tokens, tail_word_start).unwrap_or(tokens.len());
                let spell_filter_tokens = trim_commas(&tokens[tail_token_start..]);
                let spell_filter =
                    parse_object_filter_lexed(&spell_filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported spell filter in becomes-targeted trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                return Ok(TriggerSpec::ThisBecomesTargetedBySpell(spell_filter));
            }
        } else {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words)
                && let Some(subject) =
                    trigger_grammar::parse_you_or_controlled_object_subject_words(subject_words)
            {
                return Ok(
                    TriggerSpec::PlayerOrObjectBecomesTargetedBySourceController {
                        player: subject.player,
                        object: subject.filter,
                        source_controller,
                    },
                );
            }
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words)
                && let Some(filter) = subject_filter.clone()
            {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: filter,
                    source_controller,
                });
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_ABILITY_TARGET_TAIL_PATTERN)
                && let Some(filter) = subject_filter
            {
                return Ok(TriggerSpec::BecomesTargeted(filter));
            }
            if trigger_pattern_accepts(tail_words, BACKUP_ABILITY_TARGET_TAIL_PATTERN)
                && let Some(filter) = subject_filter
            {
                let ability_filter = ObjectFilter::ability().with_ability_marker("backup");
                return Ok(TriggerSpec::BecomesTargetedByStackObject {
                    target: filter,
                    stack_object: ability_filter,
                });
            }
            if trigger_pattern_accepts(tail_words, SPELL_OR_SPELLS_SUFFIX_PATTERN)
                && let Some(filter) = subject_filter
            {
                let tail_token_start =
                    trigger_word_token_start(tokens, tail_word_start).unwrap_or(tokens.len());
                let spell_filter_tokens = trim_commas(&tokens[tail_token_start..]);
                let spell_filter =
                    parse_object_filter_lexed(&spell_filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported spell filter in becomes-targeted trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                return Ok(TriggerSpec::BecomesTargetedByStackObject {
                    target: filter,
                    stack_object: spell_filter,
                });
            }
        }
    }

    if let Some((is_word_idx, dealt_combat_damage)) = dealt_damage_suffix_subject_word_idx(&words)
        && !trigger_pattern_accepts(&words, SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX)
    {
        let is_token_idx = trigger_word_token_start(tokens, is_word_idx).unwrap_or(tokens.len());
        if is_word_idx == 0
            && words.first().is_some_and(|word| {
                trigger_word_accepts_pattern(word, YOU_CONTRACTION_WORD_PATTERN)
            })
        {
            return Ok(TriggerSpec::DealsDamageToPlayer {
                source: ObjectFilter::default(),
                player: PlayerFilter::You,
            });
        }
        let subject_tokens = &tokens[..is_token_idx];
        if let Some(player) = trigger_subject_player_selector_lexed(subject_tokens) {
            return Ok(TriggerSpec::DealsDamageToPlayer {
                source: ObjectFilter::default(),
                player,
            });
        }
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            if dealt_combat_damage {
                return Ok(TriggerSpec::IsDealtCombatDamage(filter));
            }
            return Ok(TriggerSpec::IsDealtDamage(filter));
        }
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX) {
        if trigger_pattern_accepts(&words, SOURCE_DEALT_COMBAT_DAMAGE_TRIGGER_PREFIX) {
            return Ok(TriggerSpec::ThisIsDealtCombatDamage);
        }
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALS_TRIGGER_PREFIX)
        && let Some(deals_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Deal)
        && let Some(damage_idx_rel) =
            trigger_atom_token(&tokens[deals_idx + 1..], TriggerClauseAtom::Damage)
    {
        let damage_idx = deals_idx + 1 + damage_idx_rel;
        if let Some(to_idx_rel) =
            trigger_atom_token(&tokens[damage_idx + 1..], TriggerClauseAtom::To)
        {
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_tokens = trim_commas(&tokens[deals_idx + 1..damage_idx]);
            if !amount_tokens
                .first()
                .is_some_and(|token| token_matches_clause_shape(token, COMBAT_WORD_PATTERN))
            {
                let amount_view = ActivationRestrictionCompatWords::new(&amount_tokens);
                let amount_words = amount_view.to_word_refs();
                if let Some((amount, _)) =
                    parse_filter_comparison_tokens("damage amount", &amount_words, &words)?
                {
                    let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
                    let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
                    let target_words = target_view.to_word_refs();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                            player,
                            amount: Some(amount),
                        });
                    }
                }
            }
        }
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALS_DAMAGE_TO_TRIGGER_PREFIX)
        && let Some(to_idx) = trigger_atom_token(tokens, TriggerClauseAtom::To)
    {
        let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
        let target_words = target_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
            return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                player,
                amount: None,
            });
        }
        let target_filter = parse_object_filter_lexed(&target_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::ThisDealsDamageTo(target_filter));
    }

    if trigger_pattern_accepts(&words, SOURCE_DEALS_DAMAGE_TRIGGER_PREFIX) {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    if has_deal
        && trigger_pattern_accepts(&words, DAMAGE_WORD_PATTERN)
        && let Some(deals_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Deal)
    {
        let subject_tokens = &tokens[..deals_idx];
        if let Some(damage_idx_rel) =
            trigger_atom_token(&tokens[deals_idx + 1..], TriggerClauseAtom::Damage)
            && let Some(to_idx_rel) = trigger_atom_token(
                &tokens[deals_idx + 1 + damage_idx_rel + 1..],
                TriggerClauseAtom::To,
            )
        {
            let damage_idx = deals_idx + 1 + damage_idx_rel;
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_words =
                ActivationRestrictionCompatWords::new(&tokens[deals_idx + 1..damage_idx])
                    .to_word_refs();
            let target_tokens = split_target_clause_before_comma(&tokens[to_idx + 1..]);
            let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
            let target_words = target_view.to_word_refs();
            if trigger_pattern_accepts(&amount_words, NONCOMBAT_DAMAGE_AMOUNT_PATTERN)
                && let Some(player) = parse_trigger_subject_player_filter(&target_words)
                && let Some(source) = parse_trigger_subject_filter_lexed(subject_tokens)?
            {
                return Ok(TriggerSpec::DealsNoncombatDamageToPlayer {
                    source,
                    player,
                    source_surface:
                        crate::runtime_backend::grammar::trigger_subjects::parse_damage_source_surface(
                            subject_tokens,
                        ),
                });
            }
            if let Some(player) = parse_trigger_subject_player_filter(&target_words)
                && let Some(source) = parse_trigger_subject_filter_lexed(subject_tokens)?
            {
                return Ok(TriggerSpec::DealsDamageToPlayer { source, player });
            }
            if let Ok(target) = parse_object_filter_lexed(&target_tokens, false)
                && let Some(source) = parse_trigger_subject_filter_lexed(subject_tokens)?
            {
                return Ok(TriggerSpec::DealsDamageTo {
                    source,
                    target,
                    source_surface:
                        crate::runtime_backend::grammar::trigger_subjects::parse_damage_source_surface(
                            subject_tokens,
                        ),
                });
            }
        }
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::DealsDamage(filter),
            None => TriggerSpec::ThisDealsDamage,
        });
    }

    if trigger_pattern_accepts(&words, YOU_GAIN_LIFE_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.len() >= 6
        && trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX)
        && trigger_pattern_accepts(&words[..words.len() - 3], YOU_GAIN_LIFE_PREFIX_PATTERN)
    {
        return Ok(TriggerSpec::YouGainLifeDuringTurn(PlayerFilter::You));
    }

    if let Some(amount) = trigger_grammar::parse_opponents_each_lose_exact_life_words(&words) {
        return Ok(TriggerSpec::OpponentsEachLoseExactLife { amount });
    }

    if let Some(clause) = trigger_grammar::parse_players_lose_life_one_or_more_clause(tokens) {
        return Ok(TriggerSpec::PlayersLoseLifeOneOrMore(clause.player));
    }

    if trigger_pattern_accepts(&words, LOSE_LIFE_TRIGGER_SUFFIX) {
        let subject = &words[..words.len().saturating_sub(2)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLife(player));
        }
    }

    if trigger_pattern_accepts(&words, LOSE_GAME_TRIGGER_SUFFIX) {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesGame(player));
        }
    }

    if words.len() >= 5
        && trigger_pattern_accepts(&words, DURING_YOUR_TURN_TRIGGER_SUFFIX)
        && trigger_pattern_accepts(&words[..words.len() - 3], LOSE_LIFE_TRIGGER_SUFFIX)
    {
        let subject = &words[..words.len() - 5];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLifeDuringTurn {
                player,
                during_turn: PlayerFilter::You,
            });
        }
    }

    if let Some(draw_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Draw) {
        let subject = &words[..draw_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[draw_word_idx + 1..];
            if let Some(during_turn) =
                trigger_grammar::parse_not_during_turn_draw_suffix_words(tail)
            {
                return Ok(TriggerSpec::PlayerDrawsCardNotDuringTurn {
                    player,
                    during_turn,
                });
            }
            if has_draw_except_first_in_draw_step_pattern(tail) {
                return Ok(TriggerSpec::PlayerDrawsCardExceptFirstInDrawStep(player));
            }
            if let Some(card_number) = parse_exact_draw_count_each_turn(tail) {
                return Ok(TriggerSpec::PlayerDrawsNthCardEachTurn {
                    player,
                    card_number,
                });
            }
        }
    }

    if trigger_pattern_accepts(&words, DRAW_A_CARD_TRIGGER_SUFFIX) {
        let subject = &words[..words.len().saturating_sub(3)];
        if trigger_pattern_accepts(subject, YOU_DRAW_CARD_TRIGGER_SUBJECT_PATTERN) {
            return Ok(TriggerSpec::YouDrawCard);
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerDrawsCard(player));
        }
    }

    if trigger_pattern_accepts(&words, OPPONENT_EFFECT_DISCARDS_THIS_CARD_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::PlayerDiscardsCard {
            player: PlayerFilter::You,
            filter: Some(ObjectFilter::source()),
            cause_controller: Some(PlayerFilter::Opponent),
            effect_like_only: true,
            one_or_more: false,
        });
    }

    if let Some(discard_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Discard)
        && let Some(discard_token_idx) = trigger_word_token_start(tokens, discard_word_idx)
    {
        let subject_words = &words[..discard_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            if let Ok(filter) =
                parse_discard_trigger_card_filter(&tokens[discard_token_idx + 1..], &words)
            {
                let tail_words = ActivationRestrictionCompatWords::new(
                    tokens.get(discard_token_idx + 1..).unwrap_or_default(),
                )
                .to_word_refs();
                let one_or_more = trigger_grammar::find_trigger_surface_window(
                    &tail_words,
                    3,
                    ONE_OR_MORE_QUANTIFIER_PATTERN,
                )
                .is_some();
                return Ok(TriggerSpec::PlayerDiscardsCard {
                    player,
                    filter,
                    cause_controller: None,
                    effect_like_only: false,
                    one_or_more,
                });
            }
        }
    }

    if let Some(reveal_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Reveal)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..reveal_word_idx])
    {
        let mut tail_tokens = trim_commas(
            &tokens
                [trigger_word_token_start(tokens, reveal_word_idx + 1).unwrap_or(tokens.len())..],
        );
        let tail_view = ActivationRestrictionCompatWords::new(&tail_tokens);
        let tail_words = tail_view.to_word_refs();
        let from_source = trigger_pattern_accepts(&tail_words, THIS_WAY_REVEAL_TAIL_PATTERN);
        if from_source {
            let cutoff = trigger_word_token_start(&tail_tokens, tail_words.len().saturating_sub(2))
                .unwrap_or(tail_tokens.len());
            tail_tokens = trim_commas(&tail_tokens[..cutoff]);
        }
        if !tail_tokens.is_empty()
            && let Ok(mut filter) = parse_object_filter_lexed(&tail_tokens, false)
        {
            filter.zone = None;
            return Ok(TriggerSpec::PlayerRevealsCard {
                player,
                filter,
                from_source,
            });
        }
    }

    if let Some(sacrifice_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Sacrifice)
        && let Some(sacrifice_token_idx) = trigger_word_token_start(tokens, sacrifice_word_idx)
    {
        let subject_words = &words[..sacrifice_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let mut filter_tokens = &tokens[sacrifice_token_idx + 1..];
            let mut other = false;
            if filter_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                filter_tokens = &filter_tokens[1..];
            }

            let filter = if filter_tokens.is_empty() {
                let mut filter = ObjectFilter::permanent();
                if other {
                    filter.other = true;
                }
                filter
            } else if filter_tokens
                .first()
                .is_some_and(|token| token_matches_clause_shape(token, THIS_OR_IT_PATTERN))
            {
                let filter_word_view = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_word_view.to_word_refs();
                let mut filter = ObjectFilter::source();
                let is_artifact =
                    trigger_pattern_accepts(&filter_words, SOURCE_ARTIFACT_WORD_PATTERN);
                let is_creature =
                    trigger_pattern_accepts(&filter_words, SOURCE_CREATURE_WORD_PATTERN);
                let is_enchantment =
                    trigger_pattern_accepts(&filter_words, SOURCE_ENCHANTMENT_WORD_PATTERN);
                let is_land = trigger_pattern_accepts(&filter_words, SOURCE_LAND_WORD_PATTERN);
                let is_planeswalker =
                    trigger_pattern_accepts(&filter_words, SOURCE_PLANESWALKER_WORD_PATTERN);
                if is_artifact {
                    filter = filter.with_type(CardType::Artifact);
                } else if is_creature {
                    filter = filter.with_type(CardType::Creature);
                } else if is_enchantment {
                    filter = filter.with_type(CardType::Enchantment);
                } else if is_land {
                    filter = filter.with_type(CardType::Land);
                } else if is_planeswalker {
                    filter = filter.with_type(CardType::Planeswalker);
                }
                filter
            } else {
                parse_object_filter_lexed(filter_tokens, other).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported sacrifice trigger filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::PlayerSacrifices { player, filter });
        }
    }

    if let Some(roll_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Roll) {
        let subject_words = &words[..roll_word_idx];
        let result_words = &words[roll_word_idx + 1..];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            use crate::runtime_backend::grammar::trigger_clauses::RollResultShape;
            match trigger_grammar::parse_roll_result_words(result_words) {
                Some(RollResultShape::HighestNatural) => {
                    return Ok(TriggerSpec::PlayerRollsHighestNaturalResult { player });
                }
                Some(RollResultShape::Fixed(result)) => {
                    return Ok(TriggerSpec::PlayerRollsResult { player, result });
                }
                Some(RollResultShape::UnspecifiedDie) => {
                    return Ok(TriggerSpec::PlayerRollsDie { player });
                }
                None => {}
            }
        }
    }

    if let Some(last_word) = words.last().copied()
        && let Some(action) = crate::events::KeywordActionKind::from_trigger_word(last_word)
    {
        let subject = &words[..words.len().saturating_sub(1)];
        if is_source_reference_words(subject) {
            return Ok(TriggerSpec::KeywordActionFromSource {
                action,
                player: PlayerFilter::You,
            });
        }
        if subject.len() > 2 && is_source_reference_words(&subject[..2]) {
            let trailing_ok = subject[2..].iter().all(|word| {
                trigger_word_accepts_pattern(word, SOURCE_KEYWORD_ACTION_TRAILING_WORD_PATTERN)
            });
            if trailing_ok {
                return Ok(TriggerSpec::KeywordActionFromSource {
                    action,
                    player: PlayerFilter::You,
                });
            }
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::KeywordAction {
                action,
                player,
                source_filter: None,
            });
        }
    }

    if trigger_pattern_accepts(&words, YOU_OPEN_ATTRACTION_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::OpenAttraction,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if trigger_pattern_accepts(&words, YOU_CLAIM_ATTRACTION_PRIZE_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ClaimAttractionPrize,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if let Some(exploit_word_idx) =
        trigger_keyword_action_word(&words, crate::events::KeywordActionKind::Exploit)
    {
        let subject_words = &words[..exploit_word_idx];
        let tail_words = &words[exploit_word_idx + 1..];
        if !is_source_reference_words(subject_words) {
            let subject_end = word_view
                .token_index_after_words(exploit_word_idx)
                .unwrap_or(exploit_word_idx);
            let tail_start = word_view
                .token_index_after_words(exploit_word_idx + 1)
                .unwrap_or(tokens.len());
            let tail_tokens = tokens.get(tail_start..).unwrap_or_default();
            let object_filter = if tail_words.is_empty()
                || trigger_pattern_accepts(tail_words, EXPLOIT_CREATURE_TAIL_PATTERN)
            {
                None
            } else {
                Some(parse_object_filter_lexed(tail_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported exploit object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?)
            };
            let subject_tokens = &tokens[..subject_end];
            if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
                return Ok(match object_filter {
                    Some(object_filter) => TriggerSpec::KeywordActionTaggedObject {
                        action: crate::events::KeywordActionKind::Exploit,
                        player: PlayerFilter::Any,
                        source_filter: filter,
                        object_tag: TagKey::from(crate::tag::EXPLOITED_TAG),
                        object_filter,
                        during_your_main_phase: false,
                    },
                    None => TriggerSpec::KeywordAction {
                        action: crate::events::KeywordActionKind::Exploit,
                        player: PlayerFilter::Any,
                        source_filter: Some(filter),
                    },
                });
            }
        }
    }

    if trigger_pattern_accepts(&words, THIS_EXPLOITS_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Exploit,
            player: PlayerFilter::You,
        });
    }

    if trigger_pattern_accepts(&words, YOU_COMPLETE_DUNGEON_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CompleteDungeon,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if trigger_pattern_accepts(&words, WINS_CLASH_TRIGGER_SUFFIX_PATTERN) {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::WinsClash { player });
        }
    }

    if let Some(counter_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Counter)
        && trigger_pattern_accepts(&words[counter_word_idx..], PASSIVE_COUNTER_PUT_TAIL_PATTERN)
    {
        let one_or_more = trigger_pattern_accepts(&words, ONE_OR_MORE_PREFIX_PATTERN);
        let descriptor_token_end =
            trigger_word_token_start(tokens, counter_word_idx).unwrap_or(tokens.len());
        let counter_descriptor_tokens = &tokens[..(descriptor_token_end + 1)];
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 4;
        let object_tokens = trigger_counter_recipient_tokens(tokens, object_word_start, &words)?;
        let filter = parse_object_filter_lexed(&object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: None,
            one_or_more,
        });
    }

    if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
        let tail_words = &words[attacks_word_idx + 1..];
        if trigger_pattern_accepts(tail_words, ATTACKS_AND_IS_NOT_BLOCKED_TAIL_PATTERN) {
            let attacks_token_idx =
                trigger_word_token_start(tokens, attacks_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            return Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(filter) => TriggerSpec::AttacksAndIsntBlocked(filter),
                    None => TriggerSpec::ThisAttacksAndIsntBlocked,
                },
            );
        }
    }

    if trigger_pattern_accepts(&words, THIS_BLOCKS_OR_BECOMES_BLOCKED_TRIGGER_PATTERN) {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::ThisBlocks),
            Box::new(TriggerSpec::ThisBecomesBlocked),
        ));
    }

    if trigger_pattern_accepts(&words, THIS_BECOMES_BLOCKED_BY_TRIGGER_PREFIX)
        && let Some(by_idx) = trigger_atom_token(tokens, TriggerClauseAtom::By)
    {
        let blocker_tokens = trim_commas(&tokens[by_idx + 1..]);
        if !blocker_tokens.is_empty() {
            let blocker_filter =
                parse_object_filter_lexed(&blocker_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported blocking-object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            return Ok(TriggerSpec::ThisBecomesBlockedByObject(blocker_filter));
        }
    }

    if trigger_pattern_accepts(&words, THIS_BLOCKS_OR_BECOMES_BLOCKED_BY_TRIGGER_PREFIX)
        && let Some(by_idx) = trigger_atom_token(tokens, TriggerClauseAtom::By)
    {
        let blocker_tokens = trim_commas(&tokens[by_idx + 1..]);
        if !blocker_tokens.is_empty() {
            let blocker_filter =
                parse_object_filter_lexed(&blocker_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported blocking-object filter in trigger clause (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::ThisBlocksObject(blocker_filter.clone())),
                Box::new(TriggerSpec::ThisBecomesBlockedByObject(blocker_filter)),
            ));
        }
    }

    if trigger_pattern_accepts(&words, THIS_BLOCKS_PREFIX_PATTERN)
        && let Some(blocks_idx) = trigger_atom_token(tokens, TriggerClauseAtom::Block)
    {
        let tail_tokens = trim_commas(&tokens[blocks_idx + 1..]);
        if !tail_tokens.is_empty() && !token_slice_at_is(&tail_tokens, 0, "or") {
            let blocked_filter = parse_object_filter_lexed(&tail_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported blocked-object filter in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            return Ok(TriggerSpec::ThisBlocksObject(blocked_filter));
        }
    }

    if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
        let subject_words = &words[..attacks_word_idx];
        let tail = &words[attacks_word_idx + 1..];
        if matches!(subject_words.first(), Some(&"this") | Some(&"it"))
            && let Some((count, filter)) = parse_attacks_player_who_controls_at_least_tail(tail)
        {
            return Ok(TriggerSpec::ThisAttacksPlayerWhoControlsAtLeast { count, filter });
        }
    }

    let (words, attacked_player_filter, attacked_target_must_be_player) =
        if let Some(attacks_word_idx) = trigger_atom_word(&words, TriggerClauseAtom::Attack) {
            let tail = &words[attacks_word_idx + 1..];
            if trigger_pattern_accepts(tail, ATTACKS_A_PLAYER_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::Any), true)
            } else if trigger_pattern_accepts(tail, ATTACKS_YOU_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::You), true)
            } else if trigger_pattern_accepts(tail, ATTACKS_OPPONENT_TAIL_PATTERN) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::Opponent),
                    true,
                )
            } else if trigger_pattern_accepts(tail, ATTACKS_DEFENDING_PLAYER_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::Any), true)
            } else if trigger_pattern_accepts(tail, ATTACKS_OPPONENT_OR_PLANESWALKER_TAIL_PATTERN) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::Opponent),
                    false,
                )
            } else if trigger_pattern_accepts(tail, ATTACKS_PLANESWALKER_OR_BATTLE_TAIL_PATTERN) {
                (&words[..=attacks_word_idx], None, false)
            } else {
                (&words[..], None, false)
            }
        } else {
            (&words[..], None, false)
        };

    let last = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    if let Some(attacked) =
        crate::runtime_backend::grammar::trigger_clauses::parse_players_attacked_clause(tokens)
    {
        let attacked_player_words =
            crate::runtime_backend::token_word_refs(&tokens[attacked.player]);
        if let Some(player_filter) = parse_trigger_subject_player_filter(&attacked_player_words) {
            return Ok(TriggerSpec::PlayersAttackedOneOrMore(player_filter));
        }
    }

    if last == "blocked" && words.len() >= 2 && words[words.len().saturating_sub(2)] == "becomes" {
        let becomes_word_idx = words.len().saturating_sub(2);
        let becomes_token_idx =
            trigger_word_token_start(tokens, becomes_word_idx).unwrap_or(tokens.len());
        let subject_tokens = &tokens[..becomes_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::BecomesBlocked(filter),
                None => TriggerSpec::ThisBecomesBlocked,
            },
        );
    }

    match last {
        "attack" | "attacks" => {
            let attack_word_idx = words.len().saturating_sub(1);
            let attack_token_idx =
                trigger_word_token_start(tokens, attack_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attack_token_idx];
            if let Some(and_idx) = trigger_atom_token(subject_tokens, TriggerClauseAtom::And) {
                let left = trim_edge_punctuation(&subject_tokens[..and_idx]);
                let right = trim_edge_punctuation(&subject_tokens[and_idx + 1..]);
                if !left.is_empty()
                    && token_slice_at_is(&right, 0, "at")
                    && token_slice_at_is(&right, 1, "least")
                    && let Some((other_count, used)) = parse_number(&right[2..])
                    && right
                        .get(2 + used)
                        .is_some_and(|token| token.is_word("other"))
                    && !right[3 + used..].is_empty()
                    && let Some(other_filter) =
                        parse_attack_trigger_subject_filter_lexed(&right[3 + used..])?
                {
                    let rendered_subject = crate::runtime_backend::lexer::render_token_slice(&left)
                        .trim()
                        .to_string();
                    let display_subject = if rendered_subject == "this" {
                        current_source_reference_name()
                    } else {
                        Some(rendered_subject)
                    }
                    .filter(|subject| !subject.is_empty());
                    return Ok(TriggerSpec::ThisAttacksWithNOthers {
                        other_count,
                        display_subject,
                        other_filter: Some(other_filter),
                    });
                }
            }
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
            let one_or_more =
                trigger_pattern_accepts(&subject_words.to_word_refs(), ONE_OR_MORE_PREFIX_PATTERN)
                    || player_subject;
            Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(mut filter) => {
                        if let Some(player_filter) = attacked_player_filter.clone() {
                            filter.attacking_player_or_planeswalker_controlled_by =
                                Some(player_filter.clone());
                            if attacked_target_must_be_player {
                                filter.targets_only_player = Some(player_filter);
                            }
                        }
                        if one_or_more {
                            TriggerSpec::AttacksOneOrMore(filter)
                        } else {
                            TriggerSpec::Attacks(filter)
                        }
                    }
                    None => TriggerSpec::ThisAttacks,
                },
            )
        }
        "block" | "blocks" => {
            let block_word_idx = words.len().saturating_sub(1);
            let block_token_idx =
                trigger_word_token_start(tokens, block_word_idx).unwrap_or(tokens.len());
            let subject_tokens = &tokens[..block_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) if one_or_more => TriggerSpec::BlocksOneOrMore(filter),
                Some(filter) => TriggerSpec::Blocks(filter),
                None => TriggerSpec::ThisBlocks,
            })
        }
        "dies" | "die" => {
            let dies_word_idx = words.len().saturating_sub(1);
            let dies_token_idx =
                trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, THIS_DESTINATION_TRIGGER_NAME_PATTERN)
            }) {
                let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
                let subject_words = subject_word_view.to_word_refs();
                if let Some(or_word_idx) =
                    find_phrase_shape(&subject_words, OR_ANOTHER_WORDS.len(), OR_ANOTHER_PATTERN)
                {
                    let rhs_word_idx = or_word_idx + 2;
                    let rhs_token_idx = trigger_word_token_start(subject_tokens, rhs_word_idx)
                        .unwrap_or(subject_tokens.len());
                    if rhs_token_idx < subject_tokens.len() {
                        let rhs_tokens = trim_edge_punctuation(&subject_tokens[rhs_token_idx..]);
                        if !rhs_tokens.is_empty()
                            && let Ok(filter) = parse_object_filter_lexed(&rhs_tokens, false)
                        {
                            return Ok(TriggerSpec::Either(
                                Box::new(TriggerSpec::ThisDies),
                                Box::new(TriggerSpec::Dies(filter)),
                            ));
                        }
                    }
                }
                if is_source_reference_words(&subject_words) {
                    return Ok(TriggerSpec::ThisDies);
                }
                return Err(CardTextError::ParseError(format!(
                    "unsupported this-prefixed dies trigger subject (clause: '{}')",
                    words.join(" ")
                )));
            }

            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if trigger_pattern_accepts(&subject_words, THE_CREATURE_HAUNTS_PATTERN) {
                return Ok(TriggerSpec::HauntedCreatureDies);
            }

            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }

            if let Some(damaged_by_trigger) =
                parse_damage_by_dies_trigger_lexed(subject_tokens, other, &words)?
            {
                return Ok(damaged_by_trigger);
            }

            if let Ok(filter) = parse_object_filter_lexed(subject_tokens, other) {
                return Ok(if one_or_more {
                    TriggerSpec::DiesOneOrMore(filter)
                } else {
                    TriggerSpec::Dies(filter)
                });
            }
            let mut normalized_subject_tokens = Vec::with_capacity(subject_tokens.len());
            let mut idx = 0usize;
            while idx < subject_tokens.len() {
                if token_matches_clause_shape(&subject_tokens[idx], AND_WORD_PATTERN)
                    && subject_tokens
                        .get(idx + 1)
                        .is_some_and(|token| token_matches_clause_shape(token, OR_WORD_PATTERN))
                {
                    idx += 1;
                    continue;
                }
                normalized_subject_tokens.push(subject_tokens[idx].clone());
                idx += 1;
            }
            if normalized_subject_tokens.len() != subject_tokens.len()
                && let Ok(filter) = parse_object_filter_lexed(&normalized_subject_tokens, other)
            {
                return Ok(if one_or_more {
                    TriggerSpec::DiesOneOrMore(filter)
                } else {
                    TriggerSpec::Dies(filter)
                });
            }

            Err(CardTextError::ParseError(format!(
                "unsupported dies trigger subject filter (clause: '{}')",
                words.join(" ")
            )))
        }
        "turn" if words.len() >= 3 && trigger_pattern_accepts(&words, DIES_THIS_TURN_SUFFIX) => {
            let dies_word_idx = words.len().saturating_sub(3);
            let dies_token_idx =
                trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies-this-turn trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }
            let mut filter =
                parse_trigger_subject_filter_lexed(subject_tokens)?.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported dies-this-turn trigger subject filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?;
            if other {
                filter.other = true;
            }
            Ok(if one_or_more {
                TriggerSpec::DiesOneOrMore(filter)
            } else {
                TriggerSpec::Dies(filter)
            })
        }
        "turn"
            if words.len() >= 4
                && trigger_pattern_accepts(&words, DIES_DURING_YOUR_TURN_SUFFIX) =>
        {
            let dies_word_idx = words.len().saturating_sub(4);
            let dies_token_idx =
                trigger_word_token_start(tokens, dies_word_idx).unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens.first().is_some_and(|token| {
                token_matches_clause_shape(token, OTHER_OR_ANOTHER_EXACT_PATTERN)
            }) {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies-during-turn trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }
            let filter = parse_object_filter_lexed(subject_tokens, other).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported dies-during-turn trigger subject filter (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            Ok(TriggerSpec::DiesDuringTurn {
                filter,
                one_or_more,
                during_turn: PlayerFilter::You,
            })
        }
        _ if trigger_pattern_accepts(&words, BEGINNING_END_STEP_TRIGGER_PATTERN)
            && !trigger_pattern_accepts(&words, NEXT_END_STEP_TRIGGER_PATTERN) =>
        {
            Ok(TriggerSpec::BeginningOfEndStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if trigger_pattern_accepts(&words, BEGINNING_UPKEEP_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfUpkeep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if trigger_pattern_accepts(&words, BEGINNING_DRAW_STEP_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfDrawStep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if trigger_pattern_accepts(&words, BEGINNING_FIRST_MAIN_PHASE_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfPrecombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if trigger_pattern_accepts(&words, BEGINNING_SECOND_MAIN_PHASE_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfPostcombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if trigger_pattern_accepts(&words, BEGINNING_PRECOMBAT_MAIN_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfPrecombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if trigger_pattern_accepts(&words, BEGINNING_POSTCOMBAT_MAIN_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfPostcombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if trigger_pattern_accepts(&words, BEGINNING_COMBAT_TRIGGER_PATTERN) => Ok(
            TriggerSpec::BeginningOfCombat(parse_possessive_clause_player_filter(&words)),
        ),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported trigger clause (clause: '{}')",
            words.join(" ")
        ))),
    }
}

fn parse_loyalty_ability_trigger_tail_lexed(
    tail_tokens: &[OwnedLexToken],
    tail_words: &[&str],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(tail) = trigger_grammar::parse_loyalty_ability_tail(tail_tokens) else {
        return Ok(None);
    };
    let owner_tokens = &tail_tokens[tail.owner];
    let owner_filter = parse_object_filter_lexed(&owner_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported loyalty-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    Ok(Some(owner_filter))
}

fn parse_possessive_ability_trigger_tail_lexed(
    tail_tokens: &[OwnedLexToken],
    tail_words: &[&str],
) -> Result<Option<(ObjectFilter, Option<String>)>, CardTextError> {
    let Some(tail) = trigger_grammar::parse_possessive_ability_tail(tail_tokens) else {
        return Ok(None);
    };
    let owner_subject_tokens = &tail_tokens[tail.owner];
    let owner_filter = parse_object_filter_lexed(owner_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;

    Ok(Some((owner_filter, tail.marker)))
}

fn parse_ability_of_object_trigger_tail_lexed(
    tail_tokens: &[OwnedLexToken],
    tail_words: &[&str],
) -> Result<Option<(ObjectFilter, bool)>, CardTextError> {
    let Some(tail) = trigger_grammar::parse_ability_of_object_tail(tail_tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(&tail_tokens[tail.filter], false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    Ok(Some((filter, tail.non_mana_only)))
}

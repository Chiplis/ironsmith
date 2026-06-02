use super::*;

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

const NOT_ITERATED_PLAYER_TURN_DRAW_TRIGGER_SUFFIX: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "card", "if", "it", "isnt", "that", "players", "turn"],
            &["a", "card", "if", "its", "not", "that", "players", "turn"],
            &["a", "card", "if", "it", "isnt", "their", "turn"],
            &["a", "card", "if", "its", "not", "their", "turn"],
        ]
);
const NOT_YOUR_TURN_DRAW_TRIGGER_SUFFIX: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "card", "if", "it", "isnt", "your", "turn"],
            &["a", "card", "if", "its", "not", "your", "turn"],
        ]
);
const NOT_OPPONENTS_TURN_DRAW_TRIGGER_SUFFIX: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "card", "if", "it", "isnt", "an", "opponents", "turn",],
            &["a", "card", "if", "its", "not", "an", "opponents", "turn",],
            &["a", "card", "if", "it", "isnt", "opponents", "turn"],
            &["a", "card", "if", "its", "not", "opponents", "turn"],
        ]
);

const ENTERS_FROM_YOUR_GRAVEYARD_ORIGIN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "your", "graveyard"]);
const ENTERS_FROM_GRAVEYARD_ORIGIN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "graveyard"]);
const ENTERS_FROM_YOUR_HAND_ORIGIN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "your", "hand"]);
const ENTERS_FROM_HAND_ORIGIN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "hand"]);
const ENTERS_FROM_EXILE_ORIGIN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["from", "exile"]);

const SOURCE_TRIGGER_CREATURE_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["creature"]);
const SOURCE_TRIGGER_LAND_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["land"]);
const SOURCE_TRIGGER_ARTIFACT_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["artifact"]);
const SOURCE_TRIGGER_ENCHANTMENT_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["enchantment"]);
const SOURCE_TRIGGER_PLANESWALKER_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["planeswalker"]);
const SOURCE_TRIGGER_BATTLE_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["battle"]);

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
        ]
);
const ATTACKS_A_PLAYER_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["a", "player"]);
const ATTACKS_OPPONENT_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["an", "opponent"],
            &["opponent"],
            &["one", "of", "your", "opponents"],
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
const BECOMES_TARGET_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "target", "of"]);
const SPELL_OR_SPELLS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix_any & [&["spell"], &["spells"]]);
const SPELL_OR_SPELLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["spell", "spells"]]);
const SPELL_OR_SPELLS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell"], &["spells"]]);
const ABILITY_OR_ABILITIES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["ability"], &["abilities"]]);
const CAST_OR_CASTS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["cast"], &["casts"]]);
const COPY_OR_COPIES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["copy"], &["copies"]]);
const DEAL_OR_DEALS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["deal"], &["deals"]]);
const ENTER_OR_ENTERS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enter"], &["enters"]]);
const ATTACK_OR_ATTACKS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["attack"], &["attacks"]]);
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
const LEAVE_OR_LEAVES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["leave"], &["leaves"]]);
const DIE_OR_DIES_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["die"], &["dies"]]);
const TRANSFORM_OR_TRANSFORMS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["transform"], &["transforms"]]);
const INTO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["into"]);
const SPELL_NOUN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["spell"], &["spells"]]);
const DRAW_OR_DRAWS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["draw"], &["draws"]]);
const DISCARD_OR_DISCARDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["discard"], &["discards"]]);
const REVEAL_OR_REVEALS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["reveal"], &["reveals"]]);
const SACRIFICE_OR_SACRIFICES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["sacrifice"], &["sacrifices"]]);
const BLOCK_OR_BLOCKS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["block"], &["blocks"]]);
const BY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["by"]);
const LINKING_BE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"], &["was"], &["were"], &["be"], &["been"]]);
const TRIGGER_INTRO_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["whenever"], &["when"], &["at"]]);
const AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const FOR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["for"]);
const TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const ONE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["one"]);
const MORE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["more"]);
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
const PLAY_OR_PLAYS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["play"], &["plays"]]);
const LAND_OR_LANDS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["land"], &["lands"]]);
const SEARCH_OR_SEARCHES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["search"], &["searches"]]);
const SHUFFLE_OR_SHUFFLES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["shuffle"], &["shuffles"]]);
const GIVE_OR_GIVES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["give"], &["gives"]]);
const TAP_OR_TAPS_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["tap"], &["taps"]]);
const TAPPED_WORD_EXACT_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["tapped"]);
const IS_OR_ARE_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["is"], &["are"]]);
const ACTIVATE_OR_ACTIVATES_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["activate"], &["activates"]]);
const PUT_OR_PUTS_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["put"], &["puts"]]);
const GET_OR_GETS_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["get"], &["gets"]]);
const COUNTER_OR_COUNTERS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const BECOMES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["becomes"]);
const DAMAGE_EXACT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["damage"]);
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
const ENERGY_COUNTER_DESCRIPTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["e", "energy"]]);
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
const NON_POSSESSIVE_PLURAL_SUFFIX_EXCLUSION_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["its"]]);

#[derive(Debug, Clone, Copy)]
struct TriggerSuffixShape {
    shape: ClauseShape<'static>,
    word_len: usize,
}

const fn trigger_suffix_shape(shape: ClauseShape<'static>, word_len: usize) -> TriggerSuffixShape {
    TriggerSuffixShape { shape, word_len }
}

fn token_words_match_prefix(tokens: &[OwnedLexToken], shape: &ClauseShape<'static>) -> bool {
    let words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
    shape.matches_words(&words)
}

fn find_token_shape(tokens: &[OwnedLexToken], shape: &ClauseShape<'static>) -> Option<usize> {
    find_index(tokens, |token| shape.matches_token(token))
}

fn find_phrase_shape(
    words: &[&str],
    phrase_len: usize,
    shape: ClauseShape<'static>,
) -> Option<usize> {
    words
        .windows(phrase_len)
        .position(|window| shape.matches_words(window))
}

fn subject_starts_one_or_more(words: &[&str]) -> bool {
    ONE_OR_MORE_PREFIX_PATTERN.matches_words(words)
}

fn subject_is_card_or_cards(words: &[&str]) -> bool {
    CARD_OR_CARDS_PATTERN.matches_words(words)
}

fn subject_mentions_card(words: &[&str]) -> bool {
    CARD_OR_CARDS_WORD_PATTERN.matches_words(words)
}

fn subject_mentions_permanent(words: &[&str]) -> bool {
    PERMANENT_OR_PERMANENTS_WORD_PATTERN.matches_words(words)
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
        .find(|suffix| suffix.shape.matches_words(words))
        .map(|suffix| suffix.word_len)
}

fn trigger_subject_tokens_before_suffix<'a>(
    tokens: &'a [OwnedLexToken],
    total_word_len: usize,
    suffix_word_len: usize,
) -> &'a [OwnedLexToken] {
    let subject_word_len = total_word_len.saturating_sub(suffix_word_len);
    ActivationRestrictionCompatWords::new(tokens)
        .token_index_for_word_index(subject_word_len)
        .map(|idx| &tokens[..idx])
        .unwrap_or_default()
}

fn trigger_counter_descriptor_span<'a>(
    tokens: &'a [OwnedLexToken],
    start_word_idx: usize,
    counter_word_idx: usize,
    words: &[&str],
) -> Result<(&'a [OwnedLexToken], &'a [OwnedLexToken]), CardTextError> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let descriptor_token_start = word_view
        .token_index_for_word_index(start_word_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter descriptor in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let descriptor_token_end = word_view
        .token_index_for_word_index(counter_word_idx)
        .unwrap_or(tokens.len());
    Ok((
        &tokens[descriptor_token_start..descriptor_token_end],
        &tokens[descriptor_token_start..(descriptor_token_end + 1)],
    ))
}

fn trigger_counter_type_from_descriptor(tokens: &[OwnedLexToken]) -> Option<CounterType> {
    parse_counter_type_from_tokens(tokens).or_else(|| {
        let words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
        ENERGY_COUNTER_DESCRIPTOR_PATTERN
            .matches_words(&words)
            .then_some(CounterType::Energy)
    })
}

fn trigger_counter_recipient_tokens(
    tokens: &[OwnedLexToken],
    object_word_start: usize,
    words: &[&str],
) -> Result<Vec<OwnedLexToken>, CardTextError> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let object_token_start = word_view
        .token_index_for_word_index(object_word_start)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter recipient in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let mut object_tokens = trim_commas(&tokens[object_token_start..]);
    let object_view = ActivationRestrictionCompatWords::new(&object_tokens);
    if object_view.first_is_any(&["a", "an", "the"]) {
        let start = object_view
            .token_index_for_word_index(1)
            .unwrap_or(object_tokens.len());
        object_tokens = object_tokens[start..].to_vec();
    }
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter recipient in trigger clause (clause: '{}')",
            words.join(" ")
        )));
    }
    Ok(object_tokens)
}

fn dealt_damage_suffix_subject_word_idx(words: &[&str]) -> Option<(usize, bool)> {
    if DEALT_COMBAT_DAMAGE_SUFFIX_PATTERN.matches_words(words) {
        return Some((words.len().saturating_sub(4), true));
    }
    if DEALT_DAMAGE_SUFFIX_PATTERN.matches_words(words) {
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
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let subject_words = non_article_word_refs(&word_view.to_word_refs());
    source_reference_surface_for_words(&subject_words)
}

fn this_enters_battlefield_trigger_spec(
    surface: Option<crate::target::SourceReferenceSurface>,
) -> TriggerSpec {
    match surface {
        Some(surface) => TriggerSpec::ThisEntersBattlefieldWithSurface(surface),
        None => TriggerSpec::ThisEntersBattlefield,
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
    if THIS_DESTINATION_TRIGGER_NAME_PATTERN.matches_words(&destination_words) {
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
        if !out.is_empty() && !out.ends_with(' ') {
            out.push(' ');
        }
        out.push_str(token.slice.as_str());
    }
    let trimmed = out.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn transform_destination_name_after_into(
    word_view: &ActivationRestrictionCompatWords<'_>,
    transforms_word_idx: usize,
    tokens: &[OwnedLexToken],
) -> Option<String> {
    let into_word_idx = transforms_word_idx + 1;
    let destination_word_idx = into_word_idx + 1;
    let destination_token_idx = word_view.token_index_for_word_index(destination_word_idx)?;
    trigger_destination_name_from_tokens(&tokens[destination_token_idx..])
}

pub(crate) fn split_trigger_or_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    find_index_with(tokens, |idx, token| {
        if !token
            .as_word()
            .is_some_and(|word| OR_WORD_PATTERN.matches_word(word))
        {
            return false;
        }
        let token_word = |offset: isize| {
            let target = idx.checked_add_signed(offset)?;
            tokens.get(target).and_then(OwnedLexToken::as_word)
        };
        let words = ActivationRestrictionCompatWords::new(tokens).to_word_refs();
        // Keep quantifiers like "one or more <subject>" intact.
        let quantifier_or = idx > 0
            && words
                .get(idx - 1..=idx + 1)
                .is_some_and(|words| ONE_OR_MORE_QUANTIFIER_PATTERN.matches_words(words));
        let comparison_or = is_comparison_or_delimiter(tokens, idx);
        let previous_numeric = (0..idx)
            .rev()
            .find_map(|i| tokens[i].as_word())
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let next_numeric = tokens
            .get(idx + 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let numeric_list_or = previous_numeric && next_numeric;
        let color_list_or = token_word(-1).is_some_and(|word| parse_color(word).is_some())
            && token_word(1).is_some_and(|word| parse_color(word).is_some())
            && SPELL_OR_SPELLS_WORD_PATTERN.matches_words(&words);
        let objectish_word = |word: &str| is_trigger_objectish_word(word);
        let object_list_or =
            token_word(-1).is_some_and(objectish_word) && token_word(1).is_some_and(objectish_word);
        let and_or_list_or = token_word(-1).is_some_and(|word| AND_WORD_PATTERN.matches_word(word))
            && token_word(1)
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
        let next_word = token_word(1);
        let serial_spell_list_or = SPELL_OR_SPELLS_WORD_PATTERN.matches_words(&words)
            && previous_word
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word))
            && next_word.is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let cast_or_copy_or = SPELL_OR_SPELLS_WORD_PATTERN.matches_words(&words)
            && previous_word.is_some_and(|word| CAST_OR_CASTS_PATTERN.matches_word(word))
            && next_word.is_some_and(|word| COPY_OR_COPIES_PATTERN.matches_word(word));
        let spell_or_ability_or = token_word(-1)
            .is_some_and(|word| SPELL_OR_SPELLS_PATTERN.matches_word(word))
            && token_word(1).is_some_and(|word| ABILITY_OR_ABILITIES_PATTERN.matches_word(word));
        if quantifier_or
            || comparison_or
            || numeric_list_or
            || color_list_or
            || object_list_or
            || and_or_list_or
            || serial_spell_list_or
            || cast_or_copy_or
            || spell_or_ability_or
        {
            false
        } else {
            true
        }
    })
}

pub(crate) fn has_leading_one_or_more(tokens: &[OwnedLexToken]) -> bool {
    leading_one_or_more_prefix_len(tokens).is_some()
}

pub(crate) fn strip_leading_one_or_more(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if let Some(used) = leading_one_or_more_prefix_len(tokens) {
        &tokens[used..]
    } else {
        tokens
    }
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

pub(crate) fn parse_trigger_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    fn parse_not_during_turn_suffix(words: &[&str]) -> Option<PlayerFilter> {
        if NOT_ITERATED_PLAYER_TURN_DRAW_TRIGGER_SUFFIX.matches_words(words) {
            Some(PlayerFilter::IteratedPlayer)
        } else if NOT_YOUR_TURN_DRAW_TRIGGER_SUFFIX.matches_words(words) {
            Some(PlayerFilter::You)
        } else if NOT_OPPONENTS_TURN_DRAW_TRIGGER_SUFFIX.matches_words(words) {
            Some(PlayerFilter::Opponent)
        } else {
            None
        }
    }

    fn parse_enters_origin_clause_lexed(words: &[&str]) -> Option<(Zone, Option<PlayerFilter>)> {
        let tail_words = non_article_word_refs(words);
        if ENTERS_FROM_YOUR_GRAVEYARD_ORIGIN_PATTERN.matches_words(&tail_words) {
            Some((Zone::Graveyard, Some(PlayerFilter::You)))
        } else if ENTERS_FROM_GRAVEYARD_ORIGIN_PATTERN.matches_words(&tail_words) {
            Some((Zone::Graveyard, None))
        } else if ENTERS_FROM_YOUR_HAND_ORIGIN_PATTERN.matches_words(&tail_words) {
            Some((Zone::Hand, Some(PlayerFilter::You)))
        } else if ENTERS_FROM_HAND_ORIGIN_PATTERN.matches_words(&tail_words) {
            Some((Zone::Hand, None))
        } else if ENTERS_FROM_EXILE_ORIGIN_PATTERN.matches_words(&tail_words) {
            Some((Zone::Exile, None))
        } else {
            None
        }
    }

    fn source_trigger_subject_filter_lexed(subject_words: &[&str]) -> ObjectFilter {
        let mut filter = ObjectFilter::default();
        if SOURCE_TRIGGER_CREATURE_SUBJECT_PATTERN.matches_words(subject_words) {
            filter.card_types.push(CardType::Creature);
        } else if SOURCE_TRIGGER_LAND_SUBJECT_PATTERN.matches_words(subject_words) {
            filter.card_types.push(CardType::Land);
        } else if SOURCE_TRIGGER_ARTIFACT_SUBJECT_PATTERN.matches_words(subject_words) {
            filter.card_types.push(CardType::Artifact);
        } else if SOURCE_TRIGGER_ENCHANTMENT_SUBJECT_PATTERN.matches_words(subject_words) {
            filter.card_types.push(CardType::Enchantment);
        } else if SOURCE_TRIGGER_PLANESWALKER_SUBJECT_PATTERN.matches_words(subject_words) {
            filter.card_types.push(CardType::Planeswalker);
        } else if SOURCE_TRIGGER_BATTLE_SUBJECT_PATTERN.matches_words(subject_words) {
            filter.card_types.push(CardType::Battle);
        }
        filter
    }

    fn parse_damage_by_dies_trigger_lexed(
        subject_tokens: &[OwnedLexToken],
        other: bool,
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if subject_words.len() < 8
            || !DAMAGE_BY_THIS_TURN_DIES_SUBJECT_PATTERN.matches_words(&subject_words)
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

        let victim_end = subject_word_view
            .token_index_for_word_index(dealt_word_idx)
            .unwrap_or(0);
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
        let damager_start = subject_word_view
            .token_index_for_word_index(damager_start_word_idx)
            .unwrap_or(subject_tokens.len());
        let damager_end = subject_word_view
            .token_index_for_word_index(this_word_idx)
            .unwrap_or(subject_tokens.len());
        if damager_start >= damager_end || damager_end > subject_tokens.len() {
            return Ok(None);
        }

        let damager_tokens =
            trim_edge_punctuation_tokens(&subject_tokens[damager_start..damager_end]);
        let damager_word_view = ActivationRestrictionCompatWords::new(&damager_tokens);
        let damager_words = damager_word_view.to_word_refs();
        let has_named_source_words = !damager_words.is_empty()
            && !damager_words.first().is_some_and(|word| {
                DAMAGER_NAMED_SOURCE_LEADING_EXCLUDED_PATTERN.matches_word(word)
            })
            && !damager_words
                .iter()
                .any(|word| GENERIC_DAMAGE_SOURCE_WORD_PATTERN.matches_word(word));

        let damager = if THIS_DAMAGE_SOURCE_TRIGGER_PATTERN.matches_words(&damager_words)
            || has_named_source_words
        {
            Some(DamageBySpec::ThisCreature)
        } else if EQUIPPED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN.matches_words(&damager_words) {
            Some(DamageBySpec::EquippedCreature)
        } else if ENCHANTED_CREATURE_DAMAGE_SOURCE_TRIGGER_PATTERN.matches_words(&damager_words) {
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
        if !SIMPLE_SPELL_ACTIVITY_OBJECT_PATTERN.matches_words(clause_words) {
            return Ok(None);
        }
        if SIMPLE_SPELL_ACTIVITY_EXCLUDED_WORD_PATTERN.matches_words(clause_words)
            || SIMPLE_SPELL_ACTIVITY_EXCLUDED_PHRASE_PATTERN.matches_words(clause_words)
        {
            return Ok(None);
        }

        let cast_idx = find_index(tokens, |token| CAST_OR_CASTS_PATTERN.matches_token(token));
        let copy_idx = find_index(tokens, |token| COPY_OR_COPIES_PATTERN.matches_token(token));
        if cast_idx.is_none() && copy_idx.is_none() {
            return Ok(None);
        }

        let actor = parse_subject_clause_player_filter(clause_words);
        let parse_filter =
            |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
                let filter_words = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_words.to_word_refs();
                let is_unqualified_spell =
                    UNQUALIFIED_SPELL_FILTER_PATTERN.matches_words(&filter_words);
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
            if CAST_OR_COPY_SEPARATOR_PATTERN.matches_words(&between_words) {
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
                    if LINKING_BE_WORD_PATTERN.matches_word(last_word) {
                        prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                    } else {
                        break;
                    }
                }
                let has_spell_noun = prefix_tokens
                    .iter()
                    .any(|token| SPELL_NOUN_PATTERN.matches_token(token));
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

    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "empty trigger clause".to_string(),
        ));
    }

    if CRAFT_EXILED_FROM_BATTLEFIELD_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(
            TriggerSpec::ThisExiledFromBattlefieldDuringCostOfAbilityWithMarker {
                marker: "craft".to_string(),
            },
        );
    }

    if words.len() > 6 && FINAL_CHAPTER_ABILITY_RESOLVES_TRIGGER_PATTERN.matches_words(&words) {
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

    if DAY_NIGHT_CHANGED_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::DayNightChanged);
    }

    if let Some(enters_idx) =
        find_index(tokens, |token| ENTER_OR_ENTERS_PATTERN.matches_token(token))
    {
        let tail = &tokens[enters_idx + 1..];
        let tail_words = ActivationRestrictionCompatWords::new(tail).to_word_refs();
        let shared_subject_or_combat_damage =
            SHARED_SUBJECT_ETB_OR_COMBAT_DAMAGE_TAIL_PATTERN.matches_words(&tail_words);
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
            SHARED_SUBJECT_ETB_OR_ATTACK_TAIL_PATTERN.matches_words(&tail_words);
        if shared_subject_or_attack {
            let or_idx = if OR_WORD_PATTERN.matches_words(&tail_words[..1]) {
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
    if let Some(and_idx) = find_token_shape(tokens, &AND_WORD_PATTERN)
        && tokens
            .get(and_idx + 1)
            .is_some_and(|token| TRIGGER_INTRO_WORD_PATTERN.matches_token(token))
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
        && ALONE_WORD_PATTERN.matches_word_at(&words, words.len() - 1)
        && ATTACK_OR_ATTACKS_PATTERN.matches_word_at(&words, words.len() - 2)
    {
        let attacks_word_idx = words.len().saturating_sub(2);
        let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(attacks_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksAlone(filter),
                None => TriggerSpec::AttacksAlone(ObjectFilter::source()),
            },
        );
    }

    if let Some(attacks_word_idx) = ATTACK_OR_ATTACKS_PATTERN.find_word(&words) {
        let tail_words = &words[attacks_word_idx + 1..];
        if ATTACKS_YOU_OR_PLANESWALKER_YOU_CONTROL_TAIL_PATTERN.matches_words(tail_words) {
            let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attacks_word_idx)
                .unwrap_or(tokens.len());
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
        && ATTACK_OR_ATTACKS_PATTERN.matches_word_at(&words, words.len() - 3)
        && WHILE_WORD_PATTERN.matches_word_at(&words, words.len() - 2)
        && SADDLED_WORD_PATTERN.matches_word_at(&words, words.len() - 1)
    {
        let attacks_word_idx = words.len().saturating_sub(3);
        let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(attacks_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksWhileSaddled(filter),
                None => TriggerSpec::ThisAttacksWhileSaddled,
            },
        );
    }

    if YOU_CAST_THIS_SPELL_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if let Some(spell_activity_trigger) = parse_simple_spell_activity_trigger_lexed(tokens, &words)?
    {
        return Ok(spell_activity_trigger);
    }
    if let Some(spell_activity_trigger) = parse_spell_activity_trigger(tokens)? {
        return Ok(spell_activity_trigger);
    }

    if let Some(play_idx) = find_token_shape(tokens, &PLAY_OR_PLAYS_PATTERN) {
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
                .any(|word| LAND_OR_LANDS_PATTERN.matches_word(word))
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerPlaysLand { player, filter });
            }
        }
    }

    if let Some(search_idx) = find_token_shape(tokens, &SEARCH_OR_SEARCHES_PATTERN) {
        let subject_tokens = &tokens[..search_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let searched_tokens = trim_commas(&tokens[search_idx + 1..]);
            let searched_word_view = ActivationRestrictionCompatWords::new(&searched_tokens);
            let searched_words = searched_word_view.to_word_refs();
            if LIBRARY_SEARCH_TARGET_PATTERN.matches_words(&searched_words) {
                return Ok(TriggerSpec::PlayerSearchesLibrary(player));
            }
        }
    }

    if let Some(shuffle_idx) = find_token_shape(tokens, &SHUFFLE_OR_SHUFFLES_PATTERN) {
        let subject_tokens = &tokens[..shuffle_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let shuffled_tokens = trim_commas(&tokens[shuffle_idx + 1..]);
        let shuffled_word_view = ActivationRestrictionCompatWords::new(&shuffled_tokens);
        let shuffled_words = shuffled_word_view.to_word_refs();
        if LIBRARY_SHUFFLE_TARGET_PATTERN.matches_words(&shuffled_words) {
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

    if let Some(give_idx) = find_token_shape(tokens, &GIVE_OR_GIVES_PATTERN) {
        let subject_tokens = &tokens[..give_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let gifted_tokens = trim_commas(&tokens[give_idx + 1..]);
            let gifted_word_view = ActivationRestrictionCompatWords::new(&gifted_tokens);
            let gifted_words = gifted_word_view.to_word_refs();
            if GIFT_TAIL_PATTERN.matches_words(&gifted_words) {
                return Ok(TriggerSpec::PlayerGivesGift(player));
            }
        }
    }

    if let Some(tap_idx) = find_token_shape(tokens, &TAP_OR_TAPS_PATTERN) {
        let subject_tokens = &tokens[..tap_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let after_tap = &tokens[tap_idx + 1..];
            if let Some(for_idx) = find_token_shape(after_tap, &FOR_WORD_PATTERN)
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

    if let Some(tapped_idx) = find_token_shape(tokens, &TAPPED_WORD_EXACT_PATTERN)
        && tapped_idx >= 2
        && tokens
            .get(tapped_idx.wrapping_sub(1))
            .is_some_and(|token| IS_OR_ARE_PATTERN.matches_token(token))
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

    if let Some(activate_idx) = ACTIVATE_OR_ACTIVATES_PATTERN.find_word(&words) {
        let subject_tokens = &tokens[..activate_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(activator) = parse_trigger_subject_player_filter(&subject_words) {
            let tail_words = &words[activate_idx + 1..];
            if let Some(filter) = parse_loyalty_ability_trigger_tail_lexed(
                &tokens[activate_idx + 1..],
                tail_words,
            )? {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only: false,
                    loyalty_only: true,
                });
            }
            if let Some((owner_filter, marker)) = parse_possessive_ability_trigger_tail_lexed(
                &tokens[activate_idx + 1..],
                tail_words,
            )? {
                let filter = match marker {
                    Some(marker) => owner_filter.with_ability_marker(marker),
                    None => owner_filter,
                };
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter,
                    non_mana_only: false,
                    loyalty_only: false,
                });
            }
            if ACTIVATED_ABILITY_TAIL_PATTERN.matches_words(tail_words) {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter: ObjectFilter::default(),
                    non_mana_only: MANA_ABILITY_TAIL_PATTERN.matches_words(tail_words),
                    loyalty_only: false,
                });
            }
        }
    }

    let has_deal = DEAL_OR_DEALS_PATTERN.find_word(&words).is_some();
    if has_deal && COMBAT_DAMAGE_TRIGGER_PATTERN.matches_words(&words) {
        if let Some(deals_idx) = find_token_shape(tokens, &DEAL_OR_DEALS_PATTERN) {
            let subject_tokens = &tokens[..deals_idx];
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let one_or_more = has_leading_one_or_more(subject_tokens) || player_subject;
            let source_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?;
            if let Some(damage_idx_rel) =
                find_token_shape(&tokens[deals_idx + 1..], &DAMAGE_EXACT_WORD_PATTERN)
            {
                let damage_idx = deals_idx + 1 + damage_idx_rel;
                if let Some(to_idx_rel) =
                    find_token_shape(&tokens[damage_idx + 1..], &TO_WORD_PATTERN)
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
                            None => TriggerSpec::ThisDealsCombatDamageToPlayer,
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

    if THIS_LEAVES_BATTLEFIELD_TRIGGER_PATTERN.matches_words(&words)
        || (words.len() == 5
            && THIS_WORD_PATTERN.matches_word_at(&words, 0)
            && LEAVES_WORD_PATTERN.matches_word_at(&words, 2)
            && THE_WORD_PATTERN.matches_word_at(&words, 3)
            && BATTLEFIELD_WORD_PATTERN.matches_word_at(&words, 4))
    {
        return Ok(TriggerSpec::ThisLeavesBattlefield);
    }

    if let Some(leaves_word_idx) = LEAVE_OR_LEAVES_PATTERN.find_word(&words)
        && LEAVES_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&words[leaves_word_idx..])
    {
        let leaves_token_idx = word_view
            .token_index_for_word_index(leaves_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..leaves_token_idx];

        if let Some(or_idx) = find_token_shape(subject_tokens, &OR_WORD_PATTERN) {
            let left_tokens = &subject_tokens[..or_idx];
            let mut right_tokens = &subject_tokens[or_idx + 1..];
            let left_words = non_article_word_refs(
                &ActivationRestrictionCompatWords::new(left_tokens).to_word_refs(),
            );
            if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                let mut other = false;
                if token_words_match_prefix(right_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN) {
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
        if token_words_match_prefix(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN) {
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

    if let Some(dies_word_idx) = DIE_OR_DIES_PATTERN.find_word(&words) {
        let dies_token_idx = word_view
            .token_index_for_word_index(dies_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..dies_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && OR_IS_PUT_INTO_EXILE_FROM_BATTLEFIELD_TAIL_PATTERN
                .matches_words(&words[dies_word_idx + 1..])
        {
            return Ok(TriggerSpec::ThisDiesOrIsExiled);
        }
    }

    if let Some(enters_word_idx) = ENTER_OR_ENTERS_PATTERN.find_word(&words) {
        let enters_token_idx = word_view
            .token_index_for_word_index(enters_word_idx)
            .unwrap_or(tokens.len());
        if ENTERS_OR_LEAVES_BATTLEFIELD_SUFFIX_PATTERN.matches_words(&words) {
            let subject_tokens = &tokens[..enters_token_idx];
            if token_words_match_prefix(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN) {
                return Ok(TriggerSpec::Either(
                    Box::new(TriggerSpec::ThisEntersBattlefield),
                    Box::new(TriggerSpec::ThisLeavesBattlefield),
                ));
            }
        }

        let enters_origin = parse_enters_origin_clause_lexed(&words[enters_word_idx + 1..]);
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
        if OR_TRANSFORMS_INTO_TAIL_PREFIX_PATTERN.matches_words(&words[enters_word_idx + 1..]) {
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
        if let Some(or_idx) = find_token_shape(subject_tokens, &OR_WORD_PATTERN) {
            let or_is_one_or_more_quantifier = or_idx == 1
                && subject_tokens
                    .first()
                    .is_some_and(|token| ONE_WORD_PATTERN.matches_token(token))
                && subject_tokens
                    .get(or_idx + 1)
                    .is_some_and(|token| MORE_WORD_PATTERN.matches_token(token));
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
                    if token_words_match_prefix(right_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN) {
                        other = true;
                        right_tokens = &right_tokens[1..];
                    }
                    let parsed_filter = parse_object_filter_lexed(right_tokens, other)
                        .ok()
                        .or_else(|| {
                            parse_subtype_list_enters_trigger_filter_lexed(right_tokens, other)
                        });
                    if let Some(mut filter) = parsed_filter {
                        if UNDER_YOUR_CONTROL_PATTERN.matches_words(&words) {
                            filter.controller = Some(PlayerFilter::You);
                        } else if UNDER_OPPONENT_CONTROL_PATTERN.matches_words(&words) {
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
                        let right_trigger = if UNTAPPED_WORD_PATTERN.matches_words(&words) {
                            TriggerSpec::EntersBattlefieldUntapped {
                                filter,
                                cause_filter,
                            }
                        } else if TAPPED_WORD_PATTERN.matches_words(&words) {
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
        if token_words_match_prefix(subject_tokens, &THIS_DESTINATION_TRIGGER_NAME_PATTERN) {
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: source_trigger_subject_filter_lexed(&subject_words),
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
        if token_words_match_prefix(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN) {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let one_or_more = ActivationRestrictionCompatWords::new(filtered_subject_tokens)
            .to_word_refs()
            .get(..3)
            .is_some_and(|words| ONE_OR_MORE_QUANTIFIER_PATTERN.matches_words(words));
        filtered_subject_tokens = strip_leading_one_or_more_lexed(filtered_subject_tokens);
        if token_words_match_prefix(filtered_subject_tokens, &OTHER_OR_ANOTHER_PREFIX_PATTERN) {
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
            if UNDER_YOUR_CONTROL_PATTERN.matches_words(&words) {
                filter.controller = Some(PlayerFilter::You);
            } else if UNDER_OPPONENT_CONTROL_PATTERN.matches_words(&words) {
                filter.controller = Some(PlayerFilter::Opponent);
            }
            if UNTAPPED_WORD_PATTERN.matches_words(&words) {
                return Ok(TriggerSpec::EntersBattlefieldUntapped {
                    filter,
                    cause_filter,
                });
            }
            if TAPPED_WORD_PATTERN.matches_words(&words) {
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

    if let Some(transforms_word_idx) = TRANSFORM_OR_TRANSFORMS_PATTERN.find_word(&words) {
        let transforms_token_idx = word_view
            .token_index_for_word_index(transforms_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..transforms_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && words
                .get(transforms_word_idx + 1)
                .is_some_and(|word| INTO_WORD_PATTERN.matches_word(word))
        {
            let destination_name =
                transform_destination_name_after_into(&word_view, transforms_word_idx, tokens);
            return Ok(this_transforms_trigger_spec(
                source_reference_surface_for_trigger_subject(subject_tokens),
                destination_name,
            ));
        }
    }

    let (zone_change_words, during_turn) = if DURING_YOUR_TURN_TRIGGER_SUFFIX.matches_words(&words)
    {
        (
            &words[..words.len().saturating_sub(3)],
            Some(PlayerFilter::You),
        )
    } else {
        (words.as_slice(), None)
    };

    for tail in [
        ["leave", "your", "graveyard"].as_slice(),
        ["leaves", "your", "graveyard"].as_slice(),
    ] {
        if slice_ends_with(zone_change_words, tail) {
            let subject_word_len = zone_change_words.len().saturating_sub(tail.len());
            let mut subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
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
    ] {
        if ClauseShape::new()
            .suffix(tail)
            .matches_words(zone_change_words)
        {
            let subject_word_len = zone_change_words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
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
            filter.owner = None;
            if subject_mentions_card(&subject_words) {
                filter.card_types.clear();
                filter.nontoken = true;
            }
            return Ok(TriggerSpec::PutIntoExileFromZones {
                filter,
                from: from_zones,
                one_or_more,
                during_turn,
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
        if ATTACHED_OBJECT_PREFIX_PATTERN.matches_words(&subject_words) {
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

    if let Some(put_word_idx) = PUT_OR_PUTS_PATTERN.find_word(&words)
        && let Some(source_controller) = parse_trigger_subject_player_filter(&words[..put_word_idx])
        && let Some(counter_word_idx) = COUNTER_OR_COUNTERS_PATTERN.find_word(&words)
        && counter_word_idx > put_word_idx
        && words
            .get(counter_word_idx + 1..counter_word_idx + 2)
            .is_some_and(|preposition| {
                COUNTER_RECIPIENT_PREPOSITION_PATTERN.matches_words(preposition)
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
        let one_or_more = ONE_OR_MORE_PREFIX_PATTERN.matches_words(&descriptor_words);
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

    if let Some(get_word_idx) = GET_OR_GETS_PATTERN.find_word(&words)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..get_word_idx])
        && words
            .get(get_word_idx + 1..)
            .is_some_and(|tail| PLAYER_GETS_ONE_OR_MORE_ENERGY_TAIL_PATTERN.matches_words(tail))
    {
        return Ok(TriggerSpec::PlayerGetsCounters {
            player,
            counter_type: Some(CounterType::Energy),
            one_or_more: true,
        });
    }

    if let Some(get_word_idx) = GET_OR_GETS_PATTERN.find_word(&words)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..get_word_idx])
        && let Some(counter_word_idx) = COUNTER_OR_COUNTERS_PATTERN.find_word(&words)
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
        let one_or_more = ONE_OR_MORE_PREFIX_PATTERN.matches_words(&descriptor_words);
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        return Ok(TriggerSpec::PlayerGetsCounters {
            player,
            counter_type,
            one_or_more,
        });
    }

    if PLAYERS_FINISH_VOTING_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if YOU_CYCLE_THIS_CARD_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            player: PlayerFilter::You,
        });
    }

    if YOU_CYCLE_OR_DISCARD_TRIGGER_PATTERN.matches_words(&words) {
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
            }),
        ));
    }

    if YOU_COMMIT_CRIME_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if OPPONENT_COMMITS_CRIME_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Opponent,
            source_filter: None,
        });
    }

    if PLAYER_COMMITS_CRIME_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if YOU_UNLOCK_THIS_DOOR_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::UnlockDoor,
            player: PlayerFilter::You,
        });
    }

    if THIS_CARD_BECOMES_PLOTTED_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Plot,
            player: PlayerFilter::You,
        });
    }

    if words.len() == 3
        && YOU_EXPEND_TRIGGER_PREFIX.matches_words(&words)
        && let Some(amount) = parse_named_number(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::You,
            amount,
        });
    }

    if words.len() == 4
        && OPPONENT_EXPENDS_WITH_ARTICLE_TRIGGER_PREFIX.matches_words(&words)
        && let Some(amount) = parse_named_number(words[3])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.len() == 3
        && OPPONENT_EXPENDS_TRIGGER_PREFIX.matches_words(&words)
        && let Some(amount) = parse_named_number(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if THE_RING_TEMPTS_YOU_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::RingTemptsYou,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if CHAOS_ENSUES_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ChaosEnsues,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if let Some(cycle_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Cycle)
        )
    }) {
        let subject_words = &words[..cycle_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let tail_words = &words[cycle_word_idx + 1..];
            if CYCLE_CARD_TAIL_PATTERN.matches_words(tail_words) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: None,
                });
            }
            if CYCLE_ANOTHER_CARD_TAIL_PATTERN.matches_words(tail_words) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: Some(ObjectFilter::default().other()),
                });
            }
        }
    }

    if let Some(exert_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Exert)
        )
    }) {
        let subject = &words[..exert_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[exert_word_idx + 1..];
            if EXERT_CREATURE_TAIL_PATTERN.matches_words(tail) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Exert,
                    player,
                    source_filter: Some(ObjectFilter::creature()),
                });
            }
        }
    }

    if let Some(crew_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Crew)
        )
    }) {
        let subject_words = &words[..crew_word_idx];
        let source_filter = if is_source_reference_words(subject_words) {
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
            let object_filter =
                if tail_words.is_empty() || CREW_VEHICLE_TAIL_PATTERN.matches_words(tail_words) {
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
            });
        }
    }

    if let Some(explore_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Explore)
        )
    }) {
        let subject_tokens = &tokens[..explore_word_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            let tail = &words[explore_word_idx + 1..];
            let revealed_filter = if tail.is_empty() {
                None
            } else if EXPLORE_LAND_CARD_TAIL_PATTERN.matches_words(tail) {
                Some(ObjectFilter::default().with_type(crate::types::CardType::Land))
            } else if EXPLORE_NONLAND_CARD_TAIL_PATTERN.matches_words(tail) {
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

    if let Some(fight_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Fight)
        )
    }) {
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

    if let Some(put_word_idx) = PUT_OR_PUTS_PATTERN.find_word(&words) {
        let subject = &words[..put_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[put_word_idx + 1..];
            if NAME_STICKER_PUT_TAIL_PATTERN.matches_words(tail) {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::NameSticker,
                    player,
                    source_filter: None,
                });
            }
        }
    }

    if BECOMES_TAPPED_TRIGGER_SUFFIX.matches_words(&words)
        && let Some(becomes_idx) = find_token_shape(tokens, &BECOMES_WORD_PATTERN)
    {
        let subject_tokens = &tokens[..becomes_idx];
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::PermanentBecomesTapped(filter),
            None => TriggerSpec::ThisBecomesTapped,
        });
    }

    if THIS_BECOMES_TAPPED_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if THIS_BECOMES_UNTAPPED_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if THIS_BECOMES_MONSTROUS_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }
    if BECOMES_MONSTROUS_TRIGGER_SUFFIX.matches_words(&words)
        && words.len() > 2
        && source_reference_surface_for_words(&words[..words.len() - 2]).is_some()
    {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if THIS_MUTATES_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::ThisMutates);
    }
    if MUTATES_TRIGGER_SUFFIX.matches_words(&words)
        && words.len() > 1
        && source_reference_surface_for_words(&words[..words.len() - 1]).is_some()
    {
        return Ok(TriggerSpec::ThisMutates);
    }

    if THIS_TURNED_FACE_UP_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::ThisTurnedFaceUp);
    }

    if TURNED_FACE_UP_TRIGGER_SUFFIX.matches_words(&words) {
        let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(words.len().saturating_sub(4))
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::TurnedFaceUp(filter),
            None => TriggerSpec::ThisTurnedFaceUp,
        });
    }

    if let Some(becomes_idx) = BECOMES_WORD_PATTERN.find_word(&words)
        && BECOMES_TARGET_OF_PREFIX_PATTERN.matches_words(&words[becomes_idx + 1..])
    {
        let subject_words = &words[..becomes_idx];
        let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(becomes_idx)
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
            if SPELL_OR_ABILITY_TARGET_TAIL_PATTERN.matches_words(tail_words) {
                return Ok(TriggerSpec::ThisBecomesTargeted);
            }
            if ONLY_IT_ABILITY_TARGET_TAIL_PATTERN.matches_words(tail_words) {
                let mut ability_filter = ObjectFilter::ability();
                ability_filter.target_count = Some(crate::effect::ChoiceCount::exactly(1));
                ability_filter.targets_only_object = Some(Box::new(ObjectFilter::source()));
                return Ok(TriggerSpec::ThisBecomesTargetedByStackObject(
                    ability_filter,
                ));
            }
            if SPELL_OR_SPELLS_SUFFIX_PATTERN.matches_words(tail_words) {
                let tail_token_start = ActivationRestrictionCompatWords::new(tokens)
                    .token_index_for_word_index(tail_word_start)
                    .unwrap_or(tokens.len());
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
                && let Some(filter) = subject_filter.clone()
            {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: filter,
                    source_controller,
                });
            }
            if SPELL_OR_ABILITY_TARGET_TAIL_PATTERN.matches_words(tail_words)
                && let Some(filter) = subject_filter
            {
                return Ok(TriggerSpec::BecomesTargeted(filter));
            }
            if BACKUP_ABILITY_TARGET_TAIL_PATTERN.matches_words(tail_words)
                && let Some(filter) = subject_filter
            {
                let ability_filter = ObjectFilter::ability().with_ability_marker("backup");
                return Ok(TriggerSpec::BecomesTargetedByStackObject {
                    target: filter,
                    stack_object: ability_filter,
                });
            }
            if SPELL_OR_SPELLS_SUFFIX_PATTERN.matches_words(tail_words)
                && let Some(filter) = subject_filter
            {
                let tail_token_start = ActivationRestrictionCompatWords::new(tokens)
                    .token_index_for_word_index(tail_word_start)
                    .unwrap_or(tokens.len());
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
        && !SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX.matches_words(&words)
    {
        let is_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(is_word_idx)
            .unwrap_or(tokens.len());
        if is_word_idx == 0
            && words
                .first()
                .is_some_and(|word| YOU_CONTRACTION_WORD_PATTERN.matches_word(word))
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

    if SOURCE_DEALT_DAMAGE_TRIGGER_PREFIX.matches_words(&words) {
        if SOURCE_DEALT_COMBAT_DAMAGE_TRIGGER_PREFIX.matches_words(&words) {
            return Ok(TriggerSpec::ThisIsDealtCombatDamage);
        }
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if SOURCE_DEALS_TRIGGER_PREFIX.matches_words(&words)
        && let Some(deals_idx) = find_token_shape(tokens, &DEAL_OR_DEALS_PATTERN)
        && let Some(damage_idx_rel) =
            find_token_shape(&tokens[deals_idx + 1..], &DAMAGE_EXACT_WORD_PATTERN)
    {
        let damage_idx = deals_idx + 1 + damage_idx_rel;
        if let Some(to_idx_rel) = find_token_shape(&tokens[damage_idx + 1..], &TO_WORD_PATTERN) {
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_tokens = trim_commas(&tokens[deals_idx + 1..damage_idx]);
            if !amount_tokens
                .first()
                .is_some_and(|token| COMBAT_WORD_PATTERN.matches_token(token))
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

    if SOURCE_DEALS_DAMAGE_TO_TRIGGER_PREFIX.matches_words(&words)
        && let Some(to_idx) = find_token_shape(tokens, &TO_WORD_PATTERN)
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

    if SOURCE_DEALS_DAMAGE_TRIGGER_PREFIX.matches_words(&words) {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    if has_deal
        && DAMAGE_WORD_PATTERN.matches_words(&words)
        && let Some(deals_idx) = find_token_shape(tokens, &DEAL_OR_DEALS_PATTERN)
    {
        let subject_tokens = &tokens[..deals_idx];
        if let Some(damage_idx_rel) =
            find_token_shape(&tokens[deals_idx + 1..], &DAMAGE_EXACT_WORD_PATTERN)
            && let Some(to_idx_rel) = find_token_shape(
                &tokens[deals_idx + 1 + damage_idx_rel + 1..],
                &TO_WORD_PATTERN,
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
            if NONCOMBAT_DAMAGE_AMOUNT_PATTERN.matches_words(&amount_words)
                && let Some(player) = parse_trigger_subject_player_filter(&target_words)
                && let Some(source) = parse_trigger_subject_filter_lexed(subject_tokens)?
            {
                return Ok(TriggerSpec::DealsNoncombatDamageToPlayer { source, player });
            }
            if let Some(player) = parse_trigger_subject_player_filter(&target_words)
                && let Some(source) = parse_trigger_subject_filter_lexed(subject_tokens)?
            {
                return Ok(TriggerSpec::DealsDamageToPlayer { source, player });
            }
        }
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::DealsDamage(filter),
            None => TriggerSpec::ThisDealsDamage,
        });
    }

    if YOU_GAIN_LIFE_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.len() >= 6
        && DURING_YOUR_TURN_TRIGGER_SUFFIX.matches_words(&words)
        && YOU_GAIN_LIFE_PREFIX_PATTERN.matches_words(&words[..words.len() - 3])
    {
        return Ok(TriggerSpec::YouGainLifeDuringTurn(PlayerFilter::You));
    }

    if LOSE_LIFE_TRIGGER_SUFFIX.matches_words(&words) {
        let subject = &words[..words.len().saturating_sub(2)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLife(player));
        }
    }

    if LOSE_GAME_TRIGGER_SUFFIX.matches_words(&words) {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesGame(player));
        }
    }

    if words.len() >= 5
        && DURING_YOUR_TURN_TRIGGER_SUFFIX.matches_words(&words)
        && LOSE_LIFE_TRIGGER_SUFFIX.matches_words(&words[..words.len() - 3])
    {
        let subject = &words[..words.len() - 5];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLifeDuringTurn {
                player,
                during_turn: PlayerFilter::You,
            });
        }
    }

    if let Some(draw_word_idx) = DRAW_OR_DRAWS_PATTERN.find_word(&words) {
        let subject = &words[..draw_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[draw_word_idx + 1..];
            if let Some(during_turn) = parse_not_during_turn_suffix(tail) {
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

    if DRAW_A_CARD_TRIGGER_SUFFIX.matches_words(&words) {
        let subject = &words[..words.len().saturating_sub(3)];
        if YOU_DRAW_CARD_TRIGGER_SUBJECT_PATTERN.matches_words(subject) {
            return Ok(TriggerSpec::YouDrawCard);
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerDrawsCard(player));
        }
    }

    if OPPONENT_EFFECT_DISCARDS_THIS_CARD_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::PlayerDiscardsCard {
            player: PlayerFilter::You,
            filter: Some(ObjectFilter::source()),
            cause_controller: Some(PlayerFilter::Opponent),
            effect_like_only: true,
        });
    }

    if let Some(discard_word_idx) = DISCARD_OR_DISCARDS_PATTERN.find_word(&words)
        && let Some(discard_token_idx) = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(discard_word_idx)
    {
        let subject_words = &words[..discard_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            if let Ok(filter) =
                parse_discard_trigger_card_filter(&tokens[discard_token_idx + 1..], &words)
            {
                return Ok(TriggerSpec::PlayerDiscardsCard {
                    player,
                    filter,
                    cause_controller: None,
                    effect_like_only: false,
                });
            }
        }
    }

    if let Some(reveal_word_idx) = REVEAL_OR_REVEALS_PATTERN.find_word(&words)
        && let Some(player) = parse_trigger_subject_player_filter(&words[..reveal_word_idx])
    {
        let mut tail_tokens = trim_commas(
            &tokens[ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(reveal_word_idx + 1)
                .unwrap_or(tokens.len())..],
        );
        let tail_view = ActivationRestrictionCompatWords::new(&tail_tokens);
        let tail_words = tail_view.to_word_refs();
        let from_source = THIS_WAY_REVEAL_TAIL_PATTERN.matches_words(&tail_words);
        if from_source {
            let cutoff = ActivationRestrictionCompatWords::new(&tail_tokens)
                .token_index_for_word_index(tail_words.len().saturating_sub(2))
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

    if let Some(sacrifice_word_idx) = SACRIFICE_OR_SACRIFICES_PATTERN.find_word(&words)
        && let Some(sacrifice_token_idx) = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(sacrifice_word_idx)
    {
        let subject_words = &words[..sacrifice_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let mut filter_tokens = &tokens[sacrifice_token_idx + 1..];
            let mut other = false;
            if filter_tokens
                .first()
                .is_some_and(|token| OTHER_OR_ANOTHER_EXACT_PATTERN.matches_token(token))
            {
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
                .is_some_and(|token| THIS_OR_IT_PATTERN.matches_token(token))
            {
                let filter_word_view = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_word_view.to_word_refs();
                let mut filter = ObjectFilter::source();
                if SOURCE_ARTIFACT_WORD_PATTERN.matches_words(&filter_words) {
                    filter = filter.with_type(CardType::Artifact);
                } else if SOURCE_CREATURE_WORD_PATTERN.matches_words(&filter_words) {
                    filter = filter.with_type(CardType::Creature);
                } else if SOURCE_ENCHANTMENT_WORD_PATTERN.matches_words(&filter_words) {
                    filter = filter.with_type(CardType::Enchantment);
                } else if SOURCE_LAND_WORD_PATTERN.matches_words(&filter_words) {
                    filter = filter.with_type(CardType::Land);
                } else if SOURCE_PLANESWALKER_WORD_PATTERN.matches_words(&filter_words) {
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

    if let Some(roll_word_idx) = find_index(&words, |word| *word == "roll" || *word == "rolls") {
        let subject_words = &words[..roll_word_idx];
        let result_words = &words[roll_word_idx + 1..];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let result_words = if result_words.first() == Some(&"a") {
                &result_words[1..]
            } else {
                result_words
            };
            if let Some((result, used)) = ironsmith_core::parse_cardinal_words(result_words)
                && used == result_words.len()
            {
                return Ok(TriggerSpec::PlayerRollsResult { player, result });
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
            let trailing_ok = subject[2..]
                .iter()
                .all(|word| SOURCE_KEYWORD_ACTION_TRAILING_WORD_PATTERN.matches_word(word));
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

    if YOU_OPEN_ATTRACTION_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::OpenAttraction,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if YOU_CLAIM_ATTRACTION_PRIZE_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::ClaimAttractionPrize,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if let Some(exploit_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Exploit)
        )
    }) {
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
                || EXPLOIT_CREATURE_TAIL_PATTERN.matches_words(tail_words)
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

    if THIS_EXPLOITS_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Exploit,
            player: PlayerFilter::You,
        });
    }

    if YOU_COMPLETE_DUNGEON_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CompleteDungeon,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if WINS_CLASH_TRIGGER_SUFFIX_PATTERN.matches_words(&words) {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::WinsClash { player });
        }
    }

    if let Some(counter_word_idx) = COUNTER_OR_COUNTERS_PATTERN.find_word(&words)
        && PASSIVE_COUNTER_PUT_TAIL_PATTERN.matches_words(&words[counter_word_idx..])
    {
        let word_view = ActivationRestrictionCompatWords::new(tokens);
        let one_or_more = ONE_OR_MORE_PREFIX_PATTERN.matches_words(&words);
        let descriptor_token_end = word_view
            .token_index_for_word_index(counter_word_idx)
            .unwrap_or(tokens.len());
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

    if let Some(attacks_word_idx) = ATTACK_OR_ATTACKS_PATTERN.find_word(&words) {
        let tail_words = &words[attacks_word_idx + 1..];
        if ATTACKS_AND_IS_NOT_BLOCKED_TAIL_PATTERN.matches_words(tail_words) {
            let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attacks_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            return Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(filter) => TriggerSpec::AttacksAndIsntBlocked(filter),
                    None => TriggerSpec::ThisAttacksAndIsntBlocked,
                },
            );
        }
    }

    if THIS_BLOCKS_OR_BECOMES_BLOCKED_TRIGGER_PATTERN.matches_words(&words) {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::ThisBlocks),
            Box::new(TriggerSpec::ThisBecomesBlocked),
        ));
    }

    if THIS_BLOCKS_OR_BECOMES_BLOCKED_BY_TRIGGER_PREFIX.matches_words(&words)
        && let Some(by_idx) = find_token_shape(tokens, &BY_WORD_PATTERN)
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

    if THIS_BLOCKS_PREFIX_PATTERN.matches_words(&words)
        && let Some(blocks_idx) = find_token_shape(tokens, &BLOCK_OR_BLOCKS_PATTERN)
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

    let (words, attacked_player_filter, attacked_target_must_be_player) =
        if let Some(attacks_word_idx) = ATTACK_OR_ATTACKS_PATTERN.find_word(&words) {
            let tail = &words[attacks_word_idx + 1..];
            if ATTACKS_A_PLAYER_TAIL_PATTERN.matches_words(tail) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::Any), true)
            } else if ATTACKS_OPPONENT_TAIL_PATTERN.matches_words(tail) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::Opponent),
                    true,
                )
            } else if ATTACKS_DEFENDING_PLAYER_TAIL_PATTERN.matches_words(tail) {
                (&words[..=attacks_word_idx], Some(PlayerFilter::Any), true)
            } else if ATTACKS_OPPONENT_OR_PLANESWALKER_TAIL_PATTERN.matches_words(tail) {
                (
                    &words[..=attacks_word_idx],
                    Some(PlayerFilter::Opponent),
                    false,
                )
            } else if ATTACKS_PLANESWALKER_OR_BATTLE_TAIL_PATTERN.matches_words(tail) {
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

    if word_slice_ends_with(&words, &["are", "attacked"]) && words.len() > 2 {
        let attacked_player_words = &words[..words.len() - 2];
        if let Some(player_filter) = parse_trigger_subject_player_filter(attacked_player_words) {
            return Ok(TriggerSpec::PlayersAttackedOneOrMore(player_filter));
        }
    }

    match last {
        "attack" | "attacks" => {
            let attack_word_idx = words.len().saturating_sub(1);
            let attack_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attack_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attack_token_idx];
            if let Some(and_idx) = find_index(subject_tokens, |token| token.is_word("and")) {
                let left = trim_edge_punctuation(&subject_tokens[..and_idx]);
                let right = trim_edge_punctuation(&subject_tokens[and_idx + 1..]);
                if !left.is_empty()
                    && token_slice_at_is(&right, 0, "at")
                    && token_slice_at_is(&right, 1, "least")
                    && let Some((other_count, used)) = parse_number(&right[2..])
                    && right
                        .get(2 + used)
                        .is_some_and(|token| token.is_word("other"))
                    && right.get(3 + used).is_some_and(|token| {
                        token.is_word("creature") || token.is_word("creatures")
                    })
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
                    });
                }
            }
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
            let one_or_more = ONE_OR_MORE_PREFIX_PATTERN
                .matches_words(&subject_words.to_word_refs())
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
            let block_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(block_word_idx)
                .unwrap_or(tokens.len());
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
            let dies_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(dies_word_idx)
                .unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            if subject_tokens
                .first()
                .is_some_and(|token| THIS_DESTINATION_TRIGGER_NAME_PATTERN.matches_token(token))
            {
                let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
                let subject_words = subject_word_view.to_word_refs();
                if let Some(or_word_idx) =
                    find_phrase_shape(&subject_words, OR_ANOTHER_WORDS.len(), OR_ANOTHER_PATTERN)
                {
                    let rhs_word_idx = or_word_idx + 2;
                    let rhs_token_idx = subject_word_view
                        .token_index_for_word_index(rhs_word_idx)
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
            if THE_CREATURE_HAUNTS_PATTERN.matches_words(&subject_words) {
                return Ok(TriggerSpec::HauntedCreatureDies);
            }

            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens
                .first()
                .is_some_and(|token| OTHER_OR_ANOTHER_EXACT_PATTERN.matches_token(token))
            {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens
                .first()
                .is_some_and(|token| OTHER_OR_ANOTHER_EXACT_PATTERN.matches_token(token))
            {
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
                if AND_WORD_PATTERN.matches_token(&subject_tokens[idx])
                    && subject_tokens
                        .get(idx + 1)
                        .is_some_and(|token| OR_WORD_PATTERN.matches_token(token))
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
        "turn" if words.len() >= 4 && DIES_DURING_YOUR_TURN_SUFFIX.matches_words(&words) => {
            let dies_word_idx = words.len().saturating_sub(4);
            let dies_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(dies_word_idx)
                .unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            let one_or_more = has_leading_one_or_more(subject_tokens);
            let mut other = false;
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens
                .first()
                .is_some_and(|token| OTHER_OR_ANOTHER_EXACT_PATTERN.matches_token(token))
            {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
            if subject_tokens
                .first()
                .is_some_and(|token| OTHER_OR_ANOTHER_EXACT_PATTERN.matches_token(token))
            {
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
        _ if BEGINNING_END_STEP_TRIGGER_PATTERN.matches_words(&words) => Ok(
            TriggerSpec::BeginningOfEndStep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if BEGINNING_UPKEEP_TRIGGER_PATTERN.matches_words(&words) => Ok(
            TriggerSpec::BeginningOfUpkeep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if BEGINNING_DRAW_STEP_TRIGGER_PATTERN.matches_words(&words) => Ok(
            TriggerSpec::BeginningOfDrawStep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if BEGINNING_FIRST_MAIN_PHASE_TRIGGER_PATTERN.matches_words(&words) => Ok(
            TriggerSpec::BeginningOfPrecombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if BEGINNING_SECOND_MAIN_PHASE_TRIGGER_PATTERN.matches_words(&words) => Ok(
            TriggerSpec::BeginningOfPostcombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if BEGINNING_PRECOMBAT_MAIN_TRIGGER_PATTERN.matches_words(&words) => Ok(
            TriggerSpec::BeginningOfPrecombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if BEGINNING_POSTCOMBAT_MAIN_TRIGGER_PATTERN.matches_words(&words) => Ok(
            TriggerSpec::BeginningOfPostcombatMain(parse_possessive_clause_player_filter(&words)),
        ),
        _ if BEGINNING_COMBAT_TRIGGER_PATTERN.matches_words(&words) => Ok(
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
    let Some(ability_idx) = find_token_shape(tail_tokens, &ABILITY_OR_ABILITIES_PATTERN) else {
        return Ok(None);
    };
    if ability_idx == 0 || !tail_words[..ability_idx].contains(&"loyalty") {
        return Ok(None);
    }
    let Some(of_idx) = tail_words
        .iter()
        .enumerate()
        .skip(ability_idx + 1)
        .find_map(|(idx, word)| (*word == "of").then_some(idx))
    else {
        return Ok(None);
    };
    let owner_tokens = trim_commas(&tail_tokens[of_idx + 1..]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let owner_filter = parse_object_filter_lexed(&owner_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported loyalty-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    Ok(Some(owner_filter))
}

fn parse_possessive_ability_trigger_tail_lexed<'a>(
    tail_tokens: &'a [OwnedLexToken],
    tail_words: &[&str],
) -> Result<Option<(ObjectFilter, Option<String>)>, CardTextError> {
    let Some(ability_idx) = find_token_shape(tail_tokens, &ABILITY_OR_ABILITIES_PATTERN) else {
        return Ok(None);
    };
    if ability_idx == 0 || ability_idx + 1 != tail_tokens.len() {
        return Ok(None);
    }

    let owner_tokens = &tail_tokens[..ability_idx];
    let owner_words = &tail_words[..ability_idx];
    let Some(possessive_idx) = owner_words.iter().rposition(|word| {
        word.ends_with('s') && !NON_POSSESSIVE_PLURAL_SUFFIX_EXCLUSION_PATTERN.matches_word(word)
    }) else {
        return Ok(None);
    };

    let owner_subject_tokens = &owner_tokens[..=possessive_idx];
    if owner_subject_tokens.is_empty() {
        return Ok(None);
    }
    let owner_filter = parse_object_filter_lexed(owner_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported activated-ability trigger source filter (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;

    let marker = if possessive_idx + 1 < owner_words.len() {
        Some(owner_words[possessive_idx + 1].to_string())
    } else {
        None
    };

    Ok(Some((owner_filter, marker)))
}

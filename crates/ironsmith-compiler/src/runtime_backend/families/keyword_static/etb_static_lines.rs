use ironsmith_core::ValueSurfaceHint;

const ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["if"], &["when"], &["whenever"], &["as"], &["at"]]);
const ETB_TRIGGER_INTRO_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["if"], &["when"], &["whenever"], &["as"]]);
const ETB_AS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["as"]);
const ETB_THIS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this"]);

const ENTERS_WITH_COUNTERS_REQUIRED_WORDS: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["counter", "counters"]]);

const SOURCE_PRONOUN_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["its"]]);
const ETB_IT_OR_SPELL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["spell"]]);

const ETB_IF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const ETB_WITH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["with"]);
const ETB_ENTER_OR_ENTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enter"], &["enters"]]);
const ETB_ENTER_OR_ENTERS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["enter", "enters"]]);
const ETB_ENTERS_OR_ESCAPES_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["enters"], &["escapes"]]);
const ETB_ESCAPES_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["escapes"]);
const ETB_ARTICLE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact_any & [&["a"], &["an"]]);
const ETB_ONE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["one"]);
const ETB_THE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["the"]);
const ETB_POWER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["power"]);
const ETB_TOUGHNESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["toughness"]);
const ETB_MANA_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["mana"]);
const ETB_VALUE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["value"]);
const ETB_MINUS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["minus"]);
const ETB_GREATEST_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["greatest"]);
const ETB_MANA_VALUE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["mana_value"]);
const ETB_ADDITIONAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["additional"]);
const ETB_COUNTER_OR_COUNTERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["counter"], &["counters"]]);
const ETB_SOURCE_TAIL_HEAD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this"], &["thiss"]]);
const ETB_SOURCE_TAIL_NOUN_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["source"],
            &["spell"],
            &["card"],
            &["creature"],
            &["permanent"],
        ]
);
const ETB_IF_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["if"]);
const ETB_UNLESS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["unless"]);
const ETB_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const ETB_UNLESS_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["unless"]);
const ETB_TAPPED_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["tapped"]);
const ETB_UNTAPPED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["untapped"]);
const ETB_COPY_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["copy"]);
const ETB_PLAYED_BY_YOUR_OPPONENTS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["played", "by", "your", "opponents"]);
const ETB_PLAYED_BY_AN_OPPONENT_SUFFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    suffix_any
        & [
            &["played", "by", "an", "opponent"],
            &["played", "by", "a", "opponent"],
        ]
);
const ETB_PLAYED_BY_OPPONENTS_SUFFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["played", "by", "opponents"]);
const ETB_AS_THIS_LAND_ENTERS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "this", "land", "enters"]);
const ETB_REVEAL_FROM_HAND_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["reveal", "from", "hand"]);
const ETB_REVEAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["reveal"]);
const ETB_FROM_YOUR_HAND_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["from", "your", "hand"]);
const ETB_IF_YOU_DONT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["if", "you", "dont"], &["if", "you", "don't"]]);
const ETB_LAND_REVEAL_TRAILING_TAPPED_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["this", "land", "enters", "tapped"],
            &["this", "land", "enter", "tapped"],
            &["it", "enters", "tapped"],
            &["it", "enter", "tapped"],
            &["it", "enters", "the", "battlefield", "tapped"],
            &["it", "enter", "the", "battlefield", "tapped"],
        ]
);
const ETB_ENTERS_TAPPED_PHRASE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["enters", "tapped"], &["enter", "tapped"]]]
);
const ETB_CONTROL_OR_CONTROLS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"]]);
const ETB_CONTROL_OWN_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"], &["own"], &["owns"]]);
const ETB_A_PLAYER_HAS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["a", "player", "has"]);
const ETB_LIFE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const ETB_YOU_HAVE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["you", "have"]);
const ETB_OPPONENTS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["opponents"]);
const ETB_FIRST_THREE_TURNS_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "it", "s", "your", "first", "second", "or", "third", "turn", "of", "the", "game",
            ],
            &[
                "it's", "your", "first", "second", "or", "third", "turn", "of", "the", "game",
            ],
        ]
);
const ETB_YOU_CONTROL_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "control"], &["you", "controls"]]);
const ETB_ATTACKED_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "attacked", "this", "turn"],
            &["youve", "attacked", "this", "turn"]
        ]
);
const ETB_SOURCE_WAS_CAST_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["you", "cast", "it"],
            &["you", "cast", "this"],
            &["you", "cast", "this", "spell"],
        ]
);
const ETB_THIS_SPELL_WAS_KICKED_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "spell", "was", "kicked"],
            &["this", "creature", "was", "kicked"],
            &["this", "permanent", "was", "kicked"],
            &["it", "was", "kicked"],
        ]
);
const ETB_THIS_SPELL_ESCAPED_CONDITION_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["this", "spell", "escaped"], &["it", "escaped"]]);
const ETB_CREATURE_DIED_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["a", "creature", "died", "this", "turn"],
            &["one", "or", "more", "creatures", "died", "this", "turn"],
        ]
);
const ETB_OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
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
const ETB_PERMANENT_LEFT_UNDER_YOUR_CONTROL_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "a",
                "permanent",
                "left",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
                "this",
                "turn",
            ],
            &[
                "one",
                "or",
                "more",
                "permanents",
                "left",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
                "this",
                "turn",
            ],
        ]
);
const ETB_NOT_CAST_OR_NO_MANA_SPENT_CONDITION_PATTERN: ClauseShape<'static> = clause_shape!(
    exact
        & [
            "it", "wasnt", "cast", "or", "no", "mana", "was", "spent", "to", "cast", "it",
        ]
);
const ETB_X_IS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["x", "is"]);
const ETB_YOU_CAST_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["youve", "cast"],
            &["you've", "cast"],
            &["you", "ve", "cast"],
            &["you", "cast"],
            &["you", "have", "cast"],
        ]
);
const ETB_YOU_CAST_COUNT_AT_THIRD_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["you", "ve", "cast"], &["you", "have", "cast"]]);
const ETB_SPELLS_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["this", "turn"]; contains_any_words & [&["spell", "spells"]]);
const ETB_COLORS_MANA_SPENT_CONDITION_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [&["of", "mana"], &["spent", "to", "cast"]];
    contains_any_words & [&["color", "colors"], &["it", "this"]]
);
const ETB_COLORS_MANA_SPENT_TO_CAST_SOURCE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(
        exact_any
            & [
                &[
                    "color", "of", "mana", "was", "spent", "to", "cast", "it",
                ],
                &[
                    "color", "of", "mana", "was", "spent", "to", "cast", "this",
                ],
                &[
                    "color", "of", "mana", "were", "spent", "to", "cast", "it",
                ],
                &[
                    "color", "of", "mana", "were", "spent", "to", "cast", "this",
                ],
                &[
                    "colors", "of", "mana", "was", "spent", "to", "cast", "it",
                ],
                &[
                    "colors", "of", "mana", "was", "spent", "to", "cast", "this",
                ],
                &[
                    "colors", "of", "mana", "were", "spent", "to", "cast", "it",
                ],
                &[
                    "colors", "of", "mana", "were", "spent", "to", "cast", "this",
                ],
            ]
    );
const ETB_WHERE_X_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["where", "x", "is"]);
const ETB_EQUAL_TO_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["equal", "to"]);
const ETB_EQUAL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equal"]);
const ETB_EQUAL_TO_MANA_SPENT_TO_CAST_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["equal", "to", "the", "amount", "of", "mana", "spent", "to", "cast"]);
const ETB_PLUS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["plus"]);
const ETB_PLUS_OR_MINUS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["plus"], &["minus"]]);
const ETB_NUMBER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["number"]);
const ETB_OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const ETB_OF_OR_AMONG_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["of"], &["among"]]);
const ETB_DIFFERENTLY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["differently"]);
const ETB_NAMED_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["named"]);
const ETB_DIFFERENT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["different"]);
const ETB_POWER_OR_POWERS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["power"], &["powers"]]);
const ETB_AMONG_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["among"]);
const ETB_DEVOTION_VALUE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["devotion"]);
const ETB_ALL_PLAYERS_HAND_COUNT_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["cards", "in", "all", "players"];
    contains_any_words & [&["hand", "hands"]]
);
const ETB_SAME_NAME_AS_TRIGGERING_SPELL_GRAVEYARD_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "the",
                "number",
                "of",
                "cards",
                "in",
                "all",
                "graveyards",
                "with",
                "the",
                "same",
                "name",
                "as",
                "the",
                "spell",
            ],
            &[
                "the",
                "number",
                "of",
                "cards",
                "in",
                "all",
                "graveyards",
                "with",
                "the",
                "same",
                "name",
                "as",
                "that",
                "spell",
            ],
        ]
);
const ETB_EXILED_CARD_MANA_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["the", "mana", "value", "of", "the", "exiled", "card"],
            &["the", "exiled", "card", "mana", "value"],
            &["the", "exiled", "cards", "mana", "value"],
        ]
);
const ETB_TRIGGERING_SPELL_MANA_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "spell", "mana", "value"],
            &["that", "spell's", "mana", "value"],
            &["that", "spells", "mana", "value"],
        ]
);
const ETB_YOUR_HAND_COUNT_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["cards", "in", "your"];
    contains_any_words & [&["hand", "hands"]]
);
const ETB_COMMON_CREATURE_TYPE_VALUE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["creature", "type", "common"]);
const ETB_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["basic", "land", "type", "among"], &["basic", "land", "types", "among"]]);
const ETB_CREATURE_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["creature", "type", "among"], &["creature", "types", "among"]]);
const ETB_COLORS_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["color", "among"], &["colors", "among"]]);
const ETB_CARD_TYPES_AMONG_CARDS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["card", "type", "among", "cards"],
            &["card", "types", "among", "cards"],
        ]
);
const ETB_CARD_TYPES_AMONG_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["card", "type", "among"], &["card", "types", "among"]]);
const ETB_GRAVEYARD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["graveyard"]);
const ETB_AND_GRAVEYARD_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["and", "graveyard"]);
const ETB_SACRIFICED_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["sacrificed"]);
const ETB_MANA_VALUE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["mana", "value"]);
const ETB_SACRIFICED_CREATURE_POWER_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "sacrificed", "creature", "power"],
            &["the", "sacrificed", "creatures", "power"],
            &["sacrificed", "creature", "power"],
            &["sacrificed", "creatures", "power"],
        ]
);
const ETB_SACRIFICED_CREATURE_TOUGHNESS_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "sacrificed", "creature", "toughness"],
            &["the", "sacrificed", "creatures", "toughness"],
            &["sacrificed", "creature", "toughness"],
            &["sacrificed", "creatures", "toughness"],
        ]
);
const ETB_TAGGED_CREATURE_MANA_VALUE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["the", "mana", "value", "of", "the", "sacrificed", "creature"],
            &["the", "mana", "value", "of", "the", "sacrificed", "creatures"],
            &["mana", "value", "of", "the", "sacrificed", "creature"],
            &["mana", "value", "of", "the", "sacrificed", "creatures"],
            &["the", "sacrificed", "creature", "mana", "value"],
            &["the", "sacrificed", "creatures", "mana", "value"],
            &["sacrificed", "creature", "mana", "value"],
            &["sacrificed", "creatures", "mana", "value"],
            &["the", "mana", "value", "of", "the", "exiled", "creature"],
            &["the", "mana", "value", "of", "the", "exiled", "creature's"],
            &["the", "mana", "value", "of", "the", "exiled", "creatures"],
            &["mana", "value", "of", "the", "exiled", "creature"],
            &["mana", "value", "of", "the", "exiled", "creature's"],
            &["mana", "value", "of", "the", "exiled", "creatures"],
            &["the", "exiled", "creature", "mana", "value"],
            &["the", "exiled", "creature's", "mana", "value"],
            &["the", "exiled", "creatures", "mana", "value"],
            &["exiled", "creature", "mana", "value"],
            &["exiled", "creature's", "mana", "value"],
            &["exiled", "creatures", "mana", "value"],
        ]
);
const ETB_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["your", "graveyard"]]);
const ETB_OPPONENT_GRAVEYARD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["opponents", "graveyard"], &["opponent", "graveyard"]]]);
const ETB_AS_LONG_AS_THIS_IN_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["as", "long", "as", "this"];
    contains_phrases & [&["is", "in", "your", "graveyard"]]
);
const ETB_WITH_ADDITIONAL_COUNTERS_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["with", "additional"];
    contains_any_words & [&["counter", "counters"]]
);
const ETB_IT_BECOMES_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["it", "becomes"]);
const ETB_IN_ADDITION_TO_ITS_OTHER_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["in", "addition", "to", "its", "other", "types"],
            &["in", "addition", "to", "its", "other", "type"],
        ]]
);
const ETB_IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["in", "addition", "to", "its", "other"]);
const ETB_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);


fn etb_find_prefix_shape_start(words: &[&str], shape: &ClauseShape<'static>) -> Option<usize> {
    (0..words.len()).find(|&idx| shape.matches_words(&words[idx..]))
}


const ETB_SELF_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "creature"],
            &["this", "permanent"],
            &["this", "object"],
        ]
);
const ETB_FACE_UP_CHOICE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix
        & [
            "or", "is", "turned", "face", "up", "it", "becomes", "your", "choice", "of",
        ]
);
const ETB_YOUR_PARTY_SIZE_VALUE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["party", "your"];
    contains_any_words & [&["creature", "creatures"]]
);

const ENTERS_WITH_COUNTER_PLUS_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["plus"]);

const ENTERS_WITH_COUNTER_DIED_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["for", "each", "creature", "that", "died", "this", "turn"],
            &["for", "each", "creatures", "that", "died", "this", "turn",],
        ]
);

const ENTERS_WITH_COUNTER_MANA_COLORS_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "for", "each", "color", "of", "mana", "spent", "to", "cast", "it"
            ],
            &[
                "for", "each", "colour", "of", "mana", "spent", "to", "cast", "it",
            ],
        ]
);

const ENTERS_WITH_COUNTER_CONTROLLED_DIED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "for", "each", "creature", "that", "died", "under", "your", "control", "this",
                "turn",
            ],
            &[
                "for",
                "each",
                "creatures",
                "that",
                "died",
                "under",
                "your",
                "control",
                "this",
                "turn",
            ],
        ]
);

const ENTERS_WITH_COUNTER_KICKED_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["for", "each", "time", "it", "was", "kicked"],
            &["for", "each", "time", "this", "spell", "was", "kicked"],
        ]
);

const ENTERS_WITH_COUNTER_MAGIC_LOSSES_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "for",
                "each",
                "magic",
                "game",
                "you",
                "have",
                "lost",
                "to",
                "one",
                "of",
                "your",
                "opponents",
                "since",
                "you",
                "last",
                "won",
                "a",
                "game",
                "against",
                "them",
            ],
            &[
                "for",
                "each",
                "magic",
                "games",
                "you",
                "have",
                "lost",
                "to",
                "one",
                "of",
                "your",
                "opponents",
                "since",
                "you",
                "last",
                "won",
                "a",
                "game",
                "against",
                "them",
            ],
        ]
);

const ENTERS_WITH_COUNTER_FOR_EACH_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["for", "each"]);
const ENTERS_WITH_COUNTER_EQUAL_TO_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["equal", "to"]);
const ENTERS_WITH_ADDED_ABILITIES_AND_WITH_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["and", "with"]);
const ENTERS_WITH_ADDED_ABILITIES_WITH_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["with"]);
const CAN_ATTACK_AS_THOUGH_NO_DEFENDER_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &[
                "this", "creature", "can", "attack", "as", "though", "it", "didnt", "have",
                "defender",
            ],
            &[
                "this", "creature", "can", "attack", "as", "though", "it", "didn't", "have",
                "defender",
            ],
            &[
                "this", "creature", "can", "attack", "as", "though", "it", "doesnt", "have",
                "defender",
            ],
            &[
                "this", "creature", "can", "attack", "as", "though", "it", "doesn't", "have",
                "defender",
            ],
        ]
);

fn etb_starts_with_trigger_intro_after_label(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, body_tokens)) = split_em_dash_label_prefix(tokens) else {
        return false;
    };
    ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN
        .matches_words(&crate::runtime_backend::lexer::token_word_refs(body_tokens))
}

pub(crate) fn parse_enters_tapped_with_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }

    let enters_idx = ETB_ENTER_OR_ENTERS_WORD_PATTERN.find_word(&clause_words);
    let Some(enters_idx) = enters_idx else {
        return Ok(None);
    };
    let with_idx = ETB_WITH_WORD_PATTERN.find_word(&clause_words);
    let Some(with_idx) = with_idx else {
        return Ok(None);
    };
    if with_idx <= enters_idx {
        return Ok(None);
    }

    let tapped_between = ETB_TAPPED_MARKER_PATTERN.matches_words(&clause_words[enters_idx + 1..with_idx]);
    if !tapped_between {
        return Ok(None);
    }
    if !ENTERS_WITH_COUNTERS_REQUIRED_WORDS.matches_words(&clause_words) {
        return Ok(None);
    }
    if !is_source_reference_words(&clause_words[..enters_idx]) {
        return Ok(None);
    }

    let Some(counters) = parse_enters_with_counters_line(tokens)? else {
        return Ok(None);
    };

    Ok(Some(vec![StaticAbility::enters_tapped_ability(), counters]))
}

pub(crate) fn parse_enters_with_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let full_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    let mut condition: Option<(crate::ConditionExpr, String)> = None;
    let mut clause_tokens: Vec<OwnedLexToken> = tokens.to_vec();

    // Support leading conditional form:
    // "If <condition>, it enters with ..."
    if clause_tokens
        .first()
        .is_some_and(|token| ETB_IF_WORD_PATTERN.matches_token(token))
        && let Some(comma_idx) =
            crate::runtime_backend::grammar::primitives::find_token_index(&clause_tokens, |token| {
                token.is_comma()
            })
    {
        let condition_tokens = trim_commas(&clause_tokens[1..comma_idx]);
        if !condition_tokens.is_empty() {
            let Some(parsed) = parse_enters_with_counter_condition_clause(&condition_tokens) else {
                return Ok(None);
            };
            let display =
                crate::runtime_backend::lexer::token_word_refs(&condition_tokens).join(" ");
            condition = Some((parsed, display));
            clause_tokens = trim_commas(&clause_tokens[comma_idx + 1..]);
        }
    }

    let clause_words = crate::runtime_backend::lexer::token_word_refs(&clause_tokens);
    let Some(verb_idx) = ETB_ENTERS_OR_ESCAPES_WORD_PATTERN.find_word(&clause_words)
    else {
        return Ok(None);
    };
    let escaped_line = clause_words
        .get(verb_idx)
        .is_some_and(|word| ETB_ESCAPES_WORD_PATTERN.matches_word(word));
    if escaped_line {
        condition = Some((
            crate::ConditionExpr::ThisSpellEscaped,
            "it escaped".to_string(),
        ));
    }
    let Some(enter_token_idx) = token_index_for_word_index(&clause_tokens, verb_idx) else {
        return Ok(None);
    };
    if clause_tokens[..enter_token_idx]
        .iter()
        .any(|token| token.is_period() || token.is_colon() || token.is_semicolon())
    {
        return Ok(None);
    }
    let subject_words = clause_words.get(..verb_idx).unwrap_or_default();
    let source_pronoun_subject = SOURCE_PRONOUN_SUBJECT_PATTERN.matches_words(subject_words);
    if !is_source_reference_words(subject_words) && !source_pronoun_subject {
        return Ok(None);
    }
    if !clause_words
        .iter()
        .any(|word| ETB_WITH_WORD_PATTERN.matches_word(word))
        || !ENTERS_WITH_COUNTERS_REQUIRED_WORDS.matches_words(&clause_words)
    {
        return Ok(None);
    }

    let with_idx =
        crate::runtime_backend::grammar::primitives::find_token_index(&clause_tokens, |token| {
            ETB_WITH_WORD_PATTERN.matches_token(token)
        })
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'with' in enters-with-counters clause".to_string())
        })?;
    let mut added_abilities: Vec<Ability> = Vec::new();
    let mut after_with = &clause_tokens[with_idx + 1..];
    if let Some((and_with_idx, and_with_end)) =
        crate::runtime_backend::lexer::find_token_word_sequence_span(after_with, &["and", "with"])
    {
        let ability_prefix = trim_commas(&after_with[..and_with_idx]);
        if let Some(abilities) = parse_enters_with_added_abilities_prefix(&ability_prefix) {
            added_abilities.extend(abilities);
            after_with = &after_with[and_with_end..];
        }
    }
    let (mut count, used) = if after_with
        .first()
        .is_some_and(|token| ETB_ARTICLE_WORD_PATTERN.matches_token(token))
        && after_with
            .get(1)
            .is_some_and(|token| ETB_ADDITIONAL_WORD_PATTERN.matches_token(token))
    {
        if let Some((value, value_used)) = parse_value(&after_with[2..]) {
            (value, 2 + value_used)
        } else {
            (Value::Fixed(1), 2)
        }
    } else {
        parse_value(after_with).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter count in self ETB counters (clause: '{}')",
                full_words.join(" ")
            ))
        })?
    };

    let counter_type = parse_counter_type_from_tokens(&after_with[used..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for self ETB counters (clause: '{}')",
            full_words.join(" ")
        ))
    })?;

    let counter_idx =
        crate::runtime_backend::grammar::primitives::find_token_index(after_with, |token| {
            ETB_COUNTER_OR_COUNTERS_WORD_PATTERN.matches_token(token)
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter keyword for self ETB counters (clause: '{}')",
                full_words.join(" ")
            ))
        })?;
    let mut tail = &after_with[counter_idx + 1..];
    if token_slice_first_is(tail, "on") {
        tail = &tail[1..];
    }
    if token_slice_first_is(tail, "it") {
        tail = &tail[1..];
    } else if tail
        .first()
        .is_some_and(|token| ETB_SOURCE_TAIL_HEAD_PATTERN.matches_token(token))
    {
        tail = &tail[1..];
        if let Some(word) = tail.first().and_then(OwnedLexToken::as_word)
            && (ETB_SOURCE_TAIL_NOUN_WORD_PATTERN.matches_word(word)
                || parse_card_type(word).is_some())
        {
            tail = &tail[1..];
        }
    }
    let tail = trim_commas(tail);
    let tail_has_words = tail.iter().any(|token| token.as_word().is_some());
    if tail_has_words {
        let tail_words = tail
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>();
        let scaled_for_each_count = |dynamic: Value, base_count: &Value| match base_count {
            Value::Fixed(multiplier) => scale_dynamic_cost_modifier_value(dynamic, *multiplier),
            _ => dynamic,
        };
        if let Some(abilities) = parse_enters_with_added_abilities_tail(&tail) {
            added_abilities = abilities;
        } else if tail_words
            .first()
            .is_some_and(|word| ETB_IF_TAIL_PATTERN.matches_word(word))
        {
            let condition_tokens = trim_commas(&tail[1..]);
            let parsed =
                parse_enters_with_counter_condition_clause(&condition_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported enters-with-counter condition (clause: '{}')",
                        full_words.join(" ")
                    ))
                })?;
            let display =
                crate::runtime_backend::lexer::token_word_refs(&condition_tokens).join(" ");
            condition = Some(combine_enters_with_counter_conditions(
                condition,
                (parsed, display),
            ));
        } else if tail_words
            .first()
            .is_some_and(|word| ETB_UNLESS_TAIL_PATTERN.matches_word(word))
        {
            let condition_tokens = trim_commas(&tail[1..]);
            let parsed =
                parse_enters_with_counter_condition_clause(&condition_tokens).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported enters-with-counter unless condition (clause: '{}')",
                        full_words.join(" ")
                    ))
                })?;
            let display = parse_unless_enters_with_counter_condition_display(&condition_tokens)
                .unwrap_or_else(|| {
                    format!(
                        "not {}",
                        crate::runtime_backend::lexer::token_word_refs(&condition_tokens).join(" ")
                    )
                });
            condition = Some(combine_enters_with_counter_conditions(
                condition,
                (crate::ConditionExpr::Not(Box::new(parsed)), display),
            ));
        } else if ENTERS_WITH_COUNTER_PLUS_TAIL_PATTERN.matches_words(&tail_words) {
            let for_each_idx = crate::runtime_backend::lexer::find_token_word_sequence_span(
                &tail,
                &["for", "each"],
            )
            .map(|(idx, _)| idx);
            if let Some(for_each_idx) = for_each_idx {
                let extra =
                    parse_dynamic_cost_modifier_value(&tail[for_each_idx..])?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported additional self ETB counter clause (clause: '{}')",
                            full_words.join(" ")
                        ))
                    })?;
                count = Value::Add(Box::new(count), Box::new(extra));
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported plus-self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                )));
            }
        } else if ENTERS_WITH_COUNTER_DIED_THIS_TURN_TAIL_PATTERN.matches_words(&tail_words) {
            count = scaled_for_each_count(Value::CreaturesDiedThisTurn, &count);
        } else if ENTERS_WITH_COUNTER_MANA_COLORS_TAIL_PATTERN.matches_words(&tail_words) {
            count = scaled_for_each_count(Value::ColorsOfManaSpentToCastThisSpell, &count);
        } else if ENTERS_WITH_COUNTER_CONTROLLED_DIED_TAIL_PATTERN.matches_words(&tail_words) {
            count = scaled_for_each_count(
                Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You),
                &count,
            );
        } else if ENTERS_WITH_COUNTER_KICKED_TAIL_PATTERN.matches_words(&tail_words) {
            count = scaled_for_each_count(Value::KickCount, &count);
        } else if ENTERS_WITH_COUNTER_MAGIC_LOSSES_TAIL_PATTERN.matches_words(&tail_words) {
            count = Value::MagicGamesLostToOpponentsSinceLastWin;
        } else if ENTERS_WITH_COUNTER_FOR_EACH_TAIL_PATTERN.matches_words(&tail_words) {
            count = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported for-each self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
        } else if ENTERS_WITH_COUNTER_EQUAL_TO_TAIL_PATTERN.matches_words(&tail_words) {
            count = parse_enters_with_counter_equal_to_value_clause(&tail).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported equal-to self ETB counter clause (clause: '{}')",
                    full_words.join(" ")
                ))
            })?;
        } else {
            count = parse_value_binding_clause(&tail)
                .map(|value| value.with_surface_hint(ValueSurfaceHint::WhereXIs))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported trailing self ETB counter clause (clause: '{}')",
                        full_words.join(" ")
                    ))
                })?;
        }
    }

    if let Some((condition, display)) = condition {
        return Ok(Some(
            StaticAbility::enters_with_counters_and_abilities_if_condition(
                counter_type,
                count,
                condition,
                display,
                added_abilities,
            ),
        ));
    }

    if !added_abilities.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "self ETB counter granted abilities require a condition (clause: '{}')",
            full_words.join(" ")
        )));
    }

    Ok(Some(StaticAbility::enters_with_counters_value(
        counter_type,
        count,
    )))
}

fn parse_enters_with_added_abilities_tail(tokens: &[OwnedLexToken]) -> Option<Vec<Ability>> {
    let tail = trim_commas(tokens);
    let words = crate::runtime_backend::lexer::token_word_refs(&tail);
    let ability_tokens = if ENTERS_WITH_ADDED_ABILITIES_AND_WITH_TAIL_PATTERN.matches_words(&words)
    {
        &tail[2..]
    } else if ENTERS_WITH_ADDED_ABILITIES_WITH_TAIL_PATTERN.matches_words(&words) {
        &tail[1..]
    } else {
        return None;
    };
    let ability_words = crate::runtime_backend::lexer::token_word_refs(ability_tokens);
    if CAN_ATTACK_AS_THOUGH_NO_DEFENDER_PATTERN.matches_words(&ability_words) {
        return Some(vec![Ability::static_ability(
            StaticAbility::can_attack_as_though_no_defender(),
        )]);
    }

    let actions = parse_ability_line(ability_tokens)?;
    let mut abilities = Vec::new();
    for action in actions {
        let static_ability =
            super::static_ability_helpers::static_ability_for_keyword_action(action)?;
        abilities.push(Ability::static_ability(static_ability));
    }
    (!abilities.is_empty()).then_some(abilities)
}

fn parse_enters_with_added_abilities_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<Ability>> {
    let actions = parse_ability_line(tokens)?;
    let mut abilities = Vec::new();
    for action in actions {
        let static_ability =
            super::static_ability_helpers::static_ability_for_keyword_action(action)?;
        abilities.push(Ability::static_ability(static_ability));
    }
    (!abilities.is_empty()).then_some(abilities)
}

fn combine_enters_with_counter_conditions(
    existing: Option<(crate::ConditionExpr, String)>,
    next: (crate::ConditionExpr, String),
) -> (crate::ConditionExpr, String) {
    match existing {
        Some((existing_condition, existing_display)) => {
            let combined_condition =
                crate::ConditionExpr::And(Box::new(existing_condition), Box::new(next.0));
            let combined_display =
                match (existing_display.trim().is_empty(), next.1.trim().is_empty()) {
                    (true, true) => String::new(),
                    (false, true) => existing_display,
                    (true, false) => next.1,
                    (false, false) => format!("{} and {}", existing_display.trim(), next.1.trim()),
                };
            (combined_condition, combined_display)
        }
        None => next,
    }
}

fn parse_etb_at_least_quantity_at(
    tokens: &[OwnedLexToken],
    start: usize,
) -> Option<(u32, usize)> {
    let (comparison, used) = parse_quantity_comparison_prefix(
        tokens.get(start..).unwrap_or_default(),
        false,
        false,
        "enters-with condition",
    )
    .ok()?;
    let count = crate::runtime_backend::util::comparison_to_strict_at_least_threshold(&comparison)?;
    Some((count, start + used))
}

fn parse_unless_enters_with_counter_condition_display(tokens: &[OwnedLexToken]) -> Option<String> {
    let condition_words = crate::runtime_backend::lexer::token_word_refs(tokens);
    if condition_words.len() >= 9
        && let Some((amount, rest_start)) = parse_etb_at_least_quantity_at(tokens, 0)
        && ETB_COLORS_MANA_SPENT_TO_CAST_SOURCE_TAIL_PATTERN
            .matches_words(&condition_words[rest_start..])
    {
        return Some(format!(
            "fewer than {amount} colors of mana were spent to cast it"
        ));
    }
    None
}

fn parse_enters_with_counter_condition_clause(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let condition_tokens = trim_edge_punctuation(tokens);
    let condition_words = crate::runtime_backend::lexer::token_word_refs(&condition_tokens);
    if condition_words.is_empty() {
        return None;
    }

    if ETB_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::AttackedThisTurn);
    }
    if ETB_SOURCE_WAS_CAST_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::SourceWasCast);
    }
    if ETB_THIS_SPELL_WAS_KICKED_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::ThisSpellWasKicked);
    }
    if ETB_THIS_SPELL_ESCAPED_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::ThisSpellEscaped);
    }
    if ETB_CREATURE_DIED_THIS_TURN_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::CreatureDiedThisTurn);
    }
    if ETB_OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::OpponentLostLifeThisTurn);
    }
    if ETB_PERMANENT_LEFT_UNDER_YOUR_CONTROL_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::PermanentLeftBattlefieldUnderYourControlThisTurn);
    }
    if ETB_NOT_CAST_OR_NO_MANA_SPENT_CONDITION_PATTERN.matches_words(&condition_words) {
        return Some(crate::ConditionExpr::Or(
            Box::new(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::SourceWasCast,
            ))),
            Box::new(crate::ConditionExpr::Not(Box::new(
                crate::ConditionExpr::ManaSpentToCastThisSpellAtLeast {
                    amount: 1,
                    symbol: None,
                },
            ))),
        ));
    }

    if condition_words.len() >= 4
        && ETB_X_IS_PREFIX_PATTERN.matches_words(&condition_words)
        && let Some((amount, rest_start)) = parse_etb_at_least_quantity_at(&condition_tokens, 2)
        && rest_start == condition_words.len()
    {
        return Some(crate::ConditionExpr::XValueAtLeast(amount));
    }

    if condition_words.len() >= 7 {
        let (count_word_idx, valid_prefix) =
            if ETB_YOU_CAST_PREFIX_PATTERN.matches_words(&condition_words) {
                let count_word_idx = if ETB_YOU_CAST_COUNT_AT_THIRD_WORD_PATTERN
                    .matches_words(&condition_words)
                {
                    3usize
                } else {
                    2usize
                };
                (count_word_idx, true)
            } else {
                (0usize, false)
        };
        if valid_prefix
            && let Some((amount, rest_start)) =
                parse_etb_at_least_quantity_at(&condition_tokens, count_word_idx)
            && ETB_SPELLS_THIS_TURN_TAIL_PATTERN.matches_words(&condition_words[rest_start..])
        {
            return Some(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                player: PlayerFilter::You,
                count: amount,
            });
        }
    }

    if condition_words.len() >= 9
        && let Some((amount, rest_start)) = parse_etb_at_least_quantity_at(&condition_tokens, 0)
        && ETB_COLORS_MANA_SPENT_CONDITION_TAIL_PATTERN
            .matches_words(&condition_words[rest_start..])
    {
        return Some(crate::ConditionExpr::ColorsOfManaSpentToCastThisSpellOrMore(amount));
    }

    if let Some(amount) =
        crate::runtime_backend::grammar::filters::parse_same_color_mana_spent_to_cast_predicate(
            &condition_words,
        )
    {
        return Some(crate::ConditionExpr::SameColorManaSpentToCastThisSpellAtLeast(amount));
    }

    parse_static_condition_clause(&condition_tokens).ok()
}

fn parse_enters_with_counter_equal_to_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let words_all = crate::runtime_backend::token_word_refs(&trimmed);
    if !ETB_EQUAL_TO_PREFIX_PATTERN.matches_words(&words_all) {
        return None;
    }
    if ETB_EQUAL_TO_MANA_SPENT_TO_CAST_PREFIX_PATTERN.matches_words(&words_all)
        && words_all
            .last()
            .is_some_and(|word| ETB_IT_OR_SPELL_WORD_PATTERN.matches_word(word))
    {
        return Some(Value::ManaSpentToCastThisSpell.with_surface_hint(ValueSurfaceHint::EqualTo));
    }

    if trimmed.len() < 2 {
        return None;
    }

    let mut where_tokens = Vec::with_capacity(trimmed.len() + 1);
    where_tokens.push(OwnedLexToken::word(
        "where".to_string(),
        TextSpan::synthetic(),
    ));
    where_tokens.push(OwnedLexToken::word("x".to_string(), TextSpan::synthetic()));
    where_tokens.push(OwnedLexToken::word("is".to_string(), TextSpan::synthetic()));
    where_tokens.extend_from_slice(&trimmed[2..]);

    parse_value_binding_clause(&where_tokens)
        .or_else(|| parse_equal_to_greatest_cards_drawn_this_turn_value(&trimmed))
        .or_else(|| parse_add_mana_equal_amount_value(&trimmed))
        .or_else(|| parse_equal_to_aggregate_filter_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_filter_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(&trimmed))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(&trimmed))
        .map(|value| {
            value
                .into_unhinted()
                .with_surface_hint(ValueSurfaceHint::EqualTo)
        })
}

fn parse_equal_to_greatest_cards_drawn_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    if words_all
        == [
            "equal", "to", "the", "greatest", "number", "of", "cards", "an", "opponent", "has",
            "drawn", "this", "turn",
        ]
        || words_all
            == [
                "equal", "to", "greatest", "number", "of", "cards", "an", "opponent", "has",
                "drawn", "this", "turn",
            ]
    {
        return Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent));
    }
    None
}

pub(crate) fn parse_value_binding_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }

    if let Some(value) = parse_where_x_source_stat_value(tokens) {
        return Some(value);
    }

    if let Some(value) =
        crate::runtime_backend::front_end::grammar::values::parse_players_who_control_more_than_you_value_lexed(tokens)
    {
        return Some(value);
    }

    if let Some(value) = parse_where_x_life_gained_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_life_lost_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_opponents_dealt_combat_damage_this_turn_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_noncombat_damage_to_opponents_value(tokens) {
        return Some(value);
    }

    if let Some(value) = parse_where_x_draft_noted_highest_number_value(&words) {
        return Some(value);
    }

    match words.get(3..) {
        Some(
            [
                "the",
                "number",
                "of",
                "times",
                "this",
                "ability",
                "has",
                "resolved",
                "this",
                "turn",
            ],
        )
        | Some(
            [
                "number",
                "of",
                "times",
                "this",
                "ability",
                "has",
                "resolved",
                "this",
                "turn",
            ],
        ) => {
            return Some(Value::ThisAbilityResolvedThisTurnCount);
        }
        Some(["your", "life", "total"]) => return Some(Value::LifeTotal(PlayerFilter::You)),
        Some(["half", "your", "life", "total"])
        | Some(["half", "your", "life", "total", "rounded", "up"]) => {
            return Some(Value::HalfLifeTotalRoundedUp(PlayerFilter::You));
        }
        Some(["half", "your", "life", "total", "rounded", "down"]) => {
            return Some(Value::HalfLifeTotalRoundedDown(PlayerFilter::You));
        }
        Some(["your", "speed"]) => return Some(Value::Speed(PlayerFilter::You)),
        Some(
            [
                "the",
                "amount",
                "of",
                "damage",
                "it",
                "dealt",
                "to",
                "that",
                "player",
            ],
        )
        | Some(
            [
                "amount",
                "of",
                "damage",
                "it",
                "dealt",
                "to",
                "that",
                "player",
            ],
        ) => return Some(Value::EventValue(EventValueSpec::Amount)),
        Some(["the", "number", "of", "opponents", "you", "have"])
        | Some(["number", "of", "opponents", "you", "have"])
        | Some(["the", "number", "of", "opponents"])
        | Some(["number", "of", "opponents"]) => {
            return Some(Value::CountPlayers(PlayerFilter::Opponent));
        }
        Some(["the", "number", "of", "players", "being", "attacked"])
        | Some(["number", "of", "players", "being", "attacked"]) => {
            return Some(Value::PlayersBeingAttacked);
        }
        Some(["target", "players", "life", "total"])
        | Some(["target", "player", "life", "total"]) => {
            return Some(Value::LifeTotal(PlayerFilter::target_player()));
        }
        Some(
            [
                "the",
                "difference",
                "between",
                "those",
                "players",
                "life",
                "totals",
            ],
        )
        | Some(
            [
                "difference",
                "between",
                "those",
                "players",
                "life",
                "totals",
            ],
        )
        | Some(
            [
                "the",
                "difference",
                "between",
                "the",
                "target",
                "players",
                "life",
                "totals",
            ],
        )
        | Some(
            [
                "difference",
                "between",
                "the",
                "target",
                "players",
                "life",
                "totals",
            ],
        ) => {
            return Some(Value::LifeTotalDifference(PlayerFilter::target_player()));
        }
        Some(["that", "players", "life", "total"]) | Some(["that", "player", "life", "total"]) => {
            return Some(Value::LifeTotal(PlayerFilter::target_player()));
        }
        Some(["that", "players", "speed"]) | Some(["that", "player", "speed"]) => {
            return Some(Value::Speed(PlayerFilter::target_player()));
        }
        Some(["the", "discarded", "cards", "mana", "value"])
        | Some(["the", "discarded", "card", "mana", "value"])
        | Some(["discarded", "cards", "mana", "value"])
        | Some(["discarded", "card", "mana", "value"]) => {
            return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
                TagKey::from("discarded_cost"),
            ))));
        }
        Some(
            [
                "the",
                "total",
                "mana",
                "value",
                "of",
                "all",
                "cards",
                "revealed",
                "this",
                "way",
            ],
        )
        | Some(
            [
                "the",
                "total",
                "mana",
                "value",
                "of",
                "cards",
                "revealed",
                "this",
                "way",
            ],
        )
        | Some(
            [
                "total",
                "mana",
                "value",
                "of",
                "all",
                "cards",
                "revealed",
                "this",
                "way",
            ],
        )
        | Some(
            [
                "total",
                "mana",
                "value",
                "of",
                "cards",
                "revealed",
                "this",
                "way",
            ],
        ) => {
            return Some(Value::TotalManaValue(ObjectFilter::tagged(TagKey::from(
                "__public_revealed",
            ))));
        }
        _ => {}
    }

    if let Some(value) = parse_where_x_is_aggregate_filter_value(tokens) {
        return Some(value);
    }

    // where X is your devotion to black
    if ETB_DEVOTION_VALUE_PATTERN.matches_words(&words) {
        if let Ok(Some(value)) = parse_devotion_value_from_add_clause(tokens) {
            return Some(value);
        }
    }

    // where X is the total number of cards in all players' hands
    if ETB_ALL_PLAYERS_HAND_COUNT_VALUE_PATTERN.matches_words(&words) {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }

    if words.get(3..).is_some_and(|tail| {
        ETB_SAME_NAME_AS_TRIGGERING_SPELL_GRAVEYARD_VALUE_PATTERN.matches_words(tail)
    }) {
        return Some(Value::Count(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .match_tagged(
                    TagKey::from("triggering"),
                    crate::filter::TaggedOpbjectRelation::SameNameAsTagged,
                ),
        ));
    }

    // where X is N plus the number of <objects>
    if let Some(value) = parse_where_x_is_fixed_plus_number_of_filter_value(tokens) {
        return Some(value);
    }

    // where X is N plus the sacrificed creature's mana value / power / toughness
    if let Some(value) = parse_where_x_is_fixed_plus_reference_value(tokens) {
        return Some(value);
    }

    // where X is the number of <objects> plus/minus N
    if let Some(value) = parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(tokens) {
        return Some(value);
    }

    if let Some(tail) = words.get(3..)
        && (ETB_EXILED_CARD_MANA_VALUE_PATTERN.matches_words(tail)
            || ETB_TRIGGERING_SPELL_MANA_VALUE_PATTERN.matches_words(tail))
    {
        let tag = if ETB_TRIGGERING_SPELL_MANA_VALUE_PATTERN.matches_words(tail) {
            "triggering"
        } else {
            IT_TAG
        };
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(tag),
        ))));
    }

    // where X is the number of cards in your hand
    if ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches_words(&words) {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }

    // where X is the number of creatures in your party
    if ETB_YOUR_PARTY_SIZE_VALUE_PATTERN.matches_words(&words) {
        return Some(Value::PartySize(PlayerFilter::You));
    }

    // where X is the number of differently named <objects>
    if let Some(value) = parse_where_x_is_number_of_differently_named_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of different powers among <objects>
    if let Some(value) = parse_where_x_is_number_of_different_powers_filter_value(tokens) {
        return Some(value);
    }

    // where X is the number of <objects>
    if let Some(value) = parse_where_x_is_number_of_filter_value(tokens) {
        return Some(value);
    }

    if let Some(tail) = words.get(3..)
        && let Some((value, used)) = parse_value_expr_words(tail)
        && used == tail.len()
    {
        return Some(value);
    }

    None
}

pub(crate) fn parse_value_binding_clause_lexed(
    tokens: &[crate::runtime_backend::lexer::OwnedLexToken],
) -> Option<Value> {
    parse_value_binding_clause(tokens)
}

fn parse_where_x_draft_noted_highest_number_value(words: &[&str]) -> Option<Value> {
    let tail = words.get(3..)?;
    let name_words = match tail {
        [
            "the",
            "highest",
            "number",
            "you",
            "noted",
            "for",
            "cards",
            "named",
            name @ ..,
        ]
        | [
            "highest",
            "number",
            "you",
            "noted",
            "for",
            "cards",
            "named",
            name @ ..,
        ] => name,
        _ => return None,
    };
    if name_words.is_empty() {
        return None;
    }
    Some(
        Value::DraftNotedHighestNumber {
            card_name: name_words.join(" "),
        }
        .with_surface_hint(ValueSurfaceHint::WhereXIs),
    )
}

pub(crate) fn parse_where_x_source_stat_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let word_view = crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens);
    let words = word_view.word_refs();
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }
    let tagged_it = ChooseSpec::Tagged(TagKey::from(IT_TAG));
    let tail = words.get(3..)?;
    if tail.len() >= 2
        && ETB_POWER_WORD_PATTERN.matches_last_word(tail)
        && let Some(surface) =
            source_reference_surface_for_possessive_words(&tail[..tail.len() - 1])
    {
        return Some(Value::PowerOf(Box::new(
            ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
        )));
    }
    if tail.len() >= 2
        && ETB_TOUGHNESS_WORD_PATTERN.matches_last_word(tail)
        && let Some(surface) =
            source_reference_surface_for_possessive_words(&tail[..tail.len() - 1])
    {
        return Some(Value::ToughnessOf(Box::new(
            ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
        )));
    }
    if tail.len() >= 3
        && ETB_MANA_VALUE_TAIL_PATTERN.matches_words(tail)
        && let Some(surface) =
            source_reference_surface_for_possessive_words(&tail[..tail.len() - 2])
    {
        return Some(Value::ManaValueOf(Box::new(
            ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
        )));
    }
    match Some(tail) {
        Some(["this", "power"])
        | Some(["thiss", "power"])
        | Some(["this", "creature", "power"])
        | Some(["thiss", "creature", "power"])
        | Some(["this", "creatures", "power"])
        | Some(["thiss", "creatures", "power"])
        | Some(["its", "power"]) => Some(Value::SourcePower),
        Some(["this", "toughness"])
        | Some(["thiss", "toughness"])
        | Some(["this", "creature", "toughness"])
        | Some(["thiss", "creature", "toughness"])
        | Some(["this", "creatures", "toughness"])
        | Some(["thiss", "creatures", "toughness"])
        | Some(["its", "toughness"]) => Some(Value::SourceToughness),
        Some(["this", "mana", "value"])
        | Some(["thiss", "mana", "value"])
        | Some(["this", "creature", "mana", "value"])
        | Some(["thiss", "creature", "mana", "value"])
        | Some(["this", "creatures", "mana", "value"])
        | Some(["thiss", "creatures", "mana", "value"])
        | Some(["its", "mana", "value"]) => Some(Value::ManaValueOf(Box::new(ChooseSpec::Source))),
        Some(["that", "creature", "power"])
        | Some(["that", "creatures", "power"])
        | Some(["that", "object", "power"])
        | Some(["that", "objects", "power"])
        | Some(["the", "sacrificed", "creature", "power"])
        | Some(["the", "sacrificed", "creatures", "power"])
        | Some(["sacrificed", "creature", "power"])
        | Some(["sacrificed", "creatures", "power"])
        | Some(["the", "amassed", "army", "power"])
        | Some(["the", "amassed", "armys", "power"])
        | Some(["amassed", "army", "power"])
        | Some(["amassed", "armys", "power"])
        | Some(["the", "army", "you", "amassed", "power"])
        | Some(["army", "you", "amassed", "power"]) => {
            Some(Value::PowerOf(Box::new(tagged_it.clone())))
        }
        Some(["that", "creature", "toughness"])
        | Some(["that", "creatures", "toughness"])
        | Some(["that", "object", "toughness"])
        | Some(["that", "objects", "toughness"])
        | Some(["the", "sacrificed", "creature", "toughness"])
        | Some(["the", "sacrificed", "creatures", "toughness"])
        | Some(["sacrificed", "creature", "toughness"])
        | Some(["sacrificed", "creatures", "toughness"])
        | Some(["the", "amassed", "army", "toughness"])
        | Some(["the", "amassed", "armys", "toughness"])
        | Some(["amassed", "army", "toughness"])
        | Some(["amassed", "armys", "toughness"])
        | Some(["the", "army", "you", "amassed", "toughness"])
        | Some(["army", "you", "amassed", "toughness"]) => {
            Some(Value::ToughnessOf(Box::new(tagged_it.clone())))
        }
        Some(["that", "spell", "mana", "value"])
        | Some(["that", "spell's", "mana", "value"])
        | Some(["that", "spells", "mana", "value"]) => Some(Value::ManaValueOf(Box::new(
            ChooseSpec::Tagged(TagKey::from("triggering")),
        ))),
        Some(["that", "card", "mana", "value"])
        | Some(["that", "card's", "mana", "value"])
        | Some(["that", "cards", "mana", "value"])
        | Some(["the", "sacrificed", "creature", "mana", "value"])
        | Some(["the", "sacrificed", "creatures", "mana", "value"])
        | Some(["sacrificed", "creature", "mana", "value"])
        | Some(["sacrificed", "creatures", "mana", "value"])
        | Some(["the", "amassed", "army", "mana", "value"])
        | Some(["the", "amassed", "armys", "mana", "value"])
        | Some(["amassed", "army", "mana", "value"])
        | Some(["amassed", "armys", "mana", "value"])
        | Some(["the", "mana", "value", "of", "the", "amassed", "army"])
        | Some(["the", "mana", "value", "of", "the", "amassed", "armys"])
        | Some(["mana", "value", "of", "the", "amassed", "army"])
        | Some(["mana", "value", "of", "the", "amassed", "armys"])
        | Some(
            [
                "the",
                "mana",
                "value",
                "of",
                "the",
                "army",
                "you",
                "amassed",
            ],
        )
        | Some(["mana", "value", "of", "the", "army", "you", "amassed"]) => {
            Some(Value::ManaValueOf(Box::new(tagged_it)))
        }
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_fixed_plus_reference_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let fixed_value = parse_number_word_i32(*clause_words.get(3)?)?;
    if fixed_value < 0 {
        return None;
    }
    let plus_word_idx = 4usize;
    if !clause_words
        .get(plus_word_idx)
        .is_some_and(|word| ETB_PLUS_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let value_words = clause_words.get(plus_word_idx + 1..)?;
    let reference_value = if ETB_SACRIFICED_CREATURE_POWER_PREFIX_PATTERN.matches_words(value_words) {
        Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
    } else if ETB_SACRIFICED_CREATURE_TOUGHNESS_PREFIX_PATTERN.matches_words(value_words) {
        Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
    } else if ETB_TAGGED_CREATURE_MANA_VALUE_PREFIX_PATTERN.matches_words(value_words) {
        Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
    } else {
        return None;
    };

    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value)),
        Box::new(reference_value),
    ))
}

pub(crate) fn parse_where_x_life_gained_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "amount",
                "of",
                "life",
                "you",
                "gained",
                "this",
                "turn",
            ],
        )
        | Some(["amount", "of", "life", "you", "gained", "this", "turn"]) => {
            Some(Value::LifeGainedThisTurn(PlayerFilter::You))
        }
        Some(
            [
                "the",
                "amount",
                "of",
                "life",
                "youve",
                "gained",
                "this",
                "turn",
            ],
        )
        | Some(["amount", "of", "life", "youve", "gained", "this", "turn"]) => {
            Some(Value::LifeGainedThisTurn(PlayerFilter::You))
        }
        _ => None,
    }
}

pub(crate) fn parse_where_x_life_lost_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "total",
                "life",
                "lost",
                "by",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        )
        | Some(
            [
                "total",
                "life",
                "lost",
                "by",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        ) => Some(Value::LifeLostThisTurn(PlayerFilter::Opponent)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_opponents_dealt_combat_damage_this_turn_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "number",
                "of",
                "opponents",
                "that",
                "were",
                "dealt",
                "combat",
                "damage",
                "this",
                "turn",
            ],
        )
        | Some(
            [
                "number",
                "of",
                "opponents",
                "that",
                "were",
                "dealt",
                "combat",
                "damage",
                "this",
                "turn",
            ],
        ) => Some(Value::CountPlayers(PlayerFilter::Opponent)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_noncombat_damage_to_opponents_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&words) {
        return None;
    }
    match words.get(3..) {
        Some(
            [
                "the",
                "total",
                "amount",
                "of",
                "noncombat",
                "damage",
                "dealt",
                "to",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        )
        | Some(
            [
                "total",
                "amount",
                "of",
                "noncombat",
                "damage",
                "dealt",
                "to",
                "your",
                "opponents",
                "this",
                "turn",
            ],
        ) => Some(Value::NoncombatDamageDealtToPlayersThisTurn(
            PlayerFilter::Opponent,
        )),
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let mut idx = 3usize;
    if ETB_THE_WORD_PATTERN.matches_word_at(&clause_words, idx) {
        idx += 1;
    }
    let aggregate = match clause_words.get(idx).copied() {
        Some("total") => "total",
        Some("greatest") => "greatest",
        _ => return None,
    };
    idx += 1;

    let value_kind = if ETB_POWER_WORD_PATTERN.matches_word_at(&clause_words, idx) {
        idx += 1;
        "power"
    } else if ETB_TOUGHNESS_WORD_PATTERN.matches_word_at(&clause_words, idx) {
        idx += 1;
        "toughness"
    } else if ETB_MANA_WORD_PATTERN.matches_word_at(&clause_words, idx)
        && ETB_VALUE_WORD_PATTERN.matches_word_at(&clause_words, idx + 1)
    {
        idx += 2;
        "mana_value"
    } else {
        return None;
    };

    if !ETB_OF_OR_AMONG_WORD_PATTERN.matches_word_at(&clause_words, idx) {
        return None;
    }
    idx += 1;

    if ETB_GREATEST_WORD_PATTERN.matches_words(&[aggregate])
        && ETB_MANA_VALUE_WORD_PATTERN.matches_words(&[value_kind])
    {
        if let Some(value) = parse_where_x_greatest_commander_mana_value(tokens, idx) {
            return Some(value);
        }
    }

    let object_start_token_idx = token_index_for_word_index(tokens, idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    let should_try_split = ETB_AND_GRAVEYARD_MARKER_PATTERN.matches_words(&filter_words)
        && filter_words
            .iter()
            .any(|word| ETB_CONTROL_OWN_WORD_PATTERN.matches_word(word));
    let mut filter = (if should_try_split {
        let segments =
            crate::runtime_backend::grammar::primitives::split_lexed_slices_on_and(filter_tokens);
        let mut branches = Vec::new();
        for segment in segments {
            let trimmed = trim_commas(segment);
            if trimmed.is_empty() {
                return None;
            }
            branches.push(parse_object_filter_lexed(&trimmed, false).ok()?);
        }
        if branches.len() < 2 {
            return None;
        }
        let mut combined = ObjectFilter::default();
        combined.any_of = branches;
        Some(combined)
    } else {
        None
    })
    .or_else(|| parse_object_filter_lexed(filter_tokens, false).ok())?;

    if ETB_SACRIFICED_MARKER_PATTERN.matches_words(&filter_words) {
        if matches!(filter.zone, Some(Zone::Battlefield)) {
            filter.zone = None;
        }
        if !filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && matches!(
                    constraint.relation,
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject
                )
        }) {
            filter
                .tagged_constraints
                .push(crate::filter::TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                });
        }
    }

    match (aggregate, value_kind) {
        ("total", "power") => Some(Value::TotalPower(filter)),
        ("total", "toughness") => Some(Value::TotalToughness(filter)),
        ("total", "mana_value") => Some(Value::TotalManaValue(filter)),
        ("greatest", "power") => Some(Value::GreatestPower(filter)),
        ("greatest", "toughness") => Some(Value::GreatestToughness(filter)),
        ("greatest", "mana_value") => Some(Value::GreatestManaValue(filter)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_greatest_commander_mana_value(
    tokens: &[OwnedLexToken],
    commander_start_word_idx: usize,
) -> Option<Value> {
    let commander_start_token_idx = token_index_for_word_index(tokens, commander_start_word_idx)?;
    let commander_words =
        crate::runtime_backend::token_word_refs(&tokens[commander_start_token_idx..]);
    let normalized = crate::runtime_backend::util::non_article_word_refs(&commander_words);
    if normalized
        != [
            "commander",
            "you",
            "own",
            "on",
            "battlefield",
            "or",
            "in",
            "command",
            "zone",
        ]
    {
        return None;
    }

    let mut battlefield_commander = ObjectFilter::default();
    battlefield_commander.zone = Some(Zone::Battlefield);
    battlefield_commander.is_commander = true;
    battlefield_commander.owner = Some(PlayerFilter::You);

    let mut command_zone_commander = battlefield_commander.clone();
    command_zone_commander.zone = Some(Zone::Command);

    let mut combined = ObjectFilter::default();
    combined.any_of = vec![battlefield_commander, command_zone_commander];

    Some(Value::GreatestManaValue(combined))
}

pub(crate) fn parse_where_x_is_number_of_differently_named_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let number_idx = ETB_NUMBER_WORD_PATTERN.find_word(&clause_words)?;
    if !clause_words
        .get(number_idx + 1)
        .is_some_and(|word| ETB_OF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    if !clause_words
        .get(number_idx + 2)
        .is_some_and(|word| ETB_DIFFERENTLY_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    if !clause_words
        .get(number_idx + 3)
        .is_some_and(|word| ETB_NAMED_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let object_start_word_idx = number_idx + 4;
    let object_start_token_idx = token_index_for_word_index(tokens, object_start_word_idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::DistinctNames(filter))
}

pub(crate) fn parse_where_x_is_number_of_different_powers_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let number_idx = ETB_NUMBER_WORD_PATTERN.find_word(&clause_words)?;
    if !clause_words
        .get(number_idx + 1)
        .is_some_and(|word| ETB_OF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    if !clause_words
        .get(number_idx + 2)
        .is_some_and(|word| ETB_DIFFERENT_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    if !clause_words
        .get(number_idx + 3)
        .is_some_and(|word| ETB_POWER_OR_POWERS_WORD_PATTERN.matches_word(word))
    {
        return None;
    }
    if !clause_words
        .get(number_idx + 4)
        .is_some_and(|word| ETB_AMONG_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let object_start_word_idx = number_idx + 5;
    let object_start_token_idx = token_index_for_word_index(tokens, object_start_word_idx)?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::DistinctPowers(filter))
}

pub(crate) fn parse_where_x_is_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    if ETB_COMMON_CREATURE_TYPE_VALUE_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let number_idx = ETB_NUMBER_WORD_PATTERN.find_word(&clause_words)?;
    let multiplier = match clause_words.get(3..number_idx) {
        Some([]) | Some(["the"]) | Some(["the", "total"]) => 1,
        Some(["twice"])
        | Some(["twice", "the"])
        | Some(["two", "times"])
        | Some(["two", "times", "the"]) => 2,
        _ => return None,
    };
    if !clause_words
        .get(number_idx + 1)
        .is_some_and(|word| ETB_OF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let object_start_word_idx = number_idx + 2;
    let mut seen_words = 0usize;
    let mut object_start_token_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_words == object_start_word_idx {
            object_start_token_idx = Some(idx);
            break;
        }
        seen_words += 1;
    }
    let object_start_token_idx = object_start_token_idx?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    if let Some(value) = parse_number_of_counters_on_source_value(&filter_words) {
        return Some(value);
    }
    if ETB_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let mut scope_tokens = &filter_tokens[4..];
        if scope_tokens
            .first()
            .is_some_and(|token| ETB_THE_WORD_PATTERN.matches_token(token))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter_lexed(scope_tokens, false).ok()?;
        return Some(scale_where_x_number_value(
            Value::BasicLandTypesAmong(scope_filter),
            multiplier,
        ));
    }
    if ETB_CREATURE_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let mut scope_tokens = &filter_tokens[3..];
        if scope_tokens
            .first()
            .is_some_and(|token| ETB_THE_WORD_PATTERN.matches_token(token))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter_lexed(scope_tokens, false).ok()?;
        return Some(scale_where_x_number_value(
            Value::CreatureTypesAmong(scope_filter),
            multiplier,
        ));
    }
    if ETB_COLORS_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let mut scope_tokens = &filter_tokens[2..];
        if scope_tokens
            .first()
            .is_some_and(|token| ETB_THE_WORD_PATTERN.matches_token(token))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter_lexed(scope_tokens, false).ok()?;
        return Some(scale_where_x_number_value(
            Value::ColorsAmong(scope_filter),
            multiplier,
        ));
    }
    if ETB_CARD_TYPES_AMONG_CARDS_PREFIX_PATTERN.matches_words(&filter_words)
        && ETB_GRAVEYARD_MARKER_PATTERN.matches_words(&filter_words)
    {
        let player = if ETB_YOUR_GRAVEYARD_PATTERN.matches_words(&filter_words) {
            PlayerFilter::You
        } else if ETB_OPPONENT_GRAVEYARD_PATTERN.matches_words(&filter_words) {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Some(scale_where_x_number_value(
            Value::CardTypesInGraveyard(player),
            multiplier,
        ));
    }
    if ETB_CARD_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let mut scope_tokens = &filter_tokens[3..];
        if scope_tokens
            .first()
            .is_some_and(|token| ETB_THE_WORD_PATTERN.matches_token(token))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter_lexed(scope_tokens, false).ok()?;
        return Some(scale_where_x_number_value(
            Value::CardTypesAmong(scope_filter),
            multiplier,
        ));
    }
    if matches!(
        filter_words.as_slice(),
        ["creature", "that", "died", "this", "turn"]
            | ["creatures", "that", "died", "this", "turn"]
    ) {
        return Some(scale_where_x_number_value(
            Value::CreaturesDiedThisTurn,
            multiplier,
        ));
    }
    if matches!(
        filter_words.as_slice(),
        [
            "times", "its", "been", "cast", "from", "the", "command", "zone", "this", "game"
        ] | [
            "times", "it", "has", "been", "cast", "from", "the", "command", "zone", "this", "game"
        ] | [
            "times",
            "this",
            "commander",
            "has",
            "been",
            "cast",
            "from",
            "the",
            "command",
            "zone",
            "this",
            "game"
        ] | [
            "times",
            "your",
            "commander",
            "has",
            "been",
            "cast",
            "from",
            "the",
            "command",
            "zone",
            "this",
            "game"
        ]
    ) {
        return Some(scale_where_x_number_value(
            Value::CommanderCastCount(PlayerFilter::You),
            multiplier,
        ));
    }
    if matches!(
        filter_words.as_slice(),
        ["creature", "those", "players", "control"] | ["creatures", "those", "players", "control"]
    ) {
        let mut filter = ObjectFilter::creature();
        filter.controller = Some(PlayerFilter::target_player());
        return Some(scale_where_x_number_value(Value::Count(filter), multiplier));
    }
    let filter = parse_object_filter_lexed(filter_tokens, false).ok()?;
    Some(scale_where_x_number_value(Value::Count(filter), multiplier))
}

fn scale_where_x_number_value(value: Value, multiplier: i32) -> Value {
    if multiplier == 1 {
        return value;
    }
    match value {
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => Value::Scaled(Box::new(other), multiplier),
    }
}

fn parse_number_of_counters_on_source_value(filter_words: &[&str]) -> Option<Value> {
    let mut idx = 0usize;
    if filter_words
        .get(idx)
        .is_some_and(|word| is_article(word) || ETB_ONE_WORD_PATTERN.matches_word(word))
    {
        idx += 1;
    }
    let counter_word = *filter_words.get(idx)?;
    let counter_type = parse_counter_type_word(counter_word).or_else(|| {
        counter_word
            .chars()
            .all(|ch| ch.is_ascii_alphabetic())
            .then_some(CounterType::Named(intern_counter_name(counter_word)))
    })?;
    idx += 1;
    if !ETB_COUNTER_OR_COUNTERS_WORD_PATTERN.matches_word_at(&filter_words, idx) {
        return None;
    }
    idx += 1;
    if filter_words.get(idx).copied() != Some("on") {
        return None;
    }
    idx += 1;
    match filter_words.get(idx..) {
        Some(["it"])
        | Some(["this"])
        | Some(["this", "card"])
        | Some(["this", "creature"])
        | Some(["this", "permanent"])
        | Some(["this", "source"])
        | Some(["this", "artifact"])
        | Some(["this", "land"])
        | Some(["this", "enchantment"])
        | Some(["thiss"])
        | Some(["thiss", "card"])
        | Some(["thiss", "creature"])
        | Some(["thiss", "permanent"])
        | Some(["thiss", "source"])
        | Some(["thiss", "artifact"])
        | Some(["this", "equipment"])
        | Some(["thiss", "land"])
        | Some(["thiss", "enchantment"])
        | Some(["thiss", "equipment"]) => Some(Value::CountersOnSource(counter_type)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_fixed_plus_number_of_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let value_start_idx = token_index_for_word_index(tokens, 3)?;
    let (fixed_value, fixed_used) = parse_number(&tokens[value_start_idx..])?;
    let plus_word_idx = 3 + fixed_used;
    if !clause_words
        .get(plus_word_idx)
        .is_some_and(|word| ETB_PLUS_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let mut number_word_idx = plus_word_idx + 1;
    if ETB_THE_WORD_PATTERN.matches_word_at(&clause_words, number_word_idx) {
        number_word_idx += 1;
    }
    if !clause_words
        .get(number_word_idx)
        .is_some_and(|word| ETB_NUMBER_WORD_PATTERN.matches_word(word))
        || !clause_words
            .get(number_word_idx + 1)
            .is_some_and(|word| ETB_OF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let filter_start_idx = token_index_for_word_index(tokens, number_word_idx + 2)?;
    let filter_tokens = &tokens[filter_start_idx..];
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    if let Some(counter_value) = parse_number_of_counters_on_source_value(&filter_words) {
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(counter_value),
        ));
    }
    if ETB_BASIC_LAND_TYPES_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let mut scope_tokens = &filter_tokens[4..];
        if scope_tokens
            .first()
            .is_some_and(|token| ETB_THE_WORD_PATTERN.matches_token(token))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(Value::BasicLandTypesAmong(scope_filter)),
        ));
    }
    if ETB_COLORS_AMONG_PREFIX_PATTERN.matches_words(&filter_words) {
        let mut scope_tokens = &filter_tokens[2..];
        if scope_tokens
            .first()
            .is_some_and(|token| ETB_THE_WORD_PATTERN.matches_token(token))
        {
            scope_tokens = &scope_tokens[1..];
        }
        let scope_filter = parse_object_filter(scope_tokens, false).ok()?;
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(Value::ColorsAmong(scope_filter)),
        ));
    }
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value as i32)),
        Box::new(Value::Count(filter)),
    ))
}

pub(crate) fn parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches_words(&clause_words) {
        return None;
    }

    let mut number_word_idx = 3usize;
    if ETB_THE_WORD_PATTERN.matches_word_at(&clause_words, number_word_idx) {
        number_word_idx += 1;
    }
    if !clause_words
        .get(number_word_idx)
        .is_some_and(|word| ETB_NUMBER_WORD_PATTERN.matches_word(word))
        || !clause_words
            .get(number_word_idx + 1)
            .is_some_and(|word| ETB_OF_WORD_PATTERN.matches_word(word))
    {
        return None;
    }

    let filter_start_word_idx = number_word_idx + 2;
    let operator_word_idx = ETB_PLUS_OR_MINUS_WORD_PATTERN
        .find_word(&clause_words[filter_start_word_idx + 1..])
        .map(|idx| filter_start_word_idx + 1 + idx)?;
    let operator = clause_words[operator_word_idx];

    let filter_start_token_idx = token_index_for_word_index(tokens, filter_start_word_idx)?;
    let operator_token_idx = token_index_for_word_index(tokens, operator_word_idx)?;
    let filter_tokens = trim_commas(&tokens[filter_start_token_idx..operator_token_idx]);
    let filter_words = crate::runtime_backend::token_word_refs(&filter_tokens);
    let count_value = if ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches_words(&filter_words)
    {
        Value::CardsInHand(PlayerFilter::You)
    } else {
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        Value::Count(filter)
    };

    let offset_start_token_idx = token_index_for_word_index(tokens, operator_word_idx + 1)?;
    let offset_tokens = trim_commas(&tokens[offset_start_token_idx..]);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = crate::runtime_backend::token_word_refs(&offset_tokens[used..]);
    if !trailing_words.is_empty() {
        return None;
    }

    let signed_offset = if ETB_MINUS_WORD_PATTERN.matches_words(&[operator]) {
        -(offset_value as i32)
    } else {
        offset_value as i32
    };
    Some(Value::Add(
        Box::new(count_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

pub(crate) fn token_index_for_word_index(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<usize> {
    crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens)
        .token_index_for_word_index(word_index)
}

pub(crate) fn parse_enters_tapped_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words
        .first()
        .is_some_and(|word| ETB_TRIGGER_INTRO_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        if ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches_words(&clause_words)
            && ETB_TAPPED_MARKER_PATTERN.matches_words(&clause_words)
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if ETB_UNLESS_MARKER_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }
    let enter_word_idx = ETB_ENTER_OR_ENTERS_WORD_PATTERN.find_word(&clause_words);
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if !ETB_TAPPED_MARKER_PATTERN.matches_words(&clause_words[enter_word_idx + 1..]) {
        return Ok(None);
    }
    if ETB_THIS_WORD_PATTERN.matches_word_at(&clause_words, 0) {
        return Ok(None);
    }
    if ETB_COPY_MARKER_PATTERN.matches_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}') [rule=enters-as-copy]",
            clause_words.join(" ")
        )));
    }
    let before_enter = &tokens[..enter_token_idx];
    let before_word_view =
        crate::runtime_backend::grammar::primitives::TokenWordView::new(before_enter);
    let before_words = before_word_view.word_refs();
    let mut controller_override: Option<PlayerFilter> = None;
    let mut filter_end = before_enter.len();
    let find_suffix_cut = |suffix_len: usize| {
        let keep_word_count = before_words.len().saturating_sub(suffix_len);
        if keep_word_count == 0 {
            0
        } else {
            before_word_view
                .token_start_indices()
                .get(keep_word_count)
                .copied()
                .unwrap_or(before_enter.len())
        }
    };
    if ETB_PLAYED_BY_YOUR_OPPONENTS_SUFFIX_PATTERN.matches_words(&before_words) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if ETB_PLAYED_BY_AN_OPPONENT_SUFFIX_PATTERN.matches_words(&before_words) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if ETB_PLAYED_BY_OPPONENTS_SUFFIX_PATTERN.matches_words(&before_words) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(3);
    }
    let mut filter = match parse_object_filter(&before_enter[..filter_end], false) {
        Ok(filter) => filter,
        Err(_) if filter_end == before_enter.len() && !before_words.is_empty() => {
            return Ok(Some(StaticAbility::enters_tapped_ability()));
        }
        Err(err) => return Err(err),
    };
    if controller_override.is_none() && filter.source {
        return Ok(Some(StaticAbility::enters_tapped_ability()));
    }
    if let Some(controller) = controller_override {
        filter.controller = Some(controller);
    }
    Ok(Some(StaticAbility::enters_tapped_for_filter(filter)))
}

pub(crate) fn parse_enters_untapped_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words
        .first()
        .is_some_and(|word| ETB_TRIGGER_INTRO_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if ETB_UNLESS_MARKER_PATTERN.matches_words(&clause_words)
        || ETB_THIS_WORD_PATTERN.matches_word_at(&clause_words, 0)
    {
        return Ok(None);
    }

    let Some(enter_word_idx) = ETB_ENTER_OR_ENTERS_WORD_PATTERN.find_word(&clause_words)
    else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if !ETB_UNTAPPED_MARKER_PATTERN.matches_words(&clause_words[enter_word_idx + 1..]) {
        return Ok(None);
    }

    let before_enter = &tokens[..enter_token_idx];
    if before_enter.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(before_enter, false)?;
    Ok(Some(StaticAbility::enters_untapped_for_filter(filter)))
}

pub(crate) fn parse_reveal_from_hand_or_enters_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_AS_THIS_LAND_ENTERS_PREFIX_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }
    if !ETB_REVEAL_FROM_HAND_MARKER_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let Some(reveal_word_idx) = ETB_REVEAL_WORD_PATTERN.find_word(&clause_words) else {
        return Err(CardTextError::ParseError(format!(
            "missing 'reveal' keyword in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let Some(from_hand_word_idx) =
        etb_find_prefix_shape_start(&clause_words[reveal_word_idx + 1..], &ETB_FROM_YOUR_HAND_PREFIX_PATTERN)
    .map(|idx| reveal_word_idx + 1 + idx) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal source in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let Some(reveal_filter_clause) =
        LexedClause::new(tokens).between_word_range(reveal_word_idx + 1, from_hand_word_idx)
    else {
        return Err(CardTextError::ParseError(format!(
            "missing reveal filter start in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let reveal_filter_tokens = trim_edge_punctuation(reveal_filter_clause.tokens());
    if reveal_filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing reveal filter in land ETB reveal clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let reveal_filter = parse_object_filter(&reveal_filter_tokens, false)?;
    let reveal_condition = crate::ConditionExpr::YouHaveCardInHandMatching(reveal_filter);

    // Pattern A: "... If you don't, this land enters tapped."
    if let Some(if_you_dont_idx) =
        etb_find_prefix_shape_start(&clause_words, &ETB_IF_YOU_DONT_PREFIX_PATTERN)
    {
        let trailing = &clause_words[if_you_dont_idx + 3..];
        if !ETB_LAND_REVEAL_TRAILING_TAPPED_PATTERN.matches_words(trailing) {
            return Err(CardTextError::ParseError(format!(
                "unsupported land ETB reveal trailing clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parser_trace("parse_static:land-reveal-or-enter-tapped:matched", tokens);
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            reveal_condition,
            clause_words.join(" "),
        )));
    }

    // Pattern B: "... This land enters tapped unless you revealed ... this way or you control ..."
    let Some(unless_idx) = ETB_UNLESS_TAIL_PATTERN.find_word(&clause_words) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported land ETB reveal clause (expected 'if you don't' or 'unless') (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let before_unless = &clause_words[..unless_idx];
    if !ETB_ENTERS_TAPPED_PHRASE_PATTERN.matches_words(before_unless) {
        return Err(CardTextError::ParseError(format!(
            "unsupported land ETB reveal unless-prefix (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut condition = reveal_condition;
    if let Some(or_idx_rel) =
        ETB_OR_WORD_PATTERN.find_word(&clause_words[unless_idx + 1..])
    {
        let or_idx = unless_idx + 1 + or_idx_rel;
        let Some(control_word_idx) =
            ETB_CONTROL_OR_CONTROLS_WORD_PATTERN.find_word(&clause_words[or_idx + 1..])
            .map(|idx| or_idx + 1 + idx)
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported land ETB reveal disjunction (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let Some(control_filter_start_token_idx) =
            token_index_for_word_index(tokens, control_word_idx + 1)
        else {
            return Err(CardTextError::ParseError(format!(
                "missing control filter in land ETB reveal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let control_filter_tokens =
            trim_edge_punctuation(&tokens[control_filter_start_token_idx..]);
        if control_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing control filter in land ETB reveal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let control_filter = parse_object_filter(&control_filter_tokens, false)?;
        condition = crate::ConditionExpr::Or(
            Box::new(condition),
            Box::new(crate::ConditionExpr::YouControl(control_filter)),
        );
    }

    parser_trace("parse_static:land-reveal-or-enter-tapped:matched", tokens);
    Ok(Some(StaticAbility::enters_tapped_unless_condition(
        condition,
        clause_words.join(" "),
    )))
}

fn parse_enters_tapped_unless_control_quantity_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    let condition_words = crate::runtime_backend::lexer::token_word_refs(condition_tokens);
    if !ETB_YOU_CONTROL_PREFIX_PATTERN.matches_words(&condition_words) {
        return None;
    }
    let control_idx = crate::runtime_backend::grammar::primitives::find_token_index(
        condition_tokens,
        |token| ETB_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token),
    )?;
    let quantified_tokens = condition_tokens.get(control_idx + 1..)?;
    let quantified_tokens = trim_edge_punctuation(quantified_tokens);
    let StaticCountedObjectCondition {
        comparison,
        mut filter,
    } = parse_counted_object_condition_after_prefix(
        condition_tokens.get(..=control_idx)?,
        &quantified_tokens,
        false,
        "enters-tapped control condition",
        &condition_words,
    )
    .ok()?;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    Some(crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(filter),
        comparison,
        display: Some(condition_words.join(" ")),
    })
}

fn parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<()> {
    let condition_words = crate::runtime_backend::lexer::token_word_refs(condition_tokens);
    if !ETB_A_PLAYER_HAS_PREFIX_PATTERN.matches_words(&condition_words) {
        return None;
    }
    let (comparison, used) = parse_quantity_comparison_prefix(
        condition_tokens.get(3..)?,
        false,
        false,
        "enters-tapped life condition",
    )
    .ok()?;
    if !ETB_LIFE_TAIL_PATTERN.matches_words(condition_words.get(3 + used..).unwrap_or_default()) {
        return None;
    }
    match comparison {
        crate::effect::Comparison::LessThanOrEqual(13) | crate::effect::Comparison::LessThan(14) => {
            Some(())
        }
        _ => None,
    }
}

fn parse_enters_tapped_unless_two_or_more_opponents_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<()> {
    let condition_words = crate::runtime_backend::lexer::token_word_refs(condition_tokens);
    if !ETB_YOU_HAVE_PREFIX_PATTERN.matches_words(&condition_words) {
        return None;
    }
    let (count, rest_start) = parse_etb_at_least_quantity_at(condition_tokens, 2)?;
    if count == 2 && ETB_OPPONENTS_TAIL_PATTERN.matches_words(&condition_words[rest_start..]) {
        Some(())
    } else {
        None
    }
}

pub(crate) fn parse_conditional_enters_tapped_unless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }
    if !ETB_TAPPED_MARKER_PATTERN.matches_words(&clause_words)
        || !ETB_UNLESS_MARKER_PATTERN.matches_words(&clause_words)
    {
        return Ok(None);
    }

    let Some(unless_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(tokens, |token| {
            ETB_UNLESS_TAIL_PATTERN.matches_token(token)
        })
    else {
        return Ok(None);
    };
    let condition_tokens = trim_edge_punctuation(&tokens[unless_idx + 1..]);
    let condition_words = crate::runtime_backend::token_word_refs(&condition_tokens);
    if let Some(condition) =
        parse_enters_tapped_unless_control_quantity_condition(&condition_tokens)
    {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            condition,
            clause_words.join(" "),
        )));
    }
    if parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(&condition_tokens)
        .is_some()
    {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_a_player_has_13_or_less_life(),
        ));
    }
    if parse_enters_tapped_unless_two_or_more_opponents_condition(&condition_tokens).is_some() {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_two_or_more_opponents(),
        ));
    }
    if ETB_FIRST_THREE_TURNS_PATTERN.matches_words(&condition_words) {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            crate::ConditionExpr::YourFirstTurnsOfTheGameOrFewer(3),
            clause_words.join(" "),
        )));
    }

    // Generic: "unless you control <object filter>" (covers Mount/Vehicle, etc.).
    if ETB_YOU_CONTROL_PREFIX_PATTERN.matches_words(&condition_words) {
        let control_idx = crate::runtime_backend::grammar::primitives::find_token_index(
            &condition_tokens,
            |token| ETB_CONTROL_OR_CONTROLS_WORD_PATTERN.matches_token(token),
        )
        .unwrap_or_default();
        let filter_tokens = trim_edge_punctuation(&condition_tokens[control_idx + 1..]);
        if !filter_tokens.is_empty() {
            if let Ok(filter) = parse_object_filter(&filter_tokens, false) {
                let condition = crate::ConditionExpr::YouControl(filter);
                return Ok(Some(StaticAbility::enters_tapped_unless_condition(
                    condition,
                    clause_words.join(" "),
                )));
            }
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported enters tapped unless condition (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_enters_with_additional_counter_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words.len() > 9
        && ETB_AS_LONG_AS_THIS_IN_YOUR_GRAVEYARD_PATTERN.matches_words(&clause_words)
        && let Some(comma_idx) = find_token_kind(tokens, TokenKind::Comma)
    {
        return parse_enters_with_additional_counter_for_filter_line(&tokens[comma_idx + 1..]);
    }

    if clause_words.len() > 6
        && word_slice_starts_with(&clause_words, &["as", "long", "as"])
        && let Some(comma_idx) = find_token_kind(tokens, TokenKind::Comma)
    {
        let condition_tokens = trim_edge_punctuation(&tokens[3..comma_idx]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        let Some(ability) =
            parse_enters_with_additional_counter_for_filter_line(&tokens[comma_idx + 1..])?
        else {
            return Ok(None);
        };
        return Ok(Some(ability.with_condition(condition)));
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let enter_word_idx = ETB_ENTER_OR_ENTERS_WORD_PATTERN.find_word(&clause_words);
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if tokens[..enter_token_idx]
        .iter()
        .any(|token| token.is_period() || token.is_colon() || token.is_semicolon())
    {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..enter_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = crate::runtime_backend::token_word_refs(&subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if matches!(
        subject_words.first().copied(),
        Some("if" | "when" | "whenever" | "as" | "at")
    ) {
        return Ok(None);
    }

    if !ETB_WITH_ADDITIONAL_COUNTERS_PATTERN.matches_words(&clause_words) {
        return Ok(None);
    }

    let Ok(filter) = parse_object_filter(&subject_tokens, false) else {
        return Ok(None);
    };

    let and_as_idx =
        crate::runtime_backend::lexer::find_token_word_sequence_span(tokens, &["and", "as"])
            .map(|(idx, _)| idx);
    let base_tokens = and_as_idx.map_or(tokens, |idx| &tokens[..idx]);

    let additional_idx =
        crate::runtime_backend::grammar::primitives::find_token_index(base_tokens, |token| {
            ETB_ADDITIONAL_WORD_PATTERN.matches_token(token)
        })
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let count = if let Some(equal_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(base_tokens, |token| {
            ETB_EQUAL_WORD_PATTERN.matches_token(token)
        }) {
        let value_start = equal_idx + 2;
        let value_tokens = trim_commas(base_tokens.get(value_start..).unwrap_or_default());
        parse_value(&value_tokens)
            .map(|(value, _)| value)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported ETB counter count value (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
    } else if additional_idx > 0
        && let Some((parsed, _)) = parse_number(&base_tokens[additional_idx - 1..additional_idx])
    {
        Value::Fixed(parsed as i32)
    } else if let Some((parsed, _)) = parse_number(&base_tokens[additional_idx + 1..]) {
        Value::Fixed(parsed as i32)
    } else {
        Value::Fixed(1)
    };

    let counter_type = parse_counter_type_from_tokens(base_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for ETB replacement (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let mut added_subtypes = Vec::new();
    if let Some(idx) = and_as_idx {
        let mut addition_tokens = tokens[idx + 1..].to_vec();
        if let Some(first) = addition_tokens.first() {
            addition_tokens[0] = OwnedLexToken::word("is".to_string(), first.span());
        }
        let Some(additions) = parse_type_color_addition_clause(&addition_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported ETB type-addition tail (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        if !additions.added_colors.is_empty()
            || !additions.set_colors.is_empty()
            || !additions.card_types.is_empty()
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-subtype ETB type addition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        added_subtypes = additions.subtypes;
    }

    Ok(Some(
        StaticAbility::enters_with_counters_and_subtypes_for_filter(
            filter,
            counter_type,
            count,
            added_subtypes,
        ),
    ))
}

pub(crate) fn parse_as_enters_becomes_characteristics_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_AS_WORD_PATTERN.matches_word_at(&clause_words, 0) {
        return Ok(None);
    }

    let Some(enter_word_idx) = ETB_ENTER_OR_ENTERS_WORD_PATTERN.find_word(&clause_words)
    else {
        return Ok(None);
    };
    if enter_word_idx <= 1 {
        return Ok(None);
    }

    let after_enter = clause_words.get(enter_word_idx + 1..).unwrap_or_default();
    if !ETB_IT_BECOMES_PREFIX_PATTERN.matches_words(after_enter) {
        return Ok(None);
    }

    let mut descriptor_idx = 2usize;
    if after_enter
        .get(descriptor_idx)
        .is_some_and(|word| ETB_ARTICLE_WORD_PATTERN.matches_word(word))
    {
        descriptor_idx += 1;
    }
    let Some(pt_word) = after_enter.get(descriptor_idx) else {
        return Ok(None);
    };
    let (power, toughness) = match parse_pt_modifier(pt_word) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    descriptor_idx += 1;

    if !ETB_IN_ADDITION_TO_ITS_OTHER_TYPE_PATTERN.matches_words(after_enter) {
        return Ok(None);
    }
    let Some(addition_idx) =
        etb_find_prefix_shape_start(after_enter, &ETB_IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN)
    else {
        return Ok(None);
    };
    if addition_idx <= descriptor_idx {
        return Ok(None);
    }

    let subject_start = token_index_for_word_index(tokens, 1)
        .ok_or_else(|| CardTextError::ParseError("missing as-enters subject".to_string()))?;
    let enter_token_idx = token_index_for_word_index(tokens, enter_word_idx)
        .ok_or_else(|| CardTextError::ParseError("missing as-enters enter token".to_string()))?;
    let subject_tokens = trim_commas(&tokens[subject_start..enter_token_idx]);
    let filter = parse_object_filter(&subject_tokens, false)?;

    let descriptor_words = &after_enter[descriptor_idx..addition_idx];
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words
        .iter()
        .copied()
        .filter(|word| {
            !ETB_ARTICLE_WORD_PATTERN.matches_word(word)
                && !ETB_AND_WORD_PATTERN.matches_word(word)
        })
    {
        if parse_color(descriptor).is_some() {
            return Err(CardTextError::ParseError(format!(
                "unsupported color-changing as-enters characteristic replacement (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported as-enters characteristic descriptor '{}' (clause: '{}')",
            descriptor,
            clause_words.join(" ")
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::enters_with_characteristics_for_filter(
        filter, card_types, subtypes, power, toughness,
    )))
}

pub(crate) fn parse_as_enters_or_turns_face_up_pt_choice_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_AS_WORD_PATTERN.matches_word_at(&clause_words, 0) {
        return Ok(None);
    }

    let Some(enter_word_idx) = ETB_ENTER_OR_ENTERS_WORD_PATTERN.find_word(&clause_words)
    else {
        return Ok(None);
    };
    if enter_word_idx <= 1 {
        return Ok(None);
    }

    let subject_words = &clause_words[1..enter_word_idx];
    if !ETB_SELF_SUBJECT_PATTERN.matches_words(subject_words) {
        return Ok(None);
    }

    let after_enter = clause_words.get(enter_word_idx + 1..).unwrap_or_default();
    if word_slice_starts_with(after_enter, &["it", "becomes", "your", "choice", "of"]) {
        let options = parse_pt_choice_characteristic_options(&after_enter[5..], &clause_words)?;
        if options.is_empty() {
            return Ok(None);
        }
        let subject = subject_words.join(" ");
        let display = format!(
            "As {subject} enters, it becomes your choice of {}",
            render_pt_choice_characteristic_options(&options)
        );
        return Ok(Some(
            StaticAbility::choose_power_toughness_options_as_enters_or_turns_face_up(
                options, display,
            ),
        ));
    }

    if after_enter.len() != 13
        || !ETB_FACE_UP_CHOICE_TAIL_PATTERN.matches_words(after_enter)
        || after_enter.get(11).copied() != Some("or")
    {
        return Ok(None);
    }

    let first = parse_pt_modifier(after_enter[10]).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported power/toughness choice '{}' (clause: '{}')",
            after_enter[10],
            clause_words.join(" ")
        ))
    })?;
    let second = parse_pt_modifier(after_enter[12]).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported power/toughness choice '{}' (clause: '{}')",
            after_enter[12],
            clause_words.join(" ")
        ))
    })?;

    let subject = subject_words.join(" ");
    let display = format!(
        "As {subject} enters or is turned face up, it becomes your choice of {}/{} or {}/{}",
        first.0, first.1, second.0, second.1
    );
    Ok(Some(
        StaticAbility::choose_power_toughness_as_enters_or_turns_face_up(
            vec![first, second],
            display,
        ),
    ))
}

fn parse_pt_choice_characteristic_options(
    words: &[&str],
    clause_words: &[&str],
) -> Result<Vec<PowerToughnessChoiceOption>, CardTextError> {
    let mut options = Vec::new();
    let mut idx = 0usize;
    while idx < words.len() {
        if words[idx] == "or" {
            idx += 1;
        }
        if matches!(words.get(idx).copied(), Some("a" | "an")) {
            idx += 1;
        }
        let Some(pt_word) = words.get(idx).copied() else {
            break;
        };
        let (power, toughness) = match parse_pt_modifier(pt_word) {
            Ok(pt) => pt,
            Err(_) if options.is_empty() => return Ok(Vec::new()),
            Err(_) => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported power/toughness choice '{}' (clause: '{}')",
                    pt_word,
                    clause_words.join(" ")
                )));
            }
        };
        idx += 1;

        if !matches!(
            words.get(idx).copied(),
            Some("creature" | "permanent" | "object")
        ) {
            return Err(CardTextError::ParseError(format!(
                "unsupported power/toughness choice descriptor after '{}' (clause: '{}')",
                pt_word,
                clause_words.join(" ")
            )));
        }
        idx += 1;

        let mut abilities = Vec::new();
        if words.get(idx).copied() == Some("with") {
            idx += 1;
            let ability_start = idx;
            while idx < words.len()
                && words[idx] != "or"
                && !(matches!(words[idx], "a" | "an")
                    && words
                        .get(idx + 1)
                        .is_some_and(|next| parse_pt_modifier(next).is_ok()))
            {
                idx += 1;
            }
            abilities =
                parse_pt_choice_keyword_abilities(&words[ability_start..idx], clause_words)?;
        }

        options.push(PowerToughnessChoiceOption::with_abilities(
            power, toughness, abilities,
        ));
    }

    Ok(options)
}

fn parse_pt_choice_keyword_abilities(
    words: &[&str],
    clause_words: &[&str],
) -> Result<Vec<StaticAbility>, CardTextError> {
    if words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing keyword ability in power/toughness choice (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let action = match words {
        [word] => parse_single_word_keyword_action(word),
        ["first", "strike"] => Some(KeywordAction::FirstStrike),
        ["double", "strike"] => Some(KeywordAction::DoubleStrike),
        _ => None,
    };
    let Some(static_ability) = action.and_then(static_ability_for_keyword_action) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported keyword ability '{}' in power/toughness choice (clause: '{}')",
            words.join(" "),
            clause_words.join(" ")
        )));
    };

    Ok(vec![static_ability])
}

fn render_pt_choice_characteristic_options(options: &[PowerToughnessChoiceOption]) -> String {
    let rendered = options
        .iter()
        .map(|option| {
            let mut text = format!("a {}/{} creature", option.power, option.toughness);
            if !option.abilities.is_empty() {
                let abilities = option
                    .abilities
                    .iter()
                    .map(|ability| ability.display().to_ascii_lowercase())
                    .collect::<Vec<_>>()
                    .join(" and ");
                text.push_str(" with ");
                text.push_str(&abilities);
            }
            text
        })
        .collect::<Vec<_>>();

    match rendered.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} or {second}"),
        _ => {
            let mut text = rendered[..rendered.len() - 1].join(", ");
            text.push_str(", or ");
            text.push_str(rendered.last().expect("nonempty options"));
            text
        }
    }
}

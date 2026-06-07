use ironsmith_core::ValueSurfaceHint;

const ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["if"], &["when"], &["whenever"], &["as"], &["at"]]);
const ETB_TRIGGER_INTRO_WORDS: &[&str] = &["if", "when", "whenever", "as"];
const ETB_THIS_WORD: &str = "this";

const SOURCE_PRONOUN_SUBJECT_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["it"], &["its"]]);
const ETB_IF_WORD: &str = "if";
const ETB_ENTER_OR_ENTERS_PHRASES: &[&[&str]] = &[&["enter"], &["enters"]];
const ETB_ENTER_OR_ENTERS_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["enter", "enters"]]);
const ETB_ENTERS_OR_ESCAPES_PHRASES: &[&[&str]] = &[&["enters"], &["escapes"]];
const ETB_ARTICLE_WORDS: &[&str] = &["a", "an"];
const ETB_ONE_WORD: &str = "one";
const ETB_THE_WORD: &str = "the";
const ETB_POWER_WORD: &str = "power";
const ETB_TOUGHNESS_WORD: &str = "toughness";
const ETB_ADDITIONAL_WORD: &str = "additional";
const ETB_COUNTER_OR_COUNTERS_WORDS: &[&str] = &["counter", "counters"];
const ETB_SOURCE_TAIL_HEAD_WORDS: &[&str] = &["this", "thiss"];
const ETB_SOURCE_TAIL_NOUN_WORDS: &[&str] = &["source", "spell", "card", "creature", "permanent"];
const ETB_OR_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["or"]);
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
    contains_any_phrases
        & [&[
            &["enters", "tapped"],
            &["enter", "tapped"],
            &["enters", "the", "battlefield", "tapped"],
            &["enter", "the", "battlefield", "tapped"],
        ]]
);
const ETB_CONTROL_OWN_WORDS: &[&str] = &["control", "controls", "own", "owns"];
const ETB_FIRST_THREE_TURNS_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "it", "s", "your", "first", "second", "or", "third", "turn", "of", "the", "game",
            ],
            &[
                "its", "your", "first", "second", "or", "third", "turn", "of", "the", "game",
            ],
            &[
                "it's", "your", "first", "second", "or", "third", "turn", "of", "the", "game",
            ],
        ]
);
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
const ETB_SPELLS_THIS_TURN_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(suffix & ["this", "turn"]; contains_any_words & [&["spell", "spells"]]);
const ETB_COLORS_MANA_SPENT_CONDITION_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [&["of", "mana"], &["spent", "to", "cast"]];
    contains_any_words & [&["color", "colors"], &["it", "this"]]
);
const ETB_FOR_EACH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["for", "each"]);
const ETB_COLOR_OF_MANA_PHRASES: &[&[&str]] =
    &[&["color", "of", "mana"], &["colors", "of", "mana"]];
const ETB_WHERE_X_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["where", "x", "is"]);
const ETB_EQUAL_WORD: &str = "equal";
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
            &["the", "spell", "mana", "value"],
            &["the", "spell's", "mana", "value"],
            &["the", "spells", "mana", "value"],
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
const ETB_MANA_VALUE_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(suffix & ["mana", "value"]);
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
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "creature"
            ],
            &[
                "the",
                "mana",
                "value",
                "of",
                "the",
                "sacrificed",
                "creatures"
            ],
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
const ETB_OPPONENT_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases & [&[&["opponents", "graveyard"], &["opponent", "graveyard"]]]
);
const ETB_AS_LONG_AS_THIS_IN_YOUR_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["as", "long", "as", "this"];
    contains_phrases & [&["is", "in", "your", "graveyard"]]
);
const ETB_AS_LONG_AS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["as", "long", "as"]);
const ETB_WITH_ADDITIONAL_COUNTERS_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_words & ["with", "additional"];
    contains_any_words & [&["counter", "counters"]]
);
const ETB_IT_BECOMES_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["it", "becomes"]);
const ETB_IT_BECOMES_YOUR_CHOICE_OF_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["it", "becomes", "your", "choice", "of"]);
const ETB_IN_ADDITION_TO_ITS_OTHER_TYPE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_phrases
        & [&[
            &["in", "addition", "to", "its", "other", "types"],
            &["in", "addition", "to", "its", "other", "type"],
        ]]
);
const ETB_IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["in", "addition", "to", "its", "other"]);
const ETB_AND_WORD: &str = "and";

fn etb_word_is_any(word: &str, expected: &[&str]) -> bool {
    expected.contains(&word)
}

fn etb_word_is(word: &str, expected: &str) -> bool {
    word == expected
}

fn etb_word_at_is_any(words: &[&str], idx: usize, expected: &[&str]) -> bool {
    words
        .get(idx)
        .is_some_and(|word| etb_word_is_any(word, expected))
}

fn etb_last_word_is(words: &[&str], expected: &str) -> bool {
    words.last().is_some_and(|word| etb_word_is(word, expected))
}

fn etb_token_word_is_any(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token
        .as_word()
        .is_some_and(|word| etb_word_is_any(word, expected))
}

fn etb_token_word_is(token: &OwnedLexToken, expected: &str) -> bool {
    token
        .as_word()
        .is_some_and(|word| etb_word_is(word, expected))
}

fn etb_find_prefix_shape_start(
    clause: LexedClause<'_>,
    shape: &ClauseShape<'static>,
) -> Option<usize> {
    (0..clause.word_len()).find(|&idx| {
        clause
            .after_words(idx)
            .is_some_and(|tail| shape.matches(tail))
    })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntersTappedWithCountersClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
    action_tokens: &'a [OwnedLexToken],
    entry_modifier_tokens: &'a [OwnedLexToken],
    with_tokens: &'a [OwnedLexToken],
    counter_clause_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntersWithCountersClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
    action_tokens: &'a [OwnedLexToken],
    counter_clause_tokens: &'a [OwnedLexToken],
    escaped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntersWithCounterConditionTailKind {
    If,
    Unless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntersWithCounterConditionTail<'a> {
    kind: EntersWithCounterConditionTailKind,
    condition_tokens: &'a [OwnedLexToken],
}

#[derive(Debug, Clone, PartialEq)]
struct EntersWithCounterKnownForEachTail {
    value: Value,
    scale_by_base_count: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum EntersWithCounterPlusTail {
    Supported(Value),
    Unsupported,
}

fn parse_enters_with_counter_condition_tail_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<EntersWithCounterConditionTail<'a>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::capture("marker", LexCaptureKind::OneOf(&["if", "unless"])),
        LexPattern::role_capture("condition", LexCaptureRole::Condition, LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let marker = matched.capture_clause("marker", clause)?.trimmed();
    let condition = matched
        .capture_clause_by_role(LexCaptureRole::Condition, clause)?
        .trimmed();
    if condition.is_empty() {
        return None;
    }
    let kind = enters_with_counter_condition_tail_kind(marker)?;

    Some(EntersWithCounterConditionTail {
        kind,
        condition_tokens: condition.tokens(),
    })
}

fn enters_with_counter_condition_tail_kind(
    marker: LexedClause<'_>,
) -> Option<EntersWithCounterConditionTailKind> {
    if clause_matches_phrase(marker, &["if"]) {
        return Some(EntersWithCounterConditionTailKind::If);
    }
    if clause_matches_phrase(marker, &["unless"]) {
        return Some(EntersWithCounterConditionTailKind::Unless);
    }
    None
}

fn clause_matches_phrase(clause: LexedClause<'_>, phrase: &[&str]) -> bool {
    LexPattern::new(&[LexPattern::phrase(phrase)]).matches_clause(clause)
}

fn clause_matches_any_phrase(clause: LexedClause<'_>, phrases: &[&[&str]]) -> bool {
    LexPattern::new(&[LexPattern::any_phrase(phrases)]).matches_clause(clause)
}

fn is_etb_source_reference_clause(clause: LexedClause<'_>) -> bool {
    if SOURCE_PRONOUN_SUBJECT_PATTERN.matches(clause) {
        return true;
    }
    let words = clause.word_refs();
    let Some(first) = words.first().copied() else {
        return false;
    };
    if !etb_word_is_any(first, ETB_SOURCE_TAIL_HEAD_WORDS) {
        return false;
    }
    if words.len() == 1 {
        return true;
    }
    if words.len() > 2 && words.get(1).copied() == Some("of") {
        return true;
    }
    if words.len() != 2 {
        return false;
    }
    etb_word_is_any(words[1], ETB_SOURCE_TAIL_NOUN_WORDS)
        || parse_card_type(words[1]).is_some()
        || parse_subtype_flexible(words[1]).is_some()
}

fn is_enters_with_counters_clause(clause: LexedClause<'_>) -> bool {
    const COUNTER_WORD_PATTERN: LexPattern<'static> =
        LexPattern::new(&[LexPattern::any_word(&["counter", "counters"])]);

    COUNTER_WORD_PATTERN.find_in_clause(clause).is_some()
}

fn is_enters_tapped_modifier_clause(clause: LexedClause<'_>) -> bool {
    const TAPPED_WORD_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::word("tapped")]);

    TAPPED_WORD_PATTERN.find_in_clause(clause).is_some()
}

fn enters_with_counter_action_escaped(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["escapes"])
}

fn is_kicked_source_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["it"], &["this", "spell"]])
}

fn is_mana_spent_cast_source_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["it"], &["spell"], &["this", "spell"]])
}

fn is_opponent_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["opponent"])
}

fn is_you_or_youve_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_any_phrase(clause, &[&["you"], &["youve"]])
}

fn is_your_opponents_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["your", "opponents"])
}

fn is_opponents_clause(clause: LexedClause<'_>) -> bool {
    clause_matches_phrase(clause, &["opponents"])
}

fn parse_enters_with_counters_clause_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<EntersWithCountersClause<'a>> {
    const ACTION_WORDS: &[&str] = &["enters", "escapes"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(ETB_ENTERS_OR_ESCAPES_PHRASES),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(ACTION_WORDS)),
        LexPattern::capture("with", LexCaptureKind::OneOf(&["with"])),
        LexPattern::object("counter_clause", LexCaptureKind::Rest),
    ]);
    const WITH_PHRASE: &[&str] = &["with"];
    const ENTRY_MODIFIER_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(ETB_ENTERS_OR_ESCAPES_PHRASES),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(ACTION_WORDS)),
        LexPattern::modifier("entry_modifier", LexCaptureKind::UntilPhrase(WITH_PHRASE)),
        LexPattern::capture("with", LexCaptureKind::OneOf(&["with"])),
        LexPattern::object("counter_clause", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN
        .match_clause(clause)
        .or_else(|| ENTRY_MODIFIER_PATTERN.match_clause(clause))?;
    let subject = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    let action = matched
        .capture_clause_by_role(LexCaptureRole::Action, clause)?
        .trimmed();
    let counter_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();

    if subject
        .tokens()
        .iter()
        .any(|token| token.is_period() || token.is_colon() || token.is_semicolon())
    {
        return None;
    }

    if !is_etb_source_reference_clause(subject) {
        return None;
    }
    if !is_enters_with_counters_clause(counter_clause) {
        return None;
    }

    Some(EntersWithCountersClause {
        subject_tokens: subject.tokens(),
        action_tokens: action.tokens(),
        counter_clause_tokens: counter_clause.tokens(),
        escaped: enters_with_counter_action_escaped(action),
    })
}

fn parse_enters_tapped_with_counters_clause_tokens<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<EntersTappedWithCountersClause<'a>> {
    const ACTION_WORDS: &[&str] = &["enter", "enters"];
    const WITH_PHRASE: &[&str] = &["with"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(ETB_ENTER_OR_ENTERS_PHRASES),
        ),
        LexPattern::action("action", LexCaptureKind::OneOf(ACTION_WORDS)),
        LexPattern::modifier("entry_modifier", LexCaptureKind::UntilPhrase(WITH_PHRASE)),
        LexPattern::capture("with", LexCaptureKind::OneOf(&["with"])),
        LexPattern::object("counter_clause", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let subject = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    let entry_modifier = matched
        .capture_clause_by_role(LexCaptureRole::Modifier, clause)?
        .trimmed();
    let action = matched
        .capture_clause_by_role(LexCaptureRole::Action, clause)?
        .trimmed();
    let with = matched.capture_clause("with", clause)?.trimmed();
    let counter_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();

    if !is_etb_source_reference_clause(subject) {
        return None;
    }
    if !is_enters_tapped_modifier_clause(entry_modifier) {
        return None;
    }
    if !is_enters_with_counters_clause(counter_clause) {
        return None;
    }

    Some(EntersTappedWithCountersClause {
        subject_tokens: subject.tokens(),
        action_tokens: action.tokens(),
        entry_modifier_tokens: entry_modifier.tokens(),
        with_tokens: with.tokens(),
        counter_clause_tokens: counter_clause.tokens(),
    })
}

fn etb_starts_with_trigger_intro_after_label(tokens: &[OwnedLexToken]) -> bool {
    let Some((_, body_tokens)) = split_em_dash_label_prefix(tokens) else {
        return false;
    };
    ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN.matches(LexedClause::new(body_tokens))
}

pub(crate) fn parse_enters_tapped_with_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }

    let Some(captured) = parse_enters_tapped_with_counters_clause_tokens(tokens) else {
        return Ok(None);
    };
    let _subject_tokens = captured.subject_tokens;
    let _entry_modifier_tokens = captured.entry_modifier_tokens;
    let _counter_clause_tokens = captured.counter_clause_tokens;

    let mut counter_line_tokens = Vec::new();
    counter_line_tokens.extend_from_slice(captured.subject_tokens);
    counter_line_tokens.extend_from_slice(captured.action_tokens);
    counter_line_tokens.extend_from_slice(captured.with_tokens);
    counter_line_tokens.extend_from_slice(captured.counter_clause_tokens);

    let Some(counters) = parse_enters_with_counters_line(&counter_line_tokens)? else {
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
        .is_some_and(|token| etb_token_word_is(token, ETB_IF_WORD))
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

    let Some(captured) = parse_enters_with_counters_clause_tokens(&clause_tokens) else {
        return Ok(None);
    };
    let _subject_tokens = captured.subject_tokens;
    let _action_tokens = captured.action_tokens;
    if captured.escaped {
        condition = Some((
            crate::ConditionExpr::ThisSpellEscaped,
            "it escaped".to_string(),
        ));
    }

    let mut added_abilities: Vec<Ability> = Vec::new();
    let mut after_with = captured.counter_clause_tokens;
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
        .is_some_and(|token| etb_token_word_is_any(token, ETB_ARTICLE_WORDS))
        && after_with
            .get(1)
            .is_some_and(|token| etb_token_word_is(token, ETB_ADDITIONAL_WORD))
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
            etb_token_word_is_any(token, ETB_COUNTER_OR_COUNTERS_WORDS)
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
        .is_some_and(|token| etb_token_word_is_any(token, ETB_SOURCE_TAIL_HEAD_WORDS))
    {
        tail = &tail[1..];
        if let Some(word) = tail.first().and_then(OwnedLexToken::as_word)
            && (etb_word_is_any(word, ETB_SOURCE_TAIL_NOUN_WORDS)
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
        } else if let Some(condition_tail) = parse_enters_with_counter_condition_tail_tokens(&tail)
        {
            let parsed =
                parse_enters_with_counter_condition_clause(condition_tail.condition_tokens)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported enters-with-counter condition (clause: '{}')",
                            full_words.join(" ")
                        ))
                    })?;
            match condition_tail.kind {
                EntersWithCounterConditionTailKind::If => {
                    let display = crate::runtime_backend::lexer::token_word_refs(
                        condition_tail.condition_tokens,
                    )
                    .join(" ");
                    condition = Some(combine_enters_with_counter_conditions(
                        condition,
                        (parsed, display),
                    ));
                }
                EntersWithCounterConditionTailKind::Unless => {
                    let display = parse_unless_enters_with_counter_condition_display(
                        condition_tail.condition_tokens,
                    )
                    .unwrap_or_else(|| {
                        format!(
                            "not {}",
                            crate::runtime_backend::lexer::token_word_refs(
                                condition_tail.condition_tokens,
                            )
                            .join(" ")
                        )
                    });
                    condition = Some(combine_enters_with_counter_conditions(
                        condition,
                        (crate::ConditionExpr::Not(Box::new(parsed)), display),
                    ));
                }
            }
        } else if let Some(plus_tail) = parse_enters_with_counter_plus_tail_tokens(&tail)? {
            match plus_tail {
                EntersWithCounterPlusTail::Supported(extra) => {
                    count = Value::Add(Box::new(count), Box::new(extra));
                }
                EntersWithCounterPlusTail::Unsupported => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported plus-self ETB counter clause (clause: '{}')",
                        full_words.join(" ")
                    )));
                }
            }
        } else if let Some(known_tail) = parse_enters_with_counter_known_for_each_tail_tokens(&tail)
        {
            count = if known_tail.scale_by_base_count {
                scaled_for_each_count(known_tail.value, &count)
            } else {
                known_tail.value
            };
        } else if let Some(dynamic) = parse_enters_with_counter_for_each_tail_tokens(&tail)? {
            count = dynamic;
        } else if tail_words.starts_with(&["for", "each"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported for-each self ETB counter clause (clause: '{}')",
                full_words.join(" ")
            )));
        } else if let Some(dynamic) = parse_enters_with_counter_equal_to_tail_tokens(&tail) {
            count = dynamic;
        } else if tail_words.starts_with(&["equal", "to"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported equal-to self ETB counter clause (clause: '{}')",
                full_words.join(" ")
            )));
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
    let tail_clause = LexedClause::new(&tail);
    let ability_tokens = if ENTERS_WITH_ADDED_ABILITIES_AND_WITH_TAIL_PATTERN.matches(tail_clause) {
        &tail[2..]
    } else if ENTERS_WITH_ADDED_ABILITIES_WITH_TAIL_PATTERN.matches(tail_clause) {
        &tail[1..]
    } else {
        return None;
    };
    if CAN_ATTACK_AS_THOUGH_NO_DEFENDER_PATTERN.matches(LexedClause::new(ability_tokens)) {
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

fn parse_enters_with_counter_known_for_each_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Option<EntersWithCounterKnownForEachTail> {
    let clause = LexedClause::new(tokens);

    const CREATURES_DIED_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["for", "each"]),
        LexPattern::object(
            "creature",
            LexCaptureKind::OneOf(&["creature", "creatures"]),
        ),
        LexPattern::phrase(&["that", "died", "this", "turn"]),
    ]);
    if CREATURES_DIED_PATTERN.match_clause(clause).is_some() {
        return Some(EntersWithCounterKnownForEachTail {
            value: Value::CreaturesDiedThisTurn,
            scale_by_base_count: true,
        });
    }

    const MANA_COLORS_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["for", "each"]),
        LexPattern::object("mana_color", LexCaptureKind::OneOf(&["color", "colour"])),
        LexPattern::phrase(&["of", "mana", "spent", "to", "cast"]),
        LexPattern::subject("source", LexCaptureKind::OneOf(&["it", "this"])),
    ]);
    if MANA_COLORS_PATTERN.match_clause(clause).is_some() {
        return Some(EntersWithCounterKnownForEachTail {
            value: Value::ColorsOfManaSpentToCastThisSpell,
            scale_by_base_count: true,
        });
    }

    const CONTROLLED_DIED_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["for", "each"]),
        LexPattern::object(
            "creature",
            LexCaptureKind::OneOf(&["creature", "creatures"]),
        ),
        LexPattern::phrase(&["that", "died", "under"]),
        LexPattern::subject("controller", LexCaptureKind::OneOf(&["your"])),
        LexPattern::phrase(&["control", "this", "turn"]),
    ]);
    if CONTROLLED_DIED_PATTERN.match_clause(clause).is_some() {
        return Some(EntersWithCounterKnownForEachTail {
            value: Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You),
            scale_by_base_count: true,
        });
    }

    const KICKED_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["for", "each", "time"]),
        LexPattern::subject("source", LexCaptureKind::UntilPhrase(&["was", "kicked"])),
        LexPattern::phrase(&["was", "kicked"]),
    ]);
    if let Some(matched) = KICKED_PATTERN.match_clause(clause) {
        let source = matched
            .capture_clause_by_role(LexCaptureRole::Subject, clause)?
            .trimmed();
        if is_kicked_source_clause(source) {
            return Some(EntersWithCounterKnownForEachTail {
                value: Value::KickCount,
                scale_by_base_count: true,
            });
        }
    }

    const MAGIC_LOSSES_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["for", "each"]),
        LexPattern::modifier("game_type", LexCaptureKind::OneOf(&["magic"])),
        LexPattern::object("game", LexCaptureKind::OneOf(&["game", "games"])),
        LexPattern::phrase(&[
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
        ]),
    ]);
    if MAGIC_LOSSES_PATTERN.match_clause(clause).is_some() {
        return Some(EntersWithCounterKnownForEachTail {
            value: Value::MagicGamesLostToOpponentsSinceLastWin,
            scale_by_base_count: false,
        });
    }

    None
}

fn parse_enters_with_counter_for_each_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["for", "each"]),
        LexPattern::object("dynamic_object", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(dynamic_object_clause) =
        matched.capture_clause_by_role(LexCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    if dynamic_object_clause.trimmed().is_empty() {
        return Ok(None);
    }

    parse_dynamic_cost_modifier_value(tokens)
}

fn parse_enters_with_counter_equal_to_tail_tokens(tokens: &[OwnedLexToken]) -> Option<Value> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["equal", "to"]),
        LexPattern::amount("value_body", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let value_body_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    if value_body_clause.is_empty() {
        return None;
    }

    parse_enters_with_counter_equal_to_value_clause(tokens)
}

fn parse_enters_with_counter_plus_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<EntersWithCounterPlusTail>, CardTextError> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("plus"),
        LexPattern::modifier("plus_body", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(plus_body_clause) = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)
    else {
        return Ok(None);
    };
    if plus_body_clause.trimmed().is_empty() {
        return Ok(Some(EntersWithCounterPlusTail::Unsupported));
    }

    if let Some(extra) = parse_enters_with_counter_plus_for_each_tail_tokens(tokens)? {
        return Ok(Some(EntersWithCounterPlusTail::Supported(extra)));
    }

    Ok(Some(EntersWithCounterPlusTail::Unsupported))
}

fn parse_enters_with_counter_plus_for_each_tail_tokens(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    const FOR_EACH_PHRASE: &[&str] = &["for", "each"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("plus"),
        LexPattern::modifier(
            "additional_counter",
            LexCaptureKind::UntilPhrase(FOR_EACH_PHRASE),
        ),
        LexPattern::object("for_each", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let Some(matched) = PATTERN.match_clause(clause) else {
        return Ok(None);
    };
    let Some(additional_counter_clause) =
        matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)
    else {
        return Ok(None);
    };
    let additional_counter_clause = additional_counter_clause.trimmed();
    let Some(for_each_clause) = matched.capture_clause_by_role(LexCaptureRole::Object, clause)
    else {
        return Ok(None);
    };
    let for_each_clause = for_each_clause.trimmed();
    if additional_counter_clause.is_empty() || !ETB_FOR_EACH_PREFIX_PATTERN.matches(for_each_clause)
    {
        return Ok(None);
    }

    parse_dynamic_cost_modifier_value(for_each_clause.tokens())
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

fn parse_enters_with_counter_colors_mana_spent_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::amount(
            "amount",
            LexCaptureKind::UntilAnyPhrase(ETB_COLOR_OF_MANA_PHRASES),
        ),
        LexPattern::object("spent_tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let amount_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    let spent_tail_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    if amount_clause.is_empty()
        || !ETB_COLORS_MANA_SPENT_CONDITION_TAIL_PATTERN.matches(spent_tail_clause)
    {
        return None;
    }

    let (comparison, used) = parse_quantity_comparison_prefix(
        amount_clause.tokens(),
        false,
        false,
        "enters-with condition",
    )
    .ok()?;
    if used != amount_clause.tokens().len() {
        return None;
    }
    crate::runtime_backend::util::comparison_to_strict_at_least_threshold(&comparison)
}

fn parse_enters_with_counter_you_cast_spells_this_turn_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    const YOU_CAST_PREFIXES: &[&[&str]] = &[
        &["youve", "cast"],
        &["you", "ve", "cast"],
        &["you", "cast"],
        &["you", "have", "cast"],
    ];
    const SPELLS_THIS_TURN_PHRASES: &[&[&str]] =
        &[&["spell", "this", "turn"], &["spells", "this", "turn"]];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::any_phrase(YOU_CAST_PREFIXES),
        LexPattern::amount(
            "amount",
            LexCaptureKind::UntilAnyPhrase(SPELLS_THIS_TURN_PHRASES),
        ),
        LexPattern::object("spell_tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let amount_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    let spell_tail_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    if amount_clause.is_empty() || !ETB_SPELLS_THIS_TURN_TAIL_PATTERN.matches(spell_tail_clause) {
        return None;
    }

    let (comparison, used) = parse_quantity_comparison_prefix(
        amount_clause.tokens(),
        false,
        false,
        "enters-with condition",
    )
    .ok()?;
    if used != amount_clause.tokens().len() {
        return None;
    }
    crate::runtime_backend::util::comparison_to_strict_at_least_threshold(&comparison)
}

fn parse_enters_with_counter_x_value_threshold_condition_tokens(
    tokens: &[OwnedLexToken],
) -> Option<u32> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["x", "is"]),
        LexPattern::amount("amount", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let amount_clause = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    if amount_clause.is_empty() {
        return None;
    }
    let (comparison, used) = parse_quantity_comparison_prefix(
        amount_clause.tokens(),
        false,
        false,
        "enters-with condition",
    )
    .ok()?;
    if used != amount_clause.tokens().len() {
        return None;
    }
    crate::runtime_backend::util::comparison_to_strict_at_least_threshold(&comparison)
}

fn parse_unless_enters_with_counter_condition_display(tokens: &[OwnedLexToken]) -> Option<String> {
    if let Some(amount) = parse_enters_with_counter_colors_mana_spent_condition_tokens(tokens) {
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
    let condition_clause = LexedClause::new(&condition_tokens);
    if condition_clause.is_empty() {
        return None;
    }

    if ETB_ATTACKED_THIS_TURN_CONDITION_PATTERN.matches(condition_clause) {
        return Some(crate::ConditionExpr::AttackedThisTurn);
    }
    if ETB_SOURCE_WAS_CAST_CONDITION_PATTERN.matches(condition_clause) {
        return Some(crate::ConditionExpr::SourceWasCast);
    }
    if ETB_THIS_SPELL_WAS_KICKED_CONDITION_PATTERN.matches(condition_clause) {
        return Some(crate::ConditionExpr::ThisSpellWasKicked);
    }
    if ETB_THIS_SPELL_ESCAPED_CONDITION_PATTERN.matches(condition_clause) {
        return Some(crate::ConditionExpr::ThisSpellEscaped);
    }
    if ETB_CREATURE_DIED_THIS_TURN_CONDITION_PATTERN.matches(condition_clause) {
        return Some(crate::ConditionExpr::CreatureDiedThisTurn);
    }
    if ETB_OPPONENT_LOST_LIFE_THIS_TURN_CONDITION_PATTERN.matches(condition_clause) {
        return Some(crate::ConditionExpr::OpponentLostLifeThisTurn);
    }
    if ETB_PERMANENT_LEFT_UNDER_YOUR_CONTROL_CONDITION_PATTERN.matches(condition_clause) {
        return Some(crate::ConditionExpr::PermanentLeftBattlefieldUnderYourControlThisTurn);
    }
    if ETB_NOT_CAST_OR_NO_MANA_SPENT_CONDITION_PATTERN.matches(condition_clause) {
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

    if let Some(amount) =
        parse_enters_with_counter_x_value_threshold_condition_tokens(&condition_tokens)
    {
        return Some(crate::ConditionExpr::XValueAtLeast(amount));
    }

    if let Some(amount) =
        parse_enters_with_counter_you_cast_spells_this_turn_condition_tokens(&condition_tokens)
    {
        return Some(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
            player: PlayerFilter::You,
            count: amount,
        });
    }

    if let Some(amount) =
        parse_enters_with_counter_colors_mana_spent_condition_tokens(&condition_tokens)
    {
        return Some(crate::ConditionExpr::ColorsOfManaSpentToCastThisSpellOrMore(amount));
    }

    if let Some(amount) =
        crate::runtime_backend::grammar::filters::parse_same_color_mana_spent_to_cast_predicate(
            &condition_tokens,
        )
    {
        return Some(crate::ConditionExpr::SameColorManaSpentToCastThisSpellAtLeast(amount));
    }

    parse_static_condition_clause(&condition_tokens).ok()
}

fn parse_enters_with_counter_equal_to_value_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let trimmed = trim_edge_punctuation(tokens);
    let value_body = parse_equal_to_value_body_clause(&trimmed)?;
    if let Some(value) = parse_equal_to_mana_spent_to_cast_value(&trimmed) {
        return Some(value);
    }

    let mut where_tokens = Vec::with_capacity(trimmed.len() + 1);
    where_tokens.push(OwnedLexToken::word(
        "where".to_string(),
        TextSpan::synthetic(),
    ));
    where_tokens.push(OwnedLexToken::word("x".to_string(), TextSpan::synthetic()));
    where_tokens.push(OwnedLexToken::word("is".to_string(), TextSpan::synthetic()));
    where_tokens.extend_from_slice(value_body.tokens());

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

fn parse_equal_to_value_body_clause<'a>(tokens: &'a [OwnedLexToken]) -> Option<LexedClause<'a>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["equal", "to"]),
        LexPattern::amount("value_body", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let value_body = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    (!value_body.is_empty()).then_some(value_body)
}

fn parse_equal_to_mana_spent_to_cast_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&[
            "equal", "to", "the", "amount", "of", "mana", "spent", "to", "cast",
        ]),
        LexPattern::subject("cast_source", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let cast_source_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    if is_mana_spent_cast_source_clause(cast_source_clause) {
        Some(Value::ManaSpentToCastThisSpell.with_surface_hint(ValueSurfaceHint::EqualTo))
    } else {
        None
    }
}

fn parse_equal_to_greatest_cards_drawn_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    const WITH_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["equal", "to", "the", "greatest", "number", "of", "cards"]),
        LexPattern::modifier("article", LexCaptureKind::OneOf(&["an"])),
        LexPattern::subject("drawer", LexCaptureKind::OneOf(&["opponent"])),
        LexPattern::phrase(&["has", "drawn", "this", "turn"]),
    ]);
    const WITHOUT_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["equal", "to", "greatest", "number", "of", "cards"]),
        LexPattern::modifier("article", LexCaptureKind::OneOf(&["an"])),
        LexPattern::subject("drawer", LexCaptureKind::OneOf(&["opponent"])),
        LexPattern::phrase(&["has", "drawn", "this", "turn"]),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = WITH_THE_PATTERN
        .match_clause(clause)
        .or_else(|| WITHOUT_THE_PATTERN.match_clause(clause))?;
    let drawer = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    if is_opponent_clause(drawer) {
        Some(Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent))
    } else {
        None
    }
}

pub(crate) fn parse_value_binding_clause(tokens: &[OwnedLexToken]) -> Option<Value> {
    let clause = LexedClause::new(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches(clause) {
        return None;
    }
    let word_view = crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens);
    let words = word_view.word_refs();

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
    if ETB_DEVOTION_VALUE_PATTERN.matches(clause) {
        if let Ok(Some(value)) = parse_devotion_value_from_add_clause(tokens) {
            return Some(value);
        }
    }

    // where X is the total number of cards in all players' hands
    if ETB_ALL_PLAYERS_HAND_COUNT_VALUE_PATTERN.matches(clause) {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }

    if clause
        .after_words(3)
        .is_some_and(|tail| ETB_SAME_NAME_AS_TRIGGERING_SPELL_GRAVEYARD_VALUE_PATTERN.matches(tail))
    {
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

    if let Some(tail) = clause.after_words(3)
        && (ETB_EXILED_CARD_MANA_VALUE_PATTERN.matches(tail)
            || ETB_TRIGGERING_SPELL_MANA_VALUE_PATTERN.matches(tail))
    {
        let tag = if ETB_TRIGGERING_SPELL_MANA_VALUE_PATTERN.matches(tail) {
            "triggering"
        } else {
            IT_TAG
        };
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(tag),
        ))));
    }

    // where X is the number of cards in your hand
    if ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches(clause) {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }

    // where X is the number of creatures in your party
    if ETB_YOUR_PARTY_SIZE_VALUE_PATTERN.matches(clause) {
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

    // where X is the greatest number of <objects> <player> controls
    if let Some(value) = parse_where_x_is_greatest_number_of_filter_value(tokens) {
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
    let clause = LexedClause::new(tokens);
    if !ETB_WHERE_X_IS_PREFIX_PATTERN.matches(clause) {
        return None;
    }
    let word_view = crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens);
    let words = word_view.word_refs();
    let tagged_it = ChooseSpec::Tagged(TagKey::from(IT_TAG));
    let tail = words.get(3..)?;
    let tail_clause = clause.after_words(3)?;
    if tail.len() >= 2
        && etb_last_word_is(tail, ETB_POWER_WORD)
        && let Some(surface) =
            source_reference_surface_for_possessive_words(&tail[..tail.len() - 1])
    {
        return Some(Value::PowerOf(Box::new(
            ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
        )));
    }
    if tail.len() >= 2
        && etb_last_word_is(tail, ETB_TOUGHNESS_WORD)
        && let Some(surface) =
            source_reference_surface_for_possessive_words(&tail[..tail.len() - 1])
    {
        return Some(Value::ToughnessOf(Box::new(
            ChooseSpec::Source.with_surface_hint(ChooseSpecSurfaceHint::SourceReference(surface)),
        )));
    }
    if tail.len() >= 3
        && ETB_MANA_VALUE_TAIL_PATTERN.matches(tail_clause)
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
    let captured = parse_where_x_fixed_plus_reference_clause(tokens)?;
    let (fixed_value, fixed_used) = parse_number(captured.fixed_tokens)?;
    if fixed_used != captured.fixed_tokens.len() {
        return None;
    }
    let fixed_value = fixed_value as i32;
    if fixed_value < 0 {
        return None;
    }

    let reference_value = if ETB_SACRIFICED_CREATURE_POWER_PREFIX_PATTERN
        .matches(captured.reference_clause)
    {
        Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
    } else if ETB_SACRIFICED_CREATURE_TOUGHNESS_PREFIX_PATTERN.matches(captured.reference_clause) {
        Value::ToughnessOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
    } else if ETB_TAGGED_CREATURE_MANA_VALUE_PREFIX_PATTERN.matches(captured.reference_clause) {
        Value::ManaValueOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))))
    } else {
        return None;
    };

    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value)),
        Box::new(reference_value),
    ))
}

#[derive(Debug, Clone, Copy)]
struct FixedPlusReferenceClause<'a> {
    fixed_tokens: &'a [OwnedLexToken],
    reference_clause: LexedClause<'a>,
}

fn parse_where_x_fixed_plus_reference_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<FixedPlusReferenceClause<'a>> {
    const PLUS_PHRASE: &[&str] = &["plus"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is"]),
        LexPattern::amount("fixed", LexCaptureKind::UntilPhrase(PLUS_PHRASE)),
        LexPattern::phrase(PLUS_PHRASE),
        LexPattern::object("reference", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let fixed = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    let reference = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    if fixed.is_empty() || reference.is_empty() {
        return None;
    }

    Some(FixedPlusReferenceClause {
        fixed_tokens: fixed.tokens(),
        reference_clause: reference,
    })
}

pub(crate) fn parse_where_x_life_gained_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let subject = parse_where_x_life_gained_this_turn_clause(tokens)?;
    if is_you_or_youve_clause(subject) {
        Some(Value::LifeGainedThisTurn(PlayerFilter::You))
    } else {
        None
    }
}

fn parse_where_x_life_gained_this_turn_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const WITH_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "the", "amount", "of", "life"]),
        LexPattern::subject("player", LexCaptureKind::OneOf(&["you", "youve"])),
        LexPattern::phrase(&["gained", "this", "turn"]),
    ]);
    const WITHOUT_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "amount", "of", "life"]),
        LexPattern::subject("player", LexCaptureKind::OneOf(&["you", "youve"])),
        LexPattern::phrase(&["gained", "this", "turn"]),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = WITH_THE_PATTERN
        .match_clause(clause)
        .or_else(|| WITHOUT_THE_PATTERN.match_clause(clause))?;
    matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)
        .map(LexedClause::trimmed)
}

pub(crate) fn parse_where_x_life_lost_this_turn_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let player = parse_where_x_life_lost_this_turn_clause(tokens)?;
    if is_your_opponents_clause(player) {
        Some(Value::LifeLostThisTurn(PlayerFilter::Opponent))
    } else {
        None
    }
}

fn parse_where_x_life_lost_this_turn_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const THIS_TURN_PHRASE: &[&str] = &["this", "turn"];
    const WITH_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "the", "total", "life", "lost", "by"]),
        LexPattern::subject("player", LexCaptureKind::UntilPhrase(THIS_TURN_PHRASE)),
        LexPattern::phrase(THIS_TURN_PHRASE),
    ]);
    const WITHOUT_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "total", "life", "lost", "by"]),
        LexPattern::subject("player", LexCaptureKind::UntilPhrase(THIS_TURN_PHRASE)),
        LexPattern::phrase(THIS_TURN_PHRASE),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = WITH_THE_PATTERN
        .match_clause(clause)
        .or_else(|| WITHOUT_THE_PATTERN.match_clause(clause))?;
    let player = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    (!player.is_empty()).then_some(player)
}

pub(crate) fn parse_where_x_opponents_dealt_combat_damage_this_turn_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let players = parse_where_x_opponents_dealt_combat_damage_this_turn_clause(tokens)?;
    if is_opponents_clause(players) {
        Some(Value::CountPlayers(PlayerFilter::Opponent))
    } else {
        None
    }
}

fn parse_where_x_opponents_dealt_combat_damage_this_turn_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const DAMAGE_TAIL: &[&str] = &["that", "were", "dealt", "combat", "damage", "this", "turn"];
    const WITH_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "the", "number", "of"]),
        LexPattern::subject("players", LexCaptureKind::UntilPhrase(DAMAGE_TAIL)),
        LexPattern::phrase(DAMAGE_TAIL),
    ]);
    const WITHOUT_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "number", "of"]),
        LexPattern::subject("players", LexCaptureKind::UntilPhrase(DAMAGE_TAIL)),
        LexPattern::phrase(DAMAGE_TAIL),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = WITH_THE_PATTERN
        .match_clause(clause)
        .or_else(|| WITHOUT_THE_PATTERN.match_clause(clause))?;
    let players = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    (!players.is_empty()).then_some(players)
}

pub(crate) fn parse_where_x_noncombat_damage_to_opponents_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let damaged_player = parse_where_x_noncombat_damage_to_opponents_clause(tokens)?;
    if is_your_opponents_clause(damaged_player) {
        Some(Value::NoncombatDamageDealtToPlayersThisTurn(
            PlayerFilter::Opponent,
        ))
    } else {
        None
    }
}

fn parse_where_x_noncombat_damage_to_opponents_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const THIS_TURN_PHRASE: &[&str] = &["this", "turn"];
    const WITH_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&[
            "where",
            "x",
            "is",
            "the",
            "total",
            "amount",
            "of",
            "noncombat",
            "damage",
            "dealt",
            "to",
        ]),
        LexPattern::subject(
            "damaged_player",
            LexCaptureKind::UntilPhrase(THIS_TURN_PHRASE),
        ),
        LexPattern::phrase(THIS_TURN_PHRASE),
    ]);
    const WITHOUT_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&[
            "where",
            "x",
            "is",
            "total",
            "amount",
            "of",
            "noncombat",
            "damage",
            "dealt",
            "to",
        ]),
        LexPattern::subject(
            "damaged_player",
            LexCaptureKind::UntilPhrase(THIS_TURN_PHRASE),
        ),
        LexPattern::phrase(THIS_TURN_PHRASE),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = WITH_THE_PATTERN
        .match_clause(clause)
        .or_else(|| WITHOUT_THE_PATTERN.match_clause(clause))?;
    let damaged_player = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    (!damaged_player.is_empty()).then_some(damaged_player)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EtbAggregateKind {
    Total,
    Greatest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EtbAggregateValueKind {
    Power,
    Toughness,
    ManaValue,
}

#[derive(Debug, Clone, Copy)]
struct EtbWhereXAggregateFilterClause<'a> {
    aggregate: EtbAggregateKind,
    value_kind: EtbAggregateValueKind,
    filter_clause: LexedClause<'a>,
}

fn parse_etb_aggregate_kind_clause(clause: LexedClause<'_>) -> Option<EtbAggregateKind> {
    if clause_matches_phrase(clause, &["total"]) {
        return Some(EtbAggregateKind::Total);
    }
    if clause_matches_phrase(clause, &["greatest"]) {
        return Some(EtbAggregateKind::Greatest);
    }
    None
}

fn parse_etb_aggregate_value_kind_clause(clause: LexedClause<'_>) -> Option<EtbAggregateValueKind> {
    if clause_matches_phrase(clause, &["power"]) {
        return Some(EtbAggregateValueKind::Power);
    }
    if clause_matches_phrase(clause, &["toughness"]) {
        return Some(EtbAggregateValueKind::Toughness);
    }
    if clause_matches_phrase(clause, &["mana", "value"]) {
        return Some(EtbAggregateValueKind::ManaValue);
    }
    None
}

fn parse_where_x_aggregate_filter_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<EtbWhereXAggregateFilterClause<'a>> {
    const OPTIONAL_THE: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
    const RELATION_PHRASES: &[&[&str]] = &[&["of"], &["among"]];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is"]),
        LexPattern::optional(OPTIONAL_THE),
        LexPattern::action("aggregate", LexCaptureKind::OneOf(&["total", "greatest"])),
        LexPattern::amount(
            "value_kind",
            LexCaptureKind::UntilAnyPhrase(RELATION_PHRASES),
        ),
        LexPattern::any_phrase(RELATION_PHRASES),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let aggregate_clause = matched.capture_clause_by_role(LexCaptureRole::Action, clause)?;
    let value_kind_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    let filter_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    if filter_clause.is_empty() {
        return None;
    }

    let aggregate = parse_etb_aggregate_kind_clause(aggregate_clause)?;
    let value_kind = parse_etb_aggregate_value_kind_clause(value_kind_clause)?;

    Some(EtbWhereXAggregateFilterClause {
        aggregate,
        value_kind,
        filter_clause,
    })
}

pub(crate) fn parse_where_x_is_aggregate_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let parsed = parse_where_x_aggregate_filter_clause(tokens)?;

    if parsed.aggregate == EtbAggregateKind::Greatest
        && parsed.value_kind == EtbAggregateValueKind::ManaValue
    {
        if let Some(value) =
            parse_where_x_greatest_commander_mana_value_filter(parsed.filter_clause.tokens())
        {
            return Some(value);
        }
    }

    let filter_tokens = parsed.filter_clause.tokens();
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    let should_try_split = ETB_AND_GRAVEYARD_MARKER_PATTERN
        .matches(LexedClause::new(filter_tokens))
        && filter_words
            .iter()
            .any(|word| etb_word_is_any(word, ETB_CONTROL_OWN_WORDS));
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

    if ETB_SACRIFICED_MARKER_PATTERN.matches(LexedClause::new(filter_tokens)) {
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
    if filter_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"))
        && filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
    {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }

    match (parsed.aggregate, parsed.value_kind) {
        (EtbAggregateKind::Total, EtbAggregateValueKind::Power) => Some(Value::TotalPower(filter)),
        (EtbAggregateKind::Total, EtbAggregateValueKind::Toughness) => {
            Some(Value::TotalToughness(filter))
        }
        (EtbAggregateKind::Total, EtbAggregateValueKind::ManaValue) => {
            Some(Value::TotalManaValue(filter))
        }
        (EtbAggregateKind::Greatest, EtbAggregateValueKind::Power) => {
            Some(Value::GreatestPower(filter))
        }
        (EtbAggregateKind::Greatest, EtbAggregateValueKind::Toughness) => {
            Some(Value::GreatestToughness(filter))
        }
        (EtbAggregateKind::Greatest, EtbAggregateValueKind::ManaValue) => {
            Some(Value::GreatestManaValue(filter))
        }
    }
}

pub(crate) fn parse_where_x_greatest_commander_mana_value_filter(
    commander_tokens: &[OwnedLexToken],
) -> Option<Value> {
    let commander_words = crate::runtime_backend::token_word_refs(commander_tokens);
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
    let filter_clause = parse_where_x_differently_named_filter_clause(tokens)?;
    let filter = parse_object_filter_lexed(filter_clause.tokens(), false).ok()?;
    Some(Value::DistinctNames(filter))
}

fn parse_where_x_differently_named_filter_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const OPTIONAL_THE: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is"]),
        LexPattern::optional(OPTIONAL_THE),
        LexPattern::phrase(&["number", "of", "differently", "named"]),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let filter_clause = PATTERN
        .match_clause(clause)?
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    (!filter_clause.is_empty()).then_some(filter_clause)
}

pub(crate) fn parse_where_x_is_number_of_different_powers_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_clause = parse_where_x_different_powers_filter_clause(tokens)?;
    let filter = parse_object_filter_lexed(filter_clause.tokens(), false).ok()?;
    Some(Value::DistinctPowers(filter))
}

fn parse_where_x_different_powers_filter_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const OPTIONAL_THE: &[LexPatternAtom<'static>] = &[LexPattern::word("the")];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is"]),
        LexPattern::optional(OPTIONAL_THE),
        LexPattern::phrase(&["number", "of", "different"]),
        LexPattern::object("power", LexCaptureKind::OneOf(&["power", "powers"])),
        LexPattern::phrase(&["among"]),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let filter_clause = PATTERN
        .match_clause(clause)?
        .capture_clause("filter", clause)?
        .trimmed();
    (!filter_clause.is_empty()).then_some(filter_clause)
}

pub(crate) fn parse_where_x_is_greatest_number_of_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let filter_clause = parse_where_x_greatest_number_filter_clause(tokens)?;
    let filter = parse_object_filter_lexed(filter_clause.tokens(), false).ok()?;
    filter.controller.as_ref()?;
    Some(Value::GreatestCount(filter))
}

fn parse_where_x_greatest_number_filter_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const WITH_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "the", "greatest", "number", "of"]),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ]);
    const WITHOUT_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "greatest", "number", "of"]),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = WITH_THE_PATTERN
        .match_clause(clause)
        .or_else(|| WITHOUT_THE_PATTERN.match_clause(clause))?;
    let filter = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    (!filter.is_empty()).then_some(filter)
}

pub(crate) fn parse_where_x_is_number_of_filter_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let captured = parse_where_x_number_of_filter_clause(tokens)?;

    if ETB_COMMON_CREATURE_TYPE_VALUE_PATTERN.matches(LexedClause::new(tokens)) {
        return None;
    }

    let multiplier = parse_number_of_filter_multiplier_clause(captured.multiplier_clause)?;
    let filter_tokens = captured.filter_tokens;
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    if let Some(value) = parse_number_of_counters_on_source_value(&filter_words) {
        return Some(value);
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) {
        return Some(scale_where_x_number_value(value, multiplier));
    }
    if ETB_CARD_TYPES_AMONG_CARDS_PREFIX_PATTERN.matches(LexedClause::new(filter_tokens))
        && ETB_GRAVEYARD_MARKER_PATTERN.matches(LexedClause::new(filter_tokens))
    {
        let player = if ETB_YOUR_GRAVEYARD_PATTERN.matches(LexedClause::new(filter_tokens)) {
            PlayerFilter::You
        } else if ETB_OPPONENT_GRAVEYARD_PATTERN.matches(LexedClause::new(filter_tokens)) {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Some(scale_where_x_number_value(
            Value::CardTypesInGraveyard(player),
            multiplier,
        ));
    }
    if ETB_CARD_TYPES_AMONG_PREFIX_PATTERN.matches(LexedClause::new(filter_tokens)) {
        let mut scope_tokens = &filter_tokens[3..];
        if scope_tokens
            .first()
            .is_some_and(|token| etb_token_word_is(token, ETB_THE_WORD))
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

#[derive(Debug, Clone, Copy)]
struct WhereXNumberOfFilterClause<'a> {
    multiplier_clause: LexedClause<'a>,
    filter_tokens: &'a [OwnedLexToken],
}

fn parse_where_x_number_of_filter_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<WhereXNumberOfFilterClause<'a>> {
    const NUMBER_OF_PHRASE: &[&str] = &["number", "of"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is"]),
        LexPattern::amount("multiplier", LexCaptureKind::UntilPhrase(NUMBER_OF_PHRASE)),
        LexPattern::phrase(NUMBER_OF_PHRASE),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let multiplier = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    let filter = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    if filter.is_empty() {
        return None;
    }

    Some(WhereXNumberOfFilterClause {
        multiplier_clause: multiplier,
        filter_tokens: filter.tokens(),
    })
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
        .is_some_and(|word| is_article(word) || etb_word_is(word, ETB_ONE_WORD))
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
    if !etb_word_at_is_any(&filter_words, idx, ETB_COUNTER_OR_COUNTERS_WORDS) {
        return None;
    }
    idx += 1;
    if filter_words.get(idx).copied() != Some("on") {
        return None;
    }
    idx += 1;
    let source_words = filter_words.get(idx..)?;
    if is_source_reference_words(source_words) {
        return Some(Value::CountersOnSource(counter_type));
    }

    match source_words {
        ["it"]
        | ["this"]
        | ["this", "card"]
        | ["this", "creature"]
        | ["this", "permanent"]
        | ["this", "source"]
        | ["this", "artifact"]
        | ["this", "land"]
        | ["this", "enchantment"]
        | ["thiss"]
        | ["thiss", "card"]
        | ["thiss", "creature"]
        | ["thiss", "permanent"]
        | ["thiss", "source"]
        | ["thiss", "artifact"]
        | ["this", "equipment"]
        | ["thiss", "land"]
        | ["thiss", "enchantment"]
        | ["thiss", "equipment"] => Some(Value::CountersOnSource(counter_type)),
        _ => None,
    }
}

pub(crate) fn parse_where_x_is_fixed_plus_number_of_filter_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let captured = parse_where_x_fixed_plus_number_of_filter_clause(tokens)?;
    let (fixed_value, fixed_used) = parse_number(captured.fixed_tokens)?;
    if fixed_used != captured.fixed_tokens.len() {
        return None;
    }
    let filter_tokens = captured.filter_tokens;
    let filter_words = crate::runtime_backend::token_word_refs(filter_tokens);
    if let Some(counter_value) = parse_number_of_counters_on_source_value(&filter_words) {
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(counter_value),
        ));
    }
    if let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) {
        return Some(Value::Add(
            Box::new(Value::Fixed(fixed_value as i32)),
            Box::new(value),
        ));
    }
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Add(
        Box::new(Value::Fixed(fixed_value as i32)),
        Box::new(Value::Count(filter)),
    ))
}

#[derive(Debug, Clone, Copy)]
struct FixedPlusNumberOfFilterClause<'a> {
    fixed_tokens: &'a [OwnedLexToken],
    filter_tokens: &'a [OwnedLexToken],
}

fn parse_where_x_fixed_plus_number_of_filter_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<FixedPlusNumberOfFilterClause<'a>> {
    const PLUS_NUMBER_OF_PHRASES: &[&[&str]] =
        &[&["plus", "number", "of"], &["plus", "the", "number", "of"]];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is"]),
        LexPattern::amount(
            "fixed",
            LexCaptureKind::UntilAnyPhrase(PLUS_NUMBER_OF_PHRASES),
        ),
        LexPattern::any_phrase(PLUS_NUMBER_OF_PHRASES),
        LexPattern::object("filter", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let fixed = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    let filter = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    if fixed.is_empty() || filter.is_empty() {
        return None;
    }

    Some(FixedPlusNumberOfFilterClause {
        fixed_tokens: fixed.tokens(),
        filter_tokens: filter.tokens(),
    })
}

pub(crate) fn parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let captured = parse_where_x_number_of_filter_plus_or_minus_fixed_clause(tokens)?;
    let filter_tokens = trim_commas(captured.filter_tokens);
    let count_value = if ETB_YOUR_HAND_COUNT_VALUE_PATTERN.matches(LexedClause::new(&filter_tokens))
    {
        Value::CardsInHand(PlayerFilter::You)
    } else {
        let filter = parse_object_filter(&filter_tokens, false).ok()?;
        Value::Count(filter)
    };

    let offset_tokens = trim_commas(captured.offset_tokens);
    let (offset_value, used) = parse_number(&offset_tokens)?;
    let trailing_words = crate::runtime_backend::token_word_refs(&offset_tokens[used..]);
    if !trailing_words.is_empty() {
        return None;
    }

    let signed_offset = signed_offset_from_number_of_filter_operator_clause(
        captured.operator_clause,
        offset_value,
    )?;
    Some(Value::Add(
        Box::new(count_value),
        Box::new(Value::Fixed(signed_offset)),
    ))
}

fn signed_offset_from_number_of_filter_operator_clause(
    clause: LexedClause<'_>,
    offset_value: u32,
) -> Option<i32> {
    if clause_matches_phrase(clause, &["minus"]) {
        return Some(-(offset_value as i32));
    }
    if clause_matches_phrase(clause, &["plus"]) {
        return Some(offset_value as i32);
    }
    None
}

fn parse_number_of_filter_multiplier_clause(clause: LexedClause<'_>) -> Option<i32> {
    if clause.is_empty() || clause_matches_any_phrase(clause, &[&["the"], &["the", "total"]]) {
        return Some(1);
    }
    if clause_matches_any_phrase(
        clause,
        &[
            &["twice"],
            &["twice", "the"],
            &["two", "times"],
            &["two", "times", "the"],
        ],
    ) {
        return Some(2);
    }
    None
}

#[derive(Debug, Clone, Copy)]
struct NumberOfFilterPlusOrMinusFixedClause<'a> {
    filter_tokens: &'a [OwnedLexToken],
    operator_clause: LexedClause<'a>,
    offset_tokens: &'a [OwnedLexToken],
}

fn parse_where_x_number_of_filter_plus_or_minus_fixed_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<NumberOfFilterPlusOrMinusFixedClause<'a>> {
    const OPERATOR_PHRASES: &[&[&str]] = &[&["plus"], &["minus"]];
    const WITH_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "the", "number", "of"]),
        LexPattern::object("filter", LexCaptureKind::UntilAnyPhrase(OPERATOR_PHRASES)),
        LexPattern::action("operator", LexCaptureKind::OneOf(&["plus", "minus"])),
        LexPattern::amount("offset", LexCaptureKind::Rest),
    ]);
    const WITHOUT_THE_PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["where", "x", "is", "number", "of"]),
        LexPattern::object("filter", LexCaptureKind::UntilAnyPhrase(OPERATOR_PHRASES)),
        LexPattern::action("operator", LexCaptureKind::OneOf(&["plus", "minus"])),
        LexPattern::amount("offset", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = WITH_THE_PATTERN
        .match_clause(clause)
        .or_else(|| WITHOUT_THE_PATTERN.match_clause(clause))?;
    let filter = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    let operator = matched
        .capture_clause_by_role(LexCaptureRole::Action, clause)?
        .trimmed();
    let offset = matched
        .capture_clause_by_role(LexCaptureRole::Amount, clause)?
        .trimmed();
    if filter.is_empty() || offset.is_empty() {
        return None;
    }

    Some(NumberOfFilterPlusOrMinusFixedClause {
        filter_tokens: filter.tokens(),
        operator_clause: operator,
        offset_tokens: offset.tokens(),
    })
}

pub(crate) fn token_index_for_word_index(
    tokens: &[OwnedLexToken],
    word_index: usize,
) -> Option<usize> {
    crate::runtime_backend::grammar::primitives::TokenWordView::new(tokens)
        .token_index_for_word_index(word_index)
}

#[derive(Debug, Clone, Copy)]
struct EtbEntryFilterClause<'a> {
    filter_tokens: &'a [OwnedLexToken],
    tail_clause: LexedClause<'a>,
}

fn parse_entry_filter_clause<'a>(tokens: &'a [OwnedLexToken]) -> Option<EtbEntryFilterClause<'a>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::object(
            "filter",
            LexCaptureKind::UntilAnyPhrase(ETB_ENTER_OR_ENTERS_PHRASES),
        ),
        LexPattern::action("entry_action", LexCaptureKind::OneOf(&["enter", "enters"])),
        LexPattern::tail("entry_tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let filter_clause = matched
        .capture_clause_by_role(LexCaptureRole::Object, clause)?
        .trimmed();
    let tail_clause = matched
        .capture_clause_by_role(LexCaptureRole::Tail, clause)?
        .trimmed();
    if filter_clause.is_empty() || tail_clause.is_empty() {
        return None;
    }

    Some(EtbEntryFilterClause {
        filter_tokens: filter_clause.tokens(),
        tail_clause,
    })
}

pub(crate) fn parse_enters_tapped_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if clause_words
        .first()
        .is_some_and(|word| etb_word_is_any(word, ETB_TRIGGER_INTRO_WORDS))
    {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        if ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches(LexedClause::new(tokens))
            && ETB_TAPPED_MARKER_PATTERN.matches(LexedClause::new(tokens))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if ETB_UNLESS_MARKER_PATTERN.matches(LexedClause::new(tokens)) {
        return Ok(None);
    }
    let Some(entry_clause) = parse_entry_filter_clause(tokens) else {
        return Ok(None);
    };
    if !ETB_TAPPED_MARKER_PATTERN.matches(entry_clause.tail_clause) {
        return Ok(None);
    }
    if LexedClause::new(tokens)
        .token(0)
        .is_some_and(|token| etb_token_word_is(token, ETB_THIS_WORD))
    {
        return Ok(None);
    }
    if ETB_COPY_MARKER_PATTERN.matches(LexedClause::new(tokens)) {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}') [rule=enters-as-copy]",
            clause_words.join(" ")
        )));
    }
    let before_enter = entry_clause.filter_tokens;
    let before_word_len = LexedClause::new(before_enter).word_len();
    let mut controller_override: Option<PlayerFilter> = None;
    let mut filter_end = before_enter.len();
    let find_suffix_cut = |suffix_len: usize, shape: ClauseShape<'static>| -> Option<usize> {
        if before_word_len < suffix_len {
            return None;
        }
        let keep_word_count = before_word_len - suffix_len;
        let suffix_clause =
            LexedClause::new(before_enter).between_word_range(keep_word_count, before_word_len)?;
        if !shape.matches(suffix_clause) {
            return None;
        }
        Some(
            LexedClause::new(before_enter)
                .token_index_for_word_or_end(keep_word_count)
                .unwrap_or(before_enter.len()),
        )
    };
    if let Some(cut) = find_suffix_cut(4, ETB_PLAYED_BY_YOUR_OPPONENTS_SUFFIX_PATTERN) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = cut;
    } else if let Some(cut) = find_suffix_cut(4, ETB_PLAYED_BY_AN_OPPONENT_SUFFIX_PATTERN) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = cut;
    } else if let Some(cut) = find_suffix_cut(3, ETB_PLAYED_BY_OPPONENTS_SUFFIX_PATTERN) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = cut;
    }
    let mut filter = match parse_object_filter(&before_enter[..filter_end], false) {
        Ok(filter) => filter,
        Err(_) if filter_end == before_enter.len() && before_word_len > 0 => {
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
        .is_some_and(|word| etb_word_is_any(word, ETB_TRIGGER_INTRO_WORDS))
    {
        return Ok(None);
    }
    if etb_starts_with_trigger_intro_after_label(tokens) {
        return Ok(None);
    }
    if ETB_UNLESS_MARKER_PATTERN.matches(LexedClause::new(tokens))
        || LexedClause::new(tokens)
            .token(0)
            .is_some_and(|token| etb_token_word_is(token, ETB_THIS_WORD))
    {
        return Ok(None);
    }

    let Some(entry_clause) = parse_entry_filter_clause(tokens) else {
        return Ok(None);
    };
    if !ETB_UNTAPPED_MARKER_PATTERN.matches(entry_clause.tail_clause) {
        return Ok(None);
    }

    let before_enter = entry_clause.filter_tokens;
    if before_enter.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(before_enter, false)?;
    Ok(Some(StaticAbility::enters_untapped_for_filter(filter)))
}

fn parse_reveal_from_hand_filter_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const FROM_YOUR_HAND_PHRASE: &[&str] = &["from", "your", "hand"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("reveal"),
        LexPattern::object(
            "reveal_filter",
            LexCaptureKind::UntilPhrase(FROM_YOUR_HAND_PHRASE),
        ),
        LexPattern::phrase(FROM_YOUR_HAND_PHRASE),
    ]);

    let clause = LexedClause::new(tokens);
    PATTERN
        .find_in_clause(clause)?
        .capture_clause_by_role(LexCaptureRole::Object, clause)
        .map(LexedClause::trimmed)
}

pub(crate) fn parse_reveal_from_hand_or_enters_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_AS_THIS_LAND_ENTERS_PREFIX_PATTERN.matches(clause) {
        return Ok(None);
    }
    if !ETB_REVEAL_FROM_HAND_MARKER_PATTERN.matches(clause) {
        return Ok(None);
    }

    let Some(reveal_filter_clause) = parse_reveal_from_hand_filter_clause(tokens) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal source in land ETB reveal clause (clause: '{}')",
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
    let mut reveal_filter = parse_object_filter(&reveal_filter_tokens, false)?;
    reveal_filter.zone = None;
    let reveal_condition = crate::ConditionExpr::YouHaveCardInHandMatching(reveal_filter);

    // Pattern A: "... If you don't, this land enters tapped."
    if let Some(if_you_dont_idx) =
        etb_find_prefix_shape_start(clause, &ETB_IF_YOU_DONT_PREFIX_PATTERN)
    {
        if !clause
            .after_words(if_you_dont_idx + 3)
            .is_some_and(|trailing| ETB_LAND_REVEAL_TRAILING_TAPPED_PATTERN.matches(trailing))
        {
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
    let condition_clause = parse_enters_tapped_unless_condition_clause(tokens);
    if condition_clause.is_none() {
        if ETB_UNLESS_MARKER_PATTERN.matches(clause) {
            return Err(CardTextError::ParseError(format!(
                "unsupported land ETB reveal unless-prefix (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }

    let mut condition = reveal_condition;
    if let Some(condition_clause) = condition_clause
        && ETB_OR_MARKER_PATTERN.matches(condition_clause)
    {
        let Some(parsed_condition) =
            parse_revealed_this_way_or_control_condition(condition_clause.tokens())
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported control condition in land ETB reveal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        condition = parsed_condition;
    }

    parser_trace("parse_static:land-reveal-or-enter-tapped:matched", tokens);
    Ok(Some(StaticAbility::enters_tapped_unless_condition(
        condition,
        clause_words.join(" "),
    )))
}

fn parse_revealed_this_way_or_control_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    const THIS_WAY_OR_PHRASE: &[&str] = &["this", "way", "or"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::phrase(&["you", "revealed"]),
        LexPattern::object(
            "reveal_filter",
            LexCaptureKind::UntilPhrase(THIS_WAY_OR_PHRASE),
        ),
        LexPattern::phrase(THIS_WAY_OR_PHRASE),
        LexPattern::role_capture(
            "control_condition",
            LexCaptureRole::Condition,
            LexCaptureKind::Rest,
        ),
    ]);

    let condition_clause = LexedClause::new(condition_tokens);
    let matched = PATTERN.match_clause(condition_clause)?;
    let reveal_filter_clause =
        matched.capture_clause_by_role(LexCaptureRole::Object, condition_clause)?;
    let reveal_filter_tokens = trim_edge_punctuation(reveal_filter_clause.tokens());
    if reveal_filter_tokens.is_empty() {
        return None;
    }
    let mut reveal_filter = parse_object_filter(&reveal_filter_tokens, false).ok()?;
    reveal_filter.zone = None;

    let control_clause =
        matched.capture_clause_by_role(LexCaptureRole::Condition, condition_clause)?;
    let control_tokens = trim_edge_punctuation(control_clause.tokens());
    let control_condition = crate::runtime_backend::grammar::conditions::parse_control_condition(
        &control_tokens,
        crate::runtime_backend::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: false,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;
    if control_condition.player_filter != Some(PlayerFilter::You)
        || control_condition
            .at_least_count()
            .map_or(true, |count| count > 1)
    {
        return None;
    }

    Some(crate::ConditionExpr::Or(
        Box::new(crate::ConditionExpr::YouHaveCardInHandMatching(
            reveal_filter,
        )),
        Box::new(crate::ConditionExpr::YouControl(control_condition.filter)),
    ))
}

fn captured_enters_tapped_unless_control_quantity_static_ability(
    control_condition: &crate::runtime_backend::grammar::conditions::ControlConditionAst,
) -> Option<StaticAbility> {
    let mut filter = control_condition.filter.clone();
    filter.zone = None;

    let normalize_template = |mut filter: ObjectFilter| {
        filter.zone = None;
        filter
    };
    let other_lands = normalize_template(
        ObjectFilter::land()
            .controlled_by(PlayerFilter::You)
            .other(),
    );
    let basic_lands = normalize_template(
        ObjectFilter::land()
            .controlled_by(PlayerFilter::You)
            .with_supertype(Supertype::Basic),
    );

    match (control_condition.comparison, filter) {
        (crate::effect::Comparison::GreaterThanOrEqual(2), filter) if filter == other_lands => {
            Some(StaticAbility::enters_tapped_unless_control_two_or_more_other_lands())
        }
        (crate::effect::Comparison::LessThanOrEqual(2), filter) if filter == other_lands => {
            Some(StaticAbility::enters_tapped_unless_control_two_or_fewer_other_lands())
        }
        (crate::effect::Comparison::GreaterThanOrEqual(2), filter) if filter == basic_lands => {
            Some(StaticAbility::enters_tapped_unless_control_two_or_more_basic_lands())
        }
        _ => None,
    }
}

fn parse_enters_tapped_unless_control_quantity_static_ability(
    condition_tokens: &[OwnedLexToken],
    display: String,
) -> Option<StaticAbility> {
    let condition_words = crate::runtime_backend::lexer::token_word_refs(condition_tokens);
    let control_condition = crate::runtime_backend::grammar::conditions::parse_control_condition(
        condition_tokens,
        crate::runtime_backend::grammar::conditions::ControlConditionOptions {
            allow_that_player: false,
            allow_opponent_players: false,
            allow_defending_player: false,
            bind_filter_controller_to_subject: true,
            allow_different_powers_tail: false,
            default_filter_zone: Some(Zone::Battlefield),
        },
    )?;
    if !control_condition.has_explicit_quantity() {
        return None;
    }
    if let Some(ability) =
        captured_enters_tapped_unless_control_quantity_static_ability(&control_condition)
    {
        return Some(ability);
    }

    let mut filter = control_condition.filter;
    if filter.zone.is_none() {
        filter.zone = Some(Zone::Battlefield);
    }
    let condition = crate::ConditionExpr::CountComparison {
        count: AnthemCountExpression::MatchingFilter(filter),
        comparison: control_condition.comparison,
        display: Some(condition_words.join(" ")),
    };
    Some(StaticAbility::enters_tapped_unless_condition(
        condition, display,
    ))
}

#[cfg(test)]
mod etb_enters_tapped_with_counters_tests {
    use super::*;

    #[test]
    fn enters_tapped_with_counters_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "this creature enters tapped with one +1/+1 counter on it.",
            0,
        )
        .expect("lex");

        let captured = parse_enters_tapped_with_counters_clause_tokens(&tokens)
            .expect("capture parser should recognize tapped-with-counters clause");
        assert_eq!(
            LexedClause::new(captured.subject_tokens).word_refs(),
            ["this", "creature"]
        );
        assert!(
            LexedClause::new(captured.entry_modifier_tokens)
                .word_refs()
                .contains(&"tapped")
        );

        let abilities = parse_enters_tapped_with_counters_line(&tokens)
            .expect("parser should not error")
            .expect("enters tapped with counters should parse");
        let ids = abilities.iter().map(StaticAbility::id).collect::<Vec<_>>();

        assert_eq!(abilities.len(), 2, "expected tapped plus counters");
        assert!(ids.contains(&crate::static_abilities::StaticAbilityId::EntersTapped));
        assert!(
            ids.contains(&crate::static_abilities::StaticAbilityId::EnterWithCounters),
            "expected enters-with-counters ability, got {ids:?}"
        );
    }

    #[test]
    fn enters_with_counters_uses_capture_parser_for_normalized_subjects() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "This creature enters with one +1/+1 counter on it.",
            0,
        )
        .expect("lex");

        let captured = parse_enters_with_counters_clause_tokens(&tokens)
            .expect("capture parser should normalize source subject");
        assert_eq!(
            LexedClause::new(captured.subject_tokens).word_refs(),
            ["this", "creature"]
        );

        let ability = parse_enters_with_counters_line(&tokens)
            .expect("parser should not error")
            .expect("enters with counters should parse");
        assert_eq!(
            ability.id(),
            crate::static_abilities::StaticAbilityId::EnterWithCounters
        );
    }

    #[test]
    fn colors_of_mana_spent_condition_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "Two or more colors of mana were spent to cast it.",
            0,
        )
        .expect("lex");

        assert_eq!(
            parse_enters_with_counter_colors_mana_spent_condition_tokens(&tokens),
            Some(2)
        );
        assert_eq!(
            parse_unless_enters_with_counter_condition_display(&tokens),
            Some("fewer than 2 colors of mana were spent to cast it".to_string())
        );
        assert!(matches!(
            parse_enters_with_counter_condition_clause(&tokens),
            Some(crate::ConditionExpr::ColorsOfManaSpentToCastThisSpellOrMore(2))
        ));
    }

    #[test]
    fn you_cast_spells_this_turn_condition_uses_capture_parser() {
        for text in [
            "you've cast two or more spells this turn",
            "you have cast three or more spells this turn",
            "you cast four or more spells this turn",
        ] {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
            let expected = if text.contains("three") {
                3
            } else if text.contains("four") {
                4
            } else {
                2
            };

            assert_eq!(
                parse_enters_with_counter_you_cast_spells_this_turn_condition_tokens(&tokens),
                Some(expected),
                "{text}"
            );
            assert!(matches!(
                parse_enters_with_counter_condition_clause(&tokens),
                Some(crate::ConditionExpr::PlayerCastSpellsThisTurnOrMore {
                    player: PlayerFilter::You,
                    count
                }) if count == expected
            ));
        }
    }

    #[test]
    fn x_value_threshold_condition_uses_capture_parser() {
        for (text, expected) in [("X is 5 or more", 5), ("x is five or more", 5)] {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");

            assert_eq!(
                parse_enters_with_counter_x_value_threshold_condition_tokens(&tokens),
                Some(expected),
                "{text}"
            );
            assert!(matches!(
                parse_enters_with_counter_condition_clause(&tokens),
                Some(crate::ConditionExpr::XValueAtLeast(amount)) if amount == expected
            ));
        }
    }

    #[test]
    fn plus_for_each_counter_tail_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "plus an additional +1/+1 counter on it for each other creature you control",
            0,
        )
        .expect("lex");

        let value = parse_enters_with_counter_plus_for_each_tail_tokens(&tokens)
            .expect("tail parser should not error")
            .expect("plus for-each tail should parse");
        let debug = format!("{value:?}");
        assert!(
            debug.contains("other: true") && debug.contains("Creature"),
            "expected other-creature dynamic counter value, got {debug}"
        );
    }

    #[test]
    fn plus_counter_tail_gate_uses_capture_parser() {
        let supported_tokens = crate::runtime_backend::lexer::lex_line(
            "plus an additional +1/+1 counter on it for each other creature you control",
            0,
        )
        .expect("lex");
        let supported = parse_enters_with_counter_plus_tail_tokens(&supported_tokens)
            .expect("plus tail parser should not error")
            .expect("plus tail should be recognized");
        assert!(
            matches!(supported, EntersWithCounterPlusTail::Supported(_)),
            "expected supported plus-for-each tail, got {supported:?}"
        );

        let unsupported_tokens =
            crate::runtime_backend::lexer::lex_line("plus a mystery counter", 0).expect("lex");
        let unsupported = parse_enters_with_counter_plus_tail_tokens(&unsupported_tokens)
            .expect("unsupported plus tail should not hard-error")
            .expect("plus tail should be recognized");
        assert!(matches!(
            unsupported,
            EntersWithCounterPlusTail::Unsupported
        ));

        let unrelated_tokens =
            crate::runtime_backend::lexer::lex_line("for each creature you control", 0)
                .expect("lex");
        assert!(
            parse_enters_with_counter_plus_tail_tokens(&unrelated_tokens)
                .expect("unrelated tail should not error")
                .is_none()
        );
    }

    #[test]
    fn for_each_counter_tail_uses_capture_parser() {
        let tokens =
            crate::runtime_backend::lexer::lex_line("for each creature card in your graveyard", 0)
                .expect("lex");

        let value = parse_enters_with_counter_for_each_tail_tokens(&tokens)
            .expect("tail parser should not error")
            .expect("for-each tail should parse");
        let debug = format!("{value:?}");
        assert!(
            debug.contains("Graveyard") && debug.contains("Creature"),
            "expected creature-card-in-graveyard dynamic value, got {debug}"
        );
    }

    #[test]
    fn equal_to_counter_tail_uses_capture_parser() {
        let tokens = crate::runtime_backend::lexer::lex_line(
            "equal to the number of creature cards in your graveyard",
            0,
        )
        .expect("lex");

        let value = parse_enters_with_counter_equal_to_tail_tokens(&tokens)
            .expect("equal-to tail should parse");
        let debug = format!("{value:?}");
        assert!(
            debug.contains("Graveyard") && debug.contains("Creature"),
            "expected creature-card-in-graveyard equal-to value, got {debug}"
        );
    }

    #[test]
    fn equal_to_mana_spent_value_uses_capture_parser() {
        for text in [
            "equal to the amount of mana spent to cast it",
            "equal to the amount of mana spent to cast this spell",
            "equal to the amount of mana spent to cast spell",
        ] {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
            let value = parse_equal_to_mana_spent_to_cast_value(&tokens)
                .unwrap_or_else(|| panic!("mana-spent value should parse: {text}"));
            let debug = format!("{value:?}");
            assert!(
                debug.contains("ManaSpentToCastThisSpell") && debug.contains("EqualTo"),
                "expected equal-to mana-spent value for {text}, got {debug}"
            );
        }

        let unrelated_tokens = crate::runtime_backend::lexer::lex_line(
            "equal to the amount of mana spent to cast that permanent",
            0,
        )
        .expect("lex");
        assert!(parse_equal_to_mana_spent_to_cast_value(&unrelated_tokens).is_none());
    }

    #[test]
    fn known_for_each_counter_tails_use_capture_parser() {
        let cases = [
            (
                "for each creature that died this turn",
                "CreaturesDiedThisTurn",
                true,
            ),
            (
                "for each color of mana spent to cast it",
                "ColorsOfManaSpentToCastThisSpell",
                true,
            ),
            (
                "for each creature that died under your control this turn",
                "CreaturesDiedThisTurnControlledBy",
                true,
            ),
            ("for each time this spell was kicked", "KickCount", true),
            (
                "for each Magic game you have lost to one of your opponents since you last won a game against them",
                "MagicGamesLostToOpponentsSinceLastWin",
                false,
            ),
        ];

        for (text, expected_debug, expected_scaled) in cases {
            let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
            let parsed = parse_enters_with_counter_known_for_each_tail_tokens(&tokens)
                .unwrap_or_else(|| panic!("known for-each tail should parse: {text}"));
            let debug = format!("{:?}", parsed.value);
            assert!(
                debug.contains(expected_debug),
                "expected {expected_debug} value for {text}, got {debug}"
            );
            assert_eq!(
                parsed.scale_by_base_count, expected_scaled,
                "unexpected scaling flag for {text}"
            );
        }
    }

    #[test]
    fn counter_condition_tail_uses_capture_parser() {
        let if_tokens =
            crate::runtime_backend::lexer::lex_line("if you attacked this turn", 0).expect("lex");
        let if_tail = parse_enters_with_counter_condition_tail_tokens(&if_tokens)
            .expect("if condition tail should parse");
        assert_eq!(if_tail.kind, EntersWithCounterConditionTailKind::If);
        assert_eq!(
            LexedClause::new(if_tail.condition_tokens).word_refs(),
            ["you", "attacked", "this", "turn"]
        );

        let unless_tokens = crate::runtime_backend::lexer::lex_line(
            "unless two or more colors of mana were spent to cast it",
            0,
        )
        .expect("lex");
        let unless_tail = parse_enters_with_counter_condition_tail_tokens(&unless_tokens)
            .expect("unless condition tail should parse");
        assert_eq!(unless_tail.kind, EntersWithCounterConditionTailKind::Unless);
        assert_eq!(
            parse_unless_enters_with_counter_condition_display(unless_tail.condition_tokens),
            Some("fewer than 2 colors of mana were spent to cast it".to_string())
        );
    }
}

#[cfg(test)]
mod etb_control_quantity_tests {
    use super::*;

    fn parse_control_quantity_condition(text: &str) -> StaticAbility {
        let tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
        parse_enters_tapped_unless_control_quantity_static_ability(&tokens, text.to_string())
            .expect("control quantity condition should parse")
    }

    #[test]
    fn enters_tapped_unless_control_quantity_special_cases_use_capture_parser() {
        let cases = [
            (
                "you control two or more other lands",
                crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreOtherLands,
            ),
            (
                "you control two or fewer other lands",
                crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrFewerOtherLands,
            ),
            (
                "you control two or more basic lands",
                crate::static_abilities::StaticAbilityId::EntersTappedUnlessControlTwoOrMoreBasicLands,
            ),
        ];

        for (text, expected_id) in cases {
            let ability = parse_control_quantity_condition(text);
            assert_eq!(ability.id(), expected_id, "{text}");
        }
    }

    #[test]
    fn enters_tapped_unless_control_quantity_generic_case_keeps_count_condition() {
        let ability = parse_control_quantity_condition("you control three or more artifacts");
        let debug = format!("{ability:?}");

        assert!(
            debug.contains("EntersTappedUnlessCondition"),
            "expected generic conditional ETB ability, got {debug}"
        );
        assert!(debug.contains("CountComparison"), "{debug}");
        assert!(debug.contains("Artifact"), "{debug}");
        assert!(debug.contains("GreaterThanOrEqual(3)"), "{debug}");
    }

    #[test]
    fn reveal_unless_revealed_or_control_disjunction_uses_capture_parser() {
        let reveal_tokens = crate::runtime_backend::lexer::lex_line(
            "As this land enters, you may reveal a Dragon card from your hand.",
            0,
        )
        .expect("lex");
        assert!(
            parse_reveal_from_hand_or_enters_tapped_line(&reveal_tokens)
                .expect("standalone reveal clause should not hard-error")
                .is_none()
        );

        let tapped_tokens = crate::runtime_backend::lexer::lex_line(
            "This land enters tapped unless you revealed a Dragon card this way or you control a Dragon.",
            0,
        )
        .expect("lex");

        let ability = parse_conditional_enters_tapped_unless_line(&tapped_tokens)
            .expect("reveal-or-control clause should parse")
            .expect("expected static ability");
        let debug = format!("{ability:?}");

        assert!(debug.contains("YouHaveCardInHandMatching"), "{debug}");
        assert!(debug.contains("YouControl"), "{debug}");
        assert!(debug.contains("Dragon"), "{debug}");
    }

    #[test]
    fn enters_tapped_unless_opponents_condition_uses_capture_parser() {
        let condition_tokens =
            crate::runtime_backend::lexer::lex_line("you have two or more opponents", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_two_or_more_opponents_condition(&condition_tokens).is_some()
        );

        let wrong_amount_tokens =
            crate::runtime_backend::lexer::lex_line("you have three or more opponents", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_two_or_more_opponents_condition(&wrong_amount_tokens)
                .is_none()
        );

        let line_tokens = crate::runtime_backend::lexer::lex_line(
            "This land enters tapped unless you have two or more opponents.",
            0,
        )
        .expect("lex");
        let ability = parse_conditional_enters_tapped_unless_line(&line_tokens)
            .expect("opponents condition should parse")
            .expect("expected static ability");

        assert_eq!(
            ability.id(),
            crate::static_abilities::StaticAbilityId::EntersTappedUnlessTwoOrMoreOpponents
        );
    }

    #[test]
    fn enters_tapped_unless_life_condition_uses_capture_parser() {
        let condition_tokens =
            crate::runtime_backend::lexer::lex_line("a player has 13 or less life", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(&condition_tokens)
                .is_some()
        );

        let wrong_amount_tokens =
            crate::runtime_backend::lexer::lex_line("a player has 12 or less life", 0)
                .expect("lex");
        assert!(
            parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(&wrong_amount_tokens)
                .is_none()
        );

        let line_tokens = crate::runtime_backend::lexer::lex_line(
            "This land enters tapped unless a player has 13 or less life.",
            0,
        )
        .expect("lex");
        let ability = parse_conditional_enters_tapped_unless_line(&line_tokens)
            .expect("life condition should parse")
            .expect("expected static ability");

        assert_eq!(
            ability.id(),
            crate::static_abilities::StaticAbilityId::EntersTappedUnlessAPlayerHas13OrLessLife
        );
    }
}

fn parse_enters_tapped_unless_a_player_has_13_or_less_life_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<()> {
    let condition = crate::runtime_backend::grammar::conditions::parse_player_life_total_condition(
        condition_tokens,
    )?;
    if condition.player != PlayerFilter::Any {
        return None;
    }
    match condition.comparison {
        crate::effect::Comparison::LessThanOrEqual(13)
        | crate::effect::Comparison::LessThan(14) => Some(()),
        _ => None,
    }
}

fn parse_enters_tapped_unless_two_or_more_opponents_condition(
    condition_tokens: &[OwnedLexToken],
) -> Option<()> {
    let opponent_phrases: &[&[&str]] = &[&["opponents"]];
    let condition =
        crate::runtime_backend::grammar::conditions::parse_player_has_quantity_object_condition(
            condition_tokens,
            opponent_phrases,
            "enters-tapped opponents condition",
        )?;
    if condition.player != PlayerFilter::You {
        return None;
    }
    let count = crate::runtime_backend::util::comparison_to_strict_at_least_threshold(
        &condition.comparison,
    )?;
    if count == 2 { Some(()) } else { None }
}

fn parse_enters_tapped_unless_condition_clause<'a>(
    tokens: &'a [OwnedLexToken],
) -> Option<LexedClause<'a>> {
    const UNLESS_PHRASE: &[&str] = &["unless"];
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::modifier("entry_prefix", LexCaptureKind::UntilPhrase(UNLESS_PHRASE)),
        LexPattern::phrase(UNLESS_PHRASE),
        LexPattern::role_capture("condition", LexCaptureRole::Condition, LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let entry_prefix = matched.capture_clause_by_role(LexCaptureRole::Modifier, clause)?;
    if !ETB_ENTERS_TAPPED_PHRASE_PATTERN.matches(entry_prefix) {
        return None;
    }
    let condition_clause = matched
        .capture_clause_by_role(LexCaptureRole::Condition, clause)?
        .trimmed();
    (!condition_clause.is_empty()).then_some(condition_clause)
}

pub(crate) fn parse_conditional_enters_tapped_unless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    if !ETB_ENTER_OR_ENTERS_MARKER_PATTERN.matches(clause) {
        return Ok(None);
    }
    if !ETB_TAPPED_MARKER_PATTERN.matches(clause) || !ETB_UNLESS_MARKER_PATTERN.matches(clause) {
        return Ok(None);
    }

    let Some(condition_clause) = parse_enters_tapped_unless_condition_clause(tokens) else {
        return Ok(None);
    };
    let condition_tokens = trim_edge_punctuation(condition_clause.tokens());
    let condition_shape_clause = LexedClause::new(&condition_tokens);
    if let Some(condition) = parse_revealed_this_way_or_control_condition(&condition_tokens) {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            condition,
            clause_words.join(" "),
        )));
    }
    if let Some(ability) = parse_enters_tapped_unless_control_quantity_static_ability(
        &condition_tokens,
        clause_words.join(" "),
    ) {
        return Ok(Some(ability));
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
    if ETB_FIRST_THREE_TURNS_PATTERN.matches(condition_shape_clause) {
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            crate::ConditionExpr::YourFirstTurnsOfTheGameOrFewer(3),
            clause_words.join(" "),
        )));
    }

    // Generic: "unless you control <object filter>" (covers Mount/Vehicle, etc.).
    if let Some(control_condition) =
        crate::runtime_backend::grammar::conditions::parse_control_condition(
            &condition_tokens,
            crate::runtime_backend::grammar::conditions::ControlConditionOptions {
                allow_that_player: false,
                allow_opponent_players: false,
                allow_defending_player: false,
                bind_filter_controller_to_subject: false,
                allow_different_powers_tail: false,
                default_filter_zone: None,
            },
        )
        && !control_condition.has_explicit_quantity()
    {
        let condition = crate::ConditionExpr::YouControl(control_condition.filter);
        return Ok(Some(StaticAbility::enters_tapped_unless_condition(
            condition,
            clause_words.join(" "),
        )));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported enters tapped unless condition (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub(crate) fn parse_enters_with_additional_counter_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let clause_word_len = clause.word_len();
    if clause_word_len > 9
        && ETB_AS_LONG_AS_THIS_IN_YOUR_GRAVEYARD_PATTERN.matches(clause)
        && let Some(comma_idx) = find_token_kind(tokens, TokenKind::Comma)
    {
        return parse_enters_with_additional_counter_for_filter_line(&tokens[comma_idx + 1..]);
    }

    if clause_word_len > 6
        && ETB_AS_LONG_AS_PREFIX_PATTERN.matches(clause)
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

    let Some(entry_clause) = parse_entry_filter_clause(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(entry_clause.filter_tokens);
    if subject_tokens
        .iter()
        .any(|token| token.is_period() || token.is_colon() || token.is_semicolon())
    {
        return Ok(None);
    }

    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if is_etb_source_reference_clause(LexedClause::new(&subject_tokens)) {
        return Ok(None);
    }
    if ETB_TRIGGER_INTRO_AFTER_LABEL_PATTERN.matches(LexedClause::new(&subject_tokens)) {
        return Ok(None);
    }

    if !ETB_WITH_ADDITIONAL_COUNTERS_PATTERN.matches(clause) {
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
            etb_token_word_is(token, ETB_ADDITIONAL_WORD)
        })
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let count = if let Some(equal_idx) =
        crate::runtime_backend::grammar::primitives::find_token_index(base_tokens, |token| {
            etb_token_word_is(token, ETB_EQUAL_WORD)
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

#[derive(Debug, Clone, Copy)]
struct AsEntersClause<'a> {
    subject_tokens: &'a [OwnedLexToken],
    tail_clause: LexedClause<'a>,
}

fn parse_as_enters_clause<'a>(tokens: &'a [OwnedLexToken]) -> Option<AsEntersClause<'a>> {
    const PATTERN: LexPattern<'static> = LexPattern::new(&[
        LexPattern::word("as"),
        LexPattern::subject(
            "subject",
            LexCaptureKind::UntilAnyPhrase(ETB_ENTER_OR_ENTERS_PHRASES),
        ),
        LexPattern::action("entry_action", LexCaptureKind::OneOf(&["enter", "enters"])),
        LexPattern::tail("entry_tail", LexCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = PATTERN.match_clause(clause)?;
    let subject_clause = matched
        .capture_clause_by_role(LexCaptureRole::Subject, clause)?
        .trimmed();
    let tail_clause = matched
        .capture_clause_by_role(LexCaptureRole::Tail, clause)?
        .trimmed();
    if subject_clause.is_empty() || tail_clause.is_empty() {
        return None;
    }

    Some(AsEntersClause {
        subject_tokens: subject_clause.tokens(),
        tail_clause,
    })
}

pub(crate) fn parse_as_enters_becomes_characteristics_for_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(as_enters) = parse_as_enters_clause(tokens) else {
        return Ok(None);
    };

    let after_enter_words = as_enters.tail_clause.word_refs();
    let after_enter = after_enter_words.as_slice();
    if !ETB_IT_BECOMES_PREFIX_PATTERN.matches(as_enters.tail_clause) {
        return Ok(None);
    }

    let mut descriptor_idx = 2usize;
    if after_enter
        .get(descriptor_idx)
        .is_some_and(|word| etb_word_is_any(word, ETB_ARTICLE_WORDS))
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

    if !ETB_IN_ADDITION_TO_ITS_OTHER_TYPE_PATTERN.matches(as_enters.tail_clause) {
        return Ok(None);
    }
    let Some(addition_idx) = etb_find_prefix_shape_start(
        as_enters.tail_clause,
        &ETB_IN_ADDITION_TO_ITS_OTHER_PREFIX_PATTERN,
    ) else {
        return Ok(None);
    };
    if addition_idx <= descriptor_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(as_enters.subject_tokens);
    let filter = parse_object_filter(&subject_tokens, false)?;

    let descriptor_words = &after_enter[descriptor_idx..addition_idx];
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words.iter().copied().filter(|word| {
        !etb_word_is_any(word, ETB_ARTICLE_WORDS) && !etb_word_is(word, ETB_AND_WORD)
    }) {
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
    let Some(as_enters) = parse_as_enters_clause(tokens) else {
        return Ok(None);
    };

    let subject_clause = LexedClause::new(as_enters.subject_tokens);
    if !ETB_SELF_SUBJECT_PATTERN.matches(subject_clause) {
        return Ok(None);
    }

    let after_enter_words = as_enters.tail_clause.word_refs();
    let after_enter = after_enter_words.as_slice();
    if ETB_IT_BECOMES_YOUR_CHOICE_OF_PREFIX_PATTERN.matches(as_enters.tail_clause) {
        let options = parse_pt_choice_characteristic_options(&after_enter[5..], &clause_words)?;
        if options.is_empty() {
            return Ok(None);
        }
        let subject = subject_clause.text();
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
        || !ETB_FACE_UP_CHOICE_TAIL_PATTERN.matches(as_enters.tail_clause)
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

    let subject = subject_clause.text();
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

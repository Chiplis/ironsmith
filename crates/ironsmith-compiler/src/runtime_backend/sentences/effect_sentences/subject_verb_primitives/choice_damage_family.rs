use super::*;
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};
use crate::runtime_backend::lexer::word_slice_contains_any_phrase;

const REVEAL_HAND_SUFFIXES: &[&[&str]] = &[
    &["in", "your", "hand"],
    &["in", "your", "hands"],
    &["from", "your", "hand"],
    &["from", "your", "hands"],
];
pub(crate) const REVEAL_SELECTED_CARDS_IN_YOUR_HAND_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("reveal"),
    LexPattern::role_capture(
        "descriptor",
        LexCaptureRole::Object,
        LexCaptureKind::UntilAnyPhrase(REVEAL_HAND_SUFFIXES),
    ),
    LexPattern::any_phrase(REVEAL_HAND_SUFFIXES),
];
const REVEAL_VERB_PHRASES: &[&[&str]] = &[&["reveal"], &["reveals"]];
const REVEAL_ARTICLE_WORDS: &[&str] = &["a", "an", "one"];
const TO_PREFIX: &[&str] = &["to"];
const UP_TO_ONE_TARGET_WORDS: &[&str] = &["up", "to", "one", "target"];
const CHOICE_DAMAGE_ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const CHOICE_DAMAGE_UNLESS_WORDS: &[&str] = &["unless"];
const CHOICE_DAMAGE_IF_UNLESS_WORDS: &[&str] = &["if", "unless"];
const CHOICE_DAMAGE_CONDITION_BOUNDARY_WORDS: &[&str] =
    &["if", "unless", "then", "where", "when", "whenever"];
const OF_WORD: &str = "of";
const TO_WORD: &str = "to";
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const HAND_REFERENCE_PHRASES: &[&[&str]] = &[
    &["their", "hand"],
    &["their", "hands"],
    &["your", "hand"],
    &["your", "hands"],
    &["that", "player", "hand"],
    &["that", "player", "hands"],
    &["target", "player", "hand"],
    &["target", "player", "hands"],
];
const DAMAGE_WORD: &str = "damage";
const DESTROY_WORD: &str = "destroy";
const AT_RANDOM_PHRASE: &[&str] = &["at", "random"];
const YOU_GAIN_X_LIFE_PHRASE: &[&str] = &["you", "gain", "x", "life"];
const LOSE_X_LIFE_PHRASES: &[&[&str]] = &[&["lose", "x", "life"], &["loses", "x", "life"]];
const CARD_WORD: &str = "card";
const TOKEN_WORDS: &[&str] = &["token"];
const SACRIFICE_WORDS: &[&str] = &["sacrifice"];
const COUNTER_WORDS: &[&str] = &["counter"];
const CREATE_WORDS: &[&str] = &["create"];
const ALTERNATE_DAMAGE_TARGET_PHRASES: &[&[&str]] = &[&["them"], &["that", "player"]];
const THEM_OR_THAT_PLAYER_PHRASES: &[&[&str]] = ALTERNATE_DAMAGE_TARGET_PHRASES;
const THAT_PLAYER_WORDS: &[&str] = &["that", "player"];
const TARGET_WORD: &str = "target";
const EXILE_WORD: &str = "exile";
const COUNTER_WORD: &str = "counter";
const CHOOSE_WORD: &str = "choose";
const DAMAGE_SOURCE_SUBJECT_PHRASES: &[&[&str]] = &[
    &["this", "aura"],
    &["this", "permanent"],
    &["this", "enchantment"],
];
const HAS_OR_HAVE_PHRASES: &[&[&str]] = &[&["has"], &["have"]];
const CONTROLLER_WORDS: &[&str] = &["controller", "controllers"];
const CHOICE_DAMAGE_ALL_OR_EACH_HEAD_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "scope",
        LexCaptureKind::OneOf(CHOICE_DAMAGE_ALL_OR_EACH_WORDS),
    )]);
const CHOICE_DAMAGE_IF_UNLESS_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::condition(
        "condition",
        LexCaptureKind::OneOf(CHOICE_DAMAGE_IF_UNLESS_WORDS),
    )]);
const CHOICE_DAMAGE_UNLESS_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::condition(
        "unless",
        LexCaptureKind::OneOf(CHOICE_DAMAGE_UNLESS_WORDS),
    )]);
const CHOICE_DAMAGE_CONDITION_BOUNDARY_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::condition(
        "boundary",
        LexCaptureKind::OneOf(CHOICE_DAMAGE_CONDITION_BOUNDARY_WORDS),
    )]);
const THAT_CONTROLLER_SUBJECT_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("that"),
    LexPattern::subject(
        "controller_subject",
        LexCaptureKind::UntilAnyPhrase(HAS_OR_HAVE_PHRASES),
    ),
]);
const CONTROLLER_WORD_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::subject(
    "controller",
    LexCaptureKind::OneOf(CONTROLLER_WORDS),
)]);
const LOSE_X_LIFE_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "lose_life",
    LexCaptureKind::OneOfPhrase(LOSE_X_LIFE_PHRASES),
)]);
const CREATE_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "create",
    LexCaptureKind::OneOf(CREATE_WORDS),
)]);
const TOKEN_MARKER_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "token",
    LexCaptureKind::OneOf(TOKEN_WORDS),
)]);
const SACRIFICE_ACTION_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::action(
    "sacrifice",
    LexCaptureKind::OneOf(SACRIFICE_WORDS),
)]);
const COUNTER_MARKER_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "counter",
    LexCaptureKind::OneOf(COUNTER_WORDS),
)]);
const UP_TO_ONE_TARGET_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "target",
    LexCaptureKind::OneOfPhrase(&[UP_TO_ONE_TARGET_WORDS]),
)]);
const EACH_OPPONENT_SCOPE_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::subject(
    "scope",
    LexCaptureKind::OneOfPhrase(EACH_OPPONENT_PREFIXES),
)]);
const EACH_PLAYER_SCOPE_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::subject(
    "scope",
    LexCaptureKind::OneOfPhrase(EACH_PLAYER_PREFIXES),
)]);
const TO_PREFIX_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::modifier(
    "to",
    LexCaptureKind::OneOfPhrase(&[TO_PREFIX]),
)]);

fn choice_damage_clause_first_is(clause: SubjectVerbPrimitiveClause<'_>, expected: &str) -> bool {
    clause.first_word().is_some_and(|word| word == expected)
}


fn choice_damage_alternate_target_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let alt_target_words = clause.word_refs();
    word_slice_eq_any(&alt_target_words, THEM_OR_THAT_PLAYER_PHRASES)
}

fn choice_damage_source_subject_matches(subject_clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    word_slice_eq_any(&subject_clause.word_refs(), DAMAGE_SOURCE_SUBJECT_PHRASES)
}

fn choice_damage_that_player_target_matches(target_clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    word_slice_eq(&target_clause.word_refs(), THAT_PLAYER_WORDS)
}

fn choice_damage_find_pattern(
    clause: SubjectVerbPrimitiveClause<'_>,
    pattern: LexPattern<'static>,
    capture: &str,
) -> bool {
    let words = clause.word_refs();
    pattern
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range(capture))
        .is_some()
}

fn choice_damage_starts_with_pattern(
    clause: SubjectVerbPrimitiveClause<'_>,
    pattern: LexPattern<'static>,
    capture: &str,
) -> bool {
    let words = clause.word_refs();
    pattern
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range(capture))
        .is_some_and(|range| range.start == 0)
}

fn choice_damage_drain_clause_matches(drain_clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    choice_damage_find_pattern(drain_clause, LOSE_X_LIFE_PATTERN, "lose_life")
        && word_slice_contains_phrase(&drain_clause.word_refs(), YOU_GAIN_X_LIFE_PHRASE)
}

fn choice_damage_random_card_descriptor_matches(
    random_descriptor_clause: SubjectVerbPrimitiveClause<'_>,
) -> bool {
    let descriptor_words = random_descriptor_clause.word_refs();
    word_slice_contains_word(&descriptor_words, CARD_WORD)
        && word_slice_contains_phrase(&random_descriptor_clause.word_refs(), AT_RANDOM_PHRASE)
}

fn choice_damage_create_token_sacrifice_counter_clause_matches(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> bool {
    choice_damage_starts_with_pattern(clause, CREATE_ACTION_PATTERN, "create")
        && choice_damage_find_pattern(clause, TOKEN_MARKER_PATTERN, "token")
        && choice_damage_find_pattern(clause, SACRIFICE_ACTION_PATTERN, "sacrifice")
        && choice_damage_find_pattern(clause, COUNTER_MARKER_PATTERN, "counter")
}

fn choice_damage_up_to_one_target_window_matches(words: &[&str]) -> bool {
    UP_TO_ONE_TARGET_PATTERN
        .match_word_refs(words)
        .and_then(|matched| matched.capture_word_range("target"))
        .is_some()
}

fn choice_damage_card_noun_at(descriptor_words: &[&str], idx: usize) -> bool {
    word_slice_at_is_any(&descriptor_words, idx, CARD_OR_CARDS_WORDS)
}

fn choice_damage_starts_with_scope(
    clause: SubjectVerbPrimitiveClause<'_>,
    pattern: LexPattern<'static>,
) -> bool {
    clause
        .match_prefix_pattern(pattern)
        .and_then(|matched| matched.capture_word_range("scope"))
        .is_some()
}

fn choice_damage_each_scope_kind(clause: SubjectVerbPrimitiveClause<'_>) -> Option<&'static str> {
    if choice_damage_starts_with_scope(clause, EACH_OPPONENT_SCOPE_PATTERN) {
        Some("opponent")
    } else if choice_damage_starts_with_scope(clause, EACH_PLAYER_SCOPE_PATTERN) {
        Some("player")
    } else {
        None
    }
}

fn choice_damage_strip_to_prefix(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> SubjectVerbPrimitiveClause<'_> {
    let Some(prefix_len) = clause
        .match_prefix_pattern(TO_PREFIX_PATTERN)
        .and_then(|matched| matched.capture_word_range("to"))
        .filter(|range| range.start == 0)
        .map(|range| range.end - range.start)
    else {
        return clause;
    };
    clause.from(prefix_len).trimmed()
}

fn choice_damage_all_or_each_after_action(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    CHOICE_DAMAGE_ALL_OR_EACH_HEAD_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("scope"))
        .is_some_and(|range| range.start == 1)
}

fn choice_damage_all_or_each_starts_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    CHOICE_DAMAGE_ALL_OR_EACH_HEAD_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("scope"))
        .is_some_and(|range| range.start == 0)
}

fn choice_damage_mentions_condition_boundary(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    CHOICE_DAMAGE_CONDITION_BOUNDARY_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("boundary"))
        .is_some()
}

fn choice_damage_mentions_if_unless(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    CHOICE_DAMAGE_IF_UNLESS_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("condition"))
        .is_some()
}

fn choice_damage_mentions_unless(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    CHOICE_DAMAGE_UNLESS_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("unless"))
        .is_some()
}

fn choice_damage_that_controller_subject_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let Some(matched) = clause.match_pattern(THAT_CONTROLLER_SUBJECT_PATTERN) else {
        return false;
    };
    let Some(subject_range) = matched.capture_word_range("controller_subject") else {
        return false;
    };
    let subject_words = clause
        .between(subject_range.start, subject_range.end)
        .word_refs();
    CONTROLLER_WORD_PATTERN
        .find_in_word_refs(&subject_words)
        .and_then(|matched| matched.capture_word_range("controller"))
        .is_some()
}

fn is_explicit_target_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    choice_damage_clause_first_is(clause, TARGET_WORD)
        || parse_choice_count_before_target_prefix(clause.tokens()).is_some()
}

pub(crate) const TARGET_PLAYER_REVEALS_RANDOM_CARD_FROM_HAND_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(REVEAL_VERB_PHRASES),
    ),
    LexPattern::any_phrase(REVEAL_VERB_PHRASES),
    LexPattern::any_word(REVEAL_ARTICLE_WORDS),
    LexPattern::role_capture(
        "descriptor",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["from"]),
    ),
    LexPattern::word("from"),
    LexPattern::role_capture("hand", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const DAMAGE_UNLESS_CONTROLLER_HAS_SOURCE_DEAL_DAMAGE_PATTERN_ATOMS:
    &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "damage",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["unless"]),
    ),
    LexPattern::word("unless"),
    LexPattern::role_capture(
        "unless",
        LexCaptureRole::Condition,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const ENCHANTED_ATTACKED_THIS_TURN_PHRASES: &[&[&str]] = &[
    &["that", "creature", "attacked", "this", "turn"],
    &["enchanted", "creature", "attacked", "this", "turn"],
];
const ENCHANTED_ATTACKED_UNLESS_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    &[LexPattern::phrase(ENCHANTED_ATTACKED_THIS_TURN_PHRASES[0])],
    &[LexPattern::phrase(ENCHANTED_ATTACKED_THIS_TURN_PHRASES[1])],
];
pub(crate) const DAMAGE_TO_THAT_PLAYER_UNLESS_ENCHANTED_ATTACKED_PATTERN_ATOMS:
    &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "damage",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["unless"]),
    ),
    LexPattern::word("unless"),
    LexPattern::any_sequence(ENCHANTED_ATTACKED_UNLESS_SEQUENCES),
];
const LEADING_UNLESS_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::word("unless"),
    LexPattern::role_capture(
        "unless",
        LexCaptureRole::Condition,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const TRAILING_UNLESS_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "effect",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["unless"]),
    ),
    LexPattern::word("unless"),
    LexPattern::role_capture(
        "unless",
        LexCaptureRole::Condition,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const UNLESS_PAYS_SEQUENCES: &[&[LexPatternAtom<'static>]] =
    &[LEADING_UNLESS_SEQUENCE, TRAILING_UNLESS_SEQUENCE];
pub(crate) const UNLESS_PAYS_PATTERN_ATOMS: &[LexPatternAtom<'static>] =
    &[LexPattern::any_sequence(UNLESS_PAYS_SEQUENCES)];

pub(crate) fn parse_sentence_each_opponent_loses_x_and_you_gain_x(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let atoms = [
        LexPattern::any_phrase(EACH_OPPONENT_PREFIXES),
        LexPattern::role_capture(
            "drain",
            LexCaptureRole::Action,
            LexCaptureKind::UntilPhrase(&["where", "x", "is"]),
        ),
        LexPattern::phrase(&["where", "x", "is"]),
        LexPattern::role_capture("where_value", LexCaptureRole::Amount, LexCaptureKind::Rest),
    ];
    let pattern = LexPattern::new(&atoms);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_each_opponent_loses_x_and_you_gain_x_matched(clause, &matched)
}

pub(crate) fn parse_sentence_each_opponent_loses_x_and_you_gain_x_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(drain_clause) = matched
        .capture_by_role(LexCaptureRole::Action)
        .and_then(|_| clause.pattern_capture_role(&matched, LexCaptureRole::Action))
    else {
        return Ok(None);
    };
    if !choice_damage_drain_clause_matches(drain_clause) {
        return Ok(None);
    }

    let Some(where_value_start) = matched
        .capture_by_role(LexCaptureRole::Amount)
        .map(|capture| capture.word_range.start)
    else {
        return Ok(None);
    };
    let Some(where_clause) = clause.from_word(where_value_start.saturating_sub(3)) else {
        return Ok(None);
    };
    let where_value = parse_value_binding_clause(where_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-x value in opponent life-drain clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(vec![
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::Implicit,
                SubjectVerbActionAst::LoseLife {
                    amount: where_value.clone(),
                },
            )],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::GainLife {
                amount: where_value,
            },
        ),
    ]))
}

pub(crate) fn parse_sentence_same_name_target_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_same_name_target_fanout_sentence)
}

pub(crate) fn parse_sentence_shared_color_target_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shared_color_target_fanout_sentence)
}

pub(crate) fn parse_sentence_compound_damage_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_compound_damage_fanout_sentence)
}

pub(crate) fn parse_sentence_same_name_gets_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_same_name_gets_fanout_sentence)
}

pub(crate) fn parse_sentence_delayed_until_next_end_step(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_delayed_until_next_end_step_sentence)
}

pub(crate) fn parse_sentence_destroy_or_exile_all_split(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_destroy_or_exile_all_split_sentence)
}

pub(crate) fn parse_sentence_exile_up_to_one_each_target_type(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_exile_up_to_one_each_target_type_sentence)
}

pub(crate) fn parse_sentence_exile_multi_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !choice_damage_clause_first_is(clause, EXILE_WORD) || choice_damage_mentions_unless(clause) {
        return Ok(None);
    }

    let Some(and_idx) = clause.find_token_word_where("and", |idx, tail_clause| {
        idx > 0 && !tail_clause.is_empty() && is_explicit_target_clause(tail_clause)
    }) else {
        return Ok(None);
    };

    let first_clause = clause.between(1, and_idx).trimmed();
    let second_clause = clause.from(and_idx + 1).trimmed();
    if first_clause.is_empty() || second_clause.is_empty() {
        return Ok(None);
    }

    let first_words = first_clause.word_refs();
    let first_is_explicit_target = is_explicit_target_clause(first_clause);
    let second_is_explicit_target = is_explicit_target_clause(second_clause);

    let mut first_target =
        if !first_is_explicit_target && is_likely_named_or_source_reference_words(&first_words) {
            TargetAst::Source(first_clause.span())
        } else {
            match parse_target_phrase(first_clause.tokens()) {
                Ok(target) => target,
                Err(err) => return Err(err),
            }
        };
    let mut second_target = parse_target_phrase(second_clause.tokens())?;

    if first_is_explicit_target
        && second_is_explicit_target
        && let (Some((mut first_filter, first_count)), Some((mut second_filter, second_count))) = (
            object_target_with_count(&first_target),
            object_target_with_count(&second_target),
        )
        && first_filter.zone == Some(Zone::Graveyard)
        && second_filter.zone == Some(Zone::Graveyard)
    {
        if first_filter.controller.is_none() {
            first_filter.controller = Some(PlayerFilter::Any);
        }
        if second_filter.controller.is_none() {
            second_filter.controller = Some(PlayerFilter::Any);
        }
        let tag = helper_tag_for_tokens(clause.tokens(), "exiled");
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
        ]));
    }

    apply_exile_subject_hand_owner_context(&mut first_target, None);
    apply_exile_subject_hand_owner_context(&mut second_target, None);
    Ok(Some(vec![
        EffectAst::subject_verb_exile(first_target, false),
        EffectAst::subject_verb_exile(second_target, false),
    ]))
}

pub(crate) fn split_destroy_target_segments(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Vec<SubjectVerbPrimitiveClause<'_>> {
    let mut segments = Vec::new();
    for segment_clause in clause.trimmed_and_comma_segments() {
        let split_starts = segment_clause
            .tokens()
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| {
                let _ = token;
                if idx >= 3
                    && choice_damage_up_to_one_target_window_matches(
                        &segment_clause.between(idx - 3, idx + 1).word_refs(),
                    )
                {
                    Some(idx - 3)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        if split_starts.len() <= 1 {
            segments.push(segment_clause);
            continue;
        }

        for (idx, start) in split_starts.iter().enumerate() {
            let end = split_starts
                .get(idx + 1)
                .copied()
                .unwrap_or(segment_clause.len());
            let segment = segment_clause.between(*start, end).trimmed();
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
    }

    segments
}

pub(crate) fn parse_sentence_destroy_multi_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !choice_damage_clause_first_is(clause, DESTROY_WORD) {
        return Ok(None);
    }
    if choice_damage_all_or_each_after_action(clause) {
        return Ok(None);
    }
    if choice_damage_mentions_if_unless(clause) {
        return Ok(None);
    }

    let target_clause = clause.from(1).trimmed();
    if target_clause.is_empty() {
        return Ok(None);
    }

    let has_separator = target_clause.contains_comma_or_any_word(&["and"]);
    let mut repeated_up_to_one_targets = 0usize;
    let mut start = 0usize;
    while start + 4 <= target_clause.len() {
        let window = target_clause.between(start, start + 4);
        if token_slice_starts_with(window.tokens(), &["up", "to", "one", "target"]) {
            repeated_up_to_one_targets += 1;
        }
        start += 1;
    }
    let has_repeated_up_to_one_targets = repeated_up_to_one_targets >= 2;
    if !has_separator && !has_repeated_up_to_one_targets {
        return Ok(None);
    }

    let repeated_target_words = target_clause.count_word("target") > 1;
    let has_followup_tail = choice_damage_mentions_condition_boundary(target_clause);
    if !repeated_target_words
        && !has_followup_tail
        && let Ok(target) = parse_target_phrase(target_clause.tokens())
        && let Some((filter, _)) = object_target_with_count(&target)
        && (filter.type_or_subtype_union
            || filter.card_types.len() > 1
            || filter.subtypes.len() > 1
            || filter.any_of.len() > 1)
    {
        return Ok(Some(vec![EffectAst::subject_verb_destroy(target)]));
    }

    let segments = split_destroy_target_segments(target_clause);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment_clause in segments {
        let segment_words = segment_clause.word_refs();
        if segment_words.iter().any(|word| {
            matches!(
                *word,
                "then" | "if" | "unless" | "where" | "when" | "whenever"
            )
        }) {
            return Ok(None);
        }
        let is_explicit_target = segment_words
            .first()
            .is_some_and(|word| *word == TARGET_WORD)
            || parse_choice_count_before_target_prefix(segment_clause.tokens()).is_some();
        if !is_explicit_target && !is_likely_named_or_source_reference_words(&segment_words) {
            return Ok(None);
        }
        let target = match parse_target_phrase(segment_clause.tokens()) {
            Ok(target) => target,
            Err(_)
                if !is_explicit_target
                    && is_likely_named_or_source_reference_words(&segment_words) =>
            {
                TargetAst::Source(segment_clause.span())
            }
            Err(err) => return Err(err),
        };
        effects.push(EffectAst::subject_verb_destroy(target));
    }

    if effects.len() < 2 {
        return Ok(None);
    }
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_reveal_selected_cards_in_your_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(REVEAL_SELECTED_CARDS_IN_YOUR_HAND_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_reveal_selected_cards_in_your_hand_matched(clause, &matched)
}

pub(crate) fn parse_sentence_reveal_selected_cards_in_your_hand_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.first() != Some(&"reveal") {
        return Ok(None);
    }
    if clause_words.iter().any(|word| {
        matches!(
            *word,
            "then" | "if" | "unless" | "where" | "when" | "whenever"
        )
    }) {
        return Ok(None);
    }

    let Some(mut descriptor_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object)
    else {
        return Ok(None);
    };
    descriptor_clause = descriptor_clause.trimmed();
    if descriptor_clause.is_empty() {
        return Ok(None);
    }

    let mut count = ChoiceCount::exactly(1);
    if let Some((parsed_count, used)) =
        crate::runtime_backend::util::parse_choice_count_token_prefix_consumed(
            descriptor_clause.tokens(),
        )
    {
        count = if parsed_count.dynamic_x {
            ChoiceCount::any_number()
        } else {
            parsed_count
        };
        descriptor_clause = descriptor_clause.from(used).trimmed();
        if choice_damage_clause_first_is(descriptor_clause, OF_WORD) {
            descriptor_clause = descriptor_clause.from(1).trimmed();
        }
    } else if descriptor_clause
        .first_word()
        .is_some_and(|word| REVEAL_ARTICLE_WORDS.contains(&word))
    {
        descriptor_clause = descriptor_clause.from(1).trimmed();
    } else if choice_damage_all_or_each_starts_clause(descriptor_clause) {
        return Ok(None);
    }

    if descriptor_clause.is_empty() {
        return Ok(None);
    }

    let mut filter = match parse_object_filter(descriptor_clause.tokens(), false) {
        Ok(filter) => filter,
        Err(_) => {
            let descriptor_words = descriptor_clause.word_refs();
            let mut filter = ObjectFilter::default();
            let mut idx = 0usize;
            if let Some(color) = descriptor_words.get(idx).and_then(|word| parse_color(word)) {
                filter.colors = Some(color.into());
                idx += 1;
            }
            if !choice_damage_card_noun_at(&descriptor_words, idx) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported reveal-hand clause (clause: '{}')",
                    clause_text
                )));
            }
            filter
        }
    };
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::You);

    let tag = helper_tag_for_tokens(clause.tokens(), "revealed");
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_reveal_tagged(tag),
    ]))
}

pub(crate) fn parse_sentence_target_player_reveals_random_card_from_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(TARGET_PLAYER_REVEALS_RANDOM_CARD_FROM_HAND_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_target_player_reveals_random_card_from_hand_matched(clause, &matched)
}

pub(crate) fn parse_sentence_target_player_reveals_random_card_from_hand_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(subject_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Subject) else {
        return Ok(None);
    };
    if subject_clause.is_empty() {
        return Ok(None);
    }

    let subject_tokens = subject_clause.trim();
    let SubjectAst::Player(player) = parse_subject(&subject_tokens) else {
        return Ok(None);
    };
    if !matches!(
        player,
        PlayerAst::You
            | PlayerAst::Target
            | PlayerAst::TargetOpponent
            | PlayerAst::Opponent
            | PlayerAst::That
    ) {
        return Ok(None);
    }

    let Some(descriptor_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object)
    else {
        return Ok(None);
    };
    if descriptor_clause.is_empty()
        || !choice_damage_random_card_descriptor_matches(descriptor_clause)
    {
        return Ok(None);
    }

    let Some(hand_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if !is_hand_reference_clause(hand_clause) {
        return Ok(None);
    }

    let filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(match player {
            PlayerAst::You => PlayerFilter::You,
            PlayerAst::Target => PlayerFilter::target_player(),
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::Opponent => PlayerFilter::Opponent,
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => return Ok(None),
        }),
        ..ObjectFilter::default()
    };
    let tag = helper_tag_for_tokens(clause.tokens(), "revealed");

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1).at_random(),
            count_value: None,
            player,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_reveal_tagged(tag),
    ]))
}

fn is_hand_reference_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let hand_words = clause.word_refs();
    word_slice_eq_any(&hand_words, HAND_REFERENCE_PHRASES)
}

pub(crate) fn object_target_with_count(target: &TargetAst) -> Option<(ObjectFilter, ChoiceCount)> {
    match target {
        TargetAst::Object(filter, _, _) => Some((filter.clone(), ChoiceCount::exactly(1))),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => Some((filter.clone(), count.clone())),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn is_likely_named_or_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if is_source_reference_words(words) {
        return true;
    }
    if words.iter().any(|word| {
        matches!(
            *word,
            "then"
                | "if"
                | "unless"
                | "where"
                | "when"
                | "whenever"
                | "for"
                | "each"
                | "search"
                | "destroy"
                | "exile"
                | "draw"
                | "gain"
                | "lose"
                | "counter"
                | "put"
                | "return"
                | "create"
                | "sacrifice"
                | "deal"
                | "populate"
        )
    }) {
        return false;
    }
    !words.iter().any(|word| {
        matches!(
            *word,
            "a" | "an"
                | "the"
                | "this"
                | "that"
                | "those"
                | "it"
                | "them"
                | "target"
                | "all"
                | "any"
                | "each"
                | "another"
                | "other"
                | "up"
                | "to"
                | "card"
                | "cards"
                | "creature"
                | "creatures"
                | "permanent"
                | "permanents"
                | "artifact"
                | "artifacts"
                | "enchantment"
                | "enchantments"
                | "land"
                | "lands"
                | "planeswalker"
                | "planeswalkers"
                | "spell"
                | "spells"
        )
    })
}

pub(crate) fn parse_sentence_damage_unless_controller_has_source_deal_damage(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(DAMAGE_UNLESS_CONTROLLER_HAS_SOURCE_DEAL_DAMAGE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_damage_unless_controller_has_source_deal_damage_matched(clause, &matched)
}

pub(crate) fn parse_sentence_damage_unless_controller_has_source_deal_damage_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(before_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Action) else {
        return Ok(None);
    };
    let Some(after_unless_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Condition)
    else {
        return Ok(None);
    };

    let before_clause = before_clause.trimmed();
    if before_clause.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(before_clause.tokens())?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let Some(main_damage) = effects.first() else {
        return Ok(None);
    };
    let (main_amount, main_target) = if let EffectAst::SubjectVerb(subject_verb) = main_damage
        && let SubjectVerbActionAst::DealDamage { amount, target } = &subject_verb.action
    {
        (amount, target)
    } else {
        return Ok(None);
    };
    if !matches!(
        main_target,
        TargetAst::Object(_, _, _) | TargetAst::WithCount(_, _)
    ) {
        return Ok(None);
    }

    let after_unless_clause = after_unless_clause.trimmed();
    let has_controller_clause = choice_damage_that_controller_subject_matches(after_unless_clause);
    if !has_controller_clause {
        return Ok(None);
    }
    let Some((_controller_clause, alt_clause)) =
        after_unless_clause.split_once_on_word_any(&["has", "have"])
    else {
        return Ok(None);
    };
    if alt_clause.is_empty() {
        return Ok(None);
    }

    let Some((_before_deal, deal_tail_clause)) =
        alt_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };
    let deal_tail = deal_tail_clause.tokens();
    let Some((alt_amount, used)) = parse_value(deal_tail) else {
        return Ok(None);
    };
    if !deal_tail
        .get(used)
        .and_then(|token| token.as_word())
        .is_some_and(|word| word == DAMAGE_WORD)
    {
        return Ok(None);
    }

    let alt_target_clause = choice_damage_strip_to_prefix(deal_tail_clause.from(used + 1));
    if !choice_damage_alternate_target_matches(alt_target_clause) {
        return Ok(None);
    }

    let alternative = EffectAst::subject_verb_damage(
        alt_amount,
        TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    );
    let unless = EffectAst::UnlessAction {
        effects: vec![EffectAst::subject_verb_damage(
            main_amount.clone(),
            main_target.clone(),
        )],
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    };
    Ok(Some(vec![unless]))
}

pub(crate) fn parse_sentence_damage_to_that_player_unless_enchanted_attacked(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !word_slice_contains_any_phrase(&clause.word_refs(), ENCHANTED_ATTACKED_THIS_TURN_PHRASES) {
        return Ok(None);
    }
    let pattern = LexPattern::new(DAMAGE_TO_THAT_PLAYER_UNLESS_ENCHANTED_ATTACKED_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_damage_to_that_player_unless_enchanted_attacked_matched(clause, &matched)
}

pub(crate) fn parse_sentence_damage_to_that_player_unless_enchanted_attacked_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(before_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Action) else {
        return Ok(None);
    };

    let before_clause = before_clause.trimmed();
    if before_clause.is_empty() {
        return Ok(None);
    }

    let Some((subject_clause, damage_clause)) =
        before_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };

    if !choice_damage_source_subject_matches(subject_clause) {
        return Ok(None);
    }
    let damage_tokens = damage_clause.tokens();
    let Some((amount, used)) = parse_value(damage_tokens) else {
        return Ok(None);
    };
    if !damage_tokens
        .get(used)
        .and_then(|token| token.as_word())
        .is_some_and(|word| word == DAMAGE_WORD)
    {
        return Ok(None);
    }

    let mut target_clause = damage_clause.from(used + 1).trimmed();
    if choice_damage_clause_first_is(target_clause, TO_WORD) {
        target_clause = target_clause.from(1).trimmed();
    }
    if !choice_damage_that_player_target_matches(target_clause) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::Conditional {
        predicate: PredicateAst::Not(Box::new(PredicateAst::EnchantedPermanentAttackedThisTurn)),
        if_true: vec![EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_sentence_unless_pays(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(UNLESS_PAYS_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_unless_pays_matched(clause, &matched)
}

pub(crate) fn parse_sentence_unless_pays_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let unless_idx = match find_unquoted_token_word(clause, "unless") {
        Some(idx) => idx,
        None => return Ok(None),
    };

    if unless_idx == 0 {
        let Some((unless_clause, effect_clause)) = clause.split_once_on_comma() else {
            return Ok(None);
        };
        if effect_clause.is_empty() {
            return Ok(None);
        }

        let effects = parse_effect_chain(effect_clause.tokens())?;
        if effects.is_empty() {
            return Ok(None);
        }

        if let Some(unless_effect) = try_build_unless(effects, unless_clause, 0)? {
            return Ok(Some(vec![unless_effect]));
        }
        return Ok(None);
    }

    let Some(before_unless_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Action)
    else {
        return Ok(None);
    };
    let before_words = before_unless_clause.word_refs();

    if before_words
        .first()
        .is_some_and(|word| *word == COUNTER_WORD)
    {
        return Ok(None);
    }
    if choice_damage_create_token_sacrifice_counter_clause_matches(before_unless_clause) {
        return Ok(None);
    }

    let sentence_words = clause.word_refs();
    if choice_damage_starts_with_scope(before_unless_clause, EACH_OPPONENT_SCOPE_PATTERN)
        && let Some(unless_word_idx) = clause.find_word("unless")
        && sentence_words.get(unless_word_idx + 1..unless_word_idx + 8)
            == Some(["its", "controller", "has", "you", "draw", "a", "card"].as_slice())
        && let Some(then_return_word_idx) =
            before_unless_clause.find_phrase_start(&["then", "return"])
        && sentence_words
            .get(3)
            .is_some_and(|word| *word == CHOOSE_WORD)
    {
        let Some(target_clause) = clause
            .after_words(4)
            .and_then(|tail| tail.before_word(then_return_word_idx.saturating_sub(4)))
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::UnlessAction {
                    effects: vec![EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        false,
                    )],
                    alternative: vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    )],
                    player: PlayerAst::ItsController,
                },
            ],
        }]));
    }

    let each_prefix = choice_damage_each_scope_kind(before_unless_clause);
    if let Some(prefix_kind) = each_prefix {
        let inner_clause = before_unless_clause
            .after_words(2)
            .unwrap_or_else(|| before_unless_clause.from(2));
        if let Ok(inner_effects) = parse_effect_chain(inner_clause.tokens()) {
            if !inner_effects.is_empty() {
                if let Some(unless_effect) = try_build_unless(inner_effects, clause, unless_idx)? {
                    let wrapper = match prefix_kind {
                        "opponent" => EffectAst::ForEachOpponent {
                            effects: vec![unless_effect],
                        },
                        _ => EffectAst::ForEachPlayer {
                            effects: vec![unless_effect],
                        },
                    };
                    return Ok(Some(vec![wrapper]));
                }
            }
        }
        return Ok(None);
    }

    let effect_clause = before_unless_clause;
    if let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(effect_clause)
    {
        let Some(delayed_effect_clause) = effect_clause
            .before_word(timing_start_word)
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        if delayed_effect_clause.is_empty() {
            return Ok(None);
        }
        let delayed_effects = parse_effect_chain(delayed_effect_clause.tokens())?;
        if delayed_effects.is_empty() {
            return Ok(None);
        }
        if let Some(unless_effect) = try_build_unless(delayed_effects, clause, unless_idx)? {
            return Ok(Some(vec![wrap_delayed_next_step_unless_pays(
                step,
                player,
                vec![unless_effect],
            )]));
        }
    }

    let effects = parse_effect_chain(effect_clause.tokens())?;
    if effects.is_empty() {
        return Ok(None);
    }

    if let Some(unless_effect) = try_build_unless(effects, clause, unless_idx)? {
        return Ok(Some(vec![unless_effect]));
    }
    Ok(None)
}

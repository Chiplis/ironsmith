use super::*;
use crate::runtime_backend::effect_sentences::clause_pattern_helpers::{ClauseShape, clause_shape};
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};

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
const FROM_PREFIX: &[&str] = &["from"];
const TO_PREFIX: &[&str] = &["to"];
const UP_TO_ONE_TARGET_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["up", "to", "one", "target"]);
const ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const OF_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["of"]);
const TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const REVEAL_ARTICLE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["one"]]);
const CARD_OR_CARDS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["card"], &["cards"]]);
const CARD_WORD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["card"]);
const HAND_REFERENCE_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["their", "hand"],
            &["their", "hands"],
            &["your", "hand"],
            &["your", "hands"],
            &["that", "player", "hand"],
            &["that", "player", "hands"],
            &["target", "player", "hand"],
            &["target", "player", "hands"],
        ]
);
const DAMAGE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["damage"]);
const DESTROY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["destroy"]);
const AT_RANDOM_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["at", "random"]]);
const YOU_GAIN_X_LIFE_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["you", "gain", "x", "life"]]);
const THEM_OR_THAT_PLAYER_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["them"], &["that", "player"]]);
const TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const EXILE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["exile"]);
const COUNTER_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["counter"]);
const CREATE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["create"]);
const CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);
const ENCHANTED_ATTACKED_THIS_TURN_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["that", "creature", "attacked", "this", "turn"],
            &["enchanted", "creature", "attacked", "this", "turn"],
        ]
);
const DAMAGE_SOURCE_SUBJECT_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["this", "aura"],
            &["this", "permanent"],
            &["this", "enchantment"],
        ]
);
const THAT_PLAYER_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["that", "player"]);

fn choice_damage_clause_first_matches(
    clause: SubjectVerbPrimitiveClause<'_>,
    shape: &ClauseShape<'static>,
) -> bool {
    clause
        .first_word()
        .is_some_and(|word| shape.matches_word(word))
}

fn is_explicit_target_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    choice_damage_clause_first_matches(clause, &TARGET_WORD_PATTERN)
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
const ENCHANTED_ATTACKED_UNLESS_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    &[LexPattern::phrase(&[
        "that", "creature", "attacked", "this", "turn",
    ])],
    &[LexPattern::phrase(&[
        "enchanted",
        "creature",
        "attacked",
        "this",
        "turn",
    ])],
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
    let has_lose_x =
        drain_clause.contains_any_phrase(&[&["lose", "x", "life"], &["loses", "x", "life"]]);
    let has_gain_x = YOU_GAIN_X_LIFE_MARKER_PATTERN.matches_words(&drain_clause.word_refs());
    if !has_lose_x || !has_gain_x {
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
    if !clause
        .first_word()
        .is_some_and(|word| EXILE_WORD_PATTERN.matches_word(word))
        || clause.contains_word("unless")
    {
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
                    && UP_TO_ONE_TARGET_PATTERN
                        .matches_words(&segment_clause.between(idx - 3, idx + 1).word_refs())
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
    if !choice_damage_clause_first_matches(clause, &DESTROY_WORD_PATTERN) {
        return Ok(None);
    }
    if clause
        .token(1)
        .is_some_and(|token| ALL_OR_EACH_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }
    if clause.contains_any_word(&["unless", "if"]) {
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
    let has_followup_tail =
        target_clause.contains_any_word(&["then", "if", "unless", "where", "when", "whenever"]);
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
            .is_some_and(|word| TARGET_WORD_PATTERN.matches_words(&[*word]))
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
    _matched: &LexPatternMatch<'_>,
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

    const HAND_SUFFIXES: &[&[&str]] = &[
        &["in", "your", "hand"],
        &["in", "your", "hands"],
        &["from", "your", "hand"],
        &["from", "your", "hands"],
    ];
    let Some((_suffix, before_in)) = clause.strip_any_suffix(HAND_SUFFIXES) else {
        return Ok(None);
    };
    if before_in.is_empty() {
        return Ok(None);
    }

    let mut descriptor_clause = before_in.from(1).trimmed();
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
        if choice_damage_clause_first_matches(descriptor_clause, &OF_WORD_PATTERN) {
            descriptor_clause = descriptor_clause.from(1).trimmed();
        }
    } else if descriptor_clause
        .first_word()
        .is_some_and(|word| REVEAL_ARTICLE_WORD_PATTERN.matches_word(word))
    {
        descriptor_clause = descriptor_clause.from(1).trimmed();
    } else if descriptor_clause
        .first_word()
        .is_some_and(|word| ALL_OR_EACH_WORD_PATTERN.matches_word(word))
    {
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
            if !descriptor_words
                .get(idx)
                .is_some_and(|word| CARD_OR_CARDS_WORD_PATTERN.matches_word(word))
            {
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((subject_clause, reveal_clause)) =
        clause.split_once_on_word_any(&["reveal", "reveals"])
    else {
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

    let reveal_clause = reveal_clause.trimmed();
    let reveal_words = reveal_clause.word_refs();
    if reveal_words.is_empty()
        || !reveal_words
            .first()
            .is_some_and(|word| REVEAL_ARTICLE_WORD_PATTERN.matches_words(&[*word]))
    {
        return Ok(None);
    }

    let Some(descriptor_clause) = reveal_clause.after_words(1) else {
        return Ok(None);
    };
    let descriptor_words = descriptor_clause.word_refs();
    if descriptor_words.is_empty() || !CARD_WORD_MARKER_PATTERN.matches_words(&descriptor_words) {
        return Ok(None);
    }

    let Some((random_descriptor_clause, hand_clause)) =
        descriptor_clause.split_once_on_phrase(FROM_PREFIX)
    else {
        return Ok(None);
    };
    if !AT_RANDOM_MARKER_PATTERN.matches_words(&random_descriptor_clause.word_refs()) {
        return Ok(None);
    }

    let hand_words = hand_clause.word_refs();
    if !HAND_REFERENCE_PATTERN.matches_words(&hand_words) {
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((before_clause, after_unless_clause)) = clause.split_once_on_word("unless") else {
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
    let has_controller_clause = after_unless_clause
        .strip_any_prefix(THAT_PREFIXES)
        .is_some()
        && after_unless_clause.contains_any_word(&["controller", "controllers"]);
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
        .is_some_and(|token| DAMAGE_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }

    let alt_target_clause = deal_tail_clause
        .from(used + 1)
        .strip_prefix_clause(TO_PREFIX)
        .unwrap_or_else(|| deal_tail_clause.from(used + 1));
    let alt_target_words = alt_target_clause.word_refs();
    if !THEM_OR_THAT_PLAYER_PATTERN.matches_words(&alt_target_words) {
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
    let pattern = LexPattern::new(DAMAGE_TO_THAT_PLAYER_UNLESS_ENCHANTED_ATTACKED_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_damage_to_that_player_unless_enchanted_attacked_matched(clause, &matched)
}

pub(crate) fn parse_sentence_damage_to_that_player_unless_enchanted_attacked_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((before_clause, after_clause)) = clause.split_once_on_word("unless") else {
        return Ok(None);
    };

    let before_clause = before_clause.trimmed();
    let after_clause = after_clause.trimmed();
    if before_clause.is_empty() || after_clause.is_empty() {
        return Ok(None);
    }

    if !ENCHANTED_ATTACKED_THIS_TURN_PATTERN.matches_words(&after_clause.word_refs()) {
        return Ok(None);
    }

    let Some((subject_clause, damage_clause)) =
        before_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };

    if !DAMAGE_SOURCE_SUBJECT_PATTERN.matches_words(&subject_clause.word_refs()) {
        return Ok(None);
    }
    let damage_tokens = damage_clause.tokens();
    let Some((amount, used)) = parse_value(damage_tokens) else {
        return Ok(None);
    };
    if !damage_tokens
        .get(used)
        .is_some_and(|token| DAMAGE_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }

    let mut target_clause = damage_clause.from(used + 1).trimmed();
    if choice_damage_clause_first_matches(target_clause, &TO_WORD_PATTERN) {
        target_clause = target_clause.from(1).trimmed();
    }
    if !THAT_PLAYER_PATTERN.matches_words(&target_clause.word_refs()) {
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
    _matched: &LexPatternMatch<'_>,
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

    let before_unless_clause = clause.before(unless_idx);
    let before_words = before_unless_clause.word_refs();

    if before_words
        .first()
        .is_some_and(|word| COUNTER_WORD_PATTERN.matches_words(&[*word]))
    {
        return Ok(None);
    }
    if before_words
        .first()
        .is_some_and(|word| CREATE_WORD_PATTERN.matches_words(&[*word]))
        && before_unless_clause.contains_word("token")
        && before_unless_clause.contains_word("sacrifice")
        && before_unless_clause.contains_word("counter")
    {
        return Ok(None);
    }

    let sentence_words = clause.word_refs();
    if before_unless_clause
        .strip_any_prefix(EACH_OPPONENT_PREFIXES)
        .is_some()
        && let Some(unless_word_idx) = clause.find_word("unless")
        && sentence_words.get(unless_word_idx + 1..unless_word_idx + 8)
            == Some(["its", "controller", "has", "you", "draw", "a", "card"].as_slice())
        && let Some(then_return_word_idx) =
            before_unless_clause.find_phrase_start(&["then", "return"])
        && sentence_words
            .get(3)
            .is_some_and(|word| CHOOSE_WORD_PATTERN.matches_words(&[*word]))
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

    let each_prefix = if before_unless_clause
        .strip_any_prefix(EACH_OPPONENT_PREFIXES)
        .is_some()
    {
        Some("opponent")
    } else if before_unless_clause
        .strip_any_prefix(EACH_PLAYER_PREFIXES)
        .is_some()
    {
        Some("player")
    } else {
        None
    };
    if let Some(prefix_kind) = each_prefix {
        let inner_clause = clause.after_words(2).unwrap_or_else(|| clause.from(2));
        let inner_clause =
            inner_clause.before(unless_idx.saturating_sub(clause.len() - inner_clause.len()));
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

    let effect_clause = clause.before(unless_idx);
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

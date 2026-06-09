use super::*;
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};

pub(crate) const END_OF_COMBAT_TIMING_PHRASES: &[&[&str]] = &[
    &["at", "end", "of", "combat"],
    &["at", "the", "end", "of", "combat"],
];
const OPTIONAL_THEN_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[LexPattern::word("then")];
pub(crate) const RETURN_WITH_COUNTERS_ON_IT_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::optional(OPTIONAL_THEN_PATTERN_ATOMS),
    LexPattern::word("return"),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilLastPhrase(&["to"]),
    ),
    LexPattern::word("to"),
    LexPattern::role_capture(
        "destination",
        LexCaptureRole::Modifier,
        LexCaptureKind::UntilPhrase(&["with"]),
    ),
    LexPattern::word("with"),
    LexPattern::role_capture(
        "counters",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilLastPhrase(&["on"]),
    ),
    LexPattern::word("on"),
    LexPattern::role_capture(
        "counter_target",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const PUT_ONTO_BATTLEFIELD_WORDS: &[&str] = &["put", "puts"];
const ON_WORD: &str = "on";
const PUT_WORDS: &[&str] = &["put"];
const AND_WORDS: &[&str] = &["and"];
const THEN_PREFIX: &[&str] = &["then"];
const IT_ENTERS_WITH_PREFIX: &[&str] = &["it", "enters", "with"];
const PUT_OR_REMOVE_COUNTER_KIND_TAIL_PHRASE: &[&str] = &[
    "put", "another", "counter", "of", "that", "kind", "on", "it", "or", "remove", "one", "from",
];
pub(crate) const PUT_ONTO_BATTLEFIELD_WITH_COUNTERS_ON_IT_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::any_word(PUT_ONTO_BATTLEFIELD_WORDS),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["onto"]),
    ),
    LexPattern::word("onto"),
    LexPattern::role_capture(
        "destination",
        LexCaptureRole::Modifier,
        LexCaptureKind::UntilPhrase(&["with"]),
    ),
    LexPattern::word("with"),
    LexPattern::role_capture(
        "counters",
        LexCaptureRole::Amount,
        LexCaptureKind::UntilLastPhrase(&["on"]),
    ),
    LexPattern::word("on"),
    LexPattern::role_capture(
        "counter_target",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const IF_ENTERS_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("if"),
    LexPattern::role_capture(
        "predicate",
        LexCaptureRole::Condition,
        LexCaptureKind::UntilPhrase(IT_ENTERS_WITH_PREFIX),
    ),
    LexPattern::phrase(IT_ENTERS_WITH_PREFIX),
    LexPattern::role_capture(
        "counter",
        LexCaptureRole::Amount,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const TAGGED_ENTERS_WITH_ADDITIONAL_COUNTER_PREFIXES: &[&[&str]] = &[
    &["each", "of", "them", "enters", "with"],
    &["all", "of", "them", "enter", "with"],
    &["that", "card", "enters", "with"],
    &["that", "creature", "enters", "with"],
    &["that", "object", "enters", "with"],
    &["that", "permanent", "enters", "with"],
    &["it", "enters", "with"],
];
pub(crate) const TAGGED_ENTERS_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS: &[LexPatternAtom<'static>] =
    &[
        LexPattern::any_phrase(TAGGED_ENTERS_WITH_ADDITIONAL_COUNTER_PREFIXES),
        LexPattern::role_capture("counter", LexCaptureRole::Amount, LexCaptureKind::Rest),
    ];
pub(crate) const PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::word("put"),
    LexPattern::role_capture(
        "move",
        LexCaptureRole::Action,
        LexCaptureKind::UntilLastPhrase(&["with"]),
    ),
    LexPattern::word("with"),
    LexPattern::role_capture(
        "counter",
        LexCaptureRole::Amount,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const SACRIFICE_WORDS: &[&str] = &["sacrifice", "sacrifices"];
const COUNTER_MARKER_PUT_ACTION_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "action",
        LexCaptureKind::OneOf(PUT_WORDS),
    )]);
const COUNTER_MARKER_AND_CONNECTOR_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "connector",
        LexCaptureKind::OneOf(AND_WORDS),
    )]);
const COUNTER_MARKER_SACRIFICE_ACTION_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "action",
        LexCaptureKind::OneOf(SACRIFICE_WORDS),
    )]);
pub(crate) const SACRIFICE_THEN_PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS:
    &[LexPatternAtom<'static>] = &[
    LexPattern::any_word(SACRIFICE_WORDS),
    LexPattern::role_capture(
        "sacrifice",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::word("put"),
    LexPattern::role_capture("put", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
pub(crate) const IF_SACRIFICE_THEN_PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS:
    &[LexPatternAtom<'static>] = &[
    LexPattern::word("if"),
    LexPattern::role_capture(
        "predicate",
        LexCaptureRole::Condition,
        LexCaptureKind::UntilAnyPhrase(&[&["sacrifice"], &["sacrifices"]]),
    ),
    LexPattern::any_word(SACRIFICE_WORDS),
    LexPattern::role_capture(
        "effect",
        LexCaptureRole::Action,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const EACH_PLAYER_RETURN_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::any_phrase(FOR_EACH_PLAYER_PREFIXES),
    LexPattern::any_word(&["return", "returns"]),
    LexPattern::role_capture(
        "return",
        LexCaptureRole::Action,
        LexCaptureKind::UntilLastPhrase(&["with"]),
    ),
    LexPattern::word("with"),
    LexPattern::role_capture(
        "counter",
        LexCaptureRole::Amount,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const FOR_EACH_COUNTER_KIND_PUT_OR_REMOVE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["for", "each", "kind", "of", "counter", "on"]),
    LexPattern::role_capture(
        "target",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["put", "another", "counter", "of", "that", "kind"]),
    ),
    LexPattern::phrase(&["put", "another", "counter", "of", "that", "kind"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const PUT_COUNTER_SEQUENCE_THEN_TAIL_SEQUENCE: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "counter_sequence",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["then"]),
    ),
    LexPattern::word("then"),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::OneOrMoreWords),
];
const PUT_COUNTER_SEQUENCE_REST_SEQUENCE: &[LexPatternAtom<'static>] = &[LexPattern::role_capture(
    "counter_sequence",
    LexCaptureRole::Action,
    LexCaptureKind::Rest,
)];
const PUT_COUNTER_SEQUENCE_SEQUENCES: &[&[LexPatternAtom<'static>]] = &[
    PUT_COUNTER_SEQUENCE_THEN_TAIL_SEQUENCE,
    PUT_COUNTER_SEQUENCE_REST_SEQUENCE,
];
pub(crate) const PUT_COUNTER_SEQUENCE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("put"),
    LexPattern::any_sequence(PUT_COUNTER_SEQUENCE_SEQUENCES),
];
const TAPPED_WORD: &str = "tapped";
const BATTLEFIELD_WORD: &str = "battlefield";
const ADDITIONAL_WORD: &str = "additional";
const BATTLEFIELD_PREFIX: &[&str] = &["battlefield"];
const COUNTER_MARKER_GAIN_ABILITY_WORDS: &[&str] = &["gain", "gains", "has", "have"];
const YOUR_CHOICE_OF_PREFIX_PHRASE: &[&str] = &["your", "choice", "of"];
const PRESERVE_CONTROL_SUBJECT_PHRASES: &[&[&str]] = &[&["its"], &["their"]];
const YOU_CONTROL_SUBJECT_PHRASES: &[&[&str]] = &[&["your"]];
const OWNER_CONTROL_SUBJECT_PHRASES: &[&[&str]] = &[
    &["its", "owner"],
    &["its", "owners"],
    &["their", "owner"],
    &["their", "owners"],
    &["his", "owner"],
    &["his", "owners"],
    &["her", "owner"],
    &["her", "owners"],
    &["that", "player"],
];
const IT_OR_THEM_PREFIXES: &[&[&str]] = &[&["it"], &["them"]];
const ENTERS_AS_CREATURE_PREDICATE_CLAUSES: &[&[&str]] = &[
    &["creature", "enters", "this", "way"],
    &["it", "enters", "as", "creature"],
];
const IT_OR_THEM_REFERENCE_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "reference",
    LexCaptureKind::OneOfPhrase(IT_OR_THEM_PREFIXES),
)]);
const COUNTER_MARKER_ONTO_BATTLEFIELD_MOVE_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("onto"),
    LexPattern::object("zone", LexCaptureKind::OneOf(BATTLEFIELD_PREFIX)),
]);
const COUNTER_MARKER_GAIN_ABILITY_ACTION_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::action(
        "ability_action",
        LexCaptureKind::OneOf(COUNTER_MARKER_GAIN_ABILITY_WORDS),
    )]);
const COUNTER_MARKER_YOUR_CHOICE_OF_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "choice",
        LexCaptureKind::OneOfPhrase(&[YOUR_CHOICE_OF_PREFIX_PHRASE]),
    )]);
const COUNTER_MARKER_THEN_PREFIX_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "then",
        LexCaptureKind::OneOfPhrase(&[THEN_PREFIX]),
    )]);

struct CounterMarkerControlTailEntry {
    controller_subjects: &'static [&'static [&'static str]],
    controller: ReturnControllerAst,
}

const COUNTER_MARKER_CONTROL_TAILS: &[CounterMarkerControlTailEntry] = &[
    CounterMarkerControlTailEntry {
        controller_subjects: PRESERVE_CONTROL_SUBJECT_PHRASES,
        controller: ReturnControllerAst::Preserve,
    },
    CounterMarkerControlTailEntry {
        controller_subjects: YOU_CONTROL_SUBJECT_PHRASES,
        controller: ReturnControllerAst::You,
    },
    CounterMarkerControlTailEntry {
        controller_subjects: OWNER_CONTROL_SUBJECT_PHRASES,
        controller: ReturnControllerAst::Owner,
    },
];

fn counter_marker_control_tail_controller(words: &[&str]) -> Option<ReturnControllerAst> {
    if words.is_empty() {
        return Some(ReturnControllerAst::Preserve);
    }

    COUNTER_MARKER_CONTROL_TAILS.iter().find_map(|entry| {
        let atoms = [
            LexPattern::word("under"),
            LexPattern::subject(
                "controller",
                LexCaptureKind::OneOfPhrase(entry.controller_subjects),
            ),
            LexPattern::word("control"),
        ];
        LexPattern::new(&atoms)
            .match_word_refs(words)
            .and_then(|matched| matched.capture_word_range("controller"))
            .map(|_| entry.controller)
    })
}

fn counter_marker_it_or_them_reference_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let on_target_words = clause.word_refs();
    word_slice_eq_any(&on_target_words, IT_OR_THEM_PREFIXES)
}

fn counter_marker_enters_as_creature_predicate_matches(predicate_words: &[&str]) -> bool {
    word_slice_eq_any(&predicate_words, ENTERS_AS_CREATURE_PREDICATE_CLAUSES)
}

fn counter_marker_battlefield_destination_word_index(base_destination_words: &[&str]) -> Option<usize> {
    if !word_slice_contains_word(&base_destination_words, BATTLEFIELD_WORD) {
        return None;
    }
    base_destination_words
        .iter()
        .position(|word| *word == BATTLEFIELD_WORD)
}

fn counter_marker_destination_is_tapped(base_destination_words: &[&str]) -> bool {
    word_slice_contains_word(&base_destination_words, TAPPED_WORD)
}

fn counter_marker_battlefield_destination_starts(base_destination_words: &[&str]) -> bool {
    word_slice_starts_with(&base_destination_words, BATTLEFIELD_PREFIX)
}

fn counter_marker_move_mentions_onto_battlefield(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    COUNTER_MARKER_ONTO_BATTLEFIELD_MOVE_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("zone"))
        .is_some()
}

fn counter_marker_it_or_them_prefix_len(words: &[&str]) -> Option<usize> {
    IT_OR_THEM_REFERENCE_PATTERN
        .find_in_word_refs(words)
        .and_then(|matched| matched.capture_word_range("reference"))
        .filter(|range| range.start == 0)
        .map(|range| range.end - range.start)
}

fn counter_marker_clause_head_len(
    clause: SubjectVerbPrimitiveClause<'_>,
    pattern: LexPattern<'static>,
    capture: &str,
) -> Option<usize> {
    clause
        .match_prefix_pattern(pattern)
        .and_then(|matched| matched.capture_word_range(capture))
        .filter(|range| range.start == 0)
        .map(|range| range.end - range.start)
}

fn counter_marker_clause_starts_with_action(
    clause: SubjectVerbPrimitiveClause<'_>,
    pattern: LexPattern<'static>,
) -> bool {
    counter_marker_clause_head_len(clause, pattern, "action").is_some()
}

fn counter_marker_strip_prefix_pattern<'a>(
    clause: SubjectVerbPrimitiveClause<'a>,
    pattern: LexPattern<'static>,
    capture: &str,
) -> Option<SubjectVerbPrimitiveClause<'a>> {
    let prefix_len = clause
        .match_prefix_pattern(pattern)
        .and_then(|matched| matched.capture_word_range(capture))
        .filter(|range| range.start == 0)
        .map(|range| range.end - range.start)?;
    Some(clause.from(prefix_len).trimmed())
}

fn counter_marker_strip_your_choice_of_prefix(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<SubjectVerbPrimitiveClause<'_>> {
    counter_marker_strip_prefix_pattern(
        clause,
        COUNTER_MARKER_YOUR_CHOICE_OF_PREFIX_PATTERN,
        "choice",
    )
}

fn counter_marker_strip_then_prefix(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<SubjectVerbPrimitiveClause<'_>> {
    counter_marker_strip_prefix_pattern(clause, COUNTER_MARKER_THEN_PREFIX_PATTERN, "then")
}

fn counter_marker_mentions_gain_ability_action(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    COUNTER_MARKER_GAIN_ABILITY_ACTION_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("ability_action"))
        .is_some()
}

fn counter_marker_mentions_additional(descriptor_clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    descriptor_clause.contains_word(ADDITIONAL_WORD)
}

fn counter_marker_matches_accepted_target<'p>(
    clause: SubjectVerbPrimitiveClause<'_>,
    accepted_targets: &'p [&'p [&'p str]],
) -> bool {
    let words = clause.word_refs();
    let atoms = [LexPattern::object(
        "target",
        LexCaptureKind::OneOfPhrase(accepted_targets),
    )];
    LexPattern::new(&atoms)
        .match_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("target"))
        .is_some()
}

fn subject_verb_put_counters_target(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::PutCounters { target, .. } => Some(target.clone()),
            SubjectVerbActionAst::PutCounterChoice { target, .. } => Some(target.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn strip_transformed_destination_tail<'a>(words: &[&'a str]) -> (Vec<&'a str>, bool) {
    let mut transformed = false;
    let stripped = words
        .iter()
        .copied()
        .filter(|word| {
            if *word == "transformed" {
                transformed = true;
                false
            } else {
                true
            }
        })
        .collect();
    (stripped, transformed)
}

fn counter_type_from_choice_segment(
    segment: SubjectVerbPrimitiveClause<'_>,
) -> Option<crate::object::CounterType> {
    let mut words = segment.word_refs();
    while words
        .first()
        .is_some_and(|word| matches!(*word, "and" | "or" | "a" | "an" | "one"))
    {
        words.remove(0);
    }
    while words
        .last()
        .is_some_and(|word| matches!(*word, "counter" | "counters"))
    {
        words.pop();
    }

    match words.as_slice() {
        ["first", "strike"] => Some(crate::object::CounterType::FirstStrike),
        ["double", "strike"] => Some(crate::object::CounterType::DoubleStrike),
        [word] => crate::runtime_backend::util::parse_counter_type_word(word),
        _ => None,
    }
}

fn parse_put_counter_choice_sequence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((descriptor_clause, target_clause)) = clause.split_once_on_word("on") else {
        return Ok(None);
    };
    let descriptor_clause = descriptor_clause.from(1).trimmed();
    let target_clause = target_clause.trimmed();
    if descriptor_clause.is_empty()
        || target_clause.is_empty()
        || descriptor_clause.contains_no_words(&["counter", "counters"])
    {
        return Ok(None);
    }

    let Some(choice_clause) = counter_marker_strip_your_choice_of_prefix(descriptor_clause) else {
        return Ok(None);
    };

    let mut counter_types = Vec::new();
    for segment in choice_clause.trimmed_comma_segments() {
        let segment = segment.trimmed();
        if segment.is_empty() {
            continue;
        }
        let Some(counter_type) = counter_type_from_choice_segment(segment) else {
            return Ok(None);
        };
        counter_types.push(counter_type);
    }

    if counter_types.len() < 2 {
        return Ok(None);
    }

    let target = parse_target_phrase(target_clause.tokens())?;
    let target_phrase = target_clause.word_refs().join(" ");
    let mode_texts = counter_types
        .iter()
        .map(|counter_type| {
            format!(
                "Put {} on {target_phrase}",
                super::super::zone_counter_helpers::describe_counter_phrase_for_mode(
                    1,
                    *counter_type,
                )
            )
        })
        .collect();

    Ok(Some(vec![EffectAst::subject_verb_put_counter_choice(
        counter_types,
        Value::Fixed(1),
        mode_texts,
        target,
        None,
    )]))
}

pub(crate) fn parse_sentence_sacrifice_at_end_of_combat(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "sacrifice <object> at [the] end of combat"
    let atoms = [
        LexPattern::word("sacrifice"),
        LexPattern::role_capture(
            "object",
            LexCaptureRole::Object,
            LexCaptureKind::UntilAnyPhrase(END_OF_COMBAT_TIMING_PHRASES),
        ),
        LexPattern::any_phrase(END_OF_COMBAT_TIMING_PHRASES),
        LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
    ];
    let pattern = LexPattern::new(&atoms);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_sacrifice_at_end_of_combat_matched(clause, &matched)
}

pub(crate) fn parse_sentence_sacrifice_at_end_of_combat_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(object_clause) = clause
        .pattern_capture_role(&matched, LexCaptureRole::Object)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if object_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in end-of-combat clause (clause: '{}')",
            clause.text()
        )));
    }

    let object_words = object_clause.word_refs();
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["that", "token"]
            | ["this", "token"]
            | ["that", "permanent"]
            | ["this", "permanent"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(object_clause.tokens(), false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            filter,
            1,
            None,
        )],
    }]))
}

pub(crate) fn parse_sentence_for_each_counter_kind_put_or_remove(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(FOR_EACH_COUNTER_KIND_PUT_OR_REMOVE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_for_each_counter_kind_put_or_remove_matched(clause, &matched)
}

pub(crate) fn parse_sentence_for_each_counter_kind_put_or_remove_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(target_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object) else {
        return Ok(None);
    };
    let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };

    let target_clause = target_clause.trimmed();
    if target_clause.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(target_clause.tokens())?;

    if !tail_clause.contains_phrase(PUT_OR_REMOVE_COUNTER_KIND_TAIL_PHRASE) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_for_each_counter_kind_put_or_remove(target),
    ]))
}

pub(crate) fn parse_put_counter_ladder_segments(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = clause.trimmed_comma_segments();
    if segments.len() != 3 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let Some(segment_clause) = counter_ladder_payload_segment(*segment, idx) else {
            return Ok(None);
        };
        if segment_clause.is_empty() {
            return Ok(None);
        }

        let Some((descriptor_clause, target_clause)) = segment_clause.split_once_on_word("on")
        else {
            return Ok(None);
        };
        let descriptor_clause = descriptor_clause.trimmed();
        let target_clause = target_clause.trimmed();
        if descriptor_clause.is_empty() || target_clause.is_empty() {
            return Ok(None);
        }

        let (count, counter_type) = parse_counter_descriptor(descriptor_clause.tokens())?;
        let target = parse_target_phrase(target_clause.tokens())?;
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            target,
            None,
            false,
        ));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_put_counter_sequence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(PUT_COUNTER_SEQUENCE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_put_counter_sequence_matched(clause, &matched)
}

pub(crate) fn parse_sentence_put_counter_sequence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !counter_marker_clause_starts_with_action(clause, COUNTER_MARKER_PUT_ACTION_PATTERN) {
        return Ok(None);
    }
    if clause.contains_no_words(&["counter", "counters"]) {
        return Ok(None);
    }

    if let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail)
        && !tail_clause.is_empty()
    {
        let Some(head_range) = matched.capture_word_range("counter_sequence") else {
            return Ok(None);
        };
        let Some(head_clause) = clause.before_word(head_range.end) else {
            return Ok(None);
        };
        let mut effects = parse_effect_chain(head_clause.tokens())?;
        if effects.is_empty() {
            return Ok(None);
        }
        effects.extend(parse_effect_chain(tail_clause.tokens())?);
        return Ok(Some(effects));
    }

    if let Some(effects) = parse_put_counter_ladder_segments(clause)? {
        return Ok(Some(effects));
    }

    if let Some(effects) = parse_put_counter_choice_sequence(clause)? {
        return Ok(Some(effects));
    }

    if let Some((descriptor_clause, target_clause)) = clause.split_once_on_word("on") {
        let descriptor_clause = descriptor_clause.from(1).trimmed();
        let target_clause = target_clause.trimmed();
        if !descriptor_clause.is_empty() && !target_clause.is_empty() {
            let mut descriptors: Vec<SubjectVerbPrimitiveOwnedClause> = Vec::new();
            let comma_segments = descriptor_clause.trimmed_comma_segments();
            if comma_segments.len() >= 2 {
                for segment in comma_segments {
                    let mut segment_clause = SubjectVerbPrimitiveOwnedClause::from_clause(segment);
                    segment_clause.remove_leading_word("and");
                    if segment_clause.is_empty() {
                        descriptors.clear();
                        break;
                    }
                    descriptors.push(segment_clause);
                }
            } else if let Some((first_clause, second_clause)) =
                descriptor_clause.split_once_on_word("and")
            {
                let first_clause = first_clause.trimmed();
                let second_clause = second_clause.trimmed();
                if !first_clause.is_empty() && !second_clause.is_empty() {
                    descriptors.push(SubjectVerbPrimitiveOwnedClause::from_clause(first_clause));
                    descriptors.push(SubjectVerbPrimitiveOwnedClause::from_clause(second_clause));
                }
            }

            if descriptors.len() >= 2 {
                let target = parse_target_phrase(target_clause.tokens())?;
                let mut effects = Vec::new();
                for descriptor in descriptors {
                    let (count, counter_type) = parse_counter_descriptor(descriptor.tokens())?;
                    effects.push(EffectAst::subject_verb_put_counters(
                        counter_type,
                        Value::Fixed(count as i32),
                        target.clone(),
                        None,
                        false,
                    ));
                }
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... counter on X and it gains ... until end of turn."
    if let Some((first_clause, second_clause)) = clause.split_once_on_phrase(&["and", "it"]) {
        let first_clause = first_clause.from(1).trimmed();
        let second_clause = second_clause.trimmed();
        if !first_clause.is_empty()
            && !second_clause.is_empty()
            && counter_marker_mentions_gain_ability_action(second_clause)
            && let Ok(first) = parse_put_counters(first_clause.tokens())
            && let Some(mut gain_effects) = parse_gain_ability_sentence(second_clause.tokens())?
        {
            let source_target = match &first {
                effect if subject_verb_put_counters_target(effect).is_some() => {
                    subject_verb_put_counters_target(effect)
                }
                EffectAst::Conditional { if_true, .. } if if_true.len() == 1 => {
                    if_true.first().and_then(subject_verb_put_counters_target)
                }
                _ => None,
            };

            if let Some(source_target) = source_target {
                for effect in &mut gain_effects {
                    match effect {
                        EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action:
                                SubjectVerbActionAst::Pump { target, .. }
                                | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
                                | SubjectVerbActionAst::GrantToTarget { target, .. }
                                | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. },
                            ..
                        }) => {
                            if let TargetAst::Tagged(tag, _) = target
                                && tag.as_str() == IT_TAG
                            {
                                *target = source_target.clone();
                            }
                        }
                        _ => {}
                    }
                }

                let mut effects = vec![first];
                effects.append(&mut gain_effects);
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... and ... counter on ..." without comma separation.
    if let Some((first_clause, second_clause)) = clause.split_once_on_word("and") {
        let first_clause = first_clause.from(1).trimmed();
        let second_clause = second_clause.trimmed();
        if !first_clause.is_empty() && !second_clause.is_empty() {
            if let (Ok(first), Ok(second)) = (
                parse_put_counters(first_clause.tokens()),
                parse_put_counters(second_clause.tokens()),
            ) {
                return Ok(Some(vec![first, second]));
            }
        }
    }

    let segments = clause.trimmed_comma_segments();
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let Some(segment_clause) = counter_ladder_payload_segment(*segment, idx) else {
            return Ok(None);
        };

        if segment_clause.is_empty() {
            return Ok(None);
        }

        if segment_clause.contains_no_words(&["counter", "counters"]) {
            return Ok(None);
        }

        let Ok(effect) = parse_put_counters(segment_clause.tokens()) else {
            return Ok(None);
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

pub(crate) fn is_pump_like_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Pump { .. }
                | SubjectVerbActionAst::PumpByLastEffect { .. }
                | SubjectVerbActionAst::SetBasePowerToughness { .. }
                | SubjectVerbActionAst::SetBasePower { .. },
            ..
        })
    )
}

pub(crate) fn parse_gets_then_fights_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let body_clause = counter_marker_strip_then_prefix(clause).unwrap_or(clause);
    if body_clause.is_empty() {
        return Ok(None);
    }

    // Split on "fight"/"fights"
    let Some((left_clause, right_clause)) =
        body_clause.split_once_on_word_any(&["fight", "fights"])
    else {
        return Ok(None);
    };

    let left_clause = left_clause.without_trailing_words_clause(&["and"]);
    let right_clause = right_clause.trimmed();
    if left_clause.is_empty() || right_clause.is_empty() {
        return Ok(None);
    }

    // Split left side on "get"/"gets" to extract subject
    let Some((subject_clause, _modifier_clause)) =
        left_clause.split_once_on_word_any(&["get", "gets"])
    else {
        return Ok(None);
    };

    let pump_effect = parse_effect_clause(left_clause.tokens())?;
    if !is_pump_like_effect(&pump_effect) {
        return Ok(None);
    }

    let subject_clause = subject_clause.trimmed();
    if subject_clause.is_empty() {
        return Ok(None);
    }
    let creature1 = parse_target_phrase(subject_clause.tokens())?;
    let creature2 = parse_target_phrase(right_clause.tokens())?;
    if matches!(
        creature1,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) || matches!(
        creature2,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "fight target must be a creature (clause: '{}')",
            clause.text()
        )));
    }

    Ok(Some(vec![
        pump_effect,
        EffectAst::subject_verb_fight(creature1, creature2),
    ]))
}

pub(crate) fn parse_sentence_gets_then_fights(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gets_then_fights_sentence(clause)
}

pub(crate) fn parse_return_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(RETURN_WITH_COUNTERS_ON_IT_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_return_with_counters_on_it_sentence_matched(clause, &matched)
}

pub(crate) fn parse_return_with_counters_on_it_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        crate::runtime_backend::util::non_article_possessive_word_refs(words)
    }

    let Some(target_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Object)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before destination (clause: '{}')",
            clause.text()
        )));
    }

    let Some(destination_clause) = clause
        .pattern_capture(matched, "destination")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if destination_clause.is_empty() {
        return Ok(None);
    }
    let base_destination_word_storage = destination_clause.word_refs();
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    let Some(battlefield_idx) =
        counter_marker_battlefield_destination_word_index(&base_destination_words)
    else {
        return Ok(None);
    };
    let tapped = counter_marker_destination_is_tapped(&base_destination_words);
    let destination_tail = base_destination_words[battlefield_idx + 1..]
        .iter()
        .copied()
        .filter(|word| *word != TAPPED_WORD)
        .collect::<Vec<_>>();
    let (destination_tail, transformed) = strip_transformed_destination_tail(&destination_tail);
    let Some(battlefield_controller) = counter_marker_control_tail_controller(&destination_tail)
    else {
        return Ok(None);
    };

    let Some(on_target_clause) = clause
        .pattern_capture(matched, "counter_target")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let on_target_words = on_target_clause.word_refs();
    let Some(prefix_len) = counter_marker_it_or_them_prefix_len(&on_target_words) else {
        return Ok(None);
    };
    let timing_words = &on_target_words[prefix_len..];
    let delayed_timing = if timing_words.is_empty() {
        None
    } else {
        super::super::zone_handlers::parse_delayed_return_timing_words(timing_words)
    };
    if !timing_words.is_empty() && delayed_timing.is_none() {
        return Ok(None);
    }

    let Some(descriptor_clause) = clause
        .pattern_capture(matched, "counters")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if descriptor_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            clause.text()
        )));
    }

    let descriptors = descriptor_clause.trimmed_and_segments();
    if descriptors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            clause.text()
        )));
    }

    let mut effects = vec![EffectAst::subject_verb_return_to_battlefield(
        parse_target_phrase(target_clause.tokens())?,
        tapped,
        transformed,
        false,
        battlefield_controller,
        None,
    )];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(descriptor.tokens())?;
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            tagged_target.clone(),
            None,
            false,
        ));
    }

    let wrapped = if let Some(timing) = delayed_timing {
        match timing {
            super::super::zone_handlers::DelayedReturnTimingAst::NextEndStep(player) => {
                vec![EffectAst::DelayedUntilNextEndStep { player, effects }]
            }
            super::super::zone_handlers::DelayedReturnTimingAst::NextUpkeep(player) => {
                vec![EffectAst::DelayedUntilNextUpkeep { player, effects }]
            }
            super::super::zone_handlers::DelayedReturnTimingAst::EndOfCombat => {
                vec![EffectAst::DelayedUntilEndOfCombat { effects }]
            }
        }
    } else {
        effects
    };

    Ok(Some(wrapped))
}

pub(crate) fn parse_put_onto_battlefield_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(PUT_ONTO_BATTLEFIELD_WITH_COUNTERS_ON_IT_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_put_onto_battlefield_with_counters_on_it_sentence_matched(clause, &matched)
}

pub(crate) fn parse_put_onto_battlefield_with_counters_on_it_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn normalize_destination_words<'a>(words: &'a [&'a str]) -> Vec<&'a str> {
        crate::runtime_backend::util::non_article_possessive_word_refs(words)
    }

    let Some(target_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Object)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing put target before destination (clause: '{}')",
            clause.text()
        )));
    }

    let Some(destination_clause) = clause
        .pattern_capture(matched, "destination")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if destination_clause.is_empty() {
        return Ok(None);
    }
    let base_destination_word_storage = destination_clause.word_refs();
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    if !counter_marker_battlefield_destination_starts(&base_destination_words) {
        return Ok(None);
    }

    let (destination_tail, transformed) =
        strip_transformed_destination_tail(&base_destination_words[1..]);
    let Some(battlefield_controller) = counter_marker_control_tail_controller(&destination_tail)
    else {
        return Ok(None);
    };

    let Some(on_target_clause) = clause
        .pattern_capture(matched, "counter_target")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if !counter_marker_it_or_them_reference_matches(on_target_clause) {
        return Ok(None);
    }

    let Some(descriptor_clause) = clause
        .pattern_capture(matched, "counters")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if descriptor_clause.is_empty() || descriptor_clause.contains_no_words(&["counter", "counters"])
    {
        return Ok(None);
    }

    let descriptors = descriptor_clause.trimmed_and_segments();
    if descriptors.is_empty() {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::subject_verb_move_to_zone(
        parse_target_phrase(target_clause.tokens())?,
        Zone::Battlefield,
        false,
        battlefield_controller,
        false,
        None,
    )];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), clause.span());
    if transformed {
        effects.push(EffectAst::subject_verb_transform(tagged_target.clone()));
    }
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(descriptor.tokens())?;
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            tagged_target.clone(),
            None,
            false,
        ));
    }

    Ok(Some(effects))
}

fn counter_ladder_payload_segment(
    segment: SubjectVerbPrimitiveClause<'_>,
    idx: usize,
) -> Option<SubjectVerbPrimitiveClause<'_>> {
    if idx == 0 {
        counter_marker_clause_head_len(segment, COUNTER_MARKER_PUT_ACTION_PATTERN, "action")
            .map(|prefix_len| segment.from(prefix_len).trimmed())
    } else if let Some(prefix_len) =
        counter_marker_clause_head_len(segment, COUNTER_MARKER_AND_CONNECTOR_PATTERN, "connector")
    {
        Some(segment.from(prefix_len).trimmed())
    } else {
        Some(segment.trimmed())
    }
}

pub(crate) fn parse_sentence_return_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(clause)
}

pub(crate) fn parse_sentence_put_onto_battlefield_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_put_onto_battlefield_with_counters_on_it_sentence(clause)
}

pub(crate) fn replace_target_subtype(target: &mut TargetAst, subtype: Subtype) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.subtypes = vec![subtype];
            true
        }
        TargetAst::WithCount(inner, _) => replace_target_subtype(inner, subtype),
        _ => false,
    }
}

pub(crate) fn clone_return_effect_with_subtype(
    base: &EffectAst,
    subtype: Subtype,
) -> Option<EffectAst> {
    match base {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ReturnToHand { target, random } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then_some(
                    EffectAst::subject_verb_return_to_hand(cloned_target, *random),
                )
            }
            SubjectVerbActionAst::ReturnAllToHand { filter } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(EffectAst::subject_verb_return_all_to_hand(cloned_filter))
            }
            SubjectVerbActionAst::ReturnToBattlefield {
                target,
                tapped,
                transformed,
                converted,
                controller,
                count_value,
                as_aura,
            } => {
                let mut cloned_target = target.clone();
                replace_target_subtype(&mut cloned_target, subtype).then(|| {
                    let mut effect = EffectAst::subject_verb_return_to_battlefield(
                        cloned_target,
                        *tapped,
                        *transformed,
                        *converted,
                        *controller,
                        count_value.clone(),
                    );
                    if let EffectAst::SubjectVerb(subject_verb) = &mut effect
                        && let SubjectVerbActionAst::ReturnToBattlefield { as_aura: dst, .. } =
                            &mut subject_verb.action
                    {
                        *dst = as_aura.clone();
                    }
                    effect
                })
            }
            SubjectVerbActionAst::ReturnAllToBattlefield {
                filter,
                tapped,
                face_down,
                controller,
            } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(EffectAst::subject_verb_return_all_to_battlefield(
                    cloned_filter,
                    *tapped,
                    *face_down,
                    *controller,
                ))
            }
            _ => None,
        },
        _ => None,
    }
}
pub(crate) fn parse_draw_then_connive_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((head_clause, tail_clause)) = clause.split_comma_then_trimmed() else {
        return Ok(None);
    };

    if head_clause.is_empty() || tail_clause.is_empty() {
        return Ok(None);
    }

    if tail_clause.contains_no_words(&["connive", "connives"]) {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(head_clause.tokens())?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(tail_clause.tokens())? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

pub(crate) fn parse_sentence_draw_then_connive(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(clause)
}

fn parse_additional_counter_descriptor_on_target(
    counter_clause: SubjectVerbPrimitiveClause<'_>,
    accepted_targets: &[&[&str]],
) -> Result<Option<(u32, crate::object::CounterType)>, CardTextError> {
    let counter_clause = counter_clause.trimmed();
    let Some((descriptor_clause, on_target_clause)) =
        counter_clause.rsplit_once_on_word_trimmed(ON_WORD)
    else {
        return Ok(None);
    };
    if descriptor_clause.is_empty()
        || !counter_marker_mentions_additional(descriptor_clause)
        || !counter_marker_matches_accepted_target(on_target_clause, accepted_targets)
    {
        return Ok(None);
    }

    parse_counter_descriptor(descriptor_clause.tokens()).map(Some)
}

pub(crate) fn parse_if_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(IF_ENTERS_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_if_enters_with_additional_counter_sentence_matched(clause, &matched)
}

pub(crate) fn parse_if_enters_with_additional_counter_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(predicate_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Condition)
    else {
        return Ok(None);
    };
    let Some(counter_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Amount) else {
        return Ok(None);
    };

    let predicate_words =
        crate::runtime_backend::util::non_article_word_refs(&predicate_clause.trimmed_word_refs());
    let predicate_is_supported =
        counter_marker_enters_as_creature_predicate_matches(&predicate_words);
    if !predicate_is_supported {
        return Ok(None);
    }

    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"], &["them"]])?
    else {
        return Ok(None);
    };
    let put_counter = EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    );
    let apply_only_if_creature = EffectAst::Conditional {
        predicate: PredicateAst::ItMatches(ObjectFilter::creature()),
        if_true: vec![put_counter],
        if_false: Vec::new(),
    };

    Ok(Some(vec![EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![apply_only_if_creature],
    }]))
}

pub(crate) fn parse_tagged_enters_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(TAGGED_ENTERS_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_tagged_enters_with_additional_counter_sentence_matched(clause, &matched)
}

pub(crate) fn parse_tagged_enters_with_additional_counter_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(counter_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Amount) else {
        return Ok(None);
    };
    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"]])?
    else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    )]))
}

pub(crate) fn parse_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_put_onto_battlefield_with_additional_counters_sentence_matched(clause, &matched)
}

pub(crate) fn parse_put_onto_battlefield_with_additional_counters_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(move_range) = matched.capture_word_range("move") else {
        return Ok(None);
    };
    let Some(move_clause) = clause.before_word(move_range.end) else {
        return Ok(None);
    };
    let Some(counter_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Amount) else {
        return Ok(None);
    };
    if move_clause.is_empty() || counter_clause.is_empty() {
        return Ok(None);
    }
    if !counter_marker_move_mentions_onto_battlefield(move_clause) {
        return Ok(None);
    }

    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"], &["them"]])?
    else {
        return Ok(None);
    };
    let mut effects = parse_effect_chain_inner(move_clause.tokens())?;
    if effects.is_empty()
        || !effects.iter().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::MoveToZone {
                        zone: Zone::Battlefield,
                        ..
                    } | SubjectVerbActionAst::ReturnToBattlefield { .. }
                        | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                    ..
                })
            )
        })
    {
        return Ok(None);
    }

    effects.push(EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    ));

    Ok(Some(effects))
}

pub(crate) fn parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern =
        LexPattern::new(SACRIFICE_THEN_PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence_matched(
        clause, &matched,
    )
}

pub(crate) fn parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(sacrifice_range) = matched.capture_word_range("sacrifice") else {
        return Ok(None);
    };
    let Some(put_range) = matched.capture_word_range("put") else {
        return Ok(None);
    };
    let Some(sacrifice_clause) = clause.before_word(sacrifice_range.end) else {
        return Ok(None);
    };
    let Some(put_clause) = clause.from_word(put_range.start.saturating_sub(1)) else {
        return Ok(None);
    };
    if sacrifice_clause.is_empty() || put_clause.is_empty() {
        return Ok(None);
    }

    let Some(mut put_effects) =
        parse_put_onto_battlefield_with_additional_counters_sentence(put_clause)?
    else {
        return Ok(None);
    };
    let mut effects = if sacrifice_clause.len() >= 2
        && counter_marker_clause_starts_with_action(
            sacrifice_clause,
            COUNTER_MARKER_SACRIFICE_ACTION_PATTERN,
        )
        && sacrifice_clause
            .from(1)
            .tokens()
            .iter()
            .all(|token| token.as_word().is_some())
    {
        vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            ObjectFilter {
                source: true,
                ..Default::default()
            },
            1,
            None,
        )]
    } else {
        parse_effect_chain_inner(sacrifice_clause.tokens())?
    };
    if effects.is_empty() {
        return Ok(None);
    }
    effects.append(&mut put_effects);
    Ok(Some(effects))
}

pub(crate) fn parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(
        IF_SACRIFICE_THEN_PUT_ONTO_BATTLEFIELD_WITH_ADDITIONAL_COUNTERS_PATTERN_ATOMS,
    );
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence_matched(
        clause, &matched,
    )
}

pub(crate) fn parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(predicate_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Condition)
    else {
        return Ok(None);
    };
    let Some(effect_range) = matched.capture_word_range("effect") else {
        return Ok(None);
    };
    let Some(effect_clause) = clause.from_word(effect_range.start.saturating_sub(1)) else {
        return Ok(None);
    };
    let predicate_clause = predicate_clause.trimmed();
    let effect_clause = effect_clause.trimmed();
    if predicate_clause.is_empty() || effect_clause.is_empty() {
        return Ok(None);
    }
    if !counter_marker_clause_starts_with_action(
        effect_clause,
        COUNTER_MARKER_SACRIFICE_ACTION_PATTERN,
    ) {
        return Ok(None);
    }

    let Some(effects) =
        parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence(effect_clause)?
    else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::Conditional {
        predicate: parse_predicate_lexed(predicate_clause.tokens())?,
        if_true: effects,
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_each_player_return_with_additional_counter_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(EACH_PLAYER_RETURN_WITH_ADDITIONAL_COUNTER_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_each_player_return_with_additional_counter_sentence_matched(clause, &matched)
}

pub(crate) fn parse_each_player_return_with_additional_counter_sentence_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(return_range) = matched.capture_word_range("return") else {
        return Ok(None);
    };
    let Some(return_clause) = clause
        .from_word(return_range.start.saturating_sub(1))
        .and_then(|clause| clause.before_word(return_range.end - return_range.start + 1))
    else {
        return Ok(None);
    };
    let Some(counter_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Amount) else {
        return Ok(None);
    };
    if return_clause.is_empty() {
        return Ok(None);
    }

    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"], &["them"]])?
    else {
        return Ok(None);
    };
    let mut per_player_effects = parse_effect_chain_inner(return_clause.tokens())?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToBattlefield { .. }
                    | SubjectVerbActionAst::ReturnAllToBattlefield { .. },
                ..
            })
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        TargetAst::Tagged(TagKey::from(IT_TAG), clause.span()),
        None,
        false,
    ));

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}

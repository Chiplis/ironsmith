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
        LexCaptureKind::UntilPhrase(&["it", "enters", "with"]),
    ),
    LexPattern::phrase(&["it", "enters", "with"]),
    LexPattern::role_capture(
        "counter",
        LexCaptureRole::Amount,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const TAGGED_ENTERS_WITH_ADDITIONAL_COUNTER_PREFIXES: &[&[&str]] = &[
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
pub(crate) const PUT_COUNTER_SEQUENCE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("put"),
    LexPattern::role_capture(
        "counter_sequence",
        LexCaptureRole::Action,
        LexCaptureKind::Rest,
    ),
];

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
        || !descriptor_clause.contains_phrase(&["your", "choice", "of"])
        || descriptor_clause.contains_no_words(&["counter", "counters"])
    {
        return Ok(None);
    }

    let Some(choice_clause) = descriptor_clause.strip_prefix_clause(&["your", "choice", "of"])
    else {
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "for each kind of counter on <target>, put another counter of that kind on it or remove one from it"
    let Some(after_prefix) =
        clause.strip_prefix_clause(&["for", "each", "kind", "of", "counter", "on"])
    else {
        return Ok(None);
    };
    let Some((target_clause, tail_clause)) = after_prefix.split_once_on_comma() else {
        return Ok(None);
    };

    let target_clause = target_clause.trimmed();
    if target_clause.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(target_clause.tokens())?;

    if !tail_clause.contains_phrase(&[
        "put", "another", "counter", "of", "that", "kind", "on", "it", "or", "remove", "one",
        "from",
    ]) {
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
        let segment_clause = if idx == 0 {
            if !segment.first_is_word("put") {
                return Ok(None);
            }
            segment.from(1).trimmed()
        } else if segment.first_is_word("and") {
            segment.from(1).trimmed()
        } else {
            segment.trimmed()
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause.first_is_word("put") {
        return Ok(None);
    }
    if clause.contains_no_words(&["counter", "counters"]) {
        return Ok(None);
    }

    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
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
            && second_clause.contains_any_word(&["gain", "gains", "has", "have"])
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
        let segment_clause = if idx == 0 {
            if !segment.first_is_word("put") {
                return Ok(None);
            }
            segment.from(1).trimmed()
        } else if segment.first_is_word("and") {
            segment.from(1).trimmed()
        } else {
            *segment
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
    let body_clause = clause.strip_prefix_clause(&["then"]).unwrap_or(clause);
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
    if !destination_clause.contains_word("battlefield") {
        return Ok(None);
    }

    let base_destination_word_storage = destination_clause.word_refs();
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    let Some(battlefield_idx) =
        crate::runtime_backend::lexer::word_slice_find_word(&base_destination_words, "battlefield")
    else {
        return Ok(None);
    };
    let tapped = word_slice_contains_word(&base_destination_words, "tapped");
    let destination_tail = super::super::super::util::word_refs_except(
        &base_destination_words[battlefield_idx + 1..],
        &["tapped"],
    );
    const PRESERVE_CONTROL_TAILS: &[&[&str]] =
        &[&["under", "its", "control"], &["under", "their", "control"]];
    const OWNER_CONTROL_TAILS: &[&[&str]] = &[
        &["under", "its", "owner", "control"],
        &["under", "their", "owner", "control"],
        &["under", "his", "owner", "control"],
        &["under", "her", "owner", "control"],
        &["under", "that", "player", "control"],
    ];
    let battlefield_controller = if destination_tail.is_empty()
        || crate::runtime_backend::lexer::word_slice_eq_any(
            &destination_tail,
            PRESERVE_CONTROL_TAILS,
        ) {
        ReturnControllerAst::Preserve
    } else if crate::runtime_backend::lexer::word_slice_eq(
        &destination_tail,
        &["under", "your", "control"],
    ) {
        ReturnControllerAst::You
    } else if crate::runtime_backend::lexer::word_slice_eq_any(
        &destination_tail,
        OWNER_CONTROL_TAILS,
    ) {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    let Some(on_target_clause) = clause
        .pattern_capture(matched, "counter_target")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let on_target_words = on_target_clause.word_refs();
    let timing_words = if word_slice_starts_with(&on_target_words, &["it"])
        || word_slice_starts_with(&on_target_words, &["them"])
    {
        &on_target_words[1..]
    } else {
        return Ok(None);
    };
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
        false,
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
    if !destination_clause.contains_word("battlefield") {
        return Ok(None);
    }

    let base_destination_word_storage = destination_clause.word_refs();
    let base_destination_words = normalize_destination_words(&base_destination_word_storage);
    if base_destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }

    let destination_tail = &base_destination_words[1..];
    const OWNER_CONTROL_TAILS: &[&[&str]] = &[
        &["under", "its", "owner", "control"],
        &["under", "their", "owner", "control"],
        &["under", "his", "owner", "control"],
        &["under", "her", "owner", "control"],
        &["under", "that", "player", "control"],
    ];
    let supported_control_tail = destination_tail.is_empty()
        || crate::runtime_backend::lexer::word_slice_eq(
            destination_tail,
            &["under", "your", "control"],
        )
        || crate::runtime_backend::lexer::word_slice_eq_any(destination_tail, OWNER_CONTROL_TAILS);
    if !supported_control_tail {
        return Ok(None);
    }
    let battlefield_controller = if crate::runtime_backend::lexer::word_slice_eq(
        destination_tail,
        &["under", "your", "control"],
    ) {
        ReturnControllerAst::You
    } else if crate::runtime_backend::lexer::word_slice_eq_any(
        destination_tail,
        OWNER_CONTROL_TAILS,
    ) {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };

    let Some(on_target_clause) = clause
        .pattern_capture(matched, "counter_target")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let on_target_words = on_target_clause.word_refs();
    if !crate::runtime_backend::lexer::word_slice_eq_any(&on_target_words, &[&["it"], &["them"]]) {
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
                controller,
            } => {
                let mut cloned_filter = filter.clone();
                cloned_filter.subtypes = vec![subtype];
                Some(EffectAst::subject_verb_return_all_to_battlefield(
                    cloned_filter,
                    *tapped,
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
        counter_clause.rsplit_once_on_word_trimmed("on")
    else {
        return Ok(None);
    };
    if descriptor_clause.is_empty()
        || !descriptor_clause.contains_word("additional")
        || !accepted_targets
            .iter()
            .any(|target_words| on_target_clause.word_refs() == *target_words)
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // "if <predicate>, it enters with <counter descriptor> on it"
    let Some(after_if) = clause.strip_prefix_clause(&["if"]) else {
        return Ok(None);
    };
    let Some((predicate_clause, followup_clause)) = after_if.split_once_on_comma() else {
        return Ok(None);
    };

    let predicate_words =
        crate::runtime_backend::util::non_article_word_refs(&predicate_clause.trimmed_word_refs());
    let predicate_is_supported = crate::runtime_backend::lexer::word_slice_eq_any(
        &predicate_words,
        &[
            &["creature", "enters", "this", "way"],
            &["it", "enters", "as", "creature"],
        ],
    );
    if !predicate_is_supported {
        return Ok(None);
    }

    let Some(counter_clause) = followup_clause
        .trimmed()
        .strip_prefix_clause(&["it", "enters", "with"])
    else {
        return Ok(None);
    };

    let Some((count, counter_type)) =
        parse_additional_counter_descriptor_on_target(counter_clause, &[&["it"]])?
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((_, counter_clause)) =
        clause.strip_any_prefix_clause(TAGGED_ENTERS_WITH_ADDITIONAL_COUNTER_PREFIXES)
    else {
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if clause.first_word() != Some("put") {
        return Ok(None);
    }
    if !clause.contains_all_words(&["onto", "battlefield"]) {
        return Ok(None);
    }

    let Some((move_clause, counter_clause)) = clause.rsplit_once_on_word_trimmed("with") else {
        return Ok(None);
    };
    if move_clause.is_empty() || counter_clause.is_empty() {
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !clause
        .token(0)
        .is_some_and(|token| token.is_word("sacrifice") || token.is_word("sacrifices"))
    {
        return Ok(None);
    }

    let Some((sacrifice_clause, put_clause)) = clause.split_once_on_then_trimmed() else {
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
        && sacrifice_clause.first_is_word("sacrifice")
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(after_if) = clause.strip_prefix_clause(&["if"]) else {
        return Ok(None);
    };
    let Some((predicate_clause, effect_clause)) = after_if.split_once_on_comma() else {
        return Ok(None);
    };

    let predicate_clause = predicate_clause.trimmed();
    let effect_clause = effect_clause.trimmed();
    if predicate_clause.is_empty() || effect_clause.is_empty() {
        return Ok(None);
    }
    if !effect_clause.first_is_any_word(&["sacrifice", "sacrifices"]) {
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
    _matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((_prefix, inner_clause)) = clause.strip_any_prefix_clause(FOR_EACH_PLAYER_PREFIXES)
    else {
        return Ok(None);
    };
    let inner_clause = inner_clause.trimmed();
    if inner_clause.is_empty() {
        return Ok(None);
    }
    if !inner_clause.first_is_any_word(&["return", "returns"]) {
        return Ok(None);
    }

    let Some((return_clause, counter_clause)) = inner_clause.rsplit_once_on_word_trimmed("with")
    else {
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

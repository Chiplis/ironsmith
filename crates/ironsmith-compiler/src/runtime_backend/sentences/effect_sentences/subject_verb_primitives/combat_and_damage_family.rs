use super::*;
use crate::runtime_backend::front_end::lex_patterns::{
    LexCaptureKind, LexCaptureRole, LexPattern, LexPatternAtom, LexPatternMatch,
};
use crate::runtime_backend::lexer::TokenKind;

const PUT_STICKER_WORDS: &[&str] = &["put", "puts"];
const STICKER_KIND_PHRASES: &[(&[&str], crate::events::KeywordActionKind)] = &[
    (
        &["name", "sticker"],
        crate::events::KeywordActionKind::NameSticker,
    ),
    (
        &["art", "sticker"],
        crate::events::KeywordActionKind::ArtSticker,
    ),
    (
        &["ability", "sticker"],
        crate::events::KeywordActionKind::AbilitySticker,
    ),
    (
        &["power", "and", "toughness", "sticker"],
        crate::events::KeywordActionKind::PowerToughnessSticker,
    ),
];
const COMBAT_AND_DAMAGE_FROM_WORDS: &[&str] = &["from"];
const COMBAT_AND_DAMAGE_INTO_OR_IN_WORDS: &[&str] = &["into", "in"];
const COMBAT_AND_DAMAGE_THEM_WORDS: &[&str] = &["them"];
const SOURCE_ZONE_BATTLEFIELD_WORDS: &[&str] = &["battlefield"];
const SOURCE_ZONE_COMMAND_PHRASES: &[&[&str]] = &[&["command", "zone"]];
const SOURCE_ZONE_GRAVEYARD_WORDS: &[&str] = &["graveyard", "graveyards"];
const SOURCE_ZONE_GRAVEYARD_PHRASES: &[&[&str]] = &[&["graveyard"], &["graveyards"]];
const THAT_WORD: &str = "that";
const ALL_OR_EACH_WORDS: &[&str] = &["all", "each"];
const TARGET_REFERENCE_WORDS: &[&str] = &["target", "it", "them", "that", "those", "this"];
const TARGET_WORDS: &[&str] = &["target"];
const ZONE_SUFFIX_START_WORDS: &[&str] = &["from", "to", "in", "on", "under"];
const TARGET_REFERENCE_HEAD_WORDS: &[&str] =
    &["target", "up", "this", "that", "it", "them", "another"];
const RETURN_SEGMENT_HEAD_WORDS: &[&str] = &[
    "target", "up", "another", "other", "this", "that", "it", "them", "all", "each",
];
const RETURN_DESTINATION_HAND_WORDS: &[&str] = &["hand", "hands"];
const RETURN_DESTINATION_HAND_PHRASES: &[&[&str]] = &[&["hand"], &["hands"]];
const RETURN_DESTINATION_BATTLEFIELD_WORDS: &[&str] = &["battlefield"];
const RETURN_DESTINATION_TAPPED_WORDS: &[&str] = &["tapped"];
const RETURN_SEGMENT_ZONE_WORDS: &[&str] = &[
    "graveyard",
    "graveyards",
    "battlefield",
    "hand",
    "hands",
    "library",
    "libraries",
    "exile",
];
const ARENT_WORDS: &[&str] = &["arent", "aren't"];
const ARE_WORD: &str = "are";
const NOT_WORD: &str = "not";
const TRANSFORM_WORD: &str = "transform";
const CONVERT_WORD: &str = "convert";
const GET_OR_GETS_WORDS: &[&str] = &["get", "gets"];
const DISTRIBUTE_WORD: &str = "distribute";

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word() == Some(expected)
}

fn token_is_any_word(token: &OwnedLexToken, expected: &[&str]) -> bool {
    token.as_word().is_some_and(|word| expected.contains(&word))
}

pub(crate) const DESTROY_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["destroy", "all", "creatures"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
const GET_VERB_PHRASES: &[&[&str]] = &[&["get"], &["gets"]];
pub(crate) const PUMP_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(GET_VERB_PHRASES),
    ),
    LexPattern::role_capture(
        "get_tail",
        LexCaptureRole::Action,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const PUT_STICKER_ON_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_word(PUT_STICKER_WORDS),
    LexPattern::role_capture(
        "sticker",
        LexCaptureRole::Action,
        LexCaptureKind::UntilLastPhrase(&["on"]),
    ),
    LexPattern::word("on"),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::OneOrMoreWords,
    ),
];
const RETURN_TO_HAND_DESTINATION_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::modifier(
        "owner",
        LexCaptureKind::UntilAnyPhrase(RETURN_DESTINATION_HAND_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(RETURN_DESTINATION_HAND_WORDS)),
]);
const RETURN_TO_BATTLEFIELD_DESTINATION_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "zone",
        LexCaptureKind::OneOf(RETURN_DESTINATION_BATTLEFIELD_WORDS),
    )]);
const RETURN_DESTINATION_TAPPED_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::modifier(
        "tapped",
        LexCaptureKind::OneOf(RETURN_DESTINATION_TAPPED_WORDS),
    )]);
const LITERAL_TARGET_REFERENCE_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "target",
        LexCaptureKind::OneOf(TARGET_WORDS),
    )]);
const BROAD_TARGET_REFERENCE_HEAD_PATTERN: LexPattern<'static> =
    LexPattern::new(&[LexPattern::object(
        "target",
        LexCaptureKind::OneOf(TARGET_REFERENCE_HEAD_WORDS),
    )]);
const RETURN_SEGMENT_HEAD_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "head",
    LexCaptureKind::OneOf(RETURN_SEGMENT_HEAD_WORDS),
)]);
const ZONE_SUFFIX_HEAD_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::modifier(
    "zone_suffix",
    LexCaptureKind::OneOf(ZONE_SUFFIX_START_WORDS),
)]);
const ALL_OR_EACH_HEAD_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::modifier(
    "quantifier",
    LexCaptureKind::OneOf(ALL_OR_EACH_WORDS),
)]);
const RETURN_SEGMENT_ZONE_PATTERN: LexPattern<'static> = LexPattern::new(&[LexPattern::object(
    "zone",
    LexCaptureKind::OneOf(RETURN_SEGMENT_ZONE_WORDS),
)]);
const SOURCE_BATTLEFIELD_AND_GRAVEYARD_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action(
        "from_battlefield",
        LexCaptureKind::OneOf(COMBAT_AND_DAMAGE_FROM_WORDS),
    ),
    LexPattern::optional(&[LexPattern::word("the")]),
    LexPattern::object(
        "battlefield",
        LexCaptureKind::OneOf(SOURCE_ZONE_BATTLEFIELD_WORDS),
    ),
    LexPattern::word("and"),
    LexPattern::action(
        "from_graveyard",
        LexCaptureKind::OneOf(COMBAT_AND_DAMAGE_FROM_WORDS),
    ),
    LexPattern::modifier(
        "graveyard_owner",
        LexCaptureKind::UntilAnyPhrase(SOURCE_ZONE_GRAVEYARD_PHRASES),
    ),
    LexPattern::object(
        "graveyard",
        LexCaptureKind::OneOf(SOURCE_ZONE_GRAVEYARD_WORDS),
    ),
]);
const SOURCE_COMMAND_AND_GRAVEYARD_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action(
        "from_command",
        LexCaptureKind::OneOf(COMBAT_AND_DAMAGE_FROM_WORDS),
    ),
    LexPattern::optional(&[LexPattern::word("the")]),
    LexPattern::object(
        "command",
        LexCaptureKind::OneOfPhrase(SOURCE_ZONE_COMMAND_PHRASES),
    ),
    LexPattern::word("and"),
    LexPattern::action(
        "from_graveyard",
        LexCaptureKind::OneOf(COMBAT_AND_DAMAGE_FROM_WORDS),
    ),
    LexPattern::modifier(
        "graveyard_owner",
        LexCaptureKind::UntilAnyPhrase(SOURCE_ZONE_GRAVEYARD_PHRASES),
    ),
    LexPattern::object(
        "graveyard",
        LexCaptureKind::OneOf(SOURCE_ZONE_GRAVEYARD_WORDS),
    ),
]);
const CHOOSE_ALL_PUT_THEM_HAND_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("put"),
    LexPattern::object(
        "target",
        LexCaptureKind::OneOf(COMBAT_AND_DAMAGE_THEM_WORDS),
    ),
    LexPattern::action(
        "destination_action",
        LexCaptureKind::OneOf(COMBAT_AND_DAMAGE_INTO_OR_IN_WORDS),
    ),
    LexPattern::modifier(
        "hand_owner",
        LexCaptureKind::UntilAnyPhrase(RETURN_DESTINATION_HAND_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(RETURN_DESTINATION_HAND_WORDS)),
]);
const CHOOSE_ALL_HAND_DESTINATION_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::action(
        "destination_action",
        LexCaptureKind::OneOf(COMBAT_AND_DAMAGE_INTO_OR_IN_WORDS),
    ),
    LexPattern::modifier(
        "hand_owner",
        LexCaptureKind::UntilAnyPhrase(RETURN_DESTINATION_HAND_PHRASES),
    ),
    LexPattern::object("zone", LexCaptureKind::OneOf(RETURN_DESTINATION_HAND_WORDS)),
]);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ReturnDestinationShape {
    zone: Zone,
    tapped: bool,
}

fn parse_return_destination_shape(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<ReturnDestinationShape> {
    let lexed = LexedClause::new(clause.tokens());
    let zone = if RETURN_TO_HAND_DESTINATION_PATTERN
        .find_in_clause(lexed)
        .is_some()
    {
        Zone::Hand
    } else if RETURN_TO_BATTLEFIELD_DESTINATION_PATTERN
        .find_in_clause(lexed)
        .is_some()
    {
        Zone::Battlefield
    } else {
        return None;
    };
    let tapped = RETURN_DESTINATION_TAPPED_PATTERN
        .find_in_clause(lexed)
        .is_some();
    Some(ReturnDestinationShape { zone, tapped })
}

fn target_reference_head_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    clause
        .word_refs()
        .first()
        .is_some_and(|word| TARGET_REFERENCE_WORDS.contains(word))
}

fn broad_target_reference_head_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    clause
        .word_refs()
        .first()
        .is_some_and(|word| TARGET_REFERENCE_HEAD_WORDS.contains(word))
}

fn literal_target_reference_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let words = clause.word_refs();
    LITERAL_TARGET_REFERENCE_PATTERN
        .find_in_word_refs(&words)
        .and_then(|matched| matched.capture_word_range("target"))
        .is_some()
}

fn return_segment_head_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    clause
        .match_prefix_pattern(RETURN_SEGMENT_HEAD_PATTERN)
        .and_then(|matched| matched.capture_word_range("head"))
        .is_some()
}

fn zone_suffix_head_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    clause
        .word_refs()
        .first()
        .is_some_and(|word| ZONE_SUFFIX_START_WORDS.contains(word))
}

fn all_or_each_head_matches(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    all_or_each_head_word(clause).is_some()
}

fn all_or_each_head_word(clause: SubjectVerbPrimitiveClause<'_>) -> Option<String> {
    clause
        .word_refs()
        .first()
        .filter(|word| ALL_OR_EACH_WORDS.contains(word))
        .map(|word| (*word).to_string())
}

fn parse_choose_all_source_zone_pair(clause: SubjectVerbPrimitiveClause<'_>) -> Option<[Zone; 2]> {
    let lexed = LexedClause::new(clause.tokens());
    if SOURCE_BATTLEFIELD_AND_GRAVEYARD_PATTERN
        .find_in_clause(lexed)
        .is_some()
    {
        return Some([Zone::Battlefield, Zone::Graveyard]);
    }
    if SOURCE_COMMAND_AND_GRAVEYARD_PATTERN
        .find_in_clause(lexed)
        .is_some()
    {
        return Some([Zone::Command, Zone::Graveyard]);
    }
    None
}

fn choose_all_put_clause_returns_them_to_hand(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    CHOOSE_ALL_PUT_THEM_HAND_PATTERN
        .match_prefix(LexedClause::new(clause.tokens()))
        .is_some()
}

fn choose_all_clause_ends_in_hand_destination(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    let word_len = clause.word_refs().len();
    CHOOSE_ALL_HAND_DESTINATION_PATTERN
        .find_in_clause(LexedClause::new(clause.tokens()))
        .is_some_and(|matched| matched.word_range.end == word_len)
}
pub(crate) const MUST_ATTACK_CREATURE_TYPE_OF_CHOICE_SUFFIXES: &[&[&str]] = &[
    &["attack", "this", "turn", "if", "able"],
    &["attacks", "this", "turn", "if", "able"],
];
pub(crate) const MUST_ATTACK_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::role_capture(
        "subject",
        LexCaptureRole::Subject,
        LexCaptureKind::UntilAnyPhrase(MUST_ATTACK_CREATURE_TYPE_OF_CHOICE_SUFFIXES),
    ),
    LexPattern::any_phrase(MUST_ATTACK_CREATURE_TYPE_OF_CHOICE_SUFFIXES),
];
pub(crate) const RETURN_TARGETS_OF_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::word("return"),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilLastPhrase(&["to"]),
    ),
    LexPattern::word("to"),
    LexPattern::role_capture(
        "destination",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const RETURN_MULTIPLE_TARGETS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("return"),
    LexPattern::role_capture(
        "objects",
        LexCaptureRole::Object,
        LexCaptureKind::UntilLastPhrase(&["to"]),
    ),
    LexPattern::word("to"),
    LexPattern::role_capture(
        "destination",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const FOR_EACH_THIS_WAY_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::phrase(&["for", "each"]),
    LexPattern::role_capture(
        "body",
        LexCaptureRole::Object,
        LexCaptureKind::UntilToken(TokenKind::Comma),
    ),
    LexPattern::token(TokenKind::Comma),
    LexPattern::role_capture(
        "effect",
        LexCaptureRole::Tail,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const CHOOSE_ALL_BATTLEFIELD_GRAVEYARD_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(CHOOSE_ALL_OR_PUT_ALL_PREFIXES),
    LexPattern::role_capture(
        "object",
        LexCaptureRole::Object,
        LexCaptureKind::UntilPhrase(&["from"]),
    ),
    LexPattern::word("from"),
    LexPattern::role_capture("zones_and_tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];
pub(crate) const FOR_EACH_TARGET_OBJECTS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::any_phrase(&[&["for", "each"], &["each"]]),
    LexPattern::role_capture("subject", LexCaptureRole::Subject, LexCaptureKind::Rest),
];
pub(crate) const DISTRIBUTE_COUNTERS_PATTERN_ATOMS: &[LexPatternAtom<'static>] = &[
    LexPattern::word("distribute"),
    LexPattern::role_capture(
        "amount",
        LexCaptureRole::Amount,
        LexCaptureKind::WordCount(1),
    ),
    LexPattern::role_capture(
        "counter_and_targets",
        LexCaptureRole::Object,
        LexCaptureKind::OneOrMoreWords,
    ),
];
pub(crate) const EACH_PLAYER_PUT_PERMANENT_CARDS_EXILED_PATTERN_ATOMS: &[LexPatternAtom<
    'static,
>] = &[
    LexPattern::phrase(&["each", "player"]),
    LexPattern::role_capture(
        "action",
        LexCaptureRole::Action,
        LexCaptureKind::UntilPhrase(&["permanent", "cards", "exiled", "with"]),
    ),
    LexPattern::phrase(&["permanent", "cards", "exiled", "with"]),
    LexPattern::role_capture("tail", LexCaptureRole::Tail, LexCaptureKind::Rest),
];

pub(crate) fn parse_sentence_destroy_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(DESTROY_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_destroy_creature_type_of_choice_matched(clause, &matched)
}

pub(crate) fn parse_sentence_destroy_creature_type_of_choice_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(tail_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if find_creature_type_choice_phrase(tail_clause).is_none() {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_destroy_all(ObjectFilter::creature().of_chosen_creature_type()),
    ]))
}

pub(crate) fn parse_sentence_pump_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(PUMP_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_pump_creature_type_of_choice_matched(clause, &matched)
}

pub(crate) fn parse_sentence_pump_creature_type_of_choice_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(subject_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Subject)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if subject_clause.is_empty() {
        return Ok(None);
    }
    let Some(get_tail_clause) = clause
        .pattern_capture(matched, "get_tail")
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if !get_tail_clause
        .first_word()
        .is_some_and(|word| GET_OR_GETS_WORDS.contains(&word))
    {
        return Ok(None);
    }
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(subject_clause) else {
        return Ok(None);
    };
    if !subject_clause
        .from(choice_idx + consumed)
        .trimmed()
        .is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            clause.text()
        )));
    }
    let trimmed_subject_clause =
        subject_clause.without_token_range_trimmed_clause(choice_idx, consumed);
    if trimmed_subject_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_clause = trimmed_subject_clause.clone();
    gain_candidate_clause.append_clause(get_tail_clause);
    if let Some(mut gain_effects) = parse_gain_ability_sentence(gain_candidate_clause.tokens())? {
        let mut patched = false;
        for effect in &mut gain_effects {
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::PumpAll { filter, .. }
                        | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. }
                        | SubjectVerbActionAst::GrantAbilitiesChoiceAll { filter, .. },
                    ..
                }) => {
                    filter.chosen_creature_type = true;
                    patched = true;
                }
                _ => {}
            }
        }
        if patched {
            let mut effects = vec![EffectAst::subject_verb_choose_creature_type(
                PlayerAst::You,
                vec![],
            )];
            effects.extend(gain_effects);
            return Ok(Some(effects));
        }
    }

    let mut filter_clause = trimmed_subject_clause;
    filter_clause.remove_leading_word("all");
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }

    let modifier = clause
        .pattern_capture(matched, "get_tail")
        .and_then(|tail| tail.token(1))
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                clause.text()
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            clause.text()
        ))
    })?;
    let (power, toughness, duration, condition) = parse_get_modifier_values_with_tail(
        get_tail_clause.from(1).tokens(),
        base_power,
        base_toughness,
    )?;
    if condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gets duration in creature-type choice pump clause (clause: '{}')",
            clause.text()
        )));
    }

    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_pump_all(filter, power, toughness, duration),
    ]))
}

pub(crate) fn parse_sentence_must_attack_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(MUST_ATTACK_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_must_attack_creature_type_of_choice_matched(clause, &matched)
}

pub(crate) fn parse_sentence_must_attack_creature_type_of_choice_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::effect::Until;

    let Some(subject_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Subject)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(subject_clause) else {
        return Ok(None);
    };
    if !subject_clause
        .from(choice_idx + consumed)
        .trimmed()
        .is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }
    let mut filter_clause = subject_clause.without_token_range_trimmed_clause(choice_idx, consumed);
    filter_clause.remove_leading_word("all");
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice attack subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }
    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_grant_abilities_all(
            filter,
            vec![crate::runtime_backend::GrantedAbilityAst::MustAttack],
            Until::EndOfTurn,
        ),
    ]))
}

pub(crate) fn parse_sentence_put_sticker_on(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(PUT_STICKER_ON_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_put_sticker_on_matched(clause, &matched)
}

pub(crate) fn parse_sentence_put_sticker_on_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(sticker_head) = clause
        .pattern_capture_role(matched, LexCaptureRole::Action)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if sticker_head.contains_no_words(&["sticker", "stickers"]) {
        return Ok(None);
    }
    let action = STICKER_KIND_PHRASES
        .iter()
        .find_map(|(phrase, action)| sticker_head.contains_phrase(phrase).then_some(*action))
        .unwrap_or(crate::events::KeywordActionKind::Sticker);

    let Some(target_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Object)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if target_clause.is_empty() {
        return Ok(None);
    }

    if target_reference_head_matches(target_clause) {
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![EffectAst::subject_verb_put_sticker(
            target, action,
        )]));
    }

    let mut filter = parse_object_filter(target_clause.tokens(), false)?;
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
    Ok(Some(vec![EffectAst::subject_verb_put_sticker(
        TargetAst::Object(filter, None, None),
        action,
    )]))
}

pub(crate) fn parse_sentence_return_targets_of_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(RETURN_TARGETS_OF_CREATURE_TYPE_OF_CHOICE_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_return_targets_of_creature_type_of_choice_matched(clause, &matched)
}

pub(crate) fn parse_sentence_return_targets_of_creature_type_of_choice_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(target_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Object)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let Some(destination_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Tail)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if destination_clause.contains_no_words(&["hand", "hands"]) {
        return Ok(None);
    }

    let inline_creature_choice = find_creature_type_choice_phrase(target_clause);
    let referenced_type_choice = if inline_creature_choice.is_none() {
        find_type_choice_phrase(target_clause)
    } else {
        None
    };
    if inline_creature_choice.is_none() && referenced_type_choice.is_none() {
        return Ok(None);
    }

    let (filter, needs_inline_choice_effect) =
        if let Some((choice_idx, consumed)) = inline_creature_choice {
            let base_filter_clause =
                target_clause.without_token_range_trimmed_clause(choice_idx, consumed);
            if base_filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return target before chosen-type qualifier (clause: '{}')",
                    clause.text()
                )));
            }
            let mut filter = parse_object_filter(base_filter_clause.tokens(), false)?;
            filter.chosen_creature_type = true;
            (filter, true)
        } else {
            let (choice_idx, consumed) = referenced_type_choice.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "type-choice return target must mention the chosen type (clause: '{}')",
                    clause.text()
                ))
            })?;
            let mut start_idx = choice_idx;
            let mut excluded = false;
            if choice_idx >= 2
                && target_clause
                    .token(choice_idx - 2)
                    .is_some_and(|token| token_is_word(token, THAT_WORD))
                && target_clause
                    .token(choice_idx - 1)
                    .is_some_and(|token| token_is_any_word(token, ARENT_WORDS))
            {
                start_idx = choice_idx - 2;
                excluded = true;
            } else if choice_idx >= 3
                && target_clause
                    .token(choice_idx - 3)
                    .is_some_and(|token| token_is_word(token, THAT_WORD))
                && target_clause
                    .token(choice_idx - 2)
                    .is_some_and(|token| token_is_word(token, ARE_WORD))
                && target_clause
                    .token(choice_idx - 1)
                    .is_some_and(|token| token_is_word(token, NOT_WORD))
            {
                start_idx = choice_idx - 3;
                excluded = true;
            } else if choice_idx >= 2
                && target_clause
                    .token(choice_idx - 2)
                    .is_some_and(|token| token_is_word(token, THAT_WORD))
                && target_clause
                    .token(choice_idx - 1)
                    .is_some_and(|token| token_is_word(token, ARE_WORD))
            {
                start_idx = choice_idx - 2;
            }

            let base_filter_clause = target_clause.without_token_ranges_trimmed_clause(&[
                (start_idx, choice_idx - start_idx),
                (choice_idx, consumed),
            ]);
            if base_filter_clause.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return target before chosen-type qualifier (clause: '{}')",
                    clause.text()
                )));
            }

            let mut filter = parse_object_filter(base_filter_clause.tokens(), false)?;
            if excluded {
                filter.excluded_chosen_creature_type = true;
            } else {
                filter.chosen_creature_type = true;
            }
            (filter, false)
        };

    // Check whether the target tokens (before the creature-type choice phrase)
    // mention "target". If so, we need to parse a proper TargetAst (which
    // captures targeting semantics and count such as X) rather than using a
    // mass-return-all filter.
    let has_target = literal_target_reference_matches(target_clause);

    let mut effects = Vec::new();
    if needs_inline_choice_effect {
        effects.push(EffectAst::subject_verb_choose_creature_type(
            PlayerAst::You,
            vec![],
        ));
    }

    if has_target {
        // Rebuild the base tokens (stripping the creature-type-of-choice phrase)
        // so that parse_target_phrase can extract count + "target" + filter.
        let base_target_clause = {
            if let Some((choice_idx, consumed)) = inline_creature_choice {
                target_clause.without_token_range_trimmed_clause(choice_idx, consumed)
            } else {
                let (choice_idx, consumed) = referenced_type_choice.unwrap();
                let mut start_idx = choice_idx;
                if choice_idx >= 2
                    && target_clause
                        .token(choice_idx - 2)
                        .is_some_and(|token| token_is_word(token, THAT_WORD))
                    && target_clause.token(choice_idx - 1).is_some_and(|token| {
                        token_is_any_word(token, ARENT_WORDS) || token_is_word(token, ARE_WORD)
                    })
                {
                    start_idx = choice_idx - 2;
                } else if choice_idx >= 3
                    && target_clause
                        .token(choice_idx - 3)
                        .is_some_and(|token| token_is_word(token, THAT_WORD))
                    && target_clause
                        .token(choice_idx - 2)
                        .is_some_and(|token| token_is_word(token, ARE_WORD))
                    && target_clause
                        .token(choice_idx - 1)
                        .is_some_and(|token| token_is_word(token, NOT_WORD))
                {
                    start_idx = choice_idx - 3;
                }
                target_clause.without_token_ranges_trimmed_clause(&[
                    (start_idx, choice_idx - start_idx),
                    (choice_idx, consumed),
                ])
            }
        };
        let mut target = parse_target_phrase(base_target_clause.tokens())?;
        // Recursively patch `chosen_creature_type` / `excluded_chosen_creature_type`
        // on the ObjectFilter buried inside the TargetAst (may be wrapped in WithCount).
        fn patch_chosen_type(t: &mut TargetAst, chosen: bool, excluded: bool) {
            match t {
                TargetAst::Object(f, _, _) => {
                    f.chosen_creature_type |= chosen;
                    f.excluded_chosen_creature_type |= excluded;
                }
                TargetAst::WithCount(inner, _) => patch_chosen_type(inner, chosen, excluded),
                _ => {}
            }
        }
        patch_chosen_type(
            &mut target,
            filter.chosen_creature_type,
            filter.excluded_chosen_creature_type,
        );
        effects.push(EffectAst::subject_verb_return_to_hand(target, false));
    } else {
        effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_text = clause.text();
    if !clause.starts_with_any(CHOOSE_ALL_OR_PUT_ALL_PREFIXES) {
        return Ok(None);
    }
    let starts_choose_all = clause.starts_with_any(CHOOSE_ALL_PREFIXES);

    let Some(from_idx) = clause.find_word("from") else {
        return Ok(None);
    };
    if from_idx <= 2 {
        return Ok(None);
    }
    let Some(zone_clause) = clause.from_word(from_idx) else {
        return Ok(None);
    };
    let Some(zone_pair) = parse_choose_all_source_zone_pair(zone_clause) else {
        return Ok(None);
    };

    let Some(filter_clause) = clause
        .after_words(2)
        .and_then(|tail| tail.before_word(from_idx - 2))
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_text
        )));
    }

    if starts_choose_all {
        let Some(put_idx) = clause.find_word("put") else {
            return Ok(None);
        };
        let Some(put_clause) = clause.from_word(put_idx) else {
            return Ok(None);
        };
        if !choose_all_put_clause_returns_them_to_hand(put_clause) {
            return Ok(None);
        }
    } else if !choose_all_clause_ends_in_hand_destination(clause) {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(filter_clause.tokens(), false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_text
        ))
    })?;
    base_filter.controller = None;

    let mut battlefield_filter = base_filter.clone();
    battlefield_filter.zone = Some(zone_pair[0]);

    let mut graveyard_filter = base_filter;
    graveyard_filter.zone = Some(zone_pair[1]);

    Ok(Some(vec![
        EffectAst::subject_verb_return_all_to_hand(battlefield_filter),
        EffectAst::subject_verb_return_all_to_hand(graveyard_filter),
    ]))
}

pub(crate) fn return_segment_mentions_zone(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    RETURN_SEGMENT_ZONE_PATTERN
        .find_in_clause(LexedClause::new(clause.tokens()))
        .is_some()
}

pub(crate) fn parse_sentence_return_multiple_targets(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(RETURN_MULTIPLE_TARGETS_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_return_multiple_targets_matched(clause, &matched)
}

pub(crate) fn parse_sentence_return_multiple_targets_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(targets_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Object)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };
    let Some(dest_clause) = clause
        .pattern_capture_role(matched, LexCaptureRole::Tail)
        .map(SubjectVerbPrimitiveClause::trimmed)
    else {
        return Ok(None);
    };

    let Some(destination) = parse_return_destination_shape(dest_clause) else {
        return Ok(None);
    };

    let has_multi_separator = targets_clause.contains_comma_or_any_word(&["and", "or", "and/or"]);
    if !has_multi_separator {
        return Ok(None);
    }

    let mut segments: Vec<SubjectVerbPrimitiveOwnedClause> = Vec::new();
    for segment_clause in targets_clause.trimmed_and_comma_segments() {
        let starts_new_target = return_segment_head_matches(segment_clause);
        let mentions_target = literal_target_reference_matches(segment_clause);
        let starts_like_zone_suffix = zone_suffix_head_matches(segment_clause);
        if !segments.is_empty()
            && !starts_new_target
            && !mentions_target
            && !starts_like_zone_suffix
        {
            let last = segments.last_mut().expect("segments is non-empty");
            last.append_comma_then(segment_clause);
        } else {
            segments.push(SubjectVerbPrimitiveOwnedClause::from_clause(segment_clause));
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(|segment| all_or_each_head_word(segment.as_clause()));

    let shared_suffix = segments
        .last()
        .and_then(|segment| {
            segment
                .find_token_word("from")
                .map(|idx| segment.from_tokens(idx).to_vec())
        })
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        if !return_segment_mentions_zone(segment.as_clause()) && !shared_suffix.is_empty() {
            segment.extend_from_slice(&shared_suffix);
        }
        if let Some(quantifier) = shared_quantifier.as_deref() {
            let has_explicit_quantifier = all_or_each_head_matches(segment.as_clause());
            let starts_like_target_reference =
                broad_target_reference_head_matches(segment.as_clause());
            if !has_explicit_quantifier
                && !starts_like_target_reference
                && !literal_target_reference_matches(segment.as_clause())
            {
                segment.insert_leading_word(quantifier);
            }
        }
        if all_or_each_head_matches(segment.as_clause()) {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    clause.text()
                )));
            }
            let filter = parse_object_filter(segment.from_tokens(1), false)?;
            if destination.zone == Zone::Battlefield {
                effects.push(EffectAst::subject_verb_return_all_to_battlefield(
                    filter,
                    destination.tapped,
                    false,
                    ReturnControllerAst::Owner,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
            }
        } else {
            let target = parse_target_phrase(segment.tokens())?;
            if destination.zone == Zone::Battlefield {
                effects.push(EffectAst::subject_verb_return_to_battlefield(
                    target,
                    destination.tapped,
                    false,
                    false,
                    ReturnControllerAst::Preserve,
                    None,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_to_hand(target, false));
            }
        }
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_for_each_of_target_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(FOR_EACH_TARGET_OBJECTS_PATTERN_ATOMS);
    if clause.match_pattern(pattern).is_none() {
        return Ok(None);
    }

    let Some((subject_clause, effect_clause)) = clause.split_once_on_comma() else {
        return Ok(None);
    };

    let subject_clause = subject_clause.trimmed();
    let Some((mut filter, count)) =
        parse_for_each_targeted_object_subject(subject_clause.tokens())?
    else {
        return Ok(None);
    };
    if filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.tagged_constraints.is_empty()
    {
        // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects
        // compilation for plain "target permanent(s)" clauses.
        filter.controller = Some(PlayerFilter::Any);
    }

    let effect_clause = effect_clause.trimmed();
    if effect_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after for-each target subject (clause: '{}')",
            clause.text()
        )));
    }
    let mut per_target_effects = parse_effect_chain(effect_clause.tokens())?;
    for effect in &mut per_target_effects {
        bind_implicit_player_context(effect, PlayerAst::You);
    }
    if per_target_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "for-each target follow-up produced no effects (clause: '{}')",
            clause.text()
        )));
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player: PlayerAst::Implicit,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: per_target_effects,
        },
    ]))
}

pub(crate) fn parse_distribute_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    if !clause
        .first_word()
        .is_some_and(|word| word == DISTRIBUTE_WORD)
    {
        return Ok(None);
    }

    let amount_clause = clause.from(1);
    let (count, used) = parse_number(amount_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed counter amount (clause: '{}')",
            clause.text()
        ))
    })?;
    let rest_clause = clause.from(1 + used);
    let counter_type = parse_counter_type_from_tokens(rest_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause.text()
        ))
    })?;
    let Some((_before_among, target_clause)) = rest_clause.split_once_on_word("among") else {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target clause after 'among' (clause: '{}')",
            clause.text()
        )));
    };
    let target_clause = target_clause.trimmed();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause.text()
        )));
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(target_clause.tokens())?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause.text()
            ))
        })?;
    let target_phrase = target_clause.from(used_count).trimmed();
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause.text()
        )));
    }
    let target = parse_target_phrase(target_phrase.tokens())?;

    Ok(Some(EffectAst::subject_verb_put_counters(
        counter_type,
        Value::Fixed(count as i32),
        target,
        Some(target_count),
        true,
    )))
}

pub(crate) fn parse_sentence_distribute_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let Some(primary) = parse_distribute_counters_sentence(head_clause)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
        effects.extend(parse_effect_chain(tail_clause.tokens())?);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_transform_with_followup(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = clause.token(0) else {
        return Ok(None);
    };
    let is_transform = token_is_word(first, TRANSFORM_WORD);
    let is_convert = token_is_word(first, CONVERT_WORD);
    if !is_transform && !is_convert {
        return Ok(None);
    }

    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let target_clause = head_clause.from(1).trimmed();
    let transform = if is_transform {
        parse_transform(target_clause.tokens())?
    } else {
        parse_convert(target_clause.tokens())?
    };
    let Some(tail_clause) = tail_clause else {
        return Ok(Some(vec![transform]));
    };
    if tail_clause.is_empty() {
        return Ok(Some(vec![transform]));
    }

    let mut effects = vec![transform];
    effects.extend(parse_effect_chain(tail_clause.tokens())?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_cant_effect(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_cant_effect_sentence)
}

pub(crate) fn parse_sentence_gain_x_plus_life(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_x_plus_life_sentence)
}

pub(crate) fn parse_sentence_for_each_exiled_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(FOR_EACH_THIS_WAY_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_for_each_exiled_this_way_matched(clause, &matched)
}

pub(crate) fn parse_sentence_for_each_exiled_this_way_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(body_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object) else {
        return Ok(None);
    };
    let Some(effect_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if body_clause.is_empty() || effect_clause.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_exiled_this_way_sentence)
}

pub(crate) fn parse_sentence_for_each_put_into_graveyard_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(FOR_EACH_THIS_WAY_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_for_each_put_into_graveyard_this_way_matched(clause, &matched)
}

pub(crate) fn parse_sentence_for_each_put_into_graveyard_this_way_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(body_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object) else {
        return Ok(None);
    };
    let Some(effect_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if body_clause.is_empty() || effect_clause.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_put_into_graveyard_this_way_sentence)
}

pub(crate) fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_each_player_put_permanent_cards_exiled_with_source_sentence)
}

pub(crate) fn parse_sentence_for_each_destroyed_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let pattern = LexPattern::new(FOR_EACH_THIS_WAY_PATTERN_ATOMS);
    let Some(matched) = clause.match_pattern(pattern) else {
        return Ok(None);
    };
    parse_sentence_for_each_destroyed_this_way_matched(clause, &matched)
}

pub(crate) fn parse_sentence_for_each_destroyed_this_way_matched(
    clause: SubjectVerbPrimitiveClause<'_>,
    matched: &LexPatternMatch<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(body_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Object) else {
        return Ok(None);
    };
    let Some(effect_clause) = clause.pattern_capture_role(matched, LexCaptureRole::Tail) else {
        return Ok(None);
    };
    if body_clause.is_empty() || effect_clause.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_destroyed_this_way_sentence)
}

pub(crate) fn parse_sentence_search_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_search_library_sentence)
}

pub(crate) fn parse_sentence_shuffle_graveyard_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_graveyard_into_library_sentence)
}

pub(crate) fn parse_sentence_shuffle_object_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_object_into_library_sentence)
}

pub(crate) fn parse_sentence_exile_hand_and_graveyard_bundle(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_exile_hand_and_graveyard_bundle_sentence)
}

pub(crate) fn parse_sentence_target_player_exiles_creature_and_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_target_player_exiles_creature_and_graveyard_sentence)
}

pub(crate) fn parse_sentence_look_at_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_hand_sentence)
}

pub(crate) fn parse_sentence_look_at_top_then_exile_one(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_top_then_exile_one_sentence)
}

pub(crate) fn parse_sentence_gain_life_equal_to_age(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_life_equal_to_age_sentence)
}

pub(crate) fn parse_sentence_for_each_player_doesnt(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_one_with_lexed(parse_for_each_player_doesnt)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DelayedNextStepKind {
    Upkeep,
    DrawStep,
}

pub(super) fn delayed_next_step_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize, DelayedNextStepKind, PlayerAst)> {
    let patterns: &[(&[&str], DelayedNextStepKind, PlayerAst)] = &[
        (
            &["at", "the", "beginning", "of", "your", "next", "upkeep"],
            DelayedNextStepKind::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::You,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "your",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::You,
        ),
        (
            &["at", "the", "beginning", "of", "their", "next", "upkeep"],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "their",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "upkeep",
                "step",
            ],
            DelayedNextStepKind::Upkeep,
            PlayerAst::That,
        ),
        (
            &[
                "at",
                "the",
                "beginning",
                "of",
                "that",
                "players",
                "next",
                "draw",
                "step",
            ],
            DelayedNextStepKind::DrawStep,
            PlayerAst::That,
        ),
    ];

    for (pattern, step, player) in patterns {
        if let Some(start) = clause.find_phrase_start(pattern) {
            return Some((start, start + pattern.len(), *step, *player));
        }
    }

    None
}

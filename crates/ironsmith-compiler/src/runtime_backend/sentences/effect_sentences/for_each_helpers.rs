use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, OwnedLexToken, PlayerAst, PredicateAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::diagnostics::TextSpan;
use crate::effect::{Until, Value};
use crate::target::{ObjectFilter, PlayerFilter};

use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::primitives as grammar;
use super::super::keyword_static::{
    parse_pt_modifier, parse_pt_modifier_values, parse_value_binding_clause,
};
use super::super::lexer::{LexedClause, contains_token_word, token_word_refs};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::find_index as find_token_index;
use super::super::util::{
    contains_until_end_of_turn, is_until_end_of_turn,
    parse_choice_or_range_count_token_prefix_consumed, parse_for_each_count_value_words,
    parse_greater_than_or_equal_quantity_prefix, parse_number, parse_target_phrase, parse_value,
    replace_unbound_x_with_value, starts_with_until_end_of_turn, value_contains_unbound_x,
};
use super::chain_carry::bind_implicit_player_context;
use super::chain_carry::{parse_effect_chain, parse_effect_chain_inner, remove_first_word};
use super::clause_pattern_helpers::{ClauseShape, clause_shape};
use super::conditionals::negated_action_word_index;
use super::{Verb, find_verb};

const FOR_EACH_LIFE_TOTAL_BECOMES_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["life", "total", "becomes"]);
const FOR_EACH_TO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["to"]);
const FOR_EACH_X_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["x"]);
const FOR_EACH_OF_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["of"]);
const FOR_EACH_TARGET_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["target", "player"], &["target", "players"]]);
const FOR_EACH_ATTACHED_TO_CREATURE_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["creature"], &["a", "creature"]]);
const FOR_EACH_THIS_TURN_DAMAGE_BY_THIS_CREATURE_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases
        & [
            &["dealt", "damage", "by", "this", "creature", "this"],
            &["this", "turn"],
        ]
);
const FOR_EACH_MANA_REPLACEMENT_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [&["for", "mana"]];
    contains_words & ["if", "instead"];
    contains_any_words & [&["tap", "taps"], &["produce", "produces"]]
);
const FOR_EACH_MANA_TRIGGER_ADDITIONAL_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_phrases & [&["for", "mana"]];
    contains_words & ["whenever", "additional"];
    contains_any_words & [&["tap", "taps"], &["add", "adds"]]
);
const FOR_EACH_BASE_POWER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["base", "power"]);
const FOR_EACH_BASE_POWER_TOUGHNESS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["base", "power", "and", "toughness"]);
const FOR_EACH_UNTIL_YOUR_NEXT_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["until", "your", "next", "turn"]);
const FOR_EACH_INSTEAD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["instead"]);
const FOR_EACH_MUST_BE_BLOCKED_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and", "must", "be", "blocked", "this", "turn", "if", "able"]);
const FOR_EACH_CANT_BE_BLOCKED_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["and", "cant", "be", "blocked", "this", "turn"]);
const FOR_EACH_MAY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["may"]);
const FOR_EACH_AND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["and"]);
const FOR_EACH_OR_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["or"]);
const FOR_EACH_GAIN_HAS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"], &["has"], &["have"]]);
const FOR_EACH_COMBAT_KEYWORD_MARKER_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words
        & [
            &["trample"],
            &["haste"],
            &["first"],
            &["strike"],
            &["infect"]
        ]
);
const FOR_EACH_CONTROL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["control"], &["controls"]]);
const FOR_EACH_CHOOSE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["choose"]);
const FOR_EACH_THEN_RETURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["then", "return"]);
const FOR_EACH_UNLESS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["unless"]);
const FOR_EACH_CONTROLLER_DRAW_TAIL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["its", "controller", "has", "you", "draw", "a"]);
const FOR_EACH_CARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["card"]);
const FOR_EACH_WHO_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["who"]);
const FOR_EACH_HAS_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["has"]);
const FOR_EACH_THIS_WAY_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["this", "way"]);
const FOR_EACH_POISON_COUNTERS_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["poison", "counter"], &["poison", "counters"]]);
const FOR_EACH_WHO_CONTROLS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["who", "controls"]);
const FOR_EACH_MOST_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["the", "most"], &["most"]]);
const FOR_EACH_THEN_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["then"]);
const FOR_EACH_TEMPORAL_TURN_MARKER_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["this", "turn"], &["next", "turn"],]]);
const FOR_EACH_WHERE_X_IS_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["where", "x", "is"]);
const FOR_EACH_OTHER_THAN_DEFENDING_PLAYER_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["other", "than", "defending", "player"]);
const FOR_EACH_TAPPED_A_LAND_FOR_MANA_THIS_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["tapped", "a", "land", "for", "mana", "this", "turn"]);
const FOR_EACH_TAPPED_LAND_FOR_MANA_THIS_TURN_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["tapped", "land", "for", "mana", "this", "turn"]);
const FOR_EACH_SCRY_SURVEIL_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["scries"], &["scry"], &["surveils"], &["surveil"]]);
const FOR_EACH_TARGET_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["target"]);
const FOR_EACH_PLAYER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["player"], &["players"]]);
const FOR_EACH_EACH_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["each"]);
const FOR_EACH_TAGGED_ACTION_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["sacrificed"], &["destroyed"], &["exiled"], &["discarded"]]);
const FOR_EACH_DISCARD_OR_DISCARDED_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["discard"], &["discarded"]]);

fn for_each_words_start_with_who(words: &[&str]) -> bool {
    words
        .first()
        .is_some_and(|word| FOR_EACH_WHO_WORD_PATTERN.matches_word(word))
}

fn for_each_this_way_index(words: &[&str]) -> Option<usize> {
    FOR_EACH_THIS_WAY_PATTERN.find_exact_window(words, 2)
}

fn for_each_strip_prefix_shape_clause<'a>(
    clause: LexedClause<'a>,
    shape: &ClauseShape<'static>,
) -> Option<LexedClause<'a>> {
    let words = clause.word_refs();
    let prefix_len = shape.matched_prefix_len(&words)?;
    let token_idx = clause.token_index_for_word_or_end(prefix_len)?;
    Some(clause.from(token_idx))
}

fn trimmed_tail_from_word(clause: LexedClause<'_>, word_idx: usize) -> LexedClause<'_> {
    let token_idx = clause
        .token_index_for_word_or_end(word_idx)
        .unwrap_or(clause.len());
    clause.from(token_idx).trimmed()
}

fn prepend_that_player_life_total_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words = token_word_refs(tokens);
    if !FOR_EACH_LIFE_TOTAL_BECOMES_PATTERN.matches_words(&words) {
        return tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(tokens.len() + 2);
    rewritten.push(OwnedLexToken::word(
        "that".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.push(OwnedLexToken::word(
        "players".to_string(),
        TextSpan::synthetic(),
    ));
    rewritten.extend_from_slice(tokens);
    rewritten
}

const PLAYER_OR_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["player"],
    &["players"],
    &["opponent"],
    &["opponents"],
    &["other", "player"],
    &["other", "players"],
    &["target", "player"],
    &["target", "players"],
    &["target", "opponent"],
    &["target", "opponents"],
];

const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"], &["each"]];
const FOR_EACH_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["for", "each", "opponent"],
    &["for", "each", "opponents"],
    &["each", "opponent"],
    &["each", "opponents"],
];
const FOR_EACH_PLAYER_PREFIXES: &[&[&str]] = &[
    &["for", "each", "player"],
    &["for", "each", "players"],
    &["each", "player"],
    &["each", "players"],
];
const WHO_ACTION_PREFIXES: &[&[&str]] = &[&["who", "does"], &["who", "do"], &["who", "did"]];
const INSTEAD_IF_PREFIXES: &[&[&str]] = &[&["instead", "if"]];
const FOR_AS_LONG_AS_PREFIXES: &[&[&str]] = &[&["for", "as", "long", "as"]];
const DEMONSTRATIVE_OBJECT_REFERENCE_PHRASES: &[&[&str]] = &[
    &["that", "creature"],
    &["that", "creatures"],
    &["that", "permanent"],
    &["that", "permanents"],
    &["that", "artifact"],
    &["that", "artifacts"],
    &["that", "enchantment"],
    &["that", "enchantments"],
    &["that", "land"],
    &["that", "lands"],
    &["that", "card"],
    &["that", "cards"],
    &["that", "token"],
    &["that", "tokens"],
    &["that", "spell"],
    &["that", "spells"],
    &["those", "creatures"],
    &["those", "permanents"],
    &["those", "artifacts"],
    &["those", "enchantments"],
    &["those", "lands"],
    &["those", "cards"],
    &["those", "tokens"],
    &["those", "spells"],
];
const DEMONSTRATIVE_OBJECT_REFERENCE_PHRASE_GROUPS: &[&[&[&str]]] =
    &[DEMONSTRATIVE_OBJECT_REFERENCE_PHRASES];
const FOR_EACH_DEMONSTRATIVE_OBJECT_REFERENCE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & DEMONSTRATIVE_OBJECT_REFERENCE_PHRASE_GROUPS);

fn find_tapped_land_for_mana_this_turn_end(words: &[&str]) -> Option<usize> {
    FOR_EACH_TAPPED_A_LAND_FOR_MANA_THIS_TURN_PATTERN
        .find_exact_window(words, 7)
        .map(|idx| idx + 6)
        .or_else(|| {
            FOR_EACH_TAPPED_LAND_FOR_MANA_THIS_TURN_PATTERN
                .find_exact_window(words, 6)
                .map(|idx| idx + 5)
        })
}

pub(crate) fn parse_for_each_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let subject_clause = LexedClause::new(subject_tokens);
    if subject_clause.is_empty() {
        return Ok(None);
    }

    let Some((_, mut filter_clause)) = subject_clause.strip_any_prefix_clause(FOR_EACH_PREFIXES)
    else {
        return Ok(None);
    };
    if let Some(after_of) =
        for_each_strip_prefix_shape_clause(filter_clause, &FOR_EACH_OF_PREFIX_PATTERN)
    {
        filter_clause = after_of;
    }
    if filter_clause.is_empty() {
        return Ok(None);
    }

    let mut normalized_filter_clause = filter_clause;
    if let Some(attached_idx) = filter_clause.find_token_word("attached")
        && filter_clause
            .tokens()
            .get(attached_idx + 1)
            .is_some_and(|token| FOR_EACH_TO_WORD_PATTERN.matches_token(token))
        && attached_idx > 0
    {
        let attached_tail_clause = filter_clause.from(attached_idx + 2);
        let attached_to_creature = FOR_EACH_ATTACHED_TO_CREATURE_TAIL_PATTERN
            .matches_words(&attached_tail_clause.word_refs());
        if attached_to_creature {
            normalized_filter_clause = filter_clause.before(attached_idx).trimmed();
        }
    }

    if normalized_filter_clause.is_empty() {
        return Ok(None);
    }

    if normalized_filter_clause.starts_with_any(PLAYER_OR_OPPONENT_PREFIXES) {
        return Ok(None);
    }

    Ok(Some(parse_object_filter(
        normalized_filter_clause.tokens(),
        false,
    )?))
}

pub(crate) fn parse_for_each_targeted_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let subject_clause = LexedClause::new(subject_tokens);
    if subject_clause.is_empty() {
        return Ok(None);
    }

    let Some((_, mut target_clause)) = subject_clause.strip_any_prefix_clause(FOR_EACH_PREFIXES)
    else {
        return Ok(None);
    };
    if let Some(after_of) =
        for_each_strip_prefix_shape_clause(target_clause, &FOR_EACH_OF_PREFIX_PATTERN)
    {
        target_clause = after_of;
    }
    if target_clause.is_empty() {
        return Ok(None);
    }

    let target = match parse_target_phrase(target_clause.tokens()) {
        Ok(target) => target,
        Err(_) => return Ok(None),
    };
    let TargetAst::WithCount(inner, count) = target else {
        return Ok(None);
    };
    let TargetAst::Object(filter, _, _) = *inner else {
        return Ok(None);
    };
    Ok(Some((filter, count)))
}

pub(crate) fn has_demonstrative_object_reference(words: &[&str]) -> bool {
    FOR_EACH_DEMONSTRATIVE_OBJECT_REFERENCE_PATTERN.matches_words(words)
}

pub(crate) fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    if words.len() < 8 {
        return false;
    }
    if !FOR_EACH_TARGET_PLAYER_PREFIX_PATTERN.matches_words(words) {
        return false;
    }
    FOR_EACH_THIS_TURN_DAMAGE_BY_THIS_CREATURE_PATTERN.matches_words(words)
}

pub(crate) fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    FOR_EACH_MANA_REPLACEMENT_PATTERN.matches_words(words)
}

pub(crate) fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    FOR_EACH_MANA_TRIGGER_ADDITIONAL_PATTERN.matches_words(words)
}

pub(crate) fn parse_has_base_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words_all = clause.word_refs();
    let Some(has_idx) = clause.find_word_any(&["has", "have"]) else {
        return Ok(None);
    };
    let Some(subject_clause) = clause.before_word(has_idx) else {
        return Ok(None);
    };
    if subject_clause.is_empty() {
        return Ok(None);
    }
    let subject_words = subject_clause.word_refs();

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 3 || !FOR_EACH_BASE_POWER_PREFIX_PATTERN.matches_words(rest_words) {
        return Ok(None);
    }
    if FOR_EACH_AND_WORD_PATTERN.matches_word_at(rest_words, 2) {
        return Ok(None);
    }

    let has_token_idx = clause.token_index_for_word_index(has_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing has/have token in base-power clause (clause: '{}')",
            words_all.join(" ")
        ))
    })?;
    let rest_tokens = &tokens[has_token_idx + 1..];

    let mut seen_words = 0usize;
    let mut value_token_idx = None;
    for (idx, token) in rest_tokens.iter().enumerate() {
        if token.as_word().is_some() {
            seen_words += 1;
            if seen_words == 3 {
                value_token_idx = Some(idx);
                break;
            }
        }
    }
    let Some(value_token_idx) = value_token_idx else {
        return Ok(None);
    };
    let (power, value_used) = parse_value(&rest_tokens[value_token_idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid base power value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let tail_words: Vec<&str> = rest_tokens[value_token_idx + value_used..]
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect();
    if tail_words.is_empty() {
        let has_target_subject = subject_clause.contains_word("target");
        let has_leading_until_eot = starts_with_until_end_of_turn(&subject_words);
        let has_temporal_words = contains_until_end_of_turn(&words_all)
            || FOR_EACH_TEMPORAL_TURN_MARKER_PATTERN.matches(clause);
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    } else if !is_until_end_of_turn(tail_words.as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_clause = if starts_with_until_end_of_turn(&subject_words) {
        let mut skip_idx = 4usize;
        if subject_clause
            .tokens()
            .get(skip_idx)
            .is_some_and(|token| token.is_comma())
        {
            skip_idx += 1;
        }
        subject_clause.from(skip_idx).trimmed()
    } else {
        subject_clause
    };
    let target = parse_target_phrase(target_clause.tokens())?;
    Ok(Some(EffectAst::subject_verb_set_base_power(
        power,
        target,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_has_base_power_toughness_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let words_all = clause.word_refs();
    let Some(has_idx) = clause.find_word_any(&["has", "have"]) else {
        return Ok(None);
    };
    let Some(subject_clause) = clause.before_word(has_idx) else {
        return Ok(None);
    };
    if subject_clause.is_empty() {
        return Ok(None);
    }
    let subject_words = subject_clause.word_refs();

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5
        || !FOR_EACH_BASE_POWER_TOUGHNESS_PREFIX_PATTERN.matches_words(rest_words)
    {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let tail = &rest_words[5..];
    if tail.is_empty() {
        let has_target_subject = subject_clause.contains_word("target");
        let has_leading_until_eot = starts_with_until_end_of_turn(&subject_words);
        let has_temporal_words = contains_until_end_of_turn(&words_all)
            || FOR_EACH_TEMPORAL_TURN_MARKER_PATTERN.matches(clause);
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    }
    let is_shared_gain_tail = matches!(
        tail,
        ["until", "end", "of", "turn", "and", "gain", ..]
            | ["until", "end", "of", "turn", "and", "gains", ..]
            | ["until", "end", "of", "turn", "and", "lose", ..]
            | ["until", "end", "of", "turn", "and", "loses", ..]
    );
    if is_shared_gain_tail {
        return Ok(None);
    }
    if !tail.is_empty() && !is_until_end_of_turn(tail) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_clause = if starts_with_until_end_of_turn(&subject_words) {
        let mut skip_idx = 4usize;
        if subject_clause
            .tokens()
            .get(skip_idx)
            .is_some_and(|token| token.is_comma())
        {
            skip_idx += 1;
        }
        subject_clause.from(skip_idx).trimmed()
    } else {
        subject_clause
    };
    let target = parse_target_phrase(target_clause.tokens())?;
    Ok(Some(EffectAst::subject_verb_set_base_power_toughness(
        Value::Fixed(power),
        Value::Fixed(toughness),
        target,
        Until::EndOfTurn,
    )))
}

pub(crate) fn parse_get_for_each_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let clause = LexedClause::new(tokens);
    if !clause.starts_with_any(FOR_EACH_PREFIXES) {
        return Ok(None);
    }
    let words = clause.word_refs();
    let Some((value, _used_words)) = parse_for_each_count_value_words(&words) else {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    };
    Ok(Some(value))
}

pub(crate) fn parse_get_modifier_values_with_tail(
    modifier_tokens: &[OwnedLexToken],
    power: Value,
    toughness: Value,
) -> Result<(Value, Value, Until, Option<crate::ConditionExpr>), CardTextError> {
    let modifier_clause = LexedClause::new(modifier_tokens);
    let clause = modifier_clause.text();
    let mut out_power = power;
    let mut out_toughness = toughness;
    let mut duration = Until::EndOfTurn;
    let mut condition = None;

    if modifier_clause.is_empty() {
        return Ok((out_power, out_toughness, duration, condition));
    }

    let after_modifier_clause = modifier_clause.from(1);
    let after_modifier_words = after_modifier_clause.word_refs();
    let until_word_count = if starts_with_until_end_of_turn(&after_modifier_words) {
        duration = Until::EndOfTurn;
        4usize
    } else if let Some((phrase, _tail)) = after_modifier_clause.strip_any_prefix_clause(&[
        &["until", "your", "next", "turn"],
        &["until", "end", "of", "combat"],
    ]) {
        duration = if FOR_EACH_UNTIL_YOUR_NEXT_TURN_PATTERN.matches_words(phrase) {
            Until::YourNextTurn
        } else {
            Until::EndOfCombat
        };
        4usize
    } else {
        0usize
    };
    let tail_start = after_modifier_clause
        .token_index_for_word_index(until_word_count)
        .unwrap_or(after_modifier_clause.tokens().len());
    let tail_clause = after_modifier_clause.from(tail_start).trimmed();

    if tail_clause.is_empty() {
        return Ok((out_power, out_toughness, duration, condition));
    }

    let tail_words = tail_clause.word_refs();
    if FOR_EACH_INSTEAD_PATTERN.matches_words(&tail_words) {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_clause
        .strip_any_prefix_clause(INSTEAD_IF_PREFIXES)
        .is_some()
    {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_clause
        .strip_any_prefix_clause(FOR_AS_LONG_AS_PREFIXES)
        .is_some()
        && tail_clause.contains_word("this")
        && tail_clause.contains_word("remains")
        && tail_clause.contains_word("tapped")
    {
        condition = Some(crate::ConditionExpr::SourceIsTapped);
        return Ok((
            out_power,
            out_toughness,
            Until::SourceUntaps,
            condition,
        ));
    }
    if FOR_EACH_MUST_BE_BLOCKED_TAIL_PATTERN.matches_words(&tail_words) {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if FOR_EACH_CANT_BE_BLOCKED_TAIL_PATTERN.matches_words(&tail_words) {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words
        .first()
        .is_some_and(|word| FOR_EACH_AND_WORD_PATTERN.matches_word(word))
        && FOR_EACH_GAIN_HAS_WORD_PATTERN.matches_word_at(&tail_words, 1)
        && FOR_EACH_COMBAT_KEYWORD_MARKER_PATTERN.matches_words(&tail_words)
        && contains_until_end_of_turn(&tail_words)
    {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words
        .first()
        .is_some_and(|word| FOR_EACH_AND_WORD_PATTERN.matches_word(word))
        && tail_words
            .iter()
            .any(|word| FOR_EACH_GAIN_HAS_WORD_PATTERN.matches_word(word))
        && FOR_EACH_COMBAT_KEYWORD_MARKER_PATTERN.matches_words(&tail_words)
        && contains_until_end_of_turn(&tail_words)
    {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words
        .first()
        .is_some_and(|word| FOR_EACH_AND_WORD_PATTERN.matches_word(word))
        && tail_words
            .iter()
            .any(|word| FOR_EACH_CONTROL_WORD_PATTERN.matches_word(word))
    {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words
        .first()
        .is_some_and(|word| FOR_EACH_OR_WORD_PATTERN.matches_word(word))
        && let Some(alt_mod) = tail_words.get(1).copied()
        && parse_pt_modifier_values(alt_mod).is_ok()
    {
        let alt_tail = &tail_words[2..];
        if alt_tail.is_empty() || is_until_end_of_turn(alt_tail) {
            return Ok((out_power, out_toughness, duration, condition));
        }
    }
    if tail_clause
        .strip_any_prefix_clause(FOR_EACH_PREFIXES)
        .is_some()
        && let Some(count) = parse_get_for_each_count_value(tail_clause.tokens())?
    {
        let scale_modifier = |modifier: Value| -> Result<Value, CardTextError> {
            match modifier {
                Value::Fixed(0) => Ok(Value::Fixed(0)),
                Value::Fixed(1) => Ok(count.clone()),
                Value::Fixed(multiplier) => Ok(Value::Scaled(Box::new(count.clone()), multiplier)),
                other if value_contains_unbound_x(&other) => {
                    replace_unbound_x_with_value(other, &count, &clause)
                }
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported dynamic gets-for-each clause (clause: '{}')",
                    clause
                ))),
            }
        };
        out_power = scale_modifier(out_power)?;
        out_toughness = scale_modifier(out_toughness)?;
        return Ok((out_power, out_toughness, duration, condition));
    }
    if !FOR_EACH_WHERE_X_IS_PREFIX_PATTERN.matches_words(&tail_clause.word_refs()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing gets clause (clause: '{}')",
            clause
        )));
    }

    if !value_contains_unbound_x(&out_power) && !value_contains_unbound_x(&out_toughness) {
        return Err(CardTextError::ParseError(format!(
            "where-X gets clause missing X modifier (clause: '{}')",
            clause
        )));
    }

    let x_value = parse_value_binding_clause(tail_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-X gets clause (clause: '{}')",
            clause
        ))
    })?;
    out_power = replace_unbound_x_with_value(out_power, &x_value, &clause)?;
    out_toughness = replace_unbound_x_with_value(out_toughness, &x_value, &clause)?;

    Ok((out_power, out_toughness, duration, condition))
}

pub(crate) fn force_implicit_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenWithMods { player, .. }
                    | SubjectVerbActionAst::CreateTokenCopy { player, .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { player, .. },
                ..
            }) => {
                if matches!(*player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                force_implicit_token_controller_you(nested);
            }),
        }
    }
}

pub(crate) fn parse_for_each_opponent_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause = LexedClause::new(tokens);
    if let Some(after_then) =
        for_each_strip_prefix_shape_clause(clause, &FOR_EACH_THEN_PREFIX_PATTERN)
    {
        clause = after_then;
    }
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let Some((_, after_prefix_clause)) = clause.strip_any_prefix_clause(FOR_EACH_OPPONENT_PREFIXES)
    else {
        return Ok(None);
    };

    let mut inner_clause = after_prefix_clause.trimmed();
    let mut iteration_filter = PlayerFilter::Opponent;
    if let Some(after_defending_player) = for_each_strip_prefix_shape_clause(
        inner_clause,
        &FOR_EACH_OTHER_THAN_DEFENDING_PLAYER_PREFIX_PATTERN,
    ) {
        inner_clause = after_defending_player.trimmed();
        iteration_filter = PlayerFilter::excluding(PlayerFilter::Opponent, PlayerFilter::Defending);
    }
    let wrap_for_each = |effects: Vec<EffectAst>| {
        if iteration_filter == PlayerFilter::Opponent {
            EffectAst::ForEachOpponent { effects }
        } else {
            EffectAst::ForEachPlayersFiltered {
                filter: iteration_filter.clone(),
                effects,
            }
        }
    };
    let inner_tokens = inner_clause.tokens();
    let inner_words = inner_clause.word_refs();
    if find_verb(inner_tokens).is_some_and(|(verb, idx)| {
        idx == 0
            && matches!(verb, Verb::Scry | Verb::Surveil)
            && inner_words
                .split_first()
                .filter(|(word, _)| FOR_EACH_SCRY_SURVEIL_WORD_PATTERN.matches_word(word))
                .is_some_and(|(_, rest)| {
                    rest.len() == 1 && !FOR_EACH_X_WORD_PATTERN.matches_words(rest)
                })
    }) {
        return Ok(None);
    }
    if inner_words
        .first()
        .is_some_and(|word| FOR_EACH_CHOOSE_WORD_PATTERN.matches_word(word))
        && let Some(then_return_idx) =
            FOR_EACH_THEN_RETURN_PATTERN.find_exact_window(&inner_words, 2)
        && let Some(unless_idx) = inner_words
            .iter()
            .position(|word| FOR_EACH_UNLESS_WORD_PATTERN.matches_word(word))
        && unless_idx > then_return_idx
        && inner_words
            .get(unless_idx + 1..unless_idx + 7)
            .is_some_and(|words| FOR_EACH_CONTROLLER_DRAW_TAIL_PATTERN.matches_words(words))
        && FOR_EACH_CARD_WORD_PATTERN.matches_word_at(&inner_words, unless_idx + 7)
    {
        let target_token_end = inner_clause
            .token_index_for_word_or_end(then_return_idx)
            .unwrap_or(inner_clause.len());
        let target_tokens = LexedClause::new(&inner_tokens[1..target_token_end]).trim();
        let target = parse_target_phrase(&target_tokens)?;
        let return_target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
        return Ok(Some(wrap_for_each(vec![
            EffectAst::subject_verb_target_only(target),
            EffectAst::UnlessAction {
                effects: vec![EffectAst::subject_verb_return_to_hand(return_target, false)],
                alternative: vec![EffectAst::subject_verb(
                    SubjectVerbRoleAst::AffectedPlayer,
                    PlayerAst::You,
                    SubjectVerbActionAst::Draw {
                        count: Value::Fixed(1),
                    },
                )],
                player: PlayerAst::ItsController,
            },
        ])));
    }
    if let Some(after_who) = grammar::words_match_prefix(
        &inner_tokens,
        &["who", "has", "less", "life", "than", "you"],
    ) {
        let effect_tokens = LexedClause::new(after_who).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who has less life than you' (clause: '{}')",
                clause_text
            )));
        }
        let mut branch_effects = if contains_token_word(&effect_tokens, "may") {
            let stripped = remove_first_word(&effect_tokens, "may");
            let inner_effects = parse_effect_chain_inner(&stripped)?;
            vec![EffectAst::May {
                effects: inner_effects,
            }]
        } else {
            parse_effect_chain(&effect_tokens)?
        };
        force_implicit_token_controller_you(&mut branch_effects);
        return Ok(Some(wrap_for_each(vec![EffectAst::Conditional {
            predicate: PredicateAst::PlayerHasLessLifeThanYou {
                player: PlayerAst::That,
            },
            if_true: branch_effects,
            if_false: Vec::new(),
        }])));
    }
    if inner_words.len() >= 7
        && inner_words
            .first()
            .is_some_and(|word| FOR_EACH_WHO_WORD_PATTERN.matches_word(word))
        && FOR_EACH_HAS_WORD_PATTERN.matches_word_at(&inner_words, 1)
        && let Ok(Some((count, used))) = parse_greater_than_or_equal_quantity_prefix(
            &inner_tokens[2..],
            false,
            false,
            "for-each poison-counter predicate",
        )
    {
        let cmp_idx = 2 + used;
        if inner_words
            .get(cmp_idx..cmp_idx + 2)
            .is_some_and(|words| FOR_EACH_POISON_COUNTERS_PATTERN.matches_words(words))
        {
            let effect_start = cmp_idx + 2;
            let effect_clause = trimmed_tail_from_word(inner_clause, effect_start);
            let effect_tokens = effect_clause.tokens();
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each opponent who has ... poison counters' (clause: '{}')",
                    clause_text
                )));
            }
            let mut branch_effects = parse_effect_chain(&effect_tokens)?;
            force_implicit_token_controller_you(&mut branch_effects);
            return Ok(Some(wrap_for_each(vec![EffectAst::Conditional {
                predicate: PredicateAst::PlayerHasPoisonCountersOrMore {
                    player: PlayerAst::That,
                    count,
                },
                if_true: branch_effects,
                if_false: Vec::new(),
            }])));
        }
    }
    if for_each_words_start_with_who(&inner_words)
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) = for_each_this_way_index(&inner_words) {
                inner_clause
                    .token_index_for_word_or_end(this_way_idx + 2)
                    .unwrap_or(inner_clause.len())
            } else {
                inner_clause
                    .token_index_for_word_or_end(negation_idx + negation_len)
                    .unwrap_or(inner_clause.len())
            };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each opponent who doesn't clause (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachOpponentDoesNot {
            effects,
            predicate,
        }));
    }

    if for_each_words_start_with_who(&inner_words)
        && let Some(this_way_idx) = for_each_this_way_index(&inner_words)
    {
        let effect_start = this_way_idx + 2;
        let effect_clause = trimmed_tail_from_word(inner_clause, effect_start);
        let effect_tokens = effect_clause.tokens();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who ... this way' (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_who_did_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachOpponentDid { effects, predicate }));
    }
    if grammar::words_match_any_prefix(&inner_tokens, WHO_ACTION_PREFIXES).is_some() {
        let comma_idx = find_token_index(&inner_tokens, |token| token.is_comma());
        let effect_token_start = if let Some(comma_idx) = comma_idx {
            comma_idx + 1
        } else {
            inner_clause
                .token_index_for_word_or_end(2)
                .unwrap_or(inner_clause.len())
        };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who does' (clause: '{}')",
                clause_text
            )));
        }
        let mut effects = parse_effect_chain_inner(&effect_tokens)?;
        let implicit_player = if comma_idx.is_some() {
            PlayerAst::You
        } else {
            PlayerAst::That
        };
        for effect in &mut effects {
            bind_implicit_player_context(effect, implicit_player);
        }
        return Ok(Some(EffectAst::ForEachOpponentDid {
            effects,
            predicate: None,
        }));
    }

    let inner_words = token_word_refs(&inner_tokens);
    if for_each_words_start_with_who(&inner_words) {
        let tapped_land_turn_idx = find_tapped_land_for_mana_this_turn_end(&inner_words);
        if let Some(turn_idx) = tapped_land_turn_idx {
            let effect_clause = trimmed_tail_from_word(inner_clause, turn_idx + 1);
            let effect_tokens = effect_clause.tokens();
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                    clause_text
                )));
            }
            let branch_effects = if contains_token_word(&effect_tokens, "may") {
                let stripped = remove_first_word(&effect_tokens, "may");
                let inner_effects = parse_effect_chain_inner(&stripped)?;
                vec![EffectAst::May {
                    effects: inner_effects,
                }]
            } else {
                parse_effect_chain_inner(&effect_tokens)?
            };
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                        player: PlayerAst::That,
                    },
                    if_true: branch_effects,
                    if_false: Vec::new(),
                }],
            }));
        }
    }
    if for_each_words_start_with_who(&inner_words)
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) = for_each_this_way_index(&inner_words) {
                inner_clause
                    .token_index_for_word_or_end(this_way_idx + 2)
                    .unwrap_or(inner_clause.len())
            } else {
                inner_clause
                    .token_index_for_word_or_end(negation_idx + negation_len)
                    .unwrap_or(inner_clause.len())
            };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each player who doesn't clause (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }));
    }
    if for_each_words_start_with_who(&inner_words)
        && let Some(this_way_idx) = for_each_this_way_index(&inner_words)
    {
        let effect_start = this_way_idx + 2;
        let effect_clause = trimmed_tail_from_word(inner_clause, effect_start);
        let effect_tokens = effect_clause.tokens();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_who_did_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDid { effects, predicate }));
    }
    if grammar::words_match_any_prefix(&inner_tokens, WHO_ACTION_PREFIXES).is_some() {
        let comma_idx = find_token_index(&inner_tokens, |token| token.is_comma());
        let effect_token_start = if let Some(comma_idx) = comma_idx {
            comma_idx + 1
        } else {
            inner_clause
                .token_index_for_word_or_end(2)
                .unwrap_or(inner_clause.len())
        };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who does' (clause: '{}')",
                clause_text
            )));
        }
        let mut effects = parse_effect_chain_inner(&effect_tokens)?;
        let implicit_player = if comma_idx.is_some() {
            PlayerAst::You
        } else {
            PlayerAst::That
        };
        for effect in &mut effects {
            bind_implicit_player_context(effect, implicit_player);
        }
        return Ok(Some(EffectAst::ForEachPlayerDid {
            effects,
            predicate: None,
        }));
    }

    let inner_words = token_word_refs(&inner_tokens);
    if for_each_words_start_with_who(&inner_words) {
        let tapped_land_turn_idx = find_tapped_land_for_mana_this_turn_end(&inner_words);
        if let Some(turn_idx) = tapped_land_turn_idx {
            let effect_clause = trimmed_tail_from_word(inner_clause, turn_idx + 1);
            let effect_tokens = effect_clause.tokens();
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                    clause_text
                )));
            }
            let branch_effects = if contains_token_word(&effect_tokens, "may") {
                let stripped = remove_first_word(&effect_tokens, "may");
                let inner_effects = parse_effect_chain_inner(&stripped)?;
                vec![EffectAst::May {
                    effects: inner_effects,
                }]
            } else {
                parse_effect_chain_inner(&effect_tokens)?
            };
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                        player: PlayerAst::That,
                    },
                    if_true: branch_effects,
                    if_false: Vec::new(),
                }],
            }));
        }
    }
    if for_each_words_start_with_who(&inner_words)
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) = for_each_this_way_index(&inner_words) {
                inner_clause
                    .token_index_for_word_or_end(this_way_idx + 2)
                    .unwrap_or(inner_clause.len())
            } else {
                inner_clause
                    .token_index_for_word_or_end(negation_idx + negation_len)
                    .unwrap_or(inner_clause.len())
            };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each player who doesn't clause (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }));
    }
    if for_each_words_start_with_who(&inner_words)
        && let Some(this_way_idx) = for_each_this_way_index(&inner_words)
    {
        let effect_start = this_way_idx + 2;
        let effect_clause = trimmed_tail_from_word(inner_clause, effect_start);
        let effect_tokens = effect_clause.tokens();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_who_did_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDid { effects, predicate }));
    }
    if grammar::words_match_any_prefix(&inner_tokens, WHO_ACTION_PREFIXES).is_some() {
        let comma_idx = find_token_index(&inner_tokens, |token| token.is_comma());
        let effect_token_start = if let Some(comma_idx) = comma_idx {
            comma_idx + 1
        } else {
            inner_clause
                .token_index_for_word_or_end(2)
                .unwrap_or(inner_clause.len())
        };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who does' (clause: '{}')",
                clause_text
            )));
        }
        let mut effects = parse_effect_chain_inner(&effect_tokens)?;
        let implicit_player = if comma_idx.is_some() {
            PlayerAst::You
        } else {
            PlayerAst::That
        };
        for effect in &mut effects {
            bind_implicit_player_context(effect, implicit_player);
        }
        return Ok(Some(EffectAst::ForEachPlayerDid {
            effects,
            predicate: None,
        }));
    }

    let normalized_inner_tokens = prepend_that_player_life_total_subject(&inner_tokens);
    let effects = if normalized_inner_tokens
        .iter()
        .any(|token| FOR_EACH_MAY_WORD_PATTERN.matches_token(token))
    {
        let stripped = remove_first_word(&normalized_inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain(&normalized_inner_tokens)?
    };
    Ok(Some(wrap_for_each(effects)))
}

pub(crate) fn parse_for_each_target_players_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause = LexedClause::new(tokens);
    if let Some(after_then) =
        for_each_strip_prefix_shape_clause(clause, &FOR_EACH_THEN_PREFIX_PATTERN)
    {
        clause = after_then;
    }
    let clause_tokens = clause.tokens();
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let (count, start) = parse_choice_or_range_count_token_prefix_consumed(clause_tokens)
        .filter(|(_, used)| {
            clause_tokens
                .get(*used)
                .is_some_and(|token| FOR_EACH_TARGET_WORD_PATTERN.matches_token(token))
        })
        .unwrap_or((ChoiceCount::exactly(1), 0));

    let Some(target_token) = clause_tokens.get(start) else {
        return Ok(None);
    };
    if !FOR_EACH_TARGET_WORD_PATTERN.matches_token(target_token) {
        return Ok(None);
    }
    if !clause_tokens
        .get(start + 1)
        .is_some_and(|token| FOR_EACH_PLAYER_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }
    if !clause_tokens
        .get(start + 2)
        .is_some_and(|token| FOR_EACH_EACH_WORD_PATTERN.matches_token(token))
    {
        return Ok(None);
    }

    let inner_clause = LexedClause::new(&clause_tokens[start + 3..]).trimmed();
    let inner_tokens = inner_clause.tokens();
    if inner_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after target-player each clause (clause: '{}')",
            clause_text
        )));
    }

    let effects = if contains_token_word(&inner_tokens, "may") {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain_inner(&inner_tokens)?
    };
    Ok(Some(EffectAst::ForEachTargetPlayers { count, effects }))
}

pub(crate) fn parse_who_did_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let inner_clause = LexedClause::new(inner_tokens);
    let inner_words = inner_clause.word_refs();
    if !for_each_words_start_with_who(&inner_words) {
        return Ok(None);
    }
    let Some(this_way_idx) = for_each_this_way_index(&inner_words) else {
        return Ok(None);
    };
    let verb = inner_words.get(1).copied().unwrap_or("");
    let supports_tag = FOR_EACH_TAGGED_ACTION_WORD_PATTERN.matches_word(verb);
    if !supports_tag || this_way_idx <= 2 {
        return Ok(None);
    }
    let filter_start = inner_clause
        .token_index_for_word_or_end(2)
        .unwrap_or(inner_clause.len());
    let filter_end = inner_clause
        .token_index_for_word_or_end(this_way_idx)
        .unwrap_or(inner_clause.len());
    if filter_start >= filter_end {
        return Ok(None);
    }
    let filter_tokens = LexedClause::new(&inner_tokens[filter_start..filter_end]).trim();
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };
    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
    }))
}

fn parse_negated_who_this_way_predicate(
    inner_tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let inner_clause = LexedClause::new(inner_tokens);
    let inner_words = inner_clause.word_refs();
    if !for_each_words_start_with_who(&inner_words) {
        return Ok(None);
    }
    let Some(this_way_idx) = for_each_this_way_index(&inner_words) else {
        return Ok(None);
    };
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    let verb_idx = negation_idx + negation_len;
    let verb = inner_words.get(verb_idx).copied().unwrap_or("");
    if !FOR_EACH_DISCARD_OR_DISCARDED_WORD_PATTERN.matches_word(verb)
        || this_way_idx <= verb_idx + 1
    {
        return Ok(None);
    }

    let filter_start = inner_clause
        .token_index_for_word_or_end(verb_idx + 1)
        .unwrap_or(inner_clause.len());
    let filter_end = inner_clause
        .token_index_for_word_or_end(this_way_idx)
        .unwrap_or(inner_clause.len());
    if filter_start >= filter_end {
        return Ok(None);
    }

    let filter_tokens = LexedClause::new(&inner_tokens[filter_start..filter_end]).trim();
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    Ok(Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::That,
        tag: TagKey::from(IT_TAG),
        filter,
    }))
}

pub(crate) fn parse_for_each_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause = LexedClause::new(tokens);
    if let Some(after_then) =
        for_each_strip_prefix_shape_clause(clause, &FOR_EACH_THEN_PREFIX_PATTERN)
    {
        clause = after_then;
    }
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let Some((_, inner_clause)) = clause.strip_any_prefix_clause(FOR_EACH_PLAYER_PREFIXES) else {
        return Ok(None);
    };

    let inner_clause = inner_clause.trimmed();
    let inner_tokens = inner_clause.tokens();
    if inner_tokens.len() > 3
        && FOR_EACH_WHO_CONTROLS_PREFIX_PATTERN.matches_words(&inner_clause.word_refs())
    {
        let mut effect_start = None;
        for idx in 2..inner_tokens.len() {
            if let Some(word) = inner_tokens[idx].as_word()
                && (FOR_EACH_MAY_WORD_PATTERN.matches_word(word)
                    || super::find_verb(&inner_tokens[idx..])
                        .is_some_and(|(_, verb_idx)| verb_idx == 0))
            {
                effect_start = Some(idx);
                break;
            }
        }
        let effect_start = effect_start.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing effect clause after 'each player who controls' (clause: '{}')",
                clause_text
            ))
        })?;

        let filter_tokens = LexedClause::new(&inner_tokens[2..effect_start]).trim();
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'each player who controls' (clause: '{}')",
                clause_text
            )));
        }
        let filter_clause = LexedClause::new(&filter_tokens);
        let (controls_most, normalized_filter_tokens) = if let Some(rest) =
            for_each_strip_prefix_shape_clause(filter_clause, &FOR_EACH_MOST_PREFIX_PATTERN)
        {
            (true, rest.trimmed().tokens().to_vec())
        } else {
            (false, filter_tokens)
        };
        if normalized_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter after 'most' (clause: '{}')",
                clause_text
            )));
        }
        let filter = parse_object_filter(&normalized_filter_tokens, false)?;

        let effect_tokens = LexedClause::new(&inner_tokens[effect_start..]).trim();
        let branch_effects = if contains_token_word(&effect_tokens, "may") {
            let stripped = remove_first_word(&effect_tokens, "may");
            let inner_effects = parse_effect_chain_inner(&stripped)?;
            vec![EffectAst::May {
                effects: inner_effects,
            }]
        } else {
            parse_effect_chain_inner(&effect_tokens)?
        };

        let predicate = if controls_most {
            PredicateAst::PlayerControlsMost {
                player: PlayerAst::That,
                filter,
            }
        } else {
            PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            }
        };
        let effects = vec![EffectAst::Conditional {
            predicate,
            if_true: branch_effects,
            if_false: Vec::new(),
        }];
        return Ok(Some(EffectAst::ForEachPlayer { effects }));
    }

    let inner_words = token_word_refs(&inner_tokens);
    if for_each_words_start_with_who(&inner_words) {
        let tapped_land_turn_idx = find_tapped_land_for_mana_this_turn_end(&inner_words);
        if let Some(turn_idx) = tapped_land_turn_idx {
            let effect_clause = trimmed_tail_from_word(inner_clause, turn_idx + 1);
            let effect_tokens = effect_clause.tokens();
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                    clause_text
                )));
            }
            let branch_effects = if contains_token_word(&effect_tokens, "may") {
                let stripped = remove_first_word(&effect_tokens, "may");
                let inner_effects = parse_effect_chain_inner(&stripped)?;
                vec![EffectAst::May {
                    effects: inner_effects,
                }]
            } else {
                parse_effect_chain_inner(&effect_tokens)?
            };
            return Ok(Some(EffectAst::ForEachPlayer {
                effects: vec![EffectAst::Conditional {
                    predicate: PredicateAst::PlayerTappedLandForManaThisTurn {
                        player: PlayerAst::That,
                    },
                    if_true: branch_effects,
                    if_false: Vec::new(),
                }],
            }));
        }
    }
    if for_each_words_start_with_who(&inner_words)
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) = for_each_this_way_index(&inner_words) {
                inner_clause
                    .token_index_for_word_or_end(this_way_idx + 2)
                    .unwrap_or(inner_clause.len())
            } else {
                inner_clause
                    .token_index_for_word_or_end(negation_idx + negation_len)
                    .unwrap_or(inner_clause.len())
            };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each player who doesn't clause (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }));
    }
    if for_each_words_start_with_who(&inner_words)
        && let Some(this_way_idx) = for_each_this_way_index(&inner_words)
    {
        let effect_start = this_way_idx + 2;
        let effect_clause = trimmed_tail_from_word(inner_clause, effect_start);
        let effect_tokens = effect_clause.tokens();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_text
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_who_did_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDid { effects, predicate }));
    }
    if grammar::words_match_any_prefix(&inner_tokens, WHO_ACTION_PREFIXES).is_some() {
        let comma_idx = find_token_index(&inner_tokens, |token| token.is_comma());
        let effect_token_start = if let Some(comma_idx) = comma_idx {
            comma_idx + 1
        } else {
            inner_clause
                .token_index_for_word_or_end(2)
                .unwrap_or(inner_clause.len())
        };
        let effect_tokens = LexedClause::new(&inner_tokens[effect_token_start..]).trim();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who does' (clause: '{}')",
                clause_text
            )));
        }
        let mut effects = parse_effect_chain_inner(&effect_tokens)?;
        let implicit_player = if comma_idx.is_some() {
            PlayerAst::You
        } else {
            PlayerAst::That
        };
        for effect in &mut effects {
            bind_implicit_player_context(effect, implicit_player);
        }
        return Ok(Some(EffectAst::ForEachPlayerDid {
            effects,
            predicate: None,
        }));
    }

    let normalized_inner_tokens = prepend_that_player_life_total_subject(&inner_tokens);
    let effects = if normalized_inner_tokens
        .iter()
        .any(|token| FOR_EACH_MAY_WORD_PATTERN.matches_token(token))
    {
        let stripped = remove_first_word(&normalized_inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain(&normalized_inner_tokens)?
    };
    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

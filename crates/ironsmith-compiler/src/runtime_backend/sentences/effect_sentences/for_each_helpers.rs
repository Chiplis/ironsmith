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
use super::super::lexer::{
    LexedClause, contains_token_word, token_slice_starts_with, token_word_refs, word_slice_at_is,
    word_slice_at_is_any, word_slice_contains_any_phrase_or_empty, word_slice_contains_any_word,
    word_slice_contains_phrase_or_empty, word_slice_contains_word, word_slice_eq,
    word_slice_find_phrase_start_or_zero, word_slice_first_is, word_slice_first_is_any,
    word_slice_starts_with, word_slice_starts_with_any, word_slice_strip_first_word_value,
};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::find_index as find_token_index;
use super::super::util::{
    contains_until_end_of_turn, is_until_end_of_turn, parse_for_each_count_value_words,
    parse_number, parse_target_count_range_prefix, parse_target_phrase, parse_value,
    replace_unbound_x_with_value, starts_with_until_end_of_turn, value_contains_unbound_x,
};
use super::chain_carry::bind_implicit_player_context;
use super::chain_carry::{parse_effect_chain, parse_effect_chain_inner, remove_first_word};
use super::conditionals::negated_action_word_index;
use super::{Verb, find_verb};

fn trimmed_tail_from_word(clause: LexedClause<'_>, word_idx: usize) -> LexedClause<'_> {
    let token_idx = clause
        .token_index_for_word_or_end(word_idx)
        .unwrap_or(clause.len());
    clause.from(token_idx).trimmed()
}

fn prepend_that_player_life_total_subject(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words = token_word_refs(tokens);
    if !word_slice_starts_with(&words, &["life", "total", "becomes"]) {
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
const ANY_NUMBER_OF_PREFIXES: &[&[&str]] = &[&["any", "number", "of"]];
const UP_TO_PREFIXES: &[&[&str]] = &[&["up", "to"]];
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

fn find_tapped_land_for_mana_this_turn_end(words: &[&str]) -> Option<usize> {
    word_slice_find_phrase_start_or_zero(
        words,
        &["tapped", "a", "land", "for", "mana", "this", "turn"],
    )
    .map(|idx| idx + 6)
    .or_else(|| {
        word_slice_find_phrase_start_or_zero(
            words,
            &["tapped", "land", "for", "mana", "this", "turn"],
        )
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
    if let Some(after_of) = filter_clause.strip_prefix_clause(&["of"]) {
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
            .is_some_and(|token| token.is_word("to"))
        && attached_idx > 0
    {
        let attached_tail_clause = filter_clause.from(attached_idx + 2);
        let attached_to_creature =
            attached_tail_clause.starts_with_any(&[&["creature"], &["a", "creature"]]);
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
    if let Some(after_of) = target_clause.strip_prefix_clause(&["of"]) {
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
    word_slice_contains_any_phrase_or_empty(words, DEMONSTRATIVE_OBJECT_REFERENCE_PHRASES)
}

pub(crate) fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    if words.len() < 8 {
        return false;
    }
    if !word_slice_starts_with_any(words, &[&["target", "player"], &["target", "players"]]) {
        return false;
    }
    word_slice_contains_phrase_or_empty(
        words,
        &["dealt", "damage", "by", "this", "creature", "this"],
    ) && word_slice_contains_phrase_or_empty(words, &["this", "turn"])
}

pub(crate) fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    let has_if = word_slice_contains_word(words, "if");
    let has_tap = word_slice_contains_any_word(words, &["tap", "taps"]);
    let has_for_mana = word_slice_contains_phrase_or_empty(words, &["for", "mana"]);
    let has_produce = word_slice_contains_any_word(words, &["produce", "produces"]);
    let has_instead = word_slice_contains_word(words, "instead");
    has_if && has_tap && has_for_mana && has_produce && has_instead
}

pub(crate) fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    let has_whenever = word_slice_contains_word(words, "whenever");
    let has_tap = word_slice_contains_any_word(words, &["tap", "taps"]);
    let has_for_mana = word_slice_contains_phrase_or_empty(words, &["for", "mana"]);
    let has_add = word_slice_contains_any_word(words, &["add", "adds"]);
    let has_additional = word_slice_contains_word(words, "additional");
    has_whenever && has_tap && has_for_mana && has_add && has_additional
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
    if rest_words.len() < 3
        || !crate::runtime_backend::lexer::word_slice_starts_with(&rest_words, &["base", "power"])
    {
        return Ok(None);
    }
    if word_slice_at_is(rest_words, 2, "and") {
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
            || clause.contains_phrase(&["this", "turn"])
            || clause.contains_phrase(&["next", "turn"]);
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
        || !crate::runtime_backend::lexer::word_slice_starts_with(
            &rest_words,
            &["base", "power", "and", "toughness"],
        )
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
            || clause.contains_phrase(&["this", "turn"])
            || clause.contains_phrase(&["next", "turn"]);
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
        duration = if word_slice_eq(phrase, &["until", "your", "next", "turn"]) {
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
    if word_slice_eq(&tail_words, &["instead"]) {
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
            Until::ThisLeavesTheBattlefield,
            condition,
        ));
    }
    if word_slice_eq(
        &tail_words,
        &["and", "must", "be", "blocked", "this", "turn", "if", "able"],
    ) {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if word_slice_eq(
        &tail_words,
        &["and", "cant", "be", "blocked", "this", "turn"],
    ) {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if word_slice_first_is(&tail_words, "and")
        && word_slice_at_is_any(&tail_words, 1, &["gain", "gains", "has", "have"])
        && tail_words
            .iter()
            .any(|word| matches!(*word, "trample" | "haste" | "first" | "strike" | "infect"))
        && contains_until_end_of_turn(&tail_words)
    {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if word_slice_first_is(&tail_words, "and")
        && tail_words
            .iter()
            .any(|word| matches!(*word, "gain" | "gains" | "has" | "have"))
        && tail_words
            .iter()
            .any(|word| matches!(*word, "trample" | "haste" | "first" | "strike" | "infect"))
        && contains_until_end_of_turn(&tail_words)
    {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if word_slice_first_is(&tail_words, "and")
        && tail_words
            .iter()
            .any(|word| matches!(*word, "control" | "controls"))
    {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if word_slice_first_is(&tail_words, "or")
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
    if !tail_clause.starts_with(&["where", "x", "is"]) {
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
    if let Some(after_then) = clause.strip_prefix_clause(&["then"]) {
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
    if let Some(after_defending_player) =
        inner_clause.strip_prefix_clause(&["other", "than", "defending", "player"])
    {
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
            && word_slice_strip_first_word_value(
                &inner_words,
                &[
                    ("scries", ()),
                    ("scry", ()),
                    ("surveils", ()),
                    ("surveil", ()),
                ],
            )
            .is_some_and(|(_, rest)| rest.len() == 1 && rest[0] != "x")
    }) {
        return Ok(None);
    }
    if word_slice_first_is(&inner_words, "choose")
        && let Some(then_return_idx) =
            word_slice_find_phrase_start_or_zero(&inner_words, &["then", "return"])
        && let Some(unless_idx) = word_slice_find_phrase_start_or_zero(&inner_words, &["unless"])
        && unless_idx > then_return_idx
        && inner_words.get(unless_idx + 1..unless_idx + 7)
            == Some(["its", "controller", "has", "you", "draw", "a"].as_slice())
        && word_slice_at_is(&inner_words, unless_idx + 7, "card")
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
        && word_slice_first_is(&inner_words, "who")
        && word_slice_at_is(&inner_words, 1, "has")
        && let Some((count, used)) = parse_number(&inner_tokens[2..])
    {
        let cmp_idx = 2 + used;
        if word_slice_at_is(&inner_words, cmp_idx, "or")
            && word_slice_at_is(&inner_words, cmp_idx + 1, "more")
            && word_slice_at_is(&inner_words, cmp_idx + 2, "poison")
            && word_slice_at_is_any(&inner_words, cmp_idx + 3, &["counter", "counters"])
        {
            let effect_start = cmp_idx + 4;
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
    if word_slice_first_is(&inner_words, "who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) =
                word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
            {
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

    if word_slice_first_is(&inner_words, "who")
        && let Some(this_way_idx) =
            word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
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
    if word_slice_first_is(&inner_words, "who") {
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
    if word_slice_first_is(&inner_words, "who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) =
                word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
            {
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
    if word_slice_first_is(&inner_words, "who")
        && let Some(this_way_idx) =
            word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
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
    if word_slice_first_is(&inner_words, "who") {
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
    if word_slice_first_is(&inner_words, "who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) =
                word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
            {
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
    if word_slice_first_is(&inner_words, "who")
        && let Some(this_way_idx) =
            word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
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
        .any(|token| token.is_word("may"))
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
    if let Some(after_then) = clause.strip_prefix_clause(&["then"]) {
        clause = after_then;
    }
    let clause_tokens = clause.tokens();
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let mut start = 0usize;
    let mut count = ChoiceCount::exactly(1);
    if grammar::words_match_any_prefix(clause_tokens, ANY_NUMBER_OF_PREFIXES).is_some() {
        count = ChoiceCount::any_number();
        start = 3;
    } else if grammar::words_match_any_prefix(clause_tokens, UP_TO_PREFIXES).is_some()
        && let Some((value, used)) = parse_number(&clause_tokens[2..])
    {
        count = ChoiceCount::up_to(value as usize);
        start = 2 + used;
    } else if let Some((range_count, used)) = parse_target_count_range_prefix(clause_tokens) {
        count = range_count;
        start = used;
    } else if let Some((value, used)) = parse_number(clause_tokens)
        && clause_tokens
            .get(used)
            .is_some_and(|token| token.is_word("target"))
    {
        count = ChoiceCount::exactly(value as usize);
        start = used;
    }

    let Some(target_token) = clause_tokens.get(start) else {
        return Ok(None);
    };
    if !target_token.is_word("target") {
        return Ok(None);
    }
    if !clause_tokens
        .get(start + 1)
        .is_some_and(|token| token.is_word("player") || token.is_word("players"))
    {
        return Ok(None);
    }
    if !clause_tokens
        .get(start + 2)
        .is_some_and(|token| token.is_word("each"))
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
    if !word_slice_first_is(&inner_words, "who") {
        return Ok(None);
    }
    let Some(this_way_idx) = word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
    else {
        return Ok(None);
    };
    let verb = inner_words.get(1).copied().unwrap_or("");
    let supports_tag = matches!(verb, "sacrificed" | "destroyed" | "exiled" | "discarded");
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
    if !word_slice_first_is(&inner_words, "who") {
        return Ok(None);
    }
    let Some(this_way_idx) = word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
    else {
        return Ok(None);
    };
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    let verb_idx = negation_idx + negation_len;
    let verb = inner_words.get(verb_idx).copied().unwrap_or("");
    if !matches!(verb, "discard" | "discarded") || this_way_idx <= verb_idx + 1 {
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
    if let Some(after_then) = clause.strip_prefix_clause(&["then"]) {
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
    if inner_tokens.len() > 3 && token_slice_starts_with(inner_tokens, &["who", "controls"]) {
        let mut effect_start = None;
        for idx in 2..inner_tokens.len() {
            if let Some(word) = inner_tokens[idx].as_word()
                && (word == "may"
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
        let (controls_most, normalized_filter_tokens) =
            if let Some(rest) = filter_clause.strip_prefix_clause(&["the", "most"]) {
                (true, rest.trimmed().tokens().to_vec())
            } else if let Some(rest) = filter_clause.strip_prefix_clause(&["most"]) {
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
    if word_slice_first_is(&inner_words, "who") {
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
    if word_slice_first_is(&inner_words, "who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start =
            if let Some(comma_idx) = find_token_index(&inner_tokens, |token| token.is_comma()) {
                comma_idx + 1
            } else if let Some(this_way_idx) =
                word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
            {
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
    if word_slice_first_is(&inner_words, "who")
        && let Some(this_way_idx) =
            word_slice_find_phrase_start_or_zero(&inner_words, &["this", "way"])
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
        .any(|token| token.is_word("may"))
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

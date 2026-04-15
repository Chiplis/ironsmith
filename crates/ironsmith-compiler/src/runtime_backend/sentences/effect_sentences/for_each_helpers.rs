use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, OwnedLexToken, PlayerAst, PredicateAst, TagKey,
    TargetAst,
};
use crate::effect::{Until, Value};
use crate::target::{ObjectFilter, PlayerFilter};

use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::primitives as grammar;
use super::super::keyword_static::{
    parse_pt_modifier, parse_pt_modifier_values, parse_where_x_value_clause,
};
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{
    contains_window as word_slice_contains_sequence, find_index as find_token_index,
    find_str_by as find_word_index, find_window_index as find_word_sequence_index,
    slice_contains_str as word_slice_contains, slice_starts_with as word_slice_starts_with,
};
use super::super::util::{
    contains_until_end_of_turn, is_until_end_of_turn, parse_for_each_count_value_words,
    parse_number, parse_target_count_range_prefix, parse_target_phrase, parse_value,
    replace_unbound_x_with_value, starts_with_until_end_of_turn, token_index_for_word_index,
    trim_commas, value_contains_unbound_x,
};
use super::chain_carry::bind_implicit_player_context;
use super::chain_carry::{parse_effect_chain, parse_effect_chain_inner, remove_first_word};
use super::conditionals::negated_action_word_index;

fn token_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    crate::cards::builders::compiler::lexer::token_word_refs(tokens)
}

const PLAYER_OR_OPPONENT_PREFIXES: &[&[&str]] = &[
    &["player"],
    &["players"],
    &["opponent"],
    &["opponents"],
    &["target", "player"],
    &["target", "players"],
    &["target", "opponent"],
    &["target", "opponents"],
];

const FOR_EACH_PREFIXES: &[&[&str]] = &[&["for", "each"], &["each"]];
const WHO_ACTION_PREFIXES: &[&[&str]] = &[&["who", "does"], &["who", "do"], &["who", "did"]];
const INSTEAD_IF_PREFIXES: &[&[&str]] = &[&["instead", "if"]];
const FOR_AS_LONG_AS_PREFIXES: &[&[&str]] = &[&["for", "as", "long", "as"]];
const ANY_NUMBER_OF_PREFIXES: &[&[&str]] = &[&["any", "number", "of"]];
const UP_TO_PREFIXES: &[&[&str]] = &[&["up", "to"]];

fn find_tapped_land_for_mana_this_turn_end(words: &[&str]) -> Option<usize> {
    find_word_sequence_index(
        words,
        &["tapped", "a", "land", "for", "mana", "this", "turn"],
    )
    .map(|idx| idx + 6)
    .or_else(|| {
        find_word_sequence_index(words, &["tapped", "land", "for", "mana", "this", "turn"])
            .map(|idx| idx + 5)
    })
}

pub(crate) fn parse_for_each_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter_tokens =
        if let Some(rest) = grammar::words_match_prefix(subject_tokens, &["for", "each"]) {
            rest
        } else if let Some(rest) = grammar::words_match_prefix(subject_tokens, &["each"]) {
            rest
        } else {
            return Ok(None);
        };
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("of"))
    {
        filter_tokens = &filter_tokens[1..];
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut normalized_filter_tokens: Vec<OwnedLexToken> = filter_tokens.to_vec();
    if let Some(attached_idx) = find_token_index(filter_tokens, |token| token.is_word("attached"))
        && filter_tokens
            .get(attached_idx + 1)
            .is_some_and(|token| token.is_word("to"))
        && attached_idx > 0
    {
        let attached_to_creature =
            grammar::words_match_prefix(&filter_tokens[attached_idx + 2..], &["creature"])
                .is_some()
                || grammar::words_match_prefix(
                    &filter_tokens[attached_idx + 2..],
                    &["a", "creature"],
                )
                .is_some();
        if attached_to_creature {
            normalized_filter_tokens = trim_commas(&filter_tokens[..attached_idx]);
        }
    }

    if normalized_filter_tokens.is_empty() {
        return Ok(None);
    }

    if grammar::words_match_any_prefix(&normalized_filter_tokens, PLAYER_OR_OPPONENT_PREFIXES)
        .is_some()
    {
        return Ok(None);
    }

    Ok(Some(parse_object_filter(&normalized_filter_tokens, false)?))
}

pub(crate) fn parse_for_each_targeted_object_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let Some((_, mut target_tokens)) =
        grammar::words_match_any_prefix(subject_tokens, FOR_EACH_PREFIXES)
    else {
        return Ok(None);
    };
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("of"))
    {
        target_tokens = &target_tokens[1..];
    }
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let target = match parse_target_phrase(target_tokens) {
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
    word_slice_contains_sequence(words, &["that", "creature"])
        || word_slice_contains_sequence(words, &["that", "creatures"])
        || word_slice_contains_sequence(words, &["that", "permanent"])
        || word_slice_contains_sequence(words, &["that", "permanents"])
        || word_slice_contains_sequence(words, &["that", "artifact"])
        || word_slice_contains_sequence(words, &["that", "artifacts"])
        || word_slice_contains_sequence(words, &["that", "enchantment"])
        || word_slice_contains_sequence(words, &["that", "enchantments"])
        || word_slice_contains_sequence(words, &["that", "land"])
        || word_slice_contains_sequence(words, &["that", "lands"])
        || word_slice_contains_sequence(words, &["that", "card"])
        || word_slice_contains_sequence(words, &["that", "cards"])
        || word_slice_contains_sequence(words, &["that", "token"])
        || word_slice_contains_sequence(words, &["that", "tokens"])
        || word_slice_contains_sequence(words, &["that", "spell"])
        || word_slice_contains_sequence(words, &["that", "spells"])
        || word_slice_contains_sequence(words, &["those", "creatures"])
        || word_slice_contains_sequence(words, &["those", "permanents"])
        || word_slice_contains_sequence(words, &["those", "artifacts"])
        || word_slice_contains_sequence(words, &["those", "enchantments"])
        || word_slice_contains_sequence(words, &["those", "lands"])
        || word_slice_contains_sequence(words, &["those", "cards"])
        || word_slice_contains_sequence(words, &["those", "tokens"])
        || word_slice_contains_sequence(words, &["those", "spells"])
}

pub(crate) fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    if words.len() < 8 {
        return false;
    }
    if !(word_slice_starts_with(words, &["target", "player"])
        || word_slice_starts_with(words, &["target", "players"]))
    {
        return false;
    }
    word_slice_contains_sequence(
        words,
        &["dealt", "damage", "by", "this", "creature", "this"],
    ) && word_slice_contains_sequence(words, &["this", "turn"])
}

pub(crate) fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    let has_if = word_slice_contains(words, "if");
    let has_tap = word_slice_contains(words, "tap") || word_slice_contains(words, "taps");
    let has_for_mana = word_slice_contains_sequence(words, &["for", "mana"]);
    let has_produce =
        word_slice_contains(words, "produce") || word_slice_contains(words, "produces");
    let has_instead = word_slice_contains(words, "instead");
    has_if && has_tap && has_for_mana && has_produce && has_instead
}

pub(crate) fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    let has_whenever = word_slice_contains(words, "whenever");
    let has_tap = word_slice_contains(words, "tap") || word_slice_contains(words, "taps");
    let has_for_mana = word_slice_contains_sequence(words, &["for", "mana"]);
    let has_add = word_slice_contains(words, "add") || word_slice_contains(words, "adds");
    let has_additional = word_slice_contains(words, "additional");
    has_whenever && has_tap && has_for_mana && has_add && has_additional
}

pub(crate) fn parse_has_base_power_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = token_words(tokens);
    let Some(has_idx) = find_word_index(&words_all, |word| word == "has" || word == "have") else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..has_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = token_words(subject_tokens);

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 3 || !word_slice_starts_with(rest_words, &["base", "power"]) {
        return Ok(None);
    }
    if rest_words.get(2).is_some_and(|word| *word == "and") {
        return Ok(None);
    }

    let has_token_idx = find_token_index(tokens, |token| {
        token.is_word("has") || token.is_word("have")
    })
    .ok_or_else(|| {
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
        let has_target_subject = grammar::contains_word(subject_tokens, "target");
        let has_leading_until_eot = starts_with_until_end_of_turn(&subject_words);
        let has_temporal_words = contains_until_end_of_turn(&words_all)
            || grammar::words_find_phrase(tokens, &["this", "turn"]).is_some()
            || grammar::words_find_phrase(tokens, &["next", "turn"]).is_some();
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    } else if !is_until_end_of_turn(tail_words.as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_tokens: Vec<OwnedLexToken> = if starts_with_until_end_of_turn(&subject_words) {
        let mut skip_idx = 4usize;
        if subject_tokens
            .get(skip_idx)
            .is_some_and(|token| token.is_comma())
        {
            skip_idx += 1;
        }
        trim_commas(&subject_tokens[skip_idx..]).to_vec()
    } else {
        subject_tokens.to_vec()
    };
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::SetBasePower {
        power,
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_has_base_power_toughness_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = token_words(tokens);
    let Some(has_idx) = find_word_index(&words_all, |word| word == "has" || word == "have") else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..has_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = token_words(subject_tokens);

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5
        || !word_slice_starts_with(rest_words, &["base", "power", "and", "toughness"])
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
        let has_target_subject = grammar::contains_word(subject_tokens, "target");
        let has_leading_until_eot = starts_with_until_end_of_turn(&subject_words);
        let has_temporal_words = contains_until_end_of_turn(&words_all)
            || grammar::words_find_phrase(tokens, &["this", "turn"]).is_some()
            || grammar::words_find_phrase(tokens, &["next", "turn"]).is_some();
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    }
    if !tail.is_empty() && !is_until_end_of_turn(tail) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_tokens: Vec<OwnedLexToken> = if starts_with_until_end_of_turn(&subject_words) {
        let mut skip_idx = 4usize;
        if subject_tokens
            .get(skip_idx)
            .is_some_and(|token| token.is_comma())
        {
            skip_idx += 1;
        }
        trim_commas(&subject_tokens[skip_idx..]).to_vec()
    } else {
        subject_tokens.to_vec()
    };
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::SetBasePowerToughness {
        power: Value::Fixed(power),
        toughness: Value::Fixed(toughness),
        target,
        duration: Until::EndOfTurn,
    }))
}

pub(crate) fn parse_get_for_each_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words = token_words(tokens);
    if !matches!(words.as_slice(), ["for", "each", ..] | ["each", ..]) {
        return Ok(None);
    }
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
    let clause = token_words(modifier_tokens).join(" ");
    let mut out_power = power;
    let mut out_toughness = toughness;
    let mut duration = Until::EndOfTurn;
    let mut condition = None;

    if modifier_tokens.is_empty() {
        return Ok((out_power, out_toughness, duration, condition));
    }

    let after_modifier = &modifier_tokens[1..];
    let after_modifier_words = token_words(after_modifier);
    let until_word_count = if starts_with_until_end_of_turn(&after_modifier_words) {
        duration = Until::EndOfTurn;
        4usize
    } else if grammar::words_match_prefix(after_modifier, &["until", "your", "next", "turn"])
        .is_some()
    {
        duration = Until::YourNextTurn;
        4usize
    } else if grammar::words_match_prefix(after_modifier, &["until", "end", "of", "combat"])
        .is_some()
    {
        duration = Until::EndOfCombat;
        4usize
    } else {
        0usize
    };
    let tail_start = token_index_for_word_index(after_modifier, until_word_count)
        .unwrap_or(after_modifier.len());
    let tail_tokens = trim_commas(&after_modifier[tail_start..]);

    if tail_tokens.is_empty() {
        return Ok((out_power, out_toughness, duration, condition));
    }

    let tail_words = token_words(&tail_tokens);
    if tail_words.as_slice() == ["instead"] {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if grammar::words_match_any_prefix(&tail_tokens, INSTEAD_IF_PREFIXES).is_some() {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if grammar::words_match_any_prefix(&tail_tokens, FOR_AS_LONG_AS_PREFIXES).is_some()
        && grammar::contains_word(&tail_tokens, "this")
        && grammar::contains_word(&tail_tokens, "remains")
        && grammar::contains_word(&tail_tokens, "tapped")
    {
        condition = Some(crate::ConditionExpr::SourceIsTapped);
        return Ok((
            out_power,
            out_toughness,
            Until::ThisLeavesTheBattlefield,
            condition,
        ));
    }
    if tail_words == ["and", "must", "be", "blocked", "this", "turn", "if", "able"] {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words == ["and", "cant", "be", "blocked", "this", "turn"] {
        return Ok((out_power, out_toughness, duration, condition));
    }
    if tail_words.first().copied() == Some("or")
        && let Some(alt_mod) = tail_words.get(1).copied()
        && parse_pt_modifier_values(alt_mod).is_ok()
    {
        let alt_tail = &tail_words[2..];
        if alt_tail.is_empty() || is_until_end_of_turn(alt_tail) {
            return Ok((out_power, out_toughness, duration, condition));
        }
    }
    if grammar::words_match_any_prefix(&tail_tokens, FOR_EACH_PREFIXES).is_some()
        && let Some(count) = parse_get_for_each_count_value(&tail_tokens)?
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
    if grammar::words_match_prefix(&tail_tokens, &["where", "x", "is"]).is_none() {
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

    let x_value = parse_where_x_value_clause(&tail_tokens).ok_or_else(|| {
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
            EffectAst::CreateTokenWithMods { player, .. }
            | EffectAst::CreateTokenCopy { player, .. }
            | EffectAst::CreateTokenCopyFromSource { player, .. } => {
                if matches!(player, PlayerAst::Implicit) {
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
    let mut clause_tokens = tokens;
    let mut clause_words = token_words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = token_words(clause_tokens);
    }
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let after_prefix = if let Some(rest) =
        grammar::words_match_prefix(clause_tokens, &["for", "each", "opponent"])
            .or_else(|| grammar::words_match_prefix(clause_tokens, &["for", "each", "opponents"]))
    {
        rest
    } else if let Some(rest) = grammar::words_match_prefix(clause_tokens, &["each", "opponent"])
        .or_else(|| grammar::words_match_prefix(clause_tokens, &["each", "opponents"]))
    {
        rest
    } else {
        return Ok(None);
    };

    let mut inner_tokens = trim_commas(after_prefix).to_vec();
    let mut iteration_filter = PlayerFilter::Opponent;
    if grammar::words_match_prefix(&inner_tokens, &["other", "than", "defending", "player"])
        .is_some()
    {
        let strip_start =
            token_index_for_word_index(&inner_tokens, 4).unwrap_or(inner_tokens.len());
        inner_tokens = trim_commas(&inner_tokens[strip_start..]).to_vec();
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
    let inner_words = token_words(&inner_tokens);
    if let Some(after_who) = grammar::words_match_prefix(
        &inner_tokens,
        &["who", "has", "less", "life", "than", "you"],
    ) {
        let effect_tokens = trim_commas(after_who);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who has less life than you' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
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
    if inner_words.first().copied() == Some("who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start = if let Some(comma_idx) =
            find_token_index(&inner_tokens, |token| token.is_comma())
        {
            comma_idx + 1
        } else if let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
        {
            token_index_for_word_index(&inner_tokens, this_way_idx + 2)
                .unwrap_or(inner_tokens.len())
        } else {
            token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
                .unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each opponent who doesn't clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachOpponentDoesNot {
            effects,
            predicate,
        }));
    }

    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who ... this way' (clause: '{}')",
                clause_words.join(" ")
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
            token_index_for_word_index(&inner_tokens, 2).unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who does' (clause: '{}')",
                clause_words.join(" ")
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

    let inner_words = token_words(&inner_tokens);
    if inner_words.first().copied() == Some("who") {
        let tapped_land_turn_idx = find_tapped_land_for_mana_this_turn_end(&inner_words);
        if let Some(turn_idx) = tapped_land_turn_idx {
            let effect_token_start = token_index_for_word_index(&inner_tokens, turn_idx + 1)
                .unwrap_or(inner_tokens.len());
            let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
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
    if inner_words.first().copied() == Some("who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start = if let Some(comma_idx) =
            find_token_index(&inner_tokens, |token| token.is_comma())
        {
            comma_idx + 1
        } else if let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
        {
            token_index_for_word_index(&inner_tokens, this_way_idx + 2)
                .unwrap_or(inner_tokens.len())
        } else {
            token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
                .unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each player who doesn't clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }));
    }
    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_words.join(" ")
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
            token_index_for_word_index(&inner_tokens, 2).unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who does' (clause: '{}')",
                clause_words.join(" ")
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

    let inner_words = token_words(&inner_tokens);
    if inner_words.first().copied() == Some("who") {
        let tapped_land_turn_idx = find_tapped_land_for_mana_this_turn_end(&inner_words);
        if let Some(turn_idx) = tapped_land_turn_idx {
            let effect_token_start = token_index_for_word_index(&inner_tokens, turn_idx + 1)
                .unwrap_or(inner_tokens.len());
            let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
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
    if inner_words.first().copied() == Some("who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start = if let Some(comma_idx) =
            find_token_index(&inner_tokens, |token| token.is_comma())
        {
            comma_idx + 1
        } else if let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
        {
            token_index_for_word_index(&inner_tokens, this_way_idx + 2)
                .unwrap_or(inner_tokens.len())
        } else {
            token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
                .unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each player who doesn't clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }));
    }
    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_words.join(" ")
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
            token_index_for_word_index(&inner_tokens, 2).unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who does' (clause: '{}')",
                clause_words.join(" ")
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

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain(&inner_tokens)?
    };
    Ok(Some(wrap_for_each(effects)))
}

pub(crate) fn parse_for_each_target_players_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = token_words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = token_words(clause_tokens);
    }
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

    let inner_tokens = trim_commas(&clause_tokens[start + 3..]);
    if inner_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after target-player each clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
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
    let inner_words = token_words(inner_tokens);
    if inner_words.first().copied() != Some("who") {
        return Ok(None);
    }
    let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"]) else {
        return Ok(None);
    };
    let verb = inner_words.get(1).copied().unwrap_or("");
    let supports_tag = matches!(verb, "sacrificed" | "destroyed" | "exiled" | "discarded");
    if !supports_tag || this_way_idx <= 2 {
        return Ok(None);
    }
    let filter_start = token_index_for_word_index(inner_tokens, 2).unwrap_or(inner_tokens.len());
    let filter_end =
        token_index_for_word_index(inner_tokens, this_way_idx).unwrap_or(inner_tokens.len());
    if filter_start >= filter_end {
        return Ok(None);
    }
    let filter_tokens = trim_commas(&inner_tokens[filter_start..filter_end]);
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
    let inner_words = token_words(inner_tokens);
    if inner_words.first().copied() != Some("who") {
        return Ok(None);
    }
    let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"]) else {
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

    let filter_start =
        token_index_for_word_index(inner_tokens, verb_idx + 1).unwrap_or(inner_tokens.len());
    let filter_end =
        token_index_for_word_index(inner_tokens, this_way_idx).unwrap_or(inner_tokens.len());
    if filter_start >= filter_end {
        return Ok(None);
    }

    let filter_tokens = trim_commas(&inner_tokens[filter_start..filter_end]);
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
    let mut clause_tokens = tokens;
    let mut clause_words = token_words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = token_words(clause_tokens);
    }
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let after_prefix = if let Some(rest) =
        grammar::words_match_prefix(clause_tokens, &["for", "each", "player"])
            .or_else(|| grammar::words_match_prefix(clause_tokens, &["for", "each", "players"]))
    {
        rest
    } else if let Some(rest) = grammar::words_match_prefix(clause_tokens, &["each", "player"])
        .or_else(|| grammar::words_match_prefix(clause_tokens, &["each", "players"]))
    {
        rest
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(after_prefix);
    if inner_tokens.len() > 3
        && inner_tokens[0].is_word("who")
        && inner_tokens[1].is_word("controls")
    {
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
                clause_words.join(" ")
            ))
        })?;

        let filter_tokens = trim_commas(&inner_tokens[2..effect_start]);
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'each player who controls' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let (controls_most, normalized_filter_tokens) =
            if let Some(rest) = grammar::words_match_prefix(&filter_tokens, &["the", "most"]) {
                (true, trim_commas(rest))
            } else if let Some(rest) = grammar::words_match_prefix(&filter_tokens, &["most"]) {
                (true, trim_commas(rest))
            } else {
                (false, filter_tokens)
            };
        if normalized_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter after 'most' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let filter = parse_object_filter(&normalized_filter_tokens, false)?;

        let effect_tokens = trim_commas(&inner_tokens[effect_start..]);
        let branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
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

    let inner_words = token_words(&inner_tokens);
    if inner_words.first().copied() == Some("who") {
        let tapped_land_turn_idx = find_tapped_land_for_mana_this_turn_end(&inner_words);
        if let Some(turn_idx) = tapped_land_turn_idx {
            let effect_token_start = token_index_for_word_index(&inner_tokens, turn_idx + 1)
                .unwrap_or(inner_tokens.len());
            let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
            if effect_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing effect after 'each player who tapped a land for mana this turn' (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
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
    if inner_words.first().copied() == Some("who")
        && let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words)
    {
        let effect_token_start = if let Some(comma_idx) =
            find_token_index(&inner_tokens, |token| token.is_comma())
        {
            comma_idx + 1
        } else if let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
        {
            token_index_for_word_index(&inner_tokens, this_way_idx + 2)
                .unwrap_or(inner_tokens.len())
        } else {
            token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
                .unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect in for each player who doesn't clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        let predicate = parse_negated_who_this_way_predicate(&inner_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDoesNot { effects, predicate }));
    }
    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = find_word_sequence_index(&inner_words, &["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_words.join(" ")
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
            token_index_for_word_index(&inner_tokens, 2).unwrap_or(inner_tokens.len())
        };
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who does' (clause: '{}')",
                clause_words.join(" ")
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

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain(&inner_tokens)?
    };
    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

use super::super::grammar::primitives::{self as grammar, find_phrase_start};
use super::super::keyword_static::parse_pt_modifier;
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::parse_object_filter;
use super::super::token_primitives::{find_window_by, rfind_index};
use super::super::util::{
    is_article, is_source_reference_words, parse_target_phrase, parse_value, span_from_tokens,
    token_index_for_word_index, trim_commas,
};
use super::zone_counter_helpers::{split_until_source_leaves_tail, target_object_filter_mut};
use super::zone_handlers::collapse_leading_signed_pt_modifier_tokens;
use super::{apply_where_x_to_damage_amounts, find_verb, parse_simple_gain_ability_clause};
use crate::cards::builders::{
    CardTextError, EffectAst, ExtraTurnAnchorAst, IT_TAG, PlayerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, TagKey, TargetAst, Verb,
};
use crate::effect::{EventValueSpec, Until, Value};
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

const THAT_MUCH_PREFIXES: &[&[&str]] = &[&["that", "much"]];

fn find_token_word_window(tokens: &[OwnedLexToken], expected: &[&str]) -> Option<usize> {
    if expected.is_empty() || tokens.len() < expected.len() {
        return None;
    }

    for start in 0..=tokens.len() - expected.len() {
        if tokens[start..start + expected.len()]
            .iter()
            .zip(expected.iter())
            .all(|(token, expected_word)| token.is_word(expected_word))
        {
            return Some(start);
        }
    }

    None
}

fn contains_word_window(words: &[&str], pattern: &[&str]) -> bool {
    if pattern.is_empty() || words.len() < pattern.len() {
        return false;
    }

    for start in 0..=words.len() - pattern.len() {
        if words[start..start + pattern.len()]
            .iter()
            .zip(pattern.iter())
            .all(|(word, expected)| word == expected)
        {
            return true;
        }
    }

    false
}

fn find_phrase_start_words(words: &[&str], phrase: &[&str]) -> Option<usize> {
    if phrase.is_empty() || words.len() < phrase.len() {
        return None;
    }
    (0..=words.len() - phrase.len()).find(|start| {
        words[*start..*start + phrase.len()]
            .iter()
            .zip(phrase.iter())
            .all(|(word, expected)| word == expected)
    })
}

pub(crate) fn find_same_name_reference_span(
    tokens: &[OwnedLexToken],
) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("with") {
            continue;
        }
        if idx + 6 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
            && tokens[idx + 5].is_word("that")
        {
            return Ok(Some((idx, idx + 7)));
        }
        if idx + 5 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
            && tokens[idx + 4].is_word("that")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
        if idx + 3 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

pub(crate) fn strip_same_controller_reference(
    tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool) {
    let mut cleaned = Vec::with_capacity(tokens.len());
    let mut idx = 0usize;
    let mut same_controller = false;
    while idx < tokens.len() {
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("that")
            && tokens[idx + 1].is_word("player")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("its")
            && tokens[idx + 1].is_word("controller")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 3 < tokens.len()
            && tokens[idx].is_word("that")
            && (tokens[idx + 1].is_word("creature")
                || tokens[idx + 1].is_word("permanent")
                || tokens[idx + 1].is_word("card"))
            && tokens[idx + 2].is_word("controller")
            && (tokens[idx + 3].is_word("control") || tokens[idx + 3].is_word("controls"))
        {
            same_controller = true;
            idx += 4;
            continue;
        }

        cleaned.push(tokens[idx].clone());
        idx += 1;
    }

    (cleaned, same_controller)
}

pub(crate) fn parse_same_name_fanout_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((same_start, same_end)) = find_same_name_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..same_start]);
    filter_tokens.extend_from_slice(&tokens[same_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in same-name fanout clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let (cleaned_tokens, same_controller) = strip_same_controller_reference(&filter_tokens);
    let cleaned_tokens = trim_commas(&cleaned_tokens);
    if cleaned_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing base object filter in same-name fanout clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&cleaned_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported same-name fanout filter (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    if same_controller {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameControllerAsTagged,
        });
    }
    Ok(Some(filter))
}

pub(crate) fn parse_same_name_target_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    let Some(first_word) = words_all.first().copied() else {
        return Ok(None);
    };

    let deal_tokens: Option<&[OwnedLexToken]> = if first_word == "deal" {
        Some(tokens)
    } else if let Some((Verb::Deal, verb_idx)) = find_verb(tokens) {
        let subject_words: Vec<&str> = crate::runtime_backend::token_word_refs(&tokens[..verb_idx])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        if is_source_reference_words(&subject_words) {
            Some(&tokens[verb_idx..])
        } else {
            None
        }
    } else {
        None
    };

    if let Some(deal_tokens) = deal_tokens {
        let deal_words = crate::runtime_backend::token_word_refs(deal_tokens);
        let (amount, used) =
            if deal_words.get(1) == Some(&"that") && deal_words.get(2) == Some(&"much") {
                (Value::EventValue(EventValueSpec::Amount), 2usize)
            } else if let Some((value, used)) = parse_value(&deal_tokens[1..]) {
                (value, used)
            } else {
                return Ok(None);
            };

        let after_amount = &deal_tokens[1 + used..];
        if !after_amount
            .first()
            .is_some_and(|token| token.is_word("damage"))
        {
            return Ok(None);
        }

        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }

        let Some(split_idx) = find_token_word_window(target_tokens, &["and", "each", "other"])
        else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }

        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::subject_verb_damage(amount.clone(), first_target),
            EffectAst::subject_verb_damage_each(amount, filter),
        ]));
    }

    let verb = first_word;
    if verb != "destroy" && verb != "exile" && verb != "return" {
        return Ok(None);
    }

    let Some(and_idx) = find_token_word_window(tokens, &["and", "all", "other"]) else {
        return Ok(None);
    };
    if and_idx <= 1 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&tokens[1..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }

    let second_clause_tokens = if verb == "return" {
        let to_idx = rfind_index(tokens, |token| token.is_word("to")).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing return destination in same-name fanout clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
        if to_idx <= and_idx + 3 {
            return Err(CardTextError::ParseError(format!(
                "missing same-name filter before return destination (clause: '{}')",
                words_all.join(" ")
            )));
        }
        if !grammar::contains_word(&tokens[to_idx + 1..], "hand")
            && !grammar::contains_word(&tokens[to_idx + 1..], "hands")
        {
            return Ok(None);
        }
        tokens[and_idx + 3..to_idx].to_vec()
    } else {
        tokens[and_idx + 3..].to_vec()
    };

    if second_clause_tokens.is_empty() {
        return Ok(None);
    }

    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let mut first_target = parse_target_phrase(&first_target_tokens)?;
    if verb == "return"
        && let Some(first_filter) = target_object_filter_mut(&mut first_target)
    {
        if first_filter.zone.is_none() {
            first_filter.zone = filter.zone;
            if first_filter.zone.is_none() && grammar::contains_word(tokens, "graveyard") {
                first_filter.zone = Some(Zone::Graveyard);
            }
        }
        if first_filter.owner.is_none() {
            first_filter.owner = filter.owner.clone();
            if first_filter.owner.is_none()
                && contains_word_window(&words_all, &["your", "graveyard"])
            {
                first_filter.owner = Some(PlayerFilter::You);
            }
        }
    }
    let first_effect = match verb {
        "destroy" => EffectAst::subject_verb_destroy(first_target),
        "exile" => {
            if until_source_leaves {
                EffectAst::subject_verb_exile_until_source_leaves(first_target, false)
            } else {
                EffectAst::subject_verb_exile(first_target, false)
            }
        }
        "return" => EffectAst::subject_verb_return_to_hand(first_target, false),
        _ => unreachable!("verb already filtered"),
    };
    let second_effect = match verb {
        "destroy" => EffectAst::subject_verb_destroy_all(filter),
        "exile" => {
            if until_source_leaves {
                EffectAst::subject_verb_exile_until_source_leaves(
                    TargetAst::Object(filter, None, None),
                    false,
                )
            } else {
                EffectAst::subject_verb_exile_all(filter, false)
            }
        }
        "return" => EffectAst::subject_verb_return_all_to_hand(filter),
        _ => unreachable!("verb already filtered"),
    };

    Ok(Some(vec![first_effect, second_effect]))
}

pub(crate) fn find_shares_color_reference_span(
    tokens: &[OwnedLexToken],
) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("that") {
            continue;
        }
        if idx + 5 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
            && tokens[idx + 5].is_word("it")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'it' in shares-color clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

pub(crate) fn parse_shared_color_fanout_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((share_start, share_end)) = find_shares_color_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..share_start]);
    filter_tokens.extend_from_slice(&tokens[share_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in shared-color fanout clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported shared-color fanout filter (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SharesColorWithTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    Ok(Some(filter))
}

fn split_full_shared_color_target(target: &TargetAst) -> Option<(TargetAst, ObjectFilter)> {
    let TargetAst::Object(filter, explicit_span, extra_span) = target else {
        return None;
    };
    let has_shared_color = filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.relation == TaggedOpbjectRelation::SharesColorWithTagged);
    if !filter.other || !has_shared_color {
        return None;
    }

    let mut first_filter = filter.clone();
    first_filter.other = false;
    first_filter.tagged_constraints.retain(|constraint| {
        !matches!(
            constraint.relation,
            TaggedOpbjectRelation::SharesColorWithTagged | TaggedOpbjectRelation::IsNotTaggedObject
        )
    });

    Some((
        TargetAst::Object(first_filter, *explicit_span, *extra_span),
        filter.clone(),
    ))
}

fn parse_explicit_shared_color_gets_or_gains(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = crate::runtime_backend::token_word_refs(tokens);
    let find_and_each_other =
        |scope: &[OwnedLexToken]| find_token_word_window(scope, &["and", "each", "other"]);

    let Some(and_idx) = find_and_each_other(tokens) else {
        return Ok(None);
    };
    if and_idx == 0 {
        return Ok(None);
    }

    let verb_idx = tokens[and_idx + 3..]
        .iter()
        .position(|token| {
            token.is_word("get")
                || token.is_word("gets")
                || token.is_word("gain")
                || token.is_word("gains")
        })
        .map(|idx| and_idx + 3 + idx);
    let Some(verb_token_idx) = verb_idx else {
        return Ok(None);
    };

    let first_target_tokens = trim_commas(&tokens[..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }

    let second_clause_tokens = trim_commas(&tokens[and_idx + 3..verb_token_idx]);
    if second_clause_tokens.is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };
    let first_target = parse_target_phrase(&first_target_tokens)?;

    if tokens[verb_token_idx].is_word("get") || tokens[verb_token_idx].is_word("gets") {
        let modifier_tokens = &tokens[verb_token_idx + 1..];
        let modifier_word = modifier_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing modifier in shared-color gets clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
        let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;

        return Ok(Some(vec![
            EffectAst::subject_verb_pump(
                Value::Fixed(power),
                Value::Fixed(toughness),
                first_target,
                Until::EndOfTurn,
                None,
            ),
            EffectAst::subject_verb_pump_all(
                filter,
                Value::Fixed(power),
                Value::Fixed(toughness),
                Until::EndOfTurn,
            ),
        ]));
    }

    let mut first_clause = first_target_tokens.clone();
    first_clause.extend_from_slice(&tokens[verb_token_idx..]);
    let Some(first_effect) = parse_simple_gain_ability_clause(&first_clause)? else {
        return Ok(None);
    };
    let (abilities, duration) = match first_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    abilities,
                    duration,
                    ..
                }
                | SubjectVerbActionAst::GrantAbilitiesAll {
                    abilities,
                    duration,
                    ..
                },
            ..
        }) => (abilities, duration),
        _ => return Ok(None),
    };

    Ok(Some(vec![
        EffectAst::subject_verb_grant_abilities_to_target(
            first_target,
            abilities.clone(),
            duration.clone(),
        ),
        EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration),
    ]))
}

pub(crate) fn parse_shared_color_target_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(effects) = parse_explicit_shared_color_gets_or_gains(tokens)? {
        return Ok(Some(effects));
    }

    let words_all = crate::runtime_backend::token_word_refs(tokens);
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    let Some(verb_token_idx) = token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    let find_and_each_other =
        |scope: &[OwnedLexToken]| find_token_word_window(scope, &["and", "each", "other"]);

    if matches!(verb, Verb::Destroy | Verb::Exile | Verb::Untap) {
        let after_verb = &tokens[verb_token_idx + 1..];
        let Some(split_idx) = find_and_each_other(after_verb) else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&after_verb[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = after_verb[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        let mut effects = Vec::with_capacity(2);
        match verb {
            Verb::Destroy => {
                effects.push(EffectAst::subject_verb_destroy(first_target));
                effects.push(EffectAst::subject_verb_destroy_all(filter));
            }
            Verb::Exile => {
                effects.push(EffectAst::subject_verb_exile(first_target, false));
                effects.push(EffectAst::subject_verb_exile_all(filter, false));
            }
            Verb::Untap => {
                effects.push(EffectAst::subject_verb_untap(first_target));
                effects.push(EffectAst::subject_verb_untap_all(filter));
            }
            _ => return Ok(None),
        }
        return Ok(Some(effects));
    }

    if verb == Verb::Deal {
        let after_verb = &tokens[verb_token_idx + 1..];
        let (amount, used) = if let Some((prefix, _)) =
            grammar::words_match_any_prefix(after_verb, THAT_MUCH_PREFIXES)
        {
            (Value::EventValue(EventValueSpec::Amount), prefix.len())
        } else if let Some((value, used)) = parse_value(after_verb) {
            (value, used)
        } else {
            return Ok(None);
        };

        let after_amount = &after_verb[used..];
        if !after_amount
            .first()
            .is_some_and(|token| token.is_word("damage"))
        {
            return Ok(None);
        }
        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }
        let Some(split_idx) = find_and_each_other(target_tokens) else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::subject_verb_damage(amount.clone(), first_target),
            EffectAst::subject_verb_damage_each(amount, filter),
        ]));
    }

    if words_all.first().copied() == Some("prevent") {
        let mut idx = verb_token_idx + 1;
        if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
            idx += 1;
        }
        if !tokens.get(idx).is_some_and(|token| token.is_word("next")) {
            return Ok(None);
        }
        idx += 1;
        let amount_token = tokens.get(idx).cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing prevent damage amount (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
        let Some((amount, _)) = parse_value(&[amount_token]) else {
            return Ok(None);
        };
        idx += 1;
        if !tokens.get(idx).is_some_and(|token| token.is_word("damage")) {
            return Ok(None);
        }
        idx += 1;
        if tokens.get(idx..idx + 4).is_none_or(|window| {
            !window[0].is_word("that")
                || !window[1].is_word("would")
                || !window[2].is_word("be")
                || !window[3].is_word("dealt")
        }) {
            return Ok(None);
        }
        idx += 4;
        if !tokens.get(idx).is_some_and(|token| token.is_word("to")) {
            return Ok(None);
        }
        idx += 1;

        let Some(this_turn_rel) = find_phrase_start_words(
            &crate::runtime_backend::token_word_refs(&tokens[idx..]),
            &["this", "turn"],
        ) else {
            return Ok(None);
        };
        let this_turn_abs = idx + this_turn_rel;
        if this_turn_abs + 2 != tokens.len() {
            return Ok(None);
        }

        let scope_tokens = &tokens[idx..this_turn_abs];
        let Some(split_idx) = find_and_each_other(scope_tokens) else {
            return Ok(None);
        };

        let first_target_tokens = trim_commas(&scope_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = scope_tokens[split_idx + 3..].to_vec();
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;

        return Ok(Some(vec![
            EffectAst::subject_verb_prevent_damage(amount.clone(), first_target, Until::EndOfTurn),
            EffectAst::subject_verb_prevent_damage_each(amount, filter, Until::EndOfTurn),
        ]));
    }

    if matches!(verb, Verb::Get | Verb::Gain) {
        if verb_idx == 0 || verb_token_idx + 1 >= tokens.len() {
            return Ok(None);
        }

        let subject_tokens = &tokens[..verb_token_idx];
        if let Ok(full_target) = parse_target_phrase(subject_tokens)
            && let Some((first_target, filter)) = split_full_shared_color_target(&full_target)
        {
            if verb == Verb::Get {
                let modifier_tokens = &tokens[verb_token_idx + 1..];
                let modifier_word = modifier_tokens
                    .first()
                    .and_then(OwnedLexToken::as_word)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "missing modifier in shared-color gets clause (clause: '{}')",
                            words_all.join(" ")
                        ))
                    })?;
                let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                        words_all.join(" ")
                    ))
                })?;

                return Ok(Some(vec![
                    EffectAst::subject_verb_pump(
                        Value::Fixed(power),
                        Value::Fixed(toughness),
                        first_target,
                        Until::EndOfTurn,
                        None,
                    ),
                    EffectAst::subject_verb_pump_all(
                        filter,
                        Value::Fixed(power),
                        Value::Fixed(toughness),
                        Until::EndOfTurn,
                    ),
                ]));
            }

            if let Some(first_effect) = parse_simple_gain_ability_clause(tokens)?
                && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::GrantAbilitiesToTarget {
                            abilities,
                            duration,
                            ..
                        },
                    ..
                }) = first_effect
            {
                return Ok(Some(vec![
                    EffectAst::subject_verb_grant_abilities_to_target(
                        first_target,
                        abilities.clone(),
                        duration.clone(),
                    ),
                    EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration),
                ]));
            }
        }

        let Some(and_idx) = find_and_each_other(subject_tokens) else {
            return Ok(None);
        };
        if and_idx == 0 {
            return Ok(None);
        }

        let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;

        if verb == Verb::Get {
            let modifier_tokens = &tokens[verb_token_idx + 1..];
            let modifier_word = modifier_tokens
                .first()
                .and_then(OwnedLexToken::as_word)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing modifier in shared-color gets clause (clause: '{}')",
                        words_all.join(" ")
                    ))
                })?;
            let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
                CardTextError::ParseError(format!(
                    "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;

            return Ok(Some(vec![
                EffectAst::subject_verb_pump(
                    Value::Fixed(power),
                    Value::Fixed(toughness),
                    first_target,
                    Until::EndOfTurn,
                    None,
                ),
                EffectAst::subject_verb_pump_all(
                    filter,
                    Value::Fixed(power),
                    Value::Fixed(toughness),
                    Until::EndOfTurn,
                ),
            ]));
        }

        let mut first_clause = first_target_tokens.clone();
        first_clause.extend_from_slice(&tokens[verb_token_idx..]);
        let Some(first_effect) = parse_simple_gain_ability_clause(&first_clause)? else {
            return Ok(None);
        };
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    abilities,
                    duration,
                    ..
                },
            ..
        }) = first_effect
        {
            return Ok(Some(vec![
                EffectAst::subject_verb_grant_abilities_to_target(
                    first_target,
                    abilities.clone(),
                    duration.clone(),
                ),
                EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration),
            ]));
        }
    }

    Ok(None)
}

#[derive(Debug, Clone)]
enum CompoundDamagePart {
    Target(TargetAst),
    EachObject(ObjectFilter),
    EachPlayer(PlayerFilter),
}

fn strip_trailing_damage_noise(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut cleaned = trim_commas(tokens);
    while cleaned
        .last()
        .is_some_and(|token| token.is_word("instead") || token.is_comma())
    {
        cleaned.pop();
    }
    cleaned
}

fn strip_trailing_where_clause(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let where_idx = tokens
        .iter()
        .position(|token| token.is_word("where"))
        .unwrap_or(tokens.len());
    strip_trailing_damage_noise(&tokens[..where_idx])
}

fn words_have_suffix(words: &[&str], suffix: &[&str]) -> bool {
    words.len() >= suffix.len() && words[words.len() - suffix.len()..] == *suffix
}

fn strip_word_suffix(tokens: &[OwnedLexToken], suffix: &[&str]) -> Option<Vec<OwnedLexToken>> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words_have_suffix(&words, suffix) {
        return None;
    }
    let base_word_len = words.len().saturating_sub(suffix.len());
    let token_end = if base_word_len == 0 {
        0
    } else if base_word_len >= words.len() {
        tokens.len()
    } else {
        token_index_for_word_index(tokens, base_word_len).unwrap_or(base_word_len)
    };
    Some(strip_trailing_damage_noise(&tokens[..token_end]))
}

fn tokens_before_word(tokens: &[OwnedLexToken], word_idx: usize) -> Vec<OwnedLexToken> {
    let token_end = if word_idx == 0 {
        0
    } else {
        token_index_for_word_index(tokens, word_idx).unwrap_or(word_idx)
    };
    strip_trailing_damage_noise(&tokens[..token_end])
}

fn target_context_for_damage_part(part: &CompoundDamagePart) -> Option<PlayerFilter> {
    match part {
        CompoundDamagePart::Target(TargetAst::Player(filter, span))
        | CompoundDamagePart::Target(TargetAst::PlayerOrPlaneswalker(filter, span)) => {
            if span.is_some() {
                Some(PlayerFilter::Target(Box::new(filter.clone())))
            } else {
                Some(filter.clone())
            }
        }
        CompoundDamagePart::EachPlayer(_) => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

fn strip_known_controller_tail(
    tokens: &[OwnedLexToken],
    player_context: Option<PlayerFilter>,
) -> (Vec<OwnedLexToken>, Option<PlayerFilter>) {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let has_controller_controls_tail = words_have_suffix(&words, &["controller", "controls"])
        || words_have_suffix(&words, &["controller", "control"])
        || words_have_suffix(&words, &["controllers", "controls"])
        || words_have_suffix(&words, &["controllers", "control"]);
    if has_controller_controls_tail && words.len() >= 6 {
        for start in 0..words.len().saturating_sub(4) {
            if words[start..].starts_with(&["that", "player", "or", "that"])
                && words[start + 4..words.len().saturating_sub(2)]
                    .iter()
                    .any(|word| word.starts_with("planeswalker"))
            {
                return (
                    tokens_before_word(tokens, start),
                    Some(PlayerFilter::TargetPlayerOrControllerOfTarget),
                );
            }
        }
    }

    for suffix in [
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "controls",
        ][..],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalker",
            "controller",
            "controls",
        ][..],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalker",
            "s",
            "controller",
            "controls",
        ][..],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalkers",
            "controller",
            "control",
        ][..],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalker",
            "s",
            "controller",
            "control",
        ][..],
        &[
            "that",
            "player",
            "or",
            "that",
            "planeswalker",
            "controller",
            "control",
        ][..],
    ] {
        if let Some(base) = strip_word_suffix(tokens, suffix) {
            return (base, Some(PlayerFilter::TargetPlayerOrControllerOfTarget));
        }
    }

    for suffix in [
        &["that", "player", "controls"][..],
        &["that", "player", "control"][..],
        &["they", "control"][..],
        &["they", "controls"][..],
    ] {
        if let Some(base) = strip_word_suffix(tokens, suffix) {
            return (
                base,
                Some(player_context.unwrap_or_else(PlayerFilter::target_player)),
            );
        }
    }

    for suffix in [
        &["your", "opponents", "control"][..],
        &["your", "opponent", "controls"][..],
    ] {
        if let Some(base) = strip_word_suffix(tokens, suffix) {
            return (base, Some(PlayerFilter::Opponent));
        }
    }

    for suffix in [
        &["you", "control"][..],
        &["you", "controls"][..],
        &["your", "control"][..],
    ] {
        if let Some(base) = strip_word_suffix(tokens, suffix) {
            return (base, Some(PlayerFilter::You));
        }
    }

    (tokens.to_vec(), None)
}

fn parse_each_damage_part(
    tokens: &[OwnedLexToken],
    player_context: Option<PlayerFilter>,
) -> Result<Option<CompoundDamagePart>, CardTextError> {
    let tokens = strip_trailing_damage_noise(tokens);
    let words = crate::runtime_backend::token_word_refs(&tokens);
    if words.is_empty() {
        return Ok(None);
    }

    match words.as_slice() {
        ["player"] | ["players"] => {
            return Ok(Some(CompoundDamagePart::EachPlayer(PlayerFilter::Any)));
        }
        ["opponent"] | ["opponents"] => {
            return Ok(Some(CompoundDamagePart::EachPlayer(PlayerFilter::Opponent)));
        }
        _ => {}
    }

    if words
        .first()
        .is_some_and(|word| *word == "player" || *word == "players")
    {
        return Ok(None);
    }

    let (filter_tokens, controller) = strip_known_controller_tail(&tokens, player_context);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };
    if filter.controller.is_none() {
        filter.controller = controller;
    }
    Ok(Some(CompoundDamagePart::EachObject(filter)))
}

fn parse_damage_part(
    tokens: &[OwnedLexToken],
    player_context: Option<PlayerFilter>,
) -> Result<Option<CompoundDamagePart>, CardTextError> {
    let tokens = strip_trailing_damage_noise(tokens);
    let words = crate::runtime_backend::token_word_refs(&tokens);
    if words.is_empty() {
        return Ok(None);
    }

    if matches!(words.first().copied(), Some("each" | "all")) {
        return parse_each_damage_part(&tokens[1..], player_context);
    }

    if words.as_slice() == ["you"] {
        return Ok(Some(CompoundDamagePart::Target(TargetAst::Player(
            PlayerFilter::You,
            span_from_tokens(&tokens),
        ))));
    }

    if words.as_slice() == ["opponent"] || words.as_slice() == ["opponents"] {
        return Ok(Some(CompoundDamagePart::Target(TargetAst::Player(
            PlayerFilter::Opponent,
            span_from_tokens(&tokens),
        ))));
    }

    if words.iter().any(|word| *word == "target") {
        return Ok(Some(CompoundDamagePart::Target(parse_target_phrase(
            &tokens,
        )?)));
    }

    Ok(None)
}

fn damage_player_iteration_effect(filter: PlayerFilter, effects: Vec<EffectAst>) -> EffectAst {
    match filter {
        PlayerFilter::Opponent => EffectAst::ForEachOpponent { effects },
        PlayerFilter::Any => EffectAst::ForEachPlayer { effects },
        other => EffectAst::ForEachPlayersFiltered {
            filter: other,
            effects,
        },
    }
}

fn compound_damage_part_to_effect(part: CompoundDamagePart, amount: Value) -> EffectAst {
    match part {
        CompoundDamagePart::Target(target) => EffectAst::subject_verb_damage(amount, target),
        CompoundDamagePart::EachObject(filter) => {
            EffectAst::subject_verb_damage_each(amount, filter)
        }
        CompoundDamagePart::EachPlayer(filter) => damage_player_iteration_effect(
            filter,
            vec![EffectAst::subject_verb_damage(
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
        ),
    }
}

fn compound_damage_effects(
    amount: Value,
    left: CompoundDamagePart,
    right: CompoundDamagePart,
) -> Vec<EffectAst> {
    match left {
        CompoundDamagePart::EachPlayer(filter) => {
            let mut nested = vec![EffectAst::subject_verb_damage(
                amount.clone(),
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )];
            nested.push(compound_damage_part_to_effect(right, amount));
            vec![damage_player_iteration_effect(filter, nested)]
        }
        other => vec![
            compound_damage_part_to_effect(other, amount.clone()),
            compound_damage_part_to_effect(right, amount),
        ],
    }
}

fn equal_damage_target_tail_starts_like_destination(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    matches!(
        words.first().copied(),
        Some("each" | "all" | "target" | "you" | "opponent" | "opponents" | "player" | "players")
    )
}

fn parse_equal_damage_amount_and_targets(
    tokens: &[OwnedLexToken],
) -> Option<(Value, Vec<OwnedLexToken>)> {
    if !tokens.first().is_some_and(|token| token.is_word("damage"))
        || !tokens.get(1).is_some_and(|token| token.is_word("equal"))
        || !tokens.get(2).is_some_and(|token| token.is_word("to"))
    {
        return None;
    }

    for target_to_idx in 3..tokens.len() {
        if !tokens[target_to_idx].is_word("to") {
            continue;
        }
        let target_tail = &tokens[target_to_idx + 1..];
        if !equal_damage_target_tail_starts_like_destination(target_tail) {
            continue;
        }
        let amount_tokens = trim_commas(&tokens[3..target_to_idx]);
        let (amount, used) = parse_value(&amount_tokens)?;
        if used != amount_tokens.len() {
            continue;
        }
        return Some((amount, target_tail.to_vec()));
    }

    None
}

pub(crate) fn parse_compound_damage_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let deal_tokens = if tokens.first().is_some_and(|token| token.is_word("deal")) {
        tokens
    } else if let Some((Verb::Deal, verb_idx)) = find_verb(tokens) {
        &tokens[verb_idx..]
    } else {
        return Ok(None);
    };

    let after_deal = &deal_tokens[1..];
    let (amount, target_tokens) =
        if let Some((amount, target_tokens)) = parse_equal_damage_amount_and_targets(after_deal) {
            (amount, target_tokens)
        } else {
            let deal_words = crate::runtime_backend::token_word_refs(deal_tokens);
            let (amount, used) =
                if deal_words.get(1) == Some(&"that") && deal_words.get(2) == Some(&"much") {
                    (Value::EventValue(EventValueSpec::Amount), 2usize)
                } else if let Some((value, used)) = parse_value(after_deal) {
                    (value, used)
                } else {
                    return Ok(None);
                };

            let after_amount = &after_deal[used..];
            if !after_amount
                .first()
                .is_some_and(|token| token.is_word("damage"))
            {
                return Ok(None);
            }

            let mut target_tokens = &after_amount[1..];
            if target_tokens
                .first()
                .is_some_and(|token| token.is_word("to"))
            {
                target_tokens = &target_tokens[1..];
            }
            (amount, target_tokens.to_vec())
        };
    let target_tokens = strip_trailing_where_clause(&target_tokens);
    if target_tokens.is_empty() {
        return Ok(None);
    }

    let Some(split_idx) = find_token_word_window(&target_tokens, &["and", "each"])
        .or_else(|| find_token_word_window(&target_tokens, &["and", "all"]))
    else {
        return Ok(None);
    };
    if split_idx == 0 || split_idx + 2 >= target_tokens.len() {
        return Ok(None);
    }

    let left_tokens = trim_commas(&target_tokens[..split_idx]);
    let right_tokens = trim_commas(&target_tokens[split_idx + 2..]);
    let Some(left) = parse_damage_part(&left_tokens, None)? else {
        return Ok(None);
    };
    let right_context = target_context_for_damage_part(&left);
    let Some(right) = parse_each_damage_part(&right_tokens, right_context)? else {
        return Ok(None);
    };

    let mut effects = compound_damage_effects(amount, left, right);
    apply_where_x_to_damage_amounts(tokens, &mut effects)?;
    Ok(Some(effects))
}

pub(crate) fn parse_same_name_gets_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    if verb != Verb::Get || verb_idx == 0 || verb_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..verb_idx];
    let Some(and_idx) = find_token_word_window(subject_tokens, &["and", "all", "other"]) else {
        return Ok(None);
    };
    if and_idx == 0 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }
    let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
    if second_clause_tokens.is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let modifier_tokens = &tokens[verb_idx + 1..];
    let collapsed_modifier_tokens = collapse_leading_signed_pt_modifier_tokens(modifier_tokens)
        .unwrap_or_else(|| modifier_tokens.to_vec());
    let modifier_word = collapsed_modifier_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing modifier in same-name gets clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in same-name gets clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        ))
    })?;
    let first_target = parse_target_phrase(&first_target_tokens)?;

    Ok(Some(vec![
        EffectAst::subject_verb_pump(
            Value::Fixed(power),
            Value::Fixed(toughness),
            first_target,
            Until::EndOfTurn,
            None,
        ),
        EffectAst::subject_verb_pump_all(
            filter,
            Value::Fixed(power),
            Value::Fixed(toughness),
            Until::EndOfTurn,
        ),
    ]))
}

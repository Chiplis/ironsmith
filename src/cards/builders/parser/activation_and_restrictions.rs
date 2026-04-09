#![allow(dead_code)]

use super::activation_helpers::{
    contains_discard_source_phrase, contains_source_from_your_graveyard_phrase,
    contains_source_from_your_hand_phrase, find_activation_cost_start, is_article,
    is_basic_color_word, is_comparison_or_delimiter, is_source_from_your_graveyard_words,
    join_sentences_with_period, parse_add_mana, parse_filter_comparison_tokens,
    parse_next_end_step_token_delay_flags, parse_subtype_flexible, split_cost_segments,
    value_contains_unbound_x,
};
use super::effect_ast_traversal::{for_each_nested_effects, for_each_nested_effects_mut};
use super::effect_sentences::find_verb;
use super::effect_sentences::{
    is_beginning_of_end_step_words, is_end_of_combat_words, is_negated_untap_clause,
    parse_effect_sentence_lexed, parse_effect_sentences_lexed, parse_mana_symbol,
    parse_mana_symbol_group, parse_restriction_duration, parse_scryfall_mana_cost,
    parse_subtype_word, parse_supertype_word, replace_unbound_x_in_effect_anywhere,
    strip_leading_articles, trim_edge_punctuation,
};
use super::grammar::primitives as grammar;
use super::keyword_static::{
    parse_add_mana_equal_amount_value, parse_cost_modifier_amount, parse_cost_modifier_mana_cost,
    parse_dynamic_cost_modifier_value, parse_static_condition_clause, parse_where_x_value_clause,
    parse_where_x_value_clause_lexed,
};
use super::leaf::{lower_activation_cost_cst, parse_activation_cost_tokens_rewrite};
use super::lexer::{OwnedLexToken, TokenKind};
use super::object_filters::{
    find_word_slice_phrase_start, parse_object_filter, parse_object_filter_lexed,
};
use super::token_primitives::{
    contains_window, find_index, find_window_by, find_window_index, lexed_head_words, rfind_index,
    slice_contains, slice_ends_with, slice_starts_with, slice_strip_prefix, slice_strip_suffix,
    str_strip_prefix, str_strip_suffix,
};
use super::util::{
    is_source_reference_words, mana_pips_from_token, parse_card_type, parse_color,
    parse_counter_type_from_tokens, parse_non_type, parse_number, parse_number_word_u32,
    parse_subject, parse_target_count_range_prefix, parse_target_phrase, span_from_tokens,
    token_index_for_word_index, trim_commas, words,
};
#[allow(unused_imports)]
use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
use crate::cards::builders::{
    CardTextError, DamageBySpec, EffectAst, IT_TAG, KeywordAction, LineAst, ParsedAbility,
    PlayerAst, PredicateAst, ReferenceImports, ReturnControllerAst, StaticAbilityAst, TagKey,
    TargetAst, TextSpan, TriggerSpec,
};
use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::effect::{ChoiceCount, Effect, Until, Value};
use crate::filter::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::mana::{ManaCost, ManaSymbol};
use crate::object::CounterType;
use crate::static_abilities::StaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

mod activated_sentence_parsers;

use activated_sentence_parsers::collect_activated_sentence_modifiers;

type ActivationRestrictionCompatWords<'a> = grammar::TokenWordView<'a>;

fn strip_prefix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &'static [&'static str],
) -> Option<&'a [OwnedLexToken]> {
    grammar::parse_prefix(tokens, grammar::phrase(phrase)).map(|(_, rest)| rest)
}

fn strip_prefix_phrases<'a>(
    tokens: &'a [OwnedLexToken],
    phrases: &[&'static [&'static str]],
) -> Option<(&'static [&'static str], &'a [OwnedLexToken])> {
    phrases
        .iter()
        .find_map(|phrase| strip_prefix_phrase(tokens, phrase).map(|rest| (*phrase, rest)))
}

fn joined_activation_clause_text(tokens: &[OwnedLexToken]) -> String {
    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
}

fn parse_prefixed_activated_ability_label(
    tokens: &[OwnedLexToken],
    cost_start: usize,
) -> Option<String> {
    if cost_start == 0 {
        return None;
    }

    let prefix = ActivationRestrictionCompatWords::new(&tokens[..cost_start]);
    match prefix.get(prefix.len().saturating_sub(1)) {
        Some("boast") => Some("Boast".to_string()),
        Some("renew") => Some("Renew".to_string()),
        _ => None,
    }
}

fn contains_granted_keyword_before_word(
    words: &ActivationRestrictionCompatWords,
    keyword_idx: usize,
) -> bool {
    (0..keyword_idx)
        .filter_map(|idx| words.get(idx))
        .any(|word| matches!(word, "has" | "have"))
}

fn find_cycling_keyword_word_index(words: &ActivationRestrictionCompatWords) -> Option<usize> {
    let mut idx = 0usize;
    while idx < words.len() {
        if words
            .get(idx)
            .is_some_and(|word| str_strip_suffix(word, "cycling").is_some())
        {
            return Some(idx);
        }
        idx += 1;
    }
    None
}

fn parse_hand_keyword_activated_body_lexed(
    body_tokens: &[OwnedLexToken],
    keyword: &str,
    display_label: &str,
    clause_text: &str,
) -> Result<Option<ParsedAbility>, CardTextError> {
    if body_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "{keyword} line missing activated ability body (clause: '{clause_text}')",
        )));
    }

    let ability_tokens = trim_commas(body_tokens);
    let Some(mut parsed) = parse_activated_line_with_raw(&ability_tokens, None)? else {
        return Ok(None);
    };
    parsed.ability.text = Some(display_label.to_string());
    parsed.ability.functional_zones = vec![Zone::Hand];
    Ok(Some(parsed))
}

pub(crate) fn parse_activated_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_activated_line_with_raw(tokens, None)
}

pub(crate) fn parse_activated_line_with_raw(
    tokens: &[OwnedLexToken],
    raw_line: Option<&str>,
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) = find_index(tokens, |token| token.is_colon()) else {
        return Ok(None);
    };

    let cost_start = find_activation_cost_start(&tokens[..colon_idx]).unwrap_or(0);
    let cost_tokens = &tokens[cost_start..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }
    let loyalty_shorthand_cost = parse_loyalty_shorthand_activation_cost(cost_tokens, raw_line);
    let ability_label = parse_prefixed_activated_ability_label(tokens, cost_start);
    let apply_ability_label = |ability: &mut Ability| {
        if ability.text.is_none() {
            if let Some(label) = &ability_label {
                ability.text = Some(label.clone());
            }
        }
    };

    let mut effect_sentences = grammar::split_lexed_slices_on_period(effect_tokens);
    let functional_zones = infer_activated_functional_zones_lexed(cost_tokens, &effect_sentences);
    let mut timing = ActivationTiming::AnyTime;
    let scanned_modifiers = collect_activated_sentence_modifiers(&effect_sentences, timing.clone());
    let mana_activation_condition = scanned_modifiers.mana_activation_condition;
    let mut additional_activation_restrictions =
        scanned_modifiers.additional_activation_restrictions;
    let mana_usage_restrictions = scanned_modifiers.mana_usage_restrictions;
    let inline_effects_ast = scanned_modifiers.inline_effects_ast;
    effect_sentences = scanned_modifiers.kept_sentences;
    timing = scanned_modifiers.timing;
    let mana_activation_condition =
        combine_mana_activation_condition(mana_activation_condition, timing.clone());
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let x_defined_by_cost = activation_cost_mentions_x(cost_tokens);
        let effect_words = ActivationRestrictionCompatWords::new(primary_sentence);
        let is_primary_add_clause = matches!(
            (
                effect_words.get(0),
                effect_words.get(1),
                effect_words.get(2)
            ),
            (Some("add" | "adds"), _, _)
                | (Some("you"), Some("add"), _)
                | (Some("that"), Some("player"), Some("add" | "adds"))
                | (Some("target"), Some("player"), Some("add" | "adds"))
        );
        if is_primary_add_clause {
            let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
                cost.clone()
            } else {
                parse_activation_cost(cost_tokens)?
            };
            let reference_imports = first_sacrifice_cost_choice_tag(&mana_cost)
                .or_else(|| last_exile_cost_choice_tag(&mana_cost))
                .map(ReferenceImports::with_last_object_tag)
                .unwrap_or_default();

            let mut extra_effects_ast = inline_effects_ast.clone();
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    let ast = parse_effect_sentence_lexed(sentence)?;
                    extra_effects_ast.extend(ast);
                }
            }

            let add_token_idx = find_index(primary_sentence, |token| {
                token.is_word("add") || token.is_word("adds")
            })
            .unwrap_or(0);
            let mana_tokens = &primary_sentence[add_token_idx + 1..];
            let mana_subject =
                (add_token_idx > 0).then(|| parse_subject(&primary_sentence[..add_token_idx]));
            let mana_words_view = ActivationRestrictionCompatWords::new(mana_tokens);
            let mana_words = mana_words_view.to_word_refs();
            let has_for_each_tail = mana_words_view.has_phrase(&["for", "each"]);
            let dynamic_amount = if has_for_each_tail {
                Some(
                    parse_dynamic_cost_modifier_value(mana_tokens)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported dynamic mana amount (clause: '{}')",
                            joined_activation_clause_text(primary_sentence)
                        ))
                    })?,
                )
            } else {
                parse_devotion_value_from_add_clause(mana_tokens)?
                    .or_else(|| parse_add_mana_equal_amount_value(mana_tokens))
            };

            let has_imprinted_colors = slice_contains(&mana_words, &"exiled")
                && (slice_contains(&mana_words, &"card") || slice_contains(&mana_words, &"cards"))
                && mana_words
                    .iter()
                    .any(|word| *word == "color" || *word == "colors");
            let has_any_combination_mana =
                contains_word_sequence(&mana_words, &["any", "combination", "of"]);
            let has_any_choice_mana = slice_contains(&mana_words, &"any")
                && (slice_contains(&mana_words, &"color")
                    || slice_contains(&mana_words, &"type")
                    || has_any_combination_mana);
            let has_or_choice_mana = slice_contains(&mana_words, &"or");
            let has_chosen_color =
                slice_contains(&mana_words, &"chosen") && slice_contains(&mana_words, &"color");
            let uses_commander_identity = mana_words
                .iter()
                .any(|word| *word == "commander" || *word == "commanders")
                && slice_contains(&mana_words, &"identity");
            let loyalty_timing = if loyalty_shorthand_cost.is_some() {
                ActivationTiming::SorcerySpeed
            } else {
                ActivationTiming::AnyTime
            };
            let loyalty_restrictions =
                loyalty_additional_restrictions(loyalty_shorthand_cost.is_some());
            let build_additional_restrictions = || {
                let mut restrictions = loyalty_restrictions.clone();
                restrictions.extend(additional_activation_restrictions.clone());
                restrictions
            };
            if has_imprinted_colors
                || has_any_choice_mana
                || has_or_choice_mana
                || uses_commander_identity
                || has_chosen_color
            {
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject.clone())?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let mut ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing.clone(),
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                apply_ability_label(&mut ability);
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast: Some(effects_ast),
                    reference_imports: reference_imports.clone(),
                    trigger_spec: None,
                }));
            }

            let mana: Vec<_> = mana_tokens
                .iter()
                .filter_map(|token| token.as_word())
                .filter(|word| !matches!(*word, "mana" | "to" | "your" | "pool"))
                .filter_map(|word| parse_mana_symbol(word).ok())
                .collect();

            if !mana.is_empty() {
                if dynamic_amount.is_none() && extra_effects_ast.is_empty() {
                    let mut ability = Ability {
                        kind: AbilityKind::Activated(ActivatedAbility {
                            mana_cost,
                            effects: crate::resolution::ResolutionProgram::default(),
                            choices: vec![],
                            timing: loyalty_timing.clone(),
                            additional_restrictions: build_additional_restrictions(),
                            activation_restrictions: vec![],
                            mana_output: Some(mana),
                            activation_condition: mana_activation_condition.clone(),
                            mana_usage_restrictions: mana_usage_restrictions.clone(),
                        }),
                        functional_zones: functional_zones.clone(),
                        text: None,
                    };
                    apply_ability_label(&mut ability);
                    return Ok(Some(ParsedAbility {
                        ability,
                        effects_ast: None,
                        reference_imports: ReferenceImports::default(),
                        trigger_spec: None,
                    }));
                }
                let mut mana_ast = parse_add_mana(mana_tokens, mana_subject)?;
                resolve_activated_mana_x_requirements(
                    &mut mana_ast,
                    primary_sentence,
                    x_defined_by_cost,
                )?;
                let mut ability = Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing: loyalty_timing,
                        additional_restrictions: build_additional_restrictions(),
                        activation_restrictions: vec![],
                        mana_output: Some(vec![]),
                        activation_condition: mana_activation_condition.clone(),
                        mana_usage_restrictions: mana_usage_restrictions.clone(),
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                apply_ability_label(&mut ability);
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast: Some(effects_ast),
                    reference_imports: reference_imports,
                    trigger_spec: None,
                }));
            }
        }
    }

    // Generic activated ability: parse costs and effects from "<costs>: <effects>"
    let mana_cost = if let Some(cost) = &loyalty_shorthand_cost {
        cost.clone()
    } else {
        parse_activation_cost(cost_tokens)?
    };
    let effect_tokens_joined = join_sentences_with_period(
        &effect_sentences
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>(),
    );
    if effect_sentences.is_empty()
        && !additional_activation_restrictions.is_empty()
        && inline_effects_ast.is_empty()
    {
        return Ok(Some(ParsedAbility {
            ability: {
                let mut ability = Ability {
                    kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                        mana_cost,
                        effects: crate::resolution::ResolutionProgram::default(),
                        choices: vec![],
                        timing,
                        additional_restrictions: additional_activation_restrictions,
                        activation_restrictions: vec![],
                        mana_output: None,
                        activation_condition: None,
                        mana_usage_restrictions,
                    }),
                    functional_zones,
                    text: None,
                };
                apply_ability_label(&mut ability);
                ability
            },
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        }));
    }
    let mut effects_ast = parse_effect_sentences_lexed(&effect_tokens_joined)?;
    effects_ast.extend(inline_effects_ast);
    if effects_ast.is_empty() {
        return Ok(None);
    }
    let reference_imports = first_sacrifice_cost_choice_tag(&mana_cost)
        .or_else(|| last_exile_cost_choice_tag(&mana_cost))
        .map(ReferenceImports::with_last_object_tag)
        .unwrap_or_default();
    if loyalty_shorthand_cost.is_some() {
        timing = ActivationTiming::SorcerySpeed;
        for restriction in loyalty_additional_restrictions(true) {
            let already_present = additional_activation_restrictions.iter().any(|existing| {
                let existing_lower = existing.to_ascii_lowercase();
                let restriction_lower = restriction.to_ascii_lowercase();
                existing.eq_ignore_ascii_case(restriction.as_str())
                    || (existing_lower.matches("once each turn").next().is_some()
                        && restriction_lower.matches("once each turn").next().is_some())
            });
            if !already_present {
                additional_activation_restrictions.push(restriction);
            }
        }
    }

    Ok(Some(ParsedAbility {
        ability: {
            let mut ability = Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost,
                    effects: crate::resolution::ResolutionProgram::default(),
                    choices: vec![],
                    timing,
                    additional_restrictions: additional_activation_restrictions,
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions,
                }),
                functional_zones,
                text: None,
            };
            apply_ability_label(&mut ability);
            ability
        },
        effects_ast: Some(effects_ast),
        reference_imports,
        trigger_spec: None,
    }))
}

fn activation_cost_mentions_x(tokens: &[OwnedLexToken]) -> bool {
    tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .any(|word| {
            matches!(word, "x" | "+x" | "-x")
                || word
                    .split('/')
                    .any(|part| matches!(part, "x" | "+x" | "-x"))
        })
}

fn resolve_activated_mana_x_requirements(
    effect: &mut EffectAst,
    sentence_tokens: &[OwnedLexToken],
    x_defined_by_cost: bool,
) -> Result<(), CardTextError> {
    let clause_word_view = ActivationRestrictionCompatWords::new(sentence_tokens);
    let clause_words = clause_word_view.to_word_refs();
    if let Some(where_idx) = find_word_sequence_start(&clause_words, &["where", "x", "is"]) {
        let clause = clause_words.join(" ");
        let where_token_idx =
            token_index_for_word_index(sentence_tokens, where_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map where-x clause in mana ability (clause: '{clause}')"
                ))
            })?;
        let where_tokens = &sentence_tokens[where_token_idx..];
        let where_value = parse_where_x_value_clause_lexed(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in mana ability (clause: '{clause}')"
            ))
        })?;
        replace_unbound_x_in_effect_anywhere(effect, &where_value, &clause)?;
    }

    let x_defined_by_removed_this_way = contains_word_sequence(&clause_words, &["this", "way"])
        && slice_contains(&clause_words, &"removed")
        && clause_words
            .iter()
            .any(|word| matches!(*word, "counter" | "counters"));

    if mana_effect_contains_unbound_x(effect)
        && !x_defined_by_cost
        && !x_defined_by_removed_this_way
    {
        return Err(CardTextError::ParseError(format!(
            "unresolved X in mana ability without an X activation cost or where-x definition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(())
}

fn mana_effect_contains_unbound_x(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. } => value_contains_unbound_x(amount),
        _ => {
            let mut contains_unbound_x = false;
            for_each_nested_effects(effect, true, |nested| {
                if nested.iter().any(mana_effect_contains_unbound_x) {
                    contains_unbound_x = true;
                }
            });
            contains_unbound_x
        }
    }
}

pub(crate) fn parse_loyalty_shorthand_activation_cost(
    cost_tokens: &[OwnedLexToken],
    raw_line: Option<&str>,
) -> Option<TotalCost> {
    let [token] = cost_tokens else {
        return None;
    };
    let word = token.as_word()?;
    if let Some(rest) = str_strip_prefix(word, "+")
        && let Ok(amount) = rest.parse::<u32>()
    {
        return Some(if amount == 0 {
            TotalCost::free()
        } else {
            TotalCost::from_cost(crate::costs::Cost::add_counters(
                CounterType::Loyalty,
                amount,
            ))
        });
    }
    if let Some(rest) = str_strip_prefix(word, "-") {
        if rest.eq_ignore_ascii_case("x") {
            return Some(TotalCost::from_cost(
                crate::costs::Cost::remove_any_counters_from_source(
                    Some(CounterType::Loyalty),
                    true,
                ),
            ));
        }
        if let Ok(amount) = rest.parse::<u32>() {
            return Some(TotalCost::from_cost(crate::costs::Cost::remove_counters(
                CounterType::Loyalty,
                amount,
            )));
        }
    }
    if word == "0"
        && raw_line.is_some_and(|line| {
            let mut parts = line.trim().splitn(2, ':');
            let Some(prefix) = parts.next() else {
                return false;
            };
            parts.next().is_some() && prefix.trim().replace('−', "-") == "0"
        })
    {
        return Some(TotalCost::free());
    }
    None
}

pub(crate) fn loyalty_additional_restrictions(is_loyalty_shorthand: bool) -> Vec<String> {
    if !is_loyalty_shorthand {
        return Vec::new();
    }
    vec!["Activate only once each turn.".to_string()]
}

pub(crate) fn first_sacrifice_cost_choice_tag(
    mana_cost: &crate::cost::TotalCost,
) -> Option<TagKey> {
    super::util::find_first_sacrifice_cost_choice_tag(mana_cost)
}

pub(crate) fn last_exile_cost_choice_tag(mana_cost: &crate::cost::TotalCost) -> Option<TagKey> {
    super::util::find_last_exile_cost_choice_tag(mana_cost)
}

pub(crate) fn infer_activated_functional_zones(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[Vec<OwnedLexToken>],
) -> Vec<Zone> {
    let cost_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(cost_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let effect_words_match = |f: fn(&[&str]) -> bool| {
        effect_sentences.iter().any(|sentence| {
            let clause_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(sentence)
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
            f(&clause_words)
        })
    };
    if contains_source_from_your_graveyard_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_graveyard_phrase)
    {
        vec![Zone::Graveyard]
    } else if contains_source_from_your_hand_phrase(&cost_words)
        || contains_discard_source_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_hand_phrase)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

pub(crate) fn infer_activated_functional_zones_lexed(
    cost_tokens: &[OwnedLexToken],
    effect_sentences: &[&[OwnedLexToken]],
) -> Vec<Zone> {
    let cost_view = ActivationRestrictionCompatWords::new(cost_tokens);
    let cost_words: Vec<&str> = cost_view
        .to_word_refs()
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let effect_words_match = |f: fn(&[&str]) -> bool| {
        effect_sentences.iter().any(|sentence| {
            let sentence_view = ActivationRestrictionCompatWords::new(sentence);
            let clause_words: Vec<&str> = sentence_view
                .to_word_refs()
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
            f(&clause_words)
        })
    };
    if contains_source_from_your_graveyard_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_graveyard_phrase)
    {
        vec![Zone::Graveyard]
    } else if contains_source_from_your_hand_phrase(&cost_words)
        || contains_discard_source_phrase(&cost_words)
        || effect_words_match(contains_source_from_your_hand_phrase)
    {
        vec![Zone::Hand]
    } else {
        vec![Zone::Battlefield]
    }
}

pub(crate) fn parse_activate_only_timing(tokens: &[OwnedLexToken]) -> Option<ActivationTiming> {
    activated_sentence_parsers::parse_activate_only_timing_lexed(tokens)
}

pub(crate) fn parse_activate_only_timing_lexed(
    tokens: &[OwnedLexToken],
) -> Option<ActivationTiming> {
    activated_sentence_parsers::parse_activate_only_timing_lexed(tokens)
}

pub(crate) fn normalize_activate_only_restriction(
    tokens: &[OwnedLexToken],
    timing: &ActivationTiming,
) -> Option<String> {
    activated_sentence_parsers::normalize_activate_only_restriction(tokens, timing)
}

pub(crate) fn contains_word_sequence(words: &[&str], sequence: &[&str]) -> bool {
    find_word_sequence_start(words, sequence).is_some()
}

pub(crate) fn find_word_sequence_start(words: &[&str], sequence: &[&str]) -> Option<usize> {
    if sequence.is_empty() {
        Some(0)
    } else {
        find_word_slice_phrase_start(words, sequence)
    }
}

fn contains_any_word_sequence(words: &[&str], sequences: &[&[&str]]) -> bool {
    sequences
        .iter()
        .any(|sequence| contains_word_sequence(words, sequence))
}

pub(crate) fn flatten_mana_activation_conditions(
    condition: &crate::ConditionExpr,
    out: &mut Vec<crate::ConditionExpr>,
) {
    match condition {
        crate::ConditionExpr::And(left, right) => {
            flatten_mana_activation_conditions(left, out);
            flatten_mana_activation_conditions(right, out);
        }
        _ => out.push(condition.clone()),
    }
}

pub(crate) fn rebuild_mana_activation_conditions(
    conditions: Vec<crate::ConditionExpr>,
) -> Option<crate::ConditionExpr> {
    let mut iter = conditions.into_iter();
    let first = iter.next()?;
    Some(iter.fold(first, |acc, next| {
        crate::ConditionExpr::And(Box::new(acc), Box::new(next))
    }))
}

pub(crate) fn combine_mana_activation_condition(
    base: Option<crate::ConditionExpr>,
    timing: ActivationTiming,
) -> Option<crate::ConditionExpr> {
    if timing == ActivationTiming::AnyTime {
        return base;
    }
    merge_mana_activation_conditions(base, crate::ConditionExpr::ActivationTiming(timing))
}

pub(crate) fn merge_mana_activation_conditions(
    base: Option<crate::ConditionExpr>,
    condition: crate::ConditionExpr,
) -> Option<crate::ConditionExpr> {
    let mut conditions: Vec<crate::ConditionExpr> = Vec::new();
    if let Some(base) = base {
        flatten_mana_activation_conditions(&base, &mut conditions);
    }
    if !conditions.iter().any(|existing| *existing == condition) {
        conditions.push(condition);
    }
    rebuild_mana_activation_conditions(conditions)
}

pub(crate) fn is_activate_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_activate_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_activate_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_spend_mana_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_spend_mana_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_spend_mana_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_usage_restriction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_usage_restriction_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_usage_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_mana_spend_bonus_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ability::ManaUsageRestriction> {
    activated_sentence_parsers::parse_mana_spend_bonus_sentence_lexed(tokens)
}

pub(crate) fn is_any_player_may_activate_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_any_player_may_activate_sentence_lexed(tokens)
}

pub(crate) fn is_any_player_may_activate_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_any_player_may_activate_sentence_lexed(tokens)
}

pub(crate) fn is_trigger_only_restriction_sentence(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn is_trigger_only_restriction_sentence_lexed(tokens: &[OwnedLexToken]) -> bool {
    activated_sentence_parsers::is_trigger_only_restriction_sentence_lexed(tokens)
}

pub(crate) fn parse_triggered_times_each_turn_sentence(
    sentences: &[Vec<OwnedLexToken>],
) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_sentence(sentences)
}

pub(crate) fn parse_triggered_times_each_turn_from_words(words: &[&str]) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_from_words(words)
}

pub(crate) fn parse_triggered_times_each_turn_lexed(tokens: &[OwnedLexToken]) -> Option<u32> {
    activated_sentence_parsers::parse_triggered_times_each_turn_lexed(tokens)
}

pub(crate) fn parse_named_number(word: &str) -> Option<u32> {
    parse_number_word_u32(word)
}

pub(crate) fn parse_cycling_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_cycling_line_lexed(tokens)
}

pub(crate) fn parse_cycling_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let Some(cycling_idx) = find_cycling_keyword_word_index(&words) else {
        return Ok(None);
    };
    // Static grant clauses like "Each Sliver card in each player's hand has slivercycling {3}."
    // must be handled by parse_filter_has_granted_ability_line, not parsed as a standalone
    // cycling keyword ability on this card.
    if contains_granted_keyword_before_word(&words, cycling_idx) {
        return Ok(None);
    }

    let clause_text = joined_activation_clause_text(tokens);
    let cycling_groups = parse_cycling_keyword_cost_groups(tokens);
    let Some((first_keyword_tokens, first_cost_tokens)) = cycling_groups.first() else {
        return Ok(None);
    };
    if first_cost_tokens.is_empty() {
        return Ok(None);
    }

    let base_cost_words = crate::cards::builders::parser::token_word_refs(first_cost_tokens);
    if cycling_groups.iter().skip(1).any(|(_, cost_tokens)| {
        crate::cards::builders::parser::token_word_refs(cost_tokens) != base_cost_words
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mixed cycling costs (clause: '{clause_text}')",
        )));
    }

    let base_cost = parse_activation_cost(first_cost_tokens)?;
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    merged_costs.push(
        crate::costs::Cost::try_from_runtime_effect(Effect::emit_keyword_action(
            crate::events::KeywordActionKind::Cycle,
            1,
        ))
        .map_err(CardTextError::ParseError)?,
    );
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut search_filter = parse_cycling_search_filter(first_keyword_tokens)?;
    for (keyword_tokens, _) in cycling_groups.iter().skip(1) {
        let next_filter = parse_cycling_search_filter(keyword_tokens)?;
        match (&mut search_filter, next_filter) {
            (Some(current), Some(next)) => merge_cycling_search_filters(current, &next),
            (None, None) => {}
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported mixed cycling variants (clause: '{clause_text}')",
                )));
            }
        }
    }
    let effect = if let Some(filter) = search_filter {
        Effect::search_library_to_hand(filter, true)
    } else {
        Effect::draw(1)
    };

    let cost_text = base_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| base_cost_words.join(" "));
    let render_text = if let Some(group) = parse_cycling_keyword_group_text(tokens) {
        group
    } else if crate::cards::builders::parser::token_word_refs(first_keyword_tokens).is_empty() {
        cost_text
    } else {
        format!(
            "{} {cost_text}",
            crate::cards::builders::parser::token_word_refs(first_keyword_tokens).join(" ")
        )
    };

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![effect]),
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(render_text),
        },
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_channel_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_channel_line_lexed(tokens)
}

pub(crate) fn parse_channel_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    if words.first() != Some("channel") {
        return Ok(None);
    }

    let clause_text = joined_activation_clause_text(tokens);
    parse_hand_keyword_activated_body_lexed(&tokens[1..], "channel", "Channel", &clause_text)
}

pub(crate) fn parse_cycling_keyword_cost_groups(
    tokens: &[OwnedLexToken],
) -> Vec<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let mut groups = Vec::new();
    let mut idx = 0usize;

    while idx < tokens.len() {
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_comma() || token.is_semicolon())
        {
            idx += 1;
            continue;
        }

        let keyword_start = idx;
        let mut keyword_end: Option<usize> = None;
        while idx < tokens.len() {
            let Some(word) = tokens[idx].as_word().map(|word| word.to_ascii_lowercase()) else {
                break;
            };
            if str_strip_suffix(word.as_str(), "cycling").is_some() {
                keyword_end = Some(idx);
                idx += 1;
                break;
            }
            idx += 1;
        }
        let Some(keyword_end) = keyword_end else {
            break;
        };

        let cost_start = idx;
        if tokens.get(idx).is_some_and(|token| token.is_word("pay")) {
            // Handle life-cycling style costs like "Cycling—Pay 2 life."
            while idx < tokens.len() {
                let Some(word) = tokens[idx].as_word().map(|word| word.to_ascii_lowercase()) else {
                    break;
                };
                idx += 1;
                if word == "life" {
                    break;
                }
            }
        } else {
            while idx < tokens.len() {
                let lower_word = tokens[idx].as_word().map(|word| word.to_ascii_lowercase());
                // Reminder text often starts with "{N}, discard this card" and would
                // otherwise be consumed as part of the cycling cost.
                let looks_like_reminder_cost = idx > cost_start
                    && lower_word
                        .as_deref()
                        .is_some_and(|word| word.chars().all(|ch| ch.is_ascii_digit()))
                    && tokens.get(idx + 1).is_some_and(|token| token.is_comma())
                    && tokens
                        .get(idx + 2)
                        .and_then(OwnedLexToken::as_word)
                        .is_some_and(|next| next.eq_ignore_ascii_case("discard"));
                let is_cost_token = mana_pips_from_token(&tokens[idx]).is_some()
                    || lower_word.as_deref().is_some_and(is_cycling_cost_word);
                if looks_like_reminder_cost || !is_cost_token {
                    break;
                }
                idx += 1;
            }
        }
        if idx == cost_start {
            break;
        }

        groups.push((
            tokens[keyword_start..=keyword_end].to_vec(),
            tokens[cost_start..idx].to_vec(),
        ));

        if tokens.get(idx).is_some_and(|token| token.is_comma()) {
            idx += 1;
            continue;
        }
        break;
    }

    groups
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if items.iter().any(|existing| existing == &item) {
        return;
    }
    items.push(item);
}

pub(crate) fn merge_cycling_search_filters(base: &mut ObjectFilter, extra: &ObjectFilter) {
    for supertype in &extra.supertypes {
        push_unique(&mut base.supertypes, *supertype);
    }
    for card_type in &extra.card_types {
        push_unique(&mut base.card_types, *card_type);
    }
    for subtype in &extra.subtypes {
        push_unique(&mut base.subtypes, *subtype);
    }
    if let Some(colors) = extra.colors {
        base.colors = Some(
            base.colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
}

pub(crate) fn parse_cycling_keyword_group_text(tokens: &[OwnedLexToken]) -> Option<String> {
    let groups = parse_cycling_keyword_cost_groups(tokens);
    if groups.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for (keyword_tokens, cost_tokens) in groups {
        let keyword = crate::cards::builders::parser::token_word_refs(&keyword_tokens).join(" ");
        if keyword.is_empty() {
            continue;
        }
        let cost_words = crate::cards::builders::parser::token_word_refs(&cost_tokens);
        let cost = if cost_words.len() >= 3 && cost_words[0] == "pay" && cost_words[2] == "life" {
            format!("pay {} life", cost_words[1])
        } else {
            parse_activation_cost(&cost_tokens)
                .ok()
                .and_then(|total_cost| total_cost.mana_cost().map(|cost| cost.to_oracle()))
                .unwrap_or_else(|| cost_words.join(" "))
        };
        parts.push(format!("{keyword} {cost}"));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

pub(crate) fn is_cycling_cost_word(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(
                    ch,
                    '{' | '}' | '/' | 'w' | 'u' | 'b' | 'r' | 'g' | 'c' | 'x'
                )
        })
}

pub(crate) fn parse_cycling_search_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return Ok(None);
    }

    let keyword = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("missing cycling keyword".to_string()))?;
    let mut filter = ObjectFilter::default();

    for word in &words[..words.len().saturating_sub(1)] {
        if let Some(supertype) = parse_supertype_word(word) {
            push_unique(&mut filter.supertypes, supertype);
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut filter.card_types, card_type);
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique(&mut filter.subtypes, subtype);
            if is_land_subtype(subtype) {
                push_unique(&mut filter.card_types, CardType::Land);
            }
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
    }

    if keyword == "cycling" {
        return Ok(None);
    }

    if keyword == "landcycling" {
        push_unique(&mut filter.card_types, CardType::Land);
        return Ok(Some(filter));
    }

    if let Some(root) = str_strip_suffix(keyword, "cycling") {
        if let Some(card_type) = parse_card_type(root) {
            push_unique(&mut filter.card_types, card_type);
        } else if let Some(subtype) = parse_subtype_flexible(root) {
            push_unique(&mut filter.subtypes, subtype);
            if is_land_subtype(subtype) {
                push_unique(&mut filter.card_types, CardType::Land);
            }
        } else if let Some(color) = parse_color(root) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported cycling variant (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(Some(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported cycling variant (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn is_land_subtype(subtype: Subtype) -> bool {
    matches!(
        subtype,
        Subtype::Plains
            | Subtype::Island
            | Subtype::Swamp
            | Subtype::Mountain
            | Subtype::Forest
            | Subtype::Desert
    )
}

pub(crate) fn parse_equip_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let tokens = grammar::split_lexed_slices_on_period(tokens)
        .into_iter()
        .next()
        .unwrap_or(tokens);
    let clause_word_view = ActivationRestrictionCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if clause_words.first().copied() != Some("equip") {
        return Ok(None);
    }

    let (mana_pips, saw_zero, saw_non_symbol) = tokens.iter().skip(1).fold(
        (Vec::new(), false, false),
        |(mut pips, mut saw_zero, mut saw_non_symbol), token| {
            if let Some(group) = mana_pips_from_token(token) {
                if group.len() == 1 && matches!(group[0], ManaSymbol::Generic(0)) {
                    saw_zero = true;
                } else {
                    pips.push(group);
                }
            } else {
                saw_non_symbol = true;
            }
            (pips, saw_zero, saw_non_symbol)
        },
    );

    if saw_non_symbol {
        let looks_like_cost_prefix = clause_words.get(1).is_some_and(|word| {
            parse_mana_symbol(word).is_ok()
                || matches!(
                    *word,
                    "tap"
                        | "t"
                        | "pay"
                        | "discard"
                        | "sacrifice"
                        | "exile"
                        | "return"
                        | "remove"
                        | "behold"
                )
        });
        if !looks_like_cost_prefix {
            return Ok(None);
        }
        let cost_tokens = trim_commas(&tokens[1..]);
        if cost_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "equip missing activation cost".to_string(),
            ));
        }
        let total_cost = parse_activation_cost(&cost_tokens)?;
        let tail_words = crate::cards::builders::parser::token_word_refs(&cost_tokens);
        if tail_words.is_empty() {
            return Err(CardTextError::ParseError(
                "equip missing activation cost".to_string(),
            ));
        }
        let equip_text = format!("Equip—{}", keyword_title(&tail_words.join(" ")));
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

        return Ok(Some(ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: total_cost,
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::attach_to(target.clone()),
                    ]),
                    choices: vec![target.clone()],
                    timing: ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(equip_text),
            },
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        }));
    }

    if mana_pips.is_empty() && !saw_zero {
        return Err(CardTextError::ParseError(
            "equip missing mana cost".to_string(),
        ));
    }

    let mana_cost = if mana_pips.is_empty() {
        ManaCost::new()
    } else {
        ManaCost::from_pips(mana_pips)
    };
    let total_cost = if mana_cost.pips().is_empty() {
        TotalCost::free()
    } else {
        TotalCost::mana(mana_cost)
    };
    let equip_text = if saw_zero && total_cost.costs().is_empty() {
        "Equip {0}".to_string()
    } else if let Some(mana) = total_cost.mana_cost() {
        format!("Equip {}", mana.to_oracle())
    } else {
        "Equip".to_string()
    };
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::attach_to(target.clone()),
                ]),
                choices: vec![target.clone()],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(equip_text),
        },
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_equip_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_equip_line(tokens)
}

pub(crate) fn parse_activation_cost(tokens: &[OwnedLexToken]) -> Result<TotalCost, CardTextError> {
    let cst = parse_activation_cost_tokens_rewrite(tokens)?;
    lower_activation_cost_cst(&cst)
}

pub(crate) fn parse_devotion_value_from_add_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    let Some(devotion_idx) = find_index(&words, |word| *word == "devotion") else {
        return Ok(None);
    };

    let player = parse_devotion_player_from_words(&words, devotion_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion player in clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let to_idx = find_index(&words[devotion_idx + 1..], |word| *word == "to")
        .map(|idx| devotion_idx + 1 + idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color after devotion clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    if words.get(to_idx + 1).copied() == Some("that")
        && words.get(to_idx + 2).copied() == Some("color")
    {
        return Ok(Some(Value::DevotionToChosenColor(player)));
    }
    let color_word = words.get(to_idx + 1).copied().ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing devotion color (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let color_set = parse_color(color_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;
    let color = color_from_color_set(color_set).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "ambiguous devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;

    Ok(Some(Value::Devotion { player, color }))
}

pub(crate) fn parse_devotion_player_from_words(
    words: &[&str],
    devotion_idx: usize,
) -> Option<PlayerFilter> {
    if devotion_idx == 0 {
        return None;
    }
    let left = &words[..devotion_idx];
    if matches!(left, [.., "your"]) {
        return Some(PlayerFilter::You);
    }
    if matches!(left, [.., "opponent"] | [.., "opponents"]) {
        return Some(PlayerFilter::Opponent);
    }
    if matches!(left, [.., "that", "players"] | [.., "that", "player"]) {
        return Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
    }
    None
}

pub(crate) fn color_from_color_set(colors: ColorSet) -> Option<crate::color::Color> {
    let mut found = None;
    for color in [
        crate::color::Color::White,
        crate::color::Color::Blue,
        crate::color::Color::Black,
        crate::color::Color::Red,
        crate::color::Color::Green,
    ] {
        if colors.intersection(ColorSet::from_color(color)).count() > 0 {
            if found.is_some() {
                return None;
            }
            found = Some(color);
        }
    }
    found
}

pub(crate) fn parse_activation_condition(tokens: &[OwnedLexToken]) -> Option<crate::ConditionExpr> {
    activated_sentence_parsers::parse_activation_condition_lexed(tokens)
}

pub(crate) fn parse_activation_condition_lexed(
    tokens: &[OwnedLexToken],
) -> Option<crate::ConditionExpr> {
    activated_sentence_parsers::parse_activation_condition_lexed(tokens)
}

pub(crate) fn parse_cardinal_u32(word: &str) -> Option<u32> {
    let token = OwnedLexToken::word(word.to_string(), TextSpan::synthetic());
    parse_number(&[token]).map(|(value, _)| value)
}

pub(crate) fn parse_activation_count_per_turn(words: &[&str]) -> Option<u32> {
    activated_sentence_parsers::parse_activation_count_per_turn(words)
}

pub(crate) fn parse_enters_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped =
            slice_contains(&clause_words, &"enters") && slice_contains(&clause_words, &"tapped");
        if has_enters_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if clause_words.first().copied() == Some("this")
        && slice_contains(&clause_words, &"enters")
        && slice_contains(&clause_words, &"tapped")
    {
        let tapped_word_idx =
            find_index(&clause_words, |word| *word == "tapped").ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let tapped_token_idx =
            token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let trailing_words =
            crate::cards::builders::parser::token_word_refs(&tokens[tapped_token_idx + 1..]);
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(StaticAbility::enters_tapped_ability()));
    }
    Ok(None)
}

pub(crate) fn parse_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = crate::cards::builders::parser::token_word_refs(tokens);
    let has_commander_cast_count_clause =
        contains_word_sequence(&line_words, &["for", "each", "time"])
            && slice_contains(&line_words, &"cast")
            && slice_contains(&line_words, &"commander")
            && contains_word_sequence(&line_words, &["from", "the", "command", "zone"]);
    if has_commander_cast_count_clause {
        return Err(CardTextError::ParseError(format!(
            "unsupported commander-cast-count static clause (clause: '{}')",
            line_words.join(" ")
        )));
    }
    if slice_starts_with(&line_words, &["this", "cost", "is", "reduced", "by"])
        && line_words.len() > 6
    {
        let amount_tokens = trim_commas(&tokens[5..]);
        let parsed_amount = parse_cost_modifier_amount(&amount_tokens);
        let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
        let amount_fixed = if let Value::Fixed(value) = amount_value {
            value
        } else {
            1
        };
        let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
        let remaining_words = crate::cards::builders::parser::token_word_refs(remaining_tokens);
        if slice_contains(&remaining_words, &"for")
            && slice_contains(&remaining_words, &"each")
            && let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)?
        {
            let reduction = scale_dynamic_cost_modifier_value(dynamic, amount_fixed);
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    reduction,
                    crate::static_abilities::ThisSpellCostCondition::Always,
                ),
            )));
        }

        let amount_word = line_words[5];
        let amount_text = if amount_word.chars().all(|ch| ch.is_ascii_digit()) {
            format!("{{{amount_word}}}")
        } else {
            amount_word.to_string()
        };
        let tail = line_words[6..].join(" ");
        let text = format!("This cost is reduced by {amount_text} {tail}");
        return Err(CardTextError::ParseError(format!(
            "unsupported cost-reduction static clause (clause: '{}')",
            text
        )));
    }

    if slice_starts_with(&line_words, &["activated", "abilities", "of"]) {
        let Some(cost_idx) = find_index(&line_words, |word| *word == "cost" || *word == "costs")
        else {
            return Ok(None);
        };
        if cost_idx <= 3 {
            return Ok(None);
        }
        let subject_tokens = trim_commas(&tokens[3..cost_idx]);
        if subject_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported activated-ability cost reduction subject (clause: '{}')",
                line_words.join(" ")
            ))
        })?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }

        let amount_tokens = trim_commas(&tokens[cost_idx + 1..]);
        let Some((amount_value, used)) = parse_cost_modifier_amount(&amount_tokens) else {
            return Ok(None);
        };
        let reduction = match amount_value {
            Value::Fixed(value) if value > 0 => value as u32,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction amount (clause: '{}')",
                    line_words.join(" ")
                )));
            }
        };
        let tail_words = crate::cards::builders::parser::token_word_refs(&amount_tokens[used..]);
        if !slice_starts_with(&tail_words, &["less", "to", "activate"]) {
            return Ok(None);
        }

        return Ok(Some(StaticAbility::reduce_activated_ability_costs(
            filter,
            reduction,
            Some(1),
        )));
    }

    if slice_starts_with(&line_words, &["this", "ability", "costs"]) {
        let amount_tokens = trim_commas(&tokens[3..]);
        let Some((amount_value, used)) = parse_cost_modifier_amount(&amount_tokens) else {
            return Ok(None);
        };
        let reduction = match amount_value {
            Value::Fixed(value) if value > 0 => value as u32,
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction amount (clause: '{}')",
                    line_words.join(" ")
                )));
            }
        };
        let tail_tokens = trim_commas(&amount_tokens[used..]);
        let tail_words = crate::cards::builders::parser::token_word_refs(&tail_tokens);
        if tail_words == ["less", "to", "activate"] {
            return Ok(Some(StaticAbility::reduce_activated_ability_costs(
                ObjectFilter::source(),
                reduction,
                Some(1),
            )));
        }
        if slice_starts_with(&tail_words, &["less", "to", "activate", "if"]) {
            let condition_tokens = trim_commas(&tail_tokens[4..]);
            let condition_words =
                crate::cards::builders::parser::token_word_refs(&condition_tokens);
            if condition_words.first().copied() == Some("it")
                && condition_words.get(1).copied() == Some("targets")
            {
                let (count, used) = parse_number(&condition_tokens[2..]).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported activated-ability target condition count (clause: '{}')",
                        line_words.join(" ")
                    ))
                })?;
                let mut filter = parse_object_filter(&condition_tokens[2 + used..], false)
                    .map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported activated-ability target condition filter (clause: '{}')",
                            line_words.join(" ")
                        ))
                    })?;
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                return Ok(Some(
                    StaticAbility::reduce_activated_ability_costs_if_targets(
                        ObjectFilter::source(),
                        reduction,
                        crate::static_abilities::ActivatedAbilityCostCondition::TargetsExactly {
                            count: count as usize,
                            filter,
                        },
                        Some(1),
                    ),
                ));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activated-ability cost reduction condition (clause: '{}')",
                line_words.join(" ")
            )));
        }
        if slice_starts_with(&tail_words, &["less", "to", "activate", "for", "each"]) {
            let mut per_filter = parse_object_filter(&tail_tokens[5..], false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported activated-ability cost reduction tail (clause: '{}')",
                    line_words.join(" ")
                ))
            })?;
            if per_filter.zone.is_none() {
                per_filter.zone = Some(Zone::Battlefield);
            }
            return Ok(Some(
                StaticAbility::reduce_activated_ability_costs_for_each(
                    ObjectFilter::source(),
                    reduction,
                    per_filter,
                    Some(1),
                ),
            ));
        }
    }

    if !slice_starts_with(&line_words, &["this", "spell", "costs"]) {
        return Ok(None);
    }

    let costs_idx = find_index(tokens, |token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
    let amount_fixed = if let Value::Fixed(value) = amount_value {
        value
    } else {
        1
    };

    let remaining_tokens = &tokens[costs_idx + 1 + used..];
    let remaining_words: Vec<&str> =
        crate::cards::builders::parser::token_word_refs(remaining_tokens);

    if !slice_contains(&remaining_words, &"less") {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        let reduction =
            crate::static_abilities::CostReduction::new(ObjectFilter::default(), dynamic);
        return Ok(Some(StaticAbility::new(reduction)));
    }

    if parsed_amount.is_none() {
        return Ok(None);
    }

    let has_each = slice_contains(&remaining_words, &"each");
    let has_card_type = contains_word_sequence(&remaining_words, &["card", "type"]);
    let has_graveyard = slice_contains(&remaining_words, &"graveyard");

    if has_each && has_card_type && has_graveyard {
        if amount_fixed != 1 {
            return Ok(None);
        }
        let reduction = crate::effect::Value::CardTypesInGraveyard(PlayerFilter::You);
        let cost_reduction =
            crate::static_abilities::CostReduction::new(ObjectFilter::default(), reduction);
        return Ok(Some(StaticAbility::new(cost_reduction)));
    }

    Ok(None)
}

pub(crate) fn scale_dynamic_cost_modifier_value(dynamic: Value, multiplier: i32) -> Value {
    if multiplier <= 0 {
        return Value::Fixed(0);
    }
    if multiplier == 1 {
        return dynamic;
    }
    match dynamic {
        Value::Count(filter) => Value::CountScaled(filter, multiplier),
        Value::CountScaled(filter, factor) => Value::CountScaled(filter, factor * multiplier),
        other => {
            let mut scaled = other.clone();
            for _ in 1..multiplier {
                scaled = Value::Add(Box::new(scaled), Box::new(other.clone()));
            }
            scaled
        }
    }
}

pub(crate) fn parse_all_creatures_able_to_block_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if matches!(
        words.as_slice(),
        [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "creature",
            "do",
            "so"
        ] | [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "do",
            "so"
        ]
    ) {
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter: ObjectFilter::creature(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_block())),
            condition: None,
        }));
    }
    Ok(None)
}

pub(crate) fn parse_source_must_be_blocked_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    if matches!(
        words.as_slice(),
        ["this", "creature", "must", "be", "blocked", "if", "able"]
            | ["this", "must", "be", "blocked", "if", "able"]
    ) {
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::must_block_specific_attacker(
                ObjectFilter::creature(),
                ObjectFilter::source(),
            ),
            "this creature must be blocked if able".to_string(),
        )));
    }
    Ok(None)
}

pub(crate) fn parse_cant_clauses(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder != tokens
    {
        let Some(abilities) = parse_cant_clauses(&remainder)? else {
            return Ok(None);
        };
        let conditioned = abilities
            .into_iter()
            .map(|ability| ability.with_condition(condition.clone()).unwrap_or(ability))
            .collect::<Vec<_>>();
        return Ok(Some(conditioned));
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let is_direct_temporary_cast_restriction =
        contains_word_sequence(&normalized_words, &["this", "turn"])
            && !slice_contains(&normalized_words, &"unless")
            && !slice_contains(&normalized_words, &"who")
            && (slice_starts_with(&normalized_words, &["your", "opponents", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["each", "opponent", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["each", "player", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["players", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["target", "player", "cant", "cast"])
                || slice_starts_with(&normalized_words, &["you", "cant", "cast"]));
    if is_direct_temporary_cast_restriction {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and"))
        && let Some((neg_start, _)) = find_negation_span(tokens)
        && tokens[..neg_start]
            .iter()
            .any(|token| token.is_word("get") || token.is_word("gets"))
    {
        return Ok(None);
    }

    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if let Some(segments) = split_cant_clause_on_or(tokens) {
        let mut abilities = Vec::new();
        for segment in segments {
            let Some(ability) = parse_cant_clause(&segment)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(&segment).join(" ")
                )));
            };
            abilities.push(ability);
        }
        if !abilities.is_empty() {
            return Ok(Some(abilities));
        }
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = grammar::split_lexed_slices_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut abilities = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.to_vec();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().cloned());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(ability) = parse_cant_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(segment).join(" ")
                )));
            };
            abilities.push(ability);
        }

        if abilities.is_empty() {
            return Ok(None);
        }
        return Ok(Some(abilities));
    }

    parse_cant_clause(tokens).map(|ability| ability.map(|ability| vec![ability]))
}

fn split_cant_clause_on_or(tokens: &[OwnedLexToken]) -> Option<Vec<Vec<OwnedLexToken>>> {
    let (neg_start, neg_end) = find_negation_span(tokens)?;
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if slice_starts_with(&remainder_words, &["attack", "or", "block"]) {
        return None;
    }
    let or_idx = find_index(&remainder_tokens, |token: &OwnedLexToken| {
        token.is_word("or")
    })?;
    let tail = trim_commas(&remainder_tokens[or_idx + 1..]);
    let starts_new_restriction = tail.first().is_some_and(|token| {
        token.is_word("cast")
            || token.is_word("activate")
            || token.is_word("attack")
            || token.is_word("block")
            || token.is_word("be")
    });
    if !starts_new_restriction {
        return None;
    }

    let negation_tokens = tokens[neg_start..neg_end].to_vec();
    let mut first = subject_tokens.clone();
    first.extend(negation_tokens.iter().cloned());
    first.extend(trim_commas(&remainder_tokens[..or_idx]).iter().cloned());

    let mut second = subject_tokens.clone();
    second.extend(negation_tokens.iter().cloned());
    second.extend(tail.iter().cloned());

    Some(vec![first, second])
}

pub(crate) fn parse_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some((condition, remainder)) = strip_static_restriction_condition(tokens)?
        && remainder != tokens
    {
        let Some(ability) = parse_cant_clause(&remainder)? else {
            return Ok(None);
        };
        if let Some(conditioned) = ability.with_condition(condition.clone()) {
            return Ok(Some(conditioned));
        }
        if let Some(parsed) = parse_cant_restriction_clause(&remainder)?
            && parsed.target.is_none()
        {
            return Ok(Some(
                StaticAbility::restriction(
                    parsed.restriction,
                    format_negated_restriction_display(tokens),
                )
                .with_condition(condition)
                .unwrap_or(ability),
            ));
        }
        return Ok(Some(ability));
    }

    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if let Some(rest) = slice_strip_prefix(
        &normalized,
        &[
            "creatures",
            "cant",
            "attack",
            "you",
            "unless",
            "their",
            "controller",
            "pays",
        ],
    ) && rest.get(1..)
        == Some(&[
            "for",
            "each",
            "creature",
            "they",
            "control",
            "thats",
            "attacking",
            "you",
        ])
    {
        if let Ok(amount) = rest[0].parse::<u32>() {
            return Ok(Some(
                StaticAbility::cant_attack_you_unless_controller_pays_per_attacker(amount),
            ));
        }
    }

    let is_collective_restraint_domain_attack_tax = slice_starts_with(
        &normalized,
        &[
            "creatures",
            "cant",
            "attack",
            "you",
            "unless",
            "their",
            "controller",
            "pays",
            "x",
            "for",
            "each",
            "creature",
            "they",
            "control",
            "thats",
            "attacking",
            "you",
        ],
    ) && (slice_ends_with(
        &normalized,
        &[
            "where", "x", "is", "the", "number", "of", "basic", "land", "types", "among", "lands",
            "you", "control",
        ],
    ) || slice_ends_with(
        &normalized,
        &[
            "where", "x", "is", "the", "number", "of", "basic", "land", "type", "among", "lands",
            "you", "control",
        ],
    ));
    if is_collective_restraint_domain_attack_tax {
        return Ok(Some(
            StaticAbility::cant_attack_you_unless_controller_pays_per_attacker_basic_land_types_among_lands_you_control(),
        ));
    }

    let starts_with_cant_be_blocked_by =
        slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "by"],
        ) || slice_starts_with(&normalized, &["this", "cant", "be", "blocked", "by"])
            || slice_starts_with(&normalized, &["cant", "be", "blocked", "by"]);
    if starts_with_cant_be_blocked_by {
        let mut idx = if slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "by"],
        ) {
            6
        } else if slice_starts_with(&normalized, &["this", "cant", "be", "blocked", "by"]) {
            5
        } else {
            4
        };
        if normalized
            .get(idx)
            .is_some_and(|word| *word == "creature" || *word == "creatures")
        {
            idx += 1;
        }
        if normalized.get(idx) == Some(&"more") && normalized.get(idx + 1) == Some(&"than") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![OwnedLexToken::word(
                amount_word.to_string(),
                TextSpan::synthetic(),
            )];
            let (max_blockers, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1 {
                return Err(CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            let noun = normalized.get(idx + 3).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if noun != "creature" && noun != "creatures" {
                return Err(CardTextError::ParseError(format!(
                    "unsupported blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            if idx + 4 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked max-blockers clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            return Ok(Some(StaticAbility::cant_be_blocked_by_more_than(
                max_blockers as usize,
            )));
        }
        if normalized.get(idx) == Some(&"with") && normalized.get(idx + 1) == Some(&"power") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![OwnedLexToken::word(
                amount_word.to_string(),
                TextSpan::synthetic(),
            )];
            let (threshold, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1 || normalized.get(idx + 3) != Some(&"or") || idx + 5 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }

            return match normalized.get(idx + 4) {
                Some(&"less") => Ok(Some(StaticAbility::cant_be_blocked_by_power_or_less(
                    threshold as i32,
                ))),
                Some(&"greater") | Some(&"more") => Ok(Some(
                    StaticAbility::cant_be_blocked_by_power_or_greater(threshold as i32),
                )),
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                ))),
            };
        }

        if normalized.get(idx) == Some(&"with")
            && normalized.get(idx + 1) == Some(&"flying")
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature()
                        .with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by creatures with flying".to_string(),
            )));
        }
        if let Some(color_word) = normalized.get(idx).copied()
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked by {color_word} creatures"),
            )));
        }

        if normalized
            .get(idx)
            .is_some_and(|word| *word == "wall" || *word == "walls")
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().with_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked by walls".to_string(),
            )));
        }
    }

    let starts_with_cant_be_blocked_except_by =
        slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "except", "by"],
        ) || slice_starts_with(
            &normalized,
            &["this", "cant", "be", "blocked", "except", "by"],
        ) || slice_starts_with(&normalized, &["cant", "be", "blocked", "except", "by"]);
    if starts_with_cant_be_blocked_except_by {
        let idx = if slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "be", "blocked", "except", "by"],
        ) {
            7
        } else if slice_starts_with(
            &normalized,
            &["this", "cant", "be", "blocked", "except", "by"],
        ) {
            6
        } else {
            5
        };
        if let Some(color_word) = normalized.get(idx)
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
            && let Some(color) = parse_color(color_word)
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_colors(crate::color::ColorSet::from(color)),
                    ObjectFilter::source(),
                ),
                format!("this creature can't be blocked except by {color_word} creatures"),
            )));
        }
        if normalized.get(idx) == Some(&"artifact")
            && normalized
                .get(idx + 1)
                .is_some_and(|word| *word == "creature" || *word == "creatures")
            && idx + 2 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_type(CardType::Artifact),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by artifact creatures".to_string(),
            )));
        }
        if normalized
            .get(idx)
            .is_some_and(|word| *word == "wall" || *word == "walls")
            && idx + 1 == normalized.len()
        {
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::creature().without_subtype(Subtype::Wall),
                    ObjectFilter::source(),
                ),
                "this creature can't be blocked except by walls".to_string(),
            )));
        }
    }

    let starts_with_cant_attack_unless_defending_player = slice_starts_with(
        &normalized,
        &[
            "this",
            "creature",
            "cant",
            "attack",
            "unless",
            "defending",
            "player",
        ],
    ) || slice_starts_with(
        &normalized,
        &["this", "cant", "attack", "unless", "defending", "player"],
    );
    let cant_attack_unless_cast_creature_spell_tail = slice_ends_with(
        &normalized,
        &[
            "unless", "youve", "cast", "a", "creature", "spell", "this", "turn",
        ],
    ) || slice_ends_with(
        &normalized,
        &[
            "unless", "youve", "cast", "creature", "spell", "this", "turn",
        ],
    );
    let cant_attack_unless_cast_noncreature_spell_tail = slice_ends_with(
        &normalized,
        &[
            "unless",
            "youve",
            "cast",
            "a",
            "noncreature",
            "spell",
            "this",
            "turn",
        ],
    ) || slice_ends_with(
        &normalized,
        &[
            "unless",
            "youve",
            "cast",
            "noncreature",
            "spell",
            "this",
            "turn",
        ],
    );
    if cant_attack_unless_cast_creature_spell_tail
        && (slice_starts_with(&normalized, &["this", "creature", "cant", "attack"])
            || slice_starts_with(&normalized, &["this", "cant", "attack"]))
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_creature_spell_this_turn(),
        ));
    }
    if cant_attack_unless_cast_noncreature_spell_tail
        && (slice_starts_with(&normalized, &["this", "creature", "cant", "attack"])
            || slice_starts_with(&normalized, &["this", "cant", "attack"]))
    {
        return Ok(Some(
            StaticAbility::cant_attack_unless_controller_cast_noncreature_spell_this_turn(),
        ));
    }

    let starts_with_this_cant_attack_unless =
        slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "attack", "unless"],
        ) || slice_starts_with(&normalized, &["this", "cant", "attack", "unless"]);
    if starts_with_this_cant_attack_unless {
        let tail = if slice_starts_with(
            &normalized,
            &["this", "creature", "cant", "attack", "unless"],
        ) {
            &normalized[5..]
        } else {
            &normalized[4..]
        };

        let static_text = format!("Can't attack unless {}", tail.join(" "));
        let static_with = |condition| {
            Ok(Some(StaticAbility::cant_attack_unless_condition(
                condition,
                static_text.clone(),
            )))
        };

        if tail
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "defending",
                "player",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Creature),
                ),
            );
        }
        if tail
            == [
                "you",
                "control",
                "more",
                "lands",
                "than",
                "defending",
                "player",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerControlsMoreThanDefendingPlayer(
                    ObjectFilter::default().with_type(crate::types::CardType::Land),
                ),
            );
        }
        if let [
            "you",
            "control",
            "another",
            "creature",
            "with",
            "power",
            amount,
            "or",
            "greater",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature().you_control().other().with_power(
                            crate::filter::Comparison::GreaterThanOrEqual(value as i32),
                        ),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "another", "artifact"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::artifact().you_control().other(),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "an", "artifact"] || tail == ["you", "control", "artifact"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(ObjectFilter::artifact().you_control()),
                ),
            );
        }
        if tail == ["you", "control", "a", "knight", "or", "a", "soldier"]
            || tail == ["you", "control", "knight", "or", "soldier"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::Or(
                        Box::new(crate::ConditionExpr::YouControl(
                            ObjectFilter::creature()
                                .you_control()
                                .with_subtype(Subtype::Knight),
                        )),
                        Box::new(crate::ConditionExpr::YouControl(
                            ObjectFilter::creature()
                                .you_control()
                                .with_subtype(Subtype::Soldier),
                        )),
                    ),
                ),
            );
        }
        if let [
            "you",
            "control",
            "a",
            "creature",
            "with",
            "power",
            amount,
            "or",
            "greater",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature().you_control().with_power(
                            crate::filter::Comparison::GreaterThanOrEqual(value as i32),
                        ),
                    ),
                ),
            );
        }
        if tail == ["you", "control", "a", "1/1", "creature"]
            || tail == ["you", "control", "1/1", "creature"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::YouControl(
                        ObjectFilter::creature()
                            .you_control()
                            .with_power(crate::filter::Comparison::Equal(1))
                            .with_toughness(crate::filter::Comparison::Equal(1)),
                    ),
                ),
            );
        }
        if tail == ["there", "is", "a", "mountain", "on", "the", "battlefield"]
            || tail == ["there", "is", "a", "mountain", "on", "battlefield"]
            || tail == ["there", "is", "mountain", "on", "battlefield"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Mountain),
                    count: 1,
                },
            );
        }
        if let [
            "there",
            "are",
            amount,
            "or",
            "more",
            "cards",
            "in",
            "your",
            "graveyard",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::ControllerGraveyardHasCardsAtLeast(
                    value,
                ),
            );
        }
        if let [
            "there",
            "are",
            amount,
            "or",
            "more",
            "islands",
            "on",
            "the",
            "battlefield",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::BattlefieldCountAtLeast {
                    filter: ObjectFilter::default()
                        .with_type(crate::types::CardType::Land)
                        .with_subtype(Subtype::Island),
                    count: value,
                },
            );
        }
        if tail == ["defending", "player", "is", "poisoned"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsPoisoned,
                ),
            );
        }
        if let [
            "defending",
            "player",
            "has",
            amount,
            "or",
            "more",
            "cards",
            "in",
            "their",
            "graveyard",
        ] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::HasCardsInGraveyardOrMore(
                        value,
                    ),
                ),
            );
        }
        if tail
            == [
                "defending",
                "player",
                "controls",
                "an",
                "enchantment",
                "or",
                "an",
                "enchanted",
                "permanent",
            ]
            || tail
                == [
                    "defending",
                    "player",
                    "controls",
                    "enchantment",
                    "or",
                    "enchanted",
                    "permanent",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::ControlsEnchantmentOrEnchantedPermanent,
                ),
            );
        }
        if tail == ["defending", "player", "controls", "a", "snow", "land"]
            || tail == ["defending", "player", "controls", "snow", "land"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default()
                            .with_type(crate::types::CardType::Land)
                            .with_supertype(crate::types::Supertype::Snow),
                    ),
                ),
            );
        }
        if tail
            == [
                "defending",
                "player",
                "controls",
                "a",
                "creature",
                "with",
                "flying",
            ]
            || tail
                == [
                    "defending",
                    "player",
                    "controls",
                    "creature",
                    "with",
                    "flying",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default()
                            .with_type(crate::types::CardType::Creature)
                            .with_static_ability(crate::static_abilities::StaticAbilityId::Flying),
                    ),
                ),
            );
        }
        if tail == ["defending", "player", "controls", "a", "blue", "permanent"]
            || tail == ["defending", "player", "controls", "blue", "permanent"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                        ObjectFilter::default().with_colors(crate::color::ColorSet::from_color(
                            crate::color::Color::Blue,
                        )),
                    ),
                ),
            );
        }
        if tail == ["at", "least", "two", "other", "creatures", "attack"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::AtLeastNOtherCreaturesAttack(
                        2,
                    ),
                ),
            );
        }
        if tail
            == [
                "a", "creature", "with", "greater", "power", "also", "attacks",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::CreatureWithGreaterPowerAlsoAttacks,
                ),
            );
        }
        if tail == ["a", "black", "or", "green", "creature", "also", "attacks"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackingGroupCondition(
                    crate::static_abilities::AttackingGroupAttackCondition::BlackOrGreenCreatureAlsoAttacks,
                ),
            );
        }
        if tail
            == [
                "an", "opponent", "has", "been", "dealt", "damage", "this", "turn",
            ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::OpponentWasDealtDamageThisTurn,
            );
        }
        if let ["you", "control", amount, "or", "more", "artifacts"] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            let mut filter = ObjectFilter::artifact();
            filter.zone = Some(Zone::Battlefield);
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::SourceCondition(
                    crate::ConditionExpr::PlayerControlsAtLeast {
                        player: PlayerFilter::You,
                        filter,
                        count: value,
                    },
                ),
            );
        }
        if tail == ["you", "sacrifice", "a", "land"] || tail == ["you", "sacrifice", "land"] {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land(),
                        count: 1,
                    },
                ),
            );
        }
        if let ["you", "sacrifice", amount, "islands"] = tail
            && let Some(value) = parse_cardinal_u32(amount)
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::SacrificePermanents {
                        filter: ObjectFilter::land().with_subtype(Subtype::Island),
                        count: value,
                    },
                ),
            );
        }
        if tail
            == [
                "you",
                "return",
                "an",
                "enchantment",
                "you",
                "control",
                "to",
                "its",
                "owners",
                "hand",
            ]
            || tail
                == [
                    "you",
                    "return",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owners",
                    "hand",
                ]
            || tail
                == [
                    "you",
                    "return",
                    "an",
                    "enchantment",
                    "you",
                    "control",
                    "to",
                    "its",
                    "owner",
                    "s",
                    "hand",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::ReturnPermanentsToOwnersHand {
                        filter: ObjectFilter::enchantment(),
                        count: 1,
                    },
                ),
            );
        }
        if tail
            == [
                "you", "pay", "1", "for", "each", "+1/+1", "counter", "on", "it",
            ]
            || tail
                == [
                    "you", "pay", "1", "for", "each", "1/1", "counter", "on", "it",
                ]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::AttackCost(
                    crate::static_abilities::AttackCostCondition::PayGenericPerSourceCounter {
                        counter_type: crate::object::CounterType::PlusOnePlusOne,
                        amount_per_counter: 1,
                    },
                ),
            );
        }
        if tail == ["defending", "player", "is", "the", "monarch"]
            || tail == ["defending", "player", "is", "monarch"]
        {
            return static_with(
                crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                    crate::static_abilities::DefendingPlayerAttackCondition::IsMonarch,
                ),
            );
        }
    }

    if starts_with_cant_attack_unless_defending_player {
        let mut idx = if slice_starts_with(
            &normalized,
            &[
                "this",
                "creature",
                "cant",
                "attack",
                "unless",
                "defending",
                "player",
            ],
        ) {
            7
        } else {
            6
        };

        if !normalized
            .get(idx)
            .is_some_and(|word| *word == "control" || *word == "controls")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported cant-attack unless clause tail (clause: '{}')",
                normalized.join(" ")
            )));
        }
        idx += 1;

        if normalized
            .get(idx)
            .is_some_and(|word| *word == "a" || *word == "an" || *word == "the")
        {
            idx += 1;
        }

        let subtype_word = normalized.get(idx).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing land subtype in cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| str_strip_suffix(subtype_word, "s").and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported land subtype in cant-attack unless clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;

        if idx + 1 != normalized.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            )));
        }

        return Ok(Some(StaticAbility::cant_attack_unless_condition(
            crate::static_abilities::CantAttackUnlessConditionSpec::DefendingPlayerCondition(
                crate::static_abilities::DefendingPlayerAttackCondition::Controls(
                    ObjectFilter::land().with_subtype(subtype),
                ),
            ),
            "",
        )));
    }

    if let Some((neg_start, neg_end)) = find_negation_span(tokens) {
        let subject_tokens = trim_commas(&tokens[..neg_start]);
        let remainder_tokens = trim_commas(&tokens[neg_end..]);
        let remainder_words_storage = normalize_cant_words(&remainder_tokens);
        let remainder_words = remainder_words_storage
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let subject_words = crate::cards::builders::parser::token_word_refs(&subject_tokens);
        if (subject_words == ["this", "creature"] || subject_words == ["this"])
            && remainder_words.first() == Some(&"block")
            && remainder_words.len() > 1
        {
            let attacker_tokens = trim_commas(&remainder_tokens[1..]);
            let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported blocker restriction filter (clause: '{}')",
                        normalized.join(" ")
                    ))
                })?;
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::block_specific_attacker(
                    ObjectFilter::source(),
                    attacker_filter,
                ),
                format!(
                    "this creature can't block {}",
                    crate::cards::builders::parser::token_word_refs(&attacker_tokens).join(" ")
                ),
            )));
        }
        if remainder_words.as_slice() == ["transform"] {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Ok(None);
            };
            let subject_text =
                crate::cards::builders::parser::token_word_refs(&subject_tokens).join(" ");
            if subject_text.is_empty() {
                return Ok(None);
            }
            return Ok(Some(StaticAbility::restriction(
                crate::effect::Restriction::transform(filter),
                format!("{subject_text} can't transform"),
            )));
        }
    }

    if slice_starts_with(
        &normalized,
        &["your", "opponents", "cant", "cast", "spells", "with"],
    ) && normalized.len() >= 8
        && normalized[6] == "mana"
        && normalized[7] == "values"
    {
        let parity = match normalized[5] {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return Ok(None),
        };
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::cast_spells_matching(
                PlayerFilter::Opponent,
                ObjectFilter::spell().with_mana_value_parity(parity),
            ),
            format_negated_restriction_display(tokens),
        )));
    }

    if slice_starts_with(
        &normalized,
        &[
            "your",
            "opponents",
            "cant",
            "block",
            "with",
            "creatures",
            "with",
        ],
    ) && normalized.len() >= 10
        && normalized[8] == "mana"
        && normalized[9] == "values"
    {
        let parity = match normalized[7] {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return Ok(None),
        };
        return Ok(Some(StaticAbility::restriction(
            crate::effect::Restriction::block(
                ObjectFilter::creature()
                    .opponent_controls()
                    .with_mana_value_parity(parity),
            ),
            format_negated_restriction_display(tokens),
        )));
    }

    if slice_starts_with(
        &normalized,
        &["this", "cant", "attack", "or", "block", "unless"],
    ) && slice_ends_with(
        &normalized,
        &["even", "number", "of", "counters", "on", "it"],
    ) {
        return Ok(Some(StaticAbility::keyword_marker(
            format_negated_restriction_display(tokens),
        )));
    }

    if (slice_starts_with(
        &normalized,
        &[
            "this", "creature", "cant", "attack", "or", "block", "unless",
        ],
    ) || slice_starts_with(
        &normalized,
        &["this", "cant", "attack", "or", "block", "unless"],
    )) && let tail = if slice_starts_with(
        &normalized,
        &[
            "this", "creature", "cant", "attack", "or", "block", "unless",
        ],
    ) {
        &normalized[7..]
    } else {
        &normalized[6..]
    } && let ["you", "control", amount, "or", "more", rest @ ..] = tail
        && !rest.is_empty()
        && let Some(count) = parse_cardinal_u32(amount)
    {
        let filter_tokens = rest
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        if let Ok(mut filter) = parse_object_filter(&filter_tokens, false) {
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            let condition =
                crate::ConditionExpr::Not(Box::new(crate::ConditionExpr::PlayerControlsAtLeast {
                    player: PlayerFilter::You,
                    filter,
                    count,
                }));
            return Ok(Some(
                StaticAbility::restriction(
                    crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                    format_negated_restriction_display(tokens),
                )
                .with_condition(condition)
                .unwrap_or_else(|| {
                    StaticAbility::restriction(
                        crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                        format_negated_restriction_display(tokens),
                    )
                }),
            ));
        }
    }

    if slice_starts_with(&normalized, &["if", "source", "you", "control", "with"])
        && slice_contains(&normalized, &"mana")
        && slice_contains(&normalized, &"value")
        && slice_contains(&normalized, &"double")
        && normalized.last().is_some_and(|word| *word == "instead")
    {
        return Ok(Some(StaticAbility::keyword_marker(
            crate::cards::builders::parser::token_word_refs(tokens).join(" "),
        )));
    }

    if let Some(parsed) = parse_cant_restriction_clause(tokens)?
        && parsed.target.is_none()
        && matches!(
            parsed.restriction,
            crate::effect::Restriction::GainLife(_)
                | crate::effect::Restriction::SearchLibraries(_)
                | crate::effect::Restriction::CastSpellsMatching(_, _)
                | crate::effect::Restriction::ActivateNonManaAbilities(_)
                | crate::effect::Restriction::ActivateAbilitiesOf(_)
                | crate::effect::Restriction::ActivateTapAbilitiesOf(_)
                | crate::effect::Restriction::ActivateNonManaAbilitiesOf(_)
                | crate::effect::Restriction::CastMoreThanOneSpellEachTurn(_, _)
                | crate::effect::Restriction::DrawCards(_)
                | crate::effect::Restriction::DrawExtraCards(_)
                | crate::effect::Restriction::ChangeLifeTotal(_)
                | crate::effect::Restriction::LoseGame(_)
                | crate::effect::Restriction::WinGame(_)
                | crate::effect::Restriction::PreventDamage
        )
    {
        let ability = match normalized.as_slice() {
            ["players", "cant", "gain", "life"] => StaticAbility::players_cant_gain_life(),
            ["players", "cant", "search", "libraries"] => StaticAbility::players_cant_search(),
            ["damage", "cant", "be", "prevented"] => StaticAbility::damage_cant_be_prevented(),
            ["you", "cant", "lose", "the", "game"] => StaticAbility::you_cant_lose_game(),
            ["your", "opponents", "cant", "win", "the", "game"] => {
                StaticAbility::opponents_cant_win_game()
            }
            ["your", "life", "total", "cant", "change"] => {
                StaticAbility::your_life_total_cant_change()
            }
            ["your", "opponents", "cant", "cast", "spells"] => {
                StaticAbility::opponents_cant_cast_spells()
            }
            [
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => StaticAbility::opponents_cant_draw_extra_cards(),
            _ => StaticAbility::restriction(
                parsed.restriction,
                format_negated_restriction_display(tokens),
            ),
        };
        return Ok(Some(ability));
    }

    let ability = match normalized.as_slice() {
        ["counters", "cant", "be", "put", "on", "this", "permanent"] => {
            StaticAbility::cant_have_counters_placed()
        }
        ["this", "spell", "cant", "be", "countered"] => StaticAbility::cant_be_countered_ability(),
        ["this", "creature", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "attack", "its", "owner"] => {
            StaticAbility::cant_attack_its_owner()
        }
        ["this", "creature", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "creature", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this creature can't attack alone".to_string(),
        ),
        ["this", "token", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this token can't attack alone".to_string(),
        ),
        ["this", "cant", "attack", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_alone(ObjectFilter::source()),
            "this can't attack alone".to_string(),
        ),
        ["this", "token", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "token", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this creature can't attack or block".to_string(),
        ),
        ["this", "token", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this token can't attack or block".to_string(),
        ),
        ["this", "cant", "attack", "or", "block"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
            "this can't attack or block".to_string(),
        ),
        ["this", "creature", "cant", "attack", "or", "block", "alone"] => {
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
                "this creature can't attack or block alone".to_string(),
            )
        }
        ["this", "token", "cant", "attack", "or", "block", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
            "this token can't attack or block alone".to_string(),
        ),
        ["this", "cant", "attack", "or", "block", "alone"] => StaticAbility::restriction(
            crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
            "this can't attack or block alone".to_string(),
        ),
        ["permanents", "you", "control", "cant", "be", "sacrificed"] => {
            StaticAbility::permanents_you_control_cant_be_sacrificed()
        }
        ["this", "creature", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["this", "creature", "cant", "be", "blocked", "this", "turn"] => {
            StaticAbility::unblockable()
        }
        ["this", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["this", "cant", "be", "blocked", "this", "turn"] => StaticAbility::unblockable(),
        ["cant", "be", "blocked"] => StaticAbility::unblockable(),
        ["cant", "be", "blocked", "this", "turn"] => StaticAbility::unblockable(),
        _ => {
            if let Some(parsed) = parse_negated_object_restriction_clause(tokens)?
                && parsed.target.is_none()
            {
                return Ok(Some(StaticAbility::restriction(
                    parsed.restriction,
                    format_negated_restriction_display(tokens),
                )));
            }
            return Ok(None);
        }
    };

    Ok(Some(ability))
}

pub(crate) fn format_negated_restriction_display(tokens: &[OwnedLexToken]) -> String {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    let mut out = Vec::with_capacity(words.len());
    let mut idx = 0usize;
    while idx < words.len() {
        match (words[idx], words.get(idx + 1).copied()) {
            ("cant", _) => {
                out.push("can't".to_string());
                idx += 1;
            }
            ("can", Some("not")) => {
                out.push("can't".to_string());
                idx += 2;
            }
            ("does", Some("not")) => {
                out.push("doesn't".to_string());
                idx += 2;
            }
            ("do", Some("not")) => {
                out.push("don't".to_string());
                idx += 2;
            }
            ("non", Some("phyrexian")) => {
                out.push("non-phyrexian".to_string());
                idx += 2;
            }
            _ => {
                out.push(words[idx].to_string());
                idx += 1;
            }
        }
    }
    out.join(" ")
}

pub(crate) fn parse_cant_restrictions(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<ParsedCantRestriction>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = grammar::split_lexed_slices_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut restrictions = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            if find_negation_span(segment).is_none() {
                continue;
            }
            let mut expanded = segment.to_vec();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().cloned());
                expanded = with_subject;
            } else if idx > 0
                && !shared_subject.is_empty()
                && starts_with_possessive_activated_ability_subject(segment)
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.iter().skip(1).cloned());
                expanded = with_subject;
            }
            let Some(restriction) = parse_cant_restriction_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant restriction segment (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(segment).join(" ")
                )));
            };
            restrictions.push(restriction);
        }

        if restrictions.is_empty() {
            return Ok(None);
        }
        return Ok(Some(restrictions));
    }

    parse_cant_restriction_clause(tokens).map(|restriction| restriction.map(|r| vec![r]))
}

pub(crate) fn parse_cant_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    if let Some((_, remainder)) = parse_restriction_duration(tokens)?
        && !remainder.is_empty()
        && remainder.len() < tokens.len()
    {
        return parse_cant_restriction_clause(&remainder);
    }

    if let Some(parsed) = parse_player_negated_restriction_clause(tokens)? {
        return Ok(Some(parsed));
    }

    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    let restriction = if let Some(parsed) = parse_cant_cast_restriction_words(&normalized) {
        parsed
    } else {
        if let [
            "your",
            "opponents",
            "cant",
            "block",
            "with",
            "creatures",
            "with",
            parity,
            "mana",
            "values",
        ] = normalized.as_slice()
        {
            let parity = match *parity {
                "odd" => crate::filter::ParityRequirement::Odd,
                "even" => crate::filter::ParityRequirement::Even,
                _ => return parse_negated_object_restriction_clause(tokens),
            };
            return Ok(Some(ParsedCantRestriction {
                restriction: Restriction::block(
                    ObjectFilter::creature()
                        .opponent_controls()
                        .with_mana_value_parity(parity),
                ),
                target: None,
            }));
        }
        match normalized.as_slice() {
            ["players", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Any),
            ["players", "cant", "search", "libraries"] => {
                Restriction::search_libraries(PlayerFilter::Any)
            }
            ["players", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::Any),
            [
                "players",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Any),
            ["damage", "cant", "be", "prevented"] => Restriction::prevent_damage(),
            ["you", "cant", "lose", "the", "game"] => Restriction::lose_game(PlayerFilter::You),
            ["your", "opponents", "cant", "win", "the", "game"] => {
                Restriction::win_game(PlayerFilter::Opponent)
            }
            ["your", "life", "total", "cant", "change"] => {
                Restriction::change_life_total(PlayerFilter::You)
            }
            [
                "your",
                "opponents",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Opponent),
            [
                "each",
                "opponent",
                "cant",
                "draw",
                "more",
                "than",
                "one",
                "card",
                "each",
                "turn",
            ] => Restriction::draw_extra_cards(PlayerFilter::Opponent),
            ["you", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::You),
            ["you", "cant", "search", "libraries"] => {
                Restriction::search_libraries(PlayerFilter::You)
            }
            ["you", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::You),
            ["you", "cant", "become", "the", "monarch"]
            | ["you", "cant", "become", "monarch"]
            | ["you", "cant", "become", "the", "monarch", "this", "turn"]
            | ["you", "cant", "become", "monarch", "this", "turn"] => {
                Restriction::become_monarch(PlayerFilter::You)
            }
            ["they", "cant", "gain", "life"] | ["that", "player", "cant", "gain", "life"] => {
                Restriction::gain_life(PlayerFilter::IteratedPlayer)
            }
            ["opponents", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Opponent),
            _ => return parse_negated_object_restriction_clause(tokens),
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target: None,
    }))
}

fn parse_cant_cast_restriction_words(words: &[&str]) -> Option<crate::effect::Restriction> {
    use crate::effect::Restriction;

    if let Some((player, used)) = parse_cant_cast_subject(words) {
        let tail = &words[used..];

        if let Some(spell_filter) = parse_cast_additional_limit_filter(tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }

        if tail.first() != Some(&"cant") {
            return None;
        }
        let cant_tail = &tail[1..];

        if cant_tail == ["cast", "spells"] || cant_tail == ["cast", "spells", "this", "turn"] {
            return Some(Restriction::cast_spells(player));
        }
        if cant_tail.len() >= 6
            && cant_tail[0] == "cast"
            && cant_tail[1] == "spells"
            && cant_tail[2] == "with"
            && cant_tail[4] == "mana"
            && cant_tail[5] == "values"
        {
            let parity = cant_tail[3];
            let parity = match parity {
                "odd" => crate::filter::ParityRequirement::Odd,
                "even" => crate::filter::ParityRequirement::Even,
                _ => return None,
            };
            return Some(Restriction::cast_spells_matching(
                player,
                ObjectFilter::spell().with_mana_value_parity(parity),
            ));
        }
        if cant_tail == ["cast", "creature", "spells"]
            || cant_tail == ["cast", "creature", "spells", "this", "turn"]
        {
            return Some(Restriction::cast_creature_spells(player));
        }
        if cant_tail.first() == Some(&"cast") {
            let mut idx = 1usize;
            if let Some((spell_filter, used)) = parse_cast_limit_qualifier(&cant_tail[idx..]) {
                idx += used;
                if cant_tail.get(idx) == Some(&"spell") || cant_tail.get(idx) == Some(&"spells") {
                    idx += 1;
                    if cant_tail.get(idx) == Some(&"this")
                        && cant_tail.get(idx + 1) == Some(&"turn")
                    {
                        idx += 2;
                    }
                    if idx == cant_tail.len() {
                        return Some(Restriction::cast_spells_matching(player, spell_filter));
                    }
                }
            }
        }
        if let Some(spell_filter) = parse_cast_more_than_one_limit_filter(cant_tail) {
            return Some(restriction_from_cast_limit_filter(player, spell_filter));
        }
        return None;
    }

    if let Some(spell_filter) = parse_cast_additional_limit_filter(words) {
        return Some(restriction_from_cast_limit_filter(
            PlayerFilter::Any,
            spell_filter,
        ));
    }

    None
}

fn parse_cant_cast_subject(words: &[&str]) -> Option<(PlayerFilter, usize)> {
    if slice_starts_with(&words, &["that", "player"]) {
        return Some((PlayerFilter::IteratedPlayer, 2));
    }
    if slice_starts_with(&words, &["your", "opponents", "who", "have"]) {
        return Some((PlayerFilter::Opponent, 4));
    }
    if slice_starts_with(&words, &["each", "player", "who", "has"]) {
        return Some((PlayerFilter::Any, 4));
    }
    if slice_starts_with(&words, &["each", "opponent", "who", "has"]) {
        return Some((PlayerFilter::Opponent, 4));
    }
    if slice_starts_with(&words, &["your", "opponents"]) {
        return Some((PlayerFilter::Opponent, 2));
    }
    if slice_starts_with(&words, &["each", "player"]) {
        return Some((PlayerFilter::Any, 2));
    }
    if slice_starts_with(&words, &["each", "opponent"]) {
        return Some((PlayerFilter::Opponent, 2));
    }
    match words.first().copied() {
        Some("players") => Some((PlayerFilter::Any, 1)),
        Some("opponents") => Some((PlayerFilter::Opponent, 1)),
        Some("they") => Some((PlayerFilter::IteratedPlayer, 1)),
        Some("you") => Some((PlayerFilter::You, 1)),
        _ => None,
    }
}

fn parse_cast_more_than_one_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    if !matches!(words, ["cast", "more", "than", "one", ..]) {
        return None;
    }
    let mut idx = 4usize;
    let (spell_filter, consumed) = if words.get(idx) == Some(&"spell") {
        (ObjectFilter::default(), 0usize)
    } else {
        parse_cast_limit_qualifier(&words[idx..])?
    };
    idx += consumed;

    if words.get(idx) != Some(&"spell")
        || words.get(idx + 1) != Some(&"each")
        || words.get(idx + 2) != Some(&"turn")
        || idx + 3 != words.len()
    {
        return None;
    }

    Some(spell_filter)
}

fn parse_cast_additional_limit_filter(words: &[&str]) -> Option<ObjectFilter> {
    let mut idx = 0usize;
    if matches!(words, ["who", "has", ..]) {
        idx += 2;
    }

    if words.get(idx) != Some(&"cast") {
        return None;
    }
    idx += 1;
    if words
        .get(idx)
        .is_some_and(|word| *word == "a" || *word == "an")
    {
        idx += 1;
    }

    let (first_filter, first_used) = parse_cast_limit_qualifier(&words[idx..])?;
    idx += first_used;

    if words.get(idx) != Some(&"spell") {
        return None;
    }
    idx += 1;

    if words.get(idx) == Some(&"this") && words.get(idx + 1) == Some(&"turn") {
        idx += 2;
    }

    if words.get(idx) != Some(&"cant")
        || words.get(idx + 1) != Some(&"cast")
        || words.get(idx + 2) != Some(&"additional")
    {
        return None;
    }
    idx += 3;

    let (second_filter, second_used) = parse_cast_limit_qualifier(&words[idx..])?;
    if second_filter != first_filter {
        return None;
    }
    idx += second_used;

    if words.get(idx) != Some(&"spells") || idx + 1 != words.len() {
        return None;
    }

    Some(first_filter)
}

fn parse_cast_limit_qualifier(words: &[&str]) -> Option<(ObjectFilter, usize)> {
    let parse_non_term = |term: &str| -> Option<ObjectFilter> {
        let normalized = term.trim_end_matches('s');
        if let Some(card_type) = parse_card_type(normalized) {
            return Some(ObjectFilter::default().without_type(card_type));
        }
        if let Some(subtype) = parse_subtype_word(normalized) {
            return Some(ObjectFilter::default().without_subtype(subtype));
        }
        None
    };
    let parse_positive_term = |term: &str| -> Option<ObjectFilter> {
        let normalized = term.trim_end_matches('s');
        if let Some(card_type) = parse_card_type(normalized) {
            return Some(ObjectFilter::default().with_type(card_type));
        }
        if let Some(subtype) = parse_subtype_word(normalized) {
            return Some(ObjectFilter::default().with_subtype(subtype));
        }
        None
    };

    if let Some(first) = words.first().copied() {
        if let Some(term) =
            str_strip_prefix(first, "non-").or_else(|| str_strip_prefix(first, "non"))
            && !term.is_empty()
            && let Some(filter) = parse_non_term(term)
        {
            return Some((filter, 1));
        }
    }

    if words.len() >= 2
        && words[0] == "non"
        && let Some(filter) = parse_non_term(words[1])
    {
        return Some((filter, 2));
    }

    if let Some(first) = words.first().copied()
        && let Some(filter) = parse_positive_term(first)
    {
        let mut filters = vec![filter];
        let mut used = 1usize;
        while words
            .get(used)
            .is_some_and(|word| *word == "or" || *word == "and")
        {
            let Some(next_word) = words.get(used + 1).copied() else {
                break;
            };
            let Some(next_filter) = parse_positive_term(next_word) else {
                break;
            };
            filters.push(next_filter);
            used += 2;
        }
        if filters.len() == 1 {
            return Some((filters.pop().expect("single filter"), used));
        }
        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = filters;
        return Some((disjunction, used));
    }

    None
}

fn strip_static_restriction_condition(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::ConditionExpr, Vec<OwnedLexToken>)>, CardTextError> {
    let normalized_storage = normalize_cant_words(tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if slice_starts_with(&normalized, &["during", "your", "turn"]) {
        let remainder = find_index(tokens, |token| token.is_comma())
            .map(|idx| trim_commas(&tokens[idx + 1..]).to_vec())
            .unwrap_or_else(|| trim_commas(&tokens[3..]).to_vec());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringYourTurn),
            remainder,
        )));
    }

    if slice_starts_with(&normalized, &["during", "combat"]) {
        let remainder = find_index(tokens, |token| token.is_comma())
            .map(|idx| trim_commas(&tokens[idx + 1..]).to_vec())
            .unwrap_or_else(|| trim_commas(&tokens[2..]).to_vec());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringCombat),
            remainder,
        )));
    }

    if slice_ends_with(&normalized, &["during", "your", "turn"]) {
        let cut = rfind_index(tokens, |token| token.is_word("during")).unwrap_or(tokens.len());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringYourTurn),
            trim_commas(&tokens[..cut]).to_vec(),
        )));
    }

    if slice_ends_with(&normalized, &["during", "combat"]) {
        let cut = rfind_index(tokens, |token| token.is_word("during")).unwrap_or(tokens.len());
        return Ok(Some((
            crate::ConditionExpr::ActivationTiming(ActivationTiming::DuringCombat),
            trim_commas(&tokens[..cut]).to_vec(),
        )));
    }

    if slice_starts_with(&normalized, &["as", "long", "as"]) {
        let Some(comma_idx) = find_index(tokens, |token| token.is_comma()) else {
            return Ok(None);
        };
        let condition_tokens = trim_commas(&tokens[3..comma_idx]);
        let condition = parse_static_condition_clause(&condition_tokens).or_else(|_| {
            let condition_words = normalize_cant_words(&condition_tokens);
            let normalized_condition = condition_words
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            match normalized_condition.as_slice() {
                ["this", "equipment", "is", "attached", "to", "a", "creature"]
                | ["this", "equipment", "is", "attached", "to", "creature"]
                | ["this", "permanent", "is", "attached", "to", "a", "creature"]
                | ["this", "permanent", "is", "attached", "to", "creature"] => {
                    Ok(crate::ConditionExpr::SourceIsEquipped)
                }
                _ => Err(CardTextError::ParseError(format!(
                    "unsupported static condition clause (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                ))),
            }
        })?;
        return Ok(Some((
            condition,
            trim_commas(&tokens[comma_idx + 1..]).to_vec(),
        )));
    }

    Ok(None)
}

fn parse_player_negated_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    let Some((player, target)) = parse_player_restriction_subject(&subject_tokens)? else {
        return Ok(None);
    };
    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Ok(None);
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if let Some(spell_filter) = parse_cast_restriction_tail_filter(&remainder_words) {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::cast_spells_matching(player, spell_filter),
            target,
        }));
    }
    if remainder_words.as_slice() == ["cast", "spells"] {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::cast_spells(player),
            target,
        }));
    }
    if remainder_words.as_slice()
        == [
            "activate",
            "abilities",
            "that",
            "arent",
            "mana",
            "abilities",
        ]
    {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::activate_non_mana_abilities(player),
            target,
        }));
    }
    if slice_starts_with(&remainder_words, &["activate", "abilities", "of"]) {
        let Some(mut filter) =
            parse_card_type_list_filter(&remainder_words[3..], Some(Zone::Battlefield))
        else {
            return Ok(None);
        };
        filter.controller = Some(player);
        let restriction =
            if slice_ends_with(&remainder_words, &["unless", "theyre", "mana", "abilities"]) {
                Restriction::activate_non_mana_abilities_of(filter)
            } else {
                Restriction::activate_abilities_of(filter)
            };
        return Ok(Some(ParsedCantRestriction {
            restriction,
            target,
        }));
    }

    Ok(None)
}

fn parse_player_restriction_subject(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerFilter, Option<TargetAst>)>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    if starts_with_target_indicator(subject_tokens) {
        let target = parse_target_phrase(subject_tokens)?;
        if let TargetAst::Player(player, span) = &target {
            return Ok(Some((
                target_ast_player_filter(player.clone(), *span),
                Some(target),
            )));
        }
        return Ok(None);
    }

    let normalized_storage = normalize_cant_words(subject_tokens);
    let normalized = normalized_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    match normalized.as_slice() {
        ["you"] => return Ok(Some((PlayerFilter::You, None))),
        ["that", "player"] | ["they"] => {
            return Ok(Some((PlayerFilter::IteratedPlayer, None)));
        }
        ["your", "opponents"] | ["each", "opponent"] | ["opponents"] => {
            return Ok(Some((PlayerFilter::Opponent, None)));
        }
        ["players"] | ["each", "player"] => return Ok(Some((PlayerFilter::Any, None))),
        ["defending", "player"] => return Ok(Some((PlayerFilter::Defending, None))),
        ["attacking", "player"] => return Ok(Some((PlayerFilter::Attacking, None))),
        ["its", "controller"] | ["their", "controller"] => {
            return Ok(Some((
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG))),
                None,
            )));
        }
        ["its", "owner"] | ["their", "owner"] => {
            return Ok(Some((
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG))),
                None,
            )));
        }
        _ => {}
    }

    let player = match parse_subject(subject_tokens) {
        crate::cards::builders::SubjectAst::Player(PlayerAst::You | PlayerAst::Implicit) => {
            PlayerFilter::You
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Opponent) => PlayerFilter::Opponent,
        crate::cards::builders::SubjectAst::Player(PlayerAst::That) => PlayerFilter::IteratedPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Defending) => PlayerFilter::Defending,
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsController) => {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::ItsOwner) => {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(TagKey::from(IT_TAG)))
        }
        crate::cards::builders::SubjectAst::Player(PlayerAst::Chosen) => PlayerFilter::ChosenPlayer,
        crate::cards::builders::SubjectAst::Player(PlayerAst::Attacking) => PlayerFilter::Attacking,
        _ => return Ok(None),
    };
    Ok(Some((player, None)))
}

fn target_ast_player_filter(player: PlayerFilter, span: Option<TextSpan>) -> PlayerFilter {
    if span.is_some() {
        match player {
            PlayerFilter::Any => PlayerFilter::target_player(),
            PlayerFilter::Opponent => PlayerFilter::target_opponent(),
            other => other,
        }
    } else {
        player
    }
}

fn parse_cast_restriction_tail_filter(words: &[&str]) -> Option<ObjectFilter> {
    if words == ["cast", "spells"] {
        return Some(ObjectFilter::default());
    }
    if words.first() != Some(&"cast") || words.last() != Some(&"spells") || words.len() < 3 {
        return None;
    }
    let tail = &words[1..words.len() - 1];
    let (filter, used) = parse_cast_limit_qualifier(tail)?;
    (used == tail.len()).then_some(filter)
}

fn parse_card_type_list_filter(words: &[&str], zone: Option<Zone>) -> Option<ObjectFilter> {
    let cleaned = words
        .iter()
        .copied()
        .filter(|word| !matches!(*word, "a" | "an" | "the" | "or" | "and" | ","))
        .filter(|word| !matches!(*word, "unless" | "theyre" | "mana" | "abilities"))
        .collect::<Vec<_>>();
    if cleaned.is_empty() {
        return None;
    }

    let mut filters = Vec::new();
    for word in cleaned {
        let normalized = word.trim_end_matches('s');
        let card_type = parse_card_type(normalized)?;
        let mut filter = ObjectFilter::default();
        filter.zone = zone;
        filter.card_types.push(card_type);
        filters.push(filter);
    }
    if filters.len() == 1 {
        return filters.pop();
    }
    let mut disjunction = ObjectFilter::default();
    disjunction.any_of = filters;
    Some(disjunction)
}

fn restriction_from_cast_limit_filter(
    player: PlayerFilter,
    spell_filter: ObjectFilter,
) -> crate::effect::Restriction {
    crate::effect::Restriction::cast_more_than_one_spell_each_turn_matching(player, spell_filter)
}

pub(crate) fn parse_negated_object_restriction_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);

    let (filter, target, ability_scope) =
        if let Some(parsed) = parse_activated_ability_subject(&subject_tokens)? {
            (parsed.filter, parsed.target, Some(parsed.scope))
        } else if starts_with_target_indicator(&subject_tokens) {
            let target = parse_target_phrase(&subject_tokens)?;
            let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported target restriction subject (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                ))
            })?;
            ensure_it_tagged_constraint(&mut filter);
            (filter, Some(target), None)
        } else if subject_tokens.is_empty() {
            // Supports carried clauses like "... and can't be blocked this turn."
            let target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
            (
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                Some(target),
                None,
            )
        } else {
            let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported subject in negated restriction clause (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            };
            (filter, None, None)
        };

    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing restriction tail in negated restriction clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let remainder_words_storage = normalize_cant_words(&remainder_tokens);
    let remainder_words = remainder_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let subject_words_storage = normalize_cant_words(&subject_tokens);
    let subject_words = subject_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();

    if matches!(
        subject_words.as_slice(),
        ["damage"] | ["the", "damage"] | ["that", "damage"]
    ) && remainder_words.as_slice() == ["be", "prevented"]
    {
        return Ok(Some(ParsedCantRestriction {
            restriction: Restriction::prevent_damage(),
            target: None,
        }));
    }

    let restriction = match remainder_words.as_slice() {
        ["attack"] => Restriction::attack(filter),
        ["attack", "this", "turn"] => Restriction::attack(filter),
        ["attack", "alone"] => Restriction::attack_alone(filter),
        ["attack", "alone", "this", "turn"] => Restriction::attack_alone(filter),
        ["attack", "or", "block"] => Restriction::attack_or_block(filter),
        ["attack", "or", "block", "this", "turn"] => Restriction::attack_or_block(filter),
        ["attack", "or", "block", "alone"] => Restriction::attack_or_block_alone(filter),
        ["attack", "or", "block", "alone", "this", "turn"] => {
            Restriction::attack_or_block_alone(filter)
        }
        ["block"] => Restriction::block(filter),
        ["block", "this", "turn"] => Restriction::block(filter),
        ["block", "alone"] => Restriction::block_alone(filter),
        ["block", "alone", "this", "turn"] => Restriction::block_alone(filter),
        ["be", "blocked"] => Restriction::be_blocked(filter),
        ["be", "blocked", "this", "turn"] => Restriction::be_blocked(filter),
        _ if slice_starts_with(&remainder_words, &["be", "blocked", "by"])
            && remainder_words.len() > 3 =>
        {
            let blocker_tokens = trim_commas(&remainder_tokens[3..]);
            let blocker_filter = parse_subject_object_filter(&blocker_tokens)?
                .or_else(|| parse_object_filter(&blocker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(blocker_filter, filter)
        }
        ["be", "destroyed"] => Restriction::be_destroyed(filter),
        ["be", "regenerated"] => Restriction::be_regenerated(filter),
        ["be", "regenerated", "this", "turn"] => Restriction::be_regenerated(filter),
        ["be", "sacrificed"] => Restriction::be_sacrificed(filter),
        ["be", "countered"] => Restriction::be_countered(filter),
        ["be", "activated"] | ["be", "activated", "this", "turn"] => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) => {
                Restriction::activate_tap_abilities_of(filter)
            }
            None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            }
        },
        ["be", "activated", "unless", "theyre", "mana", "abilities"] => match ability_scope {
            Some(ActivatedAbilityScope::All) => Restriction::activate_non_mana_abilities_of(filter),
            Some(ActivatedAbilityScope::TapCostOnly) | None => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported negated restriction tail (clause: '{}')",
                    crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                )));
            }
        },
        ["transform"] => Restriction::transform(filter),
        ["be", "targeted"] => Restriction::be_targeted(filter),
        _ if remainder_words.first() == Some(&"block") && remainder_words.len() > 1 => {
            let attacker_tokens = trim_commas(&remainder_tokens[1..]);
            let attacker_filter = parse_subject_object_filter(&attacker_tokens)?
                .or_else(|| parse_object_filter(&attacker_tokens, false).ok())
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported negated restriction tail (clause: '{}')",
                        crate::cards::builders::parser::token_word_refs(tokens).join(" ")
                    ))
                })?;
            Restriction::block_specific_attacker(filter, attacker_filter)
        }
        _ if is_supported_untap_restriction_tail(&remainder_words) => Restriction::untap(filter),
        _ => {
            if matches!(
                remainder_words.first().copied(),
                Some(
                    "put"
                        | "draw"
                        | "reveal"
                        | "look"
                        | "search"
                        | "create"
                        | "return"
                        | "exile"
                        | "sacrifice"
                        | "discard"
                        | "gain"
                        | "lose"
                )
            ) {
                return Ok(None);
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported negated restriction tail (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            )));
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target,
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActivatedAbilityScope {
    All,
    TapCostOnly,
}

#[derive(Debug, Clone)]
struct ParsedActivatedAbilitySubject {
    filter: ObjectFilter,
    target: Option<TargetAst>,
    scope: ActivatedAbilityScope,
}

fn strip_trailing_possessive_token(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut normalized = tokens.to_vec();
    if let Some(last) = normalized.last_mut()
        && let Some(word) = last.as_word().map(str::to_string)
    {
        if let Some(stripped) = str_strip_suffix(&word, "'s")
            .or_else(|| str_strip_suffix(&word, "’s"))
            .or_else(|| str_strip_suffix(&word, "s'"))
            .or_else(|| str_strip_suffix(&word, "s’"))
        {
            last.replace_word(stripped);
        }
    }
    normalized
}

fn parse_activated_ability_subject(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedActivatedAbilitySubject>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let subject_words_storage = normalize_cant_words(tokens);
    let subject_words = subject_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let (owner_word_len, scope) = if slice_ends_with(&subject_words, &["activated", "abilities"]) {
        (
            subject_words.len().saturating_sub(2),
            ActivatedAbilityScope::All,
        )
    } else if slice_ends_with(
        &subject_words,
        &[
            "activated",
            "abilities",
            "with",
            "t",
            "in",
            "their",
            "costs",
        ],
    ) {
        (
            subject_words.len().saturating_sub(7),
            ActivatedAbilityScope::TapCostOnly,
        )
    } else {
        return Ok(None);
    };

    if owner_word_len == 0 {
        return Ok(None);
    }
    let owner_end = ActivationRestrictionCompatWords::new(tokens)
        .token_index_after_words(owner_word_len)
        .unwrap_or(tokens.len());
    let owner_tokens = trim_commas(&tokens[..owner_end]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let normalized_owner_tokens = strip_trailing_possessive_token(&owner_tokens);

    let owner_word_view = ActivationRestrictionCompatWords::new(&normalized_owner_tokens);
    let owner_words = owner_word_view.to_word_refs();
    if owner_words.len() == 1 && matches!(owner_words[0], "it" | "its" | "them" | "their") {
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            target: Some(TargetAst::Tagged(
                TagKey::from(IT_TAG),
                span_from_tokens(tokens),
            )),
            scope,
        }));
    }

    if starts_with_target_indicator(&normalized_owner_tokens) {
        let target = parse_target_phrase(&normalized_owner_tokens)?;
        let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported target restriction subject (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            ))
        })?;
        ensure_it_tagged_constraint(&mut filter);
        return Ok(Some(ParsedActivatedAbilitySubject {
            filter,
            target: Some(target),
            scope,
        }));
    }

    let Some(filter) = parse_subject_object_filter(&normalized_owner_tokens)?
        .or_else(|| parse_object_filter(&normalized_owner_tokens, false).ok())
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject in negated restriction clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    };

    Ok(Some(ParsedActivatedAbilitySubject {
        filter,
        target: None,
        scope,
    }))
}

fn ensure_it_tagged_constraint(filter: &mut ObjectFilter) {
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
}

fn starts_with_possessive_activated_ability_subject(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        words.as_slice(),
        ["its", "activated", "abilities", ..] | ["their", "activated", "abilities", ..]
    )
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedCantRestriction {
    pub(crate) restriction: crate::effect::Restriction,
    pub(crate) target: Option<TargetAst>,
}

pub(crate) fn starts_with_target_indicator(tokens: &[OwnedLexToken]) -> bool {
    let mut idx = 0usize;
    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((_, used)) = parse_number(&tokens[idx..]) {
            idx += used;
        }
    } else if let Some((_, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        idx += used;
    } else if let Some((_, used)) = parse_number(&tokens[idx..])
        && tokens
            .get(idx + used)
            .is_some_and(|token: &OwnedLexToken| token.is_word("target"))
    {
        idx += used;
    } else if tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        idx += 1;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        idx += 1;
    }

    tokens.get(idx).is_some_and(|token| token.is_word("target"))
}

pub(crate) fn find_negation_span(tokens: &[OwnedLexToken]) -> Option<(usize, usize)> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    for word_idx in 0..word_view.len() {
        let Some(word) = word_view.get(word_idx) else {
            continue;
        };
        if matches!(word, "cant" | "cannot") {
            let start = word_view.token_index_for_word_index(word_idx)?;
            let end = word_view.token_index_after_words(word_idx + 1)?;
            return Some((start, end));
        }
        if matches!(word, "doesnt" | "dont") {
            let next_word = word_view.get(word_idx + 1);
            if matches!(next_word, Some("control" | "controls" | "own" | "owns")) {
                continue;
            }
            let start = word_view.token_index_for_word_index(word_idx)?;
            let end = word_view.token_index_after_words(word_idx + 1)?;
            return Some((start, end));
        }
        if matches!(word, "does" | "do" | "can") && word_view.get(word_idx + 1) == Some("not") {
            if matches!(word, "does" | "do")
                && matches!(
                    word_view.get(word_idx + 2),
                    Some("control" | "controls" | "own" | "owns")
                )
            {
                continue;
            }
            let start = word_view.token_index_for_word_index(word_idx)?;
            let end = word_view.token_index_after_words(word_idx + 2)?;
            return Some((start, end));
        }
    }
    None
}

pub(crate) fn parse_subject_object_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let normalized_words_storage = normalize_cant_words(tokens);
    let normalized_words = normalized_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if matches!(
        normalized_words.as_slice(),
        ["damage"] | ["the", "damage"] | ["that", "damage"]
    ) {
        return Ok(Some(ObjectFilter::default()));
    }
    if matches!(
        normalized_words.as_slice(),
        ["it"] | ["they"] | ["them"] | ["itself"] | ["themselves"]
    ) {
        return Ok(Some(ObjectFilter::tagged(TagKey::from(IT_TAG))));
    }

    let words_all = crate::cards::builders::parser::token_word_refs(tokens);
    if find_window_by(&words_all, 3, |window| {
        window == ["power", "or", "toughness"] || window == ["toughness", "or", "power"]
    })
    .is_some()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject object filter (clause: '{}')",
            words_all.join(" ")
        )));
    }

    if let Ok(filter) = parse_object_filter(tokens, false)
        && filter != ObjectFilter::default()
    {
        return Ok(Some(filter));
    }

    let target = parse_target_phrase(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject target phrase (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;

    Ok(target_ast_to_object_filter(target))
}

pub(crate) fn target_ast_to_object_filter(target: TargetAst) -> Option<ObjectFilter> {
    match target {
        TargetAst::Source(_) => Some(ObjectFilter::source()),
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::Spell(_) => Some(ObjectFilter::spell()),
        TargetAst::Tagged(tag, _) => Some(ObjectFilter::tagged(tag)),
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

pub(crate) fn is_supported_untap_restriction_tail(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if !(words[0] == "untap" || words[0] == "untaps") {
        return false;
    }
    if words.len() == 1 {
        return true;
    }

    let allowed = [
        "untap",
        "untaps",
        "during",
        "its",
        "their",
        "your",
        "controllers",
        "controller",
        "untap",
        "step",
        "steps",
        "next",
        "the",
    ];
    if words.iter().any(|word| !slice_contains(&allowed, word)) {
        return false;
    }

    slice_contains(&words, &"during")
        && (slice_contains(&words, &"step") || slice_contains(&words, &"steps"))
}

pub(crate) fn normalize_cant_words(tokens: &[OwnedLexToken]) -> Vec<String> {
    ActivationRestrictionCompatWords::new(tokens)
        .to_word_refs()
        .into_iter()
        .map(|word| {
            if word == "cannot" {
                "cant".to_string()
            } else {
                word.to_string()
            }
        })
        .collect()
}

pub(crate) fn keyword_title(keyword: &str) -> String {
    let mut words = keyword.split_whitespace();
    let Some(first) = words.next() else {
        return String::new();
    };
    let mut out = String::new();
    let mut first_chars = first.chars();
    if let Some(ch) = first_chars.next() {
        out.push(ch.to_ascii_uppercase());
        out.push_str(first_chars.as_str());
    }
    for word in words {
        out.push(' ');
        out.push_str(word);
    }
    out
}

pub(crate) fn leading_mana_symbols_to_oracle(words: &[&str]) -> Option<(String, usize)> {
    if words.is_empty() {
        return None;
    }
    let mut pips = Vec::new();
    let mut consumed = 0usize;
    for word in words {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        pips.push(vec![symbol]);
        consumed += 1;
    }
    if consumed == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips).to_oracle(), consumed))
}

pub(crate) fn marker_keyword_id(keyword: &str) -> Option<&'static str> {
    match keyword {
        "banding" => Some("banding"),
        "fabricate" => Some("fabricate"),
        "foretell" => Some("foretell"),
        "bestow" => Some("bestow"),
        "dash" => Some("dash"),
        "overload" => Some("overload"),
        "soulshift" => Some("soulshift"),
        "adapt" => Some("adapt"),
        "bolster" => Some("bolster"),
        "disturb" => Some("disturb"),
        "echo" => Some("echo"),
        "modular" => Some("modular"),
        "ninjutsu" => Some("ninjutsu"),
        "outlast" => Some("outlast"),
        "scavenge" => Some("scavenge"),
        "suspend" => Some("suspend"),
        "vanishing" => Some("vanishing"),
        "offering" => Some("offering"),
        "soulbond" => Some("soulbond"),
        "unearth" => Some("unearth"),
        "specialize" => Some("specialize"),
        "squad" => Some("squad"),
        "spectacle" => Some("spectacle"),
        "graft" => Some("graft"),
        "backup" => Some("backup"),
        "saddle" => Some("saddle"),
        "fading" => Some("fading"),
        "fuse" => Some("fuse"),
        "plot" => Some("plot"),
        "disguise" => Some("disguise"),
        "tribute" => Some("tribute"),
        "buyback" => Some("buyback"),
        "flashback" => Some("flashback"),
        "rebound" => Some("rebound"),
        _ => None,
    }
}

pub(crate) fn marker_keyword_display(words: &[&str]) -> Option<String> {
    let keyword = words.first().copied()?;
    let title = keyword_title(keyword);

    match keyword {
        "soulshift" | "adapt" | "bolster" | "modular" | "vanishing" | "backup" | "saddle"
        | "fading" | "graft" | "tribute" => {
            let amount = words.get(1)?.parse::<u32>().ok()?;
            Some(format!("{title} {amount}"))
        }
        "bestow" | "dash" | "disturb" | "ninjutsu" | "outlast" | "scavenge" | "unearth"
        | "specialize" | "spectacle" | "plot" | "disguise" | "flashback" | "foretell"
        | "overload" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("{title} {cost}"))
        }
        "echo" => {
            if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
                return Some(format!("Echo {cost}"));
            }
            if words.len() > 1 {
                let payload = words[1..].join(" ");
                let mut chars = payload.chars();
                let Some(first) = chars.next() else {
                    return Some("Echo".to_string());
                };
                let mut normalized = String::new();
                normalized.push(first.to_ascii_uppercase());
                normalized.push_str(chars.as_str());
                return Some(format!("Echo—{normalized}"));
            }
            Some("Echo".to_string())
        }
        "buyback" => {
            if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
                Some(format!("Buyback {cost}"))
            } else if words.len() > 1 {
                Some(format!("Buyback—{}", words[1..].join(" ")))
            } else {
                Some("Buyback".to_string())
            }
        }
        "suspend" => {
            let time = words.get(1)?.parse::<u32>().ok()?;
            let (cost, _) = leading_mana_symbols_to_oracle(&words[2..])?;
            Some(format!("Suspend {time}—{cost}"))
        }
        "rebound" => Some("Rebound".to_string()),
        "squad" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("Squad {cost}"))
        }
        _ => None,
    }
}

fn marker_text_from_words(words: &[&str]) -> Option<String> {
    let first = words.first().copied()?;
    let mut text = keyword_title(first);
    if words.len() > 1 {
        text.push(' ');
        text.push_str(&words[1..].join(" "));
    }
    Some(text)
}

fn parse_numeric_keyword_action<F>(
    words: &[&str],
    keyword: &'static str,
    build: F,
) -> Option<KeywordAction>
where
    F: FnOnce(u32) -> KeywordAction,
{
    if words.first().copied() != Some(keyword) {
        return None;
    }
    if let Some(amount) = words.get(1).and_then(|word| word.parse::<u32>().ok()) {
        return Some(build(amount));
    }
    Some(KeywordAction::Marker(keyword))
}

enum KeywordCostFallback {
    MarkerOnly,
    MarkerOrText,
}

fn parse_cost_keyword_action<F>(
    words: &[&str],
    keyword: &'static str,
    fallback: KeywordCostFallback,
    build: F,
) -> Option<KeywordAction>
where
    F: FnOnce(ManaCost) -> KeywordAction,
{
    if words.first().copied() != Some(keyword) {
        return None;
    }
    if let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[1..])
        && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
    {
        return Some(build(cost));
    }
    if matches!(fallback, KeywordCostFallback::MarkerOrText) && words.len() > 1 {
        if let Some(display) = marker_keyword_display(words) {
            return Some(KeywordAction::MarkerText(display));
        }
    }
    Some(KeywordAction::Marker(keyword))
}

pub(crate) fn parse_single_word_keyword_action(word: &str) -> Option<KeywordAction> {
    match word {
        "flying" => Some(KeywordAction::Flying),
        "menace" => Some(KeywordAction::Menace),
        "hexproof" => Some(KeywordAction::Hexproof),
        "haste" => Some(KeywordAction::Haste),
        "improvise" => Some(KeywordAction::Improvise),
        "convoke" => Some(KeywordAction::Convoke),
        "delve" => Some(KeywordAction::Delve),
        "deathtouch" => Some(KeywordAction::Deathtouch),
        "lifelink" => Some(KeywordAction::Lifelink),
        "vigilance" => Some(KeywordAction::Vigilance),
        "trample" => Some(KeywordAction::Trample),
        "reach" => Some(KeywordAction::Reach),
        "defender" => Some(KeywordAction::Defender),
        "flash" => Some(KeywordAction::Flash),
        "phasing" => Some(KeywordAction::Phasing),
        "indestructible" => Some(KeywordAction::Indestructible),
        "shroud" => Some(KeywordAction::Shroud),
        "assist" => Some(KeywordAction::Assist),
        "backup" => Some(KeywordAction::Marker("backup")),
        "cipher" => Some(KeywordAction::Cipher),
        "devoid" => Some(KeywordAction::Devoid),
        "dethrone" => Some(KeywordAction::Dethrone),
        "enlist" => Some(KeywordAction::Enlist),
        "evolve" => Some(KeywordAction::Evolve),
        "extort" => Some(KeywordAction::Extort),
        "haunt" => Some(KeywordAction::Haunt),
        "ingest" => Some(KeywordAction::Ingest),
        "mentor" => Some(KeywordAction::Mentor),
        "melee" => Some(KeywordAction::Melee),
        "training" => Some(KeywordAction::Training),
        "myriad" => Some(KeywordAction::Myriad),
        "partner" => Some(KeywordAction::Partner),
        "provoke" => Some(KeywordAction::Provoke),
        "ravenous" => Some(KeywordAction::Ravenous),
        "riot" => Some(KeywordAction::Riot),
        "skulk" => Some(KeywordAction::Skulk),
        "sunburst" => Some(KeywordAction::Sunburst),
        "undaunted" => Some(KeywordAction::Undaunted),
        "unleash" => Some(KeywordAction::Unleash),
        "wither" => Some(KeywordAction::Wither),
        "infect" => Some(KeywordAction::Infect),
        "undying" => Some(KeywordAction::Undying),
        "persist" => Some(KeywordAction::Persist),
        "prowess" => Some(KeywordAction::Prowess),
        "exalted" => Some(KeywordAction::Exalted),
        "cascade" => Some(KeywordAction::Cascade),
        "storm" => Some(KeywordAction::Storm),
        "rebound" => Some(KeywordAction::Rebound),
        "ascend" => Some(KeywordAction::Ascend),
        "compleated" => Some(KeywordAction::Marker("compleated")),
        "daybound" => Some(KeywordAction::Daybound),
        "nightbound" => Some(KeywordAction::Nightbound),
        "islandwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Island,
                snow: false,
            },
        )),
        "swampwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Swamp,
                snow: false,
            },
        )),
        "mountainwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Mountain,
                snow: false,
            },
        )),
        "forestwalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Forest,
                snow: false,
            },
        )),
        "plainswalk" => Some(KeywordAction::Landwalk(
            crate::static_abilities::LandwalkKind::Subtype {
                subtype: Subtype::Plains,
                snow: false,
            },
        )),
        "fear" => Some(KeywordAction::Fear),
        "intimidate" => Some(KeywordAction::Intimidate),
        "shadow" => Some(KeywordAction::Shadow),
        "horsemanship" => Some(KeywordAction::Horsemanship),
        "flanking" => Some(KeywordAction::Flanking),
        "changeling" => Some(KeywordAction::Changeling),
        _ => None,
    }
}

pub(crate) fn parse_ability_phrase(tokens: &[OwnedLexToken]) -> Option<KeywordAction> {
    let mut phrase_tokens = tokens;
    if phrase_tokens
        .first()
        .is_some_and(|token| token.is_word("and"))
    {
        phrase_tokens = &phrase_tokens[1..];
    }

    let word_view = ActivationRestrictionCompatWords::new(phrase_tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let (head, second) = lexed_head_words(phrase_tokens).unwrap_or(("", None));

    match words.as_slice() {
        ["landwalk"] => {
            return Some(KeywordAction::Landwalk(
                crate::static_abilities::LandwalkKind::AnyLand,
            ));
        }
        ["nonbasic", "landwalk"] => {
            return Some(KeywordAction::Landwalk(
                crate::static_abilities::LandwalkKind::NonbasicLand,
            ));
        }
        ["artifact", "landwalk"] => {
            return Some(KeywordAction::Landwalk(
                crate::static_abilities::LandwalkKind::ArtifactLand,
            ));
        }
        ["snow", subtype_walk] => {
            if let Some(action) = parse_single_word_keyword_action(subtype_walk)
                && let KeywordAction::Landwalk(crate::static_abilities::LandwalkKind::Subtype {
                    subtype,
                    ..
                }) = action
            {
                return Some(KeywordAction::Landwalk(
                    crate::static_abilities::LandwalkKind::Subtype {
                        subtype,
                        snow: true,
                    },
                ));
            }
        }
        _ => {}
    }

    if strip_prefix_phrase(phrase_tokens, &["cumulative", "upkeep"]).is_some() {
        let reminder_start =
            find_index(phrase_tokens, |token| token.is_period()).unwrap_or(phrase_tokens.len());
        let cost_tokens = trim_commas(&phrase_tokens[2..reminder_start]).to_vec();
        let cost_word_view = ActivationRestrictionCompatWords::new(&cost_tokens);
        let cost_words = cost_word_view.to_word_refs();

        if cost_words.len() == 3
            && cost_words[0] == "pay"
            && cost_words[2] == "life"
            && let Ok(life_per_counter) = cost_words[1].parse::<u32>()
            && life_per_counter > 0
        {
            return Some(KeywordAction::CumulativeUpkeep {
                mana_symbols_per_counter: Vec::new(),
                life_per_counter,
                text: format!("Cumulative upkeep—Pay {life_per_counter} life"),
            });
        }

        let mut pips = Vec::new();
        let mut parsed_all = !cost_tokens.is_empty();
        for token in &cost_tokens {
            let Some(group) = mana_pips_from_token(token) else {
                parsed_all = false;
                break;
            };
            pips.push(group);
        }
        if parsed_all && !pips.is_empty() {
            let cost = crate::mana::ManaCost::from_pips(pips.clone()).to_oracle();
            let mut mana_symbols_per_counter = Vec::new();
            let mut flattenable = true;
            for pip in pips {
                let [symbol] = pip.as_slice() else {
                    flattenable = false;
                    break;
                };
                mana_symbols_per_counter.push(*symbol);
            }
            if flattenable && !mana_symbols_per_counter.is_empty() {
                return Some(KeywordAction::CumulativeUpkeep {
                    mana_symbols_per_counter,
                    life_per_counter: 0,
                    text: format!("Cumulative upkeep {cost}"),
                });
            }
        }

        let mut text = "Cumulative upkeep".to_string();
        let tail = &words[2..];
        if !tail.is_empty() {
            if tail.first().copied() == Some("add")
                && let Some((cost, consumed)) = leading_mana_symbols_to_oracle(&tail[1..])
                && consumed + 1 == tail.len()
            {
                text = format!("Cumulative upkeep—Add {cost}");
            } else if let Some((cost, consumed)) = leading_mana_symbols_to_oracle(tail)
                && consumed == tail.len()
            {
                text = format!("Cumulative upkeep {cost}");
            } else if tail.len() == 3
                && tail[1] == "or"
                && let (Some((left, 1)), Some((right, 1))) = (
                    leading_mana_symbols_to_oracle(&tail[..1]),
                    leading_mana_symbols_to_oracle(&tail[2..3]),
                )
            {
                text = format!("Cumulative upkeep {left} or {right}");
            } else {
                let mut tail_text = tail.join(" ");
                if let Some(first) = tail_text.chars().next() {
                    let upper = first.to_ascii_uppercase().to_string();
                    let rest = &tail_text[first.len_utf8()..];
                    tail_text = format!("{upper}{rest}");
                }
                text = format!("Cumulative upkeep—{tail_text}");
            }
        }
        return Some(KeywordAction::MarkerText(text));
    }

    if let Some(action) = parse_numeric_keyword_action(&words, "bushido", KeywordAction::Bushido) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "bloodthirst", KeywordAction::Bloodthirst)
    {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "backup", KeywordAction::Backup) {
        return Some(action);
    }
    if let Some(action) = parse_numeric_keyword_action(&words, "rampage", KeywordAction::Rampage) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "annihilator", KeywordAction::Annihilator)
    {
        return Some(action);
    }

    // Crew appears as "Crew N" and is often followed by inline restrictions/reminder text.
    if head == "crew" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            let has_sorcery_speed =
                contains_word_sequence(&words, &["activate", "only", "as", "a", "sorcery"]);

            let has_once_per_turn = contains_any_word_sequence(
                &words,
                &[
                    &["activate", "only", "once", "each", "turn"],
                    &["activate", "only", "once", "per", "turn"],
                ],
            );

            let mut additional_restrictions = Vec::new();
            let timing = if has_sorcery_speed {
                if has_once_per_turn {
                    additional_restrictions.push("Activate only once each turn.".to_string());
                }
                ActivationTiming::SorcerySpeed
            } else if has_once_per_turn {
                ActivationTiming::OncePerTurn
            } else {
                ActivationTiming::AnyTime
            };

            return Some(KeywordAction::Crew {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported crew variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("crew"));
    }

    // Saddle appears as "Saddle N" and is often followed by reminder text.
    // Per CR 702.171a, Saddle can be activated only as a sorcery.
    if head == "saddle" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            let has_once_per_turn = contains_any_word_sequence(
                &words,
                &[
                    &["activate", "only", "once", "each", "turn"],
                    &["activate", "only", "once", "per", "turn"],
                ],
            );

            let mut additional_restrictions = Vec::new();
            let timing = ActivationTiming::SorcerySpeed;
            if has_once_per_turn {
                additional_restrictions.push("Activate only once each turn.".to_string());
            }

            return Some(KeywordAction::Saddle {
                amount,
                timing,
                additional_restrictions,
            });
        }
        // Fallback: preserve unsupported saddle variants as marker text.
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("saddle"));
    }

    if let Some(action) =
        parse_numeric_keyword_action(&words, "afterlife", KeywordAction::Afterlife)
    {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "fabricate", KeywordAction::Fabricate)
    {
        return Some(action);
    }

    if head == "evolve" {
        return Some(KeywordAction::Evolve);
    }

    if head == "mentor" {
        return Some(KeywordAction::Mentor);
    }

    if head == "training" {
        return Some(KeywordAction::Training);
    }

    if head == "soulbond" {
        return Some(KeywordAction::Soulbond);
    }

    if let Some(action) = parse_numeric_keyword_action(&words, "renown", KeywordAction::Renown) {
        return Some(action);
    }
    if let Some(action) =
        parse_numeric_keyword_action(&words, "soulshift", KeywordAction::Soulshift)
    {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "outlast",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Outlast,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "scavenge",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Scavenge,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "unearth",
        KeywordCostFallback::MarkerOnly,
        KeywordAction::Unearth,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "ninjutsu",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Ninjutsu,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "dash",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Dash,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "warp",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Warp,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "plot",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Plot,
    ) {
        return Some(action);
    }

    if head == "suspend" {
        if let Some(time_word) = words.get(1)
            && let Ok(time) = time_word.parse::<u32>()
            && let Some((cost_text, _consumed)) = leading_mana_symbols_to_oracle(&words[2..])
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Suspend { time, cost });
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("suspend"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("suspend"));
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "disturb",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Disturb,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "foretell",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Foretell,
    ) {
        return Some(action);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "spectacle",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Spectacle,
    ) {
        return Some(action);
    }

    if head == "hideaway" {
        if words.len() == 1 {
            return Some(KeywordAction::MarkerText("Hideaway".to_string()));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if head == "mobilize" {
        if let Some(amount_word) = words.get(1)
            && let Ok(amount) = amount_word.parse::<u32>()
        {
            return Some(KeywordAction::Mobilize(amount));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Marker("mobilize"));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if head == "impending" {
        if words.len() == 1 {
            return Some(KeywordAction::MarkerText("Impending".to_string()));
        }
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if let Some((matched_phrase, _)) = strip_prefix_phrases(
        phrase_tokens,
        &[&["emerge", "from"], &["job", "select"], &["umbra", "armor"]],
    ) {
        return match matched_phrase {
            ["emerge", "from"] => marker_text_from_words(&words).map(KeywordAction::MarkerText),
            ["job", "select"] => Some(KeywordAction::MarkerText("Job select".to_string())),
            ["umbra", "armor"] => Some(KeywordAction::UmbraArmor),
            _ => None,
        };
    }

    if head == "exert" {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if head == "airbend" {
        return marker_text_from_words(&words).map(KeywordAction::MarkerText);
    }

    if let Some(action) = parse_cost_keyword_action(
        &words,
        "overload",
        KeywordCostFallback::MarkerOrText,
        KeywordAction::Overload,
    ) {
        return Some(action);
    }

    if head == "echo" {
        if let Some((cost_text, consumed)) = leading_mana_symbols_to_oracle(&words[1..])
            && consumed > 0
            && let Ok(cost) = parse_scryfall_mana_cost(&cost_text)
        {
            return Some(KeywordAction::Echo {
                total_cost: crate::cost::TotalCost::mana(cost),
                text: format!("Echo {cost_text}"),
            });
        }

        let reminder_start = find_index(phrase_tokens, |token| token.is_period())
            .or_else(|| {
                phrase_tokens
                    .iter()
                    .enumerate()
                    .skip(1)
                    .find_map(|(idx, token)| token.is_word("at").then_some(idx))
            })
            .unwrap_or(phrase_tokens.len());
        let cost_tokens = trim_commas(&phrase_tokens[1..reminder_start]).to_vec();

        if !cost_tokens.is_empty()
            && let Ok(total_cost) = parse_activation_cost(&cost_tokens)
        {
            let text = if let Some(cost) = total_cost.mana_cost()
                && !total_cost.has_non_mana_costs()
            {
                format!("Echo {}", cost.to_oracle())
            } else {
                let payload = cost_tokens
                    .iter()
                    .filter_map(OwnedLexToken::as_word)
                    .collect::<Vec<_>>()
                    .join(" ");
                if payload.is_empty() {
                    "Echo".to_string()
                } else {
                    let mut chars = payload.chars();
                    let first = chars.next().expect("payload is not empty");
                    let mut normalized = String::new();
                    normalized.push(first.to_ascii_uppercase());
                    normalized.push_str(chars.as_str());
                    format!("Echo—{normalized}")
                }
            };
            return Some(KeywordAction::Echo { total_cost, text });
        }

        if words.len() == 1 {
            return Some(KeywordAction::Marker("echo"));
        }
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        return Some(KeywordAction::Marker("echo"));
    }

    if head == "modular" {
        if words.get(1).copied() == Some("sunburst") {
            return Some(KeywordAction::ModularSunburst);
        }
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Modular(amount));
        }
        return Some(KeywordAction::Marker("modular"));
    }

    if head == "graft" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Graft(amount));
        }
        return Some(KeywordAction::Marker("graft"));
    }

    if head == "fading" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Fading(amount));
        }
        return Some(KeywordAction::Marker("fading"));
    }

    if head == "vanishing" {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Vanishing(amount));
        }
        if words.len() == 1 {
            return Some(KeywordAction::Vanishing(0));
        }
        return Some(KeywordAction::Marker("vanishing"));
    }

    if head == "harness" {
        if words.len() > 1 {
            return Some(KeywordAction::MarkerText(format!(
                "Harness {}",
                words[1..].join(" ")
            )));
        }
        return Some(KeywordAction::MarkerText("Harness".to_string()));
    }

    if head == "sunburst" {
        return Some(KeywordAction::Sunburst);
    }
    if let Some((matched_phrase, _)) = strip_prefix_phrases(
        phrase_tokens,
        &[
            &["for", "mirrodin"],
            &["living", "weapon"],
            &["battle", "cry"],
            &["split", "second"],
            &["doctor", "companion"],
        ],
    ) {
        return Some(match matched_phrase {
            ["for", "mirrodin"] => KeywordAction::ForMirrodin,
            ["living", "weapon"] => KeywordAction::LivingWeapon,
            ["battle", "cry"] => KeywordAction::BattleCry,
            ["split", "second"] => KeywordAction::SplitSecond,
            ["doctor", "companion"] => KeywordAction::Marker("doctor companion"),
            _ => unreachable!("matched phrase must be one of the declared keyword heads"),
        });
    }
    if head == "cascade" {
        return Some(KeywordAction::Cascade);
    }

    // Casualty N - "as you cast this spell, you may sacrifice a creature with power N or greater"
    if head == "casualty" {
        if words.len() == 2 {
            if let Ok(power) = words[1].parse::<u32>() {
                return Some(KeywordAction::Casualty(power));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Casualty(1));
        }
        return None;
    }

    // Conspire - "as you cast this spell, you may tap two untapped creatures..."
    if head == "conspire" && words.len() == 1 {
        return Some(KeywordAction::Conspire);
    }

    // Devour N - "as this enters, you may sacrifice any number of creatures..."
    if head == "devour" {
        if words.len() == 2 {
            if let Ok(multiplier) = words[1].parse::<u32>() {
                return Some(KeywordAction::Devour(multiplier));
            }
        }
        if words.len() == 1 {
            return Some(KeywordAction::Devour(1));
        }
        return None;
    }

    if let Some(first) = (!head.is_empty()).then_some(head)
        && matches!(
            first,
            "banding"
                | "fabricate"
                | "foretell"
                | "bestow"
                | "dash"
                | "overload"
                | "soulshift"
                | "adapt"
                | "bolster"
                | "disturb"
                | "echo"
                | "modular"
                | "ninjutsu"
                | "outlast"
                | "suspend"
                | "vanishing"
                | "offering"
                | "specialize"
                | "spectacle"
                | "graft"
                | "backup"
                | "fading"
                | "fuse"
                | "plot"
                | "disguise"
                | "tribute"
                | "buyback"
                | "flashback"
        )
    {
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        if words.len() > 1 {
            return None;
        }
        return Some(KeywordAction::Marker(
            marker_keyword_id(first).expect("marker keyword id must exist for matched keyword"),
        ));
    }

    if words.len() == 1
        && let Some(action) = parse_single_word_keyword_action(words[0])
    {
        return Some(action);
    }

    let action = match words.as_slice() {
        ["affinity", "for", "artifacts"] => KeywordAction::AffinityForArtifacts,
        ["first", "strike"] => KeywordAction::FirstStrike,
        ["double", "strike"] => KeywordAction::DoubleStrike,
        ["for", "mirrodin"] => KeywordAction::ForMirrodin,
        ["living", "weapon"] => KeywordAction::LivingWeapon,
        ["fading", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Fading(value)
        }
        ["vanishing", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Vanishing(value)
        }
        ["modular", "sunburst"] => KeywordAction::ModularSunburst,
        ["modular", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Modular(value)
        }
        ["graft", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Graft(value)
        }
        ["soulshift", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Soulshift(value)
        }
        ["outlast", cost] => {
            let parsed_cost = parse_scryfall_mana_cost(cost).ok()?;
            KeywordAction::Outlast(parsed_cost)
        }
        ["ward", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Ward(value)
        }
        ["afterlife", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Afterlife(value)
        }
        ["backup", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Backup(value)
        }
        ["fabricate", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Fabricate(value)
        }
        ["renown", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Renown(value)
        }
        ["protection", "from", "all", "colors"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "all", "color"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "colorless"] => KeywordAction::ProtectionFromColorless,
        ["protection", "from", "everything"] => KeywordAction::ProtectionFromEverything,
        ["protection", "from", value] => {
            if let Some(color) = parse_color(value) {
                KeywordAction::ProtectionFrom(color)
            } else if let Some(card_type) = parse_card_type(value) {
                KeywordAction::ProtectionFromCardType(card_type)
            } else if let Some(subtype) = parse_subtype_flexible(value) {
                KeywordAction::ProtectionFromSubtype(subtype)
            } else {
                return None;
            }
        }
        _ => {
            // "toxic N" needs exactly 2 words
            if words.len() == 2 && words[0] == "toxic" {
                let amount = words[1].parse::<u32>().ok().unwrap_or(1);
                return Some(KeywordAction::Toxic(amount));
            }
            if words.len() >= 2 {
                if matches!((head, second), ("first", Some("strike"))) {
                    if words.len() > 2 && slice_contains(&words, &"and") {
                        return None;
                    }
                    return Some(KeywordAction::FirstStrike);
                }
                if matches!((head, second), ("double", Some("strike"))) {
                    if words.len() > 2 && slice_contains(&words, &"and") {
                        return None;
                    }
                    return Some(KeywordAction::DoubleStrike);
                }
                if matches!((head, second), ("protection", Some("from"))) && words.len() >= 3 {
                    let value = words[2];
                    return if let Some(color) = parse_color(value) {
                        Some(KeywordAction::ProtectionFrom(color))
                    } else if value == "everything" {
                        Some(KeywordAction::ProtectionFromEverything)
                    } else {
                        parse_card_type(value)
                            .map(KeywordAction::ProtectionFromCardType)
                            .or_else(|| {
                                parse_subtype_flexible(value)
                                    .map(KeywordAction::ProtectionFromSubtype)
                            })
                    };
                }
            }
            if words.len() >= 3 {
                let suffix = &words[words.len() - 3..];
                if suffix == ["cant", "be", "blocked"] || suffix == ["cannot", "be", "blocked"] {
                    return Some(KeywordAction::Unblockable);
                }
            }
            return None;
        }
    };

    Some(action)
}

pub(crate) fn rewrite_attached_controller_trigger_effect_tokens(
    trigger_tokens: &[OwnedLexToken],
    effects_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    let trigger_words = crate::cards::builders::parser::token_word_refs(trigger_tokens);
    let references_enchanted_controller = find_window_by(&trigger_words, 3, |window| {
        window[0] == "enchanted"
            && matches!(
                window[1],
                "creature"
                    | "creatures"
                    | "permanent"
                    | "permanents"
                    | "artifact"
                    | "artifacts"
                    | "enchantment"
                    | "enchantments"
                    | "land"
                    | "lands"
            )
            && window[2] == "controller"
    })
    .is_some();
    if !references_enchanted_controller {
        return effects_tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(effects_tokens.len());
    let mut idx = 0usize;
    while idx < effects_tokens.len() {
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("creature")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(OwnedLexToken::word("enchanted".to_string(), first_span));
            rewritten.push(OwnedLexToken::word("creature".to_string(), second_span));
            idx += 2;
            continue;
        }
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("permanent")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(OwnedLexToken::word("enchanted".to_string(), first_span));
            rewritten.push(OwnedLexToken::word("permanent".to_string(), second_span));
            idx += 2;
            continue;
        }
        rewritten.push(effects_tokens[idx].clone());
        idx += 1;
    }

    rewritten
}

pub(crate) fn maybe_strip_leading_damage_subject_tokens(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if matches!(
        words.get(..2),
        Some(["it", "deals"]) | Some(["this", "deals"])
    ) && !tokens.is_empty()
    {
        return Some(&tokens[1..]);
    }
    None
}

pub(crate) fn looks_like_trigger_object_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }

    let starts_with_or = words.first().copied() == Some("or");
    let first_candidate = if starts_with_or {
        words.get(1).copied()
    } else {
        words.first().copied()
    };
    let Some(first_word) = first_candidate else {
        return false;
    };

    let type_like = parse_card_type(first_word).is_some()
        || parse_subtype_word(first_word).is_some()
        || str_strip_suffix(first_word, "s").is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        });
    if !type_like {
        return false;
    }

    tokens.iter().any(|token| token.is_comma())
}

pub(crate) fn looks_like_trigger_discard_qualifier_tail(
    trigger_prefix_tokens: &[OwnedLexToken],
    tail_tokens: &[OwnedLexToken],
) -> bool {
    if tail_tokens.is_empty() {
        return false;
    }

    let prefix_words = crate::cards::builders::parser::token_word_refs(trigger_prefix_tokens);
    if !(slice_contains(&prefix_words, &"discard") || slice_contains(&prefix_words, &"discards")) {
        return false;
    }

    let tail_words = crate::cards::builders::parser::token_word_refs(tail_tokens);
    if tail_words.is_empty() {
        return false;
    }

    let Some(first_word) = tail_words.first().copied() else {
        return false;
    };
    let typeish = parse_card_type(first_word).is_some()
        || parse_non_type(first_word).is_some()
        || matches!(first_word, "and" | "or");
    if !typeish {
        return false;
    }

    find_index(tail_tokens, |token| token.is_comma()).is_some_and(|comma_idx| {
        let before_words =
            crate::cards::builders::parser::token_word_refs(&tail_tokens[..comma_idx]);
        slice_contains(&before_words, &"card") || slice_contains(&before_words, &"cards")
    })
}

pub(crate) fn looks_like_trigger_type_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    let first_is_card_type = parse_card_type(words[0]).is_some()
        || parse_subtype_word(words[0]).is_some()
        || str_strip_suffix(words[0], "s").is_some_and(|word| {
            parse_card_type(word).is_some() || parse_subtype_word(word).is_some()
        });
    first_is_card_type
        && words.iter().any(|word| matches!(*word, "spell" | "spells"))
        && words.iter().any(|word| *word == "or")
        && tokens.iter().any(|token| token.is_comma())
}

pub(crate) fn looks_like_trigger_color_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    is_basic_color_word(words[0])
        && words.iter().any(|word| *word == "or")
        && tokens.iter().any(|token| token.is_comma())
}

pub(crate) fn looks_like_trigger_numeric_list_tail(tokens: &[OwnedLexToken]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.len() < 3 {
        return false;
    }
    if words[0].parse::<i32>().is_err() {
        return false;
    }
    let has_second_number = words.iter().skip(1).any(|word| word.parse::<i32>().is_ok());
    has_second_number && words.iter().any(|word| *word == "or")
}

pub(crate) fn is_trigger_objectish_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || parse_subtype_word(word).is_some()
        || str_strip_suffix(word, "s").is_some_and(|stem| {
            parse_card_type(stem).is_some() || parse_subtype_word(stem).is_some()
        })
}

pub(crate) fn strip_leading_trigger_intro(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if tokens.first().is_some_and(|token| {
        token.is_word("when") || token.is_word("whenever") || token.is_word("at")
    }) {
        &tokens[1..]
    } else {
        tokens
    }
}

pub(crate) fn split_trigger_or_index(tokens: &[OwnedLexToken]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if !token.is_word("or") {
            return None;
        }
        // Keep quantifiers like "one or more <subject>" intact.
        let quantifier_or = idx > 0
            && tokens.get(idx - 1).is_some_and(|prev| prev.is_word("one"))
            && tokens.get(idx + 1).is_some_and(|next| next.is_word("more"));
        let comparison_or = is_comparison_or_delimiter(tokens, idx);
        let previous_numeric = (0..idx)
            .rev()
            .find_map(|i| tokens[i].as_word())
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let next_numeric = tokens
            .get(idx + 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word.parse::<i32>().is_ok());
        let numeric_list_or = previous_numeric && next_numeric;
        let color_list_or = tokens
            .get(idx - 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| parse_color(word).is_some())
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| parse_color(word).is_some())
            && tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .any(|word| word == "spell" || word == "spells");
        let objectish_word = |word: &str| is_trigger_objectish_word(word);
        let object_list_or = tokens
            .get(idx - 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(objectish_word)
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(objectish_word);
        let and_or_list_or = tokens.get(idx - 1).is_some_and(|prev| prev.is_word("and"))
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
        let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
        let serial_spell_list_or = tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .any(|word| word == "spell" || word == "spells")
            && previous_word
                .is_some_and(|word| parse_color(word).is_some() || objectish_word(word))
            && next_word.is_some_and(|word| parse_color(word).is_some() || objectish_word(word));
        let cast_or_copy_or = tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .any(|word| word == "spell" || word == "spells")
            && previous_word.is_some_and(|word| word == "cast" || word == "casts")
            && next_word.is_some_and(|word| word == "copy" || word == "copies");
        let spell_or_ability_or = tokens
            .get(idx - 1)
            .and_then(OwnedLexToken::as_word)
            .is_some_and(|word| word == "spell" || word == "spells")
            && tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| word == "ability" || word == "abilities");
        if quantifier_or
            || comparison_or
            || numeric_list_or
            || color_list_or
            || object_list_or
            || and_or_list_or
            || serial_spell_list_or
            || cast_or_copy_or
            || spell_or_ability_or
        {
            None
        } else {
            Some(idx)
        }
    })
}

pub(crate) fn has_leading_one_or_more(tokens: &[OwnedLexToken]) -> bool {
    tokens.len() >= 3
        && tokens.first().is_some_and(|token| token.is_word("one"))
        && tokens.get(1).is_some_and(|token| token.is_word("or"))
        && tokens.get(2).is_some_and(|token| token.is_word("more"))
}

pub(crate) fn strip_leading_one_or_more(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    if has_leading_one_or_more(tokens) {
        &tokens[3..]
    } else {
        tokens
    }
}

pub(crate) fn parse_leading_or_more_quantifier(
    tokens: &[OwnedLexToken],
) -> Option<(u32, &[OwnedLexToken])> {
    let (count, used) = parse_number(tokens)?;
    if tokens
        .get(used)
        .is_some_and(|token: &OwnedLexToken| token.is_word("or"))
        && tokens
            .get(used + 1)
            .is_some_and(|token: &OwnedLexToken| token.is_word("more"))
    {
        Some((count, &tokens[used + 2..]))
    } else {
        None
    }
}

pub(crate) fn parse_trigger_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<TriggerSpec, CardTextError> {
    fn parse_not_during_turn_suffix(words: &[&str]) -> Option<PlayerFilter> {
        match words {
            ["a", "card", "if", "it", "isnt", "that", "players", "turn"]
            | ["a", "card", "if", "its", "not", "that", "players", "turn"]
            | ["a", "card", "if", "it", "isnt", "their", "turn"]
            | ["a", "card", "if", "its", "not", "their", "turn"] => {
                Some(PlayerFilter::IteratedPlayer)
            }
            ["a", "card", "if", "it", "isnt", "your", "turn"]
            | ["a", "card", "if", "its", "not", "your", "turn"] => Some(PlayerFilter::You),
            ["a", "card", "if", "it", "isnt", "an", "opponents", "turn"]
            | ["a", "card", "if", "its", "not", "an", "opponents", "turn"]
            | ["a", "card", "if", "it", "isnt", "opponents", "turn"]
            | ["a", "card", "if", "its", "not", "opponents", "turn"] => {
                Some(PlayerFilter::Opponent)
            }
            _ => None,
        }
    }

    fn parse_enters_origin_clause_lexed(words: &[&str]) -> Option<(Zone, Option<PlayerFilter>)> {
        let tail_words = words
            .iter()
            .copied()
            .filter(|word| !is_article(word))
            .collect::<Vec<_>>();
        match tail_words.as_slice() {
            ["from", "your", "graveyard"] => Some((Zone::Graveyard, Some(PlayerFilter::You))),
            ["from", "graveyard"] => Some((Zone::Graveyard, None)),
            ["from", "your", "hand"] => Some((Zone::Hand, Some(PlayerFilter::You))),
            ["from", "hand"] => Some((Zone::Hand, None)),
            ["from", "exile"] => Some((Zone::Exile, None)),
            _ => None,
        }
    }

    fn source_trigger_subject_filter_lexed(subject_words: &[&str]) -> ObjectFilter {
        let mut filter = ObjectFilter::default();
        if subject_words.iter().any(|word| *word == "creature") {
            filter.card_types.push(CardType::Creature);
        } else if subject_words.iter().any(|word| *word == "land") {
            filter.card_types.push(CardType::Land);
        } else if subject_words.iter().any(|word| *word == "artifact") {
            filter.card_types.push(CardType::Artifact);
        } else if subject_words.iter().any(|word| *word == "enchantment") {
            filter.card_types.push(CardType::Enchantment);
        } else if subject_words.iter().any(|word| *word == "planeswalker") {
            filter.card_types.push(CardType::Planeswalker);
        } else if subject_words.iter().any(|word| *word == "battle") {
            filter.card_types.push(CardType::Battle);
        }
        filter
    }

    fn parse_damage_by_dies_trigger_lexed(
        subject_tokens: &[OwnedLexToken],
        other: bool,
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        fn trim_lexed_edge_punctuation(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
            let mut start = 0usize;
            let mut end = tokens.len();
            while start < end
                && matches!(
                    tokens[start].kind,
                    TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
                )
            {
                start += 1;
            }
            while end > start
                && matches!(
                    tokens[end - 1].kind,
                    TokenKind::Comma | TokenKind::Period | TokenKind::Semicolon | TokenKind::Quote
                )
            {
                end -= 1;
            }
            &tokens[start..end]
        }

        fn strip_leading_articles_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
            let view = ActivationRestrictionCompatWords::new(tokens);
            if matches!(view.first(), Some("a" | "an" | "the")) {
                let start = view.token_index_for_word_index(1).unwrap_or(tokens.len());
                &tokens[start..]
            } else {
                tokens
            }
        }

        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if subject_words.len() < 8
            || !slice_ends_with(&subject_words, &["this", "turn"])
            || !contains_word_sequence(&subject_words, &["dealt", "damage", "by"])
        {
            return Ok(None);
        }

        let Some(dealt_word_idx) =
            find_word_sequence_start(&subject_words, &["dealt", "damage", "by"])
        else {
            return Ok(None);
        };

        let victim_end = subject_word_view
            .token_index_for_word_index(dealt_word_idx)
            .unwrap_or(0);
        if victim_end == 0 || victim_end > subject_tokens.len() {
            return Ok(None);
        }

        let victim_tokens = trim_lexed_edge_punctuation(&subject_tokens[..victim_end]);
        let victim_tokens = strip_leading_articles_lexed(victim_tokens);
        if victim_tokens.is_empty() {
            return Ok(None);
        }

        let damager_start_word_idx = dealt_word_idx + 3;
        let this_word_idx = subject_words.len() - 2;
        let damager_start = subject_word_view
            .token_index_for_word_index(damager_start_word_idx)
            .unwrap_or(subject_tokens.len());
        let damager_end = subject_word_view
            .token_index_for_word_index(this_word_idx)
            .unwrap_or(subject_tokens.len());
        if damager_start >= damager_end || damager_end > subject_tokens.len() {
            return Ok(None);
        }

        let damager_tokens =
            trim_lexed_edge_punctuation(&subject_tokens[damager_start..damager_end]);
        let damager_word_view = ActivationRestrictionCompatWords::new(&damager_tokens);
        let damager_words = damager_word_view.to_word_refs();
        let has_named_source_words = !damager_words.is_empty()
            && !matches!(
                damager_words.first().copied(),
                Some("a" | "an" | "the" | "target" | "that" | "this" | "equipped" | "enchanted")
            )
            && !damager_words.iter().any(|word| {
                matches!(
                    *word,
                    "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
                )
            });

        let damager = if damager_words == ["this", "creature"]
            || damager_words == ["this", "permanent"]
            || damager_words == ["this", "source"]
            || damager_words == ["this"]
            || has_named_source_words
        {
            Some(DamageBySpec::ThisCreature)
        } else if damager_words == ["equipped", "creature"] {
            Some(DamageBySpec::EquippedCreature)
        } else if damager_words == ["enchanted", "creature"] {
            Some(DamageBySpec::EnchantedCreature)
        } else {
            None
        };

        let Some(damager) = damager else {
            return Ok(None);
        };

        let victim = parse_object_filter_lexed(&victim_tokens, other).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damaged-by trigger victim filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        Ok(Some(TriggerSpec::DiesCreatureDealtDamageByThisTurn {
            victim,
            damager,
        }))
    }

    fn parse_simple_spell_activity_trigger_lexed(
        tokens: &[OwnedLexToken],
        clause_words: &[&str],
    ) -> Result<Option<TriggerSpec>, CardTextError> {
        if !slice_contains(&clause_words, &"spell") && !slice_contains(&clause_words, &"spells") {
            return Ok(None);
        }
        if slice_contains(&clause_words, &"during")
            || slice_contains(&clause_words, &"turn")
            || slice_contains(&clause_words, &"first")
            || slice_contains(&clause_words, &"second")
            || slice_contains(&clause_words, &"third")
            || slice_contains(&clause_words, &"fourth")
            || slice_contains(&clause_words, &"fifth")
            || slice_contains(&clause_words, &"sixth")
            || slice_contains(&clause_words, &"seventh")
            || slice_contains(&clause_words, &"eighth")
            || slice_contains(&clause_words, &"ninth")
            || slice_contains(&clause_words, &"tenth")
            || contains_word_sequence(&clause_words, &["other", "than"])
            || contains_word_sequence(&clause_words, &["from", "anywhere"])
        {
            return Ok(None);
        }

        let cast_idx = find_index(tokens, |token| {
            token.is_word("cast") || token.is_word("casts")
        });
        let copy_idx = find_index(tokens, |token| {
            token.is_word("copy") || token.is_word("copies")
        });
        if cast_idx.is_none() && copy_idx.is_none() {
            return Ok(None);
        }

        let actor = parse_subject_clause_player_filter(clause_words);
        let parse_filter =
            |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
                let filter_words = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_words.to_word_refs();
                let is_unqualified_spell = filter_words.as_slice() == ["a", "spell"]
                    || filter_words.as_slice() == ["spell"]
                    || filter_words.as_slice() == ["spells"];
                if filter_tokens.is_empty() || is_unqualified_spell {
                    return Ok(None);
                }
                parse_object_filter_lexed(filter_tokens, false)
                    .map(Some)
                    .map_err(|err| {
                        CardTextError::ParseError(format!(
                            "unsupported spell trigger filter (clause: '{}') [{err:?}]",
                            filter_words.join(" ")
                        ))
                    })
            };

        if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
            let (first, second, first_is_cast) = if cast < copy {
                (cast, copy, true)
            } else {
                (copy, cast, false)
            };
            let between_view = ActivationRestrictionCompatWords::new(&tokens[first + 1..second]);
            let between_words = between_view.to_word_refs();
            if between_words.as_slice() == ["or"] {
                let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
                let cast_trigger = TriggerSpec::SpellCast {
                    filter: filter.clone(),
                    caster: actor.clone(),
                    during_turn: None,
                    min_spells_this_turn: None,
                    exact_spells_this_turn: None,
                    from_not_hand: false,
                };
                let copied_trigger = TriggerSpec::SpellCopied {
                    filter,
                    copier: actor,
                };
                return Ok(Some(if first_is_cast {
                    TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
                } else {
                    TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
                }));
            }
        }

        if let Some(cast) = cast_idx {
            let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
            if filter_tokens.is_empty() {
                let mut prefix_tokens = &tokens[..cast];
                while let Some(last_word) = prefix_tokens.last().and_then(OwnedLexToken::as_word) {
                    if matches!(last_word, "is" | "are" | "was" | "were" | "be" | "been") {
                        prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                    } else {
                        break;
                    }
                }
                let has_spell_noun = prefix_tokens
                    .iter()
                    .any(|token| token.is_word("spell") || token.is_word("spells"));
                if has_spell_noun {
                    filter_tokens = prefix_tokens;
                }
            }
            let filter = parse_filter(filter_tokens)?;
            return Ok(Some(TriggerSpec::SpellCast {
                filter,
                caster: actor,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }));
        }

        if let Some(copy) = copy_idx {
            let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
            return Ok(Some(TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            }));
        }

        Ok(None)
    }

    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return Err(CardTextError::ParseError(
            "empty trigger clause".to_string(),
        ));
    }

    if let Some(enters_idx) = find_index(tokens, |token| {
        token.is_word("enters") || token.is_word("enter")
    }) {
        let tail = &tokens[enters_idx + 1..];
        let shared_subject_or_combat_damage = tail.len() >= 6
            && tail[0].is_word("the")
            && tail[1].is_word("battlefield")
            && tail[2].is_word("or")
            && (tail[3].is_word("deal") || tail[3].is_word("deals"))
            && tail[4].is_word("combat")
            && tail[5].is_word("damage");
        if shared_subject_or_combat_damage {
            let or_idx = enters_idx + 3;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.extend_from_slice(&tokens[or_idx + 1..]);

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
        let shared_subject_or_attack = (tail.len() == 2
            && tail[0].is_word("or")
            && (tail[1].is_word("attack") || tail[1].is_word("attacks")))
            || (tail.len() == 4
                && tail[0].is_word("the")
                && tail[1].is_word("battlefield")
                && tail[2].is_word("or")
                && (tail[3].is_word("attack") || tail[3].is_word("attacks")));
        if shared_subject_or_attack {
            let or_idx = if tail[0].is_word("or") {
                enters_idx + 1
            } else {
                enters_idx + 3
            };
            let attack_idx = or_idx + 1;
            let left_tokens = &tokens[..or_idx];
            let mut right_tokens = tokens[..enters_idx].to_vec();
            right_tokens.push(tokens[attack_idx].clone());

            if !left_tokens.is_empty()
                && let (Ok(left), Ok(right)) = (
                    parse_trigger_clause_lexed(left_tokens),
                    parse_trigger_clause_lexed(&right_tokens),
                )
            {
                return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
            }
        }
    }

    if let Some(or_idx) = split_trigger_or_index(tokens) {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    if let Some(and_idx) = find_index(tokens, |token| token.is_word("and"))
        && tokens.get(and_idx + 1).is_some_and(|token| {
            token.is_word("whenever") || token.is_word("when") || token.is_word("at")
        })
    {
        let left_tokens = strip_leading_trigger_intro(&tokens[..and_idx]);
        let right_tokens = strip_leading_trigger_intro(&tokens[and_idx + 1..]);
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause_lexed(left_tokens),
                parse_trigger_clause_lexed(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }

    if words.len() >= 2
        && words.last().copied() == Some("alone")
        && matches!(
            words.get(words.len() - 2).copied(),
            Some("attack" | "attacks")
        )
    {
        let attacks_word_idx = words.len().saturating_sub(2);
        let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(attacks_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksAlone(filter),
                None => TriggerSpec::AttacksAlone(ObjectFilter::source()),
            },
        );
    }

    if let Some(attacks_word_idx) =
        find_index(&words, |word| *word == "attack" || *word == "attacks")
    {
        let tail_words = &words[attacks_word_idx + 1..];
        if tail_words == ["you", "or", "a", "planeswalker", "you", "control"]
            || tail_words == ["you", "or", "planeswalker", "you", "control"]
        {
            let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attacks_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            let subject_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?
                .unwrap_or_else(ObjectFilter::source);
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            return Ok(if player_subject {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControlOneOrMore(subject_filter)
            } else {
                TriggerSpec::AttacksYouOrPlaneswalkerYouControl(subject_filter)
            });
        }
    }

    if words.len() >= 3
        && matches!(
            words.get(words.len() - 3).copied(),
            Some("attack" | "attacks")
        )
        && words.get(words.len() - 2).copied() == Some("while")
        && words.last().copied() == Some("saddled")
    {
        let attacks_word_idx = words.len().saturating_sub(3);
        let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(attacks_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..attacks_token_idx];
        return Ok(
            match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::AttacksWhileSaddled(filter),
                None => TriggerSpec::ThisAttacksWhileSaddled,
            },
        );
    }

    let is_you_cast_this_spell = contains_any_word_sequence(
        &words,
        &[&["cast", "this", "spell"], &["casts", "this", "spell"]],
    );
    if is_you_cast_this_spell && slice_contains(&words, &"you") {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if let Some(spell_activity_trigger) = parse_simple_spell_activity_trigger_lexed(tokens, &words)?
    {
        return Ok(spell_activity_trigger);
    }
    if let Some(spell_activity_trigger) = parse_spell_activity_trigger(tokens)? {
        return Ok(spell_activity_trigger);
    }

    if let Some(play_idx) = find_index(tokens, |token| {
        token.is_word("play") || token.is_word("plays")
    }) {
        let subject_tokens = &tokens[..play_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let trimmed_object_tokens = trim_commas(&tokens[play_idx + 1..]);
            let object_tokens = strip_leading_articles(&trimmed_object_tokens);
            let object_word_view = ActivationRestrictionCompatWords::new(&object_tokens);
            let object_words = object_word_view.to_word_refs();
            if object_words
                .iter()
                .any(|word| matches!(*word, "land" | "lands"))
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerPlaysLand { player, filter });
            }
        }
    }

    if let Some(search_idx) = find_index(tokens, |token| {
        token.is_word("search") || token.is_word("searches")
    }) {
        let subject_tokens = &tokens[..search_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let searched_tokens = trim_commas(&tokens[search_idx + 1..]);
            let searched_word_view = ActivationRestrictionCompatWords::new(&searched_tokens);
            let searched_words = searched_word_view.to_word_refs();
            if slice_starts_with(&searched_words, &["their", "library"])
                || slice_starts_with(&searched_words, &["your", "library"])
                || slice_starts_with(&searched_words, &["a", "library"])
            {
                return Ok(TriggerSpec::PlayerSearchesLibrary(player));
            }
        }
    }

    if let Some(shuffle_idx) = find_index(tokens, |token| {
        token.is_word("shuffle") || token.is_word("shuffles")
    }) {
        let subject_tokens = &tokens[..shuffle_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let shuffled_tokens = trim_commas(&tokens[shuffle_idx + 1..]);
        let shuffled_word_view = ActivationRestrictionCompatWords::new(&shuffled_tokens);
        let shuffled_words = shuffled_word_view.to_word_refs();
        if slice_starts_with(&shuffled_words, &["their", "library"])
            || slice_starts_with(&shuffled_words, &["your", "library"])
            || slice_starts_with(&shuffled_words, &["a", "library"])
            || slice_starts_with(&shuffled_words, &["that", "players", "library"])
        {
            if let Some((player, caused_by_effect, source_controller_shuffles)) =
                parse_shuffle_trigger_subject(&subject_words)
            {
                return Ok(TriggerSpec::PlayerShufflesLibrary {
                    player,
                    caused_by_effect,
                    source_controller_shuffles,
                });
            }
        }
    }

    if let Some(give_idx) = find_index(tokens, |token| {
        token.is_word("give") || token.is_word("gives")
    }) {
        let subject_tokens = &tokens[..give_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let gifted_tokens = trim_commas(&tokens[give_idx + 1..]);
            let gifted_word_view = ActivationRestrictionCompatWords::new(&gifted_tokens);
            let gifted_words = gifted_word_view.to_word_refs();
            if gifted_words == ["a", "gift"] || gifted_words == ["gift"] {
                return Ok(TriggerSpec::PlayerGivesGift(player));
            }
        }
    }

    if let Some(tap_idx) = find_index(tokens, |token| {
        token.is_word("tap") || token.is_word("taps")
    }) {
        let subject_tokens = &tokens[..tap_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&subject_words) {
            let after_tap = &tokens[tap_idx + 1..];
            if let Some(for_idx) = find_index(after_tap, |token| token.is_word("for"))
                && for_idx > 0
            {
                let object_tokens = trim_commas(&after_tap[..for_idx]);
                let object_tokens = strip_leading_articles(&object_tokens);
                if !object_tokens.is_empty()
                    && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
                {
                    return Ok(TriggerSpec::PlayerTapsForMana { player, filter });
                }
            }
        }
    }

    if let Some(tapped_idx) = find_index(tokens, |token| token.is_word("tapped"))
        && tapped_idx >= 2
        && tokens
            .get(tapped_idx.wrapping_sub(1))
            .is_some_and(|token| token.is_word("is") || token.is_word("are"))
    {
        let subject_tokens = &tokens[..tapped_idx - 1];
        let after_tapped = &tokens[tapped_idx + 1..];
        if after_tapped.iter().any(|token| token.is_word("for")) {
            let object_tokens = trim_commas(subject_tokens);
            let object_tokens = strip_leading_articles(&object_tokens);
            if !object_tokens.is_empty()
                && let Ok(filter) = parse_object_filter_lexed(&object_tokens, false)
            {
                return Ok(TriggerSpec::PlayerTapsForMana {
                    player: PlayerFilter::Any,
                    filter,
                });
            }
        }
    }

    if let Some(activate_idx) =
        find_index(&words, |word| *word == "activate" || *word == "activates")
    {
        let subject_tokens = &tokens[..activate_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if let Some(activator) = parse_trigger_subject_player_filter(&subject_words) {
            let tail_words = &words[activate_idx + 1..];
            if tail_words == ["an", "ability"]
                || tail_words == ["abilities"]
                || tail_words == ["an", "ability", "that", "isnt", "a", "mana", "ability"]
                || tail_words == ["an", "ability", "that", "isn't", "a", "mana", "ability"]
                || tail_words == ["abilities", "that", "arent", "mana", "abilities"]
                || tail_words == ["abilities", "that", "aren't", "mana", "abilities"]
            {
                return Ok(TriggerSpec::AbilityActivated {
                    activator,
                    filter: ObjectFilter::default(),
                    non_mana_only: slice_contains(&tail_words, &"mana"),
                });
            }
        }
    }

    let has_deal = words.iter().any(|word| *word == "deal" || *word == "deals");
    if has_deal && slice_contains(&words, &"combat") && slice_contains(&words, &"damage") {
        if let Some(deals_idx) = find_index(tokens, |token| {
            token.is_word("deal") || token.is_word("deals")
        }) {
            let subject_tokens = &tokens[..deals_idx];
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let one_or_more = has_leading_one_or_more(subject_tokens) || player_subject;
            let source_filter = parse_attack_trigger_subject_filter_lexed(subject_tokens)?;
            if let Some(damage_idx_rel) =
                find_index(&tokens[deals_idx + 1..], |token| token.is_word("damage"))
            {
                let damage_idx = deals_idx + 1 + damage_idx_rel;
                if let Some(to_idx_rel) =
                    find_index(&tokens[damage_idx + 1..], |token| token.is_word("to"))
                {
                    let to_idx = damage_idx + 1 + to_idx_rel;
                    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
                    if target_tokens.is_empty() {
                        return Err(CardTextError::ParseError(format!(
                            "missing combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        )));
                    }
                    let target_word_view = ActivationRestrictionCompatWords::new(&target_tokens);
                    let target_words = target_word_view.to_word_refs();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(match source_filter {
                            Some(source) => {
                                if one_or_more {
                                    TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                                        source,
                                        player,
                                    }
                                } else {
                                    TriggerSpec::DealsCombatDamageToPlayer { source, player }
                                }
                            }
                            None => TriggerSpec::ThisDealsCombatDamageToPlayer,
                        });
                    }

                    let target_tokens = strip_leading_one_or_more_lexed(&target_tokens);
                    let target_filter = parse_object_filter_lexed(target_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported combat damage recipient filter in trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                    return Ok(match source_filter {
                        Some(source) => TriggerSpec::DealsCombatDamageTo {
                            source,
                            target: target_filter,
                        },
                        None => TriggerSpec::ThisDealsCombatDamageTo(target_filter),
                    });
                }
            }

            return Ok(match source_filter {
                Some(filter) => TriggerSpec::DealsCombatDamage(filter),
                None => TriggerSpec::ThisDealsCombatDamage,
            });
        }
        return Ok(TriggerSpec::ThisDealsCombatDamage);
    }

    if words.as_slice() == ["this", "leaves", "the", "battlefield"]
        || (words.len() == 5
            && words.first().copied() == Some("this")
            && words.get(2).copied() == Some("leaves")
            && words.get(3).copied() == Some("the")
            && words.get(4).copied() == Some("battlefield"))
    {
        return Ok(TriggerSpec::ThisLeavesBattlefield);
    }

    if let Some(dies_word_idx) = find_index(&words, |word| *word == "dies") {
        let dies_token_idx = word_view
            .token_index_for_word_index(dies_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..dies_token_idx];
        let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        if is_source_reference_words(&subject_words)
            && words.get(dies_word_idx + 1..)
                == Some(
                    &[
                        "or",
                        "is",
                        "put",
                        "into",
                        "exile",
                        "from",
                        "the",
                        "battlefield",
                    ][..],
                )
        {
            return Ok(TriggerSpec::ThisDiesOrIsExiled);
        }
    }

    if let Some(enters_word_idx) = find_index(&words, |word| *word == "enters" || *word == "enter")
    {
        let enters_token_idx = word_view
            .token_index_for_word_index(enters_word_idx)
            .unwrap_or(tokens.len());
        if slice_ends_with(&words, &["enters", "or", "leaves", "the", "battlefield"])
            || slice_ends_with(&words, &["enter", "or", "leave", "the", "battlefield"])
        {
            let subject_tokens = &tokens[..enters_token_idx];
            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("this"))
            {
                return Ok(TriggerSpec::Either(
                    Box::new(TriggerSpec::ThisEntersBattlefield),
                    Box::new(TriggerSpec::ThisLeavesBattlefield),
                ));
            }
        }

        let enters_origin = parse_enters_origin_clause_lexed(&words[enters_word_idx + 1..]);
        if enters_word_idx == 0 {
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: ObjectFilter::default(),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield
            });
        }

        let subject_tokens = &tokens[..enters_token_idx];
        if let Some(or_idx) =
            find_index(subject_tokens, |token: &OwnedLexToken| token.is_word("or"))
        {
            let left_tokens = &subject_tokens[..or_idx];
            let mut right_tokens = &subject_tokens[or_idx + 1..];
            let left_word_view = ActivationRestrictionCompatWords::new(left_tokens);
            let left_words: Vec<&str> = left_word_view
                .to_word_refs()
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
            if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                let mut other = false;
                if right_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"))
                {
                    other = true;
                    right_tokens = &right_tokens[1..];
                }
                let parsed_filter =
                    parse_object_filter_lexed(right_tokens, other)
                        .ok()
                        .or_else(|| {
                            parse_subtype_list_enters_trigger_filter_lexed(right_tokens, other)
                        });
                if let Some(mut filter) = parsed_filter {
                    if slice_contains(&words, &"under")
                        && slice_contains(&words, &"your")
                        && slice_contains(&words, &"control")
                    {
                        filter.controller = Some(PlayerFilter::You);
                    } else if slice_contains(&words, &"under")
                        && (slice_contains(&words, &"opponent")
                            || slice_contains(&words, &"opponents"))
                        && slice_contains(&words, &"control")
                    {
                        filter.controller = Some(PlayerFilter::Opponent);
                    }
                    let right_trigger = if slice_contains(&words, &"untapped") {
                        TriggerSpec::EntersBattlefieldUntapped(filter)
                    } else if slice_contains(&words, &"tapped") {
                        TriggerSpec::EntersBattlefieldTapped(filter)
                    } else {
                        TriggerSpec::EntersBattlefield(filter)
                    };
                    return Ok(TriggerSpec::Either(
                        Box::new(TriggerSpec::ThisEntersBattlefield),
                        Box::new(right_trigger),
                    ));
                }
            }
        }
        if subject_tokens
            .first()
            .is_some_and(|token| token.is_word("this"))
        {
            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            return Ok(if let Some((from, owner)) = enters_origin.clone() {
                TriggerSpec::ThisEntersBattlefieldFromZone {
                    subject_filter: source_trigger_subject_filter_lexed(&subject_words),
                    from,
                    owner,
                }
            } else {
                TriggerSpec::ThisEntersBattlefield
            });
        }

        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if filtered_subject_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let one_or_more = ActivationRestrictionCompatWords::new(filtered_subject_tokens)
            .slice_eq(0, &["one", "or", "more"]);
        filtered_subject_tokens = strip_leading_one_or_more_lexed(filtered_subject_tokens);
        if filtered_subject_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        let parsed_filter = parse_object_filter_lexed(filtered_subject_tokens, other)
            .ok()
            .or_else(|| {
                parse_subtype_list_enters_trigger_filter_lexed(filtered_subject_tokens, other)
            });
        if let Some(mut filter) = parsed_filter {
            if slice_contains(&words, &"under")
                && slice_contains(&words, &"your")
                && slice_contains(&words, &"control")
            {
                filter.controller = Some(PlayerFilter::You);
            } else if slice_contains(&words, &"under")
                && (slice_contains(&words, &"opponent") || slice_contains(&words, &"opponents"))
                && slice_contains(&words, &"control")
            {
                filter.controller = Some(PlayerFilter::Opponent);
            }
            if slice_contains(&words, &"untapped") {
                return Ok(TriggerSpec::EntersBattlefieldUntapped(filter));
            }
            if slice_contains(&words, &"tapped") {
                return Ok(TriggerSpec::EntersBattlefieldTapped(filter));
            }
            return Ok(if let Some((from, owner)) = enters_origin {
                TriggerSpec::EntersBattlefieldFromZone {
                    filter,
                    from,
                    owner,
                    one_or_more,
                }
            } else if one_or_more {
                TriggerSpec::EntersBattlefieldOneOrMore(filter)
            } else {
                TriggerSpec::EntersBattlefield(filter)
            });
        }
    }

    for tail in [
        ["is", "put", "into", "your", "graveyard", "from", "anywhere"].as_slice(),
        [
            "are",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "anywhere",
        ]
        .as_slice(),
        ["is", "put", "into", "your", "graveyard"].as_slice(),
        ["are", "put", "into", "your", "graveyard"].as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.controller = None;
            if filter.owner.is_none() {
                filter.owner = Some(PlayerFilter::You);
            }
            if subject_words
                .iter()
                .any(|word| matches!(*word, "card" | "cards"))
            {
                filter.nontoken = true;
            }
            return Ok(TriggerSpec::PutIntoGraveyard(filter));
        }
    }

    for tail in [
        ["is", "put", "into", "a", "graveyard", "from", "anywhere"].as_slice(),
        ["are", "put", "into", "a", "graveyard", "from", "anywhere"].as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::PutIntoGraveyard(ObjectFilter::source()));
            }
            if let Ok(filter) = parse_object_filter_lexed(subject_tokens, false) {
                return Ok(TriggerSpec::PutIntoGraveyard(filter));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported filter in put-into-graveyard-from-anywhere trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
    }

    for tail in [
        [
            "is",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
        [
            "are",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                    filter: ObjectFilter::source(),
                    from: Zone::Battlefield,
                });
            }
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported card filter in put-into-your-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.controller = None;
            if filter.owner.is_none() {
                filter.owner = Some(PlayerFilter::You);
            }
            if subject_words
                .iter()
                .any(|word| matches!(*word, "card" | "cards"))
            {
                filter.nontoken = true;
            }
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
            });
        }
    }

    for tail in [
        [
            "is",
            "put",
            "into",
            "an",
            "opponents",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
        [
            "are",
            "put",
            "into",
            "an",
            "opponents",
            "graveyard",
            "from",
            "the",
            "battlefield",
        ]
        .as_slice(),
    ] {
        if slice_ends_with(&words, tail) {
            let subject_word_len = words.len().saturating_sub(tail.len());
            let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(subject_word_len)
                .map(|idx| &tokens[..idx])
                .unwrap_or_default();
            let subject_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_view.to_word_refs();
            if is_source_reference_words(&subject_words) {
                let mut filter = ObjectFilter::source();
                filter.owner = Some(PlayerFilter::Opponent);
                return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                    filter,
                    from: Zone::Battlefield,
                });
            }
            let mut filter = parse_object_filter_lexed(subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported filter in put-into-opponents-graveyard-from-battlefield trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            filter.zone = None;
            filter.controller = None;
            filter.owner = Some(PlayerFilter::Opponent);
            return Ok(TriggerSpec::PutIntoGraveyardFromZone {
                filter,
                from: Zone::Battlefield,
            });
        }
    }

    if let Some(put_word_idx) = find_index(&words, |word| *word == "put" || *word == "puts")
        && let Some(source_controller) = parse_trigger_subject_player_filter(&words[..put_word_idx])
        && let Some(counter_word_idx) =
            find_index(&words, |word| *word == "counter" || *word == "counters")
        && counter_word_idx > put_word_idx
        && matches!(
            words.get(counter_word_idx + 1).copied(),
            Some("on") | Some("onto")
        )
    {
        let word_view = ActivationRestrictionCompatWords::new(tokens);
        let descriptor_word_start = put_word_idx + 1;
        let descriptor_token_start = word_view
            .token_index_for_word_index(descriptor_word_start)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter descriptor in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let descriptor_token_end = word_view
            .token_index_for_word_index(counter_word_idx)
            .unwrap_or(tokens.len());
        let descriptor_span = &tokens[descriptor_token_start..descriptor_token_end];
        let one_or_more = ActivationRestrictionCompatWords::new(descriptor_span)
            .slice_eq(0, &["one", "or", "more"]);
        let counter_descriptor_tokens = &tokens[descriptor_token_start..(descriptor_token_end + 1)];
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 2;
        let object_token_start = word_view
            .token_index_for_word_index(object_word_start)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter recipient in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let mut object_tokens = trim_commas(&tokens[object_token_start..]);
        let object_view = ActivationRestrictionCompatWords::new(&object_tokens);
        if matches!(object_view.first(), Some("a" | "an" | "the")) {
            let start = object_view
                .token_index_for_word_index(1)
                .unwrap_or(object_tokens.len());
            object_tokens = object_tokens[start..].to_vec();
        }
        if object_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter recipient in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let filter = parse_object_filter_lexed(&object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: Some(source_controller),
            one_or_more,
        });
    }

    if words.as_slice() == ["players", "finish", "voting"]
        || words.as_slice() == ["players", "finished", "voting"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if words.as_slice() == ["you", "cycle", "this", "card"]
        || words.as_slice() == ["you", "cycled", "this", "card"]
    {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::Cycle,
            player: PlayerFilter::You,
        });
    }

    if words.as_slice() == ["you", "cycle", "or", "discard", "a", "card"]
        || words.as_slice() == ["you", "cycle", "or", "discard", "card"]
    {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Cycle,
                player: PlayerFilter::You,
                source_filter: None,
            }),
            Box::new(TriggerSpec::PlayerDiscardsCard {
                player: PlayerFilter::You,
                filter: None,
                cause_controller: None,
                effect_like_only: false,
            }),
        ));
    }

    if words.as_slice() == ["you", "commit", "a", "crime"] {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if words.as_slice() == ["an", "opponent", "commits", "a", "crime"]
        || words.as_slice() == ["opponent", "commits", "a", "crime"]
        || words.as_slice() == ["opponents", "commit", "a", "crime"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Opponent,
            source_filter: None,
        });
    }

    if words.as_slice() == ["a", "player", "commits", "a", "crime"]
        || words.as_slice() == ["a", "player", "commit", "a", "crime"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CommitCrime,
            player: PlayerFilter::Any,
            source_filter: None,
        });
    }

    if words.as_slice() == ["you", "unlock", "this", "door"]
        || words.as_slice() == ["you", "unlocked", "this", "door"]
    {
        return Ok(TriggerSpec::KeywordActionFromSource {
            action: crate::events::KeywordActionKind::UnlockDoor,
            player: PlayerFilter::You,
        });
    }

    if words.len() == 3
        && words[0] == "you"
        && words[1] == "expend"
        && let Some(amount) = parse_cardinal_u32(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::You,
            amount,
        });
    }

    if words.len() == 4
        && (words.as_slice()[..3] == ["an", "opponent", "expends"]
            || words.as_slice()[..3] == ["an", "opponent", "expend"])
        && let Some(amount) = parse_cardinal_u32(words[3])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.len() == 3
        && (words.as_slice()[..2] == ["opponent", "expends"]
            || words.as_slice()[..2] == ["opponent", "expend"])
        && let Some(amount) = parse_cardinal_u32(words[2])
    {
        return Ok(TriggerSpec::Expend {
            player: PlayerFilter::Opponent,
            amount,
        });
    }

    if words.as_slice() == ["the", "ring", "tempts", "you"] {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::RingTemptsYou,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if let Some(cycle_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Cycle)
        )
    }) {
        let subject_words = &words[..cycle_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let tail_words = &words[cycle_word_idx + 1..];
            if tail_words == ["a", "card"] || tail_words == ["card"] {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Cycle,
                    player,
                    source_filter: None,
                });
            }
        }
    }

    if let Some(exert_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Exert)
        )
    }) {
        let subject = &words[..exert_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[exert_word_idx + 1..];
            if tail == ["a", "creature"] || tail == ["creature"] {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::Exert,
                    player,
                    source_filter: Some(ObjectFilter::creature()),
                });
            }
        }
    }

    if let Some(explore_word_idx) = find_index(&words, |word| {
        matches!(
            crate::events::KeywordActionKind::from_trigger_word(word),
            Some(crate::events::KeywordActionKind::Explore)
        )
    }) {
        let subject_tokens = &tokens[..explore_word_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)?
            && words[explore_word_idx + 1..].is_empty()
        {
            return Ok(TriggerSpec::KeywordAction {
                action: crate::events::KeywordActionKind::Explore,
                player: PlayerFilter::Any,
                source_filter: Some(filter),
            });
        }
    }

    if let Some(put_word_idx) = find_index(&words, |word| *word == "put" || *word == "puts") {
        let subject = &words[..put_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[put_word_idx + 1..];
            let has_name_sticker = contains_word_sequence(tail, &["name", "sticker"]);
            let has_on = slice_contains(&tail, &"on");
            if has_name_sticker && has_on {
                return Ok(TriggerSpec::KeywordAction {
                    action: crate::events::KeywordActionKind::NameSticker,
                    player,
                    source_filter: None,
                });
            }
        }
    }

    if slice_ends_with(&words, &["becomes", "tapped"])
        && let Some(becomes_idx) = find_index(tokens, |token| token.is_word("becomes"))
        && tokens
            .get(becomes_idx + 1)
            .is_some_and(|token| token.is_word("tapped"))
    {
        let subject_tokens = &tokens[..becomes_idx];
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::PermanentBecomesTapped(filter),
            None => TriggerSpec::ThisBecomesTapped,
        });
    }

    if words.as_slice() == ["this", "creature", "becomes", "tapped"]
        || words.as_slice() == ["this", "becomes", "tapped"]
        || words.as_slice() == ["becomes", "tapped"]
    {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if words.as_slice() == ["this", "creature", "becomes", "untapped"]
        || words.as_slice() == ["this", "becomes", "untapped"]
        || words.as_slice() == ["becomes", "untapped"]
    {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if words.as_slice() == ["this", "creature", "becomes", "monstrous"]
        || words.as_slice() == ["this", "permanent", "becomes", "monstrous"]
        || words.as_slice() == ["this", "becomes", "monstrous"]
        || words.as_slice() == ["becomes", "monstrous"]
    {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if words.as_slice() == ["this", "creature", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "permanent", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "is", "turned", "face", "up"]
    {
        return Ok(TriggerSpec::ThisTurnedFaceUp);
    }

    if slice_ends_with(&words, &["is", "turned", "face", "up"])
        || slice_ends_with(&words, &["are", "turned", "face", "up"])
    {
        let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(words.len().saturating_sub(4))
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::TurnedFaceUp(filter),
            None => TriggerSpec::ThisTurnedFaceUp,
        });
    }

    if let Some(becomes_idx) = find_index(&words, |word| *word == "becomes")
        && words.get(becomes_idx + 1).copied() == Some("the")
        && words.get(becomes_idx + 2).copied() == Some("target")
        && words.get(becomes_idx + 3).copied() == Some("of")
    {
        let subject_words = &words[..becomes_idx];
        let subject_tokens = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(becomes_idx)
            .map(|idx| &tokens[..idx])
            .unwrap_or_default();
        let subject_filter = parse_trigger_subject_filter_lexed(subject_tokens)?;
        let subject_is_source =
            subject_words.is_empty() || is_source_reference_words(subject_words);
        if subject_is_source {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words) {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: ObjectFilter::source(),
                    source_controller,
                });
            }
            if tail_words == ["a", "spell", "or", "ability"]
                || tail_words == ["spell", "or", "ability"]
            {
                return Ok(TriggerSpec::ThisBecomesTargeted);
            }
            if tail_words
                .last()
                .is_some_and(|word| *word == "spell" || *word == "spells")
            {
                let tail_token_start = ActivationRestrictionCompatWords::new(tokens)
                    .token_index_for_word_index(tail_word_start)
                    .unwrap_or(tokens.len());
                let spell_filter_tokens = trim_commas(&tokens[tail_token_start..]);
                let spell_filter =
                    parse_object_filter_lexed(&spell_filter_tokens, false).map_err(|_| {
                        CardTextError::ParseError(format!(
                            "unsupported spell filter in becomes-targeted trigger clause (clause: '{}')",
                            words.join(" ")
                        ))
                    })?;
                return Ok(TriggerSpec::ThisBecomesTargetedBySpell(spell_filter));
            }
        } else {
            let tail_word_start = becomes_idx + 4;
            let tail_words = &words[tail_word_start..];
            if let Some(source_controller) = parse_spell_or_ability_controller_tail(tail_words)
                && let Some(filter) = subject_filter.clone()
            {
                return Ok(TriggerSpec::BecomesTargetedBySourceController {
                    target: filter,
                    source_controller,
                });
            }
            if (tail_words == ["a", "spell", "or", "ability"]
                || tail_words == ["spell", "or", "ability"])
                && let Some(filter) = subject_filter
            {
                return Ok(TriggerSpec::BecomesTargeted(filter));
            }
        }
    }

    if slice_ends_with(&words, &["is", "dealt", "damage"])
        && words.len() >= 4
        && !slice_starts_with(&words, &["this", "creature", "is", "dealt", "damage"])
        && !slice_starts_with(&words, &["this", "is", "dealt", "damage"])
    {
        let is_word_idx = words.len().saturating_sub(3);
        let is_token_idx = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(is_word_idx)
            .unwrap_or(tokens.len());
        let subject_tokens = &tokens[..is_token_idx];
        if let Some(filter) = parse_trigger_subject_filter_lexed(subject_tokens)? {
            return Ok(TriggerSpec::IsDealtDamage(filter));
        }
    }

    if slice_starts_with(&words, &["this", "creature", "is", "dealt", "damage"])
        || slice_starts_with(&words, &["this", "is", "dealt", "damage"])
    {
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if (slice_starts_with(&words, &["this", "creature", "deals"])
        || slice_starts_with(&words, &["this", "permanent", "deals"])
        || slice_starts_with(&words, &["this", "deals"]))
        && let Some(deals_idx) = find_index(tokens, |token| {
            token.is_word("deal") || token.is_word("deals")
        })
        && let Some(damage_idx_rel) =
            find_index(&tokens[deals_idx + 1..], |token| token.is_word("damage"))
    {
        let damage_idx = deals_idx + 1 + damage_idx_rel;
        if let Some(to_idx_rel) = find_index(&tokens[damage_idx + 1..], |token| token.is_word("to"))
        {
            let to_idx = damage_idx + 1 + to_idx_rel;
            let amount_tokens = trim_commas(&tokens[deals_idx + 1..damage_idx]);
            if !amount_tokens
                .first()
                .is_some_and(|token| token.is_word("combat"))
            {
                let amount_view = ActivationRestrictionCompatWords::new(&amount_tokens);
                let amount_words = amount_view.to_word_refs();
                if let Some((amount, _)) =
                    parse_filter_comparison_tokens("damage amount", &amount_words, &words)?
                {
                    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
                    let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
                    let target_words = target_view.to_word_refs();
                    if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
                        return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                            player,
                            amount: Some(amount),
                        });
                    }
                }
            }
        }
    }

    if (slice_starts_with(&words, &["this", "creature", "deals", "damage", "to"])
        || slice_starts_with(&words, &["this", "permanent", "deals", "damage", "to"])
        || slice_starts_with(&words, &["this", "deals", "damage", "to"]))
        && let Some(to_idx) = find_index(tokens, |token| token.is_word("to"))
    {
        let target_tokens = trim_commas(&tokens[to_idx + 1..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let target_view = ActivationRestrictionCompatWords::new(&target_tokens);
        let target_words = target_view.to_word_refs();
        if let Some(player) = parse_trigger_subject_player_filter(&target_words) {
            return Ok(TriggerSpec::ThisDealsDamageToPlayer {
                player,
                amount: None,
            });
        }
        let target_filter = parse_object_filter_lexed(&target_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::ThisDealsDamageTo(target_filter));
    }

    if slice_starts_with(&words, &["this", "creature", "deals", "damage"])
        || slice_starts_with(&words, &["this", "permanent", "deals", "damage"])
        || slice_starts_with(&words, &["this", "deals", "damage"])
    {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    if has_deal
        && slice_contains(&words, &"damage")
        && let Some(deals_idx) = find_index(tokens, |token| {
            token.is_word("deal") || token.is_word("deals")
        })
    {
        let subject_tokens = &tokens[..deals_idx];
        return Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
            Some(filter) => TriggerSpec::DealsDamage(filter),
            None => TriggerSpec::ThisDealsDamage,
        });
    }

    if words.as_slice() == ["you", "gain", "life"] {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.len() >= 6
        && slice_ends_with(&words, &["during", "your", "turn"])
        && words[..words.len() - 3] == ["you", "gain", "life"]
    {
        return Ok(TriggerSpec::YouGainLifeDuringTurn(PlayerFilter::You));
    }

    if slice_ends_with(&words, &["lose", "life"]) || slice_ends_with(&words, &["loses", "life"]) {
        let subject = &words[..words.len().saturating_sub(2)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLife(player));
        }
    }

    if words.len() >= 5
        && slice_ends_with(&words, &["during", "your", "turn"])
        && (slice_ends_with(&words[..words.len() - 3], &["lose", "life"])
            || slice_ends_with(&words[..words.len() - 3], &["loses", "life"]))
    {
        let subject = &words[..words.len() - 5];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerLosesLifeDuringTurn {
                player,
                during_turn: PlayerFilter::You,
            });
        }
    }

    if let Some(draw_word_idx) = find_index(&words, |word| *word == "draw" || *word == "draws") {
        let subject = &words[..draw_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            let tail = &words[draw_word_idx + 1..];
            if let Some(during_turn) = parse_not_during_turn_suffix(tail) {
                return Ok(TriggerSpec::PlayerDrawsCardNotDuringTurn {
                    player,
                    during_turn,
                });
            }
            if let Some(card_number) = parse_exact_draw_count_each_turn(tail) {
                return Ok(TriggerSpec::PlayerDrawsNthCardEachTurn {
                    player,
                    card_number,
                });
            }
        }
    }

    if slice_ends_with(&words, &["draw", "a", "card"])
        || slice_ends_with(&words, &["draws", "a", "card"])
    {
        let subject = &words[..words.len().saturating_sub(3)];
        if subject == ["you"] {
            return Ok(TriggerSpec::YouDrawCard);
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerDrawsCard(player));
        }
    }

    if words.as_slice()
        == [
            "a", "spell", "or", "ability", "an", "opponent", "controls", "causes", "you", "to",
            "discard", "this", "card",
        ]
    {
        return Ok(TriggerSpec::PlayerDiscardsCard {
            player: PlayerFilter::You,
            filter: Some(ObjectFilter::source()),
            cause_controller: Some(PlayerFilter::Opponent),
            effect_like_only: true,
        });
    }

    if let Some(discard_word_idx) =
        find_index(&words, |word| *word == "discard" || *word == "discards")
        && let Some(discard_token_idx) = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(discard_word_idx)
    {
        let subject_words = &words[..discard_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            if let Ok(filter) =
                parse_discard_trigger_card_filter(&tokens[discard_token_idx + 1..], &words)
            {
                return Ok(TriggerSpec::PlayerDiscardsCard {
                    player,
                    filter,
                    cause_controller: None,
                    effect_like_only: false,
                });
            }
        }
    }

    if let Some(reveal_word_idx) =
        find_index(&words, |word| *word == "reveal" || *word == "reveals")
        && let Some(player) = parse_trigger_subject_player_filter(&words[..reveal_word_idx])
    {
        let mut tail_tokens = trim_commas(
            &tokens[ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(reveal_word_idx + 1)
                .unwrap_or(tokens.len())..],
        );
        let tail_view = ActivationRestrictionCompatWords::new(&tail_tokens);
        let tail_words = tail_view.to_word_refs();
        let from_source = slice_ends_with(&tail_words, &["this", "way"]);
        if from_source {
            let cutoff = ActivationRestrictionCompatWords::new(&tail_tokens)
                .token_index_for_word_index(tail_words.len().saturating_sub(2))
                .unwrap_or(tail_tokens.len());
            tail_tokens = trim_commas(&tail_tokens[..cutoff]);
        }
        if !tail_tokens.is_empty()
            && let Ok(mut filter) = parse_object_filter_lexed(&tail_tokens, false)
        {
            filter.zone = None;
            return Ok(TriggerSpec::PlayerRevealsCard {
                player,
                filter,
                from_source,
            });
        }
    }

    if let Some(sacrifice_word_idx) =
        find_index(&words, |word| *word == "sacrifice" || *word == "sacrifices")
        && let Some(sacrifice_token_idx) = ActivationRestrictionCompatWords::new(tokens)
            .token_index_for_word_index(sacrifice_word_idx)
    {
        let subject_words = &words[..sacrifice_word_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let mut filter_tokens = &tokens[sacrifice_token_idx + 1..];
            let mut other = false;
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("another") || token.is_word("other"))
            {
                other = true;
                filter_tokens = &filter_tokens[1..];
            }

            let filter = if filter_tokens.is_empty() {
                let mut filter = ObjectFilter::permanent();
                if other {
                    filter.other = true;
                }
                filter
            } else if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("this") || token.is_word("it"))
            {
                let filter_word_view = ActivationRestrictionCompatWords::new(filter_tokens);
                let filter_words = filter_word_view.to_word_refs();
                let mut filter = ObjectFilter::source();
                if slice_contains(&filter_words, &"artifact") {
                    filter = filter.with_type(CardType::Artifact);
                } else if slice_contains(&filter_words, &"creature") {
                    filter = filter.with_type(CardType::Creature);
                } else if slice_contains(&filter_words, &"enchantment") {
                    filter = filter.with_type(CardType::Enchantment);
                } else if slice_contains(&filter_words, &"land") {
                    filter = filter.with_type(CardType::Land);
                } else if slice_contains(&filter_words, &"planeswalker") {
                    filter = filter.with_type(CardType::Planeswalker);
                }
                filter
            } else {
                parse_object_filter_lexed(filter_tokens, other).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported sacrifice trigger filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::PlayerSacrifices { player, filter });
        }
    }

    if let Some(last_word) = words.last().copied()
        && let Some(action) = crate::events::KeywordActionKind::from_trigger_word(last_word)
    {
        let subject = &words[..words.len().saturating_sub(1)];
        if is_source_reference_words(subject) {
            return Ok(TriggerSpec::KeywordActionFromSource {
                action,
                player: PlayerFilter::You,
            });
        }
        if subject.len() > 2 && is_source_reference_words(&subject[..2]) {
            let trailing_ok = subject[2..].iter().all(|word| {
                matches!(
                    *word,
                    "become" | "becomes" | "became" | "becoming" | "has" | "had"
                )
            });
            if trailing_ok {
                return Ok(TriggerSpec::KeywordActionFromSource {
                    action,
                    player: PlayerFilter::You,
                });
            }
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::KeywordAction {
                action,
                player,
                source_filter: None,
            });
        }
    }

    if words == ["you", "complete", "a", "dungeon"]
        || words == ["you", "completed", "a", "dungeon"]
        || words == ["you", "completes", "a", "dungeon"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::CompleteDungeon,
            player: PlayerFilter::You,
            source_filter: None,
        });
    }

    if slice_ends_with(&words, &["win", "a", "clash"])
        || slice_ends_with(&words, &["wins", "a", "clash"])
        || slice_ends_with(&words, &["won", "a", "clash"])
    {
        let subject = &words[..words.len().saturating_sub(3)];
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::WinsClash { player });
        }
    }

    if let Some(counter_word_idx) =
        find_index(&words, |word| *word == "counter" || *word == "counters")
        && matches!(
            words.get(counter_word_idx + 1).copied(),
            Some("is") | Some("are")
        )
        && words.get(counter_word_idx + 2).copied() == Some("put")
        && matches!(
            words.get(counter_word_idx + 3).copied(),
            Some("on") | Some("onto")
        )
    {
        let word_view = ActivationRestrictionCompatWords::new(tokens);
        let one_or_more = slice_starts_with(&words, &["one", "or", "more"]);
        let descriptor_token_end = word_view
            .token_index_for_word_index(counter_word_idx)
            .unwrap_or(tokens.len());
        let counter_descriptor_tokens = &tokens[..(descriptor_token_end + 1)];
        let counter_type = parse_counter_type_from_tokens(counter_descriptor_tokens);

        let object_word_start = counter_word_idx + 4;
        let object_token_start = word_view
            .token_index_for_word_index(object_word_start)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing counter recipient in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        let mut object_tokens = trim_commas(&tokens[object_token_start..]);
        let object_view = ActivationRestrictionCompatWords::new(&object_tokens);
        if matches!(object_view.first(), Some("a" | "an" | "the")) {
            let start = object_view
                .token_index_for_word_index(1)
                .unwrap_or(object_tokens.len());
            object_tokens = object_tokens[start..].to_vec();
        }
        if object_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter recipient in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let filter = parse_object_filter_lexed(&object_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported counter recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;

        return Ok(TriggerSpec::CounterPutOn {
            filter,
            counter_type,
            source_controller: None,
            one_or_more,
        });
    }

    if let Some(attacks_word_idx) =
        find_index(&words, |word| *word == "attack" || *word == "attacks")
    {
        let tail_words = &words[attacks_word_idx + 1..];
        if tail_words == ["and", "isnt", "blocked"]
            || tail_words == ["and", "isn't", "blocked"]
            || tail_words == ["and", "is", "not", "blocked"]
        {
            let attacks_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attacks_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attacks_token_idx];
            return Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(filter) => TriggerSpec::AttacksAndIsntBlocked(filter),
                    None => TriggerSpec::ThisAttacksAndIsntBlocked,
                },
            );
        }
    }

    if (slice_starts_with(&words, &["this", "creature", "blocks"])
        || slice_starts_with(&words, &["this", "blocks"]))
        && let Some(blocks_idx) = find_index(tokens, |token| {
            token.is_word("block") || token.is_word("blocks")
        })
    {
        let tail_tokens = trim_commas(&tokens[blocks_idx + 1..]);
        if !tail_tokens.is_empty() && !tail_tokens.first().is_some_and(|token| token.is_word("or"))
        {
            let blocked_filter = parse_object_filter_lexed(&tail_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported blocked-object filter in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            return Ok(TriggerSpec::ThisBlocksObject(blocked_filter));
        }
    }

    let last = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    match last {
        "attack" | "attacks" => {
            let attack_word_idx = words.len().saturating_sub(1);
            let attack_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(attack_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..attack_token_idx];
            let player_subject = trigger_subject_player_selector_lexed(subject_tokens).is_some();
            let one_or_more = ActivationRestrictionCompatWords::new(subject_tokens)
                .slice_eq(0, &["one", "or", "more"])
                || player_subject;
            Ok(
                match parse_attack_trigger_subject_filter_lexed(subject_tokens)? {
                    Some(filter) => {
                        if one_or_more {
                            TriggerSpec::AttacksOneOrMore(filter)
                        } else {
                            TriggerSpec::Attacks(filter)
                        }
                    }
                    None => TriggerSpec::ThisAttacks,
                },
            )
        }
        "block" | "blocks" => {
            let block_word_idx = words.len().saturating_sub(1);
            let block_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(block_word_idx)
                .unwrap_or(tokens.len());
            let subject_tokens = &tokens[..block_token_idx];
            Ok(match parse_trigger_subject_filter_lexed(subject_tokens)? {
                Some(filter) => TriggerSpec::Blocks(filter),
                None => TriggerSpec::ThisBlocks,
            })
        }
        "dies" => {
            let dies_word_idx = words.len().saturating_sub(1);
            let dies_token_idx = ActivationRestrictionCompatWords::new(tokens)
                .token_index_for_word_index(dies_word_idx)
                .unwrap_or(tokens.len());
            let mut subject_tokens = &tokens[..dies_token_idx];
            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("this"))
            {
                let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
                let subject_words = subject_word_view.to_word_refs();
                if let Some(or_word_idx) =
                    find_word_sequence_start(&subject_words, &["or", "another"])
                {
                    let rhs_word_idx = or_word_idx + 2;
                    let rhs_token_idx = subject_word_view
                        .token_index_for_word_index(rhs_word_idx)
                        .unwrap_or(subject_tokens.len());
                    if rhs_token_idx < subject_tokens.len() {
                        let rhs_tokens = trim_edge_punctuation(&subject_tokens[rhs_token_idx..]);
                        if !rhs_tokens.is_empty()
                            && let Ok(filter) = parse_object_filter_lexed(&rhs_tokens, false)
                        {
                            return Ok(TriggerSpec::Either(
                                Box::new(TriggerSpec::ThisDies),
                                Box::new(TriggerSpec::Dies(filter)),
                            ));
                        }
                    }
                }
                if is_source_reference_words(&subject_words) {
                    return Ok(TriggerSpec::ThisDies);
                }
                return Err(CardTextError::ParseError(format!(
                    "unsupported this-prefixed dies trigger subject (clause: '{}')",
                    words.join(" ")
                )));
            }

            let subject_word_view = ActivationRestrictionCompatWords::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            if subject_words.last().copied() == Some("haunts")
                && subject_words.first().copied() == Some("the")
                && subject_words.get(1).copied() == Some("creature")
            {
                return Ok(TriggerSpec::HauntedCreatureDies);
            }

            let mut other = false;
            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in dies trigger clause (clause: '{}')",
                    words.join(" ")
                )));
            }

            if let Some(damaged_by_trigger) =
                parse_damage_by_dies_trigger_lexed(subject_tokens, other, &words)?
            {
                return Ok(damaged_by_trigger);
            }

            if let Ok(filter) = parse_object_filter_lexed(subject_tokens, other) {
                return Ok(TriggerSpec::Dies(filter));
            }
            let mut normalized_subject_tokens = Vec::with_capacity(subject_tokens.len());
            let mut idx = 0usize;
            while idx < subject_tokens.len() {
                if subject_tokens[idx].is_word("and")
                    && subject_tokens
                        .get(idx + 1)
                        .is_some_and(|token| token.is_word("or"))
                {
                    idx += 1;
                    continue;
                }
                normalized_subject_tokens.push(subject_tokens[idx].clone());
                idx += 1;
            }
            if normalized_subject_tokens.len() != subject_tokens.len()
                && let Ok(filter) = parse_object_filter_lexed(&normalized_subject_tokens, other)
            {
                return Ok(TriggerSpec::Dies(filter));
            }

            Err(CardTextError::ParseError(format!(
                "unsupported dies trigger subject filter (clause: '{}')",
                words.join(" ")
            )))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"end")
            && slice_contains(&words, &"step") =>
        {
            Ok(TriggerSpec::BeginningOfEndStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning") && slice_contains(&words, &"upkeep") => Ok(
            TriggerSpec::BeginningOfUpkeep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"draw")
            && slice_contains(&words, &"step") =>
        {
            Ok(TriggerSpec::BeginningOfDrawStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning") && slice_contains(&words, &"combat") => Ok(
            TriggerSpec::BeginningOfCombat(parse_possessive_clause_player_filter(&words)),
        ),
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"first")
            && slice_contains(&words, &"main")
            && slice_contains(&words, &"phase") =>
        {
            Ok(TriggerSpec::BeginningOfPrecombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"second")
            && slice_contains(&words, &"main")
            && slice_contains(&words, &"phase") =>
        {
            Ok(TriggerSpec::BeginningOfPostcombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"precombat")
            && slice_contains(&words, &"main") =>
        {
            Ok(TriggerSpec::BeginningOfPrecombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if slice_contains(&words, &"beginning")
            && slice_contains(&words, &"postcombat")
            && slice_contains(&words, &"main") =>
        {
            Ok(TriggerSpec::BeginningOfPostcombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ => Err(CardTextError::ParseError(format!(
            "unsupported trigger clause (clause: '{}')",
            words.join(" ")
        ))),
    }
}

pub(crate) fn parse_discard_trigger_card_filter(
    after_discard_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let remainder = trim_commas(after_discard_tokens);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let remainder_words = crate::cards::builders::parser::token_word_refs(&remainder);
    let Some(card_word_idx) =
        find_index(&remainder_words, |word| *word == "card" || *word == "cards")
    else {
        return Err(CardTextError::ParseError(format!(
            "missing discard trigger card keyword (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let qualifier_end =
        token_index_for_word_index(&remainder, card_word_idx).unwrap_or(remainder.len());
    let qualifier_tokens = trim_commas(&remainder[..qualifier_end]);
    let mut qualifier_tokens = strip_leading_articles(&qualifier_tokens);
    if qualifier_tokens.len() >= 2
        && qualifier_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .and_then(parse_cardinal_u32)
            .is_some()
        && qualifier_tokens
            .get(1)
            .is_some_and(|token| token.is_word("or"))
    {
        qualifier_tokens = qualifier_tokens[2..].to_vec();
    } else if qualifier_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .and_then(parse_cardinal_u32)
        .is_some()
    {
        qualifier_tokens = qualifier_tokens[1..].to_vec();
    }

    let trailing_tokens = if card_word_idx + 1 < remainder_words.len() {
        let trailing_start =
            token_index_for_word_index(&remainder, card_word_idx + 1).unwrap_or(remainder.len());
        trim_commas(&remainder[trailing_start..])
    } else {
        Vec::new()
    };
    if !trailing_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing discard trigger clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if qualifier_tokens.is_empty() {
        return Ok(None);
    }

    let qualifier_words = crate::cards::builders::parser::token_word_refs(&qualifier_tokens);
    if qualifier_words.as_slice() == ["one", "or", "more"] {
        return Ok(None);
    }

    if let Ok(filter) = parse_object_filter(&qualifier_tokens, false) {
        return Ok(Some(filter));
    }

    let mut fallback = ObjectFilter::default();
    let mut parsed_any = false;
    for word in qualifier_words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(non_type) = parse_non_type(word) {
            if !slice_contains(&fallback.excluded_card_types, &non_type) {
                fallback.excluded_card_types.push(non_type);
            }
            parsed_any = true;
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            if !slice_contains(&fallback.card_types, &card_type) {
                fallback.card_types.push(card_type);
            }
            parsed_any = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if parsed_any {
        Ok(Some(fallback))
    } else {
        Err(CardTextError::ParseError(format!(
            "unsupported discard trigger card qualifier (clause: '{}')",
            clause_words.join(" ")
        )))
    }
}

pub(crate) fn parse_subtype_list_enters_trigger_filter(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = if words.len() >= 2
        && words[words.len() - 2] == "you"
        && words[words.len() - 1] == "control"
    {
        (Some(PlayerFilter::You), words.len() - 2)
    } else if words.len() >= 2
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 2)
    } else if words.len() >= 3
        && words[words.len() - 3] == "an"
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 3)
    } else {
        (None, words.len())
    };

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    Some(filter)
}

pub(crate) fn parse_possessive_clause_player_filter(words: &[&str]) -> PlayerFilter {
    let attached_controller_filter =
        |tag: &str| PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(tag)));
    let normalized_words = words
        .iter()
        .map(|word| {
            str_strip_suffix(word, "'s")
                .or_else(|| str_strip_suffix(word, "’s"))
                .or_else(|| str_strip_suffix(word, "s'"))
                .or_else(|| str_strip_suffix(word, "s’"))
                .unwrap_or(word)
        })
        .collect::<Vec<_>>();
    let has_attached_controller = |subject: &str| {
        find_window_by(&normalized_words, 3, |window| {
            window[0] == subject
                && matches!(
                    window[1],
                    "creature"
                        | "creatures"
                        | "permanent"
                        | "permanents"
                        | "artifact"
                        | "artifacts"
                        | "enchantment"
                        | "enchantments"
                        | "land"
                        | "lands"
                )
                && window[2] == "controller"
        })
        .is_some()
    };

    if contains_word_sequence(&normalized_words, &["enchanted", "player"])
        || contains_word_sequence(&normalized_words, &["enchanted", "players"])
    {
        return PlayerFilter::TaggedPlayer(TagKey::from("enchanted"));
    }
    if has_attached_controller("enchanted") {
        return attached_controller_filter("enchanted");
    }
    if has_attached_controller("equipped") {
        return attached_controller_filter("equipped");
    }

    // "each player" / "a player" / "that player" should resolve to Any,
    // even if "opponent" appears elsewhere in the clause text.  Check for
    // explicit "each/a/that player" before falling through to the opponent
    // keyword scan.
    let has_each_player = contains_word_sequence(&normalized_words, &["each", "player"]);
    if contains_your_team_words(words) || slice_contains(&words, &"your") {
        PlayerFilter::You
    } else if has_each_player {
        PlayerFilter::Any
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn parse_subject_clause_player_filter(words: &[&str]) -> PlayerFilter {
    if contains_your_team_words(words) || slice_contains(&words, &"you") {
        PlayerFilter::You
    } else if contains_word_sequence(words, &["enchanted", "player"])
        || contains_word_sequence(words, &["enchanted", "players"])
    {
        PlayerFilter::TaggedPlayer(TagKey::from("enchanted"))
    } else if contains_word_sequence(words, &["chosen", "player"])
        || contains_word_sequence(words, &["chosen", "players"])
    {
        PlayerFilter::ChosenPlayer
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

pub(crate) fn contains_opponent_word(words: &[&str]) -> bool {
    words
        .iter()
        .any(|word| matches!(*word, "opponent" | "opponents"))
}

pub(crate) fn contains_your_team_words(words: &[&str]) -> bool {
    contains_any_word_sequence(words, &[&["your", "team"], &["on", "your", "team"]])
}

pub(crate) fn parse_trigger_subject_player_filter(subject: &[&str]) -> Option<PlayerFilter> {
    if subject == ["you"] {
        return Some(PlayerFilter::You);
    }
    if subject == ["the", "chosen", "player"] || subject == ["chosen", "player"] {
        return Some(PlayerFilter::ChosenPlayer);
    }
    if slice_starts_with(&subject, &["the", "player", "who", "cast"])
        || slice_starts_with(&subject, &["player", "who", "cast"])
    {
        return Some(PlayerFilter::EffectController);
    }
    if subject == ["a", "player"]
        || subject == ["any", "player"]
        || subject == ["player"]
        || subject == ["one", "or", "more", "players"]
    {
        return Some(PlayerFilter::Any);
    }
    if subject == ["an", "opponent"]
        || subject == ["opponent"]
        || subject == ["opponents"]
        || subject == ["your", "opponents"]
        || subject == ["one", "of", "your", "opponents"]
        || subject == ["one", "or", "more", "of", "your", "opponents"]
        || subject == ["one", "of", "the", "opponents"]
        || subject == ["one", "or", "more", "opponents"]
        || subject == ["each", "opponent"]
    {
        return Some(PlayerFilter::Opponent);
    }
    if slice_ends_with(&subject, &["on", "your", "team"])
        && subject
            .iter()
            .any(|word| matches!(*word, "player" | "players"))
    {
        return Some(PlayerFilter::You);
    }
    None
}

fn parse_shuffle_trigger_subject(subject: &[&str]) -> Option<(PlayerFilter, bool, bool)> {
    if let Some(player) = parse_trigger_subject_player_filter(subject) {
        return Some((player, false, false));
    }

    if !(slice_starts_with(&subject, &["a", "spell", "or", "ability", "causes"])
        && subject.last().copied() == Some("to")
        && subject.len() > 6)
    {
        return None;
    }

    let caused_player_words = &subject[5..subject.len() - 1];
    if caused_player_words == ["its", "controller"] {
        return Some((PlayerFilter::Any, true, true));
    }

    parse_trigger_subject_player_filter(caused_player_words).map(|player| (player, true, false))
}

pub(crate) fn parse_spell_or_ability_controller_tail(words: &[&str]) -> Option<PlayerFilter> {
    let (prefix_len, controller_end) = match words {
        ["a", "spell", "or", "ability", ..] => (4usize, words.len()),
        ["spell", "or", "ability", ..] => (3usize, words.len()),
        _ => return None,
    };

    if controller_end <= prefix_len + 1 {
        return None;
    }
    if !matches!(words.last().copied(), Some("control") | Some("controls")) {
        return None;
    }

    let controller_words = &words[prefix_len..controller_end - 1];
    parse_trigger_subject_player_filter(controller_words)
}

pub(crate) fn parse_trigger_subject_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = crate::cards::builders::parser::token_word_refs(subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| matches!(*word, "that" | "which" | "who" | "whom"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    parse_object_filter(subject_tokens, other)
        .map(Some)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(subject_tokens).join(" ")
            ))
        })
}

pub(crate) fn trigger_subject_player_selector(
    subject_tokens: &[OwnedLexToken],
) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more(subject_tokens);
    let subject_words = crate::cards::builders::parser::token_word_refs(subject_tokens);
    parse_trigger_subject_player_filter(&subject_words)
}

pub(crate) fn attacking_filter_for_player(player: PlayerFilter) -> ObjectFilter {
    let mut filter = ObjectFilter::creature();
    if !matches!(player, PlayerFilter::Any) {
        filter.controller = Some(player);
    }
    filter
}

pub(crate) fn parse_attack_trigger_subject_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    let Some(mut filter) = parse_trigger_subject_filter(subject_tokens)? else {
        return Ok(None);
    };

    // Attack/combat-trigger subjects are creatures by default even when
    // expressed only as a subtype ("a Sliver", "one or more Goblins", etc.).
    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    }

    Ok(Some(filter))
}

fn strip_leading_one_or_more_lexed(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let words = ActivationRestrictionCompatWords::new(tokens);
    if words.slice_eq(0, &["one", "or", "more"]) {
        let start = words.token_index_for_word_index(3).unwrap_or(tokens.len());
        &tokens[start..]
    } else {
        tokens
    }
}

fn parse_subtype_list_enters_trigger_filter_lexed(
    tokens: &[OwnedLexToken],
    other: bool,
) -> Option<ObjectFilter> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    let words = words.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let (controller, subject_end) = if words.len() >= 2
        && words[words.len() - 2] == "you"
        && words[words.len() - 1] == "control"
    {
        (Some(PlayerFilter::You), words.len() - 2)
    } else if words.len() >= 2
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 2)
    } else if words.len() >= 3
        && words[words.len() - 3] == "an"
        && words[words.len() - 2] == "opponent"
        && words[words.len() - 1] == "controls"
    {
        (Some(PlayerFilter::Opponent), words.len() - 3)
    } else {
        (None, words.len())
    };

    let mut subtypes = Vec::new();
    for word in &words[..subject_end] {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            if !slice_contains(&subtypes, &subtype) {
                subtypes.push(subtype);
            }
        }
    }
    if subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.subtypes = subtypes;
    filter.controller = controller;
    filter.other = other;
    Some(filter)
}

fn parse_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let mut other = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| matches!(*word, "that" | "which" | "who" | "whom"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    if contains_word_sequence(
        &subject_words,
        &["power", "greater", "than", "its", "base", "power"],
    ) && subject_words
        .iter()
        .any(|word| matches!(*word, "creature" | "creatures"))
    {
        let mut filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        filter.power_greater_than_base_power = true;
        if other {
            filter.other = true;
        }
        if contains_word_sequence(&subject_words, &["you", "control"]) {
            filter.controller = Some(PlayerFilter::You);
        } else if contains_any_word_sequence(
            &subject_words,
            &[&["opponents", "control"], &["opponent", "controls"]],
        ) {
            filter.controller = Some(PlayerFilter::Opponent);
        }
        return Ok(Some(filter));
    }

    let mut normalized_subject_tokens = subject_tokens.to_vec();
    if find_window_by(&normalized_subject_tokens, 2, |window| {
        window[0].is_word("each") && window[1].is_word("with")
    })
    .is_some()
    {
        let mut normalized = Vec::with_capacity(normalized_subject_tokens.len());
        let mut idx = 0usize;
        while idx < normalized_subject_tokens.len() {
            if normalized_subject_tokens[idx].is_word("each")
                && normalized_subject_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("with"))
            {
                idx += 1;
                continue;
            }
            normalized.push(normalized_subject_tokens[idx].clone());
            idx += 1;
        }
        normalized_subject_tokens = normalized;
    }

    let mut controller_override = None;
    let word_view = ActivationRestrictionCompatWords::new(&normalized_subject_tokens);
    let normalized_words = word_view.to_word_refs();
    let controller_phrase = if let Some(idx) =
        find_word_sequence_start(&normalized_words, &["you", "control"])
            .filter(|idx| idx + 2 < normalized_words.len())
    {
        controller_override = Some(PlayerFilter::You);
        Some((idx, 2usize))
    } else if let Some(idx) = find_word_sequence_start(&normalized_words, &["opponents", "control"])
        .filter(|idx| idx + 2 < normalized_words.len())
    {
        controller_override = Some(PlayerFilter::Opponent);
        Some((idx, 2usize))
    } else if let Some(idx) = find_word_sequence_start(&normalized_words, &["opponent", "controls"])
        .filter(|idx| idx + 2 < normalized_words.len())
    {
        controller_override = Some(PlayerFilter::Opponent);
        Some((idx, 2usize))
    } else {
        None
    };

    if let Some((word_idx, len)) = controller_phrase
        && let Some(start) = token_index_for_word_index(&normalized_subject_tokens, word_idx)
        && let Some(end) = token_index_for_word_index(&normalized_subject_tokens, word_idx + len)
    {
        normalized_subject_tokens.drain(start..end);
    }

    parse_object_filter_lexed(&normalized_subject_tokens, other)
        .map(|mut filter| {
            if filter.zone.is_none()
                && filter.tagged_constraints.is_empty()
                && filter.specific.is_none()
                && !filter.source
            {
                filter.zone = Some(Zone::Battlefield);
            }
            if let Some(controller) = controller_override {
                filter.controller = Some(controller);
                filter.zone.get_or_insert(Zone::Battlefield);
            }
            Some(filter)
        })
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                subject_words.join(" ")
            ))
        })
}

fn trigger_subject_player_selector_lexed(subject_tokens: &[OwnedLexToken]) -> Option<PlayerFilter> {
    let subject_tokens = strip_leading_one_or_more_lexed(subject_tokens);
    let subject_words = ActivationRestrictionCompatWords::new(subject_tokens);
    let subject_words = subject_words.to_word_refs();
    parse_trigger_subject_player_filter(&subject_words)
}

fn parse_attack_trigger_subject_filter_lexed(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if let Some(player) = trigger_subject_player_selector_lexed(subject_tokens) {
        return Ok(Some(attacking_filter_for_player(player)));
    }
    let Some(mut filter) = parse_trigger_subject_filter_lexed(subject_tokens)? else {
        return Ok(None);
    };

    if filter.card_types.is_empty() {
        filter.card_types.push(crate::types::CardType::Creature);
    }

    Ok(Some(filter))
}

pub(crate) fn parse_exact_spell_count_each_turn(words: &[&str]) -> Option<u32> {
    for (ordinal, count) in [
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        if contains_word_sequence(words, &[ordinal, "spell", "cast", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "spell", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "spell", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "spell", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "spell", "each", "turn"])
        {
            return Some(count);
        }
    }
    None
}

pub(crate) fn parse_exact_draw_count_each_turn(words: &[&str]) -> Option<u32> {
    if contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "each", "of",
            "their", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "each", "of",
            "your", "draw", "steps",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "they", "draw", "in", "their", "draw",
            "step",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "they", "draw", "in", "their", "draw",
            "step",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "one", "you", "draw", "in", "your", "draw",
            "step",
        ],
    ) || contains_word_sequence(
        words,
        &[
            "a", "card", "except", "the", "first", "card", "you", "draw", "in", "your", "draw",
            "step",
        ],
    ) {
        return Some(2);
    }

    for (ordinal, count) in [
        ("second", 2u32),
        ("third", 3u32),
        ("fourth", 4u32),
        ("fifth", 5u32),
        ("sixth", 6u32),
        ("seventh", 7u32),
        ("eighth", 8u32),
        ("ninth", 9u32),
        ("tenth", 10u32),
    ] {
        if contains_word_sequence(words, &[ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &[ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "card", "each", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "cards", "each", "turn"])
            || contains_word_sequence(words, &[ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &[ordinal, "cards", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &["your", ordinal, "cards", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "card", "this", "turn"])
            || contains_word_sequence(words, &["their", ordinal, "cards", "this", "turn"])
        {
            return Some(count);
        }
    }
    None
}

pub(crate) fn has_first_spell_each_turn_pattern(words: &[&str]) -> bool {
    let has_turn_context = contains_word_sequence(words, &["each", "turn"])
        || contains_word_sequence(words, &["this", "turn"])
        || contains_word_sequence(words, &["of", "a", "turn"])
        || contains_word_sequence(words, &["during", "your", "turn"])
        || contains_word_sequence(words, &["during", "their", "turn"])
        || contains_word_sequence(words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(words, &["during", "opponents", "turn"])
        || contains_word_sequence(words, &["during", "each", "opponents", "turn"]);
    if !has_turn_context {
        return false;
    }

    for (idx, word) in words.iter().enumerate() {
        if *word != "first" {
            continue;
        }
        let window_end = (idx + 5).min(words.len());
        if words[idx + 1..window_end]
            .iter()
            .any(|candidate| *candidate == "spell" || *candidate == "spells")
        {
            return true;
        }
    }
    false
}

pub(crate) fn has_second_spell_turn_pattern(words: &[&str]) -> bool {
    contains_word_sequence(words, &["second", "spell", "cast", "this", "turn"])
        || contains_word_sequence(words, &["second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["your", "second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["their", "second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["your", "second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["their", "second", "spell", "this", "turn"])
        || contains_word_sequence(words, &["second", "spell", "each", "turn"])
        || contains_word_sequence(words, &["second", "spell", "during", "your", "turn"])
        || contains_word_sequence(words, &["second", "spell", "during", "their", "turn"])
        || contains_word_sequence(
            words,
            &["second", "spell", "during", "an", "opponents", "turn"],
        )
        || contains_word_sequence(words, &["second", "spell", "during", "opponents", "turn"])
        || contains_word_sequence(
            words,
            &["second", "spell", "during", "each", "opponents", "turn"],
        )
}

pub(crate) fn parse_spell_activity_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<TriggerSpec>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if !slice_contains(&clause_words, &"spell") && !slice_contains(&clause_words, &"spells") {
        return Ok(None);
    }

    let cast_idx = find_index(tokens, |token| {
        token.is_word("cast") || token.is_word("casts")
    });
    let copy_idx = find_index(tokens, |token| {
        token.is_word("copy") || token.is_word("copies")
    });
    if cast_idx.is_none() && copy_idx.is_none() {
        return Ok(None);
    }

    let mut actor = parse_subject_clause_player_filter(&clause_words);
    let during_their_turn = contains_word_sequence(&clause_words, &["during", "their", "turn"])
        || contains_word_sequence(&clause_words, &["during", "that", "players", "turn"]);
    let mut during_turn = if contains_word_sequence(&clause_words, &["during", "your", "turn"]) {
        Some(PlayerFilter::You)
    } else if contains_word_sequence(&clause_words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "each", "opponents", "turn"])
    {
        Some(PlayerFilter::Opponent)
    } else {
        None
    };
    if during_their_turn {
        if matches!(actor, PlayerFilter::Any) {
            actor = PlayerFilter::Active;
            during_turn = None;
        } else if during_turn.is_none() {
            during_turn = Some(actor.clone());
        }
    }
    let has_other_than_first_spell_pattern =
        contains_word_sequence(&clause_words, &["other", "than", "your", "first", "spell"])
            || contains_word_sequence(&clause_words, &["other", "than", "the", "first", "spell"])
            || (contains_word_sequence(&clause_words, &["other", "than", "the", "first"])
                && slice_contains(&clause_words, &"spell")
                && slice_contains(&clause_words, &"casts")
                && slice_contains(&clause_words, &"turn"));
    let second_spell_turn_pattern = has_second_spell_turn_pattern(&clause_words);
    let first_spell_each_turn =
        !has_other_than_first_spell_pattern && has_first_spell_each_turn_pattern(&clause_words);
    let exact_spells_this_turn = parse_exact_spell_count_each_turn(&clause_words)
        .or_else(|| first_spell_each_turn.then_some(1))
        .or_else(|| {
            (!has_other_than_first_spell_pattern && second_spell_turn_pattern).then_some(2)
        });
    let min_spells_this_turn = if exact_spells_this_turn.is_some() {
        None
    } else if has_other_than_first_spell_pattern {
        Some(2)
    } else {
        None
    };
    let from_not_hand =
        contains_word_sequence(
            &clause_words,
            &["from", "anywhere", "other", "than", "your", "hand"],
        ) || contains_word_sequence(
            &clause_words,
            &["from", "anywhere", "other", "than", "their", "hand"],
        ) || contains_word_sequence(
            &clause_words,
            &["from", "anywhere", "other", "than", "hand"],
        ) || find_word_sequence_start(&clause_words, &["from", "anywhere", "other", "than"])
            .is_some_and(|idx| {
                clause_words[idx + 4..]
                    .iter()
                    .take(4)
                    .any(|word| *word == "hand")
            });

    let parse_filter =
        |filter_tokens: &[OwnedLexToken]| -> Result<Option<ObjectFilter>, CardTextError> {
            let filter_tokens = if let Some(idx) = find_index(filter_tokens, |token| {
                token.is_word("during") || token.is_word("other")
            }) {
                &filter_tokens[..idx]
            } else {
                filter_tokens
            };
            let filter_tokens = if let Some(idx) =
                find_index(filter_tokens, |token| token.is_word("from")).filter(|idx| {
                    filter_tokens
                        .get(idx + 1)
                        .is_some_and(|token| token.is_word("anywhere"))
                }) {
                &filter_tokens[..idx]
            } else {
                filter_tokens
            };
            let filter_words: Vec<&str> = filter_tokens
                .iter()
                .filter_map(OwnedLexToken::as_word)
                .collect();
            let is_unqualified_spell = filter_words.as_slice() == ["a", "spell"]
                || filter_words.as_slice() == ["spells"]
                || filter_words.as_slice() == ["spell"];
            if filter_tokens.is_empty() || is_unqualified_spell {
                Ok(None)
            } else {
                let parse_spell_origin_zone_filter = || -> Option<ObjectFilter> {
                    let zone = if slice_contains(&filter_words, &"graveyard") {
                        Some(Zone::Graveyard)
                    } else if slice_contains(&filter_words, &"exile") {
                        Some(Zone::Exile)
                    } else {
                        None
                    }?;
                    let mentions_spell = slice_contains(&filter_words, &"spell")
                        || slice_contains(&filter_words, &"spells");
                    if !mentions_spell {
                        return None;
                    }
                    let mut filter = ObjectFilter::spell().in_zone(zone);
                    if slice_contains(&filter_words, &"your") {
                        filter.owner = Some(actor.clone());
                    } else if slice_contains(&filter_words, &"opponent")
                        || slice_contains(&filter_words, &"their")
                    {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    Some(filter)
                };
                let compact_words = filter_words
                    .iter()
                    .copied()
                    .filter(|word| !is_article(word))
                    .collect::<Vec<_>>();
                if compact_words
                    .last()
                    .is_some_and(|last| *last == "spell" || *last == "spells")
                {
                    let mut qualifier_words = compact_words.clone();
                    qualifier_words.pop();
                    let qualifier_words = qualifier_words
                        .into_iter()
                        .filter(|word| *word != "or" && *word != "and")
                        .collect::<Vec<_>>();
                    if matches!(
                        qualifier_words.as_slice(),
                        ["of", "the", "chosen", "color"] | ["of", "chosen", "color"]
                    ) {
                        return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                    }
                }
                match parse_object_filter(filter_tokens, false) {
                    Ok(filter) => Ok(Some(filter)),
                    Err(err) => {
                        let mut compact_words = compact_words;
                        if compact_words
                            .last()
                            .is_some_and(|last| *last == "spell" || *last == "spells")
                        {
                            compact_words.pop();
                            let color_words = compact_words
                                .into_iter()
                                .filter(|word| *word != "or" && *word != "and")
                                .collect::<Vec<_>>();
                            if !color_words.is_empty()
                                && color_words.iter().all(|word| parse_color(word).is_some())
                            {
                                let mut colors = ColorSet::new();
                                for word in color_words {
                                    colors = colors
                                        .union(parse_color(word).expect("validated color word"));
                                }
                                let mut filter = ObjectFilter::spell();
                                filter.colors = Some(colors);
                                return Ok(Some(filter));
                            }
                            if matches!(
                                color_words.as_slice(),
                                ["of", "the", "chosen", "color"] | ["of", "chosen", "color"]
                            ) {
                                return Ok(Some(ObjectFilter::spell().of_chosen_color()));
                            }
                        }
                        if let Some(origin_filter) = parse_spell_origin_zone_filter() {
                            Ok(Some(origin_filter))
                        } else {
                            Err(err)
                        }
                    }
                }
            }
        };

    if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
        let (first, second, first_is_cast) = if cast < copy {
            (cast, copy, true)
        } else {
            (copy, cast, false)
        };
        let between_words =
            crate::cards::builders::parser::token_word_refs(&tokens[first + 1..second]);
        if between_words.as_slice() == ["or"] {
            let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
            let cast_trigger = TriggerSpec::SpellCast {
                filter: filter.clone(),
                caster: actor.clone(),
                during_turn: during_turn.clone(),
                min_spells_this_turn,
                exact_spells_this_turn,
                from_not_hand,
            };
            let copied_trigger = TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            };
            return Ok(Some(if first_is_cast {
                TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
            } else {
                TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
            }));
        }
    }

    if let Some(cast) = cast_idx {
        let mut filter_tokens = tokens.get(cast + 1..).unwrap_or_default();
        if filter_tokens.is_empty() {
            let mut prefix_tokens = &tokens[..cast];
            while let Some(last_word) = prefix_tokens.last().and_then(OwnedLexToken::as_word) {
                if matches!(last_word, "is" | "are" | "was" | "were" | "be" | "been") {
                    prefix_tokens = &prefix_tokens[..prefix_tokens.len() - 1];
                } else {
                    break;
                }
            }
            let has_spell_noun = prefix_tokens
                .iter()
                .any(|token| token.is_word("spell") || token.is_word("spells"));
            if has_spell_noun {
                filter_tokens = prefix_tokens;
            }
        }
        let filter = parse_filter(filter_tokens)?;
        return Ok(Some(TriggerSpec::SpellCast {
            filter,
            caster: actor,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        }));
    }

    if let Some(copy) = copy_idx {
        let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
        return Ok(Some(TriggerSpec::SpellCopied {
            filter,
            copier: actor,
        }));
    }

    Ok(None)
}

pub(crate) fn is_spawn_scion_token_mana_reminder(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    let starts_with_token_pronoun = matches!(
        words.as_slice(),
        ["they", "have", ..]
            | ["it", "has", ..]
            | ["this", "token", "has", ..]
            | ["those", "tokens", "have", ..]
    );
    starts_with_token_pronoun
        && words.iter().any(|word| *word == "sacrifice")
        && words.iter().any(|word| *word == "add")
        && words.iter().any(|word| *word == "c")
}

pub(crate) fn is_round_up_each_time_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    matches!(words.as_slice(), ["round", "up", "each", "time", ..])
}

pub(crate) enum MayCastItVerb {
    Cast,
    Play,
}

pub(crate) struct MayCastTaggedSpec {
    pub(crate) verb: MayCastItVerb,
    pub(crate) as_copy: bool,
    pub(crate) without_paying_mana_cost: bool,
    pub(crate) predicate: Option<PredicateAst>,
    pub(crate) cost_reduction: Option<ManaCost>,
}

pub(crate) fn parse_may_cast_it_sentence(tokens: &[OwnedLexToken]) -> Option<MayCastTaggedSpec> {
    let mut clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }

    if slice_starts_with(&clause_words, &["if", "you", "do"]) {
        clause_words = clause_words[3..].to_vec();
        while clause_words
            .first()
            .is_some_and(|word| *word == "then" || *word == "and")
        {
            clause_words.remove(0);
        }
    }

    if clause_words.len() < 4 || clause_words[0] != "you" || clause_words[1] != "may" {
        return None;
    }

    let verb = match clause_words[2] {
        "cast" => MayCastItVerb::Cast,
        "play" => MayCastItVerb::Play,
        _ => return None,
    };

    let rest = &clause_words[3..];
    let (as_copy, consumed) = if slice_starts_with(&rest, &["it"]) {
        (false, 1usize)
    } else if slice_starts_with(&rest, &["the", "copy"])
        || slice_starts_with(&rest, &["that", "copy"])
        || slice_starts_with(&rest, &["a", "copy"])
    {
        (true, 2usize)
    } else {
        return None;
    };

    let tail = &rest[consumed..];
    if tail.is_empty() {
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: false,
            predicate: None,
            cost_reduction: None,
        });
    }
    if tail == ["without", "paying", "its", "mana", "cost"] {
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: true,
            predicate: None,
            cost_reduction: None,
        });
    }
    if let [
        "without",
        "paying",
        "its",
        "mana",
        "cost",
        "if",
        "its",
        "mana",
        "value",
        "is",
        parity,
    ] = tail
    {
        let parity = match *parity {
            "odd" => crate::filter::ParityRequirement::Odd,
            "even" => crate::filter::ParityRequirement::Even,
            _ => return None,
        };
        return Some(MayCastTaggedSpec {
            verb,
            as_copy,
            without_paying_mana_cost: true,
            predicate: Some(PredicateAst::ItMatches(
                ObjectFilter::default().with_mana_value_parity(parity),
            )),
            cost_reduction: None,
        });
    }
    None
}

pub(crate) fn parse_copy_reference_cost_reduction_sentence(
    tokens: &[OwnedLexToken],
) -> Option<ManaCost> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return None;
    }
    if !(slice_starts_with(&clause_words, &["that", "copy", "costs"])
        || slice_starts_with(&clause_words, &["the", "copy", "costs"])
        || slice_starts_with(&clause_words, &["a", "copy", "costs"]))
    {
        return None;
    }

    let less_idx = find_index(&clause_words, |word| *word == "less")?;
    if clause_words.get(less_idx + 1).copied() != Some("to")
        || clause_words.get(less_idx + 2).copied() != Some("cast")
    {
        return None;
    }

    let costs_token_idx = find_index(tokens, |token| token.is_word("costs"))?;
    let less_token_idx = find_index(tokens, |token| token.is_word("less"))?;
    if less_token_idx <= costs_token_idx + 1 {
        return None;
    }
    let reduction_tokens = trim_commas(&tokens[costs_token_idx + 1..less_token_idx]).to_vec();
    let (reduction, consumed) = parse_cost_modifier_mana_cost(&reduction_tokens)?;
    if consumed != reduction_tokens.len() {
        return None;
    }
    Some(reduction)
}

pub(crate) fn build_may_cast_tagged_effect(spec: &MayCastTaggedSpec) -> EffectAst {
    let cast = EffectAst::CastTagged {
        tag: TagKey::from(IT_TAG),
        allow_land: matches!(spec.verb, MayCastItVerb::Play),
        as_copy: spec.as_copy,
        without_paying_mana_cost: spec.without_paying_mana_cost,
        cost_reduction: spec.cost_reduction.clone(),
    };
    let may = EffectAst::May {
        effects: vec![cast],
    };
    if let Some(predicate) = &spec.predicate {
        EffectAst::Conditional {
            predicate: predicate.clone(),
            if_true: vec![may],
            if_false: Vec::new(),
        }
    } else {
        may
    }
}

pub(crate) fn is_simple_copy_reference_sentence(tokens: &[OwnedLexToken]) -> bool {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    matches!(
        clause_words.as_slice(),
        ["copy", "it"]
            | ["copy", "this"]
            | ["copy", "that"]
            | ["copy", "that", "card"]
            | ["copy", "the", "exiled", "card"]
    )
}

pub(crate) fn token_name_mentions_eldrazi_spawn_or_scion(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.matches("eldrazi").next().is_some() && lower.matches("spawn").next().is_some())
        || (lower.matches("eldrazi").next().is_some() && lower.matches("scion").next().is_some())
}

pub(crate) fn effect_creates_eldrazi_spawn_or_scion(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateTokenWithMods { name, .. } => {
            token_name_mentions_eldrazi_spawn_or_scion(name)
        }
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_eldrazi_spawn_or_scion) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn effect_creates_any_token(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateTokenWithMods { .. }
        | EffectAst::CreateTokenCopy { .. }
        | EffectAst::CreateTokenCopyFromSource { .. }
        | EffectAst::Populate { .. } => true,
        _ => {
            let mut found = false;
            for_each_nested_effects(effect, false, |nested| {
                if !found && nested.iter().any(effect_creates_any_token) {
                    found = true;
                }
            });
            found
        }
    }
}

pub(crate) fn last_created_token_info(effects: &[EffectAst]) -> Option<(String, PlayerAst)> {
    for effect in effects.iter().rev() {
        if let Some(info) = created_token_info_from_effect(effect) {
            return Some(info);
        }
    }
    None
}

pub(crate) fn created_token_info_from_effect(effect: &EffectAst) -> Option<(String, PlayerAst)> {
    match effect {
        EffectAst::CreateTokenWithMods { name, player, .. } => Some((name.clone(), *player)),
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, true, |nested| {
                if found.is_none() {
                    found = last_created_token_info(nested);
                }
            });
            found
        }
    }
}

pub(crate) fn title_case_token_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = first.to_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}

pub(crate) fn controller_filter_for_token_player(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        _ => None,
    }
}

pub(crate) fn parse_sentence_exile_that_token_when_source_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.len() < 6 || !matches!(clause_words.first().copied(), Some("exile" | "exiles"))
    {
        return None;
    }
    let when_idx = find_index(&clause_words, |word| *word == "when")?;
    if when_idx < 2 || when_idx + 3 >= clause_words.len() {
        return None;
    }
    if !slice_ends_with(&clause_words, &["leaves", "the", "battlefield"]) {
        return None;
    }
    let object_words = &clause_words[1..when_idx];
    let is_created_token_reference = object_words == ["that", "token"]
        || object_words == ["those", "tokens"]
        || object_words == ["them"]
        || object_words == ["it"];
    if !is_created_token_reference {
        return None;
    }
    let subject_words = &clause_words[when_idx + 1..clause_words.len() - 3];
    if !is_source_reference_words(subject_words) {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::ExileWhenSourceLeaves {
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    })
}

pub(crate) fn parse_sentence_sacrifice_source_when_that_token_leaves(
    tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.len() < 8 || !matches!(clause_words[0], "sacrifice" | "sacrifices") {
        return None;
    }
    let when_idx = find_index(&clause_words, |word| *word == "when")?;
    if when_idx < 2 || when_idx + 4 > clause_words.len() {
        return None;
    }
    let subject_words = &clause_words[1..when_idx];
    if !is_source_reference_words(subject_words) {
        return None;
    }
    if clause_words[when_idx + 1..] != ["that", "token", "leaves", "the", "battlefield"] {
        return None;
    }

    let _ = last_created_token_info(prior_effects)?;

    Some(EffectAst::SacrificeSourceWhenLeaves {
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
    })
}

pub(crate) fn is_generic_token_reminder_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.is_empty() {
        return false;
    }
    if slice_starts_with(&words, &["it", "has"]) || slice_starts_with(&words, &["they", "have"]) {
        return true;
    }
    if slice_starts_with(&words, &["when", "it"])
        || slice_starts_with(&words, &["whenever", "it"])
        || slice_starts_with(&words, &["when", "they"])
        || slice_starts_with(&words, &["whenever", "they"])
    {
        return true;
    }
    if slice_starts_with(&words, &["its", "power"])
        || slice_starts_with(&words, &["its", "power", "and", "toughness"])
        || slice_starts_with(&words, &["its", "toughness"])
    {
        return true;
    }
    let delayed_lifecycle_reference = matches!(words.first().copied(), Some("exile" | "sacrifice"))
        && (is_beginning_of_end_step_words(&words) || is_end_of_combat_words(&words))
        && (slice_contains(&words, &"token")
            || slice_contains(&words, &"tokens")
            || slice_contains(&words, &"it")
            || slice_contains(&words, &"them"));
    if delayed_lifecycle_reference {
        return true;
    }
    slice_starts_with(&words, &["when", "this", "token"])
        || slice_starts_with(&words, &["whenever", "this", "token"])
        || slice_starts_with(&words, &["this", "token"])
        || slice_starts_with(&words, &["those", "tokens"])
}

pub(crate) fn strip_embedded_token_rules_text(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words_all = crate::cards::builders::parser::token_word_refs(tokens);
    if !slice_contains(&words_all, &"create") || !slice_contains(&words_all, &"token") {
        return tokens.to_vec();
    }
    let Some(with_idx) = find_index(tokens, |token| token.is_word("with")) else {
        return tokens.to_vec();
    };
    let next_word = tokens.get(with_idx + 1).and_then(OwnedLexToken::as_word);
    if matches!(next_word, Some("t")) {
        return tokens[..with_idx].to_vec();
    }
    tokens.to_vec()
}

pub(crate) fn append_token_reminder_to_last_create_effect(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) -> bool {
    let reminder_word_storage = tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::ManaGroup => {
                let inner = token.slice.trim_start_matches('{').trim_end_matches('}');
                (!inner.is_empty()).then(|| inner.to_ascii_lowercase())
            }
            _ => token.as_word().map(|word| word.to_ascii_lowercase()),
        })
        .collect::<Vec<_>>();
    let mut reminder_words = reminder_word_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut prepend_with = false;
    if slice_starts_with(&reminder_words, &["it", "has"])
        || slice_starts_with(&reminder_words, &["they", "have"])
    {
        reminder_words = reminder_words[2..].to_vec();
        prepend_with = true;
    }
    if slice_starts_with(&reminder_words, &["when", "it"]) {
        let mut rewritten = vec!["when", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if slice_starts_with(&reminder_words, &["whenever", "it"]) {
        let mut rewritten = vec!["whenever", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if slice_starts_with(&reminder_words, &["when", "they"]) {
        let mut rewritten = vec!["when", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    } else if slice_starts_with(&reminder_words, &["whenever", "they"]) {
        let mut rewritten = vec!["whenever", "this", "token"];
        rewritten.extend_from_slice(&reminder_words[2..]);
        reminder_words = rewritten;
    }
    if reminder_words.is_empty() {
        return false;
    }
    let reminder = if prepend_with {
        format!("with {}", reminder_words.join(" "))
    } else {
        reminder_words.join(" ")
    };
    for effect in effects.iter_mut().rev() {
        if append_token_reminder_to_effect(Some(effect), &reminder, &reminder_words) {
            return true;
        }
    }
    false
}

pub(crate) fn append_token_reminder_to_effect(
    effect: Option<&mut EffectAst>,
    reminder: &str,
    reminder_words: &[&str],
) -> bool {
    fn parse_dynamic_token_pt_reminder(reminder_words: &[&str]) -> Option<(Value, Value)> {
        use super::util::parse_value;

        let parse_rhs = |words: &[&str]| {
            let tokens = words
                .iter()
                .map(|word| OwnedLexToken::synthetic_word((*word).to_string()))
                .collect::<Vec<_>>();
            let (value, used) = parse_value(&tokens)?;
            (used == words.len()).then_some(value)
        };

        if let Some(rhs_words) = slice_strip_prefix(
            reminder_words,
            &[
                "its",
                "power",
                "and",
                "toughness",
                "are",
                "each",
                "equal",
                "to",
            ],
        ) {
            let value = parse_rhs(rhs_words)?;
            return Some((value.clone(), value));
        }
        let mut and_idx = None;
        let mut idx = 0usize;
        while idx < reminder_words.len() {
            if reminder_words[idx] == "and" {
                and_idx = Some(idx);
                break;
            }
            idx += 1;
        }
        if let Some(and_idx) = and_idx {
            let left = &reminder_words[..and_idx];
            let right = &reminder_words[and_idx + 1..];
            let power_words = slice_strip_prefix(left, &["its", "power", "is", "equal", "to"])?;
            let toughness_words =
                slice_strip_prefix(right, &["its", "toughness", "is", "equal", "to"])?;
            return Some((parse_rhs(power_words)?, parse_rhs(toughness_words)?));
        }

        None
    }

    let Some(effect) = effect else {
        return false;
    };
    match effect {
        EffectAst::CreateTokenCopy {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::Populate {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            if reminder_words == ["haste"] {
                *has_haste = true;
                return true;
            }
            let (sacrifice_next_end_step, exile_next_end_step) =
                parse_next_end_step_token_delay_flags(reminder_words);
            if sacrifice_next_end_step {
                *sacrifice_at_next_end_step = true;
            }
            if exile_next_end_step {
                *exile_at_next_end_step = true;
            }
            let exile_end_of_combat =
                slice_contains(&reminder_words, &"exile") && is_end_of_combat_words(reminder_words);
            if exile_end_of_combat {
                *exile_at_end_of_combat = true;
            }
            *has_haste
                || *sacrifice_at_next_end_step
                || *exile_at_next_end_step
                || *exile_at_end_of_combat
        }
        EffectAst::CreateTokenWithMods {
            name,
            dynamic_power_toughness,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            if let Some((power, toughness)) = parse_dynamic_token_pt_reminder(reminder_words) {
                *dynamic_power_toughness = Some((power, toughness));
                return true;
            }
            if !name.chars().last().is_some_and(|ch| ch == ' ') {
                name.push(' ');
            }
            name.push_str(reminder);
            let (sacrifice_next_end_step, exile_next_end_step) =
                parse_next_end_step_token_delay_flags(reminder_words);
            if sacrifice_next_end_step {
                *sacrifice_at_next_end_step = true;
            }
            if exile_next_end_step {
                *exile_at_next_end_step = true;
            }
            let exile_end_of_combat =
                slice_contains(&reminder_words, &"exile") && is_end_of_combat_words(reminder_words);
            if exile_end_of_combat {
                *exile_at_end_of_combat = true;
            }
            let sacrifice_end_of_combat = slice_contains(&reminder_words, &"sacrifice")
                && is_end_of_combat_words(reminder_words);
            if sacrifice_end_of_combat {
                *sacrifice_at_end_of_combat = true;
            }
            true
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, false, |nested| {
                if !applied {
                    applied = append_token_reminder_to_effect(
                        nested.last_mut(),
                        reminder,
                        reminder_words,
                    );
                }
            });
            applied
        }
    }
}

pub(crate) fn parse_target_player_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let (chooser, choose_start_idx) =
        if clause_words.first().copied() == Some("target") && clause_words.len() >= 4 {
            let chooser = match clause_words.get(1).copied() {
                Some("player") => PlayerAst::Target,
                Some("opponent") | Some("opponents") => PlayerAst::TargetOpponent,
                _ => return Ok(None),
            };
            if !matches!(
                clause_words.get(2).copied(),
                Some("choose") | Some("chooses")
            ) {
                return Ok(None);
            }
            (chooser, 3usize)
        } else if clause_words.len() >= 4
            && clause_words.first().copied() == Some("that")
            && matches!(clause_words.get(1).copied(), Some("player" | "players"))
            && matches!(clause_words.get(2).copied(), Some("choose" | "chooses"))
        {
            (PlayerAst::That, 3usize)
        } else if clause_words.len() >= 4
            && clause_words.first().copied() == Some("the")
            && matches!(clause_words.get(1).copied(), Some("voter"))
            && matches!(clause_words.get(2).copied(), Some("choose" | "chooses"))
        {
            (PlayerAst::That, 3usize)
        } else {
            return Ok(None);
        };

    let mut choose_object_tokens = trim_commas(&tokens[choose_start_idx..]);
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut count = ChoiceCount::exactly(1);
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("up"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("to"))
        && let Some((value, used)) = parse_number(&choose_object_tokens[2..])
    {
        count = ChoiceCount {
            min: 0,
            max: Some(value as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        };
        choose_object_tokens = trim_commas(&choose_object_tokens[2 + used..]);
    } else if let Some((value, used)) = parse_number(&choose_object_tokens) {
        count = ChoiceCount::exactly(value as usize);
        choose_object_tokens = trim_commas(&choose_object_tokens[used..]);
    }
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter after count in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("target"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("player") || token.is_word("opponent"))
    {
        return Ok(None);
    }
    if find_verb(&choose_object_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter = parse_object_filter(&choose_object_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some((chooser, choose_filter, count)))
}

pub(crate) fn parse_you_choose_objects_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let choose_word_idx = if clause_words.first().copied() == Some("you") {
        1usize
    } else {
        0usize
    };
    if !matches!(
        clause_words.get(choose_word_idx).copied(),
        Some("choose" | "chooses")
    ) {
        return Ok(None);
    }

    let choose_word_token_idx =
        token_index_for_word_index(tokens, choose_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing choose keyword in choose clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let mut choose_object_tokens = trim_commas(&tokens[choose_word_token_idx + 1..]).to_vec();
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut references_it = false;
    loop {
        let len = choose_object_tokens.len();
        let trailing_it = len >= 2
            && choose_object_tokens[len - 2]
                .as_word()
                .is_some_and(|word| matches!(word, "from" | "in"))
            && choose_object_tokens[len - 1]
                .as_word()
                .is_some_and(|word| matches!(word, "it" | "them"));
        let trailing_there = len >= 3
            && choose_object_tokens[len - 3]
                .as_word()
                .is_some_and(|word| matches!(word, "from" | "in"))
            && choose_object_tokens[len - 2].is_word("there")
            && choose_object_tokens[len - 1].is_word("in");
        if trailing_it {
            references_it = true;
            choose_object_tokens.truncate(len - 2);
            continue;
        }
        if trailing_there {
            references_it = true;
            choose_object_tokens.truncate(len - 3);
            continue;
        }
        break;
    }
    let mut choose_words = crate::cards::builders::parser::token_word_refs(&choose_object_tokens);
    loop {
        if matches!(
            choose_words.as_slice(),
            [.., "from", "it"] | [.., "from", "them"] | [.., "in", "it"] | [.., "in", "them"]
        ) {
            references_it = true;
            choose_words.truncate(choose_words.len().saturating_sub(2));
            continue;
        }
        if matches!(choose_words.as_slice(), [.., "from", "there", "in"]) {
            references_it = true;
            choose_words.truncate(choose_words.len().saturating_sub(3));
            continue;
        }
        break;
    }
    let mut count = ChoiceCount::exactly(1);
    if slice_starts_with(&choose_words, &["up", "to"])
        && let Some((value, used)) = parse_number(
            &choose_words[2..]
                .iter()
                .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>(),
        )
    {
        count = ChoiceCount {
            min: 0,
            max: Some(value as usize),
            dynamic_x: false,
            up_to_x: false,
            random: false,
        };
        choose_words = choose_words[2 + used..].to_vec();
    } else if let Some((value, used)) = parse_number(
        &choose_words
            .iter()
            .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>(),
    ) {
        count = ChoiceCount::exactly(value as usize);
        choose_words = choose_words[used..].to_vec();
    } else if choose_words.first().is_some_and(|word| is_article(word)) {
        choose_words = choose_words[1..].to_vec();
    }

    if choose_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter in choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let choose_filter_tokens = choose_words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    if find_verb(&choose_filter_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter =
        if references_it && matches!(choose_words.as_slice(), ["card"] | ["cards"]) {
            ObjectFilter::default()
        } else {
            parse_object_filter(&choose_filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported chosen object filter in choose clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        };
    if references_it {
        if choose_filter.zone.is_none() {
            choose_filter.zone = Some(Zone::Hand);
        }
        if !choose_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(IT_TAG),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
        }
    }
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if references_it {
        choose_filter.controller = None;
        choose_filter.owner = None;
    } else if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(PlayerFilter::You);
    }

    Ok(Some((PlayerAst::You, choose_filter, count)))
}

pub(crate) fn parse_you_choose_player_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<(PlayerAst, PlayerFilter, bool, usize)>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let choose_word_idx = if clause_words.first().copied() == Some("you") {
        1usize
    } else {
        0usize
    };
    if !matches!(
        clause_words.get(choose_word_idx).copied(),
        Some("choose" | "chooses")
    ) {
        return Ok(None);
    }

    let choose_word_token_idx =
        token_index_for_word_index(tokens, choose_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing choose keyword in choose-player clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let player_tokens = trim_commas(&tokens[choose_word_token_idx + 1..]);
    let mut player_words = crate::cards::builders::parser::token_word_refs(&player_tokens);
    if player_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen player in choose-player clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut exclude_previous_choices = 0usize;
    while let Some(word) = player_words.first().copied() {
        match word {
            "a" | "an" => {
                player_words = player_words[1..].to_vec();
            }
            "another" => {
                exclude_previous_choices = exclude_previous_choices.max(1);
                player_words = player_words[1..].to_vec();
            }
            "second" => {
                exclude_previous_choices = exclude_previous_choices.max(1);
                player_words = player_words[1..].to_vec();
            }
            "third" => {
                exclude_previous_choices = exclude_previous_choices.max(2);
                player_words = player_words[1..].to_vec();
            }
            _ => break,
        }
    }

    let mut filter = match player_words.first().copied() {
        Some("player") => {
            player_words = player_words[1..].to_vec();
            None
        }
        Some("opponent" | "opponents") => {
            player_words = player_words[1..].to_vec();
            Some(PlayerFilter::Opponent)
        }
        _ => return Ok(None),
    };

    let mut random = false;
    if slice_starts_with(&player_words, &["at", "random"]) {
        random = true;
        player_words = player_words[2..].to_vec();
    }

    let filter = if let Some(filter) = filter.take() {
        if player_words.is_empty() {
            filter
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported chosen player filter in choose clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    } else {
        match player_words.as_slice() {
            [] => PlayerFilter::Any,
            [
                "with",
                "the",
                "most",
                "life",
                "or",
                "tied",
                "for",
                "most",
                "life",
            ] => PlayerFilter::MostLifeTied,
            [
                "who",
                "cast",
                "one",
                "or",
                "more",
                "sorcery",
                "spells",
                "this",
                "turn",
            ] => PlayerFilter::CastCardTypeThisTurn(CardType::Sorcery),
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported chosen player filter in choose clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    Ok(Some((
        PlayerAst::You,
        filter,
        random,
        exclude_previous_choices,
    )))
}

pub(crate) fn parse_target_player_chooses_then_other_cant_block(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first)?
    else {
        return Ok(None);
    };
    if choose_filter.card_types.is_empty() {
        choose_filter.card_types.push(CardType::Creature);
    }

    let second_words = crate::cards::builders::parser::token_word_refs(second);
    let Some((neg_start, neg_end)) = find_negation_span(second) else {
        return Ok(None);
    };
    let tail_words_storage = normalize_cant_words(&second[neg_end..]);
    let tail_words = tail_words_storage
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if !matches!(tail_words.as_slice(), ["block", "this", "turn"] | ["block"]) {
        return Ok(None);
    }

    let mut subject_tokens = trim_commas(&second[..neg_start]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut exclude_tagged_choice = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("other") || token.is_word("another"))
    {
        exclude_tagged_choice = true;
        subject_tokens = trim_commas(&subject_tokens[1..]);
    }
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut restriction_filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-block subject filter (clause: '{}')",
            second_words.join(" ")
        ))
    })?;
    if restriction_filter.card_types.is_empty() {
        restriction_filter.card_types.push(CardType::Creature);
    }
    if restriction_filter.controller.is_none() {
        restriction_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }
    if exclude_tagged_choice
        && !restriction_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        restriction_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::block(restriction_filter),
            duration: Until::EndOfTurn,
            condition: None,
        },
    ]))
}

#[cfg(test)]
mod tests {
    use super::super::util::tokenize_line;
    use super::*;
    use crate::effect::Restriction;
    use crate::zone::Zone;

    #[test]
    fn parse_negated_object_restriction_clause_supports_attack_or_block_alone() {
        let tokens = tokenize_line("This creature can't attack or block alone.", 0);

        let parsed = parse_negated_object_restriction_clause(&tokens)
            .expect("parse attack-or-block-alone restriction")
            .expect("expected restriction");

        assert!(matches!(
            parsed.restriction,
            Restriction::AttackOrBlockAlone(_)
        ));
    }

    #[test]
    fn parse_you_choose_objects_clause_supports_bare_card_from_it() {
        let tokens = tokenize_line("You choose a card from it.", 0);

        let (chooser, filter, count) = parse_you_choose_objects_clause(&tokens)
            .expect("parse choose-a-card-from-it clause")
            .expect("expected choose clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(count, ChoiceCount::exactly(1));
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG),
            "expected hand choice to stay tied to the prior revealed hand, got {filter:?}"
        );
        assert!(
            filter.controller.is_none(),
            "expected no controller pin, got {filter:?}"
        );
        assert!(
            filter.owner.is_none(),
            "expected no owner pin, got {filter:?}"
        );
    }

    #[test]
    fn parse_you_choose_player_clause_supports_choose_an_opponent() {
        let tokens = tokenize_line("Choose an opponent.", 0);

        let (chooser, filter, random, exclude_previous_choices) =
            parse_you_choose_player_clause(&tokens)
                .expect("parse choose-an-opponent clause")
                .expect("expected choose-player clause");

        assert_eq!(chooser, PlayerAst::You);
        assert_eq!(filter, PlayerFilter::Opponent);
        assert!(!random);
        assert_eq!(exclude_previous_choices, 0);
    }

    #[test]
    fn parse_choose_card_type_phrase_words_supports_limited_type_lists() {
        let parsed =
            parse_choose_card_type_phrase_words(&["choose", "artifact", "creature", "or", "land"])
                .expect("limited choose-card-type phrase should parse")
                .expect("expected choose-card-type phrase");

        assert_eq!(
            parsed,
            (
                5,
                vec![CardType::Artifact, CardType::Creature, CardType::Land]
            )
        );
    }

    #[test]
    fn parse_choose_card_type_phrase_words_supports_permanent_types() {
        let parsed = parse_choose_card_type_phrase_words(&["choose", "a", "permanent", "type"])
            .expect("permanent-type choice phrase should parse")
            .expect("expected choose-card-type phrase");

        assert_eq!(
            parsed,
            (
                4,
                vec![
                    CardType::Artifact,
                    CardType::Creature,
                    CardType::Enchantment,
                    CardType::Land,
                    CardType::Planeswalker,
                    CardType::Battle,
                ]
            )
        );
    }

    #[test]
    fn parse_cant_restriction_clause_supports_that_player_cant_cast_spells() {
        let tokens = tokenize_line("That player can't cast spells.", 0);

        let parsed = parse_cant_restriction_clause(&tokens)
            .expect("parse that-player cant-cast clause")
            .expect("expected cant restriction");

        assert_eq!(
            parsed.restriction,
            Restriction::cast_spells(PlayerFilter::IteratedPlayer)
        );
    }
}

pub(crate) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::cards::builders::parser::token_word_refs(first);
    let Some(mut idx) = find_index(&first_words, |word| matches!(*word, "choose" | "chooses"))
    else {
        return Ok(None);
    };
    idx += 1;
    if first_words.get(idx).is_some_and(|word| is_article(word)) {
        idx += 1;
    }
    if first_words.get(idx) != Some(&"card") || first_words.get(idx + 1) != Some(&"type") {
        return Ok(None);
    }
    idx += 2;

    let reveal_words = &first_words[idx..];
    if !slice_starts_with(&reveal_words, &["then", "reveal", "the", "top"]) {
        return Ok(None);
    }
    let reveal_tokens = reveal_words[4..]
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&reveal_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing reveal count in choose-card-type reveal clause (clause: '{}')",
            first_words.join(" ")
        ))
    })?;
    if reveal_tokens
        .get(used)
        .and_then(OwnedLexToken::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in choose-card-type reveal clause (clause: '{}')",
            first_words.join(" ")
        )));
    }
    let reveal_tail = crate::cards::builders::parser::token_word_refs(&reveal_tokens[used + 1..]);
    if !slice_ends_with(&reveal_tail, &["of", "your", "library"]) {
        return Ok(None);
    }

    let second_words = crate::cards::builders::parser::token_word_refs(second);
    if !matches!(second_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let has_chosen_type = contains_word_sequence(&second_words, &["chosen", "type"]);
    let has_revealed_this_way = contains_word_sequence(&second_words, &["revealed", "this", "way"]);
    let has_into_your_hand = contains_word_sequence(&second_words, &["into", "your", "hand"]);
    let has_bottom_of_library =
        contains_word_sequence(&second_words, &["bottom", "of", "your", "library"]);
    if !has_chosen_type || !has_revealed_this_way || !has_into_your_hand || !has_bottom_of_library {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RevealTopChooseCardTypePutToHandRestBottom {
            player: PlayerAst::You,
            count,
        },
    ]))
}

pub(crate) fn parse_choose_creature_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<Subtype>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if words.get(idx) != Some(&"creature") || words.get(idx + 1) != Some(&"type") {
        return Ok(None);
    }
    idx += 2;

    let mut excluded_subtypes = Vec::new();
    if words.get(idx) == Some(&"other") && words.get(idx + 1) == Some(&"than") {
        let subtype_word = words.get(idx + 2).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing creature subtype exclusion in creature-type choice clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| str_strip_suffix(subtype_word, "s").and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported creature subtype exclusion in creature-type choice clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
        excluded_subtypes.push(subtype);
        idx += 3;
    }

    Ok(Some((idx, excluded_subtypes)))
}

fn parse_choose_phrase_prefix_words(words: &[&str]) -> Option<usize> {
    if words.is_empty() || !matches!(words[0], "choose" | "chooses") {
        return None;
    }

    let mut idx = 1usize;
    if words.get(idx).is_some_and(|word| is_article(word)) {
        idx += 1;
    }
    Some(idx)
}

pub(crate) fn parse_choose_color_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Option<ColorSet>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if words.get(idx) != Some(&"color") {
        return Ok(None);
    }
    idx += 1;

    let mut excluded = None;
    if words.get(idx) == Some(&"other") && words.get(idx + 1) == Some(&"than") {
        let color_word = words.get(idx + 2).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        excluded = Some(parse_color(color_word).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported color exclusion in choose-color clause (clause: '{}')",
                words.join(" ")
            ))
        })?);
        idx += 3;
    }

    Ok(Some((idx, excluded)))
}

pub(crate) fn parse_choose_card_type_phrase_words(
    words: &[&str],
) -> Result<Option<(usize, Vec<CardType>)>, CardTextError> {
    let Some(mut idx) = parse_choose_phrase_prefix_words(words) else {
        return Ok(None);
    };
    if words.get(idx) == Some(&"card") && words.get(idx + 1) == Some(&"type") {
        return Ok(Some((idx + 2, Vec::new())));
    }
    if words.get(idx) == Some(&"permanent")
        && matches!(words.get(idx + 1).copied(), Some("type" | "types"))
    {
        return Ok(Some((
            idx + 2,
            vec![
                CardType::Artifact,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Battle,
            ],
        )));
    }

    let mut options = Vec::new();
    let mut consumed_any = false;
    while let Some(word) = words.get(idx).copied() {
        if matches!(word, "or" | "and") {
            idx += 1;
            continue;
        }
        let Some(card_type) = parse_card_type(word) else {
            break;
        };
        if !options.contains(&card_type) {
            options.push(card_type);
        }
        consumed_any = true;
        idx += 1;
    }

    if !consumed_any {
        return Ok(None);
    }

    Ok(Some((idx, options)))
}

pub(crate) fn parse_choose_player_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if words.get(idx) != Some(&"player") {
        return None;
    }
    idx += 1;
    Some(idx)
}

pub(crate) fn parse_choose_basic_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if words.get(idx) != Some(&"basic")
        || words.get(idx + 1) != Some(&"land")
        || words.get(idx + 2) != Some(&"type")
    {
        return None;
    }
    idx += 3;
    Some(idx)
}

pub(crate) fn parse_choose_land_type_phrase_words(words: &[&str]) -> Option<usize> {
    let mut idx = parse_choose_phrase_prefix_words(words)?;
    if words.get(idx) != Some(&"land") || words.get(idx + 1) != Some(&"type") {
        return None;
    }
    idx += 2;
    Some(idx)
}

pub(crate) fn parse_choose_creature_type_then_become_type(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = crate::cards::builders::parser::token_word_refs(&first_tokens);
    let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(&first_words)?
    else {
        return Ok(None);
    };
    if consumed != first_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-type choice clause (clause: '{}')",
            first_words.join(" ")
        )));
    }

    let second_words = crate::cards::builders::parser::token_word_refs(second);
    let Some(become_idx) = find_index(second, |token| {
        token.is_word("become") || token.is_word("becomes")
    }) else {
        return Ok(None);
    };
    if become_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&second[..become_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in creature-type become clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let become_tail_tokens = trim_commas(&second[become_idx + 1..]);
    let (duration, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(&become_tail_tokens)? {
            (duration, remainder)
        } else {
            (Until::Forever, become_tail_tokens.to_vec())
        };
    let become_words = crate::cards::builders::parser::token_word_refs(&become_tokens);
    if become_words.as_slice() != ["that", "type"] {
        return Ok(None);
    }

    let subject_words = crate::cards::builders::parser::token_word_refs(&subject_tokens);
    let target = if slice_starts_with(&subject_words, &["each"])
        || slice_starts_with(&subject_words, &["all"])
    {
        let filter_tokens = trim_commas(&subject_tokens[1..]);
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            )));
        }
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in creature-type become clause (clause: '{}')",
                second_words.join(" ")
            ))
        })?;
        TargetAst::Object(filter, span_from_tokens(&subject_tokens), None)
    } else {
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(vec![EffectAst::BecomeCreatureTypeChoice {
        target,
        duration,
        excluded_subtypes,
    }]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(and_idx) = find_index(tokens, |token| token.is_word("and")) else {
        return Ok(None);
    };
    let first_clause = trim_commas(&tokens[..and_idx]);
    let second_clause = trim_commas(&tokens[and_idx + 1..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = crate::cards::builders::parser::token_word_refs(&second_clause);
    if !matches!(second_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let Some(on_idx) = find_index(&second_clause, |token: &OwnedLexToken| token.is_word("on"))
    else {
        return Ok(None);
    };
    if !second_clause
        .get(on_idx + 1)
        .is_some_and(|token| token.is_word("top"))
        || !second_clause
            .get(on_idx + 2)
            .is_some_and(|token| token.is_word("of"))
    {
        return Ok(None);
    }
    let destination_words =
        crate::cards::builders::parser::token_word_refs(&second_clause[on_idx + 3..]);
    if !slice_contains(&destination_words, &"library") {
        return Ok(None);
    }

    let moved_tokens = trim_commas(&second_clause[1..on_idx]);
    let moved_words = crate::cards::builders::parser::token_word_refs(&moved_tokens);
    let target = if moved_tokens.is_empty()
        || moved_words.as_slice() == ["it"]
        || moved_words.as_slice() == ["them"]
        || moved_words.as_slice() == ["those"]
        || moved_words.as_slice() == ["those", "cards"]
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause))
    } else {
        parse_target_phrase(&moved_tokens)?
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
    ]))
}

pub(crate) fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = find_window_by(tokens, 2, |window| {
        window[0].is_comma() && window[1].is_word("then")
    })
    .map(|idx| (idx, idx + 2))
    .or_else(|| {
        find_index(tokens, |token| token.is_word("then"))
            .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
    });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let first_clause = trim_commas(&tokens[..head_end]);
    let second_clause = trim_commas(&tokens[tail_start..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = crate::cards::builders::parser::token_word_refs(&second_clause);
    if second_words.len() < 4
        || second_words[0] != "you"
        || !matches!(second_words[1], "put" | "puts")
    {
        return Ok(None);
    }

    let Some(onto_idx) = find_index(&second_clause, |token: &OwnedLexToken| {
        token.is_word("onto")
    }) else {
        return Ok(None);
    };
    if onto_idx < 2 {
        return Ok(None);
    }

    let moved_words = crate::cards::builders::parser::token_word_refs(&second_clause[2..onto_idx]);
    let moved_is_tagged_choice = moved_words == ["it"]
        || moved_words == ["that", "card"]
        || moved_words == ["that", "permanent"];
    if !moved_is_tagged_choice {
        return Ok(None);
    }

    let destination_words: Vec<&str> =
        crate::cards::builders::parser::token_word_refs(&second_clause[onto_idx + 1..])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
    if destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }
    let mut destination_tail: Vec<&str> = destination_words[1..].to_vec();
    let battlefield_tapped = slice_contains(&destination_tail, &"tapped");
    destination_tail.retain(|word| *word != "tapped");
    let battlefield_controller = if destination_tail.as_slice() == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail.is_empty() {
        ReturnControllerAst::Preserve
    } else if destination_tail.as_slice() == ["under", "its", "owners", "control"]
        || destination_tail.as_slice() == ["under", "their", "owners", "control"]
        || destination_tail.as_slice() == ["under", "that", "players", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause)),
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
            battlefield_tapped,
            attached_to: None,
        },
    ]))
}

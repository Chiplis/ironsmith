use super::*;
use crate::ZoneReplacementDurationAst;
use crate::runtime_backend::GrantedAbilityAst;
use crate::runtime_backend::ast::{ChooseOneModeAst, SubjectVerbEffectAst, SubjectVerbSubjectAst};
use crate::runtime_backend::grammar::abilities::{
    is_minimum_spell_total_mana_three_line_lexed, is_players_cant_pay_life_or_sacrifice_line_lexed,
};
use crate::runtime_backend::grammar::keyword_special_lines as keyword_special_grammar;
use crate::runtime_backend::grammar::semantic_lowering as semantic_grammar;
use crate::runtime_backend::grammar::structure::{
    StatementLineFamily, classify_statement_line_family_lexed,
};
use crate::{KeywordAction, Value};

fn restore_copy_static_variant_source_display(
    abilities: &mut [crate::cards::builders::StaticAbilityAst],
    raw_line: &str,
) {
    let matching_count = abilities
        .iter()
        .filter_map(|ability| {
            let crate::cards::builders::StaticAbilityAst::Static(ability) = ability else {
                return None;
            };
            matches!(
                &ability.payload,
                crate::static_abilities::StaticAbilityPayload::CopyStaticAbilityVariants(_)
            )
            .then_some(())
        })
        .count();
    if matching_count != 1 {
        return;
    }

    let display = raw_line.trim();
    if display.is_empty() {
        return;
    }
    for ability in abilities {
        let crate::cards::builders::StaticAbilityAst::Static(ability) = ability else {
            continue;
        };
        let crate::static_abilities::StaticAbilityPayload::CopyStaticAbilityVariants(copy) =
            &mut ability.payload
        else {
            continue;
        };
        copy.display = display.to_string();
        ability.label = display.to_string();
    }
}

fn full_parse_tokens_have_triggered_intervening_if_clause(tokens: &[OwnedLexToken]) -> bool {
    let start_idx =
        if super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(tokens)
            .is_some()
        {
            1
        } else {
            0
        };

    super::super::grammar::structure::split_triggered_conditional_clause_lexed(tokens, start_idx)
        .is_some()
}

fn triggered_line_source_text(line: &RewriteTriggeredLine) -> String {
    let raw = line.info.raw_line.trim();
    let full = line.full_text.trim();
    if raw != full
        && raw_preserves_triggered_source(&line.info.source_tokens, &line.full_parse_tokens)
    {
        raw.to_string()
    } else {
        full.to_string()
    }
}

fn wrap_future_draw_replacement_effects(
    full_parse_tokens: &[OwnedLexToken],
    effects: Vec<EffectAst>,
) -> Vec<EffectAst> {
    let Some(player) =
        semantic_grammar::parse_next_draw_replacement_player_tokens(full_parse_tokens)
    else {
        return effects;
    };
    if effects.is_empty() {
        return effects;
    }

    vec![EffectAst::subject_verb_register_draw_replacement(
        player,
        effects,
        ZoneReplacementDurationAst::OneShot,
    )]
}

fn raw_preserves_triggered_source(
    raw_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> bool {
    raw_label_prefix_preserves_triggered_source(raw_tokens, full_tokens)
        || normalized_triggered_source_words_from_tokens(raw_tokens)
            == normalized_triggered_source_words_from_tokens(full_tokens)
}

fn raw_label_prefix_preserves_triggered_source(
    raw_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
) -> bool {
    let Some((_, body_tokens)) = raw_label_prefix_parts(raw_tokens) else {
        return false;
    };
    normalized_triggered_source_words_from_tokens(&body_tokens)
        == normalized_triggered_source_words_from_tokens(full_tokens)
}

fn raw_label_prefix_parts(tokens: &[OwnedLexToken]) -> Option<(String, Vec<OwnedLexToken>)> {
    let split = semantic_grammar::parse_trigger_label_split_tokens(tokens)?;
    let label_tokens = split.label_tokens;
    let body_tokens = split.body_tokens;
    if !label_tokens_form_raw_trigger_label(label_tokens) {
        return None;
    }

    let body_tokens = trim_lexed_commas(body_tokens);
    if super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(body_tokens)
        .is_none()
    {
        return None;
    }

    Some((
        render_token_slice(label_tokens).trim().to_string(),
        body_tokens.to_vec(),
    ))
}

fn label_tokens_form_raw_trigger_label(label_tokens: &[OwnedLexToken]) -> bool {
    let label = render_token_slice(label_tokens);
    !label.trim().is_empty()
        && label.len() <= 40
        && !label_tokens
            .iter()
            .any(|token| matches!(token.kind, TokenKind::Period | TokenKind::Colon))
}

fn normalized_triggered_source_words_from_tokens(tokens: &[OwnedLexToken]) -> Vec<String> {
    semantic_grammar::normalized_trigger_source_words_tokens(tokens)
}

pub(crate) fn parse_statement_token_groups_to_chunks(
    info: LineInfo,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    parse_statement_to_chunks_impl(
        &RewriteStatementLine {
            info,
            parse_tokens: parse_tokens.to_vec(),
        },
        parse_tokens,
        parse_groups,
    )
}

fn parse_statement_to_chunks_impl(
    line: &RewriteStatementLine,
    parse_tokens: &[OwnedLexToken],
    parse_groups: &[Vec<OwnedLexToken>],
) -> Result<Vec<LineAst>, CardTextError> {
    if let Some(chunk) = parse_villainous_choice_statement_chunk(line)? {
        return Ok(vec![chunk]);
    }
    if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(parse_tokens) {
        return Ok(vec![chunk]);
    }
    if effect_grammar::parse_kicked_counter_replacement_tokens(parse_tokens).is_some() {
        let effects = parse_effect_sentences_preserving_source_boundaries(parse_tokens)?;
        return Ok(vec![LineAst::Statement { effects }]);
    }
    if !parse_groups.is_empty() {
        if sentences_form_anaphoric_damage_self_replacement(parse_groups) {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if parse_groups.len() > 1
            && sentences_have_token_creation_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if parse_groups.len() > 1
            && sentences_have_temporary_static_followup_after_first(parse_groups)
        {
            let group_tokens = join_sentences_with_period(parse_groups);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        let mut chunks = Vec::with_capacity(parse_groups.len());
        for group_tokens in parse_groups {
            if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(group_tokens)
            {
                chunks.push(chunk);
            } else if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(group_tokens)
            {
                chunks.push(chunk);
            } else if statement_group_should_parse_as_effects_first(group_tokens) {
                let effects = parse_effect_sentences_preserving_source_boundaries(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(group_tokens) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(group_tokens)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                let effects = parse_effect_sentences_preserving_source_boundaries(group_tokens)?;
                chunks.push(LineAst::Statement { effects });
            }
        }
        return Ok(chunks);
    }
    if !parse_tokens.is_empty() {
        let statement_grouping =
            crate::runtime_backend::grammar::statement_grouping::parse_statement_grouping_tokens(
                parse_tokens,
            );
        let sentence_tokens = statement_grouping.sentences;
        let grouped_tokens = statement_grouping.groups;
        let keep_linked_statement_grouped = linked_statement_should_stay_grouped(parse_tokens);
        if keep_linked_statement_grouped {
            let group_tokens = join_sentences_with_period(&sentence_tokens);
            let effects = parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
            return Ok(vec![LineAst::Statement { effects }]);
        }
        if !keep_linked_statement_grouped
            && sentence_tokens.len() > 1
            && !sentences_have_token_creation_followup_after_first(&sentence_tokens)
            && !sentences_have_temporary_static_followup_after_first(&sentence_tokens)
            && sentence_tokens.iter().any(|sentence| {
                parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                    || parse_day_night_starts_day_static_chunk(sentence).is_some()
                    || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
            })
        {
            let mut chunks = Vec::with_capacity(sentence_tokens.len());
            for sentence in sentence_tokens {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(chunk) = parse_die_roll_result_adjustment_static_chunk(&sentence)
                {
                    chunks.push(chunk);
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects = parse_effect_sentences_preserving_source_boundaries(&sentence)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(chunks);
        }
        if !grouped_tokens.is_empty() {
            let mut chunks = Vec::with_capacity(grouped_tokens.len());
            for group_tokens in grouped_tokens {
                if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(chunk) =
                    parse_die_roll_result_adjustment_static_chunk(&group_tokens)
                {
                    chunks.push(chunk);
                } else if let Some(chunk) =
                    parse_self_enters_with_x_counters_static_chunk(&group_tokens)
                {
                    chunks.push(chunk);
                } else if statement_group_should_parse_as_effects_first(&group_tokens) {
                    let effects =
                        parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                } else if let Some(chunk) = parse_day_night_starts_day_static_chunk(&group_tokens) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(&group_tokens)?
                {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    let effects =
                        parse_effect_sentences_preserving_source_boundaries(&group_tokens)?;
                    chunks.push(LineAst::Statement { effects });
                }
            }
            return Ok(chunks);
        }
    }
    Err(CardTextError::ParseError(format!(
        "rewrite statement lowering expected prepared parse tokens for '{}'",
        line.info.raw_line
    )))
}

fn parse_villainous_choice_mode_program(
    program: semantic_grammar::VillainousChoiceModeProgram<'_>,
) -> Result<Vec<EffectAst>, CardTextError> {
    match program {
        semantic_grammar::VillainousChoiceModeProgram::Direct(tokens) => {
            parse_effect_sentences_lexed(tokens)
        }
        semantic_grammar::VillainousChoiceModeProgram::SharedSubjectPair(pair) => {
            let parse_action = |action_tokens: &[OwnedLexToken]| {
                let mut clause = Vec::with_capacity(
                    pair.subject_tokens
                        .len()
                        .saturating_add(action_tokens.len()),
                );
                clause.extend_from_slice(pair.subject_tokens);
                clause.extend_from_slice(action_tokens);
                parse_effect_sentences_lexed(&clause)
            };
            let mut effects = parse_action(pair.first_action_tokens)?;
            effects.extend(parse_action(pair.second_action_tokens)?);
            Ok(effects)
        }
    }
}

fn render_statement_source_tokens(
    line: &RewriteStatementLine,
    parsed_tokens: &[OwnedLexToken],
) -> String {
    let Some(first) = parsed_tokens.first() else {
        return String::new();
    };
    let Some(last) = parsed_tokens.last() else {
        return String::new();
    };
    line.info
        .raw_line
        .get(first.span.start..last.span.end)
        .map(str::to_string)
        .unwrap_or_else(|| render_token_slice(parsed_tokens))
}

fn parse_villainous_choice_statement_chunk(
    line: &RewriteStatementLine,
) -> Result<Option<LineAst>, CardTextError> {
    let Some(shape) =
        semantic_grammar::parse_villainous_choice_statement_tokens(&line.parse_tokens)
    else {
        return Ok(None);
    };
    let target_tag = TagKey::from(IT_TAG);
    let mut effects = match shape.target {
        semantic_grammar::VillainousChoiceTarget::CreaturesYouDontControl => {
            let target = TargetAst::WithCount(
                Box::new(TargetAst::Object(
                    ObjectFilter::creature().controlled_by(PlayerFilter::NotYou),
                    Some(crate::TextSpan::synthetic()),
                    None,
                )),
                shape.count,
            );
            vec![EffectAst::subject_verb_target_only(target)]
        }
    };
    let first_mode_effects = parse_villainous_choice_mode_program(shape.first_mode_program)?;
    let second_mode_effects = parse_villainous_choice_mode_program(shape.second_mode_program)?;
    let player = match shape.chooser {
        semantic_grammar::VillainousChoiceChooser::IteratedCreaturesController => {
            PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged(IT_TAG))
        }
    };
    let iteration_tag = match shape.iteration {
        semantic_grammar::VillainousChoiceIteration::EachOfThem => target_tag,
    };
    effects.push(EffectAst::ForEachTagged {
        tag: iteration_tag,
        effects: vec![EffectAst::VillainousChoice {
            player,
            player_surface: Some(render_token_slice(shape.chooser_tokens)),
            modes: vec![
                ChooseOneModeAst {
                    description: render_statement_source_tokens(line, shape.first_mode_tokens),
                    effects: first_mode_effects,
                },
                ChooseOneModeAst {
                    description: render_statement_source_tokens(line, shape.second_mode_tokens),
                    effects: second_mode_effects,
                },
            ],
        }],
    });

    Ok(Some(LineAst::Statement { effects }))
}

fn parse_die_roll_result_adjustment_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens);
    let normalized = rendered
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if normalized.starts_with("once each turn, you may pay ")
        && normalized.ends_with(" to reroll one or more dice you rolled.")
    {
        let pay_idx = tokens.iter().position(|token| {
            token
                .as_word()
                .is_some_and(|word| word.eq_ignore_ascii_case("pay"))
        })?;
        let mana_cost = super::super::grammar::leaf::parse_leaf_mana_cost_prefix_tokens(
            &tokens[pay_idx + 1..],
        )?
        .cost;
        return Some(LineAst::StaticAbilities(vec![
            crate::cards::builders::StaticAbilityAst::Static(StaticAbility::die_roll_reroll(
                PlayerFilter::You,
                mana_cost,
                true,
                rendered,
            )),
        ]));
    }

    let spec = semantic_grammar::parse_die_roll_adjustment_tokens(tokens)?;
    let life_cost = spec.life_cost;
    let amount = spec.adjustment;
    let display = format!(
        "After you roll a die, you may pay {life_cost} life. If you do, increase or decrease the result by {amount}. Do this only once each turn."
    );
    Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::die_roll_result_adjustment(
                PlayerFilter::You,
                life_cost,
                amount,
                true,
                display,
            ),
        ),
    ]))
}

fn sentences_have_token_copy_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        crate::runtime_backend::sentences::effect_sentences::parse_token_copy_followup_sentence_lexed(
            sentence.as_ref(),
        )
        .is_some()
    })
}

fn sentences_have_token_granted_ability_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        matches!(
            crate::runtime_backend::sentences::effect_sentences::parse_token_granted_ability_followup_sentence_lexed(sentence.as_ref()),
            Ok(Some(_))
        )
    })
}

fn sentences_have_token_creation_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences_have_token_copy_followup_after_first(sentences)
        || sentences_have_token_granted_ability_followup_after_first(sentences)
        || sentences.iter().skip(1).any(|sentence| {
            semantic_grammar::parse_token_characteristic_followup_tokens(sentence.as_ref())
                .is_some()
        })
}

fn sentences_have_temporary_static_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        let sentence = sentence.as_ref();
        semantic_grammar::parse_temporary_static_followup_tokens(sentence).is_some_and(|facts| {
            matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
                || facts.has_negation
        })
    })
}

fn sentences_form_anaphoric_damage_self_replacement(sentences: &[Vec<OwnedLexToken>]) -> bool {
    let [_, replacement] = sentences else {
        return false;
    };
    if !effect_grammar::followup_shapes::is_anaphoric_damage_self_replacement(replacement.as_ref())
    {
        return false;
    }

    let grouped = join_sentences_with_period(sentences);
    parse_effect_sentences_preserving_source_boundaries(&grouped)
        .is_ok_and(|effects| matches!(effects.as_slice(), [EffectAst::SelfReplacement { .. }]))
}

fn sentences_have_bound_characteristic_followup_after_first<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> bool {
    sentences.iter().skip(1).any(|sentence| {
        effect_grammar::labeled_dispatch::parse_passive_color_type_addition_shape(sentence.as_ref())
            .is_some_and(|shape| shape.tagged_subject)
    })
}

fn returned_object_static_followup_start<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> Option<usize> {
    let first_sentence = sentences.first()?;
    if semantic_grammar::parse_returned_object_move_head_tokens(first_sentence.as_ref()).is_none() {
        return None;
    }

    sentences
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(idx, sentence)| {
            let sentence = sentence.as_ref();
            let facts = semantic_grammar::parse_returned_object_followup_tokens(sentence)?;
            (matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
                || facts.has_characteristic_changes())
            .then_some(idx)
        })
}

fn filter_is_exact_tagged_it(filter: &ObjectFilter) -> bool {
    filter == &ObjectFilter::tagged(TagKey::from(IT_TAG))
}

fn push_returned_object_keyword_grant_effect(
    effects: &mut Vec<EffectAst>,
    action: KeywordAction,
    condition: Option<crate::ConditionExpr>,
) {
    let target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    let ability = GrantedAbilityAst::KeywordAction(action);
    let effect = if let Some(condition) = condition {
        EffectAst::subject_verb_grant_abilities_to_target_with_condition(
            target,
            vec![ability],
            Until::Forever,
            condition,
        )
    } else {
        EffectAst::subject_verb_grant_abilities_to_target(target, vec![ability], Until::Forever)
    };
    effects.push(effect);
}

fn returned_object_static_ability_effects(
    ability: crate::cards::builders::StaticAbilityAst,
    effects: &mut Vec<EffectAst>,
) -> bool {
    match ability {
        crate::cards::builders::StaticAbilityAst::KeywordAction(action) => {
            push_returned_object_keyword_grant_effect(effects, action, None);
            true
        }
        crate::cards::builders::StaticAbilityAst::ConditionalKeywordAction {
            action,
            condition,
        } => {
            push_returned_object_keyword_grant_effect(effects, action, Some(condition));
            true
        }
        crate::cards::builders::StaticAbilityAst::GrantKeywordAction {
            filter,
            action,
            condition,
        } if filter_is_exact_tagged_it(&filter) => {
            push_returned_object_keyword_grant_effect(effects, action, condition);
            true
        }
        _ => false,
    }
}

fn returned_object_static_followup_effects<S: AsRef<[OwnedLexToken]>>(
    sentences: &[S],
) -> Result<Option<(usize, Vec<EffectAst>)>, CardTextError> {
    let Some(first_followup_idx) = returned_object_static_followup_start(sentences) else {
        return Ok(None);
    };

    let mut effects = Vec::new();
    for sentence in sentences.iter().skip(first_followup_idx) {
        let sentence = sentence.as_ref();
        let grammar_facts = semantic_grammar::parse_returned_object_followup_tokens(sentence);
        let before_len = effects.len();
        let before_keyword_len = effects.len();
        if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
            for ability in abilities {
                returned_object_static_ability_effects(ability, &mut effects);
            }
        }
        if effects.len() == before_keyword_len
            && let Some(keyword_tokens) = grammar_facts
                .as_ref()
                .and_then(|facts| facts.keyword_tokens)
            && let Some(actions) = parse_ability_line_lexed(keyword_tokens)
        {
            for action in actions {
                push_returned_object_keyword_grant_effect(&mut effects, action, None);
            }
        }
        if let Some(colors) = grammar_facts.as_ref().and_then(|facts| facts.colors) {
            effects.push(EffectAst::subject_verb_add_colors(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                colors,
                Until::Forever,
            ));
        }
        if let Some(subtypes) = grammar_facts
            .as_ref()
            .map(|facts| facts.subtypes.clone())
            .filter(|subtypes| !subtypes.is_empty())
        {
            effects.push(EffectAst::subject_verb_add_subtypes(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                subtypes,
                Until::Forever,
            ));
        }
        if effects.len() == before_len {
            return Ok(None);
        }
    }

    Ok(Some((first_followup_idx, effects)))
}

fn linked_statement_should_stay_grouped(tokens: &[OwnedLexToken]) -> bool {
    let line_family = classify_statement_line_family_lexed(tokens);
    if matches!(
        line_family,
        Some(
            StatementLineFamily::Divvy
                | StatementLineFamily::Emblem
                | StatementLineFamily::PactNextUpkeep
                | StatementLineFamily::ExilePlayCostsMore
        )
    ) {
        return true;
    }

    semantic_grammar::parse_linked_statement_surface_tokens(tokens).is_some()
}

fn statement_group_should_parse_as_effects_first(tokens: &[OwnedLexToken]) -> bool {
    if matches!(
        crate::runtime_backend::families::keyword_static::parse_double_counters_replacement_line(
            tokens,
        ),
        Ok(Some(_))
    ) {
        return false;
    }
    if linked_statement_should_stay_grouped(tokens) {
        return true;
    }
    if matches!(
        classify_statement_line_family_lexed(tokens),
        Some(StatementLineFamily::Vote)
    ) {
        return true;
    }

    if crate::runtime_backend::front_end::grammar::effects::clause_pattern_shapes::parse_keyword_mechanic_tokens(tokens)
        .is_some()
    {
        return true;
    }

    semantic_grammar::parse_statement_effect_preference_tokens(tokens).is_some()
}

fn parse_self_enters_with_x_counters_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    match semantic_grammar::parse_self_counter_entry_tokens(tokens)? {
        semantic_grammar::SelfCounterEntrySpec::Adamant {
            condition,
            predicate_body,
        } => Some(LineAst::StaticAbilities(vec![
            crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::enters_with_counters_if_condition(
                    crate::object::CounterType::PlusOnePlusOne,
                    crate::effect::Value::Fixed(1),
                    condition,
                    predicate_body,
                ),
            ),
        ])),
        semantic_grammar::SelfCounterEntrySpec::Unconditional { count } => {
            Some(LineAst::StaticAbilities(vec![
                crate::cards::builders::StaticAbilityAst::Static(
                    StaticAbility::enters_with_counters_value(
                        crate::object::CounterType::PlusOnePlusOne,
                        count,
                    ),
                ),
            ]))
        }
    }
}

fn spell_cast_trigger_filter(trigger: &TriggerSpec) -> Option<(ObjectFilter, PlayerFilter)> {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => spell_cast_trigger_filter(trigger),
        TriggerSpec::SpellCast {
            filter: Some(filter),
            caster,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        } => Some((filter.clone(), caster.clone())),
        _ => None,
    }
}

fn lower_spell_cast_snow_mana_enter_counter_static_chunk(
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<&PredicateAst>,
) -> Result<Option<LineAst>, CardTextError> {
    let Some(spec) = semantic_grammar::parse_snow_mana_counter_entry_tokens(
        effect_parse_tokens,
        matches!(
            intervening_if,
            Some(PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell)
        ),
    ) else {
        return Ok(None);
    };

    let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
    let Some((mut filter, caster)) = spell_cast_trigger_filter(&trigger) else {
        return Ok(None);
    };
    if !matches!(filter.zone, Some(Zone::Stack))
        || filter.card_types.len() != 1
        || filter.card_types.first().copied() != Some(CardType::Creature)
    {
        return Ok(None);
    }

    filter.zone = Some(Zone::Battlefield);
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    filter.controller = Some(caster);

    let ability = StaticAbility::enters_with_counters_and_subtypes_for_filter(
        filter,
        spec.counter_type,
        spec.count,
        Vec::new(),
    )
    .with_condition(spec.condition);

    Ok(Some(LineAst::StaticAbilities(vec![
        crate::cards::builders::StaticAbilityAst::Static(ability),
    ])))
}

fn parse_day_night_starts_day_static_chunk(tokens: &[OwnedLexToken]) -> Option<LineAst> {
    let rendered = render_token_slice(tokens);
    semantic_grammar::parse_day_night_starts_day_tokens(tokens).map(|_| {
        LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
            StaticAbility::rule_fallback_text(rendered.trim().trim_end_matches('.').to_string()),
        )])
    })
}

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

#[cfg(test)]
pub(crate) fn parse_single_effect_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_sentences_lexed(tokens)?
        .into_iter()
        .next()
        .ok_or_else(|| CardTextError::ParseError("missing effect in lexed sentence".to_string()))
}

#[cfg(test)]
pub(crate) fn strip_lexed_suffix_phrase<'a>(
    tokens: &'a [OwnedLexToken],
    phrase: &[&str],
) -> Option<&'a [OwnedLexToken]> {
    let words = TokenWordView::new(tokens);
    if words.len() < phrase.len() {
        return None;
    }
    let start_word_idx = words.len() - phrase.len();
    if !words.slice_eq(start_word_idx, phrase) {
        return None;
    }
    let token_idx = words.token_boundary_for_word(start_word_idx)?;
    Some(&tokens[..token_idx])
}

pub(crate) fn parse_triggered_line(
    info: LineInfo,
    full_text: &str,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    intervening_if: Option<PredicateAst>,
    presentation: Option<&PresentationLabel>,
    max_triggers_per_turn: Option<u32>,
    chosen_option: Option<&ChosenOptionContext>,
) -> Result<LineAst, CardTextError> {
    parse_triggered_line_impl(
        &RewriteTriggeredLine {
            info,
            full_text: full_text.to_string(),
            full_parse_tokens: full_parse_tokens.to_vec(),
            intervening_if,
            max_triggers_per_turn,
            chosen_option: chosen_option.cloned(),
            presentation: presentation.cloned(),
        },
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )
}

fn parse_triggered_line_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    use crate::runtime_backend::grammar::effects::delayed_sentence_shapes::{
        DelayedScheduleStep, parse_delayed_schedule_sentence_shape,
    };

    let delayed_schedule = parse_delayed_schedule_sentence_shape(full_parse_tokens);
    let parsed = parse_triggered_ability_line_impl(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )?;
    let parsed = preserve_triggered_effect_surfaces(parsed, effect_parse_tokens);
    let Some(schedule) = delayed_schedule else {
        return Ok(parsed);
    };
    let effects = match parsed {
        LineAst::Triggered { effects, .. } => effects,
        LineAst::Ability(parsed)
            if matches!(parsed.kind(), crate::ability::AbilityKind::Triggered(_)) =>
        {
            parsed.effects_ast.ok_or_else(|| {
                CardTextError::InvariantViolation(format!(
                    "delayed schedule ability did not preserve semantic effects: '{}'",
                    line.info.raw_line
                ))
            })?
        }
        _ => {
            return Err(CardTextError::InvariantViolation(format!(
                "delayed schedule sentence did not produce triggered effects: '{}'",
                line.info.raw_line
            )));
        }
    };

    let delayed = match schedule.step {
        DelayedScheduleStep::Upkeep => EffectAst::DelayedUntilNextUpkeep {
            player: schedule.player,
            effects,
        },
        DelayedScheduleStep::DrawStep => EffectAst::DelayedUntilNextDrawStep {
            player: schedule.player,
            effects,
        },
        DelayedScheduleStep::MainPhase => EffectAst::DelayedUntilNextMainPhase {
            player: match schedule.player {
                PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                PlayerAst::That => PlayerFilter::IteratedPlayer,
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => PlayerFilter::Any,
            },
            effects,
        },
        DelayedScheduleStep::EndStep if schedule.start_next_turn => {
            EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: schedule.player,
                effects,
            }
        }
        DelayedScheduleStep::EndStep => EffectAst::DelayedUntilNextEndStep {
            player: match schedule.player {
                PlayerAst::You | PlayerAst::Implicit => PlayerFilter::You,
                PlayerAst::That => PlayerFilter::IteratedPlayer,
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => PlayerFilter::Any,
            },
            effects,
        },
    };
    Ok(LineAst::Statement {
        effects: vec![delayed],
    })
}

fn preserve_triggered_effect_surfaces(
    mut parsed: LineAst,
    effect_parse_tokens: &[OwnedLexToken],
) -> LineAst {
    let Ok(surfaced) = parse_effect_sentences_preserving_source_boundaries(effect_parse_tokens)
    else {
        return parsed;
    };
    fn without_source_sentence_markers(effects: &[EffectAst]) -> Vec<EffectAst> {
        let mut flattened = Vec::new();
        for effect in effects {
            match effect {
                EffectAst::SourceSentence { effects } => {
                    flattened.extend(without_source_sentence_markers(effects));
                }
                effect => flattened.push(effect.clone()),
            }
        }
        flattened
    }
    fn without_surface_markers(effects: &[EffectAst]) -> Vec<EffectAst> {
        let mut flattened = Vec::new();
        for effect in effects {
            match effect {
                EffectAst::SourceSentence { effects } | EffectAst::Coordinated { effects, .. } => {
                    flattened.extend(without_surface_markers(effects));
                }
                effect => flattened.push(effect.clone()),
            }
        }
        flattened
    }
    let sentence_flattened = without_source_sentence_markers(&surfaced);
    let flattened = without_surface_markers(&surfaced);
    if surfaced == flattened {
        return parsed;
    }

    fn replace_matching_effects(
        parsed: &mut LineAst,
        sentence_flattened: &[EffectAst],
        flattened: &[EffectAst],
        surfaced: &[EffectAst],
    ) -> bool {
        match parsed {
            LineAst::Triggered { effects, .. }
                if effects.as_slice() == sentence_flattened || effects.as_slice() == flattened =>
            {
                *effects = surfaced.to_vec();
                true
            }
            LineAst::Ability(parsed)
                if parsed.effects_ast.as_deref() == Some(sentence_flattened)
                    || parsed.effects_ast.as_deref() == Some(flattened) =>
            {
                parsed.effects_ast = Some(surfaced.to_vec());
                true
            }
            LineAst::Multiple(chunks) => chunks.iter_mut().any(|chunk| {
                replace_matching_effects(chunk, sentence_flattened, flattened, surfaced)
            }),
            _ => false,
        }
    }

    replace_matching_effects(&mut parsed, &sentence_flattened, &flattened, &surfaced);
    parsed
}

fn full_text_has_non_mana_activated_ability_qualifier(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    words.windows(6).any(|window| {
        window[0] == "if"
            && window[1] == "it"
            && matches!(window[2], "isnt" | "isn't" | "not")
            && window[3] == "a"
            && window[4] == "mana"
            && window[5] == "ability"
    })
}

fn mark_non_mana_activated_trigger(trigger: &mut TriggerSpec) {
    match trigger {
        TriggerSpec::AbilityActivated { non_mana_only, .. } => *non_mana_only = true,
        TriggerSpec::WithIntro { trigger, .. } => mark_non_mana_activated_trigger(trigger),
        TriggerSpec::Either(left, right) => {
            mark_non_mana_activated_trigger(left);
            mark_non_mana_activated_trigger(right);
        }
        _ => {}
    }
}

fn mark_non_mana_activated_line(line: &mut LineAst) {
    match line {
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                mark_non_mana_activated_line(chunk);
            }
        }
        LineAst::Triggered { trigger, .. } => mark_non_mana_activated_trigger(trigger),
        _ => {}
    }
}

fn parse_triggered_ability_line_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let source_text = triggered_line_source_text(line);
    let source_text_tokens = if source_text.trim() == line.info.raw_line.trim() {
        line.info.source_tokens.as_slice()
    } else {
        full_parse_tokens
    };
    let source_intro = super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(
        &source_text_tokens,
    );
    let full_intro = super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(
        full_parse_tokens,
    );
    let trigger_surface_text = if source_intro.is_some() || full_intro.is_none() {
        source_text.as_str()
    } else {
        line.full_text.trim()
    };
    let mut trigger_facts = line.info.semantic_facts.triggered_ability.clone();
    if let Some(intro_surface) =
        super::super::grammar::trigger_surface::parse_trigger_intro_surface_tokens(
            full_parse_tokens,
        )
    {
        // A physical Oracle line can contain more than one triggered sentence.
        // Each prepared chunk owns its own introduction; do not inherit the
        // first sentence's `When`/`Whenever` surface from line-level facts.
        trigger_facts.intro_surface = Some(intro_surface);
    }
    let trigger_facts = &trigger_facts;
    let chosen_option = line.chosen_option.as_ref();
    let presentation_label = line.presentation.as_ref();
    let inferred_max_triggers_per_turn = line.max_triggers_per_turn;
    let full_text_facts = semantic_grammar::parse_triggered_text_facts_tokens(full_parse_tokens);
    let effect_text_facts =
        semantic_grammar::parse_triggered_text_facts_tokens(effect_parse_tokens);

    if let Some(chunk) = parse_linked_attack_group_combat_triggered_line_lexed(full_parse_tokens)? {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    if let Some(chunk) = parse_special_triggered_line(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )? {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    if full_text_facts.has_full_party_instead
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let effect_tokens;
        if effect_text_facts.has_full_party_condition {
            effect_tokens = effect_parse_tokens;
        } else {
            effect_tokens = semantic_grammar::parse_comma_split_tokens(full_parse_tokens)
                .map(|split| split.after)
                .unwrap_or(effect_parse_tokens);
        }
        let effects = parse_effect_sentences_lexed(effect_tokens)?;
        if !effects.is_empty() {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                trigger_facts,
                inferred_max_triggers_per_turn,
                chosen_option,
                presentation_label,
            );
        }
    }

    let selected_effect_sentences = split_lexed_sentences(effect_parse_tokens);
    let selected_effect_has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&selected_effect_sentences);
    let selected_effect_has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&selected_effect_sentences);
    let selected_effect_has_bound_characteristic_followup_after_first =
        sentences_have_bound_characteristic_followup_after_first(&selected_effect_sentences);
    let selected_effect_has_counter_linked_land_subtype_followup_after_first =
        selected_effect_sentences.iter().skip(1).any(|sentence| {
            super::super::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(sentence)
                .is_some()
        });
    if let Some((first_followup_idx, mut followup_effects)) =
        returned_object_static_followup_effects(&selected_effect_sentences)?
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let trigger_effect_sentences = selected_effect_sentences[..first_followup_idx]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let trigger_effect_tokens = join_sentences_with_period(&trigger_effect_sentences);
        if let Ok(parsed_effects) = parse_effect_sentences_lexed(&trigger_effect_tokens) {
            let mut effects =
                wrap_future_draw_replacement_effects(full_parse_tokens, parsed_effects);
            if !effects.is_empty() {
                effects.append(&mut followup_effects);
                return apply_chosen_option_to_triggered_chunk(
                    apply_explicit_intervening_if_to_triggered_chunk(
                        LineAst::Triggered {
                            trigger,
                            effects,
                            max_triggers_per_turn: inferred_max_triggers_per_turn,
                        },
                        line.intervening_if.clone(),
                    )?,
                    trigger_surface_text,
                    trigger_facts,
                    inferred_max_triggers_per_turn,
                    chosen_option,
                    presentation_label,
                );
            }
        }
    }
    let selected_split_has_trailing_static_after_first = selected_effect_sentences.len() > 1
        && !selected_effect_has_token_creation_followup_after_first
        && !selected_effect_has_temporary_static_followup_after_first
        && !selected_effect_has_bound_characteristic_followup_after_first
        && selected_effect_sentences
            .iter()
            .enumerate()
            .skip(1)
            .any(|(_, sentence)| {
                parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                    || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))
            });

    let full_sentences = split_lexed_sentences(full_parse_tokens);
    let has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&full_sentences);
    let has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&full_sentences);
    let has_bound_characteristic_followup_after_first =
        sentences_have_bound_characteristic_followup_after_first(&full_sentences);
    if full_sentences.len() > 1
        && !has_token_creation_followup_after_first
        && !has_temporary_static_followup_after_first
        && !has_bound_characteristic_followup_after_first
        && !selected_effect_has_counter_linked_land_subtype_followup_after_first
        && !selected_split_has_trailing_static_after_first
        && let Ok(first_triggered) = parse_triggered_line_lexed(full_sentences[0])
    {
        let mut chunks = Vec::with_capacity(full_sentences.len());
        chunks.push(apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                first_triggered,
                line.intervening_if.clone(),
            )?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option.clone(),
            presentation_label,
        )?);

        let mut parsed_all_static = true;
        for sentence in full_sentences.iter().skip(1) {
            if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                chunks.push(chunk);
            } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                chunks.push(LineAst::StaticAbilities(abilities));
            } else {
                parsed_all_static = false;
                break;
            }
        }
        if parsed_all_static {
            return Ok(LineAst::Multiple(chunks));
        }
    }

    let effect_sentences = split_lexed_sentences(effect_parse_tokens);
    let effect_has_token_creation_followup_after_first =
        sentences_have_token_creation_followup_after_first(&effect_sentences);
    let effect_has_temporary_static_followup_after_first =
        sentences_have_temporary_static_followup_after_first(&effect_sentences);
    let effect_has_bound_characteristic_followup_after_first =
        sentences_have_bound_characteristic_followup_after_first(&effect_sentences);
    let effect_is_linked_typed_bundle =
        crate::runtime_backend::effect_sentences::parse_typed_effect_bundle_lexed(
            effect_parse_tokens,
        )
        .is_some();
    if effect_sentences.len() > 1
        && !effect_has_token_creation_followup_after_first
        && !effect_has_temporary_static_followup_after_first
        && !effect_has_bound_characteristic_followup_after_first
        && !selected_effect_has_counter_linked_land_subtype_followup_after_first
        // A sentence that looks static in isolation may modify the exact
        // exiled card established by the preceding resolution instructions.
        // Keep any complete typed bundle together so its linked target and
        // duration survive into the triggered ability instead of becoming a
        // top-level battlefield static ability.
        && !effect_is_linked_typed_bundle
        && let Some(first_static_idx) =
            effect_sentences
                .iter()
                .enumerate()
                .skip(1)
                .find_map(|(idx, sentence)| {
                    (parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                        || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
                    .then_some(idx)
                })
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
    {
        let trigger_effect_sentences = effect_sentences[..first_static_idx]
            .iter()
            .map(|sentence| sentence.to_vec())
            .collect::<Vec<_>>();
        let trigger_effect_tokens = join_sentences_with_period(&trigger_effect_sentences);
        let effects = wrap_future_draw_replacement_effects(
            full_parse_tokens,
            parse_effect_sentences_lexed(&trigger_effect_tokens)?,
        );
        if !effects.is_empty() {
            let mut chunks = Vec::new();
            chunks.push(apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                trigger_facts,
                inferred_max_triggers_per_turn,
                chosen_option.clone(),
                presentation_label,
            )?);

            for sentence in effect_sentences.iter().skip(first_static_idx) {
                if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
                    chunks.push(chunk);
                } else if let Some(abilities) = parse_static_ability_ast_line_lexed(sentence)? {
                    chunks.push(LineAst::StaticAbilities(abilities));
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "could not parse trailing static sentence in triggered line '{}'",
                        line.info.raw_line
                    )));
                }
            }
            return Ok(LineAst::Multiple(chunks));
        }
    }

    if !token_word_refs(effect_parse_tokens).is_empty()
        && !full_parse_tokens_have_triggered_intervening_if_clause(full_parse_tokens)
        && !full_text_facts.has_if_you_do
        && !full_text_facts.has_if_you_dont
        && !effect_text_facts.starts_with_if
    {
        let direct_trigger = parse_trigger_clause_lexed(trigger_parse_tokens).map(|mut trigger| {
            if full_text_has_non_mana_activated_ability_qualifier(full_parse_tokens) {
                mark_non_mana_activated_trigger(&mut trigger);
            }
            trigger
        });
        let direct_effects =
            parse_effect_sentences_preserving_source_boundaries(effect_parse_tokens)
                .map(|effects| wrap_future_draw_replacement_effects(full_parse_tokens, effects));
        if let (Ok(trigger), Ok(effects)) = (direct_trigger, direct_effects)
            && !effects.is_empty()
        {
            return apply_chosen_option_to_triggered_chunk(
                apply_explicit_intervening_if_to_triggered_chunk(
                    LineAst::Triggered {
                        trigger,
                        effects,
                        max_triggers_per_turn: inferred_max_triggers_per_turn,
                    },
                    line.intervening_if.clone(),
                )?,
                trigger_surface_text,
                trigger_facts,
                inferred_max_triggers_per_turn,
                chosen_option,
                presentation_label,
            );
        }
    }

    let mut parsed = apply_explicit_intervening_if_to_triggered_chunk(
        parse_triggered_line_lexed(full_parse_tokens)?,
        line.intervening_if.clone(),
    )?;
    if full_text_has_non_mana_activated_ability_qualifier(full_parse_tokens) {
        mark_non_mana_activated_line(&mut parsed);
    }
    apply_chosen_option_to_triggered_chunk(
        parsed,
        trigger_surface_text,
        trigger_facts,
        inferred_max_triggers_per_turn,
        chosen_option,
        presentation_label,
    )
}

#[test]
fn source_sentence_boundaries_preserve_jointly_parsed_reference_flow() {
    let independent = lex_line(
        "Put a +1/+1 counter on this creature. Each opponent loses 1 life.",
        0,
    )
    .expect("Aatchik-style effects should lex");
    let independent = parse_effect_sentences_preserving_source_boundaries(&independent)
        .expect("Aatchik-style effects should parse");
    assert_eq!(independent.len(), 2, "{independent:#?}");
    assert!(
        independent
            .iter()
            .all(|effect| matches!(effect, EffectAst::SourceSentence { .. })),
        "independent direct sentences should retain their authored boundary: {independent:#?}"
    );

    let linked = "Reveal the top card of your library and put that card into your hand. You lose life equal to its mana value.";
    let tokens = lex_line(linked, 0).expect("linked trigger effects should lex");
    let effects = parse_effect_sentences_preserving_source_boundaries(&tokens)
        .expect("linked trigger effects should keep their joint parse");
    assert_eq!(effects.len(), 2, "{effects:#?}");
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, EffectAst::SourceSentence { .. })),
        "joint parsing should retain a stable boundary without losing reference flow: {effects:#?}"
    );
}

fn lower_spell_or_activated_ability_x_cost_trigger(
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
    max_triggers_per_turn: Option<u32>,
) -> Result<Option<LineAst>, CardTextError> {
    if semantic_grammar::parse_spell_or_activated_ability_x_cost_trigger_tokens(
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )
    .is_none()
    {
        return Ok(None);
    }

    let mut spell_filter = ObjectFilter::instant_or_sorcery();
    spell_filter.has_x_in_cost = true;
    let mut ability_filter = ObjectFilter::default();
    ability_filter.has_x_in_cost = true;
    Ok(Some(LineAst::Triggered {
        trigger: TriggerSpec::Either(
            Box::new(TriggerSpec::SpellCast {
                filter: Some(spell_filter),
                caster: PlayerFilter::You,
                timing: None,
                during_turn: None,
                min_spells_this_turn: None,
                exact_spells_this_turn: None,
                from_not_hand: false,
            }),
            Box::new(TriggerSpec::AbilityActivated {
                activator: PlayerFilter::You,
                filter: ability_filter,
                non_mana_only: false,
                loyalty_only: false,
                activation_cost_has_tap: None,
            }),
        ),
        effects: parse_effect_sentences_lexed(effect_parse_tokens)?,
        max_triggers_per_turn,
    }))
}

pub(crate) fn parse_special_triggered_line(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = lower_special_rewrite_triggered_head(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) =
        lower_special_rewrite_triggered_divvy(line, trigger_parse_tokens, effect_parse_tokens)?
    {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = lower_special_rewrite_triggered_oath(line, trigger_parse_tokens)? {
        return Ok(Some(chunk));
    }
    lower_special_rewrite_triggered_tail(line, trigger_parse_tokens)
}

fn lower_special_rewrite_triggered_head(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.presentation == Some(PresentationLabel::CaseToSolve) {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects: vec![EffectAst::SolveCase],
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::PreviousTurnCreatureEntryDraw)
    ) {
        let trigger = TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any);
        let effects = vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Implicit,
            SubjectVerbActionAst::Draw {
                count: Value::Fixed(1),
            },
        )];
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::ObjectEnteredBattlefieldLastTurn(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::You)
                        .other(),
                ),
                if_true: effects,
                if_false: Vec::new(),
            }],
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if let Some(_spec) = semantic_grammar::parse_combat_death_blocked_damage_tokens(
        trigger_parse_tokens,
        effect_parse_tokens,
    ) {
        let trigger = TriggerSpec::ThisDies;
        let effects = parse_effect_sentences_lexed(effect_parse_tokens)?;
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    if let Some(chunk) = lower_spell_or_activated_ability_x_cost_trigger(
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
        line.max_triggers_per_turn,
    )? {
        return Ok(Some(chunk));
    }

    if let Some(chunk) = lower_spell_cast_snow_mana_enter_counter_static_chunk(
        trigger_parse_tokens,
        effect_parse_tokens,
        line.intervening_if.as_ref(),
    )? {
        return Ok(Some(chunk));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::SecondSpellSuspend)
    ) {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        let triggering_tag = TagKey::from("triggering");
        let triggering_spell = TargetAst::Tagged(triggering_tag.clone(), None);
        let mut suspend_filter = ObjectFilter::default();
        suspend_filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Suspend);
        let effects = vec![
            EffectAst::subject_verb_copy_spell(
                triggering_spell.clone(),
                Value::Fixed(1),
                PlayerAst::Implicit,
                false,
                false,
                Vec::new(),
            ),
            EffectAst::subject_verb_exile(triggering_spell.clone(), false),
            EffectAst::subject_verb_put_counters(
                crate::object::CounterType::Time,
                Value::Fixed(4),
                triggering_spell.clone(),
                None,
                false,
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::Not(Box::new(PredicateAst::TaggedMatches(
                    triggering_tag,
                    suspend_filter,
                ))),
                if_true: vec![EffectAst::subject_verb_grant_abilities_to_target(
                    triggering_spell,
                    vec![GrantedAbilityAst::KeywordAction(KeywordAction::Marker(
                        "suspend",
                    ))],
                    Until::Forever,
                )],
                if_false: Vec::new(),
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            line.presentation.as_ref(),
            ReferenceImports::default(),
        ))));
    }

    if semantic_grammar::parse_blocks_or_becomes_blocked_first_strike_tokens(full_parse_tokens)
        .is_some()
    {
        let trigger = TriggerSpec::ThisBecomesBlockedByObject(ObjectFilter::creature());
        let effects = if effect_parse_tokens.is_empty() {
            vec![EffectAst::subject_verb_grant_abilities_to_target(
                TargetAst::Tagged(TagKey::from("triggering"), None),
                vec![GrantedAbilityAst::KeywordAction(KeywordAction::FirstStrike)],
                Until::EndOfTurn,
            )]
        } else {
            parse_effect_sentences_lexed(effect_parse_tokens)?
        };
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

fn lower_special_rewrite_triggered_divvy(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::DifferentNamesLibraryDivvy)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::ThisEntersBattlefield
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut effects = if effect_parse_tokens.is_empty() {
            return Err(CardTextError::InvariantViolation(
                "typed library-divvy trigger is missing carried effect tokens".to_string(),
            ));
        } else {
            let grouped = split_lexed_sentences(effect_parse_tokens)
                .into_iter()
                .take(2)
                .map(|sentence| sentence.to_vec())
                .collect::<Vec<_>>();
            parse_effect_sentences_lexed(&join_sentences_with_period(&grouped))?
        };
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

fn lower_special_rewrite_triggered_oath(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentCreatureMajorityConsult)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let revealed_tag = TagKey::from("oath_revealed");
        let creature_tag = TagKey::from("oath_creature");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = None;
        let effects = vec![
            EffectAst::subject_verb_explicit_target_only_for_chooser(
                TargetAst::Player(
                    PlayerFilter::OpponentWithMoreControlledObjectsThan {
                        player: Box::new(PlayerFilter::Active),
                        filter: Box::new(ObjectFilter::creature()),
                    },
                    Some(crate::TextSpan::synthetic()),
                ),
                PlayerAst::Active,
            ),
            EffectAst::MayByPlayer {
                player: PlayerAst::Active,
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::Active,
                        crate::cards::builders::LibraryConsultModeAst::Reveal,
                        creature_card_filter,
                        crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        creature_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(creature_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: revealed_tag,
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(
                                creature_tag.as_str(),
                            ),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Graveyard,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentGraveyardMinorityReturn)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut graveyard_creature_filter = ObjectFilter::creature();
        graveyard_creature_filter.zone = Some(Zone::Graveyard);

        let mut return_filter = graveyard_creature_filter.clone();
        return_filter.owner = Some(PlayerFilter::IteratedPlayer);

        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::AnOpponentHasFewerThanPlayer {
                player: PlayerAst::That,
                filter: graveyard_creature_filter,
            },
            if_true: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![EffectAst::subject_verb_return_to_hand(
                    TargetAst::Object(return_filter, None, None),
                    false,
                )],
            }],
            if_false: Vec::new(),
        }];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    Ok(None)
}

fn lower_special_rewrite_triggered_tail(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(
        semantic_grammar::SpecialTriggeredProgram::RandomDiscardCreatureReturnUnlessLife { life },
    ) = semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens)
    {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::You)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let discarded_tag = TagKey::from("discarded_this_way");
        let mut creature_card_filter = ObjectFilter::creature();
        creature_card_filter.zone = Some(Zone::Graveyard);
        creature_card_filter.owner = Some(PlayerFilter::You);
        let effects = vec![
            EffectAst::subject_verb_discard(
                PlayerAst::You,
                crate::effect::Value::Fixed(1),
                true,
                false,
                None,
                Some(discarded_tag.clone()),
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::You,
                    tag: discarded_tag.clone(),
                    filter: creature_card_filter,
                },
                if_true: vec![EffectAst::UnlessPays {
                    effects: vec![EffectAst::subject_verb_return_to_battlefield(
                        TargetAst::Tagged(discarded_tag, None),
                        false,
                        false,
                        false,
                        ReturnControllerAst::Preserve,
                        None,
                    )],
                    player: PlayerAst::Any,
                    cost: TotalCost::from_cost(Cost::life(life)),
                }],
                if_false: Vec::new(),
            },
        ];
        return Ok(Some(LineAst::Ability(rewrite_parsed_triggered_ability(
            trigger.clone(),
            effects,
            infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &line.info.semantic_facts.triggered_ability.functional_zones,
            ),
            Some(line.info.raw_line.clone()),
            None,
            None,
            ReferenceImports::default(),
        ))));
    }

    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentCombatAttackPile)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfCombat(PlayerFilter::Opponent)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let effects = vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                Until::EndOfTurn,
                None,
            ),
        ];
        return Ok(Some(LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }

    Ok(None)
}

/// Build the display text for the first-equip-cost alternative static ability.
/// Capitalises the leading "you" and strips the trailing period.
fn capitalize_first_equip_cost_alternative_display(tokens: &[OwnedLexToken]) -> String {
    let rendered = render_token_slice(tokens);
    let s = rendered.trim().trim_end_matches('.');
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

pub(crate) fn parse_static_line(
    info: LineInfo,
    parse_tokens: &[OwnedLexToken],
    chosen_option: Option<&ChosenOptionContext>,
) -> Result<LineAst, CardTextError> {
    parse_static_line_impl(
        &RewriteStaticLine {
            info,
            parse_tokens: parse_tokens.to_vec(),
            chosen_option: chosen_option.cloned(),
        },
        parse_tokens,
    )
}

fn parse_static_line_impl(
    line: &RewriteStaticLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let chosen_option = line.chosen_option.as_ref();
    if let Some(prototype) =
        crate::runtime_backend::grammar::abilities::parse_prototype_keyword_tokens(parse_tokens)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::Abilities(vec![KeywordAction::Prototype {
                cost: prototype.cost,
                power_toughness: prototype.power_toughness,
            }]),
            chosen_option,
        );
    }
    if let Some(visible_label) =
        keyword_special_grammar::parse_partner_visible_label_tokens(&line.parse_tokens)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner().with_text(visible_label).into()),
            chosen_option,
        );
    }
    if let Some(variant) = semantic_grammar::parse_partner_variant_label_tokens(&line.parse_tokens)
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::partner_variant(variant.display).into()),
            chosen_option,
        );
    }
    let special_shape = semantic_grammar::parse_static_special_line_tokens(parse_tokens);
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::BlackManaMayBePaidWithLife)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::krrik_black_mana_may_be_paid_with_life().into()),
            chosen_option,
        );
    }
    if is_minimum_spell_total_mana_three_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::minimum_spell_total_mana(3).into()),
            chosen_option,
        );
    }
    if is_players_cant_pay_life_or_sacrifice_line_lexed(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate().into(),
            ),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::BoastTwice)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::boast_twice_each_turn().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DraftRule)
    ) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::draft_rule_text(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::HiddenAgenda)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::hidden_agenda().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DoubleAgenda)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::double_agenda().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AnyNumberNamedDeckConstruction)
    ) {
        let display = render_token_slice(parse_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::deck_construction_rule_text(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::FirstEquipCostAlternative)
    ) {
        let display = capitalize_first_equip_cost_alternative_display(parse_tokens);
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::first_equip_cost_alternative(display).into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::EquipAtInstantSpeed)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::equip_abilities_any_time().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AdditionalVoteTime)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_time_while_voting().into()),
            chosen_option,
        );
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::AdditionalVote)
    ) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::vote_additional_vote_while_voting().into()),
            chosen_option,
        );
    }
    if let Some(count) = semantic_grammar::parse_additional_land_play_count_tokens(parse_tokens) {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(StaticAbility::additional_land_plays(count).into()),
            chosen_option,
        );
    }
    if let Some(chunk) = try_lower_hideaway_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }

    let lexed = parse_tokens;
    if semantic_grammar::parse_level_up_intro_tokens(lexed).is_some() {
        if let Some(level_up) = parse_level_up_line_lexed(&lexed)? {
            return Ok(LineAst::Ability(level_up));
        }
    }
    if matches!(
        special_shape,
        Some(semantic_grammar::StaticSpecialLineShape::DoesntUntap)
    ) {
        let chunk =
            LineAst::StaticAbilities(vec![crate::cards::builders::StaticAbilityAst::Static(
                StaticAbility::doesnt_untap(),
            )]);
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if let Some(ability) = parse_if_this_spell_costs_less_to_cast_line_lexed(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(ability) = parse_spell_cost_increase_per_target_beyond_first_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(abilities) = parse_spell_and_player_activated_ability_cost_modifier_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbilities(abilities.into_iter().map(Into::into).collect()),
            chosen_option,
        );
    }
    if let Some(ability) = parse_spells_cost_modifier_line(&lexed)? {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(chunk) = parse_compound_buff_and_unblockable_static_chunk(parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if semantic_grammar::parse_combined_spell_and_activation_tax_tokens(lexed).is_some()
        && let Some(abilities) = parse_static_ability_ast_line_lexed(&lexed)?
    {
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    if let Some(ability) =
        crate::runtime_backend::families::keyword_static::parse_double_counters_replacement_line(
            &lexed,
        )?
    {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(ability.into()),
            chosen_option,
        );
    }
    if let Some(actions) = semantic_grammar::parse_source_keyword_tail_tokens(lexed)
        .and_then(|tail| parse_ability_line_lexed(tail.ability_tokens))
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    if let Some(abilities) =
        crate::runtime_backend::families::keyword_static::parse_additional_land_play_line(&lexed)?
    {
        let abilities = abilities
            .into_iter()
            .map(crate::cards::builders::StaticAbilityAst::Static)
            .collect();
        return wrap_chosen_option_static_chunk(LineAst::StaticAbilities(abilities), chosen_option);
    }
    match parse_static_ability_ast_line_lexed(&lexed) {
        Ok(Some(mut abilities)) => {
            restore_copy_static_variant_source_display(&mut abilities, &line.info.raw_line);
            return wrap_chosen_option_static_chunk(
                LineAst::StaticAbilities(abilities),
                chosen_option,
            );
        }
        Ok(None) => {}
        Err(_)
            if parse_tokens
                .iter()
                .any(|token| token.kind == TokenKind::Period) => {}
        Err(err) => return Err(err),
    }
    if semantic_grammar::parse_skip_keyword_action_probe_tokens(parse_tokens).is_none()
        && let Some(actions) = parse_ability_line_lexed(&lexed)
    {
        return wrap_chosen_option_static_chunk(LineAst::Abilities(actions), chosen_option);
    }
    if let Some(chunk) = parse_split_static_chunk(line, parse_tokens)? {
        return wrap_chosen_option_static_chunk(chunk, chosen_option);
    }
    if semantic_grammar::parse_ability_word_marker_tokens(parse_tokens).is_some() {
        return wrap_chosen_option_static_chunk(
            LineAst::StaticAbility(
                StaticAbility::keyword_marker(render_token_slice(parse_tokens).trim().to_string())
                    .into(),
            ),
            chosen_option,
        );
    }
    Err(CardTextError::ParseError(format!(
        "rewrite static lowering could not reconstitute static line '{}'",
        line.info.raw_line
    )))
}

#[test]
fn ability_word_marker_detection_uses_token_kinds() {
    let marker_tokens = lex_line("Landfall", 0).expect("marker should lex");
    assert!(semantic_grammar::parse_ability_word_marker_tokens(&marker_tokens).is_some());

    let sentence_tokens = lex_line(
        "Landfall — Whenever a land enters under your control, draw a card.",
        0,
    )
    .expect("sentence should lex");
    assert!(semantic_grammar::parse_ability_word_marker_tokens(&sentence_tokens).is_none());
}

#[test]
fn additional_land_play_static_count_uses_token_words() {
    let tokens = lex_line(
        "You may play two additional lands on each of your turns.",
        0,
    )
    .expect("lexes");
    assert_eq!(
        semantic_grammar::parse_additional_land_play_count_tokens(&tokens),
        Some(2)
    );

    let non_match = lex_line("You may play an additional land this turn.", 0).expect("lexes");
    assert_eq!(
        semantic_grammar::parse_additional_land_play_count_tokens(&non_match),
        None
    );
}

#[cfg(test)]
pub(crate) fn parse_keyword_line_for_test(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    parse_keyword_line_with_full_tokens_for_test(info, text, parse_tokens, parse_tokens, kind)
}

#[cfg(test)]
pub(crate) fn parse_keyword_line_with_full_tokens_for_test(
    info: LineInfo,
    text: &str,
    parse_tokens: &[OwnedLexToken],
    full_parse_tokens: &[OwnedLexToken],
    kind: RewriteKeywordLineKind,
) -> Result<LineAst, CardTextError> {
    super::super::keyword_registry::parse_keyword_payload_for_kind(
        info,
        text,
        parse_tokens,
        full_parse_tokens,
        kind,
    )
}

#[cfg(test)]
fn test_line_info(raw_line: &str) -> LineInfo {
    LineInfo {
        line_index: 0,
        display_line_index: 0,
        raw_line: raw_line.to_string(),
        source_tokens: lex_line(raw_line, 0).unwrap_or_default(),
        normalized: NormalizedLine {
            original: raw_line.to_string(),
            normalized: raw_line.to_ascii_lowercase(),
            char_map: Vec::new(),
        },
        semantic_facts: Default::default(),
    }
}

#[cfg(test)]
fn test_rewrite_triggered_line(raw_line: &str, full_text: &str) -> RewriteTriggeredLine {
    RewriteTriggeredLine {
        info: test_line_info(raw_line),
        full_text: full_text.to_string(),
        full_parse_tokens: lex_line(full_text, 0).unwrap_or_default(),
        intervening_if: None,
        presentation: None,
        max_triggers_per_turn: Some(1),
        chosen_option: None,
    }
}

#[test]
fn tagged_characteristic_addition_is_a_bound_effect_followup() {
    let tokens = lex_line(
        "Put target artifact onto the battlefield. That permanent is an enchantment in addition to its other types.",
        0,
    )
    .expect("bound characteristic fixture should lex");
    let sentences = split_lexed_sentences(&tokens);
    assert!(sentences_have_bound_characteristic_followup_after_first(
        &sentences
    ));

    let tokens = lex_line(
        "Draw a card. Creatures you control are artifacts in addition to their other types.",
        0,
    )
    .expect("independent static fixture should lex");
    let sentences = split_lexed_sentences(&tokens);
    assert!(!sentences_have_bound_characteristic_followup_after_first(
        &sentences
    ));
}

#[test]
fn triggered_line_source_text_keeps_raw_do_this_only_once_suffix() {
    let raw_line = "Whenever Pantlaza or another Dinosaur you control enters, you may discover X, where X is that creature's toughness. Do this only once each turn.";
    let full_text = "whenever pantlaza or another dinosaur you control enters, you may discover x, where x is that creature's toughness";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

#[test]
fn triggered_line_source_text_keeps_labelled_raw_do_this_only_once_suffix() {
    let raw_line = "Mold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.";
    let full_text = "whenever one or more lands enter under an opponent's control without being played, you may search your library for a plains card, put it onto the battlefield tapped, then shuffle";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

pub(crate) fn normalize_exert_followup_source_reference_tokens(
    source_ref: &str,
    followup_tokens: &[OwnedLexToken],
) -> Vec<OwnedLexToken> {
    semantic_grammar::normalize_exert_followup_source_tokens(source_ref, followup_tokens)
}

pub(crate) fn parse_exert_attack_keyword_line(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    let sentence_tokens = split_lexed_sentences(parse_tokens);
    let Some(head_tokens) = sentence_tokens.first().copied() else {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering could not parse exert attack line '{}'",
            line.info.raw_line
        )));
    };
    let semantic_grammar::ExertAttackHead {
        only_if_not_exerted_this_turn,
        source_ref,
    } = semantic_grammar::parse_exert_attack_head_tokens(head_tokens).map_err(|message| {
        CardTextError::ParseError(format!(
            "rewrite keyword lowering {message} '{}'",
            line.info.raw_line
        ))
    })?;

    let followup = sentence_tokens
        .get(1)
        .and_then(|tokens| semantic_grammar::parse_exert_reflexive_followup_tokens(tokens));
    let linked_trigger = if let Some(followup) = followup {
        let normalized_followup_tokens = normalize_exert_followup_source_reference_tokens(
            source_ref.as_str(),
            followup.effect_tokens,
        );
        let effects_ast = parse_effect_sentences_lexed(&normalized_followup_tokens)?;
        let prepared = rewrite_prepare_effects_with_trigger_context_for_lowering(
            None,
            &effects_ast,
            ReferenceImports::default(),
        )?;
        let lowered = materialize_prepared_effects_with_trigger_context(&prepared)?;
        Some(crate::ability::TriggeredAbility {
            trigger: crate::triggers::Trigger::state_based("When you do"),
            effects: lowered.effects,
            choices: lowered.choices,
            intervening_if: None,
            presentation_label: None,
        })
    } else if sentence_tokens
        .get(1)
        .is_some_and(|tokens| semantic_grammar::parse_when_followup_intro_tokens(tokens))
    {
        return Err(CardTextError::ParseError(format!(
            "rewrite keyword lowering expected exert reflexive followup '{}'",
            line.info.raw_line
        )));
    } else {
        None
    };

    Ok(LineAst::StaticAbility(
        StaticAbility::exert_attack(
            only_if_not_exerted_this_turn,
            linked_trigger,
            line.info.raw_line.clone(),
        )
        .into(),
    ))
}

fn rewrite_copy_count_to_times_paid_label_rewrite(effects: &mut [EffectAst], label: &str) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell { target, count, .. },
            ..
        }) = effect
            && let crate::cards::builders::TargetAst::Source(_) = target
            && let crate::effect::Value::Count(filter) = count
            && filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            *count = crate::effect::Value::TimesPaidLabel(label.into());
        }
        // Recurse into every nested-effect scope through the shared traversal
        // helper so new wrapper variants are covered automatically (the previous
        // hand-rolled match silently skipped RepeatEffects/ManaRestricted and the
        // newer ChooseOneOf/IfEffectDidNotHappen/TagAffected variants).
        crate::runtime_backend::model::effect_ast_traversal::for_each_nested_effects_mut(
            effect,
            true,
            |nested| rewrite_copy_count_to_times_paid_label_rewrite(nested, label),
        );
    }
}

pub(crate) fn parse_gift_keyword_line(line: &RewriteKeywordLine) -> Result<LineAst, CardTextError> {
    let spec =
        semantic_grammar::parse_standard_gift_spec_tokens(&line.parse_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "rewrite keyword lowering could not parse gift line '{}'",
                line.info.raw_line
            ))
        })?;
    let cost = OptionalCost::custom(
        line.info.raw_line.trim(),
        TotalCost::from_cost(Cost::effect(
            crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::Opponent,
                "gifted_player",
            )
            .remember_as_chosen_player(),
        )),
    );

    Ok(LineAst::GiftKeyword {
        cost: cost.into(),
        effects: standard_gift_effects(spec.variant),
        followup_text: standard_gift_followup_text(spec.variant).to_string(),
        timing: spec.timing,
    })
}

fn standard_gift_followup_text(variant: semantic_grammar::StandardGiftVariant) -> &'static str {
    match variant {
        semantic_grammar::StandardGiftVariant::Card => "the chosen player draws a card.",
        semantic_grammar::StandardGiftVariant::Treasure => {
            "the chosen player creates a Treasure token."
        }
        semantic_grammar::StandardGiftVariant::Food => "the chosen player creates a Food token.",
        semantic_grammar::StandardGiftVariant::TappedFish => {
            "the chosen player creates a tapped 1/1 blue Fish creature token."
        }
        semantic_grammar::StandardGiftVariant::ExtraTurn => {
            "the chosen player takes an extra turn after this one."
        }
        semantic_grammar::StandardGiftVariant::Octopus => {
            "the chosen player creates an 8/8 blue Octopus creature token."
        }
    }
}

fn standard_gift_effects(variant: semantic_grammar::StandardGiftVariant) -> Vec<EffectAst> {
    match variant {
        semantic_grammar::StandardGiftVariant::Card => vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::Chosen,
            SubjectVerbActionAst::Draw {
                count: crate::effect::Value::Fixed(1),
            },
        )],
        semantic_grammar::StandardGiftVariant::Treasure => {
            vec![standard_gift_create_token_effect(
                "Treasure",
                crate::runtime_backend::token_definition::TokenDefinitionSpec::Builtin(
                    crate::runtime_backend::token_definition::BuiltinTokenShape::Treasure,
                ),
                false,
            )]
        }
        semantic_grammar::StandardGiftVariant::Food => {
            vec![standard_gift_create_token_effect(
                "Food",
                crate::runtime_backend::token_definition::TokenDefinitionSpec::Builtin(
                    crate::runtime_backend::token_definition::BuiltinTokenShape::Food,
                ),
                false,
            )]
        }
        semantic_grammar::StandardGiftVariant::TappedFish => {
            vec![standard_gift_create_token_effect(
                "1/1 blue Fish creature",
                fixed_standard_gift_creature_definition(
                    "Fish",
                    Subtype::Fish,
                    ColorSet::BLUE,
                    (1, 1),
                ),
                true,
            )]
        }
        semantic_grammar::StandardGiftVariant::ExtraTurn => {
            vec![EffectAst::subject_verb_extra_turn_after_turn(
                PlayerAst::Chosen,
                crate::cards::builders::ExtraTurnAnchorAst::CurrentTurn,
            )]
        }
        semantic_grammar::StandardGiftVariant::Octopus => vec![standard_gift_create_token_effect(
            "8/8 blue Octopus creature",
            fixed_standard_gift_creature_definition(
                "Octopus",
                Subtype::Octopus,
                ColorSet::BLUE,
                (8, 8),
            ),
            false,
        )],
    }
}

fn fixed_standard_gift_creature_definition(
    name: &str,
    subtype: Subtype,
    colors: ColorSet,
    power_toughness: (i32, i32),
) -> crate::runtime_backend::token_definition::TokenDefinitionSpec {
    crate::runtime_backend::token_definition::TokenDefinitionSpec::Creature(
        crate::runtime_backend::token_definition::CreatureTokenShape {
            name: name.to_string(),
            card_types: vec![CardType::Creature],
            subtypes: vec![subtype],
            power_toughness,
            legendary: false,
            colors,
            keywords: Vec::new(),
            rules: Default::default(),
        },
    )
}

fn standard_gift_create_token_effect(
    name: &str,
    definition: crate::runtime_backend::token_definition::TokenDefinitionSpec,
    tapped: bool,
) -> EffectAst {
    EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Chosen,
        SubjectVerbActionAst::CreateTokenWithMods {
            name: name.to_string(),
            definition,
            count: crate::effect::Value::Fixed(1),
            dynamic_power_toughness: None,
            player: PlayerAst::Chosen,
            actor_surface_explicit: false,
            attached_to: None,
            tapped,
            attacking: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
            ability_presentation: None,
        },
    )
}

pub(crate) fn parse_keyword_special_cases(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(chunk) = try_lower_hideaway_keyword(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_variant_keyword(line, parse_tokens) {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_lower_partner_with_tokens(parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_cost_with_cast_trigger(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_behold_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    if let Some(chunk) = try_parse_optional_waterbend_additional_cost(line, parse_tokens)? {
        return Ok(Some(chunk));
    }
    Ok(None)
}

pub(crate) fn try_parse_optional_waterbend_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(generic) = semantic_grammar::parse_optional_waterbend_generic_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let total_cost =
        crate::runtime_backend::lowering::compile_support::waterbend_optional_total_cost(generic);
    Ok(Some(LineAst::OptionalCost(
        OptionalCost::custom(line.info.raw_line.trim(), total_cost).into(),
    )))
}

fn try_lower_partner_variant_keyword(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    let visible_tokens = if line.full_parse_tokens.is_empty() {
        parse_tokens
    } else {
        line.full_parse_tokens.as_slice()
    };
    let variant = semantic_grammar::parse_partner_variant_label_tokens(visible_tokens)?;
    Some(LineAst::StaticAbility(
        StaticAbility::partner_variant(variant.display).into(),
    ))
}

fn try_lower_hideaway_keyword(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    try_lower_hideaway_tokens(parse_tokens)
}

fn try_lower_hideaway_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(shape) = semantic_grammar::parse_hideaway_keyword_tokens(parse_tokens)? else {
        return Ok(None);
    };
    Ok(Some(hideaway_line_ast(shape.count)))
}

fn hideaway_line_ast(count: i32) -> LineAst {
    let looked_tag = TagKey::from("hideaway_looked");
    let chosen_tag = TagKey::from("hideaway_exiled");
    let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
    choose_filter.zone = Some(Zone::Library);

    LineAst::Triggered {
        trigger: TriggerSpec::ThisEntersBattlefield,
        effects: vec![
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                crate::effect::Value::Fixed(count),
                looked_tag.clone(),
            ),
            EffectAst::ChooseObjects {
                filter: choose_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(chosen_tag.clone(), None), true),
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(chosen_tag),
                LibraryBottomOrderAst::Random,
                PlayerAst::You,
            ),
        ],
        max_triggers_per_turn: None,
    }
}

#[test]
fn hideaway_special_case_uses_parse_tokens() {
    let tokens = lex_line("Hideaway 5.", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&tokens)
            .expect("hideaway should lower")
            .is_some()
    );

    let non_numeric = lex_line("Hideaway X.", 0).expect("hideaway should lex");
    assert!(try_lower_hideaway_tokens(&non_numeric).is_err());

    let reminder = lex_line("Hideaway 5 reminder", 0).expect("hideaway should lex");
    assert!(
        try_lower_hideaway_tokens(&reminder)
            .expect("extra words should not match the closed-form special case")
            .is_none()
    );
}

fn try_lower_partner_with_tokens(
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    let Some(partner_name) = partner_with_name_from_tokens(parse_tokens) else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::default();
    filter.name = Some(partner_name.clone());

    Ok(Some(LineAst::Multiple(vec![
        LineAst::StaticAbility(StaticAbility::partner_with(partner_name.clone()).into()),
        LineAst::Triggered {
            trigger: TriggerSpec::ThisEntersBattlefield,
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::Target,
                effects: vec![EffectAst::subject_verb_search_library(
                    filter,
                    Zone::Hand,
                    PlayerAst::Target,
                    PlayerAst::Target,
                    crate::effect::SearchSelectionMode::Exact,
                    true,
                    true,
                    ChoiceCount::up_to(1),
                    None,
                    None,
                    crate::effect::SearchResultReferenceSurface::ThatCard,
                    false,
                )],
            }],
            max_triggers_per_turn: None,
        },
    ])))
}

fn partner_with_name_from_tokens(tokens: &[OwnedLexToken]) -> Option<String> {
    keyword_special_grammar::parse_partner_with_name_tokens(tokens)
}

#[test]
fn partner_name_and_visible_label_trim_on_lexed_reminder_tokens() {
    let partner_with_tokens = lex_line(
        "Partner with Toothy, Imaginary Friend (When this creature enters...)",
        0,
    )
    .expect("partner-with line should lex");
    assert_eq!(
        partner_with_name_from_tokens(&partner_with_tokens).as_deref(),
        Some("Toothy, Imaginary Friend")
    );

    let partner_label_tokens = lex_line(
        "Partner - Friends forever (You can have two commanders.)",
        0,
    )
    .expect("partner label should lex");
    assert_eq!(
        keyword_special_grammar::parse_partner_visible_label_tokens(&partner_label_tokens)
            .as_deref(),
        Some("Partner - Friends forever")
    );
}

pub(crate) fn try_parse_optional_cost_with_cast_trigger(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(shape) =
        keyword_special_grammar::parse_optional_cost_with_cast_trigger_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let head_effects = parse_effect_sentences_lexed(shape.optional_cost_effect_tokens)?;
    let [
        EffectAst::ChooseObjects {
            filter,
            count,
            player,
            ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: sacrificed_player,
                    ..
                },
            action:
                SubjectVerbActionAst::SacrificeAll {
                    filter: sacrificed_filter,
                },
        }),
    ] = head_effects.as_slice()
    else {
        return Ok(None);
    };
    if *player != crate::cards::builders::PlayerAst::Implicit
        || *sacrificed_player != crate::cards::builders::PlayerAst::Implicit
        || count.min != 1
        || count.max.is_some()
        || !matches!(sacrificed_filter, crate::target::ObjectFilter { tagged_constraints, .. } if tagged_constraints.iter().any(|constraint| constraint.tag.as_str() == IT_TAG))
    {
        return Ok(None);
    }

    let head_words = token_word_refs(shape.label_tokens);
    let label = format!(
        "As an additional cost to cast this spell, {}",
        head_words.join(" ")
    );
    let cost = OptionalCost::custom(
        label.clone(),
        TotalCost::from_cost(Cost::sacrifice(filter.clone())),
    )
    .repeatable();
    let mut effects = parse_effect_sentences_lexed(shape.followup_effect_tokens)?;
    rewrite_copy_count_to_times_paid_label_rewrite(&mut effects, &label);
    let followup_words = token_word_refs(shape.followup_effect_tokens);

    Ok(Some(LineAst::OptionalCostWithCastTrigger {
        cost: cost.into(),
        effects,
        followup_text: format!("When you do, {}", followup_words.join(" ")),
    }))
}

pub(crate) fn try_parse_optional_behold_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if line.kind != RewriteKeywordLineKind::AdditionalCost {
        return Ok(None);
    }

    let Some(shape) =
        keyword_special_grammar::parse_optional_keyword_additional_cost_tokens(parse_tokens)
    else {
        return Ok(None);
    };

    let total_cost = parse_activation_cost(shape.cost_tokens)?;
    if total_cost.mana_cost().is_some() || total_cost.costs().len() != 1 {
        return Ok(None);
    }

    let mut optional_cost = OptionalCost::custom(line.info.raw_line.trim(), total_cost);
    if let Some(subtype) = shape.behold_subtype {
        optional_cost.reference = crate::cost::OptionalCostRef::with_discriminator(
            crate::cost::OptionalCostKind::Behold,
            subtype.to_string(),
        );
    }

    Ok(Some(LineAst::OptionalCost(optional_cost.into())))
}

pub(crate) fn rewrite_modal_to_parsed_item(
    modal: RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    let Some(header) = parse_modal_header(&modal.header, &modal.header_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header.line_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info,
            description: mode.text,
            point_cost: mode.point_cost,
            additional_mana_cost: mode.additional_mana_cost,
            effects_ast,
        });
    }

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

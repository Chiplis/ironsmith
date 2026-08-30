use super::*;

pub(super) fn parse_gain_ability_sentence_with_subject(
    tokens: &[OwnedLexToken],
    typed_subject_tokens: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let stripped_if_you_do = trim_commas(strip_leading_if_you_do_lexed(tokens));
    if stripped_if_you_do.len() < tokens.len() {
        return Ok(
            parse_gain_ability_sentence(&stripped_if_you_do)?.map(|effects| {
                vec![EffectAst::IfResult {
                    predicate: IfResultPredicate::Did,
                    effects,
                }]
            }),
        );
    }

    let word_view = GainAbilityWordView::new(tokens);
    let word_list = word_view.to_word_refs();
    if gain_shapes::gain_clause_is_defender_as_if_attack(&word_list) {
        return Ok(None);
    }
    let leading_duration_phrase =
        gain_shapes::parse_leading_affected_object_counter_duration_shape(tokens)
            .or_else(|| gain_shapes::parse_leading_gain_duration_shape(&word_list))
            .map(|shape| (shape.consumed_words, shape.duration));
    let subject_start_word_idx = leading_duration_phrase
        .as_ref()
        .map(|(len, _)| *len)
        .unwrap_or(0);
    let Some((relative_gain_idx, gain_verb)) =
        gain_shapes::find_primary_gain_ability_verb(&word_list[subject_start_word_idx..])
    else {
        return Ok(None);
    };
    let gain_idx = subject_start_word_idx + relative_gain_idx;
    let Some(gain_token_idx) = word_view.map_word_or_end_to_token_boundary(gain_idx) else {
        return Ok(None);
    };
    if let Some((Verb::Create, create_idx)) = find_verb(tokens)
        && create_idx < gain_token_idx
        && gain_shapes::gain_words_include_token_noun(&word_list)
    {
        return Ok(None);
    }
    let losing = gain_verb == gain_shapes::GainAbilityVerb::Lose;

    let after_gain = &word_list[gain_idx + 1..];
    let after_gain_tokens = tokens.get(gain_token_idx + 1..).unwrap_or_default();
    if gain_verb == gain_shapes::GainAbilityVerb::Gain
        && after_gain
            .first()
            .is_some_and(|word| gain_shapes::gain_verb_is_life_or_control_head(word))
    {
        return Ok(None);
    }

    let subject_start_token_idx = if subject_start_word_idx == 0 {
        0usize
    } else if let Some(idx) = word_view.map_word_or_end_to_token_boundary(subject_start_word_idx) {
        idx
    } else {
        return Ok(None);
    };
    if subject_start_token_idx < gain_token_idx
        && let Some((subject_verb, subject_verb_idx)) =
            find_verb(&tokens[subject_start_token_idx..gain_token_idx])
        && subject_verb != Verb::Get
    {
        // An action head before the gain verb belongs to a sibling action in
        // an authored `<action> or <subject> gains ...` choice. The or-action
        // rule owns that whole sentence; reading those action words as a
        // subject makes the broad filter parser fall back to every creature
        // and silently drops the first alternative.
        if subject_verb_idx == 0
            && matches!(
                super::super::chain_carry::parse_or_action_clause_lexed(tokens),
                Ok(Some(_))
            )
        {
            return Ok(None);
        }
        let subject_tokens = trim_commas(&tokens[subject_start_token_idx..gain_token_idx]);
        let subject_words = GainAbilityWordView::new(&subject_tokens);
        let subject_word_refs = subject_words.to_word_refs();
        let subject_shape = gain_shapes::classify_gain_subject(&subject_word_refs);
        let controller_tail_subject = subject_shape.controller_tail;
        let target_phrase_with_controller_tail = subject_shape.target && controller_tail_subject;
        let object_filter_subject = parse_object_filter(&subject_tokens, false).is_ok();
        if !target_phrase_with_controller_tail
            && !controller_tail_subject
            && !object_filter_subject
            && !subject_shape.demonstrative_object
        {
            return Ok(None);
        }
    }

    let nested_quoted_ability = if words_start_nested_triggered_ability(after_gain) {
        quoted_nested_ability_end_and_duration(tokens, gain_token_idx)
    } else {
        None
    };
    let (duration_phrase, mut duration_condition) =
        if words_start_nested_triggered_ability(after_gain) {
            (None, None)
        } else {
            parse_ability_duration_with_condition(after_gain_tokens, after_gain)
        };
    let mut duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| {
            nested_quoted_ability
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .or_else(|| {
            leading_duration_phrase
                .as_ref()
                .map(|(_, duration)| duration.clone())
        })
        .unwrap_or(Until::Forever);
    let has_explicit_duration = duration_phrase.is_some()
        || nested_quoted_ability.is_some()
        || leading_duration_phrase.as_ref().is_some();

    let shared_get_tail_word_idx = if !losing {
        gain_shapes::find_shared_ability_tail(after_gain, gain_shapes::SharedAbilityTail::Get)
    } else {
        None
    };
    let shared_gain_tail_word_idx = if losing {
        gain_shapes::find_shared_ability_tail(after_gain, gain_shapes::SharedAbilityTail::Gain)
    } else {
        None
    };
    let shared_has_tail_word_idx = if losing {
        gain_shapes::find_shared_ability_tail(after_gain, gain_shapes::SharedAbilityTail::Has)
    } else {
        None
    };
    let mut following_pump_effect = if let Some(shared_idx) = shared_get_tail_word_idx {
        let get_word_idx = gain_idx + 1 + shared_idx + 1;
        parse_shared_subject_pump_from_get_tail(
            tokens,
            get_word_idx,
            &duration,
            has_explicit_duration,
        )?
    } else {
        None
    };
    if shared_get_tail_word_idx.is_some() && following_pump_effect.is_none() {
        return Ok(None);
    }
    if !has_explicit_duration && let Some((_, _, _, pump_duration, _, _)) = &following_pump_effect {
        // In "gains ... and gets ... until end of turn", the trailing
        // duration scopes both predicates. The pump parser owns that tail,
        // so carry its typed duration back to the preceding ability grant.
        // An explicit duration attached to the gain always wins.
        duration = pump_duration.clone();
    }
    let following_base_pt_effect = if let Some(shared_idx) = shared_has_tail_word_idx {
        let has_word_idx = gain_idx + 1 + shared_idx + 1;
        parse_shared_subject_base_pt_from_has_tail(tokens, has_word_idx, &duration)?
    } else {
        None
    };
    if shared_has_tail_word_idx.is_some() && following_base_pt_effect.is_none() {
        return Ok(None);
    }
    // A shared subject may carry three continuous actions in one clause:
    // "<subject> loses all abilities, becomes ..., and has base P/T ...".
    // Keep the middle `becomes` arm separate from the lost-ability payload so
    // all three actions retain the original grammatical subject.
    let following_become = shared_has_tail_word_idx
        .filter(|_| losing && following_base_pt_effect.is_some())
        .and_then(|has_separator_idx| {
            let become_relative_idx =
                gain_shapes::find_become_verb(&after_gain[..has_separator_idx])?;
            let become_word_idx = gain_idx + 1 + become_relative_idx;
            let tail_start = word_view.map_word_or_end_to_token_boundary(become_word_idx + 1)?;
            let tail_end = word_view
                .map_word_or_end_to_token_boundary(gain_idx + 1 + has_separator_idx)
                .unwrap_or(tokens.len());
            let tail = trim_commas(tokens.get(tail_start..tail_end)?);
            (!tail.is_empty()).then(|| (become_word_idx, tail.to_vec()))
        });
    let following_grant = if let Some(shared_idx) = shared_gain_tail_word_idx {
        let ability_start_word_idx = gain_idx + 1 + shared_idx + 2;
        let ability_end_word_idx = duration_phrase
            .as_ref()
            .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel)
            .unwrap_or(word_list.len());
        let Some(ability_start_token_idx) =
            word_view.map_word_or_end_to_token_boundary(ability_start_word_idx)
        else {
            return Ok(None);
        };
        let ability_end_token_idx = word_view
            .map_word_or_end_to_token_boundary(ability_end_word_idx)
            .unwrap_or(tokens.len());
        let ability_tokens = trim_commas(
            tokens
                .get(ability_start_token_idx..ability_end_token_idx)
                .unwrap_or_default(),
        );
        let (abilities, is_choice) =
            parse_granted_abilities_for_gain_clause(&ability_tokens, &word_list, false)?;
        if abilities.is_empty() {
            return Ok(None);
        }
        Some((abilities, is_choice))
    } else {
        None
    };

    let mut trailing_tail_tokens: Vec<OwnedLexToken> = Vec::new();
    if shared_get_tail_word_idx.is_none()
        && let Some((start_rel, len_words, _)) = duration_phrase
    {
        let tail_word_idx = gain_idx + 1 + start_rel + len_words;
        if let Some(tail_token_idx) = word_view.map_word_or_end_to_token_boundary(tail_word_idx) {
            let trimmed_tail_tokens = trim_commas(&tokens[tail_token_idx..]);
            let tail_tokens =
                strip_leading_token_words_any(&trimmed_tail_tokens, &["and", "then"]).to_vec();
            if !tail_tokens.is_empty() {
                trailing_tail_tokens = tail_tokens;
            }
        }
    }
    if duration_condition.is_none()
        && !trailing_tail_tokens.is_empty()
        && let Some(predicate) = parse_trailing_if_predicate_lexed(&trailing_tail_tokens)
        && let Some(condition) = condition_from_gain_trailing_predicate(predicate)
    {
        duration_condition = Some(condition);
        trailing_tail_tokens.clear();
    }
    let mut grants_must_attack = false;
    if !trailing_tail_tokens.is_empty() {
        let tail_view = GainAbilityWordView::new(&trailing_tail_tokens);
        let mut tail_words = tail_view.to_word_refs();
        if tail_words.first().is_some_and(|word| *word == AND_WORD) {
            tail_words = tail_words[1..].to_vec();
        }
        if gain_shapes::is_must_attack_this_combat_tail(&tail_words) {
            grants_must_attack = true;
            trailing_tail_tokens.clear();
        }
    }

    let ability_end_word_idx = [
        duration_phrase
            .as_ref()
            .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel),
        shared_gain_tail_word_idx.map(|idx| gain_idx + 1 + idx),
        shared_get_tail_word_idx.map(|idx| gain_idx + 1 + idx),
        shared_has_tail_word_idx.map(|idx| gain_idx + 1 + idx),
        following_become
            .as_ref()
            .map(|(become_word_idx, _)| *become_word_idx),
    ]
    .into_iter()
    .flatten()
    .min();
    let ability_end_token_idx = if let Some((close_quote_token_idx, _)) = nested_quoted_ability {
        // This index is used as the exclusive bound below, so retain the
        // closing delimiter. The granted-ability parser can then remove the
        // matching outer quote pair without mistaking it for an unmatched
        // nested-rule delimiter.
        close_quote_token_idx + 1
    } else if let Some(end_word_idx) = ability_end_word_idx {
        word_view
            .map_word_or_end_to_token_boundary(end_word_idx)
            .unwrap_or(tokens.len())
    } else {
        tokens.len()
    };
    let ability_start_token_idx = gain_token_idx + 1;
    if ability_start_token_idx > ability_end_token_idx || ability_start_token_idx >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = trim_commas(&tokens[ability_start_token_idx..ability_end_token_idx]);

    let (mut abilities, grant_is_choice) =
        parse_granted_abilities_for_gain_clause(&ability_tokens, &word_list, !losing)?;
    if !trailing_tail_tokens.is_empty() {
        let tail_tokens = strip_leading_token_words_any(&trailing_tail_tokens, &["and", "then"]);
        let (trailing_abilities, trailing_is_choice) =
            parse_granted_abilities_for_gain_clause(tail_tokens, &word_list, false)?;
        if !trailing_abilities.is_empty() && !trailing_is_choice {
            abilities.extend(trailing_abilities);
            trailing_tail_tokens.clear();
        }
    }
    let removes_all_abilities = losing
        && gain_shapes::classify_ability_reference_surface(
            &GainAbilityWordView::new(&ability_tokens).to_word_refs(),
        ) == gain_shapes::AbilityReferenceSurface::AllAbilities;
    if abilities.is_empty() && !grants_must_attack && !removes_all_abilities {
        return Ok(None);
    }
    if grants_must_attack {
        abilities.push(GrantedAbilityAst::MustAttack);
    }
    reject_unsupported_lost_abilities(losing, &abilities)?;

    // Check for "gets +X/+Y and gains/has/loses ..." patterns - if there's a pump
    // modifier before the ability verb, extract it as a separate Pump/PumpAll effect.
    let before_gain = &word_list[subject_start_word_idx..gain_idx];
    let leading_become_subject_end_word_idx = gain_shapes::find_become_verb(before_gain)
        .map(|become_idx| subject_start_word_idx + become_idx);
    let leading_become_effect = if let Some(become_word_idx) = leading_become_subject_end_word_idx {
        let Some(become_token_idx) = word_view.map_word_or_end_to_token_boundary(become_word_idx)
        else {
            return Ok(None);
        };
        let become_subject_tokens = trim_commas(&tokens[subject_start_token_idx..become_token_idx]);
        let mut become_tail_tokens =
            trim_commas(&tokens[become_token_idx + 1..gain_token_idx]).to_vec();
        while become_tail_tokens.last().is_some_and(|token| {
            token
                .as_word()
                .is_some_and(gain_shapes::gain_word_is_connector)
        }) {
            become_tail_tokens.pop();
        }
        let become_tail_tokens = trim_commas(&become_tail_tokens);
        if become_subject_tokens.is_empty() || become_tail_tokens.is_empty() {
            None
        } else {
            let mut become_effect =
                parse_become_clause(&become_subject_tokens, &become_tail_tokens)?;
            if has_explicit_duration {
                apply_gain_clause_duration_to_leading_effect(&mut become_effect, &duration);
            }
            Some(become_effect)
        }
    } else {
        None
    };
    let get_idx = gain_shapes::find_get_verb(before_gain);
    // Run even when `losing`: cards like Will Kenrith say "...have base power and
    // toughness 0/3 and lose all abilities", where the base P/T precedes the lose
    // clause. The parser returns None when there is no leading base-P/T clause, so
    // ordinary "lose all abilities" lines are unaffected.
    let leading_base_pt_effect = parse_leading_subject_base_pt_before_gain(
        before_gain,
        subject_start_word_idx,
        gain_idx,
        &duration,
    )?;
    let mut pump_effect = if let Some(gi) = get_idx {
        let modifier_start_word_idx = subject_start_word_idx + gi + 1;
        let Some(modifier_start_token_idx) =
            word_view.map_word_or_end_to_token_boundary(modifier_start_word_idx)
        else {
            return Ok(None);
        };
        let mut modifier_tokens =
            trim_commas(&tokens[modifier_start_token_idx..gain_token_idx]).to_vec();
        while modifier_tokens.last().is_some_and(|token| {
            token
                .as_word()
                .is_some_and(gain_shapes::gain_word_is_connector)
        }) {
            modifier_tokens.pop();
        }
        let modifier_tokens = trim_commas(&modifier_tokens);
        if let Some(head) = gain_shapes::parse_gain_pump_head_shape(&modifier_tokens) {
            let power = head.power;
            let toughness = head.toughness;
            let additional_modifier = head.modifier_token_offset > 0;
            let modifier_tokens = modifier_tokens
                .get(head.modifier_token_offset..)
                .unwrap_or_default();
            let for_each = if let (Value::Fixed(power_per), Value::Fixed(toughness_per)) =
                (&power, &toughness)
            {
                parse_get_for_each_count_value(modifier_tokens.get(1..).unwrap_or_default())?.map(
                    |count| {
                        let count = if additional_modifier {
                            count.with_surface_hint(
                                ironsmith_core::ValueSurfaceHint::AdditionalPowerToughnessModifier,
                            )
                        } else {
                            count
                        };
                        (*power_per, *toughness_per, count)
                    },
                )
            } else {
                None
            };
            let has_local_duration = head.has_local_duration;
            let (power, toughness, local_duration, condition) =
                parse_get_modifier_values_with_tail(modifier_tokens, power, toughness)?;
            let pump_duration = if has_explicit_duration || !has_local_duration {
                duration.clone()
            } else {
                local_duration
            };
            let condition = if has_local_duration {
                condition
            } else {
                condition.or_else(|| duration_condition.clone())
            };
            Some((
                power,
                toughness,
                subject_start_word_idx + gi,
                pump_duration,
                condition,
                for_each,
            ))
        } else {
            None
        }
    } else {
        None
    };
    if !losing
        && let Some((power, toughness, _gi, pump_duration, condition, for_each)) = &pump_effect
        && let Some(local_get_idx) = get_idx
        && let Some(and_idx) = gain_shapes::find_gain_and_separator(before_gain, local_get_idx + 1)
        && and_idx + 1 < before_gain.len()
    {
        let source_subject_words = &before_gain[..local_get_idx];
        if gain_shapes::classify_gain_subject(source_subject_words).source_subject {
            let filter_word_start = subject_start_word_idx + and_idx + 1;
            let filter_tokens = word_view
                .map_word_or_end_to_token_boundary(filter_word_start)
                .map(|filter_token_start| trim_commas(&tokens[filter_token_start..gain_token_idx]));
            if let Some(filter_tokens) = filter_tokens
                && let Ok(filter) = parse_object_filter(&filter_tokens, false)
            {
                let mut effects = Vec::new();
                let source_target = TargetAst::Source(None);
                if let Some((power_per, toughness_per, count)) = for_each {
                    effects.push(EffectAst::subject_verb_pump_for_each(
                        *power_per,
                        *toughness_per,
                        source_target,
                        count.clone(),
                        pump_duration.clone(),
                    ));
                } else {
                    effects.push(EffectAst::subject_verb_pump(
                        power.clone(),
                        toughness.clone(),
                        source_target,
                        pump_duration.clone(),
                        condition.clone(),
                    ));
                }
                if grant_is_choice {
                    effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
                        filter, abilities, duration,
                    ));
                } else {
                    effects.push(EffectAst::subject_verb_grant_abilities_all(
                        filter, abilities, duration,
                    ));
                }
                effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
                return Ok(Some(effects));
            }
        }
    }
    let has_have_verb = gain_verb == gain_shapes::GainAbilityVerb::Has;
    let has_nested_granted_ability = abilities
        .iter()
        .any(|ability| matches!(ability, GrantedAbilityAst::ParsedObjectAbility { .. }));
    if has_have_verb
        && pump_effect.is_none()
        && !has_explicit_duration
        && !has_nested_granted_ability
    {
        return Ok(None);
    }

    // Determine the real subject (before "get"/"gets" if pump is present)
    let real_subject_end_word_idx = pump_effect
        .as_ref()
        .map(|(_, _, gi, _, _, _)| *gi)
        .or(leading_base_pt_effect
            .as_ref()
            .map(|(_, _, has_idx, _)| *has_idx))
        .or(leading_become_subject_end_word_idx)
        .unwrap_or(gain_idx);
    let real_subject_start_word_idx = if target_word_only_qualifies_a_controller(before_gain) {
        subject_start_word_idx
    } else if let Some(gi) = get_idx {
        subject_start_word_idx + gain_shapes::find_gain_real_subject_start(before_gain, gi)
    } else {
        subject_start_word_idx
    };
    let real_subject_start_token_idx = word_view
        .map_word_or_end_to_token_boundary(real_subject_start_word_idx)
        .unwrap_or(subject_start_token_idx);
    let real_subject_end_token_idx = word_view
        .map_word_or_end_to_token_boundary(real_subject_end_word_idx)
        .unwrap_or(gain_token_idx);
    if typed_subject_tokens.is_none() && real_subject_start_token_idx >= real_subject_end_token_idx
    {
        return Ok(None);
    }
    let inferred_subject_tokens = tokens
        .get(real_subject_start_token_idx..real_subject_end_token_idx)
        .unwrap_or_default();
    let real_subject_token_storage =
        trim_commas(typed_subject_tokens.unwrap_or(inferred_subject_tokens));
    let real_subject_tokens = trim_trailing_also(&real_subject_token_storage);
    let following_become_effect = if let Some((_, become_tail_tokens)) = &following_become {
        let mut effect = parse_become_clause(real_subject_tokens, become_tail_tokens)?;
        if has_explicit_duration {
            apply_gain_clause_duration_to_leading_effect(&mut effect, &duration);
        }
        Some(effect)
    } else {
        None
    };

    let mut effects = Vec::new();

    // Check for pronoun subjects ("it", "they") that reference a prior tagged object.
    let real_subject_word_view = GainAbilityWordView::new(real_subject_tokens);
    let real_subject_words = real_subject_word_view.to_word_refs();
    let real_subject_shape = gain_shapes::classify_gain_subject(&real_subject_words);
    let pronoun_set_quantifier_surface = pronoun_set_quantifier_surface(&real_subject_words);
    let target_word_qualifies_controller =
        target_word_only_qualifies_a_controller(&real_subject_words);

    // The typed get-then-gain shape owns the complete subject capture. Resolve an
    // explicit target before considering references embedded inside that target
    // (for example, "other than this creature" or "with a sticker on it").
    if real_subject_shape.target && !target_word_qualifies_controller {
        let has_preceding_target_effect = pump_effect.is_some()
            || leading_base_pt_effect.is_some()
            || leading_become_effect.is_some();
        let declares_shared_target =
            !has_preceding_target_effect && following_pump_effect.is_some();
        let target = parse_target_phrase(real_subject_tokens)?;
        if has_preceding_target_effect || declares_shared_target {
            bind_shared_subject_pump_characteristics(&mut pump_effect);
            bind_shared_subject_pump_characteristics(&mut following_pump_effect);
        }
        if declares_shared_target {
            // A gain-then-get clause has one authored target shared by both
            // continuous actions. Declare that target once, then compile both
            // consumers through the target prelude's durable `it` alias.
            // Repeating the explicit TargetAst on each child creates two
            // independently assignable target slots at cast time.
            effects.push(EffectAst::subject_verb_target_only(target.clone()));
        }
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        let grant_target = if has_preceding_target_effect || declares_shared_target {
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                span_from_tokens(real_subject_tokens),
            )
        } else {
            target.clone()
        };
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                grant_target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                grant_target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    grant_target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(
            &mut effects,
            &grant_target,
            &following_grant,
            &duration,
        );
        let following_pump_target = if has_preceding_target_effect || declares_shared_target {
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                span_from_tokens(real_subject_tokens),
            )
        } else {
            // A single-action target grant keeps its ordinary direct target.
            target
        };
        append_shared_subject_pump_to_target(
            &mut effects,
            &following_pump_target,
            &following_pump_effect,
        );
        append_shared_subject_base_pt_to_target(
            &mut effects,
            &following_pump_target,
            &following_base_pt_effect,
        );
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_pronoun_subject = real_subject_shape.pronoun;
    if is_pronoun_subject {
        let span = span_from_tokens(real_subject_tokens);
        let target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), span);
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                )
                .with_set_quantifier_surface(pronoun_set_quantifier_surface),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(&mut effects, &target, &following_grant, &duration);
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if let Some(target) = source_target_from_subject_tokens(real_subject_tokens).or_else(|| {
        named_source_target_from_granted_ability_surface(real_subject_tokens, &ability_tokens)
    }) {
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(&mut effects, &target, &following_grant, &duration);
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_demonstrative_subject = real_subject_shape.demonstrative_object;
    if is_demonstrative_subject {
        let target = TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            span_from_tokens(real_subject_tokens),
        );
        if let Some(become_effect) = &leading_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_base_pt_to_target(&mut effects, &target, &leading_base_pt_effect);
        append_shared_subject_pump_to_target(&mut effects, &target, &pump_effect);
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else if grant_is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_to_target(
                target.clone(),
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target.clone(),
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        if let Some(become_effect) = &following_become_effect {
            effects.push(become_effect.clone());
        }
        append_shared_subject_grant_to_target(&mut effects, &target, &following_grant, &duration);
        append_shared_subject_pump_to_target(&mut effects, &target, &following_pump_effect);
        append_shared_subject_base_pt_to_target(&mut effects, &target, &following_base_pt_effect);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_shape.player_you {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            real_subject_tokens,
            PlayerFilter::You,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_shape.you_and_permanents {
        let permanent_filter = crate::target::ObjectFilter::permanent().you_control();
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            real_subject_tokens,
            PlayerFilter::You,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed player/permanent gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects.push(EffectAst::subject_verb_grant_abilities_all(
            permanent_filter,
            abilities,
            duration,
        ));
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if !losing && real_subject_shape.player_any {
        let Some(mut player_effects) = player_gain_effects_for_abilities(
            &abilities,
            &duration,
            real_subject_tokens,
            PlayerFilter::Any,
        ) else {
            return Err(CardTextError::ParseError(format!(
                "unsupported player gain-ability clause (clause: '{}')",
                word_list.join(" ")
            )));
        };
        effects.append(&mut player_effects);
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    // "The chosen creature gains ..." names the accumulated chosen set, not
    // a filtered grant over every creature.
    if leading_become_effect.is_none()
        && leading_base_pt_effect.is_none()
        && pump_effect.is_none()
        && following_grant.is_none()
        && crate::grammar::targets::parse_chosen_object_target(real_subject_tokens).is_some()
    {
        let target = parse_target_phrase(real_subject_tokens)?;
        let mut effects = effects;
        if losing {
            effects.push(EffectAst::subject_verb_remove_abilities_from_target(
                target,
                abilities,
                duration.clone(),
            ));
        } else {
            effects.push(
                subject_verb_grant_abilities_to_target_with_optional_condition(
                    target,
                    abilities,
                    duration.clone(),
                    &duration_condition,
                ),
            );
        }
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let filter =
        if let Some(filter) = parse_bare_card_type_subtype_union_filter(real_subject_tokens) {
            filter
        } else {
            parse_object_filter(real_subject_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported subject in {}-ability clause (clause: '{}')",
                    if losing { "lose" } else { "gain" },
                    word_list.join(" ")
                ))
            })?
        };

    if let Some(become_effect) = &leading_become_effect {
        effects.push(become_effect.clone());
    }
    if let Some((power, toughness, _has_idx, base_pt_duration)) = &leading_base_pt_effect {
        effects.push(EffectAst::subject_verb_set_base_power_toughness(
            power.clone(),
            toughness.clone(),
            TargetAst::Object(filter.clone(), None, None),
            base_pt_duration.clone(),
        ));
    }
    if let Some((power, toughness, _, pump_duration, _condition, _for_each)) = pump_effect {
        effects.push(EffectAst::subject_verb_pump_all(
            filter.clone(),
            power,
            toughness,
            pump_duration,
        ));
    }
    if losing {
        let mut remove = subject_verb_remove_abilities_all_with_optional_condition(
            filter.clone(),
            abilities,
            duration.clone(),
            &duration_condition,
        );
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::RemoveAbilitiesAll {
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = &mut remove
        {
            *set_quantifier_surface = match real_subject_tokens.first() {
                Some(token) if token.is_word("all") => {
                    Some(ironsmith_core::SetQuantifierSurface::All)
                }
                Some(token) if token.is_word("each") => {
                    Some(ironsmith_core::SetQuantifierSurface::Each)
                }
                Some(token) if token.is_word("those") => {
                    Some(ironsmith_core::SetQuantifierSurface::Those)
                }
                _ => None,
            };
        }
        effects.push(remove);
    } else if grant_is_choice {
        effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
            filter.clone(),
            abilities,
            duration.clone(),
        ));
    } else {
        effects.push(
            subject_verb_grant_abilities_all_with_optional_condition(
                filter.clone(),
                abilities,
                duration.clone(),
                &duration_condition,
            )
            .with_set_quantifier_surface(pronoun_set_quantifier_surface),
        );
    }
    if let Some(become_effect) = &following_become_effect {
        effects.push(become_effect.clone());
    }
    if let Some((abilities, is_choice)) = &following_grant {
        if *is_choice {
            effects.push(EffectAst::subject_verb_grant_abilities_choice_all(
                filter.clone(),
                abilities.clone(),
                duration.clone(),
            ));
        } else {
            effects.push(EffectAst::subject_verb_grant_abilities_all(
                filter.clone(),
                abilities.clone(),
                duration.clone(),
            ));
        }
    }
    if let Some((power, toughness, _, pump_duration, _condition, _for_each)) = following_pump_effect
    {
        effects.push(EffectAst::subject_verb_pump_all(
            filter.clone(),
            power,
            toughness,
            pump_duration,
        ));
    }
    if let Some((power, toughness, _, base_pt_duration)) = following_base_pt_effect {
        effects.push(EffectAst::subject_verb_set_base_power_toughness(
            power,
            toughness,
            TargetAst::Object(filter.clone(), None, None),
            base_pt_duration,
        ));
    }
    effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;

    Ok(Some(effects))
}

pub fn parse_gain_ability_to_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = gain_shapes::parse_source_gain_ability_shape(tokens) else {
        return Ok(None);
    };
    let ability_tokens = trim_edge_punctuation(shape.ability_tokens);
    if let Some(parsed) = parse_activated_line(&ability_tokens)? {
        return Ok(Some(EffectAst::subject_verb_grant_ability_to_source(
            parsed,
            shape.duration,
        )));
    }

    Ok(None)
}

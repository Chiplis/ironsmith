use super::*;

pub fn parse_triggered_line(
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

pub(super) fn parse_triggered_line_impl(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<LineAst, CardTextError> {
    use crate::grammar::effects::delayed_sentence_shapes::{
        DelayedScheduleStep, parse_delayed_schedule_sentence_shape,
    };

    let delayed_schedule = parse_delayed_schedule_sentence_shape(full_parse_tokens).or_else(|| {
        // Trigger rewriting may hand this routine a punctuation-normalized
        // effect slice as `full_parse_tokens`. The immutable source tokens
        // still carry the leading next-step header that proves this is a
        // resolving delayed schedule rather than a recurring ability.
        parse_delayed_schedule_sentence_shape(&line.info.source_tokens)
    });
    let nested_combat_payment = (|| {
        let (_, after_intro) = crate::grammar::primitives::parse_prefix(
            full_parse_tokens,
            crate::grammar::primitives::phrase(&["at", "the", "beginning", "of", "each", "combat"]),
        )?;
        let after_intro = trim_lexed_commas(after_intro);
        let (_, after_pay) = crate::grammar::primitives::parse_prefix(
            after_intro,
            crate::grammar::primitives::phrase(&["unless", "you", "pay"]),
        )?;
        let (cost_tokens, nested_trigger_tokens) =
            crate::grammar::primitives::split_lexed_once_on_comma(after_pay)?;
        if !nested_trigger_tokens
            .first()
            .is_some_and(|token| token.is_word("whenever"))
        {
            return None;
        }
        let cost =
            crate::grammar::leaf::parse_leaf_mana_cost_tokens(trim_lexed_commas(cost_tokens))
                .ok()?;
        Some(ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(cost))
    })();
    let mut parsed = parse_triggered_ability_line_impl(
        line,
        full_parse_tokens,
        trigger_parse_tokens,
        effect_parse_tokens,
    )?;
    transport_delayed_copy_retarget_in_line(&mut parsed);
    apply_source_spell_cast_trigger_spec(&mut parsed, line.info.source_tokens.as_slice())?;
    apply_protected_battle_iteration_surface(&mut parsed, line.info.source_tokens.as_slice());
    if let Some(source_filter) =
        spell_cast_mana_source_filter(trigger_parse_tokens, line.info.source_tokens.as_slice())?
    {
        apply_spell_cast_mana_source_filter(&mut parsed, &source_filter);
    }
    if let Some(source_surface) = spell_cast_single_target_source_exclusion_surface(
        trigger_parse_tokens,
        line.info.source_tokens.as_slice(),
    ) {
        apply_spell_cast_single_target_source_exclusion(&mut parsed, &source_surface);
    }
    let mut parsed =
        preserve_triggered_effect_surfaces(parsed, effect_parse_tokens, full_parse_tokens);
    // Surface preservation reparses the effect body and can replace the raw
    // effect vector. Reapply the idempotent typed transport so neither public
    // trigger route can leave a copied-object retarget on the outer program.
    if let Some(source_surface) = spell_cast_single_target_source_exclusion_surface(
        trigger_parse_tokens,
        line.info.source_tokens.as_slice(),
    ) {
        apply_spell_cast_single_target_source_exclusion(&mut parsed, &source_surface);
    }
    reconcile_named_explore_source_surface(
        &mut parsed,
        effect_parse_tokens,
        line.info.raw_line.as_str(),
    )?;
    // The generic surface-preservation pass above reparses triggered bodies
    // sentence-by-sentence. For an authored dynamic death-group token this
    // can collapse the already-typed aggregate P/T back to the token
    // definition's 0/0. Reconcile from the intact source only after that
    // lossy pass, retaining the exact TotalPower + zone-change-group proof.
    let authored_source_tokens =
        crate::lexer::lex_line(line.info.raw_line.as_str(), line.info.line_index)
            .unwrap_or_else(|_| line.info.source_tokens.clone());
    // The surface-preservation pass reparses the body one clause at a time.
    // Restore a grammar-proven serial target list from the intact authored
    // tail so all independent targets keep the one shared leading duration.
    reconcile_serial_target_pt_modifiers(&mut parsed, &authored_source_tokens)?;
    reconcile_authored_correlated_trigger_programs(&mut parsed, &authored_source_tokens)?;
    reconcile_dynamic_zone_change_group_token_creation(&mut parsed, &authored_source_tokens)?;
    reconcile_dynamic_zone_change_group_token_creation(&mut parsed, effect_parse_tokens)?;
    reconcile_open_attraction_reminder(&mut parsed, line.info.raw_line.as_str());
    transport_delayed_copy_retarget_in_line(&mut parsed);
    apply_source_spell_cast_trigger_spec(&mut parsed, line.info.source_tokens.as_slice())?;
    apply_protected_battle_iteration_surface(&mut parsed, line.info.source_tokens.as_slice());
    // Source-trigger restoration above is intentionally broad for ordinary
    // spell-cast triggers. Reapply the stricter coordinated spell-or-ability
    // proof last so it cannot be simplified back to only its spell arm.
    reconcile_authored_correlated_trigger_programs(&mut parsed, &authored_source_tokens)?;
    if let Some(source_surface) = spell_cast_single_target_source_exclusion_surface(
        trigger_parse_tokens,
        line.info.source_tokens.as_slice(),
    ) {
        apply_spell_cast_single_target_source_exclusion(&mut parsed, &source_surface);
    }
    // Effect-surface reconciliation may rebuild a triggered chunk from its
    // body. Reapply the introduction from this exact authored sentence so a
    // physical `When` chunk split from a preceding static sentence cannot
    // fall back to the trigger kind's default `Whenever` surface.
    if let Some(intro) =
        super::super::super::grammar::trigger_surface::parse_trigger_intro_surface_tokens(
            full_parse_tokens,
        )
    {
        match &mut parsed {
            LineAst::Triggered { trigger, .. } => {
                *trigger = super::super::triggered_chunks::apply_trigger_intro_surface(
                    trigger.clone(),
                    Some(intro),
                );
            }
            LineAst::Ability(ability) => {
                if let Some(trigger_spec) = ability.trigger_spec.take() {
                    let trigger = super::super::triggered_chunks::apply_trigger_intro_surface(
                        *trigger_spec,
                        Some(intro),
                    );
                    ability.trigger_spec = Some(Box::new(trigger.clone()));
                    if let AbilityKind::Triggered(triggered) = ability.kind_mut() {
                        triggered.trigger = trigger;
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(cost) = nested_combat_payment {
        let (trigger, effects, nested_cap) = match parsed {
            LineAst::Triggered {
                trigger,
                effects,
                max_triggers_per_turn,
            } => (trigger, effects, max_triggers_per_turn),
            LineAst::Ability(parsed)
                if matches!(
                    parsed.kind(),
                    crate::model::CompilerAbilityKindCore::Triggered(_)
                ) =>
            {
                let trigger = parsed.trigger_spec.ok_or_else(|| {
                    CardTextError::InvariantViolation(format!(
                        "nested beginning-of-combat payment lost its trigger spec: '{}'",
                        line.info.raw_line
                    ))
                })?;
                let effects = parsed.effects_ast.ok_or_else(|| {
                    CardTextError::InvariantViolation(format!(
                        "nested beginning-of-combat payment lost its effect AST: '{}'",
                        line.info.raw_line
                    ))
                })?;
                (*trigger, effects, None)
            }
            _ => {
                return Err(CardTextError::InvariantViolation(format!(
                    "nested beginning-of-combat payment did not preserve its typed trigger: '{}'",
                    line.info.raw_line
                )));
            }
        };
        if nested_cap.is_some() {
            return Err(CardTextError::ParseError(format!(
                "nested beginning-of-combat payment trigger cannot carry a frequency cap: '{}'",
                line.info.raw_line
            )));
        }
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::BeginningOfCombat(PlayerFilter::Any),
            effects: vec![EffectAst::UnlessPays {
                effects: vec![EffectAst::DelayedTriggerForDuration {
                    trigger,
                    effects,
                    one_shot: false,
                    duration: Until::EndOfCombat,
                    either_of_watched_objects: false,
                    while_any_tagged_object_in_zone: None,
                }],
                player: PlayerAst::You,
                cost,
                before_delayed_step: false,
            }],
            max_triggers_per_turn: None,
        });
    }
    let Some(schedule) = delayed_schedule else {
        return Ok(parsed);
    };
    let effects = match parsed {
        LineAst::Triggered { effects, .. } => effects,
        LineAst::Ability(parsed)
            if matches!(
                parsed.kind(),
                crate::model::CompilerAbilityKindCore::Triggered(_)
            ) =>
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
        DelayedScheduleStep::UntapStep => EffectAst::DelayedUntilNextUntapStep {
            player: schedule.player,
            effects,
        },
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
        DelayedScheduleStep::FirstMainPhase => EffectAst::DelayedUntilNextFirstMainPhase {
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

pub(super) fn transport_delayed_copy_retarget_in_line(parsed: &mut LineAst) {
    match parsed {
        LineAst::Triggered { effects, .. } => {
            crate::effect_sentences::transport_copy_retarget_into_trailing_delayed_trigger(effects);
        }
        LineAst::Ability(parsed) => {
            if let Some(effects) = parsed.effects_ast.as_mut() {
                crate::effect_sentences::transport_copy_retarget_into_trailing_delayed_trigger(
                    effects,
                );
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                transport_delayed_copy_retarget_in_line(chunk);
            }
        }
        _ => {}
    }
}

/// Public CST routing must preserve the complete authored token stream for
/// these correlated multi-sentence trigger bodies. Broad split probes can
/// successfully parse each sentence while losing the value/object-set bridge,
/// so expose one exact predicate for the CST layer to claim them before those
/// probes run.
pub fn is_exact_correlated_trigger_effect_bundle(effect_parse_tokens: &[OwnedLexToken]) -> bool {
    exact_dynamic_exile_permission_bundle(effect_parse_tokens).is_some()
        || exact_atomic_return_as_aura_bundle(effect_parse_tokens).is_some()
        || exact_looked_hand_optional_cast_bundle(effect_parse_tokens).is_some()
}

pub(super) fn preserve_triggered_effect_surfaces(
    mut parsed: LineAst,
    effect_parse_tokens: &[OwnedLexToken],
    full_parse_tokens: &[OwnedLexToken],
) -> LineAst {
    let Ok(mut surfaced) = parse_effect_sentences_preserving_source_boundaries(effect_parse_tokens)
    else {
        return parsed;
    };
    let full_words = crate::lexer::token_word_refs(full_parse_tokens);
    let explicit_participant_order = crate::word_primitives::sequence_occurs(
        &full_words,
        &["starting", "with", "you", "each", "player"],
    );
    if explicit_participant_order
        && let Some(EffectAst::SourceSentence {
            starting_with_controller,
            ..
        }) = surfaced.first_mut()
    {
        *starting_with_controller = true;
    } else if explicit_participant_order {
        surfaced = vec![EffectAst::SourceSentence {
            effects: surfaced,
            leading_then: false,
            starting_with_controller: true,
        }];
    }
    fn without_source_sentence_markers(effects: &[EffectAst]) -> Vec<EffectAst> {
        let mut flattened = Vec::new();
        for effect in effects {
            match effect {
                EffectAst::SourceSentence { effects, .. } => {
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
                EffectAst::SourceSentence { effects, .. }
                | EffectAst::CommaThen { effects }
                | EffectAst::Coordinated { effects, .. } => {
                    flattened.extend(without_surface_markers(effects));
                }
                effect => {
                    let mut effect = effect.clone();
                    // Surface provenance can sit inside semantic owners such
                    // as `May`, `IfResult`, or a conditional branch. Compare
                    // those owners after recursively erasing only the nested
                    // presentation wrappers; a shallow comparison otherwise
                    // rejects a valid resurfacing merely because the authored
                    // `, then` was inside an optional program.
                    crate::model::visit::for_each_nested_effect_vec_mut(
                        &mut effect,
                        true,
                        |nested| {
                            *nested = without_surface_markers(nested);
                        },
                    );
                    flattened.push(effect);
                }
            }
        }
        flattened
    }
    let sentence_flattened = without_source_sentence_markers(&surfaced);
    let flattened = without_surface_markers(&surfaced);
    if surfaced == flattened {
        return parsed;
    }

    fn matches_surfaced_effects(
        effects: &[EffectAst],
        sentence_flattened: &[EffectAst],
        flattened: &[EffectAst],
    ) -> bool {
        effects == sentence_flattened
            || effects == flattened
            || without_surface_markers(effects) == flattened
    }

    fn replace_matching_effects(
        parsed: &mut LineAst,
        sentence_flattened: &[EffectAst],
        flattened: &[EffectAst],
        surfaced: &[EffectAst],
    ) -> bool {
        match parsed {
            LineAst::Triggered { effects, .. }
                if matches_surfaced_effects(effects, sentence_flattened, flattened) =>
            {
                *effects = surfaced.to_vec();
                true
            }
            LineAst::Ability(parsed)
                if parsed.effects_ast.as_deref().is_some_and(|effects| {
                    matches_surfaced_effects(effects, sentence_flattened, flattened)
                }) =>
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

    let _ = replace_matching_effects(&mut parsed, &sentence_flattened, &flattened, &surfaced);
    parsed
}

pub(super) fn mark_non_mana_activated_trigger(trigger: &mut TriggerSpec) {
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

pub(super) fn parse_triggered_ability_line_impl(
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
    let authored_raw_tokens = crate::lexer::lex_line(&line.info.raw_line, line.info.line_index)
        .unwrap_or_else(|_| line.info.source_tokens.clone());
    let source_intro =
        super::super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(
            source_text_tokens,
        );
    let full_intro =
        super::super::super::grammar::trigger_surface::parse_trigger_intro_prefix_tokens(
            full_parse_tokens,
        );
    let trigger_surface_text = if source_intro.is_some() || full_intro.is_none() {
        source_text.as_str()
    } else {
        line.full_text.trim()
    };
    let mut trigger_facts = line.info.semantic_facts.triggered_ability.clone();
    if let Some(intro_surface) =
        super::super::super::grammar::trigger_surface::parse_trigger_intro_surface_tokens(
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

    // Eminence abilities are live in two functional zones. The document
    // splitter already proves the trigger/effect boundary and intervening
    // condition, but its ordinary fallback defaults the ability to the
    // battlefield and can detach the resolution body as a spell instruction.
    // Rebuild only the typed ability-word shell that explicitly names the
    // command-zone-or-battlefield source condition.
    let authored_words = crate::lexer::parser_token_word_refs(&authored_raw_tokens);
    let has_eminence_label = crate::word_primitives::first_is(&authored_words, "eminence");
    let names_command_or_battlefield = crate::word_primitives::sequence_occurs(
        &authored_words,
        &[
            "in",
            "the",
            "command",
            "zone",
            "or",
            "on",
            "the",
            "battlefield",
        ],
    );
    if has_eminence_label && names_command_or_battlefield {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        let effects = parse_effect_sentences_lexed(effect_parse_tokens)?;
        if !effects.is_empty() {
            let label = PresentationLabel::from_ability_word("Eminence");
            return Ok(LineAst::Ability(rewrite_parsed_triggered_ability(
                trigger,
                effects,
                vec![Zone::Command, Zone::Battlefield],
                Some(line.info.raw_line.clone()),
                line.intervening_if
                    .as_ref()
                    .map(|predicate| {
                        crate::compile_support::compile_condition_from_predicate_ast_with_env(
                            predicate,
                            &crate::model::reference_state::ReferenceEnv::default(),
                            None,
                        )
                    })
                    .transpose()?,
                Some(&label),
                ReferenceImports::default(),
            )));
        }
    }

    // `while it's exiled` qualifies the source of the counter-removal event;
    // it is not part of the restriction after the comma. Prepared trigger
    // rewrites can otherwise split at `this` and feed `card while ...` into
    // the effect parser, producing an unrelated object-filter union.
    let exiled_last_counter = parse_exiled_last_counter_triggered_line(&authored_raw_tokens)?.or(
        parse_exiled_last_counter_triggered_line(source_text_tokens)?,
    );
    if let Some(chunk) = exiled_last_counter {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // The public document splitter can simplify the coordinated
    // "cast ... or activate ..." trigger head to its spell arm before this
    // semantic handoff.  The authored line still carries the exact grammar
    // proof for both trigger domains and the shared X-cost qualification, so
    // give that intact head first refusal while retaining the already-parsed
    // effect slice (which owns the copy/retarget reference flow).
    if let Some(split) = semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens)
        && let Some(chunk) = lower_spell_or_activated_ability_x_cost_trigger(
            &authored_raw_tokens,
            split.before,
            effect_parse_tokens,
            inferred_max_triggers_per_turn,
        )?
    {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // Parley-style reveal programs carry one revealed-card set across three
    // authored sentences. Prepared trigger-body normalization can split that
    // set before the `revealed this way` iterator is resolved, leaving only a
    // bare reveal, token creation, and draw. Give the intact authored tail
    // first refusal when all three grammar facts are present.
    if let Some(split) = semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens) {
        let words = crate::lexer::parser_token_word_refs(split.after);
        let conditional_gate_remainder_program = is_gate_partition_word_program(&words);
        if conditional_gate_remainder_program {
            let effects = parse_effect_sentences_lexed(split.after)?;
            if !effects.is_empty() {
                let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
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
        let parley_reveal_program = is_parley_word_program(&words);
        if parley_reveal_program {
            let effects = parse_effect_sentences_lexed(split.after)?;
            if !effects.is_empty() {
                let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
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

    // Some document routes retain the full authored tail only in the effect
    // or source-token view rather than `raw_line`. Repeat the same narrow
    // proof over those views so the public runtime-backed path cannot bypass
    // the collection semantics merely because its raw chunk was shortened.
    for candidate in [effect_parse_tokens, source_text_tokens, full_parse_tokens] {
        let candidate_words = crate::lexer::parser_token_word_refs(candidate);
        let tail = if crate::word_primitives::contains_word(&candidate_words, "whenever")
            || crate::word_primitives::contains_word(&candidate_words, "when")
        {
            semantic_grammar::parse_comma_split_tokens(candidate)
                .map(|split| split.after)
                .unwrap_or(candidate)
        } else {
            candidate
        };
        let words = crate::lexer::parser_token_word_refs(tail);
        let is_parley = is_parley_word_program(&words);
        let is_gate_partition = is_gate_partition_core_word_program(&words);
        if is_parley || is_gate_partition {
            let effects = parse_effect_sentences_lexed(tail)?;
            if !effects.is_empty() {
                let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
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

    // This exact two-sentence procedure deliberately links the token created
    // by the first sentence to a delayed sacrifice on the controller's next
    // turn. Claim it before the broad sentence probes, which otherwise try to
    // parse `end step on your next turn` as an ordinary `end` action.
    if let Some(effects) = linked_created_token_next_turn_sacrifice_effects(effect_parse_tokens)? {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
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

    // A serial list of independently targeted P/T modifiers owns its shared
    // leading duration. The ordinary triggered-body splitter can otherwise
    // treat the first required target as setup for the two optional `other`
    // targets, dropping its modifier and normalizing the surviving duration.
    // Claim the already-typed generic sequence before those split probes.
    let serial_target_modifiers =
        crate::effect_sentences::parse_serial_target_pt_modifiers_sentence(effect_parse_tokens)?
            .or_else(|| {
                crate::grammar::semantic_lowering::parse_comma_split_tokens(&authored_raw_tokens)
                    .and_then(|split| {
                        crate::effect_sentences::parse_serial_target_pt_modifiers_sentence(
                            split.after,
                        )
                        .ok()
                        .flatten()
                    })
            });
    if let Some(effects) = serial_target_modifiers {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
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

    // A prepared triggered-body slice can already have reduced an authored
    // dynamic token to its 0/0 definition. Reparse only the grammar-proven
    // aggregate death-group creation from the intact source tail before that
    // lossy slice reaches ordinary sentence parsing.
    if let Some(effect) = authored_dynamic_token_creation_from_trigger(&authored_raw_tokens)? {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects: vec![effect],
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

    // The CST's prepared effect slice can already have lost the quote
    // boundaries that distinguish token rules from the outer instruction.
    // At this semantic handoff, the source-token stream is still intact.
    // Give only a fully parsed quantified create-with-embedded-rules tail
    // first refusal, then lower it with the ordinary trigger and presentation
    // wrappers. This prevents a quoted `can't block` rule from becoming the
    // resolution's outer restriction.
    if let Some(source_split) = semantic_grammar::parse_comma_split_tokens(source_text_tokens)
        && let Some(effect) =
            crate::effect_sentences::parse_quantified_token_creation_with_embedded_rules(
                source_split.after,
            )?
    {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(
                LineAst::Triggered {
                    trigger,
                    effects: vec![effect],
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

    if let Some(chunk) =
        parse_library_origin_source_pump_unblockable_triggered_line(full_parse_tokens)?
    {
        return apply_chosen_option_to_triggered_chunk(
            apply_explicit_intervening_if_to_triggered_chunk(chunk, line.intervening_if.clone())?,
            trigger_surface_text,
            trigger_facts,
            inferred_max_triggers_per_turn,
            chosen_option,
            presentation_label,
        );
    }

    // The prepared trigger body can let the broad `can't` family claim this
    // exact shared-source conjunction before the generic subject/verb parser
    // sees its leading P/T modifier. Reparse only the intact authored tail
    // through the strict two-effect grammar, then keep the already-prepared
    // trigger and presentation wrappers.
    let authored_source_pump_unblockable =
        crate::effect_sentences::parse_source_gets_unblockable_subject_verb(effect_parse_tokens)?
            .or(
                semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens)
                    .or_else(|| semantic_grammar::parse_comma_split_tokens(source_text_tokens))
                    .and_then(|split| {
                        crate::effect_sentences::parse_source_gets_unblockable_subject_verb(
                            split.after,
                        )
                        .transpose()
                    })
                    .transpose()?,
            );
    if let Some(effects) = authored_source_pump_unblockable {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
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

    // These two-sentence programs own typed provenance across the sentence
    // boundary. Route the already-proved bundle before the generic
    // returned-object/static splitter or ordinary sentence parser can peel
    // the follow-up away from its producer. Some public CST paths retain the
    // exact authored line in `source_text_tokens` while their prepared effect
    // slice has already been simplified sentence-by-sentence. Re-probe only
    // the authored post-trigger tail so the typed dynamic count, owner, and
    // shared exiled collection survive that handoff.
    let authored_tail = semantic_grammar::parse_comma_split_tokens(&authored_raw_tokens)
        .or_else(|| semantic_grammar::parse_comma_split_tokens(source_text_tokens))
        .map(|split| split.after);
    let authored_correlated_effects = authored_tail
        .as_ref()
        .and_then(|tokens| exact_dynamic_exile_permission_bundle(tokens));
    let authored_looked_hand_cast = authored_tail
        .as_ref()
        .and_then(|tokens| exact_looked_hand_optional_cast_bundle(tokens));
    if let Some(effects) = exact_dynamic_exile_permission_bundle(effect_parse_tokens)
        .or(authored_correlated_effects)
        .or_else(|| exact_atomic_return_as_aura_bundle(effect_parse_tokens))
        .or_else(|| exact_looked_hand_optional_cast_bundle(effect_parse_tokens))
        .or(authored_looked_hand_cast)
    {
        let trigger = parse_trigger_clause_lexed(trigger_parse_tokens)?;
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
        let effect_tokens = if effect_text_facts.has_full_party_condition {
            effect_parse_tokens
        } else {
            semantic_grammar::parse_comma_split_tokens(full_parse_tokens)
                .map(|split| split.after)
                .unwrap_or(effect_parse_tokens)
        };
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
            super::super::super::grammar::effects::followup_shapes::parse_counter_linked_land_subtype_followup(sentence)
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
                !sentence_is_linked_anaphoric_conditional_effect(sentence)
                    && (parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                        || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_))))
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
            chosen_option,
            presentation_label,
        )?);

        let mut parsed_all_static = true;
        for sentence in full_sentences.iter().skip(1) {
            if sentence_is_linked_anaphoric_conditional_effect(sentence) {
                parsed_all_static = false;
                break;
            } else if let Some(chunk) = parse_self_enters_with_x_counters_static_chunk(sentence) {
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
    let effect_is_document_program = effect_sentences.len() > 1;
    let effect_is_linked_collect_evidence =
        is_optional_source_exile_collect_evidence_procedure(effect_parse_tokens);
    if let Some(effects) = linked_created_token_next_turn_sacrifice_effects(effect_parse_tokens)?
        && let Ok(trigger) = parse_trigger_clause_lexed(trigger_parse_tokens)
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
        && !effect_is_document_program
        && let Some(first_static_idx) =
            effect_sentences
                .iter()
                .enumerate()
                .skip(1)
                .find_map(|(idx, sentence)| {
                    (!sentence_is_linked_anaphoric_conditional_effect(sentence)
                        && (parse_self_enters_with_x_counters_static_chunk(sentence).is_some()
                            || matches!(parse_static_ability_ast_line_lexed(sentence), Ok(Some(_)))))
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
                chosen_option,
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
        && (!full_parse_tokens_have_triggered_intervening_if_clause(full_parse_tokens)
            || effect_is_linked_collect_evidence)
        && (!full_text_facts.has_if_you_do
            || effect_is_document_program
            || effect_is_linked_collect_evidence)
        && (!full_text_facts.has_if_you_dont || effect_is_document_program)
        && !effect_text_facts.starts_with_if
    {
        let direct_trigger = parse_trigger_clause_lexed(trigger_parse_tokens).map(|mut trigger| {
            if full_text_has_non_mana_activated_ability_qualifier(full_parse_tokens) {
                mark_non_mana_activated_trigger(&mut trigger);
            }
            trigger
        });
        let direct_effects = if effect_is_linked_collect_evidence {
            // This exact procedure correlates effects across its authored
            // sentence boundaries before the generic prefix proof runs.
            parse_effect_sentences_lexed(effect_parse_tokens)
        } else {
            // The boundary-preserving parser begins with one joint semantic
            // parse and retains sentence groups only when every prefix is an
            // exact prefix of that joint AST. Cross-sentence rewrites
            // therefore still return the required flat correlated program,
            // while independent sentences keep their authored boundaries.
            parse_effect_sentences_preserving_source_boundaries(effect_parse_tokens)
        }
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

#[cfg(test)]
pub(super) fn parse_triggered_text_for_test(
    full_text: &str,
    trigger_text: &str,
    effect_text: &str,
) -> Result<LineAst, CardTextError> {
    let full_tokens = lex_line(full_text, 0).expect("full triggered line should lex");
    let trigger_tokens = lex_line(trigger_text, 0).expect("trigger clause should lex");
    let effect_tokens = lex_line(effect_text, 0).expect("trigger effects should lex");
    parse_triggered_line(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: full_text.to_string(),
            source_tokens: full_tokens.clone(),
            normalized: NormalizedLine {
                original: full_text.to_string(),
                normalized: full_text.to_string(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        full_text,
        &full_tokens,
        &trigger_tokens,
        &effect_tokens,
        None,
        None,
        None,
        None,
    )
}

#[cfg(test)]
#[test]
pub(super) fn independent_trigger_sentences_reach_the_public_semantic_handoff() {
    let full = "Whenever another Insect you control dies, put a +1/+1 counter on this creature. Each opponent loses 1 life.";
    let effects = "put a +1/+1 counter on this creature. Each opponent loses 1 life.";
    let parsed =
        parse_triggered_text_for_test(full, "Whenever another Insect you control dies", effects)
            .expect("independent trigger sentences should parse");
    let effects = match &parsed {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(parsed) => parsed
            .effects_ast
            .as_deref()
            .expect("triggered handoff must retain its effect AST"),
        _ => panic!("expected a typed triggered handoff: {parsed:#?}"),
    };
    assert!(
        matches!(
            effects,
            [
                EffectAst::SourceSentence { .. },
                EffectAst::SourceSentence { .. }
            ]
        ),
        "independent Oracle sentences must reach lowering with typed boundaries: {effects:#?}"
    );
}

#[cfg(test)]
#[test]
pub(super) fn serial_target_modifier_reconciliation_reaches_the_public_trigger_route() {
    let full = "Lightning Breath — When this creature enters, until your next turn, target creature an opponent controls gets -3/-0, up to one other target creature gets -2/-0, and up to one other target creature gets -1/-0.";
    let effects = "until your next turn, target creature an opponent controls gets -3/-0, up to one other target creature gets -2/-0, and up to one other target creature gets -1/-0.";
    let parsed = parse_triggered_text_for_test(full, "When this creature enters", effects)
        .expect("the authored serial target list should parse");
    let effects = match &parsed {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_deref()
            .expect("runtime-backed trigger should retain its effect AST"),
        _ => panic!("expected one triggered line: {parsed:#?}"),
    };
    let [
        EffectAst::Coordinated {
            effects,
            leading_duration: true,
            ..
        },
    ] = effects
    else {
        panic!("expected one coordinated leading-duration effect: {effects:#?}");
    };
    assert_eq!(effects.len(), 3, "{effects:#?}");
}

#[cfg(test)]
#[test]
pub(super) fn collect_evidence_if_do_procedure_reaches_the_public_trigger_route() {
    let full = "When this creature dies, you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.";
    let effects = "you may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.";
    let effect_tokens = lex_line(effects, 0).expect("collect-evidence effects should lex");
    assert!(
        is_optional_source_exile_collect_evidence_procedure(&effect_tokens),
        "sentences={:#?}",
        split_lexed_sentences(&effect_tokens)
            .iter()
            .map(|sentence| token_word_refs(sentence))
            .collect::<Vec<_>>()
    );
    let parsed = parse_triggered_text_for_test(full, "this creature dies", effects)
        .expect("collect-evidence death trigger should parse");
    let debug = format!("{parsed:#?}");
    assert!(
        debug.contains("ChooseObjectsWithAggregateConstraint"),
        "{debug}"
    );
    assert!(debug.contains("IsNotTaggedObject"), "{debug}");
    assert!(debug.contains("ReturnToBattlefield"), "{debug}");

    let near_miss = lex_line(
        "you may exile it. If you do, return this card to the battlefield tapped.",
        0,
    )
    .expect("near-miss procedure should lex");
    assert!(!is_optional_source_exile_collect_evidence_procedure(
        &near_miss
    ));
}

#[cfg(test)]
#[test]
pub(super) fn quantified_token_rules_reach_the_public_trigger_semantic_handoff() {
    let effects = "each opponent creates a 1/1 red Pirate creature token with \"This token can't block\" and \"Creatures you control attack each combat if able.\"";
    let full = format!("When this creature enters, {effects}");
    let parsed = parse_triggered_text_for_test(&full, "this creature enters", effects)
        .expect("the quantified token-rule trigger should parse from source tokens");
    let effects = semantic_effects_for_test(&parsed)
        .unwrap_or_else(|| panic!("missing quantified token effects: {parsed:#?}"));
    let [EffectAst::ForEachOpponent { effects }] = effects else {
        panic!("expected opponent iteration: {effects:#?}");
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CreateTokenWithMods {
                    definition,
                    granted_abilities,
                    ..
                },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected one token creation: {effects:#?}");
    };
    let definition_debug = format!("{definition:#?}");
    let granted_debug = format!("{granted_abilities:#?}");
    assert!(definition_debug.contains("CantBlock"), "{definition_debug}");
    assert!(granted_debug.contains("MustAttack"), "{granted_debug}");
    assert!(
        !granted_debug.contains("MustBlockSpecificAttacker"),
        "a quoted token rule escaped into the trigger resolution: {granted_debug}"
    );
}

#[cfg(test)]
#[test]
pub(super) fn created_token_next_turn_sacrifice_stays_inside_the_trigger() {
    let effects = "create a Lander token. At the beginning of the end step on your next turn, sacrifice that token.";
    let effect_tokens = lex_line(effects, 0).expect("linked token procedure should lex");
    let direct = linked_created_token_next_turn_sacrifice_effects(&effect_tokens)
        .expect("linked token helper should not fail")
        .unwrap_or_else(|| {
            panic!(
                "linked token helper did not claim the exact surface: {:#?}",
                split_lexed_sentences(&effect_tokens)
                    .iter()
                    .map(|sentence| token_word_refs(sentence))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(direct.len(), 2, "{direct:#?}");
    let full = format!("When this creature enters, {effects}");
    let parsed = parse_triggered_text_for_test(&full, "this creature enters", effects)
        .expect("linked token delayed sacrifice should parse");
    let effects = match &parsed {
        LineAst::Triggered { effects, .. } => effects,
        LineAst::Ability(ability) => ability
            .effects_ast
            .as_ref()
            .expect("runtime-backed trigger should retain its typed effects"),
        _ => panic!("both sentences must remain one trigger: {parsed:#?}"),
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CreateTokenWithMods { .. },
            ..
        }),
        EffectAst::DelayedUntilEndStepOfExtraTurn {
            effects: delayed, ..
        },
    ] = effects.as_slice()
    else {
        panic!("unexpected linked token procedure: {effects:#?}");
    };
    let debug = format!("{delayed:#?}");
    assert!(debug.contains(IT_TAG), "{debug}");
    assert!(!debug.contains("token: true"), "{debug}");
}

#[cfg(test)]
#[test]
pub(super) fn dynamic_death_group_token_creation_reaches_the_public_trigger_semantic_handoff() {
    let effects = "create a green Fungus Dinosaur creature token with base power and toughness each equal to the total power of those creatures.";
    let assert_dynamic_group_payload = |line: &LineAst| {
        let Some(
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::CreateTokenWithMods {
                            dynamic_power_toughness: Some((power, toughness)),
                            ..
                        },
                    ..
                }),
            ],
        ) = semantic_effects_for_test(line)
        else {
            panic!("expected one dynamic token-creation effect: {line:#?}");
        };
        for value in [power, toughness] {
            let Value::TotalPower(filter) = value else {
                panic!("expected total power of the matched death group, got {value:#?}");
            };
            assert!(filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
                    && constraint.tag.as_str() == ironsmith_core::ZONE_CHANGE_GROUP_TAG
            }));
        }
    };
    let full = format!("Whenever one or more nontoken creatures you control die, {effects}");
    let parsed = parse_triggered_text_for_test(
        &full,
        "one or more nontoken creatures you control die",
        effects,
    )
    .expect("aggregate death-group token creation should parse from intact source tokens");
    assert_dynamic_group_payload(&parsed);

    let effect_tail = lex_line(effects, 0).expect("effect-only source tail should lex");
    assert!(
        dynamic_zone_change_group_token_creation_from_authored_trigger(&effect_tail)
            .expect("effect-only public handoff should parse")
            .is_some(),
        "the prepared effect-tail route must retain the same dynamic group payload"
    );

    let fixed_full = "Whenever one or more nontoken creatures you control die, create a 0/0 green Fungus Dinosaur creature token.";
    let mut downgraded = parse_triggered_text_for_test(
        fixed_full,
        "one or more nontoken creatures you control die",
        "create a 0/0 green Fungus Dinosaur creature token.",
    )
    .expect("fixed token shell should parse before source reconciliation");
    let intact_source = lex_line(&full, 0).expect("intact dynamic source should lex");
    reconcile_dynamic_zone_change_group_token_creation(&mut downgraded, &intact_source)
        .expect("intact source should restore the dynamic payload after surface reparsing");
    assert_dynamic_group_payload(&downgraded);

    let fixed = lex_line(
        "Whenever one or more nontoken creatures you control die, create a 0/0 green Fungus Dinosaur creature token.",
        0,
    )
    .expect("fixed token near miss should lex");
    assert!(
        dynamic_zone_change_group_token_creation_from_authored_trigger(&fixed)
            .expect("near-miss probe should not error")
            .is_none()
    );
}

#[cfg(test)]
#[test]
pub(super) fn dynamic_static_ability_token_count_survives_the_public_trigger_handoff() {
    let effects = "create X Blood tokens, where X is the number of abilities from among flying, first strike, double strike, deathtouch, haste, hexproof, indestructible, lifelink, menace, reach, trample, and vigilance found among creatures you control.";
    let full = format!("When Odric enters, {effects}");
    let intact = lex_line(&full, 0).expect("authored static-ability aggregate should lex");
    let recovered = dynamic_static_ability_count_token_creation_from_authored_trigger(&intact)
        .expect("authored aggregate probe should not error")
        .expect("authored aggregate should be recovered from its create-verb boundary");
    let debug = format!("{recovered:#?}");
    assert!(debug.contains("StaticAbilitiesAmong"), "{debug}");
    assert!(debug.contains("Vigilance"), "{debug}");

    let fixed = lex_line(
        "When Odric enters, create X Blood tokens, where X is the number of creatures you control.",
        0,
    )
    .expect("ordinary count near miss should lex");
    assert!(
        dynamic_static_ability_count_token_creation_from_authored_trigger(&fixed)
            .expect("ordinary count should not error")
            .is_none()
    );
}

#[cfg(test)]
#[test]
pub(super) fn dynamic_exile_permission_bundle_reaches_the_public_trigger_route() {
    let effects = "exile cards equal to its power from the top of its owner's library. You may cast spells from among those cards for as long as they remain exiled, and mana of any type can be spent to cast them.";
    let authored_tokens = lex_line(effects, 0).expect("authored linked bundle should lex");
    assert!(
        is_authored_dynamic_exile_permission_bundle(&authored_tokens),
        "the public CST guard must normalize sentence casing and possessive apostrophes"
    );
    let full = format!("When enchanted creature dies, {effects}");
    let parsed = parse_triggered_text_for_test(&full, "enchanted creature dies", effects)
        .expect("linked dynamic exile trigger should parse");
    let parsed_effects = semantic_effects_for_test(&parsed)
        .unwrap_or_else(|| panic!("expected one triggered line: {parsed:#?}"));
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::ItsOwner,
                    ..
                },
            action:
                SubjectVerbActionAst::ExileTopOfLibrary {
                    count,
                    tags,
                    face_down: false,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag,
                    allow_land: false,
                    ..
                },
            ..
        }),
    ] = parsed_effects
    else {
        panic!("expected linked dynamic exile and permission: {parsed_effects:#?}");
    };
    assert_eq!(tags, std::slice::from_ref(tag));
    assert!(matches!(
        count.unhinted(),
        Value::PowerOf(spec)
            if matches!(spec.as_ref(), crate::target::ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
    ));

    // The public document route can hand semantic lowering an independently
    // simplified effect slice even though LineInfo still retains the exact
    // authored trigger line. That lossy slice must not erase the correlated
    // dynamic count, owner, or plural permission.
    let full_tokens = lex_line(&full, 0).expect("full authored trigger should lex");
    let trigger_tokens = lex_line("enchanted creature dies", 0).expect("trigger clause should lex");
    let lossy_effect_tokens = lex_line(
        "exile the top card of your library. You may cast that card for as long as it remains exiled, and mana of any type can be spent to cast that spell.",
        0,
    )
    .expect("lossy public effect slice should lex");
    let recovered = parse_triggered_line(
        LineInfo {
            line_index: 0,
            display_line_index: 0,
            raw_line: full.clone(),
            source_tokens: full_tokens.clone(),
            normalized: NormalizedLine {
                original: full.clone(),
                normalized: full.clone(),
                char_map: Vec::new(),
            },
            semantic_facts: Default::default(),
        },
        &full,
        &full_tokens,
        &trigger_tokens,
        &lossy_effect_tokens,
        None,
        None,
        None,
        None,
    )
    .expect("authored source tail should recover the linked bundle");
    let recovered_debug = format!("{recovered:#?}");
    for required in [
        "ExileTopOfLibrary",
        "PowerOf",
        "ItsOwner",
        "GrantPlayTaggedForAsLongAsExiled",
    ] {
        assert!(
            recovered_debug.contains(required),
            "missing {required}: {recovered_debug}"
        );
    }

    let near_miss = "exile the top card of its owner's library. You may cast spells from among those cards for as long as they remain exiled.";
    let near_miss_tokens = lex_line(near_miss, 0).expect("near miss should lex");
    assert!(!is_authored_dynamic_exile_permission_bundle(
        &near_miss_tokens
    ));
    if let Some(near_miss_effects) =
        crate::effect_sentences::parse_typed_effect_bundle_lexed(&near_miss_tokens)
    {
        let debug = format!("{near_miss_effects:#?}");
        assert!(
            !debug.contains("PowerOf"),
            "a fixed-count exile must not inherit the dynamic count: {debug}"
        );
    }
}

#[cfg(test)]
#[test]
pub(super) fn source_spell_surface_repair_does_not_erase_a_zone_change_trigger_arm() {
    let full = "Whenever you cast a white spell or a Plains you control enters, you gain 1 life.";
    let parsed = parse_triggered_text_for_test(
        full,
        "you cast a white spell or a Plains you control enters",
        "you gain 1 life.",
    )
    .expect("cast-or-entry trigger should reach the public semantic route");
    let trigger = match &parsed {
        LineAst::Triggered { trigger, .. } => trigger,
        LineAst::Ability(ability) => ability
            .trigger_spec
            .as_ref()
            .expect("runtime-backed trigger should retain its trigger spec"),
        _ => panic!("expected one triggered line: {parsed:#?}"),
    };
    assert!(
        matches!(
            trigger,
            TriggerSpec::WithIntro { trigger, .. }
                if matches!(trigger.as_ref(), TriggerSpec::Either(_, _))
        ),
        "{trigger:#?}"
    );
}

#[test]
pub(super) fn semantic_trigger_root_restores_single_target_source_exclusion() {
    fn spell_cast_target(trigger: &TriggerSpec) -> Option<&ObjectFilter> {
        match trigger {
            TriggerSpec::SpellCast {
                filter: Some(filter),
                ..
            } => filter.targets_only_object.as_deref(),
            TriggerSpec::WithIntro { trigger, .. } => spell_cast_target(trigger),
            TriggerSpec::Either(left, right) => {
                spell_cast_target(left).or_else(|| spell_cast_target(right))
            }
            TriggerSpec::AnyOf(branches) => branches.iter().find_map(spell_cast_target),
            _ => None,
        }
    }

    fn line_spell_cast_target(line: &LineAst) -> Option<&ObjectFilter> {
        match line {
            LineAst::Multiple(chunks) => chunks.iter().find_map(line_spell_cast_target),
            LineAst::Triggered { trigger, .. } => spell_cast_target(trigger),
            LineAst::Ability(parsed) => parsed.trigger_spec.as_deref().and_then(spell_cast_target),
            _ => None,
        }
    }

    let parse = |full_text: &str, trigger_text: &str| {
        parse_triggered_text_for_test(
            full_text,
            trigger_text,
            "you may copy that spell. The copy targets Ivy.",
        )
        .expect("semantic triggered line should parse")
    };

    let excluding = parse(
        "Whenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.",
        "a player casts a spell that targets only a single creature other than Ivy",
    );
    let trigger_tokens = lex_line(
        "a player casts a spell that targets only a single creature other than Ivy",
        0,
    )
    .expect("source-exclusion trigger should lex");
    let source_tokens = lex_line(
        "Whenever a player casts a spell that targets only a single creature other than Ivy, you may copy that spell. The copy targets Ivy.",
        0,
    )
    .expect("source-exclusion line should lex");
    assert_eq!(
        spell_cast_single_target_source_exclusion_surface(&trigger_tokens, &source_tokens),
        Some(crate::target::SourceReferenceSurface::ShortName(
            "Ivy".to_string()
        ))
    );
    let excluding_target = line_spell_cast_target(&excluding)
        .unwrap_or_else(|| panic!("missing nested spell target filter: {excluding:#?}"));
    assert!(excluding_target.other, "{excluding_target:#?}");
    assert_eq!(
        excluding_target.source_surface,
        Some(crate::target::SourceReferenceSurface::ShortName(
            "Ivy".to_string()
        ))
    );

    let ordinary = parse(
        "Whenever a player casts a spell that targets only a single creature, you may copy that spell. The copy targets Ivy.",
        "a player casts a spell that targets only a single creature",
    );
    let ordinary_target = line_spell_cast_target(&ordinary)
        .unwrap_or_else(|| panic!("missing ordinary nested spell target filter: {ordinary:#?}"));
    assert!(!ordinary_target.other, "{ordinary_target:#?}");
}

#[test]
pub(super) fn triggered_semantic_split_keeps_effect_backed_static_surfaces_in_resolution()
-> Result<(), CardTextError> {
    let linked_entry = parse_triggered_text_for_test(
        "Whenever a creature you control attacks alone, draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
        "a creature you control attacks alone",
        "draw a card. Then you may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
    )?;
    let linked_entry_debug = format!("{linked_entry:#?}");
    assert!(
        linked_entry_debug.contains("May")
            && linked_entry_debug.contains("battlefield_tapped: true")
            && linked_entry_debug.contains("battlefield_attacking: true")
            && linked_entry_debug.contains("GrantAbilitiesToTarget")
            && linked_entry_debug.contains("Indestructible"),
        "{linked_entry_debug}"
    );
    assert!(
        !linked_entry_debug.contains("StaticAbilities"),
        "the moved object's entry follow-up must not become a source static tail: \
         {linked_entry_debug}"
    );

    let conditional_create = parse_triggered_text_for_test(
        "Whenever you cast an artifact spell, you may pay {2}. If you do, create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
        "you cast an artifact spell",
        "you may pay {2}. If you do, create a 0/0 colorless Construct artifact creature token with \"This token gets +1/+1 for each artifact you control.\"",
    )?;
    let conditional_create_debug = format!("{conditional_create:#?}");
    assert!(
        conditional_create_debug.contains("IfResult")
            && conditional_create_debug.contains("CreateToken"),
        "{conditional_create_debug}"
    );
    assert!(
        !conditional_create_debug.contains("StaticAbilities"),
        "the token's quoted rule must not become a source static tail: \
         {conditional_create_debug}"
    );

    for (full_text, trigger_text, effect_text) in [
        (
            "At the beginning of each combat, you may reveal the top card of your library. If you reveal a creature card this way, this creature becomes a copy of that card until end of turn, except it has flying.",
            "at the beginning of each combat",
            "you may reveal the top card of your library. If you reveal a creature card this way, this creature becomes a copy of that card until end of turn, except it has flying.",
        ),
        (
            "Whenever one or more creatures you control are put into exile, you may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has flying.",
            "one or more creatures you control are put into exile",
            "you may choose a creature card from among them. Until end of turn, target token you control becomes a copy of it, except it has flying.",
        ),
    ] {
        let copy = parse_triggered_text_for_test(full_text, trigger_text, effect_text)?;
        let copy_debug = format!("{copy:#?}");
        assert!(
            copy_debug.contains("BecomeCopy") && copy_debug.contains("Flying"),
            "{copy_debug}"
        );
        assert!(
            !copy_debug.contains("StaticAbilities"),
            "the copy exception must not become a source static tail: {copy_debug}"
        );
    }

    fn contains_stack_copy(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::CopySpell { .. }
                    | crate::cards::builders::SubjectVerbActionAst::CopySpellForEachTarget { .. },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(contains_stack_copy)
        });
        found
    }

    fn contains_plural_retarget(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: crate::cards::builders::SubjectVerbActionAst::RetargetStackObject {
                    copy_reference_plural: true,
                    ..
                },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(contains_plural_retarget)
        });
        found
    }

    fn delayed_contains_copy_and_retarget(effect: &EffectAst) -> bool {
        match effect {
            EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedTriggerForDuration { effects, .. } => {
                return effects.iter().any(contains_stack_copy)
                    && effects.iter().any(contains_plural_retarget);
            }
            _ => {}
        }
        let mut found = false;
        crate::model::visit::for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(delayed_contains_copy_and_retarget);
        });
        found
    }

    let leori = parse_triggered_text_for_test(
        "Whenever this creature deals combat damage to a player, choose a planeswalker type. Until end of turn, whenever you activate an ability of a planeswalker of that type, copy that ability. You may choose new targets for the copies.",
        "this creature deals combat damage to a player",
        "choose a planeswalker type. Until end of turn, whenever you activate an ability of a planeswalker of that type, copy that ability. You may choose new targets for the copies.",
    )?;
    let leori_effects = match &leori {
        LineAst::Triggered { effects, .. } => effects.as_slice(),
        LineAst::Ability(parsed) => parsed
            .effects_ast
            .as_deref()
            .expect("Leori's triggered ability should preserve typed effects"),
        other => panic!("expected one Leori triggered ability: {other:#?}"),
    };
    assert!(
        leori_effects.iter().any(delayed_contains_copy_and_retarget),
        "the copied-object retarget must execute inside the delayed trigger: {leori:#?}"
    );

    Ok(())
}

pub(super) fn lower_spell_or_activated_ability_x_cost_trigger(
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

    Ok(Some(LineAst::Triggered {
        trigger: spell_or_activated_ability_x_cost_trigger_spec(),
        effects: parse_effect_sentences_lexed(effect_parse_tokens)?,
        max_triggers_per_turn,
    }))
}

pub fn parse_special_triggered_line(
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

pub(super) fn lower_special_rewrite_triggered_head(
    line: &RewriteTriggeredLine,
    full_parse_tokens: &[OwnedLexToken],
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if let Some(effects) = exact_target_same_name_graveyard_may_cast_bundle(effect_parse_tokens) {
        return Ok(Some(LineAst::Triggered {
            trigger: parse_trigger_clause_lexed(trigger_parse_tokens)?,
            effects,
            max_triggers_per_turn: line.max_triggers_per_turn,
        }));
    }
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
        let triggering_tag = crate::tag::CompilerReferenceTag::Triggering.key();
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
                    vec![KeywordAction::Marker("suspend").into()],
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
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None),
                vec![KeywordAction::FirstStrike.into()],
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

pub(super) fn lower_special_rewrite_triggered_divvy(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
    effect_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::DifferentNamesLibraryDivvy)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::ThisEntersBattlefield {
                origin_condition: None,
            }
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
            crate::tag::CompilerReferenceTag::DivvySource.key(),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(crate::tag::CompilerReferenceTag::DivvySource.key()),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: crate::tag::CompilerReferenceTag::DivvyChosen.key(),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::DivvyChosen.key(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::DivvySource.key(),
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

pub(super) fn lower_special_rewrite_triggered_oath(
    line: &RewriteTriggeredLine,
    trigger_parse_tokens: &[OwnedLexToken],
) -> Result<Option<LineAst>, CardTextError> {
    if matches!(
        semantic_grammar::parse_special_triggered_program_tokens(&line.full_parse_tokens),
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentLandMajoritySearch)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let mut basic_land = ObjectFilter::land().with_supertype(crate::types::Supertype::Basic);
        basic_land.set_explicit_card_noun(true);
        let effects = vec![
            EffectAst::subject_verb_explicit_target_only_for_chooser(
                TargetAst::Player(
                    PlayerFilter::OpponentWithMoreControlledObjectsThan {
                        player: Box::new(PlayerFilter::Active),
                        filter: Box::new(ObjectFilter::land()),
                    },
                    Some(crate::TextSpan::synthetic()),
                ),
                PlayerAst::Active,
            ),
            EffectAst::MayByPlayer {
                player: PlayerAst::Active,
                effects: vec![EffectAst::subject_verb_search_library(
                    basic_land,
                    Zone::Battlefield,
                    PlayerAst::Active,
                    PlayerAst::Active,
                    crate::effect::SearchSelectionMode::Exact,
                    false,
                    None,
                    true,
                    ChoiceCount::exactly(1),
                    None,
                    None,
                    crate::effect::SearchResultReferenceSurface::ThatCard,
                    false,
                    false,
                    false,
                )],
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
        Some(semantic_grammar::SpecialTriggeredProgram::OpponentCreatureMajorityConsult)
    ) {
        let trigger = if trigger_parse_tokens.is_empty() {
            TriggerSpec::BeginningOfUpkeep(PlayerFilter::Any)
        } else {
            parse_trigger_clause_lexed(trigger_parse_tokens)?
        };
        let revealed_tag = crate::tag::CompilerReferenceTag::OathRevealed.key();
        let creature_tag = crate::tag::CompilerReferenceTag::OathCreature.key();
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

pub(super) fn lower_special_rewrite_triggered_tail(
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
        let discarded_tag = crate::tag::CompilerReferenceTag::DiscardedThisWay.key();
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
                    mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
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
                    cost: ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Life(
                        Value::Fixed(life as i32),
                    )),
                    before_delayed_step: false,
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
                tag: crate::tag::CompilerReferenceTag::DivvyChosen.key(),
            },
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(crate::tag::CompilerReferenceTag::DivvyChosen.key()),
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

#[cfg(test)]
pub(super) fn test_rewrite_triggered_line(raw_line: &str, full_text: &str) -> RewriteTriggeredLine {
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
pub(super) fn triggered_line_source_text_keeps_raw_do_this_only_once_suffix() {
    let raw_line = "Whenever Pantlaza or another Dinosaur you control enters, you may discover X, where X is that creature's toughness. Do this only once each turn.";
    let full_text = "whenever pantlaza or another dinosaur you control enters, you may discover x, where x is that creature's toughness";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

#[test]
pub(super) fn triggered_line_source_text_keeps_labelled_raw_do_this_only_once_suffix() {
    let raw_line = "Mold Earth — Whenever one or more lands enter under an opponent's control without being played, you may search your library for a Plains card, put it onto the battlefield tapped, then shuffle. Do this only once each turn.";
    let full_text = "whenever one or more lands enter under an opponent's control without being played, you may search your library for a plains card, put it onto the battlefield tapped, then shuffle";
    let line = test_rewrite_triggered_line(raw_line, full_text);

    assert_eq!(triggered_line_source_text(&line), raw_line);
}

pub fn try_parse_optional_cost_with_cast_trigger(
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
        ironsmith_core::TotalCost::from_cost(crate::model::CompilerCost::Sacrifice {
            count: crate::effect::ChoiceCount::exactly(1),
            filter: filter.clone(),
            all: false,
            binding: None,
        }),
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

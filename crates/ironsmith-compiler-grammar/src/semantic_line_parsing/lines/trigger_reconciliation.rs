use super::*;

pub fn dynamic_zone_change_group_token_creation_from_authored_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let is_zone_change_group_total_power = |value: &Value| {
        matches!(
            value.unhinted(),
            Value::TotalPower(filter)
                if filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ZoneChangeGroup.as_str()
                        && constraint.relation
                            == crate::target::TaggedOpbjectRelation::IsTaggedObject
                })
        )
    };

    // The document route supplies the intact trigger line, while the
    // prepared triggered-chunk route supplies only the effect tail. Locate
    // the grammar-owned create head in either form instead of depending on a
    // generic comma split, which can select a trigger-side boundary.
    let mut starts = crate::lexer::parser_token_word_positions(tokens)
        .into_iter()
        .filter_map(|(index, word)| matches!(word, "create" | "creates").then_some(index))
        .collect::<Vec<_>>();
    starts.dedup();
    for start in starts.into_iter().rev() {
        let Ok(effect) = crate::effect_sentences::parse_create(&tokens[start..], None) else {
            continue;
        };
        let EffectAst::SubjectVerb(subject_verb) = &effect else {
            continue;
        };
        let SubjectVerbActionAst::CreateTokenWithMods {
            dynamic_power_toughness: Some((power, toughness)),
            ..
        } = &subject_verb.action
        else {
            continue;
        };
        if is_zone_change_group_total_power(power) && is_zone_change_group_total_power(toughness) {
            return Ok(Some(effect));
        }
    }
    Ok(None)
}

pub(super) fn dynamic_static_ability_count_token_creation_from_authored_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    // The authored aggregate itself contains commas (both after `tokens` and
    // throughout the ability list), so a generic comma split can start the
    // re-recognize at `where X is ...` and silently miss the create action. Try
    // actual create-verb token boundaries, from the last one backwards; the
    // typed value guard below keeps quoted or trigger-side creates from being
    // claimed.
    let mut starts = crate::lexer::parser_token_word_positions(tokens)
        .into_iter()
        .filter_map(|(index, word)| matches!(word, "create" | "creates").then_some(index))
        .collect::<Vec<_>>();
    starts.dedup();
    for start in starts.into_iter().rev() {
        let Ok(effects) = crate::effect_sentences::parse_effect_sentences_lexed(&tokens[start..])
        else {
            continue;
        };
        let [effect] = effects.as_slice() else {
            continue;
        };
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            continue;
        };
        let SubjectVerbActionAst::CreateTokenWithMods { count, .. } = &subject_verb.action else {
            continue;
        };
        if matches!(count.unhinted(), Value::StaticAbilitiesAmong { .. }) {
            return Ok(Some(effect.clone()));
        }
    }
    Ok(None)
}

pub(super) fn authored_dynamic_token_creation_from_trigger(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if let Some(effect) = dynamic_zone_change_group_token_creation_from_authored_trigger(tokens)? {
        return Ok(Some(effect));
    }
    dynamic_static_ability_count_token_creation_from_authored_trigger(tokens)
}

pub(super) fn recognize_dynamic_zone_change_group_token_creation(
    line: &mut LineAst,
    source_tokens: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    let Some(effect) = authored_dynamic_token_creation_from_trigger(source_tokens)? else {
        return Ok(());
    };
    match line {
        LineAst::Triggered { effects, .. } => *effects = vec![effect],
        LineAst::Ability(ability) => {
            ability.effects_ast = Some(vec![effect.clone()]);
            if let AbilityKind::Triggered(triggered) = ability.kind_mut() {
                triggered.effects = ironsmith_core::ResolutionProgram::from_effects(vec![effect]);
                triggered.choices.clear();
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                recognize_dynamic_zone_change_group_token_creation(chunk, source_tokens)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn set_triggered_effects(
    line: &mut LineAst,
    replacement: &[EffectAst],
) -> Result<(), CardTextError> {
    match line {
        LineAst::Triggered { effects, .. } => *effects = replacement.to_vec(),
        LineAst::Ability(ability) => {
            ability.effects_ast = Some(replacement.to_vec());
            if let AbilityKind::Triggered(triggered) = ability.kind_mut() {
                triggered.effects =
                    ironsmith_core::ResolutionProgram::from_effects(replacement.to_vec());
                triggered.choices.clear();
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                set_triggered_effects(chunk, replacement)?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn recognize_serial_target_pt_modifiers(
    line: &mut LineAst,
    source_tokens: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    let Some(split) = semantic_grammar::parse_comma_split_tokens(source_tokens) else {
        return Ok(());
    };
    let Some(replacement) =
        crate::effect_sentences::parse_serial_target_pt_modifiers_sentence(split.after)?
    else {
        return Ok(());
    };
    set_triggered_effects(line, &replacement)
}

fn replace_trigger_spec(line: &mut LineAst, replacement: &TriggerSpec) {
    match line {
        LineAst::Triggered { trigger, .. } => *trigger = replacement.clone(),
        LineAst::Ability(ability) => {
            ability.trigger_spec = Some(Box::new(replacement.clone()));
            if let AbilityKind::Triggered(triggered) = ability.kind_mut() {
                triggered.trigger = replacement.clone();
            }
        }
        LineAst::Multiple(chunks) => {
            for chunk in chunks {
                replace_trigger_spec(chunk, replacement);
            }
        }
        _ => {}
    }
}

pub(super) fn spell_or_activated_ability_x_cost_trigger_spec() -> TriggerSpec {
    let mut spell_filter = ObjectFilter::instant_or_sorcery();
    spell_filter.has_x_in_cost = true;
    let mut ability_filter = ObjectFilter::default();
    ability_filter.has_x_in_cost = true;
    TriggerSpec::Either(
        Box::new(TriggerSpec::SpellCast {
            filter: Some(spell_filter),
            mana_source_filter: None,
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
    )
}

pub(super) fn is_parley_word_program(words: &[&str]) -> bool {
    crate::word_primitives::sequence_occurs(
        words,
        &["each", "player", "reveals", "the", "top", "card", "of"],
    ) && crate::word_primitives::sequence_occurs(
        words,
        &["for", "each", "nonland", "card", "revealed", "this", "way"],
    ) && crate::word_primitives::sequence_occurs(words, &["each", "player", "draws", "a", "card"])
}

pub(super) fn is_gate_partition_core_word_program(words: &[&str]) -> bool {
    crate::word_primitives::sequence_occurs(
        words,
        &["look", "at", "the", "top", "nine", "cards", "of", "your"],
    ) && crate::word_primitives::sequence_occurs(
        words,
        &["put", "a", "gate", "card", "from", "among", "them"],
    ) && crate::word_primitives::sequence_occurs(
        words,
        &["if", "you", "control", "nine", "or", "more", "gates"],
    )
}

pub(super) fn is_gate_partition_word_program(words: &[&str]) -> bool {
    is_gate_partition_core_word_program(words)
        && crate::word_primitives::sequence_occurs(
            words,
            &["otherwise", "put", "the", "rest", "on"],
        )
}

pub fn end_of_combat_destroy_then_next_end_step_counter_program(
    effect_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(effect_tokens);
    let [destroy_sentence, counter_sentence] = sentences.as_slice() else {
        return None;
    };
    let destroy_words = crate::lexer::parser_token_word_refs(destroy_sentence);
    let counter_words = crate::lexer::parser_token_word_refs(counter_sentence);
    let exact_destroy = crate::word_primitives::first_is(&destroy_words, "destroy")
        && crate::word_primitives::sequence_occurs(
            &destroy_words,
            &["the", "other", "creature", "at", "end", "of", "combat"],
        );
    let exact_counter_followup = crate::word_primitives::sequence_occurs(
        &counter_words,
        &["at", "the", "beginning", "of", "the", "next", "end", "step"],
    ) && crate::word_primitives::sequence_occurs(
        &counter_words,
        &["if", "that", "creature", "was", "destroyed", "this", "way"],
    ) && crate::word_primitives::sequence_occurs(
        &counter_words,
        &["counter", "on", "the", "first", "creature"],
    );
    if !exact_destroy || !exact_counter_followup {
        return None;
    }

    let destroyed = ironsmith_core::PriorEffectResultSurface::new(
        ironsmith_core::PriorEffectAction::Destroyed,
        ObjectFilter::creature(),
        ironsmith_core::PriorEffectResultActor::Passive,
        ironsmith_core::PriorEffectResultQuantifier::One,
    );
    let destroy_other = EffectAst::subject_verb_destroy(TargetAst::Tagged(
        crate::tag::CompilerReferenceTag::Blocking.key(),
        None,
    ));
    let counter_first = EffectAst::subject_verb_put_counters(
        crate::object::CounterType::PlusOnePlusOne,
        Value::Fixed(1),
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Enchanted.key(), None),
        None,
        false,
    );
    Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![
            destroy_other,
            EffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::PriorEffectResult(destroyed),
                effects: vec![EffectAst::DelayedUntilNextEndStep {
                    player: PlayerFilter::Any,
                    effects: vec![counter_first],
                }],
            },
        ],
    }])
}

pub(super) fn recognize_authored_correlated_trigger_programs(
    line: &mut LineAst,
    source_tokens: &[OwnedLexToken],
) -> Result<(), CardTextError> {
    // `parse_comma_split_tokens` intentionally permits a greedy left side,
    // which is useful for ordinary clauses but can choose the comma inside a
    // later `if ...,` sentence. This grammar-proven two-sentence program owns
    // the first physical comma after its trigger header.
    if let Some(first_split) = crate::grammar::line_families::parse_comma_split(source_tokens) {
        let physical_tail = trim_lexed_commas(first_split.after);
        if let Some(effects) =
            end_of_combat_destroy_then_next_end_step_counter_program(physical_tail)
        {
            set_triggered_effects(line, &effects)?;
            return Ok(());
        }
    }
    let Some(split) = semantic_grammar::parse_comma_split_tokens(source_tokens) else {
        return Ok(());
    };
    let words = crate::lexer::parser_token_word_refs(split.after);

    if let Some(effects) = end_of_combat_destroy_then_next_end_step_counter_program(split.after) {
        set_triggered_effects(line, &effects)?;
        return Ok(());
    }

    let parley = is_parley_word_program(&words);
    if parley {
        let sentence_tokens = split_lexed_sentences(split.after);
        let terminal_draw =
            crate::slice_primitives::select_position(&sentence_tokens, |sentence| {
                crate::word_primitives::sequence_occurs(
                    &crate::lexer::parser_token_word_refs(sentence),
                    &["each", "player", "draws", "a", "card"],
                )
            });
        if let Some(terminal_draw) = terminal_draw {
            // Named-token lexing appends the token's reminder definition to
            // the source token stream. The grammar-proven terminal draw owns
            // the end of the authored Parley procedure; do not let a reminder
            // after it become another resolution instruction. Some Parley
            // programs have a token and a pump between reveal and draw, so a
            // fixed sentence count would discard a real final effect.
            let authored = sentence_tokens[..=terminal_draw]
                .iter()
                .map(|tokens| tokens.to_vec())
                .collect::<Vec<_>>();
            let authored = crate::util::join_sentences_with_period(&authored);
            let effects = parse_effect_sentences_lexed(&authored)?;
            set_triggered_effects(line, &effects)?;
            return Ok(());
        }
    }

    let gate_partition = is_gate_partition_word_program(&words);
    if gate_partition {
        let sentence_tokens = split_lexed_sentences(split.after);
        let sentences = sentence_tokens
            .iter()
            .map(|tokens| crate::effect_sentences::SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();
        if sentences.len() == 4
            && let Some(effects) = crate::effect_sentences::
                parse_look_at_top_optional_battlefield_then_conditional_remainder(&sentences, 0)?
        {
            set_triggered_effects(line, &effects)?;
            return Ok(());
        }
    }

    let full_words = crate::lexer::parser_token_word_refs(source_tokens);
    let spell_or_ability_x_cost =
        semantic_grammar::parse_spell_or_activated_ability_x_cost_trigger_tokens(
            source_tokens,
            split.before,
            split.after,
        )
        .is_some()
            || (crate::word_primitives::sequence_occurs(
                &full_words,
                &[
                    "you", "cast", "an", "instant", "or", "sorcery", "spell", "or", "activate",
                    "an", "ability",
                ],
            ) && crate::word_primitives::sequence_occurs(
                &full_words,
                &[
                    "copy", "that", "spell", "or", "ability", "you", "may", "choose", "new",
                ],
            ));
    if spell_or_ability_x_cost {
        replace_trigger_spec(line, &spell_or_activated_ability_x_cost_trigger_spec());
    }
    Ok(())
}

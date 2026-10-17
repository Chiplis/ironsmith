use super::*;
use crate::cards::builders::ObjectChoiceEffectAst;

pub(super) fn try_parse_chosen_type_behold_two_additional_cost(
    line: &RewriteKeywordLine,
    parse_tokens: &[OwnedLexToken],
) -> Option<LineAst> {
    let words = crate::lexer::parser_token_word_refs(parse_tokens);
    if line.kind != RewriteKeywordLineKind::AdditionalCost
        || !crate::word_primitives::parse_any_sequence_complete(
            &words,
            &[
                &[
                    "as",
                    "an",
                    "additional",
                    "cost",
                    "to",
                    "cast",
                    "this",
                    "spell",
                    "you",
                    "may",
                    "choose",
                    "a",
                    "creature",
                    "type",
                    "and",
                    "behold",
                    "two",
                    "cards",
                    "of",
                    "that",
                    "type",
                ],
                &[
                    "as",
                    "an",
                    "additional",
                    "cost",
                    "to",
                    "cast",
                    "this",
                    "spell",
                    "you",
                    "may",
                    "choose",
                    "a",
                    "creature",
                    "type",
                    "and",
                    "behold",
                    "two",
                    "creatures",
                    "of",
                    "that",
                    "type",
                ],
            ],
        )
    {
        return None;
    }

    let mut battlefield = ObjectFilter::creature()
        .controlled_by(PlayerFilter::You)
        .in_zone(Zone::Battlefield);
    battlefield.chosen_creature_type = true;
    let mut hand = ObjectFilter::default()
        .owned_by(PlayerFilter::You)
        .in_zone(Zone::Hand);
    hand.chosen_creature_type = true;
    let total_cost = ironsmith_core::TotalCost::from_costs(vec![
        crate::model::CompilerCost::ValidatedEffect(Box::new(
            EffectAst::subject_verb_choose_creature_type(PlayerAst::You, Vec::new()),
        )),
        crate::model::CompilerCost::ValidatedEffect(Box::new(EffectAst::ObjectChoices(
            ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter {
                    any_of: vec![battlefield, hand],
                    ..Default::default()
                },
                count: crate::effect::ChoiceCount::exactly(2),
                count_value: None,
                player: PlayerAst::You,
                tag: crate::tag::CompilerReferenceTag::BeheldChosenType.bind(),
            },
        ))),
    ]);
    let mut optional_cost = OptionalCost::custom(line.info.raw_line.trim(), total_cost);
    optional_cost.reference =
        crate::cost::OptionalCostRef::new(crate::cost::OptionalCostKind::Additional);
    Some(LineAst::OptionalCost(optional_cost))
}

/// "<trigger>, roll two six-sided dice. For each result of 1, <effect>. For
/// each other result, it gains the indicated ability until end of turn. [If
/// you rolled doubles, <effect>.]" followed by "• N — <keyword>" rows
/// (Celebr-8000): the bullets are a per-die result table, not modes. Both
/// dice are rolled once (in order); each die's result selects its row, and
/// doubles compare the two results.
fn try_parse_per_die_result_ability_table(
    modal: &RewriteModalBlock,
) -> Result<Option<ParsedCardItem>, CardTextError> {
    use crate::cards::builders::ConditionalEffectAst;
    use ironsmith_core::{EffectMetric, EffectMetricSource};
    let tokens = modal.header_tokens.as_slice();
    let Some(comma) = tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    if !tokens
        .first()
        .is_some_and(|token| token.is_any_word(&["at", "when", "whenever"]))
    {
        return Ok(None);
    }
    let sentences = crate::lexer::split_lexed_sentences(&tokens[comma + 1..]);
    let words_of = |sentence: &[OwnedLexToken]| {
        crate::lexer::token_word_refs(sentence)
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let Some(roll) = sentences.first() else {
        return Ok(None);
    };
    let roll_words = words_of(&roll[..]).join(" ");
    if !matches!(
        roll_words.as_str(),
        "roll two six-sided dice" | "roll two six sided dice" | "roll two d6"
    ) {
        return Ok(None);
    }
    let Some(first_row) = sentences.get(1) else {
        return Ok(None);
    };
    let first_row_words = words_of(&first_row[..]);
    let (Some(first_value), true) = (
        first_row_words
            .get(4)
            .and_then(|word| crate::util::parse_number_word_i32(word)),
        first_row_words.starts_with(&[
            "for".to_string(),
            "each".to_string(),
            "result".to_string(),
            "of".to_string(),
        ]),
    ) else {
        return Ok(None);
    };
    let Some(row_comma) = first_row.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    let Some(other_rows) = sentences.get(2) else {
        return Ok(None);
    };
    if words_of(&other_rows[..]).join(" ")
        != "for each other result it gains the indicated ability until end of turn"
    {
        return Ok(None);
    }
    let doubles_effects = match sentences.get(3) {
        None => None,
        Some(doubles) => {
            let doubles_words = words_of(&doubles[..]);
            if !doubles_words.starts_with(&[
                "if".to_string(),
                "you".to_string(),
                "rolled".to_string(),
                "doubles".to_string(),
            ]) {
                return Ok(None);
            }
            let Some(doubles_comma) = doubles.iter().position(|token| token.is_comma()) else {
                return Ok(None);
            };
            let body = &doubles[doubles_comma + 1..];
            let mut rewritten = crate::lexer::synthetic_word_tokens(["this", "creature"]);
            let mut skip = 0usize;
            if body.first().is_some_and(|token| token.is_word("it")) {
                skip = 1;
                if body.get(1).is_some_and(|token| token.is_word("also")) {
                    skip = 2;
                }
            } else {
                rewritten.clear();
            }
            rewritten.extend_from_slice(&body[skip..]);
            Some(crate::effect_sentences::parse_effect_sentence_lexed(&rewritten)?)
        }
    };
    if sentences.len() > 4 {
        return Ok(None);
    }
    let mut rows: Vec<(i32, Vec<EffectAst>)> = vec![(
        first_value,
        crate::effect_sentences::parse_effect_sentence_lexed(&first_row[row_comma + 1..])?,
    )];
    for mode in &modal.modes {
        let text = mode.text.trim().trim_start_matches('•').trim();
        let Some((value, ability)) = text.split_once('—') else {
            return Ok(None);
        };
        let Ok(value) = value.trim().parse::<i32>() else {
            return Ok(None);
        };
        let ability = ability.trim().trim_end_matches('.');
        if ability.is_empty() {
            return Ok(None);
        }
        let grant = crate::lexer::lex_line(
            &format!("this creature gains {ability} until end of turn."),
            mode.info.line_index,
        )
        .map_err(|_| {
            CardTextError::ParseError(format!("unsupported die-result ability row '{text}'"))
        })?;
        rows.push((value, crate::effect_sentences::parse_effect_sentence_lexed(&grant)?));
    }
    let trigger = parse_trigger_clause_lexed(&tokens[..comma])?;

    let mut roll_effect = EffectAst::subject_verb_roll_dice_choose_result_with_surface(
        PlayerAst::Implicit,
        2,
        6,
        None,
    );
    if let EffectAst::SubjectVerb(subject_verb) = &mut roll_effect
        && let SubjectVerbActionAst::Random(crate::cards::builders::RandomActionAst::RollDiceChooseResult {
            in_order,
            ..
        }) = &mut subject_verb.action
    {
        *in_order = true;
    }
    let die_value = |metric: EffectMetric| crate::effect::Value::PendingEffectMetric {
        source: EffectMetricSource::Outcome,
        metric,
    };
    let mut effects = vec![roll_effect];
    for metric in [EffectMetric::Count, EffectMetric::OtherNumber] {
        for (value, row_effects) in &rows {
            effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: die_value(metric),
                    operator: crate::effect::ValueComparisonOperator::Equal,
                    right: crate::effect::Value::Fixed(*value),
                },
                if_true: row_effects.clone(),
                if_false: Vec::new(),
            }));
        }
    }
    if let Some(doubles_effects) = doubles_effects {
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::ValueComparison {
                left: die_value(EffectMetric::Count),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: die_value(EffectMetric::OtherNumber),
            },
            if_true: doubles_effects,
            if_false: Vec::new(),
        }));
    }
    Ok(Some(ParsedCardItem::Line(crate::cards::builders::ParsedLineAst {
        info: modal.header.semantic_info(),
        chunks: vec![LineAst::Triggered {
            trigger,
            effects,
            max_triggers_per_turn: None,
        }],
        restrictions: ParsedRestrictions::default(),
        semantic_facts: modal.header.semantic_facts.clone(),
    })))
}

pub fn rewrite_modal_to_parsed_item(
    modal: RewriteModalBlock,
) -> Result<ParsedCardItem, CardTextError> {
    if let Some(item) = try_parse_per_die_result_ability_table(&modal)? {
        return Ok(item);
    }
    let header_text = modal.header.raw_line.clone();
    let Some(mut header) = parse_modal_header(&modal.header, &modal.header_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "rewrite modal lowering could not parse modal header '{}'",
            modal.header.raw_line
        )));
    };
    // A modal Saga chapter is a triggered modal ability whose trigger is the
    // chapter itself (CR 714.2b); its body cannot carry another trigger.
    if let Some(chapters) = modal.saga_chapters.clone() {
        if header.trigger.is_some() || header.activated.is_some() {
            return Err(CardTextError::ParseError(format!(
                "modal saga chapter body has its own trigger or cost: '{}'",
                modal.header.raw_line
            )));
        }
        header.trigger = Some(crate::cards::builders::TriggerSpec::SagaChapter(chapters));
    }

    let spending_rules = modal.modes.iter().map(|mode|
        crate::consumer_mana::split_x_spending_sentences(&mode.info.source_tokens).1)
        .collect::<Vec<_>>();
    if spending_rules.iter().any(|rules| !rules.is_empty()) {
        let common = &spending_rules[0];
        if spending_rules.iter().any(|rules| rules != common) {
            return Err(CardTextError::ParseError("different modal X spending requirements need mode-bound cost scopes".into()));
        }
        let activated = header.activated.as_mut().ok_or_else(||
            CardTextError::ParseError("modal X spending requirement has no activated cost owner".into()))?;
        activated.mana_cost = crate::consumer_mana::constrain_activation_cost(activated.mana_cost.clone(), common);
    }

    let positive_x_modes = modal.modes.iter().filter(|mode| {
        crate::grammar::structure::split_lexed_sentences(&mode.info.source_tokens)
            .into_iter()
            .any(crate::grammar::effects::dispatch_entry_shapes::is_x_cant_be_zero_tokens)
    }).count();
    if positive_x_modes > 0 {
        // A repeated restriction on every mode is an announcement constraint,
        // not an effect to discard. Do not strengthen mixed-mode restrictions
        // into a global minimum or silently lose them.
        let Some(activated) = header.activated.as_mut() else {
            return Err(CardTextError::ParseError(
                "mode-specific positive X requires an activated modal header".into(),
            ));
        };
        if positive_x_modes != modal.modes.len() && !activated.x_cant_be_zero {
            return Err(CardTextError::ParseError(
                "mixed mode-specific positive X restrictions are not yet represented".into(),
            ));
        }
        activated.x_cant_be_zero = true;
    }

    if let Some(replacement) = header.x_replacement.as_ref() {
        replace_modal_header_x_in_effects_ast(
            &mut header.common_prefix_effects_ast,
            replacement,
            header_text.as_str(),
        )?;
    }

    let mut modes = Vec::with_capacity(modal.modes.len());
    for mode in modal.modes {
        let mut effects_ast = mode.effects_ast;
        if let Some(replacement) = header.x_replacement.as_ref() {
            replace_modal_header_x_in_effects_ast(
                &mut effects_ast,
                replacement,
                header_text.as_str(),
            )?;
        }
        modes.push(ParsedModalModeAst {
            info: mode.info.semantic_info(),
            description: mode.text,
            point_cost: mode.point_cost,
            additional_mana_cost: mode.additional_mana_cost,
            effects_ast,
        });
    }

    specialize_modal_common_target_suffix(
        &mut modes,
        &header.common_suffix_effects_ast,
        header_text.as_str(),
    )?;

    Ok(ParsedCardItem::Modal(ParsedModalAst { header, modes }))
}

/// Specialize a demonstrative modal-header suffix into every bare target
/// mode. The shared clause supplies zone/controller facts, while each bullet
/// supplies its own target characteristic. Each resulting mode therefore
/// owns a complete executable target action, and the modal model separately
/// records that its trailing action was authored only once.
pub(super) fn specialize_modal_common_target_suffix(
    modes: &mut [ParsedModalModeAst],
    suffix: &[EffectAst],
    header_text: &str,
) -> Result<(), CardTextError> {
    if suffix.is_empty() {
        return Ok(());
    }
    let [EffectAst::SubjectVerb(common)] = suffix else {
        return Err(CardTextError::ParseError(format!(
            "unsupported modal common suffix in '{header_text}'"
        )));
    };
    let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand {
        target: TargetAst::Object(common_filter, _, _),
        ..
    }) = &common.action
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported modal common target action in '{header_text}'"
        )));
    };

    for mode in modes {
        let [EffectAst::SubjectVerb(mode_target)] = mode.effects_ast.as_slice() else {
            return Err(CardTextError::ParseError(format!(
                "modal common target suffix requires one bare target per mode in '{header_text}'"
            )));
        };
        let SubjectVerbActionAst::TargetOnly {
            target: TargetAst::Object(mode_filter, target_span, object_span),
            ..
        } = &mode_target.action
        else {
            return Err(CardTextError::ParseError(format!(
                "modal common target suffix requires object target modes in '{header_text}'"
            )));
        };

        let mut specialized = common.clone();
        let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnToHand { target, .. }) =
            &mut specialized.action
        else {
            unreachable!("common suffix action was validated above");
        };
        *target = TargetAst::Object(
            merge_filters(common_filter, mode_filter),
            *target_span,
            *object_span,
        );
        mode.effects_ast = vec![EffectAst::SubjectVerb(specialized)];
    }
    Ok(())
}

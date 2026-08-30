use super::*;

fn cost_words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    crate::word_primitives::sequence_occurs(words, phrase)
}

fn is_exact_per_target_cost_modifier(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(idx, word)| {
        *word == "for"
            && words.get(idx + 1) == Some(&"each")
            && words.get(idx + 2) == Some(&"target")
            && words.get(idx + 3) != Some(&"beyond")
    })
}

fn apply_anywhere_other_than_hand_origin(filter: &mut ObjectFilter) {
    filter.any_of = [
        (Zone::Hand, Some(PlayerFilter::NotYou)),
        (Zone::Library, None),
        (Zone::Battlefield, None),
        (Zone::Graveyard, None),
        (Zone::Exile, None),
        (Zone::Command, None),
        (Zone::OutsideGame, None),
    ]
    .into_iter()
    .map(|(zone, owner)| {
        let mut branch = ObjectFilter::default();
        branch.zone = Some(zone);
        branch.owner = owner;
        branch
    })
    .collect();
}

/// Parse a shared first-spell subject that receives both a cost reduction and
/// flash timing. The same typed filter drives both capabilities, including the
/// per-turn ordinal, spell type, and caster restriction.
pub fn parse_first_spell_cost_reduction_and_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    const TAIL: [&str; 9] = [
        "and", "can", "be", "cast", "as", "though", "it", "had", "flash",
    ];
    let words = crate::lexer::token_word_refs(tokens);
    if !crate::word_primitives::parse_sequence_suffix(&words, &TAIL) {
        return Ok(None);
    }
    // The ordinary reducer deliberately tolerates trailing coordination and
    // already proves the complete first-spell cost shape. Reuse that exact
    // proven reduction rather than maintaining a second token-boundary model.
    let Some(reduction) = parse_spells_cost_modifier_line(tokens)? else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::CostReduction(reduction_spec) = &reduction.payload
    else {
        return Ok(None);
    };
    if !reduction_spec.filter.first_spell_cast_each_turn
        || reduction_spec.filter.cast_by != Some(PlayerFilter::You)
    {
        return Ok(None);
    }
    let flash = StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::flash_to_spells_matching(
            reduction_spec.filter.clone(),
        ),
    );
    Ok(Some(vec![reduction.into(), flash.into()]))
}

/// Lower a source-card graveyard permission with a dynamic generic surcharge
/// into the two reusable static capabilities that enforce it. Keeping these
/// in one static source-line chunk preserves both runtime behavior and the
/// authored single-sentence presentation.
pub fn parse_source_graveyard_dynamic_surcharge_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(fact) =
        permission_graveyard_facts::parse_source_graveyard_dynamic_surcharge_tokens(tokens)
    else {
        return Ok(None);
    };
    let Some(parsed_cost) = parse_leaf_fixed_mana_cost_prefix_tokens(fact.cost_tokens) else {
        return Ok(None);
    };
    if parsed_cost.consumed != fact.cost_tokens.len() {
        return Ok(None);
    }
    let [pip] = parsed_cost.cost.pips() else {
        return Ok(None);
    };
    let [ManaSymbol::Generic(multiplier)] = pip.as_slice() else {
        return Ok(None);
    };
    if *multiplier == 0 {
        return Ok(None);
    }
    let Some(repetitions) = parse_dynamic_cost_modifier_value(fact.repetition_tokens)? else {
        return Ok(None);
    };
    let source_surface = render_token_slice(fact.source_tokens)
        .trim()
        .trim_end_matches('.')
        .to_string();
    if source_surface.is_empty() {
        return Ok(None);
    }

    let permission_filter = ObjectFilter::source_with_surface(
        crate::target::SourceReferenceSurface::ThisPermanentType(source_surface),
    );
    let permission = StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(),
            permission_filter,
            Zone::Graveyard,
        )
        .with_beneficiary(PlayerFilter::You),
    );
    let increase = StaticAbility::new(crate::static_abilities::CostIncrease::new(
        ObjectFilter::source(),
        scale_dynamic_cost_modifier_value(repetitions, i32::from(*multiplier)),
    ));

    Ok(Some(vec![permission.into(), increase.into()]))
}

fn apply_this_spell_cost_increase_condition(
    filter: &mut ObjectFilter,
    condition: crate::static_abilities::ThisSpellCostCondition,
    clause_words: &[&str],
) -> Result<Option<crate::ConditionExpr>, CardTextError> {
    use crate::static_abilities::ThisSpellCostCondition;

    match condition {
        ThisSpellCostCondition::Always => Ok(None),
        ThisSpellCostCondition::YourTurn => Ok(Some(crate::ConditionExpr::YourTurn)),
        ThisSpellCostCondition::NotYourTurn => Ok(Some(crate::ConditionExpr::Not(Box::new(
            crate::ConditionExpr::YourTurn,
        )))),
        ThisSpellCostCondition::ConditionExpr { condition, .. }
        | ThisSpellCostCondition::AsLongAsConditionExpr { condition, .. } => Ok(Some(condition)),
        ThisSpellCostCondition::TargetsPlayer(player) => {
            filter.targets_player = Some(player);
            Ok(None)
        }
        ThisSpellCostCondition::TargetsObject(object) => {
            filter.targets_object = Some(Box::new(object));
            Ok(None)
        }
        other => Err(CardTextError::ParseError(format!(
            "unsupported self-only cost-increase condition (clause: '{}'; condition: {other:?})",
            clause_words.join(" ")
        ))),
    }
}

pub fn parse_cost_modifier_target_spec(
    target_tokens: &[OwnedLexToken],
) -> Result<(Option<PlayerFilter>, Option<Box<ObjectFilter>>, bool), CardTextError> {
    let alternatives = crate::grammar::primitives::split_lexed_slices_on_or(target_tokens);
    if let [left, right] = alternatives.as_slice() {
        let (left_player, left_object, left_any_of) = parse_cost_modifier_target_spec(left)?;
        let (right_player, right_object, right_any_of) = parse_cost_modifier_target_spec(right)?;
        if !left_any_of && !right_any_of {
            match (left_player, left_object, right_player, right_object) {
                (Some(player), None, None, Some(object))
                | (None, Some(object), Some(player), None) => {
                    return Ok((Some(player), Some(object), true));
                }
                _ => {}
            }
        }
    }

    match static_mid_facts::parse_cost_modifier_target_fact(target_tokens) {
        Some(static_mid_facts::CostTargetFact::You) => Ok((Some(PlayerFilter::You), None, false)),
        Some(static_mid_facts::CostTargetFact::Opponent) => {
            Ok((Some(PlayerFilter::Opponent), None, false))
        }
        Some(static_mid_facts::CostTargetFact::AnyPlayer) => {
            Ok((Some(PlayerFilter::Any), None, false))
        }
        Some(static_mid_facts::CostTargetFact::Object(filter)) => {
            Ok((None, Some(Box::new(filter)), false))
        }
        None => Ok((
            None,
            Some(Box::new(parse_object_filter(target_tokens, false)?)),
            false,
        )),
    }
}

pub fn parse_cost_modifier_prefix_condition(
    tokens: &[OwnedLexToken],
    spells_token_idx: usize,
) -> Result<(Option<crate::ConditionExpr>, usize), CardTextError> {
    if let Some(prefix) =
        keyword_static_lines::parse_cost_prefix_condition_tokens(tokens, spells_token_idx)
    {
        match prefix {
            keyword_static_lines::CostPrefixCondition::DuringTurnsOtherThanYours {
                subject_start,
            } => {
                return Ok((
                    Some(crate::ConditionExpr::Not(Box::new(
                        crate::ConditionExpr::YourTurn,
                    ))),
                    subject_start,
                ));
            }
            keyword_static_lines::CostPrefixCondition::DuringYourTurn { subject_start } => {
                return Ok((Some(crate::ConditionExpr::YourTurn), subject_start));
            }
            keyword_static_lines::CostPrefixCondition::AsLongAs {
                condition_tokens,
                subject_start,
            } => {
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after leading 'as long as' clause (clause: '{}')",
                        crate::lexer::token_word_refs(tokens).join(" ")
                    )));
                }
                let condition = match parse_static_condition_clause(condition_tokens) {
                    Ok(condition) => condition,
                    Err(_) => parse_source_tap_status_condition_lexed(condition_tokens)
                        .ok_or_else(|| {
                            CardTextError::ParseError(format!(
                                "unsupported static condition clause (clause: '{}')",
                                crate::lexer::token_word_refs(condition_tokens).join(" ")
                            ))
                        })?,
                };
                return Ok((Some(condition), subject_start));
            }
        }
    }

    Ok((None, 0))
}

pub fn parse_optional_life_additional_cost_reduction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let additional_words = crate::lexer::token_word_refs(tokens);
    let Some(spec) = static_keyword_cost_shapes::parse_additional_cost_spell_filter(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.spell_filter_tokens);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_spell_filter_with_grammar_entrypoint(&subject_tokens);
    let subject_words = crate::lexer::token_word_refs(&subject_tokens);
    if static_keyword_cost_shapes::parse_optional_life_subject_is_permanent(&subject_words) {
        filter.card_types = ObjectFilter::permanent_card().card_types;
    }
    filter.cast_by = Some(PlayerFilter::You);

    let Some(optional_life_shape) =
        static_keyword_cost_shapes::parse_optional_life_reduction_words(&additional_words)
    else {
        return Ok(None);
    };
    let pay_word_idx = optional_life_shape.pay.word;
    let payment_words = &additional_words[pay_word_idx + 1..];
    let Some(life_cost) = payment_words
        .first()
        .and_then(|word| parse_number_word_i32(word))
        .and_then(|amount| u32::try_from(amount).ok())
    else {
        return Ok(None);
    };
    if !optional_life_shape.payment_has_life {
        return Ok(None);
    }

    if !optional_life_shape.those_spells_paid_life_this_way {
        return Ok(None);
    }
    let costs_word_idx = optional_life_shape.costs.word;
    let Some(costs_idx) = static_keyword_shapes::parse_word_token_offset(tokens, costs_word_idx)
    else {
        return Ok(None);
    };
    let amount_tokens = &tokens[costs_idx + 1..];
    let (_, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let Some((reduction, _)) = parsed_mana_cost else {
        return Ok(None);
    };
    let remaining_words = crate::lexer::token_word_refs(amount_tokens);
    if static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
        != Some(CostModifierDirection::Less)
        || !static_keyword_cost_shapes::parse_cost_modifier_cast_marker(&remaining_words)
    {
        return Ok(None);
    }

    let label_end = locate_token_kind(tokens, TokenKind::Period)
        .map(|idx| idx + 1)
        .unwrap_or(costs_idx);
    let label = render_token_slice(&tokens[..label_end])
        .trim()
        .trim_end_matches('.')
        .to_string();
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostReductionManaCost::new(filter, reduction)
            .with_optional_life_additional_cost(label, life_cost),
    )))
}

fn parse_cost_reduction_characteristic_intersection(
    tokens: &[OwnedLexToken],
) -> Result<Option<ironsmith_core::CostReductionCharacteristicIntersection>, CardTextError> {
    let characteristic_at = |start: usize| {
        if tokens.get(start).is_some_and(|token| token.is_word("card"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("type") || token.is_word("types"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::CardType, 2));
        }
        if tokens
            .get(start)
            .is_some_and(|token| token.is_word("permanent"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("type") || token.is_word("types"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::PermanentType, 2));
        }
        if tokens
            .get(start)
            .is_some_and(|token| token.is_word("creature"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("type") || token.is_word("types"))
        {
            return Some((
                ironsmith_core::ObjectCharacteristic::Subtype(
                    crate::types::SubtypeFamily::Creature,
                ),
                2,
            ));
        }
        if tokens
            .get(start)
            .is_some_and(|token| token.is_word("color"))
            || tokens
                .get(start)
                .is_some_and(|token| token.is_word("colors"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::Color, 1));
        }
        if tokens.get(start).is_some_and(|token| token.is_word("mana"))
            && tokens
                .get(start + 1)
                .is_some_and(|token| token.is_word("value"))
        {
            return Some((ironsmith_core::ObjectCharacteristic::ManaValue, 2));
        }
        None
    };

    for each_index in 0..tokens.len().saturating_sub(5) {
        if !tokens[each_index].is_word("for") || !tokens[each_index + 1].is_word("each") {
            continue;
        }
        let Some((characteristic, characteristic_len)) = characteristic_at(each_index + 2) else {
            continue;
        };
        let subject_index = each_index + 2 + characteristic_len;
        if !tokens
            .get(subject_index)
            .is_some_and(|token| token.is_word("they") || token.is_word("it"))
            || !tokens
                .get(subject_index + 1)
                .is_some_and(|token| token.is_word("share") || token.is_word("shares"))
            || !tokens
                .get(subject_index + 2)
                .is_some_and(|token| token.is_word("with"))
        {
            continue;
        }
        let comparison_tokens = trim_commas(&tokens[subject_index + 3..]);
        if comparison_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "missing comparison set after shared-characteristic cost reduction".to_string(),
            ));
        }
        let comparison = parse_object_filter(&comparison_tokens, false)?;
        let comparison_surface = render_token_slice(&comparison_tokens)
            .trim()
            .trim_end_matches('.')
            .to_string();
        return Ok(Some(
            ironsmith_core::CostReductionCharacteristicIntersection::new(
                characteristic,
                comparison,
            )
            .with_comparison_surface(comparison_surface),
        ));
    }

    Ok(None)
}

pub fn parse_spells_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if spell_additional_life_cost_per_target_amount(tokens).is_some() {
        return Ok(None);
    }
    if let Some(ability) = parse_optional_life_additional_cost_reduction_line(tokens)? {
        return Ok(Some(ability));
    }

    let clause_words = crate::lexer::parser_token_word_refs(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }
    let Some(spells_token_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    else {
        return Ok(None);
    };

    let first_spell_fact = static_mid_facts::parse_first_spell_each_turn_cost_fact(tokens);
    let second_spell_each_turn = crate::word_primitives::sequence_occurs(
        &clause_words,
        &["second", "spell", "you", "cast", "each", "turn"],
    );

    let (prefix_condition, subject_start) =
        parse_cost_modifier_prefix_condition(tokens, spells_token_idx)?;
    if subject_start > spells_token_idx {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[subject_start..spells_token_idx]);
    let is_this_spell = is_this_subject_reference_lexed(&subject_tokens);

    let Some(cost_token_idx) =
        static_mid_facts::parse_cost_component_boundary(tokens, spells_token_idx + 1)
            .map(|boundary| boundary.cost_token)
    else {
        return Ok(None);
    };
    if cost_token_idx <= spells_token_idx {
        return Ok(None);
    }

    let mut filter = if is_this_spell {
        ObjectFilter::default()
    } else {
        parse_spell_filter_with_grammar_entrypoint(&subject_tokens)
    };
    if first_spell_fact.is_some() && !is_this_spell {
        filter.first_spell_cast_each_turn = true;
    }
    if second_spell_each_turn && !is_this_spell {
        filter.spell_cast_ordinal_each_turn = Some(2);
    }

    let between_tokens = &tokens[spells_token_idx + 1..cost_token_idx];
    if !is_this_spell {
        let between_fact = static_mid_facts::parse_spell_cost_between_fact(between_tokens);
        let between_words = crate::lexer::parser_token_word_refs(between_tokens);
        for descriptor_tokens in between_fact.descriptor_segments {
            let extra_filter = parse_spell_filter_with_grammar_entrypoint(
                strip_relative_target_clause(descriptor_tokens),
            );
            if spell_filter_has_identity(&extra_filter) {
                merge_spell_filters(&mut filter, extra_filter);
            }
        }
        // The actor phrase is a separate fact — "with flying you cast" must
        // parse the quality without it, or the filter silently drops it.
        let mut between_for_filter: Vec<OwnedLexToken> =
            strip_relative_target_clause(between_tokens).to_vec();
        for actor_phrase in [
            &["you", "cast"][..],
            &["your", "opponents", "cast"],
            &["an", "opponent", "casts"],
        ] {
            let view = TokenWordView::new(&between_for_filter);
            let actor_words = view.to_word_refs();
            if let Some(word_pos) =
                crate::word_primitives::parse_sequence_start(&actor_words, actor_phrase)
                && let (Some(token_start), Some(token_end)) = (
                    view.map_word_to_token_start(word_pos),
                    view.token_index_after_words(word_pos + actor_phrase.len()),
                )
            {
                between_for_filter.drain(token_start..token_end);
                break;
            }
        }
        let between_filter = parse_spell_filter_with_grammar_entrypoint(&between_for_filter);
        if spell_filter_has_identity(&between_filter) {
            merge_spell_filters(&mut filter, between_filter);
        }
        match between_fact.actor {
            Some(static_mid_facts::SpellCastActorFact::You) => {
                filter.cast_by = Some(PlayerFilter::You)
            }
            Some(static_mid_facts::SpellCastActorFact::Opponent) => {
                filter.cast_by = Some(PlayerFilter::Opponent)
            }
            None => {}
        }
        if between_fact.from_your_graveyard {
            filter.zone = Some(Zone::Graveyard);
            filter.owner = Some(PlayerFilter::You);
        }
        // "Spells with the chosen name you cast cost {2} less to cast."
        // (Council of the Absolute) — the name constraint resolves against
        // the as-enters choice at runtime. The "this turn" form is a
        // one-shot effect (Cheering Fanatic) owned by the effect-shape
        // parser, not this static line.
        if (cost_words_contain_phrase(&between_words, &["with", "the", "chosen", "name"])
            || cost_words_contain_phrase(&between_words, &["with", "chosen", "name"]))
            && !cost_words_contain_phrase(&clause_words, &["this", "turn"])
        {
            filter.name = Some("{chosen name}".to_string());
        }
        if cost_words_contain_phrase(
            &between_words,
            &["from", "anywhere", "other", "than", "your", "hand"],
        ) {
            filter.zone = None;
            filter.owner = None;
            apply_anywhere_other_than_hand_origin(&mut filter);
        }
        if cost_words_contain_phrase(&between_words, &["but", "don't", "own"])
            || cost_words_contain_phrase(&between_words, &["but", "dont", "own"])
        {
            filter.owner = Some(PlayerFilter::NotYou);
        }
        if let Some(target_tokens) = between_fact.target_tokens {
            let (target_player, target_object, targets_any_of) =
                parse_cost_modifier_target_spec(target_tokens)?;
            filter.targets_player = target_player;
            filter.targets_object = target_object;
            filter.targets_any_of = targets_any_of;
        }
    }

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, mut parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let mut parsed_mana_cost_repetitions = None;
    let (mut amount_value, used) = parsed_amount.clone().unwrap_or({
        if let Some((_, used)) = &parsed_mana_cost {
            (Value::Fixed(1), *used)
        } else {
            (Value::Fixed(1), 0)
        }
    });
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::lexer::parser_token_word_refs(remaining_tokens);
    let if_boundary =
        static_keyword_cost_shapes::parse_cost_direction_if_boundary(&remaining_words)
            .map(|boundary| boundary.word);
    let as_long_as_boundary =
        crate::word_primitives::parse_sequence_start(&remaining_words, &["as", "long", "as"]);
    let condition_boundary = match (if_boundary, as_long_as_boundary) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(boundary), None) | (None, Some(boundary)) => Some(boundary),
        (None, None) => None,
    };
    let direction_words = condition_boundary
        .map(|boundary| &remaining_words[..boundary])
        .unwrap_or(&remaining_words);
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(direction_words)
    else {
        return Ok(None);
    };
    let is_life_cost_modifier =
        crate::word_primitives::sequence_occurs(&remaining_words, &["life"]);
    let per_target = !is_life_cost_modifier && is_exact_per_target_cost_modifier(&remaining_words);
    let per_additional_target = cost_words_contain_phrase(
        &remaining_words,
        &["for", "each", "target", "beyond", "the", "first"],
    );
    let characteristic_intersection = if direction == CostModifierDirection::Less {
        parse_cost_reduction_characteristic_intersection(remaining_tokens)?
    } else {
        None
    };

    let compound_this_spell_reduction = if direction == CostModifierDirection::Less
        && is_this_spell
        && !per_target
        && characteristic_intersection.is_none()
    {
        parsed_amount
            .as_ref()
            .and_then(|(value, _)| match value {
                Value::Fixed(multiplier) => Some(*multiplier),
                _ => None,
            })
            .map(|multiplier| {
                parse_compound_this_spell_cost_reduction_value(remaining_tokens, multiplier)
            })
            .transpose()?
            .flatten()
    } else {
        None
    };

    if let Some(compound) = compound_this_spell_reduction {
        parsed_mana_cost = None;
        amount_value = compound;
    } else if !per_target
        && characteristic_intersection.is_none()
        && !crate::word_primitives::sequence_occurs(&remaining_words, &["as", "long", "as"])
        && let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)?
    {
        if parsed_mana_cost.is_some() && is_this_spell {
            parsed_mana_cost_repetitions = Some(dynamic_value);
        } else {
            if parsed_mana_cost.is_some() {
                parsed_mana_cost = None;
            }
            let multiplier = parsed_amount
                .as_ref()
                .and_then(|(value, _)| match value {
                    Value::Fixed(value) => Some(*value),
                    _ => None,
                })
                .unwrap_or(1);
            amount_value = scale_dynamic_cost_modifier_value(dynamic_value, multiplier);
        }
    } else if parsed_amount.is_none() && parsed_mana_cost.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    // Handle trailing "where X is ..." clauses, e.g.
    // "This spell costs {X} less to cast, where X is the number of differently named lands you control."
    if let Some(where_tokens) = static_mid_facts::parse_where_x_clause_tokens(remaining_tokens) {
        let clause = clause_words.join(" ");
        let x_value = parse_value_binding_clause(where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause in spells-cost modifier (clause: '{clause}')"
            ))
        })?;
        if !value_contains_unbound_x(&amount_value) {
            return Err(CardTextError::ParseError(format!(
                "missing where-x clause in spells-cost modifier (clause: '{clause}')"
            )));
        }
        amount_value = replace_unbound_x_with_value(amount_value, &x_value, &clause)?;
    }
    if direction == CostModifierDirection::Less
        && let Some(cap) = parse_cost_reduction_cap(remaining_tokens)
    {
        amount_value = Value::Min(Box::new(amount_value), Box::new(Value::Fixed(cap)));
    }

    if !is_this_spell {
        parse_trailing_targets_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
        parse_trailing_candidate_ability_condition_in_cost_modifier(
            &mut filter,
            remaining_tokens,
            &clause_words,
        )?;
        let except_during_controller_turn = [
            &["except", "during", "its", "controller's", "turn"][..],
            &["except", "during", "its", "controllers", "turn"][..],
            &["except", "during", "its", "controller", "s", "turn"][..],
        ]
        .iter()
        .any(|phrase| cost_words_contain_phrase(&remaining_words, phrase));
        if except_during_controller_turn {
            let caster = filter.cast_by.take().unwrap_or(PlayerFilter::Any);
            filter.cast_by = Some(PlayerFilter::excluding(caster, PlayerFilter::Active));
            filter.set_except_during_controller_turn_surface(true);
        }
    }

    let this_spell_condition = if is_this_spell {
        if let Some(condition) =
            parse_trailing_this_spell_cost_condition(remaining_tokens, &clause_words)?
        {
            condition
        } else if let Some(prefix) = &prefix_condition {
            match prefix {
                crate::ConditionExpr::YourTurn => {
                    crate::static_abilities::ThisSpellCostCondition::YourTurn
                }
                crate::ConditionExpr::Not(inner)
                    if matches!(inner.as_ref(), crate::ConditionExpr::YourTurn) =>
                {
                    crate::static_abilities::ThisSpellCostCondition::NotYourTurn
                }
                other => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported leading this-spell cost condition (clause: '{}'; condition: {other:?})",
                        clause_words.join(" ")
                    )));
                }
            }
        } else {
            crate::static_abilities::ThisSpellCostCondition::Always
        }
    } else {
        crate::static_abilities::ThisSpellCostCondition::Always
    };

    let mut non_this_condition = if is_this_spell {
        None
    } else {
        prefix_condition.clone()
    };
    if first_spell_fact.is_some_and(|fact| fact.during_each_of_your_turns) && !is_this_spell {
        non_this_condition = Some(match non_this_condition.take() {
            Some(existing) => crate::ConditionExpr::And(
                Box::new(existing),
                Box::new(crate::ConditionExpr::YourTurn),
            ),
            None => crate::ConditionExpr::YourTurn,
        });
    }

    if direction == CostModifierDirection::Less {
        // "This spell costs {N} less to cast" is a self-only modifier that should not
        // apply from the permanent on the battlefield after it resolves.
        if is_this_spell && parsed_mana_cost.is_none() {
            return Ok(Some(StaticAbility::new(
                crate::static_abilities::ThisSpellCostReduction::new(
                    amount_value,
                    this_spell_condition,
                ),
            )));
        }
        if is_this_spell && let Some((cost, _)) = parsed_mana_cost.clone() {
            let mut ability = crate::static_abilities::ThisSpellCostReductionManaCost::new(
                cost,
                this_spell_condition,
            );
            if let Some(repetitions) = parsed_mana_cost_repetitions {
                ability = ability.with_repetitions(repetitions);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        if let Some((cost, _)) = parsed_mana_cost {
            let mut ability = crate::static_abilities::CostReductionManaCost::new(filter, cost);
            if per_target {
                ability = ability.with_per_target();
            }
            if let Some(condition) = non_this_condition.clone() {
                ability = ability.with_condition(condition);
            }
            return Ok(Some(StaticAbility::new(ability)));
        }
        let mut ability = crate::static_abilities::CostReduction::new(filter, amount_value);
        if let Some(intersection) = characteristic_intersection {
            ability = ability.with_characteristic_intersection(intersection);
        }
        if per_target {
            ability = ability.with_per_target();
        }
        if let Some(condition) = non_this_condition.clone() {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let source_only_increase = is_this_spell && !is_life_cost_modifier && !per_additional_target;
    let mut source_only_condition = None;
    if source_only_increase {
        filter = ObjectFilter::source();
        source_only_condition = apply_this_spell_cost_increase_condition(
            &mut filter,
            this_spell_condition,
            &clause_words,
        )?;
    }

    if let Some((cost, _)) = parsed_mana_cost {
        let mut ability = crate::static_abilities::CostIncreaseManaCost::new(filter, cost);
        if per_target {
            ability = ability.with_per_target();
        }
        if let Some(condition) = source_only_condition
            .clone()
            .or_else(|| non_this_condition.clone())
        {
            ability = ability.with_condition(condition);
        }
        return Ok(Some(StaticAbility::new(ability)));
    }

    let mut ability = crate::static_abilities::CostIncrease::new(filter, amount_value);
    if per_target {
        ability = ability.with_per_target();
    }
    if let Some(condition) = source_only_condition.or_else(|| non_this_condition.clone()) {
        ability = ability.with_condition(condition);
    }
    Ok(Some(StaticAbility::new(ability)))
}

/// Preserve two executable clauses that share one authored spell subject:
/// `Spells ... cost ... less to cast and can't be countered.`
///
/// The ordinary cost-modifier parser intentionally ignores trailing prose
/// after a complete `less to cast` direction. Claim only the exact terminal
/// countering conjunction here and reuse the typed spell filter from the cost
/// reduction for the restriction.
pub fn parse_spells_cost_reduction_and_cant_be_countered_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(and_index) =
        crate::slice_primitives::select_last_position(tokens, |token| token.is_word("and"))
    else {
        return Ok(None);
    };
    if !crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(&tokens[and_index + 1..]),
        &["cant", "be", "countered"],
    ) {
        return Ok(None);
    }
    let left = trim_lexed_commas(&tokens[..and_index]);
    let Some(reduction) = parse_spells_cost_modifier_line(left)? else {
        return Ok(None);
    };
    let ironsmith_core::StaticAbilityPayload::CostReduction(payload) = &reduction.payload else {
        return Ok(None);
    };
    let filter = payload.filter.clone();
    let restriction = StaticAbility::restriction(
        crate::effect::Restriction::be_countered(filter.clone()),
        format!("{} can't be countered", filter.description()),
    );
    Ok(Some(vec![reduction, restriction]))
}

pub fn parse_spell_and_player_activated_ability_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(and_idx) = static_keyword_cost_shapes::parse_spell_and_abilities_separator(tokens)
        .map(|boundary| boundary.token)
    else {
        return Ok(None);
    };
    let right_start = and_idx + 1;

    let left_tokens = trim_commas(&tokens[..and_idx]);
    let right_tokens = trim_commas(&tokens[right_start..]);
    let Some(spell_cost_ability) = parse_spells_cost_modifier_line(&left_tokens)? else {
        return Ok(None);
    };
    let Some(mut activated_cost_ability) =
        parse_player_activated_ability_cost_modifier_clause(&right_tokens)?
    else {
        return Ok(None);
    };

    if let Some(spells_idx) =
        static_keyword_cost_shapes::parse_spells_subject(tokens).map(|boundary| boundary.token)
    {
        let (prefix_condition, _) = parse_cost_modifier_prefix_condition(tokens, spells_idx)?;
        if let Some(condition) = prefix_condition {
            activated_cost_ability = activated_cost_ability.with_condition(condition);
        }
    }

    Ok(Some(vec![spell_cost_ability, activated_cost_ability]))
}

pub fn parse_cycling_cost_alternative_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some(fact) = static_mid_facts::parse_cycling_cost_alternative_fact(tokens) else {
        return Ok(None);
    };

    let condition = fact
        .condition_tokens
        .map(parse_static_condition_clause)
        .transpose()?;
    let replacement_mana_cost = if fact.replacement_cost_tokens.is_empty() {
        ManaCost::new()
    } else {
        let replacement_total_cost = parse_compiler_activation_cost(fact.replacement_cost_tokens)?;
        if replacement_total_cost.has_non_mana_costs() {
            return Err(CardTextError::ParseError(format!(
                "unsupported non-mana cycling alternative cost (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        replacement_total_cost.mana_cost().cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing cycling alternative mana cost (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let mut filter = ObjectFilter::default().with_ability_marker("cycling");
    filter.zone = Some(Zone::Hand);
    let display = format!(
        "You may pay {} rather than pay cycling costs",
        replacement_mana_cost.to_oracle()
    );
    let mut ability =
        StaticAbility::replace_activated_ability_mana_cost(filter, replacement_mana_cost, display);
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }
    Ok(Some(ability))
}

pub fn parse_player_activated_ability_cost_modifier_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if clause_words.len() < 7 || clause_words.first().is_none_or(|word| *word != "abilities") {
        return Ok(None);
    }

    let Some(cost_words) =
        static_keyword_cost_shapes::parse_player_ability_cost_words(&clause_words)
    else {
        return Ok(None);
    };
    let activate_idx = cost_words.activate.word;
    let clause = LexedClause::new(tokens);
    let Some(activator_clause) = clause.between_word_range(1, activate_idx) else {
        return Ok(None);
    };
    let activator =
        match static_mid_facts::parse_activated_ability_cost_actor(activator_clause.tokens()) {
            Some(static_mid_facts::ActivatedAbilityCostActorFact::You) => PlayerFilter::You,
            Some(static_mid_facts::ActivatedAbilityCostActorFact::Opponent) => {
                PlayerFilter::Opponent
            }
            None => return Ok(None),
        };

    let cost_idx = cost_words.costs.word;
    let cost_token_idx = static_keyword_shapes::parse_word_token_offset(tokens, cost_idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map activated-ability cost modifier amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let amount_tokens = &tokens[cost_token_idx + 1..];
    let (parsed_amount, parsed_mana_cost) = parse_cost_modifier_components(amount_tokens);
    let (increase, used) = if let Some((mana_cost, used)) = parsed_mana_cost {
        (
            ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(mana_cost),
            used,
        )
    } else if let Some((Value::Fixed(amount), used)) = parsed_amount {
        if amount < 0 {
            return Ok(None);
        }
        let generic = amount.min(u8::MAX as i32) as u8;
        (
            ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(ManaCost::from_symbols(
                vec![ManaSymbol::Generic(generic)],
            )),
            used,
        )
    } else {
        return Ok(None);
    };
    let remaining_tokens = amount_tokens.get(used..).unwrap_or_default();
    let remaining_words = crate::lexer::token_word_refs(remaining_tokens);
    let Some(tail_fact) = static_mid_facts::parse_activated_ability_cost_tail(remaining_tokens)
    else {
        return Ok(None);
    };
    if static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
        != Some(CostModifierDirection::More)
    {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::increase_activated_ability_costs_for_activator(
            activator,
            increase,
            tail_fact.excludes_mana_abilities,
        ),
    ))
}

pub fn strip_relative_target_clause(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    let Some(target_clause_idx) = static_keyword_cost_shapes::parse_relative_target_clause(tokens)
        .map(|boundary| boundary.token)
    else {
        return tokens;
    };

    &tokens[..target_clause_idx]
}

pub fn parse_trailing_targets_condition_in_cost_modifier(
    filter: &mut ObjectFilter,
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let Some(fact) = static_mid_facts::parse_trailing_target_condition(remaining_tokens) else {
        return Ok(());
    };
    if fact.target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in trailing spells-cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let (targets_player, targets_object, targets_any_of) =
        parse_cost_modifier_target_spec(fact.target_tokens)?;
    filter.targets_player = targets_player;
    filter.targets_object = targets_object;
    filter.targets_any_of = targets_any_of;
    Ok(())
}

fn parse_trailing_candidate_ability_condition_in_cost_modifier(
    filter: &mut ObjectFilter,
    remaining_tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let remaining_words = crate::lexer::token_word_refs(remaining_tokens);
    let Some(if_idx) =
        static_keyword_cost_shapes::parse_trailing_cost_condition_if(&remaining_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(());
    };
    let Some(keyword_words) = remaining_words.get(if_idx + 3..) else {
        return Ok(());
    };
    if !crate::word_primitives::parse_sequence_prefix(
        &remaining_words[if_idx + 1..],
        &["it", "has"],
    ) {
        return Ok(());
    }
    let Some((constraints, _, consumed)) =
        crate::util::parse_filter_keyword_constraint_list_words(keyword_words)
    else {
        return Err(CardTextError::ParseError(format!(
            "unsupported candidate-spell ability condition (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if constraints.is_empty() || consumed != keyword_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported candidate-spell ability condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    for constraint in constraints {
        crate::util::apply_filter_keyword_constraint(filter, constraint, false);
    }
    filter.set_trailing_candidate_ability_condition_surface(true);
    Ok(())
}

pub fn parse_flashback_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    let Some((kind, consumed)) = parse_alternative_cast_words(&clause_words) else {
        return Ok(None);
    };
    if clause_words.len() < consumed + 5 {
        return Ok(None);
    }
    if clause_words.get(consumed).copied() != Some("costs") {
        return Ok(None);
    }
    let Some(cost_idx) =
        static_keyword_cost_shapes::parse_last_cost_verb(tokens).map(|boundary| boundary.token)
    else {
        return Ok(None);
    };
    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = crate::lexer::token_word_refs(remaining_tokens);
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
    else {
        return Ok(None);
    };
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }

    let mut filter = ObjectFilter::default();
    filter.alternative_cast = Some(kind);
    match static_mid_facts::parse_alternative_cost_payer(tokens) {
        Some(static_mid_facts::AlternativeCostPayerFact::You) => {
            filter.cast_by = Some(PlayerFilter::You)
        }
        Some(static_mid_facts::AlternativeCostPayerFact::Opponent) => {
            filter.cast_by = Some(PlayerFilter::Opponent)
        }
        None => {}
    }

    if direction == CostModifierDirection::Less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

pub fn parse_equip_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(head) = keyword_static_lines::parse_equip_cost_modifier_head_tokens(tokens) else {
        return Ok(None);
    };
    let cost_idx = head.cost_token;

    let amount_tokens = &tokens[cost_idx + 1..];
    let Some((amount_value, used)) = parse_cost_modifier_amount(amount_tokens) else {
        return Ok(None);
    };
    let Value::Fixed(amount) = amount_value else {
        return Ok(None);
    };
    if amount < 0 {
        return Ok(None);
    }

    let remaining_words = crate::lexer::token_word_refs(&amount_tokens[used..]);
    let Some(direction) = static_mid_facts::parse_cost_modifier_direction_words(&remaining_words)
    else {
        return Ok(None);
    };

    let mut filter = if head.source_relative_equipment {
        ObjectFilter::source().with_ability_marker("equip")
    } else {
        ObjectFilter::default().with_ability_marker("equip")
    };
    match head.payer {
        keyword_static_lines::EquipCostPayer::You => filter.controller = Some(PlayerFilter::You),
        keyword_static_lines::EquipCostPayer::Opponent => {
            filter.controller = Some(PlayerFilter::Opponent)
        }
        keyword_static_lines::EquipCostPayer::Unspecified => {}
    }

    if direction == CostModifierDirection::Less {
        let amount_text = format!("{{{amount}}}");
        let display = if head.source_relative_equipment {
            format!("This Equipment's equip abilities cost {amount_text} less to activate")
        } else if filter.controller == Some(PlayerFilter::Opponent) {
            format!("Equip costs your opponents pay cost {amount_text} less")
        } else {
            format!("Equip costs you pay cost {amount_text} less")
        };
        return Ok(Some(
            StaticAbility::reduce_activated_ability_costs_with_display(
                filter,
                amount as u32,
                None,
                display,
            ),
        ));
    }

    let increase = ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(
        ManaCost::from_symbols(vec![ManaSymbol::Generic(amount.min(u8::MAX as i32) as u8)]),
    );
    Ok(Some(StaticAbility::increase_activated_ability_costs(
        filter, increase,
    )))
}

pub fn parse_foretelling_cards_cost_modifier_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = crate::lexer::token_word_refs(tokens);
    if clause_words.len() < 7 {
        return Ok(None);
    }
    let Some(fact) = static_mid_facts::parse_foretell_cost_modifier_fact(tokens) else {
        return Ok(None);
    };

    if fact.direction != CostModifierDirection::Less || !fact.during_any_players_turn {
        return Ok(None);
    }

    Err(CardTextError::ParseError(format!(
        "unsupported foretelling cost modifier clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

pub fn parse_cost_modifier_amount(tokens: &[OwnedLexToken]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let first_token = tokens.first()?;
    let group = mana_pips_from_token(first_token)?;
    if group.len() != 1 {
        return None;
    }
    let symbol = group[0];
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    if symbol == ManaSymbol::X {
        return Some((Value::X, 1));
    }
    None
}

pub fn parse_cost_modifier_mana_cost(
    tokens: &[OwnedLexToken],
) -> Option<(crate::mana::ManaCost, usize)> {
    let parsed = parse_leaf_fixed_mana_cost_prefix_tokens(tokens)?;
    Some((parsed.cost, parsed.consumed))
}

pub fn parse_cost_modifier_components(
    amount_tokens: &[OwnedLexToken],
) -> (
    Option<(Value, usize)>,
    Option<(crate::mana::ManaCost, usize)>,
) {
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let parsed_mana_cost = parse_cost_modifier_mana_cost(amount_tokens);

    let amount_used = parsed_amount.as_ref().map(|(_, used)| *used).unwrap_or(0);
    let mana_used = parsed_mana_cost
        .as_ref()
        .map(|(_, used)| *used)
        .unwrap_or(0);

    // Prefer mana-symbol parsing when it consumes a longer contiguous mana sequence
    // (e.g. "{2}{U}{U}" should stay a single mana-cost reduction component).
    if mana_used > amount_used {
        return (None, parsed_mana_cost);
    }

    (parsed_amount, None)
}

fn parse_compound_this_spell_cost_reduction_value(
    tokens: &[OwnedLexToken],
    first_multiplier: i32,
) -> Result<Option<Value>, CardTextError> {
    let mut boundaries = Vec::new();
    for (and_index, token) in tokens.iter().enumerate() {
        if !token.is_word("and") {
            continue;
        }
        let Some((Value::Fixed(multiplier), amount_used)) =
            parse_cost_modifier_amount(&tokens[and_index + 1..])
        else {
            continue;
        };
        let second_start = and_index + 1 + amount_used;
        if !tokens
            .get(second_start)
            .is_some_and(|token| token.is_word("less"))
            || !tokens
                .get(second_start + 1)
                .is_some_and(|token| token.is_word("to"))
            || !tokens
                .get(second_start + 2)
                .is_some_and(|token| token.is_word("cast"))
        {
            continue;
        }
        boundaries.push((and_index, multiplier, second_start));
    }
    if boundaries.is_empty() {
        return Ok(None);
    }

    let first_end = boundaries[0].0;
    let Some(first_value) = parse_dynamic_cost_modifier_value(&tokens[..first_end])? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported first dynamic term in compound cost reduction (clause: '{}')",
            parser_token_word_refs(tokens).join(" ")
        )));
    };
    let mut value = scale_dynamic_cost_modifier_value(first_value, first_multiplier);

    for (index, (_, multiplier, segment_start)) in boundaries.iter().copied().enumerate() {
        let segment_end = boundaries
            .get(index + 1)
            .map(|boundary| boundary.0)
            .unwrap_or(tokens.len());
        let Some(term) = parse_dynamic_cost_modifier_value(&tokens[segment_start..segment_end])?
        else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing dynamic term in compound cost reduction (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        };
        value = Value::Add(
            Box::new(value),
            Box::new(scale_dynamic_cost_modifier_value(term, multiplier)),
        );
    }

    Ok(Some(value))
}

pub fn parse_cost_reduction_cap(tokens: &[OwnedLexToken]) -> Option<i32> {
    for idx in 2..tokens.len().saturating_sub(1) {
        if !tokens[idx - 2].is_word("by")
            || !tokens[idx - 1].is_word("more")
            || !tokens[idx].is_word("than")
        {
            continue;
        }
        let group = mana_pips_from_token(tokens.get(idx + 1)?)?;
        if group.len() != 1 {
            return None;
        }
        return match group[0] {
            ManaSymbol::Generic(amount) => Some(amount as i32),
            _ => None,
        };
    }
    None
}

pub fn parse_dynamic_cost_modifier_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    use keyword_static_lines::{
        CounterReferenceKind, DynamicCostValueShape, DynamicPlayerKind, DynamicThisWayMetric,
        SpellCastDynamicKind,
    };

    let for_each_value_tokens = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)
        .and_then(|boundary| tokens.get(boundary.token.saturating_add(1)..));
    let history_tokens = for_each_value_tokens.unwrap_or(tokens);
    let with_for_each_surface = |value: Value| {
        if for_each_value_tokens.is_some() {
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
        } else {
            value
        }
    };
    let parsed_shape = keyword_static_lines::parse_dynamic_cost_value_shape_tokens(tokens);
    // A card-types-among aggregate can contain a historical qualifier in its
    // object scope ("permanents you've sacrificed this turn"). Classify the
    // outer aggregate before the generic history parser sees that nested
    // phrase and incorrectly turns the whole value into a sacrifice count.
    match parsed_shape {
        Some(DynamicCostValueShape::CardTypesAmong { scope_tokens }) => {
            let Ok(filter) = parse_object_filter(scope_tokens, false) else {
                return Ok(None);
            };
            return Ok(Some(with_for_each_surface(Value::CardTypesAmong(filter))));
        }
        Some(DynamicCostValueShape::UnsupportedCardTypesAmong) => {
            return Err(CardTextError::ParseError(format!(
                "unsupported card-types-among dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        _ => {}
    }
    if let Some(value) =
        crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(history_tokens)
    {
        return Ok(Some(with_for_each_surface(value)));
    }
    let Some(shape) = parsed_shape else {
        return Ok(None);
    };
    let player_filter = |player| match player {
        DynamicPlayerKind::You => PlayerFilter::You,
        DynamicPlayerKind::Opponent => PlayerFilter::Opponent,
        DynamicPlayerKind::Any => PlayerFilter::Any,
    };
    let value = match shape {
        DynamicCostValueShape::CardsDrawn(player) => {
            with_for_each_surface(Value::MaxCardsDrawnThisTurn(player_filter(player)))
        }
        DynamicCostValueShape::LifeGained(player) => {
            with_for_each_surface(Value::LifeGainedThisTurn(player_filter(player)))
        }
        DynamicCostValueShape::KickCount => Value::KickCount,
        DynamicCostValueShape::CreaturesDiedThisTurn => Value::CreaturesDiedThisTurn,
        DynamicCostValueShape::OpponentsLifeLostThisTurn => {
            Value::LifeLostThisTurn(PlayerFilter::Opponent)
        }
        DynamicCostValueShape::ControlledCreaturesDiedThisTurn => {
            Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You)
        }
        DynamicCostValueShape::PlayersBeingAttacked => {
            with_for_each_surface(Value::PlayersBeingAttacked)
        }
        DynamicCostValueShape::SpellCast { player, kind } => {
            let player = player_filter(player);
            match kind {
                SpellCastDynamicKind::CardTypes => {
                    let mut filter = ObjectFilter::spell();
                    filter.cast_by = Some(player);
                    Value::CardTypesAmong(filter)
                }
                SpellCastDynamicKind::OtherThanFirst => Value::Add(
                    Box::new(Value::SpellsCastThisTurn(player)),
                    Box::new(Value::Fixed(-1)),
                ),
                SpellCastDynamicKind::MatchingTypes {
                    instant,
                    sorcery,
                    exclude_source,
                } => {
                    let mut filter = ObjectFilter::spell();
                    filter.card_types = match (instant, sorcery) {
                        (true, true) => vec![CardType::Instant, CardType::Sorcery],
                        (true, false) => vec![CardType::Instant],
                        (false, true) => vec![CardType::Sorcery],
                        (false, false) => Vec::new(),
                    };
                    Value::SpellsCastThisTurnMatching {
                        player,
                        filter,
                        exclude_source,
                    }
                }
                SpellCastDynamicKind::Simple => Value::SpellsCastThisTurn(player),
            }
        }
        DynamicCostValueShape::CardTypesInGraveyard(player) => {
            Value::CardTypesInGraveyard(player_filter(player))
        }
        DynamicCostValueShape::ColorsSpentToCastThisSpell => {
            Value::ColorsOfManaSpentToCastThisSpell
        }
        DynamicCostValueShape::PartySize => Value::PartySize(PlayerFilter::You),
        DynamicCostValueShape::AggregateScope => {
            let each_idx = static_keyword_cost_shapes::parse_dynamic_cost_each_word(tokens)
                .map(|boundary| boundary.token)
                .unwrap_or(0);
            let filter_tokens = tokens.get(each_idx + 1..).unwrap_or_default();
            let Some(value) = parse_aggregate_scope_value_lexed(filter_tokens) else {
                return Ok(None);
            };
            value
        }
        DynamicCostValueShape::CardTypesAmong { .. }
        | DynamicCostValueShape::UnsupportedCardTypesAmong => {
            unreachable!("card-types-among values are handled before history fallbacks")
        }
        DynamicCostValueShape::CountersRemovedThisWay => Value::PendingPriorEffectMetric(
            ironsmith_core::PriorEffectMetricQuery::new(
                EffectMetricSource::Outcome,
                EffectMetric::Count,
            )
            .with_action(ironsmith_core::PriorEffectAction::Removed),
        )
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::CountersRemovedThisWay),
        DynamicCostValueShape::PlayerCounters(counter_type) => {
            Value::PlayerCounters(PlayerFilter::You, counter_type)
        }
        DynamicCostValueShape::ThisWayMetric(metric) => match metric {
            DynamicThisWayMetric::Destroyed => {
                let mut count_words = vec!["for", "each"];
                count_words.extend(parser_token_word_refs(history_tokens));
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                    && matches!(value.unhinted(), Value::PendingPriorEffectMetric(_))
                {
                    value
                } else {
                    Value::PendingEffectMetric {
                        source: EffectMetricSource::AffectedObjects,
                        metric: EffectMetric::Count,
                    }
                }
            }
            DynamicThisWayMetric::Sacrificed => {
                let words = parser_token_word_refs(history_tokens);
                let mut count_words = vec!["for", "each"];
                count_words.extend(words.iter().copied());
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                    && matches!(value.unhinted(), Value::PendingPriorEffectMetric(_))
                {
                    with_for_each_surface(value)
                } else {
                    let kind = if words
                        .iter()
                        .any(|word| matches!(*word, "creature" | "creatures"))
                    {
                        ironsmith_core::SacrificedObjectKind::Creature
                    } else if words
                        .iter()
                        .any(|word| matches!(*word, "artifact" | "artifacts"))
                    {
                        ironsmith_core::SacrificedObjectKind::Artifact
                    } else if words
                        .iter()
                        .any(|word| matches!(*word, "enchantment" | "enchantments"))
                    {
                        ironsmith_core::SacrificedObjectKind::Enchantment
                    } else {
                        ironsmith_core::SacrificedObjectKind::Permanent
                    };
                    with_for_each_surface(
                        Value::PendingEffectMetric {
                            source: EffectMetricSource::AffectedObjects,
                            metric: EffectMetric::Count,
                        }
                        .with_surface_hint(
                            ironsmith_core::ValueSurfaceHint::PermanentsSacrificedThisWay,
                        )
                        .with_surface_hint(
                            ironsmith_core::ValueSurfaceHint::SacrificedObject(kind),
                        ),
                    )
                }
            }
            DynamicThisWayMetric::Discarded => {
                let all_words = parser_token_word_refs(tokens);
                let count_words =
                    crate::word_primitives::parse_sequence_start(&all_words, &["for", "each"])
                        .map(|start| all_words[start..].to_vec())
                        .unwrap_or_else(|| {
                            let mut words = vec!["for", "each"];
                            words.extend(parser_token_word_refs(history_tokens));
                            words
                        });
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                    && matches!(value.unhinted(), Value::PendingPriorEffectMetric(_))
                {
                    value
                } else {
                    Value::PendingEffectMetric {
                        source: EffectMetricSource::Outcome,
                        metric: EffectMetric::Count,
                    }
                }
            }
            DynamicThisWayMetric::Exiled => {
                let action_idx =
                    crate::slice_primitives::select_position(history_tokens, |token| {
                        token.is_word("exiled")
                    });
                let mut filter = action_idx
                    .and_then(|idx| {
                        let subject = &history_tokens[..idx];
                        (!subject.is_empty())
                            .then(|| parse_object_filter(subject, false).ok())
                            .flatten()
                    })
                    .unwrap_or_default();
                filter.zone = Some(Zone::Exile);
                filter = filter.match_tagged(
                    crate::tag::CompilerReferenceTag::SourceExiled.as_str(),
                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                );
                Value::Count(filter)
            }
        },
        DynamicCostValueShape::RevealedPublic => Value::Count(ObjectFilter::tagged(
            crate::tag::CompilerReferenceTag::PublicRevealed.key(),
        )),
        DynamicCostValueShape::RevealedOther => {
            let words = parser_token_word_refs(tokens);
            let Some((value, used_words)) = parse_for_each_count_value_words(&words) else {
                return Ok(None);
            };
            if used_words != words.len() {
                return Ok(None);
            }
            value
        }
        DynamicCostValueShape::CounterReference(reference) => {
            let counter_type = reference.counter_type;
            let value = match reference.reference_kind {
                CounterReferenceKind::Source => match counter_type {
                    Some(counter_type) => Value::CountersOnSource(counter_type),
                    None => Value::CountersOn(Box::new(ChooseSpec::Source), None),
                },
                CounterReferenceKind::Tagged => Value::CountersOn(
                    Box::new(ChooseSpec::Tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                    )),
                    counter_type,
                ),
                CounterReferenceKind::Other => {
                    let words = parser_token_word_refs(reference.reference_tokens);
                    let Some(surface) = source_reference_surface_for_words(&words) else {
                        return Ok(None);
                    };
                    Value::CountersOn(
                        Box::new(source_choose_spec_for_surface(surface)),
                        counter_type,
                    )
                }
            };
            with_for_each_surface(value)
        }
        DynamicCostValueShape::UnsupportedThisWay => {
            return Err(CardTextError::ParseError(format!(
                "unsupported this-way dynamic value (clause: '{}')",
                parser_token_word_refs(tokens).join(" ")
            )));
        }
        DynamicCostValueShape::Other { filter_tokens } => {
            if let Some(value) =
                crate::grammar::shared_util::value_semantics::parse_turn_history_count_value(
                    filter_tokens,
                )
            {
                with_for_each_surface(value)
            } else if let Some(player) = parse_commander_cast_count_player(filter_tokens) {
                Value::CommanderCastCount(player)
            } else {
                let filter_words = parser_token_word_refs(filter_tokens);
                let mut count_words = vec!["for", "each"];
                count_words.extend(filter_words.iter().copied());
                if let Some((value, used)) = parse_for_each_count_value_words(&count_words)
                    && used == count_words.len()
                {
                    value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach)
                } else if let Ok(filter) = parse_object_filter(filter_tokens, false) {
                    with_for_each_surface(Value::Count(filter))
                } else {
                    return Ok(None);
                }
            }
        }
    };
    Ok(Some(value))
}

pub fn parse_add_mana_that_much_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    if keyword_static_lines::parse_that_much_value_marker_tokens(tokens) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

pub fn parse_players_skip_upkeep_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let tokens = super::super::grammar::effects::split_labeled_effect_prefix_lexed(&tokens)
        .unwrap_or(&tokens);
    if let Some(fact) = type_and_color_facts::parse_skip_your_upkeep_tokens(tokens) {
        let mut ability = StaticAbility::player_skips_upkeep(crate::target::PlayerFilter::You);
        match fact.tail {
            type_and_color_facts::SkipYourUpkeepTail::None => {}
            type_and_color_facts::SkipYourUpkeepTail::Condition(condition_tokens) => {
                let condition = parse_static_condition_clause(condition_tokens)?;
                ability = ability.with_condition(condition);
            }
            type_and_color_facts::SkipYourUpkeepTail::Unsupported => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported skip-upkeep tail (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
        }
        return Ok(Some(ability));
    }
    if is_players_skip_upkeep_line_lexed(tokens) {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

pub fn parse_players_skip_extra_turns_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let words = crate::lexer::parser_token_word_refs(&tokens);
    let player = if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "an", "opponent", "would", "begin", "an", "extra", "turn", "that", "player",
            "skips", "that", "turn", "instead",
        ],
    ) {
        crate::target::PlayerFilter::Opponent
    } else if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "a", "player", "would", "begin", "an", "extra", "turn", "that", "player",
            "skips", "that", "turn", "instead",
        ],
    ) {
        crate::target::PlayerFilter::Any
    } else if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "you", "would", "begin", "an", "extra", "turn", "skip", "that", "turn", "instead",
        ],
    ) {
        crate::target::PlayerFilter::You
    } else {
        return Ok(None);
    };
    Ok(Some(StaticAbility::players_skip_extra_turns(player)))
}

pub fn parse_skip_your_draw_step_static_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_skip_your_draw_step_line_lexed(tokens) {
        return Ok(Some(StaticAbility::player_skips_draw_step(
            crate::target::PlayerFilter::You,
        )));
    }
    Ok(None)
}

pub fn parse_legend_rule_doesnt_apply_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if let Some(scope) = keyword_static_lines::parse_legend_rule_doesnt_apply_tokens(tokens) {
        return Ok(Some(match scope {
            keyword_static_lines::LegendRuleScopeShape::Global => {
                StaticAbility::legend_rule_doesnt_apply()
            }
            keyword_static_lines::LegendRuleScopeShape::Controller => {
                StaticAbility::legend_rule_doesnt_apply_to_controller()
            }
            keyword_static_lines::LegendRuleScopeShape::ControllerCreatures => {
                StaticAbility::legend_rule_doesnt_apply_to_controller_matching(
                    ObjectFilter::creature(),
                )
            }
            keyword_static_lines::LegendRuleScopeShape::ControllerTokens => {
                StaticAbility::legend_rule_doesnt_apply_to_tokens_you_control()
            }
        }));
    }
    Ok(None)
}

pub fn parse_all_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_all_permanents_colorless_line_lexed(tokens) {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

pub fn parse_subject_are_card_types_in_addition_to_their_other_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if crate::grammar::abilities::parse_source_is_chosen_type_in_addition_line_lexed(tokens)
        .is_some()
    {
        // The source-scoped chosen-type production owns its complete line.
        return Ok(None);
    }
    let Some(fact) = type_and_color_facts::parse_subject_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    if fact.chosen_type {
        let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;
        for card_type in &filter.card_types {
            if *card_type == CardType::Land {
                return Ok(Some(vec![StaticAbility::add_chosen_basic_land_type(
                    filter,
                    render_token_slice(tokens),
                )]));
            }
        }
        return Ok(Some(vec![StaticAbility::add_chosen_creature_type(
            filter,
            render_token_slice(tokens),
        )]));
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }

        let Some(subtype) = parse_subtype_flexible(descriptor) else {
            return Ok(None);
        };
        crate::slice_primitives::push_unique(&mut subtypes, subtype);
    }
    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    let mut abilities = Vec::new();
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub fn parse_subject_is_card_types_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subject_card_type_identity_tokens(tokens) else {
        return Ok(None);
    };

    let mut card_types = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "a" | "an" | "and") {
            continue;
        }
        let Some(card_type) = parse_card_type(word) else {
            return Ok(None);
        };
        crate::slice_primitives::push_unique(&mut card_types, card_type);
    }
    if card_types.is_empty() {
        return Ok(None);
    }

    let subject = parse_anthem_subject(fact.subject_tokens)?;
    let mut filter = anthem_subject_filter(&subject);
    if matches!(subject, AnthemSubjectAst::Source) {
        let subject_words = parser_token_word_refs(fact.subject_tokens);
        if subject_words.len() > 1
            && let Some(surface) = source_reference_surface_for_words(&subject_words)
        {
            filter = filter.with_source_surface(surface);
        }
    }

    Ok(Some(StaticAbility::set_card_types(filter, card_types)))
}

pub fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_all_cards_spells_permanents_colorless_tokens(tokens) {
        let mut filter = ObjectFilter::default();
        filter.set_global_characteristic_domain_surface(Some(
            ironsmith_core::GlobalCharacteristicDomainSurface::CardsOutsideBattlefieldSpellsAndPermanents,
        ));
        return Ok(Some(StaticAbility::make_colorless(filter)));
    }
    Ok(None)
}

pub fn parse_all_cards_spells_permanents_add_chosen_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if type_and_color_facts::parse_all_cards_chosen_color_addition_tokens(tokens).is_some() {
        return Ok(Some(StaticAbility::add_chosen_color(
            ObjectFilter::default(),
            render_token_slice(tokens),
        )));
    }

    Ok(None)
}

pub fn parse_conjoined_subject_filter(
    tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let subject_tokens = trim_lexed_commas(tokens);
    if let Some(filter) =
        crate::activation_and_restrictions::parse_type_adjective_conjunction_filter(subject_tokens)?
    {
        return Ok(filter);
    }
    let subject_segments = split_lexed_slices_on_and(subject_tokens);
    if subject_segments.len() <= 1 {
        return parse_object_filter_lexed(subject_tokens, false);
    }

    let mut branches = Vec::with_capacity(subject_segments.len());
    for segment in subject_segments {
        let segment = trim_lexed_commas(segment);
        if segment.is_empty() {
            return parse_object_filter_lexed(subject_tokens, false);
        }
        branches.push(parse_object_filter_lexed(segment, false)?);
    }
    let mut filter = ObjectFilter::default();
    filter.any_of = branches;
    Ok(filter)
}

pub fn parse_all_are_pt_color_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_power_toughness_type_addition_tokens(tokens)
    else {
        return Ok(None);
    };
    let mut colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if is_article(descriptor) || matches!(descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(color) = parse_color(descriptor) {
            colors = colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in pt-color-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if colors.is_empty() && card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_conjoined_subject_filter(fact.subject_tokens)?;

    let mut abilities = Vec::new();
    if !colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), colors));
    }
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter.clone(), subtypes));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter,
        fact.power,
        fact.toughness,
    ));
    Ok(Some(abilities))
}

pub fn parse_all_are_color_and_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_color_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for token in fact.descriptor_tokens {
        let Some(descriptor) = token.as_word() else {
            continue;
        };
        if matches!(descriptor, "a" | "an" | "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_flexible(descriptor) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            render_token_slice(tokens)
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    let mut abilities = vec![StaticAbility::set_colors(filter.clone(), fact.color)];
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

pub fn parse_all_creatures_are_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subject_color_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    Ok(Some(StaticAbility::set_colors(filter, fact.color)))
}

pub fn parse_subjects_are_basic_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_subjects_are_basic_tokens(tokens) else {
        return Ok(None);
    };

    let subject_segments = split_lexed_slices_on_and(fact.subject_tokens);
    let filter = if subject_segments.len() > 1 {
        let mut branches = Vec::with_capacity(subject_segments.len());
        for segment in subject_segments {
            let segment = trim_lexed_commas(segment);
            if segment.is_empty() {
                return Ok(None);
            }
            branches.push(parse_object_filter_lexed(segment, false)?);
        }
        let mut filter = ObjectFilter::default();
        filter.any_of = branches;
        filter
    } else {
        parse_object_filter_lexed(fact.subject_tokens, false)?
    };

    Ok(Some(StaticAbility::add_supertypes(
        filter,
        vec![Supertype::Basic],
    )))
}

pub fn parse_nonbasic_lands_are_basic_land_type_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_basic_land_subtype_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    Ok(Some(StaticAbility::set_land_subtypes(
        filter,
        vec![fact.subtype],
    )))
}

pub fn parse_remove_snow_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_remove_snow_line_lexed(tokens) {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

pub fn parse_land_type_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_land_type_addition_tokens(tokens) else {
        return Ok(None);
    };
    match fact {
        type_and_color_facts::LandTypeAdditionFact::EveryBasic { subject_tokens } => {
            let filter = parse_object_filter_lexed(subject_tokens, false)?;
            Ok(Some(StaticAbility::add_subtypes(
                filter,
                vec![
                    Subtype::Plains,
                    Subtype::Island,
                    Subtype::Swamp,
                    Subtype::Mountain,
                    Subtype::Forest,
                ],
            )))
        }
        type_and_color_facts::LandTypeAdditionFact::One {
            subject_tokens,
            subtype,
        } => Ok(Some(StaticAbility::add_subtypes(
            parse_object_filter_lexed(subject_tokens, false)?,
            vec![subtype],
        ))),
    }
}

pub fn parse_lands_are_pt_creatures_still_lands_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(fact) = type_and_color_facts::parse_land_animation_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_object_filter_lexed(fact.subject_tokens, false)?;

    Ok(Some(vec![
        StaticAbility::add_card_types(filter.clone(), vec![CardType::Creature]),
        StaticAbility::set_base_power_toughness(filter, fact.power, fact.toughness),
    ]))
}

pub fn parse_static_base_power_toughness_value_tail(
    tail_tokens: &[OwnedLexToken],
) -> Option<(Value, Value)> {
    if !keyword_static_lines::parse_iterated_mana_value_base_pt_tail_tokens(tail_tokens) {
        return None;
    }
    let value = Value::ManaValueOf(Box::new(ChooseSpec::Iterated));
    Some((value.clone(), value))
}

pub fn parse_filter_is_pt_creature_in_addition_and_has_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let clause_words = LexedClause::new(tokens).word_refs();
    let Some(animation_verbs) = static_keyword_line_shapes::parse_animation_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = animation_verbs.be.token;
    let has_idx = animation_verbs.has.token;

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let contracted_source_subject = subject_start == be_idx
        && tokens.get(be_idx).is_some_and(|token| {
            token.is_word("it's") || token.is_word("it’s") || token.is_word("its")
        });
    let subject_tokens = if contracted_source_subject {
        tokens[be_idx..be_idx + 1].to_vec()
    } else {
        trim_commas(&tokens[subject_start..be_idx])
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let mut subject = if contracted_source_subject {
        AnthemSubjectAst::Source
    } else {
        match parse_anthem_subject(&subject_tokens) {
            Ok(subject) => subject,
            Err(_) => return Ok(None),
        }
    };
    // Animation-subject grammar intentionally consumes leading distributive
    // quantifiers before it builds the semantic filter. Preserve that authored
    // surface on this compound animation bundle so both lowered continuous
    // abilities render with the same subject.
    if let AnthemSubjectAst::Filter(filter) = &mut subject {
        filter.set_set_quantifier_surface(leading_set_quantifier_surface(&subject_tokens));
    }
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    // In a copular animation bundle, the follow-up grant may repeat the
    // animated subject as a pronoun: "... is a 0/0 creature in addition to
    // its other types and it has annihilator 2." The pronoun belongs to the
    // grant head, not to the preceding type-addition predicate.
    let before_has_end = if has_idx >= 2
        && tokens[has_idx - 2].is_word("and")
        && tokens[has_idx - 1].is_word("it")
    {
        has_idx - 2
    } else {
        has_idx
    };
    let before_has = trim_commas(&tokens[be_idx + 1..before_has_end]);
    if before_has.is_empty() {
        return Ok(None);
    }
    let before_has_clause = LexedClause::new(&before_has);
    let raw_before_has_words = before_has_clause.word_refs();
    let before_has_words = strip_leading_article_word_refs(&raw_before_has_words);
    let skipped_article_words = raw_before_has_words
        .len()
        .saturating_sub(before_has_words.len());
    let Some(creature_idx) =
        static_keyword_line_shapes::parse_animation_creature_word(before_has_words)
            .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    let granted_tail_tokens = &tokens[has_idx + 1..];
    let (base_power_toughness, subtype_start_word, granted_tail) = match before_has_words
        .first()
        .and_then(|word| parse_pt_modifier(word).ok())
    {
        Some((power, toughness)) => {
            if creature_idx == 0 {
                return Ok(None);
            }
            let parsed_tail = parse_heterogeneous_granted_tail(
                granted_tail_tokens,
                &clause_words,
                attached_subject,
            )?;
            // Quotation marks are presentation delimiters around a granted
            // triggered ability, not part of its trigger grammar. A quoted
            // trigger at the end of a heterogeneous grant list can make the
            // segmenter retain the delimiters while the equivalent unquoted
            // trigger parses normally. Retry the same generic tail after
            // removing only quote tokens when a quote directly introduces a
            // trigger; activated abilities keep their quote-aware comma
            // grouping.
            let parsed_tail =
                if parsed_tail.is_none() && quoted_trigger_intro_present(granted_tail_tokens) {
                    let unquoted = granted_tail_tokens
                        .iter()
                        .filter(|token| token.kind != TokenKind::Quote)
                        .cloned()
                        .collect::<Vec<_>>();
                    parse_heterogeneous_granted_tail(&unquoted, &clause_words, attached_subject)?
                } else {
                    parsed_tail
                };
            let Some(granted_tail) = parsed_tail else {
                return Ok(None);
            };
            (
                (Value::Fixed(power), Value::Fixed(toughness)),
                1usize,
                granted_tail,
            )
        }
        None => {
            let Some((power, toughness)) =
                parse_static_base_power_toughness_value_tail(&tokens[has_idx + 1..])
            else {
                return Ok(None);
            };
            ((power, toughness), 0usize, ParsedGrantedTailAst::default())
        }
    };
    let subtype_words = &before_has_words[subtype_start_word..creature_idx];
    let mut subtypes = Vec::new();
    for word in subtype_words {
        if is_article(word) {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }
    let tail_start_word = skipped_article_words + creature_idx + 1;
    let mut tail_end_word = skipped_article_words + before_has_words.len();
    let tail_ends_with_and = before_has_words[creature_idx + 1..]
        .last()
        .is_some_and(|word| *word == "and");
    if tail_ends_with_and {
        tail_end_word = tail_end_word.saturating_sub(1);
    }
    if type_and_color_facts::parse_other_type_addition_tail_tokens(
        before_has_clause
            .between_word_range(tail_start_word, tail_end_word)
            .map(|tail_clause| tail_clause.tokens())
            .unwrap_or_default(),
    )
    .is_none()
    {
        return Ok(None);
    }

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some(base_power_toughness),
            granted_tail,
        },
    )))
}

/// Parse a persistent copular animation with no following granted ability,
/// such as “it's a 1/1 Insect creature in addition to its other types.”
/// Compound variants with an `and has ...` tail remain owned by
/// `parse_filter_is_pt_creature_in_addition_and_has_line` above.
pub fn parse_filter_is_pt_creature_in_addition_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(be) = static_keyword_line_shapes::parse_animation_copula(tokens) else {
        return Ok(None);
    };
    let be_idx = be.token;
    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let contracted_source_subject = subject_start == be_idx
        && tokens.get(be_idx).is_some_and(|token| {
            token.is_word("it's") || token.is_word("it’s") || token.is_word("its")
        });
    let subject_tokens = if contracted_source_subject {
        tokens[be_idx..be_idx + 1].to_vec()
    } else {
        trim_commas(&tokens[subject_start..be_idx])
    };
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = if contracted_source_subject {
        AnthemSubjectAst::Source
    } else {
        match parse_anthem_subject(&subject_tokens) {
            Ok(subject) => subject,
            Err(_) => return Ok(None),
        }
    };

    let predicate_tokens = trim_edge_punctuation_tokens(&tokens[be_idx + 1..]);
    let predicate_clause = LexedClause::new(predicate_tokens);
    let raw_words = predicate_clause.word_refs();
    let words = strip_leading_article_word_refs(&raw_words);
    let skipped_article_words = raw_words.len().saturating_sub(words.len());
    let Some((power, toughness)) = words.first().and_then(|word| parse_pt_modifier(word).ok())
    else {
        return Ok(None);
    };
    let Some(creature_idx) = static_keyword_line_shapes::parse_animation_creature_word(words)
        .map(|boundary| boundary.word)
    else {
        return Ok(None);
    };
    if creature_idx == 0 {
        return Ok(None);
    }

    let mut subtypes = Vec::new();
    for word in &words[1..creature_idx] {
        if is_article(word) {
            continue;
        }
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }

    let tail_start_word = skipped_article_words + creature_idx + 1;
    let tail_end_word = skipped_article_words + words.len();
    let Some(tail_clause) = predicate_clause.between_word_range(tail_start_word, tail_end_word)
    else {
        return Ok(None);
    };
    if type_and_color_facts::parse_other_type_addition_tail_tokens(tail_clause.tokens()).is_none() {
        return Ok(None);
    }

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::Add,
            base_power_toughness: Some((Value::Fixed(power), Value::Fixed(toughness))),
            granted_tail: ParsedGrantedTailAst::default(),
        },
    )))
}

fn quoted_trigger_intro_present(tokens: &[OwnedLexToken]) -> bool {
    tokens.iter().enumerate().any(|(idx, token)| {
        if token.kind != TokenKind::Quote {
            return false;
        }
        matches!(
            tokens.get(idx + 1).and_then(OwnedLexToken::as_word),
            Some("when" | "whenever")
        ) || (tokens.get(idx + 1).and_then(OwnedLexToken::as_word) == Some("at")
            && tokens.get(idx + 2).and_then(OwnedLexToken::as_word) == Some("the"))
    })
}

pub fn parse_subject_is_subtype_with_base_pt_and_granted_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens =
        if let Some((label_tokens, body_tokens)) = split_em_dash_label_prefix_tokens(tokens) {
            if document_grammar::parse_preserved_keyword_label_tokens(label_tokens).is_some() {
                tokens
            } else {
                body_tokens
            }
        } else {
            tokens
        };
    let Some(grant_verbs) = static_keyword_line_shapes::parse_subtype_grant_verbs(tokens) else {
        return Ok(None);
    };
    let be_idx = grant_verbs.be.token;
    let with_idx = grant_verbs.with.token;

    let (_condition, subject_start) = match parse_anthem_prefix_condition(tokens, be_idx) {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..be_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let attached_subject = LexedClause::new(&subject_tokens)
        .words()
        .first()
        .is_some_and(|word| matches!(word, "enchanted" | "equipped"));

    let type_tokens = trim_commas(&tokens[be_idx + 1..with_idx]);
    if type_tokens.is_empty() {
        return Ok(None);
    }
    let type_words = LexedClause::new(&type_tokens).word_refs();
    let type_words = strip_leading_article_word_refs(&type_words);
    if type_words.is_empty() {
        return Ok(None);
    }
    let mut subtypes = Vec::new();
    for word in type_words {
        let Some(subtype) = parse_subtype_word(word) else {
            return Ok(None);
        };
        subtypes.push(subtype);
    }

    let mut after_with = trim_commas(&tokens[with_idx + 1..]).to_vec();
    if after_with.is_empty() {
        return Ok(None);
    }

    if let Some(loses) = type_and_color_facts::find_loses_other_creature_types_tokens(&after_with) {
        after_with.truncate(loses.marker_token);
    }

    let after_with = trim_edge_punctuation_tokens(&after_with);
    let Some(base) = type_and_color_facts::parse_base_power_toughness_grant_tokens(after_with)
    else {
        return Ok(None);
    };
    let after_with_words = parser_token_word_refs(after_with);
    let Some(granted_tail) =
        parse_heterogeneous_granted_tail(base.ability_tokens, &after_with_words, attached_subject)?
    else {
        return Ok(None);
    };

    Ok(Some(lower_static_animation_bundle(
        StaticAnimationBundleAst {
            subject,
            condition: _condition,
            ensure_creature_type: true,
            subtypes,
            subtype_mode: AnimationSubtypeMode::ReplaceCreatureTypes,
            base_power_toughness: Some((Value::Fixed(base.power), Value::Fixed(base.toughness))),
            granted_tail,
        },
    )))
}

pub fn parse_creatures_cant_block_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    if is_creatures_cant_block_line_lexed(tokens) {
        return Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter: ObjectFilter::creature(),
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::cant_block())),
            condition: None,
        }));
    }
    Ok(None)
}

pub fn parse_prevent_all_damage_dealt_to_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_dealt_to_creatures_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_damage_dealt_to_creatures()));
    }
    Ok(None)
}

pub fn parse_prevent_damage_to_other_creature_you_control_put_counters_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_damage_to_other_creature_you_control_put_counters_line_lexed(tokens) {
        return Ok(None);
    }

    Ok(Some(
        StaticAbility::prevent_damage_to_other_creature_you_control_put_counters_instead(
            crate::object::CounterType::PlusOnePlusOne,
            display_text_for_tokens(tokens, true),
        ),
    ))
}

pub fn parse_damage_source_filter_words(words: &[&str]) -> Option<ObjectFilter> {
    let mut words = strip_leading_article_word_refs(words).to_vec();
    if word_slice_last_is_any(&words, &["source", "sources"]) {
        words.pop();
    }
    if words.is_empty() {
        return Some(ObjectFilter::default());
    }

    let mut filter = ObjectFilter::default();
    let mut colors: Option<ColorSet> = None;
    for word in words {
        if matches!(word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = Some(colors.unwrap_or_default().union(color));
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            filter.card_types.push(card_type);
            continue;
        }
        return None;
    }
    if let Some(colors) = colors {
        filter.colors = Some(colors);
    }
    Some(filter)
}

pub fn parse_damage_source_filter_tokens(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = LexedClause::new(tokens).word_refs();
    parse_damage_source_filter_words(&words)
}

pub fn parse_prevent_damage_to_you_from_source_filter_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_prevent_damage_to_you_tokens(tokens) else {
        return Ok(None);
    };
    let Some(source_filter) = parse_damage_source_filter_tokens(spec.source_tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);

    Ok(Some(
        StaticAbility::prevent_damage_to_you_from_source_filter(
            spec.amount,
            source_filter,
            display,
        ),
    ))
}

pub fn parse_replace_damage_with_counters_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !keyword_static_lines::parse_noncombat_damage_minus_counter_replacement_tokens(tokens) {
        return Ok(None);
    }

    Ok(Some(StaticAbility::replace_damage_with_counters_instead(
        CounterType::MinusOneMinusOne,
        ObjectFilter::default().controlled_by(PlayerFilter::You),
        ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
        Some(false),
        display_text_for_tokens(tokens, true),
    )))
}

pub fn parse_double_counters_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_counter_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::CounterReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_counters_replacement(
                ObjectFilter::permanent().controlled_by(PlayerFilter::You),
                None,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::EnergyYouGet => {
            StaticAbility::double_player_counters_replacement(
                PlayerFilter::You,
                Some(CounterType::Energy),
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::PlusOneAdd {
            filter_tokens,
            additional,
        } => StaticAbility::add_counters_placement_replacement(
            parse_object_filter_lexed(filter_tokens, false)?,
            Some(CounterType::PlusOnePlusOne),
            additional,
            display_text_for_tokens(tokens, true),
        ),
        keyword_static_lines::CounterReplacementShape::PlusOneDouble { filter_tokens } => {
            StaticAbility::double_counters_replacement(
                parse_object_filter_lexed(filter_tokens, false)?,
                Some(CounterType::PlusOnePlusOne),
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::CounterReplacementShape::PlayerCounterPerTurnLimit {
            counter_type,
            maximum,
        } => StaticAbility::player_counter_per_turn_limit_replacement(
            PlayerFilter::You,
            counter_type,
            maximum,
            display_text_for_tokens(tokens, true),
        ),
    }))
}

pub fn parse_double_token_creation_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_token_creation_replacement_tokens(tokens) else {
        return Ok(None);
    };
    Ok(Some(match shape {
        keyword_static_lines::TokenCreationReplacementShape::GenericUnderYourControl => {
            StaticAbility::double_token_creation_replacement(
                PlayerFilter::You,
                display_text_for_tokens(tokens, true),
            )
        }
        keyword_static_lines::TokenCreationReplacementShape::AddTreasure { descriptor_tokens } => {
            let mut token_filter = ObjectFilter::default().token();
            for word in parser_token_word_refs(descriptor_tokens) {
                if let Some(card_type) = parse_card_type(word) {
                    token_filter = token_filter.with_type(card_type);
                } else if let Some(subtype) = parse_subtype_flexible(word) {
                    token_filter = token_filter.with_subtype(subtype);
                } else {
                    return Ok(None);
                }
            }
            StaticAbility::add_token_creation_replacement(
                PlayerFilter::You,
                token_filter,
                ironsmith_core::AdditionalTokenKind::Treasure,
                1,
                display_text_for_tokens(tokens, true),
            )
        }
    }))
}

pub fn parse_prevent_all_combat_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_combat_damage_to_source_line_lexed(tokens) {
        return Ok(Some(StaticAbility::prevent_all_combat_damage_to_self()));
    }

    Ok(None)
}

pub fn parse_prevent_all_combat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if !is_prevent_all_combat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }
    let Some(prevention) =
        static_keyword_replacement_shapes::parse_combat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "prevent-all combat damage static line missing target filter (clause: '{}')",
            render_token_slice(tokens)
        )));
    }
    if let Some(by_idx) =
        crate::slice_primitives::select_position(target_tokens.as_slice(), |token| {
            token.is_word("by")
        })
    {
        let source_tokens = trim_commas(&target_tokens[by_idx + 1..]);
        let word_positions = source_tokens
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| token.as_word().map(|word| (idx, word)))
            .collect::<Vec<_>>();
        let (source_end, source_relation) = if word_positions.len() >= 2
            && word_positions[word_positions.len() - 2].1 == "blocking"
            && word_positions[word_positions.len() - 1].1 == "it"
        {
            (
                word_positions[word_positions.len() - 2].0,
                ironsmith_core::StaticDamageSourceRelation::BlockingStaticSource,
            )
        } else {
            (
                source_tokens.len(),
                ironsmith_core::StaticDamageSourceRelation::Any,
            )
        };
        let source_filter_tokens = trim_commas(&source_tokens[..source_end]);
        if source_filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "prevent-all combat damage static line missing source filter (clause: '{}')",
                render_token_slice(tokens)
            )));
        }
        let source_filter = parse_object_filter_lexed(&source_filter_tokens, false)?;
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_from_sources_matching(
                ironsmith_core::PreventAllDamageToSelfFromSourcesMatchingSpec {
                    source_filter,
                    combat_only: true,
                    source_relation,
                    display: display_text_for_tokens(tokens, true),
                },
            ),
        ));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_combat_damage_to_permanents_matching(filter),
    ))
}

pub fn parse_during_your_turn_prevent_all_damage_to_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_during_your_turn_prevent_all_damage_to_source_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self().with_condition(
                crate::ConditionExpr::ActivationTiming(
                    crate::ability::ActivationTiming::DuringYourTurn,
                ),
            ),
        ));
    }

    Ok(None)
}

pub fn parse_prevent_all_noncombat_damage_to_other_creatures_you_control_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_noncombat_damage_to_other_creatures_you_control(),
        ));
    }

    Ok(None)
}

pub fn parse_prevent_all_noncombat_damage_to_matching_permanents_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_noncombat_damage_to_other_creatures_you_control_line_lexed(tokens) {
        // The other-creatures-you-control production owns its complete line.
        return Ok(None);
    }
    if !is_prevent_all_noncombat_damage_to_matching_permanents_line_lexed(tokens) {
        return Ok(None);
    }

    let Some(prevention) =
        static_keyword_replacement_shapes::parse_noncombat_prevention_prefix(tokens)
    else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&tokens[prevention.end..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all noncombat damage target filter: {}",
            render_token_slice(tokens)
        )));
    }
    let filter = parse_object_filter_lexed(&target_tokens, false)?;
    Ok(Some(
        StaticAbility::prevent_all_noncombat_damage_to_permanents_matching(filter),
    ))
}

pub fn parse_prevent_all_damage_to_source_by_creatures_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_prevent_all_damage_to_source_by_creatures_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::prevent_all_damage_to_self_by_creatures(),
        ));
    }
    Ok(None)
}

pub fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_may_choose_not_untap_tokens(tokens) else {
        return Ok(None);
    };
    let subject_words = parser_token_word_refs(fact.subject_tokens);
    if !fact.simple_source_subject && !is_source_reference_words(&subject_words) {
        return Ok(None);
    }

    let subject = render_token_slice(fact.subject_tokens);
    let subject = source_reference_surface_for_words(&subject_words)
        .map(|surface| surface.display_text())
        .unwrap_or(subject);
    Ok(Some(
        StaticAbility::may_choose_not_to_untap_during_untap_step(subject),
    ))
}

pub fn parse_untap_during_each_other_players_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = split_untap_each_other_players_untap_step_line_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(spec.subject_tokens);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in other-players untap ability (clause: '{}')",
            render_token_slice(tokens)
        )));
    }

    let filter = parse_object_filter(&subject_tokens, false)?;
    let subject_text = render_token_slice(&subject_tokens);
    Ok(Some(
        StaticAbility::untap_during_each_other_players_untap_step(
            filter,
            format!("Untap all {subject_text} during each other player's untap step"),
        ),
    ))
}

pub fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    match parse_doesnt_untap_during_untap_step_spec_lexed(tokens) {
        Some(DoesntUntapDuringUntapStepSpec::Source { tail_tokens }) => {
            let clause_display = render_token_slice(tokens);
            let tail_tokens = trim_commas(tail_tokens);
            if tail_tokens.is_empty() {
                return Ok(Some(
                    StaticAbilityAst::Static(StaticAbility::doesnt_untap()),
                ));
            }
            if tail_tokens.first().is_some_and(|token| token.is_word("if")) {
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after untap-step if-clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition = parse_static_condition_clause(&condition_tokens)?;
                return Ok(Some(StaticAbilityAst::ConditionalStaticAbility {
                    ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                    condition,
                }));
            }

            Err(CardTextError::ParseError(format!(
                "unsupported trailing untap-step clause (clause: '{}')",
                clause_display
            )))
        }
        Some(DoesntUntapDuringUntapStepSpec::Attached {
            subject_tokens,
            tail_tokens,
        }) => {
            let subject = render_token_slice(subject_tokens);
            let text = format!("{subject} doesn't untap during its controller's untap step");
            let condition = if tail_tokens.is_empty() {
                None
            } else {
                let clause_display = render_token_slice(tokens);
                if tail_tokens
                    .first()
                    .is_none_or(|token| token.as_word() != Some("unless"))
                {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing attached untap-step clause (clause: '{}')",
                        clause_display
                    )));
                }
                let condition_tokens = trim_commas(&tail_tokens[1..]);
                if condition_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing condition after attached untap-step unless-clause (clause: '{}')",
                        clause_display
                    )));
                }
                Some(crate::ConditionExpr::Not(Box::new(
                    parse_static_condition_clause(&condition_tokens)?,
                )))
            };
            Ok(Some(StaticAbilityAst::AttachedStaticAbilityGrant {
                ability: Box::new(StaticAbilityAst::Static(StaticAbility::doesnt_untap())),
                display: text,
                condition,
            }))
        }
        None => Ok(None),
    }
}

pub fn parse_flying_restriction_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(match parse_flying_block_restriction_line_lexed(tokens) {
        Some(FlyingBlockRestrictionKind::FlyingOnly) => {
            Some(StaticAbility::flying_only_restriction())
        }
        Some(FlyingBlockRestrictionKind::FlyingOrReach) => {
            Some(StaticAbility::flying_restriction())
        }
        None => None,
    })
}

pub fn parse_can_block_only_flying_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_block_only_flying_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

pub fn parse_can_block_subtype_as_though_reach_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(parse_can_block_subtype_as_though_reach_line_lexed(tokens)
        .map(StaticAbility::can_block_subtype_as_though_reach))
}

pub fn parse_assign_damage_as_unblocked_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_may_assign_damage_as_unblocked_line_lexed(tokens) {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
}

pub fn parse_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::mana_value_as_generic_from_hand(),
            filter,
            Zone::Hand,
        ),
    )))
}

pub fn parse_life_mana_value_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_life_mana_value_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    let usage_limit = match spec.usage_limit {
        keyword_static_lines::LifeManaValueGrantUsageLimit::OnceDuringEachOfYourTurns => {
            crate::grant::GrantUsageLimit::OnceDuringEachOfYourTurns
        }
    };
    Ok(Some(StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::life_equal_mana_value_from_hand(Some(usage_limit)),
            filter,
            Zone::Hand,
        ),
    )))
}

pub fn parse_fixed_mana_cost_instead_of_mana_cost_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_fixed_mana_cost_grant_tokens(tokens) else {
        return Ok(None);
    };
    let filter = parse_spell_filter_with_grammar_entrypoint_lexed(spec.subject_tokens);
    Ok(Some(StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::cast_from_hand_for_alternative_mana_cost_matching(
            filter,
            spec.mana_cost,
        ),
    )))
}

pub fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec == crate::model::CompilerGrantSpecCore::flash_to_noncreature_spells() => {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub fn static_grant_beneficiary(player: crate::cards::builders::PlayerAst) -> Option<PlayerFilter> {
    match player {
        crate::cards::builders::PlayerAst::You | crate::cards::builders::PlayerAst::Implicit => {
            Some(PlayerFilter::You)
        }
        crate::cards::builders::PlayerAst::Any => Some(PlayerFilter::Any),
        _ => None,
    }
}

pub fn parse_you_may_cast_exile_counter_cards_with_mana_permission_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_counter_permission_tokens(tokens) else {
        return Ok(None);
    };
    let is_play_lands_and_cast_noncreature_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayLandsAndCastNoncreatureCardsExiledBySource
    );
    let is_play_owned_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayLandsAndCastSpellsOwnedInExile
    );
    let is_play_not_owned_family = matches!(
        spec.family,
        keyword_static_lines::ExileCounterPermissionFamily::PlayCardsNotOwnedInExile
    );
    let owner = match spec.owner {
        keyword_static_lines::ExileCounterPermissionOwner::Any => None,
        keyword_static_lines::ExileCounterPermissionOwner::Opponent => Some(PlayerFilter::Opponent),
        keyword_static_lines::ExileCounterPermissionOwner::You => Some(PlayerFilter::You),
        keyword_static_lines::ExileCounterPermissionOwner::NotYou => Some(PlayerFilter::NotYou),
    };

    let uses_snow_sources = matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::SnowSources
    );

    let mut base_filter = ObjectFilter {
        zone: Some(Zone::Exile),
        owner,
        with_counter: Some(crate::filter::CounterConstraint::Typed(spec.counter_type)),
        ..ObjectFilter::default()
    };
    if is_play_lands_and_cast_noncreature_family {
        base_filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::SourceExiled.key(),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
    }

    let mut filter = if is_play_lands_and_cast_noncreature_family {
        ObjectFilter {
            any_of: vec![
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
                ObjectFilter {
                    excluded_card_types: vec![CardType::Creature, CardType::Land],
                    ..base_filter.clone()
                },
            ],
            ..ObjectFilter::default()
        }
    } else if is_play_owned_family {
        ObjectFilter {
            any_of: vec![
                ObjectFilter {
                    card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
                ObjectFilter {
                    excluded_card_types: vec![CardType::Land],
                    ..base_filter.clone()
                },
            ],
            ..ObjectFilter::default()
        }
    } else if is_play_not_owned_family {
        base_filter.clone()
    } else {
        ObjectFilter {
            excluded_card_types: vec![CardType::Land],
            ..base_filter
        }
    };
    filter.has_mana_cost = false;

    let mut grant = StaticAbility::grants(
        crate::model::CompilerGrantSpecCore::new(
            crate::model::CompilerGrantableCore::play_from(),
            filter.clone(),
            Zone::Exile,
        )
        .with_beneficiary(PlayerFilter::You),
    );
    if spec.during_your_turn {
        grant = grant.with_condition(crate::ConditionExpr::ActivationTiming(
            crate::ability::ActivationTiming::DuringYourTurn,
        ));
    }
    if matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::None
    ) {
        return Ok(Some(vec![grant]));
    }
    let permission = if uses_snow_sources {
        crate::effect::ManaSpendPermission::any_color_from_sources_for_casting_matching(
            PlayerFilter::You,
            filter,
            ObjectFilter::default().with_supertype(Supertype::Snow),
        )
    } else if matches!(
        spec.mana_permission,
        keyword_static_lines::ExileCounterManaPermission::AnyTypeCanBeSpent
    ) {
        crate::effect::ManaSpendPermission::any_type_for_casting_matching(PlayerFilter::You, filter)
    } else {
        crate::effect::ManaSpendPermission::any_color_for_casting_matching(
            PlayerFilter::You,
            filter,
        )
    };
    let mana_permission =
        StaticAbility::mana_spend_permission(permission, render_token_slice(tokens));

    Ok(Some(vec![grant, mana_permission]))
}

pub fn parse_surveilled_graveyard_play_life_cost_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if !late_static_facts::is_surveilled_graveyard_play_life_cost(tokens) {
        return Ok(None);
    }

    let base_filter = ObjectFilter {
        zone: Some(Zone::Graveyard),
        owner: Some(PlayerFilter::You),
        surveilled_this_turn: true,
        ..ObjectFilter::default()
    };
    let mut spell_filter = base_filter.clone();
    spell_filter.excluded_card_types.push(CardType::Land);

    Ok(Some(vec![
        StaticAbility::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::play_from(),
                base_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
        StaticAbility::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::life_equal_mana_value_from_zone(
                    Zone::Graveyard,
                    None,
                ),
                spell_filter,
                Zone::Graveyard,
            )
            .with_beneficiary(PlayerFilter::You),
        ),
    ]))
}

pub fn parse_you_may_static_grant_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if late_static_facts::is_source_linked_exile_cast_with_any_mana(tokens) {
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.owner = Some(PlayerFilter::NotYou);
        filter
            .tagged_constraints
            .push(crate::target::TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::SourceExiled.key(),
                relation: crate::target::TaggedOpbjectRelation::IsTaggedObject,
            });
        let grant = StaticAbility::grants(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::play_from(),
                filter.clone(),
                Zone::Exile,
            )
            .with_beneficiary(PlayerFilter::Any),
        );
        let mana_permission = StaticAbility::mana_spend_permission(
            crate::effect::ManaSpendPermission::any_color_for_casting_matching(
                PlayerFilter::Any,
                filter,
            ),
            "Mana of any type can be spent to cast it",
        );
        return Ok(Some(vec![grant, mana_permission]));
    }

    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) => {
            let singular_spell = late_static_facts::contains_singular_cast_spell(tokens);
            if singular_spell
                && spec.zone == Zone::Hand
                && matches!(
                    &spec.grantable,
                    crate::model::CompilerGrantableCore::AlternativeCast(method)
                        if method.cast_from_zone() == Zone::Hand
                            && method.mana_cost().is_none()
                            && method.non_mana_costs().is_empty()
                )
            {
                return Ok(None);
            }
            Ok(static_grant_beneficiary(player)
                .map(|beneficiary| vec![StaticAbility::grants(spec.with_beneficiary(beneficiary))]))
        }
        _ => Ok(None),
    }
}

pub fn parse_as_you_cascade_land_drop_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if keyword_static_lines::parse_cascade_land_drop_tokens(tokens) {
        return Ok(Some(StaticAbility::cascade_land_drop()));
    }
    Ok(None)
}

pub fn parse_play_from_permission_with_haste_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(permission_sentence) =
        late_static_facts::parse_play_permission_with_haste_followup(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
        ) && spec.filter.card_types.len() == 1
            && spec.filter.card_types.first() == Some(&CardType::Creature) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::haste()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub fn parse_play_from_permission_with_enter_counter_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(parsed) = keyword_static_lines::parse_play_permission_enter_counter_tokens(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(parsed.permission_tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
        ) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                let count = if parsed.additional {
                    Value::Fixed(1)
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
                } else {
                    Value::Fixed(1)
                };
                let mut spec = spec.with_beneficiary(beneficiary).with_cast_this_way_grant(
                    StaticAbility::enters_with_counters_value(parsed.counter_type, count),
                );
                if let Some(filter) = parsed.cast_this_way_filter {
                    spec = spec.with_cast_this_way_filter(filter);
                }
                StaticAbility::grants(spec)
            }))
        }
        _ => Ok(None),
    }
}

pub fn parse_play_from_permission_with_enter_tapped_this_way_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(permission_sentence) =
        late_static_facts::parse_play_permission_with_enter_tapped_followup(tokens)
    else {
        return Ok(None);
    };

    match parse_permission_clause_spec(permission_sentence)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if matches!(
            spec.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
                | crate::model::CompilerGrantableCore::AlternativeCast(_)
                | crate::model::CompilerGrantableCore::DerivedAlternativeCast(_)
        ) =>
        {
            Ok(static_grant_beneficiary(player).map(|beneficiary| {
                StaticAbility::grants(
                    spec.with_beneficiary(beneficiary)
                        .with_cast_this_way_grant(StaticAbility::enters_tapped_ability()),
                )
            }))
        }
        _ => Ok(None),
    }
}

pub fn parse_you_may_look_top_card_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_top_card_any_time_line_lexed(tokens) {
        return Ok(Some(StaticAbility::look_at_top_card_of_library()));
    }
    Ok(None)
}

pub fn parse_you_may_look_face_down_creatures_you_dont_control_any_time_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_you_may_look_face_down_creatures_you_dont_control_any_time_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::look_at_face_down_creatures_you_dont_control(),
        ));
    }
    Ok(None)
}

pub fn parse_players_play_top_card_libraries_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_players_play_top_card_libraries_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_top_cards_of_libraries(),
        ));
    }
    Ok(None)
}

pub fn parse_play_top_card_your_library_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_top_card_your_library_revealed_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::all_players_look_at_your_top_library_card(),
        ));
    }
    Ok(None)
}

pub fn parse_your_opponents_play_with_hands_revealed_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_your_opponents_play_with_hands_revealed_line_lexed(tokens) {
        return Ok(Some(StaticAbility::opponents_play_with_hands_revealed()));
    }
    Ok(None)
}

pub fn parse_control_opponents_while_searching_libraries_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_control_opponents_while_searching(tokens) {
        return Ok(Some(
            StaticAbility::control_opponents_while_searching_libraries(),
        ));
    }
    Ok(None)
}

pub fn parse_opponent_search_exile_found_cards_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_opponent_search_exile_found_cards(tokens) {
        return Ok(Some(StaticAbility::opponent_search_exile_found_cards()));
    }
    Ok(None)
}

pub fn parse_cast_this_card_from_library_while_searching_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::is_cast_this_card_from_library_while_searching(tokens) {
        return Ok(Some(
            StaticAbility::cast_this_card_from_library_while_searching(),
        ));
    }
    Ok(None)
}

pub fn parse_cast_this_spell_as_though_it_had_flash_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_cast_this_spell_as_though_it_had_flash_line_lexed(tokens) {
        return Ok(Some(StaticAbility::flash()));
    }
    Ok(None)
}

pub fn parse_attacks_each_combat_if_able_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(fact) = late_static_facts::parse_attack_each_combat_if_able_tokens(tokens) else {
        return Ok(None);
    };
    if matches!(
        fact,
        late_static_facts::AttackEachCombatFact::AttachedController
    ) {
        return Ok(Some(StaticAbilityAst::Static(
            StaticAbility::all_creatures_attack_attached_controller_each_combat_if_able(),
        )));
    }
    let late_static_facts::AttackEachCombatFact::Subject(subject_tokens) = fact else {
        unreachable!("attached-controller fact returned above")
    };
    let subject_tokens = trim_commas(subject_tokens);
    if subject_tokens.is_empty() {
        return Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;
    match subject {
        AnthemSubjectAst::Source => {
            Ok(Some(StaticAbilityAst::Static(StaticAbility::must_attack())))
        }
        AnthemSubjectAst::Filter(filter) => Ok(Some(StaticAbilityAst::GrantStaticAbility {
            filter,
            ability: Box::new(StaticAbilityAst::Static(StaticAbility::must_attack())),
            condition: None,
        })),
    }
}

pub fn parse_additional_land_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let Some(count) = late_static_facts::parse_additional_land_play_count(tokens) else {
        return Ok(None);
    };

    Ok(Some(vec![StaticAbility::additional_land_plays(count)]))
}

pub fn parse_play_lands_from_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_play_lands_from_graveyard_line_lexed(tokens) {
        let spec = crate::model::CompilerGrantSpecCore::play_lands_from_graveyard();
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

pub fn parse_graveyard_cards_have_retrace_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(fact) = late_static_facts::parse_retrace_grant_tokens(tokens) else {
        return Ok(None);
    };
    let mut filter = ObjectFilter {
        card_types: fact.card_types,
        owner: Some(PlayerFilter::You),
        ..ObjectFilter::default()
    };
    filter.zone = Some(Zone::Graveyard);
    let spec = crate::model::CompilerGrantSpecCore::new(
        crate::model::CompilerGrantableCore::retrace_from_cards_mana_cost(),
        filter,
        Zone::Graveyard,
    );
    Ok(Some(StaticAbility::grants(spec)))
}

pub fn parse_cast_spells_from_hand_without_paying_mana_costs_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if late_static_facts::contains_singular_cast_spell(tokens) {
        return Ok(None);
    }
    match parse_permission_clause_spec(tokens)? {
        Some(crate::cards::builders::PermissionClauseSpec::GrantBySpec {
            player: crate::cards::builders::PlayerAst::You,
            spec,
            lifetime: crate::cards::builders::PermissionLifetime::Static,
        }) if spec.zone == Zone::Hand
            && matches!(
                &spec.grantable,
                crate::model::CompilerGrantableCore::AlternativeCast(method)
                    if method.cast_from_zone() == Zone::Hand
                        && method.mana_cost().is_none()
                        && method.non_mana_costs().is_empty()
            ) =>
        {
            Ok(Some(StaticAbility::grants(spec)))
        }
        _ => Ok(None),
    }
}

pub fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power_str = strip_leading_plus_char(power_raw);
    let toughness_str = strip_leading_plus_char(toughness_raw);
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

pub fn parse_signed_pt_component(raw: &str) -> Result<Value, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CardTextError::ParseError(
            "missing power/toughness component".to_string(),
        ));
    }

    let (sign, value_text) = split_signed_pt_component(trimmed);

    if pt_component_is_x(value_text) {
        return Ok(match sign {
            1 => Value::X,
            -1 => Value::XTimes(-1),
            _ => Value::XTimes(sign),
        });
    }

    let parsed = value_text
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power/toughness component".to_string()))?;
    Ok(Value::Fixed(parsed * sign))
}

pub fn parse_pt_modifier_values(raw: &str) -> Result<(Value, Value), CardTextError> {
    let (power_raw, toughness_raw) = split_pt_modifier_components(raw)?;
    let power = parse_signed_pt_component(power_raw)?;
    let toughness = parse_signed_pt_component(toughness_raw)?;
    Ok((power, toughness))
}

pub fn split_pt_modifier_components(raw: &str) -> Result<(&str, &str), CardTextError> {
    static_keyword_shapes::parse_pt_components(raw)
        .map(|components| (components.power, components.toughness))
        .ok_or_else(|| CardTextError::ParseError("missing power/toughness modifier".to_string()))
}

pub fn strip_leading_plus_char(raw: &str) -> &str {
    let trimmed = raw.trim();
    let mut chars = trimmed.chars();
    if chars.next().is_some_and(|ch| ch == '+') {
        chars.as_str()
    } else {
        trimmed
    }
}

pub fn split_signed_pt_component(trimmed: &str) -> (i32, &str) {
    let mut chars = trimmed.chars();
    match chars.next() {
        Some('+') => (1, chars.as_str()),
        Some('-' | '−') => (-1, chars.as_str()),
        _ => (1, trimmed),
    }
}

pub fn pt_component_is_x(text: &str) -> bool {
    let mut chars = text.chars();
    chars.next().is_some_and(|ch| matches!(ch, 'x' | 'X')) && chars.next().is_none()
}

pub fn parse_no_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_no_maximum_hand_size_line_lexed(tokens) {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

pub fn parse_can_be_your_commander_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_can_be_your_commander_line_lexed(tokens) {
        return Ok(Some(StaticAbility::can_be_commander()));
    }
    Ok(None)
}

pub fn parse_reduced_maximum_hand_size_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_hand_size_line_tokens(tokens) else {
        return Ok(None);
    };
    let player = match spec.player {
        keyword_static_lines::HandSizePlayerKind::You => PlayerFilter::You,
        keyword_static_lines::HandSizePlayerKind::Opponent => PlayerFilter::Opponent,
        keyword_static_lines::HandSizePlayerKind::Any => PlayerFilter::Any,
    };
    let min_card_types_condition = if let Some(condition_tokens) = spec.condition_tokens {
        let Some((metric, threshold)) =
            parse_graveyard_metric_threshold_condition(condition_tokens)?
        else {
            return Ok(None);
        };
        if metric != crate::static_abilities::GraveyardCountMetric::CardTypes {
            return Ok(None);
        }
        threshold
    } else {
        0
    };
    Ok(Some(match spec.operation {
        keyword_static_lines::HandSizeOperation::Reduce(amount) => {
            StaticAbility::reduce_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Increase(amount) => {
            StaticAbility::increase_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::Set(amount) => {
            StaticAbility::set_maximum_hand_size(player, amount)
        }
        keyword_static_lines::HandSizeOperation::SevenMinusGraveyardCardTypes => {
            StaticAbility::max_hand_size_seven_minus_your_graveyard_card_types(
                player,
                min_card_types_condition,
            )
        }
    }))
}

pub fn parse_effect_discard_to_library_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_effect_discard_to_library_replacement_line_lexed(tokens) {
        return Ok(Some(StaticAbility::effect_discard_to_library_replacement()));
    }

    if is_opponent_effect_discard_this_to_battlefield_replacement_line_lexed(tokens) {
        return Ok(Some(
            StaticAbility::opponent_effect_discard_this_to_battlefield_replacement(),
        ));
    }

    Ok(None)
}

pub fn parse_draw_replace_exile_top_face_down_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replace_exile_top_face_down_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_exile_top_face_down()));
    }

    Ok(None)
}

pub fn parse_draw_replacement_exile_top_and_play_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(count) = late_static_facts::parse_draw_replacement_exile_top_and_play_count(tokens)
    else {
        return Ok(None);
    };

    Ok(Some(StaticAbility::draw_replacement_exile_top_and_play(
        count,
    )))
}

pub fn parse_draw_replacement_reveal_top_matching_to_hand_rest_bottom_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) =
        static_keyword_replacement_shapes::parse_draw_reveal_matching_rest_bottom(tokens)
    else {
        return Ok(None);
    };
    let Some(card_type) = parse_card_type(spec.card_type_word) else {
        return Ok(None);
    };
    let order = match spec.order {
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Chosen => {
            ironsmith_core::LibraryBottomOrder::ChooserChooses
        }
        static_keyword_replacement_shapes::LibraryBottomOrderShape::Random => {
            ironsmith_core::LibraryBottomOrder::Random
        }
    };

    let mut filter = ObjectFilter::default();
    filter.card_types.push(card_type);

    Ok(Some(
        StaticAbility::draw_replacement_reveal_top_matching_to_hand_rest_bottom(
            spec.count,
            filter,
            order,
            render_token_slice(tokens),
        ),
    ))
}

pub fn parse_draw_replacement_double_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_double_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_double()));
    }

    Ok(None)
}

pub fn parse_draw_replacement_skip_empty_library_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    if is_draw_replacement_skip_empty_library_line_lexed(tokens) {
        return Ok(Some(StaticAbility::draw_replacement_skip_empty_library()));
    }

    Ok(None)
}

pub fn parse_conditional_draw_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = parser_token_word_refs(tokens);
    let always = || Condition::ValueComparison {
        left: Value::Fixed(1),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Fixed(1),
    };

    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if",
            "you",
            "would",
            "draw",
            "a",
            "card",
            "you",
            "may",
            "put",
            "a",
            "study",
            "counter",
            "on",
            "this",
            "enchantment",
            "instead",
        ],
    ) {
        return Ok(Some(
            StaticAbility::conditional_draw_replacement_with_optional(
                always(),
                vec![EffectAst::subject_verb_put_counters(
                    CounterType::Study,
                    Value::Fixed(1),
                    TargetAst::Source(None),
                    None,
                    false,
                )],
                true,
                render_token_slice(tokens),
            ),
        ));
    }

    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "you", "would", "draw", "a", "card", "you", "may", "instead", "search", "your",
            "library", "for", "a", "card", "put", "that", "card", "into", "your", "hand", "then",
            "shuffle",
        ],
    ) {
        let mut card = ObjectFilter::default();
        card.set_explicit_card_noun(true);
        return Ok(Some(
            StaticAbility::conditional_draw_replacement_with_optional(
                always(),
                vec![EffectAst::subject_verb_search_library(
                    card,
                    Zone::Hand,
                    PlayerAst::You,
                    PlayerAst::You,
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
                true,
                render_token_slice(tokens),
            ),
        ));
    }

    if crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "if", "you", "would", "draw", "a", "card", "you", "may", "instead", "choose", "land",
            "or", "nonland", "and", "reveal", "cards", "from", "the", "top", "of", "your",
            "library", "until", "you", "reveal", "a", "card", "of", "the", "chosen", "kind", "put",
            "that", "card", "into", "your", "hand", "and", "put", "all", "other", "cards",
            "revealed", "this", "way", "on", "the", "bottom", "of", "your", "library", "in", "any",
            "order",
        ],
    ) {
        let mode = |label: &str, filter: ObjectFilter, suffix: &str| {
            let all_tag = crate::tag::CompilerIndexedTag::DrawReplacementAll.key_in_scope(suffix);
            let match_tag =
                crate::tag::CompilerIndexedTag::DrawReplacementMatch.key_in_scope(suffix);
            ChooseOneModeAst {
                description: label.to_string(),
                effects: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::You,
                        LibraryConsultModeAst::Reveal,
                        filter,
                        LibraryConsultStopRuleAst::FirstMatch,
                        all_tag.clone(),
                        match_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(match_tag.clone(), None),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                        all_tag,
                        Some(match_tag),
                        LibraryBottomOrderAst::ChooserChooses,
                        PlayerAst::You,
                    ),
                ],
            }
        };
        let mut land = ObjectFilter {
            card_types: vec![CardType::Land],
            ..Default::default()
        };
        land.set_explicit_card_type_noun(Some(CardType::Land));
        let mut nonland = ObjectFilter {
            excluded_card_types: vec![CardType::Land],
            ..Default::default()
        };
        nonland.set_explicit_card_noun(true);
        let choose_kind = EffectAst::ChooseOneOf {
            modes: vec![
                mode("land", land, "land"),
                mode("nonland", nonland, "nonland"),
            ],
        };
        return Ok(Some(
            StaticAbility::conditional_draw_replacement_with_optional(
                always(),
                vec![choose_kind],
                true,
                render_token_slice(tokens),
            ),
        ));
    }

    if is_draw_replacement_win_empty_library_line_lexed(tokens) {
        return Ok(Some(StaticAbility::conditional_draw_replacement(
            Condition::ValueComparison {
                left: Value::CardsInLibrary(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(0),
            },
            vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
            render_token_slice(tokens),
        )));
    }

    let Some(fact) = late_static_facts::parse_conditional_draw_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let Some(no_cards_condition) =
        crate::grammar::conditions::parse_player_cards_in_hand_condition(fact.condition_tokens)
    else {
        return Ok(None);
    };
    if no_cards_condition.player != PlayerFilter::You || !no_cards_condition.is_no_cards_in_hand() {
        return Ok(None);
    }

    let draw_count = fact.draw_count;
    let mut replacement_effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::Draw {
            count: Value::Fixed(draw_count as i32),
        },
    )];
    if let Some(amount) = fact.life_loss {
        replacement_effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife {
                amount: Value::Fixed(amount as i32),
            },
        ));
    }

    let draw_amount_text = match draw_count {
        1 => "a".to_string(),
        2 => "two".to_string(),
        3 => "three".to_string(),
        4 => "four".to_string(),
        5 => "five".to_string(),
        _ => draw_count.to_string(),
    };
    let draw_card_text = if draw_count == 1 { "card" } else { "cards" };
    let mut display = format!(
        "If you would draw a card while you have no cards in hand, instead you draw {draw_amount_text} {draw_card_text}"
    );
    if let Some(amount) = fact.life_loss {
        display.push_str(&format!(" and you lose {amount} life"));
    }
    display.push('.');

    Ok(Some(StaticAbility::conditional_draw_replacement(
        Condition::Not(Box::new(Condition::CardsInHandOrMore(1))),
        replacement_effects,
        display,
    )))
}

pub fn parse_lose_game_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let words = TokenWordView::new(tokens);
    if !words.parses_prefix(&["if", "you", "would", "lose"])
        || words.parse_any_word_position_from(&["game"], 4).is_none()
    {
        return Ok(None);
    }
    let Some(instead_word) = words.parse_any_word_position_from(&["instead"], 4) else {
        return Ok(None);
    };
    let Some(effect_start) = words.token_index_after_words(instead_word + 1) else {
        return Ok(None);
    };
    let effect_tokens = trim_lexed_commas(&tokens[effect_start..]);
    if effect_tokens.is_empty() {
        return Ok(None);
    }
    let effects = super::super::clause_support::parse_effect_sentences_lexed(effect_tokens)?;
    let optional = (4..instead_word).any(|idx| words.get(idx) == Some("may"));
    Ok(Some(StaticAbilityAst::LoseGameReplacement {
        effects,
        optional,
        display: render_token_slice(tokens),
    }))
}

pub fn parse_keyword_action_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_keyword_action_replacement_tokens(tokens) else {
        return Ok(None);
    };
    let display = render_token_slice(tokens);
    Ok(Some(match shape {
        keyword_static_lines::KeywordActionReplacementShape::ProliferateYouTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::You),
                vec![EffectAst::subject_verb_proliferate(Value::Fixed(2))],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ProliferateOpponentTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Proliferate,
                ObjectFilter::default().controlled_by(PlayerFilter::Opponent),
                vec![EffectAst::subject_verb_proliferate(Value::Fixed(2))],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![
                    EffectAst::subject_verb_explore(TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                        None,
                    )),
                    EffectAst::subject_verb_explore(TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                        None,
                    )),
                ],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::ExploreAfterScry { value_tokens } => {
            let value_words = parser_token_word_refs(value_tokens);
            let (count, used) = parse_value_expr_words(&value_words).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                ))
            })?;
            if used != value_words.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported scry amount in keyword-action replacement (clause: '{}')",
                    render_token_slice(tokens)
                )));
            }
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::Explore,
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                vec![
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::You,
                        SubjectVerbActionAst::Scry { count },
                    ),
                    EffectAst::subject_verb_explore(TargetAst::Tagged(
                        crate::tag::CompilerReferenceTag::It.key(),
                        None,
                    )),
                ],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::AssembleRiggerTwice => {
            StaticAbility::keyword_action_replacement(
                crate::events::KeywordActionKind::AssembleContraption,
                ObjectFilter::default()
                    .with_subtype(crate::types::Subtype::Rigger)
                    .controlled_by(PlayerFilter::You),
                vec![EffectAst::subject_verb_emit_keyword_action(
                    crate::events::KeywordActionKind::AssembleContraption,
                    2,
                )],
                display,
            )
        }
        keyword_static_lines::KeywordActionReplacementShape::PlaneswalkAfterPlanarDeckChoice {
            count,
        } => StaticAbility::keyword_action_replacement_for_player(
            crate::events::KeywordActionKind::Planeswalk,
            PlayerFilter::You,
            vec![
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::Actor,
                    PlayerAst::You,
                    SubjectVerbActionAst::ReorderTopPlanarDeck { count },
                ),
                EffectAst::subject_verb_emit_keyword_action(
                    crate::events::KeywordActionKind::Planeswalk,
                    1,
                ),
            ],
            display,
        ),
        keyword_static_lines::KeywordActionReplacementShape::LearnReturnThisFromGraveyard => {
            StaticAbility::keyword_action_replacement_for_player_with_optional(
                crate::events::KeywordActionKind::Learn,
                PlayerFilter::You,
                vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Source(None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Owner,
                    false,
                    None,
                )],
                true,
                display,
            )
        }
    }))
}

pub fn parse_exile_to_countered_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = parse_exile_to_countered_exile_instead_of_graveyard_spec_lexed(tokens) else {
        return Ok(None);
    };

    Ok(Some(
        StaticAbility::exile_to_countered_exile_instead_of_graveyard(
            spec.player,
            spec.counter_type,
        ),
    ))
}

#[cfg(test)]
mod optional_draw_replacement_regression_tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse(text: &str) -> Option<StaticAbility> {
        let tokens = lex_line(text, 0).expect("draw replacement should lex");
        parse_conditional_draw_replacement_line(&tokens)
            .expect("draw replacement parser should not error")
    }

    #[test]
    fn strict_optional_draw_replacement_families_lower_to_event_replacements() {
        for text in [
            "If you would draw a card, you may put a study counter on this enchantment instead.",
            "If you would draw a card, you may instead search your library for a card, put that card into your hand, then shuffle.",
            "If you would draw a card, you may instead choose land or nonland and reveal cards from the top of your library until you reveal a card of the chosen kind. Put that card into your hand and put all other cards revealed this way on the bottom of your library in any order.",
        ] {
            let ability =
                parse(text).unwrap_or_else(|| panic!("expected typed replacement: {text}"));
            let ironsmith_core::StaticAbilityPayload::ConditionalDrawReplacement {
                optional,
                replacement_effects,
                ..
            } = ability.payload
            else {
                panic!("expected draw replacement payload: {ability:#?}");
            };
            assert!(optional, "{text}");
            assert!(!replacement_effects.is_empty(), "{text}");
        }
    }

    #[test]
    fn similar_nonreplacement_instructions_are_not_claimed() {
        for text in [
            "Whenever you draw a card, you may put a study counter on this enchantment.",
            "If you would draw a card, search your library for a card, put that card into your hand, then shuffle.",
            "If you would draw a card, you may instead choose land and reveal the top card of your library.",
        ] {
            assert!(parse(text).is_none(), "near miss was overclaimed: {text}");
        }
    }
}

pub fn parse_exile_to_exile_instead_of_graveyard_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_to_graveyard_replacement_tokens(tokens)
    else {
        return Ok(None);
    };
    let graveyard_owner = match spec.graveyard_owner {
        keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
        keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
        keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
    };
    let filter = match spec.filter_kind {
        keyword_static_lines::ExileGraveyardFilterKind::Source => ObjectFilter::source(),
        keyword_static_lines::ExileGraveyardFilterKind::AnyCard => {
            let mut filter = ObjectFilter::default();
            filter.set_explicit_card_noun(true);
            filter
        }
        keyword_static_lines::ExileGraveyardFilterKind::CreatureCard => ObjectFilter::creature(),
        keyword_static_lines::ExileGraveyardFilterKind::CyclingCard => {
            let mut filter = ObjectFilter::default().with_ability_marker("cycling");
            filter.set_explicit_card_noun(true);
            filter
        }
        keyword_static_lines::ExileGraveyardFilterKind::ObjectFilter => {
            match parse_object_filter(spec.filter_tokens, false) {
                Ok(filter) => filter,
                Err(_) if crate::lexer::is_bare_card_name_phrase(spec.filter_tokens) => {
                    ObjectFilter::source()
                }
                Err(error) => return Err(error),
            }
        }
    };
    let ability = if spec.exclude_cycled {
        StaticAbility::exile_to_exile_instead_of_graveyard_unless_cycled(filter, graveyard_owner)
    } else {
        StaticAbility::exile_to_exile_instead_of_graveyard(filter, graveyard_owner)
    };
    Ok(Some(ability))
}

pub fn parse_exile_would_die_instead_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(spec) = keyword_static_lines::parse_exile_would_die_tokens(tokens) else {
        return Ok(None);
    };
    let ability = match spec {
        keyword_static_lines::ExileWouldDieSpec::NontokenCreature {
            controller,
            exile_counter,
            follow_up_token,
        } => {
            let matched_filter = match controller {
                keyword_static_lines::ReplacementPlayerKind::Any => {
                    ObjectFilter::creature().nontoken()
                }
                keyword_static_lines::ReplacementPlayerKind::You => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::You),
                keyword_static_lines::ReplacementPlayerKind::Opponent => ObjectFilter::creature()
                    .nontoken()
                    .controlled_by(PlayerFilter::Opponent),
            };
            let exile_with_counters = exile_counter
                .map(|counter_type| vec![(counter_type, 1)])
                .unwrap_or_default();
            let follow_up = follow_up_token
                .map(build_replacement_creature_token)
                .map(|token| vec![token])
                .unwrap_or_default();
            StaticAbility::exile_would_die_instead_with_damage_source_counters_and_follow_up(
                matched_filter,
                None,
                exile_with_counters,
                follow_up,
            )
        }
        keyword_static_lines::ExileWouldDieSpec::DamagedBy { victim, damaged_by } => {
            let victim = match victim {
                keyword_static_lines::ExileWouldDieVictimKind::Creature => ObjectFilter::creature(),
                keyword_static_lines::ExileWouldDieVictimKind::Permanent => {
                    ObjectFilter::permanent()
                }
            };
            StaticAbility::exile_would_die_instead_with_damage_source(victim, Some(damaged_by))
        }
        keyword_static_lines::ExileWouldDieSpec::DamagedByFilter {
            victim,
            damager_filter_tokens,
        } => {
            let victim = match victim {
                keyword_static_lines::ExileWouldDieVictimKind::Creature => ObjectFilter::creature(),
                keyword_static_lines::ExileWouldDieVictimKind::Permanent => {
                    ObjectFilter::permanent()
                }
            };
            let damager_filter = if keyword_static_lines::parse_you_controlled_source_filter_tokens(
                &damager_filter_tokens,
            ) {
                ObjectFilter::default().controlled_by(PlayerFilter::You)
            } else {
                parse_object_filter_lexed(&damager_filter_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported filtered damage source in would-die replacement (clause: '{}')",
                        render_token_slice(&damager_filter_tokens)
                    ))
                })?
            };
            StaticAbility::exile_would_die_instead_with_damage_filter_surface(
                victim,
                damager_filter,
                Some(render_token_slice(&damager_filter_tokens)),
            )
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleSource(kind) => {
            let filter = match kind {
                keyword_static_lines::SimpleSourceReplacementKind::Any => ObjectFilter::source(),
                keyword_static_lines::SimpleSourceReplacementKind::Creature => {
                    ObjectFilter::source().with_type(CardType::Creature)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Artifact => {
                    ObjectFilter::source().with_type(CardType::Artifact)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Enchantment => {
                    ObjectFilter::source().with_type(CardType::Enchantment)
                }
                keyword_static_lines::SimpleSourceReplacementKind::Permanent => {
                    ObjectFilter::source()
                }
            };
            StaticAbility::exile_would_die_instead(filter)
        }
        keyword_static_lines::ExileWouldDieSpec::SimpleCreature(player) => {
            let player = match player {
                keyword_static_lines::ReplacementPlayerKind::Any => PlayerFilter::Any,
                keyword_static_lines::ReplacementPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ReplacementPlayerKind::Opponent => PlayerFilter::Opponent,
            };
            StaticAbility::exile_would_die_instead(ObjectFilter::creature().controlled_by(player))
        }
    };
    Ok(Some(ability))
}

pub fn build_replacement_creature_token(
    shape: crate::model::token_definition::CreatureTokenShape,
) -> EffectAst {
    let name = shape.name.clone();
    EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::CreateTokenWithMods {
            name,
            definition: crate::model::token_definition::TokenDefinitionSpec::Creature(shape),
            count: Value::Fixed(1),
            dynamic_power_toughness: None,
            player: PlayerAst::Implicit,
            actor_surface_explicit: false,
            attached_to: None,
            tapped: false,
            attacking: false,
            attack_target_player: None,
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

pub fn parse_discard_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    Ok(
        static_keyword_replacement_shapes::parse_discard_or_redirect_replacement(tokens).map(
            |shape| {
                StaticAbility::discard_or_redirect_replacement(
                    ObjectFilter::default().with_type(shape.discard_type),
                    shape.redirect_zone,
                )
            },
        ),
    )
}

pub fn parse_sacrifice_or_redirect_replacement_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let Some(shape) =
        static_keyword_replacement_shapes::parse_sacrifice_or_redirect_replacement(tokens)
    else {
        return Ok(None);
    };
    let filter = parse_object_filter(shape.filter_tokens, false)?;
    Ok(Some(StaticAbility::sacrifice_or_redirect_replacement(
        filter,
        shape.count,
        shape.redirect_zone,
    )))
}

pub fn parse_pay_life_or_enter_tapped_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let fact = match late_static_facts::parse_pay_life_or_enter_tapped_tokens(tokens) {
        Ok(Some(fact)) => fact,
        Ok(None) => return Ok(None),
        Err(error) => {
            let clause = parser_token_word_refs(tokens).join(" ");
            let detail = match error {
                late_static_facts::PayLifeOrEnterTappedError::MissingPay => {
                    "missing 'pay' keyword in pay-life ETB clause"
                }
                late_static_facts::PayLifeOrEnterTappedError::UnsupportedPrefix => {
                    "unsupported pay-life ETB prefix"
                }
                late_static_facts::PayLifeOrEnterTappedError::MissingAmount => {
                    "missing life payment amount in pay-life ETB clause"
                }
                late_static_facts::PayLifeOrEnterTappedError::MissingIfYouDont => {
                    "unsupported pay-life ETB trailing clause (expected 'if you don't ...')"
                }
                late_static_facts::PayLifeOrEnterTappedError::UnsupportedTail => {
                    "unsupported pay-life ETB trailing clause"
                }
            };
            return Err(CardTextError::ParseError(format!(
                "{detail} (clause: '{clause}')"
            )));
        }
    };

    parser_trace("parse_static:pay-life-etb:matched", tokens);
    Ok(Some(StaticAbility::pay_life_or_enter_tapped(fact.amount)))
}

pub fn parse_copy_activated_abilities_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(fact) = late_static_facts::parse_copy_activated_abilities_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);

    let (condition, subject_start) = match parse_anthem_prefix_condition(tokens, fact.marker_token)
    {
        Ok(parsed) => parsed,
        Err(_) => return Ok(None),
    };
    let subject_tokens = trim_commas(&tokens[subject_start..fact.marker_token]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject = if crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(&subject_tokens),
        &["this"],
    ) {
        AnthemSubjectAst::Source
    } else {
        match parse_anthem_subject(&subject_tokens) {
            Ok(subject) => subject,
            Err(_) => return Ok(None),
        }
    };

    let filter_tokens =
        trim_edge_punctuation(&tokens[fact.filter_start_token..fact.filter_end_token]);
    let filter_tokens = strip_leading_token_words_any(&filter_tokens, &["all", "each"]).to_vec();
    let force_once_each_turn = fact.once_each_turn_word_start.is_some();
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter = match parse_object_filter(&filter_tokens, false) {
        Ok(filter) => filter,
        Err(_) => return Ok(None),
    };

    let counter = match filter.with_counter {
        Some(crate::filter::CounterConstraint::Typed(counter_type)) => Some(counter_type),
        _ => None,
    };

    let display_words = copy_activated_abilities_display_words(&clause_words);
    let display = if force_once_each_turn {
        let display_tail_start = fact
            .once_each_turn_word_start
            .map(|start| copy_activated_display_index_for_original_word(&clause_words, start));
        if let Some(start) = display_tail_start {
            format!(
                "{}. You may activate each of those abilities only once each turn",
                display_words[..start].join(" ").trim()
            )
        } else {
            display_words.join(" ")
        }
    } else {
        display_words.join(" ")
    };

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(fact.exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(display);
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if fact.only_loyalty {
        ability = ability.with_only_loyalty();
    }
    if force_once_each_turn {
        ability = ability.with_once_each_turn();
    }

    let ability = StaticAbility::copy_activated_abilities(ability);
    let ast = match subject {
        AnthemSubjectAst::Source => match condition {
            Some(condition) => StaticAbilityAst::ConditionalStaticAbility {
                ability: Box::new(StaticAbilityAst::Static(ability)),
                condition,
            },
            None => StaticAbilityAst::Static(ability),
        },
        AnthemSubjectAst::Filter(subject_filter) => StaticAbilityAst::GrantStaticAbility {
            filter: subject_filter,
            ability: Box::new(StaticAbilityAst::Static(ability)),
            condition,
        },
    };

    Ok(Some(ast))
}

pub fn copy_activated_abilities_display_words<'a>(clause_words: &[&'a str]) -> Vec<&'a str> {
    let mut display_words = Vec::with_capacity(clause_words.len());
    for (idx, word) in clause_words.iter().copied().enumerate() {
        if copy_activated_should_skip_display_word(clause_words, idx, word) {
            continue;
        }
        display_words.push(word);
    }
    display_words
}

pub fn copy_activated_display_index_for_original_word(
    clause_words: &[&str],
    original_idx: usize,
) -> usize {
    clause_words
        .iter()
        .enumerate()
        .take(original_idx)
        .filter(|(idx, word)| !copy_activated_should_skip_display_word(clause_words, *idx, word))
        .count()
}

pub fn copy_activated_should_skip_display_word(
    clause_words: &[&str],
    idx: usize,
    word: &str,
) -> bool {
    idx >= 2
        && word == clause_words[idx - 1]
        && clause_words[idx - 2] == "this"
        && copy_activated_display_source_noun(word)
}

pub fn copy_activated_display_source_noun(word: &str) -> bool {
    matches!(word, "card" | "permanent" | "source" | "spell")
        || parse_card_type(word).is_some()
        || parse_subtype_flexible(word).is_some()
}

pub fn parse_spend_mana_as_any_color_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbilityAst>, CardTextError> {
    let Some(shape) = keyword_static_lines::parse_mana_spend_permission_tokens(tokens) else {
        return Ok(None);
    };
    let clause_words = parser_token_word_refs(tokens);
    let (permission, display) = match shape {
        keyword_static_lines::ManaSpendPermissionShape::SymbolAsAnyColorOtherAsColorless {
            symbol,
        } => {
            let symbol_text = match symbol {
                ManaSymbol::White => "white",
                ManaSymbol::Blue => "blue",
                ManaSymbol::Black => "black",
                ManaSymbol::Red => "red",
                ManaSymbol::Green => "green",
                _ => unreachable!("typed grammar only returns colored mana symbols"),
            };
            (
                crate::effect::ManaSpendPermission::mana_symbol_as_any_color_other_as_colorless(
                    PlayerFilter::You,
                    symbol,
                ),
                format!(
                    "You may spend {symbol_text} mana as though it were mana of any color. You may spend other mana only as though it were colorless mana"
                ),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyTypeToCast { filter_tokens } => {
            let filter = parse_object_filter(filter_tokens, false)
                .map(|mut filter| {
                    filter.zone = None;
                    filter.stack_kind = None;
                    filter.has_mana_cost = false;
                    filter
                })
                .map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported mana spend cast filter (clause: '{}')",
                        clause_words.join(" ")
                    ))
                })?;
            (
                crate::effect::ManaSpendPermission::any_type_for_casting_matching(
                    PlayerFilter::You,
                    filter,
                ),
                clause_words.join(" "),
            )
        }
        keyword_static_lines::ManaSpendPermissionShape::AnyColor {
            player,
            activation_filter_tokens,
            source_activation_only,
        } => {
            let player = match player {
                keyword_static_lines::ManaSpendPlayerKind::You => PlayerFilter::You,
                keyword_static_lines::ManaSpendPlayerKind::Any => PlayerFilter::Any,
            };
            let permission = if source_activation_only {
                crate::effect::ManaSpendPermission::any_color_for_activation(
                    player.clone(),
                    ObjectFilter::source(),
                )
            } else if let Some(filter_tokens) = activation_filter_tokens {
                let filter = match parse_object_filter(filter_tokens, false) {
                    Ok(filter) => filter,
                    Err(_) => return Ok(None),
                };
                crate::effect::ManaSpendPermission::any_color_for_activation(player.clone(), filter)
            } else {
                crate::effect::ManaSpendPermission::any_color(player.clone())
            };
            let display = if player == PlayerFilter::Any {
                "Players may spend mana as though it were mana of any color".to_string()
            } else {
                clause_words.join(" ")
            };
            (permission, display)
        }
    };

    Ok(Some(StaticAbilityAst::Static(
        StaticAbility::mana_spend_permission(permission, display),
    )))
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn spell_reduction_and_cant_be_countered_keeps_both_typed_clauses() {
        let tokens = lex_line(
            "Spells with flash you cast cost {1} less to cast and can't be countered.",
            0,
        )
        .expect("compound spell rule should lex");
        let abilities = parse_spells_cost_reduction_and_cant_be_countered_line(&tokens)
            .expect("compound spell rule should not hard-error")
            .expect("compound spell rule should parse");
        let [reduction, restriction] = abilities.as_slice() else {
            panic!("expected reduction plus restriction, got {abilities:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &reduction.payload
        else {
            panic!("expected typed cost reduction, got {reduction:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::RuleRestriction {
            restriction: crate::effect::Restriction::BeCountered(protected),
            additional_restrictions,
            ..
        } = &restriction.payload
        else {
            panic!("expected typed counter restriction, got {restriction:#?}");
        };
        assert!(additional_restrictions.is_empty());
        assert_eq!(protected, &reduction.filter);

        let unrelated = lex_line(
            "Spells with flash you cast cost {1} less to cast and have ward {1}.",
            0,
        )
        .expect("unrelated rule should lex");
        assert!(
            parse_spells_cost_reduction_and_cant_be_countered_line(&unrelated)
                .expect("unrelated rule should not hard-error")
                .is_none()
        );
    }

    #[test]
    fn source_graveyard_dynamic_surcharge_lowers_to_permission_and_counted_tax() {
        let tokens = lex_line(
            "You may cast this creature from your graveyard if you pay {1} more to cast it for each other creature card in your graveyard.",
            0,
        )
        .expect("source graveyard surcharge should lex");
        let abilities = parse_source_graveyard_dynamic_surcharge_line(&tokens)
            .expect("source graveyard surcharge should not hard-error")
            .expect("source graveyard surcharge should parse");
        let [
            StaticAbilityAst::Static(permission),
            StaticAbilityAst::Static(tax),
        ] = abilities.as_slice()
        else {
            panic!("expected a permission and source tax, got {abilities:#?}");
        };

        let ironsmith_core::StaticAbilityPayload::Grants(grant) = &permission.payload else {
            panic!("expected typed grant, got {permission:#?}");
        };
        assert!(matches!(
            &grant.grantable,
            crate::model::CompilerGrantableCore::PlayFrom
        ));
        assert_eq!(grant.zone, Zone::Graveyard);
        assert_eq!(grant.beneficiary, PlayerFilter::You);
        assert!(grant.filter.source);
        assert!(matches!(
            grant.filter.source_surface,
            Some(crate::target::SourceReferenceSurface::ThisPermanentType(ref surface))
                if surface == "this creature"
        ));

        let ironsmith_core::StaticAbilityPayload::CostIncrease(increase) = &tax.payload else {
            panic!("expected dynamic cost increase, got {tax:#?}");
        };
        assert!(increase.filter.source);
        let Value::Count(counted) = increase.amount.unhinted() else {
            panic!("expected counted dynamic tax, got {:#?}", increase.amount);
        };
        assert_eq!(counted.zone, Some(Zone::Graveyard));
        assert_eq!(counted.owner, Some(PlayerFilter::You));
        assert_eq!(counted.card_types, vec![CardType::Creature]);
        assert!(counted.other);

        let plain = lex_line("You may cast this creature from your graveyard.", 0)
            .expect("plain permission should lex");
        assert!(
            parse_source_graveyard_dynamic_surcharge_line(&plain)
                .expect("plain permission should not hard-error")
                .is_none()
        );
    }

    #[test]
    fn global_colorless_line_lowers_with_typed_multi_domain_surface() {
        let tokens = lex_line(
            "All cards that aren't on the battlefield, spells, and permanents are colorless.",
            0,
        )
        .expect("global colorless line should lex");
        let ability = parse_all_cards_spells_permanents_colorless_line(&tokens)
            .expect("global colorless line should not hard-error")
            .expect("global colorless line should parse");
        let ironsmith_core::StaticAbilityPayload::MakeColorless(filter) = &ability.payload else {
            panic!("expected MakeColorless payload, got {ability:#?}");
        };
        assert_eq!(
            filter.global_characteristic_domain_surface(),
            Some(
                ironsmith_core::GlobalCharacteristicDomainSurface::CardsOutsideBattlefieldSpellsAndPermanents
            )
        );
        assert_eq!(filter, &ObjectFilter::default());
    }

    #[test]
    fn animation_bundle_accepts_a_quoted_granted_trigger() {
        let tokens = lex_line(
            "During your turn, each non-Equipment artifact and non-Aura enchantment you control \
             with mana value 4 or greater is a 4/4 Elemental creature in addition to its other \
             types and has indestructible, haste, and \"Whenever this creature deals combat \
             damage to a player, draw a card.\"",
            0,
        )
        .expect("compound animation should lex");
        let abilities = parse_filter_is_pt_creature_in_addition_and_has_line(&tokens)
            .expect("compound animation should not hard-error")
            .expect("quoted granted trigger should not mask the animation parser");

        assert_eq!(abilities.len(), 6, "{abilities:#?}");
        let debug = format!("{abilities:#?}");
        assert!(debug.contains("Triggered"), "{debug}");
        assert!(debug.contains("action: Draw("), "{debug}");
    }

    #[test]
    fn conditioned_source_animation_without_a_grant_is_structural() {
        let tokens = lex_line(
            "As long as this isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.",
            0,
        )
        .expect("source animation should lex");
        let abilities = parse_filter_is_pt_creature_in_addition_line(&tokens)
            .expect("source animation should not hard-error")
            .expect("source animation should parse");

        assert_eq!(abilities.len(), 3, "{abilities:#?}");
        let debug = format!("{abilities:#?}");
        assert!(debug.contains("AddCardTypes"), "{debug}");
        assert!(debug.contains("SetBasePowerToughness"), "{debug}");
        assert!(debug.contains("Insect"), "{debug}");
        assert!(debug.contains("SourceIsInZone"), "{debug}");
        assert!(debug.contains("Not("), "{debug}");
    }

    #[test]
    fn plural_creature_type_addition_is_a_static_grant() {
        let tokens = lex_line(
            "Creatures you control are Slivers in addition to their other creature types.",
            0,
        )
        .expect("creature-type addition should lex");
        let abilities = parse_subject_are_card_types_in_addition_to_their_other_types_line(&tokens)
            .expect("static type addition should not hard-error")
            .expect("static type addition should parse");

        assert_eq!(abilities.len(), 1, "{abilities:#?}");
        assert_eq!(abilities[0].id(), StaticAbilityId::AddSubtypes);
        let ironsmith_core::StaticAbilityPayload::AddSubtypes { filter, subtypes } =
            &abilities[0].payload
        else {
            panic!("expected a typed add-subtypes payload, got {abilities:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(subtypes, &[crate::types::Subtype::Sliver]);
    }

    #[test]
    fn supported_keyword_marker_uses_token_shapes_for_crew_markers() {
        for line in [
            "This creature crews Vehicles using its toughness rather than its power.",
            "This token saddles Mounts and crews Vehicles as though its power were 2 greater.",
            "You may remove a loyalty counter from a planeswalker you control rather than pay this creature's crew cost.",
        ] {
            let tokens = lex_line(line, 0).expect("marker line should lex");
            let text = render_token_slice(&tokens);
            assert!(
                supported_keyword_marker_tokens(&tokens, &text),
                "{line} should be recognized through token shapes"
            );
        }
    }

    #[test]
    fn pt_modifier_parsers_use_char_signs() {
        assert_eq!(parse_pt_modifier("+2/-1").unwrap(), (2, -1));
        assert_eq!(parse_pt_modifier("2/+3").unwrap(), (2, 3));
        assert_eq!(
            parse_pt_modifier_values("−X/+2").unwrap(),
            (Value::XTimes(-1), Value::Fixed(2))
        );
    }

    #[test]
    fn early_static_ability_parser_uses_parser_token_words() {
        for line in [
            "X can't be greater than the number of players in the game.",
            "This creature can't attack unless you've cast a creature spell this turn.",
            "During your turn, as long as you haven't activated an exhaust ability this turn, you may activate exhaust abilities as though they haven't been activated.",
        ] {
            let tokens = lex_line(line, 0).expect("static line should lex");
            assert!(
                parse_static_ability_ast_line_early_lexed(&tokens)
                    .expect("early static line should parse")
                    .is_some(),
                "{line} should match through parser token words"
            );
        }
    }

    #[test]
    fn play_permission_keeps_cast_this_way_entry_counter_rider() {
        let tokens = lex_line(
            "You may play lands and cast Mutant, Ninja, or Turtle spells from the top of your library. If you cast a creature spell this way, that creature enters with an additional +1/+1 counter on it.",
            0,
        )
        .expect("play permission should lex");
        let ability = parse_play_from_permission_with_enter_counter_this_way_line(&tokens)
            .expect("play permission should not hard-error")
            .expect("play permission should keep its entry-counter rider");
        let debug = format!("{ability:#?}");

        assert!(debug.contains("PlayFrom"), "{debug}");
        assert!(debug.contains("cast_this_way_grants"), "{debug}");
        assert!(debug.contains("cast_this_way_filter"), "{debug}");
        assert!(debug.contains("Creature"), "{debug}");
        assert!(debug.contains("PlusOnePlusOne"), "{debug}");
    }

    #[test]
    fn source_linked_exile_permission_keeps_owner_subtype_and_finality_rider() {
        let tokens = lex_line(
            "You may cast Dinosaur creature spells from among cards you own exiled with this creature. If you cast a spell this way, that creature enters with a finality counter on it.",
            0,
        )
        .expect("source-linked exile permission should lex");
        let ability = parse_play_from_permission_with_enter_counter_this_way_line(&tokens)
            .expect("source-linked exile permission should not hard-error")
            .expect("source-linked exile permission should keep its finality rider");
        let ironsmith_core::StaticAbilityPayload::Grants(spec) = &ability.payload else {
            panic!("expected a grant ability, got {ability:#?}");
        };

        assert_eq!(spec.zone, crate::zone::Zone::Exile);
        assert_eq!(spec.filter.zone, Some(crate::zone::Zone::Exile));
        assert_eq!(spec.filter.owner, Some(PlayerFilter::You));
        assert_eq!(spec.filter.card_types, vec![CardType::Creature]);
        assert!(
            spec.filter
                .subtypes
                .contains(&crate::types::Subtype::Dinosaur)
        );
        assert!(spec.filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::SourceExiled.as_str()
                && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(spec.cast_this_way_grants.iter().any(|grant| {
            grant.id() == crate::static_abilities::StaticAbilityId::EnterWithCounters
        }));
        assert_eq!(
            spec.display(),
            "You may cast Dinosaur creature spells from among cards you own exiled with this creature. If you cast a spell this way, that creature enters with a finality counter on it"
        );
    }

    #[test]
    fn scorched_ruins_entry_payment_is_a_typed_sacrifice_replacement() {
        let definition = crate::cards::builders::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Scorched Ruins",
        )
            .card_types(vec![CardType::Land])
            .parse_text(
                "If this land would enter, sacrifice two untapped lands instead. If you do, put this land onto the battlefield. If you don't, put it into its owner's graveyard.\n{T}: Add {C}{C}{C}{C}.",
            )
            .expect("Scorched Ruins should parse through the static replacement family");
        let replacement = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                crate::ability::AbilityKind::Static(static_ability)
                    if static_ability.id()
                        == crate::static_abilities::StaticAbilityId::SacrificeOrRedirectReplacement =>
                {
                    Some(static_ability)
                }
                _ => None,
            })
            .expect("Scorched Ruins should retain the typed replacement");
        let ironsmith_core::StaticAbilityPayload::SacrificeOrRedirectReplacement {
            filter,
            count,
            redirect_zone,
        } = &replacement.payload
        else {
            panic!("expected sacrifice-or-redirect payload: {replacement:#?}");
        };

        assert_eq!(*count, 2);
        assert_eq!(*redirect_zone, Zone::Graveyard);
        assert_eq!(filter.card_types, vec![CardType::Land]);
        assert!(filter.untapped);
        assert_eq!(
            replacement.display(),
            "sacrifice or redirect replacement",
            "the core label remains an implementation label; runtime rendering owns Oracle text"
        );
        assert!(definition.abilities.iter().any(|ability| {
            matches!(&ability.kind, crate::ability::AbilityKind::Activated(activated)
                if format!("{activated:#?}").matches("Colorless").count() >= 4)
        }));
    }

    #[test]
    fn parse_keyword_action_replacement_static_line() {
        let tokens =
            lex_line("If you would proliferate, proliferate twice instead.", 0).expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("keyword-action replacement parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|ability| ability.id() == StaticAbilityId::KeywordActionReplacement),
            "expected keyword-action replacement static ability, got {parsed:?}"
        );
        let parsed = parse_static_ability_ast_line_lexed(&tokens)
            .expect("static ability line parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|abilities| abilities.iter().any(
                    |ability| matches!(ability, StaticAbilityAst::Static(static_ability)
                    if static_ability.id() == StaticAbilityId::KeywordActionReplacement)
                )),
            "expected static line parser to preserve keyword-action replacement, got {parsed:?}"
        );

        let tokens = lex_line(
            "If a Rigger you control would assemble a Contraption, it assembles two Contraptions instead.",
            0,
        )
        .expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("assemble replacement parser should not hard-error");
        assert!(
            parsed
                .as_ref()
                .is_some_and(|ability| ability.id() == StaticAbilityId::KeywordActionReplacement),
            "expected typed assemble replacement static ability, got {parsed:?}"
        );

        let tokens = lex_line(
            "If you would planeswalk, instead look at the top two cards of your planar deck, put one on the bottom of your planar deck and the other on top, then planeswalk.",
            0,
        )
        .expect("lex");
        let parsed = parse_keyword_action_replacement_line(&tokens)
            .expect("planeswalk replacement parser should not hard-error")
            .expect("planeswalk replacement should lower");
        let debug = format!("{parsed:?}");
        assert_eq!(parsed.id(), StaticAbilityId::KeywordActionReplacement);
        assert!(debug.contains("performer_filter: Some(You)"), "{debug}");
        assert!(debug.contains("ReorderTopPlanarDeck"), "{debug}");
        assert!(debug.contains("Planeswalk"), "{debug}");
    }

    #[test]
    fn dynamic_cost_other_shapes_prefer_typed_turn_history_counts() {
        for (text, expected_query) in [
            (
                "less to cast for each opponent who was dealt damage this turn.",
                "PlayersDealtDamage",
            ),
            (
                "less to cast for each card you've cycled or discarded this turn.",
                "DiscardedOrCycled",
            ),
            (
                "less to cast for each creature you attacked with this turn.",
                "CreaturesAttackedWith",
            ),
            (
                "for each of your opponents who lost life this turn.",
                "PlayersLostLife",
            ),
        ] {
            let tokens = lex_line(text, 0).expect("dynamic cost text should lex");
            let value = parse_dynamic_cost_modifier_value(&tokens)
                .expect("dynamic cost should not hard-error")
                .expect("dynamic cost should produce a value");
            let debug = format!("{value:?}");
            assert!(
                debug.contains("TurnHistoryCount")
                    && debug.contains(expected_query)
                    && debug.contains("ForEach"),
                "expected typed for-each turn-history value for {text}, got {debug}"
            );
        }
    }

    #[test]
    fn dynamic_for_each_commander_cast_count_precedes_generic_object_count() {
        let tokens = lex_line(
            "for each time you've cast your commander from the command zone this game.",
            0,
        )
        .expect("commander-count text should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic count should not hard-error")
            .expect("dynamic count should produce a value");
        assert_eq!(
            value,
            Value::CommanderCastCount(PlayerFilter::You),
            "the narrow command-zone history value must win before generic object counting"
        );
    }

    #[test]
    fn dynamic_attacked_opponent_cost_uses_player_count_value() {
        let tokens = lex_line(
            "This spell costs {1} less to cast for each opponent you're attacking.",
            0,
        )
        .expect("attacked-opponent reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("attacked-opponent reduction should not hard-error")
            .expect("attacked-opponent reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
            &ability.payload
        else {
            panic!("expected this-spell reduction payload: {ability:#?}");
        };

        assert!(
            reduction.amount.has_surface_hint(ValueSurfaceHint::ForEach),
            "{:#?}",
            reduction.amount
        );
        assert_eq!(reduction.amount.unhinted(), &Value::PlayersBeingAttacked);
    }

    #[test]
    fn dynamic_discarded_this_way_cost_retains_typed_action() {
        let direct_words = ["for", "each", "card", "discarded", "this", "way"];
        let (direct, used) = parse_for_each_count_value_words(&direct_words)
            .expect("shared count grammar should parse discarded-this-way");
        assert_eq!(used, direct_words.len());
        assert!(
            matches!(direct.unhinted(), Value::PendingPriorEffectMetric(_)),
            "shared count grammar erased the discarded action: {direct:#?}"
        );

        let tokens =
            lex_line("for each card discarded this way.", 0).expect("discard count should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic count should not hard-error")
            .expect("dynamic count should produce a value");
        let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
            panic!("expected a typed discarded-card action count, got {value:#?}");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Discarded)
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
        assert!(
            query
                .filter
                .as_ref()
                .is_some_and(|filter| { filter.union_surface.explicit_card_noun() })
        );
    }

    #[test]
    fn dynamic_sacrificed_this_way_cost_retains_typed_action_and_for_each_surface() {
        let tokens = lex_line("for each permanent sacrificed this way.", 0)
            .expect("sacrifice count should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic count should not hard-error")
            .expect("dynamic count should produce a value");
        assert!(value.has_surface_hint(ValueSurfaceHint::ForEach));
        let Value::PendingPriorEffectMetric(query) = value.unhinted() else {
            panic!("expected a typed sacrificed-permanent count, got {value:#?}");
        };
        assert_eq!(
            query.action,
            Some(ironsmith_core::PriorEffectAction::Sacrificed)
        );
        assert_eq!(query.metric, ironsmith_core::EffectMetric::Count);
    }

    #[test]
    fn compound_this_spell_reduction_keeps_each_scaled_dynamic_basis() {
        let tokens = lex_line(
            "This spell costs {2} less to cast for each permanent sacrificed this way and {2} less to cast for each other artifact or creature you've sacrificed this turn.",
            0,
        )
        .expect("compound reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("compound reduction should not hard-error")
            .expect("compound reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
            &ability.payload
        else {
            panic!("expected this-spell reduction payload: {ability:#?}");
        };
        let debug = format!("{:#?}", reduction.amount);
        assert!(
            debug.contains("PendingPriorEffectMetric")
                && debug.contains("Sacrificed")
                && debug.contains("TurnHistoryCount"),
            "expected both sacrifice bases to remain typed, got {debug}"
        );
        assert_eq!(
            debug.matches("PendingPriorEffectMetric").count(),
            2,
            "the first {{2}} reduction should scale its prior-effect basis twice: {debug}"
        );
        assert_eq!(
            debug.matches("TurnHistoryCount").count(),
            2,
            "the second {{2}} reduction should scale its turn-history basis twice: {debug}"
        );
    }

    #[test]
    fn specialized_card_types_among_cost_value_precedes_history_fallback() {
        let tokens = lex_line(
            "less to cast for each card type among permanents you've sacrificed this turn.",
            0,
        )
        .expect("card-types-among cost text should lex");
        let value = parse_dynamic_cost_modifier_value(&tokens)
            .expect("dynamic cost should not hard-error")
            .expect("dynamic cost should produce a value");
        assert!(
            matches!(value.unhinted(), Value::CardTypesAmong(_)),
            "expected specialized card-types-among value, got {value:?}"
        );
    }

    #[test]
    fn shared_characteristic_cost_reduction_keeps_candidate_intersection() {
        let tokens = lex_line(
            "Spells you cast cost {1} less to cast for each card type they share with cards exiled with this creature.",
            0,
        )
        .expect("shared-characteristic reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("shared-characteristic reduction should not hard-error")
            .expect("shared-characteristic reduction should parse");
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &ability.payload
        else {
            panic!("expected shared cost-reduction payload: {ability:#?}");
        };
        let intersection = reduction
            .characteristic_intersection
            .as_ref()
            .expect("expected typed characteristic intersection");
        assert_eq!(
            intersection.characteristic,
            ironsmith_core::ObjectCharacteristic::CardType
        );
        assert_eq!(
            intersection.comparison_surface.as_deref(),
            Some("cards exiled with this creature")
        );
        assert!(
            intersection
                .comparison
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str()
                    == crate::tag::CompilerReferenceTag::SourceExiled.as_str()),
            "{:#?}",
            intersection.comparison
        );
    }

    #[test]
    fn self_cost_reductions_keep_for_each_player_and_disjoint_zone_counts() {
        let opponent_tokens = lex_line(
            "This spell costs {1} less to cast for each opponent you have.",
            0,
        )
        .expect("opponent-count reduction should lex");
        let opponent_ability = parse_spells_cost_modifier_line(&opponent_tokens)
            .expect("opponent-count reduction should not hard-error")
            .expect("opponent-count reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(opponent_reduction) =
            &opponent_ability.payload
        else {
            panic!("expected this-spell reduction payload: {opponent_ability:#?}");
        };
        assert!(
            opponent_reduction
                .amount
                .has_surface_hint(ValueSurfaceHint::ForEach)
        );
        assert_eq!(
            opponent_reduction.amount.unhinted(),
            &Value::CountPlayers(PlayerFilter::Opponent)
        );

        let cave_tokens = lex_line(
            "This spell costs {1} less to cast for each Cave you control and each Cave card in your graveyard.",
            0,
        )
        .expect("multi-zone Cave reduction should lex");
        let cave_ability = parse_spells_cost_modifier_line(&cave_tokens)
            .expect("multi-zone Cave reduction should not hard-error")
            .expect("multi-zone Cave reduction should parse");
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(cave_reduction) =
            &cave_ability.payload
        else {
            panic!("expected this-spell reduction payload: {cave_ability:#?}");
        };
        assert!(
            cave_reduction
                .amount
                .has_surface_hint(ValueSurfaceHint::ForEach),
            "{:#?}",
            cave_reduction.amount
        );
        let Value::Count(cave_filter) = cave_reduction.amount.unhinted() else {
            panic!("expected a Cave object count: {:#?}", cave_reduction.amount);
        };
        assert_eq!(cave_filter.any_of.len(), 2, "{cave_filter:#?}");
        assert!(
            cave_filter
                .any_of
                .iter()
                .any(|arm| arm.zone == Some(Zone::Battlefield)),
            "{cave_filter:#?}"
        );
        assert!(
            cave_filter
                .any_of
                .iter()
                .any(|arm| arm.zone == Some(Zone::Graveyard)),
            "{cave_filter:#?}"
        );
    }

    fn parsed_spell_cost_filter(line: &str) -> ObjectFilter {
        let tokens = lex_line(line, 0).expect("spell-cost line should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("spell-cost parser should not hard-error")
            .expect("spell-cost line should be recognized");
        match ability.payload {
            ironsmith_core::StaticAbilityPayload::CostReduction(reduction) => reduction.filter,
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => {
                reduction.filter
            }
            other => panic!("expected a shared spell-cost reduction, got {other:?}"),
        }
    }

    #[test]
    fn chosen_type_spell_cost_filters_survive_actor_word_order_variants() {
        for (line, creature_only) in [
            (
                "Spells you cast of the chosen type cost {1} less to cast.",
                false,
            ),
            (
                "Creature spells you cast of the chosen type cost {1} less to cast.",
                true,
            ),
            (
                "Spells of the chosen type you cast cost {W}{U}{B}{R}{G} less to cast.",
                false,
            ),
            (
                "Creature spells of the chosen type cost {2} less to cast.",
                true,
            ),
        ] {
            let filter = parsed_spell_cost_filter(line);
            assert!(filter.chosen_creature_type, "{line}: {filter:#?}");
            assert_eq!(
                filter.card_types.contains(&CardType::Creature),
                creature_only,
                "{line}: {filter:#?}"
            );
            if line.contains("you cast") {
                assert_eq!(filter.cast_by, Some(PlayerFilter::You), "{line}");
            }
        }
    }

    #[test]
    fn explicit_chosen_card_type_cost_filter_and_unrestricted_control_stay_distinct() {
        let chosen = parsed_spell_cost_filter(
            "Spells of the chosen card type you cast cost {1} less to cast.",
        );
        assert!(chosen.chosen_card_type, "{chosen:#?}");
        assert_eq!(chosen.cast_by, Some(PlayerFilter::You));

        let unrestricted =
            parsed_spell_cost_filter("Creature spells you cast cost {1} less to cast.");
        assert!(!unrestricted.chosen_creature_type, "{unrestricted:#?}");
        assert!(!unrestricted.chosen_card_type, "{unrestricted:#?}");
        assert_eq!(unrestricted.card_types, vec![CardType::Creature]);
        assert_eq!(unrestricted.cast_by, Some(PlayerFilter::You));
    }

    #[test]
    fn permanent_spell_cost_filter_keeps_adventure_as_a_relative_characteristic() {
        let filter = parsed_spell_cost_filter(
            "Permanent spells you cast that have an Adventure cost {1} less to cast.",
        );

        assert!(filter.has_all_permanent_card_types(), "{filter:#?}");
        assert_eq!(filter.subtypes, [Subtype::Adventure]);
        assert_eq!(filter.cast_by, Some(PlayerFilter::You));

        let adventure_only =
            parsed_spell_cost_filter("Adventure spells you cast cost {1} less to cast.");
        assert!(adventure_only.card_types.is_empty(), "{adventure_only:#?}");
        assert_eq!(adventure_only.subtypes, [Subtype::Adventure]);
    }

    #[test]
    fn colored_spell_cost_modifiers_preserve_exact_per_target_scaling() {
        let reduction_tokens =
            lex_line("Spells you cast cost {W} less to cast for each target.", 0)
                .expect("colored reduction should lex");
        let reduction = parse_spells_cost_modifier_line(&reduction_tokens)
            .expect("colored reduction should not hard-error")
            .expect("colored reduction should parse");
        let reduction = match reduction.payload {
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction) => reduction,
            other => panic!("expected a mana-symbol reduction, got {other:?}"),
        };
        assert!(reduction.per_target);
        assert_eq!(reduction.cost.to_oracle(), "{W}");

        let increase_tokens = lex_line(
            "Spells your opponents cast cost {U} more to cast for each target.",
            0,
        )
        .expect("colored increase should lex");
        let increase = parse_spells_cost_modifier_line(&increase_tokens)
            .expect("colored increase should not hard-error")
            .expect("colored increase should parse");
        let increase = match increase.payload {
            ironsmith_core::StaticAbilityPayload::CostIncreaseManaCost(increase) => increase,
            other => panic!("expected a mana-symbol increase, got {other:?}"),
        };
        assert!(increase.per_target);
        assert_eq!(increase.cost.to_oracle(), "{U}");
    }

    #[test]
    fn non_hand_origin_filter_includes_cards_owned_by_another_player_in_hand() {
        let filter = parsed_spell_cost_filter(
            "Spells you cast from anywhere other than your hand cost {1} less to cast.",
        );

        assert!(filter.any_of.iter().any(|branch| {
            branch.zone == Some(Zone::Hand) && branch.owner == Some(PlayerFilter::NotYou)
        }));
    }

    #[test]
    fn extra_turn_skip_static_rule_preserves_player_scope_and_rejects_trigger_wording() {
        for (text, expected) in [
            (
                "If an opponent would begin an extra turn, that player skips that turn instead.",
                PlayerFilter::Opponent,
            ),
            (
                "If a player would begin an extra turn, that player skips that turn instead.",
                PlayerFilter::Any,
            ),
            (
                "If you would begin an extra turn, skip that turn instead.",
                PlayerFilter::You,
            ),
        ] {
            let tokens = lex_line(text, 0).expect("extra-turn skip line should lex");
            let ability = parse_players_skip_extra_turns_line(&tokens)
                .expect("extra-turn skip parser should not fail")
                .unwrap_or_else(|| {
                    panic!(
                        "exact extra-turn skip line should parse: {:?}",
                        crate::lexer::parser_token_word_refs(&tokens)
                    )
                });
            assert!(matches!(
                ability.payload,
                ironsmith_core::StaticAbilityPayload::PlayersSkipExtraTurns { player }
                    if player == expected
            ));
        }

        let near_miss = lex_line(
            "Whenever an opponent begins an extra turn, that player skips that turn.",
            0,
        )
        .expect("near-miss line should lex");
        assert!(
            parse_players_skip_extra_turns_line(&near_miss)
                .expect("near-miss parser should not fail")
                .is_none()
        );
    }

    #[test]
    fn cost_modifier_target_spec_preserves_player_or_controlled_permanent_union() {
        let tokens =
            lex_line("you or a permanent you control", 0).expect("cost-modifier target should lex");
        let (player, object, targets_any_of) =
            parse_cost_modifier_target_spec(&tokens).expect("cost-modifier target should parse");

        assert_eq!(player, Some(PlayerFilter::You));
        assert!(targets_any_of);
        let object = object.expect("permanent target branch should be retained");
        assert_eq!(object.zone, Some(Zone::Battlefield));
        assert_eq!(object.controller, Some(PlayerFilter::You));
    }

    #[test]
    fn fixed_reduction_with_as_long_as_draw_threshold_keeps_amount_and_condition_distinct() {
        let tokens = lex_line(
            "This spell costs {2} less to cast as long as you've drawn two or more cards this turn.",
            0,
        )
        .expect("conditional fixed reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("conditional reduction should not error")
            .unwrap_or_else(|| {
                panic!(
                    "conditional reduction should parse: {:?}",
                    crate::lexer::parser_token_word_refs(&tokens)
                )
            });
        let ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(reduction) =
            ability.payload
        else {
            panic!("expected a typed self reduction: {ability:#?}");
        };
        assert!(matches!(reduction.amount.unhinted(), Value::Fixed(2)));
        let crate::static_abilities::ThisSpellCostCondition::AsLongAsConditionExpr {
            condition,
            display,
        } = reduction.condition
        else {
            panic!("expected an as-long-as condition: {reduction:#?}");
        };
        assert_eq!(display, "you've drawn two or more cards this turn");
        assert!(matches!(
            condition,
            crate::ConditionExpr::ValueComparison {
                left,
                operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
                right,
            } if matches!(left.unhinted(), Value::MaxCardsDrawnThisTurn(PlayerFilter::You))
                && matches!(right.unhinted(), Value::Fixed(2))
        ));

        let ordinary = lex_line(
            "This spell costs {2} less to cast if you've drawn two or more cards this turn.",
            0,
        )
        .expect("ordinary conditional should lex");
        let ordinary = parse_spells_cost_modifier_line(&ordinary)
            .expect("ordinary condition should not error")
            .expect("ordinary condition should parse");
        assert!(matches!(
            ordinary.payload,
            ironsmith_core::StaticAbilityPayload::ThisSpellCostReduction(
                ironsmith_core::ThisSpellCostReduction {
                    condition: crate::static_abilities::ThisSpellCostCondition::ConditionExpr { .. },
                    ..
                }
            )
        ));
    }

    #[test]
    fn first_matching_spell_during_each_own_turn_is_typed_and_conditioned() {
        let tokens = lex_line(
            "The first non-Lemur creature spell with flying you cast during each of your turns costs {1} less to cast.",
            0,
        )
        .expect("first-spell reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("first-spell reduction should not error")
            .expect("first-spell reduction should parse");
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &ability.payload
        else {
            panic!("expected typed cost reduction: {ability:#?}");
        };
        assert!(
            reduction.filter.first_spell_cast_each_turn,
            "{reduction:#?}"
        );
        assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
        assert_eq!(reduction.filter.card_types, [CardType::Creature]);
        assert_eq!(reduction.filter.excluded_subtypes, [Subtype::Lemur]);
        assert_eq!(reduction.filter.static_abilities, [StaticAbilityId::Flying]);
        assert_eq!(reduction.condition, Some(crate::ConditionExpr::YourTurn));
    }

    #[test]
    fn second_spell_each_turn_cost_reduction_keeps_exact_ordinal() {
        let tokens = lex_line(
            "The second spell you cast each turn costs {2} less to cast.",
            0,
        )
        .expect("second-spell reduction should lex");
        let ability = parse_spells_cost_modifier_line(&tokens)
            .expect("second-spell reduction should not error")
            .expect("second-spell reduction should parse");
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &ability.payload
        else {
            panic!("expected typed cost reduction: {ability:#?}");
        };
        assert_eq!(reduction.filter.spell_cast_ordinal_each_turn, Some(2));
        assert!(!reduction.filter.first_spell_cast_each_turn);
        assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
    }

    #[test]
    fn first_spell_cost_reduction_and_flash_share_one_typed_filter() {
        let tokens = lex_line(
            "The first creature spell you cast each turn costs {2} less to cast and can be cast as though it had flash.",
            0,
        )
        .expect("compound first-spell line should lex");
        let parsed = parse_first_spell_cost_reduction_and_flash_line(&tokens)
            .expect("compound first-spell line should not error")
            .expect("compound first-spell line should parse");
        let [
            StaticAbilityAst::Static(reduction),
            StaticAbilityAst::Static(flash),
        ] = parsed.as_slice()
        else {
            panic!("expected two typed static capabilities: {parsed:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::CostReduction(reduction) = &reduction.payload
        else {
            panic!("expected cost reduction: {reduction:#?}");
        };
        let ironsmith_core::StaticAbilityPayload::Grants(flash) = &flash.payload else {
            panic!("expected hand-zone flash grant: {flash:#?}");
        };
        assert!(reduction.filter.first_spell_cast_each_turn);
        assert_eq!(reduction.filter.cast_by, Some(PlayerFilter::You));
        assert_eq!(flash.filter, reduction.filter);
        assert_eq!(flash.zone, Zone::Hand);

        let near_miss = lex_line(
            "The first creature spell you cast each turn costs {2} less to cast and has flash.",
            0,
        )
        .expect("near miss should lex");
        assert!(
            parse_first_spell_cost_reduction_and_flash_line(&near_miss)
                .expect("near miss should not error")
                .is_none(),
            "a granted keyword is not the authored cast-as-though permission"
        );
    }

    #[test]
    fn would_die_replacement_keeps_filtered_damage_source_history() {
        let tokens = lex_line(
            "If a creature dealt damage this turn by a source you controlled would die, exile it instead.",
            0,
        )
        .expect("filtered-damager replacement should lex");
        let ability = parse_exile_would_die_instead_line(&tokens)
            .expect("filtered-damager replacement should not error")
            .expect("filtered-damager replacement should parse");
        let ironsmith_core::StaticAbilityPayload::ExileWouldDieInstead {
            filter,
            damaged_by,
            damager_filter,
            damager_filter_surface,
            ..
        } = &ability.payload
        else {
            panic!("expected typed would-die replacement: {ability:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(*damaged_by, None);
        assert_eq!(
            damager_filter
                .as_ref()
                .and_then(|filter| filter.controller.clone()),
            Some(PlayerFilter::You)
        );
        assert_eq!(
            damager_filter_surface.as_deref(),
            Some("a source you controlled")
        );
    }
}

//! Sentence readings 67–88, in rank order.

use super::super::*;
use super::Sentence;

pub(super) fn read_for_each_counter_removed(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // Counter-result clauses also have the generic surface shape
    // `for each <noun phrase>, <effect>`. Route their typed grammar shapes
    // first so `counter(s) removed this way` is not treated as an object
    // filter or target phrase.
    if let Some(effect) = parse_for_each_counter_removed_sentence(tokens)? {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_for_each_counter_group_removed_this_way(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) =
            super::super::super::super::clause_dispatch::parse_for_each_counter_group_removed_this_way_clause(tokens)?
        {
            return Ok(Some(vec![effect]));
        }
    Ok(None)
}
pub(super) fn read_for_each_prevent_damage(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effect) =
        super::super::super::super::clause_dispatch::parse_for_each_prevent_damage_clause(tokens)?
    {
        return Ok(Some(vec![effect]));
    }
    Ok(None)
}
pub(super) fn read_for_each_destroyed_this_way(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::search_library::parse_for_each_destroyed_this_way_sentence(
            tokens,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_for_each_sacrificed_this_way(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::search_library::parse_for_each_sacrificed_this_way_sentence(
            tokens,
        )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_for_each_put_into_graveyard_this_way(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
            super::super::super::super::search_library::parse_for_each_put_into_graveyard_this_way_sentence(tokens)?
        {
            return Ok(Some(effects));
        }
    Ok(None)
}
pub(super) fn read_for_each_exiled_this_way(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) =
        super::super::super::super::search_library::parse_for_each_exiled_this_way_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_each_chosen_player_search_put_top(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    // This typed search sequence contains an internal `then` chain. Route it
    // before the generic object iterator can interpret "each of them" as an
    // object filter and detach the final put-on-top clause.
    if effect_grammar::parse_each_chosen_player_search_put_top_shape(tokens).is_some()
        && let Some(effects) = parse_search_library_sentence_lexed(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_for_each_mana_symbol_spent_effect(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_mana_symbol_spent_effect_shape(tokens)
    {
        let base = Value::ManaSymbolSpentToCastThisSpell {
            symbol: shape.symbol,
            reference: shape.reference,
        };
        let count = if shape.group_size == 1 {
            base
        } else {
            Value::DividedRoundedDown(Box::new(base), shape.group_size as i32)
        }
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each mana-symbol clause has no effect payload".to_string(),
            ))
            .map(Some);
        }
        return Ok(Some(vec![EffectAst::RepeatEffects { count, effects }]));
    }
    Ok(None)
}
pub(super) fn read_for_each_spent_mana_effect(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_spent_mana_effect_shape(tokens)
    {
        let source_words = crate::lexer::token_word_refs(shape.source_tokens);
        let count = crate::grammar::shared_util::count_shapes::mana_from_source_spent_to_cast_value_with_reference(
                &source_words,
                shape.reference,
            )
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported for-each spent-mana source (source: '{}')",
                    render_token_slice(shape.source_tokens).trim()
                ))
            })?
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "for-each spent-mana clause has no effect payload (effect: '{}')",
                render_token_slice(shape.effect_tokens).trim()
            )))
            .map(Some);
        }
        return Ok(Some(vec![EffectAst::RepeatEffects { count, effects }]));
    }
    Ok(None)
}
pub(super) fn read_for_each_object_effect(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) = effect_grammar::for_each_shapes::parse_for_each_object_effect_shape(tokens)
    {
        let mut count_words = vec!["for", "each"];
        count_words.extend(crate::lexer::token_word_refs(shape.filter_tokens));
        if let Some((count, used)) = crate::util::parse_for_each_count_value_words(&count_words)
            && used == count_words.len()
            && !matches!(count.unhinted(), Value::Count(_))
        {
            let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "for-each scalar sentence missing effect payload".to_string(),
                ))
                .map(Some);
            }
            return Ok(Some(vec![EffectAst::RepeatEffects {
                count: count.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
                effects,
            }]));
        }
    }
    Ok(None)
}
pub(super) fn read_for_each_dynamic_target_effect(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) =
        effect_grammar::for_each_shapes::parse_for_each_dynamic_target_effect_shape(tokens)
    {
        let mut filter = parse_object_filter_lexed(shape.filter_tokens, false)?;
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each dynamic target sentence missing effect payload".to_string(),
            ))
            .map(Some);
        }
        let tag = crate::tag::CompilerReferenceTag::It.bind();
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::dynamic_x(),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ForEachTagged { tag, effects },
        ]));
    }
    Ok(None)
}
pub(super) fn read_for_each_object_filter_effect(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(shape) = effect_grammar::for_each_shapes::parse_for_each_object_effect_shape(tokens)
    {
        let filter = super::super::super::super::for_each_helpers::parse_for_each_object_filter(
            shape.filter_tokens,
        )?;
        let effects = parse_effect_sentence_lexed(shape.effect_tokens)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(
                "for-each object sentence missing effect payload".to_string(),
            ))
            .map(Some);
        }
        return Ok(Some(vec![EffectAst::ForEachObject { filter, effects }]));
    }
    Ok(None)
}
pub(super) fn read_delayed_until_next_end_step(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let delayed_shape = sentence_shapes::parse_delayed_sentence_tokens(tokens);
    if matches!(
        delayed_shape,
        Some(sentence_shapes::DelayedSentenceShape::NextEndStep)
    ) && let Some(effects) = parse_delayed_until_next_end_step_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_delayed_next_combat_phase_this_turn(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let delayed_shape = sentence_shapes::parse_delayed_sentence_tokens(tokens);
    if matches!(
        delayed_shape,
        Some(sentence_shapes::DelayedSentenceShape::NextCombat)
    ) && let Some(effects) = parse_delayed_next_combat_phase_this_turn_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_it_is_aura_enchantment_sentence(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if let Some(effects) = parse_it_is_aura_enchantment_sentence_lexed(tokens)? {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_quoted_ability_shared_color_fanout(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let quoted_ability_shape = sentence_shapes::parse_quoted_ability_sentence_tokens(tokens);
    if quoted_ability_shape.is_some()
        && let Some(effects) =
            super::super::super::super::fanout_family::parse_shared_color_target_fanout_sentence(
                tokens,
            )?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_quoted_ability_leading_may(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let quoted_ability_shape = sentence_shapes::parse_quoted_ability_sentence_tokens(tokens);
    // Preserve the chooser on optional quoted restrictions. The broad quoted
    // grant parser can otherwise consume the whole sentence before the chain
    // parser turns the leading "you may have" into a MayByPlayer node.
    if quoted_ability_shape.is_some()
        && super::super::super::super::parse_leading_player_may_lexed(tokens).is_some()
    {
        return super::super::super::super::parse_effect_chain_lexed(tokens).map(Some);
    }
    Ok(None)
}
pub(super) fn read_quoted_ability_conditional(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let quoted_ability_shape = sentence_shapes::parse_quoted_ability_sentence_tokens(tokens);
    let quoted_animation_grant = tokens
        .iter()
        .filter(|token| token.kind == crate::lexer::TokenKind::Quote)
        .count()
        >= 2
        && tokens.iter().any(|token| token.is_word("becomes"))
        && tokens.iter().any(|token| token.is_word("gains"));
    // A leading conditional owns the whole sentence. Do not let a quoted
    // ability's inner verbs make the broad gain parser consume the unsplit
    // condition and body; the conditional route below parses the body with
    // this same gain parser after removing the predicate.
    if (quoted_ability_shape.is_some() || quoted_animation_grant)
        && !matches!(
            sentence_shapes::parse_leading_if_sentence_tokens(tokens),
            Some(sentence_shapes::LeadingIfSentenceShape { replacement: false })
        )
        && let Some(effects) =
            super::super::super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_source_tapped_gain_duration(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if effect_grammar::gain_ability_shapes::parse_source_tapped_gain_duration_shape(tokens)
        .is_some()
        && let Some(effects) =
            super::super::super::super::gain_ability::parse_gain_ability_sentence(tokens)?
    {
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_immediate_sacrifice_sentence(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    if sentence_shapes::parse_immediate_sacrifice_sentence_tokens(tokens).is_some() {
        let mut effects = super::super::super::super::parse_effect_chain_inner_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(Some(effects));
    }
    Ok(None)
}
pub(super) fn read_end_of_combat_remainder(
    input: &Sentence<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = input.tokens;
    let delayed_shape = sentence_shapes::parse_delayed_sentence_tokens(tokens);
    if let Some(sentence_shapes::DelayedSentenceShape::EndOfCombat { remainder_tokens }) =
        delayed_shape
    {
        let remainder = trim_commas(remainder_tokens);
        if remainder.is_empty() {
            return Err(CardTextError::ParseError(
                "end-of-combat delayed trigger missing effect payload".to_string(),
            ))
            .map(Some);
        }
        let effects = parse_effect_sentence_lexed_inner(&remainder)?;
        return Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat { effects }]));
    }
    Ok(None)
}

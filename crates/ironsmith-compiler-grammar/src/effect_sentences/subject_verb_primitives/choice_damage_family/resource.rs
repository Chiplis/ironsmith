use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::LifeResourceActionAst;
use super::*;

pub fn parse_sentence_unless_pays(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // This causative alternative is an action choice, not a payment. Keep it
    // ahead of the broad unless parser even when a caller reaches this rule
    // through a generic conditional-dispatch path.
    if let Some(effects) = parse_sentence_damage_unless_controller_has_source_deal_damage(clause)? {
        return Ok(Some(effects));
    }
    let Some(shape) = choice_shapes::parse_unless_sentence_shape(clause.tokens()) else {
        return Ok(None);
    };
    let unless_idx = shape.unless_token;

    if unless_idx == 0 {
        let Some((unless_clause, effect_clause)) = clause.split_once_on_comma() else {
            return Ok(None);
        };
        if effect_clause.is_empty() {
            return Ok(None);
        }

        let effects = parse_effect_chain(effect_clause.tokens())?;
        if effects.is_empty() {
            return Ok(None);
        }

        if let Some(unless_effect) = try_build_unless(effects, unless_clause, 0)? {
            return Ok(Some(vec![unless_effect]));
        }
        return Ok(None);
    }

    let before_unless_clause = SubjectVerbPrimitiveClause::new(shape.action_tokens);
    let before_words = before_unless_clause.word_refs();

    if choice_shapes::first_choice_damage_word_is(&before_words, "counter") {
        return Ok(None);
    }
    if choice_shapes::is_create_token_sacrifice_counter_shape(&before_unless_clause.word_refs()) {
        return Ok(None);
    }

    // In `A, then B unless you pay C`, only the final action B is replaced
    // by the payment. Parsing the entire prefix as the UnlessPays body both
    // weakens the temporal boundary and lets a prefix-tolerant parser claim A
    // while silently dropping B. Split only on the grammar-proven comma/then
    // boundary, retain every earlier action, and wrap the final action in the
    // payment choice.
    let comma_then_segments =
        super::super::super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![
            shape.action_tokens,
        ]);
    if comma_then_segments.len() > 1 {
        let (last, leading) = comma_then_segments
            .split_last()
            .expect("comma/then split has at least two segments");
        let mut effects = Vec::new();
        for segment in leading {
            effects.extend(parse_effect_chain(segment)?);
        }
        let final_effects = parse_effect_chain(last)?;
        if effects.is_empty() || final_effects.is_empty() {
            return Ok(None);
        }
        let Some(unless_effect) = try_build_unless(final_effects, clause, unless_idx)? else {
            return Ok(None);
        };
        effects.push(unless_effect);
        return Ok(Some(effects));
    }

    let sentence_words = clause.word_refs();
    if let Some(special) =
        choice_shapes::parse_each_opponent_return_unless_draw_shape(&sentence_words)
    {
        let Some(target_clause) = clause
            .after_words(special.target_start_word)
            .and_then(|tail| {
                tail.before_word(
                    special
                        .target_end_word
                        .saturating_sub(special.target_start_word),
                )
            })
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
                    effects: vec![EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                        false,
                    )],
                    alternative: vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                            count: Value::Fixed(1),
                        }),
                    )],
                    player: PlayerAst::ItsController,
                }),
            ],
        })]));
    }

    let each_prefix = choice_shapes::parse_choice_damage_scope(&before_unless_clause.word_refs());
    if let Some(prefix_kind) = each_prefix {
        let inner_clause = before_unless_clause
            .after_words(2)
            .unwrap_or_else(|| before_unless_clause.from(2));
        if let Ok(inner_effects) = parse_effect_chain(inner_clause.tokens())
            && !inner_effects.is_empty()
            && let Some(unless_effect) = try_build_unless(inner_effects, clause, unless_idx)?
        {
            let wrapper = match prefix_kind {
                choice_shapes::ChoiceDamageScope::Opponent => EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
                    effects: vec![unless_effect],
                }),
                choice_shapes::ChoiceDamageScope::Player => EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                    effects: vec![unless_effect],
                }),
            };
            return Ok(Some(vec![wrapper]));
        }
        return Ok(None);
    }

    let effect_clause = before_unless_clause;
    if let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(effect_clause)
    {
        let Some(delayed_effect_clause) = effect_clause
            .before_word(timing_start_word)
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        if delayed_effect_clause.is_empty() {
            return Ok(None);
        }
        let delayed_effects = parse_effect_chain(delayed_effect_clause.tokens())?;
        if delayed_effects.is_empty() {
            return Ok(None);
        }
        if let Some(unless_effect) = try_build_unless(delayed_effects, clause, unless_idx)? {
            return Ok(Some(vec![wrap_delayed_next_step_unless_pays(
                step,
                player,
                vec![unless_effect],
            )]));
        }
    }

    let effects = parse_effect_chain(effect_clause.tokens())?;
    if effects.is_empty() {
        return Ok(None);
    }

    if let Some(unless_effect) = try_build_unless(effects, clause, unless_idx)? {
        return Ok(Some(vec![unless_effect]));
    }
    Ok(None)
}

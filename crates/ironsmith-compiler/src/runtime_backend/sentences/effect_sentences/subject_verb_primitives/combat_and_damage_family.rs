use super::*;
use crate::runtime_backend::front_end::grammar::effects::combat_damage_family_shapes as combat_shapes;
use crate::runtime_backend::front_end::grammar::effects::delayed_step_shapes as delayed_shapes;
const TRANSFORM_WORD: &str = "transform";
const CONVERT_WORD: &str = "convert";
const DISTRIBUTE_WORD: &str = "distribute";

fn token_is_word(token: &OwnedLexToken, expected: &str) -> bool {
    token.as_word() == Some(expected)
}

pub(crate) fn parse_sentence_destroy_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if combat_shapes::parse_destroy_creature_type_choice_shape(clause.tokens()).is_none() {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_destroy_all(ObjectFilter::creature().of_chosen_creature_type()),
    ]))
}

pub(crate) fn parse_sentence_pump_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_pump_creature_type_choice_shape(clause.tokens()) else {
        return Ok(None);
    };
    if !shape.trailing_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            clause.text()
        )));
    }
    let trimmed_subject_clause =
        SubjectVerbPrimitiveClause::new(shape.base_subject_tokens).trimmed();
    if trimmed_subject_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }
    let get_tail_clause = SubjectVerbPrimitiveClause::new(shape.get_tail_tokens).trimmed();

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_clause =
        SubjectVerbPrimitiveOwnedClause::from_clause(trimmed_subject_clause);
    gain_candidate_clause.append_clause(get_tail_clause);
    if let Some(mut gain_effects) = parse_gain_ability_sentence(gain_candidate_clause.tokens())? {
        let mut patched = false;
        for effect in &mut gain_effects {
            match effect {
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::PumpAll { filter, .. }
                        | SubjectVerbActionAst::GrantAbilitiesAll { filter, .. }
                        | SubjectVerbActionAst::GrantAbilitiesChoiceAll { filter, .. },
                    ..
                }) => {
                    filter.chosen_creature_type = true;
                    patched = true;
                }
                _ => {}
            }
        }
        if patched {
            let mut effects = vec![EffectAst::subject_verb_choose_creature_type(
                PlayerAst::You,
                vec![],
            )];
            effects.extend(gain_effects);
            return Ok(Some(effects));
        }
    }

    let filter_clause = SubjectVerbPrimitiveClause::new(shape.filter_subject_tokens).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }

    let modifier = get_tail_clause
        .token(1)
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                clause.text()
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            clause.text()
        ))
    })?;
    let (power, toughness, duration, condition) = parse_get_modifier_values_with_tail(
        get_tail_clause.from(1).tokens(),
        base_power,
        base_toughness,
    )?;
    if condition.is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gets duration in creature-type choice pump clause (clause: '{}')",
            clause.text()
        )));
    }

    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_pump_all(filter, power, toughness, duration),
    ]))
}

pub(crate) fn parse_sentence_must_attack_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_must_attack_creature_type_choice_shape(clause.tokens())
    else {
        return Ok(None);
    };
    use crate::effect::Until;

    if !shape.trailing_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }
    let filter_clause = SubjectVerbPrimitiveClause::new(shape.filter_subject_tokens).trimmed();
    if filter_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice attack clause (clause: '{}')",
            clause.text()
        )));
    }

    let mut filter = parse_object_filter(filter_clause.tokens(), false)?;
    if !iter_contains(filter.card_types.iter(), &CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice attack subject must be creature-based (clause: '{}')",
            clause.text()
        )));
    }
    filter.chosen_creature_type = true;

    Ok(Some(vec![
        EffectAst::subject_verb_choose_creature_type(PlayerAst::You, vec![]),
        EffectAst::subject_verb_grant_abilities_all(
            filter,
            vec![crate::runtime_backend::GrantedAbilityAst::MustAttack],
            Until::EndOfTurn,
        ),
    ]))
}

pub(crate) fn parse_sentence_put_sticker_on(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_put_sticker_shape(clause.tokens()) else {
        return Ok(None);
    };
    let target_clause = SubjectVerbPrimitiveClause::new(shape.target_tokens).trimmed();

    if let Some((sticker_target, aura_target, attachment_filter)) =
        parse_put_sticker_then_becomes_aura(target_clause)?
    {
        return Ok(Some(vec![
            EffectAst::subject_verb_put_sticker(sticker_target, shape.action),
            EffectAst::subject_verb_become_aura_enchantment(
                aura_target,
                attachment_filter,
                crate::effect::Until::Forever,
            ),
        ]));
    }

    if shape.target_is_reference {
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![EffectAst::subject_verb_put_sticker(
            target,
            shape.action,
        )]));
    }

    let mut filter = parse_object_filter(target_clause.tokens(), false)?;
    if filter.zone.is_none() {
        filter.zone = Some(crate::zone::Zone::Battlefield);
    }
    Ok(Some(vec![EffectAst::subject_verb_put_sticker(
        TargetAst::Object(filter, None, None),
        shape.action,
    )]))
}

fn parse_put_sticker_then_becomes_aura(
    target_clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<(TargetAst, TargetAst, ObjectFilter)>, CardTextError> {
    let Some(shape) = combat_shapes::parse_sticker_aura_shape(target_clause.tokens()) else {
        return Ok(None);
    };

    Ok(Some((
        parse_target_phrase(shape.sticker_target_tokens)?,
        TargetAst::Tagged(
            TagKey::from(IT_TAG),
            span_from_tokens(shape.sticker_target_tokens),
        ),
        parse_object_filter(shape.enchant_filter_tokens, false)?,
    )))
}

pub(crate) fn parse_sentence_return_targets_of_creature_type_of_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_return_creature_type_choice_shape(clause.tokens())
    else {
        return Ok(None);
    };
    if shape.base_target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before chosen-type qualifier (clause: '{}')",
            clause.text()
        )));
    }
    let mut filter = parse_object_filter(&shape.base_target_tokens, false)?;
    if shape.excluded {
        filter.excluded_chosen_creature_type = true;
    } else {
        filter.chosen_creature_type = true;
    }

    let mut effects = Vec::new();
    if shape.needs_inline_choice_effect {
        effects.push(EffectAst::subject_verb_choose_creature_type(
            PlayerAst::You,
            vec![],
        ));
    }

    if shape.has_explicit_target {
        let mut target = parse_target_phrase(&shape.base_target_tokens)?;
        // Recursively patch `chosen_creature_type` / `excluded_chosen_creature_type`
        // on the ObjectFilter buried inside the TargetAst (may be wrapped in WithCount).
        fn patch_chosen_type(t: &mut TargetAst, chosen: bool, excluded: bool) {
            match t {
                TargetAst::Object(f, _, _) => {
                    f.chosen_creature_type |= chosen;
                    f.excluded_chosen_creature_type |= excluded;
                }
                TargetAst::WithCount(inner, _) => patch_chosen_type(inner, chosen, excluded),
                _ => {}
            }
        }
        patch_chosen_type(
            &mut target,
            filter.chosen_creature_type,
            filter.excluded_chosen_creature_type,
        );
        effects.push(EffectAst::subject_verb_return_to_hand(target, false));
    } else {
        effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_choose_all_from_battlefield_and_graveyard_to_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_text = clause.text();
    let Some(shape) = combat_shapes::parse_choose_all_zones_to_hand_shape(clause.tokens()) else {
        return Ok(None);
    };
    let mut base_filter = parse_object_filter(shape.filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported object filter in choose-all battlefield/graveyard clause (clause: '{}')",
            clause_text
        ))
    })?;
    base_filter.controller = None;

    let mut battlefield_filter = base_filter.clone();
    battlefield_filter.zone = Some(shape.zones[0]);

    let mut graveyard_filter = base_filter;
    graveyard_filter.zone = Some(shape.zones[1]);

    Ok(Some(vec![
        EffectAst::subject_verb_return_all_to_hand(battlefield_filter),
        EffectAst::subject_verb_return_all_to_hand(graveyard_filter),
    ]))
}

pub(crate) fn parse_sentence_return_multiple_targets(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_return_multiple_targets_shape(clause.tokens()) else {
        return Ok(None);
    };
    let targets_clause = SubjectVerbPrimitiveClause::new(shape.targets_tokens).trimmed();
    let destination = shape.destination;

    let mut segments: Vec<SubjectVerbPrimitiveOwnedClause> = Vec::new();
    for segment_clause in targets_clause.trimmed_and_comma_segments() {
        let facts = combat_shapes::parse_return_segment_facts(segment_clause.tokens());
        if !segments.is_empty()
            && !facts.starts_new_target
            && !facts.mentions_target
            && !facts.starts_like_zone_suffix
        {
            let last = segments.last_mut().expect("segments is non-empty");
            last.append_comma_then(segment_clause);
        } else {
            segments.push(SubjectVerbPrimitiveOwnedClause::from_clause(segment_clause));
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(|segment| combat_shapes::parse_return_segment_facts(segment.tokens()).quantifier);

    let shared_suffix = segments
        .last()
        .and_then(|segment| combat_shapes::return_zone_suffix_tokens(segment.tokens()))
        .map(<[OwnedLexToken]>::to_vec)
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        let mut facts = combat_shapes::parse_return_segment_facts(segment.tokens());
        if !facts.mentions_zone && !shared_suffix.is_empty() {
            segment.extend_from_slice(&shared_suffix);
            facts = combat_shapes::parse_return_segment_facts(segment.tokens());
        }
        if let Some(quantifier) = shared_quantifier {
            if facts.quantifier.is_none()
                && !facts.starts_like_target_reference
                && !facts.mentions_target
            {
                segment.insert_leading_word(quantifier.as_str());
                facts = combat_shapes::parse_return_segment_facts(segment.tokens());
            }
        }
        if facts.quantifier.is_some() {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    clause.text()
                )));
            }
            let filter = parse_object_filter(segment.from_tokens(1), false)?;
            if destination.zone == Zone::Battlefield {
                effects.push(EffectAst::subject_verb_return_all_to_battlefield(
                    filter,
                    destination.tapped,
                    false,
                    ReturnControllerAst::Owner,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_all_to_hand(filter));
            }
        } else {
            let target = parse_target_phrase(segment.tokens())?;
            if destination.zone == Zone::Battlefield {
                effects.push(EffectAst::subject_verb_return_to_battlefield(
                    target,
                    destination.tapped,
                    false,
                    false,
                    ReturnControllerAst::Preserve,
                    None,
                ));
            } else {
                effects.push(EffectAst::subject_verb_return_to_hand(target, false));
            }
        }
    }

    Ok(Some(vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
    }]))
}

pub(crate) fn parse_sentence_for_each_of_target_objects(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_target_objects_shape(clause.tokens()) else {
        return Ok(None);
    };

    let subject_clause = SubjectVerbPrimitiveClause::new(shape.subject_tokens).trimmed();
    let Some((mut filter, count)) =
        parse_for_each_targeted_object_subject(subject_clause.tokens())?
    else {
        return Ok(None);
    };
    if filter.zone == Some(Zone::Battlefield)
        && filter.controller.is_none()
        && filter.tagged_constraints.is_empty()
    {
        // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects
        // compilation for plain "target permanent(s)" clauses.
        filter.controller = Some(PlayerFilter::Any);
    }

    let effect_clause = SubjectVerbPrimitiveClause::new(shape.effect_tokens).trimmed();
    if effect_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after for-each target subject (clause: '{}')",
            clause.text()
        )));
    }
    let mut per_target_effects = parse_effect_chain(effect_clause.tokens())?;
    for effect in &mut per_target_effects {
        bind_implicit_player_context(effect, PlayerAst::You);
    }
    if per_target_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "for-each target follow-up produced no effects (clause: '{}')",
            clause.text()
        )));
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player: PlayerAst::Implicit,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: per_target_effects,
        },
    ]))
}

pub(crate) fn parse_distribute_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<EffectAst>, CardTextError> {
    if !clause
        .first_word()
        .is_some_and(|word| word == DISTRIBUTE_WORD)
    {
        return Ok(None);
    }

    let amount_clause = clause.from(1);
    let (count, used) = parse_value(amount_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed counter amount (clause: '{}')",
            clause.text()
        ))
    })?;
    let rest_clause = clause.from(1 + used);
    let counter_type = parse_counter_type_from_tokens(rest_clause.tokens()).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause.text()
        ))
    })?;
    let Some((_before_among, target_clause)) = rest_clause.split_once_on_word("among") else {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target clause after 'among' (clause: '{}')",
            clause.text()
        )));
    };
    let target_clause = target_clause.trimmed();
    if target_clause.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause.text()
        )));
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(target_clause.tokens())?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause.text()
            ))
        })?;
    let target_phrase = target_clause.from(used_count).trimmed();
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause.text()
        )));
    }
    let target = parse_target_phrase(target_phrase.tokens())?;

    Ok(Some(EffectAst::subject_verb_put_counters(
        counter_type,
        count,
        target,
        Some(target_count),
        true,
    )))
}

pub(crate) fn parse_sentence_distribute_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let Some(primary) = parse_distribute_counters_sentence(head_clause)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if let Some(tail_clause) = tail_clause
        && !tail_clause.is_empty()
    {
        effects.extend(parse_effect_chain(tail_clause.tokens())?);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_sentence_transform_with_followup(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = clause.token(0) else {
        return Ok(None);
    };
    let is_transform = token_is_word(first, TRANSFORM_WORD);
    let is_convert = token_is_word(first, CONVERT_WORD);
    if !is_transform && !is_convert {
        return Ok(None);
    }

    let (head_clause, tail_clause) = if let Some((head, tail)) = clause.split_once_on_then_trimmed()
    {
        (head, Some(tail))
    } else {
        (clause, None)
    };

    let target_clause = head_clause.from(1).trimmed();
    let transform = if is_transform {
        parse_transform(target_clause.tokens())?
    } else {
        parse_convert(target_clause.tokens())?
    };
    let Some(tail_clause) = tail_clause else {
        return Ok(Some(vec![transform]));
    };
    if tail_clause.is_empty() {
        return Ok(Some(vec![transform]));
    }

    let mut effects = vec![transform];
    effects.extend(parse_effect_chain(tail_clause.tokens())?);
    Ok(Some(effects))
}

pub(crate) fn parse_sentence_cant_effect(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_cant_effect_sentence)
}

pub(crate) fn parse_sentence_gain_x_plus_life(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_x_plus_life_sentence)
}

pub(crate) fn parse_sentence_for_each_exiled_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_this_way_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.subject_tokens.is_empty() || shape.effect_tokens.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_exiled_this_way_sentence)
}

pub(crate) fn parse_sentence_for_each_put_into_graveyard_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_this_way_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.subject_tokens.is_empty() || shape.effect_tokens.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_put_into_graveyard_this_way_sentence)
}

pub(crate) fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_each_player_put_permanent_cards_exiled_with_source_sentence)
}

pub(crate) fn parse_sentence_for_each_destroyed_this_way(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = combat_shapes::parse_for_each_this_way_shape(clause.tokens()) else {
        return Ok(None);
    };
    if shape.subject_tokens.is_empty() || shape.effect_tokens.is_empty() {
        return Ok(None);
    }
    clause.parse_with_lexed(parse_for_each_destroyed_this_way_sentence)
}

pub(crate) fn parse_sentence_search_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_search_library_sentence)
}

pub(crate) fn parse_sentence_shuffle_graveyard_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_graveyard_into_library_sentence)
}

pub(crate) fn parse_sentence_shuffle_object_into_library(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shuffle_object_into_library_sentence)
}

pub(crate) fn parse_sentence_exile_hand_and_graveyard_bundle(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_exile_hand_and_graveyard_bundle_sentence)
}

pub(crate) fn parse_sentence_target_player_exiles_creature_and_graveyard(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_target_player_exiles_creature_and_graveyard_sentence)
}

pub(crate) fn parse_sentence_look_at_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_hand_sentence)
}

pub(crate) fn parse_sentence_look_at_top_then_exile_one(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_look_at_top_then_exile_one_sentence)
}

pub(crate) fn parse_sentence_gain_life_equal_to_age(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_gain_life_equal_to_age_sentence)
}

pub(crate) fn parse_sentence_for_each_player_doesnt(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_one_with_lexed(parse_for_each_player_doesnt)
}

pub(super) use delayed_shapes::DelayedTimingStepShape as DelayedNextStepKind;

pub(super) fn delayed_next_step_marker(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Option<(usize, usize, DelayedNextStepKind, PlayerAst)> {
    delayed_shapes::parse_delayed_timing_marker_shape(clause.tokens())
        .map(|shape| (shape.start_word, shape.end_word, shape.step, shape.player))
}

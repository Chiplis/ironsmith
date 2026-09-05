use crate::cards::builders::{
    CardTextError, EffectAst, EffectLoweringContext, PlayerAst, PredicateAst, SubjectVerbActionAst,
    TagKey, TargetAst,
};
use crate::effect::{ChoiceCount, Condition, Effect, EffectPredicate, SearchSelectionMode, Value};
use crate::filter::ObjectRef;
use crate::model::visit::{for_each_nested_effects, for_each_nested_effects_mut};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};

use super::{
    AnnotatedEffect, AnnotatedEffectSequence, EffectPreludeTag, LoweredEffects,
    PreparedEffectsForLowering, PreparedPredicateForLowering, PreparedTriggeredEffectsForLowering,
    ReferenceEnv, ReferenceExports, ReferenceImports, compile_annotated_effects_with_context,
    compile_condition_from_predicate_ast, effects_reference_tag, merge_compiled_choices,
    push_choice, stage_effects_for_lowering,
};

#[cfg(any(test, feature = "test-support"))]
pub fn compile_statement_effects(effects: &[EffectAst]) -> Result<Vec<Effect>, CardTextError> {
    Ok(
        compile_statement_effects_with_imports(effects, &ReferenceImports::default())?
            .effects
            .to_vec(),
    )
}

pub fn compile_statement_effects_with_imports(
    effects: &[EffectAst],
    imports: &ReferenceImports,
) -> Result<LoweredEffects, CardTextError> {
    let prepared = stage_effects_for_lowering(effects, imports.clone())?;
    materialize_prepared_statement_effects(&prepared)
}

pub fn materialize_prepared_statement_effects(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    if let Some(mut lowered) = materialize_trailing_self_replacement(prepared)? {
        dedupe_adjacent_target_only_effects(&mut lowered);
        return Ok(lowered);
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    if let Some((effects, choices)) = materialize_source_sentence_segments(prepared, &mut ctx)? {
        let final_env = ctx.reference_env();
        let mut lowered = LoweredEffects {
            effects,
            choices,
            exports: ReferenceExports::from_env(&final_env),
        };
        dedupe_adjacent_target_only_effects(&mut lowered);
        return Ok(lowered);
    }
    let (compiled, _) = compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = normalize_compiled_effects(compiled);
    let final_env = ctx.reference_env();
    let mut lowered = LoweredEffects {
        effects: crate::resolution::ResolutionProgram::from_effects(prepend_effect_prelude(
            compiled,
            compile_effect_prelude_tags(&prepared.prelude),
        )),
        choices: Vec::new(),
        exports: ReferenceExports::from_env(&final_env),
    };
    dedupe_adjacent_target_only_effects(&mut lowered);
    Ok(lowered)
}

pub fn materialize_prepared_effects_with_trigger_context(
    prepared: &PreparedEffectsForLowering,
) -> Result<LoweredEffects, CardTextError> {
    if let Some(lowered) = materialize_trailing_self_replacement(prepared)? {
        return Ok(fuse_trigger_context_battlefield_entry_counters(lowered));
    }

    let mut ctx = EffectLoweringContext::new();
    ctx.force_auto_tag_object_targets = prepared.force_auto_tag_object_targets;
    ctx.apply_reference_env(&prepared.initial_env);
    if let Some((effects, choices)) = materialize_source_sentence_segments(prepared, &mut ctx)? {
        let final_env = ctx.reference_env();
        return Ok(fuse_trigger_context_battlefield_entry_counters(
            LoweredEffects {
                effects,
                choices,
                exports: ReferenceExports::from_env(&final_env),
            },
        ));
    }
    let (compiled, choices) =
        compile_annotated_effects_with_context(&prepared.annotated, &mut ctx)?;
    let compiled = normalize_compiled_effects(compiled);
    let final_env = ctx.reference_env();
    Ok(fuse_trigger_context_battlefield_entry_counters(
        LoweredEffects {
            effects: crate::resolution::ResolutionProgram::from_effects(prepend_effect_prelude(
                compiled,
                compile_effect_prelude_tags(&prepared.prelude),
            )),
            choices,
            exports: ReferenceExports::from_env(&final_env),
        },
    ))
}

fn fuse_trigger_context_battlefield_entry_counters(mut lowered: LoweredEffects) -> LoweredEffects {
    super::super::battlefield_entry_counter_fusion::fuse_program(&mut lowered.effects);
    lowered
}

fn normalize_compiled_effects(mut compiled: Vec<Effect>) -> Vec<Effect> {
    // A prepared statement is one authored clause. Preserve a single trailing
    // duration across adjacent same-target grants even when the AST's
    // coordination wrapper was flattened before this normalization stage.
    super::effect_dispatch::preserve_shared_trailing_coordinated_duration(&mut compiled);
    let compiled = normalize_plural_coordinated_result_references(compiled);
    let compiled = normalize_coordinated_two_target_fight_sequences(compiled);
    let compiled = normalize_iterated_consult_exile_collection(compiled);
    let compiled = normalize_controller_grouped_exile_search(compiled);
    let compiled = normalize_targeted_conditional_action_then_fight(compiled);
    let compiled = normalize_two_target_conditional_then_fight(compiled);
    let compiled = normalize_two_target_counter_then_fight(compiled);
    let compiled = normalize_random_destroy_across_target_groups(compiled);
    let compiled = normalize_mixed_target_exile_top_damage(compiled);
    fold_local_zone_change_self_replacements(compiled)
}

fn with_wrapped_damage_target(effect: &Effect, target: ChooseSpec) -> Option<Effect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Some(Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            with_wrapped_damage_target(&tagged.effect, target)?,
        )));
    }
    let mut damage = effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?
        .clone();
    damage.target = target;
    Some(Effect::new(damage))
}

fn exact_exile_top_damage_procedure(
    effects: &[Effect],
) -> Option<(
    &crate::effects::ExileTopOfLibraryEffect,
    &crate::effects::DealDamageEffect,
)> {
    let [sequence_effect] = effects else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let [exile_effect, damage_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let exile = exile_effect.downcast_ref::<crate::effects::ExileTopOfLibraryEffect>()?;
    let mut damage_effect = damage_effect;
    while let Some(tagged) = damage_effect.downcast_ref::<crate::effects::TaggedEffect>() {
        damage_effect = &tagged.effect;
    }
    let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
    let [exiled_tag] = exile.moved_tags.as_slice() else {
        return None;
    };
    if exile.player != PlayerFilter::You
        || exile.count.unhinted() != &Value::Fixed(1)
        || exile.face_down
        || !exile.accumulated_tags.is_empty()
        || damage.source_is_combat
        || damage.unpreventable
        || !matches!(damage.amount.unhinted(), Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag == exiled_tag))
    {
        return None;
    }
    Some((exile, damage))
}

/// A mixed player/permanent target declaration is executed by two disjoint
/// runtime iterators. Rebind the damage recipient in each mirrored procedure
/// to that iterator's current member; the original broad target phrase is
/// only the declaration and must not become a fresh choice during resolution.
fn normalize_mixed_target_exile_top_damage(mut compiled: Vec<Effect>) -> Vec<Effect> {
    let [
        declaration_effect,
        player_loop_effect,
        object_loop_effect,
        _permission,
    ] = compiled.as_slice()
    else {
        return compiled;
    };
    let Some(declaration) = declaration_effect.downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return compiled;
    };
    let Some(target_only) = declaration
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    else {
        return compiled;
    };
    let ChooseSpec::ObjectOrPlayer(object_filter, PlayerFilter::Any) = target_only.target.base()
    else {
        return compiled;
    };
    let mut semantic_filter = object_filter.clone();
    semantic_filter.union_surface = Default::default();
    let mut expected_filter = ObjectFilter::default().in_zone(crate::zone::Zone::Battlefield);
    expected_filter.card_types = vec![
        crate::types::CardType::Creature,
        crate::types::CardType::Planeswalker,
    ];
    if !target_only.explicit_declaration
        || target_only.chooser.is_some()
        || target_only.target.count() != ChoiceCount::any_number()
        || semantic_filter != expected_filter
    {
        return compiled;
    }
    let Some(player_loop) =
        player_loop_effect.downcast_ref::<crate::effects::ForPlayersEffect<Effect>>()
    else {
        return compiled;
    };
    let Some(object_loop) =
        object_loop_effect.downcast_ref::<crate::effects::ForEachTaggedEffect<Effect>>()
    else {
        return compiled;
    };
    if player_loop.filter != PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any))
        || player_loop.starting_with_controller
        || player_loop.stop_after_first_happened
        || object_loop.tag != declaration.tag
        || object_loop.controller_at_last_blocked_by.is_some()
    {
        return compiled;
    }
    let Some((player_exile, player_damage)) =
        exact_exile_top_damage_procedure(&player_loop.effects)
    else {
        return compiled;
    };
    let Some((object_exile, object_damage)) =
        exact_exile_top_damage_procedure(&object_loop.effects)
    else {
        return compiled;
    };
    if player_exile != object_exile
        || player_damage.amount != object_damage.amount
        || player_damage.target != object_damage.target
    {
        return compiled;
    }
    let mut normalized_player = player_loop.clone();
    let mut normalized_object = object_loop.clone();
    let player_sequence = normalized_player.effects[0]
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("validated player procedure")
        .clone();
    let object_sequence = normalized_object.effects[0]
        .downcast_ref::<crate::effects::SequenceEffect>()
        .expect("validated object procedure")
        .clone();
    let mut player_sequence = player_sequence;
    let mut object_sequence = object_sequence;
    player_sequence.effects[1] = with_wrapped_damage_target(
        &player_sequence.effects[1],
        ChooseSpec::Player(PlayerFilter::IteratedPlayer),
    )
    .expect("validated player damage");
    object_sequence.effects[1] =
        with_wrapped_damage_target(&object_sequence.effects[1], ChooseSpec::Iterated)
            .expect("validated object damage");
    normalized_player.effects[0] = Effect::new(player_sequence);
    normalized_object.effects[0] = Effect::new(object_sequence);
    compiled[1] = Effect::new(normalized_player);
    compiled[2] = Effect::new(normalized_object);
    compiled
}

fn plural_result_reference_tag(effect: &Effect) -> Option<&TagKey> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return plural_result_reference_tag(&tagged.effect);
    }
    let apply = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    if !matches!(
        apply.set_quantifier_surface,
        Some(
            ironsmith_core::SetQuantifierSurface::They
                | ironsmith_core::SetQuantifierSurface::Those
        )
    ) {
        return None;
    }
    match apply.target_spec.as_ref()?.base() {
        ChooseSpec::Tagged(tag) => Some(tag),
        _ => None,
    }
}

fn coordinated_result_type_noun(effect: &Effect) -> Option<crate::types::CardType> {
    let mut current = effect;
    while let Some(tagged) = current.downcast_ref::<crate::effects::TaggedEffect>() {
        current = &tagged.effect;
    }
    let apply = current.downcast_ref::<crate::effects::ApplyContinuousEffect>()?;
    let ChooseSpec::Object(filter) = apply.target_spec.as_ref()?.base() else {
        return None;
    };
    filter.explicit_card_type_noun().or_else(|| {
        let [card_type] = filter.card_types.as_slice() else {
            return None;
        };
        Some(*card_type)
    })
}

fn coordinated_result_tags(
    effect: &Effect,
    final_tag: &TagKey,
) -> Option<(Vec<TagKey>, Option<crate::types::CardType>)> {
    let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if !sequence.surface.is_coordinated() {
        return None;
    }
    let tagged_effects = sequence
        .effects
        .iter()
        .filter(|effect| {
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .is_some()
        })
        .collect::<Vec<_>>();
    let tags = tagged_effects
        .iter()
        .filter_map(|effect| {
            effect
                .downcast_ref::<crate::effects::TaggedEffect>()
                .map(|tagged| tagged.tag.clone())
        })
        .collect::<Vec<_>>();
    if tags.len() <= 1 || tags.last() != Some(final_tag) {
        return None;
    }
    let type_noun = tagged_effects
        .first()
        .and_then(|effect| coordinated_result_type_noun(effect));
    let type_noun = type_noun.filter(|noun| {
        tagged_effects
            .iter()
            .all(|effect| coordinated_result_type_noun(effect) == Some(*noun))
    });
    Some((tags, type_noun))
}

fn with_plural_result_reference(
    effect: &Effect,
    tags: &[TagKey],
    type_noun: Option<crate::types::CardType>,
) -> Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            with_plural_result_reference(&tagged.effect, tags, type_noun),
        ));
    }
    let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() else {
        return effect.clone();
    };
    let mut apply = apply.clone();
    let mut result_filter = ObjectFilter::default();
    result_filter.any_of = tags.iter().cloned().map(ObjectFilter::tagged).collect();
    result_filter.set_explicit_card_type_noun(type_noun);
    apply.target_spec = Some(ChooseSpec::Object(result_filter));
    Effect::new(apply)
}

/// A plural pronoun after one coordinated producer clause refers to the union
/// of every independently tagged result in that clause, not merely its last
/// target slot. Keep the independent tags for target legality, then lower the
/// follow-up to an executable OR-filter over their exact result identities.
fn normalize_plural_coordinated_result_references(mut effects: Vec<Effect>) -> Vec<Effect> {
    for index in 1..effects.len() {
        let Some(final_tag) = plural_result_reference_tag(&effects[index]).cloned() else {
            continue;
        };
        let Some((tags, type_noun)) = coordinated_result_tags(&effects[index - 1], &final_tag)
        else {
            continue;
        };
        effects[index] = with_plural_result_reference(&effects[index], &tags, type_noun);
    }
    effects
}

fn normalize_cross_segment_plural_coordinated_result_references(
    segments: &mut [crate::resolution::ResolutionSegment],
) {
    for segment_idx in 1..segments.len() {
        let (earlier, current) = segments.split_at_mut(segment_idx);
        let Some(producer) = earlier
            .last()
            .and_then(|segment| segment.default_effects.last())
        else {
            continue;
        };
        let Some(followup) = current[0].default_effects.first_mut() else {
            continue;
        };
        let Some(final_tag) = plural_result_reference_tag(followup).cloned() else {
            continue;
        };
        let Some((tags, type_noun)) = coordinated_result_tags(producer, &final_tag) else {
            continue;
        };
        *followup = with_plural_result_reference(followup, &tags, type_noun);
    }
}

/// Oracle can assign one reveal-and-exile procedure to each object in a
/// tagged result, then move the complete exiled collection and shuffle only
/// after every iteration has finished. A literal `ForEachTagged` lowering
/// incorrectly performs the move and shuffle during every iteration.
///
/// Keep the consult and exile paired with each iterated object, tag the
/// aggregate outcome of that loop, then move that collection and shuffle once
/// for each distinct controller represented by the original tagged result.
fn unwrap_iterated_collection_result_tag(effect: &Effect) -> &Effect {
    let mut current = effect;
    while let Some(tagged) = current.downcast_ref::<crate::effects::TaggedEffect>() {
        current = &tagged.effect;
    }
    current
}

fn normalize_iterated_consult_exile_collection(compiled: Vec<Effect>) -> Vec<Effect> {
    let [producer_effect, per_object_effect] = compiled.as_slice() else {
        return compiled;
    };
    let Some(producer_tagged) = producer_effect.downcast_ref::<crate::effects::TaggedEffect>()
    else {
        return compiled;
    };
    if unwrap_iterated_collection_result_tag(&producer_tagged.effect)
        .downcast_ref::<crate::effects::DestroyEffect>()
        .is_none()
    {
        return compiled;
    }

    let Some(per_object) =
        per_object_effect.downcast_ref::<crate::effects::ForEachTaggedEffect<Effect>>()
    else {
        return compiled;
    };
    let [consult_effect, exile_effect, move_effect, shuffle_effect] = per_object.effects.as_slice()
    else {
        return compiled;
    };
    if per_object.tag != producer_tagged.tag {
        return compiled;
    }

    let Some(consult) = unwrap_iterated_collection_result_tag(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
    else {
        return compiled;
    };
    let single_match_stop = matches!(
        &consult.stop_rule,
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
            | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
    );
    let consult_uses_iterated_controller = consult.player == PlayerFilter::IteratedPlayer
        || matches!(
            &consult.player,
            PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
                if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        );
    if consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || !single_match_stop
        || !consult_uses_iterated_controller
    {
        return compiled;
    }

    let unwrapped_exile = unwrap_iterated_collection_result_tag(exile_effect);
    let exile_matches_consult =
        if let Some(exile) = unwrapped_exile.downcast_ref::<crate::effects::ExileEffect>() {
            matches!(exile.spec.base(), ChooseSpec::Tagged(tag) if tag == &consult.match_tag)
        } else if let Some(move_to_zone) =
            unwrapped_exile.downcast_ref::<crate::effects::MoveToZoneEffect>()
        {
            move_to_zone.zone == crate::zone::Zone::Exile
                && matches!(
                    move_to_zone.target.base(),
                    ChooseSpec::Tagged(tag) if tag == &consult.match_tag
                )
        } else {
            false
        };
    if !exile_matches_consult {
        return compiled;
    }

    let Some(move_to_zone) = unwrap_iterated_collection_result_tag(move_effect)
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return compiled;
    };
    if move_to_zone.zone != crate::zone::Zone::Battlefield
        || move_to_zone.to_top
        || move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || move_to_zone.battlefield_controller != crate::effects::BattlefieldController::Preserve
        || !matches!(
            move_to_zone.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return compiled;
    }

    let Some(shuffle) = unwrap_iterated_collection_result_tag(shuffle_effect)
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
    else {
        return compiled;
    };
    let shuffle_uses_iterated_controller = shuffle.player == PlayerFilter::IteratedPlayer
        || matches!(
            &shuffle.player,
            PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
                if tag == &consult.match_tag
                    || tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        );
    if !shuffle_uses_iterated_controller || shuffle.target_spec.is_some() {
        return compiled;
    }

    let collection_tag = crate::tag::CompilerDerivedTag::ExiledCollection.key(&consult.match_tag);
    let collected_loop = Effect::for_each_tagged(
        per_object.tag.clone(),
        vec![consult_effect.clone(), exile_effect.clone()],
    )
    .tag_all(collection_tag.clone());
    let mut collected_move = move_to_zone.clone();
    collected_move.target = ChooseSpec::Tagged(collection_tag);

    vec![
        producer_effect.clone(),
        collected_loop,
        Effect::new(collected_move),
        Effect::new(crate::effects::ForEachControllerOfTaggedEffect {
            tag: per_object.tag.clone(),
            effects: vec![Effect::shuffle_library_player(PlayerFilter::IteratedPlayer)],
        }),
    ]
}

/// Sentence segmentation preserves authored presentation boundaries, but a
/// staged "for each object destroyed this way" procedure semantically spans
/// the destroy sentence and the following reveal/exile sentence. Normalize
/// that adjacent pair while keeping the producer in its original segment and
/// the aggregate collection work in the following segment.
fn normalize_cross_segment_iterated_consult_exile_collections(
    segments: &mut [crate::resolution::ResolutionSegment],
) {
    for segment_idx in 0..segments.len().saturating_sub(1) {
        let (left, right) = segments.split_at_mut(segment_idx + 1);
        let producer_segment = &mut left[segment_idx];
        let procedure_segment = &mut right[0];
        if !producer_segment.self_replacements.is_empty()
            || !procedure_segment.self_replacements.is_empty()
        {
            continue;
        }
        let (Some(producer), Some(procedure)) = (
            producer_segment.default_effects.last().cloned(),
            procedure_segment.default_effects.first().cloned(),
        ) else {
            continue;
        };
        let normalized = normalize_iterated_consult_exile_collection(vec![producer, procedure]);
        let Ok(normalized): Result<[Effect; 4], _> = normalized.try_into() else {
            continue;
        };
        let [producer, collected_loop, collected_move, grouped_shuffle] = normalized;

        let producer_idx = producer_segment.default_effects.len() - 1;
        producer_segment.default_effects[producer_idx] = producer;
        procedure_segment
            .default_effects
            .splice(0..1, [collected_loop, collected_move, grouped_shuffle]);
    }
}

fn normalize_coordinated_two_target_fight_sequences(effects: Vec<Effect>) -> Vec<Effect> {
    let mut normalized = Vec::with_capacity(effects.len() + 1);
    let mut idx = 0usize;
    while idx < effects.len() {
        if idx + 2 < effects.len()
            && let Some(repaired) = normalize_coordinated_two_target_fight_window(
                &effects[idx],
                &effects[idx + 1],
                &effects[idx + 2],
            )
        {
            normalized.extend(repaired);
            idx += 3;
            continue;
        }

        normalized.push(effects[idx].clone());
        idx += 1;
    }
    normalized
}

fn local_zone_change_fallback_target(effect: &Effect) -> Option<&ChooseSpec> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return local_zone_change_fallback_target(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return local_zone_change_fallback_target(&with_id.effect);
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>() {
        let [inner] = unless_pays.effects.as_slice() else {
            return None;
        };
        return local_zone_change_fallback_target(inner);
    }
    effect.target_spec()
}

fn tagged_counter_spell_target(effect: &Effect) -> Option<(&TagKey, &ChooseSpec)> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_counter_spell_target(&with_id.effect);
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let target = local_zone_change_fallback_target(&tagged.effect)?;
    let counter = {
        let mut current = tagged.effect.as_ref();
        loop {
            if let Some(with_id) = current.downcast_ref::<crate::effects::WithIdEffect>() {
                current = &with_id.effect;
                continue;
            }
            if let Some(unless_pays) =
                current.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>()
            {
                let [inner] = unless_pays.effects.as_slice() else {
                    return None;
                };
                current = inner;
                continue;
            }
            break current.downcast_ref::<crate::effects::CounterEffect>()?;
        }
    };
    let ChooseSpec::Object(filter) = counter.target.base() else {
        return None;
    };
    if filter.zone != Some(crate::zone::Zone::Stack)
        || filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        || target != &counter.target
    {
        return None;
    }
    Some((&tagged.tag, target))
}

fn is_supported_counter_destination(
    replacement: &crate::effects::RegisterZoneReplacementEffect,
) -> bool {
    use ironsmith_core::ZoneReplacementLibraryPlacement;

    matches!(
        (replacement.replacement_zone, replacement.library_placement),
        (crate::zone::Zone::Exile | crate::zone::Zone::Hand, None)
            | (
                crate::zone::Zone::Library,
                Some(
                    ZoneReplacementLibraryPlacement::Top
                        | ZoneReplacementLibraryPlacement::Bottom
                        | ZoneReplacementLibraryPlacement::TopOrBottom,
                ),
            )
    )
}

fn fold_cross_segment_counter_rewrites(segments: &mut Vec<crate::resolution::ResolutionSegment>) {
    let mut idx = 0usize;
    while idx + 1 < segments.len() {
        let replacement = if segments[idx].self_replacements.is_empty()
            && segments[idx + 1].self_replacements.is_empty()
            && segments[idx].default_effects.len() == 1
            && segments[idx + 1].default_effects.len() == 1
        {
            segments[idx + 1].default_effects[0]
                .downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()
                .cloned()
        } else {
            None
        };
        let Some(replacement) = replacement else {
            idx += 1;
            continue;
        };
        let Some((producer_tag, _target)) =
            tagged_counter_spell_target(&segments[idx].default_effects[0])
        else {
            idx += 1;
            continue;
        };
        if !matches!(&replacement.target, ChooseSpec::Tagged(tag) if tag == producer_tag)
            || replacement.from_zone != Some(crate::zone::Zone::Stack)
            || replacement.to_zone != Some(crate::zone::Zone::Graveyard)
            || replacement.mode != crate::effects::ReplacementApplyMode::OneShot
            || replacement.optional
            || replacement.choice_description.is_some()
            || !replacement.counters.is_empty()
            || !is_supported_counter_destination(&replacement)
        {
            idx += 1;
            continue;
        }

        let producer = segments[idx].default_effects[0].clone();
        segments[idx].default_effects[0] = Effect::new(crate::effects::LocalRewriteEffect::new(
            producer,
            vec![replacement],
        ));
        segments.remove(idx + 1);
        idx += 1;
    }
}

/// Source-sentence boundaries are presentation boundaries, not reference
/// boundaries. Keep the generic ordered-target fight repairs effective when
/// the target declarations, conditional action, and final fight originated in
/// separate Oracle sentences.
fn sentence_leading_fight_tail(effect: &Effect) -> Option<&[Effect]> {
    let sequence = effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::SentenceLeadingThen {
        return None;
    }
    match sequence.effects.as_slice() {
        [fight]
            if fight
                .downcast_ref::<crate::effects::FightEffect>()
                .is_some() =>
        {
            Some(&sequence.effects)
        }
        [target, fight]
            if untagged_target_only(target).is_some()
                && fight
                    .downcast_ref::<crate::effects::FightEffect>()
                    .is_some() =>
        {
            Some(&sequence.effects)
        }
        _ => None,
    }
}

fn normalize_cross_segment_fight_sequences(segments: &mut [crate::resolution::ResolutionSegment]) {
    if segments.len() < 2
        || segments
            .iter()
            .any(|segment| !segment.self_replacements.is_empty())
    {
        return;
    }

    let mut flattened = Vec::new();
    for (segment_idx, segment) in segments.iter().enumerate() {
        for effect in &segment.default_effects {
            if let Some(tail) = sentence_leading_fight_tail(effect) {
                flattened.extend(
                    tail.iter()
                        .cloned()
                        .map(|tail_effect| (segment_idx, tail_effect)),
                );
            } else {
                flattened.push((segment_idx, effect.clone()));
            }
        }
    }
    let mut effects = Vec::with_capacity(flattened.len() + 1);
    let mut owners = Vec::with_capacity(flattened.len() + 1);
    let mut idx = 0usize;
    while idx < flattened.len() {
        if idx + 2 < flattened.len()
            && let Some(repaired) = normalize_coordinated_two_target_fight_window(
                &flattened[idx].1,
                &flattened[idx + 1].1,
                &flattened[idx + 2].1,
            )
        {
            let [first_target, second_target, conditional, fight] = repaired;
            effects.extend([first_target, second_target, conditional, fight]);
            owners.extend([
                flattened[idx].0,
                flattened[idx].0,
                flattened[idx + 1].0,
                flattened[idx + 2].0,
            ]);
            idx += 3;
            continue;
        }

        owners.push(flattened[idx].0);
        effects.push(flattened[idx].1.clone());
        idx += 1;
    }

    let effect_count = effects.len();
    let effects = normalize_targeted_conditional_action_then_fight(effects);
    let effects = normalize_two_target_conditional_then_fight(effects);
    let effects = normalize_two_target_counter_then_fight(effects);
    debug_assert_eq!(effects.len(), effect_count);
    if effects.len() != effect_count {
        return;
    }

    let mut repaired_by_segment = (0..segments.len())
        .map(|_| Vec::new())
        .collect::<Vec<Vec<Effect>>>();
    for (effect, owner) in effects.into_iter().zip(owners) {
        repaired_by_segment[owner].push(effect);
    }
    for (segment, repaired) in segments.iter_mut().zip(repaired_by_segment) {
        segment.default_effects = repaired;
    }
}

fn single_is_tagged_constraint(filter: &ObjectFilter, expected: &TagKey) -> bool {
    matches!(
        filter.tagged_constraints.as_slice(),
        [constraint]
            if constraint.tag == *expected
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::IsTaggedObject
    )
}

/// Preserve the one-to-one association between an object iterated by a
/// producer sentence and the tagged result created during that iteration.
///
/// A later "each of those [results] ... a different one of those [sources]"
/// sentence is otherwise lowered as a second `ForEachObject` whose `__it__`
/// binding shadows the source set. Besides rendering "it ... it", that can
/// make the produced object act on itself. Collapse only the exact typed
/// create-token/fight pipeline into the reusable two-phase correlated effect.
fn normalize_cross_segment_correlated_created_result_fights(
    segments: &mut Vec<crate::resolution::ResolutionSegment>,
) {
    let mut idx = 0;
    while idx + 1 < segments.len() {
        if !segments[idx].self_replacements.is_empty()
            || !segments[idx + 1].self_replacements.is_empty()
        {
            idx += 1;
            continue;
        }
        let [producer_effect] = segments[idx].default_effects.as_slice() else {
            idx += 1;
            continue;
        };
        let [consumer_effect] = segments[idx + 1].default_effects.as_slice() else {
            idx += 1;
            continue;
        };
        let Some(producer) = producer_effect.downcast_ref::<crate::effects::ForEachObject>() else {
            idx += 1;
            continue;
        };
        let [tagged_create_effect] = producer.effects.as_slice() else {
            idx += 1;
            continue;
        };
        let Some(tagged_create) =
            tagged_create_effect.downcast_ref::<crate::effects::TaggedEffect>()
        else {
            idx += 1;
            continue;
        };
        let Some(create) = tagged_create
            .effect
            .downcast_ref::<crate::effects::CreateTokenEffect>()
        else {
            idx += 1;
            continue;
        };
        if create.count != Value::Fixed(1)
            || create.controller != PlayerFilter::You
            || create.controller_target.is_some()
            || producer.filter.zone != Some(crate::zone::Zone::Battlefield)
            || !matches!(
                producer.filter.card_types.as_slice(),
                [crate::types::CardType::Creature]
            )
            || !matches!(
                producer.filter.controller.as_ref(),
                Some(PlayerFilter::Opponent | PlayerFilter::NotYou)
            )
        {
            idx += 1;
            continue;
        }

        let Some(consumer) = consumer_effect.downcast_ref::<crate::effects::ForEachObject>() else {
            idx += 1;
            continue;
        };
        if !consumer.filter.token
            || consumer.filter.zone.is_some()
            || consumer.filter.controller.is_some()
            || !consumer.filter.card_types.is_empty()
            || !single_is_tagged_constraint(&consumer.filter, &tagged_create.tag)
        {
            idx += 1;
            continue;
        }
        let [fight_effect] = consumer.effects.as_slice() else {
            idx += 1;
            continue;
        };
        let Some(fight) = fight_effect.downcast_ref::<crate::effects::FightEffect>() else {
            idx += 1;
            continue;
        };
        let ChooseSpec::Object(source_reference) = fight.creature2.base() else {
            idx += 1;
            continue;
        };
        let it_tag = crate::tag::CompilerReferenceTag::It.bind();
        if !matches!(fight.creature1.base(), ChooseSpec::Iterated)
            || source_reference.zone != Some(crate::zone::Zone::Battlefield)
            || !matches!(
                source_reference.card_types.as_slice(),
                [crate::types::CardType::Creature]
            )
            || !single_is_tagged_constraint(source_reference, &it_tag)
        {
            idx += 1;
            continue;
        }

        let source_binding_tag =
            crate::tag::CompilerDerivedTag::CorrelatedSource.key(&tagged_create.tag);
        let result_binding_tag =
            crate::tag::CompilerDerivedTag::CorrelatedResult.key(&tagged_create.tag);
        let fixed_fight = Effect::fight(
            ChooseSpec::Tagged(result_binding_tag.clone()),
            ChooseSpec::Tagged(source_binding_tag.clone()),
        );
        let correlated = Effect::new(crate::effects::ForEachObjectCorrelatedResultEffect::new(
            producer.filter.clone(),
            producer.effects.clone(),
            tagged_create.tag.clone(),
            source_binding_tag,
            result_binding_tag,
            vec![fixed_fight],
        ));
        segments[idx].default_effects = vec![correlated];
        segments.remove(idx + 1);
        idx += 1;
    }
}

fn conditional_fight_actions_target_tag(effects: &[Effect], target_tag: &TagKey) -> bool {
    !effects.is_empty()
        && effects.iter().all(|effect| {
            let effect = if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>()
            {
                &tagged.effect
            } else {
                effect
            };
            if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
                return conditional_fight_actions_target_tag(&sequence.effects, target_tag);
            }
            if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
                return matches!(
                    apply.target_spec.as_ref(),
                    Some(ChooseSpec::Tagged(tag)) if tag == target_tag
                );
            }
            if let Some(counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
                return matches!(&counters.target, ChooseSpec::Tagged(tag) if tag == target_tag);
            }
            false
        })
}

fn normalize_coordinated_two_target_fight_window(
    target_pair: &Effect,
    conditional: &Effect,
    fight: &Effect,
) -> Option<[Effect; 4]> {
    let sequence = target_pair.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated {
        return None;
    }
    let [first_target, second_target] = sequence.effects.as_slice() else {
        return None;
    };
    let (first_tag, first_spec) = tagged_target_only(first_target)?;
    let (second_tag, second_spec) = tagged_target_only(second_target)?;
    let first_filter = explicit_target_object_filter(first_spec)?;
    let second_filter = explicit_target_object_filter(second_spec)?;
    if first_tag == second_tag || !is_controlled_creature_fight_pair(first_filter, second_filter) {
        return None;
    }

    let conditional_effect = conditional.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let fight_effect = fight.downcast_ref::<crate::effects::FightEffect>()?;
    let both_authored_group =
        (choose_spec_has_authored_creature_group_surface(&fight_effect.creature1)
            || choose_spec_references_tag(
                &fight_effect.creature1,
                crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
            ))
            && (choose_spec_has_authored_creature_group_surface(&fight_effect.creature2)
                || choose_spec_references_tag(
                    &fight_effect.creature2,
                    crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
                ));
    if conditional_effect.if_false.is_empty()
        && both_authored_group
        && let Some(fixed_branch) = normalize_conditional_branch_target(
            &conditional_effect.if_true,
            first_tag,
            first_filter,
        )
    {
        return Some([
            first_target.clone(),
            second_target.clone(),
            Effect::conditional(
                conditional_effect.condition.clone(),
                fixed_branch,
                Vec::new(),
            ),
            Effect::fight(
                ChooseSpec::Tagged(first_tag.clone()),
                ChooseSpec::Tagged(second_tag.clone()),
            ),
        ]);
    }

    let candidate = vec![
        first_target.clone(),
        second_target.clone(),
        conditional.clone(),
        fight.clone(),
    ];
    let candidate = normalize_two_target_conditional_then_fight(candidate);
    let candidate = normalize_two_target_counter_then_fight(candidate);
    let [first_target, second_target, conditional, fight] = candidate.as_slice() else {
        return None;
    };
    let (first_tag, _) = tagged_target_only(first_target)?;
    let (second_tag, _) = tagged_target_only(second_target)?;
    let conditional = conditional.downcast_ref::<crate::effects::ConditionalEffect>()?;
    let fight = fight.downcast_ref::<crate::effects::FightEffect>()?;
    if !conditional.if_false.is_empty()
        || !conditional_fight_actions_target_tag(&conditional.if_true, first_tag)
        || !matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == first_tag)
        || !matches!(&fight.creature2, ChooseSpec::Tagged(tag) if tag == second_tag)
    {
        return None;
    }

    candidate.try_into().ok()
}

fn tagged_creature_damage_tag(effect: &Effect) -> Option<&TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_creature_damage_tag(&with_id.effect);
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let damage = tagged
        .effect
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    let ChooseSpec::Object(filter) = damage.target.base() else {
        return None;
    };
    (damage.target.is_target()
        && matches!(
            filter.card_types.as_slice(),
            [crate::types::CardType::Creature]
        ))
    .then_some(&tagged.tag)
}

fn tagged_exile_attached_to<'a>(effect: &'a Effect, anchor: &TagKey) -> Option<&'a TagKey> {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return tagged_exile_attached_to(&with_id.effect, anchor);
    }
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()?;
    let ChooseSpec::Object(filter) = exile.spec.base() else {
        return None;
    };
    let mut attachment_links = filter.tagged_constraints.iter().filter(|constraint| {
        constraint.relation == crate::filter::TaggedOpbjectRelation::AttachedToTaggedObject
    });
    let attachment_link = attachment_links.next()?;
    (exile.spec.is_target() && attachment_links.next().is_none() && attachment_link.tag == *anchor)
        .then_some(&tagged.tag)
}

/// A later explicit target normally becomes the pronoun antecedent. One
/// important exception is a death clause that names the permanent an exiled
/// attachment was attached to: the attachment has already left the
/// battlefield, so a battlefield-to-graveyard replacement cannot refer to
/// it. Recover the independently encoded attachment anchor in that exact
/// three-sentence shape.
fn link_death_replacement_to_exiled_attachment(
    segments: &mut [crate::resolution::ResolutionSegment],
) {
    for idx in 0..segments.len().saturating_sub(2) {
        let [producer] = segments[idx].default_effects.as_slice() else {
            continue;
        };
        let [attachment_exile] = segments[idx + 1].default_effects.as_slice() else {
            continue;
        };
        let [replacement_effect] = segments[idx + 2].default_effects.as_slice() else {
            continue;
        };
        if !segments[idx].self_replacements.is_empty()
            || !segments[idx + 1].self_replacements.is_empty()
            || !segments[idx + 2].self_replacements.is_empty()
        {
            continue;
        }

        let Some(producer_tag) = tagged_creature_damage_tag(producer) else {
            continue;
        };
        let Some(attachment_tag) = tagged_exile_attached_to(attachment_exile, producer_tag) else {
            continue;
        };
        let Some(replacement) =
            replacement_effect.downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()
        else {
            continue;
        };
        if !matches!(replacement.target.base(), ChooseSpec::Tagged(tag) if tag == attachment_tag)
            || replacement.from_zone != Some(crate::zone::Zone::Battlefield)
            || replacement.to_zone != Some(crate::zone::Zone::Graveyard)
            || replacement.replacement_zone != crate::zone::Zone::Exile
            || replacement.library_placement.is_some()
            || replacement.mode != crate::effects::ReplacementApplyMode::UntilEndOfTurn
            || replacement.optional
            || replacement.choice_description.is_some()
            || !replacement.counters.is_empty()
        {
            continue;
        }

        let mut replacement = replacement.clone();
        replacement.target = ChooseSpec::Tagged(producer_tag.clone());
        segments[idx + 2].default_effects[0] = Effect::new(replacement);
    }
}

/// Keep both attachment classes anchored to the same triggering permanent in
/// a return-and-reattach procedure. The intervening Aura result becomes the
/// destination reference, but it must not replace the historical object in
/// the later "Equipment that were attached to it" filter.
pub fn bind_returned_attachment_history_to_triggering_object(
    segments: &mut [crate::resolution::ResolutionSegment],
) {
    for segment in segments {
        let [first_snapshot, second_snapshot, sequence_effect] =
            segment.default_effects.as_mut_slice()
        else {
            continue;
        };
        let Some(first_snapshot) =
            first_snapshot.downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
        else {
            continue;
        };
        if second_snapshot
            .downcast_ref::<crate::effects::TagAttachedToSourceEffect>()
            .is_none()
        {
            continue;
        }
        let anchor_tag = first_snapshot.tag.clone();
        let Some(sequence) = sequence_effect
            .downcast_ref::<crate::effects::SequenceEffect>()
            .cloned()
        else {
            continue;
        };
        if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
            continue;
        }
        let [move_effect, attach_returned_effect, attach_history_effect] =
            sequence.effects.as_slice()
        else {
            continue;
        };
        let Some(moved) = move_effect.downcast_ref::<crate::effects::TaggedEffect>() else {
            continue;
        };
        let moved_tag = moved.tag.clone();
        let Some(movement) = moved
            .effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
        else {
            continue;
        };
        let ChooseSpec::WithCount(aura_spec, aura_count) = movement.target.unhinted() else {
            continue;
        };
        let ChooseSpec::Object(aura_filter) = aura_spec.unhinted() else {
            continue;
        };
        if !aura_count.is_any_number()
            || movement.zone != crate::zone::Zone::Battlefield
            || !matches!(
                aura_filter.subtypes.as_slice(),
                [crate::types::Subtype::Aura]
            )
            || !aura_filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == anchor_tag
                    && constraint.relation
                        == crate::filter::TaggedOpbjectRelation::WasAttachedToTaggedObject
            })
        {
            continue;
        }
        let Some(attach_returned) =
            attach_returned_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()
        else {
            continue;
        };
        if !matches!(
            attach_returned.objects.base(),
            ChooseSpec::Object(filter) | ChooseSpec::All(filter)
                if filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag == moved_tag
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
                })
        ) {
            continue;
        }
        let Some(attach_history) =
            attach_history_effect.downcast_ref::<crate::effects::AttachObjectsEffect>()
        else {
            continue;
        };
        let ChooseSpec::Object(equipment_filter) = attach_history.objects.base() else {
            continue;
        };
        if !matches!(
            equipment_filter.subtypes.as_slice(),
            [crate::types::Subtype::Equipment]
        ) || !matches!(attach_history.target.base(), ChooseSpec::Tagged(tag) if tag == &moved_tag)
        {
            continue;
        }
        let mut normalized = sequence;
        let Some(normalized_attach) = normalized.effects[2]
            .downcast_ref::<crate::effects::AttachObjectsEffect>()
            .cloned()
        else {
            continue;
        };
        let mut normalized_attach = normalized_attach;
        let ChooseSpec::WithCount(objects, count) = &mut normalized_attach.objects else {
            continue;
        };
        if !count.is_any_number() {
            continue;
        }
        let ChooseSpec::Object(filter) = objects.as_mut() else {
            continue;
        };
        let [constraint] = filter.tagged_constraints.as_mut_slice() else {
            continue;
        };
        if constraint.relation != crate::filter::TaggedOpbjectRelation::WasAttachedToTaggedObject
            || constraint.tag != moved_tag
        {
            continue;
        }
        constraint.tag = anchor_tag;
        normalized.effects[2] = Effect::new(normalized_attach);
        *sequence_effect = Effect::new(normalized);
    }
}

/// Oracle sometimes iterates the individual objects affected by a zone change
/// while assigning a follow-up search to each distinct controller. Lowering
/// the sentence literally as `ForEachTagged` repeats the search once per
/// object and leaves the later collective move/shuffle attributed to the
/// caster. Collapse that precise linked pipeline into controller groups while
/// retaining the shared result tag and authored ordering.
fn normalize_controller_grouped_exile_search(compiled: Vec<Effect>) -> Vec<Effect> {
    let [exile_effect, per_object_effect, move_effect, shuffle_effect] = compiled.as_slice() else {
        return compiled;
    };

    let Some(exile_tagged) = exile_effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return compiled;
    };
    let exile_filter = if let Some(move_to_zone) = exile_tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>(
    ) {
        if move_to_zone.zone != crate::zone::Zone::Exile {
            return compiled;
        }
        match move_to_zone.target.base() {
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
            _ => return compiled,
        }
    } else if let Some(exile) = exile_tagged
        .effect
        .downcast_ref::<crate::effects::ExileEffect>()
    {
        match exile.spec.base() {
            ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter,
            _ => return compiled,
        }
    } else {
        return compiled;
    };
    if !matches!(
        exile_filter.card_types.as_slice(),
        [crate::types::CardType::Creature]
    ) || exile_filter.controller != Some(PlayerFilter::NotYou)
    {
        return compiled;
    }

    let Some(per_object) =
        per_object_effect.downcast_ref::<crate::effects::ForEachTaggedEffect<Effect>>()
    else {
        return compiled;
    };
    let [search_effect] = per_object.effects.as_slice() else {
        return compiled;
    };
    let Some(search) = search_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
        return compiled;
    };
    if per_object.tag != exile_tagged.tag
        || !search.is_search
        || search.zone != Some(crate::zone::Zone::Library)
        || !search.additional_zones.is_empty()
        || !search.count.is_single()
        || search.count_value.is_some()
        || search.filter.zone != Some(crate::zone::Zone::Library)
        || !matches!(
            search.filter.card_types.as_slice(),
            [crate::types::CardType::Land]
        )
        || !matches!(
            search.filter.supertypes.as_slice(),
            [crate::types::Supertype::Basic]
        )
        || search.chooser
            != PlayerFilter::ControllerOf(ObjectRef::tagged(
                crate::tag::CompilerReferenceTag::It.bind(),
            ))
    {
        return compiled;
    }

    let Some(moved_tagged) = move_effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return compiled;
    };
    let Some(move_to_zone) = moved_tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return compiled;
    };
    if move_to_zone.zone != crate::zone::Zone::Battlefield
        || !move_to_zone.enters_tapped
        || move_to_zone.enters_attacking
        || move_to_zone.enters_face_down
        || !matches!(move_to_zone.target.base(), ChooseSpec::Tagged(tag) if tag == &search.tag)
    {
        return compiled;
    }
    let Some(shuffle) = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()
    else {
        return compiled;
    };
    if shuffle.player != PlayerFilter::You || shuffle.target_spec.is_some() {
        return compiled;
    }

    let mut grouped_search = search.clone();
    grouped_search.filter.owner = Some(PlayerFilter::IteratedPlayer);
    grouped_search.chooser = PlayerFilter::IteratedPlayer;
    grouped_search.count = ChoiceCount::up_to_dynamic_x();
    grouped_search.count_value = Some(Value::TaggedCount);
    grouped_search.search_mode = SearchSelectionMode::Optional;
    grouped_search.replace_tagged_objects = false;

    vec![
        exile_effect.clone(),
        Effect::new(crate::effects::ForEachControllerOfTaggedEffect {
            tag: exile_tagged.tag.clone(),
            effects: vec![Effect::new(grouped_search)],
        }),
        move_effect.clone(),
        Effect::new(crate::effects::ForEachControllerOfTaggedEffect {
            tag: exile_tagged.tag.clone(),
            effects: vec![Effect::shuffle_library_player(PlayerFilter::IteratedPlayer)],
        }),
    ]
}

fn materialize_source_sentence_segments(
    prepared: &PreparedEffectsForLowering,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(crate::resolution::ResolutionProgram, Vec<ChooseSpec>)>, CardTextError> {
    if prepared.source_sentence_segments.is_empty() {
        return Ok(None);
    }
    let expected_effect_count = prepared
        .source_sentence_segments
        .iter()
        .map(|segment| segment.effect_count)
        .sum::<usize>();
    if expected_effect_count != prepared.annotated.effects.len() {
        return Err(CardTextError::InvariantViolation(format!(
            "source-sentence effect counts cover {expected_effect_count} effects, but preparation annotated {}",
            prepared.annotated.effects.len()
        )));
    }

    let mut segments = Vec::with_capacity(prepared.source_sentence_segments.len());
    let mut choices = Vec::new();
    let mut start = 0;
    let mut previous_sentence_was_only_you_draw = false;
    for source_segment in &prepared.source_sentence_segments {
        let end = start + source_segment.effect_count;
        let mut annotated_effects = prepared.annotated.effects[start..end].to_vec();
        if source_segment.leading_then && previous_sentence_was_only_you_draw {
            bind_implicit_discard_after_you_draw(&mut annotated_effects);
        }
        let final_env = annotated_effects
            .last()
            .map(|effect| effect.out_env.clone())
            .unwrap_or_else(|| prepared.initial_env.clone());
        let annotated = AnnotatedEffectSequence {
            effects: annotated_effects,
            final_env,
        };
        // The compiler derives its effective auto-tag policy once per call.
        // Do not let the prior sentence's final effect become the next
        // sentence's global policy merely because these groups compile in
        // separate calls.
        ctx.auto_tag_object_targets = false;
        let (compiled, sentence_choices) = compile_annotated_effects_with_context(&annotated, ctx)?;
        let mut compiled = normalize_compiled_effects(compiled);
        // Source-sentence materialization can expose the members of one
        // coordinated grant as adjacent top-level effects even when the AST
        // coordination wrapper was normalized away. The authored trailing
        // duration still scopes every same-target continuous member.
        super::effect_dispatch::preserve_shared_trailing_coordinated_duration(&mut compiled);
        previous_sentence_was_only_you_draw = compiled_sentence_is_only_you_draw(&compiled);
        if source_segment.starting_with_controller
            && let [effect] = compiled.as_mut_slice()
            && let Some(mut for_players) = effect
                .downcast_ref::<crate::effects::ForPlayersEffect<Effect>>()
                .cloned()
        {
            for_players.starting_with_controller = true;
            *effect = Effect::new(for_players);
        }
        let compiled = if source_segment.leading_then && !compiled.is_empty() {
            vec![Effect::new(
                crate::effects::SequenceEffect::sentence_leading_then(compiled),
            )]
        } else {
            compiled
        };
        merge_compiled_choices(&mut choices, &compiled, sentence_choices);
        if !compiled.is_empty() {
            let mut segment = crate::resolution::ResolutionSegment::from_effects(compiled);
            segment.starts_new_source_line = !segments.is_empty();
            segments.push(segment);
        }
        start = end;
    }

    normalize_cross_segment_plural_coordinated_result_references(&mut segments);
    normalize_cross_segment_iterated_consult_exile_collections(&mut segments);
    normalize_cross_segment_correlated_created_result_fights(&mut segments);
    normalize_cross_segment_fight_sequences(&mut segments);
    link_death_replacement_to_exiled_attachment(&mut segments);
    bind_returned_attachment_history_to_triggering_object(&mut segments);
    fold_cross_segment_counter_rewrites(&mut segments);
    if let Some(first) = segments.first_mut() {
        first.default_effects = prepend_effect_prelude(
            std::mem::take(&mut first.default_effects),
            compile_effect_prelude_tags(&prepared.prelude),
        );
    }
    Ok(Some((
        crate::resolution::ResolutionProgram::new(segments),
        choices,
    )))
}

fn bind_implicit_discard_after_you_draw(annotated_effects: &mut [AnnotatedEffect]) {
    let [annotated] = annotated_effects else {
        return;
    };
    let EffectAst::Conditional {
        predicate: PredicateAst::Not(_),
        if_true,
        if_false,
    } = &mut annotated.effect
    else {
        return;
    };
    let [EffectAst::SubjectVerb(discard)] = if_true.as_mut_slice() else {
        return;
    };
    if !if_false.is_empty()
        || !matches!(discard.action, SubjectVerbActionAst::Discard { .. })
        || discard.subject.player != PlayerAst::Implicit
    {
        return;
    }
    discard.subject.player = PlayerAst::You;
}

fn compiled_sentence_is_only_you_draw(effects: &[Effect]) -> bool {
    let [effect] = effects else {
        return false;
    };
    effect
        .downcast_ref::<crate::effects::DrawCardsEffect>()
        .is_some_and(|draw| draw.player == PlayerFilter::You)
}

fn target_has_explicit_declaration_span(target: &TargetAst) -> bool {
    match target {
        TargetAst::AnyTarget(span) | TargetAst::AnyOtherTarget(span) | TargetAst::Spell(span) => {
            span.is_some()
        }
        TargetAst::ObjectOrPlayer(_, _, span)
        | TargetAst::PlayerOrPlaneswalker(_, span)
        | TargetAst::Player(_, span) => span.is_some(),
        TargetAst::Object(_, target_span, _) => target_span.is_some(),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_has_explicit_declaration_span(inner)
        }
        TargetAst::Source(_)
        | TargetAst::AttackedPlayerOrPlaneswalker(_)
        | TargetAst::Tagged(_, _) => false,
    }
}

fn primary_put_counters_target(effects: &[EffectAst]) -> Option<TargetAst> {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::PutCounters { target, .. } = &subject_verb.action
        {
            return Some(target.clone());
        }
        let mut nested_target = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_target.is_none() {
                nested_target = primary_put_counters_target(nested);
            }
        });
        if nested_target.is_some() {
            return nested_target;
        }
    }
    None
}

fn primary_double_counters_target(effects: &[EffectAst]) -> Option<TargetAst> {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject_verb) = effect
            && let SubjectVerbActionAst::DoubleCountersOnTarget { target, .. } =
                &subject_verb.action
        {
            return Some(target.clone());
        }
        let mut nested_target = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_target.is_none() {
                nested_target = primary_double_counters_target(nested);
            }
        });
        if nested_target.is_some() {
            return nested_target;
        }
    }
    None
}

/// A counter self-replacement can refer to the default action's target from a
/// condition that does not itself mention that target (for example, an
/// ability-resolution count). In that case ordinary reference preparation
/// has no reason to expose a TargetOnly prelude. Identical declaration spans
/// prove the replacement target was copied from the default action rather
/// than authored as a second equal-looking target.
fn shared_counter_self_replacement_target(
    if_false: &[EffectAst],
    if_true: &[EffectAst],
) -> Option<TargetAst> {
    let default_target = primary_put_counters_target(if_false)?;
    let replacement_target = primary_double_counters_target(if_true)?;
    (target_has_explicit_declaration_span(&default_target) && default_target == replacement_target)
        .then_some(default_target)
}

fn bind_shared_counter_target_to_it(effects: &mut [EffectAst], shared_target: &TargetAst) {
    for effect in effects {
        if let EffectAst::SubjectVerb(subject_verb) = effect {
            let counter_target = match &mut subject_verb.action {
                SubjectVerbActionAst::PutCounters { target, .. }
                | SubjectVerbActionAst::DoubleCountersOnTarget { target, .. } => Some(target),
                _ => None,
            };
            if let Some(target) = counter_target
                && target == shared_target
            {
                *target = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
            }
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            bind_shared_counter_target_to_it(nested, shared_target);
        });
    }
}

/// A self-replacement may repeat an earlier target anaphorically ("that
/// artifact") even though both branch actions carry a concrete `TargetAst`
/// after parsing. `replace_it_target_in_effects` deliberately copies the
/// original target, including its declaration span, into the replacement
/// branch. Matching that exact identity lets lowering share the target choice
/// without conflating separately authored, equal-looking target phrases.
fn self_replacement_branches_share_explicit_target(
    if_false: &[EffectAst],
    if_true: &[EffectAst],
) -> bool {
    if !effects_reference_tag(if_false, crate::tag::CompilerReferenceTag::It.as_str())
        || !effects_reference_tag(if_true, crate::tag::CompilerReferenceTag::It.as_str())
    {
        return false;
    }
    let Some(default_target) = if_false
        .iter()
        .find_map(crate::model::ast::primary_target_from_effect)
    else {
        return false;
    };
    let Some(replacement_target) = if_true
        .iter()
        .find_map(crate::model::ast::primary_target_from_effect)
    else {
        return false;
    };
    target_has_explicit_declaration_span(&default_target) && default_target == replacement_target
}

fn leading_target_only_spec(effect: &Effect) -> Option<&ChooseSpec> {
    tagged_target_only(effect)
        .map(|(_, target)| target)
        .or_else(|| untagged_target_only(effect))
}

fn materialize_trailing_self_replacement(
    prepared: &PreparedEffectsForLowering,
) -> Result<Option<LoweredEffects>, CardTextError> {
    if let Some((replacement, prefix_annotated)) = prepared.annotated.effects.split_last()
        && let EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            ..
        } = &replacement.effect
        && prefix_annotated
            .iter()
            .all(|effect| !matches!(effect.effect, EffectAst::SelfReplacement { .. }))
    {
        let prefix_effects = prefix_annotated
            .iter()
            .map(|annotated| annotated.effect.clone())
            .collect::<Vec<_>>();
        let shared_counter_target = shared_counter_self_replacement_target(if_false, if_true);
        let mut effective_if_false = if_false.to_vec();
        let mut effective_if_true = if_true.to_vec();
        if let Some(target) = shared_counter_target.as_ref() {
            bind_shared_counter_target_to_it(&mut effective_if_false, target);
            bind_shared_counter_target_to_it(&mut effective_if_true, target);
            effective_if_false.insert(
                0,
                EffectAst::subject_verb_explicit_target_only(target.clone()),
            );
        }
        let prefix_lowered =
            compile_statement_effects_with_imports(&prefix_effects, &prepared.imports)?;
        let mut condition_env = if prefix_effects.is_empty() {
            prepared.initial_env.clone()
        } else {
            prefix_annotated[prefix_effects.len() - 1].out_env.clone()
        };
        // Both arms occur after the authored prefix and may refer to a target
        // or object established there. Recompiling them from the statement's
        // original imports loses that antecedent at source-sentence boundaries
        // (for example, "target player's library ... instead ... that
        // player's library").
        let branch_imports = ReferenceExports::from_env(&condition_env).to_imports();
        let default_lowered =
            compile_statement_effects_with_imports(&effective_if_false, &branch_imports)?;
        let shared_target_prelude = (shared_counter_target.is_some()
            || self_replacement_branches_share_explicit_target(if_false, if_true))
        .then(|| {
            default_lowered
                .effects
                .flattened_default_effects()
                .first()
                .and_then(tagged_target_only)
                .map(|(tag, target)| (tag.clone(), target.clone()))
        })
        .flatten();
        let mut replacement_imports = branch_imports.clone();
        if let Some((tag, _)) = shared_target_prelude.as_ref() {
            // The declaration is an unconditional prelude of the default arm,
            // so it is also available before the replacement branch resolves.
            // Seed only that object identity; result/player exports from the
            // mutually exclusive default action must not leak across.
            replacement_imports.last_object_tag = Some(tag.clone());
            replacement_imports.last_it_choice_is_set =
                default_lowered.exports.last_it_choice_is_set;
        }
        let replacement_lowered =
            compile_statement_effects_with_imports(&effective_if_true, &replacement_imports)?;
        let mut default_effects = prefix_lowered.effects.flattened_default_effects().to_vec();
        default_effects.extend(default_lowered.effects.flattened_default_effects().to_vec());
        let implicit_condition_tag = if condition_env.known_last_object_tag().is_none()
            && predicate_uses_implicit_object_reference(predicate)
        {
            last_tagged_default_target(&default_effects).map(|(tag, _)| tag)
        } else {
            None
        };
        let has_default_action_target = default_effects
            .iter()
            .rev()
            .any(|effect| crate::lower::extract_previous_replacement_target(effect).is_some());
        let has_resolution_target = default_effects
            .iter()
            .any(|effect| effect.target_spec().is_some());
        if condition_env.known_last_object_tag().is_none()
            && implicit_condition_tag.is_none()
            && predicate_uses_implicit_object_reference(predicate)
            && !has_default_action_target
        {
            condition_env.source_object_antecedent = true;
        }
        let condition_imports = ReferenceExports::from_env(&condition_env).to_imports();
        // Explicit target predicates stay relative to the target selected by
        // this resolution. Candidate-ability predicates ("if it has
        // unearth") do too: an ambient trigger/result tag must not steal that
        // authored target from the replacement condition.
        let explicitly_tests_resolution_target =
            matches!(predicate, PredicateAst::TargetMatches(_));
        let tests_candidate_ability_on_resolution_target = matches!(
            predicate,
            PredicateAst::ItMatches(filter) | PredicateAst::TargetMatches(filter)
                if filter.has_trailing_candidate_ability_condition_surface()
        );
        let compares_target_to_prior_choice = matches!(
            predicate,
            PredicateAst::TargetMatches(filter)
                if super::filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
        );
        let default_action_condition_tag =
            last_tagged_default_target(&default_effects).map(|(tag, _)| tag);
        let condition_tag = if has_resolution_target
            && (tests_candidate_ability_on_resolution_target || compares_target_to_prior_choice)
        {
            None
        } else if has_resolution_target && explicitly_tests_resolution_target {
            // A demonstrative condition ordinarily denotes the target of the
            // default action, whose stable tag preserves authored identity
            // surfaces. Some actions (notably token-copy source choices) do
            // not export such a tag; their explicit TargetMatches predicate
            // must remain resolution-target relative instead of falling back
            // to an ambient trigger tag.
            implicit_condition_tag
                .as_ref()
                .or(default_action_condition_tag.as_ref())
        } else {
            implicit_condition_tag
                .as_ref()
                .or(condition_imports.last_object_tag.as_ref())
        };
        let condition = compile_condition_from_predicate_ast_with_env(
            predicate,
            &condition_env,
            condition_tag,
        )?;
        let mut replacement_effects = replacement_lowered
            .effects
            .flattened_default_effects()
            .to_vec();
        if let Some((_, shared_target)) = shared_target_prelude.as_ref()
            && replacement_effects
                .first()
                .and_then(leading_target_only_spec)
                .is_some_and(|target| target == shared_target)
        {
            // Compiling a conditional arm in isolation exposes its hidden
            // target choice with a TargetOnly prelude. The identical target
            // is already exposed by the shared default prelude above.
            replacement_effects.remove(0);
        }
        let mut replacement_effects =
            strip_duplicate_self_replacement_prelude(&default_effects, replacement_effects);
        if let Some(previous_target) = default_effects
            .iter()
            .rev()
            .find_map(crate::lower::extract_previous_replacement_target)
        {
            replacement_effects = replacement_effects
                .into_iter()
                .map(|effect| {
                    crate::lower::replacement_effect_with_target(&effect, &previous_target)
                        .unwrap_or(effect)
                })
                .collect();
        }
        let replacement_effects = if let Some(antecedent) = default_effects
            .iter()
            .rev()
            .find(|effect| effect.target_spec().is_some())
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&replacement_effects, antecedent)
        {
            vec![Effect::new(crate::effects::LocalRewriteEffect::new(
                antecedent.clone(),
                zone_replacements,
            ))]
        } else {
            replacement_effects
        };

        let mut choices = prefix_lowered.choices;
        for choice in default_lowered
            .choices
            .into_iter()
            .chain(replacement_lowered.choices)
        {
            push_choice(&mut choices, choice);
        }
        return Ok(Some(LoweredEffects {
            effects: crate::resolution::ResolutionProgram::new(vec![
                crate::resolution::ResolutionSegment {
                    default_effects,
                    self_replacements: vec![crate::resolution::SelfReplacementBranch::new(
                        condition,
                        replacement_effects,
                    )],
                    starts_new_source_line: false,
                },
            ]),
            choices,
            exports: prepared.exports.clone(),
        }));
    }
    Ok(None)
}

pub(super) fn last_tagged_default_target(
    default_effects: &[Effect],
) -> Option<(TagKey, ChooseSpec)> {
    default_effects
        .iter()
        .rev()
        .find_map(last_tagged_target_in_effect)
}

fn last_tagged_target_in_effect(effect: &Effect) -> Option<(TagKey, ChooseSpec)> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        if let Some(target_only) = tagged
            .effect
            .downcast_ref::<crate::effects::TargetOnlyEffect>()
        {
            return Some((tagged.tag.clone(), target_only.target.clone()));
        }
        if let Some(target) = tagged.effect.target_spec() {
            return Some((tagged.tag.clone(), target.clone()));
        }
        return last_tagged_target_in_effect(&tagged.effect);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return last_tagged_target_in_effect(&with_id.effect);
    }
    if let Some(local_rewrite) = effect.downcast_ref::<crate::effects::LocalRewriteEffect>() {
        return last_tagged_target_in_effect(&local_rewrite.effect);
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        return sequence
            .effects
            .iter()
            .rev()
            .find_map(last_tagged_target_in_effect);
    }
    if let Some(may) = effect.downcast_ref::<crate::effects::MayEffect<Effect>>() {
        return may
            .effects
            .iter()
            .rev()
            .find_map(last_tagged_target_in_effect);
    }
    None
}

fn predicate_uses_implicit_object_reference(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::ItIsLandCard
        | PredicateAst::ItIsSoulbondPaired
        | PredicateAst::ItMatches(_)
        | PredicateAst::ItMatchedLastKnown(_)
        | PredicateAst::TargetMatches(_) => true,
        PredicateAst::Not(inner) => predicate_uses_implicit_object_reference(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_uses_implicit_object_reference(left)
                || predicate_uses_implicit_object_reference(right)
        }
        _ => false,
    }
}

pub fn materialize_prepared_triggered_effects(
    prepared: &PreparedTriggeredEffectsForLowering,
) -> Result<(LoweredEffects, Option<Condition>), CardTextError> {
    let mut lowered = materialize_prepared_effects_with_trigger_context(&prepared.prepared)?;
    strip_erroneous_meld_player_exile_effect(&mut lowered);
    dedupe_adjacent_target_only_effects(&mut lowered);
    let intervening_if = prepared
        .intervening_if
        .as_ref()
        .map(compile_prepared_predicate_for_lowering)
        .transpose()?;
    if let Some(condition) = intervening_if.as_ref() {
        link_source_move_to_damaged_death_card(&mut lowered, condition);
    }
    Ok((lowered, intervening_if))
}

fn damaged_death_condition_target_filter(condition: &Condition) -> Option<ObjectFilter> {
    match condition {
        Condition::CreatureDealtDamageBySourceDiedThisTurn {
            victim,
            damager,
            count,
        } if *count == 1 => {
            let mut filter = victim.clone();
            filter.zone = Some(crate::zone::Zone::Graveyard);
            filter.entered_graveyard_from_battlefield_this_turn = true;
            filter.dealt_damage_by_source_this_turn = Some(*damager);
            Some(filter)
        }
        Condition::And(left, right) => damaged_death_condition_target_filter(left)
            .or_else(|| damaged_death_condition_target_filter(right)),
        _ => None,
    }
}

fn link_source_move_to_damaged_death_card(lowered: &mut LoweredEffects, condition: &Condition) {
    let Some(filter) = damaged_death_condition_target_filter(condition) else {
        return;
    };
    let Some(segment) = lowered.effects.segments.first_mut() else {
        return;
    };
    let Some(effect) = segment.default_effects.first_mut() else {
        return;
    };
    let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
        return;
    };
    let Some(move_to_zone) = tagged
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
    else {
        return;
    };
    if !matches!(move_to_zone.target.base(), ChooseSpec::Source)
        || move_to_zone.zone != crate::zone::Zone::Battlefield
    {
        return;
    }

    let mut replacement = move_to_zone.clone();
    replacement.target =
        ChooseSpec::Object(filter).with_count(crate::effect::ChoiceCount::exactly(1));
    *effect = Effect::new(crate::effects::TaggedEffect::new(
        tagged.tag.clone(),
        Effect::new(replacement),
    ));
}

fn dedupe_adjacent_target_only_effects(lowered: &mut LoweredEffects) {
    fn same_target_domain(left: &ChooseSpec, right: &ChooseSpec) -> bool {
        if left == right {
            return true;
        }
        matches!(
            (left, right),
            (plain, ChooseSpec::WithCount(counted, _))
                | (ChooseSpec::WithCount(counted, _), plain)
                if counted.as_ref() == plain
        )
    }

    fn dedupe_synthetic_targets_in_sequence(effect: &Effect) -> Effect {
        let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() else {
            return effect.clone();
        };
        let synthetic_targets = sequence
            .effects
            .iter()
            .filter_map(|effect| {
                effect
                    .downcast_ref::<crate::effects::TargetOnlyEffect>()
                    .filter(|target| !target.explicit_declaration)
            })
            .collect::<Vec<_>>();
        let Some(first) = synthetic_targets.first() else {
            return effect.clone();
        };
        if synthetic_targets.len() < 2
            || synthetic_targets
                .iter()
                .any(|target| target.target != first.target || target.chooser != first.chooser)
        {
            return effect.clone();
        }
        let mut retained_target = false;
        let mut rewritten = sequence.clone();
        rewritten.effects.retain(|effect| {
            let Some(target) = effect.downcast_ref::<crate::effects::TargetOnlyEffect>() else {
                return true;
            };
            if target.explicit_declaration
                || target.target != first.target
                || target.chooser != first.chooser
            {
                return true;
            }
            if retained_target {
                false
            } else {
                retained_target = true;
                true
            }
        });
        Effect::new(rewritten)
    }

    for segment in &mut lowered.effects.segments {
        for effect in &mut segment.default_effects {
            *effect = dedupe_synthetic_targets_in_sequence(effect);
        }
    }
    lowered.effects = crate::resolution::ResolutionProgram::new(lowered.effects.segments.clone());

    let flattened = lowered.effects.flattened_default_effects();
    if flattened.len() < 2 {
        return;
    }

    let mut rewritten = Vec::with_capacity(flattened.len());
    for effect in flattened {
        let duplicate_target_only = rewritten.last().and_then(|previous: &Effect| {
            let previous_target = previous.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            let current_target = effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
            same_target_domain(&previous_target.target, &current_target.target).then_some((
                previous_target.explicit_declaration,
                current_target.explicit_declaration,
            ))
        });
        match duplicate_target_only {
            Some((false, true)) => {
                // Presentation metadata must not create another executable
                // target choice. If either duplicate came from an authored
                // standalone declaration, retain that surface on the one
                // shared target effect.
                *rewritten.last_mut().expect("checked above") = effect.clone();
            }
            Some(_) => {}
            None => rewritten.push(effect.clone()),
        }
    }

    if rewritten.len() != flattened.len() {
        lowered.effects = crate::resolution::ResolutionProgram::from_effects(rewritten);
    }
}

fn strip_erroneous_meld_player_exile_effect(lowered: &mut LoweredEffects) {
    fn is_synthetic_meld_exile(effect: &Effect) -> bool {
        effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .is_some_and(|effect| {
                if effect.zone != crate::zone::Zone::Exile {
                    return false;
                }
                match &effect.target {
                    crate::target::ChooseSpec::Player(
                        crate::target::PlayerFilter::IteratedPlayer,
                    ) => true,
                    crate::target::ChooseSpec::Object(filter) => {
                        filter.zone == Some(crate::zone::Zone::Hand)
                            && filter.owner == Some(crate::target::PlayerFilter::You)
                            && {
                                let mut generic_hand_card = filter.clone();
                                generic_hand_card.zone = None;
                                generic_hand_card.owner = None;
                                generic_hand_card == crate::target::ObjectFilter::default()
                            }
                    }
                    _ => false,
                }
            })
    }

    fn strip_effect_list(effects: &mut Vec<Effect>) {
        for effect in effects.iter_mut() {
            if let Some(if_effect) = effect.downcast_ref::<crate::effects::IfEffect>() {
                let mut replacement = if_effect.clone();
                strip_effect_list(&mut replacement.then);
                strip_effect_list(&mut replacement.else_);
                *effect = Effect::new(replacement);
            } else if let Some(conditional) =
                effect.downcast_ref::<crate::effects::ConditionalEffect>()
            {
                let mut replacement = conditional.clone();
                strip_effect_list(&mut replacement.if_true);
                strip_effect_list(&mut replacement.if_false);
                *effect = Effect::new(replacement);
            } else if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
                let mut replacement = sequence.clone();
                strip_effect_list(&mut replacement.effects);
                *effect = Effect::new(replacement);
            }
        }

        let mut idx = 0usize;
        while idx + 1 < effects.len() {
            if is_synthetic_meld_exile(&effects[idx])
                && effects[idx + 1]
                    .downcast_ref::<crate::effects::MeldEffect>()
                    .is_some()
            {
                effects.remove(idx);
            } else {
                idx += 1;
            }
        }
    }

    let mut segments = lowered.effects.segments.clone();
    for segment in &mut segments {
        strip_effect_list(&mut segment.default_effects);
        for branch in &mut segment.self_replacements {
            strip_effect_list(&mut branch.replacement_effects);
        }
    }
    lowered.effects = crate::resolution::ResolutionProgram::new(segments);
}

fn fold_local_zone_change_self_replacements(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::new();
    let mut idx = 0usize;

    while idx < effects.len() {
        if idx + 1 < effects.len()
            && let Some(with_id) = effects[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(delayed) = with_id
                .effect
                .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            && let Some(if_effect) = effects[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && if_effect.condition == with_id.id
            && let [inner_effect] = delayed.effects.as_slice()
        {
            let mut delayed = delayed.clone();
            delayed.effects = vec![
                Effect::with_id(with_id.id.0, inner_effect.clone()),
                effects[idx + 1].clone(),
            ];
            rewritten.push(Effect::new(delayed));
            idx += 2;
            continue;
        }
        if idx + 1 < effects.len()
            && let Some(with_id) = effects[idx].downcast_ref::<crate::effects::WithIdEffect>()
            && let Some(if_effect) = effects[idx + 1].downcast_ref::<crate::effects::IfEffect>()
            && {
                #[cfg(not(feature = "serialization"))]
                {
                    if_effect.condition == with_id.id
                }
                #[cfg(feature = "serialization")]
                {
                    if_effect.condition == with_id.id
                }
            }
            && if_effect.predicate == EffectPredicate::Happened
            && if_effect.else_.is_empty()
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&if_effect.then, &with_id.effect)
        {
            rewritten.push(Effect::with_id(
                with_id.id.0,
                Effect::new(crate::effects::LocalRewriteEffect::new(
                    {
                        #[cfg(not(feature = "serialization"))]
                        {
                            (*with_id.effect).clone()
                        }
                        #[cfg(feature = "serialization")]
                        {
                            (*with_id.effect).clone()
                        }
                    },
                    zone_replacements,
                )),
            ));
            idx += 2;
            continue;
        }
        if idx + 1 < effects.len()
            && effects[idx].target_spec().is_some()
            && let Some(zone_replacements) =
                extract_local_zone_replacement_followups(&effects[idx + 1..idx + 2], &effects[idx])
            && zone_replacements.iter().all(|replacement| {
                replacement.from_zone == Some(crate::zone::Zone::Stack)
                    && replacement.to_zone == Some(crate::zone::Zone::Graveyard)
            })
        {
            rewritten.push(Effect::new(crate::effects::LocalRewriteEffect::new(
                effects[idx].clone(),
                zone_replacements,
            )));
            idx += 2;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }

    rewritten
}

fn strip_duplicate_self_replacement_prelude(
    default_effects: &[Effect],
    mut replacement_effects: Vec<Effect>,
) -> Vec<Effect> {
    let shared_prelude_len = default_effects
        .iter()
        .zip(replacement_effects.iter())
        .take_while(|(default, replacement)| same_resolution_prelude(default, replacement))
        .count();
    if shared_prelude_len > 0 {
        replacement_effects.drain(0..shared_prelude_len);
    }
    replacement_effects
}

fn same_resolution_prelude(left: &Effect, right: &Effect) -> bool {
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagAttachedToSourceEffect>(),
        right.downcast_ref::<crate::effects::TagAttachedToSourceEffect>(),
    ) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagTriggeringObjectEffect>(),
        right.downcast_ref::<crate::effects::TagTriggeringObjectEffect>(),
    ) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagTriggeringSourceEffect>(),
        right.downcast_ref::<crate::effects::TagTriggeringSourceEffect>(),
    ) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (
        left.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>(),
        right.downcast_ref::<crate::effects::TagTriggeringDamageTargetEffect>(),
    ) {
        return left == right;
    }
    false
}

fn extract_local_zone_replacement_followups(
    effects: &[Effect],
    antecedent: &Effect,
) -> Option<Vec<crate::effects::RegisterZoneReplacementEffect>> {
    let mut replacements = Vec::new();
    let antecedent_target = antecedent.target_spec().cloned();
    for effect in effects {
        let mut register = effect
            .downcast_ref::<crate::effects::RegisterZoneReplacementEffect>()?
            .clone();
        if register.mode != crate::effects::ReplacementApplyMode::OneShot {
            return None;
        }
        if choose_spec_contains_it_tag(&register.target)
            && let Some(target_spec) = &antecedent_target
        {
            register.target = target_spec.clone();
        }
        replacements.push(register);
    }
    Some(replacements)
}

fn normalize_two_target_counter_then_fight(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::with_capacity(effects.len());
    let mut idx = 0;
    while idx < effects.len() {
        if idx + 3 < effects.len()
            && let Some((first_tag, first_target)) = tagged_target_only(&effects[idx])
            && let Some((second_tag, second_target)) = tagged_target_only(&effects[idx + 1])
            && first_tag != second_tag
            && let Some(first_filter) = explicit_target_object_filter(first_target)
            && let Some(second_filter) = explicit_target_object_filter(second_target)
            && is_controlled_creature_fight_pair(first_filter, second_filter)
            && let Some((counter_tag, condition, counters)) =
                single_conditional_tagged_put_counters(&effects[idx + 2])
            && choose_spec_is_first_target(&counters.target, first_tag, first_filter)
            && fight_references_counter_tag(&effects[idx + 3], counter_tag.as_str())
        {
            let mut fixed_counters = counters.clone();
            fixed_counters.target = ChooseSpec::Tagged(first_tag.clone());
            let fixed_counter_effect = Effect::new(fixed_counters).tag(counter_tag.clone());
            rewritten.push(effects[idx].clone());
            rewritten.push(effects[idx + 1].clone());
            rewritten.push(Effect::conditional(
                condition.clone(),
                vec![fixed_counter_effect],
                Vec::new(),
            ));
            rewritten.push(Effect::fight(
                ChooseSpec::Tagged(first_tag.clone()),
                ChooseSpec::Tagged(second_tag.clone()),
            ));
            idx += 4;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }
    rewritten
}

fn normalize_two_target_conditional_then_fight(effects: Vec<Effect>) -> Vec<Effect> {
    // A conditional modifier exports its auto-generated result tag. Without this
    // repair, a following "those creatures fight each other" can resolve both
    // fighters to that one result instead of the two explicit target slots.
    let mut rewritten = Vec::with_capacity(effects.len());
    let mut idx = 0;
    while idx < effects.len() {
        if idx + 3 < effects.len()
            && let Some((first_tag, first_target)) = tagged_target_only(&effects[idx])
            && let Some((second_tag, second_target)) = tagged_target_only(&effects[idx + 1])
            && first_tag != second_tag
            && let Some(first_filter) = explicit_target_object_filter(first_target)
            && let Some(second_filter) = explicit_target_object_filter(second_target)
            && is_controlled_creature_fight_pair(first_filter, second_filter)
            && let Some(conditional) =
                effects[idx + 2].downcast_ref::<crate::effects::ConditionalEffect>()
            && conditional.if_false.is_empty()
            && !conditional.if_true.is_empty()
            && let Some(fight) = effects[idx + 3].downcast_ref::<crate::effects::FightEffect>()
            && conditional_result_is_both_fighters(conditional, fight)
            && let Some(fixed_branch) =
                normalize_conditional_branch_target(&conditional.if_true, first_tag, first_filter)
        {
            let link_target_condition = single_conditional_result_is_continuous(conditional);
            let fixed_condition = if link_target_condition {
                substitute_condition_tag(&conditional.condition, second_tag, first_tag)
            } else {
                conditional.condition.clone()
            };
            rewritten.push(effects[idx].clone());
            rewritten.push(effects[idx + 1].clone());
            rewritten.push(Effect::conditional(
                fixed_condition,
                fixed_branch,
                Vec::new(),
            ));
            rewritten.push(Effect::fight(
                ChooseSpec::Tagged(first_tag.clone()),
                ChooseSpec::Tagged(second_tag.clone()),
            ));
            idx += 4;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }
    rewritten
}

fn normalize_targeted_conditional_action_then_fight(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::with_capacity(effects.len());
    let mut idx = 0;
    while idx < effects.len() {
        if idx + 3 < effects.len()
            && let Some((friendly_tag, friendly_target)) = tagged_target_only(&effects[idx])
            && let Some((counter_tag, condition, counters)) =
                single_conditional_tagged_put_counters(&effects[idx + 1])
            && let Some(opposing_target) = untagged_target_only(&effects[idx + 2])
            && let Some(friendly_filter) = explicit_target_object_filter(friendly_target)
            && let Some(opposing_filter) = explicit_target_object_filter(opposing_target)
            && is_opposing_then_friendly_creature_pair(opposing_filter, friendly_filter)
            && choose_spec_is_first_target(&counters.target, friendly_tag, friendly_filter)
            && (fight_references_counter_tag(&effects[idx + 3], counter_tag.as_str())
                || fight_references_authored_it_and_target(&effects[idx + 3], opposing_target))
        {
            let opposing_tag = crate::tag::CompilerDerivedTag::OpposingTarget.key(friendly_tag);
            let mut fixed_counters = counters.clone();
            fixed_counters.target = ChooseSpec::Tagged(friendly_tag.clone());
            rewritten.push(effects[idx].clone());
            rewritten.push(
                Effect::new(crate::effects::TargetOnlyEffect::new(
                    opposing_target.clone(),
                ))
                .tag(opposing_tag.clone()),
            );
            rewritten.push(Effect::conditional(
                substitute_condition_tag(condition, counter_tag, friendly_tag),
                vec![Effect::new(fixed_counters).tag(counter_tag.clone())],
                Vec::new(),
            ));
            rewritten.push(Effect::fight(
                ChooseSpec::Tagged(friendly_tag.clone()),
                ChooseSpec::Tagged(opposing_tag),
            ));
            idx += 4;
            continue;
        }

        // Oracle commonly introduces the friendly target and its conditional
        // action in one sentence, then introduces the opposing target in the
        // following fight sentence:
        //
        //   [friendly target, conditional action, opposing target, fight]
        //
        // Canonicalize that authored order to the same four-effect shape used
        // by the renderer and legality checks. The positional ownership kept
        // by `normalize_cross_segment_fight_sequences` intentionally leaves
        // both target declarations in the first presentation segment.
        if idx + 3 < effects.len()
            && let Some((friendly_tag, friendly_target)) = tagged_target_only(&effects[idx])
            && let Some(conditional) =
                effects[idx + 1].downcast_ref::<crate::effects::ConditionalEffect>()
            && let Some(opposing_target) = untagged_target_only(&effects[idx + 2])
            && let Some(friendly_filter) = explicit_target_object_filter(friendly_target)
            && let Some(opposing_filter) = explicit_target_object_filter(opposing_target)
            && is_opposing_then_friendly_creature_pair(opposing_filter, friendly_filter)
            && conditional.if_false.is_empty()
            && matches!(
                &conditional.condition,
                Condition::TaggedObjectMatches(tag, _) if tag == friendly_tag
            )
            && let [branch_effect] = conditional.if_true.as_slice()
            && effect_outer_tag(branch_effect) == Some(friendly_tag)
            && let Some(fight) = effects[idx + 3].downcast_ref::<crate::effects::FightEffect>()
            && matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == friendly_tag)
            && &fight.creature2 == opposing_target
            && let Some(fixed_branch) = normalize_conditional_branch_target(
                &conditional.if_true,
                friendly_tag,
                friendly_filter,
            )
        {
            let opposing_tag = crate::tag::CompilerDerivedTag::OpposingTarget.key(friendly_tag);
            rewritten.push(
                Effect::new(crate::effects::TargetOnlyEffect::new(
                    opposing_target.clone(),
                ))
                .tag(opposing_tag.clone()),
            );
            rewritten.push(effects[idx].clone());
            rewritten.push(Effect::conditional(
                conditional.condition.clone(),
                fixed_branch,
                Vec::new(),
            ));
            rewritten.push(Effect::fight(
                ChooseSpec::Tagged(friendly_tag.clone()),
                ChooseSpec::Tagged(opposing_tag),
            ));
            idx += 4;
            continue;
        }

        if idx + 3 < effects.len()
            && let Some(opposing_target) = untagged_target_only(&effects[idx])
            && let Some((friendly_tag, friendly_target)) = tagged_target_only(&effects[idx + 1])
            && let Some(opposing_filter) = explicit_target_object_filter(opposing_target)
            && let Some(friendly_filter) = explicit_target_object_filter(friendly_target)
            && is_opposing_then_friendly_creature_pair(opposing_filter, friendly_filter)
            && let Some(conditional) =
                effects[idx + 2].downcast_ref::<crate::effects::ConditionalEffect>()
            && conditional.if_false.is_empty()
            && matches!(
                &conditional.condition,
                Condition::TaggedObjectMatches(tag, _) if tag == friendly_tag
            )
            && let [branch_effect] = conditional.if_true.as_slice()
            && effect_outer_tag(branch_effect) == Some(friendly_tag)
            && let Some(fight) = effects[idx + 3].downcast_ref::<crate::effects::FightEffect>()
            && matches!(&fight.creature1, ChooseSpec::Tagged(tag) if tag == friendly_tag)
            && &fight.creature2 == opposing_target
            && let Some(fixed_branch) = normalize_conditional_branch_target(
                &conditional.if_true,
                friendly_tag,
                friendly_filter,
            )
        {
            let opposing_tag = crate::tag::CompilerDerivedTag::OpposingTarget.key(friendly_tag);
            rewritten.push(
                Effect::new(crate::effects::TargetOnlyEffect::new(
                    opposing_target.clone(),
                ))
                .tag(opposing_tag.clone()),
            );
            rewritten.push(effects[idx + 1].clone());
            rewritten.push(Effect::conditional(
                conditional.condition.clone(),
                fixed_branch,
                Vec::new(),
            ));
            rewritten.push(Effect::fight(
                ChooseSpec::Tagged(friendly_tag.clone()),
                ChooseSpec::Tagged(opposing_tag),
            ));
            idx += 4;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }
    rewritten
}

fn untagged_target_only(effect: &Effect) -> Option<&ChooseSpec> {
    effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
        .map(|target_only| &target_only.target)
}

fn explicit_target_object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    let ChooseSpec::Target(inner) = spec else {
        return None;
    };
    let ChooseSpec::Object(filter) = inner.as_ref() else {
        return None;
    };
    Some(filter)
}

fn is_controlled_creature_fight_pair(first: &ObjectFilter, second: &ObjectFilter) -> bool {
    first.zone == Some(crate::zone::Zone::Battlefield)
        && second.zone == Some(crate::zone::Zone::Battlefield)
        && matches!(
            first.card_types.as_slice(),
            [crate::types::CardType::Creature]
        )
        && matches!(
            second.card_types.as_slice(),
            [crate::types::CardType::Creature]
        )
        && first.controller == Some(crate::target::PlayerFilter::You)
        && matches!(
            second.controller,
            Some(crate::target::PlayerFilter::NotYou | crate::target::PlayerFilter::Opponent)
        )
}

fn is_opposing_then_friendly_creature_pair(
    opposing: &ObjectFilter,
    friendly: &ObjectFilter,
) -> bool {
    opposing.zone == Some(crate::zone::Zone::Battlefield)
        && friendly.zone == Some(crate::zone::Zone::Battlefield)
        && matches!(
            opposing.card_types.as_slice(),
            [crate::types::CardType::Creature]
        )
        && matches!(
            friendly.card_types.as_slice(),
            [crate::types::CardType::Creature]
        )
        && matches!(
            &opposing.controller,
            Some(crate::target::PlayerFilter::NotYou) | Some(crate::target::PlayerFilter::Opponent)
        )
        && friendly.controller == Some(crate::target::PlayerFilter::You)
}

fn object_filters_match_ignoring_reference_tags(left: &ObjectFilter, right: &ObjectFilter) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    left.tagged_constraints.clear();
    right.tagged_constraints.clear();
    left == right
}

fn choose_spec_is_first_target(
    spec: &ChooseSpec,
    first_tag: &TagKey,
    first_filter: &ObjectFilter,
) -> bool {
    match spec {
        ChooseSpec::Tagged(tag) => tag == first_tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filters_match_ignoring_reference_tags(filter, first_filter)
        }
        ChooseSpec::SurfaceHinted { spec: inner, .. }
        | ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => {
            choose_spec_is_first_target(inner, first_tag, first_filter)
        }
        _ => false,
    }
}

fn normalize_conditional_branch_target(
    effects: &[Effect],
    first_tag: &TagKey,
    first_filter: &ObjectFilter,
) -> Option<Vec<Effect>> {
    effects
        .iter()
        .map(|effect| normalize_conditional_action_target(effect, first_tag, first_filter))
        .collect()
}

fn normalize_conditional_action_target(
    effect: &Effect,
    first_tag: &TagKey,
    first_filter: &ObjectFilter,
) -> Option<Effect> {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return normalize_conditional_action_target(&tagged.effect, first_tag, first_filter)
            .map(|effect| effect.tag(tagged.tag.clone()));
    }

    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut fixed = sequence.clone();
        fixed.effects =
            normalize_conditional_branch_target(&sequence.effects, first_tag, first_filter)?;
        return Some(Effect::new(fixed));
    }

    if let Some(counters) = effect.downcast_ref::<crate::effects::PutCountersEffect>() {
        if !choose_spec_is_first_target(&counters.target, first_tag, first_filter) {
            return None;
        }
        let mut fixed = counters.clone();
        fixed.target = ChooseSpec::Tagged(first_tag.clone());
        return Some(Effect::new(fixed));
    }

    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        let targets_first = apply
            .target_spec
            .as_ref()
            .is_some_and(|spec| choose_spec_is_first_target(spec, first_tag, first_filter))
            || matches!(
                &apply.target,
                crate::continuous::EffectTarget::Filter(filter)
                    if object_filters_match_ignoring_reference_tags(filter, first_filter)
            );
        if !targets_first {
            return None;
        }
        let mut fixed = apply.clone();
        fixed.target_spec = Some(ChooseSpec::Tagged(first_tag.clone()));
        return Some(Effect::new(fixed));
    }

    None
}

fn effect_outer_tag(effect: &Effect) -> Option<&TagKey> {
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map(|tagged| &tagged.tag)
}

fn collect_effect_result_tags<'a>(effect: &'a Effect, tags: &mut Vec<&'a TagKey>) {
    if let Some(tag) = effect_outer_tag(effect) {
        tags.push(tag);
        return;
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        for effect in &sequence.effects {
            collect_effect_result_tags(effect, tags);
        }
    }
}

fn conditional_result_is_both_fighters(
    conditional: &crate::effects::ConditionalEffect,
    fight: &crate::effects::FightEffect,
) -> bool {
    let [effect] = conditional.if_true.as_slice() else {
        return false;
    };
    let mut result_tags = Vec::new();
    collect_effect_result_tags(effect, &mut result_tags);
    result_tags
        .into_iter()
        .any(|tag| fight_references_result_tag_or_authored_group(fight, tag.as_str()))
}

fn choose_spec_has_authored_creature_group_surface(spec: &ChooseSpec) -> bool {
    spec.source_reference_surface().is_some_and(|surface| {
        matches!(
            surface.display_text().trim().to_ascii_lowercase().as_str(),
            "those creatures" | "the chosen creatures"
        )
    })
}

fn fight_references_result_tag_or_authored_group(
    fight: &crate::effects::FightEffect,
    result_tag: &str,
) -> bool {
    let first_is_result = choose_spec_references_tag(&fight.creature1, result_tag);
    let second_is_result = choose_spec_references_tag(&fight.creature2, result_tag);
    let first_is_authored_group = choose_spec_has_authored_creature_group_surface(&fight.creature1)
        || choose_spec_references_tag(
            &fight.creature1,
            crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
        );
    let second_is_authored_group =
        choose_spec_has_authored_creature_group_surface(&fight.creature2)
            || choose_spec_references_tag(
                &fight.creature2,
                crate::tag::CompilerReferenceTag::ChosenObjects.as_str(),
            );
    (first_is_result && (second_is_result || second_is_authored_group))
        || (first_is_authored_group && second_is_result)
        || (first_is_authored_group && second_is_authored_group)
}

fn single_conditional_result_is_continuous(
    conditional: &crate::effects::ConditionalEffect,
) -> bool {
    let [effect] = conditional.if_true.as_slice() else {
        return false;
    };
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .and_then(|tagged| {
            tagged
                .effect
                .downcast_ref::<crate::effects::ApplyContinuousEffect>()
        })
        .is_some()
}

fn substitute_condition_tag(
    condition: &Condition,
    from_tag: &TagKey,
    to_tag: &TagKey,
) -> Condition {
    match condition {
        // Coordinated target declarations carry an authored collection tag
        // outside their independent target-slot tags. A singular trailing
        // predicate ("if it's a Knight") belongs to the creature receiving
        // the conditional action, not to that synthetic collection.
        Condition::TaggedObjectMatches(tag, filter)
            if tag == from_tag
                || tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str() =>
        {
            Condition::TaggedObjectMatches(to_tag.clone(), filter.clone())
        }
        _ => condition.clone(),
    }
}

fn tagged_target_only(effect: &Effect) -> Option<(&TagKey, &ChooseSpec)> {
    let tagged = effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if let Some(target_only) = tagged
        .effect
        .downcast_ref::<crate::effects::TargetOnlyEffect>()
    {
        return Some((&tagged.tag, &target_only.target));
    }

    // A coordinated target declaration may carry a shared collection tag
    // outside each independently targeted object. The inner tag is the
    // actual target slot consumed by later actions; using the shared wrapper
    // makes both targets appear identical and prevents fight-reference repair.
    tagged_target_only(&tagged.effect)
}

fn random_single_tagged_destroy_tag(destroy: &crate::effects::DestroyEffect) -> Option<&TagKey> {
    let ChooseSpec::WithCount(inner, count) = &destroy.spec else {
        return None;
    };
    if !count.is_single() || !count.is_random() {
        return None;
    }
    match inner.as_ref() {
        ChooseSpec::Tagged(tag) => Some(tag),
        _ => None,
    }
}

fn any_of_tagged_objects(tags: &[&TagKey]) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.any_of = tags
        .iter()
        .map(|tag| ObjectFilter::tagged((*tag).clone()))
        .collect();
    filter
}

fn normalize_random_destroy_across_target_groups(effects: Vec<Effect>) -> Vec<Effect> {
    let mut rewritten = Vec::with_capacity(effects.len());
    let mut idx = 0usize;
    while idx < effects.len() {
        if idx + 2 < effects.len()
            && let Some((first_tag, _)) = tagged_target_only(&effects[idx])
            && let Some((second_tag, _)) = tagged_target_only(&effects[idx + 1])
            && first_tag != second_tag
            && let Some(destroy) = effects[idx + 2].downcast_ref::<crate::effects::DestroyEffect>()
            && random_single_tagged_destroy_tag(destroy) == Some(second_tag)
            && let ChooseSpec::WithCount(_, count) = &destroy.spec
        {
            let target = ChooseSpec::WithCount(
                Box::new(ChooseSpec::Object(any_of_tagged_objects(&[
                    first_tag, second_tag,
                ]))),
                *count,
            );
            rewritten.push(effects[idx].clone());
            rewritten.push(effects[idx + 1].clone());
            rewritten.push(Effect::new(crate::effects::DestroyEffect::with_spec(
                target,
            )));
            idx += 3;
            continue;
        }

        rewritten.push(effects[idx].clone());
        idx += 1;
    }
    rewritten
}

fn single_conditional_tagged_put_counters(
    effect: &Effect,
) -> Option<(&TagKey, &Condition, &crate::effects::PutCountersEffect)> {
    let conditional = effect.downcast_ref::<crate::effects::ConditionalEffect>()?;
    if !conditional.if_false.is_empty() || conditional.if_true.len() != 1 {
        return None;
    }
    let tagged = conditional.if_true[0].downcast_ref::<crate::effects::TaggedEffect>()?;
    let counters = tagged
        .effect
        .downcast_ref::<crate::effects::PutCountersEffect>()?;
    Some((&tagged.tag, &conditional.condition, counters))
}

fn fight_references_counter_tag(effect: &Effect, tag: &str) -> bool {
    let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() else {
        return false;
    };
    fight_references_result_tag_or_authored_group(fight, tag)
}

fn fight_references_authored_it_and_target(effect: &Effect, target: &ChooseSpec) -> bool {
    let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() else {
        return false;
    };
    matches!(fight.creature1.base(), ChooseSpec::Source)
        && fight
            .creature1
            .source_reference_surface()
            .is_some_and(|surface| surface.display_text().trim().eq_ignore_ascii_case("it"))
        && &fight.creature2 == target
}

fn choose_spec_references_tag(spec: &ChooseSpec, tag: &str) -> bool {
    match spec {
        ChooseSpec::Tagged(candidate) => candidate.as_str() == tag,
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == tag),
        ChooseSpec::SurfaceHinted { spec: inner, .. }
        | ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => choose_spec_references_tag(inner, tag),
        _ => false,
    }
}

fn choose_spec_contains_it_tag(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(tag) => tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str(),
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            choose_spec_contains_it_tag(inner)
        }
        _ => false,
    }
}

pub fn compile_effect_prelude_tags(prelude: &[EffectPreludeTag]) -> Vec<Effect> {
    prelude
        .iter()
        .map(|tag| match tag {
            EffectPreludeTag::AttachedSource(tag) => Effect::tag_attached_to_source(tag.as_str()),
            EffectPreludeTag::TriggeringObject(tag) => Effect::tag_triggering_object(tag.as_str()),
            EffectPreludeTag::TriggeringAttacker(tag, filter) => {
                Effect::tag_triggering_attacker(tag.as_str(), Some(filter.clone()))
            }
            EffectPreludeTag::TriggeringBlockers(tag, filter) => {
                Effect::tag_triggering_blockers(tag.as_str(), Some(filter.clone()))
            }
            EffectPreludeTag::OtherBlockParticipant(tag, filter) => {
                Effect::tag_other_block_participant(tag.as_str(), Some(filter.clone()))
            }
            EffectPreludeTag::OtherBlockParticipantMatchingSubject {
                tag,
                subject,
                other,
            } => Effect::tag_other_block_participant_matching_subject(
                tag.as_str(),
                subject.clone(),
                other.clone(),
            ),
            EffectPreludeTag::TriggeringSource(tag) => Effect::tag_triggering_source(tag.as_str()),
            EffectPreludeTag::TriggeringDamageTarget(tag) => {
                Effect::tag_triggering_damage_target(tag.as_str())
            }
        })
        .collect()
}

pub fn compile_condition_from_predicate_ast_with_env(
    predicate: &PredicateAst,
    refs: &ReferenceEnv,
    saved_last_object_tag: Option<&TagKey>,
) -> Result<Condition, CardTextError> {
    crate::reference_resolution_support::resolve_condition_from_predicate(
        predicate,
        refs,
        &saved_last_object_tag.cloned(),
    )
}

pub fn compile_prepared_predicate_for_lowering(
    prepared: &PreparedPredicateForLowering,
) -> Result<Condition, CardTextError> {
    compile_condition_from_predicate_ast_with_env(
        &prepared.predicate,
        &prepared.reference_env,
        prepared.saved_last_object_tag.as_ref(),
    )
}

fn prepend_effect_prelude(mut compiled: Vec<Effect>, mut prelude: Vec<Effect>) -> Vec<Effect> {
    if prelude.is_empty() {
        return compiled;
    }
    prelude.append(&mut compiled);
    prelude
}

#[cfg(test)]
mod plural_result_reference_tests {
    use super::*;

    fn tagged_slot(tag: &str) -> Effect {
        Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::Object(
            ObjectFilter::creature(),
        )))
        .tag(tag)
    }

    fn plural_followup(tag: &str) -> Effect {
        Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                ChooseSpec::Tagged(ironsmith_compiler_semantic::tag::declared_key(tag)),
                crate::continuous::Modification::AddCardTypes(vec![
                    crate::types::CardType::Creature,
                ]),
                crate::effect::Until::Forever,
            )
            .with_set_quantifier_surface(Some(ironsmith_core::SetQuantifierSurface::They)),
        )
    }

    #[test]
    fn plural_pronoun_unions_only_coordinated_result_slots() {
        let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            tagged_slot("first"),
            tagged_slot("second"),
        ]));
        let normalized = normalize_plural_coordinated_result_references(vec![
            coordinated,
            plural_followup("second"),
        ]);
        let apply = normalized[1]
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("plural follow-up");
        let Some(ChooseSpec::Object(filter)) = apply.target_spec.as_ref() else {
            panic!("coordinated plural result must lower to an exact union: {apply:#?}");
        };
        assert_eq!(filter.any_of.len(), 2);

        let sequential = Effect::new(crate::effects::SequenceEffect::new(vec![
            tagged_slot("first"),
            tagged_slot("second"),
        ]));
        let untouched = normalize_plural_coordinated_result_references(vec![
            sequential,
            plural_followup("second"),
        ]);
        let apply = untouched[1]
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("singular sequential follow-up");
        assert!(matches!(
            apply.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "second"
        ));
    }

    #[test]
    fn plural_pronoun_unions_coordinated_results_across_sentence_segments_only() {
        let coordinated = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            tagged_slot("first"),
            tagged_slot("second"),
        ]));
        let mut segments = vec![
            crate::resolution::ResolutionSegment::from_effects(vec![coordinated.clone()]),
            crate::resolution::ResolutionSegment::from_effects(vec![plural_followup("second")]),
        ];
        normalize_cross_segment_plural_coordinated_result_references(&mut segments);
        let apply = segments[1].default_effects[0]
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("plural follow-up");
        let Some(ChooseSpec::Object(filter)) = apply.target_spec.as_ref() else {
            panic!("cross-sentence plural result must lower to an exact union: {apply:#?}");
        };
        assert_eq!(filter.any_of.len(), 2);

        let mut singular = plural_followup("second");
        let apply = singular
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("singular follow-up");
        singular = Effect::new(apply.clone().with_set_quantifier_surface(None));
        let mut segments = vec![
            crate::resolution::ResolutionSegment::from_effects(vec![coordinated]),
            crate::resolution::ResolutionSegment::from_effects(vec![singular]),
        ];
        normalize_cross_segment_plural_coordinated_result_references(&mut segments);
        let apply = segments[1].default_effects[0]
            .downcast_ref::<crate::effects::ApplyContinuousEffect>()
            .expect("singular follow-up");
        assert!(matches!(
            apply.target_spec.as_ref().map(ChooseSpec::base),
            Some(ChooseSpec::Tagged(tag)) if tag.as_str() == "second"
        ));
    }
}

#[cfg(test)]
mod counter_rewrite_mode_tests {
    use super::*;

    fn counter_then_replacement(
        mode: crate::effects::ReplacementApplyMode,
    ) -> Vec<crate::resolution::ResolutionSegment> {
        let tag = ironsmith_compiler_semantic::tag::declared_key("countered");
        let mut spell = ObjectFilter::default().in_zone(crate::zone::Zone::Stack);
        spell.stack_kind = Some(crate::filter::StackObjectKind::Spell);
        let target = ChooseSpec::target(ChooseSpec::Object(spell));
        let counter = Effect::counter(target).tag(tag.clone());
        let replacement = Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
            ChooseSpec::Tagged(tag),
            Some(crate::zone::Zone::Stack),
            Some(crate::zone::Zone::Graveyard),
            crate::zone::Zone::Exile,
            mode,
        ));
        vec![
            crate::resolution::ResolutionSegment::from_effects(vec![counter]),
            crate::resolution::ResolutionSegment::from_effects(vec![replacement]),
        ]
    }

    #[test]
    fn one_shot_counter_destination_folds_into_local_rewrite() {
        let mut segments = counter_then_replacement(crate::effects::ReplacementApplyMode::OneShot);
        fold_cross_segment_counter_rewrites(&mut segments);

        assert_eq!(segments.len(), 1);
        assert!(
            segments[0].default_effects[0]
                .downcast_ref::<crate::effects::LocalRewriteEffect>()
                .is_some(),
            "a counter destination replacement is scoped to the counter action: {segments:#?}"
        );
    }

    #[test]
    fn turn_scoped_replacement_is_not_misclassified_as_counter_rewrite() {
        let mut segments =
            counter_then_replacement(crate::effects::ReplacementApplyMode::UntilEndOfTurn);
        fold_cross_segment_counter_rewrites(&mut segments);
        assert_eq!(segments.len(), 2);
    }
}

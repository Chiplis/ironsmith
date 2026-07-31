use super::*;

#[path = "effect_dispatch/subject_verb_early.rs"]
mod subject_verb_early;
#[path = "effect_dispatch/subject_verb_late.rs"]
mod subject_verb_late;
#[path = "effect_dispatch/subject_verb_middle.rs"]
mod subject_verb_middle;

use subject_verb_early::compile_subject_verb_early;
use subject_verb_late::compile_subject_verb_late;
use subject_verb_middle::compile_subject_verb_middle;

type EffectCompileOutcome = (Vec<Effect>, Vec<ChooseSpec>);
type EffectCompileHandler = fn(
    &EffectAst,
    &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError>;

#[derive(Clone, Copy)]
struct EffectCompileHandlerDef {
    run: EffectCompileHandler,
}

fn with_target_count_preserving_value(spec: ChooseSpec, count: ChoiceCount) -> ChooseSpec {
    if let Some(value) = spec.count_value().cloned() {
        spec.with_count_value(count, value)
    } else {
        spec.with_count(count)
    }
}

#[derive(Clone, Copy)]
enum NestedResultReferenceKind {
    Outcome,
    Metric(ironsmith_core::EffectMetricSource),
    PriorMetric {
        source: ironsmith_core::EffectMetricSource,
        action: Option<ironsmith_core::PriorEffectAction>,
    },
}

#[derive(Clone, Copy)]
struct NestedResultReference {
    id: EffectId,
    kind: NestedResultReferenceKind,
}

fn collect_nested_result_references(value: &Value, references: &mut Vec<NestedResultReference>) {
    let reference = match value {
        Value::EffectValue(id) | Value::EffectValueOffset(id, _) => Some(NestedResultReference {
            id: *id,
            kind: NestedResultReferenceKind::Outcome,
        }),
        Value::EffectMetric {
            effect_id, source, ..
        }
        | Value::EffectMetricOffset {
            effect_id, source, ..
        } => Some(NestedResultReference {
            id: *effect_id,
            kind: NestedResultReferenceKind::Metric(*source),
        }),
        Value::PriorEffectMetric { effect_id, query } => Some(NestedResultReference {
            id: *effect_id,
            kind: NestedResultReferenceKind::PriorMetric {
                source: query.source,
                action: query.action,
            },
        }),
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => {
            collect_nested_result_references(value, references);
            None
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            collect_nested_result_references(left, references);
            collect_nested_result_references(right, references);
            None
        }
        _ => None,
    };
    if let Some(reference) = reference
        && !references
            .iter()
            .any(|existing| existing.id == reference.id)
    {
        references.push(reference);
    }
}

fn visit_direct_nested_effect_values(effect: &Effect, visit: &mut impl FnMut(&Value)) {
    if let Some(with_id) = effect.as_with_id() {
        visit_direct_nested_effect_values(&with_id.effect, visit);
        return;
    }
    if let Some(tagged) = effect.as_tagged() {
        visit_direct_nested_effect_values(&tagged.effect, visit);
        return;
    }

    if let Some(count_value) = effect.target_spec().and_then(|spec| spec.count_value()) {
        visit(count_value);
    }
    if let Some(modal) = effect.as_choose_mode() {
        for value in [
            &modal.min,
            &modal.max,
            &modal.choose_count,
            &modal.min_choose_count,
        ] {
            visit(value);
        }
    }

    macro_rules! value_field {
        ($type:ty, $field:ident) => {
            if let Some(value_effect) = effect.downcast_ref::<$type>() {
                visit(&value_effect.$field);
            }
        };
    }
    value_field!(crate::effects::DealDamageEffect, amount);
    value_field!(crate::effects::DrawCardsEffect, count);
    value_field!(crate::effects::PutCountersEffect, amount);
    value_field!(crate::effects::RemoveCountersEffect, count);
    value_field!(crate::effects::RepeatEffectsEffect, count);
    value_field!(crate::effects::CreateTokenEffect, count);
    value_field!(crate::effects::CreateTokenCopyEffect, count);
    value_field!(crate::effects::DiscardEffect, count);
    value_field!(crate::effects::MillEffect, count);
    value_field!(crate::effects::ScryEffect, count);
    value_field!(crate::effects::SurveilEffect, count);
    value_field!(crate::effects::FatesealEffect, count);
    value_field!(crate::effects::ExileTopOfLibraryEffect, count);
    value_field!(crate::effects::InvestigateEffect, count);
    value_field!(crate::effects::GainLifeEffect, amount);
    value_field!(crate::effects::LoseLifeEffect, amount);
    value_field!(crate::effects::SetLifeTotalEffect, amount);
    value_field!(crate::effects::PoisonCountersEffect, count);
    value_field!(crate::effects::AdditionalLandPlaysEffect, count);
    value_field!(crate::effects::PreventDamageEffect, amount);
    value_field!(crate::effects::AddManaOfLandProducedTypesEffect, amount);
    value_field!(crate::effects::ConniveEffect, count);
    value_field!(crate::effects::RemoveUpToCountersEffect, max_count);
    value_field!(crate::effects::mana::AddScaledManaEffect, amount);

    if let Some(incubate) = effect.downcast_ref::<crate::effects::IncubateEffect>() {
        visit(&incubate.amount);
        visit(&incubate.count);
    }
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && let Some(count) = &choose.count_value
    {
        visit(count);
    }
    if let Some(search) = effect.downcast_ref::<crate::effects::SearchLibraryEffect>()
        && let Some(position) = &search.library_position_from_top
    {
        visit(position);
    }
}

fn direct_nested_effect_result_references(effect: &Effect) -> Vec<NestedResultReference> {
    let mut references = Vec::new();
    visit_direct_nested_effect_values(effect, &mut |value| {
        collect_nested_result_references(value, &mut references);
    });
    references
}

fn nested_effect_is_exile(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::ExileEffect>()
        .is_some()
    {
        return true;
    }
    if let Some(move_to_zone) = effect.downcast_ref::<crate::effects::MoveToZoneEffect>()
        && move_to_zone.zone == Zone::Exile
    {
        return true;
    }
    if let Some(with_id) = effect.as_with_id() {
        return nested_effect_is_exile(&with_id.effect);
    }
    if let Some(tagged) = effect.as_tagged() {
        return nested_effect_is_exile(&tagged.effect);
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && nested_effect_is_exile(child) {
            found = true;
        }
    });
    found
}

fn nested_effect_is_move_to_zone(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()
        .is_some()
    {
        return true;
    }
    if let Some(with_id) = effect.as_with_id() {
        return nested_effect_is_move_to_zone(&with_id.effect);
    }
    if let Some(tagged) = effect.as_tagged() {
        return nested_effect_is_move_to_zone(&tagged.effect);
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && nested_effect_is_move_to_zone(child) {
            found = true;
        }
    });
    found
}

fn nested_effect_defines_result_id(effect: &Effect, id: EffectId) -> bool {
    if let Some(with_id) = effect.as_with_id() {
        if with_id.id == id {
            return true;
        }
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && nested_effect_defines_result_id(child, id) {
            found = true;
        }
    });
    found
}

fn nested_effect_can_produce_reference(effect: &Effect, reference: NestedResultReference) -> bool {
    match reference.kind {
        // Move-to-zone effects report the number of objects they actually
        // moved as their scalar outcome. Search/exile procedures are commonly
        // lowered behind a sequence wrapper, so accept that transparent
        // aggregate just as we already accept nested mana producers.
        NestedResultReferenceKind::Outcome => {
            effect.contains_mana_production() || nested_effect_is_move_to_zone(effect)
        }
        NestedResultReferenceKind::Metric(ironsmith_core::EffectMetricSource::AffectedObjects) => {
            nested_effect_is_move_to_zone(effect)
        }
        NestedResultReferenceKind::PriorMetric {
            action: Some(ironsmith_core::PriorEffectAction::Exiled),
            ..
        } => nested_effect_is_exile(effect),
        NestedResultReferenceKind::PriorMetric {
            source: ironsmith_core::EffectMetricSource::AffectedObjects,
            action: None,
        } => nested_effect_is_move_to_zone(effect) || nested_effect_is_exile(effect),
        _ => false,
    }
}

/// Reference annotation can resolve a pending result value before a typed
/// sequence is lowered recursively. The second annotation pass sees an
/// already-resolved ID and therefore does not assign that ID to the nested
/// producer again. Restore the adjacent producer wrapper for result shapes
/// whose runtime producer is unambiguous.
fn preserve_nested_result_value_links(effects: &mut [Effect]) {
    for consumer_index in 1..effects.len() {
        let references = direct_nested_effect_result_references(&effects[consumer_index]);
        let producer_index = consumer_index - 1;
        for reference in references {
            if effects[..consumer_index]
                .iter()
                .any(|effect| nested_effect_defines_result_id(effect, reference.id))
            {
                continue;
            }
            if nested_effect_can_produce_reference(&effects[producer_index], reference) {
                effects[producer_index] =
                    Effect::with_id(reference.id.0, effects[producer_index].clone());
                break;
            }
        }
    }
}

fn lower_source_top_only_choice(
    spec: &ChooseSpec,
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    tag_prefix: &str,
) -> Result<(Effect, ChooseSpec), CardTextError> {
    let ChooseSpec::Object(source_filter) = spec.base() else {
        return Err(CardTextError::ParseError(
            "top-of-zone selection requires an object filter".to_string(),
        ));
    };
    let mut filter = source_filter.clone();
    match filter.zone {
        Some(Zone::Graveyard | Zone::Library) => {}
        // This helper is reached only from an AST carrying the lexically
        // proven `source_top_only` marker. A bare "the top N cards" therefore
        // has the same ordered-library source as the explicit "of your
        // library" form; do not make that default available to ordinary
        // object choices.
        None => filter.zone = Some(Zone::Library),
        Some(_) => {
            return Err(CardTextError::ParseError(
                "top-of-zone selection requires an ordered graveyard or library source".to_string(),
            ));
        }
    }
    let chooser = match player {
        PlayerAst::Target => PlayerFilter::Target(Box::new(PlayerFilter::Any)),
        PlayerAst::TargetOpponent => PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
        player => resolve_non_target_player_filter(player, &current_reference_env(ctx))?,
    };
    let tag = ctx.next_tag(tag_prefix);
    ctx.last_object_tag = Some(tag.clone());
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(filter, spec.count(), chooser, tag.clone())
            .with_count_value_opt(spec.count_value().cloned())
            .top_only(),
    );
    Ok((choose, ChooseSpec::tagged(tag)))
}

fn coordinated_introduced_target(effect: &Effect) -> Option<(TagKey, ChooseSpec)> {
    if let Some(with_id) = effect.as_with_id() {
        return coordinated_introduced_target(&with_id.effect);
    }

    let tagged = effect.as_tagged()?;
    let spec = tagged.effect.target_spec()?;
    spec.is_target().then(|| (tagged.tag.clone(), spec.clone()))
}

fn preserve_independent_coordinated_targets(
    mut effects: Vec<Effect>,
    choices: Vec<ChooseSpec>,
) -> (Vec<Effect>, Vec<ChooseSpec>) {
    let mut introduced = Vec::<(TagKey, ChooseSpec)>::new();
    for effect in &effects {
        let Some((tag, spec)) = coordinated_introduced_target(effect) else {
            continue;
        };
        if !introduced.iter().any(|(seen, _)| seen == &tag) {
            introduced.push((tag, spec));
        }
    }

    let has_repeated_spec = introduced
        .iter()
        .enumerate()
        .any(|(idx, (_, spec))| introduced[idx + 1..].iter().any(|(_, later)| later == spec));
    if !has_repeated_spec {
        return (effects, choices);
    }

    // `compile_effects` normally collapses equal choices and compensates by
    // inserting an untagged TargetOnly prelude. In a coordinated clause, two
    // distinctly tagged effects with equal-looking specs introduce two target
    // slots instead. Keep those occurrences and discard only that synthetic
    // prelude; explicit target-only clauses are tagged by the same annotation
    // pass and therefore remain wrapped.
    effects.retain(|effect| {
        effect.as_target_only().is_none_or(|target_only| {
            introduced
                .iter()
                .filter(|(_, spec)| spec == &target_only.target)
                .count()
                < 2
        })
    });

    let mut preserved = introduced
        .iter()
        .map(|(_, spec)| spec.clone())
        .collect::<Vec<_>>();
    for choice in choices {
        if !introduced.iter().any(|(_, spec)| spec == &choice) {
            push_choice(&mut preserved, choice);
        }
    }
    (effects, preserved)
}

fn describe_value_for_mode(value: &Value) -> String {
    match value {
        Value::Fixed(amount) => amount.to_string(),
        Value::X => "X".to_string(),
        _ => "that many".to_string(),
    }
}

pub(super) fn bind_source_value_to_damage_source(value: &Value, source: &ChooseSpec) -> Value {
    match value {
        Value::SourcePower => Value::PowerOf(Box::new(source.clone())),
        Value::SourceToughness => Value::ToughnessOf(Box::new(source.clone())),
        Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Source) => {
            Value::PowerOf(Box::new(source.clone()))
        }
        Value::ToughnessOf(spec) if matches!(spec.base(), ChooseSpec::Source) => {
            Value::ToughnessOf(Box::new(source.clone()))
        }
        Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Source) => {
            Value::ManaValueOf(Box::new(source.clone()))
        }
        Value::CountersOn(spec, counter_type) if matches!(spec.base(), ChooseSpec::Source) => {
            Value::CountersOn(Box::new(source.clone()), *counter_type)
        }
        Value::ManaSymbolsInManaCostOf { spec, color }
            if matches!(spec.base(), ChooseSpec::Source) =>
        {
            Value::ManaSymbolsInManaCostOf {
                spec: Box::new(source.clone()),
                color: *color,
            }
        }
        Value::Add(left, right) => Value::Add(
            Box::new(bind_source_value_to_damage_source(left, source)),
            Box::new(bind_source_value_to_damage_source(right, source)),
        ),
        Value::Scaled(inner, multiplier) => Value::Scaled(
            Box::new(bind_source_value_to_damage_source(inner, source)),
            *multiplier,
        ),
        Value::DividedRoundedDown(inner, divisor) => Value::DividedRoundedDown(
            Box::new(bind_source_value_to_damage_source(inner, source)),
            *divisor,
        ),
        Value::HalfRoundedDown(inner) => {
            Value::HalfRoundedDown(Box::new(bind_source_value_to_damage_source(inner, source)))
        }
        Value::Min(left, right) => Value::Min(
            Box::new(bind_source_value_to_damage_source(left, source)),
            Box::new(bind_source_value_to_damage_source(right, source)),
        ),
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_source_value_to_damage_source(value, source)),
            hints: hints.clone(),
        },
        _ => value.clone(),
    }
}

fn bind_explicit_that_card_token_stat_reference(
    value: &Value,
    ctx: &EffectLoweringContext,
) -> Value {
    let reference_tag = ctx
        .last_exiled_collection_tag
        .as_deref()
        .map(TagKey::from)
        .or_else(|| current_reference_env(ctx).known_last_object_tag().cloned());
    let bind_spec = |spec: &ChooseSpec| {
        if matches!(
            spec.base(),
            ChooseSpec::Tagged(tag)
                if tag.as_str()
                    == crate::runtime_backend::token_definition::TOKEN_DYNAMIC_THAT_CARD_TAG
        ) {
            reference_tag
                .as_ref()
                .map(|tag| ChooseSpec::Tagged(tag.clone()))
                .unwrap_or_else(|| spec.clone())
        } else {
            spec.clone()
        }
    };
    match value {
        Value::PowerOf(spec) => Value::PowerOf(Box::new(bind_spec(spec))),
        Value::ToughnessOf(spec) => Value::ToughnessOf(Box::new(bind_spec(spec))),
        Value::ManaSymbolsInManaCostOf { spec, color } => Value::ManaSymbolsInManaCostOf {
            spec: Box::new(bind_spec(spec)),
            color: *color,
        },
        Value::Add(left, right) => Value::Add(
            Box::new(bind_explicit_that_card_token_stat_reference(left, ctx)),
            Box::new(bind_explicit_that_card_token_stat_reference(right, ctx)),
        ),
        Value::Scaled(inner, multiplier) => Value::Scaled(
            Box::new(bind_explicit_that_card_token_stat_reference(inner, ctx)),
            *multiplier,
        ),
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_explicit_that_card_token_stat_reference(value, ctx)),
            hints: hints.clone(),
        },
        _ => value.clone(),
    }
}

fn bind_iterated_value_to_choose_spec(value: &Value, spec: &ChooseSpec) -> Value {
    match value {
        Value::PowerOf(inner) if matches!(inner.base(), ChooseSpec::Iterated) => {
            Value::PowerOf(Box::new(spec.clone()))
        }
        Value::ToughnessOf(inner) if matches!(inner.base(), ChooseSpec::Iterated) => {
            Value::ToughnessOf(Box::new(spec.clone()))
        }
        Value::ManaValueOf(inner) if matches!(inner.base(), ChooseSpec::Iterated) => {
            Value::ManaValueOf(Box::new(spec.clone()))
        }
        Value::ManaSymbolsInManaCostOf { spec: inner, color }
            if matches!(inner.base(), ChooseSpec::Iterated) =>
        {
            Value::ManaSymbolsInManaCostOf {
                spec: Box::new(spec.clone()),
                color: *color,
            }
        }
        Value::Add(left, right) => Value::Add(
            Box::new(bind_iterated_value_to_choose_spec(left, spec)),
            Box::new(bind_iterated_value_to_choose_spec(right, spec)),
        ),
        Value::Scaled(inner, multiplier) => Value::Scaled(
            Box::new(bind_iterated_value_to_choose_spec(inner, spec)),
            *multiplier,
        ),
        Value::DividedRoundedDown(inner, divisor) => Value::DividedRoundedDown(
            Box::new(bind_iterated_value_to_choose_spec(inner, spec)),
            *divisor,
        ),
        Value::HalfRoundedDown(inner) => {
            Value::HalfRoundedDown(Box::new(bind_iterated_value_to_choose_spec(inner, spec)))
        }
        Value::Min(left, right) => Value::Min(
            Box::new(bind_iterated_value_to_choose_spec(left, spec)),
            Box::new(bind_iterated_value_to_choose_spec(right, spec)),
        ),
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_iterated_value_to_choose_spec(value, spec)),
            hints: hints.clone(),
        },
        _ => value.clone(),
    }
}

fn choose_spec_owned_by_iterated_player(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => choose_spec_owned_by_iterated_player(spec),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            matches!(filter.owner, Some(PlayerFilter::IteratedPlayer))
                || filter
                    .any_of
                    .iter()
                    .any(|filter| matches!(filter.owner, Some(PlayerFilter::IteratedPlayer)))
        }
        _ => false,
    }
}

fn reserved_or_next_object_tag(ctx: &mut EffectLoweringContext, prefix: &str) -> String {
    let prefix_with_sep = format!("{prefix}_");
    ctx.take_reserved_object_result_tag(prefix)
        .or_else(|| {
            ctx.last_object_tag
                .clone()
                .filter(|tag| tag.starts_with(&prefix_with_sep))
        })
        .unwrap_or_else(|| ctx.next_tag(prefix))
}

fn prevention_target_from_non_choice_target(
    target: &TargetAst,
    ctx: &EffectLoweringContext,
) -> Result<ironsmith_core::PreventionTarget, CardTextError> {
    match target {
        TargetAst::Player(PlayerFilter::You, _) => Ok(ironsmith_core::PreventionTarget::You),
        TargetAst::Player(PlayerFilter::Any, _) => Ok(ironsmith_core::PreventionTarget::Players),
        TargetAst::Object(filter, explicit_target_span, _) if explicit_target_span.is_none() => {
            Ok(ironsmith_core::PreventionTarget::PermanentsMatching(
                resolve_it_tag(filter, &current_reference_env(ctx))?,
            ))
        }
        TargetAst::ObjectOrPlayer(filter, PlayerFilter::You, explicit_target_span)
            if explicit_target_span.is_none() =>
        {
            Ok(ironsmith_core::PreventionTarget::YouAndPermanentsMatching(
                resolve_it_tag(filter, &current_reference_env(ctx))?,
            ))
        }
        _ => Err(CardTextError::ParseError(
            "unsupported prevent-all damage protected target with source filter".to_string(),
        )),
    }
}

fn resolve_tagged_top_library_condition(
    condition: &crate::ConditionExpr,
    ctx: &EffectLoweringContext,
) -> Result<crate::ConditionExpr, CardTextError> {
    match condition {
        crate::ConditionExpr::TaggedObjectIsTopOfLibrary { tag, player } => {
            let resolved_tag = if tag.as_str() == "__last_revealed__" {
                TagKey::from(ctx.last_revealed_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve last revealed card without prior reveal".to_string(),
                    )
                })?)
            } else if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            Ok(crate::ConditionExpr::TaggedObjectIsTopOfLibrary {
                tag: resolved_tag,
                player: player.clone(),
            })
        }
        crate::ConditionExpr::Not(inner) => Ok(crate::ConditionExpr::Not(Box::new(
            resolve_tagged_top_library_condition(inner, ctx)?,
        ))),
        crate::ConditionExpr::And(left, right) => Ok(crate::ConditionExpr::And(
            Box::new(resolve_tagged_top_library_condition(left, ctx)?),
            Box::new(resolve_tagged_top_library_condition(right, ctx)?),
        )),
        crate::ConditionExpr::Or(left, right) => Ok(crate::ConditionExpr::Or(
            Box::new(resolve_tagged_top_library_condition(left, ctx)?),
            Box::new(resolve_tagged_top_library_condition(right, ctx)?),
        )),
        _ => Ok(condition.clone()),
    }
}

const EFFECT_COMPILE_HANDLERS: [EffectCompileHandlerDef; 14] = [
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_combat_and_damage_effect,
    },
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_board_state_effect,
    },
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_player_resource_and_choice_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_timing_and_control_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_flow_and_iteration_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_destroy_and_exile_effect,
    },
    EffectCompileHandlerDef {
        run: effect_visibility_object_handlers::try_compile_visibility_and_card_selection_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_stack_and_condition_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_attachment_and_setup_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_token_generation_effect,
    },
    EffectCompileHandlerDef {
        run: effect_continuous_turn_handlers::try_compile_continuous_and_modifier_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_search_and_reorder_effect,
    },
    EffectCompileHandlerDef {
        run: effect_visibility_object_handlers::try_compile_object_zone_and_exchange_effect,
    },
    EffectCompileHandlerDef {
        run: effect_continuous_turn_handlers::try_compile_player_turn_and_counter_effect,
    },
];

fn retarget_target_is_bare_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter == &ObjectFilter::tagged(IT_TAG),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            retarget_target_is_bare_it(inner)
        }
        _ => false,
    }
}

fn target_is_any_damage_target(target: &TargetAst) -> bool {
    match target {
        TargetAst::AnyTarget(_)
        | TargetAst::AnyOtherTarget(_)
        | TargetAst::ObjectOrPlayer(_, _, _) => true,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_is_any_damage_target(inner)
        }
        _ => false,
    }
}

pub(crate) fn compile_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    // `compile_subject_verb_effect` has a large debug-build frame because it lowers the
    // complete typed action enum. Keep the recursive lowering guard consistent with the
    // other typed effect entry points; a 2 MiB alternate stack is smaller than that frame.
    stacker::maybe_grow(8 * 1024 * 1024, 16 * 1024 * 1024, || {
        compile_effect_inner(effect, ctx)
    })
}

fn compile_effect_inner(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    if let EffectAst::SubjectVerb(subject_verb) = effect {
        return compile_subject_verb_effect(subject_verb, ctx);
    }
    if let EffectAst::SolveCase = effect {
        return Ok((
            vec![Effect::new(crate::effects::SolveCaseEffect::new())],
            Vec::new(),
        ));
    }
    if let EffectAst::RestartGame {
        cards_left_in_exile,
        source_surface,
    } = effect
    {
        let mut payload = crate::effects::RestartGameEffect::new(cards_left_in_exile.clone());
        if let Some(surface) = source_surface {
            payload = payload.with_source_surface(surface.clone());
        }
        let mut runtime_effect = Effect::new(payload);
        if ctx.auto_tag_object_targets && cards_left_in_exile.is_some() {
            let tag = ctx.next_tag("restarted");
            runtime_effect = runtime_effect.tag(tag.clone());
            ctx.last_object_tag = Some(tag);
        }
        return Ok((vec![runtime_effect], Vec::new()));
    }
    if let EffectAst::PlaySubgame { nonwinner_effects } = effect {
        let (compiled, choices) =
            compile_effects_in_iterated_player_context(nonwinner_effects, ctx, None)?;
        return Ok((
            vec![Effect::new(crate::effects::PlaySubgameEffect::new(
                compiled,
            ))],
            choices,
        ));
    }
    if let EffectAst::Sequence { effects } = effect {
        let (mut effects, choices) = compile_effects(effects, ctx)?;
        preserve_nested_result_value_links(&mut effects);
        return Ok((effects, choices));
    }
    if let EffectAst::CommaThen { effects } = effect {
        let (mut effects, choices) = compile_effects(effects, ctx)?;
        preserve_nested_result_value_links(&mut effects);
        return Ok((
            vec![Effect::new(crate::effects::SequenceEffect::comma_then(
                effects,
            ))],
            choices,
        ));
    }
    if let EffectAst::SourceSentence { effects, .. } = effect {
        // SourceSentence is compiler-only provenance used to keep one Oracle
        // sentence together while references are resolved. It has no
        // separate runtime effect; lower its typed children in order.
        let (mut effects, choices) = compile_effects(effects, ctx)?;
        preserve_nested_result_value_links(&mut effects);
        return Ok((effects, choices));
    }
    if let EffectAst::Coordinated {
        effects,
        leading_duration,
        result_conjunction,
    } = effect
    {
        // A coordinated clause establishes its own target-reference scope.
        // Always tag object targets here so a later member of the same clause
        // binds to the newly introduced target rather than an antecedent from
        // an activated cost or an earlier instruction.
        let saved_force_auto_tag_object_targets = ctx.force_auto_tag_object_targets;
        let saved_auto_tag_object_targets = ctx.auto_tag_object_targets;
        ctx.force_auto_tag_object_targets = true;
        let compiled = compile_effects(effects, ctx);
        ctx.force_auto_tag_object_targets = saved_force_auto_tag_object_targets;
        ctx.auto_tag_object_targets = saved_auto_tag_object_targets;
        let (mut effects, choices) = compiled?;
        preserve_nested_result_value_links(&mut effects);
        let (effects, choices) = preserve_independent_coordinated_targets(effects, choices);
        let sequence = if *result_conjunction {
            crate::effects::SequenceEffect::result_conjunction(effects, *leading_duration)
        } else if *leading_duration {
            crate::effects::SequenceEffect::coordinated_with_leading_duration(effects)
        } else {
            crate::effects::SequenceEffect::coordinated(effects)
        };
        return Ok((vec![Effect::new(sequence)], choices));
    }
    if let EffectAst::ChooseOneOf { modes } = effect {
        use crate::effect::EffectMode;
        let mut lowered_modes = Vec::with_capacity(modes.len());
        let mut choices = Vec::new();
        for mode in modes {
            let (mode_effects, mode_choices) = compile_effects(&mode.effects, ctx)?;
            for choice in mode_choices {
                push_choice(&mut choices, choice);
            }
            lowered_modes.push(EffectMode {
                source_text: mode.description.clone(),
                effects: mode_effects,
            });
        }
        // `EffectAst::ChooseOneOf` represents an instruction-level choice
        // made while the effect resolves. Printed modal spell/ability blocks
        // have their own document AST and deliberately lower without a
        // chooser so their modes are announced before targets. Keeping the
        // chooser explicit here prevents inline "A or B" instructions from
        // being mistaken for casting-time modal choices.
        let choose = crate::effects::ChooseModeEffect::choose_one(lowered_modes)
            .with_chooser(crate::target::PlayerFilter::You);
        return Ok((vec![Effect::new(choose)], choices));
    }
    if let EffectAst::VillainousChoice {
        player,
        player_surface,
        modes,
    } = effect
    {
        use crate::effect::EffectMode;
        let mut lowered_modes = Vec::with_capacity(modes.len());
        let mut choices = Vec::new();
        for mode in modes {
            let (mode_effects, mode_choices) = compile_effects(&mode.effects, ctx)?;
            for choice in mode_choices {
                push_choice(&mut choices, choice);
            }
            lowered_modes.push(EffectMode {
                source_text: mode.description.clone(),
                effects: mode_effects,
            });
        }
        return Ok((
            vec![Effect::villainous_choice(
                player.clone(),
                player_surface.clone(),
                lowered_modes,
            )],
            choices,
        ));
    }
    if let EffectAst::IfEffectDidNotHappen { effect, otherwise } = effect {
        let id = ctx.next_effect_id();
        let (mut lowered, mut choices) = compile_effect(effect, ctx)?;
        let inner = lowered.pop().ok_or_else(|| {
            CardTextError::ParseError(
                "if-effect-did-not-happen requires a single nested effect".to_string(),
            )
        })?;
        if !lowered.is_empty() {
            return Err(CardTextError::ParseError(
                "if-effect-did-not-happen nested effect must lower to a single effect".to_string(),
            ));
        }
        let wrapped = Effect::with_id(id.0, inner);
        ctx.last_effect_id = Some(id);
        let (otherwise_effects, otherwise_choices) = compile_effects(otherwise, ctx)?;
        for choice in otherwise_choices {
            push_choice(&mut choices, choice);
        }
        let fallback = Effect::if_then(id, EffectPredicate::DidNotHappen, otherwise_effects);
        return Ok((vec![wrapped, fallback], choices));
    }
    if let EffectAst::TagAffected { effect, tag } = effect {
        // This wrapper is itself the authoritative outcome tag. Letting the
        // nested action auto-tag its object result first creates two nested
        // tags, and the outer group tag can then overwrite the explicit
        // primary alias used by linked fanout filters. A coordinated
        // choose-target prelude is the deliberate exception: its shared
        // `__chosen_objects__` wrapper aggregates independently addressable
        // target slots, so each explicit TargetOnly action must keep its
        // inner `targeted_N` tag.
        let preserves_explicit_chosen_target_slot = tag.as_str()
            == crate::cards::builders::CHOSEN_OBJECTS_TAG
            && matches!(
                effect.as_ref(),
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::TargetOnly {
                        explicit_declaration: true,
                        ..
                    },
                    ..
                })
            );
        let saved_auto_tag_object_targets = ctx.auto_tag_object_targets;
        if !preserves_explicit_chosen_target_slot {
            ctx.auto_tag_object_targets = false;
        }
        let compiled = compile_effect(effect, ctx);
        ctx.auto_tag_object_targets = saved_auto_tag_object_targets;
        let (mut lowered, choices) = compiled?;
        let inner = lowered.pop().ok_or_else(|| {
            CardTextError::ParseError("tag-affected requires a single nested effect".to_string())
        })?;
        // A single semantic action can lower with target/capture preludes
        // before its executable effect. Keep those preludes outside the
        // wrapper and tag only the final action's actual outcome.
        if lowered.iter().any(|effect| {
            effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_none()
                && effect
                    .downcast_ref::<crate::effects::TagMatchingObjectsEffect>()
                    .is_none()
        }) {
            return Err(CardTextError::ParseError(
                "tag-affected nested effect may only have target or capture preludes".to_string(),
            ));
        }
        ctx.last_object_tag = Some(tag.as_str().to_string());
        lowered.push(inner.tag_all(tag.clone()));
        return Ok((lowered, choices));
    }
    if let EffectAst::ManaRestricted {
        effects,
        restrictions,
    } = effect
    {
        let mut compiled_effects = Vec::new();
        let mut choices = Vec::new();
        for child in effects {
            let (mut child_effects, mut child_choices) = compile_effect(child, ctx)?;
            compiled_effects.append(&mut child_effects);
            choices.append(&mut child_choices);
        }
        return Ok((
            vec![Effect::mana_restricted(
                compiled_effects,
                restrictions.clone(),
            )],
            choices,
        ));
    }
    if let EffectAst::RepeatEffects { count, effects } = effect {
        let repeats_manifest_dread = matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ManifestDread,
                ..
            })]
        );
        let hoisted_result_tag = (ctx.auto_tag_object_targets && repeats_manifest_dread)
            .then(|| reserved_or_next_object_tag(ctx, "manifested"));
        let saved_auto_tag_object_targets = ctx.auto_tag_object_targets;
        if hoisted_result_tag.is_some() {
            // The repeated action's results form one provenance set for a
            // following plural reference. Tag the aggregate RepeatEffects
            // outcome, rather than overwriting the tag once per iteration.
            ctx.auto_tag_object_targets = false;
        }
        let mut compiled_effects = Vec::new();
        let mut choices = Vec::new();
        for child in effects {
            let (mut child_effects, mut child_choices) = compile_effect(child, ctx)?;
            compiled_effects.append(&mut child_effects);
            choices.append(&mut child_choices);
        }
        ctx.auto_tag_object_targets = saved_auto_tag_object_targets;
        let mut repeated = Effect::repeat_effects(count.clone(), compiled_effects);
        if let Some(tag) = hoisted_result_tag {
            repeated = repeated.tag(tag.clone());
            ctx.last_object_tag = Some(tag);
        }
        return Ok((vec![repeated], choices));
    }
    if let EffectAst::BidLife {
        target,
        starting_bid,
        winner_effects,
    } = effect
    {
        let refs = current_reference_env(ctx);
        let (target_spec, mut choices) = resolve_target_spec_with_choices(target, &refs)?;
        let (winner_effects, winner_choices) = compile_effects(winner_effects, ctx)?;
        for choice in winner_choices {
            push_choice(&mut choices, choice);
        }
        return Ok((
            vec![Effect::new(crate::effects::BidLifeEffect::new(
                target_spec,
                crate::effects::LifeBidStart::Fixed(*starting_bid),
                winner_effects,
            ))],
            choices,
        ));
    }
    if let EffectAst::SecretChoiceStart {
        options,
        participants,
        object_choice,
    } = effect
    {
        let secret_choice = if let Some(object_choice) = object_choice {
            crate::effects::SecretChoiceEffect::new_objects(
                participants.clone(),
                object_choice.clone(),
            )
        } else {
            crate::effects::SecretChoiceEffect::new(options.clone(), participants.clone())
        };
        return Ok((vec![Effect::new(secret_choice)], Vec::new()));
    }
    if let EffectAst::SecretChoiceReveal = effect {
        return Ok((Vec::new(), Vec::new()));
    }
    if let EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
        player,
        zone_owner,
        filter,
        zone,
        payment,
    } = effect
    {
        let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
        let player = resolve_non_target_player_filter(player.clone(), &current_reference_env(ctx))?;
        let zone_owner =
            resolve_non_target_player_filter(zone_owner.clone(), &current_reference_env(ctx))?;
        return Ok((
            vec![Effect::new({
                let mut effect =
                    crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect::new(
                        player,
                        resolved_filter,
                        *zone,
                    )
                    .with_zone_owner(zone_owner);
                effect.payment = payment.clone();
                effect
            })],
            Vec::new(),
        ));
    }
    if matches!(
        effect,
        EffectAst::RepeatThisProcess | EffectAst::RepeatThisProcessOnce
    ) {
        return Err(CardTextError::ParseError(
            "unsupported repeat this process effect tail".to_string(),
        ));
    }
    if matches!(effect, EffectAst::SelfReplacement { .. }) {
        return Err(CardTextError::ParseError(
            "unsupported nested self-replacement effect".to_string(),
        ));
    }
    if let Some(compiled) = try_compile_effect_via_handlers(effect, ctx)? {
        return Ok(compiled);
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing compile-effect dispatch route for effect variant: {effect:?}"
    )))
}

fn try_compile_effect_via_handlers(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    for EffectCompileHandlerDef { run, .. } in EFFECT_COMPILE_HANDLERS {
        if let Some(compiled) = run(effect, ctx)? {
            return Ok(Some(compiled));
        }
    }
    Ok(None)
}

fn compile_subject_verb_effect(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<EffectCompileOutcome, CardTextError> {
    if let Some(outcome) = compile_subject_verb_early(subject_verb, ctx)? {
        return Ok(outcome);
    }
    if let Some(outcome) = compile_subject_verb_middle(subject_verb, ctx)? {
        return Ok(outcome);
    }
    if let Some(outcome) = compile_subject_verb_late(subject_verb, ctx)? {
        return Ok(outcome);
    }
    unreachable!(
        "subject-verb action was not handled: {:?}",
        subject_verb.action
    )
}
fn subject_verb_role(role: SubjectVerbRoleAst) -> SubjectRole {
    match role {
        SubjectVerbRoleAst::Actor => SubjectRole::Actor,
        SubjectVerbRoleAst::AffectedPlayer => SubjectRole::AffectedPlayer,
        SubjectVerbRoleAst::Chooser => SubjectRole::Chooser,
        SubjectVerbRoleAst::LibraryOwner => SubjectRole::LibraryOwner,
    }
}

fn resolve_subject_verb_subject(
    role: SubjectRole,
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
) -> Result<LoweredSubject, CardTextError> {
    match role {
        SubjectRole::Actor => LoweredSubject::resolve_actor(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::AffectedPlayer => LoweredSubject::resolve_affected_player(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::Chooser => LoweredSubject::resolve_chooser(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::LibraryOwner => LoweredSubject::resolve_library_owner(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::ZoneOwner => LoweredSubject::resolve_zone_owner(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
    }
}

fn compile_subject_verb_player_value_effect<YouBuilder, OtherBuilder>(
    role: SubjectRole,
    player: PlayerAst,
    value: &Value,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
    resolve_it_tags: bool,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce(Value) -> Effect,
    OtherBuilder: FnOnce(Value, PlayerFilter) -> Effect,
{
    let subject = resolve_subject_verb_subject(
        role,
        player,
        ctx,
        allow_target,
        allow_target_opponent,
        track_last_player_filter,
    )?;
    let mut value = value.clone();
    if !ctx.iterated_player {
        let binding_player = ctx
            .last_player_filter
            .as_ref()
            .unwrap_or_else(|| subject.player_filter());
        bind_relative_iterated_player_in_value_to_player_filter(&mut value, binding_player);
    }
    let value = if resolve_it_tags {
        resolve_value_it_tag(&value, &current_reference_env(ctx))?
    } else {
        value
    };
    let you_value = value.clone();
    let (player_filter, choices) = subject.into_parts();
    let value = per_player_partition_value_for_filter(value, &player_filter);
    let you_value = per_player_partition_value_for_filter(you_value, &PlayerFilter::You);
    let mut prelude_effects = Vec::new();
    let mut merged_choices = choices.clone();
    collect_value_player_target_choices(&value, &mut merged_choices);
    if let Some(spec) = value_object_target_spec(&value)
        && ctx.auto_tag_object_targets
    {
        let effect = tag_object_target_effect(
            Effect::new(crate::effects::TargetOnlyEffect::new(spec.clone())),
            &spec,
            ctx,
            "targeted",
        );
        prelude_effects.push(effect);
        push_choice(&mut merged_choices, spec);
    }
    let (mut effects, choices) = compile_player_effect_from_resolved_filter(
        player_filter,
        choices,
        || build_you(you_value),
        |filter| build_other(value, filter),
    )?;
    prelude_effects.append(&mut effects);
    for choice in choices {
        push_choice(&mut merged_choices, choice);
    }
    Ok((prelude_effects, merged_choices))
}

fn collect_value_player_target_choices(value: &Value, choices: &mut Vec<ChooseSpec>) {
    match value {
        Value::SurfaceHinted { value, .. } => collect_value_player_target_choices(value, choices),
        Value::Add(left, right) | Value::Min(left, right) => {
            collect_value_player_target_choices(left, choices);
            collect_value_player_target_choices(right, choices);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => collect_value_player_target_choices(inner, choices),
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::TotalPower(filter)
        | Value::TotalToughness(filter)
        | Value::TotalManaValue(filter)
        | Value::GreatestPower(filter)
        | Value::GreatestToughness(filter)
        | Value::GreatestManaValue(filter)
        | Value::LeastPower(filter)
        | Value::LeastToughness(filter)
        | Value::LeastManaValue(filter)
        | Value::BasicLandTypesAmong(filter)
        | Value::CreatureTypesAmong(filter)
        | Value::CardTypesAmong(filter)
        | Value::ColorsAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => {
            collect_object_filter_player_target_choices(filter, choices);
        }
        Value::PlayersWhoControlMoreThanYou { players, filter }
        | Value::PlayersWhoControlAtLeastMoreThanYou {
            players, filter, ..
        } => {
            collect_player_filter_target_choice(players, choices);
            collect_object_filter_player_target_choices(filter, choices);
        }
        Value::StaticAbilitiesAmong { filter, .. } => {
            collect_object_filter_player_target_choices(filter, choices);
        }
        Value::SpellsCastThisTurnMatching { player, filter, .. } => {
            collect_player_filter_target_choice(player, choices);
            collect_object_filter_player_target_choices(filter, choices);
        }
        Value::Devotion { player, .. }
        | Value::CountPlayers(player)
        | Value::CountPlayersWithCardsInHandAtLeast(player, _)
        | Value::PartySize(player)
        | Value::LifeTotal(player)
        | Value::LifeTotalAsTurnBegan(player)
        | Value::LifeTotalDifference(player)
        | Value::Speed(player)
        | Value::StartingLifeTotal(player)
        | Value::DevotionToChosenColor(player)
        | Value::LifeGainedThisTurn(player)
        | Value::LifeLostThisTurn(player)
        | Value::CardsDiscardedThisTurn(player)
        | Value::DamageDealtToPlayersThisTurn(player)
        | Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | Value::MaxCardsDrawnThisTurn(player)
        | Value::LandsEnteredBattlefieldThisTurn(player)
        | Value::MaxCardsInHand(player)
        | Value::CardsInHand(player)
        | Value::CardsInLibrary(player)
        | Value::CardsInGraveyard(player)
        | Value::SpellsCastThisTurn(player)
        | Value::SpellsCastBeforeThisTurn(player)
        | Value::CommanderCastCount(player)
        | Value::CardTypesInGraveyard(player)
        | Value::HalfLifeTotalRoundedUp(player)
        | Value::HalfLifeTotalRoundedDown(player)
        | Value::HalfStartingLifeTotalRoundedUp(player)
        | Value::HalfStartingLifeTotalRoundedDown(player)
        | Value::CreaturesDiedThisTurnControlledBy(player)
        | Value::PlayerCounters(player, _)
        | Value::PlayerVoteCount(player) => {
            collect_player_filter_target_choice(player, choices);
        }
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, .. } => {
            collect_player_filter_target_choice(player, choices);
        }
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::ManaSymbolsInManaCostOf { spec, .. }
        | Value::CountersOn(spec, _) => collect_choose_spec_player_target_choices(spec, choices),
        _ => {}
    }
}

fn collect_choose_spec_player_target_choices(spec: &ChooseSpec, choices: &mut Vec<ChooseSpec>) {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _) => {
            collect_choose_spec_player_target_choices(spec, choices)
        }
        ChooseSpec::Player(player)
        | ChooseSpec::PlayerOrPlaneswalker(player)
        | ChooseSpec::EachPlayer(player) => collect_player_filter_target_choice(player, choices),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            collect_object_filter_player_target_choices(filter, choices)
        }
        _ => {}
    }
}

fn collect_object_filter_player_target_choices(
    filter: &ObjectFilter,
    choices: &mut Vec<ChooseSpec>,
) {
    for player in [
        filter.controller.as_ref(),
        filter.cast_by.as_ref(),
        filter.owner.as_ref(),
        filter.targets_player.as_ref(),
        filter.targets_only_player.as_ref(),
        filter
            .attacking_player_or_planeswalker_controlled_by
            .as_ref(),
        filter.attached_to_player.as_ref(),
        filter.entered_battlefield_controller.as_ref(),
        filter.dealt_damage_to_player_this_turn.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        collect_player_filter_target_choice(player, choices);
    }
    if let Some(targets) = filter.targets_object.as_deref() {
        collect_object_filter_player_target_choices(targets, choices);
    }
    if let Some(targets) = filter.targets_only_object.as_deref() {
        collect_object_filter_player_target_choices(targets, choices);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref() {
        collect_object_filter_player_target_choices(attached_to, choices);
    }
    if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_deref() {
        collect_object_filter_player_target_choices(combat_partner, choices);
    }
    for option in &filter.any_of {
        collect_object_filter_player_target_choices(option, choices);
    }
}

fn collect_player_filter_target_choice(player: &PlayerFilter, choices: &mut Vec<ChooseSpec>) {
    if let PlayerFilter::Target(inner) = player {
        if matches!(**inner, PlayerFilter::Any) {
            // `Target(Any)` inside a value is how resolved back-references to the
            // spell's existing player targets surface (e.g. "those players"); the
            // explicit choose effect already carries the target requirement.
            return;
        }
        push_choice(
            choices,
            ChooseSpec::target(ChooseSpec::Player((**inner).clone())),
        );
    }
}

fn value_object_target_spec(value: &Value) -> Option<ChooseSpec> {
    match value {
        Value::SurfaceHinted { value, .. } => value_object_target_spec(value),
        Value::Add(left, right) => {
            value_object_target_spec(left).or_else(|| value_object_target_spec(right))
        }
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::ManaSymbolsInManaCostOf { spec, .. }
        | Value::CountersOn(spec, _) => {
            (spec.is_target() && choose_spec_targets_object(spec)).then(|| (**spec).clone())
        }
        _ => None,
    }
}

fn per_player_partition_value_for_filter(value: Value, player_filter: &PlayerFilter) -> Value {
    if !matches!(player_filter, PlayerFilter::IteratedPlayer) {
        return value;
    }
    match value {
        Value::EffectValue(effect_id) => Value::EffectMetric {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::IteratedPlayerCount,
        },
        Value::EffectValueOffset(effect_id, offset) => Value::EffectMetricOffset {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::IteratedPlayerCount,
            offset,
        },
        Value::EffectMetric {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::Count,
        } => Value::EffectMetric {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::IteratedPlayerCount,
        },
        Value::EffectMetricOffset {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::Count,
            offset,
        } => Value::EffectMetricOffset {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::IteratedPlayerCount,
            offset,
        },
        other => other,
    }
}

fn replace_iterated_player_with_target_player_in_choose_spec(spec: &mut ChooseSpec) {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            replace_iterated_player_with_target_player_in_choose_spec(spec);
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            replace_iterated_player_with_target_player_in_object_filter(filter);
        }
        ChooseSpec::Player(filter)
        | ChooseSpec::EachPlayer(filter)
        | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            replace_iterated_player_with_target_player(filter);
        }
        _ => {}
    }
}

fn replace_iterated_player_with_target_player_in_object_filter(filter: &mut ObjectFilter) {
    if let Some(owner) = &mut filter.owner {
        replace_iterated_player_with_target_player(owner);
    }
    if let Some(controller) = &mut filter.controller {
        replace_iterated_player_with_target_player(controller);
    }
    if let Some(attached_to_player) = &mut filter.attached_to_player {
        replace_iterated_player_with_target_player(attached_to_player);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        replace_iterated_player_with_target_player_in_object_filter(attached_to);
    }
    if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_deref_mut() {
        replace_iterated_player_with_target_player_in_object_filter(combat_partner);
    }
    for nested in &mut filter.any_of {
        replace_iterated_player_with_target_player_in_object_filter(nested);
    }
}

fn replace_iterated_player_with_target_player(filter: &mut PlayerFilter) {
    match filter {
        PlayerFilter::IteratedPlayer => {
            *filter = PlayerFilter::target_player();
        }
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            replace_iterated_player_with_target_player(inner)
        }
        _ => {}
    }
}

#[cfg(test)]
mod nested_result_value_link_tests {
    use super::*;

    #[test]
    fn scaled_mana_keeps_the_result_id_used_by_following_draw() {
        let result_id = EffectId(7);
        let mut effects = vec![
            Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                vec![ManaSymbol::Red],
                Value::Fixed(2),
                PlayerFilter::You,
            )),
            Effect::new(crate::effects::DrawCardsEffect::you(
                Value::EffectValueOffset(result_id, 1),
            )),
        ];

        preserve_nested_result_value_links(&mut effects);

        let with_id = effects[0]
            .as_with_id()
            .expect("the scaled-mana producer should retain its result ID");
        assert_eq!(with_id.id, result_id);
        assert!(
            with_id
                .effect
                .downcast_ref::<crate::effects::mana::AddScaledManaEffect>()
                .is_some()
        );
    }

    #[test]
    fn exile_keeps_the_result_id_used_by_following_token_count() {
        let result_id = EffectId(11);
        let token = CardDefinitionBuilder::new(CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .build();
        let query = ironsmith_core::PriorEffectMetricQuery::new(
            ironsmith_core::EffectMetricSource::AffectedObjects,
            ironsmith_core::EffectMetric::Count,
        )
        .with_action(ironsmith_core::PriorEffectAction::Exiled);
        let mut effects = vec![
            Effect::new(crate::effects::ExileEffect::all(ObjectFilter::creature())),
            Effect::new(crate::effects::CreateTokenEffect::you(
                token,
                Value::PriorEffectMetric {
                    effect_id: result_id,
                    query,
                },
            )),
        ];

        preserve_nested_result_value_links(&mut effects);

        let with_id = effects[0]
            .as_with_id()
            .expect("the exile producer should retain its result ID");
        assert_eq!(with_id.id, result_id);
        assert!(
            with_id
                .effect
                .downcast_ref::<crate::effects::ExileEffect>()
                .is_some()
        );
    }

    #[test]
    fn explicit_tag_affected_suppresses_nested_automatic_result_tag() {
        let target = TargetAst::Object(
            ObjectFilter::creature(),
            Some(crate::cards::builders::TextSpan::synthetic()),
            None,
        );
        let ast = EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_destroy(target)),
            tag: TagKey::from("linked_primary"),
        };
        let mut ctx = EffectLoweringContext::new();
        ctx.auto_tag_object_targets = true;

        let (effects, _) = compile_effect(&ast, &mut ctx).expect("explicit tagged destroy");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("linked_primary"), "{debug}");
        assert!(!debug.contains("destroyed_"), "{debug}");
        assert_eq!(ctx.last_object_tag.as_deref(), Some("linked_primary"));
    }

    #[test]
    fn chosen_collection_wrapper_keeps_an_explicit_target_slot_tag() {
        let target = TargetAst::Object(
            ObjectFilter::creature(),
            Some(crate::cards::builders::TextSpan::synthetic()),
            None,
        );
        let ast = EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_explicit_target_only(target)),
            tag: TagKey::from(crate::cards::builders::CHOSEN_OBJECTS_TAG),
        };
        let mut ctx = EffectLoweringContext::new();
        ctx.auto_tag_object_targets = true;

        let (effects, _) = compile_effect(&ast, &mut ctx).expect("chosen target slot");
        let [outer] = effects.as_slice() else {
            panic!("expected one doubly tagged target slot: {effects:#?}");
        };
        let outer = outer
            .as_tagged()
            .expect("chosen collection should remain the outer tag");
        assert_eq!(
            outer.tag.as_str(),
            crate::cards::builders::CHOSEN_OBJECTS_TAG
        );
        let inner = outer
            .effect
            .as_tagged()
            .expect("explicit target slot should retain its automatic tag");
        assert_eq!(inner.tag.as_str(), "targeted_0");
        assert!(
            inner
                .effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some()
        );
        assert_eq!(
            ctx.last_object_tag.as_deref(),
            Some(crate::cards::builders::CHOSEN_OBJECTS_TAG)
        );
    }
}

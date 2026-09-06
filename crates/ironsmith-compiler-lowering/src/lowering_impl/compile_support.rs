use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming, TriggeredAbility};
use crate::card::PowerToughness;
use crate::cards::CardDefinition;
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, ClashOpponentAst, ControlDurationAst, DamageBySpec,
    EffectAst, EffectLoweringContext, ExchangeValueAst, ExchangeValueKindAst, ExtraTurnAnchorAst,
    GrantedAbilityAst, IdGenContext, IfResultPredicate, LoweringFrame, NormalizedLine,
    ObjectRefAst, PlayerAst, PredicateAst, PreventNextTimeDamageSourceAst,
    PreventNextTimeDamageTargetAst, RetargetModeAst, ReturnControllerAst, SharedTypeConstraintAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TriggerSpec,
    TurnHistoryPredicateAst, ZoneMoveActionAst,
};
use crate::color::{Color, ColorSet};
use crate::cost::TotalCost;
use crate::effect::{
    ChoiceCount, Condition, Effect, EffectId, EffectMode, EffectPredicate, EmblemDescription,
    EventValueSpec, Until, Value,
};
use crate::effects::composition::VoteOption;
use crate::filter::{
    ObjectFilter, ObjectRef, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::ids::CardId;
use crate::mana::{ManaCost, ManaSymbol};
use crate::model::token_definition::{ConstructArtifactScalingShape, TokenDefinitionSpec};
use crate::static_abilities::{CopyTriggeredAbilities, StaticAbility};
use crate::target::ChooseSpec;
use crate::triggers::{DamagedBySource, Trigger};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;
use ironsmith_compiler_semantic::model_impl::token_definition as token_grammar;

use super::effect_ast_traversal::{
    TerminalResultProducer, for_each_nested_effects, for_each_nested_effects_mut,
    terminal_result_producer,
};
use super::effect_pipeline::{
    EffectPreludeTag, PreparedEffectsForLowering, PreparedPredicateForLowering,
    PreparedTriggeredEffectsForLowering,
};
use super::lowering_support::{
    assemble_parsed_triggered_ability, lower_parsed_ability, lower_static_ability_ast,
    stage_effects_for_lowering, stage_effects_with_trigger_context_for_lowering,
};
use super::reference_helpers::{
    as_followup_player_alias, choose_spec_targets_object, is_you_player_filter,
    object_filter_as_tagged_reference, player_filter_from_object_filter,
    resolve_attach_object_spec, resolve_choose_spec_it_tag, resolve_it_tag, resolve_it_tag_key,
    resolve_non_target_player_filter, resolve_restriction_it_tag, resolve_target_spec_with_choices,
    resolve_total_cost_it_tags, resolve_unless_player_filter, resolve_value_it_tag,
    watch_tag_from_filter, with_target_reference_surface_hint,
};
use super::reference_resolution::{
    EffectReferenceResolutionConfig, annotate_effect_sequence,
    effect_references_prior_prevention_amount, effect_references_typed_removed_counter_metric,
    preserves_existing_it_for_power_self_damage_followup,
};
use super::runtime_static_ability_helpers::{
    lower_granted_abilities_ast, lower_granted_abilities_ast_to_object_abilities,
    object_abilities_to_static_carriers,
};
use crate::model::ast::{EmblemAbilityAst, EmblemDescriptionAst};
use crate::model::reference_state::{
    AnnotatedEffect, AnnotatedEffectSequence, LoweredEffects, ReferenceEnv, ReferenceExports,
    ReferenceImports,
};

#[path = "compile_support/choose_effect_helpers.rs"]
mod choose_effect_helpers;
#[path = "compile_support/control_flow_handlers.rs"]
mod control_flow_handlers;
#[path = "compile_support/effect_dispatch.rs"]
mod effect_dispatch;
#[path = "compile_support/effect_flow_search_handlers.rs"]
mod effect_flow_search_handlers;
#[path = "compile_support/effect_handlers.rs"]
mod effect_handlers;
#[path = "compile_support/effect_visibility_object_handlers.rs"]
mod effect_visibility_object_handlers;
#[path = "compile_support/iterated_player_validation.rs"]
mod iterated_player_validation;
#[path = "compile_support/player_effect_helpers.rs"]
mod player_effect_helpers;
#[path = "compile_support/prepared_effects.rs"]
mod prepared_effects;
use ironsmith_compiler_resolve::tag_support;
#[path = "compile_support/trigger_support.rs"]
mod trigger_support;

#[cfg(test)]
use crate::cards::builders::ParseAnnotations;
pub use choose_effect_helpers::{
    compile_choose_objects_across_zones_with_subject, compile_choose_objects_with_subject,
    compile_choose_player_with_subject,
};
pub use control_flow_handlers::{
    collect_targeted_player_specs_from_filter, collect_targeted_player_specs_from_player_filter,
    compile_effects_in_iterated_object_context, compile_effects_in_iterated_player_context,
    compile_effects_preserving_last_effect, compile_if_do_with_opponent_did,
    compile_if_do_with_opponent_doesnt, compile_if_do_with_player_did,
    compile_if_do_with_player_doesnt, compile_repeat_process_body, compile_result_followup,
    compile_vote_sequence, effect_predicate_from_if_result,
    force_implicit_vote_token_controller_you, target_context_prelude_for_filter,
    with_preserved_lowering_context,
};
pub use effect_dispatch::compile_effect;
pub use effect_handlers::compile_delayed_trigger_spec;
#[cfg(test)]
pub use ironsmith_compiler_resolve::SpanMappingContext;
pub use iterated_player_validation::{
    choose_spec_mentions_iterated_player, condition_mentions_iterated_player,
    effect_mentions_iterated_player, effects_contain_pending_effect_metric,
    object_filter_mentions_iterated_player, value_mentions_iterated_player,
};
pub use player_effect_helpers::{
    LoweredSubject, SubjectRole, compile_player_effect_from_resolved_filter,
    compile_player_role_effect,
};
pub use prepared_effects::{
    bind_returned_attachment_history_to_triggering_object,
    compile_condition_from_predicate_ast_with_env,
    materialize_prepared_effects_with_trigger_context, materialize_prepared_statement_effects,
    materialize_prepared_triggered_effects,
};
#[cfg(any(test, feature = "test-support"))]
pub use prepared_effects::{compile_statement_effects, compile_statement_effects_with_imports};
#[cfg(test)]
pub use tag_support::collect_tag_spans_from_effect;
pub use tag_support::{
    choose_spec_references_exiled_tag, collect_tag_spans_from_effects_with_context,
    effect_references_it_tag, effect_references_its_controller, effect_references_tag,
    effects_have_cross_arm_tag_dependency, effects_reference_it_tag,
    effects_reference_its_controller, effects_reference_tag,
    effects_reference_tag_in_object_position, filter_references_tag, is_exile_cost_collection_tag,
    is_revealed_collection_tag, is_searched_collection_tag,
    is_sentence_helper_exiled_collection_tag, predicate_references_tag,
};
pub use trigger_support::{
    compile_trigger_effects, compile_trigger_effects_with_imports, compile_trigger_spec,
    ensure_concrete_trigger_spec, inferred_trigger_player_filter,
    trigger_binds_player_reference_context, trigger_supports_event_value,
};

/// Resolve a predicate into a condition using the lowering context's references.
///
/// The resolution itself belongs to reference binding and lives in the
/// resolver; lowering reaches it through this thin adapter because that is
/// where the reference environment is held.
pub fn compile_condition_from_predicate_ast(
    predicate: &PredicateAst,
    ctx: &mut EffectLoweringContext,
    saved_last_tag: &Option<TagKey>,
) -> Result<Condition, CardTextError> {
    let refs = current_reference_env(ctx);
    ironsmith_compiler_resolve::predicate_conditions::resolve_condition_from_predicate(
        predicate,
        &refs,
        saved_last_tag,
    )
}

pub fn compile_effects(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let annotated = annotate_effect_sequence(
        effects,
        &ReferenceImports::from_lowering_frame(&ctx.lowering_frame()),
        EffectReferenceResolutionConfig {
            allow_life_event_value: ctx.allow_life_event_value,
            bind_unbound_x_to_last_effect: ctx.bind_unbound_x_to_last_effect,
            initial_last_effect_id: ctx.last_effect_id,
            initial_iterated_player: ctx.iterated_player,
            force_auto_tag_object_targets: ctx.force_auto_tag_object_targets
                || ctx.auto_tag_object_targets,
            force_export_last_memory_effect_id: false,
        },
        ctx.id_gen_context(),
    )?;
    compile_annotated_effects_with_context(&annotated, ctx)
}

pub fn compile_annotated_effects_with_context(
    annotated: &AnnotatedEffectSequence,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut compiled = Vec::new();
    let mut choices = Vec::new();
    let mut idx = 0;
    let effective_force_auto_tag_object_targets =
        ctx.force_auto_tag_object_targets || ctx.auto_tag_object_targets;

    while idx < annotated.effects.len() {
        let current = &annotated.effects[idx];
        apply_local_reference_env_for_effect(ctx, &current.in_env, &current.effect);
        let suppress_force_for_power_self_damage =
            preserves_existing_it_for_power_self_damage_followup(
                &current.effect,
                annotated.effects.get(idx + 1).map(|next| &next.effect),
            );
        ctx.auto_tag_object_targets = (effective_force_auto_tag_object_targets
            && !suppress_force_for_power_self_damage)
            || current.auto_tag_object_targets;

        if let Some((effect_sequence, effect_choices, consumed)) =
            compile_vote_sequence(&annotated.effects[idx..], ctx)?
        {
            merge_compiled_choices(&mut choices, &effect_sequence, effect_choices);
            compiled.extend(effect_sequence);
            apply_local_reference_env(ctx, &annotated.effects[idx + consumed - 1].out_env);
            idx += consumed;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_opponent_doesnt(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            merge_compiled_choices(&mut choices, &effect_sequence, effect_choices);
            compiled.extend(effect_sequence);
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_player_doesnt(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            merge_compiled_choices(&mut choices, &effect_sequence, effect_choices);
            compiled.extend(effect_sequence);
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_opponent_did(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            merge_compiled_choices(&mut choices, &effect_sequence, effect_choices);
            compiled.extend(effect_sequence);
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) = compile_if_do_with_player_did(
                &current.effect,
                &annotated.effects[idx + 1].effect,
                ctx,
            )?
        {
            merge_compiled_choices(&mut choices, &effect_sequence, effect_choices);
            compiled.extend(effect_sequence);
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        if idx + 1 < annotated.effects.len()
            && let Some((effect_sequence, effect_choices)) =
                compile_result_followup(&current.effect, &annotated.effects[idx + 1].effect, ctx)?
        {
            merge_compiled_choices(&mut choices, &effect_sequence, effect_choices);
            compiled.extend(effect_sequence);
            apply_local_reference_env(ctx, &annotated.effects[idx + 1].out_env);
            idx += 2;
            continue;
        }

        ctx.reserve_object_result_tag(current.out_env.known_last_object_tag().cloned());
        let (mut effect_list, effect_choices) = compile_effect(&current.effect, ctx)?;
        ctx.reserve_object_result_tag(None);
        if let Some(id) = current.assigned_effect_id
            && !effect_list.is_empty()
        {
            control_flow_handlers::assign_effect_result_id_for_ast(
                &mut effect_list,
                &current.effect,
                id,
                "missing final effect while assigning event id (annotated effect)",
            )?;
        }
        let effect_list_is_empty = effect_list.is_empty();
        merge_compiled_choices(&mut choices, &effect_list, effect_choices);
        compiled.extend(effect_list);
        let concrete_runtime_player = ctx.last_player_filter.clone();
        let mut frame_out = current.out_env.to_lowering_frame(false, false);
        if annotated
            .effects
            .get(idx + 1)
            .is_some_and(|next| effect_has_anaphoric_player_subject(&next.effect))
            && current.out_env.known_last_player_filter() == Some(&PlayerFilter::Opponent)
            && matches!(concrete_runtime_player, Some(PlayerFilter::TaggedPlayer(_)))
        {
            frame_out.last_player_filter = concrete_runtime_player;
        }
        if current.assigned_effect_id.is_some() && effect_list_is_empty {
            frame_out.last_effect_id = None;
        }
        ctx.apply_reference_frame(frame_out);
        idx += 1;
    }

    let compiled = prepend_missing_target_choice_prelude(compiled, &choices);
    Ok((compiled, choices))
}

/// Merge a lowered child program's choices without erasing target
/// occurrences that the child deliberately kept distinct.
///
/// Most lowering paths return a set-like choice list, so the historical
/// `push_choice` behavior remains correct. Coordinated clauses are the one
/// important exception: two explicit target phrases may have equal-looking
/// `ChooseSpec`s while still being separate target slots. Their lowering path
/// signals that distinction by returning duplicate occurrences. Preserve that
/// multiset as it crosses enclosing sequence/control-flow boundaries.
fn merge_compiled_choices(
    choices: &mut Vec<ChooseSpec>,
    compiled: &[Effect],
    incoming: Vec<ChooseSpec>,
) {
    let preserves_explicit_occurrences = incoming
        .iter()
        .enumerate()
        .any(|(idx, choice)| incoming[idx + 1..].iter().any(|later| later == choice))
        && compiled.iter().any(effect_contains_coordinated_sequence);
    if preserves_explicit_occurrences {
        choices.extend(incoming);
    } else {
        for choice in incoming {
            push_choice(choices, choice);
        }
    }
}

fn effect_contains_coordinated_sequence(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .is_some_and(|sequence| sequence.surface.is_coordinated())
    {
        return true;
    }

    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && effect_contains_coordinated_sequence(child) {
            found = true;
        }
    });
    found
}

fn assign_effect_result_id(
    effects: &mut Vec<Effect>,
    id: EffectId,
    error_message: &str,
) -> Result<(), CardTextError> {
    let Some(last) = effects.pop() else {
        return Err(CardTextError::InvariantViolation(error_message.to_string()));
    };
    effects.push(Effect::with_id(id.0, last));
    Ok(())
}

pub fn compile_effects_with_explicit_frame(
    effects: &[EffectAst],
    id_gen: &mut IdGenContext,
    frame: LoweringFrame,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>, LoweringFrame), CardTextError> {
    let mut ctx = EffectLoweringContext::from_parts(id_gen.clone(), frame);
    let (compiled, choices) = compile_effects(effects, &mut ctx)?;
    *id_gen = ctx.id_gen_context();
    let frame_out = ctx.lowering_frame();
    Ok((compiled, choices, frame_out))
}

fn prepend_missing_target_choice_prelude(
    mut compiled: Vec<Effect>,
    choices: &[ChooseSpec],
) -> Vec<Effect> {
    let mut missing_targets = Vec::new();
    for choice in choices {
        if !choice.is_target() {
            continue;
        }
        let correlated_fight_target = compiled.iter().any(|effect| {
            let Some(fight) = effect.downcast_ref::<crate::effects::FightEffect>() else {
                return false;
            };
            (matches!(&fight.creature1, ChooseSpec::Tagged(_)) && fight.creature2 == *choice)
                || (matches!(&fight.creature2, ChooseSpec::Tagged(_)) && fight.creature1 == *choice)
        });
        let exposed_count = compiled
            .iter()
            .filter(|effect| effect_exposes_target_choice(effect, choice))
            .count();
        if correlated_fight_target || exposed_count != 1 {
            missing_targets.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    if missing_targets.is_empty() {
        return compiled;
    }
    if compiled.iter().any(effect_contains_exchange_control) {
        return compiled;
    }
    missing_targets.append(&mut compiled);
    missing_targets
}

fn effect_exposes_target_choice(effect: &Effect, choice: &ChooseSpec) -> bool {
    if effect.target_spec().is_some_and(|spec| spec == choice) {
        return true;
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return effect_exposes_target_choice(&tagged.effect, choice);
    }
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return effect_exposes_target_choice(&with_id.effect, choice);
    }
    if let Some(unless_pays) = effect.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>() {
        return unless_pays
            .effects
            .iter()
            .any(|child| effect_exposes_target_choice(child, choice));
    }
    effect
        .downcast_ref::<crate::effects::SequenceEffect>()
        .filter(|sequence| {
            sequence.surface.is_coordinated()
                || matches!(
                    sequence.surface,
                    ironsmith_core::SequenceSurface::SentenceLeadingThen
                        | ironsmith_core::SequenceSurface::CommaThen
                )
        })
        .is_some_and(|sequence| {
            sequence
                .effects
                .iter()
                .any(|child| effect_exposes_target_choice(child, choice))
        })
}

fn effect_contains_exchange_control(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::ExchangeControlEffect>()
        .is_some()
    {
        return true;
    }
    effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .is_some_and(|tagged| effect_contains_exchange_control(&tagged.effect))
}

fn preserve_chooser_relative_player_filters(
    original: &ObjectFilter,
    resolved: &mut ObjectFilter,
    chooser: &PlayerFilter,
) {
    if !matches!(
        chooser,
        PlayerFilter::Opponent | PlayerFilter::Target(_) | PlayerFilter::IteratedPlayer
    ) {
        return;
    }

    if matches!(original.owner, Some(PlayerFilter::IteratedPlayer)) {
        resolved.owner = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(original.controller, Some(PlayerFilter::IteratedPlayer)) {
        resolved.controller = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(original.cast_by, Some(PlayerFilter::IteratedPlayer)) {
        resolved.cast_by = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(original.targets_player, Some(PlayerFilter::IteratedPlayer)) {
        resolved.targets_player = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(
        original.targets_only_player,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        resolved.targets_only_player = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(
        original.attacking_player_or_planeswalker_controlled_by,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        resolved.attacking_player_or_planeswalker_controlled_by =
            Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(original.protected_by, Some(PlayerFilter::IteratedPlayer)) {
        resolved.protected_by = Some(PlayerFilter::IteratedPlayer);
    }
    if matches!(
        original.entered_battlefield_controller,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        resolved.entered_battlefield_controller = Some(PlayerFilter::IteratedPlayer);
    }
    if original
        .counters_put_on_this_turn
        .as_ref()
        .is_some_and(|constraint| {
            matches!(constraint.source_controller, PlayerFilter::IteratedPlayer)
        })
        && let Some(constraint) = resolved.counters_put_on_this_turn.as_mut()
    {
        constraint.source_controller = PlayerFilter::IteratedPlayer;
    }
    if matches!(
        original.attached_to_player,
        Some(PlayerFilter::IteratedPlayer)
    ) {
        resolved.attached_to_player = Some(PlayerFilter::IteratedPlayer);
    }
    if let (Some(original_targets), Some(resolved_targets)) = (
        original.targets_object.as_deref(),
        resolved.targets_object.as_deref_mut(),
    ) {
        preserve_chooser_relative_player_filters(original_targets, resolved_targets, chooser);
    }
    if let (Some(original_targets), Some(resolved_targets)) = (
        original.targets_only_object.as_deref(),
        resolved.targets_only_object.as_deref_mut(),
    ) {
        preserve_chooser_relative_player_filters(original_targets, resolved_targets, chooser);
    }
    if let (Some(original_attached_to), Some(resolved_attached_to)) = (
        original.attached_to_object.as_deref(),
        resolved.attached_to_object.as_deref_mut(),
    ) {
        preserve_chooser_relative_player_filters(
            original_attached_to,
            resolved_attached_to,
            chooser,
        );
    }
    if let (Some(original_partner), Some(resolved_partner)) = (
        original.blocked_or_was_blocked_by_this_turn.as_deref(),
        resolved.blocked_or_was_blocked_by_this_turn.as_deref_mut(),
    ) {
        preserve_chooser_relative_player_filters(original_partner, resolved_partner, chooser);
    }
    for (original_nested, resolved_nested) in original
        .no_shared_creature_types_with
        .iter()
        .zip(resolved.no_shared_creature_types_with.iter_mut())
    {
        preserve_chooser_relative_player_filters(original_nested, resolved_nested, chooser);
    }
    for (original_relation, resolved_relation) in original
        .characteristic_relations
        .iter()
        .zip(resolved.characteristic_relations.iter_mut())
    {
        preserve_chooser_relative_player_filters(
            &original_relation.comparison,
            &mut resolved_relation.comparison,
            chooser,
        );
    }
    for (original_any_of, resolved_any_of) in original.any_of.iter().zip(resolved.any_of.iter_mut())
    {
        preserve_chooser_relative_player_filters(original_any_of, resolved_any_of, chooser);
    }
}

fn bind_relative_iterated_player_filters_to_chooser(
    filter: &mut ObjectFilter,
    chooser: &PlayerFilter,
) {
    if matches!(chooser, PlayerFilter::IteratedPlayer) {
        return;
    }
    let chooser = as_followup_player_alias(chooser.clone());

    for relative in [
        &mut filter.owner,
        &mut filter.controller,
        &mut filter.cast_by,
        &mut filter.targets_player,
        &mut filter.targets_only_player,
        &mut filter.attacking_player_or_planeswalker_controlled_by,
        &mut filter.protected_by,
        &mut filter.entered_battlefield_controller,
        &mut filter.attached_to_player,
    ] {
        if let Some(relative) = relative.as_mut() {
            bind_relative_iterated_player_filter_to_player_filter(relative, &chooser);
        }
    }
    if let Some(constraint) = filter.counters_put_on_this_turn.as_mut() {
        bind_relative_iterated_player_filter_to_player_filter(
            &mut constraint.source_controller,
            &chooser,
        );
    }
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        bind_relative_iterated_player_filters_to_chooser(targets, &chooser);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        bind_relative_iterated_player_filters_to_chooser(targets, &chooser);
    }
    if let Some(attached_to) = filter.attached_to_object.as_deref_mut() {
        bind_relative_iterated_player_filters_to_chooser(attached_to, &chooser);
    }
    if let Some(combat_partner) = filter.blocked_or_was_blocked_by_this_turn.as_deref_mut() {
        bind_relative_iterated_player_filters_to_chooser(combat_partner, &chooser);
    }
    for nested in &mut filter.no_shared_creature_types_with {
        bind_relative_iterated_player_filters_to_chooser(nested, &chooser);
    }
    for relation in &mut filter.characteristic_relations {
        bind_relative_iterated_player_filters_to_chooser(&mut relation.comparison, &chooser);
    }
    for any_of in &mut filter.any_of {
        bind_relative_iterated_player_filters_to_chooser(any_of, &chooser);
    }
}

fn bind_relative_iterated_player_to_last_player_filter(
    player_filter: &mut PlayerFilter,
    filter: &mut ObjectFilter,
    last_player_filter: &PlayerFilter,
) {
    if last_player_filter.mentions_iterated_player() {
        return;
    }

    if matches!(player_filter, PlayerFilter::IteratedPlayer) {
        *player_filter = as_followup_player_alias(last_player_filter.clone());
    }
    bind_relative_iterated_player_filters_to_chooser(filter, last_player_filter);
}

fn bind_relative_iterated_player_filter_to_player_filter(
    relative: &mut PlayerFilter,
    player_filter: &PlayerFilter,
) {
    if matches!(player_filter, PlayerFilter::IteratedPlayer) {
        return;
    }
    match relative {
        PlayerFilter::IteratedPlayer => {
            *relative = as_followup_player_alias(player_filter.clone());
        }
        PlayerFilter::AliasedTarget(inner)
            if matches!(inner.as_ref(), PlayerFilter::IteratedPlayer) =>
        {
            // The alias wrapper means "the previously announced target." If
            // the enclosing trigger instead binds a stable contextual player
            // (for example, its persistent chosen player), preserve that
            // participant directly. Leaving `AliasedTarget(ChosenPlayer)`
            // would incorrectly require a target announcement at runtime.
            *relative = as_followup_player_alias(player_filter.clone());
        }
        PlayerFilter::Target(inner) | PlayerFilter::AliasedTarget(inner) => {
            bind_relative_iterated_player_filter_to_player_filter(inner, player_filter);
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | PlayerFilter::HasMoreLifeThanYou { base }
        | PlayerFilter::MaxSpeed { base, .. }
        | PlayerFilter::WasDealtDamageBySourceThisGame { base }
        | PlayerFilter::LostLifeThisTurn { base } => {
            bind_relative_iterated_player_filter_to_player_filter(base, player_filter);
        }
        PlayerFilter::WasDealtCombatDamageByDistinctSourcesThisTurn { base, sources, .. } => {
            bind_relative_iterated_player_filter_to_player_filter(base, player_filter);
            bind_relative_iterated_player_filters_to_chooser(sources, player_filter);
        }
        PlayerFilter::Excluding { base, excluded } => {
            bind_relative_iterated_player_filter_to_player_filter(base, player_filter);
            bind_relative_iterated_player_filter_to_player_filter(excluded, player_filter);
        }
        _ => {}
    }
}

pub fn bind_relative_iterated_player_in_value_to_player_filter(
    value: &mut Value,
    player_filter: &PlayerFilter,
) {
    match value {
        Value::SurfaceHinted { value, .. } => {
            bind_relative_iterated_player_in_value_to_player_filter(value, player_filter);
        }
        Value::Add(left, right) => {
            bind_relative_iterated_player_in_value_to_player_filter(left, player_filter);
            bind_relative_iterated_player_in_value_to_player_filter(right, player_filter);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner) => {
            bind_relative_iterated_player_in_value_to_player_filter(inner, player_filter);
        }
        Value::Min(left, right) => {
            bind_relative_iterated_player_in_value_to_player_filter(left, player_filter);
            bind_relative_iterated_player_in_value_to_player_filter(right, player_filter);
        }
        Value::Count(filter)
        | Value::CountScaled(filter, _)
        | Value::GreatestCount(filter)
        | Value::GreatestSharedCreatureTypeCount(filter)
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
        | Value::ColorPairsAmong(filter)
        | Value::DistinctCounterTypesAmong(filter)
        | Value::DistinctNames(filter)
        | Value::DistinctPowers(filter) => {
            bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
        }
        Value::StaticAbilitiesAmong { filter, .. } => {
            bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
        }
        Value::TurnHistoryCount(query) => {
            use ironsmith_core::TurnHistoryCount;

            match query {
                TurnHistoryCount::Died { filter, .. }
                | TurnHistoryCount::EnteredBattlefield(filter) => {
                    bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
                }
                TurnHistoryCount::TokensCreated(player)
                | TurnHistoryCount::PlayersAttackedThisCombat(player)
                | TurnHistoryCount::OpponentsAttacked(player)
                | TurnHistoryCount::PlayersDiscarded(player)
                | TurnHistoryCount::PlayersDealtDamage(player)
                | TurnHistoryCount::DiscardedOrCycled(player)
                | TurnHistoryCount::Cycled(player)
                | TurnHistoryCount::PlayersLostLife(player)
                | TurnHistoryCount::UntappedLandsAtTurnStart(player)
                | TurnHistoryCount::Descended(player)
                | TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(player) => {
                    bind_relative_iterated_player_filter_to_player_filter(player, player_filter);
                }
                TurnHistoryCount::PutIntoGraveyard { owner, .. } => {
                    bind_relative_iterated_player_filter_to_player_filter(owner, player_filter);
                }
                TurnHistoryCount::MovedZones { filter, .. }
                | TurnHistoryCount::CountersPutOn { filter, .. } => {
                    bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
                }
                TurnHistoryCount::Sacrificed { player, filter }
                | TurnHistoryCount::CreaturesAttackedWith { player, filter } => {
                    bind_relative_iterated_player_filter_to_player_filter(player, player_filter);
                    bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
                }
                TurnHistoryCount::PlayersDealtCombatDamageBy { players, sources } => {
                    bind_relative_iterated_player_filter_to_player_filter(players, player_filter);
                    bind_relative_iterated_player_filters_to_chooser(sources, player_filter);
                }
                TurnHistoryCount::SpellsCast { player, filter, .. } => {
                    bind_relative_iterated_player_filter_to_player_filter(player, player_filter);
                    bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
                }
                TurnHistoryCount::DamageDealtToSource | TurnHistoryCount::DamageDealtBySource => {}
            }
        }
        Value::CreaturesDiedThisTurnControlledBy(player)
        | Value::CountPlayers(player)
        | Value::CountPlayersWithCardsInHandAtLeast(player, _)
        | Value::PartySize(player)
        | Value::LifeTotal(player)
        | Value::LifeTotalAsTurnBegan(player)
        | Value::LifeTotalDifference(player)
        | Value::UnspentMana(player)
        | Value::Speed(player)
        | Value::StartingLifeTotal(player)
        | Value::CardsInHand(player)
        | Value::CardsInLibrary(player)
        | Value::DevotionToChosenColor(player)
        | Value::LifeGainedThisTurn(player)
        | Value::LifeLostThisTurn(player)
        | Value::CardsDiscardedThisTurn(player)
        | Value::AttractionsVisitedThisTurn(player)
        | Value::DamageDealtToPlayersThisTurn(player)
        | Value::NoncombatDamageDealtToPlayersThisTurn(player)
        | Value::MaxCardsDrawnThisTurn(player)
        | Value::MaxDiceRolledThisTurn(player)
        | Value::LandsEnteredBattlefieldThisTurn(player)
        | Value::MaxCardsInHand(player)
        | Value::CardsInGraveyard(player)
        | Value::SpellsCastThisTurn(player)
        | Value::SpellsCastBeforeThisTurn(player)
        | Value::CommanderCastCount(player)
        | Value::CardTypesInGraveyard(player)
        | Value::PlayerCounters(player, _)
        | Value::PlayerVoteCount(player)
        | Value::Devotion { player, .. }
        | Value::HalfLifeTotalRoundedUp(player)
        | Value::HalfLifeTotalRoundedDown(player)
        | Value::HalfStartingLifeTotalRoundedUp(player)
        | Value::HalfStartingLifeTotalRoundedDown(player) => {
            bind_relative_iterated_player_filter_to_player_filter(player, player_filter);
        }
        Value::PlayersWhoControlMoreThanYou { players, filter }
        | Value::PlayersWhoControlAtLeastMoreThanYou {
            players, filter, ..
        }
        | Value::SpellsCastThisTurnMatching {
            player: players,
            filter,
            ..
        }
        | Value::TotalManaValueOfSpellsCastThisTurnMatching {
            player: players,
            filter,
            ..
        } => {
            bind_relative_iterated_player_filter_to_player_filter(players, player_filter);
            bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
        }
        Value::NoncombatDamageDealtBySourcesControlledThisTurn { player, .. } => {
            bind_relative_iterated_player_filter_to_player_filter(player, player_filter);
        }
        Value::PowerOf(spec)
        | Value::ToughnessOf(spec)
        | Value::ManaValueOf(spec)
        | Value::ManaSymbolsInManaCostOf { spec, .. }
        | Value::CountersOn(spec, _) => {
            bind_relative_iterated_player_in_choose_spec_to_player_filter(spec, player_filter);
        }
        _ => {}
    }
}

fn bind_relative_iterated_player_in_choose_spec_to_player_filter(
    spec: &mut ChooseSpec,
    player_filter: &PlayerFilter,
) {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _) => {
            bind_relative_iterated_player_in_choose_spec_to_player_filter(spec, player_filter);
        }
        ChooseSpec::Player(player)
        | ChooseSpec::PlayerOrPlaneswalker(player)
        | ChooseSpec::EachPlayer(player) => {
            if matches!(player, PlayerFilter::IteratedPlayer)
                && !matches!(player_filter, PlayerFilter::IteratedPlayer)
            {
                *player = as_followup_player_alias(player_filter.clone());
            }
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            bind_relative_iterated_player_filters_to_chooser(filter, player_filter);
        }
        _ => {}
    }
}

pub fn resolve_player_scoped_value(
    value: &Value,
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
) -> Result<(Value, PlayerFilter, Vec<ChooseSpec>), CardTextError> {
    let subject = LoweredSubject::resolve_affected_player(
        player,
        ctx,
        allow_target,
        allow_target_opponent,
        track_last_player_filter,
    )?;
    let value = subject.resolve_object_refs_and_bind_player_refs_in_value(value, ctx)?;
    Ok((value, subject.into_player_filter(), subject.into_choices()))
}

fn choose_followup_player_filter(
    filter: &ObjectFilter,
    chooser: &PlayerFilter,
) -> Option<PlayerFilter> {
    let inferred = player_filter_from_object_filter(filter);
    if inferred
        .as_ref()
        .is_some_and(PlayerFilter::mentions_iterated_player)
        && matches!(
            chooser,
            PlayerFilter::Target(_) | PlayerFilter::Opponent | PlayerFilter::Specific(_)
        )
    {
        Some(chooser.clone())
    } else {
        inferred.or_else(|| Some(chooser.clone()))
    }
}

pub fn hand_exile_filter_and_count(
    target: &TargetAst,
    ctx: &EffectLoweringContext,
) -> Result<Option<(ObjectFilter, ChoiceCount, Vec<Zone>)>, CardTextError> {
    let (filter, count) = match target {
        TargetAst::Object(filter, _, _) => (filter, ChoiceCount::exactly(1)),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => (filter, *count),
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
    normalize_hand_or_graveyard_cross_zone_filter(&mut resolved_filter);
    let zones = hand_or_graveyard_choice_zones(&resolved_filter);
    let Some(zones) = zones else {
        return Ok(None);
    };
    Ok(Some((resolved_filter, count, zones)))
}

fn normalize_hand_or_graveyard_cross_zone_filter(filter: &mut ObjectFilter) {
    if filter.any_of.is_empty() {
        return;
    }

    let branch_zones = filter
        .any_of
        .iter()
        .filter_map(|option| option.zone)
        .collect::<Vec<_>>();
    let has_hand_or_graveyard_zone = branch_zones
        .iter()
        .any(|zone| matches!(zone, Zone::Hand | Zone::Graveyard));
    // A parsed union may retain either the implicit battlefield default or
    // its first branch as the outer zone. The executable choice must instead
    // search every authored branch zone.
    if has_hand_or_graveyard_zone
        && (filter.zone == Some(Zone::Battlefield)
            || filter.zone.is_some_and(|zone| branch_zones.contains(&zone)))
    {
        filter.zone = None;
        filter.controller = None;
    }
}

fn hand_or_graveyard_choice_zones(filter: &ObjectFilter) -> Option<Vec<Zone>> {
    if filter.zone == Some(Zone::Hand) {
        return Some(vec![Zone::Hand]);
    }
    if filter.zone.is_some() || filter.any_of.is_empty() {
        return None;
    }

    let mut zones = Vec::new();
    for option in &filter.any_of {
        let zone = option.zone?;
        if !matches!(zone, Zone::Hand | Zone::Graveyard) {
            return None;
        }
        if !zones.contains(&zone) {
            zones.push(zone);
        }
    }
    if zones.contains(&Zone::Hand) {
        Some(zones)
    } else {
        None
    }
}

fn merge_cross_zone_player_scope(
    outer: &ObjectFilter,
    shared: &ObjectFilter,
) -> Option<ObjectFilter> {
    fn merge_scope_field<T: PartialEq>(base: &mut Option<T>, overlay: Option<T>) -> bool {
        match (base.as_ref(), overlay) {
            (_, None) => true,
            (None, Some(value)) => {
                *base = Some(value);
                true
            }
            (Some(existing), Some(value)) => existing == &value,
        }
    }

    let mut remaining_outer = outer.clone();
    let owner = remaining_outer.owner.take();
    let controller = remaining_outer.controller.take();
    let cast_by = remaining_outer.cast_by.take();
    if remaining_outer != ObjectFilter::default() {
        return None;
    }

    let mut merged = shared.clone();
    if !merge_scope_field(&mut merged.owner, owner)
        || !merge_scope_field(&mut merged.controller, controller)
        || !merge_scope_field(&mut merged.cast_by, cast_by)
    {
        return None;
    }
    Some(merged)
}

fn strip_choice_zones_from_filter(filter: &mut ObjectFilter, zones: &[Zone]) {
    if zones.len() > 1 && filter.zone.is_some_and(|zone| zones.contains(&zone)) {
        filter.zone = None;
    }

    // Zone-distributed branches such as "an Aura or Equipment card from your
    // hand or graveyard" carry the same non-zone predicate twice. Factor that
    // predicate back out before lowering so rendering and runtime matching do
    // not duplicate it or accidentally retain only the first zone.
    if !filter.any_of.is_empty() {
        let mut shared = None::<ObjectFilter>;
        let factorizable = filter.any_of.iter().all(|option| {
            let mut bare = option.clone();
            let Some(zone) = bare.zone.take() else {
                return false;
            };
            if !zones.contains(&zone) {
                return false;
            }
            if let Some(existing) = &shared {
                existing == &bare
            } else {
                shared = Some(bare);
                true
            }
        });
        if factorizable && let Some(shared) = shared {
            let mut outer = filter.clone();
            outer.any_of.clear();

            // Zone-only arms contribute no object predicate. Preserve any
            // owner, type, or exclusion constraints carried by the outer
            // filter instead of replacing them with an empty arm.
            if shared == ObjectFilter::default() {
                *filter = outer;
                return;
            }
            if outer == ObjectFilter::default() || outer == shared {
                *filter = shared;
                return;
            }
            // Some grammar paths retain only the player scope outside the
            // zone-distributed predicate. Fold that scope into the shared
            // predicate instead of rendering both as alternatives.
            if let Some(merged) = merge_cross_zone_player_scope(&outer, &shared) {
                *filter = merged;
                return;
            }
        }
    }

    filter.any_of.retain(|option| {
        let mut bare = option.clone();
        let Some(zone) = bare.zone.take() else {
            return true;
        };
        bare != ObjectFilter::default() || !zones.contains(&zone)
    });
}

pub fn normalized_hand_or_graveyard_choice_filter(
    filter: &ObjectFilter,
) -> Option<(ObjectFilter, Vec<Zone>)> {
    let mut filter = filter.clone();
    normalize_hand_or_graveyard_cross_zone_filter(&mut filter);
    let zones = hand_or_graveyard_choice_zones(&filter)?;
    strip_choice_zones_from_filter(&mut filter, &zones);
    Some((filter, zones))
}

pub fn lower_hand_exile_target(
    target: &TargetAst,
    face_down: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let Some((mut filter, count, zones)) = hand_exile_filter_and_count(target, ctx)? else {
        return Ok(None);
    };
    strip_choice_zones_from_filter(&mut filter, &zones);

    let mut chooser = filter
        .owner
        .clone()
        .or_else(|| filter.controller.clone())
        .unwrap_or(PlayerFilter::You);

    if ctx.iterated_player && matches!(chooser, PlayerFilter::Target(_)) {
        chooser = PlayerFilter::IteratedPlayer;
        if matches!(filter.owner, Some(PlayerFilter::Target(_))) {
            filter.owner = Some(PlayerFilter::IteratedPlayer);
        }
        if matches!(filter.controller, Some(PlayerFilter::Target(_))) {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
    } else {
        bind_relative_iterated_player_filters_to_chooser(&mut filter, &chooser);
    }

    let (mut prelude, choices) = target_context_prelude_for_filter(&filter);
    let tag = ctx.next_tag("exiled");
    let tag_key: TagKey = tag.as_str().into();
    ctx.last_object_tag = Some(tag.clone());
    ctx.last_player_filter = Some(chooser.clone());

    prelude.push(Effect::new(
        crate::effects::ChooseObjectsEffect::new(filter, count, chooser, tag_key.clone())
            .in_zones(zones),
    ));
    prelude.push(Effect::new(
        crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag_key))
            .with_face_down(face_down),
    ));
    Ok(Some((prelude, choices)))
}

pub fn lower_counted_non_target_exile_target(
    target: &TargetAst,
    face_down: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let (filter, count) = match target {
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() && !count.is_single() =>
            {
                (filter, *count)
            }
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
    let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
    if choice_zone != Zone::Library {
        return Ok(None);
    }

    let mut chooser = resolved_filter
        .owner
        .clone()
        .or_else(|| resolved_filter.controller.clone())
        .unwrap_or(PlayerFilter::You);

    if ctx.iterated_player && matches!(chooser, PlayerFilter::Target(_)) {
        chooser = PlayerFilter::IteratedPlayer;
        if matches!(resolved_filter.owner, Some(PlayerFilter::Target(_))) {
            resolved_filter.owner = Some(PlayerFilter::IteratedPlayer);
        }
        if matches!(resolved_filter.controller, Some(PlayerFilter::Target(_))) {
            resolved_filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
    } else {
        bind_relative_iterated_player_filters_to_chooser(&mut resolved_filter, &chooser);
    }

    if choice_zone == Zone::Battlefield
        && resolved_filter.controller.is_none()
        && resolved_filter.tagged_constraints.is_empty()
    {
        resolved_filter.controller = Some(chooser.clone());
    }

    let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
    let tag = ctx.next_tag("exiled");
    let tag_key: TagKey = tag.as_str().into();
    ctx.last_object_tag = Some(tag.clone());
    ctx.last_player_filter = Some(chooser.clone());

    prelude.push(Effect::new(
        crate::effects::ChooseObjectsEffect::new(resolved_filter, count, chooser, tag_key.clone())
            .in_zone(choice_zone)
            .top_only(),
    ));
    prelude.push(Effect::new(
        crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag_key))
            .with_face_down(face_down),
    ));
    Ok(Some((prelude, choices)))
}

pub fn lower_single_non_target_exile_target(
    target: &TargetAst,
    face_down: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let (filter, count) = match target {
        TargetAst::Object(filter, explicit_target_span, _) if explicit_target_span.is_none() => {
            (filter, ChoiceCount::exactly(1))
        }
        TargetAst::WithCount(inner, count) if count.is_single() => match inner.as_ref() {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() =>
            {
                (filter, *count)
            }
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };

    let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
    let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
    if choice_zone != Zone::Library {
        return Ok(None);
    }

    let mut chooser = resolved_filter
        .owner
        .clone()
        .or_else(|| resolved_filter.controller.clone())
        .unwrap_or(PlayerFilter::You);

    if ctx.iterated_player && matches!(chooser, PlayerFilter::Target(_)) {
        chooser = PlayerFilter::IteratedPlayer;
        if matches!(resolved_filter.owner, Some(PlayerFilter::Target(_))) {
            resolved_filter.owner = Some(PlayerFilter::IteratedPlayer);
        }
        if matches!(resolved_filter.controller, Some(PlayerFilter::Target(_))) {
            resolved_filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
    } else {
        bind_relative_iterated_player_filters_to_chooser(&mut resolved_filter, &chooser);
    }

    let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
    let tag = ctx.next_tag("exiled");
    let tag_key: TagKey = tag.as_str().into();
    ctx.last_object_tag = Some(tag.clone());
    ctx.last_player_filter = Some(chooser.clone());

    let choose =
        crate::effects::ChooseObjectsEffect::new(resolved_filter, count, chooser, tag_key.clone())
            .in_zone(choice_zone)
            .top_only();

    prelude.push(Effect::new(choose));
    prelude.push(Effect::new(
        crate::effects::ExileEffect::with_spec(ChooseSpec::Tagged(tag_key))
            .with_face_down(face_down),
    ));
    Ok(Some((prelude, choices)))
}

pub fn lower_may_imprint_from_hand_effect(
    effects: &[EffectAst],
    ctx: &EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    if effects.len() != 1 {
        return Ok(None);
    }

    let EffectAst::SubjectVerb(subject_verb) = &effects[0] else {
        return Ok(None);
    };
    let SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Exile {
        target, face_down, ..
    }) = &subject_verb.action
    else {
        return Ok(None);
    };
    if *face_down {
        return Ok(None);
    }

    let Some((filter, count, zones)) = hand_exile_filter_and_count(target, ctx)? else {
        return Ok(None);
    };
    if !count.is_single() || zones.len() != 1 || zones.first().copied() != Some(Zone::Hand) {
        return Ok(None);
    }
    let is_effect_controller = |player: &PlayerFilter| {
        matches!(player, PlayerFilter::You | PlayerFilter::EffectController)
    };
    if filter
        .owner
        .as_ref()
        .is_some_and(|owner| !is_effect_controller(owner))
        || filter
            .controller
            .as_ref()
            .is_some_and(|controller| !is_effect_controller(controller))
    {
        // ImprintFromHandEffect deliberately selects from the effect
        // controller's hand. Preserve the generic choose/exile lowering when
        // Oracle points at another player's hand (for example, a hand that was
        // just looked at), otherwise the filter's provenance is silently lost.
        return Ok(None);
    }

    Ok(Some((
        vec![Effect::new(
            crate::effects::cards::ImprintFromHandEffect::new(filter),
        )],
        Vec::new(),
    )))
}

fn resolve_effect_player_filter(
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
) -> Result<(PlayerFilter, Vec<ChooseSpec>), CardTextError> {
    let refs = current_reference_env(ctx);
    let (filter, choices) = match player {
        PlayerAst::Target if allow_target => (
            PlayerFilter::target_player(),
            vec![ChooseSpec::target_player()],
        ),
        PlayerAst::TargetOpponent if allow_target_opponent => (
            PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
            vec![ChooseSpec::target(ChooseSpec::Player(
                PlayerFilter::Opponent,
            ))],
        ),
        _ => (resolve_non_target_player_filter(player, &refs)?, Vec::new()),
    };

    if track_last_player_filter && !matches!(player, PlayerAst::Implicit) {
        let preserve_existing_non_you = matches!(player, PlayerAst::You)
            && ctx
                .last_player_filter
                .as_ref()
                .is_some_and(|existing| !is_you_player_filter(existing));
        if !preserve_existing_non_you {
            ctx.last_player_filter = Some(as_followup_player_alias(filter.clone()));
        }
    }
    Ok((filter, choices))
}

fn try_compile_simultaneous_each_player_scry(
    player_filter: PlayerFilter,
    inner_effects: &[Effect],
) -> Option<Effect> {
    if inner_effects.len() != 1 {
        return None;
    }
    let scry = inner_effects[0].downcast_ref::<crate::effects::ScryEffect>()?;
    if scry.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    Some(Effect::new(crate::effects::EachPlayerScryEffect::new(
        scry.count.clone(),
        player_filter,
    )))
}

fn compile_emblem_description(
    emblem: &EmblemDescriptionAst,
) -> Result<EmblemDescription, CardTextError> {
    let mut abilities = Vec::new();
    for ability in &emblem.abilities {
        match ability {
            EmblemAbilityAst::Static(static_abilities) => {
                for static_ability in static_abilities {
                    if let Ok(static_ability) = lower_static_ability_ast(static_ability.clone()) {
                        abilities.push(Ability::static_ability(static_ability));
                    }
                }
            }
            EmblemAbilityAst::Activated(ability) => {
                if let Ok(ability) = lower_parsed_ability(ability.clone()) {
                    abilities.push(ability);
                }
            }
            EmblemAbilityAst::Triggered {
                trigger,
                effects,
                trigger_limit_condition,
            } => {
                let parsed = assemble_parsed_triggered_ability(
                    trigger.clone(),
                    effects.clone(),
                    vec![Zone::Battlefield],
                    trigger_limit_condition.clone(),
                    None,
                    ReferenceImports::default(),
                );
                if let Ok(ability) = lower_parsed_ability(parsed) {
                    abilities.push(ability);
                }
            }
        }
    }
    Ok(EmblemDescription {
        name: "Emblem".to_string(),
        text: emblem.text.clone(),
        abilities,
    })
}

fn compile_exchange_life_totals_effect(
    player1: PlayerAst,
    player2: PlayerAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let subject1 = LoweredSubject::resolve_affected_player(player1, ctx, true, true, true)?;
    let subject2 = LoweredSubject::resolve_affected_player(player2, ctx, true, true, true)?;

    let effect = Effect::exchange_life_totals(
        subject1.clone_player_filter(),
        subject2.clone_player_filter(),
    );
    let mut choices = Vec::new();

    if subject1.choices().len() == 1
        && subject2.choices().len() == 1
        && subject1.choices()[0].base() == subject2.choices()[0].base()
        && subject1.choices()[0].is_target()
    {
        push_choice(
            &mut choices,
            subject1.choices()[0]
                .clone()
                .with_count(ChoiceCount::exactly(2)),
        );
    } else {
        for choice in subject1
            .into_choices()
            .into_iter()
            .chain(subject2.into_choices())
        {
            push_choice(&mut choices, choice);
        }
    }

    let mut effects = Vec::new();
    if effect.target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

fn compile_exchange_control_heterogeneous_effect(
    permanent1: &TargetAst,
    permanent2: &TargetAst,
    shared_type: Option<SharedTypeConstraintAst>,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (spec1, mut choices) =
        resolve_target_spec_with_choices(permanent1, &current_reference_env(ctx))?;
    let reference_tag = ctx.next_tag("exchange_first");
    let original_last_object_tag = ctx.last_object_tag.clone();
    ctx.last_object_tag = Some(reference_tag.clone());
    let (spec2, other_choices) =
        resolve_target_spec_with_choices(permanent2, &current_reference_env(ctx))?;
    ctx.last_object_tag = original_last_object_tag;
    for choice in other_choices {
        push_choice(&mut choices, choice);
    }

    let exchange = crate::effects::ExchangeControlEffect::new(spec1, spec2)
        .with_permanent1_reference_tag(reference_tag);
    let exchange = if let Some(shared_type) = shared_type {
        let constraint = match shared_type {
            SharedTypeConstraintAst::CardType => crate::effects::SharedTypeConstraint::CardType,
            SharedTypeConstraintAst::PermanentType => {
                crate::effects::SharedTypeConstraint::PermanentType
            }
        };
        exchange.with_shared_type(constraint)
    } else {
        exchange
    };

    let mut effect = Effect::new(exchange);
    let tag = ctx.next_tag("exchanged");
    effect = effect.tag(tag.clone());
    ctx.last_object_tag = Some(tag);
    Ok((vec![effect], choices))
}

fn compile_exchange_zones_effect(
    player: PlayerAst,
    zone1: Zone,
    zone2: Zone,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let subject = LoweredSubject::resolve_zone_owner(player, ctx, true, true, true)?;
    let effect = Effect::exchange_zones(subject.clone_player_filter(), zone1, zone2);
    let mut effects = Vec::new();
    if effect.target_spec().is_none() {
        for choice in subject.choices() {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, subject.into_choices()))
}

fn compile_exchange_text_boxes_effect(
    target: &TargetAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (spec, choices) = resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
    let effect = Effect::exchange_text_boxes(spec);
    let tag = ctx.next_tag("exchanged");
    ctx.last_object_tag = Some(tag.clone());
    Ok((vec![effect.tag(tag)], choices))
}

fn compile_exchange_value_operand(
    operand: &ExchangeValueAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(crate::effects::ExchangeValueOperand, Vec<ChooseSpec>), CardTextError> {
    match operand {
        ExchangeValueAst::LifeTotal(player) => {
            let subject = LoweredSubject::resolve_affected_player(*player, ctx, true, true, true)?;
            let (player_filter, choices) = subject.into_parts();
            Ok((
                crate::effects::ExchangeValueOperand::LifeTotal(player_filter),
                choices,
            ))
        }
        ExchangeValueAst::Stat { target, kind } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let operand = match kind {
                ExchangeValueKindAst::Power => crate::effects::ExchangeValueOperand::Power(spec),
                ExchangeValueKindAst::Toughness => {
                    crate::effects::ExchangeValueOperand::Toughness(spec)
                }
            };
            Ok((operand, choices))
        }
    }
}

fn compile_exchange_values_effect(
    left: &ExchangeValueAst,
    right: &ExchangeValueAst,
    duration: Until,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let (left, mut choices) = compile_exchange_value_operand(left, ctx)?;
    let (right, other_choices) = compile_exchange_value_operand(right, ctx)?;
    for choice in other_choices {
        push_choice(&mut choices, choice);
    }
    let effect = Effect::exchange_values(left, right, duration);
    let mut effects = Vec::new();
    if effect.target_spec().is_none() {
        for choice in &choices {
            effects.push(Effect::new(crate::effects::TargetOnlyEffect::new(
                choice.clone(),
            )));
        }
    }
    effects.push(effect);
    Ok((effects, choices))
}

fn current_reference_env(ctx: &EffectLoweringContext) -> ReferenceEnv {
    ctx.reference_env()
}

fn apply_local_reference_env(ctx: &mut EffectLoweringContext, env: &ReferenceEnv) {
    let reference_env: crate::cards::builders::ReferenceEnv = env.clone();
    ctx.apply_reference_env(&reference_env);
}

fn apply_local_reference_env_for_effect(
    ctx: &mut EffectLoweringContext,
    env: &ReferenceEnv,
    effect: &EffectAst,
) {
    let concrete_runtime_player = ctx.last_player_filter.clone();
    apply_local_reference_env(ctx, env);

    let anaphoric_player_subject = effect_has_anaphoric_player_subject(effect);
    let annotation_only_knows_opponent_class =
        env.known_last_player_filter() == Some(&PlayerFilter::Opponent);
    if anaphoric_player_subject
        && annotation_only_knows_opponent_class
        && matches!(concrete_runtime_player, Some(PlayerFilter::TaggedPlayer(_)))
    {
        // A singular authored `an opponent` is selected while the preceding
        // effect resolves.  Its compiler AST necessarily carries the broad
        // Opponent class, while materialization exports the exact selected
        // player as a tag.  Do not let the next annotation frame widen that
        // concrete export before an authored `that player`/`they` consumes
        // it.  Explicit Opponent subjects are intentionally unaffected and
        // still establish their own selection.
        ctx.last_player_filter = concrete_runtime_player;
    }
}

fn effect_has_anaphoric_player_subject(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(subject_verb)
            if subject_verb.subject.player == PlayerAst::That
    )
}

fn lower_granted_ability_grant_modifications(
    abilities: &[GrantedAbilityAst],
) -> Result<Vec<crate::continuous::Modification>, CardTextError> {
    let lowered = lower_granted_abilities_ast_to_object_abilities(abilities)?;
    let mut modifications = Vec::with_capacity(lowered.len());
    for ability in lowered {
        match ability.kind {
            crate::ability::AbilityKind::Static(static_ability) => {
                modifications.push(crate::continuous::Modification::AddAbility(static_ability));
            }
            _ => modifications.push(crate::continuous::Modification::AddAbilityGeneric(ability)),
        }
    }
    Ok(modifications)
}

fn granted_ability_mode_description(
    ability: &GrantedAbilityAst,
    spec: &ChooseSpec,
) -> Result<String, CardTextError> {
    if !matches!(spec, ChooseSpec::Source) {
        return Ok(String::new());
    }

    let display = match ability {
        GrantedAbilityAst::ThisAbility => "this ability".to_string(),
        GrantedAbilityAst::ParsedObjectAbility { display, .. } => display.clone(),
        GrantedAbilityAst::KeywordAction(action) => action.display_text(),
        _ => lower_granted_abilities_ast(std::slice::from_ref(ability))?
            .into_iter()
            .next()
            .map(|ability| ability.display())
            .unwrap_or_default(),
    };

    Ok(format!("This creature gains {display} until end of turn."))
}

pub fn tagged_alias_for_choice(effects: &[Effect], choice: &ChooseSpec) -> Option<TagKey> {
    for effect in effects {
        let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
            continue;
        };
        if let Some(target_spec) = tagged.effect.target_spec()
            && target_spec == choice
        {
            return Some(tagged.tag.clone());
        }
    }
    None
}

pub fn tag_object_target_effect(
    effect: Effect,
    spec: &ChooseSpec,
    ctx: &mut EffectLoweringContext,
    prefix: &str,
) -> Effect {
    // A quantified object phrase can be lowered from `Object` to `All` after
    // reference annotation has already reserved a result tag for it.  Keep
    // that complete affected set taggable just like an ordinary object
    // choice; otherwise a following plural reference (for example, "they
    // can't phase in") points at a tag that no runtime effect ever fills.
    let produces_object_results =
        choose_spec_targets_object(spec) || matches!(spec.base(), ChooseSpec::All(_));
    if ctx.auto_tag_object_targets && produces_object_results {
        let tag = ctx.next_tag(prefix);
        ctx.last_object_tag = Some(tag.clone());
        effect.tag(tag)
    } else {
        effect
    }
}

fn selected_object_filter(spec: &ChooseSpec) -> Option<&ObjectFilter> {
    match spec.unhinted() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => Some(filter),
        ChooseSpec::Target(inner)
        | ChooseSpec::WithCount(inner, _)
        | ChooseSpec::WithCountValue(inner, _, _) => selected_object_filter(inner),
        _ => None,
    }
}

/// Preserve the exact player associated with a selected object for a later
/// "that player" reference. A broad lexical filter such as `Opponent` is only
/// the legal set; once the object is selected, its tagged owner/controller is
/// the concrete multiplayer antecedent.
pub fn track_selected_object_player_provenance(spec: &ChooseSpec, ctx: &mut EffectLoweringContext) {
    let Some(filter) = selected_object_filter(spec) else {
        return;
    };
    let reference = if spec.is_target() && !ctx.auto_tag_object_targets {
        ObjectRef::Target
    } else {
        ctx.last_object_tag
            .as_ref()
            .map(|tag| ObjectRef::tagged(tag.clone()))
            .unwrap_or(ObjectRef::Target)
    };
    if filter.owner.is_some() {
        ctx.last_player_filter = Some(PlayerFilter::AliasedOwnerOf(reference));
    } else if filter.controller.is_some() {
        ctx.last_player_filter = Some(PlayerFilter::AliasedControllerOf(reference));
    }
}

pub fn eldrazi_spawn_or_scion_mana_ability() -> Ability {
    Ability {
        kind: AbilityKind::Activated(ActivatedAbility::mana_with_costs(
            TotalCost::free(),
            vec![crate::costs::Cost::sacrifice_self()],
            vec![ManaSymbol::Colorless],
        )),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn eldrazi_spawn_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Eldrazi Spawn")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Spawn])
        .power_toughness(PowerToughness::fixed(0, 1))
        .with_ability(eldrazi_spawn_or_scion_mana_ability())
        .build()
}

pub fn eldrazi_scion_token_definition() -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Eldrazi Scion")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Eldrazi, Subtype::Scion])
        .power_toughness(PowerToughness::fixed(1, 1))
        .with_ability(eldrazi_spawn_or_scion_mana_ability())
        .build()
}

fn generic_mana_cost(amount: u32) -> Option<ManaCost> {
    if amount == 0 {
        Some(ManaCost::new())
    } else {
        Some(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
            u8::try_from(amount).ok()?,
        )]]))
    }
}

/// Build the alternative payment branches for a "waterbend {N}" cost. The
/// player may pay the {N} generic with mana, or tap untapped artifacts/creatures
/// they control (each paying {1}), so the cost expands into N+1 branches: pay
/// all the mana (0 taps), down to fully paying by tapping N permanents.
pub fn waterbend_optional_total_cost(generic: u32) -> TotalCost {
    let tag = crate::tag::CompilerIndexedTag::WaterbendCost.key(generic);
    let mut branches = Vec::new();
    for taps in 0..=generic {
        if taps == 0 {
            branches.push(TotalCost::mana(
                generic_mana_cost(generic).unwrap_or_default(),
            ));
            continue;
        }
        let mana_remaining = generic - taps;
        let mut costs = Vec::new();
        if mana_remaining > 0
            && let Some(mana) = generic_mana_cost(mana_remaining)
        {
            costs.push(crate::costs::Cost::mana(mana));
        }

        let artifact_filter = ObjectFilter {
            card_types: vec![CardType::Artifact],
            ..ObjectFilter::default()
        };
        let creature_filter = ObjectFilter {
            card_types: vec![CardType::Creature],
            ..ObjectFilter::default()
        };
        let mut filter = ObjectFilter::default();
        filter.untapped = true;
        filter.controller = Some(PlayerFilter::You);
        filter.zone = Some(Zone::Battlefield);
        filter.any_of = vec![artifact_filter, creature_filter];

        let choose = crate::effects::ChooseObjectsEffect::new(
            filter,
            ChoiceCount::exactly(taps as usize),
            PlayerFilter::You,
            tag.clone(),
        )
        .in_zone(Zone::Battlefield);
        costs.push(crate::costs::Cost::effect(Effect::new(choose)));
        costs.push(crate::costs::Cost::effect(Effect::new(
            crate::effects::TapEffect::with_spec(ChooseSpec::Tagged(tag.clone().into())),
        )));
        branches.push(TotalCost::from_costs(costs));
    }
    TotalCost::one_of(branches)
}

fn equipment_equip_ability(amount: u32) -> Option<Ability> {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let total_cost = if amount == 0 {
        TotalCost::free()
    } else {
        TotalCost::mana(generic_mana_cost(amount)?)
    };
    Some(Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: total_cost,
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::attach_to(
                target.clone(),
            )]),
            choices: vec![target],
            timing: ActivationTiming::SorcerySpeed,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    })
}

fn equipment_granted_damage_ability(
    shape: &token_grammar::EquipmentDamageGrantShape,
    token_name: &str,
) -> Option<Ability> {
    let mut costs = Vec::new();
    if let Some(amount) = shape.generic_amount
        && amount > 0
    {
        costs.push(crate::costs::Cost::mana(generic_mana_cost(amount)?));
    }
    if shape.tap_cost {
        costs.push(crate::costs::Cost::Tap);
    }
    if shape.sacrifice_equipment {
        costs.push(crate::costs::Cost::sacrifice(
            ObjectFilter::artifact().you_control().named(token_name),
        ));
    }

    let target = ChooseSpec::AnyTarget;
    Some(Ability {
        kind: AbilityKind::Activated(ActivatedAbility {
            mana_cost: TotalCost::from_costs(costs),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::Fixed(shape.damage_amount),
                target.clone(),
            )]),
            choices: vec![target],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: vec![],
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    })
}

fn static_ability_for_token_keyword(
    keyword: token_grammar::TokenKeywordShape,
) -> Option<StaticAbility> {
    Some(match keyword {
        token_grammar::TokenKeywordShape::Firebending(_) => return None,
        token_grammar::TokenKeywordShape::Flying => StaticAbility::flying(),
        token_grammar::TokenKeywordShape::WardGeneric(amount) => {
            StaticAbility::ward(TotalCost::mana(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(amount as u8),
            ])))
        }
        token_grammar::TokenKeywordShape::Defender => StaticAbility::defender(),
        token_grammar::TokenKeywordShape::Prowess => StaticAbility::prowess(),
        token_grammar::TokenKeywordShape::Vigilance => StaticAbility::vigilance(),
        token_grammar::TokenKeywordShape::Trample => StaticAbility::trample(),
        token_grammar::TokenKeywordShape::Lifelink => StaticAbility::lifelink(),
        token_grammar::TokenKeywordShape::Deathtouch => StaticAbility::deathtouch(),
        token_grammar::TokenKeywordShape::Haste => StaticAbility::haste(),
        token_grammar::TokenKeywordShape::Menace => StaticAbility::menace(),
        token_grammar::TokenKeywordShape::Reach => StaticAbility::reach(),
        token_grammar::TokenKeywordShape::FirstStrike => StaticAbility::first_strike(),
        token_grammar::TokenKeywordShape::DoubleStrike => StaticAbility::double_strike(),
        token_grammar::TokenKeywordShape::Hexproof => StaticAbility::hexproof(),
        token_grammar::TokenKeywordShape::Indestructible => StaticAbility::indestructible(),
        token_grammar::TokenKeywordShape::Infect => StaticAbility::infect(),
        token_grammar::TokenKeywordShape::Flash => StaticAbility::flash(),
        token_grammar::TokenKeywordShape::Islandwalk => {
            StaticAbility::landwalk(crate::types::Subtype::Island)
        }
        token_grammar::TokenKeywordShape::Mountainwalk => {
            StaticAbility::landwalk(crate::types::Subtype::Mountain)
        }
        token_grammar::TokenKeywordShape::Forestwalk => {
            StaticAbility::landwalk(crate::types::Subtype::Forest)
        }
        token_grammar::TokenKeywordShape::Swampwalk => {
            StaticAbility::landwalk(crate::types::Subtype::Swamp)
        }
        token_grammar::TokenKeywordShape::Plainswalk => {
            StaticAbility::landwalk(crate::types::Subtype::Plains)
        }
    })
}

fn build_equipment_token_from_rules_shape(
    mut builder: CardDefinitionBuilder,
    rules: &token_grammar::EquipmentRulesShape,
    token_name: &str,
) -> Option<CardDefinition> {
    let mut handled_any = false;
    for line in &rules.lines {
        match line {
            token_grammar::EquipmentRuleLineShape::GrantedDamage {
                display_text,
                grant,
            } => {
                let ability = equipment_granted_damage_ability(grant, token_name)?;
                builder = builder.with_ability(Ability::static_ability(StaticAbility::new(
                    crate::static_abilities::AttachedAbilityGrant::new(
                        ability,
                        display_text.clone(),
                    ),
                )));
                handled_any = true;
            }
            token_grammar::EquipmentRuleLineShape::StaticGrant {
                display_text,
                power_toughness,
                scaled_power_toughness,
                keywords,
            } => {
                let stat_grant = if let Some(scaled) = scaled_power_toughness {
                    let count = match scaled.count {
                        token_grammar::EquipmentGrantCountShape::CountersAmongPermanentsYouControl(
                            counter_type,
                        ) => crate::static_abilities::AnthemCountExpression::CountersAmong(
                            ObjectFilter::permanent().you_control(),
                            counter_type,
                        ),
                    };
                    Some(
                        crate::static_abilities::Anthem::<crate::ConditionExpr>::for_source(0, 0)
                            .with_values(
                                crate::static_abilities::AnthemValue::scaled(
                                    scaled.power,
                                    count.clone(),
                                ),
                                crate::static_abilities::AnthemValue::scaled(
                                    scaled.toughness,
                                    count,
                                ),
                            ),
                    )
                } else {
                    power_toughness.map(|(power, toughness)| {
                        crate::static_abilities::Anthem::for_source(power, toughness)
                    })
                };
                if let Some(grant) = stat_grant {
                    let grant = StaticAbility::new(grant);
                    let display = if scaled_power_toughness.is_some() {
                        display_text.clone()
                    } else {
                        let (power, toughness) = power_toughness.expect("fixed equipment grant");
                        format!("Equipped creature gets {:+}/{:+}.", power, toughness)
                    };
                    builder = builder.with_ability(Ability::static_ability(StaticAbility::new(
                        crate::static_abilities::AttachedAbilityGrant::new(
                            Ability::static_ability(grant),
                            display,
                        ),
                    )));
                }
                for keyword in keywords {
                    let Some(grant) = static_ability_for_token_keyword(*keyword) else {
                        continue;
                    };
                    let display = format!("Equipped creature has {}.", grant.display());
                    builder = builder.with_ability(Ability::static_ability(StaticAbility::new(
                        crate::static_abilities::AttachedAbilityGrant::new(
                            Ability::static_ability(grant),
                            display,
                        ),
                    )));
                }
                handled_any = true;
            }
            token_grammar::EquipmentRuleLineShape::Equip(equip) => {
                builder = builder.with_ability(equipment_equip_ability(equip.amount)?);
                handled_any = true;
            }
        }
    }

    handled_any.then(|| builder.build())
}

fn apply_embedded_token_rules(
    mut builder: CardDefinitionBuilder,
    rules: &token_grammar::TokenRulesSurfaces,
) -> CardDefinitionBuilder {
    for rule in &rules.embedded_rules {
        builder = match rule {
            token_grammar::TokenEmbeddedRuleShape::CantBlockOrBeBlockedByNonSubtypeCreatures {
                subtype,
            } => {
                let source = ObjectFilter::source();
                let non_subtype_creature =
                    ObjectFilter::creature().without_subtype(*subtype);
                let restrictions = vec![
                    crate::effect::Restriction::block_specific_attacker(
                        source.clone(),
                        non_subtype_creature.clone(),
                    ),
                    crate::effect::Restriction::block_specific_attacker(
                        non_subtype_creature,
                        source,
                    ),
                ];
                let display = format!(
                    "This token can't block or be blocked by non-{subtype} creatures."
                );
                builder.with_ability(Ability::static_ability(StaticAbility::restrictions(
                    restrictions,
                    display,
                )))
            }
            token_grammar::TokenEmbeddedRuleShape::OpponentCastsCreatureRemoveCreatureTypeUntilEndOfTurn => {
                let effect = Effect::new(crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Source,
                    crate::continuous::Modification::RemoveCardTypes(vec![CardType::Creature]),
                    Until::EndOfTurn,
                ));
                builder.with_ability(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger: Trigger::spell_cast(
                            Some(ObjectFilter::creature()),
                            PlayerFilter::Opponent,
                        ),
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![effect]),
                        choices: Vec::new(),
                        intervening_if: None,
                        presentation_label: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            token_grammar::TokenEmbeddedRuleShape::PowerToughnessEqualCreaturesYouControl => {
                let count = Value::Count(ObjectFilter::creature().you_control());
                builder.with_ability(Ability::static_ability(
                    StaticAbility::characteristic_defining_pt(count.clone(), count),
                ))
            }
            token_grammar::TokenEmbeddedRuleShape::LandEntersPutCountersOnSelf {
                counter_type,
                count,
            } => builder.with_ability(Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: Trigger::enters_battlefield(
                        ObjectFilter::land().you_control(),
                        None,
                    ),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::put_counters_on_source(*counter_type, *count as i32),
                    ]),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            }),
            token_grammar::TokenEmbeddedRuleShape::DiesCreateBuiltinToken { token, count } => {
                let created = build_builtin_token_definition(*token);
                builder.with_ability(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        // This typed embedded-rule grammar is introduced only
                        // by authored `When this token dies`. Keep that
                        // one-shot intro distinct from the generic
                        // `Whenever a creature dies` surface.
                        trigger: Trigger::this_dies()
                            .with_intro_surface(crate::triggers::TriggerIntroSurface::When),
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::create_tokens(created, Value::Fixed(*count as i32)),
                        ]),
                        choices: Vec::new(),
                        intervening_if: None,
                        presentation_label: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            token_grammar::TokenEmbeddedRuleShape::DealsDamageToPlayerPutCounters {
                combat_only,
                counter_type,
                count,
            } => {
                let trigger = if *combat_only {
                    Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any)
                } else {
                    Trigger::this_deals_damage_to_player(PlayerFilter::Any, None)
                };
                let effect = if matches!(counter_type, crate::object::CounterType::Poison) {
                    Effect::poison_counters_player(*count as i32, PlayerFilter::DamagedPlayer)
                } else {
                    Effect::put_counters(
                        *counter_type,
                        *count as i32,
                        ChooseSpec::Player(PlayerFilter::DamagedPlayer),
                    )
                };
                builder.with_ability(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![effect]),
                        choices: Vec::new(),
                        intervening_if: None,
                        presentation_label: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            token_grammar::TokenEmbeddedRuleShape::DealsDamageToPlayerLoseGame {
                combat_only,
            } => {
                let trigger = if *combat_only {
                    Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any)
                } else {
                    Trigger::this_deals_damage_to_player(PlayerFilter::Any, None)
                };
                builder.with_ability(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::lose_the_game_player(PlayerFilter::DamagedPlayer),
                        ]),
                        choices: Vec::new(),
                        intervening_if: None,
                        presentation_label: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            token_grammar::TokenEmbeddedRuleShape::DealsDamageToPlaneswalkerDestroy {
                combat_only,
            } => {
                let trigger = if *combat_only {
                    Trigger::this_deals_combat_damage_to(ObjectFilter::planeswalker())
                } else {
                    Trigger::this_deals_damage_to(ObjectFilter::planeswalker())
                };
                let damaged_tag = crate::tag::CompilerReferenceTag::Damaged.bind();
                builder.with_ability(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::tag_triggering_damage_target(damaged_tag.clone()),
                            Effect::new(crate::effects::DestroyEffect::with_spec(
                                ChooseSpec::Tagged(damaged_tag.key.clone()),
                            )),
                        ]),
                        choices: Vec::new(),
                        intervening_if: None,
                        presentation_label: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            token_grammar::TokenEmbeddedRuleShape::BeginningOfYourUpkeepSacrificeAnotherCreatureOrSourceDamagesYou {
                damage,
            } => {
                let effect_id = EffectId(1);
                let sacrifice = Effect::sacrifice_player(
                    ObjectFilter::creature().you_control().other(),
                    Value::Fixed(1),
                    PlayerFilter::You,
                );
                builder.with_ability(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::with_id(effect_id.0, sacrifice),
                            Effect::if_then(
                                effect_id,
                                EffectPredicate::DidNotHappen,
                                vec![Effect::deal_damage(
                                    Value::Fixed(*damage),
                                    ChooseSpec::SourceController,
                                )],
                            ),
                        ]),
                        choices: Vec::new(),
                        intervening_if: None,
                        presentation_label: None,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            token_grammar::TokenEmbeddedRuleShape::TapSacrificeAddManaOfAnyColor => {
                let costs = TotalCost::from_costs(vec![
                    crate::costs::Cost::tap(),
                    crate::costs::Cost::sacrifice_self(),
                ]);
                builder.with_ability(Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: costs,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::add_mana_of_any_color(1),
                        ]),
                        choices: Vec::new(),
                        timing: ActivationTiming::AnyTime,
                        additional_restrictions: Vec::new(),
                        activation_restrictions: Vec::new(),
                        mana_output: Some(Vec::new()),
                        activation_condition: None,
                        mana_usage_restrictions: Vec::new(),
                        is_loyalty_ability: false,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
            token_grammar::TokenEmbeddedRuleShape::TapSacrificeAddManaOrGainLife(shape) => {
                let colors = shape
                    .mana_options
                    .iter()
                    .filter_map(|symbol| match symbol {
                        ManaSymbol::White => Some(Color::White),
                        ManaSymbol::Blue => Some(Color::Blue),
                        ManaSymbol::Black => Some(Color::Black),
                        ManaSymbol::Red => Some(Color::Red),
                        ManaSymbol::Green => Some(Color::Green),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let costs = TotalCost::from_costs(vec![
                    crate::costs::Cost::tap(),
                    crate::costs::Cost::sacrifice_self(),
                ]);
                builder.with_ability(Ability {
                    kind: AbilityKind::Activated(ActivatedAbility {
                        mana_cost: costs,
                        effects: crate::resolution::ResolutionProgram::from_effects(vec![
                            Effect::add_mana_of_any_color_restricted(1, colors),
                            Effect::gain_life(shape.life as i32),
                        ]),
                        choices: Vec::new(),
                        timing: ActivationTiming::AnyTime,
                        additional_restrictions: Vec::new(),
                        activation_restrictions: Vec::new(),
                        mana_output: Some(Vec::new()),
                        activation_condition: None,
                        mana_usage_restrictions: Vec::new(),
                        is_loyalty_ability: false,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                })
            }
        };
    }
    builder
}

pub fn token_dies_deals_damage_any_target_ability(amount: i32) -> Ability {
    let target = ChooseSpec::AnyTarget;
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::Fixed(amount),
                target.clone(),
            )]),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_leaves_deals_damage_any_target_ability(amount: i32) -> Ability {
    let target = ChooseSpec::AnyTarget;
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_leaves_battlefield(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::Fixed(amount),
                target.clone(),
            )]),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_becomes_tapped_deals_damage_target_player_ability(amount: i32) -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::becomes_tapped(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::deal_damage(
                Value::Fixed(amount),
                target.clone(),
            )]),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_dies_target_creature_gets_minus_one_minus_one_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::pump(
                -1,
                -1,
                target.clone(),
                Until::EndOfTurn,
            )]),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_red_pump_ability() -> Ability {
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Red]])),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::pump(
                1,
                0,
                ChooseSpec::Source,
                Until::EndOfTurn,
            )]),
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_white_tap_target_creature_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![
                crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::White]])),
                crate::costs::Cost::tap(),
            ]),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::tap(
                target.clone(),
            )]),
            choices: vec![target],
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_tap_mana_ability(shape: token_grammar::TokenTapManaAbilityShape) -> Option<Ability> {
    let mana_usage_restrictions = shape
        .restrictions
        .into_iter()
        .map(|restriction| {
            restriction.try_map_effects(&mut |effect| {
                super::lowering_support::lower_compiler_child_effect(effect)
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some(Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs(vec![crate::costs::Cost::tap()]),
            effects: crate::resolution::ResolutionProgram::default(),
            choices: Vec::new(),
            timing: crate::ability::ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: Some(shape.mana),
            activation_condition: None,
            mana_usage_restrictions,
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    })
}

pub fn token_damage_to_player_poison_counter_ability() -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::poison_counters_player(1, PlayerFilter::DamagedPlayer),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_noncreature_spell_each_opponent_damage_ability(amount: i32) -> Ability {
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::spell_cast(
                Some(ObjectFilter::spell().without_type(CardType::Creature)),
                PlayerFilter::You,
            ),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::for_each_opponent(vec![Effect::deal_damage(
                    Value::Fixed(amount),
                    ChooseSpec::Player(PlayerFilter::IteratedPlayer),
                )]),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_combat_damage_gain_control_target_artifact_ability() -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::artifact().controlled_by(PlayerFilter::DamagedPlayer),
    ));
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_deals_combat_damage_to_player(PlayerFilter::Any),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    target.clone(),
                    crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
                    Until::Forever,
                ),
            )]),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_leaves_return_named_from_graveyard_to_hand_ability(
    card_name: &str,
    self_surface: Option<crate::target::SourceReferenceSurface>,
) -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    ));
    let trigger = match self_surface {
        Some(surface) => Trigger::new(
            crate::triggers::ZoneChangeTrigger::new()
                .from(Zone::Battlefield)
                .this()
                .this_surface(surface),
        ),
        None => Trigger::this_leaves_battlefield(),
    };
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger,
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::return_from_graveyard_to_hand(target.clone()),
            ]),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

fn token_combat_restriction_ability(
    restriction: token_grammar::TokenCombatRestrictionShape,
    self_surface: Option<crate::target::SourceReferenceSurface>,
) -> Ability {
    let ability = match restriction {
        token_grammar::TokenCombatRestrictionShape::CantAttackOrBlockAlone => {
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block_alone(ObjectFilter::source()),
                "this token can't attack or block alone".to_string(),
            )
        }
        token_grammar::TokenCombatRestrictionShape::CantAttackOrBlock => {
            StaticAbility::restriction(
                crate::effect::Restriction::attack_or_block(ObjectFilter::source()),
                "this token can't attack or block".to_string(),
            )
        }
        token_grammar::TokenCombatRestrictionShape::Unblockable => StaticAbility::unblockable(),
        token_grammar::TokenCombatRestrictionShape::CantBlock => StaticAbility::cant_block(),
        token_grammar::TokenCombatRestrictionShape::MustAttack => StaticAbility::must_attack(),
    };
    let ability = match self_surface {
        Some(surface) => ability.with_self_subject_surface(surface),
        None => ability,
    };
    Ability::static_ability(ability)
}

pub fn token_sacrifice_return_named_from_graveyard_ability(
    card_name: &str,
    mana_symbols: Vec<ManaSymbol>,
    tap_cost: bool,
) -> Ability {
    let mut costs = Vec::new();
    if tap_cost {
        costs.push(crate::costs::Cost::tap());
    }
    costs.push(crate::costs::Cost::validated_effect(Effect::new(
        crate::effects::SacrificeTargetEffect::source(),
    )));
    let mana_cost = if mana_symbols.is_empty() {
        ManaCost::new()
    } else {
        ManaCost::from_pips(
            mana_symbols
                .into_iter()
                .map(|symbol| vec![symbol])
                .collect(),
        )
    };
    let target = ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    );
    Ability {
        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
            mana_cost: TotalCost::from_costs({
                let mut total_costs = vec![crate::costs::Cost::mana(mana_cost)];
                total_costs.extend(costs);
                total_costs
            }),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::return_from_graveyard_to_battlefield(target.clone(), false),
            ]),
            choices: Vec::new(),
            timing: ActivationTiming::AnyTime,
            additional_restrictions: Vec::new(),
            activation_restrictions: vec![],
            mana_output: None,
            activation_condition: None,
            mana_usage_restrictions: vec![],
            is_loyalty_ability: false,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_upkeep_sacrifice_return_named_from_graveyard_ability(
    card_name: &str,
    grants_haste: bool,
) -> Ability {
    let target = ChooseSpec::target(ChooseSpec::Object(
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .named(card_name.to_string()),
    ));
    let mut effects = vec![
        Effect::sacrifice_source(),
        Effect::return_from_graveyard_to_battlefield(target.clone(), false),
    ];
    if grants_haste {
        effects.push(Effect::new(
            crate::effects::ApplyContinuousEffect::with_spec(
                target.clone(),
                crate::continuous::Modification::AddAbility(StaticAbility::haste()),
                Until::EndOfTurn,
            ),
        ));
    }
    let mut text = format!(
        "At the beginning of your upkeep, sacrifice this token and return target card named {card_name} from your graveyard to the battlefield."
    );
    if grants_haste {
        text.push_str(" It gains haste until end of turn.");
    }
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::beginning_of_upkeep(PlayerFilter::You),
            effects: effects.into(),
            choices: vec![target],
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

pub fn token_dies_create_dragon_with_firebreathing_ability() -> Ability {
    let dragon = CardDefinitionBuilder::new(CardId::new(), "Dragon")
        .token()
        .card_types(vec![CardType::Creature])
        .subtypes(vec![Subtype::Dragon])
        .color_indicator(ColorSet::RED)
        .power_toughness(PowerToughness::fixed(2, 2))
        .flying()
        .with_ability(token_red_pump_ability())
        .build();
    Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_dies(),
            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::create_tokens(dragon, Value::Fixed(1)),
            ]),
            choices: Vec::new(),
            intervening_if: None,
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    }
}

fn build_builtin_token_definition(shape: token_grammar::BuiltinTokenShape) -> CardDefinition {
    match shape {
        token_grammar::BuiltinTokenShape::Treasure => {
            crate::cards::tokens::treasure_token_definition()
        }
        token_grammar::BuiltinTokenShape::Clue => crate::cards::tokens::clue_token_definition(),
        token_grammar::BuiltinTokenShape::Map => crate::cards::tokens::map_token_definition(),
        token_grammar::BuiltinTokenShape::Lander => crate::cards::tokens::lander_token_definition(),
        token_grammar::BuiltinTokenShape::Junk => crate::cards::tokens::junk_token_definition(),
        token_grammar::BuiltinTokenShape::Mutagen => {
            crate::cards::tokens::mutagen_token_definition()
        }
        token_grammar::BuiltinTokenShape::Gold => crate::cards::tokens::gold_token_definition(),
        token_grammar::BuiltinTokenShape::Shard => crate::cards::tokens::shard_token_definition(),
        token_grammar::BuiltinTokenShape::Walker => crate::cards::tokens::walker_token_definition(),
        token_grammar::BuiltinTokenShape::EldraziSpawn => eldrazi_spawn_token_definition(),
        token_grammar::BuiltinTokenShape::EldraziScion => eldrazi_scion_token_definition(),
        token_grammar::BuiltinTokenShape::Food => crate::cards::tokens::food_token_definition(),
        token_grammar::BuiltinTokenShape::WickedRole => {
            crate::cards::tokens::wicked_role_token_definition()
        }
        token_grammar::BuiltinTokenShape::YoungHeroRole => {
            crate::cards::tokens::young_hero_role_token_definition()
        }
        token_grammar::BuiltinTokenShape::MonsterRole => {
            crate::cards::tokens::monster_role_token_definition()
        }
        token_grammar::BuiltinTokenShape::SorcererRole => {
            crate::cards::tokens::sorcerer_role_token_definition()
        }
        token_grammar::BuiltinTokenShape::RoyalRole => {
            crate::cards::tokens::royal_role_token_definition()
        }
        token_grammar::BuiltinTokenShape::CursedRole => {
            crate::cards::tokens::cursed_role_token_definition()
        }
        token_grammar::BuiltinTokenShape::Blood => crate::cards::tokens::blood_token_definition(),
        token_grammar::BuiltinTokenShape::Powerstone => {
            crate::cards::tokens::powerstone_token_definition()
        }
    }
}

fn build_vehicle_token_definition(
    shape: token_grammar::VehicleTokenShape,
) -> Option<CardDefinition> {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), &shape.name)
        .token()
        .card_types(vec![CardType::Artifact])
        .subtypes(vec![Subtype::Vehicle]);
    if let Some((power, toughness)) = shape.power_toughness {
        builder = builder.power_toughness(PowerToughness::fixed(power, toughness));
    }
    if shape.colorless {
        builder = builder.with_ability(Ability::static_ability(StaticAbility::make_colorless(
            ObjectFilter::source(),
        )));
    }
    if shape.flying {
        builder = builder.flying();
    }
    if let Some(crew_amount) = shape.crew_amount {
        builder = builder.crew(crew_amount, ActivationTiming::AnyTime, Vec::new());
    }
    Some(builder.build())
}

fn build_artifact_token_definition(
    shape: token_grammar::ArtifactTokenShape,
) -> Option<CardDefinition> {
    let mut builder = CardDefinitionBuilder::new(CardId::new(), &shape.name)
        .token()
        .card_types(vec![CardType::Artifact]);
    if shape.legendary {
        builder = builder.supertypes(vec![crate::types::Supertype::Legendary]);
    }
    if !shape.subtypes.is_empty() {
        builder = builder.subtypes(shape.subtypes);
    }
    if !shape.colors.is_empty() {
        builder = builder.color_indicator(shape.colors);
    } else if shape.colorless {
        builder = builder.with_ability(Ability::static_ability(StaticAbility::make_colorless(
            ObjectFilter::source(),
        )));
    }
    if let Some(rules) = shape.equipment_rules.as_ref()
        && let Some(def) =
            build_equipment_token_from_rules_shape(builder.clone(), rules, &shape.name)
    {
        return Some(def);
    }
    builder = apply_embedded_token_rules(builder, &shape.token_rules);
    if let Some(amount) = shape.leaves_damage_any_target {
        builder = builder.with_ability(token_leaves_deals_damage_any_target_ability(amount));
    }
    Some(builder.build())
}

pub fn apply_standard_token_keyword(
    builder: CardDefinitionBuilder,
    keyword: token_grammar::TokenKeywordShape,
) -> CardDefinitionBuilder {
    match keyword {
        token_grammar::TokenKeywordShape::Flying => builder.flying(),
        token_grammar::TokenKeywordShape::WardGeneric(amount) => builder.ward_generic(amount),
        token_grammar::TokenKeywordShape::Firebending(amount) => builder.firebending(amount),
        token_grammar::TokenKeywordShape::Defender => builder.defender(),
        token_grammar::TokenKeywordShape::Prowess => builder.prowess(),
        token_grammar::TokenKeywordShape::Vigilance => builder.vigilance(),
        token_grammar::TokenKeywordShape::Trample => builder.trample(),
        token_grammar::TokenKeywordShape::Lifelink => builder.lifelink(),
        token_grammar::TokenKeywordShape::Deathtouch => builder.deathtouch(),
        token_grammar::TokenKeywordShape::Haste => builder.haste(),
        token_grammar::TokenKeywordShape::Menace => builder.menace(),
        token_grammar::TokenKeywordShape::Reach => builder.reach(),
        token_grammar::TokenKeywordShape::FirstStrike => builder.first_strike(),
        token_grammar::TokenKeywordShape::DoubleStrike => builder.double_strike(),
        token_grammar::TokenKeywordShape::Hexproof => builder.hexproof(),
        token_grammar::TokenKeywordShape::Indestructible => builder.indestructible(),
        other => match static_ability_for_token_keyword(other) {
            Some(ability) => builder.with_ability(crate::ability::Ability::static_ability(ability)),
            None => builder,
        },
    }
}

fn build_creature_token_definition(
    shape: token_grammar::CreatureTokenShape,
) -> Option<CardDefinition> {
    let (power, toughness) = shape.power_toughness;
    let mut builder = CardDefinitionBuilder::new(CardId::new(), &shape.name)
        .token()
        .card_types(shape.card_types)
        .power_toughness(PowerToughness::fixed(power, toughness));
    if shape.legendary {
        builder = builder.supertypes(vec![crate::types::Supertype::Legendary]);
    }
    if !shape.subtypes.is_empty() {
        builder = builder.subtypes(shape.subtypes);
    }
    if !shape.colors.is_empty() {
        builder = builder.color_indicator(shape.colors);
    }
    for keyword in shape.keywords {
        builder = apply_standard_token_keyword(builder, keyword);
    }

    let rules = shape.rules;
    builder = apply_embedded_token_rules(builder, &rules.token_rules);
    if let Some(symbols) = rules.cumulative_upkeep_mana_symbols.as_ref() {
        let total_cost = if symbols.is_empty() {
            TotalCost::free()
        } else {
            TotalCost::mana(ManaCost::from_symbols(symbols.clone()))
        };
        builder = builder.cumulative_upkeep(total_cost);
    }
    if let Some(shape) = rules.tap_mana_ability {
        builder = builder.with_ability(token_tap_mana_ability(shape)?);
    }
    if let Some(amount) = rules.saddle_crew_power_bonus {
        builder = builder.with_ability(Ability::static_ability(StaticAbility::keyword_marker(
            format!(
                "This creature saddles Mounts and crews Vehicles as though its power were {amount} greater."
            ),
        )));
    }
    if rules.banding {
        builder = builder.with_ability(Ability::static_ability(StaticAbility::banding()));
    }
    if rules.hexproof {
        builder = builder.hexproof();
    }
    if rules.indestructible {
        builder = builder.indestructible();
    }
    if rules.copies_exiled_triggered_abilities {
        let filter = ObjectFilter::default().in_zone(Zone::Exile);
        builder = builder.with_ability(Ability::static_ability(
            StaticAbility::copy_triggered_abilities(
                CopyTriggeredAbilities::new(filter)
                    .with_display("all triggered abilities of the exiled cards"),
            ),
        ));
    }
    if let Some(amount) = rules.toxic_amount {
        builder = builder.toxic(amount);
    }
    if let Some(return_shape) = rules.sacrifice_return {
        builder = builder.with_ability(token_sacrifice_return_named_from_graveyard_ability(
            &return_shape.card_name,
            return_shape.mana_symbols,
            return_shape.tap_cost,
        ));
    }
    if let Some(card_name) = rules.upkeep_return_name.as_deref() {
        builder = builder.with_ability(token_upkeep_sacrifice_return_named_from_graveyard_ability(
            card_name,
            rules.upkeep_return_grants_haste,
        ));
    }
    if rules.dies_create_firebreathing_dragon {
        builder = builder.with_ability(token_dies_create_dragon_with_firebreathing_ability());
    }
    if let Some(amount) = rules.dies_damage_any_target {
        builder = builder.with_ability(token_dies_deals_damage_any_target_ability(amount));
    }
    if rules.dies_minus_one_target_creature {
        builder =
            builder.with_ability(token_dies_target_creature_gets_minus_one_minus_one_ability());
    }
    if let Some(amount) = rules.leaves_damage_you_and_creatures {
        let ability = Ability {
            kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: Trigger::this_leaves_battlefield(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::deal_damage(amount, ChooseSpec::SourceController),
                    Effect::for_each(
                        ObjectFilter::creature().you_control(),
                        vec![Effect::deal_damage(amount, ChooseSpec::Iterated)],
                    ),
                ]),
                choices: Vec::new(),
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        builder = builder.with_ability(ability);
    }
    if rules.bands_with_wolves {
        builder = builder.with_ability(Ability::static_ability(StaticAbility::bands_with_other(
            ObjectFilter::creature().named("Wolves of the Hunt"),
            "bands with other creatures named Wolves of the Hunt",
        )));
    }
    if rules.red_pump {
        builder = builder.with_ability(token_red_pump_ability());
    }
    if rules.white_tap_target_creature {
        builder = builder.with_ability(token_white_tap_target_creature_ability());
    }
    if rules.combat_damage_poison {
        builder = builder.with_ability(token_damage_to_player_poison_counter_ability());
    }
    if let Some(amount) = rules.noncreature_spell_each_opponent_damage {
        builder =
            builder.with_ability(token_noncreature_spell_each_opponent_damage_ability(amount));
    }
    if let Some(amount) = rules.becomes_tapped_damage_player {
        builder = builder.with_ability(token_becomes_tapped_deals_damage_target_player_ability(
            amount,
        ));
    }
    if rules.combat_damage_gain_artifact {
        builder = builder.with_ability(token_combat_damage_gain_control_target_artifact_ability());
    }
    for presentation in &rules.authored_inline_rules {
        builder = match presentation.kind {
            token_grammar::CreatureTokenInlineRuleKind::CombatRestriction => {
                match rules.combat_restriction {
                    Some(restriction) => builder.with_ability(token_combat_restriction_ability(
                        restriction,
                        presentation.self_surface.clone(),
                    )),
                    None => builder,
                }
            }
            token_grammar::CreatureTokenInlineRuleKind::LeavesReturnNamedToHand => {
                match rules.leaves_return_named_to_hand.as_deref() {
                    Some(card_name) => builder.with_ability(
                        token_leaves_return_named_from_graveyard_to_hand_ability(
                            card_name,
                            presentation.self_surface.clone(),
                        ),
                    ),
                    None => builder,
                }
            }
        };
    }
    let has_authored_leaves_rule = rules.authored_inline_rules.iter().any(|presentation| {
        presentation.kind == token_grammar::CreatureTokenInlineRuleKind::LeavesReturnNamedToHand
    });
    if let Some(card_name) = rules
        .leaves_return_named_to_hand
        .as_deref()
        .filter(|_| !has_authored_leaves_rule)
    {
        builder = builder.with_ability(token_leaves_return_named_from_graveyard_to_hand_ability(
            card_name, None,
        ));
    }
    if rules.pest_dies_gain_life {
        let ability = Ability {
            kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                trigger: Trigger::this_dies(),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::gain_life(1),
                ]),
                choices: Vec::new(),
                intervening_if: None,
                presentation_label: None,
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        builder = builder.with_ability(ability);
    }
    if rules.first_strike {
        builder = builder.first_strike();
    }
    if rules.double_strike {
        builder = builder.double_strike();
    }
    if rules.mercenary_pump {
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
        let ability =
            Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: TotalCost::from_cost(crate::costs::Cost::tap()),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::pump(1, 0, target.clone(), Until::EndOfTurn),
                    ]),
                    choices: vec![target],
                    timing: crate::ability::ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec!["activate only as a sorcery".to_string()],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: false,
                }),
                functional_zones: vec![Zone::Battlefield],
            };
        builder = builder.with_ability(ability);
    }
    let has_authored_combat_rule = rules.authored_inline_rules.iter().any(|presentation| {
        presentation.kind == token_grammar::CreatureTokenInlineRuleKind::CombatRestriction
    });
    if let Some(restriction) = rules
        .combat_restriction
        .filter(|_| !has_authored_combat_rule)
    {
        builder = builder.with_ability(token_combat_restriction_ability(restriction, None));
    }
    if rules.can_block_only_flying {
        builder = builder.with_ability(Ability::static_ability(
            StaticAbility::can_block_only_flying(),
        ));
    }
    if rules.counter_noncreature_unless_pays {
        let target = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::spell().without_type(CardType::Creature),
        ));
        let counter_ability = Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: TotalCost::from_costs(vec![
                    crate::costs::Cost::mana(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(
                        1,
                    )]])),
                    crate::costs::Cost::sacrifice_self(),
                ]),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::counter_unless_pays(target.clone(), vec![ManaSymbol::Generic(1)]),
                ]),
                choices: vec![target],
                timing: crate::ability::ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        };
        builder = builder.with_ability(counter_ability);
    }
    if rules.changeling {
        builder = builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
    }
    if let Some(card_name) = rules.graveyard_anthem_card_name {
        let mut named_filter = ObjectFilter::default();
        named_filter.zone = Some(Zone::Graveyard);
        named_filter.name = Some(card_name);
        let count = crate::static_abilities::AnthemCountExpression::MatchingFilter(named_filter);
        let anthem = crate::static_abilities::Anthem::<crate::ConditionExpr>::for_source(0, 0)
            .with_values(
                crate::static_abilities::AnthemValue::scaled(1, count.clone()),
                crate::static_abilities::AnthemValue::scaled(1, count),
            );
        builder = builder.with_ability(Ability::static_ability(StaticAbility::new(anthem)));
    }
    if rules.landfall_pump {
        let ability =
            Ability {
                kind: AbilityKind::Triggered(crate::ability::TriggeredAbility {
                    trigger: Trigger::enters_battlefield(ObjectFilter::land().you_control(), None),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::pump(1, 0, ChooseSpec::Source, Until::EndOfTurn),
                    ]),
                    choices: Vec::new(),
                    intervening_if: None,
                    presentation_label: None,
                }),
                functional_zones: vec![Zone::Battlefield],
            };
        builder = builder.with_ability(ability);
    }
    Some(builder.build())
}

pub fn lower_token_definition_shape(shape: TokenDefinitionSpec) -> Option<CardDefinition> {
    match shape {
        TokenDefinitionSpec::PriorCreated => None,
        TokenDefinitionSpec::Builtin(builtin) => Some(build_builtin_token_definition(builtin)),
        TokenDefinitionSpec::Vehicle(vehicle) => build_vehicle_token_definition(vehicle),
        TokenDefinitionSpec::Artifact(artifact) => build_artifact_token_definition(artifact),
        TokenDefinitionSpec::Angel => Some(
            CardDefinitionBuilder::new(CardId::new(), "Angel")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Angel])
                .color_indicator(ColorSet::WHITE)
                .power_toughness(PowerToughness::fixed(4, 4))
                .flying()
                .build(),
        ),
        TokenDefinitionSpec::Wall => Some(
            CardDefinitionBuilder::new(CardId::new(), "Wall")
                .token()
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .subtypes(vec![Subtype::Wall])
                .power_toughness(PowerToughness::fixed(0, 4))
                .defender()
                .build(),
        ),
        TokenDefinitionSpec::Squirrel => Some(
            CardDefinitionBuilder::new(CardId::new(), "Squirrel")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Squirrel])
                .color_indicator(ColorSet::GREEN)
                .power_toughness(PowerToughness::fixed(1, 1))
                .build(),
        ),
        TokenDefinitionSpec::DragonEgg => Some(
            CardDefinitionBuilder::new(CardId::new(), "Dragon Egg")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Dragon, Subtype::Egg])
                .color_indicator(ColorSet::RED)
                .power_toughness(PowerToughness::fixed(0, 2))
                .defender()
                .with_ability(token_dies_create_dragon_with_firebreathing_ability())
                .build(),
        ),
        TokenDefinitionSpec::Elephant => Some(
            CardDefinitionBuilder::new(CardId::new(), "Elephant")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Elephant])
                .color_indicator(ColorSet::GREEN)
                .power_toughness(PowerToughness::fixed(3, 3))
                .build(),
        ),
        TokenDefinitionSpec::Construct(construct) => {
            let mut builder = CardDefinitionBuilder::new(CardId::new(), "Construct")
                .token()
                .card_types(vec![CardType::Artifact, CardType::Creature])
                .subtypes(vec![Subtype::Construct])
                .power_toughness(PowerToughness::fixed(
                    construct.power_toughness.0,
                    construct.power_toughness.1,
                ));
            match construct.artifact_scaling {
                Some(ConstructArtifactScalingShape::CharacteristicDefining) => {
                    let count = Value::Count(ObjectFilter::artifact().you_control());
                    builder = builder.with_ability(Ability::static_ability(
                        StaticAbility::characteristic_defining_pt(count.clone(), count),
                    ));
                }
                Some(ConstructArtifactScalingShape::GetsPlusOnePerArtifact) => {
                    let count = crate::static_abilities::AnthemCountExpression::MatchingFilter(
                        ObjectFilter::artifact().you_control(),
                    );
                    let anthem =
                        crate::static_abilities::Anthem::<crate::ConditionExpr>::for_source(0, 0)
                            .with_values(
                                crate::static_abilities::AnthemValue::scaled(1, count.clone()),
                                crate::static_abilities::AnthemValue::scaled(1, count),
                            );
                    builder =
                        builder.with_ability(Ability::static_ability(StaticAbility::new(anthem)));
                }
                None => {}
            }
            Some(builder.build())
        }
        TokenDefinitionSpec::Shapeshifter(shapeshifter) => {
            let mut builder = CardDefinitionBuilder::new(CardId::new(), "Shapeshifter")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Shapeshifter])
                .power_toughness(PowerToughness::fixed(3, 2));
            if shapeshifter.changeling {
                builder =
                    builder.with_ability(Ability::static_ability(StaticAbility::changeling()));
            }
            Some(builder.build())
        }
        TokenDefinitionSpec::AstartesWarrior(astartes) => {
            let mut builder = CardDefinitionBuilder::new(CardId::new(), "Astartes Warrior")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Astartes, Subtype::Warrior])
                .color_indicator(ColorSet::WHITE)
                .power_toughness(PowerToughness::fixed(2, 2));
            if astartes.vigilance {
                builder = builder.vigilance();
            }
            Some(builder.build())
        }
        TokenDefinitionSpec::Creature(creature) => build_creature_token_definition(creature),
    }
}

pub fn target_mentions_graveyard(target: &TargetAst) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => filter.zone == Some(Zone::Graveyard),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_mentions_graveyard(inner)
        }
        _ => false,
    }
}

pub fn compile_effect_for_target<Builder>(
    target: &TargetAst,
    ctx: &mut EffectLoweringContext,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(ChooseSpec) -> Effect,
{
    let refs = current_reference_env(ctx);
    let (spec, choices) = resolve_target_spec_with_choices(target, &refs)?;
    let effect = tag_object_target_effect(build(spec.clone()), &spec, ctx, "targeted");
    Ok((vec![effect], choices))
}

pub fn compile_tagged_effect_for_target<Builder>(
    target: &TargetAst,
    ctx: &mut EffectLoweringContext,
    tag_prefix: &str,
    build: Builder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    Builder: FnOnce(ChooseSpec) -> Effect,
{
    let refs = current_reference_env(ctx);
    let (spec, choices) = resolve_target_spec_with_choices(target, &refs)?;
    let effect = tag_object_target_effect(build(spec.clone()), &spec, ctx, tag_prefix);
    Ok((vec![effect], choices))
}

pub fn push_choice(choices: &mut Vec<ChooseSpec>, choice: ChooseSpec) {
    if !choices.iter().any(|existing| existing == &choice) {
        choices.push(choice);
    }
}

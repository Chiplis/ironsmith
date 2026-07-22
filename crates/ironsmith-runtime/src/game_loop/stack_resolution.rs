use super::*;
use crate::effects::rebase_target_scope;
use crate::game_loop::targeting::DeclaredTarget;
use crate::triggers::Trigger;

pub(super) fn active_target_assignments_for_effect(
    effect: &Effect,
    chosen_modes: Option<&[usize]>,
    consumed_modal_selection: &mut bool,
    declared_targets: &mut Vec<DeclaredTarget>,
    assignments: &[crate::game_state::TargetAssignment],
    cursor: &mut usize,
) -> Vec<crate::game_state::TargetAssignment> {
    let count = count_target_selection_slots_for_effect(
        effect,
        chosen_modes,
        consumed_modal_selection,
        declared_targets,
    );
    if count == 1
        && let Some(profile) = effect.target_selection_profile()
    {
        if let Some(next) = assignments.get(*cursor)
            && (next.spec == *profile.spec || next.spec.base() == profile.spec.base())
        {
            *cursor += 1;
            return vec![next.clone()];
        }
        return Vec::new();
    }
    let start = *cursor;
    let end = start.saturating_add(count).min(assignments.len());
    *cursor = end;
    let selected = assignments[start..end].to_vec();
    if !selected.is_empty() {
        return selected;
    }

    if let Some(profile) = effect.target_selection_profile()
        && let Some(next) = assignments.get(*cursor)
        && (next.spec == *profile.spec || next.spec.base() == profile.spec.base())
    {
        *cursor += 1;
        return vec![next.clone()];
    }

    selected
}

fn player_filter_references_target_player(filter: &crate::target::PlayerFilter) -> bool {
    use crate::target::PlayerFilter;

    match filter {
        PlayerFilter::Target(_) | PlayerFilter::TargetPlayerOrControllerOfTarget => true,
        PlayerFilter::Excluding { base, excluded } => {
            player_filter_references_target_player(base)
                || player_filter_references_target_player(excluded)
        }
        PlayerFilter::CardsInHandAtLeastMoreThanYou { base, .. }
        | PlayerFilter::HasMoreLifeThanYou { base }
        | PlayerFilter::MaxSpeed { base, .. } => player_filter_references_target_player(base),
        _ => false,
    }
}

fn object_filter_references_target_player(filter: &crate::target::ObjectFilter) -> bool {
    [
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
    ]
    .into_iter()
    .flatten()
    .any(player_filter_references_target_player)
        || filter
            .targets_object
            .as_deref()
            .is_some_and(object_filter_references_target_player)
        || filter
            .targets_only_object
            .as_deref()
            .is_some_and(object_filter_references_target_player)
        || filter
            .attached_to_object
            .as_deref()
            .is_some_and(object_filter_references_target_player)
        || filter
            .any_of
            .iter()
            .any(object_filter_references_target_player)
}

fn pop_sneak_attack_target(game: &mut GameState, source: ObjectId) -> Option<AttackTarget> {
    game.pop_sneak_attack_target(source)
}

fn attack_target_still_valid(game: &GameState, target: &AttackTarget) -> bool {
    match target {
        AttackTarget::Player(player) => game.player(*player).is_some(),
        AttackTarget::Planeswalker(planeswalker) => game.object(*planeswalker).is_some_and(|obj| {
            obj.zone == Zone::Battlefield && obj.has_card_type(CardType::Planeswalker)
        }),
        AttackTarget::Battle(battle) => game.object(*battle).is_some_and(|obj| {
            obj.zone == Zone::Battlefield
                && obj.has_card_type(CardType::Battle)
                && game.battle_protector(*battle).is_some_and(|protector| {
                    game.player(protector)
                        .is_some_and(|player| player.is_in_game())
                })
        }),
    }
}

fn stack_entry_cast_with_named_alternative(
    game: &GameState,
    entry: &StackEntry,
    obj: &crate::object::Object,
    name: &str,
) -> bool {
    match &entry.casting_method {
        CastingMethod::Alternative(idx) => obj
            .alternative_casts
            .get(*idx)
            .is_some_and(|method| method.name().eq_ignore_ascii_case(name)),
        CastingMethod::PlayFrom {
            use_alternative: Some(idx),
            zone,
            ..
        }
        | CastingMethod::SplitOtherHalfPlayFrom {
            use_alternative: idx,
            zone,
            ..
        } => crate::decision::resolve_play_from_alternative_method(
            game,
            entry.controller,
            obj,
            *zone,
            *idx,
        )
        .or_else(|| obj.cast_alternative_method_owned())
        .is_some_and(|method| method.name().eq_ignore_ascii_case(name)),
        _ => false,
    }
}

fn choose_spec_references_target_player(spec: &crate::target::ChooseSpec) -> bool {
    use crate::target::ChooseSpec;

    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => choose_spec_references_target_player(spec),
        ChooseSpec::Player(filter)
        | ChooseSpec::EachPlayer(filter)
        | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            player_filter_references_target_player(filter)
        }
        ChooseSpec::ObjectOrPlayer(object_filter, player_filter) => {
            object_filter_references_target_player(object_filter)
                || player_filter_references_target_player(player_filter)
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            object_filter_references_target_player(filter)
        }
        ChooseSpec::SpecificObject(_)
        | ChooseSpec::SpecificPlayer(_)
        | ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::AttackedPlayerOrPlaneswalker
        | ChooseSpec::Source
        | ChooseSpec::SourceController
        | ChooseSpec::SourceOwner
        | ChooseSpec::Tagged(_)
        | ChooseSpec::Iterated => false,
    }
}

fn runtime_modification_references_target_player(
    modification: &crate::effects::continuous::RuntimeModification,
) -> bool {
    matches!(
        modification,
        crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(player)
            if player_filter_references_target_player(player)
    )
}

fn effect_references_prior_target_player(effect: &Effect) -> bool {
    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>()
        && (apply
            .runtime_modifications
            .iter()
            .any(runtime_modification_references_target_player)
            || apply
                .target_spec
                .as_ref()
                .is_some_and(choose_spec_references_target_player))
    {
        return true;
    }

    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        if !found && effect_references_prior_target_player(child) {
            found = true;
        }
    });
    found
}

fn previous_player_target_assignments(
    targets: &[crate::effects::ResolvedTarget],
    assignments: &[crate::game_state::TargetAssignment],
    before: usize,
) -> Vec<crate::game_state::TargetAssignment> {
    assignments
        .iter()
        .take(before)
        .filter(|assignment| {
            targets[assignment.range.clone()]
                .iter()
                .any(|target| matches!(target, crate::effects::ResolvedTarget::Player(_)))
        })
        .cloned()
        .collect()
}

fn previous_object_target_assignments(
    targets: &[crate::effects::ResolvedTarget],
    assignments: &[crate::game_state::TargetAssignment],
    before: usize,
) -> Vec<crate::game_state::TargetAssignment> {
    assignments
        .iter()
        .take(before)
        .filter(|assignment| {
            targets[assignment.range.clone()]
                .iter()
                .any(|target| matches!(target, crate::effects::ResolvedTarget::Object(_)))
        })
        .cloned()
        .collect()
}

fn effect_references_prior_object_targets(effect: &Effect) -> bool {
    effect
        .downcast_ref::<crate::effects::FightEffect>()
        .is_some()
        || effect
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .is_some_and(|schedule| schedule.target_tag.is_some())
}

fn representative_segment_targets(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effect: &Effect,
    effect_target_assignments: Vec<crate::game_state::TargetAssignment>,
) -> Result<Option<Vec<crate::effects::ResolvedTarget>>, GameLoopError> {
    ctx.with_temp_target_assignments(effect_target_assignments, |ctx| {
        let Some(profile) = effect.target_selection_profile() else {
            return Ok(None);
        };
        let object_id = match crate::effects::helpers::resolve_single_object_for_effect(
            game,
            ctx,
            profile.spec,
        ) {
            Ok(id) => id,
            Err(crate::effects::ExecutionError::InvalidTarget) => return Ok(None),
            Err(err) => return Err(GameLoopError::ResolutionFailed(err.to_string())),
        };
        Ok(Some(vec![crate::effects::ResolvedTarget::Object(
            object_id,
        )]))
    })
}

fn apply_self_replacement_tag_prelude(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<(), GameLoopError> {
    for effect in effects {
        let is_prelude_effect = effect.is_resolution_prelude();
        if !is_prelude_effect {
            break;
        }
        crate::effects::execute_effect(game, effect, ctx)
            .map_err(|err| GameLoopError::ResolutionFailed(err.to_string()))?;
    }
    Ok(())
}

fn apply_self_replacement_declared_target_tags(
    game: &GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
    chosen_modes: Option<&[usize]>,
    valid_target_assignments: &[crate::game_state::TargetAssignment],
    assignment_cursor: usize,
) {
    let mut temp_cursor = assignment_cursor;
    let mut temp_consumed_modal_selection = false;
    let mut temp_declared_targets = Vec::new();

    for effect in effects {
        let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() else {
            let _ = active_target_assignments_for_effect(
                effect,
                chosen_modes,
                &mut temp_consumed_modal_selection,
                &mut temp_declared_targets,
                valid_target_assignments,
                &mut temp_cursor,
            );
            continue;
        };

        let assignments = active_target_assignments_for_effect(
            effect,
            chosen_modes,
            &mut temp_consumed_modal_selection,
            &mut temp_declared_targets,
            valid_target_assignments,
            &mut temp_cursor,
        );
        let mut snapshots = assignments
            .iter()
            .flat_map(|assignment| ctx.targets[assignment.range.clone()].iter())
            .filter_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => game.object(*id).map(|object| {
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        object, game,
                    )
                }),
                crate::effects::ResolvedTarget::Player(_) => None,
            })
            .collect::<Vec<_>>();
        if snapshots.is_empty()
            && effect.target_selection_profile().is_some()
            && let Some(snapshot) = ctx.targets.iter().find_map(|target| match target {
                crate::effects::ResolvedTarget::Object(id) => game.object(*id).map(|object| {
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        object, game,
                    )
                }),
                crate::effects::ResolvedTarget::Player(_) => None,
            })
        {
            snapshots.push(snapshot);
        }
        if snapshots.is_empty() {
            continue;
        }
        ctx.tag_objects(tagged.tag.clone(), snapshots.clone());
        if tagged.tag.as_str() != "__it__" && tagged.tag.as_str() != "__copied_stack_object__" {
            ctx.tag_objects(crate::tag::TagKey::from("__it__"), snapshots);
        }
    }
}

fn collect_tagged_constraints_from_spec(
    spec: &crate::target::ChooseSpec,
    out: &mut Vec<crate::tag::TagKey>,
) {
    match spec {
        crate::target::ChooseSpec::SurfaceHinted { spec, .. }
        | crate::target::ChooseSpec::Target(spec)
        | crate::target::ChooseSpec::WithCount(spec, _)
        | crate::target::ChooseSpec::WithCountValue(spec, _, _) => {
            collect_tagged_constraints_from_spec(spec, out);
        }
        _ => {
            let filter = match spec.base() {
                crate::target::ChooseSpec::Object(filter)
                | crate::target::ChooseSpec::ObjectOrPlayer(filter, _) => Some(filter),
                _ => None,
            };
            if let Some(filter) = filter {
                for constraint in &filter.tagged_constraints {
                    if !out.contains(&constraint.tag) {
                        out.push(constraint.tag.clone());
                    }
                }
            }
        }
    }
}

fn apply_self_replacement_referenced_target_tags(
    game: &GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) {
    let Some(snapshot) = ctx.targets.iter().find_map(|target| match target {
        crate::effects::ResolvedTarget::Object(id) => game.object(*id).map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        }),
        crate::effects::ResolvedTarget::Player(_) => None,
    }) else {
        return;
    };

    let mut tags = Vec::new();
    for effect in effects {
        if let Some(profile) = effect.target_selection_profile() {
            collect_tagged_constraints_from_spec(profile.spec, &mut tags);
        }
    }

    for tag in tags {
        if !ctx.tagged_objects.contains_key(&tag) {
            ctx.tag_object(tag, snapshot.clone());
        }
    }
}

fn evaluate_self_replacement_branch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    branch: &crate::resolution::SelfReplacementBranch,
    segment_effects: &[Effect],
    representative_effect: Option<&Effect>,
    representative_assignments: Vec<crate::game_state::TargetAssignment>,
) -> Result<bool, GameLoopError> {
    let original_tagged_objects = ctx.tagged_objects.clone();
    apply_self_replacement_tag_prelude(game, ctx, segment_effects)?;
    let Some(effect) = representative_effect else {
        let result =
            crate::condition_eval::evaluate_condition_resolution(game, &branch.condition, ctx)
                .map_err(|err| GameLoopError::ResolutionFailed(err.to_string()));
        ctx.tagged_objects = original_tagged_objects;
        return result;
    };

    let representative_targets =
        representative_segment_targets(game, ctx, effect, representative_assignments.clone())?;
    let result = ctx
        .with_temp_target_assignments(representative_assignments, |ctx| {
            if let Some(targets) = representative_targets {
                ctx.with_temp_targets(targets, |ctx| {
                    crate::condition_eval::evaluate_condition_resolution(
                        game,
                        &branch.condition,
                        ctx,
                    )
                })
            } else {
                crate::condition_eval::evaluate_condition_resolution(game, &branch.condition, ctx)
            }
        })
        .map_err(|err| GameLoopError::ResolutionFailed(err.to_string()));
    ctx.tagged_objects = original_tagged_objects;
    result
}

fn bind_singular_active_player_choice(
    game: &GameState,
    ctx: &mut ExecutionContext,
    references_active_player: bool,
) -> bool {
    if !references_active_player
        || !game.shared_team_turns_enabled()
        || game.active_players().len() <= 1
        || game
            .singular_active_player(ctx.combat.chosen_player)
            .is_some_and(|player| ctx.combat.chosen_player == Some(player))
    {
        return true;
    }

    let options = game
        .active_players()
        .into_iter()
        .filter_map(|player| {
            game.player(player)
                .map(|candidate| (candidate.name.to_string(), player))
        })
        .collect::<Vec<_>>();
    let Some(chosen) = crate::decisions::ask_choose_one(
        game,
        &mut ctx.decision_maker,
        ctx.controller,
        ctx.source,
        &options,
    ) else {
        return false;
    };
    if ctx.decision_maker.awaiting_choice() {
        return false;
    }
    ctx.combat.chosen_player = Some(chosen);
    true
}

fn bind_singular_combat_player_choice(
    game: &GameState,
    ctx: &mut ExecutionContext,
    references_player: bool,
    anchor: Option<PlayerId>,
    attacking: bool,
) -> bool {
    if !references_player || !game.shared_team_turns_enabled() {
        return true;
    }
    let Some(anchor) = anchor else {
        return true;
    };
    let options = game
        .team_players_for(anchor)
        .into_iter()
        .filter_map(|player| {
            game.player(player)
                .map(|candidate| (candidate.name.to_string(), player))
        })
        .collect::<Vec<_>>();
    let chosen = match options.as_slice() {
        [] => return true,
        [(_, player)] => *player,
        _ => {
            let Some(chosen) = crate::decisions::ask_choose_one(
                game,
                &mut ctx.decision_maker,
                ctx.controller,
                ctx.source,
                &options,
            ) else {
                return false;
            };
            if ctx.decision_maker.awaiting_choice() {
                return false;
            }
            chosen
        }
    };
    if attacking {
        ctx.combat.attacking_player = Some(chosen);
    } else {
        ctx.combat.defending_player = Some(chosen);
    }
    true
}

pub(crate) fn execute_resolution_program(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    _controller: PlayerId,
    _source_id: ObjectId,
    program: &crate::resolution::ResolutionProgram,
    chosen_modes: Option<&[usize]>,
    valid_target_assignments: &[crate::game_state::TargetAssignment],
) -> Result<Vec<crate::triggers::TriggerEvent>, GameLoopError> {
    // CR 805.9: a singular "active player" in an ability is selected by that
    // ability's controller when its effect is applied. Bind the selection once
    // for this resolution so player filters, object filters, values, and nested
    // effects all observe the same active teammate.
    let program_debug = format!("{program:?}");
    let attacking_anchor = ctx.combat.attacking_player;
    let defending_anchor = ctx.combat.defending_player;
    if !bind_singular_active_player_choice(game, ctx, program_debug.contains("Active"))
        || !bind_singular_combat_player_choice(
            game,
            ctx,
            program_debug.contains("Attacking"),
            attacking_anchor,
            true,
        )
        || !bind_singular_combat_player_choice(
            game,
            ctx,
            program_debug.contains("Defending"),
            defending_anchor,
            false,
        )
    {
        return Ok(Vec::new());
    }

    let initial_subgame_depth = game.subgame_depth();
    let mut all_events = Vec::new();
    let mut consumed_modal_selection = false;
    let mut assignment_cursor = 0usize;
    let mut declared_targets = Vec::new();
    for segment in &program.segments {
        let (selected_effects, selected_self_replacement) = if segment.self_replacements.is_empty()
        {
            (segment.default_effects.clone(), false)
        } else {
            let representative_effect = segment
                .default_effects
                .iter()
                .find(|effect| effect.target_selection_profile().is_some())
                .or_else(|| {
                    segment
                        .self_replacements
                        .iter()
                        .flat_map(|branch| branch.replacement_effects.iter())
                        .find(|effect| effect.target_selection_profile().is_some())
                });
            let representative_assignments = representative_effect
                .map(|effect| {
                    let mut temp_cursor = assignment_cursor;
                    let mut temp_consumed_modal_selection = consumed_modal_selection;
                    let mut temp_declared_targets = declared_targets.clone();
                    active_target_assignments_for_effect(
                        effect,
                        chosen_modes,
                        &mut temp_consumed_modal_selection,
                        &mut temp_declared_targets,
                        valid_target_assignments,
                        &mut temp_cursor,
                    )
                })
                .unwrap_or_default();
            let mut applicable = Vec::new();
            for branch in &segment.self_replacements {
                if evaluate_self_replacement_branch(
                    game,
                    ctx,
                    branch,
                    &segment.default_effects,
                    representative_effect,
                    representative_assignments.clone(),
                )? {
                    applicable.push(branch);
                }
            }

            match applicable.len() {
                0 => (segment.default_effects.clone(), false),
                1 => (applicable[0].replacement_effects.clone(), true),
                _ => {
                    return Err(GameLoopError::ResolutionFailed(
                        "multiple self-replacement branches applied during resolution".to_string(),
                    ));
                }
            }
        };

        if selected_self_replacement {
            apply_self_replacement_declared_target_tags(
                game,
                ctx,
                &segment.default_effects,
                chosen_modes,
                valid_target_assignments,
                assignment_cursor,
            );
            apply_self_replacement_referenced_target_tags(game, ctx, &selected_effects);
            apply_self_replacement_tag_prelude(game, ctx, &segment.default_effects)?;
        }

        let mut active_scope: Option<(
            Vec<crate::effects::ResolvedTarget>,
            Vec<crate::game_state::TargetAssignment>,
        )> = None;
        for effect in &selected_effects {
            let is_modal_effect = effect.modal_effect_spec().is_some();
            let assignment_start = assignment_cursor;
            let effect_target_assignments = active_target_assignments_for_effect(
                effect,
                chosen_modes,
                &mut consumed_modal_selection,
                &mut declared_targets,
                valid_target_assignments,
                &mut assignment_cursor,
            );
            if is_modal_effect {
                active_scope = None;
            } else if !effect_target_assignments.is_empty()
                || effect_references_prior_target_player(effect)
                || effect_references_prior_object_targets(effect)
            {
                let scope_assignments = if effect_references_prior_target_player(effect) {
                    let previous_assignment_end =
                        if assignment_start == 0 && effect_target_assignments.is_empty() {
                            valid_target_assignments.len()
                        } else {
                            assignment_start
                        };
                    let mut assignments = previous_player_target_assignments(
                        &ctx.targets,
                        valid_target_assignments,
                        previous_assignment_end,
                    );
                    assignments.extend(effect_target_assignments.clone());
                    assignments
                } else if effect_references_prior_object_targets(effect) {
                    let mut assignments = previous_object_target_assignments(
                        &ctx.targets,
                        valid_target_assignments,
                        assignment_start,
                    );
                    assignments.extend(effect_target_assignments.clone());
                    assignments
                } else {
                    effect_target_assignments.clone()
                };
                let (effect_targets, effect_target_assignments) =
                    rebase_target_scope(&ctx.targets, &scope_assignments);
                active_scope = Some((effect_targets, effect_target_assignments));
            }
            let outcome = if !is_modal_effect
                && let Some((effect_targets, effect_target_assignments)) = &active_scope
            {
                ctx.with_temp_targets(effect_targets.clone(), |ctx| {
                    ctx.with_temp_target_assignments(effect_target_assignments.clone(), |ctx| {
                        execute_effect(game, effect, ctx)
                    })
                })
            } else {
                execute_effect(game, effect, ctx)
            };
            match outcome {
                Ok(outcome) => {
                    all_events.extend(outcome.events);
                }
                Err(crate::effects::ExecutionError::InvalidTarget) => {}
                Err(err) => return Err(GameLoopError::ResolutionFailed(err.to_string())),
            }
            // CR 724.1b/724.2b exile the resolving object. No later
            // instructions in its resolution program are performed.
            if game.turn_store.end_turn_procedure_pending
                || game.turn_store.end_combat_phase_procedure_pending
            {
                return Ok(Vec::new());
            }
            if game.subgame_depth() > initial_subgame_depth {
                return Ok(all_events);
            }
            if ctx.decision_maker.awaiting_choice() {
                return Ok(all_events);
            }
        }
    }
    Ok(all_events)
}

// ============================================================================
// Stack Resolution
// ============================================================================

/// Resolve the top entry on the stack.
///
/// This function:
/// 1. Pops the top entry from the stack
/// 2. Validates targets
/// 3. Executes effects
/// 4. Moves spell to graveyard (if spell, not ability)
///
/// Note: May effects will be auto-declined. Use `resolve_stack_entry_with` to
/// provide a decision maker for interactive May choices.
pub fn resolve_stack_entry(game: &mut GameState) -> Result<(), GameLoopError> {
    let mut auto_dm = crate::decision::AutoPassDecisionMaker;
    resolve_stack_entry_full(game, &mut auto_dm, None)
}

/// Resolve the top entry on the stack with both a decision maker and trigger queue.
///
/// Use this for ETB replacement effects that need player decisions (like Mox Diamond).
pub(super) fn resolve_stack_entry_with_dm_and_triggers(
    game: &mut GameState,
    decision_maker: &mut impl DecisionMaker,
    trigger_queue: &mut TriggerQueue,
) -> Result<(), GameLoopError> {
    resolve_stack_entry_full(game, decision_maker, Some(trigger_queue))
}

/// Resolve the top entry on the stack with an optional decision maker.
///
/// If a decision maker is provided, May effects will prompt the player.
/// Otherwise, May effects are auto-declined.
pub fn resolve_stack_entry_with(
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
) -> Result<(), GameLoopError> {
    resolve_stack_entry_full(game, decision_maker, None)
}

/// Resolve the top entry on the stack with optional decision maker and trigger queue.
///
/// If a trigger_queue is provided, saga lore counters are processed immediately.
/// Otherwise, saga processing must be handled by the caller.
pub(super) fn resolve_stack_entry_full(
    game: &mut GameState,
    decision_maker: &mut dyn DecisionMaker,
    mut trigger_queue: Option<&mut TriggerQueue>,
) -> Result<(), GameLoopError> {
    game.refresh_continuous_state();
    let entry = game
        .pop_from_stack()
        .ok_or_else(|| GameLoopError::InvalidState("Stack is empty".to_string()))?;

    // Get the object for this stack entry
    let mut obj = game.object(entry.object_id).cloned();

    if stack_entry_is_countered_by_unpaid_ward(game, &entry, decision_maker) {
        return Ok(());
    }

    // Create execution context
    // Resolution effects use EventCause::from_effect to distinguish from cost effects
    let execution_source = if entry.is_ability {
        entry
            .source_snapshot
            .as_ref()
            .map(|snapshot| snapshot.object_id)
            .unwrap_or(entry.object_id)
    } else {
        entry.object_id
    };
    let mut ctx = ExecutionContext::new(execution_source, entry.controller, decision_maker)
        .with_optional_costs_paid(entry.optional_costs_paid.clone())
        .with_casting_method(entry.casting_method.clone())
        .with_mana_usage_restrictions(entry.mana_usage_restrictions.clone())
        .with_mana_source_chosen_creature_type(entry.mana_source_chosen_creature_type)
        .with_activation_mana_payment(entry.mana_spent_on_activation.clone())
        .with_cause(EventCause::from_effect(entry.object_id, entry.controller))
        .with_provenance(entry.provenance);
    if let Some(x) = entry.x_value {
        ctx = ctx.with_x(x);
    }
    ctx.effect_outcomes = entry.effect_outcomes.clone();
    if let Some(defending) = entry.defending_player {
        ctx = ctx.with_defending_player(defending);
    }
    if let Some(triggering_event) = entry.triggering_event.clone() {
        if let Some(attacked) =
            triggering_event.downcast::<crate::events::combat::CreatureAttackedEvent>()
        {
            if let Some(attacker) = game.object(attacked.attacker) {
                ctx = ctx.with_attacking_player(game.controller_of(attacker));
            }
        } else if let Some(attacked) =
            triggering_event.downcast::<crate::events::combat::CreatureAttackedAndUnblockedEvent>()
            && let Some(attacker) = game.object(attacked.attacker)
        {
            ctx = ctx.with_attacking_player(game.controller_of(attacker));
        }
    }
    if entry.chosen_player.is_some() {
        ctx = ctx.with_chosen_player(entry.chosen_player);
    }
    if let Some(triggering_event) = entry.triggering_event.clone() {
        ctx = ctx.with_triggering_event(triggering_event);
    }
    if let Some(event_value_amount) = entry.event_value_amount {
        ctx = ctx.with_event_value_amount(event_value_amount);
    }
    if let Some(trigger_identity) = entry.trigger_identity {
        ctx = ctx.with_trigger_identity(trigger_identity);
    }
    if let Some(ability_index) = entry.ability_index {
        ctx = ctx.with_ability_index(ability_index);
    }
    if let Some(source_snapshot) = entry.source_snapshot.clone() {
        ctx = ctx.with_source_snapshot(source_snapshot);
    }
    let mut tagged_objects = entry.tagged_objects.clone();
    let source_exiled = game
        .get_exiled_with_source_links(execution_source)
        .iter()
        .filter_map(|id| {
            game.object(*id).map(|obj| {
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                )
            })
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        tagged_objects.insert(
            crate::tag::TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            source_exiled,
        );
    }
    if !tagged_objects.is_empty() {
        ctx = ctx.with_tagged_objects(tagged_objects);
    }
    // Pass pre-chosen modes from casting (per MTG rule 601.2b)
    if let Some(ref modes) = entry.chosen_modes {
        ctx = ctx.with_chosen_modes(Some(modes.clone()));
    }
    apply_keyword_payment_tags_for_resolution(game, &entry, &mut ctx);

    // Convert targets and validate them
    // Per MTG Rule 608.2b, if ALL targets are now illegal, the spell/ability fizzles
    let target_validation_view = crate::derived_view::DerivedGameView::from_refreshed_state(game);
    let (valid_targets, valid_target_assignments, all_targets_invalid) =
        validate_stack_entry_targets_with_view(game, &entry, &target_validation_view);

    let mutating_creature_spell = !entry.is_ability
        && obj.as_ref().is_some_and(|obj| {
            obj.zone == Zone::Stack
                && stack_entry_cast_with_named_alternative(game, &entry, obj, "Mutate")
        });
    let mutate_target = mutating_creature_spell.then(|| {
        valid_targets.iter().find_map(|target| match target {
            crate::effects::ResolvedTarget::Object(id) => Some(*id),
            crate::effects::ResolvedTarget::Player(_) => None,
        })
    });

    let bestow_resolves_as_creature_after_illegal_target = !entry.is_ability
        && all_targets_invalid
        && obj
            .as_ref()
            .is_some_and(|obj| obj.zone == Zone::Stack && obj.is_bestow_overlay_active());
    if bestow_resolves_as_creature_after_illegal_target {
        if let Some(stack_obj) = game.object_mut(entry.object_id) {
            stack_obj.end_bestow_cast_overlay();
            obj = Some(stack_obj.clone());
        }
    }

    // CR 702.140b: an illegally targeted mutating creature spell does not
    // fizzle. It stops being a mutating creature spell and continues resolving
    // as an ordinary creature spell.
    let mutate_resolves_as_creature_after_illegal_target =
        mutating_creature_spell && all_targets_invalid;

    // If the spell/ability had targets and ALL are now invalid, it fizzles.
    // Bestow and Mutate are keyword-specific exceptions: each stops using its
    // alternative permanent behavior and continues resolving as a creature.
    if !entry.targets.is_empty() && all_targets_invalid {
        if !bestow_resolves_as_creature_after_illegal_target
            && !mutate_resolves_as_creature_after_illegal_target
        {
            // Spell fizzles - move to graveyard without executing effects
            if let Some(obj) = &obj
                && obj.zone == Zone::Stack
                && !entry.is_ability
            {
                // Move spell to owner's graveyard (via replacement effects)
                let _ = crate::effects::zones::apply_zone_change(
                    game,
                    entry.object_id,
                    Zone::Stack,
                    Zone::Graveyard,
                    crate::events::cause::EventCause::from_game_rule(),
                    &mut *decision_maker,
                );
            }
            return Ok(());
        }
    }

    if let Some(trigger_identity) = entry.trigger_identity {
        game.record_triggered_ability_resolved(execution_source, trigger_identity);
    }
    if let Some(ability_index) = entry.ability_index {
        game.record_activated_ability_resolved(execution_source, ability_index);
    }

    if !bind_singular_active_player_choice(
        game,
        &mut ctx,
        entry
            .intervening_if
            .as_ref()
            .is_some_and(|condition| format!("{condition:?}").contains("Active")),
    ) {
        return Ok(());
    }

    // Check intervening-if condition at resolution time
    // If the condition is false, the ability does nothing (but doesn't fizzle)
    if let Some(ref condition) = entry.intervening_if
        && let Some(ref triggering_event) = entry.triggering_event
        && !crate::triggers::verify_intervening_if(
            game,
            condition,
            entry.controller,
            triggering_event,
            execution_source,
            None,
            Some(&entry.optional_costs_paid),
        )
    {
        // Condition no longer true - ability resolves but does nothing
        return Ok(());
    }
    // If no triggering event is set (shouldn't happen for triggered abilities),
    // we allow the ability to proceed rather than creating a fake event

    ctx = ctx
        .with_targets(valid_targets)
        .with_target_assignments(valid_target_assignments.clone())
        .with_target_distributions(entry.target_distributions.clone());

    // Snapshot target objects for "last known information" before effects execute
    // This allows effects to access power/controller of targets even after they're exiled
    ctx.snapshot_targets(game);

    // Get effects to execute
    // For abilities with stored effects (like triggered abilities), use those directly
    // even if the source object no longer exists (e.g., undying triggers from dead creatures)
    let program = if let Some(ref ability_effects) = entry.ability_effects {
        ability_effects.clone()
    } else if let Some(obj) = &obj {
        get_effects_for_stack_entry(game, &entry, obj)
    } else {
        crate::resolution::ResolutionProgram::default()
    };
    // ETB replacement is resolved when the spell actually moves to the battlefield.
    let etb_replacement_result: Option<(bool, bool, Zone)> = None;
    let chapter_resolution = entry
        .is_ability
        .then(|| resolved_chapter_ability_event(game, &entry))
        .flatten();

    let initial_subgame_depth = game.subgame_depth();
    let all_events = execute_resolution_program(
        game,
        &mut ctx,
        entry.controller,
        entry.object_id,
        &program,
        entry.chosen_modes.as_deref(),
        &valid_target_assignments,
    )?;
    if game.subgame_depth() > initial_subgame_depth {
        return Ok(());
    }
    if game.turn_store.end_turn_procedure_pending
        || game.turn_store.end_combat_phase_procedure_pending
    {
        // The ending procedure already handled the resolving stack object.
        // Preserve its newly emitted events for the procedure's deferred
        // trigger check and suppress ordinary post-resolution processing.
        game.turn.priority_player = None;
        return Ok(());
    }
    if ctx.decision_maker.awaiting_choice() {
        return Ok(());
    }
    // Process events from effect outcomes for triggers
    if let Some(ref mut tq) = trigger_queue {
        for event in all_events {
            queue_triggers_from_event(game, tq, event, false);
        }
    }

    // Process pending primitive trigger events emitted by effects and zone changes.
    if let Some(ref mut tq) = trigger_queue {
        drain_pending_trigger_events(game, tq);
    }

    if let Some(chapter_resolution) = chapter_resolution {
        let event_provenance = game
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::ChapterAbilityResolved);
        let event = TriggerEvent::new_with_provenance(
            crate::events::other::ChapterAbilityResolvedEvent::new(
                chapter_resolution.saga_id,
                chapter_resolution.controller,
                true,
            ),
            event_provenance,
        )
        .with_source_snapshot(chapter_resolution.source_snapshot);
        if let Some(ref mut tq) = trigger_queue {
            queue_triggers_from_event(game, tq, event, false);
        }
    }

    if !entry.is_ability
        && let Some(obj) = &obj
    {
        install_epic_resolution_effects(game, &entry, obj)?;
    }

    // Resolving an ability removes only that stack entry. The source object can
    // itself be a spell on the stack, such as Lightning Storm, and must remain
    // there until the spell entry resolves.
    if entry.is_ability {
        preserve_resolved_spell_ability_tags(game, execution_source, &ctx);
        return Ok(());
    }

    // Move spell to appropriate zone after resolution
    if let Some(obj) = &obj {
        if obj.zone == Zone::Stack && obj.is_permanent() {
            if let Some(target_id) = mutate_target.flatten() {
                let options = vec![
                    crate::decisions::SelectableOption::new(0, "Put the mutating spell on top")
                        .with_object(entry.object_id),
                    crate::decisions::SelectableOption::new(1, "Put the mutating spell on bottom")
                        .with_object(target_id),
                ];
                let choice_context = crate::decisions::SelectOptionsContext::new(
                    entry.controller,
                    Some(entry.object_id),
                    "Choose the order of the merged permanent",
                    options,
                    1,
                    1,
                );
                let spell_on_top = decision_maker
                    .decide_options(game, &choice_context)
                    .into_iter()
                    .next()
                    .unwrap_or(0)
                    == 0;
                if decision_maker.awaiting_choice() {
                    return Ok(());
                }

                if game
                    .merge_mutating_creature_spell(entry.object_id, target_id, spell_on_top)
                    .is_some()
                {
                    let event_provenance = game
                        .provenance_graph_mut()
                        .alloc_root_event(crate::events::EventKind::Mutated);
                    let event = TriggerEvent::new_with_provenance(
                        crate::events::other::MutatedEvent::new(target_id, entry.controller),
                        event_provenance,
                    );
                    if let Some(ref mut tq) = trigger_queue {
                        queue_triggers_from_event(game, tq, event, false);
                    } else {
                        game.record_turn_history_event(&event);
                    }
                    return Ok(());
                }
            }

            let chosen_player = entry
                .chosen_player
                .or_else(|| game.chosen_player(entry.object_id));
            let cast_with_sneak =
                stack_entry_cast_with_named_alternative(game, &entry, obj, "Sneak");
            let mut sneak_attack_target = if cast_with_sneak {
                pop_sneak_attack_target(game, entry.object_id)
            } else {
                None
            };
            // Handle ETB replacement: if player didn't satisfy the replacement, redirect
            if let Some((enters, enters_tapped, redirect_zone)) = etb_replacement_result {
                if !enters {
                    // Permanent goes to redirect zone instead of battlefield
                    let _ = crate::effects::zones::apply_zone_change(
                        game,
                        entry.object_id,
                        Zone::Stack,
                        redirect_zone,
                        crate::events::cause::EventCause::from_effect(
                            entry.object_id,
                            entry.controller,
                        ),
                        &mut *decision_maker,
                    );
                    return Ok(());
                }

                // Copy optional_costs_paid to the permanent before moving to battlefield
                if let Some(perm) = game.object_mut(entry.object_id) {
                    perm.optional_costs_paid = entry.optional_costs_paid.clone();
                    perm.cast_tagged_objects = entry.tagged_objects.clone();
                }

                // Interactive replacement was already processed above - skip second ETB processing
                // and move directly to battlefield (avoids double-processing)
                let new_id = game.move_object_by_effect(entry.object_id, Zone::Battlefield);
                if let Some(id) = new_id {
                    if entry.controller != obj.owner {
                        game.set_current_controller(id, entry.controller);
                    }
                    if let Some(chosen_player) = chosen_player {
                        game.set_chosen_player(id, chosen_player);
                    }
                    // Apply enters tapped if needed (e.g., shock land not paying life)
                    if enters_tapped || cast_with_sneak {
                        game.tap(id);
                    }
                    if let Some(attack_target) = sneak_attack_target.take()
                        && attack_target_still_valid(game, &attack_target)
                        && let Some(combat) = game.combat.as_mut()
                    {
                        combat.attackers.push(crate::combat_state::AttackerInfo {
                            creature: id,
                            target: attack_target,
                        });
                    }

                    if let Some(ref mut tq) = trigger_queue {
                        // Drain pending ZoneChangeEvent emitted by move_object.
                        drain_pending_trigger_events(game, tq);
                    }

                    // Check for ETB triggers
                    if let Some(ref mut tq) = trigger_queue {
                        let etb_event_provenance = game
                            .provenance_graph_mut()
                            .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                        let etb_event = if enters_tapped || cast_with_sneak {
                            TriggerEvent::new_with_provenance(
                                EnterBattlefieldEvent::tapped(id, Zone::Stack),
                                etb_event_provenance,
                            )
                        } else {
                            TriggerEvent::new_with_provenance(
                                EnterBattlefieldEvent::new(id, Zone::Stack),
                                etb_event_provenance,
                            )
                        };
                        let etb_event = game.ensure_trigger_event_provenance(etb_event);
                        let etb_triggers = check_triggers(game, &etb_event);
                        for trigger in etb_triggers {
                            tq.add(trigger);
                        }
                    }
                }
                return Ok(());
            }

            // No interactive replacement was handled above - use normal ETB processing
            // Copy optional_costs_paid to the permanent before moving to battlefield
            // (so ETB triggers can access kick count, etc.)
            if let Some(perm) = game.object_mut(entry.object_id) {
                perm.optional_costs_paid = entry.optional_costs_paid.clone();
                perm.cast_tagged_objects = entry.tagged_objects.clone();
                // Preserve Convoke/Improvise contributors for later triggered ability resolution.
                perm.keyword_payment_contributions_to_cast =
                    entry.keyword_payment_contributions.clone();
            }

            // It's a permanent spell, move to battlefield with ETB processing
            // This handles replacement effects like "enters tapped" or "enters with counters"
            let etb_result = game.move_object_with_etb_processing_with_dm(
                entry.object_id,
                Zone::Battlefield,
                decision_maker,
            );

            // Note: Use the new ID from ETB result since zone change creates a new object
            if let Some(result) = etb_result {
                if entry.controller != obj.owner {
                    game.set_current_controller(result.new_id, entry.controller);
                }
                if let Some(chosen_player) = chosen_player {
                    game.set_chosen_player(result.new_id, chosen_player);
                }
                // If this is an Aura, attach it to its target as it enters
                if obj.subtypes.contains(&Subtype::Aura) {
                    let attached = entry.targets.iter().find_map(|target| match target {
                        Target::Object(id) => Some(crate::object::AttachmentTarget::Object(*id)),
                        Target::Player(id) => Some(crate::object::AttachmentTarget::Player(*id)),
                    });
                    if let Some(target) = attached
                        && game.attach_object_to_target(result.new_id, target)
                    {
                        game.effect_store
                            .continuous_effects
                            .record_attachment(result.new_id);
                    }
                }
                if cast_with_sneak && !result.enters_tapped {
                    game.tap(result.new_id);
                }
                if let Some(attack_target) = sneak_attack_target.take()
                    && attack_target_still_valid(game, &attack_target)
                    && let Some(combat) = game.combat.as_mut()
                {
                    combat.attackers.push(crate::combat_state::AttackerInfo {
                        creature: result.new_id,
                        target: attack_target,
                    });
                }

                let cast_with_dash = match &entry.casting_method {
                    CastingMethod::Alternative(idx) => matches!(
                        obj.alternative_casts.get(*idx),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Dash { .. })
                    ),
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    }
                    | CastingMethod::SplitOtherHalfPlayFrom {
                        use_alternative: idx,
                        zone,
                        ..
                    } => matches!(
                        crate::decision::resolve_play_from_alternative_method(
                            game,
                            entry.controller,
                            obj,
                            *zone,
                            *idx,
                        ),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Dash { .. })
                    ),
                    _ => false,
                };
                let cast_with_blitz = match &entry.casting_method {
                    CastingMethod::Alternative(idx) => matches!(
                        obj.alternative_casts.get(*idx),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Blitz { .. })
                    ),
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    }
                    | CastingMethod::SplitOtherHalfPlayFrom {
                        use_alternative: idx,
                        zone,
                        ..
                    } => matches!(
                        crate::decision::resolve_play_from_alternative_method(
                            game,
                            entry.controller,
                            obj,
                            *zone,
                            *idx,
                        ),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Blitz { .. })
                    ),
                    _ => false,
                } || entry.optional_costs_paid.was_paid_label("Blitz")
                    || obj.optional_costs_paid.was_paid_label("Blitz");
                let cast_with_warp = match &entry.casting_method {
                    CastingMethod::Alternative(idx) => matches!(
                        obj.alternative_casts.get(*idx),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Warp { .. })
                    ),
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    }
                    | CastingMethod::SplitOtherHalfPlayFrom {
                        use_alternative: idx,
                        zone,
                        ..
                    } => matches!(
                        crate::decision::resolve_play_from_alternative_method(
                            game,
                            entry.controller,
                            obj,
                            *zone,
                            *idx,
                        ),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Warp { .. })
                    ),
                    _ => false,
                };
                let cast_with_suspend = match &entry.casting_method {
                    CastingMethod::PlayFrom {
                        use_alternative: Some(idx),
                        zone,
                        ..
                    }
                    | CastingMethod::SplitOtherHalfPlayFrom {
                        use_alternative: idx,
                        zone,
                        ..
                    } if *zone == Zone::Exile => matches!(
                        crate::decision::resolve_play_from_alternative_method(
                            game,
                            entry.controller,
                            obj,
                            *zone,
                            *idx,
                        ),
                        Some(crate::alternative_cast::AlternativeCastingMethod::Suspend { .. })
                    ),
                    _ => false,
                };
                if cast_with_dash {
                    let dash_haste = crate::effects::ApplyContinuousEffect::new(
                        crate::continuous::EffectTarget::Specific(result.new_id),
                        crate::continuous::Modification::AddAbility(
                            crate::static_abilities::StaticAbility::haste(),
                        ),
                        crate::effect::Until::EndOfTurn,
                    )
                    .with_source_type(
                        crate::continuous::EffectSourceType::Resolution {
                            locked_targets: vec![result.new_id],
                        },
                    );
                    let _ = crate::effects::execute_effect(
                        game,
                        &crate::effect::Effect::new(dash_haste),
                        &mut crate::effects::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );

                    let return_to_hand = crate::effects::ScheduleDelayedTriggerEffect::new(
                        Trigger::beginning_of_end_step(crate::target::PlayerFilter::Any),
                        vec![crate::effect::Effect::new(
                            crate::effects::ReturnToHandEffect::with_spec(
                                crate::target::ChooseSpec::SpecificObject(result.new_id),
                            ),
                        )],
                        true,
                        vec![result.new_id],
                        crate::target::PlayerFilter::Specific(entry.controller),
                    );
                    let _ = crate::effects::execute_effect(
                        game,
                        &crate::effect::Effect::new(return_to_hand),
                        &mut crate::effects::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );
                }
                if cast_with_blitz {
                    let blitz_haste = crate::effects::ApplyContinuousEffect::new(
                        crate::continuous::EffectTarget::Specific(result.new_id),
                        crate::continuous::Modification::AddAbility(
                            crate::static_abilities::StaticAbility::haste(),
                        ),
                        crate::effect::Until::YouStopControllingThis,
                    )
                    .with_source_type(
                        crate::continuous::EffectSourceType::Resolution {
                            locked_targets: vec![result.new_id],
                        },
                    );
                    let _ = crate::effects::execute_effect(
                        game,
                        &crate::effect::Effect::new(blitz_haste),
                        &mut crate::effects::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );

                    let draw_when_dies = crate::effects::ScheduleDelayedTriggerEffect::new(
                        Trigger::this_dies(),
                        vec![crate::effect::Effect::target_draws(
                            1,
                            crate::target::PlayerFilter::Specific(entry.controller),
                        )],
                        true,
                        vec![result.new_id],
                        crate::target::PlayerFilter::Specific(entry.controller),
                    );
                    let _ = crate::effects::execute_effect(
                        game,
                        &crate::effect::Effect::new(draw_when_dies),
                        &mut crate::effects::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );

                    let sacrifice_at_end_step = crate::effects::ScheduleDelayedTriggerEffect::new(
                        Trigger::beginning_of_end_step(crate::target::PlayerFilter::Any),
                        vec![crate::effect::Effect::new(
                            crate::effects::SacrificeTargetEffect::new(
                                crate::target::ChooseSpec::SpecificObject(result.new_id),
                            ),
                        )],
                        true,
                        vec![result.new_id],
                        crate::target::PlayerFilter::Specific(entry.controller),
                    );
                    let _ = crate::effects::execute_effect(
                        game,
                        &crate::effect::Effect::new(sacrifice_at_end_step),
                        &mut crate::effects::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );
                }
                if cast_with_suspend && obj.has_card_type(crate::types::CardType::Creature) {
                    let suspend_haste = crate::effects::ApplyContinuousEffect::new(
                        crate::continuous::EffectTarget::Specific(result.new_id),
                        crate::continuous::Modification::AddAbility(
                            crate::static_abilities::StaticAbility::haste(),
                        ),
                        crate::effect::Until::YouStopControllingThis,
                    );
                    let _ = crate::effects::execute_effect(
                        game,
                        &crate::effect::Effect::new(suspend_haste),
                        &mut crate::effects::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );
                }
                if cast_with_warp {
                    let exile_then_grant = crate::effects::ScheduleDelayedTriggerEffect::new(
                        Trigger::beginning_of_end_step(crate::target::PlayerFilter::Any),
                        vec![crate::effect::Effect::new(
                            crate::effects::ExileThenGrantPlayEffect::new(
                                crate::target::ChooseSpec::SpecificObject(result.new_id),
                                crate::target::PlayerFilter::Specific(obj.owner),
                                crate::grant::GrantDuration::Forever,
                            )
                            .starting_next_turn(),
                        )],
                        true,
                        vec![result.new_id],
                        crate::target::PlayerFilter::Specific(entry.controller),
                    );
                    let _ = crate::effects::execute_effect(
                        game,
                        &crate::effect::Effect::new(exile_then_grant),
                        &mut crate::effects::ExecutionContext::new_default(
                            result.new_id,
                            entry.controller,
                        ),
                    );
                }

                if let Some(ref mut tq) = trigger_queue {
                    handle_saga_enters_battlefield(game, result.new_id, tq, decision_maker);
                } else {
                    let mut temp_queue = TriggerQueue::new();
                    handle_saga_enters_battlefield(
                        game,
                        result.new_id,
                        &mut temp_queue,
                        decision_maker,
                    );
                }

                // Check for ETB triggers and add them to the trigger queue
                if let Some(ref mut tq) = trigger_queue {
                    // Drain pending ZoneChangeEvent emitted by ETB move processing.
                    drain_pending_trigger_events(game, tq);

                    let etb_event_provenance = game
                        .provenance_graph_mut()
                        .alloc_root_event(crate::events::EventKind::EnterBattlefield);
                    let etb_event = if result.enters_tapped || cast_with_sneak {
                        TriggerEvent::new_with_provenance(
                            EnterBattlefieldEvent::tapped(result.new_id, Zone::Stack),
                            etb_event_provenance,
                        )
                    } else {
                        TriggerEvent::new_with_provenance(
                            EnterBattlefieldEvent::new(result.new_id, Zone::Stack),
                            etb_event_provenance,
                        )
                    };
                    let etb_event = game.ensure_trigger_event_provenance(etb_event);
                    let etb_triggers = check_triggers(game, &etb_event);
                    for trigger in etb_triggers {
                        tq.add(trigger);
                    }
                }
            }
        } else if obj.zone == Zone::Stack {
            if obj.kind == crate::object::ObjectKind::SpellCopy {
                game.remove_object(entry.object_id);
                return Ok(());
            }

            // It's an instant/sorcery
            let has_rebound = matches!(entry.casting_method, CastingMethod::Normal)
                && obj.abilities.iter().any(|ability| {
                    ability.functions_in(&Zone::Stack)
                        && matches!(
                            &ability.kind,
                            AbilityKind::Static(static_ability)
                                if static_ability.id()
                                    == crate::static_abilities::StaticAbilityId::Rebound
                        )
                });

            // Check if cast with flashback/escape/jump-start/granted escape (exiles after resolution)
            let should_exile = match &entry.casting_method {
                CastingMethod::Normal => false,
                CastingMethod::FaceDown => false,
                CastingMethod::SplitOtherHalf => {
                    obj.subtypes.contains(&crate::types::Subtype::Adventure)
                }
                CastingMethod::SplitOtherHalfPlayFrom { .. } => true,
                CastingMethod::Fuse => false,
                CastingMethod::Alternative(idx) => obj
                    .alternative_casts
                    .get(*idx)
                    .map(|m| m.exiles_after_resolution())
                    .unwrap_or(false),
                CastingMethod::GrantedEscape { .. } => true, // Granted escape always exiles
                CastingMethod::GrantedFlashback => true,     // Granted flashback always exiles
                CastingMethod::PlayFrom {
                    use_alternative: Some(idx),
                    zone,
                    ..
                } => {
                    // Check if the alternative cost used exiles after resolution
                    crate::decision::resolve_play_from_alternative_method(
                        game,
                        entry.controller,
                        obj,
                        *zone,
                        *idx,
                    )
                    .or_else(|| obj.cast_alternative_method_owned())
                    .map(|m| m.exiles_after_resolution())
                    .unwrap_or(false)
                }
                CastingMethod::PlayFrom {
                    use_alternative: None,
                    ..
                } => {
                    // Normal cost via Yawgmoth's Will - replacement effect handles exile
                    false
                }
            };

            if has_rebound {
                if let crate::events::processing::EventOutcome::Proceed(result) =
                    crate::effects::zones::apply_zone_change(
                        game,
                        entry.object_id,
                        Zone::Stack,
                        Zone::Exile,
                        crate::events::cause::EventCause::from_effect(
                            entry.object_id,
                            entry.controller,
                        ),
                        &mut *decision_maker,
                    )
                    && result.final_zone == Zone::Exile
                    && let Some(exiled_id) = result.new_object_id
                {
                    game.effect_store
                        .delayed_triggers
                        .push(crate::triggers::DelayedTrigger {
                            trigger: crate::triggers::Trigger::beginning_of_upkeep(
                                crate::target::PlayerFilter::Specific(entry.controller),
                            ),
                            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                                Effect::may_single(Effect::new(
                                    crate::effects::CastSourceEffect::new()
                                        .without_paying_mana_cost()
                                        .require_exile(),
                                )),
                            ]),
                            one_shot: true,
                            x_value: entry.x_value,
                            not_before_turn: None,
                            expires_at_turn: None,
                            expires_before_controller_turn_after: None,
                            expires_at_end_of_combat: false,
                            target_objects: vec![exiled_id],
                            ability_source: None,
                            ability_source_stable_id: None,
                            ability_source_name: None,
                            ability_source_snapshot: None,
                            controller: entry.controller,
                            choices: vec![],
                            tagged_objects: std::collections::HashMap::new(),
                        });
                }
            } else if should_exile {
                let was_adventure = obj.subtypes.contains(&crate::types::Subtype::Adventure);
                if let crate::events::processing::EventOutcome::Proceed(result) =
                    crate::effects::zones::apply_zone_change(
                        game,
                        entry.object_id,
                        Zone::Stack,
                        Zone::Exile,
                        crate::events::cause::EventCause::from_effect(
                            entry.object_id,
                            entry.controller,
                        ),
                        &mut *decision_maker,
                    )
                    && result.final_zone == Zone::Exile
                    && let Some(exiled_id) = result.new_object_id
                    && was_adventure
                {
                    game.set_adventure_exiled(exiled_id);
                }
            } else if entry.optional_costs_paid.was_bought_back()
                || obj.optional_costs_paid.was_bought_back()
            {
                let _ = crate::effects::zones::apply_zone_change(
                    game,
                    entry.object_id,
                    Zone::Stack,
                    Zone::Hand,
                    crate::events::cause::EventCause::from_effect(
                        entry.object_id,
                        entry.controller,
                    ),
                    &mut *decision_maker,
                );
            } else {
                // Process zone change through replacement effects
                // (e.g., Yawgmoth's Will exiles cards going to graveyard)
                let _ = crate::effects::zones::apply_zone_change(
                    game,
                    entry.object_id,
                    Zone::Stack,
                    Zone::Graveyard,
                    crate::events::cause::EventCause::from_effect(
                        entry.object_id,
                        entry.controller,
                    ),
                    &mut *decision_maker,
                );
            }
        }
        // Abilities just disappear from the stack
    }

    Ok(())
}

struct ChapterAbilityResolutionInfo {
    saga_id: ObjectId,
    controller: PlayerId,
    source_snapshot: crate::snapshot::ObjectSnapshot,
}

fn resolved_chapter_ability_event(
    game: &GameState,
    entry: &StackEntry,
) -> Option<ChapterAbilityResolutionInfo> {
    let saga_id = entry.chapter_ability_source?;
    let trigger_identity = entry.trigger_identity?;
    let source_snapshot = game
        .object(saga_id)
        .map(|obj| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(obj, game)
        })
        .or_else(|| entry.source_snapshot.clone())?;
    let final_chapter = crate::game_loop::final_chapter_number_from_abilities(
        source_snapshot.abilities.as_slice(),
    )?;
    let resolved_final_chapter = source_snapshot.abilities.iter().any(|ability| {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            return false;
        };
        crate::triggers::compute_trigger_identity(triggered) == trigger_identity
            && triggered
                .trigger
                .saga_chapters()
                .is_some_and(|chapters| chapters.contains(&final_chapter))
    });
    if !resolved_final_chapter {
        return None;
    }
    Some(ChapterAbilityResolutionInfo {
        saga_id,
        controller: source_snapshot.controller,
        source_snapshot,
    })
}

fn stack_entry_is_countered_by_unpaid_ward(
    game: &mut GameState,
    entry: &StackEntry,
    decision_maker: &mut dyn DecisionMaker,
) -> bool {
    let target_ids = entry
        .targets
        .iter()
        .filter_map(|target| match target {
            Target::Object(id) => Some(*id),
            _ => None,
        })
        .collect::<Vec<_>>();
    if target_ids.is_empty() {
        return false;
    }

    for ward_cost in crate::targeting::collect_ward_costs(game, &target_ids, entry.controller) {
        if crate::targeting::handle_ward_payment(
            game,
            &ward_cost,
            entry.controller,
            entry.object_id,
            decision_maker,
        ) == crate::targeting::WardPaymentResult::Paid
        {
            continue;
        }

        if !entry.is_ability
            && let Some(obj) = game.object(entry.object_id)
            && obj.zone == Zone::Stack
        {
            let _ = crate::effects::zones::apply_zone_change(
                game,
                entry.object_id,
                Zone::Stack,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_game_rule(),
                decision_maker,
            );
        }
        return true;
    }

    false
}

/// Get effects for a stack entry.
pub(super) fn get_effects_for_stack_entry(
    _game: &GameState,
    entry: &StackEntry,
    obj: &crate::object::Object,
) -> crate::resolution::ResolutionProgram {
    // If this is an ability with stored effects, use those directly
    if let Some(ref effects) = entry.ability_effects {
        return effects.clone();
    }

    // For spells, check the spell_effect field (instants/sorceries)
    if let Some(effects) = obj.spell_effect.as_ref() {
        return effects.to_owned_value();
    }

    // Permanent spells (creatures, artifacts, enchantments, etc.) don't have effects
    // that execute on resolution - they just enter the battlefield.
    // Don't fall back to looking at their abilities.
    if obj.is_permanent() {
        return crate::resolution::ResolutionProgram::default();
    }

    crate::resolution::ResolutionProgram::default()
}

fn preserve_resolved_spell_ability_tags(
    game: &mut GameState,
    source: ObjectId,
    ctx: &ExecutionContext,
) {
    if ctx.tagged_objects.is_empty() {
        return;
    }
    let source_is_pending_spell = game
        .object(source)
        .is_some_and(|object| object.zone == Zone::Stack);
    if !source_is_pending_spell {
        return;
    }

    if let Some(object) = game.object_mut(source) {
        merge_tagged_objects(&mut object.cast_tagged_objects, &ctx.tagged_objects);
    }
    if let Some(entry) = game
        .stack
        .iter_mut()
        .find(|entry| entry.object_id == source)
    {
        merge_tagged_objects(&mut entry.tagged_objects, &ctx.tagged_objects);
    }
}

fn merge_tagged_objects(
    target: &mut std::collections::HashMap<
        crate::tag::TagKey,
        Vec<crate::snapshot::ObjectSnapshot>,
    >,
    source: &std::collections::HashMap<crate::tag::TagKey, Vec<crate::snapshot::ObjectSnapshot>>,
) {
    for (tag, snapshots) in source {
        let entry = target.entry(tag.clone()).or_default();
        for snapshot in snapshots {
            if !entry.iter().any(|existing| {
                existing.object_id == snapshot.object_id && existing.stable_id == snapshot.stable_id
            }) {
                entry.push(snapshot.clone());
            }
        }
    }
}

fn install_epic_resolution_effects(
    game: &mut GameState,
    entry: &StackEntry,
    obj: &crate::object::Object,
) -> Result<(), GameLoopError> {
    if obj.zone != Zone::Stack || !spell_has_epic_ability(obj) {
        return Ok(());
    }

    let cant_cast = Effect::cant_until(
        crate::effect::Restriction::cast_spells(crate::target::PlayerFilter::You),
        crate::effect::Until::Forever,
    );
    let mut ctx = ExecutionContext::new_default(entry.object_id, entry.controller)
        .with_provenance(entry.provenance);
    crate::effects::execute_effect(game, &cant_cast, &mut ctx)
        .map_err(|err| GameLoopError::ResolutionFailed(err.to_string()))?;

    let copy_effect_id = crate::effect::EffectId(0);
    let copy_effect = Effect::with_id(
        copy_effect_id.0,
        Effect::new(crate::effects::EpicSpellCopyEffect::new(obj, entry)),
    );
    let choose_new_targets = Effect::may_choose_new_targets(copy_effect_id);
    let delayed_program =
        crate::resolution::ResolutionProgram::from_effects(vec![copy_effect, choose_new_targets]);

    let delayed = crate::effects::delayed::DelayedTriggerConfig::new(
        Trigger::beginning_of_upkeep(crate::target::PlayerFilter::Specific(entry.controller)),
        delayed_program,
        false,
        Vec::new(),
        entry.controller,
    )
    .with_ability_source(Some(entry.object_id))
    .with_x_value(entry.x_value)
    .with_tagged_objects(entry.tagged_objects.clone());
    crate::effects::delayed::queue_delayed_trigger(game, delayed);

    Ok(())
}

fn spell_has_epic_ability(obj: &crate::object::Object) -> bool {
    obj.abilities.iter().any(|ability| {
        let AbilityKind::Static(static_ability) = &ability.kind else {
            return false;
        };
        static_ability.id() == crate::static_abilities::StaticAbilityId::KeywordMarker
            && static_ability
                .display()
                .trim()
                .trim_end_matches('.')
                .eq_ignore_ascii_case("epic")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::events::phase::BeginningOfUpkeepEvent;
    use crate::ids::CardId;
    use crate::object::CounterType;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;

    #[derive(Default)]
    struct MatchingOptionDecisionMaker {
        needle: Option<String>,
    }

    impl MatchingOptionDecisionMaker {
        fn new(needle: &str) -> Self {
            Self {
                needle: Some(needle.to_ascii_lowercase()),
            }
        }
    }

    impl crate::decision::DecisionMaker for MatchingOptionDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if let Some(needle) = self.needle.take()
                && let Some(option) = ctx.options.iter().find(|option| {
                    option.legal && option.description.to_ascii_lowercase().contains(&needle)
                })
            {
                return vec![option.index];
            }

            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    #[derive(Default)]
    struct AnswerThenCaptureDecisionMaker {
        answers_remaining: usize,
        captured: Option<Vec<String>>,
    }

    impl AnswerThenCaptureDecisionMaker {
        fn new(answers_remaining: usize) -> Self {
            Self {
                answers_remaining,
                captured: None,
            }
        }
    }

    impl crate::decision::DecisionMaker for AnswerThenCaptureDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.captured.is_some()
        }

        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            if self.answers_remaining > 0 {
                self.answers_remaining -= 1;
                return ctx
                    .options
                    .iter()
                    .filter(|option| option.legal)
                    .map(|option| option.index)
                    .take(ctx.min)
                    .collect();
            }

            if self.captured.is_none() {
                self.captured = Some(
                    ctx.options
                        .iter()
                        .filter(|option| option.legal)
                        .map(|option| option.description.clone())
                        .collect(),
                );
            }

            ctx.options
                .iter()
                .filter(|option| option.legal)
                .map(|option| option.index)
                .take(ctx.min)
                .collect()
        }
    }

    fn parse_spell_definition(
        name: &str,
        card_types: Vec<CardType>,
        oracle_text: &str,
    ) -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(card_types)
            .parse_text(oracle_text)
            .unwrap_or_else(|err| panic!("{name} should parse: {err:?}"))
    }

    fn parse_sorcery_definition(name: &str, oracle_text: &str) -> crate::cards::CardDefinition {
        parse_spell_definition(name, vec![CardType::Sorcery], oracle_text)
    }

    fn parse_instant_definition(name: &str, oracle_text: &str) -> crate::cards::CardDefinition {
        parse_spell_definition(name, vec![CardType::Instant], oracle_text)
    }

    fn tangle_wire_definition() -> crate::cards::CardDefinition {
        CardDefinitionBuilder::new(CardId::from_raw(3_694), "Tangle Wire")
            .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                crate::mana::ManaSymbol::Generic(3),
            ]]))
            .card_types(vec![CardType::Artifact])
            .parse_text(concat!(
                "Fading 4 (This artifact enters with four fade counters on it. ",
                "At the beginning of your upkeep, remove a fade counter from it. ",
                "If you can't, sacrifice it.)\n",
                "At the beginning of each player's upkeep, that player taps an untapped artifact, ",
                "creature, or land they control for each fade counter on this artifact."
            ))
            .expect("Tangle Wire should parse for runtime test")
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        power: i32,
        toughness: i32,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn create_artifact(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn register_spell_cast_this_turn_for_test(
        game: &mut GameState,
        spell_id: ObjectId,
        caster: PlayerId,
    ) {
        let event = TriggerEvent::new_with_provenance(
            crate::events::spells::SpellCastEvent::new(spell_id, caster, Zone::Hand),
            crate::provenance::ProvNodeId::default(),
        );
        game.stage_turn_history_event(&event);
    }

    #[test]
    fn epic_resolution_schedules_repeating_upkeep_copy_without_epic() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(91_101), "Epic Life")
            .card_types(vec![CardType::Sorcery])
            .build();
        let spell_id = game.create_object_from_card(&card, alice, Zone::Stack);
        {
            let spell = game.object_mut(spell_id).expect("spell exists");
            spell.abilities_mut().push(
                Ability::static_ability(StaticAbility::keyword_marker("Epic"))
                    .in_zones(vec![Zone::Stack]),
            );
            spell.spell_effect = Some(
                crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life(1)])
                    .into(),
            );
        }
        game.push_to_stack(StackEntry::new(spell_id, alice));

        let mut trigger_queue = TriggerQueue::new();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Epic spell should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 21);
        assert!(!game.can_cast_spells(alice));
        assert_eq!(game.effect_store.delayed_triggers.len(), 1);

        let upkeep_event = TriggerEvent::new_with_provenance(
            BeginningOfUpkeepEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );
        for trigger in crate::triggers::check_delayed_triggers(&mut game, &upkeep_event) {
            trigger_queue.add(trigger);
        }
        super::super::sba_triggers::put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("Epic delayed trigger should go on the stack");
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Epic delayed trigger should resolve");

        let copy_id = game
            .stack
            .last()
            .expect("copy should be on stack")
            .object_id;
        let copy = game.object(copy_id).expect("copy object");
        assert!(
            !spell_has_epic_ability(copy),
            "Epic upkeep copy must not copy Epic"
        );

        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Epic copy should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 22);
        assert_eq!(
            game.effect_store.delayed_triggers.len(),
            1,
            "upkeep copy should not install another Epic delayed trigger"
        );
    }

    #[test]
    fn tangle_wire_upkeep_trigger_has_active_player_choose_their_permanent() {
        struct TangleWireDecisionMaker {
            expected_player: PlayerId,
            selected: ObjectId,
            saw_object_choice: bool,
        }

        impl crate::decision::DecisionMaker for TangleWireDecisionMaker {
            fn decide_objects(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                assert_eq!(
                    ctx.player, self.expected_player,
                    "Tangle Wire's active upkeep player should choose what they tap"
                );
                assert!(
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.id == self.selected),
                    "expected selected permanent to be eligible: {:?}",
                    ctx.candidates
                );
                self.saw_object_choice = true;
                vec![self.selected]
            }
        }

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.active_player = bob;
        game.turn.phase = crate::game_state::Phase::Beginning;
        game.turn.step = Some(crate::game_state::Step::Upkeep);

        let tangle_wire =
            game.create_object_from_definition(&tangle_wire_definition(), alice, Zone::Battlefield);
        game.add_counters(tangle_wire, CounterType::Fade, 1)
            .expect("Tangle Wire should accept fade counters");
        let alice_artifact = create_artifact(&mut game, "Alice Relic", alice);
        let bob_default_artifact = create_artifact(&mut game, "Bob Relic", bob);
        let bob_selected_artifact = create_artifact(&mut game, "Bob Choice", bob);

        let upkeep_event = crate::triggers::generate_step_trigger_events(&game)
            .expect("Bob's upkeep should generate a trigger event");
        let mut trigger_queue = TriggerQueue::new();
        for trigger in crate::triggers::check_triggers(&game, &upkeep_event) {
            trigger_queue.add(trigger);
        }
        put_triggers_on_stack(&mut game, &mut trigger_queue)
            .expect("Tangle Wire trigger should go on the stack");

        let mut dm = TangleWireDecisionMaker {
            expected_player: bob,
            selected: bob_selected_artifact,
            saw_object_choice: false,
        };
        resolve_stack_entry_with(&mut game, &mut dm).expect("Tangle Wire trigger should resolve");

        assert!(
            dm.saw_object_choice,
            "Tangle Wire should ask the active player to choose a permanent"
        );
        assert!(game.is_tapped(bob_selected_artifact));
        assert!(
            !game.is_tapped(bob_default_artifact),
            "the test chooses a non-default eligible permanent"
        );
        assert!(
            !game.is_tapped(alice_artifact),
            "Tangle Wire should not tap the controller's permanent during Bob's upkeep"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn stack_resolution_tracks_creature_damage_for_backdraft_history() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        create_creature(&mut game, "Goblin 1", bob, 2, 2);
        create_creature(&mut game, "Goblin 2", bob, 2, 2);

        let blasphemous_act = parse_sorcery_definition(
            "Blasphemous Act",
            "This spell deals 13 damage to each creature.",
        );
        let backdraft = parse_sorcery_definition(
            "Backdraft",
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        );

        let blasphemous_act_id =
            game.create_object_from_definition(&blasphemous_act, bob, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, blasphemous_act_id, bob);
        game.push_to_stack(StackEntry::new(blasphemous_act_id, bob));

        let mut trigger_queue = TriggerQueue::new();
        let mut auto_dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut auto_dm, &mut trigger_queue)
            .expect("Blasphemous Act should resolve");

        assert_eq!(
            game.turn_store
                .turn_history
                .damage_dealt_by_spell_this_turn(game.provenance_graph(), blasphemous_act_id),
            26,
            "stack-resolved creature damage should be queryable from turn history"
        );

        let bob_life_before = game.player(bob).expect("bob exists").life;
        let backdraft_id = game.create_object_from_definition(&backdraft, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, backdraft_id, alice);
        game.push_to_stack(StackEntry::new(backdraft_id, alice));

        let mut dm = MatchingOptionDecisionMaker::new("Bob");
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Backdraft should resolve");

        assert_eq!(
            game.player(bob).expect("bob exists").life,
            bob_life_before - 13,
            "Backdraft should use the creature damage dealt by Blasphemous Act"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn backdraft_prompts_for_spell_history_when_the_same_player_cast_both_spells() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        create_creature(&mut game, "Ornithopter 1", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 2", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 3", alice, 0, 2);

        let blasphemous_act = parse_sorcery_definition(
            "Blasphemous Act",
            "This spell deals 13 damage to each creature.",
        );
        let backdraft = parse_sorcery_definition(
            "Backdraft",
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        );

        let blasphemous_act_id =
            game.create_object_from_definition(&blasphemous_act, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, blasphemous_act_id, alice);
        game.push_to_stack(StackEntry::new(blasphemous_act_id, alice));

        let mut trigger_queue = TriggerQueue::new();
        let mut auto_dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut auto_dm, &mut trigger_queue)
            .expect("Blasphemous Act should resolve");

        assert_eq!(
            game.turn_store
                .turn_history
                .damage_dealt_by_spell_this_turn(game.provenance_graph(), blasphemous_act_id),
            39,
            "Blasphemous Act should record the 39 damage dealt to the three Ornithopters"
        );

        let history_names = game
            .turn_store
            .turn_history
            .spell_cast_snapshot_history()
            .into_iter()
            .map(|snapshot| snapshot.name)
            .collect::<Vec<_>>();
        assert!(
            history_names.iter().any(|name| name == "Blasphemous Act"),
            "resolved Blasphemous Act should remain in spell-cast history, got {history_names:?}"
        );

        let backdraft_id = game.create_object_from_definition(&backdraft, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, backdraft_id, alice);
        game.push_to_stack(StackEntry::new(backdraft_id, alice));

        let mut dm = AnswerThenCaptureDecisionMaker::new(1);
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut dm, &mut trigger_queue)
            .expect("Backdraft should resolve far enough to ask for a spell-history choice");

        let captured = dm.captured.expect(
            "Backdraft should prompt for one of Alice's sorcery spells after choosing Alice",
        );
        assert!(
            captured
                .iter()
                .any(|option| option.contains("Blasphemous Act")),
            "expected Blasphemous Act to be a legal Backdraft history option, got {captured:?}"
        );
        assert!(
            captured.iter().any(|option| option.contains("Backdraft")),
            "expected Backdraft to also remain a legal history option, got {captured:?}"
        );
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn run_priority_loop_finishes_backdraft_after_the_single_player_prompt() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        game.turn.active_player = alice;
        game.turn.priority_player = Some(alice);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;

        create_creature(&mut game, "Ornithopter 1", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 2", alice, 0, 2);
        create_creature(&mut game, "Ornithopter 3", alice, 0, 2);

        let blasphemous_act = parse_sorcery_definition(
            "Blasphemous Act",
            "This spell deals 13 damage to each creature.",
        );
        let backdraft = parse_instant_definition(
            "Backdraft",
            "Choose a player who cast one or more sorcery spells this turn. Backdraft deals damage to that player equal to half the damage dealt by one of those sorcery spells this turn, rounded down.",
        );

        let blasphemous_act_id =
            game.create_object_from_definition(&blasphemous_act, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, blasphemous_act_id, alice);
        game.push_to_stack(StackEntry::new(blasphemous_act_id, alice));

        let mut trigger_queue = TriggerQueue::new();
        let mut auto_dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with_dm_and_triggers(&mut game, &mut auto_dm, &mut trigger_queue)
            .expect("Blasphemous Act should resolve");

        assert_eq!(
            game.turn_store
                .turn_history
                .damage_dealt_by_spell_this_turn(game.provenance_graph(), blasphemous_act_id),
            39,
            "Blasphemous Act should record the 39 damage dealt to the three Ornithopters"
        );

        let alice_life_before = game.player(alice).expect("alice exists").life;
        let backdraft_id = game.create_object_from_definition(&backdraft, alice, Zone::Stack);
        register_spell_cast_this_turn_for_test(&mut game, backdraft_id, alice);
        game.push_to_stack(StackEntry::new(backdraft_id, alice));

        let mut dm = MatchingOptionDecisionMaker::new("Alice");
        let result =
            crate::game_loop::run_priority_loop_with(&mut game, &mut trigger_queue, &mut dm)
                .expect("priority loop should resolve Backdraft after choosing Alice");

        assert!(
            matches!(result, crate::decision::GameProgress::Continue),
            "priority loop should finish cleanly after resolving Backdraft, got {result:?}"
        );
        assert_eq!(
            game.player(alice).expect("alice exists").life,
            alice_life_before - 19,
            "Backdraft should still deal half of Blasphemous Act's 39 damage after the player choice"
        );
    }

    #[test]
    fn shared_turn_resolution_asks_the_ability_controller_for_singular_active_player() {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Charlie".into(),
                "Diana".into(),
            ],
            20,
        );
        let [alice, bob, charlie, diana] = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
            PlayerId::from_index(3),
        ];
        game.set_teams(vec![vec![alice, bob], vec![charlie, diana]])
            .expect("teams");
        game.enable_shared_team_turns().expect("shared turns");
        let source = game.new_object_id();
        let program =
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life_player(
                1,
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Active),
            )]);
        let mut dm = MatchingOptionDecisionMaker::new("Alice");
        let mut ctx = ExecutionContext::new(source, charlie, &mut dm);

        execute_resolution_program(&mut game, &mut ctx, charlie, source, &program, None, &[])
            .expect("resolution");

        assert_eq!(game.player(alice).expect("Alice").life, 21);
        assert_eq!(game.player(bob).expect("Bob").life, 20);
    }

    #[test]
    fn shared_combat_resolution_selects_singular_attacking_and_defending_players() {
        let mut game = GameState::new(
            vec![
                "Alice".into(),
                "Bob".into(),
                "Charlie".into(),
                "Diana".into(),
            ],
            20,
        );
        let [alice, bob, charlie, diana] = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
            PlayerId::from_index(3),
        ];
        game.set_teams(vec![vec![alice, bob], vec![charlie, diana]])
            .expect("teams");
        game.enable_shared_team_turns().expect("shared turns");
        let source = game.new_object_id();

        let creature_card = CardBuilder::new(CardId::from_raw(805_010), "Team Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let charlie_creature =
            game.create_object_from_card(&creature_card, charlie, Zone::Battlefield);
        let diana_creature = game.create_object_from_card(&creature_card, diana, Zone::Battlefield);
        let target_spec = crate::target::ChooseSpec::Object(
            crate::target::ObjectFilter::creature()
                .controlled_by(crate::target::PlayerFilter::Defending),
        );
        let view = crate::derived_view::DerivedGameView::new(&game);
        let legal_targets =
            crate::targeting::compute_legal_targets_with_tagged_objects_combat_context_with_view(
                &game,
                &target_spec,
                alice,
                Some(source),
                None,
                None,
                Some((charlie, alice)),
                &view,
            );
        assert!(legal_targets.contains(&crate::game_state::Target::Object(charlie_creature)));
        assert!(legal_targets.contains(&crate::game_state::Target::Object(diana_creature)));

        let attacking_program =
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life_player(
                1,
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Attacking),
            )]);
        let mut attacking_dm = MatchingOptionDecisionMaker::new("Bob");
        let mut attacking_ctx =
            ExecutionContext::new(source, charlie, &mut attacking_dm).with_attacking_player(alice);
        execute_resolution_program(
            &mut game,
            &mut attacking_ctx,
            charlie,
            source,
            &attacking_program,
            None,
            &[],
        )
        .expect("attacking-player resolution");

        let defending_program =
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::gain_life_player(
                1,
                crate::target::ChooseSpec::Player(crate::target::PlayerFilter::Defending),
            )]);
        let mut defending_dm = MatchingOptionDecisionMaker::new("Diana");
        let mut defending_ctx =
            ExecutionContext::new(source, alice, &mut defending_dm).with_defending_player(charlie);
        execute_resolution_program(
            &mut game,
            &mut defending_ctx,
            alice,
            source,
            &defending_program,
            None,
            &[],
        )
        .expect("defending-player resolution");

        assert_eq!(game.player(alice).expect("Alice").life, 20);
        assert_eq!(game.player(bob).expect("Bob").life, 21);
        assert_eq!(game.player(charlie).expect("Charlie").life, 20);
        assert_eq!(game.player(diana).expect("Diana").life, 21);
    }

    #[test]
    fn stack_resolution_passes_full_target_scope_to_modal_effects() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(91_000), "Modal Test")
            .card_types(vec![CardType::Instant])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
        let bounced_card = CardBuilder::new(CardId::from_raw(91_001), "Bounced")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 4))
            .build();
        let bounced = game.create_object_from_card(&bounced_card, bob, Zone::Stack);
        let damaged_card = CardBuilder::new(CardId::from_raw(91_002), "Damaged")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let damaged = game.create_object_from_card(&damaged_card, bob, Zone::Battlefield);

        let return_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object({
            let mut filter = crate::filter::ObjectFilter::default();
            filter.any_of = vec![
                crate::filter::ObjectFilter {
                    zone: Some(Zone::Stack),
                    ..Default::default()
                },
                crate::filter::ObjectFilter::creature(),
            ];
            filter
        }));
        let damage_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object({
            let mut filter = crate::filter::ObjectFilter::default();
            filter.zone = Some(Zone::Battlefield);
            filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
            filter
        }));
        let program = crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::ChooseModeEffect::choose_exactly(
                2,
                vec![
                    crate::effect::EffectMode::new(
                        "Return target spell or creature",
                        vec![Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                            return_spec.clone(),
                        ))],
                    ),
                    crate::effect::EffectMode::new(
                        "Deal damage to target creature or planeswalker",
                        vec![Effect::deal_damage(2, damage_spec.clone()).tag("damaged_0")],
                    ),
                ],
            ),
        )]);
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_chosen_modes(Some(vec![0, 1]))
            .with_targets(vec![
                crate::effects::ResolvedTarget::Object(bounced),
                crate::effects::ResolvedTarget::Object(damaged),
            ])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: return_spec,
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: damage_spec,
                    range: 1..2,
                },
            ]);
        let assignments = ctx.target_assignments.clone();

        execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            source,
            &program,
            Some(&[0, 1]),
            &assignments,
        )
        .expect("modal program should resolve");

        assert_eq!(game.damage_on(damaged), 2);
    }

    #[test]
    fn stack_resolution_preserves_explicit_player_target_for_filtered_continuous_effect() {
        let mut game = GameState::new(
            vec![
                "Alice".to_string(),
                "Bob".to_string(),
                "Charlie".to_string(),
            ],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let charlie = PlayerId::from_index(2);

        let source_card = CardBuilder::new(CardId::from_raw(91_005), "Control Test")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let bob_creature = create_creature(&mut game, "Bob Creature", bob, 2, 2);
        let bob_second_creature = create_creature(&mut game, "Bob Creature Two", bob, 2, 2);
        let charlie_creature = create_creature(&mut game, "Charlie Creature", charlie, 2, 2);

        let mut filter = crate::filter::ObjectFilter::creature();
        filter.controller = Some(crate::target::PlayerFilter::target_player());
        let spec = crate::target::ChooseSpec::Object(filter);
        let program = crate::resolution::ResolutionProgram::from_effects(vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(
                crate::target::ChooseSpec::target_player(),
            )),
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                spec,
                crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController,
                crate::effect::Until::Forever,
            )),
        ]);
        let target_spec = crate::target::ChooseSpec::target_player();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: target_spec,
                range: 0..1,
            }]);
        let assignments = ctx.target_assignments.clone();

        execute_resolution_program(
            &mut game,
            &mut ctx,
            alice,
            source,
            &program,
            None,
            &assignments,
        )
        .expect("continuous effect should resolve");

        assert_eq!(game.current_controller(bob_creature), Some(alice));
        assert_eq!(game.current_controller(bob_second_creature), Some(alice));
        assert_eq!(game.current_controller(charlie_creature), Some(charlie));
    }

    #[test]
    fn stack_entry_validation_preserves_modal_target_assignments() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(91_010), "Modal Test")
            .card_types(vec![CardType::Instant])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
        let bounced_card = CardBuilder::new(CardId::from_raw(91_011), "Bounced")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 4))
            .build();
        let bounced = game.create_object_from_card(&bounced_card, bob, Zone::Stack);
        let damaged_card = CardBuilder::new(CardId::from_raw(91_012), "Damaged")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let damaged = game.create_object_from_card(&damaged_card, bob, Zone::Battlefield);

        let return_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object({
            let mut filter = crate::filter::ObjectFilter::default();
            filter.any_of = vec![
                crate::filter::ObjectFilter {
                    zone: Some(Zone::Stack),
                    ..Default::default()
                },
                crate::filter::ObjectFilter::creature(),
            ];
            filter
        }));
        let damage_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object({
            let mut filter = crate::filter::ObjectFilter::default();
            filter.zone = Some(Zone::Battlefield);
            filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
            filter
        }));
        let program = crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::ChooseModeEffect::choose_exactly(
                2,
                vec![
                    crate::effect::EffectMode::new(
                        "Return target spell or creature",
                        vec![Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                            return_spec.clone(),
                        ))],
                    ),
                    crate::effect::EffectMode::new(
                        "Deal damage to target creature or planeswalker",
                        vec![Effect::deal_damage(2, damage_spec.clone()).tag("damaged_0")],
                    ),
                ],
            ),
        )]);
        game.object_mut(source).expect("source").spell_effect = Some(program.into());

        let mut entry = StackEntry::new(source, alice)
            .with_chosen_modes(Some(vec![0, 1]))
            .with_targets(vec![
                crate::game_state::Target::Object(bounced),
                crate::game_state::Target::Object(damaged),
            ])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: return_spec,
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: damage_spec,
                    range: 1..2,
                },
            ]);
        entry.provenance = game.provenance_graph_mut().alloc_root(
            crate::provenance::ProvenanceNodeKind::EffectExecution {
                source,
                controller: alice,
            },
        );
        game.push_to_stack(entry);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with(&mut game, &mut dm).expect("stack entry should resolve");

        assert_eq!(game.damage_on(damaged), 2);
    }

    #[test]
    fn modal_tagged_damage_then_replacement_exiles_dying_second_target() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(91_020), "Brutal Probe")
            .card_types(vec![CardType::Instant])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Stack);
        let bounced_card = CardBuilder::new(CardId::from_raw(91_021), "Bounced")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 4))
            .build();
        let bounced = game.create_object_from_card(&bounced_card, bob, Zone::Stack);
        let damaged_card = CardBuilder::new(CardId::from_raw(91_022), "Damaged")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let damaged = game.create_object_from_card(&damaged_card, bob, Zone::Battlefield);

        let return_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object({
            let mut filter = crate::filter::ObjectFilter::default();
            filter.any_of = vec![
                crate::filter::ObjectFilter {
                    zone: Some(Zone::Stack),
                    ..Default::default()
                },
                crate::filter::ObjectFilter::creature(),
            ];
            filter
        }));
        let damage_spec = crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object({
            let mut filter = crate::filter::ObjectFilter::default();
            filter.zone = Some(Zone::Battlefield);
            filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
            filter
        }));
        let program = crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
            crate::effects::ChooseModeEffect::choose_exactly(
                2,
                vec![
                    crate::effect::EffectMode::new(
                        "Return target spell or creature",
                        vec![
                            Effect::new(crate::effects::ReturnToHandEffect::with_spec(
                                return_spec.clone(),
                            ))
                            .tag("returned_0"),
                        ],
                    ),
                    crate::effect::EffectMode::new(
                        "Deal damage and exile if it would die",
                        vec![
                            Effect::deal_damage(2, damage_spec.clone()).tag("damaged_0"),
                            Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
                                crate::target::ChooseSpec::Tagged(crate::tag::TagKey::from(
                                    "damaged_0",
                                )),
                                Some(Zone::Battlefield),
                                Some(Zone::Graveyard),
                                Zone::Exile,
                                crate::effects::ReplacementApplyMode::OneShot,
                            )),
                        ],
                    ),
                ],
            ),
        )]);
        game.object_mut(source).expect("source").spell_effect = Some(program.into());

        let mut entry = StackEntry::new(source, alice)
            .with_chosen_modes(Some(vec![0, 1]))
            .with_targets(vec![
                crate::game_state::Target::Object(bounced),
                crate::game_state::Target::Object(damaged),
            ])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: return_spec,
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: damage_spec,
                    range: 1..2,
                },
            ]);
        entry.provenance = game.provenance_graph_mut().alloc_root(
            crate::provenance::ProvenanceNodeKind::EffectExecution {
                source,
                controller: alice,
            },
        );
        game.push_to_stack(entry);

        let mut dm = crate::decision::SelectFirstDecisionMaker;
        resolve_stack_entry_with(&mut game, &mut dm).expect("stack entry should resolve");

        assert_eq!(game.damage_on(damaged), 2);
        crate::rules::state_based::apply_state_based_actions(&mut game);
        assert!(
            game.exile
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|object| object.name == "Damaged")
        );
    }
}

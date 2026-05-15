use crate::alternative_cast::CastingMethod;
use crate::cost::OptionalCostsPaid;
use crate::effect::EffectOutcome;
use crate::effects::ExecutionContext;
use crate::events::other::LandPlayedEvent;
use crate::events::spells::SpellCastEvent;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::{GameState, StackEntry, Target, TargetAssignment};
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::triggers::TriggerEvent;
use crate::types::Subtype;
use crate::zone::Zone;

pub(super) fn register_effect_driven_spell_cast(
    game: &mut GameState,
    new_id: ObjectId,
    caster: PlayerId,
    from_zone: Zone,
    provenance: crate::provenance::ProvNodeId,
) -> TriggerEvent {
    if from_zone == Zone::Command {
        game.record_commander_cast_from_command_zone(new_id);
    }
    let event = if let Some(obj) = game.object(new_id) {
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(obj, game);
        SpellCastEvent::new_with_snapshot(new_id, caster, from_zone, snapshot)
    } else {
        SpellCastEvent::new(new_id, caster, from_zone)
    };
    TriggerEvent::new_with_provenance(event, provenance)
}

pub(super) fn queue_effect_driven_land_play(
    game: &mut GameState,
    ctx: &ExecutionContext,
    land_id: ObjectId,
    player: PlayerId,
    from_zone: Zone,
) {
    game.queue_trigger_event(
        ctx.provenance,
        TriggerEvent::new_with_provenance(
            LandPlayedEvent::new(land_id, player, from_zone),
            ctx.provenance,
        ),
    );

    if game
        .object(land_id)
        .is_some_and(|obj| obj.subtypes.contains(&Subtype::Saga))
        && let Some(event) = game.add_counters(land_id, CounterType::Lore, 1)
    {
        game.queue_trigger_event(ctx.provenance, event);
    }

    if let Some(player_data) = game.player_mut(player) {
        player_data.record_land_play();
    }
}

pub(super) fn with_spell_cast_event(
    outcome: EffectOutcome,
    game: &mut GameState,
    new_id: ObjectId,
    caster: PlayerId,
    from_zone: Zone,
    provenance: crate::provenance::ProvNodeId,
) -> EffectOutcome {
    let event = register_effect_driven_spell_cast(game, new_id, caster, from_zone, provenance);
    outcome.with_event(event)
}

#[derive(Debug, Clone)]
pub(super) struct EffectDrivenCastOption {
    pub object_id: ObjectId,
    pub from_zone: Zone,
    pub casting_method: CastingMethod,
    pub label: String,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct EffectDrivenCastResult {
    pub new_id: ObjectId,
    pub from_zone: Zone,
}

fn cast_filter_matches(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    view: &crate::object::Object,
    filter: &crate::target::ObjectFilter,
) -> bool {
    let filter_ctx = game
        .filter_context_for(caster, Some(source))
        .with_caster(Some(caster));
    filter.matches(view, &filter_ctx, game)
}

pub(super) fn effect_driven_cast_options_for_card(
    game: &GameState,
    caster: PlayerId,
    source: ObjectId,
    object_id: ObjectId,
    from_zone: Zone,
    filter: &crate::target::ObjectFilter,
) -> Vec<EffectDrivenCastOption> {
    let Some(object) = game.object(object_id) else {
        return Vec::new();
    };
    if object.is_land() || object.zone != from_zone {
        return Vec::new();
    }

    let mut options = Vec::new();
    if cast_filter_matches(game, caster, source, object, filter) {
        let casting_method = if from_zone == Zone::Hand {
            CastingMethod::Normal
        } else {
            CastingMethod::PlayFrom {
                source,
                zone: from_zone,
                use_alternative: None,
            }
        };
        options.push(EffectDrivenCastOption {
            object_id,
            from_zone,
            casting_method,
            label: format!("Cast {}", object.name),
        });
    }

    if let Some(other_half) = crate::decision::spell_view_for_split_other_half_cast(game, object)
        && cast_filter_matches(game, caster, source, &other_half, filter)
    {
        options.push(EffectDrivenCastOption {
            object_id,
            from_zone,
            casting_method: CastingMethod::SplitOtherHalf,
            label: format!("Cast {}", other_half.name),
        });
    }

    options
}

fn target_assignments_for_requirements(
    requirements: &[crate::decision::TargetRequirement],
    targets: &[Target],
) -> Option<Vec<TargetAssignment>> {
    let requirement_contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
            },
        )
        .collect::<Vec<_>>();
    let ranges = crate::targeting::assigned_target_ranges(&requirement_contexts, targets)?;
    Some(
        requirements
            .iter()
            .zip(ranges)
            .map(|(requirement, range)| TargetAssignment {
                spec: requirement.spec.clone(),
                range,
            })
            .collect(),
    )
}

fn choose_effect_driven_cast_targets(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    stack_id: ObjectId,
    caster: PlayerId,
    spell_name: String,
) -> Option<(Vec<Target>, Vec<TargetAssignment>)> {
    let requirements = game
        .object(stack_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            crate::game_loop::extract_target_requirements_from_program_with_modes(
                game,
                program,
                caster,
                Some(stack_id),
                None,
            )
        })
        .unwrap_or_default();
    let requirement_contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
            },
        )
        .collect::<Vec<_>>();
    let selected_targets = if requirement_contexts.is_empty() {
        Vec::new()
    } else {
        let targets_ctx = crate::decisions::context::TargetsContext::new(
            caster,
            stack_id,
            spell_name,
            requirement_contexts,
        );
        let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        crate::targeting::normalize_targets_for_requirements(&targets_ctx.requirements, proposed)?
    };
    let target_assignments = target_assignments_for_requirements(&requirements, &selected_targets)?;
    Some((selected_targets, target_assignments))
}

pub(super) fn cast_effect_driven_spell_without_paying(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    caster: PlayerId,
    option: &EffectDrivenCastOption,
) -> Result<Option<EffectDrivenCastResult>, crate::effects::ExecutionError> {
    let stack_id = crate::game_loop::propose_spell_cast(
        game,
        option.object_id,
        option.from_zone,
        caster,
        &option.casting_method,
    )
    .map_err(|err| crate::effects::ExecutionError::Impossible(err.to_string()))?;

    let (spell_name, stable_id, x_value) = {
        let Some(spell) = game.object(stack_id) else {
            return Ok(None);
        };
        let x_value = spell
            .mana_cost
            .as_ref()
            .and_then(|cost| if cost.has_x() { Some(0u32) } else { None });
        (spell.name.clone(), spell.stable_id, x_value)
    };
    if let Some(spell) = game.object_mut(stack_id) {
        spell.x_value = x_value;
    }

    let Some((targets, target_assignments)) =
        choose_effect_driven_cast_targets(game, ctx, stack_id, caster, spell_name.clone())
    else {
        return Ok(None);
    };

    let stack_entry = StackEntry {
        object_id: stack_id,
        controller: caster,
        provenance: ctx.provenance,
        targets,
        target_assignments,
        x_value,
        ability_effects: None,
        mana_usage_restrictions: Vec::new(),
        mana_source_chosen_creature_type: None,
        is_ability: false,
        casting_method: option.casting_method.clone(),
        optional_costs_paid: OptionalCostsPaid::default(),
        defending_player: None,
        chosen_player: None,
        chapter_ability_source: None,
        source_stable_id: Some(stable_id),
        source_snapshot: None,
        source_name: Some(spell_name),
        triggering_event: None,
        event_value_amount: None,
        trigger_identity: None,
        ability_index: None,
        intervening_if: None,
        keyword_payment_contributions: vec![],
        crew_contributors: vec![],
        saddle_contributors: vec![],
        chosen_modes: None,
        tagged_objects: std::collections::HashMap::new(),
    };
    game.push_to_stack(stack_entry);

    Ok(Some(EffectDrivenCastResult {
        new_id: stack_id,
        from_zone: option.from_zone,
    }))
}

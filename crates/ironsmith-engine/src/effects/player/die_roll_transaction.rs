#[path = "die_roll_replacements.rs"]
mod die_roll_replacements;
use crate::decision::FallbackStrategy;
use crate::decisions::{ask_choose_multiple, ask_choose_one, ask_may_choice};
use crate::effect::{EffectOutcome, OutcomeStatus};
use crate::effects::{
    CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError, PayManaEffect,
};
use crate::filter::PlayerFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::static_abilities::{DieRollResultAdjustmentSpec, StaticAbilityInstanceId};
use crate::target::ChooseSpec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedDieRoll {
    pub natural_result: u32,
    pub result: u32,
}

#[derive(Debug, Clone)]
struct AvailableDieRollModifier {
    source: ObjectId,
    ability: StaticAbilityInstanceId,
    display: String,
    spec: DieRollResultAdjustmentSpec,
}

fn draw_die_face(game: &mut GameState, sides: u32) -> Result<u32, ExecutionError> {
    if sides == 0 {
        return Err(ExecutionError::UnresolvableValue(
            "a die must have at least one side".into(),
        ));
    }
    if let Some(forced) = game.take_forced_die_roll() {
        return Ok(forced.clamp(1, sides));
    }
    let mut faces = Vec::new();
    faces.try_reserve_exact(sides as usize).map_err(|_| {
        ExecutionError::ResourceAllocationFailed {
            resource: "die faces",
            requested: sides as usize,
        }
    })?;
    faces.extend(1..=sides);
    game.shuffle_slice(&mut faces);
    faces.first().copied().ok_or_else(|| {
        ExecutionError::UnresolvableValue("a die must have at least one side".into())
    })
}

fn available_modifiers(
    game: &GameState,
    player: PlayerId,
    reroll: bool,
) -> Result<Vec<AvailableDieRollModifier>, ExecutionError> {
    let checked = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let game = &checked;
    Ok(game
        .battlefield
        .iter()
        .flat_map(|source| {
            let Some(object) = game
                .object(*source)
                .filter(|_| !game.is_phased_out(*source))
            else {
                return Vec::new();
            };
            let controller = game.controller_of(object);
            let filter_context = game.filter_context_for(controller, Some(*source));
            game.current_abilities(*source)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|ability| {
                    let crate::ability::AbilityKind::Static(static_ability) = ability.kind else {
                        return None;
                    };
                    let spec = static_ability.die_roll_result_adjustment_spec()?;
                    if spec.reroll != reroll
                        || spec.exchange_with_source_base_pt
                        || !spec.player.matches_player(player, &filter_context)
                        || (spec.once_each_turn
                            && game
                                .turn_store
                                .turn_history
                                .die_roll_modifier_used_this_turn(
                                    *source,
                                    static_ability.instance_id(),
                                ))
                        || (!spec.reroll && !game.can_pay_life(player, spec.life_cost))
                        || spec.source_counter_cost.is_some_and(|counter_type| {
                            game.counter_count(*source, counter_type) == 0
                        })
                    {
                        return None;
                    }
                    Some(AvailableDieRollModifier {
                        source: *source,
                        ability: static_ability.instance_id(),
                        display: static_ability.display(),
                        spec,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect())
}

fn choose_next_modifier(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    remaining: &[AvailableDieRollModifier],
    rolls: &[ResolvedDieRoll],
) -> Option<usize> {
    if remaining.len() == 1 {
        return Some(0);
    }
    let options = remaining
        .iter()
        .enumerate()
        .map(|(index, modifier)| {
            (
                format!(
                    "{} (current die results: {})",
                    modifier.display,
                    rolls
                        .iter()
                        .map(|roll| roll.result.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                index,
            )
        })
        .collect::<Vec<_>>();
    ask_choose_one(game, &mut ctx.decision_maker, player, ctx.source, &options)
}

fn pay_mana_cost(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    modifier: &AvailableDieRollModifier,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let Some(cost) = modifier.spec.mana_cost.clone() else {
        return Ok(CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved(),
        ));
    };
    let binding = (
        modifier.source,
        crate::snapshot::ObjectSnapshot::from_object_id(game, modifier.source),
    );
    crate::effects::composition::execute_with_source::with_source_binding(ctx, &binding, |ctx| {
        crate::effects::composition::execute_with_source::with_actor_binding(
            ctx,
            player,
            None,
            |ctx| {
                let outcome = PayManaEffect::new(cost, ChooseSpec::SpecificPlayer(player))
                    .execute_child_with_outputs(game, ctx)
                    .and_then(|mut outcome| {
                        if !ctx.decision_maker.awaiting_choice() {
                            crate::effects::runtime::capture_triggers_before_added_program(
                                game,
                                ctx,
                                None,
                                outcome.outcome.events.iter_mut(),
                            )?;
                        }
                        outcome.synchronize_observations();
                        Ok(outcome)
                    });
                outcome
            },
        )
    })
}

fn mark_used(game: &mut GameState, modifier: &AvailableDieRollModifier) {
    if modifier.spec.once_each_turn {
        game.turn_store
            .turn_history
            .record_die_roll_result_adjustment(modifier.source, modifier.ability);
    }
}

fn apply_reroll_modifiers(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    sides: u32,
    rolls: &mut [ResolvedDieRoll],
    payments: &mut Vec<CompletedEffectOutputs>,
) -> Result<bool, ExecutionError> {
    let mut remaining = available_modifiers(game, player, true)?;
    while !remaining.is_empty() {
        let Some(index) = choose_next_modifier(game, ctx, player, &remaining, rolls) else {
            return Ok(false);
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        let modifier = remaining.remove(index);
        let should_apply = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            player,
            modifier.source,
            format!(
                "{} (rolled {})",
                modifier.display,
                rolls
                    .iter()
                    .map(|roll| roll.result.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FallbackStrategy::Decline,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        if !should_apply {
            continue;
        }
        let payment = pay_mana_cost(game, ctx, player, &modifier)?;
        let paid = payment.outcome.status != OutcomeStatus::Impossible;
        payments.push(payment);
        if !paid {
            continue;
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }

        let selected = if rolls.len() == 1 {
            vec![0]
        } else {
            let options = rolls
                .iter()
                .enumerate()
                .map(|(index, roll)| {
                    (
                        format!("Die {} (rolled {})", index + 1, roll.natural_result),
                        index,
                    )
                })
                .collect::<Vec<_>>();
            ask_choose_multiple(
                game,
                &mut ctx.decision_maker,
                player,
                modifier.source,
                &options,
                1,
                rolls.len(),
            )
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        for index in selected {
            if let Some(roll) = rolls.get_mut(index) {
                let face = draw_die_face(game, sides)?;
                roll.natural_result = face;
                roll.result = face;
            }
        }
        mark_used(game, &modifier);
    }
    Ok(true)
}

fn apply_numerical_modifiers(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    roll: &mut ResolvedDieRoll,
    payments: &mut Vec<CompletedEffectOutputs>,
    authored_modifier: Option<&ironsmith_core::effect::DieResultModifier>,
) -> Result<bool, ExecutionError> {
    let mut remaining = available_modifiers(game, player, false)?;
    let mut authored_modifier = authored_modifier;
    while !remaining.is_empty() || authored_modifier.is_some() {
        // The mandatory arithmetic printed on the rolling instruction is a
        // numerical modifier too (CR 706.2b). The roller chooses its order
        // relative to external numerical modifiers, after all rerolls.
        let selected = if authored_modifier.is_some() {
            if remaining.is_empty() {
                Some(0)
            } else {
                let mut options = remaining
                    .iter()
                    .enumerate()
                    .map(|(index, modifier)| {
                        (
                            format!("{} (current die result: {})", modifier.display, roll.result),
                            index,
                        )
                    })
                    .collect::<Vec<_>>();
                options.push((
                    "Apply this instruction's die-result arithmetic".into(),
                    remaining.len(),
                ));
                ask_choose_one(game, &mut ctx.decision_maker, player, ctx.source, &options)
            }
        } else {
            choose_next_modifier(game, ctx, player, &remaining, std::slice::from_ref(roll))
        };
        let Some(index) = selected else {
            return Ok(false);
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        if index == remaining.len()
            && let Some(modifier) = authored_modifier.take()
        {
            let amount = crate::effects::helpers::resolve_value_wide(game, modifier.value(), ctx)?;
            let result = match modifier {
                ironsmith_core::effect::DieResultModifier::Add(_) => {
                    i128::from(roll.result) + i128::from(amount)
                }
                ironsmith_core::effect::DieResultModifier::Subtract(_) => {
                    i128::from(roll.result) - i128::from(amount)
                }
            };
            roll.result = bounded_die_result(result)?;
            continue;
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        let modifier = remaining.remove(index);
        let description = if let Some(counter_type) = modifier.spec.source_counter_cost {
            format!(
                "Die result {}: remove a {} counter to increase or decrease it by {}",
                roll.result,
                counter_type.description(),
                modifier.spec.amount
            )
        } else {
            format!(
                "Die result {}: pay {} life to increase or decrease it by {}",
                roll.result, modifier.spec.life_cost, modifier.spec.amount
            )
        };
        let should_apply = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            player,
            modifier.source,
            description,
            FallbackStrategy::Decline,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        if !should_apply {
            continue;
        }
        let options = [
            ("Increase".to_string(), true),
            ("Decrease".to_string(), false),
        ];
        let Some(increase) = ask_choose_one(
            game,
            &mut ctx.decision_maker,
            player,
            modifier.source,
            &options,
        ) else {
            return Ok(false);
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        if let Some(counter_type) = modifier.spec.source_counter_cost {
            // "you may remove a <kind> counter from this creature. If you do,
            // ...": the removal is the payment, observed like any removal.
            if game.counter_count(modifier.source, counter_type) == 0 {
                continue;
            }
            let removal = crate::effects::RemoveCountersEffect::new(
                counter_type,
                1,
                ChooseSpec::SpecificObject(modifier.source),
            );
            let binding = (
                modifier.source,
                crate::snapshot::ObjectSnapshot::from_object_id(game, modifier.source),
            );
            let mut payment = crate::effects::composition::execute_with_source::with_source_binding(
                ctx,
                &binding,
                |ctx| removal.execute_child_with_outputs(game, ctx),
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(false);
            }
            if payment.outcome.count_or_zero() == 0 {
                continue;
            }
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                ctx,
                None,
                payment.outcome.events.iter_mut(),
            )?;
            payment.synchronize_observations();
            payments.push(payment);
        }
        if modifier.spec.life_cost > 0 {
            let payment =
                game.pay_life_with_context_and_outputs(player, modifier.spec.life_cost, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(false);
            }
            let Some(mut payment) = payment else {
                continue;
            };
            crate::effects::runtime::capture_triggers_before_added_program(
                game,
                ctx,
                None,
                payment.outcome.events.iter_mut(),
            )?;
            payment.synchronize_observations();
            payments.push(payment);
        }
        roll.result = bounded_die_result(if increase {
            i128::from(roll.result) + i128::from(modifier.spec.amount)
        } else {
            i128::from(roll.result) - i128::from(modifier.spec.amount)
        })?;
        mark_used(game, &modifier);
    }
    Ok(true)
}

/// Sources that may swap one retained result of this roll with their own
/// base power or base toughness (Vedalken Squirrel-Whacker).
fn available_exchanges(
    game: &GameState,
    player: PlayerId,
    sides: u32,
) -> Result<Vec<AvailableDieRollModifier>, ExecutionError> {
    let checked = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let game = &checked;
    Ok(game
        .battlefield
        .iter()
        .flat_map(|source| {
            let Some(object) = game
                .object(*source)
                .filter(|_| !game.is_phased_out(*source))
            else {
                return Vec::new();
            };
            let controller = game.controller_of(object);
            let filter_context = game.filter_context_for(controller, Some(*source));
            game.current_abilities(*source)
                .unwrap_or_default()
                .into_iter()
                .filter_map(|ability| {
                    let crate::ability::AbilityKind::Static(static_ability) = ability.kind else {
                        return None;
                    };
                    let spec = static_ability.die_roll_result_adjustment_spec()?;
                    if !spec.exchange_with_source_base_pt
                        || spec.sides.is_some_and(|required| required != sides)
                        || !spec.player.matches_player(player, &filter_context)
                    {
                        return None;
                    }
                    Some(AvailableDieRollModifier {
                        source: *source,
                        ability: static_ability.instance_id(),
                        display: static_ability.display(),
                        spec,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect())
}

/// "you may exchange one result with this creature's base power or base
/// toughness": the chosen die takes the old base value and the creature's
/// base characteristic becomes that die's result (CR 613.4b).
fn apply_exchange_modifiers(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    sides: u32,
    rolls: &mut [ResolvedDieRoll],
    payments: &mut Vec<CompletedEffectOutputs>,
) -> Result<bool, ExecutionError> {
    for modifier in available_exchanges(game, player, sides)? {
        if rolls.is_empty() || !game.battlefield.contains(&modifier.source) {
            continue;
        }
        let should_apply = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            player,
            modifier.source,
            format!(
                "{} (rolled {})",
                modifier.display,
                rolls
                    .iter()
                    .map(|roll| roll.result.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            FallbackStrategy::Decline,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        if !should_apply {
            continue;
        }
        let index = if rolls.len() == 1 {
            0
        } else {
            let options = rolls
                .iter()
                .enumerate()
                .map(|(index, roll)| (format!("Die {} (result {})", index + 1, roll.result), index))
                .collect::<Vec<_>>();
            let Some(index) =
                ask_choose_one(game, &mut ctx.decision_maker, player, modifier.source, &options)
            else {
                return Ok(false);
            };
            index
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        let options = [
            ("Exchange with base power".to_string(), true),
            ("Exchange with base toughness".to_string(), false),
        ];
        let Some(power) =
            ask_choose_one(game, &mut ctx.decision_maker, player, modifier.source, &options)
        else {
            return Ok(false);
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        let Some(chars) = game.calculated_characteristics(modifier.source) else {
            continue;
        };
        let Some(base) = (if power { chars.base_power } else { chars.base_toughness }) else {
            continue;
        };
        let Some(roll) = rolls.get_mut(index) else {
            continue;
        };
        let result = i32::try_from(roll.result).map_err(|_| ExecutionError::ResourceLimitExceeded {
            resource: "exchanged die result",
            requested: u128::from(roll.result),
            maximum: i32::MAX as u128,
        })?;
        let value = crate::effect::Value::Fixed(result);
        let modification = if power {
            crate::continuous::Modification::SetPower {
                value,
                sublayer: crate::continuous::PtSublayer::Setting,
            }
        } else {
            crate::continuous::Modification::SetToughness {
                value,
                sublayer: crate::continuous::PtSublayer::Setting,
            }
        };
        let exchange = crate::effects::ApplyContinuousEffect::with_spec(
            ChooseSpec::SpecificObject(modifier.source),
            modification,
            crate::effect::Until::Forever,
        );
        let binding = (
            modifier.source,
            crate::snapshot::ObjectSnapshot::from_object_id(game, modifier.source),
        );
        let outputs = crate::effects::composition::execute_with_source::with_source_binding(
            ctx,
            &binding,
            |ctx| exchange.execute_child_with_outputs(game, ctx),
        )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        payments.push(outputs);
        // CR 107.1b: a negative base value gives the die a result of zero.
        roll.result = u32::try_from(base.max(0)).unwrap_or(0);
    }
    Ok(true)
}

fn bounded_die_result(result: i128) -> Result<u32, ExecutionError> {
    // CR 107.1b: a negative result of an effect is zero; an unrepresentable
    // positive result is an execution-resource error, never saturation.
    u32::try_from(result.max(0)).map_err(|_| ExecutionError::ResourceLimitExceeded {
        resource: "modified die result",
        requested: result.max(0) as u128,
        maximum: u32::MAX as u128,
    })
}

pub(crate) fn roll_dice_with_modifiers(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    count: u32,
    sides: u32,
) -> Result<Option<DieRollTransaction>, ExecutionError> {
    roll_dice_with_authored_modifier(game, ctx, player, count, sides, None)
}

pub(crate) fn roll_dice_with_authored_modifier(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    count: u32,
    sides: u32,
    authored_modifier: Option<&ironsmith_core::effect::DieResultModifier>,
) -> Result<Option<DieRollTransaction>, ExecutionError> {
    let Some(mut rolls) =
        die_roll_replacements::roll_replacement_batch(game, ctx, player, count, sides)?
    else {
        return Ok(None);
    };
    let mut payments = Vec::new();
    if !apply_reroll_modifiers(game, ctx, player, sides, &mut rolls, &mut payments)? {
        return Ok(None);
    }
    for roll in &mut rolls {
        if !apply_numerical_modifiers(game, ctx, player, roll, &mut payments, authored_modifier)? {
            return Ok(None);
        }
    }
    if !apply_exchange_modifiers(game, ctx, player, sides, &mut rolls, &mut payments)? {
        return Ok(None);
    }
    Ok(Some(DieRollTransaction { rolls, payments }))
}

/// Retained dice and complete modifier payments are separate observations.
pub(crate) struct DieRollTransaction {
    pub rolls: Vec<ResolvedDieRoll>,
    pub payments: Vec<CompletedEffectOutputs>,
}

impl DieRollTransaction {
    /// Complete the retained dice after caller-owned result selection, keeping
    /// each modifier payment and the native completion observation as children.
    pub(crate) fn complete_with_outputs(
        self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        player: PlayerId,
        sides: u32,
        displayed_result: u32,
        mode: DieRollCompletion,
        primary: EffectOutcome,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let completion = complete_die_rolls(
            game,
            ctx,
            player,
            sides,
            &self.rolls,
            displayed_result,
            mode,
        )?;
        Ok(CompletedEffectOutputs::from_children(
            self.payments
                .into_iter()
                .chain([CompletedEffectOutputs::aggregate_only(completion)]),
            |children| EffectOutcome::aggregate_with_primary_result(primary, children),
        ))
    }
}

#[derive(Clone, Copy)]
pub(crate) enum DieRollCompletion {
    Single,
    Simultaneous,
    AttractionVisit,
}

/// Commit die history and completion observations only after result selection.
/// Ignored replacement dice have already been removed from this retained set.
pub(crate) fn complete_die_rolls(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    sides: u32,
    rolls: &[ResolvedDieRoll],
    displayed_result: u32,
    mode: DieRollCompletion,
) -> Result<EffectOutcome, ExecutionError> {
    let ordinal = game.turn_store.turn_history.record_completed_die_rolls(
        player,
        &rolls.iter().map(|roll| roll.result).collect::<Vec<_>>(),
        false,
    )?;
    game.mark_continuous_state_dirty();
    let simultaneous = matches!(mode, DieRollCompletion::Simultaneous);
    let attraction_visit = matches!(mode, DieRollCompletion::AttractionVisit);
    game.record_ui_effect_event(
        if attraction_visit {
            "attraction_visit_roll"
        } else {
            "die_roll"
        },
        Some(player),
        None,
        Vec::new(),
        Some(i64::from(displayed_result)),
        Some(format!("d{sides}")),
    );
    let batch = simultaneous.then(|| {
        game.alloc_child_event_provenance(ctx.provenance, crate::events::EventKind::DieRolled)
    });
    let events = rolls
        .iter()
        .enumerate()
        .map(|(index, roll)| {
            // The instruction is the causal parent. Every retained physical
            // roll has its own completion identity, including single dice.
            let provenance = game
                .alloc_child_event_provenance(ctx.provenance, crate::events::EventKind::DieRolled);
            let observation = crate::events::other::DieRolledEvent::new_with_natural_result(
                player,
                ctx.source,
                roll.natural_result,
                roll.result,
                sides,
            )
            .with_turn_ordinal(ordinal + index as u32);
            let observation = if attraction_visit {
                observation.for_attraction_visit()
            } else {
                observation
            };
            let event = crate::triggers::TriggerEvent::new_with_provenance(observation, provenance);
            if let Some(batch) = batch {
                event.with_simultaneous_batch(batch)
            } else {
                event
            }
        })
        .collect::<Vec<_>>();
    Ok(EffectOutcome::resolved().with_events(events))
}

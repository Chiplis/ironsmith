#[path = "die_roll_replacements.rs"]
mod die_roll_replacements;
use crate::decision::FallbackStrategy;
use crate::decisions::{ask_choose_multiple, ask_choose_one, ask_may_choice};
use crate::effect::OutcomeStatus;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, PayManaEffect};
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

fn draw_die_face(game: &mut GameState, sides: u32) -> u32 {
    if let Some(forced) = game.take_forced_die_roll() {
        return forced.clamp(1, sides);
    }
    let mut faces: Vec<u32> = (1..=sides).collect();
    game.shuffle_slice(&mut faces);
    faces[0]
}

fn available_modifiers(
    game: &GameState,
    player: PlayerId,
    reroll: bool,
) -> Vec<AvailableDieRollModifier> {
    game.battlefield
        .iter()
        .flat_map(|source| {
            let Some(object) = game.object(*source).filter(|_| !game.is_phased_out(*source)) else {
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
        .collect()
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
        .map(|(index, modifier)| (
            format!("{} (current die results: {})", modifier.display,
                rolls.iter().map(|roll| roll.result.to_string()).collect::<Vec<_>>().join(", ")),
            index,
        ))
        .collect::<Vec<_>>();
    ask_choose_one(game, &mut ctx.decision_maker, player, ctx.source, &options)
}

fn pay_mana_cost(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    modifier: &AvailableDieRollModifier,
) -> Result<bool, ExecutionError> {
    let Some(cost) = modifier.spec.mana_cost.clone() else {
        return Ok(true);
    };
    let original_source = ctx.source;
    let original_controller = ctx.controller;
    ctx.source = modifier.source;
    ctx.controller = player;
    let outcome = PayManaEffect::new(cost, ChooseSpec::SpecificPlayer(player)).execute(game, ctx);
    ctx.source = original_source;
    ctx.controller = original_controller;
    Ok(outcome?.status != OutcomeStatus::Impossible)
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
) -> Result<bool, ExecutionError> {
    let mut remaining = available_modifiers(game, player, true);
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
            format!("{} (rolled {})", modifier.display,
                rolls.iter().map(|roll| roll.result.to_string()).collect::<Vec<_>>().join(", ")),
            FallbackStrategy::Decline,
        );
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        if !should_apply || !pay_mana_cost(game, ctx, player, &modifier)? {
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
                let face = draw_die_face(game, sides);
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
) -> Result<bool, ExecutionError> {
    let mut remaining = available_modifiers(game, player, false);
    while !remaining.is_empty() {
        let Some(index) = choose_next_modifier(game, ctx, player, &remaining, std::slice::from_ref(roll)) else {
            return Ok(false);
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(false);
        }
        let modifier = remaining.remove(index);
        let description = format!(
            "Die result {}: pay {} life to increase or decrease it by {}",
            roll.result, modifier.spec.life_cost, modifier.spec.amount
        );
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
        if modifier.spec.life_cost > 0 {
            let paid = game.pay_life_with_context(player, modifier.spec.life_cost, ctx)?.is_some();
            if ctx.decision_maker.awaiting_choice() { return Ok(false); }
            if !paid { continue; }
        }
        roll.result = if increase {
            roll.result.saturating_add(modifier.spec.amount)
        } else {
            roll.result.saturating_sub(modifier.spec.amount)
        };
        mark_used(game, &modifier);
    }
    Ok(true)
}

pub(crate) fn roll_dice_with_modifiers(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    count: u32,
    sides: u32,
) -> Result<Option<Vec<ResolvedDieRoll>>, ExecutionError> {
    let Some(mut rolls) = die_roll_replacements::roll_replacement_batch(game, ctx, player, count, sides)? else { return Ok(None); };
    if !apply_reroll_modifiers(game, ctx, player, sides, &mut rolls)? {
        return Ok(None);
    }
    for roll in &mut rolls {
        if !apply_numerical_modifiers(game, ctx, player, roll)? {
            return Ok(None);
        }
    }
    Ok(Some(rolls))
}

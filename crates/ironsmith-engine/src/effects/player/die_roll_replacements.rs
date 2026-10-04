//! Pre-roll replacements and their ignored results are one numerical roll
//! transaction. They never publish an ignored die as a completed occurrence.
use super::*;

#[derive(Debug, Clone)]
struct ExtraDiceReplacement {
    source: ObjectId,
    ability: StaticAbilityInstanceId,
    occurrence: usize,
    additional: u32,
    display: String,
}

pub(super) fn roll_replacement_batch(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    count: u32,
    sides: u32,
) -> Result<Option<Vec<ResolvedDieRoll>>, ExecutionError> {
    if count == 0 || sides == 0 {
        return Ok(Some(Vec::new()));
    }
    // Ignored dice never enter history; capacity concerns the retained count.
    game.turn_store
        .turn_history
        .check_completed_die_roll_capacity(player, count as usize)?;
    let checked = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    let mut remaining = Vec::new();
    for &source in &checked.battlefield {
        if checked.is_phased_out(source) {
            continue;
        }
        let Some(controller) = checked.current_controller(source) else {
            continue;
        };
        let filter_context = checked.filter_context_for(controller, Some(source));
        for (occurrence, ability) in checked
            .current_abilities(source)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            let crate::ability::AbilityKind::Static(ability) = ability.kind else {
                continue;
            };
            let Some(model) = ability.compiled_model() else {
                continue;
            };
            let ironsmith_core::StaticAbilityPayload::ExtraDieIgnoreLowest {
                player: filter,
                additional,
            } = &model.payload
            else {
                continue;
            };
            if *additional == 0 || !filter.matches_player(player, &filter_context) {
                continue;
            }
            remaining.push(ExtraDiceReplacement {
                source,
                ability: ability.instance_id(),
                occurrence,
                additional: *additional,
                display: ability.display(),
            });
        }
    }
    // Each source/ability occurrence replaces this original event once (CR614.5).
    // Distinct entries remain distinct even when cloned from one shared model.
    // The chooser is the affected roller, independent of the effect controller.
    // The chosen order also owns the later ignored-roll instructions.
    let mut applied = Vec::<ExtraDiceReplacement>::new();
    let mut total = count;
    while !remaining.is_empty() {
        let index = if remaining.len() == 1 {
            0
        } else {
            let options = remaining
                .iter()
                .enumerate()
                .map(|(i, replacement)| {
                    (
                        format!("Apply {} from {:?}", replacement.display, replacement.source),
                        i,
                    )
                })
                .collect::<Vec<_>>();
            let selected =
                ask_choose_one(game, &mut ctx.decision_maker, player, ctx.source, &options);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(None);
            }
            selected.ok_or_else(|| {
                ExecutionError::UnresolvableValue("a die-roll replacement must be selected".into())
            })?
        };
        let replacement = remaining.remove(index);
        total = total.checked_add(replacement.additional).ok_or(
            ExecutionError::ResourceLimitExceeded {
                resource: "physical replacement dice",
                requested: u128::from(total) + u128::from(replacement.additional),
                maximum: u32::MAX as u128,
            },
        )?;
        remaining.retain(|other| {
            (other.source, other.ability, other.occurrence)
                != (
                    replacement.source,
                    replacement.ability,
                    replacement.occurrence,
                )
        });
        applied.push(replacement);
    }
    let mut rolls = Vec::new();
    rolls.try_reserve_exact(total as usize).map_err(|_| {
        ExecutionError::ResourceAllocationFailed {
            resource: "physical replacement dice",
            requested: total as usize,
        }
    })?;
    for _ in 0..total {
        let face = draw_die_face(game, sides);
        rolls.push(ResolvedDieRoll {
            natural_result: face,
            result: face,
        });
    }
    // Nested replacements finish inside-out. Ignored rolls are removed before
    // rerolls or numerical modifiers: no effects apply to them (CR706.6).
    for replacement in applied.iter().rev() {
        for _ in 0..replacement.additional {
            let minimum = rolls
                .iter()
                .map(|roll| roll.natural_result)
                .min()
                .expect("extra dice preserve original count");
            let lowest = rolls
                .iter()
                .enumerate()
                .filter_map(|(index, roll)| (roll.natural_result == minimum).then_some(index))
                .collect::<Vec<_>>();
            let ignored = if lowest.len() == 1 {
                lowest[0]
            } else {
                let options = lowest
                    .into_iter()
                    .map(|index| {
                        (
                            format!("Ignore die {} (rolled {})", index + 1, minimum),
                            index,
                        )
                    })
                    .collect::<Vec<_>>();
                let selected = ask_choose_one(
                    game,
                    &mut ctx.decision_maker,
                    player,
                    replacement.source,
                    &options,
                );
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(None);
                }
                selected.ok_or_else(|| {
                    ExecutionError::UnresolvableValue("a tied lowest die must be selected".into())
                })?
            };
            rolls.remove(ignored);
        }
    }
    debug_assert_eq!(rolls.len(), count as usize);
    Ok(Some(rolls))
}

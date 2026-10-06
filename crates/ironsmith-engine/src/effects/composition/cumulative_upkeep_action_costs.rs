//! Bounded action costs for the cumulative-upkeep owner. Choices belong to
//! installments (CR 702.24a); the resulting actions do not prove payment by
//! their output count (CR 118.11).

use crate::decisions::context::{SelectObjectsContext, SelectableObject};
use crate::effect::{Effect, EffectOutcome, OutcomeStatus};
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError, execute_effect_with_outputs};
use crate::filter::ObjectFilterExt;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

pub(super) enum ActionCost<'a> {
    Mana(&'a Effect),
    Draw(&'a Effect, &'a crate::effects::DrawCardsEffect),
    MoveGroup(&'a crate::effects::ChooseObjectsEffect, &'a crate::effects::MoveToZoneEffect),
}

fn payload(mut effect: &Effect) -> &Effect {
    while let Some(child) = effect.transparent_child_effect() { effect = child; }
    effect
}

impl<'a> ActionCost<'a> {
    pub(super) fn read(effects: &'a [Effect]) -> Option<Self> {
        match effects {
            [effect] => {
                if payload(effect).downcast_ref::<crate::effects::AddManaEffect>()
                    .is_some_and(|add| add.player == PlayerFilter::You)
                { return Some(Self::Mana(effect)); }
                let draw = payload(effect).downcast_ref::<crate::effects::DrawCardsEffect>()?;
                (draw.player == PlayerFilter::You).then_some(Self::Draw(effect, draw))
            }
            [choice, movement] => {
                let choice = choice.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
                let movement = movement.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
                let expected_choice = crate::effects::ChooseObjectsEffect::new(
                    crate::target::ObjectFilter::default().in_zone(Zone::Graveyard).single_graveyard(),
                    choice.count, PlayerFilter::You, choice.tag.clone(),
                );
                let mut expected_movement = crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Tagged(choice.tag.clone()), Zone::Library, false,
                );
                expected_movement.library_order = movement.library_order.clone();
                // Public-zone choices with fixed installment cardinality. Other
                // cost programs retain their existing owner, not an inferred
                // approximation of this bounded protocol.
                (choice == &expected_choice && movement == &expected_movement
                    && matches!(movement.library_order, None | Some(crate::effects::LibraryPlacementOrder::Owners))
                    && !choice.count.dynamic_x && !choice.count.up_to_x
                    && choice.count.min > 0 && choice.count.max == Some(choice.count.min)
                )
                    .then_some(Self::MoveGroup(choice, movement))
            }
            _ => None,
        }
    }

    fn groups(
        choice: &crate::effects::ChooseObjectsEffect,
        game: &GameState,
        ctx: &ExecutionContext,
        reserved: &[ObjectId],
    ) -> Vec<Vec<ObjectId>> {
        let filter_ctx = ctx.filter_context(game);
        game.players.iter().map(|player| {
            player.graveyard.iter().copied().filter(|id| {
                !reserved.contains(id) && game.object(*id).is_some_and(|object| {
                    choice.filter.matches(object, &filter_ctx, game)
                })
            }).collect::<Vec<_>>()
        }).filter(|ids| ids.len() >= choice.count.min).collect()
    }

    pub(super) fn can_pay(
        &self, game: &GameState, ctx: &ExecutionContext, repetitions: usize,
    ) -> Result<bool, ExecutionError> {
        if repetitions == 0 { return Ok(true); }
        match self {
            Self::Mana(_) => Ok(true),
            Self::Draw(_, draw) => {
                let count = crate::effects::helpers::resolve_value(game, &draw.count, ctx)?.max(0) as usize;
                let total = count.checked_mul(repetitions).ok_or_else(|| ExecutionError::IncompleteEvidence(
                    "cumulative draw cost exceeds the supported count range".into(),
                ))?;
                // CR 121.2b/121.3: a prohibition is different from a draw
                // replacement. An empty library is not a prohibition.
                Ok(total == 0 || (game.can_draw(ctx.controller)
                    && (game.can_draw_extra_cards(ctx.controller)
                        || (total == 1 && game.turn_store.turn_history.cards_drawn_by_player(ctx.controller) == 0))))
            }
            Self::MoveGroup(choice, _) => Ok(Self::groups(choice, game, ctx, &[])
                .iter().map(|group| group.len() / choice.count.min).sum::<usize>() >= repetitions),
        }
    }

    pub(super) fn pay(
        &self, game: &mut GameState, ctx: &mut ExecutionContext, repetitions: usize,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        match self {
            Self::Mana(effect) | Self::Draw(effect, _) => {
                let mut outcomes = Vec::new();
                for _ in 0..repetitions {
                    let mut outcome = execute_effect_with_outputs(game, effect, ctx)?;
                    if ctx.decision_maker.awaiting_choice() { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
                    crate::effects::capture_triggers_before_added_program(game, ctx, None, outcome.outcome.events.iter_mut())?;
                    // A payment action has been attempted. Replacement/prevention
                    // receipts remain intact, while the enclosing payment succeeds.
                    outcome.outcome.status = OutcomeStatus::Succeeded;
                    outcomes.push(outcome);
                }
                Ok(CompletedEffectOutputs::from_children(outcomes, EffectOutcome::aggregate_summing_counts))
            }
            Self::MoveGroup(choice, movement) => {
                let mut installments = Vec::new();
                let mut reserved = Vec::new();
                for age in 0..repetitions {
                    let groups = Self::groups(choice, game, ctx, &reserved);
                    let candidates = groups.iter().flatten().map(|id| {
                        let object = game.object(*id).expect("current graveyard candidate");
                        SelectableObject::new(*id, object.name.to_string())
                    }).collect();
                    let decision = SelectObjectsContext::new(
                        ctx.controller, Some(ctx.source),
                        format!("Choose {} cards from one graveyard for age counter {} of {}", choice.count.min, age + 1, repetitions),
                        candidates, choice.count.min, Some(choice.count.min),
                    ).require_explicit_choice().with_relation_filter(choice.filter.clone());
                    let selected = ctx.decision_maker.decide_objects(game, &decision);
                    if ctx.decision_maker.awaiting_choice() { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
                    let unique = selected.iter().copied().collect::<std::collections::HashSet<_>>();
                    if selected.len() != choice.count.min || unique.len() != selected.len()
                        || !groups.iter().any(|group| selected.iter().all(|id| group.contains(id)))
                    {
                        return Err(ExecutionError::IncompleteEvidence(
                            "cumulative upkeep requires a complete, disjoint single-graveyard choice for every age counter".into(),
                        ));
                    }
                    reserved.extend(selected.iter().copied());
                    installments.push(selected);
                }
                if repetitions == 0 { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
                // Every separately constrained installment is selected before
                // the original movement. The native multi-object action owns
                // simultaneous proposals, owner ordering, replacements and
                // trigger capture before any added program.
                let snapshots = installments.into_iter().flatten().map(|id| {
                    game.object(id).map(|object| game.cached_object_snapshot_with_calculated_characteristics(object))
                        .ok_or_else(|| ExecutionError::IncompleteEvidence("selected upkeep object disappeared before payment".into()))
                }).collect::<Result<Vec<_>, _>>()?;
                ctx.set_tagged_objects(choice.tag.clone(), snapshots);
                let mut movement = (**movement).clone();
                movement.library_order = Some(crate::effects::LibraryPlacementOrder::Owners);
                let mut outcome = execute_effect_with_outputs(game, &Effect::new(movement), ctx)?;
                if ctx.decision_maker.awaiting_choice() { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
                crate::effects::capture_triggers_before_added_program(game, ctx, None, outcome.outcome.events.iter_mut())?;
                outcome.outcome.status = OutcomeStatus::Succeeded;
                Ok(outcome)
            }
        }
    }
}

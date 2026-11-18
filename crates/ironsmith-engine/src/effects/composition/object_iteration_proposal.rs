//! Captured children of a simultaneous object loop, in iteration order.

use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::{
    CompletedEffectOutputs, DamageActionBinding, ExecutionContext, ExecutionError,
    PaymentResourceClaim, SimultaneousEffectCommit, SimultaneousEffectProposal,
};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

#[derive(Debug)]
pub(super) struct ObjectIterationProposal {
    children: Vec<Box<dyn SimultaneousEffectProposal>>,
    // Tagged loops also project each iteration's outcomes onto its captured player.
    player_ranges: Option<Vec<(PlayerId, usize, usize)>>,
}

impl ObjectIterationProposal {
    pub(super) fn total(capacity: usize) -> Self {
        Self {
            children: Vec::with_capacity(capacity),
            player_ranges: None,
        }
    }

    pub(super) fn per_player(capacity: usize) -> Self {
        Self {
            player_ranges: Some(Vec::with_capacity(capacity)),
            ..Self::total(capacity)
        }
    }

    pub(super) fn prepare_children(
        &mut self,
        effects: &[Effect],
        game: &GameState,
        ctx: &mut ExecutionContext,
        object: ObjectId,
        player: PlayerId,
        capture_tags: impl Fn() -> Vec<(TagKey, Vec<ObjectSnapshot>)>,
    ) -> Result<(), ExecutionError> {
        let start = self.children.len();
        for effect in effects {
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
            let child = effect.prepare_simultaneous_player_action(game, ctx)?;
            self.children.push(super::scope_prepared_iteration(
                child,
                object,
                player,
                capture_tags(),
            ));
        }
        if let Some(ranges) = &mut self.player_ranges {
            ranges.push((player, start, self.children.len()));
        }
        Ok(())
    }

    fn prepare_phase(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        phase: impl Fn(
            &mut dyn SimultaneousEffectProposal,
            &mut GameState,
            &mut ExecutionContext,
        ) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        for child in &mut self.children {
            phase(child.as_mut(), game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
        }
        Ok(())
    }
}

fn project(
    outcomes: Vec<EffectOutcome>,
    ranges: Option<&[(PlayerId, usize, usize)]>,
) -> EffectOutcome {
    match ranges {
        Some(ranges) => summarize_tagged_iterations(outcomes, ranges),
        None => EffectOutcome::aggregate_summing_counts(outcomes),
    }
}

pub(super) fn correlated_player_count(outcomes: &[EffectOutcome]) -> i64 {
    let summary = EffectOutcome::aggregate_summing_counts(outcomes.iter().cloned());
    let count = summary.as_count().unwrap_or(0);
    if count != 0 {
        return count;
    }
    i64::from(
        summary
            .execution_facts
            .iter()
            .any(|fact| matches!(fact, ExecutionFact::Accepted)),
    )
}

pub(super) fn summarize_tagged_iterations(
    outcomes: Vec<EffectOutcome>,
    ranges: &[(PlayerId, usize, usize)],
) -> EffectOutcome {
    let mut counts = Vec::new();
    for &(player, start, end) in ranges {
        let count = correlated_player_count(&outcomes[start..end]);
        if let Some((_, total)) = counts.iter_mut().find(|(actor, _)| *actor == player) {
            *total += count;
        } else {
            counts.push((player, count));
        }
    }
    EffectOutcome::aggregate_summing_counts(outcomes).with_player_counts(counts)
}

impl SimultaneousEffectProposal for ObjectIterationProposal {
    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        crate::effects::damage::DamageActionInputs::collect(
            self.children
                .iter()
                .map(|child| child.damage_action_inputs()),
        )
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &CompletedEffectOutputs,
    ) -> Result<DamageActionBinding, ExecutionError> {
        let mut bindings = Vec::with_capacity(self.children.len());
        for child in self.children {
            bindings.push(child.bind_damage_action(game, ctx, owner)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(DamageActionBinding::from_outcome(EffectOutcome::count(0)));
            }
        }
        Ok(DamageActionBinding::from_bindings(bindings, |outcomes| {
            project(outcomes, self.player_ranges.as_deref())
        }))
    }

    fn declared_payment_resources(&self) -> Vec<PaymentResourceClaim> {
        self.children
            .iter()
            .flat_map(|child| child.declared_payment_resources())
            .collect()
    }

    fn declared_life_payments(&self) -> Vec<(PlayerId, u32)> {
        self.children
            .iter()
            .flat_map(|child| child.declared_life_payments())
            .collect()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_phase(game, ctx, |child, game, ctx| {
            child.prepare_selection(game, ctx)
        })
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_phase(game, ctx, |child, game, ctx| {
            child.prepare_original(game, ctx)
        })
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_phase(game, ctx, |child, game, ctx| child.seal_original(game, ctx))
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let mut receipts = Vec::with_capacity(self.children.len());
        for child in self.children {
            receipts.push(child.commit_original_with_outputs(game, ctx)?);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(
                    CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
                ));
            }
        }
        let ranges = self.player_ranges;
        Ok(super::compose_original_commits_with_projection_outputs(
            receipts,
            Box::new(move |outcomes| project(outcomes, ranges.as_deref())),
        ))
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::complete_prepared_original(self, game, ctx)
    }
}

//! Selected branch scopes delegate a child's prepared action lifecycle.

use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::{
    ExecutionContext, ExecutionError, SimultaneousEffectCommit, SimultaneousEffectCompletion,
    SimultaneousEffectProposal,
};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct ChoiceState {
    objects: HashMap<TagKey, Vec<ObjectSnapshot>>,
    players: HashMap<TagKey, Vec<PlayerId>>,
    outcomes: HashMap<crate::effect::EffectId, EffectOutcome>,
    x: Option<u32>,
}

impl ChoiceState {
    fn capture(ctx: &ExecutionContext) -> Self {
        Self {
            objects: ctx.tagged_objects.clone(),
            players: ctx.tagged_players.clone(),
            outcomes: ctx.effect_outcomes.clone(),
            x: ctx.x_value,
        }
    }

    fn restore(self, ctx: &mut ExecutionContext) {
        ctx.tagged_objects = self.objects;
        ctx.tagged_players = self.players;
        ctx.effect_outcomes = self.outcomes;
        ctx.x_value = self.x;
    }
}

#[derive(Debug, Clone, Default)]
struct ChoiceChanges {
    objects: Vec<(TagKey, Option<Vec<ObjectSnapshot>>)>,
    players: Vec<(TagKey, Option<Vec<PlayerId>>)>,
    outcomes: Vec<(crate::effect::EffectId, Option<EffectOutcome>)>,
    x: Option<Option<u32>>,
}

fn changed_entries<K: Clone + Eq + std::hash::Hash, V: Clone + PartialEq>(
    before: &HashMap<K, V>,
    after: &HashMap<K, V>,
) -> Vec<(K, Option<V>)> {
    before
        .keys()
        .chain(after.keys())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter(|key| before.get(*key) != after.get(*key))
        .map(|key| (key.clone(), after.get(key).cloned()))
        .collect()
}

fn apply_entries<K: Clone + Eq + std::hash::Hash, V: Clone>(
    changes: &[(K, Option<V>)],
    target: &mut HashMap<K, V>,
) {
    for (key, value) in changes {
        if let Some(value) = value {
            target.insert(key.clone(), value.clone());
        } else {
            target.remove(key);
        }
    }
}

impl ChoiceChanges {
    fn capture(before: &ChoiceState, ctx: &ExecutionContext) -> Self {
        Self {
            objects: changed_entries(&before.objects, &ctx.tagged_objects),
            players: changed_entries(&before.players, &ctx.tagged_players),
            outcomes: changed_entries(&before.outcomes, &ctx.effect_outcomes),
            x: (before.x != ctx.x_value).then_some(ctx.x_value),
        }
    }

    fn retain_selection_outputs(&mut self, selection: &[Effect], ctx: &ExecutionContext) {
        for effect in selection {
            effect
                .0
                .visit_prepared_selection_bindings(&mut |binding| match binding {
                    crate::effects::PreparedSelectionBinding::ObjectTag(tag) => {
                        if !self.objects.iter().any(|(key, _)| key == &tag) {
                            let value = ctx.tagged_objects.get(&tag).cloned();
                            self.objects.push((tag, value));
                        }
                    }
                    crate::effects::PreparedSelectionBinding::Outcome(id) => {
                        if !self.outcomes.iter().any(|(key, _)| *key == id) {
                            self.outcomes
                                .push((id, ctx.effect_outcomes.get(&id).cloned()));
                        }
                    }
                });
        }
        if !selection.is_empty() {
            self.x = Some(ctx.x_value);
        }
    }

    // Retain every selected input even if it happens to equal another
    // participant's current value when this original commits. Then overlay
    // outputs produced by the action, so completion sees its original world.
    fn completion_bindings(&self, before: &ChoiceState, ctx: &ExecutionContext) -> Self {
        let mut changes = Self::capture(before, ctx);
        for (key, _) in &self.objects {
            if !changes
                .objects
                .iter()
                .any(|(candidate, _)| candidate == key)
            {
                changes
                    .objects
                    .push((key.clone(), ctx.tagged_objects.get(key).cloned()));
            }
        }
        for (key, _) in &self.players {
            if !changes
                .players
                .iter()
                .any(|(candidate, _)| candidate == key)
            {
                changes
                    .players
                    .push((key.clone(), ctx.tagged_players.get(key).cloned()));
            }
        }
        for (key, _) in &self.outcomes {
            if !changes
                .outcomes
                .iter()
                .any(|(candidate, _)| candidate == key)
            {
                changes
                    .outcomes
                    .push((*key, ctx.effect_outcomes.get(key).cloned()));
            }
        }
        if self.x.is_some() {
            changes.x = Some(ctx.x_value);
        }
        changes
    }

    fn apply(&self, ctx: &mut ExecutionContext) {
        apply_entries(&self.objects, &mut ctx.tagged_objects);
        apply_entries(&self.players, &mut ctx.tagged_players);
        apply_entries(&self.outcomes, &mut ctx.effect_outcomes);
        if let Some(x) = self.x {
            ctx.x_value = x;
        }
    }
}

#[derive(Debug, Clone)]
struct BranchScope {
    payment: Option<crate::costs::PaymentScope>,
    player: Option<PlayerId>,
    optional: bool,
    accepted: bool,
}

impl BranchScope {
    fn run<T>(
        &self,
        ctx: &mut ExecutionContext,
        body: impl FnOnce(&mut ExecutionContext) -> Result<T, ExecutionError>,
    ) -> Result<T, ExecutionError> {
        let previous_optional = ctx.optional_action;
        ctx.optional_action |= self.optional;
        let result = if let Some(payment) = &self.payment {
            payment.run(ctx, |ctx| ctx.with_temp_iterated_player(self.player, body))
        } else {
            ctx.with_temp_iterated_player(self.player, body)
        };
        ctx.optional_action = previous_optional;
        result
    }

    fn project(&self, outcome: EffectOutcome, ctx: &ExecutionContext) -> EffectOutcome {
        if ctx.decision_maker.awaiting_choice() {
            return EffectOutcome::count(0);
        }
        let outcome = EffectOutcome::aggregate(vec![outcome]);
        if self.accepted {
            outcome.with_execution_fact(ExecutionFact::Accepted)
        } else {
            outcome
        }
    }
}

struct PreparedBranch {
    purpose: crate::effects::EffectExecutionPurpose,
    scope: BranchScope,
    inner: Option<Box<dyn SimultaneousEffectProposal>>,
    selection: Vec<Effect>,
    action: Option<Effect>,
    selection_receipts: Vec<crate::effects::CompletedEffectOutputs>,
    bindings: ChoiceChanges,
}

struct BranchCompletion {
    scope: BranchScope,
    bindings: ChoiceChanges,
    inner: Box<dyn SimultaneousEffectCompletion>,
}

impl SimultaneousEffectCompletion for BranchCompletion {
    fn original_phase_status(&self) -> crate::effects::OriginalPhaseStatus {
        self.inner.original_phase_status()
    }

    fn complete_original_phase_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self {
            scope,
            bindings,
            inner,
        } = *self;
        let before = ChoiceState::capture(ctx);
        let mut receipt = scope.run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.complete_original_phase_with_outputs(game, ctx, original)
        })?;
        let bindings = bindings.completion_bindings(&before, ctx);
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(Self {
                scope,
                bindings,
                inner,
            }));
        } else if !ctx.decision_maker.awaiting_choice() {
            receipt.outcome.outcome = scope.project(receipt.outcome.outcome, ctx);
            receipt.outcome.synchronize_observations();
        }
        Ok(receipt)
    }

    fn complete_original_phase_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self {
            scope,
            bindings,
            inner,
        } = *self;
        let before = ChoiceState::capture(ctx);
        let mut receipt = scope.run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.complete_original_phase_from_outputs(game, ctx, original)
        })?;
        let bindings = bindings.completion_bindings(&before, ctx);
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(Self {
                scope,
                bindings,
                inner,
            }));
        } else if !ctx.decision_maker.awaiting_choice() {
            receipt.outcome.outcome = scope.project(receipt.outcome.outcome, ctx);
            receipt.outcome.synchronize_observations();
        }
        Ok(receipt)
    }

    fn prepare_draw_boundary_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: EffectOutcome,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self {
            scope,
            bindings,
            inner,
        } = *self;
        let before = ChoiceState::capture(ctx);
        let mut receipt = scope.run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.prepare_draw_boundary_with_outputs(game, ctx, original)
        })?;
        let bindings = bindings.completion_bindings(&before, ctx);
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(Self {
                scope,
                bindings,
                inner,
            }));
        } else if !ctx.decision_maker.awaiting_choice() {
            receipt.outcome.outcome = scope.project(receipt.outcome.outcome, ctx);
            receipt.outcome.synchronize_observations();
        }
        Ok(receipt)
    }

    fn prepare_draw_boundary_from_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        crate::effects::ExecutionError,
    > {
        let Self {
            scope,
            bindings,
            inner,
        } = *self;
        let before = ChoiceState::capture(ctx);
        let mut receipt = scope.run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.prepare_draw_boundary_from_outputs(game, ctx, original)
        })?;
        let bindings = bindings.completion_bindings(&before, ctx);
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(Self {
                scope,
                bindings,
                inner,
            }));
        } else if !ctx.decision_maker.awaiting_choice() {
            receipt.outcome.outcome = scope.project(receipt.outcome.outcome, ctx);
            receipt.outcome.synchronize_observations();
        }
        Ok(receipt)
    }

    fn observe_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        original: &mut EffectOutcome,
    ) -> Result<(), crate::effects::ExecutionError> {
        let parent = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = self.scope.clone().run(ctx, |ctx| {
            self.bindings.apply(ctx);
            self.inner.observe_original(game, ctx, original)
        });
        parent.restore(ctx);
        result
    }

    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        self.inner.freeze(game)
    }

    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.complete_with_outputs(game, ctx, original)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn complete_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: EffectOutcome,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self {
            scope,
            bindings,
            inner,
        } = *self;
        let mut outputs = scope.clone().run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.complete_with_outputs(game, ctx, original)
        })?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        outputs.outcome = scope.clone().project(outputs.outcome, ctx);
        outputs.synchronize_observations();
        Ok(outputs)
    }

    fn complete_from_original_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        original: crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        let Self {
            scope,
            bindings,
            inner,
        } = *self;
        let mut outputs = scope.clone().run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.complete_from_original_outputs(game, ctx, original)
        })?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        outputs.outcome = scope.clone().project(outputs.outcome, ctx);
        outputs.synchronize_observations();
        Ok(outputs)
    }
}

impl std::fmt::Debug for PreparedBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PreparedBranch")
            .field("scope", &self.scope)
            .field("inner", &self.inner)
            .field("selection", &self.selection)
            .field("action", &self.action)
            .field(
                "selection_receipts",
                &self
                    .selection_receipts
                    .iter()
                    .map(|outputs| &outputs.outcome)
                    .collect::<Vec<_>>(),
            )
            .field("bindings", &self.bindings)
            .finish()
    }
}

impl SimultaneousEffectProposal for PreparedBranch {
    fn has_simultaneous_originals(&self) -> bool {
        self.inner
            .as_ref()
            .is_some_and(|inner| inner.has_simultaneous_originals())
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        self.inner
            .as_ref()
            .and_then(|inner| inner.nominal_payment_quantity())
    }

    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        self.inner.as_ref()?.damage_action_inputs()
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &crate::effects::CompletedEffectOutputs,
    ) -> Result<crate::effects::DamageActionBinding, ExecutionError> {
        let Self {
            scope,
            inner,
            bindings,
            selection_receipts,
            ..
        } = *self;
        let inner = inner.ok_or_else(|| {
            ExecutionError::InternalError("damage branch bound before child preparation".into())
        })?;
        let mut binding = scope.clone().run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.bind_damage_action(game, ctx, owner)
        })?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(crate::effects::DamageActionBinding::from_outcome(
                EffectOutcome::count(0),
            ));
        }
        let outcome = if selection_receipts.is_empty() {
            binding.outcome
        } else {
            EffectOutcome::aggregate(
                selection_receipts
                    .iter()
                    .map(|outputs| outputs.outcome.clone())
                    .chain(std::iter::once(binding.outcome)),
            )
        };
        binding.outcome = scope.clone().project(outcome, ctx);
        let mut preludes = selection_receipts;
        preludes.extend(binding.preludes);
        binding.preludes = preludes;
        if scope.accepted {
            binding.completion_facts.push(ExecutionFact::Accepted);
        }
        Ok(binding)
    }

    fn declared_life_payment(&self) -> Option<(PlayerId, u32)> {
        self.inner
            .as_ref()
            .and_then(|inner| inner.declared_life_payment())
    }

    fn declared_payment_resources(&self) -> Vec<crate::effects::PaymentResourceClaim> {
        self.inner
            .as_ref()
            .map(|inner| inner.declared_payment_resources())
            .unwrap_or_default()
    }

    fn declared_life_payments(&self) -> Vec<(PlayerId, u32)> {
        self.inner
            .as_ref()
            .map(|inner| inner.declared_life_payments())
            .unwrap_or_default()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if let Some(inner) = &mut self.inner {
            let before = ChoiceState::capture(ctx);
            let result = self.scope.clone().run(ctx, |ctx| {
                self.bindings.apply(ctx);
                inner.prepare_selection(game, ctx)
            });
            if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
                self.bindings = self.bindings.completion_bindings(&before, ctx);
                self.bindings.retain_selection_outputs(&self.selection, ctx);
            }
            if !self.selection.is_empty() {
                before.restore(ctx);
            }
            return result;
        }
        let before = ChoiceState::capture(ctx);
        let result = self.scope.clone().run(ctx, |ctx| {
            for (index, effect) in self.selection.iter().enumerate() {
                let mut outcome = self.purpose.execute(game, effect, ctx)?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(());
                }
                if self.scope.accepted {
                    outcome.outcome.set_value(crate::effect::OutcomeValue::None);
                }
                self.selection_receipts.push(outcome);
                crate::effects::match_triggers_at_instruction_boundary(
                    game,
                    ctx,
                    self.selection.get(index + 1).or(self.action.as_ref()),
                    self.selection_receipts
                        .iter()
                        .flat_map(|outcome| outcome.outcome.events.iter()),
                )?;
            }
            let action = self.action.as_ref().ok_or_else(|| {
                ExecutionError::Impossible("prepared branch has no selected action".into())
            })?;
            let Some(mut inner) = prepare_action_for_purpose(action, self.purpose, game, ctx)?
            else {
                return Ok(());
            };
            if ctx.decision_maker.awaiting_choice() {
                return Ok(());
            }
            inner.prepare_selection(game, ctx)?;
            self.inner = Some(inner);
            Ok(())
        });
        self.bindings = ChoiceChanges::capture(&before, ctx);
        self.bindings.retain_selection_outputs(&self.selection, ctx);
        before.restore(ctx);
        result
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.inner.is_none() {
            self.prepare_selection(game, ctx)?;
        }
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        let inner = self.inner.as_mut().ok_or_else(|| {
            ExecutionError::InternalError("selected branch has no prepared original".into())
        })?;
        let before = ChoiceState::capture(ctx);
        let result = self.scope.run(ctx, |ctx| {
            self.bindings.apply(ctx);
            inner.prepare_original(game, ctx)
        });
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            self.bindings = self.bindings.completion_bindings(&before, ctx);
        }
        if !self.selection.is_empty() {
            before.restore(ctx);
        }
        result
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if ctx.decision_maker.awaiting_choice() {
            return Ok(());
        }
        let inner = self.inner.as_mut().ok_or_else(|| {
            ExecutionError::InternalError("selected branch sealed before preparation".into())
        })?;
        let before = ChoiceState::capture(ctx);
        let result = self.scope.clone().run(ctx, |ctx| {
            self.bindings.apply(ctx);
            inner.seal_original(game, ctx)
        });
        if result.is_ok() && !ctx.decision_maker.awaiting_choice() {
            self.bindings = self.bindings.completion_bindings(&before, ctx);
        }
        if !self.selection.is_empty() {
            before.restore(ctx);
        }
        result
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>, ExecutionError>
    {
        let Self {
            scope,
            inner,
            selection_receipts,
            bindings,
            ..
        } = *self;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let inner = inner.ok_or_else(|| {
            ExecutionError::Impossible("selected branch committed before preparation".into())
        })?;
        let before = ChoiceState::capture(ctx);
        let receipt = scope.clone().run(ctx, |ctx| {
            bindings.apply(ctx);
            inner.commit_original_with_outputs(game, ctx)
        })?;
        let completion_bindings = bindings.completion_bindings(&before, ctx);
        let mut receipts = selection_receipts
            .into_iter()
            .map(SimultaneousEffectCommit::finished)
            .collect::<Vec<_>>();
        receipts.push(receipt);
        let mut receipt = super::compose_original_commits_with_outputs(receipts);
        if let Some(inner) = receipt.completion.take() {
            receipt.completion = Some(Box::new(BranchCompletion {
                scope,
                bindings: completion_bindings,
                inner,
            }));
        } else {
            let outcome = scope.clone().project(receipt.outcome.outcome.clone(), ctx);
            receipt.outcome = receipt.outcome.project_aggregate(outcome);
        }
        Ok(receipt)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        super::complete_prepared_original(self, game, ctx)
    }
}

/// A selected action unit consists of object-selection preludes and one
/// prepared action. Separate mutating instructions still need action-unit
/// scheduling; they must not be prepared against the same original world.
pub(super) fn prepare_action_branch(
    effects: &[Effect],
    game: &GameState,
    ctx: &mut ExecutionContext,
    player: Option<PlayerId>,
    optional: bool,
    accepted: bool,
) -> Result<Option<Box<dyn SimultaneousEffectProposal>>, ExecutionError> {
    prepare_branch_for_purpose(
        effects,
        game,
        ctx,
        player,
        optional,
        accepted,
        crate::effects::EffectExecutionPurpose::Action,
    )
}

pub(super) fn prepare_action_for_purpose(
    action: &Effect,
    purpose: crate::effects::EffectExecutionPurpose,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<Option<Box<dyn SimultaneousEffectProposal>>, ExecutionError> {
    match purpose {
        crate::effects::EffectExecutionPurpose::Action => action
            .prepare_simultaneous_player_action(game, ctx)
            .map(Some),
        crate::effects::EffectExecutionPurpose::Payment => {
            let component = crate::costs::Cost::try_effect(action.clone())
                .map_err(ExecutionError::Impossible)?;
            let total = crate::cost::TotalCost::from_cost(component);
            let payer = ctx.controller;
            let reason = ctx
                .mana
                .payment_reason
                .unwrap_or(crate::costs::PaymentReason::Other);
            let prepared = crate::costs::prepare_total_cost(&total, game, ctx, payer, reason)?;
            if prepared.is_none() && !ctx.decision_maker.awaiting_choice() {
                return Err(ExecutionError::Impossible(
                    "selected branch payment has no prepared owner".into(),
                ));
            }
            Ok(prepared)
        }
    }
}

pub(super) fn supports_action_preparation(
    action: &Effect,
    purpose: crate::effects::EffectExecutionPurpose,
) -> bool {
    match purpose {
        crate::effects::EffectExecutionPurpose::Action => {
            action.0.supports_simultaneous_player_action()
        }
        crate::effects::EffectExecutionPurpose::Payment => {
            crate::costs::Cost::try_effect(action.clone())
                .is_ok_and(|cost| cost.0.supports_prepared_payment())
        }
    }
}

pub(super) fn prepare_branch_for_purpose(
    effects: &[Effect],
    game: &GameState,
    ctx: &mut ExecutionContext,
    player: Option<PlayerId>,
    optional: bool,
    accepted: bool,
    purpose: crate::effects::EffectExecutionPurpose,
) -> Result<Option<Box<dyn SimultaneousEffectProposal>>, ExecutionError> {
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }
    let Some((action, selection)) = effects.split_last() else {
        return Ok(None);
    };
    let supported = supports_action_preparation(action, purpose);
    if !supported
        || !selection.iter().all(|effect| {
            super::may::is_object_selection(effect)
                && effect.0.is_read_only_simultaneous_player_action()
        })
    {
        return Ok(None);
    }
    let scope = BranchScope {
        payment: matches!(purpose, crate::effects::EffectExecutionPurpose::Payment).then(|| {
            crate::costs::PaymentScope::new(
                ctx,
                ctx.controller,
                ctx.mana
                    .payment_reason
                    .unwrap_or(crate::costs::PaymentReason::Other),
            )
        }),
        player,
        optional,
        accepted,
    };
    let inner = if selection.is_empty() {
        scope.clone().run(ctx, |ctx| {
            prepare_action_for_purpose(action, purpose, game, ctx)
        })?
    } else {
        None
    };
    Ok(Some(Box::new(PreparedBranch {
        purpose,
        scope,
        inner,
        selection: selection.to_vec(),
        action: Some(action.clone()),
        selection_receipts: Vec::new(),
        bindings: ChoiceChanges::default(),
    })))
}

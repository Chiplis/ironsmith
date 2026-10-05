//! Retained native object-iterator execution. Selection and iteration bindings
//! belong to the iterator; every authored child still uses normal instruction
//! dispatch, or that child's native replacement continuation capability.
use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::{ExecutionContext, ExecutionContextCheckpoint, ExecutionError,
    SimultaneousEffectCommit, SimultaneousEffectCompletion};
use crate::effects::replacement::{PreparedReplacementChild, ReplacementResume};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

#[derive(Debug, Clone)]
pub(super) struct ObjectIterationBinding {
    pub object: ObjectId,
    pub snapshot: ObjectSnapshot,
    pub player: PlayerId,
    pub previous: Option<Vec<ObjectSnapshot>>,
}
impl ObjectIterationBinding {
    pub fn with_scope<T>(&self, ctx: &mut ExecutionContext,
        execute: impl FnOnce(&mut ExecutionContext) -> T) -> T {
        let it = TagKey::from("__it__");
        let previous = TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG);
        let saved_it = ctx.tagged_objects.insert(it.clone(), vec![self.snapshot.clone()]);
        let saved_previous = self.previous.as_ref().map(|snapshots|
            ctx.tagged_objects.insert(previous.clone(), snapshots.clone()));
        let object = ctx.iteration.iterated_object.replace(self.object);
        let player = ctx.iteration.iterated_player.replace(self.player);
        let result = execute(ctx);
        ctx.iteration.iterated_object = object;
        ctx.iteration.iterated_player = player;
        restore_tag(ctx, it, saved_it);
        if let Some(saved) = saved_previous { restore_tag(ctx, previous, saved); }
        result
    }
}
fn restore_tag(ctx: &mut ExecutionContext, tag: TagKey, saved: Option<Vec<ObjectSnapshot>>) {
    if let Some(saved) = saved { ctx.tagged_objects.insert(tag, saved); }
    else { ctx.tagged_objects.remove(&tag); }
}

pub(super) fn correlated_player_count(outcomes: &[EffectOutcome]) -> i64 {
    let summary = EffectOutcome::aggregate_summing_counts(outcomes.iter().cloned());
    let count = summary.as_count().unwrap_or(0);
    if count != 0 {
        return count;
    }
    // Accepting an optional action is itself the correlated "did" result,
    // even when a hidden-zone search legally finds no card.
    i64::from(
        summary
            .execution_facts
            .iter()
            .any(|fact| matches!(fact, ExecutionFact::Accepted)),
    )
}

pub(super) fn finish_iterations(bindings: &[ObjectIterationBinding],
    outcomes: &[Vec<EffectOutcome>], correlated: bool) -> EffectOutcome {
    if bindings.is_empty() { return EffectOutcome::count(0); }
    let outcome = EffectOutcome::aggregate_summing_counts(outcomes.iter().flatten().cloned());
    if !correlated { return outcome; }
    let mut counts: Vec<(PlayerId, i64)> = Vec::new();
    for (binding, iteration) in bindings.iter().zip(outcomes) {
        if iteration.is_empty() { continue; }
        let count = correlated_player_count(iteration);
        if let Some((_, total)) = counts.iter_mut().find(|(player, _)| *player == binding.player) {
            *total += count;
        } else { counts.push((binding.player, count)); }
    }
    outcome.with_player_counts(counts)
}

pub(super) struct ObjectIterationState {
    bindings: Vec<ObjectIterationBinding>,
    effects: Vec<Effect>,
    outcomes: Vec<Vec<EffectOutcome>>,
    correlated: bool,
    entry_attachments: bool,
    iteration: usize,
    instruction: usize,
    pending: Option<Box<dyn ReplacementResume>>,
    active_scope: Option<(crate::effects::context::IterationContext,
        Option<Vec<ObjectSnapshot>>, Option<Vec<ObjectSnapshot>>)>,
}
impl ObjectIterationState {
    pub fn new(bindings: Vec<ObjectIterationBinding>, effects: Vec<Effect>,
        correlated: bool, entry_attachments: bool) -> Self {
        let outcomes = vec![Vec::new(); bindings.len()];
        Self { bindings, effects, outcomes, correlated, entry_attachments,
            iteration: 0, instruction: 0, pending: None, active_scope: None }
    }

    pub fn run(mut self, game: &mut GameState, ctx: &mut ExecutionContext,
        defer_draws: bool) -> Result<SimultaneousEffectCommit, ExecutionError> {
        while self.iteration < self.bindings.len() {
            while self.instruction < self.effects.len() {
                let effect = &self.effects[self.instruction];
                crate::effects::capture_triggers_before_added_program(game, ctx, Some(effect),
                    self.outcomes.iter_mut().flatten().flat_map(|outcome| outcome.events.iter_mut()))?;
                let pending = self.pending.take();
                let binding = &self.bindings[self.iteration];
                let mut active_scope = self.active_scope.take();
                let mut prepared = binding.with_scope(ctx, |ctx| {
                    if let Some((iteration, it, previous)) = active_scope.take() {
                        ctx.iteration = iteration;
                        restore_tag(ctx, TagKey::from("__it__"), it);
                        if binding.previous.is_some() {
                            restore_tag(ctx, TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG), previous);
                        }
                    }
                    let saved_attachment = ctx.pending_entry_attachment.clone();
                    if self.entry_attachments {
                        ctx.pending_entry_attachment = crate::effects::permanents::entry_attachment_for_move(
                            effect, self.effects.get(self.instruction + 1));
                    }
                    let result = if let Some(pending) = pending {
                        pending.resume(game, ctx).map(|prefix| PreparedReplacementChild { prefix, resume: None })
                    } else if defer_draws {
                        crate::effects::replacement::prepare_replacement_child(game, ctx, effect)
                    } else {
                        crate::effects::execute_effect(game, effect, ctx)
                            .map(|prefix| PreparedReplacementChild { prefix, resume: None })
                    };
                    ctx.pending_entry_attachment = saved_attachment;
                    active_scope = Some((ctx.iteration, ctx.tagged_objects.get(&TagKey::from("__it__")).cloned(),
                        ctx.tagged_objects.get(&TagKey::from(ironsmith_core::PREVIOUS_ITERATED_OBJECTS_TAG)).cloned()));
                    result
                })?;
                self.active_scope = active_scope;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0)));
                }
                if let Some(pending) = prepared.resume {
                    crate::effects::capture_triggers_before_added_program(game, ctx, None,
                        self.outcomes.iter_mut().flatten().flat_map(|outcome| outcome.events.iter_mut())
                            .chain(prepared.prefix.events.iter_mut()))?;
                    // The provisional child prefix is evidence only. The retained
                    // child returns its entire result once, in this same slot.
                    let mut prefix = self.outcomes.clone();
                    prefix[self.iteration].push(prepared.prefix);
                    let outcome = finish_iterations(&self.bindings, &prefix, self.correlated);
                    self.pending = Some(pending);
                    return Ok(SimultaneousEffectCommit { outcome,
                        completion: Some(Box::new(ObjectIterationCompletion {
                            state: self, context: ExecutionContextCheckpoint::capture(ctx),
                        })),
                    });
                }
                self.outcomes[self.iteration].push(prepared.prefix);
                self.instruction += 1;
            }
            self.iteration += 1;
            self.instruction = 0;
            self.active_scope = None;
        }
        crate::effects::capture_triggers_before_added_program(game, ctx, None,
            self.outcomes.iter_mut().flatten().flat_map(|outcome| outcome.events.iter_mut()))?;
        Ok(SimultaneousEffectCommit::finished(finish_iterations(&self.bindings, &self.outcomes, self.correlated)))
    }
}
struct ObjectIterationCompletion {
    state: ObjectIterationState,
    context: ExecutionContextCheckpoint,
}
impl SimultaneousEffectCompletion for ObjectIterationCompletion {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        if let Some(pending) = &mut self.state.pending { pending.freeze(game)?; }
        Ok(())
    }
    fn prepare_draw_boundary(self: Box<Self>, _game: &mut GameState, _ctx: &mut ExecutionContext,
        original: EffectOutcome) -> Result<SimultaneousEffectCommit, ExecutionError> {
        // The cursor has already reached an actual child's draw boundary.
        Ok(SimultaneousEffectCommit { outcome: original, completion: Some(self) })
    }
    fn complete(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        _original: EffectOutcome) -> Result<EffectOutcome, ExecutionError> {
        let executing_effect = ctx.executing_effect;
        self.context.restore_ref(ctx);
        let result = self.state.run(game, ctx, false).map(|committed| committed.outcome);
        ctx.executing_effect = executing_effect;
        result
    }
}

/// Native simultaneous object proposals forward every phase rather than
/// completing a child's replacement program in the original mutation phase.
#[derive(Debug)]
pub(super) struct ObjectIterationProposal {
    pub bindings: Vec<ObjectIterationBinding>,
    pub iterations: Vec<Vec<Box<dyn crate::effects::SimultaneousEffectProposal>>>,
    pub correlated: bool,
    pub tagged_set: Option<(TagKey, Vec<ObjectSnapshot>)>,
    pub shuffle_owners: Vec<PlayerId>,
    pub attachments: Vec<Option<crate::target::ChooseSpec>>,
}
fn with_attachment<T>(ctx: &mut ExecutionContext, attachment: Option<crate::target::ChooseSpec>,
    execute: impl FnOnce(&mut ExecutionContext) -> T) -> T {
    let saved = std::mem::replace(&mut ctx.pending_entry_attachment, attachment);
    let result = execute(ctx);
    ctx.pending_entry_attachment = saved;
    result
}
fn with_tagged_set<T>(ctx: &mut ExecutionContext,
    tagged: &Option<(TagKey, Vec<ObjectSnapshot>)>, execute: impl FnOnce(&mut ExecutionContext) -> T) -> T {
    let saved = tagged.as_ref().map(|(tag, objects)| (tag.clone(),
        ctx.tagged_objects.insert(tag.clone(), objects.clone())));
    let result = execute(ctx);
    if let Some((tag, objects)) = saved { restore_tag(ctx, tag, objects); }
    result
}
impl crate::effects::SimultaneousEffectProposal for ObjectIterationProposal {
    fn prepare_selection(&mut self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        with_tagged_set(ctx, &self.tagged_set, |ctx| {
            for (binding, proposals) in self.bindings.iter().zip(&mut self.iterations) {
                for (index, proposal) in proposals.iter_mut().enumerate() {
                    binding.with_scope(ctx, |ctx| with_attachment(ctx, self.attachments[index].clone(),
                        |ctx| proposal.prepare_selection(game, ctx)))?;
                    if ctx.decision_maker.awaiting_choice() { return Ok(()); }
                }
            }
            Ok(())
        })
    }
    fn prepare_original(&mut self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        with_tagged_set(ctx, &self.tagged_set, |ctx| {
            for (binding, proposals) in self.bindings.iter().zip(&mut self.iterations) {
                for (index, proposal) in proposals.iter_mut().enumerate() {
                    binding.with_scope(ctx, |ctx| with_attachment(ctx, self.attachments[index].clone(),
                        |ctx| proposal.prepare_original(game, ctx)))?;
                    if ctx.decision_maker.awaiting_choice() { return Ok(()); }
                }
            }
            Ok(())
        })
    }
    fn commit_original(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<SimultaneousEffectCommit, ExecutionError> {
        let Self { bindings, iterations, correlated, tagged_set, shuffle_owners, attachments } = *self;
        let external_tag = tagged_set.as_ref().map(|(tag, _)| (tag.clone(), ctx.tagged_objects.get(tag).cloned()));
        with_tagged_set(ctx, &tagged_set, |ctx| {
            let mut records = Vec::new();
            for (index, (binding, proposals)) in bindings.iter().zip(iterations).enumerate() {
                for (instruction, proposal) in proposals.into_iter().enumerate() {
                    let committed = binding.with_scope(ctx, |ctx| with_attachment(ctx, attachments[instruction].clone(),
                        |ctx| proposal.commit_original(game, ctx)))?;
                    records.push(ObjectIterationRecord { iteration: index, outcome: committed.outcome,
                        completion: committed.completion, attachment: attachments[instruction].clone(),
                        context: ExecutionContextCheckpoint::capture(ctx) });
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0)));
                    }
                }
            }
            let mut original_events = records.iter().flat_map(|record| record.outcome.events.iter().cloned())
                .collect::<Vec<_>>();
            for owner in shuffle_owners {
                game.shuffle_player_library(owner);
                let provenance = game.alloc_child_event_provenance(ctx.provenance, crate::events::EventKind::ShuffleLibrary);
                original_events.push(crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::ShuffleLibraryEvent::new(owner, ctx.cause.clone()), provenance));
            }
            let batch = ObjectIterationBatch { bindings, records, original_events, correlated, external_tag,
                context: ExecutionContextCheckpoint::capture(ctx) };
            let outcome = batch.outcome();
            if batch.records.iter().any(|record| record.completion.is_some()) {
                Ok(SimultaneousEffectCommit { outcome, completion: Some(Box::new(batch)) })
            } else { Ok(SimultaneousEffectCommit::finished(outcome)) }
        })
    }
    fn commit(mut self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        self.prepare_selection(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        self.prepare_original(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let committed = self.commit_original(game, ctx)?;
        finish_original(committed, game, ctx, false).map(|committed| committed.outcome)
    }
}

struct ObjectIterationRecord {
    iteration: usize,
    attachment: Option<crate::target::ChooseSpec>,
    outcome: EffectOutcome,
    completion: Option<Box<dyn SimultaneousEffectCompletion>>,
    context: ExecutionContextCheckpoint,
}
struct ObjectIterationBatch {
    bindings: Vec<ObjectIterationBinding>,
    records: Vec<ObjectIterationRecord>,
    original_events: Vec<crate::triggers::TriggerEvent>,
    correlated: bool,
    external_tag: Option<(TagKey, Option<Vec<ObjectSnapshot>>)>,
    context: ExecutionContextCheckpoint,
}
impl ObjectIterationBatch {
    fn outcome(&self) -> EffectOutcome {
        let mut iterations = vec![Vec::new(); self.bindings.len()];
        for record in &self.records { iterations[record.iteration].push(record.outcome.clone()); }
        let mut outcome = finish_iterations(&self.bindings, &iterations, self.correlated);
        // Original mutations (including the owner's physical shuffle) precede
        // every appended program in both the game and the reported evidence.
        let originals = self.original_events.iter().map(|event| event.occurrence_key())
            .collect::<std::collections::HashSet<_>>();
        let additions = outcome.events.into_iter().filter(|event| !originals.contains(&event.occurrence_key()));
        outcome.events = self.original_events.iter().cloned().chain(additions).collect();
        outcome
    }
    fn run(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        defer_draws: bool) -> Result<SimultaneousEffectCommit, ExecutionError> {
        let external_tag = self.external_tag.clone();
        let executing_effect = ctx.executing_effect;
        let result = self.run_inner(game, ctx, defer_draws);
        if let Some((tag, saved)) = external_tag { restore_tag(ctx, tag, saved); }
        ctx.executing_effect = executing_effect;
        result
    }
    fn run_inner(mut self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        defer_draws: bool) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.context.restore_ref(ctx);
        crate::effects::capture_triggers_before_added_program(game, ctx, None,
            self.original_events.iter_mut().chain(self.records.iter_mut()
                .flat_map(|record| record.outcome.events.iter_mut())))?;
        for record in &mut self.records {
            let Some(completion) = record.completion.take() else { continue; };
            record.context.restore_ref(ctx);
            let original = record.outcome.clone();
            let committed = self.bindings[record.iteration].with_scope(ctx, |ctx| with_attachment(ctx, record.attachment.clone(), |ctx| {
                if defer_draws { completion.prepare_draw_boundary(game, ctx, original) }
                else { completion.complete(game, ctx, original).map(SimultaneousEffectCommit::finished) }
            }))?;
            record.outcome = committed.outcome;
            record.completion = committed.completion;
            record.context = ExecutionContextCheckpoint::capture(ctx);
            if ctx.decision_maker.awaiting_choice() {
                return Ok(SimultaneousEffectCommit::finished(EffectOutcome::count(0)));
            }
        }
        crate::effects::capture_triggers_before_added_program(game, ctx, None,
            self.records.iter_mut().flat_map(|record| record.outcome.events.iter_mut()))?;
        let outcome = self.outcome();
        if self.records.iter().any(|record| record.completion.is_some()) {
            Ok(SimultaneousEffectCommit { outcome, completion: Some(self) })
        } else { Ok(SimultaneousEffectCommit::finished(outcome)) }
    }
}
impl SimultaneousEffectCompletion for ObjectIterationBatch {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
        game.freeze_completed_entry_events(self.original_events.iter_mut().chain(self.records.iter_mut()
            .flat_map(|record| record.outcome.events.iter_mut())))?;
        for record in &mut self.records {
            if let Some(completion) = &mut record.completion { completion.freeze(game)?; }
        }
        Ok(())
    }
    fn prepare_draw_boundary(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        _original: EffectOutcome) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.run(game, ctx, true)
    }
    fn complete(self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext,
        _original: EffectOutcome) -> Result<EffectOutcome, ExecutionError> {
        self.run(game, ctx, false).map(|committed| committed.outcome)
    }
}

pub(super) fn finish_original(mut committed: SimultaneousEffectCommit,
    game: &mut GameState, ctx: &mut ExecutionContext, defer_draws: bool)
    -> Result<SimultaneousEffectCommit, ExecutionError> {
    if let Some(mut completion) = committed.completion.take() {
        completion.freeze(game)?;
        if defer_draws { completion.prepare_draw_boundary(game, ctx, committed.outcome) }
        else { completion.complete(game, ctx, committed.outcome).map(SimultaneousEffectCommit::finished) }
    } else { Ok(committed) }
}

#[cfg(test)]
#[path = "object_iteration_tests.rs"]
mod tests;

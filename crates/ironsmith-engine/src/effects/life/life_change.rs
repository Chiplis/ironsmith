//! Commit life changes from their resolved event, never the authored amount/player.

use crate::effect::{EffectOutcome, OutcomeValue};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::{TraitEventResult, process_trait_event_with_execution_context};
use crate::events::{Event, LifeGainEvent, LifeLossEvent, downcast_event};
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;

/// Frozen operands of one life action. Adapters resolve their authored inputs;
/// this owner retains the proposal, prepares replacements and commits receipts.
#[derive(Debug)]
pub(crate) struct LifeChangeProposal {
    player: crate::ids::PlayerId,
    amount: u32,
    kind: LifeProposalKind,
    prepared: Option<TraitEventResult>,
}

#[derive(Debug, Clone, Copy)]
enum LifeProposalKind {
    Gain,
    Loss {
        provenance: crate::provenance::ProvNodeId,
        can_change_life_total: bool,
    },
}

impl LifeChangeProposal {
    pub(crate) fn gain(player: crate::ids::PlayerId, amount: u32) -> Self {
        Self {
            player,
            amount,
            kind: LifeProposalKind::Gain,
            prepared: None,
        }
    }

    pub(crate) fn loss(
        player: crate::ids::PlayerId,
        amount: u32,
        provenance: crate::provenance::ProvNodeId,
        can_change_life_total: bool,
    ) -> Self {
        Self {
            player,
            amount,
            kind: LifeProposalKind::Loss {
                provenance,
                can_change_life_total,
            },
            prepared: None,
        }
    }

    /// Preserve each adapter's attribution policy. Gain uses its executing
    /// source scope; loss retains the proposal's original provenance.
    pub(crate) fn event(&self, ctx: &ExecutionContext) -> Event {
        match self.kind {
            LifeProposalKind::Gain => Event::new_with_provenance(
                LifeGainEvent::new(self.player, self.amount).with_source(ctx.source),
                ctx.provenance,
            ),
            LifeProposalKind::Loss { provenance, .. } => Event::new_with_provenance(
                LifeLossEvent::from_effect(self.player, self.amount),
                provenance,
            ),
        }
    }
}

impl crate::effects::SimultaneousEffectProposal for LifeChangeProposal {
    fn nominal_payment_quantity(&self) -> Option<u64> {
        match self.kind {
            LifeProposalKind::Gain => Some(u64::from(self.amount)),
            LifeProposalKind::Loss { .. } => None,
        }
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self
            .prepared
            .as_ref()
            .is_some_and(|original| !original.requires_replacement_input())
        {
            return Ok(());
        }
        self.prepared = Some(match self.kind {
            LifeProposalKind::Loss {
                can_change_life_total: false,
                ..
            } => TraitEventResult::Prevented,
            _ => prepare_life_change(game, ctx, self.event(ctx))?,
        });
        Ok(())
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<
        crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
        ExecutionError,
    > {
        if self.prepared.is_none() {
            self.prepare_original(game, ctx)?;
        }
        commit_prepared_life_original_with_outputs(
            game,
            ctx,
            self.prepared.take().expect("life proposal prepared"),
        )
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.commit_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
}

impl LifeChangeProposal {
    /// Ordinary loss keeps its existing early exits; pre-prepared proposals
    /// retain their outputs through the same original coordinator.
    pub(crate) fn commit_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if self.prepared.is_some() {
            return crate::effects::composition::complete_prepared_original_with_outputs(
                self, game, ctx, false,
            );
        }
        // Preserve the loss adapter's ordinary early-exit ordering. Prepared
        // originals instead project their resolved event through the owner.
        if let LifeProposalKind::Loss {
            can_change_life_total,
            ..
        } = self.kind
        {
            if !can_change_life_total {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::prevented(),
                ));
            }
            if game.player(self.player).is_none() {
                return Err(ExecutionError::PlayerNotFound(self.player));
            }
            if self.amount == 0 {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
        }
        execute_life_change_with_outputs(game, ctx, self.event(ctx))
    }
}

fn prevented_life_change() -> EffectOutcome {
    let mut outcome = EffectOutcome::prevented();
    outcome.value = OutcomeValue::Count(0);
    outcome
}

/// Keep replacement choices and payload execution in the same transaction.
/// The decision maker retains its prompt/answers, but no game mutation or
/// emitted result escapes a suspended or failed operation.
pub(crate) fn execute_life_change(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    execute_life_changes(game, ctx, vec![event])
}

/// Retain the completed child owners through the same life instruction.
pub(crate) fn execute_life_change_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    LifeChanges(vec![event]).execute_child_with_outputs(game, ctx)
}

/// Resolve every life proposal against the pre-commit state. In particular,
/// the first side of an exchange must not affect the second side's matching.
pub(crate) fn execute_life_changes(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    events: Vec<Event>,
) -> Result<EffectOutcome, ExecutionError> {
    LifeChanges(events).execute_child(game, ctx)
}

#[derive(Debug, Clone)]
struct LifeChanges(Vec<Event>);

impl EffectExecutor for LifeChanges {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let simultaneous = self.0.len() > 1;
                let prepared = prepare_life_changes(game, ctx, self.0.clone())?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                let outcomes = crate::effects::composition::execute_simultaneous_originals_with_default_outputs(
                game,
                ctx,
                simultaneous,
                |game, ctx| commit_prepared_life_changes_with_outputs(game, ctx, prepared),
            )?;
                if ctx.decision_maker.awaiting_choice() {
                    return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                        EffectOutcome::count(0),
                    ));
                }
                crate::events::damage::checked_damage_count(
                    outcomes
                        .iter()
                        .filter_map(|outcome| outcome.outcome.instruction_result().as_count())
                        .map(|count| count.max(0) as u128)
                        .sum(),
                    "simultaneous life outcome total",
                )?;
                let aggregate = EffectOutcome::aggregate_summing_counts(
                    outcomes.iter().map(|outputs| outputs.outcome.clone()),
                );
                let mut outputs = crate::effects::CompletedEffectOutputs::aggregate_only(aggregate);
                outputs.projections_complete = !outcomes.is_empty();
                for child in outcomes {
                    outputs = outputs.append_owned_child(child);
                }
                Ok(outputs)
            },
        )
    }
}

/// The same preparation owner is used by ordinary life batches and compounds
/// that combine life changes with other original actions.
pub(crate) fn prepare_life_changes(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut events: Vec<Event>,
) -> Result<Vec<TraitEventResult>, ExecutionError> {
    let order = game.team_apnap_player_order();
    events.sort_by_key(|event| {
        let player = event.inner().affected_player(game);
        order
            .iter()
            .position(|candidate| *candidate == player)
            .unwrap_or(usize::MAX)
    });
    let mut prepared = Vec::with_capacity(events.len());
    for event in events {
        prepared.push(prepare_life_change(game, ctx, event)?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
    }
    Ok(prepared)
}

pub(crate) fn commit_prepared_life_changes(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: Vec<TraitEventResult>,
) -> Result<Vec<crate::effects::SimultaneousEffectCommit>, ExecutionError> {
    commit_prepared_life_changes_with_outputs(game, ctx, prepared).map(|receipts| {
        receipts
            .into_iter()
            .map(crate::effects::SimultaneousEffectCommit::into_aggregate)
            .collect()
    })
}

pub(crate) fn commit_prepared_life_changes_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: Vec<TraitEventResult>,
) -> Result<
    Vec<crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>>,
    ExecutionError,
> {
    let mut committed = Vec::with_capacity(prepared.len());
    for proposal in prepared {
        committed.push(commit_prepared_life_original_with_outputs(
            game, ctx, proposal,
        )?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(Vec::new());
        }
    }
    Ok(committed)
}

pub(crate) fn commit_prepared_life_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: TraitEventResult,
) -> Result<
    crate::effects::SimultaneousEffectCommit<crate::effects::CompletedEffectOutputs>,
    ExecutionError,
> {
    let (original, programs) = prepared.into_expansion();
    let deferred = if let TraitEventResult::Replaced {
        effects,
        source,
        controller,
        context,
        ..
    } = &original
    {
        crate::effects::replacement::prepare_draw_continuation_with_outputs(
            game,
            ctx,
            effects,
            *source,
            *controller,
            context,
        )?
    } else {
        None
    };
    let (outcome, original_continuation) = if let Some(receipt) = deferred {
        (receipt.outcome, receipt.completion)
    } else {
        (commit_life_change_with_outputs(game, ctx, original)?, None)
    };
    Ok(
        crate::effects::replacement::defer_replacement_programs_with_outputs(
            crate::effects::SimultaneousEffectCommit {
                outcome,
                completion: original_continuation,
            },
            programs,
            |_| {
                Ok(crate::effects::replacement::ReplacementProgramBindings {
                    targets: None,
                    object_tags: Vec::new(),
                })
            },
        ),
    )
}

pub(crate) fn prepare_life_change(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<TraitEventResult, ExecutionError> {
    let (player, amount, allowed) =
        if let Some(gain) = downcast_event::<LifeGainEvent>(event.inner()) {
            (gain.player, gain.amount, game.can_gain_life(gain.player))
        } else if let Some(loss) = downcast_event::<LifeLossEvent>(event.inner()) {
            (loss.player, loss.amount, game.can_lose_life(loss.player))
        } else {
            return Err(ExecutionError::InternalError(
                "life change requires a life event".into(),
            ));
        };
    if game.player(player).is_none() {
        return Err(ExecutionError::PlayerNotFound(player));
    }
    if amount == 0 {
        return Ok(TraitEventResult::Proceed(event));
    }
    if !allowed {
        return Ok(TraitEventResult::Prevented);
    }
    process_trait_event_with_execution_context(game, event, ctx)
}

fn check_life_representation(
    game: &GameState,
    player: crate::PlayerId,
    amount: u32,
    gain: bool,
) -> Result<(), ExecutionError> {
    crate::events::damage::checked_damage_count(u128::from(amount), "life change outcome")?;
    let current = game
        .player(player)
        .ok_or(ExecutionError::PlayerNotFound(player))?
        .life;
    let next = i64::from(current)
        + if gain {
            i64::from(amount)
        } else {
            -i64::from(amount)
        };
    i32::try_from(next).map_err(|_| ExecutionError::ResourceLimitExceeded {
        resource: "signed life total magnitude",
        requested: u128::from(next.unsigned_abs()),
        maximum: if next < 0 {
            1u128 << 31
        } else {
            i32::MAX as u128
        },
    })?;
    Ok(())
}

fn commit_life_change_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    result: TraitEventResult,
) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
    match result {
        expanded @ TraitEventResult::Expanded { .. } => {
            crate::effects::replacement::execute_event_expansion_with_outputs(
                game,
                ctx,
                expanded,
                commit_life_change_with_outputs,
                |_, _, _| {
                    Ok(crate::effects::replacement::ReplacementProgramBindings {
                        targets: None,
                        object_tags: Vec::new(),
                    })
                },
            )
        }
        TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
            commit_resolved_life_change(game, ctx, event)
                .map(crate::effects::CompletedEffectOutputs::aggregate_only)
        }
        TraitEventResult::Replaced {
            effects,
            source,
            controller,
            context,
            ..
        } => {
            // Preserve the original life acknowledgement while retaining the payload's owners.
            let mut original = EffectOutcome::replaced();
            original.set_value(OutcomeValue::Count(0));
            crate::effects::replacement::execute_replacement_original_payload_with_outputs(
                game,
                ctx,
                &effects,
                source,
                controller,
                &context,
                crate::effects::replacement::ReplacementProgramBindings {
                    targets: None,
                    object_tags: Vec::new(),
                },
                None,
                original,
            )
        }
        TraitEventResult::Prevented => Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
            prevented_life_change(),
        )),
        TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
            if ctx.decision_maker.awaiting_choice() {
                Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ))
            } else {
                Err(ExecutionError::InternalError(
                    "life replacement suspended without a captured decision".into(),
                ))
            }
        }
    }
}

fn commit_resolved_life_change(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    event: Event,
) -> Result<EffectOutcome, ExecutionError> {
    let provenance = event.provenance();
    let (actual, mut notification) = if let Some(gain) =
        downcast_event::<LifeGainEvent>(event.inner())
    {
        if gain.amount == 0 {
            return Ok(EffectOutcome::count(0));
        }
        if game.can_gain_life(gain.player) {
            check_life_representation(game, gain.player, gain.amount, true)?;
        }
        let actual = game.gain_life(gain.player, gain.amount);
        (
            actual,
            TriggerEvent::new_with_provenance(gain.with_amount(actual), provenance),
        )
    } else if let Some(loss) = downcast_event::<LifeLossEvent>(event.inner()) {
        if loss.amount == 0 {
            return Ok(EffectOutcome::count(0));
        }
        let amount = if loss.from_damage && game.damage_cant_reduce_life_below_one(loss.player) {
            let life = game
                .player(loss.player)
                .ok_or(ExecutionError::PlayerNotFound(loss.player))?
                .life;
            loss.amount
                .min(u32::try_from(life.saturating_sub(1).max(0)).unwrap_or(0))
        } else {
            loss.amount
        };
        if game.can_lose_life(loss.player) {
            check_life_representation(game, loss.player, amount, false)?;
        }
        let actual = game.lose_life(loss.player, amount);
        (
            actual,
            TriggerEvent::new_with_provenance(loss.with_amount(actual), provenance),
        )
    } else {
        return Err(ExecutionError::InternalError(
            "life replacement returned an incompatible event".into(),
        ));
    };
    if actual == 0 {
        return Ok(prevented_life_change());
    }
    // A proposal can produce several physical life changes. Each
    // committed observation has its own identity under that proposal.
    let observation = game.alloc_child_event_provenance(provenance, notification.kind());
    notification = notification.with_provenance(observation);
    if let Some(batch) = game.simultaneous_action_batch() {
        notification = notification.with_simultaneous_batch(batch);
    }
    if game.object(ctx.source).is_none()
        && let Some(snapshot) = &ctx.source_snapshot
    {
        notification = notification.with_source_snapshot(snapshot.clone());
    }
    Ok(
        EffectOutcome::count(crate::events::damage::checked_damage_count(
            u128::from(actual),
            "life change outcome",
        )?)
        .with_event(notification),
    )
}

#[cfg(test)]
mod removed_life_operation_tests {
    use super::*;
    fn check_removed_life_change(gain: bool) {
        struct PreferSubtractor(crate::ids::ObjectId);
        impl crate::decision::DecisionMaker for PreferSubtractor {
            fn decide_options(&mut self, _game: &GameState, ctx: &crate::decisions::context::SelectOptionsContext) -> Vec<usize> {
                let choice = ctx.options.iter().find(|option| option.legal && option.object_id == Some(self.0))
                    .or_else(|| ctx.options.iter().find(|option| option.legal)).unwrap();
                vec![choice.index]
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Life replacement source")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let subtractor = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let adder = game.create_object_from_definition(&definition, alice, crate::zone::Zone::Battlefield);
        let effect = |source, modification| {
            let action = crate::replacement::ReplacementAction::Modify(modification);
            if gain {
                crate::replacement::ReplacementEffect::with_matcher(source, alice,
                    crate::events::life::matchers::WouldGainLifeMatcher::any_player(), action)
            } else {
                crate::replacement::ReplacementEffect::with_matcher(source, alice,
                    crate::events::life::matchers::WouldLoseLifeMatcher::any_player(), action)
            }
        };
        game.effect_store.replacement_effects.add_resolution_effect(effect(subtractor, crate::replacement::EventModification::Subtract(1)));
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(effect(adder, crate::replacement::EventModification::Add(2)));
        let mut chooser = PreferSubtractor(subtractor);
        for count in [0, 1, 2] {
            game.take_pending_trigger_events();
            let before = game.player(alice).unwrap().life;
            let event = if gain { Event::new_with_provenance(LifeGainEvent::new(alice, count).with_source(subtractor), Default::default()) }
                else { Event::life_loss(alice, count, false) };
            let mut ctx = ExecutionContext::new(subtractor, alice, &mut chooser);
            let outcome = execute_life_change(&mut game, &mut ctx, event).unwrap();
            let amount = if count == 2 { 3 } else { 0 };
            assert_eq!(outcome.value, OutcomeValue::Count(amount));
            assert_eq!(i64::from(game.player(alice).unwrap().life), i64::from(before) + if gain { amount } else { -amount });
            assert_eq!(outcome.events.len(), usize::from(count == 2));
            assert!(game.take_pending_trigger_events().is_empty(), "owner returns notifications for its caller to publish");
            assert_eq!(game.effect_store.replacement_effects.get_effect(shield).is_some(), count != 2,
                "absent life operation must preserve later one-shot until a positive change");
        }
    }
    #[test]
    fn life_gain_reduced_to_zero_does_not_revive_or_consume_later_one_shot() { check_removed_life_change(true); }
    #[test]
    fn life_loss_reduced_to_zero_does_not_revive_or_consume_later_one_shot() { check_removed_life_change(false); }
}

#[cfg(test)]
mod simultaneous_life_preparation_tests {
    use super::*;
    use crate::effects::{EffectExecutor, ForPlayersEffect, GainLifeEffect, LoseLifeEffect};
    use crate::target::PlayerFilter;
    #[derive(Debug)]
    #[derive(Clone)]
    struct WhileFirstPlayerAtTwenty;
    impl crate::events::ReplacementMatcher for WhileFirstPlayerAtTwenty {
        fn matches_prepared_event(&self, event: &dyn crate::events::GameEventType, ctx: &crate::events::context::PreparedEventContext) -> bool {
            matches!(event.event_kind(), crate::events::EventKind::LifeGain | crate::events::EventKind::LifeLoss)
                && ctx.game.player(crate::ids::PlayerId(0)).is_some_and(|player| player.life == 20)
        }
        fn display(&self) -> String { "while first player's life is twenty".into() }
    }
    #[test]
    fn each_player_life_replacement_eligibility_uses_the_shared_precommit_world() {
        for gained in [false, true] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let a = crate::ids::PlayerId(0); let b = crate::ids::PlayerId(1);
            let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Life replacement witness")
                .card_types(vec![crate::types::CardType::Artifact]).build();
            let source = game.create_object_from_definition(&definition, a, crate::zone::Zone::Battlefield);
            game.effect_store.replacement_effects.add_resolution_effect(crate::replacement::ReplacementEffect::with_matcher(
                source, a, WhileFirstPlayerAtTwenty, crate::replacement::ReplacementAction::Double));
            let effect = if gained { crate::effect::Effect::new(GainLifeEffect::with_filter(1, PlayerFilter::IteratedPlayer)) }
                else { crate::effect::Effect::new(LoseLifeEffect::with_filter(1, PlayerFilter::IteratedPlayer)) };
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(source, a, &mut dm);
            ForPlayersEffect::new(PlayerFilter::Any, vec![effect]).execute(&mut game, &mut ctx).unwrap();
            for player in [a, b] { assert_eq!(game.player(player).unwrap().life, if gained { 22 } else { 18 }); }
        }
    }
}

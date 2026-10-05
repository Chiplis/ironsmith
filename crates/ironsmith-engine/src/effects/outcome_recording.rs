//! Typed, immutable result evidence shared by every effect execution.
//!
//! Queue receipts are captured when committed, before trigger matching can drain
//! them. Nested executions own separate scopes; replacement additions therefore
//! cannot become evidence for the instruction they replace.
use crate::effect::{EffectOutcome, ExecutionFact, PriorEffectAction};
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

fn record(
    action: PriorEffectAction,
    player: Option<PlayerId>,
    objects: Vec<ObjectSnapshot>,
) -> ExecutionFact {
    ExecutionFact::ActionObjects {
        action,
        player,
        objects,
    }
}

pub(crate) fn event_facts(game: &GameState, event: &TriggerEvent) -> Vec<ExecutionFact> {
    use crate::events::*;
    use PriorEffectAction as A;
    let mut facts = Vec::new();
    if let Some(draw) = event.downcast::<CardsDrawnEvent>() {
        facts.push(record(A::Drawn, Some(draw.player), draw.snapshots.clone()));
    } else if let Some(zone) = event.downcast::<ZoneChangeEvent>() {
        let snapshots = zone.snapshots().to_vec();
        let action = match zone.to {
            Zone::Hand => Some(A::Returned),
            Zone::Graveyard => Some(A::PutIntoGraveyard),
            Zone::Battlefield => Some(A::PutOntoBattlefield),
            Zone::Exile => Some(A::Exiled),
            // Moving a card to the library does not establish a shuffle.
            Zone::Library => None,
            _ => None,
        };
        if let Some(action) = action {
            facts.push(record(action, None, snapshots.clone()));
        }
        if zone.from == Zone::Battlefield && zone.to == Zone::Graveyard {
            facts.push(record(A::Died, None, snapshots.clone()));
        }
        if zone.to == Zone::Hand {
            let mut players = std::collections::BTreeMap::<PlayerId, Vec<ObjectSnapshot>>::new();
            for snapshot in &zone.destination_snapshots {
                players
                    .entry(snapshot.owner)
                    .or_default()
                    .push(snapshot.clone());
            }
            // Arrival snapshots can be absent on non-battlefield moves. These
            // identities are still read at queue time, before any added program.
            if players.is_empty() {
                for id in &zone.result_objects {
                    if let Some(snapshot) = ObjectSnapshot::from_object_id(game, *id) {
                        players.entry(snapshot.owner).or_default().push(snapshot);
                    }
                }
            }
            for (player, cards) in players {
                facts.push(ExecutionFact::CardsPutIntoHand {
                    player,
                    cards: cards.clone(),
                });
                facts.push(record(A::PutIntoHand, Some(player), cards));
            }
        }
    } else if let Some(discard) = event.downcast::<CardDiscardedEvent>() {
        facts.push(record(
            A::Discarded,
            Some(discard.player),
            if discard.batch_snapshots.is_empty() {
                discard.snapshot.iter().cloned().collect()
            } else {
                discard.batch_snapshots.clone()
            },
        ));
    } else if let Some(sacrifice) = event.downcast::<SacrificeEvent>() {
        facts.push(record(
            A::Sacrificed,
            sacrifice.sacrificing_player,
            sacrifice.snapshot.iter().cloned().collect(),
        ));
    } else if let Some(reveal) = event.downcast::<CardRevealedEvent>() {
        facts.push(record(
            A::Revealed,
            Some(reveal.player),
            reveal.snapshot.iter().cloned().collect(),
        ));
    } else if let Some(cast) = event.downcast::<crate::events::spells::SpellCastEvent>() {
        facts.push(record(
            A::Cast,
            Some(cast.caster),
            cast.snapshot.iter().cloned().collect(),
        ));
    } else if let Some(counter) = event.downcast::<SpellCounteredEvent>() {
        facts.push(record(
            A::Countered,
            Some(counter.controller),
            counter.snapshot.iter().cloned().collect(),
        ));
    } else if let Some(damage) = event.downcast::<DamageEvent>() {
        if damage.amount > 0 {
            facts.push(record(
                A::DealtDamage,
                None,
                damage.target_snapshot.iter().cloned().collect(),
            ));
        }
    } else if let Some(prevented) = event.downcast::<DamagePreventedEvent>() {
        facts.push(ExecutionFact::PreventedDamageReceipt {
            receipt: event.provenance(),
            amount: prevented.amount,
        });
    } else if let Some(tap) = event.downcast::<PermanentTappedEvent>() {
        facts.push(record(
            A::Tapped,
            tap.actor,
            tap.snapshot.iter().cloned().collect(),
        ));
    }
    facts
}

/// None means that this action's object projection was never recorded. Some([])
/// means it was recorded and affected nothing; it must not fall back to another
/// action's memory or to mutable live game objects.
pub(crate) fn action_objects(
    outcome: &EffectOutcome,
    action: PriorEffectAction,
    players: Option<&[PlayerId]>,
) -> Option<Vec<ObjectSnapshot>> {
    let mut known = false;
    let mut objects = Vec::new();
    for fact in &outcome.instruction_result().execution_facts {
        if let ExecutionFact::ActionObjects {
            action: recorded,
            player,
            objects: snapshots,
        } = fact
        {
            if *recorded != action {
                continue;
            }
            known = true;
            if players
                .is_some_and(|players| player.is_some_and(|player| !players.contains(&player)))
            {
                continue;
            }
            for snapshot in snapshots {
                if player.is_none()
                    && players.is_some_and(|players| !players.contains(&snapshot.controller))
                {
                    continue;
                }
                if !objects
                    .iter()
                    .any(|old: &ObjectSnapshot| old.object_id == snapshot.object_id)
                {
                    objects.push(snapshot.clone());
                }
            }
        }
    }
    known.then_some(objects)
}

pub(crate) fn complete_outcome(
    game: &GameState,
    action: Option<PriorEffectAction>,
    actor: Option<PlayerId>,
    outcome: &mut EffectOutcome,
    mut recorded: Vec<ExecutionFact>,
) {
    if let Some(original) = outcome.instruction_result.as_mut() {
        complete_outcome(game, action, actor, original, recorded);
        return;
    }
    let queued_prevention = recorded
        .iter()
        .chain(&outcome.execution_facts)
        .any(|fact| matches!(fact, ExecutionFact::PreventedDamageReceipt { .. }));
    for event in &outcome.events {
        for fact in event_facts(game, event) {
            // Queued receipts have a unique notification identity. Reporting
            // the same prevention after queuing must not count it twice.
            if queued_prevention && matches!(fact, ExecutionFact::PreventedDamageReceipt { .. }) {
                continue;
            }
            if let ExecutionFact::ActionObjects { action, .. } = &fact {
                if action_objects(outcome, *action, None).is_some() {
                    continue;
                }
            }
            recorded.push(fact);
        }
    }
    let explicit_actions = outcome
        .execution_facts
        .iter()
        .filter_map(|fact| match fact {
            ExecutionFact::ActionObjects { action, .. } => Some(*action),
            _ => None,
        })
        .collect::<Vec<_>>();
    for fact in recorded {
        if matches!(&fact, ExecutionFact::ActionObjects { action, .. } if explicit_actions.contains(action))
        {
            continue;
        }
        if !outcome.execution_facts.contains(&fact) {
            outcome.execution_facts.push(fact);
        }
    }
    if let Some(action) = action {
        // These actions are performed by the resolving instruction's player,
        // rather than the controller/owner of the affected object. Draw,
        // discard, reveal and sacrifice retain their event's actual player.
        if matches!(
            action,
            PriorEffectAction::Countered
                | PriorEffectAction::Destroyed
                | PriorEffectAction::Exiled
                | PriorEffectAction::Returned
                | PriorEffectAction::Goaded
        ) {
            for fact in &mut outcome.execution_facts {
                if let ExecutionFact::ActionObjects {
                    action: recorded,
                    player,
                    ..
                } = fact
                    && *recorded == action
                {
                    *player = actor;
                }
            }
        }
        // Explicit producers remain authoritative for operations without a
        // domain event (goad), and for semantics not inferable from a zone move
        // (destroy, mill). Never substitute replacement facts for a draw.
        if action_objects(outcome, action, None).is_none() {
            let objects = if action == PriorEffectAction::Drawn {
                Vec::new()
            } else {
                outcome
                    .affected_object_memory()
                    .map(<[ObjectSnapshot]>::to_vec)
                    .unwrap_or_else(|| {
                        capture_ids(
                            game,
                            outcome,
                            outcome
                                .affected_objects()
                                .or_else(|| outcome.result_objects())
                                .or_else(|| outcome.explicit_objects())
                                .unwrap_or(&[]),
                        )
                    })
            };
            let player = if matches!(
                action,
                PriorEffectAction::Countered
                    | PriorEffectAction::Destroyed
                    | PriorEffectAction::Exiled
                    | PriorEffectAction::Returned
                    | PriorEffectAction::Goaded
            ) {
                actor
            } else {
                None
            };
            outcome
                .execution_facts
                .push(record(action, player, objects));
        }
        if let Some(objects) = action_objects(outcome, action, None) {
            outcome.execution_facts.retain(|fact| {
                !matches!(
                    fact,
                    ExecutionFact::AffectedObjectMemory(_) | ExecutionFact::AffectedObjects(_)
                )
            });
            outcome.execution_facts.push(ExecutionFact::AffectedObjects(
                objects.iter().map(|object| object.object_id).collect(),
            ));
            outcome
                .execution_facts
                .push(ExecutionFact::AffectedObjectMemory(objects));
        }
    } else if outcome.affected_object_memory().is_none() {
        let mut objects = Vec::new();
        for fact in &outcome.execution_facts {
            if let ExecutionFact::ActionObjects {
                objects: captured, ..
            } = fact
            {
                for snapshot in captured {
                    if !objects
                        .iter()
                        .any(|old: &ObjectSnapshot| old.stable_id == snapshot.stable_id)
                    {
                        objects.push(snapshot.clone());
                    }
                }
            }
        }
        if objects.is_empty() {
            objects = outcome
                .affected_objects()
                .unwrap_or(&[])
                .iter()
                .filter_map(|id| ObjectSnapshot::from_object_id(game, *id))
                .collect();
        }
        if !objects.is_empty() {
            outcome
                .execution_facts
                .push(ExecutionFact::AffectedObjectMemory(objects));
        }
    }
    if outcome.chosen_object_memory().is_none()
        && let Some(ids) = outcome.chosen_objects()
    {
        let snapshots = capture_ids(game, outcome, ids);
        outcome
            .execution_facts
            .push(ExecutionFact::ChosenObjectMemory(snapshots));
    }
    if outcome.result_object_memory().is_none()
        && let Some(ids) = outcome
            .explicit_objects()
            .or_else(|| outcome.result_objects())
    {
        let snapshots = capture_ids(game, outcome, ids);
        outcome
            .execution_facts
            .push(ExecutionFact::ResultObjectMemory(snapshots));
    }
}

fn capture_ids(
    game: &GameState,
    outcome: &EffectOutcome,
    ids: &[crate::ids::ObjectId],
) -> Vec<ObjectSnapshot> {
    ids.iter()
        .filter_map(|id| {
            outcome
                .execution_facts
                .iter()
                .find_map(|fact| {
                    let snapshots = match fact {
                        ExecutionFact::ActionObjects { objects, .. } => objects.as_slice(),
                        ExecutionFact::ChosenObjectMemory(objects)
                        | ExecutionFact::AffectedObjectMemory(objects) => objects.as_slice(),
                        _ => return None,
                    };
                    snapshots
                        .iter()
                        .find(|snapshot| snapshot.object_id == *id)
                        .cloned()
                })
                .or_else(|| ObjectSnapshot::from_object_id(game, *id))
        })
        .collect()
}

#[derive(Debug)]
struct RecordedProposal {
    inner: Box<dyn crate::effects::SimultaneousEffectProposal>,
    action: Option<PriorEffectAction>,
}

pub(crate) fn record_proposal(
    inner: Box<dyn crate::effects::SimultaneousEffectProposal>,
    action: Option<PriorEffectAction>,
) -> Box<dyn crate::effects::SimultaneousEffectProposal> {
    Box::new(RecordedProposal { inner, action })
}

impl crate::effects::SimultaneousEffectProposal for RecordedProposal {
    fn declared_life_payment(&self) -> Option<(PlayerId, u32)> {
        self.inner.declared_life_payment()
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<(), crate::effects::ExecutionError> {
        self.inner.prepare_original(game, ctx)
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<crate::effects::SimultaneousEffectCommit, crate::effects::ExecutionError> {
        game.effect_store
            .instruction_result_records
            .push(Vec::new());
        let mut result = self.inner.commit_original(game, ctx);
        let recorded = game
            .effect_store
            .instruction_result_records
            .pop()
            .unwrap_or_default();
        if let Ok(committed) = &mut result {
            if !ctx.decision_maker.awaiting_choice() {
                let had_result_memory = committed.outcome.result_object_memory().is_some();
                complete_outcome(
                    game,
                    self.action,
                    Some(ctx.controller),
                    &mut committed.outcome,
                    recorded,
                );
                if let Some(completion) = committed.completion.take() {
                    // Origin receipts are available now. Destination
                    // characteristics must wait until every simultaneous
                    // original commits and the completed entry view freezes.
                    if !had_result_memory {
                        clear_result_memory(&mut committed.outcome);
                    }
                    committed.completion = Some(Box::new(RecordedCompletion {
                        inner: completion,
                        action: self.action,
                        actor: ctx.controller,
                    }));
                }
            }
        }
        result
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
    ) -> Result<EffectOutcome, crate::effects::ExecutionError> {
        game.effect_store
            .instruction_result_records
            .push(Vec::new());
        let mut result = self.inner.commit(game, ctx);
        let recorded = game
            .effect_store
            .instruction_result_records
            .pop()
            .unwrap_or_default();
        if let Ok(outcome) = &mut result {
            if !ctx.decision_maker.awaiting_choice() {
                complete_outcome(game, self.action, Some(ctx.controller), outcome, recorded);
            }
        }
        result
    }
}

struct RecordedCompletion {
    inner: Box<dyn crate::effects::SimultaneousEffectCompletion>,
    action: Option<PriorEffectAction>,
    actor: PlayerId,
}
impl crate::effects::SimultaneousEffectCompletion for RecordedCompletion {
    fn freeze(&mut self, game: &mut GameState) -> Result<(), crate::effects::ExecutionError> {
        self.inner.freeze(game)
    }
    fn complete(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut crate::effects::ExecutionContext,
        mut original: EffectOutcome,
    ) -> Result<EffectOutcome, crate::effects::ExecutionError> {
        complete_outcome(
            game,
            self.action,
            Some(self.actor),
            &mut original,
            Vec::new(),
        );
        self.inner.complete(game, ctx, original)
    }
}

fn clear_result_memory(outcome: &mut EffectOutcome) {
    if let Some(original) = outcome.instruction_result.as_deref_mut() {
        clear_result_memory(original);
    } else {
        outcome
            .execution_facts
            .retain(|fact| !matches!(fact, ExecutionFact::ResultObjectMemory(_)));
    }
}

//! Prepare all damage results, commit their originals, then complete additions.
use super::{AppliedDamageAssignment, SourceDamageKeywords};
use crate::effects::{ExecutionContext, ExecutionError, OriginalEffectOutput, SimultaneousEffectCompletion};
use crate::events::DamageTarget;
use crate::events::processing::TraitEventResult;
use crate::game_state::GameState;
#[cfg(test)]
use crate::ids::ObjectId;
use crate::ids::StableId;
use crate::snapshot::ObjectSnapshot;
use crate::types::CardType;

enum PreparedConsequence {
    None,
    Life(TraitEventResult),
    Counters(crate::effects::counters::PreparedCounterPlacement),
}

/// One already replacement-adjusted damage assignment. `applied` and recipient
/// characteristics describe the damage occurrence before its results (CR 120.4b).
pub(crate) struct PreparedDamageAssignment {
    pub applied: bool,
    pub target_snapshot: Option<ObjectSnapshot>,
    target: DamageTarget,
    amount: u32,
    keywords: SourceDamageKeywords,
    creature: bool,
    removals: Vec<crate::effects::counters::PreparedCounterRemoval>,
    consequence: PreparedConsequence,
    ui_ids: Vec<StableId>,
}

pub(crate) struct DamageAssignmentReceipt<Output = crate::effect::EffectOutcome> {
    pub original: AppliedDamageAssignment<Output>,
    pub completion: Option<Box<dyn SimultaneousEffectCompletion>>,
    completion_frozen: bool,
}

impl DamageAssignmentReceipt<crate::effects::CompletedEffectOutputs> {
    fn into_aggregate(self) -> DamageAssignmentReceipt {
        DamageAssignmentReceipt {
            original: self.original.into_aggregate(),
            completion: self.completion,
            completion_frozen: self.completion_frozen,
        }
    }
}

/// Select replacements against the pre-commit state. This does not apply any
/// life change, counter placement, damage mark or added replacement program.
pub(crate) fn prepare_processed_damage_assignment(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    target: DamageTarget,
    amount: u32,
    keywords: SourceDamageKeywords,
) -> Result<PreparedDamageAssignment, ExecutionError> {
    let mut plan = PreparedDamageAssignment {
        applied: false,
        target_snapshot: None,
        target,
        amount,
        keywords,
        creature: false,
        removals: Vec::new(),
        consequence: PreparedConsequence::None,
        ui_ids: Vec::new(),
    };
    if amount == 0 {
        return Ok(plan);
    }
    let observed = game
        .continuous_query_snapshot()
        .map_err(ExecutionError::ContinuousDiscovery)?;
    if let Some(source) = ctx
        .source_snapshot
        .as_ref()
        .map(|source| source.stable_id)
        .or_else(|| observed.object(ctx.source).map(|source| source.stable_id))
    {
        plan.ui_ids.push(source);
    }
    let consequence = match target {
        DamageTarget::Player(player) => {
            if !observed
                .player(player)
                .is_some_and(|player| player.is_in_game())
            {
                return Ok(plan);
            }
            if keywords.has_infect {
                Some(
                    crate::events::Event::put_player_counters(
                        player,
                        crate::CounterType::Poison,
                        amount,
                        ctx.cause.clone(),
                    )
                    .with_provenance(ctx.provenance),
                )
            } else if observed.can_damage_cause_life_loss(player) {
                let event = crate::events::Event::life_loss(player, amount, true)
                    .with_provenance(ctx.provenance);
                plan.consequence = PreparedConsequence::Life(
                    crate::effects::life::life_change::prepare_life_change(game, ctx, event)?,
                );
                None
            } else {
                None
            }
        }
        DamageTarget::Object(object) => {
            let Some(object) = observed.object(object).filter(|object| {
                object.zone == crate::zone::Zone::Battlefield && !observed.is_phased_out(object.id)
            }) else {
                return Ok(plan);
            };
            let snapshot =
                ObjectSnapshot::from_object_with_calculated_characteristics(object, &observed);
            plan.creature = snapshot.card_types.contains(&CardType::Creature);
            let planeswalker = snapshot.card_types.contains(&CardType::Planeswalker);
            let battle = snapshot.card_types.contains(&CardType::Battle);
            if !plan.creature && !planeswalker && !battle {
                return Ok(plan);
            }
            plan.ui_ids.push(snapshot.stable_id);
            plan.target_snapshot = Some(snapshot);
            for (applies, kind) in [
                (planeswalker, crate::CounterType::Loyalty),
                (battle, crate::CounterType::Defense),
            ] {
                if applies {
                    let event = crate::events::Event::remove_counters(object.id, kind, amount)
                        .with_provenance(ctx.provenance);
                    plan.removals
                        .push(crate::effects::counters::prepare_counter_removal(
                            game, ctx, event,
                        )?);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(plan);
                    }
                }
            }
            if plan.creature && (keywords.has_infect || keywords.has_wither) {
                Some(
                    crate::events::Event::put_counters(
                        object.id,
                        crate::CounterType::MinusOneMinusOne,
                        amount,
                        ctx.cause.clone(),
                    )
                    .with_provenance(ctx.provenance),
                )
            } else {
                None
            }
        }
    };
    if let Some(event) = consequence {
        plan.consequence = PreparedConsequence::Counters(
            crate::effects::counters::prepare_counter_placement(game, ctx, event)?,
        );
    }
    crate::events::damage::checked_damage_count(u128::from(amount), "damage assignment outcome")?;
    plan.applied = true;
    Ok(plan)
}

/// Commit only the prepared original consequences. Damage legality/type is
/// frozen by the proposal, so an earlier simultaneous result cannot erase a
/// later occurrence. A departed object still cannot receive new counters/marks.
pub(crate) fn commit_prepared_damage_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedDamageAssignment,
) -> Result<DamageAssignmentReceipt, ExecutionError> {
    commit_prepared_damage_original_with_outputs(game, ctx, prepared)
        .map(DamageAssignmentReceipt::into_aggregate)
}

pub(crate) fn commit_prepared_damage_original_with_outputs(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    prepared: PreparedDamageAssignment,
) -> Result<DamageAssignmentReceipt<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    if !prepared.applied {
        return Ok(DamageAssignmentReceipt {
            original: AppliedDamageAssignment::default(),
            completion: None,
            completion_frozen: false,
        });
    }
    // The damage is being dealt (prevention already happened during
    // preparation): its source permanent now has a damage history.
    if prepared.amount > 0 {
        game.mark_dealt_damage_since_entered(ctx.source);
    }
    let mut consequences = Vec::new();
    for removal in prepared.removals {
        let receipt =
            crate::effects::counters::commit_prepared_counter_removal_original_with_outputs(
                game, ctx, removal,
            )?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(DamageAssignmentReceipt {
                original: AppliedDamageAssignment::default(),
                completion: None,
                completion_frozen: false,
            });
        }
        // Damage's low-level callers also publish these original notifications.
        // The batch observer deduplicates them by their occurrence provenance.
        for event in &receipt.outcome.outcome.events {
            let event = if let Some(batch) = game.simultaneous_action_batch() {
                event.clone().with_simultaneous_batch(batch)
            } else {
                event.clone()
            };
            game.queue_trigger_event(event.provenance(), event);
        }
        consequences.push(receipt);
    }
    if let DamageTarget::Object(object) = prepared.target
        && game
            .object(object)
            .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
        && !game.is_phased_out(object)
    {
        if prepared.creature && !prepared.keywords.has_infect && !prepared.keywords.has_wither {
            let total = crate::events::damage::checked_damage_amount(
                u128::from(game.damage_on(object)) + u128::from(prepared.amount),
                "marked damage",
            )?;
            game.set_damage_marked(object, total);
        }
        if prepared.creature && prepared.keywords.has_deathtouch {
            game.mark_deathtouch_damage_since_sba(object);
        }
    }
    let consequence = match prepared.consequence {
        PreparedConsequence::None => None,
        PreparedConsequence::Life(proposal) => Some(
            crate::effects::life::life_change::commit_prepared_life_original_with_outputs(
                game, ctx, proposal,
            )?,
        ),
        PreparedConsequence::Counters(proposal) => Some(
            crate::effects::counters::commit_prepared_counter_original_with_outputs(
                game, ctx, proposal,
            )?,
        ),
    };
    consequences.extend(consequence);
    let (consequence_outcome, completion) = if consequences.is_empty() {
        (None, None)
    } else {
        let receipt =
            crate::effects::composition::compose_original_commits_with_outputs(consequences);
        (Some(receipt.outcome), receipt.completion)
    };
    let life_lost = crate::events::damage::checked_damage_amount(
        consequence_outcome.as_ref().map_or(0, |outcome| {
            outcome
                .outcome
                .events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeLossEvent>())
                .filter(|event| event.from_damage)
                .map(|event| u128::from(event.amount))
                .sum()
        }),
        "damage life-loss receipt total",
    )?;
    game.record_ui_effect_event(
        "damage",
        match prepared.target {
            DamageTarget::Player(player) => Some(player),
            _ => None,
        },
        None,
        prepared.ui_ids,
        Some(i64::from(prepared.amount)),
        None,
    );
    Ok(DamageAssignmentReceipt {
        original: AppliedDamageAssignment {
            applied: true,
            life_lost,
            consequence_outcome,
        },
        completion,
        completion_frozen: false,
    })
}

/// Freeze each simultaneous receipt against the shared post-original state,
/// before any sibling's added program can mutate that state.
pub(crate) fn freeze_damage_original<Output>(
    game: &mut GameState,
    receipt: &mut DamageAssignmentReceipt<Output>,
) -> Result<(), ExecutionError> {
    if !receipt.completion_frozen {
        if let Some(completion) = &mut receipt.completion {
            completion.freeze(game)?;
        }
        receipt.completion_frozen = true;
    }
    Ok(())
}

/// Observe a frozen consequence in its own source context. Batch callers
/// observe every receipt before starting the first consequence continuation.
pub(crate) fn observe_damage_original<Output: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipt: &mut DamageAssignmentReceipt<Output>,
) -> Result<(), ExecutionError> {
    freeze_damage_original(game, receipt)?;
    if let Some(completion) = &mut receipt.completion {
        let original = receipt.original.consequence_outcome.get_or_insert_with(|| {
            Output::from_aggregate(crate::effect::EffectOutcome::resolved())
        });
        crate::effects::composition::observe_original_completion(
            game,
            ctx,
            completion.as_mut(),
            original.aggregate_mut(),
        )?;
    }
    Ok(())
}

/// Standalone damage consequences share the frozen-original observation phase.
pub(crate) fn complete_damage_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut receipt: DamageAssignmentReceipt,
) -> Result<AppliedDamageAssignment, ExecutionError> {
    observe_damage_original(game, ctx, &mut receipt)?;
    if ctx.decision_maker.awaiting_choice() {
        return Ok(receipt.original);
    }
    complete_observed_damage_original(game, ctx, receipt)
}

/// Complete only after the batch has frozen and observed every consequence.
pub(crate) fn complete_observed_damage_original(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    receipt: DamageAssignmentReceipt,
) -> Result<AppliedDamageAssignment, ExecutionError> {
    complete_observed_damage_original_with_outputs(game, ctx, receipt)
        .map(AppliedDamageAssignment::into_aggregate)
}

/// Retain supplied consequence packets at the existing post-observation boundary.
pub(crate) fn complete_observed_damage_original_with_outputs<Output: OriginalEffectOutput>(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    mut receipt: DamageAssignmentReceipt<Output>,
) -> Result<AppliedDamageAssignment<crate::effects::CompletedEffectOutputs>, ExecutionError> {
    freeze_damage_original(game, &mut receipt)?;
    let consequence_outcome = if receipt.completion.is_some()
        || receipt.original.consequence_outcome.is_some()
    {
        let original = receipt
            .original
            .consequence_outcome
            .take()
            .unwrap_or_else(|| Output::from_aggregate(crate::effect::EffectOutcome::resolved()));
        Some(
            crate::effects::composition::complete_committed_original_with_outputs(
                game,
                ctx,
                crate::effects::SimultaneousEffectCommit {
                    outcome: original,
                    completion: receipt.completion,
                },
            )?,
        )
    } else {
        None
    };
    Ok(AppliedDamageAssignment {
        applied: receipt.original.applied,
        life_lost: receipt.original.life_lost,
        consequence_outcome,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    fn setup() -> (GameState, ObjectId, crate::PlayerId, crate::PlayerId) {
        let mut game = GameState::new(vec!["A".into(), "B".into(), "C".into()], 30);
        let a = crate::PlayerId(0);
        let b = crate::PlayerId(1);
        let card = crate::card::CardBuilder::new(crate::CardId::new(), "Damage source")
            .card_types(vec![CardType::Artifact])
            .build();
        let source = game.create_object_from_card(&card, a, crate::zone::Zone::Battlefield);
        (game, source, a, b)
    }
    fn context(source: ObjectId, controller: crate::PlayerId) -> ExecutionContext<'static> {
        ExecutionContext::new_default(source, controller).with_cause(
            crate::events::cause::EventCause::from_effect(source, controller),
        )
    }
    #[test]
    fn every_completion_freezes_once_before_any_sibling_addition() {
        struct Observe {
            player: crate::PlayerId,
            frames: std::sync::Arc<std::sync::Mutex<Vec<i32>>>,
        }
        impl SimultaneousEffectCompletion for Observe {
            fn freeze(&mut self, game: &mut GameState) -> Result<(), ExecutionError> {
                self.frames
                    .lock()
                    .unwrap()
                    .push(game.player(self.player).unwrap().life);
                Ok(())
            }
            fn complete(
                self: Box<Self>,
                game: &mut GameState,
                _ctx: &mut ExecutionContext,
                original: crate::effect::EffectOutcome,
            ) -> Result<crate::effect::EffectOutcome, ExecutionError> {
                game.lose_life(self.player, 1);
                Ok(original)
            }
        }
        let (mut game, source, a, b) = setup();
        let frames = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut receipts = (0..2)
            .map(|_| DamageAssignmentReceipt {
                original: AppliedDamageAssignment::default(),
                completion: Some(Box::new(Observe {
                    player: b,
                    frames: frames.clone(),
                }) as Box<dyn SimultaneousEffectCompletion>),
                completion_frozen: false,
            })
            .collect::<Vec<_>>();
        for receipt in &mut receipts {
            freeze_damage_original(&mut game, receipt).unwrap();
        }
        let mut ctx = context(source, a);
        for receipt in receipts {
            complete_damage_original(&mut game, &mut ctx, receipt).unwrap();
        }
        assert_eq!(*frames.lock().unwrap(), vec![30, 30]);
        assert_eq!(game.player(b).unwrap().life, 28);
    }

    #[test]
    fn life_results_commit_all_originals_before_an_added_program_reads_later_player() {
        let (mut game, source, a, b) = setup();
        let c = crate::PlayerId(2);
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                b,
                crate::events::WouldLoseLifeMatcher::you(),
                ReplacementAction::Additionally(vec![Effect::gain_life(Value::LifeTotal(
                    PlayerFilter::Specific(c),
                ))]),
            ));
        let mut ctx = context(source, a);
        let first = prepare_processed_damage_assignment(
            &mut game,
            &mut ctx,
            DamageTarget::Player(b),
            3,
            SourceDamageKeywords::default(),
        )
        .unwrap();
        let second = prepare_processed_damage_assignment(
            &mut game,
            &mut ctx,
            DamageTarget::Player(c),
            4,
            SourceDamageKeywords::default(),
        )
        .unwrap();
        assert_eq!(game.player(b).unwrap().life, 30);
        assert_eq!(game.player(c).unwrap().life, 30);
        let first = commit_prepared_damage_original(&mut game, &mut ctx, first).unwrap();
        let second = commit_prepared_damage_original(&mut game, &mut ctx, second).unwrap();
        assert_eq!(first.original.life_lost, 3);
        assert_eq!(second.original.life_lost, 4);
        assert_eq!(game.player(b).unwrap().life, 27);
        assert_eq!(game.player(c).unwrap().life, 26);
        complete_damage_original(&mut game, &mut ctx, first).unwrap();
        complete_damage_original(&mut game, &mut ctx, second).unwrap();
        assert_eq!(
            game.player(b).unwrap().life,
            53,
            "replacement controller receives the later player's committed life total"
        );
    }
    #[test]
    fn wither_results_commit_later_recipient_before_first_addition_exiles_it() {
        let (mut game, source, a, b) = setup();
        let creature = crate::card::CardBuilder::new(crate::CardId::new(), "Counter recipient")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(8, 8))
            .build();
        let first = game.create_object_from_card(&creature, b, crate::zone::Zone::Battlefield);
        let second = game.create_object_from_card(&creature, b, crate::zone::Zone::Battlefield);
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                source,
                a,
                crate::events::counters::matchers::WouldPutCountersMatcher::new(
                    ObjectFilter::specific(first),
                    Some(crate::CounterType::MinusOneMinusOne),
                ),
                ReplacementAction::Additionally(vec![Effect::exile(ChooseSpec::SpecificObject(
                    second,
                ))]),
            ));
        let mut ctx = context(source, a);
        let keywords = SourceDamageKeywords {
            has_wither: true,
            ..Default::default()
        };
        let p1 = prepare_processed_damage_assignment(
            &mut game,
            &mut ctx,
            DamageTarget::Object(first),
            3,
            keywords,
        )
        .unwrap();
        let p2 = prepare_processed_damage_assignment(
            &mut game,
            &mut ctx,
            DamageTarget::Object(second),
            4,
            keywords,
        )
        .unwrap();
        let r1 = commit_prepared_damage_original(&mut game, &mut ctx, p1).unwrap();
        let r2 = commit_prepared_damage_original(&mut game, &mut ctx, p2).unwrap();
        assert_eq!(
            game.counter_count(first, crate::CounterType::MinusOneMinusOne),
            3
        );
        assert_eq!(
            game.counter_count(second, crate::CounterType::MinusOneMinusOne),
            4
        );
        assert!(game.object(second).is_some());
        complete_damage_original(&mut game, &mut ctx, r1).unwrap();
        let r2 = complete_damage_original(&mut game, &mut ctx, r2).unwrap();
        assert!(game.object(second).is_none());
        assert!(r2.applied);
        assert_eq!(r2.consequence_outcome.unwrap().count_or_zero(), 4);
    }
    #[test]
    fn damage_still_happens_when_infect_counter_result_is_prevented() {
        let (mut game, source, a, b) = setup();
        let mut replacement =
            crate::static_abilities::StaticAbility::double_player_counters_replacement(
                PlayerFilter::Specific(b),
                Some(crate::CounterType::Poison),
                "Prevent poison fixture".into(),
            )
            .generate_replacement_effect(source, a)
            .unwrap();
        replacement.replacement = ReplacementAction::Prevent;
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(replacement);
        let mut ctx = context(source, a);
        let prepared = prepare_processed_damage_assignment(
            &mut game,
            &mut ctx,
            DamageTarget::Player(b),
            3,
            SourceDamageKeywords {
                has_infect: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(prepared.applied);
        let receipt = commit_prepared_damage_original(&mut game, &mut ctx, prepared).unwrap();
        let actual = complete_damage_original(&mut game, &mut ctx, receipt).unwrap();
        assert!(actual.applied);
        assert_eq!(actual.life_lost, 0);
        assert_eq!(game.player(b).unwrap().life, 30);
        assert_eq!(game.player(b).unwrap().poison_counters, 0);
    }
}

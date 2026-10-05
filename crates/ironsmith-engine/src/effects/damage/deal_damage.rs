//! Deal damage effect implementation.
//!
//! This module implements the `DealDamage` effect, which deals damage to a target
//! creature, planeswalker, or player.
use crate::effect::ExecutionFact;
use crate::events::LifeGainEvent;
use crate::events::processing::ProcessedDamageResult;

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_nonnegative_u32, resolve_objects_for_effect, resolve_player_from_spec,
    resolve_players_from_spec, validate_target,
};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::events::DamageEvent;
use crate::events::DamageTarget;
use crate::events::LifeLossEvent;
use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
use crate::events::processing::SimultaneousDamageEvent;
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};
use crate::triggers::AttackEventTarget;
use crate::triggers::TriggerEvent;
use crate::types::CardType;
pub use ironsmith_core::DealDamageEffect;

/// Effect that deals damage to a target creature, planeswalker, or player.
///
/// # Fields
///
/// * `amount` - The amount of damage to deal (can be fixed or variable)
/// * `target` - The target specification (creature, player, or "any target")
/// * `source_is_combat` - Whether this damage is combat damage
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to any target (Lightning Bolt)
/// let effect = DealDamageEffect {
///     amount: Value::Fixed(3),
///     target: ChooseSpec::AnyTarget,
///     source_is_combat: false,
/// };
/// ```
pub(crate) fn apply_processed_damage_outcome(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    initial_target: DamageTarget,
    amount: u32,
    source_is_combat: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<EffectOutcome, ExecutionError> {
    apply_processed_damage_outcome_opts(
        game,
        source,
        source_snapshot,
        initial_target,
        amount,
        source_is_combat,
        false,
        provenance,
        cause,
        replacement_scope,
        dm,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_processed_damage_outcome_opts(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    initial_target: DamageTarget,
    amount: u32,
    source_is_combat: bool,
    unpreventable: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<EffectOutcome, ExecutionError> {
    let checkpoint = game.clone();
    let result = crate::events::processing::with_deferred_prevention_follow_up_outcome(
        game,
        dm,
        |game, dm| {
            let controller = game
                .current_controller(source)
                .or_else(|| source_snapshot.map(|snapshot| snapshot.controller))
                .unwrap_or(game.turn.active_player);
            let mut parent = ExecutionContext::new(source, controller, dm)
                .with_cause(cause.clone())
                .with_provenance(provenance);
            parent.source_snapshot = source_snapshot.cloned();
            parent.replacement = replacement_scope.clone();
            let batch = game.simultaneous_action_batch();
            super::multi_source_damage::commit_damage_batch(
                game,
                &mut parent,
                vec![SimultaneousDamageEvent {
                    source,
                    target: initial_target,
                    amount,
                    is_combat: source_is_combat,
                    unpreventable,
                    cause,
                    source_snapshot: source_snapshot.cloned(),
                }],
                batch,
            )
        },
    );
    let pending = dm.awaiting_choice();
    if pending || result.is_err() {
        game.restore_execution_checkpoint(checkpoint, result.is_ok() && pending);
    }
    if pending && result.is_ok() {
        return Ok(EffectOutcome::count(0));
    }
    result
}

fn apply_simultaneous_damage_outcome_opts(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    initial_targets: Vec<DamageTarget>,
    amount: u32,
    source_is_combat: bool,
    unpreventable: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<EffectOutcome, ExecutionError> {
    apply_simultaneous_damage_assignments_opts(
        game,
        source,
        source_snapshot,
        initial_targets
            .into_iter()
            .map(|target| (target, amount))
            .collect(),
        source_is_combat,
        unpreventable,
        provenance,
        cause,
        replacement_scope,
        dm,
    )
}

/// Deals possibly different amounts from one source to several recipients as
/// one simultaneous damage event (one batch, one lifelink gain: CR 702.15e).
#[allow(clippy::too_many_arguments)]
fn apply_simultaneous_damage_assignments_opts(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    assignments: Vec<(DamageTarget, u32)>,
    source_is_combat: bool,
    unpreventable: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<EffectOutcome, ExecutionError> {
    let checkpoint = game.clone();
    let result = crate::events::processing::with_deferred_prevention_follow_up_outcome(
        game,
        dm,
        |game, dm| {
            let controller = game
                .current_controller(source)
                .or_else(|| source_snapshot.map(|snapshot| snapshot.controller))
                .unwrap_or(game.turn.active_player);
            let mut parent = ExecutionContext::new(source, controller, dm)
                .with_cause(cause.clone())
                .with_provenance(provenance);
            parent.source_snapshot = source_snapshot.cloned();
            parent.replacement = replacement_scope.clone();
            let events = assignments
                .into_iter()
                .map(|(target, amount)| SimultaneousDamageEvent {
                    source,
                    target,
                    amount,
                    is_combat: source_is_combat,
                    unpreventable,
                    cause: cause.clone(),
                    source_snapshot: source_snapshot.cloned(),
                })
                .collect();
            let batch = game.simultaneous_action_batch().unwrap_or_else(|| {
                game.alloc_child_event_provenance(provenance, crate::events::EventKind::Damage)
            });
            super::multi_source_damage::commit_damage_batch(game, &mut parent, events, Some(batch))
        },
    );
    let pending = dm.awaiting_choice();
    if pending || result.is_err() {
        game.restore_execution_checkpoint(checkpoint, result.is_ok() && pending);
    }
    if pending && result.is_ok() {
        return Ok(EffectOutcome::count(0));
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn apply_processed_damage_results(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    processed_results: impl IntoIterator<Item = ProcessedDamageResult>,
    simultaneous_batch: Option<crate::provenance::ProvNodeId>,
    source_is_combat: bool,
    provenance: crate::provenance::ProvNodeId,
    cause: crate::events::cause::EventCause,
    replacement_scope: &crate::effects::ReplacementExecutionContext,
    dm: &mut dyn crate::decision::DecisionMaker,
) -> Result<EffectOutcome, ExecutionError> {
    let source_controller = game
        .object(source)
        .filter(|_| !game.is_phased_out(source))
        .map(|obj| game.controller_of(obj))
        .or_else(|| source_snapshot.map(|snapshot| snapshot.controller));

    let keywords = crate::rules::damage::source_damage_keywords(game, source, source_snapshot);
    let mut outcomes = Vec::new();
    let mut programs = Vec::new();
    let damage_source_snapshot = game
        .object(source)
        .filter(|_| !game.is_phased_out(source))
        .map(|object| {
            crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                object, game,
            )
        })
        .or_else(|| source_snapshot.cloned());
    let mut total_damage_dealt = 0u32;
    let mut affected_objects = Vec::new();
    let mut any_replacement_prevented = false;
    let mut lifelink_outcome = None;
    for processed in processed_results {
        programs.extend(processed.programs);
        any_replacement_prevented |= processed.replacement_prevented;
        if let Some(mut payload) = processed.payload_outcome {
            if let Some(batch) = simultaneous_batch {
                for event in &mut payload.events {
                    *event = event.clone().with_simultaneous_batch(batch);
                }
            }
            outcomes.push(payload);
        }
        for assignment in processed.assignments {
            let target_snapshot = match assignment.target {
                DamageTarget::Object(object_id) => game.object(object_id).map(|obj| {
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        obj, game,
                    )
                }),
                DamageTarget::Player(_) => None,
            };
            let excess_damage = match assignment.target {
                DamageTarget::Object(object_id) => {
                    excess_damage_to_object(game, object_id, assignment.amount, keywords)
                }
                DamageTarget::Player(_) => 0,
            };
            let applied = crate::rules::damage::apply_processed_damage_assignment_with_scope(
                game,
                source,
                assignment.target,
                assignment.amount,
                keywords,
                cause.clone(),
                dm,
                replacement_scope,
                source_snapshot,
                provenance,
            )?;
            if dm.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if !applied.applied {
                continue;
            }

            total_damage_dealt = total_damage_dealt.saturating_add(assignment.amount);
            if let DamageTarget::Object(object_id) = assignment.target {
                affected_objects.push(object_id);
            }
            let mut outcome = EffectOutcome::count(i64::from(assignment.amount));
            if excess_damage > 0 {
                outcome = outcome
                    .with_execution_fact(ExecutionFact::ExcessDamageDealt)
                    .with_execution_fact(ExecutionFact::ExcessDamage(excess_damage));
            }
            if assignment.amount > 0 {
                let mut damage_event = DamageEvent::with_cause(
                    source,
                    assignment.target,
                    assignment.amount,
                    source_is_combat,
                    cause.clone(),
                )
                .with_excess_damage(excess_damage);
                if let Some(snapshot) = target_snapshot {
                    damage_event = damage_event.with_target_snapshot(snapshot);
                }
                let mut event = TriggerEvent::new_with_provenance(damage_event, provenance);
                if let Some(batch) = simultaneous_batch {
                    event = event.with_simultaneous_batch(batch);
                }
                if let Some(snapshot) = damage_source_snapshot.as_ref() {
                    event = event.with_source_snapshot(snapshot.clone());
                }
                outcome = outcome.with_event(event);
            }

            if let Some(mut consequence_outcome) = applied.consequence_outcome {
                if let Some(batch) = simultaneous_batch {
                    for event in &mut consequence_outcome.events {
                        *event = event.clone().with_simultaneous_batch(batch);
                    }
                }
                outcome =
                    EffectOutcome::aggregate_replacement_outcomes(outcome, [consequence_outcome]);
            }

            outcomes.push(outcome);
        }
    }

    if keywords.has_lifelink
        && total_damage_dealt > 0
        && let Some(controller) = source_controller
    {
        let mut life_ctx = ExecutionContext::new(source, controller, &mut *dm);
        life_ctx.provenance = provenance;
        life_ctx.cause = cause.clone();
        life_ctx.source_snapshot = source_snapshot.cloned();
        life_ctx.replacement = replacement_scope.clone();
        let gain = crate::effects::life::life_change::execute_life_change(
            game,
            &mut life_ctx,
            crate::events::Event::new_with_provenance(
                LifeGainEvent::new(controller, total_damage_dealt).with_source(source),
                provenance,
            ),
        )?;
        if life_ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        lifelink_outcome = Some(gain);
    }

    let mut outcome = if outcomes.is_empty() && any_replacement_prevented {
        EffectOutcome::prevented()
    } else if outcomes.is_empty() {
        EffectOutcome::count(0)
    } else {
        EffectOutcome::aggregate_summing_counts(outcomes)
    };
    if !affected_objects.is_empty() {
        outcome = outcome.with_affected_objects_from_game(game, affected_objects);
    }
    if let Some(gain) = lifelink_outcome {
        outcome = EffectOutcome::aggregate_replacement_outcomes(outcome, [gain]);
    }
    let mut parent = ExecutionContext::new(
        source,
        source_controller.unwrap_or(game.turn.active_player),
        dm,
    )
    .with_cause(cause)
    .with_provenance(provenance);
    parent.source_snapshot = damage_source_snapshot;
    parent.replacement = replacement_scope.clone();
    finish_damage_replacement_programs(game, &mut parent, outcome, programs)
}

/// Execute captured additions after their owning damage operation. Targets
/// use the matched event and its captured incarnation, never a later ID chase.
pub(crate) fn finish_damage_replacement_programs(
    game: &mut GameState,
    parent: &mut ExecutionContext,
    outcome: EffectOutcome,
    programs: Vec<crate::events::processing::PreparedReplacementProgram>,
) -> Result<EffectOutcome, ExecutionError> {
    crate::effects::replacement::execute_deferred_replacement_programs_with_bindings(
        game,
        parent,
        outcome,
        programs,
        |_, context, _| {
            let damage = crate::events::downcast_event::<DamageEvent>(context.event.inner())
                .ok_or_else(|| {
                    ExecutionError::InternalError("damage addition lost its matched event".into())
                })?;
            let target = match damage.target {
                DamageTarget::Player(player) => ResolvedTarget::Player(player),
                DamageTarget::Object(object) => ResolvedTarget::Object(object),
            };
            let snapshots = damage
                .target_snapshot
                .clone()
                .into_iter()
                .collect::<Vec<_>>();
            Ok(crate::effects::replacement::ReplacementProgramBindings {
                targets: Some(vec![target]),
                object_tags: vec![
                    ("it".into(), snapshots.clone()),
                    ("__it__".into(), snapshots),
                ],
            })
        },
    )
}

/// CR 120.4a / 120.10: excess damage dealt to a permanent. A permanent that
/// is more than one of creature, planeswalker and battle uses the greatest of
/// the per-type excess amounts.
fn excess_damage_to_object(
    game: &GameState,
    target: crate::ids::ObjectId,
    amount: u32,
    keywords: crate::rules::damage::SourceDamageKeywords,
) -> u32 {
    if amount == 0 {
        return 0;
    }
    let Some(object) = game.object(target) else {
        return 0;
    };
    let mut excess: Option<u32> = None;
    if game.current_has_card_type(target, CardType::Creature) {
        let lethal = if keywords.has_deathtouch {
            Some(1)
        } else {
            game.calculated_toughness(target)
                .or_else(|| object.toughness())
                .map(|toughness| (toughness - game.damage_on(target) as i32).max(0) as u32)
        };
        if let Some(lethal) = lethal {
            excess = Some(excess.unwrap_or(0).max(amount.saturating_sub(lethal)));
        }
    }
    if game.current_has_card_type(target, CardType::Planeswalker) {
        let loyalty = object.loyalty().unwrap_or(0);
        excess = Some(excess.unwrap_or(0).max(amount.saturating_sub(loyalty)));
    }
    if game.current_has_card_type(target, CardType::Battle) {
        // CR 310.4c / 120.4a: a battle's defense is the number of defense
        // counters on it, not its printed defense.
        let defense = object
            .counters
            .get(&crate::object::CounterType::Defense)
            .copied()
            .unwrap_or(0);
        excess = Some(excess.unwrap_or(0).max(amount.saturating_sub(defense)));
    }
    excess.unwrap_or(0)
}

/// CR 120.3 / 120.4: damage can be dealt to a creature, planeswalker or
/// battle. Reads the object's current (layered) card types so an animated
/// land or a crewed Vehicle is a legal recipient.
pub(crate) fn object_can_be_dealt_damage(
    game: &GameState,
    object_id: crate::ids::ObjectId,
) -> bool {
    game.object(object_id).is_some()
        && (game.current_has_card_type(object_id, CardType::Creature)
            || game.current_has_card_type(object_id, CardType::Planeswalker)
            || game.current_has_card_type(object_id, CardType::Battle))
}

trait ExcessDamageRedirectExt {
    fn deal_with_excess_redirect(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
        amount: u32,
    ) -> Result<Option<EffectOutcome>, ExecutionError>;
}

impl ExcessDamageRedirectExt for DealDamageEffect {
    /// "Excess damage is dealt to that creature's controller instead": the
    /// creature is dealt only lethal damage and the rest goes to its
    /// controller (CR 120.4a).
    fn deal_with_excess_redirect(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        object_id: crate::ids::ObjectId,
        amount: u32,
    ) -> Result<Option<EffectOutcome>, ExecutionError> {
        let Some(redirect) = &self.excess_to_controller else {
            return Ok(None);
        };
        if let Some(condition) = &redirect.condition
            && !crate::condition_eval::evaluate_condition_resolution(game, condition, ctx)?
        {
            return Ok(None);
        }
        let Some(object) = game.object(object_id) else {
            return Ok(None);
        };
        if !game.current_is_creature(object_id) {
            return Ok(None);
        }
        let controller = game.controller_of(object);
        let keywords = crate::rules::damage::source_damage_keywords(
            game,
            ctx.source,
            ctx.source_snapshot.as_ref(),
        );
        let excess = excess_damage_to_object(game, object_id, amount, keywords).min(amount);
        if excess == 0 {
            return Ok(None);
        }
        // CR 120.4a modifies the one damage event: the lethal part and the
        // redirected excess are dealt simultaneously, so lifelink yields a
        // single life-gain event (CR 702.15e) and watchers see one batch.
        Ok(Some(apply_simultaneous_damage_assignments_opts(
            game,
            ctx.source,
            ctx.source_snapshot.as_ref(),
            vec![
                (DamageTarget::Object(object_id), amount - excess),
                (DamageTarget::Player(controller), excess),
            ],
            self.source_is_combat,
            self.unpreventable,
            ctx.provenance,
            ctx.cause.clone(),
            &ctx.replacement,
            &mut *ctx.decision_maker,
        )?))
    }
}

impl EffectExecutor for DealDamageEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Dealing damage involves no player choices (targets were fixed
        // earlier); defer to commit so the batch lands simultaneously.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_nonnegative_u32(game, &self.amount, ctx)?;

        // Check if this is targeting IteratedPlayer (used in ForEachOpponent)
        // If so, resolve the target from the context's iterated_player
        if let ChooseSpec::Player(PlayerFilter::IteratedPlayer) = &self.target {
            if let Some(player_id) = ctx.iteration.iterated_player {
                return Ok(apply_processed_damage_outcome_opts(
                    game,
                    ctx.source,
                    ctx.source_snapshot.as_ref(),
                    DamageTarget::Player(player_id),
                    amount,
                    self.source_is_combat,
                    self.unpreventable,
                    ctx.provenance,
                    ctx.cause.clone(),
                    &ctx.replacement,
                    &mut *ctx.decision_maker,
                )?);
            }
            return Ok(EffectOutcome::target_invalid());
        }

        if let ChooseSpec::Iterated = &self.target {
            if let Some(object_id) = ctx.iteration.iterated_object {
                if game.object(object_id).is_some() {
                    if !object_can_be_dealt_damage(game, object_id) {
                        return Ok(EffectOutcome::target_invalid());
                    }
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &ctx.replacement,
                        &mut *ctx.decision_maker,
                    )?);
                }
                return Ok(EffectOutcome::target_invalid());
            }
            return Ok(EffectOutcome::target_invalid());
        }

        if let ChooseSpec::AttackedPlayerOrPlaneswalker = &self.target {
            let attacked_target = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| {
                    if let Some(attacked) = event.downcast::<CreatureAttackedEvent>() {
                        return Some(attacked.target);
                    }
                    if let Some(blocked) = event.downcast::<CreatureBecameBlockedEvent>() {
                        return blocked.attack_target;
                    }
                    None
                })
                .or_else(|| ctx.combat.defending_player.map(AttackEventTarget::Player));

            let Some(attacked_target) = attacked_target else {
                return Ok(EffectOutcome::target_invalid());
            };

            match attacked_target {
                AttackEventTarget::Player(player_id) => {
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Player(player_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &ctx.replacement,
                        &mut *ctx.decision_maker,
                    )?);
                }
                AttackEventTarget::Planeswalker(object_id) => {
                    if game.object(object_id).is_none()
                        || !game.current_has_card_type(object_id, CardType::Planeswalker)
                    {
                        return Ok(EffectOutcome::target_invalid());
                    }
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &ctx.replacement,
                        &mut *ctx.decision_maker,
                    )?);
                }
                AttackEventTarget::Battle(object_id) => {
                    if game.object(object_id).is_none()
                        || !game.current_has_card_type(object_id, CardType::Battle)
                    {
                        return Ok(EffectOutcome::target_invalid());
                    }
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &ctx.replacement,
                        &mut *ctx.decision_maker,
                    )?);
                }
                // CR 506.4c: it isn't attacking anything.
                AttackEventTarget::Nothing => return Ok(EffectOutcome::target_invalid()),
            }
        }

        // Handle SourceController - deal damage to the controller of the source (e.g., Ancient Tomb)
        if let ChooseSpec::SourceController = &self.target {
            let controller = ctx.controller;
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(controller),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }

        // "Each player" is one simultaneous damage event per matching player,
        // not a request to resolve the player filter to a single representative.
        // This distinction is observable in shared-life variants (CR 810.9).
        if matches!(self.target.base(), ChooseSpec::EachPlayer(_)) {
            let damage_targets = resolve_players_from_spec(game, &self.target, ctx)?
                .into_iter()
                .map(DamageTarget::Player)
                .collect::<Vec<_>>();
            if damage_targets.is_empty() {
                return Ok(EffectOutcome::count(0));
            }
            return Ok(apply_simultaneous_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                damage_targets,
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }

        // A triggered damage follow-up can refer to the exact participant of
        // the triggering damage event as "that permanent or player". This is
        // a reference, not a new announced target choice, so resolve the
        // correlated event participant before the ordinary ObjectOrPlayer
        // targeting path.
        if let ChooseSpec::ObjectOrPlayer(object_filter, PlayerFilter::DamagedPlayer) =
            self.target.base()
            && matches!(
                object_filter.tagged_constraints.as_slice(),
                [constraint]
                    if constraint.tag.as_str() == "damaged"
                        && constraint.relation
                            == crate::filter::TaggedOpbjectRelation::IsTaggedObject
            )
            && let Some(damage) = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<DamageEvent>())
        {
            let recipient = match damage.target {
                DamageTarget::Object(object_id) if object_can_be_dealt_damage(game, object_id) => {
                    Some(DamageTarget::Object(object_id))
                }
                DamageTarget::Player(player_id)
                    if game
                        .player(player_id)
                        .is_some_and(|player| player.is_in_game()) =>
                {
                    Some(DamageTarget::Player(player_id))
                }
                _ => None,
            };
            let Some(recipient) = recipient else {
                return Ok(EffectOutcome::target_invalid());
            };
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                recipient,
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }

        if matches!(
            self.target.base(),
            ChooseSpec::AnyTarget
                | ChooseSpec::AnyOtherTarget
                | ChooseSpec::ObjectOrPlayer(_, _)
                | ChooseSpec::PlayerOrPlaneswalker(_)
        ) {
            let mut found_assignment = false;
            let mut damage_targets = Vec::new();
            for assignment in &ctx.target_assignments {
                if assignment.spec == self.target || assignment.spec.base() == self.target.base() {
                    found_assignment = true;
                    let Some(assigned_targets) = ctx.targets.get(assignment.range.clone()) else {
                        continue;
                    };
                    for target in assigned_targets {
                        match target {
                            ResolvedTarget::Player(player_id) => {
                                if !game
                                    .player(*player_id)
                                    .is_some_and(|player| player.is_in_game())
                                {
                                    continue;
                                }
                                damage_targets.push(DamageTarget::Player(*player_id));
                            }
                            ResolvedTarget::Object(object_id) => {
                                if !object_can_be_dealt_damage(game, *object_id) {
                                    continue;
                                }
                                damage_targets.push(DamageTarget::Object(*object_id));
                            }
                        }
                    }
                    // This effect owns one announced target requirement. A later
                    // requirement may have the same surface `ChooseSpec`, but it
                    // belongs to a different effect and must not be consumed here.
                    break;
                }
            }
            if found_assignment {
                return if damage_targets.is_empty() {
                    Ok(EffectOutcome::target_invalid())
                } else {
                    Ok(apply_simultaneous_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        damage_targets,
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &ctx.replacement,
                        &mut *ctx.decision_maker,
                    )?)
                };
            }
        }

        let controller_of_tagged = match &self.target {
            ChooseSpec::Player(
                PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
                | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag)),
            ) => Some(tag),
            ChooseSpec::Target(inner) => match inner.as_ref() {
                ChooseSpec::Player(
                    PlayerFilter::ControllerOf(ObjectRef::Tagged(tag))
                    | PlayerFilter::AliasedControllerOf(ObjectRef::Tagged(tag)),
                ) => Some(tag),
                _ => None,
            },
            _ => None,
        };
        if let Some(tag) = controller_of_tagged {
            let controller = ctx
                .get_tagged(tag)
                .map(|snapshot| snapshot.controller)
                .or_else(|| {
                    ctx.triggering_event
                        .as_ref()
                        .and_then(|event| event.snapshot())
                        .map(|snapshot| snapshot.controller)
                });
            let Some(controller) = controller else {
                return Ok(EffectOutcome::target_invalid());
            };
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(controller),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }

        let controller_of_specific = match &self.target {
            ChooseSpec::Player(
                PlayerFilter::ControllerOf(ObjectRef::Specific(id))
                | PlayerFilter::AliasedControllerOf(ObjectRef::Specific(id)),
            ) => Some(*id),
            _ => None,
        };
        if let Some(object_id) = controller_of_specific {
            let controller = game
                .object(object_id)
                .map(|object| game.controller_of(object))
                .or_else(|| {
                    ctx.target_snapshots
                        .get(&object_id)
                        .map(|snapshot| snapshot.controller)
                })
                .or_else(|| {
                    ctx.triggering_event
                        .as_ref()
                        .and_then(|event| event.downcast::<DamageEvent>())
                        .filter(|event| event.source == object_id)
                        .and_then(|event| {
                            event.cause.source_controller.or_else(|| {
                                game.object(object_id)
                                    .map(|object| game.controller_of(object))
                            })
                        })
                });
            let Some(controller) = controller else {
                return Ok(EffectOutcome::target_invalid());
            };
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(controller),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }

        if matches!(
            self.target,
            ChooseSpec::Player(_)
                | ChooseSpec::PlayerOrPlaneswalker(_)
                | ChooseSpec::SourceOwner
                | ChooseSpec::SpecificPlayer(_)
                | ChooseSpec::EachPlayer(_)
        ) && let Ok(player_id) = resolve_player_from_spec(game, &self.target, ctx)
        {
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(player_id),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }

        let resolved_objects = resolve_objects_for_effect(game, ctx, &self.target)
            .map(|object_ids| {
                object_ids
                    .into_iter()
                    .filter(|object_id| {
                        game.object(*object_id).is_some_and(|obj| {
                            obj.zone == crate::zone::Zone::Battlefield
                                && object_can_be_dealt_damage(game, *object_id)
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        // "Deals N damage to each of up to two target creatures": every
        // announced object that is still legal is dealt the damage, as one
        // simultaneous event (CR 601.2c, 608.2b, 120.3).
        let names_several = matches!(
            self.target.base(),
            ChooseSpec::All(_) | ChooseSpec::Tagged(_)
        ) || self.target.count().max.is_none_or(|max| max > 1);
        if names_several && resolved_objects.len() > 1 {
            return Ok(apply_simultaneous_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                resolved_objects
                    .into_iter()
                    .map(DamageTarget::Object)
                    .collect(),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }
        if let Some(object_id) = resolved_objects.first().copied() {
            if let Some(outcome) = self.deal_with_excess_redirect(game, ctx, object_id, amount)? {
                return Ok(outcome);
            }
            return Ok(apply_processed_damage_outcome_opts(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Object(object_id),
                amount,
                self.source_is_combat,
                self.unpreventable,
                ctx.provenance,
                ctx.cause.clone(),
                &ctx.replacement,
                &mut *ctx.decision_maker,
            )?);
        }

        // Otherwise, use pre-resolved targets from ctx.targets
        for target in &ctx.targets {
            // A resolution context can contain several independently
            // announced targets. The fallback must not consume the first one
            // merely because it can receive damage: it still has to satisfy
            // this particular damage effect's target specification.
            if !validate_target(game, target, &self.target, ctx) {
                continue;
            }
            match target {
                ResolvedTarget::Player(player_id) => {
                    return Ok(apply_processed_damage_outcome_opts(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Player(*player_id),
                        amount,
                        self.source_is_combat,
                        self.unpreventable,
                        ctx.provenance,
                        ctx.cause.clone(),
                        &ctx.replacement,
                        &mut *ctx.decision_maker,
                    )?);
                }
                ResolvedTarget::Object(object_id) => {
                    if game.object(*object_id).is_some() {
                        if !object_can_be_dealt_damage(game, *object_id) {
                            continue;
                        }
                        return Ok(apply_processed_damage_outcome_opts(
                            game,
                            ctx.source,
                            ctx.source_snapshot.as_ref(),
                            DamageTarget::Object(*object_id),
                            amount,
                            self.source_is_combat,
                            self.unpreventable,
                            ctx.provenance,
                            ctx.cause.clone(),
                            &ctx.replacement,
                            &mut *ctx.decision_maker,
                        )?);
                    }
                }
            }
        }

        Ok(EffectOutcome::target_invalid())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        // SourceController is deterministic at resolution time (no cast-time selection),
        // but exposing it here keeps downstream wrappers/tests able to inspect
        // what subject this damage effect is bound to.
        if self.target.is_target() || matches!(self.target, ChooseSpec::SourceController) {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target for damage"
    }
}

#[cfg(test)]
mod replacement_scope_tests {
    use super::*;
    use crate::effect::{Effect, OutcomeValue};
    use crate::events::damage::matchers::DamageFromSourceMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    fn source(game: &mut GameState, alice: PlayerId, lifelink: bool) -> crate::ids::ObjectId {
        let mut card = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Damage source")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2));
        if lifelink {
            card = card.with_ability(crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::lifelink(),
            ));
        }
        game.create_object_from_definition(&card.build(), alice, Zone::Battlefield)
    }

    #[test]
    fn single_recipient_damage_retains_only_its_enclosing_action_batch() {
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let first_source = source(&mut game, alice, false);
        let second_source = source(&mut game, alice, false);
        let opened = game.open_simultaneous_action();
        assert!(opened);
        let expected_batch = game.simultaneous_action_batch().unwrap();
        for damage_source in [first_source, second_source] {
            let mut ctx = ExecutionContext::new_default(damage_source, alice);
            let outcome = DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(bob))
                .execute(&mut game, &mut ctx)
                .unwrap();
            let event = outcome
                .events
                .iter()
                .find(|event| event.downcast::<DamageEvent>().is_some())
                .unwrap();
            assert_eq!(event.simultaneous_batch(), Some(expected_batch));
        }
        game.close_simultaneous_action(opened);
        let mut ctx = ExecutionContext::new_default(first_source, alice);
        let outcome = DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let event = outcome
            .events
            .iter()
            .find(|event| event.downcast::<DamageEvent>().is_some())
            .unwrap();
        assert_eq!(
            event.simultaneous_batch(),
            None,
            "a later independent damage instruction must not inherit the old batch"
        );
        assert_eq!(game.player(bob).unwrap().life, 17);
    }

    #[test]
    fn damage_preserves_parent_temporary_effects_and_both_suppression_carriers() {
        for mask in 0..4 {
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
            let source = source(&mut game, alice, false);
            let global = game.effect_store.replacement_effects.add_resolution_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                    ReplacementAction::Modify(EventModification::Multiply(2)),
                ),
            );
            let temporary = ReplacementEffect::with_matcher(
                source,
                alice,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Modify(EventModification::Multiply(3)),
            );
            let key = temporary.application_key();
            let mut dm = crate::decision::SelectFirstDecisionMaker;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ctx.replacement
                .additional_replacement_effects
                .push(temporary);
            if mask & 1 != 0 {
                ctx.replacement
                    .suppressed_replacement_effects
                    .insert(global);
            }
            if mask & 2 != 0 {
                ctx.replacement
                    .suppressed_replacement_effect_keys
                    .insert(key.clone());
            }
            let expected =
                3 * if mask & 1 == 0 { 2 } else { 1 } * if mask & 2 == 0 { 3 } else { 1 };
            let outcome = DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob))
                .execute(&mut game, &mut ctx)
                .unwrap();
            assert_eq!(outcome.value, OutcomeValue::Count(expected));
            assert_eq!(i64::from(game.player(bob).unwrap().life), 20 - expected);
            assert_eq!(ctx.replacement.additional_replacement_effects.len(), 1);
            assert_eq!(
                ctx.replacement
                    .suppressed_replacement_effects
                    .contains(&global),
                mask & 1 != 0
            );
            assert_eq!(
                ctx.replacement
                    .suppressed_replacement_effect_keys
                    .contains(&key),
                mask & 2 != 0
            );
            let mut events = outcome.events;
            events.extend(game.take_pending_trigger_events());
            let damage = events
                .iter()
                .filter_map(|event| event.downcast::<DamageEvent>())
                .collect::<Vec<_>>();
            assert_eq!(damage.len(), 1);
            assert_eq!(damage[0].amount, expected as u32);
            assert_eq!(damage[0].target, DamageTarget::Player(bob));
        }
    }

    #[test]
    fn damage_life_infect_and_wither_consequences_preserve_parent_scope() {
        for kind in 0..3 {
            for suppress in [false, true] {
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
                let source = source(&mut game, alice, false);
                if kind > 0 {
                    game.object_mut(source).unwrap().abilities_mut().push(
                        crate::ability::Ability::static_ability(if kind == 1 {
                            crate::static_abilities::StaticAbility::infect()
                        } else {
                            crate::static_abilities::StaticAbility::wither()
                        }),
                    );
                }
                let victim = game.create_object_from_definition(
                    &crate::cards::CardDefinitionBuilder::new(CardId::new(), "Counter recipient")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(crate::card::PowerToughness::fixed(10, 10))
                        .build(),
                    bob,
                    Zone::Battlefield,
                );
                let replacement = match kind {
                    0 => ReplacementEffect::with_matcher(
                        source,
                        alice,
                        crate::events::WouldLoseLifeMatcher::new(PlayerFilter::Specific(bob)),
                        ReplacementAction::Modify(EventModification::Multiply(2)),
                    ),
                    1 => {
                        crate::static_abilities::StaticAbility::double_player_counters_replacement(
                            PlayerFilter::Specific(bob),
                            Some(crate::CounterType::Poison),
                            "Double poison".into(),
                        )
                        .generate_replacement_effect(source, alice)
                        .unwrap()
                    }
                    _ => crate::static_abilities::StaticAbility::double_counters_replacement(
                        ObjectFilter::specific(victim),
                        Some(crate::CounterType::MinusOneMinusOne),
                        "Double wither".into(),
                    )
                    .generate_replacement_effect(source, alice)
                    .unwrap(),
                };
                let key = replacement.application_key();
                let mut dm = crate::decision::SelectFirstDecisionMaker;
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                ctx.replacement
                    .additional_replacement_effects
                    .push(replacement);
                if suppress {
                    ctx.replacement
                        .suppressed_replacement_effect_keys
                        .insert(key);
                }
                let target = if kind == 2 {
                    ChooseSpec::SpecificObject(victim)
                } else {
                    ChooseSpec::SpecificPlayer(bob)
                };
                let outcome = DealDamageEffect::new(3, target)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(outcome.value, OutcomeValue::Count(3));
                let actual = if suppress { 3 } else { 6 };
                assert_eq!(
                    game.player(bob).unwrap().life,
                    if kind == 0 { 20 - actual } else { 20 }
                );
                assert_eq!(
                    game.player(bob).unwrap().poison_counters,
                    if kind == 1 { actual as u32 } else { 0 }
                );
                assert_eq!(
                    game.counter_count(victim, crate::CounterType::MinusOneMinusOne),
                    if kind == 2 { actual as u32 } else { 0 }
                );
                assert_eq!(game.damage_on(victim), 0);
                let mut events = outcome.events;
                events.extend(game.take_pending_trigger_events());
                assert_eq!(
                    events
                        .iter()
                        .filter_map(|event| event.downcast::<DamageEvent>())
                        .count(),
                    1
                );
                let markers = events
                    .iter()
                    .filter_map(|event| event.downcast::<crate::events::MarkersChangedEvent>())
                    .filter(|event| event.is_added())
                    .collect::<Vec<_>>();
                assert_eq!(markers.len(), usize::from(kind != 0));
                if kind > 0 {
                    assert_eq!(markers[0].amount, actual as u32);
                    assert_eq!(markers[0].source, Some(source));
                    assert_eq!(markers[0].source_controller, Some(alice));
                }
            }
        }
    }

    #[test]
    fn nested_single_and_simultaneous_damage_retain_history_and_lifelink_scope() {
        for simultaneous in [false, true] {
            for departed in [false, true] {
                let alice = PlayerId::from_index(0);
                let bob = PlayerId::from_index(1);
                let carol = PlayerId::from_index(2);
                let mut game =
                    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20);
                let source = source(&mut game, alice, true);
                let target = if simultaneous {
                    ChooseSpec::EachPlayer(PlayerFilter::Opponent)
                } else {
                    ChooseSpec::SpecificPlayer(bob)
                };
                let replacement = game.effect_store.replacement_effects.add_resolution_effect(
                    ReplacementEffect::with_matcher(
                        source,
                        alice,
                        DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                        ReplacementAction::Instead(vec![Effect::deal_damage(2, target)]),
                    ),
                );
                let snapshot =
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        game.object(source).unwrap(),
                        &game,
                    );
                if departed {
                    game.move_object(
                        source,
                        Zone::Exile,
                        crate::events::cause::EventCause::effect(),
                    )
                    .unwrap();
                }
                game.take_pending_trigger_events();
                let mut dm = crate::decision::SelectFirstDecisionMaker;
                let mut ctx = ExecutionContext::new(source, alice, &mut dm);
                ctx.source_snapshot = Some(snapshot);
                ctx.replacement.additional_replacement_effects.push(
                    ReplacementEffect::with_matcher(
                        source,
                        alice,
                        crate::events::WouldGainLifeMatcher::new(PlayerFilter::Specific(alice)),
                        ReplacementAction::Modify(EventModification::Multiply(2)),
                    ),
                );
                let outcome = DealDamageEffect::new(5, ChooseSpec::SpecificPlayer(bob))
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(outcome.count_or_zero(), 0);
                assert_eq!(game.player(bob).unwrap().life, 18);
                assert_eq!(
                    game.player(carol).unwrap().life,
                    if simultaneous { 18 } else { 20 }
                );
                assert_eq!(
                    game.player(alice).unwrap().life,
                    if simultaneous { 28 } else { 24 }
                );
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(replacement)
                        .is_some()
                );
                assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
                assert!(
                    ctx.replacement
                        .suppressed_replacement_effect_keys
                        .is_empty()
                );
                let mut events = outcome.events;
                events.extend(game.take_pending_trigger_events());
                let damage = events
                    .iter()
                    .filter_map(|event| event.downcast::<DamageEvent>())
                    .collect::<Vec<_>>();
                assert_eq!(damage.len(), if simultaneous { 2 } else { 1 });
                assert!(damage.iter().all(|event| event.amount == 2));
                let gains = events
                    .iter()
                    .filter_map(|event| event.downcast::<LifeGainEvent>())
                    .collect::<Vec<_>>();
                assert_eq!(gains.len(), 1);
                assert_eq!(gains[0].amount, if simultaneous { 8 } else { 4 });
                assert_eq!(gains[0].player, alice);
            }
        }
    }
}

#[cfg(test)]
#[path = "deal_damage_split_history_tests.rs"]
mod split_history_tests;

#[cfg(test)]
#[path = "deal_damage_prevention_scope_tests.rs"]
mod prevention_scope_tests;

#[cfg(test)]
#[path = "deal_damage_choice_tests.rs"]
mod choice_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::cause::CauseFilter;
    use crate::events::counters::matchers::WouldPutCountersMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        power: i32,
        toughness: i32,
        controller: PlayerId,
        abilities: Vec<StaticAbility>,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build();
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        for ability in abilities {
            obj.abilities_mut().push(Ability::static_ability(ability));
        }
        game.add_object(obj);
        id
    }

    fn add_doubling_season_like_effect(
        game: &mut GameState,
        controller: PlayerId,
        target: crate::ids::ObjectId,
    ) {
        let source = game.new_object_id();
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                controller,
                WouldPutCountersMatcher::new(
                    ObjectFilter::specific(target),
                    Some(CounterType::MinusOneMinusOne),
                )
                .with_cause_filter(CauseFilter::from_effect()),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );
    }

    fn create_player_aura_attached_to(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        player: PlayerId,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::White]]))
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![crate::types::Subtype::Aura])
            .build();
        let mut obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        obj.attached_to = Some(crate::object::AttachmentTarget::Player(player));
        game.add_object(obj);
        game.player_mut(player)
            .expect("attached player should exist")
            .attachments
            .push(id);
        id
    }

    #[test]
    fn damage_effect_uses_affected_players_tied_replacement_choice() {
        struct ChooseLastReplacement {
            expected_player: PlayerId,
        }

        impl crate::decision::DecisionMaker for ChooseLastReplacement {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.expected_player);
                ctx.options
                    .iter()
                    .rev()
                    .find(|option| option.legal)
                    .map(|option| vec![option.index])
                    .unwrap_or_default()
            }
        }

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let add_source = create_creature(&mut game, "Add Replacement", 1, 1, alice, vec![]);
        let double_source = create_creature(&mut game, "Double Replacement", 1, 1, alice, vec![]);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                add_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Add(1)),
            ),
        );
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                double_source,
                alice,
                crate::events::damage::matchers::DamageToPlayerMatcher::to_any_player(),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );

        let mut dm = ChooseLastReplacement {
            expected_player: bob,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);
        let outcome = DealDamageEffect::new(3, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve through both replacements");

        assert_eq!(outcome.count_or_zero(), 7);
        assert_eq!(game.player(bob).expect("bob should exist").life, 13);
    }

    #[test]
    fn damage_to_object_records_affected_object_memory() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let target = create_creature(&mut game, "Target", 2, 2, bob, vec![]);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let outcome = DealDamageEffect::new(1, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve");

        assert_eq!(outcome.affected_objects(), Some([target].as_slice()));
        let memory = outcome
            .affected_object_memory()
            .expect("damaged object memory should be recorded");
        assert_eq!(memory.len(), 1);
        assert_eq!(memory[0].controller, bob);
        assert_eq!(memory[0].toughness, Some(2));
    }

    #[test]
    fn damage_to_object_records_numeric_excess_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Overkiller", 5, 5, alice, vec![]);
        let target = create_creature(&mut game, "Small Target", 2, 2, bob, vec![]);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let outcome = DealDamageEffect::new(5, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve");

        assert!(
            outcome
                .execution_facts()
                .contains(&ExecutionFact::ExcessDamageDealt)
        );
        assert!(
            outcome
                .execution_facts()
                .contains(&ExecutionFact::ExcessDamage(3))
        );
        let event = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<DamageEvent>())
            .expect("damage outcome should emit its damage event");
        assert_eq!(event.excess_damage, 3);
    }

    #[test]
    fn damage_to_planeswalker_records_damage_beyond_remaining_loyalty_as_excess() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Overkiller", 5, 5, alice, vec![]);
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), "Low Loyalty")
            .card_types(vec![CardType::Planeswalker])
            .loyalty(3)
            .build();
        game.add_object(Object::from_card(id, &card, bob, Zone::Battlefield));
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = DealDamageEffect::new(5, ChooseSpec::SpecificObject(id))
            .execute(&mut game, &mut ctx)
            .expect("planeswalker damage should resolve");
        let event = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<DamageEvent>())
            .expect("damage outcome should emit its damage event");
        assert_eq!(event.excess_damage, 2);
    }

    #[test]
    fn damage_to_tagged_enchanted_player_resolves_from_aura_attachment() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let aura = create_player_aura_attached_to(&mut game, "Curse", alice, bob);
        let mut ctx = ExecutionContext::new_default(aura, alice);

        let outcome = DealDamageEffect::new(
            6,
            ChooseSpec::Player(PlayerFilter::TaggedPlayer(crate::tag::TagKey::from(
                "enchanted",
            ))),
        )
        .execute(&mut game, &mut ctx)
        .expect("damage should resolve");

        assert_eq!(outcome.count_or_zero(), 6);
        assert_eq!(
            game.player(bob).expect("bob should exist").life,
            14,
            "damage should apply to the player enchanted by the Aura source"
        );
    }

    #[test]
    fn object_damage_resolves_matching_target_instead_of_first_raw_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let bounced = create_creature(&mut game, "Bounced", 2, 4, bob, vec![]);
        let damaged = create_creature(&mut game, "Damaged", 2, 2, bob, vec![]);
        game.move_object_by_effect(bounced, Zone::Hand);

        let mut filter = crate::filter::ObjectFilter::default();
        filter.zone = Some(Zone::Battlefield);
        filter.card_types = vec![CardType::Creature, CardType::Planeswalker];

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(bounced),
            ResolvedTarget::Object(damaged),
        ]);

        DealDamageEffect::new(2, ChooseSpec::target(ChooseSpec::Object(filter)))
            .execute(&mut game, &mut ctx)
            .expect("damage should resolve");

        assert_eq!(game.damage_on(damaged), 2);
        assert_eq!(game.damage_on(bounced), 0);
    }

    #[test]
    fn counted_damage_assignment_hits_every_remaining_legal_target_as_one_batch() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(
            &mut game,
            "Lifelink Volley Source",
            2,
            2,
            alice,
            vec![StaticAbility::lifelink()],
        );
        let first = create_creature(&mut game, "First Target", 2, 5, bob, vec![]);
        let illegal = create_creature(&mut game, "Illegal Target", 2, 5, bob, vec![]);
        game.move_object_by_effect(illegal, Zone::Hand);

        let counted_target = ChooseSpec::AnyTarget.with_count(crate::effect::ChoiceCount::up_to(3));
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![
                ResolvedTarget::Object(first),
                ResolvedTarget::Object(illegal),
                ResolvedTarget::Player(bob),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec: counted_target.clone(),
                range: 0..3,
            }]);

        let outcome = DealDamageEffect::new(2, counted_target)
            .execute(&mut game, &mut ctx)
            .expect("every remaining legal target should be damaged");

        assert_eq!(game.damage_on(first), 2);
        assert_eq!(game.damage_on(illegal), 0);
        assert_eq!(game.player(bob).expect("Bob should exist").life, 18);
        assert_eq!(game.player(alice).expect("Alice should exist").life, 24);
        assert_eq!(outcome.count_or_zero(), 4);
        assert_eq!(outcome.affected_objects(), Some([first].as_slice()));
        assert_eq!(
            outcome
                .events
                .iter()
                .filter(|event| event.downcast::<DamageEvent>().is_some())
                .count(),
            2
        );
        let life_gain = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<LifeGainEvent>())
            .expect("lifelink damage should emit a causal life-gain event");
        assert_eq!(life_gain.amount, 4);
        assert_eq!(life_gain.source, Some(source));
    }

    #[test]
    fn noncombat_infect_counter_redirect_instead_and_error_keep_full_outcomes() {
        for action in 0..3 {
            let mut game = setup_game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let source = create_creature(
                &mut game,
                "Infector",
                2,
                2,
                alice,
                vec![StaticAbility::infect()],
            );
            let mut replacement = StaticAbility::double_player_counters_replacement(
                crate::target::PlayerFilter::Specific(bob),
                Some(CounterType::Poison),
                "Counter proposal".into(),
            )
            .generate_replacement_effect(source, alice)
            .unwrap();
            replacement.replacement = if action == 0 {
                ReplacementAction::Redirect {
                    target: crate::replacement::RedirectTarget::ToPlayer(alice),
                    which: crate::replacement::RedirectWhich::First,
                }
            } else {
                let mut payload = vec![crate::effect::Effect::gain_life(4)];
                if action == 2 {
                    payload.push(crate::effect::Effect::lose_life(crate::effect::Value::X));
                }
                ReplacementAction::Instead(payload)
            };
            let one_shot = game
                .effect_store
                .replacement_effects
                .add_one_shot_effect(replacement);
            game.take_pending_trigger_events();
            let mut ctx = ExecutionContext::new_default(source, alice);
            let result = DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(bob))
                .execute(&mut game, &mut ctx);
            if action == 2 {
                assert!(result.is_err());
                assert_eq!(game.player(alice).unwrap().life, 20);
                assert_eq!(game.player(alice).unwrap().poison_counters, 0);
                assert_eq!(game.player(bob).unwrap().poison_counters, 0);
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_some()
                );
            } else {
                let outcome = result.unwrap();
                assert_eq!(
                    outcome.count_or_zero(),
                    2,
                    "counter replacements do not change damage dealt"
                );
                assert_eq!(
                    game.player(alice).unwrap().poison_counters,
                    if action == 0 { 2 } else { 0 }
                );
                assert_eq!(game.player(bob).unwrap().poison_counters, 0);
                assert_eq!(
                    game.player(alice).unwrap().life,
                    if action == 0 { 20 } else { 24 }
                );
                assert_eq!(game.player(bob).unwrap().life, 20);
                assert_eq!(outcome.events.len(), 2);
                let damage = outcome
                    .events
                    .iter()
                    .find_map(|event| event.downcast::<DamageEvent>())
                    .unwrap();
                assert_eq!(damage.target, crate::events::DamageTarget::Player(bob));
                assert_eq!(damage.amount, 2);
                if action == 0 {
                    let marker = outcome
                        .events
                        .iter()
                        .find_map(|event| event.downcast::<crate::events::MarkersChangedEvent>())
                        .unwrap();
                    assert_eq!(
                        marker.location,
                        crate::marker::MarkerLocation::Player(alice)
                    );
                    assert_eq!(marker.amount, 2);
                } else {
                    assert!(
                        outcome.events.iter().any(|event| event
                            .downcast::<crate::events::LifeGainEvent>()
                            .is_some())
                    );
                }
                assert!(
                    game.effect_store
                        .replacement_effects
                        .get_effect(one_shot)
                        .is_none()
                );
            }
            assert!(game.take_pending_trigger_events().is_empty());
        }
    }

    #[test]
    fn noncombat_infect_damage_to_creature_uses_effect_counter_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(
            &mut game,
            "Infector",
            1,
            1,
            alice,
            vec![StaticAbility::infect()],
        );
        let target = create_creature(&mut game, "Target", 2, 2, bob, vec![]);
        add_doubling_season_like_effect(&mut game, bob, target);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        let effect = DealDamageEffect::new(1, ChooseSpec::AnyTarget);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("damage resolves");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), 2);
        assert_eq!(game.damage_on(target), 0);
    }

    #[test]
    fn noncombat_damage_replacement_from_your_source_puts_counters_on_opponent_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let _replacement_source = create_creature(
            &mut game,
            "Soul-Scar Stand-In",
            1,
            2,
            alice,
            vec![StaticAbility::replace_damage_with_counters_instead(
                CounterType::MinusOneMinusOne,
                ObjectFilter::default().controlled_by(PlayerFilter::You),
                ObjectFilter::creature().controlled_by(PlayerFilter::Opponent),
                Some(false),
                "If a source you control would deal noncombat damage to a creature an opponent controls, put that many -1/-1 counters on that creature instead.",
            )],
        );
        let source = create_creature(&mut game, "Pinger", 1, 1, alice, vec![]);
        let target = create_creature(&mut game, "Target", 2, 2, bob, vec![]);
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        DealDamageEffect::new(2, ChooseSpec::AnyTarget)
            .execute(&mut game, &mut ctx)
            .expect("damage replacement should resolve");

        assert_eq!(game.counter_count(target, CounterType::MinusOneMinusOne), 2);
        assert_eq!(game.damage_on(target), 0);
    }

    #[test]
    fn damage_event_history_uses_source_lki_after_source_leaves() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature(&mut game, "Departed Pinger", 1, 1, alice, vec![]);
        let source_snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(source).expect("source should exist"),
            &game,
        );
        game.remove_object(source);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Player(bob)])
            .with_source_snapshot(source_snapshot);

        let effect = DealDamageEffect::new(1, ChooseSpec::AnyTarget);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("departed source should still deal damage from LKI");

        assert_eq!(outcome.events.len(), 2);
        game.record_turn_history_event(&outcome.events[0]);

        assert_eq!(
            game.turn_store
                .turn_history
                .total_creature_damage_to_player(bob),
            1,
            "damage history should treat the departed source as the creature it last was"
        );
    }

    #[test]
    fn damage_life_loss_redirect_preserves_recipient_and_notification() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Damage source", 3, 3, alice, vec![]);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldLoseLifeMatcher::new(PlayerFilter::Specific(bob)),
                ReplacementAction::Redirect {
                    target: crate::replacement::RedirectTarget::ToPlayer(alice),
                    which: crate::replacement::RedirectWhich::First,
                },
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().life, 17);
        assert_eq!(outcome.count_or_zero(), 3, "damage is still dealt to Bob");
        let loss = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<LifeLossEvent>())
            .unwrap();
        assert_eq!(loss.player, alice);
        assert_eq!(loss.amount, 3);
        assert!(loss.from_damage);
        let damage = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<DamageEvent>())
            .unwrap();
        assert_eq!(damage.target, DamageTarget::Player(bob));
    }

    #[test]
    fn damage_life_loss_instead_executes_payload_without_original_loss() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Damage source", 3, 3, alice, vec![]);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldLoseLifeMatcher::new(PlayerFilter::Specific(bob)),
                ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(4)]),
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.player(alice).unwrap().life, 24);
        assert_eq!(outcome.count_or_zero(), 3);
        assert!(
            !outcome
                .events
                .iter()
                .any(|event| event.downcast::<LifeLossEvent>().is_some())
        );
        let gain = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<LifeGainEvent>())
            .unwrap();
        assert_eq!(gain.player, alice);
        assert_eq!(gain.amount, 4);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.downcast::<DamageEvent>().is_some())
        );
    }

    #[test]
    fn damage_life_loss_payload_error_restores_prior_payload_and_one_shot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Damage source", 3, 3, alice, vec![]);
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldLoseLifeMatcher::new(PlayerFilter::Specific(bob)),
                ReplacementAction::Instead(vec![
                    crate::effect::Effect::gain_life(4),
                    crate::effect::Effect::lose_life(crate::effect::Value::X),
                ]),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result =
            DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob)).execute(&mut game, &mut ctx);
        assert!(result.is_err());
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(replacement)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn damage_instead_payload_error_restores_partial_payload_and_one_shot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Damage source", 3, 3, alice, vec![]);
        let replacement = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageFromSourceMatcher::new(
                    ObjectFilter::specific(source),
                ),
                ReplacementAction::Instead(vec![
                    crate::effect::Effect::gain_life(4),
                    crate::effect::Effect::lose_life(crate::effect::Value::X),
                ]),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        assert!(
            DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob))
                .execute(&mut game, &mut ctx)
                .is_err()
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(replacement)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn damage_prevention_follow_up_error_restores_payload_and_shield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Damage source", 3, 3, alice, vec![]);
        game.effect_store.prevention_effects.add_shield(
            crate::prevention::PreventionShield::prevent_next_n(
                source,
                alice,
                crate::prevention::PreventionTarget::Player(bob),
                2,
            )
            .with_follow_up_effects(vec![
                crate::effect::Effect::gain_life(4),
                crate::effect::Effect::lose_life(crate::effect::Value::X),
            ]),
        );
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        assert!(
            DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob))
                .execute(&mut game, &mut ctx)
                .is_err()
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
        assert_eq!(
            game.effect_store.prevention_effects.shields()[0].amount_remaining,
            Some(2)
        );
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn damage_instead_payload_retains_reported_events_without_original_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Damage source", 3, 3, alice, vec![]);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::damage::matchers::DamageFromSourceMatcher::new(
                    ObjectFilter::specific(source),
                ),
                ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(4)]),
            ),
        );
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 24);
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(outcome.count_or_zero(), 0);
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        assert_eq!(outcome.events.len(), 1);
        let gain = outcome.events[0].downcast::<LifeGainEvent>().unwrap();
        assert_eq!(gain.player, alice);
        assert_eq!(gain.amount, 4);
        assert!(
            game.take_pending_trigger_events().is_empty(),
            "reported payload must not also be queued"
        );
    }

    #[test]
    fn lifelink_replacement_redirects_the_committed_gain_and_notification() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(
            &mut game,
            "Lifelink source",
            2,
            2,
            alice,
            vec![StaticAbility::lifelink()],
        );
        let target = create_creature(&mut game, "Target", 2, 5, bob, vec![]);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldGainLifeMatcher::you(),
                ReplacementAction::Redirect {
                    target: crate::replacement::RedirectTarget::ToPlayer(bob),
                    which: crate::replacement::RedirectWhich::First,
                },
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = DealDamageEffect::new(2, ChooseSpec::SpecificObject(target))
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, 22);
        assert_eq!(game.damage_on(target), 2);
        assert_eq!(outcome.count_or_zero(), 2);
        let gain = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<LifeGainEvent>())
            .unwrap();
        assert_eq!(gain.player, bob);
        assert_eq!(gain.source, Some(source));
    }

    #[test]
    fn lifelink_payload_error_rolls_back_damage_and_life() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(
            &mut game,
            "Lifelink source",
            2,
            2,
            alice,
            vec![StaticAbility::lifelink()],
        );
        let target = create_creature(&mut game, "Target", 2, 5, bob, vec![]);
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                crate::events::WouldGainLifeMatcher::you(),
                ReplacementAction::Instead(vec![
                    crate::effect::Effect::lose_life(1),
                    crate::effect::Effect::lose_life(crate::effect::Value::X),
                ]),
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        assert!(
            DealDamageEffect::new(2, ChooseSpec::SpecificObject(target))
                .execute(&mut game, &mut ctx)
                .is_err()
        );
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.damage_on(target), 0);
    }
}

#[cfg(test)]
mod replacement_damage_addition_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::ObjectFilter;
    struct Answers {
        victims: Vec<ObjectId>,
        pause: bool,
        pending: bool,
        calls: usize,
        binding: bool,
    }
    impl DecisionMaker for Answers {
        fn decide_boolean(
            &mut self,
            game: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.calls += 1;
            for id in &self.victims {
                assert_eq!(
                    game.damage_on(*id),
                    3,
                    "all original damage precedes additions"
                );
            }
            if self.binding {
                assert_eq!(
                    game.counter_count(self.victims[0], CounterType::PlusOnePlusOne),
                    1
                );
            }
            self.pending = self.pause;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn creature(game: &mut GameState, owner: PlayerId) -> ObjectId {
        game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Damage owner fixture")
                .card_types(vec![CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(3, 12))
                .build(),
            owner,
            crate::zone::Zone::Battlefield,
        )
    }
    fn check(multiple: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = creature(&mut game, alice);
        let replacement_source = creature(&mut game, bob);
        let sentinel = creature(&mut game, alice);
        let first = creature(&mut game, bob);
        let mut victims = vec![first];
        if multiple {
            victims.push(creature(&mut game, bob));
        }
        let actions = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![
                Effect::new(crate::effects::PutCountersEffect::new(
                    CounterType::PlusOnePlusOne,
                    1,
                    ChooseSpec::tagged("it"),
                )),
                Effect::may(vec![Effect::gain_life(0)]),
            ],
            _ => vec![
                Effect::gain_life(3),
                Effect::may(vec![Effect::gain_life(4)]),
            ],
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                replacement_source,
                bob,
                crate::events::damage::matchers::DamageToObjectMatcher::new(
                    ObjectFilter::specific(first),
                ),
                ReplacementAction::Additionally(actions),
            ),
        );
        let snapshots = victims
            .iter()
            .map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game))
            .collect::<Vec<_>>();
        let sentinel_snapshot = ObjectSnapshot::from_object(game.object(sentinel).unwrap(), &game);
        game.take_pending_trigger_events();
        let ids = game.next_object_id_counter();
        let objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers {
            victims: victims.clone(),
            pause: mode == 2,
            pending: false,
            calls: 0,
            binding: mode == 3,
        };
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(sentinel)]);
        ctx.set_tagged_objects("it", vec![sentinel_snapshot.clone()]);
        ctx.set_tagged_objects("victims", snapshots.clone());
        let effect = DealDamageEffect::new(
            3,
            if multiple {
                ChooseSpec::tagged("victims")
            } else {
                ChooseSpec::SpecificObject(first)
            },
        );
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 {
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
        } else if mode == 2 {
            assert!(ctx.decision_maker.awaiting_choice());
            assert!(result.unwrap().events.is_empty());
        } else {
            let outcome = result.unwrap();
            assert_eq!(outcome.count_or_zero(), 3 * victims.len() as i64);
            for id in &victims {
                assert_eq!(game.damage_on(*id), 3);
            }
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(
                game.player(bob).unwrap().life,
                if mode == 3 { 20 } else { 27 }
            );
            assert_eq!(
                outcome
                    .events
                    .iter()
                    .filter(|event| event.downcast::<DamageEvent>().is_some())
                    .count(),
                victims.len()
            );
            if mode == 3 {
                assert_eq!(game.counter_count(first, CounterType::PlusOnePlusOne), 1);
            } else {
                assert_eq!(
                    outcome
                        .events
                        .iter()
                        .filter_map(|event| event.downcast::<LifeGainEvent>())
                        .map(|event| (event.player, event.amount))
                        .collect::<Vec<_>>(),
                    vec![(bob, 3), (bob, 4)]
                );
            }
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
        assert_eq!(ctx.source, source);
        assert_eq!(ctx.controller, alice);
        assert_eq!(ctx.targets, vec![ResolvedTarget::Object(sentinel)]);
        assert_eq!(
            ctx.get_tagged_all("it").unwrap()[0].object_id,
            sentinel_snapshot.object_id
        );
        assert_eq!(game.counter_count(sentinel, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            for id in &victims {
                assert_eq!(game.damage_on(*id), 0);
                assert_eq!(game.counter_count(*id, CounterType::PlusOnePlusOne), 0);
            }
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.next_object_id_counter(), ids);
            assert_eq!(game.objects_in_deterministic_order().len(), objects);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode == 0 || mode == 3 {
            assert_eq!(dm.calls, 1);
        }
        if mode == 2 {
            assert_eq!(dm.calls, 1);
            dm.pause = false;
            dm.pending = false;
            let mut ctx = ExecutionContext::new(source, alice, &mut dm);
            ctx.set_tagged_objects("victims", snapshots);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(outcome.count_or_zero(), 3 * victims.len() as i64);
            for id in &victims {
                assert_eq!(game.damage_on(*id), 3);
            }
            assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(!ctx.decision_maker.awaiting_choice());
            drop(ctx);
            assert_eq!(dm.calls, 2);
        }
    }
    #[test]
    fn single_additions_follow_damage() {
        check(false, 0);
    }
    #[test]
    fn single_error_restores_damage() {
        check(false, 1);
    }
    #[test]
    fn single_pending_replays_once() {
        check(false, 2);
    }
    #[test]
    fn single_addition_binds_matched_target() {
        check(false, 3);
    }
    #[test]
    fn batch_additions_follow_all_damage() {
        check(true, 0);
    }
    #[test]
    fn batch_error_restores_all_damage() {
        check(true, 1);
    }
    #[test]
    fn batch_pending_replays_once() {
        check(true, 2);
    }
    #[test]
    fn batch_addition_binds_matched_target() {
        check(true, 3);
    }
}

#[cfg(test)]
mod damage_source_current_information_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::color::ColorSet;
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;

    fn check(to_player: bool, departed: bool, current_red: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Damage source information fixture")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let sponsor = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).unwrap(),
            &game,
        );
        assert_eq!(snapshot.colors, ColorSet::RED);
        if !current_red {
            game.object_mut(source).unwrap().color_override = Some(ColorSet::BLUE);
        }
        if departed {
            game.move_object(
                source,
                Zone::Graveyard,
                crate::events::cause::EventCause::effect(),
            )
            .unwrap();
        }
        let action = ReplacementAction::Modify(EventModification::Add(1));
        let red = ObjectFilter::creature().with_colors(ColorSet::RED);
        let effect = if to_player {
            ReplacementEffect::with_matcher(
                sponsor,
                alice,
                crate::events::damage::matchers::DamageFromSourceToPlayerMatcher::new(
                    red,
                    PlayerFilter::Specific(bob),
                ),
                action,
            )
        } else {
            ReplacementEffect::with_matcher(
                sponsor,
                alice,
                crate::events::damage::matchers::DamageFromSourceMatcher::new(red),
                action,
            )
        };
        let shield = game
            .effect_store
            .replacement_effects
            .add_one_shot_effect(effect);
        game.take_pending_trigger_events();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.source_snapshot = Some(snapshot);
        let out = DealDamageEffect::new(3, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ctx)
            .unwrap();
        let expected = if departed || current_red { 4 } else { 3 };
        assert_eq!(
            game.player(bob).unwrap().life,
            20 - expected,
            "present source uses current characteristics rather than a stale stack snapshot"
        );
        assert_eq!(out.count_or_zero(), i64::from(expected));
        assert_eq!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none(),
            expected == 4
        );
        let damage = out
            .events
            .iter()
            .filter_map(|e| e.downcast::<DamageEvent>())
            .collect::<Vec<_>>();
        assert_eq!(damage.len(), 1);
        assert_eq!(damage[0].amount, expected as u32);
    }
    #[test]
    fn general_source_filter_uses_current_information() {
        check(false, false, false);
    }
    #[test]
    fn player_source_filter_uses_current_information() {
        check(true, false, false);
    }
    #[test]
    fn current_red_source_still_matches() {
        check(false, false, true);
        check(true, false, true);
    }
    #[test]
    fn departed_red_source_still_matches_snapshot() {
        check(false, true, true);
        check(true, true, true);
    }
}

#[cfg(test)]
mod damage_source_phase_publication_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::color::ColorSet;
    use crate::ids::{CardId, PlayerId};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;
    fn check(state: u8, simultaneous: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Phased damage source publication fixture")
            .card_types(vec![CardType::Creature])
            .color_indicator(ColorSet::RED)
            .power_toughness(PowerToughness::fixed(2, 4))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        game.object_mut(source).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::set_colors(
                    ObjectFilter::source(),
                    ColorSet::BLUE,
                ),
            ),
        );
        game.object_mut(source).unwrap().abilities_mut().push(
            crate::ability::Ability::static_ability(
                crate::static_abilities::StaticAbility::lifelink(),
            ),
        );
        game.refresh_continuous_state().unwrap();
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).unwrap(),
            &game,
        );
        assert_eq!(
            snapshot.colors,
            ColorSet::BLUE,
            "last present source had its continuous color"
        );
        if state == 1 {
            game.phase_out(source);
            assert!(game.is_phased_out(source));
        }
        if state == 2 {
            game.move_object(
                source,
                Zone::Exile,
                crate::events::cause::EventCause::effect(),
            )
            .unwrap();
        }
        game.refresh_continuous_state().unwrap();
        game.take_pending_trigger_events();
        let mut dm = crate::decision::SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.source_snapshot = Some(snapshot);
        let target = if simultaneous {
            ChooseSpec::EachPlayer(PlayerFilter::Opponent)
        } else {
            ChooseSpec::SpecificPlayer(bob)
        };
        let out = DealDamageEffect::new(3, target)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(bob).unwrap().life, 17);
        assert_eq!(
            game.player(alice).unwrap().life,
            23,
            "damage consequences use the last present source's lifelink"
        );
        let damage = out
            .events
            .iter()
            .filter(|event| event.downcast::<DamageEvent>().is_some())
            .collect::<Vec<_>>();
        assert_eq!(damage.len(), 1);
        assert_eq!(
            damage[0].source_snapshot().unwrap().colors,
            ColorSet::BLUE,
            "damage observations must retain the same authoritative source characteristics as its consequences"
        );
        assert_eq!(damage[0].source_snapshot().unwrap().controller, alice);
        assert_eq!(out.count_or_zero(), 3);
    }
    #[test]
    fn phased_single_damage_retains_source_snapshot() {
        check(1, false);
    }
    #[test]
    fn phased_simultaneous_damage_retains_source_snapshot() {
        check(1, true);
    }
    #[test]
    fn present_source_publication_control() {
        check(0, false);
        check(0, true);
    }
    #[test]
    fn departed_source_publication_control() {
        check(2, false);
        check(2, true);
    }
}

#[cfg(test)]
mod wide_replaced_damage_receipt_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{Effect, EffectId, Value};
    use crate::effects::{PutCountersEffect, execute_effect};
    use crate::events::damage::matchers::DamageFromSourceMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;
    fn check(simultaneous: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Damage source")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let victims = (0..if simultaneous { 2 } else { 1 })
            .map(|_| {
                game.create_object_from_card(
                    &CardBuilder::new(CardId::new(), "Damage recipient")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(PowerToughness::fixed(1, 1))
                        .build(),
                    bob,
                    Zone::Battlefield,
                )
            })
            .collect::<Vec<_>>();
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let target = if simultaneous {
            ChooseSpec::All(ObjectFilter::creature())
        } else {
            ChooseSpec::SpecificObject(victims[0])
        };
        let outcome = execute_effect(
            &mut game,
            &Effect::with_id(19, Effect::new(DealDamageEffect::new(i32::MAX, target))),
            &mut ctx,
        )
        .unwrap();
        let expected = u32::MAX - 1;
        for victim in &victims {
            assert_eq!(
                game.damage_on(*victim),
                expected,
                "commit preserves full resolved unsigned amount"
            );
        }
        let damage = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<DamageEvent>())
            .collect::<Vec<_>>();
        assert_eq!(
            damage.len(),
            victims.len(),
            "one actual damage observation per recipient"
        );
        for event in damage {
            assert_eq!(
                event.amount, expected,
                "published damage agrees with committed damage"
            );
        }
        let total = i64::from(expected) * i64::try_from(victims.len()).unwrap();
        assert_eq!(
            outcome.as_count(),
            Some(total),
            "receipt preserves every resolved amount and their sum"
        );
        let amount = if simultaneous {
            Value::HalfRoundedDown(Box::new(Value::EffectValue(EffectId(19))))
        } else {
            Value::EffectValue(EffectId(19))
        };
        let following = execute_effect(
            &mut game,
            &Effect::new(PutCountersEffect::new(
                CounterType::Charge,
                amount,
                ChooseSpec::SpecificObject(source),
            )),
            &mut ctx,
        )
        .unwrap();
        assert_eq!(
            game.counter_count(source, CounterType::Charge),
            expected,
            "following arithmetic reads the complete damage receipt"
        );
        assert_eq!(following.as_count(), Some(i64::from(expected)));
    }
    #[test]
    fn single_replaced_damage_preserves_unsigned_receipt_and_following_effect() {
        check(false);
    }
    #[test]
    fn simultaneous_replaced_damage_preserves_wide_sum_and_following_effect() {
        check(true);
    }
}

#[cfg(test)]
mod wide_damage_receipt_consumer_tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::{Effect, EffectId, Value};
    use crate::effects::execute_effect;
    use crate::events::damage::matchers::DamageFromSourceMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::zone::Zone;
    fn check_following(simultaneous: bool, out_of_range: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Damage source")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let victims = (0..if simultaneous { 2 } else { 1 })
            .map(|_| {
                game.create_object_from_card(
                    &CardBuilder::new(CardId::new(), "Damage recipient")
                        .card_types(vec![CardType::Creature])
                        .power_toughness(PowerToughness::fixed(1, 1))
                        .build(),
                    bob,
                    Zone::Battlefield,
                )
            })
            .collect::<Vec<_>>();
        let replacement = game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let target = if simultaneous {
            ChooseSpec::All(ObjectFilter::creature())
        } else {
            ChooseSpec::SpecificObject(victims[0])
        };
        let outcome = execute_effect(
            &mut game,
            &Effect::with_id(19, Effect::new(DealDamageEffect::new(i32::MAX, target))),
            &mut ctx,
        )
        .unwrap();
        let expected = u32::MAX - 1;
        for victim in &victims {
            assert_eq!(
                game.damage_on(*victim),
                expected,
                "commit preserves full resolved unsigned amount"
            );
        }
        let damage = outcome
            .events
            .iter()
            .filter_map(|event| event.downcast::<DamageEvent>())
            .collect::<Vec<_>>();
        assert_eq!(
            damage.len(),
            victims.len(),
            "one actual damage observation per recipient"
        );
        for event in damage {
            assert_eq!(
                event.amount, expected,
                "published damage agrees with committed damage"
            );
        }
        let total = i64::from(expected) * i64::try_from(victims.len()).unwrap();
        assert_eq!(
            outcome.as_count(),
            Some(total),
            "receipt preserves every resolved amount and their sum"
        );
        let next = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Following damage recipient")
                .card_types(vec![CardType::Creature])
                .power_toughness(PowerToughness::fixed(1, 1))
                .build(),
            bob,
            Zone::Battlefield,
        );
        ctx.replacement
            .suppressed_replacement_effects
            .insert(replacement);
        let amount = if simultaneous && !out_of_range {
            Value::HalfRoundedDown(Box::new(Value::EffectValue(EffectId(19))))
        } else {
            Value::EffectValue(EffectId(19))
        };
        game.take_pending_trigger_events();
        let following = execute_effect(
            &mut game,
            &Effect::new(DealDamageEffect::new(
                amount,
                ChooseSpec::SpecificObject(next),
            )),
            &mut ctx,
        );
        if out_of_range {
            assert!(
                matches!(following, Err(ExecutionError::UnresolvableValue(_))),
                "a quantity beyond one damage event's unsigned range must reject explicitly"
            );
            assert_eq!(
                game.damage_on(next),
                0,
                "range rejection occurs before mutation"
            );
            assert!(
                game.take_pending_trigger_events().is_empty(),
                "rejected damage publishes no observation"
            );
        } else {
            let following =
                following.expect("representable unsigned following damage must resolve");
            assert_eq!(
                game.damage_on(next),
                expected,
                "damage consumer reads the complete unsigned receipt"
            );
            assert_eq!(following.as_count(), Some(i64::from(expected)));
            let observations = following
                .events
                .iter()
                .filter_map(|event| event.downcast::<DamageEvent>())
                .collect::<Vec<_>>();
            assert_eq!(observations.len(), 1);
            assert_eq!(observations[0].amount, expected);
        }
    }
    #[test]
    fn single_receipt_drives_full_unsigned_following_damage() {
        check_following(false, false);
    }
    #[test]
    fn simultaneous_receipt_arithmetic_drives_unsigned_following_damage() {
        check_following(true, false);
    }
    #[test]
    fn out_of_range_following_damage_rejects_before_mutation() {
        check_following(true, true);
    }
}

pub use ironsmith_core::DealDamageToRecipientsEffect;

impl EffectExecutor for DealDamageToRecipientsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // The amount and every recipient belong to one pre-damage world.
        // In particular, lifelink or a prevention follow-up on one recipient
        // must not change the amount or membership of a later recipient.
        let amount = crate::effects::helpers::resolve_nonnegative_u32(game, &self.amount, ctx)?;
        let mut targets = Vec::new();
        for spec in &self.recipients {
            let proposed = match spec.base() {
                ChooseSpec::Player(_)
                | ChooseSpec::EachPlayer(_)
                | ChooseSpec::SpecificPlayer(_)
                | ChooseSpec::SourceOwner
                | ChooseSpec::SourceController => resolve_players_from_spec(game, spec, ctx)?
                    .into_iter()
                    .map(DamageTarget::Player)
                    .collect::<Vec<_>>(),
                ChooseSpec::All(_)
                | ChooseSpec::Object(_)
                | ChooseSpec::Tagged(_)
                | ChooseSpec::SpecificObject(_)
                | ChooseSpec::Source
                | ChooseSpec::Iterated => {
                    let objects =
                        match crate::effects::helpers::resolve_objects_from_spec(game, spec, ctx) {
                            Ok(objects) => objects,
                            Err(ExecutionError::InvalidTarget) => Vec::new(),
                            Err(error) => return Err(error),
                        };
                    objects
                        .into_iter()
                        .filter(|id| {
                            game.object(*id).is_some_and(|object| {
                                object.zone == crate::zone::Zone::Battlefield
                                    && object_can_be_dealt_damage(game, *id)
                            })
                        })
                        .map(DamageTarget::Object)
                        .collect::<Vec<_>>()
                }
                _ => {
                    return Err(ExecutionError::UnresolvableValue(
                        "damage recipient set requires resolved references or groups".into(),
                    ));
                }
            };
            for target in proposed {
                if !targets.contains(&target) {
                    targets.push(target);
                }
            }
        }
        if targets.is_empty() {
            return Ok(EffectOutcome::count(0));
        }
        apply_simultaneous_damage_outcome_opts(
            game,
            ctx.source,
            ctx.source_snapshot.as_ref(),
            targets,
            amount,
            false,
            false,
            ctx.provenance,
            ctx.cause.clone(),
            &ctx.replacement,
            &mut *ctx.decision_maker,
        )
    }
}

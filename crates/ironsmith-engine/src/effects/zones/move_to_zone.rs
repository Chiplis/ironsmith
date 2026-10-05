//! Move to zone effect implementation.

use crate::combat_state::AttackTarget;
use crate::decisions::context::{OrderContext, SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_player_filter, resolve_tagged_object_id,
};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{
    EventOutcome, PreparedEventOutcome, PreparedZoneProposal, ReplacementEventContext,
    commit_prepared_zone_change, prepare_zone_change_proposal_scoped,
};
use crate::filter::FilterContext;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::PlayerId;
use crate::snapshot::ObjectSnapshot;
use crate::tag::SOURCE_EXILED_TAG;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::CardType;
use crate::zone::Zone;

use super::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, maybe_prompt_for_split_result_order,
    move_to_battlefield_batch_with_options_and_zone_proposals, resolve_battlefield_entry_counters,
    take_recorded_zone_change,
};
pub use ironsmith_core::BattlefieldController;
pub type LibraryPlacementOrder = ironsmith_core::LibraryPlacementOrder;
pub type MoveToZoneAttackTargetMode = ironsmith_core::MoveToZoneAttackTargetMode;
pub type MoveToZoneEffect = ironsmith_core::MoveToZoneEffect;

fn normalize_order_response(
    response: Vec<crate::ids::ObjectId>,
    original: &[crate::ids::ObjectId],
) -> Vec<crate::ids::ObjectId> {
    let mut remaining = original.to_vec();
    let mut ordered = Vec::with_capacity(original.len());
    for object_id in response {
        if let Some(position) = remaining
            .iter()
            .position(|candidate| *candidate == object_id)
        {
            ordered.push(remaining.remove(position));
        }
    }
    ordered.extend(remaining);
    ordered
}

fn order_library_move_objects(
    game: &GameState,
    ctx: &mut ExecutionContext,
    object_ids: Vec<crate::ids::ObjectId>,
    order: &LibraryPlacementOrder,
    to_top: bool,
) -> Result<Vec<crate::ids::ObjectId>, ExecutionError> {
    if object_ids.len() <= 1 {
        return Ok(object_ids);
    }

    match order {
        LibraryPlacementOrder::Random => {
            let mut ordered = object_ids;
            game.shuffle_slice(&mut ordered);
            Ok(ordered)
        }
        LibraryPlacementOrder::ChosenBy(player) => {
            let chooser =
                crate::effects::helpers::resolve_player_filter_as_chooser(game, player, ctx)?;
            let position = if to_top { "top" } else { "bottom" };
            let edge = if to_top { "topmost" } else { "bottom-most" };
            let items = object_ids
                .iter()
                .map(|object_id| {
                    let name = game
                        .object(*object_id)
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());
                    (*object_id, name)
                })
                .collect();
            let order_ctx = OrderContext::new(
                chooser,
                Some(ctx.source),
                format!(
                    "Order the selected cards for the {position} of the library. The first option becomes the {edge} card."
                ),
                items,
            );
            Ok(normalize_order_response(
                ctx.decision_maker.decide_order(game, &order_ctx),
                &object_ids,
            ))
        }
    }
}

/// Rebuild each affected library after all zone-change replacements resolve.
/// `placed_ids` is in player-facing order: topmost first for top placement,
/// bottom-most first for bottom placement.
fn apply_library_placement_order(
    game: &mut GameState,
    placed_ids: &[crate::ids::ObjectId],
    to_top: bool,
) {
    let mut by_owner: Vec<(PlayerId, Vec<crate::ids::ObjectId>)> = Vec::new();
    for &object_id in placed_ids {
        let Some(object) = game.object(object_id) else {
            continue;
        };
        if object.zone != Zone::Library {
            continue;
        }
        let owner = object.owner;
        if let Some((_, ids)) = by_owner
            .iter_mut()
            .find(|(candidate, _)| *candidate == owner)
        {
            if !ids.contains(&object_id) {
                ids.push(object_id);
            }
        } else {
            by_owner.push((owner, vec![object_id]));
        }
    }

    for (owner, ordered_ids) in by_owner {
        let Some(current) = game.player(owner).map(|player| player.library.clone()) else {
            continue;
        };
        let mut unaffected = current
            .into_iter()
            .filter(|object_id| !ordered_ids.contains(object_id))
            .collect::<Vec<_>>();
        let final_order = if to_top {
            unaffected.extend(ordered_ids.iter().rev().copied());
            unaffected
        } else {
            let mut bottom_first = ordered_ids;
            bottom_first.extend(unaffected);
            bottom_first
        };
        game.set_player_library_order_with_audit(
            owner,
            final_order,
            "ordered multi-card library placement",
        );
    }
}

fn attack_targets_for_player(game: &GameState, player_id: PlayerId) -> Vec<AttackTarget> {
    let mut targets = Vec::new();
    if game
        .player(player_id)
        .is_some_and(|player| player.is_in_game())
    {
        targets.push(AttackTarget::Player(player_id));
    }

    for &object_id in &game.battlefield {
        if let Some(object) = game.object(object_id) {
            if game.controller_of(object) == player_id
                && game.current_has_card_type(object_id, CardType::Planeswalker)
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            } else if game.current_has_card_type(object_id, CardType::Battle)
                && game.battle_protector(object_id) == Some(player_id)
            {
                targets.push(AttackTarget::Battle(object_id));
            }
        }
    }

    targets
}

fn choose_attack_target_for_player(
    game: &GameState,
    ctx: &mut ExecutionContext,
    player_id: PlayerId,
    targets: &[AttackTarget],
) -> Option<AttackTarget> {
    if targets.len() == 1 {
        return Some(targets[0].clone());
    }

    let player_name = game
        .player(player_id)
        .map(|player| player.name.to_string())
        .unwrap_or_else(|| "that player".to_string());
    let options: Vec<SelectableOption> = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            let description = match target {
                AttackTarget::Player(_) => format!("Attack {player_name}"),
                AttackTarget::Planeswalker(planeswalker_id) => {
                    let walker_name = game
                        .object(*planeswalker_id)
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "a planeswalker".to_string());
                    format!("Attack {walker_name} controlled by {player_name}")
                }
                AttackTarget::Battle(battle_id) => {
                    let battle_name = game
                        .object(*battle_id)
                        .map(|object| object.name.to_string())
                        .unwrap_or_else(|| "a battle".to_string());
                    format!("Attack {battle_name} protected by {player_name}")
                }
                AttackTarget::Nothing { .. } => "Attack nothing".to_string(),
            };
            SelectableOption::new(index, description)
        })
        .collect();
    let choice_ctx = SelectOptionsContext::new(
        ctx.controller,
        Some(ctx.source),
        format!("Choose attack target for creature entering attacking {player_name}"),
        options,
        1,
        1,
    );
    let chosen = ctx.decision_maker.decide_options(game, &choice_ctx);
    if ctx.decision_maker.awaiting_choice() {
        return None;
    }
    chosen
        .first()
        .copied()
        .filter(|selected| *selected < targets.len())
        .and_then(|index| targets.get(index))
        .cloned()
}

fn fixed_cost_filter(effect: &MoveToZoneEffect) -> Option<(&ObjectFilter, usize)> {
    let ChooseSpec::Object(filter) = effect.target.base() else {
        return None;
    };
    let count = effect.target.count();
    if count.min == 0 || count.max != Some(count.min) {
        return None;
    }
    Some((filter, count.min))
}

fn matching_cost_candidate_count(
    game: &GameState,
    filter: &ObjectFilter,
    source: crate::ids::ObjectId,
    controller: crate::ids::PlayerId,
) -> usize {
    let filter_ctx = FilterContext::new(controller).with_source(source);
    let Some(zone) = filter.zone else {
        return 0;
    };

    // Peers cannot evaluate hidden hand cards; count their placeholders as
    // payable (see `game_state::hidden_hand_choices`).
    let placeholders = if zone == crate::zone::Zone::Hand {
        game.hidden_hand_payable_placeholders(filter, &filter_ctx, game.zone_ids(zone))
    } else {
        Vec::new()
    };
    game.zone_ids(zone)
        .filter(|id| {
            (placeholders.contains(id) && *id != source)
                || game
                    .object(*id)
                    .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        })
        .count()
}

impl EffectExecutor for MoveToZoneEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        // Moving the source itself ("exile ~") involves no player choices;
        // tagged objects selected by a preceding read-only choice are also
        // fully determined before the simultaneous commit. An all-matching
        // filter also chooses no targets and uses the shared batch commit.
        // Targeted and counted specs can prompt and remain outside this path.
        matches!(
            self.target.base(),
            ChooseSpec::Source | ChooseSpec::Tagged(_) | ChooseSpec::All(_)
        )
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
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
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::context::ExecutionContextCheckpoint::capture(ctx);
        // CR 603.10a: objects this instruction moves together share one
        // pre-event look-back, so a leaves-the-battlefield observer moved in
        // the same event sees every other object leave.
        let pinned_lookback = (self.zone != Zone::Battlefield
            && (!self.target.is_single() || matches!(self.target.base(), ChooseSpec::Tagged(_))))
            && crate::effects::helpers::begin_simultaneous_zone_change_lookback(game);
        let outcome = self.execute_with_shared_lookback(game, ctx);
        crate::effects::helpers::end_simultaneous_zone_change_lookback(game, pinned_lookback);
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || outcome.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending {
            return Ok(EffectOutcome::count(0));
        }
        outcome
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to move"
    }
}

trait SharedLookbackExecute {
    fn execute_with_shared_lookback(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}

impl SharedLookbackExecute for MoveToZoneEffect {
    fn execute_with_shared_lookback(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let moves_source = matches!(self.target.base(), ChooseSpec::Source);
        if moves_source && crate::effects::helpers::resolve_source_object_id(game, ctx).is_none() {
            return Ok(EffectOutcome::target_invalid());
        }
        // "That player puts one of them back on top of their library": a
        // counted pick out of an earlier collection is made by the player
        // performing the instruction, not by the ability's controller.
        let actor_pick = !self.target.is_target()
            && matches!(
                self.target,
                ChooseSpec::WithCount(..) | ChooseSpec::WithCountValue(..)
            );
        let actor = match (&self.actor_surface, actor_pick) {
            (Some(actor), true) => Some(crate::effects::helpers::resolve_player_filter_as_chooser(
                game, actor, ctx,
            )?),
            _ => None,
        };
        let saved_iterated_player = ctx.iteration.iterated_player;
        if let Some(actor) = actor {
            ctx.iteration.iterated_player = Some(actor);
        }
        let resolved = resolve_objects_for_effect(game, ctx, &self.target);
        ctx.iteration.iterated_player = saved_iterated_player;
        let mut object_ids = resolved?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        // When a tag snapshot carries a stale ObjectId (the tagged object
        // changed zones since the snapshot was taken), resolve through
        // stable_id so the move can find the actual game object.
        if let ChooseSpec::Tagged(tag) = &self.target
            && let Some(tagged) = ctx.get_tagged_all(tag)
        {
            for (idx, snapshot) in tagged.iter().enumerate() {
                if idx < object_ids.len()
                    && game.object(object_ids[idx]).is_none()
                    && let Some(resolved) = resolve_tagged_object_id(game, ctx, snapshot)
                {
                    object_ids[idx] = resolved;
                }
            }
        }
        // CR 726.4: after a restart, "then put those cards onto the
        // battlefield" happens once the new game's starting procedure is done.
        if ctx.restarted_game && self.zone == Zone::Battlefield {
            let controller = match self.battlefield_controller {
                BattlefieldController::You => Some(ctx.controller),
                BattlefieldController::Owner | BattlefieldController::Preserve => None,
            };
            game.defer_restart_battlefield_entry(
                crate::game_state::PendingRestartBattlefieldEntry {
                    cards: object_ids,
                    controller,
                    enters_tapped: self.enters_tapped,
                },
            );
            return Ok(EffectOutcome::resolved());
        }
        // CR 400.7: an object that has left the zone its trigger recorded is
        // a new object. A tagged reference to the triggering object follows
        // the physical card by stable identity, so it must stop at the zone
        // the trigger named — "When this creature dies, put it on the bottom
        // of its owner's library" does nothing once it has left the graveyard.
        if let ChooseSpec::Tagged(tag) = self.target.base()
            && let Some(event) = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| event.downcast::<crate::events::ZoneChangeEvent>())
            && let Some(tagged) = ctx.get_tagged_all(tag).cloned()
        {
            // An object this same resolution exiled is still findable by the
            // rest of the effect (CR 400.7 exception): "exile it and ... . If
            // you do, ... put it back on top of your library."
            let exiled_this_resolution = ctx
                .get_tagged_all(SOURCE_EXILED_TAG)
                .map(|snapshots| {
                    snapshots
                        .iter()
                        .map(|snapshot| snapshot.object_id)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            object_ids.retain(|id| {
                let Some(object) = game.object(*id) else {
                    return true;
                };
                if exiled_this_resolution.contains(id) {
                    return true;
                }
                let names_triggering_object = tagged
                    .iter()
                    .any(|snapshot| snapshot.stable_id == object.stable_id)
                    && event
                        .snapshots
                        .iter()
                        .any(|snapshot| snapshot.stable_id == object.stable_id);
                if !names_triggering_object {
                    return true;
                }
                // The object the event recorded, still where it recorded it.
                // A card that left and returned is a new object again, even
                // though it is back in the same zone.
                let recorded = if event.result_objects.is_empty() {
                    event.objects.contains(id)
                } else {
                    event.result_objects.contains(id)
                };
                let tagged_arrival = tagged
                    .iter()
                    .any(|snapshot| snapshot.object_id == *id && snapshot.zone == event.to);
                (recorded || tagged_arrival) && object.zone == event.to
            });
        }

        if object_ids.is_empty() {
            // Tagged references are internal results of earlier instructions,
            // not declared targets. If an earlier search/reveal found no
            // object, moving "that card" simply does nothing and later
            // instructions in the same sequence still resolve.
            return if matches!(self.target.base(), ChooseSpec::Tagged(_)) {
                Ok(EffectOutcome::count(0))
            } else {
                Ok(EffectOutcome::target_invalid())
            };
        }
        let orders_library = self.zone == Zone::Library && self.library_order.is_some();
        if let Some(order) = self.library_order.as_ref()
            && self.zone == Zone::Library
        {
            object_ids = order_library_move_objects(game, ctx, object_ids, order, self.to_top)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
        }
        let configured_attack_player = match &self.attack_target_mode {
            Some(MoveToZoneAttackTargetMode::PlayerOrPlaneswalkerControlledBy(player_filter)) => {
                Some(resolve_player_filter(game, player_filter, ctx)?)
            }
            None => None,
        };

        let mut moved_ids = Vec::new();
        let mut affected_ids = Vec::new();
        let mut affected_memory = Vec::new();
        let mut any_prevented = false;
        let mut any_replaced = false;
        let mut any_unchanged = false;
        let mut moved_source_lki = None;
        let mut ordered_library_results = Vec::new();
        let mut battlefield_entries = Vec::new();
        // "... onto the battlefield attached to X" (the attach instruction
        // that follows this move names X).
        let entry_attachment = if self.zone == Zone::Battlefield {
            ctx.pending_entry_attachment.clone().and_then(|spec| {
                crate::effects::permanents::resolve_entry_attachment_target(game, &spec, ctx)
            })
        } else {
            None
        };

        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let pre_event_lookback = game.trigger_source_lookback_snapshots();
        let original_snapshots = object_ids
            .iter()
            .filter_map(|id| {
                game.object(*id).map(|object| {
                    (
                        *id,
                        ObjectSnapshot::from_object_with_calculated_characteristics(object, game),
                    )
                })
            })
            .collect::<std::collections::HashMap<_, _>>();
        // Resolve authored counters while every selected object still exists.
        let mut authored_counters = std::collections::HashMap::new();
        for id in &object_ids {
            if game.object(*id).is_none() {
                continue;
            }
            authored_counters.insert(
                *id,
                resolve_battlefield_entry_counters(game, ctx, *id, &self.enters_with_counters)?,
            );
        }
        let mut zone_entry_proposals = std::collections::HashMap::new();
        let original_order = object_ids.clone();
        let mut zone_receipts = Vec::new();
        let mut authored_facts = Vec::new();
        let opened_batch = game.open_simultaneous_action();
        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;
            let requested_zone = ctx
                .simultaneous_zone_destination(object_id)
                .unwrap_or(self.zone);
            // CR 303.4i: an Aura an effect would put onto the battlefield
            // attached to something it can't legally enchant stays where it
            // is; it doesn't enter and choose some other object instead.
            if requested_zone == Zone::Battlefield
                && from_zone != Zone::Battlefield
                && let Some(target) = entry_attachment
                && obj.subtypes.contains(&crate::types::Subtype::Aura)
            {
                let entering_controller = match self.battlefield_controller {
                    BattlefieldController::You => ctx.controller,
                    BattlefieldController::Owner | BattlefieldController::Preserve => obj.owner,
                };
                if !crate::effects::permanents::aura_can_enter_attached_to(
                    game,
                    object_id,
                    entering_controller,
                    target,
                ) {
                    continue;
                }
            }
            let source_lki_before_move = if moves_source && object_id == ctx.source {
                Some(
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        obj, game,
                    ),
                )
            } else {
                None
            };
            let target_lki_before_move =
                crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                    obj, game,
                );
            let additional_effects = ctx.additional_replacement_effects_snapshot();

            let scope = ReplacementEventContext::with_scope(
                &game,
                crate::events::Event::zone_change(
                    object_id,
                    from_zone,
                    requested_zone,
                    ctx.cause.clone(),
                    original_snapshots.get(&object_id).cloned(),
                )
                .with_provenance(ctx.provenance),
                &ctx.replacement,
            );
            let result = prepare_zone_change_proposal_scoped(
                game,
                object_id,
                from_zone,
                requested_zone,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
                &additional_effects,
                original_snapshots.get(&object_id).cloned(),
                Some(&scope),
                Some(&pre_event_lookback),
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }

            let PreparedEventOutcome {
                original: result,
                mut programs,
            } = result;
            match result {
                EventOutcome::Prevented => {
                    any_prevented = true;
                    zone_receipts.push((
                        object_id,
                        PreparedEventOutcome {
                            original: EventOutcome::Prevented,
                            programs,
                        },
                    ));
                    continue;
                }
                EventOutcome::Proceed(PreparedZoneProposal::Battlefield(mut proposal)) => {
                    proposal.prepend_programs(programs);
                    let options = match self.battlefield_controller {
                        BattlefieldController::Preserve => {
                            BattlefieldEntryOptions::preserve(self.enters_tapped)
                        }
                        BattlefieldController::Owner => {
                            BattlefieldEntryOptions::owner(self.enters_tapped)
                        }
                        BattlefieldController::You => {
                            BattlefieldEntryOptions::specific(ctx.controller, self.enters_tapped)
                        }
                    };
                    let initial_counters = authored_counters.remove(&object_id).unwrap_or_default();
                    let options = options
                        .with_initial_counters(initial_counters)
                        .with_entry_attachment(entry_attachment)
                        .transformed(self.enters_transformed);
                    if self.enters_face_down
                        && let Some(card) = game.object_mut(object_id)
                    {
                        card.apply_face_down_cast_overlay();
                    }
                    zone_entry_proposals.insert(object_id, proposal);
                    battlefield_entries.push((
                        object_id,
                        options,
                        target_lki_before_move,
                        source_lki_before_move,
                    ));
                    continue;
                }
                EventOutcome::Proceed(PreparedZoneProposal::Ready(prepared)) => {
                    let replacement_context = prepared.context.clone();
                    let mut committed = commit_prepared_zone_change(
                        game,
                        object_id,
                        prepared,
                        &mut *ctx.decision_maker,
                    )?;
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::count(0));
                    }
                    programs.append(&mut committed.programs);
                    let new_object_id = match committed.original {
                        EventOutcome::Proceed(id) => Some(id),
                        EventOutcome::Prevented => {
                            any_prevented = true;
                            zone_receipts.push((
                                object_id,
                                PreparedEventOutcome {
                                    original: EventOutcome::Prevented,
                                    programs,
                                },
                            ));
                            continue;
                        }
                        EventOutcome::NotApplicable => {
                            any_unchanged = true;
                            affected_ids.push(object_id);
                            affected_memory
                                .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                            zone_receipts.push((
                                object_id,
                                PreparedEventOutcome {
                                    original: EventOutcome::NotApplicable,
                                    programs,
                                },
                            ));
                            continue;
                        }
                        EventOutcome::Replaced => {
                            any_replaced = true;
                            if let Some(change) = take_recorded_zone_change(game, object_id) {
                                game.record_zone_change_results(
                                    object_id,
                                    change.new_object_ids.clone(),
                                );
                                if orders_library && change.final_zone == Zone::Library {
                                    ordered_library_results
                                        .extend(change.new_object_ids.iter().copied());
                                }
                                affected_ids.extend(change.new_object_ids);
                            }
                            affected_memory
                                .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                            zone_receipts.push((
                                object_id,
                                PreparedEventOutcome {
                                    original: EventOutcome::Replaced,
                                    programs,
                                },
                            ));
                            continue;
                        }
                    };
                    let final_zone = game
                        .object(new_object_id.unwrap())
                        .map(|card| card.zone)
                        .ok_or_else(|| {
                            ExecutionError::InternalError(
                                "move arrival disappeared before authored work".into(),
                            )
                        })?;
                    let mut new_object_ids = game.take_zone_change_results(object_id);
                    if new_object_ids.is_empty()
                        && let Some(id) = new_object_id
                    {
                        new_object_ids.push(id);
                    }
                    let mut result = super::AppliedZoneChange {
                        final_zone,
                        new_object_id,
                        new_object_ids,
                    };
                    if final_zone == Zone::Hand {
                        for &id in &result.new_object_ids {
                            let arriving = game.object(id).ok_or_else(|| {
                                ExecutionError::InternalError(
                                    "hand arrival disappeared before result capture".into(),
                                )
                            })?;
                            let snapshot = ObjectSnapshot::from_object(arriving, game);
                            if arriving.kind == crate::object::ObjectKind::Card {
                                let memory = OutcomeObjectMemory::from_snapshot(&snapshot);
                                authored_facts.push(
                                    crate::effect::ExecutionFact::CardsPutIntoHand {
                                        player: arriving.owner,
                                        cards: vec![memory],
                                    },
                                );
                            }
                        }
                    }
                    // Retain the exact arrival identities for enclosing replacement owners.
                    game.record_zone_change_results(object_id, result.new_object_ids.clone());
                    if !result.new_object_ids.is_empty() {
                        ctx.refresh_target_snapshot(target_lki_before_move.clone());
                        affected_memory
                            .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                        if let Some(snapshot) = source_lki_before_move.clone() {
                            moved_source_lki = Some(snapshot);
                        }
                    }
                    if !result.new_object_ids.is_empty() {
                        // Counters authored as part of the move ("Exile this
                        // with three time counters on it") belong to the object
                        // that arrives, not the one that left.
                        let arriving_counters =
                            authored_counters.remove(&object_id).unwrap_or_default();
                        for &new_id in &result.new_object_ids {
                            for &(counter_type, amount) in &arriving_counters {
                                let mut counter_ctx = ExecutionContext::new(
                                    ctx.source,
                                    ctx.controller,
                                    &mut *ctx.decision_maker,
                                );
                                counter_ctx.source_snapshot = source_lki_before_move
                                    .clone()
                                    .or_else(|| ctx.source_snapshot.clone());
                                counter_ctx.cause = ctx.cause.clone();
                                counter_ctx.provenance = ctx.provenance;
                                counter_ctx.replacement.additional_replacement_effects =
                                    additional_effects.clone();
                                replacement_context.apply_to(&mut counter_ctx);
                                let event = crate::events::Event::put_counters(
                                    new_id,
                                    counter_type,
                                    amount,
                                    counter_ctx.cause.clone(),
                                )
                                .with_provenance(counter_ctx.provenance);
                                let mut outcome =
                                    crate::effects::counters::execute_object_counter_placement(
                                        game,
                                        &mut counter_ctx,
                                        event,
                                    )?;
                                if counter_ctx.decision_maker.awaiting_choice() {
                                    return Ok(EffectOutcome::count(0));
                                }
                                crate::effects::retain_unmatched_outcome_events(
                                    game,
                                    &mut outcome.events,
                                );
                                authored_facts.append(&mut outcome.execution_facts);
                                for event in outcome.events {
                                    game.queue_trigger_event(event.provenance(), event);
                                }
                            }
                            if final_zone == Zone::Exile {
                                game.add_exiled_with_source_link(ctx.source, new_id);
                                if let Some(object) = game.object(new_id) {
                                    ctx.tag_source_exiled_result(ObjectSnapshot::from_object(
                                        object, game,
                                    ));
                                }
                            }
                            if final_zone == Zone::Library
                                && !self.to_top
                                && let Some(owner) = game.object(new_id).map(|obj| obj.owner)
                            {
                                game.move_library_card_to_bottom(
                                    owner,
                                    new_id,
                                    "card put on bottom of library",
                                );
                            }
                        }
                        if final_zone == Zone::Library && from_zone == Zone::Battlefield {
                            maybe_prompt_for_split_result_order(
                                game,
                                &mut ctx.decision_maker,
                                final_zone,
                                &ctx.cause,
                                &mut result,
                            );
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(EffectOutcome::count(0));
                            }
                            game.record_zone_change_results(
                                object_id,
                                result.new_object_ids.clone(),
                            );
                        }
                        affected_ids.extend(result.new_object_ids.iter().copied());
                        if orders_library && final_zone == Zone::Library {
                            ordered_library_results.extend(result.new_object_ids.iter().copied());
                        }
                        moved_ids.extend(result.new_object_ids.iter().copied());
                        zone_receipts.push((
                            object_id,
                            PreparedEventOutcome {
                                original: EventOutcome::Proceed(result),
                                programs,
                            },
                        ));
                        continue;
                    }
                    zone_receipts.push((
                        object_id,
                        PreparedEventOutcome {
                            original: EventOutcome::NotApplicable,
                            programs,
                        },
                    ));
                    continue;
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                    if let Some(result) = take_recorded_zone_change(game, object_id) {
                        game.record_zone_change_results(object_id, result.new_object_ids.clone());
                        if orders_library && result.final_zone == Zone::Library {
                            ordered_library_results.extend(result.new_object_ids.iter().copied());
                        }
                        affected_ids.extend(result.new_object_ids);
                    }
                    affected_memory
                        .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                    zone_receipts.push((
                        object_id,
                        PreparedEventOutcome {
                            original: EventOutcome::Replaced,
                            programs,
                        },
                    ));
                }
                EventOutcome::NotApplicable => {
                    if game
                        .object(object_id)
                        .is_some_and(|object| object.zone == from_zone)
                    {
                        any_unchanged = true;
                        affected_ids.push(object_id);
                        affected_memory
                            .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                    }
                    // Positioning a card within its current library preserves
                    // object identity and does not count as a zone movement.
                    if self.zone == Zone::Library
                        && from_zone == Zone::Library
                        && game
                            .object(object_id)
                            .is_some_and(|object| object.zone == Zone::Library)
                    {
                        ordered_library_results.push(object_id);
                    }
                    zone_receipts.push((
                        object_id,
                        PreparedEventOutcome {
                            original: EventOutcome::NotApplicable,
                            programs,
                        },
                    ));
                    continue;
                }
            }
        }

        if !battlefield_entries.is_empty() {
            let entry_outcomes = move_to_battlefield_batch_with_options_and_zone_proposals(
                game,
                ctx,
                battlefield_entries
                    .iter()
                    .map(|(object, options, _, _)| (*object, options.clone()))
                    .collect(),
                zone_entry_proposals,
            )?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if entry_outcomes.len() != battlefield_entries.len() {
                return Err(ExecutionError::InternalError(
                    "move entry batch lost an original receipt".into(),
                ));
            }
            for ((object_id, _, target_lki_before_move, source_lki_before_move), receipt) in
                battlefield_entries.into_iter().zip(entry_outcomes)
            {
                let entry_outcome = receipt.outcome.clone();
                let (receipt_object, zone_receipt) = receipt.into_zone_receipt();
                if receipt_object != object_id {
                    return Err(ExecutionError::InternalError(
                        "move entry receipt changed original identity".into(),
                    ));
                }
                if matches!(&zone_receipt.original, EventOutcome::Replaced) {
                    any_replaced = true;
                }
                match entry_outcome {
                    BattlefieldEntryOutcome::Moved(new_id) => {
                        // CR 506.3a/b/f, 508.4: only a creature controlled by
                        // an attacking player, during combat, becomes attacking.
                        if self.enters_attacking
                            && crate::effects::combat::can_enter_attacking(game, new_id)
                        {
                            let target = if let Some(attack_player) = configured_attack_player {
                                let targets = attack_targets_for_player(game, attack_player);
                                choose_attack_target_for_player(game, ctx, attack_player, &targets)
                            } else {
                                crate::effects::combat::choose_enters_attacking_target(
                                    game, ctx, new_id,
                                )
                            };
                            if ctx.decision_maker.awaiting_choice() {
                                return Ok(EffectOutcome::count(0));
                            }
                            if let Some(target) = target {
                                game.add_entering_attacker(new_id, target);
                            }
                        }
                        ctx.refresh_target_snapshot(target_lki_before_move.clone());
                        affected_memory
                            .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                        if let Some(snapshot) = source_lki_before_move {
                            moved_source_lki = Some(snapshot);
                        }
                        affected_ids.push(new_id);
                        moved_ids.push(new_id);
                    }
                    BattlefieldEntryOutcome::Redirected(receipt) => {
                        ctx.refresh_target_snapshot(target_lki_before_move.clone());
                        affected_memory
                            .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                        if let Some(snapshot) = source_lki_before_move {
                            moved_source_lki = Some(snapshot);
                        }
                        affected_ids.extend(receipt.new_object_ids.iter().copied());
                        moved_ids.extend(receipt.new_object_ids);
                    }
                    BattlefieldEntryOutcome::Prevented => {
                        if self.enters_face_down
                            && let Some(card) = game.object_mut(object_id)
                        {
                            card.end_face_down_cast_overlay();
                        }
                        if !matches!(&zone_receipt.original, EventOutcome::Replaced) {
                            any_prevented = true;
                        }
                    }
                }
                zone_receipts.push((object_id, zone_receipt));
            }
        }

        if moves_source && let Some(new_source_id) = moved_ids.first().copied() {
            let old_source_id = ctx.source;
            if self.transfer_exiled_with_source_links {
                game.transfer_exiled_with_source_links(old_source_id, new_source_id);
            }
            ctx.source = new_source_id;
        }
        if let Some(snapshot) = moved_source_lki {
            ctx.refresh_source_snapshot(snapshot);
        }
        if !ordered_library_results.is_empty() {
            apply_library_placement_order(game, &ordered_library_results, self.to_top);
        }

        game.close_simultaneous_action(opened_batch);
        let mut original = (|| -> EffectOutcome {
            if !moved_ids.is_empty() {
                let mut outcome =
                    EffectOutcome::with_objects(moved_ids).with_affected_objects(affected_ids);
                if !affected_memory.is_empty() {
                    outcome = outcome.with_affected_object_memory(affected_memory);
                }
                return outcome;
            }
            if any_prevented {
                return EffectOutcome::prevented();
            }
            if any_replaced {
                let mut outcome = EffectOutcome::replaced().with_affected_objects(affected_ids);
                if !affected_memory.is_empty() {
                    outcome = outcome.with_affected_object_memory(affected_memory);
                }
                return outcome;
            }
            if any_unchanged {
                return EffectOutcome::count(0)
                    .with_affected_objects(affected_ids)
                    .with_affected_object_memory(affected_memory);
            }
            EffectOutcome::target_invalid()
        })();
        original.execution_facts.append(&mut authored_facts);
        // All original moves and authored work are complete. Keep replacement
        // programs in the original object order across ordinary and entry paths.
        zone_receipts.sort_by_key(|(object, _)| {
            original_order
                .iter()
                .position(|id| id == object)
                .unwrap_or(usize::MAX)
        });
        super::finish_zone_change_receipts(game, ctx, original, zone_receipts)
    }
}

impl CostExecutableEffect for MoveToZoneEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), CostValidationError> {
        if matches!(self.target.base(), ChooseSpec::Source) && game.object(source).is_some() {
            return Ok(());
        }

        if let Some((filter, count)) = fixed_cost_filter(self) {
            let matching = matching_cost_candidate_count(game, filter, source, controller);
            if matching >= count {
                return Ok(());
            }
            return Err(CostValidationError::NotEnoughCards);
        }

        Err(CostValidationError::Other(
            "unsupported move-to-zone cost".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;
    use crate::events::zones::matchers::WouldGoToGraveyardMatcher;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, owner: PlayerId) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), "Move Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(id, &card, owner, Zone::Battlefield));
        id
    }

    fn create_named_creature_in_zone(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        zone: Zone,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .build();
        game.add_object(Object::from_card(id, &card, owner, zone));
        id
    }

    struct ChooseLastOptionDecisionMaker;

    impl crate::decision::DecisionMaker for ChooseLastOptionDecisionMaker {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .filter(|option| option.legal)
                .next_back()
                .map(|option| vec![option.index])
                .unwrap_or_default()
        }
    }

    #[test]
    fn paladin_elizabeth_taggerdy_move_enters_tapped_and_attacking_chosen_defender() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Cara".to_string()],
            20,
        );
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let cara = PlayerId::from_index(2);
        let paladin = create_named_creature_in_zone(
            &mut game,
            alice,
            "Paladin Elizabeth Taggerdy",
            Zone::Battlefield,
        );
        let other_attacker =
            create_named_creature_in_zone(&mut game, alice, "Wasteland Raider", Zone::Battlefield);
        let vault_dweller =
            create_named_creature_in_zone(&mut game, alice, "Vault Dweller", Zone::Hand);
        game.combat = Some(CombatState {
            attackers: vec![
                AttackerInfo {
                    creature: paladin,
                    target: AttackTarget::Player(bob),
                },
                AttackerInfo {
                    creature: other_attacker,
                    target: AttackTarget::Player(cara),
                },
            ],
            ..CombatState::default()
        });
        game.turn.phase = crate::game_state::Phase::Combat;

        let mut decision_maker = ChooseLastOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(paladin, alice, &mut decision_maker);
        let outcome = MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(vault_dweller),
            Zone::Battlefield,
            false,
        )
        .tapped()
        .attacking()
        .execute(&mut game, &mut ctx)
        .expect("Paladin Elizabeth Taggerdy move should resolve");

        let moved = outcome
            .affected_objects()
            .and_then(|ids| ids.first().copied())
            .or_else(|| match outcome.value {
                crate::effect::OutcomeValue::Objects(ref ids) => ids.first().copied(),
                _ => None,
            })
            .expect("moved creature id should be reported");
        assert!(game.battlefield.contains(&moved));
        assert!(game.is_tapped(moved), "moved creature should enter tapped");
        let combat = game.combat.as_ref().expect("combat should remain active");
        let moved_attacker = combat
            .attackers
            .iter()
            .find(|info| info.creature == moved)
            .expect("moved creature should enter attacking");
        assert_eq!(moved_attacker.target, AttackTarget::Player(cara));
    }

    #[test]
    fn paladin_elizabeth_taggerdy_move_without_active_combat_does_not_attack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let paladin = create_named_creature_in_zone(
            &mut game,
            alice,
            "Paladin Elizabeth Taggerdy",
            Zone::Battlefield,
        );
        let vault_dweller =
            create_named_creature_in_zone(&mut game, alice, "Vault Dweller", Zone::Hand);

        let mut ctx = ExecutionContext::new_default(paladin, alice);
        let outcome = MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(vault_dweller),
            Zone::Battlefield,
            false,
        )
        .tapped()
        .attacking()
        .execute(&mut game, &mut ctx)
        .expect("Paladin Elizabeth Taggerdy move should resolve outside combat");

        let moved = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            _ => panic!("expected moved object id"),
        };
        assert!(game.battlefield.contains(&moved));
        assert!(
            game.is_tapped(moved),
            "moved creature should still enter tapped"
        );
        assert!(
            game.combat.is_none(),
            "no attacker should be added without combat"
        );
    }

    #[test]
    fn battlefield_move_records_exact_source_relative_object_identity() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source =
            create_named_creature_in_zone(&mut game, alice, "Linked Source", Zone::Battlefield);
        let other_source =
            create_named_creature_in_zone(&mut game, alice, "Other Source", Zone::Battlefield);
        let card = create_named_creature_in_zone(&mut game, alice, "Linked Card", Zone::Hand);

        let mut source_ctx = ExecutionContext::new_default(source, alice);
        let moved =
            MoveToZoneEffect::new(ChooseSpec::SpecificObject(card), Zone::Battlefield, false)
                .execute(&mut game, &mut source_ctx)
                .expect("source move should resolve")
                .affected_objects()
                .and_then(|objects| objects.first().copied())
                .expect("battlefield move should report the new identity");

        let mut linked_filter = ObjectFilter::creature().in_zone(Zone::Battlefield);
        linked_filter.put_onto_battlefield_with_source = true;
        let filter_ctx = source_ctx.filter_context(&game);
        assert!(linked_filter.matches(
            game.object(moved).expect("moved card should exist"),
            &filter_ctx,
            &game,
        ));
        assert!(
            !linked_filter.matches(
                game.object(other_source)
                    .expect("other source should exist"),
                &filter_ctx,
                &game,
            )
        );

        // A later zone change produces a new identity. If another source puts
        // that card back, it must not satisfy the original source's link.
        let mut other_ctx = ExecutionContext::new_default(other_source, alice);
        let in_hand = MoveToZoneEffect::new(ChooseSpec::SpecificObject(moved), Zone::Hand, false)
            .execute(&mut game, &mut other_ctx)
            .expect("move out should resolve")
            .affected_objects()
            .and_then(|objects| objects.first().copied())
            .expect("move out should report the new identity");
        let returned = MoveToZoneEffect::new(
            ChooseSpec::SpecificObject(in_hand),
            Zone::Battlefield,
            false,
        )
        .execute(&mut game, &mut other_ctx)
        .expect("other source move should resolve")
        .affected_objects()
        .and_then(|objects| objects.first().copied())
        .expect("return should report the new identity");

        let filter_ctx = source_ctx.filter_context(&game);
        assert!(!linked_filter.matches(
            game.object(returned).expect("returned card should exist"),
            &filter_ctx,
            &game,
        ));
    }

    #[test]
    fn counted_source_linked_move_chooses_only_one_matching_exiled_card() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source =
            create_named_creature_in_zone(&mut game, alice, "Linked Source", Zone::Battlefield);
        let first = create_named_creature_in_zone(&mut game, alice, "First Linked", Zone::Exile);
        let second = create_named_creature_in_zone(&mut game, alice, "Second Linked", Zone::Exile);
        game.add_exiled_with_source_link(source, first);
        game.add_exiled_with_source_link(source, second);

        let target =
            ChooseSpec::Object(ObjectFilter::tagged(SOURCE_EXILED_TAG).in_zone(Zone::Exile))
                .with_count(crate::effect::ChoiceCount::exactly(1));
        let mut decision_maker = ChooseLastOptionDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
        MoveToZoneEffect::new(target, Zone::Hand, false)
            .execute(&mut game, &mut ctx)
            .expect("one linked exiled card should move");

        let exiled_names = game
            .zone_ids(Zone::Exile)
            .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
            .collect::<Vec<_>>();
        let hand_names = game
            .zone_ids(Zone::Hand)
            .filter_map(|id| game.object(id).map(|object| object.name.to_string()))
            .collect::<Vec<_>>();
        assert_eq!(exiled_names.len(), 1);
        assert_eq!(hand_names.len(), 1);
        let remaining_and_moved = exiled_names
            .iter()
            .chain(hand_names.iter())
            .cloned()
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            remaining_and_moved,
            std::collections::HashSet::from([
                "First Linked".to_string(),
                "Second Linked".to_string(),
            ])
        );
    }

    #[test]
    fn batch_move_applies_each_conditional_entry_counter_to_matching_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source =
            create_named_creature_in_zone(&mut game, alice, "Semester Source", Zone::Battlefield);
        let _creature =
            create_named_creature_in_zone(&mut game, alice, "Returning Creature", Zone::Exile);

        let planeswalker = game.new_object_id();
        let planeswalker_card = CardBuilder::new(
            CardId::from_raw(planeswalker.0 as u32),
            "Returning Planeswalker",
        )
        .card_types(vec![CardType::Planeswalker])
        .build();
        game.add_object(Object::from_card(
            planeswalker,
            &planeswalker_card,
            alice,
            Zone::Exile,
        ));

        let creature_counter = ironsmith_core::BattlefieldEntryCounterSpec::new(
            CounterType::PlusOnePlusOne,
            crate::effect::Value::Fixed(1),
            ironsmith_core::BattlefieldEntryCounterSurface::EachOfThemEnters,
        )
        .for_matching_object(ObjectFilter::default().with_type(CardType::Creature));
        let loyalty_counter = ironsmith_core::BattlefieldEntryCounterSpec::new(
            CounterType::Loyalty,
            crate::effect::Value::Fixed(1),
            ironsmith_core::BattlefieldEntryCounterSurface::EachOfThemEnters,
        )
        .for_matching_object(ObjectFilter::default().with_type(CardType::Planeswalker));

        let mut ctx = ExecutionContext::new_default(source, alice);
        MoveToZoneEffect::new(
            ChooseSpec::All(ObjectFilter::default().in_zone(Zone::Exile)),
            Zone::Battlefield,
            false,
        )
        .under_owner_control()
        .with_entry_counter(creature_counter)
        .with_entry_counter(loyalty_counter)
        .execute(&mut game, &mut ctx)
        .expect("aggregate return should resolve");

        let returned_creature = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Returning Creature")
            })
            .expect("creature should return");
        let returned_planeswalker = game
            .battlefield
            .iter()
            .copied()
            .find(|id| {
                game.object(*id)
                    .is_some_and(|object| object.name == "Returning Planeswalker")
            })
            .expect("planeswalker should return");

        assert_eq!(
            game.counter_count(returned_creature, CounterType::PlusOnePlusOne),
            1
        );
        assert_eq!(
            game.counter_count(returned_creature, CounterType::Loyalty),
            0
        );
        assert_eq!(
            game.counter_count(returned_planeswalker, CounterType::PlusOnePlusOne),
            0
        );
        assert_eq!(
            game.counter_count(returned_planeswalker, CounterType::Loyalty),
            1
        );
    }

    #[test]
    fn non_target_move_to_zone_does_not_request_cast_time_targets() {
        let move_choice = MoveToZoneEffect::new(
            ChooseSpec::WithCount(
                Box::new(ChooseSpec::Object(crate::filter::ObjectFilter {
                    zone: Some(Zone::Exile),
                    ..crate::filter::ObjectFilter::default()
                })),
                crate::effect::ChoiceCount::exactly(1),
            ),
            Zone::Graveyard,
            false,
        );
        assert!(move_choice.target_selection_profile().is_none());

        let move_target = MoveToZoneEffect::new(
            ChooseSpec::target(ChooseSpec::Object(crate::filter::ObjectFilter {
                zone: Some(Zone::Battlefield),
                ..crate::filter::ObjectFilter::default()
            })),
            Zone::Graveyard,
            false,
        );
        assert!(move_target.target_selection_profile().is_some());
    }

    #[test]
    fn replaced_move_preserves_redirected_object_ids_in_outcome() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature = create_creature(&mut game, alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGoToGraveyardMatcher::new(crate::target::ObjectFilter::specific(creature)),
                ReplacementAction::Instead(vec![Effect::new(MoveToZoneEffect::to_exile(
                    ChooseSpec::SpecificObject(creature),
                ))]),
            ),
        );

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = MoveToZoneEffect::to_graveyard(ChooseSpec::SpecificObject(creature));
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Replaced);
        let affected = outcome
            .affected_objects()
            .expect("redirected object ids should be preserved");
        assert_eq!(affected.len(), 1);
        assert!(
            game.object(affected[0])
                .is_some_and(|obj| obj.zone == Zone::Exile && obj.name == "Move Probe")
        );
        assert!(game.players[0].graveyard.is_empty());
    }
    #[test]
    fn later_pending_destination_restores_all_moves_and_context_memory() {
        struct Answers {
            calls: usize,
            pause: bool,
            pending: bool,
        }
        impl crate::decision::DecisionMaker for Answers {
            fn decide_options(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                self.calls += 1;
                self.pending = self.pause && self.calls == 2;
                if self.pending { Vec::new() } else { vec![0] }
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let cards = (0..3)
            .map(|_| create_named_creature_in_zone(&mut game, alice, "Pending move", Zone::Hand))
            .collect::<Vec<_>>();
        let mut shields = Vec::new();
        for card in &cards {
            shields.push(game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    *card,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(*card),
                        Some(Zone::Hand),
                        Some(Zone::Graveyard),
                    ),
                    ReplacementAction::InteractiveChooseDestination {
                        destinations: vec![Zone::Exile, Zone::Graveyard],
                        description: "Choose destination".into(),
                    },
                ),
            ));
        }
        let snapshots = cards
            .iter()
            .map(|card| ObjectSnapshot::from_object(game.object(*card).unwrap(), &game))
            .collect::<Vec<_>>();
        let mut dm = Answers {
            calls: 0,
            pause: true,
            pending: false,
        };
        let mut ctx = ExecutionContext::new(cards[0], alice, &mut dm);
        ctx.target_snapshots.insert(cards[0], snapshots[0].clone());
        ctx.tag_objects("batch", snapshots);
        // Earlier LKI must also be restored when a provisional move refreshes
        // it from the object's more recent characteristics.
        game.object_mut(cards[0])
            .unwrap()
            .add_counters(CounterType::Charge, 3);
        let effect = MoveToZoneEffect::to_graveyard(ChooseSpec::Tagged("batch".into()));
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        for card in &cards {
            assert_eq!(
                game.object(*card).map(|object| object.zone),
                Some(Zone::Hand)
            );
        }
        for shield in &shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(*shield)
                    .is_some()
            );
        }
        assert!(ctx.get_tagged_all(SOURCE_EXILED_TAG).is_none());
        assert_eq!(
            ctx.target_snapshots[&cards[0]]
                .counters
                .get(&CounterType::Charge),
            None
        );
        assert_eq!(ctx.get_tagged_all("batch").unwrap().len(), 3);
        assert!(
            ctx.get_tagged_all("__source_exiled_this_resolution__")
                .is_none()
        );
        assert_eq!(outcome.count_or_zero(), 0);
        assert!(game.take_pending_trigger_events().is_empty());
        drop(ctx);
        assert_eq!(dm.calls, 2);
        dm.calls = 0;
        dm.pause = false;
        dm.pending = false;
        let mut ctx = ExecutionContext::new(cards[0], alice, &mut dm);
        let snapshots = cards
            .iter()
            .map(|card| ObjectSnapshot::from_object(game.object(*card).unwrap(), &game))
            .collect();
        ctx.tag_objects("batch", snapshots);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(outcome.affected_objects().unwrap().len(), 3);
        assert_eq!(ctx.get_tagged_all(SOURCE_EXILED_TAG).unwrap().len(), 3);
        for before in ctx.get_tagged_all("batch").unwrap() {
            assert!(outcome.affected_objects().unwrap().iter().any(|moved| {
                game.object(*moved).is_some_and(|object| {
                    object.stable_id == before.stable_id && object.zone == Zone::Exile
                })
            }));
        }
        for shield in &shields {
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(*shield)
                    .is_none()
            );
        }
        drop(ctx);
        assert_eq!(dm.calls, 3);
    }

    #[test]
    fn arriving_counter_error_restores_move_and_refreshed_context_snapshot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = create_named_creature_in_zone(&mut game, alice, "Counter error", Zone::Hand);
        let snapshot = ObjectSnapshot::from_object(game.object(card).unwrap(), &game);
        let mut ctx = ExecutionContext::new_default(card, alice);
        ctx.target_snapshots.insert(card, snapshot);
        game.object_mut(card)
            .unwrap()
            .add_counters(CounterType::Charge, 3);
        let effect = MoveToZoneEffect::to_exile(ChooseSpec::SpecificObject(card))
            .with_entry_counter(ironsmith_core::BattlefieldEntryCounterSpec::new(
                CounterType::Charge,
                crate::effect::Value::X,
                ironsmith_core::BattlefieldEntryCounterSurface::Inline,
            ));
        let error = effect.execute(&mut game, &mut ctx).unwrap_err();
        assert!(matches!(error, ExecutionError::UnresolvableValue(_)));
        assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
        assert!(game.exile.is_empty());
        assert_eq!(
            ctx.target_snapshots[&card]
                .counters
                .get(&CounterType::Charge),
            None
        );
        assert!(ctx.get_tagged_all(SOURCE_EXILED_TAG).is_none());
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn pending_library_order_stops_before_destination_replacement_prompt() {
        struct PendingOrder {
            pending: bool,
        }
        impl crate::decision::DecisionMaker for PendingOrder {
            fn decide_order(
                &mut self,
                _: &GameState,
                _: &OrderContext,
            ) -> Vec<crate::ids::ObjectId> {
                self.pending = true;
                Vec::new()
            }
            fn decide_options(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                panic!("no replacement prompt may follow an unanswered ordering prompt");
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let cards = (0..2)
            .map(|_| create_named_creature_in_zone(&mut game, alice, "Order pending", Zone::Hand))
            .collect::<Vec<_>>();
        let snapshots = cards
            .iter()
            .map(|card| ObjectSnapshot::from_object(game.object(*card).unwrap(), &game))
            .collect();
        for card in &cards {
            game.effect_store
                .replacement_effects
                .add_effect(ReplacementEffect::with_matcher(
                    *card,
                    alice,
                    crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                        ObjectFilter::specific(*card),
                        Some(Zone::Hand),
                        Some(Zone::Library),
                    ),
                    ReplacementAction::InteractiveChooseDestination {
                        destinations: vec![Zone::Exile, Zone::Library],
                        description: "Choose destination".into(),
                    },
                ));
        }
        let mut dm = PendingOrder { pending: false };
        let mut ctx = ExecutionContext::new(cards[0], alice, &mut dm);
        ctx.tag_objects("batch", snapshots);
        let mut effect =
            MoveToZoneEffect::new(ChooseSpec::Tagged("batch".into()), Zone::Library, true);
        effect.library_order = Some(LibraryPlacementOrder::ChosenBy(
            crate::target::PlayerFilter::You,
        ));
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(outcome.count_or_zero(), 0);
        for card in cards {
            assert_eq!(game.object(card).unwrap().zone, Zone::Hand);
        }
        assert!(game.take_pending_trigger_events().is_empty());
    }
}

#[cfg(test)]
mod replacement_move_owner_contract_tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, StableId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    struct Answers {
        originals: Vec<StableId>,
        destination: Zone,
        pause: bool,
        pending: bool,
        calls: usize,
    }
    impl crate::decision::DecisionMaker for Answers {
        fn decide_boolean(
            &mut self,
            game: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.calls += 1;
            assert!(
                self.originals.iter().all(|stable| game
                    .find_object_by_stable_id(*stable)
                    .is_some_and(|id| game.object(id).unwrap().zone == self.destination)),
                "replacement additions must see the entire original move batch committed"
            );
            self.pending = self.pause;
            !self.pending
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    fn card(game: &mut GameState, name: &str, owner: crate::ids::PlayerId, zone: Zone) -> ObjectId {
        let card = crate::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, zone)
    }
    fn check(battlefield: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let parent = card(&mut game, "Move parent", alice, Zone::Battlefield);
        let replacement_source = card(&mut game, "Move replacement", bob, Zone::Battlefield);
        let first = card(&mut game, "Move first", alice, Zone::Hand);
        let second = card(&mut game, "Move second", alice, Zone::Hand);
        let originals = vec![first, second];
        let stable = originals
            .iter()
            .map(|id| game.object(*id).unwrap().stable_id)
            .collect::<Vec<_>>();
        let destination = if battlefield {
            Zone::Battlefield
        } else {
            Zone::Graveyard
        };
        let effects = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(
                crate::object::CounterType::PlusOnePlusOne,
                1,
                ChooseSpec::tagged("it"),
            ))],
            _ => vec![Effect::may(vec![Effect::gain_life(7)])],
        };
        let action = if mode == 4 {
            ReplacementAction::Prevent
        } else {
            ReplacementAction::Additionally(effects)
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                replacement_source,
                bob,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::specific(first),
                    Some(Zone::Hand),
                    Some(destination),
                ),
                action,
            ),
        );
        let snapshots = originals
            .iter()
            .map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game))
            .collect::<Vec<_>>();
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let before_ids = game.next_object_id_counter();
        game.take_pending_trigger_events();
        let mut dm = Answers {
            originals: stable.clone(),
            destination,
            pause: mode == 2,
            pending: false,
            calls: 0,
        };
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
        ctx.set_tagged_objects("selected", snapshots);
        ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let effect =
            MoveToZoneEffect::new(ChooseSpec::tagged("selected"), destination, battlefield);
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 {
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
        } else if mode == 2 {
            assert!(ctx.decision_maker.awaiting_choice());
        } else {
            let outcome = result.unwrap();
            assert_eq!(
                outcome.output_objects().len(),
                if mode == 4 { 1 } else { 2 }
            );
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(
                game.player(bob).unwrap().life,
                if mode == 0 { 27 } else { 20 }
            );
            let arrived = game.find_object_by_stable_id(stable[0]).unwrap();
            if mode == 3 {
                assert_eq!(
                    game.counter_count(arrived, crate::object::CounterType::PlusOnePlusOne),
                    1
                );
                assert_eq!(
                    game.counter_count(parent, crate::object::CounterType::PlusOnePlusOne),
                    0
                );
            }
            if mode == 4 {
                assert_eq!(game.object(first).unwrap().zone, Zone::Hand);
            }
            let second_arrived = game.find_object_by_stable_id(stable[1]).unwrap();
            assert_eq!(game.object(second_arrived).unwrap().zone, destination);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
        }
        assert_eq!(
            ctx.get_tagged_all("it").unwrap()[0].object_id,
            sentinel.object_id
        );
        if mode == 1 || mode == 2 {
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert_eq!(game.player(alice).unwrap().life, 20);
            assert_eq!(game.player(bob).unwrap().life, 20);
            for id in &originals {
                assert_eq!(game.object(*id).unwrap().zone, Zone::Hand);
            }
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_some()
            );
            assert!(game.take_pending_trigger_events().is_empty());
        }
        if mode == 2 {
            drop(ctx);
            assert_eq!(dm.calls, 1);
            dm.pause = false;
            dm.pending = false;
            let snapshots = originals
                .iter()
                .map(|id| ObjectSnapshot::from_object(game.object(*id).unwrap(), &game))
                .collect::<Vec<_>>();
            let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
            ctx.set_tagged_objects("selected", snapshots);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(outcome.output_objects().len(), 2);
            assert_eq!(game.player(bob).unwrap().life, 27);
            assert!(!ctx.decision_maker.awaiting_choice());
            drop(ctx);
            assert_eq!(dm.calls, 2);
        }
    }
    #[test]
    fn graveyard_addition_sees_whole_batch() {
        check(false, 0);
    }
    #[test]
    fn graveyard_addition_error_restores_owner() {
        check(false, 1);
    }
    #[test]
    fn graveyard_addition_pending_replays_owner() {
        check(false, 2);
    }
    #[test]
    fn graveyard_addition_binds_arrival() {
        check(false, 3);
    }
    #[test]
    fn graveyard_prevention_does_not_abort_batch() {
        check(false, 4);
    }
    #[test]
    fn battlefield_addition_sees_whole_batch() {
        check(true, 0);
    }
    #[test]
    fn battlefield_addition_error_restores_owner() {
        check(true, 1);
    }
    #[test]
    fn battlefield_addition_pending_replays_owner() {
        check(true, 2);
    }
    #[test]
    fn battlefield_addition_binds_arrival() {
        check(true, 3);
    }
    #[test]
    fn battlefield_prevention_does_not_abort_batch() {
        check(true, 4);
    }
}

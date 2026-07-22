//! Move to zone effect implementation.

use crate::combat_state::{AttackTarget, AttackerInfo, CombatState};
use crate::decisions::context::{OrderContext, SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_player_filter, resolve_tagged_object_id,
};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{EventOutcome, process_zone_change_with_additional_effects};
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
    BattlefieldEntryOptions, BattlefieldEntryOutcome, finalize_zone_change_move,
    maybe_prompt_for_split_result_order, move_to_battlefield_batch_with_options,
    resolve_battlefield_entry_counters, take_recorded_zone_change,
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
                && object.has_card_type(CardType::Planeswalker)
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            } else if object.has_card_type(CardType::Battle)
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
    Some((filter, count.min as usize))
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

    game.zone_ids(zone)
        .filter(|id| {
            game.object(*id)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        })
        .count()
}

fn enters_attacking_targets(game: &GameState, combat: &CombatState) -> Vec<AttackTarget> {
    let mut defending_players = Vec::new();
    for attacker in &combat.attackers {
        let defending_player = match attacker.target {
            AttackTarget::Player(player) => Some(player),
            AttackTarget::Planeswalker(planeswalker) => game
                .object(planeswalker)
                .map(|object| game.controller_of(object)),
            AttackTarget::Battle(battle) => game.battle_protector(battle),
        };
        if let Some(player) = defending_player
            && !defending_players.contains(&player)
        {
            defending_players.push(player);
        }
    }

    let all_effects = game.all_continuous_effects();
    let mut targets = Vec::new();
    for defender in defending_players {
        targets.push(AttackTarget::Player(defender));
        for &object_id in &game.battlefield {
            let Some(object) = game.object(object_id) else {
                continue;
            };
            if game.controller_of(object) == defender
                && object.zone == Zone::Battlefield
                && game.object_has_card_type_with_effects(
                    object_id,
                    crate::types::CardType::Planeswalker,
                    &all_effects,
                )
            {
                targets.push(AttackTarget::Planeswalker(object_id));
            } else if game.object_has_card_type_with_effects(
                object_id,
                crate::types::CardType::Battle,
                &all_effects,
            ) && game.battle_protector(object_id) == Some(defender)
            {
                targets.push(AttackTarget::Battle(object_id));
            }
        }
    }
    targets
}

fn attack_target_description(game: &GameState, target: &AttackTarget) -> String {
    match target {
        AttackTarget::Player(player) => game
            .player(*player)
            .map(|player| player.name.to_string())
            .unwrap_or_else(|| format!("player {}", player.0)),
        AttackTarget::Planeswalker(object_id) => game
            .object(*object_id)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| format!("planeswalker #{}", object_id.0)),
        AttackTarget::Battle(object_id) => game
            .object(*object_id)
            .map(|object| object.name.to_string())
            .unwrap_or_else(|| format!("battle #{}", object_id.0)),
    }
}

fn choose_enters_attacking_target(
    game: &GameState,
    ctx: &mut ExecutionContext<'_>,
    moved_id: crate::ids::ObjectId,
) -> Option<AttackTarget> {
    let combat = game.combat.as_ref()?;
    let targets = enters_attacking_targets(game, combat);
    if targets.len() <= 1 {
        return targets.first().cloned();
    }

    let options = targets
        .iter()
        .enumerate()
        .map(|(index, target)| {
            crate::decisions::DisplayOption::new(index, attack_target_description(game, target))
        })
        .collect();
    let chooser = game
        .object(moved_id)
        .map(|object| game.controller_of(object))
        .unwrap_or(ctx.controller);
    let source = ctx.source;
    let selected = crate::decisions::make_decision(
        game,
        &mut *ctx.decision_maker,
        chooser,
        Some(source),
        crate::decisions::ChoiceSpec::single(source, options),
    );
    let selected_index = selected.into_iter().next().unwrap_or(0);
    targets
        .get(selected_index)
        .cloned()
        .or_else(|| targets.first().cloned())
}

impl EffectExecutor for MoveToZoneEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        // Moving the source itself ("exile ~") involves no player choices;
        // tagged objects selected by a preceding read-only choice are also
        // fully determined before the simultaneous commit.  Broader target
        // specs can still prompt and remain outside this path.
        matches!(
            self.target.base(),
            ChooseSpec::Source | ChooseSpec::Tagged(_)
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
        let moves_source = matches!(self.target.base(), ChooseSpec::Source);
        let mut object_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        // When a tag snapshot carries a stale ObjectId (the tagged object
        // changed zones since the snapshot was taken), resolve through
        // stable_id so the move can find the actual game object.
        if let ChooseSpec::Tagged(tag) = &self.target {
            if let Some(tagged) = ctx.get_tagged_all(tag) {
                for (idx, snapshot) in tagged.iter().enumerate() {
                    if idx < object_ids.len() && game.object(object_ids[idx]).is_none() {
                        if let Some(resolved) = resolve_tagged_object_id(game, snapshot) {
                            object_ids[idx] = resolved;
                        }
                    }
                }
            }
        }
        if object_ids.is_empty() {
            // Tagged references are internal results of earlier instructions,
            // not declared targets. If an earlier search/reveal found no
            // object, moving "that card" simply does nothing and later
            // instructions in the same sequence still resolve.
            return if matches!(self.target.base(), ChooseSpec::Tagged(_)) {
                Ok(EffectOutcome::resolved())
            } else {
                Ok(EffectOutcome::target_invalid())
            };
        }
        let orders_library = self.zone == Zone::Library && self.library_order.is_some();
        if let Some(order) = self.library_order.as_ref()
            && self.zone == Zone::Library
        {
            object_ids = order_library_move_objects(game, ctx, object_ids, order, self.to_top)?;
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
        let mut moved_source_lki = None;
        let mut ordered_library_results = Vec::new();
        let mut battlefield_entries = Vec::new();

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let stable_id = obj.stable_id;
            let from_zone = obj.zone;
            let requested_zone = ctx
                .simultaneous_zone_destination(object_id)
                .unwrap_or(self.zone);
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

            // Process through replacement effects with decision maker
            let result = process_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                requested_zone,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => {
                    if orders_library {
                        apply_library_placement_order(game, &ordered_library_results, self.to_top);
                    }
                    return Ok(EffectOutcome::prevented());
                }
                EventOutcome::Proceed(final_zone) => {
                    if final_zone == Zone::Battlefield {
                        let options = match self.battlefield_controller {
                            BattlefieldController::Preserve => {
                                BattlefieldEntryOptions::preserve(self.enters_tapped)
                            }
                            BattlefieldController::Owner => {
                                BattlefieldEntryOptions::owner(self.enters_tapped)
                            }
                            BattlefieldController::You => BattlefieldEntryOptions::specific(
                                ctx.controller,
                                self.enters_tapped,
                            ),
                        };
                        let initial_counters = resolve_battlefield_entry_counters(
                            game,
                            ctx,
                            object_id,
                            &self.enters_with_counters,
                        )?;
                        let options = options
                            .with_initial_counters(initial_counters)
                            .transformed(self.enters_transformed);
                        if self.enters_face_down
                            && let Some(card) = game.object_mut(object_id)
                        {
                            card.apply_face_down_cast_overlay();
                        }
                        battlefield_entries.push((
                            object_id,
                            options,
                            target_lki_before_move,
                            source_lki_before_move,
                        ));
                        continue;
                    }

                    let mut result =
                        finalize_zone_change_move(game, object_id, final_zone, ctx.cause.clone());
                    if !result.new_object_ids.is_empty() {
                        ctx.refresh_target_snapshot(target_lki_before_move.clone());
                        affected_memory
                            .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                        if let Some(snapshot) = source_lki_before_move.clone() {
                            moved_source_lki = Some(snapshot);
                        }
                    }
                    if !result.new_object_ids.is_empty() {
                        for &new_id in &result.new_object_ids {
                            if final_zone == Zone::Exile {
                                game.add_exiled_with_source_link(ctx.source, new_id);
                                if let Some(object) = game.object(new_id) {
                                    ctx.tag_object(
                                        SOURCE_EXILED_TAG,
                                        ObjectSnapshot::from_object(object, game),
                                    );
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
                        continue;
                    }

                    continue;
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                    if let Some(result) = take_recorded_zone_change(game, object_id) {
                        if orders_library && result.final_zone == Zone::Library {
                            ordered_library_results.extend(result.new_object_ids.iter().copied());
                        }
                        affected_ids.extend(result.new_object_ids);
                    } else if let Some(result_id) = game.find_object_by_stable_id(stable_id) {
                        if orders_library
                            && game
                                .object(result_id)
                                .is_some_and(|object| object.zone == Zone::Library)
                        {
                            ordered_library_results.push(result_id);
                        }
                        affected_ids.push(result_id);
                    }
                    affected_memory
                        .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                }
                EventOutcome::NotApplicable => continue,
            }
        }

        if !battlefield_entries.is_empty() {
            let entry_outcomes = move_to_battlefield_batch_with_options(
                game,
                ctx,
                battlefield_entries
                    .iter()
                    .map(|(object, options, _, _)| (*object, options.clone()))
                    .collect(),
            );
            for ((object_id, _, target_lki_before_move, source_lki_before_move), outcome) in
                battlefield_entries.into_iter().zip(entry_outcomes)
            {
                match outcome {
                    BattlefieldEntryOutcome::Moved(new_id) => {
                        if self.enters_attacking {
                            let target = if let Some(attack_player) = configured_attack_player {
                                let targets = attack_targets_for_player(game, attack_player);
                                choose_attack_target_for_player(game, ctx, attack_player, &targets)
                            } else {
                                choose_enters_attacking_target(game, ctx, new_id)
                            };
                            if let Some(target) = target
                                && let Some(combat) = game.combat.as_mut()
                            {
                                combat.attackers.push(AttackerInfo {
                                    creature: new_id,
                                    target,
                                });
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
                    BattlefieldEntryOutcome::Prevented => {
                        if self.enters_face_down
                            && let Some(card) = game.object_mut(object_id)
                        {
                            card.end_face_down_cast_overlay();
                        }
                        any_prevented = true;
                    }
                }
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
        if orders_library {
            apply_library_placement_order(game, &ordered_library_results, self.to_top);
        }

        if !moved_ids.is_empty() {
            let mut outcome =
                EffectOutcome::with_objects(moved_ids).with_affected_objects(affected_ids);
            if !affected_memory.is_empty() {
                outcome = outcome.with_affected_object_memory(affected_memory);
            }
            return Ok(outcome);
        }
        if any_prevented {
            return Ok(EffectOutcome::prevented());
        }
        if any_replaced {
            let mut outcome = EffectOutcome::replaced().with_affected_objects(affected_ids);
            if !affected_memory.is_empty() {
                outcome = outcome.with_affected_object_memory(affected_memory);
            }
            return Ok(outcome);
        }
        Ok(EffectOutcome::target_invalid())
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
    use crate::object::Object;
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
                .last()
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
}

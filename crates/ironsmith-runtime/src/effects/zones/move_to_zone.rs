//! Move to zone effect implementation.

use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_tagged_object_id};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{EventOutcome, process_zone_change_with_additional_effects};
use crate::filter::FilterContext;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

use super::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, finalize_zone_change_move,
    maybe_prompt_for_split_result_order, move_to_battlefield_with_options,
    take_recorded_zone_change,
};
pub use ironsmith_core::BattlefieldController;
pub type MoveToZoneEffect = ironsmith_core::MoveToZoneEffect;

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
    let candidate_ids: Vec<_> = match filter.zone {
        Some(Zone::Hand) => game
            .players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .collect(),
        Some(Zone::Graveyard) => game
            .players
            .iter()
            .flat_map(|player| player.graveyard.iter().copied())
            .collect(),
        Some(Zone::Battlefield) => game.battlefield.clone(),
        Some(Zone::Library) => game
            .players
            .iter()
            .flat_map(|player| player.library.iter().copied())
            .collect(),
        Some(Zone::Stack) => game.stack.iter().map(|entry| entry.object_id).collect(),
        Some(Zone::Exile) => game.exile.clone(),
        Some(Zone::Command) => game.command_zone.clone(),
        None => Vec::new(),
    };

    candidate_ids
        .into_iter()
        .filter(|id| {
            game.object(*id)
                .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
        })
        .count()
}

impl EffectExecutor for MoveToZoneEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
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
            return Ok(EffectOutcome::target_invalid());
        }

        let mut moved_ids = Vec::new();
        let mut affected_ids = Vec::new();
        let mut affected_memory = Vec::new();
        let mut any_prevented = false;
        let mut any_replaced = false;
        let mut moved_source_lki = None;

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let stable_id = obj.stable_id;
            let from_zone = obj.zone;
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
                self.zone,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match result {
                EventOutcome::Prevented => {
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
                        match move_to_battlefield_with_options(game, ctx, object_id, options) {
                            BattlefieldEntryOutcome::Moved(new_id) => {
                                ctx.refresh_target_snapshot(target_lki_before_move.clone());
                                affected_memory.push(OutcomeObjectMemory::from_snapshot(
                                    &target_lki_before_move,
                                ));
                                if let Some(snapshot) = source_lki_before_move.clone() {
                                    moved_source_lki = Some(snapshot);
                                }
                                moved_ids.push(new_id);
                            }
                            BattlefieldEntryOutcome::Prevented => {
                                any_prevented = true;
                            }
                        }
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
                            }
                            if final_zone == Zone::Library && !self.to_top {
                                if let Some(obj) = game.object(new_id) {
                                    if let Some(player) = game.player_mut(obj.owner) {
                                        if let Some(pos) =
                                            player.library.iter().position(|id| *id == new_id)
                                        {
                                            player.library.remove(pos);
                                            player.library.insert(0, new_id);
                                        }
                                    }
                                }
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
                        moved_ids.extend(result.new_object_ids.iter().copied());
                        continue;
                    }

                    continue;
                }
                EventOutcome::Replaced => {
                    any_replaced = true;
                    if let Some(result) = take_recorded_zone_change(game, object_id) {
                        affected_ids.extend(result.new_object_ids);
                    } else if let Some(result_id) = game.find_object_by_stable_id(stable_id) {
                        affected_ids.push(result_id);
                    }
                    affected_memory
                        .push(OutcomeObjectMemory::from_snapshot(&target_lki_before_move));
                }
                EventOutcome::NotApplicable => continue,
            }
        }

        if moves_source && let Some(new_source_id) = moved_ids.first().copied() {
            ctx.source = new_source_id;
        }
        if let Some(snapshot) = moved_source_lki {
            ctx.refresh_source_snapshot(snapshot);
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
        Some(&self.target)
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

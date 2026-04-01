//! Shuffle specific objects into a library, then shuffle that library.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter};
use crate::event_processor::{EventOutcome, process_zone_change_with_additional_effects};
use crate::events::ShuffleLibraryEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::{finalize_zone_change_move, maybe_prompt_for_split_result_order};

#[derive(Debug, Clone, PartialEq)]
pub struct ShuffleObjectsIntoLibraryEffect {
    pub target: ChooseSpec,
    pub player: PlayerFilter,
}

impl ShuffleObjectsIntoLibraryEffect {
    pub fn new(target: ChooseSpec, player: PlayerFilter) -> Self {
        Self { target, player }
    }

    fn expected_zone_for_object(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
        object_id: crate::ids::ObjectId,
    ) -> Option<Zone> {
        match self.target.base() {
            ChooseSpec::Object(filter) => filter.zone,
            ChooseSpec::Tagged(tag) => ctx
                .get_tagged_all(tag)
                .and_then(|snapshots| snapshots.iter().find(|s| s.object_id == object_id))
                .map(|snapshot| snapshot.zone),
            ChooseSpec::Source => game.object(ctx.source).map(|obj| obj.zone),
            _ => None,
        }
    }
}

impl EffectExecutor for ShuffleObjectsIntoLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let object_ids = match resolve_objects_for_effect(game, ctx, &self.target) {
            Ok(ids) => ids,
            Err(ExecutionError::InvalidTarget) => Vec::new(),
            Err(err) => return Err(err),
        };

        let mut moved_ids = Vec::new();
        let additional_effects = ctx.additional_replacement_effects_snapshot();

        for object_id in object_ids {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            if let Some(expected_zone) = self.expected_zone_for_object(game, ctx, object_id)
                && obj.zone != expected_zone
            {
                continue;
            }

            let from_zone = obj.zone;
            match process_zone_change_with_additional_effects(
                game,
                object_id,
                from_zone,
                Zone::Library,
                ctx.cause.clone(),
                &mut *ctx.decision_maker,
                &additional_effects,
            ) {
                EventOutcome::Proceed(final_zone) => {
                    if final_zone != Zone::Library {
                        continue;
                    }
                    let mut result =
                        finalize_zone_change_move(game, object_id, final_zone, ctx.cause.clone());
                    if !result.new_object_ids.is_empty() {
                        for &new_id in &result.new_object_ids {
                            if let Some(moved) = game.object(new_id) {
                                if let Some(player) = game.player_mut(moved.owner) {
                                    if let Some(pos) =
                                        player.library.iter().position(|id| *id == new_id)
                                    {
                                        player.library.remove(pos);
                                        player.library.insert(0, new_id);
                                    }
                                }
                            }
                        }
                        if from_zone == Zone::Battlefield {
                            maybe_prompt_for_split_result_order(
                                game,
                                &mut *ctx.decision_maker,
                                final_zone,
                                &ctx.cause,
                                &mut result,
                            );
                            game.record_zone_change_results(
                                object_id,
                                result.new_object_ids.clone(),
                            );
                        }
                        moved_ids.extend(result.new_object_ids.iter().copied());
                    }
                }
                EventOutcome::Prevented | EventOutcome::Replaced | EventOutcome::NotApplicable => {}
            }
        }

        game.shuffle_player_library(player_id);
        let shuffle_event = TriggerEvent::new_with_provenance(
            ShuffleLibraryEvent::new(player_id, ctx.cause.clone()),
            ctx.provenance,
        );

        if moved_ids.is_empty() {
            Ok(EffectOutcome::resolved().with_event(shuffle_event))
        } else {
            Ok(EffectOutcome::with_objects(moved_ids).with_event(shuffle_event))
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "objects to shuffle"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::ChoiceCount;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::target::{ObjectFilter, PlayerFilter};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_card_in_zone(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![crate::types::CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn shuffle_objects_into_library_still_shuffles_with_zero_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_card_in_zone(&mut game, alice, Zone::Library, "A");
        create_card_in_zone(&mut game, alice, Zone::Library, "B");
        let before = game.irreversible_random_count();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let spec = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        ))
        .with_count(ChoiceCount::up_to(2));
        let effect = ShuffleObjectsIntoLibraryEffect::new(spec, PlayerFilter::You);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("shuffle should resolve");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "zero-target shuffle-into-library effects should still shuffle"
        );
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.downcast::<ShuffleLibraryEvent>().is_some()),
            "zero-target shuffle-into-library effects should still emit a shuffle event"
        );
    }

    #[test]
    fn shuffle_objects_into_library_still_shuffles_if_object_left_expected_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let graveyard_card = create_card_in_zone(&mut game, alice, Zone::Graveyard, "Target");
        create_card_in_zone(&mut game, alice, Zone::Library, "A");
        create_card_in_zone(&mut game, alice, Zone::Library, "B");
        let before = game.irreversible_random_count();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let spec = ChooseSpec::Object(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
        let effect = ShuffleObjectsIntoLibraryEffect::new(spec, PlayerFilter::You);

        let moved = game
            .move_object_by_effect(graveyard_card, Zone::Exile)
            .expect("card should move to exile");
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("shuffle should resolve");

        assert_eq!(
            game.irreversible_random_count(),
            before + 1,
            "library should still shuffle when the object left the expected zone"
        );
        assert!(
            outcome.events.iter().any(|event| event
                .downcast::<ShuffleLibraryEvent>()
                .is_some_and(|shuffle| { shuffle.player == alice })),
            "shuffle-into-library effects should emit a shuffle event even if nothing moves"
        );
        assert_eq!(
            game.object(moved).expect("moved card").zone,
            Zone::Exile,
            "object should remain in its new zone rather than being moved back into library"
        );
    }
}

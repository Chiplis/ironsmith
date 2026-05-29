//! Shuffle specific objects as a pile and put that pile on top of a library.

use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::processing::{EventOutcome, process_zone_change_with_additional_effects};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::{ChooseSpec, ObjectRef, PlayerFilter};
use crate::zone::Zone;
pub use ironsmith_core::ShuffleObjectsOntoLibraryEffect;

use super::finalize_zone_change_move;

fn expected_zone_for_object(
    target: &ChooseSpec,
    game: &GameState,
    ctx: &ExecutionContext,
    object_id: crate::ids::ObjectId,
) -> Option<Zone> {
    match target.base() {
        ChooseSpec::Object(filter) => filter.zone,
        ChooseSpec::Tagged(tag) => ctx
            .get_tagged_all(tag)
            .and_then(|snapshots| snapshots.iter().find(|s| s.object_id == object_id))
            .map(|snapshot| snapshot.zone),
        ChooseSpec::Source => game.object(ctx.source).map(|obj| obj.zone),
        _ => None,
    }
}

impl EffectExecutor for ShuffleObjectsOntoLibraryEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = if matches!(self.target, ChooseSpec::Source)
            && matches!(self.player, PlayerFilter::OwnerOf(ObjectRef::Target))
        {
            game.object(ctx.source)
                .map(|object| object.owner)
                .ok_or(ExecutionError::ObjectNotFound(ctx.source))?
        } else {
            resolve_player_filter(game, &self.player, ctx)?
        };
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
            if let Some(expected_zone) = expected_zone_for_object(&self.target, game, ctx, object_id)
                && obj.zone != expected_zone
            {
                continue;
            }

            let from_zone = obj.zone;
            let pre_snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(obj, game);
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
                    let result = finalize_zone_change_move(game, object_id, final_zone, ctx.cause.clone());
                    if !result.new_object_ids.is_empty() {
                        ctx.refresh_target_snapshot(pre_snapshot.clone());
                        if pre_snapshot.object_id == ctx.source {
                            ctx.refresh_source_snapshot(pre_snapshot);
                        }
                        moved_ids.extend(result.new_object_ids.iter().copied());
                    }
                }
                EventOutcome::Prevented | EventOutcome::Replaced | EventOutcome::NotApplicable => {}
            }
        }

        if moved_ids.len() > 1 {
            game.shuffle_slice(&mut moved_ids);
        }
        if !moved_ids.is_empty()
            && let Some(player) = game.player(player_id)
        {
            let moved_set = moved_ids.iter().copied().collect::<std::collections::HashSet<_>>();
            let mut library = player
                .library
                .iter()
                .copied()
                .filter(|id| !moved_set.contains(id))
                .collect::<Vec<_>>();
            library.extend(moved_ids.iter().copied());
            game.set_player_library_order_with_audit(
                player_id,
                library,
                "shuffled objects onto library",
            );
        }

        if moved_ids.is_empty() {
            Ok(EffectOutcome::resolved())
        } else {
            Ok(EffectOutcome::with_objects(moved_ids.clone()).with_affected_objects(moved_ids))
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "objects to put on top of library"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::target::ObjectFilter;

    fn create_card_in_zone(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name).build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn shuffle_objects_onto_library_moves_only_selected_pile_to_top() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bottom = create_card_in_zone(&mut game, alice, Zone::Library, "Library Bottom");
        let source = game.new_object_id();
        create_card_in_zone(&mut game, alice, Zone::Exile, "Pile One");
        create_card_in_zone(&mut game, alice, Zone::Exile, "Pile Two");
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = ShuffleObjectsOntoLibraryEffect::new(
            ChooseSpec::All(ObjectFilter::default().in_zone(Zone::Exile)),
            PlayerFilter::You,
        );
        let outcome = effect.execute(&mut game, &mut ctx).expect("shuffle pile");

        assert!(outcome.something_happened());
        let library = &game.player(alice).expect("alice").library;
        assert_eq!(library.first().copied(), Some(bottom));
        let top_pile = library[library.len() - 2..]
            .iter()
            .filter_map(|id| game.object(*id).map(|object| object.name.clone()))
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(top_pile, ["Pile One".to_string(), "Pile Two".to_string()].into_iter().collect());
        assert!(game.exile.is_empty());
    }
}

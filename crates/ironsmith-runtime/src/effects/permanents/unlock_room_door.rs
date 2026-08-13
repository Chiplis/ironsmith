use crate::decisions::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter_as_chooser;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

pub use ironsmith_core::UnlockRoomDoorEffect;

impl EffectExecutor for UnlockRoomDoorEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter_as_chooser(game, &self.player, ctx)?;
        let filter_ctx = game.filter_context_for(chooser, Some(ctx.source));
        let candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|object_id| {
                game.object(*object_id).is_some_and(|object| {
                    object.zone == Zone::Battlefield
                        && self.room_filter.matches(object, &filter_ctx, game)
                        && game.room_has_locked_door(*object_id)
                })
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let options = candidates
            .iter()
            .enumerate()
            .map(|(index, object_id)| {
                let name = game
                    .object(*object_id)
                    .map(|object| object.name.to_string())
                    .unwrap_or_else(|| "Room".to_string());
                SelectableOption::new(index, name).with_object(*object_id)
            })
            .collect();
        let choice_ctx = SelectOptionsContext::new(
            chooser,
            Some(ctx.source),
            "Choose a Room with a locked door",
            options,
            1,
            1,
        );
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(room_id) = selected
            .into_iter()
            .next()
            .and_then(|index| candidates.get(index).copied())
        else {
            return Ok(EffectOutcome::count(0));
        };
        if !crate::special_actions::apply_room_door_unlock(game, room_id) {
            return Ok(EffectOutcome::count(0));
        }

        let event = TriggerEvent::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::UnlockDoor, chooser, room_id, 1),
            ctx.provenance,
        );
        Ok(EffectOutcome::with_objects(vec![room_id])
            .with_affected_objects(vec![room_id])
            .with_event(event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::LinkedFaceLayout;
    use crate::cards::builders::CardDefinitionBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::{CardType, Subtype};

    #[test]
    fn resolution_unlocks_one_matching_room_without_paying_its_door_cost() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let front_id = CardId::from_raw(7_401_001);
        let back_id = CardId::from_raw(7_401_002);
        let front = CardDefinitionBuilder::new(front_id, "Locked Room")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Room])
            .other_face(back_id)
            .other_face_name("Other Locked Room")
            .linked_face_layout(LinkedFaceLayout::Split)
            .build();
        let back = CardDefinitionBuilder::new(back_id, "Other Locked Room")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Room])
            .other_face(front_id)
            .other_face_name("Locked Room")
            .linked_face_layout(LinkedFaceLayout::Split)
            .build();
        game.register_linked_face_definition(&back);
        let room_id = game.create_object_from_definition(&front, alice, Zone::Battlefield);
        assert!(game.room_has_locked_door(room_id));

        let mut room_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        room_filter.controller = Some(PlayerFilter::You);
        room_filter.subtypes = vec![Subtype::Room];
        let filter_ctx = game.filter_context_for(alice, Some(room_id));
        assert!(
            room_filter.matches(
                game.object(room_id).expect("Room object should exist"),
                &filter_ctx,
                &game,
            ),
            "Room filter should match its only legal candidate: {:#?}",
            game.object(room_id),
        );
        let effect = UnlockRoomDoorEffect {
            player: PlayerFilter::You,
            room_filter,
        };
        let mut ctx = ExecutionContext::new_default(room_id, alice);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("resolution-time unlock should succeed without a payment prompt");

        assert_eq!(outcome.affected_objects(), Some([room_id].as_slice()));
        assert!(!game.room_has_locked_door(room_id));
        let event = outcome
            .events
            .first()
            .and_then(|event| event.downcast::<KeywordActionEvent>())
            .expect("unlock should emit its keyword-action event");
        assert_eq!(event.action, KeywordActionKind::UnlockDoor);
        assert_eq!(event.player, alice);
        assert_eq!(event.source, room_id);
    }
}

//! Incubate keyword action implementation.

use crate::cards::tokens::incubator_token_definitions;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::object::{CounterType, Object};
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::zone::Zone;

use super::lifecycle::{TokenEntryOptions, apply_token_battlefield_entry};

pub type IncubateEffect = ironsmith_core::IncubateEffect;

impl EffectExecutor for IncubateEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id = resolve_player_filter(game, &self.controller, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;

        let mut created_ids = Vec::with_capacity(count);
        let mut events = Vec::with_capacity(count * 2);
        let entry_options = TokenEntryOptions::default();

        for _ in 0..count {
            let (front, back) = incubator_token_definitions();
            game.register_linked_face_definition(&front);
            game.register_linked_face_definition(&back);

            let id = game.new_object_id();
            let mut token_obj = Object::from_token_definition(id, &front, controller_id);
            token_obj.zone = Zone::Command;
            let token_is_creature = token_obj.is_creature();

            game.add_object(token_obj);

            let initial_counters = if amount > 0 {
                vec![(CounterType::PlusOnePlusOne, amount)]
            } else {
                Vec::new()
            };
            let Some(entry_result) = game
                .move_object_with_etb_processing_with_initial_counters_with_dm(
                    id,
                    Zone::Battlefield,
                    initial_counters,
                    &mut ctx.decision_maker,
                )
            else {
                game.remove_object(id);
                continue;
            };

            let entered_id = entry_result.new_id;
            created_ids.push(entered_id);

            let entered_battlefield = game
                .object(entered_id)
                .is_some_and(|obj| obj.zone == Zone::Battlefield);
            if entered_battlefield {
                let entered_is_creature = game.current_is_creature(entered_id);
                let tracks_creature_etb = entered_is_creature || token_is_creature;
                apply_token_battlefield_entry(
                    game,
                    ctx,
                    entered_id,
                    controller_id,
                    tracks_creature_etb,
                    entry_options,
                    Zone::Command,
                    entry_result.enters_tapped,
                    &mut events,
                )?;
            }

            events.push(TriggerEvent::new_with_provenance(
                KeywordActionEvent::new(
                    KeywordActionKind::Incubate,
                    controller_id,
                    ctx.source,
                    amount,
                ),
                ctx.provenance,
            ));
        }

        Ok(EffectOutcome::with_objects(created_ids.clone())
            .with_events(events)
            .with_affected_objects_from_game(game, created_ids))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.controller_target.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to incubate"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::LinkedFaceLayout;
    use crate::effect::Value;
    use crate::effects::TransformEffect;
    use crate::ids::PlayerId;
    use crate::types::{CardType, Subtype};

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn incubate_creates_incubator_with_counters_and_transform_face() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = IncubateEffect::you(Value::Fixed(3), Value::Fixed(1))
            .execute(&mut game, &mut ctx)
            .expect("incubate should resolve");

        let ids = outcome.objects().expect("incubate should create a token");
        assert_eq!(ids.len(), 1);
        let token_id = ids[0];
        let token = game.object(token_id).expect("incubator should exist");
        assert_eq!(token.name, "Incubator");
        assert!(token.card_types.contains(&CardType::Artifact));
        assert!(token.subtypes.contains(&Subtype::Incubator));
        assert!(!game.current_is_creature(token_id));
        assert_eq!(game.counter_count(token_id, CounterType::PlusOnePlusOne), 3);
        assert_eq!(token.linked_face_layout, LinkedFaceLayout::TransformLike);
        assert_eq!(token.abilities.len(), 1);

        let mut transform_ctx = ExecutionContext::new_default(token_id, alice);
        TransformEffect::source()
            .execute(&mut game, &mut transform_ctx)
            .expect("incubator should transform");

        let transformed = game
            .object(token_id)
            .expect("transformed token should exist");
        assert_eq!(transformed.name, "Phyrexian Token");
        assert!(transformed.card_types.contains(&CardType::Artifact));
        assert!(transformed.card_types.contains(&CardType::Creature));
        assert!(transformed.subtypes.contains(&Subtype::Phyrexian));
        assert_eq!(game.counter_count(token_id, CounterType::PlusOnePlusOne), 3);
        assert_eq!(game.calculated_power(token_id), Some(3));
        assert_eq!(game.calculated_toughness(token_id), Some(3));
    }

    #[test]
    fn incubate_count_creates_multiple_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = IncubateEffect::you(Value::Fixed(2), Value::Fixed(3))
            .execute(&mut game, &mut ctx)
            .expect("incubate should resolve");

        let ids = outcome.objects().expect("incubate should create tokens");
        assert_eq!(ids.len(), 3);
        for &id in ids {
            let token = game.object(id).expect("incubator should exist");
            assert_eq!(token.name, "Incubator");
            assert_eq!(game.controller_of(token), alice);
            assert_eq!(game.counter_count(id, CounterType::PlusOnePlusOne), 2);
        }
    }
}

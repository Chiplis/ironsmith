//! Create token effect implementation.

use crate::cards::CardDefinition;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::object::Object;
use crate::target::ChooseSpec;
use crate::zone::Zone;

use super::lifecycle::{
    TokenCleanupOptions, TokenEntryOptions, apply_token_battlefield_entry, schedule_token_cleanup,
};

/// Effect that creates token creatures or other token permanents.
///
/// # Fields
///
/// * `token` - The token definition (use CardDefinitionBuilder with .token())
/// * `count` - How many tokens to create
/// * `controller` - Who controls the tokens
/// * `suppress_aura_attachment_choice` - Whether Aura token attachment is handled by a later effect
/// * `enters_tapped` - Whether the tokens enter tapped
/// * `enters_attacking` - Whether the tokens enter attacking
/// * `exile_at_end_of_combat` - Whether to exile the tokens at end of combat
/// * `sacrifice_at_end_of_combat` - Whether to sacrifice the tokens at end of combat
/// * `sacrifice_at_next_end_step` - Whether to sacrifice the tokens at the
///   beginning of the next end step.
/// * `exile_at_next_end_step` - Whether to exile the tokens at the beginning
///   of the next end step.
///
/// # Example
///
/// ```ignore
/// // Create two 1/1 white Soldier tokens
/// let soldier = CardDefinitionBuilder::new(CardId::new(), "Soldier")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Soldier])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(1, 1))
///     .build();
/// let effect = CreateTokenEffect::new(soldier, 2, PlayerFilter::You);
///
/// // Create a 4/4 Angel token that enters tapped and attacking, exiled at EOC
/// let angel = CardDefinitionBuilder::new(CardId::new(), "Angel")
///     .token()
///     .card_types(vec![CardType::Creature])
///     .subtypes(vec![Subtype::Angel])
///     .color_indicator(ColorSet::WHITE)
///     .power_toughness(PowerToughness::fixed(4, 4))
///     .flying()
///     .build();
/// let effect = CreateTokenEffect::one(angel)
///     .tapped()
///     .attacking()
///     .exile_at_end_of_combat();
/// ```
pub type CreateTokenEffect = ironsmith_core::CreateTokenEffect<CardDefinition>;

impl EffectExecutor for CreateTokenEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller_id =
            crate::effects::helpers::resolve_player_filter(game, &self.controller, ctx)?;
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        let cleanup_options = TokenCleanupOptions::new(
            self.exile_at_end_of_combat,
            self.sacrifice_at_end_of_combat,
            self.sacrifice_at_next_end_step,
            self.exile_at_next_end_step,
        );
        let entry_options = TokenEntryOptions::new(self.enters_tapped, self.enters_attacking);

        let mut created_ids = Vec::with_capacity(count);
        let mut events = Vec::with_capacity(count);

        for _ in 0..count {
            let id = game.new_object_id();
            let mut token_obj = Object::from_token_definition(id, &self.token, controller_id);
            token_obj.zone = Zone::Command;
            let token_is_creature = token_obj.is_creature();

            game.add_object(token_obj);
            let entry_result = if self.suppress_aura_attachment_choice {
                game.move_object_with_etb_processing_without_aura_attachment_choice(
                    id,
                    Zone::Battlefield,
                    &mut ctx.decision_maker,
                )
            } else {
                game.move_object_with_etb_processing_with_dm(
                    id,
                    Zone::Battlefield,
                    &mut ctx.decision_maker,
                )
            };
            let Some(entry_result) = entry_result else {
                game.remove_object(id);
                continue;
            };
            let entered_id = entry_result.new_id;
            created_ids.push(entered_id);
            let entered_battlefield = game
                .object(entered_id)
                .is_some_and(|obj| obj.zone == Zone::Battlefield);

            if entered_battlefield {
                let effective_tapped = entry_result.enters_tapped || self.enters_tapped;
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
                    effective_tapped,
                    &mut events,
                )?;

                schedule_token_cleanup(game, ctx, entered_id, controller_id, cleanup_options)?;
            }
        }

        Ok(EffectOutcome::with_objects(created_ids).with_events(events))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.controller_target.as_ref()
    }

    fn target_description(&self) -> &'static str {
        "player to create tokens"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::PowerToughness;
    use crate::cards::CardDefinitionBuilder;
    use crate::cards::definitions::tayam_luminous_enigma;
    use crate::color::{Color, ColorSet};
    use crate::ids::{CardId, PlayerId};
    use crate::object::{CounterType, ObjectKind};
    use crate::test_prelude::*;
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn soldier_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Soldier")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Soldier])
            .color_indicator(ColorSet::from(Color::White))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn goblin_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Goblin")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Goblin])
            .color_indicator(ColorSet::from(Color::Red))
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    fn zombie_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Zombie")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Zombie])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn beast_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Beast")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Beast])
            .power_toughness(PowerToughness::fixed(3, 3))
            .build()
    }

    fn spirit_token() -> CardDefinition {
        CardDefinitionBuilder::new(CardId::new(), "Spirit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Spirit])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    #[test]
    fn test_create_single_token() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::one(soldier_token());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 1);
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.name, "Soldier");
            assert_eq!(token.kind, ObjectKind::Token);
            assert!(token.is_creature());
            assert_eq!(token.power(), Some(1));
            assert_eq!(token.toughness(), Some(1));
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_multiple_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(goblin_token(), 3);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert_eq!(ids.len(), 3);
            for id in ids {
                let token = game.object(id).unwrap();
                assert_eq!(token.name, "Goblin");
                assert_eq!(token.kind, ObjectKind::Token);
            }
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_zero_tokens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(zombie_token(), 0);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            assert!(ids.is_empty());
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_tracks_creature_etb() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::you(beast_token(), 2);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have tracked 2 creatures entering
        assert_eq!(
            game.turn_store
                .turn_history
                .creatures_entered_under_controller(alice),
            2
        );
    }

    #[test]
    fn test_create_token_for_other_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        // Use Specific instead of Opponent since Opponent requires targeting context
        let effect = CreateTokenEffect::new(spirit_token(), 1, PlayerFilter::Specific(bob));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        if let crate::effect::OutcomeValue::Objects(ids) = result.value {
            let token = game.object(ids[0]).unwrap();
            assert_eq!(token.controller, bob);
            assert_eq!(
                token.owner, bob,
                "the player who creates the token should own it"
            );
        } else {
            panic!("Expected Objects result");
        }
    }

    #[test]
    fn test_create_token_clone_box() {
        let effect = CreateTokenEffect::one(soldier_token());
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("CreateTokenEffect"));
    }

    #[test]
    #[cfg(ironsmith_runtime_parser_tests)]
    fn test_created_creature_token_gets_etb_replacement_counter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let _tayam =
            game.create_object_from_definition(&tayam_luminous_enigma(), alice, Zone::Battlefield);
        game.refresh_continuous_state();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CreateTokenEffect::one(soldier_token());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        let created_id = match result.value {
            crate::effect::OutcomeValue::Objects(ids) => {
                *ids.first().expect("expected created token")
            }
            other => panic!("expected created token object ids, got {other:?}"),
        };

        let token = game.object(created_id).expect("created token should exist");
        assert_eq!(
            token.counters.get(&CounterType::Vigilance).copied(),
            Some(1),
            "token creature should get Tayam's additional vigilance counter on entry"
        );
    }
}

//! Lose the game effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::LoseTheGameEffect;

/// Effect that causes a player to lose the game.
///
/// Checks for effects that prevent losing (e.g., Platinum Angel).
///
/// # Fields
///
/// * `player` - The player who loses the game
///
/// # Example
///
/// ```ignore
/// // Target player loses the game
/// let effect = LoseTheGameEffect::new(PlayerFilter::Opponent);
///
/// // You lose the game (alternate win condition trigger)
/// let effect = LoseTheGameEffect::you();
/// ```
impl EffectExecutor for LoseTheGameEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
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
        let player_id = resolve_player_filter(game, &self.player, ctx)?;

        match crate::events::processing::process_player_loss(
            game,
            player_id,
            &mut *ctx.decision_maker,
        ) {
            crate::events::processing::PlayerLossOutcome::Lost => Ok(EffectOutcome::resolved()),
            crate::events::processing::PlayerLossOutcome::Replaced
            | crate::events::processing::PlayerLossOutcome::Prevented => {
                Ok(EffectOutcome::prevented())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::events::other::WouldLoseGameMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    struct ChooseReplacement(usize);

    impl DecisionMaker for ChooseReplacement {
        fn decide_options(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            vec![self.0]
        }
    }

    fn setup_game() -> (GameState, PlayerId, PlayerId) {
        (
            GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20),
            PlayerId::from_index(0),
            PlayerId::from_index(1),
        )
    }

    fn source_permanent(game: &mut GameState, controller: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn register_loss_replacement(
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
        effects: Vec<Effect>,
    ) -> ReplacementEffect {
        let replacement = ReplacementEffect::with_matcher(
            source,
            controller,
            WouldLoseGameMatcher,
            ReplacementAction::Instead(effects),
        );
        game.effect_store
            .replacement_effects
            .add_resolution_effect(replacement.clone());
        replacement
    }

    #[test]
    fn ordinary_lose_game_effect_commits_loss_and_emits_event() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Loss Source");
        let mut dm = ChooseReplacement(0);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        LoseTheGameEffect::new(PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("loss resolves");

        assert!(!game.player(alice).expect("alice").is_in_game());
        assert_eq!(
            game.take_pending_trigger_events()
                .iter()
                .filter(|event| event
                    .downcast::<crate::events::PlayerLosesGameEvent>()
                    .is_some())
                .count(),
            1
        );
    }

    #[test]
    fn lose_game_replacement_executes_instead_of_committing_loss() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Mirror");
        register_loss_replacement(&mut game, source, alice, vec![Effect::set_life_total(7)]);
        game.player_mut(alice).expect("alice").life = 0;

        let mut dm = ChooseReplacement(0);
        assert_eq!(
            crate::events::processing::process_player_loss(&mut game, alice, &mut dm),
            crate::events::processing::PlayerLossOutcome::Replaced
        );
        assert!(game.player(alice).expect("alice").is_in_game());
        assert_eq!(game.player(alice).expect("alice").life, 7);
    }

    #[test]
    fn affected_player_chooses_between_loss_replacements() {
        struct ChooseSecondFor(PlayerId);

        impl DecisionMaker for ChooseSecondFor {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                assert_eq!(ctx.player, self.0);
                vec![1]
            }
        }

        let (mut game, alice, _) = setup_game();
        let first = source_permanent(&mut game, alice, "First Mirror");
        let second = source_permanent(&mut game, alice, "Second Mirror");
        register_loss_replacement(&mut game, first, alice, vec![Effect::set_life_total(3)]);
        register_loss_replacement(&mut game, second, alice, vec![Effect::set_life_total(9)]);
        game.player_mut(alice).expect("alice").life = 0;

        let mut dm = ChooseSecondFor(alice);
        crate::events::processing::process_player_loss(&mut game, alice, &mut dm);

        assert_eq!(game.player(alice).expect("alice").life, 9);
        assert!(game.player(alice).expect("alice").is_in_game());
    }

    #[test]
    fn declining_optional_loss_replacement_allows_original_loss() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Optional Mirror");
        let replacement = ReplacementEffect::with_matcher(
            source,
            alice,
            WouldLoseGameMatcher,
            ReplacementAction::Instead(vec![Effect::set_life_total(5)]),
        )
        .optional();
        let decline = replacement
            .optional_decline_effect()
            .expect("optional replacement has decline choice");
        game.effect_store
            .replacement_effects
            .add_resolution_effect(replacement);
        game.effect_store
            .replacement_effects
            .add_resolution_effect(decline);

        let mut dm = ChooseReplacement(1);
        assert_eq!(
            crate::events::processing::process_player_loss(&mut game, alice, &mut dm),
            crate::events::processing::PlayerLossOutcome::Lost
        );
        assert!(!game.player(alice).expect("alice").is_in_game());
    }

    #[test]
    fn empty_replacement_sequence_removes_loss_event() {
        let (mut game, alice, _) = setup_game();
        let source = source_permanent(&mut game, alice, "Null Mirror");
        register_loss_replacement(&mut game, source, alice, Vec::new());

        let mut dm = ChooseReplacement(0);
        assert_eq!(
            crate::events::processing::process_player_loss(&mut game, alice, &mut dm),
            crate::events::processing::PlayerLossOutcome::Replaced
        );
        assert!(game.player(alice).expect("alice").is_in_game());
        assert!(game.take_pending_trigger_events().is_empty());
    }

    #[test]
    fn replacement_source_lki_survives_source_exile() {
        let (mut game, alice, _) = setup_game();
        let card = CardBuilder::new(CardId::new(), "LKI Angel")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(4, 4))
            .build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        register_loss_replacement(
            &mut game,
            source,
            alice,
            vec![
                Effect::exile(ChooseSpec::Source),
                Effect::gain_life(Value::PowerOf(Box::new(ChooseSpec::Source))),
            ],
        );
        game.player_mut(alice).expect("alice").life = 0;

        let mut dm = ChooseReplacement(0);
        crate::events::processing::process_player_loss(&mut game, alice, &mut dm);

        assert_eq!(game.player(alice).expect("alice").life, 4);
        assert!(game.objects_in_zone(Zone::Exile).iter().any(|object_id| {
            game.object(*object_id)
                .is_some_and(|object| object.name == "LKI Angel")
        }));
        assert!(game.player(alice).expect("alice").is_in_game());
    }
}

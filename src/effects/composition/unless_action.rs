//! "Unless [player does action]" effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

fn execute_effect_sequence(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<EffectOutcome, ExecutionError> {
    let mut outcomes = Vec::new();
    for effect in effects {
        outcomes.push(execute_effect(game, effect, ctx)?);
    }
    Ok(EffectOutcome::aggregate(outcomes))
}

/// Effect that executes main effects unless a player performs an alternative action.
///
/// "Sacrifice this creature unless you sacrifice another creature" — the player
/// can choose to perform the alternative action to prevent the main effects.
///
/// # Fields
///
/// * `effects` - The effects to execute if the player does NOT perform the alternative
/// * `alternative` - The alternative action the player can choose to perform
/// * `player` - Which player chooses whether to perform the alternative
///
/// # Result
///
/// - If player performs an alternative action that actually happens: result of alternative effects
/// - If player declines OR chosen alternative does not happen: result of executing main effects
#[derive(Debug, Clone, PartialEq)]
pub struct UnlessActionEffect {
    /// The effects to execute if the player does not perform the alternative.
    pub effects: Vec<Effect>,
    /// The alternative action to prevent the main effects.
    pub alternative: Vec<Effect>,
    /// Which player chooses.
    pub player: PlayerFilter,
}

impl UnlessActionEffect {
    /// Create a new "unless action" effect.
    pub fn new(effects: Vec<Effect>, alternative: Vec<Effect>, player: PlayerFilter) -> Self {
        Self {
            effects,
            alternative,
            player,
        }
    }
}

impl EffectExecutor for UnlessActionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let deciding_player = resolve_player_filter(game, &self.player, ctx)?;

        // Ask the player if they want to perform the alternative action
        let wants_alternative = make_boolean_decision(
            game,
            &mut ctx.decision_maker,
            deciding_player,
            ctx.source,
            "Perform alternative action to prevent effect?".to_string(),
            FallbackStrategy::Accept,
        );

        if wants_alternative {
            // Only prevent the main effects if the alternative action actually happens.
            let mut alternative_outcome = execute_effect_sequence(game, ctx, &self.alternative)?;
            if alternative_outcome.something_happened() {
                return Ok(alternative_outcome);
            }

            let mut main_outcome = execute_effect_sequence(game, ctx, &self.effects)?;
            main_outcome.events.append(&mut alternative_outcome.events);
            Ok(main_outcome)
        } else {
            execute_effect_sequence(game, ctx, &self.effects)
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        self.effects
            .iter()
            .find_map(|effect| effect.0.get_target_spec())
    }

    fn target_description(&self) -> &'static str {
        for effect in &self.effects {
            if effect.0.get_target_spec().is_some() {
                return effect.0.target_description();
            }
        }
        "target"
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        for effect in &self.effects {
            if effect.0.get_target_spec().is_some() {
                return effect.0.get_target_count();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::EffectResult;
    use crate::effects::{ChooseObjectsEffect, SacrificeEffect};
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[derive(Default)]
    struct AcceptBooleanDecisionMaker;

    impl DecisionMaker for AcceptBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            true
        }
    }

    fn sacrifice_creature_alternative(player: PlayerFilter, chooser: PlayerFilter) -> Vec<Effect> {
        vec![
            Effect::new(ChooseObjectsEffect::new(
                ObjectFilter::creature().controlled_by(player.clone()),
                1,
                chooser,
                "sacrificed",
            )),
            Effect::new(SacrificeEffect::player(
                ObjectFilter::tagged("sacrificed"),
                1,
                player,
            )),
        ]
    }

    #[test]
    fn test_unless_action_falls_back_to_main_when_alternative_noops() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = AcceptBooleanDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let initial_life = game.player(alice).expect("alice").life;
        let effect = UnlessActionEffect::new(
            vec![Effect::gain_life(3)],
            sacrifice_creature_alternative(PlayerFilter::You, PlayerFilter::You),
            PlayerFilter::You,
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless resolves");

        // Alternative was chosen, but no creature existed to sacrifice, so main effect applies.
        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(alice).expect("alice").life, initial_life + 3);
    }

    #[test]
    fn test_unless_action_prevents_main_when_alternative_happens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, "Test Creature", alice);
        let source = game.new_object_id();
        let mut dm = AcceptBooleanDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let initial_life = game.player(alice).expect("alice").life;
        let effect = UnlessActionEffect::new(
            vec![Effect::gain_life(3)],
            sacrifice_creature_alternative(PlayerFilter::You, PlayerFilter::You),
            PlayerFilter::You,
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless resolves");

        assert!(result.something_happened());
        assert_eq!(game.player(alice).expect("alice").life, initial_life);
        assert!(!game.battlefield.contains(&creature));
    }

    #[test]
    fn test_unless_action_uses_main_when_declined() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let initial_life = game.player(alice).expect("alice").life;
        let effect = UnlessActionEffect::new(
            vec![Effect::gain_life(2)],
            vec![Effect::gain_life(100)],
            PlayerFilter::You,
        );

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("unless resolves");

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).expect("alice").life, initial_life + 2);
    }

    #[test]
    fn unless_action_forwards_main_target_spec() {
        let effect = UnlessActionEffect::new(
            vec![Effect::counter(ChooseSpec::target_spell())],
            vec![Effect::gain_life(1)],
            PlayerFilter::You,
        );

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }
}

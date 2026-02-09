//! May effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::ask_may_choice;
use crate::effect::{Effect, EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that offers an optional choice to the player.
///
/// "You may X" - the player can choose whether to execute the effects.
///
/// # Fields
///
/// * `effects` - The optional effects to execute if accepted
/// * `fallback` - Strategy when no decision maker is present (default: Decline)
///
/// # Result
///
/// - If player declines: `EffectResult::Declined`
/// - If player accepts: the result of the last inner effect (or Count(0) if no effects)
///
/// # Example
///
/// ```ignore
/// // "You may draw a card"
/// let effect = MayEffect::new(vec![Effect::draw(1)]);
///
/// // "You may sacrifice a creature" - composed with ChooseObjectsEffect
/// let effect = MayEffect::new(vec![
///     Effect::choose_objects(ObjectFilter::creature().you_control(), 1, PlayerFilter::You, "sac"),
///     Effect::sacrifice(ChooseSpec::tagged("sac")),
/// ]);
///
/// // With auto-accept fallback
/// let effect = MayEffect::new(vec![Effect::draw(1)]).with_fallback(FallbackStrategy::Accept);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct MayEffect {
    /// The optional effects to execute.
    pub effects: Vec<Effect>,
    /// Optional explicit decider for "that player may ..." patterns.
    pub decider: Option<PlayerFilter>,
    /// Strategy when no decision maker is present.
    pub fallback: FallbackStrategy,
}

impl MayEffect {
    /// Create a new May effect with default Decline fallback.
    pub fn new(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            decider: None,
            fallback: FallbackStrategy::Decline,
        }
    }

    /// Create a new May effect where a specific player decides.
    pub fn new_for_player(effects: Vec<Effect>, decider: PlayerFilter) -> Self {
        Self {
            effects,
            decider: Some(decider),
            fallback: FallbackStrategy::Decline,
        }
    }

    /// Create a new May effect from a single effect (convenience).
    pub fn single(effect: Effect) -> Self {
        Self::new(vec![effect])
    }

    /// Set the fallback strategy for when no decision maker is present.
    pub fn with_fallback(mut self, fallback: FallbackStrategy) -> Self {
        self.fallback = fallback;
        self
    }
}

impl EffectExecutor for MayEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        // Generate a description from the effects
        let description = format!("{:?}", self.effects);

        // Use explicit decider when present ("that player may ..."), otherwise
        // preserve legacy behavior: iterated player if set, then controller.
        let deciding_player = if let Some(decider) = &self.decider {
            resolve_player_filter(game, decider, ctx)?
        } else {
            ctx.iterated_player.unwrap_or(ctx.controller)
        };

        let should_do = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            deciding_player,
            ctx.source,
            description,
            self.fallback,
        );

        if should_do {
            // Execute all effects and aggregate outcomes
            let mut outcomes = Vec::new();
            for effect in &self.effects {
                outcomes.push(execute_effect(game, effect, ctx)?);
            }
            Ok(EffectOutcome::aggregate(outcomes))
        } else {
            Ok(EffectOutcome::from_result(EffectResult::Declined))
        }
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    use crate::target::PlayerFilter;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    #[test]
    fn test_may_auto_decline_without_decision_maker() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        // Use AutoPassDecisionMaker which declines boolean choices
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let initial_life = game.player(alice).unwrap().life;

        let effect = MayEffect::new(vec![Effect::gain_life(5)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Declined);
        // Life should not have changed
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_may_clone_box() {
        let effect = MayEffect::new(vec![Effect::gain_life(1)]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("MayEffect"));
    }

    #[test]
    fn test_may_with_multiple_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        // Use AutoPassDecisionMaker which declines boolean choices
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        // Create a May with multiple effects
        let effect = MayEffect::new(vec![Effect::gain_life(2), Effect::lose_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // With AutoPassDecisionMaker, should decline
        assert_eq!(result.result, EffectResult::Declined);
    }

    #[test]
    fn test_may_single_convenience() {
        let effect = MayEffect::single(Effect::gain_life(1));
        assert_eq!(effect.effects.len(), 1);
    }

    #[test]
    fn test_may_for_specific_player_constructor() {
        let effect =
            MayEffect::new_for_player(vec![Effect::draw(1)], PlayerFilter::target_player());
        assert!(matches!(effect.decider, Some(PlayerFilter::Target(_))));
    }
}

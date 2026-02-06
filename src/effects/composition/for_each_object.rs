//! ForEach effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ObjectFilter;

/// Effect that applies effects once for each object matching a filter.
///
/// # Fields
///
/// * `filter` - Filter for which objects to iterate over
/// * `effects` - Effects to execute for each matching object
///
/// # Example
///
/// ```ignore
/// // For each creature you control, gain 1 life
/// let effect = ForEachEffect::new(
///     ObjectFilter::creature().you_control(),
///     vec![Effect::gain_life(1)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ForEachObject {
    /// Filter for which objects to iterate over.
    pub filter: ObjectFilter,
    /// Effects to execute for each matching object.
    pub effects: Vec<Effect>,
}

impl ForEachObject {
    /// Create a new ForEach effect.
    pub fn new(filter: ObjectFilter, effects: Vec<Effect>) -> Self {
        Self { filter, effects }
    }
}

impl EffectExecutor for ForEachObject {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);

        // Find all objects matching the filter
        let matching: Vec<_> = game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(_, obj)| self.filter.matches(obj, &filter_ctx, game))
            .map(|(id, _)| id)
            .collect();

        let mut outcomes = Vec::new();

        // Execute the effects once for each matching object
        // Note: Effects aren't targeted at specific objects, just executed N times
        for _ in &matching {
            for effect in &self.effects {
                outcomes.push(execute_effect(game, effect, ctx)?);
            }
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) {
        let card = CardBuilder::new(CardId::new(), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let id = game.new_object_id();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
    }

    #[test]
    fn test_for_each_no_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        // No creatures on battlefield
        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // Empty aggregate returns Resolved (no effects executed)
        assert_eq!(result.result, EffectResult::Resolved);
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_for_each_multiple_matches() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Create 3 creatures
        create_creature(&mut game, "Bear 1", alice);
        create_creature(&mut game, "Bear 2", alice);
        create_creature(&mut game, "Bear 3", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        // Gained 1 life for each creature (3 total)
        assert_eq!(game.player(alice).unwrap().life, initial_life + 3);
    }

    #[test]
    fn test_for_each_filtered() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        // Create 2 creatures for Alice, 1 for Bob
        create_creature(&mut game, "Alice Bear 1", alice);
        create_creature(&mut game, "Alice Bear 2", alice);
        create_creature(&mut game, "Bob Bear", bob);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        // Only count creatures Alice controls
        let effect = ForEachObject::new(
            ObjectFilter::creature().you_control(),
            vec![Effect::gain_life(1)],
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(alice).unwrap().life, initial_life + 2);
    }

    #[test]
    fn test_for_each_clone_box() {
        let effect = ForEachObject::new(ObjectFilter::creature(), vec![Effect::gain_life(1)]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ForEachObject"));
    }
}

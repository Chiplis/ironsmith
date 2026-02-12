//! Conditional effect implementation.

use crate::effect::{Condition, Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ModalSpec};
use crate::executor::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId, StableId};

/// Effect that branches based on game state conditions.
///
/// Unlike `If` which checks the result of a prior effect, `Conditional`
/// evaluates game state conditions like "if you control a creature" or
/// "if your life total is 10 or less".
///
/// # Fields
///
/// * `condition` - The game state condition to check
/// * `if_true` - Effects to execute if condition is true
/// * `if_false` - Effects to execute if condition is false
///
/// # Example
///
/// ```ignore
/// // If you control a creature, draw a card. Otherwise, gain 2 life.
/// let effect = ConditionalEffect::new(
///     Condition::YouControl(ObjectFilter::creature()),
///     vec![Effect::draw(1)],
///     vec![Effect::gain_life(2)],
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalEffect {
    /// The game state condition to check.
    pub condition: Condition,
    /// Effects to execute if condition is true.
    pub if_true: Vec<Effect>,
    /// Effects to execute if condition is false.
    pub if_false: Vec<Effect>,
}

impl ConditionalEffect {
    /// Create a new Conditional effect.
    pub fn new(condition: Condition, if_true: Vec<Effect>, if_false: Vec<Effect>) -> Self {
        Self {
            condition,
            if_true,
            if_false,
        }
    }

    /// Create a conditional with no else clause.
    pub fn if_only(condition: Condition, if_true: Vec<Effect>) -> Self {
        Self::new(condition, if_true, vec![])
    }
}

impl EffectExecutor for ConditionalEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result = evaluate_condition(game, &self.condition, ctx)?;

        let effects_to_execute = if result {
            &self.if_true
        } else {
            &self.if_false
        };

        let mut outcomes = Vec::new();
        for effect in effects_to_execute {
            outcomes.push(execute_effect(game, effect, ctx)?);
        }

        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_modal_spec_with_context(
        &self,
        game: &GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> Option<ModalSpec> {
        // Evaluate the condition at cast time to determine which branch to use
        let condition_result = evaluate_condition_simple(game, &self.condition, controller, source);

        // Search the appropriate branch for modal specs
        let effects_to_search = if condition_result {
            &self.if_true
        } else {
            &self.if_false
        };

        // Recursively search through the effects in this branch
        for effect in effects_to_search {
            // First try the context-aware version
            if let Some(spec) = effect
                .0
                .get_modal_spec_with_context(game, controller, source)
            {
                return Some(spec);
            }
            // Fall back to the simple version
            if let Some(spec) = effect.0.get_modal_spec() {
                return Some(spec);
            }
        }

        None
    }
}

/// Evaluate a condition with minimal context (for cast-time evaluation).
///
/// This simplified version is used during spell casting to evaluate conditions
/// like `YouControlCommander` before targets are chosen. It handles common
/// conditions that don't require targets or other context-dependent information.
fn evaluate_condition_simple(
    game: &GameState,
    condition: &Condition,
    controller: PlayerId,
    source: ObjectId,
) -> bool {
    // Build a simple filter context with opponents
    let opponents: Vec<PlayerId> = game
        .players
        .iter()
        .filter(|p| p.id != controller)
        .map(|p| p.id)
        .collect();
    let filter_ctx =
        crate::filter::FilterContext::new(controller).with_opponents(opponents.clone());

    match condition {
        Condition::YouControl(filter) => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| obj.controller == controller)
            .any(|obj| filter.matches(obj, &filter_ctx, game)),
        Condition::OpponentControls(filter) => game
            .battlefield
            .iter()
            .filter_map(|&id| game.object(id))
            .filter(|obj| opponents.contains(&obj.controller))
            .any(|obj| filter.matches(obj, &filter_ctx, game)),
        Condition::LifeTotalOrLess(threshold) => {
            let life = game.player(controller).map(|p| p.life).unwrap_or(0);
            life <= *threshold
        }
        Condition::LifeTotalOrGreater(threshold) => {
            let life = game.player(controller).map(|p| p.life).unwrap_or(0);
            life >= *threshold
        }
        Condition::CardsInHandOrMore(threshold) => {
            let count = game.player(controller).map(|p| p.hand.len()).unwrap_or(0);
            count >= *threshold as usize
        }
        Condition::YourTurn => game.turn.active_player == controller,
        Condition::CreatureDiedThisTurn => game.creatures_died_this_turn > 0,
        Condition::CastSpellThisTurn => game.spells_cast_this_turn.values().any(|&count| count > 0),
        Condition::AttackedThisTurn => game.players_attacked_this_turn.contains(&controller),
        Condition::NoSpellsWereCastLastTurn => game.spells_cast_last_turn_total == 0,
        Condition::YouControlCommander => {
            // Check if the player controls a commander on the battlefield
            if let Some(player) = game.player(controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    // First check: is the commander ID directly on battlefield?
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && obj.controller == controller
                    {
                        return true;
                    }
                    // Second check: is there an object on battlefield whose stable_id
                    // matches the commander ID? (handles zone transitions)
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && obj.controller == controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return true;
                        }
                    }
                }
            }
            false
        }
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let Some(source_obj) = game.object(source) else {
                return false;
            };
            let spent = if let Some(sym) = symbol {
                source_obj.mana_spent_to_cast.amount(*sym)
            } else {
                source_obj.mana_spent_to_cast.total()
            };
            spent >= *amount
        }
        Condition::TaggedObjectMatches(_, _) => false,
        Condition::Not(inner) => !evaluate_condition_simple(game, inner, controller, source),
        Condition::And(a, b) => {
            evaluate_condition_simple(game, a, controller, source)
                && evaluate_condition_simple(game, b, controller, source)
        }
        Condition::Or(a, b) => {
            evaluate_condition_simple(game, a, controller, source)
                || evaluate_condition_simple(game, b, controller, source)
        }
        // Target-dependent conditions default to false during casting
        Condition::TargetIsTapped | Condition::TargetIsAttacking | Condition::SourceIsTapped => {
            false
        }
    }
}

/// Evaluate a condition.
fn evaluate_condition(
    game: &GameState,
    condition: &Condition,
    ctx: &ExecutionContext,
) -> Result<bool, ExecutionError> {
    match condition {
        Condition::YouControl(filter) => {
            let filter_ctx = ctx.filter_context(game);

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| obj.controller == ctx.controller)
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        }
        Condition::OpponentControls(filter) => {
            let filter_ctx = ctx.filter_context(game);
            let opponents = &filter_ctx.opponents;

            let has_matching = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| opponents.contains(&obj.controller))
                .any(|obj| filter.matches(obj, &filter_ctx, game));

            Ok(has_matching)
        }
        Condition::LifeTotalOrLess(threshold) => {
            let life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            Ok(life <= *threshold)
        }
        Condition::LifeTotalOrGreater(threshold) => {
            let life = game.player(ctx.controller).map(|p| p.life).unwrap_or(0);
            Ok(life >= *threshold)
        }
        Condition::CardsInHandOrMore(threshold) => {
            let count = game
                .player(ctx.controller)
                .map(|p| p.hand.len())
                .unwrap_or(0);
            Ok(count >= *threshold as usize)
        }
        Condition::YourTurn => Ok(game.turn.active_player == ctx.controller),
        Condition::CreatureDiedThisTurn => {
            // Check if any creature died this turn
            Ok(game.creatures_died_this_turn > 0)
        }
        Condition::CastSpellThisTurn => {
            // Check if any spell was cast this turn by anyone
            Ok(game.spells_cast_this_turn.values().any(|&count| count > 0))
        }
        Condition::AttackedThisTurn => Ok(game.players_attacked_this_turn.contains(&ctx.controller)),
        Condition::NoSpellsWereCastLastTurn => Ok(game.spells_cast_last_turn_total == 0),
        Condition::TargetIsTapped => {
            // Check if the target is tapped
            if let Some(crate::executor::ResolvedTarget::Object(id)) = ctx.targets.first() {
                return Ok(game.is_tapped(*id));
            }
            Ok(false)
        }
        Condition::SourceIsTapped => Ok(game.is_tapped(ctx.source)),
        Condition::TargetIsAttacking => {
            // Check if the target is among declared attackers
            // Note: Combat attackers are tracked in game_loop, not game_state directly.
            // For now, check ctx.attacking_creatures if it exists
            if let Some(crate::executor::ResolvedTarget::Object(id)) = ctx.targets.first() {
                // Simplified: check if it's a creature that's tapped (attackers are usually tapped)
                // Full implementation would need access to combat state from game loop
                if let Some(obj) = game.object(*id) {
                    return Ok(obj.is_creature() && game.is_tapped(*id));
                }
            }
            Ok(false)
        }
        Condition::YouControlCommander => {
            // Check if you control a commander on the battlefield
            // This matches the logic in GameState::player_controls_a_commander
            // which checks both direct ID and stable_id (important when commander
            // was cast from command zone and got a new object ID)
            if let Some(player) = game.player(ctx.controller) {
                let commanders = player.get_commanders();
                for &commander_id in commanders {
                    // First check: is the commander ID directly on battlefield?
                    if game.battlefield.contains(&commander_id)
                        && let Some(obj) = game.object(commander_id)
                        && obj.controller == ctx.controller
                    {
                        return Ok(true);
                    }
                    // Second check: is there an object on battlefield whose stable_id
                    // matches the commander ID? (handles zone transitions)
                    for &bf_id in &game.battlefield {
                        if let Some(obj) = game.object(bf_id)
                            && obj.controller == ctx.controller
                            && obj.stable_id == StableId::from(commander_id)
                        {
                            return Ok(true);
                        }
                    }
                }
            }
            Ok(false)
        }
        Condition::TaggedObjectMatches(tag, filter) => {
            let filter_ctx = ctx.filter_context(game);
            let Some(snapshot) = ctx.get_tagged(tag.as_str()) else {
                return Ok(false);
            };
            Ok(filter.matches_snapshot(snapshot, &filter_ctx, game))
        }
        Condition::ManaSpentToCastThisSpellAtLeast { amount, symbol } => {
            let Some(source_obj) = game.object(ctx.source) else {
                return Ok(false);
            };
            let spent = if let Some(sym) = symbol {
                source_obj.mana_spent_to_cast.amount(*sym)
            } else {
                source_obj.mana_spent_to_cast.total()
            };
            Ok(spent >= *amount)
        }
        Condition::Not(inner) => {
            let inner_result = evaluate_condition(game, inner, ctx)?;
            Ok(!inner_result)
        }
        Condition::And(a, b) => {
            let a_result = evaluate_condition(game, a, ctx)?;
            if !a_result {
                return Ok(false);
            }
            evaluate_condition(game, b, ctx)
        }
        Condition::Or(a, b) => {
            let a_result = evaluate_condition(game, a, ctx)?;
            if a_result {
                return Ok(true);
            }
            evaluate_condition(game, b, ctx)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
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
    fn test_conditional_you_control_true() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        create_creature(&mut game, "Bear", alice);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        let effect = ConditionalEffect::new(
            Condition::YouControl(ObjectFilter::creature()),
            vec![Effect::gain_life(5)],
            vec![Effect::gain_life(1)],
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have gained 5 (condition true)
        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }

    #[test]
    fn test_conditional_you_control_false() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // No creatures
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life;

        let effect = ConditionalEffect::new(
            Condition::YouControl(ObjectFilter::creature()),
            vec![Effect::gain_life(5)],
            vec![Effect::gain_life(1)],
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have gained 1 (condition false)
        assert_eq!(game.player(alice).unwrap().life, initial_life + 1);
    }

    #[test]
    fn test_conditional_life_total() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        // Set Alice's life to 5
        if let Some(p) = game.player_mut(alice) {
            p.life = 5;
        }

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            ConditionalEffect::if_only(Condition::LifeTotalOrLess(10), vec![Effect::gain_life(3)]);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should have gained 3 (life <= 10)
        assert_eq!(game.player(alice).unwrap().life, 8);
    }

    #[test]
    fn test_conditional_if_only_false() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let initial_life = game.player(alice).unwrap().life; // 20

        // Life > 10, so condition is false
        let effect =
            ConditionalEffect::if_only(Condition::LifeTotalOrLess(10), vec![Effect::gain_life(3)]);
        effect.execute(&mut game, &mut ctx).unwrap();

        // Should not have gained anything
        assert_eq!(game.player(alice).unwrap().life, initial_life);
    }

    #[test]
    fn test_conditional_clone_box() {
        let effect =
            ConditionalEffect::if_only(Condition::LifeTotalOrLess(10), vec![Effect::gain_life(1)]);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("ConditionalEffect"));
    }

    #[test]
    fn test_conditional_mana_spent_to_cast_this_spell() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::new(), "Adamant Probe")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(4)]]))
            .card_types(vec![CardType::Instant])
            .build();
        let source = game.new_object_id();
        let mut source_obj = Object::from_card(source, &card, alice, Zone::Stack);
        source_obj.mana_spent_to_cast.blue = 3;
        source_obj.mana_spent_to_cast.white = 1;
        game.add_object(source_obj);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let initial_life = game.player(alice).unwrap().life;

        let effect = ConditionalEffect::new(
            Condition::ManaSpentToCastThisSpellAtLeast {
                amount: 3,
                symbol: Some(ManaSymbol::Blue),
            },
            vec![Effect::gain_life(5)],
            vec![Effect::gain_life(1)],
        );
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.player(alice).unwrap().life, initial_life + 5);
    }
}

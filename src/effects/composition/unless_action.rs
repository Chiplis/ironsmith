//! "Unless [player does action]" effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::game_state::GameState;
use crate::ids::PlayerId;
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

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    if game.turn_order.is_empty() {
        return Vec::new();
    }

    let start = game
        .turn_order
        .iter()
        .position(|&player_id| player_id == game.turn.active_player)
        .unwrap_or(0);

    (0..game.turn_order.len())
        .filter_map(|offset| {
            let player_id = game.turn_order[(start + offset) % game.turn_order.len()];
            game.player(player_id)
                .filter(|player| player.is_in_game())
                .map(|_| player_id)
        })
        .collect()
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
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let deciding_players = if matches!(self.player, PlayerFilter::Any) {
            players_in_turn_order(game)
        } else {
            vec![resolve_player_filter(game, &self.player, ctx)?]
        };
        let mut attempted_alternative_events = Vec::new();

        for deciding_player in deciding_players {
            // Ask the player if they want to perform the alternative action.
            let wants_alternative = make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                deciding_player,
                ctx.source,
                "Perform alternative action to prevent effect?".to_string(),
                FallbackStrategy::Accept,
            );

            if !wants_alternative {
                continue;
            }

            // Only prevent the main effects if the alternative action actually happens.
            let mut alternative_outcome = if matches!(self.player, PlayerFilter::Any) {
                ctx.with_temp_targets(vec![ResolvedTarget::Player(deciding_player)], |ctx| {
                    execute_effect_sequence(game, ctx, &self.alternative)
                })?
            } else {
                execute_effect_sequence(game, ctx, &self.alternative)?
            };

            if alternative_outcome.something_happened() {
                return Ok(alternative_outcome);
            }

            attempted_alternative_events.append(&mut alternative_outcome.events);
        }

        let mut main_outcome = execute_effect_sequence(game, ctx, &self.effects)?;
        main_outcome
            .events
            .append(&mut attempted_alternative_events);
        Ok(main_outcome)
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.effects], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::EffectResult;
    use crate::effects::{ChooseObjectsEffect, DestroyEffect, ForEachObject, SacrificeEffect};
    use crate::filter::{ObjectFilter, TaggedOpbjectRelation};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::tag::TagKey;
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, owner: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    fn create_land(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Land])
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

    struct ByPlayerBooleanDecisionMaker {
        responses: HashMap<PlayerId, bool>,
    }

    impl ByPlayerBooleanDecisionMaker {
        fn new(responses: impl IntoIterator<Item = (PlayerId, bool)>) -> Self {
            Self {
                responses: responses.into_iter().collect(),
            }
        }
    }

    impl DecisionMaker for ByPlayerBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.responses.get(&ctx.player).copied().unwrap_or(false)
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

    #[test]
    fn test_unless_action_any_player_declines_then_main_effect_happens() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let land = create_land(&mut game, "Test Land", alice);
        let source = game.new_object_id();
        let mut dm = ByPlayerBooleanDecisionMaker::new([(alice, false), (bob, false)]);
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = ForEachObject::new(
            ObjectFilter::land(),
            vec![Effect::unless_action(
                vec![Effect::new(DestroyEffect::with_spec(ChooseSpec::Object(
                    ObjectFilter::land().match_tagged(
                        TagKey::from("__it__"),
                        TaggedOpbjectRelation::IsTaggedObject,
                    ),
                )))],
                vec![Effect::lose_life_player(1, PlayerFilter::Any)],
                PlayerFilter::Any,
            )],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("unless action resolves");

        assert!(!game.battlefield.contains(&land));
        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 20);
    }

    #[test]
    fn test_unless_action_any_player_can_pay_to_prevent_main_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let land = create_land(&mut game, "Test Land", alice);
        let source = game.new_object_id();
        let mut dm = ByPlayerBooleanDecisionMaker::new([(alice, false), (bob, true)]);
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = ForEachObject::new(
            ObjectFilter::land(),
            vec![Effect::unless_action(
                vec![Effect::new(DestroyEffect::with_spec(ChooseSpec::Object(
                    ObjectFilter::land().match_tagged(
                        TagKey::from("__it__"),
                        TaggedOpbjectRelation::IsTaggedObject,
                    ),
                )))],
                vec![Effect::lose_life_player(1, PlayerFilter::Any)],
                PlayerFilter::Any,
            )],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("unless action resolves");

        assert!(game.battlefield.contains(&land));
        assert_eq!(game.player(alice).expect("alice").life, 20);
        assert_eq!(game.player(bob).expect("bob").life, 19);
    }
}

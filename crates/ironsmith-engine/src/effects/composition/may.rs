//! May effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::ask_may_choice;
use crate::effect::{Effect, EffectOutcome, ExecutionFact};
use crate::effects::helpers::{resolve_player_from_spec, resolve_value};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
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
/// - If player declines: `crate::effect::OutcomeStatus::Declined`
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

    /// What this offer says, phrased to follow "You may ".
    ///
    /// The optional branch was compiled from a sentence of the source's own
    /// card text, so the prompt quotes that sentence back rather than exposing
    /// the compiled structure the engine actually holds.
    fn prompt_description(&self, game: &GameState, ctx: &ExecutionContext) -> String {
        crate::runtime_display::effect_sentences::optional_effect_prompt(
            game,
            ctx.source,
            ctx.source_snapshot.as_ref(),
            ctx.ability_index,
            &self.effects,
        )
    }
}

impl EffectExecutor for MayEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.should_auto_decline_without_prompt(game, ctx)? {
            return Ok(EffectOutcome::declined());
        }

        // Prefer a friendly search prompt over the raw compiled lowering text
        // for same-name library searches like Doubling Chant.
        let description = self
            .effects
            .first()
            .and_then(|effect| {
                let choose = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
                if !choose.is_search {
                    return None;
                }
                let max = choose.count.max.unwrap_or(choose.count.min);
                super::choose_objects_runtime::friendly_same_name_search_prompt(
                    game,
                    ctx,
                    &choose.filter,
                    choose.count.min,
                    max,
                )
            })
            .unwrap_or_else(|| self.prompt_description(game, ctx));

        // Use explicit decider when present ("that player may ..."), otherwise
        // preserve established behavior: iterated player if set, then controller.
        let deciding_player = if let Some(decider) = &self.decider {
            crate::effects::helpers::resolve_player_filter_as_chooser(game, decider, ctx)?
        } else {
            ctx.iteration.iterated_player.unwrap_or(ctx.controller)
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
            Ok(EffectOutcome::aggregate(outcomes).with_execution_fact(ExecutionFact::Accepted))
        } else {
            Ok(EffectOutcome::declined())
        }
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // The yes/no choice is made against the pre-action state in APNAP
        // order; the accepted effects execute during the batched commit.
        if self.should_auto_decline_without_prompt(game, ctx)? {
            return Ok(Box::new(MayProposal {
                effects: Vec::new(),
                iterated_player: ctx.iteration.iterated_player,
            }));
        }
        let description = self.prompt_description(game, ctx);
        let deciding_player = if let Some(decider) = &self.decider {
            crate::effects::helpers::resolve_player_filter_as_chooser(game, decider, ctx)?
        } else {
            ctx.iteration.iterated_player.unwrap_or(ctx.controller)
        };
        let should_do = ask_may_choice(
            game,
            &mut ctx.decision_maker,
            deciding_player,
            ctx.source,
            description,
            self.fallback,
        );
        Ok(Box::new(MayProposal {
            effects: if should_do {
                self.effects.clone()
            } else {
                Vec::new()
            },
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }

    fn decision_related_object_specs(&self) -> Vec<crate::target::ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
    }

    fn target_description(&self) -> &'static str {
        super::target_metadata::first_target_description(&[&self.effects], "target")
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        super::target_metadata::first_target_count(&[&self.effects])
    }
}

/// A player's accepted (or declined) "may" action for a simultaneous
/// each-player instruction: the choice was made at prepare time, the accepted
/// effects run in the batched commit.
#[derive(Debug)]
struct MayProposal {
    effects: Vec<crate::effect::Effect>,
    iterated_player: Option<PlayerId>,
}

impl crate::effects::SimultaneousEffectProposal for MayProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.effects.is_empty() {
            return Ok(EffectOutcome::declined());
        }
        let effects = self.effects;
        ctx.with_temp_iterated_player(self.iterated_player, |ctx| {
            let mut outcomes = Vec::new();
            for effect in &effects {
                outcomes.push(execute_effect(game, effect, ctx)?);
            }
            Ok(EffectOutcome::aggregate(outcomes).with_execution_fact(ExecutionFact::Accepted))
        })
    }
}

impl CostExecutableEffect for MayEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        Ok(())
    }
}

impl MayEffect {
    /// Some parsed patterns compile to "may (if condition, do X)" where the
    /// condition is a strict gate (no else branch). If the gate is false, skip
    /// prompting entirely so UI doesn't offer an option that cannot do anything.
    fn should_auto_decline_without_prompt(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
    ) -> Result<bool, ExecutionError> {
        // "Any player may sacrifice two creatures of their choice" is not an
        // option for a player who controls one: an optional action has to be
        // performed in full, so a leading fixed-count choice the deciding
        // player cannot satisfy withdraws the offer instead of shrinking it.
        if let Some(choose) = self
            .effects
            .first()
            .and_then(|effect| effect.downcast_ref::<crate::effects::ChooseObjectsEffect>())
            && super::choose_objects_runtime::fixed_choice_requirement_is_unmet(choose, game, ctx)?
        {
            return Ok(true);
        }

        if self.effects.len() != 1 {
            return Ok(false);
        }

        if let Some(pay_life) = self.effects[0].downcast_ref::<crate::effects::PayLifeEffect>() {
            let player = resolve_player_from_spec(game, &pay_life.player, ctx)?;
            let amount = resolve_value(game, &pay_life.amount, ctx)?.max(0) as u32;
            return Ok(!game.can_pay_life_with_reason(
                player,
                amount,
                crate::costs::PaymentReason::Effect,
            ));
        }

        let Some(conditional) = self.effects[0].downcast_ref::<crate::effects::ConditionalEffect>()
        else {
            return Ok(false);
        };

        if !conditional.if_false.is_empty() {
            return Ok(false);
        }

        let condition_met = crate::condition_eval::evaluate_condition_resolution(
            game,
            &conditional.condition,
            ctx,
        )?;
        Ok(!condition_met)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::PowerToughness;
    use crate::cards::tokens::lander_token_definition;
    use crate::effect::{Condition, ExecutionFact};
    use crate::effect::{EffectId, EffectPredicate};
    use crate::effects::{ExecutionContext, execute_effect};
    use crate::ids::{CardId, PlayerId};
    use crate::object::CounterType;
    use crate::target::{ChooseSpec, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn source_creature_definition() -> crate::cards::CardDefinition {
        crate::cards::CardDefinitionBuilder::new(CardId::new(), "Terrapact Source")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn terrapact_style_effects(fallback: crate::decision::FallbackStrategy) -> Vec<Effect> {
        vec![
            Effect::with_id(
                0,
                Effect::new(
                    MayEffect::new_for_player(
                        vec![Effect::create_tokens(lander_token_definition(), 2)],
                        PlayerFilter::target_opponent(),
                    )
                    .with_fallback(fallback),
                ),
            ),
            Effect::if_then(
                EffectId(0),
                EffectPredicate::DidNotHappen,
                vec![Effect::put_counters_on_source(
                    CounterType::PlusOnePlusOne,
                    2,
                )],
            ),
        ]
    }

    fn count_lander_tokens(game: &GameState, controller: PlayerId) -> usize {
        game.battlefield
            .iter()
            .filter(|&&id| {
                game.object(id).is_some_and(|obj| {
                    matches!(obj.kind, crate::object::ObjectKind::Token)
                        && obj.name == "Lander"
                        && game.controller_of(obj) == controller
                })
            })
            .count()
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

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
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
        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
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

    #[test]
    fn may_forwards_inner_target_spec() {
        let effect = MayEffect::new(vec![Effect::counter(ChooseSpec::target_spell())]);

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[derive(Default)]
    struct PanicOnBooleanDecisionMaker;

    impl crate::decision::DecisionMaker for PanicOnBooleanDecisionMaker {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            panic!("boolean prompt should be skipped for false guarded condition");
        }
    }

    #[test]
    fn may_skips_prompt_for_single_guarded_conditional_when_false() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut dm = PanicOnBooleanDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let initial_life = game.player(alice).expect("alice should exist").life;

        let guarded = Effect::new(crate::effects::ConditionalEffect::if_only(
            Condition::LifeTotalOrLess(0),
            vec![Effect::gain_life(5)],
        ));
        let effect = MayEffect::new(vec![guarded]);

        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert_eq!(
            game.player(alice).expect("alice should exist").life,
            initial_life
        );
    }

    fn create_battlefield_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
    ) -> crate::ids::ObjectId {
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_definition(&definition, controller, Zone::Battlefield)
    }

    fn sacrifice_two_effects(chooser: PlayerFilter) -> Vec<Effect> {
        let mut chosen = crate::filter::ObjectFilter::creature();
        chosen.controller = Some(chooser.clone());
        let mut sacrificed = crate::filter::ObjectFilter::creature();
        sacrificed
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: crate::TagKey::from("sacrificed_0"),
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        vec![
            Effect::choose_objects(chosen, 2, chooser.clone(), "sacrificed_0"),
            Effect::sacrifice_player(sacrificed, 2, chooser),
        ]
    }

    /// "Any player may sacrifice two creatures of their choice" (Prowling
    /// Pangolin): a player who controls only one creature can't sacrifice two,
    /// so the option is never offered to them.
    #[test]
    fn may_sacrifice_two_is_not_offered_when_only_one_creature_is_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let lone_creature = create_battlefield_creature(&mut game, "Lone Bear", alice);

        let mut dm = PanicOnBooleanDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        let effect = MayEffect::new(sacrifice_two_effects(PlayerFilter::You))
            .with_fallback(FallbackStrategy::Accept);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Declined);
        assert!(
            !result.execution_facts().contains(&ExecutionFact::Accepted),
            "an unperformable optional action must not count as taken"
        );
        assert!(
            game.battlefield.contains(&lone_creature),
            "the lone creature must not be sacrificed"
        );
    }

    #[test]
    fn may_sacrifice_two_is_offered_when_two_creatures_are_available() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let first = create_battlefield_creature(&mut game, "Bear One", alice);
        let second = create_battlefield_creature(&mut game, "Bear Two", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = MayEffect::new(sacrifice_two_effects(PlayerFilter::You))
            .with_fallback(FallbackStrategy::Accept);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert!(result.execution_facts().contains(&ExecutionFact::Accepted));
        assert!(!game.battlefield.contains(&first));
        assert!(!game.battlefield.contains(&second));
    }

    /// The whole card: "any player may sacrifice two creatures of their choice.
    /// If a player does, sacrifice this creature." The controller, holding one
    /// creature, is skipped rather than allowed to half-pay, so the offer
    /// passes on to the next player in turn order.
    #[test]
    fn any_player_may_sacrifice_two_skips_players_who_control_only_one() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let alice_creature = create_battlefield_creature(&mut game, "Alice Bear", alice);
        let bob_first = create_battlefield_creature(&mut game, "Bob Bear One", bob);
        let bob_second = create_battlefield_creature(&mut game, "Bob Bear Two", bob);

        let mut ctx = ExecutionContext::new_default(source, alice);

        let each_player = Effect::new(
            super::super::ForPlayersEffect::new_starting_with_controller(
                PlayerFilter::Any,
                vec![Effect::new(
                    MayEffect::new_for_player(
                        sacrifice_two_effects(PlayerFilter::IteratedPlayer),
                        PlayerFilter::IteratedPlayer,
                    )
                    .with_fallback(FallbackStrategy::Accept),
                )],
            )
            .stop_after_first_happened(),
        );

        execute_effect(&mut game, &each_player, &mut ctx).expect("effect should resolve");

        assert!(
            game.battlefield.contains(&alice_creature),
            "a player controlling one creature can't sacrifice two"
        );
        assert!(!game.battlefield.contains(&bob_first));
        assert!(!game.battlefield.contains(&bob_second));
    }

    #[test]
    fn may_acceptance_emits_execution_fact() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect =
            MayEffect::new(vec![Effect::gain_life(1)]).with_fallback(FallbackStrategy::Accept);
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should execute");

        assert!(result.execution_facts().contains(&ExecutionFact::Accepted));
    }

    #[test]
    fn terrapact_style_choice_creates_landers_when_accepted() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.create_object_from_definition(
            &source_creature_definition(),
            alice,
            Zone::Battlefield,
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![crate::effects::ResolvedTarget::Player(bob)]);

        for effect in terrapact_style_effects(crate::decision::FallbackStrategy::Accept) {
            execute_effect(&mut game, &effect, &mut ctx).expect("effect should resolve");
        }

        assert_eq!(count_lander_tokens(&game, alice), 2);
        assert_eq!(game.counter_count(source, CounterType::PlusOnePlusOne), 0);
    }
}

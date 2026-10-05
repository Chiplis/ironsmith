//! "Unless [player does action]" effect implementation.

use crate::decision::FallbackStrategy;
use crate::decisions::make_boolean_decision;
use crate::effect::{Effect, EffectOutcome};
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget, execute_effect};
use crate::filter::ObjectFilterExt;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;

fn execute_effect_sequence(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    effects: &[Effect],
) -> Result<EffectOutcome, ExecutionError> {
    let mut outcomes = Vec::new();
    for effect in effects {
        outcomes.push(execute_effect(game, effect, ctx)?);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
    }
    Ok(EffectOutcome::aggregate(outcomes))
}

fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    game.team_apnap_player_order()
}

fn cost_effect_sequence_can_execute(
    effects: &[Effect],
    game: &GameState,
    source: ObjectId,
    controller: PlayerId,
) -> Result<(), CostValidationError> {
    for effect in effects {
        let Some(cost_effect) = effect.0.as_cost_executable() else {
            return Err(CostValidationError::Other(format!(
                "unless-action branch contains a non-cost effect: {effect:?}"
            )));
        };
        CostExecutableEffect::can_execute_as_cost(cost_effect, game, source, controller)?;
    }
    Ok(())
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
/// - If the player chooses a feasible alternative cost: its replacement-modified outcome
/// - If the player declines or cannot pay: the outcome of the main effects
/// - Pending input and errors leave the owning operation uncommitted
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

impl UnlessActionEffect {
    /// CR 118.12: the alternative in "[effect] unless [player] [does X]"
    /// works like a cost. A player who can't do all of it can't choose to do
    /// it (Avatar of Discord: discarding one card of two doesn't keep it).
    /// Only alternatives whose feasibility can be checked are gated; an
    /// unknown shape stays offerable.
    fn alternative_is_infeasible(
        &self,
        game: &GameState,
        ctx: &ExecutionContext,
        deciding_player: PlayerId,
    ) -> bool {
        let definitely_unpayable = |result: Result<(), CostValidationError>| -> bool {
            matches!(result, Err(err) if !matches!(err, CostValidationError::Other(_)))
        };
        // Choice effects establish bindings used by later costs. A fresh
        // cost-only context cannot resolve those tags before the choice runs.
        // Keep a read-only set of possible bindings for affordability checks;
        // the actual choice and payment still happen during execution.
        let mut planned_filter = ctx.filter_context(game);
        let mut planned_choice_limits = std::collections::HashMap::new();
        for effect in &self.alternative {
            if let Some(choice) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
                if definitely_unpayable(CostExecutableEffect::can_execute_as_cost(
                    choice,
                    game,
                    ctx.source,
                    deciding_player,
                )) {
                    return true;
                }
                if let Ok(zones) = super::choose_objects::search_zones(choice) {
                    let snapshots = zones.into_iter()
                        .flat_map(|zone| game.objects_in_zone(zone))
                        .filter_map(|id| game.object(id))
                        .filter(|object| !game.is_phased_out(object.id)
                            && choice.filter.matches(object, &planned_filter, game))
                        .map(|object| crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object, game))
                        .collect::<Vec<_>>();
                    let limit = if !choice.count.dynamic_x && choice.count_value.is_none() {
                        choice
                            .count
                            .max
                            .unwrap_or(snapshots.len())
                            .min(snapshots.len())
                    } else {
                        snapshots.len()
                    };
                    planned_choice_limits.insert(choice.tag.clone(), limit);
                    planned_filter
                        .tagged_objects
                        .insert(choice.tag.clone(), snapshots);
                }
                continue;
            }
            if let Some(sacrifice) = effect.downcast_ref::<crate::effects::SacrificeEffect>()
                && sacrifice
                    .filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| planned_choice_limits.contains_key(&constraint.tag))
            {
                // Only a fixed amount has an exact bound before choices.
                // Dynamic dependencies remain unknown rather than falsely
                // declaring an otherwise payable alternative impossible.
                if let crate::effect::Value::Fixed(amount) = sacrifice.count
                    && let Ok(player) = resolve_player_filter(game, &sacrifice.player, ctx)
                {
                    let required = amount.max(0) as usize;
                    if sacrifice.filter.tagged_constraints.iter().any(|constraint| {
                        matches!(constraint.relation,
                            crate::target::TaggedOpbjectRelation::IsTaggedObject
                            | crate::target::TaggedOpbjectRelation::SameObjectId
                            | crate::target::TaggedOpbjectRelation::IsTaggedObjectSacrificedAsSourceEntered)
                            && planned_choice_limits.get(&constraint.tag)
                                .is_some_and(|limit| *limit < required)
                    }) { return true; }
                    let available = game
                        .battlefield
                        .iter()
                        .filter_map(|id| game.object(*id))
                        .filter(|object| {
                            game.controller_of(object) == player
                                && game.can_be_sacrificed(object.id)
                                && sacrifice.filter.matches(object, &planned_filter, game)
                        })
                        .count();
                    if available < required {
                        return true;
                    }
                }
                // Payment mutates the candidate set. Do not project these
                // speculative bindings through another state-changing cost.
                planned_choice_limits.clear();
                planned_filter = ctx.filter_context(game);
                continue;
            }
            if let Some(discard) = effect.downcast_ref::<crate::effects::DiscardEffect>() {
                let Ok(discarder) = resolve_player_filter(game, &discard.player, ctx) else {
                    continue;
                };
                let mut concrete = discard.clone();
                concrete.player = PlayerFilter::Specific(discarder);
                if definitely_unpayable(CostExecutableEffect::can_execute_as_cost(
                    &concrete, game, ctx.source, discarder,
                )) {
                    return true;
                }
                planned_choice_limits.clear();
                planned_filter = ctx.filter_context(game);
                continue;
            }
            if !planned_choice_limits.is_empty() {
                // An intervening action can change identities or eligibility.
                // Existing unknown-shape policy keeps this alternative offerable.
                return false;
            }
            if let Some(cost_effect) = effect.0.as_cost_executable()
                && definitely_unpayable(CostExecutableEffect::can_execute_as_cost(
                    cost_effect,
                    game,
                    ctx.source,
                    deciding_player,
                ))
            {
                return true;
            }
        }
        false
    }
}

/// The per-player part of a simultaneous "each opponent ... unless they
/// [do X]" (CR 101.4 / 608.2e): the choice is made while preparing, the
/// alternative or the main effects happen at commit.
#[derive(Debug)]
struct UnlessActionProposal {
    effects: Vec<Effect>,
    alternative: Vec<Effect>,
    wants_alternative: bool,
    iterated_player: Option<PlayerId>,
}

impl crate::effects::SimultaneousEffectProposal for UnlessActionProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            let proposal = *self;
            ctx.with_temp_iterated_player(proposal.iterated_player, |ctx| {
                if proposal.wants_alternative {
                    // CR 118.11 / 118.12a: replacement-modified payment remains
                    // payment. Control flow follows the accepted feasible cost.
                    return execute_effect_sequence(game, ctx, &proposal.alternative);
                }
                execute_effect_sequence(game, ctx, &proposal.effects)
            })
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending {
            return Ok(EffectOutcome::count(0));
        }
        result
    }
}

impl EffectExecutor for UnlessActionEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        self.player == PlayerFilter::IteratedPlayer
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        if self.player != PlayerFilter::IteratedPlayer {
            return Err(ExecutionError::Impossible(
                "simultaneous unless-action requires an iterated-player chooser".to_string(),
            ));
        }
        let deciding_player = resolve_player_filter(game, &self.player, ctx)?;
        let wants_alternative = !self.alternative_is_infeasible(game, ctx, deciding_player)
            && make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                deciding_player,
                ctx.source,
                "Perform alternative action to prevent effect?".to_string(),
                FallbackStrategy::Accept,
            );
        Ok(Box::new(UnlessActionProposal {
            effects: self.effects.clone(),
            alternative: self.alternative.clone(),
            wants_alternative,
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
        for effect in &self.alternative {
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
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
            let deciding_players = if matches!(self.player, PlayerFilter::Any) {
                players_in_turn_order(game)
            } else {
                vec![resolve_player_filter(game, &self.player, ctx)?]
            };
            // "Target opponent loses 2 life unless they sacrifice ...": outside a
            // player loop, "they"/"that player" in either branch is the single
            // deciding player.
            let bound_player = match deciding_players.as_slice() {
                [player] if ctx.iteration.iterated_player.is_none() => Some(*player),
                _ => ctx.iteration.iterated_player,
            };
            ctx.with_temp_iterated_player(bound_player, |ctx| {
                self.execute_for_deciding_players(game, ctx, deciding_players)
            })
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending {
            return Ok(EffectOutcome::count(0));
        }
        result
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

impl UnlessActionEffect {
    fn execute_for_deciding_players(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        deciding_players: Vec<PlayerId>,
    ) -> Result<EffectOutcome, ExecutionError> {
        for deciding_player in deciding_players {
            if self.alternative_is_infeasible(game, ctx, deciding_player) {
                continue;
            }
            // Ask the player if they want to perform the alternative action.
            let wants_alternative = make_boolean_decision(
                game,
                &mut ctx.decision_maker,
                deciding_player,
                ctx.source,
                "Perform alternative action to prevent effect?".to_string(),
                FallbackStrategy::Accept,
            );

            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0));
            }
            if !wants_alternative {
                continue;
            }

            // Payment choice is authoritative even when a replacement changes
            // or prevents the payment event (CR 118.11 / 118.12a).
            let alternative_outcome = if matches!(self.player, PlayerFilter::Any) {
                ctx.with_temp_targets(vec![ResolvedTarget::Player(deciding_player)], |ctx| {
                    execute_effect_sequence(game, ctx, &self.alternative)
                })?
            } else {
                execute_effect_sequence(game, ctx, &self.alternative)?
            };

            return Ok(alternative_outcome);
        }

        execute_effect_sequence(game, ctx, &self.effects)
    }
}

impl CostExecutableEffect for UnlessActionEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        let main = cost_effect_sequence_can_execute(&self.effects, game, source, controller);
        let alternative =
            cost_effect_sequence_can_execute(&self.alternative, game, source, controller);
        if main.is_ok() || alternative.is_ok() {
            Ok(())
        } else {
            Err(CostValidationError::Other(format!(
                "neither unless-action branch can be paid as a cost: main={main:?}; alternative={alternative:?}"
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
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
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
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

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
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
    #[test]
    fn unless_preflight_retains_prior_choice_bindings() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_creature(&mut game, "Selectable", alice);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        let effect = UnlessActionEffect::new(
            vec![Effect::gain_life(3)],
            sacrifice_creature_alternative(PlayerFilter::You, PlayerFilter::You),
            PlayerFilter::You,
        );
        assert!(!effect.alternative_is_infeasible(&game, &ctx, alice));
        assert!(
            ctx.tagged_objects.is_empty(),
            "preflight must not make the choice"
        );
    }

    #[test]
    fn unless_preflight_respects_selected_tag_cardinality() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        create_creature(&mut game, "First eligible", alice);
        create_creature(&mut game, "Second eligible", alice);
        let source = game.new_object_id();
        let ctx = ExecutionContext::new_default(source, alice);
        let alternative = vec![
            Effect::new(ChooseObjectsEffect::new(
                ObjectFilter::creature().controlled_by(PlayerFilter::You),
                1,
                PlayerFilter::You,
                "chosen",
            )),
            Effect::new(SacrificeEffect::player(
                ObjectFilter::tagged("chosen"),
                2,
                PlayerFilter::You,
            )),
        ];
        let effect =
            UnlessActionEffect::new(vec![Effect::gain_life(3)], alternative, PlayerFilter::You);
        assert!(
            effect.alternative_is_infeasible(&game, &ctx, alice),
            "one chosen object cannot pay a two-object sacrifice"
        );
        assert!(ctx.tagged_objects.is_empty());
    }

    #[test]
    fn unless_chosen_payable_sacrifice_prevented_by_replacement_still_pays_cost() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, "Payment subject", alice);
        let source = game.new_object_id();
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                creature,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::creature().controlled_by(PlayerFilter::You),
                    Some(Zone::Battlefield),
                    Some(Zone::Graveyard),
                ),
                crate::replacement::ReplacementAction::Prevent,
            ),
        );
        game.take_pending_trigger_events();
        let mut dm = AcceptBooleanDecisionMaker;
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        let effect = UnlessActionEffect::new(
            vec![Effect::gain_life(3)],
            vec![Effect::new(SacrificeEffect::player(
                ObjectFilter::creature(),
                1,
                PlayerFilter::You,
            ))],
            PlayerFilter::You,
        );
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("payable optional cost resolves through replacement");
        assert!(!ctx.decision_maker.awaiting_choice());
        assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
        assert_eq!(
            game.player(alice).unwrap().life,
            20,
            "choosing to pay the feasible cost suppresses the unless branch even when payment's event is prevented"
        );
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_none()
        );
        let events = game
            .turn_store
            .turn_history
            .projected_records()
            .map(|record| &record.event)
            .collect::<Vec<_>>();
        assert!(
            !events
                .iter()
                .any(|event| event.downcast::<crate::events::LifeGainEvent>().is_some())
        );
        assert!(
            !events
                .iter()
                .any(|event| event.downcast::<crate::events::SacrificeEvent>().is_some())
        );
    }

    struct PausePaymentPayload {
        pause: bool,
        pending: bool,
        questions: usize,
    }
    impl DecisionMaker for PausePaymentPayload {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _context: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.questions += 1;
            if self.pause && self.questions == 2 {
                self.pending = true;
                false
            } else {
                true
            }
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }

    fn run_optional_payment_owner(
        effect: &UnlessActionEffect,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        simultaneous: bool,
    ) -> Result<EffectOutcome, ExecutionError> {
        if simultaneous {
            let proposal = effect.prepare_simultaneous_player_action(game, ctx)?;
            proposal.commit(game, ctx)
        } else {
            effect.execute(game, ctx)
        }
    }

    fn check_optional_payment_payload_owner(simultaneous: bool, pending: bool) {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let creature = create_creature(&mut game, "Cost replacement object", alice);
        let source = game.new_object_id();
        let effects = if pending {
            vec![
                Effect::gain_life(2),
                Effect::may(vec![Effect::gain_life(4)]),
                Effect::gain_life(8),
            ]
        } else {
            vec![
                Effect::gain_life(2),
                Effect::lose_life(crate::effect::Value::X),
                Effect::gain_life(8),
            ]
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(
            crate::replacement::ReplacementEffect::with_matcher(
                creature,
                alice,
                crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                    ObjectFilter::creature().controlled_by(PlayerFilter::You),
                    Some(Zone::Battlefield),
                    Some(Zone::Graveyard),
                ),
                crate::replacement::ReplacementAction::Instead(effects),
            ),
        );
        let effect = UnlessActionEffect::new(
            vec![Effect::gain_life(3)],
            vec![Effect::new(SacrificeEffect::player(
                ObjectFilter::creature(),
                1,
                PlayerFilter::You,
            ))],
            if simultaneous {
                PlayerFilter::IteratedPlayer
            } else {
                PlayerFilter::You
            },
        );
        game.take_pending_trigger_events();
        let before_live = game.objects_in_deterministic_order().len();
        let before_id = game.next_object_id_counter();
        let mut dm = PausePaymentPayload {
            pause: pending,
            pending: false,
            questions: 0,
        };
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);
        if simultaneous {
            ctx.iteration.iterated_player = Some(alice);
        }
        let result = run_optional_payment_owner(&effect, &mut game, &mut ctx, simultaneous);
        if pending {
            let outcome = result.expect("a pending payment is not an error");
            assert!(ctx.decision_maker.awaiting_choice());
            assert!(outcome.events.is_empty());
        } else {
            assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
            assert!(!ctx.decision_maker.awaiting_choice());
        }
        assert_eq!(ctx.source, source);
        assert_eq!(ctx.controller, alice);
        assert!(ctx.replacement.suppressed_replacement_effects.is_empty());
        drop(ctx);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.objects_in_deterministic_order().len(), before_live);
        assert_eq!(game.next_object_id_counter(), before_id);
        assert!(
            game.effect_store
                .replacement_effects
                .get_effect(shield)
                .is_some()
        );
        assert!(game.take_pending_trigger_events().is_empty());
        if pending {
            assert_eq!(dm.questions, 2);
            let mut replay = PausePaymentPayload {
                pause: false,
                pending: false,
                questions: 0,
            };
            let mut ctx =
                ExecutionContext::new_default(source, alice).with_decision_maker(&mut replay);
            if simultaneous {
                ctx.iteration.iterated_player = Some(alice);
            }
            let outcome =
                run_optional_payment_owner(&effect, &mut game, &mut ctx, simultaneous).unwrap();
            assert!(!ctx.decision_maker.awaiting_choice());
            drop(ctx);
            assert_eq!(replay.questions, 2);
            assert_eq!(game.player(alice).unwrap().life, 34);
            assert_eq!(game.object(creature).unwrap().zone, Zone::Battlefield);
            assert!(
                game.effect_store
                    .replacement_effects
                    .get_effect(shield)
                    .is_none()
            );
            let events = game
                .turn_store
                .turn_history
                .projected_records()
                .map(|record| &record.event)
                .collect::<Vec<_>>();
            let mut gains = events
                .iter()
                .filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                .map(|gain| gain.amount)
                .collect::<Vec<_>>();
            gains.sort();
            assert_eq!(gains, vec![2, 4, 8]);
        }
    }
    #[test]
    fn unless_direct_payment_payload_error_restores_owner() {
        check_optional_payment_payload_owner(false, false);
    }
    #[test]
    fn unless_direct_payment_payload_pending_replays_once() {
        check_optional_payment_payload_owner(false, true);
    }
    #[test]
    fn unless_simultaneous_payment_payload_error_restores_owner() {
        check_optional_payment_payload_owner(true, false);
    }
    #[test]
    fn unless_simultaneous_payment_payload_pending_replays_once() {
        check_optional_payment_payload_owner(true, true);
    }
}

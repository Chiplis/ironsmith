//! Sequence effect implementation.
//!
//! Runs a list of effects in order and aggregates their outcomes.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError, execute_effect, rebase_target_scope};
use crate::game_state::GameState;

/// Effect that executes multiple effects in sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct SequenceEffect {
    /// Effects to execute in order.
    pub effects: Vec<Effect>,
    /// Whether these effects were printed as one coordinated Oracle clause.
    pub surface: ironsmith_core::SequenceSurface,
}

impl SequenceEffect {
    /// Create a new SequenceEffect.
    pub fn new(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::Sequential,
        }
    }

    pub fn coordinated(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::Coordinated,
        }
    }

    pub fn coordinated_with_leading_duration(effects: Vec<Effect>) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::CoordinatedLeadingDuration,
        }
    }

    pub fn result_conjunction(effects: Vec<Effect>, leading_duration: bool) -> Self {
        Self {
            effects,
            surface: ironsmith_core::SequenceSurface::ResultConjunction { leading_duration },
        }
    }
}

impl EffectExecutor for SequenceEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects {
            visitor(effect);
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.effects.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let mut outcomes = Vec::with_capacity(self.effects.len());
        let mut events = Vec::new();
        let mut execution_facts = Vec::new();
        let coordinated_assignments = (self.surface != ironsmith_core::SequenceSurface::Sequential
            && !ctx.target_assignments.is_empty())
        .then(|| ctx.target_assignments.clone());
        let chosen_modes = ctx.chosen_modes.clone();
        let mut consumed_modal_selection = false;
        let mut assignment_cursor = 0usize;

        for effect in &self.effects {
            let assignment_count = if coordinated_assignments.is_some() {
                crate::game_loop::count_target_selection_slots_for_isolated_effect(
                    effect,
                    chosen_modes.as_deref(),
                    &mut consumed_modal_selection,
                )
            } else {
                0
            };
            let outcome = if assignment_count > 0 {
                let assignments = coordinated_assignments
                    .as_ref()
                    .expect("coordinated assignments checked above");
                let end = assignment_cursor
                    .saturating_add(assignment_count)
                    .min(assignments.len());
                let scoped_assignments = assignments[assignment_cursor..end].to_vec();
                assignment_cursor = end;
                let (scoped_targets, scoped_assignments) =
                    rebase_target_scope(&ctx.targets, &scoped_assignments);
                ctx.with_temp_targets(scoped_targets, |ctx| {
                    ctx.with_temp_target_assignments(scoped_assignments, |ctx| {
                        execute_effect(game, effect, ctx)
                    })
                })?
            } else {
                execute_effect(game, effect, ctx)?
            };
            events.extend(outcome.events.clone());
            execution_facts.extend(outcome.execution_facts.clone());

            if outcome.status.is_failure() {
                return Ok(EffectOutcome::with_details(
                    outcome.status,
                    outcome.value.clone(),
                    events,
                    execution_facts,
                ));
            }

            outcomes.push(outcome);
            if ctx.decision_maker.awaiting_choice() {
                let aggregate = EffectOutcome::aggregate(outcomes);
                return Ok(EffectOutcome::with_details(
                    aggregate.status,
                    aggregate.value,
                    events,
                    execution_facts,
                ));
            }
        }

        let aggregate = EffectOutcome::aggregate(outcomes);
        Ok(EffectOutcome::with_details(
            aggregate.status,
            aggregate.value,
            events,
            execution_facts,
        ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::decision::DecisionMaker;
    use crate::decisions::context::BooleanContext;
    use crate::effect::{ChoiceCount, Until, Value};
    use crate::effects::ResolvedTarget;
    use crate::effects::continuous::RuntimeModification;
    use crate::game_state::TargetAssignment;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ChooseSpec;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.add_object(Object::from_card(id, &card, controller, Zone::Battlefield));
        id
    }

    #[derive(Clone, Debug)]
    struct PendingChoiceEffect;

    impl EffectExecutor for PendingChoiceEffect {
        fn execute(
            &self,
            game: &mut GameState,
            ctx: &mut ExecutionContext,
        ) -> Result<EffectOutcome, ExecutionError> {
            let prompt = BooleanContext::new(ctx.controller, Some(ctx.source), "pause");
            ctx.decision_maker.decide_boolean(game, &prompt);
            Ok(EffectOutcome::count(0))
        }
    }

    #[derive(Default)]
    struct CapturingDecisionMaker {
        pending: bool,
    }

    impl DecisionMaker for CapturingDecisionMaker {
        fn awaiting_choice(&self) -> bool {
            self.pending
        }

        fn decide_boolean(&mut self, _game: &GameState, _ctx: &BooleanContext) -> bool {
            self.pending = true;
            false
        }
    }

    #[test]
    fn sequence_forwards_inner_target_spec() {
        let effect = SequenceEffect::new(vec![
            Effect::gain_life(1),
            Effect::counter(ChooseSpec::target_spell()),
        ]);

        assert!(effect.get_target_spec().is_some());
        assert_eq!(effect.target_description(), "spell to counter");
    }

    #[test]
    fn sequence_uses_conservative_summary_for_multiple_meaningful_results() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let result = SequenceEffect::new(vec![Effect::gain_life(1), Effect::gain_life(2)])
            .execute(&mut game, &mut ctx)
            .expect("sequence should execute");

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn sequence_stops_before_later_effects_when_inner_effect_needs_choice() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let starting_life = game.player(alice).expect("Alice exists").life;
        let source = game.new_object_id();
        let mut dm = CapturingDecisionMaker::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        SequenceEffect::new(vec![Effect::new(PendingChoiceEffect), Effect::gain_life(3)])
            .execute(&mut game, &mut ctx)
            .expect("sequence should surface the pending choice");

        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(
            game.player(alice).expect("Alice exists").life,
            starting_life,
            "later sequence effects must not run before the pending choice is answered"
        );
    }

    #[test]
    fn coordinated_runtime_effects_use_independent_equal_target_assignments() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Blue Dragon", alice);
        let first = create_creature(&mut game, "First Target", alice);
        let second = create_creature(&mut game, "Second Target", alice);
        let third = create_creature(&mut game, "Third Target", alice);
        let spec = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::up_to(1));
        let pump = |amount, tag: &'static str| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec.clone(),
                    RuntimeModification::ModifyPowerToughness {
                        power: Value::Fixed(amount),
                        toughness: Value::Fixed(0),
                    },
                    Until::YourNextTurn,
                )
                .require_creature_target(),
            )
            .tag(tag)
        };
        let sequence = SequenceEffect::coordinated(vec![
            pump(-3, "first"),
            pump(-2, "second"),
            pump(-1, "third"),
        ]);
        let mut consumed_modal_selection = false;
        assert_eq!(
            crate::game_loop::count_target_selection_slots_for_isolated_effect(
                &Effect::new(sequence.clone()),
                None,
                &mut consumed_modal_selection,
            ),
            3,
            "the runtime planner must retain all three target words"
        );
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![
                ResolvedTarget::Object(first),
                ResolvedTarget::Object(second),
                ResolvedTarget::Object(third),
            ])
            .with_target_assignments(vec![
                TargetAssignment {
                    spec: spec.clone(),
                    range: 0..1,
                },
                TargetAssignment {
                    spec: spec.clone(),
                    range: 1..2,
                },
                TargetAssignment { spec, range: 2..3 },
            ]);

        sequence
            .execute(&mut game, &mut ctx)
            .expect("execute coordinated pumps");

        assert_eq!(game.calculated_power(first), Some(-1));
        assert_eq!(game.calculated_power(second), Some(0));
        assert_eq!(game.calculated_power(third), Some(1));
    }
}

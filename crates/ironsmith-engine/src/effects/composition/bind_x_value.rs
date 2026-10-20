//! A die-result table row that fixes X for the program it governs.
//!
//! "Roll a d20. ... You may cast up to X ... . 1—9 | X is one. 10—19 | X is
//! two. 20 | X is three." (Wand of Wonder): the row the result selects runs
//! the governed program with X equal to its value (CR 706.2, 107.3).
use crate::effect::{Effect, EffectOutcome, Value};
use crate::effects::helpers::resolve_value;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError, execute_effect};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

#[derive(Debug, Clone, PartialEq)]
pub struct BindXValueEffect {
    pub value: Value,
    pub effects: Vec<Effect>,
}

impl BindXValueEffect {
    pub fn new(value: Value, effects: Vec<Effect>) -> Self {
        Self { value, effects }
    }
}

impl EffectExecutor for BindXValueEffect {
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
        let value = resolve_value(game, &self.value, ctx)?.max(0) as u32;
        let saved_x = ctx.x_value;
        ctx.x_value = Some(value);
        let mut outcomes = Vec::with_capacity(self.effects.len());
        let mut failure = None;
        for effect in &self.effects {
            match execute_effect(game, effect, ctx) {
                Ok(outcome) => outcomes.push(outcome),
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
        }
        ctx.x_value = saved_x;
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(EffectOutcome::aggregate(outcomes))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        super::target_metadata::first_target_spec(&[&self.effects])
    }
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        super::target_metadata::related_object_specs(&[&self.effects])
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
    use crate::decisions::context::SelectObjectsContext;
    use crate::effect::ChoiceCount;
    use crate::filter::{ObjectFilter, ObjectFilterExt as _, PlayerFilter};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::Object;
    use crate::zone::Zone;

    #[derive(Default)]
    struct PausingObjectChoice {
        pause: bool,
        offered_max: Vec<usize>,
    }

    impl DecisionMaker for PausingObjectChoice {
        fn awaiting_choice(&self) -> bool {
            self.pause
        }

        fn decide_objects(&mut self, _game: &GameState, choice: &SelectObjectsContext) -> Vec<ObjectId> {
            self.offered_max.push(choice.max.expect("bounded dynamic choice"));
            if self.offered_max.len() == 1 {
                self.pause = true;
                Vec::new()
            } else {
                choice.candidates.iter().take(2).map(|candidate| candidate.id).collect()
            }
        }
    }

    #[test]
    fn x_bound_survives_pending_choice_replay() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..3 {
            let id = game.new_object_id();
            let card = CardBuilder::new(CardId::from_raw(id.0 as u32), format!("Exiled spell {index}"))
                .card_types(vec![crate::types::CardType::Instant])
                .build();
            game.add_object(Object::from_card(id, &card, alice, Zone::Exile));
        }
        let source = game.new_object_id();
        let choice = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::default().in_zone(Zone::Exile),
            ChoiceCount::up_to_dynamic_x(),
            PlayerFilter::You,
            "chosen",
        ).in_zone(Zone::Exile);
        let bound = BindXValueEffect::new(Value::Fixed(2), vec![Effect::new(choice)]);
        let mut decision = PausingObjectChoice::default();
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision);
            bound.execute(&mut game, &mut ctx).unwrap();
            assert!(ctx.decision_maker.awaiting_choice());
            assert_eq!(ctx.x_value, None, "binder restores the outer X on pause");
        }
        decision.pause = false;
        {
            let mut ctx = ExecutionContext::new(source, alice, &mut decision);
            bound.execute(&mut game, &mut ctx).unwrap();
            assert!(!ctx.decision_maker.awaiting_choice());
            assert_eq!(ctx.get_tagged_all("chosen").unwrap().len(), 2);
            assert_eq!(ctx.x_value, None, "binder restores the outer X on completion");
        }
        assert_eq!(decision.offered_max, vec![2, 2], "neither prompt may offer three cards");
    }
}

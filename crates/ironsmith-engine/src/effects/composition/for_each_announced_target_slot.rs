//! Ordered iteration over target slots chosen while a spell or ability was
//! announced. Invalid slots remain iterations so their untargeted riders run.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::{GameState, TargetAssignment};
use crate::target::ChooseSpec;

pub type ForEachAnnouncedTargetSlotEffect =
    ironsmith_core::ForEachAnnouncedTargetSlotEffect<Effect>;

struct AnnouncedSlotPlan {
    effects: Vec<Effect>,
    spec: ChooseSpec,
    slots: Vec<(crate::effects::ResolvedTarget, bool)>,
}

impl super::iteration_program::SelectedIterationPlan for AnnouncedSlotPlan {
    fn len(&self) -> usize { self.slots.len() }
    fn effects(&self) -> &[Effect] { &self.effects }
    fn target_scope(&self, index: usize) -> Option<(
        Vec<crate::effects::ResolvedTarget>, Vec<TargetAssignment>,
    )> {
        let (target, legal) = self.slots[index];
        let targets = if legal { vec![target] } else { Vec::new() };
        let len = targets.len();
        Some((targets, vec![TargetAssignment {
            spec: self.spec.clone(), range: 0..len,
        }]))
    }
    fn select(
        &mut self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
        index: usize,
    ) -> Result<super::iteration_program::IterationInput, ExecutionError> {
        let target = self.slots[index].0;
        let (object, player) = match target {
            crate::effects::ResolvedTarget::Object(id) => (Some(id), ctx.controller),
            crate::effects::ResolvedTarget::Player(id) => (None, id),
        };
        Ok(super::iteration_program::IterationInput { object, player, tags: Vec::new() })
    }
}

fn slot_cursor(
    effect: &ForEachAnnouncedTargetSlotEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<Box<dyn crate::effects::ActionProgramCursor>, ExecutionError> {
    let start = crate::resolve_value(game, &effect.start, ctx)?;
    let count = crate::resolve_value(game, &effect.count, ctx)?;
    let (Ok(start), Ok(count)) = (usize::try_from(start), usize::try_from(count)) else {
        return Err(ExecutionError::UnresolvableValue(
            "announced target range has a negative start or count".into(),
        ));
    };
    let assignment = ctx.announced_target_assignments.iter()
        .find(|assignment| assignment.spec == effect.target_spec)
        .ok_or_else(|| ExecutionError::IncompleteEvidence(
            "ordered target iteration has no announced target assignment".into(),
        ))?;
    let end = start.checked_add(count).ok_or_else(|| ExecutionError::UnresolvableValue(
        "announced target range overflow".into(),
    ))?;
    if end > assignment.range.len() || assignment.range.end > ctx.announced_target_slots.len() {
        return Err(ExecutionError::IncompleteEvidence(
            "ordered target range exceeds announced slots".into(),
        ));
    }
    let range_start = assignment.range.start + start;
    let slots = ctx.announced_target_slots[range_start..range_start + count].to_vec();
    Ok(super::iteration_program::selected_iteration_cursor(Box::new(AnnouncedSlotPlan {
        effects: effect.effects.clone(),
        spec: effect.target_spec.clone(),
        slots,
    }), ctx))
}

impl EffectExecutor for ForEachAnnouncedTargetSlotEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> { Box::new(self.clone()) }
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        for effect in &self.effects { visitor(effect); }
    }
    fn supports_prepared_action_program(&self) -> bool {
        self.effects.iter().all(super::action_program::action_program_child_is_prepared)
    }
    fn select_prepared_action_program(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<Option<Box<dyn crate::effects::ActionProgramCursor>>, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(None); }
        Ok(Some(slot_cursor(self, game, ctx)?))
    }
    fn execute(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self, game: &mut GameState, ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game, ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let cursor = slot_cursor(self, game, ctx)?;
                super::action_program::execute_action_program_with_outputs(
                    cursor, game, ctx, crate::effects::EffectExecutionPurpose::Action,
                )
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{ChoiceCount, Value};
    use crate::effects::ResolvedTarget;
    use crate::ids::CardId;
    use crate::target::ObjectFilter;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn artifact(game: &mut GameState, owner: crate::ids::PlayerId) -> crate::ids::ObjectId {
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Artifact")
            .card_types(vec![CardType::Artifact])
            .build();
        game.create_object_from_definition(&definition, owner, Zone::Battlefield)
    }

    fn group(spec: &ChooseSpec, start: i32, count: i32, with_life: bool) -> Effect {
        let destroy = Effect::conditional_only(
            crate::effect::Condition::TargetMatches(ObjectFilter::default()),
            vec![Effect::new(crate::effects::DestroyEffect::with_spec(ChooseSpec::Iterated))],
        );
        let mut effects = vec![destroy];
        if with_life { effects.push(Effect::gain_life(1)); }
        Effect::new(ForEachAnnouncedTargetSlotEffect {
            target_spec: spec.clone(),
            start: Value::Fixed(start),
            count: Value::Fixed(count),
            effects,
        })
    }

    #[test]
    fn illegal_green_target_still_gains_life_after_ordered_base_and_red_actions() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let base = artifact(&mut game, alice);
        let red = artifact(&mut game, alice);
        let green = artifact(&mut game, alice);
        let source = game.new_object_id();
        let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact()))
            .with_count_value(ChoiceCount::dynamic_x(), Value::Fixed(3));
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(base), ResolvedTarget::Object(red)])
            .with_announced_target_assignments(vec![TargetAssignment {
                spec: spec.clone(), range: 0..3,
            }])
            .with_announced_target_slots(vec![
                (ResolvedTarget::Object(base), true),
                (ResolvedTarget::Object(red), true),
                (ResolvedTarget::Object(green), false),
            ]);
        crate::effects::execute_effect(&mut game, &group(&spec, 0, 1, false), &mut ctx).unwrap();
        assert!(!game.battlefield.contains(&base));
        assert!(game.battlefield.contains(&red));
        assert_eq!(game.player(alice).unwrap().life, 20);
        crate::effects::execute_effect(&mut game, &group(&spec, 1, 1, false), &mut ctx).unwrap();
        assert!(!game.battlefield.contains(&red));
        assert_eq!(game.player(alice).unwrap().life, 20);
        crate::effects::execute_effect(&mut game, &group(&spec, 2, 1, true), &mut ctx).unwrap();
        assert!(game.battlefield.contains(&green));
        assert_eq!(game.player(alice).unwrap().life, 21);
    }

    #[test]
    fn illegal_early_slot_does_not_shift_a_later_payment_group() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = game.players[0].id;
        let base = artifact(&mut game, alice);
        let green = artifact(&mut game, alice);
        let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::artifact()))
            .with_count_value(ChoiceCount::dynamic_x(), Value::Fixed(2));
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![ResolvedTarget::Object(green)])
            .with_announced_target_assignments(vec![TargetAssignment {
                spec: spec.clone(), range: 0..2,
            }])
            .with_announced_target_slots(vec![
                (ResolvedTarget::Object(base), false),
                (ResolvedTarget::Object(green), true),
            ]);
        crate::effects::execute_effect(&mut game, &group(&spec, 0, 1, false), &mut ctx).unwrap();
        assert!(game.battlefield.contains(&base));
        crate::effects::execute_effect(&mut game, &group(&spec, 1, 1, true), &mut ctx).unwrap();
        assert!(!game.battlefield.contains(&green));
        assert_eq!(game.player(alice).unwrap().life, 21);
    }
}

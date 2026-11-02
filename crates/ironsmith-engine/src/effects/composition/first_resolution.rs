//! First resolving program in one authored family, shared across sources.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError, SequenceEffect};
use crate::game_state::GameState;
pub type FirstResolutionEachTurnEffect = ironsmith_core::FirstResolutionEachTurnEffect<Effect>;
impl EffectExecutor for FirstResolutionEachTurnEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> { Box::new(self.clone()) }
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) { for effect in &self.effects { visitor(effect); } }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx).map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(game, ctx, || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)), |game, ctx| {
            let label = self.label.trim().to_ascii_lowercase();
            if label.is_empty() { return Err(ExecutionError::InternalError("shared resolution label is empty".into())); }
            let key = format!("shared-ability-resolution:{}:{}:{}", ctx.controller.0, label.len(), label);
            let first = game.named_turn_counter(&key) == 0;
            // Record resolution before executing the child program: prevention,
            // replacement, and zero created objects do not undo a resolution.
            // The enclosing transaction rolls this back for a pending choice.
            game.increment_named_turn_counter(key);
            if !first { return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))); }
            SequenceEffect::new(self.effects.clone()).execute_with_outputs(game, ctx)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::PlayerId;
    #[derive(Debug,Clone,PartialEq)]
    struct PreventedChild;
    impl EffectExecutor for PreventedChild {
        fn clone_box(&self) -> Box<dyn EffectExecutor> {Box::new(self.clone())}
        fn execute(&self, game:&mut GameState, _: &mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
            game.increment_named_turn_counter("first-resolution-test-child");
            Ok(EffectOutcome::prevented())
        }
    }
    #[test]
    fn prevented_child_still_consumes_shared_resolution_independently_for_each_controller() {
        let a=PlayerId(0);let b=PlayerId(1);
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=crate::card::CardBuilder::new(crate::ids::CardId::new(),"Witness")
            .card_types(vec![crate::types::CardType::Artifact]).build();
        let one=game.create_object_from_card(&card,a,crate::Zone::Battlefield);
        let two=game.create_object_from_card(&card,a,crate::Zone::Battlefield);
        let wrapper=FirstResolutionEachTurnEffect{label:"shared test label".into(),effects:vec![Effect::new(PreventedChild)]};
        for source in [one,two] {wrapper.execute(&mut game,&mut ExecutionContext::new_default(source,a)).unwrap();}
        assert_eq!(game.named_turn_counter("first-resolution-test-child"),1);
        wrapper.execute(&mut game,&mut ExecutionContext::new_default(two,b)).unwrap();
        assert_eq!(game.named_turn_counter("first-resolution-test-child"),2,"controller, not source owner, owns the shared receipt");
        game.next_turn();
        wrapper.execute(&mut game,&mut ExecutionContext::new_default(one,a)).unwrap();
        assert_eq!(game.named_turn_counter("first-resolution-test-child"),1,"the turn owner clears both receipt and test observation");
    }
}

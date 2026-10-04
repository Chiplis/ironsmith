use crate::decisions::context::NumberContext;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::ChooseNumberEffect;
impl EffectExecutor for ChooseNumberEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> { Box::new(self.clone()) }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        if self.min > self.max { return Err(ExecutionError::Impossible("numeric choice has an empty range".into())); }
        if self.max > i32::MAX as u32 { return Err(ExecutionError::ResourceLimitExceeded { resource:"chosen number representation", requested:self.max as u128, maximum:i32::MAX as u128 }); }
        let chooser=crate::effects::helpers::resolve_player_filter_as_chooser(game,&self.chooser,ctx)?;
        let choice=NumberContext::new(chooser,Some(ctx.source),self.min,self.max,"Choose a number");
        let number=ctx.decision_maker.decide_number(game,&choice);
        if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::resolved())}
        if !(self.min..=self.max).contains(&number) {return Err(ExecutionError::Impossible("number is outside the authored choice range".into()))}
        Ok(EffectOutcome::count(number as i32).with_execution_fact(ExecutionFact::ChosenNumber(number)))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId,PlayerId};
    use crate::target::PlayerFilter;
    struct Number { value:u32, pending:bool }
    impl crate::decision::DecisionMaker for Number {
        fn decide_number(&mut self,_:&GameState,ctx:&NumberContext)->u32 {assert_eq!((ctx.min,ctx.max),(0,13));self.value}
        fn awaiting_choice(&self)->bool {self.pending}
    }
    #[test]
    fn finite_bounds_and_pending_are_not_silently_clamped_or_published() {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);let a=PlayerId::from_index(0);let source=ObjectId::from_raw(7);
        for number in [0,13] {let mut dm=Number{value:number,pending:false};let outcome=ChooseNumberEffect::new(PlayerFilter::You,0,13).execute(&mut game,&mut ExecutionContext::new(source,a,&mut dm)).unwrap();assert_eq!(outcome.as_count(),Some(i64::from(number)));assert!(outcome.execution_facts().contains(&ExecutionFact::ChosenNumber(number)));}
        let mut invalid=Number{value:14,pending:false};assert!(ChooseNumberEffect::new(PlayerFilter::You,0,13).execute(&mut game,&mut ExecutionContext::new(source,a,&mut invalid)).is_err());
        let mut pending=Number{value:5,pending:true};let outcome=ChooseNumberEffect::new(PlayerFilter::You,0,13).execute(&mut game,&mut ExecutionContext::new(source,a,&mut pending)).unwrap();assert!(outcome.execution_facts().is_empty());
        let mut dm=Number{value:0,pending:false};assert!(matches!(ChooseNumberEffect::new(PlayerFilter::You,0,u32::MAX).execute(&mut game,&mut ExecutionContext::new(source,a,&mut dm)),Err(ExecutionError::ResourceLimitExceeded{..})));
    }
}

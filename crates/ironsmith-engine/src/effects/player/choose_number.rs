use crate::decisions::context::NumberContext;
use crate::effect::{EffectOutcome, ExecutionFact};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub use ironsmith_core::ChooseNumberEffect;
impl EffectExecutor for ChooseNumberEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> { Box::new(self.clone()) }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        if self.max.is_some_and(|max| self.min > max) { return Err(ExecutionError::Impossible("numeric choice has an empty range".into())); }
        let owner=if self.source_owned {
            Some(ctx.source_number_owner.as_ref().filter(|owner|owner.host==ctx.source)
                .ok_or_else(||ExecutionError::IncompleteEvidence("source numeric choice has no admitted linked acquisition".into()))?.clone())
        }else{None};
        let chooser=crate::effects::helpers::resolve_player_filter_as_chooser(game,&self.chooser,ctx)?;
        let mut choice=NumberContext::new(chooser,Some(ctx.source),self.min,self.max.unwrap_or(u32::MAX),"Choose a number");
        choice.authored_max = self.max;
        let number=ctx.decision_maker.decide_number(game,&choice);
        if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::resolved())}
        if number < self.min || self.max.is_some_and(|max| number > max) {return Err(ExecutionError::Impossible("number is outside the authored choice range".into()))}
        if let Some(owner)=owner {
            if game.object(ctx.source).is_some() { game.set_number_for_acquisition(owner,number)?; }
        }
        Ok(EffectOutcome::count(i64::from(number)).with_execution_fact(ExecutionFact::ChosenNumber(number)))
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
    }
}

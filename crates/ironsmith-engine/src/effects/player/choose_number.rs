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
        let legal_max = if self.draw_count_choice {
            if !game.can_draw(chooser) {
                0
            } else if game.can_draw_extra_cards(chooser) {
                self.max.unwrap_or(u32::MAX)
            } else if game.turn_store.turn_history.has_drawn_cards_this_turn(chooser)? {
                0
            } else {
                self.max.unwrap_or(u32::MAX).min(1)
            }
        } else {
            self.max.unwrap_or(u32::MAX)
        };
        if legal_max < self.min { return Err(ExecutionError::Impossible("draw count choice has no legal value".into())); }
        let mut choice=NumberContext::new(chooser,Some(ctx.source),self.min,legal_max,"Choose a number");
        choice.authored_max = self.max;
        let number=ctx.decision_maker.decide_number(game,&choice);
        if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::resolved())}
        if number < self.min || number > legal_max {return Err(ExecutionError::Impossible("number is outside the legal choice range".into()))}
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

    #[test]
    fn draw_count_choices_obey_static_one_draw_limit_without_changing_other_numbers() {
        use crate::ability::Ability;
        use crate::card::CardBuilder;
        use crate::ids::CardId;
        use crate::static_abilities::StaticAbility;
        use crate::types::CardType;
        use crate::zone::Zone;
        struct Answer { expected_max: u32 }
        impl crate::decision::DecisionMaker for Answer {
            fn decide_number(&mut self, _: &GameState, context: &NumberContext) -> u32 {
                assert_eq!(context.max, self.expected_max);
                assert_eq!(context.authored_max, Some(2));
                2
            }
        }
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let card = CardBuilder::new(CardId::new(), "Draw limit")
            .card_types(vec![CardType::Creature]).build();
        let source = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let filler = CardBuilder::new(CardId::new(), "Filler")
            .card_types(vec![CardType::Land]).build();
        game.create_object_from_card(&filler, bob, Zone::Library);
        game.object_mut(source).unwrap().abilities_mut().push(Ability::static_ability(
            StaticAbility::opponents_cant_draw_extra_cards(),
        ));
        game.update_cant_effects();
        for (already_drew, legal_max) in [(false, 1), (true, 0)] {
            if already_drew {
                let mut answers = crate::decision::SelectFirstDecisionMaker;
                let mut ctx = ExecutionContext::new(source, alice, &mut answers);
                crate::effects::execute_effect(&mut game, &crate::effect::Effect::new(
                    crate::effects::DrawCardsEffect::new(1, PlayerFilter::Specific(bob)),
                ), &mut ctx).unwrap();
            }
            let mut answers = Answer { expected_max: legal_max };
            let mut ctx = ExecutionContext::new(source, alice, &mut answers);
            assert!(ChooseNumberEffect::new(PlayerFilter::Specific(bob), 0, 2)
                .for_draw_count().execute(&mut game, &mut ctx).is_err());
        }
        let mut answers = Answer { expected_max: 2 };
        let mut ctx = ExecutionContext::new(source, alice, &mut answers);
        assert_eq!(ChooseNumberEffect::new(PlayerFilter::Specific(bob), 0, 2)
            .execute(&mut game, &mut ctx).unwrap().as_count(), Some(2));
    }
}

//! Effect-driven activation uses the ordinary retained-occurrence announcer;
//! it never executes another ability's effect list as a substitute activation.
use crate::effects::{CompletedEffectOutputs,EffectExecutor,ExecutionContext,ExecutionError};
use crate::effect::EffectOutcome;
use crate::game_state::GameState;
pub use ironsmith_core::ActivateAbilityWithoutCostsEffect;
impl EffectExecutor for ActivateAbilityWithoutCostsEffect {
    fn own_preflight_object_specs(&self)->Vec<crate::target::ChooseSpec>{vec![self.source.clone()]}
    fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{self.execute_with_outputs(game,ctx).map(CompletedEffectOutputs::into_outcome)}
    fn execute_with_outputs(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<CompletedEffectOutputs,ExecutionError>{
        let objects=match crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.source){Ok(objects)=>objects,Err(ExecutionError::InvalidTarget)=>Vec::new(),Err(error)=>return Err(error)};
        if ctx.decision_maker.awaiting_choice()||objects.is_empty(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
        let mut candidates=Vec::new();
        let mut occurrences=Vec::new();
        for source in &objects {
        let Some(chars)=game.current_characteristics(*source)else{continue;};
        for(index,ability)in chars.abilities.iter().enumerate(){
            let crate::ability::AbilityKind::Activated(activated)=&ability.kind else{continue;};
            if !designation_matches(activated, self.designation.as_deref()) { continue; }
            if !crate::decision::can_invoke_designated_ability_without_costs(game,*source,index,ctx.controller,activated,self.invocation_action.as_deref()){continue;}
            let label=chars.ability_labels.get(index).filter(|label|!label.is_empty()).cloned().unwrap_or_else(||format!("Ability {} ({})",index+1,activated.mana_cost.display()));
            let option=occurrences.len();
            occurrences.push((*source,index));
            candidates.push(crate::decisions::context::SelectableOption::new(option,label));
        }
        }
        if candidates.is_empty(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
        let options=crate::decisions::context::SelectOptionsContext::new(ctx.controller,Some(ctx.source),"Choose an activated ability",candidates,1,1);
        let chosen=ctx.decision_maker.decide_options(game,&options);
        if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
        let [index]=chosen.as_slice()else{return Err(ExecutionError::InvalidTarget);};
        if !options.options.iter().any(|option|option.index==*index&&option.legal){return Err(ExecutionError::InvalidTarget);}
        let (source,ability_index)=occurrences[*index];
        let occurrence=crate::game_loop::RetainedActivatedOccurrence::capture(game,source,ability_index,ctx.controller)?;
        let published=crate::game_loop::announce_retained_activated_occurrence_without_costs(game,ctx,occurrence).map_err(|error|match error{crate::game_loop::GameLoopError::ExecutionFailed(error)=>error,other=>ExecutionError::InternalError(other.to_string())})?;
        let mut outputs=CompletedEffectOutputs::aggregate_only(EffectOutcome::count(i32::from(published.is_some())));
        if let Some(published)=published{outputs.retain_published_references([published]);}
        Ok(outputs)
    }
}

fn designation_matches(ability: &crate::ability::ActivatedAbility, wanted: Option<&str>) -> bool {
    wanted.is_none_or(|wanted| ability.effects.activation_designations.iter().any(|actual| actual.eq_ignore_ascii_case(wanted)))
}

#[cfg(test)]
mod designation_tests {
    use super::*;
    #[test]
    fn shared_designations_are_typed_and_independent_of_effect_similarity() {
        let wrapper=crate::ability::Ability::activated(crate::cost::TotalCost::free(),vec![crate::effect::Effect::gain_life(1)]);
        let crate::ability::AbilityKind::Activated(mut ability)=wrapper.kind else {unreachable!()};
        assert!(designation_matches(&ability,None));
        assert!(!designation_matches(&ability,Some("pledge")));
        ability.effects.activation_designations.push("pledge".into());
        assert!(designation_matches(&ability,Some("Pledge")));
        assert!(!designation_matches(&ability,Some("another")));
    }
}

#[cfg(test)]
mod designated_invocation_behavior {
    use super::*;
    use crate::{Ability,CardDefinitionBuilder,CardId,TotalCost,PlayerId,Zone};
    fn fixture(exception: bool) -> (GameState, crate::ids::ObjectId) {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        game.turn.priority_player=Some(PlayerId(1));
        let mut marked=Ability::activated_with_timing(TotalCost::from_costs(vec![crate::costs::Cost::tap(),crate::costs::Cost::sacrifice_self()]),vec![crate::effect::Effect::gain_life(2)],crate::ability::ActivationTiming::SorcerySpeed);
        if let crate::ability::AbilityKind::Activated(ability)=&mut marked.kind {
            ability.effects.activation_designations.push("pledge".into());
            if exception { ability.effects.activation_sorcery_exceptions.push("making-a-pledge".into()); }
        }
        let definition=CardDefinitionBuilder::new(CardId::new(),"Authored labeled abilities")
            .card_types(vec![crate::types::CardType::Enchantment])
            .with_ability(Ability::activated(TotalCost::free(),vec![crate::effect::Effect::gain_life(9)]))
            .with_ability(marked).build();
        let source=game.create_object_from_definition(&definition,PlayerId(0),Zone::Battlefield);
        (game,source)
    }
    fn invoke(game:&mut GameState,source:crate::ids::ObjectId,action:Option<&str>) {
        let effect=ActivateAbilityWithoutCostsEffect { source:crate::target::ChooseSpec::SpecificObject(source),designation:Some("pledge".into()),invocation_action:action.map(str::to_string) };
        let mut dm=crate::decision::SelectFirstDecisionMaker;
        let mut ctx=ExecutionContext::new(source,PlayerId(0),&mut dm);
        effect.execute(game,&mut ctx).unwrap();
    }
    #[test]
    fn instruction_selects_designation_and_explicit_exception_without_paying_or_resolving() {
        let(mut game,source)=fixture(true);let priority=game.turn.priority_player;
        invoke(&mut game,source,Some("making-a-pledge"));
        assert_eq!(game.stack.len(),1);
        assert_eq!(game.stack[0].ability_index,Some(1));
        assert!(game.stack[0].is_ability);
        assert!(game.object(source).is_some());assert!(!game.is_tapped(source));
        assert_eq!(game.player(PlayerId(0)).unwrap().life,20);
        assert_eq!(game.turn.priority_player,priority);
    }
    #[test]
    fn waived_costs_do_not_override_unrelated_or_missing_timing_exception() {
        for (exception,action) in [(false,Some("making-a-pledge")),(true,None),(true,Some("another-action"))] {
            let(mut game,source)=fixture(exception);invoke(&mut game,source,action);assert!(game.stack.is_empty());
        }
    }
}

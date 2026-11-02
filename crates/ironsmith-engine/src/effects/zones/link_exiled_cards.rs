//! A named association between exiled physical cards and a battlefield host.
//! Selection, exile, and optionality are owned by ordinary composition.
use crate::effects::{CompletedEffectOutputs,EffectExecutor,ExecutionContext,ExecutionError};
use crate::effect::EffectOutcome;
use crate::{GameState,Zone};
pub use ironsmith_core::LinkExiledCardsEffect;
impl EffectExecutor for LinkExiledCardsEffect {
    fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
        self.execute_with_outputs(game,ctx).map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<CompletedEffectOutputs,ExecutionError>{
        crate::effects::composition::execute_transaction(game,ctx,
            ||CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),|game,ctx|{
            let hosts=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.host)?;
            if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
            if hosts.len()!=1{return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid()));}
            let host=hosts[0];
            if !game.object(host).is_some_and(|object|object.zone==Zone::Battlefield)||game.is_phased_out(host){
                return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid()));
            }
            let cards=crate::effects::helpers::resolve_objects_for_effect(game,ctx,&self.cards)?;
            if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
            let mut seen=std::collections::HashSet::new();let mut linked=Vec::new();let mut children=Vec::new();
            for card in cards.into_iter().filter(|card|seen.insert(*card)){
                if !game.link_exiled_card(&self.relationship,card,host){continue;}
                linked.push(card);
                // The existing named completion event carries the host as
                // performer and the exact related card as its object packet.
                let receipt=crate::effects::PerformNamedActionEffect{
                    label:self.relationship.trim().to_ascii_lowercase(),
                    performer:Some(crate::target::ChooseSpec::SpecificObject(host)),
                    objects:Some(crate::target::ChooseSpec::SpecificObject(card)),prior_objects:None,
                }.execute_with_outputs(game,ctx)?;
                children.push(receipt);
                if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
            }
            Ok(CompletedEffectOutputs::from_children(children,|outcomes|
                EffectOutcome::aggregate_with_primary_result(EffectOutcome::count(linked.len().min(i32::MAX as usize)as i32)
                    .with_affected_objects(linked),outcomes)))
        })
    }
}

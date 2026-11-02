//! Reuse physical merged components for an atomic battlefield merge.
use crate::effect::EffectOutcome;
use crate::effects::{CompletedEffectOutputs,EffectExecutor,ExecutionContext,ExecutionError};
use crate::effects::helpers::resolve_objects_for_effect;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub use ironsmith_core::MergePermanentsEffect;
impl EffectExecutor for MergePermanentsEffect {
 fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{self.execute_with_outputs(game,ctx).map(CompletedEffectOutputs::into_outcome)}
 fn execute_with_outputs(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<CompletedEffectOutputs,ExecutionError>{
  crate::effects::composition::execute_transaction(game,ctx,||CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),|game,ctx|{
   let incoming=resolve_objects_for_effect(game,ctx,&ChooseSpec::Object(self.incoming.clone()).with_count(crate::effect::ChoiceCount::exactly(1)))?.first().copied();
   if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)))}
   let Some(incoming)=incoming else{return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid()))};
   let mut host_filter=self.host.clone();
   // The second operand's "another" is relative to the incoming operand,
   // independently of which object owns the resolving ability.
   if host_filter.other {
    host_filter.other=false;let tag=crate::tag::TagKey::from("__merge_incoming__");
    if let Some(snapshot)=crate::snapshot::ObjectSnapshot::from_object_id(game,incoming){ctx.set_tagged_objects(tag.clone(),vec![snapshot]);}
    host_filter.tagged_constraints.push(crate::target::TaggedObjectConstraint{tag,relation:crate::target::TaggedOpbjectRelation::IsNotTaggedObject});
   }
   let host=resolve_objects_for_effect(game,ctx,&ChooseSpec::Object(host_filter).with_count(crate::effect::ChoiceCount::exactly(1)))?.first().copied();
   if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)))}
   let Some(host)=host else{return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid()))};
   if incoming==host||game.is_phased_out(incoming)||game.is_phased_out(host)||!game.object(incoming).is_some_and(|o|o.zone==Zone::Battlefield)||!game.object(host).is_some_and(|o|o.zone==Zone::Battlefield){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid()))}
   let incoming_on_top=if self.choose_top {
    let choices=vec![crate::decisions::SelectableOption::new(0,"Put the incoming permanent on top").with_object(incoming),crate::decisions::SelectableOption::new(1,"Keep the host permanent on top").with_object(host)];
    let decision=crate::decisions::SelectOptionsContext::new(ctx.controller,Some(ctx.source),"Choose the order of the merged permanent",choices,1,1);
    ctx.decision_maker.decide_options(game,&decision).first().copied().unwrap_or(0)==0
   }else{false};
   if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)))}
   let attachments=game.object(incoming).unwrap().attachments.clone();
   let Some(merged)=game.begin_battlefield_merge(incoming,host,incoming_on_top)else{return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::target_invalid()))};
   let mut children=Vec::new();
   if self.transfer_counters {
    children.push(crate::effects::MoveAllCountersEffect::new(ChooseSpec::SpecificObject(incoming),ChooseSpec::SpecificObject(merged)).execute_child_with_outputs(game,ctx)?);
    if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)))}
   }
   if self.transfer_attachments {for attachment in attachments {
    children.push(crate::effects::AttachObjectsEffect::new(ChooseSpec::SpecificObject(attachment),ChooseSpec::SpecificObject(merged)).execute_child_with_outputs(game,ctx)?);
    if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)))}
   }}
   game.finish_battlefield_merge(incoming);
   let outcome=EffectOutcome::aggregate_with_primary_result(EffectOutcome::count(1),children.iter().map(|child|child.outcome.clone()));let mut output=CompletedEffectOutputs::aggregate_only(outcome);for child in children{output.retain_owned_child(child);}Ok(output)
  })
 }
}

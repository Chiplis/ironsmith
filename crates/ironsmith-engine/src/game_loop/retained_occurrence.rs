//! Normal announcements for explicitly selected, retained ability occurrences.
//!
//! Matchers/selection own kind-specific legality. These adapters retain the
//! acquired occurrence and use existing targeting, modes, notification and
//! stack consumers; they do not execute nonmana ability effects directly.
use super::*;
use crate::ability::{AbilityKind, ActivatedAbility, ActivatedAbilityRuntimeExt};
use crate::effects::{PublishedEffectOutputs, ExecutionError};

#[derive(Debug, Clone)]
pub(crate) struct RetainedActivatedOccurrence {
    pub source: ObjectId,
    pub ability_index: usize,
    pub ability_origin: crate::continuous::AbilityOrigin,
    pub activator: PlayerId,
    pub ability: ActivatedAbility,
    pub source_snapshot: ObjectSnapshot,
}
impl RetainedActivatedOccurrence {
    /// Capture CURRENT characteristics after the enclosing instruction has
    /// chosen the occurrence. An old ETB snapshot is not a source ability list.
    pub(crate) fn capture(game: &GameState, source: ObjectId, ability_index: usize, activator: PlayerId) -> Result<Self, ExecutionError> {
        let object=game.object(source).ok_or(ExecutionError::ObjectNotFound(source))?;
        let snapshot=ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game)?;
        let ability=game.current_ability(source,ability_index).ok_or_else(||ExecutionError::IncompleteEvidence("selected activation occurrence is unavailable".into()))?;
        let AbilityKind::Activated(ability)=ability.kind else { return Err(ExecutionError::InvalidTarget); };
        let origin=snapshot.ability_origins.as_ref().and_then(|origins|origins.get(ability_index)).cloned().ok_or_else(||ExecutionError::IncompleteEvidence("selected activation lacks acquisition identity".into()))?;
        Ok(Self{source,ability_index,ability_origin:origin,activator,ability,source_snapshot:snapshot})
    }
}

/// Cause exact prevalidated triggered occurrences to trigger. Their ordinary
/// targets/modes are chosen by the normal queue after this resolution finishes.
/// Projection collectors inside check_triggers must append to that original
/// batch instead, so trigger duplication is applied once, not twice.
pub(crate) fn defer_forced_trigger_occurrences(game:&mut GameState,entries:Vec<TriggeredAbilityEntry>) {
    let entries=crate::triggers::check::with_additional_trigger_copies(game,entries);
    game.defer_trigger_entries(entries);
}

/// Announce a real activation while ALL activation costs are waived. Optional
/// choice, ability restrictions and occurrence matching belong to the consumer.
/// Suspension rolls back to the enclosing effect checkpoint; replay supplies
/// its retained decision route and publishes the completed action only once.
pub(crate) fn announce_retained_activated_occurrence_without_costs(
    game:&mut GameState,ctx:&mut ExecutionContext<'_>,occurrence:RetainedActivatedOccurrence,
)->Result<Option<PublishedEffectOutputs>,GameLoopError>{
    crate::effects::composition::execute_transaction(game,ctx,||None,move |game,ctx|announce_inner(game,ctx,&occurrence))
}
fn announce_inner(game:&mut GameState,ctx:&mut ExecutionContext<'_>,occurrence:&RetainedActivatedOccurrence)->Result<Option<PublishedEffectOutputs>,GameLoopError>{
    let source=occurrence.source;let actor=occurrence.activator;let index=occurrence.ability_index;
    let current=game.current_characteristics(source).ok_or_else(||GameLoopError::ExecutionFailed(ExecutionError::ObjectNotFound(source)))?;
    if current.abilities.origin(index)!=Some(&occurrence.ability_origin){return Err(GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence("selected activation occurrence changed before announcement".into())));}
    if occurrence.ability.is_runtime_mana_ability(game,source,actor){return announce_mana(game,ctx,occurrence);}
    let mut state=PriorityLoopState::new(game.players_in_game());let mut queue=TriggerQueue::new();state.save_checkpoint(game);
    let provenance=game.provenance_graph_mut().alloc_root(crate::provenance::ProvenanceNodeKind::EffectExecution{source,controller:actor});
    state.activation_output_receiver=Some(NativeCastOutputReceiver::new(source,actor,provenance));
    game.begin_exhaust_activation(source,index);
    game.begin_library_top_announcement(crate::game_state::LibraryTopAnnouncement::Activation(provenance));
    let mut effects=occurrence.ability.effects.clone();
    if effects.all_effects().is_empty()&&!occurrence.ability.mana_symbols().is_empty(){effects.push_segment(crate::resolution::ResolutionSegment::from_effects(vec![Effect::add_mana(occurrence.ability.mana_symbols().to_vec())]));}
    let requirements=extract_target_requirements_from_program_with_modes(game,&effects,actor,Some(source),None);
    let original_cost=&occurrence.ability.mana_cost;
    let (has_x,has_tap)=cost_flags(original_cost);
    let mut tags=std::collections::HashMap::new();game.insert_granting_source_tag(source,index,&mut tags);
    let mut pending=PendingActivation::new(source,index,Some(occurrence.ability_origin.clone()),actor,provenance,
        ActivationStage::ChoosingModes,effects,requirements,None,Vec::new(),occurrence.ability.payment_reason(game,source,actor),Vec::new(),Vec::new(),tags,0,
        occurrence.ability.max_activations_per_turn().is_some(),occurrence.ability.is_loyalty_ability(),occurrence.source_snapshot.stable_id,occurrence.source_snapshot.clone(),occurrence.source_snapshot.name.clone(),
        has_x.then_some(0),has_x,has_tap,occurrence.ability.mana_usage_restrictions.clone(),game.chosen_creature_type(source),Vec::new());
    pending.effect_driven=true;pending.optional_reduction_ready=true;pending.mana_ability_window_closed=true;
    let mut facts=crate::decision::ActivationCostAbility::of(game,actor,source,&occurrence.ability);facts.ability_index=Some(index);
    pending.announced_cost=Some(AnnouncedActivationCost{ability:occurrence.ability.clone(),facts});
    let mut dm=&mut *ctx.decision_maker;
    let mut progress=check_activation_modes_or_continue(game,&mut queue,&mut state,pending,&mut dm)?;
    loop{match progress{
        GameProgress::NeedsDecisionCtx(context)=>{
            let next=apply_decision_context_with_dm(game,&mut queue,&mut state,&context,&mut dm);
            if dm.awaiting_choice(){state.rollback_action(game);return Ok(None);}
            progress=match next{
                Ok(value)=>value,
                Err(GameLoopError::ActionCancelled(_))=>{
                    state.rollback_action(game);
                    return Ok(None);
                }
                Err(error)=>{state.rollback_action(game);return Err(error);}
            };
        }
        GameProgress::Continue if state.pending_activation.is_none()=>{
            // A normal native target/mode cancellation rolls the announcement
            // checkpoint back and clears its receipt owner. No activation was
            // completed, so the enclosing instruction has no success receipt.
            if state.activation_output_receiver.is_none(){
                if state.checkpoint.is_some(){return Err(GameLoopError::InvalidState("effect activation lost receipt owner before rollback".into()));}
                return Ok(None);
            }
            let outputs=state.activation_output_receiver.as_mut().ok_or_else(||GameLoopError::InvalidState("effect activation lost native receipt owner".into()))?.take_outputs(source,actor,provenance)?;
            queue.defer_into_game(game);return Ok(Some(outputs));
        }
        other=>{state.rollback_action(game);return Err(GameLoopError::InvalidState(format!("effect activation advanced outer priority: {other:?}")));}
    }}
}
fn cost_flags(cost:&crate::cost::TotalCost)->(bool,bool){
    match cost.kind(){
        ironsmith_core::TotalCostKind::All(costs)=>costs.iter().fold((false,false),|(x,tap),cost|(x||cost.mana_cost_ref().is_some_and(|mana|mana.has_x())||super::priority_cast::cost_references_x(cost),tap||cost.requires_tap())),
        ironsmith_core::TotalCostKind::OneOf(branches)=>branches.iter().map(cost_flags).fold((false,false),|(x,tap),(next_x,next_tap)|(x||next_x,tap||next_tap)),
    }
}
fn announce_mana(game:&mut GameState,ctx:&mut ExecutionContext<'_>,occurrence:&RetainedActivatedOccurrence)->Result<Option<PublishedEffectOutputs>,GameLoopError>{
    let mut dm=&mut *ctx.decision_maker;let mut queue=TriggerQueue::new();
    let completed=crate::special_actions::perform_mana_ability_without_activation_costs(game,occurrence.activator,occurrence.source,occurrence.ability_index,&mut dm).map_err(|error|match error{
        crate::special_actions::ActionError::ExecutionFailure{error,..}=>GameLoopError::ExecutionFailed(error),
        other=>GameLoopError::InvalidState(format!("effect-driven mana activation: {other}")),
    })?;
    if dm.awaiting_choice(){return Ok(None);}
    let notification=completed.activation_notification.ok_or_else(||GameLoopError::ExecutionFailed(ExecutionError::IncompleteEvidence("effect-driven mana activation lacks its native notification".into())))?;
    let mut outputs=crate::effects::CompletedEffectOutputs::aggregate_only(crate::effect::EffectOutcome::resolved());outputs.retain_published_children(completed.outputs);
    queue_triggers_for_events(game,&mut queue,completed.events)?;
    let notifications=queue_prepared_activation_notification_with_outputs(game,&mut queue,&mut dm,notification)?;
    if dm.awaiting_choice(){return Ok(None);}
    outputs.retain_published_children(notifications);queue.defer_into_game(game);
    Ok(Some(PublishedEffectOutputs::retain(outputs)))
}

#[cfg(test)]
mod tests{
 use super::*;
 use crate::{Ability,CardDefinitionBuilder,CardId,TotalCost};
 use crate::decision::SelectFirstDecisionMaker;
 fn fixture(ability:Ability)->(GameState,ObjectId){
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);game.turn.active_player=PlayerId(0);game.turn.priority_player=Some(PlayerId(1));
  let definition=CardDefinitionBuilder::new(CardId::new(),"Selected acquired ability").card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1,1)).with_ability(ability).build();
  let source=game.create_object_from_definition(&definition,PlayerId(0),Zone::Battlefield);(game,source)
 }
 fn impossible_price()->TotalCost{TotalCost::from_costs(vec![crate::costs::Cost::tap(),crate::costs::Cost::untap(),crate::costs::Cost::sacrifice_self(),crate::costs::Cost::discard(2,None),crate::costs::Cost::mana(crate::mana::ManaCost::from_pips(vec![vec![crate::mana::ManaSymbol::Generic(9)]]))])}
 #[test]
 fn waived_activation_uses_ordinary_stack_and_notification_without_paying_any_cost_or_granting_priority(){
  let(mut game,source)=fixture(Ability::activated(impossible_price(),vec![Effect::gain_life(2)]));let before=game.turn.priority_player;
  let occurrence=RetainedActivatedOccurrence::capture(&game,source,0,PlayerId(0)).unwrap();let mut dm=SelectFirstDecisionMaker;let mut ctx=ExecutionContext::new(source,PlayerId(0),&mut dm);
  let output=announce_retained_activated_occurrence_without_costs(&mut game,&mut ctx,occurrence).unwrap().expect("completed native activation receipt");drop(output);
  assert!(game.object(source).is_some());assert!(!game.is_tapped(source));assert_eq!(game.player(PlayerId(0)).unwrap().hand.len(),0);assert_eq!(game.player(PlayerId(0)).unwrap().mana_pool.total(),0);assert_eq!(game.player(PlayerId(0)).unwrap().life,20);
  assert_eq!(game.stack.len(),1);assert!(game.stack[0].is_ability);assert_eq!(game.stack[0].ability_index,Some(0));assert_eq!(game.turn.priority_player,before);
  assert_eq!(game.turn_store.turn_history.event_records.iter().filter(|record|record.event.downcast::<crate::events::AbilityActivatedEvent>().is_some()).count(),1);
  resolve_stack_entry_with(&mut game,&mut SelectFirstDecisionMaker).unwrap();assert_eq!(game.player(PlayerId(0)).unwrap().life,22);
 }
 #[test]
 fn waived_mana_activation_is_immediate_and_reports_a_real_mana_activation(){
  let ability=Ability::mana(impossible_price(),vec![crate::mana::ManaSymbol::Green]);
  let(mut game,source)=fixture(ability);let occurrence=RetainedActivatedOccurrence::capture(&game,source,0,PlayerId(0)).unwrap();let mut dm=SelectFirstDecisionMaker;let mut ctx=ExecutionContext::new(source,PlayerId(0),&mut dm);
  assert!(announce_retained_activated_occurrence_without_costs(&mut game,&mut ctx,occurrence).unwrap().is_some());assert!(game.stack.is_empty());assert!(game.object(source).is_some());assert!(!game.is_tapped(source));assert_eq!(game.player(PlayerId(0)).unwrap().mana_pool.total(),1);
  assert!(game.turn_store.turn_history.event_records.iter().any(|record|record.event.downcast::<crate::events::AbilityActivatedEvent>().is_some_and(|event|event.is_mana_ability&&event.activator==PlayerId(0))));
 }
}

#[cfg(test)]
mod forced_trigger_tests{
 use super::*;
 #[test]
 fn externally_caused_occurrence_uses_the_normal_trigger_queue_and_targets(){
  let a=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let definition=crate::CardDefinitionBuilder::new(crate::CardId::new(),"External ability source").card_types(vec![CardType::Artifact])
   .with_ability(crate::Ability::triggered(crate::triggers::Trigger::custom("externally-triggered:resonance","Externally triggered ability named resonance".into()),vec![Effect::deal_damage(2,ChooseSpec::AnyTarget)])).build();
  let source=game.create_object_from_definition(&definition,a,Zone::Battlefield);let snapshot=ObjectSnapshot::try_from_object_with_calculated_characteristics(game.object(source).unwrap(),&game).unwrap();
  let acquired=game.current_ability(source,0).unwrap();let AbilityKind::Triggered(ability)=acquired.kind else{unreachable!()};
  let event=TriggerEvent::new(crate::events::LifeGainEvent::new(a,0),Default::default());assert!(crate::triggers::check_triggers(&game,&event).is_empty());
  let identity=crate::triggers::compute_trigger_identity(&ability);
  defer_forced_trigger_occurrences(&mut game,vec![TriggeredAbilityEntry{source,controller:a,x_value:None,event_value_amount:None,ability,triggering_event:event,source_stable_id:snapshot.stable_id,source_name:snapshot.name.clone(),source_snapshot:Some(snapshot),tagged_objects:Default::default(),source_kind:crate::triggers::TriggeredAbilitySourceKind::Object,trigger_identity:identity,linked_exile_owner:None,source_number_owner:None}]);
  assert!(game.stack.is_empty());let mut queue=TriggerQueue::new();drain_pending_trigger_events_with_dm(&mut game,&mut queue,&mut crate::decision::SelectFirstDecisionMaker).unwrap();put_triggers_on_stack_with_dm(&mut game,&mut queue,&mut crate::decision::SelectFirstDecisionMaker).unwrap();assert_eq!(game.stack.len(),1);assert_eq!(game.stack[0].targets.len(),1);assert_eq!(game.stack[0].trigger_identity,Some(identity));
  assert!(!game.turn_store.turn_history.event_records.iter().any(|record|record.event.downcast::<crate::events::LifeGainEvent>().is_some()),"invocation context isn't a fabricated game action");
 }
}

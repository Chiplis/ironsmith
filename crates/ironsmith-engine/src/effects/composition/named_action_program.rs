//! A replacement envelope around an ordinary independently parsed child program.
use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, EffectExecutor, ExecutionContext, ExecutionError, SequenceEffect};
use crate::events::processing::{TraitEventProcessingState, TraitEventResult, process_trait_event_with_execution_context_and_state};
use crate::events::{Event, other::NamedActionProposalEvent};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;

pub type NamedActionProgramEffect = ironsmith_core::NamedActionProgramEffect<Effect>;
impl EffectExecutor for NamedActionProgramEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> { Box::new(self.clone()) }
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) { for effect in &self.effects { visitor(effect); } }
    fn execute(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx).map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(&self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::execute_transaction(game, ctx, || CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)), |game, ctx| {
            if self.completion == ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment {
                let owner = self.effects.first().filter(|_| self.effects.len() == 1).map(|effect| {
                    let mut owner = effect;
                    while let Some(child) = owner.transparent_child_effect() { owner = child; }
                    owner
                });
                if !owner.and_then(|owner| owner.downcast_ref::<crate::effects::MayEffect>()).is_some_and(|payment| payment.pay_as_cost) {
                    return Err(ExecutionError::InternalError("named paid operation lost its owning optional payment".into()));
                }
            }
            let player = crate::effects::helpers::resolve_player_filter(game, &self.player, ctx)?;
            let snapshot = ObjectSnapshot::from_object_id(game, ctx.source).or_else(|| ctx.source_snapshot.clone())
                .ok_or_else(|| ExecutionError::IncompleteEvidence("named operation has no performer snapshot".into()))?;
            let proposed_objects=if self.completion.captures_proposed_objects() {
                let spec=self.objects.as_ref().ok_or_else(||ExecutionError::IncompleteEvidence("proposed operation lacks its declared input collection".into()))?;
                let ids=crate::effects::helpers::resolve_objects_for_effect(game,ctx,spec)?;
                if ctx.decision_maker.awaiting_choice(){return Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));}
                ids.into_iter().filter_map(|id|game.object(id)).map(|object|ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game)).collect::<Result<Vec<_>,_>>()?
            }else{Vec::new()};
            let label = self.label.trim().to_ascii_lowercase();
            let proposal = Event::new_with_provenance(NamedActionProposalEvent {
                label: label.clone(), player, source: ctx.source, snapshot, objects:proposed_objects, original_program:self.effects.clone(), first_then_other_mode:None, cause: ctx.cause.clone(),
            }, ctx.provenance);
            let processed = process_trait_event_with_execution_context_and_state(game, proposal, ctx, &mut TraitEventProcessingState::default())?;
            // The commit owner supplies exact completion bindings. An empty
            // exported object set is evidence, distinct from an operandless
            // action whose subject is its performer.
            let completed_bindings = std::cell::RefCell::new(None::<(ObjectSnapshot, Vec<ObjectSnapshot>)>);
            crate::effects::replacement::execute_event_expansion_with_outputs(game, ctx, processed, |game, ctx, original| match original {
                TraitEventResult::Replaced { effects, source, controller, context, .. } => {
                    let snapshots: Vec<_> = context.event.inner().snapshot().cloned().into_iter().collect();
                    let objects=context.event.inner().as_any().downcast_ref::<NamedActionProposalEvent>().map(|proposal|proposal.objects.clone()).unwrap_or_default();
                    let tags = vec![("it".into(), snapshots.clone()), ("__it__".into(), snapshots.clone()), ("__named_action_source".into(), snapshots), ("__proposed_action_objects".into(),objects)];
                    // "catalyze twice instead": performing the replaced
                    // action again is a new proposal of the same original
                    // program (CR 614.5 keeps this replacement off it), not a
                    // bare completion receipt.
                    let effects: Vec<Effect> = effects.iter().map(|effect| match effect.downcast_ref::<crate::effects::PerformNamedActionEffect>() {
                        Some(repeat) if repeat.label.trim().eq_ignore_ascii_case(&label) && repeat.performer.is_none()
                            && repeat.objects.is_none() && repeat.prior_objects.is_none() => Effect::new(self.clone()),
                        _ => effect.clone(),
                    }).collect();
                    let replacement = crate::effects::replacement::execute_replacement_payload_with_outputs(
                        game, ctx, &effects, source, controller, &context, None, None, tags)?;
                    let aggregate = EffectOutcome::aggregate_replacement_outcomes(EffectOutcome::replaced(), [replacement.outcome.clone()]);
                    Ok(replacement.project_aggregate(aggregate))
                }
                TraitEventResult::Prevented => Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::prevented())),
                TraitEventResult::Proceed(event) | TraitEventResult::Modified(event) => {
                    let proposal = event.inner().as_any().downcast_ref::<NamedActionProposalEvent>()
                        .filter(|proposal| proposal.label == label)
                        .ok_or_else(|| ExecutionError::InternalError("named replacement changed its operation identity".into()))?;
                    let previous_token_scope = ctx.named_action_token_scope.replace((label.clone(), proposal.player));
                    let previous_created_objects = ctx.named_action_created_objects.replace(Vec::new());
                    ctx.set_tagged_objects("__named_action_created_objects", Vec::new());
                    let previous_controller = ctx.controller;
                    let previous_snapshot = ctx.source_snapshot.clone();
                    ctx.controller = proposal.player;
                    ctx.source_snapshot = Some(proposal.snapshot.clone());
                    let proposed_tag=crate::tag::TagKey::from("__proposed_action_objects");
                    let previous_proposed=if self.completion.captures_proposed_objects() {
                        Some(ctx.tagged_objects.insert(proposed_tag.clone(),proposal.objects.clone()))
                    }else{None};
                    let result = (|| {
                        let planned = match proposal.first_then_other_mode {
                            Some(index) => super::named_action_mode_plan::first_then_other_mode(&proposal.original_program, index)?,
                            None => proposal.original_program.clone(),
                        };
                        let effects = operation_payment_alternatives(game, ctx, &label, proposal.player, &planned)?;
                        let mut body = SequenceEffect::new(effects).execute_with_outputs(game, ctx)?;
                        // Completion belongs to this operation boundary, not
                        // to material changes or a nested optional instruction.
                        if ctx.decision_maker.awaiting_choice() || ctx.resolution_stopped() { return Ok(body); }
                        // Only an explicitly creation-qualified operation
                        // exports this packet as its instruction result set.
                        // Payment acceptance and other action receipts keep
                        // their independent original meanings.
                        export_created_operation_result(&mut body.outcome, self.objects.as_ref(),
                            ctx.named_action_created_objects.as_deref().unwrap_or(&[]));
                        let complete = match self.completion {
                            ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted | ironsmith_core::effect::NamedActionProgramCompletion::ProposedObjects => true,
                            ironsmith_core::effect::NamedActionProgramCompletion::ProposedObjectsWithAvailableChoices => {
                                // Read only the authored child-program result. A
                                // replacement's incidental choices are observations,
                                // never evidence that this operation was possible.
                                let available = body.outcome.instruction_result().execution_facts().iter()
                                    .filter_map(|fact| match fact {
                                        crate::effect::ExecutionFact::ChoiceAvailability { available } => Some(*available),
                                        _ => None,
                                    }).collect::<Vec<_>>();
                                !available.is_empty() && available.into_iter().all(|available| available)
                            }
                            ironsmith_core::effect::NamedActionProgramCompletion::NonemptyObjects => {
                                let objects = self.objects.as_ref().ok_or_else(|| ExecutionError::InternalError(
                                    "object-qualified operation has no declared objects".into()))?;
                                match objects.base() {
                                    crate::target::ChooseSpec::Tagged(tag) => !ctx.get_tagged_all(tag)
                                        .ok_or_else(|| ExecutionError::IncompleteEvidence(
                                            "object-qualified operation lost its captured object packet".into()))?.is_empty(),
                                    _ => !crate::effects::helpers::resolve_objects_from_spec(game, objects, ctx)?.is_empty(),
                                }
                            }

                            ironsmith_core::effect::NamedActionProgramCompletion::AcceptedPayment => {
                                body.outcome.instruction_result().execution_facts().contains(&crate::effect::ExecutionFact::Accepted)
                            }
                        };
                        if !complete { return Ok(body); }
                        ctx.set_tagged_objects("__named_action_created_objects", ctx.named_action_created_objects.clone().unwrap_or_default());
                        let completion_objects=if self.completion.captures_proposed_objects() {
                            Some(crate::target::ChooseSpec::Tagged(proposed_tag.clone()))
                        }else{self.objects.clone()};
                        let completed = crate::effects::PerformNamedActionEffect { label: label.clone(), performer: None, objects: completion_objects, prior_objects: None }
                            .execute_with_outputs(game, ctx)?;
                        if ctx.decision_maker.awaiting_choice() { return Ok(body.append_batch_completion_outputs(completed)); }
                        let receipt = completed.outcome.events.iter().rev()
                            .filter_map(|event| event.downcast::<crate::events::NamedActionEvent>())
                            .find(|receipt| receipt.label == label && receipt.source == proposal.source);
                        let (source, subjects) = if let Some(receipt) = receipt {
                            let subjects = if receipt.objects.is_empty() && self.objects.is_none() {
                                vec![receipt.snapshot.clone()]
                            } else { receipt.objects.clone() };
                            (receipt.snapshot.clone(), subjects)
                        } else if self.objects.is_some()
                            && completed.outcome.status.is_success()
                            && completed.outcome.value.as_count() == Some(0) {
                            // The completion owner suppresses an empty affected-
                            // object receipt; it did resolve an exact empty set.
                            (proposal.snapshot.clone(), Vec::new())
                        } else {
                            return Err(ExecutionError::IncompleteEvidence("named operation lost its completion source or subject receipt".into()));
                        };
                        *completed_bindings.borrow_mut() = Some((source, subjects));
                        let mut output=if self.completion.captures_proposed_objects() {
                            let count=proposal.objects.len().min(i32::MAX as usize)as i32;
                            CompletedEffectOutputs::from_children([body,completed],|outcomes|
                                EffectOutcome::aggregate_with_primary_result(EffectOutcome::count(count),outcomes))
                        }else{body.append_batch_completion_outputs(completed)};
                        export_created_operation_result(&mut output.outcome,self.objects.as_ref(),
                            ctx.named_action_created_objects.as_deref().unwrap_or(&[]));
                        Ok(output)
                    })();
                    if let Some(previous)=previous_proposed {match previous {
                        Some(previous)=>{ctx.tagged_objects.insert(proposed_tag,previous);},
                        None=>{ctx.tagged_objects.remove(&proposed_tag);},
                    }}
                    ctx.controller = previous_controller;
                    ctx.source_snapshot = previous_snapshot;
                    ctx.named_action_token_scope = previous_token_scope;
                    ctx.named_action_created_objects = previous_created_objects;
                    if let Some(outer) = &ctx.named_action_created_objects {
                        ctx.set_tagged_objects("__named_action_created_objects", outer.clone());
                    }
                    result
                }
                TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                    if ctx.decision_maker.awaiting_choice() { Ok(CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0))) }
                    else { Err(ExecutionError::InternalError("named operation suspended without a decision".into())) }
                }
                TraitEventResult::Expanded { .. } => unreachable!("expansion owner removes wrappers"),
            }, |_game, context, _original_receipt| {
                let proposal = context.event.inner().as_any().downcast_ref::<NamedActionProposalEvent>()
                    .filter(|proposal| proposal.label == label)
                    .ok_or_else(|| ExecutionError::IncompleteEvidence("named action addition lost its captured proposal".into()))?;
                let (source, subjects) = completed_bindings.borrow().clone()
                    .unwrap_or_else(|| (proposal.snapshot.clone(), vec![proposal.snapshot.clone()]));
                if source.object_id != proposal.source {
                    return Err(ExecutionError::IncompleteEvidence("named action addition lost its exact performer incarnation".into()));
                }
                Ok(crate::effects::replacement::ReplacementProgramBindings {
                    targets: None,
                    object_tags: vec![("it".into(), subjects.clone()), ("__it__".into(), subjects),
                        ("__named_action_source".into(), vec![source])],
                })
            })
        })
    }
}

/// Offer source-declared prices only at this operation's owning optional
/// payment. Existing modal-cost admission/payment owns affordability, atomicity
/// and selected action receipts; no separate payment engine is introduced.
fn operation_payment_alternatives(game: &GameState, ctx: &ExecutionContext, label: &str, player: crate::ids::PlayerId, effects: &[Effect]) -> Result<Vec<Effect>, ExecutionError> {
    use crate::filter::PlayerFilterExt as _;
    let view = crate::derived_view::DerivedGameView::new(game);
    let mut alternatives = Vec::new();
    for source in game.battlefield.iter().chain(game.command_zone.iter()).copied() {
        if game.is_phased_out(source) { continue; }
        let Some(object) = game.object(source) else { continue; };
        let Some(abilities) = view.static_abilities_rc(source) else { continue; };
        for ability in abilities.iter() {
            if !ability.is_active(game, source) { continue; }
            let Some(model) = ability.compiled_model() else { continue; };
            let payload = match &model.payload { ironsmith_core::StaticAbilityPayload::Conditional { ability, .. } => &ability.payload, payload => payload };
            let ironsmith_core::StaticAbilityPayload::NamedActionPaymentAlternative { label: required, player: performer, effects, display } = payload else { continue; };
            let filter_ctx = game.filter_context_for(game.controller_of(object), Some(source));
            if !required.eq_ignore_ascii_case(label) || !performer.matches_player(player, &filter_ctx) { continue; }
            let program = Effect::new(crate::effects::SequenceEffect::new(effects.clone()));
            alternatives.push(crate::effect::EffectMode::new(display.clone(), vec![Effect::new(crate::effects::ExecuteWithSourceEffect::new(crate::target::ChooseSpec::SpecificObject(source), program))]));
        }
    }
    if alternatives.is_empty() { return Ok(effects.to_vec()); }
    let mut owners = 0;
    let result = effects.iter().map(|effect| bind_operation_payment(effect, &alternatives, player, &mut owners)).collect::<Vec<_>>();
    if owners != 1 { return Err(ExecutionError::InternalError("a resolving operation price alternative requires exactly one owning optional cost".into())); }
    let _ = ctx;
    Ok(result)
}

fn bind_operation_payment(effect: &Effect, alternatives: &[crate::effect::EffectMode], player: crate::ids::PlayerId, owners: &mut usize) -> Effect {
    if let Some(wrapper) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Effect::new(crate::effects::WithIdEffect::new(wrapper.id, bind_operation_payment(&wrapper.effect, alternatives, player, owners)));
    }
    if let Some(wrapper) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Effect::new(wrapper.with_effect(bind_operation_payment(&wrapper.effect, alternatives, player, owners)));
    }
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut sequence = sequence.clone();
        sequence.effects = sequence.effects.iter().map(|effect| bind_operation_payment(effect, alternatives, player, owners)).collect();
        return Effect::new(sequence);
    }
    if let Some(payment) = effect.downcast_ref::<crate::effects::MayEffect>().filter(|payment| payment.pay_as_cost) {
        *owners += 1;
        let mut modes = vec![crate::effect::EffectMode::new("Pay the original price", payment.effects.clone())];
        modes.extend_from_slice(alternatives);
        let mut choice = crate::effects::ChooseModeEffect::new(modes, crate::effect::Value::Fixed(1), crate::effect::Value::Fixed(1), false);
        choice.chooser = Some(crate::target::PlayerFilter::Specific(player));
        let mut payment = payment.clone();
        payment.effects = vec![Effect::new(choice)];
        return Effect::new(payment);
    }
    // Nested named operations and unrelated branches own their own prices.
    effect.clone()
}

/// The source-authored created-object operand owns its result packet. Neither
/// ordinary named actions nor independent replacement additions may replace it.
fn export_created_operation_result(outcome: &mut EffectOutcome, objects: Option<&crate::target::ChooseSpec>, created: &[ObjectSnapshot]) {
    if !objects.is_some_and(|objects| matches!(objects.base(), crate::target::ChooseSpec::Tagged(tag) if tag.as_str()=="__named_action_created_objects")) { return; }
    // Keep explicit empty memory at an original instruction boundary: generic
    // aggregation may omit an empty snapshot vector from its observations.
    if outcome.instruction_result.is_none() {outcome.instruction_result=Some(Box::new(outcome.clone()));}
    let mut primary=outcome;
    while let Some(ref mut original)=primary.instruction_result { primary=original; }
    primary.execution_facts.retain(|fact| !matches!(fact,
        crate::effect::ExecutionFact::ResultObjects(_) | crate::effect::ExecutionFact::ResultObjectMemory(_)));
    primary.execution_facts.push(crate::effect::ExecutionFact::ResultObjects(created.iter().map(|object|object.object_id).collect()));
    primary.execution_facts.push(crate::effect::ExecutionFact::ResultObjectMemory(created.to_vec()));
}

#[cfg(test)]
mod created_result_contract_tests {
 use super::*;
 use crate::{CardId,PlayerId,Zone};
 use crate::cards::builders::CardDefinitionBuilder;
 use crate::target::{ChooseSpec,ObjectFilter,PlayerFilter};
 const A:PlayerId=PlayerId(0);
 fn token(name:&str)->crate::CardDefinition {
  CardDefinitionBuilder::new(CardId::new(),name).token().card_types(vec![crate::CardType::Creature])
   .power_toughness(crate::card::PowerToughness::fixed(2,2)).build()
 }
 #[test]
 fn original_created_results_exclude_independent_addition_tokens_including_known_empty_original() {
  for count in [0,1] {
   let mut game=GameState::new(vec!["A".into(),"B".into()],20);
   let source=game.create_object_from_definition(&token("Performer"),A,Zone::Battlefield);
   game.effect_store.replacement_effects.add_one_shot_effect(crate::replacement::ReplacementEffect::with_matcher(
    source,A,crate::events::other::WouldNamedActionMatcher {label:"assemble".into(),source_filter:ObjectFilter::specific(source),performer_filter:Some(PlayerFilter::You),object_requirement:None},
    crate::replacement::ReplacementAction::Additionally(vec![Effect::new(crate::effects::CreateTokenEffect::new(token("Independent token"),1,PlayerFilter::You))])));
   let operation=Effect::new(NamedActionProgramEffect {label:"assemble".into(),player:PlayerFilter::You,
    objects:Some(ChooseSpec::Tagged("__named_action_created_objects".into())),completion:ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
    effects:vec![Effect::new(crate::effects::CreateTokenEffect::new(token("Original token"),count,PlayerFilter::You))]});
   let mut context=ExecutionContext::new_default(source,A);
   let outcome=crate::effects::execute_effect(&mut game,&operation,&mut context).unwrap();
   let originals=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.as_ref()=="Original token").collect::<Vec<_>>();
   let additions=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.as_ref()=="Independent token").collect::<Vec<_>>();
   assert_eq!(originals.len(),count as usize); assert_eq!(additions.len(),1);
   assert_eq!(outcome.result_objects(),Some(originals.as_slice()));
   assert_eq!(outcome.result_object_memory().unwrap().iter().map(|snapshot|snapshot.object_id).collect::<Vec<_>>(),originals);
   assert!(outcome.result_objects().unwrap().iter().all(|id|!additions.contains(id)));
  }
 }
 #[test]
 fn explicit_created_export_preserves_acceptance_and_unrelated_action_facts() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&token("Receipt subject"),A,Zone::Battlefield);
  let snapshot=ObjectSnapshot::from_object_id(&game,source).unwrap();
  let original=EffectOutcome::count(7).with_execution_fact(crate::effect::ExecutionFact::Accepted)
   .with_execution_fact(crate::effect::ExecutionFact::ActionObjects{action:crate::effect::PriorEffectAction::Exiled,player:Some(A),objects:vec![snapshot.clone()]})
   .with_result_objects(vec![source]);
  let mut ordinary=original.clone();export_created_operation_result(&mut ordinary,None,&[]);
  assert_eq!(ordinary,original,"ordinary named action receipts must be unchanged");
  let mut projected=EffectOutcome::aggregate_replacement_outcomes(original.clone(),[EffectOutcome::count(9).with_result_objects(vec![source])]);
  export_created_operation_result(&mut projected,Some(&ChooseSpec::Tagged("__named_action_created_objects".into())),&[]);
  assert_eq!(projected.instruction_result().status,original.status);
  assert_eq!(projected.instruction_result().value,original.value);
  assert!(projected.instruction_result().execution_facts.contains(&crate::effect::ExecutionFact::Accepted));
  assert_eq!(projected.result_objects(),Some([].as_slice()));
  assert!(projected.instruction_result().execution_facts.iter().any(|fact|matches!(fact,crate::effect::ExecutionFact::ActionObjects{action:crate::effect::PriorEffectAction::Exiled,..})));
 }
 #[test]
 fn ordinary_named_operation_does_not_acquire_an_empty_creation_result() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&token("Ordinary performer"),A,Zone::Battlefield);
  let operation=Effect::new(NamedActionProgramEffect{label:"assess".into(),player:PlayerFilter::You,objects:None,
   completion:ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,effects:vec![Effect::gain_life(2)]});
  let outcome=crate::effects::execute_effect(&mut game,&operation,&mut ExecutionContext::new_default(source,A)).unwrap();
  assert_eq!(game.player(A).unwrap().life,22);
  assert_eq!(outcome.result_objects(),None);assert_eq!(outcome.result_object_memory(),None);
 }

 #[test]
 fn original_creation_result_excludes_tokens_from_real_as_enters_replacement_prefix() {
  let mut game=GameState::new(vec!["A".into(),"B".into()],20);
  let source=game.create_object_from_definition(&token("Prefix performer"),A,Zone::Battlefield);
  let prefix=Effect::new(crate::effects::CreateTokenEffect::new(token("Entry prefix token"),1,PlayerFilter::You));
  let replacement=crate::static_abilities::CompiledStaticAbility::as_enters_effect_program(
   vec![prefix].into(),"this creature",false,false,None);
  let mut original=token("Original entering token");
  original.abilities.push(crate::Ability::static_ability(crate::static_abilities::StaticAbility::from_model(replacement)));
  let operation=Effect::new(NamedActionProgramEffect {label:"assemble".into(),player:PlayerFilter::You,
   objects:Some(ChooseSpec::Tagged("__named_action_created_objects".into())),completion:ironsmith_core::effect::NamedActionProgramCompletion::ProgramCompleted,
   effects:vec![Effect::new(crate::effects::CreateTokenEffect::new(original,1,PlayerFilter::You))]});
  let outcome=crate::effects::execute_effect(&mut game,&operation,&mut ExecutionContext::new_default(source,A)).unwrap();
  let originals=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.as_ref()=="Original entering token").collect::<Vec<_>>();
  let prefixes=game.battlefield.iter().copied().filter(|id|game.object(*id).unwrap().name.as_ref()=="Entry prefix token").collect::<Vec<_>>();
  assert_eq!(originals.len(),1);assert_eq!(prefixes.len(),1,"the independent replacement prefix must really execute");
  assert_eq!(outcome.result_objects(),Some(originals.as_slice()));
  assert_eq!(outcome.result_object_memory().unwrap().iter().map(|snapshot|snapshot.object_id).collect::<Vec<_>>(),originals);
 }

}

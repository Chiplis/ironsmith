//! Layer3 selection of already compiled optional-fragment alternatives.
//! Selection never parses source text and never changes layer1 copiable values.
use super::*;
pub(crate) type FragmentSpec = ironsmith_core::OptionalTextFragmentsSpec<crate::triggers::Trigger,crate::effect::Effect,crate::costs::Cost,crate::static_abilities::ThisSpellCostCondition>;
fn plan(chars:&CalculatedCharacteristics)->Option<(usize,crate::ability::Ability,crate::static_abilities::CompiledStaticAbility,FragmentSpec)>{
 chars.abilities.iter().enumerate().find_map(|(slot,ability)|{
  let crate::ability::AbilityKind::Static(ability_model)=&ability.kind else{return None};let model=ability_model.canonical_model()?;
  let ironsmith_core::StaticAbilityPayload::OptionalTextFragments(spec)=&model.payload else{return None};
  Some((slot,ability.clone(),model.clone(),(**spec).clone()))
 })
}
pub(crate) fn declared_plan(object:&crate::object::Object)->Option<FragmentSpec>{
 object.abilities.iter().find_map(|ability|{
  let crate::ability::AbilityKind::Static(ability)=&ability.kind else{return None};let model=ability.canonical_model()?;
  match model.payload {ironsmith_core::StaticAbilityPayload::OptionalTextFragments(spec)=>Some(*spec),_=>None}
 })
}
pub(crate) fn has_selected_cast_choice(object:&crate::object::Object)->bool{declared_plan(object).and_then(|spec|spec.casting_payment).is_some_and(|index|object.optional_costs_paid.text_fragment_choice(index).is_some())}
pub(crate) fn current_plan(game:&crate::game_state::GameState,target:crate::ids::ObjectId)->Result<Option<FragmentSpec>,crate::effects::ExecutionError>{
 let Some(chars)=game.try_current_characteristics(target).map_err(crate::effects::ExecutionError::ContinuousDiscovery)?else{return Ok(None)};
 let result=plan(&chars).map(|(_,_,_,spec)|spec);
 if let Some(spec)=&result{spec.validate().map_err(|reason|crate::effects::ExecutionError::IncompleteEvidence(reason.into()))?;}
 Ok(result)
}
fn fail(chars:&mut CalculatedCharacteristics){chars.text_change_error=Some(super::text_changes::TextChangeDomainError::OptionalFragments);}
pub(crate) fn apply_cast_choice(chars:&mut CalculatedCharacteristics,object:&crate::object::Object){
 let Some((_,_,_,spec))=plan(chars)else{return};let Some(index)=spec.casting_payment else{return};
 let count=object.optional_costs_paid.times_paid(index);
 let mask=match object.optional_costs_paid.text_fragment_choice(index){Some(mask)=>mask,None if count==0=>0,None=>{fail(chars);return}};
 if !spec.selection_matches_payments(mask,count){fail(chars);return}
 apply_removed_mask(chars,mask,object);
}
pub(crate) fn apply_removed_mask(chars:&mut CalculatedCharacteristics,mask:u64,object:&crate::object::Object){
 let Some((marker_slot,mut marker_ability,mut model,mut spec))=plan(chars)else{return};
 if spec.validate().is_err()||spec.complete_mask().is_none_or(|all|mask & !all!=0){fail(chars);return}
 let desired=spec.current_removed|mask;
 if desired==spec.current_removed{return}
 let Some(variant)=spec.variant(desired).cloned()else{fail(chars);return};
 let rules=chars.abilities.iter().enumerate().filter(|(slot,_)|chars.abilities.origin(*slot).is_some_and(|origin|origin.is_rules_text())).map(|(slot,ability)|(slot,ability.clone())).collect::<Vec<_>>();
 if rules.len()!=variant.abilities.len()+1 || !rules.iter().any(|(slot,_)|*slot==marker_slot){fail(chars);return}
 spec.current_removed=desired;model.payload=ironsmith_core::StaticAbilityPayload::OptionalTextFragments(Box::new(spec.clone()));
 marker_ability.kind=crate::ability::AbilityKind::Static(crate::static_abilities::StaticAbility::from_model(model));
 let mut labels=chars.ability_labels.to_vec();labels.resize(chars.abilities.len(),String::new());
 let mut replacements=Vec::with_capacity(rules.len());let mut source_index=0;
 for (slot,old) in &rules{
  if *slot==marker_slot{replacements.push(marker_ability.clone());continue}
  let mut replacement=crate::static_abilities::StaticAbilityModelInterpreter::ability_from_model(&variant.abilities[source_index]);
  // Preserve the acquisition's history owner, never its already rewritten body.
  if spec.invariant_slots.contains(&source_index) {
   if let (crate::ability::AbilityKind::Activated(previous),crate::ability::AbilityKind::Activated(selected))=(&old.kind,&mut replacement.kind){selected.effects.activation_definition=previous.effects.activation_definition;}
  }
  labels[*slot]=variant.ability_labels[source_index].clone();replacements.push(replacement);source_index+=1;
 }
 let mut next=0;let result=chars.abilities.try_map_rules_text::<()>(|_|{let replacement=replacements[next].clone();next+=1;Ok(replacement)});debug_assert!(result.is_ok());
 chars.ability_labels=labels.into();if !variant.canonical_text.is_empty(){chars.compiled_card_text=variant.canonical_text.into();}chars.aura_attach_filter=variant.aura_attach_filter;
 chars.spell_effect=Arc::new(crate::snapshot::SpellProgramState::from_option(variant.spell_effect));
 chars.static_abilities=crate::ability::extract_static_abilities(&chars.abilities).into();
 // Active word substitutions preceding this deletion still apply to every
 // retained instruction. They are replayed from typed templates each query,
 // so a temporary substitution is never baked into the lasting deletion.
 let changes=std::mem::take(&mut chars.text_changes);let actual_subtypes=chars.subtypes.clone();
 for change in changes{super::text_changes::apply_text_change(chars,change,object);}
 chars.subtypes=actual_subtypes;
}

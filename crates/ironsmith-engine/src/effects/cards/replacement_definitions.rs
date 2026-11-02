//! Exact source-owned definition operands captured before replacement choices.
use std::sync::Arc;
use crate::{GameState,ObjectId};
use crate::effects::ExecutionError;
#[derive(Debug,Clone)]
pub struct CapturedReplacementDefinition {pub source:ObjectId,pub name:String,pub definition:Arc<crate::cards::CardDefinition>}
#[derive(Debug,Clone,Default)]
pub struct CapturedReplacementDefinitions {entries:Vec<CapturedReplacementDefinition>}
impl CapturedReplacementDefinitions {
 pub fn is_empty(&self)->bool{self.entries.is_empty()}
 pub fn contains_source(&self,source:ObjectId)->bool{self.entries.iter().any(|entry|entry.source==source)}
 pub fn resolve(&self,source:ObjectId,name:&str)->Result<Arc<crate::cards::CardDefinition>,ExecutionError>{
  let mut candidates=self.entries.iter().filter(|entry|entry.source==source&&crate::filter::names_match(&entry.name,name));
  let first=candidates.next().ok_or_else(||ExecutionError::IncompleteEvidence(format!("replacement source {} has no captured supplied definition named {name}",source.0)))?;
  for other in candidates {
   // Runtime effect equality is acquisition identity. Compare complete typed
   // executable models, never display text, for independently retained clones.
   let equal=super::native_definition_equivalence::definitions_equal(&first.definition,&other.definition)
       .map_err(ExecutionError::IncompleteEvidence)?;
   if !equal{return Err(ExecutionError::IncompleteEvidence(format!("replacement source {} has conflicting supplied definitions named {name}",source.0)));}
  }
  Ok(first.definition.clone())
 }
}
pub fn capture_replacement_definitions(game:&GameState)->Result<CapturedReplacementDefinitions,ExecutionError>{
 let mut result=CapturedReplacementDefinitions::default();
 for object in game.objects_map().values() {
  let source=object.id;
  if game.is_phased_out(source){continue;}
  let Some(characteristics)=game.try_current_characteristics(source).map_err(ExecutionError::ContinuousDiscovery)? else{continue;};
  for ability in characteristics.abilities.iter() {
   if !ability.functional_zones.contains(&object.zone){continue;}
   let crate::ability::AbilityKind::Static(rule)=&ability.kind else{continue;};
   if !rule.is_active(game,source){continue;}
   let Some(template)=rule.supplied_replacement_definition()else{continue;};
   // The carrier owns its complete lexical graph. Never peel its scope
   // and rehydrate a selected member from the initiating operation's pool.
   let definition=crate::effects::cards::replacement_template_definition(&template)?;
   result.entries.push(CapturedReplacementDefinition{source,name:definition.card.name.clone(),definition:Arc::new(definition)});
  }
 }
 Ok(result)
}

#[cfg(test)]
mod tests {
 use super::*;
 fn definition()->crate::cards::CardDefinition {
  crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Shared supplied identity")
   .card_types(vec![crate::types::CardType::Sorcery]).with_spell_effect(vec![crate::effect::Effect::draw(1)]).build()
 }
 fn entry(source:ObjectId,definition:crate::cards::CardDefinition)->CapturedReplacementDefinition{
  CapturedReplacementDefinition{source,name:definition.card.name.clone(),definition:Arc::new(definition)}
 }
 #[test]
 fn source_owned_lookup_keeps_independent_same_name_providers_separate(){
  let one=definition();let mut two=one.clone();two.spell_effect=Some(vec![crate::effect::Effect::draw(2)].into());
  let first=ObjectId(100);let second=ObjectId(101);
  let captured=CapturedReplacementDefinitions{entries:vec![entry(first,one.clone()),entry(second,two.clone())]};
  assert_eq!(super::super::native_definition_equivalence::definitions_equal(&captured.resolve(first,&one.card.name).unwrap(),&one),Ok(true));
  assert_eq!(super::super::native_definition_equivalence::definitions_equal(&captured.resolve(second,&two.card.name).unwrap(),&two),Ok(true));
  assert!(captured.resolve(ObjectId(102),&one.card.name).is_err());
 }
 #[test]
 fn independently_constructed_equivalent_carriers_pass_but_conflicting_programs_fail(){
  // A lib test and runtime-catalog's engine dependency are distinct Rust crate
  // instances. Artifact parity belongs in the compiler-runtime integration
  // target; this unit proves native structural equality without that bridge.
  let one=definition();let mut same=one.clone();
  same.spell_effect=Some(vec![crate::effect::Effect::draw(1)].into());
  let source=ObjectId(100);let mut captured=CapturedReplacementDefinitions{entries:vec![entry(source,one.clone()),entry(source,same)]};
  assert!(captured.resolve(source,&one.card.name).is_ok(),"native acquisition identity is not definition inequality");
  let mut changed=one.clone();changed.spell_effect=Some(vec![crate::effect::Effect::draw(2)].into());captured.entries.push(entry(source,changed));
  assert!(captured.resolve(source,&one.card.name).is_err(),"same provider/name with different complete programs is ambiguous");
 }
}

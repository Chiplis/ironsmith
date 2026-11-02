//! Structural equality is for independently compiled immutable programs;
//! runtime Effect::PartialEq intentionally remains acquisition identity.
use crate::ability::Ability;
type ModelTrigger=ironsmith_core::trigger_model::Trigger;
type ModelCost=ironsmith_core::Cost<String>;
type ModelStatic=ironsmith_core::StaticAbility<ModelTrigger,String,ModelCost,crate::static_abilities::ThisSpellCostCondition>;
type ModelAbility=ironsmith_core::Ability<ModelStatic,ModelTrigger,String,ModelCost>;
fn effect(value:crate::effect::Effect)->Result<String,()>{value.serialized_model().map(str::to_owned).ok_or(())}
fn trigger(value:crate::triggers::Trigger)->Result<ModelTrigger,()>{value.compiled_model().cloned().ok_or(())}
fn cost(value:crate::costs::Cost)->Result<ModelCost,()>{value.compiled_model().cloned().ok_or(())?.try_map_effect(effect)}
fn static_ability(value:crate::static_abilities::StaticAbility)->Result<ModelStatic,()>{value.canonical_model().ok_or(())?.try_map(trigger,effect,cost,Ok)}
fn normalize(value:&Ability)->Result<ModelAbility,()>{value.clone().try_map(static_ability,trigger,effect,cost,Ok)}
/// A missing canonical executable model is not evidence that two independently
/// supplied programs have the same meaning. Display strings are never compared.
pub fn ability_programs_equivalent(left:&[Ability],right:&[Ability])->bool{
 if left.len()!=right.len(){return false}
 match(left.iter().map(normalize).collect::<Result<Vec<_>,_>>(),right.iter().map(normalize).collect::<Result<Vec<_>,_>>()){
  (Ok(left),Ok(right))=>{
   fn subtype_metadata(program:&[ModelAbility])->Vec<(ironsmith_core::Subtype,Vec<ironsmith_core::SubtypeFamily>)>{
    use ironsmith_core::tag::TagKeyWalk;
    let mut result=Vec::new();
    for ability in program {ability.for_each_subtype(&mut |subtype| {
     if let ironsmith_core::Subtype::Custom(custom)=subtype {result.push((*subtype,custom.families().collect()));}
    });}
    result
   }
   left==right && subtype_metadata(&left)==subtype_metadata(&right)
  },
  _=>false,
 }
}

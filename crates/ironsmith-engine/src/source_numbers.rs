//! Numeric choices share CR 607 acquisition identity with other linked abilities.
//! This module never reads the linked-exile member store.
use crate::ability::{Ability, AbilityKind};
use crate::continuous::AbilityOrigin;
use crate::effects::ExecutionError;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::linked_exile::LinkedExileOwner;
use crate::snapshot::ObjectSnapshot;
use ironsmith_core::{LinkedExilePair, Value};

pub type NumberChoiceOwner = LinkedExileOwner;
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct NumberChoiceRecord { pub number:u32, pub public_group:u64 }
pub type NumberChoiceMemory = std::collections::HashMap<NumberChoiceOwner,NumberChoiceRecord>;

/// Canonical public proof is not a serialized runtime acquisition. Group
/// ordinals are assigned by completed public choices, never from local IDs.
#[derive(Debug,Clone,PartialEq,Eq)]
#[cfg_attr(feature="serialization",derive(serde::Serialize))]
pub struct NumberChoicePublicRecord {pub group:u64,pub definition:[u8;32],pub pair:u32,pub number:u32}
#[derive(Debug,Clone,PartialEq,Eq)]
#[cfg_attr(feature="serialization",derive(serde::Serialize))]
pub struct NumberChoicePublicBinding {pub slot:usize,pub definition:[u8;32],pub pair:u32,pub group:Option<u64>}
#[derive(Debug,Clone,Default,PartialEq,Eq)]
#[cfg_attr(feature="serialization",derive(serde::Serialize))]
pub struct NumberChoicePublicProof {pub records:Vec<NumberChoicePublicRecord>,pub bindings:Vec<NumberChoicePublicBinding>}

pub fn public_proof(game:&GameState,source:ObjectId,public_identity:bool)->Result<NumberChoicePublicProof,ExecutionError>{
    if !public_identity {return Ok(NumberChoicePublicProof::default());}
    let memory=game.numeric_choice_memory(source);
    let mut records=memory.iter().map(|(owner,record)|NumberChoicePublicRecord{
        group:record.public_group,definition:owner.pair.definition.0,pair:owner.pair.pair,number:record.number,
    }).collect::<Vec<_>>();
    records.sort_by_key(|record|record.group);
    if records.windows(2).any(|records|records[0].group==records[1].group){return Err(missing());}
    let mut bindings=Vec::new();
    if public_identity {
        if let Some(chars)=game.try_current_characteristics(source).map_err(ExecutionError::ContinuousDiscovery)? {
            for (slot,ability) in chars.abilities.iter().enumerate(){
                if ability_requires_number(ability) && ability_pair(ability).is_none(){return Err(missing());}
                if let Some(pair)=ability_pair(ability){
                    let owner=capture(source,Some(pair),chars.abilities.origin(slot)).ok_or_else(missing)?;
                    bindings.push(NumberChoicePublicBinding{slot,definition:pair.definition.0,pair:pair.pair,
                        group:memory.get(&owner).map(|record|record.public_group)});
                }
            }
        }
    }
    Ok(NumberChoicePublicProof{records,bindings})
}
fn missing() -> ExecutionError { ExecutionError::IncompleteEvidence("numeric choice requires its exact linked ability acquisition; native recovery or verified replay required".into()) }

pub(crate) fn contains_value(value:&Value)->bool{
    match value {
        Value::SourceChosenNumber{..}=>true,
        Value::SurfaceHinted{value,..}|Value::Scaled(value,_)|Value::DividedRoundedDown(value,_)|Value::HalfRoundedDown(value)=>contains_value(value),
        Value::Add(a,b)|Value::Min(a,b)=>contains_value(a)||contains_value(b),
        _=>false,
    }
}
pub(crate) fn value_pair(value: &Value) -> Option<LinkedExilePair> {
    match value {
        Value::SourceChosenNumber{pair,..}=>*pair,
        Value::SurfaceHinted{value,..}|Value::Scaled(value,_)|Value::DividedRoundedDown(value,_)|Value::HalfRoundedDown(value)=>value_pair(value),
        Value::Add(a,b)|Value::Min(a,b)=>value_pair(a).or_else(||value_pair(b)),
        _=>None,
    }
}
pub(crate) fn filter_pair(filter: &crate::target::ObjectFilter) -> Option<LinkedExilePair> {
    use crate::filter::Comparison;
    if filter.name_noted_for.is_some(){return filter.noted_name_pair;}
    [&filter.mana_value,&filter.power,&filter.toughness].into_iter().find_map(|comparison|match comparison {
        Some(Comparison::EqualExpr(value)|Comparison::NotEqualExpr(value)|Comparison::LessThanExpr(value)
            |Comparison::LessThanOrEqualExpr(value)|Comparison::GreaterThanExpr(value)|Comparison::GreaterThanOrEqualExpr(value))=>value_pair(value),
        _=>None,
    }).or_else(||filter.any_of.iter().find_map(filter_pair))
}
fn condition_pair(condition: &ironsmith_core::Condition) -> Option<LinkedExilePair> {
    match condition {
        ironsmith_core::Condition::ValueComparison {left,right,..}=>value_pair(left).or_else(||value_pair(right)),
        ironsmith_core::Condition::ValueIsPrime(value)=>value_pair(value),
        ironsmith_core::Condition::And(left,right)|ironsmith_core::Condition::Or(left,right)=>condition_pair(left).or_else(||condition_pair(right)),
        ironsmith_core::Condition::Not(inner)=>condition_pair(inner),
        _=>None,
    }
}
fn condition_requires_number(condition: &ironsmith_core::Condition) -> bool {
    match condition {
        ironsmith_core::Condition::ValueComparison {left,right,..}=>contains_value(left)||contains_value(right),
        ironsmith_core::Condition::ValueIsPrime(value)=>contains_value(value),
        ironsmith_core::Condition::And(left,right)|ironsmith_core::Condition::Or(left,right)=>condition_requires_number(left)||condition_requires_number(right),
        ironsmith_core::Condition::Not(inner)=>condition_requires_number(inner),
        _=>false,
    }
}
pub(crate) fn static_pair(ability:&crate::static_abilities::StaticAbility)->Option<LinkedExilePair>{
    fn pair(model:&crate::static_abilities::CompiledStaticAbility)->Option<LinkedExilePair>{
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram{program,..}=>program.source_number_pair,
            ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt{power,toughness}=>value_pair(power).or_else(||value_pair(toughness)),
            ironsmith_core::StaticAbilityPayload::RuleRestriction{restriction:crate::effect::Restriction::CastSpellsMatching(_,filter),..}=>filter_pair(filter),
            ironsmith_core::StaticAbilityPayload::Conditional{ability,condition,..}=>condition_pair(condition).or_else(||pair(ability)),
            ironsmith_core::StaticAbilityPayload::Anthem(anthem)=>anthem.condition.as_ref().and_then(condition_pair),
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant)=>grant.condition.as_ref().and_then(condition_pair),
            ironsmith_core::StaticAbilityPayload::CostReductionManaCost(reduction)=>filter_pair(&reduction.filter),
            _=>None,
        }
    }
    ability.canonical_model().as_ref().and_then(pair)
}
fn filter_requires_number(filter:&crate::target::ObjectFilter)->bool{
    use crate::filter::Comparison;
    [&filter.mana_value,&filter.power,&filter.toughness].into_iter().any(|comparison|match comparison{
        Some(Comparison::EqualExpr(value)|Comparison::NotEqualExpr(value)|Comparison::LessThanExpr(value)
            |Comparison::LessThanOrEqualExpr(value)|Comparison::GreaterThanExpr(value)|Comparison::GreaterThanOrEqualExpr(value))=>contains_value(value),
        _=>false,
    })||filter.any_of.iter().any(filter_requires_number)
}
fn program_uses_source_number(program: &crate::resolution::ResolutionProgram) -> bool {
    fn chooses(effect: &crate::effect::Effect) -> bool {
        let mut found = effect.downcast_ref::<crate::effects::ChooseNumberEffect>()
            .is_some_and(|choice| choice.source_owned);
        found |= effect.downcast_ref::<crate::effects::PayAnyLifeEffect>()
            .is_some_and(|payment| payment.source_owned);
        found |= effect.downcast_ref::<crate::effects::CreateTokenEffect>()
            .and_then(|create| create.dynamic_power_toughness.as_ref())
            .is_some_and(|(power, toughness)| contains_value(power) || contains_value(toughness));
        effect.visit_child_effects(&mut |child| found |= chooses(child));
        found
    }
    program.all_effects().into_iter().any(chooses)
}
fn ability_requires_number(ability: &Ability) -> bool {
    fn static_requires(model: &crate::static_abilities::CompiledStaticAbility) -> bool {
        match &model.payload {
            ironsmith_core::StaticAbilityPayload::AsEntersEffectProgram { program, .. } =>
                program_uses_source_number(program),
            ironsmith_core::StaticAbilityPayload::CharacteristicDefiningPt { power, toughness } =>
                contains_value(power) || contains_value(toughness),
            ironsmith_core::StaticAbilityPayload::RuleRestriction {
                restriction: crate::effect::Restriction::CastSpellsMatching(_, filter), ..
            } => filter_requires_number(filter),
            ironsmith_core::StaticAbilityPayload::Conditional { ability, condition, .. } => condition_requires_number(condition)||static_requires(ability),
            ironsmith_core::StaticAbilityPayload::Anthem(anthem)=>anthem.condition.as_ref().is_some_and(condition_requires_number),
            ironsmith_core::StaticAbilityPayload::GrantObjectAbilityForFilter(grant)=>grant.condition.as_ref().is_some_and(condition_requires_number),
            _ => false,
        }
    }
    match &ability.kind {
        AbilityKind::Static(ability) => ability.canonical_model().as_ref().is_some_and(static_requires),
        AbilityKind::Triggered(ability) => program_uses_source_number(&ability.effects)
            || ability.trigger.compiled_model().is_some_and(|model| match &model.kind {
                ironsmith_core::TriggerKind::SpellCast { filter: Some(filter), .. }
                | ironsmith_core::TriggerKind::SpellCastQualified { filter: Some(filter), .. } => filter_requires_number(filter),
                _ => false,
            }),
        AbilityKind::Activated(ability) => program_uses_source_number(&ability.effects),
    }
}
pub(crate) fn ability_pair(ability:&Ability)->Option<LinkedExilePair>{
    match &ability.kind {
        AbilityKind::Triggered(ability)=>ability.effects.source_number_pair,
        AbilityKind::Activated(ability)=>ability.effects.source_number_pair,
        AbilityKind::Static(ability)=>static_pair(ability),
    }
}
fn from_snapshot(source:ObjectId,pair:LinkedExilePair,snapshot:&ObjectSnapshot)->Result<NumberChoiceOwner,ExecutionError>{
    if snapshot.object_id!=source{return Err(missing())}
    let origins=snapshot.ability_origins.as_ref().ok_or_else(missing)?;
    if origins.len()!=snapshot.abilities.len(){return Err(missing())}
    let mut owners=Vec::new();
    for (slot,ability) in snapshot.abilities.iter().enumerate(){
        if ability_pair(ability)==Some(pair){
            let owner=NumberChoiceOwner::capture(source,Some(pair),origins.get(slot)).ok_or_else(missing)?;
            if !owners.contains(&owner){owners.push(owner);}
        }
    }
    match owners.as_slice(){[owner]=>Ok(owner.clone()),_=>Err(missing())}
}
/// Explicit admission ownership wins. A fallback may identify only one exact
/// current acquisition; multiple equal copied definitions are not interchangeable.
pub(crate) fn resolve_owner(game:&GameState,source:ObjectId,pair:Option<LinkedExilePair>,
    explicit:Option<&NumberChoiceOwner>,snapshot:Option<&ObjectSnapshot>)->Result<NumberChoiceOwner,ExecutionError>{
    let pair=pair.ok_or_else(missing)?;
    if let Some(owner)=explicit {
        return (owner.host==source && owner.pair==pair).then(||owner.clone()).ok_or_else(missing);
    }
    if let Some(snapshot)=snapshot {return from_snapshot(source,pair,snapshot);}
    let chars=crate::continuous::in_progress_characteristics(game,source)
        .map(Ok).unwrap_or_else(||game.try_current_characteristics(source).map_err(ExecutionError::ContinuousDiscovery)
            .and_then(|chars|chars.ok_or_else(missing)))?;
    let mut owners=Vec::new();
    for (slot,ability) in chars.abilities.iter().enumerate(){
        if ability_pair(ability)==Some(pair){
            let owner=NumberChoiceOwner::capture(source,Some(pair),chars.abilities.origin(slot)).ok_or_else(missing)?;
            if !owners.contains(&owner){owners.push(owner);}
        }
    }
    match owners.as_slice(){[owner]=>Ok(owner.clone()),_=>Err(missing())}
}
pub(crate) fn capture(source:ObjectId,pair:Option<LinkedExilePair>,origin:Option<&AbilityOrigin>)->Option<NumberChoiceOwner>{
    NumberChoiceOwner::capture(source,pair,origin)
}
pub(crate) fn read(game:&GameState,source:ObjectId,pair:Option<LinkedExilePair>,
    explicit:Option<&NumberChoiceOwner>,snapshot:Option<&ObjectSnapshot>)->Result<Option<u32>,ExecutionError>{
    let owner=resolve_owner(game,source,pair,explicit,snapshot)?;
    game.number_for_acquisition(&owner,snapshot)
}

/// Admission-time ownership of a source-noted-information predicate. The
/// acquired trigger identity selects the actual occurrence, not equal text.
pub(crate) fn trigger_memory_owner(game:&GameState,source:ObjectId,pair:Option<LinkedExilePair>,identity:Option<crate::triggers::TriggerIdentity>)->Result<NumberChoiceOwner,ExecutionError>{
    let Some(identity)=identity else{return resolve_owner(game,source,pair,None,None);};
    let pair=pair.ok_or_else(missing)?;
    let snapshot=if let Some(object)=game.object(source){
        ObjectSnapshot::try_from_object_with_calculated_characteristics(object,game)?
    }else{game.source_last_known_snapshot(source).cloned().ok_or_else(missing)?};
    let origins=snapshot.ability_origins.as_ref().ok_or_else(missing)?;
    let mut owners=Vec::new();
    for (slot,ability) in snapshot.abilities.iter().enumerate(){
        let AbilityKind::Triggered(trigger)=&ability.kind else{continue;};
        if trigger.effects.source_number_pair==Some(pair) && crate::triggers::compute_trigger_identity(trigger)==identity{
            let owner=capture(source,Some(pair),origins.get(slot)).ok_or_else(missing)?;
            if !owners.contains(&owner){owners.push(owner);}
        }
    }
    match owners.as_slice(){[owner]=>Ok(owner.clone()),_=>Err(missing())}
}

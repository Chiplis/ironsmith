//! Complete typed captures for prospective entry replacement continuations.
//! Native snapshot serialization is deliberately not used by an owning codec:
//! every ability, program, historical snapshot and duration must be converted.
use crate::ids::{ObjectId, PlayerId};
use crate::zone::Zone;
use crate::object::CounterType;
use crate::color::ColorSet;
use crate::types::{CardType,Subtype,Supertype};
#[cfg(feature="serialization")]
fn present_option<'de,T:serde::Deserialize<'de>,D:serde::Deserializer<'de>>(d:D)->Result<Option<T>,D::Error>{<Option<T> as serde::Deserialize>::deserialize(d)}
#[derive(Debug,Clone)]
#[cfg_attr(feature="serialization",derive(serde::Serialize,serde::Deserialize))]
#[cfg_attr(feature="serialization",serde(deny_unknown_fields))]
pub struct RetainedPreparedEntryChoices<B=crate::static_abilities::StaticAbility,S=crate::snapshot::ObjectSnapshot> {
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub chosen_color: Option<crate::color::Color>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub chosen_basic_land_type: Option<crate::types::Subtype>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub chosen_land_type: Option<crate::types::Subtype>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub chosen_creature_type: Option<crate::types::Subtype>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub chosen_card_type: Option<crate::types::CardType>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub chosen_player: Option<PlayerId>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub chosen_named_option: Option<String>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub noted_life_total: Option<i32>,
    pub power_toughness_choices: Vec<(i32, i32, Vec<B>)>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub battle_protector: Option<PlayerId>,
    pub discard_hand: bool,
    pub as_enters_counters: Vec<(crate::object::CounterType, u32)>,
    pub as_enters_tagged_objects: Vec<(crate::tag::TagKey, Vec<S>)>,
    pub transfer_as_enters_source_links: bool,
    pub as_enters_continuous_effects: Vec<crate::continuous::ContinuousEffectId>,
}
#[derive(Debug,Clone)]
#[cfg_attr(feature="serialization",derive(serde::Serialize,serde::Deserialize))]
#[cfg_attr(feature="serialization",serde(deny_unknown_fields,bound(deserialize="A: serde::Deserialize<'de>, B: serde::Deserialize<'de>, P: serde::Deserialize<'de>, S: serde::Deserialize<'de>, U: serde::Deserialize<'de>")))]
pub struct RetainedEntryEvent<A=crate::ability::Ability,B=crate::static_abilities::StaticAbility,P=crate::resolution::ResolutionProgram,S=crate::snapshot::ObjectSnapshot,U=crate::effect::Until> {
    pub object: ObjectId,
    pub from: Zone,
    pub enters_tapped: bool,
    pub enters_with_counters: Vec<(CounterType, u32)>,
    pub linked_exile_with_entering: Vec<ObjectId>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub enters_as_copy_of: Option<ObjectId>,
    pub copy_followups: Vec<ironsmith_core::EnterAsCopyFollowup>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub copy_duration: Option<U>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub copy_name_override: Option<String>,
    pub added_colors: ColorSet,
    pub added_card_types: Vec<CardType>,
    pub removes_other_card_types: bool,
    pub added_supertypes: Vec<Supertype>,
    pub removed_supertypes: Vec<Supertype>,
    pub added_subtypes: Vec<Subtype>,
    pub added_abilities: Vec<A>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub set_base_power_toughness: Option<(i32, i32)>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub controller_override: Option<PlayerId>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub pending_program: Option<(P, PlayerId)>,
    pub program_choices: RetainedPreparedEntryChoices<B,S>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub prepared_choices: Option<RetainedPreparedEntryChoices<B,S>>,
}
impl RetainedPreparedEntryChoices {
    fn capture(value:crate::game_state::PreparedEtbChoices)->Self {
        let crate::game_state::PreparedEtbChoices {chosen_color,chosen_basic_land_type,chosen_land_type,chosen_creature_type,chosen_card_type,chosen_player,chosen_named_option,noted_life_total,power_toughness_choices,battle_protector,discard_hand,as_enters_counters,as_enters_tagged_objects,transfer_as_enters_source_links,as_enters_continuous_effects}=value;
        let mut as_enters_tagged_objects:Vec<_>=as_enters_tagged_objects.into_iter().collect();
        as_enters_tagged_objects.sort_by(|a,b|a.0.cmp(&b.0));
        Self {chosen_color,chosen_basic_land_type,chosen_land_type,chosen_creature_type,chosen_card_type,chosen_player,chosen_named_option,noted_life_total,power_toughness_choices,battle_protector,discard_hand,as_enters_counters,as_enters_tagged_objects,transfer_as_enters_source_links,as_enters_continuous_effects}
    }
    fn into_native(self)->crate::game_state::PreparedEtbChoices {
        let Self {chosen_color,chosen_basic_land_type,chosen_land_type,chosen_creature_type,chosen_card_type,chosen_player,chosen_named_option,noted_life_total,power_toughness_choices,battle_protector,discard_hand,as_enters_counters,as_enters_tagged_objects,transfer_as_enters_source_links,as_enters_continuous_effects}=self;
        crate::game_state::PreparedEtbChoices {chosen_color,chosen_basic_land_type,chosen_land_type,chosen_creature_type,chosen_card_type,chosen_player,chosen_named_option,noted_life_total,power_toughness_choices,battle_protector,discard_hand,as_enters_counters,as_enters_tagged_objects: as_enters_tagged_objects.into_iter().collect(),transfer_as_enters_source_links,as_enters_continuous_effects}
    }
}
impl<B,S> RetainedPreparedEntryChoices<B,S> {
    fn validate(&self)->Result<(),String>{if self.as_enters_tagged_objects.windows(2).any(|pair|pair[0].0>=pair[1].0){return Err("entry capture tags must be ordered and unique".into());}Ok(())}
    fn map_payloads<B2,S2,Error>(self,ability:&mut impl FnMut(B)->Result<B2,Error>,snapshot:&mut impl FnMut(S)->Result<S2,Error>)->Result<RetainedPreparedEntryChoices<B2,S2>,Error>{
        let Self {chosen_color,chosen_basic_land_type,chosen_land_type,chosen_creature_type,chosen_card_type,chosen_player,chosen_named_option,noted_life_total,power_toughness_choices,battle_protector,discard_hand,as_enters_counters,as_enters_tagged_objects,transfer_as_enters_source_links,as_enters_continuous_effects}=self;
        let power_toughness_choices=power_toughness_choices.into_iter().map(|(p,t,abilities)|Ok((p,t,abilities.into_iter().map(&mut *ability).collect::<Result<Vec<_>,Error>>()?))).collect::<Result<Vec<_>,Error>>()?;
        let as_enters_tagged_objects=as_enters_tagged_objects.into_iter().map(|(tag,values)|Ok((tag,values.into_iter().map(&mut *snapshot).collect::<Result<Vec<_>,Error>>()?))).collect::<Result<Vec<_>,Error>>()?;
        Ok(RetainedPreparedEntryChoices {chosen_color,chosen_basic_land_type,chosen_land_type,chosen_creature_type,chosen_card_type,chosen_player,chosen_named_option,noted_life_total,power_toughness_choices,battle_protector,discard_hand,as_enters_counters,as_enters_tagged_objects,transfer_as_enters_source_links,as_enters_continuous_effects})
    }
}
impl RetainedEntryEvent {
    pub fn capture(value:crate::events::EnterBattlefieldEvent)->Self {
        let crate::events::EnterBattlefieldEvent {object,from,enters_tapped,enters_with_counters,linked_exile_with_entering,enters_as_copy_of,copy_followups,copy_duration,copy_name_override,added_colors,added_card_types,removes_other_card_types,added_supertypes,removed_supertypes,added_subtypes,added_abilities,set_base_power_toughness,controller_override,pending_program,program_choices,prepared_choices}=value;
        let program_choices=RetainedPreparedEntryChoices::capture(program_choices);
        let prepared_choices=prepared_choices.map(RetainedPreparedEntryChoices::capture);
        Self {object,from,enters_tapped,enters_with_counters,linked_exile_with_entering,enters_as_copy_of,copy_followups,copy_duration,copy_name_override,added_colors,added_card_types,removes_other_card_types,added_supertypes,removed_supertypes,added_subtypes,added_abilities,set_base_power_toughness,controller_override,pending_program,program_choices,prepared_choices}
    }
    pub fn into_native(self)->Result<crate::events::EnterBattlefieldEvent,String>{
        self.validate()?;
        let Self {object,from,enters_tapped,enters_with_counters,linked_exile_with_entering,enters_as_copy_of,copy_followups,copy_duration,copy_name_override,added_colors,added_card_types,removes_other_card_types,added_supertypes,removed_supertypes,added_subtypes,added_abilities,set_base_power_toughness,controller_override,pending_program,program_choices,prepared_choices}=self;
        let program_choices=program_choices.into_native();
        let prepared_choices=prepared_choices.map(RetainedPreparedEntryChoices::into_native);
        Ok(crate::events::EnterBattlefieldEvent {object,from,enters_tapped,enters_with_counters,linked_exile_with_entering,enters_as_copy_of,copy_followups,copy_duration,copy_name_override,added_colors,added_card_types,removes_other_card_types,added_supertypes,removed_supertypes,added_subtypes,added_abilities,set_base_power_toughness,controller_override,pending_program,program_choices,prepared_choices})
    }
}
impl<A,B,P,S,U> RetainedEntryEvent<A,B,P,S,U> {
    pub fn validate(&self)->Result<(),String>{self.program_choices.validate()?;if let Some(choices)=&self.prepared_choices {choices.validate()?;}Ok(())}
    /// Validate all canonical collections before calling any allocator/binder.
    /// Conversion failures must roll back owning table/world mutations.
    pub fn try_map_payloads<A2,B2,P2,S2,U2,Error>(self,mut ability:impl FnMut(A)->Result<A2,Error>,mut static_ability:impl FnMut(B)->Result<B2,Error>,program:impl FnOnce(P)->Result<P2,Error>,mut snapshot:impl FnMut(S)->Result<S2,Error>,duration:impl FnOnce(U)->Result<U2,Error>,invalid:impl FnOnce(String)->Error)->Result<RetainedEntryEvent<A2,B2,P2,S2,U2>,Error>{
        self.validate().map_err(invalid)?;
        let Self {object,from,enters_tapped,enters_with_counters,linked_exile_with_entering,enters_as_copy_of,copy_followups,copy_duration,copy_name_override,added_colors,added_card_types,removes_other_card_types,added_supertypes,removed_supertypes,added_subtypes,added_abilities,set_base_power_toughness,controller_override,pending_program,program_choices,prepared_choices}=self;
        let copy_duration=copy_duration.map(duration).transpose()?;
        let added_abilities=added_abilities.into_iter().map(&mut ability).collect::<Result<Vec<_>,Error>>()?;
        let pending_program=pending_program.map(|(value,player)|program(value).map(|value|(value,player))).transpose()?;
        let program_choices=program_choices.map_payloads(&mut static_ability,&mut snapshot)?;
        let prepared_choices=prepared_choices.map(|choices|choices.map_payloads(&mut static_ability,&mut snapshot)).transpose()?;
        Ok(RetainedEntryEvent {object,from,enters_tapped,enters_with_counters,linked_exile_with_entering,enters_as_copy_of,copy_followups,copy_duration,copy_name_override,added_colors,added_card_types,removes_other_card_types,added_supertypes,removed_supertypes,added_subtypes,added_abilities,set_base_power_toughness,controller_override,pending_program,program_choices,prepared_choices})
    }
}
#[cfg(test)]
mod prospective_entry_capture_contract_tests {
    use super::*;
    #[test] fn native_capture_restores_entry_fields_and_selected_ability_identity(){
        let alice=PlayerId::from_index(0);let bob=PlayerId::from_index(1);let mut event=crate::events::EnterBattlefieldEvent::tapped(ObjectId::from_raw(41),Zone::Hand);let flying=crate::static_abilities::StaticAbility::flying();let identity=flying.instance_id();event.enters_with_counters=vec![(CounterType::PlusOnePlusOne,u32::MAX)];event.controller_override=Some(bob);event.program_choices.chosen_player=Some(alice);event.program_choices.noted_life_total=Some(-4);event.program_choices.power_toughness_choices=vec![(3,5,vec![flying.clone(),flying])];event.prepared_choices=Some(event.program_choices.clone());event.copy_name_override=Some("copied name".into());
        let retained=RetainedEntryEvent::capture(event);let restored=retained.try_map_payloads(Ok::<_,String>,Ok,Ok,Ok,Ok,|e|e).unwrap().into_native().unwrap();assert!(restored.enters_tapped);assert_eq!(restored.enters_with_counters,vec![(CounterType::PlusOnePlusOne,u32::MAX)]);assert_eq!(restored.controller_override,Some(bob));assert_eq!(restored.copy_name_override.as_deref(),Some("copied name"));assert_eq!(restored.program_choices.chosen_player,Some(alice));assert_eq!(restored.program_choices.noted_life_total,Some(-4));assert_eq!(restored.program_choices.power_toughness_choices[0].2.len(),2);assert!(restored.program_choices.power_toughness_choices[0].2.iter().all(|a|a.instance_id()==identity));assert_eq!(restored.prepared_choices.unwrap().power_toughness_choices[0].2[0].instance_id(),identity);
    }
    #[test] fn duplicate_tag_capture_rejects_before_any_converter(){
        let mut value=RetainedEntryEvent::capture(crate::events::EnterBattlefieldEvent::new(ObjectId::from_raw(42),Zone::Library));let tag=crate::tag::TagKey::new("entry");value.program_choices.as_enters_tagged_objects=vec![(tag.clone(),vec![]),(tag,vec![])];let calls=std::cell::Cell::new(0);let result=value.try_map_payloads(|a|{calls.set(calls.get()+1);Ok::<_,String>(a)},|a|{calls.set(calls.get()+1);Ok(a)},|a|{calls.set(calls.get()+1);Ok(a)},|a|{calls.set(calls.get()+1);Ok(a)},|a|{calls.set(calls.get()+1);Ok(a)},|e|e);assert!(result.unwrap_err().contains("ordered and unique"));assert_eq!(calls.get(),0);
    }
    #[test] fn selected_ability_conversion_error_is_not_discarded(){
        let mut value=RetainedEntryEvent::capture(crate::events::EnterBattlefieldEvent::new(ObjectId::from_raw(43),Zone::Graveyard));value.program_choices.power_toughness_choices=vec![(1,2,vec![crate::static_abilities::StaticAbility::flying()])];let result=value.try_map_payloads(Ok::<_,String>,|_|Err::<crate::static_abilities::StaticAbility,_>("selected ability".into()),Ok,Ok,Ok,|e|e);assert_eq!(result.unwrap_err(),"selected ability");
    }
}

/// Exact damage event retained for prevention follow-ups. Historical executable
/// abilities use the mandatory snapshot converter, never native snapshot serde.
#[derive(Debug,Clone)]
#[cfg_attr(feature="serialization",derive(serde::Serialize,serde::Deserialize))]
#[cfg_attr(feature="serialization",serde(deny_unknown_fields,bound(deserialize="S: serde::Deserialize<'de>")))]
pub struct RetainedDamageEvent<S=crate::snapshot::ObjectSnapshot>{
    pub source:ObjectId,
    pub target:crate::events::DamageTarget,
    pub amount:u32,
    pub excess_damage:u32,
    pub is_combat:bool,
    pub is_unpreventable:bool,
    pub cause:crate::events::cause::EventCause,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub remainder:Option<(crate::events::DamageTarget,u32)>,
    #[cfg_attr(feature="serialization",serde(deserialize_with="present_option"))]
    pub target_snapshot:Option<S>,
}
impl RetainedDamageEvent {
    pub fn capture(value:crate::events::DamageEvent)->Self{let crate::events::DamageEvent{source,target,amount,excess_damage,is_combat,is_unpreventable,cause,remainder,target_snapshot}=value;Self{source,target,amount,excess_damage,is_combat,is_unpreventable,cause,remainder,target_snapshot}}
    pub fn into_native(self)->crate::events::DamageEvent{let Self{source,target,amount,excess_damage,is_combat,is_unpreventable,cause,remainder,target_snapshot}=self;crate::events::DamageEvent{source,target,amount,excess_damage,is_combat,is_unpreventable,cause,remainder,target_snapshot}}
}
impl<S> RetainedDamageEvent<S>{
    pub fn try_map_snapshot<S2,Error>(self,snapshot:impl FnOnce(S)->Result<S2,Error>)->Result<RetainedDamageEvent<S2>,Error>{let Self{source,target,amount,excess_damage,is_combat,is_unpreventable,cause,remainder,target_snapshot}=self;Ok(RetainedDamageEvent{source,target,amount,excess_damage,is_combat,is_unpreventable,cause,remainder,target_snapshot:target_snapshot.map(snapshot).transpose()?})}
}

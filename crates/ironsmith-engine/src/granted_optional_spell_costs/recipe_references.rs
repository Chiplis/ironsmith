//! Per-acquisition executable namespaces through native typed reference owners.
use super::*;
use crate::execution_reference_graph::{ReferenceMapper, ReferenceInventory, visit_declared_effect_ids};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn used_effect_ids(
    _game: &GameState,
    recipient: &crate::object::Object,
) -> Result<BTreeSet<u32>, String> {
    let mut inventory=ReferenceInventory::default();
    let mut inventory_recipient=recipient.clone();
    if inventory_recipient.card.is_none() {inventory_recipient.card=Some(crate::ids::CardId::from_raw(0));}
    inventory.definition(&inventory_recipient.to_card_definition())?;
    for grant in &recipient.temporary_static_ability_grants {
        if let Some(ability)=&grant.ability_payload {inventory.static_ability(ability)?;}
    }
    Ok(inventory.ids)
}

pub(super) fn acquire(
    price: &GrantedOptionalCostPrice<crate::costs::Cost>,
    effects: &[crate::effect::Effect],
    reduction: &Option<crate::effect::Value>,
    locals: &[crate::tag::TagKey],
    scope: &str,
    used: &mut BTreeSet<u32>,
) -> Result<(
    GrantedOptionalCostPrice<crate::costs::Cost>, Vec<crate::effect::Effect>,
    Option<crate::effect::Value>, Vec<crate::tag::TagKey>,
), String> {
    let mut declared=Vec::new();
    let mut declare=|id| {if !declared.contains(&id) {declared.push(id);}};
    if let GrantedOptionalCostPrice::Fixed(cost)=price {
        crate::ability::visit_total_cost_owned_effects(cost, &mut |effect|
            visit_declared_effect_ids(effect, &mut declare));
    }
    for effect in effects {visit_declared_effect_ids(effect,&mut declare);}
    let mut fresh=BTreeMap::new();
    let mut reserved=used.clone();
    let mut next=0u32;
    for old in declared {
        if old>=ironsmith_core::EffectId::PENDING_PRIOR_ACTION.0 {continue;}
        while reserved.contains(&next) {
            next=next.checked_add(1).ok_or("effect reference namespace exhausted")?;
        }
        if next>=ironsmith_core::EffectId::PENDING_PRIOR_ACTION.0 {return Err("effect reference namespace exhausted".into());}
        fresh.insert(old,next);reserved.insert(next);
    }
    let local=locals.iter().cloned().collect::<BTreeSet<_>>();
    let mut tags=|tag:&mut crate::tag::TagKey| {
        if local.contains(tag) {*tag=format!("__optional_recipe:{scope}:{}",tag.as_str()).into();}
    };
    let mut ids=|id:&mut crate::effect::EffectId| {
        if let Some(mapped)=fresh.get(&id.0) {id.0=*mapped;}
    };
    let mut mapper=ReferenceMapper{tags:&mut tags,ids:&mut ids};
    let price=match price {
        GrantedOptionalCostPrice::Fixed(cost)=>GrantedOptionalCostPrice::Fixed(mapper.total_cost(cost)?),
        GrantedOptionalCostPrice::GenericManaValue=>GrantedOptionalCostPrice::GenericManaValue,
    };
    let effects=mapper.effects(effects)?;
    let mut reduction=reduction.clone();mapper.pure(&mut reduction);
    let mut locals=locals.to_vec();mapper.pure(&mut locals);
    // Unknown native executors/costs fail before publishing any reservation.
    *used=reserved;
    Ok((price,effects,reduction,locals))
}

#[cfg(test)]
mod native_recipe_reference_tests {
    use super::*;
    use crate::effect::{Effect,EffectId,Value};
    use crate::effects::{ChooseObjectsEffect,DrawCardsEffect,WithIdEffect};
    #[test]
    fn native_recipe_acquisitions_keep_price_effect_value_and_local_tag_namespaces_distinct() {
        let choose=Effect::with_id(7,Effect::new(ChooseObjectsEffect::new(
            crate::filter::ObjectFilter::creature(),1,crate::target::PlayerFilter::You,"local")));
        let price=GrantedOptionalCostPrice::Fixed(crate::cost::TotalCost::from_cost(
            crate::costs::Cost::try_effect(choose).unwrap()));
        let body=vec![Effect::with_id(8,Effect::draw(Value::EffectValue(EffectId(7)))),
            Effect::draw(Value::EffectValue(EffectId(8)))];
        let reduction=Some(Value::EffectValue(EffectId(7)));
        let mut used=[0u32,7,8].into_iter().collect();
        let first=acquire(&price,&body,&reduction,&["local".into()],"first",&mut used).unwrap();
        let second=acquire(&price,&body,&reduction,&["local".into()],"second",&mut used).unwrap();
        let first_id=first.1[0].downcast_ref::<WithIdEffect>().unwrap().id;
        let second_id=second.1[0].downcast_ref::<WithIdEffect>().unwrap().id;
        assert_ne!(first_id,second_id);assert_ne!(first_id,EffectId(8));
        assert_eq!(first.3,vec![crate::tag::TagKey::from("__optional_recipe:first:local")]);
        assert_eq!(second.3,vec![crate::tag::TagKey::from("__optional_recipe:second:local")]);
        assert_eq!(first.1[1].downcast_ref::<DrawCardsEffect>().unwrap().count,Value::EffectValue(first_id));
        let GrantedOptionalCostPrice::Fixed(first_price)=first.0 else{panic!("fixedprice")};
        let price_id=first_price.costs()[0].effect_ref().unwrap().downcast_ref::<WithIdEffect>().unwrap().id;
        assert_eq!(first.2,Some(Value::EffectValue(price_id)));
        let priced_choice=first_price.costs()[0].effect_ref().unwrap().downcast_ref::<WithIdEffect>().unwrap()
            .effect.downcast_ref::<ChooseObjectsEffect>().unwrap();
        assert_eq!(priced_choice.tag,crate::tag::TagKey::from("__optional_recipe:first:local"));
        assert_eq!(body[0].downcast_ref::<WithIdEffect>().unwrap().id,EffectId(8),"immutable original remains unchanged");
    }
    #[test]
    fn host_referenced_only_ids_are_reserved_without_fabricating_declarations() {
        let card=crate::card::CardBuilder::new(crate::ids::CardId::new(),"Host").build();
        let mut definition=crate::cards::CardDefinition::new(card);
        definition.spell_effect=Some(vec![Effect::if_then(EffectId(0),crate::effect::EffectPredicate::Happened,vec![Effect::draw(1)])].into());
        let mut inventory=ReferenceInventory::default();inventory.definition(&definition).unwrap();
        assert!(inventory.ids.contains(&0));
        let mut declared=Vec::new();for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
            visit_declared_effect_ids(effect,&mut |id|declared.push(id));
        }
        assert!(declared.is_empty());
        let (_,effects,_,_)=acquire(&GrantedOptionalCostPrice::GenericManaValue,
            &[Effect::with_id(7,Effect::draw(1))],&None,&[],"new",&mut inventory.ids).unwrap();
        assert_ne!(effects[0].downcast_ref::<WithIdEffect>().unwrap().id,EffectId(0));
    }
    #[derive(Debug,Clone)]struct Opaque;
    impl crate::effects::EffectExecutor for Opaque {
        fn execute(&self,_:&mut GameState,_:&mut crate::effects::ExecutionContext)->Result<crate::effect::EffectOutcome,crate::effects::ExecutionError> {
            Ok(crate::effect::EffectOutcome::count(0))
        }
    }
    #[test]
    fn opaque_native_recipe_fails_without_reserving_partial_ids() {
        let mut used:BTreeSet<u32>=[10].into_iter().collect();let before=used.clone();
        assert!(acquire(&GrantedOptionalCostPrice::GenericManaValue,
            &[Effect::with_id(7,Effect::new(Opaque).with_serialized_model("{\"kind\":\"DrawCardsEffect\",\"payload\":{}}"))],&None,&[],"opaque",&mut used).is_err());
        assert_eq!(used,before);
        let card=crate::card::CardBuilder::new(crate::ids::CardId::new(),"Opaque host").build();
        let mut host=crate::cards::CardDefinition::new(card);
        host.spell_effect=Some(vec![Effect::new(Opaque)].into());
        assert!(ReferenceInventory::default().definition(&host).is_err(),"opaque host cannot prove its namespace is free");
    }
}

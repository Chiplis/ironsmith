//! Optional scoped discounts on a named casting price, preserving nonmana children.
use crate::{GameState,ObjectId,PlayerId};
use crate::cost::TotalCost;
use crate::filter::ObjectFilterExt as _;
fn reduce(cost:&TotalCost,amount:u32)->TotalCost {
 if let Some(branches)=cost.as_one_of(){return TotalCost::one_of(branches.iter().map(|b|reduce(b,amount)).collect())}
 // A component-local discount must not consume the receiving spell's
 // generic mana. Preserve X until that spell's ordinary X announcement.
 let mut mana=crate::mana::ManaCost::new();let mut other=Vec::new();let mut position=None;
 for part in cost.costs(){
  if let Some(part)=part.mana_cost_ref(){position.get_or_insert(other.len());mana=crate::decision::add_mana_cost(&mana,part);}
  else{other.push(part.clone());}
 }
 if let Some(position)=position {
  let payment=if (mana.has_x()||mana.pips().iter().any(|pip|pip.len()>1))&&amount>0{
   let mut recipe=ironsmith_core::DynamicManaCost::new(mana,None,None,None,ironsmith_core::DynamicManaDisplayHint::Default);
   recipe.generic_reduction=amount;crate::costs::Cost::dynamic_mana(recipe)
  }else{crate::costs::Cost::mana(mana.reduce_generic(amount))};
  other.insert(position,payment);
 }
 TotalCost::from_costs(other)
}
/// Every optional reduction amount remains a real price branch. Providers are
/// read at this announcement boundary; the chosen concrete cost is retained.
pub(crate) fn alternatives(game:&GameState,spell_id:ObjectId,caster:PlayerId,keyword:&str,cost:&TotalCost)->Vec<TotalCost>{
 let Some(mut spell)=game.object(spell_id).cloned()else{return vec![cost.clone()]};
 if let Some(chars)=game.current_characteristics(spell_id){spell.name=chars.name;spell.card_types=chars.card_types;spell.subtypes=chars.subtypes;spell.supertypes=chars.supertypes;spell.color_override=Some(chars.colors);}
 let(mut required,mut optional)=(0u32,0u32);let view=crate::derived_view::DerivedGameView::new(game);
 for source in game.battlefield.iter().copied().filter(|id|!game.is_phased_out(*id)){
  let Some(abilities)=view.static_abilities_rc(source)else{continue};let Some(owner)=game.object(source)else{continue};
  for ability in abilities.iter(){if !ability.is_active(game,source){continue}let Some(model)=ability.compiled_model()else{continue};
   let payload=match &model.payload{ironsmith_core::StaticAbilityPayload::Conditional{ability,..}=>&ability.payload,p=>p};
   let ironsmith_core::StaticAbilityPayload::NamedCastingPriceReduction{filter,keywords,maximum,optional:may}=payload else{continue};
   if !keywords.iter().any(|name|name.eq_ignore_ascii_case(keyword)){continue}
   let context=game.filter_context_for(game.controller_of(owner),Some(source)).with_caster(Some(caster)).with_prospective_cast(spell_id);let mut filter=filter.clone();filter.zone=None;
   if filter.matches_non_recursive(&spell,&context,game){if *may{optional=optional.saturating_add(*maximum)}else{required=required.saturating_add(*maximum)}}
  }
 }
 // More reduction than the generic part is economically identical; retain a
 // single such branch instead of allocating an unbounded redundant menu.
 let bounds=|price:&TotalCost|{
  let mut generic=0u32;let mut variable=false;
  for mana in price.costs().iter().filter_map(|p|p.mana_cost_ref()){
   // Announced hybrid halves may introduce generic mana too. Clipping
   // against only literal generic pips would suppress valid discounts.
   generic=generic.saturating_add(mana.pips().iter().map(|pip|pip.iter().filter_map(|symbol|match symbol {
    crate::mana::ManaSymbol::Generic(n)=>Some(u32::from(*n)),_=>None,
   }).max().unwrap_or(0)).sum::<u32>());variable|=mana.has_x();
  }
  (generic,variable)
 };
 let (generic,variable)=if let Some(branches)=cost.as_one_of(){branches.iter().map(bounds).fold((0,false),|(g,x),(next,y)|(g.max(next),x||y))}else{bounds(cost)};
 // X is announced after the optional component's discount choice. Every
 // provider-supported amount remains distinct until X is actually locked.
 let cap=if variable{required.saturating_add(optional)}else{generic};
 let min=required.min(cap);let max=required.saturating_add(optional).min(cap);
 let mut prices=Vec::new();for amount in min..=max{let total=reduce(cost,amount);if let Some(branches)=total.as_one_of(){prices.extend(branches.iter().cloned())}else{prices.push(total)}}prices
}

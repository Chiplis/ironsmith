//! An immutable additional-price recipe whose only remaining input is
//! the receiving spell's X and hybrid/Phyrexian announcements. The discount belongs to
//! this price, never to the combined spell cost.
use crate::{cost::TotalCost,costs::Cost,mana::ManaCost};

pub(crate) fn recipe(part:&Cost)->Option<&ironsmith_core::DynamicManaCost>{
 let recipe=part.dynamic_mana_cost_ref()?;
 (!recipe.source_mana_cost&&recipe.mana_cost_of.is_none()
  &&recipe.source_mana_cost_reduction_condition.is_none()&&recipe.x_value.is_none()
  &&recipe.additional_generic.is_none()&&recipe.multiplier.is_none()
  &&recipe.generic_reduction>0).then_some(recipe)
}
/// Used only to detect X before its announcement. This is not a payment price.
pub(crate) fn unresolved_mana(part:&Cost)->Option<ManaCost>{
 part.mana_cost_ref().cloned().or_else(||recipe(part).map(|r|r.base.clone()))
}
/// Resolve pure retained recipes only after their symbol choices are fixed.
/// Hybrid/Phyrexian recipes stay retained until their own halves are announced.
/// All other dynamic costs retain their
/// ordinary context-dependent owner and cannot become an accidentally free cost.
pub(crate) fn resolve(cost:&TotalCost,x:u32)->TotalCost{
 if let Some(branches)=cost.as_one_of(){return TotalCost::one_of(branches.iter().map(|branch|resolve(branch,x)).collect())}
 TotalCost::from_costs(cost.costs().iter().map(|part|{
  if let Some(recipe)=recipe(part).filter(|recipe|!recipe.base.pips().iter().any(|pip|pip.len()>1)){Cost::mana(crate::decision::mana_cost_with_locked_x_and_generic_reduction(&recipe.base,x,recipe.generic_reduction))}
  else{part.clone()}
 }).collect())
}

/// Complete preview branches for one retained price. Hybrid payment is bound
/// before its local discount, never flattened into independent invented pips.
pub(crate) fn mana_options(cost:&TotalCost,x:u32)->Vec<ManaCost>{
 if let Some(branches)=cost.as_one_of(){return branches.iter().flat_map(|branch|mana_options(branch,x)).collect()}
 let mut options=vec![ManaCost::new()];
 for part in cost.costs(){
  let additions=if let Some(recipe)=recipe(part){
   let bound=crate::decision::mana_cost_with_locked_x_and_generic_reduction(&recipe.base,x,0);
   bound.reduced_by_mana_cost_options_with_spill(&ManaCost::new().add_generic(recipe.generic_reduction),true)
  }else if let Some(mana)=part.mana_cost_ref(){
   vec![crate::decision::mana_cost_with_locked_x_and_generic_reduction(mana,x,0)]
  }else if part.is_mana_cost(){return Vec::new()}else{continue};
  options=options.iter().flat_map(|base|additions.iter().map(move|add|crate::decision::add_mana_cost(base,add))).collect();
 }
 options
}
/// Read-only affordability hypotheses keep all nonmana obligations. These
/// branches never replace the retained announcement recipe or its identity.
pub(crate) fn preview_cost_options(cost:&TotalCost,x:u32)->Vec<TotalCost>{
 if let Some(branches)=cost.as_one_of(){return branches.iter().flat_map(|branch|preview_cost_options(branch,x)).collect()}
 let nonmana=cost.non_mana_costs().cloned().collect::<Vec<_>>();
 mana_options(cost,x).into_iter().map(|mana|{let mut parts=nonmana.clone();parts.insert(0,Cost::mana(mana));TotalCost::from_costs(parts)}).collect()
}

/// Bind one component against the original complete announcement's pip
/// indices. Removing discounted pips never redirects a later component's choice.
pub(crate) fn bind_mana(
 mana:&ManaCost,reduction:u32,x:u32,choices:&[(usize,crate::mana::ManaSymbol)],hybrid_indices:&[usize],cursor:&mut usize,
)->Result<ManaCost,String>{
 let mut pips=Vec::new();
 for pip in mana.pips(){
  if pip.len()>1 {
   let index=*hybrid_indices.get(*cursor).ok_or_else(||"A retained price changed its hybrid layout".to_owned())?;*cursor+=1;
   let symbol=choices.iter().find(|(position,_)|*position==index).map(|(_,symbol)|*symbol)
    .filter(|symbol|pip.contains(symbol)).ok_or_else(||"A retained price has an unannounced hybrid payment".to_owned())?;
   pips.push(vec![symbol]);
  }else{pips.push(pip.clone())}
 }
 Ok(crate::decision::mana_cost_with_locked_x_and_generic_reduction(&mana.with_pips(pips),x,reduction))
}
pub(crate) fn bind_price(
 cost:&TotalCost,x:u32,choices:&[(usize,crate::mana::ManaSymbol)],hybrid_indices:&[usize],cursor:&mut usize,
)->Result<ManaCost,String>{
 if cost.as_one_of().is_some(){return Err("A retained price lost its announced branch".into())}
 let mut combined=ManaCost::new();
 for part in cost.costs(){
  let bound=if let Some(recipe)=recipe(part){bind_mana(&recipe.base,recipe.generic_reduction,x,choices,hybrid_indices,cursor)?}
   else if let Some(mana)=part.mana_cost_ref(){bind_mana(mana,0,x,choices,hybrid_indices,cursor)?}
   else if part.is_mana_cost(){return Err("Unresolved contextual mana price during spell payment".into())}else{continue};
  combined=crate::decision::add_mana_cost(&combined,&bound);
 }
 Ok(combined)
}

#[cfg(test)]
mod tests {
 use super::*;
 #[test]
 fn actual_x_is_locked_before_a_component_discount_and_nonmana_obligations_survive(){
  let mana=ManaCost::from_symbols(vec![crate::mana::ManaSymbol::X,crate::mana::ManaSymbol::Generic(1),crate::mana::ManaSymbol::Red])
   .with_spending_restriction(ironsmith_core::mana::ManaSpendingRestriction::Colors(crate::color::ColorSet::RED));
  let mut recipe=ironsmith_core::DynamicManaCost::new(mana.clone(),None,None,None,ironsmith_core::DynamicManaDisplayHint::Default);recipe.generic_reduction=2;
  let nonmana=Cost::life(3);
  let retained=TotalCost::from_costs(vec![Cost::dynamic_mana(recipe),nonmana.clone()]);
  assert!(unresolved_mana(&retained.costs()[0]).unwrap().has_x());
  let locked=resolve(&retained,4);let price=locked.mana_cost().unwrap();
  assert!(!price.has_x());assert_eq!(price.generic_mana_total(),3);
  assert_eq!(price.spending_restrictions(),mana.spending_restrictions());
  assert_eq!(locked.costs()[1],nonmana);
  assert_eq!(resolve(&retained,0).mana_cost().unwrap().to_oracle(),"{R}","excess local reduction cannot reduce colored mana or the spell's other costs");
 }
 #[test]
 fn unrelated_dynamic_recipes_are_not_reinterpreted_as_fixed_or_free(){
  let dynamic=ironsmith_core::DynamicManaCost::generic_equal_to(crate::effect::Value::X);
  let retained=TotalCost::from_cost(Cost::dynamic_mana(dynamic));
  assert_eq!(resolve(&retained,4),retained);
  assert!(unresolved_mana(&retained.costs()[0]).is_none());
 }
 #[test]
 fn x_and_hybrid_halves_are_bound_before_local_discount_without_losing_nonmana_or_spending_rules(){
  use crate::mana::ManaSymbol::*;
  let mana=ManaCost::from_pips(vec![vec![X],vec![Generic(2),crate::mana::ManaSymbol::Blue]])
   .with_spending_restriction(ironsmith_core::mana::ManaSpendingRestriction::Colors(crate::color::ColorSet::RED));
  let mut recipe=ironsmith_core::DynamicManaCost::new(mana.clone(),None,None,None,ironsmith_core::DynamicManaDisplayHint::Default);recipe.generic_reduction=1;
  let nonmana=Cost::life(3);let price=TotalCost::from_costs(vec![Cost::dynamic_mana(recipe),nonmana.clone()]);
  assert_eq!(resolve(&price,0),price,"a chosen X does not discard an unannounced hybrid discount");
  for(x,half,generic)in[(0,Generic(2),1),(3,Generic(2),4),(0,crate::mana::ManaSymbol::Blue,0)]{
   let result=bind_price(&price,x,&[(4,half)],&[4],&mut 0).unwrap();
   assert_eq!(result.generic_mana_total(),generic);assert_eq!(result.spending_restrictions(),mana.spending_restrictions());
  }
  assert_eq!(price.costs()[1],nonmana);assert!(bind_price(&price,0,&[],&[4],&mut 0).is_err(),"missing announced half cannot become free");
 }

}

//! Activation-local as-though-zone permissions (CR 609.4).
//! The real object and the later resolving program retain their actual zone.
use crate::{GameState,ObjectId,Zone};
use crate::ability::{AbilityKind,ActivatedAbility};
use crate::cost::TotalCost;
use crate::effects::RevealSourceFromHandEffect;
use crate::effect::Condition;
use crate::filter::ObjectFilterExt as _;

/// Find a current permission for this exact authored ability occurrence.
/// A keyword identity on the card alone never qualifies another ability.
pub fn permission(game:&GameState,source:ObjectId,index:usize)->Option<(Zone,Zone)> {
    let object=game.object(source)?;
    let actor=game.turn.priority_player.unwrap_or_else(||game.controller_of(object));
    let occurrence=game.current_ability(source,index)?;
    if !matches!(&occurrence.kind,AbilityKind::Activated(_)){return None;}
    let view=crate::derived_view::DerivedGameView::new(game);
    for grant_source in game.battlefield.iter().copied().filter(|id|!game.is_phased_out(*id)) {
        let Some(granter)=game.object(grant_source) else{continue};
        let controller=game.controller_of(granter);
        if controller!=actor {continue;}
        let Some(statics)=view.static_abilities_rc(grant_source) else{continue};
        for ability in statics.iter() {
            if !ability.is_active(game,grant_source){continue;}
            let Some(model)=ability.compiled_model() else{continue};
            let payload=match &model.payload {
                ironsmith_core::StaticAbilityPayload::Conditional{ability,..}=>&ability.payload,
                payload=>payload,
            };
            let ironsmith_core::StaticAbilityPayload::ActivateAbilitiesAsThoughZone{filter,designation,from,as_though,..}=payload else{continue};
            if from==as_though || object.zone!=*from || !occurrence.functions_in(as_though) || !occurrence.has_designation(designation){continue;}
            let context=game.filter_context_for(controller,Some(grant_source));
            if filter.matches(object,&context,game){return Some((*from,*as_though));}
        }
    }
    None
}

/// Snapshot permission into the announced cost definition. Cost payment may
/// remove the granting permanent; the captured cost remains payable. Never
/// alter `effects`: an instruction later requiring hand still requires hand.
pub fn prepare(game:&GameState,source:ObjectId,index:usize,ability:&ActivatedAbility)->Option<ActivatedAbility> {
    let (actual,virtual_zone)=permission(game,source,index)?;
    let mut prepared=ability.clone();
    if let Some(condition)=&mut prepared.activation_condition {
        rewrite_source_zone(condition,actual,virtual_zone);
    }
    prepared.mana_cost=rewrite_cost(&prepared.mana_cost,actual,virtual_zone);
    Some(prepared)
}

/// Mana-ability admission also needs the functional-zone permission. Its
/// ordinary immediate execution still uses the normal mana-activation owner.
pub(crate) fn prepare_occurrence(game:&GameState,source:ObjectId,index:usize,ability:&crate::ability::Ability)->Option<crate::ability::Ability> {
    let AbilityKind::Activated(activated)=&ability.kind else{return None};
    let prepared=prepare(game,source,index,activated)?;
    let mut occurrence=ability.clone();occurrence.kind=AbilityKind::Activated(prepared);
    let actual=game.object(source)?.zone;
    if !occurrence.functional_zones.contains(&actual){occurrence.functional_zones.push(actual);}
    Some(occurrence)
}

fn rewrite_cost(cost:&TotalCost,actual:Zone,virtual_zone:Zone)->TotalCost {
    cost.clone().try_map(|component| -> Result<_,std::convert::Infallible> {
        if virtual_zone==Zone::Hand {
            if let Some(reveal)=component.effect_ref().and_then(|effect|effect.downcast_ref::<RevealSourceFromHandEffect>()) {
                if reveal.activation_source_zone.is_none() {
                    let mut captured=reveal.clone();captured.activation_source_zone=Some(actual);
                    return Ok(crate::costs::Cost::effect(captured));
                }
            }
        }
        Ok(component)
    }).unwrap_or_else(|never|match never {})
}
fn rewrite_source_zone(condition:&mut Condition,actual:Zone,virtual_zone:Zone) {
    match condition {
        Condition::SourceIsInZone(zone)=>{
            *condition=if *zone==virtual_zone {Condition::SourceIsInZone(actual)}
                else {Condition::Not(Box::new(Condition::SourceIsInZone(actual)))};
        }
        Condition::SourceMatches(filter)=>rewrite_source_filter_zone(filter,actual,virtual_zone),
        Condition::Not(inner)=>rewrite_source_zone(inner,actual,virtual_zone),
        Condition::And(left,right)|Condition::Or(left,right)=>{
            rewrite_source_zone(left,actual,virtual_zone);rewrite_source_zone(right,actual,virtual_zone);
        }
        _=>{}
    }
}

fn rewrite_source_filter_zone(filter:&mut crate::target::ObjectFilter,actual:Zone,virtual_zone:Zone){
    if let Some(zone)=filter.zone {
        // `actual != virtual_zone`: a virtual-zone request becomes a real
        // actual-zone request; every other zone request must remain false.
        filter.zone=Some(if zone==virtual_zone {actual}else{virtual_zone});
    }
    for branch in &mut filter.any_of {rewrite_source_filter_zone(branch,actual,virtual_zone);}
}

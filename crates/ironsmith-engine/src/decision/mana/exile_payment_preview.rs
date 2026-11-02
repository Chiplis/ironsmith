//! A selected exile cost's characteristic reduction. Correlation uses the
//! actual typed exile instruction and its selector, never source text.
use super::*;
fn collect_specs(effect:&crate::effect::Effect,out:&mut Vec<crate::target::ChooseSpec>){
    if let Some(exile)=effect.downcast_ref::<crate::effects::ExileEffect>(){out.push(exile.spec.clone());return;}
    if let Some(exile)=effect.downcast_ref::<crate::effects::ExileUntilEffect>(){out.push(exile.spec.clone());return;}
    if let Some(inner)=effect.transparent_child_effect(){collect_specs(inner,out);return;}
    effect.0.visit_child_effects(&mut |child|collect_specs(child,out));
}
pub(super) fn maximum(game:&GameState,player:PlayerId,spell:&crate::object::Object,selected_only:bool,query:&ironsmith_core::PriorEffectMetricQuery)->Option<i32>{
    use ironsmith_core::{PriorEffectAction,EffectMetric};
    if query.action!=Some(PriorEffectAction::Exiled)||!matches!(query.metric,EffectMetric::FirstManaValue|EffectMetric::TotalManaValue|EffectMetric::GreatestManaValue){return None;}
    let mut specs=Vec::new();let mut choices=Vec::new();
    let mut costs=spell.additional_non_mana_costs();
    costs.extend(spell.optional_costs.iter().enumerate().filter(|(i,_)|!selected_only||spell.optional_costs_paid.was_paid(*i)).flat_map(|(_,cost)|cost.cost.non_mana_costs().cloned()));
    for cost in costs{if let Some(effect)=cost.effect_ref(){collect_specs(effect,&mut specs);collect_cost_choices(effect,&mut choices);}}
    let mut scopes=Vec::new();
    for spec in specs{
        if spec.is_target() || spec.count_value().is_some() { return Some(0); }
        let (filter,count,tag)=match spec.base(){
            crate::target::ChooseSpec::Object(filter)=>(filter.clone(),spec.count(),None),
            crate::target::ChooseSpec::Tagged(tag)=>{
                let matching=choices.iter().filter(|choice|choice.tag==*tag).collect::<Vec<_>>();
                let [choice]=matching.as_slice()else{return Some(0);};
                if choice.count_value.is_some() || !choice.additional_zones.is_empty() { return Some(0); }
                (choice.filter.clone(),choice.count,Some(tag.clone()))
            },
            _=>return Some(0),
        };
        if let Some(query_filter)=&query.filter{
            if let [constraint]=query_filter.tagged_constraints.as_slice(){if tag.as_ref()!=Some(&constraint.tag){continue;}}
            else if !query_filter.tagged_constraints.is_empty(){return Some(0);}
        }
        scopes.push((filter,count));
    }
    // Without a scoped receipt, two exile operations do not identify which
    // cost produced "the exiled card". Do not sum unrelated instructions.
    let [(filter,count)]=scopes.as_slice()else{return Some(0);};
    let mut query_filter=query.filter.clone().unwrap_or_default();query_filter.tagged_constraints.clear();
    let ctx=game.filter_context_for(player,Some(spell.id));
    let mut values=game.objects_map().values().filter_map(|object|{
        if !filter.matches(object,&ctx,game)||!query_filter.matches(object,&ctx,game){return None;}
        Some(crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(object,game).mana_value() as i32)
    }).collect::<Vec<_>>();
    let maximum=count.max.unwrap_or(values.len()).min(values.len());
    if maximum<count.min||values.len()<count.min{return Some(0);}
    values.sort_unstable_by(|a,b|b.cmp(a));
    Some(if maximum==0{0}else if query.metric==EffectMetric::TotalManaValue{values.into_iter().take(maximum).sum()}else{values[0]})
}

//! Definition-local CR 607.2e note producer/reader relationship.
use crate::{ability::AbilityKind,cards::{CardDefinition,builders::CardTextError},effect::{Condition,Effect},target::ObjectFilter};
use ironsmith_core::{LinkedExileDefinition,LinkedExilePair,StaticAbilityPayload};
use sha2::{Digest,Sha256};
pub(super) struct GeneratedNameDefinition(LinkedExileDefinition);
impl GeneratedNameDefinition {pub(super) fn definition(&self)->LinkedExileDefinition{self.0}}
fn choices(effect:&Effect)->usize{
    let mut count=usize::from(effect.downcast_ref::<ironsmith_core::NoteObjectNamesEffect>().is_some());
    effect.visit_child_effects(&mut |effect|count+=choices(effect));count
}
fn filter(filter:&mut ObjectFilter,pair:LinkedExilePair)->bool{
    let mut found=false;
    if filter.name_noted_for.is_some(){filter.noted_name_pair=Some(pair);found=true;}
    for branch in &mut filter.any_of{found|=self::filter(branch,pair);}found
}
fn condition(condition:&mut Condition,pair:LinkedExilePair)->bool{
    match condition{
        Condition::TaggedObjectMatches(_,query)|Condition::TaggedObjectMatchedLastKnown(_,query)|Condition::TargetMatches(query)=>filter(query,pair),
        Condition::Not(inner)=>self::condition(inner,pair),
        Condition::And(left,right)|Condition::Or(left,right)=>self::condition(left,pair)|self::condition(right,pair),
        _=>false,
    }
}
fn effect(effect:Effect,pair:LinkedExilePair)->Result<Effect,CardTextError>{
    if let Some(note)=effect.downcast_ref::<ironsmith_core::NoteObjectNamesEffect>(){
        let mut note=note.clone();note.pair=Some(pair);return Ok(Effect::new(note));
    }
    if let Some(optional)=effect.downcast_ref::<crate::effects::MayEffect<Effect>>(){
        let mut optional=optional.clone();optional.effects=optional.effects.into_iter().map(|e|self::effect(e,pair)).collect::<Result<Vec<_>,_>>()?;return Ok(Effect::new(optional));
    }
    if let Some(sequence)=effect.downcast_ref::<crate::effects::SequenceEffect>(){
        let mut sequence=sequence.clone();sequence.effects=sequence.effects.into_iter().map(|e|self::effect(e,pair)).collect::<Result<Vec<_>,_>>()?;return Ok(Effect::new(sequence));
    }
    if let Some(branch)=effect.downcast_ref::<crate::effects::ConditionalEffect>(){
        let mut branch=branch.clone();condition(&mut branch.condition,pair);
        branch.if_true=branch.if_true.into_iter().map(|e|self::effect(e,pair)).collect::<Result<Vec<_>,_>>()?;
        branch.if_false=branch.if_false.into_iter().map(|e|self::effect(e,pair)).collect::<Result<Vec<_>,_>>()?;return Ok(Effect::new(branch));
    }
    // Discovery deliberately visits every child composition. Binding may only
    // accept wrappers reconstructed above; never admit a recursively found
    // producer whose inner pair would remain unstamped.
    if choices(&effect)>0{
        return Err(CardTextError::ParseError("noted-name producer requires a supported scoped effect wrapper".into()));
    }
    Ok(effect)
}
fn apply(definition:&mut CardDefinition,pair:LinkedExilePair)->Result<(),CardTextError>{
    for ability in &mut definition.abilities{
        let (program,reads)=match &mut ability.kind{
            AbilityKind::Triggered(trigger)=>{
                let reads=trigger.intervening_if.as_mut().is_some_and(|c|condition(c,pair));
                (Some(&mut trigger.effects),reads)
            },
            AbilityKind::Activated(activation)=>{
                let reads=activation.activation_condition.as_mut().is_some_and(|c|condition(c,pair));
                (Some(&mut activation.effects),reads)
            },
            AbilityKind::Static(ability)=>{
                match &mut ability.payload{
                    StaticAbilityPayload::CostReductionManaCost(reduction)=>{filter(&mut reduction.filter,pair);},
                    _=>{},
                }
                (None,false)
            },
        };
        if let Some(program)=program{
            let writes=program.all_effects().into_iter().map(choices).sum::<usize>()>0;
            if writes||reads{
                if program.source_number_pair.is_some_and(|existing|existing!=pair){return Err(CardTextError::ParseError("one program requires independently linked noted-information acquisitions; supply an explicit scoped construction".into()));}
                program.source_number_pair=Some(pair);
                *program=program.clone().try_map_effects(|e|effect(e,pair))?;
            }
        }
    }Ok(())
}
pub(super) fn bind_source_name_pair(definition:&mut CardDefinition,previous:&[LinkedExileDefinition])->Result<Option<GeneratedNameDefinition>,CardTextError>{
    let producers=definition.abilities.iter().enumerate().filter_map(|(slot,ability)|{
        let count=match &ability.kind{
            AbilityKind::Triggered(ability)=>ability.effects.all_effects().into_iter().map(choices).sum::<usize>(),
            AbilityKind::Activated(ability)=>ability.effects.all_effects().into_iter().map(choices).sum::<usize>(),
            _=>0,
        };(count>0).then_some((slot,count))
    }).collect::<Vec<_>>();
    if producers.is_empty(){return Ok(None);}
    let [(slot,1)]=producers.as_slice() else{return Err(CardTextError::ParseError("noted-name readers require one unambiguous linked note producer".into()));};
    let provisional=LinkedExileDefinition(Sha256::digest(b"ironsmith-source-name-construction-v1\0").into());
    let pair=LinkedExilePair{definition:provisional,pair:*slot as u32};
    // Refinalization may replace this exact typed producer's previous stamp;
    // it cannot clear an independently owned numeric relationship.
    fn previous_note_pair(effect:&Effect)->Option<LinkedExilePair>{
        if let Some(note)=effect.downcast_ref::<ironsmith_core::NoteObjectNamesEffect>(){return note.pair;}
        let mut found=None;effect.visit_child_effects(&mut |child|{found=found.or_else(||previous_note_pair(child));});found
    }
    let prior=match &definition.abilities[*slot].kind{
        AbilityKind::Triggered(a)=>a.effects.all_effects().into_iter().find_map(previous_note_pair),
        AbilityKind::Activated(a)=>a.effects.all_effects().into_iter().find_map(previous_note_pair),_=>None,
    };
    if let Some(prior)=prior{for ability in &mut definition.abilities{
        let program=match &mut ability.kind{AbilityKind::Triggered(a)=>Some(&mut a.effects),AbilityKind::Activated(a)=>Some(&mut a.effects),_=>None};
        if let Some(program)=program{if program.source_number_pair==Some(prior){program.source_number_pair=None;}}
    }}
    apply(definition,pair)?;
    let mut generated=super::first_draw_reveals::compiler_pair_definitions(definition)?;
    generated.extend_from_slice(previous);generated.push(provisional);
    let namespace=super::trigger_definitions::authored_root_namespace_with_generated(definition,&generated)?
        .ok_or_else(||CardTextError::InvariantViolation("noted-name definition contains an unproven opaque identity namespace".into()))?;
    let mut hash=Sha256::new();hash.update(b"ironsmith-source-name-definition-v1\0");hash.update(namespace);
    let stamp=LinkedExileDefinition(hash.finalize().into());
    // The second pass replaces only this construction's provisional acquisition.
    for ability in &mut definition.abilities{
        match &mut ability.kind{
            AbilityKind::Triggered(a) if a.effects.source_number_pair==Some(pair)=>a.effects.source_number_pair=None,
            AbilityKind::Activated(a) if a.effects.source_number_pair==Some(pair)=>a.effects.source_number_pair=None,
            _=>{},
        }
    }
    apply(definition,LinkedExilePair{definition:stamp,..pair})?;
    Ok(Some(GeneratedNameDefinition(stamp)))
}

#[cfg(test)]
mod tests{
    use super::*;
    fn pair()->LinkedExilePair{LinkedExilePair{definition:LinkedExileDefinition([47;32]),pair:0}}
    fn note()->Effect{Effect::new(ironsmith_core::NoteObjectNamesEffect{pair:None,
        object:crate::target::ChooseSpec::Source,for_object:crate::target::ChooseSpec::Source})}
    #[test]
    fn discovery_cannot_admit_an_unstamped_producer_under_an_unsupported_wrapper(){
        let wrapped=Effect::new(ironsmith_core::TaggedEffect{tag:"named-note".into(),effect:Box::new(note()),outcome_only:true,action:None});
        assert_eq!(choices(&wrapped),1);
        assert!(matches!(effect(wrapped,pair()),Err(CardTextError::ParseError(_))));
    }
    #[test]
    fn accepted_optional_producer_is_stamped_recursively(){
        let optional=Effect::new(crate::effects::MayEffect::<Effect>{
            payment_receipt:None,decider:Some(ironsmith_core::PlayerFilter::You),effects:vec![note()],pay_as_cost:false,
        });
        let bound=effect(optional,pair()).unwrap();
        let mut observed=0;
        bound.visit_child_effects(&mut |child|{
            if let Some(note)=child.downcast_ref::<ironsmith_core::NoteObjectNamesEffect>(){
                assert_eq!(note.pair,Some(pair()));observed+=1;
            }
        });
        assert_eq!(observed,1);
    }
}

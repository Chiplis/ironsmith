//! Definition-local numeric links, using the shared immutable ability-pair key.
use crate::ability::AbilityKind;
use crate::cards::{CardDefinition, builders::CardTextError};
use crate::effect::{Effect, Value};
use crate::resolution::ResolutionProgram;
use crate::target::ObjectFilter;
use ironsmith_core::{FilterComparison as Comparison, LinkedExileDefinition, LinkedExilePair, StaticAbilityPayload, TriggerKind};
use sha2::{Digest, Sha256};

/// Ephemeral compiler proof for the numeric construction namespace. Native or
/// runtime acquisitions cannot manufacture it from an equal-looking pair.
pub(super) struct GeneratedSourceNumberDefinition(LinkedExileDefinition);
impl GeneratedSourceNumberDefinition {
    pub(super) fn definition(&self) -> LinkedExileDefinition { self.0 }
}

fn source_choices(effect: &Effect, count: &mut usize) {
    if effect.downcast_ref::<crate::effects::ChooseNumberEffect>().is_some_and(|choice| choice.source_owned) { *count += 1; }
    effect.visit_child_effects(&mut |child| source_choices(child,count));
}
fn program_choices(program: &ResolutionProgram) -> usize {
    let mut count=0;for effect in program.all_effects(){source_choices(effect,&mut count);}count
}
fn bind_value(value: &mut Value, pair: LinkedExilePair) -> usize {
    match value {
        Value::SourceChosenNumber { pair: binding, .. } => { *binding=Some(pair);1 }
        Value::SurfaceHinted {value,..}|Value::Scaled(value,_)|Value::DividedRoundedDown(value,_)|Value::HalfRoundedDown(value)=>bind_value(value,pair),
        Value::Add(a,b)|Value::Min(a,b)=>bind_value(a,pair)+bind_value(b,pair),
        _=>0,
    }
}
fn bind_comparison(comparison: &mut Option<Comparison>, pair: LinkedExilePair) -> usize {
    match comparison {
        Some(Comparison::EqualExpr(value)|Comparison::NotEqualExpr(value)|Comparison::LessThanExpr(value)
            |Comparison::LessThanOrEqualExpr(value)|Comparison::GreaterThanExpr(value)|Comparison::GreaterThanOrEqualExpr(value))=>bind_value(value,pair),
        _=>0,
    }
}
fn bind_filter(filter: &mut ObjectFilter, pair: LinkedExilePair) -> usize {
    let mut count=bind_comparison(&mut filter.mana_value,pair)+bind_comparison(&mut filter.power,pair)+bind_comparison(&mut filter.toughness,pair);
    for branch in &mut filter.any_of {count+=bind_filter(branch,pair);}count
}
fn last_choice_cda(value:&Value)->bool{
    match value {
        Value::SourceChosenNumber{if_unset:Some(0),..}=>true,
        Value::SurfaceHinted{value,..}|Value::Scaled(value,_)=>last_choice_cda(value),
        Value::Add(a,b)=>last_choice_cda(a)||last_choice_cda(b),
        _=>false,
    }
}
/// One entry producer establishes the definition-local numeric relationship.
/// Reader and upkeep membership is determined from typed source-owned values,
/// never from card names, text labels, or the number selected at runtime.
pub(super) fn bind_source_number_pair(definition: &mut CardDefinition) -> Result<Option<GeneratedSourceNumberDefinition>,CardTextError> {
    let entries=definition.abilities.iter().enumerate().filter_map(|(slot,ability)| {
        let AbilityKind::Static(ability)=&ability.kind else{return None};
        let StaticAbilityPayload::AsEntersEffectProgram{program,..}=&ability.payload else{return None};
        (program_choices(program)>0).then_some((slot,program_choices(program)))
    }).collect::<Vec<_>>();
    if entries.is_empty(){return Ok(None)}
    let [(producer,1)]=entries.as_slice() else{return Err(CardTextError::ParseError("source numeric choices require an unambiguous entry pair".into()))};
    let has_last_choice=definition.abilities.iter().any(|ability|matches!(&ability.kind,
        AbilityKind::Static(ability) if matches!(&ability.payload,
            StaticAbilityPayload::CharacteristicDefiningPt{power,toughness} if last_choice_cda(power)||last_choice_cda(toughness))));
    if !has_last_choice && definition.abilities.iter().any(|ability|matches!(&ability.kind,
        AbilityKind::Triggered(ability) if program_choices(&ability.effects)>0)) {
        return Err(CardTextError::ParseError("numeric upkeep reselection requires an explicit last-chosen-number reader".into()));
    }
    // Rebind only this typed construction's entry, readers and reselection.
    // This stable provisional namespace stays distinct from the first-draw
    // placeholder; graph normalization retains that independent relationship.
    let provisional = LinkedExileDefinition(Sha256::digest(b"ironsmith-source-number-construction-v1\0").into());
    let pair = LinkedExilePair { definition: provisional, pair: *producer as u32 };
    apply_pair(definition, pair);
    let mut generated = super::first_draw_reveals::compiler_pair_definitions(definition)?;
    generated.push(provisional);
    let bytes = super::trigger_definitions::authored_root_namespace_with_generated(definition, &generated)?
        .ok_or_else(|| CardTextError::InvariantViolation("numeric definition contains an unproven opaque identity namespace".into()))?;
    let mut digest = Sha256::new();
    digest.update(b"ironsmith-source-number-definition-v2\0");
    digest.update(bytes);
    let stamp = LinkedExileDefinition(digest.finalize().into());
    apply_pair(definition, LinkedExilePair { definition: stamp, ..pair });
    Ok(Some(GeneratedSourceNumberDefinition(stamp)))
}

fn apply_pair(definition: &mut CardDefinition, pair: LinkedExilePair) {
    for ability in &mut definition.abilities {
        match &mut ability.kind {
            AbilityKind::Static(ability)=>match &mut ability.payload {
                StaticAbilityPayload::AsEntersEffectProgram{program,..} if program_choices(program)>0=>program.source_number_pair=Some(pair),
                StaticAbilityPayload::CharacteristicDefiningPt{power,toughness}=>{bind_value(power,pair);bind_value(toughness,pair);}
                StaticAbilityPayload::RuleRestriction{restriction:crate::effect::Restriction::CastSpellsMatching(_,filter),..}=>{bind_filter(filter,pair);}
                _=>{}
            },
            AbilityKind::Triggered(ability)=>{
                let reads=match &mut ability.trigger.kind {
                    TriggerKind::SpellCast{filter:Some(filter),..}|TriggerKind::SpellCastQualified{filter:Some(filter),..}=>bind_filter(filter,pair),
                    _=>0,
                };
                if reads>0 || program_choices(&ability.effects)>0 {ability.effects.source_number_pair=Some(pair);}
            }
            _=>{}
        }
    }
}

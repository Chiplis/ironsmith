//! Immutable lexical real-card catalogs retained by programs, not game globals.
use crate::{CardDefinition,Effect,GameState};
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError,CompletedEffectOutputs};
use crate::effect::EffectOutcome;
use ironsmith_core::CardId;

pub type CardDefinitionScopeEffect=ironsmith_core::card_creation::CardDefinitionScopeEffect<CardDefinition,Effect>;
#[derive(Debug,Clone)]
pub(crate) struct CardDefinitionScope {pub scope:CardId,pub definitions:Vec<CardDefinition>}

fn uses_scope(effect:&Effect,scope:CardId)->bool{
    if effect.downcast_ref::<CardDefinitionScopeEffect>().is_some(){return false;}
    if let Some(create)=effect.downcast_ref::<crate::effects::CreateCardsEffect>() {
        return create.definition_scope==Some(scope) && create.definitions.is_empty();
    }
    let mut found=false;
    effect.visit_child_effects(&mut |child|found|=uses_scope(child,scope));
    if !found { effect.visit_card_definitions(&mut |definition| found |= definition_uses_scope(definition,scope)); }

    found
}
fn definition_uses_scope(definition:&CardDefinition,scope:CardId)->bool {
    use std::cell::Cell;
    let found=Cell::new(false);
    fn inspect(effect:Effect,scope:CardId,found:&Cell<bool>)->Result<Effect,std::convert::Infallible>{
        found.set(found.get()||uses_scope(&effect,scope));Ok(effect)
    }
    fn cost(cost:crate::costs::Cost,scope:CardId,found:&Cell<bool>)->Result<crate::costs::Cost,std::convert::Infallible>{
        if let Some(model)=cost.compiled_model(){let _=model.clone().try_map_effect(|effect|inspect(effect,scope,found));}
        Ok(cost)
    }
    fn ability(ability:crate::static_abilities::StaticAbility,scope:CardId,found:&Cell<bool>)->Result<crate::static_abilities::StaticAbility,std::convert::Infallible>{
        if let Some(model)=ability.canonical_model(){let _=model.try_map(Ok::<_,std::convert::Infallible>,|effect|inspect(effect,scope,found),|value|cost(value,scope,found),Ok::<_,std::convert::Infallible>);}
        else if let Some(template)=ability.supplied_replacement_definition(){found.set(found.get()||uses_scope(&template,scope));}
        Ok(ability)
    }
    let _=definition.clone().try_map(
        |value|value.try_map(|value|ability(value,scope,&found),Ok::<_,std::convert::Infallible>,|effect|inspect(effect,scope,&found),|value|cost(value,scope,&found),Ok::<_,std::convert::Infallible>),
        |effect|inspect(effect,scope,&found),|value|cost(value,scope,&found),
        |method|method.try_map(|effect|inspect(effect,scope,&found),|value|cost(value,scope,&found)),
        |value|value.try_map(|value|cost(value,scope,&found)));
    found.get()
}
pub(crate) fn wrap_effect(effect:Effect,scope:&CardDefinitionScope)->Effect {
    if !uses_scope(&effect,scope.scope){return effect;}
    Effect::new(CardDefinitionScopeEffect {scope:scope.scope,definitions:scope.definitions.clone(),effect:Box::new(effect)})
}

fn wrap_cost(cost:crate::costs::Cost,scope:&CardDefinitionScope)->Result<crate::costs::Cost,String>{
    let Some(model)=cost.compiled_model().cloned() else{return Ok(cost)};
    // Ordinary payment primitives keep their cost-executable capability.
    if !model.effect_ref().is_some_and(|effect|uses_scope(effect,scope.scope)){return Ok(cost);}
    crate::costs::Cost::from_model(model.try_map_effect(|effect|Ok::<_,String>(wrap_effect(effect,scope)))?)
}
fn wrap_static(ability:crate::static_abilities::StaticAbility,scope:&CardDefinitionScope)->Result<crate::static_abilities::StaticAbility,String>{
    let Some(model)=ability.canonical_model() else{
        if ability.supplied_replacement_definition().is_some(){return Err("source-owned supplied definition carrier has no canonical model for lexical binding".into());}
        return Ok(ability);
    };
    Ok(crate::static_abilities::StaticAbility::from_model(model.try_map(
        Ok::<_,String>,|effect|Ok(wrap_effect(effect,scope)),|cost|wrap_cost(cost,scope),Ok::<_,String>)?))
}
pub(crate) fn wrap_ability(ability:crate::Ability,scope:&CardDefinitionScope)->Result<crate::Ability,String>{
    ability.try_map(|ability|wrap_static(ability,scope),Ok::<_,String>,|effect|Ok(wrap_effect(effect,scope)),|cost|wrap_cost(cost,scope),Ok::<_,String>)
}
/// Bind and validate an owned native program without an artifact interpreter.
pub fn bind_card_creation_scope_runtime(definition:CardDefinition,scope:CardId,nodes:&[CardDefinition])->Result<CardDefinition,String>{
    let bound=bind_native_definition(definition,scope,nodes)?;
    validate_native_definition(&bound)?;
    Ok(bound)
}
pub(crate) use super::native_creation_graph::{validate_native_definition,validate_native_scope_nodes};
fn bind_native_definition(definition:CardDefinition,scope:CardId,nodes:&[CardDefinition])->Result<CardDefinition,String>{
    validate_native_scope_nodes(scope,nodes)?;
    let scope=CardDefinitionScope {scope,definitions:nodes.to_vec()};
    definition.try_map(|ability|wrap_ability(ability,&scope),
        |effect|Ok(wrap_effect(effect,&scope)),|cost|wrap_cost(cost,&scope),
        |method|method.try_map(|effect|Ok(wrap_effect(effect,&scope)),|cost|wrap_cost(cost,&scope)),
        |cost|cost.try_map(|cost|wrap_cost(cost,&scope)))
}

impl EffectExecutor for CardDefinitionScopeEffect {
    fn transparent_child_effect(&self)->Option<&Effect>{Some(&self.effect)}
    fn mana_production(&self)->Option<crate::mana_payment::program::ManaProduction<'_>>{self.effect.0.mana_production()}
    fn directly_produces_mana(&self)->bool{self.effect.0.directly_produces_mana()}
    fn producible_mana_symbols(&self,game:&GameState,source:crate::ObjectId,controller:crate::PlayerId)->Option<Vec<crate::mana::ManaSymbol>>{
        self.effect.0.producible_mana_symbols(game,source,controller)
    }
    fn visit_child_effects(&self,visitor:&mut dyn FnMut(&Effect)){visitor(&self.effect)}
    fn visit_card_definitions(&self,visitor:&mut dyn FnMut(&CardDefinition)){for definition in &self.definitions{visitor(definition)}}
    fn get_target_spec(&self)->Option<&crate::target::ChooseSpec>{self.effect.0.get_target_spec()}
    fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError>{
        self.execute_with_outputs(game,ctx).map(CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<CompletedEffectOutputs,ExecutionError>{
        let previous=ctx.card_definition_scopes.clone();
        ctx.card_definition_scopes.retain(|scope|scope.scope!=self.scope);
        ctx.card_definition_scopes.push(CardDefinitionScope {scope:self.scope,definitions:self.definitions.clone()});
        let result=self.effect.0.execute_child_with_outputs(game,ctx);
        ctx.card_definition_scopes=previous;
        result
    }
}
impl ExecutionContext<'_>{
    pub(crate) fn bind_definition_scope_definition(&self,mut definition:CardDefinition)->Result<CardDefinition,ExecutionError>{
        if self.card_definition_scopes.is_empty(){return Ok(definition);}
        for scope in &self.card_definition_scopes{definition=bind_native_definition(definition,scope.scope,&scope.definitions).map_err(ExecutionError::IncompleteEvidence)?;}
        validate_native_definition(&definition).map_err(ExecutionError::IncompleteEvidence)?;
        Ok(definition)
    }
    pub(crate) fn bind_definition_scope_program(&self,program:crate::resolution::ResolutionProgram)->crate::resolution::ResolutionProgram{
        program.try_map_effects(|mut effect|{for scope in &self.card_definition_scopes{effect=wrap_effect(effect,scope);}Ok::<_,std::convert::Infallible>(effect)}).unwrap_or_else(|never|match never{})
    }
    pub(crate) fn bind_definition_scope_modification(&self,mut modification:crate::continuous::Modification)->Result<crate::continuous::Modification,ExecutionError>{
        for scope in &self.card_definition_scopes{
            modification=modification.try_map_payloads(|ability|wrap_static(ability,scope),|ability|wrap_ability(ability,scope),Ok::<_,String>,Ok::<_,String>,Ok::<_,String>,Ok::<_,String>).map_err(ExecutionError::IncompleteEvidence)?;
        }
        Ok(modification)
    }
    pub(crate) fn bind_definition_scope_grantable(&self,mut grant:crate::grant::Grantable)->Result<crate::grant::Grantable,ExecutionError>{
        for scope in &self.card_definition_scopes{grant=grant.try_map(|ability|wrap_static(ability,scope),|effect|Ok(wrap_effect(effect,scope)),|cost|wrap_cost(cost,scope)).map_err(ExecutionError::IncompleteEvidence)?;}
        Ok(grant)
    }
    pub(crate) fn bind_definition_scope_grant_spec(&self,mut spec:crate::grant::GrantSpec)->Result<crate::grant::GrantSpec,ExecutionError>{
        for scope in &self.card_definition_scopes{spec=spec.try_map(|ability|wrap_static(ability,scope),|effect|Ok(wrap_effect(effect,scope)),|cost|wrap_cost(cost,scope)).map_err(ExecutionError::IncompleteEvidence)?;}
        Ok(spec)
    }
    pub(crate) fn bind_definition_scope_ability(&self,mut ability:crate::Ability)->Result<crate::Ability,ExecutionError>{
        for scope in &self.card_definition_scopes{ability=wrap_ability(ability,scope).map_err(ExecutionError::IncompleteEvidence)?;}
        Ok(ability)
    }
}

/// Freeze a source-owned replacement template with the lexical operands carried
/// by transparent scopes. The provider's scope never comes from its caller.
pub(crate) fn scoped_creation_template(template:&Effect)->Result<Option<crate::effects::CreateCardsEffect>,ExecutionError>{
    let mut operand=template;let mut scopes=Vec::new();
    loop {
        if let Some(scope)=operand.downcast_ref::<CardDefinitionScopeEffect>() {
            scopes.push(scope);operand=&scope.effect;continue;
        }
        if let Some(child)=operand.transparent_child_effect(){operand=child;continue;}
        break;
    }
    let Some(create)=operand.downcast_ref::<crate::effects::CreateCardsEffect>() else{return Ok(None)};
    let mut create=create.clone();
    if let Some(scope)=create.definition_scope && create.definitions.is_empty(){
        let catalog=scopes.into_iter().rev().find(|catalog|catalog.scope==scope)
            .ok_or_else(||ExecutionError::IncompleteEvidence("replacement template has no lexical catalog".into()))?;
        create.definitions=catalog.definitions.clone();
    }
    Ok(Some(create))
}

/// Resolve exactly one source-owned supplied operand before replacement choices.
pub(crate) fn replacement_template_definition(template:&Effect)->Result<CardDefinition,ExecutionError>{
    use super::CardCreationDefinitionGraphExt;
    if let Some(create)=scoped_creation_template(template)? {
        let ironsmith_core::CardCreationSelection::Named(name)=&create.selection else{
            return Err(ExecutionError::IncompleteEvidence("replacement creation template requires one explicit supplied name".into()));
        };
        let definitions=create.admitted_definitions()?;
        let mut found=definitions.into_iter().filter(|definition|definition.card.name.trim().eq_ignore_ascii_case(name.trim()));
        let selected=found.next().ok_or_else(||ExecutionError::IncompleteEvidence("replacement template has no admitted selected identity".into()))?;
        if found.next().is_some(){return Err(ExecutionError::IncompleteEvidence("replacement template has ambiguous selected identities".into()));}
        return create.close_generated_definition(selected.clone());
    }
    let mut operand=template;let mut scopes=Vec::new();
    loop{
        if let Some(scope)=operand.downcast_ref::<CardDefinitionScopeEffect>(){scopes.push(scope);operand=&scope.effect;continue;}
        if let Some(child)=operand.transparent_child_effect(){operand=child;continue;}
        break;
    }
    let Some(copy)=operand.downcast_ref::<crate::effects::CopyFromDefinitionEffect>() else{
        return Err(ExecutionError::IncompleteEvidence("replacement source has no typed supplied definition operand".into()));
    };
    let mut definition=copy.definition.clone();
    for scope in scopes.into_iter().rev(){definition=bind_card_creation_scope_runtime(definition,scope.scope,&scope.definitions).map_err(ExecutionError::IncompleteEvidence)?;}
    Ok(definition)
}

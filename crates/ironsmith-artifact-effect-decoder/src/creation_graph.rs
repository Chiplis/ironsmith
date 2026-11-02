//! Finite, explicitly admitted real-card catalogs. Nodes may refer to one
//! another by identity but never recursively embed their catalog.
use std::collections::{HashMap, HashSet};
use ironsmith_compiled_artifact::WireCardDefinition;
use ironsmith_core::{CardId, CreateCardsEffect};
type Scope=ironsmith_core::card_creation::CardDefinitionScopeEffect<WireCardDefinition,ironsmith_compiled_artifact::WireEffect>;
use serde_json::Value;

type Creation = CreateCardsEffect<WireCardDefinition>;
type Catalogs = HashMap<CardId, Vec<WireCardDefinition>>;

fn visit(value: &Value, f: &mut impl FnMut(&Creation) -> Result<(), String>) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            if object.get("kind").and_then(Value::as_str) == Some("CreateCardsEffect") {
                let payload = object.get("payload").ok_or("missing real-card creation payload")?;
                let creation: Creation = serde_json::from_value(payload.clone()).map_err(|e| e.to_string())?;
                if creation.definition_refs.iter().collect::<HashSet<_>>().len()!=creation.definition_refs.len() {
                    return Err("duplicate identity in real-card reference subset".into());
                }
                f(&creation)?;
            }
            if object.get("kind").and_then(Value::as_str)==Some("CardDefinitionScopeEffect") {
                let scope:Scope=serde_json::from_value(object.get("payload").ok_or("missing lexical catalog payload")?.clone()).map_err(|e|e.to_string())?;
                let refs=scope.definitions.iter().map(|node|node.card.id).collect();
                let declaration=Creation {definitions:scope.definitions,definition_scope:Some(scope.scope),definition_refs:refs,
                    operation_label:None,selection:ironsmith_core::CardCreationSelection::Named(String::new()),count:0.into(),
                    player:ironsmith_core::PlayerFilter::You,zone:ironsmith_core::Zone::Hand,library_position:ironsmith_core::CardCreationLibraryPosition::Top};
                f(&declaration)?;
            }
            for child in object.values() { visit(child, f)?; }
        },
        Value::Array(values) => for child in values { visit(child, f)?; },
        _ => {},
    }
    Ok(())
}

pub fn validate_card_creation_scope_nodes(scope:CardId,nodes:&[WireCardDefinition])->Result<(),String>{
    admit(scope,nodes)
}

fn admit(scope: CardId, nodes: &[WireCardDefinition]) -> Result<(), String> {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for node in nodes {
        if node.card.name.trim().is_empty(){return Err("real-card catalog identity has no name".into());}
        if node.card.is_token { return Err("real-card catalog contains a token recipe".into()); }
        if !ids.insert(node.card.id) { return Err("duplicate identity in real-card catalog".into()); }
        if !names.insert(node.card.name.trim().to_lowercase()) { return Err("ambiguous name in real-card catalog".into()); }
    }
    if !ids.contains(&scope) { return Err("real-card catalog scope is not a declared identity".into()); }
    for node in nodes {
        visit(&serde_json::to_value(node).map_err(|e|e.to_string())?, &mut |creation| {
            if creation.definition_scope != Some(scope) || !creation.definitions.is_empty() {
                return Err("real-card graph member must use shallow references to its own catalog".into());
            }
            if creation.definition_refs.iter().any(|id|!ids.contains(id)) {
                return Err("real-card graph reference escapes its admitted catalog".into());
            }
            Ok(())
        })?;
    }
    Ok(())
}

/// Validate the entire executable definition, including declarations found on
/// different abilities. A dangling reference never falls back to a global name.
pub fn validate_card_creation_graph(definition: &WireCardDefinition) -> Result<(), String> {
    validate_graph_value(serde_json::to_value(definition).map_err(|e|e.to_string())?)
}

/// Standalone effect export has the same closure requirement as a card.
pub fn validate_card_creation_effect_graph(effect:&ironsmith_compiled_artifact::WireEffect)->Result<(),String>{
    validate_graph_value(serde_json::to_value(effect).map_err(|e|e.to_string())?)
}

fn validate_graph_value(value:Value)->Result<(),String>{
    fn check(value:&Value,scopes:&Catalogs)->Result<(),String>{
        match value {
            Value::Object(object)=>{
                if object.get("kind").and_then(Value::as_str)==Some("CardDefinitionScopeEffect") {
                    let scope:Scope=serde_json::from_value(object.get("payload").ok_or("missing lexical catalog payload")?.clone()).map_err(|e|e.to_string())?;
                    admit(scope.scope,&scope.definitions)?;
                    let mut local=scopes.clone();local.insert(scope.scope,scope.definitions);
                    return check(&serde_json::to_value(scope.effect).map_err(|e|e.to_string())?,&local);
                }
                if object.get("kind").and_then(Value::as_str)==Some("CreateCardsEffect") {
                    let creation:Creation=serde_json::from_value(object.get("payload").ok_or("missing real-card creation payload")?.clone()).map_err(|e|e.to_string())?;
                    if creation.definition_refs.iter().collect::<HashSet<_>>().len()!=creation.definition_refs.len(){return Err("duplicate identity in real-card reference subset".into());}
                    if let Some(scope)=creation.definition_scope {
                        let declared;
                        let nodes=if creation.definitions.is_empty(){scopes.get(&scope).ok_or("real-card reference has no lexical catalog declaration")?}
                        else{admit(scope,&creation.definitions)?;declared=creation.definitions;&declared};
                        if creation.definition_refs.iter().any(|id|!nodes.iter().any(|node|node.card.id==*id)){return Err("real-card reference escapes its admitted catalog".into());}
                    }else{
                        if !creation.definition_refs.is_empty(){return Err("real-card references require an explicit catalog scope".into());}
                        // Legacy embedded identities must themselves be closed.
                        for node in creation.definitions{check(&serde_json::to_value(node).map_err(|e|e.to_string())?,&Catalogs::new())?;}
                    }
                    return Ok(());
                }
                for child in object.values(){check(child,scopes)?;}
            },
            Value::Array(values)=>for child in values{check(child,scopes)?;},
            _=>{},
        }
        Ok(())
    }
    check(&value,&Catalogs::new())
}

/// A generated node carries its closure when exported, copied, or restarted.
/// Fill only that node's programs, never the shallow catalog members inserted
/// into them; cycles therefore have finite wire representations.
pub fn bind_card_creation_scope(
    definition: WireCardDefinition, scope: CardId, nodes: &[WireCardDefinition],
) -> Result<WireCardDefinition,String> {
    admit(scope,nodes)?;
    fn bind(value:&mut Value,scope:CardId,nodes:&[WireCardDefinition])->Result<(),String>{
        match value {
            Value::Object(object)=>{
                if object.get("kind").and_then(Value::as_str)==Some("CardDefinitionScopeEffect") {
                    let declared:Scope=serde_json::from_value(object.get("payload").ok_or("missing lexical catalog payload")?.clone()).map_err(|e|e.to_string())?;
                    // A closed nested program owns its lexical declaration,
                    // including when another artifact used the same local ID.
                    let _=declared;
                    return Ok(());
                }
                if object.get("kind").and_then(Value::as_str)==Some("CreateCardsEffect") {
                    let payload=object.get_mut("payload").ok_or("missing real-card creation payload")?;
                    let mut creation:Creation=serde_json::from_value(payload.clone()).map_err(|e|e.to_string())?;
                    if creation.definition_scope==Some(scope) {
                        if !creation.definitions.is_empty(){return Ok(());}
                        creation.definitions=nodes.to_vec();
                        *payload=serde_json::to_value(creation).map_err(|e|e.to_string())?;
                    }
                    // Catalog members are data dependencies, not programs of
                    // the root currently being hydrated. Preserve other scopes.
                    return Ok(());
                }
                for child in object.values_mut(){bind(child,scope,nodes)?;}
            },
            Value::Array(values)=>for child in values{bind(child,scope,nodes)?;},
            _=>{},
        }
        Ok(())
    }
    let mut value=serde_json::to_value(definition).map_err(|e|e.to_string())?;
    bind(&mut value,scope,nodes)?;
    let bound=serde_json::from_value(value).map_err(|e|e.to_string())?;
    validate_card_creation_graph(&bound)?;
    Ok(bound)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_definition_references_use_the_existing_typed_card_id_mapper() {
        let node=WireCardDefinition::new(ironsmith_core::card::CardBuilder::new(CardId(41),"Scoped card").build());
        let creation=Creation {definitions:vec![node],definition_scope:Some(CardId(41)),definition_refs:vec![CardId(41)],
            operation_label:None,selection:ironsmith_core::CardCreationSelection::Named("Scoped card".into()),
            count:1.into(),player:ironsmith_core::PlayerFilter::You,zone:ironsmith_core::Zone::Hand,
            library_position:ironsmith_core::CardCreationLibraryPosition::Top};
        let (normalized,_)=crate::authored_definition_graph(&creation,&mut |id|Ok(id+1000),&[]).unwrap();
        assert_eq!(normalized["definitions"][0]["card"]["id"],serde_json::json!(1041));
        assert_eq!(normalized["definition_scope"],serde_json::json!(1041));
        assert_eq!(normalized["definition_refs"],serde_json::json!([1041]));
        assert!(normalized["definitions"][0].get("canonical_text").is_none(),"authored identity deliberately excludes presentation; it is not a materialization envelope");
        // The lossless mapper owns a typed runtime transport roundtrip.
        let value=crate::remap_card_ids(&creation,&mut |id|Ok(id+1000)).unwrap();
        let mapped:Creation=serde_json::from_value(value).unwrap();
        assert_eq!(mapped.definitions[0].card.id,CardId(1041));
        assert_eq!(mapped.definition_scope,Some(CardId(1041)));
        assert_eq!(mapped.definition_refs,vec![CardId(1041)]);
        let scope=Scope {scope:CardId(41),definitions:creation.definitions.clone(),effect:Box::new(ironsmith_compiled_artifact::WireEffect::new("CreateCardsEffect",serde_json::to_value(Creation{definitions:Vec::new(),..creation}).unwrap()))};
        let effect=ironsmith_compiled_artifact::WireEffect::new("CardDefinitionScopeEffect",serde_json::to_value(scope).unwrap());
        let (normalized,_)=crate::authored_definition_graph(&effect,&mut |id|Ok(id+2000),&[]).unwrap();
        assert_eq!(normalized["payload"]["scope"],serde_json::json!(2041));
        assert_eq!(normalized["payload"]["definitions"][0]["card"]["id"],serde_json::json!(2041));
        assert_eq!(normalized["payload"]["effect"]["payload"]["definition_refs"],serde_json::json!([2041]));
        let value=crate::remap_card_ids(&effect,&mut |id|Ok(id+2000)).unwrap();
        let mapped:ironsmith_compiled_artifact::WireEffect=serde_json::from_value(value).unwrap();
        let scope:Scope=serde_json::from_value(mapped.payload().clone()).unwrap();
        assert_eq!(scope.scope,CardId(2041));
        assert_eq!(scope.definitions[0].card.id,CardId(2041));
        let create:Creation=serde_json::from_value(scope.effect.payload().clone()).unwrap();
        assert_eq!(create.definition_refs,vec![CardId(2041)]);

    }
}

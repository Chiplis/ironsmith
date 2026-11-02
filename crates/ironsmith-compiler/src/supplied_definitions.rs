//! Strict compilation of explicitly supplied real-card identities.
use crate::{CardDefinitionBuilder,CompilerFacade,CompilePolicy};
/// A supplied definition is never resolved through the official catalog and
/// never compiled as a token, even when its source uses a checklist print.
pub fn compile_supplied_card_definition(name:&str,text:&str,identities:&[String],payments:&[String],subtypes:&[crate::CustomSubtypeDefinition])->Result<crate::cards::CardDefinition,String>{
 compile_supplied_card_definition_at_id(crate::ids::CardId::new(),name,text,identities,payments,subtypes)
}
fn compile_supplied_card_definition_at_id(id:crate::ids::CardId,name:&str,text:&str,identities:&[String],payments:&[String],subtypes:&[crate::CustomSubtypeDefinition])->Result<crate::cards::CardDefinition,String>{
 if name.trim().is_empty(){return Err("supplied real-card definition needs a name".into());}
 if text.lines().any(|line|line.split_once(':').is_some_and(|(heading,value)|heading.trim().eq_ignore_ascii_case("type")&&value.split_whitespace().next().is_some_and(|word|word.eq_ignore_ascii_case("token")))) {
  return Err("supplied real-card definition cannot have a Token type header".into());
 }
 crate::with_custom_subtypes(subtypes,||{
  let mut builder=CardDefinitionBuilder::new(id,name);
  for identity in identities{builder=builder.with_custom_keyword(identity.clone());}
  let mut definition=crate::word_claims::with_enforcement_options(true,true,||CompilerFacade::new().compile_definition(builder,text,CompilePolicy{allow_unsupported:false})).map_err(|error|format!("supplied real-card {name}: {error}"))?.definition;
  for payment in payments{if !definition.attribute_custom_keyword_payment(payment){return Err(format!("supplied real-card {name} has no unique attributable payment {payment}"));}}
  Ok(definition)
 }).and_then(|result|result)
}

/// Flat executable inputs; references between these inputs never recursively embed text.
#[derive(Clone)]
pub struct SuppliedCardDefinitionSource {
    pub name: String,
    pub text: String,
    pub identities: Vec<String>,
    pub payments: Vec<String>,
    pub subtypes: Vec<crate::CustomSubtypeDefinition>,
}

/// The root and catalogue carry compiler-native identity; conversion closes each route independently.
pub struct SuppliedCardDefinitionBundle {
    pub root: crate::cards::CardDefinition,
    pub scope: crate::ids::CardId,
    pub catalog: Vec<crate::cards::CardDefinition>,
}

/// Predeclare every lexical identity before compiling any creation program.
/// `compile_root` is called once, preserving its original compiler-stamped ability IDs.
pub fn compile_supplied_card_definition_bundle(
    root_id: crate::ids::CardId,
    root_name: &str,
    root_text: &str,
    root_is_real_card: bool,
    sources: &[SuppliedCardDefinitionSource],
    compile_root: impl FnOnce() -> Result<crate::cards::CardDefinition, String>,
) -> Result<SuppliedCardDefinitionBundle, String> {
    use ironsmith_compiler_lowering::supplied_card_definitions::{
        SuppliedCardDefinitionGraph, with_supplied_card_definition_graph,
        publish_supplied_graph_definition,
    };
    if root_name.trim().is_empty() { return Err("supplied graph root requires a name".into()); }
    crate::ids::CardId::reserve_generated_namespace_through(root_id).map_err(str::to_owned)?;
    let mut seen = std::collections::HashSet::new();
    let mut declarations = Vec::new();
    if root_is_real_card {
        declarations.push((root_name.to_owned(), root_id));
        seen.insert(root_name.trim().to_ascii_lowercase());
    }
    let mut pending = Vec::new();
    let mut source_names = std::collections::HashSet::new();
    for source in sources {
        let key = source.name.trim().to_ascii_lowercase();
        if key.is_empty() || !source_names.insert(key.clone()) { return Err(format!("duplicate or empty supplied graph identity {}", source.name)); }
        if key == root_name.trim().to_ascii_lowercase() && root_is_real_card {
            if source.text.trim() != root_text.trim() { return Err(format!("supplied root identity {} has conflicting source text",source.name)); }
            continue;
        }
        if !seen.insert(key) { return Err(format!("ambiguous supplied graph identity {}", source.name)); }
        let id = crate::ids::CardId::new();
        declarations.push((source.name.clone(), id));
        pending.push((source, id));
    }
    let scope = declarations.first().map(|(_, id)| *id).ok_or("a supplied graph requires a real-card member")?;
    with_supplied_card_definition_graph(SuppliedCardDefinitionGraph {
        scope, declarations, compiled: Vec::new(),
    }, || {
        let mut catalog = Vec::new();
        // Creation cycles need only declared IDs. Other embedded-definition consumers
        // wait for their concrete dependency, without replacing it by a global name.
        while !pending.is_empty() {
            let mut deferred = Vec::new();
            let mut errors = Vec::new();
            let before = pending.len();
            for (source, id) in pending {
                match compile_supplied_card_definition_at_id(id, &source.name, &source.text, &source.identities, &source.payments, &source.subtypes) {
                    Ok(definition) => { publish_supplied_graph_definition(definition.clone()); catalog.push(definition); }
                    Err(error) => { errors.push(error); deferred.push((source, id)); }
                }
            }
            if deferred.len() == before { return Err(errors.join("\n")); }
            pending = deferred;
        }
        let root = compile_root()?;
        if root.card.id != root_id || !root.card.name.trim().eq_ignore_ascii_case(root_name.trim()) {
            return Err("compiled supplied graph root changed its declared identity".into());
        }
        if root_is_real_card {
            if root.card.is_token { return Err("a token cannot be admitted as a supplied real-card definition".into()); }
            catalog.push(root.clone());
        }
        Ok(SuppliedCardDefinitionBundle { root, scope, catalog })
    })
}

#[cfg(test)]
mod graph_compilation_tests {
    use super::*;

    fn source(name: &str, target: &str) -> SuppliedCardDefinitionSource {
        SuppliedCardDefinitionSource {
            name: name.into(),
            text: format!("Type: Sorcery\nCreate a real card named \"{target}\" in your hand as part of the named making action."),
            identities: Vec::new(), payments: Vec::new(), subtypes: Vec::new(),
        }
    }

    #[test]
    fn cyclic_sources_compile_to_finite_predeclared_references() {
        let root_id = crate::ids::CardId::new();
        let sources = vec![source("First design", "Second design"), source("Second design", "First design")];
        let bundle = compile_supplied_card_definition_bundle(root_id, "First design", &sources[0].text, true, &sources, || {
            compile_supplied_card_definition_at_id(root_id, "First design", &sources[0].text, &[], &[], &[])
        }).unwrap();
        assert_eq!(bundle.catalog.len(), 2);
        assert_eq!(bundle.root.card.id, root_id);
        // A flat catalogue serializes finitely; no dependency definition is embedded
        // into any shallow node's creation operation.
        let serialized = serde_json::to_value(&bundle.catalog).unwrap();
        fn check(value: &serde_json::Value, scope: u32, count: &mut usize) {
            match value {
                serde_json::Value::Object(map) => {
                    if map.contains_key("definition_refs") {
                        assert_eq!(map.get("definition_scope").and_then(serde_json::Value::as_u64), Some(scope as u64));
                        assert_eq!(map.get("definitions").and_then(serde_json::Value::as_array).unwrap().len(), 0);
                        assert_eq!(map.get("definition_refs").and_then(serde_json::Value::as_array).unwrap().len(), 2);
                        *count += 1;
                    }
                    for child in map.values() { check(child, scope, count); }
                }
                serde_json::Value::Array(values) => for child in values { check(child, scope, count); },
                _ => {}
            }
        }
        // ResolutionProgram serializes its authoritative segments and a
        // mirrored flattened-default cache. Count executable occurrences from
        // segments, while independently proving that the cache is identical.
        let mut authoritative_references=0;
        let mut cached_references=0;
        for node in &bundle.catalog {
            let program=node.spell_effect.as_ref().expect("one creation spell per catalog node");
            let authoritative=program.segments.iter().flat_map(|segment|segment.default_effects.iter()).collect::<Vec<_>>();
            assert_eq!(program.flattened_default_effects().iter().collect::<Vec<_>>(),authoritative);
            check(&serde_json::to_value(&program.segments).unwrap(),root_id.0,&mut authoritative_references);
            check(&serde_json::to_value(program.flattened_default_effects()).unwrap(),root_id.0,&mut cached_references);
        }
        assert_eq!(authoritative_references,2,"one actual shallow creation per declared source");
        assert_eq!(cached_references,authoritative_references,"the cache cannot introduce another executable program");
        let mut serialized_references=0;
        check(&serialized,root_id.0,&mut serialized_references);
        assert_eq!(serialized_references,authoritative_references+cached_references,"all serialized reference occurrences belong to the checked programs or their cache");
    }

    #[test]
    fn conflicting_same_named_root_source_is_rejected_before_compilation() {
        let root_id=crate::ids::CardId::new();
        let sources=vec![source("Root design","Root design")];
        let error=compile_supplied_card_definition_bundle(root_id,"Root design","Type: Sorcery\nDraw a card.",true,&sources,||unreachable!()).err().unwrap();
        assert!(error.contains("conflicting source text"));
    }

    #[test]
    fn graph_rejects_ambiguous_full_names_and_restores_outer_scope() {
        let root_id = crate::ids::CardId::new();
        let sources = vec![source("Duplicate design", "Duplicate design"), source("duplicate DESIGN", "Duplicate design")];
        assert!(compile_supplied_card_definition_bundle(root_id, "Root design", "", true, &sources, || unreachable!()).is_err());
        assert!(crate::supplied_card_definition_named("Duplicate design").unwrap().is_none());
    }
}

#[cfg(test)]
mod supplied_real_kind_tests {
    #[test]
    fn token_type_header_is_not_silently_reinterpreted_as_a_real_card() {
        let error=super::compile_supplied_card_definition("Wrong kind","Type: Token Creature\nPower/Toughness: 1/1",&[],&[],&[]).unwrap_err();
        assert!(error.contains("Token type header"));
    }
}

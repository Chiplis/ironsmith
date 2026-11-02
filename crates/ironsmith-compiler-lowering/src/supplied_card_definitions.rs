//! Explicit real-card definitions scoped to one caller-owned compilation.
use std::cell::RefCell;
use crate::cards::CardDefinition;
thread_local!{static DEFINITIONS:RefCell<Vec<CardDefinition>>=const{RefCell::new(Vec::new())};}
/// Nested compilations see only the supplied inner pool; unwinding restores it.
pub fn with_supplied_card_definitions<R>(definitions:Vec<CardDefinition>,f:impl FnOnce()->R)->R{
 struct Restore(Vec<CardDefinition>,Option<SuppliedCardDefinitionGraph>);impl Drop for Restore{fn drop(&mut self){let previous=std::mem::take(&mut self.0);DEFINITIONS.with(|cell|*cell.borrow_mut()=previous);let graph=self.1.take();GRAPH.with(|cell|*cell.borrow_mut()=graph);}}
 let _restore=Restore(DEFINITIONS.with(|cell|cell.replace(definitions)),GRAPH.with(|cell|cell.replace(None)));f()
}
pub(crate) fn definitions()->Vec<CardDefinition>{DEFINITIONS.with(|cell|cell.borrow().clone())}

/// Resolve one exact supplied identity; never consult the official catalog.
pub fn supplied_card_definition_named(name:&str)->Result<Option<CardDefinition>,String>{
 let pool = GRAPH.with(|cell| cell.borrow().as_ref().map(|graph| graph.compiled.clone())).unwrap_or_else(definitions);
 let matching=pool.into_iter().filter(|definition|definition.card.name.trim().eq_ignore_ascii_case(name.trim())).collect::<Vec<_>>();
 if matching.len()>1{return Err(format!("ambiguous supplied real-card definition named {name}"));}Ok(matching.into_iter().next())
}

/// Lexical declarations are available before any cyclic creation body is compiled.
#[derive(Clone)]
pub struct SuppliedCardDefinitionGraph {
    pub scope: ironsmith_core::CardId,
    pub declarations: Vec<(String, ironsmith_core::CardId)>,
    pub compiled: Vec<CardDefinition>,
}
thread_local! { static GRAPH: RefCell<Option<SuppliedCardDefinitionGraph>> = const { RefCell::new(None) }; }

pub fn with_supplied_card_definition_graph<R>(graph: SuppliedCardDefinitionGraph, f: impl FnOnce() -> R) -> R {
    struct Restore(Option<SuppliedCardDefinitionGraph>);
    impl Drop for Restore { fn drop(&mut self) {
        let previous = self.0.take(); GRAPH.with(|cell| *cell.borrow_mut() = previous);
    }}
    let _restore = Restore(GRAPH.with(|cell| cell.replace(Some(graph))));
    f()
}

pub fn publish_supplied_graph_definition(definition: CardDefinition) {
    GRAPH.with(|cell| {
        if let Some(graph) = cell.borrow_mut().as_mut() {
            graph.compiled.retain(|existing| existing.card.id != definition.card.id);
            graph.compiled.push(definition);
        }
    });
}

pub(crate) fn creation_scope() -> Option<(ironsmith_core::CardId, Vec<ironsmith_core::CardId>)> {
    GRAPH.with(|cell| cell.borrow().as_ref().map(|graph| (
        graph.scope, graph.declarations.iter().map(|(_, id)| *id).collect()
    )))
}

pub(crate) fn admitted_names() -> Vec<String> {
    GRAPH.with(|cell| cell.borrow().as_ref().map(|graph| graph.declarations.iter().map(|(name, _)| name.clone()).collect()))
        .unwrap_or_else(|| definitions().iter().map(|definition| definition.card.name.clone()).collect())
}

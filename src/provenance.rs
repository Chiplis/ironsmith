use std::collections::HashMap;

use crate::events::EventKind;
use crate::ids::{ObjectId, PlayerId};

/// Stable identifier for a provenance graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ProvNodeId(u64);

/// Semantic type of a provenance graph node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvenanceNodeKind {
    RootEvent {
        kind: EventKind,
    },
    DerivedEvent {
        kind: EventKind,
    },
    TriggerQueued,
    TriggerMatched {
        source: ObjectId,
        controller: PlayerId,
    },
    EffectExecution {
        source: ObjectId,
        controller: PlayerId,
    },
}

/// One node in the provenance graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenanceNode {
    pub id: ProvNodeId,
    pub parent: Option<ProvNodeId>,
    pub kind: ProvenanceNodeKind,
}

/// In-memory provenance graph for the current game.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceGraph {
    next_id: u64,
    nodes: HashMap<ProvNodeId, ProvenanceNode>,
}

impl ProvenanceGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node(&self, id: ProvNodeId) -> Option<&ProvenanceNode> {
        self.nodes.get(&id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn alloc_root_event(&mut self, kind: EventKind) -> ProvNodeId {
        self.alloc_root(ProvenanceNodeKind::RootEvent { kind })
    }

    pub fn alloc_child_event(&mut self, parent: ProvNodeId, kind: EventKind) -> ProvNodeId {
        self.alloc_child(parent, ProvenanceNodeKind::DerivedEvent { kind })
    }

    pub fn alloc_root(&mut self, kind: ProvenanceNodeKind) -> ProvNodeId {
        self.alloc_node(None, kind)
    }

    pub fn alloc_child(&mut self, parent: ProvNodeId, kind: ProvenanceNodeKind) -> ProvNodeId {
        let normalized_parent =
            if parent == ProvNodeId::default() || !self.nodes.contains_key(&parent) {
                None
            } else {
                Some(parent)
            };
        self.alloc_node(normalized_parent, kind)
    }

    pub fn ensure_event_root(&mut self, provenance: ProvNodeId, kind: EventKind) -> ProvNodeId {
        if provenance == ProvNodeId::default() {
            self.alloc_root_event(kind)
        } else {
            provenance
        }
    }

    fn alloc_node(&mut self, parent: Option<ProvNodeId>, kind: ProvenanceNodeKind) -> ProvNodeId {
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("provenance node id overflow");
        let id = ProvNodeId(self.next_id);
        self.nodes.insert(id, ProvenanceNode { id, parent, kind });
        id
    }
}

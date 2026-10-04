use crate::events::EventKind;
use crate::ids::{ObjectId, PlayerId};

/// Stable identifier for a provenance graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature="serialization",derive(serde::Serialize,serde::Deserialize))]
pub struct ProvNodeId(u64);

impl ProvNodeId {
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Semantic type of a provenance graph node.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(deny_unknown_fields))]
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
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(deny_unknown_fields))]
pub struct ProvenanceNode {
    pub id: ProvNodeId,
    #[cfg_attr(feature = "serialization", serde(deserialize_with = "deserialize_present_parent"))]
    pub parent: Option<ProvNodeId>,
    pub kind: ProvenanceNodeKind,
}

/// In-memory provenance graph for the current game.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceGraph {
    next_id: u64,
    nodes: im::Vector<ProvenanceNode>,
}


/// Complete chronological graph. References remain native IDs; the owning
/// checkpoint validates its queued work against this graph before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serialization", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialization", serde(deny_unknown_fields))]
pub struct RetainedProvenanceGraph {
    pub next_id: u64,
    pub nodes: Vec<ProvenanceNode>,
}

#[cfg(feature = "serialization")]
fn deserialize_present_parent<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<ProvNodeId>, D::Error> {
    <Option<ProvNodeId> as serde::Deserialize>::deserialize(deserializer)
}

impl ProvenanceGraph {
    pub fn retained_state(&self) -> RetainedProvenanceGraph {
        RetainedProvenanceGraph { next_id: self.next_id, nodes: self.nodes.iter().cloned().collect() }
    }

    /// Validate before constructing the graph. Native IDs are consecutive and
    /// every parent predates its child, which also rules out cycles.
    pub fn from_retained_state(state: RetainedProvenanceGraph) -> Result<Self, String> {
        let count = u64::try_from(state.nodes.len()).map_err(|_| "provenance graph is too large")?;
        if state.next_id != count || state.next_id == u64::MAX {
            return Err("invalid provenance allocator chronology".into());
        }
        for (index, node) in state.nodes.iter().enumerate() {
            let expected = u64::try_from(index).map_err(|_| "provenance graph is too large")? + 1;
            if node.id.raw() != expected {
                return Err("provenance node identities must be consecutive and ordered".into());
            }
            if node.parent.is_some_and(|parent| parent.raw() == 0 || parent.raw() >= expected) {
                return Err("provenance parent must identify an earlier node".into());
            }
        }
        Ok(Self { next_id: state.next_id, nodes: state.nodes.into_iter().collect() })
    }

    pub fn restore_retained_state(&mut self, state: RetainedProvenanceGraph) -> Result<(), String> {
        let restored = Self::from_retained_state(state)?;
        *self = restored;
        Ok(())
    }

    /// Zero is the existing unassigned sentinel. Every assigned reference must
    /// resolve to this world's graph, rather than being silently reparented.
    pub fn validate_reference(&self, id: ProvNodeId) -> Result<(), String> {
        if id != ProvNodeId::default() && self.node(id).is_none() {
            return Err(format!("unknown provenance node {}", id.raw()));
        }
        Ok(())
    }
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node(&self, id: ProvNodeId) -> Option<&ProvenanceNode> {
        let index = usize::try_from(id.raw().checked_sub(1)?).ok()?;
        self.nodes.get(index)
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
        let normalized_parent = if parent == ProvNodeId::default() || self.node(parent).is_none() {
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

    pub fn is_descendant_of(&self, node: ProvNodeId, ancestor: ProvNodeId) -> bool {
        if node == ProvNodeId::default() || ancestor == ProvNodeId::default() {
            return false;
        }

        let mut current = Some(node);
        while let Some(id) = current {
            if id == ancestor {
                return true;
            }
            current = self.node(id).and_then(|entry| entry.parent);
        }

        false
    }

    fn alloc_node(&mut self, parent: Option<ProvNodeId>, kind: ProvenanceNodeKind) -> ProvNodeId {
        self.next_id = self
            .next_id
            .checked_add(1)
            .expect("provenance node id overflow");
        let id = ProvNodeId(self.next_id);
        self.nodes.push_back(ProvenanceNode { id, parent, kind });
        id
    }
}

#[cfg(all(test, feature = "serialization"))]
mod retained_provenance_graph_contract_tests {
    use super::*;
    fn fixture() -> ProvenanceGraph {
        let mut graph=ProvenanceGraph::new();
        let root=graph.alloc_root_event(EventKind::Damage);
        let child=graph.alloc_child_event(root,EventKind::LifeGain);
        let queued=graph.alloc_child(child,ProvenanceNodeKind::TriggerQueued);
        let matched=graph.alloc_child(queued,ProvenanceNodeKind::TriggerMatched{source:ObjectId::from_raw(19),controller:PlayerId::from_index(0)});
        graph.alloc_child(matched,ProvenanceNodeKind::EffectExecution{source:ObjectId::from_raw(19),controller:PlayerId::from_index(0)});
        graph
    }
    #[test]
    fn retained_graph_preserves_all_node_kinds_parents_ids_and_next_allocation() {
        let original=fixture();let wire=serde_json::to_string(&original.retained_state()).unwrap();
        let mut restored=ProvenanceGraph::from_retained_state(serde_json::from_str(&wire).unwrap()).unwrap();
        assert_eq!(restored.retained_state(),original.retained_state());
        assert!(restored.is_descendant_of(ProvNodeId(5),ProvNodeId(1)));
        let next=restored.alloc_child_event(ProvNodeId(5),EventKind::DamagePrevented);
        assert_eq!(next.raw(),6);assert!(restored.is_descendant_of(next,ProvNodeId(1)));
        assert!(restored.validate_reference(ProvNodeId::default()).is_ok());
        assert!(restored.validate_reference(next).is_ok());assert!(restored.validate_reference(ProvNodeId(99)).is_err());
    }
    #[test]
    fn malformed_graph_rejects_before_receiver_mutation_and_retry_preserves_chronology() {
        let original=fixture().retained_state();let mut receiver=fixture();
        for mode in 0..7 {
            let mut bad=original.clone();
            match mode {
                0=>bad.nodes[1].id=bad.nodes[0].id,
                1=>bad.nodes.swap(0,1),
                2=>bad.nodes[0].id=ProvNodeId::default(),
                3=>bad.nodes[1].parent=Some(ProvNodeId(2)),
                4=>bad.nodes[1].parent=Some(ProvNodeId(99)),
                5=>bad.nodes[1].parent=Some(ProvNodeId::default()),
                _=>bad.next_id+=1,
            }
            assert!(receiver.restore_retained_state(bad).is_err());assert_eq!(receiver.retained_state(),original);
        }
        receiver.restore_retained_state(original).unwrap();assert_eq!(receiver.alloc_root_event(EventKind::Draw).raw(),6);
    }
    #[test]
    fn graph_wire_requires_explicit_nullable_parent_and_rejects_unknown_fields() {
        let json=serde_json::to_value(fixture().retained_state()).unwrap();
        for mode in 0..4 {
            let mut bad=json.clone();match mode {
                0=>{bad.as_object_mut().unwrap().remove("next_id");},
                1=>{bad["nodes"][0].as_object_mut().unwrap().remove("parent");},
                2=>{bad["extra"]=serde_json::json!(1);},
                _=>{bad["nodes"][0]["extra"]=serde_json::json!(1);},
            }
            assert!(serde_json::from_value::<RetainedProvenanceGraph>(bad).is_err());
        }
    }
    #[test]
    fn long_finite_graph_restores_complete_ancestry() {
        let mut original=ProvenanceGraph::new();let root=original.alloc_root_event(EventKind::Damage);let mut last=root;
        for _ in 0..4096 {last=original.alloc_child_event(last,EventKind::DamagePrevented);}
        let restored=ProvenanceGraph::from_retained_state(original.retained_state()).unwrap();
        assert_eq!(restored.node_count(),4097);assert!(restored.is_descendant_of(last,root));
    }
}

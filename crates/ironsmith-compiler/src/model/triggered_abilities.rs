use crate::model::ast::{EffectAst, PredicateAst, TriggerIntroSurfaceAst, TriggerSpec};
use crate::model::provenance::{ProvenanceId, SemanticProvenance};
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolReference, SymbolResolutionError,
};
use crate::parse_context::ParseContextView;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKindAst {
    Normal,
    Reflexive,
    Delayed,
    State,
    ZoneChange,
    Dies,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TriggerSubjectAst {
    Source,
    Object(ObjectFilter),
    Player(PlayerFilter),
    Event(SymbolReference),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerZoneTransitionAst {
    pub from: Option<Zone>,
    pub to: Option<Zone>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerFrequencyAst {
    EachOccurrence,
    Once,
    AtMostPerTurn(u32),
    StateUntilFalse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerReferenceSurfaceAst {
    It,
    That,
    Those,
    SacrificedObject,
    TriggeringObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerReferenceAst {
    pub surface: TriggerReferenceSurfaceAst,
    pub reference: SymbolReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TriggerBindingsAst {
    pub triggering_object: Option<SymbolReference>,
    pub triggering_event: SymbolReference,
}

impl TriggerBindingsAst {
    pub fn allocate(
        context: ParseContextView<'_>,
        triggering_object_cardinality: Option<Cardinality>,
        provenance: Option<ProvenanceId>,
    ) -> Result<Self, SymbolResolutionError> {
        let triggering_event = SymbolReference {
            symbol: context.bind_symbol(
                ReferenceRole::Triggering,
                Cardinality::ExactlyOne,
                ObjectDomain::Event,
                provenance,
            )?,
            role: ReferenceRole::Triggering,
            domain: ObjectDomain::Event,
            cardinality: Cardinality::ExactlyOne,
        };
        let triggering_object = triggering_object_cardinality
            .map(|cardinality| {
                context
                    .bind_symbol(
                        ReferenceRole::Triggering,
                        cardinality,
                        ObjectDomain::Object,
                        provenance,
                    )
                    .map(|symbol| SymbolReference {
                        symbol,
                        role: ReferenceRole::Triggering,
                        domain: ObjectDomain::Object,
                        cardinality,
                    })
            })
            .transpose()?;
        Ok(Self {
            triggering_object,
            triggering_event,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerTriggerEventAst {
    pub intro: TriggerIntroSurfaceAst,
    pub kind: TriggerKindAst,
    pub subject: TriggerSubjectAst,
    pub zones: Option<TriggerZoneTransitionAst>,
    pub condition: Option<PredicateAst>,
    pub frequency: TriggerFrequencyAst,
    /// The complete compiler-owned event vocabulary. The surrounding facts
    /// make event category, scope, bindings, and frequency explicit without
    /// replacing the existing exhaustive trigger semantics.
    pub semantics: TriggerSpec,
    pub bindings: TriggerBindingsAst,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedTriggerEffectAst {
    pub effect_index: usize,
    pub triggering_object: Option<SymbolReference>,
    pub triggering_event: SymbolReference,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerTriggeredAbilityAst {
    pub event: CompilerTriggerEventAst,
    /// Branch-scoped executable program. `effects` remains as the finite
    /// compatibility payload until the PR-31 lowering migration.
    pub program: crate::model::CompilerControlFlowAst,
    pub effects: Vec<EffectAst>,
    pub intervening_if: Option<PredicateAst>,
    pub linked_effects: Vec<LinkedTriggerEffectAst>,
    pub references: Vec<TriggerReferenceAst>,
    pub functional_zones: Vec<Zone>,
    pub provenance: Option<SemanticProvenance>,
}

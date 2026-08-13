use crate::model::ast::{EffectAst, PredicateAst, TriggerSpec};
use crate::model::provenance::{ProvenanceId, SemanticProvenance};
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolReference, SymbolResolutionError,
};
use crate::parse_context::ParseContextView;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerKindAst {
    Normal,
    Reflexive,
    Delayed,
    State,
    ZoneChange,
    Dies,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TriggerSubjectAst {
    Source,
    Object(ObjectFilter),
    Player(PlayerFilter),
    Event(SymbolReference),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerZoneTransitionAst {
    pub from: Option<Zone>,
    pub to: Option<Zone>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerFrequencyAst {
    EachOccurrence,
    Once,
    AtMostPerTurn(u32),
    StateUntilFalse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TriggerBindingsAst {
    pub triggering_object: Option<SymbolReference>,
    pub triggering_event: SymbolReference,
}

impl TriggerBindingsAst {
    pub(crate) fn allocate(
        context: ParseContextView<'_>,
        has_triggering_object: bool,
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
        let triggering_object = has_triggering_object
            .then(|| {
                context
                    .bind_symbol(
                        ReferenceRole::Triggering,
                        Cardinality::ExactlyOne,
                        ObjectDomain::Object,
                        provenance,
                    )
                    .map(|symbol| SymbolReference {
                        symbol,
                        role: ReferenceRole::Triggering,
                        domain: ObjectDomain::Object,
                        cardinality: Cardinality::ExactlyOne,
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
pub(crate) struct CompilerTriggerEventAst {
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
pub(crate) struct LinkedTriggerEffectAst {
    pub effect_index: usize,
    pub triggering_object: Option<SymbolReference>,
    pub triggering_event: SymbolReference,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerTriggeredAbilityAst {
    pub event: CompilerTriggerEventAst,
    pub effects: Vec<EffectAst>,
    pub intervening_if: Option<PredicateAst>,
    pub linked_effects: Vec<LinkedTriggerEffectAst>,
    pub functional_zones: Vec<Zone>,
    pub provenance: Option<SemanticProvenance>,
}

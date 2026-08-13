use crate::effect::{Comparison, Value};
use crate::model::provenance::{ProvenanceId, SemanticProvenance};
use crate::model::symbols::{
    Cardinality, ObjectDomain, ReferenceRole, SymbolReference, SymbolResolutionError,
};
use crate::parse_context::ParseContextView;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerFilterAst {
    Object(ObjectFilter),
    Player(PlayerFilter),
    Spell(ObjectFilter),
    Card(ObjectFilter),
}

impl CompilerFilterAst {
    pub(crate) fn domain(&self) -> ObjectDomain {
        match self {
            Self::Object(_) => ObjectDomain::Object,
            Self::Player(_) => ObjectDomain::Player,
            Self::Spell(_) => ObjectDomain::Spell,
            Self::Card(_) => ObjectDomain::Card,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArithmeticOperatorAst {
    Add,
    Subtract,
    Multiply,
    Divide,
    Minimum,
    Maximum,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CompilerValueAst {
    Fixed(i32),
    X,
    Dynamic(Value),
    Count(CompilerFilterAst),
    Arithmetic {
        operator: ArithmeticOperatorAst,
        operands: Vec<CompilerValueAst>,
    },
    Compared {
        value: Box<CompilerValueAst>,
        comparison: Comparison,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SelectionKindAst {
    Target,
    Choose,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SelectionDomainAst {
    Source,
    AnyTarget,
    AnyOtherTarget,
    Filter(CompilerFilterAst),
    ObjectOrPlayer {
        object: ObjectFilter,
        player: PlayerFilter,
    },
    PlayerOrPlaneswalker(PlayerFilter),
    Spell(ObjectFilter),
}

impl SelectionDomainAst {
    pub(crate) fn symbol_domain(&self) -> ObjectDomain {
        match self {
            Self::Source | Self::AnyTarget | Self::AnyOtherTarget | Self::ObjectOrPlayer { .. } => {
                ObjectDomain::Object
            }
            Self::Filter(filter) => filter.domain(),
            Self::PlayerOrPlaneswalker(_) => ObjectDomain::Player,
            Self::Spell(_) => ObjectDomain::Spell,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SelectionCardinalityAst {
    pub min: CompilerValueAst,
    pub max: Option<CompilerValueAst>,
    pub reference_cardinality: Cardinality,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SelectionLegalityAst {
    pub targetable: bool,
    pub zones: Vec<Zone>,
    pub controller_only: bool,
    pub owner_only: bool,
    pub distinct: bool,
    pub random: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompilerSelectionAst {
    pub kind: SelectionKindAst,
    pub domain: SelectionDomainAst,
    pub cardinality: SelectionCardinalityAst,
    pub legality: SelectionLegalityAst,
    pub binding: SymbolReference,
    pub provenance: Option<SemanticProvenance>,
}

impl CompilerSelectionAst {
    pub(crate) fn bind(
        context: ParseContextView<'_>,
        kind: SelectionKindAst,
        domain: SelectionDomainAst,
        cardinality: SelectionCardinalityAst,
        legality: SelectionLegalityAst,
        provenance: Option<ProvenanceId>,
    ) -> Result<Self, SymbolResolutionError> {
        let role = match kind {
            SelectionKindAst::Target => ReferenceRole::Target,
            SelectionKindAst::Choose => ReferenceRole::Chosen,
        };
        let symbol_domain = domain.symbol_domain();
        let binding = SymbolReference {
            symbol: context.bind_symbol(
                role,
                cardinality.reference_cardinality,
                symbol_domain,
                provenance,
            )?,
            role,
            domain: symbol_domain,
            cardinality: cardinality.reference_cardinality,
        };
        Ok(Self {
            kind,
            domain,
            cardinality,
            legality,
            binding,
            provenance: provenance.map(|primary| SemanticProvenance {
                primary,
                related: Vec::new(),
            }),
        })
    }
}

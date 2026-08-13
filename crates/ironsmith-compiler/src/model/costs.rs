use crate::color::ColorSet;
use crate::effect::{ChoiceCount, Value};
use crate::mana::ManaCost;
use crate::model::provenance::SemanticProvenance;
use crate::model::symbols::SymbolReference;
use crate::object::CounterType;
use crate::target::{ObjectFilter, SourceReferenceSurface};
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostRelationship {
    Ordinary,
    Additional,
    Optional,
    Alternative,
    RatherThan,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerTotalCost {
    pub branches: Vec<Vec<CompilerCost>>,
    pub relationship: CostRelationship,
    pub repeatable: bool,
    pub is_loyalty_shorthand: bool,
    pub provenance: Option<SemanticProvenance>,
}

impl CompilerTotalCost {
    pub fn ordered(costs: Vec<CompilerCost>) -> Self {
        Self {
            branches: vec![costs],
            relationship: CostRelationship::Ordinary,
            repeatable: false,
            is_loyalty_shorthand: false,
            provenance: None,
        }
    }

    pub fn alternatives(branches: Vec<Vec<CompilerCost>>) -> Self {
        Self {
            branches,
            relationship: CostRelationship::Alternative,
            repeatable: false,
            is_loyalty_shorthand: false,
            provenance: None,
        }
    }

    pub fn costs(&self) -> Option<&[CompilerCost]> {
        match self.branches.as_slice() {
            [branch] => Some(branch),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerCost {
    Mana(ManaCost),
    VariableMana {
        generic: u32,
    },
    Tap,
    TapChosen {
        count: u32,
        filter: ObjectFilter,
    },
    Untap,
    Life(Value),
    Energy(u32),
    DiscardSource,
    DiscardHand,
    Discard {
        count: u32,
        card_types: Vec<CardType>,
        supertypes: Vec<Supertype>,
        filter: Option<ObjectFilter>,
        random: bool,
        name: Option<String>,
        other: bool,
        binding: Option<SymbolReference>,
    },
    Mill(u32),
    SacrificeSelf {
        surface: Option<SourceReferenceSurface>,
    },
    Sacrifice {
        count: ChoiceCount,
        filter: ObjectFilter,
        all: bool,
        binding: Option<SymbolReference>,
    },
    Unattach {
        count: u32,
        filter: ObjectFilter,
    },
    ExileSelf {
        from_graveyard: bool,
    },
    ExileFromHand {
        count: u32,
        color_filter: Option<ColorSet>,
    },
    ExileChosen {
        count: ChoiceCount,
        filter: ObjectFilter,
        top_only: bool,
        turn_face_up: bool,
        binding: Option<SymbolReference>,
    },
    ExileSourceAndChosen {
        source_filter: ObjectFilter,
        count: ChoiceCount,
        filter: ObjectFilter,
    },
    ExileSelfAndNamedArtifacts {
        names: Vec<String>,
    },
    ExileTopLibrary {
        count: u32,
    },
    RevealSourceFromHand,
    RevealFromHand {
        count: Value,
        color_filter: Option<ColorSet>,
        card_type: Option<CardType>,
        binding: Option<SymbolReference>,
    },
    ReturnSelfToHand,
    ReturnChosenToHand {
        count: u32,
        filter: ObjectFilter,
    },
    MoveChosenToLibraryTop {
        filter: ObjectFilter,
    },
    MoveSelfToLibraryBottom {
        surface: SourceReferenceSurface,
    },
    MoveOpponentOwnedExiledCardToGraveyard,
    ExertSelf {
        display: String,
    },
    PutCounters {
        counter_type: CounterType,
        count: u32,
        filter: Option<ObjectFilter>,
    },
    Blight {
        count: u32,
    },
    RemoveCounters {
        counter_type: Option<CounterType>,
        count: u32,
        filter: Option<ObjectFilter>,
        display_x: bool,
        dynamic: bool,
        single_object: bool,
        remove_all: bool,
    },
    Behold {
        subtype: Subtype,
        count: u32,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerOptionalCost {
    pub kind: crate::cost::OptionalCostKind,
    pub source_label: String,
    pub cost: CompilerTotalCost,
    pub repeatable: bool,
    pub returns_to_hand: bool,
    pub provenance: Option<SemanticProvenance>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CastingConditionAst<Condition> {
    Condition(Condition),
    Trap(crate::alternative_cast::TrapCondition),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompilerAlternativeCastingMethod<Condition, Effect> {
    pub name: String,
    pub cast_from: Zone,
    pub total_cost: CompilerTotalCost,
    pub additional_cost: Option<CompilerTotalCost>,
    pub condition: Option<CastingConditionAst<Condition>>,
    pub setup_effects: Vec<Effect>,
    pub exiles_after_resolution: bool,
    pub rather_than_printed_cost: bool,
    pub provenance: Option<SemanticProvenance>,
}

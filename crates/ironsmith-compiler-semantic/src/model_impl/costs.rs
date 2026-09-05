use ironsmith_core::tag::TagKeyWalk;

use crate::color::ColorSet;
use crate::effect::{ChoiceCount, Value};
use crate::mana::ManaCost;
use crate::model::provenance::SemanticProvenance;
use crate::model::symbols::SymbolReference;
use crate::object::CounterType;
use crate::target::{ObjectFilter, SourceReferenceSurface};
use crate::types::{CardType, Subtype, Supertype};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(TagKeyWalk)]
pub enum CostRelationship {
    Ordinary,
    Additional,
    Optional,
    Alternative,
    RatherThan,
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct CompilerTotalCost {
    pub branches: Vec<Vec<CompilerCost>>,
    pub relationship: CostRelationship,
    pub repeatable: bool,
    pub is_loyalty_shorthand: bool,
    #[tag_walk(skip)]
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

    pub fn from_core_total_cost(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        match cost.kind() {
            ironsmith_core::TotalCostKind::All(costs) => Self::ordered(costs.clone()),
            ironsmith_core::TotalCostKind::OneOf(branches) => Self::alternatives(
                branches
                    .iter()
                    .map(|branch| branch.costs().to_vec())
                    .collect(),
            ),
        }
    }

    pub fn costs(&self) -> Option<&[CompilerCost]> {
        match self.branches.as_slice() {
            [branch] => Some(branch),
            _ => None,
        }
    }

    pub fn to_core_total_cost(&self) -> ironsmith_core::TotalCost<CompilerCost> {
        let branches = self
            .branches
            .iter()
            .cloned()
            .map(ironsmith_core::TotalCost::from_costs)
            .collect::<Vec<_>>();
        match branches.as_slice() {
            [] => ironsmith_core::TotalCost::from_costs(Vec::new()),
            [single] => single.clone(),
            _ => ironsmith_core::TotalCost::one_of(branches),
        }
    }
}

impl ironsmith_core::CostComponent for CompilerCost {
    fn mana(mana_cost: ManaCost) -> Self {
        Self::Mana(mana_cost)
    }

    fn display(&self) -> String {
        match self {
            Self::Mana(cost) => cost.to_oracle(),
            Self::DynamicMana(cost) => cost.base.to_oracle(),
            Self::VariableMana { generic } => format!("{{{generic}}}"),
            Self::Tap => "{T}".to_string(),
            Self::TapChosen { count, .. } => format!("tap {count} chosen permanent(s)"),
            Self::Untap => "{Q}".to_string(),
            Self::Life(amount) => format!("pay {amount:?} life"),
            Self::Energy(amount) => format!("pay {amount} energy"),
            Self::DiscardSource => "discard this card".to_string(),
            Self::DiscardHand => "discard your hand".to_string(),
            Self::Discard { count, .. } => format!("discard {count} card(s)"),
            Self::Mill(count) => format!("mill {count}"),
            Self::SacrificeSelf { .. } => "sacrifice this permanent".to_string(),
            Self::Sacrifice { count, .. } => format!("sacrifice {count:?} permanent(s)"),
            Self::Unattach { count, .. } => format!("unattach {count} permanent(s)"),
            Self::ExileSelf { .. } => "exile this card".to_string(),
            Self::ExileFromHand { count, .. } => {
                format!("exile {count} card(s) from your hand")
            }
            Self::ExileChosen { count, .. } => format!("exile {count:?} chosen card(s)"),
            Self::ExileSourceAndChosen { count, .. } => {
                format!("exile this card and {count:?} chosen card(s)")
            }
            Self::ExileSelfAndNamedArtifacts { names } => {
                format!("exile this card and {}", names.join(" and "))
            }
            Self::ExileTopLibrary { count } => {
                format!("exile the top {count} card(s) of your library")
            }
            Self::RevealSourceFromHand => "reveal this card from your hand".to_string(),
            Self::RevealSourceFromHandUntilUpkeepEnds => {
                "reveal this card from your hand until upkeep ends".to_string()
            }
            Self::RevealFromHand { count, .. } => format!("reveal {count:?} card(s) from hand"),
            Self::ReturnSelfToHand => "return this permanent to its owner's hand".to_string(),
            Self::ReturnChosenToHand { count, .. } => {
                format!("return {count} chosen permanent(s) to hand")
            }
            Self::MoveChosenToLibraryTop { .. } => {
                "put a chosen card on top of its owner's library".to_string()
            }
            Self::MoveChosenToLibraryBottom { count, .. } => {
                format!("put {count} chosen card(s) on the bottom of their owners' libraries")
            }
            Self::MoveSelfToLibraryBottom { .. } => {
                "put this permanent on the bottom of its owner's library".to_string()
            }
            Self::MoveOpponentOwnedExiledCardToGraveyard => {
                "put an opponent-owned exiled card into its owner's graveyard".to_string()
            }
            Self::ExertSelf { display } => display.clone(),
            Self::EmitKeywordAction { kind, amount } => {
                format!("emit {kind:?} keyword action ({amount})")
            }
            Self::Crew { amount } => format!("crew {amount:?}"),
            Self::Sneak => "sneak".to_string(),
            Self::Effect(effect) | Self::ValidatedEffect(effect) => {
                format!("compiler effect cost: {effect:?}")
            }
            Self::PutCounters {
                counter_type,
                count,
                ..
            } => format!("put {count} {counter_type:?} counter(s)"),
            Self::Blight { count } => format!("blight {count}"),
            Self::RemoveCounters {
                counter_type,
                count,
                dynamic,
                remove_all,
                ..
            } => format!(
                "remove {count} {counter_type:?} counter(s) (dynamic={dynamic}, all={remove_all})"
            ),
            Self::Behold { subtype, count } => format!("behold {count} {subtype:?}"),
        }
    }

    fn sacrifice_filter(&self) -> Option<&ObjectFilter> {
        match self {
            Self::Sacrifice { filter, .. } => Some(filter),
            _ => None,
        }
    }

    fn is_mana_cost(&self) -> bool {
        matches!(
            self,
            Self::Mana(_) | Self::DynamicMana(_) | Self::VariableMana { .. }
        )
    }

    fn requires_tap(&self) -> bool {
        matches!(self, Self::Tap | Self::TapChosen { .. })
    }

    fn is_sacrifice_self(&self) -> bool {
        matches!(self, Self::SacrificeSelf { .. })
    }

    fn is_loyalty_activation_cost(&self) -> bool {
        matches!(
            self,
            Self::PutCounters {
                counter_type: CounterType::Loyalty,
                ..
            } | Self::RemoveCounters {
                counter_type: Some(CounterType::Loyalty),
                ..
            }
        )
    }

    fn mana_cost_ref(&self) -> Option<&ManaCost> {
        match self {
            Self::Mana(cost) => Some(cost),
            Self::DynamicMana(cost) => Some(&cost.base),
            _ => None,
        }
    }
}

impl ironsmith_core::CoreCostComponent for CompilerCost {
    fn tap_cost() -> Self {
        Self::Tap
    }
}

#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum CompilerCost {
    Mana(ManaCost),
    DynamicMana(ironsmith_core::DynamicManaCost),
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
    RevealSourceFromHandUntilUpkeepEnds,
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
    MoveChosenToLibraryBottom {
        count: u32,
        filter: ObjectFilter,
    },
    MoveSelfToLibraryBottom {
        surface: SourceReferenceSurface,
    },
    MoveOpponentOwnedExiledCardToGraveyard,
    ExertSelf {
        display: String,
    },
    EmitKeywordAction {
        kind: crate::events::KeywordActionKind,
        amount: u32,
    },
    Crew {
        amount: u32,
    },
    Sneak,
    Effect(Box<crate::model::ast::EffectAst>),
    ValidatedEffect(Box<crate::model::ast::EffectAst>),
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
    pub reference: crate::cost::OptionalCostRef,
    pub source_label: String,
    pub cost: ironsmith_core::TotalCost<CompilerCost>,
    pub repeatable: bool,
    pub returns_to_hand: bool,
    pub provenance: Option<SemanticProvenance>,
}

impl CompilerOptionalCost {
    fn typed(
        kind: crate::cost::OptionalCostKind,
        source_label: impl Into<String>,
        cost: ironsmith_core::TotalCost<CompilerCost>,
    ) -> Self {
        let source_label = source_label.into();
        Self {
            kind,
            reference: crate::cost::OptionalCostRef::from_label(&source_label),
            source_label,
            cost,
            repeatable: false,
            returns_to_hand: false,
            provenance: None,
        }
    }

    pub fn custom(label: impl Into<String>, cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        let label = label.into();
        Self::typed(
            crate::cost::OptionalCostKind::from_label(&label),
            label,
            cost,
        )
    }

    pub fn kicker(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        Self::typed(crate::cost::OptionalCostKind::Kicker, "Kicker", cost)
    }

    pub fn multikicker(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        let mut cost = Self::typed(
            crate::cost::OptionalCostKind::Multikicker,
            "Multikicker",
            cost,
        );
        cost.repeatable = true;
        cost
    }

    pub fn replicate(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        let mut cost = Self::typed(crate::cost::OptionalCostKind::Replicate, "Replicate", cost);
        cost.repeatable = true;
        cost
    }

    pub fn buyback(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        let mut cost = Self::typed(crate::cost::OptionalCostKind::Buyback, "Buyback", cost);
        cost.returns_to_hand = true;
        cost
    }

    pub fn entwine(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        Self::typed(crate::cost::OptionalCostKind::Entwine, "Entwine", cost)
    }

    pub fn squad(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        let mut cost = Self::typed(crate::cost::OptionalCostKind::Squad, "Squad", cost);
        cost.repeatable = true;
        cost
    }

    pub fn offspring(cost: ironsmith_core::TotalCost<CompilerCost>) -> Self {
        Self::typed(crate::cost::OptionalCostKind::Offspring, "Offspring", cost)
    }

    pub fn repeatable(mut self) -> Self {
        self.repeatable = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CastingConditionAst<Condition> {
    Condition(Condition),
    Trap(crate::alternative_cast::TrapCondition),
}

pub type CompilerAlternativeCastingMethod = ironsmith_core::AlternativeCastingMethod<
    crate::model::ast::EffectAst,
    CompilerCost,
    crate::static_abilities::ThisSpellCostCondition,
>;

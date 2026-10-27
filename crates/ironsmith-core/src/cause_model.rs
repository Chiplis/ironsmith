use crate::tag::TagKeyWalk;

use crate::{ObjectFilter, ObjectId, PlayerId};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum CauseType {
    Cost,
    Effect,
    StateBasedAction,
    GameRule,
    CombatDamage,
    SpecialAction,
    LegendRule,
}

impl CauseType {
    pub fn is_effect_like(&self) -> bool {
        matches!(self, Self::Effect)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct EventCause {
    pub cause_type: CauseType,
    pub source: Option<ObjectId>,
    pub source_controller: Option<PlayerId>,
    /// The source whose linked exile duration caused this zone change when it ended.
    /// Ordinary effects from the same source leave this unset.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub duration_return_source: Option<ObjectId>,
    /// The spell currently resolving, distinct from an effect or ability source.
    #[cfg_attr(feature = "serde", serde(default))]
    pub resolving_spell: Option<ObjectId>,
    /// Objects targeted by the resolving spell or ability that causes this
    /// event (CR 115.1). Empty when the cause targets nothing or is not a
    /// resolving spell or ability.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Vec::is_empty"))]
    pub targeted_objects: Vec<ObjectId>,
    /// The kind of resolving ability that causes this event, when the cause
    /// is a resolving activated or triggered ability (CR 113.3b-c). `None`
    /// for spells, costs, rules and anything not resolving from the stack.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub resolving_ability: Option<CauseAbilityKind>,
}

/// What kind of ability is resolving when an event is caused (CR 113.3).
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum CauseAbilityKind {
    Activated,
    Triggered,
    /// The triggered ability granted by the modular keyword (CR 702.43a).
    ModularTrigger,
    /// Exact keyword of the ability whose activation cost caused this event.
    ActivatedKeywordCost(crate::ActivatedAbilityKeyword),
    ChampionTrigger,
}

impl CauseAbilityKind {
    pub fn is_triggered(self) -> bool {
        matches!(self, Self::Triggered | Self::ModularTrigger | Self::ChampionTrigger)
    }
}

/// Restricts a cause to a resolving ability of a given kind ("triggered
/// abilities you control can't cause ...", "if a modular triggered ability
/// would put ...").
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, TagKeyWalk)]
pub enum CauseAbilityKindFilter {
    Activated,
    Triggered,
    ModularTrigger,
    ActivatedKeywordCost(crate::ActivatedAbilityKeyword),
    ChampionTrigger,
}

impl CauseAbilityKindFilter {
    pub fn matches(self, kind: Option<CauseAbilityKind>) -> bool {
        match self {
            Self::Activated => kind == Some(CauseAbilityKind::Activated),
            Self::Triggered => kind.is_some_and(CauseAbilityKind::is_triggered),
            Self::ModularTrigger => kind == Some(CauseAbilityKind::ModularTrigger),
            Self::ChampionTrigger => kind == Some(CauseAbilityKind::ChampionTrigger),
            Self::ActivatedKeywordCost(keyword) => kind == Some(CauseAbilityKind::ActivatedKeywordCost(keyword)),
        }
    }
}

impl EventCause {
    pub fn effect() -> Self {
        Self {
            cause_type: CauseType::Effect,
            source: None,
            source_controller: None,
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    pub fn from_cost(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::Cost,
            source: Some(source),
            source_controller: Some(controller),
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    pub fn from_effect(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::Effect,
            source: Some(source),
            source_controller: Some(controller),
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    /// A cause occurring during a spell's successful resolution (CR 608.2).
    pub fn from_spell_resolution(spell: ObjectId, controller: PlayerId) -> Self {
        Self {
            resolving_spell: Some(spell),
            ..Self::from_effect(spell, controller)
        }
    }

    pub fn from_sba() -> Self {
        Self {
            cause_type: CauseType::StateBasedAction,
            source: None,
            source_controller: None,
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    pub fn from_game_rule() -> Self {
        Self {
            cause_type: CauseType::GameRule,
            source: None,
            source_controller: None,
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    pub fn from_combat_damage(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::CombatDamage,
            source: Some(source),
            source_controller: Some(controller),
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    pub fn combat_damage(source: ObjectId) -> Self {
        Self {
            cause_type: CauseType::CombatDamage,
            source: Some(source),
            source_controller: None,
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    pub fn from_special_action(source: Option<ObjectId>, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::SpecialAction,
            source,
            source_controller: Some(controller),
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }

    pub fn from_legend_rule(controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::LegendRule,
            source: None,
            source_controller: Some(controller),
            duration_return_source: None,
            resolving_spell: None,
            targeted_objects: Vec::new(),
            resolving_ability: None,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub struct CauseFilter {
    pub cause_type: Option<CauseTypeFilter>,
    pub source_filter: Option<ObjectFilter>,
    pub controller_filter: Option<ControllerFilter>,
    /// The kind of resolving ability the cause must be, when restricted.
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub ability_kind: Option<CauseAbilityKindFilter>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum CauseTypeFilter {
    Exact(CauseType),
    Not(CauseType),
    EffectLike,
    NotCost,
    OneOf(Vec<CauseType>),
}

impl CauseTypeFilter {
    pub fn matches(&self, cause_type: CauseType) -> bool {
        match self {
            Self::Exact(ct) => cause_type == *ct,
            Self::Not(ct) => cause_type != *ct,
            Self::EffectLike => cause_type.is_effect_like(),
            Self::NotCost => cause_type != CauseType::Cost,
            Self::OneOf(types) => types.contains(&cause_type),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, TagKeyWalk)]
pub enum ControllerFilter {
    Player(PlayerId),
    You,
    Opponent,
    ContextController,
    /// An opponent of the player evaluating the filter (the trigger's
    /// controller), independent of who controls the affected object.
    ContextOpponent,
    Any,
}

impl CauseFilter {
    pub fn any() -> Self {
        Self {
            cause_type: None,
            source_filter: None,
            controller_filter: None,
            ability_kind: None,
        }
    }

    pub fn effect_like() -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::EffectLike),
            source_filter: None,
            controller_filter: None,
            ability_kind: None,
        }
    }

    pub fn exact(cause_type: CauseType) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::Exact(cause_type)),
            source_filter: None,
            controller_filter: None,
            ability_kind: None,
        }
    }

    pub fn not_type(cause_type: CauseType) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::Not(cause_type)),
            source_filter: None,
            controller_filter: None,
            ability_kind: None,
        }
    }

    pub fn from_effect() -> Self {
        Self::exact(CauseType::Effect)
    }

    pub fn from_cost() -> Self {
        Self::exact(CauseType::Cost)
    }

    pub fn effect_from_source(source_filter: ObjectFilter) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::EffectLike),
            source_filter: Some(source_filter),
            controller_filter: None,
            ability_kind: None,
        }
    }

    pub fn from_source(source_filter: ObjectFilter) -> Self {
        Self {
            cause_type: None,
            source_filter: Some(source_filter),
            controller_filter: None,
            ability_kind: None,
        }
    }

    pub fn with_source(mut self, source_filter: ObjectFilter) -> Self {
        self.source_filter = Some(source_filter);
        self
    }

    pub fn with_controller(mut self, controller_filter: ControllerFilter) -> Self {
        self.controller_filter = Some(controller_filter);
        self
    }

    pub fn with_ability_kind(mut self, ability_kind: CauseAbilityKindFilter) -> Self {
        self.ability_kind = Some(ability_kind);
        self
    }
}

use crate::tag::TagKeyWalk;

use crate::{ObjectFilter, ObjectId, PlayerId};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(TagKeyWalk)]
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
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct EventCause {
    pub cause_type: CauseType,
    pub source: Option<ObjectId>,
    pub source_controller: Option<PlayerId>,
}

impl EventCause {
    pub fn effect() -> Self {
        Self {
            cause_type: CauseType::Effect,
            source: None,
            source_controller: None,
        }
    }

    pub fn from_cost(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::Cost,
            source: Some(source),
            source_controller: Some(controller),
        }
    }

    pub fn from_effect(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::Effect,
            source: Some(source),
            source_controller: Some(controller),
        }
    }

    pub fn from_sba() -> Self {
        Self {
            cause_type: CauseType::StateBasedAction,
            source: None,
            source_controller: None,
        }
    }

    pub fn from_game_rule() -> Self {
        Self {
            cause_type: CauseType::GameRule,
            source: None,
            source_controller: None,
        }
    }

    pub fn from_combat_damage(source: ObjectId, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::CombatDamage,
            source: Some(source),
            source_controller: Some(controller),
        }
    }

    pub fn combat_damage(source: ObjectId) -> Self {
        Self {
            cause_type: CauseType::CombatDamage,
            source: Some(source),
            source_controller: None,
        }
    }

    pub fn from_special_action(source: Option<ObjectId>, controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::SpecialAction,
            source,
            source_controller: Some(controller),
        }
    }

    pub fn from_legend_rule(controller: PlayerId) -> Self {
        Self {
            cause_type: CauseType::LegendRule,
            source: None,
            source_controller: Some(controller),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub struct CauseFilter {
    pub cause_type: Option<CauseTypeFilter>,
    pub source_filter: Option<ObjectFilter>,
    pub controller_filter: Option<ControllerFilter>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
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
#[derive(Debug, Clone, PartialEq)]
#[derive(TagKeyWalk)]
pub enum ControllerFilter {
    Player(PlayerId),
    You,
    Opponent,
    ContextController,
    Any,
}

impl CauseFilter {
    pub fn any() -> Self {
        Self {
            cause_type: None,
            source_filter: None,
            controller_filter: None,
        }
    }

    pub fn effect_like() -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::EffectLike),
            source_filter: None,
            controller_filter: None,
        }
    }

    pub fn exact(cause_type: CauseType) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::Exact(cause_type)),
            source_filter: None,
            controller_filter: None,
        }
    }

    pub fn not_type(cause_type: CauseType) -> Self {
        Self {
            cause_type: Some(CauseTypeFilter::Not(cause_type)),
            source_filter: None,
            controller_filter: None,
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
        }
    }

    pub fn from_source(source_filter: ObjectFilter) -> Self {
        Self {
            cause_type: None,
            source_filter: Some(source_filter),
            controller_filter: None,
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
}

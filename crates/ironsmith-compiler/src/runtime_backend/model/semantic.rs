#![allow(dead_code)]

use crate::ConditionExpr;
use crate::ability::{Ability, AbilityKind, ActivationTiming};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cost::OptionalCost;
use crate::effect::{EffectPredicate, Value};
use crate::zone::Zone;

use super::super::{KeywordAction, PlayerAst, SharedTypeConstraintAst, TargetAst, TotalCost};
use super::ast::{EffectAst, StaticAbilityAst, TriggerSpec};
use super::reference_model::ReferenceImports;
use super::shared_types::LineInfo;

#[derive(Debug, Clone)]
pub(crate) enum GiftTimingAst {
    SpellResolution,
    PermanentEtb,
}

#[derive(Debug, Clone)]
pub(crate) enum LineAst {
    Abilities(Vec<KeywordAction>),
    StaticAbility(StaticAbilityAst),
    StaticAbilities(Vec<StaticAbilityAst>),
    Ability(ParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        effects: Vec<EffectAst>,
        max_triggers_per_turn: Option<u32>,
    },
    Statement {
        effects: Vec<EffectAst>,
    },
    AdditionalCost {
        effects: Vec<EffectAst>,
    },
    OptionalCost(ParsedOptionalCostAst),
    GiftKeyword {
        cost: ParsedOptionalCostAst,
        effects: Vec<EffectAst>,
        followup_text: String,
        timing: GiftTimingAst,
    },
    OptionalCostWithCastTrigger {
        cost: ParsedOptionalCostAst,
        effects: Vec<EffectAst>,
        followup_text: String,
    },
    AdditionalCostChoice {
        options: Vec<AdditionalCostChoiceOptionAst>,
    },
    AlternativeCastingMethod(ParsedAlternativeCastingMethodAst),
}

#[derive(Debug, Clone)]
pub(crate) struct AdditionalCostChoiceOptionAst {
    pub(crate) description: String,
    pub(crate) effects: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAbilityRuntime {
    runtime: Ability,
}

impl ParsedAbilityRuntime {
    pub(crate) fn new(runtime: Ability) -> Self {
        Self { runtime }
    }

    pub(crate) fn as_runtime(&self) -> &Ability {
        &self.runtime
    }

    pub(crate) fn as_runtime_mut(&mut self) -> &mut Ability {
        &mut self.runtime
    }

    pub(crate) fn into_runtime(self) -> Ability {
        self.runtime
    }
}

impl From<Ability> for ParsedAbilityRuntime {
    fn from(value: Ability) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAbility {
    pub(crate) ability: ParsedAbilityRuntime,
    pub(crate) effects_ast: Option<Vec<EffectAst>>,
    pub(crate) reference_imports: ReferenceImports,
    pub(crate) trigger_spec: Option<TriggerSpec>,
}

impl ParsedAbility {
    pub(crate) fn new(
        ability: Ability,
        effects_ast: Option<Vec<EffectAst>>,
        reference_imports: ReferenceImports,
        trigger_spec: Option<TriggerSpec>,
    ) -> Self {
        Self {
            ability: ability.into(),
            effects_ast,
            reference_imports,
            trigger_spec,
        }
    }

    pub(crate) fn runtime(&self) -> &Ability {
        self.ability.as_runtime()
    }

    pub(crate) fn runtime_mut(&mut self) -> &mut Ability {
        self.ability.as_runtime_mut()
    }

    pub(crate) fn into_runtime(self) -> Ability {
        self.ability.into_runtime()
    }

    pub(crate) fn kind(&self) -> &AbilityKind {
        &self.runtime().kind
    }

    pub(crate) fn kind_mut(&mut self) -> &mut AbilityKind {
        &mut self.runtime_mut().kind
    }

    pub(crate) fn text(&self) -> &Option<String> {
        &self.runtime().text
    }

    pub(crate) fn text_mut(&mut self) -> &mut Option<String> {
        &mut self.runtime_mut().text
    }

    pub(crate) fn functional_zones_mut(&mut self) -> &mut Vec<Zone> {
        &mut self.runtime_mut().functional_zones
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedOptionalCostAst {
    runtime: OptionalCost,
}

impl ParsedOptionalCostAst {
    pub(crate) fn new(runtime: OptionalCost) -> Self {
        Self { runtime }
    }

    pub(crate) fn as_runtime(&self) -> &OptionalCost {
        &self.runtime
    }

    pub(crate) fn into_runtime(self) -> OptionalCost {
        self.runtime
    }
}

impl From<OptionalCost> for ParsedOptionalCostAst {
    fn from(value: OptionalCost) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedAlternativeCastingMethodAst {
    runtime: AlternativeCastingMethod,
}

impl ParsedAlternativeCastingMethodAst {
    pub(crate) fn new(runtime: AlternativeCastingMethod) -> Self {
        Self { runtime }
    }

    pub(crate) fn as_runtime(&self) -> &AlternativeCastingMethod {
        &self.runtime
    }

    pub(crate) fn into_runtime(self) -> AlternativeCastingMethod {
        self.runtime
    }
}

impl From<AlternativeCastingMethod> for ParsedAlternativeCastingMethodAst {
    fn from(value: AlternativeCastingMethod) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ParsedCardItem {
    Line(ParsedLineAst),
    Modal(ParsedModalAst),
    LevelAbility(ParsedLevelAbilityAst),
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedLineAst {
    pub(crate) info: LineInfo,
    pub(crate) chunks: Vec<LineAst>,
    pub(crate) restrictions: ParsedRestrictions,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedRestrictions {
    pub(crate) activation: Vec<String>,
    pub(crate) trigger: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalAst {
    pub(crate) header: ParsedModalHeader,
    pub(crate) modes: Vec<ParsedModalModeAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalHeader {
    pub(crate) min: Value,
    pub(crate) max: Option<Value>,
    pub(crate) same_mode_more_than_once: bool,
    pub(crate) mode_must_be_unchosen: bool,
    pub(crate) mode_must_be_unchosen_this_turn: bool,
    pub(crate) commander_allows_both: bool,
    pub(crate) trigger: Option<TriggerSpec>,
    pub(crate) activated: Option<ParsedModalActivatedHeader>,
    pub(crate) x_replacement: Option<Value>,
    pub(crate) prefix_effects_ast: Vec<EffectAst>,
    pub(crate) modal_gate: Option<ParsedModalGate>,
    pub(crate) line_text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalActivatedHeader {
    pub(crate) mana_cost: TotalCost,
    pub(crate) functional_zones: Vec<Zone>,
    pub(crate) timing: ActivationTiming,
    pub(crate) additional_restrictions: Vec<String>,
    pub(crate) activation_restrictions: Vec<ConditionExpr>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalModeAst {
    pub(crate) info: LineInfo,
    pub(crate) description: String,
    pub(crate) effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalGate {
    pub(crate) predicate: EffectPredicate,
    pub(crate) remove_mode_only: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedLevelAbilityAst {
    pub(crate) min_level: u32,
    pub(crate) max_level: Option<u32>,
    pub(crate) pt: Option<(i32, i32)>,
    pub(crate) items: Vec<ParsedLevelAbilityItemAst>,
}

#[derive(Debug, Clone)]
pub(crate) enum ParsedLevelAbilityItemAst {
    StaticAbilities(Vec<StaticAbilityAst>),
    KeywordActions(Vec<KeywordAction>),
}

use crate::ConditionExpr;
use crate::ability::{Ability, AbilityKind, ActivationTiming, ManaUsageRestriction};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cost::OptionalCost;
use crate::effect::{EffectPredicate, Value};
use crate::zone::Zone;

use super::super::{KeywordAction, TotalCost};
use super::ast::{EffectAst, StaticAbilityAst, TriggerSpec};
use super::reference_model::ReferenceImports;
use super::shared_types::{LineInfo, LineSemanticFacts};

#[derive(Debug, Clone)]
pub(crate) enum GiftTimingAst {
    SpellResolution,
    PermanentEtb,
}

#[derive(Debug, Clone)]
pub(crate) enum LineAst {
    Multiple(Vec<LineAst>),
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
pub(crate) struct AdditionalCostChoiceOptionAst<Effect = EffectAst> {
    pub(crate) description: String,
    pub(crate) effects: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq)]
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

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedAbility {
    pub(crate) ability: ParsedAbilityRuntime,
    pub(crate) text: Option<String>,
    pub(crate) effects_ast: Option<Vec<EffectAst>>,
    pub(crate) reference_imports: ReferenceImports,
    pub(crate) trigger_spec: Option<TriggerSpec>,
}

impl ParsedAbility {
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
        &self.text
    }

    pub(crate) fn text_mut(&mut self) -> &mut Option<String> {
        &mut self.text
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

    #[cfg(test)]
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
    pub(crate) semantic_facts: LineSemanticFacts,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedRestrictions {
    pub(crate) activation: Vec<ParsedActivationRestriction>,
    pub(crate) trigger: Vec<ParsedTriggerRestriction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivationRestrictionNormalizationFact {
    Preserve,
    Redundant,
    Residual(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedActivationRestriction {
    /// Normalized Oracle surface retained only for presentation/fallback behavior.
    pub(crate) presentation_text: String,
    pub(crate) timing: Option<ActivationTiming>,
    pub(crate) condition: Option<ConditionExpr>,
    pub(crate) text_only_condition: Option<ConditionExpr>,
    pub(crate) normalization: ActivationRestrictionNormalizationFact,
    pub(crate) mana_usage_restriction: Option<ManaUsageRestriction>,
    /// Oracle placed the once-per-turn clause after another activation
    /// restriction (for example, "... only if ... and only once each turn").
    pub(crate) once_per_turn_after_other_restrictions: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedTriggerRestriction {
    pub(crate) presentation_text: String,
    pub(crate) max_times_each_turn: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedManaRestriction {
    /// Normalized Oracle surface retained for diagnostics and unsupported fallback behavior.
    pub(crate) presentation_text: String,
    pub(crate) timing: ActivationTiming,
    pub(crate) condition: Option<ConditionExpr>,
    pub(crate) usage_restriction: Option<ManaUsageRestriction>,
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
    pub(crate) spree: bool,
    pub(crate) tiered: bool,
    pub(crate) weighted_mode_points: bool,
    pub(crate) random: bool,
    pub(crate) same_mode_more_than_once: bool,
    pub(crate) mode_must_be_unchosen: bool,
    pub(crate) mode_must_be_unchosen_this_turn: bool,
    pub(crate) distinct_player_targets_per_mode: bool,
    pub(crate) if_kicked_choose_any_number: bool,
    pub(crate) conditional_mode_change: Option<ParsedConditionalModeChange>,
    pub(crate) presentation_label: Option<crate::ability::PresentationLabel>,
    pub(crate) commander_allows_both: bool,
    pub(crate) choose_both_control_card_types: Vec<crate::types::CardType>,
    pub(crate) choose_both_exact_life_total: Option<i32>,
    pub(crate) trigger: Option<TriggerSpec>,
    pub(crate) activated: Option<ParsedModalActivatedHeader>,
    pub(crate) x_replacement: Option<Value>,
    pub(crate) prefix_effects_ast: Vec<EffectAst>,
    /// Typed effects authored after `choose ... and` and before the mode list.
    pub(crate) common_prefix_effects_ast: Vec<EffectAst>,
    /// Typed effects authored after the modal choice sentence but before the
    /// bullet list. Semantic lowering specializes target-dependent suffixes
    /// into every mode while retaining their shared presentation boundary.
    pub(crate) common_suffix_effects_ast: Vec<EffectAst>,
    pub(crate) modal_gate: Option<ParsedModalGate>,
    pub(crate) line_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionalModeSelection {
    BothOrTwo,
    AnyNumber,
    OneOrMore,
    One,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedConditionalModeChange {
    pub(crate) condition: crate::cards::builders::PredicateAst,
    pub(crate) selection: ConditionalModeSelection,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalActivatedHeader {
    pub(crate) mana_cost: TotalCost,
    pub(crate) functional_zones: Vec<Zone>,
    pub(crate) timing: ActivationTiming,
    pub(crate) is_loyalty_ability: bool,
    pub(crate) additional_restrictions: Vec<String>,
    pub(crate) activation_restrictions: Vec<ConditionExpr>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalModeAst {
    pub(crate) info: LineInfo,
    pub(crate) description: String,
    pub(crate) point_cost: Option<u32>,
    pub(crate) additional_mana_cost: Option<crate::mana::ManaCost>,
    pub(crate) effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedModalGate {
    pub(crate) predicate: EffectPredicate,
    pub(crate) remove_mode_only: bool,
    pub(crate) reflexive: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedLevelAbilityAst {
    pub(crate) min_level: u32,
    pub(crate) max_level: Option<u32>,
    pub(crate) pt: Option<(i32, i32)>,
    pub(crate) items: Vec<ParsedLevelAbilityItemAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedLevelActivatedAbilityAst {
    pub(crate) info: LineInfo,
    pub(crate) chunk: LineAst,
    pub(crate) restrictions: ParsedRestrictions,
}

#[derive(Debug, Clone)]
pub(crate) enum ParsedLevelAbilityItemAst {
    StaticAbilities(Vec<StaticAbilityAst>),
    KeywordActions(Vec<KeywordAction>),
    ActivatedAbility(ParsedLevelActivatedAbilityAst),
}

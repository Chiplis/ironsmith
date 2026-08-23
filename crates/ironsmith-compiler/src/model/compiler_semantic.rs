use crate::ConditionExpr;
use crate::ability::{Ability, AbilityKind, ActivationTiming, ManaUsageRestriction};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cost::OptionalCost;
use crate::effect::{EffectPredicate, Value};
use crate::zone::Zone;

use super::ast::{EffectAst, StaticAbilityAst, TriggerSpec};
use super::facts::{LineInfo, LineSemanticFacts};
use super::reference_state::ReferenceImports;
use crate::{KeywordAction, TotalCost};

#[derive(Debug, Clone)]
pub enum GiftTimingAst {
    SpellResolution,
    PermanentEtb,
}

#[derive(Debug, Clone)]
pub enum LineAst {
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
pub struct AdditionalCostChoiceOptionAst<Effect = EffectAst> {
    pub description: String,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAbility {
    pub ability:
        crate::model::CompilerAbilityPayload<Ability, EffectAst, ReferenceImports, TriggerSpec>,
    pub text: Option<String>,
    pub effects_ast: Option<Vec<EffectAst>>,
    pub reference_imports: ReferenceImports,
    pub trigger_spec: Option<TriggerSpec>,
}

impl ParsedAbility {
    pub fn runtime(&self) -> &Ability {
        self.ability.legacy()
    }

    pub fn runtime_mut(&mut self) -> &mut Ability {
        self.ability.legacy_mut()
    }

    pub fn into_runtime(self) -> Ability {
        self.ability.into_legacy()
    }

    pub fn kind(&self) -> &AbilityKind {
        &self.runtime().kind
    }

    pub fn kind_mut(&mut self) -> &mut AbilityKind {
        &mut self.runtime_mut().kind
    }

    pub fn text(&self) -> &Option<String> {
        &self.text
    }

    pub fn text_mut(&mut self) -> &mut Option<String> {
        &mut self.text
    }

    pub fn functional_zones_mut(&mut self) -> &mut Vec<Zone> {
        &mut self.runtime_mut().functional_zones
    }
}

impl From<Ability>
    for crate::model::CompilerAbilityPayload<Ability, EffectAst, ReferenceImports, TriggerSpec>
{
    fn from(value: Ability) -> Self {
        Self::from_legacy(value, None, ReferenceImports::default(), None)
    }
}

#[derive(Debug, Clone)]
pub struct ParsedOptionalCostAst {
    runtime: OptionalCost,
}

impl ParsedOptionalCostAst {
    pub fn new(runtime: OptionalCost) -> Self {
        Self { runtime }
    }

    pub fn into_runtime(self) -> OptionalCost {
        self.runtime
    }
}

impl From<OptionalCost> for ParsedOptionalCostAst {
    fn from(value: OptionalCost) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone)]
pub struct ParsedAlternativeCastingMethodAst {
    runtime: AlternativeCastingMethod,
}

impl ParsedAlternativeCastingMethodAst {
    pub fn new(runtime: AlternativeCastingMethod) -> Self {
        Self { runtime }
    }

    pub fn as_runtime(&self) -> &AlternativeCastingMethod {
        &self.runtime
    }

    pub fn into_runtime(self) -> AlternativeCastingMethod {
        self.runtime
    }
}

impl From<AlternativeCastingMethod> for ParsedAlternativeCastingMethodAst {
    fn from(value: AlternativeCastingMethod) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone)]
pub enum ParsedCardItem {
    Line(ParsedLineAst),
    Modal(ParsedModalAst),
    LevelAbility(ParsedLevelAbilityAst),
}

#[derive(Debug, Clone)]
pub struct ParsedLineAst {
    pub info: LineInfo,
    pub chunks: Vec<LineAst>,
    pub restrictions: ParsedRestrictions,
    pub semantic_facts: LineSemanticFacts,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedRestrictions {
    pub activation: Vec<ParsedActivationRestriction>,
    pub trigger: Vec<ParsedTriggerRestriction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivationRestrictionNormalizationFact {
    Preserve,
    Redundant,
    Residual(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedActivationRestriction {
    /// Normalized Oracle surface retained only for presentation/fallback behavior.
    pub presentation_text: String,
    pub timing: Option<ActivationTiming>,
    pub condition: Option<ConditionExpr>,
    pub text_only_condition: Option<ConditionExpr>,
    pub normalization: ActivationRestrictionNormalizationFact,
    pub mana_usage_restriction: Option<ManaUsageRestriction>,
    /// Oracle placed the once-per-turn clause after another activation
    /// restriction (for example, "... only if ... and only once each turn").
    pub once_per_turn_after_other_restrictions: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTriggerRestriction {
    pub presentation_text: String,
    pub max_times_each_turn: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedManaRestriction {
    /// Normalized Oracle surface retained for diagnostics and unsupported fallback behavior.
    pub presentation_text: String,
    pub timing: ActivationTiming,
    pub condition: Option<ConditionExpr>,
    pub usage_restriction: Option<ManaUsageRestriction>,
}

#[derive(Debug, Clone)]
pub struct ParsedModalAst {
    pub header: ParsedModalHeader,
    pub modes: Vec<ParsedModalModeAst>,
}

#[derive(Debug, Clone)]
pub struct ParsedModalHeader {
    pub min: Value,
    pub max: Option<Value>,
    pub spree: bool,
    pub tiered: bool,
    pub weighted_mode_points: bool,
    pub random: bool,
    pub same_mode_more_than_once: bool,
    pub mode_must_be_unchosen: bool,
    pub mode_must_be_unchosen_this_turn: bool,
    pub distinct_player_targets_per_mode: bool,
    pub if_kicked_choose_any_number: bool,
    pub conditional_mode_change: Option<ParsedConditionalModeChange>,
    pub presentation_label: Option<crate::ability::PresentationLabel>,
    pub commander_allows_both: bool,
    pub choose_both_control_card_types: Vec<crate::types::CardType>,
    pub choose_both_exact_life_total: Option<i32>,
    pub trigger: Option<TriggerSpec>,
    pub activated: Option<ParsedModalActivatedHeader>,
    pub x_replacement: Option<Value>,
    pub prefix_effects_ast: Vec<EffectAst>,
    /// Typed effects authored after `choose ... and` and before the mode list.
    pub common_prefix_effects_ast: Vec<EffectAst>,
    /// Typed effects authored after the modal choice sentence but before the
    /// bullet list. Semantic lowering specializes target-dependent suffixes
    /// into every mode while retaining their shared presentation boundary.
    pub common_suffix_effects_ast: Vec<EffectAst>,
    pub modal_gate: Option<ParsedModalGate>,
    pub line_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalModeSelection {
    BothOrTwo,
    AnyNumber,
    OneOrMore,
    One,
}

#[derive(Debug, Clone)]
pub struct ParsedConditionalModeChange {
    pub condition: crate::cards::builders::PredicateAst,
    pub selection: ConditionalModeSelection,
}

#[derive(Debug, Clone)]
pub struct ParsedModalActivatedHeader {
    pub mana_cost: TotalCost,
    pub functional_zones: Vec<Zone>,
    pub timing: ActivationTiming,
    pub is_loyalty_ability: bool,
    pub additional_restrictions: Vec<String>,
    pub activation_restrictions: Vec<ConditionExpr>,
}

#[derive(Debug, Clone)]
pub struct ParsedModalModeAst {
    pub info: LineInfo,
    pub description: String,
    pub point_cost: Option<u32>,
    pub additional_mana_cost: Option<crate::mana::ManaCost>,
    pub effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub struct ParsedModalGate {
    pub predicate: EffectPredicate,
    pub remove_mode_only: bool,
    pub reflexive: bool,
}

#[derive(Debug, Clone)]
pub struct ParsedLevelAbilityAst {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub pt: Option<(i32, i32)>,
    pub items: Vec<ParsedLevelAbilityItemAst>,
}

#[derive(Debug, Clone)]
pub struct ParsedLevelActivatedAbilityAst {
    pub info: LineInfo,
    pub chunk: LineAst,
    pub restrictions: ParsedRestrictions,
}

#[derive(Debug, Clone)]
pub enum ParsedLevelAbilityItemAst {
    StaticAbilities(Vec<StaticAbilityAst>),
    KeywordActions(Vec<KeywordAction>),
    ActivatedAbility(ParsedLevelActivatedAbilityAst),
}

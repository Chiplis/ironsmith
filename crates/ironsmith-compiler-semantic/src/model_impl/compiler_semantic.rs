use crate::ability::ActivationTiming;
use crate::effect::{EffectPredicate, Value};
use crate::zone::Zone;

use super::ast::{EffectAst, StaticAbilityAst, TriggerSpec};
use super::facts::{LineInfo, LineSemanticFacts};
use super::reference_state::ReferenceImports;
use crate::KeywordAction;

pub type CompilerAbilityCore = ironsmith_core::Ability<
    crate::model::CompilerStaticAbilityCore,
    TriggerSpec,
    EffectAst,
    crate::model::CompilerCost,
    super::ast::PredicateAst,
>;
pub type CompilerAbilityKindCore = ironsmith_core::AbilityKind<
    crate::model::CompilerStaticAbilityCore,
    TriggerSpec,
    EffectAst,
    crate::model::CompilerCost,
    super::ast::PredicateAst,
>;
/// A triggered ability as recognition produces it.
///
/// The intervening-if is the recognized [`PredicateAst`], not a resolved
/// condition: what a recognizer read is a fact about the text, and binding it
/// to the game state is the resolver's job.
pub type CompilerTriggeredAbilityCore =
    ironsmith_core::TriggeredAbility<TriggerSpec, EffectAst, super::ast::PredicateAst>;
/// An activated ability as recognition produces it.
///
/// The activation condition is the recognized predicate, like a triggered
/// ability's intervening-if. Binding it happens once, where lowering turns the
/// recognized ability into the runtime one.
pub type CompilerActivatedAbilityCore = ironsmith_core::ActivatedAbility<
    EffectAst,
    crate::model::CompilerCost,
    super::ast::PredicateAst,
>;
pub type CompilerManaUsageRestriction = ironsmith_core::ManaUsageRestriction<EffectAst>;

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
        timing: GiftTimingAst,
    },
    OptionalCostWithCastTrigger {
        cost: ParsedOptionalCostAst,
        effects: Vec<EffectAst>,
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
    pub ability: Box<CompilerAbilityCore>,
    pub effects_ast: Option<Vec<EffectAst>>,
    pub reference_imports: ReferenceImports,
    pub trigger_spec: Option<Box<TriggerSpec>>,
}

impl ParsedAbility {
    pub fn kind(&self) -> &CompilerAbilityKindCore {
        &self.ability.kind
    }

    pub fn kind_mut(&mut self) -> &mut CompilerAbilityKindCore {
        &mut self.ability.kind
    }

    pub fn functional_zones_mut(&mut self) -> &mut Vec<Zone> {
        &mut self.ability.functional_zones
    }
}

pub type ParsedOptionalCostAst = crate::model::CompilerOptionalCost;
pub type ParsedAlternativeCastingMethodAst = crate::model::CompilerAlternativeCastingMethod;

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
    /// The condition the restriction line stated, as recognized.
    pub condition: Option<super::ast::PredicateAst>,
    /// A restriction that only affects rendering, as recognized.
    pub text_only_condition: Option<super::ast::PredicateAst>,
    pub normalization: ActivationRestrictionNormalizationFact,
    pub mana_usage_restriction: Option<CompilerManaUsageRestriction>,
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
    /// The condition the restriction line stated, as recognized.
    pub condition: Option<super::ast::PredicateAst>,
    pub usage_restriction: Option<CompilerManaUsageRestriction>,
}

#[derive(Debug, Clone)]
pub struct ParsedModalAst {
    pub header: ParsedModalHeader,
    pub modes: Vec<ParsedModalModeAst>,
}

#[derive(Debug, Clone)]
pub struct ParsedModalHeader {
    pub info: LineInfo,
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
    pub mana_cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    pub functional_zones: Vec<Zone>,
    pub timing: ActivationTiming,
    pub is_loyalty_ability: bool,
    pub once_per_turn: bool,
    /// The activation restrictions the header stated, as recognized.
    pub activation_restrictions: Vec<super::ast::PredicateAst>,
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
    pub semantic_facts: LineSemanticFacts,
}

#[derive(Debug, Clone)]
pub enum ParsedLevelAbilityItemAst {
    StaticAbilities(Vec<StaticAbilityAst>),
    KeywordActions(Vec<KeywordAction>),
    ActivatedAbility(ParsedLevelActivatedAbilityAst),
}

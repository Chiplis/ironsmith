use crate::TagKey;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::ParseAnnotations;
use crate::cards::builders::{
    CardDefinition, CardDefinitionBuilder, EffectAst, KeywordAction, PredicateAst,
};
use crate::cost::OptionalCost;
use crate::model::provenance::ProvenanceStore;
use crate::model::symbols::SymbolTable;

use crate::model::ast::{StaticAbilityAst, TriggerSpec};
use crate::model::compiler_semantic::{
    GiftTimingAst, ParsedAbility, ParsedLevelAbilityAst, ParsedModalHeader, ParsedRestrictions,
};
use crate::model::facts::{LineInfo, LineSemanticFacts};
use crate::model::reference_state::{
    AnnotatedEffectSequence, ReferenceEnv, ReferenceExports, ReferenceImports,
};

#[derive(Debug, Clone, PartialEq)]
pub enum EffectPreludeTag {
    AttachedSource(TagKey),
    TriggeringObject(TagKey),
    TriggeringAttacker(TagKey, crate::target::ObjectFilter),
    TriggeringBlockers(TagKey, crate::target::ObjectFilter),
    OtherBlockParticipant(TagKey, crate::target::ObjectFilter),
    OtherBlockParticipantMatchingSubject {
        tag: TagKey,
        subject: crate::target::ObjectFilter,
        other: crate::target::ObjectFilter,
    },
    TriggeringSource(TagKey),
    TriggeringDamageTarget(TagKey),
}

#[derive(Debug, Clone)]
pub struct PreparedPredicateForLowering {
    pub predicate: PredicateAst,
    pub reference_env: ReferenceEnv,
    pub saved_last_object_tag: Option<TagKey>,
}

#[derive(Debug, Clone)]
pub struct SourceSentenceSegment {
    pub effect_count: usize,
    pub leading_then: bool,
    pub starting_with_controller: bool,
}

#[derive(Debug, Clone)]
pub struct PreparedEffectsForLowering {
    /// Top-level semantic-effect span and typed leading connective for each
    /// authored source sentence. Empty means the source did not carry an
    /// independently verified multi-sentence boundary into preparation.
    pub source_sentence_segments: Vec<SourceSentenceSegment>,
    pub imports: ReferenceImports,
    pub initial_env: ReferenceEnv,
    pub annotated: AnnotatedEffectSequence,
    pub exports: ReferenceExports,
    pub prelude: Vec<EffectPreludeTag>,
    pub force_auto_tag_object_targets: bool,
}

#[derive(Debug, Clone)]
pub struct PreparedTriggeredEffectsForLowering {
    pub prepared: PreparedEffectsForLowering,
    pub intervening_if: Option<PreparedPredicateForLowering>,
}

#[derive(Debug, Clone)]
pub enum NormalizedPreparedAbility {
    Activated(PreparedEffectsForLowering),
    Triggered {
        trigger: TriggerSpec,
        prepared: PreparedTriggeredEffectsForLowering,
    },
}

#[derive(Debug, Clone)]
pub struct NormalizedParsedAbility {
    pub parsed: ParsedAbility,
    pub prepared: Option<NormalizedPreparedAbility>,
}

#[derive(Debug, Clone)]
pub struct NormalizedAdditionalCostChoiceOptionAst {
    pub description: String,
    pub effects_ast: Vec<EffectAst>,
    pub prepared: PreparedEffectsForLowering,
}

#[derive(Debug, Clone)]
pub struct NormalizedModalModeAst {
    pub info: LineInfo,
    pub description: String,
    pub point_cost: Option<u32>,
    pub additional_mana_cost: Option<crate::mana::ManaCost>,
    pub prepared: PreparedEffectsForLowering,
}

#[derive(Debug, Clone)]
pub struct NormalizedModalAst {
    pub header: ParsedModalHeader,
    pub prepared_prefix: Option<PreparedEffectsForLowering>,
    pub prepared_common_prefix: Option<PreparedEffectsForLowering>,
    pub modes: Vec<NormalizedModalModeAst>,
}

#[derive(Debug, Clone)]
pub enum NormalizedLineChunk {
    Abilities(Vec<KeywordAction>),
    StaticAbility(StaticAbilityAst),
    StaticAbilities(Vec<StaticAbilityAst>),
    Ability(NormalizedParsedAbility),
    Triggered {
        trigger: TriggerSpec,
        prepared: PreparedTriggeredEffectsForLowering,
        max_triggers_per_turn: Option<u32>,
    },
    Statement {
        effects_ast: Vec<EffectAst>,
        prepared: PreparedEffectsForLowering,
    },
    AdditionalCost {
        effects_ast: Vec<EffectAst>,
        prepared: PreparedEffectsForLowering,
    },
    OptionalCost(OptionalCost),
    GiftKeyword {
        cost: OptionalCost,
        prepared: PreparedEffectsForLowering,
        timing: GiftTimingAst,
    },
    OptionalCostWithCastTrigger {
        cost: OptionalCost,
        prepared: PreparedEffectsForLowering,
    },
    AdditionalCostChoice {
        options: Vec<NormalizedAdditionalCostChoiceOptionAst>,
    },
    AlternativeCastingMethod(AlternativeCastingMethod),
}

#[derive(Debug, Clone)]
pub struct NormalizedLineAst {
    pub info: LineInfo,
    pub chunks: Vec<NormalizedLineChunk>,
    pub restrictions: ParsedRestrictions,
    pub semantic_facts: LineSemanticFacts,
}

#[derive(Debug, Clone)]
pub enum NormalizedCardItem {
    Line(NormalizedLineAst),
    Modal(NormalizedModalAst),
    LevelAbility(ParsedLevelAbilityAst),
}

#[derive(Debug, Clone)]
pub struct NormalizedOverloadBranch {
    pub items: Vec<NormalizedCardItem>,
}

#[derive(Debug, Clone)]
pub struct NormalizedCleaveBranch {
    pub items: Vec<NormalizedCardItem>,
}

#[derive(Debug, Clone)]
pub struct NormalizedCardAst {
    pub builder: CardDefinitionBuilder,
    pub annotations: ParseAnnotations,
    pub provenance: ProvenanceStore,
    pub symbols: SymbolTable,
    pub items: Vec<NormalizedCardItem>,
    pub overload_branch: Option<NormalizedOverloadBranch>,
    pub cleave_branch: Option<NormalizedCleaveBranch>,
    pub allow_unsupported: bool,
}

#[derive(Debug, Clone)]
pub struct LoweredCardDocument {
    /// The symbol table after lowering: every key the definition carries was
    /// bound here, by the grammar or by lowering itself.
    pub symbols: SymbolTable,
    pub definition: CardDefinition,
    pub annotations: ParseAnnotations,
}

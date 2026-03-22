use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::ParseAnnotations;
use crate::cards::builders::{
    CardDefinitionBuilder, CardTextError, EffectAst, KeywordAction, LineInfo, ParsedAbility,
    ParsedLevelAbilityAst, ParsedModalHeader, ParsedRestrictions, PredicateAst, StaticAbilityAst,
    TriggerSpec,
};
use crate::cost::OptionalCost;
use crate::{CardDefinition, TagKey};

use super::reference_model::{
    AnnotatedEffectSequence, ReferenceEnv, ReferenceExports, ReferenceImports,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EffectPreludeTag {
    AttachedSource(TagKey),
    TriggeringObject(TagKey),
    TriggeringDamageTarget(TagKey),
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedPredicateForLowering {
    pub(crate) predicate: PredicateAst,
    pub(crate) reference_env: ReferenceEnv,
    pub(crate) saved_last_object_tag: Option<TagKey>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedEffectsForLowering {
    pub(crate) effects: Vec<EffectAst>,
    pub(crate) imports: ReferenceImports,
    pub(crate) initial_env: ReferenceEnv,
    pub(crate) annotated: AnnotatedEffectSequence,
    pub(crate) exports: ReferenceExports,
    pub(crate) prelude: Vec<EffectPreludeTag>,
    pub(crate) force_auto_tag_object_targets: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedTriggeredEffectsForLowering {
    pub(crate) prepared: PreparedEffectsForLowering,
    pub(crate) intervening_if: Option<PreparedPredicateForLowering>,
}

#[derive(Debug, Clone)]
pub(crate) enum NormalizedPreparedAbility {
    Activated(PreparedEffectsForLowering),
    Triggered {
        trigger: TriggerSpec,
        prepared: PreparedTriggeredEffectsForLowering,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedParsedAbility {
    pub(crate) parsed: ParsedAbility,
    pub(crate) prepared: Option<NormalizedPreparedAbility>,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedAdditionalCostChoiceOptionAst {
    pub(crate) description: String,
    pub(crate) effects_ast: Vec<EffectAst>,
    pub(crate) prepared: PreparedEffectsForLowering,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedModalModeAst {
    pub(crate) info: LineInfo,
    pub(crate) description: String,
    pub(crate) prepared: PreparedEffectsForLowering,
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedModalAst {
    pub(crate) header: ParsedModalHeader,
    pub(crate) prepared_prefix: Option<PreparedEffectsForLowering>,
    pub(crate) modes: Vec<NormalizedModalModeAst>,
}

#[derive(Debug, Clone)]
pub(crate) enum NormalizedLineChunk {
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
    OptionalCostWithCastTrigger {
        cost: OptionalCost,
        prepared: PreparedEffectsForLowering,
        followup_text: String,
    },
    AdditionalCostChoice {
        options: Vec<NormalizedAdditionalCostChoiceOptionAst>,
    },
    AlternativeCastingMethod(AlternativeCastingMethod),
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedLineAst {
    pub(crate) info: LineInfo,
    pub(crate) chunks: Vec<NormalizedLineChunk>,
    pub(crate) restrictions: ParsedRestrictions,
}

#[derive(Debug, Clone)]
pub(crate) enum NormalizedCardItem {
    Line(NormalizedLineAst),
    Modal(NormalizedModalAst),
    LevelAbility(ParsedLevelAbilityAst),
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedCardAst {
    pub(crate) builder: CardDefinitionBuilder,
    pub(crate) annotations: ParseAnnotations,
    pub(crate) items: Vec<NormalizedCardItem>,
    pub(crate) allow_unsupported: bool,
}

pub(crate) fn parse_text_with_annotations(
    builder: CardDefinitionBuilder,
    text: String,
    allow_unsupported: bool,
) -> Result<(CardDefinition, ParseAnnotations), CardTextError> {
    super::parse_text_with_annotations_rewrite_lowered(builder, text, allow_unsupported)
}

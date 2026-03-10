use crate::ability::ActivationTiming;
use crate::effect::{EffectPredicate, Value};
use crate::zone::Zone;

use super::{
    CardDefinitionBuilder, EffectAst, LineAst, LineInfo, ParseAnnotations, StaticAbilityAst,
    TotalCost, TriggerSpec,
};

#[derive(Debug, Clone)]
pub(crate) struct ParsedCardAst {
    pub(crate) builder: CardDefinitionBuilder,
    pub(crate) annotations: ParseAnnotations,
    pub(crate) items: Vec<ParsedCardItem>,
    pub(crate) allow_unsupported: bool,
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
    pub(crate) activation_restrictions: Vec<crate::ConditionExpr>,
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
    KeywordActions(Vec<super::KeywordAction>),
}

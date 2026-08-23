use super::grammar::activation_costs::ActivationCostCst;
use super::lexer::OwnedLexToken;
use crate::ability::PresentationLabel;
use crate::cards::builders::{EffectAst, LineAst, ParsedLevelAbilityItemAst, PredicateAst};
use crate::ir::ChosenOptionContext;
use crate::model::facts::{LineInfo, MetadataLine};

#[derive(Debug, Clone)]
pub enum KeywordLinePayloadCst {
    Ast(LineAst),
    Kicker {
        cost: crate::cost::TotalCost,
        label: Option<String>,
    },
}

impl KeywordLinePayloadCst {
    pub fn ast(ast: LineAst) -> Self {
        Self::Ast(ast)
    }

    pub fn kicker(cost: crate::cost::TotalCost) -> Self {
        Self::Kicker { cost, label: None }
    }

    pub fn set_kicker_label(&mut self, label: String) -> Result<(), String> {
        let Self::Kicker {
            label: current_label,
            ..
        } = self
        else {
            return Err("custom kicker label attached to a non-kicker payload".to_string());
        };
        *current_label = Some(label);
        Ok(())
    }

    pub fn to_line_ast(&self) -> LineAst {
        match self {
            Self::Ast(ast) => ast.clone(),
            Self::Kicker { cost, label } => {
                let cost = match label {
                    Some(label) => crate::cost::OptionalCost::custom(label, cost.clone()),
                    None => crate::cost::OptionalCost::kicker(cost.clone()),
                };
                LineAst::OptionalCost(cost.into())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RewriteDocumentCst {
    pub lines: Vec<RewriteLineCst>,
}

#[derive(Debug, Clone)]
pub enum RewriteLineCst {
    Metadata(MetadataLineCst),
    Keyword(KeywordLineCst),
    Activated(ActivatedLineCst),
    Triggered(TriggeredLineCst),
    Static(StaticLineCst),
    Statement(StatementLineCst),
    Modal(ModalBlockCst),
    LevelHeader(LevelHeaderCst),
    SagaChapter(SagaChapterLineCst),
    Unsupported(UnsupportedLineCst),
}

#[derive(Debug, Clone)]
pub struct MetadataLineCst {
    pub value: MetadataLine,
}

#[derive(Debug, Clone)]
pub struct KeywordLineCst {
    pub info: LineInfo,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub full_parse_tokens: Vec<OwnedLexToken>,
    pub kind: KeywordLineKindCst,
    pub payload: KeywordLinePayloadCst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordLineKindCst {
    AdditionalCost,
    AdditionalCostChoice,
    AlternativeCast,
    Bestow,
    Blitz,
    Bargain,
    Buyback,
    Channel,
    Craft,
    Cycling,
    Equip,
    Escape,
    Flashback,
    Harmonize,
    Kicker,
    Madness,
    Morph,
    Mutate,
    Multikicker,
    Replicate,
    Offspring,
    Reconfigure,
    Reinforce,
    Retrace,
    Squad,
    Splice,
    Transmute,
    Transfigure,
    Entwine,
    Escalate,
    Eternalize,
    Evoke,
    CastThisSpellOnly,
    Gift,
    Epic,
    Warp,
    ExertAttack,
    Exploit,
}

#[derive(Debug, Clone)]
pub struct ActivatedLineCst {
    pub info: LineInfo,
    pub cost: ActivationCostCst,
    pub cost_parse_tokens: Vec<OwnedLexToken>,
    pub effect_parse_tokens: Vec<OwnedLexToken>,
    pub presentation: Option<PresentationLabel>,
    pub chosen_option: Option<ChosenOptionContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerIntroCst {
    When,
    Whenever,
    At,
}

#[derive(Debug, Clone)]
pub struct TriggeredLineCst {
    pub info: LineInfo,
    pub full_text: String,
    pub full_parse_tokens: Vec<OwnedLexToken>,
    pub trigger_parse_tokens: Vec<OwnedLexToken>,
    pub effect_parse_tokens: Vec<OwnedLexToken>,
    pub intervening_if: Option<PredicateAst>,
    pub max_triggers_per_turn: Option<u32>,
    pub chosen_option: Option<ChosenOptionContext>,
    pub presentation: Option<PresentationLabel>,
}

#[derive(Debug, Clone)]
pub struct StaticLineCst {
    pub info: LineInfo,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub chosen_option: Option<ChosenOptionContext>,
    pub parsed: Option<LineAst>,
}

#[derive(Debug, Clone)]
pub struct StatementLineCst {
    pub info: LineInfo,
    pub text: String,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub parse_groups: Vec<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone)]
pub struct ModalBlockCst {
    pub header: LineInfo,
    pub header_tokens: Vec<OwnedLexToken>,
    pub modes: Vec<ModalModeCst>,
}

#[derive(Debug, Clone)]
pub struct ModalModeCst {
    pub info: LineInfo,
    pub text: String,
    pub point_cost: Option<u32>,
    pub additional_mana_cost: Option<crate::mana::ManaCost>,
    pub effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub struct LevelHeaderCst {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub pt: Option<(i32, i32)>,
    pub items: Vec<LevelItemCst>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelItemKindCst {
    KeywordActions,
    StaticAbilities,
    ActivatedAbility,
}

#[derive(Debug, Clone)]
pub struct LevelItemCst {
    pub info: LineInfo,
    pub text: String,
    pub kind: LevelItemKindCst,
    pub parsed: ParsedLevelAbilityItemAst,
}

#[derive(Debug, Clone)]
pub struct SagaChapterLineCst {
    pub info: LineInfo,
    pub chapters: Vec<u32>,
    pub presentation_label: Option<String>,
    pub text: String,
    pub effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub struct UnsupportedLineCst {
    pub info: LineInfo,
    pub reason_code: &'static str,
}

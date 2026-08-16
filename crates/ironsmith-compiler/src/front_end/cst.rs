use super::grammar::activation_costs::ActivationCostCst;
use super::lexer::OwnedLexToken;
use crate::ability::PresentationLabel;
use crate::cards::builders::{EffectAst, LineAst, ParsedLevelAbilityItemAst, PredicateAst};
use crate::ir::ChosenOptionContext;
use crate::model::facts::{LineInfo, MetadataLine};

#[derive(Debug, Clone)]
pub(crate) enum KeywordLinePayloadCst {
    Ast(LineAst),
    Kicker {
        cost: crate::cost::TotalCost,
        label: Option<String>,
    },
}

impl KeywordLinePayloadCst {
    pub(crate) fn ast(ast: LineAst) -> Self {
        Self::Ast(ast)
    }

    pub(crate) fn kicker(cost: crate::cost::TotalCost) -> Self {
        Self::Kicker { cost, label: None }
    }

    pub(crate) fn set_kicker_label(&mut self, label: String) -> Result<(), String> {
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

    pub(crate) fn to_line_ast(&self) -> LineAst {
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
pub(crate) struct RewriteDocumentCst {
    pub(crate) lines: Vec<RewriteLineCst>,
}

#[derive(Debug, Clone)]
pub(crate) enum RewriteLineCst {
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
pub(crate) struct MetadataLineCst {
    pub(crate) value: MetadataLine,
}

#[derive(Debug, Clone)]
pub(crate) struct KeywordLineCst {
    pub(crate) info: LineInfo,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) full_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) kind: KeywordLineKindCst,
    pub(crate) payload: KeywordLinePayloadCst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordLineKindCst {
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
pub(crate) struct ActivatedLineCst {
    pub(crate) info: LineInfo,
    pub(crate) cost: ActivationCostCst,
    pub(crate) cost_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) effect_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) presentation: Option<PresentationLabel>,
    pub(crate) chosen_option: Option<ChosenOptionContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TriggerIntroCst {
    When,
    Whenever,
    At,
}

#[derive(Debug, Clone)]
pub(crate) struct TriggeredLineCst {
    pub(crate) info: LineInfo,
    pub(crate) full_text: String,
    pub(crate) full_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) trigger_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) effect_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) intervening_if: Option<PredicateAst>,
    pub(crate) max_triggers_per_turn: Option<u32>,
    pub(crate) chosen_option: Option<ChosenOptionContext>,
    pub(crate) presentation: Option<PresentationLabel>,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticLineCst {
    pub(crate) info: LineInfo,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) chosen_option: Option<ChosenOptionContext>,
    pub(crate) parsed: Option<LineAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct StatementLineCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) parse_groups: Vec<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModalBlockCst {
    pub(crate) header: LineInfo,
    pub(crate) header_tokens: Vec<OwnedLexToken>,
    pub(crate) modes: Vec<ModalModeCst>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModalModeCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) point_cost: Option<u32>,
    pub(crate) additional_mana_cost: Option<crate::mana::ManaCost>,
    pub(crate) effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct LevelHeaderCst {
    pub(crate) min_level: u32,
    pub(crate) max_level: Option<u32>,
    pub(crate) pt: Option<(i32, i32)>,
    pub(crate) items: Vec<LevelItemCst>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LevelItemKindCst {
    KeywordActions,
    StaticAbilities,
    ActivatedAbility,
}

#[derive(Debug, Clone)]
pub(crate) struct LevelItemCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) kind: LevelItemKindCst,
    pub(crate) parsed: ParsedLevelAbilityItemAst,
}

#[derive(Debug, Clone)]
pub(crate) struct SagaChapterLineCst {
    pub(crate) info: LineInfo,
    pub(crate) chapters: Vec<u32>,
    pub(crate) presentation_label: Option<String>,
    pub(crate) text: String,
    pub(crate) effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct UnsupportedLineCst {
    pub(crate) info: LineInfo,
    pub(crate) reason_code: &'static str,
}

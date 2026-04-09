use super::leaf::ActivationCostCst;
use super::lexer::OwnedLexToken;
use super::shared_types::{LineInfo, MetadataLine};
use crate::cards::builders::PredicateAst;

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
    pub(crate) text: String,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) kind: KeywordLineKindCst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordLineKindCst {
    AdditionalCost,
    AdditionalCostChoice,
    AlternativeCast,
    Bestow,
    Bargain,
    Buyback,
    Channel,
    Cycling,
    Equip,
    Escape,
    Flashback,
    Harmonize,
    Kicker,
    Madness,
    Morph,
    Multikicker,
    Offspring,
    Reinforce,
    Squad,
    Transmute,
    Entwine,
    CastThisSpellOnly,
    Gift,
    Warp,
    ExertAttack,
}

#[derive(Debug, Clone)]
pub(crate) struct ActivatedLineCst {
    pub(crate) info: LineInfo,
    pub(crate) cost: ActivationCostCst,
    pub(crate) cost_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) effect_text: String,
    pub(crate) effect_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) chosen_option_label: Option<String>,
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
    pub(crate) trigger_text: String,
    pub(crate) trigger_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) effect_text: String,
    pub(crate) effect_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) intervening_if: Option<PredicateAst>,
    pub(crate) max_triggers_per_turn: Option<u32>,
    pub(crate) chosen_option_label: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticLineCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) chosen_option_label: Option<String>,
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
    pub(crate) modes: Vec<ModalModeCst>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModalModeCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
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
}

#[derive(Debug, Clone)]
pub(crate) struct LevelItemCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) kind: LevelItemKindCst,
}

#[derive(Debug, Clone)]
pub(crate) struct SagaChapterLineCst {
    pub(crate) info: LineInfo,
    pub(crate) chapters: Vec<u32>,
    pub(crate) text: String,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone)]
pub(crate) struct UnsupportedLineCst {
    pub(crate) info: LineInfo,
    pub(crate) reason_code: &'static str,
}

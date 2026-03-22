use super::leaf::ActivationCostCst;
use super::shared_types::{LineInfo, MetadataLine};

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
    pub(crate) kind: KeywordLineKindCst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KeywordLineKindCst {
    AdditionalCost,
    AdditionalCostChoice,
    AlternativeCast,
    Bestow,
    Buyback,
    Channel,
    Cycling,
    Equip,
    Escape,
    Flashback,
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
    Warp,
}

#[derive(Debug, Clone)]
pub(crate) struct ActivatedLineCst {
    pub(crate) info: LineInfo,
    pub(crate) cost: ActivationCostCst,
    pub(crate) effect_text: String,
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
    pub(crate) trigger_text: String,
    pub(crate) effect_text: String,
    pub(crate) max_triggers_per_turn: Option<u32>,
    pub(crate) chosen_option_label: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StaticLineCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) chosen_option_label: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct StatementLineCst {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
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
    pub(crate) kind: LevelItemKindCst,
}

#[derive(Debug, Clone)]
pub(crate) struct SagaChapterLineCst {
    pub(crate) info: LineInfo,
    pub(crate) chapters: Vec<u32>,
    pub(crate) text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct UnsupportedLineCst {
    pub(crate) info: LineInfo,
    pub(crate) reason_code: &'static str,
}

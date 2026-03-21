#![allow(dead_code)]

use crate::ability::ActivationTiming;
use crate::cards::builders::{
    CardDefinitionBuilder, LineInfo, MetadataLine, ParseAnnotations, TotalCost,
};

#[derive(Debug, Clone)]
pub(crate) struct RewriteSemanticDocument {
    pub(crate) builder: CardDefinitionBuilder,
    pub(crate) annotations: ParseAnnotations,
    pub(crate) items: Vec<RewriteSemanticItem>,
    pub(crate) allow_unsupported: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum RewriteSemanticItem {
    Metadata(RewriteMetadataLine),
    Keyword(RewriteKeywordLine),
    Activated(RewriteActivatedLine),
    Triggered(RewriteTriggeredLine),
    Static(RewriteStaticLine),
    Statement(RewriteStatementLine),
    Modal(RewriteModalBlock),
    LevelHeader(RewriteLevelHeader),
    SagaChapter(RewriteSagaChapterLine),
    Unsupported(RewriteUnsupportedLine),
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteKeywordLine {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) kind: RewriteKeywordLineKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RewriteKeywordLineKind {
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
pub(crate) struct RewriteMetadataLine {
    pub(crate) info: LineInfo,
    pub(crate) value: MetadataLine,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteActivatedLine {
    pub(crate) info: LineInfo,
    pub(crate) cost: TotalCost,
    pub(crate) effect_text: String,
    pub(crate) timing_hint: ActivationTiming,
    pub(crate) chosen_option_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RewriteTriggerIntro {
    When,
    Whenever,
    At,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteTriggeredLine {
    pub(crate) info: LineInfo,
    pub(crate) intro: RewriteTriggerIntro,
    pub(crate) full_text: String,
    pub(crate) trigger_text: String,
    pub(crate) effect_text: String,
    pub(crate) max_triggers_per_turn: Option<u32>,
    pub(crate) chosen_option_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RewriteStaticLineKind {
    KeywordActions,
    StaticAbilities,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteStaticLine {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) kind: RewriteStaticLineKind,
    pub(crate) item_count: usize,
    pub(crate) chosen_option_label: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteStatementLine {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) sentence_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteModalBlock {
    pub(crate) header: LineInfo,
    pub(crate) modes: Vec<RewriteModalMode>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteModalMode {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteLevelHeader {
    pub(crate) info: LineInfo,
    pub(crate) min_level: u32,
    pub(crate) max_level: Option<u32>,
    pub(crate) pt: Option<(i32, i32)>,
    pub(crate) items: Vec<RewriteLevelItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RewriteLevelItemKind {
    KeywordActions,
    StaticAbilities,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteLevelItem {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) kind: RewriteLevelItemKind,
    pub(crate) item_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteSagaChapterLine {
    pub(crate) info: LineInfo,
    pub(crate) chapters: Vec<u32>,
    pub(crate) text: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteUnsupportedLine {
    pub(crate) info: LineInfo,
    pub(crate) reason_code: &'static str,
}

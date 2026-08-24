use crate::ability::{ActivationTiming, PresentationLabel};
use crate::cards::builders::{CardDefinitionBuilder, EffectAst, ParseAnnotations, PredicateAst};
use crate::color::Color;
use crate::cost::TotalCost;
use crate::model::provenance::ProvenanceStore;
use crate::model::symbols::SymbolTable;
use crate::types::Subtype;

use super::cst::{KeywordLineKindCst, KeywordLinePayloadCst};
use super::lexer::OwnedLexToken;
use crate::model::compiler_semantic::{ParsedLevelAbilityItemAst, ParsedLineAst};
use crate::model::facts::LineInfo;

#[derive(Debug, Clone)]
pub struct RewriteSemanticDocument {
    pub builder: CardDefinitionBuilder,
    pub annotations: ParseAnnotations,
    pub provenance: ProvenanceStore,
    pub symbols: SymbolTable,
    pub items: Vec<RewriteSemanticItem>,
    pub overload_items: Option<Vec<RewriteSemanticItem>>,
    pub cleave_items: Option<Vec<RewriteSemanticItem>>,
    pub allow_unsupported: bool,
}

/// Typed, document-wide facts that are recognized while the front end still owns
/// the lexed Oracle text. Later stages may consume these facts, but must not
/// rediscover them from the source text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentSemanticFacts {
    pub overload_rewrite: Option<OverloadRewritePayload>,
    pub cleave_rewrite: Option<CleaveRewritePayload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverloadRewritePayload {
    pub keyword_line_index: usize,
    pub target_spans: Vec<crate::cards::TextSpan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleaveRewritePayload {
    pub keyword_line_index: usize,
}

#[derive(Debug, Clone)]
pub enum RewriteSemanticItem {
    Metadata,
    Keyword(RewriteKeywordLine),
    ParsedLine(ParsedLineAst),
    Modal(RewriteModalBlock),
    LevelHeader(RewriteLevelHeader),
    SagaChapter(RewriteSagaChapterLine),
    Unsupported(RewriteUnsupportedLine),
}

/// A semantic condition attached to an Oracle choice or threshold label.
///
/// The front end recognizes these variants while it still owns the label or
/// line-family tokens. Preparation and lowering consume the typed fact instead
/// of decoding an internal string prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChosenOptionContext {
    SourceOption(String),
    MaxSpeed,
    StationThreshold(i32),
    /// Executable support synthesized for a Station row's implicit creature
    /// characteristics. It shares the row threshold but is not itself an
    /// authored `N+ | ...` surface.
    StationThresholdSupport(i32),
    ControlsSubtypePermanent(Subtype),
    ControlsEitherColorPermanent {
        left: Color,
        right: Color,
    },
}

impl ChosenOptionContext {
    pub fn source_option(label: impl Into<String>) -> Self {
        Self::SourceOption(label.into())
    }

    pub fn station_threshold(&self) -> Option<i32> {
        match self {
            Self::StationThreshold(threshold) => Some(*threshold),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RewriteKeywordLine {
    pub info: LineInfo,
    pub kind: RewriteKeywordLineKind,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub full_parse_tokens: Vec<OwnedLexToken>,
    pub payload: KeywordLinePayloadCst,
}

pub type RewriteKeywordLineKind = KeywordLineKindCst;

#[derive(Debug, Clone)]
pub struct RewriteActivatedLine {
    pub info: LineInfo,
    pub compiler_cost: crate::model::CompilerTotalCost,
    pub cost_parse_tokens: Vec<OwnedLexToken>,
    pub effect_parse_tokens: Vec<OwnedLexToken>,
    pub timing_hint: ActivationTiming,
    pub is_loyalty_ability: bool,
    pub functional_zones: Vec<crate::zone::Zone>,
    pub presentation_kind: Option<ActivatedPresentationKind>,
    pub presentation: Option<PresentationLabel>,
    pub chosen_option: Option<ChosenOptionContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivatedPresentationKind {
    Throw,
    ThrowEllipsis,
    Boast,
    Exhaust,
    Renew,
    Channel,
    Cohort,
    Teleport,
    Transmute,
}

impl ActivatedPresentationKind {
    pub fn display(self) -> &'static str {
        match self {
            Self::Throw => "Throw",
            Self::ThrowEllipsis => "Throw ...",
            Self::Boast => "Boast",
            Self::Exhaust => "Exhaust",
            Self::Renew => "Renew",
            Self::Channel => "Channel",
            Self::Cohort => "Cohort",
            Self::Teleport => "Teleport",
            Self::Transmute => "Transmute",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RewriteTriggeredLine {
    pub info: LineInfo,
    pub full_text: String,
    pub full_parse_tokens: Vec<OwnedLexToken>,
    pub intervening_if: Option<PredicateAst>,
    pub max_triggers_per_turn: Option<u32>,
    pub chosen_option: Option<ChosenOptionContext>,
    pub presentation: Option<PresentationLabel>,
}

#[derive(Debug, Clone)]
pub struct RewriteStaticLine {
    pub info: LineInfo,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub chosen_option: Option<ChosenOptionContext>,
}

#[derive(Debug, Clone)]
pub struct RewriteStatementLine {
    pub info: LineInfo,
    pub parse_tokens: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone)]
pub struct RewriteModalBlock {
    pub header: LineInfo,
    pub header_tokens: Vec<OwnedLexToken>,
    pub modes: Vec<RewriteModalMode>,
}

#[derive(Debug, Clone)]
pub struct RewriteModalMode {
    pub info: LineInfo,
    pub text: String,
    pub point_cost: Option<u32>,
    pub additional_mana_cost: Option<crate::mana::ManaCost>,
    pub effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub struct RewriteLevelHeader {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub pt: Option<(i32, i32)>,
    pub items: Vec<RewriteLevelItem>,
}

#[derive(Debug, Clone)]
pub struct RewriteLevelItem {
    pub parsed: ParsedLevelAbilityItemAst,
}

#[derive(Debug, Clone)]
pub struct RewriteSagaChapterLine {
    pub info: LineInfo,
    pub chapters: Vec<u32>,
    pub presentation_label: Option<PresentationLabel>,
    #[cfg(test)]
    pub text: String,
    pub effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub struct RewriteUnsupportedLine {
    pub info: LineInfo,
    pub reason_code: &'static str,
}

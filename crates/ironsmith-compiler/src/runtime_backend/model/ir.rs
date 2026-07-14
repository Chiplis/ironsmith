use crate::ability::{ActivationTiming, PresentationLabel};
use crate::cards::builders::{CardDefinitionBuilder, EffectAst, ParseAnnotations, PredicateAst};
use crate::color::Color;
use crate::cost::TotalCost;
use crate::types::Subtype;

use super::cst::{KeywordLineKindCst, KeywordLinePayloadCst};
use super::lexer::OwnedLexToken;
use super::semantic::{ParsedLevelAbilityItemAst, ParsedLineAst};
use super::shared_types::LineInfo;

#[derive(Debug, Clone)]
pub(crate) struct RewriteSemanticDocument {
    pub(crate) builder: CardDefinitionBuilder,
    pub(crate) annotations: ParseAnnotations,
    pub(crate) items: Vec<RewriteSemanticItem>,
    pub(crate) overload_items: Option<Vec<RewriteSemanticItem>>,
    pub(crate) allow_unsupported: bool,
}

/// Typed, document-wide facts that are recognized while the front end still owns
/// the lexed Oracle text. Later stages may consume these facts, but must not
/// rediscover them from the source text.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DocumentSemanticFacts {
    pub(crate) overload_rewrite: Option<OverloadRewritePayload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverloadRewritePayload {
    pub(crate) keyword_line_index: usize,
    pub(crate) target_spans: Vec<crate::cards::TextSpan>,
}

#[derive(Debug, Clone)]
pub(crate) enum RewriteSemanticItem {
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
pub(crate) enum ChosenOptionContext {
    SourceOption(String),
    MaxSpeed,
    StationThreshold(i32),
    ControlsSubtypePermanent(Subtype),
    ControlsEitherColorPermanent { left: Color, right: Color },
}

impl ChosenOptionContext {
    pub(crate) fn source_option(label: impl Into<String>) -> Self {
        Self::SourceOption(label.into())
    }

    pub(crate) fn station_threshold(&self) -> Option<i32> {
        match self {
            Self::StationThreshold(threshold) => Some(*threshold),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteKeywordLine {
    pub(crate) info: LineInfo,
    pub(crate) kind: RewriteKeywordLineKind,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) full_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) payload: KeywordLinePayloadCst,
}

pub(crate) type RewriteKeywordLineKind = KeywordLineKindCst;

#[derive(Debug, Clone)]
pub(crate) struct RewriteActivatedLine {
    pub(crate) info: LineInfo,
    pub(crate) cost: TotalCost,
    pub(crate) cost_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) effect_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) timing_hint: ActivationTiming,
    pub(crate) is_loyalty_ability: bool,
    pub(crate) functional_zones: Vec<crate::zone::Zone>,
    pub(crate) presentation_kind: Option<ActivatedPresentationKind>,
    pub(crate) presentation: Option<PresentationLabel>,
    pub(crate) chosen_option: Option<ChosenOptionContext>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivatedPresentationKind {
    Throw,
    Boast,
    Exhaust,
    Renew,
    Channel,
    Cohort,
    Teleport,
    Transmute,
}

impl ActivatedPresentationKind {
    pub(crate) fn display(self) -> &'static str {
        match self {
            Self::Throw => "Throw ...",
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
pub(crate) struct RewriteTriggeredLine {
    pub(crate) info: LineInfo,
    pub(crate) full_text: String,
    pub(crate) full_parse_tokens: Vec<OwnedLexToken>,
    pub(crate) intervening_if: Option<PredicateAst>,
    pub(crate) max_triggers_per_turn: Option<u32>,
    pub(crate) chosen_option: Option<ChosenOptionContext>,
    pub(crate) presentation: Option<PresentationLabel>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteStaticLine {
    pub(crate) info: LineInfo,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
    pub(crate) chosen_option: Option<ChosenOptionContext>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteStatementLine {
    pub(crate) info: LineInfo,
    pub(crate) parse_tokens: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteModalBlock {
    pub(crate) header: LineInfo,
    pub(crate) header_tokens: Vec<OwnedLexToken>,
    pub(crate) modes: Vec<RewriteModalMode>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteModalMode {
    pub(crate) info: LineInfo,
    pub(crate) text: String,
    pub(crate) point_cost: Option<u32>,
    pub(crate) effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteLevelHeader {
    pub(crate) min_level: u32,
    pub(crate) max_level: Option<u32>,
    pub(crate) pt: Option<(i32, i32)>,
    pub(crate) items: Vec<RewriteLevelItem>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteLevelItem {
    pub(crate) parsed: ParsedLevelAbilityItemAst,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteSagaChapterLine {
    pub(crate) info: LineInfo,
    pub(crate) chapters: Vec<u32>,
    #[cfg(test)]
    pub(crate) text: String,
    pub(crate) effects_ast: Vec<EffectAst>,
}

#[derive(Debug, Clone)]
pub(crate) struct RewriteUnsupportedLine {
    pub(crate) info: LineInfo,
    pub(crate) reason_code: &'static str,
}

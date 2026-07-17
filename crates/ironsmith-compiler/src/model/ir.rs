use crate::diagnostics::ParseAnnotations;
use crate::front_end::{KeywordLineKindCst, LineInfo, OwnedLexToken};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteSemanticDocument<Builder, Item> {
    pub builder: Builder,
    pub annotations: ParseAnnotations,
    pub items: Vec<Item>,
    pub allow_unsupported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteSemanticItem<
    KeywordAction,
    StaticAbility,
    ParsedAbility,
    Effect,
    TriggerSpec,
    OptionalCost,
    AlternativeCastingMethod,
    TotalCost,
    Predicate,
    LevelItem,
> {
    Metadata,
    Keyword(RewriteKeywordLine),
    Activated(RewriteActivatedLine<TotalCost>),
    Triggered(RewriteTriggeredLine<Predicate>),
    Static(RewriteStaticLine),
    Statement(RewriteStatementLine),
    Modal(RewriteModalBlock<Effect>),
    LevelHeader(RewriteLevelHeader<LevelItem>),
    SagaChapter(RewriteSagaChapterLine<Effect>),
    Unsupported(RewriteUnsupportedLine),
    _TypeMarker(
        std::marker::PhantomData<(
            KeywordAction,
            StaticAbility,
            ParsedAbility,
            TriggerSpec,
            OptionalCost,
            AlternativeCastingMethod,
        )>,
    ),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteKeywordLine {
    pub info: LineInfo,
    pub text: String,
    pub kind: KeywordLineKindCst,
    pub parse_tokens: Vec<OwnedLexToken>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteActivatedLine<TotalCost> {
    pub info: LineInfo,
    pub cost: TotalCost,
    pub cost_parse_tokens: Vec<OwnedLexToken>,
    pub effect_text: String,
    pub effect_parse_tokens: Vec<OwnedLexToken>,
    pub timing_hint: String,
    pub chosen_option_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteTriggeredLine<Predicate> {
    pub info: LineInfo,
    pub full_text: String,
    pub full_parse_tokens: Vec<OwnedLexToken>,
    pub trigger_text: String,
    pub trigger_parse_tokens: Vec<OwnedLexToken>,
    pub effect_text: String,
    pub effect_parse_tokens: Vec<OwnedLexToken>,
    pub intervening_if: Option<Predicate>,
    pub max_triggers_per_turn: Option<u32>,
    pub chosen_option_label: Option<String>,
    pub presentation_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteStaticLine {
    pub info: LineInfo,
    pub text: String,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub chosen_option_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteStatementLine {
    pub info: LineInfo,
    pub text: String,
    pub parse_tokens: Vec<OwnedLexToken>,
    pub parse_groups: Vec<Vec<OwnedLexToken>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteModalBlock<Effect> {
    pub header: LineInfo,
    pub modes: Vec<RewriteModalMode<Effect>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteModalMode<Effect> {
    pub info: LineInfo,
    pub text: String,
    pub effects_ast: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteLevelHeader<LevelItem> {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub pt: Option<(i32, i32)>,
    pub items: Vec<LevelItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewriteLevelItemKind {
    KeywordActions,
    StaticAbilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteLevelItem<Parsed> {
    pub info: LineInfo,
    pub text: String,
    pub kind: RewriteLevelItemKind,
    pub parsed: Parsed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteSagaChapterLine<Effect> {
    pub info: LineInfo,
    pub chapters: Vec<u32>,
    pub presentation_label: Option<String>,
    pub text: String,
    pub effects_ast: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteUnsupportedLine {
    pub info: LineInfo,
    pub reason_code: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::front_end::{make_line_info, normalize_trimmed_line};

    fn sample_line_info(raw: &str) -> LineInfo {
        let normalized = normalize_trimmed_line(raw).expect("sample line should normalize");
        make_line_info(0, raw.to_string(), normalized)
    }

    #[test]
    fn rewrite_semantic_document_keeps_annotations_and_items() {
        let document = RewriteSemanticDocument {
            builder: "builder".to_string(),
            annotations: ParseAnnotations::default(),
            items: vec![
                RewriteSemanticItem::<
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                    String,
                >::Metadata,
            ],
            allow_unsupported: true,
        };

        assert_eq!(document.builder, "builder");
        assert_eq!(document.items.len(), 1);
        assert!(document.allow_unsupported);
    }

    #[test]
    fn rewrite_modal_block_tracks_effect_descriptions() {
        let info = sample_line_info("Choose one —");
        let modal = RewriteModalBlock {
            header: info.clone(),
            modes: vec![RewriteModalMode {
                info,
                text: "Draw a card".to_string(),
                effects_ast: vec!["draw".to_string()],
            }],
        };

        assert_eq!(modal.modes[0].effects_ast, vec!["draw"]);
    }
}

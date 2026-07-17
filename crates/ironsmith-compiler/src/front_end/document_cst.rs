use crate::front_end::{
    KeywordLineCst, LineInfo, MetadataLineCst, OwnedLexToken, StatementLineCst, StaticLineCst,
    UnsupportedLineCst,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RewriteDocumentCst<ActivationCost, Predicate, Effect, LevelItem, Token = OwnedLexToken> {
    pub lines: Vec<RewriteLineCst<ActivationCost, Predicate, Effect, LevelItem, Token>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteLineCst<ActivationCost, Predicate, Effect, LevelItem, Token = OwnedLexToken> {
    Metadata(MetadataLineCst),
    Keyword(KeywordLineCst),
    Activated(ActivatedLineCst<ActivationCost, Token>),
    Triggered(TriggeredLineCst<Predicate, Token>),
    Static(StaticLineCst),
    Statement(StatementLineCst),
    Modal(ModalBlockCst<Effect>),
    LevelHeader(LevelHeaderCst<LevelItem>),
    SagaChapter(SagaChapterLineCst<Effect>),
    Unsupported(UnsupportedLineCst),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivatedLineCst<ActivationCost, Token = OwnedLexToken> {
    pub info: LineInfo,
    pub cost: ActivationCost,
    pub cost_parse_tokens: Vec<Token>,
    pub effect_text: String,
    pub effect_parse_tokens: Vec<Token>,
    pub chosen_option_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggeredLineCst<Predicate, Token = OwnedLexToken> {
    pub info: LineInfo,
    pub full_text: String,
    pub full_parse_tokens: Vec<Token>,
    pub trigger_text: String,
    pub trigger_parse_tokens: Vec<Token>,
    pub effect_text: String,
    pub effect_parse_tokens: Vec<Token>,
    pub intervening_if: Option<Predicate>,
    pub max_triggers_per_turn: Option<u32>,
    pub chosen_option_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalBlockCst<Effect> {
    pub header: LineInfo,
    pub modes: Vec<ModalModeCst<Effect>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalModeCst<Effect> {
    pub info: LineInfo,
    pub text: String,
    pub effects_ast: Vec<Effect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelHeaderCst<LevelItem> {
    pub min_level: u32,
    pub max_level: Option<u32>,
    pub pt: Option<(i32, i32)>,
    pub items: Vec<LevelItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelItemKindCst {
    KeywordActions,
    StaticAbilities,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelItemCst<Parsed> {
    pub info: LineInfo,
    pub text: String,
    pub kind: LevelItemKindCst,
    pub parsed: Parsed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SagaChapterLineCst<Effect> {
    pub info: LineInfo,
    pub chapters: Vec<u32>,
    pub presentation_label: Option<String>,
    pub text: String,
    pub effects_ast: Vec<Effect>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::TextSpan;
    use crate::front_end::{
        KeywordLineKindCst, OwnedLexToken, lex_line, make_line_info, normalize_trimmed_line,
    };

    fn sample_line_info(raw: &str) -> LineInfo {
        let normalized = normalize_trimmed_line(raw).expect("sample line should normalize");
        make_line_info(0, raw.to_string(), normalized)
    }

    #[test]
    fn rewrite_document_cst_can_hold_mixed_lines() {
        let info = sample_line_info("Flying");
        let keyword = KeywordLineCst {
            info: info.clone(),
            text: "Flying".to_string(),
            parse_tokens: lex_line("Flying", 0).expect("keyword line should lex"),
            kind: KeywordLineKindCst::AdditionalCost,
        };
        let modal = ModalBlockCst {
            header: info.clone(),
            modes: vec![ModalModeCst {
                info: info.clone(),
                text: "Draw a card".to_string(),
                effects_ast: vec!["draw".to_string()],
            }],
        };
        let document = RewriteDocumentCst::<String, String, String, String> {
            lines: vec![
                RewriteLineCst::Keyword(keyword),
                RewriteLineCst::Modal(modal),
                RewriteLineCst::Unsupported(UnsupportedLineCst {
                    info,
                    reason_code: "unsupported",
                }),
            ],
        };

        assert_eq!(document.lines.len(), 3);
        assert!(matches!(document.lines[1], RewriteLineCst::Modal(_)));
    }

    #[test]
    fn triggered_line_cst_tracks_intervening_if_and_tokens() {
        let info = sample_line_info("Whenever this attacks, draw a card.");
        let line = TriggeredLineCst::<String> {
            info,
            full_text: "Whenever this attacks, draw a card.".to_string(),
            full_parse_tokens: vec![OwnedLexToken::period(TextSpan::synthetic())],
            trigger_text: "Whenever this attacks".to_string(),
            trigger_parse_tokens: vec![OwnedLexToken::synthetic_word("whenever")],
            effect_text: "draw a card".to_string(),
            effect_parse_tokens: vec![OwnedLexToken::synthetic_word("draw")],
            intervening_if: Some("if you control a Wizard".to_string()),
            max_triggers_per_turn: Some(1),
            chosen_option_label: Some("A".to_string()),
        };

        assert_eq!(
            line.intervening_if.as_deref(),
            Some("if you control a Wizard")
        );
        assert_eq!(line.max_triggers_per_turn, Some(1));
    }
}

use crate::cards::CardDefinitionBuilder;
use crate::diagnostics::CardTextError;
use crate::ids::CardId;
use crate::runtime_backend::cst::{LevelItemKindCst, RewriteLineCst, UnsupportedLineCst};
use crate::runtime_backend::document_parser::parse_document_cst;
use crate::runtime_backend::ir::ChosenOptionContext;
use crate::runtime_backend::lexer::{OwnedLexToken, render_token_slice};
use crate::runtime_backend::model::LineInfo;
use crate::runtime_backend::preprocess::preprocess_document;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleGrammarDocument {
    pub lines: Vec<OracleGrammarLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleGrammarLineInfo {
    pub line_index: usize,
    pub display_line_index: usize,
    pub raw_line: String,
    pub normalized_line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleGrammarMode {
    pub info: OracleGrammarLineInfo,
    pub text: String,
    pub effects_debug: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleGrammarLevelItem {
    pub info: OracleGrammarLineInfo,
    pub text: String,
    pub kind: String,
    pub parsed_debug: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleGrammarLine {
    Metadata {
        kind: String,
        value: String,
    },
    Keyword {
        info: OracleGrammarLineInfo,
        kind: String,
        text: String,
        parse_text: String,
    },
    Activated {
        info: OracleGrammarLineInfo,
        cost_text: String,
        cost_debug: String,
        effect_text: String,
        effect_parse_text: String,
        chosen_option_label: Option<String>,
    },
    Triggered {
        info: OracleGrammarLineInfo,
        full_text: String,
        trigger_text: String,
        trigger_parse_text: String,
        effect_text: String,
        effect_parse_text: String,
        intervening_if_debug: Option<String>,
        max_triggers_per_turn: Option<u32>,
        chosen_option_label: Option<String>,
    },
    Static {
        info: OracleGrammarLineInfo,
        text: String,
        parse_text: String,
        chosen_option_label: Option<String>,
    },
    Statement {
        info: OracleGrammarLineInfo,
        text: String,
        parse_text: String,
        parse_group_texts: Vec<String>,
    },
    Modal {
        header: OracleGrammarLineInfo,
        modes: Vec<OracleGrammarMode>,
    },
    LevelHeader {
        min_level: u32,
        max_level: Option<u32>,
        pt: Option<(i32, i32)>,
        items: Vec<OracleGrammarLevelItem>,
    },
    SagaChapter {
        info: OracleGrammarLineInfo,
        chapters: Vec<u32>,
        text: String,
        effects_debug: Vec<String>,
    },
    Unsupported {
        info: OracleGrammarLineInfo,
        reason_code: String,
    },
}

pub fn parse_oracle_grammar_document(
    name: &str,
    text: impl AsRef<str>,
    allow_unsupported: bool,
) -> Result<OracleGrammarDocument, CardTextError> {
    let builder = CardDefinitionBuilder::new(CardId::from_raw(1), name);
    let preprocessed = preprocess_document(builder, text.as_ref())?;
    let cst = parse_document_cst(&preprocessed, allow_unsupported)?;
    Ok(OracleGrammarDocument {
        lines: cst.lines.into_iter().map(convert_line).collect(),
    })
}

fn convert_line(line: RewriteLineCst) -> OracleGrammarLine {
    match line {
        RewriteLineCst::Metadata(line) => {
            let (kind, value) = match line.value {
                crate::runtime_backend::model::MetadataLine::ManaCost(value) => ("ManaCost", value),
                crate::runtime_backend::model::MetadataLine::TypeLine(value) => ("TypeLine", value),
                crate::runtime_backend::model::MetadataLine::FirstPrintedSet(value) => {
                    ("FirstPrintedSet", value)
                }
                crate::runtime_backend::model::MetadataLine::AttractionLights(value) => {
                    ("AttractionLights", value)
                }
                crate::runtime_backend::model::MetadataLine::PowerToughness(value) => {
                    ("PowerToughness", value)
                }
                crate::runtime_backend::model::MetadataLine::Loyalty(value) => ("Loyalty", value),
                crate::runtime_backend::model::MetadataLine::Defense(value) => ("Defense", value),
            };
            OracleGrammarLine::Metadata {
                kind: kind.to_string(),
                value,
            }
        }
        RewriteLineCst::Keyword(line) => OracleGrammarLine::Keyword {
            info: convert_info(&line.info),
            kind: format!("{:?}", line.kind),
            text: render_tokens(&line.full_parse_tokens),
            parse_text: render_tokens(&line.parse_tokens),
        },
        RewriteLineCst::Activated(line) => OracleGrammarLine::Activated {
            info: convert_info(&line.info),
            cost_text: render_tokens(&line.cost_parse_tokens),
            cost_debug: format!("{:?}", line.cost),
            effect_text: render_tokens(&line.effect_parse_tokens),
            effect_parse_text: render_tokens(&line.effect_parse_tokens),
            chosen_option_label: chosen_option_surface(line.chosen_option),
        },
        RewriteLineCst::Triggered(line) => OracleGrammarLine::Triggered {
            info: convert_info(&line.info),
            full_text: line.full_text,
            trigger_text: render_tokens(&line.trigger_parse_tokens),
            trigger_parse_text: render_tokens(&line.trigger_parse_tokens),
            effect_text: render_tokens(&line.effect_parse_tokens),
            effect_parse_text: render_tokens(&line.effect_parse_tokens),
            intervening_if_debug: line
                .intervening_if
                .as_ref()
                .map(|predicate| format!("{predicate:?}")),
            max_triggers_per_turn: line.max_triggers_per_turn,
            chosen_option_label: chosen_option_surface(line.chosen_option),
        },
        RewriteLineCst::Static(line) => OracleGrammarLine::Static {
            info: convert_info(&line.info),
            text: render_tokens(&line.parse_tokens),
            parse_text: render_tokens(&line.parse_tokens),
            chosen_option_label: chosen_option_surface(line.chosen_option),
        },
        RewriteLineCst::Statement(line) => OracleGrammarLine::Statement {
            info: convert_info(&line.info),
            text: line.text,
            parse_text: render_tokens(&line.parse_tokens),
            parse_group_texts: line
                .parse_groups
                .iter()
                .map(|tokens| render_tokens(tokens))
                .collect(),
        },
        RewriteLineCst::Modal(block) => OracleGrammarLine::Modal {
            header: convert_info(&block.header),
            modes: block
                .modes
                .into_iter()
                .map(|mode| OracleGrammarMode {
                    info: convert_info(&mode.info),
                    text: mode.text,
                    effects_debug: mode
                        .effects_ast
                        .iter()
                        .map(|effect| format!("{effect:?}"))
                        .collect(),
                })
                .collect(),
        },
        RewriteLineCst::LevelHeader(level) => OracleGrammarLine::LevelHeader {
            min_level: level.min_level,
            max_level: level.max_level,
            pt: level.pt,
            items: level
                .items
                .into_iter()
                .map(|item| OracleGrammarLevelItem {
                    info: convert_info(&item.info),
                    text: item.text,
                    kind: match item.kind {
                        LevelItemKindCst::KeywordActions => "KeywordActions",
                        LevelItemKindCst::StaticAbilities => "StaticAbilities",
                        LevelItemKindCst::ActivatedAbility => "ActivatedAbility",
                    }
                    .to_string(),
                    parsed_debug: format!("{:?}", item.parsed),
                })
                .collect(),
        },
        RewriteLineCst::SagaChapter(line) => OracleGrammarLine::SagaChapter {
            info: convert_info(&line.info),
            chapters: line.chapters,
            text: line.text,
            effects_debug: line
                .effects_ast
                .iter()
                .map(|effect| format!("{effect:?}"))
                .collect(),
        },
        RewriteLineCst::Unsupported(UnsupportedLineCst { info, reason_code }) => {
            OracleGrammarLine::Unsupported {
                info: convert_info(&info),
                reason_code: reason_code.to_string(),
            }
        }
    }
}

fn chosen_option_surface(context: Option<ChosenOptionContext>) -> Option<String> {
    context.map(|context| match context {
        ChosenOptionContext::SourceOption(label) => label,
        other => format!("{other:?}"),
    })
}

fn convert_info(info: &LineInfo) -> OracleGrammarLineInfo {
    OracleGrammarLineInfo {
        line_index: info.line_index,
        display_line_index: info.display_line_index,
        raw_line: info.raw_line.clone(),
        normalized_line: info.normalized.normalized.clone(),
    }
}

fn render_tokens(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(tokens).trim().to_string()
}

//! Converts the typed rewrite/CST output into the semantic card AST.
//!
//! This is the final front-end stage. Preparation and runtime lowering consume
//! the returned [`ParsedCardAst`] and never inspect Oracle tokens themselves.

use crate::cards::builders::{
    CardTextError, LineAst, ParsedCardItem, ParsedLevelAbilityAst, ParsedLineAst,
    ParsedRestrictions, TriggerSpec,
};
use crate::static_abilities::StaticAbility;

use super::effect_pipeline::{ParsedCardAst, ParsedOverloadBranch};
use super::ir::{RewriteSemanticDocument, RewriteSemanticItem};
use super::semantic_line_parsing::rewrite_modal_to_parsed_item;

fn unsupported_line_ast(raw_line: &str, reason: impl Into<String>) -> LineAst {
    LineAst::StaticAbility(StaticAbility::unsupported_parser_line(raw_line, reason).into())
}

fn parse_rewrite_item(item: RewriteSemanticItem) -> Result<Option<ParsedCardItem>, CardTextError> {
    match item {
        RewriteSemanticItem::Metadata => Ok(None),
        RewriteSemanticItem::Keyword(line) => {
            let parsed = super::keyword_registry::lower_keyword_line_ast(&line)?;
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: line.info.clone(),
                chunks: vec![parsed],
                restrictions: ParsedRestrictions::default(),
                semantic_facts: line.info.semantic_facts.clone(),
            })))
        }
        RewriteSemanticItem::ParsedLine(line) => Ok(Some(ParsedCardItem::Line(line))),
        RewriteSemanticItem::Unsupported(line) => Ok(Some(ParsedCardItem::Line(ParsedLineAst {
            info: line.info.clone(),
            chunks: vec![unsupported_line_ast(
                line.info.raw_line.as_str(),
                line.reason_code,
            )],
            restrictions: ParsedRestrictions::default(),
            semantic_facts: line.info.semantic_facts.clone(),
        }))),
        RewriteSemanticItem::Modal(modal) => Ok(Some(rewrite_modal_to_parsed_item(modal)?)),
        RewriteSemanticItem::LevelHeader(level) => {
            Ok(Some(ParsedCardItem::LevelAbility(ParsedLevelAbilityAst {
                min_level: level.min_level,
                max_level: level.max_level,
                pt: level.pt,
                items: level.items.into_iter().map(|item| item.parsed).collect(),
            })))
        }
        RewriteSemanticItem::SagaChapter(saga) => Ok(Some(ParsedCardItem::Line(ParsedLineAst {
            info: saga.info.clone(),
            chunks: vec![LineAst::Triggered {
                trigger: TriggerSpec::SagaChapter(saga.chapters),
                effects: saga.effects_ast,
                max_triggers_per_turn: None,
            }],
            restrictions: ParsedRestrictions::default(),
            semantic_facts: saga.info.semantic_facts.clone(),
        }))),
    }
}

fn parse_rewrite_items(
    items: Vec<RewriteSemanticItem>,
) -> Result<Vec<ParsedCardItem>, CardTextError> {
    items
        .into_iter()
        .filter_map(|item| parse_rewrite_item(item).transpose())
        .collect()
}

pub(crate) fn parse_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<ParsedCardAst, CardTextError> {
    let RewriteSemanticDocument {
        builder,
        annotations,
        items,
        overload_items,
        allow_unsupported,
    } = doc;
    let overload_branch = overload_items
        .map(parse_rewrite_items)
        .transpose()?
        .map(|items| ParsedOverloadBranch { items });

    Ok(ParsedCardAst {
        builder,
        annotations,
        items: parse_rewrite_items(items)?,
        overload_branch,
        allow_unsupported,
    })
}

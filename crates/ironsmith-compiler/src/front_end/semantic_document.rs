//! Converts the typed rewrite/CST output into the semantic card AST.
//!
//! This is the final front-end stage. Preparation and runtime lowering consume
//! the returned [`ParsedCardAst`] and never inspect Oracle tokens themselves.

use crate::cards::builders::{
    CardTextError, LineAst, ParsedCardItem, ParsedLevelAbilityAst, ParsedLineAst,
    ParsedRestrictions, TriggerSpec,
};
use crate::model::{ParsedCardAst, ParsedCleaveBranch, ParsedOverloadBranch};
use crate::static_abilities::StaticAbility;

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
        RewriteSemanticItem::SagaChapter(saga) => {
            let mut info = saga.info;
            info.semantic_facts.triggered_ability.presentation_label = saga.presentation_label;
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: info.clone(),
                chunks: vec![LineAst::Triggered {
                    trigger: TriggerSpec::SagaChapter(saga.chapters),
                    effects: saga.effects_ast,
                    max_triggers_per_turn: None,
                }],
                restrictions: ParsedRestrictions::default(),
                semantic_facts: info.semantic_facts.clone(),
            })))
        }
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

pub fn parse_semantic_document(
    doc: RewriteSemanticDocument,
) -> Result<ParsedCardAst, CardTextError> {
    let RewriteSemanticDocument {
        builder,
        annotations,
        provenance,
        mut symbols,
        items,
        overload_items,
        cleave_items,
        allow_unsupported,
    } = doc;
    let overload_branch = overload_items
        .map(parse_rewrite_items)
        .transpose()?
        .map(|items| ParsedOverloadBranch { items });
    let cleave_branch = cleave_items
        .map(parse_rewrite_items)
        .transpose()?
        .map(|items| ParsedCleaveBranch { items });

    let mut items = parse_rewrite_items(items)?;
    let mut overload_branch = overload_branch;
    let mut cleave_branch = cleave_branch;
    super::front_end::semantic_domain_migration::migrate_semantic_domains(
        &mut items,
        &mut symbols,
    )?;
    if let Some(branch) = &mut overload_branch {
        super::front_end::semantic_domain_migration::migrate_semantic_domains(
            &mut branch.items,
            &mut symbols,
        )?;
    }
    if let Some(branch) = &mut cleave_branch {
        super::front_end::semantic_domain_migration::migrate_semantic_domains(
            &mut branch.items,
            &mut symbols,
        )?;
    }
    let mut reference_resolution =
        crate::model::canonical_references::resolve_parsed_items_references(&items, &symbols);
    if let Some(branch) = &overload_branch {
        reference_resolution.append(
            crate::model::canonical_references::resolve_parsed_items_references(
                &branch.items,
                &symbols,
            ),
        );
    }
    if let Some(branch) = &cleave_branch {
        reference_resolution.append(
            crate::model::canonical_references::resolve_parsed_items_references(
                &branch.items,
                &symbols,
            ),
        );
    }

    Ok(ParsedCardAst {
        builder,
        annotations,
        provenance,
        symbols,
        reference_resolution,
        items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    })
}

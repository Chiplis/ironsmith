use super::*;

pub(super) fn parse_rewrite_item(
    item: RewriteSemanticItem,
) -> Result<Option<ParsedCardItem>, CardTextError> {
    match item {
        RewriteSemanticItem::Metadata => Ok(None),
        RewriteSemanticItem::Keyword(line) => {
            let parsed = super::super::keyword_registry::lower_keyword_line_ast(&line)?;
            Ok(Some(ParsedCardItem::Line(ParsedLineAst {
                info: line.info.semantic_info(),
                chunks: vec![parsed],
                restrictions: ParsedRestrictions::default(),
                semantic_facts: line.info.semantic_facts.clone(),
            })))
        }
        RewriteSemanticItem::ParsedLine(line) => Ok(Some(ParsedCardItem::Line(line))),
        RewriteSemanticItem::Unsupported(line) => Ok(Some(ParsedCardItem::Line(ParsedLineAst {
            info: line.info.semantic_info(),
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
                info: info.semantic_info(),
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

pub(super) fn parse_rewrite_items(
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
        card,
        annotations,
        provenance,
        symbols,
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

    let items = parse_rewrite_items(items)?;
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
        card,
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

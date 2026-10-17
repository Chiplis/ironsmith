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

/// The display line an item was recognized on, when it has one; keys the
/// item's conversion mints bind in that line's symbol scope.
fn rewrite_item_display_line(item: &RewriteSemanticItem) -> Option<usize> {
    match item {
        RewriteSemanticItem::Keyword(line) => Some(line.info.display_line_index),
        RewriteSemanticItem::ParsedLine(line) => Some(line.info.display_line_index),
        RewriteSemanticItem::Unsupported(line) => Some(line.info.display_line_index),
        RewriteSemanticItem::SagaChapter(saga) => Some(saga.info.display_line_index),
        RewriteSemanticItem::Modal(modal) => Some(modal.header.display_line_index),
        RewriteSemanticItem::LevelHeader(level) => {
            level.items.iter().find_map(|item| match &item.parsed {
                crate::model::ParsedLevelAbilityItemAst::ActivatedAbility(activated)
                | crate::model::ParsedLevelAbilityItemAst::TriggeredAbility(activated) => {
                    Some(activated.info.display_line_index)
                }
                _ => None,
            })
        }
        RewriteSemanticItem::Metadata => None,
    }
}

pub(super) fn parse_rewrite_items(
    items: Vec<RewriteSemanticItem>,
    symbols: &std::cell::RefCell<ironsmith_compiler_ast::SymbolTable>,
) -> Result<Vec<ParsedCardItem>, CardTextError> {
    let mut parsed = Vec::with_capacity(items.len());
    for item in items {
        let scope =
            rewrite_item_display_line(&item).and_then(|line| symbols.borrow().line_scope(line));
        let _references = scope.map(|scope| {
            ironsmith_compiler_ast::reference_ledger::ReferenceScopeGuard::enter(symbols, scope)
        });
        if let Some(item) = parse_rewrite_item(item)? {
            parsed.push(item);
        }
    }
    Ok(parsed)
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
    // Conversion below still mints keys (keyword lines, saga chapters); they
    // bind in the scopes of the lines they came from.
    let symbols = std::cell::RefCell::new(symbols);
    let overload_branch = overload_items
        .map(|items| parse_rewrite_items(items, &symbols))
        .transpose()?
        .map(|items| ParsedOverloadBranch { items });
    let cleave_branch = cleave_items
        .map(|items| parse_rewrite_items(items, &symbols))
        .transpose()?
        .map(|items| ParsedCleaveBranch { items });

    let items = splice_attraction_prize_programs(parse_rewrite_items(items, &symbols)?);
    let symbols = symbols.into_inner();
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

/// An Attraction's "Prize — <instructions>" line is not an ability of its
/// own: "claim the prize" means following those instructions (Pick-a-Beeble).
/// Each claim instruction (which also performs the claim keyword action that
/// "whenever you claim the prize of an Attraction" observes) is followed by
/// the prize line's program, and the prize line is consumed.
fn splice_attraction_prize_programs(mut items: Vec<ParsedCardItem>) -> Vec<ParsedCardItem> {
    use crate::cards::builders::EffectAst;
    use crate::model::ast::{KeywordActionAst, SubjectVerbActionAst};
    fn is_claim(effect: &EffectAst) -> bool {
        matches!(
            effect,
            EffectAst::SubjectVerb(subject_verb)
                if matches!(
                    subject_verb.action,
                    SubjectVerbActionAst::KeywordActions(KeywordActionAst::EmitKeywordAction {
                        action: crate::events::KeywordActionKind::ClaimAttractionPrize,
                        ..
                    })
                )
        )
    }
    fn splice(effects: &mut Vec<EffectAst>, prize: &[EffectAst], spliced: &mut usize) {
        let mut idx = 0;
        while idx < effects.len() {
            if is_claim(&effects[idx]) {
                for (offset, effect) in prize.iter().enumerate() {
                    effects.insert(idx + 1 + offset, effect.clone());
                }
                *spliced += 1;
                idx += 1 + prize.len();
                continue;
            }
            crate::model::visit::for_each_nested_effect_vec_mut(&mut effects[idx], true, |nested| {
                splice(nested, prize, spliced)
            });
            idx += 1;
        }
    }
    let is_prize_line = |item: &ParsedCardItem| {
        matches!(item, ParsedCardItem::Line(line)
            if matches!(
                &line.semantic_facts.statement.presentation_label,
                Some(crate::ability::PresentationLabel::AbilityWord(word))
                    if word.eq_ignore_ascii_case("prize")
            ) && matches!(line.chunks.as_slice(), [LineAst::Statement { .. }]))
    };
    let Some(prize_idx) = items.iter().position(is_prize_line) else {
        return items;
    };
    let prize = match &items[prize_idx] {
        ParsedCardItem::Line(line) => match line.chunks.as_slice() {
            [LineAst::Statement { effects }] => effects.clone(),
            _ => return items,
        },
        _ => return items,
    };
    let mut spliced = 0usize;
    for (idx, item) in items.iter_mut().enumerate() {
        if idx == prize_idx {
            continue;
        }
        let ParsedCardItem::Line(line) = item else {
            continue;
        };
        for chunk in &mut line.chunks {
            match chunk {
                LineAst::Statement { effects } | LineAst::Triggered { effects, .. } => {
                    splice(effects, &prize, &mut spliced)
                }
                LineAst::Ability(ability) => {
                    if let Some(effects) = ability.effects_ast.as_mut() {
                        splice(effects, &prize, &mut spliced);
                    }
                }
                _ => {}
            }
        }
    }
    if spliced > 0 {
        items.remove(prize_idx);
    }
    items
}

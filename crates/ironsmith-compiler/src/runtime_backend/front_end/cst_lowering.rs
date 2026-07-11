use crate::ability::ActivationTiming;
use crate::cards::builders::{CardTextError, ParsedLineAst, ParsedRestrictions};

use super::cst::{
    ActivatedLineCst, KeywordLineCst, LevelItemKindCst, RewriteLineCst, SagaChapterLineCst,
    StatementLineCst, StaticLineCst, TriggeredLineCst,
};
use super::ir::{
    RewriteKeywordLine, RewriteLevelHeader, RewriteLevelItem, RewriteLevelItemKind,
    RewriteModalBlock, RewriteModalMode, RewriteSagaChapterLine, RewriteSemanticItem,
    RewriteUnsupportedLine,
};
use super::leaf::{ActivationCostCst, ActivationCostSegmentCst, lower_activation_cost_cst};
use super::lexer::render_token_slice;
use super::parser_support::split_tokens_for_parse;
use super::util::join_sentences_with_period;

fn parsed_line_item(
    info: super::shared_types::LineInfo,
    chunks: Vec<crate::cards::builders::LineAst>,
    restrictions: ParsedRestrictions,
) -> RewriteSemanticItem {
    let semantic_facts = info.semantic_facts.clone();
    RewriteSemanticItem::ParsedLine(ParsedLineAst {
        info,
        chunks,
        restrictions,
        semantic_facts,
    })
}

fn activation_cost_cst_is_loyalty(cost: &ActivationCostCst) -> bool {
    if cost.is_loyalty_shorthand {
        return true;
    }

    cost.segments.iter().any(|segment| {
        matches!(
            segment,
            ActivationCostSegmentCst::PutCounters {
                counter_type: crate::object::CounterType::Loyalty,
                ..
            } | ActivationCostSegmentCst::RemoveCounters {
                counter_type: crate::object::CounterType::Loyalty,
                ..
            } | ActivationCostSegmentCst::RemoveCountersDynamic {
                counter_type: Some(crate::object::CounterType::Loyalty),
                ..
            }
        )
    })
}

pub(crate) fn lower_non_metadata_rewrite_line_cst(
    line: RewriteLineCst,
    allow_unsupported: bool,
) -> Result<RewriteSemanticItem, CardTextError> {
    match line {
        RewriteLineCst::Metadata(_) => Err(CardTextError::InvariantViolation(
            "metadata lowering must stay in document_parser".to_string(),
        )),
        RewriteLineCst::Keyword(keyword) => Ok(RewriteSemanticItem::Keyword(RewriteKeywordLine {
            info: keyword.info,
            text: keyword.text,
            kind: keyword.kind,
            parse_tokens: keyword.parse_tokens,
            full_parse_tokens: keyword.full_parse_tokens,
            payload: keyword.payload,
        })),
        RewriteLineCst::Activated(activated) => lower_activated_line(activated, allow_unsupported),
        RewriteLineCst::Triggered(triggered) => lower_triggered_line(triggered),
        RewriteLineCst::Static(static_line) => lower_static_line(static_line),
        RewriteLineCst::Statement(statement_line) => lower_statement_line(statement_line),
        RewriteLineCst::Modal(modal) => lower_modal_block(modal),
        RewriteLineCst::LevelHeader(level) => lower_level_header(level),
        RewriteLineCst::SagaChapter(saga) => lower_saga_chapter(saga),
        RewriteLineCst::Unsupported(unsupported) => {
            crate::parse_loss::record(
                "allow_unsupported_cst_line",
                format!(
                    "{} ({})",
                    unsupported.info.raw_line.trim(),
                    unsupported.reason_code
                ),
            );
            Ok(RewriteSemanticItem::Unsupported(RewriteUnsupportedLine {
                info: unsupported.info,
                reason_code: unsupported.reason_code,
            }))
        }
    }
}

fn lower_activated_line(
    activated: ActivatedLineCst,
    allow_unsupported: bool,
) -> Result<RewriteSemanticItem, CardTextError> {
    let cost = match lower_activation_cost_cst(&activated.cost) {
        Ok(cost) => cost,
        Err(err) => {
            if allow_unsupported {
                crate::parse_loss::record(
                    "allow_unsupported_activated_cost",
                    format!("{} ({err:?})", activated.info.raw_line.trim()),
                );
                return Ok(RewriteSemanticItem::Unsupported(RewriteUnsupportedLine {
                    info: activated.info,
                    reason_code: "activated-cost-not-yet-supported",
                }));
            }
            return Err(err);
        }
    };
    let info = activated.info;
    let parsed = super::semantic_line_parsing::parse_activated_line(
        info.clone(),
        cost,
        activated.cost_parse_tokens,
        activated.effect_text,
        activated.effect_parse_tokens,
        ActivationTiming::AnyTime,
        activation_cost_cst_is_loyalty(&activated.cost),
        activated.presentation_label,
        activated.chosen_option,
    )?;
    Ok(parsed_line_item(
        info,
        vec![parsed.chunk],
        parsed.restrictions,
    ))
}

fn lower_triggered_line(triggered: TriggeredLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    let info = triggered.info;
    let parsed = super::lower::apply_explicit_intervening_if_to_triggered_chunk(
        super::semantic_line_parsing::parse_triggered_line(
            info.clone(),
            &triggered.full_text,
            &triggered.full_parse_tokens,
            &triggered.trigger_text,
            &triggered.trigger_parse_tokens,
            &triggered.effect_text,
            &triggered.effect_parse_tokens,
            triggered.intervening_if.clone(),
            triggered.presentation.as_ref(),
            triggered.max_triggers_per_turn,
            triggered.chosen_option.as_ref(),
        )?,
        triggered.intervening_if,
    )?;
    Ok(parsed_line_item(
        info,
        vec![parsed],
        ParsedRestrictions::default(),
    ))
}

fn lower_static_line(static_line: StaticLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    let info = static_line.info;
    if let Some(parsed) = static_line.parsed {
        return Ok(parsed_line_item(
            info,
            vec![parsed],
            ParsedRestrictions::default(),
        ));
    }
    let (parsed_sentences, restrictions) = split_tokens_for_parse(&static_line.parse_tokens);
    let chunks = if !restrictions.activation.is_empty() || !restrictions.trigger.is_empty() {
        if parsed_sentences.is_empty() {
            Vec::new()
        } else {
            let parsed_tokens = join_sentences_with_period(&parsed_sentences);
            let parsed_text = render_token_slice(&parsed_tokens).trim().to_string();
            vec![super::semantic_line_parsing::parse_static_line(
                info.clone(),
                &parsed_text,
                &parsed_tokens,
                static_line.chosen_option.as_ref(),
            )?]
        }
    } else {
        vec![super::semantic_line_parsing::parse_static_line(
            info.clone(),
            &static_line.text,
            &static_line.parse_tokens,
            static_line.chosen_option.as_ref(),
        )?]
    };
    Ok(parsed_line_item(info, chunks, restrictions))
}

fn lower_statement_line(
    statement_line: StatementLineCst,
) -> Result<RewriteSemanticItem, CardTextError> {
    let info = statement_line.info;
    let chunks = super::semantic_line_parsing::parse_statement_token_groups_to_chunks(
        info.clone(),
        &statement_line.text,
        &statement_line.parse_tokens,
        &statement_line.parse_groups,
    )?;
    Ok(parsed_line_item(
        info,
        chunks,
        ParsedRestrictions::default(),
    ))
}

fn lower_modal_block(
    modal: super::cst::ModalBlockCst,
) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::Modal(RewriteModalBlock {
        header: modal.header,
        header_tokens: modal.header_tokens,
        modes: modal
            .modes
            .into_iter()
            .map(|mode| RewriteModalMode {
                info: mode.info,
                text: mode.text,
                point_cost: mode.point_cost,
                effects_ast: mode.effects_ast,
            })
            .collect(),
    }))
}

fn lower_level_header(
    level: super::cst::LevelHeaderCst,
) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::LevelHeader(RewriteLevelHeader {
        min_level: level.min_level,
        max_level: level.max_level,
        pt: level.pt,
        items: level
            .items
            .into_iter()
            .map(|item| RewriteLevelItem {
                info: item.info,
                text: item.text,
                kind: match item.kind {
                    LevelItemKindCst::KeywordActions => RewriteLevelItemKind::KeywordActions,
                    LevelItemKindCst::StaticAbilities => RewriteLevelItemKind::StaticAbilities,
                    LevelItemKindCst::ActivatedAbility => RewriteLevelItemKind::ActivatedAbility,
                },
                parsed: item.parsed,
            })
            .collect(),
    }))
}

fn lower_saga_chapter(saga: SagaChapterLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::SagaChapter(RewriteSagaChapterLine {
        info: saga.info,
        chapters: saga.chapters,
        text: saga.text,
        effects_ast: saga.effects_ast,
    }))
}

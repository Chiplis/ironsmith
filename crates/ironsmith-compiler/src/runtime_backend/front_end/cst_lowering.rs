use crate::ability::{ActivationTiming, PresentationLabel};
use crate::cards::builders::{CardTextError, ParsedLineAst, ParsedRestrictions};

use super::cst::{
    ActivatedLineCst, RewriteLineCst, SagaChapterLineCst, StatementLineCst, StaticLineCst,
    TriggeredLineCst,
};
use super::grammar::activation_costs::{ActivationCostCst, ActivationCostSegmentCst};
use super::ir::{
    RewriteKeywordLine, RewriteLevelHeader, RewriteLevelItem, RewriteModalBlock, RewriteModalMode,
    RewriteSagaChapterLine, RewriteSemanticItem, RewriteUnsupportedLine,
};
use super::parser_support::split_tokens_for_parse;
use super::util::join_sentences_with_period;

#[path = "cst_lowering/activation_costs.rs"]
mod activation_costs;
pub(crate) use activation_costs::{lower_activation_cost_cst, recognize_activation_cost_cst};

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
    let presentation = activated.presentation.clone().or_else(|| {
        activated
            .cost
            .waterbend_generic
            .map(|generic| PresentationLabel::AbilityWord(format!("Waterbend {{{generic}}}")))
    });
    let compiler_cost = match activation_costs::recognize_activation_cost_cst(&activated.cost) {
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
    let cost = crate::runtime_backend::lowering::cost_materialization::materialize_compiler_total_cost(
        &compiler_cost,
    )?;
    let info = activated.info;
    let parsed = super::semantic_line_parsing::parse_activated_line(
        info.clone(),
        cost,
        compiler_cost,
        activated.cost_parse_tokens,
        activated.effect_parse_tokens,
        ActivationTiming::AnyTime,
        activation_cost_cst_is_loyalty(&activated.cost),
        presentation,
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
    let parsed = super::semantic_line_parsing::parse_triggered_line(
        info.clone(),
        &triggered.full_text,
        &triggered.full_parse_tokens,
        &triggered.trigger_parse_tokens,
        &triggered.effect_parse_tokens,
        triggered.intervening_if,
        triggered.presentation.as_ref(),
        triggered.max_triggers_per_turn,
        triggered.chosen_option.as_ref(),
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
            vec![super::semantic_line_parsing::parse_static_line(
                info.clone(),
                &parsed_tokens,
                static_line.chosen_option.as_ref(),
            )?]
        }
    } else {
        vec![super::semantic_line_parsing::parse_static_line(
            info.clone(),
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
                additional_mana_cost: mode.additional_mana_cost,
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
                parsed: item.parsed,
            })
            .collect(),
    }))
}

fn lower_saga_chapter(saga: SagaChapterLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::SagaChapter(RewriteSagaChapterLine {
        info: saga.info,
        chapters: saga.chapters,
        presentation_label: saga.presentation_label.map(PresentationLabel::AbilityWord),
        #[cfg(test)]
        text: saga.text,
        effects_ast: saga.effects_ast,
    }))
}

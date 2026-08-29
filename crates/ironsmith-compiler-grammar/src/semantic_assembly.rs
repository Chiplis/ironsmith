use crate::ability::{ActivationTiming, PresentationLabel};
use crate::cards::builders::{CardTextError, ParsedLineAst, ParsedRestrictions};

use super::grammar::activation_costs::{ActivationCostCst, ActivationCostSegmentCst};
use super::ir::{
    RewriteKeywordLine, RewriteLevelHeader, RewriteLevelItem, RewriteModalBlock, RewriteModalMode,
    RewriteSagaChapterLine, RewriteSemanticItem, RewriteUnsupportedLine,
};
use super::parser_support::split_tokens_for_parse;
use super::recognized_document::{
    RecognizedActivatedLine, RecognizedLine, RecognizedSagaChapterLine, RecognizedStatementLine,
    RecognizedStaticLine, RecognizedTriggeredLine,
};
use super::util::join_sentences_with_period;

pub mod activation_costs;
pub use activation_costs::assemble_activation_cost;

fn parsed_line_item(
    info: crate::line_info::LineInfo,
    chunks: Vec<crate::cards::builders::LineAst>,
    restrictions: ParsedRestrictions,
) -> RewriteSemanticItem {
    let semantic_facts = info.semantic_facts.clone();
    let semantic_info = info.semantic_info();
    RewriteSemanticItem::ParsedLine(ParsedLineAst {
        info: semantic_info,
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

pub fn assemble_non_metadata_line(
    line: RecognizedLine,
    allow_unsupported: bool,
) -> Result<RewriteSemanticItem, CardTextError> {
    match line {
        RecognizedLine::Metadata(_) => Err(CardTextError::InvariantViolation(
            "metadata lowering must stay in document_parser".to_string(),
        )),
        RecognizedLine::Keyword(keyword) => Ok(RewriteSemanticItem::Keyword(RewriteKeywordLine {
            info: keyword.info,
            kind: keyword.kind,
            parse_tokens: keyword.parse_tokens,
            full_parse_tokens: keyword.full_parse_tokens,
            payload: keyword.payload,
        })),
        RecognizedLine::Activated(activated) => {
            assemble_activated_line(activated, allow_unsupported)
        }
        RecognizedLine::Triggered(triggered) => assemble_triggered_line(triggered),
        RecognizedLine::Static(static_line) => assemble_static_line(static_line),
        RecognizedLine::Statement(statement_line) => assemble_statement_line(statement_line),
        RecognizedLine::Modal(modal) => assemble_modal_block(modal),
        RecognizedLine::LevelHeader(level) => assemble_level_header(level),
        RecognizedLine::SagaChapter(saga) => assemble_saga_chapter(saga),
        RecognizedLine::Unsupported(unsupported) => {
            crate::parse_loss::record(
                "allow_unsupported_recognized_line",
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

fn assemble_activated_line(
    activated: RecognizedActivatedLine,
    allow_unsupported: bool,
) -> Result<RewriteSemanticItem, CardTextError> {
    let presentation = activated.presentation.clone().or_else(|| {
        activated
            .cost
            .waterbend_generic
            .map(|generic| PresentationLabel::AbilityWord(format!("Waterbend {{{generic}}}")))
    });
    let compiler_cost = match activation_costs::assemble_activation_cost(&activated.cost) {
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
    let parsed = crate::semantic_line_parsing::parse_activated_line(
        info.clone(),
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

fn assemble_triggered_line(
    triggered: RecognizedTriggeredLine,
) -> Result<RewriteSemanticItem, CardTextError> {
    let mut info = triggered.info;
    let compiler_ability = *info
        .semantic_facts
        .triggered_ability
        .compiler_ability
        .take()
        .ok_or_else(|| {
            CardTextError::InvariantViolation(
                "triggered CST reached lowering without compiler trigger facts".to_string(),
            )
        })?;
    let chunk = crate::cards::builders::LineAst::Triggered {
        trigger: compiler_ability.event.semantics,
        effects: compiler_ability.effects,
        max_triggers_per_turn: triggered.max_triggers_per_turn,
    };
    let chunk = crate::semantic_line_parsing::apply_explicit_intervening_if_to_triggered_chunk(
        chunk,
        compiler_ability.intervening_if,
    )?;
    let chunk = crate::semantic_line_parsing::apply_chosen_option_to_triggered_chunk(
        chunk,
        &triggered.full_text,
        &info.semantic_facts.triggered_ability,
        triggered.max_triggers_per_turn,
        triggered.chosen_option.as_ref(),
        triggered.presentation.as_ref(),
    )?;
    Ok(parsed_line_item(
        info,
        vec![chunk],
        ParsedRestrictions::default(),
    ))
}

fn assemble_static_line(
    static_line: RecognizedStaticLine,
) -> Result<RewriteSemanticItem, CardTextError> {
    let info = static_line.info;
    if let Some(parsed) = static_line.parsed {
        let parsed = crate::semantic_line_parsing::wrap_chosen_option_static_chunk(
            *parsed,
            static_line.chosen_option.as_ref(),
        )?;
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
            vec![crate::semantic_line_parsing::parse_static_line(
                info.clone(),
                &parsed_tokens,
                static_line.chosen_option.as_ref(),
            )?]
        }
    } else {
        vec![crate::semantic_line_parsing::parse_static_line(
            info.clone(),
            &static_line.parse_tokens,
            static_line.chosen_option.as_ref(),
        )?]
    };
    Ok(parsed_line_item(info, chunks, restrictions))
}

fn assemble_statement_line(
    statement_line: RecognizedStatementLine,
) -> Result<RewriteSemanticItem, CardTextError> {
    let info = statement_line.info;
    let chunks = match statement_line.parsed_effects {
        Some(effects) => vec![crate::cards::builders::LineAst::Statement { effects }],
        None => crate::semantic_line_parsing::parse_statement_token_groups_to_chunks(
            info.clone(),
            &statement_line.parse_tokens,
            &statement_line.parse_groups,
        )?,
    };
    Ok(parsed_line_item(
        info,
        chunks,
        ParsedRestrictions::default(),
    ))
}

fn assemble_modal_block(
    modal: super::recognized_document::RecognizedModalBlock,
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

fn assemble_level_header(
    level: super::recognized_document::RecognizedLevelHeader,
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

fn assemble_saga_chapter(
    saga: RecognizedSagaChapterLine,
) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::SagaChapter(RewriteSagaChapterLine {
        info: saga.info,
        chapters: saga.chapters,
        presentation_label: saga.presentation_label.map(PresentationLabel::AbilityWord),
        #[cfg(test)]
        text: saga.text,
        effects_ast: saga.effects_ast,
    }))
}

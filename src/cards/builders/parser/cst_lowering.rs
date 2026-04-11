use crate::ability::ActivationTiming;
use crate::cards::builders::CardTextError;

use super::cst::{
    ActivatedLineCst, KeywordLineCst, LevelItemKindCst, RewriteLineCst, SagaChapterLineCst,
    StatementLineCst, StaticLineCst, TriggeredLineCst,
};
use super::ir::{
    RewriteActivatedLine, RewriteLevelHeader, RewriteLevelItem, RewriteLevelItemKind,
    RewriteModalBlock, RewriteModalMode, RewriteSagaChapterLine, RewriteSemanticItem,
    RewriteStatementLine, RewriteStaticLine, RewriteTriggeredLine, RewriteUnsupportedLine,
};
use super::keyword_registry::lower_keyword_line_cst;
use super::leaf::lower_activation_cost_cst;
use super::lower::{lower_rewrite_activated_to_chunk, lower_rewrite_static_to_chunk};
use super::rewrite_exceptions::{
    lower_rewrite_statement_token_groups_to_chunks, lower_rewrite_triggered_to_chunk,
};

pub(crate) fn lower_non_metadata_rewrite_line_cst(
    line: RewriteLineCst,
    allow_unsupported: bool,
) -> Result<RewriteSemanticItem, CardTextError> {
    match line {
        RewriteLineCst::Metadata(_) => Err(CardTextError::InvariantViolation(
            "metadata lowering must stay in document_parser".to_string(),
        )),
        RewriteLineCst::Keyword(keyword) => Ok(RewriteSemanticItem::Keyword(
            lower_keyword_line_cst(keyword)?,
        )),
        RewriteLineCst::Activated(activated) => lower_activated_line(activated, allow_unsupported),
        RewriteLineCst::Triggered(triggered) => lower_triggered_line(triggered),
        RewriteLineCst::Static(static_line) => lower_static_line(static_line),
        RewriteLineCst::Statement(statement_line) => lower_statement_line(statement_line),
        RewriteLineCst::Modal(modal) => lower_modal_block(modal),
        RewriteLineCst::LevelHeader(level) => lower_level_header(level),
        RewriteLineCst::SagaChapter(saga) => lower_saga_chapter(saga),
        RewriteLineCst::Unsupported(unsupported) => {
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
                return Ok(RewriteSemanticItem::Unsupported(RewriteUnsupportedLine {
                    info: activated.info,
                    reason_code: "activated-cost-not-yet-supported",
                }));
            }
            return Err(err);
        }
    };
    let lowered = lower_rewrite_activated_to_chunk(
        activated.info.clone(),
        cost.clone(),
        activated.cost_parse_tokens.clone(),
        activated.effect_text.clone(),
        activated.effect_parse_tokens.clone(),
        ActivationTiming::AnyTime,
        activated.chosen_option_label.clone(),
    )?;
    Ok(RewriteSemanticItem::Activated(RewriteActivatedLine {
        info: activated.info,
        cost,
        effect_text: activated.effect_text,
        timing_hint: ActivationTiming::AnyTime,
        chosen_option_label: activated.chosen_option_label,
        parsed: lowered.chunk,
        restrictions: lowered.restrictions,
    }))
}

fn lower_triggered_line(triggered: TriggeredLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    let parsed = lower_rewrite_triggered_to_chunk(
        triggered.info.clone(),
        &triggered.full_text,
        &triggered.full_parse_tokens,
        &triggered.trigger_text,
        &triggered.trigger_parse_tokens,
        &triggered.effect_text,
        &triggered.effect_parse_tokens,
        triggered.intervening_if.clone(),
        triggered.max_triggers_per_turn,
        triggered.chosen_option_label.as_deref(),
    )?;
    Ok(RewriteSemanticItem::Triggered(RewriteTriggeredLine {
        info: triggered.info,
        full_text: triggered.full_text,
        trigger_text: triggered.trigger_text,
        effect_text: triggered.effect_text,
        intervening_if: triggered.intervening_if,
        max_triggers_per_turn: triggered.max_triggers_per_turn,
        chosen_option_label: triggered.chosen_option_label,
        parsed,
    }))
}

fn lower_static_line(static_line: StaticLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    let parsed = if static_line.text == "activate only once each turn." {
        crate::cards::builders::LineAst::Statement {
            effects: Vec::new(),
        }
    } else {
        lower_rewrite_static_to_chunk(
            static_line.info.clone(),
            &static_line.text,
            &static_line.parse_tokens,
            static_line.chosen_option_label.as_deref(),
        )?
    };
    Ok(RewriteSemanticItem::Static(RewriteStaticLine {
        info: static_line.info,
        text: static_line.text,
        chosen_option_label: static_line.chosen_option_label,
        parsed,
    }))
}

fn lower_statement_line(
    statement_line: StatementLineCst,
) -> Result<RewriteSemanticItem, CardTextError> {
    let parsed_chunks = lower_rewrite_statement_token_groups_to_chunks(
        statement_line.info.clone(),
        &statement_line.text,
        &statement_line.parse_tokens,
        &statement_line.parse_groups,
    )?;
    Ok(RewriteSemanticItem::Statement(RewriteStatementLine {
        info: statement_line.info,
        text: statement_line.text,
        parsed_chunks,
    }))
}

fn lower_modal_block(
    modal: super::cst::ModalBlockCst,
) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::Modal(RewriteModalBlock {
        header: modal.header,
        modes: modal
            .modes
            .into_iter()
            .map(|mode| RewriteModalMode {
                info: mode.info,
                text: mode.text,
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

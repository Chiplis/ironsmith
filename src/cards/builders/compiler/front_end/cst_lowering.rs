use crate::ability::ActivationTiming;
use crate::cards::builders::CardTextError;

use super::cst::{
    ActivatedLineCst, KeywordLineCst, LevelItemKindCst, RewriteLineCst, SagaChapterLineCst,
    StatementLineCst, StaticLineCst, TriggeredLineCst,
};
use super::ir::{
    RewriteActivatedLine, RewriteKeywordLine, RewriteLevelHeader, RewriteLevelItem,
    RewriteLevelItemKind, RewriteModalBlock, RewriteModalMode, RewriteSagaChapterLine,
    RewriteSemanticItem, RewriteStatementLine, RewriteStaticLine, RewriteTriggeredLine,
    RewriteUnsupportedLine,
};
use super::leaf::lower_activation_cost_cst;

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
        })),
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
    Ok(RewriteSemanticItem::Activated(RewriteActivatedLine {
        info: activated.info,
        cost,
        cost_parse_tokens: activated.cost_parse_tokens,
        effect_text: activated.effect_text,
        effect_parse_tokens: activated.effect_parse_tokens,
        timing_hint: ActivationTiming::AnyTime,
        chosen_option_label: activated.chosen_option_label,
    }))
}

fn lower_triggered_line(triggered: TriggeredLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::Triggered(RewriteTriggeredLine {
        info: triggered.info,
        full_text: triggered.full_text,
        full_parse_tokens: triggered.full_parse_tokens,
        trigger_text: triggered.trigger_text,
        trigger_parse_tokens: triggered.trigger_parse_tokens,
        effect_text: triggered.effect_text,
        effect_parse_tokens: triggered.effect_parse_tokens,
        intervening_if: triggered.intervening_if,
        max_triggers_per_turn: triggered.max_triggers_per_turn,
        chosen_option_label: triggered.chosen_option_label,
    }))
}

fn lower_static_line(static_line: StaticLineCst) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::Static(RewriteStaticLine {
        info: static_line.info,
        text: static_line.text,
        parse_tokens: static_line.parse_tokens,
        chosen_option_label: static_line.chosen_option_label,
    }))
}

fn lower_statement_line(
    statement_line: StatementLineCst,
) -> Result<RewriteSemanticItem, CardTextError> {
    Ok(RewriteSemanticItem::Statement(RewriteStatementLine {
        info: statement_line.info,
        text: statement_line.text,
        parse_tokens: statement_line.parse_tokens,
        parse_groups: statement_line.parse_groups,
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

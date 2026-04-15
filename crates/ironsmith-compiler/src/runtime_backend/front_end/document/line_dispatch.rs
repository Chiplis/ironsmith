use super::line_family_handlers::{
    run_activation_line_family, run_colon_nonactivation_statement_line_family,
    run_combined_static_line_family, run_keyword_line_family, run_labeled_line_family,
    run_statement_line_family, run_statement_probe_line_family, run_static_line_family,
    run_trailing_keyword_activation_line_family, run_triggered_line_family,
    run_unsupported_line_family, run_ward_or_echo_static_prefix_line_family,
};
use super::*;

pub(super) struct LineDispatchResult {
    pub(super) lines: Vec<RewriteLineCst>,
    pub(super) next_idx: usize,
}

impl LineDispatchResult {
    pub(super) fn single(line: RewriteLineCst, next_idx: usize) -> Self {
        Self {
            lines: vec![line],
            next_idx,
        }
    }
}

pub(super) struct LineDispatchContext<'a> {
    pub(super) preprocessed: &'a PreprocessedDocument,
    pub(super) idx: usize,
    pub(super) line: &'a PreprocessedLine,
    pub(super) allow_unsupported: bool,
}

type LineFamilyRuleFn =
    for<'a> fn(&LineDispatchContext<'a>) -> Result<Option<LineDispatchResult>, CardTextError>;

#[derive(Clone, Copy)]
struct LineFamilyRuleDef {
    id: &'static str,
    priority: u16,
    heads: &'static [&'static str],
    run: LineFamilyRuleFn,
}

const LINE_FAMILY_RULES: [LineFamilyRuleDef; 11] = [
    LineFamilyRuleDef {
        id: "trailing-keyword-activation",
        priority: 10,
        heads: &[],
        run: run_trailing_keyword_activation_line_family,
    },
    LineFamilyRuleDef {
        id: "labeled-line",
        priority: 20,
        heads: &[],
        run: run_labeled_line_family,
    },
    LineFamilyRuleDef {
        id: "triggered-line",
        priority: 30,
        heads: &["when", "whenever", "at"],
        run: run_triggered_line_family,
    },
    LineFamilyRuleDef {
        id: "keyword-line",
        priority: 40,
        heads: &[],
        run: run_keyword_line_family,
    },
    LineFamilyRuleDef {
        id: "ward-or-echo-static-prefix",
        priority: 50,
        heads: &["ward", "echo"],
        run: run_ward_or_echo_static_prefix_line_family,
    },
    LineFamilyRuleDef {
        id: "activated-line",
        priority: 60,
        heads: &[],
        run: run_activation_line_family,
    },
    LineFamilyRuleDef {
        id: "combined-static-pair",
        priority: 70,
        heads: &["as", "if"],
        run: run_combined_static_line_family,
    },
    LineFamilyRuleDef {
        id: "statement-probe",
        priority: 80,
        heads: &[],
        run: run_statement_probe_line_family,
    },
    LineFamilyRuleDef {
        id: "static-line",
        priority: 90,
        heads: &[],
        run: run_static_line_family,
    },
    LineFamilyRuleDef {
        id: "statement-line",
        priority: 100,
        heads: &[],
        run: run_statement_line_family,
    },
    LineFamilyRuleDef {
        id: "colon-nonactivation-statement",
        priority: 110,
        heads: &[],
        run: run_colon_nonactivation_statement_line_family,
    },
];

static LINE_FAMILY_RULE_INDEX: LazyLock<LexRuleHintIndex> = LazyLock::new(|| {
    build_lex_rule_hint_index(LINE_FAMILY_RULES.len(), |idx| {
        LINE_FAMILY_RULES[idx]
            .heads
            .iter()
            .copied()
            .map(LexRuleHeadHint::Single)
            .collect()
    })
});

fn dispatch_line_family_registry(
    ctx: &LineDispatchContext<'_>,
) -> Result<LineDispatchResult, CardTextError> {
    let (head, second) = lexed_head_words(&ctx.line.tokens).unwrap_or(("", None));
    let mut candidate_indices = LINE_FAMILY_RULE_INDEX.candidate_indices(head, second);
    let mut hinted = vec![false; LINE_FAMILY_RULES.len()];
    for idx in &candidate_indices {
        hinted[*idx] = true;
    }
    candidate_indices.extend(
        LINE_FAMILY_RULES
            .iter()
            .enumerate()
            .filter(|(idx, _)| !hinted[*idx])
            .map(|(idx, _)| idx),
    );
    candidate_indices.sort_by_key(|idx| LINE_FAMILY_RULES[*idx].priority);

    for idx in candidate_indices {
        if let Some(dispatch) = (LINE_FAMILY_RULES[idx].run)(ctx)? {
            return Ok(dispatch);
        }
    }

    run_unsupported_line_family(ctx)?.ok_or_else(|| {
        CardTextError::InvariantViolation(format!(
            "line-family registry exhausted without handling line: '{}' [last_rule={}]",
            ctx.line.info.raw_line,
            LINE_FAMILY_RULES
                .last()
                .map(|rule| rule.id)
                .unwrap_or("none")
        ))
    })
}

pub(super) fn dispatch_standard_line_cst(
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<LineDispatchResult, CardTextError> {
    let ctx = LineDispatchContext {
        preprocessed,
        idx,
        line,
        allow_unsupported,
    };
    dispatch_line_family_registry(&ctx)
}

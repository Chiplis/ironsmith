use super::line_family_handlers::{
    run_activation_line_family, run_champion_line_family,
    run_championed_with_this_trigger_line_family, run_colon_nonactivation_statement_line_family,
    run_combined_static_line_family, run_escape_enters_with_counter_line_family,
    run_graveyard_cast_control_condition_line_family, run_keyword_line_family,
    run_labeled_line_family, run_learn_line_family,
    run_max_speed_labeled_line_family, run_non_turn_conditional_untap_line_family,
    run_partner_with_keyword_line_family, run_split_top_and_face_down_look_line_family,
    run_split_top_look_and_top_land_play_line_family, run_start_your_engines_line_family,
    run_statement_line_family, run_statement_probe_line_family, run_static_line_family,
    run_station_line_family, run_station_threshold_line_family, run_surge_line_family,
    run_trailing_keyword_activation_line_family, run_triggered_line_family,
    run_unsupported_line_family, run_ward_or_echo_static_prefix_line_family,
};
use super::*;
use crate::parse_trace;

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

const LINE_FAMILY_RULES: [LineFamilyRuleDef; 25] = [
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
        id: "max-speed-labeled-line",
        priority: 18,
        heads: &[],
        run: run_max_speed_labeled_line_family,
    },
    LineFamilyRuleDef {
        id: "triggered-line",
        priority: 30,
        heads: &["when", "whenever", "at"],
        run: run_triggered_line_family,
    },
    LineFamilyRuleDef {
        id: "championed-with-this-trigger-line",
        priority: 29,
        heads: &["when"],
        run: run_championed_with_this_trigger_line_family,
    },
    LineFamilyRuleDef {
        id: "partner-with-keyword-line",
        priority: 35,
        heads: &["partner"],
        run: run_partner_with_keyword_line_family,
    },
    LineFamilyRuleDef {
        id: "start-your-engines-line",
        priority: 36,
        heads: &["start"],
        run: run_start_your_engines_line_family,
    },
    LineFamilyRuleDef {
        id: "learn-line",
        priority: 37,
        heads: &["learn"],
        run: run_learn_line_family,
    },
    LineFamilyRuleDef {
        id: "split-top-and-face-down-look-line",
        priority: 38,
        heads: &["you"],
        run: run_split_top_and_face_down_look_line_family,
    },
    LineFamilyRuleDef {
        id: "split-top-look-and-top-land-play-line",
        priority: 39,
        heads: &["you"],
        run: run_split_top_look_and_top_land_play_line_family,
    },
    LineFamilyRuleDef {
        id: "champion-line",
        priority: 40,
        heads: &["champion"],
        run: run_champion_line_family,
    },
    LineFamilyRuleDef {
        id: "station-line",
        priority: 40,
        heads: &["station"],
        run: run_station_line_family,
    },
    LineFamilyRuleDef {
        id: "station-threshold-line",
        priority: 41,
        heads: &[],
        run: run_station_threshold_line_family,
    },
    LineFamilyRuleDef {
        id: "escape-enters-with-counter-line",
        priority: 42,
        heads: &[],
        run: run_escape_enters_with_counter_line_family,
    },
    LineFamilyRuleDef {
        id: "surge-line",
        priority: 43,
        heads: &["surge"],
        run: run_surge_line_family,
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
        id: "non-turn-conditional-untap",
        priority: 75,
        heads: &["creatures"],
        run: run_non_turn_conditional_untap_line_family,
    },
    LineFamilyRuleDef {
        id: "graveyard-cast-control-condition",
        priority: 76,
        heads: &["you"],
        run: run_graveyard_cast_control_condition_line_family,
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

fn dispatch_kind_summary(dispatch: &LineDispatchResult) -> String {
    dispatch
        .lines
        .iter()
        .map(rewrite_line_cst_kind)
        .collect::<Vec<_>>()
        .join(" + ")
}

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
        let rule = &LINE_FAMILY_RULES[idx];
        match (rule.run)(ctx) {
            Ok(Some(dispatch)) => {
                parse_trace::event(format!(
                    "line-family: {} -> {}",
                    rule.id,
                    dispatch_kind_summary(&dispatch)
                ));
                return Ok(dispatch);
            }
            Ok(None) => {}
            Err(err) => {
                parse_trace::event(format!("line-family: {} errored: {err:?}", rule.id));
                return Err(err);
            }
        }
    }

    match run_unsupported_line_family(ctx) {
        Ok(Some(dispatch)) => {
            parse_trace::event(format!(
                "line-family: unsupported -> {}",
                dispatch_kind_summary(&dispatch)
            ));
            Ok(dispatch)
        }
        Ok(None) => Err(CardTextError::InvariantViolation(format!(
            "line-family registry exhausted without handling line: '{}' [last_rule={}]",
            ctx.line.info.raw_line,
            LINE_FAMILY_RULES
                .last()
                .map(|rule| rule.id)
                .unwrap_or("none")
        ))),
        Err(err) => {
            parse_trace::event(format!("line-family: unsupported errored: {err:?}"));
            Err(err)
        }
    }
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

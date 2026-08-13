use super::line_family_handlers::{
    run_activation_line_family, run_additional_combat_after_this_phase_line_family,
    run_assign_damage_as_unblocked_enchanted_creature_controller_line_family,
    run_champion_line_family, run_championed_with_this_trigger_line_family,
    run_colon_nonactivation_statement_line_family, run_combined_static_line_family,
    run_draft_rule_line_family, run_escape_enters_with_counter_line_family,
    run_freerunning_line_family, run_graveyard_cast_control_condition_line_family,
    run_graveyard_or_exile_cast_line_family, run_keyword_line_family, run_labeled_line_family,
    run_leading_unless_statement_line_family, run_learn_line_family,
    run_max_speed_labeled_line_family, run_non_turn_conditional_untap_line_family,
    run_partner_variant_keyword_line_family, run_partner_with_keyword_line_family,
    run_split_top_and_face_down_look_line_family, run_split_top_look_and_top_land_play_line_family,
    run_start_your_engines_line_family, run_statement_line_family, run_statement_probe_line_family,
    run_static_line_family, run_station_line_family, run_station_threshold_line_family,
    run_surge_line_family, run_trailing_keyword_activation_line_family, run_triggered_line_family,
    run_unsupported_line_family, run_ward_or_echo_static_prefix_line_family,
};
use super::*;
use crate::parse_trace;
use crate::recognition::{ParseDiagnostic, ParseOutcome, RuleId};
use crate::registry::{
    HeadDiscriminator, RegistryCandidate, RegistryRuleMetadata, resolve_registry_candidates,
};

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
    pub(super) parse: ParseContextView<'a>,
    pub(super) preprocessed: &'a PreprocessedDocument,
    pub(super) idx: usize,
    pub(super) line: &'a PreprocessedLine,
    pub(super) allow_unsupported: bool,
}

type StructuredLineFamilyRuleFn =
    for<'a> fn(&LineDispatchContext<'a>) -> ParseOutcome<LineDispatchResult>;
type LegacyLineFamilyRuleFn =
    for<'a> fn(&LineDispatchContext<'a>) -> Result<Option<LineDispatchResult>, CardTextError>;

#[derive(Clone, Copy)]
enum LineFamilyRuleHandler {
    Structured(StructuredLineFamilyRuleFn),
    Legacy(LegacyLineFamilyRuleFn),
}

impl LineFamilyRuleHandler {
    fn recognize(
        self,
        id: RuleId,
        ctx: &LineDispatchContext<'_>,
    ) -> ParseOutcome<LineDispatchResult> {
        match self {
            Self::Structured(run) => run(ctx).within(id),
            Self::Legacy(run) => ParseOutcome::from_legacy_result_option(
                id,
                span_from_tokens(&ctx.line.tokens),
                run(ctx),
            ),
        }
    }
}

#[derive(Clone, Copy)]
struct LineFamilyRuleDef {
    id: RuleId,
    head: HeadDiscriminator,
    run: LineFamilyRuleHandler,
}

const LINE_FAMILY_RULES: [LineFamilyRuleDef; 32] = [
    LineFamilyRuleDef {
        id: RuleId::new("trailing-keyword-activation"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_trailing_keyword_activation_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("labeled-line"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_labeled_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("max-speed-labeled-line"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_max_speed_labeled_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("triggered-line"),
        head: HeadDiscriminator::words(&["when", "whenever", "at"]),
        run: LineFamilyRuleHandler::Legacy(run_triggered_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("championed-with-this-trigger-line"),
        head: HeadDiscriminator::words(&["when"]),
        run: LineFamilyRuleHandler::Legacy(run_championed_with_this_trigger_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("partner-with-keyword-line"),
        head: HeadDiscriminator::words(&["partner"]),
        run: LineFamilyRuleHandler::Legacy(run_partner_with_keyword_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("partner-variant-keyword-line"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_partner_variant_keyword_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("start-your-engines-line"),
        head: HeadDiscriminator::words(&["start"]),
        run: LineFamilyRuleHandler::Legacy(run_start_your_engines_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("learn-line"),
        head: HeadDiscriminator::words(&["learn"]),
        run: LineFamilyRuleHandler::Legacy(run_learn_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("draft-rule-line"),
        head: HeadDiscriminator::words(&["draft", "reveal", "as", "during", "immediately", "each"]),
        run: LineFamilyRuleHandler::Legacy(run_draft_rule_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("split-top-and-face-down-look-line"),
        head: HeadDiscriminator::words(&["you"]),
        run: LineFamilyRuleHandler::Legacy(run_split_top_and_face_down_look_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("split-top-look-and-top-land-play-line"),
        head: HeadDiscriminator::words(&["you"]),
        run: LineFamilyRuleHandler::Legacy(run_split_top_look_and_top_land_play_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("assign-damage-as-unblocked-enchanted-creature-controller"),
        head: HeadDiscriminator::words(&["enchanted"]),
        run: LineFamilyRuleHandler::Legacy(run_assign_damage_as_unblocked_enchanted_creature_controller_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("champion-line"),
        head: HeadDiscriminator::words(&["champion"]),
        run: LineFamilyRuleHandler::Legacy(run_champion_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("station-line"),
        head: HeadDiscriminator::words(&["station"]),
        run: LineFamilyRuleHandler::Legacy(run_station_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("station-threshold-line"),
        // Threshold rows contain a colon in their activation body. They must
        // be recognized before the generic activation probe gets a chance to
        // treat the threshold header as part of the payment cost.
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_station_threshold_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("escape-enters-with-counter-line"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_escape_enters_with_counter_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("surge-line"),
        head: HeadDiscriminator::words(&["surge"]),
        run: LineFamilyRuleHandler::Legacy(run_surge_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("freerunning-line"),
        head: HeadDiscriminator::words(&["freerunning"]),
        run: LineFamilyRuleHandler::Legacy(run_freerunning_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("keyword-line"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_keyword_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("ward-or-echo-static-prefix"),
        head: HeadDiscriminator::words(&["ward", "echo"]),
        run: LineFamilyRuleHandler::Legacy(run_ward_or_echo_static_prefix_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("activated-line"),
        // A valid colon-separated activation must be classified before the
        // broad keyword probe, which can otherwise find a keyword in the
        // effect half and claim the complete line.
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_activation_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("combined-static-pair"),
        head: HeadDiscriminator::words(&["as", "if"]),
        run: LineFamilyRuleHandler::Legacy(run_combined_static_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("non-turn-conditional-untap"),
        head: HeadDiscriminator::words(&["creatures"]),
        run: LineFamilyRuleHandler::Legacy(run_non_turn_conditional_untap_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("graveyard-cast-control-condition"),
        head: HeadDiscriminator::words(&["you"]),
        run: LineFamilyRuleHandler::Legacy(run_graveyard_cast_control_condition_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("graveyard-or-exile-cast"),
        head: HeadDiscriminator::words(&["you"]),
        run: LineFamilyRuleHandler::Legacy(run_graveyard_or_exile_cast_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("additional-combat-after-this-phase"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_additional_combat_after_this_phase_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("statement-probe"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_statement_probe_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("leading-unless-statement"),
        head: HeadDiscriminator::words(&["unless"]),
        run: LineFamilyRuleHandler::Legacy(run_leading_unless_statement_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("static-line"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_static_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("statement-line"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_statement_line_family),
    },
    LineFamilyRuleDef {
        id: RuleId::new("colon-nonactivation-statement"),
        head: HeadDiscriminator::words(&[]),
        run: LineFamilyRuleHandler::Legacy(run_colon_nonactivation_statement_line_family),
    },
];

fn dispatch_kind_summary(dispatch: &LineDispatchResult) -> String {
    dispatch
        .lines
        .iter()
        .map(rewrite_line_cst_kind)
        .collect::<Vec<_>>()
        .join(" + ")
}

fn triggered_program_from_line_ast(
    line: LineAst,
) -> Option<(
    crate::model::ast::TriggerSpec,
    Vec<crate::model::ast::EffectAst>,
)> {
    match line {
        LineAst::Triggered {
            trigger, effects, ..
        } => Some((trigger, effects)),
        LineAst::Multiple(lines) => lines
            .into_iter()
            .find_map(triggered_program_from_line_ast),
        _ => None,
    }
}

fn attach_compiler_trigger_facts(
    context: ParseContextView<'_>,
    dispatch: &mut LineDispatchResult,
) -> Result<(), CardTextError> {
    for line in &mut dispatch.lines {
        let RewriteLineCst::Triggered(triggered) = line else {
            continue;
        };

        let direct = if triggered.trigger_parse_tokens.is_empty()
            || triggered.effect_parse_tokens.is_empty()
        {
            None
        } else {
            parse_trigger_clause_lexed(&triggered.trigger_parse_tokens)
                .and_then(|trigger| {
                    parse_effect_sentences_lexed(&triggered.effect_parse_tokens)
                        .map(|effects| (trigger, effects))
                })
                .ok()
        };
        let (trigger, effects) = match direct {
            Some(program) => program,
            None => triggered_program_from_line_ast(parse_triggered_line_lexed(
                &triggered.full_parse_tokens,
            )?)
            .ok_or_else(|| {
                CardTextError::InvariantViolation(
                    "trigger line produced no compiler trigger program".to_string(),
                )
            })?,
        };
        let functional_zones =
            super::super::semantic_line_parsing::infer_triggered_ability_functional_zones_from_facts(
                &trigger,
                &triggered.info.semantic_facts.triggered_ability.functional_zones,
            );
        let compiler_ability =
            super::super::grammar::trigger_event_facts::build_compiler_triggered_ability(
                context,
                &triggered.full_parse_tokens,
                if triggered.effect_parse_tokens.is_empty() {
                    &triggered.full_parse_tokens
                } else {
                    &triggered.effect_parse_tokens
                },
                trigger,
                effects,
                triggered.intervening_if.clone(),
                triggered.max_triggers_per_turn,
                functional_zones,
            )?;
        triggered
            .info
            .semantic_facts
            .triggered_ability
            .compiler_ability = Some(compiler_ability);
    }
    Ok(())
}

fn dispatch_line_family_registry(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    // Borrow preprocessing expands a removed-from-draft `The same is true`
    // ladder into independent leading-condition sentences. Preserve that
    // complete typed program before keyword discovery can claim consequence
    // words such as flying or haste as one unconditional keyword line.
    match ParseOutcome::from_legacy_result_option(
        RuleId::new("removed-draft-leading-conditional-static-chain"),
        span_from_tokens(&ctx.line.tokens),
        crate::families::keyword_static::parse_removed_draft_leading_conditional_static_sentence_chain(
            &ctx.line.tokens,
        ),
    ) {
        ParseOutcome::Match(matched) => {
            return ParseOutcome::matched(
                LineDispatchResult::single(
                    RewriteLineCst::Static(StaticLineCst {
                        info: ctx.line.info.clone(),
                        parse_tokens: ctx.line.tokens.clone(),
                        chosen_option: None,
                        parsed: Some(LineAst::StaticAbilities(matched.value)),
                    }),
                    ctx.idx + 1,
                ),
                matched.span,
            );
        }
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return ParseOutcome::Error(diagnostic),
    }

    let (head, _) = lexed_head_words(&ctx.line.tokens).unwrap_or(("", None));
    parse_trace::event(format!(
        "line-family scope: {:?} ({:?})",
        ctx.parse.scope(),
        ctx.parse.scope_kind()
    ));
    let candidate_indices = LINE_FAMILY_RULES
        .iter()
        .enumerate()
        .filter(|(_, rule)| rule.head.accepts(head))
        .map(|(idx, _)| idx)
        .collect::<Vec<_>>();
    let mut candidates = Vec::new();
    let mut diagnostics = Vec::new();

    for idx in candidate_indices {
        let rule = &LINE_FAMILY_RULES[idx];
        match rule.run.recognize(rule.id, ctx) {
            ParseOutcome::Match(matched) => {
                candidates.push(RegistryCandidate::new(
                    RegistryRuleMetadata::distinct(rule.id, rule.head),
                    matched.value,
                    matched.span,
                ));
            }
            ParseOutcome::NoMatch => {}
            ParseOutcome::Error(diagnostic) => {
                parse_trace::event(format!(
                    "line-family: {} errored: {diagnostic:?}",
                    rule.id
                ));
                diagnostics.push(diagnostic);
            }
        }
    }

    match resolve_registry_candidates(
        RuleId::new("line-family-registry"),
        candidates,
        diagnostics,
    ) {
        ParseOutcome::Match(matched) => {
            let rule_match = matched.value;
            let mut dispatch = rule_match.value;
            if let Err(error) = attach_compiler_trigger_facts(ctx.parse, &mut dispatch) {
                return ParseOutcome::Error(ParseDiagnostic::from_legacy_error(
                    RuleId::new("compiler-trigger-facts"),
                    span_from_tokens(&ctx.line.tokens),
                    error,
                ));
            }
            parse_trace::event(format!(
                "line-family: {} -> {}",
                rule_match.rule,
                dispatch_kind_summary(&dispatch)
            ));
            return ParseOutcome::matched(dispatch, rule_match.span);
        }
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return ParseOutcome::Error(diagnostic),
    }

    match ParseOutcome::from_legacy_result_option(
        RuleId::new("unsupported-line-family"),
        span_from_tokens(&ctx.line.tokens),
        run_unsupported_line_family(ctx),
    ) {
        ParseOutcome::Match(matched) => {
            let dispatch = matched.value;
            parse_trace::event(format!(
                "line-family: unsupported -> {}",
                dispatch_kind_summary(&dispatch)
            ));
            ParseOutcome::matched(dispatch, matched.span)
        }
        ParseOutcome::NoMatch => ParseOutcome::Error(ParseDiagnostic::invariant(
            RuleId::new("line-family-registry"),
            span_from_tokens(&ctx.line.tokens),
            format!(
                "line-family registry exhausted without handling line: '{}' [last_rule={}]",
                ctx.line.info.raw_line,
                LINE_FAMILY_RULES
                    .last()
                    .map(|rule| rule.id.as_str())
                    .unwrap_or("none")
            ),
        )),
        ParseOutcome::Error(diagnostic) => {
            parse_trace::event(format!(
                "line-family: unsupported errored: {diagnostic:?}"
            ));
            ParseOutcome::Error(diagnostic)
        }
    }
}

pub(super) fn dispatch_standard_line_cst(
    parse: ParseContextView<'_>,
    preprocessed: &PreprocessedDocument,
    idx: usize,
    line: &PreprocessedLine,
    allow_unsupported: bool,
) -> Result<LineDispatchResult, CardTextError> {
    let ctx = LineDispatchContext {
        parse,
        preprocessed,
        idx,
        line,
        allow_unsupported,
    };
    dispatch_line_family_registry(&ctx).into_legacy_result(|| {
        CardTextError::InvariantViolation(format!(
            "line-family registry returned no match for '{}'",
            line.info.raw_line
        ))
    })
}

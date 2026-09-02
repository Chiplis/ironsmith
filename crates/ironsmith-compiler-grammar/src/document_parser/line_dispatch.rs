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
    pub(super) lines: Vec<RecognizedLine>,
    pub(super) next_idx: usize,
}

impl LineDispatchResult {
    pub(super) fn single(line: RecognizedLine, next_idx: usize) -> Self {
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

#[derive(Clone, Copy)]
struct LineFamilyRuleDef {
    id: RuleId,
    head: HeadDiscriminator,
    run: StructuredLineFamilyRuleFn,
}

const LINE_FAMILY_RULES: [LineFamilyRuleDef; 32] = [
    LineFamilyRuleDef {
        id: RuleId::new("trailing-keyword-activation"),
        head: HeadDiscriminator::words(&[]),
        run: run_trailing_keyword_activation_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("max-speed-labeled-line"),
        head: HeadDiscriminator::words(&[]),
        run: run_max_speed_labeled_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("labeled-line"),
        head: HeadDiscriminator::words(&[]),
        run: run_labeled_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("triggered-line"),
        head: HeadDiscriminator::words(&["when", "whenever", "at"]),
        run: run_triggered_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("championed-with-this-trigger-line"),
        head: HeadDiscriminator::words(&["when"]),
        run: run_championed_with_this_trigger_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("partner-with-keyword-line"),
        head: HeadDiscriminator::words(&["partner"]),
        run: run_partner_with_keyword_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("partner-variant-keyword-line"),
        head: HeadDiscriminator::words(&[]),
        run: run_partner_variant_keyword_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("start-your-engines-line"),
        head: HeadDiscriminator::words(&["start"]),
        run: run_start_your_engines_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("learn-line"),
        head: HeadDiscriminator::words(&["learn"]),
        run: run_learn_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("draft-rule-line"),
        head: HeadDiscriminator::words(&["draft", "reveal", "as", "during", "immediately", "each"]),
        run: run_draft_rule_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("split-top-and-face-down-look-line"),
        head: HeadDiscriminator::words(&["you"]),
        run: run_split_top_and_face_down_look_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("split-top-look-and-top-land-play-line"),
        head: HeadDiscriminator::words(&["you"]),
        run: run_split_top_look_and_top_land_play_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("assign-damage-as-unblocked-enchanted-creature-controller"),
        head: HeadDiscriminator::words(&["enchanted"]),
        run: run_assign_damage_as_unblocked_enchanted_creature_controller_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("champion-line"),
        head: HeadDiscriminator::words(&["champion"]),
        run: run_champion_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("station-line"),
        head: HeadDiscriminator::words(&["station"]),
        run: run_station_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("station-threshold-line"),
        // Threshold rows contain a colon in their activation body. They must
        // be recognized before the generic activation probe gets a chance to
        // treat the threshold header as part of the payment cost.
        head: HeadDiscriminator::words(&[]),
        run: run_station_threshold_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("escape-enters-with-counter-line"),
        head: HeadDiscriminator::words(&[]),
        run: run_escape_enters_with_counter_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("surge-line"),
        head: HeadDiscriminator::words(&["surge"]),
        run: run_surge_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("freerunning-line"),
        head: HeadDiscriminator::words(&["freerunning"]),
        run: run_freerunning_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("keyword-line"),
        head: HeadDiscriminator::words(&[]),
        run: run_keyword_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("ward-or-echo-static-prefix"),
        head: HeadDiscriminator::words(&["ward", "echo"]),
        run: run_ward_or_echo_static_prefix_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("activated-line"),
        // A valid colon-separated activation must be classified before the
        // broad keyword probe, which can otherwise find a keyword in the
        // effect half and claim the complete line.
        head: HeadDiscriminator::words(&[]),
        run: run_activation_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("combined-static-pair"),
        head: HeadDiscriminator::words(&["as", "if"]),
        run: run_combined_static_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("non-turn-conditional-untap"),
        head: HeadDiscriminator::words(&["creatures"]),
        run: run_non_turn_conditional_untap_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("graveyard-cast-control-condition"),
        head: HeadDiscriminator::words(&["you"]),
        run: run_graveyard_cast_control_condition_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("graveyard-or-exile-cast"),
        head: HeadDiscriminator::words(&["you"]),
        run: run_graveyard_or_exile_cast_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("additional-combat-after-this-phase"),
        head: HeadDiscriminator::words(&[]),
        run: run_additional_combat_after_this_phase_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("statement-probe"),
        head: HeadDiscriminator::words(&[]),
        run: run_statement_probe_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("leading-unless-statement"),
        head: HeadDiscriminator::words(&["unless"]),
        run: run_leading_unless_statement_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("static-line"),
        head: HeadDiscriminator::words(&[]),
        run: run_static_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("statement-line"),
        head: HeadDiscriminator::words(&[]),
        run: run_statement_line_family,
    },
    LineFamilyRuleDef {
        id: RuleId::new("colon-nonactivation-statement"),
        head: HeadDiscriminator::words(&[]),
        run: run_colon_nonactivation_statement_line_family,
    },
];

fn dispatch_kind_summary(dispatch: &LineDispatchResult) -> String {
    dispatch
        .lines
        .iter()
        .map(recognized_line_kind)
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
        LineAst::Multiple(lines) => lines.into_iter().find_map(triggered_program_from_line_ast),
        _ => None,
    }
}

fn authored_named_source_surface_after(
    context: ParseContextView<'_>,
    source: &[OwnedLexToken],
    action_word: &str,
) -> Option<crate::target::SourceReferenceSurface> {
    crate::grammar::source_surface_shapes::parse_unique_named_operand_after(
        Some(context),
        source,
        action_word,
    )
    .map(|shape| shape.surface)
}

fn authored_source_surface_after(
    context: ParseContextView<'_>,
    source: &[OwnedLexToken],
    action_word: &str,
) -> Option<crate::target::SourceReferenceSurface> {
    crate::grammar::source_surface_shapes::parse_unique_source_operand_after(
        context,
        source,
        action_word,
    )
    .map(|shape| shape.surface)
}

fn plain_source_target(target: &crate::cards::builders::TargetAst) -> bool {
    match target {
        crate::cards::builders::TargetAst::Source(_) => true,
        crate::cards::builders::TargetAst::Object(filter, _, _) if filter.source => {
            let mut plain = filter.clone();
            plain.source_surface = None;
            plain == crate::target::ObjectFilter::source()
        }
        _ => false,
    }
}

fn apply_named_source_surface(
    target: &mut crate::cards::builders::TargetAst,
    surface: &crate::target::SourceReferenceSurface,
) {
    match target {
        crate::cards::builders::TargetAst::Source(span) => {
            *target = crate::cards::builders::TargetAst::Object(
                crate::target::ObjectFilter::source_with_surface(surface.clone()),
                None,
                *span,
            );
        }
        crate::cards::builders::TargetAst::Object(filter, _, _) => {
            filter.source_surface = Some(surface.clone());
        }
        _ => unreachable!("plain_source_target accepted a non-source target"),
    }
}

fn preserve_named_source_exile_surface(
    context: ParseContextView<'_>,
    source: &[OwnedLexToken],
    effects: &mut [crate::model::ast::EffectAst],
) {
    fn candidate_count(effects: &[crate::model::ast::EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect {
                let target = match &subject_verb.action {
                    crate::cards::builders::SubjectVerbActionAst::Exile { target, .. }
                    | crate::cards::builders::SubjectVerbActionAst::MoveToZone {
                        target,
                        zone: crate::zone::Zone::Exile,
                        ..
                    } => Some(target),
                    _ => None,
                };
                count += target.is_some_and(plain_source_target) as usize;
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += candidate_count(nested)
            });
        }
        count
    }

    fn apply(
        effects: &mut [crate::model::ast::EffectAst],
        surface: &crate::target::SourceReferenceSurface,
    ) {
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect {
                let target = match &mut subject_verb.action {
                    crate::cards::builders::SubjectVerbActionAst::Exile { target, .. }
                    | crate::cards::builders::SubjectVerbActionAst::MoveToZone {
                        target,
                        zone: crate::zone::Zone::Exile,
                        ..
                    } => Some(target),
                    _ => None,
                };
                if let Some(target) = target
                    && plain_source_target(target)
                {
                    apply_named_source_surface(target, surface);
                }
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                apply(nested, surface)
            });
        }
    }

    let Some(surface) = authored_named_source_surface_after(context, source, "exile") else {
        return;
    };
    let count = candidate_count(effects);
    if count == 1 {
        apply(effects, &surface);
    }
}

fn preserve_named_source_transform_surface(
    context: ParseContextView<'_>,
    source: &[OwnedLexToken],
    effects: &mut [crate::model::ast::EffectAst],
) {
    fn candidate_count(effects: &[crate::model::ast::EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect
                && let crate::cards::builders::SubjectVerbActionAst::Transform { target } =
                    &subject_verb.action
            {
                count += plain_source_target(target) as usize;
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += candidate_count(nested)
            });
        }
        count
    }

    fn apply(
        effects: &mut [crate::model::ast::EffectAst],
        surface: &crate::target::SourceReferenceSurface,
    ) {
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect
                && let crate::cards::builders::SubjectVerbActionAst::Transform { target } =
                    &mut subject_verb.action
                && plain_source_target(target)
            {
                apply_named_source_surface(target, surface);
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                apply(nested, surface)
            });
        }
    }

    let Some(surface) = authored_source_surface_after(context, source, "transform") else {
        return;
    };
    if candidate_count(effects) == 1 {
        apply(effects, &surface);
    }
}

fn preserve_named_source_unattach_surface(
    context: ParseContextView<'_>,
    source: &[OwnedLexToken],
    effects: &mut [crate::model::ast::EffectAst],
) {
    fn candidate_count(effects: &[crate::model::ast::EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect
                && let crate::cards::builders::SubjectVerbActionAst::Unattach { object } =
                    &subject_verb.action
            {
                count += plain_source_target(object) as usize;
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += candidate_count(nested)
            });
        }
        count
    }

    fn apply(
        effects: &mut [crate::model::ast::EffectAst],
        surface: &crate::target::SourceReferenceSurface,
    ) {
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect
                && let crate::cards::builders::SubjectVerbActionAst::Unattach { object } =
                    &mut subject_verb.action
                && plain_source_target(object)
            {
                apply_named_source_surface(object, surface);
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                apply(nested, surface)
            });
        }
    }

    let Some(surface) = authored_named_source_surface_after(context, source, "unattach") else {
        return;
    };
    if candidate_count(effects) == 1 {
        apply(effects, &surface);
    }
}

fn preserve_named_source_put_counters_surface(
    context: ParseContextView<'_>,
    source: &[OwnedLexToken],
    effects: &mut [crate::model::ast::EffectAst],
) {
    fn candidate_count(effects: &[crate::model::ast::EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect
                && let crate::cards::builders::SubjectVerbActionAst::PutCounters { target, .. } =
                    &subject_verb.action
            {
                count += plain_source_target(target) as usize;
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += candidate_count(nested)
            });
        }
        count
    }

    fn apply(
        effects: &mut [crate::model::ast::EffectAst],
        surface: &crate::target::SourceReferenceSurface,
    ) {
        for effect in effects {
            if let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect
                && let crate::cards::builders::SubjectVerbActionAst::PutCounters { target, .. } =
                    &mut subject_verb.action
                && plain_source_target(target)
            {
                apply_named_source_surface(target, surface);
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                apply(nested, surface)
            });
        }
    }

    let Some(surface) = authored_named_source_surface_after(context, source, "on") else {
        return;
    };
    if candidate_count(effects) == 1 {
        apply(effects, &surface);
    }
}

fn preserve_named_source_chosen_complement_surface(
    context: ParseContextView<'_>,
    source: &[OwnedLexToken],
    effects: &mut [crate::model::ast::EffectAst],
) {
    if !crate::grammar::source_surface_shapes::parse_chosen_complement_surface(source) {
        return;
    }
    let Some(surface) = crate::util::authored_named_source_reference_surface(context, source)
    else {
        return;
    };

    fn matching_filter(
        effect: &crate::model::ast::EffectAst,
    ) -> Option<&crate::target::ObjectFilter> {
        let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect else {
            return None;
        };
        let crate::cards::builders::SubjectVerbActionAst::PumpAll { filter, .. } =
            &subject_verb.action
        else {
            return None;
        };
        let [chosen_exclusion] = filter.tagged_constraints.as_slice() else {
            return None;
        };
        (filter.card_types.len() == 1
            && filter.card_types.first() == Some(&crate::types::CardType::Creature)
            && filter.other
            && matches!(
                filter.source_surface,
                Some(crate::target::SourceReferenceSurface::ThisPermanentType(_))
            )
            && chosen_exclusion.relation == crate::target::TaggedOpbjectRelation::IsNotTaggedObject)
            .then_some(filter)
    }

    fn candidate_count(effects: &[crate::model::ast::EffectAst]) -> usize {
        let mut count = 0;
        for effect in effects {
            count += matching_filter(effect).is_some() as usize;
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                count += candidate_count(nested)
            });
        }
        count
    }

    fn apply(
        effects: &mut [crate::model::ast::EffectAst],
        surface: &crate::target::SourceReferenceSurface,
    ) {
        for effect in effects {
            if matching_filter(effect).is_some()
                && let crate::model::ast::EffectAst::SubjectVerb(subject_verb) = effect
                && let crate::cards::builders::SubjectVerbActionAst::PumpAll { filter, .. } =
                    &mut subject_verb.action
            {
                filter.source_surface = Some(surface.clone());
            }
            crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                apply(nested, surface)
            });
        }
    }

    if candidate_count(effects) == 1 {
        apply(effects, &surface);
    }
}

fn preserve_split_participant_order_surface(
    trigger_tokens: &[OwnedLexToken],
    effect_tokens: &[OwnedLexToken],
    full_tokens: &[OwnedLexToken],
    effects: &mut Vec<crate::model::ast::EffectAst>,
) {
    fn first_effect_is_each_player_loop(effects: &[crate::model::ast::EffectAst]) -> bool {
        let Some(first) = effects.first() else {
            return false;
        };
        match first {
            crate::model::ast::EffectAst::ForEachPlayer { .. } => true,
            crate::model::ast::EffectAst::Sequence { effects }
            | crate::model::ast::EffectAst::CommaThen { effects }
            | crate::model::ast::EffectAst::Coordinated { effects, .. }
            | crate::model::ast::EffectAst::SourceSentence { effects, .. } => {
                first_effect_is_each_player_loop(effects)
            }
            _ => false,
        }
    }

    let authored_order = crate::grammar::line_families::parse_starting_with_controller_boundary(
        full_tokens,
        trigger_tokens,
        effect_tokens,
    );
    if !authored_order || !first_effect_is_each_player_loop(effects) {
        return;
    }

    if let Some(crate::model::ast::EffectAst::SourceSentence {
        starting_with_controller,
        ..
    }) = effects.first_mut()
    {
        *starting_with_controller = true;
    } else {
        let ordered = std::mem::take(effects);
        effects.push(crate::model::ast::EffectAst::SourceSentence {
            effects: ordered,
            leading_then: false,
            starting_with_controller: true,
        });
    }
}

pub(super) fn attach_compiler_trigger_facts(
    context: ParseContextView<'_>,
    dispatch: &mut LineDispatchResult,
) -> Result<(), CardTextError> {
    for line in &mut dispatch.lines {
        let RecognizedLine::Triggered(triggered) = line else {
            continue;
        };

        let parse_nested_combat_cost = |tokens: &[crate::lexer::OwnedLexToken]| -> Result<
            Option<ironsmith_core::TotalCost<crate::model::CompilerCost>>,
            CardTextError,
        > {
            let Some((_, after_intro)) = crate::grammar::primitives::parse_prefix(
                tokens,
                crate::grammar::primitives::phrase(&[
                    "at",
                    "the",
                    "beginning",
                    "of",
                    "each",
                    "combat",
                ]),
            ) else {
                return Ok(None);
            };
            let after_intro = crate::lexer::trim_lexed_commas(after_intro);
            let Some((_, after_pay)) = crate::grammar::primitives::parse_prefix(
                after_intro,
                crate::grammar::primitives::phrase(&["unless", "you", "pay"]),
            ) else {
                return Ok(None);
            };
            let Some((cost_tokens, nested_trigger_tokens)) =
                crate::grammar::primitives::split_lexed_once_on_comma(after_pay)
            else {
                return Ok(None);
            };
            if !nested_trigger_tokens
                .first()
                .is_some_and(|token| token.is_word("whenever"))
            {
                return Ok(None);
            }
            let cost = crate::grammar::leaf::parse_leaf_mana_cost_tokens(
                crate::lexer::trim_lexed_commas(cost_tokens),
            )?;
            Ok(Some(
                ironsmith_core::TotalCost::<crate::model::CompilerCost>::mana(cost),
            ))
        };
        let mut nested_combat_cost = parse_nested_combat_cost(&triggered.full_parse_tokens)?;
        if nested_combat_cost.is_none() {
            nested_combat_cost = parse_nested_combat_cost(&triggered.info.source_tokens)?;
        }
        let recognized_special =
            semantic_grammar::parse_special_triggered_program_tokens(&triggered.full_parse_tokens);
        let special_triggered_program = match recognized_special {
            Some(semantic_grammar::SpecialTriggeredProgram::OpponentCreatureMajorityConsult) => {
                let trigger = super::super::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
                    context,
                    &triggered.trigger_parse_tokens,
                )?;
                let revealed_tag = crate::tag::CompilerReferenceTag::OathRevealed.key();
                let creature_tag = crate::tag::CompilerReferenceTag::OathCreature.key();
                let mut creature_card_filter = crate::ObjectFilter::creature();
                creature_card_filter.zone = None;
                let membership_filter = crate::ObjectFilter::default()
                    .same_stable_id_as_tagged(crate::tag::CompilerReferenceTag::It.key());
                Some((
                    trigger,
                    vec![
                        crate::host::EffectAst::subject_verb_explicit_target_only_for_chooser(
                            crate::TargetAst::Player(
                                crate::PlayerFilter::OpponentWithMoreControlledObjectsThan {
                                    player: Box::new(crate::PlayerFilter::Active),
                                    filter: Box::new(crate::ObjectFilter::creature()),
                                },
                                Some(crate::TextSpan::synthetic()),
                            ),
                            crate::PlayerAst::Active,
                        ),
                        crate::host::EffectAst::MayByPlayer {
                            player: crate::PlayerAst::Active,
                            effects: vec![
                                crate::host::EffectAst::subject_verb_consult_top_of_library(
                                    crate::PlayerAst::Active,
                                    crate::cards::builders::LibraryConsultModeAst::Reveal,
                                    creature_card_filter,
                                    crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
                                    revealed_tag.clone(),
                                    creature_tag.clone(),
                                ),
                                crate::host::EffectAst::subject_verb_move_to_zone(
                                    crate::TargetAst::Tagged(creature_tag.clone(), None),
                                    crate::Zone::Battlefield,
                                    false,
                                    crate::ReturnControllerAst::Preserve,
                                    false,
                                    None,
                                ),
                                crate::host::EffectAst::ForEachTagged {
                                    tag: revealed_tag,
                                    effects: vec![crate::host::EffectAst::Conditional {
                                        predicate: crate::host::PredicateAst::TaggedMatches(
                                            creature_tag,
                                            membership_filter,
                                        ),
                                        if_true: Vec::new(),
                                        if_false: vec![
                                            crate::host::EffectAst::subject_verb_move_to_zone(
                                                crate::TargetAst::Tagged(
                                                    crate::tag::CompilerReferenceTag::It.key(),
                                                    None,
                                                ),
                                                crate::Zone::Graveyard,
                                                false,
                                                crate::ReturnControllerAst::Preserve,
                                                false,
                                                None,
                                            ),
                                        ],
                                    }],
                                },
                            ],
                        },
                    ],
                ))
            }
            Some(semantic_grammar::SpecialTriggeredProgram::PrimeControlledLandCountToken) => {
                use crate::model::ast::{EffectAst, SubjectVerbActionAst};

                let trigger = super::super::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
                    context,
                    &triggered.trigger_parse_tokens,
                )?;
                let segments =
                    crate::grammar::effects::chain_splitting::split_segments_on_comma_then_tokens(
                        vec![triggered.effect_parse_tokens.as_slice()],
                        |_| false,
                    );
                let [create_tokens, counter_tokens] = segments.as_slice() else {
                    return Err(CardTextError::InvariantViolation(
                        "prime-count token program lost its create/counter boundary".to_string(),
                    ));
                };
                let mut create_effects = parse_effect_sentences_lexed(create_tokens)?;
                let [create_effect] = create_effects.as_mut_slice() else {
                    return Err(CardTextError::InvariantViolation(
                        "prime-count token program did not produce one creation effect".to_string(),
                    ));
                };
                let created_tag = crate::util::helper_tag_for_tokens(
                    &triggered.effect_parse_tokens,
                    "prime_count_token",
                );
                let tagged_create = EffectAst::TagAffected {
                    effect: Box::new(create_effect.clone()),
                    tag: created_tag.clone(),
                };

                let mut counter_effects = parse_effect_sentences_lexed(counter_tokens)?;
                let [counter_effect] = counter_effects.as_mut_slice() else {
                    return Err(CardTextError::InvariantViolation(
                        "prime-count token program did not produce one counter effect".to_string(),
                    ));
                };
                let EffectAst::SubjectVerb(counter_verb) = counter_effect else {
                    return Err(CardTextError::InvariantViolation(
                        "prime-count token follow-up was not a subject/verb counter effect"
                            .to_string(),
                    ));
                };
                let SubjectVerbActionAst::PutCounters { count, target, .. } =
                    &mut counter_verb.action
                else {
                    return Err(CardTextError::InvariantViolation(
                        "prime-count token follow-up did not put counters".to_string(),
                    ));
                };
                let controlled_lands = crate::ObjectFilter::land().you_control();
                *count = crate::Value::Count(controlled_lands.clone())
                    .with_surface_hint(ironsmith_core::ValueSurfaceHint::ThatMany);
                *target = crate::TargetAst::Tagged(created_tag, Some(crate::TextSpan::synthetic()));
                triggered.intervening_if = Some(crate::host::PredicateAst::And(
                    Box::new(crate::host::PredicateAst::ObjectEnteredBattlefieldThisTurn(
                        controlled_lands.clone(),
                    )),
                    Box::new(crate::host::PredicateAst::ValueIsPrime(
                        crate::Value::Count(controlled_lands),
                    )),
                ));
                Some((
                    trigger,
                    vec![EffectAst::CommaThen {
                        effects: vec![tagged_create, counter_effect.clone()],
                    }],
                ))
            }
            Some(semantic_grammar::SpecialTriggeredProgram::OpponentGraveyardMinorityReturn) => {
                let trigger = super::super::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
                    context,
                    &triggered.trigger_parse_tokens,
                )?;
                let mut graveyard_creatures = crate::ObjectFilter::creature();
                graveyard_creatures.zone = Some(crate::Zone::Graveyard);

                let mut return_filter = graveyard_creatures.clone();
                return_filter.owner = Some(crate::PlayerFilter::IteratedPlayer);
                Some((
                    trigger,
                    vec![crate::host::EffectAst::Conditional {
                        predicate: crate::host::PredicateAst::AnOpponentHasFewerThanPlayer {
                            player: crate::PlayerAst::That,
                            filter: graveyard_creatures,
                        },
                        if_true: vec![crate::host::EffectAst::MayByPlayer {
                            player: crate::PlayerAst::That,
                            effects: vec![crate::host::EffectAst::subject_verb_return_to_hand(
                                crate::TargetAst::Object(return_filter, None, None),
                                false,
                            )],
                        }],
                        if_false: Vec::new(),
                    }],
                ))
            }
            _ => None,
        };
        let direct = if let Some(program) = special_triggered_program {
            Ok(Some(program))
        } else if let Some(cost) = nested_combat_cost {
            super::super::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
                context,
                &triggered.trigger_parse_tokens,
            )
            .and_then(|nested_trigger| {
                crate::semantic_line_parsing::parse_effect_sentences_preserving_source_boundaries(
                    &triggered.effect_parse_tokens,
                )
                .map(|nested_effects| {
                    (
                        crate::model::ast::TriggerSpec::BeginningOfCombat(
                            crate::target::PlayerFilter::Any,
                        ),
                        vec![crate::model::ast::EffectAst::UnlessPays {
                            effects: vec![
                                crate::model::ast::EffectAst::DelayedTriggerForDuration {
                                    trigger: nested_trigger,
                                    effects: nested_effects,
                                    one_shot: false,
                                    duration: crate::effect::Until::EndOfCombat,
                                    either_of_watched_objects: false,
                                    while_any_tagged_object_in_zone: None,
                                },
                            ],
                            player: crate::PlayerAst::You,
                            cost,
                            before_delayed_step: false,
                        }],
                    )
                })
            })
            .map(Some)
        } else if triggered.trigger_parse_tokens.is_empty()
            || triggered.effect_parse_tokens.is_empty()
        {
            Ok(None)
        } else {
            super::super::activation_and_restrictions::parse_trigger_clause_lexed_with_context(
                context,
                &triggered.trigger_parse_tokens,
            )
            .and_then(|trigger| {
                let linked_token_effects = crate::semantic_line_parsing::linked_created_token_next_turn_sacrifice_effects(
                    &triggered.effect_parse_tokens,
                )?;
                let effects = linked_token_effects
                .or_else(|| crate::semantic_line_parsing::end_of_combat_destroy_then_next_end_step_counter_program(
                    &triggered.effect_parse_tokens,
                ))
                .or_else(|| crate::semantic_line_parsing::exact_target_graveyard_any_type_may_cast_bundle(
                    &triggered.effect_parse_tokens,
                ))
                .map(Ok)
                .unwrap_or_else(|| {
                    crate::semantic_line_parsing::parse_effect_sentences_preserving_source_boundaries(
                        &triggered.effect_parse_tokens,
                    )
                });
                effects
                .map(|effects| (trigger, effects))
            })
            .map(Some)
        };
        let fallback = || {
            triggered_program_from_line_ast(parse_triggered_line_lexed(
                &triggered.full_parse_tokens,
            )?)
            .ok_or_else(|| {
                CardTextError::InvariantViolation(
                    "trigger line produced no compiler trigger program".to_string(),
                )
            })
        };
        let (trigger, mut effects) = match direct {
            Ok(Some(program)) => program,
            Ok(None) => fallback()?,
            Err(direct_error) => match fallback() {
                Ok(program) => program,
                Err(_) => return Err(direct_error),
            },
        };
        preserve_split_participant_order_surface(
            &triggered.trigger_parse_tokens,
            &triggered.effect_parse_tokens,
            &triggered.full_parse_tokens,
            &mut effects,
        );
        preserve_named_source_exile_surface(context, &triggered.info.source_tokens, &mut effects);
        preserve_named_source_transform_surface(
            context,
            &triggered.info.source_tokens,
            &mut effects,
        );
        preserve_named_source_unattach_surface(
            context,
            &triggered.info.source_tokens,
            &mut effects,
        );
        preserve_named_source_put_counters_surface(
            context,
            &triggered.info.source_tokens,
            &mut effects,
        );
        preserve_named_source_chosen_complement_surface(
            context,
            &triggered.info.source_tokens,
            &mut effects,
        );
        let functional_zones =
            super::super::semantic_line_parsing::derive_triggered_ability_functional_zones_from_facts(
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
            .compiler_ability = Some(Box::new(compiler_ability));
    }
    Ok(())
}

fn dispatch_line_family_registry(
    ctx: &LineDispatchContext<'_>,
) -> ParseOutcome<LineDispatchResult> {
    // Sticker-sheet ticket rows are presentation data even when the text to
    // the right of the dash looks like a trigger, activation, or keyword.
    // Claim them before every ordinary line-family rule.
    if let Some(dispatch) = super::line_family_handlers::sticker_sheet_ticket_marker_result(ctx) {
        return ParseOutcome::matched(dispatch, span_from_tokens(&ctx.line.tokens));
    }

    // Borrow preprocessing expands a removed-from-draft `The same is true`
    // ladder into independent leading-condition sentences. Preserve that
    // complete typed program before keyword discovery can claim consequence
    // words such as flying or haste as one unconditional keyword line.
    let removed_draft_span = span_from_tokens(&ctx.line.tokens);
    let removed_draft_outcome =
        match crate::keyword_static::parse_removed_draft_leading_conditional_static_sentence_chain(
            &ctx.line.tokens,
        ) {
            Ok(Some(abilities)) => ParseOutcome::matched(abilities, removed_draft_span),
            Ok(None) => ParseOutcome::NoMatch,
            Err(error) => ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
                RuleId::new("removed-draft-leading-conditional-static-chain"),
                removed_draft_span,
                error,
            )),
        };
    match removed_draft_outcome {
        ParseOutcome::Match(matched) => {
            return ParseOutcome::matched(
                LineDispatchResult::single(
                    RecognizedLine::Static(RecognizedStaticLine {
                        info: ctx.line.info.clone(),
                        parse_tokens: ctx.line.tokens.clone(),
                        chosen_option: None,
                        parsed: Some(Box::new(LineAst::StaticAbilities(matched.value))),
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
        match (rule.run)(ctx).within(rule.id) {
            ParseOutcome::Match(matched) => {
                candidates.push(RegistryCandidate::new(
                    RegistryRuleMetadata::distinct(rule.id, rule.head),
                    matched.value,
                    matched.span,
                ));
            }
            ParseOutcome::NoMatch => {
                parse_trace::event(format!("line-family: {} -> no match", rule.id));
            }
            ParseOutcome::Error(diagnostic) => {
                parse_trace::event(format!("line-family: {} errored: {diagnostic:?}", rule.id));
                diagnostics.push(diagnostic);
            }
        }
    }

    match resolve_registry_candidates(RuleId::new("line-family-registry"), candidates, diagnostics)
    {
        ParseOutcome::Match(matched) => {
            let rule_match = matched.value;
            let mut dispatch = rule_match.value;
            if let Err(error) = attach_compiler_trigger_facts(ctx.parse, &mut dispatch) {
                return ParseOutcome::Error(ParseDiagnostic::from_card_text_error(
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

    match run_unsupported_line_family(ctx) {
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
            parse_trace::event(format!("line-family: unsupported errored: {diagnostic:?}"));
            ParseOutcome::Error(diagnostic)
        }
    }
}

pub(super) fn dispatch_standard_line(
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
    match dispatch_line_family_registry(&ctx) {
        ParseOutcome::Match(matched) => Ok(matched.value),
        ParseOutcome::Error(diagnostic) => Err(diagnostic.into_card_text_error()),
        ParseOutcome::NoMatch => Err(CardTextError::InvariantViolation(format!(
            "line-family registry returned no match for '{}'",
            line.info.raw_line
        ))),
    }
}

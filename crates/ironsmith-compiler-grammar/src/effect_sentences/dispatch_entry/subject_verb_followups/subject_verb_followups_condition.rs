use super::*;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::SourcePredicateAst;

/// Binds the replacement's pronouns to the default action's object up to and
/// including the first action that moves that object to another zone
/// ("instead exile it, then return that card to its owner's hand"). After
/// the move the object is a new one (CR 400.7), so a later pronoun names the
/// moved card through ordinary reference flow instead of the old target.
fn replace_it_target_until_antecedent_moves(effects: &mut [EffectAst], target: &TargetAst) -> bool {
    for effect in effects {
        match effect {
            EffectAst::Sequence { effects } | EffectAst::CommaThen { effects } => {
                if replace_it_target_until_antecedent_moves(effects, target) {
                    return true;
                }
            }
            _ => {
                replace_it_target(effect, target);
                if matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ZoneMoves(
                            ZoneMoveActionAst::Exile { .. }
                                | ZoneMoveActionAst::ReturnToHand { .. }
                                | ZoneMoveActionAst::MoveToZone { .. }
                        ),
                        ..
                    })
                ) {
                    return true;
                }
            }
        }
    }
    false
}

/// "If this spell's madness cost was paid, instead gain control of that
/// creature if its toughness is X or less" (Welcome to the Fold): the
/// replacement's local gate reads the same object as its action ("that
/// creature"), which the replacement already addresses through the prior
/// action's result tag. The replacement branch never runs after the default
/// action, so an unbound `its` there must not fall back to the source.
fn bind_replacement_it_characteristics_to_tag(effects: &mut [EffectAst], tag: &TagKey) {
    fn bind_value(value: &mut Value, tag: &TagKey) {
        match value {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) => {
                let rebind = match spec.base() {
                    crate::ChooseSpec::Tagged(found) => {
                        found.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                    }
                    crate::ChooseSpec::Source => true,
                    _ => false,
                };
                if rebind {
                    **spec = crate::ChooseSpec::Tagged(tag.clone());
                }
            }
            Value::SurfaceHinted { value, .. } => bind_value(value, tag),
            _ => {}
        }
    }
    fn bind_predicate(predicate: &mut PredicateAst, tag: &TagKey) {
        match predicate {
            PredicateAst::ValueComparison { left, right, .. } => {
                bind_value(left, tag);
                bind_value(right, tag);
            }
            PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
                bind_predicate(left, tag);
                bind_predicate(right, tag);
            }
            PredicateAst::Not(inner) => bind_predicate(inner, tag),
            _ => {}
        }
    }
    for effect in effects {
        match effect {
            EffectAst::Conditionals(ConditionalEffectAst::Conditional { predicate, .. })
            | EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, .. }) => {
                bind_predicate(predicate, tag);
            }
            EffectAst::ControlFlow(control) => {
                if let crate::model::control_flow::ControlFlowNodeAst::Condition {
                    condition, ..
                } = &mut control.node
                    && let crate::model::control_flow::ControlPredicateAst::State(predicate) =
                        &mut condition.predicate
                {
                    bind_predicate(predicate, tag);
                }
            }
            _ => {}
        }
    }
}

pub(super) fn pre_rule_conditional_optional_result_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let continuation = super::super::super::super::token_primitives::strip_leading_if_you_do_lexed(
        sentence_tokens,
    );
    let is_when_you_do = sentence_tokens.len() >= 3
        && sentence_tokens[0].is_word("when")
        && sentence_tokens[1].is_word("you")
        && sentence_tokens[2].is_word("do");
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    let is_if_you_dont = words.starts_with(&["if", "you", "don't"])
        || words.starts_with(&["if", "you", "dont"])
        || words.starts_with(&["if", "you", "do", "not"]);
    if continuation.len() == sentence_tokens.len() && !is_when_you_do && !is_if_you_dont {
        return Ok(None);
    }
    // An explicit failed action ("If you don't cast it, ...") observes
    // whether that action happened, including when its eligibility condition
    // was false. The ordinary result-followup grammar owns this sibling
    // instruction. Moving it into the condition would skip the fallback.
    // Compact "if/when you do" continuations retain their branch scope.
    if is_if_you_dont
        && crate::effect_sentences::consult_family::explicit_if_you_dont_action_remainder(
            sentence_tokens,
        ).is_some()
    {
        return Ok(None);
    }
    let effects = match state.effects.last_mut() {
        Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. })) => effects,
        Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true,
            if_false,
            ..
        })) if if_false.is_empty() => if_true,
        _ => return Ok(None),
    };
    // Both acceptance and refusal continuations belong to the optional action's
    // branch. A skipped outer condition must not execute the refusal outcome.
    let producer = effects.iter().rev().find(|effect| {
        !matches!(
            effect,
            EffectAst::Conditionals(ConditionalEffectAst::IfResult { .. })
        )
    });
    if !matches!(
        producer,
        Some(EffectAst::Permissions(
            PermissionEffectAst::May { .. } | PermissionEffectAst::MayByPlayer { .. }
        ))
    ) {
        return Ok(None);
    }

    // The authored "if/when you do" refers to the optional action inside the
    // preceding branch, not that branch's original coin flip/clash result.
    // Preserve that scope before both spellings lower to a result predicate.
    let followup = super::super::super::parse_effect_chain_lexed(sentence_tokens)?;
    effects.extend(followup);
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

pub(super) fn pre_rule_if_no_one_does_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    if shape.kind != followup_shapes::ConditionalFollowupKind::IfNoOneDoes {
        return Ok(None);
    }
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::DidNot);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

pub(super) fn pre_rule_if_you_win_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    let predicate = match shape.kind {
        followup_shapes::ConditionalFollowupKind::IfYouWinClash => IfResultPredicate::WonClash,
        followup_shapes::ConditionalFollowupKind::IfYouWinFlip => IfResultPredicate::Did,
        followup_shapes::ConditionalFollowupKind::IfYouWin => {
            let preceded_by_clash = state.effects.last().is_some_and(|effect| {
                terminal_result_producer(effect) == Some(TerminalResultProducer::Clash)
            });
            if preceded_by_clash {
                IfResultPredicate::WonClash
            } else {
                IfResultPredicate::Did
            }
        }
        _ => return Ok(None),
    };
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(predicate);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

/// A sacrifice-qualified self-replacement tests an action that already
/// happened (possibly an activation cost), before either token-producing
/// branch runs. Preserve the typed action/filter as a pending count rather
/// than linking an `IfResult` to the default instruction it replaces.
fn sacrifice_self_replacement_predicate(predicate: &IfResultPredicate) -> Option<PredicateAst> {
    use ironsmith_core::{
        EffectMetric, EffectMetricSource, PriorEffectAction, PriorEffectMetricQuery,
        PriorEffectResultActor, PriorEffectResultQuantifier,
    };
    let IfResultPredicate::PriorEffectResult(surface) = predicate else {
        return None;
    };
    if surface.action != PriorEffectAction::Sacrificed
        || surface.quantifier != PriorEffectResultQuantifier::One
        || surface.required_count.is_some()
        || surface.shared_characteristic.is_some()
    {
        return None;
    }
    let mut query =
        PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count)
            .with_filter(surface.filter.clone())
            .with_action(PriorEffectAction::Sacrificed);
    match surface.actor {
        PriorEffectResultActor::You => query = query.with_player(PlayerFilter::You),
        PriorEffectResultActor::Passive => {}
        _ => return None,
    }
    Some(PredicateAst::ValueComparison {
        left: Value::PendingPriorEffectMetric(query),
        operator: ironsmith_core::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    })
}

pub(super) fn take_self_replacement_condition(
    effect: EffectAst,
) -> Option<(PredicateAst, Vec<EffectAst>, Vec<EffectAst>)> {
    match effect {
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }) => Some((predicate, if_true, if_false)),
        EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, effects }) => Some((
            sacrifice_self_replacement_predicate(&predicate)?,
            effects,
            Vec::new(),
        )),
        // Damage parsing preserves authored trailing condition order with a
        // typed `TrailingIf`. Once an `instead` follow-up has been classified
        // as a self-replacement, both surfaces carry the same semantic branch
        // and must be normalized before ordinary object-reference lowering.
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects }) => {
            Some((predicate, effects, Vec::new()))
        }
        EffectAst::ControlFlow(control) => {
            let crate::model::control_flow::ControlFlowNodeAst::Condition {
                condition,
                consequence_program,
                alternative_program,
                ..
            } = &control.node
            else {
                return None;
            };
            let predicate = match &condition.predicate {
                crate::model::control_flow::ControlPredicateAst::State(predicate) => {
                    predicate.clone()
                }
                crate::model::control_flow::ControlPredicateAst::Result(predicate) => {
                    sacrifice_self_replacement_predicate(predicate)?
                }
                _ => return None,
            };
            let if_true = control.program(*consequence_program)?.effects.clone();
            let if_false = alternative_program
                .and_then(|program| control.program(program))
                .map(|program| program.effects.clone())
                .unwrap_or_default();
            Some((predicate.clone(), if_true, if_false))
        }
        _ => None,
    }
}

/// "If you control a Villain with greater mana value than that creature, ...
/// instead" (Evil's Thrall): the gate compares against the default action's
/// object. A self-replacement gate is evaluated before either branch runs, so
/// no tag names that object yet, and an unbound `it` would fall back to the
/// source. Report it rather than comparing against the wrong object.
fn gate_compares_against_unbound_antecedent(predicate: &PredicateAst) -> bool {
    fn reads_it(value: &Value) -> bool {
        match value {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) => {
                matches!(spec.base(), crate::target::ChooseSpec::Tagged(found)
                    if found.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
            }
            Value::SurfaceHinted { value, .. } => reads_it(value),
            _ => false,
        }
    }
    fn comparison_reads_it(comparison: Option<&crate::filter::Comparison>) -> bool {
        use crate::filter::Comparison as C;
        matches!(
            comparison,
            Some(
                C::EqualExpr(value)
                    | C::NotEqualExpr(value)
                    | C::LessThanExpr(value)
                    | C::LessThanOrEqualExpr(value)
                    | C::GreaterThanExpr(value)
                    | C::GreaterThanOrEqualExpr(value)
            ) if reads_it(value)
        )
    }
    fn filter_reads_it(filter: &ObjectFilter) -> bool {
        comparison_reads_it(filter.power.as_ref())
            || comparison_reads_it(filter.toughness.as_ref())
            || comparison_reads_it(filter.mana_value.as_ref())
            || filter.any_of.iter().any(filter_reads_it)
    }
    use crate::cards::builders::PlayerPredicateAst as P;
    match predicate {
        PredicateAst::Player(
            P::PlayerControls { filter, .. }
            | P::PlayerControlsNo { filter, .. }
            | P::PlayerControlsExactly { filter, .. }
            | P::PlayerHasAtLeast { filter, .. },
        ) => filter_reads_it(filter),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            gate_compares_against_unbound_antecedent(left)
                || gate_compares_against_unbound_antecedent(right)
        }
        PredicateAst::Not(inner) => gate_compares_against_unbound_antecedent(inner),
        _ => false,
    }
}

/// The resolution-time spec for the default action's single announced
/// target. The runtime evaluates a self-replacement gate with that action's
/// targets in scope, so a target spec reads the targeted object.
fn gate_antecedent_target_spec(target: &TargetAst) -> Option<crate::target::ChooseSpec> {
    use crate::target::ChooseSpec;
    match target {
        TargetAst::Object(filter, Some(_), _) => {
            Some(ChooseSpec::Target(Box::new(ChooseSpec::Object(filter.clone()))))
        }
        TargetAst::AnyTarget(Some(_)) => Some(ChooseSpec::AnyTarget),
        TargetAst::AnyOtherTarget(Some(_)) => Some(ChooseSpec::AnyOtherTarget),
        TargetAst::WithCount(inner, count) | TargetAst::WithCountValue(inner, count, _)
            if count.min == 1 && count.max == Some(1) =>
        {
            gate_antecedent_target_spec(inner)
        }
        _ => None,
    }
}

/// Rebinds the gate's characteristic comparisons that read the unbound `it`
/// ("than that creature") to the default action's announced target.
fn bind_gate_antecedent_comparisons_to_target(
    predicate: &mut PredicateAst,
    target: &crate::target::ChooseSpec,
) {
    fn bind_spec(spec: &mut crate::target::ChooseSpec, target: &crate::target::ChooseSpec) {
        if let crate::target::ChooseSpec::SurfaceHinted { spec, .. } = spec {
            bind_spec(spec, target);
            return;
        }
        if matches!(spec, crate::target::ChooseSpec::Tagged(found)
            if found.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        {
            *spec = target.clone();
        }
    }
    fn bind_value(value: &mut Value, target: &crate::target::ChooseSpec) {
        match value {
            Value::PowerOf(spec) | Value::ToughnessOf(spec) | Value::ManaValueOf(spec) => {
                bind_spec(spec, target)
            }
            Value::SurfaceHinted { value, .. } => bind_value(value, target),
            _ => {}
        }
    }
    fn bind_comparison(
        comparison: Option<&mut crate::filter::Comparison>,
        target: &crate::target::ChooseSpec,
    ) {
        use crate::filter::Comparison as C;
        if let Some(
            C::EqualExpr(value)
            | C::NotEqualExpr(value)
            | C::LessThanExpr(value)
            | C::LessThanOrEqualExpr(value)
            | C::GreaterThanExpr(value)
            | C::GreaterThanOrEqualExpr(value),
        ) = comparison
        {
            bind_value(value, target);
        }
    }
    fn bind_filter(filter: &mut ObjectFilter, target: &crate::target::ChooseSpec) {
        bind_comparison(filter.power.as_mut(), target);
        bind_comparison(filter.toughness.as_mut(), target);
        bind_comparison(filter.mana_value.as_mut(), target);
        for branch in &mut filter.any_of {
            bind_filter(branch, target);
        }
    }
    use crate::cards::builders::PlayerPredicateAst as P;
    match predicate {
        PredicateAst::Player(
            P::PlayerControls { filter, .. }
            | P::PlayerControlsNo { filter, .. }
            | P::PlayerControlsExactly { filter, .. }
            | P::PlayerHasAtLeast { filter, .. },
        ) => bind_filter(filter, target),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            bind_gate_antecedent_comparisons_to_target(left, target);
            bind_gate_antecedent_comparisons_to_target(right, target);
        }
        PredicateAst::Not(inner) => bind_gate_antecedent_comparisons_to_target(inner, target),
        _ => {}
    }
}

fn explicit_self_replacement_result_tag(effect: &EffectAst) -> Option<TagKey> {
    match effect {
        EffectAst::TagAffected { tag, .. }
            if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            Some(tag.clone().into())
        }
        EffectAst::Sequence { effects }
        | EffectAst::CommaThen { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::Coordinated { effects, .. }
        | EffectAst::Permissions(PermissionEffectAst::May { effects })
        | EffectAst::Permissions(PermissionEffectAst::MayByPlayer { effects, .. })
            if effects.len() == 1 =>
        {
            explicit_self_replacement_result_tag(&effects[0])
        }
        _ => None,
    }
}

pub(super) fn predicate_explicitly_says_that_land(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::Source(SourcePredicateAst::SourceMatches(filter))
        | PredicateAst::ItMatches(filter)
        | PredicateAst::TargetMatches(filter) => {
            filter.demonstrative_antecedent_surface()
                == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land)
        }
        PredicateAst::Not(inner) => predicate_explicitly_says_that_land(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_explicitly_says_that_land(left) || predicate_explicitly_says_that_land(right)
        }
        _ => false,
    }
}

/// Effects authored after a self-replacement happen regardless of which arm
/// replaced the original event. Keep that common suffix in both arms so the
/// lowering boundary remains one executable self-replacement segment and
/// branch-local pronouns resolve against the object produced by that arm.
pub(super) fn post_rule_self_replacement_common_suffix(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    if sentence_effects.is_empty()
        || matches!(
            classify_instead_followup_tokens(sentence_tokens),
            InsteadSemantics::SelfReplacement
        )
    {
        return Ok(None);
    }
    let Some(EffectAst::SelfReplacement {
        if_true, if_false, ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };

    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    if crate::word_primitives::parse_sequence_prefix(
        &words,
        &["exile", "the", "chosen", "creature", "then"],
    ) && crate::word_primitives::sequence_occurs(
        &words,
        &["controller", "gains", "life", "equal", "to"],
    ) && crate::word_primitives::sequence_occurs(&words, &["mana", "value"])
        && !effects_contain_gain_life(sentence_effects)
    {
        let iterated = crate::tag::CompilerReferenceTag::It.bind();
        sentence_effects.push(EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::ItsController,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::GainLife {
                    amount: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(iterated.key.clone())))
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
                }),
            )],
        }));
    }

    if_true.extend(sentence_effects.iter().cloned());
    if_false.append(sentence_effects);
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

pub(in super::super) fn post_rule_future_zone_and_self_replacement(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    lowered_sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let sentence_effects_baseline = sentence_effects.clone();
    let sentence_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lexed)
        .unwrap_or(lowered_sentence_tokens);
    if matches!(
        classify_instead_followup_tokens(sentence_tokens),
        InsteadSemantics::SelfReplacement
    ) {
        for effect in sentence_effects.iter() {
            let result_predicate = match effect {
                EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, .. }) => {
                    Some(predicate)
                }
                EffectAst::ControlFlow(control) => match &control.node {
                    crate::model::control_flow::ControlFlowNodeAst::Condition {
                        condition, ..
                    } => match &condition.predicate {
                        crate::model::control_flow::ControlPredicateAst::Result(predicate) => {
                            Some(predicate)
                        }
                        _ => None,
                    },
                    _ => None,
                },
                _ => None,
            };
            if result_predicate
                .is_some_and(|predicate| sacrifice_self_replacement_predicate(predicate).is_none())
            {
                // The consequence parser removes the outer `instead` from
                // its action tokens. An unsupported result gate must not
                // survive as an ordinary additive IfResult, which would run
                // the default action before its supposed replacement.
                return Err(CardTextError::ParseError(format!(
                    "unsupported prior-result self-replacement predicate (clause: '{}')",
                    LexedClause::new(sentence_tokens).text(),
                )));
            }
        }
    }
    // "If you do, put a +1/+1 counter on it. If it's a Unicorn, put two
    // +1/+1 counters on it instead." The replacement modifies the action
    // inside the result branch, so apply it there and keep the branch linked
    // to the optional payment.
    if matches!(
        classify_instead_followup_tokens(sentence_tokens),
        InsteadSemantics::SelfReplacement
    ) && matches!(
        state.effects.last(),
        Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult { effects, .. }))
            if effects.len() == 1
    ) {
        let Some(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate,
            mut effects,
        })) = state.effects.pop()
        else {
            unreachable!("checked above");
        };
        state.effects.push(effects.remove(0));
        let result = post_rule_future_zone_and_self_replacement(
            state,
            sentences,
            sentence_idx,
            lowered_sentence_tokens,
            sentence_effects,
        );
        let inner = state.effects.pop().expect("the branch action was restored");
        state
            .effects
            .push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate,
                effects: vec![inner],
            }));
        return result;
    }
    maybe_rewrite_future_zone_replacement_sentence(sentence_effects, sentence_tokens);
    if matches!(
        classify_instead_followup_tokens(sentence_tokens),
        InsteadSemantics::SelfReplacement
    ) && sentence_effects.len() == 1
        && !state.effects.is_empty()
        && sentence_effects.first().is_some_and(|effect| {
            matches!(
                effect,
                EffectAst::Conditionals(ConditionalEffectAst::Conditional { .. })
                    | EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { .. })
            ) || matches!(
                effect,
                EffectAst::Conditionals(ConditionalEffectAst::IfResult { predicate, .. })
                    if sacrifice_self_replacement_predicate(predicate).is_some()
            ) || matches!(
                effect,
                EffectAst::ControlFlow(control)
                    if matches!(
                        &control.node,
                        crate::model::control_flow::ControlFlowNodeAst::Condition { .. }
                    )
            )
        })
        && let Some((predicate, mut if_true, mut if_false)) = sentence_effects
            .first()
            .cloned()
            .and_then(take_self_replacement_condition)
    {
        sentence_effects.clear();
        if let Some(replacement) = materialize_search_count_self_replacement(
            state.effects,
            predicate.clone(),
            &if_true,
            sentence_tokens,
        ) {
            state.effects.push(replacement);
            return Ok(Some(PostParseFollowupResult::Handled {
                consumed_sentences: 1,
            }));
        }
        let Some(mut previous) = state.effects.pop() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous effect for 'instead' conditional rewrite".to_string(),
            ));
        };
        let previous_target = primary_target_from_effect(&previous);
        let mut predicate = predicate;
        if gate_compares_against_unbound_antecedent(&predicate)
            && let Some(target) = previous_target
                .as_ref()
                .and_then(gate_antecedent_target_spec)
        {
            bind_gate_antecedent_comparisons_to_target(&mut predicate, &target);
        }
        if gate_compares_against_unbound_antecedent(&predicate)
            && !matches!(previous_target.as_ref(), Some(TargetAst::Source(_)))
        {
            return Err(CardTextError::ParseError(format!(
                "self-replacement gate compares against the default action's object, which is unbound when the gate is evaluated (clause: '{}')",
                LexedClause::new(sentence_tokens).text(),
            )));
        }
        let mut previous_result_tag = explicit_self_replacement_result_tag(&previous);
        let replacement_qualifies_antecedent = if_true
            .iter()
            .find_map(primary_target_from_effect)
            .as_ref()
            .is_some_and(target_has_authored_it_qualification);
        if previous_result_tag.is_none()
            && previous_target.is_some()
            && replacement_qualifies_antecedent
        {
            let tag =
                crate::util::helper_tag_for_tokens(sentence_tokens, "self_replacement_antecedent");
            previous = EffectAst::TagAffected {
                effect: Box::new(previous),
                tag: crate::tag::TagRef::of(tag.clone()),
            };
            previous_result_tag = Some(tag.key.clone());
        }
        let previous_damage_target = primary_damage_target_from_effect(&previous);
        let previous_damage_source = primary_damage_source_from_effect(&previous);
        let predicate = bind_self_replacement_condition_to_previous_target(
            predicate,
            sentence_tokens,
            previous_target.as_ref(),
        );
        bind_nested_self_replacement_condition_to_previous_target(
            &mut if_true,
            sentence_tokens,
            previous_target.as_ref(),
        );
        if has_trailing_unpreventable_damage_rider(sentence_tokens)
            && !mark_last_deal_damage_unpreventable(&mut if_true)
        {
            return Err(CardTextError::ParseError(format!(
                "unpreventable-damage replacement rider has no damage effect (clause: '{}')",
                LexedClause::new(sentence_tokens).text(),
            )));
        }
        let (mut default_effects, carried_player) =
            default_effects_for_self_replacement(state.effects, previous);
        if let Some(mill_count) = default_effects
            .iter()
            .rev()
            .find_map(mill_count_from_effect)
        {
            replace_mill_event_amounts_with_value(&mut if_true, &mill_count);
        }
        if let Some(player) = carried_player {
            bind_that_player_subjects_in_effects(&mut if_true, player);
        }
        preserve_search_owner_anaphor_in_self_replacement(&mut default_effects);
        preserve_search_owner_anaphor_in_self_replacement(&mut if_true);
        if let Some(owner) = first_search_library_owner(&default_effects) {
            bind_self_replacement_search_owner(&mut if_true, &owner);
        }
        if let Some(target) = previous_result_tag
            .as_ref()
            .map(|tag| TargetAst::Tagged(crate::tag::TagRef::of(tag.clone()), None))
            .as_ref()
            .or(previous_target.as_ref())
        {
            replace_it_target_until_antecedent_moves(&mut if_true, target);
        }
        if replacement_qualifies_antecedent && let Some(tag) = previous_result_tag.as_ref() {
            bind_replacement_it_characteristics_to_tag(&mut if_true, tag);
        }
        if let Some(target) = previous_damage_target.as_ref() {
            replace_it_damage_target_in_effects(&mut if_true, target);
            replace_placeholder_damage_target_in_effects(&mut if_true, target);
        }
        if let Some(source) = previous_damage_source.as_ref()
            && !previous_damage_target.as_ref().is_some_and(|target| {
                normalize_anaphoric_damage_self_replacement(
                    &mut if_true,
                    sentence_tokens,
                    source,
                    target,
                )
            })
        {
            // In an authored damage self-replacement, a leading source
            // pronoun ("It deals ... instead") repeats the source of the
            // default damage event. It must not bind to the most recent
            // object antecedent, which may come from an additional cost.
            replace_anaphoric_damage_source_in_effects(&mut if_true, source);
        }
        for effect in default_effects.into_iter().rev() {
            if_false.insert(0, effect);
        }
        state.effects.push(EffectAst::SelfReplacement {
            predicate,
            if_true,
            if_false,
            attach_to_previous_ability: false,
        });
        return Ok(Some(PostParseFollowupResult::Handled {
            consumed_sentences: 1,
        }));
    }
    if *sentence_effects == sentence_effects_baseline {
        Ok(None)
    } else {
        Ok(Some(PostParseFollowupResult::Annotated))
    }
}

pub(super) fn default_effects_for_self_replacement(
    prior_effects: &mut Vec<EffectAst>,
    previous: EffectAst,
) -> (Vec<EffectAst>, Option<PlayerAst>) {
    let mut default_effects = vec![previous];
    let mut carried_player = default_effects
        .iter()
        .rev()
        .find_map(carried_player_from_effect);

    let anchor_idx =
        if carried_player.is_none() && default_effects.iter().any(effect_has_that_player_subject) {
            let mut idx = prior_effects.len();
            let mut found = None;
            while idx > 0 {
                idx -= 1;
                if carried_player_from_effect(&prior_effects[idx]).is_some() {
                    found = Some(idx);
                    break;
                }
            }
            found
        } else {
            None
        };
    if let Some(anchor_idx) = anchor_idx {
        carried_player = carried_player_from_effect(&prior_effects[anchor_idx]);
        let mut anchored_default_effects = prior_effects.split_off(anchor_idx);
        anchored_default_effects.append(&mut default_effects);
        default_effects = anchored_default_effects;
    }

    if let Some(player) = carried_player {
        bind_that_player_subjects_in_effects(&mut default_effects, player);
    }

    (default_effects, carried_player)
}

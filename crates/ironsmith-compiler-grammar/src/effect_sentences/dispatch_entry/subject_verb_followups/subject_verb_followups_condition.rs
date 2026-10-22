use super::*;
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::SourcePredicateAst;

// An object pronoun in a mixed player/object instruction names its unique
// announced object, even when the player recipient was written first.
fn unique_announced_object_target(effect: &EffectAst) -> Option<TargetAst> {
    fn collect(effect: &EffectAst, targets: &mut Vec<TargetAst>) {
        if let Some(target @ TargetAst::Object(_, Some(_), _)) = primary_target_from_effect(effect)
            && !targets.contains(&target)
        {
            targets.push(target);
        }
        for_each_nested_effects(effect, true, |nested| {
            for effect in nested { collect(effect, targets); }
        });
    }
    let mut targets = Vec::new();
    collect(effect, &mut targets);
    (targets.len() == 1).then(|| targets.remove(0))
}

/// A count-only replacement's "of those creatures" repeats the selection
/// domain, not the result of a default sacrifice that will never execute.
fn preserve_sacrifice_domain_in_count_replacement(
    defaults: &[EffectAst], replacements: &mut [EffectAst], tokens: &[OwnedLexToken],
) {
    let words = LexedClause::new(tokens).word_refs();
    let Some(index) = words.windows(2).position(|part| part == ["of", "those"]) else { return; };
    let Some(noun) = words.get(index + 2).copied() else { return; };
    let tail = &words[index + 3..];
    if !(tail == ["instead"] || tail == ["of", "their", "choice", "instead"])
        || words.iter().filter(|word| **word == "sacrifices").count() != 1
    { return; }
    let card_type = match noun {
        "creatures" => crate::types::CardType::Creature,
        "artifacts" => crate::types::CardType::Artifact,
        "enchantments" => crate::types::CardType::Enchantment,
        "lands" => crate::types::CardType::Land,
        "planeswalkers" => crate::types::CardType::Planeswalker,
        "battles" => crate::types::CardType::Battle,
        _ => return,
    };
    fn collect(effects: &[EffectAst], found: &mut Vec<(SubjectVerbSubjectAst, ObjectFilter)>) {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {subject, action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {filter,target:None,one_of_referenced_set:false,..})}) = effect {
                found.push((subject.clone(),filter.clone()));
            }
            for_each_nested_effects(effect,true,|nested| collect(nested,found));
        }
    }
    let mut original = Vec::new(); collect(defaults,&mut original);
    let mut replacement = Vec::new(); collect(replacements,&mut replacement);
    let ([(actor, domain)], [(new_actor, new_domain)]) = (original.as_slice(),replacement.as_slice()) else { return; };
    let mut bare = ObjectFilter::default(); bare.zone = Some(Zone::Battlefield); bare.card_types = vec![card_type];
    let mut new_domain = new_domain.clone(); new_domain.union_surface = ObjectFilter::default().union_surface;
    // The demonstrative can arrive as the ordinary `it` result reference.
    // A replaced action produces no such result; it repeats the domain.
    if new_domain.tagged_constraints == ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()).tagged_constraints {
        new_domain.tagged_constraints.clear();
    }

    if actor != new_actor || domain.card_types != [card_type] || new_domain != bare { return; }
    fn replace(effects: &mut [EffectAst], domain: &ObjectFilter) {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {action: SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::Sacrifice {filter,target:None,..}),..}) = effect { *filter = domain.clone(); }
            for_each_nested_effects_mut(effect,true,|nested| replace(nested,domain));
        }
    }
    replace(replacements,domain);
}

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
        Some(EffectAst::ControlFlow(flow)) => {
            let crate::model::ControlFlowNodeAst::Condition {
                consequence_program, alternative_program, reflexive: false, ..
            } = &flow.node else { return Ok(None); };
            // The last authored branch owns its own optional payment and the
            // immediately following acceptance/refusal continuation.
            let branch = alternative_program.unwrap_or(*consequence_program);
            &mut flow.programs[branch].effects
        }
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
    let mut followup = super::super::super::parse_effect_chain_lexed(sentence_tokens)?;
    // Some chain readings ("If you do, you may play that card without paying
    // its mana cost", Leaf-Crowned Elder) read past the leading "if you do"
    // and return the bare action. The continuation is still gated on the
    // optional action having happened.
    let has_if_you_do_lead = continuation.len() < sentence_tokens.len()
        && !is_when_you_do
        && !is_if_you_dont;
    if has_if_you_do_lead
        && matches!(
            followup.as_slice(),
            [EffectAst::Permissions(
                PermissionEffectAst::May { .. } | PermissionEffectAst::MayByPlayer { .. }
            )]
        )
    {
        followup = vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followup,
        })];
    }
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
            // "you may clash with an opponent. If you win, ..." (Sentry Oak):
            // an optional clash still produces the result the follow-up reads.
            let preceded_by_clash = state.effects.last().is_some_and(|effect| {
                terminal_result_producer(effect) == Some(TerminalResultProducer::Clash)
                    || matches!(
                        effect,
                        EffectAst::Permissions(
                            crate::cards::builders::PermissionEffectAst::May { effects }
                                | crate::cards::builders::PermissionEffectAst::MayByPlayer {
                                    effects,
                                    ..
                                }
                        ) if effects.last().is_some_and(|inner| {
                            terminal_result_producer(inner)
                                == Some(TerminalResultProducer::Clash)
                        })
                    )
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
    // "If a creature card was discarded this way, draw two cards instead"
    // (Lukka, Wayward Bonder): a discard, like a sacrifice, already
    // happened before the replaced instruction, so its count is readable.
    let action = surface.action;
    if !matches!(action, PriorEffectAction::Sacrificed | PriorEffectAction::Discarded)
        || surface.quantifier != PriorEffectResultQuantifier::One
        || surface.required_count.is_some()
        || surface.shared_characteristic.is_some()
    {
        return None;
    }
    let mut query =
        PriorEffectMetricQuery::new(EffectMetricSource::AffectedObjects, EffectMetric::Count)
            .with_filter(surface.filter.clone())
            .with_action(action);
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
        let mut previous_target = primary_target_from_effect(&previous);
        if matches!(previous_target.as_ref(), Some(TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _))) {
            previous_target = unique_announced_object_target(&previous).or(previous_target);
        }
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
        preserve_sacrifice_domain_in_count_replacement(&default_effects, &mut if_true, sentence_tokens);
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

/// "Tap that creature as it phases in this way." (Oubliette): a rider on the
/// preceding source-linked phase-out, not a separate tap now.
pub(super) fn pre_rule_tap_as_it_phases_in_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    if !matches!(
        words.as_slice(),
        ["tap", "that" | "the", "creature" | "permanent", "as", "it", "phases", "in", "this", "way"]
            | ["tap", "it", "as", "it", "phases", "in", "this", "way"]
    ) {
        return Ok(None);
    }
    fn mark(effect: &mut EffectAst) -> bool {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PermanentState(
                        crate::cards::builders::PermanentStateActionAst::PhaseOut { duration, .. }
                        | crate::cards::builders::PermanentStateActionAst::PhaseOutAll {
                            duration,
                            ..
                        },
                    ),
                ..
            }) if *duration == crate::effects::PhaseOutDuration::UntilSourceLeaves => {
                *duration = crate::effects::PhaseOutDuration::UntilSourceLeavesThenTapped;
                true
            }
            _ => {
                let mut found = false;
                crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                    found |= nested.iter_mut().rev().any(mark);
                });
                found
            }
        }
    }
    if !state.effects.last_mut().is_some_and(mark) {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

/// "It can't phase in for as long as The Pandorica remains tapped. When The
/// Pandorica becomes untapped or leaves the battlefield, that permanent phases
/// in." Both sentences restate one source-linked phasing duration for the
/// preceding phase-out.
pub(super) fn pre_rule_phase_in_while_source_tapped_followup(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    let Some(source) = words
        .strip_prefix(&["it", "cant", "phase", "in", "for", "as", "long", "as"][..])
        .or_else(|| words.strip_prefix(&["it", "can't", "phase", "in", "for", "as", "long", "as"][..]))
        .and_then(|rest| rest.strip_suffix(&["remains", "tapped"][..]))
    else {
        return Ok(None);
    };
    if source.first() != Some(&"this") {
        let view = crate::lexer::TokenWordView::new(sentence_tokens);
        let (Some(start), Some(end)) = (view.map_word_to_token_start(8), view.map_word_to_token_start(words.len() - 2))
            else { return Ok(None); };
        if !matches!(crate::util::parse_target_phrase(&sentence_tokens[start..end]),
            Ok(TargetAst::Source(_)) | Ok(TargetAst::Object(ObjectFilter { source: true, .. }, ..)))
        { return Ok(None); }
    }
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let next_words = crate::lexer::parser_token_word_refs(next.lexed());
    let Some(next_source) = next_words.strip_prefix(&["when"][..])
        .and_then(|words| words.strip_suffix(&[
            "becomes", "untapped", "or", "leaves", "the", "battlefield", "that", "permanent", "phases", "in",
        ][..]))
    else { return Ok(None); };
    // Contextual self-reference normalization can spell one occurrence as
    // "this" while retaining the authored name in the following trigger.
    if next_source != source {
        let view = crate::lexer::TokenWordView::new(next.lexed());
        let (Some(start), Some(end)) = (view.map_word_to_token_start(1),
            view.map_word_to_token_start(next_source.len() + 1))
            else { return Ok(None); };
        let next_subject = &next.lexed()[start..end];
        if !matches!(crate::util::parse_target_phrase(next_subject),
            Ok(TargetAst::Source(_)) | Ok(TargetAst::Object(ObjectFilter { source: true, .. }, ..)))
            && !(source.first() == Some(&"this")
                && crate::grammar::source_surface_shapes::parse_named_surface(next_subject).is_some())
        { return Ok(None); }
    }
    fn mark(effect: &mut EffectAst) -> bool {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PermanentState(
                        crate::cards::builders::PermanentStateActionAst::PhaseOut {
                            duration,
                            ..
                        },
                    ),
                ..
            }) if *duration == crate::effects::PhaseOutDuration::UntilNextUntap => {
                *duration = crate::effects::PhaseOutDuration::UntilSourceUntapsOrLeaves;
                true
            }
            _ => {
                let mut found = false;
                crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                    if let Some(last) = nested.last_mut() { found |= mark(last); }
                });
                found
            }
        }
    }
    if !state.effects.last_mut().is_some_and(mark) {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 2,
        route: None,
    }))
}

/// "After the second main phase this turn, there's an additional combat phase
/// followed by an additional main phase. At the beginning of that combat,
/// untap all creatures that attacked this turn." (World at War)
///
/// During the second main phase the phases are added after it now; otherwise
/// a delayed trigger at the beginning of the postcombat main phase adds them.
/// "That combat" is the next combat after the insertion, so its rider is a
/// delayed beginning-of-combat trigger registered with the phases.
pub(super) fn pre_rule_after_second_main_additional_phases_followup(
    _state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    let head_ok = matches!(
        words.as_slice(),
        ["after", "the", "second", "main", "phase", "this", "turn", "theres" | "there's", "an", "additional", "combat", "phase", "followed", "by", "an", "additional", "main", "phase"]
            | ["after", "the", "second", "main", "phase", "this", "turn", "there", "is", "an", "additional", "combat", "phase", "followed", "by", "an", "additional", "main", "phase"]
    );
    if !head_ok {
        return Ok(None);
    }
    let mut phase_program = vec![EffectAst::subject_verb_additional_phases(vec![
        crate::effects::AdditionalPhase::Combat,
        crate::effects::AdditionalPhase::Main,
    ])];
    let mut consumed = 1;
    if let Some(next) = sentences.get(sentence_idx + 1) {
        let next_tokens = next.lexed();
        let view = crate::lexer::TokenWordView::new(next_tokens);
        let next_words = view.word_refs();
        if next_words.starts_with(&["at", "the", "beginning", "of", "that", "combat"]) {
            let Some(start) = view.token_index_after_words(6) else {
                return Ok(None);
            };
            let body = trim_commas(&next_tokens[start..]);
            let rider = crate::clause_support::parse_effect_sentences_lexed(&body)?;
            if rider.is_empty() {
                return Ok(None);
            }
            phase_program.push(EffectAst::Delayed(crate::cards::builders::DelayedEffectAst::DelayedTriggerThisTurn {
                trigger: crate::cards::builders::TriggerSpec::BeginningOfCombat(crate::target::PlayerFilter::Any),
                effects: rider,
                one_shot: true,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            }));
            consumed = 2;
        }
    }
    let delayed = EffectAst::Delayed(crate::cards::builders::DelayedEffectAst::DelayedTriggerThisTurn {
        trigger: crate::cards::builders::TriggerSpec::BeginningOfPostcombatMain {
            player: crate::target::PlayerFilter::You,
            surface: ironsmith_core::trigger_model::PostcombatMainPhaseSurface::SecondMain,
        },
        effects: phase_program.clone(),
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    });
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::SecondMainPhaseOfTurn,
            if_true: phase_program,
            if_false: vec![delayed],
        })]),
        consumed_sentences: consumed,
    })))
}

/// "There's an additional combat phase after this phase. At the beginning of
/// that combat, untap all creatures you control." (Moraug, Fury of Akoum)
///
/// The combat is inserted directly after the current phase, so "that combat"
/// is the next combat this turn: its rider is a one-shot delayed
/// beginning-of-combat trigger created with the phase, not a separate
/// "at the beginning of each combat" ability.
pub(super) fn pre_rule_additional_combat_after_this_phase_that_combat_followup(
    _state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    let head_ok = matches!(
        words.as_slice(),
        ["theres" | "there's", "an", "additional", "combat", "phase", "after", "this", "phase"]
            | ["there", "is", "an", "additional", "combat", "phase", "after", "this", "phase"]
    );
    if !head_ok {
        return Ok(None);
    }
    let Some(next) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let next_tokens = next.lexed();
    let view = crate::lexer::TokenWordView::new(next_tokens);
    if !view
        .word_refs()
        .starts_with(&["at", "the", "beginning", "of", "that", "combat"])
    {
        return Ok(None);
    }
    let Some(start) = view.token_index_after_words(6) else {
        return Ok(None);
    };
    let body = trim_commas(&next_tokens[start..]);
    let rider = crate::clause_support::parse_effect_sentences_lexed(&body)?;
    if rider.is_empty() {
        return Ok(None);
    }
    let mut program = crate::clause_support::parse_effect_sentences_lexed(sentence_tokens)?;
    if program.is_empty() {
        return Ok(None);
    }
    program.push(EffectAst::Delayed(
        crate::cards::builders::DelayedEffectAst::DelayedTriggerThisTurn {
            trigger: crate::cards::builders::TriggerSpec::BeginningOfCombat(
                crate::target::PlayerFilter::Any,
            ),
            effects: rider,
            one_shot: true,
            until_end_of_combat: false,
            attach_to_previous_ability: false,
        },
    ));
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(program),
        consumed_sentences: 2,
    })))
}

/// "After the first combat phase this turn, there's an additional combat
/// phase. At the beginning of that combat, untap all creatures that attacked
/// this turn." (Swinging Ship)
///
/// During the first combat phase the combat is added after it now; otherwise
/// a one-shot delayed trigger at the next end of combat this turn adds it,
/// provided that combat is still the turn's first (an instruction that
/// resolves after the first combat phase adds nothing). "That combat" is the
/// added combat, so its rider is a delayed beginning-of-combat trigger
/// registered with the phase.
pub(super) fn pre_rule_after_first_combat_additional_combat_followup(
    _state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    let head_ok = matches!(
        words.as_slice(),
        ["after", "the", "first", "combat", "phase", "this", "turn", "theres" | "there's", "an", "additional", "combat", "phase"]
            | ["after", "the", "first", "combat", "phase", "this", "turn", "there", "is", "an", "additional", "combat", "phase"]
    );
    if !head_ok {
        return Ok(None);
    }
    let mut phase_program = vec![EffectAst::subject_verb_additional_phases(vec![
        crate::effects::AdditionalPhase::Combat,
    ])];
    let mut consumed = 1;
    if let Some(next) = sentences.get(sentence_idx + 1) {
        let next_tokens = next.lexed();
        let view = crate::lexer::TokenWordView::new(next_tokens);
        let next_words = view.word_refs();
        if next_words.starts_with(&["at", "the", "beginning", "of", "that", "combat"]) {
            let Some(start) = view.token_index_after_words(6) else {
                return Ok(None);
            };
            let body = trim_commas(&next_tokens[start..]);
            let rider = crate::clause_support::parse_effect_sentences_lexed(&body)?;
            if rider.is_empty() {
                return Ok(None);
            }
            phase_program.push(EffectAst::Delayed(crate::cards::builders::DelayedEffectAst::DelayedTriggerThisTurn {
                trigger: crate::cards::builders::TriggerSpec::BeginningOfCombat(crate::target::PlayerFilter::Any),
                effects: rider,
                one_shot: true,
                until_end_of_combat: false,
                attach_to_previous_ability: false,
            }));
            consumed = 2;
        }
    }
    let delayed = EffectAst::Delayed(crate::cards::builders::DelayedEffectAst::DelayedTriggerThisTurn {
        trigger: crate::cards::builders::TriggerSpec::EndOfCombat,
        effects: vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::FirstCombatPhaseOfTurn,
            if_true: phase_program.clone(),
            if_false: Vec::new(),
        })],
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    });
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::FirstCombatPhaseOfTurn,
            if_true: phase_program,
            if_false: vec![delayed],
        })]),
        consumed_sentences: consumed,
    })))
}

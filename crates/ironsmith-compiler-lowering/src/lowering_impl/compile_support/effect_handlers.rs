use super::*;
use crate::cards::builders::ConditionalEffectAst;
use crate::cards::builders::CounterActionAst;
use crate::cards::builders::DelayedEffectAst;
use crate::condition_antecedent::{
    ConditionAntecedentBinding, bind_condition_antecedent_in_effects,
    bind_condition_counter_antecedent_in_effects,
    bind_random_count_condition_antecedent_in_effects, predicate_object_filter_antecedent,
    predicate_source_counter_antecedent,
};

pub fn compile_delayed_trigger_spec(
    trigger: &TriggerSpec,
) -> Result<ironsmith_core::DelayedTriggerSpec, CardTextError> {
    match trigger {
        TriggerSpec::ConditionQualified {
            trigger,
            condition,
            surface,
        } => Ok(ironsmith_core::DelayedTriggerSpec::ConditionQualified {
            trigger: Box::new(compile_delayed_trigger_spec(trigger)?),
            condition: compile_condition_from_predicate_ast(
                condition,
                &mut EffectLoweringContext::new(),
                &None,
            )?,
            surface: surface.clone(),
        }),
        TriggerSpec::WithIntro { trigger, .. } => compile_delayed_trigger_spec(trigger),
        TriggerSpec::BeginningOfUpkeep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(player.clone()),
        ),
        TriggerSpec::BeginningOfDrawStep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(player.clone()),
        ),
        TriggerSpec::BeginningOfEndStep(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player.clone()),
        ),
        TriggerSpec::BeginningOfTheEndStep => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(PlayerFilter::Any),
        ),
        TriggerSpec::BeginningOfCombat(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::BeginningOfCombat(player.clone()),
        ),
        TriggerSpec::BeginningOfPrecombatMain(player) => {
            Ok(ironsmith_core::DelayedTriggerSpec::BeginningOfPrecombatMainPhase(player.clone()))
        }
        TriggerSpec::BeginningOfPostcombatMain { player, .. } => {
            Ok(ironsmith_core::DelayedTriggerSpec::BeginningOfPostcombatMainPhase(player.clone()))
        }
        TriggerSpec::ThisEntersBattlefield { .. } => {
            Ok(ironsmith_core::DelayedTriggerSpec::ThisEntersBattlefield)
        }
        TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface,
            subject_number,
            ..
        } => Ok(
            ironsmith_core::DelayedTriggerSpec::ThisEntersBattlefieldWithSurface {
                surface: surface.clone(),
                subject_number: *subject_number,
            },
        ),
        TriggerSpec::EntersBattlefield {
            filter,
            cause_filter,
            ..
        } => Ok(ironsmith_core::DelayedTriggerSpec::EntersBattlefield {
            filter: filter.clone(),
            cause_filter: cause_filter.clone(),
            count: ironsmith_core::trigger_model::CountMode::One,
            tapped: None,
        }),
        TriggerSpec::EntersBattlefieldOneOrMore {
            filter,
            cause_filter,
            ..
        } => Ok(ironsmith_core::DelayedTriggerSpec::EntersBattlefield {
            filter: filter.clone(),
            cause_filter: cause_filter.clone(),
            count: ironsmith_core::trigger_model::CountMode::OneOrMore,
            tapped: None,
        }),
        TriggerSpec::EntersBattlefieldTapped {
            filter,
            cause_filter,
        } => Ok(ironsmith_core::DelayedTriggerSpec::EntersBattlefield {
            filter: filter.clone(),
            cause_filter: cause_filter.clone(),
            count: ironsmith_core::trigger_model::CountMode::One,
            tapped: Some(true),
        }),
        TriggerSpec::EntersBattlefieldUntapped {
            filter,
            cause_filter,
        } => Ok(ironsmith_core::DelayedTriggerSpec::EntersBattlefield {
            filter: filter.clone(),
            cause_filter: cause_filter.clone(),
            count: ironsmith_core::trigger_model::CountMode::One,
            tapped: Some(false),
        }),
        TriggerSpec::IsDealtDamage(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Object(filter.clone())),
        ),
        TriggerSpec::IsDealtCombatDamage(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Object(filter.clone())),
        ),
        TriggerSpec::PutIntoGraveyard(filter) | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::PutIntoGraveyard(
                filter.clone(),
            ))
        }
        TriggerSpec::PutIntoGraveyardFromZone {
            filter,
            from,
            one_or_more,
            cause_filter: None,
        } => Ok(
            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyardFromZone {
                filter: filter.clone(),
                from: *from,
                one_or_more: *one_or_more,
            },
        ),
        TriggerSpec::ThisDies => Ok(ironsmith_core::DelayedTriggerSpec::ThisDies),
        TriggerSpec::ThisLeavesBattlefield | TriggerSpec::ThisLeavesBattlefieldWithSurface(_) => {
            Ok(ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield)
        }
        TriggerSpec::ThisAttacksAndIsntBlocked => {
            Ok(ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked)
        }
        TriggerSpec::ThisBlocksObject {
            filter,
            min_blocked_objects,
        } => Ok(ironsmith_core::DelayedTriggerSpec::ThisBlocksObject {
            filter: filter.clone(),
            min_blocked_objects: *min_blocked_objects,
        }),
        TriggerSpec::ThisBecomesBlockedByObject(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::ThisBecomesBlockedByObject(filter.clone()))
        }
        TriggerSpec::Attacks(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::Attacks(filter.clone()))
        }
        TriggerSpec::AttacksYouOrPlaneswalkerYouControl(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::AttacksYou(filter.clone()),
        ),
        TriggerSpec::PlayerTapsForMana { player, filter } => {
            Ok(ironsmith_core::DelayedTriggerSpec::PlayerTapsForMana {
                player: player.clone(),
                filter: filter.clone(),
            })
        }
        TriggerSpec::AttacksAndIsntBlocked(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::AttacksAndIsntBlocked(filter.clone()),
        ),
        TriggerSpec::AttacksOneOrMore(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::AttacksOneOrMore(filter.clone()),
        ),
        TriggerSpec::Blocks(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::Blocks(filter.clone()))
        }
        TriggerSpec::BlocksOneOrMore(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::BlocksOneOrMore(filter.clone()),
        ),
        TriggerSpec::BecomesBlocked(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::BecomesBlocked(filter.clone()),
        ),
        TriggerSpec::LeavesBattlefield(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::LeavesBattlefield(filter.clone()),
        ),
        TriggerSpec::Dies(filter) | TriggerSpec::DiesOneOrMore(filter) => {
            Ok(ironsmith_core::DelayedTriggerSpec::Dies(filter.clone()))
        }
        TriggerSpec::PermanentBecomesTapped(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::PermanentBecomesTapped(filter.clone()),
        ),
        TriggerSpec::DealsCombatDamage(filter) => Ok(
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(filter.clone()),
        ),
        TriggerSpec::DealsCombatDamageTo { source, target } => {
            Ok(ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                source: source.clone(),
                target: target.clone(),
            })
        }
        TriggerSpec::DealsCombatDamageToPlayer { source, player } => Ok(
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                source: source.clone(),
                player: player.clone(),
            },
        ),
        TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
            source,
            player,
            each_damaged_player,
        } => Ok(
            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                source: source.clone(),
                player: player.clone(),
                each_damaged_player: *each_damaged_player,
            },
        ),
        TriggerSpec::SpellCast {
            filter,
            mana_source_filter,
            caster,
            timing,
            during_turn,
            min_spells_this_turn,
            exact_spells_this_turn,
            from_not_hand,
        } => {
            if mana_source_filter.is_some() {
                return Err(CardTextError::InvariantViolation(
                    "delayed spell-cast triggers do not yet carry mana-source provenance"
                        .to_string(),
                ));
            }
            Ok(ironsmith_core::DelayedTriggerSpec::SpellCast {
                filter: filter.clone(),
                caster: caster.clone(),
                timing: *timing,
                during_turn: during_turn.clone(),
                min_spells_this_turn: *min_spells_this_turn,
                exact_spells_this_turn: *exact_spells_this_turn,
                from_not_hand: *from_not_hand,
                first_spell_of_game: false,
            })
        }
        TriggerSpec::PlayerPlaysLand { player, filter } => {
            Ok(ironsmith_core::DelayedTriggerSpec::PlayerPlaysLand {
                player: player.clone(),
                filter: filter.clone(),
            })
        }
        TriggerSpec::YouDrawCard => Ok(ironsmith_core::DelayedTriggerSpec::PlayerDrawsCard(
            PlayerFilter::You,
        )),
        TriggerSpec::PlayerDrawsCard(player) => Ok(
            ironsmith_core::DelayedTriggerSpec::PlayerDrawsCard(player.clone()),
        ),
        TriggerSpec::AbilityActivated {
            activator,
            filter,
            non_mana_only,
            loyalty_only,
            activation_cost_has_tap,
        } => Ok(ironsmith_core::DelayedTriggerSpec::AbilityActivated {
            activator: activator.clone(),
            filter: filter.clone(),
            non_mana_only: *non_mana_only,
            loyalty_only: *loyalty_only,
            activation_cost_has_tap: *activation_cost_has_tap,
        }),
        TriggerSpec::Either(left, right) => Ok(ironsmith_core::DelayedTriggerSpec::Either(
            Box::new(compile_delayed_trigger_spec(left)?),
            Box::new(compile_delayed_trigger_spec(right)?),
        )),
        other => Err(CardTextError::ParseError(format!(
            "unsupported delayed trigger spec: {other:?}"
        ))),
    }
}

/// Turn an authored entry-time counter on the next creature spell into a
/// replacement tied to that spell's stable identity. A counter put directly
/// on the stack object would be cleared by the later zone change and is not
/// equivalent to "that creature enters with an additional counter."
fn fuse_next_cast_entry_counter_body(
    trigger: &TriggerSpec,
    one_shot: bool,
    effects: &mut [Effect],
) {
    let TriggerSpec::SpellCast {
        filter: Some(spell_filter),
        mana_source_filter: None,
        caster: PlayerFilter::You,
        timing: None,
        during_turn: None,
        min_spells_this_turn: None,
        exact_spells_this_turn: None,
        from_not_hand: false,
    } = trigger
    else {
        return;
    };
    if !one_shot
        || spell_filter.zone != Some(Zone::Stack)
        || spell_filter.stack_kind != Some(crate::filter::StackObjectKind::Spell)
        || !spell_filter
            .card_types
            .contains(&crate::types::CardType::Creature)
    {
        return;
    }
    let [tag_effect, counter_effect] = effects else {
        return;
    };
    let Some(tag_triggering) =
        tag_effect.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
    else {
        return;
    };
    let Some(counter) = counter_effect.downcast_ref::<crate::effects::PutCountersEffect>() else {
        return;
    };
    if counter.distributed
        || counter.target_count.is_some()
        || !counter
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
        || !counter
            .amount
            .has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
        || !matches!(
            counter.target.base(),
            ChooseSpec::Tagged(tag) if tag == &tag_triggering.tag
        )
    {
        return;
    }

    let mut entry_filter = spell_filter.clone();
    entry_filter.zone = Some(Zone::Battlefield);
    entry_filter.stack_kind = None;
    entry_filter.has_mana_cost = false;
    let replacement = crate::effects::RegisterNextBatchEnterWithCountersEffect::new(
        entry_filter,
        counter.counter_type,
        counter.amount.clone(),
    )
    .same_stable_id_as_tag(tag_triggering.tag.clone());
    *counter_effect = Effect::new(replacement);
}

/// Delayed events whose own object a demonstrative in the delayed body cannot
/// name: the source itself ("When this artifact leaves the battlefield this
/// turn, destroy that creature") or a whole attack by a player ("Whenever you
/// attack this turn, Jaya deals damage ... to that creature").
fn delayed_trigger_event_object_is_source(trigger: &TriggerSpec) -> bool {
    matches!(
        trigger_without_intro(trigger),
        TriggerSpec::ThisLeavesBattlefield
            | TriggerSpec::ThisLeavesBattlefieldWithSurface(_)
            | TriggerSpec::ThisDies
            | TriggerSpec::ThisDiesOrIsExiled
            | TriggerSpec::ThisDiesOrIsExiledWithSurface(_)
            | TriggerSpec::AttacksOneOrMore(_)
            | TriggerSpec::PlayerAttacksOneOrMore { .. }
    )
}

/// A delayed body that measures the registering instruction's result ("for
/// each creature card put into your graveyard this way") reads that result's
/// tagged objects through the registration alias. Bind the alias to the
/// result tag current where the delayed trigger is scheduled; the delayed
/// trigger captures that tag's objects when it is registered.
fn delayed_registration_result_imports(
    effects: &[EffectAst],
    ctx: &EffectLoweringContext,
) -> Option<ReferenceImports> {
    use ironsmith_core::tag::TagKeyWalk;
    let alias = crate::reference_helpers::DELAYED_REGISTRATION_RESULT_ALIAS;
    let mut references_alias = false;
    for effect in effects {
        effect.for_each_tag_key(&mut |key| {
            references_alias |= key.as_str() == alias;
        });
    }
    if !references_alias {
        return None;
    }
    let result_tag = ctx.last_object_tag.clone()?;
    Some(ReferenceImports {
        snapshot_tag_aliases: vec![(TagKey::from(alias), result_tag.into())],
        ..Default::default()
    })
}

fn compile_delayed_effects_preserving_outer_context(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    compile_delayed_effects_preserving_outer_context_with_event_value(effects, ctx, false)
}

fn compile_delayed_effects_preserving_outer_context_with_event_value(
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
    allow_event_value: bool,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let saved_frame = ctx.lowering_frame();
    let mut delayed_frame = saved_frame.clone();
    delayed_frame.last_effect_id = None;
    delayed_frame.allow_life_event_value = allow_event_value;
    let mut id_gen = ctx.id_gen_context();
    let (compiled, _delayed_choices, _delayed_frame) =
        compile_effects_with_explicit_frame(effects, &mut id_gen, delayed_frame)?;
    // A delayed ability chooses its own targets when it is put on the stack.
    // Its body can read the registering ability's references, but new targets
    // and references inside that body must not escape into the outer ability.
    ctx.apply_id_gen_context(id_gen);
    ctx.apply_lowering_frame(saved_frame);
    Ok((compiled, Vec::new()))
}

/// A next-end-step payload can consume the amount actually prevented by the
/// shield registered immediately before it. Reference resolution first pins
/// the authored "prevented this way" metric to that producer so unrelated
/// results cannot capture it. The delayed trigger then supplies the shield's
/// accumulated amount as its event value when it resolves.
fn effect_has_put_counters(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(subject_verb)
            if matches!(
                subject_verb.action,
                SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { .. })
            )
    ) {
        return true;
    }
    let mut found = false;
    for_each_nested_effects(effect, false, |nested| {
        found |= nested.iter().any(effect_has_put_counters);
    });
    found
}

fn bind_event_amount_value(value: &Value, count: &Value) -> Option<Value> {
    match value {
        // Neither a triggering amount nor an outer instruction's outcome is
        // available to a delayed ability when it resolves.
        Value::EventValue(EventValueSpec::Amount) | Value::EffectValue(_) => Some(count.clone()),
        Value::SurfaceHinted { value, hints } => Some(Value::SurfaceHinted {
            value: Box::new(bind_event_amount_value(value, count)?),
            hints: hints.clone(),
        }),
        _ => None,
    }
}

/// Re-point a delayed body's bare "that many" (no triggering amount exists
/// for a phase trigger) at the scheduling instruction's tagged objects.
fn bind_delayed_event_amount(effect: &Effect, count: &Value) -> Effect {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let mut sequence = sequence.clone();
        sequence.effects = sequence
            .effects
            .iter()
            .map(|child| bind_delayed_event_amount(child, count))
            .collect();
        return Effect::new(sequence);
    }
    if let Some(consult) = effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()
        && let crate::effects::ConsultTopOfLibraryStopRule::MatchCount(value) = &consult.stop_rule
        && let Some(bound) = bind_event_amount_value(value, count)
    {
        let mut consult = consult.clone();
        consult.stop_rule = crate::effects::ConsultTopOfLibraryStopRule::MatchCount(bound);
        return Effect::new(consult);
    }
    effect.clone()
}

fn replace_delayed_prior_prevention_amounts(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(subject_verb) = effect
        && let SubjectVerbActionAst::Counters(CounterActionAst::PutCounters { count, .. }) =
            &mut subject_verb.action
        && matches!(
            count.unhinted(),
            Value::PendingPriorEffectMetric(query) | Value::PriorEffectMetric { query, .. }
                if query.action == Some(ironsmith_core::PriorEffectAction::Prevented)
        )
    {
        let hints = count.surface_hints().to_vec();
        *count = Value::EventValue(EventValueSpec::Amount).with_surface_hints(hints);
    }
    for_each_nested_effects_mut(effect, false, |nested| {
        for nested_effect in nested {
            replace_delayed_prior_prevention_amounts(nested_effect);
        }
    });
}

fn split_delayed_prepayment(
    effects: Vec<Effect>,
) -> (Vec<Effect>, Option<(PlayerFilter, crate::cost::TotalCost)>) {
    let [effect] = effects.as_slice() else {
        return (effects, None);
    };
    let Some(unless) = effect.downcast_ref::<crate::effects::UnlessPaysEffect<Effect>>() else {
        return (effects, None);
    };
    if unless.leading_surface || !unless.before_delayed_step {
        return (effects, None);
    }
    (
        unless.effects.clone(),
        Some((unless.player.clone(), unless.cost.clone())),
    )
}

fn set_filter_tag_relation(
    filter: &mut ObjectFilter,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) {
    for constraint in &mut filter.tagged_constraints {
        if constraint.tag.as_str() == tag && constraint.relation == from {
            constraint.relation = to;
        }
    }
}

fn set_choose_spec_tag_relation(
    spec: &mut ChooseSpec,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            set_choose_spec_tag_relation(spec, tag, from, to);
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            set_filter_tag_relation(filter, tag, from, to);
        }
        _ => {}
    }
}

fn set_effect_tag_relation(
    effect: Effect,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) -> Effect {
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Effect::new(tagged.with_effect(set_effect_tag_relation(
            (*tagged.effect).clone(),
            tag,
            from,
            to,
        )));
    }

    if let Some(conditional) = effect.downcast_ref::<crate::effects::ConditionalEffect>() {
        return Effect::new(
            crate::effects::ConditionalEffect::new(
                conditional.condition.clone(),
                set_effects_tag_relation(conditional.if_true.clone(), tag, from, to),
                set_effects_tag_relation(conditional.if_false.clone(), tag, from, to),
            )
            .with_surface(conditional.surface),
        );
    }

    if let Some(apply) = effect.downcast_ref::<crate::effects::ApplyContinuousEffect>() {
        let mut rewritten = apply.clone();
        if let crate::continuous::EffectTarget::Filter(filter) = &mut rewritten.target {
            set_filter_tag_relation(filter, tag, from, to);
        }
        if let Some(spec) = &mut rewritten.target_spec {
            set_choose_spec_tag_relation(spec, tag, from, to);
        }
        return Effect::new(rewritten);
    }

    effect
}

fn set_effects_tag_relation(
    effects: Vec<Effect>,
    tag: &str,
    from: TaggedOpbjectRelation,
    to: TaggedOpbjectRelation,
) -> Vec<Effect> {
    effects
        .into_iter()
        .map(|effect| set_effect_tag_relation(effect, tag, from, to))
        .collect()
}

/// "the creature an opponent controls" as a delayed-trigger subject: a
/// definite description that carries a controller or owner restriction
/// describes which announced target it names, rather than repeating the
/// most recent object result ("that creature").
fn definite_description_of_announced_target(filter: &ObjectFilter) -> bool {
    filter.demonstrative_antecedent_surface().is_some()
        && filter.tagged_constraints.is_empty()
        && !filter.source
        && (filter.controller.is_some() || filter.owner.is_some())
}

fn trigger_without_intro(trigger: &TriggerSpec) -> &TriggerSpec {
    match trigger {
        TriggerSpec::WithIntro { trigger, .. } => trigger_without_intro(trigger),
        trigger => trigger,
    }
}

fn apply_delayed_trigger_duration(
    delayed: crate::effects::ScheduleDelayedTriggerEffect,
    duration: &Until,
) -> Result<crate::effects::ScheduleDelayedTriggerEffect, CardTextError> {
    match duration {
        Until::Forever => Ok(delayed),
        Until::EndOfTurn => Ok(delayed.until_end_of_turn()),
        Until::EndOfCombat => Ok(delayed.until_end_of_combat()),
        Until::YourNextTurn => Ok(delayed.until_controller_next_turn()),
        other => Err(CardTextError::ParseError(format!(
            "unsupported delayed-trigger duration: {other:?}"
        ))),
    }
}

/// "whenever you play a land or cast a spell this way": the played land or
/// cast spell must be one of the cards the scheduling instruction made
/// playable. Bind that prior-object reference before the delayed trigger is
/// registered; the trigger captures the tagged cards it names.
fn resolve_play_or_cast_trigger_references(
    trigger: &TriggerSpec,
    refs: &ReferenceEnv,
) -> Result<TriggerSpec, CardTextError> {
    fn references_it(filter: &ObjectFilter) -> bool {
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        })
    }
    Ok(match trigger {
        TriggerSpec::WithIntro { intro, trigger } => TriggerSpec::WithIntro {
            intro: intro.clone(),
            trigger: Box::new(resolve_play_or_cast_trigger_references(trigger, refs)?),
        },
        TriggerSpec::Either(left, right) => TriggerSpec::Either(
            Box::new(resolve_play_or_cast_trigger_references(left, refs)?),
            Box::new(resolve_play_or_cast_trigger_references(right, refs)?),
        ),
        TriggerSpec::AnyOf(triggers) => TriggerSpec::AnyOf(
            triggers
                .iter()
                .map(|trigger| resolve_play_or_cast_trigger_references(trigger, refs))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        TriggerSpec::PlayerPlaysLand { player, filter } if references_it(filter) => {
            TriggerSpec::PlayerPlaysLand {
                player: player.clone(),
                filter: resolve_it_tag(filter, refs)?,
            }
        }
        TriggerSpec::SpellCast {
            filter: Some(filter),
            ..
        } if references_it(filter) => {
            let mut resolved = trigger.clone();
            if let TriggerSpec::SpellCast {
                filter: Some(filter),
                ..
            } = &mut resolved
            {
                *filter = resolve_it_tag(filter, refs)?;
            }
            resolved
        }
        trigger => trigger.clone(),
    })
}

fn compile_duration_scoped_delayed_trigger(
    trigger: &TriggerSpec,
    effects: &[EffectAst],
    one_shot: bool,
    duration: &Until,
    either_of_watched_objects: bool,
    while_any_tagged_object_in_zone: &Option<(TagKey, Zone)>,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    if let TriggerSpec::ConditionQualified {
        trigger,
        condition,
        surface,
    } = trigger_without_intro(trigger)
    {
        let (compiled, choices) = compile_duration_scoped_delayed_trigger(
            trigger,
            effects,
            one_shot,
            duration,
            either_of_watched_objects,
            while_any_tagged_object_in_zone,
            ctx,
        )?;
        let [effect] = compiled.as_slice() else {
            unreachable!("delayed registration is one effect");
        };
        let mut schedule = effect
            .downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>()
            .unwrap()
            .clone();
        schedule.trigger = ironsmith_core::DelayedTriggerSpec::ConditionQualified {
            trigger: Box::new(schedule.trigger),
            condition: compile_condition_from_predicate_ast(condition, ctx, &None)?,
            surface: surface.clone(),
        };
        return Ok((vec![Effect::new(schedule)], choices));
    }
    // A delayed trigger whose event names its own untagged object ("whenever
    // a creature becomes tapped, destroy it") supplies the antecedent for its
    // body. Only a trigger that watches a prior object ("whenever that
    // creature ...") or has no event object inherits the scheduling
    // instruction's object antecedent.
    // "whenever you play a land or cast a spell this way, its owner draws a
    // card" (Apple of Eden): each branch's played/cast object is the body's
    // antecedent.
    fn event_object_trigger_supplies_own_object(trigger: &TriggerSpec) -> bool {
        match trigger {
            TriggerSpec::Either(left, right) => {
                event_object_trigger_supplies_own_object(left)
                    && event_object_trigger_supplies_own_object(right)
            }
            TriggerSpec::AnyOf(triggers) => {
                !triggers.is_empty()
                    && triggers.iter().all(event_object_trigger_supplies_own_object)
            }
            // A cast spell or a played land is always a new object of its
            // own event; a tagged constraint ("... this way") only restricts
            // which cards qualify and never makes it a watched prior object.
            TriggerSpec::SpellCast { filter, .. } => {
                filter.as_ref().is_none_or(|filter| !filter.source)
            }
            TriggerSpec::PlayerPlaysLand { filter, .. } => !filter.source,
            _ => false,
        }
    }
    let trigger_supplies_own_object = matches!(
        trigger_without_intro(trigger),
        TriggerSpec::Either(..) | TriggerSpec::AnyOf(..)
    ) && event_object_trigger_supplies_own_object(trigger_without_intro(trigger))
        || {
        let event_filter = match trigger_without_intro(trigger) {
            TriggerSpec::PermanentBecomesTapped(filter)
            | TriggerSpec::Dies(filter)
            | TriggerSpec::Attacks(filter)
            | TriggerSpec::AttacksAndIsntBlocked(filter)
            | TriggerSpec::DealsCombatDamage(filter)
            | TriggerSpec::IsDealtDamage(filter)
            | TriggerSpec::IsDealtCombatDamage(filter)
            | TriggerSpec::LeavesBattlefield(filter)
            | TriggerSpec::Blocks(filter)
            | TriggerSpec::BecomesBlocked(filter)
            | TriggerSpec::PutIntoGraveyard(filter)
            | TriggerSpec::DealsCombatDamageToPlayer { source: filter, .. }
            | TriggerSpec::EntersBattlefield { filter, .. } => Some(filter),
            _ => None,
        };
        event_filter.is_some_and(|filter| {
            let mut references_tag = false;
            ironsmith_core::tag::TagKeyWalk::for_each_tag_key(filter, &mut |_| {
                references_tag = true
            });
            !filter.source && !references_tag
        })
    };
    let lowered = compile_trigger_effects_with_imports(
        Some(trigger),
        effects,
        &ReferenceImports {
            last_object_tag: if trigger_supplies_own_object {
                None
            } else {
                ctx.last_object_tag.clone()
            },
            ..Default::default()
        },
    )?;
    let delayed_effects = lowered.effects.to_vec();
    let refs = current_reference_env(ctx);
    let mut watched_tag = None;
    let mut watched_filter = None;
    let mut watch_ability_source = false;
    let mut watch_all_object_targets = false;

    let delayed_trigger = match trigger_without_intro(trigger) {
        TriggerSpec::Attacks(filter) => {
            let resolved = resolve_it_tag(filter, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                // Evaluate attack restrictions when the attack happens, not
                // while registering a watcher before attackers are declared.
                let mut event_filter = resolved;
                event_filter.tagged_constraints.clear();
                event_filter.source = true;
                ironsmith_core::DelayedTriggerSpec::Attacks(event_filter)
            } else {
                ironsmith_core::DelayedTriggerSpec::Attacks(resolved)
            }
        }
        TriggerSpec::Dies(filter) => {
            let resolved = resolve_it_tag(filter, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::Dies(ObjectFilter::source())
            } else {
                ironsmith_core::DelayedTriggerSpec::Dies(resolved)
            }
        }
        TriggerSpec::ThisLeavesBattlefield | TriggerSpec::ThisLeavesBattlefieldWithSurface(_) => {
            watch_ability_source = true;
            ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield
        }
        TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface: crate::target::SourceReferenceSurface::ThisPermanentType(surface),
            subject_number,
            origin_condition: None,
        } if surface == "that permanent" => {
            let prior_result_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot schedule 'that permanent enters' without a prior object result"
                        .to_string(),
                )
            })?;
            watched_tag = Some(prior_result_tag);
            ironsmith_core::DelayedTriggerSpec::ThisEntersBattlefieldWithSurface {
                surface: crate::target::SourceReferenceSurface::ThisPermanentType(surface.clone()),
                subject_number: *subject_number,
            }
        }
        TriggerSpec::PermanentBecomesTapped(filter) => {
            let resolved = resolve_it_tag(filter, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::PermanentBecomesTapped(ObjectFilter::source())
            } else {
                ironsmith_core::DelayedTriggerSpec::PermanentBecomesTapped(resolved)
            }
        }
        TriggerSpec::DealsCombatDamage(filter) => {
            let resolved = resolve_it_tag(filter, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(ObjectFilter::source())
            } else if either_of_watched_objects {
                watch_all_object_targets = true;
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(ObjectFilter::source())
            } else {
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(resolved)
            }
        }
        TriggerSpec::DealsCombatDamageTo { source, target } => {
            let resolved_source = resolve_it_tag(source, &refs)?;
            let resolved_target = resolve_it_tag(target, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved_source) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved_source);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                    source: ObjectFilter::source(),
                    target: resolved_target,
                }
            } else {
                watch_ability_source = resolved_source.source || resolved_target.source;
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                    source: resolved_source,
                    target: resolved_target,
                }
            }
        }
        TriggerSpec::DealsCombatDamageToPlayer { source, player } => {
            let resolved = resolve_it_tag(source, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                    source: ObjectFilter::source(),
                    player: player.clone(),
                }
            } else {
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                    source: resolved,
                    player: player.clone(),
                }
            }
        }
        TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
            source,
            player,
            each_damaged_player,
        } => {
            let resolved = resolve_it_tag(source, &refs)?;
            if let Some(tag) = watch_tag_from_filter(&resolved) {
                watched_tag = Some(tag);
                watched_filter = Some(resolved);
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                    source: ObjectFilter::source(),
                    player: player.clone(),
                    each_damaged_player: *each_damaged_player,
                }
            } else {
                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                    source: resolved,
                    player: player.clone(),
                    each_damaged_player: *each_damaged_player,
                }
            }
        }
        _ => compile_delayed_trigger_spec(&resolve_play_or_cast_trigger_references(
            trigger, &refs,
        )?)?,
    };

    let mut delayed = if let Some(tag) = watched_tag {
        crate::effects::ScheduleDelayedTriggerEffect::from_tag(
            tag,
            delayed_trigger,
            delayed_effects,
            one_shot,
            Vec::new(),
            PlayerFilter::You,
        )
    } else {
        crate::effects::ScheduleDelayedTriggerEffect::new(
            delayed_trigger,
            delayed_effects,
            one_shot,
            Vec::new(),
            PlayerFilter::You,
        )
    };
    if let Some(filter) = watched_filter {
        delayed = delayed.with_target_filter(filter);
    }
    if watch_ability_source {
        delayed = delayed.watch_ability_source();
    }
    if watch_all_object_targets {
        delayed = delayed.watch_all_object_targets();
    }
    if either_of_watched_objects {
        delayed = delayed.with_either_of_watched_objects_surface();
    }
    if let Some((tag, zone)) = while_any_tagged_object_in_zone {
        delayed = delayed.while_any_tagged_object_in_zone(resolve_it_tag_key(tag, &refs)?, *zone);
    }
    delayed = apply_delayed_trigger_duration(delayed, duration)?.with_leading_duration_surface();

    Ok((vec![Effect::new(delayed)], Vec::new()))
}

pub(super) fn try_compile_timing_and_control_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep { player, effects }) => {
            // A prevention amount reaches a delayed body only as a counter
            // count ("put a counter on it for each 1 damage prevented this
            // way"); a bare "that many" elsewhere names an earlier result.
            let reads_event_amount = effects
                .iter()
                .any(effect_references_prior_prevention_amount);
            let uses_prior_prevention_amount =
                reads_event_amount && effects.iter().any(effect_has_put_counters);
            let mut effects = effects.clone();
            if uses_prior_prevention_amount {
                for effect in &mut effects {
                    replace_delayed_prior_prevention_amounts(effect);
                }
            }
            let (mut delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context_with_event_value(
                    &effects,
                    ctx,
                    reads_event_amount,
                )?;
            // "Exile all creatures you control. At the beginning of the next
            // end step, reveal cards until you reveal that many creature
            // cards": the delayed ability has no triggering amount; "that
            // many" counts the objects the scheduling instruction tagged.
            if !uses_prior_prevention_amount
                && let Some(tag) = ctx.last_object_tag.clone()
            {
                let mut filter = ObjectFilter::default();
                filter.zone = None;
                filter.tagged_constraints.push(crate::filter::TaggedObjectConstraint {
                    tag: tag.clone(),
                    relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                });
                let count = Value::Count(filter);
                delayed_effects = delayed_effects
                    .iter()
                    .map(|effect| bind_delayed_event_amount(effect, &count))
                    .collect();
            }
            let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            );
            if uses_prior_prevention_amount {
                delayed = delayed.with_prior_prevention_event_value();
            }
            let effect = Effect::new(delayed);
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextCleanupStep { player, effects }) => {
            let (delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfNextCleanupStep(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUntapStep { player, effects }) => {
            let subject = LoweredSubject::resolve_affected_player(*player, ctx, true, true, true)?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    ironsmith_core::DelayedTriggerSpec::AsPermanentsUntap {
                        player: player_filter,
                        source_must_be_controlled: true,
                    },
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .watch_ability_source(),
            );
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextUpkeep { player, effects }) => {
            // "at the beginning of the next turn's upkeep" names no player,
            // so it must not displace the body's "that player" antecedent.
            let subject = LoweredSubject::resolve_affected_player(
                *player,
                ctx,
                true,
                true,
                !matches!(player, PlayerAst::Any),
            )?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            choices.extend(nested_choices);
            let (delayed_effects, prepayment) = split_delayed_prepayment(delayed_effects);
            let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(player_filter),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            )
            .starting_next_turn();
            if let Some((payer, cost)) = prepayment {
                delayed = delayed.unless_paid_before_trigger(payer, cost);
            }
            let effect = Effect::new(delayed);
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextDrawStep { player, effects }) => {
            let subject = LoweredSubject::resolve_affected_player(
                *player,
                ctx,
                true,
                true,
                !matches!(player, PlayerAst::Any),
            )?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            choices.extend(nested_choices);
            let (delayed_effects, prepayment) = split_delayed_prepayment(delayed_effects);
            let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(player_filter),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            )
            .starting_next_turn();
            if let Some((payer, cost)) = prepayment {
                delayed = delayed.unless_paid_before_trigger(payer, cost);
            }
            let effect = Effect::new(delayed);
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextMainPhase { player, effects }) => {
            let (delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfMainPhase(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextFirstMainPhase {
            player,
            effects,
        }) => {
            let (delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::BeginningOfPrecombatMainPhase(player.clone()),
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndStepOfExtraTurn {
            player,
            effects,
        }) => {
            let subject = LoweredSubject::resolve_affected_player(*player, ctx, true, true, true)?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (delayed_effects, nested_choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            choices.extend(nested_choices);
            let effect = Effect::new(
                crate::effects::ScheduleDelayedTriggerEffect::new(
                    ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(player_filter),
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .starting_next_turn(),
            );
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilEndOfCombat { effects }) => {
            let (delayed_effects, choices) =
                compile_delayed_effects_preserving_outer_context(effects, ctx)?;
            let effect = Effect::new(crate::effects::ScheduleDelayedTriggerEffect::new(
                ironsmith_core::DelayedTriggerSpec::EndOfCombat,
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            ));
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedTriggerForDuration {
            trigger,
            effects,
            one_shot,
            duration,
            either_of_watched_objects,
            while_any_tagged_object_in_zone,
        }) => {
            return compile_duration_scoped_delayed_trigger(
                trigger,
                effects,
                *one_shot,
                duration,
                *either_of_watched_objects,
                &while_any_tagged_object_in_zone
                    .as_ref()
                    .map(|(tag, zone)| (tag.key.clone(), *zone)),
                ctx,
            )
            .map(Some);
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedTriggerThisTurn {
            trigger,
            effects,
            one_shot,
            until_end_of_combat,
            attach_to_previous_ability,
        }) => {
            if matches!(
                trigger_without_intro(trigger),
                TriggerSpec::ConditionQualified { .. }
            ) {
                return compile_duration_scoped_delayed_trigger(
                    trigger,
                    effects,
                    *one_shot,
                    &if *until_end_of_combat {
                        Until::EndOfCombat
                    } else {
                        Until::EndOfTurn
                    },
                    false,
                    &None,
                    ctx,
                )
                .map(Some);
            }
            // "When this artifact leaves the battlefield this turn, destroy
            // that creature": when the delayed event's own object is the
            // source, a demonstrative in its body names the registering
            // ability's object (captured with the delayed trigger), never
            // the source that left.
            let (mut delayed_effects, _delayed_choices) =
                match ctx.last_object_tag.clone().filter(|_| {
                    delayed_trigger_event_object_is_source(trigger)
                }) {
                    Some(outer) => {
                        let lowered = compile_trigger_effects_with_imports(
                            Some(trigger),
                            effects,
                            &ReferenceImports {
                                last_object_tag: Some(outer.into()),
                                ..Default::default()
                            },
                        )?;
                        (lowered.effects.to_vec(), lowered.choices)
                    }
                    None => match delayed_registration_result_imports(effects, ctx) {
                        Some(imports) => {
                            let lowered = compile_trigger_effects_with_imports(
                                Some(trigger),
                                effects,
                                &imports,
                            )?;
                            (lowered.effects.to_vec(), lowered.choices)
                        }
                        None => compile_trigger_effects(Some(trigger), effects)?,
                    },
                };
            fuse_next_cast_entry_counter_body(trigger, *one_shot, &mut delayed_effects);
            let choices = Vec::new();
            match trigger {
                TriggerSpec::Dies(filter)
                    if *attach_to_previous_ability
                        && filter.dealt_damage_by_source_this_turn
                            == Some(ironsmith_core::DamagedBySource::ThisCreature) =>
                {
                    let watched_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                        CardTextError::ParseError(
                            "cannot schedule a damage-history death trigger without the prior creature target"
                                .to_string(),
                        )
                    })?;
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                        watched_tag.into(),
                        ironsmith_core::DelayedTriggerSpec::Dies(resolved_filter),
                        delayed_effects,
                        *one_shot,
                        Vec::new(),
                        PlayerFilter::You,
                    )
                    .until_end_of_turn();
                    (vec![Effect::new(delayed)], choices)
                }
                TriggerSpec::IsDealtDamage(filter) | TriggerSpec::IsDealtCombatDamage(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::IsDealtDamage(ChooseSpec::Source),
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::IsDealtDamage(
                                    ChooseSpec::Object(resolved_filter),
                                ),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::DealsCombatDamageToPlayer { source, player } => {
                    let resolved_filter = resolve_it_tag(source, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                                source: crate::target::ObjectFilter::source(),
                                player: player.clone(),
                            },
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let mut delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        if *until_end_of_combat {
                            delayed = delayed.until_end_of_combat();
                        }
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
                            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayer {
                                source: resolved_filter,
                                player: player.clone(),
                            },
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn();
                        if *until_end_of_combat {
                            delayed = delayed.until_end_of_combat();
                        }
                        (vec![Effect::new(delayed)], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyard(filter)
                | TriggerSpec::PutIntoGraveyardOneOrMore(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let watched_tag = watch_tag_from_filter(&resolved_filter).or_else(|| {
                        filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
                            .then(|| ctx.last_object_tag.clone())
                            .flatten()
                            .map(TagKey::from)
                    });
                    if let Some(watched_tag) = watched_tag {
                        let lowered = compile_trigger_effects_with_imports(
                            Some(trigger),
                            effects,
                            &ReferenceImports {
                                last_object_tag: Some(watched_tag.clone()),
                                ..Default::default()
                            },
                        )?;
                        let delayed_effects = lowered.effects.to_vec();
                        // The delayed matcher's source is the watched object ID.
                        // A tag alone can match later zone incarnations of the
                        // same card, which are no longer the watched object.
                        let mut event_filter = resolved_filter.clone();
                        event_filter.source = true;
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyard(event_filter),
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::PutIntoGraveyard(
                                    resolved_filter,
                                ),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::Dies(filter) | TriggerSpec::DiesOneOrMore(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let watched_tag = watch_tag_from_filter(&resolved_filter).or_else(|| {
                        filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
                            .then(|| ctx.last_object_tag.clone())
                            .flatten()
                            .map(TagKey::from)
                    });
                    if let Some(watched_tag) = watched_tag {
                        let lowered = compile_trigger_effects_with_imports(
                            Some(trigger),
                            effects,
                            &ReferenceImports {
                                last_object_tag: Some(watched_tag.clone()),
                                ..Default::default()
                            },
                        )?;
                        let delayed_effects = lowered.effects.to_vec();
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::ThisDies,
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::Dies(resolved_filter),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::LeavesBattlefield(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let watched_tag = watch_tag_from_filter(&resolved_filter).or_else(|| {
                        filter_references_tag(filter, crate::tag::CompilerReferenceTag::It.as_str())
                            .then(|| crate::tag::CompilerReferenceTag::Targeted0.bind())
                            .map(Into::into)
                    });
                    if let Some(watched_tag) = watched_tag {
                        let lowered = compile_trigger_effects_with_imports(
                            Some(trigger),
                            effects,
                            &ReferenceImports {
                                last_object_tag: Some(watched_tag.clone()),
                                ..Default::default()
                            },
                        )?;
                        let delayed_effects = lowered.effects.to_vec();
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield,
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        );
                        let delayed = delayed
                            .with_target_filter(resolved_filter)
                            .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::LeavesBattlefield(
                                    resolved_filter,
                                ),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::PutIntoGraveyardFromZone {
                    filter,
                    from,
                    one_or_more,
                    cause_filter: None,
                } => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            ironsmith_core::DelayedTriggerSpec::PutIntoGraveyardFromZone {
                                filter: resolved_filter,
                                from: *from,
                                one_or_more: *one_or_more,
                            },
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn(),
                    );
                    (vec![effect], choices)
                }
                TriggerSpec::ThisAttacksAndIsntBlocked => {
                    if let Some(target_tag) = ctx.last_object_tag.clone() {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            target_tag.clone().into(),
                            ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked,
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked,
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::AttacksAndIsntBlocked(filter) => {
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::ThisAttacksAndIsntBlocked,
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_filter)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::AttacksAndIsntBlocked(
                                    resolved_filter,
                                ),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::DealsCombatDamageTo { source, target } => {
                    let resolved_source = resolve_it_tag(source, &current_reference_env(ctx))?;
                    let resolved_target = resolve_it_tag(target, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_source) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                                source: crate::target::ObjectFilter::source(),
                                target: resolved_target,
                            },
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_source)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::DealsCombatDamageTo {
                                    source: resolved_source,
                                    target: resolved_target,
                                },
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::DealsCombatDamage(filter) => {
                    // "Target creature ... Whenever that creature deals combat
                    // damage this turn, ...": watch the referenced object
                    // itself rather than any creature carrying the tag.
                    let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_filter) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(
                                crate::target::ObjectFilter::source(),
                            ),
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_filter)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                ironsmith_core::DelayedTriggerSpec::DealsCombatDamage(
                                    resolved_filter,
                                ),
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                TriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                    source,
                    player,
                    each_damaged_player,
                } => {
                    let resolved_source = resolve_it_tag(source, &current_reference_env(ctx))?;
                    let trigger =
                        ironsmith_core::DelayedTriggerSpec::DealsCombatDamageToPlayerOneOrMore {
                            source: resolved_source.clone(),
                            player: player.clone(),
                            each_damaged_player: *each_damaged_player,
                        };
                    if let Some(watched_tag) = watch_tag_from_filter(&resolved_source) {
                        let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                            watched_tag.clone(),
                            trigger,
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .with_target_filter(resolved_source)
                        .until_end_of_turn();
                        (vec![Effect::new(delayed)], choices)
                    } else {
                        let effect = Effect::new(
                            crate::effects::ScheduleDelayedTriggerEffect::new(
                                trigger,
                                delayed_effects,
                                *one_shot,
                                Vec::new(),
                                PlayerFilter::You,
                            )
                            .until_end_of_turn(),
                        );
                        (vec![effect], choices)
                    }
                }
                _ => {
                    let effect = Effect::new(
                        crate::effects::ScheduleDelayedTriggerEffect::new(
                            compile_delayed_trigger_spec(trigger)?,
                            delayed_effects,
                            *one_shot,
                            Vec::new(),
                            PlayerFilter::You,
                        )
                        .until_end_of_turn(),
                    );
                    (vec![effect], choices)
                }
            }
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectDiesThisTurn {
            filter,
            effects,
        }) => {
            let target_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot schedule 'dies this turn' trigger without prior object context"
                        .to_string(),
                )
            })?;
            let previous_last = ctx.last_object_tag.clone();
            ctx.last_object_tag =
                Some((crate::tag::CompilerReferenceTag::Triggering.bind()).into());
            let compiled = compile_effects_preserving_last_effect(effects, ctx);
            ctx.last_object_tag = previous_last;
            let (delayed_effects, choices) = compiled?;
            if let Some(filter) = filter
                && definite_description_of_announced_target(filter)
            {
                // "When the creature an opponent controls dies this turn"
                // after "... fights target creature an opponent controls":
                // the definite description names the announced target it
                // describes, not the most recent object result.
                let delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
                    ironsmith_core::DelayedTriggerSpec::ThisDies,
                    delayed_effects,
                    true,
                    Vec::new(),
                    PlayerFilter::You,
                )
                .watch_all_object_targets()
                .with_target_filter(resolve_it_tag(filter, &current_reference_env(ctx))?)
                .until_end_of_turn();
                return Ok(Some((vec![Effect::new(delayed)], choices)));
            }
            let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                target_tag.clone().into(),
                ironsmith_core::DelayedTriggerSpec::ThisDies,
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            )
            .until_end_of_turn();
            if let Some(filter) = filter {
                delayed = delayed
                    .with_target_filter(resolve_it_tag(filter, &current_reference_env(ctx))?);
            }
            let effect = Effect::new(delayed);
            (vec![effect], choices)
        }
        EffectAst::Delayed(DelayedEffectAst::DelayedWhenLastObjectLeavesBattlefield {
            filter,
            effects,
            watched_object_is_source,
        }) => {
            let target_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "cannot schedule leaves-the-battlefield trigger without prior object context"
                        .to_string(),
                )
            })?;
            let previous_last = ctx.last_object_tag.clone();
            ctx.last_object_tag =
                Some((crate::tag::CompilerReferenceTag::Triggering.bind()).into());
            let compiled = compile_effects_preserving_last_effect(effects, ctx);
            ctx.last_object_tag = previous_last;
            let (mut delayed_effects, choices) = compiled?;
            if *watched_object_is_source {
                // "When it leaves the battlefield, it deals ...": the watched
                // object, as it last existed, is the damage source
                // (CR 608.2h); the delayed ability's controller is still the
                // scheduling ability's controller.
                let source_tag: crate::tag::TagKey = target_tag.clone().into();
                delayed_effects = delayed_effects
                    .into_iter()
                    .map(|effect| {
                        Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                            ChooseSpec::Tagged(source_tag.clone()),
                            effect,
                        ))
                    })
                    .collect();
            }

            let mut watched_filter = filter.clone();
            watched_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: target_tag.clone().into(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            let delayed = crate::effects::ScheduleDelayedTriggerEffect::from_tag(
                target_tag.into(),
                ironsmith_core::DelayedTriggerSpec::ThisLeavesBattlefield,
                delayed_effects,
                true,
                Vec::new(),
                PlayerFilter::You,
            )
            .with_target_filter(watched_filter);
            (vec![Effect::new(delayed)], choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

pub(super) fn try_compile_stack_and_condition_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult {
            condition,
            predicate,
            effects,
        }) => {
            // A correlated follow-up outside a participant loop needs the
            // result's player partition to bind its accepting participant.
            // This is reference context, not an announced target: damage to
            // "that player" deliberately exposes no target requirement.
            // "Otherwise" tests the complete antecedent instruction. An
            // earlier participant-bound follow-up may leave its reference
            // context here; it must not turn that collective fallback into
            // one action for every participant who declined.
            let per_player_result = !ctx.iterated_player
                && ctx.last_player_filter == Some(PlayerFilter::IteratedPlayer)
                && !matches!(predicate, IfResultPredicate::Otherwise);
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(*condition);
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(predicate.clone());
            let effect = Effect::new(
                crate::effects::IfEffect::if_then(*condition, predicate, inner_effects)
                    .with_per_player_result(per_player_result),
            );
            (vec![effect], inner_choices)
        }
        EffectAst::Conditionals(ConditionalEffectAst::ResolvedWhenResult {
            condition,
            predicate,
            effects,
        }) => {
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(*condition);
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(predicate.clone());
            let effect =
                Effect::reflexive_trigger(*condition, predicate, inner_effects, inner_choices);
            (vec![effect], Vec::new())
        }
        EffectAst::Conditionals(ConditionalEffectAst::TrailingIf { predicate, effects }) => {
            let predicate = predicate.clone();
            let (mut compiled, choices) =
                compile_conditional_ast(&predicate, effects, &[], true, ctx)?;
            let Some(lowered_conditional_effect) = compiled.pop() else {
                return Err(CardTextError::ParseError(
                    "trailing-if condition lowered without an effect".to_string(),
                ));
            };
            let Some(lowered_conditional) =
                lowered_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
            else {
                return Err(CardTextError::ParseError(
                    "trailing-if condition did not lower to a conditional".to_string(),
                ));
            };
            compiled.push(Effect::new(
                crate::effects::ConditionalEffect::new(
                    lowered_conditional.condition.clone(),
                    lowered_conditional.if_true.clone(),
                    lowered_conditional.if_false.clone(),
                )
                .with_surface(ironsmith_core::ConditionalSurface::TrailingIf),
            ));
            (compiled, choices)
        }
        EffectAst::Conditionals(ConditionalEffectAst::TrailingUnless { predicate, effects }) => {
            let predicate = PredicateAst::Not(Box::new(predicate.clone()));
            let (mut compiled, choices) =
                compile_conditional_ast(&predicate, effects, &[], true, ctx)?;
            let Some(lowered_conditional_effect) = compiled.pop() else {
                return Err(CardTextError::ParseError(
                    "trailing-unless condition lowered without an effect".to_string(),
                ));
            };
            let Some(lowered_conditional) =
                lowered_conditional_effect.downcast_ref::<crate::effects::ConditionalEffect>()
            else {
                return Err(CardTextError::ParseError(
                    "trailing-unless condition did not lower to a conditional".to_string(),
                ));
            };
            compiled.push(Effect::new(
                crate::effects::ConditionalEffect::new(
                    lowered_conditional.condition.clone(),
                    lowered_conditional.if_true.clone(),
                    lowered_conditional.if_false.clone(),
                )
                .with_surface(ironsmith_core::ConditionalSurface::TrailingUnless),
            ));
            (compiled, choices)
        }
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        }) => compile_conditional_ast(predicate, if_true, if_false, false, ctx)?,
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

/// Lower a state conditional. `trailing` marks a condition written after
/// its consequence ("Destroy target creature if it's tapped", "Put a counter
/// on that creature if the exiled creature was a Thrull"): references in the
/// consequence were written before the condition's subject, so they keep
/// their own antecedents rather than binding to that subject, while a
/// trailing `it` may still name a target the consequence announced.
fn compile_conditional_ast(
    predicate: &PredicateAst,
    if_true: &[EffectAst],
    if_false: &[EffectAst],
    trailing: bool,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let mut effective_if_true = if_true.to_vec();
    let predicate_names_explicit_subject = matches!(
        predicate,
        PredicateAst::TaggedMatches(tag, _)
            if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
    );
    if let Some(antecedent) = predicate_object_filter_antecedent(predicate)
        && !(trailing && predicate_names_explicit_subject)
    {
        bind_condition_antecedent_in_effects(
            &mut effective_if_true,
            &antecedent,
            ConditionAntecedentBinding::IncludeRandomWithCountObjects,
        );
    }
    bind_random_count_condition_antecedent_in_effects(&mut effective_if_true, predicate);
    if let Some(counter_type) = predicate_source_counter_antecedent(predicate) {
        bind_condition_counter_antecedent_in_effects(&mut effective_if_true, counter_type);
    }
    let saved_last_tag = ctx.last_object_tag.clone();
    // The predicate is evaluated before either branch runs, so it
    // reads the player context from before the branches.
    let saved_last_player = ctx.last_player_filter.clone();
    let saved_source_object_antecedent = ctx.source_object_antecedent;
    ctx.source_object_antecedent |= predicate.establishes_source_object_antecedent();
    let (true_effects, true_choices) = compile_effects(&effective_if_true, ctx)?;
    let true_last_tag = ctx.last_object_tag.clone();
    ctx.last_object_tag = saved_last_tag.clone();
    ctx.source_object_antecedent =
        saved_source_object_antecedent || predicate.establishes_source_object_antecedent();
    let (false_effects, false_choices) = compile_effects(if_false, ctx)?;
    ctx.source_object_antecedent = saved_source_object_antecedent;
    let predicate_references_it = predicate_uses_implicit_object_reference(predicate)
        || predicate_references_tag(
            predicate,
            crate::tag::CompilerReferenceTag::It.as_str(),
        );

    // A leading condition's `it` precedes every target its consequence
    // announces; with an established source antecedent it names the source
    // ("When you do, if it has four or more quest counters on it, put a
    // +1/+1 counter on target creature you control").
    // A trailing condition's `it` ("Exile target creature if its power ...")
    // names the target its own consequence announces, even when an earlier
    // antecedent (an additional-cost reveal) is still in scope.
    let antecedent_choice = if predicate_references_it
        && (trailing || (saved_last_tag.is_none() && !saved_source_object_antecedent))
    {
        let mut antecedent_choice = None;
        for choice in true_choices.iter().chain(false_choices.iter()) {
            if choice.is_target() && choose_spec_targets_object(choice) {
                antecedent_choice = Some(choice.clone());
                break;
            }
        }
        antecedent_choice
    } else {
        None
    };

    let mut condition_reference_tag = saved_last_tag.clone();
    let mut prelude = Vec::new();
    if let Some(choice) = antecedent_choice.clone() {
        let tag = if let Some(existing) = tagged_alias_for_choice(&true_effects, &choice) {
            existing
        } else {
            ctx.next_tag("targeted")
        };
        prelude.push(
            Effect::new(crate::effects::TargetOnlyEffect::new(choice)).tag(tag.clone()),
        );
        condition_reference_tag = Some(tag);
    }

    let original_last_tag = ctx.last_object_tag.clone();
    ctx.last_object_tag = condition_reference_tag.clone().or(saved_last_tag.clone());
    let original_source_object_antecedent = ctx.source_object_antecedent;
    if ctx.last_object_tag.is_none()
        && antecedent_choice.is_none()
        && predicate_uses_implicit_object_reference(predicate)
    {
        ctx.source_object_antecedent = true;
    }
    // A predicate naming the declared target player ("if target
    // opponent controls more lands than you") reads it from before the
    // branches, which may have moved the player context elsewhere.
    let predicate_names_target_player = matches!(
        predicate,
        PredicateAst::Player(
            crate::cards::builders::PlayerPredicateAst::PlayerControlsMoreThanYou {
                player: PlayerAst::Target | PlayerAst::TargetOpponent,
                ..
            }
        )
    );
    // A leading condition ("Then if that player has more cards in hand than
    // you, return ...") likewise reads its "that player" from the discourse
    // before the branches; a branch's own player (the returned card's owner)
    // must not become the condition's antecedent. A trailing condition may
    // still name a player its consequence introduced.
    let branch_last_player = (predicate_names_target_player || !trailing)
        .then(|| std::mem::replace(&mut ctx.last_player_filter, saved_last_player));
    let condition =
        compile_condition_from_predicate_ast(predicate, ctx, &condition_reference_tag)?;
    if let Some(branch_last_player) = branch_last_player {
        ctx.last_player_filter = branch_last_player;
    }
    ctx.last_object_tag = original_last_tag;
    ctx.source_object_antecedent = original_source_object_antecedent;

    let true_effects = if matches!(predicate, PredicateAst::ItIsSoulbondPaired)
        && let Some(reference_tag) = condition_reference_tag.as_ref()
    {
        set_effects_tag_relation(
            true_effects,
            reference_tag.as_str(),
            TaggedOpbjectRelation::IsTaggedObject,
            TaggedOpbjectRelation::SoulbondPartnerOfTagged,
        )
    } else {
        true_effects
    };

    let conditional = if false_effects.is_empty() {
        Effect::conditional_only(condition, true_effects)
    } else {
        Effect::conditional(condition, true_effects, false_effects)
    };
    prelude.push(conditional);

    if let Some(reference_tag) = condition_reference_tag {
        ctx.last_object_tag = Some(reference_tag);
    } else if if_false.is_empty() {
        ctx.last_object_tag = true_last_tag.clone().or(saved_last_tag.clone());
    } else {
        ctx.last_object_tag = saved_last_tag.clone();
    }

    let mut choices = true_choices;
    for choice in false_choices {
        push_choice(&mut choices, choice);
    }
    if let Some(choice) = antecedent_choice {
        push_choice(&mut choices, choice);
    }
    Ok((prelude, choices))
}

fn predicate_uses_implicit_object_reference(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::ItIsLandCard
        | PredicateAst::ItIsSoulbondPaired
        | PredicateAst::ItMatches(_)
        | PredicateAst::ItMatchedLastKnown(_)
        | PredicateAst::TargetMatches(_) => true,
        PredicateAst::Not(inner) => predicate_uses_implicit_object_reference(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_uses_implicit_object_reference(left)
                || predicate_uses_implicit_object_reference(right)
        }
        _ => false,
    }
}

#[cfg(test)]
mod collective_otherwise_scope_tests {
    use super::*;
    use crate::cards::builders::LifeResourceActionAst;

    fn lower_followup(predicate: IfResultPredicate, in_player_loop: bool) -> crate::effect::Effect {
        let mut ctx=EffectLoweringContext::new();
        ctx.last_player_filter=Some(PlayerFilter::IteratedPlayer);
        ctx.iterated_player=in_player_loop;
        let ast=EffectAst::Conditionals(ConditionalEffectAst::ResolvedIfResult {
            condition:crate::effect::EffectId(77),predicate,
            effects:vec![EffectAst::subject_verb(SubjectVerbRoleAst::AffectedPlayer,PlayerAst::You,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count:Value::Fixed(1) }))],
        });
        let (mut effects,choices)=try_compile_stack_and_condition_effect(&ast,&mut ctx).unwrap().unwrap();
        assert!(choices.is_empty());assert_eq!(effects.len(),1);
        effects.remove(0)
    }

    #[test]
    fn collective_otherwise_does_not_inherit_participant_partition() {
        let effect=lower_followup(IfResultPredicate::Otherwise,false);
        let condition=effect.as_if_effect().unwrap();
        assert!(!condition.per_player_result);
        assert_eq!(condition.condition,crate::effect::EffectId(77));
        assert_eq!(condition.predicate,EffectPredicate::DidNotHappen);
    }

    #[test]
    fn explicit_participant_conditions_keep_partition_binding() {
        for predicate in [IfResultPredicate::Did,IfResultPredicate::DidNot,IfResultPredicate::ExplicitDidNot] {
            assert!(lower_followup(predicate,false).as_if_effect().unwrap().per_player_result);
        }
    }

    #[test]
    fn condition_inside_participant_loop_does_not_repeat_all_participants() {
        for predicate in [IfResultPredicate::Did,IfResultPredicate::DidNot,IfResultPredicate::Otherwise] {
            assert!(!lower_followup(predicate,true).as_if_effect().unwrap().per_player_result);
        }
    }
}

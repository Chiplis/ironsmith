use super::*;

fn normalize_unless_cost_for_payer(cost: crate::cost::TotalCost) -> crate::cost::TotalCost {
    cost.try_map(|component| {
        let component = match component {
            crate::costs::Cost::Sacrifice(mut filter) => {
                bind_relative_iterated_player_filters_to_chooser(&mut filter, &PlayerFilter::You);
                crate::costs::Cost::Sacrifice(filter)
            }
            crate::costs::Cost::DynamicMana(mut dynamic) => {
                for value in [
                    &mut dynamic.x_value,
                    &mut dynamic.additional_generic,
                    &mut dynamic.multiplier,
                ] {
                    if let Some(value) = value.as_mut() {
                        bind_relative_iterated_player_in_value_to_player_filter(
                            value,
                            &PlayerFilter::You,
                        );
                    }
                }
                crate::costs::Cost::DynamicMana(dynamic)
            }
            crate::costs::Cost::Energy(mut value) => {
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut value,
                    &PlayerFilter::You,
                );
                crate::costs::Cost::Energy(value)
            }
            crate::costs::Cost::Mill(mut value) => {
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut value,
                    &PlayerFilter::You,
                );
                crate::costs::Cost::Mill(value)
            }
            crate::costs::Cost::Life(mut value) => {
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut value,
                    &PlayerFilter::You,
                );
                crate::costs::Cost::Life(value)
            }
            crate::costs::Cost::Effect(effect) => {
                if let Some(put) = effect
                    .downcast_ref::<crate::effects::PutCountersEffect>()
                    .cloned()
                {
                    let mut put = put;
                    bind_relative_iterated_player_in_choose_spec_to_player_filter(
                        &mut put.target,
                        &PlayerFilter::You,
                    );
                    bind_relative_iterated_player_in_value_to_player_filter(
                        &mut put.amount,
                        &PlayerFilter::You,
                    );
                    crate::costs::Cost::Effect(Effect::new(put))
                } else if let Some(sacrifice) = effect
                    .downcast_ref::<crate::effects::SacrificeTargetEffect>()
                    .cloned()
                {
                    let mut sacrifice = sacrifice;
                    bind_relative_iterated_player_in_choose_spec_to_player_filter(
                        &mut sacrifice.target,
                        &PlayerFilter::You,
                    );
                    crate::costs::Cost::Effect(Effect::new(sacrifice))
                } else {
                    crate::costs::Cost::Effect(effect)
                }
            }
            other => other,
        };
        Ok::<_, std::convert::Infallible>(component)
    })
    .expect("payer-relative unless-cost normalization is infallible")
}

fn bind_relative_attachment_count_player(value: &mut Value, player: &PlayerFilter) {
    match value {
        Value::SurfaceHinted { value, .. }
        | Value::Scaled(value, _)
        | Value::DividedRoundedDown(value, _)
        | Value::HalfRoundedDown(value) => bind_relative_attachment_count_player(value, player),
        Value::Add(left, right) | Value::Min(left, right) => {
            bind_relative_attachment_count_player(left, player);
            bind_relative_attachment_count_player(right, player);
        }
        Value::Count(filter) | Value::CountScaled(filter, _) => {
            if matches!(
                filter.attached_to_player.as_ref(),
                Some(PlayerFilter::AliasedTarget(inner))
                    if matches!(inner.as_ref(), PlayerFilter::Any)
            ) {
                filter.attached_to_player = Some(player.clone());
            }
        }
        _ => {}
    }
}

fn bind_target_iteration_exclusion_in_attachment_counts(
    effects: &mut [EffectAst],
    excluded: &PlayerFilter,
) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }) = effect
        {
            bind_relative_attachment_count_player(count, excluded);
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            bind_target_iteration_exclusion_in_attachment_counts(nested, excluded);
        });
    }
}

fn sacrifice_all_tag_relation(effects: &[EffectAst]) -> Option<(String, TaggedOpbjectRelation)> {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { filter },
            ..
        }),
    ] = effects
    else {
        return None;
    };
    filter.tagged_constraints.iter().find_map(|constraint| {
        matches!(
            constraint.relation,
            TaggedOpbjectRelation::IsTaggedObject | TaggedOpbjectRelation::IsNotTaggedObject
        )
        .then(|| (constraint.tag.as_str().to_string(), constraint.relation))
    })
}

fn binary_pile_choice_labels(
    effects: &[EffectAst],
    alternative: &[EffectAst],
) -> Option<(&'static str, &'static str)> {
    let (main_tag, main_relation) = sacrifice_all_tag_relation(effects)?;
    let (alternative_tag, alternative_relation) = sacrifice_all_tag_relation(alternative)?;
    if main_tag != alternative_tag
        || !matches!(
            (main_relation, alternative_relation),
            (
                TaggedOpbjectRelation::IsTaggedObject,
                TaggedOpbjectRelation::IsNotTaggedObject
            ) | (
                TaggedOpbjectRelation::IsNotTaggedObject,
                TaggedOpbjectRelation::IsTaggedObject
            )
        )
    {
        return None;
    }

    Some(match main_relation {
        TaggedOpbjectRelation::IsTaggedObject => {
            ("Choose the separated pile", "Choose the other pile")
        }
        TaggedOpbjectRelation::IsNotTaggedObject => {
            ("Choose the other pile", "Choose the separated pile")
        }
        _ => return None,
    })
}

fn compiled_effects_are_play_permissions(effects: &[Effect]) -> bool {
    let mut saw_play_grant = false;
    for effect in effects {
        if effect
            .downcast_ref::<crate::effects::GrantPlayTaggedEffect>()
            .is_some()
        {
            saw_play_grant = true;
            continue;
        }
        if effect
            .downcast_ref::<crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect>()
            .is_some()
        {
            continue;
        }
        return false;
    }
    saw_play_grant
}

fn compiled_effects_handle_their_own_optional_cast_choice(effects: &[Effect]) -> bool {
    matches!(
        effects,
        [effect]
            if effect
                .downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()
                .is_some()
    )
}

fn effect_has_may_decider_scoped_search_followup(effect: &Effect, decider: &PlayerFilter) -> bool {
    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect<Effect>>() {
        return for_each.effects.iter().any(|inner| {
            inner
                .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()
                .is_some_and(|put| put.controller == *decider)
        });
    }
    effect
        .downcast_ref::<crate::effects::ShuffleLibraryEffect>()
        .is_some_and(|shuffle| shuffle.player == *decider)
}

fn reconcile_may_decider_scoped_search_effect(
    effect: &Effect,
    decider: &PlayerFilter,
    force_search_scope: bool,
) -> Effect {
    if let Some(sequence) = effect.downcast_ref::<crate::effects::SequenceEffect>() {
        let sequence_scopes_search = sequence
            .effects
            .iter()
            .any(|child| effect_has_may_decider_scoped_search_followup(child, decider));
        let effects = sequence
            .effects
            .iter()
            .map(|child| {
                reconcile_may_decider_scoped_search_effect(
                    child,
                    decider,
                    force_search_scope || sequence_scopes_search,
                )
            })
            .collect();
        let mut reconciled = sequence.clone();
        reconciled.effects = effects;
        return Effect::new(reconciled);
    }

    if let Some(for_each) = effect.downcast_ref::<crate::effects::ForEachTaggedEffect<Effect>>() {
        let effects = for_each
            .effects
            .iter()
            .map(|child| reconcile_may_decider_scoped_search_effect(child, decider, false))
            .collect();
        return Effect::for_each_tagged(for_each.tag.clone(), effects);
    }

    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return Effect::with_id(
            with_id.id.0,
            reconcile_may_decider_scoped_search_effect(
                &with_id.effect,
                decider,
                force_search_scope,
            ),
        );
    }

    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return Effect::new(crate::effects::TaggedEffect::new(
            tagged.tag.clone(),
            reconcile_may_decider_scoped_search_effect(&tagged.effect, decider, force_search_scope),
        ));
    }

    if !matches!(decider, PlayerFilter::You)
        && let Some(shuffle) =
            effect.downcast_ref::<crate::effects::ShuffleHandAndGraveyardIntoLibraryEffect>()
        && shuffle.player == PlayerFilter::You
    {
        return Effect::shuffle_hand_and_graveyard_into_library_player(decider.clone());
    }

    if !matches!(decider, PlayerFilter::You)
        && let Some(shuffle) =
            effect.downcast_ref::<crate::effects::ShuffleGraveyardIntoLibraryEffect>()
        && shuffle.player == PlayerFilter::You
    {
        return Effect::shuffle_graveyard_into_library_player_with_surface(
            decider.clone(),
            shuffle.explicit_all_cards_from,
        );
    }

    if !matches!(decider, PlayerFilter::You)
        && let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose.is_search
        && choose.zone == Some(Zone::Library)
        && ((choose.filter.owner == Some(PlayerFilter::You) && &choose.chooser == decider)
            || (force_search_scope
                && choose.chooser == PlayerFilter::You
                && matches!(choose.filter.owner.as_ref(), Some(owner) if owner == decider)))
    {
        let mut choose = choose.clone();
        choose.chooser = decider.clone();
        if choose.filter.owner == Some(PlayerFilter::You) {
            choose.filter.owner = Some(decider.clone());
        }
        return Effect::new(choose);
    }

    effect.clone()
}

fn reconcile_may_decider_scoped_search_effects(
    effects: Vec<Effect>,
    decider: &PlayerFilter,
) -> Vec<Effect> {
    effects
        .iter()
        .map(|effect| reconcile_may_decider_scoped_search_effect(effect, decider, false))
        .collect()
}

fn try_compile_for_each_object_become_copy_of_prior_choice(
    filter: &ObjectFilter,
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeCopy {
                    target,
                    source,
                    duration,
                    preserve_source_abilities,
                    name_override,
                    name_override_surface,
                    add_supertypes,
                    remove_supertypes,
                    add_card_types,
                    set_card_types,
                    add_subtypes,
                    set_subtypes,
                    granted_abilities,
                    set_base_power_toughness,
                    copy_exception_surface,
                },
            ..
        }),
    ] = effects
    else {
        return Ok(None);
    };
    let source_reference_span = match source {
        TargetAst::Tagged(source_tag, span) if source_tag.as_str() == IT_TAG => *span,
        TargetAst::Object(source_filter, _, reference_span)
            if source_filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            }) =>
        {
            *reference_span
        }
        _ => return Ok(None),
    };
    if !matches!(
        target,
        TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG
    ) && !matches!(target, TargetAst::Object(_, _, _))
    {
        return Ok(None);
    }

    let refs = current_reference_env(ctx);
    let Some(prior_choice_tag) = refs.known_last_object_tag().cloned() else {
        return Ok(None);
    };

    let mut target_filter = resolve_it_tag(filter, &refs)?;
    if target_filter.other {
        target_filter.other = false;
        target_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: prior_choice_tag.clone(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    let rewritten = EffectAst::subject_verb_become_copy(
        TargetAst::Object(target_filter, None, None),
        TargetAst::Tagged(prior_choice_tag, source_reference_span),
        duration.clone(),
        *preserve_source_abilities,
        name_override.clone(),
        name_override_surface.clone(),
        add_supertypes.clone(),
        remove_supertypes.clone(),
        add_card_types.clone(),
        set_card_types.clone(),
        add_subtypes.clone(),
        set_subtypes.clone(),
        granted_abilities.clone(),
        set_base_power_toughness.clone(),
        copy_exception_surface.clone(),
    );
    Ok(Some(compile_effect(&rewritten, ctx)?))
}

fn choose_spec_is_single_damage_recipient(spec: &ChooseSpec) -> bool {
    if spec.is_target() {
        return true;
    }
    match spec.base() {
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject),
        _ => true,
    }
}

fn bind_iterated_source_stat_value(value: &Value) -> Value {
    match value {
        Value::PowerOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
            Value::PowerOf(Box::new(
                ChooseSpec::Iterated.with_surface_hints(spec.surface_hints().to_vec()),
            ))
        }
        Value::ToughnessOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) => {
            Value::ToughnessOf(Box::new(
                ChooseSpec::Iterated.with_surface_hints(spec.surface_hints().to_vec()),
            ))
        }
        Value::SurfaceHinted { value, hints } => Value::SurfaceHinted {
            value: Box::new(bind_iterated_source_stat_value(value)),
            hints: hints.clone(),
        },
        Value::Add(left, right) => Value::Add(
            Box::new(bind_iterated_source_stat_value(left)),
            Box::new(bind_iterated_source_stat_value(right)),
        ),
        Value::Scaled(value, multiplier) => Value::Scaled(
            Box::new(bind_iterated_source_stat_value(value)),
            *multiplier,
        ),
        value => value.clone(),
    }
}

/// Lower an authored "`Each <object> deals ... equal to its ...`" source set
/// as the damage-source loop. The target is resolved before entering that loop
/// so a demonstrative such as "that permanent" keeps referring to the prior
/// choice instead of being rebound to the current source object.
fn try_compile_for_each_object_as_damage_source(
    filter: &ObjectFilter,
    effects: &[EffectAst],
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. })] = effects else {
        return Ok(None);
    };

    let (source, amount, target, unpreventable) = match action {
        SubjectVerbActionAst::DealDamageEqualToPower {
            source,
            amount,
            target,
            unpreventable,
        } => (Some(source), amount, target, unpreventable),
        SubjectVerbActionAst::DealDamage {
            amount,
            target,
            unpreventable,
        } if matches!(
            amount.unhinted(),
            Value::PowerOf(spec) | Value::ToughnessOf(spec)
                if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG)
        ) =>
        {
            (None, amount, target, unpreventable)
        }
        _ => return Ok(None),
    };

    let source_is_iterand = source.is_none()
        || matches!(
            source,
            Some(TargetAst::Object(source_filter, _, _)) if source_filter == filter
        )
        || matches!(
            source,
            Some(TargetAst::Tagged(tag, _)) if tag.as_str() == IT_TAG
        );
    if !source_is_iterand {
        return Ok(None);
    }

    let refs = current_reference_env(ctx);
    let resolved_filter = resolve_it_tag(filter, &refs)?;
    let other_member_of_prior_set = matches!(target, TargetAst::AnyOtherTarget(_))
        && filter.set_quantifier_surface() == Some(ironsmith_core::SetQuantifierSurface::Those)
        && filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].relation == TaggedOpbjectRelation::IsTaggedObject
        && matches!(
            filter.tagged_constraints[0].tag.as_str(),
            IT_TAG | ironsmith_core::CHOSEN_OBJECTS_TAG
        );
    let (target_spec, choices) = if source.is_some_and(|source| source == target) {
        (ChooseSpec::Iterated, Vec::new())
    } else if other_member_of_prior_set {
        // "The other" is an anaphoric member of the already chosen pair, not
        // a third target. Keep the prior-set tag and exclude the temporarily
        // rebound damage source while each member is iterated.
        (
            ChooseSpec::Object(resolved_filter.clone().other()),
            Vec::new(),
        )
    } else {
        resolve_target_spec_with_choices(target, &refs)?
    };
    if !choose_spec_is_single_damage_recipient(&target_spec) {
        return Ok(None);
    }

    let resolved_amount = if source.is_none() {
        bind_iterated_source_stat_value(amount)
    } else {
        resolve_value_it_tag(amount, &refs)?
    };
    let damage_amount = super::effect_dispatch::bind_source_value_to_damage_source(
        &resolved_amount,
        &ChooseSpec::Iterated,
    );
    let damage = if *unpreventable {
        Effect::deal_unpreventable_damage(damage_amount, target_spec.clone())
    } else {
        Effect::deal_damage(damage_amount, target_spec.clone())
    };
    let per_source_damage = Effect::new(crate::effects::ExecuteWithSourceEffect::new(
        ChooseSpec::Iterated,
        damage,
    ));
    let per_source_damage =
        tag_object_target_effect(per_source_damage, &target_spec, ctx, "damaged");

    if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) = target {
        ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
    } else if matches!(
        target,
        TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_)
    ) {
        ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
    }

    Ok(Some((
        vec![Effect::for_each(resolved_filter, vec![per_source_damage])],
        choices,
    )))
}

pub(super) fn try_compile_flow_and_iteration_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::May { effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty may-effect branch is unsupported".to_string(),
                ));
            }
            if let Some(compiled) = lower_may_imprint_from_hand_effect(effects, ctx)? {
                return Ok(Some(compiled));
            }
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-effect branch is unsupported".to_string(),
                ));
            }
            if compiled_effects_are_play_permissions(&inner_effects)
                || compiled_effects_handle_their_own_optional_cast_choice(&inner_effects)
            {
                return Ok(Some((inner_effects, inner_choices)));
            }
            let effect = if ctx.iterated_player
                && ctx.last_player_filter.as_ref() == Some(&PlayerFilter::IteratedPlayer)
            {
                let inner_effects = reconcile_may_decider_scoped_search_effects(
                    inner_effects,
                    &PlayerFilter::IteratedPlayer,
                );
                Effect::may_player(PlayerFilter::IteratedPlayer, inner_effects)
            } else {
                Effect::may(inner_effects)
            };
            (vec![effect], inner_choices)
        }
        EffectAst::MayByPlayer { player, effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty may-by-player effect branch is unsupported".to_string(),
                ));
            }
            if matches!(player, PlayerAst::You | PlayerAst::Implicit)
                && let Some(compiled) = lower_may_imprint_from_hand_effect(effects, ctx)?
            {
                return Ok(Some(compiled));
            }
            let saved_last_object_tag = ctx.last_object_tag.clone();
            let saved_last_player_filter = ctx.last_player_filter.clone();
            if matches!(player, PlayerAst::ItsController | PlayerAst::ItsOwner)
                && let Some(tag) = single_cast_tagged_reference_tag(effects, ctx)?
            {
                ctx.last_object_tag = Some(tag);
            }
            let subject = LoweredSubject::resolve_chooser(*player, ctx, true, true, true)?;
            let player_filter = subject.into_player_filter();
            ctx.last_object_tag = saved_last_object_tag;
            // The may-decider ("you" in "you may …") is the chooser, not a
            // referenced player; don't let it shadow "that player" inside the may
            // ("you may have that player lose 2 life" → the triggering opponent).
            // An iterated "that player may" actor is different: relative
            // filters inside that offer name the same participant and must not
            // fall back to the trigger's broad lexical player filter.
            ctx.last_player_filter = if matches!(player_filter, PlayerFilter::IteratedPlayer) {
                Some(PlayerFilter::IteratedPlayer)
            } else if matches!(player, PlayerAst::Target | PlayerAst::TargetOpponent) {
                // The enclosing offer introduced this player as an explicit
                // target. Keep that target in scope while lowering references
                // to the same participant inside the offer (for example,
                // "target opponent may ... They may ..."). This does not
                // create a second target choice: the MayByPlayer subject owns
                // the one explicit target declaration.
                Some(player_filter.clone())
            } else {
                saved_last_player_filter
            };
            let (inner_effects, inner_choices) =
                compile_effects_preserving_last_effect(effects, ctx)?;
            if inner_effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty compiled may-by-player effect branch is unsupported".to_string(),
                ));
            }
            let inner_effects =
                reconcile_may_decider_scoped_search_effects(inner_effects, &player_filter);
            let mut choices = inner_choices;
            choices.extend(subject.into_choices());
            if compiled_effects_are_play_permissions(&inner_effects)
                || compiled_effects_handle_their_own_optional_cast_choice(&inner_effects)
            {
                return Ok(Some((inner_effects, choices)));
            }
            let effect = Effect::may_player(player_filter, inner_effects);
            (vec![effect], choices)
        }
        EffectAst::AnyPlayerMay { players, effects } => {
            if effects.is_empty() {
                return Err(CardTextError::ParseError(
                    "empty any-player-may effect branch is unsupported".to_string(),
                ));
            }
            let offers = [EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: effects.clone(),
            }];
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(&offers, ctx, None)?;
            let effect = Effect::new(crate::effects::ForPlayersEffect {
                filter: players.clone(),
                effects: inner_effects,
                starting_with_controller: true,
                stop_after_first_happened: true,
            });
            (vec![effect], inner_choices)
        }
        EffectAst::RepeatThisProcessMay => (
            vec![Effect::new(crate::effects::RepeatProcessPromptEffect::new(
                ironsmith_core::RepeatProcessPromptKind::MayRepeatAnyNumberOfTimes,
            ))],
            Vec::new(),
        ),
        EffectAst::UnlessPays {
            effects,
            player,
            cost,
            before_delayed_step,
        } => {
            // A trailing "unless they pay" attached to an each-player
            // instruction is evaluated separately for every iterated player.
            // Keep the payment inside the loop so `they` resolves against the
            // loop binding rather than an outer (often "you") antecedent.
            if matches!(player, PlayerAst::That)
                && let [per_player] = effects.as_slice()
            {
                let rewritten = match per_player {
                    EffectAst::ForEachPlayer {
                        effects: per_player_effects,
                    } => Some(EffectAst::ForEachPlayer {
                        effects: vec![EffectAst::UnlessPays {
                            effects: per_player_effects.clone(),
                            player: *player,
                            cost: cost.clone(),
                            before_delayed_step: *before_delayed_step,
                        }],
                    }),
                    EffectAst::ForEachOpponent {
                        effects: per_player_effects,
                    } => Some(EffectAst::ForEachOpponent {
                        effects: vec![EffectAst::UnlessPays {
                            effects: per_player_effects.clone(),
                            player: *player,
                            cost: cost.clone(),
                            before_delayed_step: *before_delayed_step,
                        }],
                    }),
                    EffectAst::ForEachPlayersFiltered {
                        filter,
                        effects: per_player_effects,
                    } => Some(EffectAst::ForEachPlayersFiltered {
                        filter: filter.clone(),
                        effects: vec![EffectAst::UnlessPays {
                            effects: per_player_effects.clone(),
                            player: *player,
                            cost: cost.clone(),
                            before_delayed_step: *before_delayed_step,
                        }],
                    }),
                    _ => None,
                };
                if let Some(rewritten) = rewritten {
                    return Ok(Some(compile_effect(&rewritten, ctx)?));
                }
            }

            if effects.len() == 1
                && let EffectAst::ForEachObject {
                    filter,
                    effects: per_object_effects,
                } = &effects[0]
            {
                let rewritten = EffectAst::ForEachObject {
                    filter: filter.clone(),
                    effects: vec![EffectAst::UnlessPays {
                        effects: per_object_effects.clone(),
                        player: *player,
                        cost: cost.clone(),
                        before_delayed_step: *before_delayed_step,
                    }],
                };
                return Ok(Some(compile_effect(&rewritten, ctx)?));
            }

            let previous_last_player_filter = ctx.last_player_filter.clone();
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let (player_filter, mut player_choices) = match player {
                PlayerAst::Target => (
                    PlayerFilter::target_player(),
                    vec![ChooseSpec::target_player()],
                ),
                PlayerAst::TargetOpponent => (
                    PlayerFilter::target_opponent(),
                    vec![ChooseSpec::target(ChooseSpec::Player(
                        PlayerFilter::Opponent,
                    ))],
                ),
                _ => (
                    resolve_unless_player_filter(
                        *player,
                        &current_reference_env(ctx),
                        previous_last_player_filter,
                    )?,
                    Vec::new(),
                ),
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let mut choices = inner_choices;
            choices.append(&mut player_choices);
            let runtime_cost =
                crate::lowering::cost_materialization::materialize_compiler_core_total_cost(cost)?;
            let payer_relative_cost = normalize_unless_cost_for_payer(runtime_cost);
            let resolved_cost =
                resolve_total_cost_it_tags(&payer_relative_cost, &current_reference_env(ctx))?;
            let effect = Effect::new(crate::effects::UnlessPaysEffect {
                player: player_filter,
                effects: inner_effects,
                cost: resolved_cost,
                leading_surface: false,
                before_delayed_step: *before_delayed_step,
            });
            (vec![effect], choices)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            player,
        } => {
            if effects.len() == 1
                && let EffectAst::ForEachObject {
                    filter,
                    effects: per_object_effects,
                } = &effects[0]
            {
                let rewritten = EffectAst::ForEachObject {
                    filter: filter.clone(),
                    effects: vec![EffectAst::UnlessAction {
                        effects: per_object_effects.clone(),
                        alternative: alternative.clone(),
                        player: *player,
                    }],
                };
                return Ok(Some(compile_effect(&rewritten, ctx)?));
            }

            let previous_last_player_filter = ctx.last_player_filter.clone();
            let (inner_effects, inner_choices) = compile_effects(effects, ctx)?;
            let (alt_effects, alt_choices) = compile_effects(alternative, ctx)?;
            // In `destroy/counter target ... unless its controller ...`, the
            // possessive refers to the target declared by this same primary
            // action. A trigger setup tag may still be the ambient
            // `last_object_tag`; do not let that unrelated antecedent steal
            // the decision from the actual target's controller.
            let player_filter = if matches!(player, PlayerAst::ItsController)
                && inner_choices.iter().any(ChooseSpec::is_target)
            {
                PlayerFilter::ControllerOf(crate::target::ObjectRef::Target)
            } else {
                resolve_unless_player_filter(
                    *player,
                    &current_reference_env(ctx),
                    previous_last_player_filter,
                )?
            };
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let effect = if matches!(player_filter, PlayerFilter::You)
                && let Some((main_label, alternative_label)) =
                    binary_pile_choice_labels(effects, alternative)
            {
                Effect::choose_one(vec![
                    EffectMode {
                        source_text: main_label.to_string(),
                        effects: inner_effects,
                    },
                    EffectMode {
                        source_text: alternative_label.to_string(),
                        effects: alt_effects,
                    },
                ])
            } else {
                Effect::unless_action(inner_effects, alt_effects, player_filter)
            };
            let mut choices = inner_choices;
            choices.extend(alt_choices);
            (vec![effect], choices)
        }
        EffectAst::IfResult { predicate, effects } => {
            let condition = if crate::reference_resolution::if_result_predicate_is_searched_library(predicate) {
                ctx.last_library_search_effect_id.or(ctx.last_effect_id)
            } else {
                ctx.last_effect_id
            }
            .ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for if clause".to_string())
            })?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(condition);
                    ctx.bind_unbound_x_to_last_effect =
                        predicate != &IfResultPredicate::AcceptedChoice;
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(predicate.clone());
            let effect = Effect::if_then(condition, predicate, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::WhenResult { predicate, effects } => {
            let condition = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for when clause".to_string())
            })?;
            let (inner_effects, inner_choices) = with_preserved_lowering_context(
                ctx,
                |ctx| {
                    ctx.last_effect_id = Some(condition);
                    ctx.bind_unbound_x_to_last_effect = true;
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(predicate.clone());
            let effect =
                Effect::reflexive_trigger(condition, predicate, inner_effects, inner_choices);
            (vec![effect], Vec::new())
        }
        EffectAst::ForEachOpponent { effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_opponent(inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachPlayersFiltered { filter, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = try_compile_simultaneous_each_player_scry(filter.clone(), &inner_effects)
                .unwrap_or_else(|| Effect::for_players(filter.clone(), inner_effects));
            let mut target_choices = Vec::new();
            collect_targeted_player_specs_from_player_filter(filter, &mut target_choices);
            let mut compiled = target_choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect::<Vec<_>>();
            compiled.push(effect);
            let mut choices = target_choices;
            for choice in inner_choices {
                push_choice(&mut choices, choice);
            }
            (compiled, choices)
        }
        EffectAst::ForEachPlayer { effects } => {
            if let [
                EffectAst::May {
                    effects: may_effects,
                },
            ] = effects.as_slice()
                && let [EffectAst::Coordination(coordination)] = may_effects.as_slice()
                && coordination.kind == crate::model::CoordinationKindAst::Sequence
                && coordination.boundaries.last().is_some_and(|boundary| {
                    boundary.operator == crate::model::CoordinationOperatorAst::CommaThen
                })
                && let Some((followup_member, antecedent_members)) =
                    coordination.members.split_last()
                && let [followup] = followup_member.effects.as_slice()
                && matches!(followup, EffectAst::ForEachOpponentDoesNot { .. })
            {
                let antecedent_may_effects = antecedent_members
                    .iter()
                    .flat_map(|member| member.effects.iter().cloned())
                    .collect::<Vec<_>>();
                if !antecedent_may_effects.is_empty() {
                    let antecedent = EffectAst::ForEachPlayer {
                        effects: vec![EffectAst::May {
                            effects: antecedent_may_effects,
                        }],
                    };
                    if let Some((effects, choices)) =
                        compile_if_do_with_opponent_doesnt(&antecedent, followup, ctx)?
                    {
                        return Ok(Some((effects, choices)));
                    }
                }
            }
            if let [
                EffectAst::May {
                    effects: may_effects,
                },
            ] = effects.as_slice()
                && let [
                    EffectAst::CommaThen {
                        effects: comma_then_effects,
                    },
                ] = may_effects.as_slice()
                && let Some((followup, antecedent_may_effects)) = comma_then_effects.split_last()
                && !antecedent_may_effects.is_empty()
                && matches!(followup, EffectAst::ForEachOpponentDoesNot { .. })
            {
                let antecedent = EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::May {
                        effects: antecedent_may_effects.to_vec(),
                    }],
                };
                if let Some((effects, choices)) =
                    compile_if_do_with_opponent_doesnt(&antecedent, followup, ctx)?
                {
                    return Ok(Some((effects, choices)));
                }
            }
            if let [
                EffectAst::May {
                    effects: may_effects,
                },
            ] = effects.as_slice()
                && let Some((followup, antecedent_may_effects)) = may_effects.split_last()
                && !antecedent_may_effects.is_empty()
                && matches!(followup, EffectAst::ForEachOpponentDoesNot { .. })
            {
                let antecedent = EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::May {
                        effects: antecedent_may_effects.to_vec(),
                    }],
                };
                if let Some((effects, choices)) =
                    compile_if_do_with_opponent_doesnt(&antecedent, followup, ctx)?
                {
                    return Ok(Some((effects, choices)));
                }
            }
            if let Some((followup, antecedent_effects)) = effects.split_last()
                && !antecedent_effects.is_empty()
                && matches!(followup, EffectAst::ForEachOpponentDoesNot { .. })
            {
                let antecedent = EffectAst::ForEachPlayer {
                    effects: antecedent_effects.to_vec(),
                };
                if let Some((effects, choices)) =
                    compile_if_do_with_opponent_doesnt(&antecedent, followup, ctx)?
                {
                    return Ok(Some((effects, choices)));
                }
            }
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect =
                try_compile_simultaneous_each_player_scry(PlayerFilter::Any, &inner_effects)
                    .unwrap_or_else(|| Effect::for_players(PlayerFilter::Any, inner_effects));
            (vec![effect], inner_choices)
        }
        EffectAst::DirectionalAdjacentPlayerControl {
            filter,
            left_option,
            right_option,
        } => {
            let effect = Effect::new(crate::effects::DirectionalAdjacentPlayerControlEffect::new(
                filter.clone(),
                left_option.clone(),
                right_option.clone(),
            ));
            (vec![effect], Vec::new())
        }
        EffectAst::ForEachTargetPlayers {
            count,
            filter,
            effects,
        } => {
            let target_spec = resolve_choose_spec_it_tag(
                &ChooseSpec::target(ChooseSpec::Player(filter.clone())).with_count(*count),
                &current_reference_env(ctx),
            )?;
            let ChooseSpec::Player(resolved_filter) = target_spec.base() else {
                return Err(CardTextError::ParseError(
                    "target-player iterator resolved to a non-player target".to_string(),
                ));
            };
            let mut scoped_effects = effects.clone();
            if let PlayerFilter::Excluding { excluded, .. } = resolved_filter {
                bind_target_iteration_exclusion_in_attachment_counts(&mut scoped_effects, excluded);
            }
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(&scoped_effects, ctx, None)?;
            let choose_targets =
                Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()));
            let iteration_filter = PlayerFilter::Target(Box::new(resolved_filter.clone()));
            let effect =
                try_compile_simultaneous_each_player_scry(iteration_filter.clone(), &inner_effects)
                    .unwrap_or_else(|| Effect::for_players(iteration_filter, inner_effects));
            let mut choices = vec![target_spec];
            for choice in inner_choices {
                push_choice(&mut choices, choice);
            }
            (vec![choose_targets, effect], choices)
        }
        EffectAst::ForEachObject { filter, effects } => {
            if let Some(compiled) =
                try_compile_for_each_object_become_copy_of_prior_choice(filter, effects, ctx)?
            {
                return Ok(Some(compiled));
            }
            if let Some(compiled) =
                try_compile_for_each_object_as_damage_source(filter, effects, ctx)?
            {
                return Ok(Some(compiled));
            }
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_object_context(effects, ctx)?;
            let effect = Effect::for_each(resolved_filter, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTagged { tag, effects } => {
            let effective_tag = if let Some(concrete) = ctx
                .snapshot_tag_aliases
                .iter()
                .find(|(alias, _)| alias == tag.as_str())
                .map(|(_, concrete)| concrete.clone())
            {
                concrete
            } else if tag.as_str() == IT_TAG {
                ctx.last_object_tag
                    .clone()
                    .unwrap_or_else(|| IT_TAG.to_string())
            } else {
                tag.as_str().to_string()
            };

            let (inner_effects, inner_choices) = compile_effects_in_iterated_player_context(
                effects,
                ctx,
                Some(effective_tag.clone()),
            )?;
            let effect = Effect::for_each_tagged(effective_tag, inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachTaggedWithControllerAtLastBlockedBy {
            tag,
            blocker_tag,
            effects,
        } => {
            let resolve_tag = |tag: &TagKey| {
                ctx.snapshot_tag_aliases
                    .iter()
                    .find(|(alias, _)| alias == tag.as_str())
                    .map(|(_, concrete)| concrete.clone())
                    .unwrap_or_else(|| tag.as_str().to_string())
            };
            let effective_tag = resolve_tag(tag);
            let effective_blocker_tag = resolve_tag(blocker_tag);
            let (inner_effects, inner_choices) = compile_effects_in_iterated_player_context(
                effects,
                ctx,
                Some(effective_tag.clone()),
            )?;
            let effect = Effect::for_each_tagged_with_controller_at_last_blocked_by(
                effective_tag,
                inner_effects,
                effective_blocker_tag,
            );
            (vec![effect], inner_choices)
        }
        EffectAst::MoveTaggedGroupToZone { tag, zone } => {
            let effective_tag = ctx
                .snapshot_tag_aliases
                .iter()
                .find(|(alias, _)| alias == tag.as_str())
                .map(|(_, concrete)| concrete.clone())
                .unwrap_or_else(|| tag.as_str().to_string());
            let effect = Effect::for_each_tagged(
                effective_tag,
                vec![Effect::move_to_zone(ChooseSpec::Iterated, *zone, false)],
            );
            (vec![effect], Vec::new())
        }
        EffectAst::SnapshotLastObjectTag { into } => {
            // Lowering-time alias: bind `into` to the concrete tag currently in
            // `last_object_tag` so later composed effects can reference the
            // earlier looked-at pool even after an intervening `ChooseObjects`
            // clobbers `last_object_tag`. Emits no runtime effect.
            if let Some(concrete) = ctx.last_object_tag.clone() {
                ctx.snapshot_tag_aliases
                    .retain(|(alias, _)| alias != into.as_str());
                ctx.snapshot_tag_aliases
                    .push((into.as_str().to_string(), concrete));
            }
            (Vec::new(), Vec::new())
        }
        EffectAst::ForEachTaggedPlayer { tag, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let effect = Effect::for_each_tagged_player(tag.clone(), inner_effects);
            (vec![effect], inner_choices)
        }
        EffectAst::RepeatProcess {
            effects,
            continue_effect_index,
            continue_predicate,
        } => {
            let (mut body_effects, choices, condition) = with_preserved_lowering_context(
                ctx,
                |_| {},
                |ctx| compile_repeat_process_body(effects, *continue_effect_index, ctx),
            )?;
            // Targets for a repeated process are declared once when the
            // ability is put on the stack. Synthetic TargetOnly declarations
            // inserted while compiling the body must therefore stay outside
            // the loop; otherwise a target changed by the first iteration is
            // revalidated before the second iteration can begin.
            let target_prelude_count = body_effects
                .iter()
                .take_while(|effect| {
                    effect
                        .downcast_ref::<crate::effects::TargetOnlyEffect>()
                        .is_some()
                })
                .count();
            let mut compiled = body_effects
                .drain(..target_prelude_count)
                .collect::<Vec<_>>();
            let effect = Effect::repeat_process(
                body_effects,
                condition,
                effect_predicate_from_if_result(continue_predicate.clone()),
            );
            compiled.push(effect);
            (compiled, choices)
        }
        EffectAst::ForEachOpponentDoesNot { .. } => {
            return Err(CardTextError::ParseError(
                "for each opponent who doesn't must follow an opponent clause".to_string(),
            ));
        }
        EffectAst::ForEachPlayerDoesNot { .. } => {
            return Err(CardTextError::ParseError(
                "for each player who doesn't must follow a player clause".to_string(),
            ));
        }
        EffectAst::ForEachOpponentDid { .. } => {
            return Err(CardTextError::ParseError(
                "for each opponent who ... this way must follow an opponent clause".to_string(),
            ));
        }
        EffectAst::ForEachPlayerDid { .. } => {
            return Err(CardTextError::ParseError(
                "for each player who ... this way must follow a player clause".to_string(),
            ));
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

fn single_cast_tagged_reference_tag(
    effects: &[EffectAst],
    ctx: &EffectLoweringContext,
) -> Result<Option<String>, CardTextError> {
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CastTagged { tag, .. },
            ..
        }),
    ] = effects
    else {
        return Ok(None);
    };

    if tag.as_str() == "__last_revealed__" {
        return Ok(ctx.last_revealed_tag.clone());
    }
    if tag.as_str() == IT_TAG {
        return Ok(ctx.last_object_tag.clone());
    }
    Ok(Some(tag.as_str().to_string()))
}

pub(super) fn try_compile_token_generation_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let _ = (effect, ctx);
    Ok(None)
}

pub(super) fn try_compile_search_and_reorder_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::VoteOption { option, effects } => {
            let mut option_effects_ast = effects.clone();
            force_implicit_vote_token_controller_you(&mut option_effects_ast);
            let (repeat_effects, repeat_choices) = compile_effects(&option_effects_ast, ctx)?;
            (
                vec![Effect::repeat_effects(
                    Value::VoteCount(option.clone()),
                    repeat_effects,
                )],
                repeat_choices,
            )
        }
        EffectAst::SecretChoiceReveal => (Vec::new(), Vec::new()),
        EffectAst::VoteStart { .. }
        | EffectAst::VoteStartObjects { .. }
        | EffectAst::VoteStartPlayers { .. }
        | EffectAst::VoteExtra { .. } => {
            return Err(CardTextError::ParseError(
                "vote clauses must appear together".to_string(),
            ));
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

use super::*;

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
        return Effect::new(crate::effects::SequenceEffect::new(effects));
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

    if force_search_scope
        && !matches!(decider, PlayerFilter::You)
        && let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()
        && choose.is_search
        && choose.zone == Some(Zone::Library)
        && choose.chooser == PlayerFilter::You
        && choose.filter.owner == Some(PlayerFilter::You)
    {
        let mut choose = choose.clone();
        choose.chooser = decider.clone();
        choose.filter.owner = Some(decider.clone());
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
                    source: TargetAst::Tagged(source_tag, _),
                    duration,
                    preserve_source_abilities,
                    name_override,
                    name_override_surface,
                    add_supertypes,
                },
            ..
        }),
    ] = effects
    else {
        return Ok(None);
    };
    if source_tag.as_str() != IT_TAG {
        return Ok(None);
    }
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
        TargetAst::Tagged(prior_choice_tag, None),
        duration.clone(),
        *preserve_source_abilities,
        name_override.clone(),
        name_override_surface.clone(),
        add_supertypes.clone(),
    );
    Ok(Some(compile_effect(&rewritten, ctx)?))
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
            if compiled_effects_are_play_permissions(&inner_effects) {
                return Ok(Some((inner_effects, inner_choices)));
            }
            let effect = Effect::may(inner_effects);
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
            ctx.last_player_filter = saved_last_player_filter;
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
            if compiled_effects_are_play_permissions(&inner_effects) {
                return Ok(Some((inner_effects, choices)));
            }
            let effect = Effect::may_player(player_filter, inner_effects);
            (vec![effect], choices)
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
        } => {
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
            let effect = Effect::unless_pays_total_cost(inner_effects, player_filter, cost.clone());
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
            let player_filter = resolve_unless_player_filter(
                *player,
                &current_reference_env(ctx),
                previous_last_player_filter,
            )?;
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
            let condition = if matches!(predicate, IfResultPredicate::SearchedLibrary) {
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
                    ctx.bind_unbound_x_to_last_effect = true;
                },
                |ctx| compile_effects(effects, ctx),
            )?;
            let predicate = effect_predicate_from_if_result(*predicate);
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
            let predicate = effect_predicate_from_if_result(*predicate);
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
            (vec![effect], inner_choices)
        }
        EffectAst::ForEachPlayer { effects } => {
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
        EffectAst::ForEachTargetPlayers { count, effects } => {
            let (inner_effects, inner_choices) =
                compile_effects_in_iterated_player_context(effects, ctx, None)?;
            let target_spec =
                ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)).with_count(*count);
            let choose_targets =
                Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()));
            let effect = try_compile_simultaneous_each_player_scry(
                PlayerFilter::target_player(),
                &inner_effects,
            )
            .unwrap_or_else(|| Effect::for_players(PlayerFilter::target_player(), inner_effects));
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
            let (body_effects, choices, condition) = with_preserved_lowering_context(
                ctx,
                |_| {},
                |ctx| compile_repeat_process_body(effects, *continue_effect_index, ctx),
            )?;
            let effect = Effect::repeat_process(
                body_effects,
                condition,
                effect_predicate_from_if_result(*continue_predicate),
            );
            (vec![effect], choices)
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

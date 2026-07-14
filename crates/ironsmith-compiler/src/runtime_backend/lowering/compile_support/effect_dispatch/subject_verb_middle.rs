use super::*;

fn choose_spec_may_hold_multiple_objects(spec: &ChooseSpec) -> bool {
    match spec.unhinted() {
        ChooseSpec::WithCount(_, count) | ChooseSpec::WithCountValue(_, count, _) => {
            count.max != Some(1)
        }
        ChooseSpec::All(_) | ChooseSpec::EachPlayer(_) => true,
        _ => false,
    }
}

pub(super) fn compile_subject_verb_middle(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    let player = subject_verb.subject.player;
    let result = match &subject_verb.action {
        SubjectVerbActionAst::PutOrRemoveCounters {
            put_counter_type,
            put_count,
            remove_counter_type,
            remove_count,
            put_mode_text,
            remove_mode_text,
            target,
            target_count,
        } => {
            use crate::effect::EffectMode;

            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }

            let put_effect =
                Effect::put_counters(*put_counter_type, put_count.clone(), spec.clone());
            let remove_effect =
                Effect::remove_counters(*remove_counter_type, remove_count.clone(), spec.clone());

            let effect = Effect::choose_one(vec![
                EffectMode {
                    source_text: put_mode_text.clone(),
                    effects: vec![put_effect],
                },
                EffectMode {
                    source_text: remove_mode_text.clone(),
                    effects: vec![remove_effect],
                },
            ]);

            let effect = tag_object_target_effect(effect, &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::CopySpell {
            target,
            all_matches,
            count,
            player,
            may_choose_new_targets,
            choose_new_target_singular,
            removed_supertypes,
        } => {
            let (mut spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if *all_matches && let ChooseSpec::Object(filter) = &spec {
                spec = ChooseSpec::All(filter.clone());
            }
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter.clone());
            }
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let copy_effect = Effect::with_id(
                id.0,
                Effect::new(
                    crate::effects::CopySpellEffect::new_for_player(
                        spec.clone(),
                        count.clone(),
                        player_filter.clone(),
                    )
                    .with_removed_supertypes(removed_supertypes.clone()),
                ),
            )
            .tag(COPIED_STACK_OBJECT_TAG);
            let retarget_effect = if *may_choose_new_targets {
                let retarget = crate::effects::ChooseNewTargetsEffect::may_for_player(
                    id,
                    player_filter.clone(),
                );
                let retarget = if *choose_new_target_singular {
                    retarget.with_single_target_surface()
                } else {
                    retarget
                };
                Some(Effect::new(retarget))
            } else {
                None
            };
            let mut compiled = vec![copy_effect];
            if let Some(retarget) = retarget_effect {
                compiled.push(retarget);
            }
            Ok((compiled, choices))
        }
        SubjectVerbActionAst::CopySpellForEachTarget {
            target,
            object_filter,
            player_filter,
            player,
            exclude_current_targets,
            removed_supertypes,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let player_filter_for_copies =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            if !matches!(*player, PlayerAst::Implicit) {
                ctx.last_player_filter = Some(player_filter_for_copies.clone());
            }
            let mut copy_effect = crate::effects::CopySpellForEachTargetEffect::new(spec.clone())
                .with_copier(player_filter_for_copies)
                .exclude_current_targets(*exclude_current_targets)
                .with_removed_supertypes(removed_supertypes.clone());
            if let Some(filter) = object_filter {
                copy_effect = copy_effect
                    .with_object_filter(resolve_it_tag(filter, &current_reference_env(ctx))?);
            }
            if let Some(filter) = player_filter {
                copy_effect = copy_effect.with_player_filter(filter.clone());
            }
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let effect =
                Effect::with_id(id.0, Effect::new(copy_effect)).tag(COPIED_STACK_OBJECT_TAG);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PutTaggedRemainderInZone {
            tag,
            keep_tagged,
            zone,
        } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let resolved_keep = resolve_it_tag_key(keep_tagged, &current_reference_env(ctx))?;
            let mut membership_filter = ObjectFilter::default();
            membership_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_keep = Condition::TaggedObjectMatches(resolved_keep, membership_filter);
            let move_rest = Effect::for_each_tagged(
                resolved_tag,
                vec![Effect::conditional(
                    in_keep,
                    Vec::new(),
                    vec![Effect::move_to_zone(ChooseSpec::Iterated, *zone, false)],
                )],
            );
            Ok((vec![move_rest], Vec::new()))
        }
        SubjectVerbActionAst::ScaleXValue { target, multiplier } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            Ok((vec![Effect::scale_x_value(spec, *multiplier)], choices))
        }
        SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
            tag,
            keep_tagged,
            order,
            player,
        } => {
            let current_refs = current_reference_env(ctx);
            let (resolved_tag, inferred_keep_tagged) = if tag.as_str() == IT_TAG
                && let Some(revealed_tag) = ctx.last_revealed_tag.clone()
                && ctx.last_object_tag.as_deref() != Some(revealed_tag.as_str())
            {
                (
                    TagKey::from(revealed_tag.as_str()),
                    ctx.last_object_tag
                        .as_ref()
                        .map(|tag| TagKey::from(tag.as_str())),
                )
            } else if tag.as_str() == "__last_revealed__" {
                (
                    TagKey::from(ctx.last_revealed_tag.clone().ok_or_else(|| {
                        CardTextError::ParseError(
                            "unable to resolve revealed remainder without prior reveal".to_string(),
                        )
                    })?),
                    None,
                )
            } else {
                (resolve_it_tag_key(tag, &current_refs)?, None)
            };
            let resolved_keep_tagged = keep_tagged
                .as_ref()
                .map(|tag| resolve_it_tag_key(tag, &current_refs))
                .transpose()?
                .or(inferred_keep_tagged);
            // A chooser can intervene between a player's reveal/look and
            // "that player puts the rest ...". Resolve the disposition's
            // library owner from the tagged revealed collection instead of
            // letting that intervening chooser replace the antecedent.
            let subject = if *player == PlayerAst::That
                && ctx.last_revealed_tag.as_deref() == Some(resolved_tag.as_str())
                && let Some(revealed_player) = ctx.last_revealed_player_filter.clone()
            {
                LoweredSubject::from_resolved(as_followup_player_alias(revealed_player), Vec::new())
                    .as_role(SubjectRole::LibraryOwner)
            } else {
                LoweredSubject::resolve_library_owner(*player, ctx, true, true, true)?
            };
            let player_filter = subject.clone_player_filter();
            let resolved_order = match order {
                crate::cards::builders::LibraryBottomOrderAst::Random => {
                    crate::effects::consult_helpers::LibraryBottomOrder::Random
                }
                crate::cards::builders::LibraryBottomOrderAst::ChooserChooses => {
                    crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
                }
            };
            Ok((
                vec![Effect::put_tagged_remainder_on_library_bottom(
                    resolved_tag,
                    resolved_keep_tagged,
                    resolved_order,
                    player_filter,
                )],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::CastTagged {
            tag,
            player,
            allow_land,
            as_copy,
            without_paying_mana_cost,
            cost_reduction,
        } => {
            let resolved_tag = if tag.as_str() == "__last_revealed__" {
                TagKey::from(ctx.last_revealed_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve last revealed card without prior reveal".to_string(),
                    )
                })?)
            } else if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else if tag.as_str() == "__source_exiled__" {
                TagKey::from(ctx.last_exiled_collection_tag.clone().unwrap_or_else(|| {
                    format!(
                        "__sentence_helper_exiled_l0_s0_e{}",
                        ctx.id_gen_context().next_tag_id.saturating_sub(1)
                    )
                }))
            } else {
                tag.clone()
            };
            let player_filter = match player {
                PlayerAst::ItsOwner => {
                    PlayerFilter::OwnerOf(ObjectRef::tagged(resolved_tag.clone()))
                }
                PlayerAst::ItsController => {
                    PlayerFilter::ControllerOf(ObjectRef::tagged(resolved_tag.clone()))
                }
                _ => resolve_non_target_player_filter(*player, &current_reference_env(ctx))?,
            };
            Ok((
                vec![Effect::cast_tagged(
                    resolved_tag,
                    player_filter,
                    *allow_land,
                    *as_copy,
                    *without_paying_mana_cost,
                    cost_reduction.clone(),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            while_on_top_of_library,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == "__last_revealed__" {
                TagKey::from(ctx.last_revealed_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve last revealed card without prior reveal".to_string(),
                    )
                })?)
            } else if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else if tag.as_str() == "__source_exiled__" {
                TagKey::from(ctx.last_exiled_collection_tag.clone().unwrap_or_else(|| {
                    format!(
                        "__sentence_helper_exiled_l0_s0_e{}",
                        ctx.id_gen_context().next_tag_id.saturating_sub(1)
                    )
                }))
            } else {
                tag.clone()
            };
            let mut grant_play = crate::effects::GrantPlayTaggedEffect::new(
                resolved_tag.clone(),
                player_filter.clone(),
                crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                *allow_land,
                *allow_any_color_for_cast,
            );
            if is_sentence_helper_exiled_collection_tag(resolved_tag.as_str())
                && ctx.last_exiled_collection_is_plural
            {
                grant_play = grant_play.cast_pool_is_plural(true);
            }
            if *while_on_top_of_library {
                grant_play = grant_play.while_on_top_of_library();
            }
            let mut effects = vec![Effect::new(grant_play)];
            if *without_paying_mana_cost {
                let mut grant_free_cast =
                    crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
                        resolved_tag,
                        player_filter,
                    );
                if *while_on_top_of_library {
                    grant_free_cast = grant_free_cast
                        .while_on_top_of_library()
                        .from_current_zone();
                }
                effects.push(Effect::new(grant_free_cast));
            }
            Ok((effects, Vec::new()))
        }
        SubjectVerbActionAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
            tag,
            player,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else if tag.as_str() == "__source_exiled__" {
                TagKey::from(ctx.last_exiled_collection_tag.clone().unwrap_or_else(|| {
                    format!(
                        "__sentence_helper_exiled_l0_s0_e{}",
                        ctx.id_gen_context().next_tag_id.saturating_sub(1)
                    )
                }))
            } else {
                tag.clone()
            };
            Ok((
                vec![Effect::new(
                    crate::effects::GrantTaggedSpellLifeCostByManaValueEffect::new(
                        resolved_tag,
                        player_filter,
                    ),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
            tag,
            player,
            allow_land,
            allow_any_color_for_cast,
            until_next_end_step,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else if tag.as_str() == "__source_exiled__" {
                TagKey::from(ctx.last_exiled_collection_tag.clone().unwrap_or_else(|| {
                    format!(
                        "__sentence_helper_exiled_l0_s0_e{}",
                        ctx.id_gen_context().next_tag_id.saturating_sub(1)
                    )
                }))
            } else {
                tag.clone()
            };
            let mut grant_play = crate::effects::GrantPlayTaggedEffect::new(
                resolved_tag.clone(),
                player_filter,
                if *until_next_end_step {
                    crate::effects::GrantPlayTaggedDuration::UntilYourNextEndStep
                } else {
                    crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd
                },
                *allow_land,
                *allow_any_color_for_cast,
            );
            if is_sentence_helper_exiled_collection_tag(resolved_tag.as_str())
                && ctx.last_exiled_collection_is_plural
            {
                grant_play = grant_play.cast_pool_is_plural(true);
            }
            Ok((vec![Effect::new(grant_play)], Vec::new()))
        }
        SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else if tag.as_str() == "__source_exiled__" {
                TagKey::from(ctx.last_exiled_collection_tag.clone().unwrap_or_else(|| {
                    format!(
                        "__sentence_helper_exiled_l0_s0_e{}",
                        ctx.id_gen_context().next_tag_id.saturating_sub(1)
                    )
                }))
            } else {
                tag.clone()
            };
            let mut grant_play = crate::effects::GrantPlayTaggedEffect::new(
                resolved_tag.clone(),
                player_filter.clone(),
                crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled,
                *allow_land,
                *allow_any_color_for_cast,
            );
            if is_sentence_helper_exiled_collection_tag(resolved_tag.as_str())
                && ctx.last_exiled_collection_is_plural
            {
                grant_play = grant_play.cast_pool_is_plural(true);
            }
            if let Some(filter) = filter.clone() {
                grant_play = grant_play.with_filter(filter);
            }
            let mut effects = vec![Effect::new(grant_play)];
            if *without_paying_mana_cost {
                effects.push(Effect::new(
                    crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
                        resolved_tag,
                        player_filter,
                    )
                    .for_as_long_as_exiled(),
                ));
            }
            Ok((effects, Vec::new()))
        }
        SubjectVerbActionAst::GrantPlayTaggedForAsLongAsYouControlSource {
            tag,
            player,
            allow_land,
            allow_any_color_for_cast,
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else if tag.as_str() == "__source_exiled__" {
                TagKey::from(ctx.last_exiled_collection_tag.clone().unwrap_or_else(|| {
                    format!(
                        "__sentence_helper_exiled_l0_s0_e{}",
                        ctx.id_gen_context().next_tag_id.saturating_sub(1)
                    )
                }))
            } else {
                tag.clone()
            };
            let mut grant_play = crate::effects::GrantPlayTaggedEffect::new(
                resolved_tag.clone(),
                player_filter,
                crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
                *allow_land,
                *allow_any_color_for_cast,
            );
            if is_sentence_helper_exiled_collection_tag(resolved_tag.as_str())
                && ctx.last_exiled_collection_is_plural
            {
                grant_play = grant_play.cast_pool_is_plural(true);
            }
            Ok((vec![Effect::new(grant_play)], Vec::new()))
        }
        SubjectVerbActionAst::ExileUntilSourceLeaves {
            target,
            face_down,
            all,
        } => {
            let (mut spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if *all && let ChooseSpec::Object(filter) = spec {
                spec = ChooseSpec::All(filter);
            }
            let mut effect = Effect::new(
                crate::effects::ExileUntilEffect::source_leaves(spec.clone())
                    .with_face_down(*face_down),
            );
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ReturnToBattlefield {
            target,
            tapped,
            transformed,
            converted,
            controller,
            count_value,
            as_aura,
            top_only,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let from_exile_tag = choose_spec_references_exiled_tag(&spec);
            let use_move_to_zone =
                from_exile_tag || !matches!(controller, ReturnControllerAst::Preserve);
            let implicit_chooser =
                if ctx.iterated_player && choose_spec_owned_by_iterated_player(&spec) {
                    PlayerFilter::IteratedPlayer
                } else {
                    PlayerFilter::You
                };
            let mut effects = Vec::new();
            let resolved_spec = if !spec.is_target() {
                match &spec {
                    ChooseSpec::Object(filter)
                        if filter.tagged_constraints.is_empty()
                            && filter.zone == Some(Zone::Graveyard) =>
                    {
                        let tag = ctx.next_tag("chosen_return");
                        ctx.last_object_tag = Some(tag.clone());
                        let mut choose = crate::effects::ChooseObjectsEffect::new(
                            filter.clone(),
                            1usize,
                            implicit_chooser.clone(),
                            tag.clone(),
                        );
                        if *top_only {
                            choose = choose.top_only();
                        }
                        effects.push(Effect::new(choose));
                        ChooseSpec::tagged(tag)
                    }
                    ChooseSpec::WithCount(inner, count)
                        if (count.is_single() || count_value.is_some())
                            && matches!(inner.base(), ChooseSpec::Object(filter) if filter.tagged_constraints.is_empty() && filter.zone == Some(Zone::Graveyard)) =>
                    {
                        let ChooseSpec::Object(filter) = inner.base() else {
                            unreachable!("guard ensures graveyard object base")
                        };
                        let tag = ctx.next_tag("chosen_return");
                        ctx.last_object_tag = Some(tag.clone());
                        let mut choose = crate::effects::ChooseObjectsEffect::new(
                            filter.clone(),
                            *count,
                            implicit_chooser.clone(),
                            tag.clone(),
                        );
                        if *top_only {
                            choose = choose.top_only();
                        }
                        effects.push(Effect::new(
                            choose.with_count_value_opt(count_value.clone()),
                        ));
                        ChooseSpec::tagged(tag)
                    }
                    _ => spec.clone(),
                }
            } else {
                spec.clone()
            };
            let resolved_spec = if !ctx.iterated_player
                && !ctx.iterated_object
                && ctx.last_object_tag.as_deref() == Some(IT_TAG)
                && *controller == ReturnControllerAst::Owner
                && matches!(resolved_spec.base(), ChooseSpec::Iterated)
            {
                ChooseSpec::Tagged(TagKey::from(IT_TAG))
            } else {
                resolved_spec
            };

            let mut aura_grant_effects = Vec::new();
            let mut effect = if use_move_to_zone {
                let move_back = crate::effects::MoveToZoneEffect::new(
                    resolved_spec.clone(),
                    Zone::Battlefield,
                    false,
                );
                let move_back = if *tapped {
                    move_back.tapped()
                } else {
                    move_back
                };
                let move_back = match controller {
                    ReturnControllerAst::Preserve => move_back,
                    ReturnControllerAst::Owner => move_back.under_owner_control(),
                    ReturnControllerAst::You => move_back.under_you_control(),
                };
                Effect::new(move_back)
            } else {
                let mut effect =
                    Effect::return_from_graveyard_to_battlefield(resolved_spec.clone(), *tapped);
                if let Some(as_aura) = as_aura {
                    let mut attachment_filter = as_aura.attachment_filter.clone();
                    if !as_aura.granted_abilities.is_empty() {
                        let attachment_tag = TagKey::from("enchanted");
                        effects.push(Effect::choose_objects(
                            as_aura.attachment_filter.clone(),
                            1usize,
                            PlayerFilter::You,
                            attachment_tag.clone(),
                        ));
                        attachment_filter = ObjectFilter::tagged(attachment_tag.clone());
                        let grant_target_filter = as_aura.attachment_filter.clone().match_tagged(
                            attachment_tag.clone(),
                            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                        );
                        for modification in
                            lower_granted_ability_grant_modifications(&as_aura.granted_abilities)?
                        {
                            aura_grant_effects.push(Effect::new(
                                crate::effects::ApplyContinuousEffect::with_spec(
                                    ChooseSpec::Object(grant_target_filter.clone()),
                                    modification,
                                    Until::Forever,
                                ),
                            ));
                        }
                    }
                    let mut return_effect =
                        crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
                            resolved_spec.clone(),
                            *tapped,
                        )
                        .as_aura(attachment_filter.clone());
                    if as_aura.remove_all_abilities {
                        return_effect =
                            return_effect.as_aura_removing_all_abilities(attachment_filter);
                    }
                    effect = Effect::new(return_effect);
                }
                effect
            };
            if ctx.auto_tag_object_targets && choose_spec_targets_object(&resolved_spec) {
                let tag = reserved_or_next_object_tag(ctx, "returned");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            effects.push(effect);
            effects.extend(aura_grant_effects);
            if *transformed {
                let transform_spec = if let Some(tag) = ctx.last_object_tag.clone() {
                    ChooseSpec::tagged(tag)
                } else {
                    resolved_spec.clone()
                };
                effects.push(Effect::transform(transform_spec));
            }
            if *converted {
                let convert_spec = if let Some(tag) = ctx.last_object_tag.clone() {
                    ChooseSpec::tagged(tag)
                } else {
                    resolved_spec.clone()
                };
                effects.push(Effect::convert(convert_spec));
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::ReturnAllToBattlefield {
            filter,
            tapped,
            face_down,
            controller,
            verb_surface,
        } => {
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let refers_to_milled_cards =
                resolved_filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                        && crate::runtime_backend::util::is_sentence_helper_tag(
                            constraint.tag.as_str(),
                            "milled",
                        )
                });
            if resolved_filter.zone == Some(Zone::Battlefield) && refers_to_milled_cards {
                // A tagged mill result is a graveyard snapshot. Some "cards
                // milled this way" subject shapes inherit the destination
                // battlefield zone while parsing the return action; restore
                // the provenance before runtime filtering.
                resolved_filter.zone = Some(Zone::Graveyard);
            }
            let return_all =
                crate::effects::ReturnAllToBattlefieldEffect::new(resolved_filter, *tapped)
                    .with_verb_surface(*verb_surface);
            let return_all = if *face_down {
                return_all.face_down()
            } else {
                return_all
            };
            let return_all = match controller {
                ReturnControllerAst::Preserve
                    if *verb_surface == ironsmith_core::MoveToZoneVerbSurface::Put
                        && matches!(player, PlayerAst::Implicit | PlayerAst::You) =>
                {
                    return_all.under_you_control_implicitly()
                }
                ReturnControllerAst::Preserve | ReturnControllerAst::Owner => {
                    return_all.under_owner_control()
                }
                ReturnControllerAst::You => return_all.under_you_control(),
            };
            let mut effect = Effect::new(return_all);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("returned");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::MoveToZone {
            target,
            zone,
            to_top,
            library_order,
            library_order_chooser,
            verb_surface,
            target_plural_surface,
            destination_player_surface,
            destination_player_reference_surface,
            exiled_with_source_surface,
            battlefield_controller,
            battlefield_tapped,
            battlefield_attacking,
            battlefield_attack_target_player_or_planeswalker_controlled_by,
            battlefield_face_down,
            attached_to,
            all,
        } => {
            // Inside an each-player reveal sequence, a bare "it" can arrive
            // from the generic subject parser as `Source`. Once a revealed
            // object tag exists, moving the source into every iterated
            // player's zone is not a coherent interpretation; the antecedent
            // is the card that player just revealed. Preserve that object
            // identity for both execution and compiled text.
            let revealed_target = if ctx.iterated_player
                && matches!(target, TargetAst::Source(_))
                && let Some(revealed_tag) = ctx.last_revealed_tag.as_deref()
            {
                Some(ChooseSpec::Tagged(TagKey::from(revealed_tag)))
            } else {
                None
            };
            let (mut spec, mut choices) = if let Some(spec) = revealed_target {
                (spec, Vec::new())
            } else {
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?
            };
            let resolved_library_order = match library_order {
                None => None,
                Some(crate::cards::builders::LibraryBottomOrderAst::Random) => {
                    Some(crate::effects::LibraryPlacementOrder::Random)
                }
                Some(crate::cards::builders::LibraryBottomOrderAst::ChooserChooses) => {
                    Some(crate::effects::LibraryPlacementOrder::ChosenBy(
                        resolve_non_target_player_filter(
                            *library_order_chooser,
                            &current_reference_env(ctx),
                        )?,
                    ))
                }
            };
            let actor_surface = if matches!(player, PlayerAst::Implicit) {
                None
            } else if matches!(player, PlayerAst::Target | PlayerAst::TargetOpponent) {
                None
            } else {
                Some(resolve_non_target_player_filter(
                    player,
                    &current_reference_env(ctx),
                )?)
            };
            let with_move_surfaces = |move_effect: crate::effects::MoveToZoneEffect| {
                let move_effect = if let Some(order) = resolved_library_order.clone() {
                    move_effect.with_library_order(order)
                } else {
                    move_effect
                };
                let move_effect = move_effect.with_verb_surface(*verb_surface);
                let move_effect = if *target_plural_surface {
                    move_effect.with_target_plural_surface()
                } else {
                    move_effect
                };
                if let Some(actor) = actor_surface.clone() {
                    move_effect.with_actor_surface(actor)
                } else {
                    move_effect
                }
            };
            if *all && let ChooseSpec::Object(filter) = spec {
                spec = ChooseSpec::All(filter);
            }
            if !ctx.iterated_player
                && !ctx.iterated_object
                && ctx.last_object_tag.as_deref() == Some(IT_TAG)
                && (ctx.last_it_choice_is_set
                    || (*zone == Zone::Battlefield
                        && *battlefield_controller
                            == crate::cards::builders::ReturnControllerAst::Owner))
                && matches!(spec.base(), ChooseSpec::Iterated)
            {
                spec = ChooseSpec::Tagged(TagKey::from(IT_TAG));
            }
            let resolved_attach_spec = if let Some(attach_target) = attached_to {
                if *zone != Zone::Battlefield {
                    return Err(CardTextError::ParseError(
                        "attached battlefield destination requires zone battlefield".to_string(),
                    ));
                }
                let (attach_spec, attach_choices) =
                    resolve_target_spec_with_choices(attach_target, &current_reference_env(ctx))?;
                for choice in attach_choices {
                    push_choice(&mut choices, choice);
                }
                Some(attach_spec)
            } else {
                None
            };
            if resolved_attach_spec.is_none()
                && *zone == Zone::Battlefield
                && let ChooseSpec::WithCount(inner, count) = &spec
                && !inner.is_target()
                && let ChooseSpec::Object(filter) = inner.base()
                && filter.zone == Some(Zone::Hand)
            {
                let chooser = filter
                    .owner
                    .clone()
                    .or_else(|| filter.controller.clone())
                    .unwrap_or(PlayerFilter::You);
                let chosen_tag = ctx.next_tag("chosen");
                let choose = Effect::new(
                    crate::effects::ChooseObjectsEffect::new(
                        filter.clone(),
                        count.clone(),
                        chooser,
                        chosen_tag.clone(),
                    )
                    .in_zone(Zone::Hand)
                    .replace_tagged_objects(),
                );
                let spec = ChooseSpec::tagged(chosen_tag);
                let move_effect = with_move_surfaces(crate::effects::MoveToZoneEffect::new(
                    spec.clone(),
                    *zone,
                    *to_top,
                ));
                let move_effect = if *zone == Zone::Battlefield && *battlefield_tapped {
                    move_effect.tapped()
                } else {
                    move_effect
                };
                let move_effect = if *zone == Zone::Battlefield && *battlefield_attacking {
                    move_effect.attacking()
                } else {
                    move_effect
                };
                let move_effect = if *zone == Zone::Battlefield
                    && let Some(attack_player) =
                        battlefield_attack_target_player_or_planeswalker_controlled_by
                {
                    let attack_player_filter = resolve_non_target_player_filter(
                        *attack_player,
                        &current_reference_env(ctx),
                    )?;
                    move_effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter)
                } else {
                    move_effect
                };
                let move_effect = if *zone == Zone::Battlefield && *battlefield_face_down {
                    move_effect.face_down()
                } else {
                    move_effect
                };
                let move_effect = match battlefield_controller {
                    ReturnControllerAst::Preserve => move_effect,
                    ReturnControllerAst::Owner => move_effect.under_owner_control(),
                    ReturnControllerAst::You => move_effect.under_you_control(),
                };
                let mut effect = Effect::new(move_effect);
                if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                    let tag = reserved_or_next_object_tag(ctx, "moved");
                    ctx.last_object_tag = Some(tag.clone());
                    effect = effect.tag(tag);
                }
                return Ok(Some((vec![choose, effect], choices)));
            }
            if resolved_attach_spec.is_none()
                && *zone == Zone::Library
                && !*to_top
                && let ChooseSpec::Object(filter) = spec.base()
                && filter.zone == Some(Zone::Exile)
            {
                let remainder_tag = ctx.last_exiled_collection_tag.clone().or_else(|| {
                    (ctx.last_object_tag.as_deref() == Some("__source_exiled__")).then(|| {
                        format!(
                            "__sentence_helper_exiled_l0_s0_e{}",
                            ctx.id_gen_context().next_tag_id.saturating_sub(1)
                        )
                    })
                });
                let Some(remainder_tag) = remainder_tag else {
                    let move_effect = with_move_surfaces(crate::effects::MoveToZoneEffect::new(
                        spec.clone(),
                        *zone,
                        *to_top,
                    ));
                    let move_effect = match battlefield_controller {
                        ReturnControllerAst::Preserve => move_effect,
                        ReturnControllerAst::Owner => move_effect.under_owner_control(),
                        ReturnControllerAst::You => move_effect.under_you_control(),
                    };
                    return Ok(Some((vec![Effect::new(move_effect)], choices)));
                };
                let library_owner = ctx.last_player_filter.clone().unwrap_or(PlayerFilter::You);
                return Ok(Some((
                    vec![Effect::put_tagged_remainder_on_library_bottom(
                        TagKey::from(remainder_tag.as_str()),
                        Some(TagKey::from("__source_exiled__")),
                        crate::effects::consult_helpers::LibraryBottomOrder::Random,
                        library_owner,
                    )],
                    choices,
                )));
            }
            if matches!(
                spec.base(),
                ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
            ) && let Some(tag) = ctx.last_exiled_collection_tag.clone()
            {
                spec = if ctx.last_exiled_collection_is_plural {
                    ChooseSpec::All(ObjectFilter::tagged(tag).in_zone(Zone::Exile))
                } else {
                    ChooseSpec::Tagged(TagKey::from(tag))
                };
            }
            if *zone != Zone::Battlefield
                && let ChooseSpec::Object(filter) = spec.base()
                && filter.zone == Some(Zone::Exile)
                && filter
                    .tagged_constraints
                    .iter()
                    .any(|constraint| constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG)
            {
                spec = ChooseSpec::All(filter.clone());
            }
            if *zone != Zone::Battlefield
                && matches!(
                    spec.base(),
                    ChooseSpec::Tagged(tag) if tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                )
            {
                spec = if let Some(tag) = ctx.last_exiled_collection_tag.clone() {
                    ChooseSpec::Tagged(TagKey::from(tag))
                } else {
                    ChooseSpec::All(
                        ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile),
                    )
                };
            }
            let move_effect = with_move_surfaces(crate::effects::MoveToZoneEffect::new(
                spec.clone(),
                *zone,
                *to_top,
            ));
            let move_effect = if let Some(surface) = exiled_with_source_surface {
                move_effect.with_exiled_with_source_surface(surface.clone())
            } else {
                move_effect
            };
            let move_effect = if let Some(destination_player) = destination_player_surface {
                move_effect.with_destination_player_surface(resolve_non_target_player_filter(
                    *destination_player,
                    &current_reference_env(ctx),
                )?)
            } else {
                move_effect
            };
            let move_effect = if let Some(surface) = destination_player_reference_surface {
                move_effect.with_destination_player_reference_surface(*surface)
            } else {
                move_effect
            };
            let move_effect = if *zone == Zone::Battlefield && *battlefield_tapped {
                move_effect.tapped()
            } else {
                move_effect
            };
            let move_effect = if *zone == Zone::Battlefield && *battlefield_attacking {
                move_effect.attacking()
            } else {
                move_effect
            };
            let move_effect = if *zone == Zone::Battlefield
                && let Some(attack_player) =
                    battlefield_attack_target_player_or_planeswalker_controlled_by
            {
                let attack_player_filter =
                    resolve_non_target_player_filter(*attack_player, &current_reference_env(ctx))?;
                move_effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter)
            } else {
                move_effect
            };
            let move_effect = if *zone == Zone::Battlefield && *battlefield_face_down {
                move_effect.face_down()
            } else {
                move_effect
            };
            let move_effect = match battlefield_controller {
                ReturnControllerAst::Preserve => move_effect,
                ReturnControllerAst::Owner => move_effect.under_owner_control(),
                ReturnControllerAst::You => move_effect.under_you_control(),
            };
            let mut effect = Effect::new(move_effect);
            let mut moved_tag: Option<String> = None;
            let moves_all_objects = matches!(spec.base(), ChooseSpec::All(_));
            let should_tag = (choose_spec_targets_object(&spec) || moves_all_objects)
                && (ctx.auto_tag_object_targets || attached_to.is_some());
            if should_tag {
                let tag = reserved_or_next_object_tag(ctx, "moved");
                moved_tag = Some(tag.clone());
                ctx.last_object_tag = Some(tag.clone());
                if *zone == Zone::Exile {
                    ctx.last_exiled_collection_tag = Some(tag.clone());
                    ctx.last_exiled_collection_is_plural =
                        choose_spec_may_hold_multiple_objects(&spec);
                }
                effect = if moves_all_objects {
                    effect.tag_all(tag)
                } else {
                    effect.tag(tag)
                };
            }

            if let Some(attach_spec) = resolved_attach_spec {
                let moved_tag = moved_tag.ok_or_else(|| {
                    CardTextError::ParseError(
                        "attached battlefield destination requires object-tagged move source"
                            .to_string(),
                    )
                })?;
                let moved_objects =
                    ChooseSpec::All(ObjectFilter::tagged(TagKey::from(moved_tag.as_str())));
                return Ok(Some((
                    vec![effect, Effect::attach_objects(moved_objects, attach_spec)],
                    choices,
                )));
            }

            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let chooser = match player {
                PlayerAst::Implicit | PlayerAst::You => PlayerFilter::You,
                PlayerAst::Target => PlayerFilter::Target(Box::new(PlayerFilter::Any)),
                PlayerAst::TargetOpponent => PlayerFilter::Target(Box::new(PlayerFilter::Opponent)),
                PlayerAst::ItsOwner => PlayerFilter::OwnerOf(crate::filter::ObjectRef::Target),
                PlayerAst::ItsController => {
                    PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target)
                }
                other => resolve_non_target_player_filter(other, &current_reference_env(ctx))?,
            };
            let mut effect = Effect::new(
                crate::effects::MoveToLibraryTopOrBottomChoiceEffect::new(spec.clone())
                    .with_chooser(chooser),
            );
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::TargetOnly { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if matches!(spec.base(), ChooseSpec::Source) {
                Ok((Vec::new(), choices))
            } else {
                let effect = tag_object_target_effect(
                    Effect::new(crate::effects::TargetOnlyEffect::new(spec.clone())),
                    &spec,
                    ctx,
                    "targeted",
                );
                Ok((vec![effect], choices))
            }
        }
        SubjectVerbActionAst::TagMatchingObjects { filter, zones, tag } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut effect =
                crate::effects::TagMatchingObjectsEffect::new(resolved_filter, tag.clone());
            if !zones.is_empty() {
                effect = effect.in_zones(zones.clone());
            }
            ctx.last_object_tag = Some(tag.as_str().to_string());
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        SubjectVerbActionAst::Pump {
            power,
            toughness,
            target,
            duration,
            condition,
            set_quantifier_surface,
        } => {
            let resolved_power = resolve_value_it_tag(power, &current_reference_env(ctx))?;
            let resolved_toughness = resolve_value_it_tag(toughness, &current_reference_env(ctx))?;
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec,
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: resolved_power.clone(),
                        toughness: resolved_toughness.clone(),
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .with_set_quantifier_surface(*set_quantifier_surface);
                if let Some(condition) = condition {
                    apply = apply.with_condition(condition.clone());
                }
                Effect::new(apply)
            })
        }
        SubjectVerbActionAst::SetBasePowerToughness {
            power,
            toughness,
            target,
            duration,
            set_quantifier_surface,
        } => compile_tagged_effect_for_target(target, ctx, "set_base_pt", |spec| {
            let resolved_power = bind_iterated_value_to_choose_spec(power, &spec);
            let resolved_toughness = bind_iterated_value_to_choose_spec(toughness, &spec);
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::SetPowerToughness {
                        power: resolved_power,
                        toughness: resolved_toughness,
                        sublayer: crate::continuous::PtSublayer::Setting,
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .with_set_quantifier_surface(*set_quantifier_surface)
                .resolve_set_pt_values_at_resolution(),
            )
        }),
        SubjectVerbActionAst::BecomeBasePtCreature {
            power,
            toughness,
            target,
            card_types,
            subtypes,
            subtype_families,
            colors,
            abilities,
            granted_abilities,
            preserve_other_types,
            type_retention_surface,
            duration,
        } => {
            let granted_modifications =
                lower_granted_ability_grant_modifications(granted_abilities)?;
            compile_tagged_effect_for_target(target, ctx, "animated_creature", |spec| {
                let resolved_power = bind_iterated_value_to_choose_spec(power, &spec);
                let resolved_toughness = bind_iterated_value_to_choose_spec(toughness, &spec);
                // CR 205.1b gives "artifact creature" an implicit preservation
                // exception even without an "in addition" clause.
                let implicitly_preserves_card_types = card_types.contains(&CardType::Artifact)
                    && card_types.contains(&CardType::Creature);
                let type_modification = if *preserve_other_types || implicitly_preserves_card_types
                {
                    crate::continuous::Modification::AddCardTypes(card_types.clone())
                } else {
                    crate::continuous::Modification::SetCardTypes(card_types.clone())
                };
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    type_modification,
                    duration.clone(),
                )
                .with_type_retention_surface(*type_retention_surface)
                .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
                    power: resolved_power,
                    toughness: resolved_toughness,
                    sublayer: crate::continuous::PtSublayer::Setting,
                })
                .resolve_set_pt_values_at_resolution();
                if let Some(colors) = colors {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::SetColors(*colors),
                    );
                }
                if !subtypes.is_empty() {
                    if !preserve_other_types {
                        apply = apply.with_additional_modification(
                            crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                                crate::types::SubtypeFamily::Creature,
                            ),
                        );
                    }
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::AddSubtypes(subtypes.clone()),
                    );
                }
                for ability in abilities {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::AddAbility(ability.clone()),
                    );
                }
                for modification in granted_modifications {
                    apply = apply.with_additional_modification(modification);
                }
                for family in subtype_families {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::AddAllSubtypesOfFamily(*family),
                    );
                }
                Effect::new(apply)
            })
        }
        SubjectVerbActionAst::SetBasePower {
            power,
            target,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_base_power", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    {
                        #[cfg(not(feature = "serialization"))]
                        {
                            crate::continuous::Modification::SetPower {
                                power: power.clone(),
                                sublayer: crate::continuous::PtSublayer::Setting,
                            }
                        }
                        #[cfg(feature = "serialization")]
                        {
                            crate::continuous::Modification::SetPower {
                                value: power.clone(),
                                sublayer: crate::continuous::PtSublayer::Setting,
                            }
                        }
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .resolve_set_pt_values_at_resolution(),
            )
        }),
        SubjectVerbActionAst::PumpForEach {
            power_per,
            toughness_per,
            target,
            count,
            duration,
        } => {
            let mut resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            if !ctx.iterated_player
                && let Some(last_player_filter) = ctx.last_player_filter.as_ref()
            {
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut resolved_count,
                    last_player_filter,
                );
            }
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::pump_for_each(
                    spec,
                    *power_per,
                    *toughness_per,
                    resolved_count.clone(),
                    duration.clone(),
                )
            })
        }
        SubjectVerbActionAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
            set_quantifier_surface,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let tag = ctx.next_tag("pumped");
            let effect = Effect::new(
                crate::effects::ApplyContinuousEffect::new_runtime(
                    crate::continuous::EffectTarget::Filter(resolved_filter.clone()),
                    crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                    },
                    duration.clone(),
                )
                .with_set_quantifier_surface(*set_quantifier_surface)
                .lock_filter_at_resolution(),
            )
            .tag_all(tag.clone());
            ctx.last_object_tag = Some(tag);
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::PumpByLastEffect {
            power,
            toughness,
            target,
            duration,
        } => {
            let id = ctx.last_effect_id.ok_or_else(|| {
                CardTextError::ParseError("missing prior effect for pump clause".to_string())
            })?;
            let power_value = if *power == 1 {
                Value::EffectValue(id)
            } else {
                Value::Fixed(*power)
            };
            compile_tagged_effect_for_target(target, ctx, "pumped", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        spec,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: power_value.clone(),
                            toughness: Value::Fixed(*toughness),
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })
        }
        SubjectVerbActionAst::AddCardTypes {
            target,
            card_types,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddCardTypes(card_types.clone()),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::SetCardTypes {
            target,
            card_types,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::SetCardTypes(card_types.clone()),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::RemoveCardTypes {
            target,
            card_types,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::RemoveCardTypes(card_types.clone()),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::AddSubtypes {
            target,
            subtypes,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "subtyped", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddSubtypes(subtypes.clone()),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::SetCreatureSubtypes {
            target,
            subtypes,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "subtyped", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::RemoveAllSubtypesOfFamily(
                        crate::types::SubtypeFamily::Creature,
                    ),
                    duration.clone(),
                )
                .with_additional_modification(
                    crate::continuous::Modification::AddSubtypes(subtypes.clone()),
                ),
            )
        }),
        SubjectVerbActionAst::BecomeSaddledUntilEndOfTurn { target } => {
            compile_tagged_effect_for_target(target, ctx, "saddled", |spec| {
                Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                    spec,
                    Effect::new(crate::effects::BecomeSaddledUntilEotEffect::new()),
                ))
            })
        }
        SubjectVerbActionAst::AddColors {
            target,
            colors,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "colored", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddColors(*colors),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::AddAllSubtypesOfFamily {
            target,
            family,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "subtyped", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::AddAllSubtypesOfFamily(*family),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::RemoveAllSubtypesOfFamily {
            target,
            family,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "subtyped", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::RemoveAllSubtypesOfFamily(*family),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::BecomeAuraEnchantment {
            target,
            attachment_filter,
            granted_abilities,
            duration,
        } => {
            let grant_modifications = lower_granted_ability_grant_modifications(granted_abilities)?;
            compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::AddCardTypes(vec![CardType::Enchantment]),
                    duration.clone(),
                )
                .with_additional_modification(crate::continuous::Modification::RemoveCardTypes(
                    vec![
                        CardType::Artifact,
                        CardType::Battle,
                        CardType::Creature,
                        CardType::Kindred,
                        CardType::Land,
                        CardType::Planeswalker,
                    ],
                ))
                .with_additional_modification(crate::continuous::Modification::AddSubtypes(vec![
                    Subtype::Aura,
                ]))
                .with_additional_runtime_modification(
                    crate::effects::continuous::RuntimeModification::SetAuraAttachmentFilter(
                        attachment_filter.clone().into(),
                    ),
                );
                for modification in grant_modifications {
                    apply = apply.with_additional_modification(modification);
                }
                Effect::new(apply)
            })
        }
        SubjectVerbActionAst::BecomeBasicLandType {
            target,
            subtype,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "become_basic_land_type", |spec| {
            Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::fixed(
                spec,
                *subtype,
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::SetColors {
            target,
            colors,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "set_colors", |spec| {
            Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                spec,
                crate::continuous::Modification::SetColors(*colors),
                duration.clone(),
            ))
        }),
        SubjectVerbActionAst::MakeColorless { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "set_colorless", |spec| {
                Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::MakeColorless,
                    duration.clone(),
                ))
            })
        }
        SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "become_basic_land_type", |spec| {
                Effect::new(crate::effects::BecomeBasicLandTypeChoiceEffect::new(
                    spec,
                    duration.clone(),
                ))
            })
        }
        SubjectVerbActionAst::BecomeCreatureTypeChoice {
            target,
            duration,
            excluded_subtypes,
        } => compile_tagged_effect_for_target(target, ctx, "become_creature_type_choice", |spec| {
            Effect::new(crate::effects::BecomeCreatureTypeChoiceEffect::new(
                spec,
                duration.clone(),
                excluded_subtypes.clone(),
            ))
        }),
        SubjectVerbActionAst::BecomeColorChoice { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "become_color_choice", |spec| {
                Effect::new(crate::effects::BecomeColorChoiceEffect::new(
                    spec,
                    duration.clone(),
                ))
            })
        }
        SubjectVerbActionAst::BecomeCopy {
            target,
            source,
            duration,
            preserve_source_abilities,
            name_override,
            name_override_surface,
            add_supertypes,
            remove_supertypes,
            granted_abilities,
            set_base_power_toughness,
        } => {
            let refs = current_reference_env(ctx);
            let (target_spec, mut choices) = resolve_target_spec_with_choices(target, &refs)?;
            let (source_spec, source_choices) = resolve_target_spec_with_choices(source, &refs)?;
            let source_spec = with_target_reference_surface_hint(source_spec, source);
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }

            let granted_modifications =
                lower_granted_ability_grant_modifications(granted_abilities)?;
            let mut apply = crate::effects::ApplyContinuousEffect::with_spec_runtime(
                target_spec.clone(),
                crate::effects::continuous::RuntimeModification::CopyOf {
                    source: source_spec,
                    preserve_source_abilities: *preserve_source_abilities,
                    name_override: name_override.clone(),
                    name_override_surface: name_override_surface.clone(),
                    add_supertypes: add_supertypes.clone(),
                },
                duration.clone(),
            );
            if !remove_supertypes.is_empty() {
                apply = apply.with_additional_modification(
                    crate::continuous::Modification::RemoveSupertypes(remove_supertypes.clone()),
                );
            }
            if let Some((power, toughness)) = set_base_power_toughness {
                apply = apply
                    .with_additional_modification(
                        crate::continuous::Modification::SetPowerToughness {
                            power: bind_iterated_value_to_choose_spec(power, &target_spec),
                            toughness: bind_iterated_value_to_choose_spec(toughness, &target_spec),
                            sublayer: crate::continuous::PtSublayer::Setting,
                        },
                    )
                    .resolve_set_pt_values_at_resolution();
            }
            for modification in granted_modifications {
                apply = apply.with_additional_modification(modification);
            }
            let effect = Effect::new(apply);
            let effect = tag_object_target_effect(effect, &target_spec, ctx, "copied");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
            condition,
            set_quantifier_surface,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesAll with no abilities"
                        .to_string(),
                ));
            }

            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_condition = condition
                .as_ref()
                .map(|condition| resolve_tagged_top_library_condition(condition, ctx))
                .transpose()?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                modifications[0].clone(),
                duration.clone(),
            )
            .with_set_quantifier_surface(*set_quantifier_surface)
            .lock_filter_at_resolution();

            for modification in modifications.iter().skip(1) {
                apply = apply.with_additional_modification(modification.clone());
            }
            if let Some(condition) = resolved_condition {
                apply = apply.with_condition(condition);
            }

            Ok((vec![Effect::new(apply)], Vec::new()))
        }
        SubjectVerbActionAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
            set_quantifier_surface,
        } => {
            let abilities = lower_granted_abilities_ast(abilities)?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if abilities.is_empty() {
                Ok((
                    vec![Effect::new(
                        crate::effects::ApplyContinuousEffect::new_runtime(
                            crate::continuous::EffectTarget::Filter(resolved_filter),
                            crate::effects::continuous::RuntimeModification::RemoveAllAbilities,
                            duration.clone(),
                        )
                        .with_set_quantifier_surface(*set_quantifier_surface)
                        .lock_filter_at_resolution(),
                    )],
                    Vec::new(),
                ))
            } else {
                let mut apply = crate::effects::ApplyContinuousEffect::new(
                    crate::continuous::EffectTarget::Filter(resolved_filter),
                    crate::continuous::Modification::RemoveAbility(abilities[0].clone().into()),
                    duration.clone(),
                )
                .with_set_quantifier_surface(*set_quantifier_surface)
                .lock_filter_at_resolution();

                for ability in abilities.iter().skip(1) {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::RemoveAbility(ability.clone().into()),
                    );
                }

                Ok((vec![Effect::new(apply)], Vec::new()))
            }
        }
        SubjectVerbActionAst::GrantAbilitiesChoiceAll {
            filter,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesChoiceAll with no abilities"
                        .to_string(),
                ));
            }
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let modes = modifications
                .iter()
                .map(|modification| EffectMode {
                    source_text: String::new(),
                    effects: vec![Effect::new(
                        crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Filter(resolved_filter.clone()),
                            modification.clone(),
                            duration.clone(),
                        )
                        .lock_filter_at_resolution(),
                    )],
                })
                .collect::<Vec<_>>();
            Ok((vec![Effect::choose_one(modes)], Vec::new()))
        }
        SubjectVerbActionAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
            condition,
            set_quantifier_surface,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            let Some(first_modification) = modifications.first() else {
                return compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                    Effect::new(crate::effects::TargetOnlyEffect::new(spec))
                })
                .map(Some);
            };
            let resolved_condition = condition
                .as_ref()
                .map(|condition| resolve_tagged_top_library_condition(condition, ctx))
                .transpose()?;

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let source_reference_surface = spec.source_reference_surface().cloned();
                let effect_spec = if matches!(spec.unhinted(), ChooseSpec::Source) {
                    spec.into_unhinted()
                } else {
                    spec
                };
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    effect_spec,
                    first_modification.clone(),
                    duration.clone(),
                )
                .with_set_quantifier_surface(*set_quantifier_surface);

                for modification in modifications.iter().skip(1) {
                    apply = apply.with_additional_modification(modification.clone());
                }
                if let Some(condition) = &resolved_condition {
                    apply = apply.with_condition(condition.clone());
                }
                if let Some(surface) = source_reference_surface {
                    apply = apply.with_source_reference_surface(surface);
                }

                Effect::new(apply)
            })
        }
        SubjectVerbActionAst::GrantToTarget {
            target,
            grantable,
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
            Effect::grant(grantable.clone(), spec, *duration)
        }),
        SubjectVerbActionAst::GrantBySpec {
            spec,
            player,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(&spec.filter, &current_reference_env(ctx))?;
            let player =
                resolve_non_target_player_filter(player.clone(), &current_reference_env(ctx))?;
            let mut resolved_spec = spec.clone();
            resolved_spec.filter = resolved_filter;
            Ok((
                vec![Effect::grant_by_spec(resolved_spec, player, *duration)],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::RemoveAbilitiesFromTarget {
            target,
            abilities,
            duration,
        } => {
            if abilities
                .iter()
                .any(|ability| matches!(ability, GrantedAbilityAst::ThisAbility))
            {
                if abilities.len() != 1 {
                    return Err(CardTextError::InvariantViolation(
                        "this ability removal cannot be combined with other abilities".to_string(),
                    ));
                }
                return compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                    Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        spec,
                        crate::effects::continuous::RuntimeModification::RemoveThisAbility,
                        duration.clone(),
                    ))
                })
                .map(Some);
            }
            let abilities = lower_granted_abilities_ast(abilities)?;
            let Some(first_ability) = abilities.first() else {
                return compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                    Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        spec,
                        crate::effects::continuous::RuntimeModification::RemoveAllAbilities,
                        duration.clone(),
                    ))
                })
                .map(Some);
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let source_reference_surface = spec.source_reference_surface().cloned();
                let effect_spec = if matches!(spec.unhinted(), ChooseSpec::Source) {
                    spec.into_unhinted()
                } else {
                    spec
                };
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    effect_spec,
                    crate::continuous::Modification::RemoveAbility(first_ability.clone().into()),
                    duration.clone(),
                );

                for ability in abilities.iter().skip(1) {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::RemoveAbility(ability.clone().into()),
                    );
                }

                if let Some(surface) = source_reference_surface {
                    apply = apply.with_source_reference_surface(surface);
                }

                Effect::new(apply)
            })
        }
        SubjectVerbActionAst::GrantAbilitiesChoiceToTarget {
            target,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                    Effect::new(crate::effects::TargetOnlyEffect::new(spec))
                })
                .map(Some);
            }

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let modes = abilities
                    .iter()
                    .zip(modifications.iter())
                    .map(|(ability, modification)| EffectMode {
                        source_text: granted_ability_mode_description(ability, &spec)
                            .unwrap_or_default(),
                        effects: vec![Effect::new(
                            crate::effects::ApplyContinuousEffect::with_spec(
                                spec.clone(),
                                modification.clone(),
                                duration.clone(),
                            ),
                        )],
                    })
                    .collect::<Vec<_>>();
                Effect::choose_one(modes)
            })
        }
        SubjectVerbActionAst::ConsultTopOfLibrary {
            player,
            mode,
            filter,
            stop_rule,
            max_exposed,
            all_tag,
            match_tag,
        } => {
            let subject = LoweredSubject::resolve_library_owner(*player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let resolved_filter =
                subject.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
            let resolved_all_tag = resolve_it_tag_key(all_tag, &current_reference_env(ctx))?;
            let resolved_match_tag = resolve_it_tag_key(match_tag, &current_reference_env(ctx))?;
            let resolved_stop_rule = match stop_rule {
                crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch => {
                    crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                }
                crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(value) => {
                    crate::effects::ConsultTopOfLibraryStopRule::MatchCount(
                        subject.resolve_object_refs_and_bind_player_refs_in_value(value, ctx)?,
                    )
                }
            };
            let resolved_max_exposed = max_exposed
                .as_ref()
                .map(|value| subject.resolve_object_refs_and_bind_player_refs_in_value(value, ctx))
                .transpose()?;
            let resolved_mode = match mode {
                crate::cards::builders::LibraryConsultModeAst::Reveal => {
                    crate::effects::consult_helpers::LibraryConsultMode::Reveal
                }
                crate::cards::builders::LibraryConsultModeAst::Exile => {
                    crate::effects::consult_helpers::LibraryConsultMode::Exile
                }
            };
            if matches!(mode, crate::cards::builders::LibraryConsultModeAst::Reveal) {
                ctx.last_revealed_tag = Some(resolved_all_tag.as_str().to_string());
                ctx.last_revealed_zone = Some(Zone::Library);
                ctx.last_revealed_player_filter = Some(player_filter.clone());
            }
            ctx.last_object_tag = Some(resolved_match_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter.clone());
            let mut consult = crate::effects::ConsultTopOfLibraryEffect::new(
                player_filter,
                resolved_mode,
                resolved_filter,
                resolved_stop_rule,
                resolved_all_tag,
                resolved_match_tag,
            );
            if let Some(max_exposed) = resolved_max_exposed {
                consult = consult.with_max_exposed(max_exposed);
            }
            Ok((vec![Effect::new(consult)], subject.into_choices()))
        }
        SubjectVerbActionAst::SearchLibrary {
            filter,
            destination,
            chooser,
            player,
            search_mode,
            reveal,
            shuffle,
            count,
            count_value,
            library_position_from_top,
            result_reference_surface,
            tapped,
        } => {
            let (chooser_filter, chooser_choices) = if matches!(*chooser, PlayerAst::Implicit)
                && matches!(*player, PlayerAst::That)
                && filter.owner.is_some()
                && ctx.last_player_filter.as_ref().is_some_and(|filter| {
                    !matches!(
                        filter,
                        PlayerFilter::IteratedPlayer | PlayerFilter::TaggedPlayer(_)
                    )
                }) {
                (PlayerFilter::You, Vec::new())
            } else {
                let subject = LoweredSubject::resolve_chooser(*chooser, ctx, true, true, true)?;
                (subject.into_player_filter(), subject.into_choices())
            };
            let subject = LoweredSubject::resolve_library_owner(*player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let count = *count;
            let filter = subject.bind_library_filter(filter, ctx)?;
            let mut choices = subject.into_choices();
            for choice in chooser_choices {
                push_choice(&mut choices, choice);
            }
            ctx.last_player_filter = Some(
                filter
                    .owner
                    .clone()
                    .unwrap_or_else(|| player_filter.clone()),
            );
            let use_search_effect = *shuffle
                && count.max == Some(1)
                && count_value.is_none()
                && *destination != Zone::Battlefield;
            if use_search_effect {
                let mut search_effect = crate::effects::SearchLibraryEffect::new(
                    filter,
                    *destination,
                    chooser_filter.clone(),
                    player_filter.clone(),
                    *reveal,
                )
                .with_search_mode(*search_mode);
                if let Some(position) = library_position_from_top.clone() {
                    search_effect = search_effect.with_library_position_from_top(position);
                }
                search_effect =
                    search_effect.with_result_reference_surface(*result_reference_surface);
                let mut effect = Effect::new(search_effect);
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("searched");
                    ctx.last_object_tag = Some(tag.clone());
                    effect = effect.tag(tag);
                }
                Ok((vec![effect], choices))
            } else {
                let tag = ctx.next_tag("searched");
                if ctx.auto_tag_object_targets {
                    ctx.last_object_tag = Some(tag.clone());
                }
                let mut generic_search_filter = ObjectFilter::default();
                generic_search_filter.owner = filter.owner.clone();
                let choose_description = if filter == generic_search_filter {
                    if count.max == Some(1) {
                        "card"
                    } else {
                        "cards"
                    }
                } else {
                    "objects"
                };
                let choose = crate::effects::ChooseObjectsEffect::new(
                    filter,
                    count,
                    chooser_filter.clone(),
                    tag.clone(),
                )
                .with_count_value_opt(count_value.clone())
                .in_zone(Zone::Library)
                .with_description(choose_description);
                let choose = match search_mode {
                    crate::effect::SearchSelectionMode::Exact => choose.as_search(),
                    crate::effect::SearchSelectionMode::Optional => choose.as_optional_search(),
                    crate::effect::SearchSelectionMode::AllMatching => {
                        choose.as_all_matching_search()
                    }
                };
                let choose = if *reveal { choose.reveal() } else { choose };

                let to_top = matches!(destination, Zone::Library);
                let move_effect = if *destination == Zone::Battlefield {
                    Effect::put_onto_battlefield(
                        ChooseSpec::Iterated,
                        *tapped,
                        player_filter.clone(),
                    )
                } else {
                    Effect::move_to_zone(ChooseSpec::Iterated, *destination, to_top)
                };
                let mut sequence_effects = vec![Effect::new(choose)];
                if *shuffle && *destination == Zone::Library {
                    sequence_effects.push(Effect::shuffle_library_player(player_filter.clone()));
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                } else {
                    sequence_effects.push(Effect::for_each_tagged(tag, vec![move_effect]));
                    if *shuffle {
                        sequence_effects.push(Effect::shuffle_library_player(player_filter));
                    }
                }
                let sequence = crate::effects::SequenceEffect::new(sequence_effects);
                Ok((vec![Effect::new(sequence)], std::mem::take(&mut choices)))
            }
        }
        SubjectVerbActionAst::Cant {
            restriction,
            duration,
            condition,
        } => {
            let restriction = resolve_restriction_it_tag(restriction, &current_reference_env(ctx))?;
            if let Some(condition) = condition {
                match &restriction {
                    crate::effect::Restriction::Untap(filter) => {
                        let apply = crate::effects::ApplyContinuousEffect::new(
                            crate::continuous::EffectTarget::Filter(filter.clone()),
                            crate::continuous::Modification::DoesntUntap,
                            duration.clone(),
                        )
                        .with_condition(condition.clone())
                        .lock_filter_at_resolution();
                        Ok((vec![Effect::new(apply)], Vec::new()))
                    }
                    other => Err(CardTextError::ParseError(format!(
                        "unsupported conditioned restriction: {other:?}"
                    ))),
                }
            } else {
                Ok((
                    vec![Effect::cant_until(restriction, duration.clone())],
                    Vec::new(),
                ))
            }
        }
        SubjectVerbActionAst::CreateTokenWithMods {
            name,
            definition,
            count,
            dynamic_power_toughness,
            player: action_player,
            attached_to,
            tapped,
            attacking,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            next_end_step_player,
            granted_abilities,
            ability_presentation,
        } => {
            let mut token = lower_token_definition_shape(definition.clone())
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            for ability in lower_granted_abilities_ast_to_object_abilities(granted_abilities)? {
                if !token.abilities.contains(&ability) {
                    token.abilities.push(ability);
                }
            }
            let subject = LoweredSubject::resolve_actor(*action_player, ctx, true, true, true)?;
            let count = subject.resolve_object_refs_and_bind_player_refs_in_value(count, ctx)?;
            let player_filter = subject.clone_player_filter();
            let count = per_player_partition_value_for_filter(count, &player_filter);
            let mut choices = subject.into_choices();
            let mut effect = if matches!(player_filter, PlayerFilter::You) {
                crate::effects::CreateTokenEffect::you(token, count.clone())
            } else {
                crate::effects::CreateTokenEffect::new(token, count.clone(), player_filter.clone())
            };
            if let Some(presentation) = ability_presentation {
                effect = effect.with_ability_presentation(*presentation);
            }
            if *tapped {
                effect = effect.tapped();
            }
            if *attacking {
                effect = effect.attacking();
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_end_of_combat();
            }
            if *sacrifice_at_end_of_combat {
                effect = effect.sacrifice_at_end_of_combat();
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step();
            }
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step();
            }
            effect = effect.next_end_step_player(next_end_step_player.clone());
            if attached_to.is_some() {
                effect = effect.suppress_aura_attachment_choice();
            }
            let mut effect = Effect::new(effect);
            let resolved_dynamic_pt = dynamic_power_toughness
                .as_ref()
                .map(|(power, toughness)| {
                    let power = bind_explicit_that_card_token_stat_reference(power, ctx);
                    let toughness = bind_explicit_that_card_token_stat_reference(toughness, ctx);
                    Ok::<_, CardTextError>((
                        resolve_value_it_tag(&power, &current_reference_env(ctx))?,
                        resolve_value_it_tag(&toughness, &current_reference_env(ctx))?,
                    ))
                })
                .transpose()?;
            let resolved_attached_to = attached_to
                .as_ref()
                .map(|target| resolve_target_spec_with_choices(target, &current_reference_env(ctx)))
                .transpose()?;
            let needs_created_tag = ctx.auto_tag_object_targets
                || attached_to.is_some()
                || resolved_dynamic_pt.is_some();
            let mut created_tag: Option<String> = None;
            if needs_created_tag {
                let tag = ctx.next_tag("created");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag.clone());
                created_tag = Some(tag);
            }

            let mut compiled = vec![effect];
            if let Some((power, toughness)) = resolved_dynamic_pt {
                let Some(created_tag) = created_tag.clone() else {
                    return Err(CardTextError::InvariantViolation(
                        "dynamic token pt requires created token tag to be present".to_string(),
                    ));
                };
                compiled.extend(
                    compile_effect_for_target(
                        &TargetAst::Tagged(TagKey::from(created_tag.as_str()), None),
                        ctx,
                        |spec| {
                            Effect::set_base_power_toughness(
                                power.clone(),
                                toughness.clone(),
                                spec,
                                Until::Forever,
                            )
                        },
                    )?
                    .0,
                );
            }
            if let Some((target_spec, target_choices)) = resolved_attached_to {
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                let Some(created_tag) = created_tag else {
                    return Err(CardTextError::InvariantViolation(
                        "attached token creation requires created token tag to be present"
                            .to_string(),
                    ));
                };
                let objects = ChooseSpec::All(ObjectFilter::tagged(created_tag));
                let mut attach_effect = Effect::attach_objects(objects, target_spec);
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("attachment_target");
                    attach_effect = attach_effect.tag(tag.clone());
                    ctx.last_object_tag = Some(tag);
                }
                compiled.push(attach_effect);
            }
            Ok((compiled, choices))
        }
        SubjectVerbActionAst::CreateTokenCopy {
            object,
            count,
            player: action_player,
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
            half_power_toughness_round_up,
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            sacrifice_at_next_end_step_ability_text,
            exile_at_next_end_step,
            next_end_step_player,
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        } => {
            let ObjectRefAst::Tagged(tag) = object;
            let tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let subject = LoweredSubject::resolve_actor(*action_player, ctx, true, true, true)?;
            let count = subject.resolve_object_refs_and_bind_player_refs_in_value(count, ctx)?;
            let player_filter = subject.into_player_filter();
            let choices = subject.into_choices();
            let mut effect = crate::effects::CreateTokenCopyEffect::new(
                ChooseSpec::Tagged(tag),
                count,
                player_filter,
            );
            if *enters_tapped {
                effect = effect.enters_tapped(true);
            }
            if *enters_attacking {
                effect = effect.attacking(true);
            }
            if let Some(attack_player) = attack_target_player_or_planeswalker_controlled_by {
                let attack_player_filter =
                    resolve_non_target_player_filter(*attack_player, &current_reference_env(ctx))?;
                effect =
                    effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter);
            }
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            if *has_haste {
                effect = effect.haste(true);
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_eoc(true);
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step(true);
            }
            effect = effect.sacrifice_at_next_end_step_ability_text(
                sacrifice_at_next_end_step_ability_text.clone(),
            );
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
            effect = effect.next_end_step_player(next_end_step_player.clone());
            if let Some(colors) = set_colors {
                effect = effect.set_colors(*colors);
            }
            if let Some(card_types) = set_card_types {
                effect = effect.set_card_types(card_types.clone());
            }
            if let Some(subtypes) = set_subtypes {
                effect = effect.set_subtypes(subtypes.clone());
            }
            for card_type in added_card_types {
                effect = effect.added_card_type(*card_type);
            }
            for subtype in added_subtypes {
                effect = effect.added_subtype(*subtype);
            }
            for supertype in removed_supertypes {
                effect = effect.removed_supertype(*supertype);
            }
            if let Some((power, toughness)) = set_base_power_toughness {
                effect = effect.set_base_power_toughness(*power, *toughness);
            }
            for ability in granted_abilities {
                effect = effect.grant_static_ability(ability.clone());
            }
            let mut effect = Effect::new(effect);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::CreateTokenCopyFromSource {
            source,
            count,
            player: action_player,
            enters_tapped,
            enters_attacking,
            attack_target_player_or_planeswalker_controlled_by,
            half_power_toughness_round_up,
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            sacrifice_at_next_end_step_ability_text,
            exile_at_next_end_step,
            next_end_step_player,
            set_colors,
            set_card_types,
            set_subtypes,
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        } => {
            let subject = LoweredSubject::resolve_actor(*action_player, ctx, true, true, true)?;
            let count = subject.resolve_object_refs_and_bind_player_refs_in_value(count, ctx)?;
            let player_filter = subject.into_player_filter();
            let mut choices = subject.into_choices();
            let (mut source_spec, source_choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }
            if let Some(last_tag) = ctx.last_object_tag.as_deref()
                && is_exile_cost_collection_tag(last_tag)
                && let ChooseSpec::Object(filter) = &source_spec
                && filter.zone == Some(Zone::Exile)
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                        && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                })
            {
                source_spec = ChooseSpec::Tagged(TagKey::from(last_tag));
            }
            source_spec = with_target_reference_surface_hint(source_spec, source);
            let mut effect =
                crate::effects::CreateTokenCopyEffect::new(source_spec, count, player_filter);
            if *enters_tapped {
                effect = effect.enters_tapped(true);
            }
            if *enters_attacking {
                effect = effect.attacking(true);
            }
            if let Some(attack_player) = attack_target_player_or_planeswalker_controlled_by {
                let attack_player_filter =
                    resolve_non_target_player_filter(*attack_player, &current_reference_env(ctx))?;
                effect =
                    effect.attacking_player_or_planeswalker_controlled_by(attack_player_filter);
            }
            if *half_power_toughness_round_up {
                effect = effect.half_power_toughness_round_up();
            }
            if *has_haste {
                effect = effect.haste(true);
            }
            if *exile_at_end_of_combat {
                effect = effect.exile_at_eoc(true);
            }
            if *sacrifice_at_next_end_step {
                effect = effect.sacrifice_at_next_end_step(true);
            }
            effect = effect.sacrifice_at_next_end_step_ability_text(
                sacrifice_at_next_end_step_ability_text.clone(),
            );
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
            effect = effect.next_end_step_player(next_end_step_player.clone());
            if let Some(colors) = set_colors {
                effect = effect.set_colors(*colors);
            }
            if let Some(card_types) = set_card_types {
                effect = effect.set_card_types(card_types.clone());
            }
            if let Some(subtypes) = set_subtypes {
                effect = effect.set_subtypes(subtypes.clone());
            }
            for card_type in added_card_types {
                effect = effect.added_card_type(*card_type);
            }
            for subtype in added_subtypes {
                effect = effect.added_subtype(*subtype);
            }
            for supertype in removed_supertypes {
                effect = effect.removed_supertype(*supertype);
            }
            if let Some((power, toughness)) = set_base_power_toughness {
                effect = effect.set_base_power_toughness(*power, *toughness);
            }
            for ability in granted_abilities {
                effect = effect.grant_static_ability(ability.clone());
            }

            let mut effect = Effect::new(effect);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Meld {
            result_name,
            enters_tapped,
            enters_attacking,
        } => Ok((
            vec![Effect::new(
                crate::effects::MeldEffect::new(result_name.clone())
                    .enters_tapped(*enters_tapped)
                    .enters_attacking(*enters_attacking),
            )],
            Vec::new(),
        )),
        SubjectVerbActionAst::SearchLibrarySlotsToHand {
            slots,
            destination,
            reveal,
            progress_tag,
        } => {
            let subject = LoweredSubject::resolve_library_owner(player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let resolved_slots = slots
                .iter()
                .map(|slot| {
                    let resolved_filter = subject
                        .resolve_object_refs_and_bind_player_refs_in_filter(&slot.filter, ctx)?;
                    Ok(if slot.optional {
                        crate::effects::SearchLibrarySlot::optional(resolved_filter)
                    } else {
                        crate::effects::SearchLibrarySlot::required(resolved_filter)
                    })
                })
                .collect::<Result<Vec<_>, CardTextError>>()?;
            let resolved_tag = resolve_it_tag_key(progress_tag, &current_reference_env(ctx))?;
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter.clone());
            Ok((
                vec![Effect::new(crate::effects::SearchLibrarySlotsEffect::new(
                    resolved_slots,
                    *destination,
                    player_filter.clone(),
                    player_filter,
                    *reveal,
                    resolved_tag,
                ))],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::RetargetStackObject {
            target,
            mode,
            require_change,
        } => {
            let refs = current_reference_env(ctx);
            let (spec, mut choices) =
                if retarget_target_is_bare_it(target) && refs.has_source_object_antecedent() {
                    (ChooseSpec::Source, Vec::new())
                } else {
                    resolve_target_spec_with_choices(target, &refs)?
                };
            let subject = LoweredSubject::resolve_chooser(player, ctx, true, true, true)?;
            for choice in subject.clone().into_choices() {
                push_choice(&mut choices, choice);
            }

            let mut effect = crate::effects::RetargetStackObjectEffect::new(spec.clone())
                .with_chooser(subject.into_player_filter());

            if *require_change {
                effect = effect.require_change();
            }

            let compiled_mode = match mode {
                RetargetModeAst::All => crate::effects::RetargetMode::All,
                RetargetModeAst::OneToFixed { target: fixed } => {
                    let (fixed_spec, fixed_choices) =
                        resolve_target_spec_with_choices(fixed, &current_reference_env(ctx))?;
                    for choice in fixed_choices {
                        push_choice(&mut choices, choice);
                    }
                    crate::effects::RetargetMode::OneToFixed(fixed_spec)
                }
            };
            effect = effect.with_mode(compiled_mode);

            let effect = tag_object_target_effect(Effect::new(effect), &spec, ctx, "retargeted");
            Ok((vec![effect], choices))
        }
        _ => return Ok(None),
    };
    result.map(Some)
}

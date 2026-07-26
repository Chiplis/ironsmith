use super::*;

fn attachment_reference_tag(spec: &ChooseSpec) -> Option<TagKey> {
    if spec.is_target() {
        return None;
    }
    match spec.base() {
        ChooseSpec::Tagged(tag) => Some(tag.clone()),
        ChooseSpec::Object(filter) => watch_tag_from_filter(filter),
        _ => None,
    }
}

fn return_graveyard_player_surface(
    target: &TargetAst,
    ctx: &EffectLoweringContext,
) -> Result<Option<PlayerFilter>, CardTextError> {
    let target = match target {
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => inner.as_ref(),
        target => target,
    };
    let TargetAst::Object(filter, _, _) = target else {
        return Ok(None);
    };
    if filter.zone != Some(Zone::Graveyard) {
        return Ok(None);
    }
    Ok(resolve_it_tag(filter, &current_reference_env(ctx))?.owner)
}

/// Preserve a mandatory complete-set discard after subject/player lowering.
///
/// `discard all <matching cards>` is parsed as both an eligible-card filter
/// and a `Value::Count` over that same filter. Player resolution can turn an
/// authored target-player reference into a follow-up alias on the eligible
/// filter. Apply that same canonical filter to the count so the runtime still
/// discards exactly the complete eligible set.
fn replace_complete_discard_count_filter(value: &mut Value, filter: &ObjectFilter) {
    match value {
        Value::SurfaceHinted { value, .. } => {
            replace_complete_discard_count_filter(value, filter);
        }
        Value::Count(count_filter) => *count_filter = filter.clone(),
        _ => {}
    }
}

pub(super) fn compile_subject_verb_late(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    let role = subject_verb_role(subject_verb.subject.role);
    let player = subject_verb.subject.player;
    let result = match &subject_verb.action {
        SubjectVerbActionAst::GrantAbilityToSource { ability, duration } => {
            let lowered = lower_parsed_ability(ability.clone())?;
            Ok((
                vec![Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec(
                        crate::target::ChooseSpec::Source,
                        crate::continuous::Modification::AddAbilityGeneric(lowered.into_runtime()),
                        duration.clone(),
                    ),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::TurnFaceUp { target } => {
            let (effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "turned_face_up", |spec| {
                    Effect::turn_face_up(spec)
                })?;
            Ok((effects, choices))
        }
        SubjectVerbActionAst::DealDamage {
            amount,
            target,
            unpreventable,
        } => {
            let mut resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
                && !ctx.iterated_player
            {
                bind_relative_iterated_player_in_value_to_player_filter(
                    &mut resolved_amount,
                    &PlayerFilter::Target(Box::new(filter.clone())),
                );
            }
            let (mut effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    if *unpreventable {
                        Effect::deal_unpreventable_damage(resolved_amount.clone(), spec)
                    } else {
                        Effect::deal_damage(resolved_amount.clone(), spec)
                    }
                })?;
            if let TargetAst::Player(filter, explicit_target_span) = target {
                ctx.last_player_filter = Some(if explicit_target_span.is_some() {
                    PlayerFilter::Target(Box::new(filter.clone()))
                } else {
                    as_followup_player_alias(filter.clone())
                });
            } else if let TargetAst::PlayerOrPlaneswalker(filter, _) = target {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if target_is_any_damage_target(target) {
                let tag = ctx.next_tag("damaged");
                ctx.last_object_tag = Some(tag.clone());
                if let Some(effect) = effects.pop() {
                    effects.push(effect.tag(tag));
                }
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::DealDamageEach { amount, filter } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let tag = ctx.next_tag("damaged");
            ctx.last_object_tag = Some(tag.clone());
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::deal_damage(resolved_amount, ChooseSpec::Iterated).tag(tag)],
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::DealDistributedDamage {
            amount,
            target,
            source,
            chooser,
        } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let (source_spec, source_choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            let (mut effects, choices) =
                compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                    Effect::new(
                        crate::effects::DealDistributedDamageEffect::new(
                            resolved_amount.clone(),
                            spec,
                        )
                        .with_source(source_spec.clone())
                        .with_chooser(chooser.clone()),
                    )
                })?;
            let mut choices = choices;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }
            if target_is_any_damage_target(target) {
                let tag = ctx.next_tag("damaged");
                ctx.last_object_tag = Some(tag.clone());
                if let Some(effect) = effects.pop() {
                    effects.push(effect.tag(tag));
                }
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::DealDamageEqualToPower {
            source,
            amount,
            target,
            unpreventable,
        } => {
            let (source_spec, mut choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
            // A bare "it" damage subject inside a becomes-blocked trigger
            // refers to the trigger's source; last-object memory is seeded
            // with the BLOCKER there (for "that creature" references), so
            // the pronoun must not inherit it as the damage source.
            let source_spec = if matches!(
                source,
                TargetAst::Tagged(tag, _) if tag.as_str() == crate::host::IT_TAG
            ) && matches!(
                &source_spec,
                ChooseSpec::Tagged(tag) if tag.as_str() == "blocking"
            ) {
                ChooseSpec::Source
            } else {
                source_spec
            };
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let mut damage_target_spec = if source == target {
                source_spec.clone()
            } else {
                let (target_spec, target_choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                for choice in target_choices {
                    push_choice(&mut choices, choice);
                }
                target_spec
            };

            let mut effects = Vec::new();
            let mut damage_source_spec = source_spec.clone();
            let per_target_source_spec = if source == target {
                ChooseSpec::Iterated
            } else {
                source_spec.clone()
            };
            let mut damage_amount = bind_source_value_to_damage_source(&amount, &source_spec);

            if source_spec.is_target() {
                let source_tag = ctx.next_tag("damage_source");
                effects.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(source_spec.clone()))
                        .tag(source_tag.clone()),
                );
                damage_source_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                damage_amount = bind_source_value_to_damage_source(&amount, &damage_source_spec);
                if source == target {
                    damage_target_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                }
            }

            if !damage_target_spec.is_target()
                && let ChooseSpec::Object(filter) | ChooseSpec::All(filter) =
                    damage_target_spec.base()
            {
                let damage = if *unpreventable {
                    Effect::deal_unpreventable_damage(
                        bind_source_value_to_damage_source(&amount, &per_target_source_spec),
                        ChooseSpec::Iterated,
                    )
                } else {
                    Effect::deal_damage(
                        bind_source_value_to_damage_source(&amount, &per_target_source_spec),
                        ChooseSpec::Iterated,
                    )
                };
                let mut per_target_damage =
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        per_target_source_spec.clone(),
                        damage,
                    ));
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("damaged");
                    ctx.last_object_tag = Some(tag.clone());
                    per_target_damage = per_target_damage.tag(tag);
                }
                effects.push(Effect::for_each(filter.clone(), vec![per_target_damage]));
            } else {
                let damage = if *unpreventable {
                    Effect::deal_unpreventable_damage(
                        damage_amount.clone(),
                        damage_target_spec.clone(),
                    )
                } else {
                    Effect::deal_damage(damage_amount.clone(), damage_target_spec.clone())
                };
                let damage_effect = tag_object_target_effect(
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        damage_source_spec.clone(),
                        damage,
                    )),
                    &damage_target_spec,
                    ctx,
                    "damaged",
                );
                effects.push(damage_effect);
            }

            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            } else if matches!(
                target,
                TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_)
            ) {
                ctx.last_player_filter = Some(PlayerFilter::DamagedPlayer);
            }

            Ok((effects, choices))
        }
        SubjectVerbActionAst::Tap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::tap(spec.clone())
            } else {
                Effect::new(crate::effects::TapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "tapped");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Untap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::untap(spec.clone())
            } else {
                Effect::new(crate::effects::UntapEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "untapped");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::TapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("tapped");
                prelude.push(Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                    resolved_filter.clone(),
                    tag.clone(),
                )));
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(Effect::tap_all(resolved_filter));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::UntapAll { filter } => {
            let refs = current_reference_env(ctx);
            let unresolved_demonstrative_set = refs.known_last_object_tag().is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let resolved_filter = resolve_it_tag(filter, &refs)?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("untapped");
                prelude.push(Effect::new(crate::effects::TagMatchingObjectsEffect::new(
                    resolved_filter.clone(),
                    tag.clone(),
                )));
                ctx.last_object_tag = Some(tag);
            }
            // If "those permanents" arrived without a usable antecedent, do
            // not broaden it into every matching permanent.  A single
            // non-target choice is the conservative executable fallback and
            // preserves the old surface until the missing choice loop is
            // represented explicitly.
            if unresolved_demonstrative_set {
                prelude.push(Effect::untap(ChooseSpec::Object(resolved_filter)));
            } else {
                prelude.push(Effect::untap_all(resolved_filter));
            }
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::TapOrUntap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let modes = vec![
                EffectMode {
                    source_text: "Tap".to_string(),
                    effects: vec![Effect::tap(spec.clone())],
                },
                EffectMode {
                    source_text: "Untap".to_string(),
                    effects: vec![Effect::untap(spec.clone())],
                },
            ];
            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "tap_or_untap");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::TapOrUntapAll {
            tap_filter,
            untap_filter,
        } => {
            let resolved_tap = resolve_it_tag(tap_filter, &current_reference_env(ctx))?;
            let resolved_untap = resolve_it_tag(untap_filter, &current_reference_env(ctx))?;
            let (mut prelude, mut choices) = target_context_prelude_for_filter(&resolved_tap);
            let (_, untap_choices) = target_context_prelude_for_filter(&resolved_untap);
            for choice in untap_choices {
                push_choice(&mut choices, choice);
            }
            let modes = vec![
                EffectMode {
                    source_text: "Tap".to_string(),
                    effects: vec![Effect::tap_all(resolved_tap)],
                },
                EffectMode {
                    source_text: "Untap".to_string(),
                    effects: vec![Effect::untap_all(resolved_untap)],
                },
            ];
            prelude.push(Effect::choose_one(modes));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PhaseOut {
            target,
            duration,
            source_surface,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut phase_out = crate::effects::PhaseOutEffect::with_spec(spec.clone());
            phase_out.duration = *duration;
            phase_out.source_surface = source_surface.clone();
            let base_effect = Effect::new(phase_out);
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "phased_out");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PhaseOutAll {
            filter,
            duration,
            source_surface,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut phase_out =
                crate::effects::PhaseOutEffect::with_spec(ChooseSpec::all(resolved_filter));
            phase_out.duration = *duration;
            phase_out.source_surface = source_surface.clone();
            prelude.push(Effect::new(phase_out));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PhaseIn { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::phase_in(spec.clone())
            } else {
                Effect::new(crate::effects::PhaseInEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "phased_in");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PhaseInAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::new(crate::effects::PhaseInEffect::with_spec(
                ChooseSpec::all(resolved_filter),
            )));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::Transform { target } => {
            compile_tagged_effect_for_target(target, ctx, "transformed", Effect::transform)
        }
        SubjectVerbActionAst::Convert { target } => {
            compile_tagged_effect_for_target(target, ctx, "converted", Effect::convert)
        }
        SubjectVerbActionAst::Destroy {
            target,
            no_regeneration,
        } => compile_tagged_effect_for_target(target, ctx, "destroyed", |spec| {
            if *no_regeneration {
                Effect::new(crate::effects::DestroyNoRegenerationEffect::with_spec(spec))
            } else {
                Effect::new(crate::effects::DestroyEffect::with_spec(spec))
            }
        }),
        SubjectVerbActionAst::DestroyAll {
            filter,
            no_regeneration,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut effect = if *no_regeneration {
                Effect::new(crate::effects::DestroyNoRegenerationEffect::all(
                    resolved_filter,
                ))
            } else {
                Effect::destroy_all(resolved_filter)
            };
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(effect);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::DestroyAllOfChosenColor {
            filter,
            no_regeneration,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ];
            let auto_tag = if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                ctx.last_object_tag = Some(tag.clone());
                Some(tag)
            } else {
                None
            };
            for color in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = if *no_regeneration {
                    format!(
                        "Destroy all {}. They can't be regenerated.",
                        filter.description()
                    )
                } else {
                    format!("Destroy all {}.", filter.description())
                };
                let mut effect = if *no_regeneration {
                    Effect::new(crate::effects::DestroyNoRegenerationEffect::all(filter))
                } else {
                    Effect::destroy_all(filter)
                };
                if let Some(tag) = &auto_tag {
                    effect = effect.tag(tag.clone());
                }
                modes.push(EffectMode {
                    source_text: description,
                    effects: vec![effect],
                });
            }
            prelude.push(Effect::choose_one(modes));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::DestroyAllAttachedTo { filter, target } => {
            let (target_spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut prelude = Vec::new();
            let mut choices = choices;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(player_filter) = match target_spec.base() {
                ChooseSpec::Player(player_filter) => Some(player_filter.clone()),
                ChooseSpec::SourceController => Some(PlayerFilter::You),
                _ => None,
            } {
                resolved_filter.attached_to_player = Some(player_filter);
                ctx.last_object_tag = None;
            } else {
                let target_tag = if let Some(tag) = attachment_reference_tag(&target_spec) {
                    tag.as_str().to_string()
                } else {
                    if !choose_spec_targets_object(&target_spec) || !target_spec.is_target() {
                        return Err(CardTextError::ParseError(
                            "destroy-attached target must be an object, player, or tagged object"
                                .to_string(),
                        ));
                    }
                    let tag = ctx.next_tag("attachment_target");
                    prelude.push(
                        Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
                            .tag(tag.clone()),
                    );
                    tag
                };
                ctx.last_object_tag = Some(target_tag.clone());

                resolved_filter
                    .tagged_constraints
                    .push(TaggedObjectConstraint {
                        tag: TagKey::from(target_tag.as_str()),
                        relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                    });
            }

            let (mut filter_prelude, filter_choices) =
                target_context_prelude_for_filter(&resolved_filter);
            for choice in filter_choices {
                push_choice(&mut choices, choice);
            }

            let mut effect = Effect::destroy_all(resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("destroyed");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            prelude.append(&mut filter_prelude);
            prelude.push(effect);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::ExileAllAttachedTo {
            filter,
            target,
            face_down,
        } => {
            let (target_spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut prelude = Vec::new();
            let mut choices = choices;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let target_tag = if let Some(tag) = attachment_reference_tag(&target_spec) {
                tag.as_str().to_string()
            } else {
                if !choose_spec_targets_object(&target_spec) || !target_spec.is_target() {
                    return Err(CardTextError::ParseError(
                        "exile-attached target must be a target object or tagged object"
                            .to_string(),
                    ));
                }
                let tag = ctx.next_tag("attachment_target");
                prelude.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(target_spec.clone()))
                        .tag(tag.clone()),
                );
                tag
            };
            ctx.last_object_tag = Some(target_tag.clone());

            resolved_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(target_tag.as_str()),
                    relation: TaggedOpbjectRelation::AttachedToTaggedObject,
                });

            let (mut filter_prelude, filter_choices) =
                target_context_prelude_for_filter(&resolved_filter);
            for choice in filter_choices {
                push_choice(&mut choices, choice);
            }
            prelude.append(&mut filter_prelude);
            prelude.push(Effect::new(
                crate::effects::ExileEffect::all(resolved_filter).with_face_down(*face_down),
            ));

            let tagged_target = ChooseSpec::Tagged(TagKey::from(target_tag.as_str()));
            let target_exile = if *face_down {
                Effect::new(
                    crate::effects::ExileEffect::with_spec(tagged_target).with_face_down(true),
                )
            } else {
                Effect::move_to_zone(tagged_target, Zone::Exile, true)
            };
            prelude.push(target_exile);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::Exile {
            target,
            face_down,
            source_top_only,
        } => {
            if *source_top_only {
                let (spec, choices) =
                    resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                let collection_is_plural = !spec.count().is_single();
                let (choose, chosen_spec) =
                    lower_source_top_only_choice(&spec, player, ctx, "chosen_top")?;
                let chosen_tag = match chosen_spec.base() {
                    ChooseSpec::Tagged(tag) => tag.clone(),
                    _ => unreachable!("ordered source choice always lowers to a tagged object"),
                };
                ctx.last_exiled_collection_tag = Some(chosen_tag.as_str().to_string());
                ctx.last_exiled_collection_is_plural = collection_is_plural;
                let exile = Effect::new(
                    crate::effects::ExileEffect::with_spec(chosen_spec).with_face_down(*face_down),
                );
                return Ok(Some((vec![choose, exile], choices)));
            }
            if let Some(compiled) = lower_hand_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_counted_non_target_exile_target(target, *face_down, ctx)?
            {
                return Ok(Some(compiled));
            }
            if let Some(compiled) = lower_single_non_target_exile_target(target, *face_down, ctx)? {
                return Ok(Some(compiled));
            }
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = if spec.count().is_single() && !*face_down {
                Effect::move_to_zone(spec.clone(), Zone::Exile, true)
            } else {
                Effect::new(
                    crate::effects::ExileEffect::with_spec(spec.clone()).with_face_down(*face_down),
                )
            };
            if ctx.auto_tag_object_targets {
                if let ChooseSpec::Tagged(tag) = spec.base()
                    && is_sentence_helper_exiled_collection_tag(tag.as_str())
                {
                    effect = effect.tag(tag.clone());
                    ctx.last_object_tag = Some(tag.as_str().to_string());
                } else if spec.is_target() {
                    let tag = ctx.next_tag("exiled");
                    effect = effect.tag(tag.clone());
                    ctx.last_object_tag = Some(tag);
                } else if choose_spec_targets_object(&spec)
                    || matches!(spec.base(), ChooseSpec::Source)
                {
                    // MoveToZone/Exile populate the source-exiled link without
                    // needing a second runtime tag wrapper.
                    ctx.last_object_tag = Some(crate::tag::SOURCE_EXILED_TAG.to_string());
                }
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ExileAll { filter, face_down } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if let Some(player_filter) = infer_player_filter_from_object_filter(&resolved_filter) {
                ctx.last_player_filter = Some(player_filter);
            }
            let keep_last_object_tag =
                resolved_filter.tagged_constraints.iter().any(|constraint| {
                    matches!(
                        constraint.relation,
                        crate::filter::TaggedOpbjectRelation::SameNameAsTagged
                    )
                });
            let mut effect = Effect::new(
                crate::effects::ExileEffect::all(resolved_filter).with_face_down(*face_down),
            );
            if ctx.auto_tag_object_targets {
                if keep_last_object_tag {
                    if let Some(tag) = ctx.last_object_tag.clone() {
                        effect = effect.tag(tag);
                    }
                } else {
                    let tag = ctx.next_tag("exiled");
                    effect = effect.tag(tag.clone());
                    ctx.last_exiled_collection_tag = Some(tag.clone());
                    ctx.last_exiled_collection_is_plural = true;
                    ctx.last_object_tag = Some(tag);
                }
            }
            prelude.push(effect);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::LookAtHand { target } => {
            let refs = current_reference_env(ctx);
            let (spec, choices) = resolve_target_spec_with_choices(target, &refs)?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::LookAtHandEffect::new(spec.clone())),
                &spec,
                ctx,
                "targeted",
            );
            match spec.unhinted() {
                ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
                    ctx.last_player_filter = Some(filter.clone());
                }
                ChooseSpec::Target(inner) => match inner.unhinted() {
                    ChooseSpec::Player(filter) | ChooseSpec::PlayerOrPlaneswalker(filter) => {
                        ctx.last_player_filter =
                            Some(PlayerFilter::Target(Box::new(filter.clone())));
                    }
                    _ => {}
                },
                _ => {}
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Counter { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect =
                tag_object_target_effect(Effect::counter(spec.clone()), &spec, ctx, "countered");
            if let Some(tag) = ctx.last_object_tag.clone() {
                ctx.last_player_filter = Some(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)));
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::CounterUnlessPays { target, cost } => {
            let cost = resolve_total_cost_it_tags(cost, &current_reference_env(ctx))?;
            let compiled = compile_tagged_effect_for_target(target, ctx, "countered", |spec| {
                Effect::counter_unless_pays_total_cost(spec, cost.clone())
            })?;
            if let Some(tag) = ctx.last_object_tag.clone() {
                ctx.last_player_filter = Some(PlayerFilter::ControllerOf(ObjectRef::tagged(tag)));
            }
            Ok(compiled)
        }
        SubjectVerbActionAst::PutCounters {
            counter_type,
            count,
            target,
            target_count,
            distributed,
        } => {
            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }
            let mut put_counters =
                crate::effects::PutCountersEffect::new(*counter_type, resolved_count, spec.clone());
            if let Some(target_count) = target_count {
                put_counters = put_counters.with_target_count(*target_count);
            }
            if *distributed {
                put_counters = put_counters.with_distributed(true);
            }
            let effect =
                tag_object_target_effect(Effect::new(put_counters), &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PutCounterChoice {
            counter_types,
            count,
            mode_texts,
            target,
            target_count,
        } => {
            use crate::effect::EffectMode;

            let (base_spec, _) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }

            let modes = counter_types
                .iter()
                .enumerate()
                .map(|(idx, counter_type)| EffectMode {
                    source_text: mode_texts
                        .get(idx)
                        .cloned()
                        .unwrap_or_else(|| format!("Put a {} counter", counter_type.description())),
                    effects: vec![Effect::put_counters(
                        *counter_type,
                        resolved_count.clone(),
                        spec.clone(),
                    )],
                })
                .collect();

            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "counters");
            let choices = if spec.is_target() {
                vec![spec.clone()]
            } else {
                Vec::new()
            };
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PutCountersAll {
            counter_type,
            count,
            filter,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let mut effect = Effect::for_each(
                resolved_filter,
                vec![Effect::put_counters(
                    *counter_type,
                    resolved_count,
                    ChooseSpec::Iterated,
                )],
            );
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("counters");
                effect = effect.tag_all(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RemoveUpToAnyCounters {
            amount,
            target,
            counter_type,
            up_to,
            all_of_them,
        } => {
            if *all_of_them {
                return Err(CardTextError::ParseError(
                    "unable to resolve 'all of them' counter reference".to_string(),
                ));
            }
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let compiled = compile_tagged_effect_for_target(target, ctx, "counters", |spec| {
                let resolved_amount = match (&resolved_amount, counter_type) {
                    (
                        Value::CountersOn(counter_source, amount_counter_type),
                        Some(counter_type),
                    ) if matches!(counter_source.as_ref(), ChooseSpec::Source)
                        && amount_counter_type == &Some(*counter_type) =>
                    {
                        Value::CountersOn(Box::new(spec.clone()), Some(*counter_type))
                    }
                    (Value::CountersOn(counter_source, None), None)
                        if matches!(counter_source.as_ref(), ChooseSpec::Source) =>
                    {
                        Value::CountersOn(Box::new(spec.clone()), None)
                    }
                    _ => resolved_amount.clone(),
                };
                let effect = if let Some(counter_type) = counter_type {
                    if *up_to {
                        Effect::remove_up_to_counters(*counter_type, resolved_amount, spec)
                    } else {
                        Effect::remove_counters(*counter_type, resolved_amount, spec)
                    }
                } else {
                    Effect::remove_up_to_any_counters(resolved_amount, spec)
                };
                Effect::with_id(id.0, effect)
            })?;
            Ok(compiled)
        }
        SubjectVerbActionAst::MoveAllCounters { from, to } => {
            let (from_spec, mut choices) =
                resolve_target_spec_with_choices(from, &current_reference_env(ctx))?;
            let (to_spec, to_choices) =
                resolve_target_spec_with_choices(to, &current_reference_env(ctx))?;
            for choice in to_choices {
                push_choice(&mut choices, choice);
            }
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_all_counters(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::MoveOneCounter { from, to } => {
            let (from_spec, mut choices) =
                resolve_target_spec_with_choices(from, &current_reference_env(ctx))?;
            let (to_spec, to_choices) =
                resolve_target_spec_with_choices(to, &current_reference_env(ctx))?;
            for choice in to_choices {
                push_choice(&mut choices, choice);
            }
            let effect = tag_object_target_effect(
                tag_object_target_effect(
                    Effect::move_one_counter(from_spec.clone(), to_spec.clone()),
                    &from_spec,
                    ctx,
                    "from",
                ),
                &to_spec,
                ctx,
                "to",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ForEachCounterKindPutOrRemove {
            target,
            all_kinds,
            fixed_counter_type,
            optional_action,
        } => {
            let (mut spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if fixed_counter_type.is_some()
                && let TargetAst::Object(filter, explicit_target_span, _) = target
                && explicit_target_span.is_none()
            {
                spec = ChooseSpec::All(resolve_it_tag(filter, &current_reference_env(ctx))?);
            }
            let effect = if let Some(counter_type) = fixed_counter_type {
                crate::effects::ForEachCounterKindPutOrRemoveEffect::fixed_counter_type(
                    spec,
                    *counter_type,
                    *optional_action,
                )
            } else if *all_kinds {
                crate::effects::ForEachCounterKindPutOrRemoveEffect::new(spec)
            } else {
                crate::effects::ForEachCounterKindPutOrRemoveEffect::one_kind(spec)
            };
            Ok((vec![Effect::new(effect)], choices))
        }
        SubjectVerbActionAst::PutCounterOfChosenKind { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            Ok((
                vec![Effect::new(
                    crate::effects::PutCounterOfChosenKindEffect::new(spec),
                )],
                choices,
            ))
        }
        SubjectVerbActionAst::ReturnToHand {
            target,
            random,
            destination_player_surface,
            exiled_with_source_surface,
            set_quantifier_surface,
            set_reference_surface,
        } => {
            let graveyard_player_surface = return_graveyard_player_surface(target, ctx)?;
            let (mut spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            // A plural demonstrative in a later per-player sentence refers to
            // the collection chosen by the preceding quantified sentence.
            // `Iterated` cannot resolve an object while only a player loop is
            // active, so retain the producer tag explicitly instead.
            if ctx.iterated_player
                && !ctx.iterated_object
                && set_reference_surface.is_some()
                && matches!(spec.base(), ChooseSpec::Iterated)
                && let Some(tag) = ctx.last_object_tag.as_deref()
            {
                spec = ChooseSpec::Tagged(TagKey::from(tag));
            }
            let destination_player_surface = destination_player_surface
                .map(|player| resolve_non_target_player_filter(player, &current_reference_env(ctx)))
                .transpose()?;
            let from_graveyard = target_mentions_graveyard(target);
            if from_graveyard
                && !ctx.iterated_player
                && ctx.last_player_filter.as_ref() != Some(&PlayerFilter::IteratedPlayer)
                && choose_spec_mentions_iterated_player(&spec)
            {
                replace_iterated_player_with_target_player_in_choose_spec(&mut spec);
            }
            let move_effect = if from_graveyard {
                let mut effect =
                    crate::effects::ReturnFromGraveyardToHandEffect::new(spec.clone(), *random);
                if let Some(player) = graveyard_player_surface {
                    effect = effect.with_graveyard_player_surface(player);
                }
                if let Some(player) = destination_player_surface.clone() {
                    effect = effect.with_destination_player_surface(player);
                }
                Effect::new(effect)
            } else {
                let mut effect = crate::effects::ReturnToHandEffect::with_spec(spec.clone());
                if let Some(player) = destination_player_surface.clone() {
                    effect = effect.with_destination_player_surface(player);
                }
                if let Some(surface) = exiled_with_source_surface {
                    effect = effect.with_exiled_with_source_surface(surface.clone());
                }
                effect = effect.with_set_quantifier_surface(*set_quantifier_surface);
                effect = effect.with_set_reference_surface(set_reference_surface.clone());
                Effect::new(effect)
            };
            let effect = tag_object_target_effect(move_effect, &spec, ctx, "returned");
            ctx.last_player_filter = Some(if spec.is_target() {
                PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
            } else if let Some(tag) = ctx.last_object_tag.clone() {
                PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(TagKey::from(tag.as_str())))
            } else {
                PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
            });
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ReturnAllToHand {
            filter,
            destination_player_surface,
            exiled_with_source_surface,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let destination_player_surface = destination_player_surface
                .map(|player| resolve_non_target_player_filter(player, &current_reference_env(ctx)))
                .transpose()?;
            let mut effect = crate::effects::ReturnToHandEffect::all(resolved_filter);
            if let Some(player) = destination_player_surface {
                effect = effect.with_destination_player_surface(player);
            }
            if let Some(surface) = exiled_with_source_surface {
                effect = effect.with_exiled_with_source_surface(surface.clone());
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        SubjectVerbActionAst::ReturnAllToHandOfChosenColor { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                crate::color::Color::White,
                crate::color::Color::Blue,
                crate::color::Color::Black,
                crate::color::Color::Red,
                crate::color::Color::Green,
            ];
            for color in colors {
                let chosen = ColorSet::from(color);
                let mut filter = resolved_filter.clone();
                filter.colors = Some(
                    filter
                        .colors
                        .map_or(chosen, |existing| existing.intersection(chosen)),
                );
                let description = format!(
                    "Return all {} to their owners' hands.",
                    filter.description()
                );
                modes.push(EffectMode {
                    source_text: description,
                    effects: vec![Effect::return_all_to_hand(filter)],
                });
            }
            prelude.push(Effect::choose_one(modes));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::MoveToLibraryNthFromTop { target, position } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = Effect::new(crate::effects::MoveToLibraryNthFromTopEffect::new(
                spec.clone(),
                position.clone(),
            ));
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::DoubleCountersOnEach {
            counter_type,
            filter,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let effect = Effect::double_counters(*counter_type, ChooseSpec::All(resolved_filter));
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::DoubleCountersOnTarget {
            counter_type,
            target,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = Effect::double_counters(*counter_type, spec);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::RemoveCountersAll {
            amount,
            filter,
            counter_type,
            up_to,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let iterated = ChooseSpec::Iterated;
            let inner = if let Some(counter_type) = counter_type {
                if *up_to {
                    Effect::remove_up_to_counters(*counter_type, resolved_amount, iterated.clone())
                } else {
                    Effect::remove_counters(*counter_type, resolved_amount, iterated.clone())
                }
            } else {
                Effect::remove_up_to_any_counters(resolved_amount, iterated.clone())
            };
            let effect = Effect::for_each(resolved_filter, vec![inner]);
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::PutSticker { target, action } => match target {
            TargetAst::Object(filter, explicit_target_span, _)
                if explicit_target_span.is_none() =>
            {
                let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
                let tag = ctx.next_tag("stickered");
                let tag_key = TagKey::from(tag.as_str());
                let choose_effect = crate::effects::ChooseObjectsEffect::new(
                    resolved_filter,
                    ChoiceCount::exactly(1),
                    PlayerFilter::You,
                    tag_key.clone(),
                )
                .in_zone(choice_zone);
                ctx.last_object_tag = Some(tag.as_str().to_string());
                Ok((
                    vec![
                        Effect::new(choose_effect),
                        Effect::put_sticker(ChooseSpec::Tagged(tag_key), *action),
                    ],
                    Vec::new(),
                ))
            }
            _ => compile_effect_for_target(target, ctx, |spec| Effect::put_sticker(spec, *action)),
        },
        SubjectVerbActionAst::SwitchPowerToughness { target, duration } => {
            compile_tagged_effect_for_target(target, ctx, "switched_pt", |spec| {
                Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec(
                        spec,
                        crate::continuous::Modification::SwitchPowerToughness,
                        duration.clone(),
                    )
                    .require_creature_target(),
                )
            })
        }
        SubjectVerbActionAst::ScalePowerToughnessAll {
            filter,
            power,
            toughness,
            multiplier,
            duration,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let scaled_stat = |value: Value| {
                if *multiplier == 1 {
                    value
                } else {
                    Value::Scaled(Box::new(value), *multiplier)
                }
            };
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::new(
                    crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        ChooseSpec::Iterated,
                        crate::effects::continuous::RuntimeModification::ModifyPowerToughness {
                            power: if *power {
                                scaled_stat(Value::PowerOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                            toughness: if *toughness {
                                scaled_stat(Value::ToughnessOf(Box::new(ChooseSpec::Iterated)))
                            } else {
                                Value::Fixed(0)
                            },
                        },
                        duration.clone(),
                    )
                    .require_creature_target(),
                )],
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::Discard {
            count,
            random,
            any_number,
            filter,
            tag,
        } => {
            let count_names_complete_discard_set = filter.as_ref().is_some_and(|filter| {
                matches!(count.unhinted(), Value::Count(count_filter) if count_filter == filter)
            });
            let discard_references_revealed_hand_choice = filter.as_ref().is_some_and(|filter| {
                filter.zone == Some(Zone::Hand) && filter_references_tag(filter, IT_TAG)
            });
            let resolved_filter = if let Some(filter) = filter {
                let mut resolved = resolve_it_tag(filter, &current_reference_env(ctx))?;
                if resolved.zone.is_none() {
                    resolved.zone = Some(Zone::Hand);
                }
                if discard_references_revealed_hand_choice
                    && resolved.zone == Some(Zone::Hand)
                    && let Some(revealed_player) = ctx.last_revealed_player_filter.clone()
                {
                    resolved.owner = Some(revealed_player);
                    resolved.controller = None;
                }
                Some(resolved)
            } else {
                None
            };
            let explicit_full_hand_owner = count
                .has_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand)
                .then(|| match count.unhinted() {
                    Value::CardsInHand(player) => Some(player.clone()),
                    Value::Count(filter) => filter.owner.clone(),
                    _ => None,
                })
                .flatten();
            let (resolved_player, choices) =
                if matches!(subject_verb.subject.player, PlayerAst::Implicit) {
                    if let Some(inferred_player) = resolved_filter
                        .as_ref()
                        .and_then(|filter| {
                            if discard_references_revealed_hand_choice
                                && filter.zone == Some(Zone::Hand)
                            {
                                ctx.last_revealed_player_filter.clone()
                            } else {
                                infer_player_filter_from_object_filter(filter)
                            }
                        })
                        // An explicit possessive full-hand phrase supplies its
                        // own actor. Do not let a prior damaged/targeted player
                        // rebind `Discard all the cards in your hand`.
                        .or(explicit_full_hand_owner)
                        .or_else(|| {
                            ctx.last_player_filter
                                .clone()
                                .filter(|player| !matches!(player, PlayerFilter::Defending))
                        })
                    {
                        (inferred_player, Vec::new())
                    } else {
                        let subject = LoweredSubject::resolve_affected_player(
                            subject_verb.subject.player,
                            ctx,
                            true,
                            true,
                            true,
                        )?;
                        (subject.into_player_filter(), subject.into_choices())
                    }
                } else if matches!(subject_verb.subject.player, PlayerAst::That)
                    && let Some(inferred_player) = resolved_filter
                        .as_ref()
                        .and_then(infer_player_filter_from_object_filter)
                {
                    (inferred_player, Vec::new())
                } else {
                    let subject = LoweredSubject::resolve_affected_player(
                        subject_verb.subject.player,
                        ctx,
                        true,
                        true,
                        true,
                    )?;
                    (subject.into_player_filter(), subject.into_choices())
                };
            let subject = LoweredSubject::from_resolved(resolved_player.clone(), choices);
            let mut resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            subject.apply_player_refs_to_value(&mut resolved_count, ctx);
            let resolved_filter = resolved_filter
                .map(|resolved| subject.bind_discard_filter(&resolved, ctx))
                .transpose()?;
            if count_names_complete_discard_set && let Some(filter) = resolved_filter.as_ref() {
                replace_complete_discard_count_filter(&mut resolved_count, filter);
            }
            let tag = tag
                .clone()
                .unwrap_or_else(|| TagKey::from(ctx.next_tag("discarded").as_str()));
            ctx.last_object_tag = Some(tag.as_str().to_string());
            let effect = Effect::new(
                crate::effects::DiscardEffect::new_with_filter(
                    resolved_count,
                    resolved_player,
                    *random,
                    resolved_filter,
                )
                .with_any_number(*any_number)
                .with_tag(tag),
            );
            Ok((vec![effect], subject.into_choices()))
        }
        SubjectVerbActionAst::DiscardHand => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let (player_filter, choices) = subject.into_parts();
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                Effect::discard_hand,
                Effect::discard_hand_player,
            )
        }
        SubjectVerbActionAst::PoisonCounters { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            false,
            Effect::poison_counters,
            Effect::poison_counters_player,
        ),
        SubjectVerbActionAst::EnergyCounters { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            false,
            Effect::energy_counters,
            Effect::energy_counters_player,
        ),
        SubjectVerbActionAst::ExperienceCounters { count } => {
            compile_subject_verb_player_value_effect(
                role,
                player,
                count,
                ctx,
                true,
                true,
                true,
                false,
                Effect::experience_counters,
                Effect::experience_counters_player,
            )
        }
        SubjectVerbActionAst::TicketCounters { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            false,
            Effect::ticket_counters,
            Effect::ticket_counters_player,
        ),
        SubjectVerbActionAst::PayEnergy { amount } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let amount = subject.bind_player_refs_in_value(amount, ctx)?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::PayEnergyEffect::new(
                        amount.clone(),
                        ChooseSpec::Player(PlayerFilter::You),
                    ))
                },
                |filter| {
                    Effect::new(crate::effects::PayEnergyEffect::new(
                        amount.clone(),
                        ChooseSpec::Player(filter),
                    ))
                },
            )
        }
        SubjectVerbActionAst::PayAnyEnergy { min_amount } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::PayAnyEnergyEffect::new(
                        ChooseSpec::Player(PlayerFilter::You),
                        *min_amount,
                    ))
                },
                |filter| {
                    Effect::new(crate::effects::PayAnyEnergyEffect::new(
                        ChooseSpec::Player(filter),
                        *min_amount,
                    ))
                },
            )
        }
        SubjectVerbActionAst::PayAnyLife { min_amount } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::PayAnyLifeEffect::new(
                        ChooseSpec::Player(PlayerFilter::You),
                        *min_amount,
                    ))
                },
                |filter| {
                    Effect::new(crate::effects::PayAnyLifeEffect::new(
                        ChooseSpec::Player(filter),
                        *min_amount,
                    ))
                },
            )
        }
        SubjectVerbActionAst::PayMana { cost, x_value } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let x_value = x_value
                .as_ref()
                .map(|value| subject.resolve_object_refs_and_bind_player_refs_in_value(value, ctx))
                .transpose()?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    let mut effect = crate::effects::PayManaEffect::new(
                        cost.clone(),
                        ChooseSpec::Player(PlayerFilter::You),
                    );
                    if let Some(x_value) = x_value.clone() {
                        effect = effect.with_x_value(x_value);
                    }
                    Effect::new(effect)
                },
                |filter| {
                    let mut effect = crate::effects::PayManaEffect::new(
                        cost.clone(),
                        ChooseSpec::Player(filter),
                    );
                    if let Some(x_value) = x_value.clone() {
                        effect = effect.with_x_value(x_value);
                    }
                    Effect::new(effect)
                },
            )
        }
        SubjectVerbActionAst::DoubleManaPool => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::double_mana_pool_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::EmptyManaPool => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::empty_mana_pool_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::SetLifeTotal { amount } => compile_subject_verb_player_value_effect(
            role,
            player,
            amount,
            ctx,
            true,
            true,
            true,
            false,
            |value| Effect::set_life_total_player(value, PlayerFilter::You),
            |value, filter| Effect::set_life_total_player(value, filter),
        ),
        SubjectVerbActionAst::EndTurn => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::end_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::EndCombatPhase => Ok((vec![Effect::end_combat_phase()], Vec::new())),
        SubjectVerbActionAst::SkipTurn => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::SkipCombatPhases => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_combat_phases_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::SkipNextCombatPhaseThisTurn => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_next_combat_phase_this_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::SkipMainPhasesThisTurn => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_main_phases_this_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::SkipCombatPhasesThisTurn => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_combat_phases_this_turn_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::SkipDrawStep => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::skip_draw_step_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::AdditionalPhases { phases } => {
            Ok((vec![Effect::additional_phases(phases.clone())], Vec::new()))
        }
        SubjectVerbActionAst::PlayFromGraveyardUntilEot => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::grant_play_from_graveyard_until_eot(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ControlPlayer {
            player: target_player,
            duration,
        } => {
            let _subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let (start, duration) = match duration {
                ControlDurationAst::UntilEndOfTurn => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::UntilYourNextTurnEnd => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::DuringNextTurn => (
                    crate::game_state::PlayerControlStart::NextTurn,
                    crate::game_state::PlayerControlDuration::UntilEndOfTurn,
                ),
                ControlDurationAst::Forever => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::Forever,
                ),
                ControlDurationAst::AsLongAsYouControlSource => (
                    crate::game_state::PlayerControlStart::Immediate,
                    crate::game_state::PlayerControlDuration::UntilSourceLeaves,
                ),
            };

            let mut choices = Vec::new();
            if let PlayerFilter::Target(inner) = target_player {
                let spec = ChooseSpec::target(ChooseSpec::Player((**inner).clone()));
                choices.push(spec);
                ctx.last_player_filter = Some(PlayerFilter::target_player());
            } else {
                ctx.last_player_filter = Some(target_player.clone());
            }

            let effect = Effect::control_player(target_player.clone(), start, duration);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ReduceNextSpellCostThisTurn { filter, reduction } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let mut player_filter = subject.into_player_filter();
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            Ok((
                vec![Effect::new(
                    crate::effects::GrantNextSpellCostReductionEffect::new(
                        player_filter,
                        resolved_filter,
                        reduction.clone(),
                    ),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::ReduceMatchingSpellCostThisTurn {
            filter,
            reduction,
            duration,
        } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            let mut player_filter = subject.into_player_filter();
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            Ok((
                vec![Effect::new(
                    crate::effects::GrantNextSpellCostReductionEffect::all_matching_until(
                        player_filter,
                        resolved_filter,
                        reduction.clone(),
                        duration.clone(),
                    ),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::GrantNextSpellAbilityThisTurn { filter, ability } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let mut player_filter = subject.clone_player_filter();
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if let Some(last_player_filter) = ctx.last_player_filter.clone() {
                bind_relative_iterated_player_to_last_player_filter(
                    &mut player_filter,
                    &mut resolved_filter,
                    &last_player_filter,
                );
            }
            let lowered =
                lower_granted_abilities_ast_to_object_abilities(std::slice::from_ref(ability))?;
            if lowered.is_empty() {
                return Err(CardTextError::ParseError(
                    "temporary next-spell grant did not lower to an object ability".to_string(),
                ));
            }
            Ok((
                lowered
                    .into_iter()
                    .map(|ability| {
                        Effect::grant_next_spell_ability_this_turn(
                            player_filter.clone(),
                            resolved_filter.clone(),
                            ability,
                        )
                    })
                    .collect(),
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::RingTemptsYou => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::ring_tempts_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::VentureIntoDungeon {
            undercity_if_no_active,
        } => compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
            if *undercity_if_no_active {
                Effect::venture_into_undercity_player(subject.into_player_filter())
            } else {
                Effect::venture_into_dungeon_player(subject.into_player_filter())
            }
        }),
        SubjectVerbActionAst::BecomeMonarch => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::become_monarch_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::TakeInitiative => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::take_initiative_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::CreateEmblem { emblem } => {
            let emblem = compile_emblem_description(emblem)?;
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let filter = subject.clone_player_filter();
            let effect = if matches!(&filter, PlayerFilter::You) {
                Effect::create_emblem(emblem)
            } else {
                Effect::for_players(filter, vec![Effect::create_emblem(emblem)])
            };
            Ok((vec![effect], subject.into_choices()))
        }
        SubjectVerbActionAst::LoseGame => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let (player_filter, choices) = subject.into_parts();
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                Effect::lose_the_game,
                Effect::lose_the_game_player,
            )
        }
        SubjectVerbActionAst::WinGame => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let (player_filter, choices) = subject.into_parts();
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                Effect::win_the_game,
                Effect::win_the_game_player,
            )
        }
        SubjectVerbActionAst::Detain { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect =
                tag_object_target_effect(Effect::detain(spec.clone()), &spec, ctx, "detained");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Goad { target, duration } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect = tag_object_target_effect(
                Effect::goad_for(spec.clone(), duration.clone()),
                &spec,
                ctx,
                "goaded",
            );
            track_selected_object_player_provenance(&spec, ctx);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Suspect { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect =
                tag_object_target_effect(Effect::suspect(spec.clone()), &spec, ctx, "suspected");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ClearSuspected { target } => {
            let Some(target) = target else {
                return Ok(Some((vec![Effect::clear_all_suspected()], Vec::new())));
            };
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let spec = if choices.is_empty() {
                match spec {
                    ChooseSpec::Object(filter) => ChooseSpec::All(filter),
                    other => other,
                }
            } else {
                spec
            };
            let effect = tag_object_target_effect(
                Effect::clear_suspected(spec.clone()),
                &spec,
                ctx,
                "no_longer_suspected",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::HealDamage { target, amount } => {
            compile_tagged_effect_for_target(target, ctx, "healed", |spec| match amount {
                Some(amount) => Effect::heal_damage(spec, amount.clone()),
                None => Effect::heal_all_damage(spec),
            })
        }
        SubjectVerbActionAst::RemoveFromCombat { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::RemoveFromCombatEffect::with_spec(
                    spec.clone(),
                )),
                &spec,
                ctx,
                "removed_from_combat",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Flip { target } => {
            compile_tagged_effect_for_target(target, ctx, "flipped", Effect::flip)
        }
        SubjectVerbActionAst::Regenerate {
            target,
            follow_up_effects,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut follow_ups = Vec::new();
            if !follow_up_effects.is_empty() {
                let saved_last_object_tag = ctx.last_object_tag.clone();
                ctx.last_object_tag = Some(IT_TAG.to_string());
                let (compiled_follow_ups, follow_up_choices) =
                    compile_effects(follow_up_effects, ctx)?;
                follow_ups = compiled_follow_ups;
                for choice in follow_up_choices {
                    push_choice(&mut choices, choice);
                }
                ctx.last_object_tag = saved_last_object_tag;
            }
            let regenerate = crate::effects::RegenerateEffect::new(
                spec.clone(),
                crate::effect::Until::EndOfTurn,
            )
            .with_follow_up_effects(follow_ups);
            let effect =
                tag_object_target_effect(Effect::new(regenerate), &spec, ctx, "regenerated");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::RegenerateAll { filter } => {
            let (mut prelude, choices) = target_context_prelude_for_filter(filter);
            prelude.push(Effect::regenerate(
                ChooseSpec::all(filter.clone()),
                crate::effect::Until::EndOfTurn,
            ));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::Sacrifice {
            filter,
            count,
            target,
            one_of_referenced_set,
        } => {
            if let Some(target) = target {
                let (effects, mut choices) =
                    compile_tagged_effect_for_target(target, ctx, "sacrificed", |spec| {
                        Effect::new(crate::effects::SacrificeTargetEffect::new(spec))
                    })?;
                let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
                let chooser = subject.into_player_filter();
                ctx.last_player_filter = Some(chooser);
                for choice in subject.into_choices() {
                    push_choice(&mut choices, choice);
                }
                return Ok(Some((effects, choices)));
            }
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let chooser = subject.clone_player_filter();
            let target_prelude = subject.target_prelude();
            let refs = current_reference_env(ctx);
            let bare_it_with_source_antecedent = !*one_of_referenced_set
                && !refs.iterated_object
                && refs.has_source_object_antecedent()
                && refs
                    .known_last_object_tag()
                    .is_none_or(|tag| tag.as_str() == IT_TAG && !refs.last_it_choice_is_set)
                && object_filter_as_tagged_reference(filter)
                    .is_some_and(|tag| tag.as_str() == IT_TAG);
            let mut resolved_filter = if bare_it_with_source_antecedent {
                ObjectFilter::source()
            } else {
                match subject.bind_sacrifice_filter(filter, ctx) {
                    Ok(resolved) => resolved,
                    Err(_)
                        if filter.tagged_constraints.len() == 1
                            && filter.tagged_constraints[0].tag.as_str() == IT_TAG =>
                    {
                        ObjectFilter::source()
                    }
                    Err(err) => return Err(err),
                }
            };
            if resolved_filter.source {
                if *count != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "source sacrifice only supports count 1 (count: {})",
                        count
                    )));
                }
                if !matches!(chooser, PlayerFilter::You) {
                    return Err(CardTextError::ParseError(
                        "source sacrifice requires source controller chooser".to_string(),
                    ));
                }
                let mut effects = target_prelude;
                effects.push(Effect::sacrifice_source());
                return Ok(Some((effects, subject.into_choices())));
            }
            if !*one_of_referenced_set
                && *count == 1
                && let Some(tag) = object_filter_as_tagged_reference(&resolved_filter)
            {
                let mut effects = target_prelude;
                effects.push(Effect::new(crate::effects::SacrificeTargetEffect::new(
                    ChooseSpec::tagged(tag),
                )));
                return Ok(Some((effects, subject.into_choices())));
            }

            if *one_of_referenced_set {
                resolved_filter.set_one_of_tagged_set_surface(true);
            }
            let tag = ctx.next_tag("sacrificed");
            ctx.last_object_tag = Some(tag.clone());
            let choose = Effect::choose_objects(
                resolved_filter,
                *count as usize,
                chooser.clone(),
                tag.clone(),
            );
            let sacrifice =
                Effect::sacrifice_player(ObjectFilter::tagged(tag), *count, chooser.clone());
            let mut effects = target_prelude;
            effects.push(choose);
            effects.push(sacrifice);
            Ok((effects, subject.into_choices()))
        }
        SubjectVerbActionAst::SacrificeAll { filter } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let chooser = subject.clone_player_filter();
            let resolved_filter = subject.bind_sacrifice_filter(filter, ctx)?;
            let count = Value::Count(resolved_filter.clone());
            let effect = Effect::sacrifice_player(resolved_filter, count, chooser.clone());
            let mut effects = subject.target_prelude();
            effects.push(effect);
            Ok((effects, subject.into_choices()))
        }
        _ => return Ok(None),
    };
    result.map(Some)
}

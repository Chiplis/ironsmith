use super::*;

type EffectCompileOutcome = (Vec<Effect>, Vec<ChooseSpec>);
type EffectCompileHandler = fn(
    &EffectAst,
    &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError>;

#[derive(Clone, Copy)]
struct EffectCompileHandlerDef {
    run: EffectCompileHandler,
}

const EFFECT_COMPILE_HANDLERS: [EffectCompileHandlerDef; 14] = [
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_combat_and_damage_effect,
    },
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_board_state_effect,
    },
    EffectCompileHandlerDef {
        run: effect_combat_resource_handlers::try_compile_player_resource_and_choice_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_timing_and_control_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_flow_and_iteration_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_destroy_and_exile_effect,
    },
    EffectCompileHandlerDef {
        run: effect_visibility_object_handlers::try_compile_visibility_and_card_selection_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_stack_and_condition_effect,
    },
    EffectCompileHandlerDef {
        run: effect_handlers::try_compile_attachment_and_setup_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_token_generation_effect,
    },
    EffectCompileHandlerDef {
        run: effect_continuous_turn_handlers::try_compile_continuous_and_modifier_effect,
    },
    EffectCompileHandlerDef {
        run: effect_flow_search_handlers::try_compile_search_and_reorder_effect,
    },
    EffectCompileHandlerDef {
        run: effect_visibility_object_handlers::try_compile_object_zone_and_exchange_effect,
    },
    EffectCompileHandlerDef {
        run: effect_continuous_turn_handlers::try_compile_player_turn_and_counter_effect,
    },
];

fn retarget_target_is_bare_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter == &ObjectFilter::tagged(IT_TAG),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            retarget_target_is_bare_it(inner)
        }
        _ => false,
    }
}

fn target_is_any_damage_target(target: &TargetAst) -> bool {
    match target {
        TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_) => true,
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_is_any_damage_target(inner)
        }
        _ => false,
    }
}

pub(crate) fn compile_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    stacker::maybe_grow(1024 * 1024, 2 * 1024 * 1024, || {
        compile_effect_inner(effect, ctx)
    })
}

fn compile_effect_inner(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    if let EffectAst::SubjectVerb(subject_verb) = effect {
        return compile_subject_verb_effect(subject_verb, ctx);
    }
    if let EffectAst::ManaRestricted {
        effects,
        restrictions,
    } = effect
    {
        let mut compiled_effects = Vec::new();
        let mut choices = Vec::new();
        for child in effects {
            let (mut child_effects, mut child_choices) = compile_effect(child, ctx)?;
            compiled_effects.append(&mut child_effects);
            choices.append(&mut child_choices);
        }
        return Ok((
            vec![Effect::mana_restricted(
                compiled_effects,
                restrictions.clone(),
            )],
            choices,
        ));
    }
    if let EffectAst::RepeatEffects { count, effects } = effect {
        let mut compiled_effects = Vec::new();
        let mut choices = Vec::new();
        for child in effects {
            let (mut child_effects, mut child_choices) = compile_effect(child, ctx)?;
            compiled_effects.append(&mut child_effects);
            choices.append(&mut child_choices);
        }
        return Ok((
            vec![Effect::repeat_effects(count.clone(), compiled_effects)],
            choices,
        ));
    }
    if let EffectAst::MayCastMatchingSpellWithoutPayingManaCost {
        player,
        filter,
        zone,
    } = effect
    {
        let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
        let player = resolve_non_target_player_filter(player.clone(), &current_reference_env(ctx))?;
        return Ok((
            vec![Effect::new(
                crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect::new(
                    player,
                    resolved_filter,
                    *zone,
                ),
            )],
            Vec::new(),
        ));
    }
    if matches!(
        effect,
        EffectAst::RepeatThisProcess | EffectAst::RepeatThisProcessOnce
    ) {
        return Err(CardTextError::ParseError(
            "unsupported repeat this process effect tail".to_string(),
        ));
    }
    if let Some(compiled) = try_compile_effect_via_handlers(effect, ctx)? {
        return Ok(compiled);
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing compile-effect dispatch route for effect variant: {effect:?}"
    )))
}

fn try_compile_effect_via_handlers(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    for EffectCompileHandlerDef { run, .. } in EFFECT_COMPILE_HANDLERS {
        if let Some(compiled) = run(effect, ctx)? {
            return Ok(Some(compiled));
        }
    }
    Ok(None)
}

fn compile_subject_verb_effect(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    let role = subject_verb_role(subject_verb.subject.role);
    let player = subject_verb.subject.player;
    match &subject_verb.action {
        SubjectVerbActionAst::Draw { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            false,
            Effect::draw,
            Effect::target_draws,
        ),
        SubjectVerbActionAst::DrawForEachTaggedMatching { tag, filter } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            Ok((
                vec![Effect::new(
                    crate::effects::DrawForEachTaggedMatchingEffect::new(
                        subject.into_player_filter(),
                        resolved_tag,
                        resolved_filter,
                    ),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::LoseLife { amount } => compile_subject_verb_player_value_effect(
            role,
            player,
            amount,
            ctx,
            true,
            true,
            true,
            false,
            Effect::lose_life,
            Effect::lose_life_player,
        ),
        SubjectVerbActionAst::GainLife { amount } => compile_subject_verb_player_value_effect(
            role,
            player,
            amount,
            ctx,
            true,
            true,
            true,
            false,
            Effect::gain_life,
            |value, filter| Effect::gain_life_player(value, ChooseSpec::Player(filter)),
        ),
        SubjectVerbActionAst::RevealHand => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let choices = subject.into_choices();
            let spec = choices
                .first()
                .cloned()
                .unwrap_or_else(|| ChooseSpec::Player(player_filter.clone()));
            ctx.last_player_filter = Some(match player {
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => player_filter.clone(),
            });
            ctx.last_object_tag = None;
            let effect = Effect::new(crate::effects::LookAtHandEffect::reveal(spec));
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Mill { count } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let count = subject.bind_player_refs_in_value(count, ctx)?;
            let effect = if matches!(&player_filter, PlayerFilter::You) {
                Effect::mill(count.clone())
            } else {
                Effect::mill_player(count.clone(), player_filter)
            };
            let mut effects = Vec::new();
            if effect.target_spec().is_none() {
                effects.extend(subject.target_prelude());
            }
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("milled");
                effects.push(effect.tag(tag.clone()));
                ctx.last_object_tag = Some(tag);
            } else {
                effects.push(effect);
            }
            Ok((effects, subject.into_choices()))
        }
        SubjectVerbActionAst::Scry { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            false,
            false,
            true,
            false,
            Effect::scry,
            Effect::scry_player,
        ),
        SubjectVerbActionAst::Surveil { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            false,
            false,
            true,
            false,
            Effect::surveil,
            Effect::surveil_player,
        ),
        SubjectVerbActionAst::Proliferate { count } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let mut effect = Effect::proliferate(count);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("proliferated");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::Investigate { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            false,
            false,
            true,
            true,
            |count| Effect::investigate_player(count, PlayerFilter::You),
            Effect::investigate_player,
        ),
        SubjectVerbActionAst::Incubate { amount, count } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let amount = subject.resolve_object_refs_and_bind_player_refs_in_value(amount, ctx)?;
            let count = subject.resolve_object_refs_and_bind_player_refs_in_value(count, ctx)?;
            let you_amount = amount.clone();
            let you_count = count.clone();
            let (player_filter, choices) = subject.into_parts();
            let amount = per_player_partition_value_for_filter(amount, &player_filter);
            let count = per_player_partition_value_for_filter(count, &player_filter);
            let you_amount = per_player_partition_value_for_filter(you_amount, &PlayerFilter::You);
            let you_count = per_player_partition_value_for_filter(you_count, &PlayerFilter::You);
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || Effect::incubate(you_amount, you_count),
                |filter| Effect::incubate_player(amount, count, filter),
            )
        }
        SubjectVerbActionAst::Learn => Ok((vec![Effect::learn()], Vec::new())),
        SubjectVerbActionAst::EmitKeywordAction { action, amount } => {
            Ok((vec![Effect::emit_keyword_action(*action, *amount)], Vec::new()))
        }
        SubjectVerbActionAst::Amass { subtype, amount } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let mut effect = Effect::amass(*subtype, amount);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("amassed");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::Bolster { amount } => {
            Ok((vec![Effect::bolster(*amount)], Vec::new()))
        }
        SubjectVerbActionAst::Support { amount } => {
            Ok((vec![Effect::support(*amount)], Vec::new()))
        }
        SubjectVerbActionAst::Adapt { amount } => Ok((vec![Effect::adapt(*amount)], Vec::new())),
        SubjectVerbActionAst::Monstrosity { amount } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            Ok((vec![Effect::monstrosity(amount)], Vec::new()))
        }
        SubjectVerbActionAst::Discover { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            false,
            false,
            true,
            false,
            Effect::discover,
            Effect::discover_player,
        ),
        SubjectVerbActionAst::Fateseal { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            false,
            false,
            true,
            false,
            Effect::fateseal,
            Effect::fateseal_player,
        ),
        SubjectVerbActionAst::Populate {
            count,
            enters_tapped,
            enters_attacking,
            has_haste,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
        } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let mut effect = Effect::new(
                crate::effects::PopulateEffect::new(count)
                    .enters_tapped(*enters_tapped)
                    .attacking(*enters_attacking)
                    .haste(*has_haste)
                    .sacrifice_at_next_end_step(*sacrifice_at_next_end_step)
                    .exile_at_next_end_step(*exile_at_next_end_step)
                    .exile_at_end_of_combat(*exile_at_end_of_combat)
                    .sacrifice_at_end_of_combat(*sacrifice_at_end_of_combat),
            );
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("created");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::Explore { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect =
                tag_object_target_effect(Effect::explore(spec.clone()), &spec, ctx, "explored");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::Exploit => {
            let id = ctx.next_effect_id();
            Ok((
                vec![
                    Effect::with_id(
                        id.0,
                        Effect::may(vec![Effect::sacrifice_with_event_tags(
                            ObjectFilter::creature(),
                            1,
                            crate::tag::EXPLOITED_TAG,
                            crate::tag::EXPLOITER_TAG,
                        )]),
                    ),
                    Effect::if_then(
                        id,
                        EffectPredicate::Happened,
                        vec![Effect::emit_keyword_action_with_affected_object_memory_tag(
                            crate::events::KeywordActionKind::Exploit,
                            1,
                            id,
                            crate::tag::EXPLOITED_TAG,
                        )],
                    ),
                ],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::Connive { target, count } => {
            let count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::connive_with_count(spec.clone(), count),
                &spec,
                ctx,
                "connived",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ConniveIterated => {
            Ok((vec![Effect::connive(ChooseSpec::Iterated)], Vec::new()))
        }
        SubjectVerbActionAst::PutRestOnBottomOfLibrary => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'rest' without prior reference".to_string(),
                )
            })?;

            let mut membership_filter = ObjectFilter::default();
            membership_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_it = Condition::TaggedObjectMatches(TagKey::from(IT_TAG), membership_filter);
            let move_rest = Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_it,
                    Vec::new(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Library,
                        false,
                    )],
                )],
            );

            Ok((vec![move_rest], Vec::new()))
        }
        SubjectVerbActionAst::DontLoseThisManaAsStepsAndPhasesEndThisTurn => Ok((
            vec![Effect::new(
                crate::effects::RetainManaUntilEndOfTurnEffect::you(),
            )],
            Vec::new(),
        )),
        SubjectVerbActionAst::OpenAttraction => Ok((vec![Effect::open_attraction()], Vec::new())),
        SubjectVerbActionAst::ManifestTopCardOfLibrary => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::manifest_top_card_of_library(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ManifestCardFromHand => {
            Ok((vec![Effect::manifest_card_from_hand()], Vec::new()))
        }
        SubjectVerbActionAst::ManifestDread => Ok((vec![Effect::manifest_dread()], Vec::new())),
        SubjectVerbActionAst::Earthbend { counters } => {
            let spec = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::land().you_control()));
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::EarthbendEffect::new(
                    spec.clone(),
                    *counters,
                )),
                &spec,
                ctx,
                "earthbend",
            );
            Ok((vec![effect], vec![spec]))
        }
        SubjectVerbActionAst::Behold { subtype, count } => {
            Ok((vec![Effect::behold(*subtype, *count)], Vec::new()))
        }
        SubjectVerbActionAst::Fight {
            creature1,
            creature2,
        } => {
            let (spec1, mut choices) =
                resolve_target_spec_with_choices(creature1, &current_reference_env(ctx))?;
            let (spec2, other_choices) =
                resolve_target_spec_with_choices(creature2, &current_reference_env(ctx))?;
            for choice in other_choices {
                push_choice(&mut choices, choice);
            }
            Ok((vec![Effect::fight(spec1, spec2)], choices))
        }
        SubjectVerbActionAst::FightIterated { creature2 } => {
            let (spec2, choices) =
                resolve_target_spec_with_choices(creature2, &current_reference_env(ctx))?;
            Ok((vec![Effect::fight(ChooseSpec::Iterated, spec2)], choices))
        }
        SubjectVerbActionAst::Clash { opponent } => match opponent {
            ClashOpponentAst::Opponent => Ok((
                vec![Effect::new(
                    crate::effects::ClashEffect::against_any_opponent(),
                )],
                Vec::new(),
            )),
            ClashOpponentAst::TargetOpponent => {
                let choice = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Opponent));
                Ok((
                    vec![Effect::new(
                        crate::effects::ClashEffect::against_target_opponent(),
                    )],
                    vec![choice],
                ))
            }
            ClashOpponentAst::DefendingPlayer => Ok((
                vec![Effect::new(
                    crate::effects::ClashEffect::against_defending_player(),
                )],
                Vec::new(),
            )),
        },
        SubjectVerbActionAst::FlipCoin => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::flip_coin(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::RollDie { sides } => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::roll_die(*sides, subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ShuffleHandAndGraveyardIntoLibrary => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::shuffle_hand_and_graveyard_into_library_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ShuffleGraveyardIntoLibrary => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::shuffle_graveyard_into_library_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ReorderGraveyard => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::reorder_graveyard_player(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ChooseColor => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::choose_color(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ChooseCardType { options } => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::choose_card_type(subject.into_player_filter(), options.clone())
            })
        }
        SubjectVerbActionAst::ChooseNamedOption { options } => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::choose_named_option(subject.into_player_filter(), options.clone())
            })
        }
        SubjectVerbActionAst::ChooseCreatureType { excluded_subtypes } => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::choose_creature_type(
                    subject.into_player_filter(),
                    excluded_subtypes.clone(),
                )
            })
        }
        SubjectVerbActionAst::ChooseCardName { filter, tag } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let chooser = subject.clone_player_filter();
            let mut effects = subject.target_prelude();
            effects.push(Effect::choose_card_name(
                chooser.clone(),
                filter.clone(),
                tag.clone(),
            ));
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(chooser);
            Ok((effects, subject.into_choices()))
        }
        SubjectVerbActionAst::ChoosePlayer {
            filter,
            tag,
            random,
            exclude_previous_choices,
        } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let resolved_filter = filter.clone();
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.next_tag("chosen_player").as_str())
            } else {
                tag.clone()
            };
            let excluded_tags = if *exclude_previous_choices == 0 {
                Vec::new()
            } else {
                let len = ctx.recent_player_choice_tags.len();
                ctx.recent_player_choice_tags[len.saturating_sub(*exclude_previous_choices)..]
                    .iter()
                    .cloned()
                    .map(TagKey::from)
                    .collect::<Vec<_>>()
            };
            let (effects, choices) = compile_choose_player_with_subject(
                subject,
                resolved_filter,
                resolved_tag.clone(),
                *random,
                excluded_tags,
            );
            ctx.last_player_filter = Some(PlayerFilter::TaggedPlayer(resolved_tag.clone()));
            ctx.recent_player_choice_tags
                .push(resolved_tag.as_str().to_string());
            Ok((effects, choices))
        }
        SubjectVerbActionAst::ChooseSpellCastHistory {
            cast_by,
            filter,
            tag,
        } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, false)?;
            let chooser_filter = subject.clone_player_filter();
            let choices = subject.into_choices();
            let cast_by_filter =
                resolve_non_target_player_filter(*cast_by, &current_reference_env(ctx))?;
            let effect = Effect::new(
                crate::effects::ChooseSpellCastHistoryEffect::new(
                    chooser_filter,
                    cast_by_filter,
                    filter.clone(),
                    tag.clone(),
                )
                .with_description("Choose one of those sorcery spells"),
            );
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            ctx.last_object_tag = Some(tag.as_str().to_string());
            Ok((effects, choices))
        }
        SubjectVerbActionAst::AddMana { mana } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                subject.clone_player_filter(),
                subject.into_choices(),
                || Effect::add_mana(mana.clone()),
                |filter| Effect::add_mana_player(mana.clone(), filter),
            )
        }
        SubjectVerbActionAst::AddManaScaled { mana, amount } => {
            let (amount, player_filter, choices) =
                resolve_player_scoped_value(amount, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || {
                    Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                        mana.clone(),
                        amount.clone(),
                        PlayerFilter::You,
                    ))
                },
                |filter| {
                    Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                        mana.clone(),
                        amount.clone(),
                        filter,
                    ))
                },
            )
        }
        SubjectVerbActionAst::AddManaAnyColor {
            amount,
            available_colors,
        } => {
            let (amount, player_filter, choices) =
                resolve_player_scoped_value(amount, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || {
                    if let Some(colors) = available_colors.clone() {
                        Effect::add_mana_of_any_color_restricted(amount.clone(), colors)
                    } else {
                        Effect::add_mana_of_any_color(amount.clone())
                    }
                },
                |filter| {
                    if let Some(colors) = available_colors.clone() {
                        Effect::add_mana_of_any_color_restricted_player(
                            amount.clone(),
                            filter,
                            colors,
                        )
                    } else {
                        Effect::add_mana_of_any_color_player(amount.clone(), filter)
                    }
                },
            )
        }
        SubjectVerbActionAst::AddManaAnyOneColor { amount } => {
            let (amount, player_filter, choices) =
                resolve_player_scoped_value(amount, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || Effect::add_mana_of_any_one_color(amount.clone()),
                |filter| Effect::add_mana_of_any_one_color_player(amount.clone(), filter),
            )
        }
        SubjectVerbActionAst::AddManaChosenColor {
            amount,
            fixed_option,
        } => {
            let (amount, player_filter, choices) =
                resolve_player_scoped_value(amount, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || {
                    if let Some(fixed) = fixed_option {
                        Effect::new(
                            crate::effects::mana::AddManaOfChosenColorEffect::with_fixed_option(
                                amount.clone(),
                                PlayerFilter::You,
                                *fixed,
                            ),
                        )
                    } else {
                        Effect::new(crate::effects::mana::AddManaOfChosenColorEffect::new(
                            amount.clone(),
                            PlayerFilter::You,
                        ))
                    }
                },
                |filter| {
                    if let Some(fixed) = fixed_option {
                        Effect::new(
                            crate::effects::mana::AddManaOfChosenColorEffect::with_fixed_option(
                                amount.clone(),
                                filter,
                                *fixed,
                            ),
                        )
                    } else {
                        Effect::new(crate::effects::mana::AddManaOfChosenColorEffect::new(
                            amount.clone(),
                            filter,
                        ))
                    }
                },
            )
        }
        SubjectVerbActionAst::AddManaFromLandCouldProduce {
            amount,
            land_filter,
            allow_colorless,
            same_type,
        } => {
            let (amount, player_filter, choices) =
                resolve_player_scoped_value(amount, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || {
                    Effect::add_mana_of_land_produced_types_player(
                        amount.clone(),
                        PlayerFilter::You,
                        land_filter.clone(),
                        *allow_colorless,
                        *same_type,
                    )
                },
                |filter| {
                    Effect::add_mana_of_land_produced_types_player(
                        amount.clone(),
                        filter,
                        land_filter.clone(),
                        *allow_colorless,
                        *same_type,
                    )
                },
            )
        }
        SubjectVerbActionAst::AddManaColorsAmong { filter } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                subject.clone_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::mana::AddManaOfColorsAmongEffect::new(
                        filter.clone(),
                        PlayerFilter::You,
                    ))
                },
                |player_filter| {
                    Effect::new(crate::effects::mana::AddManaOfColorsAmongEffect::new(
                        filter.clone(),
                        player_filter,
                    ))
                },
            )
        }
        SubjectVerbActionAst::AddManaCommanderIdentity { amount } => {
            let (amount, player_filter, choices) =
                resolve_player_scoped_value(amount, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || Effect::add_mana_from_commander_color_identity(amount.clone()),
                |filter| {
                    Effect::add_mana_from_commander_color_identity_player(amount.clone(), filter)
                },
            )
        }
        SubjectVerbActionAst::ExchangeLifeTotals { player2 } => {
            compile_exchange_life_totals_effect(player, *player2, ctx)
        }
        SubjectVerbActionAst::ExchangeTextBoxes { target } => {
            compile_exchange_text_boxes_effect(target, ctx)
        }
        SubjectVerbActionAst::ExchangeZones { zone1, zone2 } => {
            compile_exchange_zones_effect(player, *zone1, *zone2, ctx)
        }
        SubjectVerbActionAst::ExchangeValues {
            left,
            right,
            duration,
        } => compile_exchange_values_effect(left, right, duration.clone(), ctx),
        SubjectVerbActionAst::ExchangeControl {
            filter,
            count,
            shared_type,
        } => {
            let targets = ChooseSpec::target(ChooseSpec::Object(filter.clone()))
                .with_count(ChoiceCount::exactly(*count as usize));
            let exchange = crate::effects::ExchangeControlEffect::new(targets.clone(), targets);
            let exchange = if let Some(shared_type) = shared_type {
                let constraint = match shared_type {
                    SharedTypeConstraintAst::CardType => {
                        crate::effects::SharedTypeConstraint::CardType
                    }
                    SharedTypeConstraintAst::PermanentType => {
                        crate::effects::SharedTypeConstraint::PermanentType
                    }
                };
                exchange.with_shared_type(constraint)
            } else {
                exchange
            };
            let mut effect = Effect::new(exchange);
            let tag = ctx.next_tag("exchanged");
            effect = effect.tag(tag.clone());
            ctx.last_object_tag = Some(tag);
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::ExchangeControlHeterogeneous {
            permanent1,
            permanent2,
            shared_type,
        } => {
            compile_exchange_control_heterogeneous_effect(permanent1, permanent2, *shared_type, ctx)
        }
        SubjectVerbActionAst::Attach { object, target } => {
            let (objects, object_choices) =
                resolve_attach_object_spec(object, &current_reference_env(ctx))?;
            let (target, target_choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut choices = Vec::new();
            for choice in object_choices {
                push_choice(&mut choices, choice);
            }
            for choice in target_choices {
                push_choice(&mut choices, choice);
            }
            Ok((vec![Effect::attach_objects(objects, target)], choices))
        }
        SubjectVerbActionAst::Enchant { filter } => {
            let spec = filter.target_spec();
            let effect = Effect::attach_to(spec.clone());
            Ok((vec![effect], vec![spec]))
        }
        SubjectVerbActionAst::ExileWhenSourceLeaves { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let ChooseSpec::Tagged(tag) = spec.base() else {
                return Err(CardTextError::ParseError(
                    "cannot compile 'exile ... when this source leaves' without tagged context"
                        .to_string(),
                ));
            };
            let effect = Effect::new(crate::effects::ExileTaggedWhenSourceLeavesEffect::new(
                tag.clone(),
                PlayerFilter::You,
            ));
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::SacrificeSourceWhenLeaves { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let ChooseSpec::Tagged(tag) = spec.base() else {
                return Err(CardTextError::ParseError(
                    "cannot compile 'sacrifice this source when ... leaves' without tagged context"
                        .to_string(),
                ));
            };
            let effect = Effect::new(
                crate::effects::ScheduleEffectsWhenTaggedLeavesEffect::new(
                    tag.clone(),
                    vec![Effect::sacrifice_source()],
                    PlayerFilter::You,
                )
                .with_current_source_as_ability_source(),
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::RegisterZoneReplacement {
            target,
            from_zone,
            to_zone,
            replacement_zone,
            duration,
            optional,
            choice_description,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::OneShot
                }
            };
            let mut replacement = crate::effects::RegisterZoneReplacementEffect::new(
                spec,
                *from_zone,
                *to_zone,
                *replacement_zone,
                mode,
            );
            if *optional {
                replacement.optional = true;
                replacement.choice_description = choice_description.clone();
            }
            let effect = Effect::new(replacement);
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::RegisterFutureZoneReplacement {
            filter,
            from_zone,
            to_zone,
            replacement_zone,
            duration,
        } => {
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::OneShot
                }
            };
            let effect = Effect::new(
                crate::effects::RegisterFutureZoneReplacementEffect::new(
                    filter.clone(),
                    *from_zone,
                    *to_zone,
                    *replacement_zone,
                    mode,
                )
                .with_cause_filter(crate::events::cause::CauseFilter::effect_like())
                .requiring_cause_source_match(),
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement {
            filter,
            from_zone,
            to_zone,
            replacement_zone,
            duration: _,
        } => {
            let effect = Effect::new(
                crate::effects::RegisterDamagedBySourceZoneReplacementEffect::new(
                    filter.clone(),
                    *from_zone,
                    *to_zone,
                    *replacement_zone,
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn,
                ),
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RegisterEnterUnderControlReplacement { filter, duration } => {
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
            };
            let effect = Effect::new(
                crate::effects::RegisterEnterUnderControlReplacementEffect::new(
                    filter.clone(),
                    mode,
                ),
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::ExileInsteadOfGraveyardThisTurn => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::exile_instead_of_graveyard_this_turn(subject.into_player_filter())
            })
        }
        SubjectVerbActionAst::ControlCombatChoicesThisTurn {
            attackers,
            blockers,
        } => Ok((
            vec![Effect::control_combat_choices_this_turn(
                *attackers, *blockers,
            )],
            Vec::new(),
        )),
        SubjectVerbActionAst::GainControl { target, duration } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let controller = subject.into_player_filter();
            choices.extend(subject.into_choices());
            let runtime_modification = if matches!(controller, PlayerFilter::You) {
                crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController
            } else {
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    controller,
                )
            };
            let effect = tag_object_target_effect(
                Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                    spec.clone(),
                    runtime_modification,
                    duration.clone(),
                )),
                &spec,
                ctx,
                "controlled",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::RevealTop => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let tag = ctx.next_tag("revealed");
            ctx.last_object_tag = Some(tag.clone());
            ctx.last_revealed_tag = Some(tag.clone());
            Ok((
                vec![Effect::reveal_top(player_filter, tag)],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::ExileTopOfLibrary {
            count,
            tags,
            accumulated_tags,
        } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let resolved_count =
                subject.resolve_object_refs_and_bind_player_refs_in_value(count, ctx)?;
            let player_filter = subject.clone_player_filter();
            let mut effect =
                crate::effects::ExileTopOfLibraryEffect::new(resolved_count, player_filter.clone());
            for tag in tags {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                effect = effect.tag_moved(resolved_tag);
            }
            for tag in accumulated_tags {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                effect = effect.append_tagged(resolved_tag);
            }
            if let Some(tag) = tags.first() {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            }
            ctx.last_player_filter = Some(player_filter);
            Ok((vec![Effect::new(effect)], subject.into_choices()))
        }
        SubjectVerbActionAst::RevealTagged { tag } => {
            let resolved_tag = if tag.as_str() == IT_TAG {
                if let Some(existing) = ctx.last_object_tag.clone() {
                    existing
                } else {
                    let generated = ctx.next_tag("revealed");
                    ctx.last_object_tag = Some(generated.clone());
                    generated
                }
            } else {
                let explicit = tag.as_str().to_string();
                ctx.last_object_tag = Some(explicit.clone());
                explicit
            };
            Ok((
                vec![Effect::new(crate::effects::RevealTaggedEffect::new(
                    resolved_tag,
                ))],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::RevealCardsFromHand {
            count,
            count_value,
            tag,
        } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.next_tag("revealed").as_str())
            } else {
                tag.clone()
            };
            let mut filter = ObjectFilter::default();
            filter.zone = Some(Zone::Hand);
            filter.owner = Some(player_filter.clone());
            let choose = crate::effects::ChooseObjectsEffect::new(
                filter,
                *count,
                player_filter.clone(),
                resolved_tag.clone(),
            )
            .with_count_value_opt(count_value.clone())
            .in_zone(Zone::Hand)
            .reveal();
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            ctx.last_revealed_tag = Some(resolved_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter);
            Ok((vec![Effect::new(choose)], subject.into_choices()))
        }
        SubjectVerbActionAst::LookAtTopCards { count, tag, reveal } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.next_tag("revealed").as_str())
            } else {
                tag.clone()
            };
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            if *reveal {
                ctx.last_revealed_tag = Some(resolved_tag.as_str().to_string());
            }
            let effect = if *reveal {
                Effect::reveal_top_cards(player_filter, count.clone(), resolved_tag)
            } else {
                Effect::look_at_top_cards(player_filter, count.clone(), resolved_tag)
            };
            Ok((
                vec![effect],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::PutIntoHand { object } => {
            let ObjectRefAst::Tagged(tag) = object;
            let tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            Ok((
                vec![Effect::move_to_zone(
                    ChooseSpec::Tagged(tag),
                    Zone::Hand,
                    false,
                )],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::MayMoveToZone { target, zone } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            for choice in subject.into_choices() {
                push_choice(&mut choices, choice);
            }
            Ok((
                vec![Effect::may_move_to_zone(
                    spec,
                    *zone,
                    subject.into_player_filter(),
                )],
                choices,
            ))
        }
        SubjectVerbActionAst::PutSomeIntoHandRestIntoGraveyard { count } => {
            compile_put_some_into_hand_rest_to_zone(role, player, *count, Zone::Graveyard, ctx)
        }
        SubjectVerbActionAst::PutSomeIntoHandRestOnBottomOfLibrary { count } => {
            compile_put_some_into_hand_rest_to_zone(role, player, *count, Zone::Library, ctx)
        }
        SubjectVerbActionAst::AdditionalLandPlays { count, duration } => {
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::additional_land_plays(
                    resolved_count.clone(),
                    subject.into_player_filter(),
                    duration.clone(),
                )
            })
        }
        SubjectVerbActionAst::ExtraTurnAfterTurn { anchor } => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                let player_filter = subject.into_player_filter();
                match anchor {
                    ExtraTurnAnchorAst::CurrentTurn => Effect::extra_turn_player(player_filter),
                    ExtraTurnAnchorAst::ReferencedTurn => {
                        Effect::extra_turn_after_next_turn_player(player_filter)
                    }
                }
            })
        }
        SubjectVerbActionAst::RearrangeLookedCardsInLibrary { tag, count } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            Ok((
                vec![Effect::rearrange_looked_cards_in_library(
                    resolved_tag,
                    player_filter,
                    *count,
                )],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::ReorderTopOfLibrary { tag } => {
            let effective_tag = if tag.as_str() == IT_TAG {
                ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "cannot resolve 'them' without prior tagged object".to_string(),
                    )
                })?
            } else {
                tag.as_str().to_string()
            };
            Ok((
                vec![Effect::new(crate::effects::ReorderLibraryTopEffect::new(
                    effective_tag,
                ))],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::AddManaImprintedColors => Ok((
            vec![Effect::new(
                crate::effects::mana::AddManaOfImprintedColorsEffect::new(),
            )],
            Vec::new(),
        )),
        SubjectVerbActionAst::ShuffleLibrary => {
            let ast_is_explicit_you = matches!(player, PlayerAst::You | PlayerAst::Implicit);
            if !ast_is_explicit_you
                && ctx
                    .last_object_tag
                    .as_ref()
                    .is_some_and(|tag| tag.starts_with("searched"))
                && ctx
                    .last_player_filter
                    .as_ref()
                    .is_some_and(|filter| *filter != PlayerFilter::You)
            {
                Ok((
                    vec![Effect::shuffle_library_player(
                        ctx.last_player_filter.clone().expect("checked above"),
                    )],
                    Vec::new(),
                ))
            } else {
                compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                    Effect::shuffle_library_player(subject.into_player_filter())
                })
            }
        }
        SubjectVerbActionAst::ShuffleObjectsIntoLibrary { target } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            for choice in subject.into_choices() {
                push_choice(&mut choices, choice);
            }
            let mut effect =
                Effect::shuffle_objects_into_library(spec.clone(), subject.into_player_filter());
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            effect = Effect::with_id(id.0, effect);
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::GrantProtectionChoice {
            target,
            allow_colorless,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut modes = Vec::new();
            if *allow_colorless {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Colorless);
                modes.push(EffectMode {
                    description: "Colorless".to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }
            for (name, color) in [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ] {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Color(
                    ColorSet::from(color),
                ));
                modes.push(EffectMode {
                    description: name.to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }
            let effect =
                tag_object_target_effect(Effect::choose_one(modes), &spec, ctx, "protected");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PreventAllCombatDamage { duration } => Ok((
            vec![Effect::prevent_all_combat_damage(duration.clone())],
            Vec::new(),
        )),
        SubjectVerbActionAst::PreventAllCombatDamageFromSource { duration, source } => {
            compile_effect_for_target(source, ctx, |spec| {
                Effect::prevent_all_combat_damage_from(spec, duration.clone())
            })
        }
        SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter {
            duration,
            source_filter,
        } => Ok((
            vec![Effect::prevent_all_combat_damage_from_filter(
                source_filter.clone(),
                duration.clone(),
            )],
            Vec::new(),
        )),
        SubjectVerbActionAst::PreventAllCombatDamageToPlayers { duration } => Ok((
            vec![Effect::prevent_all_combat_damage_to_players(
                duration.clone(),
            )],
            Vec::new(),
        )),
        SubjectVerbActionAst::PreventAllCombatDamageToYou { duration } => Ok((
            vec![Effect::prevent_all_combat_damage_to_you(duration.clone())],
            Vec::new(),
        )),
        SubjectVerbActionAst::PreventNextTimeDamage {
            source,
            target,
            reflect_damage_to_source_controller,
        } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::PreventNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::PreventNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            let target_spec = match target {
                PreventNextTimeDamageTargetAst::AnyTarget => {
                    crate::effects::PreventNextTimeDamageTarget::AnyTarget
                }
                PreventNextTimeDamageTargetAst::You => {
                    crate::effects::PreventNextTimeDamageTarget::You
                }
            };
            let mut effect = crate::effects::PreventNextTimeDamageEffect::new(
                source_spec,
                target_spec,
            );
            if *reflect_damage_to_source_controller {
                effect = effect.reflecting_to_source_controller();
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
        }
        SubjectVerbActionAst::PreventDamage {
            amount,
            target,
            duration,
            source_of_your_choice,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            if let TargetAst::Object(filter, explicit_target_span, _) = target
                && explicit_target_span.is_none()
            {
                let filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                let effect = Effect::for_each(
                    filter,
                    vec![Effect::prevent_damage(
                        amount,
                        ChooseSpec::Iterated,
                        duration.clone(),
                    )],
                );
                Ok((vec![effect], Vec::new()))
            } else {
                compile_effect_for_target(target, ctx, |spec| {
                    if *source_of_your_choice {
                        Effect::prevent_damage_with_source_choice(
                            amount.clone(),
                            spec,
                            duration.clone(),
                        )
                    } else {
                        Effect::prevent_damage(amount.clone(), spec, duration.clone())
                    }
                })
            }
        }
        SubjectVerbActionAst::PreventAllDamageToTarget { target, duration } => {
            if let TargetAst::Object(filter, explicit_target_span, _) = target
                && explicit_target_span.is_none()
            {
                let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                Ok((
                    vec![Effect::prevent_all_damage_to(
                        resolved_filter,
                        duration.clone(),
                    )],
                    Vec::new(),
                ))
            } else {
                compile_effect_for_target(target, ctx, |spec| {
                    Effect::prevent_all_damage_to_target(spec, duration.clone())
                })
            }
        }
        SubjectVerbActionAst::PreventAllDamageFromSourceFilter {
            duration,
            source_filter,
        } => {
            let source_filter = resolve_it_tag(source_filter, &current_reference_env(ctx))?;
            Ok((
                vec![Effect::prevent_all_damage_from_filter(
                    source_filter,
                    duration.clone(),
                )],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::PreventDamageToTargetPutCounters {
            amount,
            target,
            duration,
            counter_type,
        } => {
            let follow_up = vec![Effect::put_counters(
                *counter_type,
                Value::EventValue(EventValueSpec::Amount),
                ChooseSpec::AnyTarget,
            )];
            match amount {
                Some(amount) => {
                    let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
                    compile_effect_for_target(target, ctx, |spec| {
                        Effect::new(
                            crate::effects::PreventDamageEffect::new(
                                amount.clone(),
                                spec,
                                duration.clone(),
                            )
                            .with_follow_up_effects(follow_up.clone()),
                        )
                    })
                }
                None => compile_effect_for_target(target, ctx, |spec| {
                    Effect::new(
                        crate::effects::PreventAllDamageToTargetEffect::new(spec, duration.clone())
                            .with_follow_up_effects(follow_up.clone()),
                    )
                }),
            }
        }
        SubjectVerbActionAst::PreventDamageEach {
            amount,
            filter,
            duration,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let effect = Effect::for_each(
                filter,
                vec![Effect::prevent_damage(
                    amount,
                    ChooseSpec::Iterated,
                    duration.clone(),
                )],
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget { amount, target } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::RedirectNextDamageToTargetEffect::new(
                    amount.clone(),
                    spec,
                ))
            })
        }
        SubjectVerbActionAst::RedirectNextTimeDamageToSource {
            source,
            target,
            all_this_turn,
        } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::RedirectNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::RedirectNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            compile_effect_for_target(target, ctx, |spec| {
                let effect = crate::effects::RedirectNextTimeDamageToSourceEffect::new(
                    source_spec.clone(),
                    spec,
                );
                let effect = if *all_this_turn {
                    effect.all_this_turn()
                } else {
                    effect
                };
                Effect::new(effect)
            })
        }
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
                    description: put_mode_text.clone(),
                    effects: vec![put_effect],
                },
                EffectMode {
                    description: remove_mode_text.clone(),
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
            count,
            player,
            may_choose_new_targets,
            removed_supertypes,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
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
                Some(Effect::may_choose_new_targets_player(
                    id,
                    player_filter.clone(),
                ))
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
        SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
            tag,
            keep_tagged,
            order,
            player,
        } => {
            let subject = LoweredSubject::resolve_library_owner(*player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let resolved_keep_tagged = keep_tagged
                .as_ref()
                .map(|tag| resolve_it_tag_key(tag, &current_reference_env(ctx)))
                .transpose()?;
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
            } else {
                tag.clone()
            };
            let player_filter = match player {
                PlayerAst::ItsOwner => PlayerFilter::OwnerOf(ObjectRef::tagged(resolved_tag.clone())),
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
            } else {
                tag.clone()
            };
            let mut effects = vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                resolved_tag.clone(),
                player_filter.clone(),
                crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn,
                *allow_land,
                *allow_any_color_for_cast,
            ))];
            if *without_paying_mana_cost {
                effects.push(Effect::new(
                    crate::effects::GrantTaggedSpellFreeCastUntilEndOfTurnEffect::new(
                        resolved_tag,
                        player_filter,
                    ),
                ));
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
        } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.last_object_tag.clone().ok_or_else(|| {
                    CardTextError::ParseError(
                        "unable to resolve 'it' without prior reference".to_string(),
                    )
                })?)
            } else {
                tag.clone()
            };
            Ok((
                vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    resolved_tag,
                    player_filter,
                    crate::effects::GrantPlayTaggedDuration::UntilYourNextTurnEnd,
                    *allow_land,
                    *allow_any_color_for_cast,
                ))],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
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
            } else {
                tag.clone()
            };
            Ok((
                vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    resolved_tag,
                    player_filter,
                    crate::effects::GrantPlayTaggedDuration::ForAsLongAsExiled,
                    *allow_land,
                    *allow_any_color_for_cast,
                ))],
                Vec::new(),
            ))
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
            } else {
                tag.clone()
            };
            Ok((
                vec![Effect::new(crate::effects::GrantPlayTaggedEffect::new(
                    resolved_tag,
                    player_filter,
                    crate::effects::GrantPlayTaggedDuration::ForAsLongAsYouControlSource,
                    *allow_land,
                    *allow_any_color_for_cast,
                ))],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::ExileUntilSourceLeaves { target, face_down } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
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
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let from_exile_tag = choose_spec_references_exiled_tag(&spec);
            let use_move_to_zone =
                from_exile_tag || !matches!(controller, ReturnControllerAst::Preserve);
            let mut effects = Vec::new();
            let resolved_spec = if !spec.is_target() {
                match &spec {
                    ChooseSpec::Object(filter)
                        if filter.tagged_constraints.is_empty()
                            && filter.zone == Some(Zone::Graveyard) =>
                    {
                        let tag = ctx.next_tag("chosen_return");
                        ctx.last_object_tag = Some(tag.clone());
                        effects.push(Effect::choose_objects(
                            filter.clone(),
                            1usize,
                            PlayerFilter::You,
                            tag.clone(),
                        ));
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
                        effects.push(Effect::new(
                            crate::effects::ChooseObjectsEffect::new(
                                filter.clone(),
                                *count,
                                PlayerFilter::You,
                                tag.clone(),
                            )
                            .with_count_value_opt(count_value.clone()),
                        ));
                        ChooseSpec::tagged(tag)
                    }
                    _ => spec.clone(),
                }
            } else {
                spec.clone()
            };
            let resolved_spec = if !ctx.iterated_player
                && ctx.last_object_tag.as_deref() == Some(IT_TAG)
                && *controller == ReturnControllerAst::Owner
                && matches!(resolved_spec.base(), ChooseSpec::Iterated)
            {
                ChooseSpec::Tagged(TagKey::from(IT_TAG))
            } else {
                resolved_spec
            };

            let mut effect = tag_object_target_effect(
                if use_move_to_zone {
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
                        let mut return_effect = crate::effects::ReturnFromGraveyardToBattlefieldEffect::new(
                            resolved_spec.clone(),
                            *tapped,
                        )
                        .as_aura(as_aura.attachment_filter.clone());
                        if as_aura.remove_all_abilities {
                            return_effect = return_effect
                                .as_aura_removing_all_abilities(as_aura.attachment_filter.clone());
                        }
                        effect = Effect::new(return_effect);
                    }
                    effect
                },
                &resolved_spec,
                ctx,
                "returned",
            );
            if ctx.auto_tag_object_targets
                && !resolved_spec.is_target()
                && choose_spec_targets_object(&resolved_spec)
            {
                let tag = ctx.next_tag("returned");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            effects.push(effect);
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
            controller,
        } => {
            let return_all = crate::effects::ReturnAllToBattlefieldEffect::new(
                resolve_it_tag(filter, &current_reference_env(ctx))?,
                *tapped,
            );
            let return_all = match controller {
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
            battlefield_controller,
            battlefield_tapped,
            attached_to,
        } => {
            let (mut spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if !ctx.iterated_player
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
                let move_effect =
                    crate::effects::MoveToZoneEffect::new(spec.clone(), *zone, *to_top);
                let move_effect = if *zone == Zone::Battlefield && *battlefield_tapped {
                    move_effect.tapped()
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
                    let tag = ctx.next_tag("moved");
                    ctx.last_object_tag = Some(tag.clone());
                    effect = effect.tag(tag);
                }
                return Ok((vec![choose, effect], choices));
            }
            let move_effect = crate::effects::MoveToZoneEffect::new(spec.clone(), *zone, *to_top);
            let move_effect = if *zone == Zone::Battlefield && *battlefield_tapped {
                move_effect.tapped()
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
            let should_tag = choose_spec_targets_object(&spec)
                && (ctx.auto_tag_object_targets || attached_to.is_some());
            if should_tag {
                let tag = ctx.next_tag("moved");
                moved_tag = Some(tag.clone());
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
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
                return Ok((
                    vec![effect, Effect::attach_objects(moved_objects, attach_spec)],
                    choices,
                ));
            }

            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mut effect = Effect::move_to_library_top_or_bottom_choice(spec.clone());
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::TargetOnly { target } => {
            compile_tagged_effect_for_target(target, ctx, "targeted", |spec| {
                Effect::new(crate::effects::TargetOnlyEffect::new(spec))
            })
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
                .require_creature_target();
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
        } => compile_tagged_effect_for_target(target, ctx, "set_base_pt", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::SetPowerToughness {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        sublayer: crate::continuous::PtSublayer::Setting,
                    },
                    duration.clone(),
                )
                .require_creature_target()
                .resolve_set_pt_values_at_resolution(),
            )
        }),
        SubjectVerbActionAst::BecomeBasePtCreature {
            power,
            toughness,
            target,
            card_types,
            subtypes,
            colors,
            abilities,
            granted_abilities,
            duration,
        } => {
            let granted_modifications =
                lower_granted_ability_grant_modifications(granted_abilities)?;
            compile_tagged_effect_for_target(target, ctx, "animated_creature", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::AddCardTypes(card_types.clone()),
                    duration.clone(),
                )
                .with_additional_modification(crate::continuous::Modification::SetPowerToughness {
                    power: power.clone(),
                    toughness: toughness.clone(),
                    sublayer: crate::continuous::PtSublayer::Setting,
                })
                .resolve_set_pt_values_at_resolution();
                if let Some(colors) = colors {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::SetColors(*colors),
                    );
                }
                if !subtypes.is_empty() {
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
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
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
            duration,
        } => compile_tagged_effect_for_target(target, ctx, "typed", |spec| {
            Effect::new(
                crate::effects::ApplyContinuousEffect::with_spec(
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
                ),
            )
        }),
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
        } => {
            let refs = current_reference_env(ctx);
            let (target_spec, mut choices) = resolve_target_spec_with_choices(target, &refs)?;
            let (source_spec, source_choices) = resolve_target_spec_with_choices(source, &refs)?;
            for choice in source_choices {
                push_choice(&mut choices, choice);
            }

            let effect = Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                target_spec.clone(),
                crate::effects::continuous::RuntimeModification::CopyOf {
                    source: source_spec,
                    preserve_source_abilities: *preserve_source_abilities,
                },
                duration.clone(),
            ));
            let effect = tag_object_target_effect(effect, &target_spec, ctx, "copied");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            if modifications.is_empty() {
                return Err(CardTextError::InvariantViolation(
                    "normalize_effects_ast should remove GrantAbilitiesAll with no abilities"
                        .to_string(),
                ));
            }

            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut apply = crate::effects::ApplyContinuousEffect::new(
                crate::continuous::EffectTarget::Filter(resolved_filter),
                modifications[0].clone(),
                duration.clone(),
            )
            .lock_filter_at_resolution();

            for modification in modifications.iter().skip(1) {
                apply = apply.with_additional_modification(modification.clone());
            }

            Ok((vec![Effect::new(apply)], Vec::new()))
        }
        SubjectVerbActionAst::RemoveAbilitiesAll {
            filter,
            abilities,
            duration,
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
                    description: String::new(),
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
        } => {
            let modifications = lower_granted_ability_grant_modifications(abilities)?;
            let Some(first_modification) = modifications.first() else {
                return compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                    Effect::new(crate::effects::TargetOnlyEffect::new(spec))
                });
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    first_modification.clone(),
                    duration.clone(),
                );

                for modification in modifications.iter().skip(1) {
                    apply = apply.with_additional_modification(modification.clone());
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
                });
            }
            let abilities = lower_granted_abilities_ast(abilities)?;
            let Some(first_ability) = abilities.first() else {
                return compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                    Effect::new(crate::effects::ApplyContinuousEffect::with_spec_runtime(
                        spec,
                        crate::effects::continuous::RuntimeModification::RemoveAllAbilities,
                        duration.clone(),
                    ))
                });
            };

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let mut apply = crate::effects::ApplyContinuousEffect::with_spec(
                    spec,
                    crate::continuous::Modification::RemoveAbility(first_ability.clone().into()),
                    duration.clone(),
                );

                for ability in abilities.iter().skip(1) {
                    apply = apply.with_additional_modification(
                        crate::continuous::Modification::RemoveAbility(ability.clone().into()),
                    );
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
                });
            }

            compile_tagged_effect_for_target(target, ctx, "granted", |spec| {
                let modes = abilities
                    .iter()
                    .zip(modifications.iter())
                    .map(|(ability, modification)| EffectMode {
                        description: granted_ability_mode_description(ability, &spec)
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
            let resolved_mode = match mode {
                crate::cards::builders::LibraryConsultModeAst::Reveal => {
                    crate::effects::consult_helpers::LibraryConsultMode::Reveal
                }
                crate::cards::builders::LibraryConsultModeAst::Exile => {
                    crate::effects::consult_helpers::LibraryConsultMode::Exile
                }
            };
            ctx.last_object_tag = Some(resolved_match_tag.as_str().to_string());
            ctx.last_player_filter = Some(player_filter.clone());
            Ok((
                vec![Effect::consult_top_of_library(
                    player_filter,
                    resolved_mode,
                    resolved_filter,
                    resolved_stop_rule,
                    resolved_all_tag,
                    resolved_match_tag,
                )],
                subject.into_choices(),
            ))
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
            granted_abilities,
        } => {
            let mut token = token_definition_for(name.as_str())
                .or_else(|| {
                    dynamic_power_toughness
                        .as_ref()
                        .and_then(|_| token_definition_for(format!("0/0 {name}").as_str()))
                })
                .ok_or_else(|| CardTextError::ParseError(format!("unsupported token '{name}'")))?;
            token
                .abilities
                .extend(lower_granted_abilities_ast_to_object_abilities(granted_abilities)?);
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
            if attached_to.is_some() {
                effect = effect.suppress_aura_attachment_choice();
            }
            let mut effect = Effect::new(effect);
            let resolved_dynamic_pt = dynamic_power_toughness
                .as_ref()
                .map(|(power, toughness)| {
                    Ok::<_, CardTextError>((
                        resolve_value_it_tag(power, &current_reference_env(ctx))?,
                        resolve_value_it_tag(toughness, &current_reference_env(ctx))?,
                    ))
                })
                .transpose()?;
            let resolved_attached_to = attached_to
                .as_ref()
                .map(|target| resolve_target_spec_with_choices(target, &current_reference_env(ctx)))
                .transpose()?;
            let needs_created_tag =
                ctx.auto_tag_object_targets || attached_to.is_some() || resolved_dynamic_pt.is_some();
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
            exile_at_next_end_step,
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
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
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
            exile_at_next_end_step,
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
                && str_starts_with(last_tag, "exile_cost_")
                && let ChooseSpec::Object(filter) = &source_spec
                && filter.zone == Some(Zone::Exile)
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                        && constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG
                })
            {
                source_spec = ChooseSpec::Tagged(TagKey::from(last_tag));
            }
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
            if *exile_at_next_end_step {
                effect = effect.exile_at_next_end_step(true);
            }
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
                vec![Effect::search_library_slots_to_hand(
                    resolved_slots,
                    player_filter,
                    *reveal,
                    resolved_tag,
                )],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::RevealTopChooseCardTypePutToHandRestBottom { count } => {
            use crate::effect::{Condition, EffectMode, Value};

            let subject = LoweredSubject::resolve_library_owner(player, ctx, true, true, false)?;
            let player_filter = subject.clone_player_filter();
            let choices = subject.into_choices();
            let mut modes = Vec::new();
            let card_type_modes = [
                ("Artifact", CardType::Artifact),
                ("Battle", CardType::Battle),
                ("Creature", CardType::Creature),
                ("Enchantment", CardType::Enchantment),
                ("Instant", CardType::Instant),
                ("Kindred", CardType::Kindred),
                ("Land", CardType::Land),
                ("Planeswalker", CardType::Planeswalker),
                ("Sorcery", CardType::Sorcery),
            ];

            for (label, card_type) in card_type_modes {
                let looked_tag = ctx.next_tag("revealed");
                let mut card_type_filter = ObjectFilter::default();
                card_type_filter.card_types.push(card_type);

                let reveal = Effect::look_at_top_cards(
                    player_filter.clone(),
                    Value::Fixed(*count as i32),
                    TagKey::from(looked_tag.as_str()),
                );
                let reveal_tagged =
                    Effect::new(crate::effects::RevealTaggedEffect::new(looked_tag.clone()));
                let move_by_type = Effect::for_each_tagged(
                    looked_tag,
                    vec![Effect::conditional(
                        Condition::TaggedObjectMatches(TagKey::from("__it__"), card_type_filter),
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Hand,
                            false,
                        )],
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Library,
                            false,
                        )],
                    )],
                );

                modes.push(EffectMode {
                    description: label.to_string(),
                    effects: vec![reveal, reveal_tagged, move_by_type],
                });
            }

            Ok((vec![Effect::choose_one(modes)], choices))
        }
        SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestIntoGraveyard { count, filter } => {
            use crate::effect::{Condition, Value};

            let subject = LoweredSubject::resolve_library_owner(player, ctx, true, true, false)?;
            let player_filter = subject.into_player_filter();
            let choices = subject.into_choices();
            let looked_tag = ctx.next_tag("revealed");
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            resolved_filter.zone = None;

            let reveal = Effect::look_at_top_cards(
                player_filter,
                Value::Fixed(*count as i32),
                TagKey::from(looked_tag.as_str()),
            );
            let reveal_tagged =
                Effect::new(crate::effects::RevealTaggedEffect::new(looked_tag.clone()));
            let distribute = Effect::for_each_tagged(
                looked_tag.clone(),
                vec![Effect::conditional(
                    Condition::TaggedObjectMatches(TagKey::from("__it__"), resolved_filter),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Hand,
                        false,
                    )],
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Graveyard,
                        false,
                    )],
                )],
            );

            ctx.last_object_tag = Some(looked_tag);
            Ok((vec![reveal, reveal_tagged, distribute], choices))
        }
        SubjectVerbActionAst::RevealTopPutMatchingIntoHandRestOnBottomOfLibrary {
            count,
            filter,
            order,
        } => {
            use crate::effect::Value;
            use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};

            let subject = LoweredSubject::resolve_library_owner(player, ctx, true, true, false)?;
            let player_filter = subject.clone_player_filter();
            let choices = subject.into_choices();
            let looked_tag = ctx.next_tag("revealed");
            let matched_tag = ctx.next_tag("matched");
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            resolved_filter.zone = None;
            resolved_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });

            let reveal = Effect::look_at_top_cards(
                player_filter.clone(),
                Value::Fixed(*count as i32),
                TagKey::from(looked_tag.as_str()),
            );
            let reveal_tagged =
                Effect::new(crate::effects::RevealTaggedEffect::new(looked_tag.clone()));
            let tag_matching = Effect::new(
                crate::effects::TagMatchingObjectsEffect::new(resolved_filter, matched_tag.clone())
                    .in_zones(vec![Zone::Library]),
            );
            let move_matching = Effect::for_each_tagged(
                matched_tag.clone(),
                vec![Effect::move_to_zone(
                    ChooseSpec::Iterated,
                    Zone::Hand,
                    false,
                )],
            );
            let resolved_order = match order {
                crate::cards::builders::LibraryBottomOrderAst::Random => {
                    crate::effects::consult_helpers::LibraryBottomOrder::Random
                }
                crate::cards::builders::LibraryBottomOrderAst::ChooserChooses => {
                    crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
                }
            };
            let move_rest = Effect::put_tagged_remainder_on_library_bottom(
                TagKey::from(looked_tag.as_str()),
                Some(TagKey::from(matched_tag.as_str())),
                resolved_order,
                player_filter,
            );

            ctx.last_object_tag = Some(looked_tag);
            Ok((
                vec![
                    reveal,
                    reveal_tagged,
                    tag_matching,
                    move_matching,
                    move_rest,
                ],
                choices,
            ))
        }
        SubjectVerbActionAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
            filter,
            count,
            reveal,
            if_not_chosen,
        } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve looked-at cards without prior reference".to_string(),
                )
            })?;
            let subject = LoweredSubject::resolve_chooser(player, ctx, true, true, false)?;
            let chooser = subject.clone_player_filter();
            let mut choose_filter =
                subject.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
            let mut choices = subject.into_choices();
            let source_zone = choose_filter.zone.unwrap_or(Zone::Library);
            choose_filter.zone = Some(source_zone);
            choose_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });

            let chosen_tag = ctx.next_tag("chosen");
            let chosen_tag_key: TagKey = chosen_tag.as_str().into();
            let choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    choose_filter,
                    *count,
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(source_zone),
            );

            let mut compiled = vec![choose];
            if *reveal {
                compiled.push(Effect::for_each_tagged(
                    chosen_tag.clone(),
                    vec![Effect::new(crate::effects::RevealTaggedEffect::new(
                        chosen_tag.clone(),
                    ))],
                ));
            }
            let move_to_hand_id = ctx.next_effect_id();
            compiled.push(Effect::with_id(
                move_to_hand_id.0,
                Effect::for_each_tagged(
                    chosen_tag.clone(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Hand,
                        false,
                    )],
                ),
            ));

            if source_zone == Zone::Library {
                let mut membership_filter = ObjectFilter::default();
                membership_filter
                    .tagged_constraints
                    .push(TaggedObjectConstraint {
                        tag: TagKey::from("__it__"),
                        relation: TaggedOpbjectRelation::SameStableId,
                    });
                let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key, membership_filter);
                compiled.push(Effect::for_each_tagged(
                    looked_tag,
                    vec![Effect::conditional(
                        in_chosen,
                        Vec::new(),
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Graveyard,
                            false,
                        )],
                    )],
                ));
            }

            if !if_not_chosen.is_empty() {
                let (if_not_effects, if_not_choices) =
                    with_preserved_lowering_context(ctx, |_| {}, |ctx| {
                        compile_effects(if_not_chosen, ctx)
                    })?;
                compiled.push(Effect::if_then(
                    move_to_hand_id,
                    EffectPredicate::DidNotHappen,
                    if_not_effects,
                ));
                choices.extend(if_not_choices);
            }

            ctx.last_object_tag = Some(chosen_tag);
            ctx.last_effect_id = Some(move_to_hand_id);
            Ok((compiled, choices))
        }
        SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
            spell_filter,
            order,
        } => effect_visibility_object_handlers::compile_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
            player,
            order.clone(),
            &[
                CardType::Artifact,
                CardType::Battle,
                CardType::Enchantment,
                CardType::Instant,
                CardType::Kindred,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Sorcery,
            ],
            Some(spell_filter),
            ctx,
        ),
        SubjectVerbActionAst::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary {
            order,
        } => effect_visibility_object_handlers::compile_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
            player,
            order.clone(),
            &[
                CardType::Artifact,
                CardType::Battle,
                CardType::Creature,
                CardType::Enchantment,
                CardType::Instant,
                CardType::Land,
                CardType::Planeswalker,
                CardType::Sorcery,
            ],
            None,
            ctx,
        ),
        SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            battlefield_filter,
            tapped,
        } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve looked-at cards without prior reference".to_string(),
                )
            })?;

            let subject = LoweredSubject::resolve_chooser(player, ctx, true, true, false)?;
            let chooser = subject.clone_player_filter();

            let mut primary_filter = subject
                .resolve_object_refs_and_bind_player_refs_in_filter(battlefield_filter, ctx)?;
            let choices = subject.into_choices();
            primary_filter.zone = Some(Zone::Library);
            primary_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });

            let battlefield_tag = ctx.next_tag("chosen");
            let battlefield_tag_key: TagKey = battlefield_tag.as_str().into();
            let choose_primary = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    primary_filter,
                    ChoiceCount::up_to(1),
                    chooser.clone(),
                    battlefield_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            let move_primary_id = ctx.next_effect_id();
            let move_primary = Effect::with_id(
                move_primary_id.0,
                Effect::for_each_tagged(
                    battlefield_tag.clone(),
                    vec![Effect::put_onto_battlefield(
                        ChooseSpec::Iterated,
                        *tapped,
                        chooser.clone(),
                    )],
                ),
            );

            let hand_tag = ctx.next_tag("chosen");
            let hand_tag_key: TagKey = hand_tag.as_str().into();
            let mut fallback_filter = ObjectFilter::tagged(looked_tag.clone());
            fallback_filter.zone = Some(Zone::Library);
            let fallback_choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    fallback_filter,
                    ChoiceCount::exactly(1),
                    chooser.clone(),
                    hand_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );
            let move_fallback = Effect::for_each_tagged(
                hand_tag.clone(),
                vec![Effect::move_to_zone(
                    ChooseSpec::Iterated,
                    Zone::Hand,
                    false,
                )],
            );
            let fallback = Effect::if_then(
                move_primary_id,
                EffectPredicate::DidNotHappen,
                vec![fallback_choose, move_fallback],
            );

            let mut in_battlefield_choice_filter = ObjectFilter::default();
            in_battlefield_choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_battlefield_choice =
                Condition::TaggedObjectMatches(battlefield_tag_key, in_battlefield_choice_filter);

            let mut in_hand_choice_filter = ObjectFilter::default();
            in_hand_choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from("__it__"),
                    relation: TaggedOpbjectRelation::SameStableId,
                });
            let in_hand_choice =
                Condition::TaggedObjectMatches(hand_tag_key, in_hand_choice_filter);

            let move_rest = Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_battlefield_choice,
                    Vec::new(),
                    vec![Effect::conditional(
                        in_hand_choice,
                        Vec::new(),
                        vec![Effect::move_to_zone(
                            ChooseSpec::Iterated,
                            Zone::Library,
                            false,
                        )],
                    )],
                )],
            );

            ctx.last_object_tag = Some(hand_tag);
            ctx.last_effect_id = Some(move_primary_id);
            Ok((
                vec![choose_primary, move_primary, fallback, move_rest],
                choices,
            ))
        }
        SubjectVerbActionAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
            battlefield_filter,
            hand_filter,
            tapped,
            order,
        } => {
            use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve looked-at cards without prior reference".to_string(),
                )
            })?;

            let subject = LoweredSubject::resolve_chooser(player, ctx, true, true, false)?;
            let chooser = subject.clone_player_filter();

            let mut primary_filter = subject
                .resolve_object_refs_and_bind_player_refs_in_filter(battlefield_filter, ctx)?;
            let choices = subject.into_choices();
            primary_filter.zone = Some(Zone::Library);
            primary_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });

            let battlefield_tag = ctx.next_tag("chosen");
            let battlefield_tag_key: TagKey = battlefield_tag.as_str().into();
            let choose_primary = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    primary_filter,
                    ChoiceCount::up_to(1),
                    chooser.clone(),
                    battlefield_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            let kept_tag = ctx.next_tag("kept");
            let kept_tag_key: TagKey = kept_tag.as_str().into();
            let move_primary = Effect::put_onto_battlefield(
                ChooseSpec::Tagged(battlefield_tag_key.clone()),
                *tapped,
                chooser.clone(),
            )
            .tag_all(kept_tag_key.clone());

            let mut secondary_filter = resolve_it_tag(hand_filter, &current_reference_env(ctx))?;
            secondary_filter.zone = Some(Zone::Library);
            secondary_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: TagKey::from(looked_tag.as_str()),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            secondary_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: battlefield_tag_key.clone(),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                });

            let hand_tag = ctx.next_tag("chosen");
            let hand_tag_key: TagKey = hand_tag.as_str().into();
            let choose_secondary = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    secondary_filter,
                    ChoiceCount::up_to(1),
                    chooser.clone(),
                    hand_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );
            let move_secondary =
                Effect::move_to_zone(ChooseSpec::Tagged(hand_tag_key.clone()), Zone::Hand, false)
                    .tag_all(kept_tag_key.clone());

            let resolved_order = match order {
                crate::cards::builders::LibraryBottomOrderAst::Random => {
                    crate::effects::consult_helpers::LibraryBottomOrder::Random
                }
                crate::cards::builders::LibraryBottomOrderAst::ChooserChooses => {
                    crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
                }
            };
            let move_rest = Effect::put_tagged_remainder_on_library_bottom(
                TagKey::from(looked_tag.as_str()),
                Some(kept_tag_key.clone()),
                resolved_order,
                chooser.clone(),
            );

            ctx.last_object_tag = Some(kept_tag);
            ctx.last_effect_id = None;
            Ok((
                vec![
                    choose_primary,
                    move_primary,
                    choose_secondary,
                    move_secondary,
                    move_rest,
                ],
                choices,
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
        SubjectVerbActionAst::GrantAbilityToSource { ability, duration } => {
            let lowered = lower_parsed_ability(ability.clone())?;
            Ok((
                vec![Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                    crate::target::ChooseSpec::Source,
                    crate::continuous::Modification::AddAbilityGeneric(lowered.into_runtime()),
                    duration.clone(),
                ))],
                Vec::new(),
            ))
        }
        SubjectVerbActionAst::DealDamage { amount, target } => {
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
                    Effect::deal_damage(resolved_amount.clone(), spec)
                })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
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
        SubjectVerbActionAst::DealDistributedDamage { amount, target } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let (mut effects, choices) = compile_tagged_effect_for_target(target, ctx, "damaged", |spec| {
                Effect::new(crate::effects::DealDistributedDamageEffect::new(
                    resolved_amount.clone(),
                    spec,
                ))
            })?;
            if target_is_any_damage_target(target) {
                let tag = ctx.next_tag("damaged");
                ctx.last_object_tag = Some(tag.clone());
                if let Some(effect) = effects.pop() {
                    effects.push(effect.tag(tag));
                }
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::DealDamageEqualToPower { source, target } => {
            let (source_spec, mut choices) =
                resolve_target_spec_with_choices(source, &current_reference_env(ctx))?;
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

            if source_spec.is_target() {
                let source_tag = ctx.next_tag("damage_source");
                effects.push(
                    Effect::new(crate::effects::TargetOnlyEffect::new(source_spec.clone()))
                        .tag(source_tag.clone()),
                );
                damage_source_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                if source == target {
                    damage_target_spec = ChooseSpec::Tagged(source_tag.as_str().into());
                }
            }

            if !damage_target_spec.is_target()
                && let ChooseSpec::Object(filter) | ChooseSpec::All(filter) =
                    damage_target_spec.base()
            {
                let mut per_target_damage =
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        per_target_source_spec.clone(),
                        Effect::deal_damage(
                            Value::PowerOf(Box::new(per_target_source_spec.clone())),
                            ChooseSpec::Iterated,
                        ),
                    ));
                if ctx.auto_tag_object_targets {
                    let tag = ctx.next_tag("damaged");
                    ctx.last_object_tag = Some(tag.clone());
                    per_target_damage = per_target_damage.tag(tag);
                }
                effects.push(Effect::for_each(filter.clone(), vec![per_target_damage]));
            } else {
                let damage_effect = tag_object_target_effect(
                    Effect::new(crate::effects::ExecuteWithSourceEffect::new(
                        damage_source_spec.clone(),
                        Effect::deal_damage(
                            Value::PowerOf(Box::new(damage_source_spec.clone())),
                            damage_target_spec.clone(),
                        ),
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
            prelude.push(Effect::tap_all(resolved_filter));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::UntapAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("untapped");
                prelude.push(Effect::new(
                    crate::effects::TagMatchingObjectsEffect::new(resolved_filter.clone(), tag.clone()),
                ));
                ctx.last_object_tag = Some(tag);
            }
            prelude.push(Effect::untap_all(resolved_filter));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::TapOrUntap { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let modes = vec![
                EffectMode {
                    description: "Tap".to_string(),
                    effects: vec![Effect::tap(spec.clone())],
                },
                EffectMode {
                    description: "Untap".to_string(),
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
                    description: "Tap".to_string(),
                    effects: vec![Effect::tap_all(resolved_tap)],
                },
                EffectMode {
                    description: "Untap".to_string(),
                    effects: vec![Effect::untap_all(resolved_untap)],
                },
            ];
            prelude.push(Effect::choose_one(modes));
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::PhaseOut { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let base_effect = if spec.is_target() {
                Effect::phase_out(spec.clone())
            } else {
                Effect::new(crate::effects::PhaseOutEffect::with_spec(spec.clone()))
            };
            let effect = tag_object_target_effect(base_effect, &spec, ctx, "phased_out");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PhaseOutAll { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            prelude.push(Effect::new(crate::effects::PhaseOutEffect::with_spec(
                ChooseSpec::all(resolved_filter),
            )));
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
                    description,
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
                let target_tag = if let ChooseSpec::Tagged(tag) = &target_spec {
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
        SubjectVerbActionAst::Exile { target, face_down } => {
            if let Some(compiled) = lower_hand_exile_target(target, *face_down, ctx)? {
                return Ok(compiled);
            }
            if let Some(compiled) = lower_counted_non_target_exile_target(target, *face_down, ctx)?
            {
                return Ok(compiled);
            }
            if let Some(compiled) = lower_single_non_target_exile_target(target, *face_down, ctx)? {
                return Ok(compiled);
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
            if spec.is_target() {
                let tag = ctx.next_tag("exiled");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
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
                    ctx.last_object_tag = Some(tag);
                }
            }
            prelude.push(effect);
            Ok((prelude, choices))
        }
        SubjectVerbActionAst::LookAtHand { target } => {
            let (effects, choices) = compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::LookAtHandEffect::new(spec))
            })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::Counter { target } => {
            compile_tagged_effect_for_target(target, ctx, "countered", Effect::counter)
        }
        SubjectVerbActionAst::CounterUnlessPays { target, cost } => {
            let cost = cost.clone();
            compile_tagged_effect_for_target(target, ctx, "countered", |spec| {
                Effect::counter_unless_pays_total_cost(spec, cost.clone())
            })
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
            let mut spec = base_spec;
            if let Some(target_count) = target_count {
                spec = spec.with_count(*target_count);
            }
            let mut put_counters =
                crate::effects::PutCountersEffect::new(*counter_type, count.clone(), spec.clone());
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
        SubjectVerbActionAst::PutCountersAll {
            counter_type,
            count,
            filter,
        } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let resolved_count = resolve_value_it_tag(count, &current_reference_env(ctx))?;
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::put_counters(
                    *counter_type,
                    resolved_count,
                    ChooseSpec::Iterated,
                )],
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RemoveUpToAnyCounters {
            amount,
            target,
            counter_type,
            up_to,
        } => {
            let resolved_amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let id = ctx.next_effect_id();
            ctx.last_effect_id = Some(id);
            let compiled = compile_tagged_effect_for_target(target, ctx, "counters", |spec| {
                let resolved_amount = match (&resolved_amount, counter_type) {
                    (Value::CountersOn(counter_source, amount_counter_type), Some(counter_type))
                        if matches!(counter_source.as_ref(), ChooseSpec::Source)
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
        SubjectVerbActionAst::ForEachCounterKindPutOrRemove { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            Ok((
                vec![Effect::new(
                    crate::effects::ForEachCounterKindPutOrRemoveEffect::new(spec),
                )],
                choices,
            ))
        }
        SubjectVerbActionAst::ReturnToHand { target, random } => {
            let (mut spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let from_graveyard = target_mentions_graveyard(target);
            if from_graveyard && format!("{spec:?}").contains("IteratedPlayer") {
                replace_iterated_player_with_target_player_in_choose_spec(&mut spec);
            }
            let effect = tag_object_target_effect(
                if from_graveyard {
                    Effect::return_from_graveyard_to_hand_with_random(spec.clone(), *random)
                } else {
                    Effect::new(crate::effects::ReturnToHandEffect::with_spec(spec.clone()))
                },
                &spec,
                ctx,
                "returned",
            );
            ctx.last_player_filter = Some(if spec.is_target() {
                PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
            } else if let Some(tag) = ctx.last_object_tag.clone() {
                PlayerFilter::AliasedOwnerOf(ObjectRef::tagged(TagKey::from(tag.as_str())))
            } else {
                PlayerFilter::AliasedOwnerOf(ObjectRef::Target)
            });
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::ReturnAllToHand { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            Ok((
                vec![Effect::return_all_to_hand(resolved_filter)],
                Vec::new(),
            ))
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
                    description,
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
            let iterated = ChooseSpec::Iterated;
            let count = Value::CountersOn(Box::new(iterated.clone()), Some(*counter_type));
            let effect = Effect::for_each(
                resolved_filter,
                vec![Effect::put_counters(*counter_type, count, iterated)],
            );
            Ok((vec![effect], Vec::new()))
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
            let resolved_filter = if let Some(filter) = filter {
                let mut resolved = resolve_it_tag(filter, &current_reference_env(ctx))?;
                if resolved.zone.is_none() {
                    resolved.zone = Some(Zone::Hand);
                }
                Some(resolved)
            } else {
                None
            };
            let (resolved_player, choices) =
                if matches!(subject_verb.subject.player, PlayerAst::Implicit) {
                    if let Some(inferred_player) = resolved_filter
                        .as_ref()
                        .and_then(infer_player_filter_from_object_filter)
                        .or_else(|| ctx.last_player_filter.clone())
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
            let mut resolved_count = count.clone();
            subject.apply_player_refs_to_value(&mut resolved_count, ctx);
            let resolved_filter = resolved_filter
                .map(|resolved| subject.bind_discard_filter(&resolved, ctx))
                .transpose()?;
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
        SubjectVerbActionAst::PayAnyEnergy => {
            let subject = resolve_subject_verb_subject(role, player, ctx, false, false, true)?;
            compile_player_effect_from_resolved_filter(
                subject.into_player_filter(),
                subject.into_choices(),
                || {
                    Effect::new(crate::effects::PayAnyEnergyEffect::new(ChooseSpec::Player(
                        PlayerFilter::You,
                    )))
                },
                |filter| {
                    Effect::new(crate::effects::PayAnyEnergyEffect::new(ChooseSpec::Player(
                        filter,
                    )))
                },
            )
        }
        SubjectVerbActionAst::PayMana { cost } => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::new(crate::effects::PayManaEffect::new(
                    cost.clone(),
                    ChooseSpec::Player(subject.into_player_filter()),
                ))
            })
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
            let mut lowered = lower_granted_abilities_ast(std::slice::from_ref(ability))?;
            let Some(ability) = lowered.pop() else {
                return Err(CardTextError::ParseError(
                    "temporary next-spell grant did not lower to a static ability".to_string(),
                ));
            };
            Ok((
                vec![Effect::grant_next_spell_ability_this_turn(
                    player_filter,
                    resolved_filter,
                    ability,
                )],
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
        SubjectVerbActionAst::CreateEmblem { text } => {
            let emblem = compile_emblem_description_from_text(text)?;
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
        SubjectVerbActionAst::Goad { target } => {
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
            let effect = tag_object_target_effect(Effect::goad(spec.clone()), &spec, ctx, "goaded");
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
                return Ok((vec![Effect::clear_all_suspected()], Vec::new()));
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
        SubjectVerbActionAst::Regenerate { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect = tag_object_target_effect(
                Effect::regenerate(spec.clone(), crate::effect::Until::EndOfTurn),
                &spec,
                ctx,
                "regenerated",
            );
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
                return Ok((effects, choices));
            }
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let chooser = subject.clone_player_filter();
            let target_prelude = subject.target_prelude();
            let resolved_filter = match subject.bind_sacrifice_filter(filter, ctx) {
                Ok(resolved) => resolved,
                Err(_)
                    if filter.tagged_constraints.len() == 1
                        && filter.tagged_constraints[0].tag.as_str() == IT_TAG =>
                {
                    ObjectFilter::source()
                }
                Err(err) => return Err(err),
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
                return Ok((effects, subject.into_choices()));
            }
            if *count == 1
                && let Some(tag) = object_filter_as_tagged_reference(&resolved_filter)
            {
                let mut effects = target_prelude;
                effects.push(Effect::new(crate::effects::SacrificeTargetEffect::new(
                    ChooseSpec::tagged(tag),
                )));
                return Ok((effects, subject.into_choices()));
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
    }
}

fn compile_put_some_into_hand_rest_to_zone(
    role: SubjectRole,
    player: PlayerAst,
    count: ChoiceCount,
    rest_zone: Zone,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    use crate::effect::Condition;
    use crate::effects::consult_helpers::LibraryBottomOrder;
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

    let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
        CardTextError::ParseError("unable to resolve 'them' without prior reference".to_string())
    })?;
    let subject = resolve_subject_verb_subject(role, player, ctx, true, true, false)?;
    let chooser = subject.as_chooser();
    let player_filter = subject.clone_player_filter();
    let choices = subject.into_choices();

    let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
    choose_filter.zone = Some(Zone::Library);
    let chosen_tag = ctx.next_tag("chosen");
    let chosen_tag_key: TagKey = chosen_tag.as_str().into();
    let choose = Effect::new(
        crate::effects::ChooseObjectsEffect::new(
            choose_filter,
            count,
            chooser,
            chosen_tag_key.clone(),
        )
        .in_zone(Zone::Library),
    );
    let move_chosen = Effect::for_each_tagged(
        chosen_tag,
        vec![Effect::move_to_zone(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        )],
    );

    let mut membership_filter = ObjectFilter::default();
    membership_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from("__it__"),
            relation: TaggedOpbjectRelation::SameStableId,
        });
    let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key.clone(), membership_filter);
    let move_rest = if rest_zone == Zone::Library {
        Effect::put_tagged_remainder_on_library_bottom(
            TagKey::from(looked_tag.as_str()),
            Some(chosen_tag_key),
            LibraryBottomOrder::Random,
            player_filter,
        )
    } else {
        Effect::for_each_tagged(
            looked_tag,
            vec![Effect::conditional(
                in_chosen,
                Vec::new(),
                vec![Effect::move_to_zone(ChooseSpec::Iterated, rest_zone, false)],
            )],
        )
    };

    Ok((vec![choose, move_chosen, move_rest], choices))
}

fn subject_verb_role(role: SubjectVerbRoleAst) -> SubjectRole {
    match role {
        SubjectVerbRoleAst::Actor => SubjectRole::Actor,
        SubjectVerbRoleAst::AffectedPlayer => SubjectRole::AffectedPlayer,
        SubjectVerbRoleAst::Chooser => SubjectRole::Chooser,
        SubjectVerbRoleAst::LibraryOwner => SubjectRole::LibraryOwner,
        SubjectVerbRoleAst::ZoneOwner => SubjectRole::ZoneOwner,
    }
}

fn resolve_subject_verb_subject(
    role: SubjectRole,
    player: PlayerAst,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
) -> Result<LoweredSubject, CardTextError> {
    match role {
        SubjectRole::Actor => LoweredSubject::resolve_actor(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::AffectedPlayer => LoweredSubject::resolve_affected_player(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::Chooser => LoweredSubject::resolve_chooser(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::LibraryOwner => LoweredSubject::resolve_library_owner(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
        SubjectRole::ZoneOwner => LoweredSubject::resolve_zone_owner(
            player,
            ctx,
            allow_target,
            allow_target_opponent,
            track_last_player_filter,
        ),
    }
}

fn compile_subject_verb_player_value_effect<YouBuilder, OtherBuilder>(
    role: SubjectRole,
    player: PlayerAst,
    value: &Value,
    ctx: &mut EffectLoweringContext,
    allow_target: bool,
    allow_target_opponent: bool,
    track_last_player_filter: bool,
    resolve_it_tags: bool,
    build_you: YouBuilder,
    build_other: OtherBuilder,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError>
where
    YouBuilder: FnOnce(Value) -> Effect,
    OtherBuilder: FnOnce(Value, PlayerFilter) -> Effect,
{
    let subject = resolve_subject_verb_subject(
        role,
        player,
        ctx,
        allow_target,
        allow_target_opponent,
        track_last_player_filter,
    )?;
    let value = subject.bind_player_refs_in_value(value, ctx)?;
    let value = if resolve_it_tags {
        resolve_value_it_tag(&value, &current_reference_env(ctx))?
    } else {
        value
    };
    let you_value = value.clone();
    let (player_filter, choices) = subject.into_parts();
    let value = per_player_partition_value_for_filter(value, &player_filter);
    let you_value = per_player_partition_value_for_filter(you_value, &PlayerFilter::You);
    compile_player_effect_from_resolved_filter(
        player_filter,
        choices,
        || build_you(you_value),
        |filter| build_other(value, filter),
    )
}

fn per_player_partition_value_for_filter(value: Value, player_filter: &PlayerFilter) -> Value {
    if !matches!(player_filter, PlayerFilter::IteratedPlayer) {
        return value;
    }
    match value {
        Value::EffectValue(effect_id) => Value::EffectMetric {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::IteratedPlayerCount,
        },
        Value::EffectValueOffset(effect_id, offset) => Value::EffectMetricOffset {
            effect_id,
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::IteratedPlayerCount,
            offset,
        },
        other => other,
    }
}

fn replace_iterated_player_with_target_player_in_choose_spec(spec: &mut ChooseSpec) {
    match spec {
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => {
            replace_iterated_player_with_target_player_in_choose_spec(spec);
        }
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            replace_iterated_player_with_target_player_in_object_filter(filter);
        }
        ChooseSpec::Player(filter)
        | ChooseSpec::EachPlayer(filter)
        | ChooseSpec::PlayerOrPlaneswalker(filter) => {
            replace_iterated_player_with_target_player(filter);
        }
        _ => {}
    }
}

fn replace_iterated_player_with_target_player_in_object_filter(filter: &mut ObjectFilter) {
    if let Some(owner) = &mut filter.owner {
        replace_iterated_player_with_target_player(owner);
    }
    if let Some(controller) = &mut filter.controller {
        replace_iterated_player_with_target_player(controller);
    }
    for nested in &mut filter.any_of {
        replace_iterated_player_with_target_player_in_object_filter(nested);
    }
}

fn replace_iterated_player_with_target_player(filter: &mut PlayerFilter) {
    match filter {
        PlayerFilter::IteratedPlayer => {
            *filter = PlayerFilter::target_player();
        }
        PlayerFilter::Target(inner) => replace_iterated_player_with_target_player(inner),
        _ => {}
    }
}

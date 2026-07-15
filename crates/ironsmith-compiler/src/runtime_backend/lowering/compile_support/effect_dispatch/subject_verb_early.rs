use super::*;

pub(super) fn compile_subject_verb_early(
    subject_verb: &SubjectVerbEffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<EffectCompileOutcome>, CardTextError> {
    let role = subject_verb_role(subject_verb.subject.role);
    let player = subject_verb.subject.player;
    let result = match &subject_verb.action {
        SubjectVerbActionAst::Draw { count } => compile_subject_verb_player_value_effect(
            role,
            player,
            count,
            ctx,
            true,
            true,
            true,
            true,
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
            true,
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
            true,
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
            true,
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
        SubjectVerbActionAst::EmitKeywordAction { action, amount } => Ok((
            vec![Effect::emit_keyword_action(*action, *amount)],
            Vec::new(),
        )),
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
            next_end_step_player,
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
                    .next_end_step_player(next_end_step_player.clone())
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
        SubjectVerbActionAst::Endure { target, amount } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let Value::Fixed(token_size) = amount.clone() else {
                return Err(CardTextError::ParseError(
                    "unsupported variable endure token size".to_string(),
                ));
            };
            if token_size < 0 {
                return Err(CardTextError::ParseError(
                    "unsupported negative endure count".to_string(),
                ));
            }
            let token = CardDefinitionBuilder::new(CardId::new(), "Spirit")
                .token()
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Spirit])
                .color_indicator(ColorSet::WHITE)
                .power_toughness(PowerToughness::fixed(token_size, token_size))
                .build();
            let amount_text = describe_value_for_mode(&amount);
            let counter_description = if amount == Value::Fixed(1) {
                "Put a +1/+1 counter on it".to_string()
            } else {
                format!("Put {amount_text} +1/+1 counters on it")
            };
            let token_description = if amount == Value::Fixed(1) {
                "Create a 1/1 white Spirit creature token".to_string()
            } else {
                format!("Create a {amount_text}/{amount_text} white Spirit creature token")
            };
            let modes = vec![
                crate::effect::EffectMode::new(
                    counter_description,
                    vec![Effect::put_counters(
                        crate::object::CounterType::PlusOnePlusOne,
                        amount.clone(),
                        spec,
                    )],
                ),
                crate::effect::EffectMode::new(
                    token_description,
                    vec![Effect::create_tokens(token, Value::Fixed(1))],
                ),
            ];
            Ok((vec![Effect::choose_one(modes)], choices))
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
        SubjectVerbActionAst::ManifestTopCardOfLibrary
        | SubjectVerbActionAst::CloakTopCardOfLibrary => {
            let cloak = matches!(
                &subject_verb.action,
                SubjectVerbActionAst::CloakTopCardOfLibrary
            );
            let (mut effects, choices) =
                compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                    let player = subject.into_player_filter();
                    if cloak {
                        Effect::cloak_top_card_of_library(player)
                    } else {
                        Effect::manifest_top_card_of_library(player)
                    }
                })?;
            if ctx.auto_tag_object_targets {
                let tag =
                    reserved_or_next_object_tag(ctx, if cloak { "cloaked" } else { "manifested" });
                let manifest = effects.pop().ok_or_else(|| {
                    CardTextError::InvariantViolation(
                        "manifest-top lowering produced no effect to tag".to_string(),
                    )
                })?;
                effects.push(manifest.tag(tag.clone()));
                ctx.last_object_tag = Some(tag);
            }
            Ok((effects, choices))
        }
        SubjectVerbActionAst::ManifestCardFromHand => {
            let mut effect = Effect::manifest_card_from_hand();
            if ctx.auto_tag_object_targets {
                let tag = reserved_or_next_object_tag(ctx, "manifested");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            Ok((vec![effect], Vec::new()))
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
        SubjectVerbActionAst::RollDie { sides, die_text } => {
            compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
                Effect::roll_die_with_die_text(
                    *sides,
                    subject.into_player_filter(),
                    die_text.clone(),
                )
            })
        }
        SubjectVerbActionAst::RollDiceChooseResult {
            count,
            sides,
            die_text,
        } => compile_player_role_effect(role, player, ctx, false, false, true, |subject| {
            Effect::roll_dice_choose_result_with_die_text(
                *count,
                *sides,
                subject.into_player_filter(),
                die_text.clone(),
            )
        }),
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
        SubjectVerbActionAst::ChooseLandType { exclude_basic } => {
            compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                Effect::choose_land_type(subject.into_player_filter(), *exclude_basic)
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
        SubjectVerbActionAst::NoteLifeTotal => Ok((vec![Effect::note_life_total()], Vec::new())),
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
            distinct_colors,
        } => {
            let (amount, player_filter, choices) =
                resolve_player_scoped_value(amount, player, ctx, true, true, true)?;
            compile_player_effect_from_resolved_filter(
                player_filter,
                choices,
                || {
                    if let Some(colors) = available_colors.clone() {
                        if *distinct_colors {
                            Effect::add_mana_of_different_colors_restricted(amount.clone(), colors)
                        } else {
                            Effect::add_mana_of_any_color_restricted(amount.clone(), colors)
                        }
                    } else if *distinct_colors {
                        Effect::add_mana_of_different_colors(amount.clone())
                    } else {
                        Effect::add_mana_of_any_color(amount.clone())
                    }
                },
                |filter| {
                    if let Some(colors) = available_colors.clone() {
                        if *distinct_colors {
                            Effect::add_mana_of_different_colors_restricted_player(
                                amount.clone(),
                                filter,
                                colors,
                            )
                        } else {
                            Effect::add_mana_of_any_color_restricted_player(
                                amount.clone(),
                                filter,
                                colors,
                            )
                        }
                    } else if *distinct_colors {
                        Effect::add_mana_of_different_colors_player(amount.clone(), filter)
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
            mana_type_source,
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
                        *mana_type_source,
                    )
                },
                |filter| {
                    Effect::add_mana_of_land_produced_types_player(
                        amount.clone(),
                        filter,
                        land_filter.clone(),
                        *allow_colorless,
                        *same_type,
                        *mana_type_source,
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
        SubjectVerbActionAst::Unattach { object } => {
            let (mut objects, choices) =
                resolve_target_spec_with_choices(object, &current_reference_env(ctx))?;
            if let ChooseSpec::WithCount(inner, count) = objects.clone()
                && count.is_any_number()
                && let ChooseSpec::Object(filter) = inner.base()
            {
                objects = ChooseSpec::All(filter.clone());
            }
            Ok((vec![Effect::unattach_objects(objects)], choices))
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
            library_placement,
            duration,
            optional,
            choice_description,
            counters,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::OneShot
                }
                crate::cards::builders::ZoneReplacementDurationAst::UntilEndOfTurn => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
                crate::cards::builders::ZoneReplacementDurationAst::Persistent => {
                    crate::effects::ReplacementApplyMode::Resolution
                }
            };
            let mut replacement = crate::effects::RegisterZoneReplacementEffect::new(
                spec,
                *from_zone,
                *to_zone,
                *replacement_zone,
                mode,
            )
            .with_counters(counters.clone());
            if let Some(placement) = library_placement {
                replacement = replacement.with_library_placement(*placement);
            }
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
            cause_policy,
            link_exiled_to_source,
        } => {
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::OneShot
                }
                crate::cards::builders::ZoneReplacementDurationAst::UntilEndOfTurn => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
                crate::cards::builders::ZoneReplacementDurationAst::Persistent => {
                    crate::effects::ReplacementApplyMode::Resolution
                }
            };
            let mut replacement = crate::effects::RegisterFutureZoneReplacementEffect::new(
                filter.clone(),
                *from_zone,
                *to_zone,
                *replacement_zone,
                mode,
            );
            if matches!(
                cause_policy,
                crate::cards::builders::FutureZoneReplacementCausePolicyAst::ChangedObjectIsCause
            ) {
                replacement = replacement
                    .with_cause_filter(crate::events::cause::CauseFilter::effect_like())
                    .requiring_cause_source_match();
            }
            if *link_exiled_to_source {
                replacement = replacement.linking_exiled_to_source();
            }
            let effect = Effect::new(replacement);
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RegisterDrawReplacement {
            player,
            replacement_effects,
            duration,
        } => {
            let player_filter = player.clone();
            let mut choices = Vec::new();
            let (replacement_effects, replacement_choices) =
                compile_effects(replacement_effects, ctx)?;
            for choice in replacement_choices {
                push_choice(&mut choices, choice);
            }
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::OneShot
                }
                crate::cards::builders::ZoneReplacementDurationAst::UntilEndOfTurn => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
                crate::cards::builders::ZoneReplacementDurationAst::Persistent => {
                    crate::effects::ReplacementApplyMode::Resolution
                }
            };
            let effect = Effect::new(crate::effects::RegisterDrawReplacementEffect::new(
                player_filter,
                replacement_effects,
                mode,
            ));
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::RegisterManaReplacement {
            source_filter,
            replacement_mana,
            mode,
        } => {
            let effect = Effect::new(crate::effects::RegisterManaReplacementEffect::new(
                source_filter.clone(),
                replacement_mana.clone(),
                *mode,
            ));
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RegisterDamagedBySourceZoneReplacement {
            filter,
            from_zone,
            to_zone,
            replacement_zone,
            duration,
        } => {
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
                crate::cards::builders::ZoneReplacementDurationAst::UntilEndOfTurn => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
                crate::cards::builders::ZoneReplacementDurationAst::Persistent => {
                    crate::effects::ReplacementApplyMode::Resolution
                }
            };
            let effect = Effect::new(
                crate::effects::RegisterDamagedBySourceZoneReplacementEffect::new(
                    filter.clone(),
                    *from_zone,
                    *to_zone,
                    *replacement_zone,
                    mode,
                ),
            );
            Ok((vec![effect], Vec::new()))
        }
        SubjectVerbActionAst::RegisterEnterUnderControlReplacement { filter, duration } => {
            let mode = match duration {
                crate::cards::builders::ZoneReplacementDurationAst::OneShot => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
                crate::cards::builders::ZoneReplacementDurationAst::UntilEndOfTurn => {
                    crate::effects::ReplacementApplyMode::UntilEndOfTurn
                }
                crate::cards::builders::ZoneReplacementDurationAst::Persistent => {
                    crate::effects::ReplacementApplyMode::Resolution
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
            this_combat,
        } => Ok((
            vec![if *this_combat {
                Effect::control_combat_choices_this_combat(*attackers, *blockers)
            } else {
                Effect::control_combat_choices_this_turn(*attackers, *blockers)
            }],
            Vec::new(),
        )),
        SubjectVerbActionAst::GainControl {
            target,
            duration,
            condition,
            source_reference_surface,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (controller, subject_choices) = if matches!(player, PlayerAst::Implicit) {
                if ctx.iterated_player && choose_spec_owned_by_iterated_player(&spec) {
                    (PlayerFilter::IteratedPlayer, Vec::new())
                } else {
                    (PlayerFilter::You, Vec::new())
                }
            } else {
                let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
                (subject.into_player_filter(), subject.into_choices())
            };
            choices.extend(subject_choices);
            let runtime_modification = if matches!(controller, PlayerFilter::You) {
                crate::effects::continuous::RuntimeModification::ChangeControllerToEffectController
            } else {
                crate::effects::continuous::RuntimeModification::ChangeControllerToPlayer(
                    controller,
                )
            };
            let mut continuous_effect = crate::effects::ApplyContinuousEffect::with_spec_runtime(
                spec.clone(),
                runtime_modification,
                duration.clone(),
            );
            if let Some(condition) = condition {
                continuous_effect = continuous_effect.with_condition(condition.clone());
            }
            if let Some(surface) = source_reference_surface {
                continuous_effect =
                    continuous_effect.with_source_reference_surface(surface.clone());
            }
            let effect =
                tag_object_target_effect(Effect::new(continuous_effect), &spec, ctx, "controlled");
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::RevealTop => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let tag = ctx.next_tag("revealed");
            ctx.last_object_tag = Some(tag.clone());
            ctx.last_revealed_tag = Some(tag.clone());
            ctx.last_revealed_zone = Some(Zone::Library);
            ctx.last_revealed_player_filter = Some(player_filter.clone());
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
            let exiled_is_plural = !matches!(&resolved_count, crate::effect::Value::Fixed(1));
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
            if let Some(tag) = tags.first().or_else(|| accumulated_tags.first()) {
                let resolved_tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
                if is_sentence_helper_exiled_collection_tag(resolved_tag.as_str()) {
                    ctx.last_exiled_collection_tag = Some(resolved_tag.as_str().to_string());
                }
                ctx.last_exiled_collection_is_plural = exiled_is_plural;
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
            ctx.last_revealed_tag = Some(resolved_tag.clone());
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
            ctx.last_revealed_zone = Some(Zone::Hand);
            ctx.last_revealed_player_filter = Some(player_filter.clone());
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
                ctx.last_revealed_zone = Some(Zone::Library);
                ctx.last_revealed_player_filter = Some(player_filter.clone());
            }
            let effect = if *reveal {
                Effect::reveal_top_cards(player_filter, count.clone(), resolved_tag)
            } else {
                Effect::look_at_top_cards(player_filter, count.clone(), resolved_tag)
            };
            Ok((vec![effect], subject.into_choices()))
        }
        SubjectVerbActionAst::PutOntoBattlefield {
            target,
            tapped,
            controller,
        } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let chooser = subject.clone_player_filter();
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let controller_filter = match controller {
                ReturnControllerAst::Preserve => chooser,
                ReturnControllerAst::You => PlayerFilter::You,
                ReturnControllerAst::Owner => {
                    return Err(CardTextError::ParseError(
                        "put-onto-battlefield under owner control is unsupported".to_string(),
                    ));
                }
            };
            let mut all_choices = subject.into_choices();
            for choice in choices {
                push_choice(&mut all_choices, choice);
            }
            let mut effect = Effect::put_onto_battlefield(spec.clone(), *tapped, controller_filter);
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = reserved_or_next_object_tag(ctx, "moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            Ok((vec![effect], all_choices))
        }
        SubjectVerbActionAst::LookAtObjects { filter } => {
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            let player_filter = subject.clone_player_filter();
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            resolved_filter
                .controller
                .get_or_insert(player_filter.clone());
            Ok((
                vec![Effect::new(crate::effects::LookAtObjectsEffect::new(
                    resolved_filter,
                    PlayerFilter::You,
                    player_filter,
                ))],
                subject.into_choices(),
            ))
        }
        SubjectVerbActionAst::LookAtTarget { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if !choose_spec_targets_object(&spec) {
                return Err(CardTextError::ParseError(
                    "look-at-target object clause requires an object target".to_string(),
                ));
            }
            let tag = TagKey::from(ctx.next_tag("targeted").as_str());
            ctx.last_object_tag = Some(tag.as_str().to_string());
            Ok((
                vec![
                    Effect::new(crate::effects::TargetOnlyEffect::new(spec)).tag(tag.clone()),
                    Effect::new(crate::effects::LookAtObjectsEffect::new(
                        ObjectFilter::tagged(tag),
                        PlayerFilter::You,
                        PlayerFilter::You,
                    )),
                ],
                choices,
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
            let follows_search_collection = ctx
                .last_object_tag
                .as_ref()
                .is_some_and(|tag| is_searched_collection_tag(tag));
            if !ast_is_explicit_you
                && follows_search_collection
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
            } else if matches!(player, PlayerAst::That)
                && !ctx.iterated_player
                && ctx
                    .last_player_filter
                    .as_ref()
                    .is_some_and(|filter| !is_you_player_filter(filter))
            {
                Ok((
                    vec![Effect::shuffle_library_player(as_followup_player_alias(
                        ctx.last_player_filter.clone().expect("checked above"),
                    ))],
                    Vec::new(),
                ))
            } else if matches!(player, PlayerAst::That) && !ctx.iterated_player {
                Ok((
                    vec![Effect::shuffle_library_player(PlayerFilter::target_player())],
                    vec![ChooseSpec::target_player()],
                ))
            } else {
                compile_player_role_effect(role, player, ctx, true, true, true, |subject| {
                    Effect::shuffle_library_player(subject.into_player_filter())
                })
            }
        }
        SubjectVerbActionAst::ShuffleObjectsIntoLibrary {
            target,
            all,
            owner_library_destination,
        } => {
            let (mut spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            if *all && let ChooseSpec::Object(filter) = spec {
                spec = ChooseSpec::All(filter);
            }
            let subject = resolve_subject_verb_subject(role, player, ctx, true, true, true)?;
            for choice in subject.into_choices() {
                push_choice(&mut choices, choice);
            }
            let mut shuffle = crate::effects::ShuffleObjectsIntoLibraryEffect::new(
                spec.clone(),
                subject.into_player_filter(),
            );
            if *owner_library_destination {
                shuffle = shuffle.with_owner_library_destination();
            }
            let mut effect = Effect::new(shuffle);
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
            chooser,
            allow_colorless,
            allow_artifacts,
        } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let chooser = if matches!(chooser, PlayerAst::ItsController) {
                PlayerFilter::ControllerOf(crate::target::ObjectRef::Target)
            } else {
                resolve_non_target_player_filter(*chooser, &current_reference_env(ctx))?
            };
            let mut modes = Vec::new();
            if *allow_colorless {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::Colorless);
                modes.push(EffectMode {
                    source_text: "Colorless".to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }
            if *allow_artifacts {
                let ability = StaticAbility::protection(crate::ability::ProtectionFrom::CardType(
                    crate::types::CardType::Artifact,
                ));
                modes.push(EffectMode {
                    source_text: "Artifacts".to_string(),
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
                    source_text: name.to_string(),
                    effects: vec![Effect::new(
                        crate::effects::GrantAbilitiesTargetEffect::new(
                            spec.clone(),
                            vec![ability],
                            crate::effect::Until::EndOfTurn,
                        ),
                    )],
                });
            }
            let effect = tag_object_target_effect(
                Effect::new(
                    crate::effects::ChooseModeEffect::choose_one(modes).with_chooser(chooser),
                ),
                &spec,
                ctx,
                "protected",
            );
            Ok((vec![effect], choices))
        }
        SubjectVerbActionAst::PreventAllCombatDamage { duration } => Ok((
            vec![Effect::prevent_all_combat_damage(duration.clone())],
            Vec::new(),
        )),
        SubjectVerbActionAst::AssignNoCombatDamage { source, duration } => {
            compile_effect_for_target(source, ctx, |spec| {
                Effect::assign_no_combat_damage(spec, duration.clone())
            })
        }
        SubjectVerbActionAst::PreventAllCombatDamageFromSource {
            duration,
            source,
            source_would_deal_surface,
        } => compile_effect_for_target(source, ctx, |spec| {
            if *source_would_deal_surface {
                Effect::prevent_all_combat_damage_source_would_deal(spec, duration.clone())
            } else {
                Effect::prevent_all_combat_damage_from(spec, duration.clone())
            }
        }),
        SubjectVerbActionAst::PreventAllCombatDamageFromSourceFilter {
            duration,
            source_filter,
            excluded_source_target,
        } => {
            let mut damage_filter = ironsmith_core::DamageFilter::combat();
            damage_filter.from_source = Some(source_filter.clone());
            let mut effect = crate::effects::PreventAllDamageEffect::all_with_filter(
                damage_filter,
                duration.clone(),
            );
            let mut choices = Vec::new();
            if let Some(excluded_source_target) = excluded_source_target {
                let (spec, target_choices) = resolve_target_spec_with_choices(
                    excluded_source_target,
                    &current_reference_env(ctx),
                )?;
                effect = effect.excluding_target_source(spec);
                choices = target_choices;
            }
            Ok((vec![Effect::new(effect)], choices))
        }
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
            follow_up_effects,
        } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::PreventNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Target(TargetAst::Object(
                    filter,
                    None,
                    None,
                )) => crate::effects::PreventNextTimeDamageSource::ChoiceMatching(
                    resolve_it_tag(filter, &current_reference_env(ctx))?,
                ),
                PreventNextTimeDamageSourceAst::Target(target) => {
                    let (spec, _) =
                        resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                    crate::effects::PreventNextTimeDamageSource::Target(spec)
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::PreventNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            let (target_spec, mut choices) = match target {
                PreventNextTimeDamageTargetAst::AnyTarget => (
                    crate::effects::PreventNextTimeDamageTarget::Omitted,
                    Vec::new(),
                ),
                PreventNextTimeDamageTargetAst::You => {
                    (crate::effects::PreventNextTimeDamageTarget::You, Vec::new())
                }
                PreventNextTimeDamageTargetAst::Target(target) => {
                    if matches!(target, TargetAst::AnyTarget(_)) {
                        (
                            crate::effects::PreventNextTimeDamageTarget::AnyTarget,
                            Vec::new(),
                        )
                    } else {
                        let (spec, choices) =
                            resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
                        (
                            crate::effects::PreventNextTimeDamageTarget::Target(spec),
                            choices,
                        )
                    }
                }
            };
            let mut effect =
                crate::effects::PreventNextTimeDamageEffect::new(source_spec, target_spec);
            let (follow_up_effects, follow_up_choices) = if follow_up_effects.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                let mut follow_up_ctx =
                    EffectLoweringContext::from_parts(ctx.id_gen_context(), ctx.lowering_frame());
                follow_up_ctx.allow_life_event_value = true;
                let compiled = compile_effects(follow_up_effects, &mut follow_up_ctx)?;
                ctx.apply_id_gen_context(follow_up_ctx.id_gen_context());
                compiled
            };
            if !follow_up_effects.is_empty() {
                effect = effect.with_follow_up_effects(follow_up_effects);
            }
            if *reflect_damage_to_source_controller {
                effect = effect.reflecting_to_source_controller();
            }
            choices.extend(follow_up_choices);
            Ok((vec![Effect::new(effect)], choices))
        }
        SubjectVerbActionAst::PreventDamage {
            amount,
            target,
            duration,
            source_of_your_choice,
            protect_you_and_permanents_you_control,
            follow_up_effects,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let (follow_up_effects, follow_up_choices) = if follow_up_effects.is_empty() {
                (Vec::new(), Vec::new())
            } else {
                let mut follow_up_ctx =
                    EffectLoweringContext::from_parts(ctx.id_gen_context(), ctx.lowering_frame());
                follow_up_ctx.allow_life_event_value = true;
                let compiled = compile_effects(follow_up_effects, &mut follow_up_ctx)?;
                ctx.apply_id_gen_context(follow_up_ctx.id_gen_context());
                compiled
            };
            if *protect_you_and_permanents_you_control {
                let mut prevent = crate::effects::PreventDamageEffect::new(
                    amount,
                    ChooseSpec::SourceController,
                    duration.clone(),
                )
                .protecting_you_and_permanents_you_control();
                if *source_of_your_choice {
                    prevent = prevent.with_source_of_your_choice();
                }
                if !follow_up_effects.is_empty() {
                    prevent = prevent.with_follow_up_effects(follow_up_effects);
                }
                return Ok(Some((vec![Effect::new(prevent)], follow_up_choices)));
            }
            if let TargetAst::Object(filter, explicit_target_span, reference_span) = target
                && explicit_target_span.is_none()
                && reference_span.is_none()
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
                Ok((vec![effect], follow_up_choices))
            } else {
                let (effects, mut choices) = compile_effect_for_target(target, ctx, |spec| {
                    if *source_of_your_choice {
                        let mut prevent = crate::effects::PreventDamageEffect::new(
                            amount.clone(),
                            spec,
                            duration.clone(),
                        )
                        .with_source_of_your_choice();
                        if !follow_up_effects.is_empty() {
                            prevent = prevent.with_follow_up_effects(follow_up_effects.clone());
                        }
                        Effect::new(prevent)
                    } else if !follow_up_effects.is_empty() {
                        Effect::new(
                            crate::effects::PreventDamageEffect::new(
                                amount.clone(),
                                spec,
                                duration.clone(),
                            )
                            .with_follow_up_effects(follow_up_effects.clone()),
                        )
                    } else {
                        Effect::prevent_damage(amount.clone(), spec, duration.clone())
                    }
                })?;
                if !follow_up_effects.is_empty() {
                    choices.extend(follow_up_choices);
                }
                Ok((effects, choices))
            }
        }
        SubjectVerbActionAst::PreventAllDamageToTarget {
            target,
            duration,
            source_of_your_choice,
            source_choice_shares_activation_mana_color,
            source_target,
        } => {
            if let Some(source_target) = source_target {
                let (source_spec, choices) = resolve_target_spec_with_choices(
                    source_target,
                    &current_reference_env(ctx),
                )?;
                let protect_source = matches!(target, TargetAst::Source(_));
                let protected = if protect_source {
                    ironsmith_core::PreventionTarget::All
                } else {
                    prevention_target_from_non_choice_target(target, ctx)?
                };
                let mut effect = crate::effects::PreventAllDamageEffect::new(
                    protected,
                    ironsmith_core::DamageFilter::all(),
                    duration.clone(),
                )
                .with_target_source(source_spec);
                if protect_source {
                    effect = effect.protecting_source();
                }
                return Ok(Some((vec![Effect::new(effect)], choices)));
            }
            if *source_of_your_choice
                && let TargetAst::Player(crate::target::PlayerFilter::You, _) = target
            {
                let mut effect = crate::effects::PreventAllDamageEffect::new(
                    ironsmith_core::PreventionTarget::You,
                    ironsmith_core::DamageFilter::all(),
                    duration.clone(),
                );
                effect = if *source_choice_shares_activation_mana_color {
                    effect.with_source_choice_sharing_activation_mana_color()
                } else {
                    effect.with_source_of_your_choice()
                };
                return Ok(Some((vec![Effect::new(effect)], Vec::new())));
            }
            if let TargetAst::ObjectOrPlayer(
                filter,
                crate::target::PlayerFilter::You,
                explicit_target_span,
            ) = target
                && explicit_target_span.is_none()
            {
                let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                let mut effect = crate::effects::PreventAllDamageEffect::new(
                    ironsmith_core::PreventionTarget::YouAndPermanentsMatching(resolved_filter),
                    ironsmith_core::DamageFilter::all(),
                    duration.clone(),
                );
                if *source_of_your_choice {
                    effect = effect.with_source_of_your_choice();
                }
                return Ok(Some((vec![Effect::new(effect)], Vec::new())));
            }
            if let TargetAst::Object(filter, explicit_target_span, _) = target
                && explicit_target_span.is_none()
            {
                let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
                let mut effect = crate::effects::PreventAllDamageEffect::matching(
                    resolved_filter,
                    duration.clone(),
                );
                if *source_of_your_choice {
                    effect = effect.with_source_of_your_choice();
                }
                Ok((vec![Effect::new(effect)], Vec::new()))
            } else {
                if *source_of_your_choice {
                    return Err(CardTextError::ParseError(
                        "prevent-all damage by a source of your choice currently supports non-targeted recipients"
                            .to_string(),
                    ));
                }
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
        SubjectVerbActionAst::PreventAllDamageToTargetFromSourceFilter {
            target,
            duration,
            source_filter,
        } => {
            let protect_source = matches!(target, TargetAst::Source(_));
            let target = if protect_source {
                ironsmith_core::PreventionTarget::All
            } else {
                prevention_target_from_non_choice_target(target, ctx)?
            };
            let source_filter = resolve_it_tag(source_filter, &current_reference_env(ctx))?;
            let mut damage_filter = ironsmith_core::DamageFilter::all();
            damage_filter.from_source = Some(source_filter);
            let mut effect = crate::effects::PreventAllDamageEffect::new(
                target,
                damage_filter,
                duration.clone(),
            );
            if protect_source {
                effect = effect.protecting_source();
            }
            Ok((vec![Effect::new(effect)], Vec::new()))
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
        SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
            amount,
            protected_target,
            destination,
            destination_target,
        } => {
            let amount = resolve_value_it_tag(amount, &current_reference_env(ctx))?;
            let refs = current_reference_env(ctx);
            let (protected_spec, mut choices) = if let Some(protected_target) = protected_target {
                let (spec, choices) = resolve_target_spec_with_choices(protected_target, &refs)?;
                (Some(spec), choices)
            } else {
                (None, Vec::new())
            };
            let effect = match destination {
                crate::cards::builders::RedirectNextTimeDamageDestinationAst::Controller => {
                    let protected_spec = protected_spec.ok_or_else(|| {
                        CardTextError::ParseError(
                            "missing redirected-next damage protected target".to_string(),
                        )
                    })?;
                    crate::effects::RedirectNextDamageToTargetEffect::to_controller(
                        amount,
                        protected_spec,
                    )
                }
                crate::cards::builders::RedirectNextTimeDamageDestinationAst::TargetObject => {
                    let destination_target = destination_target.as_ref().ok_or_else(|| {
                        CardTextError::ParseError(
                            "missing redirected-next damage destination target".to_string(),
                        )
                    })?;
                    let (destination_spec, destination_choices) =
                        resolve_target_spec_with_choices(destination_target, &refs)?;
                    for choice in destination_choices {
                        push_choice(&mut choices, choice);
                    }
                    let mut effect = crate::effects::RedirectNextDamageToTargetEffect::new(
                        amount,
                        destination_spec,
                    );
                    effect.protected_target = protected_spec;
                    effect
                }
                crate::cards::builders::RedirectNextTimeDamageDestinationAst::SourceObject
                | crate::cards::builders::RedirectNextTimeDamageDestinationAst::SourceController => {
                    return Err(CardTextError::ParseError(
                        "unsupported redirected-next damage destination".to_string(),
                    ));
                }
            };
            Ok((vec![Effect::new(effect)], choices))
        }
        SubjectVerbActionAst::RedirectNextTimeDamageToSource {
            source,
            target,
            destination,
            destination_target,
            all_this_turn,
        } => {
            let source_spec = match source {
                PreventNextTimeDamageSourceAst::Choice => {
                    crate::effects::RedirectNextTimeDamageSource::Choice
                }
                PreventNextTimeDamageSourceAst::Target(_) => {
                    return Err(CardTextError::ParseError(
                        "target-referenced redirect damage source is unsupported".to_string(),
                    ));
                }
                PreventNextTimeDamageSourceAst::Filter(filter) => {
                    crate::effects::RedirectNextTimeDamageSource::Filter(resolve_it_tag(
                        filter,
                        &current_reference_env(ctx),
                    )?)
                }
            };
            let refs = current_reference_env(ctx);
            let (protected_spec, mut choices) = resolve_target_spec_with_choices(target, &refs)?;
            let mut effect = crate::effects::RedirectNextTimeDamageToSourceEffect::new(
                source_spec,
                protected_spec,
            );
            effect = match destination {
                crate::cards::builders::RedirectNextTimeDamageDestinationAst::SourceObject => {
                    effect
                }
                crate::cards::builders::RedirectNextTimeDamageDestinationAst::Controller => {
                    effect.to_controller()
                }
                crate::cards::builders::RedirectNextTimeDamageDestinationAst::SourceController => {
                    effect.to_source_controller()
                }
                crate::cards::builders::RedirectNextTimeDamageDestinationAst::TargetObject => {
                    let destination_target = destination_target.as_ref().ok_or_else(|| {
                        CardTextError::ParseError(
                            "missing redirected-next-time damage destination target".to_string(),
                        )
                    })?;
                    let (destination_spec, destination_choices) =
                        resolve_target_spec_with_choices(destination_target, &refs)?;
                    for choice in destination_choices {
                        push_choice(&mut choices, choice);
                    }
                    effect.to_target(destination_spec)
                }
            };
            let effect = if *all_this_turn {
                effect.all_this_turn()
            } else {
                effect
            };
            Ok((vec![Effect::new(effect)], choices))
        }
        SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController { source } => {
            compile_effect_for_target(source, ctx, |spec| {
                Effect::new(
                    crate::effects::RedirectNextTimeDamageToSourceEffect::from_source_target(spec)
                        .to_source_controller()
                        .all_this_turn(),
                )
            })
        }
        SubjectVerbActionAst::RedirectAllDamageThisTurnToTarget {
            player_filter,
            object_filter,
            target,
        } => {
            let object_filter = resolve_it_tag(object_filter, &current_reference_env(ctx))?;
            compile_effect_for_target(target, ctx, |spec| {
                Effect::new(
                    crate::effects::RedirectAllDamageThisTurnToTargetEffect::new(
                        player_filter.clone(),
                        object_filter.clone(),
                        spec,
                    ),
                )
            })
        }
        _ => return Ok(None),
    };
    result.map(Some)
}

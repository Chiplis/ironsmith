use super::*;

fn mark_choose_effects_reveal(mut effects: Vec<Effect>) -> Vec<Effect> {
    for effect in &mut effects {
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.reveal {
            continue;
        }
        *effect = Effect::new(choose.clone().reveal());
    }
    effects
}

pub(super) fn compile_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
    player: PlayerAst,
    order: crate::cards::builders::LibraryBottomOrderAst,
    card_type_modes: &[CardType],
    spell_filter: Option<&ObjectFilter>,
    ctx: &mut EffectLoweringContext,
) -> Result<(Vec<Effect>, Vec<ChooseSpec>), CardTextError> {
    use crate::effect::{Condition, Value, ValueComparisonOperator};
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

    let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
        CardTextError::ParseError(
            "unable to resolve looked-at cards without prior reference".to_string(),
        )
    })?;

    let subject = LoweredSubject::resolve_chooser(player, ctx, true, true, false)?;
    let chooser = subject.clone_player_filter();
    let choices = subject.into_choices();

    let chosen_tag = ctx.next_tag("chosen");
    let chosen_tag_key: TagKey = chosen_tag.as_str().into();

    let mut compiled = Vec::new();
    for card_type in card_type_modes {
        let mut choose_filter = ObjectFilter::default();
        choose_filter.zone = Some(Zone::Library);
        choose_filter.card_types.push(*card_type);
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(looked_tag.as_str()),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: chosen_tag_key.clone(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });

        let choose = Effect::new(
            crate::effects::ChooseObjectsEffect::new(
                choose_filter,
                ChoiceCount::up_to(1),
                chooser.clone(),
                chosen_tag_key.clone(),
            )
            .in_zone(Zone::Library),
        );

        if let Some(spell_filter) = spell_filter {
            let mut typed_spell_filter = (*spell_filter).clone();
            if !typed_spell_filter.card_types.contains(card_type) {
                typed_spell_filter.card_types.push(*card_type);
            }

            compiled.push(Effect::conditional(
                Condition::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching {
                        player: chooser.clone(),
                        filter: typed_spell_filter,
                        exclude_source: false,
                    },
                    operator: ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                vec![choose],
                Vec::new(),
            ));
        } else {
            compiled.push(choose);
        }
    }

    compiled.push(Effect::for_each_tagged(
        chosen_tag.clone(),
        vec![Effect::move_to_zone(
            ChooseSpec::Iterated,
            Zone::Hand,
            false,
        )],
    ));
    compiled.push(Effect::put_tagged_remainder_on_library_bottom(
        looked_tag,
        Some(chosen_tag_key),
        match order {
            crate::cards::builders::LibraryBottomOrderAst::Random => {
                crate::effects::consult_helpers::LibraryBottomOrder::Random
            }
            crate::cards::builders::LibraryBottomOrderAst::ChooserChooses => {
                crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses
            }
        },
        chooser,
    ));

    ctx.last_object_tag = Some(chosen_tag);
    Ok((compiled, choices))
}

pub(super) fn try_compile_visibility_and_card_selection_effect(
    effect: &EffectAst,
    _ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let _ = effect;
    Ok(None)
}

fn chooses_tagged_object_pool(filter: &ObjectFilter) -> bool {
    filter
        .tagged_constraints
        .iter()
        .any(|constraint| matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject))
}

fn scoped_collection_zones() -> Vec<Zone> {
    vec![
        Zone::Battlefield,
        Zone::Hand,
        Zone::Graveyard,
        Zone::Library,
        Zone::Exile,
    ]
}

pub(super) fn try_compile_object_zone_and_exchange_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    let compiled = match effect {
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value,
            player,
            tag,
        } => {
            let subject = LoweredSubject::resolve_chooser(*player, ctx, true, true, false)?;
            let chooser = subject.clone_player_filter();
            let references_revealed_hand = filter.zone == Some(Zone::Hand)
                && filter.owner.is_none()
                && filter.controller.is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let mut resolved_filter =
                if references_revealed_hand && ctx.last_player_filter.is_some() {
                    subject.bind_revealed_hand_choice_filter(filter, ctx)?
                } else if chooses_tagged_object_pool(filter) {
                    subject.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?
                } else if matches!(player, PlayerAst::Implicit) {
                    subject.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?
                } else {
                    subject.bind_battlefield_filter_with_default_controller(filter, ctx)?
                };
            if references_revealed_hand && ctx.last_player_filter.is_some() {
                let has_revealed_collection_tag = ctx
                    .last_object_tag
                    .as_deref()
                    .is_some_and(is_revealed_collection_tag);
                if !has_revealed_collection_tag {
                    resolved_filter.tagged_constraints.retain(|constraint| {
                        !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                    });
                }
            }
            if !matches!(chooser, PlayerFilter::ChosenPlayer) {
                preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            }
            if chooses_tagged_object_pool(&resolved_filter)
                && matches!(resolved_filter.zone, None | Some(Zone::Battlefield))
            {
                resolved_filter.zone = None;
            }
            normalize_hand_or_graveyard_cross_zone_filter(&mut resolved_filter);
            let chooses_revealed_pool = ctx.last_revealed_tag.as_deref().is_some_and(|tag| {
                resolved_filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == tag
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                })
            });
            let cross_zone_choices = hand_or_graveyard_choice_zones(&resolved_filter);
            if let Some(zones) = &cross_zone_choices {
                strip_choice_zones_from_filter(&mut resolved_filter, zones);
            }
            let followup_player = choose_followup_player_filter(&resolved_filter, &chooser)
                .unwrap_or_else(|| chooser.clone());
            let chooses_tagged_pool = chooses_tagged_object_pool(&resolved_filter);
            let (mut effects, choices) = if let Some(zones) = cross_zone_choices {
                compile_choose_objects_across_zones_with_subject(
                    subject,
                    resolved_filter,
                    *count,
                    count_value.clone(),
                    tag.clone(),
                    zones,
                    None,
                    false,
                )
            } else if chooses_tagged_pool {
                compile_choose_objects_across_zones_with_subject(
                    subject,
                    resolved_filter,
                    *count,
                    count_value.clone(),
                    tag.clone(),
                    scoped_collection_zones(),
                    None,
                    false,
                )
            } else {
                let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
                compile_choose_objects_with_subject(
                    subject,
                    resolved_filter,
                    *count,
                    count_value.clone(),
                    tag.clone(),
                    choice_zone,
                )
            };
            if chooses_revealed_pool {
                effects = mark_choose_effects_reveal(effects);
            }
            ctx.last_it_choice_is_set = tag.as_str() == IT_TAG;
            ctx.last_object_tag = Some(tag.as_str().to_string());
            if is_sentence_helper_exiled_collection_tag(tag.as_str()) {
                ctx.last_exiled_collection_tag = Some(tag.as_str().to_string());
            }
            ctx.last_player_filter = Some(followup_player);
            (effects, choices)
        }
        EffectAst::ChooseObjectsBottomOfLibrary {
            filter,
            count,
            count_value,
            player,
            tag,
        } => {
            let subject = LoweredSubject::resolve_chooser(*player, ctx, true, true, false)?;
            let chooser = subject.clone_player_filter();
            let mut resolved_filter = subject.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?;
            resolved_filter.zone = Some(Zone::Library);
            let mut choose_effect = crate::effects::ChooseObjectsEffect::new(
                resolved_filter,
                *count,
                chooser.clone(),
                tag.clone(),
            )
            .with_count_value_opt(count_value.clone())
            .in_zone(Zone::Library)
            .bottom_only();
            choose_effect.description = "Choose bottom library card".to_string();
            let effects = subject.prepend_target_prelude_if_needed(Effect::new(choose_effect));
            ctx.last_it_choice_is_set = tag.as_str() == IT_TAG;
            ctx.last_object_tag = Some(tag.as_str().to_string());
            if is_sentence_helper_exiled_collection_tag(tag.as_str()) {
                ctx.last_exiled_collection_tag = Some(tag.as_str().to_string());
            }
            ctx.last_player_filter = Some(chooser);
            (effects, subject.into_choices())
        }
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            count_value,
            player,
            tag,
            zones,
            search_mode,
        } => {
            let subject = LoweredSubject::resolve_chooser(*player, ctx, true, true, false)?;
            let chooser = subject.as_chooser();
            let references_revealed_hand = filter.zone == Some(Zone::Hand)
                && filter.owner.is_none()
                && filter.controller.is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let mut resolved_filter =
                if references_revealed_hand && ctx.last_player_filter.is_some() {
                    subject.bind_revealed_hand_choice_filter(filter, ctx)?
                } else {
                    subject.resolve_object_refs_and_bind_player_refs_in_filter(filter, ctx)?
                };
            if references_revealed_hand && ctx.last_player_filter.is_some() {
                let has_revealed_collection_tag = ctx
                    .last_object_tag
                    .as_deref()
                    .is_some_and(is_revealed_collection_tag);
                if !has_revealed_collection_tag {
                    resolved_filter.tagged_constraints.retain(|constraint| {
                        !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                    });
                }
            }
            if !matches!(chooser, PlayerFilter::ChosenPlayer) {
                preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            }
            if slice_contains(zones.as_slice(), &Zone::Battlefield)
                && resolved_filter.controller.is_none()
                && resolved_filter.owner.is_none()
                && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
            let followup_player = choose_followup_player_filter(&resolved_filter, &chooser)
                .unwrap_or_else(|| chooser.clone());
            let chooses_tagged_pool = chooses_tagged_object_pool(&resolved_filter);
            let default_search =
                slice_contains(zones.as_slice(), &Zone::Library) && !chooses_tagged_pool;
            let (effects, choices) = compile_choose_objects_across_zones_with_subject(
                subject,
                resolved_filter,
                *count,
                count_value.clone(),
                tag.clone(),
                zones.clone(),
                *search_mode,
                default_search,
            );
            ctx.last_it_choice_is_set = tag.as_str() == IT_TAG;
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(followup_player);
            (effects, choices)
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

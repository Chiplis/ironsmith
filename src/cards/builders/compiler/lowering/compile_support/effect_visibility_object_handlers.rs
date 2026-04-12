use super::*;

pub(super) fn try_compile_visibility_and_card_selection_effect(
    effect: &EffectAst,
    ctx: &mut EffectLoweringContext,
) -> Result<Option<(Vec<Effect>, Vec<ChooseSpec>)>, CardTextError> {
    fn compile_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
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

        let (chooser, choices) = resolve_effect_player_filter(player, ctx, true, true, false)?;

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

    let compiled = match effect {
        EffectAst::LookAtHand { target } => {
            let (effects, choices) = compile_effect_for_target(target, ctx, |spec| {
                Effect::new(crate::effects::LookAtHandEffect::new(spec))
            })?;
            if let TargetAst::Player(filter, _) | TargetAst::PlayerOrPlaneswalker(filter, _) =
                target
            {
                ctx.last_player_filter = Some(PlayerFilter::Target(Box::new(filter.clone())));
            }
            (effects, choices)
        }
        EffectAst::TargetOnly { target } => {
            compile_tagged_effect_for_target(target, ctx, "targeted", |spec| {
                Effect::new(crate::effects::TargetOnlyEffect::new(spec))
            })?
        }
        EffectAst::RevealTop { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let tag = ctx.next_tag("revealed");
            ctx.last_object_tag = Some(tag.clone());
            let effect = Effect::reveal_top(player_filter, tag);
            (vec![effect], choices)
        }
        EffectAst::RevealTopChooseCardTypePutToHandRestBottom { player, count } => {
            use crate::effect::{Condition, EffectMode, Value};

            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;
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

            (vec![Effect::choose_one(modes)], choices)
        }
        EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
            player,
            count,
            filter,
        } => {
            use crate::effect::{Condition, Value};

            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;
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
            (vec![reveal, reveal_tagged, distribute], choices)
        }
        EffectAst::RevealTagged { tag } => {
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
            (
                vec![Effect::new(crate::effects::RevealTaggedEffect::new(
                    resolved_tag,
                ))],
                Vec::new(),
            )
        }
        EffectAst::LookAtTopCards { player, count, tag } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let resolved_tag = if tag.as_str() == IT_TAG {
                TagKey::from(ctx.next_tag("revealed").as_str())
            } else {
                tag.clone()
            };
            ctx.last_object_tag = Some(resolved_tag.as_str().to_string());
            let effect = Effect::look_at_top_cards(player_filter, count.clone(), resolved_tag);
            (vec![effect], choices)
        }
        EffectAst::RevealHand { player } => {
            let (player_filter, choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let spec = choices
                .first()
                .cloned()
                .unwrap_or_else(|| ChooseSpec::Player(player_filter.clone()));
            ctx.last_player_filter = Some(match *player {
                PlayerAst::Target => PlayerFilter::target_player(),
                PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
                _ => player_filter.clone(),
            });
            let effect = Effect::new(crate::effects::LookAtHandEffect::reveal(spec));
            (vec![effect], choices)
        }
        EffectAst::PutIntoHand { player, object } => {
            let ObjectRefAst::Tagged(tag) = object;
            let tag = resolve_it_tag_key(tag, &current_reference_env(ctx))?;
            let (_, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let effect = Effect::move_to_zone(ChooseSpec::Tagged(tag), Zone::Hand, false);
            (vec![effect], choices)
        }
        EffectAst::PutSomeIntoHandRestIntoGraveyard { player, count } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'them' without prior reference".to_string(),
                )
            })?;

            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
            choose_filter.zone = Some(Zone::Library);
            let chosen_tag = ctx.next_tag("chosen");
            let chosen_tag_key: TagKey = chosen_tag.as_str().into();
            let choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    choose_filter,
                    ChoiceCount::exactly(*count as usize),
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            let move_chosen = Effect::for_each_tagged(
                chosen_tag.clone(),
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
            let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key, membership_filter);
            let move_rest = Effect::for_each_tagged(
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
            );

            (vec![choose, move_chosen, move_rest], choices)
        }
        EffectAst::PutSomeIntoHandRestOnBottomOfLibrary { player, count } => {
            use crate::effect::Condition;
            use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};

            let looked_tag = ctx.last_object_tag.clone().ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to resolve 'them' without prior reference".to_string(),
                )
            })?;

            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut choose_filter = ObjectFilter::tagged(looked_tag.clone());
            choose_filter.zone = Some(Zone::Library);
            let chosen_tag = ctx.next_tag("chosen");
            let chosen_tag_key: TagKey = chosen_tag.as_str().into();
            let choose = Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    choose_filter,
                    ChoiceCount::exactly(*count as usize),
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(Zone::Library),
            );

            let move_chosen = Effect::for_each_tagged(
                chosen_tag.clone(),
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
            let in_chosen = Condition::TaggedObjectMatches(chosen_tag_key, membership_filter);
            let move_rest = Effect::for_each_tagged(
                looked_tag,
                vec![Effect::conditional(
                    in_chosen,
                    Vec::new(),
                    vec![Effect::move_to_zone(
                        ChooseSpec::Iterated,
                        Zone::Library,
                        false,
                    )],
                )],
            );

            (vec![choose, move_chosen, move_rest], choices)
        }
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
            player,
            filter,
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

            let (chooser, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut choose_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
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
                    ChoiceCount::up_to(1),
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
                let (if_not_effects, if_not_choices) = with_preserved_lowering_context(
                    ctx,
                    |_| {},
                    |ctx| compile_effects(if_not_chosen, ctx),
                )?;
                compiled.push(Effect::if_then(
                    move_to_hand_id,
                    EffectPredicate::DidNotHappen,
                    if_not_effects,
                ));
                choices.extend(if_not_choices);
            }

            ctx.last_object_tag = Some(chosen_tag);
            ctx.last_effect_id = Some(move_to_hand_id);
            (compiled, choices)
        }
        EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
            player,
            filter,
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

            let (chooser, mut choices) =
                resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut choose_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            choose_filter.zone = Some(Zone::Library);
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
                    ChoiceCount::up_to(1),
                    chooser,
                    chosen_tag_key.clone(),
                )
                .in_zone(Zone::Library),
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
                        Zone::Library,
                        false,
                    )],
                )],
            ));

            if !if_not_chosen.is_empty() {
                let (if_not_effects, if_not_choices) = with_preserved_lowering_context(
                    ctx,
                    |_| {},
                    |ctx| compile_effects(if_not_chosen, ctx),
                )?;
                compiled.push(Effect::if_then(
                    move_to_hand_id,
                    EffectPredicate::DidNotHappen,
                    if_not_effects,
                ));
                choices.extend(if_not_choices);
            }

            ctx.last_object_tag = Some(chosen_tag);
            ctx.last_effect_id = Some(move_to_hand_id);
            (compiled, choices)
        }
        EffectAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
            player,
            spell_filter,
            order,
        } => compile_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
            *player,
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
        )?,
        EffectAst::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary {
            player,
            order,
        } => compile_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
            *player,
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
        )?,
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player,
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

            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut primary_filter =
                resolve_it_tag(battlefield_filter, &current_reference_env(ctx))?;
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
            (
                vec![choose_primary, move_primary, fallback, move_rest],
                choices,
            )
        }
        EffectAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
            player,
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

            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;

            let mut primary_filter =
                resolve_it_tag(battlefield_filter, &current_reference_env(ctx))?;
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
            (
                vec![
                    choose_primary,
                    move_primary,
                    choose_secondary,
                    move_secondary,
                    move_rest,
                ],
                choices,
            )
        }
        EffectAst::PutRestOnBottomOfLibrary => {
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

            (vec![move_rest], Vec::new())
        }
        _ => return Ok(None),
    };

    Ok(Some(compiled))
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
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let references_revealed_hand = filter.zone == Some(Zone::Hand)
                && filter.owner.is_none()
                && filter.controller.is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if references_revealed_hand && ctx.last_player_filter.is_some() {
                resolved_filter.tagged_constraints.retain(|constraint| {
                    !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
                resolved_filter.owner = ctx.last_player_filter.clone();
            }
            if !matches!(chooser, PlayerFilter::ChosenPlayer) {
                preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            }
            let choice_zone = resolved_filter.ensure_zone(Zone::Battlefield);
            if choice_zone == Zone::Battlefield
                && resolved_filter.controller.is_none()
                && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
            let followup_player = choose_followup_player_filter(&resolved_filter, &chooser)
                .unwrap_or_else(|| chooser.clone());
            let choose_effect = crate::effects::ChooseObjectsEffect::new(
                resolved_filter,
                *count,
                chooser,
                tag.clone(),
            )
            .with_count_value_opt(count_value.clone())
            .in_zone(choice_zone);
            let effect = Effect::new(choose_effect);
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(followup_player);
            (effects, choices)
        }
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            player,
            tag,
            zones,
            search_mode,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let references_revealed_hand = filter.zone == Some(Zone::Hand)
                && filter.owner.is_none()
                && filter.controller.is_none()
                && filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            if references_revealed_hand && ctx.last_player_filter.is_some() {
                resolved_filter.tagged_constraints.retain(|constraint| {
                    !matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
                });
                resolved_filter.owner = ctx.last_player_filter.clone();
            }
            if !matches!(chooser, PlayerFilter::ChosenPlayer) {
                preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            }
            if slice_contains(zones.as_slice(), &Zone::Battlefield)
                && resolved_filter.controller.is_none()
                && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
            let followup_player = choose_followup_player_filter(&resolved_filter, &chooser)
                .unwrap_or_else(|| chooser.clone());
            let mut choose_effect = crate::effects::ChooseObjectsEffect::new(
                resolved_filter,
                *count,
                chooser,
                tag.clone(),
            )
            .in_zones(zones.clone());
            if let Some(search_mode) = search_mode {
                choose_effect = match search_mode {
                    crate::effect::SearchSelectionMode::Exact => choose_effect.as_search(),
                    crate::effect::SearchSelectionMode::Optional => {
                        choose_effect.as_optional_search()
                    }
                    crate::effect::SearchSelectionMode::AllMatching => {
                        choose_effect.as_all_matching_search()
                    }
                };
            } else if slice_contains(zones.as_slice(), &Zone::Library) {
                choose_effect = choose_effect.as_search();
            }
            let effect = Effect::new(choose_effect);
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(followup_player);
            (effects, choices)
        }
        EffectAst::ChoosePlayer {
            chooser,
            filter,
            tag,
            random,
            exclude_previous_choices,
        } => {
            let (chooser_filter, choices) =
                resolve_effect_player_filter(*chooser, ctx, true, true, false)?;
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
            let mut choose_effect = crate::effects::ChoosePlayerEffect::new(
                chooser_filter,
                resolved_filter,
                resolved_tag.clone(),
            )
            .excluding_tags(excluded_tags);
            if *random {
                choose_effect = choose_effect.at_random();
            }
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::new(choose_effect));
            ctx.last_player_filter = Some(PlayerFilter::TaggedPlayer(resolved_tag.clone()));
            ctx.recent_player_choice_tags
                .push(resolved_tag.as_str().to_string());
            (effects, choices)
        }
        EffectAst::TagMatchingObjects { filter, zones, tag } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let mut effect =
                crate::effects::TagMatchingObjectsEffect::new(resolved_filter, tag.clone());
            if !zones.is_empty() {
                effect = effect.in_zones(zones.clone());
            }
            ctx.last_object_tag = Some(tag.as_str().to_string());
            (vec![Effect::new(effect)], Vec::new())
        }
        EffectAst::ChooseSpellCastHistory {
            chooser,
            cast_by,
            filter,
            tag,
        } => {
            let (chooser_filter, choices) =
                resolve_effect_player_filter(*chooser, ctx, true, true, false)?;
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
            (effects, choices)
        }
        EffectAst::ChooseCardName {
            player,
            filter,
            tag,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_card_name(
                chooser.clone(),
                filter.clone(),
                tag.clone(),
            ));
            ctx.last_object_tag = Some(tag.as_str().to_string());
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::ChooseColor { player } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_color(chooser.clone()));
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::ChooseCardType { player, options } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_card_type(chooser.clone(), options.clone()));
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::FlipCoin { player } => {
            let player_filter =
                resolve_non_target_player_filter(*player, &current_reference_env(ctx))?;
            ctx.last_player_filter = Some(player_filter.clone());
            (vec![Effect::flip_coin(player_filter)], Vec::new())
        }
        EffectAst::ChooseNamedOption { player, options } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_named_option(
                chooser.clone(),
                options.clone(),
            ));
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::ChooseCreatureType {
            player,
            excluded_subtypes,
        } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, false)?;
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(Effect::choose_creature_type(
                chooser.clone(),
                excluded_subtypes.clone(),
            ));
            ctx.last_player_filter = Some(chooser);
            (effects, choices)
        }
        EffectAst::Sacrifice {
            filter,
            player,
            count,
            target,
        } => {
            if let Some(target) = target {
                let (effects, mut choices) =
                    compile_tagged_effect_for_target(target, ctx, "sacrificed", |spec| {
                        Effect::new(crate::effects::SacrificeTargetEffect::new(spec))
                    })?;
                let (chooser, player_choices) =
                    resolve_effect_player_filter(*player, ctx, true, true, true)?;
                ctx.last_player_filter = Some(chooser);
                for choice in player_choices {
                    push_choice(&mut choices, choice);
                }
                return Ok(Some((effects, choices)));
            }
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let target_prelude: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            let mut resolved_filter = match resolve_it_tag(filter, &current_reference_env(ctx)) {
                Ok(resolved) => resolved,
                Err(_)
                    if filter.tagged_constraints.len() == 1
                        && filter.tagged_constraints[0].tag.as_str() == IT_TAG =>
                {
                    ObjectFilter::source()
                }
                Err(err) => return Err(err),
            };
            preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            if resolved_filter.controller.is_none() && resolved_filter.tagged_constraints.is_empty()
            {
                resolved_filter.controller = Some(chooser.clone());
            }
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
                return Ok(Some((effects, choices)));
            }
            if *count == 1
                && let Some(tag) = object_filter_as_tagged_reference(&resolved_filter)
            {
                let mut effects = target_prelude;
                effects.push(Effect::new(crate::effects::SacrificeTargetEffect::new(
                    ChooseSpec::tagged(tag),
                )));
                return Ok(Some((effects, choices)));
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
            (effects, choices)
        }
        EffectAst::SacrificeAll { filter, player } => {
            let (chooser, choices) = resolve_effect_player_filter(*player, ctx, true, true, true)?;
            let mut resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            preserve_chooser_relative_player_filters(filter, &mut resolved_filter, &chooser);
            if resolved_filter.controller.is_none() {
                resolved_filter.controller = Some(chooser.clone());
            }
            let count = Value::Count(resolved_filter.clone());
            let effect = Effect::sacrifice_player(resolved_filter, count, chooser.clone());
            let mut effects: Vec<Effect> = choices
                .iter()
                .cloned()
                .map(|spec| Effect::new(crate::effects::TargetOnlyEffect::new(spec)))
                .collect();
            effects.push(effect);
            (effects, choices)
        }
        EffectAst::DiscardHand { player } => compile_player_effect(
            *player,
            ctx,
            true,
            Effect::discard_hand,
            Effect::discard_hand_player,
        )?,
        EffectAst::Discard {
            count,
            player,
            random,
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
            let (resolved_player, choices) = if matches!(*player, PlayerAst::Implicit) {
                if let Some(inferred_player) = resolved_filter
                    .as_ref()
                    .and_then(infer_player_filter_from_object_filter)
                    .or_else(|| ctx.last_player_filter.clone())
                {
                    (inferred_player, Vec::new())
                } else {
                    resolve_effect_player_filter(*player, ctx, true, true, true)?
                }
            } else {
                resolve_effect_player_filter(*player, ctx, true, true, true)?
            };
            let resolved_filter = resolved_filter.map(|mut resolved| {
                if resolved.owner.is_none() {
                    resolved.owner = Some(resolved_player.clone());
                }
                resolved
            });
            let tag = tag
                .clone()
                .unwrap_or_else(|| TagKey::from(ctx.next_tag("discarded").as_str()));
            ctx.last_object_tag = Some(tag.as_str().to_string());
            let effect = Effect::new(
                crate::effects::DiscardEffect::new_with_filter(
                    count.clone(),
                    resolved_player,
                    *random,
                    resolved_filter,
                )
                .with_tag(tag),
            );
            (vec![effect], choices)
        }
        EffectAst::Connive { target } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let effect =
                tag_object_target_effect(Effect::connive(spec.clone()), &spec, ctx, "connived");
            (vec![effect], choices)
        }
        EffectAst::ConniveIterated => (vec![Effect::connive(ChooseSpec::Iterated)], Vec::new()),
        EffectAst::Detain { target } => {
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
            (vec![effect], choices)
        }
        EffectAst::Goad { target } => {
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
            (vec![effect], choices)
        }
        EffectAst::ReturnToHand { target, random } => {
            let (spec, choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let from_graveyard = target_mentions_graveyard(target);
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
            (vec![effect], choices)
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            transformed,
            converted,
            controller,
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
                        if count.is_single()
                            && matches!(inner.base(), ChooseSpec::Object(filter) if filter.tagged_constraints.is_empty() && filter.zone == Some(Zone::Graveyard)) =>
                    {
                        let ChooseSpec::Object(filter) = inner.base() else {
                            unreachable!("guard ensures graveyard object base")
                        };
                        let tag = ctx.next_tag("chosen_return");
                        ctx.last_object_tag = Some(tag.clone());
                        effects.push(Effect::choose_objects(
                            filter.clone(),
                            count.clone(),
                            PlayerFilter::You,
                            tag.clone(),
                        ));
                        ChooseSpec::tagged(tag)
                    }
                    _ => spec.clone(),
                }
            } else {
                spec.clone()
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
                    Effect::return_from_graveyard_to_battlefield(resolved_spec.clone(), *tapped)
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
            (effects, choices)
        }
        EffectAst::MoveToLibraryNthFromTop { target, position } => {
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
            (vec![effect], choices)
        }
        EffectAst::MoveToZone {
            target,
            zone,
            to_top,
            battlefield_controller,
            battlefield_tapped,
            attached_to,
        } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
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
                return Ok(Some((vec![choose, effect], choices)));
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
                return Ok(Some((
                    vec![effect, Effect::attach_objects(moved_objects, attach_spec)],
                    choices,
                )));
            }

            (vec![effect], choices)
        }
        EffectAst::ShuffleObjectsIntoLibrary { target, player } => {
            let (spec, mut choices) =
                resolve_target_spec_with_choices(target, &current_reference_env(ctx))?;
            let (player_filter, player_choices) =
                resolve_effect_player_filter(*player, ctx, true, true, true)?;
            for choice in player_choices {
                push_choice(&mut choices, choice);
            }
            let mut effect = Effect::shuffle_objects_into_library(spec.clone(), player_filter);
            if choose_spec_targets_object(&spec) && ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("moved");
                ctx.last_object_tag = Some(tag.clone());
                effect = effect.tag(tag);
            }
            (vec![effect], choices)
        }
        EffectAst::ReturnAllToHand { filter } => {
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            (
                vec![Effect::return_all_to_hand(resolved_filter)],
                Vec::new(),
            )
        }
        EffectAst::ReturnAllToHandOfChosenColor { filter } => {
            use crate::effect::EffectMode;
            let resolved_filter = resolve_it_tag(filter, &current_reference_env(ctx))?;
            let (mut prelude, choices) = target_context_prelude_for_filter(&resolved_filter);
            let mut modes = Vec::new();
            let colors = [
                ("White", crate::color::Color::White),
                ("Blue", crate::color::Color::Blue),
                ("Black", crate::color::Color::Black),
                ("Red", crate::color::Color::Red),
                ("Green", crate::color::Color::Green),
            ];
            for (_name, color) in colors {
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
            (prelude, choices)
        }
        EffectAst::ReturnAllToBattlefield { filter, tapped } => {
            let mut effect = Effect::new(crate::effects::ReturnAllToBattlefieldEffect::new(
                resolve_it_tag(filter, &current_reference_env(ctx))?,
                *tapped,
            ));
            if ctx.auto_tag_object_targets {
                let tag = ctx.next_tag("returned");
                effect = effect.tag(tag.clone());
                ctx.last_object_tag = Some(tag);
            }
            (vec![effect], Vec::new())
        }
        EffectAst::ExchangeControl {
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
            (vec![effect], Vec::new())
        }
        EffectAst::ExchangeControlHeterogeneous {
            permanent1,
            permanent2,
            shared_type,
        } => compile_exchange_control_heterogeneous_effect(
            permanent1,
            permanent2,
            *shared_type,
            ctx,
        )?,
        EffectAst::ExchangeTextBoxes { target } => compile_exchange_text_boxes_effect(target, ctx)?,
        EffectAst::ExchangeZones {
            player,
            zone1,
            zone2,
        } => compile_exchange_zones_effect(*player, *zone1, *zone2, ctx)?,
        EffectAst::ExchangeValues {
            left,
            right,
            duration,
        } => compile_exchange_values_effect(left, right, duration.clone(), ctx)?,
        _ => return Ok(None),
    };

    Ok(Some(compiled))
}

use super::*;

pub(super) fn parse_look_hand_optional_exile_play_tax_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    fn object_filter_mut(target: &mut TargetAst) -> Option<&mut ObjectFilter> {
        match target {
            TargetAst::Object(filter, ..) => Some(filter),
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
                object_filter_mut(inner)
            }
            _ => None,
        }
    }

    let sentences = split_lexed_sentences(tokens);
    let [
        look_sentence,
        exile_sentence,
        permission_sentence,
        tax_sentence,
    ] = sentences.as_slice()
    else {
        return None;
    };

    let mut look_effects = crate::grammar::primitives::probe_shape(
        effect_sentences::parse_look_at_hand_sentence(look_sentence),
    )??;
    let [look_effect] = look_effects.as_mut_slice() else {
        return None;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtHand {
            target: hand_target,
        },
        ..
    }) = look_effect
    else {
        return None;
    };
    let TargetAst::Player(hand_owner, _) = hand_target else {
        return None;
    };
    let hand_owner = hand_owner.clone();

    let mut optional_exile = crate::grammar::primitives::probe_shape(
        effect_sentences::parse_effect_sentence_lexed(exile_sentence),
    )?;
    let [optional] = optional_exile.as_mut_slice() else {
        return None;
    };
    let exile_effects = match optional {
        EffectAst::May { effects } => effects,
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects,
        } => effects,
        _ => return None,
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target,
                    face_down: false,
                    ..
                },
            ..
        }),
    ] = exile_effects.as_mut_slice()
    else {
        return None;
    };
    let exile_filter = object_filter_mut(target)?;
    if !crate::slice_primitives::contains(&exile_filter.excluded_card_types, &CardType::Land) {
        return None;
    }
    // "from it" refers to the hand established by the first sentence. Make
    // that provenance executable instead of leaving an unscoped nonland-card
    // choice that could select from another zone or player.
    exile_filter.zone = Some(Zone::Hand);
    exile_filter.owner = Some(hand_owner);
    let exile_filter = exile_filter.clone();
    let exiled_tag = helper_tag_for_tokens(exile_sentence, "exiled");

    let permission_effects = crate::grammar::primitives::probe_shape(
        effect_sentences::parse_effect_sentence_lexed(permission_sentence),
    )?;
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    tag,
                    player: PlayerAst::ItsOwner,
                    allow_land: true,
                    without_paying_mana_cost: false,
                    allow_any_color_for_cast,
                    filter: None,
                    ..
                },
            ..
        }),
    ] = permission_effects.as_slice()
    else {
        return None;
    };
    if tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
        || *allow_any_color_for_cast != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }

    let tax = bundle_grammar::parse_spell_cast_this_way_tax_tokens(tax_sentence)?;
    let mut spell_filter = ObjectFilter::spell().without_type(CardType::Land);
    if let Some(caster) = tax.taxed_caster {
        spell_filter = spell_filter.cast_by(caster);
    }
    spell_filter.zone = None;

    Some(vec![
        look_effect.clone(),
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: exile_filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: exiled_tag.clone(),
                },
                EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
            ],
        },
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            exiled_tag.clone(),
            PlayerAst::ItsOwner,
            true,
            false,
            false,
            None,
        ),
        EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(exiled_tag, None),
            crate::model::CompilerGrantableCore::Ability(
                crate::model::CompilerStaticAbilityCore::new(
                    crate::static_abilities::CostIncreaseManaCost::new(
                        spell_filter,
                        tax.additional_cost,
                    ),
                ),
            ),
            crate::grant::GrantDuration::Forever,
        ),
    ])
}

pub(super) fn parse_discard_redraw_mana_value_ladder_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_discard_redraw_mana_value_ladder_tokens(tokens)?;
    let discarded_tag = helper_tag_for_tokens(tokens, "discarded_mana_ladder");
    let selected_tag = helper_tag_for_tokens(tokens, "selected_mana_ladder");

    let mut effects = vec![
        EffectAst::subject_verb_discard(
            PlayerAst::You,
            Value::CardsInHand(PlayerFilter::You)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::AllCardsInHand),
            false,
            false,
            None,
            Some(discarded_tag.clone()),
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: Value::PendingEffectMetric {
                    source: ironsmith_core::EffectMetricSource::Outcome,
                    metric: ironsmith_core::EffectMetric::Count,
                },
            },
        ),
    ];

    for mana_value in shape.mana_values {
        let mut filter = shape.filter.clone();
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
        filter.mana_value = Some(crate::filter::Comparison::Equal(mana_value as i32));
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: discarded_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        effects.push(EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
        });
    }

    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(selected_tag, None),
        Zone::Battlefield,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Some(effects)
}

pub(super) fn parse_each_player_shuffle_then_consult_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_each_player_shuffle_then_consult_tokens(tokens)?;
    let mut shuffled_filter = shape.shuffled_filter;
    shuffled_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let mut qualifying_filter = shape.qualifying_filter;
    qualifying_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let mut tagged_library_filter = ObjectFilter::default();
    tagged_library_filter.zone = Some(Zone::Library);

    let shuffled_tag = crate::tag::CompilerReferenceTag::EachPlayerShuffled.key();
    let qualifying_tag = crate::tag::CompilerReferenceTag::EachPlayerQualifyingShuffled.key();
    let revealed_tag = crate::tag::CompilerReferenceTag::EachPlayerConsultRevealed.key();
    let matched_tag = crate::tag::CompilerReferenceTag::EachPlayerConsultMatched.key();
    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::subject_verb_tag_matching_objects(
                shuffled_filter.clone(),
                vec![Zone::Battlefield],
                shuffled_tag.clone(),
            ),
            EffectAst::subject_verb_tag_matching_objects(
                qualifying_filter,
                vec![Zone::Battlefield],
                qualifying_tag.clone(),
            ),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(shuffled_tag, None),
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::That,
                    tag: qualifying_tag,
                    filter: tagged_library_filter,
                    mode: ironsmith_core::TaggedObjectMatchMode::CurrentOrLastKnown,
                },
                if_true: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::That,
                        LibraryConsultModeAst::Reveal,
                        shape.match_filter,
                        LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        matched_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(matched_tag.clone(), None),
                        shape.destination,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                        revealed_tag,
                        Some(matched_tag),
                        shape.remainder_order,
                        PlayerAst::That,
                    ),
                ],
                if_false: Vec::new(),
            },
        ],
    }])
}

use super::*;

pub(super) fn parse_reveal_until_land_put_all_graveyard_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let revealing_player = bundle_grammar::parse_reveal_until_land_player(tokens)?;
    let (player, target_effect) = match revealing_player {
        bundle_grammar::RevealUntilLandPlayer::TargetPlayer => (
            PlayerAst::Target,
            Some(EffectAst::subject_verb_target_only(TargetAst::Player(
                PlayerFilter::Any,
                span_from_tokens(tokens),
            ))),
        ),
        bundle_grammar::RevealUntilLandPlayer::TargetOpponent => (
            PlayerAst::TargetOpponent,
            Some(EffectAst::subject_verb_target_only(TargetAst::Player(
                PlayerFilter::Opponent,
                span_from_tokens(tokens),
            ))),
        ),
        bundle_grammar::RevealUntilLandPlayer::ThatPlayer => (PlayerAst::That, None),
        bundle_grammar::RevealUntilLandPlayer::DefendingPlayer => (PlayerAst::Defending, None),
    };

    let revealed_tag = TagKey::from("reveal_until_land_revealed");
    let matched_tag = TagKey::from("reveal_until_land_matched");
    let mut land_card = ObjectFilter::default();
    land_card.card_types.push(CardType::Land);
    land_card.zone = None;

    let mut effects = Vec::new();
    if let Some(target_effect) = target_effect {
        effects.push(target_effect);
    }
    effects.push(EffectAst::subject_verb_consult_top_of_library(
        player,
        LibraryConsultModeAst::Reveal,
        land_card,
        LibraryConsultStopRuleAst::FirstMatch,
        revealed_tag.clone(),
        matched_tag,
    ));
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(revealed_tag, None),
        Zone::Graveyard,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Some(effects)
}

pub(super) fn parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
    consult_sentence: &[OwnedLexToken],
    followup_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) =
        super::super::consult_family::parse_consult_traversal_sentence(consult_sentence)?
    else {
        return Ok(None);
    };
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: LibraryConsultModeAst::Reveal,
                ..
            },
        ..
    })) = parts.effects.last()
    else {
        return Ok(None);
    };

    let Some(followup) =
        bundle_grammar::parse_consult_battlefield_followup_shape(followup_sentence)
    else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        Zone::Battlefield,
        false,
        ReturnControllerAst::Preserve,
        followup.enters_tapped,
        None,
    ));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            parts.all_tag,
            Some(parts.match_tag),
            followup.order,
            parts.player,
        ),
    );

    Ok(Some(effects))
}

fn move_consult_tagged_group(tag: TagKey, zone: Zone, controller_you: bool) -> EffectAst {
    EffectAst::ForEachTagged {
        tag,
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
            zone,
            false,
            if controller_you {
                ReturnControllerAst::You
            } else {
                ReturnControllerAst::Preserve
            },
            false,
            None,
        )],
    }
}

fn append_consult_remainder(
    effects: &mut Vec<EffectAst>,
    remainder: bundle_grammar::ConsultRemainderDispositionShape,
    all_tag: TagKey,
    keep_tag: TagKey,
    player: PlayerAst,
) {
    match remainder {
        bundle_grammar::ConsultRemainderDispositionShape::Graveyard => {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag: all_tag,
                    keep_tagged: keep_tag,
                    zone: Zone::Graveyard,
                },
            ));
        }
        bundle_grammar::ConsultRemainderDispositionShape::LibraryBottom(order) => {
            effects.push(
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    all_tag,
                    Some(keep_tag),
                    order,
                    player,
                ),
            );
        }
        bundle_grammar::ConsultRemainderDispositionShape::ShuffleLibrary => {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
        }
    }
}

fn lower_consult_repeated_move(
    repeated: bundle_grammar::ConsultRepeatedMoveShape,
    all_tag: TagKey,
    tag_seed: &[OwnedLexToken],
) -> Option<(Vec<EffectAst>, TagKey)> {
    let mut first = parse_object_filter_lexed(&repeated.first_filter, false).ok()?;
    let mut second = parse_object_filter_lexed(&repeated.repeated_filter, false).ok()?;
    first.zone = None;
    second.zone = None;
    first = first.match_tagged(all_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    second = second.match_tagged(all_tag, TaggedOpbjectRelation::IsTaggedObject);
    let mut union = ObjectFilter::default();
    union.any_of = vec![first, second];
    let moved_tag = helper_tag_for_tokens(tag_seed, "consult_repeated_moved");
    Some((
        vec![
            EffectAst::subject_verb_tag_matching_objects(
                union,
                vec![Zone::Library],
                moved_tag.clone(),
            ),
            move_consult_tagged_group(moved_tag.clone(), repeated.zone, false),
        ],
        moved_tag,
    ))
}

pub(crate) fn parse_consult_disposition_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let leading_result =
        crate::runtime_backend::grammar::structure::split_leading_result_prefix_lexed(tokens);
    let bundle_tokens = leading_result
        .as_ref()
        .map(|prefix| prefix.trailing_tokens)
        .unwrap_or(tokens);
    let shape = bundle_grammar::parse_consult_disposition_sequence_shape(bundle_tokens)?;
    let parts =
        super::super::consult_family::parse_consult_traversal_sentence(&shape.consult_tokens)
            .ok()
            .flatten()?;
    let mut effects = parts.effects;
    let keep_tag = match shape.middle {
        bundle_grammar::ConsultMiddleShape::MatchedMove(matched) => match matched.selection {
            bundle_grammar::ConsultMoveSelectionShape::AllMatched => {
                effects.push(move_consult_tagged_group(
                    parts.match_tag.clone(),
                    matched.zone,
                    matched.controller_you,
                ));
                parts.match_tag.clone()
            }
            bundle_grammar::ConsultMoveSelectionShape::AnyNumberOfMatched => {
                let chosen_tag = helper_tag_for_tokens(&shape.consult_tokens, "consult_chosen");
                let mut filter = ObjectFilter::tagged(parts.match_tag.clone());
                filter.zone = Some(Zone::Library);
                effects.push(EffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: chosen_tag.clone(),
                });
                effects.push(move_consult_tagged_group(
                    chosen_tag.clone(),
                    matched.zone,
                    matched.controller_you,
                ));
                chosen_tag
            }
        },
        bundle_grammar::ConsultMiddleShape::RepeatedMove(repeated) => {
            let (mut repeated_effects, moved_tag) = lower_consult_repeated_move(
                repeated,
                parts.all_tag.clone(),
                &shape.consult_tokens,
            )?;
            effects.append(&mut repeated_effects);
            moved_tag
        }
        bundle_grammar::ConsultMiddleShape::Generic(clauses) => {
            for clause in clauses {
                let mut clause_effects =
                    effect_sentences::parse_effect_sentence_lexed(&clause).ok()?;
                effects.append(&mut clause_effects);
            }
            parts.match_tag.clone()
        }
    };
    append_consult_remainder(
        &mut effects,
        shape.remainder,
        parts.all_tag,
        keep_tag,
        parts.player,
    );
    match leading_result {
        Some(prefix) => Some(vec![match prefix.kind {
            crate::runtime_backend::grammar::structure::LeadingResultPrefixKind::If => {
                EffectAst::IfResult {
                    predicate: prefix.predicate,
                    effects,
                }
            }
            crate::runtime_backend::grammar::structure::LeadingResultPrefixKind::When => {
                EffectAst::WhenResult {
                    predicate: prefix.predicate,
                    effects,
                }
            }
        }]),
        None => Some(effects),
    }
}

pub(super) fn parse_reveal_repeated_disposition_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_reveal_repeated_disposition_sequence_shape(tokens)?;
    let mut effects = effect_sentences::parse_effect_chain(&shape.reveal_tokens).ok()?;
    let all_tag = helper_tag_for_tokens(&shape.reveal_tokens, "revealed_collection");
    effects.push(EffectAst::SnapshotLastObjectTag {
        into: all_tag.clone(),
    });
    let (mut repeated_effects, moved_tag) =
        lower_consult_repeated_move(shape.repeated, all_tag.clone(), &shape.reveal_tokens)?;
    effects.append(&mut repeated_effects);
    append_consult_remainder(
        &mut effects,
        shape.remainder,
        all_tag,
        moved_tag,
        PlayerAst::You,
    );
    Some(effects)
}

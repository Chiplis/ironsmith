use super::*;
use crate::cards::builders::ForEachEffectAst;

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

    let revealed_tag = crate::tag::CompilerReferenceTag::RevealUntilLandRevealed.bind();
    let matched_tag = crate::tag::CompilerReferenceTag::RevealUntilLandMatched.bind();
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

pub(crate) fn parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
    consult_sentence: &[OwnedLexToken],
    followup_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) =
        super::super::consult_family::parse_consult_traversal_sentence(consult_sentence)?
    else {
        return Ok(None);
    };
    fn contains_reveal_consult(effects: &[EffectAst]) -> bool {
        effects.iter().any(|effect| {
            if matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Library(LibraryActionAst::ConsultTopOfLibrary {
                        mode: LibraryConsultModeAst::Reveal,
                        ..
                    }),
                    ..
                })
            ) {
                return true;
            }
            let mut found = false;
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                found |= contains_reveal_consult(nested);
            });
            found
        })
    }
    if !contains_reveal_consult(&parts.effects) {
        return Ok(None);
    }

    let Some(followup) =
        bundle_grammar::parse_consult_battlefield_followup_shape(followup_sentence)
    else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(parts.match_tag.clone()), None),
        Zone::Battlefield,
        false,
        ReturnControllerAst::Preserve,
        followup.enters_tapped,
        None,
    ));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(parts.all_tag),
            Some(crate::tag::TagRef::of(parts.match_tag)),
            followup.order,
            parts.player,
        ),
    );

    Ok(Some(effects))
}

fn move_consult_tagged_group(tag: TagKey, zone: Zone, controller_you: bool) -> EffectAst {
    EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
        tag: crate::tag::TagRef::of(tag),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
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
    })
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
                SubjectVerbActionAst::Library(LibraryActionAst::PutTaggedRemainderInZone {
                    tag: crate::tag::TagRef::of(all_tag),
                    keep_tagged: crate::tag::TagRef::of(keep_tag),
                    zone: Zone::Graveyard,
                    surface: ironsmith_core::LibraryRemainderSurface::Rest,
                }),
            ));
        }
        bundle_grammar::ConsultRemainderDispositionShape::LibraryBottom(order) => {
            effects.push(
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    crate::tag::TagRef::of(all_tag),
                    Some(crate::tag::TagRef::of(keep_tag)),
                    order,
                    player,
                ),
            );
        }
        bundle_grammar::ConsultRemainderDispositionShape::ShuffleLibrary => {
            effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                player,
                SubjectVerbActionAst::Library(LibraryActionAst::ShuffleLibrary),
            ));
        }
    }
}

fn lower_consult_repeated_move(
    repeated: bundle_grammar::ConsultRepeatedMoveShape,
    all_tag: TagKey,
    tag_seed: &[OwnedLexToken],
) -> Option<(Vec<EffectAst>, TagKey)> {
    let mut first = crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
        &repeated.first_filter,
        false,
    ))?;
    let mut second = crate::grammar::primitives::probe_shape(parse_object_filter_lexed(
        &repeated.repeated_filter,
        false,
    ))?;
    first.zone = None;
    second.zone = None;
    // Each authored partition says "revealed this way", so the standalone
    // filter parser carries an unresolved `__it__` collection constraint.
    // This bundle already has the exact LookAtTopCards result tag.  Remove
    // only that generic collection marker before installing the exact tag;
    // otherwise lowering the preceding union capture makes `__it__` resolve
    // to the newly-created moved-union tag instead of the reveal collection.
    for filter in [&mut first, &mut second] {
        filter.tagged_constraints.retain(|constraint| {
            constraint.tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str()
                || constraint.relation != TaggedOpbjectRelation::IsTaggedObject
        });
    }
    first = first.match_tagged(all_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    second = second.match_tagged(all_tag, TaggedOpbjectRelation::IsTaggedObject);
    let mut union = ObjectFilter::default();
    union.any_of = vec![first.clone(), second.clone()];
    let moved_tag = helper_tag_for_tokens(tag_seed, "consult_repeated_moved");
    let first_tag = helper_tag_for_tokens(tag_seed, "consult_repeated_first");
    let second_tag = helper_tag_for_tokens(tag_seed, "consult_repeated_second");
    Some((
        vec![
            EffectAst::subject_verb_tag_matching_objects(
                union,
                vec![Zone::Library],
                crate::tag::TagRef::of(moved_tag.clone()),
            ),
            EffectAst::subject_verb_tag_matching_objects(
                first,
                vec![Zone::Library],
                crate::tag::TagRef::of(first_tag.clone()),
            ),
            move_consult_tagged_group(first_tag.key.clone(), repeated.zone, false),
            EffectAst::subject_verb_tag_matching_objects(
                second,
                vec![Zone::Library],
                crate::tag::TagRef::of(second_tag.clone()),
            ),
            move_consult_tagged_group(second_tag.key.clone(), repeated.zone, false),
        ],
        moved_tag.key.clone(),
    ))
}

pub(super) fn parse_look_manifest_partition_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [look, partition] = sentences else { return Ok(None); };
    let words = crate::lexer::parser_token_word_refs(partition);
    if words != ["manifest", "one", "of", "those", "cards", "then", "put", "the", "other",
        "on", "the", "top", "or", "bottom", "of", "your", "library"]
    { return Ok(None); }
    let mut effects = effect_sentences::parse_effect_sentence_lexed(look)?;
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject,
        action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
            count, tag: looked, reveal: false,
        }),
    })] = effects.as_slice() else { return Ok(None); };
    if count.unhinted() != &Value::Fixed(2)
        || !matches!(subject.player, PlayerAst::You | PlayerAst::Implicit)
    { return Ok(None); }
    let looked = looked.clone();
    let chosen = helper_tag_for_tokens(partition, "manifest_chosen");
    let remainder = helper_tag_for_tokens(partition, "manifest_remainder");
    effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
        filter: ObjectFilter::tagged(looked.key.clone()),
        count: ChoiceCount::exactly(1), player: PlayerAst::You,
        tag: crate::tag::TagRef::of(chosen.clone()), zone: Zone::Library,
    }));
    effects.push(EffectAst::subject_verb_manifest_onto_battlefield(
        PlayerAst::You, TargetAst::Tagged(crate::tag::TagRef::of(chosen.clone()), None),
        false, ReturnControllerAst::You, false,
    ));
    effects.push(EffectAst::subject_verb_tag_matching_objects(
        ObjectFilter::tagged(looked.key).match_tagged(chosen, TaggedOpbjectRelation::IsNotTaggedObject),
        vec![Zone::Library], crate::tag::TagRef::of(remainder.clone()),
    ));
    effects.push(EffectAst::subject_verb(SubjectVerbRoleAst::Actor, PlayerAst::You,
        SubjectVerbActionAst::Library(LibraryActionAst::MoveToLibraryTopOrBottomChoice {
            target: TargetAst::Tagged(crate::tag::TagRef::of(remainder), None), top_position: 0,
        }),
    ));
    Ok(Some(effects))
}

pub fn parse_consult_disposition_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let leading_result = crate::grammar::structure::split_leading_result_prefix_lexed(tokens);
    let bundle_tokens = leading_result
        .as_ref()
        .map(|prefix| prefix.trailing_tokens)
        .unwrap_or(tokens);
    let shape = bundle_grammar::parse_consult_disposition_sequence_shape(bundle_tokens)?;
    let parts = crate::grammar::primitives::probe_shape(
        super::super::consult_family::parse_consult_traversal_sentence(&shape.consult_tokens),
    )
    .flatten()?;
    let consult_effect_count = parts.effects.len();
    let optional_consult = shape
        .consult_tokens
        .iter()
        .position(|token| {
            token.is_word("reveal") || token.is_word("look") || token.is_word("exile")
        })
        .is_some_and(|verb| verb > 0 && shape.consult_tokens[verb - 1].is_word("may"));
    let mut effects = parts.effects;
    // "Sacrifice X Zombies, then reveal cards ... until ...": keep the
    // authored ", then" boundary between the leading action and the consult.
    if !optional_consult
        && effects.len() >= 2
        && shape
            .consult_tokens
            .windows(2)
            .any(|pair| pair[0].is_comma() && pair[1].is_word("then"))
    {
        effects = vec![EffectAst::CommaThen { effects }];
    }
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
                effects.push(EffectAst::ObjectChoices(
                    ObjectChoiceEffectAst::ChooseObjects {
                        filter,
                        count: ChoiceCount::any_number(),
                        count_value: None,
                        player: PlayerAst::You,
                        tag: crate::tag::TagRef::of(chosen_tag.clone()),
                    },
                ));
                effects.push(move_consult_tagged_group(
                    chosen_tag.clone().into(),
                    matched.zone,
                    matched.controller_you,
                ));
                chosen_tag.key.clone()
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
            fn bind_revealed_origin(effect: &mut EffectAst, revealed: &TagKey) {
                if let EffectAst::SubjectVerb(subject_verb) = effect
                    && let SubjectVerbActionAst::ZoneMoves(
                        crate::cards::builders::ZoneMoveActionAst::MoveToZone {
                            target: TargetAst::Object(filter, _, _),
                            all: true,
                            ..
                        },
                    ) = &mut subject_verb.action
                    && let Some(constraint) =
                        filter.tagged_constraints.iter_mut().find(|constraint| {
                            constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                                && [
                                    crate::tag::CompilerReferenceTag::RevealedThisWay.as_str(),
                                    crate::tag::CompilerReferenceTag::It.as_str(),
                                ]
                                .contains(&constraint.tag.as_str())
                        })
                {
                    // Revealing during a consult leaves these cards in the library.
                    // A creature-card noun must not add a battlefield restriction.
                    filter.zone = Some(Zone::Library);
                    constraint.tag = revealed.clone();
                }
                crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                    for effect in nested {
                        bind_revealed_origin(effect, revealed);
                    }
                });
            }
            if clauses.iter().any(|clause|
                bundle_grammar::followup_shapes::parse_damage_amount_replacement(clause).is_some())
            {
                // A dependent damage replacement needs both the preceding
                // damage instruction and the consult's matched-card receipt.
                // The sentence planner binds those identities; isolated
                // clause parsing has neither and cannot supply "that damage".
                let mut sentences = vec![SentenceInput::from_lexed(&shape.consult_tokens)];
                sentences.extend(clauses.iter().map(|clause| SentenceInput::from_lexed(clause)));
                effects = crate::grammar::primitives::probe_shape(
                    super::super::dispatch_entry::parse_effect_sentences_from_sentence_inputs(sentences),
                )?;
            } else {
            for clause in clauses {
                if let Some(matched) = bundle_grammar::parse_consult_matched_move_shape(&clause)
                    && matched.selection == bundle_grammar::ConsultMoveSelectionShape::AllMatched
                {
                    effects.push(move_consult_tagged_group(
                        parts.match_tag.clone(),
                        matched.zone,
                        matched.controller_you,
                    ));
                    continue;
                }

                let mut clause_effects = crate::grammar::primitives::probe_shape(
                    effect_sentences::parse_effect_sentence_lexed(&clause),
                )?;
                if crate::word_primitives::sequence_occurs(
                    &crate::lexer::token_word_refs(&clause),
                    &["revealed", "this", "way"],
                ) {
                    for effect in &mut clause_effects {
                        bind_revealed_origin(effect, &parts.all_tag);
                    }
                }
                effects.append(&mut clause_effects);
            }
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
    if optional_consult {
        let disposition = effects.split_off(consult_effect_count);
        let optional = if matches!(parts.player, PlayerAst::You | PlayerAst::Implicit) {
            EffectAst::Permissions(PermissionEffectAst::May { effects })
        } else {
            EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: parts.player,
                effects,
            })
        };
        effects = vec![
            optional,
            EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                predicate: crate::cards::builders::IfResultPredicate::Did,
                effects: disposition,
            }),
        ];
    }
    match leading_result {
        Some(prefix) => Some(vec![match prefix.kind {
            crate::grammar::structure::LeadingResultPrefixKind::If => {
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    predicate: prefix.predicate,
                    effects,
                })
            }
            crate::grammar::structure::LeadingResultPrefixKind::When => {
                EffectAst::Conditionals(ConditionalEffectAst::WhenResult {
                    predicate: prefix.predicate,
                    effects,
                })
            }
        }]),
        None => Some(effects),
    }
}

pub(super) fn parse_reveal_repeated_disposition_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    fn revealed_top_collection_tag(effects: &[EffectAst]) -> Option<TagKey> {
        fn collect(effect: &EffectAst, tags: &mut Vec<TagKey>) {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                        tag,
                        reveal: true,
                        ..
                    }),
                ..
            }) = effect
            {
                tags.push(tag.clone().into());
            }
            crate::model::visit::for_each_nested_effects(effect, true, |nested| {
                for effect in nested {
                    collect(effect, tags);
                }
            });
        }

        let mut tags = Vec::new();
        for effect in effects {
            collect(effect, &mut tags);
        }
        let [tag] = tags.as_slice() else {
            return None;
        };
        Some(tag.clone())
    }

    let shape = bundle_grammar::parse_reveal_repeated_disposition_sequence_shape(tokens)?;
    let mut effects = crate::grammar::primitives::probe_shape(
        effect_sentences::parse_effect_chain(&shape.reveal_tokens),
    )?;
    if effects.len() > 1 {
        effects = vec![EffectAst::CommaThen { effects }];
    }
    // Repeated disposition filters and the final remainder must consume the
    // exact collection populated by the preceding reveal. A synthetic
    // SnapshotLastObjectTag alias cannot cross the public sentence-boundary
    // lowering route reliably, leaving every later filter pointed at an empty
    // tag. The grammar has already proved one revealed top-card collection,
    // so transport that tag directly.
    let all_tag = revealed_top_collection_tag(&effects)?;
    effects.push(EffectAst::SnapshotLastObjectTag {
        into: crate::tag::TagRef::of(all_tag.clone()),
    });
    let (mut repeated_effects, moved_tag) =
        lower_consult_repeated_move(shape.repeated, all_tag.clone(), &shape.reveal_tokens)?;
    append_consult_remainder(
        &mut repeated_effects,
        shape.remainder,
        all_tag,
        moved_tag,
        PlayerAst::You,
    );
    Some(vec![
        EffectAst::SourceSentence {
            effects,
            leading_then: false,
            starting_with_controller: false,
        },
        EffectAst::SourceSentence {
            effects: vec![EffectAst::CommaThen {
                effects: repeated_effects,
            }],
            leading_then: false,
            starting_with_controller: false,
        },
    ])
}

#[cfg(test)]
mod provenance_tests {
    use super::*;
    #[test]
    fn consult_filtered_disposition_retains_library_origin() {
        let tokens = crate::lexer::lex_line("Reveal cards from the top of your library until you reveal X creature cards. Put all creature cards revealed this way into your graveyard, then put the rest on the bottom of your library in a random order.", 0).unwrap();
        let effects = parse_consult_disposition_bundle(&tokens).unwrap();
        let debug = format!("{effects:#?}");
        assert!(!debug.contains("Battlefield"), "{debug}");
    }
}

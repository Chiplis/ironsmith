use winnow::Parser as _;

use super::super::activation_and_restrictions::choice_object_clauses::{
    parse_choose_card_type_phrase_words, parse_target_player_choose_objects_clause,
    parse_you_choose_objects_clause, parse_you_choose_objects_clause_with_count_value,
};
use super::super::lexer::{OwnedLexToken, split_lexed_sentences};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::permission_helpers::{
    parse_cast_or_play_tagged_clause, parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::util::{
    helper_tag_for_tokens, parse_subject, parse_target_phrase, span_from_tokens, trim_commas, words,
};
use super::dispatch_entry::{
    SentenceInput, parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
};
use super::zone_handlers::{parse_exile_top_library_clause, split_exile_face_down_suffix};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, IfResultPredicate, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, Verb,
};
use crate::effect::Value;
use crate::filter::AlternativeCastKind;
use crate::object::CounterType;
use crate::runtime_backend::effect_ast_traversal::for_each_nested_effects_mut;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::front_end::grammar::effects as bundle_grammar;
use crate::target::{
    ObjectFilter, PlayerFilter, SourceReferenceSurface, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::zone::Zone;

pub(crate) fn parse_same_sentence_copy_and_may_cast_copy(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<
        crate::runtime_backend::activation_and_restrictions::trigger_subject_filters::MayCastTaggedSpec,
    >,
    CardTextError,
>{
    use super::super::grammar::primitives as grammar;

    let split = grammar::split_lexed_once_on_separator(tokens, || grammar::kw("and").void())
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((copy_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let copy_tokens = trim_commas(copy_slice).to_vec();
    if !effect_sentences::is_simple_copy_reference_sentence(&copy_tokens) {
        return Ok(None);
    }

    let tail_tokens = trim_commas(tail_slice).to_vec();
    let Some(spec) = effect_sentences::parse_may_cast_it_sentence(&tail_tokens) else {
        return Ok(None);
    };
    if !spec.as_copy {
        return Ok(None);
    }

    Ok(Some(spec))
}

fn parse_during_counter_on_source_turn_play_permission(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let token_words = words(tokens);
    if token_words.get(..5)? != ["during", "any", "turn", "you", "put"] {
        return None;
    }
    let counter_index = token_words.iter().position(|word| *word == "counter")?;
    let counter_type = crate::runtime_backend::grammar::filters::parse_counter_type_words(
        token_words.get(5..=counter_index)?,
    )?;
    let allow_land = match token_words.get(counter_index + 1..)? {
        ["on", "this", "saga", "you", "may", "play", "that", "card"] => true,
        ["on", "this", "saga", "you", "may", "cast", "that", "card"] => false,
        _ => return None,
    };
    Some(
        EffectAst::subject_verb_grant_play_tagged_during_turns_counter_put_on_source(
            TagKey::from(IT_TAG),
            PlayerAst::You,
            allow_land,
            counter_type,
        ),
    )
}

fn parse_inline_exile_top_then_put_from_among_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::runtime_backend::front_end::grammar::primitives as grammar;

    let Some((exile_tokens, put_tokens)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
    else {
        return Ok(None);
    };
    super::sequence_rules::generic_subject_verb_sequences::exiled_collections::parse_exile_top_then_put_from_among_tokens(
        &trim_commas(exile_tokens),
        &trim_commas(put_tokens),
    )
}

fn parse_inline_mill_then_put_from_among_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::runtime_backend::front_end::grammar::primitives as grammar;

    let Some((mill_tokens, put_tokens)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
    else {
        return Ok(None);
    };
    let sentences = [
        SentenceInput::from_lexed(&trim_commas(mill_tokens)),
        SentenceInput::from_lexed(&trim_commas(put_tokens)),
    ];
    let Some(effects) = super::sequence_rules::generic_subject_verb_sequences::pairs::parse_mill_then_may_put_from_among_into_hand(
        &sentences,
        0,
    )? else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }]))
}

/// Keeps a comma-linked private look, face-down exile, and persistent play
/// permission on one provenance tag.  This is the one-sentence counterpart
/// of the existing two-sentence look/exile/permission sequence rule.
fn parse_inline_look_exile_face_down_permission_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = crate::runtime_backend::grammar::primitives::split_lexed_slices_on_comma(tokens);
    if segments.len() < 3 {
        return Ok(None);
    }

    let look_tokens = trim_commas(segments[0]);
    let exile_tokens = trim_commas(segments[1]);
    let Ok(mut look_effects) = effect_sentences::parse_effect_sentence_lexed(&look_tokens) else {
        return Ok(None);
    };
    let Ok(mut exile_effects) = effect_sentences::parse_effect_sentence_lexed(&exile_tokens) else {
        return Ok(None);
    };
    let [look_effect] = look_effects.as_mut_slice() else {
        return Ok(None);
    };
    let [exile_effect] = exile_effects.as_mut_slice() else {
        return Ok(None);
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { tag: look_tag, .. },
        ..
    }) = look_effect
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Exile {
                target: TargetAst::Tagged(exile_tag, _),
                face_down: true,
                ..
            },
        ..
    }) = exile_effect
    else {
        return Ok(None);
    };
    if exile_tag.as_str() != IT_TAG {
        return Ok(None);
    }

    let mut permission_tokens = Vec::new();
    for segment in &segments[2..] {
        permission_tokens.extend_from_slice(&trim_commas(segment));
    }
    let Some(mut permission) = parse_cast_or_play_tagged_clause(&permission_tokens)? else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag: permission_tag,
                ..
            },
        ..
    }) = &mut permission
    else {
        return Ok(None);
    };

    let linked_tag = helper_tag_for_tokens(tokens, "looked_exiled");
    *look_tag = linked_tag.clone();
    *exile_tag = linked_tag.clone();
    *permission_tag = linked_tag;

    Ok(Some(vec![
        look_effects.remove(0),
        exile_effects.remove(0),
        permission,
    ]))
}

fn parse_exile_top_library_then_play_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
    third_sentence: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::runtime_backend::front_end::grammar::primitives as grammar;

    let permission_sentence = third_sentence.unwrap_or(second_sentence);
    let mut leading_effects = Vec::new();
    let mut exile_sentence = first_sentence.to_vec();
    if let Some((prefix, suffix)) =
        grammar::split_lexed_once_on_separator(first_sentence, || grammar::kw("then").void())
    {
        let prefix = trim_commas(prefix);
        let suffix = trim_commas(suffix);
        if let Ok(prefix_effects) = effect_sentences::parse_effect_sentence_lexed(&prefix)
            && matches!(
                prefix_effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ShuffleLibrary,
                    ..
                })]
            )
            && effect_sentences::find_verb(&suffix).is_some_and(|(verb, _)| verb == Verb::Exile)
        {
            leading_effects = prefix_effects;
            exile_sentence = suffix;
        }
    }

    let Some((verb, verb_idx)) = effect_sentences::find_verb(&exile_sentence) else {
        return Ok(None);
    };
    if verb != Verb::Exile {
        return Ok(None);
    }

    let exile_subject = if verb_idx == 0 {
        None
    } else {
        Some(parse_subject(&trim_commas(&exile_sentence[..verb_idx])))
    };
    let mut exile_tokens = trim_commas(&exile_sentence[verb_idx + 1..]);
    let mut inline_choice_tokens = third_sentence.map(|_| trim_commas(second_sentence));
    if inline_choice_tokens.is_none()
        && let Some((before_then, after_then)) =
            grammar::split_lexed_once_on_separator(&exile_tokens, || grammar::kw("then").void())
    {
        let after_then = trim_commas(after_then);
        if matches!(
            words(&after_then).as_slice(),
            ["choose", "one", "of", "them"]
                | ["you", "choose", "one", "of", "them"]
                | ["choose", "one", "of", "those", "cards"]
                | ["you", "choose", "one", "of", "those", "cards"]
        ) {
            exile_tokens = trim_commas(before_then);
            inline_choice_tokens = Some(after_then);
        }
    }
    let (exile_core, face_down) = split_exile_face_down_suffix(&exile_tokens);
    let exile_effect = if face_down {
        let default_player = exile_subject
            .and_then(|subject| match subject {
                crate::cards::builders::SubjectAst::Player(player) => Some(player),
                _ => None,
            })
            .unwrap_or(PlayerAst::Implicit);
        let Some(shape) = bundle_grammar::parse_exile_top_library_shape(exile_core, default_player)
        else {
            return Ok(None);
        };
        let bundle_grammar::ExileLibraryPlayerShape::EachOpponent = shape.player else {
            return Ok(None);
        };
        let Value::Fixed(count) = shape.count.unhinted() else {
            return Ok(None);
        };
        let Ok(count) = usize::try_from(*count) else {
            return Ok(None);
        };
        let chosen_tag = helper_tag_for_tokens(exile_core, "top_exiled_choice");
        let collection_tag = helper_tag_for_tokens(exile_core, "exiled");
        let mut filter = ObjectFilter::default().in_zone(Zone::Library);
        filter.owner = Some(PlayerFilter::IteratedPlayer);
        EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::ChooseObjectsTopOfLibrary {
                    filter,
                    count: ChoiceCount::exactly(count),
                    count_value: None,
                    // The effect controller receives the printed permission
                    // to inspect the face-down cards. Choosing the sole top
                    // card also records that viewer for the subsequent exile.
                    player: PlayerAst::You,
                    tag: chosen_tag.clone(),
                },
                EffectAst::TagAffected {
                    effect: Box::new(EffectAst::subject_verb_exile(
                        TargetAst::Tagged(chosen_tag, None),
                        true,
                    )),
                    // Accumulate all cards across the opponent loop so one
                    // trailing permission covers the complete collection.
                    tag: collection_tag,
                },
            ],
        }
    } else {
        let Some(effect) = parse_exile_top_library_clause(&exile_tokens, exile_subject) else {
            return Ok(None);
        };
        effect
    };
    let permission_effect = if let Some(effect) =
        parse_during_counter_on_source_turn_play_permission(permission_sentence)
    {
        effect
    } else if let Some(effect) =
        parse_until_end_of_turn_may_play_tagged_clause(permission_sentence)?
    {
        effect
    } else if let Some(effect) =
        parse_until_your_next_turn_may_play_tagged_clause(permission_sentence)?
    {
        effect
    } else if let Some(effect) = parse_cast_or_play_tagged_clause(permission_sentence)? {
        effect
    } else {
        let effects = effect_sentences::parse_effect_sentence_lexed(permission_sentence)?;
        let [effect] = effects.as_slice() else {
            return Ok(None);
        };
        effect.clone()
    };

    // Face-down collection permissions can spell out that the controller may
    // "look at and play" the cards. Some generic surfaces preserve the look as
    // an explicit action before the persistent grant. Here the top-library
    // chooser is already `You`, which both identifies the cards and records
    // their face-down viewer, so retain the durable grant and bind it to the
    // accumulated collection produced by the opponent loop.
    let permission_effect = match permission_effect {
        EffectAst::Sequence { effects } if face_down => {
            let [look, permission] = effects.as_slice() else {
                return Ok(None);
            };
            let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject,
                action: SubjectVerbActionAst::LookAtObjects { filter },
            }) = look
            else {
                return Ok(None);
            };
            if subject.player != PlayerAst::You
                || filter.zone != Some(Zone::Exile)
                || !filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                })
            {
                return Ok(None);
            }
            permission.clone()
        }
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects,
        }
        | EffectAst::May { effects }
            if face_down && effects.len() == 1 =>
        {
            effects.into_iter().next().expect("checked one effect")
        }
        permission => permission,
    };

    let Some(tag) = (match &exile_effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ExileTopOfLibrary { tags, .. } => tags.first().cloned(),
            _ => None,
        },
        EffectAst::ForEachOpponent { effects } => match effects.as_slice() {
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::ExileTopOfLibrary {
                            accumulated_tags, ..
                        },
                    ..
                }),
            ] => accumulated_tags.first().cloned(),
            [
                EffectAst::ChooseObjectsTopOfLibrary { .. },
                EffectAst::TagAffected { tag, .. },
            ] => Some(tag.clone()),
            _ => None,
        },
        _ => None,
    }) else {
        return Ok(None);
    };

    let (permission_tag, inline_choice_effect) = if let Some(choice_tokens) = inline_choice_tokens {
        let chosen_tag = helper_tag_for_tokens(&choice_tokens, "chosen_exiled");
        let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag,
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        (
            chosen_tag.clone(),
            Some(EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_tag,
                zone: Zone::Exile,
            }),
        )
    } else {
        (tag, None)
    };

    let permission_effect = match permission_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    tag: _,
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    while_on_top_of_library,
                    free_cast_from_current_zone,
                    until_source_exiles_another,
                    surface,
                },
            ..
        }) => EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                tag: permission_tag,
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                while_on_top_of_library,
                free_cast_from_current_zone,
                until_source_exiles_another,
                surface,
            },
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player,
                    allow_land,
                    until_next_end_step,
                    ..
                },
            ..
        }) => {
            if until_next_end_step {
                EffectAst::subject_verb_grant_play_tagged_until_your_next_end_step(
                    permission_tag,
                    player,
                    allow_land,
                    false,
                )
            } else {
                EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                    permission_tag,
                    player,
                    allow_land,
                    false,
                )
            }
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    filter,
                    during_turns_counter_put_on_source,
                    spell_cost_increase,
                    lands_enter_tapped,
                    ..
                },
            ..
        }) => {
            if spell_cost_increase.is_some() || lands_enter_tapped {
                EffectAst::subject_verb_grant_play_tagged_with_play_constraints(
                    permission_tag,
                    player,
                    spell_cost_increase,
                    lands_enter_tapped,
                )
            } else if let Some(counter_type) = during_turns_counter_put_on_source {
                EffectAst::subject_verb_grant_play_tagged_during_turns_counter_put_on_source(
                    permission_tag,
                    player,
                    allow_land,
                    counter_type,
                )
            } else {
                EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                    permission_tag,
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    filter,
                )
            }
        }
        _ => return Ok(None),
    };

    leading_effects.push(exile_effect);
    if let Some(choice_effect) = inline_choice_effect {
        leading_effects.push(choice_effect);
    }
    leading_effects.push(permission_effect);
    Ok(Some(leading_effects))
}

fn parse_optional_result_exile_choice_play_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [optional_sentence, conditional_sentence, permission_sentence] = sentences else {
        return Ok(None);
    };
    let Some(prefix) =
        crate::runtime_backend::grammar::structure::split_leading_result_prefix_lexed(
            conditional_sentence,
        )
    else {
        return Ok(None);
    };
    if prefix.kind != crate::runtime_backend::grammar::structure::LeadingResultPrefixKind::If
        || prefix.predicate != IfResultPredicate::Did
    {
        return Ok(None);
    }

    let optional_effects = effect_sentences::parse_effect_sentence_lexed(optional_sentence)?;
    if !matches!(
        optional_effects.as_slice(),
        [EffectAst::May { .. } | EffectAst::MayByPlayer { .. }]
    ) {
        return Ok(None);
    }
    let Some(linked_effects) = parse_exile_top_library_then_play_bundle(
        prefix.trailing_tokens,
        permission_sentence,
        None,
    )?
    else {
        return Ok(None);
    };

    let mut effects = optional_effects;
    effects.push(EffectAst::IfResult {
        predicate: prefix.predicate,
        effects: linked_effects,
    });
    Ok(Some(effects))
}

fn parse_hidden_exile_partition_permission_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    let [look_sentence, partition_sentence, permission_sentence] = sentences.as_slice() else {
        return Ok(None);
    };

    let mut partition_tokens = look_sentence.to_vec();
    partition_tokens.extend_from_slice(partition_sentence);
    let Some(mut effects) =
        super::dispatch_inner::parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(
            &partition_tokens,
        )
    else {
        return Ok(None);
    };

    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LookAtTopCards { .. },
            ..
        }),
        EffectAst::ChooseObjects {
            tag: selected_tag, ..
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Exile {
                    target: TargetAst::Tagged(exile_tag, None),
                    face_down: true,
                    ..
                },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    keep_tagged: Some(kept_tag),
                    ..
                },
            ..
        }),
    ] = effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if selected_tag != exile_tag || selected_tag != kept_tag {
        return Ok(None);
    }

    let linked_tag = helper_tag_for_tokens(partition_sentence, "exiled");
    *selected_tag = linked_tag.clone();
    *exile_tag = linked_tag.clone();
    *kept_tag = linked_tag.clone();

    let Some(mut permission) = parse_cast_or_play_tagged_clause(permission_sentence)? else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                tag,
                player: PlayerAst::You,
                allow_land: true,
                without_paying_mana_cost: false,
                filter: None,
                ..
            },
        ..
    }) = &mut permission
    else {
        return Ok(None);
    };
    *tag = linked_tag;
    effects.push(permission);

    Ok(Some(effects))
}

fn parse_may_cast_spell_for_alternative_cost_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let kind =
        bundle_grammar::parse_alternative_cost_bundle_shape(first_sentence, second_sentence)?.kind;

    let mut filter = ObjectFilter::nonland()
        .in_zone(Zone::Hand)
        .with_alternative_cast(kind);
    filter.owner = Some(PlayerFilter::You);
    Some(vec![
        EffectAst::may_cast_matching_spell_with_alternative_cost(
            PlayerAst::You,
            filter,
            Zone::Hand,
            kind,
        ),
    ])
}

fn parse_choose_type_then_phase_out_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first_sentence)?
    else {
        return Ok(None);
    };
    if !choose_count.is_single() {
        return Ok(None);
    }

    if bundle_grammar::parse_chosen_type_reference_shape(second_sentence).is_none() {
        return Ok(None);
    }

    let mut effects = effect_sentences::parse_effect_sentence_lexed(second_sentence)?;
    let [
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: crate::cards::builders::SubjectVerbActionAst::PhaseOutAll { filter, .. },
            ..
        }),
    ] = effects.as_mut_slice()
    else {
        return Ok(None);
    };

    if choose_filter.card_types.is_empty() {
        return Ok(None);
    }

    let mut phase_out_filter = (*filter).clone();
    // The first sentence chooses a type, not an object of that type. Keep the
    // option domain on a typed card-type choice and let the second sentence
    // refer to the value stored on the source.
    phase_out_filter.card_types.clear();
    phase_out_filter.all_card_types.clear();
    phase_out_filter.excluded_subtypes = choose_filter.excluded_subtypes.clone();
    phase_out_filter.chosen_creature_type = false;
    phase_out_filter.chosen_card_type = true;
    phase_out_filter.tagged_constraints.retain(|constraint| {
        !matches!(
            constraint.relation,
            TaggedOpbjectRelation::SharesCardType | TaggedOpbjectRelation::SharesPermanentType
        )
    });

    Ok(Some(vec![
        EffectAst::subject_verb_choose_card_type(chooser, choose_filter.card_types),
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ]))
}

fn looks_like_source_leaves_return_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    bundle_grammar::parse_source_leaves_return_shape(tokens).is_some()
}

fn promote_exile_effect_to_source_leaves(effect: EffectAst) -> Option<EffectAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match subject_verb.action {
            SubjectVerbActionAst::Exile {
                target,
                face_down,
                source_top_only: false,
                ..
            } => Some(
                EffectAst::subject_verb_exile_until_source_leaves(target, face_down)
                    .with_explicit_exile_return_surface(),
            ),
            SubjectVerbActionAst::ExileAll { filter, face_down } => Some(
                EffectAst::subject_verb_exile_until_source_leaves(
                    TargetAst::Object(filter, None, None),
                    face_down,
                )
                .with_explicit_exile_return_surface(),
            ),
            _ => None,
        },
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } if if_false.is_empty() && if_true.len() == 1 => {
            let inner = promote_exile_effect_to_source_leaves(if_true.into_iter().next().unwrap())?;
            Some(EffectAst::Conditional {
                predicate,
                if_true: vec![inner],
                if_false,
            })
        }
        _ => None,
    }
}

fn parse_exile_then_source_leaves_return_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !looks_like_source_leaves_return_followup_sentence(second_sentence) {
        return Ok(None);
    }

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first_sentence)?;
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some(rewritten_first_effect) = promote_exile_effect_to_source_leaves(first_effect.clone())
    else {
        return Ok(None);
    };

    Ok(Some(vec![rewritten_first_effect]))
}

fn parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let shape = bundle_grammar::parse_outside_game_choice_shape(first, second).map_err(|_| {
        CardTextError::ParseError(format!(
            "missing outside-game clause in reveal-or-choose bundle (clause: '{}')",
            words(&trim_commas(first)).join(" ")
        ))
    })?;
    let Some(shape) = shape else {
        return Ok(None);
    };
    let reveal_filter = parse_object_filter_lexed(shape.reveal_filter, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported outside-game reveal filter in reveal-or-choose bundle (clause: '{}')",
            words(&trim_commas(first)).join(" ")
        ))
    })?;
    let mut choose_filter =
        parse_object_filter_lexed(shape.choose_filter, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported exile choice filter in reveal-or-choose bundle (clause: '{}')",
                words(&trim_commas(first)).join(" ")
            ))
        })?;

    if reveal_filter.card_types != choose_filter.card_types
        || reveal_filter.subtypes != choose_filter.subtypes
        || reveal_filter.owner != choose_filter.owner
    {
        return Ok(None);
    }

    choose_filter.zone = None;

    let chosen_tag = TagKey::from("outside_game_or_exile_selected");
    let effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
            zones: vec![Zone::OutsideGame, Zone::Exile],
            search_mode: None,
        },
        EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(chosen_tag, span_from_tokens(second)),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];

    Ok(Some(vec![EffectAst::May { effects }]))
}

fn parse_reveal_from_outside_game_to_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_outside_game_wish_shape(tokens) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(&shape.filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported outside-game wish filter in clause '{}'",
            words(&trim_commas(tokens)).join(" ")
        ))
    })?;
    filter.owner = Some(PlayerFilter::You);
    filter.zone = Some(Zone::OutsideGame);

    let wish_tag = TagKey::from("searched_outside_game");
    let effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: wish_tag.clone(),
            zones: vec![Zone::OutsideGame],
            search_mode: Some(crate::effect::SearchSelectionMode::Optional),
        },
        EffectAst::subject_verb_reveal_tagged(wish_tag.clone()),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(wish_tag, span_from_tokens(tokens)),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];
    let mut outer = vec![EffectAst::May { effects }];
    if shape.exile_source {
        outer.push(EffectAst::subject_verb_exile(
            TargetAst::Source(None),
            false,
        ));
    }

    Ok(Some(outer))
}

fn parse_choose_objects_then_for_each_of_those_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut normalized_first = first.to_vec();
    for token in &mut normalized_first {
        token.lowercase_word();
    }

    let Some((player, filter, count)) = parse_you_choose_objects_clause(&normalized_first)?
        .or_else(|| {
            parse_target_player_choose_objects_clause(&normalized_first)
                .ok()
                .flatten()
        })
    else {
        return Ok(None);
    };
    let choose_tag = TagKey::from(IT_TAG);

    let Some(loop_shape) = bundle_grammar::parse_for_each_chosen_shape(second) else {
        return Ok(None);
    };
    let loop_body_effects = effect_sentences::parse_effect_sentence_lexed(loop_shape.body)?;
    if loop_body_effects.is_empty() {
        return Ok(None);
    }

    let mut combined = vec![EffectAst::ChooseObjects {
        filter,
        count,
        count_value: None,
        player,
        tag: choose_tag.clone(),
    }];
    combined.push(EffectAst::ForEachTagged {
        tag: choose_tag,
        effects: loop_body_effects,
    });
    if let Some(third) = third {
        let trailing_effects = effect_sentences::parse_effect_sentence_lexed(third)?;
        if trailing_effects.is_empty() {
            return Ok(None);
        }
        combined.extend(trailing_effects);
    }
    Ok(Some(combined))
}

fn chosen_target_collection_player_filter(target: &TargetAst) -> Option<(PlayerFilter, bool)> {
    match target {
        TargetAst::Player(filter, _) => Some((filter.clone(), false)),
        TargetAst::PlayerOrPlaneswalker(filter, _) => Some((filter.clone(), true)),
        TargetAst::ObjectOrPlayer(_, filter, _) => Some((filter.clone(), true)),
        TargetAst::AnyTarget(_) | TargetAst::AnyOtherTarget(_) => Some((PlayerFilter::Any, true)),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
            chosen_target_collection_player_filter(inner)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum MixedTargetIteration {
    Player,
    Object,
}

fn bind_prior_mixed_target_reference(target: &mut TargetAst, iteration: MixedTargetIteration) {
    match target {
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::TargetPlayerOrControllerOfTarget, span) => {
            *target = match iteration {
                MixedTargetIteration::Player => {
                    TargetAst::Player(PlayerFilter::IteratedPlayer, span.clone())
                }
                MixedTargetIteration::Object => {
                    TargetAst::Tagged(TagKey::from(IT_TAG), span.clone())
                }
            };
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
            bind_prior_mixed_target_reference(inner, iteration);
        }
        _ => {}
    }
}

/// A mixed player/object target collection executes as two disjoint typed
/// loops. Rebind an authored “that player or planeswalker” damage recipient
/// to the current member of the corresponding loop so every chosen target,
/// rather than the first target in the shared resolution context, receives
/// its own result.
fn bind_mixed_target_iteration_damage(effects: &mut [EffectAst], iteration: MixedTargetIteration) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamage { target, .. },
            ..
        }) = effect
        {
            bind_prior_mixed_target_reference(target, iteration);
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            bind_mixed_target_iteration_damage(nested, iteration);
        });
    }
}

/// Preserve a declared target collection that can contain players as well as
/// permanents, then execute the same authored procedure once for each chosen
/// target.
///
/// Player targets use an anaphoric target-player iterator. Object targets are
/// captured from the same declaration under one tag and use the existing
/// tagged-object iterator. The two runtime loops are disjoint but share one
/// parsed body, so mixed collections retain one target declaration without
/// introducing a bespoke runtime effect.
fn parse_choose_mixed_targets_then_for_each_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(choice_shape) =
        bundle_grammar::clause_dispatch_shapes::parse_choose_target_shape(first)
    else {
        return Ok(None);
    };
    let Ok(target) = parse_target_phrase(choice_shape.target_tokens) else {
        return Ok(None);
    };
    let Some((player_filter, includes_objects)) = chosen_target_collection_player_filter(&target)
    else {
        return Ok(None);
    };
    let Some(loop_shape) = bundle_grammar::parse_for_each_chosen_shape(second) else {
        return Ok(None);
    };
    let loop_body = effect_sentences::parse_effect_sentence_lexed(loop_shape.body)?;
    if loop_body.is_empty() {
        return Ok(None);
    }
    let mut player_body = loop_body.clone();
    bind_mixed_target_iteration_damage(&mut player_body, MixedTargetIteration::Player);
    let mut object_body = loop_body;
    bind_mixed_target_iteration_damage(&mut object_body, MixedTargetIteration::Object);

    let object_targets_tag = helper_tag_for_tokens(first, "chosen_target_objects");
    let declaration = EffectAst::subject_verb_explicit_target_only(target);
    let mut combined = vec![if includes_objects {
        EffectAst::TagAffected {
            effect: Box::new(declaration),
            tag: object_targets_tag.clone(),
        }
    } else {
        declaration
    }];
    combined.push(EffectAst::ForEachPlayersFiltered {
        // The mixed declaration above already made the target choice. This
        // is an anaphoric view over its player members, not a second target
        // declaration.
        filter: PlayerFilter::AliasedTarget(Box::new(player_filter)),
        effects: player_body,
    });
    if includes_objects {
        combined.push(EffectAst::ForEachTagged {
            tag: object_targets_tag,
            effects: object_body,
        });
    }
    if let Some(third) = third {
        let mut trailing = effect_sentences::parse_effect_sentence_lexed(third)?;
        if trailing.is_empty() {
            return Ok(None);
        }
        combined.append(&mut trailing);
    }
    Ok(Some(combined))
}

fn parse_discard_reveal_choose_discard_chosen_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [first, second, third] = sentences else {
        return Ok(None);
    };
    let Some(shape) = bundle_grammar::parse_discard_reveal_choice_shape(first, second, third)
    else {
        return Ok(None);
    };
    let revealed_player = match shape.revealed_player {
        bundle_grammar::RevealedHandPlayer::TargetPlayer => PlayerAst::Target,
        bundle_grammar::RevealedHandPlayer::TargetOpponent => PlayerAst::TargetOpponent,
    };

    let Some((chooser, choose_filter, choose_count, count_value)) =
        parse_you_choose_objects_clause_with_count_value(shape.choose_clause)?
    else {
        return Ok(None);
    };
    let discarded_tag = TagKey::from("discarded_this_way");
    let count_value =
        count_value.map(|_| Value::Count(ObjectFilter::tagged(discarded_tag.clone())));

    let mut discarded_filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
    discarded_filter.zone = Some(Zone::Hand);

    Ok(Some(vec![
        EffectAst::subject_verb_discard(
            PlayerAst::Implicit,
            Value::Fixed(0),
            false,
            true,
            None,
            Some(discarded_tag),
        ),
        EffectAst::subject_verb_reveal_hand(revealed_player),
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_discard(
            PlayerAst::That,
            Value::Count(discarded_filter.clone()),
            false,
            false,
            Some(discarded_filter),
            None,
        ),
    ]))
}

fn parse_selected_hand_double_choice_discard_bundle(
    sentences: &[&[OwnedLexToken]],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let [first, second, third] = sentences else {
        return Ok(None);
    };
    let Some(shape) = bundle_grammar::parse_selected_hand_double_choice_shape(first, second, third)
    else {
        return Ok(None);
    };
    let revealed_player = match shape.revealed_player {
        bundle_grammar::RevealedHandPlayer::TargetPlayer => PlayerAst::Target,
        bundle_grammar::RevealedHandPlayer::TargetOpponent => PlayerAst::TargetOpponent,
    };

    let parse_choice = |choice_tokens: &[OwnedLexToken]| {
        let mut clause = shape.choice_prefix.to_vec();
        clause.extend_from_slice(choice_tokens);
        parse_you_choose_objects_clause_with_count_value(&clause)
    };
    let Some((first_chooser, first_filter, first_count, first_count_value)) =
        parse_choice(shape.first_choice)?
    else {
        return Ok(None);
    };
    let Some((second_chooser, second_filter, second_count, second_count_value)) =
        parse_choice(shape.second_choice)?
    else {
        return Ok(None);
    };
    if first_chooser != PlayerAst::You
        || second_chooser != PlayerAst::You
        || !first_count.is_single()
        || !second_count.is_single()
        || first_count_value.is_some()
        || second_count_value.is_some()
        || first_filter.zone != Some(Zone::Hand)
        || second_filter.zone != Some(Zone::Hand)
    {
        return Ok(None);
    }

    // A non-implicit shared tag accumulates both independent selections at
    // runtime. The final discard therefore consumes their union without
    // replacing the first choice when the second choice resolves.
    let selected_tag = helper_tag_for_tokens(second, "selected_hand");
    let discarded_filter = ObjectFilter::tagged(selected_tag.clone()).in_zone(Zone::Hand);

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_hand(revealed_player),
        EffectAst::ChooseObjects {
            filter: first_filter,
            count: first_count,
            count_value: None,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
        },
        EffectAst::ChooseObjects {
            filter: second_filter,
            count: second_count,
            count_value: None,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
        },
        EffectAst::subject_verb_discard(
            PlayerAst::That,
            Value::Count(discarded_filter.clone()),
            false,
            false,
            Some(discarded_filter),
            None,
        ),
    ]))
}

fn chosen_counter_target(
    shape: bundle_grammar::ChosenCounterTarget<'_>,
    first: &[OwnedLexToken],
) -> Result<TargetAst, CardTextError> {
    match shape {
        bundle_grammar::ChosenCounterTarget::PermanentOrSuspendedCard => Ok(TargetAst::Object(
            ObjectFilter {
                any_of: vec![
                    ObjectFilter::permanent(),
                    ObjectFilter::default()
                        .in_zone(Zone::Exile)
                        .with_alternative_cast(AlternativeCastKind::Suspend)
                        .with_counter_type(CounterType::Time),
                ],
                ..ObjectFilter::default()
            },
            span_from_tokens(first),
            None,
        )),
        bundle_grammar::ChosenCounterTarget::Clause(tokens) => parse_target_phrase(tokens),
    }
}

fn parse_choose_counter_on_target_then_put_or_remove_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_chosen_counter_bundle_shape(first, second) else {
        return Ok(None);
    };
    if shape.action != bundle_grammar::ChosenCounterAction::PutOrRemove {
        return Ok(None);
    }
    let target = chosen_counter_target(shape.target, first)?;
    Ok(Some(vec![
        EffectAst::subject_verb_one_counter_kind_put_or_remove(target),
    ]))
}

fn parse_choose_counter_on_target_then_put_additional_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_chosen_counter_bundle_shape(first, second) else {
        return Ok(None);
    };
    if shape.action != bundle_grammar::ChosenCounterAction::PutAdditional {
        return Ok(None);
    }
    let target = chosen_counter_target(shape.target, first)?;
    Ok(Some(vec![
        EffectAst::subject_verb_put_counter_of_chosen_kind(target),
    ]))
}

fn parse_search_library_slots_to_hand_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_search_library_slots_shape(tokens) else {
        return Ok(None);
    };

    let mut slots = Vec::new();
    for item in shape.filters {
        let mut filter = parse_object_filter_lexed(&item, false)?;
        if let Some(name) = bundle_grammar::parse_explicit_card_name_surface_tokens(&item) {
            filter.name = Some(name);
        }
        filter.zone = if shape.multi_zone {
            None
        } else {
            Some(Zone::Library)
        };
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        slots.push(crate::cards::builders::SearchLibrarySlotAst {
            filter,
            optional: true,
        });
    }

    Ok(Some(vec![
        EffectAst::subject_verb_search_library_slots_to_hand(
            PlayerAst::You,
            slots,
            true,
            TagKey::from("search_library_slots_progress"),
        ),
    ]))
}

fn search_library_slots_to_hand_effect_from_items(
    filter_items: Vec<Vec<OwnedLexToken>>,
) -> Result<EffectAst, CardTextError> {
    let mut slots = Vec::new();
    for item in filter_items {
        let mut filter = parse_object_filter_lexed(&item, false)?;
        if let Some(name) = bundle_grammar::parse_explicit_card_name_surface_tokens(&item) {
            filter.name = Some(name);
        }
        filter.zone = Some(Zone::Library);
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        slots.push(crate::cards::builders::SearchLibrarySlotAst {
            filter,
            optional: true,
        });
    }

    Ok(EffectAst::subject_verb_search_library_slots_to_hand(
        PlayerAst::You,
        slots,
        true,
        TagKey::from("search_library_slots_progress"),
    ))
}

fn parse_kicked_search_library_slots_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = bundle_grammar::parse_kicked_search_library_slots_shape(tokens) else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: vec![search_library_slots_to_hand_effect_from_items(
            shape.replacement_filters,
        )?],
        if_false: vec![search_library_slots_to_hand_effect_from_items(vec![
            shape.default_filter,
        ])?],
        attach_to_previous_ability: false,
    }]))
}

fn parse_kicked_counter_mana_value_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let fact = bundle_grammar::parse_kicked_counter_replacement_tokens(tokens)?;
    let base_target = match fact.base.target {
        bundle_grammar::CounterSpellTargetReference::Explicit(target) => target,
        bundle_grammar::CounterSpellTargetReference::PriorSpell(_) => return None,
    };
    let kicked_target = match fact.kicked.target {
        bundle_grammar::CounterSpellTargetReference::Explicit(target) => target,
        bundle_grammar::CounterSpellTargetReference::PriorSpell(span) => TargetAst::Spell(span),
    };

    let counter_if_matches = |target: TargetAst, limit: Value, filter: ObjectFilter| {
        let Value::Fixed(limit) = limit else {
            return None;
        };
        if filter.mana_value.as_ref() != Some(&crate::target::Comparison::LessThanOrEqual(limit)) {
            return None;
        }
        Some(EffectAst::Conditional {
            predicate: PredicateAst::ItMatches(filter),
            if_true: vec![EffectAst::subject_verb_counter(target)],
            if_false: Vec::new(),
        })
    };
    let base = counter_if_matches(base_target, fact.base.limit, fact.base.filter)?;
    let kicked = counter_if_matches(kicked_target, fact.kicked.limit, fact.kicked.filter)?;

    Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: vec![kicked],
        if_false: vec![base],
        attach_to_previous_ability: false,
    }])
}

fn multi_zone_search_destination_effects(
    shape: &bundle_grammar::KickedMultiZoneSearchDestinationShape,
    destination: Zone,
) -> Vec<EffectAst> {
    let searched_tag = TagKey::from("searched_multi_zone");
    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: shape.filter.clone(),
            count: shape.count.clone(),
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
            zones: shape.zones.clone(),
            search_mode: Some(shape.search_mode),
        },
        EffectAst::subject_verb_reveal_tagged(searched_tag.clone()),
        EffectAst::ForEachTagged {
            tag: searched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag, None),
                destination,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]
}

fn parse_kicked_multi_zone_search_destination_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_kicked_multi_zone_search_destination_tokens(tokens)?;
    Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: multi_zone_search_destination_effects(&shape, shape.kicked_destination),
        if_false: multi_zone_search_destination_effects(&shape, shape.default_destination),
        attach_to_previous_ability: false,
    }])
}

fn parse_persistent_exile_play_tax_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_persistent_exile_play_tax_tokens(tokens)?;
    let tagged = TagKey::from(IT_TAG);
    let target = TargetAst::Object(shape.target_filter, Some(TextSpan::synthetic()), None);
    let mut spell_filter = ObjectFilter::spell()
        .without_type(CardType::Land)
        .cast_by(shape.taxed_caster);
    spell_filter.zone = None;

    Some(vec![
        EffectAst::subject_verb_exile(target, false),
        EffectAst::subject_verb_grant_by_spec(
            crate::grant::GrantSpec::new(
                crate::grant::Grantable::play_from(),
                ObjectFilter::tagged(tagged.clone()),
                Zone::Exile,
            ),
            shape.permission_player,
            crate::grant::GrantDuration::Forever,
        ),
        EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(tagged, None),
            crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
                crate::static_abilities::CostIncreaseManaCost::new(
                    spell_filter,
                    shape.additional_cost,
                ),
            )),
            crate::grant::GrantDuration::Forever,
        ),
    ])
}

fn parse_each_player_hand_exile_play_constraints_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_each_player_hand_exile_play_constraints_tokens(tokens)?;
    let exiled_tag = helper_tag_for_tokens(tokens, "each_player_hand_exiled");
    let mut hand_card = ObjectFilter::default();
    hand_card.zone = Some(Zone::Hand);
    hand_card.owner = Some(PlayerFilter::IteratedPlayer);

    Some(vec![
        EffectAst::ForEachPlayersFiltered {
            filter: shape.players,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: hand_card,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: exiled_tag.clone(),
                },
                EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
            ],
        },
        EffectAst::subject_verb_grant_play_tagged_with_play_constraints(
            exiled_tag,
            PlayerAst::ItsOwner,
            Some(shape.additional_cost),
            shape.lands_enter_tapped,
        ),
    ])
}

fn parse_look_hand_optional_exile_play_tax_bundle(
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

    let mut look_effects = effect_sentences::parse_effect_sentence_lexed(look_sentence).ok()?;
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

    let mut optional_exile = effect_sentences::parse_effect_sentence_lexed(exile_sentence).ok()?;
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
    if !exile_filter.excluded_card_types.contains(&CardType::Land) {
        return None;
    }
    // "from it" refers to the hand established by the first sentence. Make
    // that provenance executable instead of leaving an unscoped nonland-card
    // choice that could select from another zone or player.
    exile_filter.zone = Some(Zone::Hand);
    exile_filter.owner = Some(hand_owner);
    let exile_filter = exile_filter.clone();
    let exiled_tag = helper_tag_for_tokens(exile_sentence, "exiled");

    let permission_effects =
        effect_sentences::parse_effect_sentence_lexed(permission_sentence).ok()?;
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
    if tag.as_str() != IT_TAG
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
            crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
                crate::static_abilities::CostIncreaseManaCost::new(
                    spell_filter,
                    tax.additional_cost,
                ),
            )),
            crate::grant::GrantDuration::Forever,
        ),
    ])
}

fn parse_discard_redraw_mana_value_ladder_bundle(
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

fn parse_controller_sacrifice_consult_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_controller_sacrifice_consult_tokens(tokens)?;
    let revealed_tag = TagKey::from("controller_consult_revealed");
    let matched_tag = TagKey::from("controller_consult_matched");
    let target = TargetAst::Object(shape.target_filter, Some(TextSpan::synthetic()), None);
    let sacrifice = EffectAst::subject_verb_sacrifice(
        PlayerAst::ItsController,
        ObjectFilter::default(),
        1,
        Some(target),
    );
    let mut match_filter = shape.match_filter;

    if shape.conditional_on_sacrifice {
        let sacrificed_tag = helper_tag_for_tokens(tokens, "sacrificed");
        for constraint in &mut match_filter.tagged_constraints {
            if constraint.relation == TaggedOpbjectRelation::SharesCardType {
                constraint.tag = sacrificed_tag.clone();
            }
        }
        match_filter.tagged_constraints.dedup();
        let followups = vec![
            EffectAst::subject_verb_consult_top_of_library(
                PlayerAst::That,
                LibraryConsultModeAst::Reveal,
                match_filter,
                LibraryConsultStopRuleAst::FirstMatch,
                revealed_tag,
                matched_tag.clone(),
            ),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(matched_tag, None),
                shape.destination,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::ItsController,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
        ];
        return Some(vec![
            EffectAst::TagAffected {
                effect: Box::new(sacrifice),
                tag: sacrificed_tag,
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: followups,
            },
        ]);
    }

    Some(vec![
        sacrifice,
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::That,
            LibraryConsultModeAst::Reveal,
            match_filter,
            LibraryConsultStopRuleAst::FirstMatch,
            revealed_tag,
            matched_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(matched_tag, None),
            shape.destination,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::ItsController,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ])
}

fn parse_each_player_shuffle_then_consult_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_each_player_shuffle_then_consult_tokens(tokens)?;
    let mut shuffled_filter = shape.shuffled_filter;
    shuffled_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let mut qualifying_filter = shape.qualifying_filter;
    qualifying_filter.owner = Some(PlayerFilter::IteratedPlayer);
    let mut tagged_library_filter = ObjectFilter::default();
    tagged_library_filter.zone = Some(Zone::Library);

    let shuffled_tag = TagKey::from("each_player_shuffled");
    let qualifying_tag = TagKey::from("each_player_qualifying_shuffled");
    let revealed_tag = TagKey::from("each_player_consult_revealed");
    let matched_tag = TagKey::from("each_player_consult_matched");
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

fn parse_proliferate_choose_phase_out_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_proliferate_choose_phase_out_tokens(tokens)?;
    let chosen_tag = TagKey::from(IT_TAG);
    let phase_out_filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .match_tagged(chosen_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    Some(vec![
        EffectAst::subject_verb_proliferate(Value::Fixed(1)),
        EffectAst::ChooseObjects {
            filter: shape.filter,
            count: shape.count,
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag,
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ])
}

fn parse_tap_controlled_objects_then_empty_mana_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_tap_controlled_objects_then_empty_mana_tokens(tokens)?;
    Some(vec![
        EffectAst::subject_verb_target_only(TargetAst::Player(
            PlayerFilter::Any,
            span_from_tokens(tokens),
        )),
        EffectAst::subject_verb_tap_all(shape.filter),
        EffectAst::subject_verb_empty_mana_pool(PlayerAst::Target),
    ])
}

fn parse_energy_pay_any_destroy_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_energy_pay_any_destroy_tokens(tokens)?;
    Some(vec![
        EffectAst::subject_verb_energy_counters(PlayerAst::You, shape.energy),
        EffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![EffectAst::subject_verb_pay_any_energy(
                PlayerAst::You,
                shape.minimum_payment,
            )],
        },
        EffectAst::subject_verb_destroy_all(shape.filter),
    ])
}

#[path = "bundle_rules/consult_bundles.rs"]
mod consult_bundles;
pub(super) use consult_bundles::parse_consult_disposition_bundle;
use consult_bundles::{
    parse_consult_then_put_matches_battlefield_rest_bottom_bundle,
    parse_reveal_repeated_disposition_bundle, parse_reveal_until_land_put_all_graveyard_bundle,
};

#[path = "bundle_rules/per_graveyard.rs"]
mod per_graveyard;
use per_graveyard::parse_choose_each_graveyard_then_owner_shuffle_bundle;

#[path = "bundle_rules/delayed_collections.rs"]
mod delayed_collections;
use delayed_collections::parse_exile_collection_each_upkeep_return_bundle;

fn parse_bid_life_for_control_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_life_bid_shape(tokens)?;
    let target = parse_target_phrase(shape.target).ok()?;

    Some(vec![EffectAst::BidLife {
        target: target.clone(),
        starting_bid: 0,
        winner_effects: vec![EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            target,
            crate::effect::Until::Forever,
        )],
    }])
}

fn parse_regenerate_then_gain_control_if_regenerates_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_regenerate_control_shape(first, second)?;
    let regenerate_target = parse_target_phrase(shape.regenerate_target).ok()?;
    let control_target = parse_target_phrase(shape.control_target).ok()?;
    let follow_up = EffectAst::subject_verb_gain_control(
        PlayerAst::Implicit,
        control_target,
        crate::effect::Until::Forever,
    );

    Some(vec![
        EffectAst::subject_verb_regenerate_with_follow_up_effects(
            regenerate_target,
            vec![follow_up],
        ),
    ])
}

/// Parse a linked-duration sequence such as
/// "untap all creatures, then those creatures phase out until this enchantment
/// leaves the battlefield." The repeated filter is semantic identity; the
/// printed "those" does not depend on whether untapping changed each object.
fn parse_untap_then_phase_out_until_source_leaves_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    use crate::runtime_backend::front_end::grammar::primitives as grammar;

    let (untap_tokens, phase_tokens) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())?;
    let untap_effects =
        effect_sentences::parse_effect_sentence_lexed(&trim_commas(untap_tokens)).ok()?;
    let [untap_effect] = untap_effects.as_slice() else {
        return None;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::UntapAll { filter },
        ..
    }) = untap_effect
    else {
        return None;
    };

    let trimmed_phase_tokens = trim_commas(phase_tokens);
    let phase_words = words(&trimmed_phase_tokens);
    let phase_idx = phase_words
        .windows(4)
        .position(|window| window == ["phase", "out", "until", "this"])?;
    if phase_idx < 2
        || phase_words.first().copied() != Some("those")
        || phase_words.len() < phase_idx + 7
        || phase_words[phase_words.len() - 3..] != ["leaves", "the", "battlefield"]
    {
        return None;
    }
    let source_words = &phase_words[phase_idx + 3..phase_words.len() - 3];
    if source_words.len() < 2 || source_words.first().copied() != Some("this") {
        return None;
    }
    let source_surface = SourceReferenceSurface::ThisPermanentType(source_words.join(" "));

    Some(vec![
        untap_effect.clone(),
        EffectAst::subject_verb_phase_out_all_until_source_leaves(filter.clone(), source_surface),
    ])
}

pub(crate) fn parse_typed_effect_bundle_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    // A consult procedure nested under "for each of" belongs to the declared
    // mixed target collection. Claim that typed declaration/iteration shape
    // before the broad consult-disposition recognizer can start at the inner
    // reveal clause and discard the outer target declaration.
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_mixed_targets_then_for_each_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if let Some(effects) = parse_untap_then_phase_out_until_source_leaves_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_inline_look_exile_face_down_permission_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_inline_exile_top_then_put_from_among_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_inline_mill_then_put_from_among_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_hidden_exile_partition_permission_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_discard_redraw_mana_value_ladder_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_energy_pay_any_destroy_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_consult_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_repeated_disposition_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_reveal_from_outside_game_to_hand(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_each_player_hand_exile_play_constraints_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_look_hand_optional_exile_play_tax_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_persistent_exile_play_tax_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_controller_sacrifice_consult_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_each_player_shuffle_then_consult_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_proliferate_choose_phase_out_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_tap_controlled_objects_then_empty_mana_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_until_land_put_all_graveyard_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_bid_life_for_control_bundle(tokens) {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_collection_each_upkeep_return_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_each_graveyard_then_owner_shuffle_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(mut effects) =
            parse_untap_then_phase_out_until_source_leaves_bundle(sentences[0])
        && let Ok(mut follow_up) = effect_sentences::parse_effect_sentence_lexed(sentences[1])
    {
        effects.append(&mut follow_up);
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(effects) =
            parse_regenerate_then_gain_control_if_regenerates_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_then_source_leaves_return_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_optional_result_exile_choice_play_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && matches!(
            words(sentences[1]).as_slice(),
            ["choose", "one", "of", "them"]
                | ["you", "choose", "one", "of", "them"]
                | ["choose", "one", "of", "those", "cards"]
                | ["you", "choose", "one", "of", "those", "cards"]
        )
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1], Some(sentences[2]))
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(effects) =
            parse_may_cast_spell_for_alternative_cost_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_type_then_phase_out_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_selected_hand_double_choice_discard_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_discard_reveal_choose_discard_chosen_bundle(&sentences)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_mixed_targets_then_for_each_bundle(
            sentences[0],
            sentences[1],
            Some(sentences[2]),
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_mixed_targets_then_for_each_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_objects_then_for_each_of_those_bundle(
            sentences[0],
            sentences[1],
            Some(sentences[2]),
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_objects_then_for_each_of_those_bundle(sentences[0], sentences[1], None)
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_or_remove_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_choose_counter_on_target_then_put_additional_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
                sentences[0],
                sentences[1],
            )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && {
            let first_words = crate::runtime_backend::token_word_refs(sentences[0]);
            let choice_words = if first_words.first().copied() == Some("you") {
                &first_words[1..]
            } else {
                &first_words[..]
            };
            matches!(
                parse_choose_card_type_phrase_words(choice_words),
                Ok(Some((consumed, _))) if consumed == choice_words.len()
            )
        }
        && let Ok(Some(mut effects)) =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
                sentences[1],
                sentences[2],
            )
    {
        let first_words = crate::runtime_backend::token_word_refs(sentences[0]);
        let choice_words = if first_words.first().copied() == Some("you") {
            &first_words[1..]
        } else {
            &first_words[..]
        };
        let (_, options) = parse_choose_card_type_phrase_words(choice_words)
            .ok()
            .flatten()
            .expect("validated choose-card-type bundle prefix");
        let mut combined = vec![EffectAst::subject_verb_choose_card_type(
            PlayerAst::You,
            options,
        )];
        combined.append(&mut effects);
        return Some(combined);
    }
    if let Ok(Some(effects)) = parse_kicked_search_library_slots_replacement_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_kicked_counter_mana_value_replacement_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_kicked_multi_zone_search_destination_bundle(tokens) {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_search_library_slots_to_hand_bundle(tokens) {
        return Some(effects);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn each_opponent_hand_exile_keeps_permission_tax_and_land_entry_linked() {
        let tokens = lex_line(
            "Each opponent exiles a card from their hand and may play that card for as long as it remains exiled. Each spell cast this way costs {1} more to cast. Each land played this way enters tapped.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens)
            .expect("linked each-opponent hand exile bundle");
        let [
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Opponent,
                effects: per_player,
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                        tag: grant_tag,
                        player: PlayerAst::ItsOwner,
                        spell_cost_increase: Some(cost),
                        lands_enter_tapped: true,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected correlated exile and constrained play grant: {effects:#?}");
        };
        let [
            EffectAst::ChooseObjects {
                tag: chosen_tag,
                filter,
                ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Exile {
                        target: TargetAst::Tagged(exile_tag, None),
                        ..
                    },
                ..
            }),
        ] = per_player.as_slice()
        else {
            panic!("expected per-player choose/exile pair: {per_player:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert_eq!(filter.owner, Some(PlayerFilter::IteratedPlayer));
        assert_eq!(chosen_tag, exile_tag);
        assert_eq!(chosen_tag, grant_tag);
        assert_eq!(cost.to_oracle(), "{1}");
    }

    fn conditional_mana_value_limit(effect: &EffectAst) -> Option<i32> {
        let EffectAst::Conditional {
            predicate: PredicateAst::ItMatches(filter),
            if_true,
            if_false,
        } = effect
        else {
            return None;
        };
        if !if_false.is_empty()
            || !matches!(
                if_true.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Counter {
                        target: TargetAst::Spell(_),
                    },
                    ..
                })]
            )
        {
            return None;
        }
        match filter.mana_value.as_ref() {
            Some(crate::target::Comparison::LessThanOrEqual(limit)) => Some(*limit),
            _ => None,
        }
    }

    #[test]
    fn per_player_type_choice_phase_out_keeps_one_shared_card_type() {
        let tokens = lex_line(
            "That player chooses artifact, creature, land, or non-Aura enchantment. All nontoken permanents of that type phase out.",
            0,
        )
        .unwrap();
        let effects =
            parse_typed_effect_bundle_lexed(&tokens).expect("typed choice/phase-out bundle");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject:
                    crate::cards::builders::SubjectVerbSubjectAst {
                        player: PlayerAst::That,
                        ..
                    },
                action: SubjectVerbActionAst::ChooseCardType { options },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::PhaseOutAll { filter, .. },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected typed card-type choice and phase-out pair: {effects:#?}");
        };

        assert_eq!(
            options,
            &[
                CardType::Artifact,
                CardType::Creature,
                CardType::Land,
                CardType::Enchantment,
            ]
        );
        assert!(filter.nontoken);
        assert!(filter.chosen_card_type);
        assert!(!filter.chosen_creature_type);
        assert!(filter.card_types.is_empty());
        assert_eq!(filter.excluded_subtypes, [crate::types::Subtype::Aura]);
        assert!(filter.tagged_constraints.is_empty());
        assert!(filter.controller.is_none());
    }

    #[test]
    fn mixed_target_collection_reuses_one_complete_consult_procedure_per_target() {
        let tokens = lex_line(
            "Choose any number of target players or planeswalkers. For each of them, reveal cards from the top of your library until you reveal a nonland card, this spell deals damage equal to that card's mana value to that player or planeswalker, then you put the revealed cards on the bottom of your library in any order.",
            0,
        )
        .unwrap();
        let effects =
            parse_typed_effect_bundle_lexed(&tokens).expect("mixed target consult bundle");
        let [
            EffectAst::TagAffected {
                effect: declaration,
                tag: object_targets,
            },
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::AliasedTarget(player_filter),
                effects: player_body,
            },
            EffectAst::ForEachTagged {
                tag: tagged_targets,
                effects: object_body,
            },
        ] = effects.as_slice()
        else {
            panic!("expected one declaration and disjoint player/object loops: {effects:#?}");
        };
        assert_eq!(player_filter.as_ref(), &PlayerFilter::Any);
        assert_eq!(object_targets, tagged_targets);
        assert!(matches!(
            declaration.as_ref(),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::TargetOnly {
                        target: TargetAst::WithCount(inner, count),
                        explicit_declaration: true,
                    },
                ..
            }) if matches!(
                inner.as_ref(),
                TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, _)
            ) && count == &ChoiceCount::any_number()
        ));
        let [
            EffectAst::CommaThen {
                effects: player_procedure,
            },
        ] = player_body.as_slice()
        else {
            panic!("expected one authored consult procedure per player: {player_body:#?}");
        };
        assert!(matches!(
            player_procedure.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
                    ..
                }),
                EffectAst::CommaThen { effects: tail },
            ] if matches!(
                tail.as_slice(),
                [
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::DealDamage {
                            target: TargetAst::Player(PlayerFilter::IteratedPlayer, _),
                            ..
                        },
                        ..
                    }),
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                            keep_tagged: None,
                            ..
                        },
                        ..
                    }),
                ]
            )
        ));
        let [
            EffectAst::CommaThen {
                effects: object_procedure,
            },
        ] = object_body.as_slice()
        else {
            panic!("expected one authored consult procedure per planeswalker: {object_body:#?}");
        };
        assert!(matches!(
            object_procedure.as_slice(),
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::ConsultTopOfLibrary { .. },
                    ..
                }),
                EffectAst::CommaThen { effects: tail },
            ] if matches!(
                tail.as_slice(),
                [
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::DealDamage {
                            target: TargetAst::Tagged(tag, _),
                            ..
                        },
                        ..
                    }),
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                            keep_tagged: None,
                            ..
                        },
                        ..
                    }),
                ] if tag.as_str() == IT_TAG
            )
        ));
    }

    #[test]
    fn conditional_controller_sacrifice_consult_keeps_result_and_object_provenance() {
        let tokens = lex_line(
            "Target artifact's controller sacrifices it. If the player does, they reveal cards from the top of their library until they reveal an artifact card that shares a card type with the sacrificed artifact, put that card onto the battlefield, then shuffle.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("conditional consult bundle");
        let [
            EffectAst::TagAffected {
                effect: sacrifice,
                tag: sacrificed_tag,
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: followups,
            },
        ] = effects.as_slice()
        else {
            panic!("expected tagged sacrifice and result-gated consult, got {effects:#?}");
        };
        assert!(matches!(
            sacrifice.as_ref(),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Sacrifice {
                    target: Some(_),
                    ..
                },
                ..
            })
        ));
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ConsultTopOfLibrary {
                        filter: match_filter,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone { .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ShuffleLibrary,
                ..
            }),
        ] = followups.as_slice()
        else {
            panic!("expected consult, move, and shuffle followups, got {followups:#?}");
        };
        assert_eq!(
            match_filter
                .tagged_constraints
                .iter()
                .filter(|constraint| {
                    constraint.tag == *sacrificed_tag
                        && constraint.relation == TaggedOpbjectRelation::SharesCardType
                })
                .count(),
            1
        );
    }

    #[test]
    fn inline_look_face_down_exile_permission_uses_one_collection_tag() {
        let tokens = lex_line(
            "Look at the top card of that player's library, exile it face down, then you may play that card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).unwrap_or_else(|| {
            let segments = crate::runtime_backend::grammar::primitives::split_lexed_slices_on_comma(&tokens);
            let look = effect_sentences::parse_effect_sentence_lexed(&trim_commas(segments[0]));
            let exile = effect_sentences::parse_effect_sentence_lexed(&trim_commas(segments[1]));
            let mut permission_tokens = Vec::new();
            for segment in &segments[2..] {
                permission_tokens.extend_from_slice(&trim_commas(segment));
            }
            let permission = parse_cast_or_play_tagged_clause(&permission_tokens);
            panic!(
                "inline bundle did not match; segments={segments:#?}\nlook={look:#?}\nexile={exile:#?}\npermission={permission:#?}"
            )
        });
        let debug = format!("{effects:#?}");
        assert!(debug.contains("LookAtTopCards"), "{debug}");
        assert!(debug.contains("face_down: true"), "{debug}");
        assert!(
            debug.contains("GrantPlayTaggedForAsLongAsExiled"),
            "{debug}"
        );
        assert!(
            debug.contains("AnyColor") || debug.contains("AnyType"),
            "{debug}"
        );
    }

    #[test]
    fn kicked_counter_bundle_builds_self_replacement_ast_before_lowering() {
        let tokens = lex_line(
            "Counter target spell if its mana value is 3 or less. If this spell was kicked, counter that spell if its mana value is 7 or less instead.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).unwrap();
        let [
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                attach_to_previous_ability,
            },
        ] = effects.as_slice()
        else {
            panic!("expected a typed self-replacement AST, got {effects:#?}");
        };

        assert_eq!(predicate, &PredicateAst::ThisSpellWasKicked);
        assert!(!*attach_to_previous_ability);
        assert_eq!(
            if_false.first().and_then(conditional_mana_value_limit),
            Some(3)
        );
        assert_eq!(
            if_true.first().and_then(conditional_mana_value_limit),
            Some(7)
        );
    }

    #[test]
    fn inline_exile_top_then_put_binds_the_exact_exiled_collection() {
        let tokens = lex_line(
            "Exile the top seven cards of that player's library, then put a creature card from among them onto the battlefield under your control.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens)
            .expect("inline exile-top collection bundle should parse");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileTopOfLibrary { count, tags, .. },
                ..
            }),
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: choice_count,
                tag: chosen_tag,
                zone,
                ..
            },
            EffectAst::ForEachTagged { tag: loop_tag, .. },
        ] = effects.as_slice()
        else {
            panic!("expected exile/choose/put typed bundle, got {effects:#?}");
        };
        assert_eq!(count, &Value::Fixed(7));
        assert_eq!(choice_count, &ChoiceCount::exactly(1));
        assert_eq!(zone, &Zone::Exile);
        assert_eq!(chosen_tag, loop_tag);
        assert!(filter.card_types.contains(&CardType::Creature));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            tags.first() == Some(&constraint.tag)
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn exile_top_bundle_preserves_source_exile_permission_duration() {
        let tokens = lex_line(
            "Exile the top card of your library. You may play that card until you exile another card with this enchantment.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens)
            .expect("source-exile-bounded permission bundle should parse");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileTopOfLibrary { tags, .. },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                        tag,
                        until_source_exiles_another: true,
                        surface: Some(surface),
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected linked exile/grant bundle, got {effects:#?}");
        };
        assert_eq!(tags.first(), Some(tag));
        assert_eq!(
            surface
                .until_source_exiles_another
                .as_ref()
                .map(ironsmith_core::SourceReferenceSurface::display_text)
                .as_deref(),
            Some("this enchantment")
        );
    }

    #[test]
    fn inline_exile_top_choose_one_rebinds_the_play_permission() {
        let tokens = lex_line(
            "Exile the top two cards of your library, then choose one of them. You may play that card this turn.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens)
            .expect("inline choose-one exile permission should parse");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ExileTopOfLibrary { count, tags, .. },
                ..
            }),
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: choice_count,
                tag: chosen_tag,
                ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                        tag: permission_tag,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected exile/choose/permission typed bundle, got {effects:#?}");
        };
        assert_eq!(count, &Value::Fixed(2));
        assert_eq!(choice_count, &ChoiceCount::exactly(1));
        assert_eq!(chosen_tag, permission_tag);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            tags.first() == Some(&constraint.tag)
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
    }

    #[test]
    fn optional_result_exile_choice_rebinds_the_trailing_play_permission() {
        let tokens = lex_line(
            "You may discard a card. If you do, exile the top two cards of your library, then choose one of them. You may play that card this turn.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens)
            .expect("optional result-gated exile choice should parse as one linked bundle");
        let [
            EffectAst::May { .. } | EffectAst::MayByPlayer { .. },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: linked,
            },
        ] = effects.as_slice()
        else {
            panic!("expected optional action plus result-gated linked program, got {effects:#?}");
        };
        assert!(
            matches!(
                linked.as_slice(),
                [
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ExileTopOfLibrary { count, .. },
                        ..
                    }),
                    EffectAst::ChooseTaggedObjectsInZone { tag: chosen, .. },
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action:
                            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                                tag: permission,
                                surface: Some(surface),
                                ..
                            },
                        ..
                    }),
                ] if count == &Value::Fixed(2)
                    && chosen == permission
                    && !surface.leading_duration
            ),
            "the choice and trailing permission must share one exact exiled-card tag and surface: {linked:#?}"
        );
    }

    #[test]
    fn shuffle_prefix_stays_in_the_exile_top_free_play_bundle() {
        let tokens = lex_line(
            "Shuffle your library, then exile the top card. Until end of turn, you may play that card without paying its mana cost.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens)
            .expect("shuffle/exile/free-play bundle should parse");
        assert!(
            matches!(
                effects.as_slice(),
                [
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ShuffleLibrary,
                        ..
                    }),
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::ExileTopOfLibrary { .. },
                        ..
                    }),
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                            without_paying_mana_cost: true,
                            ..
                        },
                        ..
                    }),
                ]
            ),
            "expected typed shuffle/exile/free-play sequence, got {effects:#?}"
        );
    }

    #[test]
    fn selected_hand_double_choice_builds_distinct_filters_with_one_accumulating_tag() {
        let tokens = lex_line(
            "Target opponent reveals their hand. You choose from it a nonland card with mana value 3 or less and a card with mana value 4 or greater. That player discards those cards.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("selected-hand bundle");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RevealHand,
                ..
            }),
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                tag: first_tag,
                ..
            },
            EffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                tag: second_tag,
                ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Discard {
                        count: Value::Count(discard_filter),
                        filter: Some(card_filter),
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected reveal, two choices, and one tagged discard, got {effects:#?}");
        };

        assert_eq!(first_count, &ChoiceCount::exactly(1));
        assert_eq!(second_count, &ChoiceCount::exactly(1));
        assert_eq!(first_tag, second_tag);
        assert!(first_filter.excluded_card_types.contains(&CardType::Land));
        assert!(matches!(
            first_filter.mana_value.as_ref(),
            Some(crate::target::Comparison::LessThanOrEqual(3))
        ));
        assert!(second_filter.excluded_card_types.is_empty());
        assert!(matches!(
            second_filter.mana_value.as_ref(),
            Some(crate::target::Comparison::GreaterThanOrEqual(4))
        ));
        for filter in [discard_filter, card_filter] {
            assert!(filter.tagged_constraints.iter().any(|constraint| {
                constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                    && &constraint.tag == first_tag
            }));
        }
    }

    #[test]
    fn each_opponent_top_card_permission_preserves_the_accumulated_collection() {
        let tokens = lex_line(
            "Exile the top card of each opponent's library face down. You may look at and play those cards for as long as they remain exiled.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("exile/permission bundle");
        let [
            EffectAst::ForEachOpponent {
                effects: exile_each,
            },
            permission,
        ] = effects.as_slice()
        else {
            panic!("expected each-opponent exile plus shared permission, got {effects:#?}");
        };
        let [
            EffectAst::ChooseObjectsTopOfLibrary {
                player: PlayerAst::You,
                ..
            },
            EffectAst::TagAffected {
                tag: collection_tag,
                ..
            },
        ] = exile_each.as_slice()
        else {
            panic!("expected typed top-library exile, got {exile_each:#?}");
        };
        assert!(matches!(
            permission,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled { tag, .. },
                ..
            }) if tag == collection_tag
        ));
    }

    #[test]
    fn hidden_exile_partition_uses_one_tag_for_choice_remainder_and_permission() {
        let tokens = lex_line(
            "Look at the top two cards of target opponent's library. Exile one of them face down and put the other on the bottom of that library. You may play the exiled card for as long as it remains exiled, and you may spend mana as though it were mana of any color to cast that spell.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("hidden-exile bundle");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtTopCards { .. },
                ..
            }),
            EffectAst::ChooseObjects {
                tag: selected_tag, ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Exile {
                        target: TargetAst::Tagged(exile_tag, None),
                        face_down: true,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        keep_tagged: Some(kept_tag),
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                        tag: permission_tag,
                        allow_any_color_for_cast:
                            ironsmith_core::value_model::ManaSpendMode::AnyColor,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected linked hidden-exile partition, got {effects:#?}");
        };

        assert_eq!(selected_tag, exile_tag);
        assert_eq!(selected_tag, kept_tag);
        assert_eq!(selected_tag, permission_tag);
    }

    #[test]
    fn looked_hand_exile_permission_tax_stays_in_one_linked_program() {
        let tokens = lex_line(
            "Look at target opponent's hand. You may exile a nonland card from it. For as long as that card remains exiled, its owner may play it. A spell cast this way costs {2} more to cast.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("linked hand-exile bundle");
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::LookAtHand { .. },
                ..
            }),
            EffectAst::MayByPlayer {
                player: PlayerAst::You,
                effects: optional_exile,
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                        tag: permission_tag,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantToTarget {
                        target: TargetAst::Tagged(tax_tag, None),
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected one linked hand-exile program, got {effects:#?}");
        };
        let [
            EffectAst::ChooseObjects {
                filter,
                player: PlayerAst::You,
                tag: choice_tag,
                ..
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Exile {
                        target: TargetAst::Tagged(exile_tag, None),
                        face_down: false,
                        ..
                    },
                ..
            }),
        ] = optional_exile.as_slice()
        else {
            panic!("expected an optional typed choose/exile pair, got {optional_exile:#?}");
        };
        assert_eq!(filter.zone, Some(Zone::Hand));
        assert!(matches!(&filter.owner, Some(PlayerFilter::Target(_))));
        assert_eq!(choice_tag, exile_tag);
        assert_eq!(choice_tag, permission_tag);
        assert_eq!(choice_tag, tax_tag);
    }

    #[test]
    fn inline_mill_then_optional_filtered_return_keeps_one_milled_collection() {
        let tokens = lex_line(
            "Mill three cards, then you may put an artifact or land card from among the milled cards into your hand.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("inline mill bundle");
        let [
            EffectAst::Coordinated {
                effects,
                leading_duration: false,
                result_conjunction: false,
            },
        ] = effects.as_slice()
        else {
            panic!("expected an authored inline sequence boundary, got {effects:#?}");
        };
        let [
            EffectAst::TagAffected {
                tag: milled_tag,
                effect: mill,
            },
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count,
                tag: chosen_tag,
                ..
            },
            EffectAst::ForEachTagged {
                tag: moved_tag,
                effects: move_effects,
            },
        ] = effects.as_slice()
        else {
            panic!("expected linked mill, choice, and move program, got {effects:#?}");
        };

        assert!(matches!(
            mill.as_ref(),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Mill {
                    count: Value::Fixed(3),
                },
                ..
            })
        ));
        assert_eq!(count, &ChoiceCount::up_to(1));
        assert_eq!(chosen_tag, moved_tag);
        assert_eq!(
            filter.prior_effect_action_surface(),
            Some(ironsmith_core::PriorEffectAction::Milled)
        );
        for card_type in [CardType::Artifact, CardType::Land] {
            assert!(
                filter.card_types.contains(&card_type)
                    || filter
                        .any_of
                        .iter()
                        .any(|branch| branch.card_types.contains(&card_type)),
                "{filter:#?}"
            );
        }
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *milled_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(matches!(
            move_effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Hand,
                    ..
                },
                ..
            })]
        ));
    }

    #[test]
    fn inline_mill_then_return_from_among_them_keeps_one_milled_collection() {
        let tokens = lex_line(
            "Mill four cards, then you may return a permanent card from among them to your hand.",
            0,
        )
        .unwrap();
        let effects = parse_typed_effect_bundle_lexed(&tokens).expect("inline mill bundle");
        let debug = format!("{effects:#?}");

        assert!(debug.contains("TagAffected"), "{debug}");
        assert!(debug.contains("ChooseTaggedObjectsInZone"), "{debug}");
        assert!(debug.contains("zone: Graveyard"), "{debug}");
        assert!(debug.contains("IsTaggedObject"), "{debug}");
        assert!(debug.contains("zone: Hand"), "{debug}");
    }
}

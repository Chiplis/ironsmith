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
use super::zone_handlers::{
    parse_exile, parse_exile_top_library_clause, split_exile_face_down_suffix,
};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IfResultPredicate, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, Verb,
    ZoneReplacementDurationAst,
};
use crate::effect::Value;
use crate::effect_sentences;
use crate::filter::AlternativeCastKind;
use crate::grammar::effects as bundle_grammar;
use crate::model::visit::for_each_nested_effects_mut;
use crate::object::CounterType;
use crate::target::{
    ObjectFilter, PlayerFilter, SourceReferenceSurface, TaggedObjectConstraint,
    TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::zone::Zone;

pub fn parse_same_sentence_copy_and_may_cast_copy(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<crate::activation_and_restrictions::trigger_subject_filters::MayCastTaggedSpec>,
    CardTextError,
> {
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
    if !crate::word_primitives::parse_sequence_prefix(
        &token_words,
        &["during", "any", "turn", "you", "put"],
    ) {
        return None;
    }
    let counter_index =
        crate::slice_primitives::select_position(&token_words, |word| *word == "counter")?;
    let counter_type =
        crate::grammar::filters::parse_counter_type_words(token_words.get(5..=counter_index)?)?;
    let tail = token_words.get(counter_index + 1..)?;
    let allow_land = if crate::word_primitives::parse_sequence_complete(
        tail,
        &["on", "this", "saga", "you", "may", "play", "that", "card"],
    ) {
        true
    } else if crate::word_primitives::parse_sequence_complete(
        tail,
        &["on", "this", "saga", "you", "may", "cast", "that", "card"],
    ) {
        false
    } else {
        return None;
    };
    Some(
        EffectAst::subject_verb_grant_play_tagged_during_turns_counter_put_on_source(
            crate::tag::CompilerReferenceTag::It.key(),
            PlayerAst::You,
            allow_land,
            counter_type,
        ),
    )
}

fn parse_inline_exile_top_then_put_from_among_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::grammar::primitives as grammar;

    if split_lexed_sentences(tokens).len() != 1 {
        return Ok(None);
    }
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
    use crate::grammar::primitives as grammar;

    if split_lexed_sentences(tokens).len() != 1 {
        return Ok(None);
    }
    let Some((mill_tokens, put_tokens)) =
        grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void())
    else {
        return Ok(None);
    };
    let sentences = [
        SentenceInput::from_lexed(&trim_commas(mill_tokens)),
        SentenceInput::from_lexed(&trim_commas(put_tokens)),
    ];
    let Some(effects) = super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_mill_then_may_put_from_among_into_hand(
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
    let segments = crate::grammar::primitives::split_lexed_slices_on_comma(tokens);
    if segments.len() < 3 {
        return Ok(None);
    }

    let look_tokens = trim_commas(segments[0]);
    let exile_tokens = trim_commas(segments[1]);
    let mut permission_tokens = Vec::new();
    for segment in &segments[2..] {
        permission_tokens.extend_from_slice(&trim_commas(segment));
    }
    let look_words = words(&look_tokens);
    let exile_words = words(&exile_tokens);
    let permission_words = words(&permission_tokens);
    let has_look_head =
        look_words.starts_with(&["look", "at"]) || look_words.starts_with(&["you", "look", "at"]);
    let has_face_down_exile = exile_words.contains(&"exile")
        && exile_words
            .windows(2)
            .any(|window| window == ["face", "down"]);
    let has_persistent_play_permission = permission_words
        .windows(3)
        .any(|window| window == ["you", "may", "play"] || window == ["you", "may", "cast"])
        && permission_words
            .windows(4)
            .any(|window| window == ["for", "as", "long", "as"]);
    if !has_look_head || !has_face_down_exile || !has_persistent_play_permission {
        return Ok(None);
    }

    let Some((Verb::Look, look_verb_idx)) = effect_sentences::find_verb(&look_tokens) else {
        return Ok(None);
    };
    let look_subject =
        (look_verb_idx > 0).then(|| parse_subject(&trim_commas(&look_tokens[..look_verb_idx])));
    let Ok(mut look_effect) = super::verb_handlers::parse_look(
        &trim_commas(&look_tokens[look_verb_idx + 1..]),
        look_subject,
    ) else {
        return Ok(None);
    };
    let Some((Verb::Exile, exile_verb_idx)) = effect_sentences::find_verb(&exile_tokens) else {
        return Ok(None);
    };
    let exile_subject =
        (exile_verb_idx > 0).then(|| parse_subject(&trim_commas(&exile_tokens[..exile_verb_idx])));
    let Ok(mut exile_effect) = parse_exile(
        &trim_commas(&exile_tokens[exile_verb_idx + 1..]),
        exile_subject,
    ) else {
        return Ok(None);
    };

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { tag: look_tag, .. },
        ..
    }) = &mut look_effect
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
    }) = &mut exile_effect
    else {
        return Ok(None);
    };
    if exile_tag.as_str() != crate::tag::CompilerReferenceTag::It.as_str() {
        return Ok(None);
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

    Ok(Some(vec![look_effect, exile_effect, permission]))
}

fn parse_exile_top_library_then_play_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
    third_sentence: Option<&[OwnedLexToken]>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    use crate::grammar::primitives as grammar;

    let permission_sentence = third_sentence.unwrap_or(second_sentence);
    let mut leading_effects = Vec::new();
    let mut exile_sentence = first_sentence.to_vec();
    if let Some((prefix, suffix)) =
        grammar::split_lexed_once_on_separator(first_sentence, || grammar::kw("then").void())
    {
        let prefix = trim_commas(prefix);
        let suffix = trim_commas(suffix);
        if effect_sentences::find_verb(&prefix).is_some_and(|(verb, _)| verb == Verb::Shuffle)
            && let Ok(prefix_effects) = effect_sentences::parse_effect_sentence_lexed(&prefix)
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
        let Some(effect) = parse_exile_top_library_clause(&exile_tokens, exile_subject, false)
        else {
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
                    constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
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
                    max_plays,
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
                max_plays,
                surface,
            },
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player,
                    allow_land,
                    until_next_end_step,
                    max_plays,
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
                .with_tagged_play_max_plays(max_plays)
            } else {
                EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                    permission_tag,
                    player,
                    allow_land,
                    false,
                )
                .with_tagged_play_max_plays(max_plays)
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
    let optional_words = crate::lexer::parser_token_word_refs(optional_sentence);
    let conditional_words = crate::lexer::parser_token_word_refs(conditional_sentence);
    let permission_words = crate::lexer::parser_token_word_refs(permission_sentence);
    let lexical_owner = optional_words.contains(&"may")
        && conditional_words.first() == Some(&"if")
        && crate::word_primitives::sequence_occurs(&conditional_words, &["exile", "the", "top"])
        && crate::word_primitives::sequence_occurs(&conditional_words, &["choose", "one"])
        && permission_words.contains(&"may")
        && permission_words
            .iter()
            .any(|word| matches!(*word, "play" | "cast"));
    if !lexical_owner {
        return Ok(None);
    }
    let Some(prefix) =
        crate::grammar::structure::split_leading_result_prefix_lexed(conditional_sentence)
    else {
        return Ok(None);
    };
    if prefix.kind != crate::grammar::structure::LeadingResultPrefixKind::If
        || prefix.predicate != IfResultPredicate::Did
    {
        return Ok(None);
    }

    let Some(optional_shape) =
        bundle_grammar::clause_dispatch_shapes::parse_leading_may_shape(optional_sentence)
    else {
        return Ok(None);
    };
    let Some(mut optional_effect) =
        super::clause_pattern_helpers::parse_verb_first_clause(optional_shape.effect_tokens)?
    else {
        return Ok(None);
    };
    let optional_effects = match optional_shape.actor {
        bundle_grammar::clause_dispatch_shapes::LeadingMayActorShape::Player(player) => {
            super::chain_carry::bind_implicit_player_context(&mut optional_effect, player);
            vec![EffectAst::MayByPlayer {
                player,
                effects: vec![optional_effect],
            }]
        }
        bundle_grammar::clause_dispatch_shapes::LeadingMayActorShape::Implicit => {
            vec![EffectAst::May {
                effects: vec![optional_effect],
            }]
        }
    };
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

    let chosen_tag = crate::tag::CompilerReferenceTag::OutsideGameOrExileSelected.key();
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

    let wish_tag = crate::tag::CompilerReferenceTag::SearchedOutsideGame.key();
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
            crate::grammar::primitives::probe_shape(parse_target_player_choose_objects_clause(
                &normalized_first,
            ))
            .flatten()
        })
    else {
        return Ok(None);
    };
    let choose_tag = crate::tag::CompilerReferenceTag::It.key();

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
                    TargetAst::Player(PlayerFilter::IteratedPlayer, *span)
                }
                MixedTargetIteration::Object => {
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), *span)
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
    let Some(loop_shape) = bundle_grammar::parse_for_each_chosen_shape(second) else {
        return Ok(None);
    };
    let target = if bundle_grammar::is_any_number_target_players_or_planeswalkers_declaration(first)
    {
        TargetAst::WithCount(
            Box::new(TargetAst::PlayerOrPlaneswalker(
                PlayerFilter::Any,
                span_from_tokens(first),
            )),
            ChoiceCount::any_number(),
        )
    } else {
        let Some(choice_shape) =
            bundle_grammar::clause_dispatch_shapes::parse_choose_target_shape(first)
        else {
            return Ok(None);
        };
        let Ok(target) = parse_target_phrase(choice_shape.target_tokens) else {
            return Ok(None);
        };
        target
    };
    let Some((player_filter, includes_objects)) = chosen_target_collection_player_filter(&target)
    else {
        return Ok(None);
    };
    // The loop grammar has already separated the quantified wrapper from its
    // statement body.  Let the typed consult leaf own a traversal with an
    // inline result chain instead of re-entering the aggregate effect
    // dispatcher with that body.
    let loop_body = if let Some(effects) =
        super::consult_family::parse_consult_traversal_with_inline_followup(loop_shape.body)?
    {
        super::preserve_coordinated_effect_chain_surface(loop_shape.body, effects)
    } else {
        effect_sentences::parse_effect_sentence_lexed(loop_shape.body)?
    };
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
    let discarded_tag = crate::tag::CompilerReferenceTag::DiscardedThisWay.key();
    let count_value =
        count_value.map(|_| Value::Count(ObjectFilter::tagged(discarded_tag.clone())));

    let mut discarded_filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
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
            tag: crate::tag::CompilerReferenceTag::It.key(),
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

pub(super) fn parse_search_library_slots_to_hand_bundle(
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
            crate::tag::CompilerReferenceTag::SearchLibrarySlotsProgress.key(),
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
        crate::tag::CompilerReferenceTag::SearchLibrarySlotsProgress.key(),
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
    let searched_tag = crate::tag::CompilerReferenceTag::SearchedMultiZone.key();
    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: shape.filter.clone(),
            count: shape.count,
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

pub(crate) fn parse_complete_kicked_search_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(shape) = bundle_grammar::parse_kicked_targeted_search_count_shape(tokens) {
        let search = |count| {
            EffectAst::subject_verb_search_library(
                ObjectFilter::default()
                    .owned_by(PlayerFilter::target_player())
                    .in_zone(Zone::Library),
                Zone::Exile,
                PlayerAst::Implicit,
                PlayerAst::That,
                crate::effect::SearchSelectionMode::Optional,
                false,
                None,
                true,
                ChoiceCount::up_to(count),
                None,
                None,
                crate::effect::SearchResultReferenceSurface::Them,
                false,
                false,
                false,
            )
        };
        return Ok(Some(vec![EffectAst::SelfReplacement {
            predicate: PredicateAst::ThisSpellWasKicked,
            if_true: vec![search(shape.replacement_count)],
            if_false: vec![search(shape.default_count)],
            attach_to_previous_ability: false,
        }]));
    }
    if let Some(effects) = parse_kicked_search_library_slots_replacement_bundle(tokens)? {
        return Ok(Some(effects));
    }
    Ok(parse_kicked_multi_zone_search_destination_bundle(tokens))
}

fn parse_persistent_exile_play_tax_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let shape = bundle_grammar::parse_persistent_exile_play_tax_tokens(tokens)?;
    let tagged = crate::tag::CompilerReferenceTag::It.key();
    let target = TargetAst::Object(shape.target_filter, Some(TextSpan::synthetic()), None);
    let mut spell_filter = ObjectFilter::spell()
        .without_type(CardType::Land)
        .cast_by(shape.taxed_caster);
    spell_filter.zone = None;

    Some(vec![
        EffectAst::subject_verb_exile(target, false),
        EffectAst::subject_verb_grant_by_spec(
            crate::model::CompilerGrantSpecCore::new(
                crate::model::CompilerGrantableCore::play_from(),
                ObjectFilter::tagged(tagged.clone()),
                Zone::Exile,
            ),
            shape.permission_player,
            crate::grant::GrantDuration::Forever,
        ),
        EffectAst::subject_verb_grant_to_target(
            TargetAst::Tagged(tagged, None),
            crate::model::CompilerGrantableCore::Ability(
                crate::model::CompilerStaticAbilityCore::new(
                    crate::static_abilities::CostIncreaseManaCost::new(
                        spell_filter,
                        shape.additional_cost,
                    ),
                ),
            ),
            crate::grant::GrantDuration::Forever,
        ),
    ])
}

#[path = "effect_composition/consult_bundles.rs"]
mod consult_bundles;
pub(super) use consult_bundles::parse_consult_disposition_bundle;
use consult_bundles::{
    parse_consult_then_put_matches_battlefield_rest_bottom_bundle,
    parse_reveal_repeated_disposition_bundle, parse_reveal_until_land_put_all_graveyard_bundle,
};

#[path = "effect_composition/per_graveyard.rs"]
mod per_graveyard;
use per_graveyard::parse_choose_each_graveyard_then_owner_shuffle_bundle;

#[path = "effect_composition/delayed_collections.rs"]
mod delayed_collections;
use delayed_collections::parse_exile_collection_each_upkeep_return_bundle;

#[cfg(test)]
#[path = "effect_composition_inline_tests.rs"]
mod tests;

#[path = "effect_composition/composition_core.rs"]
mod bundle_rules_core_programs;
pub use bundle_rules_core_programs::parse_typed_effect_bundle_lexed;
#[path = "effect_composition/composition_reference.rs"]
mod bundle_rules_reference_programs;
use bundle_rules_reference_programs::{
    parse_each_player_hand_exile_play_constraints_bundle,
    parse_tap_controlled_objects_then_empty_mana_bundle,
    parse_untap_then_phase_out_until_source_leaves_bundle,
};
#[path = "effect_composition/composition_object_action.rs"]
mod bundle_rules_object_action_programs;
use bundle_rules_object_action_programs::parse_regenerate_then_gain_control_if_regenerates_bundle;
#[path = "effect_composition/composition_resource.rs"]
mod bundle_rules_resource_programs;
use bundle_rules_resource_programs::{
    parse_bid_life_for_control_bundle, parse_controller_sacrifice_consult_bundle,
    parse_energy_pay_any_destroy_bundle,
};
#[path = "effect_composition/composition_counter.rs"]
mod bundle_rules_counter_programs;
use bundle_rules_counter_programs::parse_proliferate_choose_phase_out_bundle;
#[path = "effect_composition/composition_library.rs"]
mod bundle_rules_library_programs;
use bundle_rules_library_programs::{
    parse_discard_redraw_mana_value_ladder_bundle, parse_each_player_shuffle_then_consult_bundle,
    parse_look_hand_optional_exile_play_tax_bundle,
};

use super::super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{
    CardTextError, ChooseOneModeAst, EffectAst, IfResultPredicate, LibraryBottomOrderAst,
    ObjectFilter, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::effect_sentences;
use crate::effect_sentences::SentenceInput;
use crate::filter::TaggedObjectConstraint;
use crate::grammar::effects::sequence_quad_shapes as quad_grammar;
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::grammar::sentence_markers::{self, LeadingMayActor};
use crate::lexer::{LexedClause, OwnedLexToken};
use crate::object_filters::parse_object_filter_lexed;
use crate::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::target::TaggedOpbjectRelation;
use crate::util::{helper_tag_for_tokens, strip_leading_token_words_any, trim_commas};
use crate::zone::Zone;

fn look_at_top_cards_player(effect: &EffectAst) -> Option<PlayerAst> {
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        subject: crate::cards::builders::SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { .. },
    }) = effect
    else {
        return None;
    };
    Some(*player)
}

fn look_at_top_cards_player_count_reveal(
    effect: &EffectAst,
) -> Option<(PlayerAst, crate::effect::Value, bool)> {
    let EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        subject: crate::cards::builders::SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, reveal, .. },
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone(), *reveal))
}

fn effect_ast_contains_sacrifice(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Sacrifice { .. } | SubjectVerbActionAst::SacrificeAll { .. },
            ..
        }) => true,
        EffectAst::Sequence { effects }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachTaggedWithControllerAtLastBlockedBy { effects, .. } => {
            effects.iter().any(effect_ast_contains_sacrifice)
        }
        EffectAst::TagAffected { effect, .. } => effect_ast_contains_sacrifice(effect),
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            if_true.iter().any(effect_ast_contains_sacrifice)
                || if_false.iter().any(effect_ast_contains_sacrifice)
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            effects.iter().any(effect_ast_contains_sacrifice)
                || alternative.iter().any(effect_ast_contains_sacrifice)
        }
        _ => false,
    }
}

fn rebind_one_of_those_cards_target(target: &mut TargetAst, looked_tag: &TagKey) -> bool {
    match target {
        TargetAst::WithCount(inner, count) if count.is_single() => {
            rebind_one_of_those_cards_target(inner, looked_tag)
        }
        TargetAst::Tagged(tag, _)
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            *tag = looked_tag.clone();
            true
        }
        TargetAst::Object(filter, _, reference_span)
            if *filter == ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key()) =>
        {
            *target = TargetAst::Tagged(looked_tag.clone(), *reference_span);
            true
        }
        _ => false,
    }
}

fn parse_looked_card_move_result_branch(
    tokens: &[OwnedLexToken],
    predicate: IfResultPredicate,
    looked_tag: &TagKey,
) -> Result<Option<EffectAst>, CardTextError> {
    if !crate::grammar::lexical::contains_token_word_sequence(
        tokens,
        &["one", "of", "those", "cards"],
    ) {
        return Ok(None);
    }
    let Ok(mut effects) = effect_sentences::parse_effect_sentence_lexed(tokens) else {
        return Ok(None);
    };
    let [
        EffectAst::IfResult {
            predicate: parsed_predicate,
            effects: branch,
        },
    ] = effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if *parsed_predicate != predicate
        && !(predicate == IfResultPredicate::DidNot
            && *parsed_predicate == IfResultPredicate::ExplicitDidNot)
    {
        return Ok(None);
    }
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::MoveToZone { target, .. },
            ..
        }),
    ] = branch.as_mut_slice()
    else {
        return Ok(None);
    };
    if !rebind_one_of_those_cards_target(target, looked_tag) {
        return Ok(None);
    }
    Ok(effects.pop())
}

/// Keep a looked-card collection stable across an unrelated optional action:
///
/// "Look at ... . You may [act]. If you do, put one of those cards ... .
/// If you don't, put one of those cards ... ."
///
/// A source sacrifice inside the optional action intentionally establishes
/// the source as the newest singular `it` antecedent. Both authored plural
/// references still name the earlier looked collection, so bind them to that
/// producer explicitly before reference resolution walks the optional body.
pub fn parse_look_then_may_action_if_did_or_did_not_move_looked_card(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some((library_owner, count, false)) =
        effect_sentences::parse_top_cards_view_sentence(&first_tokens)
    else {
        return Ok(None);
    };
    let looked_tag = helper_tag_for_tokens(&first_tokens, "looked_before_optional_action");

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Ok(optional_effects) = effect_sentences::parse_effect_sentence_lexed(&second_tokens) else {
        return Ok(None);
    };
    if !matches!(
        optional_effects.as_slice(),
        [EffectAst::May { .. } | EffectAst::MayByPlayer { .. }]
    ) {
        return Ok(None);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let Some(did) =
        parse_looked_card_move_result_branch(&third_tokens, IfResultPredicate::Did, &looked_tag)?
    else {
        return Ok(None);
    };
    let fourth_tokens = trim_commas(sentences[sentence_idx + 3].lowered());
    let Some(did_not) = parse_looked_card_move_result_branch(
        &fourth_tokens,
        IfResultPredicate::DidNot,
        &looked_tag,
    )?
    else {
        return Ok(None);
    };

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        library_owner,
        count,
        looked_tag,
    )];
    effects.extend(optional_effects);
    effects.extend([did, did_not]);
    Ok(Some(effects))
}

fn independent_and_or_looked_card_filters(filter: &ObjectFilter) -> Option<Vec<ObjectFilter>> {
    if filter.card_types.len() > 1
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.static_abilities.is_empty()
        && filter.any_of.is_empty()
    {
        return Some(
            filter
                .card_types
                .iter()
                .map(|card_type| {
                    let mut branch = filter.clone();
                    branch.card_types = vec![*card_type];
                    branch
                })
                .collect(),
        );
    }

    if filter.card_types.is_empty()
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.static_abilities.is_empty()
        && !filter.any_of.is_empty()
    {
        return Some(filter.any_of.clone());
    }

    None
}

fn compose_independent_looked_card_choices(
    mut filter: ObjectFilter,
    looked_tag: &TagKey,
    chosen_tag: &TagKey,
    player: PlayerAst,
) -> Option<Vec<EffectAst>> {
    filter.zone = Some(Zone::Library);
    let branches = independent_and_or_looked_card_filters(&filter)?;
    if branches.len() < 2 {
        return None;
    }

    Some(
        branches
            .into_iter()
            .map(|mut branch| {
                branch.zone = Some(Zone::Library);
                branch.tagged_constraints.push(TaggedObjectConstraint {
                    tag: looked_tag.clone(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
                // All independent branches append to one selected collection.
                // Excluding that collection prevents a multitype card from
                // being selected twice while still allowing one card of each
                // authored `and/or` kind.
                branch.tagged_constraints.push(TaggedObjectConstraint {
                    tag: chosen_tag.clone(),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                });
                EffectAst::ChooseTaggedObjectsInZone {
                    filter: branch,
                    count: ChoiceCount::up_to(1),
                    player,
                    tag: chosen_tag.clone(),
                    zone: Zone::Library,
                }
            })
            .collect(),
    )
}

/// Composes a four-sentence looked-card procedure whose final sentence is a
/// threshold self-replacement of the chosen-set disposition. Both branches
/// repeat the public look and independent typed choices, because a resolution
/// self-replacement replaces the whole executable segment at runtime.
pub fn parse_reveal_top_choose_and_or_hand_rest_bottom_with_destination_override(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, true)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(choice_shape) =
        quad_grammar::parse_choose_looked_card_and_or_shape(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    if !choice_shape.uses_and_or {
        return Ok(None);
    }
    let Some(default_disposition) = quad_grammar::parse_chosen_cards_hand_remainder_shape(
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let Some(replacement_shape) = quad_grammar::parse_chosen_cards_destination_replacement_shape(
        sentences[sentence_idx + 3].lowered(),
    ) else {
        return Ok(None);
    };
    if replacement_shape.order != default_disposition.order {
        return Ok(None);
    }

    let Some(filter) =
        effect_sentences::parse_looked_card_choice_filter(choice_shape.filter_tokens)
    else {
        return Ok(None);
    };
    let predicate = crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
        replacement_shape.predicate_tokens,
    )?;
    if !matches!(
        &predicate,
        PredicateAst::ValueComparison {
            left: crate::effect::Value::X,
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: crate::effect::Value::Fixed(_),
        }
    ) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let look = EffectAst::subject_verb_reveal_top_cards(player, count, looked_tag.clone());
    let Some(choices) =
        compose_independent_looked_card_choices(filter, &looked_tag, &chosen_tag, player)
    else {
        return Ok(None);
    };

    let mut default_effects = vec![look.clone()];
    default_effects.extend(choices.clone());
    default_effects.push(EffectAst::MoveTaggedGroupToZone {
        tag: chosen_tag.clone(),
        zone: Zone::Hand,
    });
    default_effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag.clone(),
            Some(chosen_tag.clone()),
            default_disposition.order,
            player,
        ),
    );

    let mut replacement_effects = vec![look];
    replacement_effects.extend(choices);
    replacement_effects.push(EffectAst::ChooseOneOf {
        modes: vec![
            ChooseOneModeAst {
                description: "Put the chosen cards onto the battlefield".to_string(),
                effects: vec![EffectAst::MoveTaggedGroupToZone {
                    tag: chosen_tag.clone(),
                    zone: Zone::Battlefield,
                }],
            },
            ChooseOneModeAst {
                description: "Put the chosen cards into your hand".to_string(),
                effects: vec![EffectAst::MoveTaggedGroupToZone {
                    tag: chosen_tag.clone(),
                    zone: Zone::Hand,
                }],
            },
        ],
    });
    replacement_effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            replacement_shape.order,
            player,
        ),
    );

    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true: replacement_effects,
        if_false: default_effects,
        attach_to_previous_ability: false,
    }]))
}

fn exact_flat_optional_reveal_to_hand_partition(
    effects: &[EffectAst],
) -> Option<(EffectAst, ObjectFilter, ChoiceCount, PlayerAst, Zone)> {
    let [look, choose, reveal, move_selected, remainder] = effects else {
        return None;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = look
    else {
        return None;
    };
    let EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count,
        player,
        tag: selected_tag,
        zone,
    } = choose
    else {
        return None;
    };
    let EffectAst::ForEachTagged {
        tag: revealed_tag,
        effects: reveal_effects,
    } = reveal
    else {
        return None;
    };
    if revealed_tag != selected_tag
        || !matches!(
            reveal_effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RevealTagged { .. },
                ..
            })]
        )
    {
        return None;
    }
    let EffectAst::ForEachTagged {
        tag: moved_tag,
        effects: move_effects,
    } = move_selected
    else {
        return None;
    };
    if moved_tag != selected_tag
        || !matches!(
            move_effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Hand,
                    ..
                },
                ..
            })]
        )
    {
        return None;
    }
    if !matches!(
        remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    ..
                },
            ..
        }) if tag == looked_tag && keep_tagged == selected_tag
    ) {
        return None;
    }

    Some((look.clone(), filter.clone(), *count, *player, *zone))
}

fn wrap_flat_reveal_to_hand_partition_in_may(
    effects: Vec<EffectAst>,
    exact_count: usize,
) -> Option<Vec<EffectAst>> {
    let [look, mut choose, reveal, move_selected, remainder]: [EffectAst; 5] =
        effects.try_into().ok()?;
    let EffectAst::ChooseTaggedObjectsInZone { count, .. } = &mut choose else {
        return None;
    };
    *count = ChoiceCount::exactly(exact_count);
    Some(vec![
        look,
        EffectAst::May {
            effects: vec![choose, reveal, move_selected],
        },
        remainder,
    ])
}

/// Keeps the selected set and its exact complement stable when a leading
/// conditional `instead` changes how many matching looked-at cards may be
/// revealed and moved to hand. The optional action remains a real `May`
/// around an exact-size choice, so "may reveal two" cannot select only one.
pub fn parse_look_reveal_one_or_instead_two_then_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(conditional) = crate::grammar::static_line_support::parse_leading_if_clause(
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let condition_tokens = trim_commas(conditional.condition_tokens);
    let Ok(predicate) =
        crate::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(&condition_tokens)
    else {
        return Ok(None);
    };

    let replacement_tokens = trim_commas(conditional.remainder_tokens);
    let [you, may, instead, reveal, ..] = replacement_tokens.as_slice() else {
        return Ok(None);
    };
    if !you.is_word("you")
        || !may.is_word("may")
        || !instead.is_word("instead")
        || !reveal.is_word("reveal")
    {
        return Ok(None);
    }
    let mut replacement_action = replacement_tokens;
    replacement_action.remove(2);

    let default_sentences = [
        SentenceInput::from_lexed(sentences[sentence_idx].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 1].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 3].lexed()),
    ];
    let Some(default_effects) =
        super::ordered_control_flow_programs::parse_look_at_top_reveal_match_put_rest_bottom(
            &default_sentences,
            0,
        )?
    else {
        return Ok(None);
    };
    let replacement_sentences = [
        SentenceInput::from_lexed(sentences[sentence_idx].lexed()),
        SentenceInput::from_lexed(&replacement_action),
        SentenceInput::from_lexed(sentences[sentence_idx + 3].lexed()),
    ];
    let Some(replacement_effects) =
        super::ordered_control_flow_programs::parse_look_at_top_reveal_match_put_rest_bottom(
            &replacement_sentences,
            0,
        )?
    else {
        return Ok(None);
    };

    let Some((default_look, default_filter, default_count, default_player, default_zone)) =
        exact_flat_optional_reveal_to_hand_partition(&default_effects)
    else {
        return Ok(None);
    };
    let Some((
        replacement_look,
        replacement_filter,
        replacement_count,
        replacement_player,
        replacement_zone,
    )) = exact_flat_optional_reveal_to_hand_partition(&replacement_effects)
    else {
        return Ok(None);
    };
    if default_look != replacement_look
        || default_filter != replacement_filter
        || default_player != replacement_player
        || default_zone != replacement_zone
        || default_count != ChoiceCount::up_to(1)
        || replacement_count != ChoiceCount::up_to(2)
    {
        return Ok(None);
    }

    let Some(default_effects) = wrap_flat_reveal_to_hand_partition_in_may(default_effects, 1)
    else {
        return Ok(None);
    };
    let Some(replacement_effects) =
        wrap_flat_reveal_to_hand_partition_in_may(replacement_effects, 2)
    else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true: replacement_effects,
        if_false: default_effects,
        attach_to_previous_ability: false,
    }]))
}

/// A trailing reflexive "When ... this way" refers to the selected-card zone
/// move, even when the oracle text describes the remainder before stating the
/// reflexive ability.  Keep the runtime antecedent adjacent to the result node
/// and leave rendering to restore the oracle sentence order.
pub fn parse_top_cards_move_rest_then_typed_when_result(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) =
        super::ordered_control_flow_programs::parse_top_cards_put_any_matching_to_zone_rest_bottom(
            sentences,
            sentence_idx,
        )?
    else {
        return Ok(None);
    };
    let Ok(followup) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 3].lowered())
    else {
        return Ok(None);
    };
    let [when_result @ EffectAst::WhenResult { .. }] = followup.as_slice() else {
        return Ok(None);
    };
    let Some(remainder) = effects.pop() else {
        return Ok(None);
    };
    if !matches!(
        &remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. }
                | SubjectVerbActionAst::PutTaggedRemainderInZone { .. },
            ..
        })
    ) {
        return Ok(None);
    }
    effects.push(when_result.clone());
    effects.push(remainder);
    Ok(Some(effects))
}

/// Keeps an optional looked-card battlefield move, a grant to the moved card,
/// and the exact looked-set complement in one linked program.  The ordinary
/// four-sentence fallback lowers the deployment as a generic May target
/// choice; composing through the shared looked-card producer instead gives
/// the choice, move, grant pronoun, and remainder one stable selected tag.
pub fn parse_top_cards_move_then_grant_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let partition_sentences = [
        SentenceInput::from_lexed(sentences[sentence_idx].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 1].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 3].lexed()),
    ];
    let Some(mut effects) =
        super::ordered_control_flow_programs::parse_top_cards_put_any_matching_to_zone_rest_bottom(
            &partition_sentences,
            0,
        )?
    else {
        return Ok(None);
    };

    let Ok(grant_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };
    let selected_tag = match effects.as_slice() {
        [
            _,
            EffectAst::ChooseTaggedObjectsInZone {
                tag: chosen_tag, ..
            },
            EffectAst::ForEachTagged { tag: moved_tag, .. },
            _,
        ] if chosen_tag == moved_tag => chosen_tag.clone(),
        _ => return Ok(None),
    };

    let [grant] = grant_effects.as_slice() else {
        return Ok(None);
    };
    let mut grant = grant.clone();
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantAbilitiesToTarget {
                target,
                duration: crate::effect::Until::Forever,
                condition: None,
                ..
            },
        ..
    }) = &mut grant
    else {
        return Ok(None);
    };
    if !matches!(target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
    {
        return Ok(None);
    }
    // The grant is parsed in isolation, so its pronoun still carries the
    // generic `it` tag. Bind it explicitly to the singleton selected from the
    // looked-card pool before lowering; the move, grant, and complement now
    // share one stable identity.
    *target = TargetAst::Tagged(selected_tag, None);

    let Some(remainder) = effects.pop() else {
        return Ok(None);
    };
    if !matches!(
        remainder,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary { .. },
            ..
        })
    ) {
        return Ok(None);
    }
    effects.push(grant);
    effects.push(remainder);
    Ok(Some(effects))
}

pub fn parse_sacrifice_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut sacrifice_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if !sacrifice_effects.iter().any(effect_ast_contains_sacrifice) {
        return Ok(None);
    }

    let Some(mut reveal_effects) =
        super::ordered_control_flow_programs::parse_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom(
            sentences,
            sentence_idx + 1,
        )?
    else {
        return Ok(None);
    };

    sacrifice_effects.append(&mut reveal_effects);
    Ok(Some(sacrifice_effects))
}

fn title_case_card_name(words: &[&str]) -> String {
    const LOWERCASE_WORDS: &[&str] = &[
        "a", "an", "the", "and", "or", "but", "nor", "for", "so", "yet", "of", "in", "on", "at",
        "to", "from", "with", "without", "by", "as", "into", "onto", "over", "under",
    ];
    words
        .iter()
        .filter(|word| !word.is_empty())
        .enumerate()
        .map(|(idx, word)| {
            if idx > 0 && LOWERCASE_WORDS.iter().any(|candidate| candidate == word) {
                return (*word).to_string();
            }
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut out = first.to_uppercase().to_string();
            out.push_str(chars.as_str());
            out
        })
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn search_reveal_tag(effects: &[EffectAst]) -> Option<TagKey> {
    let searched_tag = effects.iter().find_map(|effect| match effect {
        EffectAst::ChooseObjects { filter, tag, .. }
        | EffectAst::ChooseObjectsAcrossZones { filter, tag, .. }
            if filter.zone == Some(Zone::Library) =>
        {
            Some(tag.clone())
        }
        _ => None,
    })?;
    effects
        .iter()
        .any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(subject_verb)
                    if matches!(
                        &subject_verb.action,
                        SubjectVerbActionAst::RevealTagged { tag } if tag == &searched_tag
                    )
            )
        })
        .then_some(searched_tag)
}

fn named_revealed_card_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let shape = quad_grammar::parse_named_revealed_card_shape(tokens)?;
    let words = LexedClause::new(shape.name_tokens).word_refs();
    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_card_name(&words));
    Some(filter)
}

fn puts_it_onto_battlefield(tokens: &[OwnedLexToken]) -> bool {
    quad_grammar::parse_put_looked_onto_battlefield_shape(tokens)
}

fn otherwise_puts_that_card_into_hand(tokens: &[OwnedLexToken]) -> bool {
    quad_grammar::parse_put_looked_into_hand_shape(tokens)
}

fn then_shuffle(tokens: &[OwnedLexToken]) -> bool {
    quad_grammar::parse_then_shuffle_shape(tokens)
}

fn exiles_one_looked_card_face_down_and_bottoms_rest(tokens: &[OwnedLexToken]) -> bool {
    quad_grammar::parse_exile_one_and_bottom_remainder_shape(tokens)
}

fn parse_counted_looked_cards_exile_face_down(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, bool)> {
    let shape = quad_grammar::parse_counted_looked_card_exile_shape(tokens)?;
    Some((shape.count, shape.includes_remainder))
}

fn puts_looked_remainder_on_bottom(tokens: &[OwnedLexToken]) -> Option<LibraryBottomOrderAst> {
    quad_grammar::parse_looked_remainder_bottom_shape(tokens)
}

fn parse_exiled_card_cast_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(shape) = quad_grammar::parse_exiled_card_cast_filter_shape(tokens) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(shape.filter_tokens, false)?;
    if filter.zone == Some(Zone::Stack) {
        filter.zone = None;
        filter.stack_kind = None;
    }
    Ok(Some(filter))
}

fn puts_exiled_card_into_hand_if_not_cast(tokens: &[OwnedLexToken]) -> bool {
    quad_grammar::parse_exiled_card_hand_followup_shape(tokens)
}

fn parse_may_reveal_up_to_from_looked_cards(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let Some(shape) = quad_grammar::parse_may_reveal_looked_card_shape(tokens) else {
        return Ok(None);
    };
    let mut filter = effect_sentences::parse_looked_card_choice_filter(shape.filter_tokens)
        .ok_or_else(|| {
            CardTextError::ParseError("unable to parse reveal filter from looked cards".to_string())
        })?;
    filter.zone = Some(Zone::Library);

    Ok(Some((filter, shape.count)))
}

#[cfg(test)]
#[path = "branching_selection_inline_looked_partition_tests.rs"]
mod looked_partition_tests;

#[path = "branching_selection_programs/branching_selection_library.rs"]
mod branching_selection_library_programs;
use branching_selection_library_programs::{
    compose_look_at_top_may_put_onto_battlefield_or_into_hand_rest_bottom,
    parse_may_exile_filtered_looked_card, parse_selected_card_leading_if,
};
pub use branching_selection_library_programs::{
    parse_look_at_top_conditional_hand_counts_then_rest_bottom,
    parse_look_at_top_exile_counted_rest_bottom_play_while_exiled,
    parse_look_at_top_exile_one_rest_bottom_cast_else_hand,
    parse_look_at_top_may_exile_match_rest_bottom_cast_exiled,
    parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom,
    parse_look_at_top_may_reveal_match_bargain_battlefield_else_hand_then_shuffle,
    parse_look_at_top_optional_battlefield_conditional_entry_counters_then_rest_bottom,
    parse_look_at_top_optional_battlefield_then_conditional_remainder,
    parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override,
    parse_look_may_reveal_then_your_turn_battlefield_else_hand_rest_bottom,
    parse_look_reveal_match_to_hand_if_selected_matches_rest_bottom,
    parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom,
    parse_reveal_top_optional_battlefield_then_hand_rest_graveyard,
    parse_search_reveal_named_match_battlefield_else_hand_then_shuffle,
};
#[path = "branching_selection_programs/branching_selection_choice.rs"]
mod branching_selection_choice_programs;
use branching_selection_choice_programs::{
    is_if_selected_not_put_onto_battlefield_put_into_hand,
    is_may_put_selected_onto_battlefield_on_your_turn,
};

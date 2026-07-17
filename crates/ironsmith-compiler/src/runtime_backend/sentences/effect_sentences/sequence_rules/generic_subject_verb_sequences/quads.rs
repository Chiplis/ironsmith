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
use crate::filter::TaggedObjectConstraint;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::front_end::lexer::{LexedClause, OwnedLexToken};
use crate::runtime_backend::grammar::effects::sequence_quad_shapes as quad_grammar;
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::util::{
    helper_tag_for_tokens, strip_leading_token_words_any, trim_commas,
};
use crate::target::TaggedOpbjectRelation;
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
        | EffectAst::ForEachTagged { effects, .. } => {
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
pub(crate) fn parse_reveal_top_choose_and_or_hand_rest_bottom_with_destination_override(
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
    let predicate =
        crate::runtime_backend::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed(
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

/// A trailing reflexive "When ... this way" refers to the selected-card zone
/// move, even when the oracle text describes the remainder before stating the
/// reflexive ability.  Keep the runtime antecedent adjacent to the result node
/// and leave rendering to restore the oracle sentence order.
pub(crate) fn parse_top_cards_move_rest_then_typed_when_result(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = super::triples::parse_top_cards_put_any_matching_to_zone_rest_bottom(
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
pub(crate) fn parse_top_cards_move_then_grant_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let partition_sentences = [
        SentenceInput::from_lexed(sentences[sentence_idx].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 1].lexed()),
        SentenceInput::from_lexed(sentences[sentence_idx + 3].lexed()),
    ];
    let Some(mut effects) = super::triples::parse_top_cards_put_any_matching_to_zone_rest_bottom(
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
    if !matches!(target, TargetAst::Tagged(tag, None) if tag.as_str() == crate::cards::builders::IT_TAG)
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

pub(crate) fn parse_sacrifice_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom(
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
        super::triples::parse_reveal_top_choose_any_revealed_land_nonland_split_rest_bottom(
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

/// Preserves a conditional selected subset and one exact remainder across the
/// common four-sentence shape:
///
/// "Look at ... . If <predicate>, put N of those cards into your hand.
/// Otherwise, put M of them into your hand. Put the rest on the bottom ... ."
///
/// Both branches deliberately write the same `selected_tag`.  The final
/// remainder can therefore be expressed once as `looked - selected` instead
/// of taking whichever branch's last-object reference happened to survive
/// conditional lowering.
pub(crate) fn parse_look_at_top_conditional_hand_counts_then_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some((library_owner, count, false)) =
        effect_sentences::parse_top_cards_view_sentence(&first_tokens)
    else {
        return Ok(None);
    };

    let conditional_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    // Reuse the ordinary conditional sentence parser for the predicate.  It
    // already understands comparative controller predicates such as
    // "you control more creatures than each other player"; the standalone
    // predicate grammar is intentionally narrower and caused this sequence
    // rule to fall through even though the sentence parsed successfully on
    // its own.
    let Ok(parsed_conditional) = effect_sentences::parse_effect_sentence_lexed(&conditional_tokens)
    else {
        return Ok(None);
    };
    let [EffectAst::Conditional { predicate, .. }] = parsed_conditional.as_slice() else {
        return Ok(None);
    };
    let predicate = predicate.clone();
    // Sentence splitting removes the period and normalizes away the comma
    // between the condition and its action.  The consult-family conditional
    // splitter intentionally requires that punctuation boundary, so using it
    // here made otherwise-valid conditional sentences unreachable.  The
    // ordinary conditional parser above proves the sentence shape; locate the
    // branch's leading action verb to preserve the exact counted selection.
    let Some(if_true_start) = conditional_tokens
        .iter()
        .rposition(|token| token.is_word("put"))
    else {
        return Ok(None);
    };
    let if_true_tokens = trim_commas(&conditional_tokens[if_true_start..]);
    let Some(if_true_count) = parse_counted_looked_cards_into_your_hand_tokens(&if_true_tokens)
    else {
        return Ok(None);
    };

    let otherwise_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let if_false_tokens = strip_leading_token_words_any(&otherwise_tokens, &["otherwise"]);
    let Some(if_false_count) = parse_counted_looked_cards_into_your_hand_tokens(if_false_tokens)
    else {
        return Ok(None);
    };

    let remainder_tokens = trim_commas(sentences[sentence_idx + 3].lowered());
    let remainder_tokens = strip_leading_token_words_any(&remainder_tokens, &["then", "and"]);
    if !is_put_rest_on_bottom_of_library_sentence(remainder_tokens) {
        return Ok(None);
    }
    let Some(order) =
        crate::runtime_backend::grammar::effects::parse_bottom_order(remainder_tokens)
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(&first_tokens, "looked_conditional_partition");
    let selected_tag = helper_tag_for_tokens(&conditional_tokens, "conditional_selected");
    let choice = |count: u32| {
        let mut filter = ObjectFilter::tagged(looked_tag.clone());
        filter.zone = Some(Zone::Library);
        vec![
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::exactly(count as usize),
                player: PlayerAst::You,
                tag: selected_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::MoveTaggedGroupToZone {
                tag: selected_tag.clone(),
                zone: Zone::Hand,
            },
        ]
    };

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
        EffectAst::Conditional {
            predicate,
            if_true: choice(if_true_count),
            if_false: choice(if_false_count),
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            library_owner,
        ),
    ]))
}

/// Keeps the original looked-card pool authoritative when an intervening
/// optional sacrifice establishes a newer last-object reference:
///
/// "Look at ... . You may sacrifice ... . If you do, you may put a card from
/// among those cards onto the battlefield. Put the rest on the bottom ... ."
///
/// The dynamic selection filter may still refer to the sacrificed object (for
/// example through X), but its candidate domain is explicitly the earlier
/// `looked_tag`; the complement is likewise computed from that tag.
pub(crate) fn parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let Some((library_owner, count, false)) =
        effect_sentences::parse_top_cards_view_sentence(&first_tokens)
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let sacrifice_tokens = strip_leading_token_words_any(&second_tokens, &["then"]);
    let Ok(sacrifice_effects) = effect_sentences::parse_effect_sentence_lexed(sacrifice_tokens)
    else {
        return Ok(None);
    };
    if !sacrifice_effects.iter().any(effect_ast_contains_sacrifice) {
        return Ok(None);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let Some(followup) = crate::runtime_backend::front_end::grammar::sentence_markers::parse_conditional_followup_tokens(&third_tokens) else {
        return Ok(None);
    };
    if followup.actor
        != crate::runtime_backend::front_end::grammar::sentence_markers::ConditionalFollowupActor::You
    {
        return Ok(None);
    }
    let where_x_at = followup
        .tail_tokens
        .iter()
        .position(|token| token.is_word("where"));
    let action_tokens = trim_commas(
        where_x_at
            .and_then(|idx| followup.tail_tokens.get(..idx))
            .unwrap_or(followup.tail_tokens),
    );
    let Some((chooser, mut filter, tapped)) =
        effect_sentences::parse_may_put_filtered_looked_card_onto_battlefield(&action_tokens)?
    else {
        return Ok(None);
    };
    if let Some(where_x_at) = where_x_at {
        let where_x_tokens = trim_commas(&followup.tail_tokens[where_x_at..]);
        let Some(x_value) =
            crate::runtime_backend::keyword_static::parse_value_binding_clause(&where_x_tokens)
        else {
            return Ok(None);
        };
        let Some(crate::filter::Comparison::LessThanOrEqualExpr(maximum)) =
            filter.mana_value.as_mut()
        else {
            return Ok(None);
        };
        **maximum = crate::runtime_backend::util::replace_unbound_x_with_value(
            (**maximum).clone(),
            &x_value,
            "looked-card selection after an intervening action",
        )?;
    }

    let remainder_tokens = trim_commas(sentences[sentence_idx + 3].lowered());
    if !is_put_rest_on_bottom_of_library_sentence(&remainder_tokens) {
        return Ok(None);
    }
    let Some(order) =
        crate::runtime_backend::grammar::effects::parse_bottom_order(&remainder_tokens)
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(&first_tokens, "looked_before_sacrifice");
    let selected_tag = helper_tag_for_tokens(&third_tokens, "selected_after_sacrifice");
    filter.zone = Some(Zone::Library);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        library_owner,
        count,
        looked_tag.clone(),
    )];
    effects.extend(sacrifice_effects);
    effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                count: ChoiceCount::up_to(1),
                player: chooser,
                tag: selected_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::ForEachTagged {
                tag: selected_tag.clone(),
                effects: vec![EffectAst::subject_verb_put_onto_battlefield(
                    chooser,
                    TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                    tapped,
                    ReturnControllerAst::Preserve,
                )],
            },
        ],
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            library_owner,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some(player) = look_at_top_cards_player(first_effect) else {
        return Ok(None);
    };

    let Some(base_count) =
        parse_counted_looked_cards_into_your_hand_tokens(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let Some(kicked_count) = parse_if_this_spell_was_kicked_counted_looked_cards_into_hand(
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    if !is_put_rest_on_bottom_of_library_sentence(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }
    let Some(bottom_order) = crate::runtime_backend::grammar::effects::parse_bottom_order(
        sentences[sentence_idx + 3].lowered(),
    ) else {
        return Ok(None);
    };

    let kicked_looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentences[sentence_idx + 2].lowered(),
        "looked",
    );
    let base_looked_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentences[sentence_idx + 1].lowered(),
        "looked",
    );
    let kicked_chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentences[sentence_idx + 2].lowered(),
        "chosen",
    );
    let base_chosen_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentences[sentence_idx + 1].lowered(),
        "chosen",
    );
    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::ThisSpellWasKicked,
            if_true: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                player,
                crate::effect::ChoiceCount::exactly(kicked_count as usize),
                kicked_looked_tag,
                kicked_chosen_tag,
                bottom_order,
            ),
            if_false: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                player,
                crate::effect::ChoiceCount::exactly(base_count as usize),
                base_looked_tag,
                base_chosen_tag,
                bottom_order,
            ),
        },
    ]))
}

pub(crate) fn parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    if look_at_top_cards_player(first_effect).is_none() {
        return Ok(None);
    }

    let Some((chooser, battlefield_filter, tapped)) =
        effect_sentences::parse_may_put_filtered_looked_card_onto_battlefield(
            sentences[sentence_idx + 1].lowered(),
        )?
    else {
        return Ok(None);
    };
    if !parse_if_you_dont_put_card_from_among_them_into_your_hand(
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }
    if !is_put_rest_on_bottom_of_library_sentence(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }

    let Some((look_player, count, reveal)) = look_at_top_cards_player_count_reveal(first_effect)
    else {
        return Ok(None);
    };

    Ok(Some(
        compose_look_at_top_may_put_onto_battlefield_or_into_hand_rest_bottom(
            sentences[sentence_idx].lowered(),
            sentences[sentence_idx + 1].lowered(),
            look_player,
            count,
            reveal,
            chooser,
            battlefield_filter,
            tapped,
        ),
    ))
}

/// Composes the "look at the top N, you may put a matching card onto the
/// battlefield; if you don't, put a card into your hand; put the rest on the
/// bottom" shape from reusable primitives, mirroring the runtime effects the
/// retired `ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary`
/// recipe lowered to:
/// - look at the top N (minting an explicit `looked_tag`),
/// - choose up to one matching looked card (`battlefield_tag`),
/// - under an internal effect id, for each chosen card put it onto the
///   battlefield; if that did not happen, choose exactly one looked card and
///   move it to hand (`hand_tag`),
/// - for each looked card not chosen for battlefield or hand, move it to the
///   bottom of the library.
#[allow(clippy::too_many_arguments)]
fn compose_look_at_top_may_put_onto_battlefield_or_into_hand_rest_bottom(
    look_tokens: &[OwnedLexToken],
    choose_tokens: &[OwnedLexToken],
    look_player: PlayerAst,
    count: crate::effect::Value,
    reveal: bool,
    chooser: PlayerAst,
    mut battlefield_filter: ObjectFilter,
    tapped: bool,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, if reveal { "revealed" } else { "looked" });
    let battlefield_tag = helper_tag_for_tokens(choose_tokens, "chosen");
    let hand_tag = helper_tag_for_tokens(choose_tokens, "chosen_hand");

    battlefield_filter.zone = Some(Zone::Library);
    battlefield_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);

    let it = || TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);
    let mut in_battlefield_choice_filter = ObjectFilter::default();
    in_battlefield_choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(crate::cards::builders::IT_TAG),
            relation: TaggedOpbjectRelation::SameStableId,
        });
    let mut in_hand_choice_filter = ObjectFilter::default();
    in_hand_choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(crate::cards::builders::IT_TAG),
            relation: TaggedOpbjectRelation::SameStableId,
        });

    let mut look =
        EffectAst::subject_verb_look_at_top_cards(look_player, count, looked_tag.clone());
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { reveal: r, .. },
        ..
    }) = &mut look
    {
        *r = reveal;
    }

    vec![
        look,
        EffectAst::ChooseTaggedObjectsInZone {
            filter: battlefield_filter,
            count: ChoiceCount::up_to(1),
            player: chooser,
            tag: battlefield_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::IfEffectDidNotHappen {
            effect: Box::new(EffectAst::ForEachTagged {
                tag: battlefield_tag.clone(),
                effects: vec![EffectAst::subject_verb_put_onto_battlefield(
                    chooser,
                    it(),
                    tapped,
                    ReturnControllerAst::Preserve,
                )],
            }),
            otherwise: vec![
                EffectAst::ChooseTaggedObjectsInZone {
                    filter: hand_filter,
                    count: ChoiceCount::exactly(1),
                    player: chooser,
                    tag: hand_tag.clone(),
                    zone: Zone::Library,
                },
                EffectAst::ForEachTagged {
                    tag: hand_tag.clone(),
                    effects: vec![EffectAst::subject_verb_move_to_zone(
                        it(),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                },
            ],
        },
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    battlefield_tag,
                    in_battlefield_choice_filter,
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::Conditional {
                    predicate: PredicateAst::TaggedMatches(hand_tag, in_hand_choice_filter),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        it(),
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            }],
        },
    ]
}

pub(crate) fn parse_look_at_top_may_reveal_match_bargain_battlefield_else_hand_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    let Some((mut filter, reveal_count)) =
        parse_may_reveal_up_to_from_looked_cards(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };

    if !quad_grammar::parse_bargained_revealed_battlefield_shape(
        sentences[sentence_idx + 2].lowered(),
    ) || !quad_grammar::parse_otherwise_revealed_hand_shape(
        sentences[sentence_idx + 3].lowered(),
    ) || !then_shuffle(sentences[sentence_idx + 4].lowered())
    {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag),
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: reveal_count,
            player,
            tag: revealed_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_reveal_tagged(revealed_tag.clone()),
        EffectAst::Conditional {
            predicate: PredicateAst::ThisSpellPaidLabel("Bargain".into()),
            if_true: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(revealed_tag.clone(), None),
                Zone::Battlefield,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(revealed_tag, None),
                Zone::Hand,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]))
}

/// "you may exile a <filter> card from among them" — the optional single-card
/// exile pick from a previously looked-at set.
fn parse_may_exile_filtered_looked_card(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(shape) = quad_grammar::parse_may_exile_looked_card_shape(tokens) else {
        return Ok(None);
    };
    let Some(mut filter) = effect_sentences::parse_looked_card_choice_filter(shape.filter_tokens)
    else {
        return Ok(None);
    };
    filter.zone = Some(Zone::Library);
    Ok(Some(filter))
}

/// "Look at the top N cards of your library. You may exile a <filter> card
/// from among them. Put the rest on the bottom of your library in
/// a random/any order. You may cast the exiled card <this turn|without paying
/// its mana cost...>."
pub(crate) fn parse_look_at_top_may_exile_match_rest_bottom_cast_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    let Some(exile_filter) =
        parse_may_exile_filtered_looked_card(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };
    let Some(order) = puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered()) else {
        return Ok(None);
    };
    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 3].lowered())?
    else {
        return Ok(None);
    };
    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");

    // The final sentence can be either a temporary permission ("this turn")
    // or an immediate cast instruction during resolution. Both consume the
    // same selected exiled-card collection, but they must remain distinct at
    // runtime: an immediate free cast is not an until-end-of-turn grant.
    let permission_effect = match permission {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: permission_player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            exiled_tag.clone(),
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CastTagged {
                    player: permission_player,
                    allow_land,
                    as_copy,
                    without_paying_mana_cost,
                    cost_reduction,
                    ..
                },
            ..
        }) if !as_copy => EffectAst::subject_verb_cast_tagged(
            exiled_tag.clone(),
            permission_player,
            allow_land,
            false,
            without_paying_mana_cost,
            cost_reduction,
        ),
        _ => return Ok(None),
    };

    let mut choice_filter = exile_filter;
    choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: ChoiceCount::up_to(1),
            player,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            order,
            player,
        ),
        permission_effect,
    ]))
}

pub(crate) fn parse_look_at_top_exile_one_rest_bottom_cast_else_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        effect_sentences::parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }
    if !exiles_one_looked_card_face_down_and_bottoms_rest(sentences[sentence_idx + 1].lowered()) {
        return Ok(None);
    }
    let Some(cast_filter) = parse_exiled_card_cast_filter(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };
    if !puts_exiled_card_into_hand_if_not_cast(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            LibraryBottomOrderAst::Random,
            player,
        ),
        EffectAst::May {
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(exiled_tag.clone(), cast_filter),
                if_true: vec![EffectAst::subject_verb_cast_tagged(
                    exiled_tag.clone(),
                    player,
                    false,
                    false,
                    true,
                    None,
                )],
                if_false: Vec::new(),
            }],
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(exiled_tag, None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
    ]))
}

pub(crate) fn parse_look_at_top_exile_counted_rest_bottom_play_while_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    let (look_tokens, exile_count, bottom_order) =
        if let Some(split) = quad_grammar::parse_look_exile_split_shape(first_clause.tokens()) {
            let Some((count, includes_remainder)) =
                parse_counted_looked_cards_exile_face_down(split.exile_tokens)
            else {
                return Ok(None);
            };
            let order = if includes_remainder {
                puts_looked_remainder_on_bottom(split.exile_tokens)
            } else {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered())
            };
            let Some(order) = order else {
                return Ok(None);
            };
            (split.look_tokens, count, order)
        } else {
            let Some((count, includes_remainder)) =
                parse_counted_looked_cards_exile_face_down(sentences[sentence_idx + 1].lowered())
            else {
                return Ok(None);
            };
            let order = if includes_remainder {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 1].lowered())
            } else {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered())
            };
            let Some(order) = order else {
                return Ok(None);
            };
            (first_clause.tokens(), count, order)
        };

    let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(look_tokens) else {
        return Ok(None);
    };
    let [look_effect] = look_effects.as_slice() else {
        return Ok(None);
    };
    let Some(library_owner) = look_at_top_cards_player(look_effect) else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
        ..
    }) = look_effect
    else {
        return Ok(None);
    };

    let Some(permission_effect) =
        parse_cast_or_play_tagged_clause(sentences[sentence_idx + 3].lowered())?
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                player: permission_player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                ..
            },
        ..
    }) = permission_effect
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(
            library_owner.clone(),
            count.clone(),
            looked_tag.clone(),
        ),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: exile_count,
            player: PlayerAst::You,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            bottom_order,
            library_owner,
        ),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            exiled_tag,
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
        ),
    ]))
}

pub(crate) fn parse_search_reveal_named_match_battlefield_else_hand_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(searched_tag) = search_reveal_tag(&effects) else {
        return Ok(None);
    };
    let Some(named_filter) = named_revealed_card_filter(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    if !puts_it_onto_battlefield(sentences[sentence_idx + 1].lowered())
        || !otherwise_puts_that_card_into_hand(sentences[sentence_idx + 2].lowered())
        || !then_shuffle(sentences[sentence_idx + 3].lowered())
    {
        return Ok(None);
    }

    effects.push(EffectAst::Conditional {
        predicate: PredicateAst::TaggedMatches(searched_tag.clone(), named_filter),
        if_true: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag.clone(), None),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
        if_false: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(searched_tag, None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::ShuffleLibrary,
    ));
    Ok(Some(effects))
}

#[cfg(test)]
mod looked_partition_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn battlefield_grant_keeps_selected_tag_and_exact_looked_complement() {
        let raw = [
            "Reveal the top four cards of your library",
            "You may put a creature card from among them onto the battlefield",
            "It gains \"At the beginning of your end step, return this creature to its owner's hand.\"",
            "Then put the rest of the cards revealed this way on the bottom of your library in any order",
        ];
        let lexed = raw
            .iter()
            .enumerate()
            .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
            .collect::<Vec<_>>();
        let sentences = lexed
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();

        let effects = parse_top_cards_move_then_grant_rest_bottom(&sentences, 0)
            .expect("grant partition parser should not error")
            .expect("looked battlefield/grant/remainder shape");
        let [look, choose, move_each, grant, remainder] = effects.as_slice() else {
            panic!("expected look/choose/move/grant/remainder program: {effects:#?}");
        };
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    reveal: true,
                    tag: looked_tag,
                    ..
                },
            ..
        }) = look
        else {
            panic!("expected one public reveal producer: {look:#?}");
        };
        let EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            tag: selected_tag,
            ..
        } = choose
        else {
            panic!("expected tagged looked-card choice: {choose:#?}");
        };
        assert_eq!(*count, ChoiceCount::up_to(1));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));
        assert!(matches!(
            move_each,
            EffectAst::ForEachTagged { tag, .. } if tag == selected_tag
        ));
        assert!(matches!(
            grant,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Tagged(tag, None),
                        ..
                    },
                ..
            }) if tag == selected_tag
        ));
        assert!(matches!(
            remainder,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag,
                        keep_tagged: Some(keep_tagged),
                        order: LibraryBottomOrderAst::ChooserChooses,
                        ..
                    },
                ..
            }) if tag == looked_tag && keep_tagged == selected_tag
        ));
    }

    #[test]
    fn conditional_cardinality_branches_share_one_selected_tag_and_one_complement() {
        let raw = [
            "Look at the top five cards of your library",
            "If you control more creatures than each other player, put two of those cards into your hand",
            "Otherwise, put one of them into your hand",
            "Then put the rest on the bottom of your library in any order",
        ];
        let lexed = raw
            .iter()
            .enumerate()
            .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
            .collect::<Vec<_>>();
        let sentences = lexed
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();

        let effects = parse_look_at_top_conditional_hand_counts_then_rest_bottom(&sentences, 0)
            .expect("conditional partition parser should not error")
            .expect("Advice from the Fae shape should parse");
        assert_eq!(effects.len(), 3);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    tag: looked_tag, ..
                },
            ..
        }) = &effects[0]
        else {
            panic!("expected explicit looked-card producer: {:?}", effects[0]);
        };
        let EffectAst::Conditional {
            if_true, if_false, ..
        } = &effects[1]
        else {
            panic!("expected cardinality conditional: {:?}", effects[1]);
        };

        let branch = |effects: &[EffectAst]| {
            let [
                EffectAst::ChooseTaggedObjectsInZone {
                    count, tag, filter, ..
                },
                EffectAst::MoveTaggedGroupToZone {
                    tag: moved_tag,
                    zone: Zone::Hand,
                },
            ] = effects
            else {
                panic!("expected choose-and-move branch: {effects:?}");
            };
            assert_eq!(tag, moved_tag);
            assert!(filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *looked_tag
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            }));
            (*count, tag.clone())
        };
        let (true_count, true_tag) = branch(if_true);
        let (false_count, false_tag) = branch(if_false);
        assert_eq!(true_count, ChoiceCount::exactly(2));
        assert_eq!(false_count, ChoiceCount::exactly(1));
        assert_eq!(true_tag, false_tag);

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order,
                    ..
                },
            ..
        }) = &effects[2]
        else {
            panic!("expected exact bottom complement: {:?}", effects[2]);
        };
        assert_eq!(tag, looked_tag);
        assert_eq!(keep_tagged, &true_tag);
        assert_eq!(*order, LibraryBottomOrderAst::ChooserChooses);
    }

    #[test]
    fn intervening_sacrifice_does_not_replace_the_looked_candidate_pool() {
        let raw = [
            "Look at the top seven cards of your library",
            "Then you may sacrifice a creature",
            "If you do, you may put a creature card with mana value X or less from among those cards onto the battlefield, where X is 1 plus the sacrificed creature's mana value",
            "Put the rest on the bottom of your library in a random order",
        ];
        let lexed = raw
            .iter()
            .enumerate()
            .map(|(idx, line)| lex_line(line, idx).expect("sentence should lex"))
            .collect::<Vec<_>>();
        let sentences = lexed
            .iter()
            .map(|tokens| SentenceInput::from_lexed(tokens))
            .collect::<Vec<_>>();

        let effects =
            parse_look_then_may_sacrifice_if_did_select_battlefield_rest_bottom(&sentences, 0)
                .expect("intervening-action partition parser should not error")
                .expect("Birthing Ritual shape should parse");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::LookAtTopCards {
                    tag: looked_tag, ..
                },
            ..
        }) = &effects[0]
        else {
            panic!("expected explicit looked-card producer: {:?}", effects[0]);
        };
        let EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: selection,
        } = &effects[effects.len() - 2]
        else {
            panic!("expected sacrifice-result gate: {effects:?}");
        };
        let [
            EffectAst::ChooseTaggedObjectsInZone {
                filter,
                tag: selected_tag,
                count,
                zone,
                ..
            },
            EffectAst::ForEachTagged { tag: moved_tag, .. },
        ] = selection.as_slice()
        else {
            panic!("expected selected subset and battlefield move: {selection:?}");
        };
        assert_eq!(*count, ChoiceCount::up_to(1));
        assert_eq!(*zone, Zone::Library);
        assert_eq!(selected_tag, moved_tag);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *looked_tag
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        }));

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                    tag,
                    keep_tagged: Some(keep_tagged),
                    order,
                    ..
                },
            ..
        }) = effects.last().expect("bottom complement")
        else {
            panic!("expected exact bottom complement: {effects:?}");
        };
        assert_eq!(tag, looked_tag);
        assert_eq!(keep_tagged, selected_tag);
        assert_eq!(*order, LibraryBottomOrderAst::Random);
    }
}

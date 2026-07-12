use super::super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, LibraryBottomOrderAst, ObjectFilter, PlayerAst,
    PredicateAst, ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::filter::TaggedObjectConstraint;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::front_end::lexer::{LexedClause, OwnedLexToken};
use crate::runtime_backend::grammar::effects::sequence_quad_shapes as quad_grammar;
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::util::helper_tag_for_tokens;
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
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. } => {
            effects.iter().any(effect_ast_contains_sacrifice)
        }
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
            ),
            if_false: EffectAst::compose_put_some_into_hand_rest_on_bottom_of_library(
                player,
                crate::effect::ChoiceCount::exactly(base_count as usize),
                base_looked_tag,
                base_chosen_tag,
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
        EffectAst::ChooseObjects {
            filter: battlefield_filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: chooser,
            tag: battlefield_tag.clone(),
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
                EffectAst::ChooseObjects {
                    filter: hand_filter,
                    count: ChoiceCount::exactly(1),
                    count_value: None,
                    player: chooser,
                    tag: hand_tag.clone(),
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
        EffectAst::ChooseObjects {
            filter,
            count: reveal_count,
            count_value: None,
            player,
            tag: revealed_tag.clone(),
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
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                player: permission_player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                ..
            },
        ..
    }) = permission
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");

    let mut choice_filter = exile_filter;
    choice_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: choice_filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player,
            tag: exiled_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            order,
            player,
        ),
        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            exiled_tag,
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
        ),
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
        EffectAst::ChooseObjects {
            filter: choice_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player,
            tag: exiled_tag.clone(),
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
        EffectAst::ChooseObjects {
            filter: choice_filter,
            count: exile_count,
            count_value: None,
            player: PlayerAst::You,
            tag: exiled_tag.clone(),
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

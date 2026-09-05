use super::super::super::clause_pattern_helpers::parse_choose_target_prelude_sentence;
use super::super::super::clause_primitives::parse_choose_card_name_clause;
use super::super::super::dispatch_entry::{
    ConsultCastCost, consult_cast_effects, consult_stop_rule_is_single_match,
    parse_bargained_face_down_cast_mana_value_gate, parse_consult_bottom_remainder_clause,
    parse_consult_cast_clause, parse_consult_traversal_sentence,
    parse_if_declined_put_match_into_hand, parse_if_you_cant_sentence, parse_if_you_dont_sentence,
    parse_looked_card_choice_filter, parse_top_cards_view_sentence,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, LibraryConsultModeAst, LibraryConsultStopRuleAst,
    ObjectFilter, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Value};
use crate::effect_sentences;
use crate::effect_sentences::SentenceInput;
use crate::grammar::effects::{
    ExileLibraryPlayerShape, control_copy_attach_shapes::BattlefieldControllerShape,
    looked_card_shapes as looked_grammar, parse_exile_dynamic_top_library_shape,
    sequence_quad_shapes as quad_grammar, triple_sequence_shapes as triple_grammar,
};
use crate::grammar::lexical::TokenWordView;
use crate::grammar::sentence_markers::{self, ConditionalFollowupActor, LeadingMayActor};
use crate::grammar::shared_util::aggregate_constraints::lift_total_mana_value_choice_constraint;
use crate::lexer::OwnedLexToken;
use crate::object::CounterType;
use crate::object_filters::parse_object_filter_lexed;
use crate::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::target::ChooseSpec;
use crate::target::{ObjectRef, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::util::{
    helper_tag_for_tokens, parse_target_phrase, strip_leading_token_words_any, trim_commas,
};
use crate::zone::Zone;

/// Preserve the exact result collection across:
///
/// "Each player mills a card. If a land card was milled this way, create ... .
/// Until end of turn, you may cast a spell from among those cards."
///
/// The tag is the semantic boundary: the land test and permission can see only
/// cards affected by this mill instruction, never unrelated graveyard cards.
pub fn parse_each_player_mill_then_land_result_then_cast_one_milled_spell(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let second_tokens = sentences[sentence_idx + 1].lowered();
    let third_tokens = sentences[sentence_idx + 2].lowered();
    if !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(first_tokens),
        &["each", "player", "mills", "a", "card"],
    ) || !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(second_tokens),
        &[
            "if", "a", "land", "card", "was", "milled", "this", "way", "create", "a", "treasure",
            "token",
        ],
    ) || !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(third_tokens),
        &[
            "until", "end", "of", "turn", "you", "may", "cast", "a", "spell", "from", "among",
            "those", "cards",
        ],
    ) {
        return Ok(None);
    }

    let Ok(mut mill_effects) = effect_sentences::parse_effect_sentence_lexed(first_tokens) else {
        return Ok(None);
    };
    let [mill_effect] = mill_effects.as_mut_slice() else {
        return Ok(None);
    };
    let milled_tag = helper_tag_for_tokens(first_tokens, "milled");
    if super::reference_linked_programs::tag_single_mill_effect(mill_effect, &milled_tag).is_none()
    {
        let exact_each_player_mill = matches!(
            mill_effect,
            EffectAst::ForEachPlayer { effects }
                if matches!(effects.as_slice(), [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::Mill { .. },
                    ..
                })])
        );
        if !exact_each_player_mill {
            return Ok(None);
        }
        let whole_batch = mill_effect.clone();
        *mill_effect = EffectAst::TagAffected {
            effect: Box::new(whole_batch),
            tag: milled_tag.clone(),
        };
    }

    let Some(create_start) =
        crate::slice_primitives::select_position(second_tokens, |token| token.is_word("create"))
    else {
        return Ok(None);
    };
    let Ok(create_effects) =
        effect_sentences::parse_effect_sentence_lexed(&second_tokens[create_start..])
    else {
        return Ok(None);
    };
    if create_effects.len() != 1 {
        return Ok(None);
    }
    let mut land = ObjectFilter::default();
    land.card_types = vec![CardType::Land];
    land.set_prior_effect_action_surface(Some(ironsmith_core::PriorEffectAction::Milled));

    let permission_surface = ironsmith_core::GrantPlayTaggedSurface::default()
        .with_leading_duration(true)
        .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::SpellsFromAmongThoseCards);
    let permission =
        EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
            milled_tag.clone(),
            PlayerAst::You,
            false,
            false,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            Some(permission_surface),
        )
        .with_tagged_play_max_plays(Some(1));

    Ok(Some(vec![
        mill_effects.pop().expect("one parsed mill effect"),
        EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(milled_tag, land),
            if_true: create_effects,
            if_false: Vec::new(),
        },
        permission,
    ]))
}

#[cfg(test)]
#[path = "ordered_control_flow_inline_mill_result_permission_tests.rs"]
mod mill_result_permission_tests;

fn look_at_top_cards_parts(effect: &EffectAst) -> Option<(PlayerAst, Value)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: crate::cards::builders::SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone()))
}

fn chosen_kind_consult_branch_effects(
    tokens: &[OwnedLexToken],
    filter: ObjectFilter,
    order: crate::cards::builders::LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let all_tag = helper_tag_for_tokens(tokens, "revealed");
    let match_tag = helper_tag_for_tokens(tokens, "chosen");
    vec![
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            LibraryConsultModeAst::Reveal,
            filter,
            LibraryConsultStopRuleAst::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(match_tag.clone(), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            all_tag,
            Some(match_tag),
            order,
            PlayerAst::You,
        ),
    ]
}

pub fn parse_choose_land_or_nonland_then_consult_to_hand_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let third = trim_commas(sentences[sentence_idx + 2].lowered());

    let Some(shape) =
        triple_grammar::parse_land_or_nonland_consult_sequence_tokens(&first, &second, &third)
    else {
        return Ok(None);
    };

    let land_filter = ObjectFilter {
        card_types: vec![CardType::Land],
        ..Default::default()
    };
    let nonland_filter = ObjectFilter {
        excluded_card_types: vec![CardType::Land],
        ..Default::default()
    };

    Ok(Some(vec![
        EffectAst::subject_verb_choose_named_option(
            PlayerAst::You,
            vec!["land".to_string(), "nonland".to_string()],
        ),
        EffectAst::Conditional {
            predicate: PredicateAst::SourceChosenOption("land".to_string()),
            if_true: chosen_kind_consult_branch_effects(
                sentences[sentence_idx + 1].lowered(),
                land_filter,
                shape.remainder_order,
            ),
            if_false: chosen_kind_consult_branch_effects(
                sentences[sentence_idx + 1].lowered(),
                nonland_filter,
                shape.remainder_order,
            ),
        },
    ]))
}

fn flatten_sequence_effects(effects: &[EffectAst]) -> Vec<EffectAst> {
    let mut flattened = Vec::new();
    for effect in effects {
        match effect {
            EffectAst::Sequence { effects } => flattened.extend(flatten_sequence_effects(effects)),
            _ => flattened.push(effect.clone()),
        }
    }
    flattened
}

fn is_payment_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PayMana { .. }
                | SubjectVerbActionAst::PayEnergy { .. }
                | SubjectVerbActionAst::PayAnyEnergy { .. }
                | SubjectVerbActionAst::PayAnyLife { .. }
                | SubjectVerbActionAst::PayLife { .. }
                | SubjectVerbActionAst::LoseLife { .. },
            ..
        })
    )
}

pub(crate) fn parse_optional_payment_sentence(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(effects) = effect_sentences::parse_effect_sentence_lexed(tokens) else {
        return Ok(None);
    };
    let payment_effects = match effects.as_slice() {
        [EffectAst::May { effects }] => flatten_sequence_effects(effects),
        [EffectAst::MayByPlayer { player, effects }]
            if *player == default_player || *player == PlayerAst::You =>
        {
            flatten_sequence_effects(effects)
        }
        _ => return Ok(None),
    };
    if payment_effects.is_empty() || !payment_effects.iter().all(is_payment_effect) {
        return Ok(None);
    }
    Ok(Some(payment_effects))
}

pub fn parse_search_then_player_names_card_conditional_put_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let Some(_shape) = triple_grammar::parse_search_then_name_shape(
        &first,
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let searched_tag = crate::tag::CompilerReferenceTag::Searched.bind();
    let mut search_filter = ObjectFilter::default();
    search_filter.owner = Some(PlayerFilter::DamagedPlayer);
    search_filter.zone = Some(Zone::Library);
    let search_effects = vec![EffectAst::ChooseObjectsAcrossZones {
        filter: search_filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::You,
        tag: searched_tag.clone(),
        zones: vec![Zone::Library],
        search_mode: Some(crate::effect::SearchSelectionMode::Exact),
    }];
    let chosen_name_tag = crate::tag::CompilerReferenceTag::ChosenName.bind();

    let mut creature_filter = ObjectFilter::default();
    creature_filter.card_types.push(CardType::Creature);
    let mut chosen_name_filter = ObjectFilter::default();
    chosen_name_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_name_tag.clone(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });

    let mut effects = search_effects;
    effects.push(EffectAst::subject_verb_choose_card_name(
        PlayerAst::That,
        None,
        chosen_name_tag,
    ));
    effects.push(EffectAst::Conditional {
        predicate: PredicateAst::And(
            Box::new(PredicateAst::TaggedMatches(
                searched_tag.clone(),
                creature_filter,
            )),
            Box::new(PredicateAst::Not(Box::new(PredicateAst::TaggedMatches(
                searched_tag.clone(),
                chosen_name_filter,
            )))),
        ),
        if_true: vec![EffectAst::May {
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag.clone(), None),
                Zone::Battlefield,
                false,
                crate::cards::builders::ReturnControllerAst::You,
                false,
                None,
            )],
        }],
        if_false: Vec::new(),
    });
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::That,
        SubjectVerbActionAst::ShuffleLibrary,
    ));

    Ok(Some(effects))
}

pub fn parse_choose_name_reveal_top_matching_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(choose_name) = parse_choose_card_name_clause(sentences[sentence_idx].lowered())?
    else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ChooseCardName {
            tag: chosen_tag, ..
        },
        ..
    }) = &choose_name
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) = triple_grammar::parse_chosen_name_reveal_shape(
        &second_tokens,
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let view_tokens = trim_commas(&second_tokens[shape.view]);
    let Some((player, count, true)) = parse_top_cards_view_sentence(&view_tokens) else {
        return Ok(None);
    };
    let looked_tag = helper_tag_for_tokens(&view_tokens, "revealed");
    let mut name_match_filter = ObjectFilter::default();
    name_match_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag.clone(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });

    Ok(Some(vec![
        choose_name,
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    name_match_filter,
                ),
                if_true: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ]))
}

/// Composes the "choose from looked-at cards into hand, rest into graveyard"
/// follow-up shape from reusable primitives, mirroring the runtime effects the
/// retired `ChooseFromLookedCardsIntoHandRestIntoGraveyard` recipe lowered to.
///
/// `looked_tag` must reference the cards already looked at / milled by a prior
/// effect (the recipe read this from `ctx.last_object_tag`):
/// - For a library source, pass the explicit tag the prior look effect minted
///   so the rest-into-graveyard split can iterate that exact collection.
/// - For a graveyard source (e.g. after a mill), pass `crate::tag::CompilerReferenceTag::It.as_str()` so the choose
///   filter resolves the prior milled collection via `resolve_it_tag`; the
///   rest already sits in the graveyard, so no split effect is emitted.
#[allow(clippy::too_many_arguments)]
pub fn compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
    chooser: PlayerAst,
    mut filter: ObjectFilter,
    looked_tag: TagKey,
    chosen_tag: TagKey,
    source_zone: Zone,
    reveal_chosen: bool,
    if_not_chosen: Vec<EffectAst>,
    choice_count: ChoiceCount,
) -> Vec<EffectAst> {
    // The producing action is authoritative. Generic object-filter parsing may
    // retain a battlefield default for a bare type word, but "from among
    // them" is scoped to the exact looked/revealed/milled collection.
    filter.zone = Some(source_zone);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let mut effects = vec![if source_zone == Zone::Library {
        EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count: choice_count,
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        }
    } else {
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: choice_count,
            count_value: None,
            player: chooser,
            tag: chosen_tag.clone(),
            zones: vec![source_zone],
            search_mode: None,
        }
    }];

    if reveal_chosen {
        effects.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_reveal_tagged(chosen_tag.clone())],
        });
    }

    let move_to_hand = EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    };
    effects.push(move_to_hand);
    if !if_not_chosen.is_empty() {
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: if_not_chosen,
        });
    }

    if source_zone == Zone::Library {
        // Keep the source collection explicit here. Self-replacement clauses
        // such as Gather the Pack replace the chosen subset while the
        // remainder must continue to range over the original revealed set.
        // Encoding the split as a nested membership test preserves those two
        // independently-scoped tags through replacement lowering.
        let mut in_chosen_filter = ObjectFilter::default();
        in_chosen_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::It.bind(),
                relation: TaggedOpbjectRelation::SameStableId,
            });
        effects.push(EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(chosen_tag, in_chosen_filter),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
    }

    effects
}

pub(crate) fn parse_any_number_revealed_this_way_choice(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, ObjectFilter)> {
    let choice_tokens = trim_commas(tokens);
    let shape = triple_grammar::parse_any_number_revealed_choice_shape(&choice_tokens)?;
    let filter_tokens = trim_commas(&choice_tokens[shape.filter]);
    let mut filter = effect_sentences::parse_looked_card_choice_filter(&filter_tokens)?;
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;
    Some((shape.count, filter))
}

pub(crate) fn looked_choice_filter_can_include_card_type(filter: &ObjectFilter, card_type: CardType) -> bool {
    filter.card_types.contains(&card_type)
        || filter
            .any_of
            .iter()
            .any(|branch| looked_choice_filter_can_include_card_type(branch, card_type))
}

pub fn parse_reveal_top_one_hand_gain_mana_value_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) = triple_grammar::parse_reveal_one_gain_mana_value_shape(
        &first,
        &second,
        sentences[sentence_idx + 2].lowered(),
    ) else {
        return Ok(None);
    };
    let Some((player, count, true)) = parse_top_cards_view_sentence(&first[shape.view]) else {
        return Ok(None);
    };
    let Ok(mut gain_effects) = effect_sentences::parse_effect_sentence_lexed(&second) else {
        return Ok(None);
    };

    let revealed_tag = helper_tag_for_tokens(&first, "revealed");
    let chosen_tag = helper_tag_for_tokens(&first, "chosen");
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { amount },
            ..
        }),
    ] = gain_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    *amount = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(chosen_tag.clone())));

    let mut choice_filter = ObjectFilter::tagged(revealed_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(player, count, revealed_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: choice_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        gain_effects.remove(0),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: revealed_tag,
                keep_tagged: chosen_tag,
                zone: Zone::Graveyard,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            },
        ),
    ]))
}

/// Composes the "for each card type, put a card of that type from among the
/// revealed cards into your hand, rest on bottom" follow-up shape from reusable
/// primitives. This replaces the retired
/// `ChooseFromLookedCardsForEachCardType*IntoHandRestOnBottomOfLibrary` recipe
/// variants and lowers to the same runtime `Effect` tree they did.
///
/// Per card type, a `ChooseObjectsAcrossZones` (up to 1, of that type, from the
/// prior looked cards not already chosen, sharing one `chosen_tag`) is emitted;
/// when `spell_filter` is set, that choose is gated behind a value comparison
/// that the player has cast at least one matching spell of that type this turn.
/// The chosen cards then move to hand via `MoveTaggedGroupToZone` (which keeps
/// the iterated reference internal to lowering, so no bare `it` surfaces) and
/// the looked remainder goes to the bottom.
///
/// `looked_tag` must reference the cards already looked at by a prior effect.
fn compose_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom(
    player: PlayerAst,
    looked_tag: TagKey,
    chosen_tag: TagKey,
    card_types: &[CardType],
    spell_filter: Option<&ObjectFilter>,
    order: crate::cards::builders::LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let chooser_player_filter = PlayerFilter::You;
    let mut effects = Vec::new();
    for card_type in card_types {
        let mut choose_filter = ObjectFilter::default();
        choose_filter.zone = Some(Zone::Library);
        choose_filter.card_types.push(*card_type);
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: looked_tag.clone(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        choose_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: chosen_tag.clone(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });

        let choose = EffectAst::ChooseObjectsAcrossZones {
            filter: choose_filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player,
            tag: chosen_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: None,
        };

        if let Some(spell_filter) = spell_filter {
            let mut typed_spell_filter = (*spell_filter).clone();
            if !typed_spell_filter.card_types.contains(card_type) {
                typed_spell_filter.card_types.push(*card_type);
            }
            effects.push(EffectAst::Conditional {
                predicate: PredicateAst::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching {
                        player: chooser_player_filter.clone(),
                        filter: typed_spell_filter,
                        exclude_source: false,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                },
                if_true: vec![choose],
                if_false: Vec::new(),
            });
        } else {
            effects.push(choose);
        }
    }

    effects.push(EffectAst::MoveTaggedGroupToZone {
        tag: chosen_tag.clone(),
        zone: Zone::Hand,
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            player,
        ),
    );

    effects
}

/// Composes a two-stage selection from one looked-at set: first up to one card
/// goes to hand, then any number of the remaining matching cards go to a public
/// zone, and everything not moved by either stage goes to the graveyard. Both
/// moves share a typed affected-object tag so the remainder excludes the union
/// even though the first subset has already left the source zone.
pub fn parse_top_cards_one_hand_then_matching_to_zone_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(hand_action) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    if !looked_grammar::is_one_looked_card_into_hand_shape(hand_action.tail_tokens) {
        return Ok(None);
    }
    let chooser = effect_sentences::leading_may_actor_to_player(hand_action.actor, player);

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let third_action_tokens = strip_leading_token_words_any(&third_tokens, &["then", "and"]);
    let Some(matching_action) =
        sentence_markers::parse_leading_may_action_tokens(third_action_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    let matching_chooser =
        effect_sentences::leading_may_actor_to_player(matching_action.actor, player);
    if matching_chooser != chooser
        || triple_grammar::parse_looked_remainder_shape(third_action_tokens)
            != Some(triple_grammar::LookedRemainderShape::Graveyard)
    {
        return Ok(None);
    }
    let Some((
        choice_count,
        mut matching_filter,
        aggregate_constraint,
        destination,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = parse_counted_from_looked_cards_action(matching_action.tail_tokens)
    else {
        return Ok(None);
    };
    if choice_count != ChoiceCount::any_number()
        || aggregate_constraint.is_some()
        || all_matching
        || !matches!(destination, Zone::Hand | Zone::Battlefield)
    {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let hand_tag = helper_tag_for_tokens(&second_tokens, "chosen_hand");
    let matching_tag = helper_tag_for_tokens(&third_tokens, "chosen_matching");
    let kept_tag = helper_tag_for_tokens(&third_tokens, "kept");

    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    matching_filter.zone = Some(Zone::Library);
    matching_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    matching_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: hand_tag.clone(),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.extend([
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::up_to(1),
            player: chooser,
            tag: hand_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(hand_tag, None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )),
            tag: kept_tag.clone(),
        },
        EffectAst::ChooseTaggedObjectsInZone {
            filter: matching_filter,
            count: choice_count,
            player: matching_chooser,
            tag: matching_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_move_to_zone_with_attack_target(
                TargetAst::Tagged(matching_tag, None),
                destination,
                false,
                controller,
                tapped,
                attacking,
                attack_target_player,
                false,
                None,
            )),
            tag: kept_tag.clone(),
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: looked_tag,
                keep_tagged: kept_tag,
                zone: Zone::Graveyard,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            },
        ),
    ]);
    Ok(Some(effects))
}

pub fn parse_counted_from_looked_cards_action(
    tokens: &[OwnedLexToken],
) -> Option<(
    ChoiceCount,
    ObjectFilter,
    Option<crate::effect::ChoiceAggregateConstraint>,
    Zone,
    ReturnControllerAst,
    bool,
    bool,
    Option<PlayerAst>,
    bool,
)> {
    let action_tokens = trim_commas(tokens);
    let shape = triple_grammar::parse_looked_move_action_shape(&action_tokens)?;
    let choice_filter_tokens = trim_commas(&action_tokens[shape.filter]);
    let mut filter = effect_sentences::parse_looked_card_choice_filter(&choice_filter_tokens)?;
    let aggregate_constraint =
        lift_total_mana_value_choice_constraint(&choice_filter_tokens, &mut filter);
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let (zone, controller, tapped, attacking, attack_target_player) = match shape.destination {
        triple_grammar::LookedMoveDestinationShape::Hand => (
            Zone::Hand,
            ReturnControllerAst::Preserve,
            false,
            false,
            None,
        ),
        triple_grammar::LookedMoveDestinationShape::Battlefield {
            tapped,
            attacking,
            attacks_that_player,
            controller,
        } => (
            Zone::Battlefield,
            match controller {
                Some(BattlefieldControllerShape::You) => ReturnControllerAst::You,
                Some(BattlefieldControllerShape::Owner) => ReturnControllerAst::Owner,
                None => ReturnControllerAst::Preserve,
            },
            tapped,
            attacking,
            attacks_that_player.then_some(PlayerAst::Defending),
        ),
    };

    Some((
        shape.count,
        filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        shape.all_matching,
    ))
}

pub fn parse_top_cards_put_any_matching_to_zone_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_tokens = strip_leading_token_words_any(&first_tokens, &["then"]);
    let (view_tokens, gate_on_previous_result) =
        if let Some(followup) = sentence_markers::parse_conditional_followup_tokens(first_tokens) {
            (trim_commas(followup.tail_tokens), true)
        } else {
            (first_tokens.to_vec(), false)
        };
    let Some((player, count, reveal_top)) = parse_top_cards_view_sentence(&view_tokens) else {
        return Ok(None);
    };
    let remainder_player = match player {
        PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
        player => player,
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let Some((
        mut choice_count,
        filter,
        aggregate_constraint,
        zone,
        controller,
        tapped,
        attacking,
        attack_target_player,
        all_matching,
    )) = parse_counted_from_looked_cards_action(action_match.tail_tokens)
    else {
        return Ok(None);
    };
    if all_matching && action_match.actor != LeadingMayActor::Default {
        return Ok(None);
    }
    if action_match.actor != LeadingMayActor::Default && choice_count == ChoiceCount::exactly(1) {
        choice_count = ChoiceCount::up_to(1);
    }

    let remainder_tokens = sentences[sentence_idx + 2].lexed();
    let remainder_surface = triple_grammar::looked_remainder_surface(remainder_tokens);
    let Some(remainder) = triple_grammar::parse_looked_remainder_shape(remainder_tokens) else {
        return Ok(None);
    };
    let order = match remainder {
        triple_grammar::LookedRemainderShape::LibraryBottom(order) => Some(order),
        triple_grammar::LookedRemainderShape::Graveyard => None,
    };

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lexed(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lexed(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut effects = vec![if reveal_top {
        EffectAst::subject_verb_reveal_top_cards(player, count, looked_tag.clone())
    } else {
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone())
    }];
    if all_matching {
        choose_filter.zone = None;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            choose_filter,
            vec![Zone::Library],
            chosen_tag.clone(),
        ));
    } else {
        effects.push(if let Some(constraint) = aggregate_constraint {
            EffectAst::ChooseObjectsWithAggregateConstraint {
                filter: choose_filter,
                count: choice_count,
                player: chooser,
                tag: chosen_tag.clone(),
                constraint,
            }
        } else {
            EffectAst::ChooseTaggedObjectsInZone {
                filter: choose_filter,
                count: choice_count,
                player: chooser,
                tag: chosen_tag.clone(),
                zone: Zone::Library,
            }
        });
    }
    let mut chosen_effects = vec![EffectAst::subject_verb_move_to_zone_with_attack_target(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
        zone,
        false,
        controller,
        tapped,
        attacking,
        attack_target_player,
        false,
        None,
    )];
    if let Some((amount, counter_type)) =
        triple_grammar::parse_looked_move_action_shape(action_match.tail_tokens)
            .and_then(|shape| shape.entry_counter)
    {
        chosen_effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(amount as i32),
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
            None,
            false,
        ));
    }
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: chosen_effects,
    });
    if let Some(order) = order {
        effects.push(
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library_with_surface(
                looked_tag,
                Some(chosen_tag),
                order,
                remainder_player,
                remainder_surface,
            ),
        );
    } else {
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: looked_tag,
                keep_tagged: chosen_tag,
                zone: Zone::Graveyard,
                surface: remainder_surface,
            },
        ));
    }

    if gate_on_previous_result {
        Ok(Some(vec![EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        }]))
    } else {
        Ok(Some(effects))
    }
}

pub(crate) fn parse_cast_from_among_looked_cards_action(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&sentence_tokens, &["cast"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_cast_action_shape(&action_tokens) else {
        return Ok(None);
    };
    let filter_tokens = trim_commas(&action_tokens[shape.filter]);
    let mentions_spell = shape.mentions_spell;
    let mut filter =
        if let Some(filter) = effect_sentences::parse_looked_card_choice_filter(&filter_tokens) {
            filter
        } else if mentions_spell {
            ObjectFilter::default()
        } else {
            return Ok(None);
        };

    if mentions_spell && filter.card_types.is_empty() {
        filter.excluded_card_types.push(CardType::Land);
    }
    filter.zone = Some(Zone::Library);
    filter.stack_kind = None;
    filter.has_mana_cost = false;
    if filter.mana_value.is_none()
        && let Some(bound) = shape.mana_value_limit
    {
        filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(bound as i32));
    }

    Ok(Some((chooser, filter)))
}

fn target_ast_contains_stack_object(target: &TargetAst) -> bool {
    fn filter_contains_stack_object(filter: &ObjectFilter) -> bool {
        filter.zone == Some(Zone::Stack) || filter.any_of.iter().any(filter_contains_stack_object)
    }

    match target {
        TargetAst::Spell(_) => true,
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            filter_contains_stack_object(filter)
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_ast_contains_stack_object(inner)
        }
        _ => false,
    }
}

#[cfg(test)]
#[path = "ordered_control_flow_inline_hidden_filtered_permission_tests_2.rs"]
mod hidden_filtered_permission_tests;

#[cfg(test)]
#[path = "ordered_control_flow_inline_optional_looked_entry_counter_tests_3.rs"]
mod optional_looked_entry_counter_tests;

#[cfg(test)]
#[path = "ordered_control_flow_inline_explicit_stack_copy_assignment_tests_4.rs"]
mod explicit_stack_copy_assignment_tests;

pub(crate) fn parse_reveal_matching_from_looked_cards_into_hand_action(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<(PlayerAst, ChoiceCount, ObjectFilter, bool)>, CardTextError> {
    let second_tokens = trim_commas(tokens);
    let Some(action_match) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let Some(shape) = triple_grammar::parse_looked_hand_action_shape(&action_tokens, true) else {
        return Ok(None);
    };
    let mut choice_count = shape.count;
    if !matches!(action_match.actor, LeadingMayActor::Default) && choice_count.min > 0 {
        choice_count = ChoiceCount::up_to(choice_count.max.unwrap_or(choice_count.min));
    }
    let filter_tokens = trim_commas(&action_tokens[shape.filter]);
    let mut filter =
        effect_sentences::parse_looked_card_choice_filter(&filter_tokens).ok_or_else(|| {
            CardTextError::ParseError("unable to parse revealed looked-card filter".to_string())
        })?;
    filter.zone = Some(Zone::Library);

    Ok(Some((
        chooser,
        choice_count,
        filter,
        shape.filter_uses_and_or,
    )))
}

pub(crate) fn looked_card_choice_filter_branches(
    filter: &ObjectFilter,
) -> Option<Vec<ObjectFilter>> {
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

#[cfg(test)]
#[path = "ordered_control_flow_inline_tests_5.rs"]
mod tests;

#[path = "ordered_control_flow_programs/ordered_control_flow_trigger.rs"]
mod ordered_control_flow_trigger_programs;
#[path = "ordered_control_flow_programs/ordered_control_flow_combat.rs"]
mod ordered_control_flow_combat_programs;
pub use ordered_control_flow_combat_programs::parse_destroy_historically_blocked_then_reanimate_from_historical_controller;
#[path = "ordered_control_flow_programs/ordered_control_flow_library.rs"]
mod ordered_control_flow_library_programs;
use ordered_control_flow_library_programs::parse_choose_from_looked_cards_for_each_filter;
pub(crate) use ordered_control_flow_library_programs::compose_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom;
pub use ordered_control_flow_library_programs::{
    parse_look_at_top_reveal_match_put_rest_bottom,
    parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard,
    parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom,
    parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom,
    };
#[path = "ordered_control_flow_programs/ordered_control_flow_choice.rs"]
mod ordered_control_flow_choice_programs;
use ordered_control_flow_choice_programs::parse_keyword_choice_filter;

/// Three-sentence counterpart to the looked-card exile/cast quad:
///
/// "Look at ... . Exile up to one <filter> card from among them and put the
/// rest on the bottom ... . You may cast the exiled card ... ."
///
/// The compound middle sentence still lowers to the same typed selection,
/// exile, and complement program as the four-sentence surface.
pub fn parse_look_at_top_exile_match_and_rest_bottom_then_cast_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(shape) = quad_grammar::parse_exile_looked_card_and_remainder_shape(
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };
    if shape.count != ChoiceCount::up_to(1) {
        return Ok(None);
    }
    let Some(mut exile_filter) = parse_looked_card_choice_filter(shape.filter_tokens) else {
        return Ok(None);
    };
    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    // This compound sentence authored an explicit "up to one" selection,
    // unlike the otherwise equivalent four-sentence "you may exile" shape.
    // Retain that surface on the internal result role while keeping the
    // conventional `exiled` prefix used by reference resolution.
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled_up_to");
    let permission_effect = match permission {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player: permission_player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    surface,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_with_optional_surface(
            exiled_tag.clone(),
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            surface,
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::CastTagged {
                    player: permission_player,
                    allow_land,
                    as_copy,
                    without_paying_mana_cost,
                    additional_mana_cost,
                    cost_reduction,
                    mana_spend_mode,
                    ..
                },
            ..
        }) if !as_copy => {
            EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                exiled_tag.clone(),
                permission_player,
                allow_land,
                false,
                without_paying_mana_cost,
                additional_mana_cost,
                cost_reduction,
                mana_spend_mode,
            )
        }
        _ => return Ok(None),
    };

    exile_filter.zone = Some(Zone::Library);
    exile_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: exile_filter,
            count: shape.count,
            player,
            tag: exiled_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag),
            shape.order,
            player,
        ),
        permission_effect,
    ]))
}

/// Preserve the authored optional-action surface for a looked-card selection
/// that enters with a counter. `May { exact-one choice }` is semantically
/// equivalent to an up-to-one choice, but unlike a bare up-to choice it also
/// proves Oracle's "You may put" wording for rendering.
pub fn parse_look_at_top_may_put_with_counter_then_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action) =
        sentence_markers::parse_leading_may_action_tokens(&second_tokens, &["put"], false)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action.actor, player);
    let Some((
        selected_count,
        mut selected_filter,
        None,
        Zone::Battlefield,
        controller,
        tapped,
        false,
        None,
        false,
    )) = parse_counted_from_looked_cards_action(action.tail_tokens)
    else {
        return Ok(None);
    };
    if !selected_count.is_single() {
        return Ok(None);
    }
    let Some((counter_amount, counter_type)) =
        triple_grammar::parse_looked_move_action_shape(action.tail_tokens)
            .and_then(|shape| shape.entry_counter)
    else {
        return Ok(None);
    };
    let Some(triple_grammar::LookedRemainderShape::LibraryBottom(order)) =
        triple_grammar::parse_looked_remainder_shape(sentences[sentence_idx + 2].lowered())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let selected_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "selected");
    selected_filter.zone = Some(Zone::Library);
    selected_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let iterated = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::May {
            effects: vec![
                EffectAst::ChooseTaggedObjectsInZone {
                    filter: selected_filter,
                    count: ChoiceCount::exactly(1),
                    player: chooser,
                    tag: selected_tag.clone(),
                    zone: Zone::Library,
                },
                EffectAst::ForEachTagged {
                    tag: selected_tag.clone(),
                    effects: vec![
                        EffectAst::subject_verb_move_to_zone_with_attack_target(
                            iterated.clone(),
                            Zone::Battlefield,
                            false,
                            controller,
                            tapped,
                            false,
                            None,
                            false,
                            None,
                        ),
                        EffectAst::subject_verb_put_counters(
                            counter_type,
                            Value::Fixed(counter_amount as i32).with_surface_hint(
                                ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter,
                            ),
                            iterated,
                            None,
                            false,
                        ),
                    ],
                },
            ],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            order,
            chooser,
        ),
    ]))
}

/// Preserve the selected card across the three authored sentences in the
/// hidden-card permission shape:
///
/// "Look at ... . Exile one face down and put the rest ... . For as long as
/// it remains exiled, you may cast it if ... ."
///
/// The ordinary two-sentence partition parser already proves the exact
/// looked/selected/remainder relationship. This rule rebinds the final cast
/// permission (and any explicit tagged-look instruction in the equivalent
/// plural grammar) to that proven selected-card tag.
pub fn parse_look_at_top_partition_face_down_then_filtered_permission(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) =
        super::reference_linked_programs::parse_look_at_top_then_partition_selected_and_remainder(
            sentences,
            sentence_idx,
        )?
    else {
        return Ok(None);
    };
    let [look_effect, choice_effect, exile_effect, remainder_effect] = effects.as_slice() else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards {
            tag: looked_tag, ..
        },
        ..
    }) = look_effect
    else {
        return Ok(None);
    };
    let (selected_tag, count, selected_filter, chooser) = match choice_effect {
        EffectAst::ChooseTaggedObjectsInZone {
            tag,
            count,
            filter,
            player,
            zone: Zone::Library,
        }
        | EffectAst::ChooseObjects {
            tag,
            count,
            count_value: None,
            filter,
            player,
        } => (tag, count, filter, player),
        _ => return Ok(None),
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
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag: remainder_tag,
                keep_tagged: Some(kept_tag),
                order: crate::cards::builders::LibraryBottomOrderAst::ChooserChooses,
                ..
            },
        ..
    }) = remainder_effect
    else {
        return Ok(None);
    };
    let expected_selected_filter = ObjectFilter::tagged(looked_tag.clone()).in_zone(Zone::Library);
    if !count.is_single()
        || chooser != &PlayerAst::You
        || selected_filter != &expected_selected_filter
        || exile_tag != selected_tag
        || remainder_tag != looked_tag
        || kept_tag != selected_tag
    {
        return Ok(None);
    }
    let selected_tag = selected_tag.clone();

    let permission_tokens = sentences[sentence_idx + 2].lexed();
    let permission_words = crate::lexer::parser_token_word_refs(permission_tokens);
    if crate::grammar::primitives::parse_word_sequence_prefix(
        &permission_words,
        &["until", "end", "of", "turn"],
    )
    .is_some()
    {
        return Ok(None);
    }
    if crate::word_primitives::parse_any_sequence_complete(
        &permission_words,
        &[
            &[
                "for", "as", "long", "as", "it", "remains", "exiled", "you", "may", "cast", "it",
                "if", "its", "a", "creature", "spell",
            ],
            &[
                "for", "as", "long", "as", "it", "remains", "exiled", "you", "may", "cast", "it",
                "if", "it", "s", "a", "creature", "spell",
            ],
            &[
                "for", "as", "long", "as", "it", "remains", "exiled", "you", "may", "cast", "it",
                "if", "it", "is", "a", "creature", "spell",
            ],
        ],
    ) {
        effects.push(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                selected_tag,
                PlayerAst::You,
                false,
                false,
                false,
                Some(ObjectFilter::creature()),
            ),
        );
        return Ok(Some(effects));
    }

    let Some(permission) = parse_cast_or_play_tagged_clause(permission_tokens)? else {
        return Ok(None);
    };
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
                during_turns_counter_put_on_source: None,
                spell_cost_increase: None,
                lands_enter_tapped: false,
                ..
            },
        ..
    }) = &permission
    {
        effects.push(
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                selected_tag,
                *player,
                *allow_land,
                *without_paying_mana_cost,
                *allow_any_color_for_cast,
                filter.clone(),
            ),
        );
        return Ok(Some(effects));
    }
    let EffectAst::Sequence {
        effects: permission_effects,
    } = permission
    else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                crate::cards::builders::SubjectVerbSubjectAst {
                    player: PlayerAst::You,
                    ..
                },
            action:
                SubjectVerbActionAst::LookAtObjects {
                    filter: look_filter,
                },
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    filter,
                    during_turns_counter_put_on_source: None,
                    spell_cost_increase: None,
                    lands_enter_tapped: false,
                    ..
                },
            ..
        }),
    ] = permission_effects.as_slice()
    else {
        return Ok(None);
    };
    if look_filter.zone != Some(Zone::Exile)
        || look_filter.tagged_constraints.len() != 1
        || !look_filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
    {
        return Ok(None);
    }

    let mut rebound_look_filter = look_filter.clone();
    rebound_look_filter.tagged_constraints[0].tag = selected_tag.clone();
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::You,
        SubjectVerbActionAst::LookAtObjects {
            filter: rebound_look_filter,
        },
    ));
    effects.push(
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            selected_tag,
            *player,
            *allow_land,
            *without_paying_mana_cost,
            *allow_any_color_for_cast,
            filter.clone(),
        ),
    );
    Ok(Some(effects))
}

/// Preserve a conditional `instead` arm together with the common sentence
/// that follows both outcomes:
///
/// `Target ... gets ... . Put a counter on it instead if ... . Then it deals ... .`
///
/// Parsing the three sentences independently loses the replacement sentence,
/// while attaching the final damage only to the nearest arm changes runtime
/// behavior. The exact typed shapes below prove one default modifier, one
/// conditional counter replacement, and one common damage continuation.
pub fn parse_target_modifier_counter_instead_then_common_damage(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let default_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())?;
    let replacement_sentence = sentences[sentence_idx + 1].lowered();
    let replacement_effects = effect_sentences::parse_effect_sentence_lexed(replacement_sentence)?;
    let common_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 2].lowered())?;

    let [
        default @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Pump { .. },
            ..
        }),
    ] = default_effects.as_slice()
    else {
        return Ok(None);
    };
    let parsed_replacement = match replacement_effects.as_slice() {
        [EffectAst::TrailingIf { predicate, effects }] => {
            Some((predicate.clone(), effects.clone()))
        }
        [EffectAst::ControlFlow(control)] => {
            let crate::model::ControlFlowNodeAst::Condition {
                condition,
                consequence_program,
                alternative_program: None,
                ..
            } = &control.node
            else {
                return Ok(None);
            };
            if condition.position != crate::model::ConditionPositionAst::Postcondition {
                return Ok(None);
            }
            let crate::model::ControlPredicateAst::State(predicate) = &condition.predicate else {
                return Ok(None);
            };
            let Some(program) = control.programs.get(*consequence_program) else {
                return Ok(None);
            };
            Some((predicate.clone(), program.effects.clone()))
        }
        _ => {
            // `instead if` changes the relationship between this sentence and
            // the preceding one; it is not part of the counter action itself.
            // Parse both owned clauses explicitly when the ordinary standalone
            // sentence route correctly declines that cross-sentence shape.
            let view = TokenWordView::new(replacement_sentence);
            let Some(instead_word) =
                crate::slice_primitives::select_position(&view.word_refs(), |word| {
                    *word == "instead"
                })
            else {
                return Ok(None);
            };
            let Some(instead_token) = view.map_word_to_token_start(instead_word) else {
                return Ok(None);
            };
            let action = effect_sentences::parse_effect_sentence_lexed(
                &replacement_sentence[..instead_token],
            )?;
            let Some(predicate) =
                crate::grammar::structure::parse_trailing_instead_if_predicate_lexed(
                    &replacement_sentence[instead_token..],
                )
            else {
                return Ok(None);
            };
            Some((predicate, action))
        }
    };
    let Some((predicate, replacement)) = parsed_replacement else {
        return Ok(None);
    };
    if !matches!(
        replacement.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters { .. },
            ..
        })]
    ) || !sentences[sentence_idx + 1]
        .lowered()
        .iter()
        .any(|token| token.is_word("instead"))
    {
        return Ok(None);
    }
    if !matches!(
        common_effects.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DealDamageEqualToPower { .. },
            ..
        })]
    ) {
        return Ok(None);
    }

    let predicate = match predicate {
        PredicateAst::ItMatches(filter) => PredicateAst::TargetMatches(filter),
        PredicateAst::TargetMatches(filter) => PredicateAst::TargetMatches(filter),
        _ => return Ok(None),
    };
    let mut if_true = replacement;
    if_true.extend(common_effects.clone());
    let mut if_false = vec![default.clone()];
    if_false.extend(common_effects);
    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate,
        if_true,
        if_false,
        attach_to_previous_ability: false,
    }]))
}

use super::super::super::dispatch_entry::{
    ConsultSentenceParts, consult_cast_effects, consult_stop_rule_is_single_match,
    leading_may_actor_to_player, parse_consult_cast_clause, parse_consult_traversal_sentence,
    parse_looked_card_choice_filter, parse_looked_card_reveal_filter,
    parse_top_cards_view_sentence, target_references_it,
};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, GrantedAbilityAst, IfResultPredicate,
    LibraryBottomOrderAst, ObjectFilter, OwnedLexToken, PlayerAst, PredicateAst,
    ReturnControllerAst, SubjectAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan, TriggerSpec,
    ZoneReplacementDurationAst,
};
use crate::effect::{EffectPredicate, Value};
use crate::effect_sentences;
use crate::effect_sentences::SentenceInput;
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::grammar::effects::{
    self as effect_grammar, parse_reciprocal_creature_control_sequence_tokens,
};
use crate::grammar::sentence_markers::{self, ConditionalFollowupActor, LeadingMayActor};
use crate::grammar::structure::{
    LeadingResultPrefixKind, parse_predicate_with_grammar_entrypoint_lexed,
    split_leading_result_prefix_lexed,
};
use crate::lexer::LexedClause;
use crate::model::CompilerStaticAbilityCore as StaticAbility;
use crate::object_filters::parse_object_filter_lexed;
use crate::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::target::{ChooseSpec, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::util::trim_commas;
use crate::util::{helper_tag_for_tokens, parse_subject};
use crate::zone::Zone;

pub(crate) fn target_opponent_filter(player: &PlayerFilter) -> bool {
    matches!(
        player,
        PlayerFilter::Target(inner)
            if matches!(inner.as_ref(), PlayerFilter::Opponent)
                || target_opponent_filter(inner)
    )
}

pub(crate) fn tagged_subset_destroy_words(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::lexer::token_word_refs(tokens);
    crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[
            &["destroy", "any", "of", "them", "that", "are"],
            &["destroy", "any", "of", "those", "creatures", "that", "are"],
            &["destroy", "any", "of", "those", "permanents", "that", "are"],
        ],
    )
}

/// Bind the two independent antecedents in the authored draw/reveal pump
/// family. The revealed-card reference feeds every `X`, while "the creature"
/// still names the triggering creature rather than the newly drawn card.
pub fn parse_draw_reveal_then_triggering_creature_mana_value_result(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(second) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    if !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(first.lowered()),
        &["draw", "a", "card", "and", "reveal", "it"],
    ) || !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(second.lowered()),
        &[
            "the", "creature", "gets", "+x/+x", "until", "end", "of", "turn", "and", "you", "lose",
            "x", "life", "where", "x", "is", "that", "cards", "mana", "value",
        ],
    ) {
        return Ok(None);
    }

    let drawn_tag = crate::tag::CompilerReferenceTag::DrawnRevealedCard.bind();
    let triggering_tag = crate::tag::CompilerReferenceTag::Triggering.bind();
    let mana_value = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(drawn_tag.clone())))
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);
    let draw = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::Draw {
            count: Value::Fixed(1),
        },
    );
    let pump = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::Pump {
            power: mana_value.clone(),
            toughness: mana_value.clone(),
            target: TargetAst::Tagged(triggering_tag, None),
            duration: crate::effect::Until::EndOfTurn,
            condition: None,
            set_quantifier_surface: None,
        },
    );
    let lose_life = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LoseLife { amount: mana_value },
    );

    Ok(Some(vec![
        EffectAst::SourceSentence {
            effects: vec![EffectAst::Coordinated {
                effects: vec![
                    EffectAst::TagAffected {
                        effect: Box::new(draw),
                        tag: drawn_tag.clone(),
                    },
                    EffectAst::subject_verb_reveal_tagged(drawn_tag),
                ],
                leading_duration: false,
                result_conjunction: false,
            }],
            leading_then: false,
            starting_with_controller: false,
        },
        EffectAst::SourceSentence {
            effects: vec![EffectAst::Coordinated {
                effects: vec![pump, lose_life],
                leading_duration: false,
                result_conjunction: false,
            }],
            leading_then: false,
            starting_with_controller: false,
        },
    ]))
}

#[cfg(test)]
#[path = "reference_linked_inline_draw_reveal_mana_value_result_tests.rs"]
mod draw_reveal_mana_value_result_tests;

pub(crate) fn counted_target_object_filter(target: &TargetAst) -> Option<&ObjectFilter> {
    let TargetAst::WithCount(inner, count) = target else {
        return None;
    };
    if count.is_random() || count.max.is_some_and(|max| max < 2) {
        return None;
    }
    let TargetAst::Object(filter, _, _) = inner.as_ref() else {
        return None;
    };
    Some(filter)
}

#[cfg(test)]
#[path = "reference_linked_inline_target_opponent_copy_triggering_spell_tests_2.rs"]
mod target_opponent_copy_triggering_spell_tests;

#[cfg(test)]
#[path = "reference_linked_inline_copy_next_spell_when_cast_tests_3.rs"]
mod copy_next_spell_when_cast_tests;

/// Preserve a simultaneous participant loot and gate a follow-up on whether
/// one participant's affected object tied for the greatest mana value among
/// every object affected by the shared discard action.
///
/// The nested `Excluding` expression is the exact set union `{you, defending
/// player}`: remove every non-you player except the defending player from the
/// full player set. This keeps the execution on the generic simultaneous
/// `ForPlayersEffect` path without inventing a bespoke participant filter.
pub fn parse_controller_defending_loot_then_greatest_mana_value_followup(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let first_words = crate::lexer::token_word_refs(first_tokens);
    if !crate::word_primitives::parse_any_sequence_complete(
        &first_words,
        &[
            &[
                "you",
                "and",
                "defending",
                "player",
                "each",
                "draw",
                "a",
                "card",
                "then",
                "discard",
                "a",
                "card",
            ],
            &[
                "you",
                "and",
                "the",
                "defending",
                "player",
                "each",
                "draw",
                "a",
                "card",
                "then",
                "discard",
                "a",
                "card",
            ],
        ],
    ) {
        return Ok(None);
    }

    let second_tokens = sentences[sentence_idx + 1].lowered();
    let second_view = crate::lexer::TokenWordView::new(second_tokens);
    let Some(if_word) = second_view.parse_word_position("if") else {
        return Ok(None);
    };
    let Some(if_idx) = second_view.map_word_to_token_start(if_word) else {
        return Ok(None);
    };
    if !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(&second_tokens[if_idx..]),
        &[
            "if",
            "you",
            "discarded",
            "the",
            "card",
            "with",
            "the",
            "greatest",
            "mana",
            "value",
            "among",
            "those",
            "cards",
            "or",
            "tied",
            "for",
            "greatest",
        ],
    ) {
        return Ok(None);
    }

    let followup_tokens = trim_commas(&second_tokens[..if_idx]);
    let followup = effect_sentences::parse_effect_sentence_lexed(&followup_tokens)?;
    if !matches!(
        followup.as_slice(),
        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PutCounters {
                target: TargetAst::Source(_),
                target_count: None,
                distributed: false,
                ..
            },
            ..
        })]
    ) {
        return Ok(None);
    }

    let participants = PlayerFilter::excluding(
        PlayerFilter::Any,
        PlayerFilter::excluding(PlayerFilter::NotYou, PlayerFilter::Defending),
    );
    let loot = EffectAst::ForEachPlayersFiltered {
        filter: participants,
        effects: vec![
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::That,
                SubjectVerbActionAst::Draw {
                    count: Value::Fixed(1),
                },
            ),
            EffectAst::subject_verb_discard(
                PlayerAst::That,
                Value::Fixed(1),
                false,
                false,
                None,
                None,
            ),
        ],
    };
    Ok(Some(vec![EffectAst::IfEffectResult {
        effect: Box::new(loot),
        predicate: EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
            player: PlayerFilter::You,
        },
        if_true: followup,
    }]))
}

#[cfg(test)]
#[path = "reference_linked_inline_participant_loot_extremum_tests_4.rs"]
mod participant_loot_extremum_tests;

#[cfg(test)]
#[path = "reference_linked_inline_tagged_target_subset_tests_5.rs"]
mod tagged_target_subset_tests;

#[cfg(test)]
#[path = "reference_linked_inline_destroy_search_partition_tests_6.rs"]
mod destroy_search_partition_tests;

#[cfg(test)]
#[path = "reference_linked_inline_resolving_card_exile_tests_7.rs"]
mod resolving_card_exile_tests;

#[cfg(test)]
#[path = "reference_linked_inline_counter_destination_replacement_tests_8.rs"]
mod counter_destination_replacement_tests;

#[cfg(test)]
#[path = "reference_linked_inline_revealed_hand_graveyard_disjunction_tests_9.rs"]
mod revealed_hand_graveyard_disjunction_tests;

#[cfg(test)]
#[path = "reference_linked_inline_looked_hand_optional_cast_tests_10.rs"]
mod looked_hand_optional_cast_tests;

#[cfg(test)]
#[path = "reference_linked_inline_revealed_hand_optional_cast_tests_11.rs"]
mod revealed_hand_optional_cast_tests;

#[cfg(test)]
#[path = "reference_linked_inline_revealed_hand_union_count_tests_12.rs"]
mod revealed_hand_union_count_tests;

pub fn parse_participant_secret_object_choice_then_reveal_and_sacrifice(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let first_words = crate::lexer::token_word_refs(first_tokens);
    let Some(choose_idx) = crate::word_primitives::parse_sequence_start(&first_words, &["choose"])
    else {
        return Ok(None);
    };
    if !first_words.get(..choose_idx).is_some_and(|prefix| {
        crate::word_primitives::parse_sequence_complete(
            prefix,
            &["you", "and", "target", "opponent", "each", "secretly"],
        )
    }) {
        return Ok(None);
    }
    let first_view = crate::lexer::TokenWordView::new(first_tokens);
    let Some(choose_token_idx) = first_view.map_word_to_token_start(choose_idx) else {
        return Ok(None);
    };
    let object_tokens = trim_commas(&first_tokens[choose_token_idx + 1..]);
    if object_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter_lexed(&object_tokens, false)?;
    let object_words = crate::lexer::token_word_refs(&object_tokens);
    if !crate::word_primitives::sequence_occurs(&object_words, &["that", "player", "controls"]) {
        return Ok(None);
    }
    filter.controller = Some(PlayerFilter::IteratedPlayer);
    filter.zone.get_or_insert(Zone::Battlefield);

    let second_tokens = sentences[sentence_idx + 1].lowered();
    let second_words = crate::lexer::token_word_refs(second_tokens);
    let sacrifice_idx =
        crate::word_primitives::parse_sequence_start(&second_words, &["player", "sacrifices"])
            .map(|idx| idx + 1);
    let Some(sacrifice_idx) = sacrifice_idx else {
        return Ok(None);
    };
    if !second_words.get(..sacrifice_idx).is_some_and(|prefix| {
        crate::word_primitives::parse_sequence_complete(
            prefix,
            &[
                "then", "those", "choices", "are", "revealed", "and", "that", "player",
            ],
        )
    }) || second_words.get(sacrifice_idx) != Some(&"sacrifices")
        || second_words.get(sacrifice_idx + 1) != Some(&"those")
    {
        return Ok(None);
    }

    let tag = helper_tag_for_tokens(first_tokens, "secret_choices");
    let mut sacrifice_filter = filter.clone();
    sacrifice_filter.controller = None;
    sacrifice_filter.zone = None;
    let object_choice = crate::effects::SecretObjectChoice {
        filter,
        count: ChoiceCount::exactly(1),
        tag: tag.clone(),
        reveal_after_choice: true,
    };

    Ok(Some(vec![
        EffectAst::SecretChoiceStart {
            options: Vec::new(),
            participants: vec![PlayerFilter::You, PlayerFilter::target_opponent()],
            object_choice: Some(object_choice),
        },
        EffectAst::ForEachTagged {
            tag,
            effects: vec![EffectAst::subject_verb_sacrifice(
                PlayerAst::ItsController,
                sacrifice_filter,
                1,
                Some(TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    None,
                )),
            )],
        },
    ]))
}

#[cfg(test)]
#[path = "reference_linked_inline_secret_object_choice_tests_13.rs"]
mod secret_object_choice_tests;

fn look_at_top_cards_parts(effect: &EffectAst) -> Option<(PlayerAst, Value)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone()))
}

fn top_cards_parts_with_reveal(effect: &EffectAst) -> Option<(PlayerAst, Value, bool)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::LookAtTopCards { count, reveal, .. },
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone(), *reveal))
}

pub(crate) fn tagged_library_candidate_filter(
    candidate: &TagKey,
    excluded: &[TagKey],
) -> ObjectFilter {
    let mut filter = ObjectFilter::tagged(candidate.clone()).in_zone(Zone::Library);
    for tag in excluded {
        filter = filter.not_tagged(tag.clone());
    }
    filter
}

pub(crate) fn looked_library_owner_filter(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You => Some(PlayerFilter::You),
        PlayerAst::Target => Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any))),
        PlayerAst::TargetOpponent => Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent,
        ))),
        _ => None,
    }
}

pub(crate) fn move_tagged_to_looked_destination(
    tag: TagKey,
    destination: effect_grammar::LookedCardDestinationShape,
) -> EffectAst {
    let (zone, to_top) = match destination {
        effect_grammar::LookedCardDestinationShape::Hand => (Zone::Hand, false),
        effect_grammar::LookedCardDestinationShape::Graveyard => (Zone::Graveyard, false),
        effect_grammar::LookedCardDestinationShape::Battlefield => (Zone::Battlefield, false),
        effect_grammar::LookedCardDestinationShape::LibraryTop => (Zone::Library, true),
        effect_grammar::LookedCardDestinationShape::LibraryBottom => (Zone::Library, false),
    };
    EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(tag, None),
        zone,
        to_top,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
}

pub(crate) fn move_looked_partition_group(
    tag: TagKey,
    destination: effect_grammar::LookedPartitionDestination,
    library_owner: PlayerAst,
) -> EffectAst {
    let (zone, to_top, order) = match destination {
        effect_grammar::LookedPartitionDestination::Hand => (Zone::Hand, false, None),
        effect_grammar::LookedPartitionDestination::Graveyard => (Zone::Graveyard, false, None),
        effect_grammar::LookedPartitionDestination::LibraryTop(order) => {
            (Zone::Library, true, Some(order))
        }
        effect_grammar::LookedPartitionDestination::LibraryBottom(order) => {
            (Zone::Library, false, Some(order))
        }
    };
    let mut effect = EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(tag, None),
        zone,
        to_top,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
    .with_library_order(order, PlayerAst::You);

    if matches!(zone, Zone::Hand | Zone::Graveyard) {
        if library_owner == PlayerAst::You {
            effect = effect.with_destination_player_surface(Some(PlayerAst::You));
        } else {
            effect = effect.with_destination_player_reference_surface(Some(
                ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer,
            ));
        }
    }
    effect
}

fn compose_singleton_hand_partition(
    look_tokens: &[OwnedLexToken],
    partition_tokens: &[OwnedLexToken],
    player: PlayerAst,
    count: Value,
    remainder_destination: effect_grammar::LookedPartitionDestination,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "looked");
    let hand_tag = helper_tag_for_tokens(partition_tokens, "hand");
    let remainder_tag = helper_tag_for_tokens(partition_tokens, "remainder");
    let hand_filter = tagged_library_candidate_filter(&looked_tag, &[]);
    let remainder_filter =
        tagged_library_candidate_filter(&looked_tag, std::slice::from_ref(&hand_tag));

    vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: hand_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_tag_matching_objects(
            remainder_filter,
            vec![Zone::Library],
            remainder_tag.clone(),
        ),
        move_looked_partition_group(
            hand_tag,
            effect_grammar::LookedPartitionDestination::Hand,
            player,
        ),
        move_looked_partition_group(remainder_tag, remainder_destination, player),
    ]
}

pub fn parse_inline_look_at_top_then_singleton_hand_partition(
    look_tokens: &[OwnedLexToken],
    partition_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let look_tokens = trim_commas(look_tokens);
    let partition_tokens = trim_commas(partition_tokens);
    let (player, count, reveal_top) = parse_top_cards_view_sentence(&look_tokens)?;
    if reveal_top {
        return None;
    }
    let remainder_destination =
        match effect_grammar::parse_looked_card_disposition(&partition_tokens)? {
            effect_grammar::LookedCardDisposition::HandAndLibraryBottom(order) => {
                effect_grammar::LookedPartitionDestination::LibraryBottom(order)
            }
            effect_grammar::LookedCardDisposition::HandAndGraveyard => {
                effect_grammar::LookedPartitionDestination::Graveyard
            }
        };
    Some(compose_singleton_hand_partition(
        &look_tokens,
        &partition_tokens,
        player,
        count,
        remainder_destination,
    ))
}

fn compose_distinct_three_way_looked_disposition(
    first_tokens: &[OwnedLexToken],
    second_tokens: &[OwnedLexToken],
    player: PlayerAst,
    count: Value,
    destinations: [effect_grammar::LookedCardDestinationShape; 3],
) -> Vec<EffectAst> {
    let candidate_tag = helper_tag_for_tokens(first_tokens, "looked_candidates");
    let chosen_tags = [
        helper_tag_for_tokens(second_tokens, "looked_choice_0"),
        helper_tag_for_tokens(second_tokens, "looked_choice_1"),
        helper_tag_for_tokens(second_tokens, "looked_choice_2"),
    ];
    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        candidate_tag.clone(),
    )];
    for (index, tag) in chosen_tags.iter().enumerate() {
        effects.push(EffectAst::ChooseTaggedObjectsInZone {
            filter: tagged_library_candidate_filter(&candidate_tag, &chosen_tags[..index]),
            count: ChoiceCount::exactly(1),
            player,
            tag: tag.clone(),
            zone: Zone::Library,
        });
    }
    for (tag, destination) in chosen_tags.into_iter().zip(destinations) {
        effects.push(move_tagged_to_looked_destination(tag, destination));
    }
    effects
}

pub fn parse_directional_adjacent_player_control(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let choice_sentence = sentences[sentence_idx].lowered();
    let gain_sentence = sentences[sentence_idx + 1].lowered();

    let Some(shape) = effect_grammar::parse_directional_adjacent_player_control_shape(
        choice_sentence,
        gain_sentence,
    ) else {
        return Ok(None);
    };
    let object_tokens = trim_commas(&choice_sentence[shape.choice_object]);
    let filter = parse_object_filter_lexed(&object_tokens, false)?;

    Ok(Some(vec![EffectAst::DirectionalAdjacentPlayerControl {
        filter,
        left_option: "left".to_string(),
        right_option: "right".to_string(),
    }]))
}

pub fn parse_reciprocal_creature_control_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = parse_reciprocal_creature_control_sequence_tokens(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
        sentences[sentence_idx + 2].lowered(),
    )?
    else {
        return Ok(None);
    };

    let your_tag = crate::tag::CompilerReferenceTag::TwistYourCreatures.bind();
    let target_tag = crate::tag::CompilerReferenceTag::TwistOpponentCreatures.bind();
    let your_tagged = ObjectFilter::tagged(your_tag.clone());
    let target_tagged = ObjectFilter::tagged(target_tag.clone());
    let mut both_tagged = ObjectFilter::default();
    both_tagged.any_of = vec![your_tagged.clone(), target_tagged.clone()];

    let mut effects = vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects {
                filter: shape.your_creatures,
                zones: vec![Zone::Battlefield],
                tag: your_tag,
                source_tags: Vec::new(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::TagMatchingObjects {
                filter: shape.target_player_creatures,
                zones: vec![Zone::Battlefield],
                tag: target_tag,
                source_tags: Vec::new(),
            },
        ),
    ];
    if shape.untap && shape.untap_before_control {
        effects.push(EffectAst::subject_verb_untap_all(both_tagged.clone()));
    }
    effects.extend([
        EffectAst::subject_verb_gain_control(
            PlayerAst::Implicit,
            TargetAst::Object(target_tagged, None, None),
            shape.duration.clone(),
        ),
        EffectAst::subject_verb_gain_control(
            PlayerAst::TargetOpponent,
            TargetAst::Object(your_tagged, None, None),
            shape.duration.clone(),
        ),
    ]);
    if shape.untap && !shape.untap_before_control {
        effects.push(EffectAst::subject_verb_untap_all(both_tagged.clone()));
    }
    if shape.grant_haste {
        let mut haste = EffectAst::subject_verb_grant_abilities_all(
            both_tagged,
            vec![GrantedAbilityAst::KeywordAction(Box::new(
                crate::payload::KeywordAction::Haste,
            ))],
            shape.duration,
        );
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesAll {
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = &mut haste
        {
            *set_quantifier_surface = Some(ironsmith_core::SetQuantifierSurface::Each);
        }
        effects.push(haste);
    }

    Ok(Some(effects))
}

pub(crate) fn parse_optional_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ConsultSentenceParts, bool)>, CardTextError> {
    if let Some(shape) = effect_grammar::parse_optional_sequence_prefix_shape(tokens) {
        let stripped = trim_commas(&tokens[shape.tail]);
        return parse_consult_traversal_sentence(&stripped)
            .map(|parts| parts.map(|parts| (parts, true)));
    }
    parse_consult_traversal_sentence(tokens).map(|parts| parts.map(|parts| (parts, false)))
}

pub(crate) fn parse_gated_optional_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ConsultSentenceParts, bool, bool)>, CardTextError> {
    if let Some(followup) = sentence_markers::parse_conditional_followup_tokens(tokens) {
        if followup.actor != ConditionalFollowupActor::You {
            return Ok(None);
        }
        let stripped = trim_commas(followup.tail_tokens);
        return parse_optional_consult_traversal_sentence(&stripped)
            .map(|parts| parts.map(|(parts, optional)| (parts, optional, true)));
    }
    parse_optional_consult_traversal_sentence(tokens)
        .map(|parts| parts.map(|(parts, optional)| (parts, optional, false)))
}

pub(crate) fn strip_leading_if_you_do_sentence(
    tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool) {
    let Some(followup) = sentence_markers::parse_conditional_followup_tokens(tokens) else {
        return (trim_commas(tokens), false);
    };
    if followup.actor != ConditionalFollowupActor::You {
        return (trim_commas(tokens), false);
    }
    (trim_commas(followup.tail_tokens), true)
}

pub(crate) fn wrap_optional_consult_effects(
    parts: ConsultSentenceParts,
    optional: bool,
    followups: Vec<EffectAst>,
    gate_on_result: bool,
    gate_on_previous_result: bool,
) -> Vec<EffectAst> {
    let mut effects = Vec::new();
    if optional {
        effects.push(EffectAst::May {
            effects: parts.effects,
        });
    } else {
        effects.extend(parts.effects);
    }
    if gate_on_result || optional {
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followups,
        });
    } else {
        effects.extend(followups);
    }
    if gate_on_previous_result {
        vec![EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        }]
    } else {
        effects
    }
}

#[cfg(test)]
#[path = "reference_linked_inline_optional_consult_gating_tests_14.rs"]
mod optional_consult_gating_tests;

fn mark_target_set_same_controller(target: TargetAst) -> TargetAst {
    match target {
        TargetAst::Object(mut filter, target_span, it_span) => {
            filter.target_set_same_controller = true;
            TargetAst::Object(filter, target_span, it_span)
        }
        TargetAst::WithCount(inner, count) => {
            TargetAst::WithCount(Box::new(mark_target_set_same_controller(*inner)), count)
        }
        TargetAst::WithCountValue(inner, count, value) => TargetAst::WithCountValue(
            Box::new(mark_target_set_same_controller(*inner)),
            count,
            value,
        ),
        other => other,
    }
}

// These tags are stable semantic identities used by the reciprocal-control
// model. Keep their established names so compiled definitions remain stable
// across parser migrations.
pub fn parse_exile_face_down_pile_then_cloak(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let Some(shape) = effect_grammar::parse_cloak_pile_sequence_shape(
        first_tokens,
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    let target = effect_sentences::parse_target_phrase(shape.target_tokens)?;
    let pile_tag = helper_tag_for_tokens(first_tokens, "cloak_pile");
    let target_exile = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_exile(target, true)),
        tag: pile_tag.clone(),
    };

    Ok(Some(vec![
        target_exile,
        EffectAst::subject_verb_exile_top_of_library_face_down(
            shape.library_owner,
            shape.library_count,
            pile_tag.clone(),
        ),
        EffectAst::subject_verb_cloak_onto_battlefield(
            PlayerAst::You,
            TargetAst::Tagged(pile_tag, None),
            shape.enters_tapped,
            ReturnControllerAst::You,
            true,
        ),
    ]))
}

pub fn parse_look_at_top_then_put_one_hand_other_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    if let Some(shape) =
        effect_grammar::parse_three_way_looked_card_disposition_shape(&second_tokens)
    {
        if count != Value::Fixed(3)
            || shape != effect_grammar::ThreeWayLookedCardDispositionShape::HandTopBottom
        {
            return Ok(None);
        }
        return Ok(Some(compose_distinct_three_way_looked_disposition(
            sentences[sentence_idx].lowered(),
            &second_tokens,
            player,
            count,
            shape.destinations(),
        )));
    }
    let Some(effect_grammar::LookedCardDisposition::HandAndLibraryBottom(bottom_order)) =
        effect_grammar::parse_looked_card_disposition(&second_tokens)
    else {
        return Ok(None);
    };

    Ok(Some(compose_singleton_hand_partition(
        sentences[sentence_idx].lowered(),
        &second_tokens,
        player,
        count,
        effect_grammar::LookedPartitionDestination::LibraryBottom(bottom_order),
    )))
}

/// Preserves a direct counted selection and its exact looked-card complement:
///
/// "Look at the top N cards ... . Put M of those cards into your hand and the
/// rest on the bottom ... ."
///
/// The standalone `put` parser can recover the prior collection through a
/// lowering-time snapshot. This sequence rule instead gives the look, choice,
/// move, and remainder one shared pair of tags up front, which makes the
/// selected subset and complement explicit in the parser AST.
pub fn parse_look_at_top_then_put_counted_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((PlayerAst::You, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(shape) = effect_grammar::parse_counted_looked_hand_remainder_shape(
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked_partition");
    let selected_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "partition_selected");
    let selected_filter = tagged_library_candidate_filter(&looked_tag, &[]);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(PlayerAst::You, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: shape.count,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::ForEachTagged {
            tag: selected_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(selected_tag),
            shape.remainder_order,
            PlayerAst::You,
        ),
    ]))
}

pub fn parse_look_at_top_then_partition_selected_and_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // The counted face-down exile parser owns the same two-sentence looked
    // pool, but its selected destination is exile rather than one of the
    // ordinary partition destinations below. Route the complete pair through
    // it before the narrower hand/graveyard/library partition grammar.
    let first = sentences[sentence_idx].lowered();
    let (view_tokens, gate_on_previous_result) =
        if let Some(followup) = sentence_markers::parse_conditional_followup_tokens(first) {
            if followup.actor != ConditionalFollowupActor::You {
                return Ok(None);
            }
            (trim_commas(followup.tail_tokens), true)
        } else {
            (first.to_vec(), false)
        };
    if super::super::super::dispatch_inner::parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(first)
        .is_some()
    {
        // The first sentence already owns the complete looked-card partition.
        // Do not let this two-sentence partition rule consume an unrelated
        // provenance-linked followup such as a cast permission; the dedicated
        // look/exile/permission rule must see that second sentence so it can
        // preserve the permission's duration and mana-spend mode.
        return Ok(None);
    }

    let mut combined = first.to_vec();
    combined.extend_from_slice(sentences[sentence_idx + 1].lowered());
    if let Some(effects) = super::super::super::dispatch_inner::parse_generic_top_cards_exile_counted_face_down_rest_bottom_subject_verb(&combined)
    {
        return Ok(Some(effects));
    }

    let Some((library_owner, count, false)) = parse_top_cards_view_sentence(&view_tokens) else {
        return Ok(None);
    };
    let Some(shape) = effect_grammar::parse_looked_card_partition_shape(&trim_commas(
        sentences[sentence_idx + 1].lowered(),
    )) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked_partition");
    let selected_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "partition_selected");
    let selected_filter = tagged_library_candidate_filter(&looked_tag, &[]);

    if let (
        effect_grammar::LookedPartitionDestination::LibraryTop(selected_order),
        effect_grammar::LookedPartitionDestination::LibraryBottom(remainder_order),
    ) = (shape.selected_destination, shape.remainder_destination)
    {
        let effects = vec![
            EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
            EffectAst::ChooseTaggedObjectsInZone {
                filter: selected_filter,
                count: shape.selected_count,
                player: PlayerAst::You,
                tag: selected_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::ForEachTagged {
                tag: selected_tag.clone(),
                effects: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                        Zone::Library,
                        true,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )
                    .with_library_order(Some(selected_order), PlayerAst::You),
                ],
            },
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag,
                Some(selected_tag),
                remainder_order,
                library_owner,
            ),
        ];
        return if gate_on_previous_result {
            Ok(Some(vec![EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects,
            }]))
        } else {
            Ok(Some(effects))
        };
    }

    let remainder_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "partition_remainder");
    let remainder_filter =
        tagged_library_candidate_filter(&looked_tag, std::slice::from_ref(&selected_tag));

    let effects = vec![
        EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: shape.selected_count,
            player: PlayerAst::You,
            tag: selected_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_tag_matching_objects(
            remainder_filter,
            vec![Zone::Library],
            remainder_tag.clone(),
        ),
        move_looked_partition_group(selected_tag, shape.selected_destination, library_owner),
        move_looked_partition_group(remainder_tag, shape.remainder_destination, library_owner),
    ];
    if gate_on_previous_result {
        Ok(Some(vec![EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        }]))
    } else {
        Ok(Some(effects))
    }
}

pub fn parse_look_at_top_then_put_one_hand_other_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if reveal_top {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    if let Some(shape) =
        effect_grammar::parse_three_way_looked_card_disposition_shape(&second_tokens)
    {
        if count != Value::Fixed(3)
            || shape != effect_grammar::ThreeWayLookedCardDispositionShape::HandGraveyardBottom
        {
            return Ok(None);
        }
        return Ok(Some(compose_distinct_three_way_looked_disposition(
            sentences[sentence_idx].lowered(),
            &second_tokens,
            player,
            count,
            shape.destinations(),
        )));
    }
    if effect_grammar::parse_looked_card_disposition(&second_tokens)
        != Some(effect_grammar::LookedCardDisposition::HandAndGraveyard)
    {
        return Ok(None);
    }

    Ok(Some(compose_singleton_hand_partition(
        sentences[sentence_idx].lowered(),
        &second_tokens,
        player,
        count,
        effect_grammar::LookedPartitionDestination::Graveyard,
    )))
}

pub fn parse_choose_same_controller_targets_then_sacrifice_one(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(
        parse_same_controller_targets_choose_sacrifice(sentences, sentence_idx)?
            .map(|(effects, _, _)| effects),
    )
}

pub fn parse_choose_same_controller_targets_then_sacrifice_one_return_other(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((mut effects, target_set_tag, chosen_tag)) =
        parse_same_controller_targets_choose_sacrifice(sentences, sentence_idx)?
    else {
        return Ok(None);
    };

    if !effect_grammar::is_return_other_to_owner_hand_shape(sentences[sentence_idx + 2].lowered()) {
        return Ok(None);
    }

    let mut other_filter = ObjectFilter::tagged(target_set_tag);
    other_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    effects.push(EffectAst::subject_verb_return_to_hand(
        TargetAst::Object(other_filter, None, None),
        false,
    ));
    Ok(Some(effects))
}

fn parse_same_controller_targets_choose_sacrifice(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<(Vec<EffectAst>, TagKey, TagKey)>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) =
        effect_grammar::parse_same_controller_sacrifice_shape(&first_tokens, &second_tokens)
    else {
        return Ok(None);
    };
    let target = mark_target_set_same_controller(effect_sentences::parse_target_phrase(
        &trim_commas(&first_tokens[shape.target]),
    )?);
    let TargetAst::WithCount(_, target_count) = &target else {
        return Ok(None);
    };
    if target_count.min != 2 || target_count.max != Some(2) || target_count.is_random() {
        return Ok(None);
    }

    let target_set_tag = helper_tag_for_tokens(&first_tokens, "target_set");
    let chosen_tag = helper_tag_for_tokens(&second_tokens, "chosen");
    Ok(Some((
        vec![
            EffectAst::subject_verb_target_only(target),
            EffectAst::SnapshotLastObjectTag {
                into: target_set_tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: ObjectFilter::tagged(target_set_tag.clone()),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::ItsController,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_sacrifice(
                PlayerAst::That,
                ObjectFilter::tagged(chosen_tag.clone()),
                1,
                None,
            ),
        ],
        target_set_tag,
        chosen_tag,
    )))
}

fn rest_action_effect(
    action: effect_grammar::RestActionShape,
    filter: ObjectFilter,
    player: PlayerAst,
) -> EffectAst {
    match action {
        effect_grammar::RestActionShape::Destroy => EffectAst::subject_verb_destroy_all(filter),
        effect_grammar::RestActionShape::Exile => EffectAst::subject_verb_exile_all(filter, false),
        effect_grammar::RestActionShape::Sacrifice => {
            EffectAst::subject_verb_sacrifice_all(player, filter)
        }
    }
}

pub(crate) fn append_rest_action_after_choice(
    effect: EffectAst,
    action: effect_grammar::RestActionShape,
) -> Option<Vec<EffectAst>> {
    match effect {
        EffectAst::ChooseObjects {
            filter,
            tag,
            count,
            count_value,
            player,
        } => {
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![
                EffectAst::ChooseObjects {
                    filter,
                    tag,
                    count,
                    count_value,
                    player,
                },
                rest_action_effect(action, rest_filter, player),
            ])
        }
        EffectAst::ForEachPlayer { effects } => {
            let [inner] = effects.as_slice() else {
                return None;
            };
            let EffectAst::ChooseObjects {
                filter,
                tag,
                count,
                count_value,
                player,
            } = inner.clone()
            else {
                return None;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![EffectAst::ForEachPlayer {
                effects: vec![
                    EffectAst::ChooseObjects {
                        filter,
                        tag,
                        count,
                        count_value,
                        player,
                    },
                    rest_action_effect(action, rest_filter, player),
                ],
            }])
        }
        EffectAst::ForEachOpponent { effects } => {
            let [inner] = effects.as_slice() else {
                return None;
            };
            let EffectAst::ChooseObjects {
                filter,
                tag,
                count,
                count_value,
                player,
            } = inner.clone()
            else {
                return None;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![EffectAst::ForEachOpponent {
                effects: vec![
                    EffectAst::ChooseObjects {
                        filter,
                        tag,
                        count,
                        count_value,
                        player,
                    },
                    rest_action_effect(action, rest_filter, player),
                ],
            }])
        }
        _ => None,
    }
}

pub fn parse_may_cast_target_graveyard_spell_then_exile_replacement(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) = effect_grammar::parse_graveyard_cast_replacement_shape(&first, &second)
    else {
        return Ok(None);
    };
    graveyard_cast_with_exile_replacement(&first, &shape)
}

/// The effects of a graveyard-cast permission whose spell is exiled instead
/// of going to a graveyard: the targeted card, the optional cast, and the
/// replacement bound to the cast spell.
pub fn graveyard_cast_with_exile_replacement(
    first: &[OwnedLexToken],
    shape: &effect_grammar::GraveyardCastReplacementShape,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let chosen_tag = helper_tag_for_tokens(first, "graveyard_cast_target");
    let cast_spell_tag = helper_tag_for_tokens(first, "cast_spell");
    let first_view = crate::lexer::TokenWordView::new(first);
    let Some(target_word) = first_view.parse_word_position("target") else {
        return Ok(None);
    };
    let Some(target_start) = first_view.token_index_after_words(target_word + 1) else {
        return Ok(None);
    };
    let without = first_view
        .parse_any_word_position_from(&["without"], target_word + 1)
        .and_then(|word| first_view.map_word_to_token_start(word));
    let by_paying = first_view
        .parse_phrase_start(&["by", "paying"])
        .filter(|word| *word > target_word)
        .and_then(|word| first_view.map_word_to_token_start(word));
    let mana_spend_clause =
        if shape.mana_spend_mode == ironsmith_core::value_model::ManaSpendMode::AnyType {
            first_view
                .parse_phrase_start(&["mana", "of", "any", "type"])
                .and_then(|word| first_view.map_word_to_token_start(word))
                .and_then(|mana| {
                    crate::slice_primitives::select_last_position(&first[..mana], |token| {
                        token.is_word("and")
                    })
                })
        } else {
            None
        };
    let target_end = without
        .into_iter()
        .chain(by_paying)
        .chain(mana_spend_clause)
        .min()
        .unwrap_or(first.len());
    let target_filter_tokens = trim_commas(&first[target_start..target_end]);
    let Ok(filter) = parse_object_filter_lexed(&target_filter_tokens, false) else {
        return Ok(None);
    };
    let spell_card = filter
        .card_types
        .iter()
        .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
    if filter.zone != Some(Zone::Graveyard)
        || !matches!(filter.owner, None | Some(PlayerFilter::You))
        || !spell_card
    {
        return Ok(None);
    }

    if shape.until_end_of_turn {
        let replacement_filter = ObjectFilter::tagged(chosen_tag.clone()).in_zone(Zone::Stack);
        let surface = ironsmith_core::GrantPlayTaggedSurface::default()
            .with_leading_duration(true)
            .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard);
        return Ok(Some(vec![
            EffectAst::TagAffected {
                effect: Box::new(EffectAst::subject_verb_target_only(TargetAst::Object(
                    filter,
                    Some(TextSpan::synthetic()),
                    None,
                ))),
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_from_current_zone_with_optional_surface(
                chosen_tag,
                PlayerAst::You,
                false,
                shape.without_paying_mana_cost,
                false,
                Some(surface),
            ),
            EffectAst::subject_verb_register_future_zone_replacement(
                replacement_filter,
                Some(Zone::Stack),
                Some(Zone::Graveyard),
                Zone::Exile,
                ZoneReplacementDurationAst::UntilEndOfTurn,
                crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
                false,
            ),
        ]));
    }

    let replacement_filter = ObjectFilter::spell().match_tagged(
        cast_spell_tag.clone(),
        TaggedOpbjectRelation::IsTaggedObject,
    );

    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_target_only(TargetAst::Object(
                filter,
                Some(TextSpan::synthetic()),
                None,
            ))),
            tag: chosen_tag.clone(),
        },
        EffectAst::May {
            effects: vec![EffectAst::TagAffected {
                effect: Box::new(
                    EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                        chosen_tag,
                        PlayerAst::You,
                        false,
                        false,
                        shape.without_paying_mana_cost,
                        shape.additional_mana_cost.clone(),
                        None,
                        shape.mana_spend_mode,
                    ),
                ),
                tag: cast_spell_tag,
            }],
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![EffectAst::subject_verb_register_future_zone_replacement(
                replacement_filter,
                Some(Zone::Stack),
                Some(Zone::Graveyard),
                Zone::Exile,
                ZoneReplacementDurationAst::OneShot,
                crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
                false,
            )],
        },
    ]))
}

fn target_for_referenced_stack_object(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    tokens: &[OwnedLexToken],
) -> TargetAst {
    let previous = sentence_idx
        .checked_sub(1)
        .map(|idx| sentences[idx].lowered());
    match effect_grammar::parse_stack_object_reference_shape(tokens, previous) {
        effect_grammar::StackObjectReferenceShape::Source => TargetAst::Source(None),
        effect_grammar::StackObjectReferenceShape::PreviousChosen => {
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None)
        }
        effect_grammar::StackObjectReferenceShape::Triggering => {
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None)
        }
    }
}

#[cfg(test)]
#[path = "reference_linked_inline_tempting_offer_copy_tests_15.rs"]
mod tempting_offer_copy_tests;

#[cfg(test)]
#[path = "reference_linked_inline_mill_result_cast_tests_16.rs"]
mod mill_result_cast_tests;

#[cfg(test)]
#[path = "reference_linked_inline_looked_partition_tests_17.rs"]
mod looked_partition_tests;

#[cfg(test)]
#[path = "reference_linked_inline_consult_shuffle_remainder_tests_18.rs"]
mod consult_shuffle_remainder_tests;

#[path = "reference_linked_programs/reference_linked_zone.rs"]
mod reference_linked_zone_programs;
pub use reference_linked_zone_programs::{
    parse_consult_match_into_hand_others_graveyard, parse_consult_match_move_all_to_graveyard,
};
#[path = "reference_linked_programs/reference_linked_permission.rs"]
mod reference_linked_permission_programs;
pub use reference_linked_permission_programs::{
    parse_consult_match_into_hand_exile_others, parse_exile_until_match_grant_play_this_turn,
};
#[path = "reference_linked_programs/reference_linked_library.rs"]
mod reference_linked_library_programs;
pub(crate) use reference_linked_library_programs::parse_put_from_milled_cards_followup;
pub(crate) use reference_linked_library_programs::tag_single_mill_effect;
use reference_linked_library_programs::{
    compose_reveal_top_put_matching_into_hand_rest_into_graveyard,
    compose_reveal_top_put_matching_into_hand_rest_on_bottom, milled_choice_filter_branches,
};
pub use reference_linked_library_programs::{
    parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
    parse_consult_match_move_and_bottom_remainder, parse_delayed_dies_exile_top_power_choose_play,
    parse_may_put_filtered_card_from_among_into_hand, parse_mill_then_may_put_from_among_into_hand,
    parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen,
    parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
    };
#[path = "reference_linked_programs/reference_linked_choice.rs"]
mod reference_linked_choice_programs;
pub use reference_linked_choice_programs::parse_choose_then_do_same_for_filter_then_return_to_battlefield;
#[path = "reference_linked_programs/reference_linked_combat.rs"]
mod reference_linked_combat_programs;
#[path = "reference_linked_programs/reference_linked_counter.rs"]
mod reference_linked_counter_programs;
#[path = "reference_linked_programs/reference_linked_condition.rs"]
mod reference_linked_condition_programs;
pub(crate) use reference_linked_condition_programs::append_to_outer_if_result;
#[path = "reference_linked_programs/reference_linked_reference.rs"]
mod reference_linked_reference_programs;
use reference_linked_reference_programs::{
    contains_tagged_source_animation,
    parse_copy_for_each_candidate_filter,
};
pub(crate) use reference_linked_reference_programs::retarget_source_self_animate_effect;
pub use reference_linked_reference_programs::parse_for_each_tagged_copy_then_copy_targets_it;
pub(crate) use reference_linked_reference_programs::parse_copy_for_each_target_sentence;
#[path = "reference_linked_programs/reference_linked_trigger.rs"]
mod reference_linked_trigger_programs;
pub(crate) use reference_linked_trigger_programs::contains_triggered_life_gain_effect;
#[path = "reference_linked_programs/reference_linked_core.rs"]
mod reference_linked_core_programs;
pub(crate) use reference_linked_core_programs::parse_self_animate_followup_effects;
#[path = "reference_linked_programs/reference_linked_object_action.rs"]
mod reference_linked_object_action_programs;

/// Install the destination and controller replacements before the counter
/// instruction fires. This preserves the single Stack -> Battlefield event;
/// it never moves a successfully countered card back out of its graveyard.
pub fn parse_counter_spell_then_artifact_or_creature_enters_under_your_control(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    if !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(sentences[sentence_idx].lexed()),
        &["counter", "target", "spell"],
    ) || !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::parser_token_word_refs(sentences[sentence_idx + 1].lexed()),
        &[
            "if",
            "an",
            "artifact",
            "or",
            "creature",
            "spell",
            "is",
            "countered",
            "this",
            "way",
            "put",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "instead",
            "of",
            "into",
            "its",
            "owners",
            "graveyard",
        ],
    ) {
        return Ok(None);
    }

    let parsed_counter = effect_sentences::parse_effect_sentence_lexed(first)?;
    let [
        EffectAst::SubjectVerb(
            counter @ SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Counter { target },
                ..
            },
        ),
    ] = parsed_counter.as_slice()
    else {
        return Ok(None);
    };
    let target = target.clone();
    let counter = EffectAst::SubjectVerb(counter.clone());
    let matching_spell = ObjectFilter {
        card_types: vec![CardType::Artifact, CardType::Creature],
        ..ObjectFilter::spell()
    };
    let mut chosen_spell = matching_spell.clone();
    chosen_spell.is_target_object = true;
    let registrations = vec![
        EffectAst::subject_verb_register_zone_replacement(
            target,
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Battlefield,
            ZoneReplacementDurationAst::OneShot,
        ),
        EffectAst::subject_verb_register_enter_under_control_replacement(
            chosen_spell,
            ZoneReplacementDurationAst::OneShot,
        ),
    ];
    Ok(Some(vec![
        EffectAst::Conditional {
            predicate: PredicateAst::TargetMatches(matching_spell),
            if_true: registrations,
            if_false: Vec::new(),
        },
        counter,
    ]))
}

pub fn parse_filtered_future_exile_then_return_next_end_step(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::is_filtered_future_exile_return_next_end_step_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    ) {
        return Ok(None);
    }

    let linked_filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind())
        .in_zone(Zone::Exile);
    Ok(Some(vec![
        EffectAst::subject_verb_register_future_zone_replacement(
            ObjectFilter::permanent().controlled_by(PlayerFilter::You),
            Some(Zone::Battlefield),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::UntilEndOfTurn,
            crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
            true,
        ),
        EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::subject_verb_return_all_to_battlefield(
                linked_filter,
                false,
                false,
                ReturnControllerAst::Owner,
            )],
        },
    ]))
}

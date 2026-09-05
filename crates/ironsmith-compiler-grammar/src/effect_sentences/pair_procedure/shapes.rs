//! The fixed-shape statements of [`super`]: each reads a statement together
//! with the sentences that complete it and spells them as one.

#![allow(unused_imports)]

use super::super::dispatch_entry::SentenceInput;
use super::super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::parse_copy_for_each_target_sentence;
use super::super::sequence_rules::generic_subject_verb_sequences::reference_linked_programs::target_opponent_filter;
use crate::cards::builders::{
    CardTextError, ChoiceCount, ChooseOneModeAst, EffectAst, IfResultPredicate, ObjectFilter,
    PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, TargetAst,
};
use crate::effect::Value;
use crate::target::PlayerFilter;
use crate::grammar::effects::triple_sequence_shapes as triple_grammar;
use crate::lexer::LexedClause;
use crate::util::{helper_tag_for_tokens, trim_commas};
use crate::zone::Zone;
use crate::grammar::effects::{self as effect_grammar, generic_sequence_shapes as sequence_grammar};
use super::super::sequence_rules::generic_subject_verb_sequences::{
    ordered_control_flow_programs, reference_linked_programs,
};

pub(super) fn destroy_all_then_search_shuffle(
    first: &SentenceInput,
    second: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((destroy_clause, search_clause)) =
        LexedClause::new(first.lowered()).split_comma_then()
    else {
        return Ok(None);
    };
    if !matches!(
        effect_grammar::followup_shapes::parse_library_shuffle_followup_shape(
            second.lowered(),
        ),
        Some(effect_grammar::followup_shapes::LibraryShuffleFollowupShape::ThatPlayer)
    ) {
        return Ok(None);
    }

    let destroy_effects = crate::effect_sentences::parse_effect_sentence_lexed(destroy_clause.tokens())?;
    let [
        destroy @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::DestroyAll { .. },
            ..
        }),
    ] = destroy_effects.as_slice()
    else {
        return Ok(None);
    };

    let Some(search_effects) =
        crate::effect_sentences::parse_search_library_sentence(search_clause.tokens())?
    else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    target: TargetAst::Player(target, _),
                    ..
                },
            ..
        }),
        search @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    filter,
                    destination: Zone::Graveyard,
                    chooser: PlayerAst::Implicit,
                    player: PlayerAst::That,
                    shuffle: false,
                    ..
                },
            ..
        }),
    ] = search_effects.as_slice()
    else {
        return Ok(None);
    };
    if !target_opponent_filter(target)
        || filter.zone != Some(Zone::Library)
        || filter
            .owner
            .as_ref()
            .is_none_or(|owner| !target_opponent_filter(owner))
    {
        return Ok(None);
    }

    Ok(Some(vec![
        destroy.clone(),
        search_effects[0].clone(),
        search.clone(),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::That,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]))
}


/// "Search your library for two cards." with its disposition and shuffle sentences, read together.
pub(super) fn search_two_disposition_then_shuffle(
    first: &SentenceInput,
    second: &SentenceInput,
    third: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first.lowered());
    let first_effects = crate::effect_sentences::parse_effect_chain(&first_tokens)?;
    let (mut search_filter, count, count_value, chooser, library_player, search_mode) =
        match first_effects.as_slice() {
            [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::SearchLibrary {
                            filter,
                            chooser,
                            player,
                            search_mode,
                            count,
                            count_value,
                            ..
                        },
                    ..
                }),
            ] => (
                filter.clone(),
                *count,
                count_value.clone(),
                *chooser,
                *player,
                *search_mode,
            ),
            [
                EffectAst::ChooseObjectsAcrossZones {
                    filter,
                    count,
                    count_value,
                    player,
                    zones,
                    search_mode,
                    ..
                },
            ] if zones.len() == 1 && zones.first().is_some_and(|zone| *zone == Zone::Library) => (
                filter.clone(),
                *count,
                count_value.clone(),
                *player,
                *player,
                search_mode.unwrap_or(crate::effect::SearchSelectionMode::Exact),
            ),
            _ => return Ok(None),
        };
    if count.min != 2 || count.max != Some(2) || count_value.is_some() {
        return Ok(None);
    }

    let second_tokens = trim_commas(second.lowered());
    let third_tokens = trim_commas(third.lowered());
    if !triple_grammar::is_search_two_disposition_then_shuffle_shape(&second_tokens, &third_tokens)
    {
        return Ok(None);
    }

    search_filter.zone = Some(Zone::Library);
    let searched_tag = helper_tag_for_tokens(&first_tokens, "searched");
    let hand_tag = helper_tag_for_tokens(&second_tokens, "hand");
    let mut hand_filter = ObjectFilter::tagged(searched_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    let iterated_is_hand_card = ObjectFilter::default()
        .same_stable_id_as_tagged(crate::tag::CompilerReferenceTag::It.bind());

    Ok(Some(vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: search_filter,
            count,
            count_value,
            player: chooser,
            tag: searched_tag.clone(),
            zones: vec![Zone::Library],
            search_mode: Some(search_mode),
        },
        EffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            player: chooser,
            tag: hand_tag.clone(),
            zone: Zone::Library,
        },
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(hand_tag.clone(), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::ForEachTagged {
            tag: searched_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(hand_tag, iterated_is_hand_card),
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
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            library_player,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ]))
}


/// The tempting offer: the targeted spell, each opponent's optional copy, and
/// your copy once plus once per opponent who copied.
pub(super) fn tempting_offer_copy_effects() -> Vec<EffectAst> {
    let stack_spell_filter = ObjectFilter {
        zone: Some(Zone::Stack),
        card_types: vec![crate::types::CardType::Instant, crate::types::CardType::Sorcery],
        has_mana_cost: true,
        ..Default::default()
    };
    let target_spell = TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None);
    let opponent_copy = EffectAst::subject_verb_copy_spell(
        target_spell.clone(),
        crate::effect::Value::Fixed(1),
        PlayerAst::That,
        true,
        false,
        Vec::new(),
    );
    let your_copy_count = crate::effect::Value::PendingEffectMetricOffset {
        source: ironsmith_core::EffectMetricSource::Outcome,
        metric: ironsmith_core::EffectMetric::PlayersWithPositiveCount,
        offset: 1,
    };
    let your_copy = EffectAst::subject_verb_copy_spell(
        target_spell,
        your_copy_count,
        PlayerAst::You,
        true,
        false,
        Vec::new(),
    )
    .with_copy_count_surface(
        ironsmith_core::effect::CopyCountSurface::OncePlusAdditionalPerOpponentWhoCopiedThisWay,
    );
    vec![
        EffectAst::subject_verb_explicit_target_only(TargetAst::Object(
            stack_spell_filter,
            Some(crate::TextSpan::synthetic()),
            None,
        )),
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::MayByPlayer {
                player: PlayerAst::That,
                effects: vec![opponent_copy],
            }],
        },
        your_copy,
    ]
}

pub(super) fn history_counter_source(
    first: &SentenceInput,
    second: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let expected_tail = [
        "if", "it", "has", "blocked", "or", "been", "blocked", "since", "your", "last", "upkeep",
    ];
    let first_words = crate::lexer::token_word_refs(first.lowered());
    let Some(if_word_index) =
        crate::word_primitives::parse_sequence_start(&first_words, &expected_tail)
    else {
        return Ok(None);
    };
    if if_word_index == 0 || if_word_index + expected_tail.len() != first_words.len() {
        return Ok(None);
    }
    let first_view = crate::lexer::TokenWordView::new(first.lowered());
    let Some(if_index) = first_view.map_word_to_token_start(if_word_index) else {
        return Ok(None);
    };
    if !matches!(
        crate::lexer::token_word_refs(second.lowered()).first(),
        Some(&"otherwise")
    ) {
        return Ok(None);
    }

    let true_effects = crate::effect_sentences::parse_effect_sentence_lexed(&first.lowered()[..if_index])?;
    let false_effects = crate::effect_sentences::parse_effect_sentence_lexed(
        second.lowered().get(1..).unwrap_or_default(),
    )?;
    if true_effects.is_empty() || false_effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::Conditional {
        predicate: PredicateAst::SourceBlockedOrBecameBlockedSinceLastUpkeep,
        if_true: true_effects,
        if_false: false_effects,
    }]))
}


pub(super) fn history_counter_enchanted(
    first: &SentenceInput,
    second: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::lexer::token_word_refs(first.lowered());
    let expected_tail = [
        "if", "it", "attacked", "or", "blocked", "since", "your", "last", "upkeep",
    ];
    let Some(if_word_index) =
        crate::word_primitives::parse_sequence_start(&first_words, &expected_tail)
    else {
        return Ok(None);
    };
    if if_word_index == 0 || if_word_index + expected_tail.len() != first_words.len() {
        return Ok(None);
    }
    let first_view = crate::lexer::TokenWordView::new(first.lowered());
    let Some(if_index) = first_view.map_word_to_token_start(if_word_index) else {
        return Ok(None);
    };
    let second_words = crate::lexer::token_word_refs(second.lowered());
    if !matches!(second_words.first(), Some(&"otherwise")) {
        return Ok(None);
    }

    let true_effects = crate::effect_sentences::parse_effect_sentence_lexed(&first.lowered()[..if_index])?;
    let false_effects = crate::effect_sentences::parse_effect_sentence_lexed(
        second.lowered().get(1..).unwrap_or_default(),
    )?;
    if true_effects.is_empty() || false_effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::Conditional {
        predicate: PredicateAst::EnchantedPermanentAttackedOrBlockedSinceLastUpkeep,
        if_true: true_effects,
        if_false: false_effects,
    }]))
}


pub(super) fn choose_phase_then_skip(
    first: &SentenceInput,
    second: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::parse_choose_then_skip_phase_shape(
        first.lowered(),
        second.lowered(),
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_choose_named_option(
            PlayerAst::That,
            vec![
                "draw step".to_string(),
                "main phase".to_string(),
                "combat phase".to_string(),
            ],
        ),
        EffectAst::Conditional {
            predicate: PredicateAst::SourceChosenOption("draw step".to_string()),
            if_true: vec![EffectAst::subject_verb_skip_draw_step(PlayerAst::That)],
            if_false: vec![EffectAst::Conditional {
                predicate: PredicateAst::SourceChosenOption("main phase".to_string()),
                if_true: vec![EffectAst::subject_verb_skip_main_phases_this_turn(
                    PlayerAst::That,
                )],
                if_false: vec![EffectAst::subject_verb_skip_combat_phases_this_turn(
                    PlayerAst::That,
                )],
            }],
        },
    ]))
}


pub(super) fn target_opponent_copy_retarget(
    first: &SentenceInput,
    second: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(first.lowered()),
        &[
            "up", "to", "one", "target", "opponent", "may", "also", "copy", "that", "spell",
        ],
    ) || !crate::word_primitives::parse_sequence_complete(
        &crate::lexer::token_word_refs(second.lowered()),
        &[
            "they", "may", "choose", "new", "targets", "for", "that", "copy",
        ],
    ) {
        return Ok(None);
    }

    let target = crate::util::parse_target_phrase(&first.lowered()[..5])?;
    let copy = EffectAst::subject_verb_copy_spell(
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.bind(), None),
        Value::Fixed(1),
        PlayerAst::TargetOpponent,
        true,
        false,
        Vec::new(),
    );
    Ok(Some(vec![
        EffectAst::subject_verb_explicit_target_only(target),
        EffectAst::MayByPlayer {
            player: PlayerAst::TargetOpponent,
            effects: vec![copy],
        },
    ]))
}


pub(super) fn starting_each_player_optional_repeat(
    first: &SentenceInput,
    second: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = sequence_grammar::parse_starting_each_player_optional_repeat_shape(
        first.lowered(),
        second.lowered(),
    ) else {
        return Ok(None);
    };

    let Ok(parsed) = crate::effect_sentences::parse_effect_sentence_lexed(shape.each_player_clause_tokens)
        .or_else(|_| crate::effect_sentences::parse_effect_chain(shape.each_player_clause_tokens))
    else {
        return Ok(None);
    };
    let [
        EffectAst::ForEachPlayer {
            effects: per_player_effects,
        },
    ] = parsed.as_slice()
    else {
        return Ok(None);
    };
    if !matches!(
        per_player_effects.as_slice(),
        [EffectAst::May { .. } | EffectAst::MayByPlayer { .. }]
    ) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::RepeatProcess {
        effects: vec![EffectAst::SourceSentence {
            effects: parsed,
            leading_then: false,
            starting_with_controller: true,
        }],
        continue_effect_index: 0,
        continue_predicate: IfResultPredicate::Did,
    }]))
}


pub(super) fn each_player_pay_life_tokens(
    first: &SentenceInput,
    second: &SentenceInput,
    third: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !sequence_grammar::parse_each_player_pay_life_sequence_shape(
        first.lowered(),
        second.lowered(),
        third.lowered(),
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RepeatProcess {
            effects: vec![EffectAst::SourceSentence {
                effects: vec![EffectAst::ForEachPlayer {
                    effects: vec![EffectAst::subject_verb_pay_any_life(PlayerAst::That, 0)],
                }],
                leading_then: false,
                starting_with_controller: true,
            }],
            continue_effect_index: 0,
            continue_predicate: IfResultPredicate::Did,
        },
        EffectAst::ForEachPlayer {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::That,
                SubjectVerbActionAst::CreateTokenWithMods {
                    name: "1/1 black Rat creature".to_string(),
                    definition: crate::effect_sentences::sequence_rules::generic_subject_verb_sequences::rat_token_definition(),
                    count: Value::PendingEffectMetric {
                        source: ironsmith_core::EffectMetricSource::Outcome,
                        metric: ironsmith_core::EffectMetric::Count,
                    },
                    dynamic_power_toughness: None,
                    player: PlayerAst::That,
                    actor_surface_explicit: false,
                    attached_to: None,
                    tapped: false,
                    attacking: false,
                    attack_target_player: None,
                    exile_at_end_of_combat: false,
                    sacrifice_at_end_of_combat: false,
                    sacrifice_at_next_end_step: false,
                    exile_at_next_end_step: false,
                    next_end_step_player: PlayerFilter::Any,
                    granted_abilities: Vec::new(),
                    ability_presentation: None,
                },
            )],
        },
    ]))
}


pub(super) fn opponents_sacrifice_or_discard_damage(
    first: &SentenceInput,
    second: &SentenceInput,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !matches!(
        crate::lexer::token_word_refs(first.lowered()).as_slice(),
        [
            "each",
            "opponent",
            "may",
            "sacrifice",
            "a",
            "nonland",
            "permanent",
            "of",
            "their",
            "choice",
            "or",
            "discard",
            "a",
            "card"
        ]
    ) || !matches!(
        crate::lexer::token_word_refs(second.lowered()).as_slice(),
        [
            "then",
            "this",
            "creature",
            "deals",
            "damage",
            "equal",
            "to",
            "its",
            "power",
            "to",
            "each",
            "opponent",
            "who",
            "didnt" | "didn't",
            "sacrifice",
            "a",
            "permanent",
            "or",
            "discard",
            "a",
            "card",
            "this",
            "way"
        ]
    ) {
        return Ok(None);
    }

    let sacrifice_filter = ObjectFilter::nonland()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::IteratedPlayer);
    let sacrifice = EffectAst::subject_verb_sacrifice(PlayerAst::That, sacrifice_filter, 1, None);
    let discard =
        EffectAst::subject_verb_discard(PlayerAst::That, Value::Fixed(1), false, false, None, None);
    let choice = EffectAst::VillainousChoice {
        player: PlayerFilter::IteratedPlayer,
        player_surface: None,
        modes: vec![
            ChooseOneModeAst {
                description: "Sacrifice a nonland permanent".to_string(),
                effects: vec![sacrifice],
            },
            ChooseOneModeAst {
                description: "Discard a card".to_string(),
                effects: vec![discard],
            },
        ],
    };
    let offer = EffectAst::ForEachOpponent {
        effects: vec![EffectAst::MayByPlayer {
            player: PlayerAst::That,
            effects: vec![choice],
        }],
    };
    let damage = EffectAst::subject_verb_damage_equal_to_power(
        TargetAst::Source(None),
        TargetAst::Player(PlayerFilter::IteratedPlayer, None),
    );
    let consequence = EffectAst::ForEachOpponentDid {
        effects: vec![damage],
        predicate: None,
        result_predicate: IfResultPredicate::DidNot,
    };

    Ok(Some(vec![offer, consequence]))
}


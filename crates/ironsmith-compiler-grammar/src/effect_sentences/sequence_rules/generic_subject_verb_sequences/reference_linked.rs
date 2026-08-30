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

fn target_opponent_filter(player: &PlayerFilter) -> bool {
    matches!(
        player,
        PlayerFilter::Target(inner)
            if matches!(inner.as_ref(), PlayerFilter::Opponent)
                || target_opponent_filter(inner)
    )
}

fn tagged_subset_destroy_words(tokens: &[OwnedLexToken]) -> bool {
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

    let drawn_tag = crate::tag::CompilerReferenceTag::DrawnRevealedCard.key();
    let triggering_tag = crate::tag::CompilerReferenceTag::Triggering.key();
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

fn counted_target_object_filter(target: &TargetAst) -> Option<&ObjectFilter> {
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

/// Preserve the selected opponent as both the copier and the retargeting
/// decision owner for clauses of the form "up to one target opponent may
/// also copy that spell. They may choose new targets for that copy."
pub fn parse_target_opponent_may_copy_triggering_spell_then_retarget(
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
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None),
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

#[cfg(test)]
#[path = "reference_linked_inline_target_opponent_copy_triggering_spell_tests_2.rs"]
mod target_opponent_copy_triggering_spell_tests;

/// Parse the authored inverted form "copy the next spell ... when you cast
/// it" as the same one-shot cast watcher used by the canonical "when you next
/// cast" spelling. The following retarget sentence is part of the copy
/// effect, not a second action on the currently resolving stack object.
pub fn parse_copy_next_spell_when_cast_then_retarget(
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
        &crate::lexer::token_word_refs(second.lowered()),
        &[
            "you", "may", "choose", "new", "targets", "for", "the", "copy",
        ],
    ) {
        return Ok(None);
    }

    let words = crate::lexer::token_word_refs(first.lowered());
    if !crate::word_primitives::parse_sequence_complete(
        &words,
        &[
            "copy", "the", "next", "spell", "you", "cast", "this", "turn", "when", "you", "cast",
            "it",
        ],
    ) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger: TriggerSpec::SpellCast {
            filter: None,
            mana_source_filter: None,
            caster: PlayerFilter::You,
            timing: None,
            during_turn: None,
            min_spells_this_turn: None,
            exact_spells_this_turn: None,
            from_not_hand: false,
        },
        effects: vec![EffectAst::subject_verb_copy_spell(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None),
            Value::Fixed(1),
            PlayerAst::You,
            true,
            false,
            Vec::new(),
        )],
        one_shot: true,
        until_end_of_combat: false,
        attach_to_previous_ability: false,
    }]))
}

#[cfg(test)]
#[path = "reference_linked_inline_copy_next_spell_when_cast_tests_3.rs"]
mod copy_next_spell_when_cast_tests;

/// Preserve an attached object's combat history as a condition rather than
/// letting the postfix `if` words collapse into the counter target filter.
pub fn parse_counter_on_enchanted_if_attacked_or_blocked_since_last_upkeep(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(second) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
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

    let true_effects = effect_sentences::parse_effect_sentence_lexed(&first.lowered()[..if_index])?;
    let false_effects = effect_sentences::parse_effect_sentence_lexed(
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

/// Preserve the source creature's two-sided block history in the postfix
/// condition instead of allowing the object-filter grammar to reinterpret
/// "blocked" as a current characteristic.
pub fn parse_counter_on_source_if_blocked_or_been_blocked_since_last_upkeep(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(first) = sentences.get(sentence_idx) else {
        return Ok(None);
    };
    let Some(second) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
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

    let true_effects = effect_sentences::parse_effect_sentence_lexed(&first.lowered()[..if_index])?;
    let false_effects = effect_sentences::parse_effect_sentence_lexed(
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

/// Keep a later typed subset action tied to the exact objects selected by an
/// earlier multi-target restriction. The second sentence still parses its own
/// quality filter (for example, `Wall`); the stable tag supplies only the
/// authored "of them" membership relation.
pub fn parse_multi_target_restriction_then_destroy_typed_subset(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let second_tokens = sentences[sentence_idx + 1].lowered();
    if !tagged_subset_destroy_words(second_tokens) {
        return Ok(None);
    }

    let mut first_effects = effect_sentences::parse_effect_sentence_lexed(first_tokens)?;
    let [target_effect, cant_effect] = first_effects.as_mut_slice() else {
        return Ok(None);
    };
    let target_filter = match target_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::TargetOnly {
                    target,
                    explicit_declaration: false,
                },
            ..
        }) => counted_target_object_filter(target).cloned(),
        _ => None,
    };
    let Some(target_filter) = target_filter else {
        return Ok(None);
    };

    let target_set_tag = helper_tag_for_tokens(first_tokens, "restricted_target_set");
    let restriction_filter = match cant_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Cant {
                    restriction: crate::effect::Restriction::Block(filter),
                    duration: crate::effect::Until::EndOfTurn,
                    start: crate::effect::RestrictionStart::Immediate,
                    duration_surface: crate::effect::RestrictionDurationSurface::Default,
                    condition: None,
                },
            ..
        }) => filter,
        _ => return Ok(None),
    };
    let expected_it_constraint = TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::It.key(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    };
    if !matches!(restriction_filter.tagged_constraints.as_slice(), [constraint] if *constraint == expected_it_constraint)
    {
        return Ok(None);
    }
    let mut restriction_base = restriction_filter.clone();
    restriction_base.tagged_constraints.clear();
    if restriction_base != target_filter {
        return Ok(None);
    }
    restriction_filter.tagged_constraints[0].tag = target_set_tag.clone();

    let Some(are_index) =
        crate::slice_primitives::select_position(second_tokens, |token| token.is_word("are"))
    else {
        return Ok(None);
    };
    let mut destroy_filter = parse_object_filter_lexed(&second_tokens[are_index + 1..], false)?;
    if !destroy_filter.tagged_constraints.is_empty() {
        return Ok(None);
    }
    destroy_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: target_set_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    let second_effects = vec![EffectAst::subject_verb_destroy(TargetAst::Object(
        destroy_filter,
        None,
        None,
    ))];

    let original_target = target_effect.clone();
    *target_effect = EffectAst::TagAffected {
        effect: Box::new(original_target),
        tag: target_set_tag,
    };
    first_effects.extend(second_effects);
    Ok(Some(first_effects))
}

#[cfg(test)]
#[path = "reference_linked_inline_tagged_target_subset_tests_5.rs"]
mod tagged_target_subset_tests;

/// Preserve the independently targeted library procedure after a global
/// destroy clause. Parsing the complete first sentence as one object filter
/// lets the broad union grammar absorb `search ... library` into the destroy
/// domain; splitting the authored comma-then boundary first keeps the two
/// executable actions and their distinct subjects.
pub fn parse_destroy_all_then_search_target_opponent_to_graveyard_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((destroy_clause, search_clause)) =
        LexedClause::new(sentences[sentence_idx].lowered()).split_comma_then()
    else {
        return Ok(None);
    };
    if !matches!(
        effect_grammar::followup_shapes::parse_library_shuffle_followup_shape(
            sentences[sentence_idx + 1].lowered(),
        ),
        Some(effect_grammar::followup_shapes::LibraryShuffleFollowupShape::ThatPlayer)
    ) {
        return Ok(None);
    }

    let destroy_effects = effect_sentences::parse_effect_sentence_lexed(destroy_clause.tokens())?;
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
        effect_sentences::parse_search_library_sentence(search_clause.tokens())?
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

#[cfg(test)]
#[path = "reference_linked_inline_destroy_search_partition_tests_6.rs"]
mod destroy_search_partition_tests;

pub fn parse_resolving_card_exile_then_return_next_end_step(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let replacement = sentences[sentence_idx].lowered();
    let delayed_return = sentences[sentence_idx + 1].lowered();
    if !effect_grammar::is_resolving_card_exile_then_return_next_end_step_shape(
        replacement,
        delayed_return,
    ) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_register_zone_replacement_with_linked_exile_follow_up(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
            ironsmith_core::LinkedExileFollowUp::ReturnToHandAtNextEndStep,
        ),
    ]))
}

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

#[cfg(test)]
#[path = "reference_linked_inline_resolving_card_exile_tests_7.rs"]
mod resolving_card_exile_tests;

#[cfg(test)]
#[path = "reference_linked_inline_counter_destination_replacement_tests_8.rs"]
mod counter_destination_replacement_tests;

/// Bind `in it` in a following subtype/color count to the hand revealed by
/// the immediately preceding targeted reveal. The generic count parser cannot
/// infer that pronoun's zone or player in isolation, so this pair rule keeps
/// the existing reveal effect and supplies the exact typed hand domain.
pub fn parse_reveal_hand_then_draw_shared_terminal_union(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(reveal_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [reveal_effect] = reveal_effects.as_slice() else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject:
            SubjectVerbSubjectAst {
                player: revealed_player,
                ..
            },
        action: SubjectVerbActionAst::RevealHand,
    }) = reveal_effect
    else {
        return Ok(None);
    };
    let revealed_player = match revealed_player {
        PlayerAst::Target => PlayerFilter::Any,
        PlayerAst::TargetOpponent => PlayerFilter::Opponent,
        _ => return Ok(None),
    };

    let draw_words = crate::lexer::token_word_refs(sentences[sentence_idx + 1].lowered());
    if draw_words.len() < 10
        || draw_words.get(..6) != Some(["you", "draw", "a", "card", "for", "each"].as_slice())
        || draw_words.get(draw_words.len() - 2..) != Some(["in", "it"].as_slice())
    {
        return Ok(None);
    }
    let filter_words = &draw_words[6..draw_words.len() - 2];
    let filter_tokens = crate::lexer::synthetic_word_tokens(filter_words);
    let Some(mut filter) =
        crate::grammar::filters::parse_subtype_color_shared_card_union_lexed(&filter_tokens, false)
    else {
        return Ok(None);
    };
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::AliasedTarget(Box::new(revealed_player)));
    let count = Value::Count(filter).with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach);

    Ok(Some(vec![
        reveal_effect.clone(),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw { count },
        ),
    ]))
}

/// Bind the two disjoint domains in "a nonland card from it or a card from
/// that player's graveyard" to the exact opponent whose hand was revealed.
/// The hand branch alone carries the nonland restriction and revealed-set
/// tag; the graveyard branch alone carries that opponent's ownership.
pub fn parse_reveal_opponent_hand_then_choose_from_it_or_their_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(reveal_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [
        reveal_effect @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject:
                SubjectVerbSubjectAst {
                    player: PlayerAst::TargetOpponent,
                    ..
                },
            action: SubjectVerbActionAst::RevealHand,
        }),
    ] = reveal_effects.as_slice()
    else {
        return Ok(None);
    };
    let words = crate::lexer::token_word_refs(sentences[sentence_idx + 1].lowered());
    if !crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &[
                "you",
                "choose",
                "a",
                "nonland",
                "card",
                "from",
                "it",
                "or",
                "a",
                "card",
                "from",
                "their",
                "graveyard",
            ],
            &[
                "you",
                "choose",
                "a",
                "nonland",
                "card",
                "from",
                "it",
                "or",
                "a",
                "card",
                "from",
                "that",
                "players",
                "graveyard",
            ],
        ],
    ) {
        return Ok(None);
    }

    let mut hand = ObjectFilter::default();
    hand.zone = Some(Zone::Hand);
    hand.excluded_card_types = vec![CardType::Land];
    hand.tagged_constraints.push(TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::It.key(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    let mut graveyard = ObjectFilter::default();
    graveyard.zone = Some(Zone::Graveyard);
    graveyard.owner = Some(PlayerFilter::AliasedTarget(Box::new(
        PlayerFilter::Opponent,
    )));
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![hand, graveyard];

    Ok(Some(vec![
        reveal_effect.clone(),
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::CompilerReferenceTag::It.key(),
        },
    ]))
}

#[cfg(test)]
#[path = "reference_linked_inline_revealed_hand_graveyard_disjunction_tests_9.rs"]
mod revealed_hand_graveyard_disjunction_tests;

/// Bind an optional free cast to the exact cards revealed from a targeted
/// opponent's hand. Parsing the cast sentence by itself cannot recover either
/// the hand owner or the optional one-card choice from "among those cards";
/// this pair rule retains both as executable structure.
pub fn parse_reveal_target_opponent_hand_then_may_cast_from_those_cards(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(reveal_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [reveal_effect] = reveal_effects.as_slice() else {
        return Ok(None);
    };
    if !matches!(
        reveal_effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst {
                player: PlayerAst::TargetOpponent,
                ..
            },
            action: SubjectVerbActionAst::RevealHand,
        })
    ) {
        return Ok(None);
    }

    let cast_tokens = sentences[sentence_idx + 1].lowered();
    let cast_words = crate::lexer::parser_token_word_refs(cast_tokens);
    let exact_spell_surface = [
        "you", "may", "cast", "an", "instant", "or", "sorcery", "spell", "from", "among", "those",
        "cards", "without", "paying", "its", "mana", "cost",
    ];
    let exact_card_surface = [
        "you", "may", "cast", "an", "instant", "or", "sorcery", "card", "from", "among", "those",
        "cards", "without", "paying", "its", "mana", "cost",
    ];
    if cast_words.as_slice() != exact_spell_surface && cast_words.as_slice() != exact_card_surface {
        return Ok(None);
    }

    let chosen_tag = helper_tag_for_tokens(cast_tokens, "chosen_revealed_spell");
    let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::RevealedThisWay.key());
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::AliasedTarget(Box::new(
        PlayerFilter::Opponent,
    )));
    filter.card_types = vec![CardType::Instant, CardType::Sorcery];

    Ok(Some(vec![
        reveal_effect.clone(),
        EffectAst::May {
            effects: vec![
                EffectAst::ChooseTaggedObjectsInZone {
                    filter,
                    count: ChoiceCount::exactly(1),
                    player: PlayerAst::You,
                    tag: chosen_tag.clone(),
                    zone: Zone::Hand,
                },
                EffectAst::subject_verb_cast_tagged(
                    chosen_tag,
                    PlayerAst::You,
                    false,
                    false,
                    true,
                    None,
                ),
            ],
        },
    ]))
}

/// Bind an optional free cast to the exact hand established by a preceding
/// look instruction. The standalone permission parser treats "those cards"
/// as an exiled collection because it cannot see the prior sentence; this
/// two-sentence rule retains the looked player's hand as the executable zone
/// and owner without introducing a new effect capability.
pub fn parse_look_at_players_hand_then_may_cast_from_those_cards(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(Some(look_effects)) =
        effect_sentences::parse_look_at_hand_sentence(sentences[sentence_idx].lexed())
    else {
        return Ok(None);
    };
    let [look_effect] = look_effects.as_slice() else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::LookAtHand {
                target: TargetAst::Player(hand_owner, _),
            },
        ..
    }) = look_effect
    else {
        return Ok(None);
    };
    let is_damaged_player_reference = matches!(
        hand_owner,
        PlayerFilter::DamagedPlayer | PlayerFilter::IteratedPlayer
    ) || matches!(
        hand_owner,
        PlayerFilter::Target(_) | PlayerFilter::AliasedTarget(_)
    );
    if !is_damaged_player_reference {
        return Ok(None);
    }

    // Keep the authored collection phrase for this pair. Generic reference
    // normalization rewrites `a spell from among those cards` to `it`, which
    // is fine for a standalone cast but destroys both the optional choice and
    // the looked-hand zone provenance that this sequence rule owns.
    let cast_tokens = sentences[sentence_idx + 1].lexed();
    let cast_words = crate::lexer::parser_token_word_refs(cast_tokens);
    let exact_surface = [
        "you", "may", "cast", "a", "spell", "from", "among", "those", "cards", "without", "paying",
        "its", "mana", "cost",
    ];
    if cast_words.as_slice() != exact_surface {
        return Ok(None);
    }

    Ok(Some(vec![
        look_effect.clone(),
        EffectAst::may_cast_matching_spell_without_paying_mana_cost_from_zone_owner(
            PlayerAst::You,
            PlayerAst::That,
            ObjectFilter::nonland().in_zone(Zone::Hand),
            Zone::Hand,
        ),
    ]))
}

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
                    crate::tag::CompilerReferenceTag::It.key(),
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

pub(super) fn tagged_library_candidate_filter(
    candidate: &TagKey,
    excluded: &[TagKey],
) -> ObjectFilter {
    let mut filter = ObjectFilter::tagged(candidate.clone()).in_zone(Zone::Library);
    for tag in excluded {
        filter = filter.not_tagged(tag.clone());
    }
    filter
}

fn looked_library_owner_filter(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You => Some(PlayerFilter::You),
        PlayerAst::Target => Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any))),
        PlayerAst::TargetOpponent => Some(PlayerFilter::AliasedTarget(Box::new(
            PlayerFilter::Opponent,
        ))),
        _ => None,
    }
}

pub(super) fn move_tagged_to_looked_destination(
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

fn move_looked_partition_group(
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

pub(super) fn parse_reveal_top_and_choose_one_of_revealed(
    first_tokens: &[OwnedLexToken],
    second_tokens: &[OwnedLexToken],
) -> Result<
    Option<(
        Vec<EffectAst>,
        TagKey,
        PlayerAst,
        Option<effect_grammar::LookedCardDestinationShape>,
    )>,
    CardTextError,
> {
    let first_effects = effect_sentences::parse_effect_sentence_lexed(first_tokens)?;
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some((library_owner, count, true)) = top_cards_parts_with_reveal(first_effect) else {
        return Ok(None);
    };
    let Some(shape) = effect_grammar::parse_revealed_card_choice_shape(second_tokens) else {
        return Ok(None);
    };
    let chooser = match shape.chooser {
        effect_grammar::RevealedCardChooserShape::You => PlayerAst::You,
        effect_grammar::RevealedCardChooserShape::TargetOpponent => PlayerAst::TargetOpponent,
    };
    let candidate_tag = helper_tag_for_tokens(first_tokens, "revealed_candidates");
    let chosen_tag = helper_tag_for_tokens(second_tokens, "revealed_choice");
    let effects = vec![
        EffectAst::subject_verb_reveal_top_cards(library_owner, count, candidate_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: tagged_library_candidate_filter(&candidate_tag, &[]),
            count: ChoiceCount::exactly(1),
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Library,
        },
    ];
    Ok(Some((effects, chosen_tag, chooser, shape.destination)))
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

    let your_tag = crate::tag::CompilerReferenceTag::TwistYourCreatures.key();
    let target_tag = crate::tag::CompilerReferenceTag::TwistOpponentCreatures.key();
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

fn parse_optional_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ConsultSentenceParts, bool)>, CardTextError> {
    if let Some(shape) = effect_grammar::parse_optional_sequence_prefix_shape(tokens) {
        let stripped = trim_commas(&tokens[shape.tail]);
        return parse_consult_traversal_sentence(&stripped)
            .map(|parts| parts.map(|parts| (parts, true)));
    }
    parse_consult_traversal_sentence(tokens).map(|parts| parts.map(|parts| (parts, false)))
}

fn parse_gated_optional_consult_traversal_sentence(
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

fn strip_leading_if_you_do_sentence(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let Some(followup) = sentence_markers::parse_conditional_followup_tokens(tokens) else {
        return (trim_commas(tokens), false);
    };
    if followup.actor != ConditionalFollowupActor::You {
        return (trim_commas(tokens), false);
    }
    (trim_commas(followup.tail_tokens), true)
}

fn wrap_optional_consult_effects(
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

pub fn parse_look_at_top_then_exile_face_down_then_play_while_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let Some(shape) = effect_grammar::parse_look_exile_face_down_shape(first_tokens) else {
        return Ok(None);
    };

    let counted = match &shape {
        effect_grammar::LookExileFaceDownShape::Counted {
            look,
            exile,
            count,
            bottom_order,
        } => Some((look, exile, count, Some(*bottom_order))),
        effect_grammar::LookExileFaceDownShape::CountedGraveyardRemainder {
            look,
            exile,
            count,
        } => Some((look, exile, count, None)),
        effect_grammar::LookExileFaceDownShape::Single { .. } => None,
    };
    if let Some((look, exile, exile_count, bottom_order)) = counted {
        let look_tokens = trim_commas(&first_tokens[look.clone()]);
        let exile_tokens = trim_commas(&first_tokens[exile.clone()]);
        let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(&look_tokens) else {
            return Ok(None);
        };
        let [look_effect] = look_effects.as_slice() else {
            return Ok(None);
        };
        let Some((library_owner, count)) = look_at_top_cards_parts(look_effect) else {
            return Ok(None);
        };

        let Some(permission_effect) =
            parse_cast_or_play_tagged_clause(sentences[sentence_idx + 1].lowered())?
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
        let exiled_tag = helper_tag_for_tokens(&exile_tokens, "exiled");
        let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
        choice_filter.zone = Some(Zone::Library);

        let remainder = if let Some(bottom_order) = bottom_order {
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                looked_tag.clone(),
                Some(exiled_tag.clone()),
                bottom_order,
                library_owner,
            )
        } else {
            EffectAst::subject_verb(
                SubjectVerbRoleAst::Actor,
                PlayerAst::Implicit,
                SubjectVerbActionAst::PutTaggedRemainderInZone {
                    tag: looked_tag.clone(),
                    keep_tagged: exiled_tag.clone(),
                    zone: Zone::Graveyard,
                    surface: ironsmith_core::LibraryRemainderSurface::Rest,
                },
            )
        };

        return Ok(Some(vec![
            EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
            EffectAst::ChooseTaggedObjectsInZone {
                filter: choice_filter,
                count: *exile_count,
                player: PlayerAst::You,
                tag: exiled_tag.clone(),
                zone: Zone::Library,
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
            remainder,
            EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
                exiled_tag,
                permission_player,
                allow_land,
                without_paying_mana_cost,
                allow_any_color_for_cast,
                filter,
            ),
        ]));
    }

    let effect_grammar::LookExileFaceDownShape::Single { look } = shape else {
        unreachable!("counted look/exile shape returned above")
    };
    let look_tokens = trim_commas(&first_tokens[look]);
    let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(&look_tokens) else {
        return Ok(None);
    };
    let [look_effect] = look_effects.as_slice() else {
        return Ok(None);
    };
    let Some((player, count)) = look_at_top_cards_parts(look_effect) else {
        return Ok(None);
    };

    let Some(permission_effect) =
        parse_cast_or_play_tagged_clause(sentences[sentence_idx + 1].lowered())?
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
    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::subject_verb_exile(TargetAst::Tagged(looked_tag.clone(), None), true),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            looked_tag,
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
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
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
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

/// Preserves a private looked-card pool followed by an exact singleton move:
///
/// "Look at the top two cards of [a player's] library. Put one of them into
/// [that player's] graveyard."
///
/// The source and selected tags make it impossible for the move to affect the
/// unselected sibling. The library owner is explicit even though the ability's
/// controller is always the chooser.
pub fn parse_look_at_top_then_move_exact_one_to_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((library_owner, count, false)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(shape) =
        effect_grammar::parse_exact_looked_card_move_shape(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    if shape.destination != effect_grammar::LookedCardDestinationShape::Graveyard {
        return Ok(None);
    }
    let Some(owner_filter) = looked_library_owner_filter(library_owner) else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked_pool");
    let selected_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "looked_selected");
    let selected_filter = tagged_library_candidate_filter(&looked_tag, &[]).owned_by(owner_filter);
    let mut move_selected = move_tagged_to_looked_destination(
        selected_tag.clone(),
        effect_grammar::LookedCardDestinationShape::Graveyard,
    );
    if library_owner == PlayerAst::You {
        move_selected = move_selected.with_destination_player_surface(Some(PlayerAst::You));
    } else {
        move_selected = move_selected.with_destination_player_reference_surface(Some(
            ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer,
        ));
    }

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
        EffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: selected_tag,
            zone: Zone::Library,
        },
        move_selected,
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
                        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
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

pub fn parse_reveal_top_then_choose_revealed_and_move(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((mut effects, chosen_tag, _, Some(destination))) =
        parse_reveal_top_and_choose_one_of_revealed(
            sentences[sentence_idx].lowered(),
            sentences[sentence_idx + 1].lowered(),
        )?
    else {
        return Ok(None);
    };
    effects.push(move_tagged_to_looked_destination(chosen_tag, destination));
    Ok(Some(effects))
}

pub fn parse_choose_draw_main_or_combat_phase_then_skip_chosen_this_turn(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !effect_grammar::parse_choose_then_skip_phase_shape(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
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

fn append_rest_action_after_choice(
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

pub fn parse_choose_then_affect_rest(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(action) =
        effect_grammar::parse_rest_action_shape(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [first] = first_effects.as_slice() else {
        return Ok(None);
    };
    Ok(append_rest_action_after_choice(first.clone(), action))
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

    let chosen_tag = helper_tag_for_tokens(&first, "graveyard_cast_target");
    let cast_spell_tag = helper_tag_for_tokens(&first, "cast_spell");
    let first_view = crate::lexer::TokenWordView::new(&first);
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
                        shape.additional_mana_cost,
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

/// Preserve a targeted graveyard cast and its one-shot exile replacement
/// inside an authored reflexive "When you do" clause. The ordinary leading
/// result parser sees the cast sentence in isolation and can collapse its
/// target to the preceding payment result; delegate the trailing body to the
/// existing strict cast/replacement pair instead.
pub fn parse_when_result_may_cast_target_graveyard_spell_then_exile_replacement(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(prefix) = split_leading_result_prefix_lexed(sentences[sentence_idx].lexed()) else {
        return Ok(None);
    };
    if prefix.kind != LeadingResultPrefixKind::When
        || prefix.predicate != IfResultPredicate::Did
        || !crate::word_primitives::parse_sequence_prefix(
            &crate::lexer::token_word_refs(prefix.trailing_tokens),
            &["you", "may", "cast", "target"],
        )
    {
        return Ok(None);
    }

    let trailing = SentenceInput::from_lexed(prefix.trailing_tokens);
    let replacement = SentenceInput::from_lexed(sentences[sentence_idx + 1].lexed());
    let pair = [trailing, replacement];
    let Some(effects) = parse_may_cast_target_graveyard_spell_then_exile_replacement(&pair, 0)?
    else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::WhenResult {
        predicate: IfResultPredicate::Did,
        effects,
    }]))
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

    let linked_filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.key())
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
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None)
        }
        effect_grammar::StackObjectReferenceShape::Triggering => {
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None)
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
    parse_consult_match_into_battlefield_or_hand,
    parse_consult_match_into_battlefield_others_graveyard,
    parse_consult_match_into_hand_others_graveyard, parse_consult_match_move_all_to_graveyard,
};
#[path = "reference_linked_programs/reference_linked_permission.rs"]
mod reference_linked_permission_programs;
pub use reference_linked_permission_programs::{
    parse_consult_match_into_hand_exile_others, parse_exile_until_match_grant_play_this_turn,
};
#[path = "reference_linked_programs/reference_linked_library.rs"]
mod reference_linked_library_programs;
pub(super) use reference_linked_library_programs::tag_single_mill_effect;
use reference_linked_library_programs::{
    compose_reveal_top_put_matching_into_hand_rest_into_graveyard,
    compose_reveal_top_put_matching_into_hand_rest_on_bottom, milled_choice_filter_branches,
    parse_put_from_milled_cards_followup,
};
pub use reference_linked_library_programs::{
    parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
    parse_conditional_consult_match_move_and_bottom_remainder,
    parse_consult_match_move_and_bottom_remainder, parse_delayed_dies_exile_top_power_choose_play,
    parse_may_put_filtered_card_from_among_into_hand, parse_mill_then_may_cast_from_among,
    parse_mill_then_may_put_from_among_into_hand,
    parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen,
    parse_optional_look_then_reveal_put_top_rest_bottom,
    parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
    parse_top_cards_put_any_matching_to_zone_rest_same_sentence,
};
#[path = "reference_linked_programs/reference_linked_choice.rs"]
mod reference_linked_choice_programs;
pub use reference_linked_choice_programs::{
    parse_choose_creature_type_then_become_type,
    parse_choose_then_do_same_for_filter_then_return_to_battlefield,
};
#[path = "reference_linked_programs/reference_linked_combat.rs"]
mod reference_linked_combat_programs;
pub use reference_linked_combat_programs::parse_target_player_chooses_then_other_cant_block;
#[path = "reference_linked_programs/reference_linked_counter.rs"]
mod reference_linked_counter_programs;
pub use reference_linked_counter_programs::parse_exile_until_match_put_counters_on_match;
#[path = "reference_linked_programs/reference_linked_condition.rs"]
mod reference_linked_condition_programs;
use reference_linked_condition_programs::append_to_outer_if_result;
#[path = "reference_linked_programs/reference_linked_reference.rs"]
mod reference_linked_reference_programs;
use reference_linked_reference_programs::{
    contains_tagged_source_animation, parse_copy_for_each_candidate_filter,
    parse_copy_for_each_target_sentence, retarget_source_self_animate_effect,
};
pub use reference_linked_reference_programs::{
    parse_copy_for_each_target_then_each_copy_targets_different,
    parse_for_each_tagged_copy_then_copy_targets_it, parse_gain_life_then_self_animate_source,
};
#[path = "reference_linked_programs/reference_linked_trigger.rs"]
mod reference_linked_trigger_programs;
use reference_linked_trigger_programs::contains_triggered_life_gain_effect;
pub use reference_linked_trigger_programs::parse_whenever_gain_life_then_self_animate_source;
#[path = "reference_linked_programs/reference_linked_core.rs"]
mod reference_linked_core_programs;
use reference_linked_core_programs::parse_self_animate_followup_effects;
#[path = "reference_linked_programs/reference_linked_object_action.rs"]
mod reference_linked_object_action_programs;
pub use reference_linked_object_action_programs::parse_tempting_offer_copy_spell_sequence;

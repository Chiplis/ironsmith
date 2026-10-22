use super::super::super::dispatch_entry::{
    ConsultSentenceParts, consult_cast_effects, consult_stop_rule_is_single_match,
    leading_may_actor_to_player, parse_consult_cast_clause, parse_consult_traversal_sentence,
    parse_looked_card_choice_filter, parse_looked_card_reveal_filter,
    parse_top_cards_view_sentence, target_references_it,
};
use crate::cards::builders::ForEachEffectAst;
use crate::cards::builders::{
    CardTextError, ChoiceCount, ConditionalEffectAst, CounterActionAst, DamagePreventionActionAst,
    DelayedEffectAst, EffectAst, GrantActionAst, GrantedAbilityAst, IfResultPredicate,
    LibraryBottomOrderAst, LifeResourceActionAst, ObjectChoiceEffectAst, ObjectFilter,
    OwnedLexToken, PermissionEffectAst, PlayerAst, PredicateAst, ReturnControllerAst,
    RevealLookActionAst, StackActionAst, StatChangeActionAst, SubjectAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey, TargetAst, TextSpan,
    TriggerSpec, VoteEffectAst, ZoneReplacementDurationAst,
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
    let mana_value = Value::ManaValueOf(Box::new(ChooseSpec::Tagged(drawn_tag.clone().into())))
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs);
    let draw = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
            count: Value::Fixed(1),
        }),
    );
    let pump = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
            power: mana_value.clone(),
            toughness: mana_value.clone(),
            target: TargetAst::Tagged(triggering_tag, None),
            duration: crate::effect::Until::EndOfTurn,
            condition: None,
            set_quantifier_surface: None,
        }),
    );
    let lose_life = EffectAst::subject_verb(
        SubjectVerbRoleAst::AffectedPlayer,
        PlayerAst::You,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::LoseLife { amount: mana_value }),
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
            action: SubjectVerbActionAst::Counters(CounterActionAst::PutCounters {
                target,
                target_count: None,
                distributed: false,
                ..
            }),
            ..
        })] if matches!(target, TargetAst::Source(_))
            || matches!(target, TargetAst::Object(filter, None, _) if filter.source)
    ) {
        return Ok(None);
    }

    let participants = PlayerFilter::excluding(
        PlayerFilter::Any,
        PlayerFilter::excluding(PlayerFilter::NotYou, PlayerFilter::Defending),
    );
    let loot = EffectAst::ForEach(ForEachEffectAst::ForEachPlayersFiltered {
        sequential: false,
        filter: participants,
        effects: vec![
            EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::That,
                SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                    count: Value::Fixed(1),
                }),
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
    });
    Ok(Some(vec![EffectAst::Conditionals(
        ConditionalEffectAst::IfEffectResult {
            effect: Box::new(loot),
            predicate: EffectPredicate::PlayerActionObjectHasGreatestManaValue {
                player: PlayerFilter::You,
                action: ironsmith_core::PriorEffectAction::Discarded,
            },
            if_true: followup,
        },
    )]))
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
        tag: tag.clone().into(),
        reveal_after_choice: true,
    };

    Ok(Some(vec![
        EffectAst::Votes(VoteEffectAst::SecretChoiceStart {
            options: Vec::new(),
            participants: vec![PlayerFilter::You, PlayerFilter::target_opponent()],
            object_choice: Some(object_choice),
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(tag),
            effects: vec![EffectAst::subject_verb_sacrifice(
                PlayerAst::ItsController,
                sacrifice_filter,
                1,
                Some(TargetAst::Tagged(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    None,
                )),
            )],
        }),
    ]))
}

/// "For each color, put a +1/+1 counter on a Dragon you control of that
/// color. If you put +1/+1 counters on five Dragons this way, you win the
/// game." (Call the Spirit Dragons): one non-target choice and placement per
/// color (W, U, B, R, G in order), then a gate on the number of distinct
/// objects that received counters across all five choices.
pub fn parse_for_each_color_counter_choice_then_distinct_count(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let first_tokens = trim_commas(first.lowered());
    let words = crate::lexer::token_word_refs(&first_tokens);
    if words.len() < 10
        || !words.starts_with(&["for", "each", "color"])
        || !words.ends_with(&["of", "that", "color"])
    {
        return Ok(None);
    }
    let first_view = crate::lexer::TokenWordView::new(&first_tokens);
    let starts = first_view.token_start_indices();
    let (Some(&body_start), Some(&body_end)) = (starts.get(3), starts.get(words.len() - 3)) else {
        return Ok(None);
    };
    let body = trim_commas(&first_tokens[body_start..body_end]);
    let body_words = crate::lexer::token_word_refs(&body);
    let body_view = crate::lexer::TokenWordView::new(&body);
    let body_starts = body_view.token_start_indices();
    if body_words.first() != Some(&"put") {
        return Ok(None);
    }
    let Some(on) = body_words.iter().position(|word| *word == "on") else {
        return Ok(None);
    };
    if on < 2 || !body_words.get(on + 1).is_some_and(|word| matches!(*word, "a" | "an")) {
        return Ok(None);
    }
    let (Some(&descriptor_start), Some(&descriptor_end), Some(&recipient_start)) =
        (body_starts.get(1), body_starts.get(on), body_starts.get(on + 2))
    else {
        return Ok(None);
    };
    let (count, counter_type) = crate::effect_sentences::zone_counter_helpers::parse_counter_descriptor(
        &body[descriptor_start..descriptor_end],
    )?;
    let recipient = parse_object_filter_lexed(&body[recipient_start..], false)?;

    let second_tokens = trim_commas(second.lowered());
    let second_words = crate::lexer::token_word_refs(&second_tokens);
    if !second_words.starts_with(&["if", "you", "put"]) {
        return Ok(None);
    }
    let Some(second_on) = second_words.iter().position(|word| *word == "on") else {
        return Ok(None);
    };
    let Some(this_way) = second_words
        .windows(2)
        .position(|pair| pair == ["this", "way"])
    else {
        return Ok(None);
    };
    let Some(required) = second_words
        .get(second_on + 1)
        .and_then(|word| crate::util::parse_number_word_i32(word))
    else {
        return Ok(None);
    };
    if this_way <= second_on + 2 {
        return Ok(None);
    }
    let second_view = crate::lexer::TokenWordView::new(&second_tokens);
    let second_starts = second_view.token_start_indices();
    let Some(&consequence_start) = second_starts.get(this_way + 2) else {
        return Ok(None);
    };
    let consequence = trim_commas(&second_tokens[consequence_start..]);
    if consequence.is_empty() {
        return Ok(None);
    }
    let consequence_effects = effect_sentences::parse_effect_sentences_lexed(&consequence)?;

    let colors = [
        ("white", crate::color::ColorSet::WHITE),
        ("blue", crate::color::ColorSet::BLUE),
        ("black", crate::color::ColorSet::BLACK),
        ("red", crate::color::ColorSet::RED),
        ("green", crate::color::ColorSet::GREEN),
    ];
    let mut effects = Vec::new();
    let mut received = ObjectFilter::default();
    received.zone = Some(Zone::Battlefield);
    for (name, color) in colors {
        let tag = helper_tag_for_tokens(first.lowered(), &format!("color_counter_choice_{name}"));
        let mut filter = recipient.clone();
        filter.colors = Some(color);
        if filter.zone.is_none() {
            filter.zone = Some(Zone::Battlefield);
        }
        effects.push(EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        }));
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(count as i32),
            TargetAst::Tagged(tag.clone(), None),
            None,
            false,
        ));
        received.any_of.push(ObjectFilter::tagged(tag));
    }
    effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate: PredicateAst::ValueComparison {
            left: Value::Count(received),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(required),
        },
        if_true: consequence_effects,
        if_false: Vec::new(),
    }));
    Ok(Some(effects))
}

/// "Each player rolls a d20. If one or more opponents had the highest
/// result, this creature can't attack those players or planeswalkers they
/// control this combat." (Chaos Dragon): every player rolls, the highest
/// rollers matching the quantified group are tagged, and the restriction is
/// applied for each of them.
pub fn parse_each_player_rolls_then_highest_result_attack_restriction(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let first_tokens = trim_commas(first.lowered());
    let first_words = crate::lexer::token_word_refs(&first_tokens);
    let ["each", "player", "rolls", "a", die] = first_words.as_slice() else {
        return Ok(None);
    };
    let Some(sides) = die.strip_prefix('d').and_then(|sides| sides.parse::<u32>().ok()) else {
        return Ok(None);
    };
    let second_tokens = trim_commas(second.lowered());
    let words = crate::lexer::token_word_refs(&second_tokens);
    let (keep, rest) = match words.as_slice() {
        ["if", "one", "or", "more", "opponents", "had", "the", "highest", "result", rest @ ..] => {
            (PlayerFilter::Opponent, rest)
        }
        ["if", "one", "or", "more", "players", "had", "the", "highest", "result", rest @ ..] => {
            (PlayerFilter::Any, rest)
        }
        _ => return Ok(None),
    };
    let (attackers, rest) = match rest {
        ["this", "creature" | "permanent", rest @ ..] => (ObjectFilter::source(), rest),
        _ => return Ok(None),
    };
    let duration = match rest {
        [
            "can't" | "cant",
            "attack",
            "those",
            "players",
            "or",
            "planeswalkers",
            "they",
            "control",
            "this",
            period,
        ] => match *period {
            "combat" => crate::effect::Until::EndOfCombat,
            "turn" => crate::effect::Until::EndOfTurn,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    let tag = helper_tag_for_tokens(first.lowered(), "highest_roll_players");
    Ok(Some(vec![
        EffectAst::RollDieKeepHighestPlayers {
            sides,
            keep,
            tag: tag.clone(),
        },
        EffectAst::ForEach(ForEachEffectAst::ForEachTaggedPlayer {
            tag,
            effects: vec![EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack_player_or_planeswalkers_controlled_by(
                    attackers,
                    PlayerFilter::IteratedPlayer,
                ),
                duration,
                None,
            )],
            require_evidence: false,
        }),
    ]))
}

/// "Create a token that's a copy of a card you exiled with cards named
/// <name>. X is the mana value of that card." (Caller of the Untamed): the
/// activator picks one linked exiled card whose mana value is the announced
/// X, then copies it.
pub fn parse_token_copy_of_chosen_card_with_x_mana_value(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let second_tokens = trim_commas(second.lowered());
    let second_words = crate::lexer::token_word_refs(&second_tokens);
    if !matches!(
        second_words.as_slice(),
        ["x", "is", "the", "mana", "value", "of", "that", "card" | "creature" | "permanent"]
            | ["x", "is", "that", "card's" | "cards", "mana", "value"]
    ) {
        return Ok(None);
    }
    let first_tokens = trim_commas(first.lowered());
    let words = crate::lexer::token_word_refs(&first_tokens);
    const HEAD: usize = 7;
    if words.len() <= HEAD
        || !matches!(
            &words[..HEAD],
            ["create", "a", "token", "that's" | "thats", "a", "copy", "of"]
        )
        || !words.windows(4).any(|window| window == ["exiled", "with", "cards", "named"])
    {
        return Ok(None);
    }
    let first_view = crate::lexer::TokenWordView::new(&first_tokens);
    let starts = first_view.token_start_indices();
    let Some(&filter_start) = starts.get(HEAD) else {
        return Ok(None);
    };
    let mut filter = parse_object_filter_lexed(&first_tokens[filter_start..], false)?;
    if filter.exiled_with_cards_named.is_none() {
        return Ok(None);
    }
    filter.mana_value = Some(crate::filter::Comparison::EqualExpr(Box::new(Value::X)));
    let copy = crate::lexer::synthetic_word_tokens(["create", "a", "token", "that's", "a", "copy", "of", "it"]);
    let mut effects = vec![EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
        filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player: PlayerAst::You,
        tag: crate::tag::CompilerReferenceTag::It.bind(),
    })];
    effects.extend(effect_sentences::parse_effect_sentences_lexed(&copy)?);
    Ok(Some(effects))
}

/// "Each player secretly chooses a creature they control and a creature they
/// don't control. Then those choices are revealed. Destroy each creature
/// chosen this way." (Call to the Void): two hidden selections per player,
/// revealed together, then one simultaneous destruction of every chosen
/// creature.
pub fn parse_each_player_secret_own_and_other_creature_then_destroy(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(first), Some(second), Some(third)) = (
        sentences.get(sentence_idx),
        sentences.get(sentence_idx + 1),
        sentences.get(sentence_idx + 2),
    ) else {
        return Ok(None);
    };
    let words_of = |sentence: &SentenceInput| {
        crate::lexer::token_word_refs(trim_commas(sentence.lowered()).as_slice())
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let first_words = words_of(first);
    let first_words = first_words.iter().map(String::as_str).collect::<Vec<_>>();
    let noun = match first_words.as_slice() {
        [
            "each",
            "player",
            "secretly",
            "chooses",
            "a",
            noun,
            "they",
            "control",
            "and",
            "a",
            other_noun,
            "they",
            "don't" | "dont",
            "control",
        ] if noun == other_noun => *noun,
        _ => return Ok(None),
    };
    let base = match noun {
        "creature" => ObjectFilter::creature(),
        "permanent" => ObjectFilter::permanent(),
        _ => return Ok(None),
    };
    let second_words = words_of(second);
    if second_words != ["then", "those", "choices", "are", "revealed"] {
        return Ok(None);
    }
    let third_words = words_of(third);
    if third_words.as_slice() != ["destroy", "each", noun, "chosen", "this", "way"] {
        return Ok(None);
    }
    let own_tag = helper_tag_for_tokens(first.lowered(), "secret_own_choice");
    let other_tag = helper_tag_for_tokens(first.lowered(), "secret_other_choice");
    let own = base
        .clone()
        .controlled_by(PlayerFilter::IteratedPlayer)
        .in_zone(Zone::Battlefield);
    let other = base
        .clone()
        .controlled_by(PlayerFilter::Excluding {
            base: Box::new(PlayerFilter::Any),
            excluded: Box::new(PlayerFilter::IteratedPlayer),
        })
        .in_zone(Zone::Battlefield);
    let mut destroyed = ObjectFilter::default();
    destroyed.any_of = vec![
        ObjectFilter::tagged(own_tag.clone()),
        ObjectFilter::tagged(other_tag.clone()),
    ];
    Ok(Some(vec![
        // The first selection stays hidden until the second one is made;
        // both are revealed together.
        EffectAst::Votes(VoteEffectAst::SecretChoiceStart {
            options: Vec::new(),
            participants: vec![PlayerFilter::Any],
            object_choice: Some(crate::effects::SecretObjectChoice {
                filter: own,
                count: ChoiceCount::exactly(1),
                tag: own_tag.into(),
                reveal_after_choice: false,
            }),
        }),
        EffectAst::Votes(VoteEffectAst::SecretChoiceStart {
            options: Vec::new(),
            participants: vec![PlayerFilter::Any],
            object_choice: Some(crate::effects::SecretObjectChoice {
                filter: other,
                count: ChoiceCount::exactly(1),
                tag: other_tag.into(),
                reveal_after_choice: true,
            }),
        }),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::Implicit,
            SubjectVerbActionAst::ZoneMoves(crate::cards::builders::ZoneMoveActionAst::DestroyAll {
                filter: destroyed,
                no_regeneration: false,
                creature_destroyed_this_way_surface: false,
            }),
        ),
    ]))
}

#[cfg(test)]
#[path = "reference_linked_inline_secret_object_choice_tests_13.rs"]
mod secret_object_choice_tests;

fn look_at_top_cards_parts(effect: &EffectAst) -> Option<(PlayerAst, Value)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action: SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards { count, .. }),
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone()))
}

fn top_cards_parts_with_reveal(effect: &EffectAst) -> Option<(PlayerAst, Value, bool)> {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: SubjectVerbSubjectAst { player, .. },
        action:
            SubjectVerbActionAst::RevealLook(RevealLookActionAst::LookAtTopCards {
                count, reveal, ..
            }),
    }) = effect
    else {
        return None;
    };
    Some((*player, count.clone(), *reveal))
}

pub(crate) fn tagged_library_candidate_filter(
    candidate: &TagKey,
    excluded: &[crate::tag::TagRef],
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
        TargetAst::Tagged(crate::tag::TagRef::of(tag), None),
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
        effect_grammar::LookedPartitionDestination::Exile => (Zone::Exile, false, None),
        effect_grammar::LookedPartitionDestination::LibraryTop(order) => {
            (Zone::Library, true, Some(order))
        }
        effect_grammar::LookedPartitionDestination::LibraryBottom(order) => {
            (Zone::Library, false, Some(order))
        }
    };
    let mut effect = EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(crate::tag::TagRef::of(tag), None),
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
        EffectAst::subject_verb_look_at_top_cards(
            player,
            count,
            crate::tag::TagRef::of(looked_tag),
        ),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            player,
            tag: crate::tag::TagRef::of(hand_tag.clone()),
            zone: Zone::Library,
        }),
        EffectAst::subject_verb_tag_matching_objects(
            remainder_filter,
            vec![Zone::Library],
            crate::tag::TagRef::of(remainder_tag.clone()),
        ),
        move_looked_partition_group(
            hand_tag.key.clone(),
            effect_grammar::LookedPartitionDestination::Hand,
            player,
        ),
        move_looked_partition_group(remainder_tag.key.clone(), remainder_destination, player),
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
            effect_grammar::LookedCardDisposition::HandAndExile => {
                effect_grammar::LookedPartitionDestination::Exile
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

/// "Look at the top four cards of your library, then put up to two of them
/// back on top of your library in any order and the rest into your
/// graveyard" (Rowan's Grim Search): a counted selection and its exact
/// remainder, each moved to its own destination.
pub fn parse_inline_look_at_top_then_counted_partition(
    look_tokens: &[OwnedLexToken],
    partition_tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let look_tokens = trim_commas(look_tokens);
    let partition_tokens = trim_commas(partition_tokens);
    let (player, count, reveal_top) = parse_top_cards_view_sentence(&look_tokens)?;
    if reveal_top {
        return None;
    }
    let shape = effect_grammar::parse_looked_card_partition_shape(
        crate::lexer::trim_lexed_commas(&partition_tokens),
    )?;
    let looked_tag = helper_tag_for_tokens(&look_tokens, "looked_partition");
    let selected_tag = helper_tag_for_tokens(&partition_tokens, "partition_selected");
    let remainder_tag = helper_tag_for_tokens(&partition_tokens, "partition_remainder");
    Some(vec![
        EffectAst::subject_verb_look_at_top_cards(
            player,
            count,
            crate::tag::TagRef::of(looked_tag.clone()),
        ),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: tagged_library_candidate_filter(&looked_tag, &[]),
            count: shape.selected_count,
            player,
            tag: crate::tag::TagRef::of(selected_tag.clone()),
            zone: Zone::Library,
        }),
        EffectAst::subject_verb_tag_matching_objects(
            tagged_library_candidate_filter(&looked_tag, std::slice::from_ref(&selected_tag)),
            vec![Zone::Library],
            crate::tag::TagRef::of(remainder_tag.clone()),
        ),
        move_looked_partition_group(selected_tag.key.clone(), shape.selected_destination, player),
        move_looked_partition_group(remainder_tag.key.clone(), shape.remainder_destination, player),
    ])
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
        crate::tag::TagRef::of(candidate_tag.clone()),
    )];
    for (index, tag) in chosen_tags.iter().enumerate() {
        effects.push(EffectAst::ObjectChoices(
            ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: tagged_library_candidate_filter(&candidate_tag, &chosen_tags[..index]),
                count: ChoiceCount::exactly(1),
                player,
                tag: crate::tag::TagRef::of(tag.clone()),
                zone: Zone::Library,
            },
        ));
    }
    for (tag, destination) in chosen_tags.into_iter().zip(destinations) {
        effects.push(move_tagged_to_looked_destination(
            tag.key.clone(),
            destination,
        ));
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
        all_matching: false,
        fixed_direction: None,
        controller_chooses: false,
    }]))
}

/// "Choose left or right. Each player gains control of all <objects>
/// controlled by the next player in the chosen direction." (Aminatou, the
/// Fateshifter): every player simultaneously takes the matching objects of
/// the neighbour in the chosen direction.
pub fn parse_directional_adjacent_player_control_all(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (Some(choice), Some(gain)) = (sentences.get(sentence_idx), sentences.get(sentence_idx + 1))
    else {
        return Ok(None);
    };
    let choice_words = crate::lexer::token_word_refs(choice.lowered());
    if choice_words.as_slice() != ["choose", "left", "or", "right"] {
        return Ok(None);
    }
    let gain_tokens = gain.lowered();
    let gain_words = crate::lexer::token_word_refs(gain_tokens);
    const PREFIX: &[&str] = &["each", "player", "gains", "control", "of", "all"];
    const SUFFIX: &[&str] = &[
        "controlled", "by", "the", "next", "player", "in", "the", "chosen", "direction",
    ];
    if gain_words.len() <= PREFIX.len() + SUFFIX.len()
        || !gain_words.starts_with(PREFIX)
        || !gain_words.ends_with(SUFFIX)
    {
        return Ok(None);
    }
    let view = crate::lexer::TokenWordView::new(gain_tokens);
    let Some(object_range) =
        view.token_span_for_words(PREFIX.len(), gain_words.len() - SUFFIX.len())
    else {
        return Ok(None);
    };
    let object_tokens = trim_commas(&gain_tokens[object_range]);
    let Ok(filter) = parse_object_filter_lexed(&object_tokens, false) else {
        return Ok(None);
    };
    Ok(Some(vec![
        EffectAst::subject_verb_choose_named_option(
            PlayerAst::You,
            vec!["left".to_string(), "right".to_string()],
        ),
        EffectAst::DirectionalAdjacentPlayerControl {
            filter,
            left_option: "left".to_string(),
            right_option: "right".to_string(),
            all_matching: true,
            fixed_direction: None,
            controller_chooses: false,
        },
    ]))
}

/// "Each player gains control of a nonland permanent of your choice
/// controlled by the player to their right." (Inniaz, the Gale Force): one
/// object per player, chosen by you from the matching objects of that
/// player's fixed-direction neighbour, all gained at once.
pub fn parse_fixed_direction_adjacent_player_control(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::lexer::token_word_refs(tokens);
    const PREFIX: &[&str] = &["each", "player", "gains", "control", "of"];
    if !words.starts_with(PREFIX) {
        return Ok(None);
    }
    let (direction, suffix_len) = match words.as_slice() {
        [.., "controlled", "by", "the", "player", "to", "their", "right"] => {
            (ironsmith_core::SeatingDirection::Right, 7)
        }
        [.., "controlled", "by", "the", "player", "to", "their", "left"] => {
            (ironsmith_core::SeatingDirection::Left, 7)
        }
        _ => return Ok(None),
    };
    let object_words = &words[PREFIX.len()..words.len() - suffix_len];
    let (controller_chooses, choice_len) = match object_words {
        [.., "of", "your", "choice"] => (true, 3),
        [.., "of", "their", "choice"] => (false, 3),
        _ => return Ok(None),
    };
    let object_end = PREFIX.len() + object_words.len() - choice_len;
    if !matches!(object_words.first(), Some(&"a" | &"an")) || object_end <= PREFIX.len() + 1 {
        return Ok(None);
    }
    let view = crate::lexer::TokenWordView::new(tokens);
    let Some(object_range) = view.token_span_for_words(PREFIX.len() + 1, object_end) else {
        return Ok(None);
    };
    let Ok(filter) = parse_object_filter_lexed(&trim_commas(&tokens[object_range]), false) else {
        return Ok(None);
    };
    Ok(Some(EffectAst::DirectionalAdjacentPlayerControl {
        filter,
        left_option: "left".to_string(),
        right_option: "right".to_string(),
        all_matching: false,
        fixed_direction: Some(direction),
        controller_chooses,
    }))
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
                filter_source_snapshots: false,
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
                filter_source_snapshots: false,
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
                SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                    set_quantifier_surface,
                    ..
                }),
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
        effects.push(EffectAst::Permissions(PermissionEffectAst::May {
            effects: parts.effects,
        }));
    } else {
        effects.extend(parts.effects);
    }
    if gate_on_result || optional {
        effects.push(EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: followups,
        }));
    } else {
        effects.extend(followups);
    }
    if gate_on_previous_result {
        vec![EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects,
        })]
    } else {
        effects
    }
}

#[cfg(test)]
#[path = "reference_linked_inline_optional_consult_gating_tests_14.rs"]
mod optional_consult_gating_tests;

fn restrict_target_controller(target: TargetAst, controller: PlayerFilter) -> TargetAst {
    match target {
        TargetAst::Object(mut filter, target_span, it_span) => {
            filter.controller = Some(controller);
            TargetAst::Object(filter, target_span, it_span)
        }
        TargetAst::WithCount(inner, count) => {
            TargetAst::WithCount(Box::new(restrict_target_controller(*inner, controller)), count)
        }
        TargetAst::WithCountValue(inner, count, value) => TargetAst::WithCountValue(
            Box::new(restrict_target_controller(*inner, controller)),
            count,
            value,
        ),
        other => other,
    }
}

/// "Put a -1/-1 counter on the other." (Retribution): one counter of a named
/// kind on the target-set member the controller did not choose.
fn put_counter_on_other_counter_type(tokens: &[OwnedLexToken]) -> Option<crate::object::CounterType> {
    let words = crate::lexer::parser_token_word_refs(tokens);
    let words: Vec<&str> = words.into_iter().filter(|word| *word != ".").collect();
    let ["put", "a" | "an" | "one", rest @ ..] = words.as_slice() else {
        return None;
    };
    let counter_idx = rest.iter().position(|word| *word == "counter")?;
    if !matches!(&rest[counter_idx + 1..], ["on", "the", "other"] | ["on", "other"]) {
        return None;
    }
    crate::grammar::filters::parse_counter_type_words(&rest[..=counter_idx])
}

#[cfg(test)]
mod same_controller_counter_program_tests {
    use super::*;

    #[test]
    fn target_pair_sacrifice_and_counter_other_form_one_program() {
        let sentences: Vec<_> = [
            "Choose two target creatures controlled by the same opponent.",
            "That player chooses and sacrifices one of those creatures.",
            "Put a -1/-1 counter on the other.",
        ].into_iter().enumerate().map(|(line, text)| {
            SentenceInput::from_lexed(&crate::lexer::lex_line(text, line).unwrap())
        }).collect();
        assert!(effect_grammar::parse_same_controller_sacrifice_shape(
            sentences[0].lowered(), sentences[1].lowered(),
        ).is_some(), "target and sacrifice sentences must retain their shared controller: {:?} / {:?}", crate::lexer::parser_token_word_refs(sentences[0].lowered()), crate::lexer::parser_token_word_refs(sentences[1].lowered()));
        assert_eq!(put_counter_on_other_counter_type(sentences[2].lowered()), Some(crate::object::CounterType::MinusOneMinusOne), "counter sentence: {:?}", crate::lexer::parser_token_word_refs(sentences[2].lowered()));
        let effects = parse_choose_same_controller_targets_then_sacrifice_one_return_other(&sentences, 0)
            .unwrap().expect("one shared target-pair program");
        assert_eq!(effects.len(), 5, "target declaration, snapshot, choice, sacrifice, counter");
        let document = crate::lexer::lex_line("Choose two target creatures controlled by the same opponent. That player chooses and sacrifices one of those creatures. Put a -1/-1 counter on the other.", 0).unwrap();
        effect_sentences::parse_effect_sentences_lexed(&document).expect("the document entry must preserve the target-pair procedure");
    }
}

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

    face_down_pile_effects(first_tokens, shape).map(Some)
}

/// "Exile <target> and the top N cards of <library> in a face-down pile,
/// shuffle that pile, then cloak/manifest those cards." as one sentence.
pub fn parse_exile_face_down_pile_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = effect_grammar::parse_face_down_pile_sentence_shape(tokens) else {
        // "Exile <objects> in a face-down pile, shuffle that pile, then
        // manifest those cards." (Ghastly Conscription): the exiled objects
        // alone form the shuffled pile that enters face down (CR 701.40a/e).
        let Some((target_tokens, manifest)) =
            effect_grammar::parse_object_face_down_pile_sentence_shape(tokens)
        else {
            return Ok(None);
        };
        let target = effect_sentences::parse_target_phrase(target_tokens)?;
        let pile_tag = helper_tag_for_tokens(tokens, "face_down_pile");
        let pile = TargetAst::Tagged(crate::tag::TagRef::of(pile_tag.clone()), None);
        let entry = if manifest {
            EffectAst::subject_verb_manifest_onto_battlefield(
                PlayerAst::You,
                pile,
                false,
                ReturnControllerAst::You,
                true,
            )
        } else {
            EffectAst::subject_verb_cloak_onto_battlefield(
                PlayerAst::You,
                pile,
                false,
                ReturnControllerAst::You,
                true,
            )
        };
        // The pile producer preserves an authored universal selection.
        // A bare Object target would ask for one card at resolution.
        let exile = match target {
            TargetAst::Object(filter, _, _)
                if filter.set_quantifier_surface()
                    == Some(ironsmith_core::SetQuantifierSurface::All) =>
            {
                EffectAst::subject_verb_exile_all(filter, true)
            }
            target => EffectAst::subject_verb_exile(target, true),
        };
        return Ok(Some(vec![
            EffectAst::TagAffected {
                effect: Box::new(exile),
                tag: crate::tag::TagRef::of(pile_tag),
            },
            entry,
        ]));
    };
    face_down_pile_effects(tokens, shape).map(Some)
}

/// The named object's exile establishes the pile, the library exile appends
/// to it, and the shuffled pile then enters face down (CR 701.58a cloak,
/// CR 701.40a manifest; CR 701.40e: one at a time).
fn face_down_pile_effects(
    first_tokens: &[OwnedLexToken],
    shape: effect_grammar::CloakPileSequenceShape<'_>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let target = effect_sentences::parse_target_phrase(shape.target_tokens)?;
    let pile_tag = helper_tag_for_tokens(first_tokens, "cloak_pile");
    let target_exile = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_exile(target, true)),
        tag: crate::tag::TagRef::of(pile_tag.clone()),
    };
    let pile = TargetAst::Tagged(crate::tag::TagRef::of(pile_tag.clone()), None);
    let entry = if shape.manifest {
        EffectAst::subject_verb_manifest_onto_battlefield(
            PlayerAst::You,
            pile,
            shape.enters_tapped,
            ReturnControllerAst::You,
            true,
        )
    } else {
        EffectAst::subject_verb_cloak_onto_battlefield(
            PlayerAst::You,
            pile,
            shape.enters_tapped,
            ReturnControllerAst::You,
            true,
        )
    };

    Ok(vec![
        target_exile,
        EffectAst::subject_verb_exile_top_of_library_face_down(
            shape.library_owner,
            shape.library_count,
            crate::tag::TagRef::of(pile_tag),
        ),
        entry,
    ])
}

/// "Exile <target> and the top N cards of <library> in a face-down pile. If
/// you do, shuffle that pile and put it back on top of your library."
///
/// The named object's exile establishes the pile; the library exile then
/// appends to it and the "if you do" gate observes the pile exile.
pub fn parse_exile_face_down_pile_then_restack(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = sentences[sentence_idx].lowered();
    let Some(shape) = effect_grammar::parse_face_down_pile_restack_shape(
        first_tokens,
        sentences[sentence_idx + 1].lowered(),
    ) else {
        return Ok(None);
    };

    let target = effect_sentences::parse_target_phrase(shape.target_tokens)?;
    let pile_tag = helper_tag_for_tokens(first_tokens, "face_down_pile");
    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_exile(target, true)),
            tag: crate::tag::TagRef::of(pile_tag.clone()),
        },
        EffectAst::subject_verb_exile_top_of_library_face_down(
            shape.library_owner,
            shape.library_count,
            crate::tag::TagRef::of(pile_tag.clone()),
        ),
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(crate::tag::TagRef::of(pile_tag), None),
                    crate::zone::Zone::Library,
                    true,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )
                .with_library_order(Some(LibraryBottomOrderAst::Random), PlayerAst::You),
            ],
        }),
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
        EffectAst::subject_verb_look_at_top_cards(
            PlayerAst::You,
            count,
            crate::tag::TagRef::of(looked_tag.clone()),
        ),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: shape.count,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(selected_tag.clone()),
            zone: Zone::Library,
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(selected_tag.clone()),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        }),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            crate::tag::TagRef::of(looked_tag),
            Some(crate::tag::TagRef::of(selected_tag)),
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
            EffectAst::subject_verb_look_at_top_cards(
                library_owner,
                count,
                crate::tag::TagRef::of(looked_tag.clone()),
            ),
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
                filter: selected_filter,
                count: shape.selected_count,
                player: PlayerAst::You,
                tag: crate::tag::TagRef::of(selected_tag.clone()),
                zone: Zone::Library,
            }),
            EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
                tag: crate::tag::TagRef::of(selected_tag.clone()),
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
            }),
            EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                crate::tag::TagRef::of(looked_tag),
                Some(crate::tag::TagRef::of(selected_tag)),
                remainder_order,
                library_owner,
            ),
        ];
        return if gate_on_previous_result {
            Ok(Some(vec![EffectAst::Conditionals(
                ConditionalEffectAst::IfResult {
                    predicate: IfResultPredicate::Did,
                    effects,
                },
            )]))
        } else {
            Ok(Some(effects))
        };
    }

    let remainder_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "partition_remainder");
    let remainder_filter =
        tagged_library_candidate_filter(&looked_tag, std::slice::from_ref(&selected_tag));

    let effects = vec![
        EffectAst::subject_verb_look_at_top_cards(
            library_owner,
            count,
            crate::tag::TagRef::of(looked_tag.clone()),
        ),
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseTaggedObjectsInZone {
            filter: selected_filter,
            count: shape.selected_count,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(selected_tag.clone()),
            zone: Zone::Library,
        }),
        EffectAst::subject_verb_tag_matching_objects(
            remainder_filter,
            vec![Zone::Library],
            crate::tag::TagRef::of(remainder_tag.clone()),
        ),
        move_looked_partition_group(
            selected_tag.key.clone(),
            shape.selected_destination,
            library_owner,
        ),
        move_looked_partition_group(
            remainder_tag.key.clone(),
            shape.remainder_destination,
            library_owner,
        ),
    ];
    if gate_on_previous_result {
        Ok(Some(vec![EffectAst::Conditionals(
            ConditionalEffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects,
            },
        )]))
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

    let third = sentences[sentence_idx + 2].lowered();
    let returns_other = effect_grammar::is_return_other_to_owner_hand_shape(third);
    let other_counter = if returns_other { None } else { put_counter_on_other_counter_type(third) };
    if !returns_other && other_counter.is_none() {
        return Ok(None);
    }

    let mut other_filter = ObjectFilter::tagged(target_set_tag);
    other_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag,
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });
    if let Some(counter_type) = other_counter {
        effects.push(EffectAst::subject_verb_put_counters(
            counter_type,
            Value::Fixed(1),
            TargetAst::Object(other_filter, None, None),
            None,
            false,
        ));
        return Ok(Some(effects));
    }
    effects.push(EffectAst::subject_verb_return_to_hand(
        TargetAst::Object(other_filter, None, None),
        false,
    ));
    Ok(Some(effects))
}

/// "Choose two target creatures controlled by the same opponent. That player
/// chooses one of those creatures. <Source> deals N damage to that creature,
/// and the other can't block this turn." (Trial of Agony)
pub fn parse_choose_same_controller_targets_then_damage_chosen_other_cant_block(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(third) = sentences.get(sentence_idx + 2) else {
        return Ok(None);
    };
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(shape) =
        effect_grammar::parse_same_controller_choose_one_shape(&first_tokens, &second_tokens)
    else {
        return Ok(None);
    };
    let third_words: Vec<&str> = crate::lexer::parser_token_word_refs(third.lowered())
        .into_iter()
        .filter(|word| !matches!(*word, "." | ","))
        .collect();
    let Some(deals_idx) = third_words.iter().position(|word| *word == "deals") else {
        return Ok(None);
    };
    if !crate::util::is_source_reference_words(&third_words[..deals_idx]) {
        return Ok(None);
    }
    let [amount_word, "damage", "to", "that", "creature", "and", "the", "other", "can't" | "cant", "block", "this", "turn"] =
        &third_words[deals_idx + 1..]
    else {
        return Ok(None);
    };
    let Some(amount) = crate::util::parse_number_word_u32(amount_word) else {
        return Ok(None);
    };

    let mut target = mark_target_set_same_controller(effect_sentences::parse_target_phrase(
        &trim_commas(&first_tokens[shape.target]),
    )?);
    if crate::lexer::token_word_refs(&first_tokens)
        .windows(2)
        .any(|pair| pair == ["same", "opponent"])
    {
        target = restrict_target_controller(target, PlayerFilter::Opponent);
    }
    let TargetAst::WithCount(_, target_count) = &target else {
        return Ok(None);
    };
    if target_count.min != 2 || target_count.max != Some(2) || target_count.is_random() {
        return Ok(None);
    }
    let target_set_tag = helper_tag_for_tokens(&first_tokens, "target_set");
    let chosen_tag = helper_tag_for_tokens(&second_tokens, "chosen");
    let mut other_filter = ObjectFilter::tagged(target_set_tag.clone());
    other_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: chosen_tag.key.clone(),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    Ok(Some(vec![
        EffectAst::subject_verb_target_only(target),
        EffectAst::SnapshotLastObjectTag {
            into: crate::tag::TagRef::of(target_set_tag.clone()),
        },
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter: ObjectFilter::tagged(target_set_tag),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::ItsController,
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
        }),
        EffectAst::subject_verb_damage(
            Value::Fixed(amount as i32),
            TargetAst::Object(ObjectFilter::tagged(chosen_tag), None, None),
        ),
        EffectAst::subject_verb_cant(
            crate::effect::Restriction::block(other_filter),
            crate::effect::Until::EndOfTurn,
            None,
        ),
    ]))
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
    let mut target = mark_target_set_same_controller(effect_sentences::parse_target_phrase(
        &trim_commas(&first_tokens[shape.target]),
    )?);
    // "controlled by the same opponent" (Retribution): the shared controller
    // must also be an opponent.
    if crate::lexer::token_word_refs(&first_tokens)
        .windows(2)
        .any(|pair| pair == ["same", "opponent"])
    {
        target = restrict_target_controller(target, PlayerFilter::Opponent);
    }
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
                into: crate::tag::TagRef::of(target_set_tag.clone()),
            },
            EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter: ObjectFilter::tagged(target_set_tag.clone()),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::ItsController,
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
            }),
            EffectAst::subject_verb_sacrifice(
                PlayerAst::That,
                ObjectFilter::tagged(chosen_tag.clone()),
                1,
                None,
            ),
        ],
        target_set_tag.key.clone(),
        chosen_tag.key.clone(),
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
        // Every other card the chooser holds that matches the chosen set's
        // description (CR 701.9a discards from that player's hand).
        effect_grammar::RestActionShape::Discard => EffectAst::subject_verb_discard(
            player,
            Value::Count(filter.clone()),
            false,
            false,
            Some(filter),
            None,
        ),
    }
}

pub(crate) fn append_rest_action_after_choice(
    effect: EffectAst,
    action: effect_grammar::RestActionShape,
) -> Option<Vec<EffectAst>> {
    match effect {
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            tag,
            count,
            count_value,
            player,
        }) => {
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    tag,
                    count,
                    count_value,
                    player,
                }),
                rest_action_effect(action, rest_filter, player),
            ])
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects }) => {
            let [inner] = effects.as_slice() else {
                return None;
            };
            let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                tag,
                count,
                count_value,
                player,
            }) = inner.clone()
            else {
                return None;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachPlayer {
                effects: vec![
                    EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                        filter,
                        tag,
                        count,
                        count_value,
                        player,
                    }),
                    rest_action_effect(action, rest_filter, player),
                ],
            })])
        }
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects }) => {
            let [inner] = effects.as_slice() else {
                return None;
            };
            let EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                filter,
                tag,
                count,
                count_value,
                player,
            }) = inner.clone()
            else {
                return None;
            };
            let rest_filter = filter.clone().not_tagged(tag.clone());
            Some(vec![EffectAst::ForEach(
                ForEachEffectAst::ForEachOpponent {
                    effects: vec![
                        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                            filter,
                            tag,
                            count,
                            count_value,
                            player,
                        }),
                        rest_action_effect(action, rest_filter, player),
                    ],
                },
            )])
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
    let rider_words = crate::lexer::token_word_refs(&second);
    let cast_this_way = crate::word_primitives::sequence_occurs(
        &rider_words,
        &["cast", "this", "way"],
    );
    let cast_from_your_graveyard = crate::word_primitives::sequence_occurs(
        &rider_words,
        &["cast", "from", "your", "graveyard", "this", "way"],
    );
    graveyard_cast_with_exile_replacement_surface(
        &first, &shape, cast_this_way, cast_from_your_graveyard,
    )
}

/// The effects of a graveyard-cast permission whose spell is exiled instead
/// of going to a graveyard: the targeted card, the optional cast, and the
/// replacement bound to the cast spell.
pub fn graveyard_cast_with_exile_replacement(
    first: &[OwnedLexToken],
    shape: &effect_grammar::GraveyardCastReplacementShape,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    graveyard_cast_with_exile_replacement_surface(first, shape, false, false)
}

/// `cast_this_way` records the rider wording "If an instant or sorcery spell
/// cast this way would be put into your graveyard, exile it instead".
pub fn graveyard_cast_with_exile_replacement_surface(
    first: &[OwnedLexToken],
    shape: &effect_grammar::GraveyardCastReplacementShape,
    cast_this_way: bool,
    cast_from_your_graveyard: bool,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    graveyard_cast_with_exile_replacement_and_surcharge(
        first, shape, cast_this_way, cast_from_your_graveyard, None,
    )
}

/// `unless_targets_surcharge` is "If that spell doesn't target <filter>, it
/// costs {N} more to cast this way", carried on the turn-long permission.
pub fn graveyard_cast_with_exile_replacement_and_surcharge(
    first: &[OwnedLexToken],
    shape: &effect_grammar::GraveyardCastReplacementShape,
    cast_this_way: bool,
    cast_from_your_graveyard: bool,
    unless_targets_surcharge: Option<(crate::mana::ManaCost, ObjectFilter)>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Keep the participant binder around the entire target/cast/replacement
    // procedure. Its target's "that player" is local to this iteration.
    let view = crate::lexer::TokenWordView::new(first);
    if view.word_refs().starts_with(&["for", "each", "opponent"]) {
        let start = view.token_index_after_words(3).unwrap_or(first.len());
        let body = trim_commas(&first[start..]);
        return Ok(graveyard_cast_with_exile_replacement_and_surcharge(
            &body, shape, cast_this_way, cast_from_your_graveyard,
            unless_targets_surcharge,
        )?.map(|effects| vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })]));
    }
    // "you may cast a spell from each opponent's graveyard without paying its
    // mana cost. If a spell cast this way would be put into a graveyard,
    // exile it instead." (Jetsam): for each opponent, one optional spell
    // chosen from that player's graveyard; the spell cast that way is exiled
    // instead of going to a graveyard.
    let each_opponent_words = view.word_refs();
    if let [
        "you", "may", "cast", "a", "spell", "from", "each", "opponent's" | "opponents", "graveyard",
        rest @ ..,
    ] = each_opponent_words.as_slice()
    {
        if !(rest.is_empty() || rest == ["without", "paying", "its", "mana", "cost"])
            || shape.until_end_of_turn
            || unless_targets_surcharge.is_some()
            || shape.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
            || shape.additional_mana_cost.is_some()
        {
            return Ok(None);
        }
        let chosen_tag = helper_tag_for_tokens(first, "graveyard_cast_target");
        let cast_spell_tag = helper_tag_for_tokens(first, "cast_spell");
        let mut filter = ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::IteratedPlayer);
        filter.excluded_card_types.push(CardType::Land);
        let replacement_filter = ObjectFilter::spell()
            .match_tagged(cast_spell_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
        let replacement = EffectAst::subject_verb_register_future_zone_replacement(
            replacement_filter,
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
            crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
            false,
        );
        let replacement = if cast_this_way {
            replacement.with_cast_this_way_replacement_surface()
        } else {
            replacement
        };
        return Ok(Some(vec![EffectAst::ForEach(ForEachEffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
                    filter,
                    count: ChoiceCount::up_to(1),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: crate::tag::TagRef::of(chosen_tag.clone()),
                }),
                EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                    player: PlayerAst::You,
                    effects: vec![EffectAst::TagAffected {
                        effect: Box::new(
                            EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                                crate::tag::TagRef::of(chosen_tag),
                                PlayerAst::You,
                                false,
                                false,
                                shape.without_paying_mana_cost,
                                None,
                                None,
                                shape.mana_spend_mode,
                            ),
                        ),
                        tag: crate::tag::TagRef::of(cast_spell_tag),
                    }],
                }),
                EffectAst::Conditionals(ConditionalEffectAst::IfResult {
                    predicate: IfResultPredicate::Did,
                    effects: vec![replacement],
                }),
            ],
        })]));
    }
    // "... from your graveyard this turn": the trailing duration is not part
    // of the target description.
    let first = if shape.trailing_this_turn {
        crate::grammar::primitives::strip_lexed_suffix_phrase(first, &["this", "turn"])
            .unwrap_or(first)
    } else {
        first
    };
    if unless_targets_surcharge.is_some() && !shape.until_end_of_turn {
        return Ok(None);
    }
    let chosen_tag = helper_tag_for_tokens(first, "graveyard_cast_target");
    let cast_spell_tag = helper_tag_for_tokens(first, "cast_spell");
    let first_view = crate::lexer::TokenWordView::new(first);
    // "you may cast an artifact, instant, or sorcery spell from your
    // graveyard" (Bilbo, Thief in the Night) chooses the card instead of
    // targeting it; the article after "cast" opens the card description.
    let chosen_not_targeted = first_view.parse_word_position("target").is_none();
    let target_word = if let Some(target_word) = first_view.parse_word_position("target") {
        target_word
    } else if let Some(cast_word) = first_view.parse_word_position("cast")
        && first_view
            .word_refs()
            .get(cast_word + 1)
            .is_some_and(|word| matches!(*word, "a" | "an"))
    {
        cast_word + 1
    } else {
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
    let mut target_filter_tokens = trim_commas(&first[target_start..target_end]);
    if chosen_not_targeted {
        // "an artifact, instant, or sorcery spell from your graveyard" names
        // cards in the graveyard; "spell" is the cast-time noun.
        for token in &mut target_filter_tokens {
            if token.is_word("spell") {
                *token = OwnedLexToken::word("card".to_string(), token.span());
            }
        }
    }
    // "up to one target card ..." (Gale, Waterdeep Prodigy): the single
    // target is optional.
    let up_to_one_target = !chosen_not_targeted
        && target_word >= 3
        && first_view.word_refs()[target_word - 3..target_word] == ["up", "to", "one"];
    // "a card of the other type" relative to the triggering instant or
    // sorcery spell: an instant or sorcery card sharing no card type with it.
    let other_type_start = target_filter_tokens.windows(4).position(|window| {
        window[0].is_word("of")
            && window[1].is_word("the")
            && window[2].is_word("other")
            && window[3].is_word("type")
    });
    if let Some(start) = other_type_start {
        target_filter_tokens.drain(start..start + 4);
    }
    let Ok(mut filter) = parse_object_filter_lexed(&target_filter_tokens, false) else {
        return Ok(None);
    };
    if other_type_start.is_some() {
        if !filter.card_types.is_empty() {
            return Ok(None);
        }
        filter.card_types = vec![CardType::Instant, CardType::Sorcery];
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::Triggering.key(),
            TaggedOpbjectRelation::SharesNoCardTypeWithTagged,
        );
    }
    let spell_card = filter
        .card_types
        .iter()
        .any(|card_type| matches!(card_type, CardType::Instant | CardType::Sorcery));
    // The graveyard's owner may be another player ("from an opponent's
    // graveyard", "from that player's graveyard"); the filter carries it.
    if filter.zone != Some(Zone::Graveyard) || !spell_card {
        return Ok(None);
    }
    let graveyard_card_target = |filter: ObjectFilter| {
        let target = TargetAst::Object(filter, Some(TextSpan::synthetic()), None);
        if up_to_one_target {
            TargetAst::WithCount(Box::new(target), ChoiceCount::up_to(1))
        } else {
            target
        }
    };

    if shape.until_end_of_turn {
        let mut replacement_filter = ObjectFilter::tagged(chosen_tag.clone()).in_zone(Zone::Stack);
        if shape.replacement_to_your_graveyard {
            replacement_filter = replacement_filter.owned_by(PlayerFilter::You);
        }
        let surface = ironsmith_core::GrantPlayTaggedSurface::default()
            .with_leading_duration(!shape.trailing_this_turn)
            .with_object(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCard);
        let mut permission =
            EffectAst::subject_verb_grant_play_tagged_until_end_of_turn_from_current_zone_with_optional_surface(
                crate::tag::TagRef::of(chosen_tag.clone()),
                PlayerAst::You,
                false,
                shape.without_paying_mana_cost,
                false,
                Some(surface),
            );
        if let Some(surcharge) = unless_targets_surcharge
            && let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Grants(GrantActionAst::GrantPlayTaggedUntilEndOfTurn {
                        spell_cost_increase_unless_targets,
                        ..
                    }),
                ..
            }) = &mut permission
        {
            *spell_cost_increase_unless_targets = Some(surcharge);
        }
        return Ok(Some(vec![
            EffectAst::TagAffected {
                effect: Box::new(EffectAst::subject_verb_target_only(graveyard_card_target(
                    filter,
                ))),
                tag: crate::tag::TagRef::of(chosen_tag.clone()),
            },
            permission,
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

    let mut replacement_filter = ObjectFilter::spell().match_tagged(
        cast_spell_tag.clone(),
        TaggedOpbjectRelation::IsTaggedObject,
    );
    if shape.replacement_to_your_graveyard {
        replacement_filter = replacement_filter.owned_by(PlayerFilter::You);
    }

    // CR 601.3e: "cast an instant or sorcery spell with mana value ..."
    // permits only a spell with those characteristics; the face actually
    // proposed for casting is checked, not just the card chosen.
    let required_spell = chosen_not_targeted.then(|| {
        let mut spell = filter.clone();
        spell.zone = None;
        spell.owner = None;
        spell
    });
    let select_card = if chosen_not_targeted {
        // The card may qualify through any face it could be cast as (a split
        // half, an Adventure, a modal DFC's back face); the cast itself then
        // requires the proposed spell to qualify.
        let mut filter = filter;
        filter.castable_face_match = true;
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
        })
    } else {
        EffectAst::TagAffected {
            effect: Box::new(EffectAst::subject_verb_target_only(graveyard_card_target(filter))),
            tag: crate::tag::TagRef::of(chosen_tag.clone()),
        }
    };

    Ok(Some(vec![
        select_card,
        // This shape starts with "you may cast". Preserve the chooser as
        // well as the caster: a triggering event may refer to another player.
        EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
            player: PlayerAst::You,
            effects: vec![EffectAst::TagAffected {
                effect: Box::new({
                    let mut cast =
                        EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                            crate::tag::TagRef::of(chosen_tag),
                            PlayerAst::You,
                            false,
                            false,
                            shape.without_paying_mana_cost,
                            shape.additional_mana_cost.clone(),
                            None,
                            shape.mana_spend_mode,
                        );
                    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action:
                            SubjectVerbActionAst::Stack(
                                crate::cards::builders::StackActionAst::CastTagged {
                                    required_spell: slot,
                                    ..
                                },
                            ),
                        ..
                    }) = &mut cast
                    {
                        *slot = required_spell;
                    }
                    cast
                }),
                tag: crate::tag::TagRef::of(cast_spell_tag),
            }],
        }),
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![{
                let replacement = EffectAst::subject_verb_register_future_zone_replacement(
                    replacement_filter,
                    Some(Zone::Stack),
                    Some(Zone::Graveyard),
                    Zone::Exile,
                    ZoneReplacementDurationAst::OneShot,
                    crate::cards::builders::FutureZoneReplacementCausePolicyAst::Any,
                    false,
                );
                if cast_from_your_graveyard {
                    replacement.with_cast_from_your_graveyard_replacement_surface()
                } else if cast_this_way {
                    replacement.with_cast_this_way_replacement_surface()
                } else {
                    replacement
                }
            }],
        }),
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
#[path = "reference_linked_programs/reference_linked_condition.rs"]
mod reference_linked_condition_programs;
#[path = "reference_linked_programs/reference_linked_counter.rs"]
mod reference_linked_counter_programs;
pub(crate) use reference_linked_condition_programs::append_to_outer_if_result;
#[path = "reference_linked_programs/reference_linked_reference.rs"]
mod reference_linked_reference_programs;
pub(crate) use reference_linked_reference_programs::parse_copy_for_each_target_sentence;
pub use reference_linked_reference_programs::parse_for_each_tagged_copy_then_copy_targets_it;
pub(crate) use reference_linked_reference_programs::retarget_source_self_animate_effect;
use reference_linked_reference_programs::{
    contains_tagged_source_animation, parse_copy_for_each_candidate_filter,
};
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
                action: SubjectVerbActionAst::Stack(StackActionAst::Counter { target, .. }),
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
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            predicate: PredicateAst::TargetMatches(matching_spell),
            if_true: registrations,
            if_false: Vec::new(),
        }),
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
        EffectAst::Delayed(DelayedEffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::subject_verb_return_all_to_battlefield(
                linked_filter,
                false,
                false,
                ReturnControllerAst::Owner,
            )],
        }),
    ]))
}

use super::super::dispatch_entry::{
    consult_cast_effects, consult_stop_rule_is_single_match, find_from_among_looked_cards_phrase,
    leading_may_actor_to_player, parse_consult_cast_clause, parse_consult_remainder_order,
    parse_consult_traversal_sentence, parse_looked_card_choice_filter,
    parse_looked_card_reveal_filter, parse_prefixed_top_of_your_library_count,
};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, ObjectFilter, OwnedLexToken, PlayerAst, PredicateAst,
    ReturnControllerAst, TagKey, TargetAst,
};
use crate::effect::Value;
use crate::runtime_backend::activation_and_restrictions::activated_line_core::contains_word_sequence;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::lexer::TokenWordView;
use crate::runtime_backend::token_index_for_word_index;
use crate::runtime_backend::token_primitives::{
    find_index, parse_leading_may_action_lexed, slice_contains, slice_ends_with, slice_starts_with,
    word_view_has_any_prefix,
};
use crate::runtime_backend::util::trim_commas;
use crate::runtime_backend::util::{helper_tag_for_tokens, is_article};
use crate::target::{ChooseSpec, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

fn looks_like_keyword_bundle_choice_filter(tokens: &[OwnedLexToken]) -> bool {
    let tokens = trim_commas(tokens);
    let words = TokenWordView::new(&tokens).word_refs();
    let mut card_choice_segments = 0usize;
    for idx in 0..words.len().saturating_sub(2) {
        if is_article(words[idx])
            && matches!(words[idx + 1], "card" | "cards")
            && words[idx + 2] == "with"
        {
            card_choice_segments += 1;
            if card_choice_segments >= 2 {
                return true;
            }
        }
    }
    false
}

fn parse_may_put_filtered_card_from_among_into_hand(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
    zone: Zone,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(action_match) = parse_leading_may_action_lexed(&sentence_tokens, &["put"], true)
    else {
        return Ok(None);
    };
    let chooser = leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let action_words = TokenWordView::new(&action_tokens);
    if action_words.is_empty() {
        return Ok(None);
    }
    let action_word_refs = action_words.word_refs();

    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };
    let filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    if looks_like_keyword_bundle_choice_filter(&action_tokens[..filter_end]) {
        return Ok(None);
    }
    let mut filter =
        if let Some(filter) = parse_looked_card_choice_filter(&action_tokens[..filter_end]) {
            filter
        } else {
            return Ok(None);
        };
    filter.zone = Some(zone);

    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    let moves_into_hand =
        slice_starts_with(after_from_words, &["into"]) && slice_contains(after_from_words, &"hand");
    if !moves_into_hand {
        return Ok(None);
    }

    Ok(Some((chooser, filter)))
}

fn retarget_source_self_animate_effect(effect: EffectAst) -> EffectAst {
    match effect {
        EffectAst::BecomeBasePtCreature {
            power,
            toughness,
            target,
            card_types,
            subtypes,
            colors,
            abilities,
            duration,
        } => {
            let target = match target {
                TargetAst::Tagged(tag, span)
                    if tag.as_str() == crate::cards::builders::IT_TAG =>
                {
                    TargetAst::Source(span)
                }
                target => target,
            };
            EffectAst::BecomeBasePtCreature {
                power,
                toughness,
                target,
                card_types,
                subtypes,
                colors,
                abilities,
                duration,
            }
        }
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => EffectAst::Conditional {
            predicate,
            if_true: if_true
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
            if_false: if_false
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
        },
        EffectAst::IfResult { predicate, effects } => EffectAst::IfResult {
            predicate,
            effects: effects
                .into_iter()
                .map(retarget_source_self_animate_effect)
                .collect(),
        },
        other => other,
    }
}

fn contains_triggered_life_gain_effect(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::GainLife { .. } => true,
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            if_true.iter().any(contains_triggered_life_gain_effect)
                || if_false.iter().any(contains_triggered_life_gain_effect)
        }
        EffectAst::IfResult { effects, .. } => {
            effects.iter().any(contains_triggered_life_gain_effect)
        }
        _ => false,
    }
}

fn contains_tagged_source_animation(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::BecomeBasePtCreature {
            target,
            duration,
            ..
        } => {
            let self_animate_target = matches!(
                target,
                TargetAst::Tagged(tag, _) if tag.as_str() == crate::cards::builders::IT_TAG
            ) || matches!(target, TargetAst::Source(_));
            *duration == crate::effect::Until::EndOfTurn && self_animate_target
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            if_true.iter().any(contains_tagged_source_animation)
                || if_false.iter().any(contains_tagged_source_animation)
        }
        EffectAst::IfResult { effects, .. } => {
            effects.iter().any(contains_tagged_source_animation)
        }
        _ => false,
    }
}

fn parse_self_animate_followup_effects(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Ok(effects) = effect_sentences::parse_effect_sentence_lexed(tokens)
        && effects.iter().any(contains_tagged_source_animation)
    {
        return Ok(Some(effects));
    }

    let words = TokenWordView::new(tokens).word_refs();
    if !slice_starts_with(&words, &["if", "this"]) {
        return Ok(None);
    }
    let Some(comma_idx) = find_index(tokens, |token: &OwnedLexToken| token.is_comma()) else {
        return Ok(None);
    };
    let condition_words = TokenWordView::new(&tokens[..comma_idx]).word_refs();
    if !condition_words.contains(&"isnt") || !condition_words.contains(&"creature") {
        return Ok(None);
    }

    let tail = trim_commas(&tokens[comma_idx + 1..]);
    if !TokenWordView::new(&tail)
        .word_refs()
        .first()
        .is_some_and(|word| *word == "it")
    {
        return Ok(None);
    }
    let effects = effect_sentences::parse_effect_sentence_lexed(&tail)?;
    if effects.iter().any(contains_tagged_source_animation) {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

pub(super) fn parse_whenever_gain_life_then_self_animate_source(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first)?;
    if !first_effects.iter().any(contains_triggered_life_gain_effect) {
        return Ok(None);
    }

    let Some(second_effects) = parse_self_animate_followup_effects(second)? else {
        return Ok(None);
    };

    let mut effects = first_effects;
    effects.extend(
        second_effects
            .into_iter()
            .map(retarget_source_self_animate_effect),
    );
    Ok(Some(effects))
}

pub(super) fn parse_gain_life_then_self_animate_source(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first)?;
    if !first_effects.iter().any(contains_triggered_life_gain_effect) {
        return Ok(None);
    }

    let Some(second_effects) = parse_self_animate_followup_effects(second)? else {
        return Ok(None);
    };

    let mut effects = first_effects;
    effects.extend(
        second_effects
            .into_iter()
            .map(retarget_source_self_animate_effect),
    );
    Ok(Some(effects))
}

pub(super) fn parse_choose_then_do_same_for_filter_then_return_to_battlefield(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = effect_sentences::parse_sentence_choose_then_do_same_for_filter(
        sentences[sentence_idx].lowered(),
    )?
    else {
        return Ok(None);
    };

    let second_words: Vec<&str> =
        crate::runtime_backend::token_word_refs(sentences[sentence_idx + 1].lowered())
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
    let tapped = slice_contains(&second_words, &"tapped");
    let second_without_tapped = second_words
        .iter()
        .copied()
        .filter(|word| *word != "tapped")
        .collect::<Vec<_>>();
    if !matches!(
        second_without_tapped.as_slice(),
        ["return", "those", "cards", "to", "battlefield"] | ["return", "them", "to", "battlefield"]
    ) {
        return Ok(None);
    }

    effects.push(EffectAst::ReturnToBattlefield {
        target: TargetAst::Tagged(
            TagKey::from(crate::cards::builders::IT_TAG),
            effect_sentences::span_from_tokens(sentences[sentence_idx + 1].lowered()),
        ),
        tapped,
        transformed: false,
        converted: false,
        controller: ReturnControllerAst::Preserve,
    });
    Ok(Some(effects))
}

pub(super) fn parse_delayed_dies_exile_top_power_choose_play(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    if crate::runtime_backend::grammar::primitives::words_match_prefix(
        &first_tokens,
        &["when", "that", "creature", "dies", "this", "turn"],
    )
    .is_none()
    {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(&first_tokens, |token: &OwnedLexToken| token.is_comma())
    else {
        return Ok(None);
    };
    let action_tokens = trim_commas(&first_tokens[comma_idx + 1..]);
    let action_words: Vec<&str> = crate::runtime_backend::token_word_refs(&action_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let starts_with_exile_top_power = slice_starts_with(
        &action_words,
        &[
            "exile", "number", "of", "cards", "from", "top", "of", "your", "library", "equal",
            "to", "its", "power",
        ],
    );
    let ends_with_choose_exiled =
        slice_ends_with(&action_words, &["choose", "card", "exiled", "this", "way"]);
    if !starts_with_exile_top_power || !ends_with_choose_exiled {
        return Ok(None);
    }

    let second_words: Vec<&str> =
        crate::runtime_backend::token_word_refs(sentences[sentence_idx + 1].lowered())
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
    let is_until_next_turn_play_clause = second_words.as_slice()
        == [
            "until", "end", "of", "your", "next", "turn", "you", "may", "play", "that", "card",
        ];
    if !is_until_next_turn_play_clause {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "chosen");
    let mut exiled_filter = ObjectFilter::default();
    exiled_filter.zone = Some(Zone::Exile);
    exiled_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: None,
        effects: vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                    crate::cards::builders::IT_TAG,
                )))),
                tag: looked_tag.clone(),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(looked_tag, None),
                face_down: false,
            },
            EffectAst::ChooseObjects {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::GrantPlayTaggedUntilYourNextTurn {
                tag: chosen_tag,
                player: PlayerAst::You,
                allow_land: true,
            },
        ],
    }]))
}

pub(super) fn parse_target_gains_flashback_until_eot_with_targets_mana_cost(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_words = crate::runtime_backend::token_word_refs(&first_tokens);
    let Some(gain_idx) = find_index(&first_words, |word| matches!(*word, "gain" | "gains")) else {
        return Ok(None);
    };
    if first_words[gain_idx + 1..] != ["flashback", "until", "end", "of", "turn"] {
        return Ok(None);
    }

    let Some(gain_token_idx) = token_index_for_word_index(&first_tokens, gain_idx) else {
        return Ok(None);
    };
    let target_tokens = trim_commas(&first_tokens[..gain_token_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = effect_sentences::parse_target_phrase(&target_tokens)?;

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = crate::runtime_backend::token_word_refs(&second_tokens);
    let valid_followup = second_words.as_slice()
        == [
            "the",
            "flashback",
            "cost",
            "is",
            "equal",
            "to",
            "its",
            "mana",
            "cost",
        ]
        || second_words.as_slice()
            == [
                "that",
                "cards",
                "flashback",
                "cost",
                "is",
                "equal",
                "to",
                "its",
                "mana",
                "cost",
            ];
    if !valid_followup {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::GrantToTarget {
        target,
        grantable: crate::grant::Grantable::flashback_from_cards_mana_cost(),
        duration: crate::grant::GrantDuration::UntilEndOfTurn,
    }]))
}

pub(super) fn parse_mill_then_may_put_from_among_into_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [EffectAst::Mill { player, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let Some((chooser, filter)) =
        parse_may_put_filtered_card_from_among_into_hand(second, *player, Zone::Graveyard)?
    else {
        return Ok(None);
    };

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
            player: chooser,
            filter,
            reveal: false,
            if_not_chosen: Vec::new(),
        },
    ]))
}

pub(super) fn parse_exile_until_match_grant_play_this_turn(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: crate::cards::builders::LibraryConsultModeAst::Exile,
            stop_rule,
            ..
        }) if consult_stop_rule_is_single_match(&stop_rule)
    ) {
        return Ok(None);
    }

    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag)?);
    Ok(Some(effects))
}

pub(super) fn parse_target_player_chooses_then_other_cant_block(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_target_player_chooses_then_other_cant_block(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(super) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(super) fn parse_choose_creature_type_then_become_type(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_choose_creature_type_then_become_type(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(crate) fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((_, count)) = parse_prefixed_top_of_your_library_count(
        sentences[sentence_idx].lowered(),
        &[
            (&["reveal", "the", "top"][..], ()),
            (&["reveal", "top"][..], ()),
        ],
    ) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = TokenWordView::new(&second_tokens);
    if !word_view_has_any_prefix(&second_words, &[&["put", "all"], &["puts", "all"]]) {
        return Ok(None);
    }
    let second_word_refs = second_words.word_refs();
    let Some(revealed_idx) = second_words.find_phrase_start(&["revealed", "this", "way"]) else {
        return Ok(None);
    };
    if revealed_idx <= 2 {
        return Ok(None);
    }

    let Some(filter_start) = second_words.token_index_for_word_index(2) else {
        return Ok(None);
    };
    let filter_end = second_words
        .token_index_for_word_index(revealed_idx)
        .unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    if looks_like_keyword_bundle_choice_filter(&filter_tokens) {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    let filter_words = crate::runtime_backend::token_word_refs(&filter_tokens);
    if contains_word_sequence(&filter_words, &["chosen", "type"])
        || contains_word_sequence(&filter_words, &["that", "type"])
    {
        filter.chosen_creature_type = true;
    }
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_revealed = &second_word_refs[revealed_idx + 3..];
    let has_hand_clause = contains_word_sequence(after_revealed, &["into", "your", "hand"]);
    let has_rest_clause =
        contains_word_sequence(after_revealed, &["and", "the", "rest", "into", "your"])
            && slice_contains(after_revealed, &"graveyard");
    if !has_hand_clause || !has_rest_clause {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
            player: PlayerAst::You,
            count,
            filter,
        },
    ]))
}

pub(super) fn parse_consult_match_move_and_bottom_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
            ..
        })
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words = crate::runtime_backend::token_word_refs(&second_tokens);
    let (zone, battlefield_tapped) =
        if crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &["put", "that", "card", "into", "your", "hand"],
        )
        .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &["put", "it", "into", "your", "hand"],
            )
            .is_some()
        {
            (Zone::Hand, false)
        } else if crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &[
                "put",
                "that",
                "card",
                "onto",
                "the",
                "battlefield",
                "tapped",
            ],
        )
        .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &["put", "it", "onto", "the", "battlefield", "tapped"],
            )
            .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &["put", "that", "card", "onto", "battlefield", "tapped"],
            )
            .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &["put", "it", "onto", "battlefield", "tapped"],
            )
            .is_some()
        {
            (Zone::Battlefield, true)
        } else if crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &["put", "that", "card", "onto", "the", "battlefield"],
        )
        .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &["put", "it", "onto", "the", "battlefield"],
            )
            .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &["put", "that", "card", "onto", "battlefield"],
            )
            .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &["put", "it", "onto", "battlefield"],
            )
            .is_some()
        {
            (Zone::Battlefield, false)
        } else {
            return Ok(None);
        };

    if !crate::runtime_backend::grammar::primitives::contains_word(&second_tokens, "rest")
        && !crate::runtime_backend::grammar::primitives::contains_word(&second_tokens, "other")
    {
        return Ok(None);
    }
    let Some(order) = parse_consult_remainder_order(&second_words) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.match_tag.clone(), None),
        zone,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped,
        attached_to: None,
    });
    effects.push(EffectAst::PutTaggedRemainderOnBottomOfLibrary {
        tag: parts.all_tag,
        keep_tagged: Some(parts.match_tag),
        order,
        player: parts.player,
    });
    Ok(Some(effects))
}

pub(super) fn parse_consult_match_move_all_to_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = crate::runtime_backend::token_word_refs(&second_tokens);
    let puts_all = slice_starts_with(&second_words, &["put", "all"])
        || slice_starts_with(&second_words, &["puts", "all"])
        || slice_starts_with(&second_words, &["that", "player", "puts", "all"]);
    if !puts_all {
        return Ok(None);
    }
    if !contains_word_sequence(&second_words, &["revealed", "this", "way"])
        || !slice_contains(&second_words, &"graveyard")
    {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.all_tag, None),
        zone: Zone::Graveyard,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    });
    Ok(Some(effects))
}

pub(super) fn parse_consult_match_into_hand_exile_others(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
            ..
        })
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let moves_to_hand = crate::runtime_backend::grammar::primitives::words_match_prefix(
        &second_tokens,
        &["put", "that", "card", "into", "your", "hand"],
    )
    .is_some()
        || crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &["put", "it", "into", "your", "hand"],
        )
        .is_some();
    let exiles_rest =
        crate::runtime_backend::grammar::primitives::contains_word(&second_tokens, "exile")
            && crate::runtime_backend::grammar::primitives::contains_word(&second_tokens, "other")
            && crate::runtime_backend::grammar::primitives::contains_word(&second_tokens, "cards");
    if !moves_to_hand || !exiles_rest {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.match_tag.clone(), None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    });
    effects.push(EffectAst::ForEachTagged {
        tag: parts.all_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(parts.match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::Exile {
                target: TargetAst::Tagged(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    None,
                ),
                face_down: false,
            }],
        }],
    });
    Ok(Some(effects))
}

/// Parses the two-sentence pattern:
///   S1: "Reveal cards from the top of your library until you reveal a <filter> card."
///   S2: "Put that card into your hand and all other cards revealed this way into your graveyard."
///
/// This covers cards like Hermit Druid and similar "reveal until, match to hand, rest to graveyard"
/// patterns.
pub(super) fn parse_consult_match_into_hand_others_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::ConsultTopOfLibrary {
            mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
            ..
        })
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let moves_to_hand = crate::runtime_backend::grammar::primitives::words_match_prefix(
        &second_tokens,
        &["put", "that", "card", "into", "your", "hand"],
    )
    .is_some()
        || crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &["put", "it", "into", "your", "hand"],
        )
        .is_some();
    let second_words = crate::runtime_backend::token_word_refs(&second_tokens);
    let others_to_graveyard = (contains_word_sequence(&second_words, &["other", "cards"])
        || contains_word_sequence(&second_words, &["all", "other"]))
        && slice_contains(&second_words, &"graveyard");
    if !moves_to_hand || !others_to_graveyard {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(parts.match_tag.clone(), None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    });
    effects.push(EffectAst::ForEachTagged {
        tag: parts.all_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(parts.match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::MoveToZone {
                target: TargetAst::Tagged(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    None,
                ),
                zone: Zone::Graveyard,
                to_top: false,
                battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            }],
        }],
    });
    Ok(Some(effects))
}

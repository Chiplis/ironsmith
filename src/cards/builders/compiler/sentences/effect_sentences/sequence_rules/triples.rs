use super::super::dispatch_entry::{
    ConsultCastCost, consult_cast_effects, consult_stop_rule_is_single_match,
    parse_bargained_face_down_cast_mana_value_gate, parse_consult_bottom_remainder_clause,
    parse_consult_cast_clause, parse_consult_traversal_sentence,
    parse_if_declined_put_match_into_hand, parse_if_no_card_into_hand_this_way_sentence,
    parse_if_you_dont_sentence, parse_top_cards_view_sentence,
};
use crate::cards::builders::compiler::activation_and_restrictions::activated_line_core::find_word_sequence_start;
use crate::cards::builders::compiler::effect_sentences;
use crate::cards::builders::compiler::effect_sentences::SentenceInput;
use crate::cards::builders::compiler::lexer::TokenWordView;
use crate::cards::builders::compiler::token_primitives::{
    parse_leading_may_action_lexed, slice_contains, slice_ends_with, slice_starts_with,
};
use crate::cards::builders::compiler::util::is_article;
use crate::cards::builders::compiler::util::trim_commas;
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, ObjectFilter, PredicateAst, TagKey, TargetAst,
};
use crate::effect::Value;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub(super) fn parse_mill_then_may_put_from_among_into_hand_then_if_you_dont(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) =
        super::pairs::parse_mill_then_may_put_from_among_into_hand(sentences, sentence_idx)?
    else {
        return Ok(None);
    };
    let Some(if_not_chosen) = parse_if_you_dont_sentence(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };

    let Some(EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        if_not_chosen: existing,
        ..
    }) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}

pub(super) fn parse_search_face_down_exile_conditional_cast_else_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let third = sentences[sentence_idx + 2].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_chain(first) else {
        return Ok(None);
    };
    let searched_tag: TagKey = "searched_face_down".into();
    let has_face_down_search = first_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ChooseObjectsAcrossZones { tag, .. } if *tag == searched_tag
        ) || matches!(
            effect,
            EffectAst::ChooseObjects { tag, .. } if *tag == searched_tag
        )
    }) && first_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::Exile {
                target: TargetAst::Tagged(tag, _),
                face_down: true,
            } if *tag == searched_tag
        )
    });
    if !has_face_down_search {
        return Ok(None);
    }

    let Some(hand_effects) = parse_if_declined_put_match_into_hand(third, searched_tag.clone())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let Some((operator, right)) = parse_bargained_face_down_cast_mana_value_gate(&second_tokens)?
    else {
        return Ok(None);
    };
    let combined_predicate = PredicateAst::And(
        Box::new(PredicateAst::ThisSpellPaidLabel("Bargain".to_string())),
        Box::new(PredicateAst::ValueComparison {
            left: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(searched_tag.clone()))),
            operator,
            right,
        }),
    );
    let mut effects = first_effects;
    effects.push(EffectAst::Conditional {
        predicate: combined_predicate,
        if_true: vec![
            EffectAst::May {
                effects: vec![EffectAst::CastTagged {
                    tag: searched_tag.clone(),
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                }],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::WasDeclined,
                effects: hand_effects.clone(),
            },
        ],
        if_false: hand_effects,
    });
    Ok(Some(effects))
}

pub(super) fn parse_exile_until_match_cast_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let third = sentences[sentence_idx + 2].lowered();
    let Some(parts) = parse_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    let Some(clause) = parse_consult_cast_clause(second) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) {
        return Ok(None);
    }
    let Some(order) = parse_consult_bottom_remainder_clause(
        third,
        match parts.effects.last() {
            Some(EffectAst::ConsultTopOfLibrary { mode, .. }) => *mode,
            _ => return Ok(None),
        },
    ) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag.clone())?);
    effects.push(EffectAst::PutTaggedRemainderOnBottomOfLibrary {
        tag: parts.all_tag,
        keep_tagged: None,
        order,
        player: parts.player,
    });
    Ok(Some(effects))
}

pub(super) fn parse_exile_until_match_cast_else_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(sentences[sentence_idx].lowered())? else {
        return Ok(None);
    };
    let Some(EffectAst::ConsultTopOfLibrary {
        mode: crate::cards::builders::LibraryConsultModeAst::Exile,
        stop_rule,
        ..
    }) = parts.effects.last()
    else {
        return Ok(None);
    };
    if !consult_stop_rule_is_single_match(stop_rule) {
        return Ok(None);
    }
    let Some(clause) = parse_consult_cast_clause(sentences[sentence_idx + 1].lowered()) else {
        return Ok(None);
    };
    if !matches!(clause.cost, ConsultCastCost::WithoutPayingManaCost) || clause.allow_land {
        return Ok(None);
    }
    let Some(hand_effects) = parse_if_declined_put_match_into_hand(
        sentences[sentence_idx + 2].lowered(),
        parts.match_tag.clone(),
    ) else {
        return Ok(None);
    };

    let cast_effects = consult_cast_effects(&clause, parts.match_tag)?;
    let mut effects = parts.effects;
    if cast_effects.len() == 1 {
        let single_effect = cast_effects.into_iter().next().ok_or_else(|| {
            CardTextError::ParseError("missing cast effect for consult follow-up".to_string())
        })?;
        let EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } = single_effect
        else {
            effects.push(single_effect);
            effects.push(EffectAst::IfResult {
                predicate: IfResultPredicate::WasDeclined,
                effects: hand_effects,
            });
            return Ok(Some(effects));
        };
        let mut gated_if_true = if_true;
        gated_if_true.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects.clone(),
        });
        let mut gated_if_false = if_false;
        gated_if_false.extend(hand_effects);
        effects.push(EffectAst::Conditional {
            predicate,
            if_true: gated_if_true,
            if_false: gated_if_false,
        });
    } else {
        effects.extend(cast_effects);
        effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::WasDeclined,
            effects: hand_effects,
        });
    }
    Ok(Some(effects))
}

pub(super) fn parse_top_cards_put_match_into_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) =
        parse_leading_may_action_lexed(&second_tokens, &["reveal", "put"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let reveal_chosen = action_match.verb == "reveal";
    let action_tokens = trim_commas(action_match.tail_tokens);
    let action_words = TokenWordView::new(&action_tokens);
    if action_words.is_empty() {
        return Ok(None);
    }
    let action_word_refs = action_words.word_refs();

    let Some((from_among_word_idx, from_among_len)) =
        effect_sentences::find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };

    let filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    let filter = if let Some(filter) =
        effect_sentences::parse_looked_card_choice_filter(&action_tokens[..filter_end])
    {
        filter
    } else {
        return Ok(None);
    };

    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    let moves_into_hand = if reveal_chosen {
        (slice_starts_with(after_from_words, &["and", "put", "it", "into"])
            || slice_starts_with(after_from_words, &["put", "it", "into"]))
            && slice_contains(after_from_words, &"hand")
    } else {
        slice_starts_with(after_from_words, &["into"]) && slice_contains(after_from_words, &"hand")
    };
    if !moves_into_hand {
        return Ok(None);
    }

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    let puts_rest_graveyard = matches!(third_words.first(), Some("put" | "puts"))
        && third_words.find_word("rest").is_some()
        && third_words.find_word("graveyard").is_some();
    if !puts_rest_graveyard {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::LookAtTopCards {
        player,
        count,
        tag: TagKey::from(crate::cards::builders::IT_TAG),
    }];
    if reveal_top {
        effects.push(EffectAst::RevealTagged {
            tag: TagKey::from(crate::cards::builders::IT_TAG),
        });
    }
    effects.push(EffectAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
        player: chooser,
        filter,
        reveal: reveal_chosen,
        if_not_chosen: Vec::new(),
    });
    Ok(Some(effects))
}

pub(crate) fn parse_top_cards_for_each_card_type_among_spells_put_matching_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if !reveal_top {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = TokenWordView::new(&second_tokens);
    let word_refs = second_words.word_refs();
    if !slice_starts_with(&word_refs, &["for", "each", "card", "type", "among"]) {
        return Ok(None);
    }

    let Some(put_idx) = find_word_sequence_start(&word_refs[5..], &["you", "may", "put"]) else {
        return Ok(None);
    };
    let put_idx = put_idx + 5;
    let mut tail_idx = put_idx + 3;
    if word_refs.get(tail_idx).is_some_and(|word| is_article(word)) {
        tail_idx += 1;
    }
    if !slice_starts_with(
        &word_refs[tail_idx..],
        &[
            "card", "of", "that", "type", "from", "among", "the", "revealed", "cards", "into",
        ],
    ) || !slice_contains(&word_refs[tail_idx..], &"hand")
    {
        return Ok(None);
    }

    let filter_start = second_words
        .token_index_for_word_index(5)
        .unwrap_or(second_tokens.len());
    let filter_end = second_words
        .token_index_for_word_index(put_idx)
        .unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    let filter_word_view = TokenWordView::new(&filter_tokens);
    let filter_words = filter_word_view.word_refs();
    let suffix_patterns: &[&[&str]] = &[
        &["youve", "cast", "this", "turn"],
        &["you", "have", "cast", "this", "turn"],
        &["you", "cast", "this", "turn"],
    ];
    let Some(suffix) = suffix_patterns
        .iter()
        .copied()
        .find(|suffix| slice_ends_with(&filter_words, suffix))
    else {
        return Ok(None);
    };
    let filter_word_len = filter_words.len().saturating_sub(suffix.len());
    let filter_token_end = crate::cards::builders::compiler::token_index_for_word_index(
        &filter_tokens,
        filter_word_len,
    )
    .unwrap_or(filter_tokens.len());
    let filter_prefix_tokens = trim_commas(&filter_tokens[..filter_token_end]);
    let mut spell_filter =
        crate::cards::builders::compiler::parse_spell_filter_lexed(&filter_prefix_tokens);
    spell_filter.zone = Some(Zone::Stack);
    spell_filter.has_mana_cost = true;

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    if !matches!(third_words.first(), Some("put" | "puts"))
        || third_words.find_word("rest").is_none()
    {
        return Ok(None);
    }
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::LookAtTopCards {
            player,
            count,
            tag: TagKey::from(crate::cards::builders::IT_TAG),
        },
        EffectAst::RevealTagged {
            tag: TagKey::from(crate::cards::builders::IT_TAG),
        },
        EffectAst::ChooseFromLookedCardsForEachCardTypeAmongSpellsCastThisTurnIntoHandRestOnBottomOfLibrary {
            player,
            spell_filter,
            order,
        },
    ]))
}

pub(super) fn parse_top_cards_put_match_onto_battlefield_and_match_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some((chooser, battlefield_filter, tapped, hand_filter)) =
        effect_sentences::parse_may_put_filtered_looked_card_onto_battlefield_and_filtered_into_hand(
            sentences[sentence_idx + 1].lowered(),
        )?
    else {
        return Ok(None);
    };

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    if !matches!(third_words.first(), Some("put" | "puts"))
        || third_words.find_word("rest").is_none()
    {
        return Ok(None);
    }
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

    let mut effects = vec![EffectAst::LookAtTopCards {
        player,
        count,
        tag: TagKey::from(crate::cards::builders::IT_TAG),
    }];
    if reveal_top {
        effects.push(EffectAst::RevealTagged {
            tag: TagKey::from(crate::cards::builders::IT_TAG),
        });
    }
    effects.push(
        EffectAst::ChooseFromLookedCardsOntoBattlefieldAndIntoHandRestOnBottomOfLibrary {
            player: chooser,
            battlefield_filter,
            hand_filter,
            tapped,
            order,
        },
    );
    Ok(Some(effects))
}

pub(super) fn parse_look_at_top_reveal_match_put_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { player, count, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) = parse_leading_may_action_lexed(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, *player);
    let reveal_tokens = trim_commas(action_match.tail_tokens);
    let reveal_words = TokenWordView::new(&reveal_tokens);
    if reveal_words.is_empty() {
        return Ok(None);
    }
    let reveal_word_refs = reveal_words.word_refs();

    let Some((from_among_word_idx, from_among_len)) =
        effect_sentences::find_from_among_looked_cards_phrase(&reveal_words)
    else {
        return Ok(None);
    };

    let filter_end = reveal_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(reveal_tokens.len());
    let filter_tokens = trim_commas(&reveal_tokens[..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter =
        if let Some(filter) = effect_sentences::parse_looked_card_reveal_filter(&filter_tokens) {
            filter
        } else {
            return Ok(None);
        };
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_words = &reveal_word_refs[from_among_word_idx + from_among_len..];
    let puts_into_hand = (slice_starts_with(after_from_words, &["and", "put", "it", "into"])
        || slice_starts_with(after_from_words, &["put", "it", "into"]))
        && slice_contains(after_from_words, &"hand");
    if !puts_into_hand {
        return Ok(None);
    }

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    let puts_rest_bottom = matches!(third_words.first(), Some("put" | "puts"))
        && third_words.find_word("rest").is_some()
        && third_words.find_word("bottom").is_some()
        && third_words.find_word("library").is_some();
    if !puts_rest_bottom {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::LookAtTopCards {
        player: *player,
        count: count.clone(),
        tag: TagKey::from(crate::cards::builders::IT_TAG),
    }];
    effects.push(
        EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
            player: chooser,
            filter,
            reveal: true,
            if_not_chosen: Vec::new(),
        },
    );
    Ok(Some(effects))
}

pub(super) fn parse_prefix_then_consult_match_move_and_bottom_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(prefix_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
            .or_else(|_| effect_sentences::parse_effect_chain(sentences[sentence_idx].lowered()))
    else {
        return Ok(None);
    };
    if prefix_effects.is_empty() {
        return Ok(None);
    }
    let Some(mut combined) =
        super::pairs::parse_consult_match_move_and_bottom_remainder(sentences, sentence_idx + 1)?
    else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

pub(super) fn parse_prefix_then_consult_match_into_hand_exile_others(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(prefix_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
            .or_else(|_| effect_sentences::parse_effect_chain(sentences[sentence_idx].lowered()))
    else {
        return Ok(None);
    };
    if prefix_effects.is_empty() {
        return Ok(None);
    }
    let Some(mut combined) =
        super::pairs::parse_consult_match_into_hand_exile_others(sentences, sentence_idx + 1)?
    else {
        return Ok(None);
    };
    let mut effects = prefix_effects;
    effects.append(&mut combined);
    Ok(Some(effects))
}

pub(super) fn parse_tainted_pact_sequence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_words: Vec<&str> = crate::cards::builders::compiler::token_word_refs(&first_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if first_words.as_slice() != ["exile", "top", "card", "of", "your", "library"] {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words: Vec<&str> = crate::cards::builders::compiler::token_word_refs(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let second_matches = second_words.as_slice()
        == [
            "you", "may", "put", "that", "card", "into", "your", "hand", "unless", "it", "has",
            "same", "name", "as", "another", "card", "exiled", "this", "way",
        ]
        || second_words.as_slice()
            == [
                "you", "may", "put", "it", "into", "your", "hand", "unless", "it", "has", "same",
                "name", "as", "another", "card", "exiled", "this", "way",
            ];
    if !second_matches {
        return Ok(None);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let third_words: Vec<&str> = crate::cards::builders::compiler::token_word_refs(&third_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let third_matches = third_words.as_slice()
        == [
            "repeat",
            "this",
            "process",
            "until",
            "you",
            "put",
            "card",
            "into",
            "your",
            "hand",
            "or",
            "you",
            "exile",
            "two",
            "cards",
            "with",
            "same",
            "name",
            "whichever",
            "comes",
            "first",
        ];
    if !third_matches {
        return Ok(None);
    }

    let current_tag = TagKey::from("tainted_pact_current");
    let exiled_tag = TagKey::from("tainted_pact_exiled");
    let all_exiled_filter = ObjectFilter::tagged(exiled_tag.clone()).in_zone(Zone::Exile);
    Ok(Some(vec![EffectAst::RepeatProcess {
        effects: vec![
            EffectAst::ExileTopOfLibrary {
                count: Value::Fixed(1),
                player: crate::cards::builders::PlayerAst::You,
                tags: vec![current_tag.clone()],
                accumulated_tags: vec![exiled_tag.clone()],
            },
            EffectAst::Conditional {
                predicate: PredicateAst::And(
                    Box::new(PredicateAst::TaggedMatches(
                        current_tag.clone(),
                        ObjectFilter::default().in_zone(Zone::Exile),
                    )),
                    Box::new(PredicateAst::ValueComparison {
                        left: Value::Count(all_exiled_filter.clone()),
                        operator: crate::effect::ValueComparisonOperator::Equal,
                        right: Value::DistinctNames(all_exiled_filter),
                    }),
                ),
                if_true: vec![EffectAst::MayMoveToZone {
                    target: TargetAst::Tagged(current_tag.clone(), None),
                    zone: Zone::Hand,
                    player: crate::cards::builders::PlayerAst::You,
                }],
                if_false: Vec::new(),
            },
        ],
        continue_effect_index: 1,
        continue_predicate: IfResultPredicate::WasDeclined,
    }]))
}

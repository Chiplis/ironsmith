use super::super::dispatch_entry::{
    ConsultCastCost, consult_cast_effects, consult_stop_rule_is_single_match,
    find_from_among_looked_cards_phrase, parse_bargained_face_down_cast_mana_value_gate,
    parse_consult_bottom_remainder_clause, parse_consult_cast_clause,
    parse_consult_traversal_sentence, parse_if_declined_put_match_into_hand,
    parse_if_no_card_into_hand_this_way_sentence, parse_if_you_dont_sentence,
    parse_top_cards_view_sentence,
};
use crate::cards::builders::compiler::activation_and_restrictions::activated_line_core::find_word_sequence_start;
use crate::cards::builders::compiler::effect_sentences;
use crate::cards::builders::compiler::effect_sentences::SentenceInput;
use crate::cards::builders::compiler::front_end::lexer::OwnedLexToken;
use crate::cards::builders::compiler::lexer::TokenWordView;
use crate::cards::builders::compiler::token_primitives::{
    parse_leading_may_action_lexed, slice_contains, slice_ends_with, slice_starts_with,
};
use crate::cards::builders::compiler::util::trim_commas;
use crate::cards::builders::compiler::util::{helper_tag_for_tokens, is_article};
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, LibraryConsultModeAst, LibraryConsultStopRuleAst,
    ObjectFilter, PlayerAst, PredicateAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Value};
use crate::target::ChooseSpec;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

fn abundant_harvest_choice_sentence(words: &[&str]) -> bool {
    matches!(words, ["choose", "land", "or", "nonland"])
}

fn abundant_harvest_reveal_sentence(words: &[&str]) -> bool {
    slice_starts_with(
        words,
        &[
            "reveal", "cards", "from", "the", "top", "of", "your", "library",
        ],
    ) && words.ends_with(&["a", "card", "of", "the", "chosen", "kind"])
}

fn abundant_harvest_branch_effects(
    tokens: &[OwnedLexToken],
    filter: ObjectFilter,
    order: crate::cards::builders::LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let all_tag = helper_tag_for_tokens(tokens, "revealed");
    let match_tag = helper_tag_for_tokens(tokens, "chosen");
    vec![
        EffectAst::ConsultTopOfLibrary {
            player: PlayerAst::You,
            mode: LibraryConsultModeAst::Reveal,
            filter,
            stop_rule: LibraryConsultStopRuleAst::FirstMatch,
            all_tag: all_tag.clone(),
            match_tag: match_tag.clone(),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(match_tag.clone(), None),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
        EffectAst::PutTaggedRemainderOnBottomOfLibrary {
            tag: all_tag,
            keep_tagged: Some(match_tag),
            order,
            player: PlayerAst::You,
        },
    ]
}

pub(super) fn parse_choose_land_or_nonland_then_consult_to_hand_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let third = trim_commas(sentences[sentence_idx + 2].lowered());

    let first_words = crate::cards::builders::compiler::token_word_refs(&first);
    if !abundant_harvest_choice_sentence(&first_words) {
        return Ok(None);
    }

    let second_words = crate::cards::builders::compiler::token_word_refs(&second);
    if !abundant_harvest_reveal_sentence(&second_words) {
        return Ok(None);
    }

    let third_words = crate::cards::builders::compiler::token_word_refs(&third);
    let moves_to_hand =
        slice_starts_with(
            &third_words,
            &["put", "that", "card", "into", "your", "hand"],
        ) || slice_starts_with(&third_words, &["put", "it", "into", "your", "hand"]);
    if !moves_to_hand || !slice_contains(&third_words, &"rest") {
        return Ok(None);
    }
    let Some(order) = super::super::dispatch_entry::parse_consult_remainder_order(&third_words)
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
        EffectAst::ChooseNamedOption {
            player: PlayerAst::You,
            options: vec!["land".to_string(), "nonland".to_string()],
        },
        EffectAst::Conditional {
            predicate: PredicateAst::SourceChosenOption("land".to_string()),
            if_true: abundant_harvest_branch_effects(
                sentences[sentence_idx + 1].lowered(),
                land_filter,
                order,
            ),
            if_false: abundant_harvest_branch_effects(
                sentences[sentence_idx + 1].lowered(),
                nonland_filter,
                order,
            ),
        },
    ]))
}

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
                    player: PlayerAst::Implicit,
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

fn trim_keyword_choice_segment(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && (tokens[start].is_comma() || tokens[start].is_period() || tokens[start].is_word("and"))
    {
        start += 1;
    }
    while end > start
        && (tokens[end - 1].is_comma()
            || tokens[end - 1].is_period()
            || tokens[end - 1].is_word("and"))
    {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

fn split_keyword_choice_segments(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        if token.is_comma() {
            let trimmed = trim_keyword_choice_segment(&current);
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
            current.clear();
            continue;
        }
        current.push(token.clone());
    }
    let trimmed = trim_keyword_choice_segment(&current);
    if !trimmed.is_empty() {
        segments.push(trimmed);
    }
    segments
}

fn parse_keyword_choice_filter(segment: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let trimmed = trim_keyword_choice_segment(segment);
    if trimmed.is_empty() {
        return None;
    }
    effect_sentences::parse_looked_card_choice_filter(&trimmed).or_else(|| {
        let mut expanded = vec![
            OwnedLexToken::word("a".to_string(), TextSpan::synthetic()),
            OwnedLexToken::word("card".to_string(), TextSpan::synthetic()),
            OwnedLexToken::word("with".to_string(), TextSpan::synthetic()),
        ];
        expanded.extend(trimmed);
        effect_sentences::parse_looked_card_choice_filter(&expanded)
    })
}

fn parse_choose_from_looked_cards_for_each_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<ObjectFilter>>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let words = TokenWordView::new(&sentence_tokens);
    if !matches!(words.first(), Some("choose")) {
        return Ok(None);
    }

    let Some((from_among_word_idx, from_among_len)) = find_from_among_looked_cards_phrase(&words)
    else {
        return Ok(None);
    };
    if from_among_word_idx != 1 {
        return Ok(None);
    }

    let tail_start = words
        .token_index_after_words(from_among_word_idx + from_among_len)
        .unwrap_or(sentence_tokens.len());
    let tail_tokens = trim_commas(&sentence_tokens[tail_start..]);
    let tail_words = TokenWordView::new(&tail_tokens);
    let tail_refs = tail_words.word_refs();
    let Some(and_so_on_idx) = find_word_sequence_start(&tail_refs, &["and", "so", "on", "for"])
    else {
        return Ok(None);
    };

    let prelude_end = tail_words
        .token_index_for_word_index(and_so_on_idx)
        .unwrap_or(tail_tokens.len());
    let suffix_start = tail_words
        .token_index_after_words(and_so_on_idx + 4)
        .unwrap_or(tail_tokens.len());

    let mut filters = Vec::new();
    for segment in split_keyword_choice_segments(&tail_tokens[..prelude_end]) {
        let Some(filter) = parse_keyword_choice_filter(&segment) else {
            return Err(CardTextError::ParseError(
                "unable to parse initial looked-card choice filter".to_string(),
            ));
        };
        filters.push(filter);
    }
    for segment in split_keyword_choice_segments(&tail_tokens[suffix_start..]) {
        let Some(filter) = parse_keyword_choice_filter(&segment) else {
            return Err(CardTextError::ParseError(
                "unable to parse repeated looked-card choice filter".to_string(),
            ));
        };
        filters.push(filter);
    }

    if filters.len() < 3 {
        return Ok(None);
    }
    Ok(Some(filters))
}

fn is_one_chosen_to_battlefield_others_to_hand_rest_to_graveyard(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words = TokenWordView::new(&trimmed);
    let word_refs = words.word_refs();
    if !slice_starts_with(
        &word_refs,
        &[
            "put",
            "one",
            "of",
            "the",
            "chosen",
            "cards",
            "onto",
            "the",
            "battlefield",
        ],
    ) {
        return false;
    }
    find_word_sequence_start(
        &word_refs,
        &["the", "other", "chosen", "cards", "into", "your", "hand"],
    )
    .is_some()
        && find_word_sequence_start(&word_refs, &["the", "rest", "into", "your", "graveyard"])
            .is_some()
}

pub(super) fn parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(choice_filters) =
        parse_choose_from_looked_cards_for_each_filter(sentences[sentence_idx + 1].lowered())?
    else {
        return Ok(None);
    };
    if !is_one_chosen_to_battlefield_others_to_hand_rest_to_graveyard(
        sentences[sentence_idx + 2].lowered(),
    ) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "revealed");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let battlefield_tag =
        helper_tag_for_tokens(sentences[sentence_idx + 2].lowered(), "battlefield");

    let mut effects = vec![EffectAst::LookAtTopCards {
        player,
        count,
        tag: looked_tag.clone(),
    }];
    if reveal_top {
        effects.push(EffectAst::RevealTagged {
            tag: looked_tag.clone(),
        });
    }

    for filter in choice_filters {
        let mut choose_filter = filter;
        choose_filter.zone = Some(Zone::Library);
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
        effects.push(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player,
            tag: chosen_tag.clone(),
        });
    }

    let mut battlefield_filter = ObjectFilter::default();
    battlefield_filter.zone = Some(Zone::Library);
    battlefield_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: chosen_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    effects.push(EffectAst::ChooseObjects {
        filter: battlefield_filter,
        count: ChoiceCount::up_to(1),
        count_value: None,
        player,
        tag: battlefield_tag.clone(),
    });
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(battlefield_tag.clone(), None),
        zone: Zone::Battlefield,
        to_top: false,
        battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(battlefield_tag.clone()),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::MoveToZone {
                target: TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                zone: Zone::Hand,
                to_top: false,
                battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            }],
        }],
    });
    effects.push(EffectAst::ForEachTagged {
        tag: looked_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(chosen_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::MoveToZone {
                target: TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
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

pub(crate) fn parse_top_cards_for_each_card_type_put_matching_into_hand_rest_bottom(
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
    if !slice_starts_with(&word_refs, &["for", "each", "card", "type"]) {
        return Ok(None);
    }
    if word_refs.get(4).is_some_and(|word| *word == "among") {
        return Ok(None);
    }

    let Some(put_idx) = find_word_sequence_start(&word_refs[4..], &["you", "may", "put"]) else {
        return Ok(None);
    };
    let put_idx = put_idx + 4;
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
        EffectAst::ChooseFromLookedCardsForEachCardTypeIntoHandRestOnBottomOfLibrary {
            player,
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
        || slice_starts_with(after_from_words, &["put", "it", "into"])
        || slice_starts_with(after_from_words, &["and", "put", "that", "card", "into"])
        || slice_starts_with(after_from_words, &["put", "that", "card", "into"]))
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
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

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
            order,
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

use super::super::super::dispatch_entry::{
    ConsultCastCost, consult_cast_effects, consult_stop_rule_is_single_match,
    find_from_among_looked_cards_phrase, parse_bargained_face_down_cast_mana_value_gate,
    parse_consult_bottom_remainder_clause, parse_consult_cast_clause,
    parse_consult_traversal_sentence, parse_if_declined_put_match_into_hand,
    parse_if_you_dont_sentence, parse_looked_card_choice_filter, parse_top_cards_view_sentence,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, IfResultPredicate, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, ObjectFilter, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::{ChoiceCount, Value};
use crate::runtime_backend::activation_and_restrictions::activated_line_core::find_word_sequence_start;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::front_end::lexer::OwnedLexToken;
use crate::runtime_backend::lexer::TokenWordView;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::token_primitives::{
    LeadingMayActor, parse_count_range_prefix, parse_leading_may_action_lexed, slice_contains,
    slice_ends_with, slice_starts_with,
};
use crate::runtime_backend::util::trim_commas;
use crate::runtime_backend::util::{helper_tag_for_tokens, is_article};
use crate::target::ChooseSpec;
use crate::target::{PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

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

fn looked_cards_choice_count(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, Vec<OwnedLexToken>)> {
    let trimmed = trim_commas(tokens);
    let Some(((min, max), rest)) = parse_count_range_prefix(&trimmed) else {
        return Some((ChoiceCount::up_to(1), trimmed));
    };
    let count = match (min, max) {
        (Some(Value::Fixed(0)), Some(Value::Fixed(max))) if max >= 0 => {
            ChoiceCount::up_to(max as usize)
        }
        (Some(Value::Fixed(min)), Some(Value::Fixed(max))) if min >= 0 && max >= min => {
            ChoiceCount {
                min: min as usize,
                max: Some(max as usize),
                dynamic_x: false,
                up_to_x: false,
                random: false,
            }
        }
        _ => return None,
    };
    Some((count, trim_commas(rest)))
}

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

pub(crate) fn parse_choose_land_or_nonland_then_consult_to_hand_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let third = trim_commas(sentences[sentence_idx + 2].lowered());

    let first_words = crate::runtime_backend::token_word_refs(&first);
    if !abundant_harvest_choice_sentence(&first_words) {
        return Ok(None);
    }

    let second_words = crate::runtime_backend::token_word_refs(&second);
    if !abundant_harvest_reveal_sentence(&second_words) {
        return Ok(None);
    }

    let third_words = crate::runtime_backend::token_word_refs(&third);
    let moves_to_hand =
        slice_starts_with(
            &third_words,
            &["put", "that", "card", "into", "your", "hand"],
        ) || slice_starts_with(&third_words, &["put", "it", "into", "your", "hand"]);
    if !moves_to_hand || !slice_contains(&third_words, &"rest") {
        return Ok(None);
    }
    let Some(order) =
        super::super::super::dispatch_entry::parse_consult_remainder_order(&third_words)
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

pub(crate) fn parse_exile_top_put_land_then_cast_each_nonland_type(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let third = trim_commas(sentences[sentence_idx + 2].lowered());

    let first_words = crate::runtime_backend::token_word_refs(&first);
    if !slice_starts_with(&first_words, &["exile", "the", "top"])
        || first_words.get(4..) != Some(["cards", "of", "your", "library"].as_slice())
    {
        return Ok(None);
    }

    let second_words = crate::runtime_backend::token_word_refs(&second);
    if second_words.as_slice()
        != [
            "you",
            "may",
            "put",
            "a",
            "land",
            "card",
            "from",
            "among",
            "them",
            "onto",
            "the",
            "battlefield",
        ]
    {
        return Ok(None);
    }

    let third_words = crate::runtime_backend::token_word_refs(&third);
    if !slice_starts_with(
        &third_words,
        &[
            "until", "end", "of", "turn", "for", "each", "nonland", "card", "type", "you",
            "may", "cast", "a", "spell", "of", "that", "type", "from", "among", "the",
            "exiled", "cards",
        ],
    ) || !slice_ends_with(&third_words, &["without", "paying", "its", "mana", "cost"])
    {
        return Ok(None);
    }

    let mut effects = effect_sentences::parse_effect_sentence_lexed(&first)?;
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::ExileTopOfLibrary { tags, .. },
        ..
    })) = effects.first()
    else {
        return Ok(None);
    };
    let Some(exiled_tag) = tags.first().cloned() else {
        return Ok(None);
    };

    let mut land_filter = ObjectFilter::default()
        .in_zone(Zone::Exile)
        .with_type(CardType::Land)
        .match_tagged(exiled_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
    land_filter.zone = Some(Zone::Exile);
    let land_target = TargetAst::WithCount(
        Box::new(TargetAst::Object(land_filter, None, None)),
        ChoiceCount::up_to(1),
    );

    effects.push(EffectAst::May {
        effects: vec![EffectAst::subject_verb_move_to_zone(
            land_target,
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(EffectAst::GrantFreeCastFromTaggedForEachCardTypeUntilEndOfTurn {
        tag: exiled_tag,
        player: PlayerAst::You,
        card_types: vec![
            CardType::Artifact,
            CardType::Battle,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Instant,
            CardType::Kindred,
            CardType::Planeswalker,
            CardType::Sorcery,
        ],
    });

    Ok(Some(effects))
}

pub(crate) fn parse_mill_then_may_put_from_among_into_hand_then_if_you_dont(
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

    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ChooseFromLookedCardsIntoHandRestIntoGraveyard {
                if_not_chosen: existing,
                ..
            },
        ..
    })) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}

pub(crate) fn parse_reveal_top_opponent_exiles_one_put_rest_hand_then_may_cast(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let Some((player, count, true)) = parse_top_cards_view_sentence(&first) else {
        return Ok(None);
    };
    if player != PlayerAst::You {
        return Ok(None);
    }

    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = TokenWordView::new(&second);
    let second_word_refs = second_words.word_refs();
    let Some(then_word_idx) = find_word_sequence_start(&second_word_refs, &["then"]) else {
        return Ok(None);
    };
    let Some(then_token_idx) = second_words.token_index_for_word_index(then_word_idx) else {
        return Ok(None);
    };
    let exile_tokens = trim_commas(&second[..then_token_idx]);
    let rest_tokens = trim_commas(&second[then_token_idx + 1..]);

    let exile_words = TokenWordView::new(&exile_tokens);
    let exile_word_refs = exile_words.word_refs();
    let Some(exile_word_idx) = find_word_sequence_start(&exile_word_refs, &["exiles"]) else {
        return Ok(None);
    };
    let actor_words = exile_word_refs[..exile_word_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    if actor_words.as_slice() != ["opponent"] {
        return Ok(None);
    }
    let Some(exile_tail_start) = exile_words.token_index_after_words(exile_word_idx + 1) else {
        return Ok(None);
    };
    let exile_tail = trim_commas(&exile_tokens[exile_tail_start..]);
    let exile_tail_words = TokenWordView::new(&exile_tail);
    let Some((from_among_word_idx, from_among_len)) =
        find_from_among_looked_cards_phrase(&exile_tail_words)
    else {
        return Ok(None);
    };
    let Some(filter_end) = exile_tail_words.token_index_for_word_index(from_among_word_idx) else {
        return Ok(None);
    };

    let revealed_tag = helper_tag_for_tokens(&first, "revealed");
    let exiled_tag = helper_tag_for_tokens(&first, "exiled");
    let mut exile_filter =
        if let Some(filter) = parse_looked_card_choice_filter(&exile_tail[..filter_end]) {
            filter
        } else {
            return Ok(None);
        };
    exile_filter.zone = Some(Zone::Library);
    exile_filter =
        exile_filter.match_tagged(revealed_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);

    let after_from_among = &exile_tail_words.word_refs()[from_among_word_idx + from_among_len..];
    if !after_from_among.is_empty() {
        return Ok(None);
    }

    let rest_words = TokenWordView::new(&rest_tokens).word_refs();
    let rest_without_articles = rest_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    if rest_without_articles.as_slice() != ["you", "put", "rest", "into", "your", "hand"] {
        return Ok(None);
    }

    let third = trim_commas(sentences[sentence_idx + 2].lowered());
    let third_words = TokenWordView::new(&third).word_refs();
    let third_without_articles = third_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    if third_without_articles.as_slice()
        != [
            "that", "opponent", "may", "cast", "exiled", "card", "without", "paying", "its",
            "mana", "cost",
        ]
    {
        return Ok(None);
    }

    let rest_filter = ObjectFilter::tagged(revealed_tag.clone())
        .not_tagged(exiled_tag.clone())
        .in_zone(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_reveal_top_cards(PlayerAst::You, count, revealed_tag),
        EffectAst::ChooseObjects {
            filter: exile_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: exiled_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), false),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Object(rest_filter, None, None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::MayByPlayer {
            player: PlayerAst::Opponent,
            effects: vec![EffectAst::subject_verb_cast_tagged(
                exiled_tag,
                PlayerAst::Opponent,
                false,
                false,
                true,
                None,
            )],
        },
    ]))
}

pub(crate) fn parse_search_then_player_names_card_conditional_put_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let Some(then_idx) = first.iter().position(|token| token.is_word("then")) else {
        return Ok(None);
    };
    let search_tokens = trim_commas(&first[..then_idx]);
    let name_tokens = trim_commas(&first[then_idx + 1..]);
    if search_tokens.is_empty() || name_tokens.is_empty() {
        return Ok(None);
    }

    let name_words = TokenWordView::new(&name_tokens).word_refs();
    if !matches!(
        name_words.as_slice(),
        ["that", "player", "chooses", "a", "card", "name"]
            | ["that", "player", "choose", "a", "card", "name"]
    ) {
        return Ok(None);
    }

    let search_words = TokenWordView::new(&search_tokens).word_refs();
    if !matches!(
        search_words.as_slice(),
        ["search", "that", "player's", "library", "for", "a", "card"]
            | ["search", "that", "players", "library", "for", "a", "card"]
    ) {
        return Ok(None);
    }
    let searched_tag = TagKey::from("searched");
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
    let chosen_name_tag = TagKey::from("__chosen_name__");

    let second_words = TokenWordView::new(sentences[sentence_idx + 1].lowered()).word_refs();
    let has_searched_creature_card =
        find_word_sequence_start(&second_words, &["if", "you", "searched", "for"]).is_some()
            && find_word_sequence_start(&second_words, &["creature", "card"]).is_some();
    let has_doesnt_have_name =
        find_word_sequence_start(&second_words, &["doesn't", "have", "that", "name"]).is_some()
            || find_word_sequence_start(&second_words, &["doesnt", "have", "that", "name"])
                .is_some()
            || find_word_sequence_start(&second_words, &["doesn", "t", "have", "that", "name"])
                .is_some();
    let has_may_put_battlefield = find_word_sequence_start(
        &second_words,
        &[
            "you",
            "may",
            "put",
            "it",
            "onto",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
        ],
    )
    .is_some();
    if !has_searched_creature_card || !has_doesnt_have_name || !has_may_put_battlefield {
        return Ok(None);
    }

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered()).word_refs();
    if !matches!(
        third_words.as_slice(),
        ["then", "that", "player", "shuffles"] | ["then", "that", "player", "shuffle"]
    ) {
        return Ok(None);
    }

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

pub(crate) fn parse_search_two_then_put_one_hand_other_graveyard_then_shuffle(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_effects = effect_sentences::parse_effect_chain(&first_tokens)?;
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
            ] if zones.as_slice() == [Zone::Library] => (
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

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = TokenWordView::new(&second_tokens).word_refs();
    let content_words = second_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    let puts_one_hand = slice_starts_with(&content_words, &["put", "one", "into", "your", "hand"])
        || slice_starts_with(
            &content_words,
            &["put", "one", "of", "them", "into", "your", "hand"],
        );
    let puts_other_graveyard =
        find_word_sequence_start(&content_words, &["other", "into", "your", "graveyard"]).is_some()
            || find_word_sequence_start(&content_words, &["other", "into", "graveyard"]).is_some();
    if !puts_one_hand || !puts_other_graveyard {
        return Ok(None);
    }

    let third_tokens = trim_commas(sentences[sentence_idx + 2].lowered());
    let third_words = TokenWordView::new(&third_tokens).word_refs();
    if !matches!(third_words.as_slice(), ["then", "shuffle"] | ["shuffle"]) {
        return Ok(None);
    }

    search_filter.zone = Some(Zone::Library);
    let searched_tag = helper_tag_for_tokens(&first_tokens, "searched");
    let hand_tag = helper_tag_for_tokens(&second_tokens, "hand");
    let mut hand_filter = ObjectFilter::tagged(searched_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    let iterated_is_hand_card =
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG));

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
        EffectAst::ChooseObjects {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: chooser,
            tag: hand_tag.clone(),
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
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
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

pub(crate) fn parse_search_face_down_exile_conditional_cast_else_hand(
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
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::Exile {
                        target: TargetAst::Tagged(tag, _),
                        face_down: true,
                    },
                ..
            }) if *tag == searched_tag
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
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    searched_tag.clone(),
                    PlayerAst::Implicit,
                    false,
                    false,
                    true,
                    None,
                )],
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

pub(crate) fn parse_exile_until_match_cast_rest_bottom(
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
            Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ConsultTopOfLibrary { mode, .. },
                ..
            })) => *mode,
            _ => return Ok(None),
        },
    ) else {
        return Ok(None);
    };

    let mut effects = parts.effects;
    effects.extend(consult_cast_effects(&clause, parts.match_tag.clone())?);
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            parts.all_tag,
            None,
            order,
            parts.player,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_exile_until_match_cast_else_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = parse_consult_traversal_sentence(sentences[sentence_idx].lowered())? else {
        return Ok(None);
    };
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                stop_rule,
                ..
            },
        ..
    })) = parts.effects.last()
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

pub(crate) fn parse_top_cards_put_match_into_hand_rest_graveyard(
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
    let filter_words = crate::runtime_backend::token_word_refs(&action_tokens[..filter_end]);

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
    let third_word_refs = third_words.word_refs();
    let third_rest_words = if third_word_refs.first().copied() == Some("then") {
        &third_word_refs[1..]
    } else {
        &third_word_refs[..]
    };
    let puts_rest_graveyard = matches!(third_rest_words.first(), Some(&"put" | &"puts"))
        && third_rest_words.contains(&"rest")
        && third_rest_words.contains(&"graveyard");
    if !puts_rest_graveyard {
        return Ok(None);
    }

    if filter.card_types.len() > 1
        && filter_words.iter().any(|word| *word == "and/or")
        && filter.all_card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.static_abilities.is_empty()
        && filter.any_of.is_empty()
    {
        let looked_tag = helper_tag_for_tokens(
            sentences[sentence_idx].lowered(),
            if reveal_top { "revealed" } else { "looked" },
        );
        let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
        let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
            player,
            count,
            looked_tag.clone(),
        )];
        if reveal_top {
            effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
        }

        for card_type in &filter.card_types {
            let mut choice_filter = filter.clone();
            choice_filter.card_types = vec![*card_type];
            choice_filter.zone = Some(Zone::Library);
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: looked_tag.clone(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            choice_filter
                .tagged_constraints
                .push(TaggedObjectConstraint {
                    tag: chosen_tag.clone(),
                    relation: TaggedOpbjectRelation::IsNotTaggedObject,
                });
            effects.push(EffectAst::ChooseObjects {
                filter: choice_filter,
                count: ChoiceCount::up_to(1),
                count_value: None,
                player: chooser,
                tag: chosen_tag.clone(),
            });
        }

        effects.push(EffectAst::ForEachTagged {
            tag: chosen_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        });
        let mut in_chosen_filter = ObjectFilter::default();
        in_chosen_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(crate::cards::builders::IT_TAG),
                relation: TaggedOpbjectRelation::SameStableId,
            });

        effects.push(EffectAst::ForEachTagged {
            tag: looked_tag.clone(),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(chosen_tag, in_chosen_filter),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        return Ok(Some(effects));
    }

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        TagKey::from(crate::cards::builders::IT_TAG),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(TagKey::from(
            crate::cards::builders::IT_TAG,
        )));
    }
    effects.push(
        EffectAst::subject_verb_choose_from_looked_cards_into_hand_rest_into_graveyard(
            chooser,
            filter,
            reveal_chosen,
            Vec::new(),
        ),
    );
    Ok(Some(effects))
}

fn parse_any_number_from_looked_cards_action(
    tokens: &[OwnedLexToken],
) -> Option<(ObjectFilter, Zone, bool)> {
    let action_tokens = trim_commas(tokens);
    let action_words = TokenWordView::new(&action_tokens);
    let action_word_refs = action_words.word_refs();
    if !slice_starts_with(&action_word_refs, &["any", "number", "of"]) {
        return None;
    }

    let Some((from_among_word_idx, from_among_len)) =
        effect_sentences::find_from_among_looked_cards_phrase(&action_words)
    else {
        return None;
    };
    if from_among_word_idx <= 3 {
        return None;
    }
    let filter_start = action_words.token_index_for_word_index(3)?;
    let filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    let filter_tokens = trim_commas(&action_tokens[filter_start..filter_end]);
    let mut filter = effect_sentences::parse_looked_card_choice_filter(&filter_tokens)?;
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    let (zone, tapped) = if slice_starts_with(after_from_words, &["into", "your", "hand"])
        || slice_starts_with(after_from_words, &["into", "hand"])
    {
        (Zone::Hand, false)
    } else if slice_starts_with(after_from_words, &["onto", "the", "battlefield", "tapped"])
        || slice_starts_with(after_from_words, &["onto", "battlefield", "tapped"])
    {
        (Zone::Battlefield, true)
    } else if slice_starts_with(after_from_words, &["onto", "the", "battlefield"])
        || slice_starts_with(after_from_words, &["onto", "battlefield"])
    {
        (Zone::Battlefield, false)
    } else {
        return None;
    };

    Some((filter, zone, tapped))
}

pub(crate) fn parse_top_cards_put_any_matching_to_zone_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) = parse_leading_may_action_lexed(&second_tokens, &["put"], true) else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
    let Some((filter, zone, tapped)) =
        parse_any_number_from_looked_cards_action(action_match.tail_tokens)
    else {
        return Ok(None);
    };

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    let third_word_refs = third_words.word_refs();
    let third_rest_words = if third_word_refs.first().copied() == Some("then") {
        &third_word_refs[1..]
    } else {
        &third_word_refs[..]
    };
    let puts_rest_bottom = matches!(third_rest_words.first(), Some(&"put" | &"puts"))
        && third_rest_words.contains(&"rest")
        && third_rest_words.contains(&"bottom")
        && third_rest_words.contains(&"library");
    if !puts_rest_bottom {
        return Ok(None);
    }
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.push(EffectAst::ChooseObjects {
        filter: choose_filter,
        count: ChoiceCount::any_number(),
        count_value: None,
        player: chooser,
        tag: chosen_tag.clone(),
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            zone,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            tapped,
            None,
        )],
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            chooser,
        ),
    );

    Ok(Some(effects))
}

fn parse_cast_from_among_looked_cards_action(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let sentence_tokens = trim_commas(tokens);
    let Some(action_match) = parse_leading_may_action_lexed(&sentence_tokens, &["cast"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let action_words = TokenWordView::new(&action_tokens);
    let action_word_refs = action_words.word_refs();
    let Some((from_among_word_idx, from_among_len)) =
        effect_sentences::find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };
    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    if !slice_starts_with(
        after_from_words,
        &["without", "paying", "its", "mana", "cost"],
    ) {
        return Ok(None);
    }

    let filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    let filter_tokens = trim_commas(&action_tokens[..filter_end]);
    let filter_words = TokenWordView::new(&filter_tokens).word_refs();
    let mentions_spell = filter_words
        .iter()
        .any(|word| matches!(*word, "spell" | "spells"));
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
        && let Some(mana_value_idx) = find_word_sequence_start(&filter_words, &["mana", "value"])
        && matches!(
            filter_words.get(mana_value_idx + 2..mana_value_idx + 5),
            Some(["3", "or", "less"])
        )
    {
        filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(3));
    }

    Ok(Some((chooser, filter)))
}

pub(crate) fn parse_top_cards_may_cast_match_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some((chooser, mut filter)) =
        parse_cast_from_among_looked_cards_action(sentences[sentence_idx + 1].lowered(), player)?
    else {
        return Ok(None);
    };

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    let third_word_refs = third_words.word_refs();
    let third_rest_words = if third_word_refs.first().copied() == Some("then") {
        &third_word_refs[1..]
    } else {
        &third_word_refs[..]
    };
    let puts_rest_bottom = matches!(third_rest_words.first(), Some(&"put" | &"puts"))
        && third_rest_words.contains(&"rest")
        && third_rest_words.contains(&"bottom")
        && third_rest_words.contains(&"library");
    if !puts_rest_bottom {
        return Ok(None);
    }
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen_cast");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.push(EffectAst::ChooseObjects {
        filter,
        count: ChoiceCount::up_to(1),
        count_value: None,
        player: chooser,
        tag: chosen_tag.clone(),
    });
    effects.push(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
            role: SubjectVerbRoleAst::Actor,
            player: chooser,
        },
        action: SubjectVerbActionAst::CastTagged {
            tag: chosen_tag.clone(),
            player: chooser,
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: true,
            cost_reduction: None,
        },
    }));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            chooser,
        ),
    );

    Ok(Some(effects))
}

fn parse_reveal_any_number_from_looked_cards_into_hand_action(
    tokens: &[OwnedLexToken],
    default_player: PlayerAst,
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let second_tokens = trim_commas(tokens);
    let Some(action_match) = parse_leading_may_action_lexed(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, default_player);
    let action_tokens = trim_commas(action_match.tail_tokens);
    let action_words = TokenWordView::new(&action_tokens);
    let action_word_refs = action_words.word_refs();
    if !slice_starts_with(&action_word_refs, &["any", "number", "of"]) {
        return Ok(None);
    }
    let Some((from_among_word_idx, from_among_len)) =
        effect_sentences::find_from_among_looked_cards_phrase(&action_words)
    else {
        return Ok(None);
    };
    let filter_start = action_words.token_index_for_word_index(3).unwrap_or(0);
    let filter_end = action_words
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(action_tokens.len());
    let mut filter =
        effect_sentences::parse_looked_card_choice_filter(&action_tokens[filter_start..filter_end])
            .ok_or_else(|| {
                CardTextError::ParseError("unable to parse revealed looked-card filter".to_string())
            })?;
    filter.zone = Some(Zone::Library);

    let after_from_words = &action_word_refs[from_among_word_idx + from_among_len..];
    let puts_revealed_into_hand =
        slice_starts_with(after_from_words, &["and", "put", "the", "revealed"])
            && after_from_words.contains(&"cards")
            && after_from_words.contains(&"hand");
    if !puts_revealed_into_hand {
        return Ok(None);
    }
    Ok(Some((chooser, filter)))
}

pub(crate) fn parse_top_cards_reveal_any_matching_to_hand_rest_bottom(
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
    let Some((chooser, mut filter)) = parse_reveal_any_number_from_looked_cards_into_hand_action(
        sentences[sentence_idx + 1].lowered(),
        player,
    )?
    else {
        return Ok(None);
    };
    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    let third_word_refs = third_words.word_refs();
    let third_rest_words = if third_word_refs.first().copied() == Some("then") {
        &third_word_refs[1..]
    } else {
        &third_word_refs[..]
    };
    let puts_rest_bottom = matches!(third_rest_words.first(), Some(&"put" | &"puts"))
        && third_rest_words.contains(&"rest")
        && third_rest_words.contains(&"bottom")
        && third_rest_words.contains(&"library");
    if !puts_rest_bottom {
        return Ok(None);
    }
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let revealed_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "revealed");
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: chooser,
            tag: revealed_tag.clone(),
        },
        EffectAst::subject_verb_reveal_tagged(revealed_tag.clone()),
        EffectAst::ForEachTagged {
            tag: revealed_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Hand,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(revealed_tag),
            order,
            chooser,
        ),
    ]))
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

pub(crate) fn parse_top_cards_choose_for_each_filter_one_battlefield_others_hand_rest_graveyard(
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

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
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
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(battlefield_tag.clone(), None),
        Zone::Battlefield,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(battlefield_tag.clone()),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Hand,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
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
            if_false: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
                Zone::Graveyard,
                false,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            )],
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
    let filter_token_end =
        crate::runtime_backend::token_index_for_word_index(&filter_tokens, filter_word_len)
            .unwrap_or(filter_tokens.len());
    let filter_prefix_tokens = trim_commas(&filter_tokens[..filter_token_end]);
    let mut spell_filter = crate::runtime_backend::parse_spell_filter_lexed(&filter_prefix_tokens);
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
        EffectAst::subject_verb_look_at_top_cards(player, count, TagKey::from(crate::cards::builders::IT_TAG)),
        EffectAst::subject_verb_reveal_tagged(TagKey::from(crate::cards::builders::IT_TAG)),
        EffectAst::subject_verb_choose_from_looked_cards_for_each_card_type_among_spells_cast_this_turn_into_hand_rest_on_bottom_of_library(
            player,
            spell_filter,
            order,
        ),
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
        EffectAst::subject_verb_look_at_top_cards(
            player,
            count,
            TagKey::from(crate::cards::builders::IT_TAG),
        ),
        EffectAst::subject_verb_reveal_tagged(TagKey::from(crate::cards::builders::IT_TAG)),
        EffectAst::subject_verb_choose_from_looked_cards_for_each_card_type_into_hand_rest_on_bottom_of_library(
            player,
            order,
        ),
    ]))
}

fn is_put_one_looked_card_hand_one_bottom_exile_one(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words = TokenWordView::new(&trimmed);
    let word_refs = words.word_refs();

    slice_starts_with(
        &word_refs,
        &["put", "one", "of", "them", "into", "your", "hand"],
    ) && find_word_sequence_start(
        &word_refs,
        &[
            "put", "one", "of", "them", "on", "the", "bottom", "of", "your", "library",
        ],
    )
    .or_else(|| {
        find_word_sequence_start(
            &word_refs,
            &[
                "put", "one", "of", "them", "on", "bottom", "of", "your", "library",
            ],
        )
    })
    .is_some()
        && find_word_sequence_start(&word_refs, &["exile", "one", "of", "them"]).is_some()
}

pub(crate) fn parse_look_at_top_split_hand_bottom_exile_then_play_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    if !is_put_one_looked_card_hand_one_bottom_exile_one(sentences[sentence_idx + 1].lowered()) {
        return Ok(None);
    }

    let Some(permission) = parse_cast_or_play_tagged_clause(sentences[sentence_idx + 2].lowered())?
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
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let bottom_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "bottom");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }

    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    effects.push(EffectAst::ChooseObjects {
        filter: hand_filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player,
        tag: hand_tag.clone(),
    });

    let mut bottom_filter = ObjectFilter::tagged(looked_tag.clone()).not_tagged(hand_tag.clone());
    bottom_filter.zone = Some(Zone::Library);
    effects.push(EffectAst::ChooseObjects {
        filter: bottom_filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player,
        tag: bottom_tag.clone(),
    });

    let mut exile_filter = ObjectFilter::tagged(looked_tag.clone())
        .not_tagged(hand_tag.clone())
        .not_tagged(bottom_tag.clone());
    exile_filter.zone = Some(Zone::Library);
    effects.push(EffectAst::ChooseObjects {
        filter: exile_filter,
        count: ChoiceCount::exactly(1),
        count_value: None,
        player,
        tag: exiled_tag.clone(),
    });

    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(hand_tag, None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(bottom_tag, None),
        Zone::Library,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb_exile(
        TargetAst::Tagged(exiled_tag.clone(), None),
        false,
    ));
    effects.push(EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
        exiled_tag,
        permission_player,
        allow_land,
        without_paying_mana_cost,
        allow_any_color_for_cast,
    ));

    Ok(Some(effects))
}

pub(crate) fn parse_top_cards_put_match_onto_battlefield_and_match_into_hand_rest_bottom(
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

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        TagKey::from(crate::cards::builders::IT_TAG),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(TagKey::from(
            crate::cards::builders::IT_TAG,
        )));
    }
    effects.push(
        EffectAst::subject_verb_choose_from_looked_cards_onto_battlefield_and_into_hand_rest_on_bottom_of_library(
            chooser,
            battlefield_filter,
            hand_filter,
            tapped,
            order,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_look_at_top_reveal_match_put_rest_bottom(
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
    let Some((player, count)) = look_at_top_cards_parts(first_effect) else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) = parse_leading_may_action_lexed(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
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
    let Some((mut choice_count, filter_tokens)) = looked_cards_choice_count(&filter_tokens) else {
        return Ok(None);
    };
    if !matches!(action_match.actor, LeadingMayActor::Default) && choice_count.min > 0 {
        choice_count = ChoiceCount::up_to(choice_count.max.unwrap_or(choice_count.min));
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
        || slice_starts_with(after_from_words, &["and", "put", "them", "into"])
        || slice_starts_with(after_from_words, &["put", "them", "into"])
        || slice_starts_with(after_from_words, &["and", "put", "that", "card", "into"])
        || slice_starts_with(after_from_words, &["put", "that", "card", "into"]))
        && slice_contains(after_from_words, &"hand");
    if !puts_into_hand {
        return Ok(None);
    }

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    let third_word_refs = third_words.word_refs();
    let third_rest_words = if third_word_refs.first().copied() == Some("then") {
        &third_word_refs[1..]
    } else {
        &third_word_refs[..]
    };
    let puts_rest_bottom = matches!(third_rest_words.first(), Some(&"put" | &"puts"))
        && third_rest_words.contains(&"rest")
        && third_rest_words.contains(&"bottom")
        && third_rest_words.contains(&"library");
    if !puts_rest_bottom {
        return Ok(None);
    }
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    effects.push(EffectAst::ChooseObjects {
        filter: choose_filter,
        count: choice_count,
        count_value: None,
        player: chooser,
        tag: chosen_tag.clone(),
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_reveal_tagged(TagKey::from(
            crate::cards::builders::IT_TAG,
        ))],
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            chooser,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_look_at_top_reveal_match_put_top_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((player, count, reveal_top)) =
        parse_top_cards_view_sentence(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let Some(action_match) = parse_leading_may_action_lexed(&second_tokens, &["reveal"], true)
    else {
        return Ok(None);
    };
    let chooser = effect_sentences::leading_may_actor_to_player(action_match.actor, player);
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
    let puts_on_top = (slice_starts_with(after_from_words, &["and", "put", "it", "on", "top"])
        || slice_starts_with(after_from_words, &["put", "it", "on", "top"])
        || slice_starts_with(
            after_from_words,
            &["and", "put", "that", "card", "on", "top"],
        )
        || slice_starts_with(after_from_words, &["put", "that", "card", "on", "top"]))
        && slice_contains(after_from_words, &"library");
    if !puts_on_top {
        return Ok(None);
    }

    let third_words = TokenWordView::new(sentences[sentence_idx + 2].lowered());
    let third_word_refs = third_words.word_refs();
    let third_rest_words = if third_word_refs.first().copied() == Some("then") {
        &third_word_refs[1..]
    } else {
        &third_word_refs[..]
    };
    let puts_rest_bottom = matches!(third_rest_words.first(), Some(&"put" | &"puts"))
        && third_rest_words.contains(&"rest")
        && third_rest_words.contains(&"bottom")
        && third_rest_words.contains(&"library");
    if !puts_rest_bottom {
        return Ok(None);
    }
    let Some(order) = effect_sentences::parse_consult_remainder_order(&third_words.word_refs())
    else {
        return Ok(None);
    };

    let looked_tag = helper_tag_for_tokens(
        sentences[sentence_idx].lowered(),
        if reveal_top { "revealed" } else { "looked" },
    );
    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut choose_filter = filter;
    choose_filter.zone = Some(Zone::Library);
    choose_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: looked_tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });

    let mut effects = vec![EffectAst::subject_verb_look_at_top_cards(
        player,
        count,
        looked_tag.clone(),
    )];
    if reveal_top {
        effects.push(EffectAst::subject_verb_reveal_tagged(looked_tag.clone()));
    }
    effects.push(EffectAst::ChooseObjects {
        filter: choose_filter,
        count: ChoiceCount::up_to(1),
        count_value: None,
        player: chooser,
        tag: chosen_tag.clone(),
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_reveal_tagged(chosen_tag.clone())],
    });
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag.clone(),
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            Zone::Library,
            true,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(chosen_tag),
            order,
            chooser,
        ),
    );

    Ok(Some(effects))
}

pub(crate) fn parse_prefix_then_consult_match_move_and_bottom_remainder(
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

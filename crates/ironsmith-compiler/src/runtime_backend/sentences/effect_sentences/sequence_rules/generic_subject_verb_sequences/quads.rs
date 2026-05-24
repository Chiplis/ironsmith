use super::super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{
    CardTextError, EffectAst, ObjectFilter, PlayerAst, PredicateAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::filter::TaggedObjectConstraint;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::grammar::primitives::TokenWordView;
use crate::runtime_backend::util::{helper_tag_for_tokens, parse_number, trim_commas};
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

fn find_word_sequence(words: &[&str], pattern: &[&str]) -> Option<usize> {
    if pattern.is_empty() || words.len() < pattern.len() {
        return None;
    }
    words
        .windows(pattern.len())
        .position(|window| window == pattern)
}

fn contains_word_sequence(words: &[&str], pattern: &[&str]) -> bool {
    find_word_sequence(words, pattern).is_some()
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

fn named_revealed_card_filter(
    tokens: &[crate::runtime_backend::front_end::lexer::OwnedLexToken],
) -> Option<ObjectFilter> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if !words.starts_with(&["if", "you", "reveal"])
        || !contains_word_sequence(&words, &["this", "way"])
    {
        return None;
    }
    let named_idx = words.iter().position(|word| *word == "named")?;
    let this_way_idx =
        find_word_sequence(&words[named_idx + 1..], &["this", "way"])? + named_idx + 1;
    if named_idx + 1 >= this_way_idx {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_card_name(&words[named_idx + 1..this_way_idx]));
    Some(filter)
}

fn puts_it_onto_battlefield(
    tokens: &[crate::runtime_backend::front_end::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    contains_word_sequence(&words, &["put", "it", "onto", "the", "battlefield"])
        || contains_word_sequence(
            &words,
            &["put", "that", "card", "onto", "the", "battlefield"],
        )
}

fn otherwise_puts_that_card_into_hand(
    tokens: &[crate::runtime_backend::front_end::lexer::OwnedLexToken],
) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let words = if words.first().copied() == Some("otherwise") {
        &words[1..]
    } else {
        words.as_slice()
    };
    words.starts_with(&["put", "that", "card", "into", "your", "hand"])
        || words.starts_with(&["put", "it", "into", "your", "hand"])
}

fn then_shuffle(tokens: &[crate::runtime_backend::front_end::lexer::OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    words == ["then", "shuffle"] || words == ["shuffle"]
}

fn parse_may_reveal_up_to_from_looked_cards(
    tokens: &[crate::runtime_backend::front_end::lexer::OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let tokens = trim_commas(tokens);
    let words = crate::runtime_backend::token_word_refs(&tokens);
    if !words.starts_with(&["you", "may", "reveal", "up", "to"]) {
        return Ok(None);
    }

    let word_view = TokenWordView::new(&tokens);
    let Some(count_start) = word_view.token_index_for_word_index(5) else {
        return Ok(None);
    };
    let (count, count_used) = parse_number(&tokens[count_start..]).ok_or_else(|| {
        CardTextError::ParseError("unable to parse reveal count from looked cards".to_string())
    })?;
    let filter_start = count_start + count_used;
    let Some(from_among_word_idx) = word_view.find_phrase_start(&["from", "among", "them"]) else {
        return Ok(None);
    };
    let filter_end = word_view
        .token_index_for_word_index(from_among_word_idx)
        .unwrap_or(tokens.len());
    let mut filter =
        effect_sentences::parse_looked_card_choice_filter(&tokens[filter_start..filter_end])
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to parse reveal filter from looked cards".to_string(),
                )
            })?;
    filter.zone = Some(Zone::Library);

    Ok(Some((filter, ChoiceCount::up_to(count as usize))))
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

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::ThisSpellWasKicked,
            if_true: vec![
                EffectAst::subject_verb_put_some_into_hand_rest_on_bottom_of_library(
                    player,
                    kicked_count,
                ),
            ],
            if_false: vec![
                EffectAst::subject_verb_put_some_into_hand_rest_on_bottom_of_library(
                    player, base_count,
                ),
            ],
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

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::subject_verb_choose_from_looked_cards_onto_battlefield_or_into_hand_rest_on_bottom_of_library(
            chooser,
            battlefield_filter,
            tapped,
        ),
    ]))
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

    let third_words =
        crate::runtime_backend::token_word_refs(sentences[sentence_idx + 2].lowered());
    let fourth_words =
        crate::runtime_backend::token_word_refs(sentences[sentence_idx + 3].lowered());
    if !third_words.starts_with(&["if", "this", "spell", "was", "bargained"])
        || !contains_word_sequence(
            &third_words,
            &[
                "put",
                "the",
                "revealed",
                "cards",
                "onto",
                "the",
                "battlefield",
            ],
        )
        || !fourth_words.starts_with(&[
            "otherwise",
            "put",
            "the",
            "revealed",
            "cards",
            "into",
            "your",
            "hand",
        ])
        || !then_shuffle(sentences[sentence_idx + 4].lowered())
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
            predicate: PredicateAst::ThisSpellPaidLabel("Bargain".to_string()),
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

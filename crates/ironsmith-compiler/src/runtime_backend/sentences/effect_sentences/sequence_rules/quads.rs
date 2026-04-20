use super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_no_card_into_hand_this_way_sentence,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{
    CardTextError, EffectAst, ObjectFilter, PlayerAst, PredicateAst, TagKey, TargetAst,
};
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::zone::Zone;

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
                EffectAst::RevealTagged { tag } if tag == &searched_tag
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

pub(super) fn parse_look_at_top_put_counted_into_hand_rest_bottom_with_kicker_override(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { player, .. }] = first_effects.as_slice() else {
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
            if_true: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: kicked_count,
            }],
            if_false: vec![EffectAst::PutSomeIntoHandRestOnBottomOfLibrary {
                player: *player,
                count: base_count,
            }],
        },
    ]))
}

pub(super) fn parse_look_at_top_may_put_match_onto_battlefield_then_if_not_put_into_hand_rest_bottom(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(first_effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let [EffectAst::LookAtTopCards { .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

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
        EffectAst::ChooseFromLookedCardsOntoBattlefieldOrIntoHandRestOnBottomOfLibrary {
            player: chooser,
            battlefield_filter,
            tapped,
        },
    ]))
}

pub(super) fn parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) =
        super::triples::parse_look_at_top_reveal_match_put_rest_bottom(sentences, sentence_idx)?
    else {
        return Ok(None);
    };
    let Some(if_not_chosen) =
        parse_if_no_card_into_hand_this_way_sentence(sentences[sentence_idx + 3].lowered())?
    else {
        return Ok(None);
    };

    let Some(EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
        if_not_chosen: existing,
        ..
    }) = effects.get_mut(1)
    else {
        return Ok(None);
    };
    *existing = if_not_chosen;
    Ok(Some(effects))
}

pub(super) fn parse_search_reveal_named_match_battlefield_else_hand_then_shuffle(
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
        if_true: vec![EffectAst::MoveToZone {
            target: TargetAst::Tagged(searched_tag.clone(), None),
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        }],
        if_false: vec![EffectAst::MoveToZone {
            target: TargetAst::Tagged(searched_tag, None),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: crate::cards::builders::ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        }],
    });
    effects.push(EffectAst::ShuffleLibrary {
        player: PlayerAst::You,
    });
    Ok(Some(effects))
}

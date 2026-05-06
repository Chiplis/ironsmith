use super::super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{
    CardTextError, EffectAst, ObjectFilter, PlayerAst, PredicateAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
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

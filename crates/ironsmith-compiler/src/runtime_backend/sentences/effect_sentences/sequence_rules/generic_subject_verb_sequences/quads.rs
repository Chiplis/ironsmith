use super::super::super::dispatch_entry::{
    is_put_rest_on_bottom_of_library_sentence, parse_counted_looked_cards_into_your_hand_tokens,
    parse_if_this_spell_was_kicked_counted_looked_cards_into_hand,
    parse_if_you_dont_put_card_from_among_them_into_your_hand,
};
use crate::cards::builders::{
    CardTextError, EffectAst, IfResultPredicate, LibraryBottomOrderAst, ObjectFilter, PlayerAst,
    PredicateAst, ReturnControllerAst, SubjectVerbActionAst, SubjectVerbEffectAst,
    SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::filter::TaggedObjectConstraint;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::front_end::lexer::{
    LexedClause, OwnedLexToken, word_slice_contains_any_phrase, word_slice_contains_phrase,
    word_slice_eq, word_slice_eq_any, word_slice_starts_with, word_slice_starts_with_any,
};
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::util::{
    helper_tag_for_tokens, non_article_token_word_refs, parse_choice_count_token_prefix_consumed,
    trim_commas,
};
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

const NAMED_REVEALED_PREFIX: &[&str] = &["if", "you", "reveal"];
const THIS_WAY_PHRASE: &[&str] = &["this", "way"];
const PUT_LOOKED_CARD_ONTO_BATTLEFIELD_PHRASES: &[&[&str]] = &[
    &["put", "it", "onto", "the", "battlefield"],
    &["put", "that", "card", "onto", "the", "battlefield"],
];
const PUT_LOOKED_CARD_INTO_HAND_PREFIXES: &[&[&str]] = &[
    &["put", "that", "card", "into", "your", "hand"],
    &["put", "it", "into", "your", "hand"],
];
const THEN_SHUFFLE_CLAUSES: &[&[&str]] = &[&["then", "shuffle"], &["shuffle"]];
const MAY_REVEAL_FROM_LOOKED_CARDS_PREFIX: &[&str] = &["you", "may", "reveal"];
const BARGAINED_PREFIX: &[&str] = &["if", "this", "spell", "was", "bargained"];
const PUT_REVEALED_CARDS_ONTO_BATTLEFIELD_PHRASE: &[&str] = &[
    "put",
    "the",
    "revealed",
    "cards",
    "onto",
    "the",
    "battlefield",
];
const OTHERWISE_PUT_REVEALED_CARDS_INTO_HAND_PREFIX: &[&str] = &[
    "otherwise",
    "put",
    "the",
    "revealed",
    "cards",
    "into",
    "your",
    "hand",
];
const EXILE_ONE_LOOKED_CARD_FACE_DOWN_PREFIX: &[&str] =
    &["exile", "one", "of", "them", "face", "down"];
const PUT_REST_PHRASE: &[&str] = &["put", "rest"];
const BOTTOM_OF_YOUR_LIBRARY_PHRASE: &[&str] = &["bottom", "of", "your", "library"];
const CAST_EXILED_CARD_FREE_PREFIX: &[&str] = &[
    "you", "may", "cast", "exiled", "card", "without", "paying", "its", "mana", "cost",
];
const EXILED_CARD_HAND_FOLLOWUP_CLAUSES: &[&[&str]] = &[
    &[
        "if", "you", "don't", "put", "that", "card", "into", "your", "hand",
    ],
    &[
        "if", "you", "dont", "put", "that", "card", "into", "your", "hand",
    ],
    &[
        "if", "you", "do", "not", "put", "that", "card", "into", "your", "hand",
    ],
];
const EXILE_COUNTED_LOOKED_CARDS_FACE_DOWN_PREFIXES: &[&[&str]] = &[
    &["of", "them", "face", "down"],
    &["of", "those", "cards", "face", "down"],
    &["them", "face", "down"],
    &["those", "cards", "face", "down"],
];

fn named_revealed_card_filter(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if !word_slice_starts_with(&words, NAMED_REVEALED_PREFIX)
        || !word_slice_contains_phrase(&words, THIS_WAY_PHRASE)
    {
        return None;
    }
    let named_idx = clause.find_word("named")?;
    let this_way_idx = clause.find_phrase_start(&["this", "way"])?;
    if named_idx + 1 >= this_way_idx {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_card_name(&words[named_idx + 1..this_way_idx]));
    Some(filter)
}

fn puts_it_onto_battlefield(tokens: &[OwnedLexToken]) -> bool {
    let words = LexedClause::new(tokens).word_refs();
    word_slice_contains_any_phrase(&words, PUT_LOOKED_CARD_ONTO_BATTLEFIELD_PHRASES)
}

fn otherwise_puts_that_card_into_hand(tokens: &[OwnedLexToken]) -> bool {
    let mut clause = LexedClause::new(tokens).trimmed();
    if clause
        .word_refs()
        .first()
        .is_some_and(|word| *word == "otherwise")
    {
        clause = clause.from(1).trimmed();
    }
    word_slice_starts_with_any(&clause.word_refs(), PUT_LOOKED_CARD_INTO_HAND_PREFIXES)
}

fn then_shuffle(tokens: &[OwnedLexToken]) -> bool {
    let words = LexedClause::new(tokens).trimmed().word_refs();
    word_slice_eq_any(&words, THEN_SHUFFLE_CLAUSES)
}

fn exiles_one_looked_card_face_down_and_bottoms_rest(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    let words = non_article_token_word_refs(&trimmed);
    word_slice_starts_with(&words, EXILE_ONE_LOOKED_CARD_FACE_DOWN_PREFIX)
        && word_slice_contains_phrase(&words, PUT_REST_PHRASE)
        && word_slice_contains_phrase(&words, BOTTOM_OF_YOUR_LIBRARY_PHRASE)
}

fn words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    !phrase.is_empty()
        && words
            .windows(phrase.len())
            .any(|window| window == phrase)
}

fn parse_counted_looked_cards_exile_face_down(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, bool)> {
    let trimmed = trim_commas(tokens);
    let clause = LexedClause::new(&trimmed).trimmed();
    if clause.word_refs().first().copied() != Some("exile") {
        return None;
    }

    let count_start = clause.token_index_for_word_index(1)?;
    let count_tokens = trim_commas(&clause.tokens()[count_start..]);
    let (count, used) = parse_choice_count_token_prefix_consumed(&count_tokens)?;
    let tail_tokens = trim_commas(&count_tokens[used..]);
    let tail_words = non_article_token_word_refs(&tail_tokens);
    if !word_slice_starts_with_any(&tail_words, EXILE_COUNTED_LOOKED_CARDS_FACE_DOWN_PREFIXES) {
        return None;
    }
    let includes_remainder = words_contain_phrase(&tail_words, &["put", "rest"])
        || words_contain_phrase(&tail_words, &["put", "the", "rest"]);
    Some((count, includes_remainder))
}

fn puts_looked_remainder_on_bottom(tokens: &[OwnedLexToken]) -> Option<LibraryBottomOrderAst> {
    let trimmed = trim_commas(tokens);
    let words = non_article_token_word_refs(&trimmed);
    let puts_rest = words_contain_phrase(&words, &["put", "rest", "on", "bottom"])
        || words_contain_phrase(&words, &["put", "rest", "onto", "bottom"])
        || words_contain_phrase(&words, &["put", "the", "rest", "on", "bottom"])
        || words_contain_phrase(&words, &["put", "the", "rest", "onto", "bottom"]);
    if !puts_rest || !words.iter().any(|word| *word == "library") {
        return None;
    }
    effect_sentences::parse_consult_remainder_order(&LexedClause::new(&trimmed).word_refs())
}

fn parse_exiled_card_cast_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let trimmed = trim_commas(tokens);
    let clause = LexedClause::new(&trimmed);
    let Some(if_word_idx) = clause.find_word("if") else {
        return Ok(None);
    };
    if !clause.before_word(if_word_idx).is_some_and(|prefix| {
        word_slice_eq(
            &non_article_token_word_refs(prefix.tokens()),
            CAST_EXILED_CARD_FREE_PREFIX,
        )
    }) {
        return Ok(None);
    }

    let Some(condition_token_idx) = clause.token_index_for_word_index(if_word_idx + 1) else {
        return Ok(None);
    };
    let mut condition = trim_commas(&trimmed[condition_token_idx..]);
    if let Some(first) = condition.first().and_then(|token| token.as_word())
        && matches!(first, "it's" | "its" | "it" | "that" | "that's")
    {
        condition = trim_commas(&condition[1..]);
    }
    if let Some(first) = condition.first().and_then(|token| token.as_word())
        && first == "card"
    {
        condition = trim_commas(&condition[1..]);
    }

    let mut filter = parse_object_filter_lexed(&condition, false)?;
    if filter.zone == Some(Zone::Stack) {
        filter.zone = None;
    }
    Ok(Some(filter))
}

fn puts_exiled_card_into_hand_if_not_cast(tokens: &[OwnedLexToken]) -> bool {
    let trimmed = trim_commas(tokens);
    word_slice_eq_any(
        &LexedClause::new(&trimmed).word_refs(),
        EXILED_CARD_HAND_FOLLOWUP_CLAUSES,
    )
}

fn parse_may_reveal_up_to_from_looked_cards(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause = LexedClause::new(tokens).trimmed();
    if !word_slice_starts_with(&clause.word_refs(), MAY_REVEAL_FROM_LOOKED_CARDS_PREFIX) {
        return Ok(None);
    }

    let Some(count_start) = clause.token_index_for_word_index(3) else {
        return Ok(None);
    };
    let tokens = clause.tokens();
    let (count, count_used) = parse_choice_count_token_prefix_consumed(&tokens[count_start..])
        .ok_or_else(|| {
            CardTextError::ParseError("unable to parse reveal count from looked cards".to_string())
        })?;
    let filter_start = count_start + count_used;
    let Some((filter_clause, _)) = clause.split_once_on_phrase(&["from", "among", "them"]) else {
        return Ok(None);
    };
    let filter_end = filter_clause.len();
    let mut filter =
        effect_sentences::parse_looked_card_choice_filter(&tokens[filter_start..filter_end])
            .ok_or_else(|| {
                CardTextError::ParseError(
                    "unable to parse reveal filter from looked cards".to_string(),
                )
            })?;
    filter.zone = Some(Zone::Library);

    Ok(Some((filter, count)))
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

    let third_clause = LexedClause::new(sentences[sentence_idx + 2].lowered());
    let fourth_clause = LexedClause::new(sentences[sentence_idx + 3].lowered());
    let third_words = third_clause.word_refs();
    let fourth_words = fourth_clause.word_refs();
    if !word_slice_starts_with(&third_words, BARGAINED_PREFIX)
        || !word_slice_contains_phrase(&third_words, PUT_REVEALED_CARDS_ONTO_BATTLEFIELD_PHRASE)
        || !word_slice_starts_with(&fourth_words, OTHERWISE_PUT_REVEALED_CARDS_INTO_HAND_PREFIX)
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

pub(crate) fn parse_look_at_top_exile_one_rest_bottom_cast_else_hand(
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
    if !exiles_one_looked_card_face_down_and_bottoms_rest(sentences[sentence_idx + 1].lowered()) {
        return Ok(None);
    }
    let Some(cast_filter) = parse_exiled_card_cast_filter(sentences[sentence_idx + 2].lowered())?
    else {
        return Ok(None);
    };
    if !puts_exiled_card_into_hand_if_not_cast(sentences[sentence_idx + 3].lowered()) {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: choice_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player,
            tag: exiled_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            LibraryBottomOrderAst::Random,
            player,
        ),
        EffectAst::May {
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(exiled_tag.clone(), cast_filter),
                if_true: vec![EffectAst::subject_verb_cast_tagged(
                    exiled_tag.clone(),
                    player,
                    false,
                    false,
                    true,
                    None,
                )],
                if_false: Vec::new(),
            }],
        },
        EffectAst::IfResult {
            predicate: IfResultPredicate::DidNot,
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(exiled_tag, None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
    ]))
}

pub(crate) fn parse_look_at_top_exile_counted_rest_bottom_play_while_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();
    let (look_tokens, exile_count, bottom_order) =
        if let Some(exile_word_idx) = first_clause.find_word("exile") {
            let Some(exile_token_idx) = first_clause.token_index_for_word_index(exile_word_idx)
            else {
                return Ok(None);
            };
            let look_clause = first_clause.before(exile_token_idx).trimmed();
            let exile_clause = first_clause.from(exile_token_idx).trimmed();
            let Some((count, includes_remainder)) =
                parse_counted_looked_cards_exile_face_down(exile_clause.tokens())
            else {
                return Ok(None);
            };
            let order = if includes_remainder {
                puts_looked_remainder_on_bottom(exile_clause.tokens())
            } else {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered())
            };
            let Some(order) = order else {
                return Ok(None);
            };
            (look_clause.tokens(), count, order)
        } else {
            let Some((count, includes_remainder)) =
                parse_counted_looked_cards_exile_face_down(sentences[sentence_idx + 1].lowered())
            else {
                return Ok(None);
            };
            let order = if includes_remainder {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 1].lowered())
            } else {
                puts_looked_remainder_on_bottom(sentences[sentence_idx + 2].lowered())
            };
            let Some(order) = order else {
                return Ok(None);
            };
            (first_clause.tokens(), count, order)
        };

    let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(look_tokens) else {
        return Ok(None);
    };
    let [look_effect] = look_effects.as_slice() else {
        return Ok(None);
    };
    let Some(library_owner) = look_at_top_cards_player(look_effect) else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::LookAtTopCards { count, .. },
        ..
    }) = look_effect
    else {
        return Ok(None);
    };

    let Some(permission_effect) =
        parse_cast_or_play_tagged_clause(sentences[sentence_idx + 3].lowered())?
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
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "exiled");
    let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
    choice_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(
            library_owner,
            count.clone(),
            looked_tag.clone(),
        ),
        EffectAst::ChooseObjects {
            filter: choice_filter,
            count: exile_count,
            count_value: None,
            player: PlayerAst::You,
            tag: exiled_tag.clone(),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(exiled_tag.clone(), None), true),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(exiled_tag.clone()),
            bottom_order,
            PlayerAst::You,
        ),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            exiled_tag,
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
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

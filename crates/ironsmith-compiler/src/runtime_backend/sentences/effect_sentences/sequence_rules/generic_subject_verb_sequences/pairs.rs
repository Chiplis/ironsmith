use super::super::super::dispatch_entry::{
    ConsultSentenceParts, consult_cast_effects, consult_stop_rule_is_single_match,
    find_from_among_looked_cards_phrase, leading_may_actor_to_player, parse_consult_cast_clause,
    parse_consult_remainder_order, parse_consult_traversal_sentence,
    parse_looked_card_choice_filter, parse_looked_card_reveal_filter,
    parse_prefixed_top_of_your_library_count, parse_top_cards_view_sentence,
};
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, IfResultPredicate, LibraryBottomOrderAst,
    ObjectFilter, OwnedLexToken, PlayerAst, PredicateAst, ReturnControllerAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, SubjectVerbRoleAst, SubjectVerbSubjectAst, TagKey,
    TargetAst, ZoneReplacementDurationAst,
};
use crate::effect::Value;
use crate::runtime_backend::activation_and_restrictions::activated_line_core::contains_word_sequence;
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed;
use crate::runtime_backend::lexer::TokenWordView;
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::token_index_for_word_index;
use crate::runtime_backend::token_primitives::{
    find_index, parse_leading_may_action_lexed, slice_contains, slice_ends_with, slice_starts_with,
    word_view_has_any_prefix,
};
use crate::runtime_backend::util::trim_commas;
use crate::runtime_backend::util::{helper_tag_for_tokens, is_article, parse_subject};
use crate::target::{ChooseSpec, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

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

fn find_word_sequence(words: &[&str], pattern: &[&str]) -> Option<usize> {
    if pattern.is_empty() {
        return Some(0);
    }
    words
        .windows(pattern.len())
        .position(|window| window == pattern)
}

fn sentence_words(tokens: &[OwnedLexToken]) -> Vec<&str> {
    TokenWordView::new(tokens).word_refs()
}

fn token_index_for_word(tokens: &[OwnedLexToken], word_idx: usize) -> Option<usize> {
    TokenWordView::new(tokens).token_index_for_word_index(word_idx)
}

fn strip_leading_you_may(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let words = sentence_words(tokens);
    let prefix_len = if words.as_slice().starts_with(&["you", "may"]) {
        2
    } else if words.as_slice().starts_with(&["that", "player", "may"]) {
        3
    } else if words.as_slice().starts_with(&["they", "may"]) {
        2
    } else {
        return None;
    };
    let start = token_index_for_word(tokens, prefix_len).unwrap_or(tokens.len());
    Some(trim_commas(&tokens[start..]))
}

fn parse_optional_consult_traversal_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<(ConsultSentenceParts, bool)>, CardTextError> {
    if let Some(parts) = parse_consult_traversal_sentence(tokens)? {
        return Ok(Some((parts, false)));
    }
    let Some(stripped) = strip_leading_you_may(tokens) else {
        return Ok(None);
    };
    parse_consult_traversal_sentence(&stripped).map(|parts| parts.map(|parts| (parts, true)))
}

fn strip_leading_if_you_do_sentence(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let stripped = crate::runtime_backend::token_primitives::strip_leading_if_you_do_lexed(tokens);
    let was_stripped = stripped.len() != tokens.len();
    (trim_commas(stripped), was_stripped)
}

fn wrap_optional_consult_effects(
    parts: ConsultSentenceParts,
    optional: bool,
    followups: Vec<EffectAst>,
    gate_on_result: bool,
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
    effects
}

fn strip_controlled_by_same_player_suffix(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let words = sentence_words(tokens);
    let suffix_len = if words.ends_with(&["controlled", "by", "the", "same", "player"]) {
        5
    } else if words.ends_with(&["controlled", "by", "same", "player"]) {
        4
    } else {
        return None;
    };
    let suffix_word_start = words.len().checked_sub(suffix_len)?;
    let suffix_token_start = token_index_for_word(tokens, suffix_word_start)?;
    Some(trim_commas(&tokens[..suffix_token_start]))
}

pub(crate) fn parse_look_at_top_then_exile_face_down_then_play_while_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_words = sentence_words(&first_tokens);
    let Some(then_idx) = find_word_sequence(&first_words, &["then", "exile"]) else {
        return Ok(None);
    };
    let Some(then_token_idx) = token_index_for_word(&first_tokens, then_idx) else {
        return Ok(None);
    };
    let Some(exile_token_idx) = token_index_for_word(&first_tokens, then_idx + 1) else {
        return Ok(None);
    };

    let look_tokens = trim_commas(&first_tokens[..then_token_idx]);
    let exile_tokens = trim_commas(&first_tokens[exile_token_idx..]);
    let exile_words: Vec<&str> = sentence_words(&exile_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let exiles_looked_card_face_down = exile_words.as_slice() == ["exile", "it", "face", "down"]
        || exile_words.as_slice() == ["exile", "that", "card", "face", "down"];
    if !exiles_looked_card_face_down {
        return Ok(None);
    }

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
                allow_any_color_for_cast,
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
            allow_any_color_for_cast,
        ),
    ]))
}

pub(crate) fn parse_look_at_top_then_put_one_hand_other_bottom(
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
    let words = sentence_words(&second_tokens);
    let starts_with_hand_choice =
        slice_starts_with(
            &words,
            &["put", "one", "of", "them", "into", "your", "hand"],
        ) || slice_starts_with(&words, &["put", "one", "into", "your", "hand"]);
    if !starts_with_hand_choice {
        return Ok(None);
    }
    let content_words = words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    let puts_other_bottom = contains_word_sequence(&content_words, &["other", "on", "bottom"])
        || contains_word_sequence(&content_words, &["other", "onto", "bottom"])
        || contains_word_sequence(&content_words, &["rest", "on", "bottom"])
        || contains_word_sequence(&content_words, &["rest", "onto", "bottom"]);
    if !puts_other_bottom || !slice_contains(&content_words, &"library") {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player,
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
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(hand_tag),
            LibraryBottomOrderAst::ChooserChooses,
            player,
        ),
    ]))
}

pub(crate) fn parse_look_at_top_then_put_one_hand_other_graveyard(
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
    let words = sentence_words(&second_tokens);
    let starts_with_hand_choice =
        slice_starts_with(
            &words,
            &["put", "one", "of", "them", "into", "your", "hand"],
        ) || slice_starts_with(&words, &["put", "one", "into", "your", "hand"]);
    if !starts_with_hand_choice {
        return Ok(None);
    }
    let content_words = words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    let puts_other_graveyard =
        contains_word_sequence(&content_words, &["other", "into", "graveyard"])
            || contains_word_sequence(&content_words, &["other", "into", "your", "graveyard"])
            || contains_word_sequence(&content_words, &["rest", "into", "your", "graveyard"])
            || contains_word_sequence(&content_words, &["rest", "into", "graveyard"]);
    if !puts_other_graveyard {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "looked");
    let hand_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "hand");
    let mut hand_filter = ObjectFilter::tagged(looked_tag.clone());
    hand_filter.zone = Some(Zone::Library);
    let mut in_chosen_filter = ObjectFilter::default();
    in_chosen_filter
        .tagged_constraints
        .push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameStableId,
        });

    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::ChooseObjects {
            filter: hand_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player,
            tag: hand_tag.clone(),
        },
        EffectAst::ForEachTagged {
            tag: hand_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(hand_tag, in_chosen_filter),
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
    ]))
}

pub(crate) fn parse_choose_same_controller_targets_then_sacrifice_one(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    if !first_tokens
        .first()
        .is_some_and(|token| token.is_word("choose"))
    {
        return Ok(None);
    }
    let Some(first_without_controller_tail) = strip_controlled_by_same_player_suffix(&first_tokens)
    else {
        return Ok(None);
    };
    if first_without_controller_tail.len() <= 1 {
        return Ok(None);
    }
    let target = effect_sentences::parse_target_phrase(&first_without_controller_tail[1..])?;
    let TargetAst::WithCount(_, target_count) = &target else {
        return Ok(None);
    };
    if target_count.min != 2 || target_count.max != Some(2) || target_count.is_random() {
        return Ok(None);
    }

    let second_tokens = trim_commas(sentences[sentence_idx + 1].lowered());
    let second_words = sentence_words(&second_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect::<Vec<_>>();
    if !matches!(
        second_words.as_slice(),
        [
            "that",
            "player",
            "sacrifices",
            "one",
            "of",
            "them",
            "of",
            "their",
            "choice"
        ] | [
            "that",
            "player",
            "sacrifice",
            "one",
            "of",
            "them",
            "of",
            "their",
            "choice"
        ]
    ) {
        return Ok(None);
    }

    let chosen_tag = helper_tag_for_tokens(&second_tokens, "chosen");
    Ok(Some(vec![
        EffectAst::subject_verb_target_only(target),
        EffectAst::ChooseObjects {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::ItsController,
            tag: chosen_tag.clone(),
        },
        EffectAst::subject_verb_sacrifice(
            PlayerAst::That,
            ObjectFilter::tagged(chosen_tag),
            1,
            None,
        ),
    ]))
}

#[derive(Clone, Copy)]
enum RestAction {
    Destroy,
    Exile,
    Sacrifice,
}

fn parse_rest_action_sentence(tokens: &[OwnedLexToken]) -> Option<RestAction> {
    let words = sentence_words(tokens);
    let words = if words.first().copied() == Some("then") {
        &words[1..]
    } else {
        words.as_slice()
    };
    match words {
        ["destroy", "the", "rest"] | ["destroy", "rest"] => Some(RestAction::Destroy),
        ["exile", "the", "rest"] | ["exile", "rest"] => Some(RestAction::Exile),
        ["sacrifice", "the", "rest"]
        | ["sacrifice", "rest"]
        | ["sacrifices", "the", "rest"]
        | ["sacrifices", "rest"] => Some(RestAction::Sacrifice),
        _ => None,
    }
}

fn rest_action_effect(action: RestAction, filter: ObjectFilter, player: PlayerAst) -> EffectAst {
    match action {
        RestAction::Destroy => EffectAst::subject_verb_destroy_all(filter),
        RestAction::Exile => EffectAst::subject_verb_exile_all(filter, false),
        RestAction::Sacrifice => EffectAst::subject_verb_sacrifice_all(player, filter),
    }
}

fn append_rest_action_after_choice(
    effect: EffectAst,
    action: RestAction,
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

pub(crate) fn parse_choose_then_affect_rest(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(action) = parse_rest_action_sentence(sentences[sentence_idx + 1].lowered()) else {
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

pub(crate) fn parse_may_cast_target_graveyard_spell_then_exile_replacement(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = trim_commas(sentences[sentence_idx].lowered());
    let second = trim_commas(sentences[sentence_idx + 1].lowered());
    let first_words = sentence_words(&first);
    let second_words = sentence_words(&second);

    let has_from_graveyard = first_words
        .windows(3)
        .any(|window| window == ["from", "your", "graveyard"]);
    let without_paying_mana_cost = first_words
        .windows(5)
        .any(|window| window == ["without", "paying", "its", "mana", "cost"]);
    let first_is_targeted_graveyard_cast = first_words
        .starts_with(&["you", "may", "cast", "target"])
        && has_from_graveyard
        && first_words.contains(&"instant")
        && first_words.contains(&"sorcery")
        && first_words.contains(&"card");
    if !first_is_targeted_graveyard_cast {
        return Ok(None);
    }
    let second_is_that_spell_replacement = second_words.as_slice()
        == [
            "if",
            "that",
            "spell",
            "would",
            "be",
            "put",
            "into",
            "your",
            "graveyard",
            "exile",
            "it",
            "instead",
        ];
    let second_is_cast_this_way_replacement = second_words.as_slice()
        == [
            "if",
            "an",
            "instant",
            "or",
            "sorcery",
            "spell",
            "cast",
            "this",
            "way",
            "would",
            "be",
            "put",
            "into",
            "your",
            "graveyard",
            "exile",
            "it",
            "instead",
        ];
    if !second_is_that_spell_replacement && !second_is_cast_this_way_replacement {
        return Ok(None);
    }

    let tag = TagKey::from(crate::cards::builders::IT_TAG);
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    if first_words.contains(&"artifact") {
        filter.card_types.push(CardType::Artifact);
    }

    let replacement_filter = ObjectFilter {
        zone: Some(Zone::Stack),
        card_types: vec![CardType::Instant, CardType::Sorcery],
        tagged_constraints: vec![TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        }],
        ..ObjectFilter::default()
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::May {
            effects: vec![EffectAst::subject_verb_cast_tagged(
                tag.clone(),
                PlayerAst::You,
                false,
                false,
                without_paying_mana_cost,
                None,
            )],
        },
        EffectAst::subject_verb_register_future_zone_replacement(
            replacement_filter,
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            ZoneReplacementDurationAst::OneShot,
        ),
    ]))
}

fn previous_sentence_chose_stack_object(sentences: &[SentenceInput], sentence_idx: usize) -> bool {
    if sentence_idx == 0 {
        return false;
    }
    let words = sentence_words(sentences[sentence_idx - 1].lowered());
    words.iter().enumerate().any(|(idx, word)| {
        *word == "target"
            && words[idx + 1..words.len().min(idx + 6)]
                .iter()
                .any(|tail| matches!(*tail, "spell" | "ability"))
    })
}

fn target_for_referenced_stack_object(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    words: &[&str],
) -> TargetAst {
    if words == ["this", "spell"] || words == ["this", "ability"] {
        return TargetAst::Source(None);
    }
    if previous_sentence_chose_stack_object(sentences, sentence_idx) {
        return TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);
    }
    TargetAst::Tagged(TagKey::from("triggering"), None)
}

fn strip_could_target_suffix(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words = sentence_words(tokens);
    for phrase in [
        ["that", "spell", "could", "target"].as_slice(),
        ["that", "ability", "could", "target"].as_slice(),
        ["that", "spell", "or", "ability", "could", "target"].as_slice(),
        ["the", "spell", "could", "target"].as_slice(),
        ["the", "ability", "could", "target"].as_slice(),
        ["it", "could", "target"].as_slice(),
    ] {
        if let Some(word_idx) = find_word_sequence(&words, phrase) {
            let start_word_idx = if word_idx > 0 && words[word_idx - 1] == "that" {
                word_idx - 1
            } else {
                word_idx
            };
            if let Some(token_idx) = token_index_for_word(tokens, start_word_idx) {
                return trim_commas(&tokens[..token_idx]);
            }
        }
    }
    trim_commas(tokens)
}

fn strip_leading_other(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let trimmed = trim_commas(tokens);
    if let Some(first) = trimmed.first()
        && (first.is_word("other") || first.is_word("another"))
    {
        return (trim_commas(&trimmed[1..]), true);
    }
    (trimmed, false)
}

fn parse_copy_for_each_candidate_filter(
    tokens: &[OwnedLexToken],
) -> Result<(Option<ObjectFilter>, Option<PlayerFilter>, bool), CardTextError> {
    let stripped = strip_could_target_suffix(tokens);
    let (candidate_tokens, exclude_current_targets) = strip_leading_other(&stripped);
    let candidate_words = sentence_words(&candidate_tokens);
    let has_player = candidate_words
        .iter()
        .any(|word| matches!(*word, "player" | "players"));
    let has_permanent = candidate_words
        .iter()
        .any(|word| matches!(*word, "permanent" | "permanents"));

    if has_player && has_permanent {
        return Ok((
            Some(ObjectFilter::permanent()),
            Some(PlayerFilter::Any),
            exclude_current_targets,
        ));
    }
    if has_player && !candidate_words.iter().any(|word| *word == "creature") {
        return Ok((None, Some(PlayerFilter::Any), exclude_current_targets));
    }

    let mut filter = parse_object_filter_lexed(&candidate_tokens, false)?;
    filter.other = false;
    filter.could_be_targeted_by = None;
    Ok((Some(filter), None, exclude_current_targets))
}

fn parse_copy_for_each_target_sentence(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = trim_commas(tokens);
    let words = sentence_words(&tokens);
    let wrap_if_result = words.starts_with(&["if", "you", "do"]);
    let Some(for_each_word_idx) = find_word_sequence(&words, &["for", "each"]) else {
        return Ok(None);
    };
    let Some(copy_word_idx) = words
        .iter()
        .position(|word| *word == "copy" || *word == "copies")
    else {
        return Ok(None);
    };
    if copy_word_idx < for_each_word_idx {
        let Some(copy_token_idx) = token_index_for_word(&tokens, copy_word_idx) else {
            return Ok(None);
        };
        let Some(for_each_token_idx) = token_index_for_word(&tokens, for_each_word_idx) else {
            return Ok(None);
        };
        let subject = parse_subject(&tokens[..copy_token_idx]);
        let player = match subject {
            SubjectAst::Player(player) => player,
            SubjectAst::This => PlayerAst::Implicit,
        };
        let target_tokens = trim_commas(&tokens[copy_token_idx + 1..for_each_token_idx]);
        let target_words = sentence_words(&target_tokens);
        let target = target_for_referenced_stack_object(sentences, sentence_idx, &target_words);
        let candidate_tokens = trim_commas(&tokens[for_each_token_idx + 2..]);
        let (object_filter, player_filter, exclude_current_targets) =
            parse_copy_for_each_candidate_filter(&candidate_tokens)?;
        let effect = EffectAst::subject_verb_copy_spell_for_each_target(
            target,
            object_filter,
            player_filter,
            player,
            exclude_current_targets,
            Vec::new(),
        );
        return Ok(Some(if wrap_if_result {
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![effect],
            }
        } else {
            effect
        }));
    }

    let Some(for_each_token_idx) = token_index_for_word(&tokens, for_each_word_idx) else {
        return Ok(None);
    };
    let Some(put_copy_word_idx) = find_word_sequence(&words, &["put", "a", "copy"]) else {
        return Ok(None);
    };
    let Some(put_copy_token_idx) = token_index_for_word(&tokens, put_copy_word_idx) else {
        return Ok(None);
    };
    let candidate_tokens = trim_commas(&tokens[for_each_token_idx + 2..put_copy_token_idx]);
    let after_copy_words = &words[put_copy_word_idx + 3..];
    let of_offset = after_copy_words.iter().position(|word| *word == "of");
    let target_start_word_idx = of_offset
        .map(|offset| put_copy_word_idx + 3 + offset + 1)
        .unwrap_or(put_copy_word_idx + 3);
    let onto_rel = find_word_sequence(&words[target_start_word_idx..], &["onto", "the", "stack"])
        .unwrap_or(words.len().saturating_sub(target_start_word_idx));
    let target_end_word_idx = target_start_word_idx + onto_rel;
    let Some(target_start_token_idx) = token_index_for_word(&tokens, target_start_word_idx) else {
        return Ok(None);
    };
    let target_end_token_idx =
        token_index_for_word(&tokens, target_end_word_idx).unwrap_or_else(|| tokens.len());
    let target_tokens = trim_commas(&tokens[target_start_token_idx..target_end_token_idx]);
    let target_words = sentence_words(&target_tokens);
    let target = target_for_referenced_stack_object(sentences, sentence_idx, &target_words);
    let (object_filter, player_filter, exclude_current_targets) =
        parse_copy_for_each_candidate_filter(&candidate_tokens)?;
    let effect = EffectAst::subject_verb_copy_spell_for_each_target(
        target,
        object_filter,
        player_filter,
        PlayerAst::Implicit,
        exclude_current_targets,
        Vec::new(),
    );
    Ok(Some(if wrap_if_result {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![effect],
        }
    } else {
        effect
    }))
}

fn each_copy_targets_different_one_of_those(tokens: &[OwnedLexToken]) -> bool {
    let words = sentence_words(tokens);
    find_word_sequence(
        &words,
        &[
            "each",
            "copy",
            "targets",
            "a",
            "different",
            "one",
            "of",
            "those",
        ],
    )
    .is_some()
}

pub(crate) fn parse_copy_for_each_target_then_each_copy_targets_different(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !each_copy_targets_different_one_of_those(sentences[sentence_idx + 1].lowered()) {
        return Ok(None);
    }
    let Some(effect) = parse_copy_for_each_target_sentence(
        sentences,
        sentence_idx,
        sentences[sentence_idx].lowered(),
    )?
    else {
        return Ok(None);
    };
    Ok(Some(vec![effect]))
}

fn first_sentence_copies_for_each_tagged_object(tokens: &[OwnedLexToken]) -> bool {
    let words = sentence_words(tokens);
    (find_word_sequence(&words, &["for", "each", "of", "those"]).is_some()
        || find_word_sequence(&words, &["for", "each", "of", "them"]).is_some()
        || (find_word_sequence(&words, &["for", "each"]).is_some()
            && find_word_sequence(&words, &["chosen", "this", "way"]).is_some()))
        && words
            .iter()
            .any(|word| *word == "copy" || *word == "copies")
}

fn second_sentence_copy_targets_iterated_object(tokens: &[OwnedLexToken]) -> bool {
    let words = sentence_words(tokens);
    words.starts_with(&["the", "copy", "targets", "that"])
        || words.starts_with(&["the", "copy", "targets", "the", "chosen"])
}

pub(crate) fn parse_for_each_tagged_copy_then_copy_targets_it(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_words = sentence_words(&first_tokens);
    if !first_sentence_copies_for_each_tagged_object(&first_tokens)
        || !second_sentence_copy_targets_iterated_object(sentences[sentence_idx + 1].lowered())
    {
        return Ok(None);
    }

    let wrap_if_result = first_words.starts_with(&["if", "you", "do"]);
    let Some(copy_word_idx) = first_words
        .iter()
        .position(|word| *word == "copy" || *word == "copies")
    else {
        return Ok(None);
    };
    let Some(copy_token_idx) = token_index_for_word(&first_tokens, copy_word_idx) else {
        return Ok(None);
    };
    let copy_target_tokens = trim_commas(&first_tokens[copy_token_idx + 1..]);
    let copy_target_words = sentence_words(&copy_target_tokens);
    let copy_effect = EffectAst::subject_verb_copy_spell(
        target_for_referenced_stack_object(sentences, sentence_idx, &copy_target_words),
        Value::Fixed(1),
        PlayerAst::You,
        false,
        Vec::new(),
    );

    let second_effects =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx + 1].lowered())?;
    let [
        retarget @ EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RetargetStackObject { .. },
            ..
        }),
    ] = second_effects.as_slice()
    else {
        return Ok(None);
    };
    let for_each = EffectAst::ForEachTagged {
        tag: TagKey::from(crate::cards::builders::IT_TAG),
        effects: vec![copy_effect, retarget.clone()],
    };

    Ok(Some(vec![if wrap_if_result {
        EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![for_each],
        }
    } else {
        for_each
    }]))
}

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
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    power,
                    toughness,
                    target,
                    card_types,
                    subtypes,
                    colors,
                    abilities,
                    granted_abilities,
                    duration,
                },
            ..
        }) => {
            let target = match target {
                TargetAst::Tagged(tag, span) if tag.as_str() == crate::cards::builders::IT_TAG => {
                    TargetAst::Source(span)
                }
                target => target,
            };
            EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                card_types,
                subtypes,
                colors,
                abilities,
                granted_abilities,
                duration,
            )
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
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::GainLife { .. },
            ..
        }) => true,
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
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::BecomeBasePtCreature {
                    target, duration, ..
                },
            ..
        }) => {
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
        EffectAst::IfResult { effects, .. } => effects.iter().any(contains_tagged_source_animation),
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

pub(crate) fn parse_whenever_gain_life_then_self_animate_source(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();

    let first_words = sentence_words(first);
    if !first_words
        .iter()
        .any(|word| *word == "gain" || *word == "gains")
        || !first_words.iter().any(|word| *word == "life")
    {
        return Ok(None);
    }

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first)?;
    if !first_effects
        .iter()
        .any(contains_triggered_life_gain_effect)
    {
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

pub(crate) fn parse_gain_life_then_self_animate_source(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();

    let first_words = sentence_words(first);
    if !first_words
        .iter()
        .any(|word| *word == "gain" || *word == "gains")
        || !first_words.iter().any(|word| *word == "life")
    {
        return Ok(None);
    }

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first)?;
    if !first_effects
        .iter()
        .any(contains_triggered_life_gain_effect)
    {
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

pub(crate) fn parse_choose_then_do_same_for_filter_then_return_to_battlefield(
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

    effects.push(EffectAst::subject_verb_return_to_battlefield(
        TargetAst::Tagged(
            TagKey::from(crate::cards::builders::IT_TAG),
            effect_sentences::span_from_tokens(sentences[sentence_idx + 1].lowered()),
        ),
        tapped,
        false,
        false,
        ReturnControllerAst::Preserve,
        None,
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_delayed_dies_exile_top_power_choose_play(
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
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(
                    crate::cards::builders::IT_TAG,
                )))),
                looked_tag.clone(),
            ),
            EffectAst::subject_verb_exile(TargetAst::Tagged(looked_tag, None), false),
            EffectAst::ChooseObjects {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
                chosen_tag,
                PlayerAst::You,
                true,
                false,
            ),
        ],
    }]))
}

pub(crate) fn parse_mill_then_may_put_from_among_into_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::Mill { .. },
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };

    let Some((chooser, filter)) =
        parse_may_put_filtered_card_from_among_into_hand(second, *player, Zone::Graveyard)?
    else {
        return Ok(None);
    };

    Ok(Some(vec![
        first_effects[0].clone(),
        EffectAst::subject_verb_choose_from_looked_cards_into_hand_rest_into_graveyard(
            chooser,
            filter,
            false,
            Vec::new(),
        ),
    ]))
}

pub(crate) fn parse_exile_until_match_grant_play_this_turn(
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
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                    stop_rule,
                    ..
                },
            ..
        })) if consult_stop_rule_is_single_match(stop_rule)
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

pub(crate) fn parse_target_player_chooses_then_other_cant_block(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_target_player_chooses_then_other_cant_block(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(crate) fn parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
        sentences[sentence_idx].lowered(),
        sentences[sentence_idx + 1].lowered(),
    )
}

pub(crate) fn parse_choose_creature_type_then_become_type(
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
    let has_graveyard_rest_clause =
        contains_word_sequence(after_revealed, &["and", "the", "rest", "into", "your"])
            && slice_contains(after_revealed, &"graveyard");
    let bottom_order = parse_consult_remainder_order(after_revealed);
    if !has_hand_clause || (!has_graveyard_rest_clause && bottom_order.is_none()) {
        return Ok(None);
    }

    let effect = if let Some(order) = bottom_order {
        EffectAst::subject_verb_reveal_top_put_matching_into_hand_rest_on_bottom_of_library(
            PlayerAst::You,
            count,
            filter,
            order,
        )
    } else {
        EffectAst::subject_verb_reveal_top_put_matching_into_hand_rest_into_graveyard(
            PlayerAst::You,
            count,
            filter,
        )
    };

    Ok(Some(vec![effect]))
}

pub(crate) fn parse_consult_match_move_and_bottom_remainder(
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
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words = crate::runtime_backend::token_word_refs(&second_tokens);
    let puts_all_revealed_matching_onto_battlefield =
        crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &["put", "all"],
        )
        .is_some()
            && contains_word_sequence(&second_words, &["cards", "revealed", "this", "way"])
            && (contains_word_sequence(&second_words, &["onto", "the", "battlefield"])
                || contains_word_sequence(&second_words, &["onto", "battlefield"]))
            && crate::runtime_backend::grammar::primitives::contains_word(
                &second_tokens,
                "shuffle",
            )
            && crate::runtime_backend::grammar::primitives::contains_word(&second_tokens, "rest")
            && crate::runtime_backend::grammar::primitives::contains_word(
                &second_tokens,
                "library",
            );
    if puts_all_revealed_matching_onto_battlefield {
        let mut effects = parts.effects;
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag, None),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            parts.player,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(effects));
    }

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
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        zone,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        battlefield_tapped,
        None,
    ));
    effects.push(
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            parts.all_tag,
            Some(parts.match_tag),
            order,
            parts.player,
        ),
    );
    Ok(Some(effects))
}

pub(crate) fn parse_conditional_consult_match_move_and_bottom_remainder(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let conditional_tokens = if first_tokens.len() >= 2
        && first_tokens[0].is_word("then")
        && first_tokens[1].is_word("if")
    {
        &first_tokens[1..]
    } else if first_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        first_tokens.as_slice()
    } else {
        return Ok(None);
    };

    let Some(comma_idx) = conditional_tokens.iter().position(|token| token.is_comma()) else {
        return Ok(None);
    };
    if comma_idx <= 1 {
        return Ok(None);
    }

    let predicate_tokens = trim_commas(&conditional_tokens[1..comma_idx]);
    let effect_tokens = trim_commas(&conditional_tokens[comma_idx + 1..]);
    if predicate_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }

    let Ok(predicate) = parse_predicate_with_grammar_entrypoint_lexed(&predicate_tokens) else {
        return Ok(None);
    };

    let synthetic = [
        SentenceInput::from_lexed(&effect_tokens),
        SentenceInput::from_lexed(sentences[sentence_idx + 1].lowered()),
    ];
    let Some(if_true) = parse_consult_match_move_and_bottom_remainder(&synthetic, 0)? else {
        return Ok(None);
    };

    Ok(Some(vec![EffectAst::Conditional {
        predicate,
        if_true,
        if_false: Vec::new(),
    }]))
}

pub(crate) fn parse_consult_match_move_all_to_graveyard(
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
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.all_tag, None),
        Zone::Graveyard,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Ok(Some(effects))
}

pub(crate) fn parse_consult_match_into_hand_exile_others(
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
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, _gate_on_result) = strip_leading_if_you_do_sentence(second);
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
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        Zone::Hand,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::ForEachTagged {
        tag: parts.all_tag,
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::TaggedMatches(
                crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                ObjectFilter::tagged(parts.match_tag),
            ),
            if_true: Vec::new(),
            if_false: vec![EffectAst::subject_verb_exile(
                TargetAst::Tagged(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    None,
                ),
                false,
            )],
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
pub(crate) fn parse_consult_match_into_hand_others_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some((parts, optional)) = parse_optional_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, gate_on_result) = strip_leading_if_you_do_sentence(second);
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

    let followups = vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            Zone::Hand,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::ForEachTagged {
            tag: parts.all_tag.clone(),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    ObjectFilter::tagged(parts.match_tag.clone()),
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(
                        crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                        None,
                    ),
                    Zone::Graveyard,
                    false,
                    crate::cards::builders::ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ];
    Ok(Some(wrap_optional_consult_effects(
        parts,
        optional,
        followups,
        gate_on_result,
    )))
}

pub(crate) fn parse_consult_match_into_battlefield_others_graveyard(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let second = sentences[sentence_idx + 1].lowered();
    let Some((parts, optional)) = parse_optional_consult_traversal_sentence(first)? else {
        return Ok(None);
    };
    if !matches!(
        parts.effects.last(),
        Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let (second_tokens, gate_on_result) = strip_leading_if_you_do_sentence(second);
    let moves_to_battlefield = crate::runtime_backend::grammar::primitives::words_match_prefix(
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
            &[
                "the",
                "player",
                "puts",
                "that",
                "card",
                "onto",
                "the",
                "battlefield",
            ],
        )
        .is_some()
        || crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &[
                "that",
                "player",
                "puts",
                "that",
                "card",
                "onto",
                "the",
                "battlefield",
            ],
        )
        .is_some();
    let second_words = crate::runtime_backend::token_word_refs(&second_tokens);
    let others_to_graveyard = (contains_word_sequence(&second_words, &["other", "cards"])
        || contains_word_sequence(&second_words, &["all", "other"]))
        && slice_contains(&second_words, &"graveyard");
    if !moves_to_battlefield || !others_to_graveyard {
        return Ok(None);
    }

    let followups = vec![
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::ForEachTagged {
            tag: parts.all_tag.clone(),
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                    ObjectFilter::tagged(parts.match_tag.clone()),
                ),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(
                        crate::cards::builders::TagKey::from(crate::cards::builders::IT_TAG),
                        None,
                    ),
                    Zone::Graveyard,
                    false,
                    crate::cards::builders::ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ];
    Ok(Some(wrap_optional_consult_effects(
        parts,
        optional,
        followups,
        gate_on_result,
    )))
}

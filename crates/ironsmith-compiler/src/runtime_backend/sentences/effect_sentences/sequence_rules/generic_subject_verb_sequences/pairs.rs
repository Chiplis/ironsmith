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
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::effect_sentences::SentenceInput;
use crate::runtime_backend::front_end::lexer::{
    LexedClause, TokenKind, find_token_kind, word_slice_contains_all_words,
    word_slice_contains_any_phrase, word_slice_contains_any_word, word_slice_contains_phrase,
    word_slice_contains_word, word_slice_eq, word_slice_eq_any, word_slice_find_any_word,
    word_slice_find_phrase_start, word_slice_find_word, word_slice_starts_with,
    word_slice_starts_with_any,
};
use crate::runtime_backend::grammar::structure::parse_predicate_with_grammar_entrypoint_lexed;
use crate::runtime_backend::lexer::TokenWordView;
use crate::runtime_backend::object_filters::parse_object_filter_lexed;
use crate::runtime_backend::permission_helpers::parse_cast_or_play_tagged_clause;
use crate::runtime_backend::token_primitives::{
    find_index, parse_leading_may_action_lexed, word_view_has_any_prefix,
};
use crate::runtime_backend::util::{
    helper_tag_for_tokens, is_article, non_article_token_word_refs, non_article_word_refs,
    parse_subject, strip_leading_token_word_once_any, word_refs_except,
};
use crate::runtime_backend::util::{parse_choice_count_token_prefix_consumed, trim_commas};
use crate::target::{ChooseSpec, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

const TAPPED_WORD: &str = "tapped";

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

fn token_range_for_word_range(
    tokens: &[OwnedLexToken],
    start_word: usize,
    end_word: usize,
) -> Option<&[OwnedLexToken]> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if start_word >= end_word || end_word > words.len() {
        return None;
    }
    let start = clause.token_index_for_word_index(start_word)?;
    let end = if end_word == words.len() {
        tokens.len()
    } else {
        clause.token_index_for_word_index(end_word)?
    };
    Some(&tokens[start..end])
}

pub(crate) fn parse_directional_adjacent_player_control(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let choice_sentence = sentences[sentence_idx].lowered();
    let gain_sentence = sentences[sentence_idx + 1].lowered();

    let choice_prefix = [
        "starting",
        "with",
        "you",
        "and",
        "proceeding",
        "in",
        "the",
        "chosen",
        "direction",
        "each",
        "player",
        "chooses",
    ];
    let choice_suffix = [
        "controlled",
        "by",
        "the",
        "next",
        "player",
        "in",
        "that",
        "direction",
    ];
    let choice_words = LexedClause::new(choice_sentence).word_refs();
    if !choice_words.starts_with(&choice_prefix) || choice_words.len() <= choice_suffix.len() {
        return Ok(None);
    }
    let choice_suffix_start = choice_words.len() - choice_suffix.len();
    if choice_words[choice_suffix_start..] != choice_suffix {
        return Ok(None);
    }

    let Some(object_tokens) =
        token_range_for_word_range(choice_sentence, choice_prefix.len(), choice_suffix_start)
    else {
        return Ok(None);
    };
    let object_tokens = trim_commas(object_tokens);
    let object_words = LexedClause::new(&object_tokens).word_refs();
    let filter = parse_object_filter_lexed(&object_tokens, false)?;

    let gain_prefix = ["each", "player", "gains", "control", "of"];
    let gain_suffix = ["they", "chose"];
    let gain_words = LexedClause::new(gain_sentence).word_refs();
    if !gain_words.starts_with(&gain_prefix) || gain_words.len() <= gain_suffix.len() {
        return Ok(None);
    }
    let gain_suffix_start = gain_words.len() - gain_suffix.len();
    if gain_words[gain_suffix_start..] != gain_suffix {
        return Ok(None);
    }
    let Some(gained_object_tokens) =
        token_range_for_word_range(gain_sentence, gain_prefix.len(), gain_suffix_start)
    else {
        return Ok(None);
    };
    let gained_object_words = LexedClause::new(gained_object_tokens).word_refs();
    if non_article_word_refs(&gained_object_words) != non_article_word_refs(&object_words) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::DirectionalAdjacentPlayerControl {
        filter,
        left_option: "left".to_string(),
        right_option: "right".to_string(),
    }]))
}

fn strip_leading_you_may(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    let clause = LexedClause::new(tokens);
    let (_, tail) = clause.strip_any_prefix_clause(&[
        &["you", "may"],
        &["that", "player", "may"],
        &["they", "may"],
    ])?;
    Some(tail.trim())
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
    let clause = LexedClause::new(tokens);
    let (_, head) = clause.strip_any_suffix_clause(&[
        &["controlled", "by", "the", "same", "player"],
        &["controlled", "by", "same", "player"],
    ])?;
    Some(head.trim())
}

const EXILE_LOOKED_CARD_FACE_DOWN_PHRASES: &[&[&str]] = &[
    &["exile", "it", "face", "down"],
    &["exile", "that", "card", "face", "down"],
];
const PAIRS_THEN_WORD: &str = "then";
const PAIRS_FOR_EACH_PREFIX: &[&str] = &["for", "each"];
const PAIRS_COPY_OR_COPIES_WORDS: &[&str] = &["copy", "copies"];
const PAIRS_OF_WORD: &str = "of";
const TARGET_WORD: &str = "target";
const CHOOSE_WORD: &str = "choose";
const THAT_WORD: &str = "that";
const WITH_WORD: &str = "with";
const SPELL_OR_ABILITY_WORDS: &[&str] = &["spell", "ability"];
const PLAYER_OR_PLAYERS_WORDS: &[&str] = &["player", "players"];
const PERMANENT_OR_PERMANENTS_WORDS: &[&str] = &["permanent", "permanents"];
const CARD_OR_CARDS_WORDS: &[&str] = &["card", "cards"];
const THIS_SPELL_OR_ABILITY_PHRASES: &[&[&str]] = &[&["this", "spell"], &["this", "ability"]];
const SACRIFICE_ONE_OF_THOSE_CHOSEN_TARGETS_PHRASES: &[&[&str]] = &[
    &[
        "that",
        "player",
        "sacrifices",
        "one",
        "of",
        "them",
        "of",
        "their",
        "choice",
    ],
    &["that", "player", "sacrifices", "one", "of", "them"],
    &[
        "that",
        "player",
        "sacrifice",
        "one",
        "of",
        "them",
        "of",
        "their",
        "choice",
    ],
];
const CHOOSE_DRAW_MAIN_OR_COMBAT_PHASE_PHRASES: &[&[&str]] = &[
    &[
        "that", "player", "chooses", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
    &[
        "that", "player", "choose", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
    &[
        "the", "player", "chooses", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
    &[
        "the", "player", "choose", "draw", "step", "main", "phase", "or", "combat", "phase",
    ],
];
const SKIPS_CHOSEN_STEP_OR_PHASE_THIS_TURN_PHRASES: &[&[&str]] = &[
    &[
        "that", "player", "skips", "each", "instance", "of", "the", "chosen", "step", "or",
        "phase", "this", "turn",
    ],
    &[
        "that", "player", "skip", "each", "instance", "of", "the", "chosen", "step", "or", "phase",
        "this", "turn",
    ],
    &[
        "the", "player", "skips", "each", "instance", "of", "the", "chosen", "step", "or", "phase",
        "this", "turn",
    ],
    &[
        "the", "player", "skip", "each", "instance", "of", "the", "chosen", "step", "or", "phase",
        "this", "turn",
    ],
];
const YOU_MAY_CAST_TARGET_PREFIX: &[&str] = &["you", "may", "cast", "target"];
const FROM_YOUR_GRAVEYARD_PHRASE: &[&str] = &["from", "your", "graveyard"];
const WITHOUT_PAYING_ITS_MANA_COST_PHRASE: &[&str] = &["without", "paying", "its", "mana", "cost"];
const INSTANT_SORCERY_CARD_WORDS: &[&str] = &["instant", "sorcery", "card"];
const THAT_SPELL_GRAVEYARD_REPLACEMENT_WORDS: &[&str] = &[
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
const CAST_THIS_WAY_GRAVEYARD_REPLACEMENT_WORDS: &[&str] = &[
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
const ARTIFACT_WORD: &str = "artifact";
const COULD_TARGET_REFERENCE_PREFIXES: &[&[&str]] = &[
    &["that", "spell", "could", "target"],
    &["that", "ability", "could", "target"],
    &["that", "spell", "or", "ability", "could", "target"],
    &["the", "spell", "could", "target"],
    &["the", "ability", "could", "target"],
    &["it", "could", "target"],
];
const THEN_IF_PREFIX: &[&str] = &["then", "if"];
const IF_WORD: &str = "if";
const IF_YOU_DO_PREFIX: &[&str] = &["if", "you", "do"];
const PUT_ALL_REVEALED_PREFIXES: &[&[&str]] = &[
    &["put", "all"],
    &["puts", "all"],
    &["that", "player", "puts", "all"],
];
const REVEALED_THIS_WAY_PHRASE: &[&str] = &["revealed", "this", "way"];
const CREATURE_WORD: &str = "creature";
const PUT_COPY_PREFIX: &[&str] = &["put", "a", "copy"];
const ONTO_STACK_PHRASE: &[&str] = &["onto", "the", "stack"];
const EACH_COPY_TARGETS_DIFFERENT_ONE_PHRASE: &[&str] = &[
    "each",
    "copy",
    "targets",
    "a",
    "different",
    "one",
    "of",
    "those",
];
const FOR_EACH_OF_TAGGED_OBJECT_PHRASES: &[&[&str]] = &[
    &["for", "each", "of", "those"],
    &["for", "each", "of", "them"],
];
const FOR_EACH_PHRASE: &[&str] = &["for", "each"];
const CHOSEN_THIS_WAY_PHRASE: &[&str] = &["chosen", "this", "way"];
const COPY_TARGETS_ITERATED_OBJECT_PREFIXES: &[&[&str]] = &[
    &["the", "copy", "targets", "that"],
    &["the", "copy", "targets", "the", "chosen"],
];
const IF_THIS_PREFIX: &[&str] = &["if", "this"];
const ISNT_CREATURE_CONDITION_WORDS: &[&str] = &["isnt", "creature"];
const IT_WORD: &str = "it";
const LIFE_GAIN_WORDS: &[&str] = &["gain", "gains"];
const RETURN_TAGGED_CARDS_TO_BATTLEFIELD_PHRASES: &[&[&str]] = &[
    &["return", "those", "cards", "to", "battlefield"],
    &["return", "them", "to", "battlefield"],
];
const DELAYED_DIES_THIS_TURN_PREFIX: &[&str] =
    &["when", "that", "creature", "dies", "this", "turn"];
const EXILE_TOP_POWER_CARDS_PREFIX: &[&str] = &[
    "exile", "number", "of", "cards", "from", "top", "of", "your", "library", "equal", "to", "its",
    "power",
];
const CHOOSE_CARD_EXILED_THIS_WAY_SUFFIX: &[&str] = &["choose", "card", "exiled", "this", "way"];
const UNTIL_NEXT_TURN_PLAY_THAT_CARD_WORDS: &[&str] = &[
    "until", "end", "of", "your", "next", "turn", "you", "may", "play", "that", "card",
];
const CARDS_REVEALED_THIS_WAY_PHRASE: &[&str] = &["cards", "revealed", "this", "way"];
const ONTO_BATTLEFIELD_PHRASES: &[&[&str]] =
    &[&["onto", "the", "battlefield"], &["onto", "battlefield"]];
const PUT_REVEALED_MATCHES_ONTO_BATTLEFIELD_REQUIRED_WORDS: &[&str] =
    &["shuffle", "rest", "library"];
const PUT_MATCHED_CARD_INTO_HAND_PREFIXES: &[&[&str]] = &[
    &["put", "that", "card", "into", "your", "hand"],
    &["put", "it", "into", "your", "hand"],
];
const PUT_MATCHED_CARD_ONTO_BATTLEFIELD_PREFIXES: &[&[&str]] = &[
    &["put", "that", "card", "onto", "the", "battlefield"],
    &["put", "it", "onto", "the", "battlefield"],
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
];
const OTHER_REVEALED_CARD_PHRASES: &[&[&str]] = &[&["other", "cards"], &["all", "other"]];
const INTO_HAND_PREFIX: &[&str] = &["into"];
const CHOSEN_TYPE_REFERENCE_PHRASES: &[&[&str]] = &[&["chosen", "type"], &["that", "type"]];
const PUT_MATCHING_INTO_HAND_PHRASE: &[&str] = &["into", "your", "hand"];
const REST_INTO_YOUR_PHRASE: &[&str] = &["and", "the", "rest", "into", "your"];

fn pairs_words_match_value<T: Copy>(words: &[&str], choices: &[(&[&str], T)]) -> Option<T> {
    choices
        .iter()
        .find_map(|(phrase, value)| (*phrase == words).then_some(*value))
}

const PUT_ONE_LOOKED_CARD_INTO_HAND_PREFIXES: &[&[&str]] = &[
    &["put", "one", "of", "them", "into", "your", "hand"],
    &["put", "one", "of", "those", "cards", "into", "your", "hand"],
    &["put", "one", "into", "your", "hand"],
];
const PUT_OTHER_LOOKED_CARDS_ON_BOTTOM_PHRASES: &[&[&str]] = &[
    &["other", "on", "bottom"],
    &["other", "onto", "bottom"],
    &["rest", "on", "bottom"],
    &["rest", "onto", "bottom"],
];
const PUT_OTHER_LOOKED_CARDS_INTO_GRAVEYARD_PHRASES: &[&[&str]] = &[
    &["other", "into", "graveyard"],
    &["other", "into", "your", "graveyard"],
    &["rest", "into", "your", "graveyard"],
    &["rest", "into", "graveyard"],
];

const COUNTED_EXILE_LOOKED_FACE_DOWN_PREFIXES: &[&[&str]] = &[
    &["of", "them", "face", "down"],
    &["of", "those", "cards", "face", "down"],
    &["them", "face", "down"],
    &["those", "cards", "face", "down"],
];

fn words_contain_phrase(words: &[&str], phrase: &[&str]) -> bool {
    !phrase.is_empty() && words.windows(phrase.len()).any(|window| window == phrase)
}

fn parse_counted_looked_exile_face_down_rest_bottom(
    tokens: &[OwnedLexToken],
) -> Option<(ChoiceCount, LibraryBottomOrderAst)> {
    let clause = LexedClause::new(tokens).trimmed();
    if clause.word_refs().first().copied() != Some("exile") {
        return None;
    }

    let count_start = clause.token_index_for_word_index(1)?;
    let count_tokens = trim_commas(&clause.tokens()[count_start..]);
    let (count, used) = parse_choice_count_token_prefix_consumed(&count_tokens)?;
    let tail_tokens = trim_commas(&count_tokens[used..]);
    let tail_words = non_article_token_word_refs(&tail_tokens);
    if !word_slice_starts_with_any(&tail_words, COUNTED_EXILE_LOOKED_FACE_DOWN_PREFIXES)
        || !tail_words.iter().any(|word| *word == "library")
    {
        return None;
    }
    let bottoms_rest = words_contain_phrase(&tail_words, &["put", "rest", "on", "bottom"])
        || words_contain_phrase(&tail_words, &["put", "rest", "onto", "bottom"])
        || words_contain_phrase(&tail_words, &["put", "the", "rest", "on", "bottom"])
        || words_contain_phrase(&tail_words, &["put", "the", "rest", "onto", "bottom"]);
    if !bottoms_rest {
        return None;
    }

    let order = parse_consult_remainder_order(&LexedClause::new(&tail_tokens).word_refs())?;
    Some((count, order))
}

pub(crate) fn parse_look_at_top_then_exile_face_down_then_play_while_exiled(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_clause = LexedClause::new(sentences[sentence_idx].lowered()).trimmed();

    if let Some(exile_word_idx) = first_clause.find_word("exile") {
        let Some(exile_token_idx) = first_clause.token_index_for_word_index(exile_word_idx) else {
            return Ok(None);
        };
        let look_clause = first_clause.before(exile_token_idx).trimmed();
        let exile_clause = first_clause.from(exile_token_idx).trimmed();

        if let Some((exile_count, bottom_order)) =
            parse_counted_looked_exile_face_down_rest_bottom(exile_clause.tokens())
        {
            let Ok(look_effects) =
                effect_sentences::parse_effect_sentence_lexed(look_clause.tokens())
            else {
                return Ok(None);
            };
            let [look_effect] = look_effects.as_slice() else {
                return Ok(None);
            };
            let Some((library_owner, count)) = look_at_top_cards_parts(look_effect) else {
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
            let exiled_tag = helper_tag_for_tokens(exile_clause.tokens(), "exiled");
            let mut choice_filter = ObjectFilter::tagged(looked_tag.clone());
            choice_filter.zone = Some(Zone::Library);

            return Ok(Some(vec![
                EffectAst::subject_verb_look_at_top_cards(library_owner, count, looked_tag.clone()),
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
            ]));
        }
    }

    let Some(then_idx) = first_clause.find_phrase_start(&["then", "exile"]) else {
        return Ok(None);
    };
    let Some(then_token_idx) = first_clause.token_index_for_word_index(then_idx) else {
        return Ok(None);
    };
    let Some(exile_token_idx) = first_clause.token_index_for_word_index(then_idx + 1) else {
        return Ok(None);
    };

    let look_clause = first_clause.before(then_token_idx).trimmed();
    let exile_clause = first_clause.from(exile_token_idx).trimmed();
    let exile_words =
        crate::runtime_backend::util::non_article_token_word_refs(exile_clause.tokens());
    if !word_slice_eq_any(&exile_words, EXILE_LOOKED_CARD_FACE_DOWN_PHRASES) {
        return Ok(None);
    }

    let Ok(look_effects) = effect_sentences::parse_effect_sentence_lexed(look_clause.tokens())
    else {
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
    Ok(Some(vec![
        EffectAst::subject_verb_look_at_top_cards(player, count, looked_tag.clone()),
        EffectAst::subject_verb_exile(TargetAst::Tagged(looked_tag.clone(), None), true),
        EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            looked_tag,
            permission_player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            filter,
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
    let words = LexedClause::new(&second_tokens).word_refs();
    if !word_slice_starts_with_any(&words, PUT_ONE_LOOKED_CARD_INTO_HAND_PREFIXES) {
        return Ok(None);
    }
    let content_words = non_article_word_refs(&words);
    if !word_slice_contains_any_phrase(&content_words, PUT_OTHER_LOOKED_CARDS_ON_BOTTOM_PHRASES)
        || !word_slice_contains_word(&content_words, "library")
    {
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
    let words = LexedClause::new(&second_tokens).word_refs();
    if !word_slice_starts_with_any(&words, PUT_ONE_LOOKED_CARD_INTO_HAND_PREFIXES) {
        return Ok(None);
    }
    let content_words = non_article_word_refs(&words);
    if !word_slice_contains_any_phrase(
        &content_words,
        PUT_OTHER_LOOKED_CARDS_INTO_GRAVEYARD_PHRASES,
    ) {
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

pub(crate) fn parse_choose_draw_main_or_combat_phase_then_skip_chosen_this_turn(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_words = crate::runtime_backend::token_word_refs(sentences[sentence_idx].lowered());
    if !word_slice_eq_any(&first_words, CHOOSE_DRAW_MAIN_OR_COMBAT_PHASE_PHRASES) {
        return Ok(None);
    }

    let second_words =
        crate::runtime_backend::token_word_refs(sentences[sentence_idx + 1].lowered());
    if !word_slice_eq_any(&second_words, SKIPS_CHOSEN_STEP_OR_PHASE_THIS_TURN_PHRASES) {
        return Ok(None);
    }

    Ok(Some(vec![
        EffectAst::subject_verb_choose_named_option(
            PlayerAst::That,
            vec![
                "draw step".to_string(),
                "main phase".to_string(),
                "combat phase".to_string(),
            ],
        ),
        EffectAst::Conditional {
            predicate: PredicateAst::SourceChosenOption("draw step".to_string()),
            if_true: vec![EffectAst::subject_verb_skip_draw_step(PlayerAst::That)],
            if_false: vec![EffectAst::Conditional {
                predicate: PredicateAst::SourceChosenOption("main phase".to_string()),
                if_true: vec![EffectAst::subject_verb_skip_main_phases_this_turn(
                    PlayerAst::That,
                )],
                if_false: vec![EffectAst::subject_verb_skip_combat_phases_this_turn(
                    PlayerAst::That,
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
        .is_some_and(|token| token.as_word() == Some(CHOOSE_WORD))
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
    let second_words = non_article_token_word_refs(&second_tokens);
    if !word_slice_eq_any(&second_words, SACRIFICE_ONE_OF_THOSE_CHOSEN_TARGETS_PHRASES) {
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
    let words = LexedClause::new(tokens).word_refs();
    let words = if words.first().copied() == Some(PAIRS_THEN_WORD) {
        &words[1..]
    } else {
        words.as_slice()
    };
    pairs_words_match_value(
        words,
        &[
            (&["destroy", "the", "rest"], RestAction::Destroy),
            (&["destroy", "rest"], RestAction::Destroy),
            (&["exile", "the", "rest"], RestAction::Exile),
            (&["exile", "rest"], RestAction::Exile),
            (&["sacrifice", "the", "rest"], RestAction::Sacrifice),
            (&["sacrifice", "rest"], RestAction::Sacrifice),
            (&["sacrifices", "the", "rest"], RestAction::Sacrifice),
            (&["sacrifices", "rest"], RestAction::Sacrifice),
        ],
    )
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
    let first_words = LexedClause::new(&first).word_refs();
    let second_words = LexedClause::new(&second).word_refs();

    let without_paying_mana_cost =
        word_slice_contains_phrase(&first_words, WITHOUT_PAYING_ITS_MANA_COST_PHRASE);
    let first_is_targeted_graveyard_cast =
        word_slice_starts_with(&first_words, YOU_MAY_CAST_TARGET_PREFIX)
            && word_slice_contains_phrase(&first_words, FROM_YOUR_GRAVEYARD_PHRASE)
            && word_slice_contains_all_words(&first_words, INSTANT_SORCERY_CARD_WORDS);
    if !first_is_targeted_graveyard_cast {
        return Ok(None);
    }
    let second_is_that_spell_replacement =
        word_slice_eq(&second_words, THAT_SPELL_GRAVEYARD_REPLACEMENT_WORDS);
    let second_is_cast_this_way_replacement =
        word_slice_eq(&second_words, CAST_THIS_WAY_GRAVEYARD_REPLACEMENT_WORDS);
    if !second_is_that_spell_replacement && !second_is_cast_this_way_replacement {
        return Ok(None);
    }

    let tag = TagKey::from(crate::cards::builders::IT_TAG);
    let mut filter = ObjectFilter::default();
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.card_types = vec![CardType::Instant, CardType::Sorcery];
    if word_slice_contains_word(&first_words, ARTIFACT_WORD) {
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
    let words = LexedClause::new(sentences[sentence_idx - 1].lowered()).word_refs();
    words.iter().enumerate().any(|(idx, word)| {
        *word == TARGET_WORD
            && words[idx + 1..words.len().min(idx + 6)]
                .iter()
                .any(|tail| SPELL_OR_ABILITY_WORDS.contains(tail))
    })
}

fn target_for_referenced_stack_object(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    words: &[&str],
) -> TargetAst {
    if word_slice_eq_any(words, THIS_SPELL_OR_ABILITY_PHRASES) {
        return TargetAst::Source(None);
    }
    if previous_sentence_chose_stack_object(sentences, sentence_idx) {
        return TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None);
    }
    TargetAst::Tagged(TagKey::from("triggering"), None)
}

fn strip_could_target_suffix(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let words = LexedClause::new(tokens).word_refs();
    for word_idx in 0..words.len() {
        if !word_slice_starts_with_any(&words[word_idx..], COULD_TARGET_REFERENCE_PREFIXES) {
            continue;
        }
        let start_word_idx = if word_idx > 0 && words[word_idx - 1] == THAT_WORD {
            word_idx - 1
        } else {
            word_idx
        };
        if let Some(token_idx) = LexedClause::new(tokens).token_index_for_word_index(start_word_idx)
        {
            return trim_commas(&tokens[..token_idx]);
        }
    }
    trim_commas(tokens)
}

fn strip_leading_other(tokens: &[OwnedLexToken]) -> (Vec<OwnedLexToken>, bool) {
    let trimmed = trim_commas(tokens);
    let (stripped, was_stripped) =
        strip_leading_token_word_once_any(&trimmed, &["other", "another"]);
    (trim_commas(stripped), was_stripped)
}

fn parse_copy_for_each_candidate_filter(
    tokens: &[OwnedLexToken],
) -> Result<(Option<ObjectFilter>, Option<PlayerFilter>, bool), CardTextError> {
    let stripped = strip_could_target_suffix(tokens);
    let (candidate_tokens, exclude_current_targets) = strip_leading_other(&stripped);
    let candidate_words = LexedClause::new(&candidate_tokens).word_refs();
    let has_player = candidate_words
        .iter()
        .any(|word| PLAYER_OR_PLAYERS_WORDS.contains(word));
    let has_permanent = candidate_words
        .iter()
        .any(|word| PERMANENT_OR_PERMANENTS_WORDS.contains(word));

    if has_player && has_permanent {
        return Ok((
            Some(ObjectFilter::permanent()),
            Some(PlayerFilter::Any),
            exclude_current_targets,
        ));
    }
    if has_player && !candidate_words.iter().any(|word| *word == CREATURE_WORD) {
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
    let words = LexedClause::new(&tokens).word_refs();
    let wrap_if_result = word_slice_starts_with(&words, IF_YOU_DO_PREFIX);
    let Some(for_each_word_idx) = word_slice_find_phrase_start(&words, PAIRS_FOR_EACH_PREFIX)
    else {
        return Ok(None);
    };
    let Some(copy_word_idx) = word_slice_find_any_word(&words, PAIRS_COPY_OR_COPIES_WORDS) else {
        return Ok(None);
    };
    if copy_word_idx < for_each_word_idx {
        let Some(copy_token_idx) =
            LexedClause::new(&tokens).token_index_for_word_index(copy_word_idx)
        else {
            return Ok(None);
        };
        let Some(for_each_token_idx) =
            LexedClause::new(&tokens).token_index_for_word_index(for_each_word_idx)
        else {
            return Ok(None);
        };
        let subject = parse_subject(&tokens[..copy_token_idx]);
        let player = match subject {
            SubjectAst::Player(player) => player,
            SubjectAst::This => PlayerAst::Implicit,
        };
        let target_tokens = trim_commas(&tokens[copy_token_idx + 1..for_each_token_idx]);
        let target_words = LexedClause::new(&target_tokens).word_refs();
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

    let Some(for_each_token_idx) =
        LexedClause::new(&tokens).token_index_for_word_index(for_each_word_idx)
    else {
        return Ok(None);
    };
    let Some(put_copy_word_idx) = word_slice_find_phrase_start(&words, PUT_COPY_PREFIX) else {
        return Ok(None);
    };
    let Some(put_copy_token_idx) =
        LexedClause::new(&tokens).token_index_for_word_index(put_copy_word_idx)
    else {
        return Ok(None);
    };
    let candidate_tokens = trim_commas(&tokens[for_each_token_idx + 2..put_copy_token_idx]);
    let after_copy_words = &words[put_copy_word_idx + 3..];
    let of_offset = word_slice_find_word(after_copy_words, PAIRS_OF_WORD);
    let target_start_word_idx = of_offset
        .map(|offset| put_copy_word_idx + 3 + offset + 1)
        .unwrap_or(put_copy_word_idx + 3);
    let onto_rel = (0..words[target_start_word_idx..].len())
        .find(|idx| {
            word_slice_contains_phrase(&words[target_start_word_idx + *idx..], ONTO_STACK_PHRASE)
        })
        .unwrap_or(words.len().saturating_sub(target_start_word_idx));
    let target_end_word_idx = target_start_word_idx + onto_rel;
    let Some(target_start_token_idx) =
        LexedClause::new(&tokens).token_index_for_word_index(target_start_word_idx)
    else {
        return Ok(None);
    };
    let target_end_token_idx = LexedClause::new(&tokens)
        .token_index_for_word_index(target_end_word_idx)
        .unwrap_or_else(|| tokens.len());
    let target_tokens = trim_commas(&tokens[target_start_token_idx..target_end_token_idx]);
    let target_words = LexedClause::new(&target_tokens).word_refs();
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
    let words = LexedClause::new(tokens).word_refs();
    word_slice_contains_phrase(&words, EACH_COPY_TARGETS_DIFFERENT_ONE_PHRASE)
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
    let words = LexedClause::new(tokens).word_refs();
    (word_slice_contains_any_phrase(&words, FOR_EACH_OF_TAGGED_OBJECT_PHRASES)
        || (word_slice_contains_phrase(&words, FOR_EACH_PHRASE)
            && word_slice_contains_phrase(&words, CHOSEN_THIS_WAY_PHRASE)))
        && word_slice_contains_any_word(&words, PAIRS_COPY_OR_COPIES_WORDS)
}

fn second_sentence_copy_targets_iterated_object(tokens: &[OwnedLexToken]) -> bool {
    let words = LexedClause::new(tokens).word_refs();
    word_slice_starts_with_any(&words, COPY_TARGETS_ITERATED_OBJECT_PREFIXES)
}

pub(crate) fn parse_for_each_tagged_copy_then_copy_targets_it(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(sentences[sentence_idx].lowered());
    let first_words = LexedClause::new(&first_tokens).word_refs();
    if !first_sentence_copies_for_each_tagged_object(&first_tokens)
        || !second_sentence_copy_targets_iterated_object(sentences[sentence_idx + 1].lowered())
    {
        return Ok(None);
    }

    let wrap_if_result = word_slice_starts_with(&first_words, IF_YOU_DO_PREFIX);
    let Some(copy_word_idx) = word_slice_find_any_word(&first_words, PAIRS_COPY_OR_COPIES_WORDS)
    else {
        return Ok(None);
    };
    let Some(copy_token_idx) =
        LexedClause::new(&first_tokens).token_index_for_word_index(copy_word_idx)
    else {
        return Ok(None);
    };
    let copy_target_tokens = trim_commas(&first_tokens[copy_token_idx + 1..]);
    let copy_target_words = LexedClause::new(&copy_target_tokens).word_refs();
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
            && CARD_OR_CARDS_WORDS.contains(&words[idx + 1])
            && words[idx + 2] == WITH_WORD
        {
            card_choice_segments += 1;
            if card_choice_segments >= 2 {
                return true;
            }
        }
    }
    false
}

pub(crate) fn parse_may_put_filtered_card_from_among_into_hand(
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
    if !word_slice_starts_with(after_from_words, INTO_HAND_PREFIX)
        || !word_slice_contains_word(after_from_words, "hand")
    {
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
    if !word_slice_starts_with(&words, IF_THIS_PREFIX) {
        return Ok(None);
    }
    let Some(comma_idx) = find_index(tokens, |token: &OwnedLexToken| token.is_comma()) else {
        return Ok(None);
    };
    let condition_words = TokenWordView::new(&tokens[..comma_idx]).word_refs();
    if !word_slice_contains_all_words(&condition_words, ISNT_CREATURE_CONDITION_WORDS) {
        return Ok(None);
    }

    let tail = trim_commas(&tokens[comma_idx + 1..]);
    if !TokenWordView::new(&tail)
        .word_refs()
        .first()
        .is_some_and(|word| *word == IT_WORD)
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

    let first_words = LexedClause::new(first).word_refs();
    if !word_slice_contains_word(&first_words, "life")
        || !word_slice_contains_any_word(&first_words, LIFE_GAIN_WORDS)
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

    let first_words = LexedClause::new(first).word_refs();
    if !word_slice_contains_word(&first_words, "life")
        || !word_slice_contains_any_word(&first_words, LIFE_GAIN_WORDS)
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
        effect_sentences::SubjectVerbPrimitiveClause::new(sentences[sentence_idx].lowered()),
    )?
    else {
        return Ok(None);
    };

    let second_words = non_article_token_word_refs(sentences[sentence_idx + 1].lowered());
    let tapped = second_words.iter().any(|word| *word == TAPPED_WORD);
    let second_without_tapped = word_refs_except(&second_words, &["tapped"]);
    if !word_slice_eq_any(
        &second_without_tapped,
        RETURN_TAGGED_CARDS_TO_BATTLEFIELD_PHRASES,
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
    let first_words = LexedClause::new(&first_tokens).word_refs();
    if !word_slice_starts_with(&first_words, DELAYED_DIES_THIS_TURN_PREFIX) {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(&first_tokens, |token: &OwnedLexToken| token.is_comma())
    else {
        return Ok(None);
    };
    let action_tokens = trim_commas(&first_tokens[comma_idx + 1..]);
    let action_words = non_article_token_word_refs(&action_tokens);
    let starts_with_exile_top_power =
        word_slice_starts_with(&action_words, EXILE_TOP_POWER_CARDS_PREFIX);
    let ends_with_choose_exiled = action_words.ends_with(CHOOSE_CARD_EXILED_THIS_WAY_SUFFIX);
    if !starts_with_exile_top_power || !ends_with_choose_exiled {
        return Ok(None);
    }

    let second_words = non_article_token_word_refs(sentences[sentence_idx + 1].lowered());
    let is_until_next_turn_play_clause =
        word_slice_eq(&second_words, UNTIL_NEXT_TURN_PLAY_THAT_CARD_WORDS);
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

    parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen(
        sentences,
        sentence_idx,
        *player,
        chooser,
        filter,
        Vec::new(),
    )
}

/// Shared body for the mill-then-choose follow-up, parameterized by the
/// optional "if you don't" branch so both the bare and the if-you-don't
/// callers compose the same reusable primitive sequence (mirroring the retired
/// `ChooseFromLookedCardsIntoHandRestIntoGraveyard` recipe). The milled cards
/// already sit in the graveyard, so the choose filter references them via
/// `IT_TAG` (resolved to the mill's collection tag at lowering) and no
/// rest-into-graveyard split is emitted.
pub(crate) fn parse_mill_then_may_put_from_among_into_hand_with_if_not_chosen(
    sentences: &[SentenceInput],
    sentence_idx: usize,
    player: PlayerAst,
    chooser: PlayerAst,
    filter: ObjectFilter,
    if_not_chosen: Vec<EffectAst>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first = sentences[sentence_idx].lowered();
    let Ok(first_effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let [
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mill { .. },
            ..
        }),
    ] = first_effects.as_slice()
    else {
        return Ok(None);
    };
    let _ = player;

    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen");
    let mut effects = vec![first_effects[0].clone()];
    effects.extend(
        super::triples::compose_choose_from_looked_cards_into_hand_rest_into_graveyard(
            chooser,
            filter,
            TagKey::from(crate::cards::builders::IT_TAG),
            chosen_tag,
            Zone::Graveyard,
            false,
            if_not_chosen,
        ),
    );
    Ok(Some(effects))
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
    let filter_words = LexedClause::new(&filter_tokens).word_refs();
    if word_slice_contains_any_phrase(&filter_words, CHOSEN_TYPE_REFERENCE_PHRASES) {
        filter.chosen_creature_type = true;
    }
    effect_sentences::normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_revealed = &second_word_refs[revealed_idx + 3..];
    let has_hand_clause = word_slice_contains_phrase(after_revealed, PUT_MATCHING_INTO_HAND_PHRASE);
    let has_graveyard_rest_clause =
        word_slice_contains_phrase(after_revealed, REST_INTO_YOUR_PHRASE)
            && word_slice_contains_word(after_revealed, "graveyard");
    let bottom_order = parse_consult_remainder_order(after_revealed);
    if !has_hand_clause || (!has_graveyard_rest_clause && bottom_order.is_none()) {
        return Ok(None);
    }

    let effects = if let Some(order) = bottom_order {
        compose_reveal_top_put_matching_into_hand_rest_on_bottom(
            sentences[sentence_idx].lowered(),
            &second_tokens,
            count,
            filter,
            order,
        )
    } else {
        compose_reveal_top_put_matching_into_hand_rest_into_graveyard(
            sentences[sentence_idx].lowered(),
            count,
            filter,
        )
    };

    Ok(Some(effects))
}

/// Composes the "reveal top N, put all matching into hand, rest on bottom" shape
/// from reusable primitives (look + reveal-tagged + tag-matching + move-group +
/// remainder-to-bottom), matching the runtime effects the retired
/// `RevealTopPutMatchingIntoHandRestOnBottomOfLibrary` recipe lowered to.
fn compose_reveal_top_put_matching_into_hand_rest_on_bottom(
    look_tokens: &[OwnedLexToken],
    matched_tokens: &[OwnedLexToken],
    count: u32,
    mut filter: ObjectFilter,
    order: LibraryBottomOrderAst,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "revealed");
    let matched_tag = helper_tag_for_tokens(matched_tokens, "matched");
    filter.zone = None;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    vec![
        EffectAst::subject_verb_look_at_top_cards(
            PlayerAst::You,
            Value::Fixed(count as i32),
            looked_tag.clone(),
        ),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
        EffectAst::subject_verb_tag_matching_objects(
            filter,
            vec![Zone::Library],
            matched_tag.clone(),
        ),
        EffectAst::ForEachTagged {
            tag: matched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Hand,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            )],
        },
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            looked_tag,
            Some(matched_tag),
            order,
            PlayerAst::You,
        ),
    ]
}

/// Composes the "reveal top N, put matching into hand, rest into graveyard" shape:
/// look + reveal-tagged + per-looked-card conditional split (matches filter -> hand,
/// else -> graveyard), matching the retired
/// `RevealTopPutMatchingIntoHandRestIntoGraveyard` recipe's lowering.
fn compose_reveal_top_put_matching_into_hand_rest_into_graveyard(
    look_tokens: &[OwnedLexToken],
    count: u32,
    mut filter: ObjectFilter,
) -> Vec<EffectAst> {
    let looked_tag = helper_tag_for_tokens(look_tokens, "revealed");
    filter.zone = None;
    let iterated = || TargetAst::Tagged(TagKey::from(IT_TAG), None);
    vec![
        EffectAst::subject_verb_look_at_top_cards(
            PlayerAst::You,
            Value::Fixed(count as i32),
            looked_tag.clone(),
        ),
        EffectAst::subject_verb_reveal_tagged(looked_tag.clone()),
        EffectAst::ForEachTagged {
            tag: looked_tag,
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter),
                if_true: vec![EffectAst::subject_verb_move_to_zone(
                    iterated(),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    iterated(),
                    Zone::Graveyard,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        },
    ]
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
    let second_words = LexedClause::new(&second_tokens).word_refs();
    let puts_all_revealed_matching_onto_battlefield =
        word_slice_starts_with(&second_words, &["put", "all"])
            && word_slice_contains_phrase(&second_words, CARDS_REVEALED_THIS_WAY_PHRASE)
            && word_slice_contains_any_phrase(&second_words, ONTO_BATTLEFIELD_PHRASES)
            && word_slice_contains_all_words(
                &second_words,
                PUT_REVEALED_MATCHES_ONTO_BATTLEFIELD_REQUIRED_WORDS,
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
    let first_words = LexedClause::new(&first_tokens).word_refs();
    let conditional_tokens = if word_slice_starts_with(&first_words, THEN_IF_PREFIX) {
        &first_tokens[1..]
    } else if first_tokens
        .first()
        .is_some_and(|token| token.as_word() == Some(IF_WORD))
    {
        first_tokens.as_slice()
    } else {
        return Ok(None);
    };

    let Some(comma_idx) = find_token_kind(conditional_tokens, TokenKind::Comma) else {
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
    let second_words = LexedClause::new(&second_tokens).word_refs();
    if !word_slice_starts_with_any(&second_words, PUT_ALL_REVEALED_PREFIXES)
        || !word_slice_contains_phrase(&second_words, REVEALED_THIS_WAY_PHRASE)
        || !word_slice_contains_word(&second_words, "graveyard")
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

pub(crate) fn parse_consult_match_into_battlefield_or_hand(
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
                mode: crate::cards::builders::LibraryConsultModeAst::Exile,
                ..
            },
            ..
        }))
    ) {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let moves_to_battlefield_or_hand =
        crate::runtime_backend::grammar::primitives::words_match_prefix(
            &second_tokens,
            &[
                "put",
                "that",
                "card",
                "onto",
                "the",
                "battlefield",
                "or",
                "into",
                "your",
                "hand",
            ],
        )
        .is_some()
            || crate::runtime_backend::grammar::primitives::words_match_prefix(
                &second_tokens,
                &[
                    "put",
                    "it",
                    "onto",
                    "the",
                    "battlefield",
                    "or",
                    "into",
                    "your",
                    "hand",
                ],
            )
            .is_some();
    if !moves_to_battlefield_or_hand {
        return Ok(None);
    }

    let mut effects = parts.effects;
    effects.push(EffectAst::May {
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag.clone(), None),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        )],
    });
    effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::DidNot,
        effects: vec![EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(parts.match_tag, None),
            Zone::Hand,
            false,
            ReturnControllerAst::You,
            false,
            None,
        )],
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
    let second_words = LexedClause::new(&second_tokens).word_refs();
    let moves_to_hand =
        word_slice_starts_with_any(&second_words, PUT_MATCHED_CARD_INTO_HAND_PREFIXES);
    let others_to_graveyard =
        word_slice_contains_any_phrase(&second_words, OTHER_REVEALED_CARD_PHRASES)
            && word_slice_contains_word(&second_words, "graveyard");
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
    let second_words = LexedClause::new(&second_tokens).word_refs();
    let moves_to_battlefield =
        word_slice_starts_with_any(&second_words, PUT_MATCHED_CARD_ONTO_BATTLEFIELD_PREFIXES);
    let others_to_graveyard =
        word_slice_contains_any_phrase(&second_words, OTHER_REVEALED_CARD_PHRASES)
            && word_slice_contains_word(&second_words, "graveyard");
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

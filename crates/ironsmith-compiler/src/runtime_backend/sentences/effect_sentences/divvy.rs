use super::super::grammar::primitives::TokenWordView;
use super::super::lexer::{OwnedLexToken, split_lexed_sentences, word_slice_starts_with};
use super::dispatch_entry::{SentenceInput, parse_prefixed_top_of_your_library_count};
use super::dispatch_inner::parse_effect_sentence_lexed;
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

fn membership_predicate_for_iterated_object(tag: &str) -> PredicateAst {
    PredicateAst::TaggedMatches(
        TagKey::from(tag),
        ObjectFilter::default().same_stable_id_as_tagged(TagKey::from(IT_TAG)),
    )
}

fn parse_effect_sentence_sequence(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let effects = parse_effect_sentence_lexed(tokens)?;
    if effects.is_empty() {
        Err(CardTextError::ParseError(
            "missing effect sentence".to_string(),
        ))
    } else {
        Ok(effects)
    }
}

fn normalized_divvy_match_word(word: &str) -> String {
    word.chars().filter(|ch| *ch != '\'').collect()
}

fn matches_sentence(words: &TokenWordView<'_>, expected: &[&str]) -> bool {
    words.len() == expected.len()
        && expected.iter().enumerate().all(|(idx, expected)| {
            words.get(idx).is_some_and(|actual| {
                normalized_divvy_match_word(actual) == normalized_divvy_match_word(expected)
            })
        })
}

fn matches_sentence_sequence(sentence_words: &[TokenWordView<'_>], expected: &[&[&str]]) -> bool {
    sentence_words.len() == expected.len()
        && sentence_words
            .iter()
            .zip(expected.iter().copied())
            .all(|(words, expected)| matches_sentence(words, expected))
}

fn first_sentence_has_prefix(sentence_words: &[TokenWordView<'_>], prefix: &[&str]) -> bool {
    sentence_words
        .first()
        .is_some_and(|words| word_slice_starts_with(&words.word_refs(), prefix))
}

fn sentence_has_phrase(sentence_words: &[TokenWordView<'_>], phrase: &[&str]) -> bool {
    sentence_words.iter().any(|words| words.has_phrase(phrase))
}

pub(super) fn try_parse_divvy_sentence_sequence(
    sentences: &[SentenceInput],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_words = sentences
        .iter()
        .map(|sentence| TokenWordView::new(sentence.lowered()))
        .collect::<Vec<_>>();

    if sentences.len() == 1 {
        let words = TokenWordView::new(sentences[0].lowered());
        let word_refs = words.word_refs();
        if words.has_phrase(&["chooses", "two", "of", "those", "cards"])
            && words.has_phrase(&["shuffle", "the", "chosen", "cards"])
            && words.has_phrase(&["put", "the", "rest", "onto", "the", "battlefield"])
            && word_slice_starts_with(
                &word_refs,
                &[
                    "search",
                    "your",
                    "library",
                    "and",
                    "graveyard",
                    "for",
                    "up",
                    "to",
                    "four",
                    "creature",
                    "cards",
                ],
            )
        {
            let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
                .into_iter()
                .next()
                .unwrap_or_else(|| sentences[0].lowered());
            let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
            effects.extend(vec![
                EffectAst::subject_verb_tag_matching_objects(
                    ObjectFilter::tagged(TagKey::from(IT_TAG)),
                    vec![Zone::Library, Zone::Graveyard],
                    TagKey::from("divvy_source"),
                ),
                EffectAst::ChooseObjectsAcrossZones {
                    filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                    count: ChoiceCount::exactly(2),
                    count_value: None,
                    player: PlayerAst::Opponent,
                    tag: TagKey::from("divvy_chosen"),
                    zones: vec![Zone::Library, Zone::Graveyard],
                    search_mode: None,
                },
                EffectAst::ForEachTagged {
                    tag: TagKey::from("divvy_source"),
                    effects: vec![EffectAst::Conditional {
                        predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                        if_true: Vec::new(),
                        if_false: vec![EffectAst::subject_verb_move_to_zone(
                            TargetAst::Tagged(TagKey::from(IT_TAG), None),
                            Zone::Battlefield,
                            false,
                            ReturnControllerAst::Preserve,
                            false,
                            None,
                        )],
                    }],
                },
                EffectAst::ForEachTagged {
                    tag: TagKey::from("divvy_chosen"),
                    effects: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                },
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::You,
                    SubjectVerbActionAst::ShuffleLibrary,
                ),
                EffectAst::subject_verb_exile(TargetAst::Source(None), false),
            ]);
            return Ok(Some(effects));
        }
    }

    if matches_sentence_sequence(
        &sentence_words,
        &[
            &[
                "choose",
                "any",
                "number",
                "of",
                "creatures",
                "target",
                "player",
                "controls",
            ],
            &[
                "choose",
                "the",
                "same",
                "number",
                "of",
                "creatures",
                "another",
                "target",
                "player",
                "controls",
            ],
            &[
                "those",
                "players",
                "exchange",
                "control",
                "of",
                "those",
                "creatures",
            ],
        ],
    ) {
        let first_player_tag = TagKey::from("exchange_player_one");
        let second_player_tag = TagKey::from("exchange_player_two");
        let first_creatures_tag = TagKey::from("exchange_creatures_one");
        let second_creatures_tag = TagKey::from("exchange_creatures_two");

        return Ok(Some(vec![
            EffectAst::subject_verb_target_only(TargetAst::WithCount(
                Box::new(TargetAst::Player(PlayerFilter::Any, None)),
                ChoiceCount::exactly(2),
            )),
            EffectAst::subject_verb_choose_player(
                PlayerAst::You,
                PlayerFilter::target_player(),
                first_player_tag.clone(),
                false,
                0,
            ),
            EffectAst::subject_verb_choose_player(
                PlayerAst::You,
                PlayerFilter::target_player(),
                second_player_tag.clone(),
                false,
                1,
            ),
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature()
                    .controlled_by(PlayerFilter::TaggedPlayer(first_player_tag.clone())),
                count: ChoiceCount::up_to_dynamic_x(),
                count_value: Some(Value::Count(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::TaggedPlayer(second_player_tag.clone())),
                )),
                player: PlayerAst::You,
                tag: first_creatures_tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature()
                    .controlled_by(PlayerFilter::TaggedPlayer(second_player_tag.clone())),
                count: ChoiceCount::dynamic_x(),
                count_value: Some(Value::Count(ObjectFilter::tagged(
                    first_creatures_tag.clone(),
                ))),
                player: PlayerAst::You,
                tag: second_creatures_tag.clone(),
            },
            EffectAst::ForEachTaggedPlayer {
                tag: second_player_tag,
                effects: vec![EffectAst::subject_verb_gain_control(
                    PlayerAst::That,
                    TargetAst::Tagged(first_creatures_tag, None),
                    Until::Forever,
                )],
            },
            EffectAst::ForEachTaggedPlayer {
                tag: first_player_tag,
                effects: vec![EffectAst::subject_verb_gain_control(
                    PlayerAst::That,
                    TargetAst::Tagged(second_creatures_tag, None),
                    Until::Forever,
                )],
            },
        ]));
    }

    if matches_sentence_sequence(
        &sentence_words,
        &[
            &[
                "separate",
                "all",
                "creatures",
                "target",
                "player",
                "controls",
                "into",
                "two",
                "piles",
            ],
            &[
                "destroy",
                "all",
                "creatures",
                "in",
                "the",
                "pile",
                "of",
                "that",
                "player's",
                "choice",
            ],
            &["they", "can't", "be", "regenerated"],
        ],
    ) {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Target,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_destroy_no_regeneration(TargetAst::Tagged(
                TagKey::from("divvy_chosen"),
                None,
            )),
        ]));
    }

    if matches_sentence_sequence(
        &sentence_words,
        &[
            &[
                "separate",
                "all",
                "creature",
                "cards",
                "in",
                "your",
                "graveyard",
                "into",
                "two",
                "piles",
            ],
            &[
                "exile",
                "the",
                "pile",
                "of",
                "an",
                "opponent's",
                "choice",
                "and",
                "return",
                "the",
                "other",
                "to",
                "the",
                "battlefield",
            ],
        ],
    ) {
        let mut graveyard_creatures = ObjectFilter::creature();
        graveyard_creatures.zone = Some(Zone::Graveyard);
        graveyard_creatures.owner = Some(PlayerFilter::You);
        let rest_filter = graveyard_creatures
            .clone()
            .not_tagged(TagKey::from("divvy_chosen"));
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: graveyard_creatures,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_exile(
                TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                false,
            ),
            EffectAst::subject_verb_return_all_to_battlefield(
                rest_filter,
                false,
                ReturnControllerAst::You,
            ),
        ]));
    }

    if first_sentence_has_prefix(
        &sentence_words,
        &[
            "each",
            "opponent",
            "separates",
            "the",
            "creatures",
            "they",
            "control",
            "into",
            "two",
            "piles",
        ],
    ) && sentence_has_phrase(&sentence_words, &["for", "each", "opponent"])
        && sentence_has_phrase(
            &sentence_words,
            &[
                "each",
                "opponent",
                "sacrifices",
                "the",
                "creatures",
                "in",
                "their",
                "chosen",
                "pile",
            ],
        )
    {
        let chosen_pile_filter = ObjectFilter::creature()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .match_tagged(
                TagKey::from("divvy_pile"),
                TaggedOpbjectRelation::IsTaggedObject,
            );
        let other_pile_filter = ObjectFilter::creature()
            .controlled_by(PlayerFilter::IteratedPlayer)
            .not_tagged(TagKey::from("divvy_pile"));

        return Ok(Some(vec![
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Opponent,
                effects: vec![EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: TagKey::from("divvy_pile"),
                }],
            },
            EffectAst::ForEachPlayersFiltered {
                filter: PlayerFilter::Opponent,
                effects: vec![EffectAst::UnlessAction {
                    player: PlayerAst::You,
                    effects: vec![EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::Implicit,
                        chosen_pile_filter,
                    )],
                    alternative: vec![EffectAst::subject_verb_sacrifice_all(
                        PlayerAst::Implicit,
                        other_pile_filter,
                    )],
                }],
            },
        ]));
    }

    if first_sentence_has_prefix(
        &sentence_words,
        &[
            "separate",
            "all",
            "permanents",
            "target",
            "player",
            "controls",
            "into",
            "two",
            "piles",
        ],
    ) && sentence_has_phrase(
        &sentence_words,
        &[
            "that",
            "player",
            "sacrifices",
            "all",
            "permanents",
            "in",
            "the",
            "pile",
            "of",
            "their",
            "choice",
        ],
    ) {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::permanent().controlled_by(PlayerFilter::target_player()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Target,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_sacrifice_all(
                PlayerAst::Target,
                ObjectFilter::tagged(TagKey::from("divvy_chosen")),
            ),
        ]));
    }

    if matches_sentence_sequence(
        &sentence_words,
        &[
            &[
                "for",
                "each",
                "defending",
                "player",
                "separate",
                "all",
                "creatures",
                "that",
                "player",
                "controls",
                "into",
                "two",
                "piles",
                "and",
                "that",
                "player",
                "chooses",
                "one",
            ],
            &[
                "only",
                "creatures",
                "in",
                "the",
                "chosen",
                "piles",
                "can",
                "block",
                "this",
                "turn",
            ],
        ],
    ) {
        return Ok(Some(vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Defending,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::subject_verb_cant(
                    crate::effect::Restriction::block(
                        ObjectFilter::creature()
                            .controlled_by(PlayerFilter::IteratedPlayer)
                            .not_tagged(TagKey::from("divvy_chosen")),
                    ),
                    Until::EndOfTurn,
                    None,
                ),
            ],
        }]));
    }

    if first_sentence_has_prefix(
        &sentence_words,
        &[
            "separate",
            "all",
            "creatures",
            "that",
            "player",
            "controls",
            "into",
            "two",
            "piles",
        ],
    ) && sentence_has_phrase(
        &sentence_words,
        &[
            "only",
            "creatures",
            "in",
            "the",
            "pile",
            "of",
            "their",
            "choice",
            "can",
            "attack",
            "this",
            "turn",
        ],
    ) {
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("divvy_chosen"),
            },
            EffectAst::subject_verb_cant(
                crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                Until::EndOfTurn,
                None,
            ),
        ]));
    }

    if matches_sentence_sequence(
        &sentence_words,
        &[
            &[
                "each",
                "player",
                "separates",
                "all",
                "nontoken",
                "lands",
                "they",
                "control",
                "into",
                "two",
                "piles",
            ],
            &[
                "for",
                "each",
                "player",
                "one",
                "of",
                "their",
                "piles",
                "is",
                "chosen",
                "by",
                "one",
                "of",
                "their",
                "opponents",
                "of",
                "their",
                "choice",
            ],
            &["destroy", "all", "lands", "in", "the", "chosen", "piles"],
            &["tap", "all", "lands", "in", "the", "other", "piles"],
        ],
    ) {
        return Ok(Some(vec![EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::subject_verb_choose_player(
                    PlayerAst::Implicit,
                    PlayerFilter::Opponent,
                    TagKey::from("divvy_opponent"),
                    false,
                    0,
                ),
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::That,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::subject_verb_destroy(TargetAst::Tagged(
                    TagKey::from("divvy_chosen"),
                    None,
                )),
                EffectAst::subject_verb_tap_all(
                    ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
            ],
        }]));
    }

    if first_sentence_has_prefix(
        &sentence_words,
        &[
            "exile",
            "up",
            "to",
            "five",
            "target",
            "permanent",
            "cards",
            "from",
            "your",
            "graveyard",
            "and",
            "separate",
            "them",
            "into",
            "two",
            "piles",
        ],
    ) && sentence_has_phrase(
        &sentence_words,
        &["an", "opponent", "chooses", "one", "of", "those", "piles"],
    ) && sentence_has_phrase(
        &sentence_words,
        &["put", "that", "pile", "into", "your", "hand"],
    ) && sentence_has_phrase(
        &sentence_words,
        &["the", "other", "into", "your", "graveyard"],
    ) {
        let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                vec![Zone::Exile],
                TagKey::from("divvy_source"),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: TagKey::from("divvy_pile"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::UnlessAction {
                player: PlayerAst::Opponent,
                effects: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from("divvy_pile"), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: TagKey::from("divvy_source"),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object("divvy_pile"),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Hand,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
                alternative: vec![
                    EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(TagKey::from("divvy_pile"), None),
                        false,
                    ),
                    EffectAst::ForEachTagged {
                        tag: TagKey::from("divvy_source"),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object("divvy_pile"),
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
                ],
            },
        ]);
        return Ok(Some(effects));
    }

    if matches_sentence_sequence(
        &sentence_words,
        &[
            &[
                "exile",
                "up",
                "to",
                "five",
                "target",
                "creature",
                "cards",
                "from",
                "graveyards",
            ],
            &[
                "an",
                "opponent",
                "separates",
                "those",
                "cards",
                "into",
                "two",
                "piles",
            ],
            &[
                "put",
                "all",
                "cards",
                "from",
                "the",
                "pile",
                "of",
                "your",
                "choice",
                "onto",
                "the",
                "battlefield",
                "under",
                "your",
                "control",
                "and",
                "the",
                "rest",
                "into",
                "their",
                "owners'",
                "graveyards",
            ],
        ],
    ) {
        let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                vec![Zone::Exile],
                TagKey::from("divvy_source"),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                Zone::Battlefield,
                false,
                ReturnControllerAst::You,
                false,
                None,
            ),
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
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
        ]);
        return Ok(Some(effects));
    }

    if sentences.len() == 3
        && let Some((_, count)) = parse_prefixed_top_of_your_library_count(
            sentences[0].lowered(),
            &[
                (&["reveal", "the", "top"][..], ()),
                (&["reveal", "top"][..], ()),
            ],
        )
        && matches_sentence(
            &sentence_words[1],
            &[
                "an",
                "opponent",
                "separates",
                "those",
                "cards",
                "into",
                "two",
                "piles",
            ],
        )
        && matches_sentence(
            &sentence_words[2],
            &[
                "put",
                "one",
                "pile",
                "into",
                "your",
                "hand",
                "and",
                "the",
                "other",
                "into",
                "your",
                "graveyard",
            ],
        )
    {
        let source_tag = TagKey::from("divvy_source");
        let pile_tag = TagKey::from("divvy_pile");
        return Ok(Some(vec![
            EffectAst::subject_verb_reveal_top_cards(
                PlayerAst::You,
                Value::Fixed(count as i32),
                source_tag.clone(),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(source_tag.clone()),
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: pile_tag.clone(),
                zones: vec![Zone::Library],
                search_mode: None,
            },
            EffectAst::UnlessAction {
                player: PlayerAst::You,
                effects: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(pile_tag.clone(), None),
                        Zone::Graveyard,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: source_tag.clone(),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(pile_tag.as_str()),
                            if_true: Vec::new(),
                            if_false: vec![EffectAst::subject_verb_move_to_zone(
                                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                                Zone::Hand,
                                false,
                                ReturnControllerAst::Preserve,
                                false,
                                None,
                            )],
                        }],
                    },
                ],
                alternative: vec![
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(pile_tag.clone(), None),
                        Zone::Hand,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::ForEachTagged {
                        tag: source_tag.clone(),
                        effects: vec![EffectAst::Conditional {
                            predicate: membership_predicate_for_iterated_object(pile_tag.as_str()),
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
                ],
            },
        ]));
    }

    if first_sentence_has_prefix(
        &sentence_words,
        &[
            "search",
            "your",
            "library",
            "and",
            "graveyard",
            "for",
            "up",
            "to",
            "four",
            "creature",
            "cards",
        ],
    ) && sentence_has_phrase(&sentence_words, &["different", "names"])
        && sentence_has_phrase(&sentence_words, &["mana", "value", "x", "or", "less"])
        && sentence_has_phrase(&sentence_words, &["reveal", "them"])
        && sentence_has_phrase(
            &sentence_words,
            &["an", "opponent", "chooses", "two", "of", "those", "cards"],
        )
        && sentence_has_phrase(
            &sentence_words,
            &[
                "shuffle", "the", "chosen", "cards", "into", "your", "library",
            ],
        )
        && sentence_has_phrase(
            &sentence_words,
            &["put", "the", "rest", "onto", "the", "battlefield"],
        )
    {
        let first_effect_tokens = split_lexed_sentences(sentences[0].lowered())
            .into_iter()
            .next()
            .unwrap_or_else(|| sentences[0].lowered());
        let mut effects = parse_effect_sentence_sequence(first_effect_tokens)?;
        effects.extend(vec![
            EffectAst::subject_verb_tag_matching_objects(
                ObjectFilter::tagged(TagKey::from(IT_TAG)),
                vec![Zone::Library, Zone::Graveyard],
                TagKey::from("divvy_source"),
            ),
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::exactly(2),
                count_value: None,
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Library, Zone::Graveyard],
                search_mode: None,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    )],
                }],
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_chosen"),
                effects: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Library,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            },
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::You,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
            EffectAst::subject_verb_exile(TargetAst::Source(None), false),
        ]);
        return Ok(Some(effects));
    }

    if sentences.len() >= 2
        && sentence_has_phrase(
            &sentence_words,
            &["an", "opponent", "chooses", "one", "of", "them"],
        )
        && sentence_has_phrase(
            &sentence_words,
            &["put", "the", "chosen", "card", "into", "your", "hand"],
        )
        && sentence_has_phrase(
            &sentence_words,
            &["the", "other", "into", "your", "graveyard"],
        )
    {
        let mut prefix = Vec::new();
        prefix.extend(parse_effect_sentence_lexed(sentences[0].lowered())?);
        prefix.extend(parse_effect_sentence_lexed(sentences[1].lowered())?);
        let mut effects = prefix;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
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
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(effects));
    }

    if first_sentence_has_prefix(
        &sentence_words,
        &[
            "search",
            "your",
            "library",
            "for",
            "up",
            "to",
            "four",
            "cards",
            "with",
            "different",
            "names",
            "and",
            "reveal",
            "them",
        ],
    ) && sentence_has_phrase(
        &sentence_words,
        &[
            "target", "opponent", "chooses", "two", "of", "those", "cards",
        ],
    ) && sentence_has_phrase(
        &sentence_words,
        &["put", "the", "chosen", "cards", "into", "your", "graveyard"],
    ) && sentence_has_phrase(&sentence_words, &["the", "rest", "into", "your", "hand"])
    {
        let mut effects = parse_effect_sentence_lexed(sentences[0].lowered())?;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(2),
            count_value: None,
            player: PlayerAst::TargetOpponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Graveyard,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    Zone::Hand,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                )],
            }],
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(effects));
    }

    if sentence_has_phrase(&sentence_words, &["target", "opponent", "chooses", "one"])
        && sentence_has_phrase(
            &sentence_words,
            &["put", "that", "card", "into", "your", "hand"],
        )
        && sentence_has_phrase(
            &sentence_words,
            &["the", "rest", "into", "your", "graveyard"],
        )
    {
        let mut effects = parse_effect_sentence_lexed(sentences[0].lowered())?;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            ObjectFilter::tagged(TagKey::from(IT_TAG)),
            vec![Zone::Library],
            TagKey::from("divvy_source"),
        ));
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::TargetOpponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ));
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
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
        });
        effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(effects));
    }

    Ok(None)
}

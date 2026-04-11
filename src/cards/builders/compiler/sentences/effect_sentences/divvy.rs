use super::super::grammar::primitives::TokenWordView;
use super::super::lexer::OwnedLexToken;
use super::dispatch_entry::SentenceInput;
use super::dispatch_inner::parse_effect_sentence_lexed;
use crate::cards::builders::{
    CardTextError, EffectAst, IT_TAG, PlayerAst, PredicateAst, ReturnControllerAst, TagKey,
    TargetAst,
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

fn parse_single_effect_sentence(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    parse_effect_sentence_lexed(tokens)?
        .into_iter()
        .next()
        .ok_or_else(|| CardTextError::ParseError("missing effect sentence".to_string()))
}

fn matches_sentence(words: &TokenWordView<'_>, expected: &[&str]) -> bool {
    words.len() == expected.len() && words.starts_with(expected)
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
        .is_some_and(|words| words.starts_with(prefix))
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
            EffectAst::TargetOnly {
                target: TargetAst::WithCount(
                    Box::new(TargetAst::Player(PlayerFilter::Any, None)),
                    ChoiceCount::exactly(2),
                ),
            },
            EffectAst::ChoosePlayer {
                chooser: PlayerAst::You,
                filter: PlayerFilter::target_player(),
                tag: first_player_tag.clone(),
                random: false,
                exclude_previous_choices: 0,
            },
            EffectAst::ChoosePlayer {
                chooser: PlayerAst::You,
                filter: PlayerFilter::target_player(),
                tag: second_player_tag.clone(),
                random: false,
                exclude_previous_choices: 1,
            },
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
                effects: vec![EffectAst::GainControl {
                    target: TargetAst::Tagged(first_creatures_tag, None),
                    player: PlayerAst::That,
                    duration: Until::Forever,
                }],
            },
            EffectAst::ForEachTaggedPlayer {
                tag: first_player_tag,
                effects: vec![EffectAst::GainControl {
                    target: TargetAst::Tagged(second_creatures_tag, None),
                    player: PlayerAst::That,
                    duration: Until::Forever,
                }],
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
            EffectAst::DestroyNoRegeneration {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            },
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
            EffectAst::Exile {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                face_down: false,
            },
            EffectAst::ReturnAllToBattlefield {
                filter: rest_filter,
                tapped: false,
            },
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
        return Ok(Some(vec![EffectAst::ForEachPlayersFiltered {
            filter: PlayerFilter::Opponent,
            effects: vec![
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::You,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::SacrificeAll {
                    filter: ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .match_tagged(
                            TagKey::from("divvy_chosen"),
                            TaggedOpbjectRelation::IsTaggedObject,
                        ),
                    player: PlayerAst::Implicit,
                },
            ],
        }]));
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
            EffectAst::SacrificeAll {
                filter: ObjectFilter::tagged(TagKey::from("divvy_chosen")),
                player: PlayerAst::Target,
            },
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
                EffectAst::Cant {
                    restriction: crate::effect::Restriction::block(
                        ObjectFilter::creature()
                            .controlled_by(PlayerFilter::IteratedPlayer)
                            .not_tagged(TagKey::from("divvy_chosen")),
                    ),
                    duration: Until::EndOfTurn,
                    condition: None,
                },
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
            EffectAst::Cant {
                restriction: crate::effect::Restriction::attack(
                    ObjectFilter::creature()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                ),
                duration: Until::EndOfTurn,
                condition: None,
            },
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
                EffectAst::ChoosePlayer {
                    chooser: PlayerAst::Implicit,
                    filter: PlayerFilter::Opponent,
                    tag: TagKey::from("divvy_opponent"),
                    random: false,
                    exclude_previous_choices: 0,
                },
                EffectAst::ChooseObjects {
                    filter: ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer),
                    count: ChoiceCount::any_number(),
                    count_value: None,
                    player: PlayerAst::Chosen,
                    tag: TagKey::from("divvy_chosen"),
                },
                EffectAst::Destroy {
                    target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                },
                EffectAst::TapAll {
                    filter: ObjectFilter::land()
                        .nontoken()
                        .controlled_by(PlayerFilter::IteratedPlayer)
                        .not_tagged(TagKey::from("divvy_chosen")),
                },
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
        return Ok(Some(vec![
            parse_single_effect_sentence(sentences[0].lowered())?,
            EffectAst::TagMatchingObjects {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                zones: vec![Zone::Exile],
                tag: TagKey::from("divvy_source"),
            },
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::ReturnToHand {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                random: false,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::MoveToZone {
                        target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        zone: Zone::Graveyard,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
            },
        ]));
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
        return Ok(Some(vec![
            parse_single_effect_sentence(sentences[0].lowered())?,
            EffectAst::TagMatchingObjects {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                zones: vec![Zone::Exile],
                tag: TagKey::from("divvy_source"),
            },
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::any_number(),
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Exile],
                search_mode: None,
            },
            EffectAst::MoveToZone {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                zone: Zone::Battlefield,
                to_top: false,
                battlefield_controller: ReturnControllerAst::You,
                battlefield_tapped: false,
                attached_to: None,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::MoveToZone {
                        target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        zone: Zone::Graveyard,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
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
            "with",
            "different",
            "names",
            "that",
            "each",
            "have",
            "mana",
            "value",
            "x",
            "or",
            "less",
            "and",
            "reveal",
            "them",
        ],
    ) && sentence_has_phrase(
        &sentence_words,
        &["an", "opponent", "chooses", "two", "of", "those", "cards"],
    ) && sentence_has_phrase(
        &sentence_words,
        &[
            "shuffle", "the", "chosen", "cards", "into", "your", "library",
        ],
    ) && sentence_has_phrase(
        &sentence_words,
        &["put", "the", "rest", "onto", "the", "battlefield"],
    ) {
        return Ok(Some(vec![
            parse_single_effect_sentence(sentences[0].lowered())?,
            EffectAst::TagMatchingObjects {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                zones: vec![Zone::Library, Zone::Graveyard],
                tag: TagKey::from("divvy_source"),
            },
            EffectAst::ChooseObjectsAcrossZones {
                filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
                count: ChoiceCount::exactly(2),
                player: PlayerAst::Opponent,
                tag: TagKey::from("divvy_chosen"),
                zones: vec![Zone::Library, Zone::Graveyard],
                search_mode: None,
            },
            EffectAst::MoveToZone {
                target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            },
            EffectAst::ShuffleLibrary {
                player: PlayerAst::You,
            },
            EffectAst::ForEachTagged {
                tag: TagKey::from("divvy_source"),
                effects: vec![EffectAst::Conditional {
                    predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                    if_true: Vec::new(),
                    if_false: vec![EffectAst::MoveToZone {
                        target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        zone: Zone::Battlefield,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::You,
                        battlefield_tapped: false,
                        attached_to: None,
                    }],
                }],
            },
            EffectAst::Exile {
                target: TargetAst::Source(None),
                face_down: false,
            },
        ]));
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
        effects.push(EffectAst::TagMatchingObjects {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            zones: vec![Zone::Library],
            tag: TagKey::from("divvy_source"),
        });
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Opponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    zone: Zone::Graveyard,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                }],
            }],
        });
        effects.push(EffectAst::ShuffleLibrary {
            player: PlayerAst::You,
        });
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
        effects.push(EffectAst::TagMatchingObjects {
            filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
            zones: vec![Zone::Library],
            tag: TagKey::from("divvy_source"),
        });
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::tagged(TagKey::from("divvy_source")),
            count: ChoiceCount::exactly(1),
            player: PlayerAst::TargetOpponent,
            tag: TagKey::from("divvy_chosen"),
            zones: vec![Zone::Library],
            search_mode: None,
        });
        effects.push(EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from("divvy_chosen"), None),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        });
        effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("divvy_source"),
            effects: vec![EffectAst::Conditional {
                predicate: membership_predicate_for_iterated_object("divvy_chosen"),
                if_true: Vec::new(),
                if_false: vec![EffectAst::MoveToZone {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), None),
                    zone: Zone::Graveyard,
                    to_top: false,
                    battlefield_controller: ReturnControllerAst::Preserve,
                    battlefield_tapped: false,
                    attached_to: None,
                }],
            }],
        });
        effects.push(EffectAst::ShuffleLibrary {
            player: PlayerAst::You,
        });
        return Ok(Some(effects));
    }

    Ok(None)
}

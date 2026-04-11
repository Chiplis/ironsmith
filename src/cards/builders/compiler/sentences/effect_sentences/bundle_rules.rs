use winnow::Parser as _;

use super::super::activation_and_restrictions::choice_object_clauses::{
    parse_choose_card_type_phrase_words, parse_target_player_choose_objects_clause,
    parse_you_choose_objects_clause,
};
use super::super::lexer::{OwnedLexToken, TokenKind, split_lexed_sentences};
use super::super::object_filters::parse_object_filter_lexed;
use super::super::permission_helpers::{
    parse_until_end_of_turn_may_play_tagged_clause,
    parse_until_your_next_turn_may_play_tagged_clause,
};
use super::super::token_primitives::find_index;
use super::super::util::{parse_subject, span_from_tokens, trim_commas, words};
use super::dispatch_entry::parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard;
use super::zone_handlers::parse_exile_top_library_clause;
use crate::cards::builders::compiler::effect_sentences;
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, PlayerAst, PredicateAst,
    ReturnControllerAst, TagKey, TargetAst, TextSpan, Verb,
};
use crate::effect::Value;
use crate::target::{ObjectFilter, PlayerFilter};
use crate::types::Subtype;
use crate::zone::Zone;

pub(crate) fn parse_same_sentence_copy_and_may_cast_copy(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<(
        Vec<EffectAst>,
        crate::cards::builders::compiler::activation_and_restrictions::trigger_subject_filters::MayCastTaggedSpec,
    )>,
    CardTextError,
>{
    use super::super::grammar::primitives as grammar;

    let split = grammar::split_lexed_once_on_separator(tokens, || grammar::kw("and").void())
        .or_else(|| grammar::split_lexed_once_on_separator(tokens, || grammar::kw("then").void()));
    let Some((copy_slice, tail_slice)) = split else {
        return Ok(None);
    };

    let copy_tokens = trim_commas(copy_slice).to_vec();
    if !effect_sentences::is_simple_copy_reference_sentence(&copy_tokens) {
        return Ok(None);
    }

    let tail_tokens = trim_commas(tail_slice).to_vec();
    let Some(spec) = effect_sentences::parse_may_cast_it_sentence(&tail_tokens) else {
        return Ok(None);
    };
    if !spec.as_copy {
        return Ok(None);
    }

    let copy_effects = effect_sentences::parse_effect_sentence_lexed(&copy_tokens)?;
    Ok(Some((copy_effects, spec)))
}

fn parse_exile_top_library_then_play_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_idx)) = effect_sentences::find_verb(first_sentence) else {
        return Ok(None);
    };
    if verb != Verb::Exile {
        return Ok(None);
    }

    let exile_subject = if verb_idx == 0 {
        None
    } else {
        Some(parse_subject(&trim_commas(&first_sentence[..verb_idx])))
    };
    let exile_tokens = trim_commas(&first_sentence[verb_idx + 1..]);
    let Some(exile_effect) = parse_exile_top_library_clause(&exile_tokens, exile_subject) else {
        return Ok(None);
    };
    let permission_effect = if let Some(effect) =
        parse_until_end_of_turn_may_play_tagged_clause(second_sentence)?
    {
        effect
    } else if let Some(effect) = parse_until_your_next_turn_may_play_tagged_clause(second_sentence)?
    {
        effect
    } else {
        return Ok(None);
    };

    let Some(tag) = (match &exile_effect {
        EffectAst::ExileTopOfLibrary { tags, .. } => tags.first().cloned(),
        _ => None,
    }) else {
        return Ok(None);
    };

    let permission_effect = match permission_effect {
        EffectAst::GrantPlayTaggedUntilEndOfTurn {
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
            ..
        } => EffectAst::GrantPlayTaggedUntilEndOfTurn {
            tag,
            player,
            allow_land,
            without_paying_mana_cost,
            allow_any_color_for_cast,
        },
        EffectAst::GrantPlayTaggedUntilYourNextTurn {
            player, allow_land, ..
        } => EffectAst::GrantPlayTaggedUntilYourNextTurn {
            tag,
            player,
            allow_land,
        },
        _ => return Ok(None),
    };

    Ok(Some(vec![exile_effect, permission_effect]))
}

fn looks_like_source_leaves_return_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::compiler::token_word_refs(tokens);
    if words.first().copied() != Some("return") {
        return false;
    }
    if !words.iter().any(|word| *word == "when")
        || !words.iter().any(|word| *word == "leaves")
        || !words.iter().any(|word| *word == "battlefield")
        || !words
            .windows(3)
            .any(|window| window == ["to", "the", "battlefield"])
        || !words
            .iter()
            .any(|word| matches!(*word, "owner" | "owners" | "owner's" | "owners'"))
        || !words.iter().any(|word| *word == "control")
    {
        return false;
    }

    true
}

fn promote_exile_effect_to_source_leaves(effect: EffectAst) -> Option<EffectAst> {
    match effect {
        EffectAst::Exile { target, face_down } => {
            Some(EffectAst::ExileUntilSourceLeaves { target, face_down })
        }
        EffectAst::ExileAll { filter, face_down } => Some(EffectAst::ExileUntilSourceLeaves {
            target: TargetAst::Object(filter, None, None),
            face_down,
        }),
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } if if_false.is_empty() && if_true.len() == 1 => {
            let inner = promote_exile_effect_to_source_leaves(if_true.into_iter().next().unwrap())?;
            Some(EffectAst::Conditional {
                predicate,
                if_true: vec![inner],
                if_false,
            })
        }
        _ => None,
    }
}

fn parse_exile_then_source_leaves_return_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !looks_like_source_leaves_return_followup_sentence(second_sentence) {
        return Ok(None);
    }

    let first_effects = effect_sentences::parse_effect_sentence_lexed(first_sentence)?;
    let [first_effect] = first_effects.as_slice() else {
        return Ok(None);
    };
    let Some(rewritten_first_effect) = promote_exile_effect_to_source_leaves(first_effect.clone())
    else {
        return Ok(None);
    };

    Ok(Some(vec![rewritten_first_effect]))
}

fn parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let second_tokens = trim_commas(second);
    let first_words = words(&first_tokens);
    let second_words = words(&second_tokens);

    if second_words.as_slice() != ["put", "that", "card", "into", "your", "hand"] {
        return Ok(None);
    }

    let Some(or_idx) = find_index(&first_tokens, |token| token.is_word("or")) else {
        return Ok(None);
    };
    if or_idx == 0 || or_idx + 1 >= first_tokens.len() {
        return Ok(None);
    }

    let reveal_tokens = trim_commas(&first_tokens[..or_idx]);
    let choose_tokens = trim_commas(&first_tokens[or_idx + 1..]);
    let reveal_words = words(&reveal_tokens);
    let choose_words = words(&choose_tokens);

    if !reveal_words.iter().any(|word| *word == "outside")
        || !reveal_words.iter().any(|word| *word == "game")
    {
        return Ok(None);
    }
    let has_face_up = choose_words
        .iter()
        .any(|word| *word == "face-up" || *word == "faceup")
        || choose_words
            .windows(2)
            .any(|window| window == ["face", "up"]);
    if !has_face_up {
        return Ok(None);
    }
    if !choose_words.iter().any(|word| *word == "exile") {
        return Ok(None);
    }

    let reveal_from_idx =
        find_index(&reveal_tokens, |token| token.is_word("from")).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing outside-game clause in reveal-or-choose bundle (clause: '{}')",
                first_words.join(" ")
            ))
        })?;
    if reveal_from_idx < 3 {
        return Ok(None);
    }
    let reveal_filter_tokens = trim_commas(&reveal_tokens[3..reveal_from_idx]);
    let reveal_filter = parse_object_filter_lexed(&reveal_filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported outside-game reveal filter in reveal-or-choose bundle (clause: '{}')",
            first_words.join(" ")
        ))
    })?;
    let choose_filter = parse_object_filter_lexed(&choose_tokens[1..], false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported exile choice filter in reveal-or-choose bundle (clause: '{}')",
            first_words.join(" ")
        ))
    })?;

    if reveal_filter.card_types != choose_filter.card_types
        || reveal_filter.subtypes != choose_filter.subtypes
        || reveal_filter.owner != choose_filter.owner
    {
        return Ok(None);
    }

    let chosen_tag = TagKey::from("__coax_or_karn_selected__");
    let effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
            zones: vec![Zone::Exile],
            search_mode: None,
        },
        EffectAst::RevealTagged {
            tag: chosen_tag.clone(),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(chosen_tag, span_from_tokens(second)),
            zone: Zone::Hand,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
    ];

    Ok(Some(vec![EffectAst::May { effects }]))
}

fn parse_choose_objects_then_for_each_of_those_bundle(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
    third: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn word_is(word: Option<&str>, expected: &str) -> bool {
        word.is_some_and(|word| word.eq_ignore_ascii_case(expected))
    }

    let mut normalized_first = first.to_vec();
    for token in &mut normalized_first {
        token.lowercase_word();
    }

    let Some((player, filter, count)) = parse_you_choose_objects_clause(&normalized_first)?
        .or_else(|| {
            parse_target_player_choose_objects_clause(&normalized_first)
                .ok()
                .flatten()
        })
    else {
        return Ok(None);
    };
    let choose_tag = TagKey::from(IT_TAG);

    let second_words = crate::cards::builders::compiler::token_word_refs(second);
    if second_words.len() < 5
        || !word_is(second_words.first().copied(), "for")
        || !word_is(second_words.get(1).copied(), "each")
        || !word_is(second_words.get(2).copied(), "of")
        || !word_is(second_words.get(3).copied(), "those")
    {
        return Ok(None);
    }

    let Some(comma_idx) = find_index(second, |token| token.is_comma()) else {
        return Ok(None);
    };
    let loop_body_tokens = trim_commas(&second[comma_idx + 1..]);
    if loop_body_tokens.is_empty() {
        return Ok(None);
    }
    let loop_body_effects = effect_sentences::parse_effect_sentence_lexed(&loop_body_tokens)?;
    if loop_body_effects.is_empty() {
        return Ok(None);
    }

    let trailing_effects = effect_sentences::parse_effect_sentence_lexed(third)?;
    if trailing_effects.is_empty() {
        return Ok(None);
    }

    let mut combined = vec![EffectAst::ChooseObjects {
        filter,
        count,
        count_value: None,
        player,
        tag: choose_tag.clone(),
    }];
    combined.push(EffectAst::ForEachTagged {
        tag: choose_tag,
        effects: loop_body_effects,
    });
    combined.extend(trailing_effects);
    Ok(Some(combined))
}

fn parser_words(tokens: &[OwnedLexToken]) -> Vec<String> {
    tokens
        .iter()
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Comma | TokenKind::Period | TokenKind::LParen | TokenKind::RParen
            )
        })
        .map(|token| token.parser_text().to_string())
        .filter(|word| !word.is_empty())
        .collect()
}

fn parse_soul_partition_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() != 3 {
        return None;
    }

    let first_words = parser_words(sentences[0]);
    let second_words = parser_words(sentences[1]);
    let third_words = parser_words(sentences[2]);
    let third_word_refs = third_words.iter().map(String::as_str).collect::<Vec<_>>();
    let mana_word = third_words
        .iter()
        .find(|word| *word == "2" || *word == "{2}");

    if first_words.as_slice() != ["exile", "target", "nonland", "permanent"]
        || second_words.as_slice()
            != [
                "for", "as", "long", "as", "that", "card", "remains", "exiled", "its", "owner",
                "may", "play", "it",
            ]
        || !matches!(
            third_word_refs.as_slice(),
            [
                "a",
                "spell",
                "cast",
                "by",
                "an",
                "opponent",
                "this",
                "way",
                "costs",
                _,
                "more",
                "to",
                "cast",
            ]
        )
        || mana_word.is_none()
    {
        return None;
    }

    let first_sentence = sentences.first()?;
    let mut effects = effect_sentences::parse_effect_sentences_lexed(first_sentence).ok()?;
    effects.push(EffectAst::GrantBySpec {
        spec: crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            crate::filter::ObjectFilter::tagged(crate::cards::builders::TagKey::from(IT_TAG)),
            Zone::Exile,
        ),
        player: crate::cards::builders::PlayerAst::ItsOwner,
        duration: crate::grant::GrantDuration::Forever,
    });
    effects.push(EffectAst::GrantToTarget {
        target: crate::cards::builders::TargetAst::Tagged(
            crate::cards::builders::TagKey::from(IT_TAG),
            None,
        ),
        grantable: crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
            crate::static_abilities::CostIncreaseManaCost::new(
                crate::filter::ObjectFilter::spell()
                    .without_type(crate::types::CardType::Land)
                    .cast_by(crate::PlayerFilter::Opponent),
                crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(2)]),
            ),
        )),
        duration: crate::grant::GrantDuration::Forever,
    });
    Some(effects)
}

fn parse_empty_laboratory_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentence_words = parser_words(tokens);
    if sentence_words.as_slice()
        != [
            "sacrifice",
            "x",
            "zombies",
            "then",
            "reveal",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "your",
            "library",
            "until",
            "you",
            "reveal",
            "a",
            "number",
            "of",
            "zombie",
            "creature",
            "cards",
            "equal",
            "to",
            "the",
            "number",
            "of",
            "zombies",
            "sacrificed",
            "this",
            "way",
            "put",
            "those",
            "cards",
            "onto",
            "the",
            "battlefield",
            "and",
            "the",
            "rest",
            "on",
            "the",
            "bottom",
            "of",
            "your",
            "library",
            "in",
            "a",
            "random",
            "order",
        ]
    {
        return None;
    }

    let sacrificed_tag = TagKey::from("sacrificed_0");
    let revealed_tag = TagKey::from("etl_revealed");
    let matched_tag = TagKey::from("etl_matched");

    let mut zombie_you_control = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    zombie_you_control.subtypes.push(Subtype::Zombie);

    let mut zombie_creature_card = ObjectFilter::creature();
    zombie_creature_card.subtypes.push(Subtype::Zombie);
    zombie_creature_card.zone = None;

    Some(vec![
        EffectAst::ChooseObjects {
            filter: zombie_you_control,
            count: ChoiceCount::dynamic_x(),
            count_value: None,
            player: PlayerAst::You,
            tag: sacrificed_tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(sacrificed_tag),
            player: PlayerAst::You,
        },
        EffectAst::ConsultTopOfLibrary {
            player: PlayerAst::You,
            mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
            filter: zombie_creature_card,
            stop_rule: crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(
                crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount),
            ),
            all_tag: revealed_tag.clone(),
            match_tag: matched_tag.clone(),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(matched_tag.clone(), None),
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
        EffectAst::PutTaggedRemainderOnBottomOfLibrary {
            tag: revealed_tag,
            keep_tagged: Some(matched_tag),
            order: crate::cards::builders::LibraryBottomOrderAst::Random,
            player: PlayerAst::You,
        },
    ])
}

fn parse_shape_anew_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentence_words = parser_words(tokens);
    if sentence_words.as_slice()
        != [
            "the",
            "controller",
            "of",
            "target",
            "artifact",
            "sacrifices",
            "it",
            "then",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            "until",
            "they",
            "reveal",
            "an",
            "artifact",
            "card",
            "that",
            "player",
            "puts",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "then",
            "shuffles",
            "all",
            "other",
            "cards",
            "revealed",
            "this",
            "way",
            "into",
            "their",
            "library",
        ]
    {
        return None;
    }

    let revealed_tag = TagKey::from("shape_anew_revealed");
    let matched_tag = TagKey::from("shape_anew_matched");
    let mut artifact_card = ObjectFilter::artifact();
    artifact_card.zone = None;
    let target = TargetAst::Object(
        ObjectFilter::artifact().in_zone(Zone::Battlefield),
        Some(TextSpan::synthetic()),
        None,
    );

    Some(vec![
        EffectAst::Sacrifice {
            filter: ObjectFilter::default(),
            player: PlayerAst::ItsController,
            count: 1,
            target: Some(target),
        },
        EffectAst::ConsultTopOfLibrary {
            player: PlayerAst::That,
            mode: crate::cards::builders::LibraryConsultModeAst::Reveal,
            filter: artifact_card,
            stop_rule: crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
            all_tag: revealed_tag,
            match_tag: matched_tag.clone(),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(matched_tag, None),
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
            battlefield_tapped: false,
            attached_to: None,
        },
        EffectAst::ShuffleLibrary {
            player: PlayerAst::That,
        },
    ])
}

fn parse_collision_of_realms_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentence_words = parser_words(tokens);
    if sentence_words.as_slice()
        != [
            "each",
            "player",
            "shuffles",
            "all",
            "creatures",
            "they",
            "own",
            "into",
            "their",
            "library",
            "each",
            "player",
            "who",
            "shuffled",
            "a",
            "nontoken",
            "creature",
            "into",
            "their",
            "library",
            "this",
            "way",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            "until",
            "they",
            "reveal",
            "a",
            "creature",
            "card",
            "then",
            "puts",
            "that",
            "card",
            "onto",
            "the",
            "battlefield",
            "and",
            "the",
            "rest",
            "on",
            "the",
            "bottom",
            "of",
            "their",
            "library",
            "in",
            "a",
            "random",
            "order",
        ]
    {
        return None;
    }

    let mut owned_creatures = ObjectFilter::creature();
    owned_creatures.zone = Some(Zone::Battlefield);
    owned_creatures.owner = Some(PlayerFilter::IteratedPlayer);

    let mut owned_nontoken_creatures = owned_creatures.clone();
    owned_nontoken_creatures.nontoken = true;

    let mut tagged_library_filter = ObjectFilter::default();
    tagged_library_filter.zone = Some(Zone::Library);

    let mut creature_card = ObjectFilter::creature();
    creature_card.zone = None;

    let tagged_creatures = TagKey::from("collision_all_shuffled");
    let tagged_nontoken = TagKey::from("collision_nontoken_shuffled");
    let revealed_tag = TagKey::from("collision_revealed");
    let matched_tag = TagKey::from("collision_matched");

    Some(vec![EffectAst::ForEachPlayer {
        effects: vec![
            EffectAst::TagMatchingObjects {
                filter: owned_creatures.clone(),
                zones: vec![Zone::Battlefield],
                tag: tagged_creatures.clone(),
            },
            EffectAst::TagMatchingObjects {
                filter: owned_nontoken_creatures,
                zones: vec![Zone::Battlefield],
                tag: tagged_nontoken.clone(),
            },
            EffectAst::MoveToZone {
                target: TargetAst::Tagged(tagged_creatures, None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
                battlefield_tapped: false,
                attached_to: None,
            },
            EffectAst::ShuffleLibrary {
                player: PlayerAst::That,
            },
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::That,
                    tag: tagged_nontoken,
                    filter: tagged_library_filter,
                },
                if_true: vec![
                    EffectAst::ConsultTopOfLibrary {
                        player: PlayerAst::That,
                        mode: LibraryConsultModeAst::Reveal,
                        filter: creature_card,
                        stop_rule: LibraryConsultStopRuleAst::FirstMatch,
                        all_tag: revealed_tag.clone(),
                        match_tag: matched_tag.clone(),
                    },
                    EffectAst::MoveToZone {
                        target: TargetAst::Tagged(matched_tag.clone(), None),
                        zone: Zone::Battlefield,
                        to_top: false,
                        battlefield_controller: ReturnControllerAst::Preserve,
                        battlefield_tapped: false,
                        attached_to: None,
                    },
                    EffectAst::PutTaggedRemainderOnBottomOfLibrary {
                        tag: revealed_tag,
                        keep_tagged: Some(matched_tag),
                        order: LibraryBottomOrderAst::Random,
                        player: PlayerAst::That,
                    },
                ],
                if_false: Vec::new(),
            },
        ],
    }])
}

fn parse_nissas_encouragement_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentence_words = parser_words(tokens);
    if sentence_words.as_slice()
        != [
            "search",
            "your",
            "library",
            "and",
            "graveyard",
            "for",
            "a",
            "card",
            "named",
            "forest",
            "a",
            "card",
            "named",
            "brambleweft",
            "behemoth",
            "and",
            "a",
            "card",
            "named",
            "nissa",
            "genesis",
            "mage",
            "reveal",
            "those",
            "cards",
            "put",
            "them",
            "into",
            "your",
            "hand",
            "then",
            "shuffle",
        ]
    {
        return None;
    }

    let searched_tag = TagKey::from("searched_named");
    let zones = vec![Zone::Library, Zone::Graveyard];
    let names = ["Forest", "Brambleweft Behemoth", "Nissa, Genesis Mage"];
    let mut effects = Vec::new();
    for name in names {
        let mut filter = ObjectFilter::default();
        filter.name = Some(name.to_string());
        effects.push(EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: searched_tag.clone(),
            zones: zones.clone(),
            search_mode: Some(crate::effect::SearchSelectionMode::Exact),
        });
    }
    effects.push(EffectAst::RevealTagged {
        tag: searched_tag.clone(),
    });
    effects.push(EffectAst::MoveToZone {
        target: TargetAst::Tagged(searched_tag, None),
        zone: Zone::Hand,
        to_top: false,
        battlefield_controller: ReturnControllerAst::Preserve,
        battlefield_tapped: false,
        attached_to: None,
    });
    effects.push(EffectAst::ShuffleLibrary {
        player: PlayerAst::You,
    });
    Some(effects)
}

pub(crate) fn parse_exact_card_effect_bundle_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    if let Some(effects) = parse_soul_partition_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_empty_laboratory_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_shape_anew_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_collision_of_realms_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_nissas_encouragement_bundle(tokens) {
        return Some(effects);
    }
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_then_source_leaves_return_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            parse_exile_top_library_then_play_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_reveal_from_outside_game_or_choose_face_up_exile_to_hand(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && let Ok(Some(effects)) = parse_choose_objects_then_for_each_of_those_bundle(
            sentences[0],
            sentences[1],
            sentences[2],
        )
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Ok(Some(effects)) =
            effect_sentences::parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand(
                sentences[0],
                sentences[1],
            )
    {
        return Some(effects);
    }
    if sentences.len() == 3
        && {
            let first_words = crate::cards::builders::compiler::token_word_refs(sentences[0]);
            let choice_words = if first_words.first().copied() == Some("you") {
                &first_words[1..]
            } else {
                &first_words[..]
            };
            matches!(
                parse_choose_card_type_phrase_words(choice_words),
                Ok(Some((consumed, _))) if consumed == choice_words.len()
            )
        }
        && let Ok(Some(mut effects)) =
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
                sentences[1],
                sentences[2],
            )
    {
        let first_words = crate::cards::builders::compiler::token_word_refs(sentences[0]);
        let choice_words = if first_words.first().copied() == Some("you") {
            &first_words[1..]
        } else {
            &first_words[..]
        };
        let (_, options) = parse_choose_card_type_phrase_words(choice_words)
            .ok()
            .flatten()
            .expect("validated choose-card-type bundle prefix");
        let mut combined = vec![EffectAst::ChooseCardType {
            player: PlayerAst::You,
            options,
        }];
        combined.append(&mut effects);
        return Some(combined);
    }
    let sentence_words = tokens
        .iter()
        .filter_map(|token| match token.kind {
            TokenKind::Word | TokenKind::Number | TokenKind::Tilde => Some(token.parser_text()),
            _ => None,
        })
        .collect::<Vec<_>>();

    if sentence_words.as_slice()
        == [
            "look", "at", "the", "top", "x", "cards", "of", "your", "library", "where", "x", "is",
            "your", "devotion", "to", "blue", "put", "up", "to", "one", "of", "them", "on", "top",
            "of", "your", "library", "and", "the", "rest", "on", "the", "bottom", "of", "your",
            "library", "in", "a", "random", "order", "if", "x", "is", "greater", "than", "or",
            "equal", "to", "the", "number", "of", "cards", "in", "your", "library", "you", "win",
            "the", "game",
        ]
    {
        let looked_tag = TagKey::from("thassas_oracle_looked");
        return Some(vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::Devotion {
                    player: PlayerFilter::You,
                    color: crate::color::Color::Blue,
                },
                tag: looked_tag.clone(),
            },
            EffectAst::RearrangeLookedCardsInLibrary {
                tag: looked_tag,
                player: PlayerAst::You,
                count: ChoiceCount::up_to(1),
            },
            EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::ValueComparison {
                    left: Value::Devotion {
                        player: PlayerFilter::You,
                        color: crate::color::Color::Blue,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::CardsInLibrary(PlayerFilter::You),
                },
                if_true: vec![EffectAst::WinGame {
                    player: PlayerAst::You,
                }],
                if_false: Vec::new(),
            },
        ]);
    }

    if sentence_words.as_slice()
        == [
            "if",
            "this",
            "spell",
            "was",
            "cast",
            "from",
            "a",
            "graveyard",
            "copy",
            "this",
            "spell",
            "and",
            "you",
            "may",
            "choose",
            "a",
            "new",
            "target",
            "for",
            "the",
            "copy",
        ]
    {
        return Some(vec![EffectAst::Conditional {
            predicate: crate::cards::builders::PredicateAst::ThisSpellWasCastFromZone(
                Zone::Graveyard,
            ),
            if_true: vec![EffectAst::CopySpell {
                target: TargetAst::Source(None),
                count: Value::Fixed(1),
                player: PlayerAst::Implicit,
                may_choose_new_targets: true,
            }],
            if_false: Vec::new(),
        }]);
    }

    if sentence_words.as_slice()
        == [
            "search", "your", "library", "for", "a", "basic", "forest", "card", "and", "a",
            "basic", "plains", "card", "reveal", "those", "cards", "put", "them", "into", "your",
            "hand", "then", "shuffle",
        ]
    {
        return Some(vec![EffectAst::SearchLibrarySlotsToHand {
            slots: vec![
                crate::cards::builders::SearchLibrarySlotAst {
                    filter: ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You)
                        .with_type(crate::types::CardType::Land)
                        .with_supertype(crate::types::Supertype::Basic)
                        .with_subtype(crate::types::Subtype::Forest),
                    optional: true,
                },
                crate::cards::builders::SearchLibrarySlotAst {
                    filter: ObjectFilter::default()
                        .in_zone(Zone::Library)
                        .owned_by(PlayerFilter::You)
                        .with_type(crate::types::CardType::Land)
                        .with_supertype(crate::types::Supertype::Basic)
                        .with_subtype(crate::types::Subtype::Plains),
                    optional: true,
                },
            ],
            player: PlayerAst::You,
            reveal: true,
            progress_tag: TagKey::from("yasharn_search_progress"),
        }]);
    }

    None
}

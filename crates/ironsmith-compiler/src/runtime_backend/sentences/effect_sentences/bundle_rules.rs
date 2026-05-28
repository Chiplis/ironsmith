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
use crate::cards::builders::{
    CardTextError, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst, LibraryConsultModeAst,
    LibraryConsultStopRuleAst, PlayerAst, PredicateAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan, Verb,
};
use crate::effect::{EventValueSpec, Value};
use crate::runtime_backend::effect_sentences;
use crate::target::{ObjectFilter, PlayerFilter, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

pub(crate) fn parse_same_sentence_copy_and_may_cast_copy(
    tokens: &[OwnedLexToken],
) -> Result<
    Option<(
        Vec<EffectAst>,
        crate::runtime_backend::activation_and_restrictions::trigger_subject_filters::MayCastTaggedSpec,
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
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::ExileTopOfLibrary { tags, .. } => tags.first().cloned(),
            _ => None,
        },
        _ => None,
    }) else {
        return Ok(None);
    };

    let permission_effect = match permission_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilEndOfTurn {
                    player,
                    allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    single_spell,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_end_of_turn(
            tag,
            player,
            allow_land,
                    without_paying_mana_cost,
                    allow_any_color_for_cast,
                    single_spell,
                ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedUntilYourNextTurn {
                    player, allow_land, ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_until_your_next_turn(
            tag, player, allow_land, false,
        ),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantPlayTaggedForAsLongAsExiled {
                    player,
                    allow_land,
                    allow_any_color_for_cast,
                    ..
                },
            ..
        }) => EffectAst::subject_verb_grant_play_tagged_for_as_long_as_exiled(
            tag,
            player,
            allow_land,
            allow_any_color_for_cast,
        ),
        _ => return Ok(None),
    };

    Ok(Some(vec![exile_effect, permission_effect]))
}

fn parse_choose_type_then_phase_out_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first_sentence)?
    else {
        return Ok(None);
    };
    if !choose_count.is_single() {
        return Ok(None);
    }

    let second_words = crate::runtime_backend::token_word_refs(second_sentence);
    if !second_words
        .iter()
        .any(|word| matches!(*word, "that" | "chosen"))
        || !second_words.iter().any(|word| *word == "type")
    {
        return Ok(None);
    }

    let mut effects = effect_sentences::parse_effect_sentence_lexed(second_sentence)?;
    let [
        EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
            action: crate::cards::builders::SubjectVerbActionAst::PhaseOutAll { filter },
            ..
        }),
    ] = effects.as_mut_slice()
    else {
        return Ok(None);
    };

    if choose_filter.card_types.is_empty() {
        return Ok(None);
    }

    let mut phase_out_filter = (*filter).clone();
    phase_out_filter.card_types = choose_filter.card_types.clone();
    phase_out_filter.excluded_subtypes = choose_filter.excluded_subtypes.clone();
    if choose_filter
        .card_types
        .contains(&crate::types::CardType::Enchantment)
        && choose_filter.excluded_subtypes.contains(&Subtype::Aura)
        && !phase_out_filter.excluded_subtypes.contains(&Subtype::Aura)
    {
        phase_out_filter.excluded_subtypes.push(Subtype::Aura);
    }
    phase_out_filter =
        phase_out_filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::SharesCardType);

    let mut choose_filter = choose_filter;
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            count_value: None,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ]))
}

fn parse_proliferate_then_choose_permanents_phase_out_bundle(
    first_sentence: &[OwnedLexToken],
    second_sentence: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let first_words = crate::runtime_backend::token_word_refs(first_sentence);
    let first_words = if first_words.first().copied() == Some("you") {
        &first_words[1..]
    } else {
        &first_words[..]
    };
    if first_words
        != [
            "proliferate",
            "then",
            "choose",
            "any",
            "number",
            "of",
            "permanents",
            "you",
            "control",
            "that",
            "had",
            "a",
            "counter",
            "put",
            "on",
            "them",
            "this",
            "way",
        ]
    {
        return None;
    }

    let second_words = crate::runtime_backend::token_word_refs(second_sentence);
    if second_words != ["those", "permanents", "phase", "out"] {
        return None;
    }

    let eligible_filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    let chosen_tag = TagKey::from(IT_TAG);
    let mut phase_out_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    phase_out_filter =
        phase_out_filter.match_tagged(chosen_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);

    Some(vec![
        EffectAst::subject_verb_proliferate(Value::Fixed(1)),
        EffectAst::ChooseObjects {
            filter: eligible_filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag,
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ])
}

fn parse_proliferate_then_choose_permanents_phase_out_single_sentence(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let words = if words.first().copied() == Some("you") {
        &words[1..]
    } else {
        &words[..]
    };
    if words
        != [
            "proliferate",
            "then",
            "choose",
            "any",
            "number",
            "of",
            "permanents",
            "you",
            "control",
            "that",
            "had",
            "a",
            "counter",
            "put",
            "on",
            "them",
            "this",
            "way",
            "those",
            "permanents",
            "phase",
            "out",
        ]
    {
        return None;
    }

    let eligible_filter = ObjectFilter::default()
        .in_zone(Zone::Battlefield)
        .controlled_by(PlayerFilter::You);
    let chosen_tag = TagKey::from(IT_TAG);
    let mut phase_out_filter = ObjectFilter::default().in_zone(Zone::Battlefield);
    phase_out_filter =
        phase_out_filter.match_tagged(chosen_tag.clone(), TaggedOpbjectRelation::IsTaggedObject);

    Some(vec![
        EffectAst::subject_verb_proliferate(Value::Fixed(1)),
        EffectAst::ChooseObjects {
            filter: eligible_filter,
            count: ChoiceCount::any_number(),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag,
        },
        EffectAst::subject_verb_phase_out_all(phase_out_filter),
    ])
}

fn parse_draw_create_treasure_lose_life_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let words = if clause_words.first().copied() == Some("you") {
        &clause_words[1..]
    } else {
        clause_words.as_slice()
    };
    if words
        != [
            "draw", "that", "many", "cards", "create", "that", "many", "tapped", "treasure",
            "tokens", "then", "lose", "that", "much", "life",
        ]
    {
        return None;
    }

    let amount = Value::EventValue(EventValueSpec::Amount);
    Some(vec![
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::Draw {
                count: amount.clone(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::Actor,
            PlayerAst::You,
            SubjectVerbActionAst::CreateTokenWithMods {
                name: "Treasure".to_string(),
                count: amount.clone(),
                dynamic_power_toughness: None,
                player: PlayerAst::You,
                attached_to: None,
                tapped: true,
                attacking: false,
                exile_at_end_of_combat: false,
                sacrifice_at_end_of_combat: false,
                sacrifice_at_next_end_step: false,
                exile_at_next_end_step: false,
                granted_abilities: Vec::new(),
            },
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LoseLife { amount },
        ),
    ])
}

fn looks_like_source_leaves_return_followup_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
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
        EffectAst::SubjectVerb(subject_verb) => match subject_verb.action {
            SubjectVerbActionAst::Exile { target, face_down } => Some(
                EffectAst::subject_verb_exile_until_source_leaves(target, face_down),
            ),
            SubjectVerbActionAst::ExileAll { filter, face_down } => {
                Some(EffectAst::subject_verb_exile_until_source_leaves(
                    TargetAst::Object(filter, None, None),
                    face_down,
                ))
            }
            _ => None,
        },
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
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
            zones: vec![Zone::Exile],
            search_mode: None,
        },
        EffectAst::subject_verb_reveal_tagged(chosen_tag.clone()),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(chosen_tag, span_from_tokens(second)),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];

    Ok(Some(vec![EffectAst::May { effects }]))
}

fn parse_reveal_from_outside_game_to_hand(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = trim_commas(tokens);
    let lowered = words(&tokens);
    if !lowered.iter().any(|word| *word == "outside") || !lowered.iter().any(|word| *word == "game")
    {
        return Ok(None);
    }
    let Some(reveal_idx) = lowered.iter().position(|word| *word == "reveal") else {
        return Ok(None);
    };
    let Some(from_idx) = lowered.iter().position(|word| *word == "from") else {
        return Ok(None);
    };
    if from_idx <= reveal_idx + 1 {
        return Ok(None);
    }

    let put_tail = &["and", "put", "it", "into", "your", "hand"];
    let Some(put_idx) = lowered
        .windows(put_tail.len())
        .position(|window| window == put_tail)
    else {
        return Ok(None);
    };
    if put_idx <= from_idx {
        return Ok(None);
    }

    let mut filter_tokens = trim_commas(&tokens[reveal_idx + 1..from_idx]).to_vec();
    if filter_tokens
        .windows(2)
        .position(|window| window[0].is_word("you") && window[1].is_word("own"))
        .is_none()
        && !lowered[from_idx..put_idx]
            .windows(2)
            .any(|window| window == ["you", "own"])
    {
        return Ok(None);
    }
    while filter_tokens
        .last()
        .is_some_and(|token| token.is_word("you") || token.is_word("own"))
    {
        filter_tokens.pop();
    }

    let mut filter = parse_object_filter_lexed(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported outside-game wish filter in clause '{}'",
            lowered.join(" ")
        ))
    })?;
    filter.owner = Some(PlayerFilter::You);
    filter.zone = Some(Zone::OutsideGame);

    let wish_tag = TagKey::from("searched_outside_game");
    let effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: wish_tag.clone(),
            zones: vec![Zone::OutsideGame],
            search_mode: Some(crate::effect::SearchSelectionMode::Optional),
        },
        EffectAst::subject_verb_reveal_tagged(wish_tag.clone()),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(wish_tag, span_from_tokens(&tokens)),
            Zone::Hand,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];
    let mut outer = vec![EffectAst::May { effects }];
    if lowered[put_idx + put_tail.len()..]
        .iter()
        .any(|word| *word == "exile")
    {
        outer.push(EffectAst::subject_verb_exile(
            TargetAst::Source(None),
            false,
        ));
    }

    Ok(Some(outer))
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

    let second_words = crate::runtime_backend::token_word_refs(second);
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

fn split_search_library_slot_filter_items_lexed(
    filter_tokens: &[OwnedLexToken],
) -> Option<Vec<Vec<OwnedLexToken>>> {
    let mut items = Vec::new();
    let mut item_start = 0usize;
    let mut cursor = 0usize;

    while cursor < filter_tokens.len() {
        let mut next_item_start = None;
        if filter_tokens[cursor].is_comma() || filter_tokens[cursor].is_word("and") {
            let mut probe = cursor;
            while filter_tokens
                .get(probe)
                .is_some_and(OwnedLexToken::is_comma)
            {
                probe += 1;
            }
            if filter_tokens
                .get(probe)
                .is_some_and(|token| token.is_word("and"))
            {
                probe += 1;
                while filter_tokens
                    .get(probe)
                    .is_some_and(OwnedLexToken::is_comma)
                {
                    probe += 1;
                }
            }
            if filter_tokens
                .get(probe)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
                && probe > cursor
            {
                next_item_start = Some(probe);
            }
        }

        if let Some(start) = next_item_start {
            let item = trim_commas(&filter_tokens[item_start..cursor]);
            if item.is_empty() {
                return None;
            }
            items.push(item);
            item_start = start;
            cursor = start;
            continue;
        }

        cursor += 1;
    }

    let item = trim_commas(&filter_tokens[item_start..]);
    if item.is_empty() {
        return None;
    }
    items.push(item);

    (items.len() >= 2).then_some(items)
}

fn parse_search_library_slots_to_hand_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_words = parser_words(tokens);
    let multi_zone = if sentence_words.len() >= 15
        && sentence_words[..4] == ["search", "your", "library", "for"]
    {
        false
    } else if sentence_words.len() >= 17
        && (sentence_words[..6] == ["search", "your", "library", "and", "graveyard", "for"]
            || sentence_words[..6] == ["search", "your", "library", "or", "graveyard", "for"]
            || sentence_words[..6] == ["search", "your", "library", "and/or", "graveyard", "for"])
    {
        true
    } else {
        return Ok(None);
    };

    let reveal_phrase = ["reveal", "those", "cards"];
    let reveal_them_phrase = ["reveal", "them"];
    let put_them_phrase = ["put", "them", "into", "your", "hand", "then", "shuffle"];
    let put_those_cards_phrase = [
        "put", "those", "cards", "into", "your", "hand", "then", "shuffle",
    ];
    let reveal_match = sentence_words
        .windows(reveal_phrase.len())
        .position(|window| window == reveal_phrase)
        .map(|idx| (idx, reveal_phrase.len()))
        .or_else(|| {
            sentence_words
                .windows(reveal_them_phrase.len())
                .position(|window| window == reveal_them_phrase)
                .map(|idx| (idx, reveal_them_phrase.len()))
        });
    let Some((reveal_word_idx, reveal_word_len)) = reveal_match else {
        return Ok(None);
    };
    let tail_words = &sentence_words[reveal_word_idx + reveal_word_len..];
    if tail_words != put_them_phrase && tail_words != put_those_cards_phrase {
        return Ok(None);
    }

    let Some(for_idx) = find_index(tokens, |token: &OwnedLexToken| token.is_word("for")) else {
        return Ok(None);
    };
    let Some(reveal_idx) = find_index(tokens, |token: &OwnedLexToken| token.is_word("reveal"))
    else {
        return Ok(None);
    };
    if reveal_idx <= for_idx + 1 {
        return Ok(None);
    }

    let filter_items = split_search_library_slot_filter_items_lexed(&trim_commas(
        &tokens[for_idx + 1..reveal_idx],
    ))
    .ok_or_else(|| {
        CardTextError::ParseError(
            "expected multiple slot filters in search-library hand bundle".to_string(),
        )
    })?;

    let mut slots = Vec::new();
    for item in filter_items {
        let mut filter = parse_object_filter_lexed(&item, false)?;
        filter.zone = if multi_zone {
            None
        } else {
            Some(Zone::Library)
        };
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        slots.push(crate::cards::builders::SearchLibrarySlotAst {
            filter,
            optional: true,
        });
    }

    Ok(Some(vec![
        EffectAst::subject_verb_search_library_slots_to_hand(
            PlayerAst::You,
            slots,
            true,
            TagKey::from("search_library_slots_progress"),
        ),
    ]))
}

fn search_library_slots_to_hand_effect_from_items(
    filter_items: Vec<Vec<OwnedLexToken>>,
) -> Result<EffectAst, CardTextError> {
    let mut slots = Vec::new();
    for item in filter_items {
        let mut filter = parse_object_filter_lexed(&item, false)?;
        filter.zone = Some(Zone::Library);
        if filter.owner.is_none() {
            filter.owner = Some(PlayerFilter::You);
        }
        slots.push(crate::cards::builders::SearchLibrarySlotAst {
            filter,
            optional: true,
        });
    }

    Ok(EffectAst::subject_verb_search_library_slots_to_hand(
        PlayerAst::You,
        slots,
        true,
        TagKey::from("search_library_slots_progress"),
    ))
}

fn parse_kicked_search_library_slots_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() != 3 {
        return Ok(None);
    }

    let first_words = parser_words(sentences[0]);
    let second_words = parser_words(sentences[1]);
    let third_words = parser_words(sentences[2]);
    if first_words.as_slice()
        != [
            "search", "your", "library", "for", "a", "basic", "land", "card",
        ]
    {
        return Ok(None);
    }
    if !second_words.iter().take(10).map(String::as_str).eq([
        "if", "this", "spell", "was", "kicked", "instead", "search", "your", "library", "for",
    ]) {
        return Ok(None);
    }
    if third_words.as_slice()
        != [
            "reveal", "those", "cards", "put", "them", "into", "your", "hand", "then", "shuffle",
        ]
    {
        return Ok(None);
    }

    let Some(first_for_idx) = find_index(sentences[0], |token| token.is_word("for")) else {
        return Ok(None);
    };
    let Some(second_for_idx) = find_index(sentences[1], |token| token.is_word("for")) else {
        return Ok(None);
    };
    let default_item = trim_commas(&sentences[0][first_for_idx + 1..]);
    let replacement_items = split_search_library_slot_filter_items_lexed(&trim_commas(
        &sentences[1][second_for_idx + 1..],
    ))
    .ok_or_else(|| {
        CardTextError::ParseError(
            "expected replacement search-library slot filters after kicked instead clause"
                .to_string(),
        )
    })?;

    Ok(Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: vec![search_library_slots_to_hand_effect_from_items(
            replacement_items,
        )?],
        if_false: vec![search_library_slots_to_hand_effect_from_items(vec![
            default_item,
        ])?],
    }]))
}

fn search_library_and_graveyard_doctors_effects(destination: Zone) -> Vec<EffectAst> {
    let searched_tag = TagKey::from("searched_multi_zone");
    let mut filter = ObjectFilter::default();
    filter.owner = Some(PlayerFilter::You);
    filter.subtypes = vec![Subtype::Doctor];

    vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::up_to(5),
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
            zones: vec![Zone::Library, Zone::Graveyard],
            search_mode: Some(crate::effect::SearchSelectionMode::Optional),
        },
        EffectAst::subject_verb_reveal_tagged(searched_tag.clone()),
        EffectAst::ForEachTagged {
            tag: searched_tag.clone(),
            effects: vec![EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(searched_tag, None),
                destination,
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
    ]
}

fn parse_kicked_multi_zone_search_to_battlefield_replacement_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() != 3 {
        return None;
    }

    let first_words = parser_words(sentences[0]);
    let second_words = parser_words(sentences[1]);
    let third_words = parser_words(sentences[2]);

    if first_words.as_slice()
        != [
            "search",
            "your",
            "library",
            "and/or",
            "graveyard",
            "for",
            "up",
            "to",
            "five",
            "doctor",
            "cards",
            "reveal",
            "them",
            "and",
            "put",
            "them",
            "into",
            "your",
            "hand",
        ]
    {
        return None;
    }
    if second_words.as_slice()
        != [
            "if", "you", "search", "your", "library", "this", "way", "shuffle",
        ]
    {
        return None;
    }
    if third_words.as_slice()
        != [
            "if",
            "this",
            "spell",
            "was",
            "kicked",
            "put",
            "those",
            "cards",
            "onto",
            "the",
            "battlefield",
            "instead",
            "of",
            "putting",
            "them",
            "into",
            "your",
            "hand",
        ]
    {
        return None;
    }

    Some(vec![EffectAst::SelfReplacement {
        predicate: PredicateAst::ThisSpellWasKicked,
        if_true: search_library_and_graveyard_doctors_effects(Zone::Battlefield),
        if_false: search_library_and_graveyard_doctors_effects(Zone::Hand),
    }])
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
    effects.push(EffectAst::subject_verb_grant_by_spec(
        crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            crate::filter::ObjectFilter::tagged(crate::cards::builders::TagKey::from(IT_TAG)),
            Zone::Exile,
        ),
        crate::cards::builders::PlayerAst::ItsOwner,
        crate::grant::GrantDuration::Forever,
    ));
    effects.push(EffectAst::subject_verb_grant_to_target(
        crate::cards::builders::TargetAst::Tagged(
            crate::cards::builders::TagKey::from(IT_TAG),
            None,
        ),
        crate::grant::Grantable::Ability(crate::static_abilities::StaticAbility::new(
            crate::static_abilities::CostIncreaseManaCost::new(
                crate::filter::ObjectFilter::spell()
                    .without_type(crate::types::CardType::Land)
                    .cast_by(crate::PlayerFilter::Opponent),
                crate::mana::ManaCost::from_symbols(vec![crate::mana::ManaSymbol::Generic(2)]),
            ),
        )),
        crate::grant::GrantDuration::Forever,
    ));
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
        EffectAst::subject_verb_sacrifice_all(PlayerAst::You, ObjectFilter::tagged(sacrificed_tag)),
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::You,
            crate::cards::builders::LibraryConsultModeAst::Reveal,
            zombie_creature_card,
            crate::cards::builders::LibraryConsultStopRuleAst::MatchCount(
                crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount),
            ),
            revealed_tag.clone(),
            matched_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(matched_tag.clone(), None),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
            revealed_tag,
            Some(matched_tag),
            crate::cards::builders::LibraryBottomOrderAst::Random,
            PlayerAst::You,
        ),
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
        EffectAst::subject_verb_sacrifice(
            PlayerAst::ItsController,
            ObjectFilter::default(),
            1,
            Some(target),
        ),
        EffectAst::subject_verb_consult_top_of_library(
            PlayerAst::That,
            crate::cards::builders::LibraryConsultModeAst::Reveal,
            artifact_card,
            crate::cards::builders::LibraryConsultStopRuleAst::FirstMatch,
            revealed_tag,
            matched_tag.clone(),
        ),
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(matched_tag, None),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
        EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::That,
            SubjectVerbActionAst::ShuffleLibrary,
        ),
    ])
}

fn parse_reveal_until_land_put_all_graveyard_bundle(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    let sentence_words = parser_words(tokens);
    let sentence_word_refs = sentence_words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let (player, target_effect, consumed) = match sentence_word_refs.as_slice() {
        [
            "target",
            "player",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            ..,
        ] => (
            PlayerAst::Target,
            Some(EffectAst::subject_verb_target_only(TargetAst::Player(
                PlayerFilter::Any,
                span_from_tokens(tokens),
            ))),
            2,
        ),
        [
            "target",
            "opponent",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            ..,
        ] => (
            PlayerAst::TargetOpponent,
            Some(EffectAst::subject_verb_target_only(TargetAst::Player(
                PlayerFilter::Opponent,
                span_from_tokens(tokens),
            ))),
            2,
        ),
        [
            "that",
            "player",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            ..,
        ] => (PlayerAst::That, None, 2),
        [
            "defending",
            "player",
            "reveals",
            "cards",
            "from",
            "the",
            "top",
            "of",
            "their",
            "library",
            ..,
        ] => (PlayerAst::Defending, None, 2),
        _ => return None,
    };

    let tail = &sentence_word_refs[consumed..];
    if tail
        != [
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
            "land",
            "card",
            "then",
            "puts",
            "those",
            "cards",
            "into",
            "their",
            "graveyard",
        ]
    {
        return None;
    }

    let revealed_tag = TagKey::from("reveal_until_land_revealed");
    let matched_tag = TagKey::from("reveal_until_land_matched");
    let mut land_card = ObjectFilter::default();
    land_card.card_types.push(CardType::Land);
    land_card.zone = None;

    let mut effects = Vec::new();
    if let Some(target_effect) = target_effect {
        effects.push(target_effect);
    }
    effects.push(EffectAst::subject_verb_consult_top_of_library(
        player,
        LibraryConsultModeAst::Reveal,
        land_card,
        LibraryConsultStopRuleAst::FirstMatch,
        revealed_tag.clone(),
        matched_tag,
    ));
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(revealed_tag, None),
        Zone::Graveyard,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    Some(effects)
}

fn parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
    consult_sentence: &[OwnedLexToken],
    followup_sentence: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(parts) = super::consult_family::parse_consult_traversal_sentence(consult_sentence)?
    else {
        return Ok(None);
    };
    let Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ConsultTopOfLibrary {
                mode: LibraryConsultModeAst::Reveal,
                ..
            },
        ..
    })) = parts.effects.last()
    else {
        return Ok(None);
    };

    let followup_words = crate::runtime_backend::token_word_refs(followup_sentence);
    if !followup_words.starts_with(&["put", "those"])
        || !followup_words.contains(&"battlefield")
        || !followup_words.contains(&"rest")
        || !followup_words.contains(&"bottom")
        || !followup_words.contains(&"library")
    {
        return Ok(None);
    }
    let Some(order) = super::consult_family::parse_consult_remainder_order(&followup_words) else {
        return Ok(None);
    };

    let enters_tapped = followup_words.contains(&"tapped");
    let mut effects = parts.effects;
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(parts.match_tag.clone(), None),
        Zone::Battlefield,
        false,
        ReturnControllerAst::Preserve,
        enters_tapped,
        None,
    ));
    effects.push(EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
        parts.all_tag,
        Some(parts.match_tag),
        order,
        parts.player,
    ));

    Ok(Some(effects))
}

fn parse_tap_lands_then_empty_mana_pool_bundle(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let sentence_words = parser_words(tokens);
    let sentence_word_refs = sentence_words
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    if sentence_word_refs.as_slice()
        != [
            "tap", "all", "lands", "target", "player", "controls", "and", "that", "player",
            "loses", "all", "unspent", "mana",
        ]
    {
        return None;
    }

    let mut lands = ObjectFilter::default();
    lands.zone = Some(Zone::Battlefield);
    lands.controller = Some(PlayerFilter::target_player());
    lands.card_types.push(CardType::Land);
    Some(vec![
        EffectAst::subject_verb_target_only(TargetAst::Player(
            PlayerFilter::Any,
            span_from_tokens(tokens),
        )),
        EffectAst::subject_verb_tap_all(lands),
        EffectAst::subject_verb_empty_mana_pool(PlayerAst::That),
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
            EffectAst::subject_verb_tag_matching_objects(
                owned_creatures.clone(),
                vec![Zone::Battlefield],
                tagged_creatures.clone(),
            ),
            EffectAst::subject_verb_tag_matching_objects(
                owned_nontoken_creatures,
                vec![Zone::Battlefield],
                tagged_nontoken.clone(),
            ),
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(tagged_creatures, None),
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::That,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
            EffectAst::Conditional {
                predicate: PredicateAst::PlayerTaggedObjectMatches {
                    player: PlayerAst::That,
                    tag: tagged_nontoken,
                    filter: tagged_library_filter,
                },
                if_true: vec![
                    EffectAst::subject_verb_consult_top_of_library(
                        PlayerAst::That,
                        LibraryConsultModeAst::Reveal,
                        creature_card,
                        LibraryConsultStopRuleAst::FirstMatch,
                        revealed_tag.clone(),
                        matched_tag.clone(),
                    ),
                    EffectAst::subject_verb_move_to_zone(
                        TargetAst::Tagged(matched_tag.clone(), None),
                        Zone::Battlefield,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                        revealed_tag,
                        Some(matched_tag),
                        LibraryBottomOrderAst::Random,
                        PlayerAst::That,
                    ),
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
            count_value: None,
            player: PlayerAst::You,
            tag: searched_tag.clone(),
            zones: zones.clone(),
            search_mode: Some(crate::effect::SearchSelectionMode::Exact),
        });
    }
    effects.push(EffectAst::subject_verb_reveal_tagged(searched_tag.clone()));
    effects.push(EffectAst::subject_verb_move_to_zone(
        TargetAst::Tagged(searched_tag, None),
        Zone::Hand,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    ));
    effects.push(EffectAst::subject_verb(
        SubjectVerbRoleAst::LibraryOwner,
        PlayerAst::You,
        SubjectVerbActionAst::ShuffleLibrary,
    ));
    Some(effects)
}

pub(crate) fn parse_exact_card_effect_bundle_lexed(
    tokens: &[OwnedLexToken],
) -> Option<Vec<EffectAst>> {
    if let Ok(Some(effects)) = parse_reveal_from_outside_game_to_hand(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_tap_lands_then_empty_mana_pool_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_soul_partition_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_empty_laboratory_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_shape_anew_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_reveal_until_land_put_all_graveyard_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_collision_of_realms_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_nissas_encouragement_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_draw_create_treasure_lose_life_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) =
        parse_proliferate_then_choose_permanents_phase_out_single_sentence(tokens)
    {
        return Some(effects);
    }
    let sentences = split_lexed_sentences(tokens);
    if sentences.len() == 2
        && let Ok(Some(effects)) = parse_consult_then_put_matches_battlefield_rest_bottom_bundle(
            sentences[0],
            sentences[1],
        )
    {
        return Some(effects);
    }
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
        && let Ok(Some(effects)) =
            parse_choose_type_then_phase_out_bundle(sentences[0], sentences[1])
    {
        return Some(effects);
    }
    if sentences.len() == 2
        && let Some(effects) =
            parse_proliferate_then_choose_permanents_phase_out_bundle(sentences[0], sentences[1])
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
            let first_words = crate::runtime_backend::token_word_refs(sentences[0]);
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
        let first_words = crate::runtime_backend::token_word_refs(sentences[0]);
        let choice_words = if first_words.first().copied() == Some("you") {
            &first_words[1..]
        } else {
            &first_words[..]
        };
        let (_, options) = parse_choose_card_type_phrase_words(choice_words)
            .ok()
            .flatten()
            .expect("validated choose-card-type bundle prefix");
        let mut combined = vec![EffectAst::subject_verb_choose_card_type(
            PlayerAst::You,
            options,
        )];
        combined.append(&mut effects);
        return Some(combined);
    }
    if let Ok(Some(effects)) = parse_kicked_search_library_slots_replacement_bundle(tokens) {
        return Some(effects);
    }
    if let Some(effects) = parse_kicked_multi_zone_search_to_battlefield_replacement_bundle(tokens)
    {
        return Some(effects);
    }
    if let Ok(Some(effects)) = parse_search_library_slots_to_hand_bundle(tokens) {
        return Some(effects);
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
            EffectAst::subject_verb_look_at_top_cards(
                PlayerAst::You,
                Value::Devotion {
                    player: PlayerFilter::You,
                    color: crate::color::Color::Blue,
                },
                looked_tag.clone(),
            ),
            EffectAst::subject_verb_rearrange_looked_cards_in_library(
                PlayerAst::You,
                looked_tag,
                ChoiceCount::up_to(1),
            ),
            EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::ValueComparison {
                    left: Value::Devotion {
                        player: PlayerFilter::You,
                        color: crate::color::Color::Blue,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::CardsInLibrary(PlayerFilter::You),
                },
                if_true: vec![EffectAst::subject_verb_win_game(PlayerAst::You)],
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
            if_true: vec![EffectAst::subject_verb_copy_spell(
                TargetAst::Source(None),
                Value::Fixed(1),
                PlayerAst::Implicit,
                true,
                Vec::new(),
            )],
            if_false: Vec::new(),
        }]);
    }

    None
}

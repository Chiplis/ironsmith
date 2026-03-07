use crate::cards::builders::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut, try_for_each_nested_effects_mut,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, CarryContext, EffectAst, IT_TAG, IfResultPredicate, PlayerAst, TagKey,
    TargetAst, TextSpan, Token, TokenCopyFollowup, append_token_reminder_to_last_create_effect,
    build_may_cast_tagged_effect, collapse_token_copy_end_of_combat_exile_followup,
    collapse_token_copy_next_end_step_exile_followup, effect_creates_any_token,
    effect_creates_eldrazi_spawn_or_scion, explicit_player_for_carry,
    is_activate_only_restriction_sentence, is_article, is_exile_that_token_at_end_of_combat,
    is_generic_token_reminder_sentence, is_round_up_each_time_sentence,
    is_sacrifice_that_token_at_end_of_combat, is_simple_copy_reference_sentence,
    is_spawn_scion_token_mana_reminder, is_trigger_only_restriction_sentence,
    maybe_apply_carried_player, maybe_apply_carried_player_with_clause, normalize_cant_words,
    normalize_search_library_filter, parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
    parse_choose_creature_type_then_become_type, parse_choose_target_prelude_sentence,
    parse_effect_chain, parse_effect_clause_with_trailing_if, parse_effect_sentence, parse_number,
    parse_may_cast_it_sentence, parse_object_filter, parse_search_library_disjunction_filter,
    parse_sentence_exile_that_token_when_source_leaves,
    parse_sentence_sacrifice_source_when_that_token_leaves,
    parse_target_player_chooses_then_other_cant_block, parse_token_copy_modifier_sentence,
    parse_where_x_value_clause, parser_trace, replace_unbound_x_with_value, span_from_tokens,
    split_on_period, strip_embedded_token_rules_text, target_ast_to_object_filter,
    token_index_for_word_index, trim_commas, value_contains_unbound_x, words,
};
use crate::effect::{ChoiceCount, Until, Value};
use crate::static_abilities::StaticAbility;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::zone::Zone;

type PairSentenceRule = fn(&[Token], &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError>;
type TripleSentenceRule =
    fn(&[Token], &[Token], &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError>;
type QuadSentenceRule =
    fn(&[Token], &[Token], &[Token], &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError>;

fn parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard(
    first: &[Token],
    second: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = words(&first_tokens);
    let count_word_idx = if first_words.starts_with(&["reveal", "the", "top"]) {
        3usize
    } else if first_words.starts_with(&["reveal", "top"]) {
        2usize
    } else {
        return Ok(None);
    };

    let count_tokens = first_words[count_word_idx..]
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_number(&count_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing reveal count in reveal-top matching split clause (clause: '{}')",
            first_words.join(" ")
        ))
    })?;
    if count_tokens
        .get(used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Ok(None);
    }
    let reveal_tail = words(&count_tokens[used + 1..]);
    if reveal_tail != ["of", "your", "library"] {
        return Ok(None);
    }

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    if !matches!(second_words.get(..2), Some(["put", "all"] | ["puts", "all"])) {
        return Ok(None);
    }
    let Some(revealed_idx) = second_words
        .windows(3)
        .position(|window| window == ["revealed", "this", "way"])
    else {
        return Ok(None);
    };
    if revealed_idx <= 2 {
        return Ok(None);
    }

    let Some(filter_start) = token_index_for_word_index(&second_tokens, 2) else {
        return Ok(None);
    };
    let filter_end =
        token_index_for_word_index(&second_tokens, revealed_idx).unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_revealed = &second_words[revealed_idx + 3..];
    let has_hand_clause = after_revealed.windows(3).any(|window| window == ["into", "your", "hand"]);
    let has_rest_clause = after_revealed.windows(5).any(|window| {
        window == ["and", "the", "rest", "into", "your"]
    }) && after_revealed.contains(&"graveyard");
    if !has_hand_clause || !has_rest_clause {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::RevealTopPutMatchingIntoHandRestIntoGraveyard {
        player: PlayerAst::You,
        count,
        filter,
    }]))
}

fn parse_delayed_dies_exile_top_power_choose_play(
    first: &[Token],
    second: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_tokens = trim_commas(first);
    let first_words = words(&first_tokens);
    if !first_words.starts_with(&["when", "that", "creature", "dies", "this", "turn"]) {
        return Ok(None);
    }

    let Some(comma_idx) = first_tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    let action_tokens = trim_commas(&first_tokens[comma_idx + 1..]);
    let action_words: Vec<&str> = words(&action_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let starts_with_exile_top_power = action_words.starts_with(&[
        "exile", "number", "of", "cards", "from", "top", "of", "your", "library", "equal", "to",
        "its", "power",
    ]);
    let ends_with_choose_exiled = action_words.ends_with(&["choose", "card", "exiled", "this", "way"]);
    if !starts_with_exile_top_power || !ends_with_choose_exiled {
        return Ok(None);
    }

    let second_words: Vec<&str> = words(second)
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

    let looked_tag = TagKey::from("looked_0");
    let chosen_tag = TagKey::from("chosen_0");
    let mut exiled_filter = ObjectFilter::default();
    exiled_filter.zone = Some(Zone::Exile);
    exiled_filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: looked_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: None,
        effects: vec![
            EffectAst::LookAtTopCards {
                player: PlayerAst::You,
                count: Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG)))),
                tag: looked_tag.clone(),
            },
            EffectAst::Exile {
                target: TargetAst::Tagged(looked_tag, None),
                face_down: false,
            },
            EffectAst::ChooseObjects {
                filter: exiled_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_tag.clone(),
            },
            EffectAst::GrantPlayTaggedUntilYourNextTurn {
                tag: chosen_tag,
                player: PlayerAst::You,
            },
        ],
    }]))
}

fn parse_pair_sentence_sequence(
    first: &[Token],
    second: &[Token],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, PairSentenceRule); 5] = [
        (
            "delayed-dies-exile-top-power-choose-play",
            parse_delayed_dies_exile_top_power_choose_play,
        ),
        (
            "target-chooses-other-cant-block",
            parse_target_player_chooses_then_other_cant_block,
        ),
        (
            "choose-card-type-then-reveal-and-put",
            parse_choose_card_type_then_reveal_top_and_put_chosen_to_hand,
        ),
        (
            "choose-creature-type-then-become-type",
            parse_choose_creature_type_then_become_type,
        ),
        (
            "reveal-top-matching-into-hand-rest-graveyard",
            parse_reveal_top_count_put_all_matching_into_hand_rest_graveyard,
        ),
    ];

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_look_at_top_reveal_match_put_rest_bottom(
    first: &[Token],
    second: &[Token],
    third: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let first_effects = parse_effect_sentence(first)?;
    let [EffectAst::LookAtTopCards { player, count, .. }] = first_effects.as_slice() else {
        return Ok(None);
    };

    let second_tokens = trim_commas(second);
    let second_words = words(&second_tokens);
    if second_words.is_empty() {
        return Ok(None);
    }

    let (chooser, reveal_word_idx) = if second_words.starts_with(&["you", "may", "reveal"]) {
        (PlayerAst::You, 2usize)
    } else if second_words.starts_with(&["that", "player", "may", "reveal"]) {
        (PlayerAst::That, 3usize)
    } else if second_words.starts_with(&["they", "may", "reveal"]) {
        (PlayerAst::That, 2usize)
    } else if second_words.starts_with(&["may", "reveal"]) {
        (*player, 1usize)
    } else if second_words.starts_with(&["reveal"]) {
        (*player, 0usize)
    } else {
        return Ok(None);
    };

    let from_among_word_idx = second_words
        .windows(3)
        .position(|window| window == ["from", "among", "them"])
        .or_else(|| {
            second_words
                .windows(4)
                .position(|window| window == ["from", "among", "those", "cards"])
        });
    let Some(from_among_word_idx) = from_among_word_idx else {
        return Ok(None);
    };
    if from_among_word_idx <= reveal_word_idx {
        return Ok(None);
    }

    let filter_start = token_index_for_word_index(&second_tokens, reveal_word_idx + 1)
        .unwrap_or(second_tokens.len());
    let filter_end = token_index_for_word_index(&second_tokens, from_among_word_idx)
        .unwrap_or(second_tokens.len());
    let filter_tokens = trim_commas(&second_tokens[filter_start..filter_end]);
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = if let Some(filter) = parse_looked_card_reveal_filter(&filter_tokens) {
        filter
    } else {
        return Ok(None);
    };
    normalize_search_library_filter(&mut filter);
    filter.zone = None;

    let after_from_word_idx = if second_words
        .windows(4)
        .any(|window| window == ["from", "among", "those", "cards"])
    {
        from_among_word_idx + 4
    } else {
        from_among_word_idx + 3
    };
    let after_from_words = &second_words[after_from_word_idx..];
    let puts_into_hand = (after_from_words.starts_with(&["and", "put", "it", "into"])
        || after_from_words.starts_with(&["put", "it", "into"]))
        && after_from_words.contains(&"hand");
    if !puts_into_hand {
        return Ok(None);
    }

    let third_words = words(third);
    let puts_rest_bottom = matches!(third_words.first().copied(), Some("put" | "puts"))
        && third_words.contains(&"rest")
        && third_words.contains(&"bottom")
        && third_words.contains(&"library");
    if !puts_rest_bottom {
        return Ok(None);
    }

    let mut effects = vec![EffectAst::LookAtTopCards {
        player: *player,
        count: count.clone(),
        tag: TagKey::from(IT_TAG),
    }];
    effects.push(
        EffectAst::ChooseFromLookedCardsIntoHandRestOnBottomOfLibrary {
            player: chooser,
            filter,
            reveal: true,
            if_not_chosen: Vec::new(),
        },
    );
    Ok(Some(effects))
}

fn title_case_words(words: &[&str]) -> String {
    words
        .iter()
        .map(|word| {
            let mut chars = word.chars();
            let Some(first) = chars.next() else {
                return String::new();
            };
            let mut titled = String::new();
            titled.extend(first.to_uppercase());
            titled.push_str(chars.as_str());
            titled
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn parse_named_card_filter_segment(tokens: &[Token]) -> Option<ObjectFilter> {
    let mut segment_words = words(tokens);
    while segment_words.first().is_some_and(|word| is_article(word)) {
        segment_words.remove(0);
    }
    if matches!(segment_words.last().copied(), Some("card" | "cards")) {
        segment_words.pop();
    }
    if segment_words.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.name = Some(title_case_words(&segment_words));
    Some(filter)
}

fn split_reveal_filter_segments(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();
    for token in tokens {
        if token.is_word("or") || matches!(token, Token::Comma(_)) {
            let trimmed = trim_commas(&current);
            if !trimmed.is_empty() {
                segments.push(trimmed.to_vec());
            }
            current.clear();
            continue;
        }
        current.push(token.clone());
    }
    let trimmed = trim_commas(&current);
    if !trimmed.is_empty() {
        segments.push(trimmed.to_vec());
    }
    segments
}

fn parse_looked_card_reveal_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let words_all = words(tokens);
    if words_all.contains(&"or") {
        let shared_card_suffix = matches!(words_all.last().copied(), Some("card" | "cards"));
        let segments = split_reveal_filter_segments(tokens);
        if segments.len() >= 2 {
            let mut branches = Vec::new();
            for mut segment in segments {
                if shared_card_suffix
                    && !matches!(
                        segment.last().and_then(Token::as_word),
                        Some("card" | "cards")
                    )
                {
                    segment.push(Token::Word("card".to_string(), TextSpan::synthetic()));
                }
                let parsed = parse_object_filter(&segment, false)
                    .ok()
                    .filter(|filter| *filter != ObjectFilter::default())
                    .or_else(|| parse_named_card_filter_segment(&segment));
                let Some(parsed) = parsed else {
                    return None;
                };
                branches.push(parsed);
            }
            let mut filter = ObjectFilter::default();
            filter.any_of = branches;
            return Some(filter);
        }
    }

    parse_search_library_disjunction_filter(tokens)
        .or_else(|| parse_object_filter(tokens, false).ok())
}

fn parse_triple_sentence_sequence(
    first: &[Token],
    second: &[Token],
    third: &[Token],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, TripleSentenceRule); 1] = [(
        "look-at-top-reveal-match-put-rest-bottom",
        parse_look_at_top_reveal_match_put_rest_bottom,
    )];

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second, third)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

fn parse_if_no_card_into_hand_this_way_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let has_expected_prefix = words.starts_with(&[
        "if", "you", "didnt", "put", "card", "into", "your", "hand", "this", "way",
    ]) || words.starts_with(&[
        "if", "you", "did", "not", "put", "card", "into", "your", "hand", "this", "way",
    ]);
    if !has_expected_prefix {
        return Ok(None);
    }

    let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    else {
        return Ok(None);
    };
    if comma_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
    if effects.is_empty() {
        return Ok(None);
    }
    Ok(Some(effects))
}

fn parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand(
    first: &[Token],
    second: &[Token],
    third: &[Token],
    fourth: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(mut effects) = parse_look_at_top_reveal_match_put_rest_bottom(first, second, third)?
    else {
        return Ok(None);
    };
    let Some(if_not_chosen) = parse_if_no_card_into_hand_this_way_sentence(fourth)? else {
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

fn parse_quad_sentence_sequence(
    first: &[Token],
    second: &[Token],
    third: &[Token],
    fourth: &[Token],
) -> Result<Option<(&'static str, Vec<EffectAst>)>, CardTextError> {
    const RULES: [(&str, QuadSentenceRule); 1] = [(
        "look-at-top-reveal-match-put-rest-bottom-if-not-into-hand",
        parse_look_at_top_reveal_match_put_rest_bottom_then_if_not_into_hand,
    )];

    for (name, rule) in RULES {
        if let Some(combined) = rule(first, second, third, fourth)? {
            return Ok(Some((name, combined)));
        }
    }

    Ok(None)
}

pub(crate) fn parse_effect_sentences(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::new();
    let sentences = split_on_period(tokens);
    let mut sentence_idx = 0usize;
    let mut carried_context: Option<CarryContext> = None;

    fn effect_contains_search_library(effect: &EffectAst) -> bool {
        if matches!(effect, EffectAst::SearchLibrary { .. }) {
            return true;
        }

        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            if !found {
                found = nested.iter().any(effect_contains_search_library);
            }
        });
        found
    }

    fn is_if_you_search_library_this_way_shuffle_sentence(tokens: &[Token]) -> bool {
        let words: Vec<&str> = words(tokens)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        // "If you search your library this way, shuffle."
        words.as_slice()
            == [
                "if", "you", "search", "your", "library", "this", "way", "shuffle",
            ]
            || words.as_slice()
                == [
                    "if", "you", "search", "your", "library", "this", "way", "shuffles",
                ]
    }

    while sentence_idx < sentences.len() {
        let sentence = &sentences[sentence_idx];
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }

        if sentence_idx + 3 < sentences.len()
            && let Some((rule_name, mut combined)) = parse_quad_sentence_sequence(
                sentence,
                &sentences[sentence_idx + 1],
                &sentences[sentence_idx + 2],
                &sentences[sentence_idx + 3],
            )?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 4;
            continue;
        }

        if sentence_idx + 2 < sentences.len()
            && let Some((rule_name, mut combined)) = parse_triple_sentence_sequence(
                sentence,
                &sentences[sentence_idx + 1],
                &sentences[sentence_idx + 2],
            )?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 3;
            continue;
        }

        if sentence_idx + 1 < sentences.len()
            && let Some((rule_name, mut combined)) =
                parse_pair_sentence_sequence(sentence, &sentences[sentence_idx + 1])?
        {
            let stage = format!("parse_effect_sentences:sequence-hit:{rule_name}");
            parser_trace(stage.as_str(), sentence);
            effects.append(&mut combined);
            sentence_idx += 2;
            continue;
        }
        let mut sentence_tokens = strip_embedded_token_rules_text(sentence);
        if sentence_tokens.is_empty() {
            sentence_idx += 1;
            continue;
        }
        sentence_tokens = rewrite_when_you_do_clause_prefix(&sentence_tokens);

        // Oracle frequently splits shuffle followups as a standalone sentence:
        // "If you search your library this way, shuffle." This clause is redundant when the
        // preceding sentence already compiles a library-search effect that shuffles.
        if is_if_you_search_library_this_way_shuffle_sentence(&sentence_tokens)
            && effects.iter().any(effect_contains_search_library)
        {
            parser_trace(
                "parse_effect_sentences:skip:if-you-search-library-this-way-shuffle",
                &sentence_tokens,
            );
            sentence_idx += 1;
            continue;
        }

        let sentence_words = words(&sentence_tokens);
        let is_still_lands_followup = matches!(
            sentence_words.as_slice(),
            ["theyre", "still", "land"]
                | ["theyre", "still", "lands"]
                | ["its", "still", "a", "land"]
                | ["its", "still", "land"]
        );
        if is_still_lands_followup
            && effects
                .last()
                .is_some_and(|effect| matches!(effect, EffectAst::BecomeBasePtCreature { .. }))
        {
            parser_trace(
                "parse_effect_sentences:skip:still-lands-followup",
                &sentence_tokens,
            );
            sentence_idx += 1;
            continue;
        }

        let mut wraps_as_if_did_not = false;
        if let Some(without_otherwise) = strip_otherwise_sentence_prefix(&sentence_tokens) {
            sentence_tokens = rewrite_otherwise_referential_subject(without_otherwise);
            wraps_as_if_did_not = true;
        }
        parser_trace("parse_effect_sentences:sentence", &sentence_tokens);

        // "Destroy ... . It/They can't be regenerated." followups.
        if is_cant_be_regenerated_followup_sentence(&sentence_tokens) {
            if apply_cant_be_regenerated_to_last_destroy_effect(&mut effects) {
                parser_trace(
                    "parse_effect_sentences:cant-be-regenerated-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            if is_cant_be_regenerated_this_turn_followup_sentence(&sentence_tokens)
                && apply_cant_be_regenerated_to_last_target_effect(&mut effects)
            {
                parser_trace(
                    "parse_effect_sentences:cant-be-regenerated-this-turn-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone cant-be-regenerated clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }

        if sentence_idx + 1 < sentences.len() && is_simple_copy_reference_sentence(&sentence_tokens)
        {
            let next_tokens = strip_embedded_token_rules_text(&sentences[sentence_idx + 1]);
            if let Some(spec) = parse_may_cast_it_sentence(&next_tokens)
                && spec.as_copy
            {
                parser_trace(
                    "parse_effect_sentences:copy-reference-next-may-cast-copy",
                    &sentence_tokens,
                );
                effects.push(build_may_cast_tagged_effect(&spec));
                sentence_idx += 2;
                continue;
            }
        }

        if let Some(spec) = parse_may_cast_it_sentence(&sentence_tokens) {
            parser_trace(
                "parse_effect_sentences:may-cast-it-sentence",
                &sentence_tokens,
            );
            effects.push(build_may_cast_tagged_effect(&spec));
            sentence_idx += 1;
            continue;
        }

        if is_spawn_scion_token_mana_reminder(&sentence_tokens) {
            if effects
                .last()
                .is_some_and(effect_creates_eldrazi_spawn_or_scion)
            {
                parser_trace(
                    "parse_effect_sentences:spawn-scion-reminder",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token mana reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        if let Some(effect) =
            parse_sentence_exile_that_token_when_source_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-exile-when-source-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if let Some(effect) =
            parse_sentence_sacrifice_source_when_that_token_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-sacrifice-source-when-token-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if is_generic_token_reminder_sentence(&sentence_tokens)
            && effects.last().is_some_and(effect_creates_any_token)
        {
            if append_token_reminder_to_last_create_effect(&mut effects, &sentence_tokens) {
                parser_trace(
                    "parse_effect_sentences:token-reminder-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }

        if let Some(effect) = parse_choose_target_prelude_sentence(&sentence_tokens)? {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        let mut sentence_effects =
            if let Some(followup) = parse_token_copy_followup_sentence(&sentence_tokens) {
                if try_apply_token_copy_followup(&mut effects, followup)? {
                    parser_trace(
                        "parse_effect_sentences:token-copy-followup",
                        &sentence_tokens,
                    );
                    sentence_idx += 1;
                    continue;
                }
                apply_unapplied_token_copy_followup(sentence, &sentence_tokens, followup)?
            } else {
                parse_effect_sentence(&sentence_tokens)?
            };
        if wraps_as_if_did_not {
            sentence_effects = vec![EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        collapse_token_copy_next_end_step_exile_followup(&mut sentence_effects, &sentence_tokens);
        collapse_token_copy_end_of_combat_exile_followup(&mut sentence_effects, &sentence_tokens);
        if is_that_turn_end_step_sentence(&sentence_tokens)
            && let Some(extra_turn_player) = most_recent_extra_turn_player(&effects)
            && !sentence_effects.is_empty()
        {
            sentence_effects = vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: extra_turn_player,
                effects: sentence_effects,
            }];
        }
        if words(&sentence_tokens).first().copied() == Some("you") {
            carried_context = None;
        }
        if sentence_effects.is_empty()
            && !is_round_up_each_time_sentence(&sentence_tokens)
            && !is_nonsemantic_restriction_sentence(&sentence_tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                maybe_apply_carried_player_with_clause(effect, context, &sentence_tokens);
            }
            if let Some(context) = explicit_player_for_carry(effect) {
                carried_context = Some(context);
            }
        }
        if sentence_effects.len() == 1
            && let Some(previous_effect) = effects.last()
            && let Some(EffectAst::IfResult {
                predicate,
                effects: if_result_effects,
            }) = sentence_effects.first_mut()
        {
            if matches!(predicate, IfResultPredicate::Did)
                && matches!(previous_effect, EffectAst::UnlessPays { .. })
            {
                *predicate = IfResultPredicate::DidNot;
            }
            if let Some(previous_target) = primary_damage_target_from_effect(previous_effect) {
                replace_it_damage_target_in_effects(if_result_effects, &previous_target);
            }
        }
        let has_instead = sentence.iter().any(|token| token.is_word("instead"));
        if has_instead && sentence_effects.len() == 1 && effects.len() >= 1 {
            if matches!(
                sentence_effects.first(),
                Some(EffectAst::Conditional { .. })
            ) {
                let Some(previous) = effects.pop() else {
                    return Err(CardTextError::InvariantViolation(
                        "expected previous effect for 'instead' conditional rewrite".to_string(),
                    ));
                };
                let previous_target = primary_target_from_effect(&previous);
                let previous_damage_target = primary_damage_target_from_effect(&previous);
                if let Some(EffectAst::Conditional {
                    predicate,
                    mut if_true,
                    mut if_false,
                }) = sentence_effects.pop()
                {
                    if let Some(target) = previous_target {
                        replace_it_target_in_effects(&mut if_true, &target);
                    }
                    if let Some(target) = previous_damage_target {
                        replace_it_damage_target_in_effects(&mut if_true, &target);
                        replace_placeholder_damage_target_in_effects(&mut if_true, &target);
                    }
                    if_false.insert(0, previous);
                    effects.push(EffectAst::Conditional {
                        predicate,
                        if_true,
                        if_false,
                    });
                    sentence_idx += 1;
                    continue;
                }
            }
        }

        effects.extend(sentence_effects);
        sentence_idx += 1;
    }

    parser_trace("parse_effect_sentences:done", tokens);
    Ok(effects)
}

pub(crate) fn is_cant_be_regenerated_followup_sentence(tokens: &[Token]) -> bool {
    let words = normalize_cant_words(tokens);
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated"]
            | ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn is_cant_be_regenerated_this_turn_followup_sentence(tokens: &[Token]) -> bool {
    let words = normalize_cant_words(tokens);
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn apply_cant_be_regenerated_to_last_destroy_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };
    apply_cant_be_regenerated_to_effect(last)
}

pub(crate) fn apply_cant_be_regenerated_to_last_target_effect(
    effects: &mut Vec<EffectAst>,
) -> bool {
    let Some(previous_target) = effects.last().and_then(primary_target_from_effect) else {
        return false;
    };
    let Some(mut filter) = target_ast_to_object_filter(previous_target) else {
        return false;
    };
    if !filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    effects.push(EffectAst::Cant {
        restriction: crate::effect::Restriction::be_regenerated(filter),
        duration: Until::EndOfTurn,
    });
    true
}

fn apply_cant_be_regenerated_to_effect(effect: &mut EffectAst) -> bool {
    match effect {
        EffectAst::Destroy { target } => {
            let target = target.clone();
            *effect = EffectAst::DestroyNoRegeneration { target };
            true
        }
        EffectAst::DestroyAll { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllNoRegeneration { filter };
            true
        }
        EffectAst::DestroyAllOfChosenColor { filter } => {
            let filter = filter.clone();
            *effect = EffectAst::DestroyAllOfChosenColorNoRegeneration { filter };
            true
        }
        _ => {
            let mut applied = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if !applied {
                    applied = apply_cant_be_regenerated_to_effects_tail(nested);
                }
            });
            applied
        }
    }
}

fn apply_cant_be_regenerated_to_effects_tail(effects: &mut [EffectAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        if apply_cant_be_regenerated_to_effect(effect) {
            return true;
        }
    }
    false
}

pub(crate) fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. } | EffectAst::DealDamageEqualToPower { target, .. } => {
            Some(target.clone())
        }
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn primary_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::DealDamageEqualToPower { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::CounterUnlessPays { target, .. }
        | EffectAst::Explore { target }
        | EffectAst::Connive { target }
        | EffectAst::Goad { target }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::RemoveFromCombat { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::DestroyNoRegeneration { target }
        | EffectAst::Exile { target, .. }
        | EffectAst::ExileWhenSourceLeaves { target }
        | EffectAst::SacrificeSourceWhenLeaves { target }
        | EffectAst::ExileUntilSourceLeaves { target, .. }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Flip { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::ReturnToHand { target, .. }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::PutCounters { target, .. }
        | EffectAst::PutOrRemoveCounters { target, .. }
        | EffectAst::RemoveUpToAnyCounters { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::GrantProtectionChoice { target, .. }
        | EffectAst::PreventDamage { target, .. }
        | EffectAst::PreventAllDamageToTarget { target, .. }
        | EffectAst::PreventAllCombatDamageFromSource { source: target, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { target, .. }
        | EffectAst::RedirectNextTimeDamageToSource { target, .. }
        | EffectAst::GainControl { target, .. } => Some(target.clone()),
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_target_from_effect);
                }
            });
            found
        }
    }
}

pub(crate) fn replace_it_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_damage_target(effect, target);
    }
}

pub(crate) fn replace_it_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_target(effect, target);
    }
}

pub(crate) fn is_placeholder_damage_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
    )
}

pub(crate) fn replace_placeholder_damage_target_in_effects(
    effects: &mut [EffectAst],
    target: &TargetAst,
) {
    for effect in effects {
        replace_placeholder_damage_target(effect, target);
    }
}

pub(crate) fn replace_placeholder_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: damage_target,
            ..
        } => {
            if is_placeholder_damage_target(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_placeholder_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_unbound_x_in_damage_effects(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_damage_effect(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_damage_effect(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::LoseLife { amount, .. } => {
            if value_contains_unbound_x(amount) {
                *amount = replace_unbound_x_with_value(amount.clone(), replacement, clause)?;
            }
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_damage_effects(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effects_anywhere(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_effect_anywhere(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effect_anywhere(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    fn replace_value(
        value: &mut Value,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        if value_contains_unbound_x(value) {
            *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
        }
        Ok(())
    }

    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::PutCounters { count: amount, .. }
        | EffectAst::PutCountersAll { count: amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::Discard { count: amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::Monstrosity { amount } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            replace_value(put_count, replacement, clause)?;
            replace_value(remove_count, replacement, clause)?;
        }
        EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::CreateToken { count, .. }
        | EffectAst::CreateTokenWithMods { count, .. }
        | EffectAst::CreateTokenCopy { count, .. }
        | EffectAst::CreateTokenCopyFromSource { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            if let Some(life) = life.as_mut() {
                replace_value(life, replacement, clause)?;
            }
            if let Some(generic) = additional_generic.as_mut() {
                replace_value(generic, replacement, clause)?;
            }
        }
        _ => {
            try_for_each_nested_effects_mut(effect, true, |nested| {
                replace_unbound_x_in_effects_anywhere(nested, replacement, clause)
            })?;
        }
    }
    Ok(())
}

pub(crate) fn apply_where_x_to_damage_amounts(
    tokens: &[Token],
    effects: &mut [EffectAst],
) -> Result<(), CardTextError> {
    let clause_words = words(tokens);
    let has_deal_x = clause_words.windows(3).any(|window| {
        (window[0] == "deal" || window[0] == "deals") && window[1] == "x" && window[2] == "damage"
    });
    let has_x_life = clause_words.windows(3).any(|window| {
        (window[0] == "gain" || window[0] == "gains" || window[0] == "lose" || window[0] == "loses")
            && window[1] == "x"
            && window[2] == "life"
    });
    if !has_deal_x && !has_x_life {
        return Ok(());
    }
    let Some(where_idx) = clause_words
        .windows(3)
        .position(|window| window == ["where", "x", "is"])
    else {
        return Ok(());
    };
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Ok(());
    };
    let where_tokens = &tokens[where_token_idx..];
    let Some(where_value) = parse_where_x_value_clause(where_tokens) else {
        return Ok(());
    };
    replace_unbound_x_in_damage_effects(effects, &where_value, &clause_words.join(" "))
}

pub(crate) fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        } => {
            if target_references_it(damage_target) {
                *damage_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_damage_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn replace_it_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: effect_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: effect_target,
            ..
        }
        | EffectAst::Counter {
            target: effect_target,
        }
        | EffectAst::CounterUnlessPays {
            target: effect_target,
            ..
        }
        | EffectAst::Explore {
            target: effect_target,
        }
        | EffectAst::Connive {
            target: effect_target,
        }
        | EffectAst::Goad {
            target: effect_target,
        }
        | EffectAst::Tap {
            target: effect_target,
        }
        | EffectAst::Untap {
            target: effect_target,
        }
        | EffectAst::RemoveFromCombat {
            target: effect_target,
        }
        | EffectAst::TapOrUntap {
            target: effect_target,
        }
        | EffectAst::Destroy {
            target: effect_target,
        }
        | EffectAst::DestroyNoRegeneration {
            target: effect_target,
        }
        | EffectAst::Exile {
            target: effect_target,
            ..
        }
        | EffectAst::ExileWhenSourceLeaves {
            target: effect_target,
        }
        | EffectAst::SacrificeSourceWhenLeaves {
            target: effect_target,
        }
        | EffectAst::ExileUntilSourceLeaves {
            target: effect_target,
            ..
        }
        | EffectAst::LookAtHand {
            target: effect_target,
        }
        | EffectAst::Transform {
            target: effect_target,
        }
        | EffectAst::Flip {
            target: effect_target,
        }
        | EffectAst::Regenerate {
            target: effect_target,
        }
        | EffectAst::TargetOnly {
            target: effect_target,
        }
        | EffectAst::ReturnToHand {
            target: effect_target,
            ..
        }
        | EffectAst::ReturnToBattlefield {
            target: effect_target,
            ..
        }
        | EffectAst::MoveToZone {
            target: effect_target,
            ..
        }
        | EffectAst::PutCounters {
            target: effect_target,
            ..
        }
        | EffectAst::PutOrRemoveCounters {
            target: effect_target,
            ..
        }
        | EffectAst::RemoveUpToAnyCounters {
            target: effect_target,
            ..
        }
        | EffectAst::Pump {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesChoiceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantProtectionChoice {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamage {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllDamageToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllCombatDamageFromSource {
            source: effect_target,
            ..
        }
        | EffectAst::RedirectNextDamageFromSourceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::RedirectNextTimeDamageToSource {
            target: effect_target,
            ..
        }
        | EffectAst::GainControl {
            target: effect_target,
            ..
        } => {
            if target_references_it(effect_target) {
                *effect_target = target.clone();
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            replace_it_target_in_effects(nested, target);
        }),
    }
}

pub(crate) fn target_references_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        TargetAst::WithCount(inner, _) => target_references_it(inner),
        _ => false,
    }
}

pub(crate) fn is_that_turn_end_step_sentence(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    clause_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "that",
        "turn",
        "end",
        "step",
    ]) || clause_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "that",
        "turns",
        "end",
        "step",
    ])
}

pub(crate) fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        if let EffectAst::ExtraTurnAfterTurn { player, .. } = effect {
            Some(*player)
        } else {
            None
        }
    })
}

pub(crate) fn rewrite_when_you_do_clause_prefix(tokens: &[Token]) -> Vec<Token> {
    let clause_words = words(tokens);
    if clause_words.starts_with(&["when", "you", "do"]) {
        let mut rewritten = tokens.to_vec();
        for token in &mut rewritten {
            if let Token::Word(word, _) = token {
                if word.eq_ignore_ascii_case("when") {
                    *word = "if".to_string();
                }
                break;
            }
        }
        return rewritten;
    }

    // Generic "When one or more ... this way, ..." follow-ups are semantically
    // "If you do, ..." against the immediately previous effect result.
    let has_this_way = clause_words
        .windows(2)
        .any(|window| window == ["this", "way"]);
    if (clause_words.starts_with(&["when", "one", "or", "more"])
        || clause_words.starts_with(&["whenever", "one", "or", "more"]))
        && has_this_way
    {
        let Some(comma_idx) = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        else {
            return tokens.to_vec();
        };
        let mut rewritten = Vec::new();

        let mut if_token = tokens[0].clone();
        if let Token::Word(word, _) = &mut if_token {
            *word = "if".to_string();
        }
        rewritten.push(if_token);

        let mut you_token = tokens.get(1).cloned().unwrap_or_else(|| tokens[0].clone());
        if let Token::Word(word, _) = &mut you_token {
            *word = "you".to_string();
        }
        rewritten.push(you_token);

        let mut do_token = tokens.get(2).cloned().unwrap_or_else(|| tokens[0].clone());
        if let Token::Word(word, _) = &mut do_token {
            *word = "do".to_string();
        }
        rewritten.push(do_token);

        rewritten.push(tokens[comma_idx].clone());
        rewritten.extend_from_slice(&tokens[comma_idx + 1..]);
        return rewritten;
    }

    tokens.to_vec()
}

pub(crate) fn strip_otherwise_sentence_prefix(tokens: &[Token]) -> Option<Vec<Token>> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
    {
        return None;
    }

    let mut idx = 1usize;
    while matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("then")) {
        idx += 1;
    }
    while matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }

    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    }
}

pub(crate) fn rewrite_otherwise_referential_subject(tokens: Vec<Token>) -> Vec<Token> {
    let clause_words = words(&tokens);
    let is_referential_get = clause_words.len() >= 3
        && clause_words[0] == "that"
        && matches!(clause_words[1], "creature" | "permanent")
        && matches!(clause_words[2], "gets" | "get" | "gains" | "gain");
    if !is_referential_get {
        return tokens;
    }

    let mut rewritten = tokens;
    if let Some(first) = rewritten.get_mut(0)
        && let Token::Word(word, _) = first
    {
        *word = "target".to_string();
    }
    rewritten
}

pub(crate) fn is_nonsemantic_restriction_sentence(tokens: &[Token]) -> bool {
    is_activate_only_restriction_sentence(tokens) || is_trigger_only_restriction_sentence(tokens)
}

fn token_copy_followup_container_effects_mut(
    effect: &mut EffectAst,
) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. } => Some(effects),
        _ => None,
    }
}

fn parse_token_copy_followup_sentence(tokens: &[Token]) -> Option<TokenCopyFollowup> {
    parse_token_copy_modifier_sentence(tokens)
        .or_else(|| {
            is_exile_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::ExileAtEndOfCombat)
        })
        .or_else(|| {
            is_sacrifice_that_token_at_end_of_combat(tokens)
                .then_some(TokenCopyFollowup::SacrificeAtEndOfCombat)
        })
}

fn apply_unapplied_token_copy_followup(
    sentence: &[Token],
    _sentence_tokens: &[Token],
    followup: TokenCopyFollowup,
) -> Result<Vec<EffectAst>, CardTextError> {
    let span = span_from_tokens(sentence);
    let effects = match followup {
        TokenCopyFollowup::HasHaste => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![StaticAbility::haste().into()],
            duration: Until::Forever,
        }],
        TokenCopyFollowup::GainHasteUntilEndOfTurn => vec![EffectAst::GrantAbilitiesToTarget {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
            abilities: vec![StaticAbility::haste().into()],
            duration: Until::EndOfTurn,
        }],
        TokenCopyFollowup::SacrificeAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
            }],
        }],
        TokenCopyFollowup::ExileAtNextEndStep => vec![EffectAst::DelayedUntilNextEndStep {
            player: PlayerFilter::Any,
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::ExileAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Exile {
                target: TargetAst::Object(ObjectFilter::tagged(TagKey::from(IT_TAG)), span, None),
                face_down: false,
            }],
        }],
        TokenCopyFollowup::SacrificeAtEndOfCombat => vec![EffectAst::DelayedUntilEndOfCombat {
            effects: vec![EffectAst::Sacrifice {
                filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                player: PlayerAst::Implicit,
                count: 1,
            }],
        }],
    };
    Ok(effects)
}

pub(crate) fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    followup: TokenCopyFollowup,
) -> Result<bool, CardTextError> {
    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    match last {
        EffectAst::CreateTokenCopy {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            match followup {
                TokenCopyFollowup::HasHaste => *has_haste = true,
                TokenCopyFollowup::SacrificeAtNextEndStep => *sacrifice_at_next_end_step = true,
                TokenCopyFollowup::ExileAtNextEndStep => *exile_at_next_end_step = true,
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtEndOfCombat => return Ok(false),
            }
            Ok(true)
        }
        EffectAst::CreateTokenWithMods {
            exile_at_end_of_combat,
            sacrifice_at_end_of_combat,
            ..
        } => {
            match followup {
                TokenCopyFollowup::ExileAtEndOfCombat => *exile_at_end_of_combat = true,
                TokenCopyFollowup::SacrificeAtEndOfCombat => *sacrifice_at_end_of_combat = true,
                TokenCopyFollowup::HasHaste
                | TokenCopyFollowup::GainHasteUntilEndOfTurn
                | TokenCopyFollowup::SacrificeAtNextEndStep
                | TokenCopyFollowup::ExileAtNextEndStep => return Ok(false),
            }
            Ok(true)
        }
        _ => {
            let Some(nested_effects) = token_copy_followup_container_effects_mut(last) else {
                return Ok(false);
            };
            if nested_effects.is_empty() {
                return Ok(false);
            }
            try_apply_token_copy_followup(nested_effects.as_mut_slice(), followup)
        }
    }
}

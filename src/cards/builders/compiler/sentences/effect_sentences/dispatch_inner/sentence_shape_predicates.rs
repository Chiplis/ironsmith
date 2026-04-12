type DispatchInnerNormalizedWords<'a> = TokenWordView<'a>;

fn find_dispatch_inner_phrase_start(words: &[&str], phrase: &[&str]) -> Option<usize> {
    find_word_slice_phrase_start(words, phrase)
}

macro_rules! sentence_unsupported_adapters_lexed {
    ($(($adapter:ident, $predicate:ident)),* $(,)?) => {
        $(
            pub(super) fn $adapter(view: &LexClauseView<'_>) -> bool {
                let words = view.words.to_word_refs();
                $predicate(words.as_slice(), view.tokens)
            }
        )*
    };
}

fn slice_contains_any(words: &[&str], expected: &[&str]) -> bool {
    expected
        .iter()
        .any(|word| words.iter().any(|candidate| candidate == word))
}

fn slice_starts_with_any(words: &[&str], prefixes: &[&[&str]]) -> bool {
    prefixes
        .iter()
        .any(|prefix| starts_with_words(words, prefix))
}

fn contains_word_window(words: &[&str], pattern: &[&str]) -> bool {
    if pattern.is_empty() || words.len() < pattern.len() {
        return false;
    }

    for start in 0..=words.len() - pattern.len() {
        if words[start..start + pattern.len()]
            .iter()
            .zip(pattern.iter())
            .all(|(word, expected)| word == expected)
        {
            return true;
        }
    }

    false
}

fn starts_with_words(words: &[&str], prefix: &[&str]) -> bool {
    words.len() >= prefix.len()
        && words[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(word, expected)| word == expected)
}

fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::next_spell_family::parse_next_spell_grant_sentence_lexed(tokens)
}

pub(super) fn sentence_has_enters_as_copy_rule_lexed(view: &LexClauseView<'_>) -> bool {
    effect_grammar::is_enters_as_copy_clause_lexed(view.tokens)
}

sentence_unsupported_adapters_lexed!(
    (
        sentence_has_each_player_lose_discard_sacrifice_chain_rule_lexed,
        sentence_has_each_player_lose_discard_sacrifice_chain
    ),
    (
        sentence_has_each_player_exile_sacrifice_return_exiled_clause_rule_lexed,
        sentence_has_each_player_exile_sacrifice_return_exiled_clause
    ),
    (
        sentence_has_put_one_of_them_into_hand_rest_clause_rule_lexed,
        sentence_has_put_one_of_them_into_hand_rest_clause
    ),
    (
        sentence_has_loses_all_abilities_with_becomes_clause_rule_lexed,
        sentence_has_loses_all_abilities_with_becomes_clause
    ),
    (
        sentence_has_spent_to_cast_this_spell_without_condition_rule_lexed,
        sentence_has_spent_to_cast_this_spell_without_condition
    ),
    (
        sentence_has_would_enter_instead_replacement_clause_rule_lexed,
        sentence_has_would_enter_instead_replacement_clause
    ),
    (
        sentence_has_different_mana_value_constraint_rule_lexed,
        sentence_has_different_mana_value_constraint
    ),
    (
        sentence_has_most_common_color_constraint_rule_lexed,
        sentence_has_most_common_color_constraint
    ),
    (
        sentence_has_power_vs_count_constraint_rule_lexed,
        sentence_has_power_vs_count_constraint
    ),
    (
        sentence_has_put_into_graveyards_from_battlefield_this_turn_rule_lexed,
        sentence_has_put_into_graveyards_from_battlefield_this_turn
    ),
    (
        sentence_has_phase_out_until_leaves_clause_rule_lexed,
        sentence_has_phase_out_until_leaves_clause
    ),
    (
        sentence_has_same_name_as_another_in_hand_clause_rule_lexed,
        sentence_has_same_name_as_another_in_hand_clause
    ),
    (
        sentence_has_for_each_mana_from_spent_to_cast_clause_rule_lexed,
        sentence_has_for_each_mana_from_spent_to_cast_clause
    ),
    (
        sentence_has_when_you_sacrifice_this_way_clause_rule_lexed,
        sentence_has_when_you_sacrifice_this_way_clause
    ),
    (
        sentence_has_sacrifice_any_number_then_draw_that_many_clause_rule_lexed,
        sentence_has_sacrifice_any_number_then_draw_that_many_clause
    ),
    (
        sentence_has_greatest_mana_value_clause_rule_lexed,
        sentence_has_greatest_mana_value_clause
    ),
    (
        sentence_has_least_power_among_creatures_clause_rule_lexed,
        sentence_has_least_power_among_creatures_clause
    ),
    (
        sentence_has_villainous_choice_clause_rule_lexed,
        sentence_has_villainous_choice_clause
    ),
    (
        sentence_has_divided_evenly_clause_rule_lexed,
        sentence_has_divided_evenly_clause
    ),
    (
        sentence_has_different_names_clause_rule_lexed,
        sentence_has_different_names_clause
    ),
    (
        sentence_has_chosen_at_random_clause_rule_lexed,
        sentence_has_chosen_at_random_clause
    ),
    (
        sentence_has_defending_players_choice_clause_rule_lexed,
        sentence_has_defending_players_choice_clause
    ),
    (
        sentence_has_target_creature_token_player_planeswalker_clause_rule_lexed,
        sentence_has_target_creature_token_player_planeswalker_clause
    ),
    (
        sentence_has_if_you_sacrifice_an_island_this_way_clause_rule_lexed,
        sentence_has_if_you_sacrifice_an_island_this_way_clause
    ),
    (
        sentence_has_spent_to_cast_clause_rule_lexed,
        sentence_has_spent_to_cast_clause
    ),
    (
        sentence_has_face_down_clause_rule_lexed,
        sentence_has_face_down_clause
    ),
    (
        sentence_has_copy_spell_legendary_exception_clause_rule_lexed,
        sentence_has_copy_spell_legendary_exception_clause
    ),
    (
        sentence_has_return_each_creature_that_isnt_list_clause_rule_lexed,
        sentence_has_return_each_creature_that_isnt_list_clause
    ),
    (
        sentence_has_unsupported_negated_untap_clause_rule_lexed,
        sentence_has_unsupported_negated_untap_clause
    ),
);

pub(super) fn sentence_looks_like_supported_negated_untap_clause(tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::looks_like_supported_negated_untap_clause_lexed(tokens)
}

fn sentence_has_each_player_lose_discard_sacrifice_chain(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_each_player_lose_discard_sacrifice_chain_sentence_lexed(tokens)
}

fn sentence_has_each_player_exile_sacrifice_return_exiled_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_each_player_exile_sacrifice_return_exiled_clause_sentence_lexed(tokens)
}

fn sentence_has_put_one_of_them_into_hand_rest_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_put_one_of_them_into_hand_rest_clause_sentence_lexed(tokens)
}

fn sentence_has_loses_all_abilities_with_becomes_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_loses_all_abilities_with_becomes_clause_sentence_lexed(tokens)
}

fn sentence_has_spent_to_cast_this_spell_without_condition(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_spent_to_cast_this_spell_without_condition_sentence_lexed(tokens)
}

fn sentence_has_would_enter_instead_replacement_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_would_enter_instead_replacement_clause_sentence_lexed(tokens)
}

fn sentence_has_different_mana_value_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_different_mana_value_constraint_sentence_lexed(tokens)
}

fn sentence_has_most_common_color_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_most_common_color_constraint_sentence_lexed(tokens)
}

fn sentence_has_power_vs_count_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_power_vs_count_constraint_sentence_lexed(tokens)
}

fn sentence_has_put_into_graveyards_from_battlefield_this_turn(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_put_into_graveyards_from_battlefield_this_turn_sentence_lexed(tokens)
}

fn sentence_has_phase_out_until_leaves_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_phase_out_until_leaves_clause_sentence_lexed(tokens)
}

fn sentence_has_same_name_as_another_in_hand_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_same_name_as_another_in_hand_clause_sentence_lexed(tokens)
}

fn sentence_has_for_each_mana_from_spent_to_cast_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_for_each_mana_from_spent_to_cast_clause_sentence_lexed(tokens)
}

fn sentence_has_when_you_sacrifice_this_way_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_when_you_sacrifice_this_way_clause_sentence_lexed(tokens)
}

fn sentence_has_sacrifice_any_number_then_draw_that_many_clause(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_sacrifice_any_number_then_draw_that_many_clause_sentence_lexed(
        words, tokens,
    )
}

fn sentence_has_greatest_mana_value_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_greatest_mana_value_clause_sentence_lexed(words)
}

fn sentence_has_least_power_among_creatures_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_least_power_among_creatures_clause_sentence_lexed(words)
}

fn sentence_has_villainous_choice_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_villainous_choice_clause_sentence_lexed(tokens)
}

fn sentence_has_divided_evenly_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_divided_evenly_clause_sentence_lexed(words)
}

fn sentence_has_different_names_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_different_names_clause_sentence_lexed(words)
}

fn sentence_has_chosen_at_random_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_chosen_at_random_clause_sentence_lexed(words)
}

fn sentence_has_defending_players_choice_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_defending_players_choice_clause_sentence_lexed(tokens)
}

fn sentence_has_target_creature_token_player_planeswalker_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_target_creature_token_player_planeswalker_clause_sentence_lexed(tokens)
}

fn sentence_has_if_you_sacrifice_an_island_this_way_clause(
    words: &[&str],
    _: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_if_you_sacrifice_an_island_this_way_clause_sentence_lexed(words)
}

fn sentence_has_spent_to_cast_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    effect_grammar::has_spent_to_cast_clause_sentence_lexed(words)
}

fn sentence_has_face_down_clause(words: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_face_down_clause_sentence_lexed(words, tokens)
}

fn sentence_has_copy_spell_legendary_exception_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_copy_spell_legendary_exception_clause_sentence_lexed(tokens)
}

fn sentence_has_return_each_creature_that_isnt_list_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_return_each_creature_that_isnt_list_clause_sentence_lexed(tokens)
}

fn sentence_has_unsupported_negated_untap_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_unsupported_negated_untap_clause_sentence_lexed(tokens)
}

pub(crate) fn parse_effect_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn search_followup_shuffle_player(effect: &EffectAst) -> Option<PlayerAst> {
        match effect {
            EffectAst::SearchLibrary { player, .. } => Some(*player),
            _ => None,
        }
    }

    fn normalize_search_followup_shuffles(effects: &mut [EffectAst]) {
        for idx in 0..effects.len() {
            let is_default_shuffle = matches!(
                effects.get(idx),
                Some(EffectAst::ShuffleLibrary { player })
                    if matches!(*player, PlayerAst::You | PlayerAst::Implicit)
            );
            if !is_default_shuffle {
                continue;
            }
            let Some(search_player) = effects[..idx]
                .iter()
                .rev()
                .find_map(search_followup_shuffle_player)
            else {
                continue;
            };
            if !matches!(search_player, PlayerAst::You | PlayerAst::Implicit) {
                if let EffectAst::ShuffleLibrary { player } = &mut effects[idx] {
                    *player = search_player;
                }
            }
        }
    }

    if let Some(meld_effect) = parse_exile_then_meld_sentence(tokens)? {
        return Ok(vec![meld_effect]);
    }
    if let Some(effect) =
        crate::cards::builders::compiler::sentences::effect_sentences::special_sentence_family::parse_control_combat_choices_sentence(tokens)?
    {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_if_damage_would_be_dealt_put_counters_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_cant_effect_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    let clause_word_storage = DispatchInnerNormalizedWords::new(tokens);
    let clause_words = clause_word_storage.to_word_refs();
    if contains_word_window(clause_words.as_slice(), &["where", "x", "is"]) {
        let mut effects = parse_effect_sentence_with_where_x_lexed(tokens)?;
        normalize_search_followup_shuffles(&mut effects);
        return Ok(effects);
    }
    let mut effects = parse_effect_sentence_inner_lexed(tokens)?;
    normalize_search_followup_shuffles(&mut effects);
    Ok(effects)
}

fn parse_exile_then_meld_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    super::special_sentence_family::parse_exile_then_meld_sentence(tokens)
}

fn parse_if_damage_would_be_dealt_put_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    super::special_sentence_family::parse_if_damage_would_be_dealt_put_counters_sentence(tokens)
}

fn parse_effect_sentence_with_where_x_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn replace_search_filter_x(effect: &mut EffectAst, replacement: &Value) {
        if let EffectAst::SearchLibrary {
            filter,
            count,
            count_value,
            ..
        } = effect
        {
            if count.dynamic_x && count_value.is_none() {
                *count_value = Some(replacement.clone());
            }
            if let Some(mana_value) = filter.mana_value.as_mut() {
                use crate::filter::Comparison;

                match mana_value {
                    Comparison::EqualExpr(value)
                    | Comparison::NotEqualExpr(value)
                    | Comparison::LessThanExpr(value)
                    | Comparison::LessThanOrEqualExpr(value)
                    | Comparison::GreaterThanExpr(value)
                    | Comparison::GreaterThanOrEqualExpr(value)
                        if matches!(value.as_ref(), Value::X) =>
                    {
                        **value = replacement.clone();
                    }
                    _ => {}
                }
            }
        }
    }

    let clause_word_storage = DispatchInnerNormalizedWords::new(tokens);
    let clause_words = clause_word_storage.to_word_refs();
    let Some(where_idx) =
        find_dispatch_inner_phrase_start(clause_words.as_slice(), &["where", "x", "is"])
    else {
        return parse_effect_sentence_inner_lexed(tokens);
    };
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported where-x clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    let where_tokens = &tokens[where_token_idx..];
    let where_segments = split_lexed_slices_on_commas_or_semicolons(where_tokens);
    let primary_where_tokens = where_segments.first().copied().unwrap_or(where_tokens);
    let trailing_after_where = if where_segments.len() > 1 {
        let mut tail = Vec::new();
        for (idx, segment) in where_segments.iter().enumerate().skip(1) {
            if idx > 1 {
                tail.push(OwnedLexToken::comma(TextSpan::synthetic()));
            }
            tail.extend(segment.iter().cloned());
        }
        tail
    } else {
        Vec::new()
    };

    let stripped = trim_edge_punctuation(&tokens[..where_token_idx]);
    let stripped_word_storage = DispatchInnerNormalizedWords::new(&stripped);
    let stripped_words = stripped_word_storage.to_word_refs();
    let where_word_storage = DispatchInnerNormalizedWords::new(&primary_where_tokens);
    let where_words = where_word_storage.to_word_refs();

    let where_value = match where_words.get(3..) {
        Some(["its", "power"]) => {
            if stripped_words.iter().any(|w| *w == "target") {
                Value::PowerOf(Box::new(crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::Object(ObjectFilter::default()),
                )))
            } else {
                Value::SourcePower
            }
        }
        Some(["its", "toughness"]) => {
            if stripped_words.iter().any(|w| *w == "target") {
                Value::ToughnessOf(Box::new(crate::target::ChooseSpec::target(
                    crate::target::ChooseSpec::Object(ObjectFilter::default()),
                )))
            } else {
                Value::SourceToughness
            }
        }
        Some(["its", "mana", "value"]) => {
            Value::ManaValueOf(Box::new(if stripped_words.iter().any(|w| *w == "target") {
                crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                    ObjectFilter::default(),
                ))
            } else {
                crate::target::ChooseSpec::Source
            }))
        }
        Some(["this", "creatures", "power"]) => Value::SourcePower,
        Some(["this", "creatures", "toughness"]) => Value::SourceToughness,
        Some(["this", "creatures", "mana", "value"]) => {
            Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Source))
        }
        Some(["that", "creatures", "power"]) => {
            Value::PowerOf(Box::new(if stripped_words.iter().any(|w| *w == "target") {
                crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                    ObjectFilter::default(),
                ))
            } else {
                crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))
            }))
        }
        Some(["that", "creatures", "toughness"]) => {
            Value::ToughnessOf(Box::new(if stripped_words.iter().any(|w| *w == "target") {
                crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                    ObjectFilter::default(),
                ))
            } else {
                crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))
            }))
        }
        Some(["that", "creatures", "mana", "value"]) => {
            Value::ManaValueOf(Box::new(if stripped_words.iter().any(|w| *w == "target") {
                crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                    ObjectFilter::default(),
                ))
            } else {
                crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))
            }))
        }
        Some(
            [
                "2",
                "plus",
                "the",
                "sacrificed",
                "creature",
                "mana",
                "value",
                ..,
            ],
        )
        | Some(
            [
                "2",
                "plus",
                "the",
                "sacrificed",
                "creatures",
                "mana",
                "value",
                ..,
            ],
        )
        | Some(["2", "plus", "sacrificed", "creature", "mana", "value", ..])
        | Some(["2", "plus", "sacrificed", "creatures", "mana", "value", ..])
        | Some(
            [
                "two",
                "plus",
                "the",
                "sacrificed",
                "creature",
                "mana",
                "value",
                ..,
            ],
        )
        | Some(
            [
                "two",
                "plus",
                "the",
                "sacrificed",
                "creatures",
                "mana",
                "value",
                ..,
            ],
        )
        | Some(["two", "plus", "sacrificed", "creature", "mana", "value", ..])
        | Some(
            [
                "two",
                "plus",
                "sacrificed",
                "creatures",
                "mana",
                "value",
                ..,
            ],
        ) => Value::Add(
            Box::new(Value::Fixed(2)),
            Box::new(Value::ManaValueOf(Box::new(
                crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG)),
            ))),
        ),
        _ => parse_where_x_value_clause_lexed(&primary_where_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported where-x clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?,
    };

    let search_like = stripped_words.first().copied() == Some("search");
    let mut effects = if search_like && !trailing_after_where.is_empty() {
        let mut recombined = stripped.clone();
        recombined.extend(trailing_after_where.clone());
        parse_effect_sentence_lexed(&recombined)?
    } else {
        let mut parsed = parse_effect_sentence_inner_lexed(&stripped)?;
        if !trailing_after_where.is_empty() {
            let mut trailing_effects = parse_effect_sentence_lexed(&trailing_after_where)?;
            parsed.append(&mut trailing_effects);
        }
        parsed
    };
    replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_words.join(" "))?;
    for effect in &mut effects {
        replace_search_filter_x(effect, &where_value);
    }
    Ok(effects)
}

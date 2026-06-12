type DispatchInnerNormalizedWords<'a> = TokenWordView<'a>;

use crate::runtime_backend::effect_sentences::{
    SubjectVerbPrimitiveClause, parse_sentence_delayed_next_step_unless_pays,
};
use crate::runtime_backend::lexer::{
    word_slice_contains_any_phrase, word_slice_find_any_phrase_start, word_slice_find_phrase_start,
    word_slice_matching_phrase, word_slice_matching_prefix,
};

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

const SENTENCE_WITH_WORD: &str = "with";
const SENTENCE_NO_WORD: &str = "no";
const SENTENCE_WHERE_X_IS_ITS_POWER_PREFIX: &[&str] = &["where", "x", "is", "its", "power"];
const SENTENCE_SELF_DAMAGE_TARGET_PHRASES: &[&[&str]] = &[&["itself"], &["it"]];
const SENTENCE_DEALS_X_DAMAGE_TO_PREFIXES: &[&[&str]] = &[
    &["deals", "x", "damage", "to"],
    &["deal", "x", "damage", "to"],
];
const SENTENCE_AND_X_DAMAGE_TO_PREFIX: &[&str] = &["and", "x", "damage", "to"];
const SENTENCE_WHERE_X_NUMBER_TAPPED_THIS_WAY_PREFIX: &[&str] =
    &["where", "x", "is", "the", "number", "of"];
const SENTENCE_TAPPED_THIS_WAY_SUFFIX: &[&str] = &["tapped", "this", "way"];
const SENTENCE_THIS_WAY_PHRASE: &[&str] = &["this", "way"];
const SENTENCE_CHOSEN_WORD: &str = "chosen";
const SENTENCE_MANA_VALUE_PREFIX: &[&str] = &["mana", "value"];
const SENTENCE_WHERE_X_IS_PREFIX: &[&str] = &["where", "x", "is"];
const SENTENCE_POWER_WORD: &str = "power";
const SENTENCE_TOUGHNESS_WORD: &str = "toughness";
const SENTENCE_OF_WORD: &str = "of";
const SENTENCE_THE_WORD: &str = "the";
const SENTENCE_EXILED_CARD_REFERENCE_PHRASES: &[&[&str]] = &[
    &["the", "exiled", "card"],
    &["the", "exiled", "cards"],
    &["exiled", "card"],
    &["exiled", "cards"],
];
const SENTENCE_NUMBER_OF_PREFIX: &[&str] = &["number", "of"];
const SENTENCE_COUNTER_WORDS: &[&str] = &["counter", "counters"];
const SENTENCE_REMOVED_WORD: &str = "removed";
const SENTENCE_COMMANDER_MANA_VALUE_CHOICE_WORDS: &[&str] = &[
    "mana",
    "value",
    "of",
    "commander",
    "you",
    "own",
    "on",
    "battlefield",
    "or",
    "in",
    "command",
    "zone",
];
const SENTENCE_TO_THE_PLAYER_PHRASE: &[&str] = &["to", "the", "player"];
const SENTENCE_ITS_AN_PREFIXES: &[&[&str]] =
    &[&["it's", "an"], &["it’s", "an"], &["its", "an"], &["it", "s", "an"]];
const SENTENCE_IT_IS_AN_PREFIX: &[&str] = &["it", "is", "an"];
const SENTENCE_AURA_ENCHANT_CREATURE_PREFIX: &[&str] =
    &["aura", "enchantment", "with", "enchant", "creature"];
const SENTENCE_YOU_CONTROL_PREFIX: &[&str] = &["you", "control"];
const SENTENCE_LOSES_ALL_ABILITIES_PHRASES: &[&[&str]] = &[
    &["loses", "all", "other", "abilities"],
    &["loses", "all", "abilities"],
];
const SENTENCE_AT_THIS_PREFIX: &[&str] = &["at", "this"];
const SENTENCE_SACRIFICE_WORD: &str = "sacrifice";
const SENTENCE_SACRIFICE_COUNTED_PREFIXES: &[&[&str]] = &[
    &["sacrifice", "any", "number"],
    &["sacrifice", "one", "or", "more"],
];
const SENTENCE_DELAYED_LIFECYCLE_PHRASES: &[&[&str]] = &[
    &["at", "the", "beginning", "of", "the", "next", "end", "step"],
    &["at", "the", "beginning", "of", "next", "end", "step"],
    &["at", "end", "of", "combat"],
    &["at", "the", "end", "of", "combat"],
];
const SENTENCE_END_OF_COMBAT_PREFIX: &[&str] = &["end", "of", "combat"];
const SENTENCE_NEXT_WORD: &str = "next";
const SENTENCE_WOULD_WORD: &str = "would";
const SENTENCE_TARGET_WORD: &str = "target";
const SENTENCE_SEARCH_WORD: &str = "search";
const SENTENCE_ARTICLE_WORDS: &[&str] = &["a", "an", "the"];

fn sentence_removed_counters_this_way(words: &[&str]) -> bool {
    word_slice_contains_any_word(words, SENTENCE_COUNTER_WORDS)
        && word_slice_contains_word(words, SENTENCE_REMOVED_WORD)
        && word_slice_contains_phrase(words, SENTENCE_THIS_WAY_PHRASE)
}

fn trailing_counter_constraint(
    tokens: &[OwnedLexToken],
) -> Option<crate::filter::CounterConstraint> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let with_idx = find_index(&words, |word| *word == SENTENCE_WITH_WORD)?;
    let tail = &words[with_idx + 1..];
    if word_slice_eq(tail, &[SENTENCE_NO_WORD]) {
        return None;
    }
    let (counter_constraint, consumed) = parse_filter_counter_constraint_words(tail)?;
    (consumed == tail.len()).then_some(counter_constraint)
}

fn apply_trailing_counter_constraint_to_destroy_all(
    effects: &mut [EffectAst],
    tokens: &[OwnedLexToken],
) {
    let Some(counter_constraint) = trailing_counter_constraint(tokens) else {
        return;
    };
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::DestroyAll { filter, .. }
                    | SubjectVerbActionAst::ExileAll { filter, .. },
                ..
            }) => {
                if filter.with_counter.is_none() {
                    filter.with_counter = Some(counter_constraint);
                }
            }
            _ => {}
        }
    }
}

fn parse_target_deals_power_damage_to_other_and_self_where_x(
    tokens: &[OwnedLexToken],
    words: &[&str],
    where_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !word_slice_starts_with(&words[where_idx..], SENTENCE_WHERE_X_IS_ITS_POWER_PREFIX) {
        return Ok(None);
    }

    let Some((_deal_phrase, deal_idx)) =
        word_slice_find_any_phrase_start(words, SENTENCE_DEALS_X_DAMAGE_TO_PREFIXES)
    else {
        return Ok(None);
    };
    if deal_idx == 0 {
        return Ok(None);
    }

    let Some(and_idx) = word_slice_find_phrase_start(words, SENTENCE_AND_X_DAMAGE_TO_PREFIX) else {
        return Ok(None);
    };
    if and_idx <= deal_idx + 4 || where_idx <= and_idx + 4 {
        return Ok(None);
    }

    let self_target_words = &words[and_idx + 4..where_idx];
    if !word_slice_eq_any(self_target_words, SENTENCE_SELF_DAMAGE_TARGET_PHRASES) {
        return Ok(None);
    }

    let source_end = token_index_for_word_index(tokens, deal_idx).unwrap_or(tokens.len());
    let first_target_start =
        token_index_for_word_index(tokens, deal_idx + 4).unwrap_or(tokens.len());
    let first_target_end = token_index_for_word_index(tokens, and_idx).unwrap_or(tokens.len());
    let source_tokens = trim_edge_punctuation(&tokens[..source_end]);
    let first_target_tokens = trim_edge_punctuation(&tokens[first_target_start..first_target_end]);
    if source_tokens.is_empty() || first_target_tokens.is_empty() {
        return Ok(None);
    }

    let source = parse_target_phrase(&source_tokens)?;
    let first_target = parse_target_phrase(&first_target_tokens)?;
    let source_ref = TargetAst::Tagged(TagKey::from(IT_TAG), None);
    Ok(Some(vec![
        EffectAst::subject_verb_target_only(source.clone()),
        EffectAst::subject_verb_damage_equal_to_power(source_ref.clone(), first_target),
        EffectAst::subject_verb_damage_equal_to_power(source_ref.clone(), source_ref),
    ]))
}

fn where_x_is_number_tapped_this_way(words: &[&str]) -> bool {
    words.len() >= 9
        && word_slice_starts_with(words, SENTENCE_WHERE_X_NUMBER_TAPPED_THIS_WAY_PREFIX)
        && word_slice_ends_with(words, SENTENCE_TAPPED_THIS_WAY_SUFFIX)
}

fn prior_effect_words_reference_memory(words: &[&str]) -> bool {
    word_slice_contains_phrase(words, SENTENCE_THIS_WAY_PHRASE)
        || words.iter().any(|word| {
            matches!(
                *word,
                "chosen"
                    | "destroyed"
                    | "discarded"
                    | "exiled"
                    | "milled"
                    | "revealed"
                    | "sacrificed"
                    | "searched"
            )
        })
}

fn prior_effect_metric_source(words: &[&str]) -> ironsmith_core::EffectMetricSource {
    if word_slice_contains_word(words, SENTENCE_CHOSEN_WORD) {
        ironsmith_core::EffectMetricSource::ChosenObjects
    } else {
        ironsmith_core::EffectMetricSource::AffectedObjects
    }
}

fn parse_where_x_prior_effect_first_metric_value(words: &[&str], mut idx: usize) -> Option<Value> {
    let metric = if word_slice_at_is(words, idx, SENTENCE_POWER_WORD) {
        idx += 1;
        ironsmith_core::EffectMetric::FirstPower
    } else if word_slice_at_is(words, idx, SENTENCE_TOUGHNESS_WORD) {
        idx += 1;
        ironsmith_core::EffectMetric::FirstToughness
    } else if word_slice_starts_with(&words[idx..], SENTENCE_MANA_VALUE_PREFIX) {
        idx += 2;
        ironsmith_core::EffectMetric::FirstManaValue
    } else {
        return None;
    };
    if !word_slice_at_is(words, idx, SENTENCE_OF_WORD) {
        return None;
    }
    let object_words = &words[idx + 1..];
    if metric == ironsmith_core::EffectMetric::FirstManaValue
        && word_slice_eq_any(object_words, SENTENCE_EXILED_CARD_REFERENCE_PHRASES)
    {
        return Some(Value::ManaValueOf(Box::new(
            crate::target::ChooseSpec::Tagged(TagKey::from(crate::tag::SOURCE_EXILED_TAG)),
        )));
    }
    if !prior_effect_words_reference_memory(object_words) {
        return None;
    }

    Some(Value::PendingEffectMetric {
        source: prior_effect_metric_source(object_words),
        metric,
    })
}

fn parse_where_x_prior_effect_number_value(words: &[&str]) -> Option<Value> {
    if !word_slice_starts_with(words, SENTENCE_WHERE_X_IS_PREFIX) {
        return None;
    }
    let mut idx = 3usize;
    if word_slice_at_is(words, idx, SENTENCE_THE_WORD) {
        idx += 1;
    }
    if let Some(value) = parse_where_x_prior_effect_first_metric_value(words, idx) {
        return Some(value);
    }
    if !word_slice_starts_with(&words[idx..], SENTENCE_NUMBER_OF_PREFIX) {
        return None;
    }

    let object_words = &words[idx + 2..];
    if sentence_removed_counters_this_way(object_words) {
        return Some(Value::X);
    }
    if !prior_effect_words_reference_memory(object_words) {
        return None;
    }

    Some(Value::PendingEffectMetric {
        source: prior_effect_metric_source(object_words),
        metric: ironsmith_core::EffectMetric::Count,
    })
}

fn parse_where_x_commander_mana_value_choice(words: &[&str]) -> Option<(EffectAst, Value)> {
    if !word_slice_starts_with(words, SENTENCE_WHERE_X_IS_PREFIX) {
        return None;
    }
    let tail: Vec<&str> = words
        .get(3..)?
        .iter()
        .copied()
        .filter(|word| !SENTENCE_ARTICLE_WORDS.contains(word))
        .collect();
    if !word_slice_eq(&tail, SENTENCE_COMMANDER_MANA_VALUE_CHOICE_WORDS) {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.is_commander = true;
    filter.owner = Some(PlayerFilter::You);
    let tag = TagKey::from("__where_x_commander_mana_value");

    Some((
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
            zones: vec![Zone::Battlefield, Zone::Command],
            search_mode: None,
        },
        Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(tag))),
    ))
}

fn parse_tap_then_damage_for_number_tapped_this_way(
    stripped: &[OwnedLexToken],
    where_words: &[&str],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !where_x_is_number_tapped_this_way(where_words) {
        return Ok(None);
    }

    let mut effects = parse_effect_sentence_inner_lexed(stripped)?;
    if effects.len() != 2 {
        return Ok(None);
    }
    let first_is_tap = matches!(
        &effects[0],
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Tap { .. } | SubjectVerbActionAst::TapAll { .. },
            ..
        })
    );
    if !first_is_tap {
        return Ok(None);
    }
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::DealDamage { amount, target, .. },
        ..
    }) = &mut effects[1]
    else {
        return Ok(None);
    };
    if !matches!(amount, Value::X) {
        return Ok(None);
    }

    *amount = Value::EventValue(EventValueSpec::Amount);
    if word_slice_contains_phrase(
        &crate::runtime_backend::token_word_refs(stripped),
        SENTENCE_TO_THE_PLAYER_PHRASE,
    ) {
        *target = TargetAst::Player(PlayerFilter::Active, None);
    }
    Ok(Some(effects))
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

fn sentence_has_return_each_creature_that_isnt_list_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    effect_grammar::has_return_each_creature_that_isnt_list_clause_sentence_lexed(tokens)
}

fn sentence_has_unsupported_negated_untap_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    effect_grammar::has_unsupported_negated_untap_clause_sentence_lexed(tokens)
}





fn parse_it_is_aura_enchantment_sentence_lexed(tokens: &[OwnedLexToken]) -> Option<Vec<EffectAst>> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    let tail = if let Some(prefix) =
        word_slice_matching_prefix(&words, SENTENCE_ITS_AN_PREFIXES)
    {
        clause.after_words(prefix.len())?
    } else if word_slice_starts_with(&words, SENTENCE_IT_IS_AN_PREFIX) {
        clause.after_words(3)?
    } else {
        return None;
    };
    if !word_slice_starts_with(&tail.word_refs(), SENTENCE_AURA_ENCHANT_CREATURE_PREFIX) {
        return None;
    }

    let attachment_filter = if word_slice_starts_with(
        &tail.after_words(5)?.word_refs(),
        SENTENCE_YOU_CONTROL_PREFIX,
    ) {
        ObjectFilter::creature().you_control()
    } else {
        ObjectFilter::creature()
    };
    let mut effects = vec![EffectAst::subject_verb_become_aura_enchantment(
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic())),
        attachment_filter,
        Until::Forever,
    )];

    if word_slice_contains_any_phrase(&tail.word_refs(), SENTENCE_LOSES_ALL_ABILITIES_PHRASES) {
        effects.push(EffectAst::subject_verb_remove_abilities_all(
            ObjectFilter::default(),
            Vec::new(),
            Until::Forever,
        ));
    }
    Some(effects)
}

pub(crate) fn parse_effect_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    stacker::maybe_grow(8 * 1024 * 1024, 16 * 1024 * 1024, || {
        parse_effect_sentence_lexed_inner(tokens)
    })
}

fn parse_effect_sentence_lexed_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn search_followup_shuffle_player(effect: &EffectAst) -> Option<PlayerAst> {
        match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SearchLibrary { player, .. },
                ..
            }) => Some(*player),
            _ => None,
        }
    }

    fn normalize_search_followup_shuffles(effects: &mut [EffectAst]) {
        for idx in 0..effects.len() {
            let is_default_shuffle = matches!(
                effects.get(idx),
                Some(EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::ShuffleLibrary,
                }))
                    if matches!(subject.player, PlayerAst::You | PlayerAst::Implicit)
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
                if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    subject,
                    action: SubjectVerbActionAst::ShuffleLibrary,
                }) = &mut effects[idx]
                {
                    subject.player = search_player;
                }
            }
        }
    }

    let sentence_words = crate::runtime_backend::token_word_refs(tokens);
    // "If <player refs> would gain life this turn, that player gains no life
    // instead." == a can't-gain-life window for those players (Flames of the
    // Blood Hand). Intercept before leading-if splitting since the would-gain
    // predicate isn't a state condition.
    if sentence_words.first() == Some(&"if")
        && sentence_words
            .windows(5)
            .any(|window| window == ["would", "gain", "life", "this", "turn"])
        && (sentence_words.ends_with(&["gains", "no", "life", "instead"])
            || sentence_words.ends_with(&["gain", "no", "life", "instead"]))
    {
        return Ok(vec![EffectAst::subject_verb_cant(
            crate::effect::Restriction::gain_life(crate::target::PlayerFilter::DamagedPlayer),
            crate::effect::Until::EndOfTurn,
            None,
        )]);
    }
    if let Some(effect) = parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(tokens)
    {
        return Ok(vec![effect]);
    }
    if let Some(effect) = parse_source_and_blocked_creatures_top_library_shuffle_sentence(tokens) {
        return Ok(vec![effect]);
    }
    if sentence_words.starts_with(&["at", "the", "beginning", "of"])
        && sentence_words
            .windows(3)
            .any(|window| window == ["next", "end", "step"])
    {
        if let Some(effects) =
            parse_sentence_delayed_next_step_unless_pays(SubjectVerbPrimitiveClause::new(tokens))?
        {
            return Ok(effects);
        }
        if let Some(effects) = parse_delayed_until_next_end_step_sentence(tokens)? {
            return Ok(effects);
        }
    }
    if let Some(effects) = parse_it_is_aura_enchantment_sentence_lexed(tokens) {
        return Ok(effects);
    }
    let sacrifice_counted_prefix =
        word_slice_starts_with_any(&sentence_words, SENTENCE_SACRIFICE_COUNTED_PREFIXES);
    let sacrifice_delayed_lifecycle =
        word_slice_contains_any_phrase(&sentence_words, SENTENCE_DELAYED_LIFECYCLE_PHRASES);
    if word_slice_at_is(&sentence_words, 0, SENTENCE_SACRIFICE_WORD)
        && !sacrifice_counted_prefix
        && !sacrifice_delayed_lifecycle
    {
        let mut effects = super::parse_effect_chain_inner_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_starts_with(&sentence_words, SENTENCE_AT_THIS_PREFIX)
        && let Some(end_idx) =
            word_slice_find_phrase_start(&sentence_words, SENTENCE_END_OF_COMBAT_PREFIX)
        && let Some(end_token_idx) = token_index_for_word_index(tokens, end_idx)
        && word_slice_contains_word(
            &crate::runtime_backend::token_word_refs(&tokens[..end_token_idx]),
            SENTENCE_NEXT_WORD,
        )
    {
        let Some(remainder_start) = token_index_for_word_index(tokens, end_idx + 3) else {
            return Err(CardTextError::ParseError(
                "end-of-combat delayed trigger missing effect payload".to_string(),
            ));
        };
        let remainder = trim_commas(&tokens[remainder_start..]);
        if remainder.is_empty() {
            return Err(CardTextError::ParseError(
                "end-of-combat delayed trigger missing effect payload".to_string(),
            ));
        }
        let effects = parse_effect_sentence_lexed_inner(&remainder)?;
        return Ok(vec![EffectAst::DelayedUntilEndOfCombat { effects }]);
    }

    if let Some(effect) = parse_additional_phase_sentence(tokens) {
        return Ok(vec![effect]);
    }

    let leading_if_replacement_shape = token_slice_first_is(tokens, "if")
        && word_slice_contains_word(&sentence_words, SENTENCE_WOULD_WORD);
    if token_slice_first_is(tokens, "if")
        && !leading_if_replacement_shape
        && let Ok(Some(mut effects)) =
            parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)
        && matches!(effects.as_slice(), [EffectAst::Conditional { .. }])
    {
        apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
        normalize_search_followup_shuffles(&mut effects);
        return Ok(effects);
    }

    if let Some(effect) = parse_vote_subject_verb(tokens)? {
        crate::parse_trace::event(
            "effect-route: subject-verb verb=Vote subject=explicit recognizer=vote-procedure",
        );
        return Ok(vec![effect]);
    }

    if let Some((route, mut effects)) = parse_top_level_subject_verb_recognition(tokens)? {
        crate::parse_trace::event(format!("effect-route: {route}"));
        normalize_search_followup_shuffles(&mut effects);
        return Ok(effects);
    }
    let mut effects = parse_effect_sentence_inner_lexed(tokens)?;
    apply_trailing_counter_constraint_to_destroy_all(&mut effects, tokens);
    normalize_search_followup_shuffles(&mut effects);
    Ok(effects)
}

fn source_and_creatures_blocked_by_source_words(words: &[&str]) -> bool {
    words
        == [
            "this", "creature", "and", "each", "creature", "it's", "blocking",
        ]
}

fn parse_source_and_blocked_creatures_top_library_shuffle_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if !token_slice_first_is(tokens, "put") {
        return None;
    }
    let (target_slice, after_on_top_of) =
        grammar::split_lexed_once_on_separator(&tokens[1..], || {
            use winnow::Parser as _;
            grammar::phrase(&["on", "top", "of"]).void()
        })?;
    let target_tokens = trim_commas(target_slice);
    let target_words = crate::runtime_backend::token_word_refs(&target_tokens);
    if !source_and_creatures_blocked_by_source_words(&target_words) {
        return None;
    }

    let destination_words = crate::runtime_backend::token_word_refs(after_on_top_of);
    let has_owner_library = destination_words
        .iter()
        .any(|word| matches!(*word, "owner" | "owners" | "owner's" | "owners'"))
        && destination_words
            .iter()
            .any(|word| matches!(*word, "library" | "libraries"));
    if !has_owner_library
        || !word_slice_contains_phrase(&destination_words, &["then", "those", "players", "shuffle"])
    {
        return None;
    }

    let mut blocked_creature = ObjectFilter::creature();
    blocked_creature.blocked_by_source = true;
    let mut moved_objects = ObjectFilter::default();
    moved_objects.any_of = vec![ObjectFilter::source(), blocked_creature];

    Some(EffectAst::ForEachObject {
        filter: moved_objects,
        effects: vec![
            EffectAst::subject_verb_move_to_zone(
                TargetAst::Tagged(TagKey::from(IT_TAG), None),
                Zone::Library,
                true,
                crate::cards::builders::ReturnControllerAst::Preserve,
                false,
                None,
            ),
            EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::ItsOwner,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
        ],
    })
}

fn parse_put_cards_from_single_graveyard_on_bottom_owner_library_sentence(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    if !token_slice_first_is(tokens, "put") {
        return None;
    }
    let (count, count_used) = parse_value(&tokens[1..])?;
    let Value::Fixed(count) = count else {
        return None;
    };
    if count <= 0 {
        return None;
    }
    let card_idx = 1 + count_used;
    if !tokens
        .get(card_idx)
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "card" | "cards"))
    {
        return None;
    }

    let tail = trim_edge_punctuation(&tokens[card_idx + 1..]);
    let tail_words = crate::runtime_backend::token_word_refs(&tail);
    if !word_slice_starts_with(
        &tail_words,
        &["from", "a", "single", "graveyard", "on", "the", "bottom", "of"],
    ) {
        return None;
    }
    let owner_tail = &tail_words[8..];
    if !owner_tail
        .iter()
        .any(|word| matches!(*word, "owner" | "owners" | "owner's" | "owners'"))
        || !owner_tail.iter().any(|word| *word == "library")
        || !owner_tail
            .first()
            .is_some_and(|word| matches!(*word, "its" | "their"))
    {
        return None;
    }

    let filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .single_graveyard();
    Some(EffectAst::subject_verb_move_to_zone(
        TargetAst::WithCount(
            Box::new(TargetAst::Object(filter, None, None)),
            ChoiceCount::exactly(count as usize),
        ),
        Zone::Library,
        false,
        crate::cards::builders::ReturnControllerAst::Preserve,
        false,
        None,
    ))
}

fn parse_effect_sentence_with_where_x_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn replace_search_filter_x(effect: &mut EffectAst, replacement: &Value) {
        let (filter, count, count_value) = match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::SearchLibrary {
                        filter,
                        count,
                        count_value,
                        ..
                    },
                ..
            }) => (filter, count, count_value),
            EffectAst::ChooseObjects {
                filter,
                count,
                count_value,
                ..
            }
            | EffectAst::ChooseObjectsAcrossZones {
                filter,
                count,
                count_value,
                ..
            } => (filter, count, count_value),
            _ => return,
        };

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

    fn bind_dynamic_target_count(target: &mut TargetAst, replacement: &Value) {
        match target {
            TargetAst::WithCount(inner, count) => {
                bind_dynamic_target_count(inner, replacement);
                if count.is_dynamic_x() {
                    let old = std::mem::replace(target, TargetAst::Source(None));
                    if let TargetAst::WithCount(inner, count) = old {
                        *target = TargetAst::WithCountValue(inner, count, replacement.clone());
                    }
                }
            }
            TargetAst::WithCountValue(inner, _, value) => {
                bind_dynamic_target_count(inner, replacement);
                if matches!(value, Value::X) {
                    *value = replacement.clone();
                }
            }
            _ => {}
        }
    }

    fn bind_dynamic_target_counts(effect: &mut EffectAst, replacement: &Value) {
        let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
            return;
        };
        match action {
            SubjectVerbActionAst::Explore { target }
            | SubjectVerbActionAst::Endure { target, .. }
            | SubjectVerbActionAst::Connive { target, .. }
            | SubjectVerbActionAst::ExchangeTextBoxes { target }
            | SubjectVerbActionAst::Attach { target, .. }
            | SubjectVerbActionAst::Unattach { object: target }
            | SubjectVerbActionAst::MayMoveToZone { target, .. }
            | SubjectVerbActionAst::ReturnToBattlefield { target, .. }
            | SubjectVerbActionAst::ExileUntilSourceLeaves { target, .. }
            | SubjectVerbActionAst::MoveToZone { target, .. }
            | SubjectVerbActionAst::MoveToLibraryTopOrBottomChoice { target }
            | SubjectVerbActionAst::TargetOnly { target }
            | SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::SetBasePowerToughness { target, .. }
            | SubjectVerbActionAst::BecomeBasePtCreature { target, .. }
            | SubjectVerbActionAst::SetBasePower { target, .. }
            | SubjectVerbActionAst::PumpForEach { target, .. }
            | SubjectVerbActionAst::PumpByLastEffect { target, .. }
            | SubjectVerbActionAst::AddCardTypes { target, .. }
            | SubjectVerbActionAst::RemoveCardTypes { target, .. }
            | SubjectVerbActionAst::AddSubtypes { target, .. }
            | SubjectVerbActionAst::AddColors { target, .. }
            | SubjectVerbActionAst::AddAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::RemoveAllSubtypesOfFamily { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandType { target, .. }
            | SubjectVerbActionAst::SetColors { target, .. }
            | SubjectVerbActionAst::MakeColorless { target, .. }
            | SubjectVerbActionAst::BecomeBasicLandTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeCreatureTypeChoice { target, .. }
            | SubjectVerbActionAst::BecomeColorChoice { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::RemoveAbilitiesFromTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. }
            | SubjectVerbActionAst::RedirectNextTimeDamageToSource { target, .. }
            | SubjectVerbActionAst::RedirectAllDamageThisTurnBySourceToSourceController {
                source: target,
            }
            | SubjectVerbActionAst::RetargetStackObject { target, .. }
            | SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::DealDistributedDamage { target, .. }
            | SubjectVerbActionAst::Tap { target }
            | SubjectVerbActionAst::Untap { target } => {
                bind_dynamic_target_count(target, replacement)
            }
            SubjectVerbActionAst::RedirectNextDamageFromSourceToTarget {
                protected_target,
                destination_target,
                ..
            } => {
                if let Some(target) = protected_target {
                    bind_dynamic_target_count(target, replacement);
                }
                if let Some(target) = destination_target {
                    bind_dynamic_target_count(target, replacement);
                }
            }
            SubjectVerbActionAst::Fight {
                creature1,
                creature2,
            }
            | SubjectVerbActionAst::DealDamageEqualToPower {
                source: creature1,
                target: creature2,
                ..
            }
            | SubjectVerbActionAst::BecomeCopy {
                target: creature1,
                source: creature2,
                ..
            } => {
                bind_dynamic_target_count(creature1, replacement);
                bind_dynamic_target_count(creature2, replacement);
            }
            SubjectVerbActionAst::CreateTokenCopyFromSource { source, .. } => {
                bind_dynamic_target_count(source, replacement);
            }
            SubjectVerbActionAst::CreateTokenWithMods {
                attached_to: Some(target),
                ..
            } => bind_dynamic_target_count(target, replacement),
            _ => {}
        }
    }

    let clause_word_storage = DispatchInnerNormalizedWords::new(tokens);
    let clause_words = clause_word_storage.to_word_refs();
    let clause_display = render_token_slice(tokens).trim().to_string();
    let Some(where_idx) =
        word_slice_find_phrase_start(clause_words.as_slice(), SENTENCE_WHERE_X_IS_PREFIX)
    else {
        return parse_effect_sentence_inner_lexed(tokens);
    };
    let where_token_idx = token_index_for_word_index(tokens, where_idx)
        .or_else(|| find_token_word_sequence(tokens, &["where", "x", "is"]));
    let Some(where_token_idx) = where_token_idx else {
        return Err(CardTextError::ParseError(format!(
            "unsupported where-x clause (clause: '{}')",
            &clause_display
        )));
    };
    let where_tokens = &tokens[where_token_idx..];
    let where_segments = split_lexed_slices_on_commas_or_semicolons(where_tokens);
    let comma_tail_has_effect_clause = where_segments.iter().skip(1).any(|segment| {
        let words = crate::runtime_backend::token_word_refs(segment);
        words.iter().any(|word| *word == "then") || super::find_verb(segment).is_some()
    });
    let full_where_is_count_value = !comma_tail_has_effect_clause
        && crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_value(
            where_tokens,
        )
        .is_some();
    let primary_where_tokens = if full_where_is_count_value {
        where_tokens
    } else {
        where_segments.first().copied().unwrap_or(where_tokens)
    };
    let trailing_after_where = if !full_where_is_count_value && where_segments.len() > 1 {
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

    if let Some(effects) =
        parse_target_deals_power_damage_to_other_and_self_where_x(tokens, &clause_words, where_idx)?
    {
        return Ok(effects);
    }
    if let Some(effects) =
        parse_tap_then_damage_for_number_tapped_this_way(&stripped, &where_words)?
    {
        return Ok(effects);
    }

    let mut prelude_effects = Vec::new();
    let where_value = if let Some((choice_effect, value)) =
        parse_where_x_commander_mana_value_choice(&where_words)
    {
        prelude_effects.push(choice_effect);
        value
    } else if matches!(
        where_words.get(3..),
        Some(["the", "power", "of", "the", "creature", "tapped", "this", "way"])
            | Some(["power", "of", "the", "creature", "tapped", "this", "way"])
    ) {
        Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(
            "tap_cost_0",
        ))))
    } else if let Some(value) = parse_where_x_prior_effect_number_value(&where_words) {
        value
    } else {
        let stripped_references_target = word_slice_contains_word(
            &crate::runtime_backend::token_word_refs(&stripped),
            SENTENCE_TARGET_WORD,
        );
        match where_words.get(3..) {
            Some(["its", "power"]) => {
                if stripped_references_target {
                    Value::PowerOf(Box::new(crate::target::ChooseSpec::target(
                        crate::target::ChooseSpec::Object(ObjectFilter::default()),
                    )))
                } else {
                    Value::SourcePower
                }
            }
            Some(["its", "toughness"]) => {
                if stripped_references_target {
                    Value::ToughnessOf(Box::new(crate::target::ChooseSpec::target(
                        crate::target::ChooseSpec::Object(ObjectFilter::default()),
                    )))
                } else {
                    Value::SourceToughness
                }
            }
            Some(["its", "mana", "value"]) => {
                Value::ManaValueOf(Box::new(if stripped_references_target {
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
            Some(["that", "spell", "mana", "value"])
            | Some(["that", "spell's", "mana", "value"])
            | Some(["that", "spells", "mana", "value"]) => Value::ManaValueOf(Box::new(
                crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG)),
            )),
            Some(["that", "creatures", "power"]) => {
                Value::PowerOf(Box::new(if stripped_references_target {
                    crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                        ObjectFilter::default(),
                    ))
                } else {
                    crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))
                }))
            }
            Some(["that", "creatures", "toughness"]) => {
                Value::ToughnessOf(Box::new(if stripped_references_target {
                    crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                        ObjectFilter::default(),
                    ))
                } else {
                    crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))
                }))
            }
            Some(["that", "creatures", "mana", "value"]) => {
                Value::ManaValueOf(Box::new(if stripped_references_target {
                    crate::target::ChooseSpec::target(crate::target::ChooseSpec::Object(
                        ObjectFilter::default(),
                    ))
                } else {
                    crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))
                }))
            }
            Some(
                [
                    "the",
                    "number",
                    "of",
                    "times",
                    "it's",
                    "been",
                    "cast",
                    "from",
                    "the",
                    "command",
                    "zone",
                    "this",
                    "game",
                ],
            )
            | Some(
                [
                    "the",
                    "number",
                    "of",
                    "times",
                    "its",
                    "been",
                    "cast",
                    "from",
                    "the",
                    "command",
                    "zone",
                    "this",
                    "game",
                ],
            )
            | Some(
                [
                    "the",
                    "number",
                    "of",
                    "times",
                    "it",
                    "has",
                    "been",
                    "cast",
                    "from",
                    "the",
                    "command",
                    "zone",
                    "this",
                    "game",
                ],
            ) => Value::CommanderCastCount(PlayerFilter::You),
            Some(["the", "power", "of", "the", "creature", "tapped", "this", "way"])
            | Some(["power", "of", "the", "creature", "tapped", "this", "way"]) => {
                Value::PowerOf(Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(
                    "tap_cost_0",
                ))))
            }
            Some(["the", "sacrificed", sacrificed_kind, "mana", "value"])
            | Some(["sacrificed", sacrificed_kind, "mana", "value"]) => {
                let sacrificed_kind = sacrificed_kind.trim_end_matches('s');
                Value::ManaValueOf(Box::new(
                    crate::target::ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0"))
                        .with_surface_hint(crate::target::ChooseSpecSurfaceHint::SourceReference(
                            crate::target::SourceReferenceSurface::ThisPermanentType(format!(
                                "the sacrificed {sacrificed_kind}"
                            )),
                        )),
                ))
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
            _ => {
                let activation_time_trimmed =
                    crate::runtime_backend::lexer::find_token_word(primary_where_tokens, "as")
                    .map(|token_idx| trim_edge_punctuation(&primary_where_tokens[..token_idx]));
                let specific_where_value =
                    crate::runtime_backend::front_end::grammar::values::parse_players_who_control_more_than_you_value_lexed(
                        primary_where_tokens,
                    )
                    .or_else(|| {
                        crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_plus_or_minus_fixed_value(
                            primary_where_tokens,
                        )
                    })
                    .or_else(|| {
                        crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_different_powers_filter_value(
                            primary_where_tokens,
                        )
                    });
                let number_of_filter_value = specific_where_value.or_else(|| {
                    crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_value(
                        primary_where_tokens,
                    )
                })
                    .or_else(|| {
                        activation_time_trimmed.as_deref().and_then(
                            crate::runtime_backend::families::keyword_static::parse_where_x_is_number_of_filter_value,
                        )
                    });
                if let Some(value) = number_of_filter_value {
                    value
                } else if let Some(trimmed) = activation_time_trimmed.as_deref() {
                    parse_value_binding_clause_lexed(trimmed).ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported where-x clause (clause: '{}')",
                            &clause_display
                        ))
                    })?
                } else {
                    parse_value_binding_clause_lexed(&primary_where_tokens).ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported where-x clause (clause: '{}')",
                            &clause_display
                        ))
                    })?
                }
            }
        }
    }
    .with_surface_hint(ValueSurfaceHint::WhereXIs);

    let search_like = word_slice_at_is(&stripped_words, 0, SENTENCE_SEARCH_WORD);
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
    replace_unbound_x_in_effects_anywhere(&mut effects, &where_value, &clause_display)?;
    for effect in &mut effects {
        replace_search_filter_x(effect, &where_value);
        bind_dynamic_target_counts(effect, &where_value);
    }
    if !prelude_effects.is_empty() {
        prelude_effects.append(&mut effects);
        return Ok(prelude_effects);
    }
    Ok(effects)
}

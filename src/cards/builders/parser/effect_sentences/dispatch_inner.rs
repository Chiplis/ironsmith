use super::super::grammar::effects::{
    parse_cant_effect_sentence_with_grammar_entrypoint_lexed,
    parse_conditional_sentence_family_lexed,
    parse_search_library_sentence_with_grammar_entrypoint_lexed,
};
use super::super::grammar::primitives::{
    self as grammar, TokenWordView, split_lexed_slices_on_and, split_lexed_slices_on_comma,
    split_lexed_slices_on_commas_or_semicolons,
};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed,
};
use super::super::keyword_static::{
    parse_ability_line, parse_pt_modifier, parse_where_x_value_clause,
    parse_where_x_value_clause_lexed,
};
use super::super::lexer::OwnedLexToken;
use super::super::object_filters::{
    find_word_slice_phrase_start, parse_object_filter, parse_object_filter_lexed,
};
use super::super::rule_engine::{
    LexClauseView, LexRuleDef, LexRuleIndex, LexUnsupportedDiagnoser, LexUnsupportedRuleDef,
};
use super::super::token_primitives::{
    find_index, find_window_by, iter_contains, rfind_index, slice_contains, slice_ends_with,
    slice_starts_with,
};
use super::super::util::{
    is_article, is_source_reference_words, parse_card_type, parse_subject, parse_target_phrase,
    parse_value, token_index_for_word_index, words,
};
use super::super::{
    is_activate_only_restriction_sentence_lexed, is_trigger_only_restriction_sentence_lexed,
};
use super::sentence_helpers::*;
use super::zone_handlers::collapse_leading_signed_pt_modifier_tokens;
use super::{TokenCopyFollowup, parse_effect_chain_lexed, parse_simple_gain_ability_clause};

const EACH_PLAYER_PREFIXES: &[&[&str]] = &[&["each", "player"]];
const EACH_PLAYER_EXILES_ALL_PREFIXES: &[&[&str]] = &[&["each", "player", "exiles", "all"]];
const THAT_MUCH_PREFIXES: &[&[&str]] = &[&["that", "much"]];
const PREVENT_DAMAGE_BY_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "by"]];
const PREVENT_DAMAGE_TO_AND_BY_PREFIXES: &[&[&str]] =
    &[&["that", "would", "be", "dealt", "to", "and", "dealt", "by"]];
const PREVENT_DAMAGE_TO_PREFIXES: &[&[&str]] = &[&["that", "would", "be", "dealt", "to"]];
const EXILE_PREFIXES: &[&[&str]] = &[&["exile"]];
const RETURN_EACH_CREATURE_ISNT_PREFIXES: &[&[&str]] =
    &[&["return", "each", "creature", "that", "isnt"]];
const EXILE_ALL_CARDS_FROM_PREFIXES: &[&[&str]] = &[&["exile", "all", "cards", "from"]];
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, ExtraTurnAnchorAst, IT_TAG, LineAst, PlayerAst, SubjectAst, TagKey,
    TargetAst, TextSpan, TriggerSpec, Verb,
};
use crate::effect::{ChoiceCount, EventValueSpec, Until, Value};
use crate::object::CounterType;
use crate::target::{
    ChooseSpec, ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation,
};
use crate::types::CardType;
use crate::zone::Zone;

type DispatchInnerNormalizedWords<'a> = TokenWordView<'a>;

fn contains_dispatch_inner_phrase(words: &[&str], phrase: &[&str]) -> bool {
    find_dispatch_inner_phrase_start(words, phrase).is_some()
}

fn find_dispatch_inner_phrase_start(words: &[&str], phrase: &[&str]) -> Option<usize> {
    find_word_slice_phrase_start(words, phrase)
}

const SENTENCE_PRIMITIVE_RULE_HEADS: &[&str] = &[
    "if",
    "you",
    "target",
    "each",
    "for",
    "return",
    "destroy",
    "exile",
    "counter",
    "draw",
    "put",
    "gets",
    "sacrifice",
    "take",
    "earthbend",
    "enchant",
    "cant",
    "prevent",
    "gain",
    "search",
    "shuffle",
    "look",
    "play",
    "vote",
    "after",
    "reveal",
    "damage",
    "unless",
    "monstrosity",
];

macro_rules! sentence_unsupported_adapters_lexed {
    ($(($adapter:ident, $predicate:ident)),* $(,)?) => {
        $(
            fn $adapter(view: &LexClauseView<'_>) -> bool {
                let words = view.words.to_word_refs();
                $predicate(words.as_slice(), view.tokens)
            }
        )*
    };
}

fn next_spell_grant_player_ast(filter: &ObjectFilter) -> Option<PlayerAst> {
    match filter.cast_by.as_ref()? {
        crate::target::PlayerFilter::You => Some(PlayerAst::You),
        crate::target::PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        crate::target::PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        crate::target::PlayerFilter::Target(base) => match base.as_ref() {
            crate::target::PlayerFilter::Any => Some(PlayerAst::Target),
            crate::target::PlayerFilter::Opponent => Some(PlayerAst::TargetOpponent),
            _ => None,
        },
        _ => None,
    }
}

fn next_spell_grant_shared_cast_suffix<'a>(words: &'a [&'a str]) -> Option<&'a [&'a str]> {
    for suffix in [
        &["you", "cast"][..],
        &["they", "cast"][..],
        &["that", "player", "cast"][..],
        &["target", "player", "cast"][..],
        &["target", "opponent", "cast"][..],
        &["opponent", "cast"][..],
        &["opponents", "cast"][..],
    ] {
        if ends_with_words(words, suffix) {
            return Some(suffix);
        }
    }
    None
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

fn contains_any_word_window(words: &[&str], patterns: &[&[&str]]) -> bool {
    patterns
        .iter()
        .any(|pattern| contains_word_window(words, pattern))
}

fn find_token_word_window(
    tokens: &[OwnedLexToken],
    expected: &'static [&'static str],
) -> Option<usize> {
    super::super::grammar::primitives::find_phrase_start(tokens, expected)
}

fn starts_with_words(words: &[&str], prefix: &[&str]) -> bool {
    words.len() >= prefix.len()
        && words[..prefix.len()]
            .iter()
            .zip(prefix.iter())
            .all(|(word, expected)| word == expected)
}

fn ends_with_words<'a>(words: &'a [&'a str], suffix: &[&str]) -> bool {
    words.len() >= suffix.len()
        && words[words.len() - suffix.len()..]
            .iter()
            .zip(suffix.iter())
            .all(|(word, expected)| word == expected)
}

fn parse_next_spell_grant_ability(
    words: &[&str],
) -> Option<crate::cards::builders::GrantedAbilityAst> {
    let tokens = synth_word_tokens(words);
    let actions = parse_ability_line(&tokens)?;
    let [action] = actions.as_slice() else {
        return None;
    };
    if !action.lowers_to_static_ability() {
        return None;
    }
    Some(action.clone().into())
}

fn synth_word_tokens(words: &[&str]) -> Vec<OwnedLexToken> {
    words
        .iter()
        .map(|word| OwnedLexToken::word((*word).to_string(), TextSpan::synthetic()))
        .collect()
}

fn parse_next_spell_subject_filter(words: &[&str]) -> Result<Option<ObjectFilter>, CardTextError> {
    let tokens = synth_word_tokens(words);
    let filter = parse_object_filter_lexed(&tokens, false)?;
    if filter.cast_by.is_none() {
        return Ok(None);
    }
    Ok(Some(filter))
}

fn parse_next_spell_grant_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_word_storage = DispatchInnerNormalizedWords::new(tokens);
    let clause_words = clause_word_storage.to_word_refs();
    if !slice_starts_with(&clause_words, &["the", "next"]) {
        return Ok(None);
    }

    if let Some(have_idx) = find_dispatch_inner_phrase_start(&clause_words, &["each", "have"]) {
        let subject_words = &clause_words[..have_idx];
        let ability_words = &clause_words[have_idx + 2..];
        let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
            return Ok(None);
        };
        if !slice_ends_with(subject_words, &["this", "turn"]) {
            return Ok(None);
        }
        let subject_without_turn = &subject_words[..subject_words.len() - 2];
        let Some(shared_cast_words) = next_spell_grant_shared_cast_suffix(subject_without_turn)
        else {
            return Ok(None);
        };
        let shared_prefix =
            &subject_without_turn[2..subject_without_turn.len() - shared_cast_words.len()];
        let Some(split_idx) =
            find_dispatch_inner_phrase_start(shared_prefix, &["and", "the", "next"])
        else {
            return Ok(None);
        };
        let first_subject = &shared_prefix[..split_idx];
        let second_subject = &shared_prefix[split_idx + 3..];
        if first_subject.is_empty() || second_subject.is_empty() {
            return Ok(None);
        }

        let first_filter_words = [first_subject, shared_cast_words].concat();
        let second_filter_words = [second_subject, shared_cast_words].concat();
        let Some(first_filter) = parse_next_spell_subject_filter(&first_filter_words)? else {
            return Ok(None);
        };
        let Some(second_filter) = parse_next_spell_subject_filter(&second_filter_words)? else {
            return Ok(None);
        };
        let Some(player) = next_spell_grant_player_ast(&first_filter) else {
            return Ok(None);
        };

        return Ok(Some(vec![
            EffectAst::GrantNextSpellAbilityThisTurn {
                player,
                filter: first_filter,
                ability: ability.clone(),
            },
            EffectAst::GrantNextSpellAbilityThisTurn {
                player,
                filter: second_filter,
                ability,
            },
        ]));
    }

    let Some(has_idx) = find_index(&clause_words, |word| matches!(*word, "has" | "have")) else {
        return Ok(None);
    };
    let subject_words = &clause_words[..has_idx];
    let ability_words = &clause_words[has_idx + 1..];
    let Some(ability) = parse_next_spell_grant_ability(ability_words) else {
        return Ok(None);
    };
    if !slice_ends_with(subject_words, &["this", "turn"]) {
        return Ok(None);
    }

    let filter_words = &subject_words[2..subject_words.len() - 2];
    let Some(filter) = parse_next_spell_subject_filter(filter_words)? else {
        return Ok(None);
    };
    let Some(player) = next_spell_grant_player_ast(&filter) else {
        return Ok(None);
    };
    Ok(Some(vec![EffectAst::GrantNextSpellAbilityThisTurn {
        player,
        filter,
        ability,
    }]))
}

fn sentence_has_enters_as_copy_rule_lexed(view: &LexClauseView<'_>) -> bool {
    is_enters_as_copy_clause_tokens(view.tokens)
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

const SENTENCE_UNSUPPORTED_RULES_LEXED: [LexUnsupportedRuleDef; 30] = [
    LexUnsupportedRuleDef {
        id: "enters-as-copy",
        priority: 20,
        heads: &[],
        shape_mask: 0,
        message: "unsupported enters-as-copy replacement clause",
        predicate: sentence_has_enters_as_copy_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "each-player-lose-discard-sacrifice-chain",
        priority: 100,
        heads: &["each"],
        shape_mask: 0,
        message: "unsupported each-player lose/discard/sacrifice chain clause",
        predicate: sentence_has_each_player_lose_discard_sacrifice_chain_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "each-player-exile-sacrifice-return-this-way",
        priority: 110,
        heads: &["each"],
        shape_mask: 0,
        message: "unsupported each-player exile/sacrifice/return-this-way clause",
        predicate: sentence_has_each_player_exile_sacrifice_return_exiled_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "put-one-into-hand-rest-zone",
        priority: 115,
        heads: &["put", "then"],
        shape_mask: 0,
        message: "unsupported put-into-hand with rest clause",
        predicate: sentence_has_put_one_of_them_into_hand_rest_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "lose-all-abilities-with-becomes",
        priority: 120,
        heads: &["target", "that", "it", "this", "creatures"],
        shape_mask: 0,
        message: "unsupported loses-all-abilities with becomes clause",
        predicate: sentence_has_loses_all_abilities_with_becomes_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "spent-to-cast-conditional",
        priority: 130,
        heads: &["if", "unless", "when", "as"],
        shape_mask: 0,
        message: "unsupported spent-to-cast conditional clause",
        predicate: sentence_has_spent_to_cast_this_spell_without_condition_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "would-enter-instead",
        priority: 140,
        heads: &["if", "that", "it", "this"],
        shape_mask: 0,
        message: "unsupported would-enter replacement clause",
        predicate: sentence_has_would_enter_instead_replacement_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "different-mana-value-constraint",
        priority: 150,
        heads: &[],
        shape_mask: 0,
        message: "unsupported different-mana-value constraint clause",
        predicate: sentence_has_different_mana_value_constraint_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "most-common-color-constraint",
        priority: 160,
        heads: &["choose", "destroy", "exile", "return"],
        shape_mask: 0,
        message: "unsupported most-common-color constraint clause",
        predicate: sentence_has_most_common_color_constraint_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "power-vs-count-constraint",
        priority: 170,
        heads: &["if", "target", "destroy", "exile", "return"],
        shape_mask: 0,
        message: "unsupported power-vs-count conditional clause",
        predicate: sentence_has_power_vs_count_constraint_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "put-into-graveyards-from-battlefield-this-turn",
        priority: 180,
        heads: &["for", "choose", "target", "destroy"],
        shape_mask: 0,
        message: "unsupported put-into-graveyards-from-battlefield count clause",
        predicate: sentence_has_put_into_graveyards_from_battlefield_this_turn_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "phase-out-until-leaves",
        priority: 190,
        heads: &["phase", "target", "it", "that"],
        shape_mask: 0,
        message: "unsupported phase-out-until-leaves clause",
        predicate: sentence_has_phase_out_until_leaves_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "same-name-as-another-in-hand",
        priority: 200,
        heads: &["target", "choose", "discard"],
        shape_mask: 0,
        message: "unsupported same-name-as-another-in-hand discard clause",
        predicate: sentence_has_same_name_as_another_in_hand_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "for-each-mana-from-spent",
        priority: 210,
        heads: &["for"],
        shape_mask: 0,
        message: "unsupported for-each-mana-from-spent clause",
        predicate: sentence_has_for_each_mana_from_spent_to_cast_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "when-you-sacrifice-this-way",
        priority: 230,
        heads: &["when"],
        shape_mask: 0,
        message: "unsupported when-you-sacrifice-this-way clause",
        predicate: sentence_has_when_you_sacrifice_this_way_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "sacrifice-any-number-then-draw-that-many",
        priority: 240,
        heads: &["sacrifice", "each", "target", "you"],
        shape_mask: 0,
        message: "unsupported sacrifice-any-number-then-draw-that-many clause",
        predicate: sentence_has_sacrifice_any_number_then_draw_that_many_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "greatest-mana-value",
        priority: 250,
        heads: &["choose", "destroy", "exile", "return"],
        shape_mask: 0,
        message: "unsupported greatest-mana-value selection clause",
        predicate: sentence_has_greatest_mana_value_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "least-power-among-creatures",
        priority: 260,
        heads: &["choose", "destroy", "exile", "return"],
        shape_mask: 0,
        message: "unsupported least-power-among-creatures selection clause",
        predicate: sentence_has_least_power_among_creatures_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "villainous-choice",
        priority: 270,
        heads: &["villainous"],
        shape_mask: 0,
        message: "unsupported villainous-choice clause",
        predicate: sentence_has_villainous_choice_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "divided-evenly",
        priority: 280,
        heads: &["divide", "deals", "deal", "distribute"],
        shape_mask: 0,
        message: "unsupported divided-evenly damage clause",
        predicate: sentence_has_divided_evenly_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "different-names",
        priority: 290,
        heads: &["choose", "target", "destroy", "exile"],
        shape_mask: 0,
        message: "unsupported different-names selection clause",
        predicate: sentence_has_different_names_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "chosen-at-random",
        priority: 300,
        heads: &["choose", "target", "discard", "exile"],
        shape_mask: 0,
        message: "unsupported chosen-at-random clause",
        predicate: sentence_has_chosen_at_random_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "defending-players-choice",
        priority: 320,
        heads: &["defending", "target", "of"],
        shape_mask: 0,
        message: "unsupported defending-players-choice clause",
        predicate: sentence_has_defending_players_choice_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "creature-token-player-planeswalker-target",
        priority: 330,
        heads: &["target"],
        shape_mask: 0,
        message: "unsupported creature-token/player/planeswalker target clause",
        predicate: sentence_has_target_creature_token_player_planeswalker_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "if-you-sacrifice-an-island-this-way",
        priority: 340,
        heads: &["if"],
        shape_mask: 0,
        message: "unsupported if-you-sacrifice-an-island-this-way clause",
        predicate: sentence_has_if_you_sacrifice_an_island_this_way_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "spent-to-cast-condition",
        priority: 350,
        heads: &["if", "unless", "when", "as"],
        shape_mask: 0,
        message: "unsupported spent-to-cast condition clause",
        predicate: sentence_has_spent_to_cast_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "face-down",
        priority: 360,
        heads: &["face", "turn", "cast", "exile", "manifest"],
        shape_mask: 0,
        message: "unsupported face-down clause",
        predicate: sentence_has_face_down_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "copy-spell-legendary-exception",
        priority: 370,
        heads: &["copy"],
        shape_mask: 0,
        message: "unsupported copy-spell legendary-exception clause",
        predicate: sentence_has_copy_spell_legendary_exception_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "return-each-creature-that-isnt-list",
        priority: 380,
        heads: &["return"],
        shape_mask: 0,
        message: "unsupported return-each-creature-that-isnt-list clause",
        predicate: sentence_has_return_each_creature_that_isnt_list_clause_rule_lexed,
    },
    LexUnsupportedRuleDef {
        id: "negated-untap",
        priority: 390,
        heads: &["this", "that", "target", "it", "creatures", "players"],
        shape_mask: 0,
        message: "unsupported negated untap clause",
        predicate: sentence_has_unsupported_negated_untap_clause_rule_lexed,
    },
];

const SENTENCE_UNSUPPORTED_DIAGNOSER_LEXED: LexUnsupportedDiagnoser =
    LexUnsupportedDiagnoser::new(&SENTENCE_UNSUPPORTED_RULES_LEXED);

fn diagnose_sentence_unsupported_lexed(tokens: &[OwnedLexToken]) -> Option<CardTextError> {
    if sentence_looks_like_supported_negated_untap_clause(tokens) {
        return None;
    }
    let view = LexClauseView::from_tokens(tokens);
    SENTENCE_UNSUPPORTED_DIAGNOSER_LEXED.diagnose(&view, "clause")
}

fn sentence_looks_like_supported_negated_untap_clause(tokens: &[OwnedLexToken]) -> bool {
    let words_storage = normalize_cant_words(tokens);
    let words = words_storage.iter().map(String::as_str).collect::<Vec<_>>();
    let has_negated_untap = find_window_by(&words, 3, |window| {
        window == ["dont", "untap", "during"] || window == ["doesnt", "untap", "during"]
    })
    .is_some();
    let has_controllers_untap_step = find_window_by(&words, 3, |window| {
        window == ["controllers", "untap", "step"] || window == ["controllers", "untap", "steps"]
    })
    .is_some();
    let has_tapped_duration = contains_dispatch_inner_phrase(&words, &["for", "as", "long", "as"])
        && slice_contains(&words, &"remains")
        && slice_contains(&words, &"tapped");
    has_negated_untap && has_controllers_untap_step && has_tapped_duration
}

fn parse_redirect_next_damage_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_redirect_next_damage_sentence(view.tokens)
}

fn parse_prevent_next_time_damage_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_prevent_next_time_damage_sentence(view.tokens)
}

fn parse_scaled_target_power_sentence_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_scaled_target_power_sentence(view.tokens)
}

fn parse_preconditional_sentence_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    run_sentence_primitives_lexed(
        view.tokens,
        PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
        &PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
    )
}

fn parse_spell_this_way_pay_life_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if grammar::words_match_prefix(
        view.tokens,
        &["if", "you", "cast", "a", "spell", "this", "way"],
    )
    .is_some()
        && grammar::contains_word(view.tokens, "rather")
        && grammar::contains_word(view.tokens, "mana")
        && grammar::contains_word(view.tokens, "cost")
    {
        return Ok(Some(vec![
            EffectAst::GrantTaggedSpellAlternativeCostPayLifeByManaValueUntilEndOfTurn {
                tag: TagKey::from(IT_TAG),
                player: PlayerAst::You,
            },
        ]));
    }
    Ok(None)
}

fn parse_postconditional_sentence_primitives_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    run_sentence_primitives_lexed(
        view.tokens,
        POST_CONDITIONAL_SENTENCE_PRIMITIVES,
        &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
    )
}

fn parse_effect_chain_rule_lexed(
    view: &LexClauseView<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_effect_chain_lexed(view.tokens).map(Some)
}

const SENTENCE_PRE_DIAGNOSTIC_PARSE_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 5] = [
    LexRuleDef {
        id: "redirect-next-damage",
        priority: 100,
        heads: &["the"],
        shape_mask: 0,
        run: parse_redirect_next_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "prevent-next-time-damage",
        priority: 110,
        heads: &["the"],
        shape_mask: 0,
        run: parse_prevent_next_time_damage_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "scale-target-power",
        priority: 120,
        heads: &["double", "triple"],
        shape_mask: 0,
        run: parse_scaled_target_power_sentence_rule_lexed,
    },
    LexRuleDef {
        id: "spell-this-way-pay-life",
        priority: 130,
        heads: &["if"],
        shape_mask: RULE_SHAPE_STARTS_IF,
        run: parse_spell_this_way_pay_life_rule_lexed,
    },
    LexRuleDef {
        id: "preconditional-primitives",
        priority: 135,
        heads: SENTENCE_PRIMITIVE_RULE_HEADS,
        shape_mask: 0,
        run: parse_preconditional_sentence_primitives_rule_lexed,
    },
];

const SENTENCE_POST_DIAGNOSTIC_PARSE_RULES_LEXED: [LexRuleDef<Vec<EffectAst>>; 2] = [
    LexRuleDef {
        id: "postconditional-primitives",
        priority: 160,
        heads: SENTENCE_PRIMITIVE_RULE_HEADS,
        shape_mask: 0,
        run: parse_postconditional_sentence_primitives_rule_lexed,
    },
    LexRuleDef {
        id: "effect-chain",
        priority: 170,
        heads: &[],
        shape_mask: 0,
        run: parse_effect_chain_rule_lexed,
    },
];

const SENTENCE_PRE_DIAGNOSTIC_PARSE_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SENTENCE_PRE_DIAGNOSTIC_PARSE_RULES_LEXED);
const SENTENCE_POST_DIAGNOSTIC_PARSE_INDEX_LEXED: LexRuleIndex<Vec<EffectAst>> =
    LexRuleIndex::new(&SENTENCE_POST_DIAGNOSTIC_PARSE_RULES_LEXED);

fn run_sentence_parse_rules_lexed(
    tokens: &[OwnedLexToken],
) -> Result<(&'static str, Vec<EffectAst>), CardTextError> {
    let view = LexClauseView::from_tokens(tokens);
    match SENTENCE_PRE_DIAGNOSTIC_PARSE_INDEX_LEXED.run_first(&view) {
        Ok(Some((rule_id, effects))) => return Ok((rule_id, effects)),
        Ok(None) => {}
        Err(parse_err) => {
            if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
                return Err(diag);
            }
            return Err(parse_err);
        }
    }

    if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }

    if let Some((rule_id, effects)) = SENTENCE_POST_DIAGNOSTIC_PARSE_INDEX_LEXED.run_first(&view)? {
        return Ok((rule_id, effects));
    }

    Err(CardTextError::InvariantViolation(format!(
        "missing sentence parse rule for clause: '{}'",
        view.display_text()
    )))
}

fn sentence_has_each_player_lose_discard_sacrifice_chain(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::words_match_any_prefix(tokens, EACH_PLAYER_PREFIXES).is_some()
        && grammar::contains_word(tokens, "then")
        && (grammar::contains_word(tokens, "lose") || grammar::contains_word(tokens, "loses"))
        && (grammar::contains_word(tokens, "discard") || grammar::contains_word(tokens, "discards"))
        && (grammar::contains_word(tokens, "sacrifice")
            || grammar::contains_word(tokens, "sacrifices"))
}

fn sentence_has_each_player_exile_sacrifice_return_exiled_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::words_match_any_prefix(tokens, EACH_PLAYER_EXILES_ALL_PREFIXES).is_some()
        && grammar::contains_word(tokens, "sacrifices")
        && grammar::contains_word(tokens, "puts")
        && grammar::contains_word(tokens, "exiled")
        && grammar::contains_word(tokens, "this")
        && grammar::contains_word(tokens, "way")
}

fn sentence_has_put_one_of_them_into_hand_rest_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::words_find_phrase(tokens, &["one", "of", "them", "into", "your"]).is_some()
        && grammar::contains_word(tokens, "rest")
        && (grammar::contains_word(tokens, "graveyard")
            || grammar::contains_word(tokens, "graveyards"))
}

fn sentence_has_loses_all_abilities_with_becomes_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    let has_loses_all_abilities = (grammar::contains_word(tokens, "lose")
        || grammar::contains_word(tokens, "loses"))
        && grammar::words_find_phrase(tokens, &["all", "abilities"]).is_some();
    has_loses_all_abilities && grammar::contains_word(tokens, "becomes")
}

fn sentence_has_spent_to_cast_this_spell_without_condition(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    let has_spent_to_cast_this_spell =
        grammar::words_find_phrase(tokens, &["was", "spent", "to", "cast", "this", "spell"])
            .is_some();
    has_spent_to_cast_this_spell
        && !grammar::contains_word(tokens, "if")
        && !grammar::contains_word(tokens, "unless")
}

fn sentence_has_would_enter_instead_replacement_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::contains_word(tokens, "would")
        && (grammar::contains_word(tokens, "enter") || grammar::contains_word(tokens, "enters"))
        && grammar::contains_word(tokens, "instead")
}

fn sentence_has_different_mana_value_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    grammar::words_find_phrase(tokens, &["different", "mana", "value"]).is_some()
}

fn sentence_has_most_common_color_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    grammar::words_find_phrase(tokens, &["most", "common", "color", "among", "all"]).is_some()
        && grammar::contains_word(tokens, "permanents")
}

fn sentence_has_power_vs_count_constraint(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    grammar::contains_word(tokens, "power")
        && grammar::words_find_phrase(
            tokens,
            &["less", "than", "or", "equal", "to", "the", "number", "of"],
        )
        .is_some()
}

fn sentence_has_put_into_graveyards_from_battlefield_this_turn(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::words_find_phrase(
        tokens,
        &[
            "put",
            "into",
            "graveyards",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    )
    .is_some()
}

fn sentence_has_phase_out_until_leaves_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    (grammar::contains_word(tokens, "phase")
        || grammar::contains_word(tokens, "phases")
        || grammar::contains_word(tokens, "phased"))
        && grammar::contains_word(tokens, "until")
        && grammar::words_find_phrase(tokens, &["leaves", "the", "battlefield"]).is_some()
}

fn sentence_has_same_name_as_another_in_hand_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    grammar::words_find_phrase(tokens, &["same", "name", "as", "another", "card", "in"]).is_some()
        && grammar::contains_word(tokens, "hand")
}

fn sentence_has_for_each_mana_from_spent_to_cast_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::words_find_phrase(tokens, &["for", "each", "mana", "from"]).is_some()
        && grammar::contains_word(tokens, "spent")
        && grammar::words_find_phrase(tokens, &["cast", "this", "spell", "create"]).is_some()
}

fn sentence_has_when_you_sacrifice_this_way_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    grammar::words_find_phrase(tokens, &["when", "you", "sacrifice"]).is_some()
        && grammar::words_find_phrase(tokens, &["this", "way"]).is_some()
}

fn sentence_has_sacrifice_any_number_then_draw_that_many_clause(
    words: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    (grammar::contains_word(tokens, "sacrifice") || grammar::contains_word(tokens, "sacrifices"))
        && contains_word_window(words, &["any", "number", "of"])
        && (grammar::contains_word(tokens, "draw") || grammar::contains_word(tokens, "draws"))
        && contains_word_window(words, &["that", "many"])
}

fn sentence_has_greatest_mana_value_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    contains_word_window(words, &["greatest", "mana", "value"])
}

fn sentence_has_least_power_among_creatures_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    contains_word_window(words, &["least", "power", "among", "creatures"])
}

fn sentence_has_villainous_choice_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    grammar::contains_word(tokens, "villainous") && grammar::contains_word(tokens, "choice")
}

fn sentence_has_divided_evenly_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    contains_word_window(words, &["divided", "evenly"])
}

fn sentence_has_different_names_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    contains_word_window(words, &["different", "names"])
}

fn sentence_has_chosen_at_random_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    contains_word_window(words, &["chosen", "at", "random"])
}

fn sentence_has_defending_players_choice_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    grammar::words_find_phrase(tokens, &["defending", "player's", "choice"]).is_some()
        || grammar::words_find_phrase(tokens, &["defending", "player", "choice"]).is_some()
        || grammar::words_find_phrase(tokens, &["player's", "choice", "target"]).is_some()
        || grammar::words_find_phrase(tokens, &["defending", "player", "s", "choice"]).is_some()
}

fn sentence_has_target_creature_token_player_planeswalker_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::contains_word(tokens, "target")
        && grammar::contains_word(tokens, "creature")
        && grammar::contains_word(tokens, "token")
        && grammar::contains_word(tokens, "player")
        && grammar::contains_word(tokens, "planeswalker")
}

fn sentence_has_if_you_sacrifice_an_island_this_way_clause(
    words: &[&str],
    _: &[OwnedLexToken],
) -> bool {
    contains_word_window(words, &["if", "you", "sacrifice", "an", "island"])
        && contains_word_window(words, &["this", "way"])
}

fn sentence_has_spent_to_cast_clause(words: &[&str], _: &[OwnedLexToken]) -> bool {
    contains_word_window(words, &["spent", "to", "cast"])
}

fn sentence_has_face_down_clause(words: &[&str], tokens: &[OwnedLexToken]) -> bool {
    let has_face_down = contains_word_window(words, &["face", "down"])
        || words
            .iter()
            .any(|word| matches!(*word, "face-down" | "facedown"));
    if !has_face_down {
        return false;
    }

    // Simple "exile ... face down" clauses are handled by the generic exile
    // parser; keep rejecting broader manifest/pile patterns here.
    let simple_exile_face_down = grammar::words_match_any_prefix(tokens, EXILE_PREFIXES).is_some()
        && !grammar::contains_word(tokens, "then")
        && !grammar::contains_word(tokens, "manifest")
        && !grammar::contains_word(tokens, "pile");
    !simple_exile_face_down
}

fn sentence_has_copy_spell_legendary_exception_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::contains_word(tokens, "copy")
        && grammar::contains_word(tokens, "spell")
        && grammar::contains_word(tokens, "legendary")
        && (grammar::contains_word(tokens, "except") || grammar::contains_word(tokens, "isnt"))
}

fn sentence_has_return_each_creature_that_isnt_list_clause(
    _: &[&str],
    tokens: &[OwnedLexToken],
) -> bool {
    grammar::words_match_any_prefix(tokens, RETURN_EACH_CREATURE_ISNT_PREFIXES).is_some()
        && grammar::contains_word(tokens, "or")
}

fn sentence_has_unsupported_negated_untap_clause(_: &[&str], tokens: &[OwnedLexToken]) -> bool {
    let has_supported_control_duration =
        grammar::words_find_phrase(tokens, &["for", "as", "long", "as", "you", "control"])
            .is_some();
    let has_supported_source_tapped_duration =
        grammar::words_find_phrase(tokens, &["for", "as", "long", "as"]).is_some()
            && grammar::contains_word(tokens, "remains")
            && grammar::contains_word(tokens, "tapped")
            && (grammar::contains_word(tokens, "this")
                || grammar::contains_word(tokens, "thiss")
                || grammar::contains_word(tokens, "source")
                || grammar::contains_word(tokens, "artifact")
                || grammar::contains_word(tokens, "creature")
                || grammar::contains_word(tokens, "permanent"));
    is_negated_untap_clause_tokens(tokens)
        && !grammar::contains_word(tokens, "and")
        && !grammar::contains_word(tokens, "next")
        && !has_supported_control_duration
        && !has_supported_source_tapped_duration
        && grammar::contains_word(tokens, "during")
        && (grammar::contains_word(tokens, "step") || grammar::contains_word(tokens, "steps"))
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
    if let Some(effect) = parse_if_damage_would_be_dealt_put_counters_sentence(tokens)? {
        return Ok(vec![effect]);
    }
    if let Some(effects) = parse_cant_effect_sentence_with_grammar_entrypoint_lexed(tokens)? {
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
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["exile", "them"]).is_none() {
        return Ok(None);
    }
    let Some(meld_idx) =
        find_dispatch_inner_phrase_start(&clause_words, &["then", "meld", "them", "into"])
    else {
        return Ok(None);
    };
    let result_words = &clause_words[meld_idx + 4..];
    if result_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing meld result name (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    Ok(Some(EffectAst::Meld {
        result_name: result_words.join(" "),
        enters_tapped: false,
        enters_attacking: false,
    }))
}

fn parse_if_damage_would_be_dealt_put_counters_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["if", "damage", "would", "be", "dealt", "to"])
        .is_none()
    {
        return Ok(None);
    }

    let Some(this_turn_rel) =
        find_dispatch_inner_phrase_start(&clause_words[6..], &["this", "turn"])
    else {
        return Ok(None);
    };
    let this_turn_idx = 6 + this_turn_rel;
    let tail = &clause_words[this_turn_idx + 2..];
    let valid_tail = matches!(
        tail,
        [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counters", "on",
            "that", "creature"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "it"
        ] | [
            "prevent", "that", "damage", "and", "put", "that", "many", "+1/+1", "counter", "on",
            "that", "creature"
        ]
    );
    if !valid_tail {
        return Ok(None);
    }

    let target_tokens = &tokens[6..this_turn_idx];
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target = parse_target_phrase(target_tokens)?;

    Ok(Some(EffectAst::PreventDamageToTargetPutCounters {
        amount: None,
        target,
        duration: Until::EndOfTurn,
        counter_type: CounterType::PlusOnePlusOne,
    }))
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

pub(crate) fn parse_effect_sentence_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let word_view = LexClauseView::from_tokens(tokens);
    let sentence_words = word_view.words.to_word_refs();
    if is_activate_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if is_trigger_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if slice_starts_with(sentence_words.as_slice(), &["round", "up", "each", "time"]) {
        return Ok(Vec::new());
    }

    if let Some(stripped) = strip_labeled_effect_prefix_lexed(tokens) {
        return parse_effect_sentence_lexed(stripped);
    }
    if tokens.first().is_some_and(|token| token.is_word("if"))
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
            &PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(effects) =
        parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?
    {
        return Ok(effects);
    }
    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    if tokens.first().is_some_and(|token| token.is_word("exile"))
        && grammar::contains_word(tokens, "then")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if tokens.first().is_some_and(|token| token.is_word("then")) && tokens.len() > 1 {
        return parse_effect_sentence_lexed(&tokens[1..]);
    }
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        return Ok(vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: parse_effect_sentence_lexed(prefix.trailing_tokens)?,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: parse_effect_sentence_lexed(prefix.trailing_tokens)?,
            },
        }]);
    }
    if tokens
        .iter()
        .any(|token| token.is_word("search") || token.is_word("searches"))
        && let Some(mut effects) =
            parse_search_library_sentence_with_grammar_entrypoint_lexed(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_starts_with(
        sentence_words.as_slice(),
        &["exile", "all", "cards", "from"],
    ) && slice_contains_any(sentence_words.as_slice(), &["hand", "hands"])
        && slice_contains_any(sentence_words.as_slice(), &["graveyard", "graveyards"])
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words.first() == Some(&"enchant")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_contains(sentence_words.as_slice(), &"unless")
        && let Some(mut effects) = super::parse_sentence_unless_pays(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_contains(sentence_words.as_slice(), &"unless")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words
        .iter()
        .any(|word| matches!(*word, "gain" | "gains" | "lose" | "loses"))
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words
        .iter()
        .any(|word| *word == "vote" || *word == "votes")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words.first() == Some(&"return")
        && slice_contains(sentence_words.as_slice(), &"rounded")
        && slice_contains(sentence_words.as_slice(), &"up")
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words.first() == Some(&"choose")
        && contains_word_window(sentence_words.as_slice(), &["do", "the", "same", "for"])
        && let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SENTENCE_PRIMITIVES,
            &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if slice_starts_with_any(
        sentence_words.as_slice(),
        &[
            &["each", "player", "choose"],
            &["each", "player", "chooses"],
        ],
    ) {
        if let Some(mut effects) = run_sentence_primitives_lexed(
            tokens,
            PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
            &PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if let Some(diag) = diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }
    if super::parse_leading_player_may_lexed(tokens).is_some() {
        return parse_effect_chain_lexed(tokens);
    }
    if super::looks_like_multi_create_chain_lexed(tokens) {
        if let Some(unless_action) = super::parse_or_action_clause_lexed(tokens)? {
            return Ok(vec![unless_action]);
        }
        return super::parse_effect_chain_inner_lexed(tokens);
    }

    let (_, effects) = run_sentence_parse_rules_lexed(tokens)?;
    Ok(effects)
}

fn strip_labeled_effect_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    let dash_idx = find_index(tokens, |token| {
        matches!(
            token.kind,
            crate::cards::builders::parser::lexer::TokenKind::Dash
                | crate::cards::builders::parser::lexer::TokenKind::EmDash
        )
    })?;
    if !(1..=3).any(|expected| expected == dash_idx) || dash_idx + 1 >= tokens.len() {
        return None;
    }
    if tokens[..=dash_idx].iter().any(|token| {
        !matches!(
            token.kind,
            crate::cards::builders::parser::lexer::TokenKind::Word
                | crate::cards::builders::parser::lexer::TokenKind::Dash
                | crate::cards::builders::parser::lexer::TokenKind::EmDash
        )
    }) {
        return None;
    }

    let prefix_words = crate::cards::builders::parser::token_word_refs(&tokens[..dash_idx]);
    if prefix_words.is_empty() {
        return None;
    }
    let is_known_label = matches!(
        prefix_words[0],
        "adamant"
            | "addendum"
            | "ascend"
            | "battalion"
            | "delirium"
            | "domain"
            | "ferocious"
            | "formidable"
            | "hellbent"
            | "metalcraft"
            | "morbid"
            | "radiance"
            | "raid"
            | "revolt"
            | "spectacle"
            | "spell"
            | "surge"
            | "threshold"
            | "undergrowth"
    );
    if !is_known_label {
        return None;
    }

    Some(&tokens[dash_idx + 1..])
}

fn is_enters_as_copy_clause_tokens(tokens: &[OwnedLexToken]) -> bool {
    // Find "as a copy", "as an copy", or "as copy" and verify enter/enters appears before it
    let as_copy_idx = grammar::words_find_phrase(tokens, &["as", "a", "copy"])
        .or_else(|| grammar::words_find_phrase(tokens, &["as", "an", "copy"]))
        .or_else(|| grammar::words_find_phrase(tokens, &["as", "copy"]));
    match as_copy_idx {
        Some(idx) => {
            // Check that "enter" or "enters" appears in the tokens before the "as" position
            tokens[..idx]
                .iter()
                .any(|t| t.is_word("enter") || t.is_word("enters"))
        }
        None => false,
    }
}

pub(crate) fn is_negated_untap_clause(words: &[&str]) -> bool {
    if words.len() < 3 {
        return false;
    }
    let has_untap = slice_contains_any(words, &["untap", "untaps"]);
    let has_negation = slice_contains_any(words, &["doesnt", "dont", "cant"])
        || contains_any_word_window(words, &[&["does", "not"], &["do", "not"], &["can", "not"]]);
    has_untap && has_negation
}

fn is_negated_untap_clause_tokens(tokens: &[OwnedLexToken]) -> bool {
    let has_untap =
        grammar::contains_word(tokens, "untap") || grammar::contains_word(tokens, "untaps");
    let has_negation = grammar::contains_word(tokens, "doesnt")
        || grammar::contains_word(tokens, "dont")
        || grammar::contains_word(tokens, "cant")
        || grammar::words_find_phrase(tokens, &["does", "not"]).is_some()
        || grammar::words_find_phrase(tokens, &["do", "not"]).is_some()
        || grammar::words_find_phrase(tokens, &["can", "not"]).is_some();
    has_untap && has_negation
}

pub(crate) fn parse_token_copy_modifier_sentence(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let is_gain_haste_until_eot = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered.as_slice(),
        ["it", "has", "haste"]
            | ["they", "have", "haste"]
            | ["token", "created", "this", "way", "has", "haste"]
            | ["tokens", "created", "this", "way", "have", "haste"]
            | ["token", "created", "this", "way", "gains", "haste"]
            | ["tokens", "created", "this", "way", "gain", "haste"]
    );
    if is_has_haste {
        return Some(TokenCopyFollowup::HasHaste);
    }

    let enters_tapped_and_attacking = matches!(
        filtered.as_slice(),
        ["it", "enters", "tapped", "and", "attacking"]
            | ["they", "enter", "tapped", "and", "attacking"]
            | ["token", "enters", "tapped", "and", "attacking"]
            | ["tokens", "enter", "tapped", "and", "attacking"]
            | [
                "token",
                "created",
                "this",
                "way",
                "enters",
                "tapped",
                "and",
                "attacking"
            ]
            | [
                "tokens",
                "created",
                "this",
                "way",
                "enter",
                "tapped",
                "and",
                "attacking"
            ]
    );
    if enters_tapped_and_attacking {
        return Some(TokenCopyFollowup::EnterTappedAndAttacking);
    }

    if slice_starts_with_any(
        filtered.as_slice(),
        &[
            &["sacrifice", "it"],
            &["sacrifice", "them"],
            &["sacrifice", "that", "token"],
            &["sacrifice", "those", "tokens"],
        ],
    ) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
        }
    }
    if slice_starts_with_any(filtered.as_slice(), &[&["exile", "it"], &["exile", "them"]]) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::ExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
        ],
    );
    if starts_delayed_end_step_sacrifice {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "exile",
            ],
        ],
    );
    if starts_delayed_end_step_exile {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}

pub(crate) fn parse_token_copy_modifier_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let is_gain_haste_until_eot = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered.as_slice(),
        ["it", "has", "haste"]
            | ["they", "have", "haste"]
            | ["token", "created", "this", "way", "has", "haste"]
            | ["tokens", "created", "this", "way", "have", "haste"]
            | ["token", "created", "this", "way", "gains", "haste"]
            | ["tokens", "created", "this", "way", "gain", "haste"]
    );
    if is_has_haste {
        return Some(TokenCopyFollowup::HasHaste);
    }

    let enters_tapped_and_attacking = matches!(
        filtered.as_slice(),
        ["token", "enters", "tapped", "and", "attacking"]
            | ["tokens", "enter", "tapped", "and", "attacking"]
            | [
                "token",
                "created",
                "this",
                "way",
                "enters",
                "tapped",
                "and",
                "attacking"
            ]
            | [
                "tokens",
                "created",
                "this",
                "way",
                "enter",
                "tapped",
                "and",
                "attacking"
            ]
    );
    if enters_tapped_and_attacking {
        return Some(TokenCopyFollowup::EnterTappedAndAttacking);
    }

    if slice_starts_with_any(
        filtered.as_slice(),
        &[
            &["sacrifice", "it"],
            &["sacrifice", "them"],
            &["sacrifice", "that", "token"],
            &["sacrifice", "those", "tokens"],
        ],
    ) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
        }
    }
    if slice_starts_with_any(filtered.as_slice(), &[&["exile", "it"], &["exile", "them"]]) {
        let has_next_end_step = contains_word_window(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::ExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
        ],
    );
    if starts_delayed_end_step_sacrifice {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile = slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "exile",
            ],
        ],
    );
    if starts_delayed_end_step_exile {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}

pub(crate) fn parse_delayed_until_next_end_step_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut idx = 0usize;
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return Ok(None);
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("beginning"))
    {
        return Ok(None);
    }
    idx += 1;
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return Ok(None);
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }

    let mut player = if tokens.get(idx).is_some_and(|token| token.is_word("your")) {
        idx += 1;
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };
    let mut start_next_turn = false;

    if tokens.get(idx).is_some_and(|token| token.is_word("next")) {
        if !tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("end"))
            || !tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("step"))
        {
            return Ok(None);
        }
        idx += 3;
    } else {
        if !tokens.get(idx).is_some_and(|token| token.is_word("end"))
            || !tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("step"))
        {
            return Ok(None);
        }
        idx += 2;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
        if tokens.get(idx).is_some_and(|token| token.is_word("that"))
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("player") || token.is_word("players"))
        {
            player = PlayerFilter::IteratedPlayer;
            idx += 2;
        } else if tokens.get(idx).is_some_and(|token| token.is_word("your")) {
            player = PlayerFilter::You;
            idx += 1;
        } else if tokens.get(idx).is_some_and(|token| token.is_word("target"))
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("player"))
        {
            player = PlayerFilter::Target(Box::new(PlayerFilter::Any));
            idx += 2;
        } else {
            return Ok(None);
        }

        if !tokens.get(idx).is_some_and(|token| token.is_word("next"))
            || !tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("turn"))
        {
            return Ok(None);
        }
        idx += 2;
        start_next_turn = true;
    }

    if tokens.get(idx).is_some_and(OwnedLexToken::is_comma) {
        idx += 1;
    }
    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(
            "missing delayed end-step effect clause".to_string(),
        ));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed end-step effect clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    if start_next_turn {
        let player_ast = match player {
            PlayerFilter::You => PlayerAst::You,
            PlayerFilter::IteratedPlayer => PlayerAst::That,
            PlayerFilter::Target(_) => PlayerAst::Target,
            PlayerFilter::Opponent => PlayerAst::Opponent,
            _ => PlayerAst::Any,
        };
        Ok(Some(vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
            player: player_ast,
            effects: delayed_effects,
        }]))
    } else {
        Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
            player,
            effects: delayed_effects,
        }]))
    }
}

pub(crate) fn parse_sentence_delayed_trigger_this_turn(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        return Ok(None);
    }

    let Some((before_comma, after_comma)) =
        super::super::grammar::primitives::split_lexed_once_on_delimiter(
            tokens,
            super::super::lexer::TokenKind::Comma,
        )
    else {
        return Ok(None);
    };

    let mut trigger_tokens = trim_commas(before_comma);
    if trigger_tokens
        .first()
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
    {
        trigger_tokens = trigger_tokens[1..].to_vec();
    }
    if trigger_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before comma (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let trigger_word_storage = DispatchInnerNormalizedWords::new(&trigger_tokens);
    let trigger_words = trigger_word_storage.to_word_refs();
    if trigger_words.len() < 3 || !slice_ends_with(trigger_words.as_slice(), &["this", "turn"]) {
        return Ok(None);
    }

    let trim_start = trigger_word_storage
        .token_index_for_word_index(trigger_words.len() - 2)
        .unwrap_or(trigger_tokens.len());
    let trigger_core_tokens = trim_commas(&trigger_tokens[..trim_start]);
    if trigger_core_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger clause before 'this turn' (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }
    let trigger_core_words = crate::cards::builders::parser::token_word_refs(&trigger_core_tokens);
    let trigger = if matches!(
        trigger_core_words.as_slice(),
        ["that", "creature", "is", "dealt", "damage"]
            | ["that", "permanent", "is", "dealt", "damage"]
    ) {
        let mut filter = if trigger_core_words[1] == "creature" {
            ObjectFilter::creature()
        } else {
            ObjectFilter::permanent()
        };
        filter = filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
        TriggerSpec::IsDealtDamage(filter)
    } else {
        super::super::parse_trigger_clause_lexed(&trigger_core_tokens)?
    };
    let remainder = trim_commas(after_comma);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed trigger effect clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedTriggerThisTurn {
        trigger,
        effects: delayed_effects,
    }]))
}

pub(crate) fn parse_delayed_when_that_dies_this_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if !matches!(
        clause_words.first().copied(),
        Some("when" | "whenever" | "if")
    ) {
        return Ok(None);
    }
    let mut delayed_filter: Option<ObjectFilter> = None;
    let split_after_word_idx = if clause_words.get(1) == Some(&"that") {
        let Some(dies_idx) = find_index(clause_words.as_slice(), |word| *word == "dies") else {
            return Ok(None);
        };
        if clause_words.get(dies_idx + 1) != Some(&"this")
            || clause_words.get(dies_idx + 2) != Some(&"turn")
        {
            return Ok(None);
        }
        dies_idx + 2
    } else if let Some(dealt_idx) = find_dispatch_inner_phrase_start(
        &clause_words,
        &["dealt", "damage", "this", "way", "dies", "this", "turn"],
    ) {
        if dealt_idx <= 1 {
            return Ok(None);
        }
        let subject_start = token_index_for_word_index(tokens, 1).unwrap_or(tokens.len());
        let subject_end = token_index_for_word_index(tokens, dealt_idx).unwrap_or(tokens.len());
        if subject_start >= subject_end {
            return Ok(None);
        }
        let mut subject_tokens = trim_edge_punctuation(&tokens[subject_start..subject_end]);
        if subject_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let stripped_subject = strip_leading_articles(&subject_tokens);
        if !stripped_subject.is_empty() {
            subject_tokens = stripped_subject;
        }
        delayed_filter = Some(parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?);
        dealt_idx + 6
    } else if let Some(dealt_idx) = find_dispatch_inner_phrase_start(
        &clause_words,
        &[
            "dealt", "damage", "this", "way", "would", "die", "this", "turn",
        ],
    ) {
        if dealt_idx <= 1 {
            return Ok(None);
        }
        let subject_start = token_index_for_word_index(tokens, 1).unwrap_or(tokens.len());
        let subject_end = token_index_for_word_index(tokens, dealt_idx).unwrap_or(tokens.len());
        if subject_start >= subject_end {
            return Ok(None);
        }
        let mut subject_tokens = trim_edge_punctuation(&tokens[subject_start..subject_end]);
        if subject_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let stripped_subject = strip_leading_articles(&subject_tokens);
        if !stripped_subject.is_empty() {
            subject_tokens = stripped_subject;
        }
        delayed_filter = Some(parse_object_filter(&subject_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported object filter in delayed dies-this-way clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?);
        dealt_idx + 7
    } else {
        return Ok(None);
    };
    let split_idx =
        token_index_for_word_index(tokens, split_after_word_idx + 1).unwrap_or(tokens.len());
    let mut remainder = &tokens[split_idx..];
    if remainder.first().is_some_and(OwnedLexToken::is_comma) {
        remainder = &remainder[1..];
    }
    let remainder = trim_commas(remainder);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        filter: delayed_filter,
        effects: delayed_effects,
    }]))
}

pub(crate) fn parse_each_player_choose_and_sacrifice_rest(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let all_words = crate::cards::builders::parser::token_word_refs(tokens);
    if all_words.len() < 6 {
        return Ok(None);
    }

    if !slice_starts_with_any(
        &all_words,
        &[
            &["each", "player", "chooses"],
            &["each", "player", "choose"],
        ],
    ) {
        return Ok(None);
    }

    let Some((before_then, after_then)) =
        super::super::grammar::primitives::split_lexed_once_on_separator(tokens, || {
            use winnow::Parser as _;
            super::super::grammar::primitives::kw("then").void()
        })
    else {
        return Ok(None);
    };
    let then_idx = before_then.len();
    let after_words = crate::cards::builders::parser::token_word_refs(after_then);
    if !slice_starts_with_any(
        &after_words,
        &[
            &["sacrifice", "the", "rest"],
            &["sacrifices", "the", "rest"],
        ],
    ) {
        return Ok(None);
    }

    let choose_tokens = &tokens[3..then_idx];
    if choose_tokens.is_empty() {
        return Ok(None);
    }

    let from_idx = find_from_among(choose_tokens);
    let Some(from_idx) = from_idx else {
        return Ok(None);
    };

    let (list_tokens, base_tokens) = if from_idx == 0 {
        let list_start = find_list_start(&choose_tokens[2..])
            .map(|idx| idx + 2)
            .ok_or_else(|| {
                CardTextError::ParseError("missing choice list after 'from among'".to_string())
            })?;
        (
            choose_tokens.get(list_start..).unwrap_or_default(),
            choose_tokens.get(2..list_start).unwrap_or_default(),
        )
    } else {
        (
            choose_tokens.get(..from_idx).unwrap_or_default(),
            choose_tokens.get(from_idx + 2..).unwrap_or_default(),
        )
    };

    let list_tokens = trim_commas(list_tokens);
    let base_tokens = trim_commas(base_tokens);
    if list_tokens.is_empty() || base_tokens.is_empty() {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(&base_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported base filter in choose-and-sacrifice clause (clause: '{}')",
            all_words.join(" ")
        ))
    })?;
    if base_filter.controller.is_none() {
        base_filter.controller = Some(PlayerFilter::IteratedPlayer);
    }

    let mut effects = Vec::new();
    let keep_tag: TagKey = "keep".into();

    for segment in split_choose_list(&list_tokens) {
        let segment = strip_leading_articles(&segment);
        if segment.is_empty() {
            continue;
        }
        let segment_filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported choice filter in choose-and-sacrifice clause (clause: '{}')",
                all_words.join(" ")
            ))
        })?;
        let mut combined = merge_filters(&base_filter, &segment_filter);
        combined = combined.not_tagged(keep_tag.clone());
        effects.push(EffectAst::ChooseObjects {
            filter: combined,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::Implicit,
            tag: keep_tag.clone(),
        });
    }

    if effects.is_empty() {
        return Ok(None);
    }

    let sacrifice_filter = base_filter.clone().not_tagged(keep_tag.clone());
    effects.push(EffectAst::SacrificeAll {
        filter: sacrifice_filter,
        player: PlayerAst::Implicit,
    });

    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

pub(crate) fn find_from_among(tokens: &[OwnedLexToken]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if token.is_word("from") && tokens.get(idx + 1).is_some_and(|t| t.is_word("among")) {
            Some(idx)
        } else {
            None
        }
    })
}

pub(crate) fn find_list_start(tokens: &[OwnedLexToken]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) {
            if tokens
                .get(idx + 1)
                .and_then(OwnedLexToken::as_word)
                .and_then(parse_card_type)
                .is_some()
            {
                return Some(idx);
            }
        } else if parse_card_type(word).is_some() {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn trim_commas(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && tokens[start].is_comma() {
        start += 1;
    }
    while end > start && tokens[end - 1].is_comma() {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

pub(crate) fn trim_edge_punctuation(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && (tokens[start].is_comma()
            || tokens[start].is_period()
            || tokens[start].is_semicolon()
            || tokens[start].is_quote())
    {
        start += 1;
    }
    while end > start
        && (tokens[end - 1].is_comma()
            || tokens[end - 1].is_period()
            || tokens[end - 1].is_semicolon()
            || tokens[end - 1].is_quote())
    {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

pub(crate) fn strip_leading_articles(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut start = 0usize;
    while start < tokens.len() {
        if let Some(word) = tokens[start].as_word()
            && is_article(word)
        {
            start += 1;
            continue;
        }
        break;
    }
    tokens[start..].to_vec()
}

pub(crate) fn split_choose_list(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    for segment in split_lexed_slices_on_and(tokens) {
        for sub in split_lexed_slices_on_comma(segment) {
            let trimmed = trim_commas(sub);
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
        }
    }
    segments
}

pub(crate) fn merge_filters(base: &ObjectFilter, specific: &ObjectFilter) -> ObjectFilter {
    let mut merged = base.clone();

    if !specific.card_types.is_empty() {
        merged.card_types = specific.card_types.clone();
    }
    if !specific.all_card_types.is_empty() {
        merged.all_card_types = specific.all_card_types.clone();
    }
    if !specific.subtypes.is_empty() {
        merged.subtypes.extend(specific.subtypes.clone());
    }
    if !specific.excluded_card_types.is_empty() {
        merged
            .excluded_card_types
            .extend(specific.excluded_card_types.clone());
    }
    if !specific.excluded_colors.is_empty() {
        merged.excluded_colors = merged.excluded_colors.union(specific.excluded_colors);
    }
    if let Some(colors) = specific.colors {
        merged.colors = Some(
            merged
                .colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
    merged.chosen_color |= specific.chosen_color;
    if merged.zone.is_none() {
        merged.zone = specific.zone;
    }
    if merged.controller.is_none() {
        merged.controller = specific.controller.clone();
    }
    if merged
        .attacking_player_or_planeswalker_controlled_by
        .is_none()
    {
        merged.attacking_player_or_planeswalker_controlled_by = specific
            .attacking_player_or_planeswalker_controlled_by
            .clone();
    }
    if merged.owner.is_none() {
        merged.owner = specific.owner.clone();
    }
    merged.other |= specific.other;
    merged.token |= specific.token;
    merged.nontoken |= specific.nontoken;
    merged.tapped |= specific.tapped;
    merged.untapped |= specific.untapped;
    merged.attacking |= specific.attacking;
    merged.nonattacking |= specific.nonattacking;
    merged.blocking |= specific.blocking;
    merged.nonblocking |= specific.nonblocking;
    merged.blocked |= specific.blocked;
    merged.unblocked |= specific.unblocked;
    merged.is_commander |= specific.is_commander;
    merged.noncommander |= specific.noncommander;
    merged.colorless |= specific.colorless;
    merged.multicolored |= specific.multicolored;
    merged.monocolored |= specific.monocolored;

    if let Some(mv) = &specific.mana_value {
        merged.mana_value = Some(mv.clone());
    }
    if let Some(power) = &specific.power {
        merged.power = Some(power.clone());
        merged.power_reference = specific.power_reference;
    }
    if let Some(toughness) = &specific.toughness {
        merged.toughness = Some(toughness.clone());
        merged.toughness_reference = specific.toughness_reference;
    }
    if specific.has_mana_cost {
        merged.has_mana_cost = true;
    }
    if specific.no_x_in_cost {
        merged.no_x_in_cost = true;
    }
    if merged.with_counter.is_none() {
        merged.with_counter = specific.with_counter;
    }
    if merged.without_counter.is_none() {
        merged.without_counter = specific.without_counter;
    }
    if merged.alternative_cast.is_none() {
        merged.alternative_cast = specific.alternative_cast;
    }
    for ability_id in &specific.static_abilities {
        if !iter_contains(merged.static_abilities.iter(), ability_id) {
            merged.static_abilities.push(*ability_id);
        }
    }
    for ability_id in &specific.excluded_static_abilities {
        if !iter_contains(merged.excluded_static_abilities.iter(), ability_id) {
            merged.excluded_static_abilities.push(*ability_id);
        }
    }
    for marker in &specific.ability_markers {
        if !merged
            .ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.ability_markers.push(marker.clone());
        }
    }
    for marker in &specific.excluded_ability_markers {
        if !merged
            .excluded_ability_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.excluded_ability_markers.push(marker.clone());
        }
    }

    merged
}

pub(crate) fn parse_monstrosity_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.first().copied() != Some("monstrosity") {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let (amount, _) = parse_value(amount_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing monstrosity amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Monstrosity { amount }))
}

pub(crate) fn parse_for_each_counter_removed_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if grammar::words_match_prefix(
        tokens,
        &["for", "each", "counter", "removed", "this", "way"],
    )
    .is_none()
    {
        return Ok(None);
    }

    let remainder = if let Some((_before, after)) =
        super::super::grammar::primitives::split_lexed_once_on_delimiter(
            tokens,
            super::super::lexer::TokenKind::Comma,
        ) {
        after
    } else {
        &tokens[6..]
    };

    let remainder_words = crate::cards::builders::parser::token_word_refs(remainder);
    if remainder_words.is_empty() {
        return Ok(None);
    }

    let gets_idx = find_index(remainder_words.as_slice(), |word| {
        matches!(*word, "gets" | "get")
    });
    let Some(gets_idx) = gets_idx else {
        return Ok(None);
    };

    let subject_tokens = &remainder[..gets_idx];
    let subject = parse_subject(subject_tokens);
    let target = match subject {
        SubjectAst::This => TargetAst::Source(None),
        _ => return Ok(None),
    };

    let after_gets = &remainder[gets_idx + 1..];
    let modifier_token = after_gets
        .first()
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier (clause: '{}')",
                remainder_words.join(" ")
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_token)?;

    let duration = if grammar::contains_word(remainder, "until")
        && grammar::contains_word(remainder, "end")
        && grammar::contains_word(remainder, "turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    Ok(Some(EffectAst::PumpByLastEffect {
        power,
        toughness,
        target,
        duration,
    }))
}

pub(crate) fn is_exile_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.first().copied() != Some("exile") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

pub(crate) fn is_exile_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.first().copied() != Some("exile") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.first().copied() != Some("sacrifice") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

pub(crate) fn is_sacrifice_that_token_at_end_of_combat_lexed(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.first().copied() != Some("sacrifice") {
        return false;
    }

    let at_idx = if matches!(words.get(1).copied(), Some("that" | "the" | "those"))
        && matches!(words.get(2).copied(), Some("token" | "tokens"))
    {
        3
    } else if words.get(1).copied() == Some("it") {
        2
    } else {
        return false;
    };
    if words.get(at_idx).copied() != Some("at") {
        return false;
    }
    has_end_of_combat_tail(&words, at_idx)
}

fn has_end_of_combat_tail(words: &[&str], at_idx: usize) -> bool {
    matches!(
        words.get(at_idx + 1..),
        Some(["end", "of", "combat"] | ["the", "end", "of", "combat"])
    )
}

pub(crate) fn parse_take_extra_turn_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if matches!(
        words.as_slice(),
        ["take", "an", "extra", "turn", "after", "this", "one"]
    ) {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player: PlayerAst::You,
            anchor: ExtraTurnAnchorAst::CurrentTurn,
        }));
    }
    if matches!(
        words.as_slice(),
        [
            "the", "chosen", "player", "takes", "an", "extra", "turn", "after", "this", "one"
        ]
    ) {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player: PlayerAst::Chosen,
            anchor: ExtraTurnAnchorAst::CurrentTurn,
        }));
    }
    Ok(None)
}

pub(crate) fn find_same_name_reference_span(
    tokens: &[OwnedLexToken],
) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("with") {
            continue;
        }
        if idx + 6 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
            && tokens[idx + 5].is_word("that")
        {
            return Ok(Some((idx, idx + 7)));
        }
        if idx + 5 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
            && tokens[idx + 4].is_word("that")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            )));
        }
        if idx + 3 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

pub(crate) fn strip_same_controller_reference(
    tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, bool) {
    let mut cleaned = Vec::with_capacity(tokens.len());
    let mut idx = 0usize;
    let mut same_controller = false;
    while idx < tokens.len() {
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("that")
            && tokens[idx + 1].is_word("player")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("its")
            && tokens[idx + 1].is_word("controller")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 3 < tokens.len()
            && tokens[idx].is_word("that")
            && (tokens[idx + 1].is_word("creature")
                || tokens[idx + 1].is_word("permanent")
                || tokens[idx + 1].is_word("card"))
            && tokens[idx + 2].is_word("controller")
            && (tokens[idx + 3].is_word("control") || tokens[idx + 3].is_word("controls"))
        {
            same_controller = true;
            idx += 4;
            continue;
        }

        cleaned.push(tokens[idx].clone());
        idx += 1;
    }

    (cleaned, same_controller)
}

pub(crate) fn parse_same_name_fanout_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((same_start, same_end)) = find_same_name_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..same_start]);
    filter_tokens.extend_from_slice(&tokens[same_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in same-name fanout clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let (cleaned_tokens, same_controller) = strip_same_controller_reference(&filter_tokens);
    let cleaned_tokens = trim_commas(&cleaned_tokens);
    if cleaned_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing base object filter in same-name fanout clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&cleaned_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported same-name fanout filter (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    if same_controller {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameControllerAsTagged,
        });
    }
    Ok(Some(filter))
}

pub(crate) fn parse_same_name_target_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let words_all = crate::cards::builders::parser::token_word_refs(tokens);
    let Some(first_word) = words_all.first().copied() else {
        return Ok(None);
    };

    let deal_tokens: Option<&[OwnedLexToken]> = if first_word == "deal" {
        Some(tokens)
    } else if let Some((Verb::Deal, verb_idx)) = find_verb(tokens) {
        let subject_words: Vec<&str> =
            crate::cards::builders::parser::token_word_refs(&tokens[..verb_idx])
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
        if is_source_reference_words(&subject_words) {
            Some(&tokens[verb_idx..])
        } else {
            None
        }
    } else {
        None
    };

    if let Some(deal_tokens) = deal_tokens {
        let deal_words = crate::cards::builders::parser::token_word_refs(deal_tokens);
        let (amount, used) =
            if deal_words.get(1) == Some(&"that") && deal_words.get(2) == Some(&"much") {
                (Value::EventValue(EventValueSpec::Amount), 2usize)
            } else if let Some((value, used)) = parse_value(&deal_tokens[1..]) {
                (value, used)
            } else {
                return Ok(None);
            };

        let after_amount = &deal_tokens[1 + used..];
        if !after_amount
            .first()
            .is_some_and(|token| token.is_word("damage"))
        {
            return Ok(None);
        }

        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }

        let split_idx = find_token_word_window(target_tokens, &["and", "each", "other"]);
        let Some(split_idx) = split_idx else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }

        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: first_target,
            },
            EffectAst::DealDamageEach { amount, filter },
        ]));
    }

    let verb = first_word;
    if verb != "destroy" && verb != "exile" && verb != "return" {
        return Ok(None);
    }

    let and_idx = find_token_word_window(tokens, &["and", "all", "other"]);
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };
    if and_idx <= 1 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&tokens[1..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }

    let second_clause_tokens = if verb == "return" {
        let to_idx = rfind_index(tokens, |token| token.is_word("to")).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing return destination in same-name fanout clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
        if to_idx <= and_idx + 3 {
            return Err(CardTextError::ParseError(format!(
                "missing same-name filter before return destination (clause: '{}')",
                words_all.join(" ")
            )));
        }
        if !grammar::contains_word(&tokens[to_idx + 1..], "hand")
            && !grammar::contains_word(&tokens[to_idx + 1..], "hands")
        {
            return Ok(None);
        }
        tokens[and_idx + 3..to_idx].to_vec()
    } else {
        tokens[and_idx + 3..].to_vec()
    };

    if second_clause_tokens.is_empty() {
        return Ok(None);
    }

    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let mut first_target = parse_target_phrase(&first_target_tokens)?;
    if verb == "return"
        && let Some(first_filter) = target_object_filter_mut(&mut first_target)
    {
        if first_filter.zone.is_none() {
            first_filter.zone = filter.zone;
            if first_filter.zone.is_none() && grammar::contains_word(tokens, "graveyard") {
                first_filter.zone = Some(Zone::Graveyard);
            }
        }
        if first_filter.owner.is_none() {
            first_filter.owner = filter.owner.clone();
            if first_filter.owner.is_none()
                && contains_word_window(&words_all, &["your", "graveyard"])
            {
                first_filter.owner = Some(PlayerFilter::You);
            }
        }
    }
    let first_effect = match verb {
        "destroy" => EffectAst::Destroy {
            target: first_target,
        },
        "exile" => {
            if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves {
                    target: first_target,
                    face_down: false,
                }
            } else {
                EffectAst::Exile {
                    target: first_target,
                    face_down: false,
                }
            }
        }
        "return" => EffectAst::ReturnToHand {
            target: first_target,
            random: false,
        },
        _ => unreachable!("verb already filtered"),
    };
    let second_effect = match verb {
        "destroy" => EffectAst::DestroyAll { filter },
        "exile" => {
            if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves {
                    target: TargetAst::Object(filter, None, None),
                    face_down: false,
                }
            } else {
                EffectAst::ExileAll {
                    filter,
                    face_down: false,
                }
            }
        }
        "return" => EffectAst::ReturnAllToHand { filter },
        _ => unreachable!("verb already filtered"),
    };

    Ok(Some(vec![first_effect, second_effect]))
}

pub(crate) fn find_shares_color_reference_span(
    tokens: &[OwnedLexToken],
) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("that") {
            continue;
        }
        if idx + 5 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
            && tokens[idx + 5].is_word("it")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'it' in shares-color clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

pub(crate) fn parse_shared_color_fanout_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((share_start, share_end)) = find_shares_color_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..share_start]);
    filter_tokens.extend_from_slice(&tokens[share_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in shared-color fanout clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported shared-color fanout filter (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SharesColorWithTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    Ok(Some(filter))
}

fn split_full_shared_color_target(target: &TargetAst) -> Option<(TargetAst, ObjectFilter)> {
    let TargetAst::Object(filter, explicit_span, extra_span) = target else {
        return None;
    };
    let has_shared_color = filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.relation == TaggedOpbjectRelation::SharesColorWithTagged);
    if !filter.other || !has_shared_color {
        return None;
    }

    let mut first_filter = filter.clone();
    first_filter.other = false;
    first_filter.tagged_constraints.retain(|constraint| {
        !matches!(
            constraint.relation,
            TaggedOpbjectRelation::SharesColorWithTagged | TaggedOpbjectRelation::IsNotTaggedObject
        )
    });

    Some((
        TargetAst::Object(first_filter, *explicit_span, *extra_span),
        filter.clone(),
    ))
}

fn parse_explicit_shared_color_gets_or_gains(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = crate::cards::builders::parser::token_word_refs(tokens);
    let find_and_each_other =
        |scope: &[OwnedLexToken]| find_token_word_window(scope, &["and", "each", "other"]);

    let Some(and_idx) = find_and_each_other(tokens) else {
        return Ok(None);
    };
    if and_idx == 0 {
        return Ok(None);
    }

    let verb_idx = find_index(&tokens[and_idx + 3..], |token| {
        token.is_word("get")
            || token.is_word("gets")
            || token.is_word("gain")
            || token.is_word("gains")
    })
    .map(|idx| and_idx + 3 + idx);
    let Some(verb_token_idx) = verb_idx else {
        return Ok(None);
    };

    let first_target_tokens = trim_commas(&tokens[..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }

    let second_clause_tokens = trim_commas(&tokens[and_idx + 3..verb_token_idx]);
    if second_clause_tokens.is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };
    let first_target = parse_target_phrase(&first_target_tokens)?;

    if tokens[verb_token_idx].is_word("get") || tokens[verb_token_idx].is_word("gets") {
        let modifier_tokens = &tokens[verb_token_idx + 1..];
        let modifier_word = modifier_tokens
            .first()
            .and_then(OwnedLexToken::as_word)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing modifier in shared-color gets clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
        let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
            CardTextError::ParseError(format!(
                "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;

        return Ok(Some(vec![
            EffectAst::Pump {
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                target: first_target,
                duration: Until::EndOfTurn,
                condition: None,
            },
            EffectAst::PumpAll {
                filter,
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                duration: Until::EndOfTurn,
            },
        ]));
    }

    let mut first_clause = first_target_tokens.clone();
    first_clause.extend_from_slice(&tokens[verb_token_idx..]);
    let Some(first_effect) = parse_simple_gain_ability_clause(&first_clause)? else {
        return Ok(None);
    };
    let (abilities, duration) = match first_effect {
        EffectAst::GrantAbilitiesToTarget {
            abilities,
            duration,
            ..
        }
        | EffectAst::GrantAbilitiesAll {
            abilities,
            duration,
            ..
        } => (abilities, duration),
        _ => return Ok(None),
    };

    Ok(Some(vec![
        EffectAst::GrantAbilitiesToTarget {
            target: first_target,
            abilities: abilities.clone(),
            duration: duration.clone(),
        },
        EffectAst::GrantAbilitiesAll {
            filter,
            abilities,
            duration,
        },
    ]))
}

pub(crate) fn parse_shared_color_target_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(effects) = parse_explicit_shared_color_gets_or_gains(tokens)? {
        return Ok(Some(effects));
    }

    let words_all = crate::cards::builders::parser::token_word_refs(tokens);
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    let Some(verb_token_idx) = token_index_for_word_index(tokens, verb_idx) else {
        return Ok(None);
    };

    let find_and_each_other =
        |scope: &[OwnedLexToken]| find_token_word_window(scope, &["and", "each", "other"]);

    if matches!(verb, Verb::Destroy | Verb::Exile | Verb::Untap) {
        let after_verb = &tokens[verb_token_idx + 1..];
        let Some(split_idx) = find_and_each_other(after_verb) else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&after_verb[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = after_verb[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        let mut effects = Vec::with_capacity(2);
        match verb {
            Verb::Destroy => {
                effects.push(EffectAst::Destroy {
                    target: first_target,
                });
                effects.push(EffectAst::DestroyAll { filter });
            }
            Verb::Exile => {
                effects.push(EffectAst::Exile {
                    target: first_target,
                    face_down: false,
                });
                effects.push(EffectAst::ExileAll {
                    filter,
                    face_down: false,
                });
            }
            Verb::Untap => {
                effects.push(EffectAst::Untap {
                    target: first_target,
                });
                effects.push(EffectAst::UntapAll { filter });
            }
            _ => return Ok(None),
        }
        return Ok(Some(effects));
    }

    if verb == Verb::Deal {
        let after_verb = &tokens[verb_token_idx + 1..];
        let (amount, used) = if let Some((prefix, _)) =
            grammar::words_match_any_prefix(after_verb, THAT_MUCH_PREFIXES)
        {
            (Value::EventValue(EventValueSpec::Amount), prefix.len())
        } else if let Some((value, used)) = parse_value(after_verb) {
            (value, used)
        } else {
            return Ok(None);
        };

        let after_amount = &after_verb[used..];
        if !after_amount
            .first()
            .is_some_and(|token| token.is_word("damage"))
        {
            return Ok(None);
        }
        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }
        let Some(split_idx) = find_and_each_other(target_tokens) else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: first_target,
            },
            EffectAst::DealDamageEach { amount, filter },
        ]));
    }

    if words_all.first().copied() == Some("prevent") {
        let mut idx = verb_token_idx + 1;
        if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
            idx += 1;
        }
        if !tokens.get(idx).is_some_and(|token| token.is_word("next")) {
            return Ok(None);
        }
        idx += 1;
        let amount_token = tokens.get(idx).cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing prevent damage amount (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
        let Some((amount, _)) = parse_value(&[amount_token]) else {
            return Ok(None);
        };
        idx += 1;
        if !tokens.get(idx).is_some_and(|token| token.is_word("damage")) {
            return Ok(None);
        }
        idx += 1;
        if tokens.get(idx..idx + 4).is_none_or(|window| {
            !window[0].is_word("that")
                || !window[1].is_word("would")
                || !window[2].is_word("be")
                || !window[3].is_word("dealt")
        }) {
            return Ok(None);
        }
        idx += 4;
        if !tokens.get(idx).is_some_and(|token| token.is_word("to")) {
            return Ok(None);
        }
        idx += 1;

        let this_turn_rel = find_dispatch_inner_phrase_start(
            &crate::cards::builders::parser::token_word_refs(&tokens[idx..]),
            &["this", "turn"],
        );
        let Some(this_turn_rel) = this_turn_rel else {
            return Ok(None);
        };
        let this_turn_abs = idx + this_turn_rel;
        if this_turn_abs + 2 != tokens.len() {
            return Ok(None);
        }

        let scope_tokens = &tokens[idx..this_turn_abs];
        let Some(split_idx) = find_and_each_other(scope_tokens) else {
            return Ok(None);
        };

        let first_target_tokens = trim_commas(&scope_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = scope_tokens[split_idx + 3..].to_vec();
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;

        return Ok(Some(vec![
            EffectAst::PreventDamage {
                amount: amount.clone(),
                target: first_target,
                duration: Until::EndOfTurn,
            },
            EffectAst::PreventDamageEach {
                amount,
                filter,
                duration: Until::EndOfTurn,
            },
        ]));
    }

    if matches!(verb, Verb::Get | Verb::Gain) {
        if verb_idx == 0 || verb_token_idx + 1 >= tokens.len() {
            return Ok(None);
        }

        let subject_tokens = &tokens[..verb_token_idx];
        if let Ok(full_target) = parse_target_phrase(subject_tokens)
            && let Some((first_target, filter)) = split_full_shared_color_target(&full_target)
        {
            if verb == Verb::Get {
                let modifier_tokens = &tokens[verb_token_idx + 1..];
                let modifier_word = modifier_tokens
                    .first()
                    .and_then(OwnedLexToken::as_word)
                    .ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "missing modifier in shared-color gets clause (clause: '{}')",
                            words_all.join(" ")
                        ))
                    })?;
                let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                        words_all.join(" ")
                    ))
                })?;

                return Ok(Some(vec![
                    EffectAst::Pump {
                        power: Value::Fixed(power),
                        toughness: Value::Fixed(toughness),
                        target: first_target,
                        duration: Until::EndOfTurn,
                        condition: None,
                    },
                    EffectAst::PumpAll {
                        filter,
                        power: Value::Fixed(power),
                        toughness: Value::Fixed(toughness),
                        duration: Until::EndOfTurn,
                    },
                ]));
            }

            if let Some(first_effect) = parse_simple_gain_ability_clause(tokens)?
                && let EffectAst::GrantAbilitiesToTarget {
                    abilities,
                    duration,
                    ..
                } = first_effect
            {
                return Ok(Some(vec![
                    EffectAst::GrantAbilitiesToTarget {
                        target: first_target,
                        abilities: abilities.clone(),
                        duration: duration.clone(),
                    },
                    EffectAst::GrantAbilitiesAll {
                        filter,
                        abilities,
                        duration,
                    },
                ]));
            }
        }

        let Some(and_idx) = find_and_each_other(subject_tokens) else {
            return Ok(None);
        };
        if and_idx == 0 {
            return Ok(None);
        }

        let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;

        if verb == Verb::Get {
            let modifier_tokens = &tokens[verb_token_idx + 1..];
            let modifier_word = modifier_tokens
                .first()
                .and_then(OwnedLexToken::as_word)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing modifier in shared-color gets clause (clause: '{}')",
                        words_all.join(" ")
                    ))
                })?;
            let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
                CardTextError::ParseError(format!(
                    "invalid power/toughness modifier in shared-color gets clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;

            return Ok(Some(vec![
                EffectAst::Pump {
                    power: Value::Fixed(power),
                    toughness: Value::Fixed(toughness),
                    target: first_target,
                    duration: Until::EndOfTurn,
                    condition: None,
                },
                EffectAst::PumpAll {
                    filter,
                    power: Value::Fixed(power),
                    toughness: Value::Fixed(toughness),
                    duration: Until::EndOfTurn,
                },
            ]));
        }

        let mut first_clause = first_target_tokens.clone();
        first_clause.extend_from_slice(&tokens[verb_token_idx..]);
        let Some(first_effect) = parse_simple_gain_ability_clause(&first_clause)? else {
            return Ok(None);
        };
        if let EffectAst::GrantAbilitiesToTarget {
            abilities,
            duration,
            ..
        } = first_effect
        {
            return Ok(Some(vec![
                EffectAst::GrantAbilitiesToTarget {
                    target: first_target,
                    abilities: abilities.clone(),
                    duration: duration.clone(),
                },
                EffectAst::GrantAbilitiesAll {
                    filter,
                    abilities,
                    duration,
                },
            ]));
        }
    }

    Ok(None)
}

pub(crate) fn parse_same_name_gets_fanout_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    if verb != Verb::Get || verb_idx == 0 || verb_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..verb_idx];
    let and_idx = find_token_word_window(subject_tokens, &["and", "all", "other"]);
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };
    if and_idx == 0 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }
    let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
    if second_clause_tokens.is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let modifier_tokens = &tokens[verb_idx + 1..];
    let collapsed_modifier_tokens = collapse_leading_signed_pt_modifier_tokens(modifier_tokens)
        .unwrap_or_else(|| modifier_tokens.to_vec());
    let modifier_word = collapsed_modifier_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing modifier in same-name gets clause (clause: '{}')",
                crate::cards::builders::parser::token_word_refs(tokens).join(" ")
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in same-name gets clause (clause: '{}')",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))
    })?;
    let duration = if grammar::contains_word(&collapsed_modifier_tokens, "until")
        && grammar::contains_word(&collapsed_modifier_tokens, "end")
        && grammar::contains_word(&collapsed_modifier_tokens, "turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    let target = parse_target_phrase(&first_target_tokens)?;
    Ok(Some(vec![
        EffectAst::Pump {
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            target,
            duration: duration.clone(),
            condition: None,
        },
        EffectAst::PumpAll {
            filter,
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            duration,
        },
    ]))
}

pub(crate) fn parse_destroy_or_exile_all_split_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let verb = if words[0] == "destroy" {
        Some(Verb::Destroy)
    } else if words[0] == "exile" {
        Some(Verb::Exile)
    } else {
        None
    };
    let Some(verb) = verb else {
        return Ok(None);
    };
    if words[1] != "all"
        || !grammar::contains_word(tokens, "and")
        || grammar::contains_word(tokens, "except")
    {
        return Ok(None);
    }
    if grammar::words_match_any_prefix(tokens, EXILE_ALL_CARDS_FROM_PREFIXES).is_some()
        && (grammar::contains_word(tokens, "hand") || grammar::contains_word(tokens, "hands"))
        && (grammar::contains_word(tokens, "graveyard")
            || grammar::contains_word(tokens, "graveyards"))
    {
        return Ok(None);
    }

    let mut raw_segments = Vec::new();
    let mut current = Vec::new();
    for token in &tokens[2..] {
        if token.is_word("and") || token.is_comma() {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut effects = Vec::new();
    for mut segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segment.first().is_some_and(|token| token.is_word("all")) {
            segment.remove(0);
        }
        if segment.is_empty() {
            continue;
        }
        let filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in split all clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let effect = match verb {
            Verb::Destroy => EffectAst::DestroyAll { filter },
            Verb::Exile => EffectAst::ExileAll {
                filter,
                face_down: false,
            },
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported split all clause verb".to_string(),
                ));
            }
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        return Ok(Some(effects));
    }
    Ok(None)
}

pub(crate) fn parse_exile_then_return_same_object_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn target_references_it_tag(target: &TargetAst) -> bool {
        match target {
            TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
            TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && matches!(constraint.relation, TaggedOpbjectRelation::IsTaggedObject)
            }),
            _ => false,
        }
    }

    let mut clause_tokens = tokens;
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("you"))
        && clause_tokens
            .get(1)
            .is_some_and(|token| token.is_word("exile"))
    {
        clause_tokens = &clause_tokens[1..];
    }

    let words_all = crate::cards::builders::parser::token_word_refs(clause_tokens);
    if words_all.first().copied() != Some("exile")
        || !grammar::contains_word(clause_tokens, "then")
        || !grammar::contains_word(clause_tokens, "return")
    {
        return Ok(None);
    }

    let split_idx = find_window_by(clause_tokens, 3, |window: &[OwnedLexToken]| {
        window[0].is_comma() && window[1].is_word("then") && window[2].is_word("return")
    });
    let Some(split_idx) = split_idx else {
        return Ok(None);
    };

    let first_clause = &clause_tokens[..split_idx];
    let second_clause = &clause_tokens[split_idx + 2..];
    if first_clause.is_empty() || second_clause.is_empty() {
        return Ok(None);
    }

    let (first_clause, delayed_until_end_of_combat) = if let Some(before) =
        grammar::strip_lexed_suffix_phrase(first_clause, &["at", "the", "end", "of", "combat"])
    {
        (before, true)
    } else if let Some(before) =
        grammar::strip_lexed_suffix_phrase(first_clause, &["at", "end", "of", "combat"])
    {
        (before, true)
    } else {
        (first_clause, false)
    };

    let mut first_effects = parse_effect_chain_inner(first_clause)?;
    if !first_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Exile { .. }))
    {
        return Ok(None);
    }

    // Preserve return follow-up clauses (for example "with a +1/+1 counter on it")
    // while still rewriting the "it" return target to the tagged exiled object.
    let mut second_effects =
        if let Some(effects) = parse_sentence_return_with_counters_on_it(second_clause)? {
            effects
        } else {
            parse_effect_chain_inner(second_clause)?
        };
    let mut rewrote_return = false;
    for effect in &mut second_effects {
        match effect {
            EffectAst::ReturnToBattlefield {
                target,
                tapped: _,
                transformed: _,
                converted: _,
                controller: _,
            } if target_references_it_tag(target) => {
                *target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                rewrote_return = true;
            }
            EffectAst::ReturnToHand { target, random: _ } if target_references_it_tag(target) => {
                *target = TargetAst::Tagged(TagKey::from(IT_TAG), None);
                rewrote_return = true;
            }
            _ => {}
        }
    }
    if !rewrote_return {
        return Ok(None);
    }

    if delayed_until_end_of_combat {
        let mut delayed_effects = first_effects;
        delayed_effects.extend(second_effects);
        return Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
            effects: delayed_effects,
        }]));
    }

    first_effects.extend(second_effects);
    Ok(Some(first_effects))
}

pub(crate) fn parse_exile_up_to_one_each_target_type_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.len() < 6 || words[0] != "exile" {
        return Ok(None);
    }
    if grammar::words_match_prefix(tokens, &["exile", "up", "to", "one", "target"]).is_none() {
        return Ok(None);
    }
    // This primitive is for repeated clauses like:
    // "Exile up to one target artifact, up to one target creature, ..."
    // Not for a single disjunctive target like:
    // "Exile up to one target artifact, creature, or enchantment ..."
    let target_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("target").then_some(idx))
        .collect();
    if target_positions.len() < 2 {
        return Ok(None);
    }
    for pos in target_positions.iter().skip(1) {
        if *pos < 3
            || !tokens[*pos - 3].is_word("up")
            || !tokens[*pos - 2].is_word("to")
            || !tokens[*pos - 1].is_word("one")
        {
            return Ok(None);
        }
    }

    let mut raw_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    let mut current: Vec<OwnedLexToken> = Vec::new();
    for token in &tokens[1..] {
        if token.is_comma() || token.is_word("and") || token.is_word("or") {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut filters = Vec::new();
    for segment in raw_segments {
        let mut slice: &[OwnedLexToken] = &segment;
        if slice.len() >= 3
            && slice[0].is_word("up")
            && slice[1].is_word("to")
            && slice[2].is_word("one")
        {
            slice = &slice[3..];
        }
        if slice.first().is_some_and(|token| token.is_word("target")) {
            slice = &slice[1..];
        }
        if slice.is_empty() {
            continue;
        }

        let mut filter = parse_object_filter(slice, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in 'exile up to one each target type' clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        if filter.controller.is_none() {
            // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects compilation.
            filter.controller = Some(PlayerFilter::Any);
        }
        filters.push(filter);
    }

    if filters.len() < 2 {
        return Ok(None);
    }

    let tag = helper_tag_for_tokens(tokens, "exiled");
    let mut effects: Vec<EffectAst> = filters
        .into_iter()
        .map(|filter| EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        })
        .collect();
    effects.push(EffectAst::Exile {
        target: TargetAst::Tagged(tag, None),
        face_down: false,
    });

    Ok(Some(effects))
}

pub(crate) fn parse_look_at_hand_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    if words.as_slice()
        == [
            "look",
            "at",
            "an",
            "opponents",
            "hand",
            "then",
            "choose",
            "any",
            "card",
            "name",
        ]
    {
        return Ok(Some(vec![
            EffectAst::LookAtHand {
                target: TargetAst::Player(PlayerFilter::Opponent, None),
            },
            EffectAst::ChooseCardName {
                player: PlayerAst::You,
                filter: None,
                tag: TagKey::from(IT_TAG),
            },
        ]));
    }
    if words.as_slice() == ["look", "at", "target", "players", "hand"]
        || words.as_slice() == ["look", "at", "target", "player", "hand"]
    {
        let target = TargetAst::Player(PlayerFilter::target_player(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "target", "opponent", "hand"]
        || words.as_slice() == ["look", "at", "target", "opponents", "hand"]
    {
        let target =
            TargetAst::Player(PlayerFilter::target_opponent(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "an", "opponents", "hand"]
        || words.as_slice() == ["look", "at", "opponents", "hand"]
    {
        let target = TargetAst::Player(PlayerFilter::Opponent, None);
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "that", "players", "hand"] {
        let target = TargetAst::Player(PlayerFilter::IteratedPlayer, None);
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    Ok(None)
}

pub(crate) fn parse_look_at_top_then_exile_one_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    let starts_with_look_top = slice_starts_with_any(
        &clause_words,
        &[&["look", "at", "the", "top"], &["look", "at", "top"]],
    );
    if !starts_with_look_top {
        return Ok(None);
    }

    let Some(top_idx) = find_index(tokens, |token| token.is_word("top")) else {
        return Ok(None);
    };
    let Some((count, used_count)) = parse_number(&tokens[top_idx + 1..]) else {
        return Ok(None);
    };
    let mut idx = top_idx + 1 + used_count;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
    {
        idx += 1;
    }
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return Ok(None);
    }
    idx += 1;

    let Some(library_idx) =
        find_index(&tokens[idx..], |token| token.is_word("library")).map(|offset| idx + offset)
    else {
        return Ok(None);
    };
    let owner_tokens = trim_commas(&tokens[idx..library_idx]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let player = match parse_subject(&owner_tokens) {
        SubjectAst::Player(player) => player,
        _ => return Ok(None),
    };

    let mut tail_tokens = trim_commas(&tokens[library_idx + 1..]).to_vec();
    while tail_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        tail_tokens.remove(0);
    }
    let tail_words = crate::cards::builders::parser::token_word_refs(&tail_tokens);
    let looks_like_exile_one_of_looked = slice_starts_with_any(
        &tail_words,
        &[
            &["exile", "one", "of", "them"],
            &["exile", "one", "of", "those"],
            &["exile", "one", "of", "those", "cards"],
        ],
    );
    if !looks_like_exile_one_of_looked {
        return Ok(None);
    }

    let looked_tag = helper_tag_for_tokens(tokens, "looked");
    let chosen_tag = helper_tag_for_tokens(tokens, "chosen");
    let mut looked_filter = ObjectFilter::tagged(looked_tag.clone());
    looked_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::LookAtTopCards {
            player,
            count: Value::Fixed(count as i32),
            tag: looked_tag,
        },
        EffectAst::ChooseObjects {
            filter: looked_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
        },
        EffectAst::Exile {
            target: TargetAst::Tagged(chosen_tag, None),
            face_down: false,
        },
    ]))
}

pub(crate) fn parse_gain_life_equal_to_age_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Legacy fallback previously returned a hardcoded 0-life effect for age-counter clauses.
    // Let generic life parsing handle these so counter-scaled amounts compile correctly.
    let _ = tokens;
    Ok(None)
}

pub(crate) fn parse_you_and_each_opponent_voted_with_you_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    let pattern = [
        "you", "and", "each", "opponent", "who", "voted", "for", "a", "choice", "you", "voted",
        "for", "may", "scry",
    ];

    if words.len() < pattern.len() {
        return Ok(None);
    }

    if grammar::words_match_prefix(tokens, &pattern).is_none() {
        return Ok(None);
    }

    let scry_index = pattern.len() - 1;
    let value_tokens = &tokens[(scry_index + 1)..];
    let Some((count, _)) = parse_value(value_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing scry count in vote-with-you clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let you_effect = EffectAst::May {
        effects: vec![EffectAst::Scry {
            count: count.clone(),
            player: PlayerAst::You,
        }],
    };

    let opponent_effect = EffectAst::ForEachTaggedPlayer {
        tag: TagKey::from("voted_with_you"),
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::Scry {
                count,
                player: PlayerAst::Implicit,
            }],
        }],
    };

    Ok(Some(vec![you_effect, opponent_effect]))
}

pub(crate) fn parse_gain_life_equal_to_power_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    let Some(gain_idx) = find_index(words.as_slice(), |word| matches!(*word, "gain" | "gains"))
    else {
        return Ok(None);
    };

    if words.get(gain_idx + 1) != Some(&"life")
        || words.get(gain_idx + 2) != Some(&"equal")
        || words.get(gain_idx + 3) != Some(&"to")
    {
        return Ok(None);
    }

    let tail = &words[gain_idx + 4..];
    let has_its_power = contains_word_window(tail, &["its", "power"]);
    if !has_its_power {
        return Ok(None);
    }

    let subject = if gain_idx > 0 {
        Some(parse_subject(&tokens[..gain_idx]))
    } else {
        None
    };
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
    Ok(Some(vec![EffectAst::GainLife { amount, player }]))
}

pub(crate) fn parse_scaled_target_power_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let word_storage = DispatchInnerNormalizedWords::new(tokens);
    let words = word_storage.to_word_refs();
    let Some((verb, multiplier)) = word_storage.first().and_then(|word| match word {
        "double" => Some(("double", 1)),
        "triple" => Some(("triple", 2)),
        _ => None,
    }) else {
        return Ok(None);
    };

    let scaled_stat = |value: Value| {
        if multiplier == 1 {
            value
        } else {
            Value::Scaled(Box::new(value), multiplier)
        }
    };

    let scale_pt_from_value_spec =
        |target: &TargetAst, include_power: bool, include_toughness: bool| {
            let amount_source_filter =
                target_ast_to_object_filter(target.clone()).unwrap_or_else(|| {
                    let mut fallback = ObjectFilter::default();
                    fallback.card_types.push(CardType::Creature);
                    fallback
                });
            let value_spec = Box::new(ChooseSpec::target(ChooseSpec::Object(amount_source_filter)));
            EffectAst::Pump {
                power: if include_power {
                    scaled_stat(Value::PowerOf(value_spec.clone()))
                } else {
                    Value::Fixed(0)
                },
                toughness: if include_toughness {
                    scaled_stat(Value::ToughnessOf(value_spec))
                } else {
                    Value::Fixed(0)
                },
                target: target.clone(),
                duration: crate::effect::Until::EndOfTurn,
                condition: None,
            }
        };
    let scale_pt_all = |filter: ObjectFilter, include_power: bool, include_toughness: bool| {
        EffectAst::ScalePowerToughnessAll {
            filter,
            power: include_power,
            toughness: include_toughness,
            multiplier,
            duration: crate::effect::Until::EndOfTurn,
        }
    };
    let parse_double_life_total_subject =
        |subject_words: &[&str]| -> Option<(PlayerAst, PlayerFilter)> {
            match subject_words {
                ["your"] => Some((PlayerAst::You, PlayerFilter::You)),
                ["target", "player"] | ["target", "players"] => {
                    Some((PlayerAst::Target, PlayerFilter::target_player()))
                }
                ["target", "opponent"] | ["target", "opponents"] => {
                    Some((PlayerAst::TargetOpponent, PlayerFilter::target_opponent()))
                }
                ["opponent"] | ["opponents"] | ["an", "opponent"] | ["an", "opponents"] => {
                    Some((PlayerAst::Opponent, PlayerFilter::Opponent))
                }
                _ => None,
            }
        };
    let parse_double_mana_pool_subject = |subject_words: &[&str]| -> Option<PlayerAst> {
        match subject_words {
            ["you", "have"] => Some(PlayerAst::You),
            ["target", "player", "has"] | ["target", "player", "have"] => Some(PlayerAst::Target),
            ["target", "opponent", "has"] | ["target", "opponent", "have"] => {
                Some(PlayerAst::TargetOpponent)
            }
            ["opponent", "has"] | ["opponents", "have"] => Some(PlayerAst::Opponent),
            _ => None,
        }
    };

    if verb == "double"
        && let Some(life_total_idx) = find_index(words.as_slice(), |word| *word == "life")
        && words.get(life_total_idx + 1) == Some(&"total")
        && let Some((player, player_filter)) =
            parse_double_life_total_subject(&words[1..life_total_idx])
        && life_total_idx + 2 == words.len()
    {
        return Ok(Some(vec![EffectAst::SetLifeTotal {
            amount: Value::Scaled(Box::new(Value::LifeTotal(player_filter)), 2),
            player,
        }]));
    }

    let mana_prefix = [
        "double", "the", "amount", "of", "each", "type", "of", "unspent", "mana",
    ];
    if verb == "double"
        && slice_starts_with(&words, &mana_prefix)
        && let Some(player) = parse_double_mana_pool_subject(&words[mana_prefix.len()..])
    {
        return Ok(Some(vec![EffectAst::DoubleManaPool { player }]));
    }

    let duration_start = if words.len() >= 4 && is_until_end_of_turn(&words[words.len() - 4..]) {
        words.len() - 4
    } else {
        words.len()
    };
    let subject_end = duration_start;

    if words.first().copied() == Some(verb) && words.get(1).copied() == Some("the") {
        let (include_power, include_toughness, subject_start) = match words.get(2..) {
            Some(["power", "of", ..]) => (true, false, 4),
            Some(["toughness", "of", ..]) => (false, true, 4),
            Some(["power", "and", "toughness", "of", ..]) => (true, true, 6),
            _ => (false, false, 0),
        };
        if subject_start != 0 && subject_start < subject_end {
            let subject_tokens = trim_commas(&tokens[subject_start..subject_end]);
            if subject_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing subject in {verb} clause (clause: '{}')",
                    words.join(" ")
                )));
            }

            let subject_words = &words[subject_start..subject_end];
            if subject_words
                .first()
                .is_some_and(|word| *word == "each" || *word == "all")
            {
                let filter_tokens = &subject_tokens[1..];
                if filter_tokens.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "missing filter in {verb} clause (clause: '{}')",
                        words.join(" ")
                    )));
                }
                let filter = parse_object_filter(filter_tokens, false)?;
                return Ok(Some(vec![scale_pt_all(
                    filter,
                    include_power,
                    include_toughness,
                )]));
            }

            let target = parse_target_phrase(&subject_tokens)?;
            return Ok(Some(vec![scale_pt_from_value_spec(
                &target,
                include_power,
                include_toughness,
            )]));
        }
    }

    let (include_power, include_toughness, characteristic_start) = if subject_end >= 4
        && words[subject_end - 3..subject_end] == ["power", "and", "toughness"]
    {
        (true, true, subject_end - 3)
    } else if subject_end >= 1 && words[subject_end - 1] == "power" {
        (true, false, subject_end - 1)
    } else if subject_end >= 1 && words[subject_end - 1] == "toughness" {
        (false, true, subject_end - 1)
    } else {
        return Ok(None);
    };
    if characteristic_start <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..characteristic_start]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in {verb} clause (clause: '{}')",
            words.join(" ")
        )));
    }

    if words
        .get(1)
        .is_some_and(|word| *word == "each" || *word == "all")
    {
        let filter_tokens = &target_tokens[1..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter in {verb} clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(Some(vec![scale_pt_all(
            filter,
            include_power,
            include_toughness,
        )]));
    }

    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(vec![scale_pt_from_value_spec(
        &target,
        include_power,
        include_toughness,
    )]))
}

pub(crate) fn parse_prevent_damage_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    let prefix = ["prevent", "all", "combat", "damage"];
    if grammar::words_match_prefix(tokens, &prefix).is_none() {
        return Ok(None);
    }

    let Some(this_turn_idx) = find_dispatch_inner_phrase_start(&words, &["this", "turn"]) else {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    };
    if find_dispatch_inner_phrase_start(&words[this_turn_idx + 2..], &["this", "turn"]).is_some() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }
    if this_turn_idx < prefix.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut core_words = Vec::with_capacity(words.len() - prefix.len() - 2);
    core_words.extend_from_slice(&words[prefix.len()..this_turn_idx]);
    core_words.extend_from_slice(&words[this_turn_idx + 2..]);
    let mut core_tokens = Vec::with_capacity(tokens.len() - prefix.len() - 2);
    core_tokens.extend_from_slice(&tokens[prefix.len()..this_turn_idx]);
    core_tokens.extend_from_slice(&tokens[this_turn_idx + 2..]);
    let core_words = core_words;
    let core_tokens = core_tokens;

    if core_words == ["that", "would", "be", "dealt"] {
        return Ok(Some(EffectAst::PreventAllCombatDamage {
            duration: Until::EndOfTurn,
        }));
    }

    if grammar::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_BY_PREFIXES).is_some() {
        let source_tokens = &core_tokens[5..];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    if grammar::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_TO_AND_BY_PREFIXES).is_some() {
        let source_tokens = &core_tokens[8..];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    if grammar::words_match_any_prefix(&core_tokens, PREVENT_DAMAGE_TO_PREFIXES).is_some() {
        return parse_prevent_damage_target_scope(&core_tokens[5..], &words);
    }

    if let Some(would_idx) = find_index(core_words.as_slice(), |word| *word == "would")
        && core_words.get(would_idx + 1) == Some(&"deal")
    {
        let source_tokens = &core_tokens[..would_idx];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all-combat-damage clause tail (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn parse_prevent_damage_source_target(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<TargetAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all source target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let source_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let is_explicit_reference = slice_contains(&source_words, &"target")
        || source_words
            .first()
            .is_some_and(|word| matches!(*word, "this" | "that" | "it"));
    if !is_explicit_reference {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            source_words.join(" ")
        )));
    }

    let source = parse_target_phrase(tokens)?;
    match source {
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _) => Ok(source),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            crate::cards::builders::parser::token_word_refs(tokens).join(" ")
        ))),
    }
}

pub(crate) fn parse_prevent_damage_target_scope(
    tokens: &[OwnedLexToken],
    clause_words: &[&str],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words: Vec<&str> = crate::cards::builders::parser::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if target_words.as_slice() == ["player"] || target_words.as_slice() == ["players"] {
        return Ok(Some(EffectAst::PreventAllCombatDamageToPlayers {
            duration: Until::EndOfTurn,
        }));
    }
    if target_words.as_slice() == ["you"] {
        return Ok(Some(EffectAst::PreventAllCombatDamageToYou {
            duration: Until::EndOfTurn,
        }));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all target scope '{}'",
        crate::cards::builders::parser::token_word_refs(tokens).join(" ")
    )))
}

pub(crate) fn parse_gain_x_plus_life_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::cards::builders::parser::token_word_refs(tokens);
    let Some(gain_idx) = find_index(words.as_slice(), |word| matches!(*word, "gain" | "gains"))
    else {
        return Ok(None);
    };
    if gain_idx > 0
        && matches!(
            words[gain_idx - 1],
            "cant" | "cannot" | "doesnt" | "don't" | "dont"
        )
    {
        return Ok(None);
    }

    if words.len() <= gain_idx + 4 {
        return Ok(None);
    }

    if words[gain_idx + 1] != "x" || words[gain_idx + 2] != "plus" {
        return Ok(None);
    }

    let (bonus, number_used) = parse_number(&tokens[gain_idx + 3..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let life_idx = gain_idx + 3 + number_used;
    if !tokens
        .get(life_idx)
        .is_some_and(|token| token.is_word("life"))
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in gain-x-plus-life clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..gain_idx];
    let player = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let trailing_tokens = trim_commas(&tokens[life_idx + 1..]);
    let x_value = if trailing_tokens.is_empty() {
        Value::X
    } else if let Some(where_x) = parse_where_x_value_clause(&trailing_tokens) {
        where_x
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported gain-x-plus-life trailing clause (clause: '{}')",
            words.join(" ")
        )));
    };
    let amount = Value::Add(Box::new(x_value), Box::new(Value::Fixed(bonus as i32)));
    let effects = vec![EffectAst::GainLife { amount, player }];

    Ok(Some(effects))
}

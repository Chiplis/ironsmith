#![allow(dead_code)]

use super::super::compile_support::effects_reference_it_tag;
use super::super::effect_ast_traversal::for_each_nested_effects_mut;
use super::super::grammar::effects::{
    parse_cant_effect_sentence_with_grammar_entrypoint_lexed,
    parse_search_library_sentence_with_grammar_entrypoint_lexed,
};
use super::super::grammar::primitives::{self as grammar, TokenWordView};
use super::super::grammar::structure::{
    LeadingResultPrefixKind, split_leading_result_prefix_lexed, split_trailing_if_clause_lexed,
};
use super::super::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use super::super::permission_helpers::{
    parse_additional_land_plays_clause_lexed, parse_permission_clause_spec_lexed,
    parse_unsupported_play_cast_permission_clause_lexed,
};
use super::super::token_primitives::{
    find_window_index as find_word_sequence_index, rfind_index as find_last_token_index,
    slice_contains_str as word_slice_contains, slice_starts_with as word_slice_starts_with,
    str_contains as string_contains,
};
use super::super::value_helpers::{parse_number_from_lexed, parse_value_from_lexed};
use super::lex_chain_helpers::{
    find_verb_lexed, has_effect_head_without_verb_lexed, segment_has_effect_head_lexed,
    split_effect_chain_on_and_lexed, split_segments_on_comma_effect_head_lexed,
    split_segments_on_comma_then_lexed, strip_leading_instead_prefix_lexed,
};
use super::sentence_helpers::*;
use super::{
    parse_effect_clause_lexed, parse_effect_sentence_lexed, parse_predicate_lexed,
    parse_sentence_exile_source_with_counters_lexed,
    parse_sentence_put_onto_battlefield_with_counters_on_it_lexed,
    parse_sentence_return_with_counters_on_it_lexed, parse_simple_gain_ability_clause_lexed,
    parse_simple_lose_ability_clause_lexed, parse_token_copy_followup_sentence_lexed,
    try_apply_token_copy_followup,
};
#[allow(unused_imports)]
use crate::cards::builders::{
    CardTextError, EffectAst, PlayerAst, PredicateAst, TargetAst, TextSpan,
};
use crate::effect::ChoiceCount;
use crate::target::PlayerFilter;
use crate::zone::Zone;

fn synthetic_lexed_word(word: &str) -> OwnedLexToken {
    OwnedLexToken::word(word, TextSpan::synthetic())
}

fn contains_char(text: &str, expected: char) -> bool {
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == expected {
            return true;
        }
    }

    false
}

fn starts_like_create_fragment_lexed(tokens: &[OwnedLexToken]) -> bool {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    let Some(first_word) = words.first().copied() else {
        return false;
    };
    let starts_like_count = matches!(
        first_word,
        "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six"
    ) || parse_number_from_lexed(tokens).is_some()
        || contains_char(first_word, '/')
        || first_word == "x";
    starts_like_count && words.iter().any(|word| matches!(*word, "token" | "tokens"))
}

pub(crate) fn looks_like_multi_create_chain_lexed(tokens: &[OwnedLexToken]) -> bool {
    matches!(find_verb_lexed(tokens), Some((Verb::Create, _)))
        && token_word_refs(tokens)
            .iter()
            .filter(|word| matches!(**word, "token" | "tokens"))
            .count()
            >= 2
}

pub(crate) fn parse_effect_chain_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if grammar::words_match_prefix(tokens, &["exile", "them"]).is_some()
        && let Some(meld_idx) =
            find_word_sequence_index(&clause_words, &["then", "meld", "them", "into"])
    {
        let result_words = &clause_words[meld_idx + 4..];
        if result_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing meld result name (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(vec![EffectAst::Meld {
            result_name: result_words.join(" "),
            enters_tapped: false,
            enters_attacking: false,
        }]);
    }

    if let Some(stripped) = strip_leading_instead_prefix_lexed(tokens) {
        return parse_effect_chain_lexed(stripped);
    }
    let starts_with_each_opponent = grammar::words_match_prefix(tokens, &["each", "opponent"]).is_some()
        || grammar::words_match_prefix(tokens, &["each", "opponents"]).is_some();
    let starts_with_each_player = grammar::words_match_prefix(tokens, &["each", "player"]).is_some()
        || grammar::words_match_prefix(tokens, &["each", "players"]).is_some();

    if let Some(player) = parse_leading_player_may_lexed(tokens) {
        let mut stripped = remove_through_first_word_lexed(tokens, "may");
        if stripped
            .first()
            .is_some_and(|token| token.is_word("have") || token.is_word("has"))
        {
            stripped.remove(0);
        }
        let mut effects = parse_effect_chain_lexed(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        if leading_may_is_permission_clause_lexed(&stripped)? {
            return Ok(effects);
        }
        return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
    }

    if tokens.first().is_some_and(|token| token.is_word("may"))
        && !starts_with_each_opponent
        && !starts_with_each_player
    {
        let stripped = remove_first_word_lexed(tokens, "may");
        if leading_may_is_permission_clause_lexed(&stripped)? {
            return parse_effect_chain_lexed(&stripped);
        }
        let effects = parse_effect_chain_lexed(&stripped)?;
        return Ok(vec![EffectAst::May { effects }]);
    }

    if let Some(unless_action) = parse_or_action_clause_lexed(tokens)? {
        return Ok(vec![unless_action]);
    }

    parse_effect_chain_with_sentence_primitives_lexed(tokens)
}

fn leading_may_is_permission_clause_lexed(tokens: &[OwnedLexToken]) -> Result<bool, CardTextError> {
    Ok(parse_additional_land_plays_clause_lexed(tokens)?.is_some()
        || parse_permission_clause_spec_lexed(tokens)?.is_some()
        || parse_unsupported_play_cast_permission_clause_lexed(tokens)?.is_some())
}

fn starts_with_until_end_of_turn_trigger_clause(clause_words: &[&str]) -> bool {
    (word_slice_starts_with(clause_words, &["until", "end", "of", "turn"])
        || word_slice_starts_with(clause_words, &["until", "the", "end", "of", "turn"]))
        && clause_words
            .get(if clause_words.get(1) == Some(&"the") {
                5
            } else {
                4
            })
            .is_some_and(|word| matches!(*word, "when" | "whenever" | "at"))
}

fn is_would_enter_replacement_clause(clause_words: &[&str]) -> bool {
    clause_words.iter().any(|word| *word == "would")
        && clause_words
            .iter()
            .any(|word| *word == "enter" || *word == "enters")
        && clause_words.iter().any(|word| *word == "instead")
}

fn is_comparison_or_delimiter_lexed(tokens: &[OwnedLexToken], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("or")) {
        return false;
    }
    let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
    let next_word = tokens.get(idx + 1).and_then(OwnedLexToken::as_word);
    if matches!(next_word, Some("less" | "greater" | "more" | "fewer")) {
        return true;
    }
    previous_word == Some("than") && next_word == Some("equal")
}

fn split_on_or_lexed(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    let mut segments = Vec::new();
    let mut start = 0usize;

    for (idx, token) in tokens.iter().enumerate() {
        let is_separator = token.kind == TokenKind::Comma
            || (token.is_word("or") && !is_comparison_or_delimiter_lexed(tokens, idx));
        if !is_separator {
            continue;
        }
        let current = trim_lexed_commas(&tokens[start..idx]);
        if !current.is_empty() {
            segments.push(current);
        }
        start = idx + 1;
    }

    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail);
    }

    segments
}

fn normalize_or_action_option_lexed(mut option: &[OwnedLexToken]) -> &[OwnedLexToken] {
    while option
        .first()
        .is_some_and(|token| token.is_word("and") || token.is_word("or"))
    {
        option = &option[1..];
    }
    trim_lexed_commas(option)
}

pub(crate) fn parse_or_action_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    if !grammar::contains_word(tokens, "or") {
        return Ok(None);
    }

    let mut option_tokens = split_on_or_lexed(tokens);
    if option_tokens.len() != 2 {
        return Ok(None);
    }

    let first = normalize_or_action_option_lexed(option_tokens.remove(0));
    let second = normalize_or_action_option_lexed(option_tokens.remove(0));
    if first.is_empty() || second.is_empty() {
        return Ok(None);
    }

    let first_starts_effect = find_verb_lexed(first).is_some_and(|(_, verb_idx)| verb_idx == 0)
        || has_effect_head_without_verb_lexed(first);
    let second_starts_effect = find_verb_lexed(second).is_some_and(|(_, verb_idx)| verb_idx == 0)
        || has_effect_head_without_verb_lexed(second);
    if !first_starts_effect || !second_starts_effect {
        return Ok(None);
    }

    let first_effects = match parse_effect_chain_with_sentence_primitives_lexed(first) {
        Ok(effects) if !effects.is_empty() => effects,
        _ => return Ok(None),
    };
    let second_effects = match parse_effect_chain_with_sentence_primitives_lexed(second) {
        Ok(effects) if !effects.is_empty() => effects,
        _ => return Ok(None),
    };

    Ok(Some(EffectAst::UnlessAction {
        effects: first_effects,
        alternative: second_effects,
        player: PlayerAst::Implicit,
    }))
}

#[cfg(test)]
mod tests {
    use crate::cards::builders::{CardDefinitionBuilder, PlayerAst};
    use crate::ids::CardId;

    use super::super::super::lexer::lex_line;
    use super::{parse_leading_player_may_lexed, starts_like_create_fragment_lexed};

    #[test]
    fn leading_may_land_play_permission_does_not_lower_to_may_effect() {
        let def = CardDefinitionBuilder::new(CardId::from_raw(1), "Explore")
            .parse_text("You may play an additional land this turn.\nDraw a card.")
            .expect("explore-style text should parse");

        let spell_debug = format!("{:?}", def.spell_effect.as_ref().expect("spell effects"));
        assert!(
            super::string_contains(&spell_debug, "AdditionalLandPlaysEffect"),
            "expected Explore-style permission text to lower to additional land plays, got {spell_debug}"
        );
        assert!(
            !super::string_contains(&spell_debug, "MayEffect"),
            "permission-granting land-play text should not become a MayEffect: {spell_debug}"
        );
    }

    #[test]
    fn create_fragment_probe_accepts_capitalized_pt_token_clauses() {
        let tokens = lex_line("Two 1/1 white Soldier creature tokens", 0)
            .expect("rewrite lexer should classify create-fragment text");

        assert!(starts_like_create_fragment_lexed(&tokens));
    }

    #[test]
    fn leading_player_may_probe_accepts_capitalized_opponent_clauses() {
        let tokens = lex_line("An opponent may cast it", 0)
            .expect("rewrite lexer should classify player-may text");

        assert_eq!(
            parse_leading_player_may_lexed(&tokens),
            Some(PlayerAst::Opponent)
        );
    }
}

pub(crate) fn parse_effect_chain_with_sentence_primitives_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if tokens.first().is_some_and(|token| token.is_word("and")) {
        return parse_effect_chain_with_sentence_primitives_lexed(&tokens[1..]);
    }

    let clause_words = crate::cards::builders::parser::token_word_refs(tokens);
    if starts_with_until_end_of_turn_trigger_clause(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported until-end-of-turn permission clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_would_enter_replacement_clause(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported would-enter replacement clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(effects) = run_sentence_primitives_lexed(
        tokens,
        PRE_CONDITIONAL_SENTENCE_PRIMITIVES,
        &PRE_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
    )? {
        return Ok(effects);
    }
    if let Some(effects) = run_sentence_primitives_lexed(
        tokens,
        POST_CONDITIONAL_SENTENCE_PRIMITIVES,
        &POST_CONDITIONAL_SENTENCE_PRIMITIVE_INDEX,
    )? {
        return Ok(effects);
    }
    parse_effect_chain_inner_lexed(tokens)
}

pub(crate) fn parse_effect_chain_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(stripped) = strip_leading_instead_prefix_lexed(tokens) {
        return parse_effect_chain_inner_lexed(stripped);
    }

    if let Some(effects) = parse_search_library_sentence_with_grammar_entrypoint_lexed(tokens)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let raw_segments = split_effect_chain_on_and_lexed(tokens);
    let mut lexed_segments = Vec::new();
    for segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        lexed_segments.push(segment);
    }

    let mut merged_lexed_segments: Vec<Vec<OwnedLexToken>> = Vec::new();
    for lexed_segment in lexed_segments {
        let segment = lexed_segment.to_vec();
        if merged_lexed_segments.is_empty() {
            merged_lexed_segments.push(segment);
            continue;
        }
        if !super::lex_chain_helpers::segment_has_effect_head_lexed(&segment) {
            if let Some(previous) = merged_lexed_segments.last()
                && let Some(expanded) = expand_missing_verb_segment_lexed(previous, &segment)
            {
                merged_lexed_segments.push(expanded);
                continue;
            }
            let last = merged_lexed_segments
                .last_mut()
                .expect("non-empty segments");
            last.push(synthetic_lexed_word("and"));
            last.extend(segment);
            continue;
        }
        merged_lexed_segments.push(segment);
    }
    while merged_lexed_segments.len() > 1
        && !super::lex_chain_helpers::segment_has_effect_head_lexed(&merged_lexed_segments[0])
    {
        let mut first = merged_lexed_segments.remove(0);
        first.push(synthetic_lexed_word("and"));
        let mut next = merged_lexed_segments.remove(0);
        first.append(&mut next);
        merged_lexed_segments.insert(0, first);
    }
    let merged_segment_slices = merged_lexed_segments
        .iter()
        .map(Vec::as_slice)
        .collect::<Vec<_>>();
    let mut segments: Vec<Vec<OwnedLexToken>> = split_segments_on_comma_effect_head_lexed(
        split_segments_on_comma_then_lexed(merged_segment_slices),
    )
    .into_iter()
    .map(|segment| segment.to_vec())
    .collect();
    segments = expand_segments_with_comma_action_clauses_lexed(segments);
    segments = expand_segments_with_multi_create_clauses_lexed(segments);
    let mut carried_context: Option<CarryContext> = None;
    for segment in segments {
        let segment_effects =
            if let Some(effects) = parse_sentence_return_with_counters_on_it_lexed(&segment)? {
                Some(effects)
            } else if let Some(effects) =
                parse_sentence_put_onto_battlefield_with_counters_on_it_lexed(&segment)?
            {
                Some(effects)
            } else if let Some(prefix) = split_leading_result_prefix_lexed(&segment) {
                Some(vec![match prefix.kind {
                    LeadingResultPrefixKind::If => EffectAst::IfResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_sentence_lexed(prefix.trailing_tokens)?,
                    },
                    LeadingResultPrefixKind::When => EffectAst::WhenResult {
                        predicate: prefix.predicate,
                        effects: parse_effect_sentence_lexed(prefix.trailing_tokens)?,
                    },
                }])
            } else {
                parse_sentence_exile_source_with_counters_lexed(&segment)?
            };
        if let Some(segment_effects) = segment_effects {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(segment_effects) =
            parse_search_library_sentence_with_grammar_entrypoint_lexed(&segment)?
        {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(segment_effects) =
            parse_cant_effect_sentence_with_grammar_entrypoint_lexed(&segment)?
        {
            for mut effect in segment_effects {
                if let Some(context) = carried_context {
                    maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
                }
                if let Some(context) = explicit_player_for_carry(&effect) {
                    carried_context = Some(context);
                }
                effects.push(effect);
            }
            continue;
        }
        if let Some(followup) = parse_token_copy_followup_sentence_lexed(&segment)
            && try_apply_token_copy_followup(&mut effects, followup)?
        {
            continue;
        }
        let mut effect = parse_effect_clause_with_trailing_if_lexed(&segment)?;
        if let Some(context) = carried_context {
            maybe_apply_carried_player_with_clause_lexed(&mut effect, context, &segment);
        }
        if let Some(context) = explicit_player_for_carry(&effect) {
            carried_context = Some(context);
        }
        effects.push(effect);
    }
    collapse_for_each_player_it_tag_followups(&mut effects);
    collapse_token_copy_next_end_step_exile_followup_lexed(&mut effects, tokens);
    collapse_token_copy_next_end_step_sacrifice_followup_lexed(&mut effects, tokens);
    collapse_token_copy_end_of_combat_exile_followup_lexed(&mut effects, tokens);
    Ok(effects)
}

pub(crate) fn collapse_for_each_player_it_tag_followups(effects: &mut Vec<EffectAst>) {
    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let should_merge = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::ForEachPlayer { .. },
                EffectAst::ForEachPlayer {
                    effects: followup_effects,
                },
            ) => effects_reference_it_tag(followup_effects),
            _ => false,
        };

        if !should_merge {
            idx += 1;
            continue;
        }

        let followup = effects.remove(idx + 1);
        match (&mut effects[idx], followup) {
            (
                EffectAst::ForEachPlayer {
                    effects: first_effects,
                },
                EffectAst::ForEachPlayer {
                    effects: mut followup_effects,
                },
            ) => {
                first_effects.append(&mut followup_effects);
            }
            _ => {
                // Defensive: should be unreachable given should_merge checks.
            }
        }
        // Re-check this index in case we have a longer chain of followups.
    }
}

pub(crate) fn parse_effect_clause_with_trailing_if_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let Some(trailing_if) = split_trailing_if_clause_lexed(tokens) else {
        return parse_effect_clause_lexed(tokens);
    };
    let predicate = trailing_if.predicate;
    if !trailing_if_predicate_supported(&predicate) {
        return parse_effect_clause_lexed(tokens);
    }

    let base_effect = if let Ok(effect) = parse_effect_clause_lexed(trailing_if.leading_tokens) {
        effect
    } else {
        if let Some(effect) = parse_simple_lose_ability_clause_lexed(trailing_if.leading_tokens)? {
            effect
        } else if let Some(effect) =
            parse_simple_gain_ability_clause_lexed(trailing_if.leading_tokens)?
        {
            effect
        } else {
            return parse_effect_clause_lexed(tokens);
        }
    };

    Ok(EffectAst::Conditional {
        predicate,
        if_true: vec![base_effect],
        if_false: Vec::new(),
    })
}

fn trailing_if_predicate_supported(predicate: &PredicateAst) -> bool {
    matches!(
        predicate,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::ItMatches(_)
            | PredicateAst::PlayerControlsMoreThanYou { .. }
            | PredicateAst::PlayerLifeAtMostHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerLifeLessThanHalfStartingLifeTotal { .. }
            | PredicateAst::PlayerHasMoreLifeThanYou { .. }
            | PredicateAst::PlayerHasMoreCardsInHandThanYou { .. }
    ) || matches!(predicate, PredicateAst::TaggedMatches(tag, _) if tag.as_str() == "enchanted")
}

pub(crate) fn is_beginning_of_end_step_words(words: &[&str]) -> bool {
    find_word_sequence_index(words, &["beginning", "of", "the", "end", "step"]).is_some()
        || find_word_sequence_index(words, &["beginning", "of", "next", "end", "step"]).is_some()
        || find_word_sequence_index(words, &["beginning", "of", "the", "next", "end", "step"])
            .is_some()
}

pub(crate) fn is_end_of_combat_words(words: &[&str]) -> bool {
    find_word_sequence_index(words, &["end", "of", "combat"]).is_some()
}

pub(crate) fn target_is_generic_token_filter(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.token
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.tagged_constraints.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let chain_words = token_word_refs(tokens);
    if !grammar::contains_word(tokens, "exile")
        || !grammar::contains_word(tokens, "token")
        || !is_beginning_of_end_step_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::CreateTokenCopy { .. } | EffectAst::CreateTokenCopyFromSource { .. },
                EffectAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                },
            ) => target_is_generic_token_filter(target),
            (
                EffectAst::CreateTokenCopy { .. } | EffectAst::CreateTokenCopyFromSource { .. },
                EffectAst::Exile { target, .. },
            ) => target_is_generic_token_filter(target),
            _ => false,
        };

        if !mark_next_end_step_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::CreateTokenCopy {
                exile_at_next_end_step,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                exile_at_next_end_step,
                ..
            } => {
                *exile_at_next_end_step = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_next_end_step_sacrifice_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let chain_words = token_word_refs(tokens);
    if !grammar::contains_word(tokens, "sacrifice")
        || !grammar::contains_word(tokens, "token")
        || grammar::words_find_phrase(tokens, &["next", "end", "step", "repeat"]).is_none()
            && !is_beginning_of_end_step_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_sacrifice = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::CreateTokenCopy { .. } | EffectAst::CreateTokenCopyFromSource { .. },
                EffectAst::Sacrifice { filter, count, .. },
            ) => *count == 1 && filter.token,
            _ => false,
        };

        if !mark_next_end_step_sacrifice {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::CreateTokenCopy {
                sacrifice_at_next_end_step,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                sacrifice_at_next_end_step,
                ..
            } => {
                *sacrifice_at_next_end_step = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup_lexed(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    let chain_words = token_word_refs(tokens);
    if !grammar::contains_word(tokens, "exile")
        || !grammar::contains_word(tokens, "token")
        || !is_end_of_combat_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_end_of_combat_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::CreateTokenCopy { .. }
                | EffectAst::CreateTokenCopyFromSource { .. }
                | EffectAst::CreateTokenWithMods { .. },
                EffectAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                },
            ) => target_is_generic_token_filter(target),
            (
                EffectAst::CreateTokenCopy { .. }
                | EffectAst::CreateTokenCopyFromSource { .. }
                | EffectAst::CreateTokenWithMods { .. },
                EffectAst::Exile { target, .. },
            ) => target_is_generic_token_filter(target),
            _ => false,
        };

        if !mark_end_of_combat_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::CreateTokenCopy {
                exile_at_end_of_combat,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                exile_at_end_of_combat,
                ..
            }
            | EffectAst::CreateTokenWithMods {
                exile_at_end_of_combat,
                ..
            } => {
                *exile_at_end_of_combat = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

fn split_on_comma_or_semicolon_lexed(tokens: &[OwnedLexToken]) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if !matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) {
            continue;
        }
        let current = trim_lexed_commas(&tokens[start..idx]);
        if !current.is_empty() {
            segments.push(current.to_vec());
        }
        start = idx + 1;
    }
    let tail = trim_lexed_commas(&tokens[start..]);
    if !tail.is_empty() {
        segments.push(tail.to_vec());
    }
    segments
}

pub(crate) fn expand_segments_with_comma_action_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let looks_like_sac_discard_chain = (grammar::contains_word(&segment, "sacrifice")
            || grammar::contains_word(&segment, "sacrifices"))
            && (grammar::contains_word(&segment, "discard")
                || grammar::contains_word(&segment, "discards"));
        if !looks_like_sac_discard_chain {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        let mut valid_split = true;

        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while part.first().is_some_and(|token| token.is_word("and")) {
                part.remove(0);
            }
            if part.is_empty() {
                continue;
            }

            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }

            valid_split = false;
            break;
        }

        if valid_split && local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_segments_with_multi_create_clauses_lexed(
    segments: Vec<Vec<OwnedLexToken>>,
) -> Vec<Vec<OwnedLexToken>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let Some((Verb::Create, _)) = find_verb_lexed(&segment) else {
            expanded.push(segment);
            continue;
        };
        let has_token_rules_tail =
            grammar::words_find_phrase(&segment, &["when", "this", "token"]).is_some()
                || grammar::words_find_phrase(&segment, &["whenever", "this", "token"])
                    .is_some()
                || grammar::words_find_phrase(&segment, &["this", "token"]).is_some()
                || grammar::words_find_phrase(&segment, &["that", "token"]).is_some()
                || grammar::words_find_phrase(&segment, &["those", "tokens"]).is_some()
                || grammar::words_find_phrase(&segment, &["it", "has"]).is_some()
                || grammar::words_find_phrase(&segment, &["they", "have"]).is_some();
        if has_token_rules_tail {
            expanded.push(segment);
            continue;
        }
        let segment_words = token_word_refs(&segment);
        let token_mentions = segment_words
            .iter()
            .filter(|word| matches!(**word, "token" | "tokens"))
            .count();
        if token_mentions < 2 {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon_lexed(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<OwnedLexToken>> = Vec::new();
        for raw_part in comma_parts {
            let mut part = trim_lexed_commas(&raw_part).to_vec();
            while part.first().is_some_and(|token| token.is_word("and")) {
                part.remove(0);
            }
            if part.is_empty() {
                continue;
            }
            if let Some(previous) = local_parts.last()
                && is_token_creation_context(previous)
                && starts_with_inline_token_rules_tail(&part)
            {
                if let Some(last) = local_parts.last_mut() {
                    last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                    last.extend(part);
                }
                continue;
            }
            if segment_has_effect_head_lexed(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment_lexed(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }
            if let Some(last) = local_parts.last_mut() {
                last.push(OwnedLexToken::comma(TextSpan::synthetic()));
                last.extend(part);
            } else {
                local_parts.push(part);
            }
        }

        if local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

pub(crate) fn expand_missing_verb_segment_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let (verb, verb_idx) = find_verb_lexed(previous)?;
    match verb {
        Verb::Deal => {
            if parse_value_from_lexed(segment).is_none()
                || !grammar::contains_word(segment, "damage")
            {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Sacrifice => {
            let segment_words = token_word_refs(segment);
            let starts_like_object_phrase = matches!(
                segment_words.first().copied(),
                Some("a" | "an" | "another" | "target")
            ) || parse_number_from_lexed(segment).is_some();
            if !starts_like_object_phrase {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Create => {
            if !starts_like_create_fragment_lexed(segment) {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        _ => None,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarryContext {
    Player(PlayerAst),
    ForEachPlayer,
    ForEachTargetPlayers(ChoiceCount),
    ForEachOpponent,
}

pub(crate) fn player_ast_from_filter_for_carry(filter: &PlayerFilter) -> Option<PlayerAst> {
    match filter {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(inner) => {
            if matches!(inner.as_ref(), PlayerFilter::Opponent) {
                Some(PlayerAst::TargetOpponent)
            } else {
                Some(PlayerAst::Target)
            }
        }
        _ => None,
    }
}

pub(crate) fn player_owner_filter_from_target_for_carry(target: &TargetAst) -> Option<PlayerAst> {
    match target {
        TargetAst::Player(filter, _) => player_ast_from_filter_for_carry(filter),
        TargetAst::Object(filter, _, _) => {
            if !matches!(
                filter.zone,
                Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
            ) {
                return None;
            }
            filter
                .owner
                .as_ref()
                .and_then(player_ast_from_filter_for_carry)
        }
        TargetAst::WithCount(inner, _) => player_owner_filter_from_target_for_carry(inner),
        _ => None,
    }
}

fn player_target_carry_context(target: &TargetAst) -> Option<CarryContext> {
    match target {
        TargetAst::Player(filter, _) => {
            player_ast_from_filter_for_carry(filter).map(CarryContext::Player)
        }
        TargetAst::WithCount(inner, count) => {
            let inner_context = player_target_carry_context(inner.as_ref())?;
            if count.min > 1 && count.max == Some(count.min) {
                Some(CarryContext::ForEachTargetPlayers(*count))
            } else {
                Some(inner_context)
            }
        }
        _ => None,
    }
}

pub(crate) fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    if matches!(effect, EffectAst::ForEachPlayer { .. }) {
        return Some(CarryContext::ForEachPlayer);
    }
    if let EffectAst::ForEachTargetPlayers { count, .. } = effect {
        return Some(CarryContext::ForEachTargetPlayers(*count));
    }
    if matches!(effect, EffectAst::ForEachOpponent { .. }) {
        return Some(CarryContext::ForEachOpponent);
    }
    if let EffectAst::TargetOnly { target } = effect
        && let Some(context) = player_target_carry_context(target)
    {
        return Some(context);
    }
    if let EffectAst::Exile { target, .. } | EffectAst::ExileUntilSourceLeaves { target, .. } =
        effect
        && let Some(player) = player_owner_filter_from_target_for_carry(target)
    {
        return Some(CarryContext::Player(player));
    }
    if let EffectAst::ExileAll { filter, .. } = effect
        && let Some(owner) = filter.owner.as_ref()
        && let Some(player) = player_ast_from_filter_for_carry(owner)
    {
        return Some(CarryContext::Player(player));
    }
    if matches!(effect, EffectAst::ChoosePlayer { .. }) {
        return Some(CarryContext::Player(PlayerAst::That));
    }

    let player = match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::ChooseObjects { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::PayMana { player, .. }
        | EffectAst::PayEnergy { player, .. }
        | EffectAst::AddMana { player, .. }
        | EffectAst::AddManaScaled { player, .. }
        | EffectAst::AddManaAnyColor { player, .. }
        | EffectAst::AddManaAnyOneColor { player, .. }
        | EffectAst::AddManaChosenColor { player, .. }
        | EffectAst::AddManaFromLandCouldProduce { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. } => *player,
        EffectAst::SearchLibrary {
            chooser, player, ..
        } => {
            if !matches!(chooser, PlayerAst::Implicit) {
                *chooser
            } else if !matches!(player, PlayerAst::Implicit) {
                *player
            } else {
                return None;
            }
        }
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(CarryContext::Player(player))
    }
}

pub(crate) fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::ChooseObjects { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player }
        | EffectAst::PutIntoHand { player, .. }
        | EffectAst::PayMana { player, .. }
        | EffectAst::PayEnergy { player, .. }
        | EffectAst::AddMana { player, .. }
        | EffectAst::AddManaScaled { player, .. }
        | EffectAst::AddManaAnyColor { player, .. }
        | EffectAst::AddManaAnyOneColor { player, .. }
        | EffectAst::AddManaChosenColor { player, .. }
        | EffectAst::AddManaFromLandCouldProduce { player, .. }
        | EffectAst::AddManaCommanderIdentity { player, .. }
        | EffectAst::CreateTokenCopy { player, .. }
        | EffectAst::CreateTokenCopyFromSource { player, .. }
        | EffectAst::CreateTokenWithMods { player, .. } => {
            matches!(*player, PlayerAst::Implicit)
        }
        EffectAst::SearchLibrary {
            chooser, player, ..
        } => matches!(*chooser, PlayerAst::Implicit) || matches!(*player, PlayerAst::Implicit),
        _ => false,
    }
}

pub(crate) fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => {
            // When carrying an explicit target player/opponent into an implicit clause,
            // bind to the previously selected target ("that player") instead of creating
            // a fresh explicit target. This preserves shared-target semantics for chains
            // like "Target player mills..., draws..., and loses...".
            let carried_player = match carried_player {
                PlayerAst::Target | PlayerAst::TargetOpponent => PlayerAst::That,
                other => other,
            };
            match effect {
                EffectAst::Draw { player, .. }
                | EffectAst::ChooseObjects { player, .. }
                | EffectAst::DiscardHand { player }
                | EffectAst::Discard { player, .. }
                | EffectAst::GainLife { player, .. }
                | EffectAst::LoseLife { player, .. }
                | EffectAst::Scry { player, .. }
                | EffectAst::Surveil { player, .. }
                | EffectAst::Mill { player, .. }
                | EffectAst::PoisonCounters { player, .. }
                | EffectAst::EnergyCounters { player, .. }
                | EffectAst::RevealTop { player }
                | EffectAst::RevealHand { player }
                | EffectAst::PutIntoHand { player, .. }
                | EffectAst::PayMana { player, .. }
                | EffectAst::PayEnergy { player, .. }
                | EffectAst::AddMana { player, .. }
                | EffectAst::AddManaScaled { player, .. }
                | EffectAst::AddManaAnyColor { player, .. }
                | EffectAst::AddManaAnyOneColor { player, .. }
                | EffectAst::AddManaChosenColor { player, .. }
                | EffectAst::AddManaFromLandCouldProduce { player, .. }
                | EffectAst::AddManaCommanderIdentity { player, .. }
                | EffectAst::CreateTokenCopy { player, .. }
                | EffectAst::CreateTokenCopyFromSource { player, .. }
                | EffectAst::CreateTokenWithMods { player, .. } => {
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                EffectAst::SearchLibrary {
                    chooser, player, ..
                } => {
                    if matches!(*chooser, PlayerAst::Implicit) {
                        *chooser = carried_player;
                    }
                    if matches!(*player, PlayerAst::Implicit) {
                        *player = carried_player;
                    }
                }
                _ => {}
            }
        }
        CarryContext::ForEachPlayer => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachPlayer {
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachTargetPlayers(count) => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachTargetPlayers {
                    count,
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachOpponent => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachOpponent {
                    effects: vec![wrapped],
                };
            }
        }
    }
}

pub(crate) fn clause_words_for_carry_lexed(tokens: &[OwnedLexToken]) -> Vec<&str> {
    let mut clause_words = token_word_refs(tokens);
    while clause_words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        clause_words.remove(0);
    }
    clause_words
}

pub(crate) fn maybe_apply_carried_player_with_clause_lexed(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    let clause_words = clause_words_for_carry_lexed(clause_tokens);
    let should_skip = match carried_context {
        CarryContext::Player(_) => {
            matches!(
                effect,
                EffectAst::Draw {
                    player: PlayerAst::Implicit,
                    ..
                }
            ) && matches!(clause_words.first().copied(), Some("draw"))
        }
        CarryContext::ForEachPlayer
        | CarryContext::ForEachTargetPlayers(_)
        | CarryContext::ForEachOpponent => {
            let is_implicit_vision_effect = matches!(
                effect,
                EffectAst::Draw {
                    player: PlayerAst::Implicit,
                    ..
                } | EffectAst::Scry {
                    player: PlayerAst::Implicit,
                    ..
                } | EffectAst::Surveil {
                    player: PlayerAst::Implicit,
                    ..
                }
            );
            is_implicit_vision_effect
                && matches!(
                    clause_words.first().copied(),
                    Some("draw" | "scry" | "surveil")
                )
        }
    };
    if should_skip {
        return;
    }
    maybe_apply_carried_player(effect, carried_context);
}

pub(crate) fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::Draw {
            player: effect_player,
            ..
        }
        | EffectAst::DiscardHand {
            player: effect_player,
        }
        | EffectAst::Discard {
            player: effect_player,
            ..
        }
        | EffectAst::GainLife {
            player: effect_player,
            ..
        }
        | EffectAst::LoseLife {
            player: effect_player,
            ..
        }
        | EffectAst::Sacrifice {
            player: effect_player,
            ..
        }
        | EffectAst::Scry {
            player: effect_player,
            ..
        }
        | EffectAst::Surveil {
            player: effect_player,
            ..
        }
        | EffectAst::Mill {
            player: effect_player,
            ..
        }
        | EffectAst::PoisonCounters {
            player: effect_player,
            ..
        }
        | EffectAst::EnergyCounters {
            player: effect_player,
            ..
        }
        | EffectAst::RevealTop {
            player: effect_player,
        }
        | EffectAst::RevealHand {
            player: effect_player,
        }
        | EffectAst::PutIntoHand {
            player: effect_player,
            ..
        }
        | EffectAst::PayMana {
            player: effect_player,
            ..
        }
        | EffectAst::PayEnergy {
            player: effect_player,
            ..
        }
        | EffectAst::AddMana {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaScaled {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaAnyColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaAnyOneColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaChosenColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaFromLandCouldProduce {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaCommanderIdentity {
            player: effect_player,
            ..
        }
        | EffectAst::SearchLibrary {
            player: effect_player,
            ..
        }
        | EffectAst::ShuffleGraveyardIntoLibrary {
            player: effect_player,
        }
        | EffectAst::ShuffleLibrary {
            player: effect_player,
        }
        | EffectAst::AdditionalLandPlays {
            player: effect_player,
            ..
        }
        | EffectAst::GrantBySpec {
            player: effect_player,
            ..
        }
        | EffectAst::CreateTokenCopy {
            player: effect_player,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            player: effect_player,
            ..
        }
        | EffectAst::CreateTokenWithMods {
            player: effect_player,
            ..
        }
        | EffectAst::CopySpell {
            player: effect_player,
            ..
        }
        | EffectAst::SkipTurn {
            player: effect_player,
        }
        | EffectAst::SkipCombatPhases {
            player: effect_player,
        }
        | EffectAst::SkipNextCombatPhaseThisTurn {
            player: effect_player,
        }
        | EffectAst::SkipDrawStep {
            player: effect_player,
        }
        | EffectAst::RetargetStackObject {
            chooser: effect_player,
            ..
        } => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        _ => for_each_nested_effects_mut(effect, true, |nested| {
            for nested_effect in nested {
                bind_implicit_player_context(nested_effect, player);
            }
        }),
    }
}

fn parse_leading_player_may_words(words: &[&str]) -> Option<PlayerAst> {
    let mut words = words.to_vec();
    while words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        words.remove(0);
    }
    if words.len() < 2 {
        return None;
    }

    if word_slice_starts_with(&words, &["you", "may"]) {
        return Some(PlayerAst::You);
    }
    if word_slice_starts_with(&words, &["target", "opponent", "may"])
        || word_slice_starts_with(&words, &["target", "opponents", "may"])
    {
        return Some(PlayerAst::TargetOpponent);
    }
    if word_slice_starts_with(&words, &["target", "player", "may"])
        || word_slice_starts_with(&words, &["target", "players", "may"])
    {
        return Some(PlayerAst::Target);
    }
    if word_slice_starts_with(&words, &["that", "player", "may"])
        || word_slice_starts_with(&words, &["that", "players", "may"])
    {
        return Some(PlayerAst::That);
    }
    if word_slice_starts_with(&words, &["they", "may"]) {
        return Some(PlayerAst::That);
    }
    if words.len() >= 7
        && words[0] == "that"
        && words[1] == "player"
        && words[2] == "or"
        && words[3] == "that"
        && matches!(
            words[4],
            "creatures" | "permanents" | "planeswalkers" | "sources" | "spells"
        )
        && words[5] == "controller"
        && words[6] == "may"
    {
        return Some(PlayerAst::ThatPlayerOrTargetController);
    }
    if words.len() >= 4
        && words[0] == "that"
        && matches!(words[1], "creatures" | "permanents" | "sources" | "spells")
        && words[2] == "controller"
        && words[3] == "may"
    {
        return Some(PlayerAst::ItsController);
    }
    if words.len() >= 4
        && words[0] == "that"
        && matches!(words[1], "creatures" | "permanents" | "sources" | "spells")
        && words[2] == "owner"
        && words[3] == "may"
    {
        return Some(PlayerAst::ItsOwner);
    }
    if word_slice_starts_with(&words, &["the", "player", "may"])
        || word_slice_starts_with(&words, &["the", "players", "may"])
    {
        return Some(PlayerAst::That);
    }
    if word_slice_starts_with(&words, &["defending", "player", "may"]) {
        return Some(PlayerAst::Defending);
    }
    if word_slice_starts_with(&words, &["attacking", "player", "may"])
        || word_slice_starts_with(&words, &["the", "attacking", "player", "may"])
    {
        return Some(PlayerAst::Attacking);
    }
    if word_slice_starts_with(&words, &["its", "controller", "may"])
        || word_slice_starts_with(&words, &["their", "controller", "may"])
    {
        return Some(PlayerAst::ItsController);
    }
    if word_slice_starts_with(&words, &["its", "owner", "may"])
        || word_slice_starts_with(&words, &["their", "owner", "may"])
    {
        return Some(PlayerAst::ItsOwner);
    }
    if word_slice_starts_with(&words, &["opponent", "may"])
        || word_slice_starts_with(&words, &["opponents", "may"])
        || word_slice_starts_with(&words, &["an", "opponent", "may"])
    {
        return Some(PlayerAst::Opponent);
    }

    None
}

pub(crate) fn parse_leading_player_may_lexed(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.word_refs();
    parse_leading_player_may_words(&words)
}

pub(crate) fn find_verb(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    find_verb_lexed(tokens)
}

pub(crate) fn parse_effect_chain(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_lexed(tokens)
}

pub(crate) fn parse_or_action_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_or_action_clause_lexed(tokens)
}

pub(crate) fn parse_effect_chain_with_sentence_primitives(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_with_sentence_primitives_lexed(tokens)
}

pub(crate) fn parse_effect_chain_inner(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_inner_lexed(tokens)
}

pub(crate) fn parse_effect_clause_with_trailing_if(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    parse_effect_clause_with_trailing_if_lexed(tokens)
}

pub(crate) fn collapse_token_copy_next_end_step_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_next_end_step_exile_followup_lexed(effects, tokens);
}

pub(crate) fn collapse_token_copy_next_end_step_sacrifice_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_next_end_step_sacrifice_followup_lexed(effects, tokens);
}

pub(crate) fn collapse_token_copy_end_of_combat_exile_followup(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
) {
    collapse_token_copy_end_of_combat_exile_followup_lexed(effects, tokens);
}

pub(crate) fn maybe_apply_carried_player_with_clause(
    effect: &mut EffectAst,
    carried_context: CarryContext,
    clause_tokens: &[OwnedLexToken],
) {
    maybe_apply_carried_player_with_clause_lexed(effect, carried_context, clause_tokens);
}

pub(crate) fn parse_leading_player_may(tokens: &[OwnedLexToken]) -> Option<PlayerAst> {
    let token_words = tokens
        .iter()
        .filter_map(OwnedLexToken::as_word)
        .collect::<Vec<_>>();
    parse_leading_player_may_words(&token_words)
}

pub(crate) fn remove_first_word(tokens: &[OwnedLexToken], word: &str) -> Vec<OwnedLexToken> {
    let mut removed = false;
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        if !removed && token.is_word(word) {
            removed = true;
            continue;
        }
        out.push(token.clone());
    }
    out
}

pub(crate) fn remove_through_first_word(
    tokens: &[OwnedLexToken],
    word: &str,
) -> Vec<OwnedLexToken> {
    let mut seen = false;
    let mut out = Vec::new();
    for token in tokens {
        if !seen {
            if token.is_word(word) {
                seen = true;
            }
            continue;
        }
        out.push(token.clone());
    }
    out
}

fn remove_first_word_lexed(tokens: &[OwnedLexToken], word: &str) -> Vec<OwnedLexToken> {
    let mut removed = false;
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        if !removed && token.is_word(word) {
            removed = true;
            continue;
        }
        out.push(token.clone());
    }
    out
}

fn remove_through_first_word_lexed(tokens: &[OwnedLexToken], word: &str) -> Vec<OwnedLexToken> {
    let mut seen = false;
    let mut out = Vec::new();
    for token in tokens {
        if !seen {
            if token.is_word(word) {
                seen = true;
            }
            continue;
        }
        out.push(token.clone());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verb {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Untap,
    Scry,
    Discard,
    Transform,
    Convert,
    Flip,
    Regenerate,
    Mill,
    Get,
    Reveal,
    Look,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Attach,
    Remove,
    Return,
    Exchange,
    Become,
    Switch,
    Skip,
    Surveil,
    Shuffle,
    Reorder,
    Pay,
    Detain,
    Goad,
}

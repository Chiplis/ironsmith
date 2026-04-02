use super::super::activation_and_restrictions::{
    find_negation_span, parse_cant_restriction_clause, parse_cant_restrictions,
    parse_choose_color_phrase_words, parse_choose_creature_type_phrase_words,
    parse_choose_player_phrase_words, parse_single_word_keyword_action,
    parse_target_player_choose_objects_clause, parse_you_choose_objects_clause,
    parse_you_choose_player_clause,
};
use super::super::keyword_static::{
    keyword_action_to_static_ability, parse_ability_line, parse_pt_modifier,
    parse_pt_modifier_values,
};
use super::super::lexer::OwnedLexToken;
use super::super::native_tokens::{LowercaseWordView, lowercase_word_tokens};
use super::super::object_filters::parse_object_filter;
use super::super::util::{
    contains_until_end_of_turn, parse_card_type, parse_color, parse_subject, parse_target_phrase,
    parse_value, parser_trace, parser_trace_stack, span_from_tokens, starts_with_until_end_of_turn,
    token_index_for_word_index, trim_commas,
};
use super::chain_carry::{parse_leading_player_may, remove_first_word, remove_through_first_word};
use super::clause_pattern_helpers::extract_subject_player;
use super::clause_primitives::run_clause_primitives;
use super::for_each_helpers::{
    has_demonstrative_object_reference, is_mana_replacement_clause_words,
    is_mana_trigger_additional_clause_words, is_target_player_dealt_damage_by_this_turn_subject,
    parse_for_each_object_subject, parse_get_for_each_count_value,
    parse_get_modifier_values_with_tail, parse_has_base_power_clause,
    parse_has_base_power_toughness_clause,
};
use super::search_library::parse_restriction_duration;
use super::sentence_primitives::try_build_unless;
use super::verb_dispatch::parse_effect_with_verb;
use super::zone_counter_helpers::{parse_half_starting_life_total_value, parse_put_counters};
use super::zone_handlers::collapse_leading_signed_pt_modifier_tokens;
use super::{
    Verb, bind_implicit_player_context, find_verb, parse_effect_chain_with_sentence_primitives,
    parse_simple_gain_ability_clause, parse_simple_lose_ability_clause, parse_subtype_word,
};
use crate::TagKey;
use crate::cards::builders::scan_helpers::{
    find_index as find_token_index, find_str_by as find_word_index_by,
    find_str_index as find_word_index, find_window_index as find_word_sequence_index,
    slice_contains_any as word_slice_contains_any, slice_contains_str as word_slice_contains,
    slice_ends_with as word_slice_ends_with, slice_starts_with as word_slice_starts_with,
};
use crate::cards::builders::{CardTextError, EffectAst, GrantedAbilityAst, IT_TAG, TargetAst};
use crate::effect::{Until, Value};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::{CardType, Subtype};

fn render_lower_words(tokens: &[OwnedLexToken]) -> String {
    let word_view = LowercaseWordView::new(tokens);
    word_view.to_word_refs().join(" ")
}

fn contains_card_type(card_types: &[CardType], target: CardType) -> bool {
    for card_type in card_types {
        if *card_type == target {
            return true;
        }
    }
    false
}

fn push_unique_card_type(card_types: &mut Vec<CardType>, card_type: CardType) {
    if !contains_card_type(card_types, card_type) {
        card_types.push(card_type);
    }
}

fn contains_subtype(subtypes: &[Subtype], target: Subtype) -> bool {
    for subtype in subtypes {
        if *subtype == target {
            return true;
        }
    }
    false
}

fn push_unique_subtype(subtypes: &mut Vec<Subtype>, subtype: Subtype) {
    if !contains_subtype(subtypes, subtype) {
        subtypes.push(subtype);
    }
}

fn trim_plural_s(word: &str) -> Option<&str> {
    let bytes = word.as_bytes();
    let last = bytes.last().copied()?;
    if last != b's' && last != b'S' {
        return None;
    }
    word.get(..word.len().saturating_sub(1))
}

fn parse_subtype_word_or_plural(word: &str) -> Option<Subtype> {
    parse_subtype_word(word).or_else(|| trim_plural_s(word).and_then(parse_subtype_word))
}

fn has_counter_state_pronoun(subject_words: &[&str]) -> bool {
    for start in 0..subject_words.len().saturating_sub(2) {
        if matches!(subject_words[start], "counter" | "counters")
            && subject_words[start + 1] == "on"
            && matches!(subject_words[start + 2], "it" | "them")
        {
            return true;
        }
    }
    false
}

fn subject_references_base_power_toughness(subject_words: &[&str]) -> bool {
    find_word_sequence_index(subject_words, &["base", "power", "and", "toughness"]).is_some()
}

fn strip_base_power_toughness_subject_tokens<'a>(
    subject_tokens: &'a [OwnedLexToken],
    subject_words: &[&str],
) -> &'a [OwnedLexToken] {
    let Some(base_word_idx) =
        find_word_sequence_index(subject_words, &["base", "power", "and", "toughness"])
    else {
        return subject_tokens;
    };
    let Some(base_token_idx) = token_index_for_word_index(subject_tokens, base_word_idx) else {
        return subject_tokens;
    };

    let mut stripped = &subject_tokens[..base_token_idx];
    while stripped.last().is_some_and(|token| token.is_word("s")) {
        stripped = &stripped[..stripped.len().saturating_sub(1)];
    }
    stripped
}

fn parse_become_base_pt_tail<'a>(
    become_words: &'a [&'a str],
) -> Result<Option<(&'a [&'a str], i32, i32)>, CardTextError> {
    let Some(with_idx) = find_word_index(become_words, "with") else {
        return Ok(None);
    };
    let tail = &become_words[with_idx + 1..];
    if tail.len() != 5 || tail[..4] != ["base", "power", "and", "toughness"] {
        return Ok(None);
    }
    let (power, toughness) = parse_pt_modifier(tail[4])?;
    Ok(Some((&become_words[..with_idx], power, toughness)))
}

fn parse_become_creature_descriptor_words(
    descriptor_words: &[&str],
) -> Option<(Vec<CardType>, Vec<Subtype>, Option<crate::color::ColorSet>)> {
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    let mut colors = crate::color::ColorSet::new();
    let mut saw_subtype = false;

    for word in descriptor_words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique_card_type(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_word_or_plural(word) {
            push_unique_subtype(&mut subtypes, subtype);
            saw_subtype = true;
            continue;
        }
        return None;
    }

    if saw_subtype && !contains_card_type(&card_types, CardType::Creature) {
        card_types.insert(0, CardType::Creature);
    }
    if card_types.is_empty() && !saw_subtype {
        return None;
    }

    Some((
        card_types,
        subtypes,
        if colors.is_empty() {
            None
        } else {
            Some(colors)
        },
    ))
}

pub(crate) fn parse_effect_clause(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    let stripped_instead = super::strip_leading_instead_prefix(tokens);
    let tokens = stripped_instead.as_deref().unwrap_or(tokens);

    if let Some(player) = parse_leading_player_may(tokens) {
        let mut stripped = remove_through_first_word(tokens, "may");
        if stripped
            .first()
            .is_some_and(|token| token.is_word("have") || token.is_word("has"))
        {
            stripped.remove(0);
        }
        let mut effects = parse_effect_chain_with_sentence_primitives(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        return Ok(EffectAst::MayByPlayer { player, effects });
    }

    if tokens.first().is_some_and(|token| token.is_word("may")) {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_with_sentence_primitives(&stripped)?;
        return Ok(EffectAst::May { effects });
    }

    let clause_word_view = LowercaseWordView::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if clause_words.as_slice() == ["the", "ring", "tempts", "you"] {
        return Ok(EffectAst::RingTemptsYou {
            player: crate::cards::builders::PlayerAst::You,
        });
    }
    if clause_words.as_slice() == ["you", "take", "the", "initiative"] {
        return Ok(EffectAst::TakeInitiative {
            player: crate::cards::builders::PlayerAst::You,
        });
    }
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}') [rule=mana-replacement]",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}') [rule=mana-trigger-additional]",
            clause_words.join(" ")
        )));
    }

    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(effect);
    }

    if let Some(unless_idx) = find_token_index(tokens, |token| token.is_word("unless")) {
        let main_tokens = trim_commas(&tokens[..unless_idx]);
        if !main_tokens.is_empty()
            && let Ok(main_effect) = parse_effect_clause(&main_tokens)
            && let Some(unless_effect) = try_build_unless(vec![main_effect], tokens, unless_idx)?
        {
            return Ok(unless_effect);
        }
    }

    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    let choice_words = if clause_words.first().copied() == Some("you") {
        &clause_words[1..]
    } else {
        &clause_words[..]
    };

    if let Some((consumed, excluded_color)) = parse_choose_color_phrase_words(choice_words)?
        && consumed == choice_words.len()
        && excluded_color.is_none()
    {
        return Ok(EffectAst::ChooseColor {
            player: crate::cards::builders::PlayerAst::Implicit,
        });
    }

    if matches!(choice_words, ["choose", "odd", "or", "even"]) {
        return Ok(EffectAst::ChooseNamedOption {
            player: crate::cards::builders::PlayerAst::Implicit,
            options: vec!["odd".to_string(), "even".to_string()],
        });
    }

    if let Some((consumed, excluded_subtypes)) =
        parse_choose_creature_type_phrase_words(choice_words)?
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::ChooseCreatureType {
            player: crate::cards::builders::PlayerAst::Implicit,
            excluded_subtypes,
        });
    }

    if let Some(consumed) = parse_choose_player_phrase_words(choice_words)
        && consumed == choice_words.len()
    {
        return Ok(EffectAst::ChoosePlayer {
            chooser: crate::cards::builders::PlayerAst::Implicit,
            filter: PlayerFilter::Any,
            tag: TagKey::from(IT_TAG),
            random: false,
            exclude_previous_choices: 0,
        });
    }

    if matches!(clause_words.first().copied(), Some("choose" | "chooses"))
        && word_slice_contains(&clause_words, "target")
        && (word_slice_contains(&clause_words, "player")
            || word_slice_contains(&clause_words, "players"))
        && let Ok(target) = parse_target_phrase(&tokens[1..])
    {
        let is_player_target = match &target {
            TargetAst::Player(_, _) => true,
            TargetAst::WithCount(inner, _) => matches!(inner.as_ref(), TargetAst::Player(_, _)),
            _ => false,
        };
        if is_player_target {
            return Ok(EffectAst::TargetOnly { target });
        }
    }

    if let Some((chooser, choose_filter, random, exclude_previous_choices)) =
        parse_you_choose_player_clause(tokens)?
    {
        return Ok(EffectAst::ChoosePlayer {
            chooser,
            filter: choose_filter,
            tag: TagKey::from(IT_TAG),
            random,
            exclude_previous_choices,
        });
    }

    if let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if let Some((chooser, choose_filter, choose_count)) = parse_you_choose_objects_clause(tokens)? {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if find_word_sequence_index(&clause_words, &["assigns", "no", "combat", "damage"]).is_some() {
        let assigns_idx = find_token_index(tokens, |token| {
            token.is_word("assigns") || token.is_word("assign")
        })
        .unwrap_or(0);
        let subject_tokens = trim_commas(&tokens[..assigns_idx]);
        let tail_tokens = trim_commas(&tokens[assigns_idx + 1..]);
        let tail_word_view = LowercaseWordView::new(&tail_tokens);
        let tail_words = tail_word_view.to_word_refs();
        if !word_slice_starts_with(&tail_words, &["no", "combat", "damage"]) {
            return Err(CardTextError::ParseError(format!(
                "unsupported assigns-no-combat-damage clause (clause: '{}') [rule=assigns-no-combat-damage]",
                clause_words.join(" ")
            )));
        }
        let mut idx = 3usize;
        if tail_words.get(idx) == Some(&"this") && tail_words.get(idx + 1) == Some(&"turn") {
            idx += 2;
        } else if tail_words.get(idx) == Some(&"this") && tail_words.get(idx + 1) == Some(&"combat")
        {
            idx += 2;
        }
        if idx != tail_words.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported assigns-no-combat-damage clause tail (clause: '{}') [rule=assigns-no-combat-damage-tail]",
                clause_words.join(" ")
            )));
        }

        let subject_word_view = LowercaseWordView::new(&subject_tokens);
        let subject_words = subject_word_view.to_word_refs();
        let source = if subject_words.is_empty()
            || matches!(
                subject_words.as_slice(),
                ["it"] | ["this"] | ["this", "creature"]
            ) {
            TargetAst::Source(None)
        } else {
            parse_target_phrase(&subject_tokens)?
        };

        return Ok(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        });
    }

    if tokens.first().is_some_and(|token| token.is_word("target")) && find_verb(tokens).is_none() {
        let looks_like_restriction_clause = find_negation_span(tokens).is_some()
            || word_slice_contains_any(
                &clause_words,
                &[
                    "blocked", "except", "unless", "attack", "attacks", "block", "blocks",
                ],
            );
        if looks_like_restriction_clause {
            return Err(CardTextError::ParseError(format!(
                "unsupported target-only restriction clause (clause: '{}') [rule=target-only-restriction]",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(tokens)?;
        return Ok(EffectAst::TargetOnly { target });
    }

    if let Some(effect) = parse_next_turn_cant_clause(tokens)? {
        return Ok(effect);
    }

    if let Some((duration, clause_tokens)) = parse_restriction_duration(tokens)?
        && find_negation_span(&clause_tokens).is_some()
        && let Some(restrictions) = parse_cant_restrictions(&clause_tokens)?
        && let [parsed] = restrictions.as_slice()
        && parsed.target.is_none()
    {
        return Ok(EffectAst::Cant {
            restriction: parsed.restriction.clone(),
            duration,
            condition: None,
        });
    }

    let (verb, verb_idx) = find_verb(tokens).ok_or_else(|| {
        let clause = render_lower_words(tokens);
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "convert",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "look",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "remove",
            "return",
            "exchange",
            "become",
            "switch",
            "skip",
            "surveil",
            "shuffle",
            "reorder",
            "pay",
            "detain",
            "goad",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);

    if matches!(verb, Verb::Counter)
        && verb_idx > 0
        && tokens.iter().any(|token| token.is_word("on"))
        && let Ok(effect) = parse_put_counters(tokens)
    {
        parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
        return Ok(effect);
    }

    if matches!(verb, Verb::Get) {
        let subject_tokens = &tokens[..verb_idx];
        if !subject_tokens.is_empty() {
            let subject_word_view = LowercaseWordView::new(subject_tokens);
            let subject_words = subject_word_view.to_word_refs();
            let collapsed_modifier_tail =
                collapse_leading_signed_pt_modifier_tokens(&tokens[verb_idx + 1..]);
            let modifier_tail = collapsed_modifier_tail
                .as_deref()
                .unwrap_or(&tokens[verb_idx + 1..]);
            if let Some(mod_token) = modifier_tail.first().and_then(OwnedLexToken::as_word)
                && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
            {
                if let Some(count) = parse_get_for_each_count_value(modifier_tail)? {
                    let modifier_word_view = LowercaseWordView::new(modifier_tail);
                    let modifier_words = modifier_word_view.to_word_refs();
                    let duration = if starts_with_until_end_of_turn(&modifier_words)
                        || contains_until_end_of_turn(&modifier_words)
                    {
                        Until::EndOfTurn
                    } else {
                        Until::EndOfTurn
                    };
                    let target = parse_target_phrase(subject_tokens)?;
                    let power_per = match power {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each power modifier (clause: '{}')",
                                render_lower_words(tokens)
                            )));
                        }
                    };
                    let toughness_per = match toughness {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each toughness modifier (clause: '{}')",
                                render_lower_words(tokens)
                            )));
                        }
                    };
                    return Ok(EffectAst::PumpForEach {
                        power_per,
                        toughness_per,
                        target,
                        count,
                        duration,
                    });
                }

                let (power, toughness, duration, condition) =
                    parse_get_modifier_values_with_tail(modifier_tail, power, toughness)?;

                let mut normalized_subject_words: Vec<&str> = subject_words
                    .iter()
                    .copied()
                    .filter(|word| *word != "each")
                    .collect();
                if normalized_subject_words.first().copied() == Some("of") {
                    normalized_subject_words.remove(0);
                }
                if normalized_subject_words.as_slice() == ["it"]
                    || normalized_subject_words.as_slice() == ["they"]
                    || normalized_subject_words.as_slice() == ["them"]
                {
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target: TargetAst::Tagged(
                            TagKey::from(IT_TAG),
                            span_from_tokens(subject_tokens),
                        ),
                        duration,
                        condition,
                    });
                }

                let is_demonstrative_subject = normalized_subject_words
                    .first()
                    .is_some_and(|word| *word == "that" || *word == "those");
                if is_demonstrative_subject {
                    let target = parse_target_phrase(subject_tokens)?;
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target,
                        duration,
                        condition,
                    });
                }

                if word_slice_contains(&subject_words, "target") {
                    let target_tokens = if subject_tokens
                        .first()
                        .is_some_and(|token| token.is_word("have") || token.is_word("has"))
                    {
                        &subject_tokens[1..]
                    } else {
                        subject_tokens
                    };
                    let target = parse_target_phrase(target_tokens)?;
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target,
                        duration,
                        condition,
                    });
                }

                let has_counter_state_pronoun = has_counter_state_pronoun(&subject_words);
                let has_disallowed_pronoun_reference = (word_slice_contains(&subject_words, "it")
                    || word_slice_contains(&subject_words, "them"))
                    && !has_counter_state_pronoun;
                if !word_slice_contains(&subject_words, "this")
                    && !has_disallowed_pronoun_reference
                    && !has_demonstrative_object_reference(&subject_words)
                    && let Ok(filter) = parse_object_filter(subject_tokens, false)
                    && filter != ObjectFilter::default()
                {
                    return Ok(EffectAst::PumpAll {
                        filter,
                        power: power.clone(),
                        toughness: toughness.clone(),
                        duration,
                    });
                }
            }
        }
    }

    let subject_tokens = &tokens[..verb_idx];
    let subject_word_view = LowercaseWordView::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}') [rule=combat-history-player-subject]",
            render_lower_words(tokens)
        )));
    }
    if matches!(verb, Verb::Gain) && !subject_tokens.is_empty() {
        let rest_word_view = LowercaseWordView::new(&tokens[verb_idx + 1..]);
        let rest_words = rest_word_view.to_word_refs();
        let has_protection = word_slice_contains(&rest_words, "protection");
        let has_choice = word_slice_contains(&rest_words, "choice");
        let has_color = word_slice_contains(&rest_words, "color");
        let has_colorless = word_slice_contains(&rest_words, "colorless");
        if has_protection && has_choice && (has_color || has_colorless) {
            let target = parse_target_phrase(subject_tokens)?;
            return Ok(EffectAst::GrantProtectionChoice {
                target,
                allow_colorless: has_colorless,
            });
        }
    }
    if matches!(verb, Verb::Gain)
        && let Some(effect) = parse_simple_gain_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    if matches!(verb, Verb::Gain) {
        let rest_word_view = LowercaseWordView::new(&tokens[verb_idx + 1..]);
        let rest_words = rest_word_view.to_word_refs();
        let duration_phrase = super::gain_ability::parse_simple_ability_duration(&rest_words);
        let duration = duration_phrase
            .as_ref()
            .map(|(_, _, duration)| duration.clone())
            .unwrap_or(Until::Forever);
        let ability_end_word_idx = duration_phrase
            .as_ref()
            .map(|(start, _, _)| verb_idx + 1 + *start)
            .unwrap_or(clause_words.len());
        let ability_end_token_idx =
            token_index_for_word_index(tokens, ability_end_word_idx).unwrap_or(tokens.len());
        let ability_tokens = trim_commas(&tokens[verb_idx + 1..ability_end_token_idx]);
        let trailing_tokens = trim_commas(&tokens[ability_end_token_idx..]);
        let parsed_actions = parse_ability_line(&ability_tokens).or_else(|| {
            let ability_word_view = LowercaseWordView::new(&ability_tokens);
            let ability_words = ability_word_view.to_word_refs();
            if ability_words.len() == 1 {
                parse_single_word_keyword_action(ability_words[0]).map(|action| vec![action])
            } else {
                None
            }
        });
        if !ability_tokens.is_empty()
            && trailing_tokens.is_empty()
            && let Some(actions) = parsed_actions
            && !actions.is_empty()
            && subject_words.first().copied() == Some("target")
        {
            let target = parse_target_phrase(subject_tokens)?;
            let abilities = actions.into_iter().map(GrantedAbilityAst::from).collect();
            return Ok(EffectAst::GrantAbilitiesToTarget {
                target,
                abilities,
                duration,
            });
        }
    }
    if matches!(verb, Verb::Lose)
        && let Some(effect) = parse_simple_lose_ability_clause(tokens)?
    {
        return Ok(effect);
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let rest = &tokens[verb_idx + 1..];
    let mut effect = if matches!(verb, Verb::Become) {
        parse_become_clause(subject_tokens, rest)?
    } else {
        let subject = parse_subject(subject_tokens);
        parse_effect_with_verb(verb, Some(subject), rest)?
    };
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        };
    }
    Ok(effect)
}

fn parse_next_turn_cant_clause(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let lowered_tokens = lowercase_word_tokens(tokens);
    let lowered_word_view = LowercaseWordView::new(&lowered_tokens);
    let lowered_words = lowered_word_view.to_word_refs();
    for suffix in [
        ["during", "that", "players", "next", "turn"].as_slice(),
        ["during", "that", "player's", "next", "turn"].as_slice(),
        ["during", "that", "player", "s", "next", "turn"].as_slice(),
    ] {
        if !word_slice_ends_with(&lowered_words, suffix) {
            continue;
        }

        let prefix_word_len = lowered_words.len().saturating_sub(suffix.len());
        let prefix_end = lowered_word_view
            .token_index_for_word_index(prefix_word_len)
            .unwrap_or(lowered_tokens.len());
        let prefix_tokens = &lowered_tokens[..prefix_end];
        let Some(parsed) = parse_cant_restriction_clause(prefix_tokens)? else {
            continue;
        };

        let nested_restriction = match parsed.restriction {
            crate::effect::Restriction::CastSpellsMatching(player, spell_filter) => {
                let nested = crate::effect::Restriction::cast_spells_matching(
                    PlayerFilter::Active,
                    spell_filter,
                );
                match player {
                    PlayerFilter::Opponent => {
                        return Ok(Some(EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: Until::EndOfTurn,
                                    condition: None,
                                }],
                            }],
                        }));
                    }
                    PlayerFilter::IteratedPlayer => nested,
                    _ => continue,
                }
            }
            crate::effect::Restriction::CastMoreThanOneSpellEachTurn(player, spell_filter) => {
                let nested = crate::effect::Restriction::CastMoreThanOneSpellEachTurn(
                    PlayerFilter::Active,
                    spell_filter,
                );
                match player {
                    PlayerFilter::Opponent => {
                        return Ok(Some(EffectAst::ForEachOpponent {
                            effects: vec![EffectAst::DelayedUntilNextUpkeep {
                                player: crate::cards::builders::PlayerAst::That,
                                effects: vec![EffectAst::Cant {
                                    restriction: nested,
                                    duration: Until::EndOfTurn,
                                    condition: None,
                                }],
                            }],
                        }));
                    }
                    PlayerFilter::IteratedPlayer => nested,
                    _ => continue,
                }
            }
            _ => continue,
        };

        return Ok(Some(EffectAst::DelayedUntilNextUpkeep {
            player: crate::cards::builders::PlayerAst::That,
            effects: vec![EffectAst::Cant {
                restriction: nested_restriction,
                duration: Until::EndOfTurn,
                condition: None,
            }],
        }));
    }

    Ok(None)
}

pub(crate) fn parse_effect_clause_lexed(
    tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let lowered = lowercase_word_tokens(tokens);
    parse_effect_clause(&lowered)
}

pub(crate) fn parse_become_clause(
    subject_tokens: &[OwnedLexToken],
    rest_tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let (duration, subject_tokens_vec, become_tokens) =
        if let Some((duration, remainder)) = parse_restriction_duration(subject_tokens)? {
            (duration, remainder, trim_commas(rest_tokens).to_vec())
        } else if let Some((duration, remainder)) = parse_restriction_duration(rest_tokens)? {
            (duration, trim_commas(subject_tokens).to_vec(), remainder)
        } else {
            (
                Until::Forever,
                trim_commas(subject_tokens).to_vec(),
                trim_commas(rest_tokens).to_vec(),
            )
        };
    let subject_tokens = subject_tokens_vec.as_slice();
    let subject_word_view = LowercaseWordView::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    let subject_targets_base_pt = subject_references_base_power_toughness(&subject_words);
    let target_subject_tokens =
        strip_base_power_toughness_subject_tokens(subject_tokens, &subject_words);
    let target_subject_word_view = LowercaseWordView::new(target_subject_tokens);
    let target_subject_words = target_subject_word_view.to_word_refs();
    let subject = parse_subject(subject_tokens);
    let become_body_tokens = if become_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| word == "the" || word == "a" || word == "an")
    {
        &become_tokens[1..]
    } else {
        &become_tokens[..]
    };
    let become_word_view = LowercaseWordView::new(become_body_tokens);
    let become_words_vec = become_word_view.to_word_refs();
    let become_words = &become_words_vec[..];

    if let Some(player) = extract_subject_player(Some(subject)) {
        if become_words == ["monarch"] {
            return Ok(EffectAst::BecomeMonarch { player });
        }
        if word_slice_contains(&subject_words, "life")
            && word_slice_contains(&subject_words, "total")
        {
            let amount = parse_value(&become_tokens)
                .map(|(value, _)| value)
                .or_else(|| parse_half_starting_life_total_value(&become_tokens, player))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing life total amount (clause: '{}')",
                        render_lower_words(rest_tokens)
                    ))
                })?;
            return Ok(EffectAst::SetLifeTotal { amount, player });
        }
    }

    let target = if target_subject_words.is_empty()
        || target_subject_words == ["it"]
        || target_subject_words == ["they"]
        || target_subject_words == ["them"]
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
    } else if target_subject_words == ["this"]
        || target_subject_words == ["this", "permanent"]
        || target_subject_words == ["this", "creature"]
        || target_subject_words == ["this", "land"]
    {
        TargetAst::Source(span_from_tokens(subject_tokens))
    } else {
        parse_target_phrase(target_subject_tokens)?
    };

    if become_words == ["basic", "land", "type", "of", "your", "choice"] {
        return Ok(EffectAst::BecomeBasicLandTypeChoice { target, duration });
    }

    if let [word] = become_words
        && let Some(subtype) = parse_subtype_word(word)
        && matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    {
        return Ok(EffectAst::BecomeBasicLandType {
            target,
            subtype,
            duration,
        });
    }

    if become_words == ["color", "of", "your", "choice"]
        || become_words == ["color", "or", "colors", "of", "your", "choice"]
        || become_words == ["colors", "of", "your", "choice"]
    {
        return Ok(EffectAst::BecomeColorChoice { target, duration });
    }

    if become_words == ["creature", "type", "of", "your", "choice"] {
        return Ok(EffectAst::BecomeCreatureTypeChoice {
            target,
            duration,
            excluded_subtypes: Vec::new(),
        });
    }

    if word_slice_starts_with(become_words, &["copy", "of"]) {
        let Some(source_start) = token_index_for_word_index(become_body_tokens, 2) else {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                render_lower_words(rest_tokens)
            )));
        };
        let source_tokens = trim_commas(&become_body_tokens[source_start..]);
        if source_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                render_lower_words(rest_tokens)
            )));
        }
        let source = parse_target_phrase(&source_tokens)?;
        return Ok(EffectAst::BecomeCopy {
            target,
            source,
            duration,
        });
    }

    if become_words == ["colorless"] {
        return Ok(EffectAst::MakeColorless { target, duration });
    }

    if word_slice_starts_with(become_words, &["equal", "to"]) {
        let rhs = &become_words[2..];
        if rhs == ["this", "power", "and", "toughness"]
            || rhs == ["thiss", "power", "and", "toughness"]
            || rhs == ["source", "power", "and", "toughness"]
        {
            return Ok(EffectAst::SetBasePowerToughness {
                power: Value::PowerOf(Box::new(ChooseSpec::Source)),
                toughness: Value::ToughnessOf(Box::new(ChooseSpec::Source)),
                target,
                duration,
            });
        }
    }

    if let Some(pt_word) = become_words.first().copied()
        && let Ok((power, toughness)) = parse_pt_modifier(pt_word)
    {
        if subject_targets_base_pt || become_words.len() == 1 {
            return Ok(EffectAst::SetBasePowerToughness {
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                target,
                duration,
            });
        }
        if let Some(creature_idx) = find_word_index_by(become_words, |word| {
            matches!(word, "creature" | "creatures")
        }) {
            let mut card_types = vec![CardType::Creature];
            let mut subtypes = Vec::new();
            let mut colors = crate::color::ColorSet::new();
            let mut all_prefix_words_supported = true;
            for word in &become_words[1..creature_idx] {
                if let Some(color) = parse_color(word) {
                    colors = colors.union(color);
                    continue;
                }
                if let Some(card_type) = parse_card_type(word) {
                    if card_type != CardType::Creature {
                        push_unique_card_type(&mut card_types, card_type);
                    }
                    continue;
                }
                if let Some(subtype) = parse_subtype_word_or_plural(word) {
                    push_unique_subtype(&mut subtypes, subtype);
                    continue;
                }
                all_prefix_words_supported = false;
                break;
            }

            let mut abilities = Vec::new();
            let suffix_tokens = if let Some(creature_token_idx) =
                token_index_for_word_index(become_body_tokens, creature_idx)
            {
                trim_commas(&become_body_tokens[creature_token_idx + 1..]).to_vec()
            } else {
                Vec::new()
            };
            let suffix_supported = if suffix_tokens.is_empty() {
                true
            } else if suffix_tokens
                .first()
                .is_some_and(|token| token.is_word("with"))
            {
                parse_ability_line(&trim_commas(&suffix_tokens[1..]))
                    .map(|actions| {
                        abilities = actions
                            .into_iter()
                            .filter_map(|action| keyword_action_to_static_ability(action))
                            .collect::<Vec<_>>();
                        !abilities.is_empty()
                    })
                    .unwrap_or(false)
            } else {
                false
            };

            let colors = if colors.is_empty() {
                None
            } else {
                Some(colors)
            };
            if !all_prefix_words_supported || !suffix_supported {
                return Ok(EffectAst::BecomeBasePtCreature {
                    power: Value::Fixed(power),
                    toughness: Value::Fixed(toughness),
                    target,
                    card_types: vec![CardType::Creature],
                    subtypes: Vec::new(),
                    colors: None,
                    abilities: Vec::new(),
                    duration,
                });
            }
            return Ok(EffectAst::BecomeBasePtCreature {
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                target,
                card_types,
                subtypes,
                colors,
                abilities,
                duration,
            });
        }
    }

    if let Some((descriptor_words, power, toughness)) = parse_become_base_pt_tail(become_words)?
        && let Some((card_types, subtypes, colors)) =
            parse_become_creature_descriptor_words(descriptor_words)
    {
        return Ok(EffectAst::BecomeBasePtCreature {
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            target,
            card_types,
            subtypes,
            colors,
            abilities: Vec::new(),
            duration,
        });
    }

    let addition_tail_len = if word_slice_ends_with(
        become_words,
        &["in", "addition", "to", "its", "other", "types"],
    ) {
        Some(6usize)
    } else if word_slice_ends_with(
        become_words,
        &["in", "addition", "to", "their", "other", "types"],
    ) {
        Some(6usize)
    } else if word_slice_ends_with(
        become_words,
        &["in", "addition", "to", "its", "other", "type"],
    ) {
        Some(6usize)
    } else if word_slice_ends_with(
        become_words,
        &["in", "addition", "to", "their", "other", "type"],
    ) {
        Some(6usize)
    } else {
        None
    };
    let card_type_words = if let Some(tail_len) = addition_tail_len {
        &become_words[..become_words.len().saturating_sub(tail_len)]
    } else {
        become_words
    };
    if !card_type_words.is_empty() {
        let mut card_types = Vec::new();
        let mut all_card_types = true;
        for word in card_type_words {
            if let Some(card_type) = parse_card_type(word) {
                push_unique_card_type(&mut card_types, card_type);
            } else {
                all_card_types = false;
                break;
            }
        }
        if all_card_types && !card_types.is_empty() {
            return Ok(EffectAst::AddCardTypes {
                target,
                card_types,
                duration,
            });
        }
    }

    if !card_type_words.is_empty() {
        let mut subtypes = Vec::new();
        let mut all_subtypes = true;
        for word in card_type_words {
            if let Some(subtype) = parse_subtype_word_or_plural(word) {
                push_unique_subtype(&mut subtypes, subtype);
            } else {
                all_subtypes = false;
                break;
            }
        }
        if all_subtypes && !subtypes.is_empty() {
            return Ok(EffectAst::AddSubtypes {
                target,
                subtypes,
                duration,
            });
        }
    }

    let color_tokens = become_words
        .iter()
        .copied()
        .filter(|word| *word != "and" && *word != "or")
        .collect::<Vec<_>>();
    if !color_tokens.is_empty() {
        let mut colors = crate::color::ColorSet::new();
        let mut all_colors = true;
        for word in color_tokens {
            if let Some(color) = parse_color(word) {
                colors = colors.union(color);
            } else {
                all_colors = false;
                break;
            }
        }
        if all_colors && !colors.is_empty() {
            return Ok(EffectAst::SetColors {
                target,
                colors,
                duration,
            });
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported become clause (clause: '{}')",
        render_lower_words(rest_tokens)
    )))
}

use super::super::super::grammar::primitives::TokenWordView;
use super::super::super::keyword_static::{
    keyword_action_to_static_ability, parse_ability_line, parse_pt_modifier,
};
use super::super::super::lexer::OwnedLexToken;
use super::super::super::token_primitives::{
    find_str_by as find_word_index_by, slice_contains_str as word_slice_contains,
    slice_ends_with as word_slice_ends_with, slice_starts_with as word_slice_starts_with,
};
use super::super::super::util::{
    parse_card_type, parse_color, parse_subject, parse_target_phrase, parse_value,
    span_from_tokens, token_index_for_word_index, trim_commas,
};
use super::super::clause_pattern_helpers::extract_subject_player;
use super::super::parse_subtype_word;
use super::super::search_library::parse_restriction_duration;
use super::super::zone_counter_helpers::parse_half_starting_life_total_value;
use super::helpers::{
    parse_become_base_pt_tail, parse_become_creature_descriptor_words,
    parse_subtype_word_or_plural, push_unique_card_type, push_unique_subtype, render_lower_words,
    strip_base_power_toughness_subject_tokens, subject_references_base_power_toughness,
};
use crate::cards::builders::{CardTextError, EffectAst, IT_TAG, TagKey, TargetAst};
use crate::effect::{Until, Value};
use crate::target::ChooseSpec;
use crate::types::{CardType, Subtype};

fn split_trailing_except_tokens(
    tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, Option<Vec<OwnedLexToken>>) {
    let token_word_view = TokenWordView::new(tokens);
    let token_words = token_word_view.to_word_refs();
    let Some(except_word_idx) = token_words.iter().rposition(|word| *word == "except") else {
        return (tokens.to_vec(), None);
    };
    let Some(except_token_idx) = token_index_for_word_index(tokens, except_word_idx) else {
        return (tokens.to_vec(), None);
    };
    let exception = trim_commas(&tokens[except_token_idx + 1..]);
    (
        trim_commas(&tokens[..except_token_idx]),
        (!exception.is_empty()).then_some(exception),
    )
}

fn parse_copy_exception_preserves_source_abilities(tokens: &[OwnedLexToken]) -> bool {
    let token_words = TokenWordView::new(tokens).to_word_refs();
    token_words == ["it", "has", "this", "ability"]
}

pub(crate) fn parse_become_clause(
    subject_tokens: &[OwnedLexToken],
    rest_tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let subject_tokens = trim_commas(subject_tokens).to_vec();
    let rest_tokens = trim_commas(rest_tokens).to_vec();
    let (rest_core_tokens, copy_exception_tokens) = split_trailing_except_tokens(&rest_tokens);
    let preserve_source_abilities = copy_exception_tokens
        .as_deref()
        .is_some_and(parse_copy_exception_preserves_source_abilities);
    let become_clause_tokens = if preserve_source_abilities {
        rest_core_tokens.as_slice()
    } else {
        rest_tokens.as_slice()
    };
    let (duration, subject_tokens_vec, become_tokens) = if let Some((duration, remainder)) =
        parse_restriction_duration(&subject_tokens)?
    {
        (duration, remainder, become_clause_tokens.to_vec())
    } else if let Some((duration, remainder)) = parse_restriction_duration(become_clause_tokens)? {
        (duration, subject_tokens.clone(), remainder)
    } else {
        (
            Until::Forever,
            subject_tokens.clone(),
            become_clause_tokens.to_vec(),
        )
    };
    let subject_tokens = subject_tokens_vec.as_slice();
    let subject_word_view = TokenWordView::new(subject_tokens);
    let subject_words = subject_word_view.to_word_refs();
    let subject_targets_base_pt = subject_references_base_power_toughness(&subject_words);
    let target_subject_tokens =
        strip_base_power_toughness_subject_tokens(subject_tokens, &subject_words);
    let target_subject_word_view = TokenWordView::new(target_subject_tokens);
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
    let become_word_view = TokenWordView::new(become_body_tokens);
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
                        render_lower_words(&rest_tokens)
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
                render_lower_words(&rest_tokens)
            )));
        };
        let source_tokens = trim_commas(&become_body_tokens[source_start..]);
        if source_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                render_lower_words(&rest_tokens)
            )));
        }
        let source = parse_target_phrase(&source_tokens)?;
        return Ok(EffectAst::BecomeCopy {
            target,
            source,
            duration,
            preserve_source_abilities,
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
                            .filter_map(keyword_action_to_static_ability)
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
        render_lower_words(&rest_tokens)
    )))
}

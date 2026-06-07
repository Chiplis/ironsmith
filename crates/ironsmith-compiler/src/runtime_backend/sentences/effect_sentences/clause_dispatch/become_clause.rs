use super::super::super::keyword_static::{
    keyword_action_to_static_ability, parse_ability_line, parse_pt_modifier_values,
};
use super::super::super::lexer::{
    LexedClause, OwnedLexToken, word_slice_eq, word_slice_eq_any, word_slice_starts_with,
    word_slice_strip_any_suffix,
};
use super::super::super::util::{
    parse_card_type, parse_color, parse_subject, parse_target_phrase, parse_value,
    span_from_tokens, word_refs_except,
};
use super::super::super::object_filters::parse_object_filter_lexed;
use super::super::clause_pattern_helpers::extract_subject_player;
use super::super::parse_granted_abilities_for_gain_clause;
use super::super::parse_subtype_word;
use super::super::search_library::parse_restriction_duration;
use super::super::zone_counter_helpers::parse_half_starting_life_total_value;
use super::helpers::{
    parse_become_base_pt_tail, parse_become_creature_descriptor_words,
    parse_pt_value_words, parse_subtype_word_or_plural, push_unique_card_type,
    push_unique_subtype, render_lower_words, strip_base_power_toughness_subject_tokens,
    subject_references_base_power_toughness,
};
use crate::cards::builders::GrantedAbilityAst;
use crate::effect::{Until, Value};
use crate::host::{CardTextError, EffectAst, IT_TAG, TagKey, TargetAst};
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::{CardType, Subtype};

const ADDITION_TAIL_PHRASES: &[&[&str]] = &[
    &["in", "addition", "to", "its", "other", "types"],
    &["in", "addition", "to", "their", "other", "types"],
    &["in", "addition", "to", "its", "other", "type"],
    &["in", "addition", "to", "their", "other", "type"],
];
const ARTICLE_PREFIXES: &[&[&str]] = &[&["the"], &["a"], &["an"]];
const MONARCH_WORDS: &[&str] = &["monarch"];
const COPY_OF_PREFIX: &[&str] = &["copy", "of"];
const COPY_PRESERVES_SOURCE_ABILITIES_WORDS: &[&str] = &["it", "has", "this", "ability"];
const IT_THEY_THEM_CLAUSES: &[&[&str]] = &[&["it"], &["they"], &["them"]];
const THIS_SOURCE_SUBJECT_CLAUSES: &[&[&str]] = &[
    &["this"],
    &["this", "permanent"],
    &["this", "creature"],
    &["this", "land"],
];
const BASIC_LAND_TYPE_CHOICE_WORDS: &[&str] = &["basic", "land", "type", "of", "your", "choice"];
const COLOR_CHOICE_CLAUSES: &[&[&str]] = &[
    &["color", "of", "your", "choice"],
    &["color", "or", "colors", "of", "your", "choice"],
    &["colors", "of", "your", "choice"],
];
const CREATURE_TYPE_CHOICE_WORDS: &[&str] = &["creature", "type", "of", "your", "choice"];
const COLORLESS_WORDS: &[&str] = &["colorless"];
const AURA_ENCHANTMENT_WITH_ENCHANT_CREATURE_PREFIX: &[&str] =
    &["aura", "enchantment", "with", "enchant", "creature"];
const AURA_WITH_ENCHANT_CREATURE_PREFIX: &[&str] = &["aura", "with", "enchant", "creature"];
const SADDLED_WORDS: &[&str] = &["saddled"];
const YOU_CONTROL_PREFIX: &[&str] = &["you", "control"];
const EQUAL_TO_PREFIX: &[&str] = &["equal", "to"];
const SOURCE_POWER_TOUGHNESS_CLAUSES: &[&[&str]] = &[
    &["this", "power", "and", "toughness"],
    &["thiss", "power", "and", "toughness"],
    &["source", "power", "and", "toughness"],
];
const CREATURE_OR_CREATURES_WORDS: &[&str] = &["creature", "creatures"];

fn split_trailing_except_tokens(
    tokens: &[OwnedLexToken],
) -> (Vec<OwnedLexToken>, Option<Vec<OwnedLexToken>>) {
    let clause = LexedClause::new(tokens);
    let Some(except_word_idx) = clause.rfind_word("except") else {
        return (tokens.to_vec(), None);
    };
    let Some(except_token_idx) = clause.token_index_for_word_index(except_word_idx) else {
        return (tokens.to_vec(), None);
    };
    let exception = clause.from(except_token_idx + 1).trim();
    (
        clause.before(except_token_idx).trim(),
        (!exception.is_empty()).then_some(exception),
    )
}

fn strip_trailing_addition_tail_tokens(tokens: &[OwnedLexToken]) -> &[OwnedLexToken] {
    LexedClause::new(tokens)
        .strip_any_suffix_clause(ADDITION_TAIL_PHRASES)
        .map(|(_, head)| head.tokens())
        .unwrap_or(tokens)
}

fn is_addition_tail_only(tokens: &[OwnedLexToken]) -> bool {
    !tokens.is_empty() && strip_trailing_addition_tail_tokens(tokens).is_empty()
}

fn is_still_a_land_suffix(tokens: &[OwnedLexToken]) -> bool {
    let words = LexedClause::new(tokens).word_refs();
    matches!(
        words.as_slice(),
        ["still", "a", "land"]
            | ["that", "s", "still", "a", "land"]
            | ["thats", "still", "a", "land"]
            | ["it", "s", "still", "a", "land"]
            | ["its", "still", "a", "land"]
    )
}

fn recover_leading_duration_target(tokens: &[OwnedLexToken]) -> Option<TargetAst> {
    let clause = LexedClause::new(tokens).trimmed();
    let words = clause.word_refs();
    let target_word_idx = words.iter().position(|word| *word == "target")?;
    let target_token_idx = clause.token_index_for_word_index(target_word_idx)?;
    let target_tokens = clause.from(target_token_idx).trim();
    if target_tokens.len() <= 1 {
        return None;
    }
    parse_target_phrase(&target_tokens).ok()
}

fn parse_copy_exception_preserves_source_abilities(tokens: &[OwnedLexToken]) -> bool {
    word_slice_eq(
        &LexedClause::new(tokens).word_refs(),
        COPY_PRESERVES_SOURCE_ABILITIES_WORDS,
    )
}

pub(crate) fn parse_become_clause(
    subject_tokens: &[OwnedLexToken],
    rest_tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let subject_tokens = LexedClause::new(subject_tokens).trim();
    let original_subject_tokens = subject_tokens.clone();
    let rest_clause = LexedClause::new(rest_tokens).trimmed();
    let rest_tokens = if rest_clause
        .first_word()
        .is_some_and(|word| matches!(word, "become" | "becomes"))
    {
        rest_clause.from(1).trim()
    } else {
        rest_clause.tokens().to_vec()
    };
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
    let subject_clause = LexedClause::new(subject_tokens);
    let subject_words = subject_clause.word_refs();
    let subject_targets_base_pt = subject_references_base_power_toughness(&subject_words);
    let target_subject_tokens =
        strip_base_power_toughness_subject_tokens(subject_tokens, &subject_words);
    let target_subject_clause = LexedClause::new(target_subject_tokens);
    let target_subject_words = target_subject_clause.word_refs();
    let subject = parse_subject(subject_tokens);
    let become_clause = LexedClause::new(&become_tokens);
    let become_body_clause = if let Some((_article, after_article)) =
        become_clause.strip_any_prefix_clause(ARTICLE_PREFIXES)
    {
        after_article
    } else {
        become_clause
    };
    let become_words_vec = become_body_clause.word_refs();
    let become_words = &become_words_vec[..];

    if let Some(player) = extract_subject_player(Some(subject)) {
        if word_slice_eq(become_words, MONARCH_WORDS) {
            return Ok(EffectAst::subject_verb_become_monarch(player));
        }
        if subject_clause.contains_word("life") && subject_clause.contains_word("total") {
            let amount = parse_value(&become_tokens)
                .map(|(value, _)| value)
                .or_else(|| parse_half_starting_life_total_value(&become_tokens, player))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing life total amount (clause: '{}')",
                        render_lower_words(&rest_tokens)
                    ))
                })?;
            return Ok(EffectAst::subject_verb_set_life_total(player, amount));
        }
    }

    let mut target = if target_subject_words.as_slice() == ["all"] {
        let inferred_filter = if become_words.len() == 1
            && parse_color(become_words[0]).is_some()
        {
            Some(ObjectFilter::creature())
        } else if become_words.len() == 1
            && parse_subtype_word_or_plural(become_words[0]).is_some_and(|subtype| {
                matches!(
                    subtype,
                    Subtype::Plains
                        | Subtype::Island
                        | Subtype::Swamp
                        | Subtype::Mountain
                        | Subtype::Forest
                )
            })
        {
            Some(ObjectFilter::land())
        } else {
            None
        };
        TargetAst::Object(
            inferred_filter.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported mass become subject (clause: '{}')",
                    render_lower_words(subject_tokens)
                ))
            })?,
            None,
            None,
        )
    } else if target_subject_words
        .first()
        .is_some_and(|word| matches!(*word, "all" | "each"))
        && target_subject_words.len() > 1
    {
        TargetAst::Object(parse_object_filter_lexed(&target_subject_tokens[1..], false)?, None, None)
    } else if target_subject_words.is_empty()
        || word_slice_eq_any(&target_subject_words, IT_THEY_THEM_CLAUSES)
        || super::is_tagged_object_reference(&target_subject_words)
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
    } else if word_slice_eq_any(&target_subject_words, THIS_SOURCE_SUBJECT_CLAUSES) {
        TargetAst::Source(span_from_tokens(subject_tokens))
    } else {
        parse_target_phrase(target_subject_tokens)?
    };

    if matches!(target, TargetAst::AnyTarget(_))
        && let Some(recovered_target) = recover_leading_duration_target(&original_subject_tokens)
    {
        target = recovered_target;
    }

    if word_slice_eq(become_words, BASIC_LAND_TYPE_CHOICE_WORDS) {
        return Ok(EffectAst::subject_verb_become_basic_land_type_choice(
            target, duration,
        ));
    }

    if let [word] = become_words
        && let Some(subtype) = parse_subtype_word_or_plural(word)
        && matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    {
        return Ok(EffectAst::subject_verb_become_basic_land_type(
            target, subtype, duration,
        ));
    }

    if word_slice_eq_any(become_words, COLOR_CHOICE_CLAUSES) {
        return Ok(EffectAst::subject_verb_become_color_choice(
            target, duration,
        ));
    }

    if word_slice_eq(become_words, CREATURE_TYPE_CHOICE_WORDS) {
        return Ok(EffectAst::subject_verb_become_creature_type_choice(
            target,
            duration,
            Vec::new(),
        ));
    }

    if word_slice_starts_with(become_words, COPY_OF_PREFIX) {
        let Some(source_clause) = become_body_clause.after_words(2) else {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                render_lower_words(&rest_tokens)
            )));
        };
        let source_tokens = source_clause.trim();
        if source_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                render_lower_words(&rest_tokens)
            )));
        }
        let source = parse_target_phrase(&source_tokens)?;
        return Ok(EffectAst::subject_verb_become_copy(
            target,
            source,
            duration,
            preserve_source_abilities,
        ));
    }

    if word_slice_eq(become_words, COLORLESS_WORDS) {
        return Ok(EffectAst::subject_verb_make_colorless(target, duration));
    }

    let aura_with_enchant_creature_clause =
        if word_slice_starts_with(become_words, AURA_ENCHANTMENT_WITH_ENCHANT_CREATURE_PREFIX) {
            become_body_clause.after_words(5)
        } else if word_slice_starts_with(become_words, AURA_WITH_ENCHANT_CREATURE_PREFIX) {
            become_body_clause.after_words(4)
        } else {
            None
        };
    if word_slice_eq(become_words, SADDLED_WORDS) && duration == Until::EndOfTurn {
        return Ok(EffectAst::subject_verb_become_saddled_until_end_of_turn(
            target,
        ));
    }
    if let Some(aura_tail_clause) = aura_with_enchant_creature_clause {
        if matches!(
            target_subject_words.as_slice(),
            ["it"] | ["this"] | ["this", "creature"]
        ) || matches!(&target, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG)
        {
            target = TargetAst::Source(span_from_tokens(subject_tokens));
        }
        let attachment_filter =
            if word_slice_starts_with(&aura_tail_clause.word_refs(), YOU_CONTROL_PREFIX) {
                ObjectFilter::creature().you_control()
            } else {
                ObjectFilter::creature()
            };
        return Ok(EffectAst::subject_verb_become_aura_enchantment(
            target,
            attachment_filter,
            duration,
        ));
    }

    if word_slice_starts_with(become_words, EQUAL_TO_PREFIX) {
        if become_body_clause
            .after_words(2)
            .is_some_and(|rhs| word_slice_eq_any(&rhs.word_refs(), SOURCE_POWER_TOUGHNESS_CLAUSES))
        {
            return Ok(EffectAst::subject_verb_set_base_power_toughness(
                Value::PowerOf(Box::new(ChooseSpec::Source)),
                Value::ToughnessOf(Box::new(ChooseSpec::Source)),
                target,
                duration,
            ));
        }
    }

    if let Some((power, toughness, pt_word_count)) = parse_pt_value_words(become_words)
    {
        if subject_targets_base_pt || become_words.len() == 1 {
            return Ok(EffectAst::subject_verb_set_base_power_toughness(
                power, toughness, target, duration,
            ));
        }
        if let Some(creature_idx) = become_words
            .iter()
            .position(|word| CREATURE_OR_CREATURES_WORDS.contains(word))
        {
            let mut card_types = vec![CardType::Creature];
            let mut subtypes = Vec::new();
            let mut colors = crate::color::ColorSet::new();
            let mut all_prefix_words_supported = true;
            for word in &become_words[pt_word_count..creature_idx] {
                if matches!(*word, "a" | "an" | "the" | "and") {
                    continue;
                }
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
            let mut granted_abilities = Vec::<GrantedAbilityAst>::new();
            let suffix_tokens = if let Some(creature_token_idx) =
                become_body_clause.token_index_for_word_index(creature_idx)
            {
                become_body_clause.from(creature_token_idx + 1).trim()
            } else {
                Vec::new()
            };
            let suffix_supported = if suffix_tokens.is_empty() {
                true
            } else if is_addition_tail_only(&suffix_tokens) {
                true
            } else if is_still_a_land_suffix(&suffix_tokens) {
                true
            } else if suffix_tokens
                .first()
                .and_then(OwnedLexToken::as_word)
                .is_some_and(|word| word == "with")
            {
                let trimmed_suffix_tokens = LexedClause::new(&suffix_tokens[1..]).trim();
                let trimmed_suffix = strip_trailing_addition_tail_tokens(&trimmed_suffix_tokens);
                let suffix_words = LexedClause::new(trimmed_suffix).word_refs();
                if let Ok((parsed_abilities, _)) =
                    parse_granted_abilities_for_gain_clause(trimmed_suffix, &suffix_words, false)
                    && !parsed_abilities.is_empty()
                {
                    granted_abilities = parsed_abilities;
                    true
                } else {
                    parse_ability_line(trimmed_suffix)
                        .map(|actions| {
                            abilities = actions
                                .into_iter()
                                .filter_map(keyword_action_to_static_ability)
                                .collect::<Vec<_>>();
                            !abilities.is_empty()
                        })
                        .unwrap_or(false)
                }
            } else {
                false
            };

            let colors = if colors.is_empty() {
                None
            } else {
                Some(colors)
            };
            if !all_prefix_words_supported || !suffix_supported {
                return Ok(EffectAst::subject_verb_become_base_pt_creature(
                    power,
                    toughness,
                    target,
                    vec![CardType::Creature],
                    Vec::new(),
                    None,
                    Vec::new(),
                    Vec::new(),
                    duration,
                ));
            }
            return Ok(EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                card_types,
                subtypes,
                colors,
                abilities,
                granted_abilities,
                duration,
            ));
        }
    }

    if let Some((descriptor_words, power, toughness)) = parse_become_base_pt_tail(become_words)?
        && let Some((card_types, subtypes, colors)) =
            parse_become_creature_descriptor_words(descriptor_words)
    {
        return Ok(EffectAst::subject_verb_become_base_pt_creature(
            power,
            toughness,
            target,
            card_types,
            subtypes,
            colors,
            Vec::new(),
            Vec::new(),
            duration,
        ));
    }

    let card_type_words = word_slice_strip_any_suffix(become_words, ADDITION_TAIL_PHRASES)
        .map(|(_, head)| head)
        .unwrap_or(become_words);
    if !card_type_words.is_empty() {
        let mut colors = crate::color::ColorSet::new();
        let mut subtypes = Vec::new();
        let mut all_color_or_subtype_words = true;
        for word in card_type_words {
            if *word == "and" {
                continue;
            }
            if let Some(color) = parse_color(word) {
                colors = colors.union(color);
                continue;
            }
            if let Some(subtype) = parse_subtype_word_or_plural(word) {
                push_unique_subtype(&mut subtypes, subtype);
                continue;
            }
            all_color_or_subtype_words = false;
            break;
        }
        if all_color_or_subtype_words && !colors.is_empty() && !subtypes.is_empty() {
            return Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_set_colors(target.clone(), colors, duration.clone()),
                    EffectAst::subject_verb_add_subtypes(target, subtypes, duration),
                ],
            });
        }
    }
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
            return Ok(EffectAst::subject_verb_add_card_types(
                target, card_types, duration,
            ));
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
            return Ok(EffectAst::subject_verb_add_subtypes(
                target, subtypes, duration,
            ));
        }
    }

    if become_words.len() >= 8
        && let [
            color_word,
            "until",
            "end",
            "of",
            "turn",
            "and",
            "attacks",
            tail @ ..,
        ] = become_words
        && matches!(tail, ["if", "able"] | ["this", "turn", "if", "able"])
        && parse_color(color_word).is_some()
    {
        let mut colors = crate::color::ColorSet::new();
        if let Some(color) = parse_color(color_word) {
            colors = colors.union(color);
        }
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_set_colors(target.clone(), colors, Until::EndOfTurn),
                EffectAst::subject_verb_grant_abilities_to_target(
                    target,
                    vec![GrantedAbilityAst::MustAttack],
                    Until::EndOfTurn,
                ),
            ],
        });
    }

    let color_tokens = word_refs_except(become_words, &["and", "or"]);
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
            return Ok(EffectAst::subject_verb_set_colors(target, colors, duration));
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported become clause (clause: '{}')",
        render_lower_words(&rest_tokens)
    )))
}

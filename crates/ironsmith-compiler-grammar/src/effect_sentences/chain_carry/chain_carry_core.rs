use crate::cards::builders::ControlActionAst;
use crate::cards::builders::CharacteristicActionAst;
use crate::cards::builders::GrantActionAst;
use crate::cards::builders::DamagePreventionActionAst;
use super::*;

pub(super) fn is_orphan_rounded_up_where_x_tail(
    segment: &[OwnedLexToken],
    previous: Option<&[OwnedLexToken]>,
    previous_effect: Option<&EffectAst>,
) -> bool {
    if !chain_grammar::is_rounded_up_segment_tokens(segment) {
        return false;
    }
    if previous.is_none() && previous_effect.is_none() {
        return true;
    }
    previous.is_some_and(chain_grammar::has_where_x_is_half_tokens)
        || previous_effect.is_some_and(effect_uses_half_life_total_value)
}

pub(super) fn is_standalone_where_x_binding_segment(tokens: &[OwnedLexToken]) -> bool {
    let words = token_word_refs(trim_lexed_commas(tokens));
    let start = usize::from(crate::word_primitives::parse_any_sequence_prefix(
        &words,
        &[&["and"], &["then"]],
    ));
    words.get(start..).is_some_and(|tail| {
        crate::word_primitives::parse_sequence_prefix(tail, &["where", "x", "is"])
    }) && !has_explicit_comma_then_boundary_lexed(tokens)
}

pub(super) fn apply_carried_effect_duration(effect: &mut EffectAst, duration: &Until) {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::Control(ControlActionAst::GainControl {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::Pump {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpForEach {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpAll {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::PumpByLastEffect {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePowerToughness {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetBasePower {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasePtCreature {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddCardTypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCardTypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveCardTypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddSubtypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveSubtypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetCreatureSubtypes {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddColors {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::AddAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAllSubtypesOfFamily {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeAuraEnchantment {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::SetColors {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::MakeColorless {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandType {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeBasicLandTypeChoice {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeColorChoice {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCreatureTypeChoice {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Characteristics(CharacteristicActionAst::BecomeCopy {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesToTarget {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesAll {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Grants(GrantActionAst::GrantAbilitiesChoiceToTarget {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesFromTarget {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::StatChanges(StatChangeActionAst::RemoveAbilitiesAll {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTarget {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageToTargetFromSourceFilter {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::DamagePrevention(DamagePreventionActionAst::PreventAllDamageFromSourceFilter {
                    duration: effect_duration,
                    ..
                })
                | SubjectVerbActionAst::Cant {
                    duration: effect_duration,
                    ..
                },
            ..
        }) if matches!(effect_duration, Until::Forever) => {
            *effect_duration = duration.clone();
        }
        EffectAst::Conditionals(ConditionalEffectAst::Conditional {
            if_true, if_false, ..
        }) => {
            for nested in if_true.iter_mut().chain(if_false.iter_mut()) {
                apply_carried_effect_duration(nested, duration);
            }
        }
        // A compound continuous clause ("has base power and toughness 4/4
        // and gains flying") parses into a coordinated wrapper; the carried
        // duration scopes every member. The Forever guard above keeps
        // authored member durations intact.
        EffectAst::Sequence { effects } | EffectAst::Coordinated { effects, .. } => {
            for nested in effects.iter_mut() {
                apply_carried_effect_duration(nested, duration);
            }
        }
        _ => {}
    }
}

pub(super) fn split_on_comma_or_semicolon_lexed(
    tokens: &[OwnedLexToken],
) -> Vec<Vec<OwnedLexToken>> {
    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if inside_quotes || !matches!(token.kind, TokenKind::Comma | TokenKind::Semicolon) {
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

pub fn expand_segments_with_comma_action_clauses_lexed(
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
            while let Some(rest) = chain_grammar::strip_leading_and_tokens(&part) {
                part = rest.to_vec();
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

pub fn expand_missing_verb_segment_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> Option<Vec<OwnedLexToken>> {
    let (verb, verb_idx) = find_verb_lexed(previous)?;
    match verb {
        Verb::Deal => {
            if parse_value_prefix_lexed(segment).is_none()
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
            ) || parse_number_prefix_lexed(segment).is_some();
            if !starts_like_object_phrase {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Exile => {
            let segment_words = token_word_refs(segment);
            let starts_like_object_reference = matches!(
                segment_words.first().copied(),
                Some(
                    "a" | "an"
                        | "another"
                        | "target"
                        | "it"
                        | "them"
                        | "this"
                        | "that"
                        | "these"
                        | "those"
                )
            ) || parse_number_prefix_lexed(segment).is_some();
            if !starts_like_object_reference {
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

pub fn find_verb(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    find_verb_lexed(tokens)
}

pub fn parse_effect_chain(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_lexed(tokens)
}

pub fn parse_effect_chain_lexed_with_context(
    context: crate::parse_context::ParseContextView<'_>,
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    let normalized = normalize_source_references_with_context(context, tokens)?;
    parse_effect_chain_lexed(&normalized)
}

pub fn parse_effect_chain_inner(tokens: &[OwnedLexToken]) -> Result<Vec<EffectAst>, CardTextError> {
    parse_effect_chain_inner_lexed(tokens)
}

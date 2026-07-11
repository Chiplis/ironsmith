use super::super::super::keyword_static::{keyword_action_to_static_ability, parse_ability_line};
use super::super::super::lexer::{LexedClause, OwnedLexToken};
use super::super::super::object_filters::parse_object_filter_lexed;
use super::super::super::util::{
    parse_subject, parse_target_phrase, parse_value, span_from_tokens,
};
use super::super::clause_pattern_helpers::extract_subject_player;
use super::super::parse_granted_abilities_for_gain_clause;
use super::super::search_library::parse_restriction_duration;
use super::super::zone_counter_helpers::parse_half_starting_life_total_value;
use super::helpers::render_lower_words;
use crate::cards::builders::GrantedAbilityAst;
use crate::effect::{Until, Value};
use crate::host::{CardTextError, EffectAst, IT_TAG, TagKey, TargetAst};
use crate::runtime_backend::grammar::effects::become_shapes as become_grammar;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::types::{CardType, SubtypeFamily};

pub(crate) fn parse_become_clause(
    subject_tokens: &[OwnedLexToken],
    rest_tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let subject_tokens = LexedClause::new(subject_tokens).trim();
    let original_subject_tokens = subject_tokens.clone();
    let rest_shape = become_grammar::parse_become_rest_shape(rest_tokens);
    let rest_tokens = rest_shape.rest_tokens;
    let copy_exception = rest_shape.copy_exception;
    let become_clause_tokens = rest_shape.body_tokens;
    let (duration, subject_tokens_vec, become_tokens) = if let Some((duration, remainder)) =
        parse_restriction_duration(&subject_tokens)?
    {
        (duration, remainder, become_clause_tokens)
    } else if let Some((duration, remainder)) = parse_restriction_duration(&become_clause_tokens)? {
        (duration, subject_tokens.clone(), remainder)
    } else {
        (Until::Forever, subject_tokens.clone(), become_clause_tokens)
    };
    let subject_tokens = subject_tokens_vec.as_slice();
    let base_pt_subject = become_grammar::parse_base_power_toughness_subject_tokens(subject_tokens);
    let subject_targets_base_pt = base_pt_subject.is_some();
    let target_subject_tokens = base_pt_subject
        .map(|shape| shape.target_tokens)
        .unwrap_or(subject_tokens);
    let subject = parse_subject(subject_tokens);
    let become_surface = become_grammar::parse_become_body_surface_shape(&become_tokens);
    let become_body_tokens = become_surface.body_tokens;
    let become_words_vec =
        crate::runtime_backend::front_end::lexer::parser_token_word_refs(become_body_tokens);
    let become_words = &become_words_vec[..];

    if let Some(player) = extract_subject_player(Some(subject)) {
        if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Monarch) {
            return Ok(EffectAst::subject_verb_become_monarch(player));
        }
        if become_grammar::become_subject_has_life_total(subject_tokens) {
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

    let target_subject_shape = become_grammar::parse_become_target_subject_shape(
        target_subject_tokens,
        become_body_tokens,
    );
    let mut target = match target_subject_shape {
        become_grammar::BecomeTargetSubjectShape::Mass(kind) => {
            let inferred_filter = match kind {
                become_grammar::BecomeMassTargetKind::Creature => ObjectFilter::creature(),
                become_grammar::BecomeMassTargetKind::Land => ObjectFilter::land(),
                become_grammar::BecomeMassTargetKind::Unsupported => {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mass become subject (clause: '{}')",
                        render_lower_words(subject_tokens)
                    )));
                }
            };
            TargetAst::Object(inferred_filter, None, None)
        }
        become_grammar::BecomeTargetSubjectShape::Tagged => {
            TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens))
        }
        become_grammar::BecomeTargetSubjectShape::FilteredMany(filter_tokens) => {
            TargetAst::Object(parse_object_filter_lexed(filter_tokens, false)?, None, None)
        }
        become_grammar::BecomeTargetSubjectShape::Source(surface) => TargetAst::Object(
            ObjectFilter::source().with_source_surface(surface),
            None,
            span_from_tokens(subject_tokens),
        ),
        become_grammar::BecomeTargetSubjectShape::Parsed(target_tokens) => {
            parse_target_phrase(target_tokens)?
        }
    };

    if matches!(target, TargetAst::AnyTarget(_))
        && let Some(recovered_tokens) =
            become_grammar::parse_leading_duration_target_tokens(&original_subject_tokens)
        && let Ok(recovered_target) = parse_target_phrase(recovered_tokens)
    {
        target = recovered_target;
    }

    match become_surface.exact_kind {
        Some(become_grammar::BecomeExactKind::BasicLandTypeChoice) => {
            return Ok(EffectAst::subject_verb_become_basic_land_type_choice(
                target, duration,
            ));
        }
        Some(become_grammar::BecomeExactKind::BasicLandType(subtype)) => {
            return Ok(EffectAst::subject_verb_become_basic_land_type(
                target, subtype, duration,
            ));
        }
        Some(become_grammar::BecomeExactKind::ColorChoice) => {
            return Ok(EffectAst::subject_verb_become_color_choice(
                target, duration,
            ));
        }
        Some(become_grammar::BecomeExactKind::CreatureTypeChoice) => {
            return Ok(EffectAst::subject_verb_become_creature_type_choice(
                target,
                duration,
                Vec::new(),
            ));
        }
        _ => {}
    }

    match become_surface.copy_source {
        become_grammar::BecomeCopySourceShape::Missing => {
            return Err(CardTextError::ParseError(format!(
                "missing copy source in become clause (clause: '{}')",
                render_lower_words(&rest_tokens)
            )));
        }
        become_grammar::BecomeCopySourceShape::Source(source_tokens) => {
            let source = parse_target_phrase(source_tokens)?;
            return Ok(EffectAst::subject_verb_become_copy(
                target,
                source,
                duration,
                copy_exception
                    .as_ref()
                    .is_some_and(|exception| exception.preserve_source_abilities),
                copy_exception
                    .as_ref()
                    .and_then(|exception| exception.name_override.clone()),
                copy_exception
                    .as_ref()
                    .and_then(|exception| exception.name_override_surface.clone()),
                copy_exception
                    .as_ref()
                    .map(|exception| exception.add_supertypes.clone())
                    .unwrap_or_default(),
            ));
        }
        become_grammar::BecomeCopySourceShape::NotCopy => {}
    }

    if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Colorless) {
        return Ok(EffectAst::subject_verb_make_colorless(target, duration));
    }
    if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Saddled)
        && duration == Until::EndOfTurn
    {
        return Ok(EffectAst::subject_verb_become_saddled_until_end_of_turn(
            target,
        ));
    }
    if let Some(aura) = become_surface.aura {
        if become_grammar::aura_subject_prefers_source(target_subject_tokens)
            || matches!(&target, TargetAst::Tagged(tag, _) if tag.as_str() == IT_TAG)
        {
            target = TargetAst::Source(span_from_tokens(subject_tokens));
        }
        let attachment_filter = if aura.attachment_you_control {
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

    if become_surface.equal_to_source_power_toughness {
        return Ok(EffectAst::subject_verb_set_base_power_toughness(
            Value::PowerOf(Box::new(ChooseSpec::Source)),
            Value::ToughnessOf(Box::new(ChooseSpec::Source)),
            target,
            duration,
        ));
    }

    if let Some(leading_pt) =
        become_grammar::parse_become_leading_pt_shape(become_words, become_body_tokens)
    {
        let become_grammar::BecomeLeadingPtShape {
            power,
            toughness,
            value_word_count,
            creature_word_index,
            suffix_tokens,
        } = leading_pt;
        if subject_targets_base_pt || value_word_count == become_words.len() {
            return Ok(EffectAst::subject_verb_set_base_power_toughness(
                power, toughness, target, duration,
            ));
        }
        if let Some(creature_idx) = creature_word_index {
            let prefix = become_grammar::parse_become_leading_creature_prefix(
                &become_words[value_word_count..creature_idx],
            );
            let mut abilities = Vec::new();
            let mut granted_abilities = Vec::<GrantedAbilityAst>::new();
            let mut subtype_families = Vec::<SubtypeFamily>::new();
            let suffix_supported =
                match become_grammar::parse_become_animation_suffix_shape(suffix_tokens) {
                    become_grammar::BecomeAnimationSuffixShape::Ignored => true,
                    become_grammar::BecomeAnimationSuffixShape::Unsupported => false,
                    become_grammar::BecomeAnimationSuffixShape::With {
                        ability_tokens,
                        grants_all_creature_types,
                    } => {
                        if grants_all_creature_types {
                            subtype_families.push(SubtypeFamily::Creature);
                        }
                        let suffix_words =
                            crate::runtime_backend::front_end::lexer::parser_token_word_refs(
                                ability_tokens,
                            );
                        if ability_tokens.is_empty() {
                            grants_all_creature_types
                        } else if let Ok((parsed_abilities, _)) =
                            parse_granted_abilities_for_gain_clause(
                                ability_tokens,
                                &suffix_words,
                                false,
                            )
                            && !parsed_abilities.is_empty()
                        {
                            granted_abilities = parsed_abilities;
                            true
                        } else {
                            parse_ability_line(ability_tokens)
                                .map(|actions| {
                                    abilities = actions
                                        .into_iter()
                                        .filter_map(keyword_action_to_static_ability)
                                        .collect::<Vec<_>>();
                                    !abilities.is_empty()
                                })
                                .unwrap_or(false)
                        }
                    }
                };
            if !prefix.supported || !suffix_supported {
                return Ok(EffectAst::subject_verb_become_base_pt_creature(
                    power,
                    toughness,
                    target,
                    vec![CardType::Creature],
                    Vec::new(),
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
                prefix.card_types,
                prefix.subtypes,
                subtype_families,
                prefix.colors,
                abilities,
                granted_abilities,
                duration,
            ));
        }
    }

    if let Some(pt) = become_grammar::parse_become_base_pt_words(become_words)
        && let Some(descriptor) =
            become_grammar::parse_become_creature_descriptor_words(pt.descriptor_words)
    {
        return Ok(EffectAst::subject_verb_become_base_pt_creature(
            pt.power,
            pt.toughness,
            target,
            descriptor.card_types,
            descriptor.subtypes,
            Vec::new(),
            descriptor.colors,
            Vec::new(),
            Vec::new(),
            duration,
        ));
    }

    if let Some(pt) = become_grammar::parse_become_iterated_mana_value_pt_words(become_words)
        && let Some(descriptor) =
            become_grammar::parse_become_creature_descriptor_words(pt.descriptor_words)
    {
        return Ok(EffectAst::subject_verb_become_base_pt_creature(
            pt.power,
            pt.toughness,
            target,
            descriptor.card_types,
            descriptor.subtypes,
            Vec::new(),
            descriptor.colors,
            Vec::new(),
            Vec::new(),
            duration,
        ));
    }

    match become_grammar::parse_become_simple_descriptor_words(become_words) {
        become_grammar::BecomeSimpleDescriptorShape::ColorsAndSubtypes { colors, subtypes } => {
            return Ok(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_set_colors(target.clone(), colors, duration.clone()),
                    EffectAst::subject_verb_add_subtypes(target, subtypes, duration),
                ],
            });
        }
        become_grammar::BecomeSimpleDescriptorShape::CardTypes(card_types) => {
            return Ok(EffectAst::subject_verb_add_card_types(
                target, card_types, duration,
            ));
        }
        become_grammar::BecomeSimpleDescriptorShape::Subtypes {
            subtypes,
            replace_creature_subtypes,
        } => {
            if replace_creature_subtypes {
                return Ok(EffectAst::subject_verb_set_creature_subtypes(
                    target, subtypes, duration,
                ));
            }
            return Ok(EffectAst::subject_verb_add_subtypes(
                target, subtypes, duration,
            ));
        }
        become_grammar::BecomeSimpleDescriptorShape::None => {}
    }

    if let Some(colors) = become_grammar::parse_become_attack_color(become_words) {
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

    if let Some(colors) = become_grammar::parse_become_color_words(become_words) {
        return Ok(EffectAst::subject_verb_set_colors(target, colors, duration));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported become clause (clause: '{}')",
        render_lower_words(&rest_tokens)
    )))
}

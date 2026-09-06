use crate::cards::builders::ConditionalEffectAst;
use super::*;
use winnow::Parser;

pub fn parse_become_clause(
    subject_tokens: &[OwnedLexToken],
    rest_tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    let subject_tokens = LexedClause::new(subject_tokens).trim();
    let rest_clause = LexedClause::new(rest_tokens).trimmed();
    let rest_words = rest_clause.word_refs();
    const TRIGGERING_SPELL_COLOR_PROTECTION_SUFFIX: &[&str] = &[
        "with",
        "protection",
        "from",
        "each",
        "of",
        "that",
        "spell",
        "s",
        "colors",
    ];
    const NORMALIZED_TRIGGERING_SPELL_COLOR_PROTECTION_SUFFIX: &[&str] = &[
        "with",
        "protection",
        "from",
        "each",
        "of",
        "that",
        "spells",
        "colors",
    ];
    let triggering_spell_color_suffix_len = crate::word_primitives::parse_sequence_suffix(
        &rest_words,
        TRIGGERING_SPELL_COLOR_PROTECTION_SUFFIX,
    )
    .then_some(TRIGGERING_SPELL_COLOR_PROTECTION_SUFFIX.len())
    .or_else(|| {
        crate::word_primitives::parse_sequence_suffix(
            &rest_words,
            NORMALIZED_TRIGGERING_SPELL_COLOR_PROTECTION_SUFFIX,
        )
        .then_some(NORMALIZED_TRIGGERING_SPELL_COLOR_PROTECTION_SUFFIX.len())
    });
    if let Some(suffix_len) = triggering_spell_color_suffix_len
        && let Some(base_clause) = rest_clause.before_word(rest_words.len() - suffix_len)
    {
        let mut effects = vec![parse_become_clause(&subject_tokens, base_clause.tokens())?];
        for colors in [
            crate::color::ColorSet::WHITE,
            crate::color::ColorSet::BLUE,
            crate::color::ColorSet::BLACK,
            crate::color::ColorSet::RED,
            crate::color::ColorSet::GREEN,
        ] {
            effects.push(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: PredicateAst::TaggedMatches(
                    crate::tag::CompilerReferenceTag::Triggering.bind(),
                    ObjectFilter::default().with_colors(colors),
                ),
                if_true: vec![EffectAst::subject_verb_grant_abilities_to_target(
                    TargetAst::Source(None),
                    vec![GrantedAbilityAst::KeywordAction(Box::new(
                        crate::payload::KeywordAction::ProtectionFrom(colors),
                    ))],
                    Until::Forever,
                )],
                if_false: Vec::new(),
            }));
        }
        return Ok(EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        });
    }
    let original_subject_tokens = subject_tokens.clone();
    let rest_shape = become_grammar::parse_become_rest_shape(rest_tokens);
    let rest_tokens = rest_shape.rest_tokens;
    let copy_exception = rest_shape.copy_exception;
    let become_clause_tokens = rest_shape.body_tokens;
    let (duration, subject_tokens_vec, become_tokens, animation_duration_surface) =
        if let Some((duration, remainder)) = parse_restriction_duration(&subject_tokens)? {
            (
                duration,
                remainder,
                become_clause_tokens,
                Some(ironsmith_core::AnimationDurationSurface::Leading),
            )
        } else if let Some((duration, remainder)) =
            parse_restriction_duration(&become_clause_tokens)?
        {
            if trailing_duration_belongs_to_quoted_ability(&become_clause_tokens, &remainder) {
                (
                    Until::Forever,
                    subject_tokens.clone(),
                    become_clause_tokens,
                    None,
                )
            } else {
                (duration, subject_tokens.clone(), remainder, None)
            }
        } else {
            (
                Until::Forever,
                subject_tokens.clone(),
                become_clause_tokens,
                None,
            )
        };
    let subject_tokens = subject_tokens_vec.as_slice();
    let base_pt_subject = become_grammar::parse_base_power_toughness_subject_tokens(subject_tokens);
    let subject_targets_base_pt = base_pt_subject.is_some();
    let target_subject_tokens = base_pt_subject
        .map(|shape| shape.target_tokens)
        .unwrap_or(subject_tokens);
    let set_quantifier_surface =
        become_grammar::become_subject_set_quantifier_surface(target_subject_tokens);
    let subject = parse_subject(subject_tokens);
    let become_surface = become_grammar::parse_become_body_surface_shape(&become_tokens);
    let become_body_tokens = become_surface.body_tokens;
    let become_words_vec = crate::lexer::parser_token_word_refs(become_body_tokens);
    let become_words = &become_words_vec[..];

    if let Some(player) = extract_subject_player(Some(subject)) {
        if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Monarch) {
            return Ok(EffectAst::subject_verb_become_monarch(player));
        }
        if become_grammar::become_subject_has_life_total(subject_tokens) {
            let amount = parse_value(&become_tokens)
                .map(|(value, _)| value)
                .or_else(|| crate::effect_sentences::zone_counter_helpers::parse_starting_life_total_value(&become_tokens, player))
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
        become_grammar::BecomeTargetSubjectShape::Tagged => TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
            span_from_tokens(subject_tokens),
        ),
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
        Some(become_grammar::BecomeExactKind::ColorChoice { allow_multiple }) => {
            return Ok(EffectAst::subject_verb_become_color_choice(
                target,
                duration,
                allow_multiple,
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
            let mut source = parse_target_phrase(source_tokens)?;
            if crate::grammar::primitives::find_prefix(source_tokens, || {
                crate::grammar::primitives::kw("target").void()
            })
            .is_none()
            {
                fn clear_explicit_target_span(target: &mut TargetAst) {
                    match target {
                        TargetAst::Object(_, explicit_target_span, _) => {
                            *explicit_target_span = None;
                        }
                        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
                            clear_explicit_target_span(inner);
                        }
                        _ => {}
                    }
                }
                clear_explicit_target_span(&mut source);
            }
            let granted_abilities = if let Some(ability_tokens) = copy_exception
                .as_ref()
                .and_then(|exception| exception.granted_ability_tokens.as_deref())
            {
                let (abilities, is_choice) =
                    parse_granted_abilities_for_gain_clause(ability_tokens, become_words, false)?;
                if is_choice || abilities.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported copy-exception ability (clause: '{}')",
                        render_lower_words(ability_tokens)
                    )));
                }
                abilities
            } else {
                Vec::new()
            };
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
                copy_exception
                    .as_ref()
                    .map(|exception| exception.remove_supertypes.clone())
                    .unwrap_or_default(),
                copy_exception
                    .as_ref()
                    .map(|exception| exception.add_colors)
                    .unwrap_or_default(),
                copy_exception
                    .as_ref()
                    .map(|exception| exception.add_card_types.clone())
                    .unwrap_or_default(),
                copy_exception
                    .as_ref()
                    .map(|exception| exception.set_card_types.clone())
                    .unwrap_or_default(),
                copy_exception
                    .as_ref()
                    .map(|exception| exception.add_subtypes.clone())
                    .unwrap_or_default(),
                copy_exception
                    .as_ref()
                    .map(|exception| exception.set_subtypes.clone())
                    .unwrap_or_default(),
                granted_abilities,
                copy_exception
                    .as_ref()
                    .and_then(|exception| exception.set_base_power_toughness)
                    .map(|(power, toughness)| (Value::Fixed(power), Value::Fixed(toughness))),
                copy_exception.and_then(|exception| exception.surface),
            ));
        }
        become_grammar::BecomeCopySourceShape::NotCopy => {}
    }

    if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Colorless) {
        return Ok(EffectAst::subject_verb_make_colorless(target, duration));
    }
    if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Saddled) {
        return Ok(EffectAst::subject_verb_become_saddled_until_end_of_turn(
            target,
        ));
    }
    if let Some(aura) = become_surface.aura {
        if become_grammar::aura_subject_prefers_source(target_subject_tokens)
            || matches!(&target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        {
            target = TargetAst::Source(span_from_tokens(subject_tokens));
        }
        let attachment_filter = if aura.attachment_you_control {
            ObjectFilter::creature().you_control()
        } else {
            ObjectFilter::creature()
        };
        let quote_indices = become_body_tokens
            .iter()
            .enumerate()
            .filter_map(|(idx, token)| (token.kind == TokenKind::Quote).then_some(idx))
            .collect::<Vec<_>>();
        let granted_abilities = if let [open_quote, close_quote, ..] = quote_indices.as_slice() {
            let ability_tokens = &become_body_tokens[open_quote + 1..*close_quote];
            let (abilities, is_choice) =
                parse_granted_abilities_for_gain_clause(ability_tokens, become_words, false)?;
            if is_choice {
                return Err(CardTextError::ParseError(format!(
                    "unsupported modal Aura grant (clause: '{}')",
                    render_lower_words(ability_tokens)
                )));
            }
            abilities
        } else {
            Vec::new()
        };
        return Ok(EffectAst::subject_verb_become_aura_enchantment_with_grants(
            target,
            attachment_filter,
            granted_abilities,
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

    if subject_targets_base_pt && become_words.len() == 3 && become_words[1] == "or"
        && let Ok((first_power, first_toughness)) = crate::keyword_static::parse_pt_modifier_values(become_words[0])
        && let Ok((second_power, second_toughness)) = crate::keyword_static::parse_pt_modifier_values(become_words[2])
    {
        return Ok(EffectAst::ObjectChoices(crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf {
            modes: vec![(first_power, first_toughness, become_words[0]), (second_power, second_toughness, become_words[2])]
                .into_iter().map(|(power, toughness, _label)| crate::cards::builders::ChooseOneModeAst {
                    description: String::new(),
                    effects: vec![EffectAst::subject_verb_set_base_power_toughness(power, toughness, target.clone(), duration.clone())],
                }).collect(),
        }));
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
            let prefix_words = &become_words[value_word_count..creature_idx];
            let add_supertypes = prefix_words.iter().filter_map(|word| crate::util::parse_supertype_word(word)).collect::<Vec<_>>();
            let descriptor_words = prefix_words.iter().copied().filter(|word| crate::util::parse_supertype_word(word).is_none()).collect::<Vec<_>>();
            let prefix = become_grammar::parse_become_leading_creature_prefix(&descriptor_words);
            let mut suffix_tokens = suffix_tokens;
            let mut remove_all_abilities = false;
            if let Some(index) = suffix_tokens.windows(4).position(|window|
                window[0].is_word("and") && window[1].is_any_word(&["lose", "loses"])
                    && window[2].is_word("all") && window[3].is_word("abilities"))
                && crate::util::trim_edge_punctuation_tokens(&suffix_tokens[index + 4..]).is_empty()
            {
                remove_all_abilities = true;
                suffix_tokens = &suffix_tokens[..index];
            }
            let name_override = if suffix_tokens.first().is_some_and(|token| token.is_word("named")) {
                let name = crate::lexer::parser_token_word_refs(&suffix_tokens[1..]).join(" ");
                if name.is_empty() { return Err(CardTextError::ParseError("missing transformation name".into())); }
                suffix_tokens = &[];
                Some(name)
            } else { None };
            let mut abilities = Vec::new();
            let mut granted_abilities = Vec::<GrantedAbilityAst>::new();
            let mut subtype_families = Vec::<SubtypeFamily>::new();
            let (suffix_supported, preserve_other_types, type_retention_surface) =
                match become_grammar::parse_become_animation_suffix_shape(suffix_tokens) {
                    become_grammar::BecomeAnimationSuffixShape::Ignored {
                        preserve_other_types,
                        type_retention_surface,
                    } => (true, preserve_other_types, type_retention_surface),
                    become_grammar::BecomeAnimationSuffixShape::Unsupported => (false, false, None),
                    become_grammar::BecomeAnimationSuffixShape::With {
                        ability_tokens,
                        grants_all_creature_types,
                        preserve_other_types,
                        type_retention_surface,
                    } => {
                        if grants_all_creature_types {
                            subtype_families.push(SubtypeFamily::Creature);
                        }
                        let suffix_words = crate::lexer::parser_token_word_refs(ability_tokens);
                        if ability_tokens.is_empty() {
                            (
                                grants_all_creature_types,
                                preserve_other_types,
                                type_retention_surface,
                            )
                        } else if let Ok((parsed_abilities, _)) =
                            parse_granted_abilities_for_gain_clause(
                                ability_tokens,
                                &suffix_words,
                                false,
                            )
                            && !parsed_abilities.is_empty()
                        {
                            granted_abilities = parsed_abilities;
                            (true, preserve_other_types, type_retention_surface)
                        } else {
                            (
                                parse_ability_line(ability_tokens)
                                    .map(|actions| {
                                        abilities = actions
                                            .into_iter()
                                            .filter_map(keyword_action_to_static_ability)
                                            .collect::<Vec<_>>();
                                        !abilities.is_empty()
                                    })
                                    .unwrap_or(false),
                                preserve_other_types,
                                type_retention_surface,
                            )
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
                    preserve_other_types,
                    type_retention_surface,
                    Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness),
                    animation_duration_surface,
                    duration,
                )
                .with_set_quantifier_surface(set_quantifier_surface));
            }
            let mut effect = EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                prefix.card_types,
                prefix.subtypes,
                subtype_families,
                prefix.colors,
                abilities,
                granted_abilities,
                preserve_other_types,
                type_retention_surface,
                Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness),
                animation_duration_surface,
                duration,
            )
            .with_set_quantifier_surface(set_quantifier_surface);
            if let EffectAst::SubjectVerb(subject) = &mut effect
                && let crate::cards::builders::SubjectVerbActionAst::Characteristics(
                    crate::cards::builders::CharacteristicActionAst::BecomeBasePtCreature {
                        name_override: name, add_supertypes: supertypes, remove_all_abilities: remove, ..
                    }) = &mut subject.action
            {
                *name = name_override; *supertypes = add_supertypes; *remove = remove_all_abilities;
            }
            return Ok(effect);
        }
        let (descriptor_words, preserve_other_types) =
            become_grammar::strip_become_addition_tail_words(&become_words[value_word_count..]);
        if preserve_other_types
            && let Some(descriptor) =
                become_grammar::parse_become_creature_descriptor_words(descriptor_words)
            && !descriptor.subtypes.is_empty()
        {
            return Ok(EffectAst::subject_verb_become_base_pt_creature(
                power,
                toughness,
                target,
                descriptor.card_types,
                descriptor.subtypes,
                Vec::new(),
                descriptor.colors,
                Vec::new(),
                Vec::new(),
                true,
                Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypesImplicitCreature),
                Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness),
                animation_duration_surface,
                duration,
            )
            .with_set_quantifier_surface(set_quantifier_surface));
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
            false,
            None,
            Some(ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness),
            animation_duration_surface,
            duration,
        )
        .with_set_quantifier_surface(set_quantifier_surface));
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
            false,
            None,
            Some(ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness),
            animation_duration_surface,
            duration,
        )
        .with_set_quantifier_surface(set_quantifier_surface));
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
        become_grammar::BecomeSimpleDescriptorShape::CardTypes {
            card_types,
            preserve_other_types,
        } => {
            return Ok(if preserve_other_types {
                EffectAst::subject_verb_add_card_types(target, card_types, duration)
            } else {
                EffectAst::subject_verb_set_card_types(target, card_types, duration)
            });
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

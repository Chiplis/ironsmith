use super::*;
use crate::cards::builders::ConditionalEffectAst;
use winnow::Parser;

pub fn parse_become_clause(
    subject_tokens: &[OwnedLexToken],
    rest_tokens: &[OwnedLexToken],
) -> Result<EffectAst, CardTextError> {
    // "It becomes night." / "It becomes day." (CR 731.2-731.3): the impersonal
    // "it" names the game's day/night designation, not an object.
    if let [subject] = LexedClause::new(subject_tokens).trim().as_slice()
        && subject.is_word("it")
    {
        let rest = crate::util::trim_edge_punctuation_tokens(rest_tokens);
        if let [designation] = rest {
            if designation.is_word("night") {
                return Ok(EffectAst::SetDayNight(ironsmith_core::DayNightDesignation::Night));
            }
            if designation.is_word("day") {
                return Ok(EffectAst::SetDayNight(ironsmith_core::DayNightDesignation::Day));
            }
        }
    }
    let mut designation_clause = subject_tokens.to_vec();
    designation_clause.push(OwnedLexToken::synthetic_word("become"));
    designation_clause.extend_from_slice(rest_tokens);
    if let Some(effect) = super::super::suspected::parse_clear_suspected_clause(&designation_clause)? {
        return Ok(effect);
    }
    let subject_tokens = LexedClause::new(subject_tokens).trim();
    let rest_clause = LexedClause::new(rest_tokens).trimmed();
    let rest_words = rest_clause.word_refs();
    // This packet admits the beginning-of-step lifetime only through the
    // typed basic-land conversion owner. Generic durations must not carry it into
    // prevention shields, permissions, grants, or other unrepresented owners.
    if let Some(parsed) = crate::grammar::leaf::parse_leaf_restriction_duration_suffix_tokens(rest_tokens)
        && parsed.duration == crate::grammar::leaf::LeafDurationPhrase::UntilControllersNextUntapStep
        && !trailing_duration_belongs_to_quoted_ability(rest_tokens, parsed.rest)
    {
        let mut effect = parse_become_clause(&subject_tokens, parsed.rest)?;
        let EffectAst::SubjectVerb(subject) = &mut effect else {
            return Err(CardTextError::ParseError("next-untap beginning requires one basic-land conversion".into()));
        };
        let duration = match &mut subject.action {
            crate::cards::builders::SubjectVerbActionAst::Characteristics(
                crate::cards::builders::CharacteristicActionAst::BecomeBasicLandType { duration, .. }
                | crate::cards::builders::CharacteristicActionAst::BecomeBasicLandTypeChoice { duration, .. }
            ) => duration,
            _ => return Err(CardTextError::ParseError("next-untap beginning is unsupported for this become owner".into())),
        };
        if !matches!(duration, Until::Forever) {
            return Err(CardTextError::ParseError("conflicting basic-land conversion durations".into()));
        }
        *duration = Until::UntilControllersNextUntapStep {
            object: ironsmith_core::ContinuousDurationObject::AffectedObject,
        };
        return Ok(effect);
    }
    if rest_words == ["blocked"] {
        let subject = parse_target_phrase(&subject_tokens).or_else(|_| {
            parse_object_filter_lexed(&subject_tokens, false)
                .map(|filter| TargetAst::Object(filter, None, None))
        })?;
        return Ok(EffectAst::subject_verb_become_blocked(subject));
    }

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
        } else if let Some((counter_type, body)) =
            crate::grammar::effects::parse_affected_object_counter_duration_suffix(
                &become_clause_tokens,
            )
            && !trailing_duration_belongs_to_quoted_ability(&become_clause_tokens, body)
        {
            // "It's a green Dinosaur with base power and toughness 5/5 for as
            // long as it has a saurian counter on it": the animation lasts
            // while the animated object keeps that counter.
            (
                Until::ForAsLongAs(
                    ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                        counter_type,
                    ),
                ),
                subject_tokens.clone(),
                body.to_vec(),
                None,
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
    let dynamic_base_subject = dynamic_base_values::subject(subject_tokens);
    let base_pt_subject = become_grammar::parse_base_power_toughness_subject_tokens(subject_tokens);
    let subject_targets_base_pt = base_pt_subject.is_some();
    let target_subject_tokens = dynamic_base_subject
        .as_ref()
        .map(|(_, target)| target.as_slice())
        .or_else(|| base_pt_subject.map(|shape| shape.target_tokens))
        .unwrap_or(subject_tokens);
    let set_quantifier_surface =
        become_grammar::become_subject_set_quantifier_surface(target_subject_tokens);
    let subject = parse_subject(subject_tokens);
    let become_surface = become_grammar::parse_become_body_surface_shape(&become_tokens);
    let become_body_tokens = become_surface.body_tokens;
    let mut become_words_vec = crate::lexer::parser_token_word_refs(become_body_tokens);
    // "becomes a creature in addition to its other types and has base power
    // and base toughness each equal to its mana value" (Zur, Eternal Schemer)
    // states the base P/T with "and has"; the body grammar reads "with".
    if let Some(index) = become_words_vec
        .windows(4)
        .position(|window| window == ["and", "has", "base", "power"])
        && !become_body_tokens.windows(5).any(|window| {
            window[0].is_comma()
                && window[1].is_word("and")
                && window[2].is_word("has")
                && window[3].is_word("base")
                && window[4].is_word("power")
        })
    {
        become_words_vec.remove(index + 1);
        become_words_vec[index] = "with";
    }
    let become_words = &become_words_vec[..];
    let preserve_other_colors = become_words.ends_with(&[
        "in", "addition", "to", "its", "other", "colors", "and", "types",
    ]) || become_words.ends_with(&[
        "in", "addition", "to", "their", "other", "colors", "and", "types",
    ]);

    if let Some(player) = extract_subject_player(Some(subject)) {
        if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Monarch) {
            return Ok(EffectAst::subject_verb_become_monarch(player));
        }
        if become_grammar::become_subject_has_life_total(subject_tokens) {
            let amount = parse_value(&become_tokens)
                .map(|(value, _)| value)
                .or_else(|| {
                    crate::effect_sentences::zone_counter_helpers::parse_starting_life_total_value(
                        &become_tokens,
                        player,
                    )
                })
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

    if let Some((colors, kind)) =
        crate::grammar::effects::characteristic_assertions::color_then_remove_card_type(
            become_body_tokens,
        )
    {
        // The first instruction owns the authored target and records its exact
        // affected objects. A source reference has no newly allocated result.
        let alias = if matches!(&target, TargetAst::Source(_))
            || matches!(&target, TargetAst::Object(filter, _, _) if filter.source)
        {
            target.clone()
        } else {
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), None)
        };
        return Ok(EffectAst::Sequence {
            effects: vec![
                EffectAst::subject_verb_set_colors(target, colors, duration.clone()),
                EffectAst::subject_verb_remove_card_types(alias, vec![kind], duration),
            ],
        });
    }

    if let Some(shape) = become_grammar::parse_basic_land_choice_template(become_words) {
        let mut effect = EffectAst::subject_verb_become_basic_land_type_choice(target, duration);
        if let EffectAst::SubjectVerb(subject) = &mut effect
            && let crate::cards::builders::SubjectVerbActionAst::Characteristics(
                crate::cards::builders::CharacteristicActionAst::BecomeBasicLandTypeChoice {
                    allowed_subtypes,
                    preserve_other_types,
                    ..
                },
            ) = &mut subject.action
        {
            *allowed_subtypes = shape.allowed_subtypes;
            *preserve_other_types = shape.preserve_other_types;
        }
        return Ok(effect);
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
    if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Plotted) {
        return Ok(EffectAst::subject_verb_become_plotted(target));
    }
    if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Prepared) {
        return Ok(EffectAst::subject_verb_prepare(target));
    }
    if become_surface.exact_kind == Some(become_grammar::BecomeExactKind::Unprepared) {
        return Ok(EffectAst::subject_verb_unprepare(target));
    }
    if let Some(aura) = become_surface.aura {
        if become_grammar::aura_subject_prefers_source(target_subject_tokens)
            || matches!(&target, TargetAst::Tagged(tag, _) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str())
        {
            target = TargetAst::Source(span_from_tokens(subject_tokens));
        }
        let enchant_words = crate::lexer::parser_token_word_refs(aura.enchant_filter_tokens);
        let attachment_filter = if enchant_words == ["creature"] {
            ObjectFilter::creature()
        } else if aura.attachment_you_control && enchant_words == ["creature", "you", "control"] {
            ObjectFilter::creature().you_control()
        } else {
            // "enchant creature put onto the battlefield with Necromancy"
            crate::object_filters::parse_object_filter(aura.enchant_filter_tokens, false)?
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

    if let Some((axes, _)) = dynamic_base_subject.as_ref()
        && let Some(effect) = dynamic_base_values::assignment(
            *axes,
            target.clone(),
            become_body_tokens,
            duration.clone(),
            set_quantifier_surface,
        )?
    {
        return Ok(effect);
    }

    if become_surface.equal_to_source_power_toughness {
        return Ok(EffectAst::subject_verb_set_base_power_toughness(
            Value::PowerOf(Box::new(ChooseSpec::Source)),
            Value::ToughnessOf(Box::new(ChooseSpec::Source)),
            target,
            duration,
        ));
    }

    if subject_targets_base_pt
        && become_words.len() == 3
        && become_words[1] == "or"
        && let Ok((first_power, first_toughness)) =
            crate::keyword_static::parse_pt_modifier_values(become_words[0])
        && let Ok((second_power, second_toughness)) =
            crate::keyword_static::parse_pt_modifier_values(become_words[2])
    {
        return Ok(EffectAst::ObjectChoices(
            crate::cards::builders::ObjectChoiceEffectAst::ChooseOneOf {
                chooser: crate::target::PlayerFilter::You,
                modes: vec![
                    (first_power, first_toughness, become_words[0]),
                    (second_power, second_toughness, become_words[2]),
                ]
                .into_iter()
                .map(
                    |(power, toughness, _label)| crate::cards::builders::ChooseOneModeAst {
                        description: String::new(),
                        effects: vec![EffectAst::subject_verb_set_base_power_toughness(
                            power,
                            toughness,
                            target.clone(),
                            duration.clone(),
                        )],
                    },
                )
                .collect(),
            },
        ));
    }

    if let Some(leading_pt) =
        become_grammar::parse_become_leading_pt_shape(become_words, become_body_tokens)
    {
        let become_grammar::BecomeLeadingPtShape {
            power,
            toughness,
            value_word_count,
            leading_supertypes,
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
            let mut add_supertypes = leading_supertypes;
            add_supertypes.extend(
                prefix_words
                    .iter()
                    .filter_map(|word| crate::util::parse_supertype_word(word))
                    .collect::<Vec<_>>(),
            );
            let descriptor_words = prefix_words
                .iter()
                .copied()
                .filter(|word| crate::util::parse_supertype_word(word).is_none())
                .collect::<Vec<_>>();
            let prefix = become_grammar::parse_become_leading_creature_prefix(&descriptor_words);
            // A sentence-final period ("it becomes a 4/4 Crocodile creature.",
            // Veiled Crocodile) is not an animation suffix.
            let mut suffix_tokens = suffix_tokens;
            while suffix_tokens.last().is_some_and(|token| token.is_period()) {
                suffix_tokens = &suffix_tokens[..suffix_tokens.len() - 1];
            }
            // "becomes a 4/4 Spirit artifact creature that's no longer an
            // Equipment" (Haunted Plate Mail): a trailing subtype removal.
            let mut removed_subtypes = Vec::new();
            {
                let positions = crate::lexer::parser_token_word_positions(suffix_tokens);
                let words = positions.iter().map(|(_, word)| *word).collect::<Vec<_>>();
                let head_len = match words.as_slice() {
                    ["that's" | "thats", "no", "longer", "a" | "an", ..] => Some(4),
                    ["that", "s" | "is", "no", "longer", "a" | "an", ..] => Some(5),
                    _ => None,
                };
                if let Some(head_len) = head_len
                    && words.len() > head_len
                    && let Some(subtypes) = words[head_len..]
                        .iter()
                        .filter(|word| !matches!(**word, "or" | "and" | "a" | "an"))
                        .map(|word| crate::util::parse_subtype_flexible(word))
                        .collect::<Option<Vec<_>>>()
                    && !subtypes.is_empty()
                {
                    removed_subtypes = subtypes;
                    suffix_tokens = &[];
                }
            }
            let removal_target = target.clone();
            let removal_duration = duration.clone();
            let with_subtype_removal = |effect: EffectAst| -> EffectAst {
                if removed_subtypes.is_empty() {
                    effect
                } else {
                    EffectAst::Sequence {
                        effects: vec![
                            effect,
                            EffectAst::subject_verb_remove_subtypes(
                                removal_target.clone(),
                                removed_subtypes.clone(),
                                removal_duration.clone(),
                            ),
                        ],
                    }
                }
            };
            let mut remove_all_abilities = false;
            if let Some(index) = suffix_tokens.windows(4).position(|window| {
                window[0].is_word("and")
                    && window[1].is_any_word(&["lose", "loses"])
                    && window[2].is_word("all")
                    && window[3].is_word("abilities")
            }) && crate::util::trim_edge_punctuation_tokens(&suffix_tokens[index + 4..])
                .is_empty()
            {
                remove_all_abilities = true;
                suffix_tokens = &suffix_tokens[..index];
            }
            let mut quote_depth = false;
            let name_start = suffix_tokens.iter().enumerate().find_map(|(index, token)| {
                if token.kind == TokenKind::Quote {
                    quote_depth = !quote_depth;
                }
                (!quote_depth && token.is_word("named")).then_some(index)
            });
            let name_override = if let Some(index) = name_start {
                let name_tokens =
                    crate::util::trim_edge_punctuation_tokens(&suffix_tokens[index + 1..]);
                let name = crate::lexer::render_literal_token_slice(name_tokens)
                    .trim()
                    .to_string();
                if name.is_empty() {
                    return Err(CardTextError::ParseError(
                        "missing transformation name".into(),
                    ));
                }
                suffix_tokens = crate::util::trim_edge_punctuation_tokens(&suffix_tokens[..index]);
                Some(name)
            } else {
                None
            };
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
                        } else if let Ok((parsed_abilities, is_choice)) =
                            parse_granted_abilities_for_gain_clause(
                                ability_tokens,
                                &suffix_words,
                                false,
                            )
                            && !is_choice
                            && !parsed_abilities.is_empty()
                        {
                            granted_abilities = parsed_abilities;
                            (true, preserve_other_types, type_retention_surface)
                        } else {
                            (
                                parse_ability_line(ability_tokens)
                                    .map(|actions| {
                                        let expected = actions.len();
                                        abilities = actions
                                            .into_iter()
                                            .filter_map(keyword_action_to_static_ability)
                                            .collect::<Vec<_>>();
                                        !abilities.is_empty() && abilities.len() == expected
                                    })
                                    .unwrap_or(false),
                                preserve_other_types,
                                type_retention_surface,
                            )
                        }
                    }
                };
            if !prefix.supported || !suffix_supported {
                return Err(CardTextError::ParseError(format!(
                    "unsupported complete animation descriptor (clause: '{}')",
                    render_lower_words(&rest_tokens)
                )));
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
            .with_set_quantifier_surface(set_quantifier_surface)
            .with_animation_color_retention(preserve_other_colors);
            if let EffectAst::SubjectVerb(subject) = &mut effect
                && let crate::cards::builders::SubjectVerbActionAst::Characteristics(
                    crate::cards::builders::CharacteristicActionAst::BecomeBasePtCreature {
                        name_override: name,
                        add_supertypes: supertypes,
                        remove_all_abilities: remove,
                        ..
                    },
                ) = &mut subject.action
            {
                *name = name_override;
                *supertypes = add_supertypes;
                *remove = remove_all_abilities;
            }
            return Ok(with_subtype_removal(effect));
        }
        let (mut descriptor_words, mut preserve_other_types) =
            become_grammar::strip_become_addition_tail_words(&become_words[value_word_count..]);
        // "Each of them is a 1/1 Spirit with flying in addition to its other
        // types" (Storm of Souls): keywords granted after the implied-creature
        // subtype (CR 205.3m; the abilities are added in layer 6).
        let mut implied_creature_grants = Vec::<GrantedAbilityAst>::new();
        if let Some(with_word) = descriptor_words
            .iter()
            .position(|word| *word == "with")
            .filter(|index| *index > 0)
        {
            let Some(with_token) = become_body_tokens.iter().position(|token| token.is_word("with"))
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported implied-creature animation suffix (clause: '{}')",
                    render_lower_words(&rest_tokens)
                )));
            };
            let become_grammar::BecomeAnimationSuffixShape::With {
                ability_tokens,
                grants_all_creature_types: false,
                preserve_other_types: suffix_preserves,
                ..
            } = become_grammar::parse_become_animation_suffix_shape(
                &become_body_tokens[with_token..],
            )
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported implied-creature animation suffix (clause: '{}')",
                    render_lower_words(&rest_tokens)
                )));
            };
            let ability_words = crate::lexer::parser_token_word_refs(ability_tokens);
            let (parsed, is_choice) =
                parse_granted_abilities_for_gain_clause(ability_tokens, &ability_words, false)?;
            if is_choice || parsed.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported implied-creature animation abilities (clause: '{}')",
                    render_lower_words(&rest_tokens)
                )));
            }
            implied_creature_grants = parsed;
            descriptor_words = &descriptor_words[..with_word];
            preserve_other_types = preserve_other_types || suffix_preserves;
        }
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
                implied_creature_grants,
                true,
                Some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypesImplicitCreature),
                Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness),
                animation_duration_surface,
                duration,
            )
            .with_set_quantifier_surface(set_quantifier_surface)
            .with_animation_color_retention(preserve_other_colors));
        }
    }

    // "becomes a Kithkin Spirit Warrior Avatar with base power and toughness
    // 8/8, flying, and first strike" (Figure of Destiny): keyword abilities
    // listed after the base P/T are gained along with it.
    let (base_pt_words, outer_retains_types) =
        become_grammar::strip_become_addition_tail_words(become_words);
    let positions = crate::lexer::parser_token_word_positions(become_body_tokens);
    let base_pt_tokens = if base_pt_words.len() < positions.len() {
        crate::util::trim_edge_punctuation_tokens(
            &become_body_tokens[..positions[base_pt_words.len()].0],
        )
    } else {
        become_body_tokens
    };
    if let Some(effect) = dynamic_base_values::animation_with_preceding_grants(
        target.clone(),
        base_pt_tokens,
        duration.clone(),
        animation_duration_surface,
        set_quantifier_surface,
        outer_retains_types,
        preserve_other_colors,
    )? {
        return Ok(effect);
    }
    let base_pt_with_abilities = become_grammar::parse_become_base_pt_words(base_pt_words)
        .map(|pt| (pt, Vec::new()))
        .or_else(|| {
            let positions = crate::lexer::parser_token_word_positions(base_pt_tokens);
            if positions.len() != base_pt_words.len() {
                return None;
            }
            (1..base_pt_words.len()).rev().find_map(|split| {
                let pt = become_grammar::parse_become_base_pt_words(&base_pt_words[..split])?;
                let ability_tokens = &base_pt_tokens[positions[split].0..];
                let ability_tokens = crate::util::trim_edge_punctuation_tokens(ability_tokens);
                let ability_tokens = if ability_tokens
                    .first()
                    .is_some_and(|token| token.is_word("and"))
                {
                    crate::util::trim_edge_punctuation_tokens(&ability_tokens[1..])
                } else {
                    ability_tokens
                };
                let words = crate::lexer::parser_token_word_refs(ability_tokens);
                let (abilities, choice) =
                    parse_granted_abilities_for_gain_clause(ability_tokens, &words, false).ok()?;
                (!choice && !abilities.is_empty()).then_some((pt, abilities))
            })
        });
    if let Some((pt, trailing_abilities)) = base_pt_with_abilities
        && let (descriptor_words, preserve_other_types) =
            become_grammar::strip_become_addition_tail_words(pt.descriptor_words)
        && let Some(descriptor) =
            become_grammar::parse_become_creature_descriptor_words(descriptor_words)
    {
        // "becomes a creature in addition to its other types and has base
        // power and base toughness each equal to its mana value" (Zur,
        // Eternal Schemer) keeps the object's other types.
        let preserve_other_types = preserve_other_types || outer_retains_types;
        let explicit_creature_noun = descriptor.subtypes.is_empty();
        // "becomes a green Wurm with base power and toughness 6/4" names no
        // card type: the creature type is implied, and the object's own card
        // types are untouched. An empty list asks lowering for exactly that.
        let authored_card_type = descriptor_words
            .iter()
            .any(|word| crate::util::parse_card_type(word).is_some());
        let card_types = if authored_card_type || preserve_other_types {
            descriptor.card_types
        } else {
            Vec::new()
        };
        return Ok(EffectAst::subject_verb_become_base_pt_creature(
            pt.power,
            pt.toughness,
            target,
            card_types,
            descriptor.subtypes,
            Vec::new(),
            descriptor.colors,
            Vec::new(),
            trailing_abilities,
            preserve_other_types,
            preserve_other_types.then_some(if explicit_creature_noun {
                ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes
            } else {
                ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypesImplicitCreature
            }),
            Some(ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness),
            animation_duration_surface,
            duration,
        )
        .with_set_quantifier_surface(set_quantifier_surface)
        .with_animation_color_retention(preserve_other_colors));
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
        .with_set_quantifier_surface(set_quantifier_surface)
        .with_animation_color_retention(preserve_other_colors));
    }

    if let Some(shape) = become_grammar::parse_object_template_tokens(become_body_tokens) {
        let words = crate::lexer::parser_token_word_refs(shape.ability_tokens);
        let (grants, choice) = if shape.ability_tokens.is_empty() {
            (Vec::new(), false)
        } else {
            parse_granted_abilities_for_gain_clause(shape.ability_tokens, &words, false)?
        };
        if choice || (!shape.ability_tokens.is_empty() && grants.is_empty()) {
            return Err(CardTextError::ParseError(
                "unsupported complete object-template grant".into(),
            ));
        }
        let mut effect = EffectAst::subject_verb_become_object_template(
            shape.base_power_toughness.clone(),
            target,
            shape.card_types,
            shape.subtypes,
            Vec::new(),
            shape.colors,
            Vec::new(),
            grants,
            shape.preserve_other_types,
            shape
                .preserve_other_types
                .then_some(ironsmith_core::TypeRetentionSurface::InAdditionToOtherTypes),
            shape
                .base_power_toughness
                .is_some()
                .then_some(ironsmith_core::AnimationPtSurface::ExplicitBasePowerToughness),
            animation_duration_surface,
            duration,
        )
        .with_set_quantifier_surface(set_quantifier_surface)
        .with_animation_color_retention(shape.preserve_other_colors);
        if let EffectAst::SubjectVerb(subject) = &mut effect
            && let crate::cards::builders::SubjectVerbActionAst::Characteristics(
                crate::cards::builders::CharacteristicActionAst::BecomeBasePtCreature {
                    add_supertypes,
                    remove_other_abilities,
                    name_override,
                    ..
                },
            ) = &mut subject.action
        {
            *add_supertypes = shape.supertypes;
            *remove_other_abilities = shape.remove_other_abilities;
            *name_override = shape.name_override;
        }
        return Ok(effect);
    }

    // A creature conversion can leave power/toughness unstated, for example
    // when a separately granted ability supplies its dynamic characteristics.
    // Preserve that absence rather than inventing a fixed base size.
    let (descriptor_words, preserve_other_types) =
        become_grammar::strip_become_addition_tail_words(become_words);
    if descriptor_words.contains(&"creature")
        && let Some(descriptor) =
            become_grammar::parse_become_creature_descriptor_words(descriptor_words)
        && !descriptor.subtypes.is_empty()
    {
        let mut effects = vec![if preserve_other_types {
            EffectAst::subject_verb_add_card_types(
                target.clone(),
                descriptor.card_types,
                duration.clone(),
            )
        } else {
            EffectAst::subject_verb_set_card_types(
                target.clone(),
                descriptor.card_types,
                duration.clone(),
            )
        }];
        effects.push(if preserve_other_types {
            EffectAst::subject_verb_add_subtypes(
                target.clone(),
                descriptor.subtypes,
                duration.clone(),
            )
        } else {
            EffectAst::subject_verb_set_creature_subtypes(
                target.clone(),
                descriptor.subtypes,
                duration.clone(),
            )
        });
        if let Some(colors) = descriptor.colors {
            effects.push(if preserve_other_colors {
                EffectAst::subject_verb_add_colors(target, colors, duration)
            } else {
                EffectAst::subject_verb_set_colors(target, colors, duration)
            });
        }
        return Ok(EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        });
    }

    if let Some(shape) = become_grammar::parse_become_mixed_characteristics(become_words) {
        let implicit_artifact_creature = shape.card_types.contains(&CardType::Artifact)
            && shape.card_types.contains(&CardType::Creature);
        let retains_types = shape.preserve_other_types || implicit_artifact_creature;
        let mut effects = vec![if shape.preserve_other_colors {
            EffectAst::subject_verb_add_colors(target.clone(), shape.colors, duration.clone())
        } else {
            EffectAst::subject_verb_set_colors(target.clone(), shape.colors, duration.clone())
        }];
        effects.push(if retains_types {
            EffectAst::subject_verb_add_card_types(
                target.clone(),
                shape.card_types,
                duration.clone(),
            )
        } else {
            EffectAst::subject_verb_set_card_types(
                target.clone(),
                shape.card_types,
                duration.clone(),
            )
        });
        if !shape.subtypes.is_empty() {
            effects.push(if shape.preserve_other_types {
                EffectAst::subject_verb_add_subtypes(
                    target.clone(),
                    shape.subtypes,
                    duration.clone(),
                )
            } else {
                EffectAst::subject_verb_set_creature_subtypes(
                    target.clone(),
                    shape.subtypes,
                    duration.clone(),
                )
            });
        }
        return Ok(EffectAst::Coordinated {
            effects,
            leading_duration: false,
            result_conjunction: false,
        });
    }

    match become_grammar::parse_become_simple_descriptor_words(become_words) {
        become_grammar::BecomeSimpleDescriptorShape::ColorsAndSubtypes { colors, subtypes } => {
            let (_, retains_types) = become_grammar::strip_become_addition_tail_words(become_words);
            return Ok(EffectAst::Coordinated {
                effects: vec![
                    if preserve_other_colors {
                        EffectAst::subject_verb_add_colors(target.clone(), colors, duration.clone())
                    } else {
                        EffectAst::subject_verb_set_colors(target.clone(), colors, duration.clone())
                    },
                    if retains_types {
                        EffectAst::subject_verb_add_subtypes(target, subtypes, duration)
                    } else {
                        EffectAst::subject_verb_set_creature_subtypes(target, subtypes, duration)
                    },
                ],
                leading_duration: false,
                result_conjunction: false,
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

    for tail in [
        &["in", "addition", "to", "its", "other", "colors"][..],
        &["in", "addition", "to", "their", "other", "colors"][..],
    ] {
        if let Some(words) = become_words.strip_suffix(tail)
            && let Some(colors) = become_grammar::parse_become_color_words(words)
        {
            return Ok(EffectAst::subject_verb_add_colors(target, colors, duration));
        }
    }

    if let Some(colors) = become_grammar::parse_become_color_words(become_words) {
        return Ok(EffectAst::subject_verb_set_colors(target, colors, duration));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported become clause (clause: '{}')",
        render_lower_words(&rest_tokens)
    )))
}

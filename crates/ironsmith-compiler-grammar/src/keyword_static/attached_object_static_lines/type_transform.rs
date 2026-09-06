use super::*;

pub fn parse_attached_type_transform_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let Some(parsed) = attached_grammar::parse_attached_transform_tokens(tokens) else {
        return Ok(None);
    };
    let line_text = crate::lexer::render_token_slice(tokens);
    let subject_text = parsed.subject.display();
    let filter = parse_object_filter(parsed.subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported attached transform subject (clause: '{}')",
            line_text
        ))
    })?;
    let descriptor_words = crate::lexer::token_word_refs(parsed.descriptor_tokens);
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_card_types = Vec::new();
    let mut add_subtypes = Vec::new();
    let mut set_colors = ColorSet::new();
    let mut make_colorless = false;
    let mut descriptor_sets_land_type = false;
    for word in descriptor_words {
        match word {
            "and" => continue,
            "colorless" => {
                make_colorless = true;
                continue;
            }
            _ => {}
        }
        if let Some(color) = parse_color(word) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            descriptor_sets_land_type |= card_type == CardType::Land;
            crate::slice_primitives::push_unique(&mut set_card_types, card_type);
            continue;
        }
        if let Some(subtype) = parse_subtype_word(word).or_else(|| {
            crate::string_primitives::strip_suffix_char(word, 's').and_then(parse_subtype_word)
        }) {
            crate::slice_primitives::push_unique(&mut add_subtypes, subtype);
            continue;
        }
        // A descriptor outside this type/color grammar belongs to another
        // static reading (for example, a supertype plus a modifier).
        return Ok(None);
    }

    let mut out = Vec::new();
    let mut preserve_other_types = false;
    let mut loss_consumed = false;

    if let Some(ability_tokens) = parsed.ability_tokens {
        let ability_tokens = trim_commas(ability_tokens);
        if ability_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing attached transform granted ability (clause: '{}')",
                line_text
            )));
        }

        if let Some(split) =
            attached_grammar::split_attached_base_pt_keyword_tokens(&ability_tokens)
        {
            let Some((power, toughness, with_preserve_other_types)) =
                parse_attached_with_base_power_toughness_clause(split.base_tokens)?
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attached transform granted ability (clause: '{}')",
                    line_text
                )));
            };
            preserve_other_types = with_preserve_other_types;
            out.push(
                StaticAbility::set_base_power_toughness(filter.clone(), power, toughness).into(),
            );

            if parsed.loss == Some(attached_grammar::AttachedTransformLossKind::AllAbilities) {
                out.push(StaticAbility::remove_all_abilities(filter.clone()).into());
                loss_consumed = true;
            }

            let Some(actions) = parse_ability_line(split.keyword_tokens) else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported attached transform granted ability (clause: '{}')",
                    line_text
                )));
            };
            for action in actions {
                reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &line_text)?;
                if !action.lowers_to_static_ability() {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported attached transform granted ability (clause: '{}')",
                        line_text
                    )));
                }
                out.push(StaticAbilityAst::AttachedKeywordActionGrant {
                    display: format!(
                        "{subject_text} has {}",
                        action.display_text().to_ascii_lowercase()
                    ),
                    action,
                    condition: None,
                    protection_does_not_remove_controlled_attachments: false,
                });
            }
        } else if let Some((power, toughness, with_preserve_other_types)) =
            parse_attached_with_base_power_toughness_clause(&ability_tokens)?
        {
            preserve_other_types = with_preserve_other_types;
            out.push(
                StaticAbility::set_base_power_toughness(filter.clone(), power, toughness).into(),
            );
        } else if let Some(parsed) = parse_attached_granted_activated_line(&ability_tokens)? {
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed,
                display: format!(
                    "{subject_text} has {}",
                    display_text_for_tokens(&ability_tokens, true)
                ),
                condition: None,
            });
        } else if let Some((parsed, display)) =
            parse_attached_nonstatic_keyword_ability(&ability_tokens)?
        {
            out.push(StaticAbilityAst::AttachedObjectAbilityGrant {
                ability: parsed,
                display: format!("{subject_text} has {display}"),
                condition: None,
            });
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached transform granted ability (clause: '{}')",
                line_text
            )));
        }
    }

    let descriptor_has_card_types = !set_card_types.is_empty();
    if descriptor_has_card_types {
        if preserve_other_types {
            out.push(StaticAbility::add_card_types(filter.clone(), set_card_types).into());
        } else {
            out.push(
                StaticAbility::set_card_types_with_surface(
                    filter.clone(),
                    set_card_types,
                    line_text.clone(),
                )
                .into(),
            );
        }
    }
    if !add_subtypes.is_empty() {
        if !preserve_other_types
            && add_subtypes
                .iter()
                .all(crate::types::Subtype::is_land_subtype)
            && (descriptor_sets_land_type
                || parsed.subject == attached_grammar::AttachedSubject::EnchantedLand)
        {
            out.push(StaticAbility::set_land_subtypes(filter.clone(), add_subtypes).into());
        } else if !preserve_other_types
            && !descriptor_has_card_types
            && add_subtypes
                .iter()
                .all(crate::types::Subtype::is_creature_type)
            && parsed.subject == attached_grammar::AttachedSubject::EnchantedCreature
        {
            out.push(StaticAbility::set_creature_subtypes(filter.clone(), add_subtypes).into());
        } else {
            out.push(StaticAbility::add_subtypes(filter.clone(), add_subtypes).into());
        }
    }
    if !set_colors.is_empty() {
        out.push(StaticAbility::set_colors(filter.clone(), set_colors).into());
    }
    if make_colorless {
        out.push(StaticAbility::make_colorless(filter.clone()).into());
    }

    if parsed.loss == Some(attached_grammar::AttachedTransformLossKind::AllAbilities)
        && !loss_consumed
    {
        out.push(StaticAbility::remove_all_abilities(filter.clone()).into());
    }

    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

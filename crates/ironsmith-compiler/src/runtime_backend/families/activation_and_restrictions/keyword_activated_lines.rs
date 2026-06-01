use super::*;

const CHANNEL_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["channel"]);
const CRAFT_WITH_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["craft", "with"]);
const PAY_LIFE_COST_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["pay"]; suffix & ["life"]);
const LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["life"]);
const CYCLING_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["cycling"]);
const LANDCYCLING_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["landcycling"]);
const EQUIP_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["equip"]);
const EQUIP_SUBTYPE_CONNECTOR_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["or"], &["and"], &["and/or"]]);
const CREATURE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["creature"]);
const RECONFIGURE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["reconfigure"]);
const CRAFT_ARTIFACT_MATERIAL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["artifact"]);
const CRAFT_CREATURE_MATERIAL_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["creature"]);
const CRAFT_ONE_OR_MORE_MATERIAL_PATTERN: ClauseShape<'static> =
    clause_shape!(exact & ["one", "or", "more"]);
const CRAFT_RED_INSTANT_SORCERY_MATERIAL_TAIL_PATTERN: ClauseShape<'static> = clause_shape!(
    exact_any
        & [
            &["red", "instant", "and", "or", "sorcery", "cards"],
            &["red", "instant", "and/or", "sorcery", "cards"],
            &["red", "instant", "or", "sorcery", "cards"],
        ]
);
const DISCARD_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["discard"]);

pub(crate) fn parse_cycling_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_cycling_line_lexed(tokens)
}

pub(crate) fn parse_cycling_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let Some(cycling_idx) = find_cycling_keyword_word_index(&words) else {
        return Ok(None);
    };
    if contains_granted_keyword_before_word(&words, cycling_idx) {
        return Ok(None);
    }

    let clause_text = joined_activation_clause_text(tokens);
    let cycling_groups = parse_cycling_keyword_cost_groups(tokens);
    let Some((first_keyword_tokens, first_cost_tokens)) = cycling_groups.first() else {
        return Ok(None);
    };
    if first_cost_tokens.is_empty() {
        return Ok(None);
    }

    let base_cost_words = crate::runtime_backend::token_word_refs(first_cost_tokens);
    if cycling_groups.iter().skip(1).any(|(_, cost_tokens)| {
        crate::runtime_backend::token_word_refs(cost_tokens) != base_cost_words
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mixed cycling costs (clause: '{clause_text}')",
        )));
    }

    let base_cost = parse_activation_cost(first_cost_tokens)?;
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::discard_source());
    merged_costs.push(
        crate::costs::payment_effect_to_cost(Effect::emit_keyword_action(
            crate::events::KeywordActionKind::Cycle,
            1,
        ))
        .map_err(CardTextError::ParseError)?,
    );
    let mana_cost = crate::cost::TotalCost::from_costs(merged_costs);

    let mut search_filter = parse_cycling_search_filter(first_keyword_tokens)?;
    for (keyword_tokens, _) in cycling_groups.iter().skip(1) {
        let next_filter = parse_cycling_search_filter(keyword_tokens)?;
        match (&mut search_filter, next_filter) {
            (Some(current), Some(next)) => merge_cycling_search_filters(current, &next),
            (None, None) => {}
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported mixed cycling variants (clause: '{clause_text}')",
                )));
            }
        }
    }
    let effect = if let Some(filter) = search_filter {
        Effect::search_library_to_hand(filter, true)
    } else {
        Effect::draw(1)
    };

    let cost_text = base_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| base_cost_words.join(" "));
    let render_text = if let Some(group) = parse_cycling_keyword_group_text(tokens) {
        group
    } else if crate::runtime_backend::token_word_refs(first_keyword_tokens).is_empty() {
        cost_text
    } else {
        format!(
            "{} {cost_text}",
            crate::runtime_backend::token_word_refs(first_keyword_tokens).join(" ")
        )
    };

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![effect]),
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Hand],
        }
        .into(),
        text: Some(render_text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub(crate) fn parse_channel_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_channel_line_lexed(tokens)
}

pub(crate) fn parse_channel_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = ActivationRestrictionCompatWords::new(tokens);
    if words
        .first()
        .is_none_or(|word| !CHANNEL_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let clause_text = joined_activation_clause_text(tokens);
    parse_hand_keyword_activated_body_lexed(&tokens[1..], "channel", "Channel", &clause_text)
}

pub(crate) fn parse_craft_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_craft_line_lexed(tokens)
}

pub(crate) fn parse_craft_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let reminder_start = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::LParen))
        .unwrap_or(tokens.len());
    let tokens = &tokens[..reminder_start];
    let words = ActivationRestrictionCompatWords::new(tokens);
    let words = words.to_word_refs();
    if !CRAFT_WITH_PREFIX_PATTERN.matches_words(&words) {
        return Ok(None);
    }

    let Some(cost_start) = crate::slice_primitives::find_index(&tokens[2..], |token| {
        mana_pips_from_token(token).is_some()
    })
    .map(|idx| idx + 2) else {
        return Ok(None);
    };
    let material_tokens = trim_edge_punctuation(&tokens[2..cost_start]);
    let cost_tokens = trim_edge_punctuation(&tokens[cost_start..]);
    if material_tokens.is_empty() || cost_tokens.is_empty() {
        return Ok(None);
    }

    let (material_filter, material_count, material_text) =
        parse_craft_material_spec(&material_tokens)?;
    let base_cost = parse_activation_cost(&cost_tokens)?;
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::costs::Cost::validated_effect(Effect::exile(
        ChooseSpec::Object(material_filter).with_count(material_count),
    )));
    merged_costs.push(
        crate::costs::payment_effect_to_cost(Effect::emit_keyword_action(
            crate::events::KeywordActionKind::Craft,
            1,
        ))
        .map_err(CardTextError::ParseError)?,
    );
    merged_costs.push(crate::costs::Cost::exile_self());

    let return_transformed = Effect::new(
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Source, Zone::Battlefield, false)
            .under_owner_control()
            .transfer_exiled_with_source_links(),
    );
    let transform = Effect::transform(ChooseSpec::Source);

    let cost_text = base_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| crate::runtime_backend::token_word_refs(&cost_tokens).join(" "));

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: crate::cost::TotalCost::from_costs(merged_costs),
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    return_transformed,
                    transform,
                ]),
                choices: Vec::new(),
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some(format!("Craft with {material_text} {cost_text}")),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

fn parse_craft_material_spec(
    tokens: &[OwnedLexToken],
) -> Result<(ObjectFilter, ChoiceCount, String), CardTextError> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if CRAFT_ARTIFACT_MATERIAL_PATTERN.matches_words(&words) {
        return Ok((
            craft_battlefield_or_graveyard_filter(CardType::Artifact),
            ChoiceCount::exactly(1),
            "artifact".to_string(),
        ));
    }
    if CRAFT_CREATURE_MATERIAL_PATTERN.matches_words(&words) {
        return Ok((
            craft_creature_battlefield_or_graveyard_filter(),
            ChoiceCount::exactly(1),
            "creature".to_string(),
        ));
    }
    if CRAFT_ONE_OR_MORE_MATERIAL_PATTERN.matches_words(&words) {
        return Ok((
            craft_any_battlefield_or_graveyard_filter(),
            ChoiceCount::at_least(1),
            "one or more".to_string(),
        ));
    }
    if let Some((count, used)) =
        parse_greater_than_or_equal_quantity_prefix(tokens, false, false, "craft material")?
        && CRAFT_RED_INSTANT_SORCERY_MATERIAL_TAIL_PATTERN.matches_words(&words[used..])
    {
        return Ok((
            craft_red_instant_or_sorcery_graveyard_filter(),
            ChoiceCount::at_least(count as usize),
            words.join(" "),
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported craft material clause '{}'",
        words.join(" ")
    )))
}

fn craft_battlefield_or_graveyard_filter(card_type: CardType) -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![
        ObjectFilter::default()
            .with_type(card_type)
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You)
            .other(),
        ObjectFilter::default()
            .with_type(card_type)
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .other(),
    ];
    filter
}

fn craft_any_battlefield_or_graveyard_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![
        ObjectFilter::permanent()
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You)
            .other(),
        ObjectFilter::default()
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You)
            .other(),
    ];
    filter
}

fn craft_red_instant_or_sorcery_graveyard_filter() -> ObjectFilter {
    ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .with_colors(ColorSet::from_color(crate::color::Color::Red))
        .with_type(CardType::Instant)
        .with_type(CardType::Sorcery)
}

fn craft_creature_battlefield_or_graveyard_filter() -> ObjectFilter {
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![
        ObjectFilter::default()
            .with_type(CardType::Creature)
            .in_zone(Zone::Battlefield)
            .controlled_by(PlayerFilter::You),
        ObjectFilter::default()
            .with_type(CardType::Creature)
            .in_zone(Zone::Graveyard)
            .owned_by(PlayerFilter::You),
    ];
    filter
}

pub(crate) fn parse_cycling_keyword_cost_groups(
    tokens: &[OwnedLexToken],
) -> Vec<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let mut groups = Vec::new();
    let mut idx = 0usize;

    while idx < tokens.len() {
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_comma() || token.is_semicolon())
        {
            idx += 1;
            continue;
        }

        let keyword_start = idx;
        let mut keyword_end: Option<usize> = None;
        while idx < tokens.len() {
            let Some(word) = tokens[idx].as_word().map(|word| word.to_ascii_lowercase()) else {
                break;
            };
            if word_is_cycling_keyword_marker(word.as_str()) {
                keyword_end = Some(idx);
                idx += 1;
                break;
            }
            idx += 1;
        }
        let Some(keyword_end) = keyword_end else {
            break;
        };

        let cost_start = idx;
        if token_slice_at_is(tokens, idx, "pay") {
            while idx < tokens.len() {
                let Some(word) = tokens[idx].as_word().map(|word| word.to_ascii_lowercase()) else {
                    break;
                };
                idx += 1;
                if LIFE_WORD_PATTERN.matches_word(word.as_str()) {
                    break;
                }
            }
        } else {
            while idx < tokens.len() {
                let lower_word = tokens[idx].as_word().map(|word| word.to_ascii_lowercase());
                let looks_like_reminder_cost = idx > cost_start
                    && lower_word
                        .as_deref()
                        .is_some_and(|word| word.chars().all(|ch| ch.is_ascii_digit()))
                    && tokens.get(idx + 1).is_some_and(|token| token.is_comma())
                    && tokens
                        .get(idx + 2)
                        .and_then(OwnedLexToken::as_word)
                        .is_some_and(|next| DISCARD_WORD_PATTERN.matches_word(next));
                let is_cost_token = mana_pips_from_token(&tokens[idx]).is_some()
                    || lower_word.as_deref().is_some_and(is_cycling_cost_word);
                if looks_like_reminder_cost || !is_cost_token {
                    break;
                }
                idx += 1;
            }
        }
        if idx == cost_start {
            break;
        }

        groups.push((
            tokens[keyword_start..=keyword_end].to_vec(),
            tokens[cost_start..idx].to_vec(),
        ));

        if tokens.get(idx).is_some_and(|token| token.is_comma()) {
            idx += 1;
            continue;
        }
        break;
    }

    groups
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    crate::slice_primitives::push_unique(items, item);
}

pub(crate) fn merge_cycling_search_filters(base: &mut ObjectFilter, extra: &ObjectFilter) {
    for supertype in &extra.supertypes {
        push_unique(&mut base.supertypes, *supertype);
    }
    for card_type in &extra.card_types {
        push_unique(&mut base.card_types, *card_type);
    }
    for subtype in &extra.subtypes {
        push_unique(&mut base.subtypes, *subtype);
    }
    if let Some(colors) = extra.colors {
        base.colors = Some(
            base.colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
}

pub(crate) fn parse_cycling_keyword_group_text(tokens: &[OwnedLexToken]) -> Option<String> {
    let groups = parse_cycling_keyword_cost_groups(tokens);
    if groups.is_empty() {
        return None;
    }

    let mut parts = Vec::new();
    for (keyword_tokens, cost_tokens) in groups {
        let keyword = crate::runtime_backend::token_word_refs(&keyword_tokens).join(" ");
        if keyword.is_empty() {
            continue;
        }
        let cost_words = crate::runtime_backend::token_word_refs(&cost_tokens);
        let cost = if cost_words.len() >= 3 && PAY_LIFE_COST_PATTERN.matches_words(&cost_words) {
            format!("pay {} life", cost_words[1])
        } else {
            parse_activation_cost(&cost_tokens)
                .ok()
                .and_then(|total_cost| total_cost.mana_cost().map(|cost| cost.to_oracle()))
                .unwrap_or_else(|| cost_words.join(" "))
        };
        parts.push(format!("{keyword} {cost}"));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

pub(crate) fn is_cycling_cost_word(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(
                    ch,
                    '{' | '}' | '/' | 'w' | 'u' | 'b' | 'r' | 'g' | 'c' | 'x'
                )
        })
}

pub(crate) fn parse_cycling_search_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let word_view = ActivationRestrictionCompatWords::new(tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return Ok(None);
    }

    let keyword = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("missing cycling keyword".to_string()))?;
    let mut filter = ObjectFilter::default();

    for word in &words[..words.len().saturating_sub(1)] {
        if let Some(supertype) = parse_supertype_word(word) {
            push_unique(&mut filter.supertypes, supertype);
        }
        if let Some(card_type) = parse_card_type(word) {
            push_unique(&mut filter.card_types, card_type);
        }
        if let Some(subtype) = parse_subtype_flexible(word) {
            push_unique(&mut filter.subtypes, subtype);
            if is_land_subtype(subtype) {
                push_unique(&mut filter.card_types, CardType::Land);
            }
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
    }

    if CYCLING_WORD_PATTERN.matches_word(keyword) {
        return Ok(None);
    }

    if LANDCYCLING_WORD_PATTERN.matches_word(keyword) {
        push_unique(&mut filter.card_types, CardType::Land);
        return Ok(Some(filter));
    }

    if let Some(root) = cycling_keyword_root(keyword).filter(|root| !root.is_empty()) {
        if let Some(card_type) = parse_card_type(root) {
            push_unique(&mut filter.card_types, card_type);
        } else if let Some(subtype) = parse_subtype_flexible(root) {
            push_unique(&mut filter.subtypes, subtype);
            if is_land_subtype(subtype) {
                push_unique(&mut filter.card_types, CardType::Land);
            }
        } else if let Some(color) = parse_color(root) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported cycling variant (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(Some(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported cycling variant (clause: '{}')",
        words.join(" ")
    )))
}

pub(crate) fn is_land_subtype(subtype: Subtype) -> bool {
    matches!(
        subtype,
        Subtype::Plains
            | Subtype::Island
            | Subtype::Swamp
            | Subtype::Mountain
            | Subtype::Forest
            | Subtype::Desert
    )
}

pub(crate) fn parse_equip_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let tokens = grammar::split_lexed_slices_on_period(tokens)
        .into_iter()
        .next()
        .unwrap_or(tokens);
    let clause_word_view = ActivationRestrictionCompatWords::new(tokens);
    let clause_words = clause_word_view.to_word_refs();
    if !clause_words
        .first()
        .is_some_and(|word| EQUIP_WORD_PATTERN.matches_words(&[*word]))
    {
        return Ok(None);
    }

    let first_mana_idx = crate::slice_primitives::find_index(&tokens[1..], |token| {
        mana_pips_from_token(token).is_some()
    })
    .map(|idx| idx + 1);

    let (mana_pips, saw_zero, saw_non_symbol) = tokens.iter().skip(1).fold(
        (Vec::new(), false, false),
        |(mut pips, mut saw_zero, mut saw_non_symbol), token| {
            if let Some(group) = mana_pips_from_token(token) {
                if group.len() == 1 && matches!(group[0], ManaSymbol::Generic(0)) {
                    saw_zero = true;
                } else {
                    pips.push(group);
                }
            } else {
                saw_non_symbol = true;
            }
            (pips, saw_zero, saw_non_symbol)
        },
    );

    if saw_non_symbol {
        if let Some(cost_start_idx) = first_mana_idx {
            let qualifier_tokens = trim_commas(&tokens[1..cost_start_idx]);
            let cost_tokens = trim_commas(&tokens[cost_start_idx..]);
            if !qualifier_tokens.is_empty() && !cost_tokens.is_empty() {
                let Some(target_filter) = parse_equip_target_filter_qualifier(&qualifier_tokens)
                else {
                    return Ok(None);
                };
                let total_cost = parse_activation_cost(&cost_tokens)?;
                let cost_text = total_cost
                    .mana_cost()
                    .map(|mana| mana.to_oracle())
                    .unwrap_or_else(|| {
                        crate::runtime_backend::token_word_refs(&cost_tokens).join(" ")
                    });
                let qualifier_text = keyword_title(
                    &crate::runtime_backend::token_word_refs(&qualifier_tokens).join(" "),
                );
                let equip_text = format!("Equip {qualifier_text} {cost_text}");
                let target = ChooseSpec::target(ChooseSpec::Object(target_filter));

                return Ok(Some(ParsedAbility {
                    ability: Ability {
                        kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                            mana_cost: total_cost,
                            effects: crate::resolution::ResolutionProgram::from_effects(vec![
                                Effect::attach_to(target.clone()),
                            ]),
                            choices: vec![target.clone()],
                            timing: ActivationTiming::SorcerySpeed,
                            additional_restrictions: vec![],
                            activation_restrictions: vec![],
                            mana_output: None,
                            activation_condition: None,
                            mana_usage_restrictions: vec![],
                            is_loyalty_ability: false,
                        }),
                        functional_zones: vec![Zone::Battlefield],
                    }
                    .into(),
                    text: Some(equip_text),
                    effects_ast: None,
                    reference_imports: ReferenceImports::default(),
                    trigger_spec: None,
                }));
            }
        }

        let looks_like_cost_prefix = clause_words.get(1).is_some_and(|word| {
            parse_mana_symbol(word).is_ok()
                || matches!(
                    *word,
                    "tap"
                        | "t"
                        | "pay"
                        | "discard"
                        | "sacrifice"
                        | "exile"
                        | "return"
                        | "remove"
                        | "behold"
                )
        });
        if !looks_like_cost_prefix {
            return Ok(None);
        }
        let cost_tokens = trim_commas(&tokens[1..]);
        if cost_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "equip missing activation cost".to_string(),
            ));
        }
        let total_cost = parse_activation_cost(&cost_tokens)?;
        let tail_words = crate::runtime_backend::token_word_refs(&cost_tokens);
        if tail_words.is_empty() {
            return Err(CardTextError::ParseError(
                "equip missing activation cost".to_string(),
            ));
        }
        let equip_text = format!("Equip—{}", keyword_title(&tail_words.join(" ")));
        let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

        return Ok(Some(ParsedAbility {
            ability: Ability {
                kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                    mana_cost: total_cost,
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![
                        Effect::attach_to(target.clone()),
                    ]),
                    choices: vec![target.clone()],
                    timing: ActivationTiming::SorcerySpeed,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: false,
                }),
                functional_zones: vec![Zone::Battlefield],
            }
            .into(),
            text: Some(equip_text),
            effects_ast: None,
            reference_imports: ReferenceImports::default(),
            trigger_spec: None,
        }));
    }

    if mana_pips.is_empty() && !saw_zero {
        return Err(CardTextError::ParseError(
            "equip missing mana cost".to_string(),
        ));
    }

    let mana_cost = if mana_pips.is_empty() {
        ManaCost::new()
    } else {
        ManaCost::from_pips(mana_pips)
    };
    let total_cost = if mana_cost.pips().is_empty() {
        TotalCost::free()
    } else {
        TotalCost::mana(mana_cost)
    };
    let equip_text = if saw_zero && total_cost.costs().is_empty() {
        "Equip {0}".to_string()
    } else if let Some(mana) = total_cost.mana_cost() {
        format!("Equip {}", mana.to_oracle())
    } else {
        "Equip".to_string()
    };
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![
                    Effect::attach_to(target.clone()),
                ]),
                choices: vec![target.clone()],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some(equip_text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

fn parse_equip_target_filter_qualifier(tokens: &[OwnedLexToken]) -> Option<ObjectFilter> {
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words.is_empty() {
        return None;
    }
    let mut subtype_words = words.as_slice();
    if subtype_words
        .last()
        .is_some_and(|word| CREATURE_WORD_PATTERN.matches_word(word))
    {
        subtype_words = &subtype_words[..subtype_words.len().saturating_sub(1)];
    }
    if subtype_words.is_empty() {
        return None;
    }

    let mut parsed_subtypes = Vec::new();
    let mut saw_subtype = false;
    for word in subtype_words {
        if EQUIP_SUBTYPE_CONNECTOR_PATTERN.matches_word(word) {
            if !saw_subtype {
                return None;
            }
            continue;
        }

        let subtype = parse_subtype_flexible(word)?;
        push_unique(&mut parsed_subtypes, subtype);
        saw_subtype = true;
    }

    if parsed_subtypes.is_empty() {
        return None;
    }

    let mut filter = ObjectFilter::creature().you_control();
    for subtype in parsed_subtypes {
        push_unique(&mut filter.subtypes, subtype);
    }
    Some(filter)
}

pub(crate) fn parse_equip_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_equip_line(tokens)
}

pub(crate) fn parse_reconfigure_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let tokens = grammar::split_lexed_slices_on_period(tokens)
        .into_iter()
        .next()
        .unwrap_or(tokens);
    let words = ActivationRestrictionCompatWords::new(tokens);
    if words
        .first()
        .is_none_or(|word| !RECONFIGURE_WORD_PATTERN.matches_word(word))
    {
        return Ok(None);
    }

    let mut cost_end = 1usize;
    while cost_end < tokens.len() {
        if matches!(tokens[cost_end].kind, TokenKind::LParen | TokenKind::Period) {
            break;
        }
        cost_end += 1;
    }
    let cost_tokens = trim_commas(&tokens[1..cost_end]);
    if cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "reconfigure missing activation cost".to_string(),
        ));
    }
    let total_cost = parse_activation_cost(&cost_tokens)?;
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
    let text = total_cost
        .mana_cost()
        .map(|mana| format!("Reconfigure {}", mana.to_oracle()))
        .unwrap_or_else(|| "Reconfigure".to_string());

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(crate::ability::ActivatedAbility {
                mana_cost: total_cost,
                effects: crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                    crate::effects::ReconfigureEffect::new(target.clone()),
                )]),
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
                activation_restrictions: vec![],
                mana_output: None,
                activation_condition: None,
                mana_usage_restrictions: vec![],
                is_loyalty_ability: false,
            }),
            functional_zones: vec![Zone::Battlefield],
        }
        .into(),
        text: Some(text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

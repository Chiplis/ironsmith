use super::*;
use crate::runtime_backend::grammar::activated_lines::{
    self as activated_line_grammar, ActivatedCyclingContext,
};
use crate::runtime_backend::grammar::keyword_activated_lines::{
    self as keyword_activated_grammar, CraftMaterialKind, CyclingFilterSpec,
    CyclingKeywordCostGroup, CyclingKeywordCostKind, CyclingSearchParseError, CyclingSearchSpec,
    EquipLineSpec,
};

pub(crate) fn parse_cycling_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_cycling_line_lexed(tokens)
}

pub(crate) fn parse_cycling_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_refs = crate::runtime_backend::token_word_refs(tokens);
    if word_refs.is_empty() {
        return Ok(None);
    }

    let Some(cycling_head) = activated_line_grammar::parse_cycling_keyword_head_words(&word_refs)
    else {
        return Ok(None);
    };
    if cycling_head.context == ActivatedCyclingContext::Granted {
        return Ok(None);
    }

    let clause_text = joined_activation_clause_text(tokens);
    let cycling_groups =
        keyword_activated_grammar::parse_cycling_keyword_cost_groups_tokens(tokens);
    let Some(first_group) = cycling_groups.first() else {
        return Ok(None);
    };
    if first_group.cost_tokens.is_empty() {
        return Ok(None);
    }

    let base_cost = parse_activation_cost(&first_group.cost_tokens)?;
    let base_cost_display = base_cost.display();
    for group in cycling_groups.iter().skip(1) {
        let next_cost = parse_activation_cost(&group.cost_tokens)?;
        if next_cost.display() != base_cost_display {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed cycling costs (clause: '{clause_text}')",
            )));
        }
    }

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

    let mut search_filter = parse_cycling_search_filter(&first_group.keyword_tokens)?;
    for group in cycling_groups.iter().skip(1) {
        let next_filter = parse_cycling_search_filter(&group.keyword_tokens)?;
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

    let cost_text = first_group
        .cost_kind
        .mana_cost()
        .map(ManaCost::to_oracle)
        .or_else(|| base_cost.mana_cost().map(|cost| cost.to_oracle()))
        .unwrap_or_else(|| {
            ActivationRestrictionCompatWords::new(&first_group.cost_tokens).join(" ")
        });
    let render_text = if let Some(group) = parse_cycling_keyword_group_text(&cycling_groups) {
        group
    } else if crate::runtime_backend::token_word_refs(&first_group.keyword_tokens).is_empty() {
        cost_text
    } else {
        format!(
            "{} {cost_text}",
            crate::runtime_backend::token_word_refs(&first_group.keyword_tokens).join(" ")
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
    let Some(spec) = keyword_activated_grammar::parse_channel_line_spec_tokens(tokens) else {
        return Ok(None);
    };

    let clause_text = joined_activation_clause_text(tokens);
    parse_hand_keyword_activated_body_lexed(spec.body_tokens, "channel", "Channel", &clause_text)
}

pub(crate) fn parse_craft_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_craft_line_lexed(tokens)
}

pub(crate) fn parse_craft_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(spec) = keyword_activated_grammar::parse_craft_line_spec_tokens(tokens) else {
        return Ok(None);
    };
    let material_text = crate::runtime_backend::token_word_refs(spec.material_tokens).join(" ");
    let (material_filter, material_count) = match spec.material {
        CraftMaterialKind::Artifact => (
            craft_battlefield_or_graveyard_filter(CardType::Artifact),
            ChoiceCount::exactly(1),
        ),
        CraftMaterialKind::Creature => (
            craft_creature_battlefield_or_graveyard_filter(),
            ChoiceCount::exactly(1),
        ),
        CraftMaterialKind::OneOrMore => (
            craft_any_battlefield_or_graveyard_filter(),
            ChoiceCount::at_least(1),
        ),
        CraftMaterialKind::RedInstantOrSorcery { minimum } => (
            craft_red_instant_or_sorcery_graveyard_filter(),
            ChoiceCount::at_least(minimum as usize),
        ),
        CraftMaterialKind::Unsupported => {
            return Err(CardTextError::ParseError(format!(
                "unsupported craft material clause '{material_text}'"
            )));
        }
    };
    let base_cost = parse_activation_cost(spec.cost_tokens)?;
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
        .unwrap_or_else(|| crate::runtime_backend::token_word_refs(spec.cost_tokens).join(" "));

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

fn parse_cycling_keyword_group_text(groups: &[CyclingKeywordCostGroup]) -> Option<String> {
    let parts = groups
        .iter()
        .filter_map(|group| {
            let keyword = crate::runtime_backend::token_word_refs(&group.keyword_tokens).join(" ");
            if keyword.is_empty() {
                return None;
            }
            let cost = match &group.cost_kind {
                CyclingKeywordCostKind::Mana(mana_cost) => mana_cost.to_oracle(),
                CyclingKeywordCostKind::PayLife { amount } => format!("pay {amount} life"),
                CyclingKeywordCostKind::Activation { .. } => {
                    crate::runtime_backend::lexer::render_token_slice(group.cost_tokens)
                        .trim()
                        .to_string()
                }
            };
            Some(format!("{keyword} {cost}"))
        })
        .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

pub(crate) fn parse_cycling_search_filter(
    tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    match keyword_activated_grammar::parse_cycling_search_spec_tokens(tokens) {
        Ok(CyclingSearchSpec::Draw) => Ok(None),
        Ok(CyclingSearchSpec::Search(CyclingFilterSpec {
            supertypes,
            card_types,
            subtypes,
            colors,
        })) => Ok(Some(ObjectFilter {
            supertypes,
            card_types,
            subtypes,
            colors,
            ..ObjectFilter::default()
        })),
        Err(CyclingSearchParseError::MissingKeyword) => Err(CardTextError::ParseError(
            "missing cycling keyword".to_string(),
        )),
        Err(CyclingSearchParseError::UnsupportedRoot(_)) => {
            let words = crate::runtime_backend::token_word_refs(tokens);
            Err(CardTextError::ParseError(format!(
                "unsupported cycling variant (clause: '{}')",
                words.join(" ")
            )))
        }
    }
}

pub(crate) fn is_land_subtype(subtype: Subtype) -> bool {
    subtype.is_land_subtype()
}

pub(crate) fn parse_equip_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(spec) = keyword_activated_grammar::parse_equip_line_spec_tokens(tokens) else {
        return Ok(None);
    };
    match spec {
        EquipLineSpec::MissingCost => Err(CardTextError::ParseError(
            "equip missing activation cost".to_string(),
        )),
        EquipLineSpec::Mana { cost } => {
            let (total_cost, cost_text) = equip_mana_total_cost(cost);
            Ok(Some(build_equip_ability(
                total_cost,
                format!("Equip {cost_text}"),
                ObjectFilter::creature().you_control(),
            )))
        }
        EquipLineSpec::QualifiedCost {
            qualifier,
            cost_tokens,
            mana_prefix,
            exact_mana_cost,
        } => {
            let total_cost = parse_activation_cost(cost_tokens)?;
            let cost_text = if exact_mana_cost {
                mana_prefix.to_oracle()
            } else {
                total_cost
                    .mana_cost()
                    .map(ManaCost::to_oracle)
                    .unwrap_or_else(|| ActivationRestrictionCompatWords::new(cost_tokens).join(" "))
            };
            let qualifier_text =
                keyword_title(&crate::runtime_backend::token_word_refs(qualifier.tokens).join(" "));
            let mut target_filter = ObjectFilter::creature().you_control();
            target_filter.subtypes = qualifier.subtypes;
            Ok(Some(build_equip_ability(
                total_cost,
                format!("Equip {qualifier_text} {cost_text}"),
                target_filter,
            )))
        }
        EquipLineSpec::ActivationCost { cost_tokens } => {
            let total_cost = parse_activation_cost(cost_tokens)?;
            let tail_words = crate::runtime_backend::token_word_refs(cost_tokens);
            if tail_words.is_empty() {
                return Err(CardTextError::ParseError(
                    "equip missing activation cost".to_string(),
                ));
            }
            Ok(Some(build_equip_ability(
                total_cost,
                format!("Equip—{}", keyword_title(&tail_words.join(" "))),
                ObjectFilter::creature().you_control(),
            )))
        }
    }
}

fn equip_mana_total_cost(cost: ManaCost) -> (TotalCost, String) {
    let mut saw_zero = false;
    let pips = cost
        .pips()
        .iter()
        .filter_map(|pip| {
            if matches!(pip.as_slice(), [ManaSymbol::Generic(0)]) {
                saw_zero = true;
                None
            } else {
                Some(pip.clone())
            }
        })
        .collect::<Vec<_>>();
    if pips.is_empty() {
        let text = if saw_zero { "{0}" } else { "" }.to_string();
        return (TotalCost::free(), text);
    }
    let mana_cost = ManaCost::from_pips(pips);
    let text = mana_cost.to_oracle();
    (TotalCost::mana(mana_cost), text)
}

fn build_equip_ability(
    total_cost: TotalCost,
    text: String,
    target_filter: ObjectFilter,
) -> ParsedAbility {
    let target = ChooseSpec::target(ChooseSpec::Object(target_filter));
    ParsedAbility {
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
        text: Some(text),
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }
}

pub(crate) fn parse_equip_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_equip_line(tokens)
}

pub(crate) fn parse_reconfigure_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(spec) = keyword_activated_grammar::parse_reconfigure_line_spec_tokens(tokens) else {
        return Ok(None);
    };
    if spec.cost_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "reconfigure missing activation cost".to_string(),
        ));
    }
    let total_cost = parse_activation_cost(spec.cost_tokens)?;
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

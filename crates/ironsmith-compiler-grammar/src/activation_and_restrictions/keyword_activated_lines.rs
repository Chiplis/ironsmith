use crate::cards::builders::LifeResourceActionAst;
use crate::cards::builders::ZoneMoveActionAst;
use crate::cards::builders::KeywordActionAst;
use super::*;
use crate::grammar::activated_lines::{self as activated_line_grammar, ActivatedCyclingContext};
use crate::grammar::keyword_activated_lines::{
    self as keyword_activated_grammar, CraftMaterialKind, CyclingFilterSpec,
    CyclingSearchParseError, CyclingSearchSpec, EquipLineSpec,
};

pub fn parse_cycling_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_cycling_line_lexed(tokens)
}

pub fn parse_cycling_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let word_refs = crate::lexer::token_word_refs(tokens);
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

    let base_cost = parse_compiler_activation_cost(first_group.cost_tokens)?;
    let base_cost_display = base_cost.display();
    for group in cycling_groups.iter().skip(1) {
        let next_cost = parse_compiler_activation_cost(group.cost_tokens)?;
        if next_cost.display() != base_cost_display {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed cycling costs (clause: '{clause_text}')",
            )));
        }
    }

    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::model::CompilerCost::DiscardSource);
    merged_costs.push(crate::model::CompilerCost::EmitKeywordAction {
        kind: crate::events::KeywordActionKind::Cycle,
        amount: 1,
    });
    let mana_cost = ironsmith_core::TotalCost::from_costs(merged_costs);

    let mut search_filter = parse_cycling_search_filter(first_group.keyword_tokens)?;
    for group in cycling_groups.iter().skip(1) {
        let next_filter = parse_cycling_search_filter(group.keyword_tokens)?;
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
        EffectAst::subject_verb_search_library(
            filter,
            Zone::Hand,
            PlayerAst::You,
            PlayerAst::You,
            crate::effect::SearchSelectionMode::Exact,
            true,
            None,
            true,
            ChoiceCount::exactly(1),
            None,
            None,
            crate::effect::SearchResultReferenceSurface::ThatCard,
            false,
            false,
            false,
        )
    } else {
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                count: Value::Fixed(1),
            }),
        )
    };

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects: ironsmith_core::ResolutionProgram::from_effects(vec![effect]),
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
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

pub fn parse_channel_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(spec) = keyword_activated_grammar::parse_channel_line_spec_tokens(tokens) else {
        return Ok(None);
    };

    let clause_text = joined_activation_clause_text(tokens);
    parse_hand_keyword_activated_body_lexed(spec.body_tokens, "channel", &clause_text)
}

pub fn parse_craft_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(spec) = keyword_activated_grammar::parse_craft_line_spec_tokens(tokens) else {
        return Ok(None);
    };
    let material_text = crate::lexer::token_word_refs(spec.material_tokens).join(" ");
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
    let base_cost = parse_compiler_activation_cost(spec.cost_tokens)?;
    let mut merged_costs = base_cost.costs().to_vec();
    merged_costs.push(crate::model::CompilerCost::ExileChosen {
        count: material_count,
        filter: material_filter,
        top_only: false,
        turn_face_up: false,
        binding: None,
    });
    merged_costs.push(crate::model::CompilerCost::EmitKeywordAction {
        kind: crate::events::KeywordActionKind::Craft,
        amount: 1,
    });
    merged_costs.push(crate::model::CompilerCost::ExileSelf {
        from_graveyard: false,
    });

    let return_transformed = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::ZoneMoves(ZoneMoveActionAst::ReturnSourceTransformedFromExile),
    );
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: ironsmith_core::TotalCost::from_costs(merged_costs),
                effects: ironsmith_core::ResolutionProgram::from_effects(vec![return_transformed]),
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

pub fn merge_cycling_search_filters(base: &mut ObjectFilter, extra: &ObjectFilter) {
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

pub fn parse_cycling_search_filter(
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
            let words = crate::lexer::token_word_refs(tokens);
            Err(CardTextError::ParseError(format!(
                "unsupported cycling variant (clause: '{}')",
                words.join(" ")
            )))
        }
    }
}

pub fn parse_equip_line(tokens: &[OwnedLexToken]) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(spec) = keyword_activated_grammar::parse_equip_line_spec_tokens(tokens) else {
        return Ok(None);
    };
    let Some(mut parsed) = (match spec {
        EquipLineSpec::MissingCost => Err(CardTextError::ParseError(
            "equip missing activation cost".to_string(),
        )),
        EquipLineSpec::Mana { cost } => {
            let total_cost = equip_mana_total_cost(cost);
            Ok(Some(build_equip_ability(
                total_cost,
                ObjectFilter::creature().you_control(),
            )))
        }
        EquipLineSpec::QualifiedCost {
            qualifier,
            cost_tokens,
            mana_prefix: _,
            exact_mana_cost: _,
        } => {
            let total_cost = parse_compiler_activation_cost(cost_tokens)?;
            let mut target_filter = ObjectFilter::creature().you_control();
            target_filter.subtypes = qualifier.subtypes;
            Ok(Some(build_equip_ability(total_cost, target_filter)))
        }
        EquipLineSpec::ActivationCost { cost_tokens } => {
            let total_cost = parse_compiler_activation_cost(cost_tokens)?;
            let tail_words = crate::lexer::token_word_refs(cost_tokens);
            if tail_words.is_empty() {
                return Err(CardTextError::ParseError(
                    "equip missing activation cost".to_string(),
                ));
            }
            Ok(Some(build_equip_ability(
                total_cost,
                ObjectFilter::creature().you_control(),
            )))
        }
    })?
    else {
        return Err(CardTextError::InvariantViolation(
            "equip grammar matched without producing an ability".to_string(),
        ));
    };

    let AbilityKind::Activated(activated) = &mut parsed.ability.kind else {
        return Err(CardTextError::InvariantViolation(
            "equip grammar produced a non-activated ability".to_string(),
        ));
    };
    for sentence in crate::lexer::split_lexed_sentences(tokens)
        .into_iter()
        .skip(1)
    {
        let Some(restriction) =
            crate::grammar::restriction_facts::parse_activation_restriction_tokens(sentence)
        else {
            continue;
        };
        if restriction.timing == Some(ActivationTiming::OncePerTurn) {
            crate::slice_primitives::push_unique(
                &mut activated.additional_restrictions,
                restriction.presentation_text,
            );
        }
    }

    Ok(Some(parsed))
}

fn equip_mana_total_cost(cost: ManaCost) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
    let pips = cost
        .pips()
        .iter()
        .filter_map(|pip| {
            if matches!(pip.as_slice(), [ManaSymbol::Generic(0)]) {
                None
            } else {
                Some(pip.clone())
            }
        })
        .collect::<Vec<_>>();
    if pips.is_empty() {
        return ironsmith_core::TotalCost::free();
    }
    let mana_cost = ManaCost::from_pips(pips);
    ironsmith_core::TotalCost::mana(mana_cost)
}

fn build_equip_ability(
    total_cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    target_filter: ObjectFilter,
) -> ParsedAbility {
    let target = TargetAst::Object(target_filter, None, None);
    ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: total_cost,
                effects: ironsmith_core::ResolutionProgram::from_effects(vec![
                    EffectAst::subject_verb_attach(TargetAst::Source(None), target),
                ]),
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
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }
}

pub fn parse_equip_line_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<ParsedAbility>, CardTextError> {
    parse_equip_line(tokens)
}

pub fn parse_reconfigure_line_lexed(
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
    let total_cost = parse_compiler_activation_cost(spec.cost_tokens)?;
    let target = TargetAst::Object(ObjectFilter::creature().you_control(), None, None);
    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: total_cost,
                effects: ironsmith_core::ResolutionProgram::from_effects(vec![
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::Actor,
                        PlayerAst::Implicit,
                        SubjectVerbActionAst::KeywordActions(KeywordActionAst::Reconfigure { target }),
                    ),
                ]),
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
        effects_ast: None,
        reference_imports: ReferenceImports::default(),
        trigger_spec: None,
    }))
}

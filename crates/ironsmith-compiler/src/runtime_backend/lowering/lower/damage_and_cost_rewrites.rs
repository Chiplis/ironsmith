use super::*;

fn static_ability_uses_persistent_chosen_player(
    ability: &crate::static_abilities::StaticAbility,
) -> bool {
    let filter_uses_chosen_player = |filter: &ObjectFilter| {
        filter.attacking_player_or_planeswalker_controlled_by
            == Some(crate::target::PlayerFilter::ChosenPlayer)
    };

    match &ability.payload {
        crate::static_abilities::StaticAbilityPayload::Anthem(anthem) => anthem
            .filter
            .as_ref()
            .is_some_and(filter_uses_chosen_player),
        crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
            filter_uses_chosen_player(&grant.filter)
        }
        crate::static_abilities::StaticAbilityPayload::Conditional { ability, .. } => {
            static_ability_uses_persistent_chosen_player(ability)
        }
        _ => false,
    }
}

fn remember_single_cross_ability_player_choice(builder: &mut CardDefinitionBuilder) {
    let has_persistent_reference = builder.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability_uses_persistent_chosen_player(static_ability)
        )
    });
    if !has_persistent_reference {
        return;
    }

    let choices = builder
        .abilities
        .iter()
        .enumerate()
        .flat_map(|(ability_index, ability)| {
            let AbilityKind::Triggered(triggered) = &ability.kind else {
                return Vec::new().into_iter();
            };
            triggered
                .effects
                .segments
                .iter()
                .enumerate()
                .flat_map(move |(segment_index, segment)| {
                    segment.default_effects.iter().enumerate().filter_map(
                        move |(effect_index, effect)| {
                            effect
                                .downcast_ref::<crate::effects::ChoosePlayerEffect>()
                                .map(|choice| {
                                    (ability_index, segment_index, effect_index, choice.clone())
                                })
                        },
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect::<Vec<_>>();
    let [(ability_index, segment_index, effect_index, choice)] = choices.as_slice() else {
        return;
    };
    let AbilityKind::Triggered(triggered) = &mut builder.abilities[*ability_index].kind else {
        return;
    };
    triggered.effects.segments[*segment_index].default_effects[*effect_index] =
        crate::effect::Effect::new(choice.clone().remember_as_chosen_player());
}

pub(crate) fn lower_normalized_card_ast_with_facts(
    ast: NormalizedCardAst,
) -> Result<LoweredCardDocument, CardTextError> {
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    } = ast;
    let overload_ast = overload_branch.map(|branch| NormalizedCardAst {
        builder: builder.clone(),
        annotations: ParseAnnotations::default(),
        items: branch.items,
        overload_branch: None,
        cleave_branch: None,
        allow_unsupported,
    });
    let cleave_ast = cleave_branch.map(|branch| NormalizedCardAst {
        builder: builder.clone(),
        annotations: ParseAnnotations::default(),
        items: branch.items,
        overload_branch: None,
        cleave_branch: None,
        allow_unsupported,
    });

    let mut level_abilities = Vec::new();
    let mut level_activated_lines = Vec::new();
    let mut last_restrictable_ability: Option<usize> = None;
    let mut state = RewriteLoweredCardState::default();

    for item in items {
        match item {
            NormalizedCardItem::Line(line) => {
                rewrite_lower_line_ast(
                    &mut builder,
                    &mut state,
                    &mut annotations,
                    line,
                    allow_unsupported,
                    &mut last_restrictable_ability,
                )?;
            }
            NormalizedCardItem::Modal(modal) => {
                let abilities_before = builder.abilities.len();
                builder = rewrite_lower_parsed_modal(builder, modal, allow_unsupported)?;
                rewrite_update_last_restrictable_ability(
                    &builder,
                    abilities_before,
                    &mut last_restrictable_ability,
                );
            }
            NormalizedCardItem::LevelAbility(level) => {
                let lowered = rewrite_lower_level_ability_ast(level)?;
                level_abilities.push(lowered.level_ability);
                level_activated_lines.extend(lowered.activated_lines);
            }
        }
    }

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }
    for line in level_activated_lines {
        rewrite_lower_line_ast(
            &mut builder,
            &mut state,
            &mut annotations,
            line,
            allow_unsupported,
            &mut last_restrictable_ability,
        )?;
    }

    remember_single_cross_ability_player_choice(&mut builder);
    builder = rewrite_finalize_lowered_card(builder, &mut state);
    if let Some(overload_ast) = overload_ast {
        let overloaded = lower_normalized_card_ast_with_facts(overload_ast)?;
        let overload_effects = overloaded
            .definition
            .spell_effect
            .unwrap_or_default()
            .to_vec();
        for method in &mut builder.alternative_casts {
            if let crate::alternative_cast::AlternativeCastingMethod::Overload { effects, .. } =
                method
            {
                *effects = overload_effects.clone();
            }
        }
    }
    if let Some(cleave_ast) = cleave_ast {
        let cleaved = lower_normalized_card_ast_with_facts(cleave_ast)?;
        let cleave_effects = cleaved.definition.spell_effect.unwrap_or_default().to_vec();
        for method in &mut builder.alternative_casts {
            if let crate::alternative_cast::AlternativeCastingMethod::Cleave { effects, .. } =
                method
            {
                *effects = cleave_effects.clone();
            }
        }
    }
    Ok(LoweredCardDocument {
        definition: builder.build(),
        annotations,
    })
}

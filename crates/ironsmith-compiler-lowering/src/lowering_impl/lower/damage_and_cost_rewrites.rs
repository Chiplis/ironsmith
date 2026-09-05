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

fn filter_uses_persistent_chosen_object(filter: &ObjectFilter) -> bool {
    filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
            && matches!(
                constraint.relation,
                crate::filter::TaggedOpbjectRelation::IsTaggedObject
                    | crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
            )
    }) || filter
        .any_of
        .iter()
        .any(filter_uses_persistent_chosen_object)
}

fn static_ability_uses_persistent_chosen_object(
    ability: &crate::static_abilities::StaticAbility,
) -> bool {
    match &ability.payload {
        crate::static_abilities::StaticAbilityPayload::Anthem(anthem) => anthem
            .filter
            .as_ref()
            .is_some_and(filter_uses_persistent_chosen_object),
        crate::static_abilities::StaticAbilityPayload::GrantObjectAbilityForFilter(grant) => {
            filter_uses_persistent_chosen_object(&grant.filter)
        }
        crate::static_abilities::StaticAbilityPayload::Conditional { ability, .. } => {
            static_ability_uses_persistent_chosen_object(ability)
        }
        _ => false,
    }
}

fn choose_spec_uses_persistent_chosen_object(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Tagged(tag) => {
            tag.as_str() == crate::tag::CompilerReferenceTag::ChosenObjects.as_str()
        }
        ChooseSpec::SurfaceHinted { spec, .. }
        | ChooseSpec::Target(spec)
        | ChooseSpec::WithCount(spec, _)
        | ChooseSpec::WithCountValue(spec, _, _) => choose_spec_uses_persistent_chosen_object(spec),
        ChooseSpec::Object(filter) | ChooseSpec::All(filter) => {
            filter_uses_persistent_chosen_object(filter)
        }
        _ => false,
    }
}

fn effect_uses_persistent_chosen_object(effect: &crate::effect::Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::SacrificeTargetEffect>()
        .is_some_and(|sacrifice| choose_spec_uses_persistent_chosen_object(&sacrifice.target))
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= effect_uses_persistent_chosen_object(child);
    });
    found
}

fn triggered_ability_uses_persistent_chosen_object(
    triggered: &crate::ability::TriggeredAbility,
) -> bool {
    fn trigger_uses_persistent_chosen_object(trigger: &crate::triggers::Trigger) -> bool {
        match &trigger.kind {
            crate::triggers::TriggerKind::LeavesBattlefield { filter } => {
                filter_uses_persistent_chosen_object(filter)
            }
            crate::triggers::TriggerKind::ZoneChange(zone_change) => zone_change
                .filter
                .as_ref()
                .is_some_and(filter_uses_persistent_chosen_object),
            crate::triggers::TriggerKind::AnyOf(triggers) => {
                triggers.iter().any(trigger_uses_persistent_chosen_object)
            }
            crate::triggers::TriggerKind::Either { left, right } => {
                trigger_uses_persistent_chosen_object(left)
                    || trigger_uses_persistent_chosen_object(right)
            }
            _ => false,
        }
    }

    trigger_uses_persistent_chosen_object(&triggered.trigger)
        || triggered
            .effects
            .segments
            .iter()
            .flat_map(|segment| segment.default_effects.iter())
            .any(effect_uses_persistent_chosen_object)
}

/// Persist an object choice only when a different ability proves an authored
/// `the chosen <object>` reference. This keeps ordinary resolution-local
/// choices out of permanent state and avoids treating generic `it` tags as
/// cross-ability identity.
fn remember_single_cross_ability_object_choice(builder: &mut CardDefinitionBuilder) {
    let reference_abilities = builder
        .abilities
        .iter()
        .enumerate()
        .filter_map(|(ability_index, ability)| {
            match &ability.kind {
                AbilityKind::Triggered(triggered) => {
                    triggered_ability_uses_persistent_chosen_object(triggered)
                }
                AbilityKind::Static(static_ability) => {
                    static_ability_uses_persistent_chosen_object(static_ability)
                }
                _ => false,
            }
            .then_some(ability_index)
        })
        .collect::<Vec<_>>();
    if reference_abilities.is_empty() {
        return;
    }

    fn single_choice(
        effect: &crate::effect::Effect,
    ) -> Option<crate::effects::ChooseObjectsEffect> {
        if let Some(choice) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() {
            return (choice.count.min == 1
                && choice.count.max == Some(1)
                && choice.count_value.is_none())
            .then(|| choice.clone());
        }
        let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        single_choice(&with_id.effect)
    }

    #[derive(Clone, Copy)]
    enum ChoiceContainer {
        Triggered,
        AsEnters,
    }

    let choices = builder
        .abilities
        .iter()
        .enumerate()
        .flat_map(|(ability_index, ability)| {
            let (container, program) = match &ability.kind {
                AbilityKind::Triggered(triggered) => {
                    (ChoiceContainer::Triggered, &triggered.effects)
                }
                AbilityKind::Static(static_ability) => {
                    let crate::static_abilities::StaticAbilityPayload::AsEntersEffectProgram {
                        program,
                        ..
                    } = &static_ability.payload
                    else {
                        return Vec::new().into_iter();
                    };
                    (ChoiceContainer::AsEnters, program)
                }
                _ => return Vec::new().into_iter(),
            };
            program
                .segments
                .iter()
                .enumerate()
                .flat_map(move |(segment_index, segment)| {
                    segment.default_effects.iter().enumerate().filter_map(
                        move |(effect_index, effect)| {
                            let choice = single_choice(effect)?;
                            Some((
                                ability_index,
                                segment_index,
                                effect_index,
                                container,
                                choice,
                            ))
                        },
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect::<Vec<_>>();
    let [(ability_index, segment_index, effect_index, container, choice)] = choices.as_slice()
    else {
        return;
    };
    if !reference_abilities
        .iter()
        .any(|reference_index| reference_index != ability_index)
    {
        return;
    }
    fn remember_choice(
        effect: &crate::effect::Effect,
        choice: &crate::effects::ChooseObjectsEffect,
    ) -> Option<crate::effect::Effect> {
        if effect
            .downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .is_some()
        {
            return Some(crate::effect::Effect::new(
                choice.clone().remember_as_chosen_object(),
            ));
        }
        let with_id = effect.downcast_ref::<crate::effects::WithIdEffect>()?;
        Some(crate::effect::Effect::new(
            crate::effects::WithIdEffect::new(
                with_id.id,
                remember_choice(&with_id.effect, choice)?,
            ),
        ))
    }

    let root = match container {
        ChoiceContainer::Triggered => {
            let AbilityKind::Triggered(triggered) = &mut builder.abilities[*ability_index].kind
            else {
                return;
            };
            &mut triggered.effects.segments[*segment_index].default_effects[*effect_index]
        }
        ChoiceContainer::AsEnters => {
            let AbilityKind::Static(static_ability) = &mut builder.abilities[*ability_index].kind
            else {
                return;
            };
            let crate::static_abilities::StaticAbilityPayload::AsEntersEffectProgram {
                program,
                ..
            } = &mut static_ability.payload
            else {
                return;
            };
            &mut program.segments[*segment_index].default_effects[*effect_index]
        }
    };
    if let Some(rewritten) = remember_choice(root, choice) {
        *root = rewritten;
    }
}

/// The reference scope of the line with this display index, entered for the
/// duration of its lowering; `None` when the parse opened no scope for it.
fn line_reference_scope(
    symbols: &std::cell::RefCell<crate::model::symbols::SymbolTable>,
    display_line_index: usize,
) -> Option<ironsmith_compiler_ast::reference_ledger::ReferenceScopeGuard<'_>> {
    let scope = symbols.borrow().line_scope(display_line_index)?;
    Some(ironsmith_compiler_ast::reference_ledger::ReferenceScopeGuard::enter(symbols, scope))
}

pub fn lower_normalized_card_ast_with_facts(
    ast: NormalizedCardAst,
) -> Result<LoweredCardDocument, CardTextError> {
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        provenance,
        symbols,
        items,
        overload_branch,
        cleave_branch,
        allow_unsupported,
    } = ast;
    let provenance_view = provenance.view();
    // Lowering mints keys of its own (auto-tags, pronoun antecedents): they
    // bind in the scope of the line being lowered, like the grammar's.
    let symbols = std::cell::RefCell::new(symbols);
    // Whatever lowering mints outside a line (finalization, shared rewrites)
    // binds in the document's scope, visible from every line.
    let document_scope = {
        let table = symbols.borrow();
        table
            .scopes()
            .iter()
            .find(|scope| scope.kind == crate::model::symbols::SymbolScopeKind::Document)
            .map(|scope| scope.id)
            .unwrap_or(table.root_scope())
    };
    let _document_references =
        ironsmith_compiler_ast::reference_ledger::ReferenceScopeGuard::enter(&symbols, document_scope);
    let overload_ast = overload_branch.map(|branch| NormalizedCardAst {
        builder: builder.clone(),
        annotations: ParseAnnotations::default(),
        provenance: provenance.clone(),
        symbols: symbols.borrow().clone(),
        items: branch.items,
        overload_branch: None,
        cleave_branch: None,
        allow_unsupported,
    });
    let cleave_ast = cleave_branch.map(|branch| NormalizedCardAst {
        builder: builder.clone(),
        annotations: ParseAnnotations::default(),
        provenance: provenance.clone(),
        symbols: symbols.borrow().clone(),
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
                let _references = line_reference_scope(&symbols, line.info.display_line_index);
                lower_line_ast(
                    &mut builder,
                    &mut state,
                    &mut annotations,
                    line,
                    allow_unsupported,
                    &mut last_restrictable_ability,
                )?;
            }
            NormalizedCardItem::Modal(modal) => {
                let _references =
                    line_reference_scope(&symbols, modal.header.info.display_line_index);
                let abilities_before = builder.abilities.len();
                builder = lower_parsed_modal(builder, modal, allow_unsupported, provenance_view)?;
                update_last_restrictable_ability(
                    &builder,
                    abilities_before,
                    &mut last_restrictable_ability,
                );
            }
            NormalizedCardItem::LevelAbility(level) => {
                let _references = level.items.iter().find_map(|item| match item {
                    crate::model::ParsedLevelAbilityItemAst::ActivatedAbility(activated) => {
                        line_reference_scope(&symbols, activated.info.display_line_index)
                    }
                    _ => None,
                });
                let lowered = lower_level_ability_ast(level)?;
                level_abilities.push(lowered.level_ability);
                level_activated_lines.extend(lowered.activated_lines);
            }
        }
    }

    if !level_abilities.is_empty() {
        builder = builder.with_level_abilities(level_abilities);
    }
    for line in level_activated_lines {
        lower_line_ast(
            &mut builder,
            &mut state,
            &mut annotations,
            line,
            allow_unsupported,
            &mut last_restrictable_ability,
        )?;
    }

    remember_single_cross_ability_player_choice(&mut builder);
    remember_single_cross_ability_object_choice(&mut builder);
    builder = finalize_lowered_card(builder, &mut state);
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
    // Building the definition expands keywords (undying, persist, ...) that
    // mint keys of their own: still inside the document's reference scope.
    let definition = builder.build();
    drop(_document_references);
    Ok(LoweredCardDocument {
        symbols: symbols.into_inner(),
        definition,
        annotations,
    })
}

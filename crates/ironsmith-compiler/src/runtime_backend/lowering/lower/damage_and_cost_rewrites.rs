use super::*;
pub(crate) fn lower_normalized_card_ast_with_facts(
    ast: NormalizedCardAst,
) -> Result<LoweredCardDocument, CardTextError> {
    let NormalizedCardAst {
        mut builder,
        mut annotations,
        items,
        overload_branch,
        allow_unsupported,
    } = ast;
    let overload_ast = overload_branch.map(|branch| NormalizedCardAst {
        builder: builder.clone(),
        annotations: ParseAnnotations::default(),
        items: branch.items,
        overload_branch: None,
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
    Ok(LoweredCardDocument {
        definition: builder.build(),
        annotations,
    })
}

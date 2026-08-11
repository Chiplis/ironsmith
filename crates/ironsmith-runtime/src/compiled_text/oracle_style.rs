use super::*;

pub fn canonical_compiled_lines(def: &CardDefinition) -> Vec<String> {
    let oracle_short = super::ast_render::oracle_short_self_name(def);
    let lines = super::normalize_ast_surface_lines(super::debug_compiled_surface_lines(def))
        .into_iter()
        .map(|line| {
            super::substitute_legendary_source_reference(
                &line,
                &def.card,
                "",
                oracle_short.as_deref(),
            )
        })
        .map(|line| super::normalize_punctuated_card_name_damage_case(line, &def.card.name))
        .map(|line| super::capitalize_first(&line))
        .map(|line| {
            if line == "Convoke."
                && def.abilities.iter().any(|ability| {
                    matches!(
                        &ability.kind,
                        AbilityKind::Static(static_ability)
                            if static_ability.id()
                                == crate::static_abilities::StaticAbilityId::Convoke
                    )
                })
            {
                "Convoke".to_string()
            } else {
                line
            }
        })
        .map(super::restore_cleave_bracket_surface)
        .collect();
    super::append_typed_standard_reminder_lines(def, lines)
}

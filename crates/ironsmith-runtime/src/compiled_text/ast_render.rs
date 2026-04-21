use super::*;
use crate::cards::CardDefinitionRuntimeExt;

pub(super) fn debug_safe_surface_definition(def: &CardDefinition) -> CardDefinition {
    let mut structured_def = def.clone();
    structured_def.card.oracle_text.clear();
    for ability in &mut structured_def.abilities {
        ability.text = None;
    }
    structured_def
}

pub(super) fn ast_compiled_lines(def: &CardDefinition) -> Vec<String> {
    stacker::maybe_grow(1024 * 1024, 8 * 1024 * 1024, || compiled_lines_inner(def))
}

pub(super) fn describe_resolution_program(
    program: &crate::resolution::ResolutionProgram,
) -> String {
    let mut rendered_segments = Vec::new();
    for segment in &program.segments {
        if segment.self_replacements.len() == 1 {
            let branch = &segment.self_replacements[0];
            rendered_segments.push(describe_effect_list(&[Effect::conditional(
                branch.condition.clone(),
                branch.replacement_effects.clone(),
                segment.default_effects.clone(),
            )]));
            continue;
        }

        if !segment.default_effects.is_empty() {
            rendered_segments.push(describe_effect_list(&segment.default_effects));
        }
        for branch in &segment.self_replacements {
            rendered_segments.push(describe_effect_list(&branch.replacement_effects));
        }
    }
    rendered_segments.join(". ")
}

fn is_standard_gift_render_payload(lower: &str) -> bool {
    lower.contains("chosen player draws a card")
        || lower.contains("chosen player creates a treasure token")
        || lower.contains("create a treasure token under the chosen player's control")
        || lower.contains("chosen player creates a food token")
        || lower.contains("create a food token under the chosen player's control")
        || lower.contains("chosen player creates a tapped 1/1 blue fish creature token")
        || lower.contains("create a 1/1 blue fish creature token under the chosen player's control")
        || lower.contains("chosen player takes an extra turn after this one")
        || lower.contains("chosen player creates an 8/8 blue octopus creature token")
        || lower
            .contains("create an 8/8 blue octopus creature token under the chosen player's control")
        || lower
            .contains("create a 8/8 blue octopus creature token under the chosen player's control")
}

fn is_hidden_gift_resolution_segment(segment: &crate::resolution::ResolutionSegment) -> bool {
    if !segment.self_replacements.is_empty() || segment.default_effects.is_empty() {
        return false;
    }

    let lower = describe_effect_list(&segment.default_effects).to_ascii_lowercase();
    lower.starts_with("if the gift was promised") && is_standard_gift_render_payload(&lower)
}

fn describe_resolution_program_for_card(
    def: &CardDefinition,
    program: &crate::resolution::ResolutionProgram,
) -> String {
    let has_visible_gift_line = def
        .optional_costs
        .iter()
        .any(|cost| cost.label.trim().to_ascii_lowercase().starts_with("gift "));
    if !has_visible_gift_line {
        return describe_resolution_program(program);
    }

    let mut rendered_segments = Vec::new();
    for segment in &program.segments {
        if is_hidden_gift_resolution_segment(segment) {
            continue;
        }

        if segment.self_replacements.len() == 1 {
            let branch = &segment.self_replacements[0];
            rendered_segments.push(describe_effect_list(&[Effect::conditional(
                branch.condition.clone(),
                branch.replacement_effects.clone(),
                segment.default_effects.clone(),
            )]));
            continue;
        }

        if !segment.default_effects.is_empty() {
            rendered_segments.push(describe_effect_list(&segment.default_effects));
        }
        for branch in &segment.self_replacements {
            rendered_segments.push(describe_effect_list(&branch.replacement_effects));
        }
    }

    rendered_segments.join(". ")
}

pub(super) fn describe_alternative_cast_line(
    method: &AlternativeCastingMethod,
    idx: usize,
) -> String {
    match method {
        method if method.is_composed_cost() => {
            let name = method.name();
            let mana_cost = method.mana_cost();
            let costs = method.non_mana_costs();
            let cast_condition = method.cast_condition();
            let mut parts = Vec::new();
            if let Some(cost) = mana_cost {
                parts.push(format!("pay {}", cost.to_oracle()));
            }
            if !costs.is_empty() {
                parts.push(describe_alternative_costs(&costs));
            }
            let clause = if parts.is_empty() {
                "cast this spell without paying its mana cost".to_string()
            } else {
                parts.join(" and ")
            };
            let mut line = format!("You may {clause} rather than pay this spell's mana cost");
            if !name.is_empty() {
                line.push_str(&format!(" ({name})"));
            }
            if let Some(condition) = cast_condition
                && let Some(condition_text) =
                    crate::static_abilities::describe_this_spell_cost_condition(condition)
            {
                line = format!("If {condition_text}, {}", lowercase_first(&line));
            }
            line
        }
        AlternativeCastingMethod::Madness { cost } => format!("Madness {}", cost.to_oracle()),
        AlternativeCastingMethod::Miracle { cost } => format!("Miracle {}", cost.to_oracle()),
        AlternativeCastingMethod::FlashWithAdditionalCost {
            additional_cost, ..
        } => format!(
            "You may cast this spell as though it had flash if you pay {} more to cast it",
            additional_cost.to_oracle()
        ),
        AlternativeCastingMethod::Plot { cost } => format!("Plot {}", cost.to_oracle()),
        AlternativeCastingMethod::Warp { cost } => format!("Warp {}", cost.to_oracle()),
        AlternativeCastingMethod::Suspend { cost, time } => {
            format!("Suspend {time}—{}", cost.to_oracle())
        }
        AlternativeCastingMethod::Disturb { cost } => format!("Disturb {}", cost.to_oracle()),
        AlternativeCastingMethod::Overload { cost, .. } => {
            format!("Overload {}", cost.to_oracle())
        }
        AlternativeCastingMethod::Flashback { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Flashback—{mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Flashback—{mana_cost}, {extra}")
            }
        }
        AlternativeCastingMethod::Harmonize { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Harmonize {mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Harmonize {mana_cost}, {extra}")
            }
        }
        AlternativeCastingMethod::JumpStart => "Jump-start".to_string(),
        AlternativeCastingMethod::Escape { cost, exile_count } => {
            let count_text = small_number_word(*exile_count)
                .map(str::to_string)
                .unwrap_or_else(|| exile_count.to_string());
            if let Some(cost) = cost {
                format!(
                    "Escape—{}, Exile {count_text} other cards from your graveyard",
                    cost.to_oracle()
                )
            } else {
                format!("Escape—Exile {count_text} other cards from your graveyard")
            }
        }
        AlternativeCastingMethod::Dash { cost } => format!("Dash {}", cost.to_oracle()),
        AlternativeCastingMethod::Bestow { total_cost } => {
            let costs = method.non_mana_costs();
            let mana_cost = total_cost
                .mana_cost()
                .map(|cost| cost.to_oracle())
                .unwrap_or_else(|| "{0}".to_string());
            if costs.is_empty() {
                format!("Bestow {mana_cost}")
            } else {
                let extra = capitalize_first(&describe_alternative_costs(&costs));
                format!("Bestow {mana_cost}, {extra}")
            }
        }
        other => {
            if other.name().eq_ignore_ascii_case("Parsed alternative cost") {
                if let Some(cost) = other.mana_cost() {
                    format!(
                        "You may pay {} rather than pay this spell's mana cost",
                        cost.to_oracle()
                    )
                } else {
                    "You may cast this spell rather than pay its mana cost".to_string()
                }
            } else if let Some(cost) = other.mana_cost() {
                format!(
                    "Alternative cast {}: {} {}",
                    idx + 1,
                    other.name(),
                    cost.to_oracle()
                )
            } else {
                format!("Alternative cast {}: {}", idx + 1, other.name())
            }
        }
    }
}

fn compiled_lines_inner(def: &CardDefinition) -> Vec<String> {
    let mut out = Vec::new();
    let mut alternative_cast_lines = Vec::new();
    let mut deferred_spell_optional_lines = Vec::new();
    let subject = subject_for_card(&def.card);
    let rewrite_it_deals = def.card.card_types.contains(&CardType::Creature)
        || def.card.card_types.contains(&CardType::Artifact)
        || def.card.card_types.contains(&CardType::Land)
        || def.card.card_types.contains(&CardType::Planeswalker)
        || def.card.card_types.contains(&CardType::Battle);
    let spell_like_card = def.card.card_types.contains(&CardType::Instant)
        || def.card.card_types.contains(&CardType::Sorcery);
    let has_attach_only_spell_effect = def.spell_effect.as_ref().is_some_and(|effects| {
        effects.len() == 1
            && effects[0]
                .downcast_ref::<crate::effects::AttachToEffect>()
                .is_some()
    });
    for (idx, method) in def.alternative_casts.iter().enumerate() {
        alternative_cast_lines.push(describe_alternative_cast_line(method, idx));
    }
    for cost in &def.optional_costs {
        let line = describe_optional_cost_line(cost);
        if spell_like_card && cost.label == "Conspire" {
            deferred_spell_optional_lines.push(line);
        } else {
            out.push(line);
        }
    }
    if let Some(filter) = &def.aura_attach_filter {
        out.push(format!("Enchant {}", describe_enchant_filter(filter)));
    }
    let max_saga_chapter = def.max_saga_chapter.or_else(|| {
        def.abilities
            .iter()
            .filter_map(|ability| {
                if let AbilityKind::Triggered(triggered) = &ability.kind {
                    triggered
                        .trigger
                        .saga_chapters()
                        .and_then(|chapters| chapters.iter().copied().max())
                } else {
                    None
                }
            })
            .max()
    });
    if let Some(max_chapter) = max_saga_chapter
        && let Some(roman) = chapter_number_to_roman(max_chapter)
    {
        out.push(format!(
            "(As this Saga enters and after your draw step, add a lore counter. Sacrifice after {roman}.)"
        ));
    }

    let push_abilities = |output: &mut Vec<String>| {
        let mut ability_idx = 0usize;
        while ability_idx < def.abilities.len() {
            let ability = &def.abilities[ability_idx];
            if let AbilityKind::Activated(first) = &ability.kind
                && first.is_mana_ability()
                && first.effects.is_empty()
                && first.activation_condition.is_none()
                && first.additional_restrictions.is_empty()
                && first.mana_usage_restrictions.is_empty()
                && first.mana_symbols().len() == 1
                && ability.text.is_none()
            {
                let mut symbols = vec![first.mana_symbols()[0]];
                let mut consumed = 1usize;
                while ability_idx + consumed < def.abilities.len() {
                    let next = &def.abilities[ability_idx + consumed];
                    let AbilityKind::Activated(next_mana) = &next.kind else {
                        break;
                    };
                    if !next_mana.is_mana_ability()
                        || !next_mana.effects.is_empty()
                        || next_mana.activation_condition.is_some()
                        || !next_mana.additional_restrictions.is_empty()
                        || !next_mana.mana_usage_restrictions.is_empty()
                        || next_mana.mana_symbols().len() != 1
                        || next_mana.mana_cost != first.mana_cost
                        || next.text.is_some()
                    {
                        break;
                    }
                    symbols.push(next_mana.mana_symbols()[0]);
                    consumed += 1;
                }
                if consumed > 1 {
                    let mut line = format!("Mana ability {}", ability_idx + 1);
                    let add = format!("Add {}", describe_mana_alternatives(&symbols));
                    if !first.mana_cost.costs().is_empty() {
                        let cost = describe_cost_list(first.mana_cost.costs());
                        line.push_str(": ");
                        line.push_str(&cost);
                        line.push_str(": ");
                        line.push_str(&add);
                    } else {
                        line.push_str(": ");
                        line.push_str(&add);
                    }
                    output.push(line);
                    ability_idx += consumed;
                    continue;
                }
            }
            output.extend(describe_ability(
                ability_idx + 1,
                ability,
                subject,
                rewrite_it_deals,
            ));
            ability_idx += 1;
        }
    };

    let additional_costs = def.additional_non_mana_costs();
    if !additional_costs.is_empty() {
        out.push(format!(
            "As an additional cost to cast this spell, {}",
            describe_additional_costs(&additional_costs)
        ));
    }
    if !spell_like_card {
        push_abilities(&mut out);
    }
    if let Some(spell_effects) = &def.spell_effect
        && !spell_effects.is_empty()
        && !(def.aura_attach_filter.is_some() && has_attach_only_spell_effect)
    {
        out.push(format!(
            "Spell effects: {}",
            describe_resolution_program_for_card(def, spell_effects)
        ));
    }
    out.extend(deferred_spell_optional_lines);
    if spell_like_card {
        push_abilities(&mut out);
    }
    if def.has_fuse {
        out.push("Fuse".to_string());
    }
    out.extend(alternative_cast_lines);
    out
}

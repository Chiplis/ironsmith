use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::CardDefinition;
use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::effect::{Condition, Effect, Value};
use crate::resolution::ResolutionProgram;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::Trigger;
use crate::zone::Zone;

fn overload_rewritten_text(text: &str) -> Option<String> {
    let mut rewritten_lines = Vec::new();
    let mut saw_overload = false;

    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.to_ascii_lowercase().starts_with("overload ") {
            saw_overload = true;
            continue;
        }
        rewritten_lines.push(crate::cards::builders::replace_whole_word_case_insensitive(
            line, "target", "each",
        ));
    }

    saw_overload.then(|| rewritten_lines.join("\n"))
}

fn finalize_overload_definitions(
    mut definition: CardDefinition,
    original_builder: &CardDefinitionBuilder,
    original_text: &str,
) -> Result<CardDefinition, CardTextError> {
    let Some(rewritten_text) = overload_rewritten_text(original_text) else {
        return Ok(definition);
    };

    if !definition
        .alternative_casts
        .iter()
        .any(|method| matches!(method, AlternativeCastingMethod::Overload { .. }))
    {
        return Ok(definition);
    }

    let overload_builder = original_builder.clone();
    let (overloaded_definition, _) =
        super::parse_text_with_annotations(overload_builder, rewritten_text, false)?;
    let overloaded_effects = overloaded_definition.spell_effect.unwrap_or_default();

    for method in &mut definition.alternative_casts {
        if let AlternativeCastingMethod::Overload { effects, .. } = method {
            *effects = overloaded_effects.to_vec();
        }
    }

    Ok(definition)
}

fn parse_backup_placeholder_amount(ability: &Ability) -> Option<u32> {
    let AbilityKind::Static(_) = &ability.kind else {
        return None;
    };

    let text = ability.text.as_deref()?.trim();
    let mut parts = text.split_whitespace();
    if !parts
        .next()
        .is_some_and(|part| part.eq_ignore_ascii_case("backup"))
    {
        return None;
    }
    parts.next()?.trim_end_matches(',').parse::<u32>().ok()
}

fn backup_granted_abilities_from_slice(abilities: &[Ability]) -> Vec<Ability> {
    abilities
        .iter()
        .filter(|ability| parse_backup_placeholder_amount(ability).is_none())
        .cloned()
        .collect()
}

fn is_cipher_placeholder(ability: &Ability) -> bool {
    let AbilityKind::Static(_) = &ability.kind else {
        return false;
    };

    ability
        .text
        .as_deref()
        .is_some_and(|text| text.trim().eq_ignore_ascii_case("Cipher"))
}

pub(crate) fn finalize_backup_abilities(mut definition: CardDefinition) -> CardDefinition {
    if !definition
        .abilities
        .iter()
        .any(|ability| parse_backup_placeholder_amount(ability).is_some())
    {
        return definition;
    }

    let original_abilities = definition.abilities.clone();
    definition.abilities = original_abilities
        .iter()
        .enumerate()
        .map(|(idx, ability)| {
            let Some(amount) = parse_backup_placeholder_amount(ability) else {
                return ability.clone();
            };

            let granted_abilities =
                backup_granted_abilities_from_slice(&original_abilities[idx + 1..]);
            Ability::triggered(
                Trigger::this_enters_battlefield(),
                vec![Effect::backup(amount, granted_abilities)],
            )
            .with_text(
                ability
                    .text
                    .as_deref()
                    .unwrap_or_else(|| original_abilities[idx].text.as_deref().unwrap_or("Backup")),
            )
        })
        .collect();
    definition
}

pub(crate) fn finalize_cipher_effects(mut definition: CardDefinition) -> CardDefinition {
    if !definition.abilities.iter().any(is_cipher_placeholder) {
        return definition;
    }

    definition
        .abilities
        .retain(|ability| !is_cipher_placeholder(ability));
    definition
        .spell_effect
        .get_or_insert_with(ResolutionProgram::default)
        .push(Effect::cipher());
    definition
}

fn finalize_squad_abilities(mut definition: CardDefinition) -> CardDefinition {
    if !definition
        .optional_costs
        .iter()
        .any(|cost| cost.label == "Squad")
    {
        return definition;
    }

    let squad_trigger = Ability::triggered(
        Trigger::this_enters_battlefield(),
        vec![Effect::new(crate::effects::CreateTokenCopyEffect::new(
            ChooseSpec::Source,
            Value::TimesPaidLabel("Squad".to_string()),
            PlayerFilter::You,
        ))],
    );
    definition.abilities.push(squad_trigger);
    definition
}

fn finalize_offspring_abilities(mut definition: CardDefinition) -> CardDefinition {
    if !definition
        .optional_costs
        .iter()
        .any(|cost| cost.label == "Offspring")
    {
        return definition;
    }

    let offspring_trigger = Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_enters_battlefield(),
            effects: ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::CreateTokenCopyEffect::new(
                    ChooseSpec::Source,
                    Value::WasPaidLabel("Offspring".to_string()),
                    PlayerFilter::You,
                )
                .set_base_power_toughness(1, 1),
            )]),
            choices: vec![],
            intervening_if: Some(Condition::ThisSpellPaidLabel("Offspring".to_string())),
        }),
        functional_zones: vec![Zone::Battlefield],
        text: None,
    };
    definition.abilities.push(offspring_trigger);
    definition
}

fn normalize_delayed_trigger_text(text: &str) -> String {
    text.to_ascii_lowercase()
        .replace('’', "'")
        .replace("'s", "s")
}

fn spell_battlefield_trigger_text_implies_delayed_schedule(
    ability_text: &str,
    trigger: &Trigger,
) -> Option<bool> {
    let normalized = normalize_delayed_trigger_text(ability_text);
    let trigger_text = normalize_delayed_trigger_text(trigger.display().as_str());

    let trigger_is_upkeep_or_end_step = trigger_text.contains("beginning of")
        && (trigger_text.contains("upkeep") || trigger_text.contains("end step"));
    if !trigger_is_upkeep_or_end_step {
        return None;
    }

    if normalized.contains("next upkeep") || normalized.contains("next turns upkeep") {
        return Some(true);
    }
    if normalized.contains("that turns end step")
        || normalized.contains("that players next upkeep")
        || normalized.contains("that players next end step")
        || normalized.contains("end step of that players next turn")
    {
        return Some(true);
    }
    if normalized.contains("next end step") || normalized.contains("next turns end step") {
        return Some(false);
    }

    None
}

fn convert_nonpermanent_delayed_triggered_ability_to_spell_effect(
    ability: &Ability,
) -> Option<Effect> {
    if ability.functional_zones.as_slice() != [Zone::Battlefield] {
        return None;
    }

    let AbilityKind::Triggered(triggered) = &ability.kind else {
        return None;
    };
    if !triggered.choices.is_empty() || triggered.intervening_if.is_some() {
        return None;
    }

    let ability_text = ability.text.as_deref()?;
    let start_next_turn =
        spell_battlefield_trigger_text_implies_delayed_schedule(ability_text, &triggered.trigger)?;

    let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
        triggered.trigger.clone(),
        triggered.effects.clone(),
        true,
        Vec::new(),
        PlayerFilter::You,
    );
    if start_next_turn {
        delayed = delayed.starting_next_turn();
    }

    Some(Effect::new(delayed))
}

fn finalize_nonpermanent_delayed_triggered_abilities(
    mut definition: CardDefinition,
) -> CardDefinition {
    if !definition.card.is_instant() && !definition.card.is_sorcery() {
        return definition;
    }

    let mut rewritten_effects = Vec::new();
    let mut remaining_abilities = Vec::with_capacity(definition.abilities.len());
    for ability in std::mem::take(&mut definition.abilities) {
        if let Some(effect) =
            convert_nonpermanent_delayed_triggered_ability_to_spell_effect(&ability)
        {
            rewritten_effects.push(effect);
        } else {
            remaining_abilities.push(ability);
        }
    }

    definition.abilities = remaining_abilities;
    if !rewritten_effects.is_empty() {
        definition
            .spell_effect
            .get_or_insert_with(ResolutionProgram::default)
            .extend(ResolutionProgram::from_effects(rewritten_effects));
    }
    definition
}

pub(crate) fn apply(
    definition: CardDefinition,
    original_builder: &CardDefinitionBuilder,
    original_text: &str,
) -> Result<CardDefinition, CardTextError> {
    let definition = finalize_overload_definitions(definition, original_builder, original_text)?;
    let definition = finalize_backup_abilities(definition);
    let definition = finalize_cipher_effects(definition);
    let definition = finalize_squad_abilities(definition);
    let definition = finalize_offspring_abilities(definition);
    Ok(finalize_nonpermanent_delayed_triggered_abilities(
        definition,
    ))
}

use crate::ability::{Ability, AbilityKind, TriggeredAbility};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::cards::CardDefinition;
use crate::cards::builders::{CardDefinitionBuilder, CardTextError};
use crate::cost::OptionalCostKind;
use crate::effect::{Condition, Effect, Value};
use crate::resolution::ResolutionProgram;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::{Trigger, TriggerKind};
use crate::zone::Zone;

use super::lex_patterns::{LexCaptureKind, LexCaptureRole, LexPattern};
use super::lexer::{
    LexedClause, lex_line, parser_token_word_refs, token_word_refs, word_slice_contains_any_phrase,
    word_slice_contains_phrase,
};

const BACKUP_PLACEHOLDER_PATTERN: LexPattern<'static> = LexPattern::new(&[
    LexPattern::word("backup"),
    LexPattern::amount("amount", LexCaptureKind::WordCount(1)),
]);

fn line_starts_with_keyword(line: &str, keyword: &str) -> bool {
    lex_line(line.trim_start(), 0).ok().is_some_and(|tokens| {
        parser_token_word_refs(&tokens)
            .first()
            .is_some_and(|word| *word == keyword)
    })
}

fn overload_rewritten_text(text: &str) -> Option<String> {
    let mut rewritten_lines = Vec::new();
    let mut saw_overload = false;

    for line in text.lines() {
        if line_starts_with_keyword(line, "overload") {
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
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return None;
    };

    let text = static_ability.display();
    let tokens = lex_line(text.trim(), 0).ok()?;
    let clause = LexedClause::new(&tokens);
    let matched = BACKUP_PLACEHOLDER_PATTERN.match_prefix(clause)?;
    let amount_clause = matched.capture_clause_by_role(LexCaptureRole::Amount, clause)?;
    amount_clause.word_refs().first()?.parse::<u32>().ok()
}

fn backup_granted_abilities_from_slice(abilities: &[Ability]) -> Vec<Ability> {
    abilities
        .iter()
        .filter(|ability| parse_backup_placeholder_amount(ability).is_none())
        .cloned()
        .collect()
}

fn is_cipher_placeholder(ability: &Ability) -> bool {
    let AbilityKind::Static(static_ability) = &ability.kind else {
        return false;
    };

    static_ability
        .display()
        .trim()
        .eq_ignore_ascii_case("Cipher")
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
        .any(|cost| matches!(cost.kind, OptionalCostKind::Squad))
    {
        return definition;
    }

    let squad_trigger = Ability::triggered(
        Trigger::this_enters_battlefield(),
        vec![Effect::new(crate::effects::CreateTokenCopyEffect::new(
            ChooseSpec::Source,
            Value::TimesPaidLabel("Squad".into()),
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
        .any(|cost| matches!(cost.kind, OptionalCostKind::Offspring))
    {
        return definition;
    }

    let offspring_trigger = Ability {
        kind: AbilityKind::Triggered(TriggeredAbility {
            trigger: Trigger::this_enters_battlefield(),
            effects: ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::CreateTokenCopyEffect::new(
                    ChooseSpec::Source,
                    Value::WasPaidLabel("Offspring".into()),
                    PlayerFilter::You,
                )
                .set_base_power_toughness(1, 1),
            )]),
            choices: vec![],
            intervening_if: Some(Condition::ThisSpellPaidLabel("Offspring".into())),
            presentation_label: None,
        }),
        functional_zones: vec![Zone::Battlefield],
    };
    definition.abilities.push(offspring_trigger);
    definition
}

const NEXT_UPKEEP_PHRASE: &[&str] = &["next", "upkeep"];
const NEXT_TURNS_UPKEEP_PHRASE: &[&str] = &["next", "turns", "upkeep"];
const NEXT_UPKEEP_PHRASES: &[&[&str]] = &[NEXT_UPKEEP_PHRASE, NEXT_TURNS_UPKEEP_PHRASE];
const THAT_TURNS_END_STEP_PHRASE: &[&str] = &["that", "turns", "end", "step"];
const THAT_PLAYERS_NEXT_UPKEEP_PHRASE: &[&str] = &["that", "players", "next", "upkeep"];
const THAT_PLAYERS_NEXT_END_STEP_PHRASE: &[&str] = &["that", "players", "next", "end", "step"];
const END_STEP_OF_THAT_PLAYERS_NEXT_TURN_PHRASE: &[&str] =
    &["end", "step", "of", "that", "players", "next", "turn"];
const THAT_TURN_DELAYED_STEP_PHRASES: &[&[&str]] = &[
    THAT_TURNS_END_STEP_PHRASE,
    THAT_PLAYERS_NEXT_UPKEEP_PHRASE,
    THAT_PLAYERS_NEXT_END_STEP_PHRASE,
    END_STEP_OF_THAT_PLAYERS_NEXT_TURN_PHRASE,
];
const NEXT_END_STEP_PHRASE: &[&str] = &["next", "end", "step"];
const NEXT_TURNS_END_STEP_PHRASE: &[&str] = &["next", "turns", "end", "step"];
const NEXT_END_STEP_PHRASES: &[&[&str]] = &[NEXT_END_STEP_PHRASE, NEXT_TURNS_END_STEP_PHRASE];
const YOUR_NEXT_UPKEEP_PHRASE: &[&str] = &["your", "next", "upkeep"];
const YOUR_NEXT_DRAW_STEP_PHRASE: &[&str] = &["your", "next", "draw", "step"];

fn is_upkeep_or_end_step_trigger(trigger: &Trigger) -> bool {
    matches!(
        trigger.kind,
        TriggerKind::BeginningOfUpkeep { .. } | TriggerKind::BeginningOfEndStep { .. }
    )
}

fn spell_battlefield_trigger_text_implies_delayed_schedule(
    ability_text: &str,
    trigger: &Trigger,
) -> Option<bool> {
    if !is_upkeep_or_end_step_trigger(trigger) {
        return None;
    }

    let tokens = lex_line(ability_text, 0).ok()?;
    let words = token_word_refs(&tokens);

    if word_slice_contains_any_phrase(&words, NEXT_UPKEEP_PHRASES) {
        return Some(true);
    }
    if word_slice_contains_any_phrase(&words, THAT_TURN_DELAYED_STEP_PHRASES) {
        return Some(true);
    }
    if word_slice_contains_any_phrase(&words, NEXT_END_STEP_PHRASES) {
        return Some(false);
    }

    None
}

fn convert_nonpermanent_delayed_triggered_ability_to_spell_effect(
    ability: &Ability,
    original_text: &str,
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

    let (ability_text, start_next_turn) = original_text.lines().find_map(|line| {
        let line = line.trim();
        let start_next_turn =
            spell_battlefield_trigger_text_implies_delayed_schedule(line, &triggered.trigger)?;
        Some((line, start_next_turn))
    })?;
    let trigger = delayed_trigger_spec_from_trigger(&triggered.trigger, Some(ability_text))?;

    let mut delayed = crate::effects::ScheduleDelayedTriggerEffect::new(
        trigger,
        triggered.effects.clone().to_vec(),
        true,
        Vec::new(),
        PlayerFilter::You,
    );
    if start_next_turn {
        delayed = delayed.starting_next_turn();
    }

    Some(Effect::new(delayed))
}

fn delayed_trigger_spec_from_trigger(
    trigger: &Trigger,
    ability_text: Option<&str>,
) -> Option<ironsmith_core::DelayedTriggerSpec> {
    let ability_tokens = ability_text
        .and_then(|text| lex_line(text, 0).ok())
        .unwrap_or_default();
    let ability_words = token_word_refs(&ability_tokens);

    match trigger.kind {
        TriggerKind::BeginningOfUpkeep { .. } => {
            let player = if word_slice_contains_phrase(&ability_words, YOUR_NEXT_UPKEEP_PHRASE) {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            };
            Some(ironsmith_core::DelayedTriggerSpec::BeginningOfUpkeep(
                player,
            ))
        }
        TriggerKind::BeginningOfDrawStep { .. } => {
            let player = if word_slice_contains_phrase(&ability_words, YOUR_NEXT_DRAW_STEP_PHRASE) {
                PlayerFilter::You
            } else {
                PlayerFilter::Any
            };
            Some(ironsmith_core::DelayedTriggerSpec::BeginningOfDrawStep(
                player,
            ))
        }
        TriggerKind::BeginningOfEndStep { .. } => Some(
            ironsmith_core::DelayedTriggerSpec::BeginningOfEndStep(PlayerFilter::Any),
        ),
        TriggerKind::EndOfCombat => Some(ironsmith_core::DelayedTriggerSpec::EndOfCombat),
        TriggerKind::ThisDies => Some(ironsmith_core::DelayedTriggerSpec::ThisDies),
        _ => None,
    }
}

fn finalize_nonpermanent_delayed_triggered_abilities(
    mut definition: CardDefinition,
    original_text: &str,
) -> CardDefinition {
    if !definition.card.is_instant() && !definition.card.is_sorcery() {
        return definition;
    }

    let mut rewritten_effects = Vec::new();
    let mut remaining_abilities = Vec::with_capacity(definition.abilities.len());
    for ability in std::mem::take(&mut definition.abilities) {
        if let Some(effect) =
            convert_nonpermanent_delayed_triggered_ability_to_spell_effect(&ability, original_text)
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
        original_text,
    ))
}

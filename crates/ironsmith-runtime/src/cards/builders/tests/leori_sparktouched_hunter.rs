#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Flying, vigilance\nWhenever Leori deals combat damage to a player, choose a planeswalker type. Until end of turn, whenever you activate an ability of a planeswalker of that type, copy that ability. You may choose new targets for the copies.";

fn nested_choose_subtype(effect: &Effect) -> Option<crate::effects::ChooseCreatureTypeEffect> {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseCreatureTypeEffect>() {
        return Some(choose.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_choose_subtype(child);
        }
    });
    found
}

fn nested_choose_card_type(effect: &Effect) -> Option<crate::effects::ChooseCardTypeEffect> {
    if let Some(choose) = effect.downcast_ref::<crate::effects::ChooseCardTypeEffect>() {
        return Some(choose.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_choose_card_type(child);
        }
    });
    found
}

fn nested_schedule(effect: &Effect) -> Option<crate::effects::ScheduleDelayedTriggerEffect> {
    if let Some(schedule) = effect.downcast_ref::<crate::effects::ScheduleDelayedTriggerEffect>() {
        return Some(schedule.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = nested_schedule(child);
        }
    });
    found
}

fn contains_ability_copy(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::CopySpellEffect>()
        .is_some_and(|copy| {
            copy.target_reference_kind == Some(crate::filter::StackObjectKind::Ability)
        })
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= contains_ability_copy(child);
    });
    found
}

fn contains_plural_copy_retarget(effect: &Effect) -> bool {
    if effect
        .downcast_ref::<crate::effects::RetargetStackObjectEffect>()
        .is_some_and(|retarget| retarget.copy_reference_plural)
    {
        return true;
    }
    let mut found = false;
    effect.visit_child_effects(&mut |child| {
        found |= contains_plural_copy_retarget(child);
    });
    found
}

#[test]
fn leori_preserves_chosen_planeswalker_type_and_plural_copy_provenance() {
    let definition = parse_oracle_card_definition("Leori, Sparktouched Hunter");
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);

    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Leori should have a combat-damage trigger");
    let effects = triggered.effects.flattened_default_effects();
    let choose = effects
        .iter()
        .find_map(|effect| nested_choose_subtype(effect))
        .expect("the trigger should choose one subtype family");
    assert_eq!(choose.family, crate::types::SubtypeFamily::Planeswalker);
    assert!(choose.excluded_subtypes.is_empty());

    let schedule = effects
        .iter()
        .find_map(|effect| nested_schedule(effect))
        .expect("the trigger should schedule the rest-of-turn activation trigger");
    assert!(schedule.until_end_of_turn);
    assert!(!schedule.one_shot);
    let activation = schedule
        .trigger
        .downcast_ref::<crate::triggers::AbilityActivatedTrigger>()
        .expect("the delayed trigger should watch activated abilities");
    assert_eq!(activation.activator, PlayerFilter::You);
    assert_eq!(activation.filter.card_types, [CardType::Planeswalker]);
    assert!(activation.filter.chosen_creature_type);

    let delayed = schedule.effects.flattened_default_effects();
    assert!(delayed.iter().any(contains_ability_copy));
    assert!(
        delayed.iter().any(contains_plural_copy_retarget),
        "the plural retarget must execute inside the repeating delayed trigger: {schedule:#?}"
    );
}

#[test]
fn planeswalker_card_type_choice_does_not_become_a_subtype_choice() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Object Choice Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Choose a planeswalker.")
        .expect("ordinary planeswalker object choice should parse");
    let effects = definition
        .spell_effect
        .as_ref()
        .expect("probe should be a spell")
        .flattened_default_effects();

    assert!(
        effects
            .iter()
            .all(|effect| nested_choose_subtype(effect).is_none())
    );
    let choose = effects
        .iter()
        .find_map(|effect| nested_choose_card_type(effect))
        .expect("an ordinary planeswalker noun should choose that card type");
    assert_eq!(choose.chooser, PlayerFilter::You);
    assert_eq!(choose.options, [CardType::Planeswalker]);
}

#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::{oracle_text_by_name, parse_oracle_card_definition};
use super::*;

fn assert_exact_oracle(name: &str, definition: &CardDefinition) {
    assert_eq!(
        canonical_compiled_lines(definition).join("\n"),
        oracle_text_by_name()[name]
    );
}

fn find_nested<T: Clone + 'static>(effect: &crate::effect::Effect) -> Option<T> {
    if let Some(found) = effect.downcast_ref::<T>() {
        return Some(found.clone());
    }
    let mut found = None;
    effect.visit_child_effects(&mut |child| {
        if found.is_none() {
            found = find_nested::<T>(child);
        }
    });
    found
}

#[test]
fn flamehold_registers_one_shot_next_spell_copy_instead_of_targeting_current_stack() {
    let definition = parse_oracle_card_definition("Flamehold Grappler");
    assert_exact_oracle("Flamehold Grappler", &definition);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Flamehold should have its ETB trigger");
    let schedule = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::ScheduleDelayedTriggerEffect>)
        .expect("ETB should schedule the next spell copy");
    assert!(schedule.one_shot && schedule.until_end_of_turn);
    assert!(
        format!("{:#?}", schedule.trigger).contains("SpellCastTrigger"),
        "{schedule:#?}"
    );
    let copy = schedule
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::CopySpellEffect>)
        .expect("the delayed payload should copy its triggering spell");
    assert!(matches!(copy.target.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering"));
    assert!(
        schedule
            .effects
            .segments
            .iter()
            .flat_map(|segment| &segment.default_effects)
            .any(|effect| find_nested::<crate::effects::ChooseNewTargetsEffect>(effect).is_some())
    );
}

#[test]
fn wiitigo_and_its_aura_shape_keep_distinct_combat_history_subjects() {
    let wiitigo = parse_oracle_card_definition("Wiitigo");
    assert_exact_oracle("Wiitigo", &wiitigo);
    let wiitigo_debug = format!("{wiitigo:#?}");
    assert!(
        wiitigo_debug.contains("SourceBlockedOrBecameBlockedSinceLastUpkeep"),
        "{wiitigo_debug}"
    );

    let aura = parse_oracle_card_definition("Shape of the Wiitigo");
    assert_exact_oracle("Shape of the Wiitigo", &aura);
    let aura_debug = format!("{aura:#?}");
    assert!(
        aura_debug.contains("EnchantedPermanentAttackedOrBlockedSinceLastUpkeep"),
        "{aura_debug}"
    );
    assert!(
        !aura_debug.contains("SourceBlockedOrBecameBlockedSinceLastUpkeep"),
        "{aura_debug}"
    );
}

#[test]
fn norin_keeps_the_exact_exiled_card_play_permission() {
    let definition = parse_oracle_card_definition("Norin, Swift Survivalist");
    assert_exact_oracle("Norin, Swift Survivalist", &definition);
    let grant = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => triggered
                .effects
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
                .find_map(find_nested::<crate::effects::GrantPlayTaggedEffect>),
            _ => None,
        })
        .expect("blocked-creature trigger should grant a tagged play permission");
    assert!(grant.allow_land);
    assert_eq!(
        grant.duration,
        crate::effects::GrantPlayTaggedDuration::UntilEndOfTurn
    );
    assert_eq!(
        grant.surface.and_then(|surface| surface.object),
        Some(ironsmith_core::GrantPlayTaggedObjectSurface::ThatCardFromExile)
    );
}

#[test]
fn felisa_uses_the_dying_creatures_last_known_counter_total() {
    let definition = parse_oracle_card_definition("Felisa, Fang of Silverquill");
    assert_exact_oracle("Felisa, Fang of Silverquill", &definition);
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered)
                if format!("{:#?}", triggered.trigger).contains("Dies") =>
            {
                Some(triggered)
            }
            _ => None,
        })
        .expect("Felisa should have a death trigger");
    let Condition::ValueComparison {
        left,
        operator,
        right,
    } = triggered
        .intervening_if
        .as_ref()
        .expect("the token creation should be counter-presence gated")
    else {
        panic!(
            "expected counter-presence comparison: {:#?}",
            triggered.intervening_if
        );
    };
    assert_eq!(
        *operator,
        crate::effect::ValueComparisonOperator::GreaterThanOrEqual
    );
    assert_eq!(right.unhinted(), &crate::effect::Value::Fixed(1));
    assert!(matches!(
        left.unhinted(),
        crate::effect::Value::CountersOn(spec, None)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
    ));
    let create = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::CreateTokenEffect>)
        .expect("successful branch should create Inklings");
    assert!(create.enters_tapped);
    assert!(matches!(
        create.count.unhinted(),
        crate::effect::Value::CountersOn(spec, None)
            if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == "triggering")
    ));
}

#[test]
fn melira_registers_a_one_shot_watcher_for_only_the_chosen_permanent() {
    let definition = parse_oracle_card_definition("Melira, the Living Cure");
    assert_exact_oracle("Melira, the Living Cure", &definition);
    let schedule = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Activated(activated) => activated
                .effects
                .segments
                .iter()
                .flat_map(|segment| &segment.default_effects)
                .find_map(find_nested::<crate::effects::ScheduleDelayedTriggerEffect>),
            _ => None,
        })
        .expect("Melira should register a delayed graveyard watcher");
    assert!(schedule.one_shot && schedule.until_end_of_turn);
    assert!(schedule.target_tag.is_some() || !schedule.target_objects.is_empty());
    let returned = schedule
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::MoveToZoneEffect>)
        .expect("the watched card should return to the battlefield");
    assert_eq!(returned.zone, Zone::Battlefield);
    assert_eq!(
        returned.battlefield_controller,
        crate::effects::BattlefieldController::Owner
    );
}

#[test]
fn scars_delays_one_counter_per_point_actually_prevented() {
    let definition = parse_oracle_card_definition("Scars of the Veteran");
    assert_exact_oracle("Scars of the Veteran", &definition);
    let program = definition
        .spell_effect
        .as_ref()
        .expect("Scars should have a spell program");
    let schedule = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::ScheduleDelayedTriggerEffect>)
        .expect("counter placement should be delayed until the next end step");
    assert!(schedule.one_shot);
    assert!(schedule.event_value_from_prior_prevention);
    let conditional = program
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::ConditionalEffect>)
        .expect("only a creature target should register the delayed counters");
    let mut creature_characteristic = crate::filter::ObjectFilter::default();
    creature_characteristic
        .card_types
        .push(crate::types::CardType::Creature);
    assert_eq!(
        conditional.condition,
        crate::effect::Condition::TargetMatches(creature_characteristic)
    );
    let put = schedule
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .find_map(find_nested::<crate::effects::PutCountersEffect>)
        .expect("delayed payload should put toughness counters");
    assert_eq!(
        put.counter_type,
        crate::object::CounterType::PlusZeroPlusOne
    );
    assert_eq!(
        put.amount.unhinted(),
        &crate::effect::Value::EventValue(crate::effect::EventValueSpec::Amount)
    );
    assert!(matches!(
        put.target.unhinted(),
        ChooseSpec::Tagged(tag) if tag.as_str() == "targeted_0"
    ));
}

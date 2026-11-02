//! cf8 p01: silent miscompiles where a remembered set or pronoun bound the
//! wrong object. Source-authored, deliberately unrun.
#[path = "p01_support/mod.rs"]
mod support;

fn spell_effects(definition: &ironsmith::cards::CardDefinition) -> Vec<ironsmith::effect::Effect> {
    fn collect(effect: &ironsmith::effect::Effect, all: &mut Vec<ironsmith::effect::Effect>) {
        all.push(effect.clone());
        effect.visit_child_effects(&mut |child| collect(child, all));
    }
    let mut all = Vec::new();
    for effect in definition.spell_effect.as_ref().unwrap().all_effects() {
        collect(effect, &mut all);
    }
    all
}

#[test]
fn aurelias_fury_taps_only_the_creatures_dealt_damage() {
    for definition in support::definitions("Aurelia's Fury") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Aurelia's Fury", &text);
        assert!(text.contains("tap each creature dealt damage this way"), "{text}");
        assert!(text.contains("for each player dealt damage this way"), "{text}");
        let debug = format!("{:?}", definition.spell_effect);
        // TapAll over the remembered recipients is restricted to creatures.
        let tap = debug.split("TapEffect").nth(1).expect("tap effect");
        assert!(tap.contains("Creature"), "{tap}");
    }
}

#[test]
fn hog_monkey_rampage_checks_the_creature_you_control() {
    for definition in support::definitions("Hog-Monkey Rampage") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Hog-Monkey Rampage", &text);
        let effects = spell_effects(&definition);
        let condition = effects.iter().find_map(|effect|
            effect.downcast_ref::<ironsmith::effects::ConditionalEffect>()).unwrap();
        let ironsmith::effect::Condition::TaggedObjectMatches(tag, filter) = &condition.condition else {
            panic!("expected a condition on the chosen creature");
        };
        assert!(filter.power.is_some());
        let declaration = effects.iter().filter_map(|effect|
            effect.downcast_ref::<ironsmith::effects::TaggedEffect>())
            .find(|effect| &effect.tag == tag).expect("condition must name one declared target");
        let target = declaration.effect.downcast_ref::<ironsmith::effects::TargetOnlyEffect>().unwrap();
        let ironsmith::target::ChooseSpec::Object(filter) = target.target.base() else { panic!("creature target") };
        assert_eq!(filter.controller, Some(ironsmith::target::PlayerFilter::You));
        let counters = effects.iter().find_map(|effect|
            effect.downcast_ref::<ironsmith::effects::PutCountersEffect>()).unwrap();
        assert!(matches!(counters.target.base(), ironsmith::target::ChooseSpec::Tagged(counter_tag) if counter_tag == tag));
    }
}

#[test]
fn stolen_uniform_watches_the_equipment_for_lost_control() {
    for definition in support::definitions("Stolen Uniform") {
        let text = support::rendered(&definition);
        support::assert_no_internal_markers("Stolen Uniform", &text);
        let debug = format!("{:?}", definition.spell_effect);
        assert!(debug.contains("ScheduleDelayedTriggerEffect"), "{debug}");
        assert!(debug.contains("ControlChangeTrigger"), "{debug}");
        assert!(debug.contains("attached_to_object: Some"), "{debug}");
        let effects = spell_effects(&definition);
        let delayed = effects.iter().find_map(|effect|
            effect.downcast_ref::<ironsmith::effects::ScheduleDelayedTriggerEffect>()).unwrap();
        assert!(delayed.until_end_of_turn, "the control-change watch must expire this turn");
        assert!(delayed.target_filter.as_ref().unwrap_or_else(|| panic!("missing watched filter: {delayed:#?}; full program: {debug}")).subtypes.contains(&ironsmith::Subtype::Equipment));
    }
}

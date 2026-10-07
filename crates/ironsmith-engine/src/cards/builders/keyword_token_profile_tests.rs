//! Authored native contracts; execution remains deferred.
use super::*;
use ironsmith_core::{TextChange, TokenNameTextRole, TokenTextRoles};

fn create_instruction(definition: &CardDefinition) -> Effect {
    fn collect(effect: &Effect, found: &mut Vec<Effect>) {
        if effect.downcast_ref::<crate::effects::CreateTokenEffect>().is_some() { found.push(effect.clone()); }
        effect.visit_child_effects(&mut |child| collect(child, found));
    }
    let mut found = Vec::new();
    for ability in &definition.abilities {
        if let AbilityKind::Triggered(triggered) = &ability.kind {
            for effect in triggered.effects.all_effects() { collect(effect, &mut found); }
        }
    }
    assert_eq!(found.len(), 1);
    found.pop().unwrap()
}

#[test]
fn every_native_keyword_factory_retains_implied_word_roles_and_normative_names() {
    let factories: [(&str, fn(CardDefinitionBuilder) -> CardDefinitionBuilder); 6] = [
        ("Spirit Token", |builder| builder.afterlife(2)),
        ("Servo Token", |builder| builder.fabricate(2)),
        ("Rebel Token", |builder| builder.for_mirrodin()),
        ("Hero Token", |builder| builder.job_select()),
        ("Phyrexian Germ Token", |builder| builder.living_weapon()),
        ("Warrior Token", |builder| builder.mobilize(2)),
    ];
    for (name, factory) in factories {
        let definition = factory(CardDefinitionBuilder::new(CardId::new(), "Keyword source")).build();
        let original = create_instruction(&definition);
        let native = original.downcast_ref::<crate::effects::CreateTokenEffect>().unwrap();
        assert_eq!(native.token.card.name, name);
        assert_eq!(native.token.card.name, ironsmith_core::subtype_derived_token_name(&native.token.card.subtypes).unwrap());
        assert_eq!(native.text_roles, Some(TokenTextRoles::rules_implied(TokenNameTextRole::SubtypeDerived, native.token.abilities.len())));
        let changed = original.with_text_change(TextChange::color(crate::color::Color::Red, crate::color::Color::Blue).unwrap()).unwrap()
            .with_text_change(TextChange::color(crate::color::Color::White, crate::color::Color::Green).unwrap()).unwrap();
        let changed = native.token.card.subtypes.iter().filter(|subtype| ironsmith_core::SubtypeFamily::Creature.all_subtypes().contains(subtype))
            .try_fold(changed, |effect, subtype| effect.with_text_change(TextChange::creature_type(*subtype, Subtype::Human).unwrap()))
            .unwrap();
        let changed = changed.downcast_ref::<crate::effects::CreateTokenEffect>().unwrap();
        assert_eq!(changed.token.card, native.token.card);
        assert_eq!(changed.token.abilities, native.token.abilities);
        assert_eq!(changed.count, native.count);
        assert_eq!(changed.controller, native.controller);
        assert_eq!(changed.enters_tapped, native.enters_tapped);
        assert_eq!(changed.enters_attacking, native.enters_attacking);
        assert_eq!(changed.sacrifice_at_next_end_step, native.sacrifice_at_next_end_step);
    }
}

#[test]
fn mobilize_keeps_implied_warriors_while_rewriting_its_authored_quantity_filter() {
    let filter = ObjectFilter { card_types: vec![CardType::Creature], subtypes: vec![Subtype::Warrior],
        controller: Some(PlayerFilter::You), ..ObjectFilter::default() };
    let definition = CardDefinitionBuilder::new(CardId::new(), "Dynamic mobilize")
        .mobilize_value(Value::Count(filter)).build();
    let original = create_instruction(&definition);
    let changed = original.with_text_change(TextChange::creature_type(Subtype::Warrior, Subtype::Human).unwrap()).unwrap();
    let changed = changed.downcast_ref::<crate::effects::CreateTokenEffect>().unwrap();
    let Value::Count(filter) = &changed.count else { panic!("authored quantity"); };
    assert_eq!(filter.subtypes, vec![Subtype::Human]);
    assert_eq!(changed.token.card.subtypes, vec![Subtype::Warrior]);
    assert_eq!(changed.token.card.name, "Warrior Token");
    assert_eq!(changed.token.card.colors(), ColorSet::RED);
    assert!(changed.enters_tapped && changed.enters_attacking && changed.sacrifice_at_next_end_step);
    let Value::Count(original_filter) = &original.downcast_ref::<crate::effects::CreateTokenEffect>().unwrap().count else { unreachable!() };
    assert_eq!(original_filter.subtypes, vec![Subtype::Warrior]);
}

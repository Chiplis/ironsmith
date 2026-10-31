use ironsmith_compiler::{
    CardDefinitionBuilder, ParseCardText,
    effect::{Effect, Value},
    ids::CardId,
    target::ChooseSpec,
    types::CardType,
};

#[test]
fn unless_payment_preserves_enclosing_object_reference() {
    fn payment(effect: &Effect) -> Option<ironsmith_compiler::effects::PayLifeEffect> {
        if let Some(unless) =
            effect.downcast_ref::<ironsmith_compiler::effects::UnlessPaysEffect<Effect>>()
        {
            for cost in unless.cost.costs() {
                if let Some(effect) = cost.effect_ref()
                    && let Some(pay) =
                        effect.downcast_ref::<ironsmith_compiler::effects::PayLifeEffect>()
                {
                    return Some(pay.clone());
                }
            }
        }
        let mut found = None;
        effect.visit_child_effects(&mut |child| {
            if found.is_none() {
                found = payment(child);
            }
        });
        found
    }
    for (reference, expected) in [
        ("its mana value", ChooseSpec::Iterated),
        ("that permanent's mana value", ChooseSpec::Iterated),
        ("this creature's mana value", ChooseSpec::Source),
    ] {
        let text = format!(
            "Menace\nWhen this creature enters, for each nonland permanent your opponents control, its controller sacrifices it unless they pay life equal to 2 plus {reference}."
        );
        let definition = CardDefinitionBuilder::new(CardId::new(), "Payment Reference Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(&text)
            .expect("quantified payment should compile");
        let triggered = definition
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                ironsmith_compiler::ability::AbilityKind::Triggered(ability) => Some(ability),
                _ => None,
            })
            .expect("entry trigger");
        let pay = triggered
            .effects
            .segments
            .iter()
            .flat_map(|segment| &segment.default_effects)
            .find_map(payment)
            .expect("life payment");
        let Value::Add(base, amount) = pay.amount.unhinted() else {
            panic!("{pay:#?}");
        };
        assert_eq!(base.unhinted(), &Value::Fixed(2));
        let Value::ManaValueOf(object) = amount.unhinted() else {
            panic!("{pay:#?}");
        };
        assert_eq!(
            object.base(),
            &expected,
            "incorrect binding for {reference}: {pay:#?}"
        );
    }

    let definition = CardDefinitionBuilder::new(CardId::new(), "Payment Choice Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("{0}: Choose a nonland permanent. Its controller sacrifices it unless they pay life equal to 2 plus that permanent's mana value.")
        .expect("chosen-object payment should compile");
    let activated = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            ironsmith_compiler::ability::AbilityKind::Activated(ability) => Some(ability),
            _ => None,
        })
        .expect("activated choice");
    let effects = activated.effects.flattened_default_effects();
    let chosen = effects
        .iter()
        .find_map(|effect| {
            effect.downcast_ref::<ironsmith_compiler::effects::ChooseObjectsEffect>()
        })
        .expect("object choice");
    let pay = effects.iter().find_map(payment).expect("life payment");
    let Value::Add(_, amount) = pay.amount.unhinted() else {
        panic!("{pay:#?}");
    };
    let Value::ManaValueOf(object) = amount.unhinted() else {
        panic!("{pay:#?}");
    };
    assert_eq!(
        object.base(),
        &ChooseSpec::Tagged(chosen.tag.clone()),
        "the payment must retain the chosen object's identity even when the source could be chosen"
    );
}

#[test]
fn unresolved_payment_reference_uses_announced_target_without_rebinding_explicit_source() {
    use ironsmith_compiler::{cost::TotalCost, costs::Cost, filter::ObjectFilter};
    use ironsmith_compiler_resolve::reference_helpers::resolve_total_cost_it_tags_with_enclosing_target;
    use ironsmith_compiler_semantic::model::reference_state::ReferenceEnv;

    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature()));
    let placeholder =
        ChooseSpec::Tagged(ironsmith_compiler_semantic::tag::CompilerReferenceTag::It.key());
    for (object, expected) in [
        (placeholder, target.clone()),
        (ChooseSpec::Source, ChooseSpec::Source),
    ] {
        let cost = TotalCost::from_cost(Cost::life(Value::ManaValueOf(Box::new(object))));
        let resolved = resolve_total_cost_it_tags_with_enclosing_target(
            &cost,
            &ReferenceEnv::default(),
            Some(&target),
        )
        .expect("payment reference should resolve");
        let [Cost::Life(Value::ManaValueOf(actual))] = resolved.costs() else {
            panic!("expected mana-value life cost: {resolved:#?}");
        };
        assert_eq!(actual.unhinted(), &expected);
    }
}

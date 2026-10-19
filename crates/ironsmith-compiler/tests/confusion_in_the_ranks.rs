use ironsmith_compiler::ParseCardText;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Effect;
use ironsmith_compiler::effects::{ExchangeControlEffect, TargetOnlyEffect};
use ironsmith_compiler::filter::{ObjectRef, PlayerFilter, TaggedOpbjectRelation};
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::target::ChooseSpec;
use ironsmith_compiler::triggers::TriggerKind;
use ironsmith_compiler::types::CardType;

#[test]
fn entering_permanents_controller_chooses_a_shared_type_exchange_target() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Confusion in the Ranks")
        .card_types(vec![CardType::Enchantment])
        .parse_text("Whenever an artifact, creature, or enchantment enters, its controller chooses target permanent another player controls that shares a card type with it. Exchange control of those permanents.")
        .expect("the entering-object exchange should compile");
    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected an executable triggered ability");
    };
    let TriggerKind::ZoneChange(change) = &triggered.trigger.kind else {
        panic!("expected an enters-battlefield trigger");
    };
    assert_eq!(change.to, Some(ironsmith_compiler::zone::Zone::Battlefield));
    assert!(change.from.is_none() && change.from_zones.is_none());
    let filter = change.filter.as_ref().expect("expected an entering-object filter");
    assert_eq!(
        filter.card_types,
        vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment
        ]
    );
    assert!(
        filter.controller.is_none(),
        "entries under any player's control trigger"
    );

    fn collect(effect: &Effect, effects: &mut Vec<Effect>) {
        effects.push(effect.clone());
        effect.visit_child_effects(&mut |child| collect(child, effects));
    }
    let mut effects = Vec::new();
    for effect in triggered.effects.flattened_default_effects() {
        collect(effect, &mut effects);
    }
    let declarations: Vec<_> = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<TargetOnlyEffect>())
        .collect();
    let [declaration] = declarations.as_slice() else {
        panic!("expected one target declaration: {effects:#?}");
    };
    let Some(PlayerFilter::ControllerOf(ObjectRef::Tagged(entering_tag))) = &declaration.chooser
    else {
        panic!("the entering permanent's controller must choose: {declaration:#?}");
    };
    let ChooseSpec::Target(target) = declaration.target.unhinted() else {
        panic!("the exchange counterpart must be targeted");
    };
    let ChooseSpec::Object(target_filter) = target.unhinted() else {
        panic!("expected a permanent target filter");
    };
    assert!(
        matches!(target_filter.controller.as_ref(), Some(PlayerFilter::Excluding { base, excluded })
        if matches!(base.as_ref(), PlayerFilter::Any)
            && matches!(excluded.as_ref(), PlayerFilter::ControllerOf(ObjectRef::Tagged(tag)) if tag == entering_tag))
    );
    assert!(
        target_filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag == *entering_tag
                && constraint.relation == TaggedOpbjectRelation::SharesCardType)
    );
    let exchanges: Vec<_> = effects
        .iter()
        .filter_map(|effect| effect.downcast_ref::<ExchangeControlEffect>())
        .collect();
    let [exchange] = exchanges.as_slice() else {
        panic!("expected one exchange effect: {effects:#?}");
    };
    assert!(matches!(&exchange.permanent1, ChooseSpec::Tagged(tag) if tag == entering_tag));
    assert!(matches!(&exchange.permanent2, ChooseSpec::Tagged(tag) if tag != entering_tag));
    assert_eq!(
        triggered.choices.len(),
        1,
        "only the counterpart is a target"
    );
}

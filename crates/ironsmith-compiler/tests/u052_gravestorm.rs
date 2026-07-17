use ironsmith_compiler::Zone;
use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Value;
use ironsmith_compiler::effects::{ChooseNewTargetsEffect, CopySpellEffect, WithIdEffect};
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::target::ChooseSpec;
use ironsmith_compiler::triggers::TriggerKind;
use ironsmith_compiler::types::CardType;
use ironsmith_core::TurnHistoryCount;

#[test]
fn gravestorm_instances_lower_to_independent_stack_triggers() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Double Gravestorm Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text("Gravestorm\nGravestorm")
        .expect("Gravestorm should compile");

    assert_eq!(definition.abilities.len(), 2, "{:#?}", definition.abilities);
    for ability in &definition.abilities {
        let AbilityKind::Triggered(triggered) = &ability.kind else {
            panic!("Gravestorm must be executable: {ability:#?}");
        };
        assert_eq!(triggered.trigger.kind, TriggerKind::YouCastThisSpell);
        assert_eq!(ability.functional_zones, vec![Zone::Stack]);

        let [copy, retarget] = triggered.effects.flattened_default_effects() else {
            panic!("Gravestorm should copy then offer new targets: {triggered:#?}");
        };
        let copy = copy
            .downcast_ref::<WithIdEffect>()
            .expect("copy result must be tagged for retargeting");
        let copy_spell = copy
            .effect
            .downcast_ref::<CopySpellEffect>()
            .expect("Gravestorm should copy its source spell");
        assert!(matches!(copy_spell.target, ChooseSpec::Source));
        assert_eq!(
            copy_spell.count,
            Value::TurnHistoryCount(TurnHistoryCount::Died(Default::default()))
        );

        let retarget = retarget
            .downcast_ref::<ChooseNewTargetsEffect>()
            .expect("Gravestorm should offer new targets for its copies");
        assert!(retarget.may);
        assert_eq!(retarget.from_effect, copy.id);
    }
}

#[test]
fn gravestorm_reminder_text_does_not_create_a_duplicate_ability() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Bitter Ordeal")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Gravestorm (When you cast this spell, copy it for each permanent put into a graveyard from the battlefield this turn. You may choose new targets for the copies.)",
        )
        .expect("printed Gravestorm reminder text should compile");

    assert_eq!(definition.abilities.len(), 1, "{:#?}", definition.abilities);
}

use ironsmith_compiler::ability::AbilityKind;
use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::triggers::{TriggerKind, TriggerTimingRestriction};
use ironsmith_compiler::types::CardType;

#[test]
fn spell_cast_during_combat_lowers_to_a_typed_timing_restriction() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Timing Probe")
        .card_types(vec![CardType::Creature])
        .parse_text("Whenever you cast a spell during combat, draw a card.")
        .expect("combat-scoped spell-cast trigger should compile");

    let AbilityKind::Triggered(triggered) = &definition.abilities[0].kind else {
        panic!("expected a triggered ability: {:#?}", definition.abilities);
    };
    let TriggerKind::SpellCastQualified { timing, .. } = &triggered.trigger.kind else {
        panic!(
            "expected a qualified spell-cast trigger: {:#?}",
            triggered.trigger
        );
    };

    assert_eq!(*timing, Some(TriggerTimingRestriction::DuringCombat));
}

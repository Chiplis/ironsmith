use super::*;

fn rendered(text: &str, card_type: CardType) -> String {
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Equalization Probe",
    )
    .card_types(vec![card_type])
    .parse_text(text)
    .expect("least-count equalization should compile");
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn least_count_equalization_keeps_selected_sets_and_compacts_followups() {
    let balance = "Each player chooses a number of lands they control equal to the number of lands controlled by the player who controls the fewest, then sacrifices the rest. Players discard cards and sacrifice creatures the same way.";
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(), "Equalization typed program probe",
    )
    .card_types(vec![CardType::Sorcery])
    .parse_text(balance)
    .expect("balance program compiles");
    let program = definition.spell_effect.as_ref().expect("spell program");
    assert!(super::super::effect_lists::describe_least_count_equalization(
        program.flattened_default_effects(),
    ).is_some(), "typed equalization steps must be present: {program:#?}");
    assert_eq!(rendered(balance, CardType::Sorcery), balance);

    let magus = format!("{{4}}{{W}}, {{T}}, Sacrifice this creature: {balance}");
    assert_eq!(rendered(&magus, CardType::Creature), magus);

    let balancing_act = "Each player chooses a number of permanents they control equal to the number of permanents controlled by the player who controls the fewest, then sacrifices the rest. Each player discards cards the same way.";
    assert_eq!(rendered(balancing_act, CardType::Sorcery), balancing_act);

    let restore = "Suspend 6—{W}\nEach player chooses a number of lands they control equal to the number of lands controlled by the player who controls the fewest, then sacrifices the rest. Players sacrifice creatures and discard cards the same way.";
    assert_eq!(rendered(restore, CardType::Sorcery), restore);
}

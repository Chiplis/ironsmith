use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Value;
use ironsmith_compiler::effects::ChooseModeEffect;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::types::CardType;

fn compile_spree_probe() -> ironsmith_compiler::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), "Spree Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Spree (Choose one or more additional costs.)\n\
             + {1}{U} — Counter target spell.\n\
             + {2} — Draw two cards.",
        )
        .expect("Spree modal block should compile")
}

#[test]
fn spree_lowers_plus_modes_to_typed_mandatory_additional_costs() {
    let definition = compile_spree_probe();
    let modal = definition
        .spell_effect
        .as_ref()
        .and_then(|program| {
            program
                .all_effects()
                .into_iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("Spree should lower to a typed modal effect");

    assert!(modal.spree);
    assert_eq!(modal.min_choose_count, Value::Fixed(1));
    assert_eq!(modal.choose_count, Value::Fixed(2));
    assert!(!modal.allow_repeated_modes);
    assert_eq!(modal.modes.len(), 2);
    assert_eq!(
        modal
            .mode_additional_mana_costs
            .iter()
            .map(|cost| cost.to_oracle())
            .collect::<Vec<_>>(),
        ["{1}{U}", "{2}"]
    );
}

#[test]
fn spree_mode_source_text_excludes_the_cost_label() {
    let definition = compile_spree_probe();
    let modal = definition
        .spell_effect
        .as_ref()
        .and_then(|program| {
            program
                .all_effects()
                .into_iter()
                .find_map(|effect| effect.downcast_ref::<ChooseModeEffect>())
        })
        .expect("Spree should lower to a typed modal effect");

    assert_eq!(modal.modes[0].source_text, "Counter target spell");
    assert_eq!(modal.modes[1].source_text, "Draw two cards");
}

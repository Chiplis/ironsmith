use ironsmith_compiler::cards::CardDefinitionBuilder;
use ironsmith_compiler::effect::Value;
use ironsmith_compiler::ids::CardId;
use ironsmith_compiler::types::CardType;

#[test]
fn kicked_choose_any_number_lowers_to_typed_conditional_mode_range() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Conditional Mode Probe")
        .card_types(vec![CardType::Instant])
        .parse_text(
            "Kicker {2}{G}\n\
             Choose one. If this spell was kicked, choose any number instead.\n\
             • You gain 1 life.\n\
             • You gain 2 life.\n\
             • You gain 3 life.",
        )
        .expect("conditional modal header should compile");
    let modal = definition
        .spell_effect
        .as_ref()
        .and_then(|program| {
            program.all_effects().into_iter().find_map(|effect| {
                effect.downcast_ref::<ironsmith_compiler::effects::ChooseModeEffect>()
            })
        })
        .unwrap_or_else(|| {
            panic!(
                "spell should contain a typed modal effect: {:#?}",
                definition.spell_effect
            )
        });
    let range = modal
        .conditional_mode_range
        .as_ref()
        .expect("kicker-dependent range should be retained structurally");

    assert!(range.required_optional_cost.eq_ignore_ascii_case("Kicker"));
    assert_eq!(range.min_modes, Value::Fixed(0));
    assert_eq!(range.max_modes, Value::Fixed(3));
}

#[test]
fn trailing_instead_condition_lowers_to_typed_presentation_order() {
    let definition = CardDefinitionBuilder::new(CardId::new(), "Trailing Instead Probe")
        .card_types(vec![CardType::Sorcery])
        .parse_text(
            "Search your library for a basic land card, reveal it, put it into your hand, then shuffle.\n\
             You may put that card onto the battlefield instead of putting it into your hand if a creature died this turn.",
        )
        .expect("cross-line self-replacement should compile");
    let branch = definition
        .spell_effect
        .as_ref()
        .and_then(|program| program.segments.first())
        .and_then(|segment| segment.self_replacements.first())
        .expect("replacement should attach to the search resolution segment");

    assert!(branch.condition_after_replacement, "{branch:#?}");
    assert!(branch.starts_new_source_line, "{branch:#?}");
}

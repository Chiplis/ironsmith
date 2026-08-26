use super::*;

#[test]
fn power_sink_keeps_its_nonpayment_result_linked_to_the_spell_controller() {
    let oracle = "Counter target spell unless its controller pays {X}. If that player doesn't, they tap all lands with mana abilities they control and lose all unspent mana.";
    let definition = crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Power Sink")
        .card_types(vec![CardType::Instant])
        .parse_text(oracle)
        .expect("Power Sink should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

#[test]
fn trailing_instead_is_not_part_of_the_counter_target_antecedent() {
    let oracle = "Counter target noncreature spell unless its controller pays {1}.\nFerocious — If you control a creature with power 4 or greater, counter that spell instead.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Stubborn Denial Probe")
            .card_types(vec![CardType::Instant])
            .parse_text(oracle)
            .expect("counter-unless self-replacement should compile");

    let rendered = crate::compiled_text::compiled_text_lines(&definition).join("\n");
    assert_eq!(
        rendered,
        "Counter target noncreature spell unless its controller pays {1}.\nFerocious — If you control a creature with power 4 or greater, instead counter that spell."
    );
    assert!(!rendered.contains("Instead."));

    let [segment] = definition
        .spell_effect
        .as_ref()
        .expect("spell effect")
        .segments
        .as_slice()
    else {
        panic!("expected one resolution segment");
    };
    let [branch] = segment.self_replacements.as_slice() else {
        panic!("expected one ferocious replacement");
    };
    let [replacement] = branch.replacement_effects.as_slice() else {
        panic!("expected one replacement effect");
    };
    let tagged = replacement
        .downcast_ref::<crate::effects::TaggedEffect>()
        .expect("counter replacement keeps its result tag");
    let counter = tagged
        .effect
        .downcast_ref::<crate::effects::CounterEffect>()
        .expect("replacement counters the original target");
    assert_eq!(
        counter.target.source_reference_surface(),
        Some(&crate::target::SourceReferenceSurface::ThisPermanentType(
            "that spell".to_string()
        ))
    );
}

#[test]
fn conditional_behold_uncounterability_precedes_spell_resolution() {
    let oracle = "As an additional cost to cast this spell, you may reveal a Dragon card from your hand.\nIf you revealed a Dragon card or controlled a Dragon as you cast this spell, this spell can't be countered.\nDraw four cards.";
    let definition = crate::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Conditional Uncounterability Probe",
    )
    .card_types(vec![CardType::Instant])
    .parse_text(oracle)
    .expect("behold-or-control conditional uncounterability should compile");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

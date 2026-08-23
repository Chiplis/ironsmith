use super::*;

#[test]
fn exile_target_renders_nested_historical_block_partner_filter() {
    let mut creature = ObjectFilter::creature();
    creature.blocked_or_was_blocked_by_this_turn = Some(Box::new(
        ObjectFilter::creature().with_subtype(Subtype::Zombie),
    ));
    let effect = Effect::exile(ChooseSpec::target(ChooseSpec::Object(creature)));

    assert_eq!(
        describe_effect(&effect),
        "Exile target creature that blocked or was blocked by a Zombie this turn"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn time_to_reflect_round_trips_the_historical_combat_partner_relation() {
    let oracle = "Exile target creature that blocked or was blocked by a Zombie this turn.";
    let definition = crate::cards::builders::CardDefinitionBuilder::new(
        crate::ids::CardId::new(),
        "Time to Reflect",
    )
    .card_types(vec![CardType::Instant])
    .parse_text(oracle)
    .expect("historical combat partner filter should parse");

    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        oracle
    );
}

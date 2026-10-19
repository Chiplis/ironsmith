use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_typed_base_characteristics() {
    let power = lex_line("until end of turn, target creature has base power X", 0).unwrap();
    assert_eq!(
        parse_base_power_clause_shape(&power)
            .unwrap()
            .unwrap()
            .power,
        Value::X
    );

    let pt = lex_line("target creature has base power and toughness 3/4", 0).unwrap();
    let shape = parse_base_power_toughness_clause_shape(&pt)
        .unwrap()
        .unwrap();
    assert_eq!(shape.power, Value::Fixed(3));
    assert_eq!(shape.toughness, Value::Fixed(4));
    assert_eq!(shape.duration, Until::Forever);
    assert!(shape.where_x_tokens.is_none());

    let permanent_multi_target = lex_line(
        "any number of target Shapeshifter creatures you control have base power and toughness 4/4",
        0,
    )
    .unwrap();
    let shape = parse_base_power_toughness_clause_shape(&permanent_multi_target)
        .unwrap()
        .unwrap();
    assert_eq!(shape.duration, Until::Forever);

    let pt = lex_line(
        "until your next turn, creatures target player controls have base power and toughness 1/1",
        0,
    )
    .unwrap();
    let shape = parse_base_power_toughness_clause_shape(&pt)
        .unwrap()
        .unwrap();
    assert_eq!(shape.duration, Until::YourNextTurn);
    assert!(shape.where_x_tokens.is_none());
    assert_eq!(
        render_token_slice(shape.target_tokens),
        "creatures target player controls"
    );

    let dynamic = lex_line(
            "until end of turn, creatures you control have base power and toughness X/X, where X is the number of cards in your graveyard",
            0,
        )
        .unwrap();
    let shape = parse_base_power_toughness_clause_shape(&dynamic)
        .unwrap()
        .unwrap();
    assert_eq!(shape.power, Value::X);
    assert_eq!(shape.toughness, Value::X);
    assert_eq!(shape.duration, Until::EndOfTurn);
    assert_eq!(
        render_token_slice(shape.where_x_tokens.unwrap()),
        "where X is the number of cards in your graveyard"
    );
}

#[test]
fn dynamic_base_axes_share_source_and_duration() {
    let tokens = lex_line("Until your next turn, this creature's base power becomes twice that card's power and its base toughness becomes twice that card's toughness.", 0).unwrap();
    let shape = parse_base_power_toughness_clause_shape(&tokens).unwrap().unwrap();
    assert_eq!(shape.duration, Until::YourNextTurn);
    assert!(matches!(shape.power, Value::Scaled(ref value, 2) if matches!(value.as_ref(), Value::PowerOf(_))));
    assert!(matches!(shape.toughness, Value::Scaled(ref value, 2) if matches!(value.as_ref(), Value::ToughnessOf(_))));
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens).unwrap();
    assert_eq!(effects.len(), 1, "the same subject owns both axes");
    let crate::cards::builders::EffectAst::SubjectVerb(crate::cards::builders::SubjectVerbEffectAst {
        action: crate::cards::builders::SubjectVerbActionAst::Characteristics(crate::cards::builders::CharacteristicActionAst::SetBasePowerToughness { target, duration, .. }), ..
    }) = &effects[0] else { panic!("expected combined characteristic change: {effects:?}"); };
    assert!(matches!(target, crate::cards::builders::TargetAst::Source(_)));
    assert_eq!(*duration, Until::YourNextTurn);
}

#[test]
fn historical_recipient_base_assignment_keeps_its_filter_and_permanent_duration() {
    for text in [
        "Change the base power and toughness of all creatures that dealt damage to it this turn to 0/2.",
        "The base power and toughness of each creature that dealt damage to it this turn become 0/2.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let shape = parse_base_power_toughness_clause_shape(&tokens).unwrap().unwrap();
        assert_eq!(shape.duration, Until::Forever);
        assert_eq!(shape.power, Value::Fixed(0));
        assert_eq!(shape.toughness, Value::Fixed(2));
        let filter = crate::object_filters::parse_object_filter_lexed(shape.target_tokens, false).unwrap();
        assert!(filter.tagged_constraints.iter().any(|constraint|
            constraint.relation == crate::target::TaggedOpbjectRelation::DealtDamageToTaggedThisTurn { combat_only: false }));
        let effects = crate::effect_sentences::parse_effect_chain(&tokens).unwrap();
        let debug = format!("{effects:?}");
        assert!(debug.contains("DealtDamageToTaggedThisTurn") && debug.contains("Forever"), "complete chain lost the history filter or duration: {debug}");
        assert!(!debug.contains("EndOfTurn"), "history is not an effect duration: {debug}");
    }
}

#[test]
fn changing_base_pair_copies_both_axes_of_the_referenced_creature() {
    for text in [
        "change this creature's base power and toughness to that creature's power and toughness until end of turn",
        "change the base power and toughness of target Human you control to this creature's power and toughness until end of turn",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let shape = parse_base_power_toughness_clause_shape(&tokens).unwrap().expect(text);
        assert_eq!(shape.duration, Until::EndOfTurn);
        let Value::PowerOf(power) = shape.power.unhinted() else { panic!("{:?}", shape.power); };
        let Value::ToughnessOf(toughness) = shape.toughness.unhinted() else { panic!("{:?}", shape.toughness); };
        assert_eq!(power, toughness);
        assert_eq!(crate::effect_sentences::parse_effect_sentence_lexed(&tokens).unwrap().len(), 1);
    }
}

#[test]
fn changing_one_base_axis_without_duration_is_permanent() {
    let tokens = lex_line("change this creature's base toughness to 1 plus the number of creature cards in your graveyard", 0).unwrap();
    let shape = parse_base_toughness_clause_shape(&tokens).unwrap().expect("dynamic base toughness");
    assert_eq!(shape.duration, Until::Forever);
    assert!(matches!(shape.power.unhinted(), Value::Add(_, _)), "{:?}", shape.power);
    assert_eq!(crate::effect_sentences::parse_effect_sentence_lexed(&tokens).unwrap().len(), 1);
}


#[test]
fn changing_one_base_axis_preserves_aggregate_scope_and_duration() {
    for (axis, duration_text, duration) in [
        ("power", " until end of turn", Until::EndOfTurn),
        ("toughness", " until your next turn", Until::YourNextTurn),
        ("power", "", Until::Forever),
    ] {
        let tokens = lex_line(&format!(
            "Change this creature's base {axis} to 1 plus the greatest power among other creatures you control{duration_text}."
        ), 0).unwrap();
        let shape = if axis == "power" {
            parse_base_power_clause_shape(&tokens)
        } else {
            parse_base_toughness_clause_shape(&tokens)
        }.unwrap().expect("aggregate base assignment");
        assert_eq!(shape.duration, duration);
        let Value::Add(one, greatest) = shape.power.unhinted() else {
            panic!("expected sum: {:?}", shape.power);
        };
        assert_eq!(one.unhinted(), &Value::Fixed(1));
        let Value::GreatestPower(filter) = greatest.unhinted() else {
            panic!("expected greatest power: {greatest:?}");
        };
        assert!(filter.other);
        assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
        let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens).unwrap();
        let debug = format!("{effects:?}");
        assert_eq!(effects.len(), 1, "{debug}");
        assert!(debug.contains(if axis == "power" { "SetBasePower" } else { "SetBaseToughness" }), "{debug}");
        assert!(!debug.contains("SetBasePowerToughness"), "{debug}");
    }
}

#[test]
fn changing_one_base_axis_rejects_incomplete_quantity_or_duration() {
    for text in [
        "Change this creature's base power to 1 plus until end of turn.",
        "Change this creature's base power to 1 mystery until end of turn.",
        "Change this creature's base power to 1 plus the greatest power among other creatures you control until mystery.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(parse_base_power_clause_shape(&tokens).unwrap().is_none(), "{text}");
    }
}

#[test]
fn changing_one_base_axis_declares_a_separate_characteristic_target() {
    let tokens = lex_line("Change this creature's base toughness to 1 plus the power of target creature blocking or blocked by this creature.", 0).unwrap();
    let effects = crate::effect_sentences::parse_effect_sentence_lexed(&tokens).unwrap();
    let debug = format!("{effects:?}");
    assert!(debug.contains("SetBaseToughness"), "{debug}");
    assert!(debug.contains("Target"), "{debug}");
    assert!(debug.contains("Forever"), "{debug}");
    assert!(!debug.contains("SetBasePowerToughness"), "{debug}");
}

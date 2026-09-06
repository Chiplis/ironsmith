use super::*;
use crate::lexer::lex_line;

#[test]
fn power_damage_does_not_absorb_an_earlier_action_as_its_source() {
    for text in [
        "Put a +1/+1 counter on target creature you control, then it deals damage equal to its power to target creature an opponent controls.",
        "Untap target creature, then it deals damage equal to its power to another target creature.",
    ] {
        let tokens = lex_line(text, 0).unwrap();
        assert!(parse_power_damage_shape(&tokens).unwrap().is_none());
    }
}

#[test]
fn parses_power_damage_and_fight_shapes() {
    let damage = lex_line("It deals damage to each opponent equal to its power.", 0).unwrap();
    let shape = parse_power_damage_shape(&damage).unwrap().unwrap();
    assert!(shape.source_is_tagged);
    assert!(matches!(shape.target, PowerDamageTargetShape::EachOpponent));

    let ordinary_damage = lex_line("This spell deals 2 damage to each other player.", 0).unwrap();
    assert!(
        parse_power_damage_shape(&ordinary_damage)
            .unwrap()
            .is_none(),
        "ordinary fixed damage must reach generic subject-verb lowering"
    );

    let fight = lex_line("Target creature fights one another.", 0).unwrap();
    assert!(parse_fight_shape(&fight).unwrap().right_is_tagged_other);

    let divided = lex_line(
        "That creature deals damage equal to its power divided as its controller chooses among any number of those Wolves.",
        0,
    )
    .unwrap();
    assert!(
        parse_power_damage_shape(&divided).unwrap().is_none(),
        "distributed damage must reach the divided-damage parser"
    );
}

#[test]
fn possessive_power_binds_to_the_same_clause_damage_source() {
    let damage = lex_line(
        "Target creature you control deals damage equal to its power to up to one target creature you don't control.",
        0,
    )
    .unwrap();
    let shape = parse_power_damage_shape(&damage).unwrap().unwrap();

    assert!(matches!(
        shape.amount,
        Value::PowerOf(spec)
            if matches!(spec.base(), crate::target::ChooseSpec::Source)
                && spec.source_reference_surface().is_some()
    ));
}

#[test]
fn possessive_power_offset_keeps_the_full_amount_and_target_tail() {
    let damage = lex_line(
        "It deals damage equal to its power plus 2 to another target creature.",
        0,
    )
    .unwrap();
    let shape = parse_power_damage_shape(&damage).unwrap().unwrap();

    assert!(matches!(
        shape.amount,
        Value::Add(left, right)
            if matches!(
                left.as_ref(),
                Value::PowerOf(spec)
                    if matches!(spec.base(), crate::target::ChooseSpec::Source)
                        && spec.source_reference_surface().is_some()
            ) && matches!(right.as_ref(), Value::Fixed(2))
    ));
    assert!(matches!(
        shape.target,
        PowerDamageTargetShape::Tokens(tokens)
            if TokenWordView::new(tokens).to_word_refs()
                == ["another", "target", "creature"]
    ));
}

#[test]
fn possessive_toughness_binds_to_each_member_of_a_referenced_pair() {
    let damage = lex_line(
        "Each of those creatures deals damage equal to its toughness to the other.",
        0,
    )
    .unwrap();
    let shape = parse_power_damage_shape(&damage).unwrap().unwrap();

    assert!(shape.source_is_tagged);
    assert!(matches!(
        shape.amount,
        Value::ToughnessOf(spec)
            if matches!(spec.base(), crate::target::ChooseSpec::Source)
                && spec.source_reference_surface().is_some()
    ));
    assert!(matches!(
        shape.target,
        PowerDamageTargetShape::Tokens(tokens)
            if TokenWordView::new(tokens).to_word_refs() == ["the", "other"]
    ));
}

#[test]
fn parses_retarget_repeat_and_combat_requirement_shapes() {
    let retarget = lex_line(
        "Choose new targets for the copy of that spell with a single target.",
        0,
    )
    .unwrap();
    let split = super::super::split_choose_new_targets_clause_lexed(&retarget).unwrap();
    assert_eq!(
        parse_retarget_reference_shape(split.target_tokens),
        Some(RetargetReferenceShape::Copy)
    );
    assert_eq!(
        parse_retarget_constraint_shapes(split.target_tokens),
        vec![RetargetConstraintShape::SingleTarget]
    );

    let repeat = lex_line("And you may repeat this process any number of times.", 0).unwrap();
    assert_eq!(
        parse_repeat_process_shape(&repeat),
        Some(RepeatProcessShape::May)
    );

    let attack = lex_line("Target creature attacks this turn if able.", 0).unwrap();
    let shape = parse_combat_requirement_shape(&attack).unwrap();
    assert_eq!(shape.kind, CombatRequirementKind::Attack);
    assert_eq!(shape.duration, CombatRequirementDuration::Turn);
    assert!(!shape.subject_tokens.is_empty());

    let combat = lex_line(
        "Up to one target creature attacks or blocks this combat if able.",
        0,
    )
    .unwrap();
    let shape = parse_combat_requirement_shape(&combat).unwrap();
    assert_eq!(shape.kind, CombatRequirementKind::AttackOrBlock);
    assert_eq!(shape.duration, CombatRequirementDuration::Combat);

    let separate_followup = lex_line(
        "Until end of turn, target land becomes a 4/4 creature. It must be blocked this turn if able.",
        0,
    )
    .unwrap();
    assert!(
        parse_combat_requirement_shape(&separate_followup).is_none(),
        "a later combat requirement must not consume an earlier sentence as its subject"
    );

    let block = lex_line("Each creature blocks target creature this turn if able.", 0).unwrap();
    assert!(matches!(
        parse_must_block_shape(&block),
        Some(MustBlockShape::SubjectAgainstAttacker { .. })
    ));
}

#[test]
fn parses_duration_trigger_prefixes() {
    let clause = lex_line(
        "Until your next upkeep, whenever a creature attacks, draw a card.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_duration_trigger_prefix_shape(&clause),
        Some(DurationTriggerPrefixShape::UntilYourNextUpkeep)
    );
    let (_, trigger) = primitives::split_lexed_once_on_comma(&clause).unwrap();
    assert_eq!(
        parse_trigger_clause_intro_shape(trigger),
        Some(TriggerClauseIntroShape::Event)
    );
}

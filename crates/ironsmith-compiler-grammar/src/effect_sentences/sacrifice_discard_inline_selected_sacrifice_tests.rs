use super::*;
use crate::lexer::lex_line;
use crate::model::ast::{SubjectVerbActionAst, SubjectVerbEffectAst};

#[test]
fn opponent_choice_delegates_selection_without_changing_the_sacrificing_player() {
    let tokens = lex_line("Sacrifice a land of an opponent's choice.", 0)
        .expect("sacrifice clause should lex");
    let parsed =
        parse_sacrifice(&tokens, None, None).expect("opponent-chosen sacrifice should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("player: Opponent"), "{debug}");
    assert!(debug.contains("player: You"), "{debug}");
}

#[test]
fn chooser_sacrifices_only_the_selected_set() {
    let tokens = lex_line("Sacrifices that many permanents of their choice.", 0)
        .expect("sacrifice clause should lex");
    let parsed = parse_sacrifice(
        &tokens,
        Some(SubjectAst::Player(PlayerAst::ItsController)),
        None,
    )
    .expect("sacrifice choice should parse");
    let debug = format!("{parsed:#?}");

    assert!(debug.contains("ChooseObjects"), "{debug}");
    assert!(debug.contains("player: ItsController"), "{debug}");
    assert!(debug.contains("player: That"), "{debug}");
    assert!(debug.contains("IsTaggedObject"), "{debug}");
}

#[test]
fn any_number_sacrifice_keeps_a_comma_then_mana_followup() {
    let tokens = lex_line("Sacrifice any number of lands, then add that much {C}.", 0)
        .expect("sacrifice and mana clause should lex");
    let parsed = parse_sacrifice(&tokens, None, None)
        .expect("sacrifice and mana clause should remain one typed sequence");
    let EffectAst::Sequence { effects } = parsed else {
        panic!("expected chosen sacrifice and mana sequence");
    };
    let [
        EffectAst::ChooseObjects { filter, tag, .. },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { filter: sacrificed },
            ..
        }),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::AddManaScaled { mana, amount },
            ..
        }),
    ] = effects.as_slice()
    else {
        panic!("expected exact choose/sacrifice/add sequence: {effects:#?}");
    };
    assert_eq!(filter.card_types, [crate::types::CardType::Land]);
    assert_eq!(sacrificed, &ObjectFilter::tagged(tag.clone()));
    assert_eq!(mana, &[crate::mana::ManaSymbol::Colorless]);
    assert_eq!(
        amount,
        &Value::EventValue(crate::effect::EventValueSpec::Amount)
    );

    let near_miss = lex_line("Sacrifice any number of nonbasic lands.", 0)
        .expect("ordinary sacrifice clause should lex");
    let EffectAst::Sequence { effects } =
        parse_sacrifice(&near_miss, None, None).expect("ordinary sacrifice should parse")
    else {
        panic!("ordinary chosen sacrifice should remain a sequence");
    };
    assert_eq!(effects.len(), 2, "ordinary sacrifice gained a followup");
}

#[test]
fn one_of_them_is_a_choice_from_the_referenced_set() {
    let tokens =
        lex_line("Sacrifice one of them.", 0).expect("tagged-set sacrifice clause should lex");
    let parsed =
        parse_sacrifice(&tokens, None, None).expect("tagged-set sacrifice choice should parse");

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Sacrifice {
                filter,
                count,
                target,
                one_of_referenced_set,
            },
        ..
    }) = parsed
    else {
        panic!("expected subject-verb sacrifice AST");
    };

    assert_eq!(count, 1);
    assert!(target.is_none());
    assert!(one_of_referenced_set);
    assert_eq!(filter.tagged_constraints.len(), 1);
    assert_eq!(
        filter.tagged_constraints[0].tag.as_str(),
        crate::tag::CompilerReferenceTag::It.as_str()
    );
}

#[test]
fn those_permanents_sacrifices_the_complete_referenced_set() {
    let tokens = lex_line("Sacrifice those permanents.", 0)
        .expect("plural tagged-set sacrifice clause should lex");
    let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
        .expect("plural tagged-set sacrifice should parse");

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::SacrificeAll { filter },
        ..
    }) = parsed
    else {
        panic!("expected all-of-result-set sacrifice AST");
    };
    assert!(filter.tagged_constraints.iter().any(|constraint| {
        constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
            && constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
    }));
    assert!(
        filter.card_types.len() > 1,
        "the permanent noun should survive alongside the result tag: {filter:#?}"
    );
}

#[test]
fn sacrifice_all_keeps_a_terminal_nonbasic_qualifier_on_only_the_land_arm() {
    let tokens = lex_line(
        "all artifacts, enchantments, and nonbasic lands they control.",
        0,
    )
    .expect("sacrifice-all clause should lex");
    let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
        .expect("sacrifice-all clause should parse");
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::SacrificeAll { filter },
        ..
    }) = parsed
    else {
        panic!("expected sacrifice-all subject-verb effect");
    };

    assert_eq!(filter.any_of.len(), 3, "{filter:#?}");
    assert!(filter.any_of.iter().any(|branch| {
        branch.card_types == [crate::types::CardType::Artifact]
            && branch.excluded_supertypes.is_empty()
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.card_types == [crate::types::CardType::Enchantment]
            && branch.excluded_supertypes.is_empty()
    }));
    assert!(filter.any_of.iter().any(|branch| {
        branch.card_types == [crate::types::CardType::Land]
            && branch.excluded_supertypes == [crate::types::Supertype::Basic]
    }));
}

#[test]
fn unit_fraction_rounded_up_sacrifice_chooses_the_exact_dynamic_set() {
    let tokens = lex_line(
        "Sacrifices a tenth of the creatures they control of their choice, rounded up.",
        0,
    )
    .expect("fractional sacrifice clause should lex");
    let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
        .expect("fractional sacrifice clause should parse");
    let EffectAst::Sequence { effects } = parsed else {
        panic!("fractional sacrifice must lower to a chosen-set sequence");
    };
    let [choose, sacrifice] = effects.as_slice() else {
        panic!("fractional sacrifice must have one choice and one consumer");
    };
    let EffectAst::ChooseObjects {
        filter,
        count,
        count_value: Some(count_value),
        player,
        tag,
    } = choose
    else {
        panic!("fractional sacrifice must choose a dynamic object set");
    };
    assert!(count.is_dynamic_x());
    assert_eq!(*player, PlayerAst::That);
    assert_eq!(filter.zone, Some(Zone::Battlefield));

    let Value::DividedRoundedDown(numerator, 10) = count_value else {
        panic!("a tenth rounded up must use exact ceil-division: {count_value:#?}");
    };
    let Value::Add(left, right) = numerator.as_ref() else {
        panic!("ceil-division must add denominator minus one: {numerator:#?}");
    };
    assert!(
        matches!(
            (left.as_ref(), right.as_ref()),
            (Value::Count(count_filter), Value::Fixed(9))
                | (Value::Fixed(9), Value::Count(count_filter))
                if count_filter == filter
        ),
        "ceil-division must count exactly the selectable set: {count_value:#?}"
    );

    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject,
        action: SubjectVerbActionAst::SacrificeAll { filter: sacrificed },
    }) = sacrifice
    else {
        panic!("chosen set must feed the typed sacrifice consumer");
    };
    assert_eq!(subject.player, PlayerAst::That);
    assert_eq!(sacrificed, &ObjectFilter::tagged(tag.clone()));
}

#[test]
fn sacrifice_all_except_kept_count_chooses_count_minus_keep_set() {
    let tokens = lex_line("Sacrifices all lands they control except for three.", 0)
        .expect("all-except sacrifice clause should lex");
    let parsed = parse_sacrifice(&tokens, Some(SubjectAst::Player(PlayerAst::That)), None)
        .expect("all-except sacrifice clause should parse");
    let EffectAst::Sequence { effects } = parsed else {
        panic!("all-except sacrifice must lower to a chosen-set sequence");
    };
    let [
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value: Some(Value::Add(left, right)),
            player: PlayerAst::That,
            tag,
        },
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject,
            action: SubjectVerbActionAst::SacrificeAll { filter: sacrificed },
        }),
    ] = effects.as_slice()
    else {
        panic!("expected exact dynamic choice and sacrifice: {effects:#?}");
    };
    assert!(count.is_dynamic_x());
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert_eq!(filter.card_types, [crate::types::CardType::Land]);
    assert!(matches!(
        (left.as_ref(), right.as_ref()),
        (Value::Count(count_filter), Value::Fixed(-3))
            | (Value::Fixed(-3), Value::Count(count_filter))
            if count_filter == filter
    ));
    assert_eq!(subject.player, PlayerAst::That);
    assert_eq!(sacrificed, &ObjectFilter::tagged(tag.clone()));
}

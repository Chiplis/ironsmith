use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_return_all_surface_facts() {
    let tokens = lex_line(
        "all creature cards not chosen this way to their owners' hands",
        0,
    )
    .unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    assert!(matches!(
        shape.target,
        ReturnTargetShape::All {
            set_quantifier_surface: ironsmith_core::SetQuantifierSurface::All,
            chosen_this_way_excluded: Some(true),
            ..
        }
    ));
    assert_eq!(shape.destination.zone, ReturnZoneShape::Hand);

    let each = lex_line(
        "each creature that isn't a Kraken, Leviathan, or Serpent to its owner's hand",
        0,
    )
    .unwrap();
    let each = parse_return_clause_shape(&each).expect("each-return shape");
    assert!(matches!(
        each.target,
        ReturnTargetShape::All {
            set_quantifier_surface: ironsmith_core::SetQuantifierSurface::Each,
            ..
        }
    ));
}

#[test]
fn strips_set_quantifier_from_source_linked_exiled_card_filter() {
    let tokens = lex_line(
            "all cards exiled with this Vehicle except this card to the battlefield tapped under their owners' control",
            0,
        )
        .unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("source-linked return shape");
    let ReturnTargetShape::UntargetedExiledCards {
        filter_tokens,
        count,
    } = shape.target
    else {
        panic!("expected source-linked exiled cards: {shape:#?}");
    };
    assert!(count.is_none());
    assert_eq!(
        filter_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>(),
        [
            "cards", "exiled", "with", "this", "Vehicle", "except", "this", "card"
        ]
    );
    assert!(shape.destination.tapped);
    assert_eq!(shape.destination.controller, ReturnControllerShape::Owner);
}

#[test]
fn parses_delayed_attached_return_surface() {
    let tokens = lex_line(
        "target Aura to the battlefield attached to it at the beginning of the next end step",
        0,
    )
    .unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    assert!(shape.destination.attached_to_tokens.is_some());
    assert!(matches!(
        shape.destination.timing,
        Some(ReturnTimingShape::NextEndStep(PlayerFilter::Any))
    ));
}

#[test]
fn preserves_contextual_hand_destination_without_changing_owner_destination() {
    for (text, expected) in [
        ("it to your hand", Some(PlayerAst::You)),
        ("those cards to their hand", Some(PlayerAst::That)),
        ("it to its owner's hand", None),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("shape");
        assert_eq!(
            shape.destination.destination_player_surface, expected,
            "{text}"
        );
    }
}

#[test]
fn distinguishes_that_player_battlefield_control_from_owner_and_you() {
    for (text, expected) in [
        (
            "this creature to the battlefield under that player's control at the beginning of their next upkeep",
            ReturnControllerShape::ThatPlayer,
        ),
        (
            "this creature to the battlefield under its owner's control at the beginning of their next upkeep",
            ReturnControllerShape::Owner,
        ),
        (
            "this creature to the battlefield under your control at the beginning of your next upkeep",
            ReturnControllerShape::You,
        ),
    ] {
        let tokens = lex_line(text, 0).unwrap();
        let shape = parse_return_clause_shape(&tokens).expect("return shape");
        assert_eq!(shape.destination.controller, expected, "{text}");
    }
}

#[test]
fn normalizes_destination_first_return_surface() {
    let tokens = lex_line("to their owners' hands all creatures", 0).unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    assert!(shape.destination_first);
    assert!(matches!(shape.target, ReturnTargetShape::All { .. }));
    assert_eq!(shape.destination.zone, ReturnZoneShape::Hand);
    assert_eq!(shape.destination.controller, ReturnControllerShape::Owner);
}

#[test]
fn preserves_destination_first_control_boundary() {
    let tokens = lex_line("to the battlefield under your control target creature", 0).unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    assert_eq!(shape.destination.controller, ReturnControllerShape::You);
    let ReturnTargetShape::Singular { target_tokens, .. } = shape.target else {
        panic!("expected singular target");
    };
    assert_eq!(target_tokens.len(), 2);
}

#[test]
fn preserves_top_only_graveyard_return_as_a_typed_shape_fact() {
    let tokens = lex_line(
        "the top creature card of your graveyard to the battlefield",
        0,
    )
    .unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    let ReturnTargetShape::Singular {
        target_tokens,
        top_only,
        ..
    } = shape.target
    else {
        panic!("expected singular return target");
    };

    assert!(top_only);
    assert_eq!(
        target_tokens
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>(),
        ["creature", "card", "of", "your", "graveyard"]
    );
}

#[test]
fn preserves_source_graveyard_or_exile_return_origin() {
    let tokens = lex_line(
        "this card from your graveyard or from exile to the battlefield tapped",
        0,
    )
    .unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    let ReturnTargetShape::Singular {
        source_from_graveyard_or_exile_tokens,
        source_from_graveyard_tokens,
        ..
    } = shape.target
    else {
        panic!("expected singular return target");
    };
    assert!(source_from_graveyard_tokens.is_none());
    assert_eq!(
        source_from_graveyard_or_exile_tokens
            .expect("typed multi-zone source")
            .iter()
            .filter_map(OwnedLexToken::as_word)
            .collect::<Vec<_>>(),
        ["this", "card"]
    );
    assert!(shape.destination.tapped);
}

#[test]
fn removes_random_marker_without_truncating_target() {
    let tokens = lex_line("a card exiled with it at random to its owner's hand", 0).unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    assert!(shape.random);
    let ReturnTargetShape::Singular { target_tokens, .. } = shape.target else {
        panic!("expected singular target");
    };
    assert_eq!(target_tokens.len(), 5);
    assert!(
        target_tokens
            .last()
            .is_some_and(|token| token_is(token, "it"))
    );
}

#[test]
fn preserves_source_subtype_in_paired_source_and_exiled_surface() {
    let tokens = lex_line("this Elf card and exiled cards to their owners' hands", 0).unwrap();
    let shape = parse_return_clause_shape(&tokens).expect("shape");
    assert!(matches!(
        shape.target,
        ReturnTargetShape::PairedSourceAndExiled {
            source_subtype: Some(Subtype::Elf),
        }
    ));
}

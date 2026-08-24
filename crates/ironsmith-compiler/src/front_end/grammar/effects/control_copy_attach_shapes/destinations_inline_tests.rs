use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_library_and_battlefield_destinations() {
    let choice = lex_line(
        "target card on its owner's choice of the top or bottom of their library",
        0,
    )
    .unwrap();
    assert!(
        parse_library_choice_destination_shape(&choice).is_some(),
        "{:?}",
        crate::lexer::TokenWordView::new(&choice).to_word_refs()
    );

    let onto = lex_line(
        "target creature onto the battlefield tapped and attacking",
        0,
    )
    .unwrap();
    let clause = parse_onto_clause_shape(&onto).unwrap();
    let destination = parse_onto_battlefield_destination_shape(clause.destination_tokens).unwrap();
    assert!(destination.tapped);
    assert!(destination.attacking);
    assert!(destination.supported_tail);
}

#[test]
fn onto_destination_excludes_a_trailing_where_x_binding() {
    let tokens = lex_line(
            "an artifact card with mana value X or less from your hand onto the battlefield, where X is the number of ingenuity counters on this creature",
            0,
        )
        .unwrap();
    let clause = parse_onto_clause_shape(&tokens).expect("onto clause");

    assert_eq!(
        crate::lexer::token_word_refs(clause.destination_tokens),
        ["the", "battlefield"]
    );
}

#[test]
fn battlefield_controller_prefix_preserves_attached_to_suffix() {
    let destination = lex_line(
        "battlefield under your control attached to target creature",
        0,
    )
    .unwrap();
    let parsed = parse_onto_battlefield_destination_shape(&destination).unwrap();
    assert_eq!(parsed.controller, Some(BattlefieldControllerShape::You));
    assert!(parsed.supported_tail);
    let attached = parsed
        .attached_to_tokens
        .expect("attachment suffix should survive controller parsing");
    assert_eq!(
        crate::lexer::TokenWordView::new(&attached).to_word_refs(),
        vec!["target", "creature"]
    );
}

#[test]
fn library_placement_keeps_target_player_words_out_of_destination_surface() {
    let owner_destination =
        lex_line("a creature you control on top of its owner's library", 0).unwrap();
    let owner_shape = parse_library_placement_destination_shape(&owner_destination).unwrap();
    assert_eq!(
        crate::grammar::effects::control_copy_attach_shapes::parse_destination_player(
            owner_shape.destination_tokens,
        ),
        None,
    );

    let plural_owner_destination =
        lex_line("all creatures on the bottom of their owners' libraries", 0).unwrap();
    let plural_owner_shape =
        parse_library_placement_destination_shape(&plural_owner_destination).unwrap();
    assert_eq!(
        crate::grammar::effects::control_copy_attach_shapes::parse_destination_player(
            plural_owner_shape.destination_tokens,
        ),
        None,
    );
    assert_eq!(
            crate::grammar::effects::control_copy_attach_shapes::parse_destination_player_reference_surface(
                plural_owner_shape.destination_tokens,
            ),
            None,
        );

    let your_destination = lex_line("a creature you control on top of your library", 0).unwrap();
    let your_shape = parse_library_placement_destination_shape(&your_destination).unwrap();
    assert_eq!(
        crate::grammar::effects::control_copy_attach_shapes::parse_destination_player(
            your_shape.destination_tokens,
        ),
        Some(crate::cards::builders::PlayerAst::You),
    );
}

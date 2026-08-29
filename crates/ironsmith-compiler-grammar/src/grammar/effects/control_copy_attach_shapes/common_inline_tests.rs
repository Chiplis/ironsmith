use super::*;
use crate::lexer::lex_line;

#[test]
fn parses_controller_and_counted_target_prefixes() {
    let controller = lex_line("under your control creatures", 0).unwrap();
    let parsed = parse_battlefield_controller_prefix(&controller).unwrap();
    assert_eq!(parsed.controller, BattlefieldControllerShape::You);
    assert!(permission_shapes::exact_tokens(parsed.rest, &["creatures"]));

    let target = lex_line("up to two cards from your graveyard", 0).unwrap();
    assert!(parse_counted_card_target_shape(&target).is_some());
}

#[test]
fn distinguishes_pronoun_and_explicit_player_destinations() {
    let pronoun = lex_line("their graveyard", 0).unwrap();
    assert_eq!(
        parse_destination_player_reference_surface(&pronoun),
        Some(ironsmith_core::DestinationPlayerReferenceSurface::Pronoun)
    );

    let explicit = lex_line("that player's graveyard", 0).unwrap();
    assert_eq!(
        parse_destination_player_reference_surface(&explicit),
        Some(ironsmith_core::DestinationPlayerReferenceSurface::ThatPlayer)
    );

    for owner_destination in [
        "its owner's library",
        "their owners' libraries",
        "their owner's library",
    ] {
        let owner_destination = lex_line(owner_destination, 0).unwrap();
        assert_eq!(parse_destination_player(&owner_destination), None);
        assert_eq!(
            parse_destination_player_reference_surface(&owner_destination),
            None
        );
    }
}

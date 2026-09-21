use super::*;

#[test]
pub(super) fn parses_hyphenated_face_down_look_shapes() {
    assert!(matches!(
        parse_resource_look_shape(&lex("at target face-down creature."), None),
        Some(ResourceLookShape::Object {
            kind: ResourceLookObjectKind::FaceDownCreature,
            ..
        })
    ));
    assert!(matches!(
        parse_resource_look_shape(&lex("at target face down permanent."), None),
        Some(ResourceLookShape::Object {
            kind: ResourceLookObjectKind::FaceDownPermanent,
            ..
        })
    ));
}

#[test]
pub(super) fn parses_spy_network_compound_look_shape() {
    let tokens = lex(
        "at target player's hand, the top card of that player's library, and any face-down creatures they control.",
    );
    assert!(matches!(
        parse_resource_look_shape(&tokens, None),
        Some(ResourceLookShape::Hand {
            player: PlayerAst::Target,
            followup: ResourceLookHandFollowup::TopCardAndFaceDownCreatures,
            ..
        })
    ));
}

#[test]
pub(super) fn parses_resource_shuffle_shapes() {
    assert_eq!(
        parse_resource_shuffle_shape(
            &lex("them into their library from their graveyard"),
            PlayerAst::That
        ),
        Some(ResourceShuffleShape::TaggedIntoLibrary {
            player: PlayerAst::That,
            to_bottom: false,
        })
    );
    assert_eq!(
        parse_resource_shuffle_shape(&lex("your library"), PlayerAst::Implicit),
        Some(ResourceShuffleShape::SimpleLibrary)
    );
}

#[test]
pub(super) fn random_hand_look_preserves_owner_and_rejects_extra_qualifiers() {
    for (text, expected) in [
        ("at a card at random in target player's hand", PlayerAst::Target),
        ("at a card at random from target opponent's hand", PlayerAst::TargetOpponent),
        ("at a card at random in your hand", PlayerAst::You),
    ] {
        assert!(matches!(parse_resource_look_shape(&lex(text), None), Some(ResourceLookShape::RandomHandCard { player }) if player == expected));
    }
    assert!(parse_resource_look_shape(&lex("at a card at random in target player's hand with mana value 3"), None).is_none());
}

use super::*;
use crate::runtime_backend::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_resource_look_shapes() {
    assert!(matches!(
        parse_resource_look_shape(&lex("at the top two cards of your library"), None),
        Some(ResourceLookShape::TopCards {
            player: PlayerAst::You,
            count: Value::Fixed(2)
        })
    ));
    let hand_tokens = lex("at target player's hand.");
    assert!(matches!(
        parse_resource_look_shape(&hand_tokens, None),
        Some(ResourceLookShape::Hand {
            player: PlayerAst::Target,
            ..
        })
    ));

    let dynamic_tokens =
        lex("at the top X cards of your library, where X is that creature's power");
    let Some(ResourceLookShape::TopCards { count, .. }) =
        parse_resource_look_shape(&dynamic_tokens, None)
    else {
        panic!("expected dynamic top-card look shape");
    };
    assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::WhereXIs));
    assert!(matches!(count.unhinted(), Value::PowerOf(_)));
}

#[test]
fn parses_hyphenated_face_down_look_shapes() {
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
fn parses_spy_network_compound_look_shape() {
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
fn parses_resource_shuffle_shapes() {
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
fn parses_resource_chosen_name_target_shape() {
    let tokens = lex("target creature with a name chosen for this source this way");
    let shape = parse_resource_chosen_name_target_shape(&tokens).unwrap();
    assert_eq!(
        TokenWordView::new(shape.base_tokens).word_refs(),
        vec!["target", "creature"]
    );
}

#[test]
fn parses_all_unspent_mana_resource_shape() {
    assert!(parse_resource_all_unspent_mana_shape(&lex(
        "all unspent mana"
    )));
    assert!(!parse_resource_all_unspent_mana_shape(&lex(
        "all unspent energy"
    )));
}

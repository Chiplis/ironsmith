use super::*;
use crate::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_dynamic_top_and_single_bottom_library_shapes() {
    let per_opponent = parse_exile_dynamic_top_library_shape(
        &lex("a card from the top of your library for each opponent you have"),
        PlayerAst::Implicit,
    )
    .unwrap();
    assert_eq!(
        per_opponent.player,
        ExileLibraryPlayerShape::Player(PlayerAst::You)
    );
    assert_eq!(
        per_opponent.count.unhinted(),
        &Value::CountPlayers(crate::target::PlayerFilter::Opponent)
    );
    assert!(
        per_opponent
            .count
            .has_surface_hint(ValueSurfaceHint::ForEach)
    );
    assert!(!per_opponent.face_down);

    let face_down_per_opponent = parse_exile_dynamic_top_library_shape(
        &lex("a card from the top of your library face down for each opponent you have"),
        PlayerAst::Implicit,
    )
    .unwrap();
    assert!(face_down_per_opponent.face_down);
    assert_eq!(
        face_down_per_opponent.count.unhinted(),
        &Value::CountPlayers(crate::target::PlayerFilter::Opponent)
    );

    let dynamic = parse_exile_dynamic_top_library_shape(
        &lex("that many cards from the top of your library"),
        PlayerAst::Implicit,
    )
    .unwrap();
    assert_eq!(dynamic.count, Value::EventValue(EventValueSpec::Amount));
    assert_eq!(
        dynamic.player,
        ExileLibraryPlayerShape::Player(PlayerAst::You)
    );

    let top = parse_exile_top_library_shape(
        &lex("the top two cards of each opponent's library"),
        PlayerAst::Implicit,
    )
    .unwrap();
    assert_eq!(top.count, Value::Fixed(2));
    assert_eq!(top.player, ExileLibraryPlayerShape::EachOpponent);

    let each_player_top = parse_exile_top_library_shape(
        &lex("the top card of each player's library"),
        PlayerAst::Implicit,
    )
    .unwrap();
    assert_eq!(each_player_top.count, Value::Fixed(1));
    assert_eq!(each_player_top.player, ExileLibraryPlayerShape::EachPlayer);

    let damaged_player_top = parse_exile_top_library_shape(
        &lex("the top card of that player's library"),
        PlayerAst::Implicit,
    )
    .unwrap();
    assert_eq!(damaged_player_top.count, Value::Fixed(1));
    assert_eq!(
        damaged_player_top.player,
        ExileLibraryPlayerShape::Player(PlayerAst::That)
    );

    let implicit_library_top =
        parse_exile_top_library_shape(&lex("the top four cards"), PlayerAst::You).unwrap();
    assert_eq!(implicit_library_top.count, Value::Fixed(4));
    assert_eq!(
        implicit_library_top.player,
        ExileLibraryPlayerShape::Player(PlayerAst::You)
    );

    assert!(
        parse_exile_top_library_shape(&lex("the top four cards from a graveyard"), PlayerAst::You,)
            .is_none(),
        "the implicit-library route must consume the complete top-card phrase"
    );

    let bottom = parse_exile_bottom_library_shape(
        &lex("the bottom card of their library"),
        PlayerAst::Opponent,
    )
    .unwrap();
    assert_eq!(
        bottom.player,
        ExileLibraryPlayerShape::Player(PlayerAst::Opponent)
    );

    let excess = parse_exile_dynamic_top_library_shape(
            &lex(
                "cards from the top of your library equal to the excess damage dealt to that creature this way",
            ),
            PlayerAst::Implicit,
        )
        .unwrap();
    assert_eq!(
        excess.count.unhinted(),
        &Value::PendingEffectMetric {
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::ExcessDamage,
        }
    );
    assert!(excess.count.has_surface_hint(ValueSurfaceHint::EqualTo));

    let prior_object_power = parse_exile_dynamic_top_library_shape(
        &lex("cards equal to its power from the top of its owner's library"),
        PlayerAst::Implicit,
    )
    .unwrap();
    assert_eq!(
        prior_object_power.player,
        ExileLibraryPlayerShape::Player(PlayerAst::ItsOwner)
    );
    assert_eq!(
        prior_object_power.count.unhinted(),
        &Value::PowerOf(Box::new(ChooseSpec::Tagged(
            crate::tag::CompilerReferenceTag::Triggering.key()
        )))
    );
    assert!(
        prior_object_power
            .count
            .has_surface_hint(ValueSurfaceHint::EqualTo)
    );

    assert!(
        parse_exile_dynamic_top_library_shape(
            &lex("cards equal to its power from the top of your library"),
            PlayerAst::Implicit,
        )
        .is_some_and(|shape| matches!(
            shape.count.unhinted(),
            Value::PowerOf(source) if matches!(source.unhinted(), ChooseSpec::Source)
        )),
        "a source-owned library must not inherit the prior-object LKI binding"
    );
}

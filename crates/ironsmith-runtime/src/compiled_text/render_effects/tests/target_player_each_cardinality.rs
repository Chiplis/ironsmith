use super::*;

fn target_players_each_draw(count: ChoiceCount, explicit_declaration: bool) -> Vec<Effect> {
    let target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any)).with_count(count);
    let declaration = if explicit_declaration {
        crate::effects::TargetOnlyEffect::explicit(target)
    } else {
        crate::effects::TargetOnlyEffect::new(target)
    };

    vec![
        Effect::new(declaration),
        Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
            vec![Effect::new(crate::effects::DrawCardsEffect::new(
                1,
                PlayerFilter::IteratedPlayer,
            ))],
        )),
    ]
}

#[test]
fn bounded_plural_target_players_compact_into_each_surface() {
    assert_eq!(
        describe_effect_list(&target_players_each_draw(ChoiceCount::exactly(2), false)),
        "Two target players each draw a card"
    );
    assert_eq!(
        describe_effect_list(&target_players_each_draw(ChoiceCount::up_to(2), false)),
        "Up to two target players each draw a card"
    );
}

#[test]
fn any_number_target_players_each_mill_their_own_half_library() {
    let target = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any))
        .with_count(ChoiceCount::any_number());
    let effects = vec![
        Effect::new(crate::effects::TargetOnlyEffect::new(target)),
        Effect::new(crate::effects::ForPlayersEffect::new(
            PlayerFilter::Target(Box::new(PlayerFilter::Any)),
            vec![Effect::new(crate::effects::MillEffect::new(
                Value::HalfRoundedDown(Box::new(Value::CardsInLibrary(
                    PlayerFilter::IteratedPlayer,
                ))),
                PlayerFilter::IteratedPlayer,
            ))],
        )),
    ];

    assert_eq!(
        describe_effect_list(&effects),
        "Any number of target players each mill half their library, rounded down"
    );
}

#[test]
fn singular_or_authored_target_declarations_do_not_use_each_compactor() {
    for effects in [
        target_players_each_draw(ChoiceCount::exactly(1), false),
        target_players_each_draw(ChoiceCount::up_to(1), false),
        target_players_each_draw(ChoiceCount::exactly(2), true),
    ] {
        assert!(
            !describe_effect_list(&effects).contains("players each"),
            "invalid target cardinality compacted as plural: {}",
            describe_effect_list(&effects)
        );
    }
}

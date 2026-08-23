use super::*;

fn conditional_consult_destination(hand_tag: TagKey) -> (Effect, Effect, Effect) {
    let revealed = TagKey::from("__sentence_helper_revealed_test");
    let matched = TagKey::from("__sentence_helper_consult_match_test");
    let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
        PlayerFilter::You,
        crate::effects::consult_helpers::LibraryConsultMode::Reveal,
        ObjectFilter::creature(),
        crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
        revealed.clone(),
        matched.clone(),
    ));
    let condition = Condition::ValueComparison {
        left: Value::ManaValueOf(Box::new(
            ChooseSpec::Tagged(matched.clone()).with_surface_hint(
                crate::target::ChooseSpecSurfaceHint::SourceReference(
                    crate::target::SourceReferenceSurface::ThisPermanentType("it".to_string()),
                ),
            ),
        )),
        operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
        right: Value::Count(
            ObjectFilter::land()
                .in_zone(Zone::Battlefield)
                .controlled_by(PlayerFilter::You),
        ),
    };
    let battlefield = Effect::new(
        crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(matched.clone()),
            Zone::Battlefield,
            false,
        )
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put),
    );
    let hand = Effect::new(
        crate::effects::MoveToZoneEffect::new(ChooseSpec::Tagged(hand_tag), Zone::Hand, false)
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
            .with_destination_player_surface(PlayerFilter::You),
    );
    let conditional = Effect::new(crate::effects::ConditionalEffect::new(
        condition,
        vec![battlefield],
        vec![hand],
    ));
    let remainder = Effect::new(
        crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
            revealed,
            Some(matched),
            crate::effects::consult_helpers::LibraryBottomOrder::Random,
            PlayerFilter::You,
        ),
    );
    (consult, conditional, remainder)
}

#[test]
fn linked_conditional_consult_uses_it_and_bare_rest_surfaces() {
    let matched = TagKey::from("__sentence_helper_consult_match_test");
    let (consult, conditional, remainder) = conditional_consult_destination(matched);
    assert_eq!(
        describe_effect_list(&[consult, conditional, remainder]),
        "Reveal cards from the top of your library until you reveal a creature card. If its mana value is less than or equal to the number of lands you control, put it onto the battlefield. Otherwise, put it into your hand. Put the rest on the bottom of your library in a random order"
    );
}

#[test]
fn conditional_consult_rejects_a_different_destination_tag() {
    let (consult, conditional, remainder) =
        conditional_consult_destination(TagKey::from("different_result"));
    let rendered = describe_effect_list(&[consult, conditional, remainder]);
    assert_ne!(
        rendered,
        "Reveal cards from the top of your library until you reveal a creature card. If its mana value is less than or equal to the number of lands you control, put it onto the battlefield. Otherwise, put it into your hand. Put the rest on the bottom of your library in a random order"
    );
}

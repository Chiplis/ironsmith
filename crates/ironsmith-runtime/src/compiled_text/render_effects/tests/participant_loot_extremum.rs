use super::*;

fn participant_filter() -> PlayerFilter {
    PlayerFilter::excluding(
        PlayerFilter::Any,
        PlayerFilter::excluding(PlayerFilter::NotYou, PlayerFilter::Defending),
    )
}

fn participant_loot_effects_with(
    producer_id: crate::effect::EffectId,
    condition_id: crate::effect::EffectId,
    participants: PlayerFilter,
    predicate: EffectPredicate,
    draw_count: i32,
) -> Vec<Effect> {
    participant_loot_effects_with_target(
        producer_id,
        condition_id,
        participants,
        predicate,
        draw_count,
        ChooseSpec::Source,
    )
}

fn participant_loot_effects_with_target(
    producer_id: crate::effect::EffectId,
    condition_id: crate::effect::EffectId,
    participants: PlayerFilter,
    predicate: EffectPredicate,
    draw_count: i32,
    counter_target: ChooseSpec,
) -> Vec<Effect> {
    vec![
        Effect::with_id(
            producer_id.0,
            Effect::for_players(
                participants,
                vec![
                    Effect::target_draws(Value::Fixed(draw_count), PlayerFilter::IteratedPlayer),
                    Effect::discard_player(Value::Fixed(1), PlayerFilter::IteratedPlayer, false),
                ],
            ),
        ),
        Effect::if_then(
            condition_id,
            predicate,
            vec![Effect::put_counters(
                crate::object::CounterType::PlusOnePlusOne,
                Value::Fixed(2),
                counter_target,
            )],
        ),
    ]
}

fn participant_loot_effects() -> Vec<Effect> {
    let condition = crate::effect::EffectId(41);
    participant_loot_effects_with(
        condition,
        condition,
        participant_filter(),
        EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
            player: PlayerFilter::You,
        },
        1,
    )
}

#[test]
fn renders_participant_loot_and_tied_greatest_followup() {
    assert_eq!(
        describe_structural_multisentence_effect_list(&participant_loot_effects()).as_deref(),
        Some(
            "You and defending player each draw a card, then discard a card. Put two +1/+1 counters on this creature if you discarded the card with the greatest mana value among those cards or tied for greatest"
        )
    );
}

#[test]
fn renders_named_source_surface_but_rejects_a_targeted_source() {
    let condition = crate::effect::EffectId(41);
    let named_source = ChooseSpec::Source.with_surface_hint(
        crate::target::ChooseSpecSurfaceHint::SourceReference(
            crate::target::SourceReferenceSurface::FullName("Cait".to_string()),
        ),
    );
    let surfaced = participant_loot_effects_with_target(
        condition,
        condition,
        participant_filter(),
        EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
            player: PlayerFilter::You,
        },
        1,
        named_source,
    );
    assert_eq!(
        describe_structural_multisentence_effect_list(&surfaced).as_deref(),
        Some(
            "You and defending player each draw a card, then discard a card. Put two +1/+1 counters on Cait if you discarded the card with the greatest mana value among those cards or tied for greatest"
        )
    );

    let targeted_source = participant_loot_effects_with_target(
        condition,
        condition,
        participant_filter(),
        EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
            player: PlayerFilter::You,
        },
        1,
        ChooseSpec::target(ChooseSpec::Source),
    );
    assert!(describe_structural_multisentence_effect_list(&targeted_source).is_none());
}

#[test]
fn rejects_mismatched_result_identity_and_participants() {
    let mismatched_condition = participant_loot_effects_with(
        crate::effect::EffectId(41),
        crate::effect::EffectId(99),
        participant_filter(),
        EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
            player: PlayerFilter::You,
        },
        1,
    );
    assert!(describe_structural_multisentence_effect_list(&mismatched_condition).is_none());

    let wrong_participants = participant_loot_effects_with(
        crate::effect::EffectId(41),
        crate::effect::EffectId(41),
        PlayerFilter::Any,
        EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
            player: PlayerFilter::You,
        },
        1,
    );
    assert!(describe_structural_multisentence_effect_list(&wrong_participants).is_none());
}

#[test]
fn rejects_non_tie_extremum_predicate_and_nonuniform_loot() {
    let wrong_predicate = participant_loot_effects_with(
        crate::effect::EffectId(41),
        crate::effect::EffectId(41),
        participant_filter(),
        EffectPredicate::Happened,
        1,
    );
    assert!(describe_structural_multisentence_effect_list(&wrong_predicate).is_none());

    let wrong_draw_count = participant_loot_effects_with(
        crate::effect::EffectId(41),
        crate::effect::EffectId(41),
        participant_filter(),
        EffectPredicate::PlayerAffectedObjectHasGreatestManaValue {
            player: PlayerFilter::You,
        },
        2,
    );
    assert!(describe_structural_multisentence_effect_list(&wrong_draw_count).is_none());
}

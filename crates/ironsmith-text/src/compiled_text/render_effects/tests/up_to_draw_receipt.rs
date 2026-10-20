use super::*;

#[test]
fn up_to_draw_binds_only_the_matching_chosen_number_receipt() {
    for (id, action, accepted) in [
        (7, crate::effect::PriorEffectAction::ChosenNumber, true),
        (8, crate::effect::PriorEffectAction::ChosenNumber, false),
        (7, crate::effect::PriorEffectAction::Drawn, false),
    ] {
        let effects = vec![
            Effect::new(crate::effects::WithIdEffect::new(crate::effect::EffectId(7),
                Effect::new(crate::effects::ChooseNumberEffect::new(PlayerFilter::You, 0, 4).for_draw_count()))),
            Effect::new(crate::effects::DrawCardsEffect {
                player: PlayerFilter::You,
                count: Value::PriorEffectMetric {
                    effect_id: crate::effect::EffectId(id),
                    query: crate::effect::PriorEffectMetricQuery::new(
                        crate::effect::EffectMetricSource::Outcome,
                        crate::effect::EffectMetric::Count,
                    ).with_action(action),
                },
            }),
        ];
        let text = super::super::effect_lists::describe_choose_number_then_draw_that_many(&effects);
        if accepted { assert_eq!(text.as_deref(), Some("you draw up to four cards")); }
        else { assert!(text.is_none()); }
    }
}

#[test]
fn ordinary_number_choices_do_not_claim_draw_up_to_surface() {
    let effects = vec![
        Effect::with_id(7, Effect::new(crate::effects::ChooseNumberEffect::new(PlayerFilter::You, 0, 4))),
        Effect::new(crate::effects::DrawCardsEffect::new(
            Value::PriorEffectMetric {
                effect_id: crate::effect::EffectId(7),
                query: crate::effect::PriorEffectMetricQuery::new(
                    crate::effect::EffectMetricSource::Outcome,
                    crate::effect::EffectMetric::Count,
                ).with_action(crate::effect::PriorEffectAction::ChosenNumber),
            },
            PlayerFilter::You,
        )),
    ];
    assert!(super::super::effect_lists::describe_choose_number_then_draw_that_many(&effects).is_none());
}

#[test]
fn repeated_opponent_exchange_keeps_up_to_draw_and_opponent_gate() {
    let oracle = "Target opponent draws two cards, then you draw up to four cards. That opponent may repeat this process as many times as they choose.";
    let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Repeat exchange probe")
        .card_types(vec![crate::types::CardType::Sorcery])
        .parse_text(oracle).expect("repeat exchange compiles");
    let effects = definition.spell_effect.as_ref().expect("spell")
        .segments[0].default_effects.clone();
    assert_eq!(
        super::super::effect_lists::describe_target_opponent_repeat_draw_exchange(&effects).as_deref(),
        Some(oracle.trim_end_matches('.')),
    );
    assert_eq!(crate::compiled_text::compiled_text_lines(&definition), [oracle]);

    let mut wrong = effects.clone();
    let repeated = wrong[1].downcast_ref::<crate::effects::RepeatProcessEffect>().unwrap();
    let mut repeated = repeated.clone();
    let prompt = repeated.effects[1].downcast_ref::<crate::effects::WithIdEffect>().unwrap();
    let mut prompt = prompt.clone();
    prompt.id = crate::effect::EffectId(prompt.id.0 + 1);
    repeated.effects[1] = Effect::new(prompt);
    wrong[1] = Effect::new(repeated);
    assert!(super::super::effect_lists::describe_target_opponent_repeat_draw_exchange(&wrong).is_none(),
        "an unrelated continuation receipt must not receive the compact surface");
}

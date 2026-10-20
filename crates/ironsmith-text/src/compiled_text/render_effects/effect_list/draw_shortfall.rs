use super::*;

fn result_id_on_wrapper_chain(effect: &Effect, id: crate::effect::EffectId) -> bool {
    if let Some(with_id) = effect.downcast_ref::<crate::effects::WithIdEffect>() {
        return with_id.id == id || result_id_on_wrapper_chain(&with_id.effect, id);
    }
    if let Some(tagged) = effect.downcast_ref::<crate::effects::TaggedEffect>() {
        return result_id_on_wrapper_chain(&tagged.effect, id);
    }
    if let Some(tag_all) = effect.downcast_ref::<crate::effects::TagAllEffect>() {
        return result_id_on_wrapper_chain(&tag_all.effect, id);
    }
    false
}

/// One numeric choice per player, one draw instruction per player, then a
/// separate life pass based on the exact cards each participant drew. The
/// result-id guard prevents a life clause referring to another draw from
/// inheriting this surface.
pub(super) fn describe_each_player_draw_shortfall(effects: &[Effect]) -> Option<String> {
    let [choice_effect, draw_effect, life_effect] = effects else {
        return None;
    };
    let choice = structural_unwrap_render_wrappers(choice_effect)
        .downcast_ref::<crate::effects::ChoosePlayerOptionEffect>()?;
    let range = choice.number?;
    let limit = range.max?;
    if !choice.draw_count_choice
        || choice.participants != PlayerFilter::Any
        || choice.chooser != crate::effects::PlayerOptionChooser::Participant
        || !choice.options.is_empty()
        || choice.secret
        || choice.guess
        || choice.exclude_source_history
        || choice.choose_one_participant
        || range.min != 0
        || limit == 0
    {
        return None;
    }

    let draw_players = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    let life_players = structural_unwrap_render_wrappers(life_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if draw_players.filter != PlayerFilter::Any
        || life_players.filter != PlayerFilter::Any
        || draw_players.sequential
        || life_players.sequential
        || draw_players.starting_with_controller
        || life_players.starting_with_controller
        || draw_players.stop_after_first_happened
        || life_players.stop_after_first_happened
    {
        return None;
    }
    let [draw_effect] = draw_players.effects.as_slice() else {
        return None;
    };
    let [gain_effect] = life_players.effects.as_slice() else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let gain = structural_unwrap_render_wrappers(gain_effect)
        .downcast_ref::<crate::effects::GainLifeEffect>()?;
    if draw.player != PlayerFilter::IteratedPlayer
        || draw.count != Value::IteratedPlayerChosenNumber
        || gain.player != ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    {
        return None;
    }
    let Value::Scaled(shortfall, amount) = gain.amount.unhinted() else {
        return None;
    };
    let Value::Add(limit_value, negative_draws) = shortfall.unhinted() else {
        return None;
    };
    let Value::Scaled(drawn, -1) = negative_draws.unhinted() else {
        return None;
    };
    let Value::PriorEffectMetric { effect_id, query } = drawn.unhinted() else {
        return None;
    };
    let expected_query = crate::effect::PriorEffectMetricQuery::new(
        crate::effect::EffectMetricSource::Outcome,
        crate::effect::EffectMetric::Count,
    )
    .with_action(crate::effect::PriorEffectAction::Drawn)
    .with_player(PlayerFilter::IteratedPlayer);
    if limit > i32::MAX as u32
        || *limit_value.unhinted() != Value::Fixed(limit as i32)
        || *amount <= 0
        || query != &expected_query
        || !(result_id_on_wrapper_chain(effects.get(1)?, *effect_id)
            || result_id_on_wrapper_chain(draw_effect, *effect_id))
    {
        return None;
    }

    let limit_word = small_number_word(limit).unwrap_or_else(|| limit.to_string());
    Some(format!(
        "Each player may draw up to {limit_word} cards. For each card less than {limit_word} a player draws this way, that player gains {amount} life"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects(receipt_id: u32, draw_player: PlayerFilter) -> Vec<Effect> {
        let choice = crate::effects::ChoosePlayerOptionEffect::numbers(
            PlayerFilter::Any,
            ironsmith_core::PlayerNumberChoice {
                min: 0,
                max: Some(2),
            },
        )
        .for_draw_count();
        let query = crate::effect::PriorEffectMetricQuery::new(
            crate::effect::EffectMetricSource::Outcome,
            crate::effect::EffectMetric::Count,
        )
        .with_action(crate::effect::PriorEffectAction::Drawn)
        .with_player(PlayerFilter::IteratedPlayer);
        let actual_draws = Value::PriorEffectMetric {
            effect_id: crate::effect::EffectId(receipt_id),
            query,
        };
        let shortfall = Value::Add(
            Box::new(Value::Fixed(2)),
            Box::new(Value::Scaled(Box::new(actual_draws), -1)),
        );
        vec![
            Effect::new(choice),
            Effect::for_players(
                PlayerFilter::Any,
                vec![Effect::with_id(
                    7,
                    Effect::new(crate::effects::DrawCardsEffect::new(
                        Value::IteratedPlayerChosenNumber,
                        draw_player,
                    )),
                )],
            ),
            Effect::for_players(
                PlayerFilter::Any,
                vec![Effect::new(crate::effects::GainLifeEffect::with_filter(
                    Value::Scaled(Box::new(shortfall), 2),
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
        ]
    }

    #[test]
    fn shortfall_surface_requires_the_exact_draw_receipt_and_player() {
        assert!(describe_each_player_draw_shortfall(&effects(7, PlayerFilter::IteratedPlayer))
            .is_some());
        assert!(describe_each_player_draw_shortfall(&effects(8, PlayerFilter::IteratedPlayer))
            .is_none());
        assert!(describe_each_player_draw_shortfall(&effects(7, PlayerFilter::You)).is_none());
    }
}

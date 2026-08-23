use super::*;

fn single_for_players_effect(effect: &Effect, filter: PlayerFilter) -> Option<&Effect> {
    let for_players = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != filter
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let [only] = for_players.effects.as_slice() else {
        return None;
    };
    Some(structural_unwrap_render_wrappers(only))
}

pub(in crate::compiled_text) fn describe_quantified_player_mill_discard_draw(
    sequence: &crate::effects::SequenceEffect,
) -> Option<String> {
    if sequence.surface != ironsmith_core::SequenceSurface::CommaThen {
        return None;
    }
    let [mill_players, discard_opponents, draw_controller] = sequence.effects.as_slice() else {
        return None;
    };

    let mill = single_for_players_effect(mill_players, PlayerFilter::Any)?
        .downcast_ref::<crate::effects::MillEffect>()?;
    if mill.player != PlayerFilter::IteratedPlayer {
        return None;
    }
    let discard = single_for_players_effect(discard_opponents, PlayerFilter::Opponent)?
        .downcast_ref::<crate::effects::DiscardEffect>()?;
    if discard.player != PlayerFilter::IteratedPlayer
        || discard.random
        || discard.any_number
        || discard.card_filter.is_some()
    {
        return None;
    }
    let draw = structural_unwrap_render_wrappers(draw_controller)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    if draw.player != PlayerFilter::You {
        return None;
    }

    let mill_text = describe_effect(mill_players)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let discard_text = describe_effect(discard_opponents)
        .trim()
        .trim_end_matches('.')
        .to_string();
    Some(format!(
        "{mill_text}, then {} and you draw {}",
        lowercase_first(&discard_text),
        describe_card_count(&draw.count)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sequence() -> crate::effects::SequenceEffect {
        crate::effects::SequenceEffect::comma_then(vec![
            Effect::for_players(
                PlayerFilter::Any,
                vec![Effect::new(crate::effects::MillEffect::new(
                    Value::Fixed(3),
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
            Effect::for_players(
                PlayerFilter::Opponent,
                vec![Effect::new(crate::effects::DiscardEffect::new(
                    Value::Fixed(1),
                    PlayerFilter::IteratedPlayer,
                    false,
                ))],
            ),
            Effect::draw(Value::Fixed(1)),
        ])
    }

    #[test]
    fn renders_three_explicit_player_scopes_without_repeating_the_tail() {
        assert_eq!(
            describe_quantified_player_mill_discard_draw(&sequence()).as_deref(),
            Some(
                "Each player mills three cards, then each opponent discards a card and you draw a card"
            )
        );
    }

    #[test]
    fn rejects_a_tail_that_is_still_inside_the_each_player_scope() {
        let mut nested = sequence();
        let mill_players = nested.effects[0]
            .downcast_ref::<crate::effects::ForPlayersEffect>()
            .expect("first effect should be a player fanout");
        let mut repeated_effects = mill_players.effects.clone();
        repeated_effects.extend(nested.effects.drain(1..));
        nested.effects = vec![Effect::for_players(PlayerFilter::Any, repeated_effects)];

        assert!(describe_quantified_player_mill_discard_draw(&nested).is_none());
    }

    #[test]
    fn rejects_a_non_comma_then_sequence_or_wrong_draw_actor() {
        let mut coordinated = sequence();
        coordinated.surface = ironsmith_core::SequenceSurface::Coordinated;
        assert!(describe_quantified_player_mill_discard_draw(&coordinated).is_none());

        let mut wrong_actor = sequence();
        wrong_actor.effects[2] = Effect::new(crate::effects::DrawCardsEffect::new(
            Value::Fixed(1),
            PlayerFilter::IteratedPlayer,
        ));
        assert!(describe_quantified_player_mill_discard_draw(&wrong_actor).is_none());
    }
}

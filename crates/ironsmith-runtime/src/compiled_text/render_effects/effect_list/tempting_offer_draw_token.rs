use super::*;

fn coordinated_draw_and_token<'a>(
    effect: &'a Effect,
    player: &PlayerFilter,
    explicit_create_actor: bool,
) -> Option<(
    &'a crate::effects::DrawCardsEffect,
    &'a crate::effects::CreateTokenEffect,
)> {
    let sequence = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.result_label.is_some()
    {
        return None;
    }
    let [draw_effect, create_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let create = structural_unwrap_render_wrappers(create_effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    if &draw.player != player
        || &create.controller != player
        || create.controller_target.is_some()
        || create.actor_surface_explicit != explicit_create_actor
    {
        return None;
    }
    Some((draw, create))
}

fn same_token_creation(
    first: &crate::effects::CreateTokenEffect,
    other: &crate::effects::CreateTokenEffect,
) -> bool {
    let mut first = first.clone();
    let mut other = other.clone();
    first.controller = PlayerFilter::You;
    other.controller = PlayerFilter::You;
    first.controller_target = None;
    other.controller_target = None;
    first.actor_surface_explicit = false;
    other.actor_surface_explicit = false;
    first.count = first.count.unhinted().clone();
    other.count = other.count.unhinted().clone();
    other.token.card.id = first.token.card.id;
    first == other
}

fn verb_phrase(effect: &Effect) -> String {
    let rendered = describe_effect(effect)
        .trim()
        .trim_end_matches('.')
        .to_string();
    let rendered = rendered
        .strip_prefix("You ")
        .or_else(|| rendered.strip_prefix("you "))
        .unwrap_or(&rendered);
    lowercase_first(rendered)
}

/// Restore the authored Tempting Offer procedure for the exact correlated
/// draw-and-token shape. The effect ID remains the semantic link between the
/// opponent's choice and the controller's reward; this changes only the
/// ability-word, sentence boundary, and repeated actor surfaces.
pub(super) fn describe_tempting_offer_draw_and_token(effects: &[Effect]) -> Option<String> {
    let [initial_effect, opponents_effect] = effects else {
        return None;
    };
    let (initial_draw, initial_create) =
        coordinated_draw_and_token(initial_effect, &PlayerFilter::You, false)?;

    let opponents = structural_unwrap_render_wrappers(opponents_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if opponents.filter != PlayerFilter::Opponent
        || opponents.starting_with_controller
        || opponents.stop_after_first_happened
    {
        return None;
    }
    let [offer_effect, reward_effect] = opponents.effects.as_slice() else {
        return None;
    };
    let offer = offer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = offer.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer)
        || may.fallback != crate::decision::FallbackStrategy::Decline
    {
        return None;
    }
    let [opponent_actions] = may.effects.as_slice() else {
        return None;
    };
    let (opponent_draw, opponent_create) =
        coordinated_draw_and_token(opponent_actions, &PlayerFilter::IteratedPlayer, false)?;

    let reward = reward_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if reward.condition != offer.id
        || reward.predicate != crate::effect::EffectPredicate::Chosen
        || !reward.else_.is_empty()
        || reward.per_player_result
        || reward.prior_result_replacement_surface
    {
        return None;
    }
    let [reward_actions] = reward.then.as_slice() else {
        return None;
    };
    let (reward_draw, reward_create) =
        coordinated_draw_and_token(reward_actions, &PlayerFilter::You, true)?;

    if initial_draw.count.unhinted() != opponent_draw.count.unhinted()
        || initial_draw.count.unhinted() != reward_draw.count.unhinted()
        || !same_token_creation(initial_create, opponent_create)
        || !same_token_creation(initial_create, reward_create)
    {
        return None;
    }

    let initial_sequence = initial_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    let draw = verb_phrase(&initial_sequence.effects[0]);
    let create = verb_phrase(&initial_sequence.effects[1]);
    Some(format!(
        "Tempting Offer — {} and {create}. Then each opponent may {draw} and {create}. For each opponent who does, you {draw} and you {create}",
        capitalize_first(&draw)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rabbit() -> crate::cards::CardDefinition {
        crate::cards::builders::CardDefinitionBuilder::new(crate::CardId::new(), "Rabbit")
            .token()
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Rabbit])
            .color_indicator(crate::ColorSet::WHITE)
            .power_toughness(crate::PowerToughness::fixed(1, 1))
            .build()
    }

    fn actions(player: PlayerFilter, explicit_create_actor: bool) -> Effect {
        let mut create = crate::effects::CreateTokenEffect::new(rabbit(), 1, player.clone());
        create.actor_surface_explicit = explicit_create_actor;
        Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::DrawCardsEffect {
                count: Value::Fixed(1),
                player,
            }),
            Effect::new(create),
        ]))
    }

    fn fixture(result_id: u32, condition_id: u32, reward_player: PlayerFilter) -> Vec<Effect> {
        vec![
            actions(PlayerFilter::You, false),
            Effect::for_players(
                PlayerFilter::Opponent,
                vec![
                    Effect::with_id(
                        result_id,
                        Effect::may_player(
                            PlayerFilter::IteratedPlayer,
                            vec![actions(PlayerFilter::IteratedPlayer, false)],
                        ),
                    ),
                    Effect::new(crate::effects::IfEffect::if_then(
                        crate::effect::EffectId(condition_id),
                        crate::effect::EffectPredicate::Chosen,
                        vec![actions(reward_player, true)],
                    )),
                ],
            ),
        ]
    }

    #[test]
    fn tempting_offer_requires_the_same_choice_id_actor_and_token_program() {
        const ORACLE: &str = "Tempting Offer — Draw a card and create a 1/1 white Rabbit creature token. Then each opponent may draw a card and create a 1/1 white Rabbit creature token. For each opponent who does, you draw a card and you create a 1/1 white Rabbit creature token";
        assert_eq!(
            describe_tempting_offer_draw_and_token(&fixture(7, 7, PlayerFilter::You)).as_deref(),
            Some(ORACLE)
        );

        let wrong_id = fixture(7, 8, PlayerFilter::You);
        assert!(describe_tempting_offer_draw_and_token(&wrong_id).is_none());

        let wrong_actor = fixture(7, 7, PlayerFilter::Opponent);
        assert!(describe_tempting_offer_draw_and_token(&wrong_actor).is_none());
    }
}

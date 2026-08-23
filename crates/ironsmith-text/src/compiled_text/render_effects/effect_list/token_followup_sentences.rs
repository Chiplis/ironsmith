use super::*;

fn rendered_clause(effect: &Effect) -> String {
    describe_effect(effect)
        .trim()
        .trim_end_matches('.')
        .to_string()
}

fn create_token_for_you(
    effect: &Effect,
    actor_surface_explicit: bool,
) -> Option<&crate::effects::CreateTokenEffect> {
    let create = structural_unwrap_render_wrappers(effect)
        .downcast_ref::<crate::effects::CreateTokenEffect>()?;
    (create.controller == PlayerFilter::You
        && create.controller_target.is_none()
        && create.actor_surface_explicit == actor_surface_explicit)
        .then_some(create)
}

fn describe_draw_lose_then_create(effects: &[Effect]) -> Option<String> {
    let [sequence_effect] = effects else {
        return None;
    };
    let sequence = sequence_effect.downcast_ref::<crate::effects::SequenceEffect>()?;
    if sequence.surface != ironsmith_core::SequenceSurface::Coordinated
        || sequence.result_label.is_some()
    {
        return None;
    }
    let [draw_effect, lose_effect, create_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let draw = structural_unwrap_render_wrappers(draw_effect)
        .downcast_ref::<crate::effects::DrawCardsEffect>()?;
    let lose = structural_unwrap_render_wrappers(lose_effect)
        .downcast_ref::<crate::effects::LoseLifeEffect>()?;
    create_token_for_you(create_effect, true)?;
    if draw.player != PlayerFilter::You || lose.player != ChooseSpec::Player(PlayerFilter::You) {
        return None;
    }

    let draw = rendered_clause(draw_effect);
    let lose = rendered_clause(lose_effect);
    let create = rendered_clause(create_effect);
    let draw = draw
        .strip_prefix("You ")
        .or_else(|| draw.strip_prefix("you "))
        .unwrap_or(&draw);
    let lose = lose
        .strip_prefix("You ")
        .or_else(|| lose.strip_prefix("you "))?;
    let create = create
        .strip_prefix("You ")
        .or_else(|| create.strip_prefix("you "))?;
    Some(format!(
        "You {}, {lose}, then {create}",
        lowercase_first(draw)
    ))
}

fn describe_damage_then_create_sentence(effects: &[Effect]) -> Option<String> {
    let [damage_effect, create_effect] = effects else {
        return None;
    };
    let for_players = structural_unwrap_render_wrappers(damage_effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
    {
        return None;
    }
    let [inner] = for_players.effects.as_slice() else {
        return None;
    };
    let damage = structural_unwrap_render_wrappers(inner)
        .downcast_ref::<crate::effects::DealDamageEffect>()?;
    if !matches!(
        damage.target.base(),
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    ) {
        return None;
    }
    let create = create_token_for_you(create_effect, false)?;
    if create.count.unhinted() != &Value::Fixed(1) {
        return None;
    }
    Some(format!(
        "{}. {}",
        rendered_clause(damage_effect),
        rendered_clause(create_effect)
    ))
}

fn describe_sacrifice_then_create_for_result_sentence(effects: &[Effect]) -> Option<String> {
    let [producer_effect, create_effect] = effects else {
        return None;
    };
    let producer = producer_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let for_players = structural_unwrap_render_wrappers(&producer.effect)
        .downcast_ref::<crate::effects::ForPlayersEffect>()?;
    if for_players.filter != PlayerFilter::Opponent
        || for_players.starting_with_controller
        || for_players.stop_after_first_happened
        || !for_players.effects.iter().any(|effect| {
            structural_unwrap_render_wrappers(effect)
                .downcast_ref::<ironsmith_core::SacrificePlayerEffect>()
                .is_some_and(|sacrifice| sacrifice.player == PlayerFilter::IteratedPlayer)
        })
    {
        return None;
    }
    let create = create_token_for_you(create_effect, false)?;
    let Value::PriorEffectMetric { effect_id, query } = create.count.unhinted() else {
        return None;
    };
    if *effect_id != producer.id
        || query.source != ironsmith_core::EffectMetricSource::AffectedObjects
        || query.metric != ironsmith_core::EffectMetric::Count
        || query.action != Some(ironsmith_core::PriorEffectAction::Sacrificed)
        || !query.filter.as_ref().is_some_and(|filter| {
            filter.card_types == [CardType::Creature] && filter.zone == Some(Zone::Battlefield)
        })
    {
        return None;
    }

    Some(format!(
        "{}. {}",
        rendered_clause(producer_effect),
        rendered_clause(create_effect)
    ))
}

pub(in crate::compiled_text) fn describe_token_followup_sentence_surface(
    effects: &[Effect],
) -> Option<String> {
    describe_draw_lose_then_create(effects)
        .or_else(|| describe_damage_then_create_sentence(effects))
        .or_else(|| describe_sacrifice_then_create_for_result_sentence(effects))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blood(actor_surface_explicit: bool) -> Effect {
        let mut create = crate::effects::CreateTokenEffect::new(
            crate::cards::tokens::blood_token_definition(),
            1,
            PlayerFilter::You,
        );
        create.actor_surface_explicit = actor_surface_explicit;
        Effect::new(create)
    }

    #[test]
    fn draw_lose_create_requires_one_explicit_controller_subject() {
        let effects = vec![Effect::new(crate::effects::SequenceEffect::coordinated(
            vec![
                Effect::new(crate::effects::DrawCardsEffect::you(2)),
                Effect::new(crate::effects::LoseLifeEffect {
                    amount: Value::Fixed(2),
                    player: ChooseSpec::Player(PlayerFilter::You),
                }),
                blood(true),
            ],
        ))];
        assert_eq!(
            describe_token_followup_sentence_surface(&effects).as_deref(),
            Some("You draw two cards, lose 2 life, then create a Blood token")
        );

        let mut wrong_actor = effects;
        wrong_actor[0] = Effect::new(crate::effects::SequenceEffect::coordinated(vec![
            Effect::new(crate::effects::DrawCardsEffect::you(2)),
            Effect::new(crate::effects::LoseLifeEffect {
                amount: Value::Fixed(2),
                player: ChooseSpec::Player(PlayerFilter::Opponent),
            }),
            blood(true),
        ]));
        assert!(describe_token_followup_sentence_surface(&wrong_actor).is_none());
    }

    #[test]
    fn damage_and_sacrifice_followups_require_the_same_quantified_actor_and_result_id() {
        let damage = Effect::for_players(
            PlayerFilter::Opponent,
            vec![Effect::deal_damage(
                1,
                ChooseSpec::Player(PlayerFilter::IteratedPlayer),
            )],
        );
        assert!(
            describe_token_followup_sentence_surface(&[damage.clone(), blood(false)]).is_some()
        );
        let wrong_recipient = Effect::for_players(
            PlayerFilter::Opponent,
            vec![Effect::deal_damage(
                1,
                ChooseSpec::Player(PlayerFilter::You),
            )],
        );
        assert!(
            describe_token_followup_sentence_surface(&[wrong_recipient, blood(false)]).is_none()
        );

        let mut creature = ObjectFilter::creature().in_zone(Zone::Battlefield);
        creature.controller = Some(PlayerFilter::IteratedPlayer);
        let producer = Effect::with_id(
            7,
            Effect::for_players(
                PlayerFilter::Opponent,
                vec![Effect::new(ironsmith_core::SacrificePlayerEffect::new(
                    creature.clone(),
                    1,
                    PlayerFilter::IteratedPlayer,
                ))],
            ),
        );
        let result_count = |effect_id| Value::PriorEffectMetric {
            effect_id: crate::effect::EffectId(effect_id),
            query: ironsmith_core::PriorEffectMetricQuery::new(
                ironsmith_core::EffectMetricSource::AffectedObjects,
                ironsmith_core::EffectMetric::Count,
            )
            .with_filter(ObjectFilter::creature().in_zone(Zone::Battlefield))
            .with_action(ironsmith_core::PriorEffectAction::Sacrificed),
        };
        let food = |effect_id| {
            Effect::new(crate::effects::CreateTokenEffect::new(
                crate::cards::tokens::food_token_definition(),
                result_count(effect_id),
                PlayerFilter::You,
            ))
        };
        assert!(describe_token_followup_sentence_surface(&[producer.clone(), food(7)]).is_some());
        assert!(describe_token_followup_sentence_surface(&[producer, food(8)]).is_none());
    }
}

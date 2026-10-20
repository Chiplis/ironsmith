use super::super::*;

/// A revealed outside-game pool remains a concrete tagged collection. The
/// delegated chooser selects only from that pool, and only its selected card
/// moves to the ability controller's hand. All five executable effects must
/// agree before the three authored clauses can share their shorter surface.
pub(super) fn describe(effects: &[Effect]) -> Option<String> {
    let [pool_effect, reveal_effect, player_effect, choice_effect, move_effect] = effects else {
        return None;
    };
    let pool = pool_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let count = pool.count.min;
    if count < 2 || pool.count != crate::effect::ChoiceCount::exactly(count) {
        return None;
    }
    let mut owned_outside = ObjectFilter::default()
        .owned_by(PlayerFilter::You)
        .in_zone(Zone::OutsideGame);
    owned_outside.distinct_names = true;
    let expected_pool = crate::effects::ChooseObjectsEffect::new(
        owned_outside,
        crate::effect::ChoiceCount::exactly(count),
        PlayerFilter::You,
        pool.tag.clone(),
    )
    .in_zone(Zone::OutsideGame);
    if *pool != expected_pool {
        return None;
    }

    let reveal = reveal_effect.downcast_ref::<crate::effects::RevealTaggedEffect>()?;
    if reveal.tag != pool.tag {
        return None;
    }
    let chooser = player_effect.downcast_ref::<crate::effects::ChoosePlayerEffect>()?;
    if *chooser != crate::effects::ChoosePlayerEffect::new(
        PlayerFilter::You,
        PlayerFilter::Opponent,
        chooser.tag.clone(),
    ) {
        return None;
    }
    let choice = choice_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    let expected_choice = crate::effects::ChooseObjectsEffect::new(
        ObjectFilter::tagged(pool.tag.clone()).in_zone(Zone::OutsideGame),
        crate::effect::ChoiceCount::exactly(1),
        PlayerFilter::TaggedPlayer(chooser.tag.clone()),
        choice.tag.clone(),
    )
    .in_zone(Zone::OutsideGame)
    .present_revealed_choices();
    if *choice != expected_choice {
        return None;
    }

    let tagged_move = move_effect.downcast_ref::<crate::effects::TaggedEffect>()?;
    if !tagged_move.outcome_only {
        return None;
    }
    let move_to_hand = tagged_move
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let expected_move = crate::effects::MoveToZoneEffect::new(
        ChooseSpec::Tagged(choice.tag.clone()),
        Zone::Hand,
        false,
    )
    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put);
    if *move_to_hand != expected_move {
        return None;
    }

    let count = i32::try_from(count)
        .ok()
        .and_then(number_word)
        .unwrap_or_else(|| count.to_string());
    Some(format!(
        "Reveal exactly {count} cards you own with different names from outside the game. An opponent chooses one of them. You put that card into your hand"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn effects() -> Vec<Effect> {
        let pool = TagKey::from("revealed_pool");
        let chooser = TagKey::from("chosen_opponent");
        let chosen = TagKey::from("chosen_card");
        let mut filter = ObjectFilter::default()
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::OutsideGame);
        filter.distinct_names = true;
        let pool_choice = crate::effects::ChooseObjectsEffect::new(
            filter,
            crate::effect::ChoiceCount::exactly(2),
            PlayerFilter::You,
            pool.clone(),
        )
        .in_zone(Zone::OutsideGame);
        let delegated_choice = crate::effects::ChooseObjectsEffect::new(
            ObjectFilter::tagged(pool.clone()).in_zone(Zone::OutsideGame),
            crate::effect::ChoiceCount::exactly(1),
            PlayerFilter::TaggedPlayer(chooser.clone()),
            chosen.clone(),
        )
        .in_zone(Zone::OutsideGame)
        .present_revealed_choices();
        let move_to_hand = crate::effects::MoveToZoneEffect::new(
            ChooseSpec::Tagged(chosen),
            Zone::Hand,
            false,
        )
        .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put);
        let mut tagged_move = crate::effects::TaggedEffect::new(
            TagKey::from("move_result"),
            Effect::new(move_to_hand),
        );
        tagged_move.outcome_only = true;
        vec![
            Effect::new(pool_choice),
            Effect::new(crate::effects::RevealTaggedEffect::new(pool)),
            Effect::new(crate::effects::ChoosePlayerEffect::new(
                PlayerFilter::You,
                PlayerFilter::Opponent,
                chooser,
            )),
            Effect::new(delegated_choice),
            Effect::new(tagged_move),
        ]
    }

    #[test]
    fn preserves_exact_revealed_pool_and_delegated_choice() {
        let effects = effects();
        assert_eq!(
            describe(&effects).as_deref(),
            Some("Reveal exactly two cards you own with different names from outside the game. An opponent chooses one of them. You put that card into your hand")
        );
        let wrapped = Effect::new(crate::effects::MayEffect::new(effects));
        assert_eq!(
            describe_effect(&wrapped),
            "You may reveal exactly two cards you own with different names from outside the game. An opponent chooses one of them. You put that card into your hand"
        );
    }

    #[test]
    fn rejects_broader_pool_or_different_destination() {
        let mut scenario = effects();
        let choose = scenario[3].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            .unwrap();
        let mut broader = choose.clone();
        broader.filter.tagged_constraints.clear();
        scenario[3] = Effect::new(broader);
        assert!(describe(&scenario).is_none());

        let mut scenario = effects();
        let move_effect = scenario[4].downcast_ref::<crate::effects::TaggedEffect>()
            .unwrap();
        let mut changed_move = move_effect.clone();
        let mut move_to_hand = changed_move.effect
            .downcast_ref::<crate::effects::MoveToZoneEffect>()
            .unwrap()
            .clone();
        move_to_hand.zone = Zone::Battlefield;
        changed_move.effect = Box::new(Effect::new(move_to_hand));
        scenario[4] = Effect::new(changed_move);
        assert!(describe(&scenario).is_none());
    }
}

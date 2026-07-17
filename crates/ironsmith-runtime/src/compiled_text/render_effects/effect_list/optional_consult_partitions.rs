use super::*;

fn visible_optional_consult_effects(effects: &[Effect]) -> &[Effect] {
    let hidden_prefix = effects
        .iter()
        .take_while(|effect| {
            effect
                .downcast_ref::<crate::effects::TagTriggeringObjectEffect>()
                .is_some()
        })
        .count();
    &effects[hidden_prefix..]
}

fn optional_action_and_happened_branch(
    effects: &[Effect],
) -> Option<(&crate::effects::MayEffect, &[Effect])> {
    let [may_effect, if_effect] = visible_optional_consult_effects(effects) else {
        return None;
    };
    let with_id = may_effect.downcast_ref::<crate::effects::WithIdEffect>()?;
    let may = with_id.effect.downcast_ref::<crate::effects::MayEffect>()?;
    if !matches!(may.decider, None | Some(PlayerFilter::You)) {
        return None;
    }

    let conditional = if_effect.downcast_ref::<crate::effects::IfEffect>()?;
    if conditional.condition != with_id.id
        || conditional.predicate != EffectPredicate::Happened
        || !conditional.else_.is_empty()
    {
        return None;
    }
    Some((may, conditional.then.as_slice()))
}

fn describe_optional_consult_then_battlefield_partition(
    may: &crate::effects::MayEffect,
    followups: &[Effect],
) -> Option<String> {
    let [consult_effect] = may.effects.as_slice() else {
        return None;
    };
    consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let [move_effect, remainder_effect] = followups else {
        return None;
    };

    let compact = render_consult_reveal_put_battlefield_rest_graveyard(&[
        consult_effect,
        move_effect,
        remainder_effect,
    ])?;
    let (consult_text, disposition) = compact.split_once(". ")?;
    let reveal = consult_text.strip_prefix("Reveal ")?;
    Some(format!(
        "You may reveal {reveal}. If you do, {}",
        lowercase_first(disposition)
    ))
}

fn describe_optional_payment_then_consult_hand_partition(
    may: &crate::effects::MayEffect,
    followups: &[Effect],
) -> Option<String> {
    let payment = describe_may_compound_payment(may)?;
    let [consult_effect, move_effect, remainder_effect] = followups else {
        return None;
    };
    render_consult_reveal_put_hand_rest_graveyard(&[
        consult_effect,
        move_effect,
        remainder_effect,
    ])?;

    let consult = describe_effect(consult_effect);
    let consult = consult.trim().trim_end_matches('.');
    let consult = lowercase_first(consult);
    Some(format!(
        "{}. If you do, {consult}. Put that card into your hand and the rest into your graveyard",
        capitalize_first(&payment)
    ))
}

/// Reconstruct optional reveal-until procedures whose parser AST deliberately
/// separates the optional action from an `If you do` disposition. The effect
/// ID proves the gate, while the consult's result tags prove the selected-card
/// and exact-remainder partition. This matcher is independent of card names
/// and helper-tag spelling.
pub(super) fn describe_optional_gated_consult_partition(effects: &[Effect]) -> Option<String> {
    let (may, followups) = optional_action_and_happened_branch(effects)?;
    describe_optional_consult_then_battlefield_partition(may, followups)
        .or_else(|| describe_optional_payment_then_consult_hand_partition(may, followups))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consult_partition(filter: ObjectFilter, zone: Zone) -> (Effect, Effect, Effect) {
        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::You,
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            filter,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ));
        let move_match = Effect::new(
            crate::effects::MoveToZoneEffect::new(
                ChooseSpec::Tagged(match_tag.clone()),
                zone,
                false,
            )
            .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put),
        )
        .tag("moved");

        let mut match_filter = ObjectFilter::default();
        match_filter
            .tagged_constraints
            .push(crate::filter::TaggedObjectConstraint {
                tag: match_tag,
                relation: crate::filter::TaggedOpbjectRelation::IsTaggedObject,
            });
        let move_remainder = Effect::new(crate::effects::ForEachTaggedEffect::new(
            all_tag,
            vec![Effect::conditional(
                Condition::TaggedObjectMatches(TagKey::from("__it__"), match_filter),
                vec![],
                vec![Effect::new(
                    crate::effects::MoveToZoneEffect::new(
                        ChooseSpec::Iterated,
                        Zone::Graveyard,
                        false,
                    )
                    .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put),
                )],
            )],
        ));
        (consult, move_match, move_remainder)
    }

    fn creature_filter() -> ObjectFilter {
        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Creature);
        filter
    }

    fn land_filter() -> ObjectFilter {
        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Land);
        filter
    }

    #[test]
    fn avenging_druid_optional_consult_keeps_its_if_you_do_partition() {
        let (consult, move_match, move_remainder) =
            consult_partition(land_filter(), Zone::Battlefield);
        let effects = vec![
            Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
            Effect::with_id(0, Effect::may(vec![consult])),
            Effect::if_then(
                crate::effect::EffectId(0),
                EffectPredicate::Happened,
                vec![move_match, move_remainder],
            ),
        ];

        assert_eq!(
            describe_effect_list(&effects),
            "You may reveal cards from the top of your library until you reveal a land card. If you do, put that card onto the battlefield and put all other cards revealed this way into your graveyard"
        );
    }

    #[test]
    fn foster_payment_gates_the_complete_consult_and_rest_partition() {
        let (consult, move_match, move_remainder) =
            consult_partition(creature_filter(), Zone::Hand);
        let payment = Effect::new(crate::effects::PayManaEffect::new(
            crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]),
            ChooseSpec::Player(PlayerFilter::You),
        ));
        let effects = vec![
            Effect::new(crate::effects::TagTriggeringObjectEffect::new("triggering")),
            Effect::with_id(
                0,
                Effect::new(crate::effects::MayEffect::new_for_player(
                    vec![payment],
                    PlayerFilter::You,
                )),
            ),
            Effect::if_then(
                crate::effect::EffectId(0),
                EffectPredicate::Happened,
                vec![consult, move_match, move_remainder],
            ),
        ];

        assert_eq!(
            describe_effect_list(&effects),
            "You may pay {1}. If you do, reveal cards from the top of your library until you reveal a creature card. Put that card into your hand and the rest into your graveyard"
        );
    }
}

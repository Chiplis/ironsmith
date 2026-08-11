use super::*;

/// Render the exact targeted-opponent consult procedure whose optional cast
/// removes the matched card from exile before the still-exiled collection is
/// put on the bottom. The repeated target declaration is lowering scaffolding;
/// matching it to the consult and remainder player proves Oracle's `that
/// library` antecedent without changing executable targeting.
pub(super) fn describe_targeted_opponent_consult_may_cast_remainder(
    effects: &[&Effect],
) -> Option<String> {
    let [
        declaration_effect,
        consult_effect,
        may_effect,
        repeated_declaration_effect,
        remainder_effect,
    ] = effects
    else {
        return None;
    };
    let declaration = declaration_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let repeated_declaration =
        repeated_declaration_effect.downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let targeted_opponent = PlayerFilter::target_opponent();
    if declaration.chooser.is_some()
        || declaration.explicit_declaration
        || repeated_declaration.chooser.is_some()
        || repeated_declaration.explicit_declaration
        || choose_spec_player_filter(&declaration.target) != Some(targeted_opponent.clone())
        || choose_spec_player_filter(&repeated_declaration.target)
            != Some(targeted_opponent.clone())
        || consult.player != targeted_opponent
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Exile
        || consult.stop_rule != crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        || consult.max_exposed.is_some()
    {
        return None;
    }

    let may = may_effect.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = cast_effect.downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if may.decider != Some(PlayerFilter::You)
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || cast.tag != consult.match_tag
        || cast.player != PlayerFilter::You
        || cast.allow_land
        || cast.as_copy
        || cast.copy_cast_reminder_surface
        || !cast.without_paying_mana_cost
        || cast.additional_mana_cost.is_some()
        || cast.cost_reduction.is_some()
        || cast.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }

    let remainder = remainder_effect
        .downcast_ref::<crate::effects::PutTaggedRemainderOnLibraryBottomEffect>()?;
    if remainder.tag != consult.all_tag
        || remainder.keep_tagged.is_some()
        || remainder.player != consult.player
        || remainder.order != crate::effects::consult_helpers::LibraryBottomOrder::Random
        || remainder.surface != ironsmith_core::LibraryRemainderSurface::Rest
    {
        return None;
    }

    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    let stop_text =
        describe_consult_stop_text(&selection, &consult.stop_rule, consult.max_exposed.as_ref());
    Some(format!(
        "Target opponent exiles cards from the top of their library until they exile {stop_text}. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of that library in a random order"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn procedure(order: crate::effects::consult_helpers::LibraryBottomOrder) -> Vec<Effect> {
        let all_tag = TagKey::from("consulted");
        let match_tag = TagKey::from("matched");
        vec![
            Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::Target(
                Box::new(ChooseSpec::Player(PlayerFilter::Opponent)),
            ))),
            Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
                PlayerFilter::target_opponent(),
                crate::effects::consult_helpers::LibraryConsultMode::Exile,
                ObjectFilter::instant_or_sorcery(),
                crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
                all_tag.clone(),
                match_tag.clone(),
            )),
            Effect::new(crate::effects::MayEffect::new_for_player(
                vec![Effect::new(
                    crate::effects::CastTaggedEffect::new(match_tag, PlayerFilter::You)
                        .without_paying_mana_cost(),
                )],
                PlayerFilter::You,
            )),
            Effect::new(crate::effects::TargetOnlyEffect::new(ChooseSpec::Target(
                Box::new(ChooseSpec::Player(PlayerFilter::Opponent)),
            ))),
            Effect::new(
                crate::effects::PutTaggedRemainderOnLibraryBottomEffect::new(
                    all_tag,
                    None,
                    order,
                    PlayerFilter::target_opponent(),
                ),
            ),
        ]
    }

    #[test]
    fn targeted_opponent_random_bottom_consult_compacts_exactly() {
        let effects = procedure(crate::effects::consult_helpers::LibraryBottomOrder::Random);
        let refs = effects.iter().collect::<Vec<_>>();
        assert_eq!(
            describe_targeted_opponent_consult_may_cast_remainder(&refs),
            Some(
                "Target opponent exiles cards from the top of their library until they exile an instant or sorcery card. You may cast that card without paying its mana cost. Then put the exiled cards that weren't cast this way on the bottom of that library in a random order"
                    .to_string()
            )
        );
    }

    #[test]
    fn targeted_opponent_consult_does_not_claim_a_different_bottom_order() {
        let effects =
            procedure(crate::effects::consult_helpers::LibraryBottomOrder::ChooserChooses);
        let refs = effects.iter().collect::<Vec<_>>();
        assert_eq!(
            describe_targeted_opponent_consult_may_cast_remainder(&refs),
            None
        );
    }
}

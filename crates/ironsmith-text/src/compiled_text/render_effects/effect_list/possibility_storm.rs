use super::*;

/// Render the correlated cast-from-hand replacement-style process whose
/// consult relation is anchored to the triggering spell and whose cleanup is
/// the complete collection exiled with the source permanent.
///
/// Each guard is executable provenance: the triggering object is tagged and
/// exiled, the consult's type relation consumes that exact affected-object
/// tag, only its matching card may be cast, and the final random-bottom move
/// consumes the source-exiled set rather than the consult's temporary set.
pub(super) fn describe_cast_from_hand_consult_source_exiled_cleanup(
    effects: &[Effect],
) -> Option<String> {
    let [
        tag_triggering,
        exile_triggering,
        consult_effect,
        optional_cast,
        cleanup_effect,
    ] = effects
    else {
        return None;
    };

    let triggering = tag_triggering.downcast_ref::<crate::effects::TagTriggeringObjectEffect>()?;
    let exiled = exile_triggering.downcast_ref::<crate::effects::TaggedEffect>()?;
    let exile = exiled
        .effect
        .downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    if exile.zone != Zone::Exile
        || exile.actor_surface != Some(PlayerFilter::IteratedPlayer)
        || !matches!(exile.target.base(), ChooseSpec::Tagged(tag) if tag == &triggering.tag)
        || exile.enters_face_down
        || exile.transfer_exiled_with_source_links
    {
        return None;
    }

    let consult = consult_effect.downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    let [type_relation] = consult.filter.tagged_constraints.as_slice() else {
        return None;
    };
    if consult.player != PlayerFilter::IteratedPlayer
        || consult.mode != ironsmith_core::LibraryConsultMode::Exile
        || consult.stop_rule != crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        || consult.max_exposed.is_some()
        || type_relation.tag != exiled.tag
        || type_relation.relation != crate::filter::TaggedOpbjectRelation::SharesCardType
        || consult.all_tag == consult.match_tag
    {
        return None;
    }
    let mut base_consult_filter = consult.filter.clone();
    base_consult_filter.tagged_constraints.clear();
    base_consult_filter.set_explicit_card_noun(false);
    if base_consult_filter != ObjectFilter::default() {
        return None;
    }

    let may = optional_cast.downcast_ref::<crate::effects::MayEffect>()?;
    let [cast_effect] = may.effects.as_slice() else {
        return None;
    };
    let cast = structural_unwrap_render_wrappers(cast_effect)
        .downcast_ref::<crate::effects::CastTaggedEffect>()?;
    if may.decider != Some(PlayerFilter::IteratedPlayer)
        || may.fallback != crate::decision::FallbackStrategy::Decline
        || cast.tag != consult.match_tag
        || cast.player != PlayerFilter::IteratedPlayer
        || cast.allow_land
        || cast.as_copy
        || !cast.without_paying_mana_cost
        || cast.additional_mana_cost.is_some()
        || cast.cost_reduction.is_some()
        || cast.mana_spend_mode != ironsmith_core::value_model::ManaSpendMode::Normal
    {
        return None;
    }

    let cleanup = cleanup_effect.downcast_ref::<crate::effects::MoveToZoneEffect>()?;
    let ChooseSpec::All(cleanup_filter) = cleanup.target.base() else {
        return None;
    };
    let surface = cleanup.exiled_with_source_surface.as_ref()?;
    let ironsmith_core::ExiledWithSourceReferenceSurface::Source(
        crate::target::SourceReferenceSurface::ThisPermanentType(source),
    ) = &surface.source
    else {
        return None;
    };
    if !is_source_exiled_cards_filter(cleanup_filter)
        || cleanup.zone != Zone::Library
        || cleanup.to_top
        || cleanup.library_order != Some(ironsmith_core::LibraryPlacementOrder::Random)
        || cleanup.verb_surface != ironsmith_core::MoveToZoneVerbSurface::Put
        || cleanup.destination_player_surface != Some(PlayerFilter::IteratedPlayer)
        || cleanup.destination_player_reference_surface
            != Some(ironsmith_core::DestinationPlayerReferenceSurface::Pronoun)
        || cleanup.target_plural_surface
        || cleanup.remainder_surface.is_some()
        || surface.verb != ironsmith_core::ExiledWithSourceMoveVerbSurface::Put
        || surface.subject != ironsmith_core::ExiledWithSourceSubjectSurface::AllCards
        || surface.destination
            != ironsmith_core::ExiledWithSourceDestinationSurface::ContextualPlayer
    {
        return None;
    }

    Some(format!(
        "That player exiles it, then exiles cards from the top of their library until they exile a card that shares a card type with it. That player may cast that card without paying its mana cost. Then they put all cards exiled with {source} on the bottom of their library in a random order"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Vec<Effect> {
        let triggering = TagKey::from("triggering");
        let antecedent = TagKey::from("consult_antecedent");
        let all = TagKey::from("consult_all");
        let matched = TagKey::from("consult_match");

        let exile =
            ironsmith_core::MoveToZoneEffect::to_exile(ChooseSpec::Tagged(triggering.clone()))
                .with_actor_surface(PlayerFilter::IteratedPlayer);
        let consult_filter = ObjectFilter::default().match_tagged(
            antecedent.clone(),
            crate::filter::TaggedOpbjectRelation::SharesCardType,
        );
        let consult = crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::IteratedPlayer,
            ironsmith_core::LibraryConsultMode::Exile,
            consult_filter,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            all,
            matched.clone(),
        );
        let cast = crate::effects::CastTaggedEffect::new(matched, PlayerFilter::IteratedPlayer)
            .without_paying_mana_cost();
        let may = crate::effects::MayEffect::new_for_player(
            vec![Effect::new(cast)],
            PlayerFilter::IteratedPlayer,
        );
        let source_exiled = ObjectFilter::default().in_zone(Zone::Exile).match_tagged(
            crate::tag::SOURCE_EXILED_TAG,
            crate::filter::TaggedOpbjectRelation::IsTaggedObject,
        );
        let cleanup =
            ironsmith_core::MoveToZoneEffect::to_bottom_of_library(ChooseSpec::All(source_exiled))
                .with_library_order(ironsmith_core::LibraryPlacementOrder::Random)
                .with_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Put)
                .with_destination_player_surface(PlayerFilter::IteratedPlayer)
                .with_destination_player_reference_surface(
                    ironsmith_core::DestinationPlayerReferenceSurface::Pronoun,
                )
                .with_exiled_with_source_surface(ironsmith_core::ExiledWithSourceMoveSurface {
                    verb: ironsmith_core::ExiledWithSourceMoveVerbSurface::Put,
                    subject: ironsmith_core::ExiledWithSourceSubjectSurface::AllCards,
                    source: ironsmith_core::ExiledWithSourceReferenceSurface::Source(
                        crate::target::SourceReferenceSurface::ThisPermanentType(
                            "this enchantment".to_string(),
                        ),
                    ),
                    destination:
                        ironsmith_core::ExiledWithSourceDestinationSurface::ContextualPlayer,
                });

        vec![
            Effect::new(crate::effects::TagTriggeringObjectEffect::new(triggering)),
            Effect::new(crate::effects::TaggedEffect::new(
                antecedent,
                Effect::new(exile),
            )),
            Effect::new(consult),
            Effect::new(may),
            Effect::new(cleanup),
        ]
    }

    #[test]
    fn exact_correlated_consult_and_source_exiled_cleanup_compacts() {
        let effects = fixture();
        assert_eq!(
            describe_cast_from_hand_consult_source_exiled_cleanup(&effects).as_deref(),
            Some(
                "That player exiles it, then exiles cards from the top of their library until they exile a card that shares a card type with it. That player may cast that card without paying its mana cost. Then they put all cards exiled with this enchantment on the bottom of their library in a random order"
            )
        );

        let mut nonrandom = fixture();
        let cleanup = nonrandom
            .last_mut()
            .and_then(|effect| {
                let cloned = effect
                    .downcast_ref::<crate::effects::MoveToZoneEffect>()?
                    .clone();
                Some(cloned)
            })
            .expect("cleanup move");
        let mut nonrandom_cleanup = cleanup;
        nonrandom_cleanup.library_order = None;
        *nonrandom.last_mut().expect("cleanup slot") = Effect::new(nonrandom_cleanup);
        assert!(
            describe_cast_from_hand_consult_source_exiled_cleanup(&nonrandom).is_none(),
            "a nonrandom cleanup must not inherit the authored random-bottom surface"
        );
    }
}

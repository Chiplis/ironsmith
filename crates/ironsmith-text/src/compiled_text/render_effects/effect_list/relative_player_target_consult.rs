use super::*;

/// Render a targeted reveal-until procedure whose authored disposition puts
/// every nonmatching card into that opponent's graveyard before the ability
/// controller puts the matched card onto the battlefield. The target,
/// consult tags, complementary iteration, and battlefield controller prove
/// the complete relationship without consulting card identity or source text.
pub(in crate::compiled_text) fn describe_target_opponent_consult_remainder_then_match(
    effects: &[&Effect],
) -> Option<String> {
    let [target_effect, consult_effect, remainder_effect, put_effect] = effects else {
        return None;
    };
    let target = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    let consult = structural_unwrap_render_wrappers(consult_effect)
        .downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if target.chooser.is_some()
        || target.explicit_declaration
        || target.target != ChooseSpec::target_opponent()
        || consult.player != PlayerFilter::target_opponent()
        || consult.mode != crate::effects::consult_helpers::LibraryConsultMode::Reveal
        || consult.max_exposed.is_some()
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
                | crate::effects::ConsultTopOfLibraryStopRule::MatchCount(Value::Fixed(1))
        )
    {
        return None;
    }

    let [card_type] = consult.filter.card_types.as_slice() else {
        return None;
    };
    let mut semantic_filter = consult.filter.clone();
    semantic_filter.union_surface = crate::filter::ObjectFilterUnionSurface::default();
    if semantic_filter != ObjectFilter::default().with_type(*card_type) {
        return None;
    }

    let remainder = structural_unwrap_render_wrappers(remainder_effect)
        .downcast_ref::<crate::effects::ForEachTaggedEffect>()?;
    if !for_each_moves_unselected_to_zone(
        remainder,
        consult.all_tag.as_str(),
        consult.match_tag.as_str(),
        Zone::Graveyard,
    ) {
        return None;
    }

    let put = structural_unwrap_render_wrappers(put_effect)
        .downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
    if put.tapped
        || put.controller != PlayerFilter::You
        || !matches!(
            put.target.base(),
            ChooseSpec::Tagged(tag) if tag == &consult.match_tag
        )
    {
        return None;
    }

    Some(format!(
        "Target opponent reveals cards from the top of their library until they reveal a {}. That player puts all non{} cards revealed this way into their graveyard, then you put the {} onto the battlefield under your control",
        card_type.card_phrase(),
        card_type.name(),
        card_type.card_phrase(),
    ))
}

/// Render a procedure where a named player chooses an opponent who controls
/// more matching permanents than they do, then may perform a reveal-until
/// consult with the matching card kind. The typed chooser, relative-player
/// filter, and consult tags prove the full relationship; no card identity or
/// source text participates in the match.
pub(in crate::compiled_text) fn describe_relative_player_target_then_optional_consult(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, may_effect] = effects else {
        return None;
    };

    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.explicit_declaration
        || target_only.chooser.as_ref() != Some(&PlayerFilter::Active)
        || !target_only.target.is_target()
        || target_only.target.count() != crate::effect::ChoiceCount::exactly(1)
    {
        return None;
    }
    let ChooseSpec::Player(PlayerFilter::OpponentWithMoreControlledObjectsThan {
        player: reference_player,
        filter: controlled_filter,
        fewer: false,
        ..
    }) = target_only.target.base()
    else {
        return None;
    };
    if reference_player.as_ref() != &PlayerFilter::Active {
        return None;
    }

    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::Active) {
        return None;
    }
    let consult_effects = may
        .effects
        .iter()
        .map(structural_unwrap_render_wrappers)
        .collect::<Vec<_>>();
    // Reuse the generic consult recognizer to prove that the match tag moves
    // to the battlefield and every other revealed card moves to the graveyard.
    render_consult_reveal_put_battlefield_rest_graveyard(&consult_effects)?;

    let consult = consult_effects[0].downcast_ref::<crate::effects::ConsultTopOfLibraryEffect>()?;
    if consult.player != PlayerFilter::Active
        || consult.max_exposed.is_some()
        || !matches!(
            consult.stop_rule,
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch
        )
    {
        return None;
    }

    let mut controlled_card_filter = controlled_filter.as_ref().clone();
    controlled_card_filter.zone = None;
    if controlled_card_filter != consult.filter {
        return None;
    }

    let controlled_nouns = pluralize_noun_phrase(&controlled_filter.description());
    let selection = describe_library_consult_selection_with_cards(&consult.filter);
    Some(format!(
        "That player chooses target player who controls more {controlled_nouns} than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal {selection}. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard"
    ))
}

/// Render the analogous relative-player target followed by an optional
/// library search. The active/upkeep player is both the target chooser and
/// the owner/actor of the complete search procedure.
pub(in crate::compiled_text) fn describe_relative_player_target_then_optional_search(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, may_effect] = effects else {
        return None;
    };

    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.explicit_declaration
        || target_only.chooser.as_ref() != Some(&PlayerFilter::Active)
        || !target_only.target.is_target()
        || target_only.target.count() != crate::effect::ChoiceCount::exactly(1)
    {
        return None;
    }
    let ChooseSpec::Player(PlayerFilter::OpponentWithMoreControlledObjectsThan {
        player: reference_player,
        filter: controlled_filter,
        fewer: false,
        ..
    }) = target_only.target.base()
    else {
        return None;
    };
    if reference_player.as_ref() != &PlayerFilter::Active {
        return None;
    }

    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::Active) {
        return None;
    }
    let [sequence_effect] = may.effects.as_slice() else {
        return None;
    };
    let sequence = structural_unwrap_render_wrappers(sequence_effect)
        .downcast_ref::<crate::effects::SequenceEffect>()?;
    let [choose_effect, for_each_effect, shuffle_effect] = sequence.effects.as_slice() else {
        return None;
    };
    let choose = choose_effect.downcast_ref::<crate::effects::ChooseObjectsEffect>()?;
    if !choose.is_search
        || choose.reveal
        || choose.chooser != PlayerFilter::Active
        || choose.filter.zone.is_some_and(|zone| zone != Zone::Library)
        || choose.zone != Some(Zone::Library)
        || !choose.additional_zones.is_empty()
        || choose.count != crate::effect::ChoiceCount::exactly(1)
        || choose.filter.owner.as_ref() != Some(&PlayerFilter::Active)
    {
        return None;
    }
    let (put, matches_selection) = if let Some(for_each) =
        for_each_effect.downcast_ref::<crate::effects::ForEachTaggedEffect>()
    {
        let [put_effect] = for_each.effects.as_slice() else { return None; };
        let put = put_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
        (put, for_each.tag == choose.tag
            && for_each.controller_at_last_blocked_by.is_none()
            && matches!(put.target.base(), ChooseSpec::Iterated))
    } else {
        let put = for_each_effect.downcast_ref::<crate::effects::PutOntoBattlefieldEffect>()?;
        (put, matches!(put.target.base(), ChooseSpec::Tagged(tag) if tag == &choose.tag))
    };
    let shuffle = shuffle_effect.downcast_ref::<crate::effects::ShuffleLibraryEffect>()?;
    if !matches_selection
        || put.tapped
        || put.controller != PlayerFilter::Active
        || shuffle.player != PlayerFilter::Active
        || shuffle.target_spec.is_some()
    {
        return None;
    }

    let controlled_nouns = pluralize_noun_phrase(&controlled_filter.description());
    let mut selection_filter = choose.filter.clone();
    selection_filter.zone = None;
    selection_filter.owner = None;
    let selection = with_indefinite_article(strip_leading_article(&selection_filter.description()));
    Some(format!(
        "That player chooses target player who controls more {controlled_nouns} than they do and is their opponent. The first player may search their library for {selection}, put that card onto the battlefield, then shuffle"
    ))
}

/// "At the beginning of each player's upkeep, that player chooses target
/// player whose graveyard has fewer creature cards in it than their graveyard
/// does and is their opponent. The first player may return a creature card
/// from their graveyard to their hand." (Oath of Ghouls, Oath of Scholars):
/// the upkeep player declares a relative opponent target, then may act.
pub(in crate::compiled_text) fn describe_relative_player_target_then_optional_action(
    effects: &[Effect],
) -> Option<String> {
    let [target_effect, may_effect] = effects else {
        return None;
    };
    let target_only = structural_unwrap_render_wrappers(target_effect)
        .downcast_ref::<crate::effects::TargetOnlyEffect>()?;
    if !target_only.explicit_declaration
        || target_only.chooser.as_ref() != Some(&PlayerFilter::Active)
        || !target_only.target.is_target()
        || target_only.target.count() != crate::effect::ChoiceCount::exactly(1)
    {
        return None;
    }
    let relation = match target_only.target.base() {
        ChooseSpec::Player(PlayerFilter::OpponentWithMoreControlledObjectsThan {
            player,
            filter,
            fewer: true,
            as_you_activate: false,
            minimum_difference: 0,
        }) if player.as_ref() == &PlayerFilter::Active
            && filter.zone == Some(Zone::Graveyard) =>
        {
            let mut counted = filter.as_ref().clone();
            counted.zone = None;
            counted.owner = None;
            format!(
                "whose graveyard has fewer {} in it than their graveyard does",
                pluralize_noun_phrase(strip_leading_article(&counted.description()))
            )
        }
        ChooseSpec::Player(PlayerFilter::QuantityGreaterThanPlayer {
            base,
            other,
            quantity,
        }) if base.as_ref() == &PlayerFilter::OpponentOf(Box::new(PlayerFilter::Active))
            && other.as_ref() == &PlayerFilter::Active =>
        {
            format!("who has more {} than they do", quantity.noun_phrase())
        }
        _ => return None,
    };
    let may = structural_unwrap_render_wrappers(may_effect)
        .downcast_ref::<crate::effects::MayEffect>()?;
    if may.decider.as_ref() != Some(&PlayerFilter::Active) {
        return None;
    }
    let action = match may.effects.as_slice() {
        [discard_effect, draw_effect] => {
            let discard = discard_effect.downcast_ref::<crate::effects::DiscardHandEffect>()?;
            let draw = draw_effect.downcast_ref::<crate::effects::DrawCardsEffect>()?;
            let Value::Fixed(count) = draw.count else {
                return None;
            };
            if discard.player != PlayerFilter::Active || draw.player != PlayerFilter::Active {
                return None;
            }
            let count_text = number_word(count).unwrap_or_else(|| count.to_string());
            let noun = if count == 1 { "card" } else { "cards" };
            format!("discard their hand and draw {count_text} {noun}")
        }
        // "The first player may have this enchantment deal 1 damage to the
        // second player." (Oath of Mages): the damage recipient is the
        // declared target, so it is named as the second player.
        [damage_effect]
            if damage_effect
                .downcast_ref::<crate::effects::DealDamageEffect>()
                .is_some() =>
        {
            let damage = damage_effect.downcast_ref::<crate::effects::DealDamageEffect>()?;
            let ChooseSpec::Player(PlayerFilter::AliasedTarget(aliased)) = &damage.target else {
                return None;
            };
            let ChooseSpec::Player(declared) = target_only.target.base() else {
                return None;
            };
            let Value::Fixed(amount) = damage.amount else {
                return None;
            };
            if aliased.as_ref() != declared
                || damage.source_is_combat
                || damage.unpreventable
                || damage.excess_to_controller.is_some()
            {
                return None;
            }
            format!("have this permanent deal {amount} damage to the second player")
        }
        [return_effect] => {
            let return_from_gy = return_effect
                .downcast_ref::<crate::effects::ReturnFromGraveyardToHandEffect>()?;
            if return_from_gy.random || return_from_gy.target.count() != crate::effect::ChoiceCount::exactly(1) {
                return None;
            }
            let ChooseSpec::Object(return_filter) = return_from_gy.target.base() else {
                return None;
            };
            if return_filter.zone != Some(Zone::Graveyard)
                || return_filter.owner.as_ref() != Some(&PlayerFilter::Active)
            {
                return None;
            }
            let mut card = return_filter.clone();
            card.zone = None;
            card.owner = None;
            format!(
                "return {} from their graveyard to their hand",
                with_indefinite_article(strip_leading_article(&card.description()))
            )
        }
        _ => return None,
    };
    Some(format!(
        "That player chooses target player {relation} and is their opponent. The first player may {action}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_trigger_route_keeps_relative_player_and_optional_consult() {
        let text = "At the beginning of each player's upkeep, that player chooses target player who controls more creatures than they do and is their opponent. The first player may reveal cards from the top of their library until they reveal a creature card. If the first player does, that player puts that card onto the battlefield and all other cards revealed this way into their graveyard.";
        let definition = crate::CardDefinitionBuilder::new(
            crate::ids::CardId::new(),
            "Relative Player Consult Probe",
        )
        .card_types(vec![CardType::Enchantment])
        .parse_text(text)
        .expect("relative-player consult route should compile");

        assert_eq!(
            crate::compiled_text::compiled_text_lines(&definition).join("\n"),
            text
        );
    }

    #[test]
    fn targeted_opponent_consult_keeps_complement_and_authored_disposition_order() {
        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_opponent(),
        ));
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::target_opponent(),
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            ObjectFilter::default().with_type(CardType::Creature),
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ));
        let remainder = Effect::new(crate::effects::ForEachTaggedEffect::new(
            all_tag,
            vec![Effect::new(crate::effects::ConditionalEffect::new(
                crate::effect::Condition::TaggedObjectMatches(
                    match_tag.clone(),
                    ObjectFilter::default().same_stable_id_as_tagged(TagKey::from("__it__")),
                ),
                Vec::new(),
                vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Iterated,
                    Zone::Graveyard,
                    false,
                ))],
            ))],
        ));
        let put = Effect::new(crate::effects::PutOntoBattlefieldEffect::you_control(
            ChooseSpec::Tagged(match_tag),
            false,
        ));

        assert_eq!(
            describe_target_opponent_consult_remainder_then_match(&[
                &target, &consult, &remainder, &put,
            ])
            .as_deref(),
            Some(
                "Target opponent reveals cards from the top of their library until they reveal a creature card. That player puts all noncreature cards revealed this way into their graveyard, then you put the creature card onto the battlefield under your control"
            )
        );

        let disposition = Effect::new(crate::effects::SequenceEffect::comma_then(vec![
            remainder, put,
        ]));
        assert_eq!(
            describe_cross_segment_consult_bundle(&[target, consult, disposition]).as_deref(),
            Some(
                "Target opponent reveals cards from the top of their library until they reveal a creature card. That player puts all noncreature cards revealed this way into their graveyard, then you put the creature card onto the battlefield under your control"
            )
        );
    }

    #[test]
    fn targeted_opponent_consult_requires_controller_owned_match_destination() {
        let all_tag = TagKey::from("revealed");
        let match_tag = TagKey::from("matched");
        let target = Effect::new(crate::effects::TargetOnlyEffect::new(
            ChooseSpec::target_opponent(),
        ));
        let consult = Effect::new(crate::effects::ConsultTopOfLibraryEffect::new(
            PlayerFilter::target_opponent(),
            crate::effects::consult_helpers::LibraryConsultMode::Reveal,
            ObjectFilter::default().with_type(CardType::Creature),
            crate::effects::ConsultTopOfLibraryStopRule::FirstMatch,
            all_tag.clone(),
            match_tag.clone(),
        ));
        let remainder = Effect::new(crate::effects::ForEachTaggedEffect::new(
            all_tag,
            vec![Effect::new(crate::effects::ConditionalEffect::new(
                crate::effect::Condition::TaggedObjectMatches(
                    match_tag.clone(),
                    ObjectFilter::default().same_stable_id_as_tagged(TagKey::from("__it__")),
                ),
                Vec::new(),
                vec![Effect::new(crate::effects::MoveToZoneEffect::new(
                    ChooseSpec::Iterated,
                    Zone::Graveyard,
                    false,
                ))],
            ))],
        ));
        let wrong_controller = Effect::new(crate::effects::PutOntoBattlefieldEffect::new(
            ChooseSpec::Tagged(match_tag),
            false,
            PlayerFilter::target_opponent(),
        ));

        assert!(
            describe_target_opponent_consult_remainder_then_match(&[
                &target,
                &consult,
                &remainder,
                &wrong_controller,
            ])
            .is_none()
        );
    }
}

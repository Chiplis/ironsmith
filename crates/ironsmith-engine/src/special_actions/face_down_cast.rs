//! Described face-down entries can independently grant a paid exile-and-cast
//! action. Spell announcement and payment use the ordinary casting owner.
use super::*;
use crate::alternative_cast::CastingMethod;
use crate::effect::{Effect, EffectOutcome};
use crate::target::ChooseSpec;
use ironsmith_core::DescribedFaceDownCastPermission;

pub(crate) fn register_face_down_exile_cast_action(
    game: &mut GameState, source: ObjectId, player: PlayerId,
    permission: &DescribedFaceDownCastPermission,
) {
    game.effect_store.repeatable_mana_payment_actions.push(crate::game_state::RepeatableManaPaymentAction {
        restrictions: crate::game_state::ImmediateActionRestrictions {
            source_zone: Some(Zone::Battlefield),
            source_controller_acts: true,
            face_down_exile_cast: Some(permission.clone()),
            ..Default::default()
        },
        player, source, controller: player, cost: ManaCost::new(), effects: Vec::new(),
        targets: Vec::new(), tagged_objects: Default::default(), tagged_players: Default::default(),
        expires_end_of_turn: u32::MAX, ends_continuous_effects: Vec::new(),
    });
}

fn cast_methods(
    game: &GameState, player: PlayerId, source: ObjectId, card_id: ObjectId,
    permission: &DescribedFaceDownCastPermission,
) -> Vec<(String, CastingMethod)> {
    let Some(card) = game.object(card_id) else { return Vec::new(); };
    if card.is_land() || (permission.require_printed_mana_cost && card.mana_cost_owned().is_none())
        || !permission.filter.matches(card, &game.filter_context_for(player, Some(source)), game)
    { return Vec::new(); }
    let view = crate::derived_view::DerivedGameView::new(game);
    let normal = CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: None };
    let mut methods = Vec::new();
    if crate::decision::completed_effect_driven_cast_proposal_is_legal(game, player, card, &normal, Zone::Exile)
        // "cast it face up any time you have priority": the permission
        // waives ordinary spell timing, not its costs.
        && crate::decision::can_cast_with_cost_with_view_for_casting_method_any_time(
            game, player, card, card_id, card.mana_cost_owned().as_ref(),
            &crate::decision::AdditionalCastRequirements::default(), &normal, &view,
        )
    { methods.push((format!("Cast {}", card.name), normal)); }
    for (index, alternative) in card.alternative_casts.iter().enumerate() {
        // Zone-specific alternatives retain their ordinary origin restrictions.
        if alternative.requires_cast_from_hand() || alternative.cast_from_zone() == Zone::Graveyard { continue; }
        let method = CastingMethod::PlayFrom { source, zone: Zone::Exile, use_alternative: Some(index) };
        if crate::decision::completed_effect_driven_cast_proposal_is_legal(game, player, card, &method, Zone::Exile)
            && crate::decision::can_cast_with_alternative_with_view_any_time(game, player, card, alternative, &view)
        { methods.push((format!("Cast {} for its {} cost", card.name, alternative.name()), method)); }
    }
    methods
}

pub(super) fn can_exile_cast(
    game: &GameState, player: PlayerId, source: ObjectId,
    permission: &DescribedFaceDownCastPermission,
) -> Result<bool, ActionError> {
    if !game.is_face_down(source) || game.is_phased_out(source) { return Ok(false); }
    // A query changes only its private copy. Moving off the battlefield restores
    // printed characteristics without performing a turn-face-up action/event.
    let mut projected = game.clone();
    let Some(exiled) = projected.move_object(source, Zone::Exile, EventCause::from_special_action(Some(source), player))
    else { return Ok(false); };
    Ok(!cast_methods(&projected, player, source, exiled, permission).is_empty())
}

pub(super) fn perform_exile_cast(
    game: &mut GameState, player: PlayerId, source: ObjectId,
    permission: &DescribedFaceDownCastPermission, decision_maker: &mut impl DecisionMaker,
) -> Result<(), ActionError> {
    let mut ctx = ExecutionContext::new(source, player, decision_maker)
        .with_cause(EventCause::from_special_action(Some(source), player));
    let outputs = crate::effects::with_per_event_trigger_matching(game, true, |game| {
        crate::effects::composition::execute_transaction(
        game, &mut ctx,
        || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
        |game, ctx| {
            let exile = Effect::new(crate::effects::ExileEffect::with_spec(ChooseSpec::Source)).tag("immediate_exile_cast");
            let exile_outputs = crate::effects::execute_effect_with_outputs(game, &exile, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)));
            }
            let Some(exiled) = ctx.get_tagged("immediate_exile_cast")
                .map(|snapshot| snapshot.object_id)
                .filter(|id| game.object(*id).is_some_and(|card| card.zone == Zone::Exile))
            else { return Ok(exile_outputs); };
            let options = cast_methods(game, player, source, exiled, permission);
            let method = match options.len() {
                0 => None,
                1 => options.into_iter().next().map(|(_, method)| method),
                _ => crate::decisions::ask_choose_one(game, ctx.decision_maker, player, source, &options),
            };
            let Some(method) = method else {
                if ctx.decision_maker.awaiting_choice() { return Ok(exile_outputs); }
                return Err(crate::effects::ExecutionError::Impossible("The exiled card cannot be cast".into()));
            };
            let cast = crate::game_loop::cast_spell_from_resolving_effect_with_outputs(
                game, exiled, Zone::Exile, player, &method, false, None,
                ctx.provenance, &mut ctx.decision_maker,
            ).map_err(|error| match error {
                crate::game_loop::GameLoopError::ExecutionFailed(error) => error,
                error => crate::effects::ExecutionError::Impossible(error.to_string()),
            })?;
            let Some(cast) = cast else {
                if ctx.decision_maker.awaiting_choice() { return Ok(exile_outputs); }
                return Err(crate::effects::ExecutionError::Impossible("The casting proposal did not complete".into()));
            };
            let outputs = crate::effects::player::complete_native_cast_with_outputs(
                EffectOutcome::with_objects(vec![cast.new_id]), game, cast, player, Zone::Exile, ctx.provenance,
            )?;
            let primary = outputs.outcome.clone();
            Ok(crate::effects::CompletedEffectOutputs::from_children(
                vec![exile_outputs, outputs],
                |phases| EffectOutcome::aggregate_with_primary_result(primary, phases),
            ))
        },
        )
    }).map_err(|error| ActionError::ExecutionFailure { source, error })?;
    let mut events = outputs.outcome.events;
    crate::effects::retain_unmatched_outcome_events(game, &mut events);
    for event in events { game.queue_trigger_event(ctx.provenance, event); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::EffectExecutor;
    use crate::target::PlayerFilter;
    use crate::ids::CardId;
    fn enter(card_type: CardType, cost: bool) -> (GameState, PlayerId, ObjectId) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        game.turn.priority_player = Some(player);
        game.turn.phase = Phase::Combat;
        let mut builder = CardBuilder::new(CardId::new(), "Face-down cast probe").card_types(vec![card_type]);
        if cost { builder = builder.mana_cost(ManaCost::new()); }
        let card = game.create_object_from_card(&builder.build(), player, Zone::Hand);
        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Creature);
        let mut entry = crate::effects::ManifestObjectsEffect::new(ChooseSpec::SpecificObject(card), PlayerFilter::You);
        entry.descriptive = true;
        entry.no_turn_up_permission = true;
        entry.exile_cast_permission = Some(DescribedFaceDownCastPermission { filter, require_printed_mana_cost: true });
        entry.execute(&mut game, &mut ExecutionContext::new_default(card, player)).unwrap();
        let source = *game.battlefield.iter().next().unwrap();
        (game, player, source)
    }
    #[test]
    fn described_face_down_noncreature_casts_at_priority_for_ordinary_cost() {
        let (mut game, player, source) = enter(CardType::Artifact, true);
        let action = SpecialAction::PerformRepeatableManaPaymentAction { action_index: 0 };
        assert!(can_perform_check(&action, &game, player).is_ok());
        perform(action, &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        assert!(game.object(source).is_none(), "battlefield incarnation left");
        assert_eq!(game.stack.len(), 1, "cast spell uses the stack");
        let spell = game.object(game.stack[0].object_id).unwrap();
        assert_eq!(spell.card_types.as_slice(), &[CardType::Artifact]);
        assert!(!game.is_face_down(spell.id));
    }
    #[test]
    fn described_face_down_cast_rejects_creatures_missing_cost_and_wrong_priority() {
        for (card_type, has_cost) in [(CardType::Creature, true), (CardType::Artifact, false)] {
            let (game, player, source) = enter(card_type, has_cost);
            let action = SpecialAction::PerformRepeatableManaPaymentAction { action_index: 0 };
            assert!(can_perform_check(&action, &game, player).is_err());
            assert_eq!(game.object(source).unwrap().zone, Zone::Battlefield);
            assert!(game.is_face_down(source));
        }
        let (mut game, player, source) = enter(CardType::Artifact, true);
        game.turn.priority_player = Some(PlayerId::from_index(1));
        assert!(can_perform_check(&SpecialAction::PerformRepeatableManaPaymentAction { action_index: 0 }, &game, player).is_err());
        assert!(game.is_face_down(source));
    }
    #[test]
    fn described_face_down_cast_accepts_ordinary_alternative_cost_without_mana() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = PlayerId::from_index(0);
        game.turn.priority_player = Some(player);
        game.turn.phase = Phase::Combat;
        let definition = crate::cards::CardDefinitionBuilder::new(CardId::new(), "Alternative price probe")
            .card_types(vec![CardType::Artifact])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(5)]))
            .alternative_cast(crate::alternative_cast::AlternativeCastingMethod::alternative_cost(
                "Alternative price", Some(ManaCost::new()), vec![],
            )).build();
        let card = game.create_object_from_definition(&definition, player, Zone::Hand);
        let mut entry = crate::effects::ManifestObjectsEffect::new(ChooseSpec::SpecificObject(card), PlayerFilter::You);
        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Creature);
        entry.descriptive = true;
        entry.no_turn_up_permission = true;
        entry.exile_cast_permission = Some(DescribedFaceDownCastPermission { filter, require_printed_mana_cost: true });
        entry.execute(&mut game, &mut ExecutionContext::new_default(card, player)).unwrap();
        let action = SpecialAction::PerformRepeatableManaPaymentAction { action_index: 0 };
        assert!(can_perform_check(&action, &game, player).is_ok(), "alternative price is affordable");
        perform(action, &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.stack.len(), 1);
    }

    #[test]
    fn described_face_down_permission_follows_control_and_ends_on_face_up() {
        let (mut game, alice, source) = enter(CardType::Artifact, true);
        let bob = PlayerId::from_index(1);
        game.set_current_controller(source, bob).unwrap();
        game.turn.priority_player = Some(bob);
        let action = SpecialAction::PerformRepeatableManaPaymentAction { action_index: 0 };
        assert!(can_perform_check(&action, &game, bob).is_ok());
        assert!(can_perform_check(&action, &game, alice).is_err());
        assert!(game.set_face_up(source).unwrap());
        assert!(game.set_face_down(source));
        assert!(can_perform_check(&action, &game, bob).is_err(), "a later face-down state does not renew the old entry");
    }

}

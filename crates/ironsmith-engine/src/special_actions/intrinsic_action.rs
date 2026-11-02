//! Independent static permissions for immediate paid actions.
use super::*;
use crate::effects::CompletedEffectOutputs;
use ironsmith_core::StaticAbilityPayload;
use ironsmith_core::static_ability_model::ImmediateActionTiming;

pub(super) struct IntrinsicActionSpec {
    pub cost: crate::cost::TotalCost,
    effects: Vec<crate::effect::Effect>,
    once: bool,
    shared_turn_group: Option<String>,
    occurrence: (ObjectId, Option<crate::ids::CardId>, crate::continuous::AbilityOrigin),
}

pub fn intrinsic_action_label(game: &GameState, source: ObjectId, index: usize) -> Option<String> {
    let ability = game.current_ability(source, index)?;
    let crate::ability::AbilityKind::Static(rule) = &ability.kind else { return None; };
    let model = rule.compiled_model()?;
    matches!(&model.payload, StaticAbilityPayload::IntrinsicImmediateAction { .. })
        .then(|| if model.label == "Paid special action" { "Take special action".to_owned() } else { model.label.clone() })
}

pub fn intrinsic_action_cost_display(game: &GameState, source: ObjectId, index: usize) -> Option<String> {
    let ability = game.current_ability(source, index)?;
    let crate::ability::AbilityKind::Static(rule) = &ability.kind else { return None; };
    match rule.compiled_model().map(|model| &model.payload)? {
        StaticAbilityPayload::IntrinsicImmediateAction { cost, .. } => Some(cost.display()),
        _ => None,
    }
}

pub(super) fn validate(game: &GameState, player: PlayerId, source: ObjectId, index: usize) -> Result<IntrinsicActionSpec, ActionError> {
    if !game.team_has_priority(player) { return Err(ActionError::NotYourPriority); }
    let object = game.object(source).ok_or(ActionError::ObjectNotFound)?;
    let ability = game.current_ability(source, index).ok_or(ActionError::NoSuchAbility)?;
    if !ability.functions_in(&object.zone) || game.is_phased_out(source) {
        return Err(ActionError::InvalidTiming);
    }
    let crate::ability::AbilityKind::Static(rule) = &ability.kind else { return Err(ActionError::NoSuchAbility); };
    let Some(StaticAbilityPayload::IntrinsicImmediateAction { cost, timing, source_zone, player: actor, condition, shared_turn_group, effects, once }) = rule.compiled_model().map(|model| &model.payload)
    else { return Err(ActionError::NoSuchAbility); };
    let controller = game.controller_of(object);
    if object.zone != *source_zone || !crate::filter::player_filter_matches_game(
        actor, player, game, &game.filter_context_for(controller, Some(source)),
    ) {
        return Err(ActionError::InvalidTiming);
    }
    if let Some(condition) = condition {
        if !super::check_mana_ability_condition(game, controller, source, index, condition) {
            return Err(ActionError::InvalidTiming);
        }
    }
    if let Some(group) = shared_turn_group {
        let key = crate::action_use_receipts::ActionUseKey { source, ability: None, definition: None, group: group.clone() };
        if crate::action_use_receipts::used_this_turn(game, &key) > 0 { return Err(ActionError::InvalidTiming); }
    }
    match timing {
        ImmediateActionTiming::Priority => {},
        ImmediateActionTiming::Sorcery => has_sorcery_speed_special_action_timing(game, player)?,
        ImmediateActionTiming::Attacking => {
            if !game.combat.as_ref().is_some_and(|combat| crate::combat_state::is_attacking(combat, source)) {
                return Err(ActionError::InvalidTiming);
            }
        }
    }
    let origin = game.current_characteristics(source).and_then(|characteristics|
        characteristics.abilities.origin(index).cloned()).ok_or(ActionError::NoSuchAbility)?;
    let occurrence = (source, object.card, origin);
    if *once && game.effect_store.intrinsic_immediate_actions_used.contains(&occurrence) {
        return Err(ActionError::InvalidTiming);
    }
    Ok(IntrinsicActionSpec { cost: cost.clone(), effects: effects.clone(), once: *once, shared_turn_group: shared_turn_group.clone(), occurrence })
}

pub(super) fn perform_action(
    game: &mut GameState, player: PlayerId, source: ObjectId, index: usize,
    decision_maker: &mut impl DecisionMaker,
) -> Result<(), ActionError> {
    let spec = validate(game, player, source, index)?;
    perform_prepared_action(game, player, source, spec, None, decision_maker)
}

pub(super) fn perform_prepared_action(
    game: &mut GameState, player: PlayerId, source: ObjectId, spec: IntrinsicActionSpec,
    x_value: Option<u32>, decision_maker: &mut impl DecisionMaker,
) -> Result<(), ActionError> {
    let mut ctx = ExecutionContext::new(source, player, decision_maker)
        .with_cause(EventCause::from_special_action(Some(source), player));
    ctx.x_value = x_value;
    let outputs = crate::effects::with_per_event_trigger_matching(game, true, |game| {
        crate::effects::composition::execute_transaction(game, &mut ctx,
            || CompletedEffectOutputs::aggregate_only(crate::effect::EffectOutcome::count(0)),
            |game, ctx| {
                if spec.once {
                    game.effect_store.intrinsic_immediate_actions_used.insert(spec.occurrence.clone());
                    game.mark_continuous_state_dirty();
                }
                if let Some(group) = &spec.shared_turn_group {
                    crate::action_use_receipts::consume_this_turn(game,
                        &crate::action_use_receipts::ActionUseKey { source, ability: None, definition: None, group: group.clone() }, 1)
                        .map_err(crate::effects::ExecutionError::Impossible)?;
                }
                let mut children = Vec::new();
                let mut reported = Vec::new();
                for (index, effect) in spec.effects.iter().enumerate() {
                    let outputs = crate::effects::execute_effect_with_outputs(game, effect, ctx)?;
                    reported.extend(outputs.outcome.events.iter().cloned());
                    children.push(outputs);
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(CompletedEffectOutputs::aggregate_only(crate::effect::EffectOutcome::count(0)));
                    }
                    if crate::effects::match_triggers_at_instruction_boundary(
                        game, ctx, spec.effects.get(index + 1), reported.iter(),
                    )? { reported.clear(); }
                }
                Ok(CompletedEffectOutputs::from_children(children, |outcomes|
                    crate::effect::EffectOutcome::aggregate_with_primary_result(crate::effect::EffectOutcome::resolved(), outcomes)))
            })
    }).map_err(|error| ActionError::ExecutionFailure { source, error })?;
    let mut events = outputs.outcome.events;
    crate::effects::retain_unmatched_outcome_events(game, &mut events);
    for event in events { game.queue_trigger_event(ctx.provenance, event); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ApplyContinuousEffect, EffectExecutor};
    use crate::effect::{Effect, Until, Value};
    use crate::continuous::{Modification, PtSublayer};
    use crate::types::{CardType, Supertype};
    use crate::target::{ChooseSpec, PlayerFilter};
    fn model(timing: ImmediateActionTiming, once: bool, life: bool) -> crate::static_abilities::StaticAbility {
        let mut components = vec![crate::costs::Cost::mana(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))];
        if life { components.push(crate::costs::Cost::life(4)); }
        let effects = vec![Effect::new(ApplyContinuousEffect::with_spec(ChooseSpec::Source,
            Modification::SetPowerToughness { power: Value::Fixed(4), toughness: Value::Fixed(4), sublayer: PtSublayer::Setting }, Until::Forever)),
            Effect::new(ApplyContinuousEffect::with_spec(ChooseSpec::Source, Modification::AddSupertypes(vec![Supertype::Legendary]), Until::Forever))];
        crate::static_abilities::StaticAbility::from_model(ironsmith_core::StaticAbility {
            id: Some(ironsmith_core::StaticAbilityId::IntrinsicImmediateAction), label: "Paid characteristic action".into(),
            payload: StaticAbilityPayload::IntrinsicImmediateAction {
                cost: crate::cost::TotalCost::from_costs(components), timing, source_zone: Zone::Battlefield,
                player: PlayerFilter::You, condition: None, shared_turn_group: None, effects, once,
            },
        })
    }
    fn setup(timing: ImmediateActionTiming, once: bool, life: bool) -> (GameState, ObjectId, PlayerId) {
        let player = PlayerId::from_index(0);
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        game.turn.priority_player = Some(player); game.turn.phase = Phase::FirstMain;
        let definition = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Intrinsic paid action audit")
            .card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1, 3))
            .with_ability(crate::ability::Ability::static_ability(crate::static_abilities::StaticAbility::haste()))
            .with_ability(crate::ability::Ability::static_ability(model(timing, once, life))).build();
        let source = game.create_object_from_definition(&definition, player, Zone::Battlefield);
        (game, source, player)
    }
    fn action_for(game: &GameState, source: ObjectId) -> SpecialAction {
        let index = game.current_characteristics(source).unwrap().abilities.iter().position(|ability|
            matches!(&ability.kind, crate::ability::AbilityKind::Static(rule)
                if matches!(rule.compiled_model().map(|model| &model.payload), Some(StaticAbilityPayload::IntrinsicImmediateAction { .. })))).unwrap();
        SpecialAction::PerformIntrinsicAction { source_id: source, ability_index: index }
    }
    fn attack(game: &mut GameState, source: ObjectId, defender: PlayerId) {
        let mut combat = crate::combat_state::CombatState::default();
        combat.attackers.push(crate::combat_state::AttackerInfo { creature: source, target: crate::combat_state::AttackTarget::Player(defender) });
        game.combat = Some(combat);
    }
    #[test]
    fn intrinsic_paid_action_is_immediate_preserves_counters_and_is_not_an_activation() {
        let (mut game, source, player) = setup(ImmediateActionTiming::Sorcery, false, false);
        game.add_counters(source, crate::CounterType::PlusOnePlusOne, 2);
        game.effect_store.cant_effects.cant_activate_abilities_of.insert(source);
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
        let action = action_for(&game, source);
        assert!(can_perform_check(&action, &game, player).is_ok());
        perform(action.clone(), &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        assert!(game.stack.is_empty(), "there is no activated ability to respond to");
        assert_eq!((game.calculated_power(source), game.calculated_toughness(source)), (Some(6), Some(6)));
        perform(action, &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.calculated_power(source), Some(6), "setting base characteristics is not an additive bonus");
    }
    #[test]
    fn intrinsic_paid_action_timing_and_full_life_cost_are_enforced() {
        let (mut game, source, player) = setup(ImmediateActionTiming::Sorcery, false, true);
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        let action = action_for(&game, source);
        game.turn.phase = Phase::Combat;
        assert!(can_perform_check(&action, &game, player).is_err());
        game.turn.phase = Phase::FirstMain;
        game.player_mut(player).unwrap().life = 3;
        assert!(can_perform_check(&action, &game, player).is_err());
        game.player_mut(player).unwrap().life = 20;
        perform(action, &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(player).unwrap().life, 16);
        let (mut instant_game, instant_source, player) = setup(ImmediateActionTiming::Priority, false, false);
        instant_game.turn.phase = Phase::Combat;
        instant_game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        assert!(can_perform_check(&action_for(&instant_game, instant_source), &instant_game, player).is_ok());
    }
    #[test]
    fn intrinsic_once_receipt_survives_turn_control_and_ability_index_changes_but_not_new_incarnation() {
        let (mut game, source, player) = setup(ImmediateActionTiming::Attacking, true, false);
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 2);
        let first = action_for(&game, source);
        assert!(can_perform_check(&first, &game, player).is_err(), "not attacking");
        game.turn.phase = Phase::Combat;
        attack(&mut game, source, PlayerId::from_index(1));
        perform(first, &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(source), Modification::RemoveAbility(crate::static_abilities::StaticAbility::haste()), Until::Forever)
            .execute(&mut game, &mut ExecutionContext::new_default(source, player)).unwrap();
        assert!(matches!(action_for(&game, source), SpecialAction::PerformIntrinsicAction { ability_index: 0, .. }));
        game.turn.turn_number += 1;
        assert!(can_perform_check(&action_for(&game, source), &game, player).is_err());
        let other = PlayerId::from_index(1);
        game.set_current_controller(source, other).unwrap(); game.turn.active_player = other; game.turn.priority_player = Some(other);
        game.player_mut(other).unwrap().mana_pool.add(ManaSymbol::Blue, 1); attack(&mut game, source, player);
        assert!(can_perform_check(&action_for(&game, source), &game, other).is_err(), "controller changes do not renew the action");
        let hand = game.move_object_by_effect(source, Zone::Hand).unwrap();
        let returned = game.move_object_with_etb_processing(hand, Zone::Battlefield).unwrap().assert_completed_without_additions().unwrap().new_id;
        game.turn.active_player = player; game.turn.priority_player = Some(player); attack(&mut game, returned, other);
        assert!(can_perform_check(&action_for(&game, returned), &game, player).is_ok(), "new zone incarnation has its own action");
    }
    #[test]
    fn losing_all_abilities_removes_the_static_action_permission() {
        let (mut game, source, player) = setup(ImmediateActionTiming::Priority, false, false);
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        let action = action_for(&game, source);
        ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(source), Modification::RemoveAllAbilities, Until::Forever)
            .execute(&mut game, &mut ExecutionContext::new_default(source, player)).unwrap();
        assert!(can_perform_check(&action, &game, player).is_err());
    }
    #[test]
    fn intrinsic_paid_action_retains_its_announced_occurrence_when_mana_payment_changes_indices() {
        let (mut game, source, player) = setup(ImmediateActionTiming::Sorcery, true, false);
        let remove_haste = Effect::new(ApplyContinuousEffect::with_spec(ChooseSpec::SpecificObject(source),
            Modification::RemoveAbility(crate::static_abilities::StaticAbility::haste()), Until::Forever));
        let mana_ability = crate::ability::Ability::activated(
            crate::cost::TotalCost::from_cost(crate::costs::Cost::tap()),
            vec![remove_haste, Effect::add_mana(vec![ManaSymbol::Blue])],
        );
        let land = crate::cards::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Announcement-changing mana source")
            .card_types(vec![CardType::Land]).with_ability(mana_ability).build();
        game.create_object_from_definition(&land, player, Zone::Battlefield);
        let announced = action_for(&game, source);
        assert!(matches!(announced, SpecialAction::PerformIntrinsicAction { ability_index: 1, .. }));
        assert!(can_perform_check(&announced, &game, player).is_ok());
        perform(announced, &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        assert!(matches!(action_for(&game, source), SpecialAction::PerformIntrinsicAction { ability_index: 0, .. }));
        assert_eq!(game.calculated_power(source), Some(4));
        assert!(game.stack.is_empty());
    }

    #[test]
    fn intrinsic_paid_action_choice_pause_rolls_back_payment_and_once_receipt_before_replay() {
        struct Pause { pending: bool }
        impl crate::decision::DecisionMaker for Pause {
            fn decide_boolean(&mut self, _: &GameState, _: &crate::decisions::context::BooleanContext) -> bool { self.pending = true; false }
            fn awaiting_choice(&self) -> bool { self.pending }
        }
        let (mut game, source, player) = setup(ImmediateActionTiming::Sorcery, true, true);
        let mut compiled = model(ImmediateActionTiming::Sorcery, true, true).compiled_model().unwrap().clone();
        if let StaticAbilityPayload::IntrinsicImmediateAction { effects, .. } = &mut compiled.payload {
            *effects = vec![Effect::may(vec![Effect::gain_life(1)])];
        }
        game.object_mut(source).unwrap().abilities_mut()[1] = crate::ability::Ability::static_ability(
            crate::static_abilities::StaticAbility::from_model(compiled));
        game.player_mut(player).unwrap().mana_pool.add(ManaSymbol::Blue, 1);
        let announced = action_for(&game, source);
        let mut dm = Pause { pending: false };
        perform(announced.clone(), &mut game, player, &mut dm).unwrap();
        assert!(dm.pending);
        assert_eq!(game.player(player).unwrap().life, 20);
        assert_eq!(game.player(player).unwrap().mana_pool.total(), 1);
        assert!(game.effect_store.intrinsic_immediate_actions_used.is_empty());
        perform(announced, &mut game, player, &mut crate::decision::SelectFirstDecisionMaker).unwrap();
        assert_eq!(game.player(player).unwrap().life, 17);
        assert_eq!(game.player(player).unwrap().mana_pool.total(), 0);
        assert_eq!(game.effect_store.intrinsic_immediate_actions_used.len(), 1);
    }

}

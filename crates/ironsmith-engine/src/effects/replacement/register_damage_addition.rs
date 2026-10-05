//! A resolved additive damage replacement captures its quantity once.
use crate::effect::EffectOutcome;
use crate::effects::{ApplyReplacementEffect, EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::static_abilities::StaticAbilityKind;
pub type RegisterDamageAdditionEffect = ironsmith_core::RegisterDamageAdditionEffect;

impl EffectExecutor for RegisterDamageAdditionEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let delta = crate::effects::helpers::resolve_value(game, &self.delta, ctx)?;
        // A legal announced X=0 is a completed instruction, not an unsupported
        // path. It requires no persistent zero-change replacement occurrence.
        if delta == 0 {
            return Ok(EffectOutcome::resolved());
        }
        let ability = crate::static_abilities::ModifyDamageAmountReplacement::new(
            self.source_filter.clone(),
            self.target_player_filter.clone(),
            self.target_object_filter.clone(),
            delta,
            "Resolved additive damage replacement",
        )
        .with_noncombat_only(self.noncombat_only);
        let effect = ability
            .generate_replacement_effect(ctx.source, ctx.controller)
            .expect("nonzero additive modifier has an occurrence");
        ApplyReplacementEffect {
            effect,
            mode: self.mode,
        }
        .execute(game, ctx)
    }
    fn primary_execution_category(&self) -> crate::effects::EffectExecutionCategory {
        crate::effects::EffectExecutionCategory::ReplacementRegistration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::zone::Zone;
    const A: PlayerId = PlayerId(0);
    const B: PlayerId = PlayerId(1);
    fn object(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let card = crate::card::CardBuilder::new(CardId::new(), name)
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }
    fn registration(delta: Value) -> RegisterDamageAdditionEffect {
        RegisterDamageAdditionEffect {
            source_filter: ObjectFilter::default().controlled_by(PlayerFilter::You),
            target_player_filter: Some(PlayerFilter::Any),
            target_object_filter: Some(ObjectFilter::permanent()),
            delta,
            noncombat_only: true,
            mode: crate::effects::ReplacementApplyMode::UntilEndOfTurn,
        }
    }
    #[test]
    fn announced_x_and_controller_are_captured_until_cleanup_despite_source_departure() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 30);
        let granting = object(&mut game, A, "Granter");
        let own = object(&mut game, A, "Own source");
        let other = object(&mut game, B, "Other source");
        registration(Value::X)
            .execute(
                &mut game,
                &mut ExecutionContext::new_default(granting, A).with_x(3),
            )
            .unwrap();
        game.move_object_by_effect(granting, Zone::Graveyard)
            .unwrap();
        let noncombat = crate::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B));
        noncombat
            .execute(
                &mut game,
                &mut ExecutionContext::new_default(own, A).with_x(9),
            )
            .unwrap();
        assert_eq!(game.player(B).unwrap().life, 25);
        crate::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(A))
            .execute(&mut game, &mut ExecutionContext::new_default(other, B))
            .unwrap();
        assert_eq!(game.player(A).unwrap().life, 28);
        crate::effects::DealDamageEffect::new(2, ChooseSpec::SpecificPlayer(B))
            .with_combat(true)
            .execute(&mut game, &mut ExecutionContext::new_default(own, A))
            .unwrap();
        assert_eq!(game.player(B).unwrap().life, 23);
        crate::turn::execute_cleanup_step(&mut game);
        noncombat
            .execute(&mut game, &mut ExecutionContext::new_default(own, A))
            .unwrap();
        assert_eq!(game.player(B).unwrap().life, 21);
    }
    #[test]
    fn legal_zero_and_missing_amount_are_distinct_and_leave_no_replacement() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = object(&mut game, A, "Source");
        let before = game.effect_store.replacement_effects.effects().len();
        registration(Value::X)
            .execute(
                &mut game,
                &mut ExecutionContext::new_default(source, A).with_x(0),
            )
            .unwrap();
        assert_eq!(
            game.effect_store.replacement_effects.effects().len(),
            before
        );
        assert!(
            registration(Value::X)
                .execute(&mut game, &mut ExecutionContext::new_default(source, A))
                .is_err()
        );
        assert_eq!(
            game.effect_store.replacement_effects.effects().len(),
            before
        );
    }
    #[test]
    fn replacement_expansion_enters_the_existing_checked_damage_rollback_owner() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = object(&mut game, A, "Source");
        registration(Value::Fixed(i32::MAX))
            .execute(&mut game, &mut ExecutionContext::new_default(source, A))
            .unwrap();
        let result = crate::effects::execute_effect(
            &mut game,
            &Effect::deal_damage(
                Value::Add(
                    Box::new(Value::Fixed(i32::MAX)),
                    Box::new(Value::Fixed(i32::MAX)),
                ),
                ChooseSpec::SpecificPlayer(B),
            ),
            &mut ExecutionContext::new_default(source, A),
        );
        assert!(result.is_err());
        assert_eq!(game.player(B).unwrap().life, 20);
    }
    #[test]
    fn live_counter_bonus_uses_registration_source_and_changes_between_damage_events() {
        use crate::object::CounterType;
        let mut game = GameState::new(vec!["A".into(), "B".into()], 30);
        let granter = object(&mut game, A, "Counter source");
        let dealer = object(&mut game, A, "Different damage source");
        game.object_mut(granter)
            .unwrap()
            .counters
            .insert(CounterType::Named("fire".into()), 2);
        game.object_mut(dealer)
            .unwrap()
            .counters
            .insert(CounterType::Named("fire".into()), 9);
        let live = crate::static_abilities::ModifyDamageAmountReplacement::new(
            ObjectFilter::default().controlled_by(PlayerFilter::You),
            Some(PlayerFilter::Opponent),
            None,
            0,
            "Live source counter bonus",
        )
        .with_dynamic_delta(Value::CountersOnSource(CounterType::Named("fire".into())));
        let occurrence = live.generate_replacement_effect(granter, A).unwrap();
        game.effect_store
            .replacement_effects
            .add_resolution_effect(occurrence);
        let damage = Effect::deal_damage(1, ChooseSpec::SpecificPlayer(B));
        crate::effects::execute_effect(
            &mut game,
            &damage,
            &mut ExecutionContext::new_default(dealer, A),
        )
        .unwrap();
        assert_eq!(game.player(B).unwrap().life, 27);
        game.object_mut(granter)
            .unwrap()
            .counters
            .insert(CounterType::Named("fire".into()), 4);
        crate::effects::execute_effect(
            &mut game,
            &damage,
            &mut ExecutionContext::new_default(dealer, A),
        )
        .unwrap();
        assert_eq!(game.player(B).unwrap().life, 22);
        game.object_mut(granter)
            .unwrap()
            .counters
            .insert(CounterType::Named("fire".into()), u32::MAX);
        let result = crate::effects::execute_effect(
            &mut game,
            &damage,
            &mut ExecutionContext::new_default(dealer, A),
        );
        assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_))));
        assert_eq!(game.player(B).unwrap().life, 22);
    }

    #[test]
    fn live_power_bonus_and_negative_values_are_not_read_from_the_damage_source() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 30);
        let granter = object(&mut game, A, "Power source");
        let dealer = object(&mut game, A, "Dealer");
        let live = crate::static_abilities::ModifyDamageAmountReplacement::new(
            ObjectFilter::default().controlled_by(PlayerFilter::You),
            Some(PlayerFilter::Opponent),
            None,
            0,
            "Live source power bonus",
        )
        .with_dynamic_delta(Value::SourcePower);
        game.effect_store
            .replacement_effects
            .add_resolution_effect(live.generate_replacement_effect(granter, A).unwrap());
        crate::effects::execute_effect(
            &mut game,
            &Effect::deal_damage(1, ChooseSpec::SpecificPlayer(B)),
            &mut ExecutionContext::new_default(dealer, A),
        )
        .unwrap();
        assert_eq!(game.player(B).unwrap().life, 27);
        let negative = crate::static_abilities::ModifyDamageAmountReplacement::new(
            ObjectFilter::default(),
            Some(PlayerFilter::Any),
            None,
            0,
            "Negative live bonus",
        )
        .with_dynamic_delta(Value::Fixed(-20));
        game.effect_store
            .replacement_effects
            .add_resolution_effect(negative.generate_replacement_effect(granter, A).unwrap());
        crate::effects::execute_effect(
            &mut game,
            &Effect::deal_damage(1, ChooseSpec::SpecificPlayer(B)),
            &mut ExecutionContext::new_default(dealer, A),
        )
        .unwrap();
        assert_eq!(game.player(B).unwrap().life, 27);
    }

    #[test]
    fn oversized_announced_x_and_surface_counter_reads_fail_without_registration() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = object(&mut game, A, "Source");
        let before = game.effect_store.replacement_effects.effects().len();
        assert!(matches!(
            registration(Value::X).execute(
                &mut game,
                &mut ExecutionContext::new_default(source, A).with_x(u32::MAX)
            ),
            Err(ExecutionError::UnresolvableValue(_))
        ));
        assert_eq!(
            game.effect_store.replacement_effects.effects().len(),
            before
        );
        game.object_mut(source)
            .unwrap()
            .counters
            .insert(crate::object::CounterType::Named("fire".into()), u32::MAX);
        let value = Value::CountersOn(
            Box::new(ChooseSpec::Source),
            Some(crate::object::CounterType::Named("fire".into())),
        );
        assert!(matches!(
            crate::effects::helpers::resolve_value(
                &game,
                &value,
                &ExecutionContext::new_default(source, A)
            ),
            Err(ExecutionError::UnresolvableValue(_))
        ));
    }

    #[test]
    fn oversized_player_marker_event_is_not_a_wrapped_damage_amount() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let source = object(&mut game, A, "Source");
        let event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::MarkersChangedEvent::added(
                crate::object::CounterType::Energy,
                A,
                u32::MAX,
                None,
                None,
            ),
            Default::default(),
        );
        let ctx = ExecutionContext::new_default(source, A).with_triggering_event(event);
        assert_eq!(
            crate::effects::helpers::resolve_value_wide(
                &game,
                &Value::EventValue(crate::effect::EventValueSpec::Amount),
                &ctx
            )
            .unwrap(),
            i64::from(u32::MAX)
        );
        assert!(matches!(
            crate::effects::helpers::resolve_value(
                &game,
                &Value::EventValue(crate::effect::EventValueSpec::Amount),
                &ctx
            ),
            Err(ExecutionError::ResourceLimitExceeded { requested, maximum, .. }) if requested == u128::from(u32::MAX) && maximum == i32::MAX as u128
        ));
    }
}

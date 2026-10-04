//! Checked monarch designation and its completed event boundary.
#[cfg(test)]
use crate::target::PlayerFilter;
use super::*;
use crate::effects::ExecutionError;
use crate::events::MonarchChangedEvent;
use crate::triggers::{TriggerEvent, TriggerQueue};
impl GameState {
    /// Change the designation once. Sources that return when this duration
    /// ends are processed by their existing decision-aware owner afterward.
    pub fn set_monarch(&mut self, monarch: Option<PlayerId>) -> Result<bool, ExecutionError> {
        let checkpoint = self.clone();
        let result = (|| {
            self.refresh_continuous_state()
                .map_err(ExecutionError::ContinuousDiscovery)?;
            if monarch.is_some_and(|player| {
                !self.player(player).is_some_and(|p| p.is_in_game())
                    || !self.can_become_monarch(player)
            }) {
                return Ok(false);
            }
            let previous = self.monarch;
            if previous == monarch {
                return Ok(false);
            }
            self.set_monarch_designation_unpublished(monarch);
            self.publish_monarch_change(previous)?;
            Ok(true)
        })();
        if result.is_err() {
            self.restore_execution_checkpoint(checkpoint, false);
        }
        result
    }
    pub(super) fn set_monarch_designation_unpublished(&mut self, monarch: Option<PlayerId>) {
        if self.monarch != monarch {
            self.monarch = monarch;
            self.mark_continuous_state_dirty();
            if monarch.is_some() {
                self.record_ui_effect_event("monarch", monarch, None, Vec::new(), None, None);
            }
        }
    }
    /// Called after every original departure in a simultaneous loss/draw.
    /// Publish before Palace Jailer's later duration-end return instruction.
    pub(super) fn publish_monarch_change(
        &mut self,
        previous: Option<PlayerId>,
    ) -> Result<(), ExecutionError> {
        self.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        if previous == self.monarch {
            return Ok(());
        }
        let Some(monarch) = self.monarch else {
            return Ok(());
        };
        let id = self
            .provenance_graph_mut()
            .alloc_root_event(crate::events::EventKind::MonarchChanged);
        let mut event =
            TriggerEvent::new_with_provenance(MonarchChangedEvent { previous, monarch }, id);
        self.stage_turn_history_event(&event);
        self.refresh_continuous_state()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let mut captured = TriggerQueue::new();
        crate::game_loop::queue_triggers_from_reported_events(
            self,
            &mut captured,
            vec![event.clone()],
            true,
        );
        self.defer_trigger_entries(captured.take_all());
        event.mark_triggers_captured();
        self.queue_trigger_event(id, event);
        self.return_exiled_for_opponent_becoming_monarch(monarch);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::cards::CardDefinitionBuilder;
    use crate::effect::Effect;
    use crate::triggers::Trigger;
    const A: PlayerId = PlayerId(0);
    const B: PlayerId = PlayerId(1);
    const C: PlayerId = PlayerId(2);
    fn game() -> GameState {
        GameState::new(vec!["A".into(), "B".into(), "C".into(), "D".into()], 30)
    }
    fn observer(game: &mut GameState, owner: PlayerId) -> ObjectId {
        let card = CardDefinitionBuilder::new(crate::CardId::new(), "Monarch observer")
            .card_types(vec![crate::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .with_ability(Ability::triggered(
                Trigger::player_becomes_monarch(PlayerFilter::Any),
                vec![Effect::gain_life(1)],
            ))
            .build();
        game.create_object_from_definition(&card, owner, Zone::Battlefield)
    }
    #[test]
    fn real_change_has_one_completed_notice_and_unchanged_clear_invalid_have_none() {
        let mut game = game();
        observer(&mut game, C);
        assert!(game.set_monarch(Some(A)).unwrap());
        assert_eq!(game.take_pending_trigger_entries().len(), 1);
        assert!(!game.set_monarch(Some(A)).unwrap());
        assert!(game.take_pending_trigger_entries().is_empty());
        assert!(!game.set_monarch(Some(PlayerId(9))).unwrap());
        assert_eq!(game.monarch, Some(A));
        assert!(game.set_monarch(None).unwrap());
        assert!(game.take_pending_trigger_entries().is_empty());
        assert!(game.set_monarch(Some(B)).unwrap());
        let entries = game.take_pending_trigger_entries();
        assert_eq!(entries.len(), 1);
        let event = entries[0]
            .triggering_event
            .downcast::<MonarchChangedEvent>()
            .unwrap();
        assert_eq!(event.monarch, B);
        assert_eq!(event.previous, None);
        assert!(
            !entries[0]
                .triggering_event
                .inner()
                .is_replacement_proposal()
        );
    }
    #[test]
    fn simultaneous_departures_choose_only_survivors_and_exclude_departed_observers() {
        let mut game = game();
        game.turn.active_player = B;
        game.set_monarch(Some(A)).unwrap();
        let survivor = observer(&mut game, C);
        observer(&mut game, B);
        let borrowed = observer(&mut game, A);
        game.set_current_controller(borrowed, C);
        assert_eq!(
            game.mark_players_lost_simultaneously(&[A, B]).unwrap(),
            vec![A, B]
        );
        assert_eq!(game.monarch, Some(C));
        let entries = game.take_pending_trigger_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, survivor);
        let change = entries[0]
            .triggering_event
            .downcast::<MonarchChangedEvent>()
            .unwrap();
        assert_eq!(change.previous, Some(A));
        assert_eq!(change.monarch, C);
    }
    #[test]
    fn later_duration_return_cannot_create_a_retroactive_monarch_observer() {
        let mut game = game();
        game.set_monarch(Some(A)).unwrap();
        let original = observer(&mut game, C);
        let returning = observer(&mut game, C);
        let stable = game.object(returning).unwrap().stable_id;
        game.move_object(returning, Zone::Exile, crate::events::EventCause::effect());
        game.track_exiled_until_opponent_becomes_monarch(A, vec![stable], Zone::Battlefield);
        game.set_monarch(Some(B)).unwrap();
        let entries = game.take_pending_trigger_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, original);
        assert!(game.has_pending_duration_end_returns());
        game.process_pending_duration_end_returns(&mut crate::decision::SelectFirstDecisionMaker)
            .unwrap();
        assert!(game.battlefield.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.stable_id == stable)
        }));
        let mut queue = TriggerQueue::new();
        crate::game_loop::drain_pending_trigger_events(&mut game, &mut queue);
        assert!(queue.is_empty());
    }
    #[test]
    fn turn_begin_holder_is_frozen_across_later_changes_and_extra_untap_boundaries() {
        let mut game = game();
        game.set_monarch(Some(A)).unwrap();
        game.establish_turn_start_continuous_control();
        assert_eq!(game.turn_store.turn_history.monarch_at_turn_start, Some(A));
        game.set_monarch(Some(B)).unwrap();
        game.establish_turn_start_continuous_control();
        assert_eq!(game.turn_store.turn_history.monarch_at_turn_start, Some(A));
        let restored = game.clone();
        assert_eq!(
            restored.turn_store.turn_history.monarch_at_turn_start,
            Some(A)
        );
        game.next_turn();
        assert_eq!(game.turn_store.turn_history.monarch_at_turn_start, Some(B));
    }
    #[derive(Debug, Clone)]
    struct RegrantSelf;
    impl crate::static_abilities::StaticAbilityKind for RegrantSelf {
        fn id(&self) -> crate::static_abilities::StaticAbilityId {
            crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
        }
        fn display(&self) -> String {
            "Unbounded fixture grant".into()
        }
        fn generate_effects(
            &self,
            source: ObjectId,
            controller: PlayerId,
            game: &GameState,
        ) -> Vec<crate::continuous::ContinuousEffect> {
            let crate::ability::AbilityKind::Static(parent) =
                &game.object(source).unwrap().abilities[0].kind
            else {
                panic!("fixture parent missing")
            };
            vec![crate::continuous::ContinuousEffect::new(
                source,
                controller,
                crate::continuous::EffectTarget::Source,
                crate::continuous::Modification::AddAbility(parent.clone()),
            )]
        }
    }
    fn unbounded_discovery_source(game: &mut GameState) -> ObjectId {
        let card = CardDefinitionBuilder::new(crate::CardId::new(), "Unbounded monarch fixture")
            .card_types(vec![crate::CardType::Artifact])
            .with_ability(Ability::static_ability(
                crate::static_abilities::StaticAbility::new(RegrantSelf),
            ))
            .build();
        game.create_object_from_definition(&card, B, Zone::Battlefield)
    }
    #[test]
    fn mandatory_loop_draw_preserves_exact_range_controllers_on_incomplete_departure_and_retry() {
        let mut game = game();
        game.enable_limited_range_of_influence(vec![A, B, C, PlayerId(3)], vec![0; 4])
            .unwrap();
        game.mark_mandatory_loop_draw_for([A, C]);
        let source = unbounded_discovery_source(&mut game);
        assert!(matches!(
            game.resolve_mandatory_loop_draw(),
            Err(ExecutionError::ContinuousDiscovery(_))
        ));
        assert!(game.mandatory_loop_draw_pending());
        assert_eq!(
            game.auxiliary_tracking.mandatory_loop_draw_controllers,
            HashSet::from([A, C])
        );
        assert!(game.players.iter().all(|player| player.is_in_game()));
        game.object_mut(source).unwrap().abilities_mut().clear();
        assert!(!game.resolve_mandatory_loop_draw().unwrap());
        assert!(!game.mandatory_loop_draw_pending());
        assert!(
            game.auxiliary_tracking
                .mandatory_loop_draw_controllers
                .is_empty()
        );
        assert!(!game.player(A).unwrap().is_in_game());
        assert!(!game.player(C).unwrap().is_in_game());
        assert!(game.player(B).unwrap().is_in_game());
        assert!(game.player(PlayerId(3)).unwrap().is_in_game());
    }
    #[test]
    fn direct_become_monarch_cannot_hide_discovery_failure_behind_cached_prohibition() {
        use crate::effects::EffectExecutor;
        let mut game = game();
        let source = unbounded_discovery_source(&mut game);
        game.effect_store.cant_effects.cant_become_monarch.insert(A);
        let mut ctx = crate::effects::ExecutionContext::new_default(source, A);
        assert!(matches!(
            crate::effects::BecomeMonarchEffect::you().execute(&mut game, &mut ctx),
            Err(ExecutionError::ContinuousDiscovery(_))
        ));
        assert_eq!(game.monarch, None);
        game.object_mut(source).unwrap().abilities_mut().clear();
        crate::effects::BecomeMonarchEffect::you()
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(game.monarch, Some(A));
    }
    #[test]
    fn simultaneous_draw_selects_successor_after_every_departing_restriction_is_removed() {
        for departing in [[A, B], [B, A]] {
            let mut game = game();
            game.turn.active_player = C;
            game.set_monarch(Some(A)).unwrap();
            let watcher = observer(&mut game, C);
            let restriction =
                CardDefinitionBuilder::new(crate::CardId::new(), "Monarch restriction")
                    .card_types(vec![crate::CardType::Enchantment])
                    .with_ability(Ability::static_ability(
                        crate::static_abilities::StaticAbility::restriction(
                            crate::effect::Restriction::BecomeMonarch(PlayerFilter::Specific(C)),
                            "C cannot become monarch".into(),
                        ),
                    ))
                    .build();
            game.create_object_from_definition(&restriction, B, Zone::Battlefield);
            game.refresh_continuous_state().unwrap();
            assert!(!game.can_become_monarch(C));
            game.draw_game_for_players(departing).unwrap();
            assert_eq!(game.monarch, Some(C));
            let entries = game.take_pending_trigger_entries();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].source, watcher);
            let event = entries[0]
                .triggering_event
                .downcast::<MonarchChangedEvent>()
                .unwrap();
            assert_eq!((event.previous, event.monarch), (Some(A), C));
        }
    }
    #[test]
    fn departing_active_anchor_is_retained_for_the_final_successor_scan() {
        let mut game = game();
        game.turn.active_player = C;
        game.set_monarch(Some(A)).unwrap();
        game.draw_game_for_players([A, C]).unwrap();
        assert_eq!(game.monarch, Some(PlayerId(3)));
    }
}

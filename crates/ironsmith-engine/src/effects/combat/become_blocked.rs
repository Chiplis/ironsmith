//! Change attacking creatures to blocked without creating any blocker.
use crate::effect::{ChoiceCount,EffectOutcome};
use crate::effects::{EffectExecutor,ExecutionContext,ExecutionError};
use crate::effects::helpers::resolve_objects_for_effect;
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub use ironsmith_core::BecomeBlockedEffect;
impl EffectExecutor for BecomeBlockedEffect {
    fn execute(&self,game:&mut GameState,ctx:&mut ExecutionContext)->Result<EffectOutcome,ExecutionError> {
        let checkpoint=game.clone();
        let result=(|| {
            let mut selected=resolve_objects_for_effect(game,ctx,&self.target)?;
            if ctx.decision_maker.awaiting_choice() {return Ok(EffectOutcome::resolved());}
            selected.sort_unstable();selected.dedup();
            selected.retain(|id|game.combat.as_ref().is_some_and(|combat|crate::combat_state::is_attacking(combat,*id)));
            if selected.is_empty() && self.target.is_target() && self.target.is_single() {return Err(ExecutionError::InvalidTarget);}
            let mut changed=Vec::new();
            if let Some(combat)=game.combat.as_mut() {
                for id in &selected {
                    if !crate::combat_state::is_blocked(combat,*id) {
                        let target=combat.attackers.iter().find(|entry|entry.creature==*id).map(|entry|(&entry.target).into());
                        combat.blocked_attackers.insert(*id);changed.push((*id,target));
                    }
                }
            }
            if !changed.is_empty() {game.mark_continuous_state_dirty();game.refresh_continuous_state().map_err(ExecutionError::ContinuousDiscovery)?;}
            // All flags and role-dependent continuous effects are complete
            // before any event captures characteristics or matches observers.
            let mut events=Vec::with_capacity(changed.len());
            if !changed.is_empty() {
                let observed=game.continuous_query_snapshot().map_err(ExecutionError::ContinuousDiscovery)?;
                let effects=observed.try_all_continuous_effects_arc().map_err(ExecutionError::ContinuousDiscovery)?;
                let completed=changed.into_iter().map(|(id,target)| {
                    let snapshot=observed.object(id).map(|object|crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics_and_effects(object,&observed,&effects));
                    (id,target,snapshot)
                }).collect::<Vec<_>>();
                for (id,target,snapshot) in completed {
                    let kind=crate::events::EventKind::CreatureBecameBlocked;
                    let provenance=if game.provenance_graph().node(ctx.provenance).is_some() {
                        game.alloc_child_event_provenance(ctx.provenance,kind)
                    } else {game.provenance_graph_mut().alloc_root_event(kind)};
                    let event=crate::events::CreatureBecameBlockedEvent::with_target_and_blockers(id,Vec::new(),target,snapshot,Vec::new());
                    events.push(crate::triggers::TriggerEvent::new_with_provenance(event,provenance));
                }
            }
            // A legal, already-blocked attacker is still the authored target
            // of a following instruction (for example, Choking Vines damage).
            Ok(EffectOutcome::with_objects(selected).with_events(events))
        })();
        if result.is_err() || ctx.decision_maker.awaiting_choice() {*game=checkpoint;}
        result
    }
    fn get_target_spec(&self)->Option<&ChooseSpec> { self.target.is_target().then_some(&self.target) }
    fn get_target_count(&self)->Option<ChoiceCount> { self.target.is_target().then(||self.target.count()) }
    fn target_description(&self)->&'static str {"attacking creature to become blocked"}
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CardId,CardType,ObjectId,PlayerId,Zone};
    use crate::card::{CardBuilder,PowerToughness};
    use crate::combat_state::{CombatState,AttackerInfo,AttackTarget};
    fn game()->(GameState,ObjectId,ObjectId) {
        let mut game=GameState::new(vec!["Alice".into(),"Bob".into()],20);
        let card=CardBuilder::new(CardId::new(),"Attacker").card_types(vec![CardType::Creature]).power_toughness(PowerToughness::fixed(2,2)).build();
        let first=game.create_object_from_card(&card,PlayerId::from_index(0),Zone::Battlefield);let second=game.create_object_from_card(&card,PlayerId::from_index(0),Zone::Battlefield);
        let mut combat=CombatState::default();combat.block_declaration_complete=true;
        for id in [first,second] {combat.attackers.push(AttackerInfo{creature:id,target:AttackTarget::Player(PlayerId::from_index(1))});}
        game.combat=Some(combat);(game,first,second)
    }
    #[test]
    fn actual_transitions_emit_once_with_zero_blockers_and_keep_legal_repeat_references() {
        let (mut game,first,second)=game();let mut filter=crate::target::ObjectFilter::creature();filter.attacking=true;
        let effect=BecomeBlockedEffect::with_spec(ChooseSpec::all(filter));let parent=game.provenance_graph_mut().alloc_root_event(crate::events::EventKind::SpellCast);
        let mut ctx=ExecutionContext::new_default(first,PlayerId::from_index(0)).with_provenance(parent);
        let outcome=effect.execute(&mut game,&mut ctx).unwrap();assert_eq!(outcome.events.len(),2);
        assert_ne!(outcome.events[0].provenance(),outcome.events[1].provenance());
        for event in &outcome.events { assert_ne!(event.provenance(),parent); game.stage_turn_history_event(event); }
        assert_eq!(game.turn_store.turn_history.projected_records().filter(|row|row.event.downcast::<crate::events::CreatureBecameBlockedEvent>().is_some()).count(),2);
        for event in &outcome.events {
            let event=event.downcast::<crate::events::CreatureBecameBlockedEvent>().unwrap();assert_eq!(event.blocker_count,0);assert!(event.blockers.is_empty());assert!(event.attacker_snapshot.is_some());
            assert_eq!(event.attack_target,Some(crate::triggers::AttackEventTarget::Player(PlayerId::from_index(1))));
        }
        assert!(crate::combat_state::is_blocked(game.combat.as_ref().unwrap(),first));assert!(crate::combat_state::is_blocked(game.combat.as_ref().unwrap(),second));
        let repeated=effect.execute(&mut game,&mut ctx).unwrap();assert!(repeated.events.is_empty());assert_eq!(repeated.objects().unwrap().len(),2);
        assert!(game.combat.as_ref().unwrap().blockers.is_empty());
    }
    #[test]
    fn untargeted_empty_set_resolves_and_nonattacker_target_does_not_change_combat() {
        let (mut game,first,second)=game();game.remove_object_from_combat(first);game.remove_object_from_combat(second);
        let mut ctx=ExecutionContext::new_default(first,PlayerId::from_index(0));
        let mut filter=crate::target::ObjectFilter::creature();filter.attacking=true;
        let result=BecomeBlockedEffect::with_spec(ChooseSpec::all(filter)).execute(&mut game,&mut ctx).unwrap();assert!(result.events.is_empty());
        assert!(game.combat.as_ref().unwrap().blocked_attackers.is_empty());
    }
    #[test]
    fn blocked_role_receipts_use_one_completed_calculated_characteristic_frame() {
        use crate::ability::Ability;
        use crate::static_abilities::StaticAbility;
        use crate::target::ObjectFilter;
        use crate::color::ColorSet;
        use crate::triggers::matcher_trait::{TriggerContext,TriggerMatcher};
        let (mut game,first,second)=game();
        let mut affected=ObjectFilter::creature();affected.blocked=true;
        let card=CardBuilder::new(CardId::new(),"Role observer").card_types(vec![CardType::Enchantment]).build();
        let mut definition=crate::cards::CardDefinition::new(card);
        definition.abilities=vec![
            Ability::static_ability(StaticAbility::set_colors(affected.clone(),ColorSet::BLUE)),
            Ability::static_ability(StaticAbility::add_card_types(affected.clone(),vec![CardType::Artifact])),
            Ability::static_ability(StaticAbility::set_base_power_toughness(affected,7,7)),
        ];
        let observer=game.create_object_from_definition(&definition,PlayerId::from_index(0),Zone::Battlefield);
        let mut filter=ObjectFilter::creature();filter.attacking=true;
        let effect=BecomeBlockedEffect::with_spec(ChooseSpec::all(filter));
        let result=effect.execute(&mut game,&mut ExecutionContext::new_default(observer,PlayerId::from_index(0))).unwrap();
        let matcher=crate::triggers::BlocksOrBecomesBlockedTrigger::new(ObjectFilter::creature().with_colors(ColorSet::BLUE).with_power(crate::target::Comparison::GreaterThanOrEqual(7)));
        let context=TriggerContext::new(observer,PlayerId::from_index(0),game.filter_context_for(PlayerId::from_index(0),Some(observer)),&game);
        assert_eq!(result.events.len(),2);
        for event in &result.events {
            let receipt=event.downcast::<crate::events::CreatureBecameBlockedEvent>().unwrap();
            let snapshot=receipt.attacker_snapshot.as_ref().unwrap();
            assert!(snapshot.card_types.contains(&CardType::Artifact));assert_eq!(snapshot.colors,ColorSet::BLUE);assert_eq!(snapshot.power,Some(7));
            assert!(matcher.matches(event,&context));
        }
        assert!(crate::combat_state::is_blocked(game.combat.as_ref().unwrap(),first));assert!(crate::combat_state::is_blocked(game.combat.as_ref().unwrap(),second));
    }

}

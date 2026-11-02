//! Ascend effect implementation for instants and sorceries.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;

pub use ironsmith_core::AscendEffect;

impl EffectExecutor for AscendEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        let permanent_count = game
            .battlefield
            .iter()
            .copied()
            // CR 702.26b: phased-out permanents are treated as though they
            // don't exist.
            .filter(|&object_id| {
                !game.is_phased_out(object_id)
                    && game.controller_of_id(object_id) == Some(controller)
                    && (!self.tapped_only || game.is_tapped(object_id))
                    && (!self.historic_only || game.current_card_types(object_id).is_some_and(|t| t.contains(&crate::types::CardType::Artifact))
                        || game.current_has_supertype(object_id, crate::types::Supertype::Legendary)
                        || game.current_subtypes(object_id).is_some_and(|t| t.contains(&crate::types::Subtype::Saga)))
            })
            .count();

        if permanent_count >= self.minimum as usize && game.designation_history_requirements_met(controller, &self.history_requirements) {
            if let Some(name) = &self.named_designation { game.grant_player_designation(controller, name); } else if self.enduring_story { game.grant_enduring_story(controller); } else { game.grant_citys_blessing(controller); }
        }
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::ids::{CardId, PlayerId};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn add_permanent(game: &mut GameState, controller: PlayerId, name: &str) {
        let definition = crate::CardDefinitionBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .build();
        game.create_object_from_definition(&definition, controller, Zone::Battlefield);
    }

    #[test]
    fn spell_ascend_checks_permanents_only_when_it_resolves() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..9 {
            add_permanent(&mut game, alice, &format!("Permanent {index}"));
        }

        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        AscendEffect::new().execute(&mut game, &mut ctx).unwrap();
        assert!(!game.has_citys_blessing(alice));

        add_permanent(&mut game, alice, "Tenth Permanent");
        AscendEffect::new().execute(&mut game, &mut ctx).unwrap();
        assert!(game.has_citys_blessing(alice));
    }

    #[test]
    fn permanent_ascend_is_checked_during_continuous_state_refresh() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        for index in 0..9 {
            add_permanent(&mut game, alice, &format!("Permanent {index}"));
        }
        let ascend_permanent = crate::CardDefinitionBuilder::new(CardId::new(), "Ascender")
            .card_types(vec![CardType::Creature])
            .with_ability(Ability::static_ability(StaticAbility::ascend()))
            .build();
        game.create_object_from_definition(&ascend_permanent, alice, Zone::Battlefield);

        game.refresh_continuous_state().expect("test continuous state refresh succeeds");
        assert!(game.has_citys_blessing(alice));
        assert!(
            game.stack.is_empty(),
            "permanent ascend does not use the stack"
        );
    }
}

#[cfg(test)]
mod designation_history_tests {
    use super::*;
    use crate::{CardDefinitionBuilder, CardId, PlayerId, Zone};
    use crate::types::CardType;
    use crate::events::spells::{SpellCastEvent, AbilityActivatedEvent};
    use crate::triggers::TriggerEvent;
    #[test]
    fn completed_turn_history_is_not_reconstructed_from_live_objects_or_carried_into_next_turn() {
        let a=PlayerId(0);let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let card=CardDefinitionBuilder::new(CardId::new(),"History source").card_types(vec![CardType::Creature]).power_toughness(crate::card::PowerToughness::fixed(1,1)).build();
        let actor=game.create_object_from_definition(&card,a,Zone::Battlefield);
        let spell=CardDefinitionBuilder::new(CardId::new(),"Instant receipt").card_types(vec![CardType::Instant]).build();
        let id=game.create_object_from_definition(&spell,a,Zone::Stack);
        let event=SpellCastEvent::from_completed_cast(id,a,Zone::Hand,&game);
        game.queue_trigger_event(Default::default(),TriggerEvent::new(event,Default::default()));
        game.queue_trigger_event(Default::default(),TriggerEvent::new(AbilityActivatedEvent::new(actor,a,false),Default::default()));
        game.mark_creature_attacked_this_turn(actor);
        game.remove_object(actor);game.remove_object(id);
        let requirements=vec![ironsmith_core::DesignationHistoryRequirement::CastSpellMatchingTypes(vec![CardType::Instant,CardType::Sorcery]),ironsmith_core::DesignationHistoryRequirement::ActivatedAbility{nonmana_only:true},ironsmith_core::DesignationHistoryRequirement::Attacked];
        assert!(game.designation_history_requirements_met(a,&requirements));
        game.turn_store.turn_history.clear_for_new_turn();
        assert!(!game.designation_history_requirements_met(a,&requirements));
        let mut designation=AscendEffect::designation_if_permanent_count(0,false,false);
        designation.named_designation=Some("renaissance".into());designation.history_requirements=requirements;
        let mut ctx=ExecutionContext::new_default(game.new_object_id(),a);
        designation.execute(&mut game,&mut ctx).unwrap();
        assert!(!game.has_player_designation(a,"renaissance"));
    }
}

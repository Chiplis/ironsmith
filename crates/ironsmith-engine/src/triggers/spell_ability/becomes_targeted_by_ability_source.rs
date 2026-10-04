//! Event-time participants of “the target of an ability of [source filter]”.
use crate::events::{BecomesTargetedEvent, EventKind};
use crate::filter::ObjectFilterExt as _;
use crate::target::ObjectFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct BecomesTargetedByAbilitySourceTrigger {
    pub target_filter: ObjectFilter,
    pub source_filter: ObjectFilter,
}
impl TriggerMatcher for BecomesTargetedByAbilitySourceTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(targeted) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        if !targeted.by_ability {
            return false;
        }
        let Some(target) = targeted
            .target_snapshot
            .as_ref()
            .filter(|snapshot| targeted.target_object() == Some(snapshot.object_id))
        else {
            return false;
        };
        let Some(source) = targeted
            .physical_source_snapshot
            .as_ref()
            .filter(|snapshot| snapshot.object_id == targeted.source)
        else {
            return false;
        };
        self.target_filter
            .matches_snapshot(target, &ctx.filter_ctx, ctx.game)
            && self
                .source_filter
                .matches_snapshot(source, &ctx.filter_ctx, ctx.game)
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::BecomesTargeted])
    }
    fn display(&self) -> String {
        format!(
            "Whenever {} becomes the target of an ability of {}",
            self.target_filter.description(),
            self.source_filter.description()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::game_state::{StackEntry, Target};
    use crate::ids::{CardId, PlayerId};
    use crate::target::PlayerFilter;
    use crate::types::CardType;
    use crate::zone::Zone;
    #[test]
    fn physical_source_filter_freezes_controller_name_and_type_not_ability_controller() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let creature = CardBuilder::new(CardId::new(), "Observer")
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 4))
            .build();
        let observer = game.create_object_from_card(&creature, a, Zone::Battlefield);
        let target = game.create_object_from_card(&creature, b, Zone::Battlefield);
        let land = CardBuilder::new(CardId::new(), "Named land")
            .card_types(vec![CardType::Land])
            .build();
        let land = game.create_object_from_card(&land, a, Zone::Battlefield);
        let mut source_filter = ObjectFilter::default()
            .in_zone(Zone::Battlefield)
            .with_type(CardType::Land);
        source_filter.controller = Some(PlayerFilter::You);
        source_filter.name = Some("Named land".into());
        let mut target_filter = ObjectFilter::creature();
        target_filter.other = true;
        let trigger = BecomesTargetedByAbilitySourceTrigger {
            target_filter,
            source_filter,
        };
        // The copier controls the independent ability; its source is still the
        // original land controlled by the observer's controller.
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(land).unwrap(),
            &game,
        );
        let proxy_card = CardBuilder::new(CardId::new(), "Copy proxy").build();
        let proxy = game.create_object_from_card(&proxy_card, b, Zone::Stack);
        let mut copied = StackEntry::ability(
            proxy,
            b,
            crate::resolution::ResolutionProgram::from_effects(vec![]),
        );
        copied.source_snapshot = Some(snapshot);
        game.stack.push(copied);
        let entry = game.stack.last().unwrap().clone();
        let event = BecomesTargetedEvent::from_stack_entry(Target::Object(target), &entry)
            .with_participant_snapshots(&game);
        let notification = TriggerEvent::new_with_provenance(event.clone(), Default::default());
        game.set_current_controller(land, b).unwrap();
        game.move_object_by_effect(land, Zone::Graveyard).unwrap();
        game.move_object_by_effect(target, Zone::Exile).unwrap();
        assert!(trigger.matches(
            &notification,
            &TriggerContext::for_source(observer, a, &game)
        ));
        let mut spell = event.clone();
        spell.by_ability = false;
        assert!(!trigger.matches(
            &TriggerEvent::new_with_provenance(spell, Default::default()),
            &TriggerContext::for_source(observer, a, &game)
        ));
        let mut wrong = event.clone();
        wrong.physical_source_snapshot.as_mut().unwrap().controller = b;
        assert!(!trigger.matches(
            &TriggerEvent::new_with_provenance(wrong, Default::default()),
            &TriggerContext::for_source(observer, a, &game)
        ));
        let mut wrong = event.clone();
        wrong.physical_source_snapshot.as_mut().unwrap().name = "Other land".into();
        assert!(!trigger.matches(
            &TriggerEvent::new_with_provenance(wrong, Default::default()),
            &TriggerContext::for_source(observer, a, &game)
        ));
        let mut wrong = event;
        wrong.target = Target::Object(observer);
        wrong.target_snapshot.as_mut().unwrap().object_id = observer;
        assert!(!trigger.matches(
            &TriggerEvent::new_with_provenance(wrong, Default::default()),
            &TriggerContext::for_source(observer, a, &game)
        ));
    }
}

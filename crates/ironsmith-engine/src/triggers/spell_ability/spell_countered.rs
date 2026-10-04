//! "Whenever [spell] is countered" trigger.

use crate::events::EventKind;
use crate::events::SpellCounteredEvent;
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::target::{ObjectFilter, PlayerFilter};
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCounteredTrigger {
    pub filter: Option<ObjectFilter>,
    pub controller: PlayerFilter,
}

impl SpellCounteredTrigger {
    pub fn new(filter: Option<ObjectFilter>, controller: PlayerFilter) -> Self {
        Self { filter, controller }
    }
}

impl TriggerMatcher for SpellCounteredTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::SpellCountered {
            return false;
        }
        let Some(countered) = event.downcast::<SpellCounteredEvent>() else {
            return false;
        };
        if !self
            .controller
            .matches_player(countered.controller, &ctx.filter_ctx)
        {
            return false;
        }
        let Some(filter) = &self.filter else {
            return true;
        };
        if let Some(snapshot) = countered.snapshot.as_ref().or_else(|| event.snapshot()) {
            return filter.matches_snapshot(snapshot, &ctx.filter_ctx, ctx.game);
        }
        ctx.game
            .object(countered.spell)
            .is_some_and(|object| filter.matches(object, &ctx.filter_ctx, ctx.game))
    }

    fn display(&self) -> String {
        match &self.controller {
            PlayerFilter::You => "Whenever a spell you've cast is countered".to_string(),
            PlayerFilter::Opponent => "Whenever a spell an opponent cast is countered".to_string(),
            player => format!(
                "Whenever a spell {} cast is countered",
                describe_player_filter_subject(player)
            ),
        }
    }

    fn looks_back_for_source(&self, event: &TriggerEvent) -> bool {
        event.kind() == EventKind::SpellCountered
    }
}

#[cfg(test)]
mod completed_frame_tests {
    use super::*;
    use crate::ability::Ability;
    use crate::cards::CardDefinitionBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;
    #[test]
    fn complete_counter_frame_keeps_departed_observer_and_excludes_later_arrival() {
        let mut game = crate::tests::test_helpers::setup_two_player_game(); let a = PlayerId::from_index(0);
        let definition = CardDefinitionBuilder::new(CardId::new(), "Counter observer")
            .card_types(vec![CardType::Artifact]).with_ability(Ability::triggered(
                crate::triggers::Trigger::new(SpellCounteredTrigger::new(None, PlayerFilter::Any)),
                vec![crate::effect::Effect::draw(1)])).build();
        let old = game.create_object_from_definition(&definition,a,Zone::Battlefield);
        let before = game.trigger_source_lookback_snapshots();
        game.move_object_by_effect(old,Zone::Graveyard).unwrap();
        let later = game.create_object_from_definition(&definition,a,Zone::Battlefield);
        let event = TriggerEvent::new_with_provenance(SpellCounteredEvent::new(crate::ids::ObjectId::from_raw(999),a,None)
            .with_complete_source_lookback(),Default::default()).with_lookback_source_snapshots(before);
        let triggers = crate::triggers::check_triggers(&game,&event);
        assert_eq!(triggers.len(),1);assert_eq!(triggers[0].source,old);assert_ne!(triggers[0].source,later);
    }
}

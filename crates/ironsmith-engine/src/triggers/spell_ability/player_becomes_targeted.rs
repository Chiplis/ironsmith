//! Player-only target transitions, with an independent targeting controller.
use crate::events::{BecomesTargetedEvent, EventKind};
use crate::filter::PlayerFilterExt as _;
use crate::target::PlayerFilter;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use ironsmith_core::filter_model::StackObjectKind;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerBecomesTargetedTrigger {
    pub player_filter: PlayerFilter,
    pub source_controller: PlayerFilter,
    pub source_kind: StackObjectKind,
}
impl TriggerMatcher for PlayerBecomesTargetedTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(targeted) = event.downcast::<BecomesTargetedEvent>() else {
            return false;
        };
        let Some(player) = targeted.target_player() else {
            return false;
        };
        let kind = match self.source_kind {
            StackObjectKind::Spell => !targeted.by_ability,
            StackObjectKind::Ability => targeted.by_ability,
            StackObjectKind::SpellOrAbility => true,
            // This event's boolean alone cannot prove a specific ability kind.
            StackObjectKind::ActivatedAbility | StackObjectKind::TriggeredAbility => false,
        };
        kind && self.player_filter.matches_player(player, &ctx.filter_ctx)
            && self
                .source_controller
                .matches_player(targeted.source_controller, &ctx.filter_ctx)
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::BecomesTargeted])
    }
    fn display(&self) -> String {
        let verb = if self.player_filter == PlayerFilter::You {
            "become"
        } else {
            "becomes"
        };
        let kind = match self.source_kind {
            StackObjectKind::Spell => "a spell",
            StackObjectKind::Ability => "an ability",
            _ => "a spell or ability",
        };
        let controller = match &self.source_controller {
            PlayerFilter::Any => String::new(),
            PlayerFilter::You => " you control".into(),
            player => format!(" {} controls", player.description()),
        };
        format!(
            "Whenever {} {verb} the target of {kind}{controller}",
            self.player_filter.description()
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{ObjectId, PlayerId};
    #[test]
    fn player_target_is_not_its_permanent_and_targeting_controller_is_independent() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let a = PlayerId::from_index(0);
        let b = PlayerId::from_index(1);
        let ctx = TriggerContext::for_source(ObjectId::from_raw(100), a, &game);
        let mut trigger = PlayerBecomesTargetedTrigger {
            player_filter: PlayerFilter::You,
            source_controller: PlayerFilter::Opponent,
            source_kind: StackObjectKind::SpellOrAbility,
        };
        let event = |target, actor, ability| {
            TriggerEvent::new_with_provenance(
                BecomesTargetedEvent::new_player(target, ObjectId::from_raw(200), actor, ability),
                Default::default(),
            )
        };
        assert!(trigger.matches(&event(a, b, false), &ctx));
        assert!(trigger.matches(&event(a, b, true), &ctx));
        assert!(!trigger.matches(&event(b, b, false), &ctx));
        assert!(!trigger.matches(&event(a, a, false), &ctx));
        let object = TriggerEvent::new_with_provenance(
            BecomesTargetedEvent::new(ObjectId::from_raw(100), ObjectId::from_raw(200), b, false),
            Default::default(),
        );
        assert!(!trigger.matches(&object, &ctx));
        trigger.source_kind = StackObjectKind::Spell;
        assert!(!trigger.matches(&event(a, b, true), &ctx));
    }
}

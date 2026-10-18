//! A delayed watcher bound to the exact play grants made at registration.
use crate::events::EventKind;
use crate::grant_registry::GrantPermissionIdentity;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct SpellCastThroughPermissionTrigger {
    pub permissions: Vec<GrantPermissionIdentity>,
}

impl TriggerMatcher for SpellCastThroughPermissionTrigger {
    fn matches(&self, event: &TriggerEvent, _: &TriggerContext) -> bool {
        event.downcast::<crate::events::SpellCastEvent>()
            .and_then(|cast| cast.play_permission.as_ref())
            .is_some_and(|identity| self.permissions.contains(identity))
    }

    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> { Some(vec![EventKind::SpellCast]) }

    fn display(&self) -> String { "When you cast a spell this way".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_permission_survives_departure_and_rejects_another_grant() {
        let game = crate::GameState::new(vec!["A".into(), "B".into()], 20);
        let player = crate::PlayerId::from_index(0);
        let source = crate::ObjectId::from_raw(91);
        let ctx = TriggerContext::new(source, player, game.filter_context_for(player, Some(source)), &game);
        let trigger = SpellCastThroughPermissionTrigger { permissions: vec![GrantPermissionIdentity::Stored(7)] };
        for (permission, expected) in [(None, false), (Some(GrantPermissionIdentity::Stored(8)), false), (Some(GrantPermissionIdentity::Stored(7)), true)] {
            // The event remains sufficient after both source and spell leave.
            let mut cast = crate::events::SpellCastEvent::new(crate::ObjectId::from_raw(92), player, crate::Zone::Exile);
            cast.play_permission = permission;
            let event = TriggerEvent::new(cast, crate::provenance::ProvNodeId::default());
            assert_eq!(trigger.matches(&event, &ctx), expected);
        }
    }
}

//! "Whenever [target] is dealt damage" trigger.

use crate::events::{DamageEvent, EventKind};
use crate::game_event::DamageTarget;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};

#[derive(Debug, Clone, PartialEq)]
pub struct IsDealtDamageTrigger {
    pub target: ChooseSpec,
}

impl IsDealtDamageTrigger {
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }
}

impl TriggerMatcher for IsDealtDamageTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::Damage {
            return false;
        }
        let Some(e) = event.downcast::<DamageEvent>() else {
            return false;
        };

        match e.target {
            DamageTarget::Object(object_id) => target_matches_object(&self.target, object_id, ctx),
            DamageTarget::Player(player_id) => target_matches_player(&self.target, player_id, ctx),
        }
    }

    fn display(&self) -> String {
        "Whenever this creature is dealt damage".to_string()
    }

    fn clone_box(&self) -> Box<dyn TriggerMatcher> {
        Box::new(self.clone())
    }
}

fn target_matches_object(
    spec: &ChooseSpec,
    object_id: crate::ids::ObjectId,
    ctx: &TriggerContext,
) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            target_matches_object(inner, object_id, ctx)
        }
        ChooseSpec::Source => object_id == ctx.source_id,
        ChooseSpec::SpecificObject(id) => object_id == *id,
        ChooseSpec::Object(filter) => ctx
            .game
            .object(object_id)
            .is_some_and(|obj| filter.matches(obj, &ctx.filter_ctx, ctx.game)),
        ChooseSpec::AnyTarget => true,
        _ => false,
    }
}

fn target_matches_player(
    spec: &ChooseSpec,
    player_id: crate::ids::PlayerId,
    ctx: &TriggerContext,
) -> bool {
    match spec {
        ChooseSpec::Target(inner) | ChooseSpec::WithCount(inner, _) => {
            target_matches_player(inner, player_id, ctx)
        }
        ChooseSpec::SourceController => player_id == ctx.controller,
        ChooseSpec::SourceOwner => ctx
            .game
            .object(ctx.source_id)
            .is_some_and(|obj| obj.owner == player_id),
        ChooseSpec::SpecificPlayer(id) => player_id == *id,
        ChooseSpec::Player(filter) => filter.matches_player(player_id, &ctx.filter_ctx),
        ChooseSpec::AnyTarget => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display() {
        let trigger = IsDealtDamageTrigger::new(ChooseSpec::creature());
        assert!(trigger.display().contains("dealt damage"));
    }
}

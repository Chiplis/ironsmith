use crate::events::{EventKind, PlayerAttackDeclarationEvent};
use crate::filter::player_filter_matches_game;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::SimultaneousTriggerKey;
use crate::triggers::{TriggerContext, TriggerEvent, TriggerMatcher};
use ironsmith_core::trigger_model::PlayerAttackGrouping;

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerAttackDeclarationTrigger {
    pub attacker: PlayerFilter,
    pub defender: PlayerFilter,
    pub grouping: PlayerAttackGrouping,
}
impl TriggerMatcher for PlayerAttackDeclarationTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        let Some(event) = event.downcast::<PlayerAttackDeclarationEvent>() else {
            return false;
        };
        if !event.directly_attacked_player && self.grouping != PlayerAttackGrouping::AttackerAnyTarget {
            return false;
        }
        player_filter_matches_game(&self.attacker, event.attacker, ctx.game, &ctx.filter_ctx)
            && player_filter_matches_game(&self.defender, event.defender, ctx.game, &ctx.filter_ctx)
    }
    fn subscribed_kinds(&self) -> Option<Vec<EventKind>> {
        Some(vec![EventKind::PlayerAttackDeclaration])
    }
    fn simultaneous_trigger_key(&self, event: &TriggerEvent) -> Option<SimultaneousTriggerKey> {
        let event = event.downcast::<PlayerAttackDeclarationEvent>()?;
        match self.grouping {
            PlayerAttackGrouping::Attacker | PlayerAttackGrouping::AttackerAnyTarget => {
                Some(SimultaneousTriggerKey::PlayerAttackActor(event.attacker))
            }
            PlayerAttackGrouping::Defender => {
                Some(SimultaneousTriggerKey::PlayerAttackDefender(event.defender))
            }
            PlayerAttackGrouping::Pair => None,
        }
    }
    fn display(&self) -> String {
        if self.grouping == PlayerAttackGrouping::AttackerAnyTarget {
            return format!("Whenever {} {}",
                crate::triggers::describe_player_filter_subject(&self.attacker),
                if self.attacker == PlayerFilter::You { "attack" } else { "attacks" });
        }
        if self.grouping == PlayerAttackGrouping::Defender {
            format!(
                "Whenever {} is attacked",
                crate::triggers::describe_player_filter_subject(&self.defender)
            )
        } else {
            let actor = crate::triggers::describe_player_filter_subject(&self.attacker);
            let verb = if self.attacker == PlayerFilter::You {
                "attack"
            } else {
                "attacks"
            };
            let recipient = if self.defender == PlayerFilter::Opponent
                && self.grouping == PlayerAttackGrouping::Attacker
            {
                "your opponents".to_owned()
            } else {
                self.defender.description()
            };
            format!(
                "Whenever {actor} {verb} {}{recipient}",
                if self.grouping == PlayerAttackGrouping::Attacker {
                    "one or more of "
                } else {
                    ""
                }
            )
        }
    }
}

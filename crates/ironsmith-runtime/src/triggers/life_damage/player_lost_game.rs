//! "Whenever [player] loses the game" trigger.

use crate::events::EventKind;
use crate::events::other::PlayerLostGameEvent;
use crate::target::PlayerFilter;
use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
use crate::triggers::{TriggerEvent, describe_player_filter_subject};

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerLostGameTrigger {
    pub player: PlayerFilter,
}

impl PlayerLostGameTrigger {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

impl TriggerMatcher for PlayerLostGameTrigger {
    fn matches(&self, event: &TriggerEvent, ctx: &TriggerContext) -> bool {
        if event.kind() != EventKind::PlayerLostGame {
            return false;
        }
        let Some(e) = event.downcast::<PlayerLostGameEvent>() else {
            return false;
        };
        player_lost_game_filter_matches(&self.player, e.player, ctx)
    }

    fn display(&self) -> String {
        match &self.player {
            PlayerFilter::You => "Whenever you lose the game".to_string(),
            _ => format!(
                "Whenever {} loses the game",
                describe_player_filter_subject(&self.player)
            ),
        }
    }
}

fn player_lost_game_filter_matches(
    filter: &PlayerFilter,
    player: crate::ids::PlayerId,
    ctx: &TriggerContext,
) -> bool {
    match filter {
        PlayerFilter::Any => true,
        PlayerFilter::You | PlayerFilter::EffectController => player == ctx.controller,
        PlayerFilter::NotYou | PlayerFilter::Opponent => player != ctx.controller,
        PlayerFilter::Specific(id) => player == *id,
        PlayerFilter::Active => player == ctx.game.turn.active_player,
        PlayerFilter::Defending => ctx.filter_ctx.defending_player == Some(player),
        PlayerFilter::Attacking => ctx.filter_ctx.attacking_player == Some(player),
        PlayerFilter::Teammate => ctx.filter_ctx.teammates.contains(&player),
        PlayerFilter::ChosenPlayer => ctx.filter_ctx.chosen_player == Some(player),
        PlayerFilter::TaggedPlayer(tag) => ctx
            .filter_ctx
            .tagged_players
            .get(tag)
            .is_some_and(|players| players.contains(&player)),
        PlayerFilter::IteratedPlayer => ctx.filter_ctx.iterated_player == Some(player),
        PlayerFilter::Target(inner) => {
            ctx.filter_ctx.target_players.contains(&player)
                && player_lost_game_filter_matches(inner, player, ctx)
        }
        PlayerFilter::Excluding { base, excluded } => {
            player_lost_game_filter_matches(base, player, ctx)
                && !player_lost_game_filter_matches(excluded, player, ctx)
        }
        PlayerFilter::TargetPlayerOrControllerOfTarget => {
            ctx.filter_ctx.target_players.contains(&player)
                || ctx
                    .filter_ctx
                    .target_objects
                    .first()
                    .is_some_and(|snapshot| snapshot.controller == player)
        }
        PlayerFilter::ControllerOf(object_ref) | PlayerFilter::AliasedControllerOf(object_ref) => {
            player_from_object_ref(ctx, object_ref, ObjectRefPlayer::Controller) == Some(player)
        }
        PlayerFilter::OwnerOf(object_ref) | PlayerFilter::AliasedOwnerOf(object_ref) => {
            player_from_object_ref(ctx, object_ref, ObjectRefPlayer::Owner) == Some(player)
        }
        PlayerFilter::DamagedPlayer
        | PlayerFilter::MostLifeTied
        | PlayerFilter::LowestLifeTied
        | PlayerFilter::MostCardsInHand
        | PlayerFilter::CastCardTypeThisTurn(_)
        | PlayerFilter::CardsInHandAtLeastMoreThanYou { .. }
        | PlayerFilter::HasMoreLifeThanYou { .. }
        | PlayerFilter::MaxSpeed { .. } => false,
    }
}

enum ObjectRefPlayer {
    Controller,
    Owner,
}

fn player_from_object_ref(
    ctx: &TriggerContext,
    object_ref: &crate::target::ObjectRef,
    role: ObjectRefPlayer,
) -> Option<crate::ids::PlayerId> {
    let snapshot = match object_ref {
        crate::target::ObjectRef::Target => ctx.filter_ctx.target_objects.first(),
        crate::target::ObjectRef::Specific(id) => ctx
            .filter_ctx
            .target_objects
            .iter()
            .chain(ctx.filter_ctx.tagged_objects.values().flatten())
            .find(|snapshot| snapshot.object_id == *id),
        crate::target::ObjectRef::Tagged(tag) => ctx
            .filter_ctx
            .tagged_objects
            .get(tag)
            .and_then(|snapshots| snapshots.first()),
    }?;
    Some(match role {
        ObjectRefPlayer::Controller => snapshot.controller,
        ObjectRefPlayer::Owner => snapshot.owner,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_mentions_loses_the_game() {
        let trigger = PlayerLostGameTrigger::new(PlayerFilter::Any);
        assert!(trigger.display().contains("loses the game"));
    }

    #[test]
    fn matches_you_and_opponent_from_controller_context() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);
        let source = crate::ids::ObjectId::from_raw(1);
        let ctx = TriggerContext::for_source(source, alice, &game);
        let bob_lost = TriggerEvent::new_with_provenance(
            PlayerLostGameEvent::new(bob),
            crate::provenance::ProvNodeId::default(),
        );
        let alice_lost = TriggerEvent::new_with_provenance(
            PlayerLostGameEvent::new(alice),
            crate::provenance::ProvNodeId::default(),
        );

        assert!(PlayerLostGameTrigger::new(PlayerFilter::Any).matches(&bob_lost, &ctx));
        assert!(PlayerLostGameTrigger::new(PlayerFilter::Opponent).matches(&bob_lost, &ctx));
        assert!(!PlayerLostGameTrigger::new(PlayerFilter::You).matches(&bob_lost, &ctx));
        assert!(PlayerLostGameTrigger::new(PlayerFilter::You).matches(&alice_lost, &ctx));
        assert!(!PlayerLostGameTrigger::new(PlayerFilter::Opponent).matches(&alice_lost, &ctx));
    }
}

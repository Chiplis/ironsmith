//! Grant temporary "you may play this exiled card" permissions for tagged cards.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::Grantable;
use crate::grant_registry::GrantSource;
use crate::tag::TagKey;
use crate::target::PlayerFilter;
use crate::zone::Zone;

/// Duration for play-from-exile permissions granted from tagged cards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantPlayTaggedDuration {
    /// Permission expires at end of the current turn.
    UntilEndOfTurn,
    /// Permission expires at end of the granted player's next turn.
    UntilYourNextTurnEnd,
}

/// Grant "you may play" from exile for cards tagged in the current context.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantPlayTaggedEffect {
    pub tag: TagKey,
    pub player: PlayerFilter,
    pub duration: GrantPlayTaggedDuration,
}

impl GrantPlayTaggedEffect {
    pub fn new(
        tag: impl Into<TagKey>,
        player: PlayerFilter,
        duration: GrantPlayTaggedDuration,
    ) -> Self {
        Self {
            tag: tag.into(),
            player,
            duration,
        }
    }

    pub fn until_your_next_turn(tag: impl Into<TagKey>, player: PlayerFilter) -> Self {
        Self::new(tag, player, GrantPlayTaggedDuration::UntilYourNextTurnEnd)
    }

    fn expires_end_of_turn(&self, game: &GameState, player: crate::ids::PlayerId) -> u32 {
        match self.duration {
            GrantPlayTaggedDuration::UntilEndOfTurn => game.turn.turn_number,
            GrantPlayTaggedDuration::UntilYourNextTurnEnd => {
                if game.turn.active_player == player {
                    game.turn.turn_number.saturating_add(2)
                } else {
                    game.turn.turn_number.saturating_add(1)
                }
            }
        }
    }
}

impl EffectExecutor for GrantPlayTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let Some(snapshots) = ctx.get_tagged_all(self.tag.as_str()).cloned() else {
            return Ok(EffectOutcome::count(0));
        };

        let expires_end_of_turn = self.expires_end_of_turn(game, player_id);
        let mut granted = 0usize;
        let mut seen = std::collections::HashSet::new();
        for snapshot in snapshots {
            let mut object_id = snapshot.object_id;
            if game.object(object_id).is_none() {
                if let Some(found) = game.find_object_by_stable_id(snapshot.stable_id) {
                    object_id = found;
                } else {
                    continue;
                }
            }

            let Some(object) = game.object(object_id) else {
                continue;
            };
            if object.zone != Zone::Exile || !seen.insert(object_id) {
                continue;
            }

            game.grant_registry.grant_to_card(
                object_id,
                Zone::Exile,
                player_id,
                Grantable::PlayFrom,
                GrantSource::Effect {
                    source_id: ctx.source,
                    expires_end_of_turn,
                },
            );
            granted += 1;
        }

        Ok(EffectOutcome::count(granted as i32))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::EffectResult;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::snapshot::ObjectSnapshot;

    #[test]
    fn grant_play_tagged_until_your_next_turn_applies_to_tagged_exile_cards() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let card = CardBuilder::new(CardId::from_raw(1), "Exiled Card").build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("exiled card"), &game);

        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let mut dm = SelectFirstDecisionMaker;
        let source = ObjectId::from_raw(100);
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let effect = GrantPlayTaggedEffect::until_your_next_turn("it", PlayerFilter::You);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(outcome.result, EffectResult::Count(1));
        assert!(
            game.grant_registry
                .card_can_play_from_zone(&game, exiled_id, Zone::Exile, alice),
            "tagged card should be playable from exile"
        );

        let grant = game
            .grant_registry
            .grants
            .first()
            .expect("grant should exist");
        match grant.source {
            GrantSource::Effect {
                expires_end_of_turn,
                ..
            } => {
                assert_eq!(
                    expires_end_of_turn,
                    game.turn.turn_number + 2,
                    "when cast on your own turn, permission should last through your next turn"
                );
            }
            _ => panic!("expected effect grant source"),
        }
    }
}

//! Player tagging for repeated greatest-mana-value tie breaks (Timesifter).
//!
//! Both effects only rebind resolution-local player tags; they change no
//! game state, so they need no transaction of their own.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::player_filter_matches_game;
use crate::game_state::GameState;
use crate::ids::PlayerId;
pub use ironsmith_core::{
    KeepGreatestManaValuePlayersEffect, RollDieKeepHighestPlayersEffect, TagPlayersEffect,
};

/// In-game players in APNAP order (CR 101.4).
fn players_in_turn_order(game: &GameState) -> Vec<PlayerId> {
    crate::effects::composition::player_order::active_players_from(
        game,
        game.turn.active_player,
        crate::effects::composition::player_order::PlayerOrderBasis::Seating,
    )
}

impl EffectExecutor for TagPlayersEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let filter_ctx = ctx.filter_context(game);
        let players = players_in_turn_order(game)
            .into_iter()
            .filter(|player| player_filter_matches_game(&self.filter, *player, game, &filter_ctx))
            .collect::<Vec<_>>();
        let count = players.len();
        ctx.set_tagged_players(self.tag.clone(), players);
        Ok(EffectOutcome::count(count as i32))
    }
}

impl EffectExecutor for KeepGreatestManaValuePlayersEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        _game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let contenders = ctx
            .tagged_players
            .get(&self.players_tag)
            .cloned()
            .unwrap_or_default();
        let cards = ctx
            .get_tagged_all(self.objects_tag.as_str())
            .cloned()
            .unwrap_or_default();
        // Each contender's card from this round; a contender who exiled no
        // card has none to compare.
        let kept = crate::effects::composition::captured_selection::retain_extrema_by_key(
            contenders,
            |player| {
                cards
                    .iter()
                    .filter(|card| card.owner == *player)
                    .map(|card| card.mana_value())
                    .max()
            },
            true,
        );
        let count = kept.len();
        ctx.set_tagged_players(self.players_tag.clone(), kept);
        ctx.clear_object_tag(self.objects_tag.as_str());
        Ok(EffectOutcome::count(count as i32))
    }
}

impl EffectExecutor for RollDieKeepHighestPlayersEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let contenders = ctx
            .tagged_players
            .get(&self.players_tag)
            .cloned()
            .unwrap_or_default();
        let order = players_in_turn_order(game);
        let mut events = Vec::new();
        let mut results = Vec::new();
        for player in order.into_iter().filter(|player| contenders.contains(player)) {
            let roll = crate::effects::RollDieEffect::new(
                crate::target::PlayerFilter::Specific(player),
                self.sides,
            );
            let outcome = roll.execute_child(game, ctx)?;
            if ctx.decision_maker.awaiting_choice() {
                return Ok(EffectOutcome::count(0).with_events(events));
            }
            let result = outcome.as_count();
            events.extend(outcome.events);
            if let Some(result) = result {
                results.push((player, result));
            }
        }
        let highest = results.iter().map(|(_, result)| *result).max();
        let filter_ctx = ctx.filter_context(game);
        let kept = results
            .into_iter()
            .filter(|(_, result)| Some(*result) == highest)
            .map(|(player, _)| player)
            .filter(|player| player_filter_matches_game(&self.keep, *player, game, &filter_ctx))
            .collect::<Vec<_>>();
        let count = kept.len();
        ctx.set_tagged_players(self.players_tag.clone(), kept);
        Ok(EffectOutcome::count(count as i32).with_events(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::snapshot::ObjectSnapshot;
    use crate::target::PlayerFilter;
    use crate::zone::Zone;

    #[test]
    fn keeps_only_the_players_tied_for_the_greatest_mana_value() {
        let mut game = GameState::new(
            vec!["Alice".to_string(), "Bob".to_string(), "Carol".to_string()],
            20,
        );
        let [alice, bob, carol] = [
            PlayerId::from_index(0),
            PlayerId::from_index(1),
            PlayerId::from_index(2),
        ];
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        TagPlayersEffect::new(PlayerFilter::Any, "contenders")
            .execute(&mut game, &mut ctx)
            .expect("seed contenders");
        assert_eq!(ctx.tagged_players.get("contenders").map(Vec::len), Some(3));

        let card = |id: u32, cost: crate::mana::ManaCost| {
            CardBuilder::new(CardId::from_raw(id), "Exiled")
                .card_types(vec![crate::types::CardType::Sorcery])
                .mana_cost(cost)
                .build()
        };
        let three = crate::mana::ManaCost::new().add_generic(3);
        let one = crate::mana::ManaCost::new().add_generic(1);
        for (id, owner, cost) in [(1, alice, three.clone()), (2, bob, three), (3, carol, one)] {
            let object = game.create_object_from_card(&card(id, cost), owner, Zone::Exile);
            let snapshot = ObjectSnapshot::from_object(game.object(object).expect("card"), &game);
            ctx.tag_object("round", snapshot);
        }
        let outcome = KeepGreatestManaValuePlayersEffect::new("contenders", "round")
            .execute(&mut game, &mut ctx)
            .expect("narrow contenders");
        assert_eq!(outcome.as_count(), Some(2), "Alice and Bob are tied");
        assert_eq!(
            ctx.tagged_players.get("contenders"),
            Some(&vec![alice, bob])
        );
        assert!(
            ctx.get_tagged_all("round").is_none(),
            "the next round starts empty"
        );
    }
}

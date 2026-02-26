//! Clash effect implementation.
//!
//! Clash (701.30): You and an opponent each reveal the top card of your library,
//! then each may put that card on the bottom of their library. A player wins if
//! their revealed card has greater mana value.

use crate::decisions::{ChoiceSpec, DisplayOption, ScrySpec, make_decision};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

/// Effect that performs a clash with an opponent.
#[derive(Debug, Clone, PartialEq)]
pub struct ClashEffect;

impl ClashEffect {
    /// Create a new clash effect.
    pub fn new() -> Self {
        Self
    }
}

fn in_game_opponents(game: &GameState, controller: PlayerId) -> Vec<PlayerId> {
    game.players
        .iter()
        .filter(|player| player.id != controller && player.is_in_game())
        .map(|player| player.id)
        .collect()
}

fn targeted_opponent(ctx: &ExecutionContext, opponents: &[PlayerId]) -> Option<PlayerId> {
    ctx.targets
        .iter()
        .find_map(|target| match target {
            ResolvedTarget::Player(player) if opponents.contains(player) => Some(*player),
            _ => None,
        })
}

fn choose_opponent(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    opponents: &[PlayerId],
) -> Option<PlayerId> {
    if let Some(targeted) = targeted_opponent(ctx, opponents) {
        return Some(targeted);
    }
    if opponents.is_empty() {
        return None;
    }
    if opponents.len() == 1 {
        return opponents.first().copied();
    }

    let options: Vec<DisplayOption> = opponents
        .iter()
        .enumerate()
        .map(|(index, player_id)| {
            let name = game
                .player(*player_id)
                .map(|player| player.name.clone())
                .unwrap_or_else(|| format!("Player {}", player_id.0));
            DisplayOption::new(index, name)
        })
        .collect();

    let spec = ChoiceSpec::single(ctx.source, options);
    let chosen = make_decision(
        game,
        &mut ctx.decision_maker,
        ctx.controller,
        Some(ctx.source),
        spec,
    );
    let index = chosen.first().copied().unwrap_or(0);
    opponents
        .get(index)
        .copied()
        .or_else(|| opponents.first().copied())
}

fn top_card(game: &GameState, player: PlayerId) -> Option<ObjectId> {
    game.player(player).and_then(|entry| entry.library.last().copied())
}

fn card_mana_value(game: &GameState, card: Option<ObjectId>) -> Option<u32> {
    card.and_then(|card_id| {
        game.object(card_id)
            .map(|object| object.mana_cost.as_ref().map_or(0, |cost| cost.mana_value()))
    })
}

fn maybe_put_revealed_card_on_bottom(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    player: PlayerId,
    card: ObjectId,
) {
    let spec = ScrySpec::new(ctx.source, vec![card]);
    let to_bottom: Vec<ObjectId> =
        make_decision(game, &mut ctx.decision_maker, player, Some(ctx.source), spec);

    if !to_bottom.contains(&card) {
        return;
    }

    if let Some(player_state) = game.player_mut(player)
        && let Some(pos) = player_state.library.iter().position(|id| *id == card)
    {
        player_state.library.remove(pos);
        player_state.library.insert(0, card);
    }
}

fn controller_wins_clash(controller_mv: Option<u32>, opponent_mv: Option<u32>) -> bool {
    match (controller_mv, opponent_mv) {
        (Some(left), Some(right)) => left > right,
        (Some(_), None) => true,
        _ => false,
    }
}

impl EffectExecutor for ClashEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let opponents = in_game_opponents(game, ctx.controller);
        let Some(opponent) = choose_opponent(game, ctx, &opponents) else {
            return Ok(EffectOutcome::count(0));
        };

        let controller_card = top_card(game, ctx.controller);
        let opponent_card = top_card(game, opponent);

        let did_win = controller_wins_clash(
            card_mana_value(game, controller_card),
            card_mana_value(game, opponent_card),
        );

        if let Some(card) = controller_card {
            maybe_put_revealed_card_on_bottom(game, ctx, ctx.controller, card);
        }
        if let Some(card) = opponent_card {
            maybe_put_revealed_card_on_bottom(game, ctx, opponent, card);
        }

        Ok(EffectOutcome::count(if did_win { 1 } else { 0 }))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::decision::DecisionMaker;
    use crate::effect::EffectResult;
    use crate::ids::CardId;
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn spell_card(card_id: u32, name: &str, mana_value: u8) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(mana_value)]))
            .card_types(vec![CardType::Sorcery])
            .build()
    }

    fn setup_source(game: &mut GameState, controller: PlayerId) -> ObjectId {
        let source = spell_card(100, "Clash Source", 2);
        game.create_object_from_card(&source, controller, Zone::Stack)
    }

    struct PutRevealedOnBottom;

    impl DecisionMaker for PutRevealedOnBottom {
        fn decide_partition(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::PartitionContext,
        ) -> Vec<ObjectId> {
            ctx.cards.iter().map(|(id, _)| *id).collect()
        }
    }

    #[test]
    fn clash_controller_wins() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = setup_source(&mut game, alice);

        let alice_top = spell_card(1, "Alice Top", 5);
        let bob_top = spell_card(2, "Bob Top", 1);
        game.create_object_from_card(&alice_top, alice, Zone::Library);
        game.create_object_from_card(&bob_top, bob, Zone::Library);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = ClashEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute clash");
        assert_eq!(outcome.result, EffectResult::Count(1));
    }

    #[test]
    fn clash_controller_loses() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = setup_source(&mut game, alice);

        let alice_top = spell_card(3, "Alice Top", 1);
        let bob_top = spell_card(4, "Bob Top", 6);
        game.create_object_from_card(&alice_top, alice, Zone::Library);
        game.create_object_from_card(&bob_top, bob, Zone::Library);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = ClashEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute clash");
        assert_eq!(outcome.result, EffectResult::Count(0));
    }

    #[test]
    fn clash_players_may_put_revealed_cards_on_bottom() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = setup_source(&mut game, alice);

        let alice_bottom = spell_card(5, "Alice Bottom", 2);
        let alice_revealed = spell_card(6, "Alice Revealed", 5);
        let bob_bottom = spell_card(7, "Bob Bottom", 1);
        let bob_revealed = spell_card(8, "Bob Revealed", 3);
        game.create_object_from_card(&alice_bottom, alice, Zone::Library);
        let alice_revealed_id = game.create_object_from_card(&alice_revealed, alice, Zone::Library);
        game.create_object_from_card(&bob_bottom, bob, Zone::Library);
        let bob_revealed_id = game.create_object_from_card(&bob_revealed, bob, Zone::Library);

        let mut decision_maker = PutRevealedOnBottom;
        let mut ctx = ExecutionContext::new(source, alice, &mut decision_maker);
        let outcome = ClashEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("execute clash");
        assert_eq!(outcome.result, EffectResult::Count(1));

        let alice_top_after = game
            .player(alice)
            .and_then(|player| player.library.last().copied())
            .expect("alice top card after clash");
        let bob_top_after = game
            .player(bob)
            .and_then(|player| player.library.last().copied())
            .expect("bob top card after clash");
        assert_ne!(alice_top_after, alice_revealed_id);
        assert_ne!(bob_top_after, bob_revealed_id);
    }
}

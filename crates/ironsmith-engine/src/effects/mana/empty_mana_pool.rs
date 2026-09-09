//! Empty a player's unspent mana.

use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{
    EffectExecutor, ExecutionContext, ExecutionError, SimultaneousEffectProposal,
};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that makes a player lose all unspent mana.
#[derive(Debug, Clone, PartialEq)]
pub struct EmptyManaPoolEffect {
    pub player: PlayerFilter,
}

impl EmptyManaPoolEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }
}

#[derive(Debug)]
struct EmptyManaPoolProposal {
    player: crate::ids::PlayerId,
}

impl SimultaneousEffectProposal for EmptyManaPoolProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(player) = game.player_mut(self.player) else {
            return Err(ExecutionError::InvalidTarget);
        };
        player.mana_pool.empty();
        player.restricted_mana.clear();
        player.clear_mana_source_provenance();
        Ok(EffectOutcome::default())
    }
}

impl EffectExecutor for EmptyManaPoolEffect {
    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        if game.player(player).is_none() {
            return Err(ExecutionError::InvalidTarget);
        }
        Ok(Box::new(EmptyManaPoolProposal { player }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.prepare_simultaneous_player_action(game, ctx)?
            .commit(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{ManaUsageRestriction, RestrictedManaUnit};
    use crate::ids::{ObjectId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::types::CardType;

    #[test]
    fn empty_mana_pool_removes_unrestricted_and_restricted_mana() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let restricted = RestrictedManaUnit {
            symbol: ManaSymbol::Red,
            source: ObjectId::from_raw(99),
            source_chosen_creature_type: None,
            restrictions: vec![ManaUsageRestriction::CastSpell {
                card_types: vec![CardType::Creature],
                subtype_requirement: None,
                restrict_to_matching_spell: false,
                grant_uncounterable: false,
                enters_with_counters: vec![],
                granted_abilities: vec![],
            }],
        };
        let player = game.player_mut(alice).expect("alice exists");
        player.mana_pool.add(ManaSymbol::Blue, 2);
        player.add_restricted_mana(restricted);

        EmptyManaPoolEffect::new(PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("empty mana pool should resolve");

        let player = game.player(alice).expect("alice exists");
        assert_eq!(player.mana_pool.total(), 0);
        assert!(player.restricted_mana.is_empty());
    }
    #[test]
    fn simultaneous_empty_mana_pool_binds_player_without_mutating_during_prepare() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.player_mut(alice)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Red, 2);
        game.player_mut(bob)
            .unwrap()
            .mana_pool
            .add(ManaSymbol::Blue, 3);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.iteration.iterated_player = Some(alice);
        let proposal = EmptyManaPoolEffect::new(PlayerFilter::IteratedPlayer)
            .prepare_simultaneous_player_action(&game, &mut ctx)
            .unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 2);
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 3);
        ctx.iteration.iterated_player = Some(bob);
        proposal.commit(&mut game, &mut ctx).unwrap();
        assert_eq!(game.player(alice).unwrap().mana_pool.total(), 0);
        assert_eq!(game.player(bob).unwrap().mana_pool.total(), 3);
    }
}

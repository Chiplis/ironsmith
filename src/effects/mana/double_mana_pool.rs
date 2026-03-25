//! Double a player's unspent mana.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::target::PlayerFilter;

/// Effect that doubles each type of unspent mana a player has.
///
/// Restricted mana is cloned as restricted mana so its spending permissions and
/// any source-linked bonuses are preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct DoubleManaPoolEffect {
    /// Which player's mana pool to double.
    pub player: PlayerFilter,
}

impl DoubleManaPoolEffect {
    /// Create a new mana-pool doubling effect.
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    /// Double your mana pool.
    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for DoubleManaPoolEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let (unrestricted_symbols, restricted_units) = {
            let Some(player) = game.player(player_id) else {
                return Err(ExecutionError::InvalidTarget);
            };

            let restricted_units = player.restricted_mana.clone();
            let unrestricted_symbols = [
                ManaSymbol::White,
                ManaSymbol::Blue,
                ManaSymbol::Black,
                ManaSymbol::Red,
                ManaSymbol::Green,
                ManaSymbol::Colorless,
            ]
            .into_iter()
            .flat_map(|symbol| {
                let total = player.mana_pool.amount(symbol);
                let restricted = restricted_units
                    .iter()
                    .filter(|unit| unit.symbol == symbol)
                    .count() as u32;
                std::iter::repeat_n(symbol, total.saturating_sub(restricted) as usize)
            })
            .collect::<Vec<_>>();

            (unrestricted_symbols, restricted_units)
        };

        let mut added = unrestricted_symbols.clone();
        added.extend(restricted_units.iter().map(|unit| unit.symbol));

        let Some(player) = game.player_mut(player_id) else {
            return Err(ExecutionError::InvalidTarget);
        };
        for symbol in &unrestricted_symbols {
            player.mana_pool.add(*symbol, 1);
        }
        for unit in restricted_units {
            player.add_restricted_mana(unit);
        }

        Ok(EffectOutcome::mana_added(added))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{ManaUsageRestriction, RestrictedManaUnit};
    use crate::ids::{ObjectId, PlayerId};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn double_mana_pool_doubles_each_unrestricted_type() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let player = game.player_mut(alice).expect("alice exists");
        player.mana_pool.add(ManaSymbol::White, 2);
        player.mana_pool.add(ManaSymbol::Red, 1);
        player.mana_pool.add(ManaSymbol::Colorless, 3);

        let outcome = DoubleManaPoolEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("double mana pool should resolve");

        let player = game.player(alice).expect("alice exists");
        assert_eq!(player.mana_pool.white, 4);
        assert_eq!(player.mana_pool.red, 2);
        assert_eq!(player.mana_pool.colorless, 6);
        assert_eq!(
            outcome.value,
            crate::effect::OutcomeValue::ManaAdded(vec![
                ManaSymbol::White,
                ManaSymbol::White,
                ManaSymbol::Red,
                ManaSymbol::Colorless,
                ManaSymbol::Colorless,
                ManaSymbol::Colorless,
            ])
        );
    }

    #[test]
    fn double_mana_pool_duplicates_restricted_mana_units() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let mana_source = ObjectId::from_raw(777);
        let restricted = RestrictedManaUnit {
            symbol: ManaSymbol::Red,
            source: mana_source,
            source_chosen_creature_type: None,
            restrictions: vec![ManaUsageRestriction::CastSpell {
                card_types: vec![CardType::Creature],
                subtype_requirement: None,
                grant_uncounterable: true,
            }],
        };

        let player = game.player_mut(alice).expect("alice exists");
        player.mana_pool.add(ManaSymbol::Red, 1);
        player.add_restricted_mana(restricted.clone());

        DoubleManaPoolEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("double mana pool should resolve");

        let player = game.player(alice).expect("alice exists");
        assert_eq!(player.mana_pool.red, 4);
        assert_eq!(player.restricted_mana.len(), 2);
        assert!(
            player.restricted_mana.iter().all(|unit| unit == &restricted),
            "expected restricted mana copy to preserve restriction metadata"
        );
    }
}

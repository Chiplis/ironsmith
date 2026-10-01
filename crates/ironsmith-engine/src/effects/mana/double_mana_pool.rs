//! Double a player's unspent mana.

use super::choice_helpers::{credit_mana_symbols_from_context, mana_added_value_outcome};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::mana::ManaSymbol;
use crate::target::PlayerFilter;

/// Effect that doubles each type of unspent mana a player has.
///
/// CR 701.10f / 106.6: add new mana of each type. Restrictions, bonuses,
/// and provenance of the old mana are not properties of its type.
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
    fn directly_produces_mana(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let player = game.player(player_id).ok_or(ExecutionError::InvalidTarget)?;
        let symbols = [
            ManaSymbol::White, ManaSymbol::Blue, ManaSymbol::Black,
            ManaSymbol::Red, ManaSymbol::Green, ManaSymbol::Colorless,
        ].into_iter().flat_map(|symbol| {
            std::iter::repeat_n(symbol, player.mana_pool.amount(symbol) as usize)
        }).collect::<Vec<_>>();
        let added = credit_mana_symbols_from_context(game, player_id, symbols, ctx)?;

        Ok(mana_added_value_outcome(ctx, player_id, added))
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
    fn double_mana_pool_adds_new_mana_without_copying_restrictions_or_source() {
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
                restrict_to_matching_spell: true,
                grant_uncounterable: true,
                enters_with_counters: vec![],
                granted_abilities: vec![],
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
        assert_eq!(player.restricted_mana, vec![restricted]);
        assert_eq!(player.mana_source_provenance.iter().filter(|unit| unit.source == source).count(), 2);
        assert_eq!(player.mana_source_provenance.iter().filter(|unit| unit.source == mana_source).count(), 1);
        assert!(player.mana_source_provenance.iter().filter(|unit| unit.source == source).all(|unit| !unit.restricted));
    }
}

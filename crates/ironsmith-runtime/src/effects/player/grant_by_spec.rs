//! Unified filter-based grant effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::{GrantDuration, GrantSpec};
use crate::grant_registry::GrantSource;
use crate::target::PlayerFilter;

/// Effect that grants a [`GrantSpec`] to cards matching its filter for a duration.
#[derive(Debug, Clone, PartialEq)]
pub struct GrantBySpecEffect {
    /// Shared grant definition used by static abilities and temporary effects.
    pub spec: GrantSpec,
    /// Player who may use the grant.
    pub player: PlayerFilter,
    /// How long the grant lasts.
    pub duration: GrantDuration,
}

impl GrantBySpecEffect {
    /// Create a new filter-based grant effect.
    pub fn new(spec: GrantSpec, player: PlayerFilter, duration: GrantDuration) -> Self {
        Self {
            spec,
            player,
            duration,
        }
    }
}

impl EffectExecutor for GrantBySpecEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let grant_source = match self.duration {
            GrantDuration::UntilEndOfTurn => {
                GrantSource::until_end_of_turn(ctx.source, game.turn.turn_number)
            }
            GrantDuration::Forever => GrantSource::Effect {
                source_id: ctx.source,
                expires_end_of_turn: u32::MAX,
            },
        };

        game.effect_store.grant_registry.grant_to_filter(
            self.spec.filter.clone(),
            self.spec.zone,
            player_id,
            self.spec.grantable.clone(),
            grant_source,
        );

        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_grant_flash_to_spells_in_hand_until_eot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let sorcery = CardBuilder::new(CardId::from_raw(1), "Test Sorcery")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let land = CardBuilder::new(CardId::from_raw(2), "Test Land")
            .card_types(vec![CardType::Land])
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);
        let land_id = game.create_object_from_card(&land, alice, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = GrantBySpecEffect::new(
            GrantSpec::flash_to_spells(),
            PlayerFilter::You,
            GrantDuration::UntilEndOfTurn,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);

        let flash = StaticAbility::flash();
        assert!(game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            sorcery_id,
            Zone::Hand,
            alice,
            &flash,
        ));
        assert!(!game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            land_id,
            Zone::Hand,
            alice,
            &flash,
        ));
    }
}

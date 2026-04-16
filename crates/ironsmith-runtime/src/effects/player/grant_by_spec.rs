//! Unified filter-based grant effect implementation.

use crate::continuous::{EffectSourceType, EffectTarget, Modification};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::grant::{GrantDuration, GrantSpec, Grantable};
use crate::grant_registry::GrantSource;
use crate::zone::Zone;
pub type GrantBySpecEffect = ironsmith_core::GrantBySpecEffect<GrantSpec, GrantDuration>;

/// Effect that grants a [`GrantSpec`] to cards matching its filter for a duration.
impl EffectExecutor for GrantBySpecEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        if self.spec.zone == Zone::Battlefield
            && let Grantable::Ability(ability) = &self.spec.grantable
        {
            let filter_ctx = game.filter_context_for(player_id, Some(ctx.source));
            let locked_targets: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|&id| game.object(id))
                .filter(|obj| self.spec.filter.matches(obj, &filter_ctx, game))
                .map(|obj| obj.id)
                .collect();
            let duration = match self.duration {
                GrantDuration::UntilEndOfTurn => crate::effect::Until::EndOfTurn,
                GrantDuration::Forever => crate::effect::Until::Forever,
                GrantDuration::UntilYourNextTurnEnd => {
                    return Err(ExecutionError::Impossible(
                        "grant duration until your next turn is not implemented".to_string(),
                    ));
                }
            };
            let effect = crate::effects::ApplyContinuousEffect::new(
                EffectTarget::Filter(self.spec.filter.clone()),
                Modification::AddAbility(ability.clone()),
                duration,
            )
            .with_source_type(EffectSourceType::Resolution { locked_targets });
            return effect.execute(game, ctx);
        }

        let grant_source = match self.duration {
            GrantDuration::UntilEndOfTurn => {
                GrantSource::until_end_of_turn(ctx.source, game.turn.turn_number)
            }
            GrantDuration::Forever => GrantSource::Effect {
                source_id: ctx.source,
                expires_end_of_turn: u32::MAX,
            },
            GrantDuration::UntilYourNextTurnEnd => {
                return Err(ExecutionError::Impossible(
                    "grant duration until your next turn is not implemented".to_string(),
                ));
            }
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
    use crate::target::PlayerFilter;
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

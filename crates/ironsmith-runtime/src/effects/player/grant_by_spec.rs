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
use crate::ids::PlayerId;
use crate::zone::Zone;
pub type GrantBySpecEffect = ironsmith_core::GrantBySpecEffect<GrantSpec, GrantDuration>;

fn next_turn_number_for_player(game: &GameState, player: PlayerId) -> u32 {
    if game.turn_store.turn_order.is_empty() {
        return game.turn.turn_number;
    }

    let mut simulated_active_player = game.turn.active_player;
    let mut simulated_turn_number = game.turn.turn_number;
    let mut simulated_extra_turns = game.turn_store.extra_turns.clone();
    let mut simulated_skip_next_turn = game.turn_store.skip_next_turn.clone();
    let max_iterations = game
        .turn_store
        .turn_order
        .len()
        .saturating_mul(16)
        .saturating_add(simulated_extra_turns.len().saturating_mul(2))
        .saturating_add(16)
        .max(1);

    for _ in 0..max_iterations {
        let next_player = if !simulated_extra_turns.is_empty() {
            simulated_extra_turns.remove(0)
        } else {
            let current_index = game
                .turn_store
                .turn_order
                .iter()
                .position(|&p| p == simulated_active_player)
                .unwrap_or(0);

            let mut next_index = (current_index + 1) % game.turn_store.turn_order.len();
            let start_index = next_index;

            loop {
                let candidate = game.turn_store.turn_order[next_index];
                let is_in_game = game.player(candidate).is_some_and(|p| p.is_in_game());

                if is_in_game {
                    if simulated_skip_next_turn.remove(&candidate) {
                        next_index = (next_index + 1) % game.turn_store.turn_order.len();
                        if next_index == start_index {
                            break;
                        }
                        continue;
                    }
                    break;
                }

                next_index = (next_index + 1) % game.turn_store.turn_order.len();
                if next_index == start_index {
                    break;
                }
            }

            game.turn_store.turn_order[next_index]
        };

        simulated_turn_number = simulated_turn_number.saturating_add(1);
        simulated_active_player = next_player;
        if simulated_active_player == player {
            return simulated_turn_number;
        }
    }

    game.turn.turn_number.saturating_add(1)
}

fn grant_duration_source(
    duration: GrantDuration,
    game: &GameState,
    source: crate::ids::ObjectId,
    player: PlayerId,
) -> GrantSource {
    match duration {
        GrantDuration::UntilEndOfTurn => {
            GrantSource::until_end_of_turn(source, game.turn.turn_number)
        }
        GrantDuration::Forever => GrantSource::Effect {
            source_id: source,
            expires_end_of_turn: u32::MAX,
        },
        GrantDuration::UntilYourNextTurnEnd => GrantSource::Effect {
            source_id: source,
            expires_end_of_turn: next_turn_number_for_player(game, player),
        },
    }
}

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
                GrantDuration::UntilYourNextTurnEnd => crate::effect::Until::YourNextTurn,
            };
            let effect = crate::effects::ApplyContinuousEffect::new(
                EffectTarget::Filter(self.spec.filter.clone()),
                Modification::AddAbility(ability.clone()),
                duration,
            )
            .with_source_type(EffectSourceType::Resolution { locked_targets });
            return effect.execute(game, ctx);
        }

        let grant_source = grant_duration_source(self.duration, game, ctx.source, player_id);

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
    use crate::target::{ObjectFilter, PlayerFilter};
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

    #[test]
    fn test_grant_flash_to_spells_in_hand_until_next_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let sorcery = CardBuilder::new(CardId::from_raw(3), "Test Sorcery")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Sorcery])
            .build();
        let sorcery_id = game.create_object_from_card(&sorcery, alice, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = GrantBySpecEffect::new(
            GrantSpec::flash_to_spells_matching(ObjectFilter {
                card_types: vec![CardType::Sorcery],
                ..ObjectFilter::default()
            }),
            PlayerFilter::You,
            GrantDuration::UntilYourNextTurnEnd,
        );
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.grant_registry.grants.len(), 1);
        assert_eq!(
            game.effect_store.grant_registry.grants[0].source,
            GrantSource::Effect {
                source_id: source,
                expires_end_of_turn: 3,
            }
        );

        let flash = StaticAbility::flash();
        assert!(game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            sorcery_id,
            Zone::Hand,
            alice,
            &flash,
        ));

        game.turn.turn_number = 4;
        assert!(!game.effect_store.grant_registry.card_has_granted_ability(
            &game,
            sorcery_id,
            Zone::Hand,
            alice,
            &flash,
        ));
    }
}

//! Exile a chosen object, then grant permission to cast or play it from exile.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_single_object_for_effect};
use crate::effects::player::grant_by_spec::next_turn_number_for_player;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::grant::{GrantDuration, Grantable};
use crate::grant_registry::GrantSource;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExileThenGrantPlayEffect {
    pub target: ChooseSpec,
    pub player: PlayerFilter,
    pub duration: GrantDuration,
    pub available_starting_next_turn: bool,
}

impl ExileThenGrantPlayEffect {
    pub fn new(target: ChooseSpec, player: PlayerFilter, duration: GrantDuration) -> Self {
        Self {
            target,
            player,
            duration,
            available_starting_next_turn: false,
        }
    }

    pub fn starting_next_turn(mut self) -> Self {
        self.available_starting_next_turn = true;
        self
    }
}

impl EffectExecutor for ExileThenGrantPlayEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let expires = match self.duration {
            GrantDuration::UntilEndOfTurn => game.turn.turn_number,
            GrantDuration::Forever => u32::MAX,
            GrantDuration::UntilYourNextTurnEnd => next_turn_number_for_player(game, player),
        };
        let additional_effects = ctx.additional_replacement_effects_snapshot();

        let outcome = crate::effects::zones::apply_zone_change_with_additional_effects(
            game,
            target_id,
            game.object(target_id)
                .map(|obj| obj.zone)
                .ok_or(ExecutionError::ObjectNotFound(target_id))?,
            Zone::Exile,
            crate::events::cause::EventCause::from_effect(ctx.source, ctx.controller),
            ctx.decision_maker,
            &additional_effects,
        );

        let crate::events::processing::EventOutcome::Proceed(result) = outcome else {
            return Ok(EffectOutcome::count(0));
        };
        if result.final_zone != Zone::Exile {
            return Ok(EffectOutcome::count(0));
        }
        if result.new_object_ids.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        for &exiled_id in &result.new_object_ids {
            let grant_source = match self.duration {
                GrantDuration::UntilYourNextTurnEnd => {
                    GrantSource::until_player_next_turn_end(ctx.source, player, expires)
                }
                GrantDuration::UntilEndOfTurn | GrantDuration::Forever => GrantSource::Effect {
                    source_id: ctx.source,
                    expires_end_of_turn: expires,
                },
            };
            if self.available_starting_next_turn {
                game.effect_store
                    .grant_registry
                    .grant_to_card_starting_on_turn(
                        exiled_id,
                        Zone::Exile,
                        player,
                        Grantable::PlayFrom,
                        game.turn.turn_number.saturating_add(1),
                        grant_source,
                    );
            } else {
                game.effect_store.grant_registry.grant_to_card(
                    exiled_id,
                    Zone::Exile,
                    player,
                    Grantable::PlayFrom,
                    grant_source,
                );
            }
        }

        Ok(EffectOutcome::with_objects(result.new_object_ids))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "object to exile and grant play permission to"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::filter::ObjectFilter;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;

    #[test]
    fn exiled_card_can_be_played_through_the_players_next_turn() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(1), "Next-turn Exile")
            .card_types(vec![CardType::Sorcery])
            .build();
        let hand_id = game.create_object_from_card(&card, alice, Zone::Hand);

        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.targets = vec![crate::effects::ResolvedTarget::Object(hand_id)];
        ExileThenGrantPlayEffect::new(
            ChooseSpec::Object(ObjectFilter::default().in_zone(Zone::Hand)),
            PlayerFilter::You,
            GrantDuration::UntilYourNextTurnEnd,
        )
        .execute(&mut game, &mut ctx)
        .expect("the next-turn duration must be executable");

        let exiled_id = *game.exile.last().expect("the card should be exiled");
        let grants = game.effect_store.grant_registry.get_grants_for_card(
            &game,
            exiled_id,
            Zone::Exile,
            alice,
        );
        assert_eq!(grants.len(), 1);
        assert_eq!(
            grants[0].source,
            GrantSource::until_player_next_turn_end(source, alice, 3)
        );

        game.turn.turn_number = 3;
        assert_eq!(
            game.effect_store
                .grant_registry
                .get_grants_for_card(&game, exiled_id, Zone::Exile, alice,)
                .len(),
            1,
        );
        game.turn.turn_number = 4;
        assert!(
            game.effect_store
                .grant_registry
                .get_grants_for_card(&game, exiled_id, Zone::Exile, alice,)
                .is_empty()
        );
    }
}

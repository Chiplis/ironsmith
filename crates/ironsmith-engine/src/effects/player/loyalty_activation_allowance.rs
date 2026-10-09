//! Relax the loyalty-ability activation rule for the rest of the turn
//! (CR 606.3: once per turn per permanent, at sorcery speed). The allowances
//! live in named turn counters, so they expire with the turn.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::types::{CardType, Subtype};

pub use ironsmith_core::{
    GrantLoyaltyActivationAllowanceEffect, LoyaltyActivationAllowance, LoyaltyActivationScope,
};

fn allowance_name(allowance: LoyaltyActivationAllowance) -> &'static str {
    match allowance {
        LoyaltyActivationAllowance::ExtraActivation => "extra",
        LoyaltyActivationAllowance::InstantSpeed => "instant",
    }
}

fn object_counter(allowance: LoyaltyActivationAllowance, object: ObjectId) -> String {
    format!("loyalty_allowance:{}:object:{}", allowance_name(allowance), object.0)
}

fn player_counter(
    allowance: LoyaltyActivationAllowance,
    player: PlayerId,
    subtype: Option<Subtype>,
) -> String {
    let subtype = subtype.map_or_else(|| "any".to_string(), |subtype| format!("{subtype:?}"));
    format!(
        "loyalty_allowance:{}:player:{}:{subtype}",
        allowance_name(allowance),
        player.0
    )
}

/// The named turn counter that counts this permanent's loyalty activations.
pub(crate) fn loyalty_activation_counter(object: ObjectId) -> String {
    format!("loyalty_activation:{}", object.0)
}

/// How many allowances of this kind cover `source`, controlled by
/// `controller`, this turn.
pub(crate) fn loyalty_allowance_count(
    game: &GameState,
    controller: PlayerId,
    source: ObjectId,
    allowance: LoyaltyActivationAllowance,
) -> u32 {
    let mut count = game.named_turn_counter(&object_counter(allowance, source));
    count += game.named_turn_counter(&player_counter(allowance, controller, None));
    for subtype in game.calculated_subtypes(source) {
        count += game.named_turn_counter(&player_counter(allowance, controller, Some(subtype)));
    }
    count
}

impl EffectExecutor for GrantLoyaltyActivationAllowanceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_atomically(game, ctx, |game, ctx| {
            match &self.scope {
                LoyaltyActivationScope::Source => {
                    let source = crate::effects::helpers::resolve_source_object_id(game, ctx)
                        .unwrap_or(ctx.source);
                    game.increment_named_turn_counter(object_counter(self.allowance, source));
                }
                LoyaltyActivationScope::EachControlledPlaneswalkerNow => {
                    let planeswalkers = game
                        .battlefield
                        .iter()
                        .copied()
                        .filter(|id| {
                            game.object(*id)
                                .is_some_and(|object| game.controller_of(object) == ctx.controller)
                                && game.object_has_card_type(*id, CardType::Planeswalker)
                        })
                        .collect::<Vec<_>>();
                    for planeswalker in planeswalkers {
                        game.increment_named_turn_counter(object_counter(
                            self.allowance,
                            planeswalker,
                        ));
                    }
                }
                LoyaltyActivationScope::ControlledPlaneswalkers { subtype } => {
                    game.increment_named_turn_counter(player_counter(
                        self.allowance,
                        ctx.controller,
                        *subtype,
                    ));
                }
            }
            Ok(EffectOutcome::resolved())
        })
    }
}

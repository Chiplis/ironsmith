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
    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        match &self.scope {
            LoyaltyActivationScope::Objects(target) => Some(target),
            _ => None,
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::tokens::execute_resource_transaction_atomically(game, ctx, |game, ctx| {
            match &self.scope {
                LoyaltyActivationScope::Objects(target) => {
                    let objects=crate::effects::helpers::resolve_objects_for_effect(game, ctx, target)?;
                    for object in &objects {
                        game.increment_named_turn_counter(object_counter(self.allowance, *object));
                    }
                    return Ok(EffectOutcome::with_objects(objects));
                }
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

#[cfg(test)]
mod targeted_allowance_tests {
    use super::*;
    #[test]
    fn ordinary_object_operand_grants_only_its_exact_recipient_incarnation(){
        let mut game=crate::tests::test_helpers::setup_two_player_game();let player=PlayerId(0);
        let definition=crate::cards::builders::CardDefinitionBuilder::new(crate::ids::CardId::new(),"Allowance recipient").card_types(vec![CardType::Planeswalker]).loyalty(3).build();
        let first=game.create_object_from_definition(&definition,player,crate::zone::Zone::Battlefield);
        let other=game.create_object_from_definition(&definition,player,crate::zone::Zone::Battlefield);
        let grant=GrantLoyaltyActivationAllowanceEffect::new(LoyaltyActivationScope::Objects(crate::target::ChooseSpec::SpecificObject(first)),LoyaltyActivationAllowance::ExtraActivation);
        let outcome=grant.execute(&mut game,&mut ExecutionContext::new_default(other,player)).unwrap();
        assert_eq!(outcome.explicit_objects(),Some(&[first][..]));
        assert_eq!(loyalty_allowance_count(&game,player,first,LoyaltyActivationAllowance::ExtraActivation),1);
        assert_eq!(loyalty_allowance_count(&game,player,other,LoyaltyActivationAllowance::ExtraActivation),0);
        let departed=game.move_object_by_effect(first,crate::zone::Zone::Graveyard).unwrap();
        let returned=game.move_object_by_effect(departed,crate::zone::Zone::Battlefield).unwrap();
        assert_eq!(loyalty_allowance_count(&game,player,returned,LoyaltyActivationAllowance::ExtraActivation),0);
    }
}

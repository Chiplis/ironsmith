//! Exchange whole-zone contents effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct ExchangeZonesEffect {
    pub player: PlayerFilter,
    pub zone1: Zone,
    pub zone2: Zone,
}

impl ExchangeZonesEffect {
    pub fn new(player: PlayerFilter, zone1: Zone, zone2: Zone) -> Self {
        Self {
            player,
            zone1,
            zone2,
        }
    }

    fn zone_objects(
        game: &GameState,
        player: crate::ids::PlayerId,
        zone: Zone,
    ) -> Vec<crate::ids::ObjectId> {
        let Some(player_state) = game.player(player) else {
            return Vec::new();
        };
        match zone {
            Zone::Hand => player_state.hand.clone(),
            Zone::Library => player_state.library.clone(),
            Zone::Graveyard => player_state.graveyard.clone(),
            _ => Vec::new(),
        }
    }

    fn supported_zone(zone: Zone) -> bool {
        matches!(zone, Zone::Hand | Zone::Library | Zone::Graveyard)
    }
}

impl EffectExecutor for ExchangeZonesEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.zone1 == self.zone2 {
            return Ok(EffectOutcome::resolved());
        }
        if !Self::supported_zone(self.zone1) || !Self::supported_zone(self.zone2) {
            return Ok(EffectOutcome::impossible());
        }

        let player = match resolve_player_filter(game, &self.player, ctx) {
            Ok(player) => player,
            Err(ExecutionError::InvalidTarget) => return Ok(EffectOutcome::target_invalid()),
            Err(err) => return Err(err),
        };

        let from_zone1 = Self::zone_objects(game, player, self.zone1);
        let from_zone2 = Self::zone_objects(game, player, self.zone2);
        let mut moved = Vec::new();

        for object_id in from_zone1 {
            let Some(new_id) = game.move_object_by_effect(object_id, self.zone2) else {
                return Ok(EffectOutcome::prevented());
            };
            moved.push(new_id);
        }
        for object_id in from_zone2 {
            let Some(new_id) = game.move_object_by_effect(object_id, self.zone1) else {
                return Ok(EffectOutcome::prevented());
            };
            moved.push(new_id);
        }

        Ok(EffectOutcome::with_objects(moved))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::object::Object;

    fn add_card(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        zone: Zone,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name).build();
        let object = Object::from_card(id, &card, owner, zone);
        game.add_object(object);
        id
    }

    #[test]
    fn exchange_zones_swaps_hand_and_graveyard_contents() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let hand_card = add_card(&mut game, alice, "Hand Card", Zone::Hand);
        let graveyard_card = add_card(&mut game, alice, "Graveyard Card", Zone::Graveyard);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ExchangeZonesEffect::new(PlayerFilter::You, Zone::Hand, Zone::Graveyard)
            .execute(&mut game, &mut ctx)
            .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert!(game.object(hand_card).is_none());
        assert!(game.object(graveyard_card).is_none());
        let alice_state = game.player(alice).expect("alice exists");
        assert_eq!(alice_state.hand.len(), 1);
        assert_eq!(alice_state.graveyard.len(), 1);
        let new_hand = alice_state.hand[0];
        let new_graveyard = alice_state.graveyard[0];
        assert_eq!(
            game.object(new_hand).expect("new hand object").name,
            "Graveyard Card"
        );
        assert_eq!(
            game.object(new_graveyard)
                .expect("new graveyard object")
                .name,
            "Hand Card"
        );
    }

    #[test]
    fn exchange_zones_handles_empty_zone() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        add_card(&mut game, alice, "Only Card", Zone::Graveyard);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        let outcome = ExchangeZonesEffect::new(PlayerFilter::You, Zone::Hand, Zone::Graveyard)
            .execute(&mut game, &mut ctx)
            .expect("exchange should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        let alice_state = game.player(alice).expect("alice exists");
        assert_eq!(alice_state.hand.len(), 1);
        assert!(alice_state.graveyard.is_empty());
        let new_hand = alice_state.hand[0];
        assert_eq!(
            game.object(new_hand).expect("new hand object").name,
            "Only Card"
        );
    }
}

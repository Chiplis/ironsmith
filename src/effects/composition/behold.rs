//! Behold mechanic effect.
//!
//! This is a custom mechanic used in some card sets in this repository.
//!
//! Reminder text example:
//! "To behold an Elemental, choose an Elemental you control or reveal an Elemental card from your hand."

use crate::effect::EffectOutcome;
use crate::effects::helpers::normalize_object_selection;
use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;
use crate::types::Subtype;

/// Effect that "beholds" one or more objects of a given subtype.
///
/// For each behold, the player chooses a matching object they control on the battlefield
/// or reveals a matching card from their hand.
///
/// The engine does not model hidden information, so "reveal" is a no-op other than
/// validating that the chosen card exists in hand.
#[derive(Debug, Clone, PartialEq)]
pub struct BeholdEffect {
    pub subtype: Subtype,
    pub count: u32,
    pub chooser: PlayerFilter,
}

impl BeholdEffect {
    pub fn new(subtype: Subtype, count: u32, chooser: PlayerFilter) -> Self {
        Self {
            subtype,
            count,
            chooser,
        }
    }

    pub fn you(subtype: Subtype, count: u32) -> Self {
        Self::new(subtype, count, PlayerFilter::You)
    }
}

fn candidates(
    game: &GameState,
    chooser: PlayerId,
    source: ObjectId,
    subtype: Subtype,
) -> Vec<ObjectId> {
    let mut out = Vec::new();

    out.extend(
        game.battlefield
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| (id, obj)))
            .filter(|(id, _)| {
                game.current_controller(*id) == Some(chooser)
                    && game.current_has_subtype(*id, subtype)
            })
            .map(|(id, _)| id),
    );

    if let Some(player) = game.player(chooser) {
        out.extend(
            player
                .hand
                .iter()
                .copied()
                .filter(|id| *id != source)
                .filter_map(|id| game.object(id).map(|obj| (id, obj)))
                .filter(|(id, _)| game.current_has_subtype(*id, subtype))
                .map(|(id, _)| id),
        );
    }

    out
}

impl EffectExecutor for BeholdEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::decisions::context::ViewCardsContext;
        use crate::decisions::make_decision;
        use crate::decisions::specs::ChooseObjectsSpec;
        use crate::effects::helpers::resolve_player_filter;
        use crate::zone::Zone;

        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let required = self.count as usize;
        if required == 0 {
            return Ok(EffectOutcome::resolved());
        }

        let pool = candidates(game, chooser, ctx.source, self.subtype);
        if pool.len() < required {
            return Err(ExecutionError::Impossible(format!(
                "Not enough objects to behold ({} needed, {} available)",
                required,
                pool.len()
            )));
        }

        let chosen = if pool.len() == required {
            pool.clone()
        } else {
            let subtype_name = self.subtype.to_string().to_ascii_lowercase();
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                format!("Choose {} {} to behold", required, subtype_name),
                pool.clone(),
                required,
                Some(required),
            );
            make_decision(game, ctx.decision_maker, chooser, Some(ctx.source), spec)
        };
        let chosen = normalize_object_selection(chosen, &pool, required);

        let revealed_from_hand: Vec<_> = chosen
            .iter()
            .copied()
            .filter(|id| {
                game.player(chooser)
                    .is_some_and(|player| player.hand.contains(id))
            })
            .collect();
        if !revealed_from_hand.is_empty() {
            for viewer_idx in 0..game.players.len() {
                let viewer = PlayerId::from_index(viewer_idx as u8);
                let view_ctx = ViewCardsContext::new(
                    viewer,
                    chooser,
                    Some(ctx.source),
                    Zone::Hand,
                    "Reveal cards from hand",
                )
                .with_public(true);
                ctx.decision_maker
                    .view_cards(game, viewer, &revealed_from_hand, &view_ctx);
            }
        }

        Ok(EffectOutcome::with_objects(chosen))
    }

    fn cost_description(&self) -> Option<String> {
        let subtype_name = self.subtype.to_string().to_ascii_lowercase();
        if self.count == 1 {
            return Some(format!("Behold a {}", subtype_name));
        }
        Some(format!("Behold {} {}s", self.count, subtype_name))
    }
}

impl CostExecutableEffect for BeholdEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        use crate::effects::CostValidationError;

        let chooser = match self.chooser {
            PlayerFilter::You => controller,
            PlayerFilter::Specific(id) => id,
            _ => controller,
        };

        let available = candidates(game, chooser, source, self.subtype).len() as u32;
        if available < self.count {
            return Err(CostValidationError::Other(format!(
                "Not enough {}s to behold ({} needed, {} available)",
                self.subtype.to_string().to_ascii_lowercase(),
                self.count,
                available
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn simple_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        subtype: Subtype,
        zone: Zone,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(vec![CardType::Creature])
            .subtypes(vec![subtype])
            .build();
        game.create_object_from_card(&card, controller, zone)
    }

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<ObjectId>)>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }
    }

    #[test]
    fn test_behold_validates_candidates_across_battlefield_and_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        // One Elemental on battlefield and one in hand.
        let _bf = simple_creature(
            &mut game,
            "BF Elemental",
            alice,
            Subtype::Elemental,
            Zone::Battlefield,
        );
        let _hand = simple_creature(
            &mut game,
            "Hand Elemental",
            alice,
            Subtype::Elemental,
            Zone::Hand,
        );

        let effect = BeholdEffect::you(Subtype::Elemental, 2);
        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice)
                .is_ok()
        );
    }

    #[test]
    fn test_behold_errors_when_insufficient_candidates() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let _hand = simple_creature(
            &mut game,
            "Hand Elemental",
            alice,
            Subtype::Elemental,
            Zone::Hand,
        );

        let effect = BeholdEffect::you(Subtype::Elemental, 2);
        assert!(
            crate::effects::EffectExecutor::can_execute_as_cost(&effect, &game, source, alice)
                .is_err()
        );
    }

    #[test]
    fn test_behold_reveals_hand_cards_publicly_when_chosen() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let hand = simple_creature(&mut game, "Hand Dragon", alice, Subtype::Dragon, Zone::Hand);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![crate::executor::ResolvedTarget::Object(hand)]);

        BeholdEffect::you(Subtype::Dragon, 1)
            .execute(&mut game, &mut ctx)
            .expect("behold from hand should execute");

        assert_eq!(
            dm.calls.len(),
            2,
            "all players should see the revealed hand card"
        );
        assert!(dm.calls.iter().all(|(_, subject, zone, public, cards)| {
            *subject == alice && *zone == Zone::Hand && *public && cards == &vec![hand]
        }));
    }
}

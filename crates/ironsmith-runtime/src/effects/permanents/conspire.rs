//! Conspire cost implementation.
//!
//! Comprehensive Rules reference (current TXT discovered on 2026-04-08):
//! - 702.78a: Conspire is an optional additional cost to cast a spell and a
//!   cast trigger that copies the spell if that cost was paid.
//! - 702.78b: Multiple instances are paid separately and trigger separately.
//!
//! This module handles the additional-cost half:
//! - choose exactly two untapped creatures you control
//! - each must share a color with the spell being cast
//! - tap them as part of paying the cost

use crate::color::ColorSet;
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::events::PermanentTappedEvent;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::triggers::TriggerEvent;

#[derive(Debug, Clone, PartialEq)]
pub struct ConspireCostEffect;

impl ConspireCostEffect {
    pub fn new() -> Self {
        Self
    }

    fn source_colors(game: &GameState, source: ObjectId) -> ColorSet {
        game.current_colors(source)
            .or_else(|| game.object(source).map(|obj| obj.colors()))
            .unwrap_or_default()
    }

    fn conspire_candidates(
        game: &GameState,
        controller: PlayerId,
        source: ObjectId,
    ) -> Vec<ObjectId> {
        let source_colors = Self::source_colors(game, source);
        if source_colors.is_empty() {
            return Vec::new();
        }

        game.battlefield
            .iter()
            .copied()
            .filter(|&id| {
                let Some(obj) = game.object(id) else {
                    return false;
                };
                if !game.current_is_creature(id)
                    || obj.controller != controller
                    || game.is_tapped(id)
                {
                    return false;
                }

                let creature_colors = game.current_colors(id).unwrap_or_else(|| obj.colors());
                !source_colors.intersection(creature_colors).is_empty()
            })
            .collect()
    }
}

impl EffectExecutor for ConspireCostEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let controller = ctx.controller;
        let source = ctx.source;
        let candidates = Self::conspire_candidates(game, controller, source);
        if candidates.len() < 2 {
            return Err(ExecutionError::Impossible(
                "Not enough untapped creatures that share a color with this spell".to_string(),
            ));
        }

        let spec = ChooseObjectsSpec::new(
            source,
            "Choose two untapped creatures to conspire",
            candidates.clone(),
            2,
            Some(2),
        );
        let mut chosen = make_decision(game, ctx.decision_maker, controller, Some(source), spec);
        chosen.sort();
        chosen.dedup();

        if chosen.len() < 2 {
            for candidate in candidates {
                if chosen.len() >= 2 {
                    break;
                }
                if !chosen.contains(&candidate) {
                    chosen.push(candidate);
                }
            }
        }

        if chosen.len() != 2 {
            return Err(ExecutionError::Impossible(
                "Conspire requires exactly two untapped creatures".to_string(),
            ));
        }

        let mut events = Vec::new();
        for id in chosen {
            if game.object(id).is_some() && !game.is_tapped(id) {
                game.tap(id);
                events.push(TriggerEvent::new_with_provenance(
                    PermanentTappedEvent::new(id),
                    ctx.provenance,
                ));
            }
        }

        Ok(EffectOutcome::resolved().with_events(events))
    }

    fn cost_description(&self) -> Option<String> {
        Some(
            "Tap two untapped creatures you control that each share a color with this spell"
                .to_string(),
        )
    }
}

impl CostExecutableEffect for ConspireCostEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if Self::conspire_candidates(game, controller, source).len() >= 2 {
            Ok(())
        } else {
            Err(CostValidationError::Other(
                "Not enough untapped creatures that share a color with this spell".to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::tests::test_helpers::setup_two_player_game;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn make_creature(
        game: &mut GameState,
        controller: PlayerId,
        name: &str,
        colors: ColorSet,
    ) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::PowerToughness::fixed(2, 2))
            .color_indicator(colors)
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn make_spell(game: &mut GameState, controller: PlayerId, colors: ColorSet) -> ObjectId {
        let spell = CardBuilder::new(CardId::new(), "Conspire Test")
            .card_types(vec![CardType::Sorcery])
            .color_indicator(colors)
            .build();
        game.create_object_from_card(&spell, controller, Zone::Stack)
    }

    #[test]
    fn conspire_cost_taps_exactly_two_shared_color_creatures() {
        let mut game = setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let spell = make_spell(&mut game, alice, ColorSet::RED);
        let red_one = make_creature(&mut game, alice, "Red One", ColorSet::RED);
        let red_two = make_creature(&mut game, alice, "Red Two", ColorSet::RED);
        let green = make_creature(&mut game, alice, "Green", ColorSet::GREEN);

        let effect = ConspireCostEffect::new();
        let mut ctx = ExecutionContext::new_default(spell, alice);
        effect
            .execute(&mut game, &mut ctx)
            .expect("conspire cost should resolve");

        assert!(
            game.is_tapped(red_one),
            "first shared-color creature should tap"
        );
        assert!(
            game.is_tapped(red_two),
            "second shared-color creature should tap"
        );
        assert!(
            !game.is_tapped(green),
            "off-color creature should stay untapped"
        );
    }

    #[test]
    fn conspire_cost_requires_two_shared_color_creatures() {
        let mut game = setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let spell = make_spell(&mut game, alice, ColorSet::RED);
        let _red = make_creature(&mut game, alice, "Red", ColorSet::RED);
        let _green = make_creature(&mut game, alice, "Green", ColorSet::GREEN);

        let effect = ConspireCostEffect::new();
        let err =
            crate::effects::CostExecutableEffect::can_execute_as_cost(&effect, &game, spell, alice)
                .expect_err("single shared-color creature should not satisfy conspire");
        assert!(
            matches!(err, CostValidationError::Other(_)),
            "expected a descriptive conspire validation error"
        );
    }
}

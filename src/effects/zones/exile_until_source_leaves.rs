//! Exile-until effect implementation.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::event_processor::{EventOutcome, process_zone_change};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Duration for "exile ... until ..." effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExileUntilDuration {
    /// Return when the source leaves the battlefield.
    SourceLeavesBattlefield,
    /// Return at the beginning of the next end step.
    NextEndStep,
    /// Return at end of combat.
    EndOfCombat,
}

/// Exile objects with an associated duration.
#[derive(Debug, Clone, PartialEq)]
pub struct ExileUntilEffect {
    /// What to exile.
    pub spec: ChooseSpec,
    /// How long the exile lasts.
    pub duration: ExileUntilDuration,
    /// Zone to return cards to when the duration expires.
    pub return_zone: Zone,
    /// Whether exiled cards should be turned face down.
    pub face_down: bool,
}

impl ExileUntilEffect {
    /// Create a new exile-until effect.
    pub fn new(spec: ChooseSpec, duration: ExileUntilDuration) -> Self {
        Self {
            spec,
            duration,
            return_zone: Zone::Battlefield,
            face_down: false,
        }
    }

    /// Mark exiled cards as face down.
    pub fn with_face_down(mut self, face_down: bool) -> Self {
        self.face_down = face_down;
        self
    }

    /// Exile until this source leaves the battlefield.
    pub fn source_leaves(spec: ChooseSpec) -> Self {
        Self::new(spec, ExileUntilDuration::SourceLeavesBattlefield)
    }
}

impl EffectExecutor for ExileUntilEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let objects = resolve_objects_from_spec(game, &self.spec, ctx)?;
        let mut exiled_count = 0_i32;
        for object_id in objects {
            let Some(obj) = game.object(object_id) else {
                continue;
            };
            let from_zone = obj.zone;

            let result = process_zone_change(
                game,
                object_id,
                from_zone,
                Zone::Exile,
                &mut ctx.decision_maker,
            );

            if let EventOutcome::Proceed(final_zone) = result
                && let Some(new_id) = game.move_object(object_id, final_zone)
                && final_zone == Zone::Exile
            {
                if self.face_down {
                    game.set_face_down(new_id);
                }
                game.add_exiled_with_source_link(ctx.source, new_id);
                exiled_count += 1;
            }
        }
        Ok(EffectOutcome::count(exiled_count))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target to exile"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::zones::matchers::WouldBeExiledMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::target::ObjectFilter;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_on_battlefield(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_exile_until_respects_destination_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        let creature_id = create_creature_on_battlefield(&mut game, "Elite Vanguard", alice);

        game.replacement_effects
            .add_resolution_effect(ReplacementEffect::with_matcher(
                source,
                alice,
                WouldBeExiledMatcher::new(ObjectFilter::permanent()),
                ReplacementAction::ChangeDestination(Zone::Hand),
            ));

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ExileUntilEffect::source_leaves(ChooseSpec::SpecificObject(creature_id));
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, crate::effect::EffectResult::Count(0));
        assert!(game.exile.is_empty());
        assert_eq!(game.get_exiled_with_source_links(source).len(), 0);
        assert_eq!(game.players[0].hand.len(), 1);
        assert!(game.battlefield.is_empty());
    }
}

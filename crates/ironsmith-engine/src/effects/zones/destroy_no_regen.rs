//! Destroy effect implementation that ignores regeneration.
//!
//! Used for oracle text like:
//! - "Destroy target creature. It can't be regenerated."
//! - "Destroy all creatures. They can't be regenerated."

use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter};

/// Effect that destroys permanents while ignoring regeneration shields.
///
/// This matches "can't be regenerated" tails on destroy effects: regeneration shields
/// don't replace the destruction event.
#[derive(Debug, Clone, PartialEq)]
pub struct DestroyNoRegenerationEffect {
    /// What to destroy - can be targeted, all matching, source, etc.
    pub spec: ChooseSpec,
    pub creature_destroyed_this_way_surface: bool,
}

impl DestroyNoRegenerationEffect {
    /// Create a destroy-no-regeneration effect with a custom spec.
    pub fn with_spec(spec: ChooseSpec) -> Self {
        Self {
            spec,
            creature_destroyed_this_way_surface: false,
        }
    }

    /// Create a targeted destroy-no-regeneration effect (single target).
    pub fn target(spec: ChooseSpec) -> Self {
        Self {
            spec: ChooseSpec::target(spec),
            creature_destroyed_this_way_surface: false,
        }
    }

    /// Create a targeted destroy-no-regeneration effect with a specific target count.
    pub fn targets(spec: ChooseSpec, count: ChoiceCount) -> Self {
        Self {
            spec: ChooseSpec::target(spec).with_count(count),
            creature_destroyed_this_way_surface: false,
        }
    }

    /// Create a non-targeted destroy-no-regeneration effect for all matching permanents.
    pub fn all(filter: ObjectFilter) -> Self {
        Self {
            spec: ChooseSpec::all(filter),
            creature_destroyed_this_way_surface: false,
        }
    }

    pub fn with_creature_destroyed_this_way_surface(mut self, present: bool) -> Self {
        self.creature_destroyed_this_way_surface = present;
        self
    }
}

impl EffectExecutor for DestroyNoRegenerationEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Destroyed)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        // Regeneration shields are one-shot replacement effects; "can't be
        // regenerated" means they can't replace this destruction (CR 701.19c).
        // Otherwise this is an ordinary destroy: a mass destruction is one
        // simultaneous event (CR 701.8a, 603.10a).
        if self.spec.is_target() && self.spec.is_single() {
            return super::destroy::execute_single_target_destroy_with_outputs(
                game, ctx, &self.spec, false,
            );
        }
        super::destroy::execute_simultaneous_destroy_with_outputs(game, ctx, &self.spec, false)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.spec)
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        Some(self.spec.count())
    }

    fn target_description(&self) -> &'static str {
        "permanent to destroy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::RegenerateEffect;
    use crate::effects::{ExecutionContext, ResolvedTarget};
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn destroy_no_regeneration_ignores_regeneration_shields() {
        let mut game = setup_game();
        let alice = crate::ids::PlayerId::from_index(0);
        let bob = crate::ids::PlayerId::from_index(1);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Shielded Bear")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
            .build();
        let creature_id: ObjectId =
            game.create_object_from_card(&creature_card, bob, Zone::Battlefield);

        // Apply regeneration via the proper effect (creates replacement effect).
        let mut regen_ctx = ExecutionContext::new_default(creature_id, bob);
        RegenerateEffect::source(crate::effect::Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .unwrap();
        assert!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(creature_id)
                > 0
        );

        let effect = DestroyNoRegenerationEffect::target(ChooseSpec::creature());
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let out = effect.execute(&mut game, &mut ctx).expect("execute");
        assert!(
            out.status.is_success(),
            "expected destroy to succeed, got {:?}",
            out
        );
        assert!(
            game.object(creature_id).is_none(),
            "expected creature to be destroyed"
        );
        assert_eq!(
            game.effect_store
                .replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            0
        );
    }

    #[test]
    fn destroy_no_regeneration_multi_target_records_graveyard_results() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let first = CardBuilder::new(CardId::from_raw(2), "First Shieldless Bear")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
            .build();
        let second = CardBuilder::new(CardId::from_raw(3), "Second Shieldless Bear")
            .card_types(vec![CardType::Creature])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
            .build();
        let first_id = game.create_object_from_card(&first, bob, Zone::Battlefield);
        let second_id = game.create_object_from_card(&second, bob, Zone::Battlefield);

        let spec = ChooseSpec::target(ChooseSpec::creature()).with_count(ChoiceCount::exactly(2));
        let effect = DestroyNoRegenerationEffect::with_spec(spec.clone());
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![
                ResolvedTarget::Object(first_id),
                ResolvedTarget::Object(second_id),
            ])
            .with_target_assignments(vec![crate::game_state::TargetAssignment {
                spec,
                range: 0..2,
            }]);

        let outcome = effect.execute(&mut game, &mut ctx).expect("execute");

        assert_eq!(outcome.as_count(), Some(2));
        assert_eq!(outcome.output_objects().len(), 2);
        assert!(
            outcome.output_objects().iter().all(|id| {
                game.object(*id).is_some_and(|obj| {
                    obj.zone == Zone::Graveyard && game.controller_of(obj) == bob
                })
            }),
            "destroy-no-regeneration effect should surface graveyard objects, got {:?}",
            outcome.output_objects()
        );
    }

    #[test]
    fn destroy_all_other_without_regeneration_spares_source_and_breaks_shields() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source_card = CardBuilder::new(CardId::from_raw(4), "Wrathful Source")
            .card_types(vec![CardType::Creature])
            .build();
        let shielded_card = CardBuilder::new(CardId::from_raw(5), "Shielded Creature")
            .card_types(vec![CardType::Creature])
            .build();
        let source_id = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let shielded_id = game.create_object_from_card(&shielded_card, bob, Zone::Battlefield);

        let mut regen_ctx = ExecutionContext::new_default(shielded_id, bob);
        RegenerateEffect::source(crate::effect::Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .expect("regeneration shield should resolve");

        let mut filter = ObjectFilter::creature();
        filter.other = true;
        let effect = DestroyNoRegenerationEffect::all(filter);
        let mut ctx = ExecutionContext::new_default(source_id, alice);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("mass destruction should resolve");

        assert_eq!(outcome.as_count(), Some(1));
        assert!(
            game.object(source_id).is_some(),
            "the source-identity exception must survive"
        );
        assert!(
            game.object(shielded_id).is_none(),
            "the affected creature must die despite its regeneration shield"
        );
    }
}

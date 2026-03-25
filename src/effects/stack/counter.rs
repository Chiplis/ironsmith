//! Counter spell effect implementation.

use crate::ability::AbilityKind;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_single_object_for_effect;
use crate::event_processor::{EventOutcome, process_zone_change_with_additional_effects};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::zone::Zone;

/// Effect that counters a target spell on the stack.
///
/// This removes the spell from the stack and puts it into its owner's graveyard.
/// Abilities that are countered simply disappear.
///
/// # Fields
///
/// * `target` - Which spell to counter
///
/// # Example
///
/// ```ignore
/// // Counter target spell
/// let effect = CounterEffect::new(ChooseSpec::spell());
///
/// // Counter target creature spell
/// let effect = CounterEffect::new(ChooseSpec::creature_spell());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CounterEffect {
    /// The targeting specification (for UI/validation purposes).
    pub target: ChooseSpec,
}

impl CounterEffect {
    /// Create a new counter effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self { target }
    }

    /// Create an effect that counters any spell.
    pub fn any_spell() -> Self {
        Self::new(ChooseSpec::spell())
    }
}

impl EffectExecutor for CounterEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = resolve_single_object_for_effect(game, ctx, &self.target)?;

        // Check if the spell can't be countered
        if let Some(obj) = game.object(target_id) {
            let cant_be_countered = obj.abilities.iter().any(|ability| {
                if let AbilityKind::Static(s) = &ability.kind {
                    s.cant_be_countered()
                } else {
                    false
                }
            });
            if cant_be_countered {
                // Spell can't be countered - effect does nothing
                return Ok(EffectOutcome::protected());
            }
        }

        // Find the stack entry for this object
        if game.stack.iter().any(|e| e.object_id == target_id) {
            let additional_effects = ctx.additional_replacement_effects_snapshot();
            let outcome = process_zone_change_with_additional_effects(
                game,
                target_id,
                Zone::Stack,
                Zone::Graveyard,
                ctx.cause.clone(),
                &mut ctx.decision_maker,
                &additional_effects,
            );

            match outcome {
                EventOutcome::Prevented => return Ok(EffectOutcome::prevented()),
                EventOutcome::Proceed(final_zone) => {
                    if let Some(idx) = game.stack.iter().position(|e| e.object_id == target_id) {
                        let entry = game.stack.remove(idx);
                        // Countered abilities simply disappear; countered spells leave the stack
                        // through zone-change processing so replacement effects can rewrite
                        // destinations like Force of Negation's exile clause.
                        if !entry.is_ability {
                            let move_result = game
                                .move_object_with_etb_processing_with_dm_and_cause(
                                    entry.object_id,
                                    final_zone,
                                    ctx.cause.clone(),
                                    &mut ctx.decision_maker,
                                );
                            if final_zone == Zone::Exile
                                && let Some(result) = move_result
                            {
                                game.add_exiled_with_source_link(ctx.source, result.new_id);
                            }
                        }
                    }
                }
                EventOutcome::Replaced => {
                    if let Some(idx) = game.stack.iter().position(|e| e.object_id == target_id) {
                        game.stack.remove(idx);
                    }
                }
                EventOutcome::NotApplicable => return Ok(EffectOutcome::target_invalid()),
            }

            if !game.stack.iter().any(|e| e.object_id == target_id) {
                Ok(EffectOutcome::resolved())
            } else {
                Ok(EffectOutcome::target_invalid())
            }
        } else {
            // Target is no longer on the stack
            Ok(EffectOutcome::target_invalid())
        }
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "spell to counter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::{Effect, OutcomeStatus};
    use crate::executor::execute_effect;
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_instant(
        game: &mut GameState,
        owner: PlayerId,
        zone: Zone,
        name: &str,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Instant])
            .build();
        game.create_object_from_card(&card, owner, zone)
    }

    #[test]
    fn counter_spell_honors_registered_stack_to_graveyard_replacement() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = create_instant(&mut game, bob, Zone::Stack, "Target Spell");
        let stable_id = game
            .object(target_spell)
            .expect("target spell should exist")
            .stable_id;
        game.stack.push(StackEntry::new(target_spell, bob));

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let register = Effect::new(crate::effects::RegisterZoneReplacementEffect::new(
            ChooseSpec::SpecificObject(target_spell),
            Some(Zone::Stack),
            Some(Zone::Graveyard),
            Zone::Exile,
            crate::effects::ReplacementApplyMode::OneShot,
        ));
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        execute_effect(&mut game, &register, &mut ctx)
            .expect("replacement registration should succeed");

        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target_spell))),
            &mut ctx,
        )
        .expect("counter should resolve");
        assert!(
            outcome.status.is_success(),
            "counter should resolve successfully"
        );
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == target_spell),
            "countered spell should be removed from the stack"
        );

        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("countered spell should still be findable after the zone change");
        assert_eq!(
            game.object(moved_id)
                .expect("countered spell should still exist after being moved")
                .zone,
            Zone::Exile
        );
    }

    #[test]
    fn counter_spell_moves_spell_to_owners_graveyard() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = create_instant(&mut game, bob, Zone::Stack, "Target Spell");
        let stable_id = game
            .object(target_spell)
            .expect("target spell should exist")
            .stable_id;
        game.stack.push(StackEntry::new(target_spell, bob));

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(target_spell))),
            &mut ctx,
        )
        .expect("counter should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            !game.stack.iter().any(|entry| entry.object_id == target_spell),
            "countered spell should leave the stack"
        );

        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("countered spell should still be tracked after moving");
        let moved_obj = game
            .object(moved_id)
            .expect("countered spell should still exist after moving");
        assert_eq!(moved_obj.zone, Zone::Graveyard);
        assert_eq!(moved_obj.owner, bob);
    }

    #[test]
    fn counter_ability_only_removes_it_from_the_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Ability Source")
                .card_types(vec![CardType::Artifact])
                .build(),
            bob,
            Zone::Battlefield,
        );
        game.stack.push(StackEntry::ability(
            source,
            bob,
            vec![Effect::draw(1)],
        ));

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(source))),
            &mut ctx,
        )
        .expect("counter should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            !game.stack.iter().any(|entry| entry.object_id == source),
            "countered ability should disappear from the stack"
        );
        assert_eq!(
            game.object(source)
                .expect("ability source permanent should still exist")
                .zone,
            Zone::Battlefield
        );
    }

    #[test]
    fn countering_a_spell_does_not_refund_paid_mana() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Paid Spell")
                .card_types(vec![CardType::Instant])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
                .build(),
            bob,
            Zone::Hand,
        );
        game.player_mut(bob)
            .expect("bob exists")
            .mana_pool
            .add(ManaSymbol::Blue, 1);
        assert!(
            game.try_pay_mana_cost_with_reason(
                bob,
                Some(target_spell),
                &ManaCost::from_symbols(vec![ManaSymbol::Blue]),
                0,
                crate::costs::PaymentReason::CastSpell,
            ),
            "bob should be able to pay for the spell before it is countered"
        );
        let stack_spell = game
            .move_object_by_effect(target_spell, Zone::Stack)
            .expect("paid spell should move to stack");
        game.stack.push(StackEntry::new(stack_spell, bob));
        assert_eq!(
            game.player(bob).expect("bob exists").mana_pool.total(),
            0,
            "mana should already be spent before the counter resolves"
        );

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::SpecificObject(stack_spell))),
            &mut ctx,
        )
        .expect("counter should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert_eq!(
            game.player(bob).expect("bob exists").mana_pool.total(),
            0,
            "countering a spell must not refund the mana already paid to cast it"
        );
    }
}

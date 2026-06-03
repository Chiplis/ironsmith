//! Counter spell effect implementation.

use crate::ability::AbilityKind;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_for_effect;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::{EventOutcome, process_zone_change_with_additional_effects};
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
use crate::zone::Zone;
pub use ironsmith_core::CounterEffect;

fn counter_one_stack_object(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    target_id: ObjectId,
) -> EffectOutcome {
    if !game.can_be_countered(target_id) {
        return EffectOutcome::protected();
    }

    // Check if the spell can't be countered
    if let Some(obj) = game.object(target_id) {
        let abilities = game
            .current_abilities(target_id)
            .unwrap_or_else(|| obj.abilities.clone());
        let cant_be_countered = abilities.iter().any(|ability| {
            if let AbilityKind::Static(s) = &ability.kind {
                let display = s.display().to_ascii_lowercase();
                s.cant_be_countered()
                    || s.id() == crate::static_abilities::StaticAbilityId::CantBeCountered
                    || display.contains("can't be countered")
                    || display.contains("cant be countered")
            } else {
                false
            }
        });
        if cant_be_countered {
            // Spell can't be countered - effect does nothing
            return EffectOutcome::protected();
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
            EventOutcome::Prevented => return EffectOutcome::prevented(),
            EventOutcome::Proceed(final_zone) => {
                if let Some(idx) = game.stack.iter().position(|e| e.object_id == target_id) {
                    let entry = game.stack.remove(idx);
                    // Countered abilities simply disappear; countered spells leave the stack
                    // through zone-change processing so replacement effects can rewrite
                    // destinations like Force of Negation's exile clause.
                    if !entry.is_ability {
                        let move_result = game.move_object_with_etb_processing_with_dm_and_cause(
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
            EventOutcome::NotApplicable => return EffectOutcome::target_invalid(),
        }

        if !game.stack.iter().any(|e| e.object_id == target_id) {
            EffectOutcome::resolved()
        } else {
            EffectOutcome::target_invalid()
        }
    } else {
        // Target is no longer on the stack
        EffectOutcome::target_invalid()
    }
}

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
impl EffectExecutor for CounterEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_ids = resolve_objects_for_effect(game, ctx, &self.target)?;
        if target_ids.is_empty() {
            return Ok(EffectOutcome::target_invalid());
        }

        Ok(EffectOutcome::aggregate(target_ids.into_iter().map(
            |target_id| counter_one_stack_object(game, ctx, target_id),
        )))
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
    use crate::effects::execute_effect;
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::static_abilities::StaticAbility;
    use crate::target::{ObjectFilter, PlayerFilter};
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
    fn counter_spell_honors_own_from_anywhere_exile_replacement_on_stack() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let target_spell = create_instant(&mut game, bob, Zone::Stack, "Self Replacing Spell");
        let stable_id = game
            .object(target_spell)
            .expect("target spell should exist")
            .stable_id;
        game.object_mut(target_spell)
            .expect("target spell should exist")
            .abilities
            .push(
                crate::ability::Ability::static_ability(
                    StaticAbility::exile_to_exile_instead_of_graveyard(
                        ObjectFilter::source(),
                        PlayerFilter::Any,
                    ),
                )
                .in_zones(vec![
                    Zone::Battlefield,
                    Zone::Stack,
                    Zone::Graveyard,
                    Zone::Hand,
                    Zone::Library,
                    Zone::Exile,
                    Zone::Command,
                ]),
            );
        game.stack.push(StackEntry::new(target_spell, bob));
        game.update_replacement_effects();

        let counter_source = create_instant(&mut game, alice, Zone::Stack, "Counter Source");
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(counter_source, alice, &mut dm);
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
        let moved_id = game
            .find_object_by_stable_id(stable_id)
            .expect("countered spell should still be tracked after moving");
        assert_eq!(
            game.object(moved_id)
                .expect("countered spell should still exist after moving")
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
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == target_spell),
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
        game.stack
            .push(StackEntry::ability(source, bob, vec![Effect::draw(1)]));

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
    fn kadenas_silencer_counter_all_opponent_abilities_leaves_your_stack_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let opponent_first_ability_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Opponent Ability Source A")
                .card_types(vec![CardType::Artifact])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let opponent_second_ability_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Opponent Ability Source B")
                .card_types(vec![CardType::Artifact])
                .build(),
            bob,
            Zone::Battlefield,
        );
        let your_ability_source = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Your Ability Source")
                .card_types(vec![CardType::Artifact])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let opponent_spell = create_instant(&mut game, bob, Zone::Stack, "Opponent Spell");

        game.stack.push(StackEntry::ability(
            opponent_first_ability_source,
            bob,
            vec![Effect::draw(1)],
        ));
        game.stack.push(StackEntry::ability(
            opponent_second_ability_source,
            bob,
            vec![Effect::draw(1)],
        ));
        game.stack.push(StackEntry::ability(
            your_ability_source,
            alice,
            vec![Effect::draw(1)],
        ));
        game.stack.push(StackEntry::new(opponent_spell, bob));

        let silencer = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Kadena's Silencer")
                .card_types(vec![CardType::Creature])
                .build(),
            alice,
            Zone::Battlefield,
        );
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(silencer, alice, &mut dm);
        let outcome = execute_effect(
            &mut game,
            &Effect::new(CounterEffect::new(ChooseSpec::All(
                ObjectFilter::ability().controlled_by(PlayerFilter::Opponent),
            ))),
            &mut ctx,
        )
        .expect("Kadena's Silencer counter-all-abilities effect should resolve");

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == opponent_first_ability_source),
            "first opponent ability should be countered"
        );
        assert!(
            !game
                .stack
                .iter()
                .any(|entry| entry.object_id == opponent_second_ability_source),
            "second opponent ability should be countered"
        );
        assert!(
            game.stack
                .iter()
                .any(|entry| entry.object_id == your_ability_source && entry.is_ability),
            "your ability should not be countered"
        );
        assert!(
            game.stack
                .iter()
                .any(|entry| entry.object_id == opponent_spell && !entry.is_ability),
            "opponent spell should not be countered by an ability-only filter"
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

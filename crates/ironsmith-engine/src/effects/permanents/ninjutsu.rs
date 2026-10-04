//! Ninjutsu keyword support.
//!
//! Ninjutsu is modeled as:
//! - a cost effect (`NinjutsuCostEffect`) that returns an unblocked attacker you control
//!   to hand and records that attack target.
//! - a resolution effect (`NinjutsuEffect`) that puts the source card from hand onto the
//!   battlefield tapped and attacking the recorded target.

use crate::effects::zones::{finish_zone_change_receipts, finish_battlefield_entry_receipts};
use crate::combat_state::{AttackTarget, AttackerInfo, get_attack_target, is_unblocked};
use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, Phase, Step};
use crate::ids::{ObjectId, PlayerId};
use crate::types::CardType;
use crate::zone::Zone;
pub use ironsmith_core::{NinjutsuCostEffect, NinjutsuEffect, SneakCostEffect};

fn in_ninjutsu_window(game: &GameState) -> bool {
    if game.turn.phase != Phase::Combat {
        return false;
    }
    matches!(
        game.turn.step,
        Some(Step::DeclareBlockers | Step::CombatDamage | Step::EndCombat)
    )
}

fn in_sneak_window(game: &GameState) -> bool {
    game.turn.phase == Phase::Combat && game.turn.step == Some(Step::DeclareBlockers)
}

fn unblocked_attackers(game: &GameState, controller: PlayerId) -> Vec<ObjectId> {
    let Some(combat) = game.combat.as_ref() else {
        return Vec::new();
    };

    combat
        .attackers
        .iter()
        .filter_map(|info| {
            let creature = info.creature;
            let obj = game.object(creature)?;
            if obj.zone != Zone::Battlefield || game.controller_of(obj) != controller {
                return None;
            }
            if !is_unblocked(combat, creature) {
                return None;
            }
            Some(creature)
        })
        .collect()
}

impl EffectExecutor for NinjutsuCostEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
        if !in_ninjutsu_window(game) {
            return Err(ExecutionError::Impossible(
                "Ninjutsu can only be activated during combat after blockers are declared"
                    .to_string(),
            ));
        }

        let Some(source_obj) = game.object(ctx.source) else {
            return Err(ExecutionError::ObjectNotFound(ctx.source));
        };
        if source_obj.zone != Zone::Hand {
            return Err(ExecutionError::Impossible(
                "Ninjutsu source must be in hand".to_string(),
            ));
        }

        let candidates = unblocked_attackers(game, ctx.controller);
        if candidates.is_empty() {
            return Err(ExecutionError::Impossible(
                "No unblocked attacker you control to return".to_string(),
            ));
        }

        let chosen = {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose an unblocked attacker you control to return to hand",
                candidates.clone(),
                1,
                Some(1),
            );
            make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            )
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        if chosen.len() != 1 || !candidates.contains(&chosen[0]) {
            return Err(ExecutionError::Impossible(
                "No valid unblocked attacker was chosen for ninjutsu".to_string()));
        }
        let chosen_attacker = chosen[0];

        let attack_target = game
            .combat
            .as_ref()
            .and_then(|combat| get_attack_target(combat, chosen_attacker))
            .cloned()
            .ok_or_else(|| {
                ExecutionError::Impossible(
                    "Chosen attacker has no combat attack target".to_string(),
                )
            })?;

        // Return is a cost, but its zone change still sees replacements
        // (CR 118.11), notably Unearth's exile replacement.
        let outcome = {
    let zone_additional_effects = ctx.additional_replacement_effects_snapshot();
    crate::effects::zones::apply_zone_change_with_context_and_additional_effects(
        game,
        chosen_attacker,
        Zone::Battlefield,
        Zone::Hand,
        ctx.cause.clone(),
        ctx,
        &zone_additional_effects
    )
}?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if matches!(&outcome.original, crate::events::processing::EventOutcome::NotApplicable) {
            return Err(ExecutionError::Impossible("Chosen attacker cannot be returned".to_string()));
        }
        game.record_ninjutsu_attack_target(ctx.source, attack_target);
        finish_zone_change_receipts(game, ctx, EffectOutcome::resolved(), vec![(chosen_attacker, outcome)])
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending { return Ok(EffectOutcome::count(0)); }
        result
    }

    fn cost_description(&self) -> Option<String> {
        Some("Return an unblocked attacker you control to its owner's hand".to_string())
    }
}

impl CostExecutableEffect for NinjutsuCostEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if !in_ninjutsu_window(game) {
            return Err(CostValidationError::Other(
                "Ninjutsu can only be activated during combat after blockers are declared"
                    .to_string(),
            ));
        }

        let Some(source_obj) = game.object(source) else {
            return Err(CostValidationError::Other(
                "Ninjutsu source does not exist".to_string(),
            ));
        };
        if source_obj.zone != Zone::Hand {
            return Err(CostValidationError::Other(
                "Ninjutsu source must be in hand".to_string(),
            ));
        }

        if unblocked_attackers(game, controller).is_empty() {
            return Err(CostValidationError::Other(
                "No unblocked attacker you control to return".to_string(),
            ));
        }

        Ok(())
    }
}

impl EffectExecutor for SneakCostEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
        if !in_sneak_window(game) {
            return Err(ExecutionError::Impossible(
                "Sneak can only be paid during the declare blockers step".to_string(),
            ));
        }

        let Some(source_obj) = game.object(ctx.source) else {
            return Err(ExecutionError::ObjectNotFound(ctx.source));
        };
        if !matches!(source_obj.zone, Zone::Hand | Zone::Stack) {
            return Err(ExecutionError::Impossible(
                "Sneak source must be in hand or on the stack".to_string(),
            ));
        }

        let candidates = unblocked_attackers(game, ctx.controller);
        if candidates.is_empty() {
            return Err(ExecutionError::Impossible(
                "No unblocked attacker you control to return".to_string(),
            ));
        }

        let chosen = {
            let spec = ChooseObjectsSpec::new(
                ctx.source,
                "Choose an unblocked attacker you control to return to hand",
                candidates.clone(),
                1,
                Some(1),
            );
            make_decision(
                game,
                ctx.decision_maker,
                ctx.controller,
                Some(ctx.source),
                spec,
            )
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }

        if chosen.len() != 1 || !candidates.contains(&chosen[0]) {
            return Err(ExecutionError::Impossible(
                "No valid unblocked attacker was chosen for sneak".to_string()));
        }
        let chosen_attacker = chosen[0];

        let attack_target = game
            .combat
            .as_ref()
            .and_then(|combat| get_attack_target(combat, chosen_attacker))
            .cloned()
            .ok_or_else(|| {
                ExecutionError::Impossible(
                    "Chosen attacker has no combat attack target".to_string(),
                )
            })?;

        // Return is a cost, but its zone change still sees replacements
        // (CR 118.11), notably Unearth's exile replacement.
        let outcome = {
    let zone_additional_effects = ctx.additional_replacement_effects_snapshot();
    crate::effects::zones::apply_zone_change_with_context_and_additional_effects(
        game,
        chosen_attacker,
        Zone::Battlefield,
        Zone::Hand,
        ctx.cause.clone(),
        ctx,
        &zone_additional_effects
    )
}?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        if matches!(&outcome.original, crate::events::processing::EventOutcome::NotApplicable) {
            return Err(ExecutionError::Impossible("Chosen attacker cannot be returned".to_string()));
        }
        game.record_sneak_attack_target(ctx.source, attack_target);
        finish_zone_change_receipts(game, ctx, EffectOutcome::resolved(), vec![(chosen_attacker, outcome)])
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending { return Ok(EffectOutcome::count(0)); }
        result
    }

    fn cost_description(&self) -> Option<String> {
        Some("Return an unblocked attacker you control to its owner's hand".to_string())
    }
}

impl CostExecutableEffect for SneakCostEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if !in_sneak_window(game) {
            return Err(CostValidationError::Other(
                "Sneak can only be paid during the declare blockers step".to_string(),
            ));
        }

        let Some(source_obj) = game.object(source) else {
            return Err(CostValidationError::Other(
                "Sneak source does not exist".to_string(),
            ));
        };
        if !matches!(source_obj.zone, Zone::Hand | Zone::Stack) {
            return Err(CostValidationError::Other(
                "Sneak source must be in hand or on the stack".to_string(),
            ));
        }

        if unblocked_attackers(game, controller).is_empty() {
            return Err(CostValidationError::Other(
                "No unblocked attacker you control to return".to_string(),
            ));
        }

        Ok(())
    }
}

fn pop_ninjutsu_attack_target(game: &mut GameState, source: ObjectId) -> Option<AttackTarget> {
    game.pop_ninjutsu_attack_target(source)
}

/// CR 508.4a: the creature enters attacking only if the player it would attack
/// is still in the game, or the planeswalker is still controlled by a
/// defending player (an opponent of the attacking player who's still in it).
fn attack_target_still_valid(
    game: &GameState,
    attacker_controller: crate::ids::PlayerId,
    target: &AttackTarget,
) -> bool {
    match target {
        AttackTarget::Player(player) => game
            .player(*player)
            .is_some_and(|player| player.is_in_game()),
        AttackTarget::Planeswalker(planeswalker) => game.object(*planeswalker).is_some_and(|obj| {
            let controller = game.controller_of(obj);
            obj.zone == Zone::Battlefield
                && game.current_has_card_type(*planeswalker, CardType::Planeswalker)
                && controller != attacker_controller
                && game.are_opponents(attacker_controller, controller)
                && game
                    .player(controller)
                    .is_some_and(|player| player.is_in_game())
        }),
        AttackTarget::Battle(battle) => game.object(*battle).is_some_and(|obj| {
            obj.zone == Zone::Battlefield
                && game.current_has_card_type(*battle, CardType::Battle)
                && game.battle_protector(*battle).is_some_and(|protector| {
                    game.player(protector)
                        .is_some_and(|player| player.is_in_game())
                })
        }),
        // CR 506.4c / 508.4a: the returned creature wasn't attacking anything.
        AttackTarget::Nothing { .. } => false,
    }
}

impl EffectExecutor for NinjutsuEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let result = (|| -> Result<EffectOutcome, ExecutionError> {
        let Some(attack_target) = ctx.ninjutsu_attack_target.clone()
            .or_else(|| pop_ninjutsu_attack_target(game, ctx.source)) else {
            return Ok(EffectOutcome::target_invalid());
        };

        let Some(source_obj) = game.object(ctx.source) else {
            return Ok(EffectOutcome::target_invalid());
        };
        if source_obj.zone != Zone::Hand {
            return Ok(EffectOutcome::target_invalid());
        }

        let outcome = move_to_battlefield_with_options(
            game,
            ctx,
            ctx.source,
            BattlefieldEntryOptions::specific(ctx.controller, true),
        )?;

        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let receipt = outcome.ok_or_else(|| ExecutionError::InternalError(
            "completed ninjutsu entry has no receipt".into()))?;
        let original = match &receipt.outcome {
            BattlefieldEntryOutcome::Moved(new_id) => {
                let new_id = *new_id;
                let valid_target = attack_target_still_valid(game, ctx.controller, &attack_target);
                if let Some(combat) = game.combat.as_mut()
                    && valid_target
                {
                    combat.attackers.push(AttackerInfo {
                        creature: new_id,
                        target: attack_target,
                    });
                }
                EffectOutcome::with_objects(vec![new_id])
            }
            BattlefieldEntryOutcome::Redirected(change) => EffectOutcome::with_objects(change.new_object_ids.clone()),
            BattlefieldEntryOutcome::Prevented => EffectOutcome::prevented(),
        };
        finish_battlefield_entry_receipts(game, ctx, original, vec![receipt])
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || result.is_err() {
            *game = checkpoint;
            context_checkpoint.restore(ctx);
        }
        if pending { return Ok(EffectOutcome::count(0)); }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::combat_state::CombatState;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build()
    }

    fn create_creature_in_zone(
        game: &mut GameState,
        name: &str,
        owner: PlayerId,
        zone: Zone,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, zone);
        game.add_object(obj);
        id
    }

    #[test]
    fn ninjutsu_cost_returns_unblocked_attacker_and_records_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature_in_zone(&mut game, "Ninja", alice, Zone::Hand);
        let attacker = create_creature_in_zone(&mut game, "Attacker", alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);

        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareBlockers);
        game.combat = Some(CombatState {
            block_declaration_complete: true,
            attackers: vec![AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = NinjutsuCostEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("ninjutsu cost should resolve");

        assert!(
            matches!(result.status, crate::effect::OutcomeStatus::Succeeded),
            "expected resolved cost effect, got {:?}",
            result
        );
        assert!(
            game.combat
                .as_ref()
                .is_some_and(|combat| combat.attackers.is_empty()),
            "returned attacker should be removed from combat"
        );
        assert!(
            game.players[0]
                .hand
                .iter()
                .filter_map(|id| game.object(*id))
                .any(|obj| obj.name == "Attacker"),
            "returned attacker should be in hand"
        );
        let recorded = game.last_ninjutsu_attack_target(source).cloned();
        assert_eq!(recorded, Some(AttackTarget::Player(bob)));
    }

    #[test]
    fn ninjutsu_effect_puts_source_tapped_and_attacking_recorded_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature_in_zone(&mut game, "Ninja", alice, Zone::Hand);
        game.record_ninjutsu_attack_target(source, AttackTarget::Player(bob));
        game.combat = Some(CombatState::default());
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::CombatDamage);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = NinjutsuEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("ninjutsu effect should resolve");

        let entered = match result.value {
            crate::effect::OutcomeValue::Objects(ids) => ids[0],
            other => panic!("expected moved object result, got {other:?}"),
        };

        assert!(
            game.battlefield.contains(&entered),
            "ninjutsu source should enter the battlefield"
        );
        assert!(
            game.is_tapped(entered),
            "ninjutsu source should enter tapped"
        );
        let attackers = game
            .combat
            .as_ref()
            .map(|combat| combat.attackers.clone())
            .unwrap_or_default();
        assert!(
            attackers
                .iter()
                .any(|info| info.creature == entered && info.target == AttackTarget::Player(bob)),
            "ninjutsu source should be attacking recorded target"
        );
    }

    #[test]
    fn ninjutsu_cost_not_payable_before_blockers_declared() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let source = create_creature_in_zone(&mut game, "Ninja", alice, Zone::Hand);
        let attacker = create_creature_in_zone(&mut game, "Attacker", alice, Zone::Battlefield);
        game.remove_summoning_sickness(attacker);
        game.turn.phase = Phase::Combat;
        game.turn.step = Some(Step::DeclareAttackers);
        game.combat = Some(CombatState {
            block_declaration_complete: true,
            attackers: vec![AttackerInfo {
                creature: attacker,
                target: AttackTarget::Player(bob),
            }],
            ..CombatState::default()
        });

        let can_pay = crate::effects::EffectExecutor::can_execute_as_cost(
            &NinjutsuCostEffect::new(),
            &game,
            source,
            alice,
        );
        assert!(
            can_pay.is_err(),
            "ninjutsu should not be payable before blockers are declared"
        );
    }
}

#[cfg(test)]
mod replacement_cost_owner_contract_tests {
    use super::*;
    use crate::ids::CardId;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    fn check(sneak: bool, prevent: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let card = crate::card::CardBuilder::new(CardId::new(), "Cost creature")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2)).build();
        let source = game.create_object_from_card(&card, alice, Zone::Hand);
        let attacker = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let replacement_source = game.create_object_from_card(&card, bob, Zone::Battlefield);
        game.turn.phase = Phase::Combat; game.turn.step = Some(Step::DeclareBlockers);
        game.combat = Some(crate::combat_state::CombatState {
            block_declaration_complete: true,
            attackers: vec![AttackerInfo { creature: attacker, target: AttackTarget::Player(bob) }],
            ..Default::default()
        });
        let action = if prevent { ReplacementAction::Prevent } else {
            ReplacementAction::Instead(vec![crate::effect::Effect::gain_life(3)])
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            replacement_source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(
                crate::target::ObjectFilter::specific(attacker), Some(Zone::Battlefield), Some(Zone::Hand)), action));
        game.take_pending_trigger_events();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let result = if sneak { SneakCostEffect::new().execute(&mut game, &mut ctx) }
            else { NinjutsuCostEffect::new().execute(&mut game, &mut ctx) };
        let outcome = result.expect("a legal attempted cost remains paid after replacement (118.11)");
        assert!(matches!(outcome.status, crate::effect::OutcomeStatus::Succeeded));
        assert_eq!(game.object(attacker).unwrap().zone, Zone::Battlefield);
        assert_eq!(game.object(source).unwrap().zone, Zone::Hand);
        assert_eq!(game.player(alice).unwrap().life, 20);
        assert_eq!(game.player(bob).unwrap().life, if prevent {20} else {23});
        let target = if sneak { game.last_sneak_attack_target(source) } else { game.last_ninjutsu_attack_target(source) };
        assert_eq!(target.cloned(), Some(AttackTarget::Player(bob)));
        assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
    }
    #[test] fn ninjutsu_instead_cost_is_paid() { check(false, false); }
    #[test] fn ninjutsu_prevented_cost_is_paid() { check(false, true); }
    #[test] fn sneak_instead_cost_is_paid() { check(true, false); }
    #[test] fn sneak_prevented_cost_is_paid() { check(true, true); }
}

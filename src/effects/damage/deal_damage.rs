//! Deal damage effect implementation.
//!
//! This module implements the `DealDamage` effect, which deals damage to a target
//! creature, planeswalker, or player.

use crate::effect::{EffectOutcome, EffectResult, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_value;
use crate::event_processor::process_damage_assignments_with_event_with_source_snapshot;
use crate::events::DamageEvent;
use crate::events::LifeLossEvent;
use crate::events::combat::{CreatureAttackedEvent, CreatureBecameBlockedEvent};
use crate::executor::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::game_event::DamageTarget;
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};
use crate::triggers::AttackEventTarget;
use crate::triggers::TriggerEvent;
use crate::types::CardType;

/// Effect that deals damage to a target creature, planeswalker, or player.
///
/// # Fields
///
/// * `amount` - The amount of damage to deal (can be fixed or variable)
/// * `target` - The target specification (creature, player, or "any target")
/// * `source_is_combat` - Whether this damage is combat damage
///
/// # Example
///
/// ```ignore
/// // Deal 3 damage to any target (Lightning Bolt)
/// let effect = DealDamageEffect {
///     amount: Value::Fixed(3),
///     target: ChooseSpec::AnyTarget,
///     source_is_combat: false,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct DealDamageEffect {
    /// The amount of damage to deal.
    pub amount: Value,
    /// The target specification.
    pub target: ChooseSpec,
    /// Whether this damage is combat damage.
    pub source_is_combat: bool,
}

impl DealDamageEffect {
    /// Create a new deal damage effect.
    pub fn new(amount: impl Into<Value>, target: ChooseSpec) -> Self {
        Self {
            amount: amount.into(),
            target,
            source_is_combat: false,
        }
    }

    /// Set whether this is combat damage.
    pub fn with_combat(mut self, is_combat: bool) -> Self {
        self.source_is_combat = is_combat;
        self
    }
}

fn apply_processed_damage_outcome(
    game: &mut GameState,
    source: crate::ids::ObjectId,
    source_snapshot: Option<&crate::snapshot::ObjectSnapshot>,
    initial_target: DamageTarget,
    amount: u32,
    source_is_combat: bool,
) -> EffectOutcome {
    let processed = process_damage_assignments_with_event_with_source_snapshot(
        game,
        source,
        initial_target,
        amount,
        source_is_combat,
        source_snapshot,
    );

    if processed.replacement_prevented {
        return EffectOutcome::from_result(EffectResult::Prevented);
    }

    let mut outcomes = Vec::new();
    for assignment in processed.assignments {
        let mut outcome = EffectOutcome::count(assignment.amount as i32);
        if assignment.amount > 0 {
            outcome = outcome.with_event(TriggerEvent::new(DamageEvent::new(
                source,
                assignment.target,
                assignment.amount,
                source_is_combat,
            )));
        }

        match assignment.target {
            DamageTarget::Player(player_id) => {
                let life_loss = if assignment.amount == 0 || !game.can_change_life_total(player_id) {
                    0
                } else if let Some(player) = game.player_mut(player_id) {
                    player.deal_damage(assignment.amount)
                } else {
                    0
                };
                if life_loss > 0 {
                    outcome = outcome.with_event(TriggerEvent::new(LifeLossEvent::new(
                        player_id, life_loss, true,
                    )));
                }
            }
            DamageTarget::Object(object_id) => {
                let Some(obj) = game.object(object_id) else {
                    continue;
                };
                let can_be_damaged = obj.has_card_type(CardType::Creature)
                    || obj.has_card_type(CardType::Planeswalker);
                if !can_be_damaged {
                    continue;
                }
                let is_creature = obj.has_card_type(CardType::Creature);
                if assignment.amount > 0 {
                    game.mark_damage(object_id, assignment.amount);
                    if is_creature {
                        game.record_creature_damaged_by_this_turn(object_id, source);
                    }
                }
            }
        }

        outcomes.push(outcome);
    }

    if outcomes.is_empty() {
        EffectOutcome::count(0)
    } else {
        EffectOutcome::aggregate(outcomes)
    }
}

impl EffectExecutor for DealDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let amount = resolve_value(game, &self.amount, ctx)?.max(0) as u32;

        // Check if this is targeting IteratedPlayer (used in ForEachOpponent)
        // If so, resolve the target from the context's iterated_player
        if let ChooseSpec::Player(PlayerFilter::IteratedPlayer) = &self.target {
            if let Some(player_id) = ctx.iterated_player {
                return Ok(apply_processed_damage_outcome(
                    game,
                    ctx.source,
                    ctx.source_snapshot.as_ref(),
                    DamageTarget::Player(player_id),
                    amount,
                    self.source_is_combat,
                ));
            }
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        if let ChooseSpec::Iterated = &self.target {
            if let Some(object_id) = ctx.iterated_object {
                if let Some(obj) = game.object(object_id) {
                    let can_be_damaged = obj.has_card_type(CardType::Creature)
                        || obj.has_card_type(CardType::Planeswalker);
                    if !can_be_damaged {
                        return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
                    }
                    return Ok(apply_processed_damage_outcome(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                    ));
                }
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            }
            return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
        }

        if let ChooseSpec::AttackedPlayerOrPlaneswalker = &self.target {
            let attacked_target = ctx
                .triggering_event
                .as_ref()
                .and_then(|event| {
                    if let Some(attacked) = event.downcast::<CreatureAttackedEvent>() {
                        return Some(attacked.target);
                    }
                    if let Some(blocked) = event.downcast::<CreatureBecameBlockedEvent>() {
                        return blocked.attack_target;
                    }
                    None
                })
                .or_else(|| ctx.defending_player.map(AttackEventTarget::Player));

            let Some(attacked_target) = attacked_target else {
                return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
            };

            match attacked_target {
                AttackEventTarget::Player(player_id) => {
                    return Ok(apply_processed_damage_outcome(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Player(player_id),
                        amount,
                        self.source_is_combat,
                    ));
                }
                AttackEventTarget::Planeswalker(object_id) => {
                    if !game
                        .object(object_id)
                        .is_some_and(|obj| obj.has_card_type(CardType::Planeswalker))
                    {
                        return Ok(EffectOutcome::from_result(EffectResult::TargetInvalid));
                    }
                    return Ok(apply_processed_damage_outcome(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Object(object_id),
                        amount,
                        self.source_is_combat,
                    ));
                }
            }
        }

        // Handle SourceController - deal damage to the controller of the source (e.g., Ancient Tomb)
        if let ChooseSpec::SourceController = &self.target {
            let controller = ctx.controller;
            return Ok(apply_processed_damage_outcome(
                game,
                ctx.source,
                ctx.source_snapshot.as_ref(),
                DamageTarget::Player(controller),
                amount,
                self.source_is_combat,
            ));
        }

        // Otherwise, use pre-resolved targets from ctx.targets
        for target in &ctx.targets {
            match target {
                ResolvedTarget::Player(player_id) => {
                    return Ok(apply_processed_damage_outcome(
                        game,
                        ctx.source,
                        ctx.source_snapshot.as_ref(),
                        DamageTarget::Player(*player_id),
                        amount,
                        self.source_is_combat,
                    ));
                }
                ResolvedTarget::Object(object_id) => {
                    if let Some(obj) = game.object(*object_id) {
                        let can_be_damaged = obj.has_card_type(CardType::Creature)
                            || obj.has_card_type(CardType::Planeswalker);
                        if !can_be_damaged {
                            continue;
                        }
                        return Ok(apply_processed_damage_outcome(
                            game,
                            ctx.source,
                            ctx.source_snapshot.as_ref(),
                            DamageTarget::Object(*object_id),
                            amount,
                            self.source_is_combat,
                        ));
                    }
                }
            }
        }

        Ok(EffectOutcome::from_result(EffectResult::TargetInvalid))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        // SourceController is deterministic at resolution time (no cast-time selection),
        // but exposing it here keeps downstream wrappers/tests able to inspect
        // what subject this damage effect is bound to.
        if self.target.is_target() || matches!(self.target, ChooseSpec::SourceController) {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "target for damage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::EventKind;
    use crate::ids::{CardId, ObjectId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn new_test_game() -> GameState {
        GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20)
    }

    /// Create a simple source card (like a spell)
    fn make_source_card() -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(1), "Test Source")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build()
    }

    /// Create a simple creature card
    fn make_creature_card(
        card_id: u32,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    /// Create a test game with two players and a creature on the battlefield.
    fn setup_game_with_creature() -> (GameState, ObjectId, ObjectId) {
        let mut game = new_test_game();
        let player_id = game.players[0].id;

        // Create a source object (the spell dealing damage)
        let source_id = game.new_object_id();
        let source_card = make_source_card();
        let source_obj = Object::from_card(source_id, &source_card, player_id, Zone::Stack);
        game.add_object(source_obj);

        // Create a creature to damage
        let creature_id = game.new_object_id();
        let creature_card = make_creature_card(2, "Grizzly Bears", 2, 2);
        let creature_obj =
            Object::from_card(creature_id, &creature_card, player_id, Zone::Battlefield);
        game.add_object(creature_obj);
        game.battlefield.push(creature_id);

        (game, source_id, creature_id)
    }

    #[test]
    fn test_deal_damage_to_creature() {
        let (mut game, source_id, creature_id) = setup_game_with_creature();
        let player_id = game.players[0].id;

        let effect = DealDamageEffect::new(3, ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.damage_on(creature_id), 3);
    }

    #[test]
    fn test_deal_damage_to_player() {
        let mut game = new_test_game();
        let player1_id = game.players[0].id;
        let player2_id = game.players[1].id;

        let source_id = game.new_object_id();
        let source_card = make_source_card();
        let source_obj = Object::from_card(source_id, &source_card, player1_id, Zone::Stack);
        game.add_object(source_obj);

        let initial_life = game.player(player2_id).unwrap().life;

        let effect = DealDamageEffect::new(3, ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player1_id)
            .with_targets(vec![ResolvedTarget::Player(player2_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(3));
        assert_eq!(game.player(player2_id).unwrap().life, initial_life - 3);
    }

    #[test]
    fn test_deal_damage_to_player_emits_life_loss_event() {
        let mut game = new_test_game();
        let player1_id = game.players[0].id;
        let player2_id = game.players[1].id;

        let source_id = game.new_object_id();
        let source_card = make_source_card();
        let source_obj = Object::from_card(source_id, &source_card, player1_id, Zone::Stack);
        game.add_object(source_obj);

        let effect = DealDamageEffect::new(3, ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player1_id)
            .with_targets(vec![ResolvedTarget::Player(player2_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.events.len(), 2);
        assert_eq!(result.events[0].kind(), EventKind::Damage);
        assert_eq!(result.events[1].kind(), EventKind::LifeLoss);

        let life_loss = result.events[1].downcast::<LifeLossEvent>().unwrap();
        assert_eq!(life_loss.player, player2_id);
        assert_eq!(life_loss.amount, 3);
        assert!(life_loss.from_damage);
    }

    #[test]
    fn test_deal_damage_to_player_life_locked_emits_no_life_loss_event() {
        let mut game = new_test_game();
        let player1_id = game.players[0].id;
        let player2_id = game.players[1].id;

        game.cant_effects.life_total_cant_change.insert(player2_id);

        let source_id = game.new_object_id();
        let source_card = make_source_card();
        let source_obj = Object::from_card(source_id, &source_card, player1_id, Zone::Stack);
        game.add_object(source_obj);

        let initial_life = game.player(player2_id).unwrap().life;

        let effect = DealDamageEffect::new(3, ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player1_id)
            .with_targets(vec![ResolvedTarget::Player(player2_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].kind(), EventKind::Damage);
        assert_eq!(game.player(player2_id).unwrap().life, initial_life);
    }

    #[test]
    fn test_deal_variable_damage() {
        let (mut game, source_id, creature_id) = setup_game_with_creature();
        let player_id = game.players[0].id;

        // Deal X damage where X = 5
        let effect = DealDamageEffect::new(Value::X, ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_x(5)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(5));
        assert_eq!(game.damage_on(creature_id), 5);
    }

    #[test]
    fn test_deal_damage_no_target() {
        let mut game = new_test_game();
        let player_id = game.players[0].id;

        let source_id = game.new_object_id();
        let source_card = make_source_card();
        let source_obj = Object::from_card(source_id, &source_card, player_id, Zone::Stack);
        game.add_object(source_obj);

        let effect = DealDamageEffect::new(3, ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player_id);
        // No targets set

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::TargetInvalid);
    }

    #[test]
    fn test_deal_zero_damage() {
        let (mut game, source_id, creature_id) = setup_game_with_creature();
        let player_id = game.players[0].id;

        let effect = DealDamageEffect::new(0, ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        // 0 damage is still counted as the effect executing
        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.damage_on(creature_id), 0);
    }

    #[test]
    fn test_deal_damage_to_iterated_player() {
        let mut game = new_test_game();
        let player1_id = game.players[0].id;
        let player2_id = game.players[1].id;

        let source_id = game.new_object_id();
        let source_card = make_source_card();
        let source_obj = Object::from_card(source_id, &source_card, player1_id, Zone::Stack);
        game.add_object(source_obj);

        let initial_life = game.player(player2_id).unwrap().life;

        // Use IteratedPlayer target (as in ForEachOpponent)
        let effect = DealDamageEffect::new(2, ChooseSpec::Player(PlayerFilter::IteratedPlayer));
        let mut ctx = ExecutionContext::new_default(source_id, player1_id);
        ctx.iterated_player = Some(player2_id);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(2));
        assert_eq!(game.player(player2_id).unwrap().life, initial_life - 2);
    }

    #[test]
    fn test_deal_damage_negative_becomes_zero() {
        let (mut game, source_id, creature_id) = setup_game_with_creature();
        let player_id = game.players[0].id;

        // Negative damage should be treated as 0
        let effect = DealDamageEffect::new(Value::Fixed(-5), ChooseSpec::AnyTarget);
        let mut ctx = ExecutionContext::new_default(source_id, player_id)
            .with_targets(vec![ResolvedTarget::Object(creature_id)]);

        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.result, EffectResult::Count(0));
        assert_eq!(game.damage_on(creature_id), 0);
    }

    #[test]
    fn test_deal_damage_is_debug() {
        let effect = DealDamageEffect::new(3, ChooseSpec::AnyTarget);
        let debug_str = format!("{:?}", effect);
        assert!(debug_str.contains("DealDamageEffect"));
        assert!(debug_str.contains("amount"));
    }
}

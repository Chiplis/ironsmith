//! Set life total effect implementation.

use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError, SimultaneousEffectProposal};
use crate::game_state::GameState;
use crate::target::PlayerFilter;

/// Effect that sets a player's life total to a specific value.
///
/// This is different from gaining or losing life:
/// - If the new total is higher, the player gains the difference
/// - If the new total is lower, the player loses the difference
/// - Used by cards like "Your life total becomes 10"
///
/// # Fields
///
/// * `amount` - The life total to set (can be fixed or variable)
/// * `player` - Which player's life total changes
///
/// # Example
///
/// ```ignore
/// // Set life total to 10 (like Sorin Markov's ability)
/// let effect = SetLifeTotalEffect {
///     amount: Value::Fixed(10),
///     player: PlayerFilter::Opponent,
/// };
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct SetLifeTotalEffect {
    /// The life total to set.
    pub amount: Value,
    /// Which player's life total changes.
    pub player: PlayerFilter,
}

#[derive(Debug)]
struct SetLifeTotalProposal {
    player: crate::ids::PlayerId,
    amount: i32,
    current: i32,
    can_change: bool,
    prepared: Option<crate::events::processing::TraitEventResult>,
    provenance: crate::provenance::ProvNodeId,
}

impl SimultaneousEffectProposal for SetLifeTotalProposal {
    fn prepare_original(&mut self, game: &mut GameState, ctx: &mut ExecutionContext) -> Result<(), ExecutionError> {
        if self.amount == self.current || !self.can_change { return Ok(()); }
        let amount = self.amount.abs_diff(self.current);
        let event = if self.amount > self.current {
            crate::events::Event::new_with_provenance(crate::events::LifeGainEvent::new(self.player, amount).with_source(ctx.source), self.provenance)
        } else {
            crate::events::Event::new_with_provenance(crate::events::LifeLossEvent::from_effect(self.player, amount), self.provenance)
        };
        self.prepared = Some(super::life_change::prepare_life_change(game, ctx, event)?);
        Ok(())
    }
    fn commit_original(mut self: Box<Self>, game: &mut GameState, ctx: &mut ExecutionContext)
        -> Result<crate::effects::SimultaneousEffectCommit, ExecutionError>
    {
        if self.amount == self.current { return Ok(crate::effects::SimultaneousEffectCommit::finished(EffectOutcome::resolved())); }
        if !self.can_change { return Ok(crate::effects::SimultaneousEffectCommit::finished(EffectOutcome::prevented())); }
        if self.prepared.is_none() { self.prepare_original(game, ctx)?; }
        super::life_change::commit_prepared_life_original(game, ctx, self.prepared.take().expect("life proposal prepared"))
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        apply_set_life_total(
            game,
            self.player,
            self.amount,
            self.current,
            self.can_change,
            self.provenance,
            ctx,
        )
    }
}

fn apply_set_life_total(
    game: &mut GameState,
    player_id: crate::ids::PlayerId,
    amount: i32,
    current: i32,
    can_change: bool,
    provenance: crate::provenance::ProvNodeId,
    ctx: &mut ExecutionContext,
) -> Result<EffectOutcome, ExecutionError> {
    if amount == current {
        return Ok(EffectOutcome::resolved());
    }

    if !can_change {
        return Ok(EffectOutcome::prevented());
    }
    let difference = amount.abs_diff(current);
    let event = if amount > current {
        crate::events::Event::new_with_provenance(
            crate::events::LifeGainEvent::new(player_id, difference).with_source(ctx.source), provenance,
        )
    } else {
        crate::events::Event::new_with_provenance(
            crate::events::LifeLossEvent::from_effect(player_id, difference), provenance,
        )
    };
    super::life_change::execute_life_change(game, ctx, event)
}

impl SetLifeTotalEffect {
    /// Create a new set life total effect.
    pub fn new(amount: impl Into<Value>, player: PlayerFilter) -> Self {
        Self {
            amount: amount.into(),
            player,
        }
    }

    /// Create an effect that sets your life total.
    pub fn you(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::You)
    }

    /// Create an effect that sets an opponent's life total.
    pub fn opponent(amount: impl Into<Value>) -> Self {
        Self::new(amount, PlayerFilter::Opponent)
    }
}

impl EffectExecutor for SetLifeTotalEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?;

        let current = game.player(player_id).map(|p| p.life).unwrap_or(amount);
        let can_change = game.can_change_life_total(player_id);
        apply_set_life_total(
            game,
            player_id,
            amount,
            current,
            can_change,
            ctx.provenance,
            ctx,
        )
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
        let player = resolve_player_filter(game, &self.player, ctx)?;
        let amount = resolve_value(game, &self.amount, ctx)?;
        let current = game
            .player(player)
            .map(|candidate| candidate.life)
            .unwrap_or(amount);
        Ok(Box::new(SetLifeTotalProposal {
            player,
            amount,
            current,
            can_change: game.can_change_life_total(player),
            prepared: None,
            provenance: ctx.provenance,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventKind;
    use crate::ids::PlayerId;

    #[test]
    fn set_life_total_emits_life_gain_event_when_total_increases() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice exists").life = 10;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = SetLifeTotalEffect::you(15)
            .execute(&mut game, &mut ctx)
            .expect("set life total should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 15);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::LifeGain),
            "raising life total should emit a LifeGainEvent"
        );
    }

    #[test]
    fn set_life_total_emits_life_loss_event_when_total_decreases() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice).expect("alice exists").life = 10;

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let outcome = SetLifeTotalEffect::you(4)
            .execute(&mut game, &mut ctx)
            .expect("set life total should resolve");

        assert_eq!(game.player(alice).expect("alice exists").life, 4);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == EventKind::LifeLoss),
            "lowering life total should emit a LifeLossEvent"
        );
    }
}

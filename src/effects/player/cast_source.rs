//! Cast-source effect implementation.
//!
//! Casts the source card of the resolving effect/ability.

use crate::alternative_cast::CastingMethod;
use crate::cost::OptionalCostsPaid;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::zone::Zone;

use super::runtime_helpers::with_spell_cast_event;

/// Effect that casts the source card immediately.
#[derive(Debug, Clone, PartialEq)]
pub struct CastSourceEffect {
    pub without_paying_mana_cost: bool,
    pub require_exile: bool,
}

impl CastSourceEffect {
    /// Create a new cast-source effect.
    pub fn new() -> Self {
        Self {
            without_paying_mana_cost: false,
            require_exile: false,
        }
    }

    /// Cast without paying mana cost.
    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    /// Require the source card to be in exile.
    pub fn require_exile(mut self) -> Self {
        self.require_exile = true;
        self
    }
}

impl EffectExecutor for CastSourceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let source_id = ctx.source;
        let Some(source_obj) = game.object(source_id) else {
            return Ok(EffectOutcome::target_invalid());
        };

        if source_obj.is_land() {
            return Ok(EffectOutcome::target_invalid());
        }
        if self.require_exile && source_obj.zone != Zone::Exile {
            return Ok(EffectOutcome::target_invalid());
        }

        let from_zone = source_obj.zone;
        let mana_cost = source_obj.mana_cost.clone();
        let stable_id = source_obj.stable_id;
        let source_name = source_obj.name.clone();
        let suspend_alternative_index = if from_zone == Zone::Exile {
            source_obj
                .alternative_casts
                .iter()
                .position(|method| method.suspend_spec().is_some())
        } else {
            None
        };
        let x_value = mana_cost
            .as_ref()
            .and_then(|cost| if cost.has_x() { Some(0u32) } else { None });

        if !self.without_paying_mana_cost
            && let Some(cost) = mana_cost.as_ref()
        {
            let effective_cost = crate::decision::calculate_effective_mana_cost(
                game,
                ctx.controller,
                source_obj,
                cost,
            );
            if !game.try_pay_mana_cost_with_reason(
                ctx.controller,
                Some(source_id),
                &effective_cost,
                0,
                crate::costs::PaymentReason::CastSpell,
            ) {
                return Ok(EffectOutcome::impossible());
            }
        }

        let Some(new_id) = game.move_object_by_effect(source_id, Zone::Stack) else {
            return Ok(EffectOutcome::impossible());
        };

        if let Some(obj) = game.object_mut(new_id) {
            obj.x_value = x_value;
        }

        let stack_entry = StackEntry {
            object_id: new_id,
            controller: ctx.controller,
            provenance: ctx.provenance,
            targets: vec![],
            target_assignments: vec![],
            x_value,
            ability_effects: None,
            is_ability: false,
            casting_method: CastingMethod::PlayFrom {
                source: source_id,
                zone: from_zone,
                use_alternative: suspend_alternative_index,
            },
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            chosen_player: None,
            saga_final_chapter_source: None,
            source_stable_id: Some(stable_id),
            source_snapshot: None,
            source_name: Some(source_name),
            triggering_event: None,
            intervening_if: None,
            keyword_payment_contributions: vec![],
            crew_contributors: vec![],
            saddle_contributors: vec![],
            chosen_modes: None,
            tagged_objects: std::collections::HashMap::new(),
        };

        game.push_to_stack(stack_entry);
        Ok(with_spell_cast_event(
            EffectOutcome::with_objects(vec![new_id]),
            game,
            new_id,
            ctx.controller,
            from_zone,
            ctx.provenance,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::SelectFirstDecisionMaker;
    use crate::effect::{OutcomeStatus, OutcomeValue};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn cast_source_requires_exile_when_requested() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Suspend Probe")
                .card_types(vec![CardType::Sorcery])
                .build(),
            alice,
            Zone::Hand,
        );

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        let outcome = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .execute(&mut game, &mut ctx)
            .expect("cast source should execute");

        assert_eq!(outcome.status, OutcomeStatus::TargetInvalid);
        assert!(game.stack.is_empty());
    }

    #[test]
    fn cast_source_free_cast_sets_x_to_zero_and_emits_spell_cast_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source_id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "X Fireball")
                .card_types(vec![CardType::Sorcery])
                .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::X, ManaSymbol::Red]))
                .build(),
            alice,
            Zone::Exile,
        );

        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source_id, alice, &mut dm);
        let outcome = CastSourceEffect::new()
            .without_paying_mana_cost()
            .require_exile()
            .execute(&mut game, &mut ctx)
            .expect("free cast from exile should resolve");

        let OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected the source card to move to the stack");
        };
        let cast_id = ids[0];

        assert_eq!(outcome.status, OutcomeStatus::Succeeded);
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == crate::events::EventKind::SpellCast),
            "cast-source should emit a SpellCast event"
        );

        let stack_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == cast_id)
            .expect("cast object should be on the stack");
        assert_eq!(stack_entry.x_value, Some(0));

        let spell = game.object(cast_id).expect("stack spell should exist");
        assert_eq!(spell.zone, Zone::Stack);
        assert_eq!(spell.x_value, Some(0));
    }
}

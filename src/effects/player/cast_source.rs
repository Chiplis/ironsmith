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
                use_alternative: None,
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

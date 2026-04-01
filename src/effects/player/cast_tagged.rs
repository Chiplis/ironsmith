//! Cast a previously tagged card effect implementation.
//!
//! This effect is used for one-shot "You may cast it" patterns where a prior
//! effect tagged a specific card (often from exile). The cast is performed
//! immediately during resolution and returns an outcome that can be used by
//! subsequent "If you don't" clauses.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::zones::{
    BattlefieldEntryOptions, BattlefieldEntryOutcome, move_to_battlefield_with_options,
};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry};
use crate::mana::ManaCost;
use crate::tag::TagKey;
use crate::zone::Zone;

use super::runtime_helpers::{queue_effect_driven_land_play, with_spell_cast_event};

/// Effect that casts a tagged card immediately.
#[derive(Debug, Clone, PartialEq)]
pub struct CastTaggedEffect {
    pub tag: TagKey,
    pub allow_land: bool,
    pub as_copy: bool,
    pub without_paying_mana_cost: bool,
    pub cost_reduction: Option<ManaCost>,
}

impl CastTaggedEffect {
    /// Create a new cast-tagged effect.
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self {
            tag: tag.into(),
            allow_land: false,
            as_copy: false,
            without_paying_mana_cost: false,
            cost_reduction: None,
        }
    }

    /// Allow playing a tagged land (best-effort, ignores ownership restrictions).
    pub fn allow_land(mut self) -> Self {
        self.allow_land = true;
        self
    }

    /// Cast a copy of the tagged object instead of the tagged object itself.
    pub fn as_copy(mut self) -> Self {
        self.as_copy = true;
        self
    }

    /// Cast without paying mana cost.
    pub fn without_paying_mana_cost(mut self) -> Self {
        self.without_paying_mana_cost = true;
        self
    }

    /// Reduce the generated spell's mana cost during this immediate cast.
    pub fn cost_reduction(mut self, reduction: ManaCost) -> Self {
        self.cost_reduction = Some(reduction);
        self
    }
}

impl EffectExecutor for CastTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::alternative_cast::CastingMethod;
        use crate::cost::OptionalCostsPaid;

        let Some(snapshot) = ctx.get_tagged(self.tag.as_str()) else {
            return Ok(EffectOutcome::target_invalid());
        };

        let mut object_id = snapshot.object_id;
        if game.object(object_id).is_none() {
            if let Some(found) = game.find_object_by_stable_id(snapshot.stable_id) {
                object_id = found;
            } else {
                return Ok(EffectOutcome::target_invalid());
            }
        }

        let (is_land, mana_cost, from_zone, card_name, stable_id) = {
            let Some(obj) = game.object(object_id) else {
                return Ok(EffectOutcome::target_invalid());
            };
            (
                obj.is_land(),
                obj.mana_cost.clone(),
                obj.zone,
                obj.name.clone(),
                obj.stable_id,
            )
        };
        let x_value = mana_cost
            .as_ref()
            .and_then(|cost| if cost.has_x() { Some(0u32) } else { None });

        if self.as_copy {
            let caster = ctx.controller;
            let copy_id = game.new_object_id();

            let source_obj = match game.object(object_id) {
                Some(obj) => obj.clone(),
                None => return Ok(EffectOutcome::target_invalid()),
            };
            let mut copy_obj = crate::object::Object::token_copy_of(&source_obj, copy_id, caster);
            copy_obj.controller = caster;
            copy_obj.x_value = x_value;

            if is_land {
                if !self.allow_land {
                    return Ok(EffectOutcome::target_invalid());
                }
                copy_obj.zone = Zone::Command;
                game.add_object(copy_obj);
                return match move_to_battlefield_with_options(
                    game,
                    ctx,
                    copy_id,
                    BattlefieldEntryOptions::specific(caster, false),
                ) {
                    BattlefieldEntryOutcome::Moved(new_id) => {
                        queue_effect_driven_land_play(game, ctx, new_id, caster, from_zone);
                        Ok(EffectOutcome::with_objects(vec![new_id]))
                    }
                    BattlefieldEntryOutcome::Prevented => {
                        game.remove_object(copy_id);
                        Ok(EffectOutcome::impossible())
                    }
                };
            }

            if !self.without_paying_mana_cost
                && let Some(cost) = mana_cost.as_ref()
            {
                let mut effective_cost =
                    crate::decision::calculate_effective_mana_cost(game, caster, &copy_obj, cost);
                if let Some(reduction) = self.cost_reduction.as_ref() {
                    effective_cost = crate::decision::reduce_mana_cost(&effective_cost, reduction);
                }
                if !game.try_pay_mana_cost_with_reason(
                    caster,
                    Some(copy_id),
                    &effective_cost,
                    0,
                    crate::costs::PaymentReason::CastSpell,
                ) {
                    return Ok(EffectOutcome::impossible());
                }
            }

            copy_obj.zone = Zone::Stack;
            game.add_object(copy_obj);

            let mut stack_entry = StackEntry::new(copy_id, caster);
            stack_entry.x_value = x_value;
            stack_entry.source_stable_id = Some(stable_id);
            stack_entry.source_name = Some(card_name);
            game.push_to_stack(stack_entry);
            return Ok(with_spell_cast_event(
                EffectOutcome::with_objects(vec![copy_id]),
                game,
                copy_id,
                caster,
                from_zone,
                ctx.provenance,
            ));
        }

        if is_land {
            if !self.allow_land {
                return Ok(EffectOutcome::target_invalid());
            }

            return match move_to_battlefield_with_options(
                game,
                ctx,
                object_id,
                BattlefieldEntryOptions::specific(ctx.controller, false),
            ) {
                BattlefieldEntryOutcome::Moved(new_id) => {
                    queue_effect_driven_land_play(game, ctx, new_id, ctx.controller, from_zone);
                    Ok(EffectOutcome::with_objects(vec![new_id]))
                }
                BattlefieldEntryOutcome::Prevented => Ok(EffectOutcome::impossible()),
            };
        }

        let caster = ctx.controller;
        if !self.without_paying_mana_cost
            && let Some(cost) = mana_cost.as_ref()
        {
            let Some(cast_object) = game.object(object_id) else {
                return Ok(EffectOutcome::impossible());
            };
            let effective_cost =
                crate::decision::calculate_effective_mana_cost(game, caster, cast_object, cost);
            if !game.try_pay_mana_cost_with_reason(
                caster,
                Some(object_id),
                &effective_cost,
                0,
                crate::costs::PaymentReason::CastSpell,
            ) {
                return Ok(EffectOutcome::impossible());
            }
        }

        let Some(new_id) = game.move_object_by_effect(object_id, Zone::Stack) else {
            return Ok(EffectOutcome::impossible());
        };
        if let Some(obj) = game.object_mut(new_id) {
            obj.x_value = x_value;
        }

        let casting_method = if from_zone == Zone::Hand {
            CastingMethod::Normal
        } else {
            CastingMethod::PlayFrom {
                source: ctx.source,
                zone: from_zone,
                use_alternative: None,
            }
        };

        let stack_entry = StackEntry {
            object_id: new_id,
            controller: caster,
            provenance: ctx.provenance,
            targets: vec![],
            target_assignments: vec![],
            x_value,
            ability_effects: None,
            is_ability: false,
            casting_method,
            optional_costs_paid: OptionalCostsPaid::default(),
            defending_player: None,
            chosen_player: None,
            saga_final_chapter_source: None,
            source_stable_id: Some(stable_id),
            source_snapshot: None,
            source_name: Some(card_name),
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
            caster,
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
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::types::CardType;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn cast_tagged_spell_emits_spell_cast_event_and_bookkeeping() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Tagged Spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("tagged card"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let outcome = CastTaggedEffect::new("it")
            .without_paying_mana_cost()
            .execute(&mut game, &mut ctx)
            .expect("cast tagged should resolve");

        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected cast tagged to create a stack object");
        };
        let cast_id = ids[0];
        for event in &outcome.events {
            game.stage_turn_history_event(event);
        }
        assert!(game.stack.iter().any(|entry| entry.object_id == cast_id));
        assert_eq!(game.turn_history.spells_cast_by_player(alice), 1);
        assert!(game.turn_history.spell_cast_order(cast_id).is_some());
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == crate::events::EventKind::SpellCast),
            "cast-tagged spells should emit SpellCastEvent"
        );
    }

    #[test]
    fn cast_tagged_land_emits_land_play_and_etb_events() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Tagged Land")
            .card_types(vec![CardType::Land])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("tagged land"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let outcome = CastTaggedEffect::new("it")
            .allow_land()
            .execute(&mut game, &mut ctx)
            .expect("play tagged land should resolve");

        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected played land to move to battlefield");
        };
        let land_id = ids[0];
        assert!(game.battlefield.contains(&land_id));
        assert_eq!(
            game.player(alice)
                .expect("alice exists")
                .lands_played_this_turn,
            1
        );

        let pending = game.take_pending_trigger_events();
        assert!(
            pending
                .iter()
                .any(|event| event.kind() == crate::events::EventKind::EnterBattlefield),
            "playing a tagged land should queue an ETB event"
        );
        assert!(
            pending
                .iter()
                .any(|event| event.kind() == crate::events::EventKind::LandPlayed),
            "playing a tagged land should queue a LandPlayedEvent"
        );
    }

    #[test]
    fn cast_tagged_land_is_invalid_without_play_permission() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Tagged Land")
            .card_types(vec![CardType::Land])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("tagged land"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let outcome = CastTaggedEffect::new("it")
            .execute(&mut game, &mut ctx)
            .expect("cast tagged should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::TargetInvalid);
        assert!(game.stack.is_empty());
        assert!(!game.battlefield.contains(&exiled_id));
    }

    #[test]
    fn cast_tagged_copy_applies_inline_cost_reduction() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Blue, 2);

        let card = CardBuilder::new(CardId::new(), "Reduced Copy Spell")
            .card_types(vec![CardType::Instant])
            .mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Generic(3),
                ManaSymbol::Blue,
            ]))
            .build();
        let hand_id = game.create_object_from_card(&card, alice, Zone::Hand);
        let snapshot =
            ObjectSnapshot::from_object(game.object(hand_id).expect("tagged card"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("it"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let outcome = CastTaggedEffect::new("it")
            .as_copy()
            .cost_reduction(ManaCost::from_symbols(vec![ManaSymbol::Generic(2)]))
            .execute(&mut game, &mut ctx)
            .expect("cast tagged should resolve");

        assert!(
            outcome.status.is_success(),
            "expected reduced copy cast to succeed"
        );
        assert_eq!(
            game.player(alice).expect("alice exists").mana_pool.blue,
            0,
            "expected the reduced cost to spend exactly the available mana"
        );
        assert_eq!(
            game.stack.len(),
            1,
            "expected the copied spell on the stack"
        );
    }
}

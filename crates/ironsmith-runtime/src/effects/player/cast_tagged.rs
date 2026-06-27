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
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::{GameState, StackEntry, Target, TargetAssignment};
use crate::zone::Zone;
pub use ironsmith_core::CastTaggedEffect;

use super::runtime_helpers::{queue_effect_driven_land_play, with_spell_cast_event};

fn build_target_assignments_for_cast_tagged_copy(
    requirements: &[crate::decision::TargetRequirement],
    targets: &[Target],
) -> Option<Vec<TargetAssignment>> {
    let requirement_contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
            },
        )
        .collect::<Vec<_>>();
    let ranges = crate::targeting::assigned_target_ranges(&requirement_contexts, targets)?;
    Some(
        requirements
            .iter()
            .zip(ranges)
            .map(|(requirement, range)| TargetAssignment {
                spec: requirement.spec.clone(),
                range,
            })
            .collect(),
    )
}

fn choose_targets_for_cast_tagged_spell(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    source_id: crate::ids::ObjectId,
    caster: crate::ids::PlayerId,
    card_name: String,
) -> Option<(Vec<Target>, Vec<TargetAssignment>)> {
    let requirements = game
        .object(source_id)
        .and_then(|obj| obj.spell_effect.as_ref())
        .map(|program| {
            crate::game_loop::extract_target_requirements_from_program_with_modes(
                game,
                program,
                caster,
                Some(source_id),
                None,
            )
        })
        .unwrap_or_default();
    let requirement_contexts = requirements
        .iter()
        .map(
            |requirement| crate::decisions::context::TargetRequirementContext {
                description: requirement.description.clone(),
                legal_targets: requirement.legal_targets.clone(),
                legal_target_sets: requirement.legal_target_sets.clone(),
                min_targets: requirement.min_targets,
                max_targets: requirement.max_targets,
            },
        )
        .collect::<Vec<_>>();
    let selected_targets = if requirement_contexts.is_empty() {
        Vec::new()
    } else {
        let targets_ctx = crate::decisions::context::TargetsContext::new(
            caster,
            source_id,
            card_name,
            requirement_contexts,
        );
        let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        crate::targeting::normalize_targets_for_requirements(&targets_ctx.requirements, proposed)?
    };
    let target_assignments =
        build_target_assignments_for_cast_tagged_copy(&requirements, &selected_targets)?;
    Some((selected_targets, target_assignments))
}

fn choose_and_pay_optional_costs_for_cast_tagged_spell(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    source_id: crate::ids::ObjectId,
    caster: crate::ids::PlayerId,
) -> Result<Option<crate::cost::OptionalCostsPaid>, ExecutionError> {
    let Some((card_name, optional_costs)) = game
        .object(source_id)
        .map(|obj| (obj.name.clone(), obj.optional_costs.clone()))
    else {
        return Ok(Some(crate::cost::OptionalCostsPaid::default()));
    };
    if optional_costs.is_empty() {
        return Ok(Some(crate::cost::OptionalCostsPaid::default()));
    }

    let options = optional_costs
        .iter()
        .enumerate()
        .map(|(index, opt_cost)| {
            let affordable = if let Some(mana_cost) = opt_cost.cost.mana_cost() {
                let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
                    caster,
                    Some(source_id),
                    mana_cost,
                    crate::costs::PaymentReason::CastSpell,
                );
                crate::decision::can_potentially_pay(game, caster, &adjusted_cost, 0)
            } else {
                crate::cost::can_pay_cost_with_reason(
                    game,
                    source_id,
                    caster,
                    &opt_cost.cost,
                    crate::costs::PaymentReason::CastSpell,
                )
                .is_ok()
            };
            let cost_description = opt_cost
                .cost
                .mana_cost()
                .map(|mana| mana.mana_value().to_string())
                .unwrap_or_else(|| opt_cost.cost.display());
            crate::decisions::context::SelectableOption::with_legality(
                index,
                format!("{}: {}", opt_cost.display_label(), cost_description),
                affordable,
            )
        })
        .collect::<Vec<_>>();
    let decision_ctx = crate::decisions::context::SelectOptionsContext::new(
        caster,
        Some(source_id),
        format!("Choose optional costs for {card_name}"),
        options,
        0,
        if optional_costs.iter().any(|cost| cost.repeatable) {
            64
        } else {
            optional_costs.len()
        },
    );
    let selected = ctx.decision_maker.decide_options(game, &decision_ctx);
    if ctx.decision_maker.awaiting_choice() {
        return Ok(None);
    }

    let mut paid = crate::cost::OptionalCostsPaid::from_costs(&optional_costs);
    for index in selected {
        let Some(optional_cost) = optional_costs.get(index) else {
            continue;
        };
        pay_total_cost_for_cast_tagged_spell(game, ctx, source_id, caster, &optional_cost.cost)?;
        paid.pay_times(index, 1);
    }
    if let Some(obj) = game.object_mut(source_id) {
        obj.optional_costs_paid = paid.clone();
    }
    Ok(Some(paid))
}

fn pay_total_cost_for_cast_tagged_spell(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    source_id: crate::ids::ObjectId,
    caster: crate::ids::PlayerId,
    total_cost: &crate::cost::TotalCost,
) -> Result<(), ExecutionError> {
    if let Some(mana_cost) = total_cost.mana_cost() {
        let adjusted_cost = game.adjust_mana_cost_for_payment_reason(
            caster,
            Some(source_id),
            mana_cost,
            crate::costs::PaymentReason::CastSpell,
        );
        auto_add_mana_from_basic_lands_for_cast_tagged_cost(
            game,
            source_id,
            caster,
            &adjusted_cost,
        );
        if !game.try_pay_mana_cost_with_reason(
            caster,
            Some(source_id),
            &adjusted_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ) {
            return Err(ExecutionError::Impossible(
                "could not pay optional mana cost".to_string(),
            ));
        }
        return Ok(());
    }

    for cost in total_cost.costs() {
        let mut cost_ctx = crate::costs::CostContext::new(source_id, caster, ctx.decision_maker)
            .with_reason(crate::costs::PaymentReason::CastSpell)
            .with_provenance(ctx.provenance);
        match cost
            .pay(game, &mut cost_ctx)
            .map_err(|err| ExecutionError::Impossible(err.to_string()))?
        {
            crate::costs::CostPaymentResult::Paid => {}
            crate::costs::CostPaymentResult::NeedsChoice(description) => {
                return Err(ExecutionError::Impossible(format!(
                    "optional cost requires an unsupported staged choice: {description}"
                )));
            }
        }
    }
    Ok(())
}

fn auto_add_mana_from_basic_lands_for_cast_tagged_cost(
    game: &mut GameState,
    source_id: crate::ids::ObjectId,
    payer: crate::ids::PlayerId,
    mana_cost: &crate::mana::ManaCost,
) {
    if game.can_pay_mana_cost_with_reason(
        payer,
        Some(source_id),
        mana_cost,
        0,
        crate::costs::PaymentReason::CastSpell,
    ) {
        return;
    }

    let mut desired = Vec::new();
    for pip in mana_cost.pips() {
        if let Some(symbol) = pip.iter().find_map(|symbol| match symbol {
            crate::mana::ManaSymbol::White
            | crate::mana::ManaSymbol::Blue
            | crate::mana::ManaSymbol::Black
            | crate::mana::ManaSymbol::Red
            | crate::mana::ManaSymbol::Green => Some(*symbol),
            _ => None,
        }) {
            desired.push(Some(symbol));
        } else {
            let count = pip
                .iter()
                .find_map(|symbol| match symbol {
                    crate::mana::ManaSymbol::Generic(amount) => Some(*amount as usize),
                    crate::mana::ManaSymbol::Colorless | crate::mana::ManaSymbol::Snow => Some(1),
                    _ => None,
                })
                .unwrap_or(1);
            for _ in 0..count {
                desired.push(None);
            }
        }
    }

    for wanted in desired {
        if game.can_pay_mana_cost_with_reason(
            payer,
            Some(source_id),
            mana_cost,
            0,
            crate::costs::PaymentReason::CastSpell,
        ) {
            break;
        }
        let Some((land_id, symbol)) = game.battlefield.iter().find_map(|id| {
            let object = game.object(*id)?;
            if game.controller_of(object) != payer || game.is_tapped(*id) {
                return None;
            }
            let symbol = basic_land_mana_symbol(object)?;
            if wanted.is_none_or(|wanted| wanted == symbol) {
                Some((*id, symbol))
            } else {
                None
            }
        }) else {
            continue;
        };
        game.tap(land_id);
        if let Some(player) = game.player_mut(payer) {
            player.mana_pool.add(symbol, 1);
        }
    }
}

fn basic_land_mana_symbol(object: &crate::object::Object) -> Option<crate::mana::ManaSymbol> {
    if object.subtypes.contains(&crate::types::Subtype::Plains) || object.name == "Plains" {
        Some(crate::mana::ManaSymbol::White)
    } else if object.subtypes.contains(&crate::types::Subtype::Island) || object.name == "Island" {
        Some(crate::mana::ManaSymbol::Blue)
    } else if object.subtypes.contains(&crate::types::Subtype::Swamp) || object.name == "Swamp" {
        Some(crate::mana::ManaSymbol::Black)
    } else if object.subtypes.contains(&crate::types::Subtype::Mountain)
        || object.name == "Mountain"
    {
        Some(crate::mana::ManaSymbol::Red)
    } else if object.subtypes.contains(&crate::types::Subtype::Forest) || object.name == "Forest" {
        Some(crate::mana::ManaSymbol::Green)
    } else {
        None
    }
}

/// Effect that casts a tagged card immediately.
impl EffectExecutor for CastTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        use crate::alternative_cast::CastingMethod;
        use crate::effects::helpers::resolve_player_filter;

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

        let caster = resolve_player_filter(game, &self.player, ctx)?;

        if self.as_copy {
            let copy_id = game.new_object_id();

            let source_obj = match game.object(object_id) {
                Some(obj) => obj.clone(),
                None => return Ok(EffectOutcome::target_invalid()),
            };
            let mut copy_obj = crate::object::Object::token_copy_of(&source_obj, copy_id, caster);
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

            copy_obj.zone = Zone::Stack;
            game.add_object(copy_obj);

            let requirements = game
                .object(copy_id)
                .and_then(|obj| obj.spell_effect.as_ref())
                .map(|program| {
                    crate::game_loop::extract_target_requirements_from_program_with_modes(
                        game,
                        program,
                        caster,
                        Some(copy_id),
                        None,
                    )
                })
                .unwrap_or_default();
            let requirement_contexts = requirements
                .iter()
                .map(
                    |requirement| crate::decisions::context::TargetRequirementContext {
                        description: requirement.description.clone(),
                        legal_targets: requirement.legal_targets.clone(),
                        legal_target_sets: requirement.legal_target_sets.clone(),
                        min_targets: requirement.min_targets,
                        max_targets: requirement.max_targets,
                    },
                )
                .collect::<Vec<_>>();
            let selected_targets = if requirement_contexts.is_empty() {
                Vec::new()
            } else {
                let targets_ctx = crate::decisions::context::TargetsContext::new(
                    caster,
                    copy_id,
                    card_name.clone(),
                    requirement_contexts,
                );
                let proposed = ctx.decision_maker.decide_targets(game, &targets_ctx);
                if ctx.decision_maker.awaiting_choice() {
                    game.remove_object(copy_id);
                    return Ok(EffectOutcome::count(0));
                }
                let Some(normalized) = crate::targeting::normalize_targets_for_requirements(
                    &targets_ctx.requirements,
                    proposed,
                ) else {
                    game.remove_object(copy_id);
                    return Ok(EffectOutcome::impossible());
                };
                normalized
            };
            let Some(target_assignments) =
                build_target_assignments_for_cast_tagged_copy(&requirements, &selected_targets)
            else {
                game.remove_object(copy_id);
                return Ok(EffectOutcome::impossible());
            };

            if !self.without_paying_mana_cost
                && let Some(cost) = mana_cost.as_ref()
            {
                let Some(copy_obj) = game.object(copy_id) else {
                    return Ok(EffectOutcome::target_invalid());
                };
                let mut effective_cost =
                    crate::decision::calculate_effective_mana_cost_with_chosen_targets_for_casting_method(
                        game,
                        caster,
                        copy_obj,
                        cost,
                        &selected_targets,
                        &CastingMethod::Normal,
                    );
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
                    game.remove_object(copy_id);
                    return Ok(EffectOutcome::impossible());
                }
            }

            let mut stack_entry = StackEntry::new(copy_id, caster);
            stack_entry.x_value = x_value;
            stack_entry.source_stable_id = Some(stable_id);
            stack_entry.source_name = Some(card_name);
            stack_entry.targets = selected_targets;
            stack_entry.target_assignments = target_assignments;
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
                BattlefieldEntryOptions::specific(caster, false),
            ) {
                BattlefieldEntryOutcome::Moved(new_id) => {
                    queue_effect_driven_land_play(game, ctx, new_id, caster, from_zone);
                    Ok(EffectOutcome::with_objects(vec![new_id]))
                }
                BattlefieldEntryOutcome::Prevented => Ok(EffectOutcome::impossible()),
            };
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

        let Some(optional_costs_paid) =
            choose_and_pay_optional_costs_for_cast_tagged_spell(game, ctx, object_id, caster)?
        else {
            return Ok(EffectOutcome::count(0));
        };

        let Some((selected_targets, target_assignments)) =
            choose_targets_for_cast_tagged_spell(game, ctx, object_id, caster, card_name.clone())
        else {
            return Ok(EffectOutcome::count(0));
        };

        if !self.without_paying_mana_cost
            && let Some(cost) = mana_cost.as_ref()
        {
            let Some(cast_object) = game.object(object_id) else {
                return Ok(EffectOutcome::impossible());
            };
            let mut effective_cost =
                crate::decision::calculate_effective_mana_cost_with_chosen_targets_for_casting_method(
                    game,
                    caster,
                    cast_object,
                    cost,
                    &selected_targets,
                    &casting_method,
                );
            if let Some(reduction) = self.cost_reduction.as_ref() {
                effective_cost = crate::decision::reduce_mana_cost(&effective_cost, reduction);
            }
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

        let stack_entry = StackEntry {
            object_id: new_id,
            controller: caster,
            provenance: ctx.provenance,
            targets: selected_targets,
            target_assignments,
            x_value,
            activation_cost_has_x: false,
            activation_cost_has_tap: false,
            ability_effects: None,
            mana_usage_restrictions: Vec::new(),
            mana_source_chosen_creature_type: None,
            is_ability: false,
            casting_method,
            optional_costs_paid,
            defending_player: None,
            chosen_player: None,
            chapter_ability_source: None,
            source_stable_id: Some(stable_id),
            source_snapshot: None,
            source_name: Some(card_name),
            triggering_event: None,
            event_value_amount: None,
            trigger_identity: None,
            ability_index: None,
            intervening_if: None,
            keyword_payment_contributions: vec![],
            crew_contributors: vec![],
            saddle_contributors: vec![],
            chosen_modes: None,
            tagged_objects: std::collections::HashMap::new(),
            effect_outcomes: std::collections::HashMap::new(),
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
    use crate::events::traits::GameEventType;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::PlayerFilter;
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

        let outcome = CastTaggedEffect::new("it", PlayerFilter::You)
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
        assert_eq!(game.turn_store.turn_history.spells_cast_by_player(alice), 1);
        assert!(
            game.turn_store
                .turn_history
                .spell_cast_order(cast_id)
                .is_some()
        );
        assert!(
            outcome
                .events
                .iter()
                .any(|event| event.kind() == crate::events::EventKind::SpellCast),
            "cast-tagged spells should emit SpellCastEvent"
        );
    }

    #[test]
    fn swindlers_scheme_style_cast_tagged_uses_the_triggering_opponent_as_caster() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let card = CardBuilder::new(CardId::new(), "Revealed Spell")
            .card_types(vec![CardType::Sorcery])
            .build();
        let exiled_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let snapshot =
            ObjectSnapshot::from_object(game.object(exiled_id).expect("tagged card"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("revealed_0"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);

        let outcome = CastTaggedEffect::new("revealed_0", PlayerFilter::Specific(bob))
            .without_paying_mana_cost()
            .execute(&mut game, &mut ctx)
            .expect("cast tagged should resolve");

        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected cast tagged to create a stack object");
        };
        let cast_id = ids[0];
        let stack_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == cast_id)
            .expect("cast spell should be on the stack");
        assert_eq!(stack_entry.controller, bob);
        let spell_cast = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<crate::events::spells::SpellCastEvent>())
            .expect("cast tagged should emit a spell-cast event");
        assert_eq!(spell_cast.caster, bob);
        assert!(
            spell_cast.snapshot().is_some(),
            "spell-cast event should preserve the triggering spell snapshot"
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

        let outcome = CastTaggedEffect::new("it", PlayerFilter::You)
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

        let outcome = CastTaggedEffect::new("it", PlayerFilter::You)
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

        let outcome = CastTaggedEffect::new("it", PlayerFilter::You)
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

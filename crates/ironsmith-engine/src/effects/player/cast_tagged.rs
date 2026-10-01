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
use crate::game_state::GameState;
use crate::zone::Zone;
pub use ironsmith_core::CastTaggedEffect;

use super::runtime_helpers::{queue_effect_driven_land_play, with_spell_cast_event};

/// Effect that casts a tagged card immediately.
impl EffectExecutor for CastTaggedEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
        let checkpoint = game.clone();
        let context_checkpoint = crate::effects::ExecutionContextCheckpoint::capture(ctx);
        let instruction = (|| -> Result<EffectOutcome, ExecutionError> {
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

        let (is_land, from_zone) = {
            let Some(obj) = game.object(object_id) else {
                return Ok(EffectOutcome::target_invalid());
            };
            (obj.is_land(), obj.zone)
        };

        let caster = resolve_player_filter(game, &self.player, ctx)?;

        // A substituted payment replaces the mana cost: the spell is cast
        // without paying it, and the substitute is paid once the cast is made.
        let energy_payment = match self.alternative_payment {
            Some(ironsmith_core::CastTaggedAlternativePayment::EnergyEqualToManaValue)
                if !is_land =>
            {
                let mana_value = game
                    .object(object_id)
                    .and_then(|obj| obj.mana_cost.as_ref())
                    .map_or(0, |cost| cost.mana_value());
                if game
                    .player(caster)
                    .is_none_or(|player| player.energy_counters < mana_value)
                {
                    return Ok(EffectOutcome::impossible());
                }
                Some(mana_value)
            }
            _ => None,
        };
        let without_paying_mana_cost = self.without_paying_mana_cost || energy_payment.is_some();
        let pay_energy = |game: &mut GameState, outcome: EffectOutcome| match energy_payment {
            Some(amount) if amount > 0 => match game.remove_player_counters_with_source(
                caster,
                crate::object::CounterType::Energy,
                amount,
                Some(ctx.source),
                Some(ctx.controller),
            ) {
                Some((_, event)) => outcome.with_event(event),
                None => outcome,
            },
            _ => outcome,
        };

        if self.as_copy {
            let copy_id = game.new_object_id();

            let source_obj = match game.object(object_id) {
                Some(obj) => obj.clone(),
                None => return Ok(EffectOutcome::target_invalid()),
            };
            let mut copy_obj = crate::object::Object::token_copy_of(&source_obj, copy_id, caster);

            if is_land {
                if !self.allow_land {
                    return Ok(EffectOutcome::target_invalid());
                }
                copy_obj.zone = Zone::Command;
                game.add_object(copy_obj);
                let entry = move_to_battlefield_with_options(
                    game, ctx, copy_id, BattlefieldEntryOptions::specific(caster, false),
                )?;
                if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
                let entry = entry.ok_or_else(|| ExecutionError::InternalError("land entry lost its receipt without pending input".into()))?;
                let original = match &entry.outcome {
                    BattlefieldEntryOutcome::Moved(new_id) => {
                        queue_effect_driven_land_play(game, ctx, *new_id, caster, from_zone);
                        EffectOutcome::with_objects(vec![*new_id])
                    }
                    BattlefieldEntryOutcome::Redirected(change) => {
                        if let Some(id) = change.new_object_id {
                            queue_effect_driven_land_play(game, ctx, id, caster, from_zone);
                        }
                        EffectOutcome::with_objects(change.new_object_ids.clone())
                    }
                    BattlefieldEntryOutcome::Prevented => {
                        game.remove_object(copy_id);
                        EffectOutcome::impossible()
                    }
                };
                return crate::effects::zones::finish_battlefield_entry_receipts(game, ctx, original, vec![entry]);
            }

            copy_obj.zone = from_zone;
            game.add_object(copy_obj);
            let casting_method = if from_zone == Zone::Hand {
                CastingMethod::Normal
            } else {
                CastingMethod::PlayFrom {
                    source: ctx.source,
                    zone: from_zone,
                    use_alternative: None,
                }
            };
            let cast_tags = ctx.tagged_objects.clone();
            let result = crate::game_loop::cast_spell_from_resolving_effect_with_context(
                game,
                copy_id,
                from_zone,
                caster,
                &casting_method,
                without_paying_mana_cost,
                self.cost_reduction.as_ref(),
                self.additional_mana_cost.as_ref(),
                self.mana_spend_mode,
                cast_tags,
                ctx.provenance,
                &mut ctx.decision_maker,
            )
            .map_err(super::runtime_helpers::effect_driven_cast_error)?;
            let Some(new_id) = result else {
                game.remove_object(copy_id);
                return if ctx.decision_maker.awaiting_choice() {
                    Ok(EffectOutcome::count(0))
                } else {
                    Ok(EffectOutcome::impossible())
                };
            };
            let outcome = with_spell_cast_event(
                EffectOutcome::with_objects(vec![new_id]),
                game,
                new_id,
                caster,
                from_zone,
                ctx.provenance,
            );
            return Ok(pay_energy(game, outcome));
        }

        if is_land {
            if !self.allow_land {
                return Ok(EffectOutcome::target_invalid());
            }

            let entry = move_to_battlefield_with_options(
                game, ctx, object_id, BattlefieldEntryOptions::specific(caster, false),
            )?;
            if ctx.decision_maker.awaiting_choice() { return Ok(EffectOutcome::count(0)); }
            let entry = entry.ok_or_else(|| ExecutionError::InternalError("land entry lost its receipt without pending input".into()))?;
            let original = match &entry.outcome {
                BattlefieldEntryOutcome::Moved(new_id) => {
                    queue_effect_driven_land_play(game, ctx, *new_id, caster, from_zone);
                    EffectOutcome::with_objects(vec![*new_id])
                }
                BattlefieldEntryOutcome::Redirected(change) => {
                    if let Some(id) = change.new_object_id {
                        queue_effect_driven_land_play(game, ctx, id, caster, from_zone);
                    }
                    EffectOutcome::with_objects(change.new_object_ids.clone())
                }
                BattlefieldEntryOutcome::Prevented => {

                    EffectOutcome::impossible()
                }
            };
            return crate::effects::zones::finish_battlefield_entry_receipts(game, ctx, original, vec![entry]);
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

        let cast_tags = ctx.tagged_objects.clone();
        let result = crate::game_loop::cast_spell_from_resolving_effect_with_context(
            game,
            object_id,
            from_zone,
            caster,
            &casting_method,
            without_paying_mana_cost,
            self.cost_reduction.as_ref(),
            self.additional_mana_cost.as_ref(),
            self.mana_spend_mode,
            cast_tags,
            ctx.provenance,
            &mut ctx.decision_maker,
        )
        .map_err(super::runtime_helpers::effect_driven_cast_error)?;
        let Some(new_id) = result else {
            return if ctx.decision_maker.awaiting_choice() {
                Ok(EffectOutcome::count(0))
            } else {
                Ok(EffectOutcome::impossible())
            };
        };
        let outcome = with_spell_cast_event(
            EffectOutcome::with_objects(vec![new_id]),
            game,
            new_id,
            caster,
            from_zone,
            ctx.provenance,
        );
        Ok(pay_energy(game, outcome))
        })();
        let pending = ctx.decision_maker.awaiting_choice();
        if pending || instruction.is_err() { *game = checkpoint; context_checkpoint.restore(ctx); }
        if pending { return instruction.map(|_| EffectOutcome::count(0)); }
        instruction
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
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::triggers::matcher_trait::{TriggerContext, TriggerMatcher};
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
    fn cast_tagged_pays_additional_mana_and_retains_linked_spell_identity() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 3);
        let card = CardBuilder::new(CardId::new(), "Linked Graveyard Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Generic(1)]))
            .build();
        let graveyard_id = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let snapshot =
            ObjectSnapshot::from_object(game.object(graveyard_id).expect("tagged card"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("chosen_spell"), vec![snapshot.clone()]);
        tags.insert(TagKey::from("__it__"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
        let outcome = CastTaggedEffect::new("chosen_spell", PlayerFilter::You)
            .additional_mana_cost(ManaCost::from_symbols(vec![
                ManaSymbol::Red,
                ManaSymbol::Red,
            ]))
            .execute(&mut game, &mut ctx)
            .expect("cast tagged should resolve");

        let crate::effect::OutcomeValue::Objects(ids) = &outcome.value else {
            panic!("expected linked cast on the stack: {outcome:#?}");
        };
        let cast_id = ids[0];
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 0);
        assert!(
            game.object(cast_id)
                .expect("cast spell")
                .cast_tagged_objects
                .contains_key(&TagKey::from("__it__"))
        );

        let spell_cast = outcome
            .events
            .iter()
            .find_map(|event| event.downcast::<crate::events::spells::SpellCastEvent>())
            .expect("spell cast event");
        let mut linked_filter = ObjectFilter::spell();
        linked_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from("__it__"),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        let trigger =
            crate::triggers::SpellCastTrigger::new(Some(linked_filter), PlayerFilter::You);
        let event =
            crate::triggers::TriggerEvent::new_with_provenance(spell_cast.clone(), ctx.provenance);
        assert!(trigger.matches(&event, &TriggerContext::for_source(source, alice, &game),));
    }

    #[test]
    fn cast_tagged_any_type_mode_is_scoped_to_that_cast() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 1);
        let card = CardBuilder::new(CardId::new(), "Blue Graveyard Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let graveyard_id = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let snapshot =
            ObjectSnapshot::from_object(game.object(graveyard_id).expect("tagged card"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("chosen_spell"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
        let outcome = CastTaggedEffect::new("chosen_spell", PlayerFilter::You)
            .mana_spend_mode(ironsmith_core::value_model::ManaSpendMode::AnyType)
            .execute(&mut game, &mut ctx)
            .expect("any-type cast should resolve");

        assert!(outcome.status.is_success(), "{outcome:#?}");
        assert_eq!(game.stack.len(), 1);
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 0);
        assert_eq!(
            game.mana_spend_policy(alice, None).mode,
            ironsmith_core::value_model::ManaSpendMode::Normal,
            "the resolving instruction must not create a lasting permission"
        );
    }

    #[test]
    fn ordinary_cast_tagged_does_not_spend_red_mana_as_blue() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        game.player_mut(alice)
            .expect("alice exists")
            .mana_pool
            .add(ManaSymbol::Red, 1);
        let card = CardBuilder::new(CardId::new(), "Blue Graveyard Spell")
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_symbols(vec![ManaSymbol::Blue]))
            .build();
        let graveyard_id = game.create_object_from_card(&card, alice, Zone::Graveyard);
        let snapshot =
            ObjectSnapshot::from_object(game.object(graveyard_id).expect("tagged card"), &game);
        let mut tags = std::collections::HashMap::new();
        tags.insert(TagKey::from("chosen_spell"), vec![snapshot]);

        let source = game.new_object_id();
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
        let outcome = CastTaggedEffect::new("chosen_spell", PlayerFilter::You)
            .execute(&mut game, &mut ctx)
            .expect("declined ordinary cast should not fail resolution");

        assert!(!outcome.status.is_success(), "{outcome:#?}");
        assert!(game.stack.is_empty());
        assert_eq!(game.player(alice).expect("alice exists").mana_pool.red, 1);
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
    fn tagged_land_plays_preserve_resolved_saga_entry_counters() {
        use crate::events::MarkersChangedEvent;
        use crate::object::CounterType;
        use crate::replacement::ReplacementAction;
        use crate::types::Subtype;

        // Exercise both live notification callers: playing the original and
        // playing a newly created copy. Non-Saga and ordinary Saga controls
        // ensure this helper no longer owns any counter placement.
        for as_copy in [false, true] {
            for mode in 0..3 {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                let card = CardBuilder::new(CardId::new(), "Tagged entry land")
                    .card_types(vec![CardType::Land])
                    .subtypes(if mode == 0 {
                        vec![]
                    } else {
                        vec![Subtype::Saga]
                    })
                    .build();
                let original = game.create_object_from_card(&card, alice, Zone::Exile);
                let source = game.create_object_from_card(
                    &CardBuilder::new(CardId::new(), "Entry replacement source").build(),
                    alice,
                    Zone::Battlefield,
                );
                let prevention = if mode == 2 {
                    let mut replacement =
                        crate::static_abilities::StaticAbility::double_counters_replacement(
                            ObjectFilter::default()
                                .with_type(CardType::Land)
                                .with_subtype(Subtype::Saga),
                            Some(CounterType::Lore),
                            "Prevent entry lore".into(),
                        )
                        .generate_replacement_effect(source, alice)
                        .unwrap();
                    replacement.replacement = ReplacementAction::Prevent;
                    Some(
                        game.effect_store
                            .replacement_effects
                            .add_one_shot_effect(replacement),
                    )
                } else {
                    None
                };
                let snapshot = ObjectSnapshot::from_object(game.object(original).unwrap(), &game);
                let mut tags = std::collections::HashMap::new();
                tags.insert(TagKey::from("it"), vec![snapshot]);
                game.take_pending_trigger_events();
                let mut dm = SelectFirstDecisionMaker;
                let mut ctx = ExecutionContext::new(source, alice, &mut dm).with_tagged_objects(tags);
                let mut effect = CastTaggedEffect::new("it", PlayerFilter::You).allow_land();
                if as_copy {
                    effect = effect.as_copy();
                }
                let outcome = effect.execute(&mut game, &mut ctx).unwrap();
                assert!(!ctx.decision_maker.awaiting_choice());
                let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
                    panic!("expected a completed land play");
                };
                assert_eq!(ids.len(), 1);
                let entered = ids[0];
                assert!(game.battlefield.contains(&entered));
                assert_eq!(
                    game.counter_count(entered, CounterType::Lore),
                    u32::from(mode == 1)
                );
                assert_eq!(game.has_processed_saga_entry_lore(entered), mode != 0);
                assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 1);
                if as_copy {
                    assert_eq!(game.object(original).unwrap().zone, Zone::Exile);
                } else {
                    assert!(game.object(original).is_none());
                }
                if let Some(prevention) = prevention {
                    assert!(
                        game.effect_store
                            .replacement_effects
                            .get_effect(prevention)
                            .is_none()
                    );
                }
                let mut events = outcome.events;
                events.extend(game.take_pending_trigger_events());
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| event.kind() == crate::events::EventKind::LandPlayed)
                        .count(),
                    1
                );
                assert_eq!(
                    events
                        .iter()
                        .filter(|event| event.kind() == crate::events::EventKind::EnterBattlefield)
                        .count(),
                    1
                );
                let placements = events
                    .iter()
                    .filter_map(|event| {
                        if let Some(counter) = event.downcast::<crate::events::CounterPlacedEvent>() {
                            Some((counter.permanent, counter.counter_type, counter.amount))
                        } else if let Some(marker) = event.downcast::<MarkersChangedEvent>() {
                            if let (
                                crate::marker::MarkerLocation::Object(object),
                                crate::marker::Marker::Counter(kind),
                            ) = (&marker.location, &marker.marker)
                            {
                                marker.is_added().then_some((*object, *kind, marker.amount))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    placements,
                    if mode == 1 {
                        vec![(entered, CounterType::Lore, 1)]
                    } else {
                        vec![]
                    }
                );
            }
        }
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

#[cfg(test)]
mod replacement_cast_tagged_land_owner_contract_tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::{Effect, Value};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::object::CounterType;
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    struct Answers { player: PlayerId, pause: bool, pending: bool, calls: usize, binding: bool }
    impl DecisionMaker for Answers {
        fn decide_boolean(&mut self, game: &GameState, _: &crate::decisions::context::BooleanContext) -> bool {
            self.calls += 1;
            let lands = game.battlefield.iter().copied().filter(|id| game.object(*id).unwrap().is_land()).collect::<Vec<_>>();
            assert_eq!(lands.len(), 1, "the original land arrives before additions");
            assert_eq!(game.player(self.player).unwrap().lands_played_this_turn, 1, "authored land-play bookkeeping precedes additions");
            if self.binding { assert_eq!(game.counter_count(lands[0], CounterType::PlusOnePlusOne), 1); }
            self.pending = self.pause; !self.pending
        }
        fn awaiting_choice(&self) -> bool { self.pending }
    }
    fn card(game: &mut GameState, owner: PlayerId, kind: CardType, zone: Zone) -> ObjectId {
        game.create_object_from_card(&CardBuilder::new(CardId::new(), "Tagged land fixture")
            .card_types(vec![kind]).build(), owner, zone)
    }
    fn check(as_copy: bool, mode: u8) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0); let bob = PlayerId::from_index(1);
        let parent = card(&mut game, alice, CardType::Artifact, Zone::Battlefield);
        let source = card(&mut game, bob, CardType::Artifact, Zone::Battlefield);
        let target = card(&mut game, alice, CardType::Land, Zone::Exile);
        let selected = ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        let sentinel = ObjectSnapshot::from_object(game.object(parent).unwrap(), &game);
        let effects = match mode {
            1 => vec![Effect::gain_life(3), Effect::lose_life(Value::X)],
            3 => vec![Effect::new(crate::effects::PutCountersEffect::new(CounterType::PlusOnePlusOne, 1, ChooseSpec::tagged("it"))), Effect::may(vec![Effect::gain_life(0)])],
            _ => vec![Effect::gain_life(3), Effect::may(vec![Effect::gain_life(4)])],
        };
        let shield = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(source, bob,
            crate::events::zones::matchers::WouldChangeZoneMatcher::new(ObjectFilter::default().with_type(CardType::Land),
                Some(if as_copy {Zone::Command} else {Zone::Exile}), Some(Zone::Battlefield)), ReplacementAction::Additionally(effects)));
        game.take_pending_trigger_events(); let before_ids = game.next_object_id_counter();
        let before_objects = game.objects_in_deterministic_order().len();
        let mut dm = Answers { player: alice, pause: mode == 2, pending: false, calls: 0, binding: mode == 3 };
        let mut ctx = ExecutionContext::new(parent, alice, &mut dm);
        ctx.set_tagged_objects("casted", vec![selected.clone()]); ctx.set_tagged_objects("it", vec![sentinel.clone()]);
        let mut effect = CastTaggedEffect::new("casted", PlayerFilter::You).allow_land(); if as_copy { effect = effect.as_copy(); }
        let result = effect.execute(&mut game, &mut ctx);
        if mode == 1 { assert!(matches!(result, Err(ExecutionError::UnresolvableValue(_)))); }
        else if mode == 2 { assert!(ctx.decision_maker.awaiting_choice()); assert!(result.unwrap().events.is_empty()); }
        else {
            let outcome = result.unwrap(); assert_eq!(outcome.objects().unwrap().len(), 1);
            let arrival = outcome.objects().unwrap()[0]; assert!(game.battlefield.contains(&arrival));
            assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 1);
            assert_eq!(game.player(alice).unwrap().life, 20); assert_eq!(game.player(bob).unwrap().life, if mode == 3 {20} else {27});
            if mode == 3 {
                assert_eq!(game.counter_count(arrival, CounterType::PlusOnePlusOne), 1);
                assert!(outcome.execution_facts.iter().filter_map(|fact| match fact { crate::effect::ExecutionFact::AffectedObjectMemory(memory) => Some(memory.as_slice()), _ => None }).flatten().any(|memory| memory.object_id == arrival && memory.zone == Zone::Battlefield));
                assert!(!outcome.affected_object_memory().unwrap_or(&[]).iter().any(|memory| memory.object_id == arrival && memory.zone == Zone::Battlefield), "auxiliary post-move counter memory is not original movement memory");
            } else {
                assert_eq!(outcome.events.iter().filter_map(|event| event.downcast::<crate::events::LifeGainEvent>())
                    .map(|event| (event.player,event.amount)).collect::<Vec<_>>(), vec![(bob,3),(bob,4)]);
            }
            let events = game.take_pending_trigger_events();
            let played = events.iter().filter_map(|event| event.downcast::<crate::events::other::LandPlayedEvent>()).collect::<Vec<_>>();
            assert_eq!(played.len(), 1);
            assert_eq!(game.objects_in_deterministic_order().len(), before_objects + usize::from(as_copy));
            if as_copy { assert_eq!(game.object(target).unwrap().zone, Zone::Exile); }
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
        }
        assert_eq!(ctx.source, parent); assert_eq!(ctx.controller, alice);
        assert_eq!(ctx.get_tagged_all("it").unwrap()[0].object_id, sentinel.object_id);
        assert_eq!(ctx.get_tagged_all("casted").unwrap()[0].object_id, selected.object_id);
        assert_eq!(game.counter_count(parent, CounterType::PlusOnePlusOne), 0);
        if mode == 1 || mode == 2 {
            assert_eq!(game.next_object_id_counter(), before_ids);
            assert_eq!(game.objects_in_deterministic_order().len(), before_objects);
            assert_eq!(game.object(target).unwrap().zone, Zone::Exile);
            assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 0);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert!(game.command_zone.is_empty());
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_some()); assert!(game.take_pending_trigger_events().is_empty());
        }
        drop(ctx);
        if mode == 0 || mode == 3 { assert_eq!(dm.calls, 1); }
        if mode == 2 {
            assert_eq!(dm.calls, 1); dm.pause = false; dm.pending = false;
            let mut ctx = ExecutionContext::new(parent, alice, &mut dm); ctx.set_tagged_objects("casted", vec![selected]);
            let outcome = effect.execute(&mut game, &mut ctx).unwrap(); assert_eq!(outcome.objects().unwrap().len(), 1);
            assert_eq!(game.player(bob).unwrap().life, 27); assert_eq!(game.player(alice).unwrap().lands_played_this_turn, 1);
            assert!(game.effect_store.replacement_effects.get_effect(shield).is_none());
            assert!(!ctx.decision_maker.awaiting_choice()); drop(ctx); assert_eq!(dm.calls, 2);
        }
    }
    #[test] fn land_additions_follow_original_play() { check(false,0); }
    #[test] fn land_error_restores_entire_play() { check(false,1); }
    #[test] fn land_pending_replays_entire_play() { check(false,2); }
    #[test] fn land_addition_binds_arrival_and_returns_facts() { check(false,3); }
    #[test] fn copy_additions_follow_original_play() { check(true,0); }
    #[test] fn copy_error_restores_provisional_copy_and_play() { check(true,1); }
    #[test] fn copy_pending_replays_provisional_copy_and_play() { check(true,2); }
    #[test] fn copy_addition_binds_arrival_and_returns_facts() { check(true,3); }
}

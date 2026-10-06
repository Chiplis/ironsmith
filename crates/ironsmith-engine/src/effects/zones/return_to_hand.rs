//! Return to hand effect implementation.

use super::movement_instruction::{SelectedZoneMovement, ZoneMovementInstruction};
use crate::effect::EffectOutcome;
#[cfg(test)]
use crate::effect::OutcomeStatus;
use crate::effects::CompletedEffectOutputs;

use crate::effects::{CostExecutableEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::processing::EventOutcome;
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::zone::Zone;

pub type ReturnToHandEffect = ironsmith_core::ReturnToHandEffect;

impl EffectExecutor for ReturnToHandEffect {
    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        crate::effects::CostChoiceBindings::from_spec(&self.spec)
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Returned)
    }
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        Ok(super::movement_instruction::prepare_movement_instruction(
            self.clone(),
            ctx,
        ))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        super::movement_instruction::execute_movement_instruction(self.clone(), game, ctx)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.spec.is_target() {
            Some(&self.spec)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.spec.is_target() {
            Some(self.spec.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to return"
    }

    fn cost_description(&self) -> Option<String> {
        match self.spec.base() {
            ChooseSpec::Source => Some("Return this source to its owner's hand".to_string()),
            ChooseSpec::Object(filter) => Some(format!(
                "Return a {} you control to its owner's hand",
                filter.description()
            )),
            _ => None,
        }
    }
}

impl ZoneMovementInstruction for ReturnToHandEffect {
    fn select(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SelectedZoneMovement, ExecutionError> {
        let mut objects = match super::resolve_zone_move_objects(game, ctx, &self.spec) {
            Ok(objects) => objects,
            Err(ExecutionError::InvalidTarget) => {
                return Ok(SelectedZoneMovement::Finished(
                    if self.spec.count().min == 0 && ctx.targets.is_empty() {
                        EffectOutcome::count(0)
                    } else {
                        EffectOutcome::target_invalid()
                    },
                ));
            }
            Err(error) => return Err(error),
        };
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SelectedZoneMovement::Finished(EffectOutcome::count(0)));
        }
        let counted_targets = self.spec.is_target()
            && matches!(
                self.spec.unhinted(),
                ChooseSpec::WithCount(..) | ChooseSpec::WithCountValue(..)
            );
        let single_target = self.spec.is_target() && self.spec.is_single() && !counted_targets;
        if single_target {
            objects.truncate(1);
        }
        if objects.is_empty()
            && (self.spec.is_target() || matches!(self.spec.base(), ChooseSpec::Tagged(_)))
        {
            return Ok(SelectedZoneMovement::Finished(
                if self.spec.count().min == 0 && ctx.targets.is_empty() {
                    EffectOutcome::count(0)
                } else {
                    EffectOutcome::target_invalid()
                },
            ));
        }
        // Selection and source observations are fixed before any sibling's
        // replacement proposal or movement can change the selected world.
        let snapshots = objects
            .iter()
            .filter_map(|id| {
                ObjectSnapshot::from_object_id(game, *id).map(|snapshot| (*id, snapshot))
            })
            .collect::<std::collections::HashMap<_, _>>();
        let requests = objects
            .into_iter()
            .filter_map(|id| {
                snapshots.get(&id).map(|snapshot| {
                    super::PreparedZoneMove::capture(
                        game,
                        id,
                        snapshot.zone,
                        Zone::Hand,
                        ctx.cause.clone(),
                        Some(snapshot.clone()),
                    )
                })
            })
            .collect();

        Ok(SelectedZoneMovement::moves(
            requests,
            move |game, ctx, receipts, pending_start| {
                let originals = super::observe_zone_move_originals(
                    game,
                    ctx,
                    receipts,
                    &snapshots,
                    pending_start,
                )?;
                let affected = originals
                    .iter()
                    .flat_map(|(_, change, _)| change.new_object_ids.iter().copied())
                    .collect::<Vec<_>>();
                let memories = originals
                    .iter()
                    .map(|(_, _, snapshot)| snapshot.clone())
                    .collect();
                let outcome = if single_target {
                    match receipts.first().map(|(_, receipt)| &receipt.original) {
                        Some(EventOutcome::Proceed(_)) => EffectOutcome::resolved(),
                        Some(EventOutcome::Prevented) => EffectOutcome::prevented(),
                        Some(EventOutcome::Replaced) => EffectOutcome::replaced(),
                        Some(EventOutcome::NotApplicable) | None => EffectOutcome::target_invalid(),
                    }
                } else {
                    EffectOutcome::count(originals.len() as i32)
                };
                Ok(outcome
                    .with_affected_objects(affected)
                    .with_affected_object_memory(memories))
            },
        ))
    }
}

impl CostExecutableEffect for ReturnToHandEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: crate::ids::ObjectId,
        controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::CostValidationError> {
        match self.spec.base() {
            ChooseSpec::Source => {
                if game
                    .object(source)
                    .is_some_and(|obj| obj.zone == Zone::Battlefield)
                {
                    Ok(())
                } else {
                    Err(crate::effects::CostValidationError::Other(
                        "source must be on the battlefield".to_string(),
                    ))
                }
            }
            ChooseSpec::Object(filter) => {
                let filter_ctx = crate::filter::FilterContext::new(controller).with_source(source);
                let available = game
                    .battlefield
                    .iter()
                    .copied()
                    .filter(|id| {
                        game.object(*id)
                            .is_some_and(|obj| filter.matches(obj, &filter_ctx, game))
                    })
                    .count();
                if available == 0 {
                    Err(crate::effects::CostValidationError::Other(
                        "no valid return target".to_string(),
                    ))
                } else {
                    Ok(())
                }
            }
            _ => Err(crate::effects::CostValidationError::Other(
                "unsupported return-to-hand cost".to_string(),
            )),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::effect::Effect;
    use crate::effects::ExecutionContext;
    use crate::events::zones::matchers::WouldGoToHandMatcher;
    use crate::game_state::GameState;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::replacement::{ReplacementAction, ReplacementEffect};
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::test_prelude::*;
    use crate::types::CardType;

    struct SelectIdsDecisionMaker {
        chosen: Vec<ObjectId>,
    }

    impl DecisionMaker for SelectIdsDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.chosen
                .iter()
                .copied()
                .filter(|id| {
                    ctx.candidates
                        .iter()
                        .any(|candidate| candidate.legal && candidate.id == *id)
                })
                .collect()
        }
    }

    fn add_land(game: &mut GameState, card_id: u32, name: &str, controller: PlayerId) -> ObjectId {
        let card = CardBuilder::new(CardId::from_raw(card_id), name)
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn growth_chamber_style_bounce_can_choose_the_source_land() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let growth_chamber = add_land(&mut game, 561, "Simic Growth Chamber", alice);
        let forest = add_land(&mut game, 562, "Forest", alice);
        let mut dm = SelectIdsDecisionMaker {
            chosen: vec![growth_chamber],
        };
        let mut ctx =
            ExecutionContext::new_default(growth_chamber, alice).with_decision_maker(&mut dm);
        let effect = ReturnToHandEffect::with_spec(
            ChooseSpec::Object(ObjectFilter::land().you_control())
                .with_count(ChoiceCount::exactly(1)),
        );

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("bounce effect should resolve");
        let bounced_card_in_hand = game.players[0].hand.iter().any(|&id| {
            game.object(id)
                .is_some_and(|obj| obj.name == "Simic Growth Chamber")
        });

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(bounced_card_in_hand);
        assert!(!game.battlefield.contains(&growth_chamber));
        assert!(game.battlefield.contains(&forest));
    }

    #[test]
    fn contextual_destination_surface_does_not_change_physical_owner_hand() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let bob = game.players[1].id;
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(563), "Borrowed Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let borrowed = game.create_object_from_card(&card, bob, Zone::Battlefield);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = ReturnToHandEffect::with_spec(ChooseSpec::SpecificObject(borrowed))
            .with_destination_player_surface(crate::filter::PlayerFilter::You);

        effect
            .execute(&mut game, &mut ctx)
            .expect("return should resolve");

        assert!(game.player(bob).expect("Bob exists").hand.iter().any(|id| {
            game.object(*id)
                .is_some_and(|object| object.name == "Borrowed Relic")
        }));
        assert!(
            game.player(alice)
                .expect("Alice exists")
                .hand
                .iter()
                .all(|id| game
                    .object(*id)
                    .is_none_or(|object| object.name != "Borrowed Relic"))
        );
    }

    #[test]
    fn tagged_return_to_hand_follows_stable_id_after_zone_change() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();

        let card = CardBuilder::new(CardId::from_raw(9901), "Tagged Return Probe")
            .card_types(vec![CardType::Artifact])
            .build();
        let original_id = game.create_object_from_card(&card, alice, Zone::Exile);
        let tagged_snapshot = game
            .object(original_id)
            .map(|obj| ObjectSnapshot::from_object(obj, &game))
            .expect("exiled object should exist");
        let stable_id = tagged_snapshot.stable_id;

        let move_to_graveyard =
            crate::effects::MoveToZoneEffect::to_graveyard(ChooseSpec::SpecificObject(original_id));
        let mut move_ctx = ExecutionContext::new_default(source, alice);
        move_to_graveyard
            .execute(&mut game, &mut move_ctx)
            .expect("move to graveyard should resolve");

        let current_id = game
            .find_object_by_stable_id(stable_id)
            .expect("stable id should still resolve");
        assert_ne!(current_id, original_id);

        let mut ctx = ExecutionContext::new_default(source, alice).with_tagged_objects(
            std::collections::HashMap::from([(TagKey::from("chosen"), vec![tagged_snapshot])]),
        );
        let effect = ReturnToHandEffect::with_spec(ChooseSpec::Tagged(TagKey::from("chosen")));
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("return to hand should resolve");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(
            game.player(alice)
                .expect("alice exists")
                .hand
                .iter()
                .any(|&id| game
                    .object(id)
                    .is_some_and(|obj| obj.name == "Tagged Return Probe")),
            "the tagged card should return using the current object id"
        );
    }

    #[test]
    fn source_return_to_hand_follows_source_snapshot_stable_id() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;

        let card = CardBuilder::new(CardId::from_raw(9904), "Source Return Probe")
            .card_types(vec![CardType::Enchantment])
            .build();
        let original_id = game.create_object_from_card(&card, alice, Zone::Battlefield);
        let source_snapshot = game
            .object(original_id)
            .map(|obj| ObjectSnapshot::from_object(obj, &game))
            .expect("source object should exist");

        let move_to_graveyard =
            crate::effects::MoveToZoneEffect::to_graveyard(ChooseSpec::SpecificObject(original_id));
        let mut move_ctx = ExecutionContext::new_default(original_id, alice);
        move_to_graveyard
            .execute(&mut game, &mut move_ctx)
            .expect("move to graveyard should resolve");
        assert!(game.object(original_id).is_none());

        let mut ctx =
            ExecutionContext::new_default(original_id, alice).with_source_snapshot(source_snapshot);
        let effect = ReturnToHandEffect::with_spec(ChooseSpec::Source);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("source return should resolve through stable id");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert!(
            game.player(alice)
                .expect("alice exists")
                .hand
                .iter()
                .any(|&id| game
                    .object(id)
                    .is_some_and(|obj| obj.name == "Source Return Probe")),
            "the source card should return using its current object id"
        );
    }

    #[test]
    fn tagged_follow_up_does_not_retarget_replacement_redirected_object() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();
        let creature = add_land(&mut game, 9902, "Redirected Bounce Probe", alice);

        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                alice,
                WouldGoToHandMatcher::you(),
                ReplacementAction::Instead(vec![Effect::new(
                    crate::effects::MoveToZoneEffect::to_exile(ChooseSpec::SpecificObject(
                        creature,
                    )),
                )]),
            ),
        );

        let tagged_bounce = crate::effects::TaggedEffect::new(
            "bounced",
            Effect::new(ReturnToHandEffect::with_spec(ChooseSpec::SpecificObject(
                creature,
            ))),
        );
        let mut ctx = ExecutionContext::new_default(source, alice);
        let bounce_outcome = tagged_bounce.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(bounce_outcome.status, OutcomeStatus::Succeeded);
        assert!(game.players[0].hand.is_empty());
        assert_eq!(game.exile.len(), 1);
        assert!(
            ctx.get_tagged("bounced").is_none(),
            "replacement mismatch should not leave behind a live tagged object"
        );

        let follow_up = crate::effects::MoveToZoneEffect::to_graveyard(ChooseSpec::Tagged(
            TagKey::from("bounced"),
        ));
        let follow_up_outcome = follow_up.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(follow_up_outcome.status, OutcomeStatus::Succeeded);
        assert!(game.players[0].hand.is_empty());
        assert!(game.players[0].graveyard.is_empty());
        assert_eq!(game.exile.len(), 1);
        assert!(
            game.object(game.exile[0])
                .is_some_and(|obj| obj.zone == Zone::Exile && obj.name == "Redirected Bounce Probe")
        );
    }

    /// Selective Snare scenario: targeted return-to-hand of X creatures that
    /// match a chosen creature type. Two of three creatures on the battlefield
    /// share the chosen type; a third does not. With X=2, only the two matching
    /// creatures should be targeted and returned.
    #[test]
    fn targeted_return_to_hand_with_x_count_and_creature_type_filter() {
        use crate::effects::ChooseCreatureTypeEffect;
        use crate::types::Subtype;

        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = game.players[0].id;
        let source = game.new_object_id();

        // Create two Goblins and one Elf on the battlefield.
        let goblin1 = {
            let card = CardBuilder::new(CardId::from_raw(701), "Goblin Raider")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Goblin])
                .build();
            game.create_object_from_card(&card, alice, Zone::Battlefield)
        };
        let goblin2 = {
            let card = CardBuilder::new(CardId::from_raw(702), "Goblin Piker")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Goblin])
                .build();
            game.create_object_from_card(&card, alice, Zone::Battlefield)
        };
        let elf = {
            let card = CardBuilder::new(CardId::from_raw(703), "Llanowar Elves")
                .card_types(vec![CardType::Creature])
                .subtypes(vec![Subtype::Elf])
                .build();
            game.create_object_from_card(&card, alice, Zone::Battlefield)
        };
        assert_eq!(game.battlefield.len(), 3);

        // Step 1: Choose creature type "Goblin".
        struct ChooseGoblinDm;
        impl DecisionMaker for ChooseGoblinDm {
            fn decide_options(
                &mut self,
                _game: &GameState,
                ctx: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                ctx.options
                    .iter()
                    .find(|opt| opt.description.eq_ignore_ascii_case("goblin"))
                    .map(|opt| vec![opt.index])
                    .unwrap_or_else(|| vec![0])
            }
        }
        let mut dm = ChooseGoblinDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ChooseCreatureTypeEffect::new(crate::target::PlayerFilter::You, vec![])
            .execute(&mut game, &mut ctx)
            .expect("choose creature type should succeed");
        assert_eq!(
            game.chosen_creature_type(source),
            Some(Subtype::Goblin),
            "creature type should be set to Goblin"
        );

        // Step 2: Return all Goblins (chosen creature type) to hand.
        let filter = ObjectFilter::creature().of_chosen_creature_type();
        let effect = ReturnToHandEffect::all(filter);

        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("return all of chosen type should resolve");

        // Both goblins should be returned to hand.
        assert_eq!(
            outcome.value,
            crate::effect::OutcomeValue::Count(2),
            "both goblins of the chosen type should be returned, got {:?}",
            outcome.value
        );
        assert!(
            !game.battlefield.contains(&goblin1),
            "goblin1 should no longer be on the battlefield"
        );
        assert!(
            !game.battlefield.contains(&goblin2),
            "goblin2 should no longer be on the battlefield"
        );
        // The elf should still be on the battlefield.
        assert!(
            game.battlefield.contains(&elf),
            "elf should remain on the battlefield — it is not of the chosen type"
        );
        // Two cards should be in hand.
        let hand_names: Vec<&str> = game.players[0]
            .hand
            .iter()
            .filter_map(|&id| game.object(id).map(|obj| obj.name.as_str()))
            .collect();
        assert!(
            hand_names.contains(&"Goblin Raider") && hand_names.contains(&"Goblin Piker"),
            "both goblins should be in Alice's hand, got {:?}",
            hand_names
        );
    }
}
